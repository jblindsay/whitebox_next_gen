//! GeoParquet reader and writer.
//!
//! This module is available when built with the `geoparquet` feature.
//! Current scope:
//! - Reads one row group stream into a single [`Layer`]
//! - Writes one layer to a single Parquet table with WKB geometry
//! - Geometry from/to a binary column (`geometry`)
//! - Attribute extraction/encoding for common scalar field values
//! - GeoParquet metadata parsing and emission for geometry column + CRS hints

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use parquet::basic::{Compression, ConvertedType, Type as PhysicalType};
use parquet::data_type::{ByteArray, ByteArrayType, DoubleType, Int64Type};
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::format::KeyValue;
use parquet::record::{Field, Row};
use parquet::schema::types::{Type, TypePtr};
use serde_json::Value;

use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Layer};
use crate::geometry::Geometry;

const GEOMETRY_COL: &str = "geometry";
const WBVECTOR_FIELD_TYPES_KEY: &str = "wbvector_field_types";

/// Write-time tuning options for GeoParquet output.
#[derive(Debug, Clone)]
pub struct GeoParquetWriteOptions {
    /// Maximum number of rows per row group.
    pub max_rows_per_group: usize,
    /// Best-effort data page size target in bytes.
    pub data_page_size_limit: usize,
    /// Internal write batch size.
    pub write_batch_size: usize,
    /// Best-effort maximum row count per data page.
    pub data_page_row_count_limit: usize,
    /// Column compression codec used for all columns.
    pub compression: Compression,
}

impl Default for GeoParquetWriteOptions {
    fn default() -> Self {
        Self {
            max_rows_per_group: 1024 * 1024,
            data_page_size_limit: parquet::file::properties::DEFAULT_PAGE_SIZE,
            write_batch_size: parquet::file::properties::DEFAULT_WRITE_BATCH_SIZE,
            data_page_row_count_limit: parquet::file::properties::DEFAULT_DATA_PAGE_ROW_COUNT_LIMIT,
            compression: parquet::file::properties::DEFAULT_COMPRESSION,
        }
    }
}

impl GeoParquetWriteOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience preset for large-file exports.
    ///
    /// This profile keeps a large row-group target, increases write batching,
    /// and enables SNAPPY compression for a good size/speed balance.
    pub fn for_large_files() -> Self {
        Self::new()
            .with_max_rows_per_group(1_000_000)
            .with_data_page_size_limit(2 * 1024 * 1024)
            .with_write_batch_size(8_192)
            .with_data_page_row_count_limit(50_000)
            .with_compression(Compression::SNAPPY)
    }

    /// Convenience preset for smaller/interactive exports.
    ///
    /// This profile favors lower-latency writes and simpler output by using
    /// moderate row-group sizing and no compression.
    pub fn for_interactive_files() -> Self {
        Self::new()
            .with_max_rows_per_group(100_000)
            .with_data_page_size_limit(parquet::file::properties::DEFAULT_PAGE_SIZE)
            .with_write_batch_size(2_048)
            .with_data_page_row_count_limit(parquet::file::properties::DEFAULT_DATA_PAGE_ROW_COUNT_LIMIT)
            .with_compression(Compression::UNCOMPRESSED)
    }

    pub fn with_max_rows_per_group(mut self, value: usize) -> Self {
        self.max_rows_per_group = value.max(1);
        self
    }

    pub fn with_data_page_size_limit(mut self, value: usize) -> Self {
        self.data_page_size_limit = value.max(1);
        self
    }

    pub fn with_write_batch_size(mut self, value: usize) -> Self {
        self.write_batch_size = value.max(1);
        self
    }

    pub fn with_data_page_row_count_limit(mut self, value: usize) -> Self {
        self.data_page_row_count_limit = value.max(1);
        self
    }

    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }
}

/// Read a GeoParquet file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let file = File::open(path).map_err(GeoError::Io)?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| GeoError::GeoParquet(format!("failed opening parquet file: {e}")))?;

    let file_meta = reader.metadata().file_metadata();
    let kv_meta = file_meta.key_value_metadata().map(|v| v.as_slice());
    let geo_meta = parse_geo_metadata(kv_meta)?;
    let declared_types = parse_wbvector_field_types(kv_meta)?;
    let geom_col = geo_meta
        .primary_column
        .clone()
        .unwrap_or_else(|| "geometry".to_owned());
    let schema_types = infer_types_from_schema(file_meta, &geom_col);

    let row_iter = reader
        .get_row_iter(None)
        .map_err(|e| GeoError::GeoParquet(format!("failed to iterate rows: {e}")))?;

    let mut rows = Vec::<Row>::new();
    for row in row_iter {
        let row = row.map_err(|e| GeoError::GeoParquet(format!("failed reading parquet row: {e}")))?;
        rows.push(row);
    }

    let mut ordered_attr_names: Vec<String> = file_meta
        .schema_descr()
        .columns()
        .iter()
        .filter_map(|c| {
            let name = c.name();
            if name == geom_col.as_str() {
                None
            } else {
                Some(name.to_owned())
            }
        })
        .collect();
    let mut inferred_types: HashMap<String, FieldType> = HashMap::new();

    for name in &ordered_attr_names {
        if let Some(t) = declared_types.get(name) {
            inferred_types.insert(name.clone(), *t);
        } else if let Some(t) = schema_types.get(name) {
            inferred_types.insert(name.clone(), *t);
        }
    }

    for row in &rows {
        for (name, field) in row.get_column_iter() {
            if name.as_str() == geom_col.as_str() {
                continue;
            }
            if !ordered_attr_names.iter().any(|n| n == name) {
                ordered_attr_names.push(name.clone());
            }
            let inferred = declared_types
                .get(name)
                .copied()
                .or_else(|| schema_types.get(name).copied())
                .unwrap_or_else(|| infer_field_type(field));
            let entry = inferred_types.entry(name.clone()).or_insert(inferred);
            *entry = FieldValue::widen_type(*entry, inferred);
        }
    }

    let mut layer = Layer::new("layer");
    if let Some(epsg) = geo_meta.epsg {
        layer = layer.with_crs_epsg(epsg);
    }
    if let Some(wkt) = geo_meta.wkt {
        layer = layer.with_crs_wkt(wkt);
    }

    for name in &ordered_attr_names {
        let ty = inferred_types.get(name).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(name, ty));
    }

    for (idx, row) in rows.into_iter().enumerate() {
        let mut geom = None;
        let mut attrs = vec![FieldValue::Null; layer.schema.len()];

        for (name, field) in row.get_column_iter() {
            if name.as_str() == geom_col.as_str() {
                geom = geometry_from_field(field)?;
            } else if let Some(i) = layer.schema.field_index(name) {
                let hinted_type = layer.schema.fields()[i].field_type;
                attrs[i] = field_to_value_with_hint(field, hinted_type);
            }
        }

        layer.add_feature(geom, &[])?;
        if let Some(f) = layer.features.get_mut(idx) {
            f.fid = idx as u64;
            f.attributes = attrs;
        }
    }

    Ok(layer)
}

/// Write a [`Layer`] to a GeoParquet file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    write_with_options(layer, path, &GeoParquetWriteOptions::default())
}

/// Write a [`Layer`] to a GeoParquet file using explicit write options.
pub fn write_with_options<P: AsRef<Path>>(
    layer: &Layer,
    path: P,
    options: &GeoParquetWriteOptions,
) -> Result<()> {
    let file = File::create(path).map_err(GeoError::Io)?;
    let (schema, columns) = build_schema(layer)?;

    let props = WriterProperties::builder()
        .set_max_row_group_size(options.max_rows_per_group.max(1))
        .set_data_page_size_limit(options.data_page_size_limit.max(1))
        .set_write_batch_size(options.write_batch_size.max(1))
        .set_data_page_row_count_limit(options.data_page_row_count_limit.max(1))
        .set_compression(options.compression)
        .build();

    let mut writer = SerializedFileWriter::new(file, schema, Arc::new(props))
        .map_err(|e| GeoError::GeoParquet(format!("failed creating parquet writer: {e}")))?;

    writer.append_key_value_metadata(KeyValue {
        key: "geo".to_owned(),
        value: Some(build_geo_metadata(layer).to_string()),
    });
    writer.append_key_value_metadata(KeyValue {
        key: WBVECTOR_FIELD_TYPES_KEY.to_owned(),
        value: Some(build_wbvector_field_types_metadata(layer).to_string()),
    });

    let total_rows = layer.features.len();
    let group_size = options.max_rows_per_group.max(1);

    let mut start = 0usize;
    while start < total_rows {
        let end = (start + group_size).min(total_rows);
        let mut row_group = writer
            .next_row_group()
            .map_err(|e| GeoError::GeoParquet(format!("failed creating row group: {e}")))?;

        for col in &columns {
            let mut col_writer = row_group
                .next_column()
                .map_err(|e| GeoError::GeoParquet(format!("failed creating column writer: {e}")))?
                .ok_or_else(|| GeoError::GeoParquet("missing column writer for schema field".to_owned()))?;
            write_column_range(layer, col, start, end, &mut col_writer)?;
            col_writer
                .close()
                .map_err(|e| GeoError::GeoParquet(format!("failed closing column writer: {e}")))?;
        }

        row_group
            .close()
            .map_err(|e| GeoError::GeoParquet(format!("failed closing row group: {e}")))?;

        start = end;
    }

    writer
        .close()
        .map_err(|e| GeoError::GeoParquet(format!("failed closing parquet writer: {e}")))?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ColumnKind {
    Geometry,
    Integer(usize),
    Float(usize),
    Boolean(usize),
    Text(usize),
    Blob(usize),
}

fn build_schema(layer: &Layer) -> Result<(TypePtr, Vec<ColumnKind>)> {
    let mut fields: Vec<TypePtr> = Vec::new();
    let mut columns: Vec<ColumnKind> = Vec::new();

    fields.push(Arc::new(
        Type::primitive_type_builder(GEOMETRY_COL, PhysicalType::BYTE_ARRAY)
            .build()
            .map_err(|e| GeoError::GeoParquet(format!("failed building geometry field: {e}")))?,
    ));
    columns.push(ColumnKind::Geometry);

    for (idx, field) in layer.schema.fields().iter().enumerate() {
        let (parquet_field, kind) = parquet_field_def(field, idx)?;
        fields.push(Arc::new(parquet_field));
        columns.push(kind);
    }

    let schema = Type::group_type_builder("schema")
        .with_fields(fields)
        .build()
        .map_err(|e| GeoError::GeoParquet(format!("failed building parquet schema: {e}")))?;

    Ok((Arc::new(schema), columns))
}

fn parquet_field_def(field: &FieldDef, idx: usize) -> Result<(Type, ColumnKind)> {
    match field.field_type {
        FieldType::Integer => Ok((
            Type::primitive_type_builder(&field.name, PhysicalType::INT64)
                .build()
                .map_err(|e| GeoError::GeoParquet(format!("failed building INT64 field '{}': {e}", field.name)))?,
            ColumnKind::Integer(idx),
        )),
        FieldType::Float => Ok((
            Type::primitive_type_builder(&field.name, PhysicalType::DOUBLE)
                .build()
                .map_err(|e| GeoError::GeoParquet(format!("failed building DOUBLE field '{}': {e}", field.name)))?,
            ColumnKind::Float(idx),
        )),
        FieldType::Boolean => Ok((
            Type::primitive_type_builder(&field.name, PhysicalType::BOOLEAN)
                .build()
                .map_err(|e| GeoError::GeoParquet(format!("failed building BOOLEAN field '{}': {e}", field.name)))?,
            ColumnKind::Boolean(idx),
        )),
        FieldType::Blob => Ok((
            Type::primitive_type_builder(&field.name, PhysicalType::BYTE_ARRAY)
                .build()
                .map_err(|e| GeoError::GeoParquet(format!("failed building BYTE_ARRAY field '{}': {e}", field.name)))?,
            ColumnKind::Blob(idx),
        )),
        FieldType::Text | FieldType::Date | FieldType::DateTime | FieldType::Json => {
            let converted = if matches!(field.field_type, FieldType::Json) {
                ConvertedType::JSON
            } else {
                ConvertedType::UTF8
            };
            Ok((
                Type::primitive_type_builder(&field.name, PhysicalType::BYTE_ARRAY)
                    .with_converted_type(converted)
                    .build()
                    .map_err(|e| {
                        GeoError::GeoParquet(format!(
                            "failed building UTF-8 BYTE_ARRAY field '{}': {e}",
                            field.name
                        ))
                    })?,
                ColumnKind::Text(idx),
            ))
        }
    }
}

fn write_column(
    layer: &Layer,
    kind: &ColumnKind,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    match kind {
        ColumnKind::Geometry => write_geometry_column(layer, start, end, col_writer),
        ColumnKind::Integer(idx) => write_i64_column(layer, *idx, start, end, col_writer),
        ColumnKind::Float(idx) => write_f64_column(layer, *idx, start, end, col_writer),
        ColumnKind::Boolean(idx) => write_bool_column(layer, *idx, start, end, col_writer),
        ColumnKind::Text(idx) => write_text_column(layer, *idx, start, end, col_writer),
        ColumnKind::Blob(idx) => write_blob_column(layer, *idx, start, end, col_writer),
    }
}

fn write_column_range(
    layer: &Layer,
    kind: &ColumnKind,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    write_column(layer, kind, start, end, col_writer)
}

fn write_geometry_column(
    layer: &Layer,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<ByteArray> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        if let Some(g) = &f.geometry {
            def_levels.push(1);
            values.push(ByteArray::from(g.to_wkb()));
        } else {
            def_levels.push(0);
        }
    }

    col_writer
        .typed::<ByteArrayType>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing geometry column: {e}")))?;
    Ok(())
}

fn write_i64_column(
    layer: &Layer,
    idx: usize,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<i64> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        match f.attributes.get(idx).unwrap_or(&FieldValue::Null) {
            FieldValue::Integer(v) => {
                def_levels.push(1);
                values.push(*v);
            }
            FieldValue::Float(v) => {
                def_levels.push(1);
                values.push(*v as i64);
            }
            _ => def_levels.push(0),
        }
    }

    col_writer
        .typed::<Int64Type>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing INT64 column: {e}")))?;
    Ok(())
}

fn write_f64_column(
    layer: &Layer,
    idx: usize,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<f64> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        match f.attributes.get(idx).unwrap_or(&FieldValue::Null) {
            FieldValue::Float(v) => {
                def_levels.push(1);
                values.push(*v);
            }
            FieldValue::Integer(v) => {
                def_levels.push(1);
                values.push(*v as f64);
            }
            _ => def_levels.push(0),
        }
    }

    col_writer
        .typed::<DoubleType>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing DOUBLE column: {e}")))?;
    Ok(())
}

fn write_bool_column(
    layer: &Layer,
    idx: usize,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<bool> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        match f.attributes.get(idx).unwrap_or(&FieldValue::Null) {
            FieldValue::Boolean(v) => {
                def_levels.push(1);
                values.push(*v);
            }
            _ => def_levels.push(0),
        }
    }

    col_writer
        .typed::<parquet::data_type::BoolType>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing BOOLEAN column: {e}")))?;
    Ok(())
}

fn write_text_column(
    layer: &Layer,
    idx: usize,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<ByteArray> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        match f.attributes.get(idx).unwrap_or(&FieldValue::Null) {
            FieldValue::Text(s) | FieldValue::Date(s) | FieldValue::DateTime(s) => {
                def_levels.push(1);
                values.push(ByteArray::from(s.as_str()));
            }
            FieldValue::Integer(v) => {
                def_levels.push(1);
                values.push(ByteArray::from(v.to_string().as_str()));
            }
            FieldValue::Float(v) => {
                def_levels.push(1);
                values.push(ByteArray::from(v.to_string().as_str()));
            }
            FieldValue::Boolean(v) => {
                def_levels.push(1);
                values.push(ByteArray::from(v.to_string().as_str()));
            }
            _ => def_levels.push(0),
        }
    }

    col_writer
        .typed::<ByteArrayType>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing text column: {e}")))?;
    Ok(())
}

fn write_blob_column(
    layer: &Layer,
    idx: usize,
    start: usize,
    end: usize,
    col_writer: &mut parquet::file::writer::SerializedColumnWriter<'_>,
) -> Result<()> {
    let mut values: Vec<ByteArray> = Vec::new();
    let mut def_levels: Vec<i16> = Vec::with_capacity(end.saturating_sub(start));

    for f in &layer.features[start..end] {
        match f.attributes.get(idx).unwrap_or(&FieldValue::Null) {
            FieldValue::Blob(v) => {
                def_levels.push(1);
                values.push(ByteArray::from(v.clone()));
            }
            _ => def_levels.push(0),
        }
    }

    col_writer
        .typed::<ByteArrayType>()
        .write_batch(&values, Some(&def_levels), None)
        .map_err(|e| GeoError::GeoParquet(format!("failed writing BLOB column: {e}")))?;
    Ok(())
}

fn build_geo_metadata(layer: &Layer) -> Value {
    let mut geom_types = Vec::<String>::new();

    if let Some(gt) = layer.geom_type {
        geom_types.push(gt.as_str().to_owned());
    } else {
        for f in &layer.features {
            if let Some(g) = &f.geometry {
                let name = g.geom_type().as_str().to_owned();
                if !geom_types.iter().any(|x| x == &name) {
                    geom_types.push(name);
                }
            }
        }
    }

    if geom_types.is_empty() {
        geom_types.push("Geometry".to_owned());
    }

    let mut geom_col = serde_json::json!({
        "encoding": "WKB",
        "geometry_types": geom_types,
    });

    if let Some(epsg) = layer.crs_epsg() {
        geom_col["crs"] = serde_json::json!({
            "id": {
                "authority": "EPSG",
                "code": epsg,
            }
        });
    } else if let Some(wkt) = layer.crs_wkt() {
        geom_col["crs"] = serde_json::json!({ "wkt": wkt });
    }

    serde_json::json!({
        "version": "1.1.0",
        "primary_column": GEOMETRY_COL,
        "columns": {
            GEOMETRY_COL: geom_col,
        }
    })
}

fn geometry_from_field(field: &Field) -> Result<Option<Geometry>> {
    match field {
        Field::Null => Ok(None),
        Field::Bytes(bytes) => {
            let geom = Geometry::from_wkb(bytes.data())
                .map_err(|e| GeoError::GeoParquet(format!("invalid WKB geometry: {e}")))?;
            Ok(Some(geom))
        }
        other => Err(GeoError::GeoParquet(format!(
            "geometry column must be binary WKB, found {other:?}"
        ))),
    }
}

fn infer_field_type(field: &Field) -> FieldType {
    match field {
        Field::Null => FieldType::Text,
        Field::Bool(_) => FieldType::Boolean,
        Field::Byte(_) | Field::Short(_) | Field::Int(_) | Field::Long(_) => FieldType::Integer,
        Field::UByte(_) | Field::UShort(_) | Field::UInt(_) | Field::ULong(_) => FieldType::Integer,
        Field::Float(_) | Field::Double(_) => FieldType::Float,
        Field::Str(_) => FieldType::Text,
        Field::Bytes(_) => FieldType::Blob,
        _ => FieldType::Text,
    }
}

fn field_to_value(field: &Field) -> FieldValue {
    match field {
        Field::Null => FieldValue::Null,
        Field::Bool(v) => FieldValue::Boolean(*v),
        Field::Byte(v) => FieldValue::Integer(*v as i64),
        Field::Short(v) => FieldValue::Integer(*v as i64),
        Field::Int(v) => FieldValue::Integer(*v as i64),
        Field::Long(v) => FieldValue::Integer(*v),
        Field::UByte(v) => FieldValue::Integer(*v as i64),
        Field::UShort(v) => FieldValue::Integer(*v as i64),
        Field::UInt(v) => FieldValue::Integer(*v as i64),
        Field::ULong(v) => FieldValue::Integer(*v as i64),
        Field::Float(v) => FieldValue::Float(*v as f64),
        Field::Double(v) => FieldValue::Float(*v),
        Field::Str(v) => FieldValue::Text(v.clone()),
        Field::Bytes(v) => FieldValue::Blob(v.data().to_vec()),
        other => FieldValue::Text(format!("{other:?}")),
    }
}

fn field_to_value_with_hint(field: &Field, hint: FieldType) -> FieldValue {
    let base = field_to_value(field);
    match (hint, base) {
        (FieldType::Date, FieldValue::Text(s)) => FieldValue::Date(s),
        (FieldType::DateTime, FieldValue::Text(s)) => FieldValue::DateTime(s),
        (_, v) => v,
    }
}

fn build_wbvector_field_types_metadata(layer: &Layer) -> Value {
    let mut obj = serde_json::Map::new();
    for f in layer.schema.fields() {
        obj.insert(f.name.clone(), Value::String(f.field_type.as_str().to_owned()));
    }
    Value::Object(obj)
}

fn parse_wbvector_field_types(kv: Option<&[parquet::format::KeyValue]>) -> Result<HashMap<String, FieldType>> {
    let Some(kv_pairs) = kv else {
        return Ok(HashMap::new());
    };

    let raw = kv_pairs
        .iter()
        .find(|p| p.key == WBVECTOR_FIELD_TYPES_KEY)
        .and_then(|p| p.value.clone());

    let Some(raw) = raw else {
        return Ok(HashMap::new());
    };

    let v: Value = serde_json::from_str(&raw).map_err(|e| {
        GeoError::GeoParquet(format!(
            "invalid '{}' metadata JSON: {e}",
            WBVECTOR_FIELD_TYPES_KEY
        ))
    })?;

    let Some(obj) = v.as_object() else {
        return Ok(HashMap::new());
    };

    let mut out = HashMap::new();
    for (name, value) in obj {
        if let Some(type_name) = value.as_str().and_then(parse_field_type_name) {
            out.insert(name.clone(), type_name);
        }
    }
    Ok(out)
}

fn parse_field_type_name(s: &str) -> Option<FieldType> {
    match s {
        "Integer" => Some(FieldType::Integer),
        "Float" => Some(FieldType::Float),
        "Text" => Some(FieldType::Text),
        "Boolean" => Some(FieldType::Boolean),
        "Blob" => Some(FieldType::Blob),
        "Date" => Some(FieldType::Date),
        "DateTime" => Some(FieldType::DateTime),
        "Json" => Some(FieldType::Json),
        _ => None,
    }
}

fn infer_types_from_schema(
    file_meta: &parquet::file::metadata::FileMetaData,
    geom_col: &str,
) -> HashMap<String, FieldType> {
    let mut out = HashMap::new();
    for col in file_meta.schema_descr().columns() {
        let name = col.name();
        if name == geom_col {
            continue;
        }
        out.insert(name.to_owned(), infer_field_type_from_column(col));
    }
    out
}

fn infer_field_type_from_column(col: &parquet::schema::types::ColumnDescPtr) -> FieldType {
    match col.converted_type() {
        ConvertedType::JSON => FieldType::Json,
        ConvertedType::UTF8 => FieldType::Text,
        ConvertedType::DATE => FieldType::Date,
        _ => match col.physical_type() {
            PhysicalType::BOOLEAN => FieldType::Boolean,
            PhysicalType::INT32 | PhysicalType::INT64 => FieldType::Integer,
            PhysicalType::FLOAT | PhysicalType::DOUBLE => FieldType::Float,
            PhysicalType::BYTE_ARRAY | PhysicalType::FIXED_LEN_BYTE_ARRAY => FieldType::Blob,
            _ => FieldType::Text,
        },
    }
}

#[derive(Debug, Default)]
struct GeoMeta {
    primary_column: Option<String>,
    epsg: Option<u32>,
    wkt: Option<String>,
}

fn parse_geo_metadata(kv: Option<&[parquet::format::KeyValue]>) -> Result<GeoMeta> {
    let Some(kv_pairs) = kv else {
        return Ok(GeoMeta::default());
    };

    let geo_json = kv_pairs
        .iter()
        .find(|p| p.key == "geo")
        .and_then(|p| p.value.clone());

    let Some(raw) = geo_json else {
        return Ok(GeoMeta::default());
    };

    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| GeoError::GeoParquet(format!("invalid 'geo' metadata JSON: {e}")))?;

    let mut meta = GeoMeta::default();
    meta.primary_column = v
        .get("primary_column")
        .and_then(|x| x.as_str())
        .map(ToOwned::to_owned);

    if let Some(pc) = meta.primary_column.clone() {
        if let Some(col) = v.get("columns").and_then(|c| c.get(&pc)) {
            parse_crs_hint(col, &mut meta);
        }
    }

    Ok(meta)
}

fn parse_crs_hint(col_meta: &Value, out: &mut GeoMeta) {
    let Some(crs_v) = col_meta.get("crs") else {
        return;
    };

    if let Some(s) = crs_v.as_str() {
        out.epsg = crs::epsg_from_srs_reference(s);
        if out.epsg.is_none() {
            out.epsg = crs::epsg_from_wkt_lenient(s);
            if out.epsg.is_none() {
                out.wkt = Some(s.to_owned());
            }
        }
        return;
    }

    if let Some(obj) = crs_v.as_object() {
        if let Some(wkt) = obj.get("wkt").and_then(|x| x.as_str()) {
            out.wkt = Some(wkt.to_owned());
            out.epsg = crs::epsg_from_wkt_lenient(wkt);
            return;
        }

        if let Some(id) = obj.get("id") {
            let authority = id
                .get("authority")
                .and_then(|a| a.as_str())
                .unwrap_or_default();
            let code = id.get("code").and_then(|c| c.as_u64());
            if authority.eq_ignore_ascii_case("EPSG") {
                out.epsg = code.map(|c| c as u32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::Arc;

    use parquet::basic::Type as PhysicalType;
    use parquet::data_type::{ByteArray, ByteArrayType};
    use parquet::file::reader::{FileReader, SerializedFileReader};
    use parquet::file::writer::SerializedFileWriter;
    use parquet::schema::types::Type;
    use tempfile::tempdir;

    use crate::feature::{FieldType, FieldValue};
    use crate::geometry::{Geometry, GeometryType};

    #[test]
    fn parses_geo_metadata_primary_col_and_epsg() {
        let kv = vec![parquet::format::KeyValue {
            key: "geo".to_owned(),
            value: Some(
                r#"{"primary_column":"geometry","columns":{"geometry":{"crs":{"id":{"authority":"EPSG","code":4326}}}}}"#
                    .to_owned(),
            ),
        }];

        let meta = parse_geo_metadata(Some(&kv)).unwrap();
        assert_eq!(meta.primary_column.as_deref(), Some("geometry"));
        assert_eq!(meta.epsg, Some(4326));
    }

    #[test]
    fn write_then_read_roundtrip() {
        let mut layer = Layer::new("roundtrip")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.add_field(FieldDef::new("count", FieldType::Integer));
        layer.add_field(FieldDef::new("weight", FieldType::Float));
        layer.add_field(FieldDef::new("enabled", FieldType::Boolean));
        layer.add_field(FieldDef::new("payload", FieldType::Blob));
        layer.add_field(FieldDef::new("obs_date", FieldType::Date));
        layer.add_field(FieldDef::new("obs_dt", FieldType::DateTime));
        layer.add_field(FieldDef::new("meta", FieldType::Json));

        layer
            .add_feature(
                Some(Geometry::point(-0.1278, 51.5074)),
                &[
                    ("name", FieldValue::Text("London".to_owned())),
                    ("count", FieldValue::Integer(7)),
                    ("weight", FieldValue::Float(2.5)),
                    ("enabled", FieldValue::Boolean(true)),
                    ("payload", FieldValue::Blob(vec![1, 2, 3])),
                    ("obs_date", FieldValue::Date("2026-03-12".to_owned())),
                    (
                        "obs_dt",
                        FieldValue::DateTime("2026-03-12T10:15:30Z".to_owned()),
                    ),
                    ("meta", FieldValue::Text(r#"{"src":"sensor"}"#.to_owned())),
                ],
            )
            .unwrap();

        layer
            .add_feature(
                None,
                &[
                    ("name", FieldValue::Null),
                    ("count", FieldValue::Null),
                    ("weight", FieldValue::Null),
                    ("enabled", FieldValue::Null),
                    ("payload", FieldValue::Null),
                    ("obs_date", FieldValue::Null),
                    ("obs_dt", FieldValue::Null),
                    ("meta", FieldValue::Null),
                ],
            )
            .unwrap();

        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.parquet");

        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();

        assert_eq!(out.len(), 2);
        assert_eq!(out.crs_epsg(), Some(4326));
        assert_eq!(out.schema.len(), 8);
        assert_eq!(out.schema.fields()[5].field_type, FieldType::Date);
        assert_eq!(out.schema.fields()[6].field_type, FieldType::DateTime);
        assert_eq!(out.schema.fields()[7].field_type, FieldType::Json);

        assert!(out.features[0].geometry.is_some());
        assert!(out.features[1].geometry.is_none());
        assert_eq!(out.features[0].attributes[0], FieldValue::Text("London".to_owned()));
        assert_eq!(out.features[0].attributes[1], FieldValue::Integer(7));
        assert_eq!(out.features[0].attributes[2], FieldValue::Float(2.5));
        assert_eq!(out.features[0].attributes[3], FieldValue::Boolean(true));
        assert_eq!(out.features[0].attributes[4], FieldValue::Blob(vec![1, 2, 3]));
        assert_eq!(out.features[0].attributes[5], FieldValue::Date("2026-03-12".to_owned()));
        assert_eq!(
            out.features[0].attributes[6],
            FieldValue::DateTime("2026-03-12T10:15:30Z".to_owned())
        );
        assert_eq!(
            out.features[0].attributes[7],
            FieldValue::Text(r#"{"src":"sensor"}"#.to_owned())
        );
    }

    #[test]
    fn infers_json_type_from_schema_without_wbvector_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("external_like.parquet");

        let schema = Arc::new(
            Type::group_type_builder("schema")
                .with_fields(vec![
                    Arc::new(
                        Type::primitive_type_builder("geometry", PhysicalType::BYTE_ARRAY)
                            .build()
                            .unwrap(),
                    ),
                    Arc::new(
                        Type::primitive_type_builder("props", PhysicalType::BYTE_ARRAY)
                            .with_converted_type(ConvertedType::JSON)
                            .build()
                            .unwrap(),
                    ),
                ])
                .build()
                .unwrap(),
        );

        let file = File::create(&path).unwrap();
        let mut writer = SerializedFileWriter::new(file, schema, Default::default()).unwrap();
        writer.append_key_value_metadata(KeyValue {
            key: "geo".to_owned(),
            value: Some(
                r#"{"primary_column":"geometry","columns":{"geometry":{"encoding":"WKB","geometry_types":["Point"]}}}"#
                    .to_owned(),
            ),
        });

        let mut row_group = writer.next_row_group().unwrap();

        {
            let mut col = row_group.next_column().unwrap().unwrap();
            let geom_wkb = Geometry::point(1.0, 2.0).to_wkb();
            let values = vec![ByteArray::from(geom_wkb)];
            let defs = vec![1i16];
            col.typed::<ByteArrayType>()
                .write_batch(&values, Some(&defs), None)
                .unwrap();
            col.close().unwrap();
        }

        {
            let mut col = row_group.next_column().unwrap().unwrap();
            let values = vec![ByteArray::from(r#"{"name":"abc"}"#)];
            let defs = vec![1i16];
            col.typed::<ByteArrayType>()
                .write_batch(&values, Some(&defs), None)
                .unwrap();
            col.close().unwrap();
        }

        row_group.close().unwrap();
        writer.close().unwrap();

        let layer = read(&path).unwrap();
        assert_eq!(layer.schema.len(), 1);
        assert_eq!(layer.schema.fields()[0].name, "props");
        assert_eq!(layer.schema.fields()[0].field_type, FieldType::Json);
    }

    #[test]
    fn reads_external_style_primary_geometry_column_and_crs_string() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("external_geom_col.parquet");

        let schema = Arc::new(
            Type::group_type_builder("schema")
                .with_fields(vec![
                    Arc::new(
                        Type::primitive_type_builder("geom", PhysicalType::BYTE_ARRAY)
                            .build()
                            .unwrap(),
                    ),
                    Arc::new(
                        Type::primitive_type_builder("name", PhysicalType::BYTE_ARRAY)
                            .with_converted_type(ConvertedType::UTF8)
                            .build()
                            .unwrap(),
                    ),
                ])
                .build()
                .unwrap(),
        );

        let file = File::create(&path).unwrap();
        let mut writer = SerializedFileWriter::new(file, schema, Default::default()).unwrap();
        writer.append_key_value_metadata(KeyValue {
            key: "geo".to_owned(),
            value: Some(
                r#"{"primary_column":"geom","columns":{"geom":{"encoding":"WKB","geometry_types":["Point"],"crs":"EPSG:3857"}}}"#
                    .to_owned(),
            ),
        });

        let mut row_group = writer.next_row_group().unwrap();

        {
            let mut col = row_group.next_column().unwrap().unwrap();
            let geom_wkb = Geometry::point(1000.0, 2000.0).to_wkb();
            let values = vec![ByteArray::from(geom_wkb)];
            let defs = vec![1i16];
            col.typed::<ByteArrayType>()
                .write_batch(&values, Some(&defs), None)
                .unwrap();
            col.close().unwrap();
        }

        {
            let mut col = row_group.next_column().unwrap().unwrap();
            let values = vec![ByteArray::from("site-a")];
            let defs = vec![1i16];
            col.typed::<ByteArrayType>()
                .write_batch(&values, Some(&defs), None)
                .unwrap();
            col.close().unwrap();
        }

        row_group.close().unwrap();
        writer.close().unwrap();

        let layer = read(&path).unwrap();
        assert_eq!(layer.crs_epsg(), Some(3857));
        assert_eq!(layer.len(), 1);
        assert_eq!(layer.schema.len(), 1);
        assert_eq!(layer.schema.fields()[0].name, "name");
        assert_eq!(layer.features[0].geometry, Some(Geometry::point(1000.0, 2000.0)));
    }

    #[test]
    fn reads_external_style_crs_wkt_metadata() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("external_crs_wkt.parquet");

        let schema = Arc::new(
            Type::group_type_builder("schema")
                .with_fields(vec![Arc::new(
                    Type::primitive_type_builder("geometry", PhysicalType::BYTE_ARRAY)
                        .build()
                        .unwrap(),
                )])
                .build()
                .unwrap(),
        );

        let file = File::create(&path).unwrap();
        let mut writer = SerializedFileWriter::new(file, schema, Default::default()).unwrap();
        let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433]]";
        writer.append_key_value_metadata(KeyValue {
            key: "geo".to_owned(),
            value: Some(
                format!(
                    r#"{{"primary_column":"geometry","columns":{{"geometry":{{"encoding":"WKB","geometry_types":["Point"],"crs":{{"wkt":"{}"}}}}}}}}"#,
                    wkt.replace('"', "\\\"")
                ),
            ),
        });

        let mut row_group = writer.next_row_group().unwrap();
        let mut col = row_group.next_column().unwrap().unwrap();
        let geom_wkb = Geometry::point(-75.0, 45.0).to_wkb();
        let values = vec![ByteArray::from(geom_wkb)];
        let defs = vec![1i16];
        col.typed::<ByteArrayType>()
            .write_batch(&values, Some(&defs), None)
            .unwrap();
        col.close().unwrap();

        row_group.close().unwrap();
        writer.close().unwrap();

        let layer = read(&path).unwrap();
        assert_eq!(layer.len(), 1);
        assert!(layer.crs_wkt().is_some());
        assert!(layer
            .crs_wkt()
            .unwrap_or_default()
            .to_ascii_uppercase()
            .contains("WGS 84"));
    }

    #[test]
    fn write_with_options_splits_row_groups() {
        let mut layer = Layer::new("rg")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));

        for i in 0..3 {
            layer
                .add_feature(
                    Some(Geometry::point(i as f64, i as f64)),
                    &[("name", FieldValue::Text(format!("p{i}")))],
                )
                .unwrap();
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("rg_split.parquet");
        let options = GeoParquetWriteOptions::new().with_max_rows_per_group(1);

        write_with_options(&layer, &path, &options).unwrap();
        let out = read(&path).unwrap();
        assert_eq!(out.len(), 3);

        let file = File::open(&path).unwrap();
        let reader = SerializedFileReader::new(file).unwrap();
        assert_eq!(reader.metadata().num_row_groups(), 3);
    }

    #[test]
    fn write_with_options_applies_compression() {
        let mut layer = Layer::new("cmp")
            .with_geom_type(GeometryType::Point)
            .with_crs_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer
            .add_feature(
                Some(Geometry::point(0.0, 0.0)),
                &[("name", FieldValue::Text("a".to_owned()))],
            )
            .unwrap();

        let dir = tempdir().unwrap();
        let path = dir.path().join("cmp.parquet");
        let options = GeoParquetWriteOptions::new().with_compression(Compression::SNAPPY);

        write_with_options(&layer, &path, &options).unwrap();

        let file = File::open(&path).unwrap();
        let reader = SerializedFileReader::new(file).unwrap();
        let row_group = reader.metadata().row_group(0);
        assert_eq!(row_group.column(0).compression(), Compression::SNAPPY);
        assert_eq!(row_group.column(1).compression(), Compression::SNAPPY);
    }

    #[test]
    fn large_file_preset_sets_expected_values() {
        let options = GeoParquetWriteOptions::for_large_files();
        assert_eq!(options.max_rows_per_group, 1_000_000);
        assert_eq!(options.data_page_size_limit, 2 * 1024 * 1024);
        assert_eq!(options.write_batch_size, 8_192);
        assert_eq!(options.data_page_row_count_limit, 50_000);
        assert_eq!(options.compression, Compression::SNAPPY);
    }

    #[test]
    fn interactive_file_preset_sets_expected_values() {
        let options = GeoParquetWriteOptions::for_interactive_files();
        assert_eq!(options.max_rows_per_group, 100_000);
        assert_eq!(
            options.data_page_size_limit,
            parquet::file::properties::DEFAULT_PAGE_SIZE
        );
        assert_eq!(options.write_batch_size, 2_048);
        assert_eq!(
            options.data_page_row_count_limit,
            parquet::file::properties::DEFAULT_DATA_PAGE_ROW_COUNT_LIMIT
        );
        assert_eq!(options.compression, Compression::UNCOMPRESSED);
    }
}
