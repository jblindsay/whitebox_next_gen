use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbgeotiff::{ifd::{IfdValue, TiffReader}, tags::tag, GeoTiff};
use wbraster::{CrsInfo, DataType, Raster, RasterConfig, RasterFormat};
use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, GeometryType, Layer, Ring, VectorFormat};
use wbtopology::{
    from_wkb as topology_from_wkb, to_wkb as topology_to_wkb, Geometry as TopologyGeometry,
    Polygon as TopologyPolygon,
};

use crate::memory_store;

pub struct AddPointCoordinatesToTableTool;
pub struct CleanVectorTool;
pub struct ConvertNodataToZeroTool;
pub struct CsvPointsToVectorTool;
pub struct ExportTableToCsvTool;
pub struct FixDanglingArcsTool;
pub struct JoinTablesTool;
pub struct LinesToPolygonsTool;
pub struct ModifyNodataValueTool;
pub struct MergeTableWithCsvTool;
pub struct NewRasterFromBaseRasterTool;
pub struct NewRasterFromBaseVectorTool;
pub struct PolygonsToLinesTool;
pub struct PrintGeotiffTagsTool;
pub struct ReinitializeAttributeTableTool;
pub struct RasterToVectorPointsTool;
pub struct RemovePolygonHolesTool;
pub struct RemoveRasterPolygonHolesTool;
pub struct SetNodataValueTool;
pub struct RasterToVectorLinesTool;
pub struct RasterToVectorPolygonsTool;
pub struct VectorPointsToRasterTool;
pub struct VectorLinesToRasterTool;
pub struct VectorPolygonsToRasterTool;
pub struct MergeVectorsTool;
pub struct MultipartToSinglepartTool;
pub struct SinglepartToMultipartTool;

fn derived_vector_output_path(input: &str, suffix: &str) -> PathBuf {
    let input = Path::new(input);
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("layer");
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    parent.join(format!("{stem}_{suffix}_{millis}.geojson"))
}

fn ensure_parent_dir(path: &Path) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
        }
    }
    Ok(())
}

fn write_raster_output(
    raster: Raster,
    output_path: Option<PathBuf>,
    ctx: &ToolContext,
) -> Result<ToolRunResult, ToolError> {
    let output_locator = if let Some(output_path) = output_path {
        ensure_parent_dir(&output_path)?;
        let output_path_str = output_path.to_string_lossy().to_string();
        let output_format = RasterFormat::for_output_path(&output_path_str)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
        ctx.progress.info("writing output raster");
        raster
            .write(&output_path_str, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
        output_path_str
    } else {
        ctx.progress.info("storing output raster in memory");
        let id = memory_store::put_raster(raster);
        memory_store::make_raster_memory_path(&id)
    };

    let mut outputs = BTreeMap::new();
    outputs.insert("__wbw_type__".to_string(), json!("raster"));
    outputs.insert("path".to_string(), json!(output_locator));
    outputs.insert("active_band".to_string(), json!(0));
    Ok(ToolRunResult { outputs })
}

fn write_vector_output(layer: &Layer, output_path: &Path) -> Result<ToolRunResult, ToolError> {
    ensure_parent_dir(output_path)?;
    let format = VectorFormat::detect(output_path)
        .map_err(|e| ToolError::Validation(format!("unsupported vector output path: {e}")))?;
    wbvector::write(layer, output_path, format)
        .map_err(|e| ToolError::Execution(format!("failed writing output vector: {e}")))?;

    let mut outputs = BTreeMap::new();
    outputs.insert("__wbw_type__".to_string(), json!("vector"));
    outputs.insert(
        "path".to_string(),
        json!(output_path.to_string_lossy().to_string()),
    );
    Ok(ToolRunResult { outputs })
}

fn read_vector_layer(path: &str, label: &str) -> Result<Layer, ToolError> {
    wbvector::read(path).map_err(|e| {
        ToolError::Validation(format!("failed reading {label} vector '{path}': {e}"))
    })
}

fn write_string_output(key: &str, value: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert(key.to_string(), json!(value));
    ToolRunResult { outputs }
}

fn apply_input_crs_to_layer(input: &Layer, output: &mut Layer) {
    if let Some(epsg) = input.crs_epsg() {
        output.set_crs_epsg(Some(epsg));
    }
    if let Some(wkt) = input.crs_wkt() {
        output.set_crs_wkt(Some(wkt.to_string()));
    }
}

fn apply_raster_crs_to_layer(input: &Raster, output: &mut Layer) {
    if let Some(epsg) = input.crs.epsg {
        output.set_crs_epsg(Some(epsg));
    }
    if let Some(wkt) = &input.crs.wkt {
        output.set_crs_wkt(Some(wkt.clone()));
    }
}

fn clone_feature_attrs<'a>(layer: &'a Layer, feature: &'a wbvector::Feature) -> Vec<(&'a str, FieldValue)> {
    layer
        .schema
        .fields()
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            (
                field.name.as_str(),
                feature.attributes.get(idx).cloned().unwrap_or(FieldValue::Null),
            )
        })
        .collect()
}

fn close_ring(coords: &[wbvector::Coord]) -> Vec<wbvector::Coord> {
    let mut ring = coords.to_vec();
    if let (Some(first), Some(last)) = (ring.first().cloned(), ring.last()) {
        if first != *last {
            ring.push(first);
        }
    }
    ring
}

fn strip_polygon_holes_with_topology(geometry: &Geometry) -> Result<Geometry, ToolError> {
    let topo_geom = topology_from_wkb(&geometry.to_wkb())
        .map_err(|e| ToolError::Execution(format!("failed converting geometry for topology processing: {e}")))?;

    let stripped = match topo_geom {
        TopologyGeometry::Polygon(poly) => {
            TopologyGeometry::Polygon(TopologyPolygon::new(poly.exterior, Vec::new()))
        }
        TopologyGeometry::MultiPolygon(polys) => TopologyGeometry::MultiPolygon(
            polys
                .into_iter()
                .map(|poly| TopologyPolygon::new(poly.exterior, Vec::new()))
                .collect(),
        ),
        _ => {
            return Err(ToolError::Validation(
                "input vector layer must contain polygon geometries".to_string(),
            ));
        }
    };

    Geometry::from_wkb(&topology_to_wkb(&stripped)).map_err(|e| {
        ToolError::Execution(format!("failed converting topology geometry back to vector geometry: {e}"))
    })
}

fn tiff_tag_name(tag_code: u16) -> &'static str {
    match tag_code {
        tag::NewSubFileType => "NewSubFileType",
        tag::SubFileType => "SubFileType",
        tag::ImageWidth => "ImageWidth",
        tag::ImageLength => "ImageLength",
        tag::BitsPerSample => "BitsPerSample",
        tag::Compression => "Compression",
        tag::PhotometricInterpretation => "PhotometricInterpretation",
        tag::StripOffsets => "StripOffsets",
        tag::SamplesPerPixel => "SamplesPerPixel",
        tag::RowsPerStrip => "RowsPerStrip",
        tag::StripByteCounts => "StripByteCounts",
        tag::XResolution => "XResolution",
        tag::YResolution => "YResolution",
        tag::PlanarConfiguration => "PlanarConfiguration",
        tag::ResolutionUnit => "ResolutionUnit",
        tag::Software => "Software",
        tag::DateTime => "DateTime",
        tag::ExtraSamples => "ExtraSamples",
        tag::SampleFormat => "SampleFormat",
        tag::TileWidth => "TileWidth",
        tag::TileLength => "TileLength",
        tag::TileOffsets => "TileOffsets",
        tag::TileByteCounts => "TileByteCounts",
        tag::ModelPixelScaleTag => "ModelPixelScaleTag",
        tag::ModelTiepointTag => "ModelTiepointTag",
        tag::ModelTransformationTag => "ModelTransformationTag",
        tag::GeoKeyDirectoryTag => "GeoKeyDirectoryTag",
        tag::GeoDoubleParamsTag => "GeoDoubleParamsTag",
        tag::GeoAsciiParamsTag => "GeoAsciiParamsTag",
        tag::GdalMetadata => "GdalMetadata",
        tag::GdalNodata => "GdalNodata",
        _ => "UnknownTag",
    }
}

fn preview_values<T>(values: &[T]) -> String
where
    T: std::fmt::Display,
{
    const LIMIT: usize = 8;
    let mut preview = values
        .iter()
        .take(LIMIT)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > LIMIT {
        preview.push_str(&format!(", ... ({} total)", values.len()));
    }
    preview
}

fn preview_pairs<T>(values: &[(T, T)]) -> String
where
    T: std::fmt::Display,
{
    const LIMIT: usize = 6;
    let mut preview = values
        .iter()
        .take(LIMIT)
        .map(|(a, b)| format!("({a}, {b})"))
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > LIMIT {
        preview.push_str(&format!(", ... ({} total)", values.len()));
    }
    preview
}

fn format_ifd_value(value: &IfdValue) -> String {
    match value {
        IfdValue::Bytes(values) => preview_values(values),
        IfdValue::Shorts(values) => preview_values(values),
        IfdValue::Longs(values) => preview_values(values),
        IfdValue::Long8s(values) => preview_values(values),
        IfdValue::Rationals(values) => preview_pairs(values),
        IfdValue::SBytes(values) => preview_values(values),
        IfdValue::SShorts(values) => preview_values(values),
        IfdValue::SLongs(values) => preview_values(values),
        IfdValue::SLong8s(values) => preview_values(values),
        IfdValue::SRationals(values) => preview_pairs(values),
        IfdValue::Floats(values) => preview_values(values),
        IfdValue::Doubles(values) => preview_values(values),
        IfdValue::Ascii(value) => value.clone(),
        IfdValue::Undefined(values) => preview_values(values),
    }
}

fn looks_like_tiff_family(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".tif")
        || lower.ends_with(".tiff")
        || lower.ends_with(".gtif")
        || lower.ends_with(".gtiff")
        || lower.ends_with(".cog.tif")
        || lower.ends_with(".cog.tiff")
}

fn build_geotiff_tag_report(input_path: &str) -> Result<String, ToolError> {
    let tiff = GeoTiff::open(input_path)
        .map_err(|e| ToolError::Execution(format!("failed reading GeoTIFF metadata: {e}")))?;

    let file = File::open(input_path)
        .map_err(|e| ToolError::Execution(format!("failed opening input TIFF: {e}")))?;
    let mut reader = TiffReader::new(BufReader::new(file))
        .map_err(|e| ToolError::Execution(format!("failed parsing TIFF header: {e}")))?;
    let ifds = reader
        .read_all_ifds()
        .map_err(|e| ToolError::Execution(format!("failed reading TIFF directories: {e}")))?;

    let mut report = String::new();
    report.push_str("GeoTIFF Tag Report\n");
    report.push_str("==================\n");
    report.push_str(&format!("Input: {input_path}\n"));
    report.push_str(&format!(
        "Variant: {}\n",
        if tiff.is_bigtiff { "BigTIFF" } else { "Classic TIFF" }
    ));
    report.push_str(&format!("Dimensions: {} x {}\n", tiff.width(), tiff.height()));
    report.push_str(&format!("Bands: {}\n", tiff.band_count()));
    report.push_str(&format!("Bits per sample: {}\n", tiff.bits_per_sample()));
    report.push_str(&format!("Sample format: {:?}\n", tiff.sample_format()));
    report.push_str(&format!("Compression: {}\n", tiff.compression().name()));
    report.push_str(&format!("Photometric: {:?}\n", tiff.photometric()));
    report.push_str(&format!(
        "NoData: {}\n",
        tiff.no_data()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not set".to_string())
    ));
    report.push_str(&format!(
        "EPSG: {}\n",
        tiff.epsg()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    if let Some(transform) = tiff.geo_transform() {
        report.push_str(&format!(
            "GeoTransform: [{}, {}, {}, {}, {}, {}]\n",
            transform.x_origin,
            transform.pixel_width,
            transform.row_rotation,
            transform.y_origin,
            transform.col_rotation,
            transform.pixel_height,
        ));
    }
    report.push_str("\nTIFF directories\n");
    report.push_str("----------------\n");

    for (ifd_index, ifd) in ifds.iter().enumerate() {
        report.push_str(&format!("IFD {ifd_index}: {} entries\n", ifd.entries.len()));
        for entry in &ifd.entries {
            report.push_str(&format!(
                "  {} ({}) [{} x {}] = {}\n",
                tiff_tag_name(entry.tag),
                entry.tag,
                format!("{:?}", entry.data_type),
                entry.count,
                format_ifd_value(&entry.value),
            ));
        }
        if ifd.next_ifd_offset != 0 {
            report.push_str(&format!("  next_ifd_offset = {}\n", ifd.next_ifd_offset));
        }
        report.push('\n');
    }

    Ok(report)
}

fn parse_optional_f64(args: &ToolArgs, key: &str) -> Result<Option<f64>, ToolError> {
    args.get(key)
        .map(|value| {
            value.as_f64().ok_or_else(|| {
                ToolError::Validation(format!("parameter '{key}' must be a number when provided"))
            })
        })
        .transpose()
}

fn parse_optional_usize(args: &ToolArgs, key: &str) -> Result<Option<usize>, ToolError> {
    args.get(key)
        .map(|value| {
            if let Some(v) = value.as_u64() {
                return usize::try_from(v).map_err(|_| {
                    ToolError::Validation(format!(
                        "parameter '{key}' is too large for this platform"
                    ))
                });
            }
            if let Some(v) = value.as_i64() {
                if v < 0 {
                    return Err(ToolError::Validation(format!(
                        "parameter '{key}' must be non-negative when provided"
                    )));
                }
                return usize::try_from(v as u64).map_err(|_| {
                    ToolError::Validation(format!(
                        "parameter '{key}' is too large for this platform"
                    ))
                });
            }
            Err(ToolError::Validation(format!(
                "parameter '{key}' must be an integer when provided"
            )))
        })
        .transpose()
}

fn parse_optional_string<'a>(args: &'a ToolArgs, key: &str) -> Result<Option<&'a str>, ToolError> {
    args.get(key)
        .map(|value| {
            value.as_str().ok_or_else(|| {
                ToolError::Validation(format!("parameter '{key}' must be a string when provided"))
            })
        })
        .transpose()
}

fn parse_vector_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{key}' is required")))?;
    if let Some(s) = value.as_str() {
        let out: Vec<String> = s
            .split(|c: char| c == ',' || c == ';')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect();
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{key}' did not contain any vector paths"
            )));
        }
        return Ok(out);
    }
    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, v) in arr.iter().enumerate() {
            let s = v.as_str().ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{key}' array element {i} must be a string path"
                ))
            })?;
            let s = s.trim();
            if s.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{key}' array element {i} is empty"
                )));
            }
            out.push(s.to_string());
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{key}' must contain at least one vector path"
            )));
        }
        return Ok(out);
    }
    Err(ToolError::Validation(format!(
        "parameter '{key}' must be a string or array of paths"
    )))
}

fn single_part_geom_type(gt: GeometryType) -> GeometryType {
    match gt {
        GeometryType::MultiPoint => GeometryType::Point,
        GeometryType::MultiLineString => GeometryType::LineString,
        GeometryType::MultiPolygon => GeometryType::Polygon,
        other => other,
    }
}

fn expand_to_single_part(geom: &Geometry, exclude_holes: bool) -> Vec<Geometry> {
    match geom {
        Geometry::Point(_) | Geometry::LineString(_) | Geometry::Polygon { .. } => {
            vec![geom.clone()]
        }
        Geometry::MultiPoint(coords) => coords
            .iter()
            .map(|c| Geometry::Point(c.clone()))
            .collect(),
        Geometry::MultiLineString(lines) => lines
            .iter()
            .map(|ls| Geometry::line_string(ls.clone()))
            .collect(),
        Geometry::MultiPolygon(polys) => {
            let mut out = Vec::new();
            for (exterior, interiors) in polys {
                if exclude_holes {
                    out.push(Geometry::Polygon {
                        exterior: exterior.clone(),
                        interiors: interiors.clone(),
                    });
                } else {
                    out.push(Geometry::Polygon {
                        exterior: exterior.clone(),
                        interiors: Vec::new(),
                    });
                    for hole in interiors {
                        out.push(Geometry::Polygon {
                            exterior: hole.clone(),
                            interiors: Vec::new(),
                        });
                    }
                }
            }
            out
        }
        Geometry::GeometryCollection(parts) => parts
            .iter()
            .flat_map(|g| expand_to_single_part(g, exclude_holes))
            .collect(),
    }
}

fn merge_to_multi(
    input: &Layer,
    feat_indices: &[usize],
    input_geom_type: GeometryType,
) -> Result<Geometry, ToolError> {
    match input_geom_type {
        GeometryType::Point => {
            let mut coords = Vec::new();
            for &i in feat_indices {
                if let Some(f) = input.features.get(i) {
                    if let Some(Geometry::Point(c)) = &f.geometry {
                        coords.push(c.clone());
                    }
                }
            }
            Ok(Geometry::MultiPoint(coords))
        }
        GeometryType::LineString => {
            let mut lines = Vec::new();
            for &i in feat_indices {
                if let Some(f) = input.features.get(i) {
                    if let Some(Geometry::LineString(cs)) = &f.geometry {
                        lines.push(cs.clone());
                    }
                }
            }
            Ok(Geometry::MultiLineString(lines))
        }
        GeometryType::Polygon => {
            let mut polys = Vec::new();
            for &i in feat_indices {
                if let Some(f) = input.features.get(i) {
                    if let Some(Geometry::Polygon { exterior, interiors }) = &f.geometry {
                        polys.push((exterior.clone(), interiors.clone()));
                    }
                }
            }
            Ok(Geometry::MultiPolygon(polys))
        }
        gt => Err(ToolError::Validation(format!(
            "unsupported input geometry type for singlepart_to_multipart: {gt}"
        ))),
    }
}

fn nodata_data_type_for_background(input_type: DataType, back_value: f64) -> DataType {
    if back_value >= 0.0 {
        return input_type;
    }
    match input_type {
        DataType::U64 | DataType::U32 => DataType::I64,
        DataType::U16 => DataType::I32,
        DataType::U8 => DataType::I16,
        _ => input_type,
    }
}

fn clean_geometry(geometry: &Geometry) -> Option<Geometry> {
    match geometry {
        Geometry::Point(_) => Some(geometry.clone()),
        Geometry::LineString(coords) => {
            if coords.len() >= 2 {
                Some(Geometry::line_string(coords.clone()))
            } else {
                None
            }
        }
        Geometry::Polygon { exterior, interiors } => {
            if exterior.0.len() < 3 {
                return None;
            }
            let cleaned_holes = interiors
                .iter()
                .filter(|ring| ring.0.len() >= 3)
                .map(|ring| ring.0.clone())
                .collect::<Vec<_>>();
            Some(Geometry::polygon(exterior.0.clone(), cleaned_holes))
        }
        Geometry::MultiPoint(points) => {
            if points.is_empty() {
                None
            } else {
                Some(Geometry::multi_point(points.clone()))
            }
        }
        Geometry::MultiLineString(lines) => {
            let cleaned_lines = lines
                .iter()
                .filter(|line| line.len() >= 2)
                .cloned()
                .collect::<Vec<_>>();
            if cleaned_lines.is_empty() {
                None
            } else {
                Some(Geometry::multi_line_string(cleaned_lines))
            }
        }
        Geometry::MultiPolygon(polygons) => {
            let cleaned_polygons = polygons
                .iter()
                .filter_map(|(exterior, interiors)| {
                    if exterior.0.len() < 3 {
                        return None;
                    }
                    let cleaned_holes = interiors
                        .iter()
                        .filter(|ring| ring.0.len() >= 3)
                        .map(|ring| ring.0.clone())
                        .collect::<Vec<_>>();
                    Some((exterior.0.clone(), cleaned_holes))
                })
                .collect::<Vec<_>>();
            if cleaned_polygons.is_empty() {
                None
            } else {
                Some(Geometry::multi_polygon(cleaned_polygons))
            }
        }
        Geometry::GeometryCollection(parts) => {
            let cleaned_parts = parts.iter().filter_map(clean_geometry).collect::<Vec<_>>();
            if cleaned_parts.is_empty() {
                None
            } else {
                Some(Geometry::GeometryCollection(cleaned_parts))
            }
        }
    }
}

fn field_value_as_f64(value: &FieldValue) -> Option<f64> {
    match value {
        FieldValue::Integer(v) => Some(*v as f64),
        FieldValue::Float(v) => Some(*v),
        _ => None,
    }
}

fn feature_points(geometry: &Geometry) -> Vec<(f64, f64)> {
    match geometry {
        Geometry::Point(c) => vec![(c.x, c.y)],
        Geometry::MultiPoint(coords) => coords.iter().map(|c| (c.x, c.y)).collect(),
        _ => Vec::new(),
    }
}

fn parse_assign_op(args: &ToolArgs) -> String {
    args.get("assign")
        .and_then(|v| v.as_str())
        .unwrap_or("last")
        .trim()
        .to_ascii_lowercase()
}

fn vector_to_raster_crs(layer: &Layer) -> CrsInfo {
    let mut crs = CrsInfo::default();
    if let Some(epsg) = layer.crs_epsg() {
        crs.epsg = Some(epsg);
    }
    if let Some(wkt) = layer.crs_wkt() {
        crs.wkt = Some(wkt.to_string());
    }
    crs
}

fn geometry_line_parts(geometry: &Geometry, out: &mut Vec<Vec<Coord>>) {
    match geometry {
        Geometry::LineString(coords) => {
            if coords.len() >= 2 {
                out.push(coords.clone());
            }
        }
        Geometry::MultiLineString(lines) => {
            for line in lines {
                if line.len() >= 2 {
                    out.push(line.clone());
                }
            }
        }
        Geometry::Polygon { exterior, interiors } => {
            if exterior.0.len() >= 2 {
                out.push(exterior.0.clone());
            }
            for ring in interiors {
                if ring.0.len() >= 2 {
                    out.push(ring.0.clone());
                }
            }
        }
        Geometry::MultiPolygon(polys) => {
            for (exterior, interiors) in polys {
                if exterior.0.len() >= 2 {
                    out.push(exterior.0.clone());
                }
                for ring in interiors {
                    if ring.0.len() >= 2 {
                        out.push(ring.0.clone());
                    }
                }
            }
        }
        Geometry::GeometryCollection(parts) => {
            for g in parts {
                geometry_line_parts(g, out);
            }
        }
        Geometry::Point(_) | Geometry::MultiPoint(_) => {}
    }
}

fn coord_distance(a: &Coord, b: &Coord) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn project_point_to_segment(point: &Coord, start: &Coord, end: &Coord) -> (Coord, f64) {
    let vx = end.x - start.x;
    let vy = end.y - start.y;
    let wx = point.x - start.x;
    let wy = point.y - start.y;
    let vv = vx * vx + vy * vy;
    if vv <= f64::EPSILON {
        let c = Coord::xy(start.x, start.y);
        return (c.clone(), coord_distance(&c, point));
    }

    let t = ((wx * vx + wy * vy) / vv).clamp(0.0, 1.0);
    let c = Coord::xy(start.x + t * vx, start.y + t * vy);
    (c.clone(), coord_distance(&c, point))
}

fn point_to_segment_distance(point: &Coord, start: &Coord, end: &Coord) -> f64 {
    let (_, dist) = project_point_to_segment(point, start, end);
    dist
}

fn segment_intersection_point(a1: &Coord, a2: &Coord, b1: &Coord, b2: &Coord, tol: f64) -> Option<Coord> {
    let x1 = a1.x;
    let y1 = a1.y;
    let x2 = a2.x;
    let y2 = a2.y;
    let x3 = b1.x;
    let y3 = b1.y;
    let x4 = b2.x;
    let y4 = b2.y;

    let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if den.abs() <= f64::EPSILON {
        return None;
    }

    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den;
    let u = ((x1 - x3) * (y1 - y2) - (y1 - y3) * (x1 - x2)) / den;
    if !(-tol..=1.0 + tol).contains(&t) || !(-tol..=1.0 + tol).contains(&u) {
        return None;
    }

    Some(Coord::xy(x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
}

fn dedupe_consecutive_coords(coords: &[Coord], tol: f64) -> Vec<Coord> {
    let mut out: Vec<Coord> = Vec::with_capacity(coords.len());
    for c in coords {
        if out
            .last()
            .map(|last| coord_distance(last, c) <= tol)
            .unwrap_or(false)
        {
            continue;
        }
        out.push(c.clone());
    }
    out
}

fn collect_line_parts(layer: &Layer) -> (Vec<Vec<Coord>>, Vec<Vec<usize>>) {
    let mut parts: Vec<Vec<Coord>> = Vec::new();
    let mut feature_part_ids: Vec<Vec<usize>> = vec![Vec::new(); layer.features.len()];

    for (feature_idx, feature) in layer.features.iter().enumerate() {
        match &feature.geometry {
            Some(Geometry::LineString(line)) if line.len() >= 2 => {
                let id = parts.len();
                parts.push(line.clone());
                feature_part_ids[feature_idx].push(id);
            }
            Some(Geometry::MultiLineString(lines)) => {
                for line in lines {
                    if line.len() >= 2 {
                        let id = parts.len();
                        parts.push(line.clone());
                        feature_part_ids[feature_idx].push(id);
                    }
                }
            }
            _ => {}
        }
    }

    (parts, feature_part_ids)
}

#[derive(Clone)]
struct SnapSegment {
    part_id: usize,
    start: Coord,
    end: Coord,
}

#[derive(Clone)]
struct ArcSnapCandidate {
    nearest: Coord,
    distance: f64,
    segment: SnapSegment,
}

fn find_best_snap_candidate(
    endpoint: &Coord,
    current_part_id: usize,
    segments: &[SnapSegment],
    snap_dist: f64,
) -> Option<ArcSnapCandidate> {
    let mut best: Option<ArcSnapCandidate> = None;
    for segment in segments {
        if segment.part_id == current_part_id {
            continue;
        }
        let (nearest, dist) = project_point_to_segment(endpoint, &segment.start, &segment.end);
        if dist > snap_dist {
            continue;
        }

        if best
            .as_ref()
            .map(|b| dist < b.distance)
            .unwrap_or(true)
        {
            best = Some(ArcSnapCandidate {
                nearest,
                distance: dist,
                segment: segment.clone(),
            });
        }
    }
    best
}

fn rasterize_segment(raster: &mut Raster, start: &Coord, end: &Coord, value: f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let sx = raster.cell_size_x.abs().max(f64::EPSILON);
    let sy = raster.cell_size_y.abs().max(f64::EPSILON);
    let steps = ((dx.abs() / sx).max(dy.abs() / sy).ceil() as usize)
        .saturating_mul(2)
        .max(1);
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = start.x + t * dx;
        let y = start.y + t * dy;
        if let Some((col, row)) = raster.world_to_pixel(x, y) {
            raster.set_unchecked(0, row, col, value);
        }
    }
}

fn ring_signed_area(points: &[(f64, f64)]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        area += points[i].0 * points[j].1 - points[j].0 * points[i].1;
    }
    area * 0.5
}

fn point_in_ring(pt: (f64, f64), ring: &[(f64, f64)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let (x, y) = pt;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        let intersects = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / (yj - yi + f64::EPSILON) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn normalize_ring(points: &[(f64, f64)]) -> Vec<Coord> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Coord> = points.iter().map(|(x, y)| Coord::xy(*x, *y)).collect();
    if out.len() >= 2 {
        let first = &out[0];
        let last = &out[out.len() - 1];
        if (first.x - last.x).abs() < 1e-12 && (first.y - last.y).abs() < 1e-12 {
            out.pop();
        }
    }
    out
}

fn detect_delimiter(line: &str) -> char {
    if line.contains(',') {
        ','
    } else if line.contains(';') {
        ';'
    } else if line.contains('\t') {
        '\t'
    } else {
        ' '
    }
}

fn split_line(line: &str, delimiter: char) -> Vec<String> {
    if delimiter == ' ' {
        line.split_whitespace().map(|s| s.trim().to_string()).collect()
    } else {
        line.split(delimiter)
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect()
    }
}

fn parse_csv_table(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), ToolError> {
    let file = File::open(path)
        .map_err(|e| ToolError::Execution(format!("failed opening csv file '{}': {e}", path)))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| ToolError::Validation("csv file is empty".to_string()))?
        .map_err(|e| ToolError::Execution(format!("failed reading csv header: {e}")))?;
    let delimiter = detect_delimiter(&header_line);
    let headers = split_line(&header_line, delimiter);
    if headers.is_empty() {
        return Err(ToolError::Validation("csv header has no fields".to_string()));
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in lines {
        let line = line
            .map_err(|e| ToolError::Execution(format!("failed reading csv line: {e}")))?;
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_line(&line, delimiter);
        if fields.len() != headers.len() {
            return Err(ToolError::Validation(format!(
                "csv row has {} fields but header has {}",
                fields.len(),
                headers.len()
            )));
        }
        rows.push(fields);
    }
    Ok((headers, rows))
}

fn infer_field_type(samples: &[String]) -> FieldType {
    let mut all_bool = true;
    let mut all_int = true;
    let mut all_float = true;
    for s in samples {
        if s.trim().is_empty() {
            continue;
        }
        if s.parse::<bool>().is_err() {
            all_bool = false;
        }
        if s.parse::<i64>().is_err() {
            all_int = false;
        }
        if s.parse::<f64>().is_err() {
            all_float = false;
        }
    }
    if all_int {
        FieldType::Integer
    } else if all_float {
        FieldType::Float
    } else if all_bool {
        FieldType::Boolean
    } else {
        FieldType::Text
    }
}

fn parse_typed_value(value: &str, field_type: FieldType) -> FieldValue {
    let s = value.trim();
    if s.is_empty() {
        return FieldValue::Null;
    }
    match field_type {
        FieldType::Integer => s
            .parse::<i64>()
            .map(FieldValue::Integer)
            .unwrap_or(FieldValue::Null),
        FieldType::Float => s
            .parse::<f64>()
            .map(FieldValue::Float)
            .unwrap_or(FieldValue::Null),
        FieldType::Boolean => s
            .parse::<bool>()
            .map(FieldValue::Boolean)
            .unwrap_or(FieldValue::Null),
        _ => FieldValue::Text(s.to_string()),
    }
}

fn field_value_to_csv(value: &FieldValue) -> String {
    match value {
        FieldValue::Null => "null".to_string(),
        FieldValue::Text(s) | FieldValue::Date(s) | FieldValue::DateTime(s) => {
            format!("\"{}\"", s.replace('"', "\"\""))
        }
        FieldValue::Boolean(v) => v.to_string(),
        FieldValue::Integer(v) => v.to_string(),
        FieldValue::Float(v) => v.to_string(),
        FieldValue::Blob(_) => "\"<blob>\"".to_string(),
    }
}

impl Tool for AddPointCoordinatesToTableTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "add_point_coordinates_to_table",
            display_name: "AddPointCoordinatesToTable",
            summary: "Copies a point layer and appends XCOORD and YCOORD attribute fields.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input point vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.geojson"));
        defaults.insert("output".to_string(), json!("points_with_coords.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("samples.gpkg"));
        example_args.insert("output".to_string(), json!("samples_with_coords.geojson"));

        ToolManifest {
            id: "add_point_coordinates_to_table".to_string(),
            display_name: "AddPointCoordinatesToTable".to_string(),
            summary: "Copies a point layer and appends XCOORD and YCOORD attribute fields.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input point vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Append X and Y coordinate fields to a point layer.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "attributes".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "coords"));

        ctx.progress.info("running add_point_coordinates_to_table");
        let input = read_vector_layer(&input_path, "input")?;
        if !matches!(input.geom_type, Some(GeometryType::Point)) {
            return Err(ToolError::Validation(
                "input vector layer must have Point geometry type".to_string(),
            ));
        }

        let mut output = Layer::new(input.name.clone()).with_geom_type(GeometryType::Point);
        apply_input_crs_to_layer(&input, &mut output);
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }
        output.add_field(FieldDef::new("XCOORD", FieldType::Float).width(18).precision(8));
        output.add_field(FieldDef::new("YCOORD", FieldType::Float).width(18).precision(8));

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feature_idx, feature) in input.features.iter().enumerate() {
            let (x, y) = match &feature.geometry {
                Some(Geometry::Point(coord)) => (coord.x, coord.y),
                Some(_) => {
                    return Err(ToolError::Validation(
                        "encountered non-point geometry while converting add_point_coordinates_to_table".to_string(),
                    ));
                }
                None => {
                    return Err(ToolError::Validation(
                        "point features must contain geometry".to_string(),
                    ));
                }
            };

            let mut attrs = clone_feature_attrs(&input, feature);
            attrs.push(("XCOORD", FieldValue::Float(x)));
            attrs.push(("YCOORD", FieldValue::Float(y)));
            output
                .add_feature(feature.geometry.clone(), &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            coalescer.emit_unit_fraction(ctx.progress, (feature_idx + 1) as f64 / total);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for CleanVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "clean_vector",
            display_name: "CleanVector",
            summary: "Removes null and invalid vector geometries (e.g., undersized lines/polygons) while preserving valid features and attributes.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.geojson"));
        defaults.insert("output".to_string(), json!("cleaned.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("linework.gpkg"));
        example_args.insert("output".to_string(), json!("linework_cleaned.geojson"));

        ToolManifest {
            id: "clean_vector".to_string(),
            display_name: "CleanVector".to_string(),
            summary: "Removes null and invalid vector geometries (e.g., undersized lines/polygons) while preserving valid features and attributes.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Drop null and invalid features from a vector layer.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "cleaning".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "clean"));

        ctx.progress.info("running clean_vector");
        let input = read_vector_layer(&input_path, "input")?;

        let mut output = Layer::new(input.name.clone());
        output.geom_type = input.geom_type;
        apply_input_crs_to_layer(&input, &mut output);
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feature_idx, feature) in input.features.iter().enumerate() {
            let cleaned = feature.geometry.as_ref().and_then(clean_geometry);
            if let Some(geometry) = cleaned {
                let attrs = clone_feature_attrs(&input, feature);
                output
                    .add_feature(Some(geometry), &attrs)
                    .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            }
            coalescer.emit_unit_fraction(ctx.progress, (feature_idx + 1) as f64 / total);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for FixDanglingArcsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fix_dangling_arcs",
            display_name: "FixDanglingArcs",
            summary: "Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints within a threshold distance.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polyline vector path.", required: true },
                ToolParamSpec { name: "snap", description: "Snap distance threshold in map units.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("lines.gpkg"));
        defaults.insert("snap".to_string(), json!(2.0));
        defaults.insert("output".to_string(), json!("lines_fixed.geojson"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("stream_network.gpkg"));
        example.insert("snap".to_string(), json!(5.0));
        example.insert("output".to_string(), json!("stream_network_fixed.gpkg"));

        ToolManifest {
            id: "fix_dangling_arcs".to_string(),
            display_name: "FixDanglingArcs".to_string(),
            summary: "Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints within a threshold distance.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polyline vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "snap".to_string(), description: "Snap distance threshold in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "fix_network".to_string(),
                description: "Fix dangling arcs in a polyline network.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "topology".to_string(), "lines".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let snap = parse_optional_f64(args, "snap")?
            .ok_or_else(|| ToolError::Validation("parameter 'snap' is required".to_string()))?;
        if snap <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'snap' must be greater than 0".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let snap_dist = parse_optional_f64(args, "snap")?
            .ok_or_else(|| ToolError::Validation("parameter 'snap' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "fixed_dangles"));

        ctx.progress.info("running fix_dangling_arcs");
        let coalescer = PercentCoalescer::new(1, 99);
        let input = read_vector_layer(&input_path, "input")?;
        if !matches!(input.geom_type, Some(GeometryType::LineString | GeometryType::MultiLineString)) {
            return Err(ToolError::Validation(
                "input vector layer must have LineString or MultiLineString geometry type".to_string(),
            ));
        }

        let (parts, feature_part_ids) = collect_line_parts(&input);
        if parts.is_empty() {
            let mut passthrough = Layer::new(input.name.clone());
            passthrough.geom_type = input.geom_type;
            passthrough.crs = input.crs.clone();
            for field in input.schema.fields() {
                passthrough.add_field(field.clone());
            }
            for feature in &input.features {
                let attrs = clone_feature_attrs(&input, feature);
                passthrough
                    .add_feature(feature.geometry.clone(), &attrs)
                    .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            }
            return write_vector_output(&passthrough, &output_path);
        }

        let mut segments: Vec<SnapSegment> = Vec::new();
        for (part_id, part) in parts.iter().enumerate() {
            for i in 1..part.len() {
                segments.push(SnapSegment {
                    part_id,
                    start: part[i - 1].clone(),
                    end: part[i].clone(),
                });
            }
        }

        let precision = f64::EPSILON * 10.0;
        let mut fixed_parts: Vec<Vec<Coord>> = vec![Vec::new(); parts.len()];
        let total_parts = parts.len().max(1) as f64;

        for (part_id, part) in parts.iter().enumerate() {
            if part.len() < 2 {
                fixed_parts[part_id] = part.clone();
                continue;
            }

            let mut new_points: Vec<Coord> = Vec::new();
            let start = part[0].clone();
            let second = part[1].clone();

            if let Some(candidate) = find_best_snap_candidate(&start, part_id, &segments, snap_dist) {
                if candidate.distance >= precision {
                    let d_current = point_to_segment_distance(&candidate.nearest, &start, &second);
                    if (d_current - candidate.distance).abs() <= precision {
                        new_points.push(candidate.nearest.clone());
                        new_points.push(start.clone());
                    } else {
                        let intersection = segment_intersection_point(
                            &start,
                            &second,
                            &candidate.segment.start,
                            &candidate.segment.end,
                            precision,
                        )
                        .unwrap_or(candidate.nearest.clone());
                        if coord_distance(&second, &intersection) > precision {
                            new_points.push(intersection);
                        }
                    }
                } else {
                    new_points.push(start.clone());
                }
            } else {
                new_points.push(start.clone());
            }

            for coord in part.iter().skip(1).take(part.len().saturating_sub(2)) {
                new_points.push(coord.clone());
            }

            let end_prev = part[part.len() - 2].clone();
            let end = part[part.len() - 1].clone();

            if let Some(candidate) = find_best_snap_candidate(&end, part_id, &segments, snap_dist) {
                if candidate.distance >= precision {
                    let d_current = point_to_segment_distance(&candidate.nearest, &end_prev, &end);
                    let endpoint = if (d_current - candidate.distance).abs() <= precision {
                        new_points.push(end.clone());
                        candidate.nearest.clone()
                    } else {
                        segment_intersection_point(
                            &end_prev,
                            &end,
                            &candidate.segment.start,
                            &candidate.segment.end,
                            precision,
                        )
                        .unwrap_or(candidate.nearest.clone())
                    };

                    if new_points
                        .last()
                        .map(|last| coord_distance(last, &endpoint) > precision)
                        .unwrap_or(true)
                    {
                        new_points.push(endpoint);
                    }
                } else {
                    new_points.push(end.clone());
                }
            } else {
                new_points.push(end.clone());
            }

            let cleaned = dedupe_consecutive_coords(&new_points, precision);
            fixed_parts[part_id] = if cleaned.len() >= 2 { cleaned } else { part.clone() };
            coalescer.emit_unit_fraction(ctx.progress, (part_id + 1) as f64 / total_parts);
        }

        let mut output = Layer::new(input.name.clone());
        output.geom_type = input.geom_type;
        output.crs = input.crs.clone();
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }

        for (feature_idx, feature) in input.features.iter().enumerate() {
            let part_ids = &feature_part_ids[feature_idx];
            let out_geom = match &feature.geometry {
                Some(Geometry::LineString(_)) => {
                    let geom = part_ids
                        .first()
                        .and_then(|id| fixed_parts.get(*id))
                        .cloned()
                        .filter(|coords| coords.len() >= 2)
                        .map(Geometry::line_string);
                    geom.or_else(|| feature.geometry.clone())
                }
                Some(Geometry::MultiLineString(_)) => {
                    let lines = part_ids
                        .iter()
                        .filter_map(|id| fixed_parts.get(*id))
                        .filter(|coords| coords.len() >= 2)
                        .cloned()
                        .collect::<Vec<_>>();
                    if lines.is_empty() {
                        feature.geometry.clone()
                    } else {
                        Some(Geometry::multi_line_string(lines))
                    }
                }
                Some(_) | None => feature.geometry.clone(),
            };

            let attrs = clone_feature_attrs(&input, feature);
            output
                .add_feature(out_geom, &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for ConvertNodataToZeroTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "convert_nodata_to_zero",
            display_name: "ConvertNodataToZero",
            summary: "Replaces raster nodata cells with 0 while leaving valid cells unchanged.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path. If omitted, returns an in-memory raster.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("output".to_string(), json!("input_zeroed.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("landcover.tif"));
        example_args.insert("output".to_string(), json!("landcover_zeroed.tif"));

        ToolManifest {
            id: "convert_nodata_to_zero".to_string(),
            display_name: "ConvertNodataToZero".to_string(),
            summary: "Replaces raster nodata cells with 0 while leaving valid cells unchanged.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path. If omitted, returns an in-memory raster.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Convert raster nodata cells to zero.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "conversion".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("running convert_nodata_to_zero");
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        let mut output = input.clone();
        let len = output.data.len();
        for i in 0..len {
            let value = input.data.get_f64(i);
            if input.is_nodata(value) {
                output.data.set_f64(i, 0.0);
            }
        }
        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for ModifyNodataValueTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "modify_nodata_value",
            display_name: "ModifyNodataValue",
            summary: "Changes the raster nodata value and rewrites existing nodata cells to the new value.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "new_value", description: "New nodata value. Defaults to -32768.0.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, returns an in-memory raster.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("new_value".to_string(), json!(-32768.0));
        defaults.insert("output".to_string(), json!("nodata_modified.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("new_value".to_string(), json!(-9999.0));
        example_args.insert("output".to_string(), json!("dem_nodata_modified.tif"));

        ToolManifest {
            id: "modify_nodata_value".to_string(),
            display_name: "ModifyNodataValue".to_string(),
            summary: "Changes the raster nodata value and rewrites existing nodata cells to the new value.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "new_value".to_string(), description: "New nodata value. Defaults to -32768.0.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, returns an in-memory raster.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Update raster nodata metadata and nodata pixels together.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "nodata".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_f64(args, "new_value")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let new_value = parse_optional_f64(args, "new_value")?.unwrap_or(-32768.0);

        ctx.progress.info("running modify_nodata_value");
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        let mut output = input.clone();
        for i in 0..output.data.len() {
            let value = input.data.get_f64(i);
            if input.is_nodata(value) {
                output.data.set_f64(i, new_value);
            }
        }
        output.nodata = new_value;
        ctx.progress.progress(1.0);

        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for LinesToPolygonsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lines_to_polygons",
            display_name: "LinesToPolygons",
            summary: "Converts polyline features into polygon features, treating the first part as the exterior ring and later parts as holes.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polyline vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("lines.geojson"));
        defaults.insert("output".to_string(), json!("polygons.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("parcel_lines.gpkg"));
        example_args.insert("output".to_string(), json!("parcel_polygons.geojson"));

        ToolManifest {
            id: "lines_to_polygons".to_string(),
            display_name: "LinesToPolygons".to_string(),
            summary: "Converts polyline features into polygon features, treating the first part as the exterior ring and later parts as holes.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polyline vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Convert parcel boundary lines into polygons.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "conversion".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "polygons"));

        ctx.progress.info("running lines_to_polygons");
        let input = read_vector_layer(&input_path, "input")?;
        if !matches!(input.geom_type, Some(GeometryType::LineString | GeometryType::MultiLineString)) {
            return Err(ToolError::Validation(
                "input vector layer must have LineString or MultiLineString geometry type".to_string(),
            ));
        }

        let mut output = Layer::new(input.name.clone()).with_geom_type(GeometryType::Polygon);
        apply_input_crs_to_layer(&input, &mut output);
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feature_idx, feature) in input.features.iter().enumerate() {
            let polygon_geom = match &feature.geometry {
                Some(Geometry::LineString(coords)) => {
                    Geometry::polygon(close_ring(coords), Vec::new())
                }
                Some(Geometry::MultiLineString(parts)) => {
                    if parts.is_empty() {
                        return Err(ToolError::Validation(
                            "encountered empty multipart line geometry while converting lines_to_polygons".to_string(),
                        ));
                    }
                    let exterior = close_ring(&parts[0]);
                    let interiors = parts[1..]
                        .iter()
                        .map(|part| close_ring(part))
                        .collect::<Vec<_>>();
                    Geometry::polygon(exterior, interiors)
                }
                Some(_) => {
                    return Err(ToolError::Validation(
                        "encountered non-line geometry while converting lines_to_polygons".to_string(),
                    ));
                }
                None => {
                    return Err(ToolError::Validation(
                        "line features must contain geometry".to_string(),
                    ));
                }
            };

            let attrs = clone_feature_attrs(&input, feature);
            output
                .add_feature(Some(polygon_geom), &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            coalescer.emit_unit_fraction(ctx.progress, (feature_idx + 1) as f64 / total);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for NewRasterFromBaseRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "new_raster_from_base_raster",
            display_name: "NewRasterFromBaseRaster",
            summary: "Creates a new raster using the extent, dimensions, and CRS of a base raster.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, returns an in-memory raster.", required: false },
                ToolParamSpec { name: "out_val", description: "Optional fill value. Defaults to raster nodata.", required: false },
                ToolParamSpec { name: "data_type", description: "Optional data type: 'float', 'double', or 'integer'.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("base.tif"));
        defaults.insert("output".to_string(), json!("new_raster.tif"));
        defaults.insert("data_type".to_string(), json!("float"));

        let mut example_args = ToolArgs::new();
        example_args.insert("base".to_string(), json!("dem.tif"));
        example_args.insert("output".to_string(), json!("blank_dem.tif"));
        example_args.insert("out_val".to_string(), json!(0.0));

        ToolManifest {
            id: "new_raster_from_base_raster".to_string(),
            display_name: "NewRasterFromBaseRaster".to_string(),
            summary: "Creates a new raster using the extent, dimensions, and CRS of a base raster.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, returns an in-memory raster.".to_string(), required: false },
                ToolParamDescriptor { name: "out_val".to_string(), description: "Optional fill value. Defaults to raster nodata.".to_string(), required: false },
                ToolParamDescriptor { name: "data_type".to_string(), description: "Optional data type: 'float', 'double', or 'integer'.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Create a new blank raster from a base raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "creation".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "base")?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_f64(args, "out_val")?;
        let _ = parse_optional_string(args, "data_type")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_raster_path_arg(args, "base")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let fill_value = parse_optional_f64(args, "out_val")?;
        let data_type = parse_optional_string(args, "data_type")?.unwrap_or("float").to_ascii_lowercase();

        ctx.progress.info("running new_raster_from_base_raster");
        let base = Raster::read(&base_path)
            .map_err(|e| ToolError::Execution(format!("failed reading base raster: {e}")))?;

        let output_data_type = if data_type.contains('i') {
            DataType::I16
        } else if data_type.contains('d') {
            DataType::F64
        } else {
            DataType::F32
        };

        let nodata = base.nodata;
        let fill = fill_value.unwrap_or(nodata);
        let mut output = Raster::new(RasterConfig {
            cols: base.cols,
            rows: base.rows,
            bands: base.bands,
            x_min: base.x_min,
            y_min: base.y_min,
            cell_size: base.cell_size_x,
            cell_size_y: Some(base.cell_size_y),
            nodata,
            data_type: output_data_type,
            crs: base.crs.clone(),
            metadata: base.metadata.clone(),
        });

        if fill != nodata {
            for i in 0..output.data.len() {
                output.data.set_f64(i, fill);
            }
        }

        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for PolygonsToLinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "polygons_to_lines",
            display_name: "PolygonsToLines",
            summary: "Converts polygon and multipolygon features into linework tracing their boundaries.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output line vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.geojson"));
        defaults.insert("output".to_string(), json!("polygon_boundaries.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("watersheds.gpkg"));
        example_args.insert("output".to_string(), json!("watershed_boundaries.geojson"));

        ToolManifest {
            id: "polygons_to_lines".to_string(),
            display_name: "PolygonsToLines".to_string(),
            summary: "Converts polygon and multipolygon features into linework tracing their boundaries.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output line vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Convert polygon boundaries to linework.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "conversion".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "lines"));

        ctx.progress.info("running polygons_to_lines");
        let input = read_vector_layer(&input_path, "input")?;

        if !matches!(input.geom_type, Some(GeometryType::Polygon | GeometryType::MultiPolygon)) {
            return Err(ToolError::Validation(
                "input vector layer must have Polygon or MultiPolygon geometry type".to_string(),
            ));
        }

        let mut output = Layer::new(input.name.clone())
            .with_geom_type(GeometryType::MultiLineString);
        if let Some(epsg) = input.crs_epsg() {
            output = output.with_crs_epsg(epsg);
        }
        if let Some(wkt) = input.crs_wkt() {
            output = output.with_crs_wkt(wkt.to_string());
        }
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }

        for feature in &input.features {
            let geom = match &feature.geometry {
                Some(Geometry::Polygon { exterior, interiors }) => {
                    let mut lines = vec![exterior.0.clone()];
                    for ring in interiors {
                        lines.push(ring.0.clone());
                    }
                    Some(Geometry::multi_line_string(lines))
                }
                Some(Geometry::MultiPolygon(polys)) => {
                    let mut lines = Vec::new();
                    for (exterior, interiors) in polys {
                        lines.push(exterior.0.clone());
                        for ring in interiors {
                            lines.push(ring.0.clone());
                        }
                    }
                    Some(Geometry::multi_line_string(lines))
                }
                Some(_) => {
                    return Err(ToolError::Validation(
                        "encountered non-polygon geometry while converting polygons_to_lines".to_string(),
                    ));
                }
                None => None,
            };

            let attrs: Vec<(&str, FieldValue)> = input
                .schema
                .fields()
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    (
                        field.name.as_str(),
                        feature.attributes.get(idx).cloned().unwrap_or(FieldValue::Null),
                    )
                })
                .collect();
            output
                .add_feature(geom, &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for PrintGeotiffTagsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "print_geotiff_tags",
            display_name: "PrintGeoTiffTags",
            summary: "Produces a text report describing TIFF/GeoTIFF tags and key metadata for an input GeoTIFF-family raster.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamSpec {
                name: "input",
                description: "Input raster path.",
                required: true,
            }],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));

        ToolManifest {
            id: "print_geotiff_tags".to_string(),
            display_name: "PrintGeoTiffTags".to_string(),
            summary: "Produces a text report describing TIFF/GeoTIFF tags and key metadata for an input GeoTIFF-family raster.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input raster path.".to_string(),
                required: true,
            }],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Report TIFF and GeoTIFF tags for a raster.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "metadata".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        ctx.progress.info("running print_geotiff_tags");

        if !looks_like_tiff_family(&input_path) {
            let report = format!(
                "Warning: '{input_path}' does not appear to be a GeoTIFF, BigTIFF, or COG input. Provide a TIFF-family raster such as .tif, .tiff, .gtif, or .gtiff."
            );
            ctx.progress.info(&report);
            return Ok(write_string_output("report", report));
        }

        match build_geotiff_tag_report(&input_path) {
            Ok(report) => {
                ctx.progress.info("generated TIFF tag report");
                Ok(write_string_output("report", report))
            }
            Err(_) => {
                let report = format!(
                    "Warning: '{input_path}' could not be opened as a GeoTIFF, BigTIFF, or COG raster. Verify that the file is a TIFF-family raster with GeoTIFF-compatible structure."
                );
                ctx.progress.info(&report);
                Ok(write_string_output("report", report))
            }
        }
    }
}

impl Tool for ReinitializeAttributeTableTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "reinitialize_attribute_table",
            display_name: "ReinitializeAttributeTable",
            summary: "Creates a copy of a vector layer with only a regenerated FID attribute.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path. If omitted, overwrites the input path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("parcels.geojson"));
        defaults.insert("output".to_string(), json!("parcels_reinit.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("streams.gpkg"));
        example_args.insert("output".to_string(), json!("streams_fid_only.geojson"));

        ToolManifest {
            id: "reinitialize_attribute_table".to_string(),
            display_name: "ReinitializeAttributeTable".to_string(),
            summary: "Creates a copy of a vector layer with only a regenerated FID attribute.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, overwrites the input path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Reset the attribute table so only FID remains.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "attributes".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| PathBuf::from(&input_path));

        ctx.progress.info("running reinitialize_attribute_table");
        let input = read_vector_layer(&input_path, "input")?;

        let mut output = Layer::new(input.name.clone());
        output.geom_type = input.geom_type;
        output.crs = input.crs.clone();
        output.add_field(FieldDef::new("FID", FieldType::Integer));

        for (feature_idx, feature) in input.features.iter().enumerate() {
            output
                .add_feature(
                    feature.geometry.clone(),
                    &[("FID", FieldValue::Integer((feature_idx + 1) as i64))],
                )
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for RasterToVectorPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_to_vector_points",
            display_name: "RasterToVectorPoints",
            summary: "Converts non-zero, non-nodata cells in a raster into point features located at cell centres.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "output", description: "Output point vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("output".to_string(), json!("points.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("classified.tif"));
        example_args.insert("output".to_string(), json!("classified_points.geojson"));

        ToolManifest {
            id: "raster_to_vector_points".to_string(),
            display_name: "RasterToVectorPoints".to_string(),
            summary: "Converts non-zero, non-nodata cells in a raster into point features located at cell centres.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output point vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Convert non-zero raster cells into points.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "vectorization".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "points"));

        ctx.progress.info("running raster_to_vector_points");
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        if input.bands != 1 {
            return Err(ToolError::Validation(
                "input raster must be single-band for raster_to_vector_points".to_string(),
            ));
        }

        let mut output = Layer::new(
            Path::new(&input_path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("raster_points"),
        )
        .with_geom_type(GeometryType::Point);
        apply_raster_crs_to_layer(&input, &mut output);
        output.add_field(FieldDef::new("FID", FieldType::Integer));
        output.add_field(FieldDef::new("VALUE", FieldType::Float).width(18).precision(8));

        let total_rows = input.rows.max(1) as f64;
        let mut next_fid = 1i64;
        for row in 0..input.rows as isize {
            for col in 0..input.cols as isize {
                let value = input.get_raw(0, row, col).unwrap_or(input.nodata);
                if input.is_nodata(value) || value == 0.0 {
                    continue;
                }

                let point = Geometry::point(input.col_center_x(col), input.row_center_y(row));
                output
                    .add_feature(
                        Some(point),
                        &[
                            ("FID", FieldValue::Integer(next_fid)),
                            ("VALUE", FieldValue::Float(value)),
                        ],
                    )
                    .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
                next_fid += 1;
            }
            ctx.progress.progress((row as f64 + 1.0) / total_rows);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for RemovePolygonHolesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "remove_polygon_holes",
            display_name: "RemovePolygonHoles",
            summary: "Removes interior rings from polygon features while preserving attributes.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output polygon vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.geojson"));
        defaults.insert("output".to_string(), json!("polygons_no_holes.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("watersheds.gpkg"));
        example_args.insert("output".to_string(), json!("watersheds_no_holes.geojson"));

        ToolManifest {
            id: "remove_polygon_holes".to_string(),
            display_name: "RemovePolygonHoles".to_string(),
            summary: "Removes interior rings from polygon features while preserving attributes.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output polygon vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Remove polygon holes from a vector layer.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "topology".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "no_holes"));

        ctx.progress.info("running remove_polygon_holes");
        let input = read_vector_layer(&input_path, "input")?;
        if !matches!(input.geom_type, Some(GeometryType::Polygon | GeometryType::MultiPolygon)) {
            return Err(ToolError::Validation(
                "input vector layer must have Polygon or MultiPolygon geometry type".to_string(),
            ));
        }

        let mut output = Layer::new(input.name.clone());
        output.geom_type = input.geom_type;
        apply_input_crs_to_layer(&input, &mut output);
        for field in input.schema.fields() {
            output.add_field(field.clone());
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feature_idx, feature) in input.features.iter().enumerate() {
            let geometry = match &feature.geometry {
                Some(geometry) => Some(strip_polygon_holes_with_topology(geometry)?),
                None => None,
            };
            let attrs = clone_feature_attrs(&input, feature);
            output
                .add_feature(geometry, &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            coalescer.emit_unit_fraction(ctx.progress, (feature_idx + 1) as f64 / total);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for SetNodataValueTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "set_nodata_value",
            display_name: "SetNodataValue",
            summary: "Sets a raster nodata value and maps existing nodata cells to the specified background value.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "back_value", description: "Background value to assign as nodata. Defaults to 0.0.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, returns an in-memory raster.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("back_value".to_string(), json!(0.0));
        defaults.insert("output".to_string(), json!("nodata_set.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("landcover.tif"));
        example_args.insert("back_value".to_string(), json!(-9999.0));
        example_args.insert("output".to_string(), json!("landcover_nodata_set.tif"));

        ToolManifest {
            id: "set_nodata_value".to_string(),
            display_name: "SetNodataValue".to_string(),
            summary: "Sets a raster nodata value and maps existing nodata cells to the specified background value.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "back_value".to_string(), description: "Background value to assign as nodata. Defaults to 0.0.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, returns an in-memory raster.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Set raster nodata value using a specific background value.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "nodata".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_f64(args, "back_value")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let back_value = parse_optional_f64(args, "back_value")?.unwrap_or(0.0);

        ctx.progress.info("running set_nodata_value");
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;

        let output_data_type = nodata_data_type_for_background(input.data_type, back_value);
        let mut output = Raster::new(RasterConfig {
            cols: input.cols,
            rows: input.rows,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: back_value,
            data_type: output_data_type,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        for i in 0..input.data.len() {
            let value = input.data.get_f64(i);
            if !input.is_nodata(value) {
                output.data.set_f64(i, value);
            }
        }

        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

// ── MultipartToSinglepartTool ─────────────────────────────────────────────────

impl Tool for MultipartToSinglepartTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "multipart_to_singlepart",
            display_name: "MultipartToSinglepart",
            summary: "Converts a vector containing multi-part features into one with only single-part features. For polygon vectors, the `exclude_holes` flag controls whether interior rings are emitted as separate features (false, default) or kept attached to their enclosing exterior ring (true).",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path (polyline, polygon, or multipoint).", required: true },
                ToolParamSpec { name: "output", description: "Output vector path. Defaults to a timestamped GeoJSON beside the input.", required: false },
                ToolParamSpec { name: "exclude_holes", description: "When true, polygon holes remain attached to their enclosing exterior ring (default: false).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.geojson"));
        defaults.insert("output".to_string(), json!("singlepart.geojson"));
        defaults.insert("exclude_holes".to_string(), json!(false));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("parcels.gpkg"));
        example_args.insert("output".to_string(), json!("parcels_single.geojson"));
        example_args.insert("exclude_holes".to_string(), json!(true));

        ToolManifest {
            id: "multipart_to_singlepart".to_string(),
            display_name: "MultipartToSinglepart".to_string(),
            summary: "Converts a vector containing multi-part features into one with only single-part features.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path (polyline, polygon, or multipoint).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. Defaults to a timestamped GeoJSON beside the input.".to_string(), required: false },
                ToolParamDescriptor { name: "exclude_holes".to_string(), description: "When true, polygon holes remain attached to their enclosing exterior ring (default: false).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "split_multipolygon".to_string(),
                description: "Split multi-part parcel polygons, keeping holes attached.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "multipart".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "singlepart"));
        let exclude_holes = args
            .get("exclude_holes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        ctx.progress.info("running multipart_to_singlepart");
        let input = read_vector_layer(&input_path, "input")?;

        let out_geom_type = input.geom_type.map(single_part_geom_type).unwrap_or(GeometryType::Point);
        let mut output = Layer::new(input.name.clone());
        output.geom_type = Some(out_geom_type);
        apply_input_crs_to_layer(&input, &mut output);

        output.add_field(FieldDef::new("FID", FieldType::Integer));
        for field in input.schema.fields() {
            if field.name.to_uppercase() != "FID" {
                output.add_field(field.clone());
            }
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        let mut fid = 1i64;
        for (feat_idx, feature) in input.features.iter().enumerate() {
            if let Some(geom) = &feature.geometry {
                let parts = expand_to_single_part(geom, exclude_holes);
                let src_attrs = clone_feature_attrs(&input, feature);
                for part_geom in parts {
                    let mut attrs: Vec<(&str, FieldValue)> = vec![("FID", FieldValue::Integer(fid))];
                    for (name, val) in &src_attrs {
                        if name.to_uppercase() != "FID" {
                            attrs.push((name, val.clone()));
                        }
                    }
                    output
                        .add_feature(Some(part_geom), &attrs)
                        .map_err(|e| ToolError::Execution(format!("failed adding feature: {e}")))?;
                    fid += 1;
                }
            }
            coalescer.emit_unit_fraction(ctx.progress, (feat_idx + 1) as f64 / total);
        }

        write_vector_output(&output, &output_path)
    }
}

// ── SinglepartToMultipartTool ─────────────────────────────────────────────────

impl Tool for SinglepartToMultipartTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "singlepart_to_multipart",
            display_name: "SinglepartToMultipart",
            summary: "Merges single-part features into multi-part features. Features may be grouped by a categorical field, or all collected into one multi-part geometry when no field is specified.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "output", description: "Output vector path. Defaults to a timestamped GeoJSON beside the input.", required: false },
                ToolParamSpec { name: "field", description: "Optional attribute field name to group features by. When omitted, all features are merged into one geometry.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.geojson"));
        defaults.insert("output".to_string(), json!("multipart.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("parcels.geojson"));
        example_args.insert("field".to_string(), json!("OWNER_ID"));
        example_args.insert("output".to_string(), json!("parcels_multi.geojson"));

        ToolManifest {
            id: "singlepart_to_multipart".to_string(),
            display_name: "SinglepartToMultipart".to_string(),
            summary: "Merges single-part features into multi-part features, grouped by an optional categorical field.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. Defaults to a timestamped GeoJSON beside the input.".to_string(), required: false },
                ToolParamDescriptor { name: "field".to_string(), description: "Optional attribute field name to group features by. When omitted, all features are merged into one geometry.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "group_by_owner".to_string(),
                description: "Merge parcels belonging to the same owner into multi-part polygons.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "multipart".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "multipart"));
        let field_name = parse_optional_string(args, "field")?;

        ctx.progress.info("running singlepart_to_multipart");
        let input = read_vector_layer(&input_path, "input")?;

        let input_geom_type = input.geom_type.unwrap_or(GeometryType::Point);
        let output_geom_type = match input_geom_type {
            GeometryType::Point => GeometryType::MultiPoint,
            GeometryType::LineString => GeometryType::MultiLineString,
            GeometryType::Polygon => GeometryType::MultiPolygon,
            other => other,
        };

        let mut output = Layer::new(input.name.clone());
        output.geom_type = Some(output_geom_type);
        apply_input_crs_to_layer(&input, &mut output);
        output.add_field(FieldDef::new("FID", FieldType::Integer));

        if let Some(fname) = field_name {
            let field_idx = input
                .schema
                .field_index(fname)
                .ok_or_else(|| ToolError::Validation(format!("field '{fname}' not found in input layer")))?;

            if let Some(fdef) = input.schema.field(fname) {
                output.add_field(fdef.clone());
            }

            // Group feature indices by string representation of the grouping field value
            let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
            for (feat_idx, feature) in input.features.iter().enumerate() {
                let key = feature
                    .attributes
                    .get(field_idx)
                    .cloned()
                    .unwrap_or(FieldValue::Null)
                    .to_string();
                if let Some(group) = groups.iter_mut().find(|(k, _)| k == &key) {
                    group.1.push(feat_idx);
                } else {
                    groups.push((key, vec![feat_idx]));
                }
            }

            let total = groups.len().max(1) as f64;
            let coalescer = PercentCoalescer::new(1, 99);
            let mut fid = 1i64;
            for (group_idx, (key_str, feat_indices)) in groups.iter().enumerate() {
                let geom = merge_to_multi(&input, feat_indices, input_geom_type)?;
                let key_val: FieldValue = if let Some(f) = input.features.get(feat_indices[0]) {
                    f.attributes.get(field_idx).cloned().unwrap_or(FieldValue::Null)
                } else {
                    FieldValue::Text(key_str.clone())
                };
                output
                    .add_feature(Some(geom), &[("FID", FieldValue::Integer(fid)), (fname, key_val)])
                    .map_err(|e| ToolError::Execution(format!("failed adding feature: {e}")))?;
                fid += 1;
                coalescer.emit_unit_fraction(ctx.progress, (group_idx + 1) as f64 / total);
            }
        } else {
            let all_indices: Vec<usize> = (0..input.features.len()).collect();
            let geom = merge_to_multi(&input, &all_indices, input_geom_type)?;
            output
                .add_feature(Some(geom), &[("FID", FieldValue::Integer(1))])
                .map_err(|e| ToolError::Execution(format!("failed adding feature: {e}")))?;
            ctx.progress.progress(1.0);
        }

        write_vector_output(&output, &output_path)
    }
}

// ── MergeVectorsTool ──────────────────────────────────────────────────────────

impl Tool for MergeVectorsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "merge_vectors",
            display_name: "MergeVectors",
            summary: "Combines two or more input vectors of the same geometry type into a single output vector. Output attributes include FID, PARENT (source layer name), PARENT_FID, and any fields common to all inputs.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of input vector paths (at least two required).", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["layer1.geojson", "layer2.geojson"]));
        defaults.insert("output".to_string(), json!("merged.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("inputs".to_string(), json!(["roads_a.shp", "roads_b.shp", "roads_c.shp"]));
        example_args.insert("output".to_string(), json!("roads_merged.geojson"));

        ToolManifest {
            id: "merge_vectors".to_string(),
            display_name: "MergeVectors".to_string(),
            summary: "Combines two or more input vectors of the same geometry type into a single output vector.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "inputs".to_string(), description: "Array of input vector paths (at least two required).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "merge_three".to_string(),
                description: "Merge three road layer files into one.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "merge".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_vector_list_arg(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two vector paths".to_string(),
            ));
        }
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_vector_list_arg(args, "inputs")?;
        if input_paths.len() < 2 {
            return Err(ToolError::Validation(
                "parameter 'inputs' must contain at least two vector paths".to_string(),
            ));
        }
        let output_path_str = parse_vector_path_arg(args, "output")?;
        let output_path = PathBuf::from(&output_path_str);

        ctx.progress.info("running merge_vectors");

        let mut layers: Vec<Layer> = Vec::with_capacity(input_paths.len());
        for path in &input_paths {
            layers.push(read_vector_layer(path, "inputs")?);
        }

        let base_geom_type = layers[0].geom_type;
        for (i, layer) in layers.iter().enumerate().skip(1) {
            if layer.geom_type != base_geom_type {
                return Err(ToolError::Validation(format!(
                    "input {} has geometry type {:?} but input 0 has {:?}; all inputs must share the same geometry type",
                    i, layer.geom_type, base_geom_type
                )));
            }
        }

        // Find fields common to all layers (same name and field_type), excluding FID
        let mut common_fields: Vec<FieldDef> = layers[0]
            .schema
            .fields()
            .iter()
            .filter(|f| f.name.to_uppercase() != "FID")
            .cloned()
            .collect();
        for layer in layers.iter().skip(1) {
            common_fields.retain(|cf| {
                layer
                    .schema
                    .field(&cf.name)
                    .map(|f| f.field_type == cf.field_type)
                    .unwrap_or(false)
            });
        }

        let mut output = Layer::new("merged");
        output.geom_type = base_geom_type;
        apply_input_crs_to_layer(&layers[0], &mut output);
        output.add_field(FieldDef::new("FID", FieldType::Integer));
        output.add_field(FieldDef::new("PARENT", FieldType::Text));
        output.add_field(FieldDef::new("PARENT_FID", FieldType::Integer));
        for cf in &common_fields {
            output.add_field(cf.clone());
        }

        let total_features: usize = layers.iter().map(|l| l.features.len()).sum();
        let total = total_features.max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        let mut fid = 1i64;
        let mut processed = 0usize;

        for (layer_idx, layer) in layers.iter().enumerate() {
            let parent_name = Path::new(&input_paths[layer_idx])
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            for (feat_idx, feature) in layer.features.iter().enumerate() {
                let mut attrs: Vec<(&str, FieldValue)> = vec![
                    ("FID", FieldValue::Integer(fid)),
                    ("PARENT", FieldValue::Text(parent_name.clone())),
                    ("PARENT_FID", FieldValue::Integer(feat_idx as i64 + 1)),
                ];
                for cf in &common_fields {
                    let val = if let Some(idx) = layer.schema.field_index(&cf.name) {
                        feature.attributes.get(idx).cloned().unwrap_or(FieldValue::Null)
                    } else {
                        FieldValue::Null
                    };
                    attrs.push((cf.name.as_str(), val));
                }
                output
                    .add_feature(feature.geometry.clone(), &attrs)
                    .map_err(|e| ToolError::Execution(format!("failed adding feature: {e}")))?;
                fid += 1;
                processed += 1;
                coalescer.emit_unit_fraction(ctx.progress, processed as f64 / total);
            }
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for VectorLinesToRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "vector_lines_to_raster",
            display_name: "VectorLinesToRaster",
            summary: "Rasterizes line and polygon boundary geometries to a raster grid.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path (lines or polygons).", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, output remains in memory.", required: false },
                ToolParamSpec { name: "field", description: "Optional numeric field name to burn. Defaults to FID.", required: false },
                ToolParamSpec { name: "zero_background", description: "When true, initialize output background to 0 instead of nodata.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when 'base' is not supplied. If omitted and no base is given, auto cell size is max extent / 500.", required: false },
                ToolParamSpec { name: "base", description: "Optional base raster path defining output grid and extent.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("lines.gpkg"));
        defaults.insert("field".to_string(), json!("FID"));
        defaults.insert("cell_size".to_string(), json!(10.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("roads.gpkg"));
        example_args.insert("field".to_string(), json!("CLASS_ID"));
        example_args.insert("base".to_string(), json!("base_dem.tif"));
        example_args.insert("output".to_string(), json!("roads_lines.tif"));

        ToolManifest {
            id: "vector_lines_to_raster".to_string(),
            display_name: "VectorLinesToRaster".to_string(),
            summary: "Rasterizes line and polygon boundary geometries to a raster grid.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path (lines or polygons).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, output remains in memory.".to_string(), required: false },
                ToolParamDescriptor { name: "field".to_string(), description: "Optional numeric field name to burn. Defaults to FID.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "When true, initialize output background to 0 instead of nodata.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when 'base' is not supplied. If omitted and no base is given, auto cell size is max extent / 500.".to_string(), required: false },
                ToolParamDescriptor { name: "base".to_string(), description: "Optional base raster path defining output grid and extent.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "roads_to_raster".to_string(),
                description: "Burn line values onto a base raster grid.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "raster".to_string(), "lines".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_string(args, "field")?;
        let _ = parse_optional_f64(args, "cell_size")?;
        let _ = parse_optional_string(args, "base")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let field_name = parse_optional_string(args, "field")?.unwrap_or("FID");
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = parse_optional_f64(args, "cell_size")?.unwrap_or(0.0);
        let base_path = parse_optional_string(args, "base")?;

        ctx.progress.info("running vector_lines_to_raster");
        let input = read_vector_layer(&input_path, "input")?;
        let gt = input.geom_type.unwrap_or(GeometryType::LineString);
        if gt != GeometryType::LineString
            && gt != GeometryType::MultiLineString
            && gt != GeometryType::Polygon
            && gt != GeometryType::MultiPolygon
        {
            return Err(ToolError::Validation(
                "input vector must contain line or polygon geometries for vector_lines_to_raster".to_string(),
            ));
        }

        let nodata = -32768.0;
        let background = if zero_background { 0.0 } else { nodata };
        let use_fid = field_name.eq_ignore_ascii_case("FID");
        let field_idx = if use_fid {
            None
        } else {
            input.schema.field_index(field_name)
        };
        let data_type = if use_fid { DataType::I32 } else { DataType::F32 };

        let mut output = if let Some(base) = base_path {
            let base_raster = Raster::read(base)
                .map_err(|e| ToolError::Execution(format!("failed reading base raster: {e}")))?;
            Raster::new(RasterConfig {
                cols: base_raster.cols,
                rows: base_raster.rows,
                bands: 1,
                x_min: base_raster.x_min,
                y_min: base_raster.y_min,
                cell_size: base_raster.cell_size_x,
                cell_size_y: Some(base_raster.cell_size_y),
                nodata,
                data_type,
                crs: base_raster.crs.clone(),
                metadata: base_raster.metadata.clone(),
            })
        } else {
            let mut input_for_bbox = input.clone();
            let bbox = input_for_bbox
                .bbox()
                .ok_or_else(|| ToolError::Validation("input vector has no geometry extent".to_string()))?;
            let mut auto_cs = cell_size;
            if auto_cs <= 0.0 {
                auto_cs = ((bbox.max_x - bbox.min_x).max(bbox.max_y - bbox.min_y) / 500.0).max(1e-9);
            }
            let cols = ((bbox.max_x - bbox.min_x) / auto_cs).ceil().max(1.0) as usize;
            let rows = ((bbox.max_y - bbox.min_y) / auto_cs).ceil().max(1.0) as usize;
            Raster::new(RasterConfig {
                cols,
                rows,
                bands: 1,
                x_min: bbox.min_x,
                y_min: bbox.min_y,
                cell_size: auto_cs,
                cell_size_y: Some(auto_cs),
                nodata,
                data_type,
                crs: vector_to_raster_crs(&input),
                metadata: Vec::new(),
            })
        };

        if background != nodata {
            output.fill(background);
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feat_idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = &feature.geometry else {
                continue;
            };
            let burn_value = if use_fid {
                feat_idx as f64 + 1.0
            } else if let Some(idx) = field_idx {
                feature
                    .attributes
                    .get(idx)
                    .and_then(field_value_as_f64)
                    .unwrap_or(feat_idx as f64 + 1.0)
            } else {
                feat_idx as f64 + 1.0
            };

            let mut parts: Vec<Vec<Coord>> = Vec::new();
            geometry_line_parts(geometry, &mut parts);
            for part in parts {
                if part.len() < 2 {
                    continue;
                }
                for i in 0..part.len() - 1 {
                    rasterize_segment(&mut output, &part[i], &part[i + 1], burn_value);
                }
            }

            coalescer.emit_unit_fraction(ctx.progress, (feat_idx + 1) as f64 / total);
        }

        write_raster_output(output, output_path, ctx)
    }
}

#[derive(Clone, Copy)]
struct PolygonTraceSegment {
    p1: (f64, f64),
    p2: (f64, f64),
    value: u32,
}

impl Tool for RasterToVectorPolygonsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_to_vector_polygons",
            display_name: "RasterToVectorPolygons",
            summary: "Converts non-zero, non-nodata raster regions into polygon vector features with FID and VALUE attributes.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path (single-band).", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("classes.tif"));
        defaults.insert("output".to_string(), json!("classes_polygons.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("landcover.tif"));
        example_args.insert("output".to_string(), json!("landcover_polygons.geojson"));

        ToolManifest {
            id: "raster_to_vector_polygons".to_string(),
            display_name: "RasterToVectorPolygons".to_string(),
            summary: "Converts non-zero, non-nodata raster regions into polygon vector features with FID and VALUE attributes.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path (single-band).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygonize_classes".to_string(),
                description: "Convert classified raster patches to polygon features.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "vector".to_string(), "polygonize".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "polygons"));

        ctx.progress.info("running raster_to_vector_polygons");
        let coalescer = PercentCoalescer::new(1, 99);
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        if input.bands != 1 {
            return Err(ToolError::Validation(
                "input raster must be single-band for raster_to_vector_polygons".to_string(),
            ));
        }

        let rows = input.rows as isize;
        let cols = input.cols as isize;
        let dx = [0isize, 1, 0, -1, 1, 1, -1, -1];
        let dy = [-1isize, 0, 1, 0, -1, 1, 1, -1];

        // 8-connected clumping by equal value.
        let mut clumps = vec![0u32; input.rows * input.cols];
        let mut visited = vec![false; input.rows * input.cols];
        let mut queue: VecDeque<(isize, isize)> = VecDeque::new();
        let mut clump_val = 1u32;
        let mut clump_to_value = vec![0.0f64];

        for row in 0..rows {
            for col in 0..cols {
                let idx = row as usize * input.cols + col as usize;
                if visited[idx] {
                    continue;
                }
                let Some(v) = input.get_raw(0, row, col) else { continue };
                if input.is_nodata(v) || v == 0.0 {
                    visited[idx] = true;
                    continue;
                }

                visited[idx] = true;
                clumps[idx] = clump_val;
                clump_to_value.push(v);
                queue.push_back((row, col));
                while let Some((rr, cc)) = queue.pop_front() {
                    for n in 0..8 {
                        let rn = rr + dy[n];
                        let cn = cc + dx[n];
                        if rn < 0 || rn >= rows || cn < 0 || cn >= cols {
                            continue;
                        }
                        let n_idx = rn as usize * input.cols + cn as usize;
                        if visited[n_idx] {
                            continue;
                        }
                        let Some(vn) = input.get_raw(0, rn, cn) else { continue };
                        if !input.is_nodata(vn) && vn == v {
                            visited[n_idx] = true;
                            clumps[n_idx] = clump_val;
                            queue.push_back((rn, cn));
                        }
                    }
                }
                clump_val += 1;
            }
            ctx.progress.progress((row as f64 + 1.0) / rows.max(1) as f64 * 0.25);
        }

        let half_x = input.cell_size_x / 2.0;
        let half_y = input.cell_size_y / 2.0;
        let edge_offsets_pt1_x = [-half_x, half_x, half_x, -half_x];
        let edge_offsets_pt1_y = [half_y, half_y, -half_y, -half_y];
        let edge_offsets_pt2_x = [half_x, half_x, -half_x, -half_x];
        let edge_offsets_pt2_y = [half_y, -half_y, -half_y, half_y];

        // Build boundary segments for each clump.
        let mut segments: Vec<PolygonTraceSegment> = Vec::new();
        let mut tree = KdTree::with_capacity(2, 64);
        let mut node_id = 0usize;
        for row in 0..rows {
            for col in 0..cols {
                let idx = row as usize * input.cols + col as usize;
                let z = clumps[idx];
                if z == 0 {
                    continue;
                }
                for n in 0..4 {
                    let rn = row + dy[n];
                    let cn = col + dx[n];
                    let zn = if rn >= 0 && rn < rows && cn >= 0 && cn < cols {
                        clumps[rn as usize * input.cols + cn as usize]
                    } else {
                        0
                    };
                    if z == zn {
                        continue;
                    }

                    let cx = input.col_center_x(col);
                    let cy = input.row_center_y(row);
                    let p1 = (cx + edge_offsets_pt1_x[n], cy + edge_offsets_pt1_y[n]);
                    let p2 = (cx + edge_offsets_pt2_x[n], cy + edge_offsets_pt2_y[n]);
                    tree.add([p1.0, p1.1], node_id)
                        .map_err(|e| ToolError::Execution(format!("failed adding boundary node: {e}")))?;
                    node_id += 1;
                    tree.add([p2.0, p2.1], node_id)
                        .map_err(|e| ToolError::Execution(format!("failed adding boundary node: {e}")))?;
                    node_id += 1;
                    segments.push(PolygonTraceSegment { p1, p2, value: z });
                }
            }
            ctx.progress.progress(0.25 + (row as f64 + 1.0) / rows.max(1) as f64 * 0.25);
        }

        let mut rings_by_clump: Vec<Vec<Vec<(f64, f64)>>> = vec![Vec::new(); clump_val as usize];
        let mut node_live = vec![true; segments.len() * 2];
        let prec = (5.0 * f64::EPSILON).tan();

        for node in 0..segments.len() * 2 {
            if !node_live[node] {
                continue;
            }
            let seg_idx = node / 2;
            let z = segments[seg_idx].value as usize;
            let mut current_node = node;
            let line_start = node;
            let mut points: Vec<(f64, f64)> = Vec::new();

            loop {
                let current_seg = current_node / 2;
                let p1 = if current_node % 2 == 0 {
                    segments[current_seg].p1
                } else {
                    segments[current_seg].p2
                };
                points.push(p1);
                node_live[current_node] = false;

                let ret = tree
                    .within(&[p1.0, p1.1], prec, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("failed boundary node lookup: {e}")))?;

                let mut connected_nodes: Vec<usize> = Vec::new();
                for hit in &ret {
                    let node_n = *hit.1;
                    let seg_n = node_n / 2;
                    if segments[seg_n].value as usize == z && node_live[node_n] {
                        connected_nodes.push(node_n);
                    }
                }

                if connected_nodes.is_empty() {
                    current_node = if current_node % 2 == 0 {
                        current_node + 1
                    } else {
                        current_node - 1
                    };
                    if !node_live[current_node] {
                        let p_close = if line_start % 2 == 0 {
                            segments[line_start / 2].p1
                        } else {
                            segments[line_start / 2].p2
                        };
                        points.push(p_close);
                        break;
                    }
                } else if connected_nodes.len() == 1 {
                    current_node = if connected_nodes[0] % 2 == 0 {
                        connected_nodes[0] + 1
                    } else {
                        connected_nodes[0] - 1
                    };
                    node_live[connected_nodes[0]] = false;
                } else {
                    if points.len() < 2 {
                        current_node = if connected_nodes[0] % 2 == 0 {
                            connected_nodes[0] + 1
                        } else {
                            connected_nodes[0] - 1
                        };
                        node_live[connected_nodes[0]] = false;
                        continue;
                    }

                    let p_prev = points[points.len() - 2];
                    let p_curr = points[points.len() - 1];
                    let mut best = None;
                    let mut best_heading = -10.0;
                    for (n, connected) in connected_nodes.iter().enumerate() {
                        let seg_n = connected / 2;
                        let p_next = if connected % 2 == 0 {
                            segments[seg_n].p2
                        } else {
                            segments[seg_n].p1
                        };
                        let heading = -((p_next.1 - p_curr.1).atan2(p_next.0 - p_curr.0)
                            - (p_prev.1 - p_curr.1).atan2(p_prev.0 - p_curr.0));
                        if heading > best_heading && heading != 0.0 {
                            best_heading = heading;
                            best = Some(n);
                        }
                    }
                    if let Some(best_n) = best {
                        current_node = if connected_nodes[best_n] % 2 == 0 {
                            connected_nodes[best_n] + 1
                        } else {
                            connected_nodes[best_n] - 1
                        };
                        node_live[connected_nodes[best_n]] = false;
                    } else {
                        break;
                    }
                }
            }

            if points.len() >= 4 {
                // Remove collinear interior points.
                let mut i = 1usize;
                while i + 1 < points.len() {
                    let p0 = points[i - 1];
                    let p1 = points[i];
                    let p2 = points[i + 1];
                    let cross = ((p1.1 - p0.1) * (p2.0 - p1.0) - (p2.1 - p1.1) * (p1.0 - p0.0)).abs();
                    let dot = ((p1.0 - p0.0) * (p2.0 - p1.0) + (p1.1 - p0.1) * (p2.1 - p1.1)).abs();
                    if cross <= dot * prec {
                        points.remove(i);
                    } else {
                        i += 1;
                    }
                }
                if points.first() != points.last() {
                    points.push(points[0]);
                }
                rings_by_clump[z].push(points);
            }
        }

        // Build vector output with VALUE and FID fields.
        let mut output = Layer::new(
            Path::new(&input_path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("raster_polygons"),
        )
        .with_geom_type(GeometryType::Polygon);
        apply_raster_crs_to_layer(&input, &mut output);
        output.add_field(FieldDef::new("FID", FieldType::Integer));
        output.add_field(FieldDef::new("VALUE", FieldType::Float).width(18).precision(8));

        let mut next_fid = 1i64;
        let total_clumps = (clump_val as usize).saturating_sub(1).max(1) as f64;
        for clump_id in 1..clump_val as usize {
            let rings = &rings_by_clump[clump_id];
            if rings.is_empty() {
                continue;
            }

            let mut exteriors: Vec<Vec<(f64, f64)>> = Vec::new();
            let mut holes: Vec<Vec<(f64, f64)>> = Vec::new();
            for ring in rings {
                if ring_signed_area(ring) < 0.0 {
                    exteriors.push(ring.clone());
                } else {
                    holes.push(ring.clone());
                }
            }
            if exteriors.is_empty() {
                exteriors.push(rings[0].clone());
            }

            let mut hole_groups: Vec<Vec<Vec<(f64, f64)>>> = vec![Vec::new(); exteriors.len()];
            for hole in holes {
                let test_pt = hole[0];
                let mut assigned = false;
                for (i, ext) in exteriors.iter().enumerate() {
                    if point_in_ring(test_pt, ext) {
                        hole_groups[i].push(hole.clone());
                        assigned = true;
                        break;
                    }
                }
                if !assigned {
                    hole_groups[0].push(hole);
                }
            }

            let geom = if exteriors.len() == 1 {
                Geometry::Polygon {
                    exterior: Ring(normalize_ring(&exteriors[0])),
                    interiors: hole_groups[0]
                        .iter()
                        .map(|r| Ring(normalize_ring(r)))
                        .collect(),
                }
            } else {
                Geometry::MultiPolygon(
                    exteriors
                        .iter()
                        .enumerate()
                        .map(|(i, ext)| {
                            (
                                Ring(normalize_ring(ext)),
                                hole_groups[i].iter().map(|r| Ring(normalize_ring(r))).collect(),
                            )
                        })
                        .collect(),
                )
            };

            output
                .add_feature(
                    Some(geom),
                    &[
                        ("FID", FieldValue::Integer(next_fid)),
                        ("VALUE", FieldValue::Float(clump_to_value[clump_id])),
                    ],
                )
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
            next_fid += 1;

            coalescer.emit_unit_fraction(ctx.progress, 0.5 + (clump_id as f64 / total_clumps) * 0.5);
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for RasterToVectorLinesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_to_vector_lines",
            display_name: "RasterToVectorLines",
            summary: "Converts non-zero, non-nodata raster line cells into polyline vector features.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path (single-band).", required: true },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("lines.tif"));
        defaults.insert("output".to_string(), json!("lines.geojson"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("streams_binary.tif"));
        example_args.insert("output".to_string(), json!("streams_lines.geojson"));

        ToolManifest {
            id: "raster_to_vector_lines".to_string(),
            display_name: "RasterToVectorLines".to_string(),
            summary: "Converts non-zero, non-nodata raster line cells into polyline vector features.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path (single-band).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. If omitted, a GeoJSON path is derived beside the input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Convert line raster cells to polylines.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "vector".to_string(), "lines".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&input_path, "lines"));

        ctx.progress.info("running raster_to_vector_lines");
        let coalescer = PercentCoalescer::new(1, 99);
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        if input.bands != 1 {
            return Err(ToolError::Validation(
                "input raster must be single-band for raster_to_vector_lines".to_string(),
            ));
        }

        let rows = input.rows as isize;
        let cols = input.cols as isize;
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let mut queue: VecDeque<(isize, isize)> = VecDeque::new();
        let mut visited = vec![1i8; input.rows * input.cols];
        let mut num_neigh = vec![0i8; input.rows * input.cols];
        let mut active_cells = 0usize;

        for row in 0..rows {
            for col in 0..cols {
                let Some(v) = input.get_raw(0, row, col) else { continue };
                if input.is_nodata(v) || v == 0.0 {
                    continue;
                }
                let idx = row as usize * input.cols + col as usize;
                visited[idx] = 0;
                active_cells += 1;
                let mut count = 0i8;
                for i in 0..8 {
                    let rn = row + dy[i];
                    let cn = col + dx[i];
                    if let Some(vn) = input.get_raw(0, rn, cn) {
                        if !input.is_nodata(vn) && vn != 0.0 {
                            count += 1;
                        }
                    }
                }
                num_neigh[idx] = count;
                if count == 1 {
                    queue.push_back((row, col));
                }
            }
            ctx.progress.progress((row as f64 + 1.0) / rows.max(1) as f64 * 0.2);
        }

        let mut output = Layer::new(
            Path::new(&input_path)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("raster_lines"),
        )
        .with_geom_type(GeometryType::LineString);
        apply_raster_crs_to_layer(&input, &mut output);
        output.add_field(FieldDef::new("FID", FieldType::Integer));
        output.add_field(FieldDef::new("VALUE", FieldType::Float).width(18).precision(8));

        let mut next_fid = 1i64;
        let mut solved_cells = 0usize;

        while let Some((mut row, mut col)) = queue.pop_front() {
            let idx = row as usize * input.cols + col as usize;
            if visited[idx] != 0 {
                continue;
            }
            let current_val = input.get_raw(0, row, col).unwrap_or(input.nodata);
            let mut points: Vec<(f64, f64)> = Vec::new();

            loop {
                let current_idx = row as usize * input.cols + col as usize;
                if visited[current_idx] != 0 {
                    break;
                }
                points.push((input.col_center_x(col), input.row_center_y(row)));
                visited[current_idx] = 1;
                solved_cells += 1;

                let mut highest = 0i8;
                let mut next_rc: Option<(isize, isize)> = None;
                let mut other_nodes: Vec<(isize, isize)> = Vec::new();
                for i in 0..8 {
                    let rn = row + dy[i];
                    let cn = col + dx[i];
                    if rn < 0 || rn >= rows || cn < 0 || cn >= cols {
                        continue;
                    }
                    let n_idx = rn as usize * input.cols + cn as usize;
                    if visited[n_idx] != 0 {
                        continue;
                    }
                    let Some(vn) = input.get_raw(0, rn, cn) else { continue };
                    if input.is_nodata(vn) || vn != current_val {
                        continue;
                    }
                    let neigh = num_neigh[n_idx];
                    if neigh > highest {
                        if let Some(rc) = next_rc {
                            other_nodes.push(rc);
                        }
                        highest = neigh;
                        next_rc = Some((rn, cn));
                    } else {
                        other_nodes.push((rn, cn));
                    }
                }

                for rc in other_nodes {
                    queue.push_back(rc);
                }

                if let Some((rn, cn)) = next_rc {
                    row = rn;
                    col = cn;
                } else {
                    break;
                }
            }

            if points.len() > 1 {
                let geom = Geometry::line_string(points.into_iter().map(|(x, y)| wbvector::Coord::xy(x, y)).collect());
                output
                    .add_feature(
                        Some(geom),
                        &[
                            ("FID", FieldValue::Integer(next_fid)),
                            ("VALUE", FieldValue::Float(current_val)),
                        ],
                    )
                    .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
                next_fid += 1;
            }

            let trace_part = if active_cells == 0 {
                1.0
            } else {
                solved_cells as f64 / active_cells as f64
            };
            coalescer.emit_unit_fraction(ctx.progress, 0.2 + trace_part * 0.8);
        }

        // Catch closed loops disconnected from endpoints.
        for row in 0..rows {
            for col in 0..cols {
                let idx = row as usize * input.cols + col as usize;
                if visited[idx] != 0 {
                    continue;
                }

                let current_val = input.get_raw(0, row, col).unwrap_or(input.nodata);
                let mut points: Vec<(f64, f64)> = Vec::new();
                let (mut rr, mut cc) = (row, col);
                loop {
                    let current_idx = rr as usize * input.cols + cc as usize;
                    if visited[current_idx] != 0 {
                        break;
                    }
                    points.push((input.col_center_x(cc), input.row_center_y(rr)));
                    visited[current_idx] = 1;

                    let mut highest = 0i8;
                    let mut next_rc: Option<(isize, isize)> = None;
                    for i in 0..8 {
                        let rn = rr + dy[i];
                        let cn = cc + dx[i];
                        if rn < 0 || rn >= rows || cn < 0 || cn >= cols {
                            continue;
                        }
                        let n_idx = rn as usize * input.cols + cn as usize;
                        if visited[n_idx] != 0 {
                            continue;
                        }
                        let Some(vn) = input.get_raw(0, rn, cn) else { continue };
                        if input.is_nodata(vn) || vn != current_val {
                            continue;
                        }
                        let neigh = num_neigh[n_idx];
                        if neigh > highest {
                            highest = neigh;
                            next_rc = Some((rn, cn));
                        }
                    }

                    if let Some((rn, cn)) = next_rc {
                        rr = rn;
                        cc = cn;
                    } else {
                        break;
                    }
                }

                if points.len() > 1 {
                    let geom = Geometry::line_string(points.into_iter().map(|(x, y)| wbvector::Coord::xy(x, y)).collect());
                    output
                        .add_feature(
                            Some(geom),
                            &[
                                ("FID", FieldValue::Integer(next_fid)),
                                ("VALUE", FieldValue::Float(current_val)),
                            ],
                        )
                        .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
                    next_fid += 1;
                }
            }
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for VectorPointsToRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "vector_points_to_raster",
            display_name: "VectorPointsToRaster",
            summary: "Rasterizes point or multipoint vectors to a grid using a selected assignment operation.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path (point/multipoint).", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, result is in-memory.", required: false },
                ToolParamSpec { name: "field", description: "Optional numeric field used for raster values. Defaults to FID.", required: false },
                ToolParamSpec { name: "assign", description: "Assignment operation: last, first, min, max, sum, num, mean.", required: false },
                ToolParamSpec { name: "zero_background", description: "When true, initializes background to 0; otherwise nodata.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size. Required when 'base' is not provided.", required: false },
                ToolParamSpec { name: "base", description: "Optional base raster path to define extent/grid.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("points.gpkg"));
        defaults.insert("assign".to_string(), json!("last"));
        defaults.insert("cell_size".to_string(), json!(10.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("samples.geojson"));
        example_args.insert("field".to_string(), json!("ELEV"));
        example_args.insert("assign".to_string(), json!("mean"));
        example_args.insert("base".to_string(), json!("dem_base.tif"));
        example_args.insert("output".to_string(), json!("samples_mean.tif"));

        ToolManifest {
            id: "vector_points_to_raster".to_string(),
            display_name: "VectorPointsToRaster".to_string(),
            summary: "Rasterizes point or multipoint vectors to a grid using a selected assignment operation.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path (point/multipoint).".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, result is in-memory.".to_string(), required: false },
                ToolParamDescriptor { name: "field".to_string(), description: "Optional numeric field used for raster values. Defaults to FID.".to_string(), required: false },
                ToolParamDescriptor { name: "assign".to_string(), description: "Assignment operation: last, first, min, max, sum, num, mean.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "When true, initializes background to 0; otherwise nodata.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size. Required when 'base' is not provided.".to_string(), required: false },
                ToolParamDescriptor { name: "base".to_string(), description: "Optional base raster path to define extent/grid.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "mean_to_base".to_string(),
                description: "Rasterize point samples to a base raster grid using mean aggregation.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "raster".to_string(), "rasterize".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_f64(args, "cell_size")?;
        let _ = parse_optional_string(args, "field")?;
        let _ = parse_optional_string(args, "assign")?;
        let _ = parse_optional_string(args, "base")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let field = parse_optional_string(args, "field")?.unwrap_or("FID");
        let assign = parse_assign_op(args);
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = parse_optional_f64(args, "cell_size")?.unwrap_or(0.0);
        let base_path = parse_optional_string(args, "base")?;

        ctx.progress.info("running vector_points_to_raster");
        let input = read_vector_layer(&input_path, "input")?;
        let input_geom_type = input.geom_type.unwrap_or(GeometryType::Point);
        if input_geom_type != GeometryType::Point && input_geom_type != GeometryType::MultiPoint {
            return Err(ToolError::Validation(
                "input vector must contain point or multipoint geometries for vector_points_to_raster".to_string(),
            ));
        }

        let nodata = -32768.0;
        let background = if zero_background { 0.0 } else { nodata };
        let use_fid = field.eq_ignore_ascii_case("FID");
        let field_idx = if use_fid {
            None
        } else {
            input.schema.field_index(field)
        };

        let data_type = if use_fid || assign.contains("num") {
            DataType::I32
        } else {
            DataType::F32
        };

        let mut output = if let Some(base) = base_path {
            let base_raster = Raster::read(base)
                .map_err(|e| ToolError::Execution(format!("failed reading base raster: {e}")))?;
            Raster::new(RasterConfig {
                cols: base_raster.cols,
                rows: base_raster.rows,
                bands: 1,
                x_min: base_raster.x_min,
                y_min: base_raster.y_min,
                cell_size: base_raster.cell_size_x,
                cell_size_y: Some(base_raster.cell_size_y),
                nodata,
                data_type,
                crs: base_raster.crs.clone(),
                metadata: base_raster.metadata.clone(),
            })
        } else {
            if cell_size <= 0.0 {
                return Err(ToolError::Validation(
                    "either 'cell_size' (> 0) or 'base' must be provided".to_string(),
                ));
            }
            let mut input_for_bbox = input.clone();
            let bbox = input_for_bbox.bbox().ok_or_else(|| {
                ToolError::Validation("input vector has no geometry extent".to_string())
            })?;
            let cols = ((bbox.max_x - bbox.min_x) / cell_size).ceil().max(1.0) as usize;
            let rows = ((bbox.max_y - bbox.min_y) / cell_size).ceil().max(1.0) as usize;
            Raster::new(RasterConfig {
                cols,
                rows,
                bands: 1,
                x_min: bbox.min_x,
                y_min: bbox.min_y,
                cell_size,
                cell_size_y: Some(cell_size),
                nodata,
                data_type,
                crs: vector_to_raster_crs(&input),
                metadata: Vec::new(),
            })
        };

        if background != nodata {
            output.fill(background);
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        let mut counts: Option<Vec<f64>> = if assign.contains("mean") || assign.contains("average") {
            Some(vec![0.0; output.rows * output.cols])
        } else {
            None
        };

        for (feat_idx, feature) in input.features.iter().enumerate() {
            let Some(geom) = &feature.geometry else {
                continue;
            };
            let value = if use_fid {
                feat_idx as f64 + 1.0
            } else if let Some(idx) = field_idx {
                feature
                    .attributes
                    .get(idx)
                    .and_then(field_value_as_f64)
                    .unwrap_or(feat_idx as f64 + 1.0)
            } else {
                feat_idx as f64 + 1.0
            };

            for (x, y) in feature_points(geom) {
                let Some((col, row)) = output.world_to_pixel(x, y) else {
                    continue;
                };
                let existing = output.get(0, row, col);
                let incoming = if assign.contains("num") {
                    1.0
                } else {
                    value
                };

                if assign.contains("first") {
                    if output.is_nodata(existing) || existing == background {
                        output
                            .set(0, row, col, incoming)
                            .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                    }
                } else if assign.contains("min") {
                    if output.is_nodata(existing) || existing == background || incoming < existing {
                        output
                            .set(0, row, col, incoming)
                            .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                    }
                } else if assign.contains("max") {
                    if output.is_nodata(existing) || existing == background || incoming > existing {
                        output
                            .set(0, row, col, incoming)
                            .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                    }
                } else if assign.contains("sum") || assign.contains("total") || assign.contains("num") {
                    let updated = if output.is_nodata(existing) || existing == background {
                        incoming
                    } else {
                        existing + incoming
                    };
                    output
                        .set(0, row, col, updated)
                        .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                } else if assign.contains("mean") || assign.contains("average") {
                    let updated = if output.is_nodata(existing) || existing == background {
                        incoming
                    } else {
                        existing + incoming
                    };
                    output
                        .set(0, row, col, updated)
                        .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                    if let Some(ref mut n) = counts {
                        let idx = row as usize * output.cols + col as usize;
                        n[idx] += 1.0;
                    }
                } else {
                    // default: last
                    output
                        .set(0, row, col, incoming)
                        .map_err(|e| ToolError::Execution(format!("failed setting raster value: {e}")))?;
                }
            }

            coalescer.emit_unit_fraction(ctx.progress, (feat_idx + 1) as f64 / total * 0.9);
        }

        if let Some(n) = counts {
            for row in 0..output.rows as isize {
                for col in 0..output.cols as isize {
                    let idx = row as usize * output.cols + col as usize;
                    if n[idx] > 0.0 {
                        let sum = output.get(0, row, col);
                        output
                            .set(0, row, col, sum / n[idx])
                            .map_err(|e| ToolError::Execution(format!("failed finalizing mean value: {e}")))?;
                    }
                }
            }
        }

        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for NewRasterFromBaseVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "new_raster_from_base_vector",
            display_name: "NewRasterFromBaseVector",
            summary: "Creates a new raster from a base vector extent and cell size, filled with an optional value.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base vector path.", required: true },
                ToolParamSpec { name: "cell_size", description: "Output grid cell size (> 0).", required: true },
                ToolParamSpec { name: "out_val", description: "Optional fill value. Defaults to nodata (-32768).", required: false },
                ToolParamSpec { name: "data_type", description: "Optional output data type keyword: integer, float, or double.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, output remains in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("boundary.gpkg"));
        defaults.insert("cell_size".to_string(), json!(10.0));
        defaults.insert("out_val".to_string(), json!(-32768.0));
        defaults.insert("data_type".to_string(), json!("float"));

        let mut example_args = ToolArgs::new();
        example_args.insert("base".to_string(), json!("study_area.geojson"));
        example_args.insert("cell_size".to_string(), json!(5.0));
        example_args.insert("out_val".to_string(), json!(0.0));
        example_args.insert("data_type".to_string(), json!("integer"));
        example_args.insert("output".to_string(), json!("new_grid.tif"));

        ToolManifest {
            id: "new_raster_from_base_vector".to_string(),
            display_name: "NewRasterFromBaseVector".to_string(),
            summary: "Creates a new raster from a base vector extent and cell size, filled with an optional value.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output grid cell size (> 0).".to_string(), required: true },
                ToolParamDescriptor { name: "out_val".to_string(), description: "Optional fill value. Defaults to nodata (-32768).".to_string(), required: false },
                ToolParamDescriptor { name: "data_type".to_string(), description: "Optional output data type keyword: integer, float, or double.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, output remains in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "new_grid_from_vector".to_string(),
                description: "Create a zero-filled integer raster from vector extent.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "vector".to_string(), "grid".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "base")?;
        let cell_size = parse_optional_f64(args, "cell_size")?
            .ok_or_else(|| ToolError::Validation("parameter 'cell_size' is required".to_string()))?;
        if cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'cell_size' must be greater than zero".to_string(),
            ));
        }
        let _ = parse_optional_f64(args, "out_val")?;
        let _ = parse_optional_string(args, "data_type")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_vector_path_arg(args, "base")?;
        let cell_size = parse_optional_f64(args, "cell_size")?
            .ok_or_else(|| ToolError::Validation("parameter 'cell_size' is required".to_string()))?;
        if cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'cell_size' must be greater than zero".to_string(),
            ));
        }
        let output_path = parse_optional_output_path(args, "output")?;
        let nodata = -32768.0;
        let out_val = parse_optional_f64(args, "out_val")?.unwrap_or(nodata);
        let data_type_str = parse_optional_string(args, "data_type")?.unwrap_or("float");

        ctx.progress.info("running new_raster_from_base_vector");
        let base = read_vector_layer(&base_path, "base")?;
        let mut base_for_bbox = base.clone();
        let bbox = base_for_bbox
            .bbox()
            .ok_or_else(|| ToolError::Validation("base vector has no geometry extent".to_string()))?;

        let west = bbox.min_x;
        let north = bbox.max_y;
        let rows = ((north - bbox.min_y) / cell_size).ceil().max(1.0) as usize;
        let cols = ((bbox.max_x - west) / cell_size).ceil().max(1.0) as usize;

        let data_type = {
            let lower = data_type_str.to_ascii_lowercase();
            if lower.contains('i') {
                DataType::I16
            } else if lower.contains('d') {
                DataType::F64
            } else {
                DataType::F32
            }
        };

        let mut output = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: west,
            y_min: north - rows as f64 * cell_size,
            cell_size,
            cell_size_y: Some(cell_size),
            nodata,
            data_type,
            crs: vector_to_raster_crs(&base),
            metadata: Vec::new(),
        });

        if out_val != nodata {
            output.fill(out_val);
        }

        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for RemoveRasterPolygonHolesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "remove_raster_polygon_holes",
            display_name: "RemoveRasterPolygonHoles",
            summary: "Removes interior background holes (0 or nodata regions enclosed by foreground) from raster polygons.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path (single-band).", required: true },
                ToolParamSpec { name: "threshold", description: "Optional maximum hole size in cells to remove. 0 means all enclosed holes.", required: false },
                ToolParamSpec { name: "use_diagonals", description: "Use 8-neighbour connectedness for clumping and edge connectivity.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, output remains in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("classified.tif"));
        defaults.insert("threshold".to_string(), json!(0));
        defaults.insert("use_diagonals".to_string(), json!(false));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("water_mask.tif"));
        example_args.insert("threshold".to_string(), json!(500));
        example_args.insert("use_diagonals".to_string(), json!(true));
        example_args.insert("output".to_string(), json!("water_no_holes.tif"));

        ToolManifest {
            id: "remove_raster_polygon_holes".to_string(),
            display_name: "RemoveRasterPolygonHoles".to_string(),
            summary: "Removes interior background holes (0 or nodata regions enclosed by foreground) from raster polygons.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path (single-band).".to_string(), required: true },
                ToolParamDescriptor { name: "threshold".to_string(), description: "Optional maximum hole size in cells to remove. 0 means all enclosed holes.".to_string(), required: false },
                ToolParamDescriptor { name: "use_diagonals".to_string(), description: "Use 8-neighbour connectedness for clumping and edge connectivity.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, output remains in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "remove_small_holes".to_string(),
                description: "Remove enclosed background holes smaller than a threshold size.".to_string(),
                args: example_args,
            }],
            tags: vec!["data-tools".to_string(), "raster".to_string(), "holes".to_string(), "morphology".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_usize(args, "threshold")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut threshold = parse_optional_usize(args, "threshold")?.unwrap_or(usize::MAX);
        if threshold == 0 {
            threshold = usize::MAX;
        }
        let use_diagonals = args
            .get("use_diagonals")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        ctx.progress.info("running remove_raster_polygon_holes");
        let input = Raster::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        if input.bands != 1 {
            return Err(ToolError::Validation(
                "input raster must be single-band for remove_raster_polygon_holes".to_string(),
            ));
        }

        let rows = input.rows as isize;
        let cols = input.cols as isize;
        let is_bg = |v: f64| input.is_nodata(v) || v == 0.0;

        let (dx, dy): (Vec<isize>, Vec<isize>) = if use_diagonals {
            (
                vec![1, 1, 1, 0, -1, -1, -1, 0],
                vec![-1, 0, 1, 1, 1, 0, -1, -1],
            )
        } else {
            (vec![1, 0, -1, 0], vec![0, 1, 0, -1])
        };

        let mut labels = vec![-1i32; input.rows * input.cols];
        let mut clump_sizes: Vec<usize> = vec![0];
        let mut clump_touches_edge: Vec<bool> = vec![false];
        let mut next_label = 1i32;
        let mut stack: Vec<(isize, isize)> = Vec::new();

        for row in 0..rows {
            for col in 0..cols {
                let idx = row as usize * input.cols + col as usize;
                if labels[idx] != -1 {
                    continue;
                }
                let v = input.get(0, row, col);
                if !is_bg(v) {
                    labels[idx] = 0;
                    continue;
                }

                labels[idx] = next_label;
                stack.push((row, col));
                let mut size = 0usize;
                let mut touches_edge = row == 0 || col == 0 || row == rows - 1 || col == cols - 1;

                while let Some((r, c)) = stack.pop() {
                    size += 1;
                    for n in 0..dx.len() {
                        let rn = r + dy[n];
                        let cn = c + dx[n];
                        if rn < 0 || rn >= rows || cn < 0 || cn >= cols {
                            continue;
                        }
                        if rn == 0 || cn == 0 || rn == rows - 1 || cn == cols - 1 {
                            touches_edge = true;
                        }
                        let n_idx = rn as usize * input.cols + cn as usize;
                        if labels[n_idx] != -1 {
                            continue;
                        }
                        let vn = input.get(0, rn, cn);
                        if is_bg(vn) {
                            labels[n_idx] = next_label;
                            stack.push((rn, cn));
                        } else {
                            labels[n_idx] = 0;
                        }
                    }
                }

                clump_sizes.push(size);
                clump_touches_edge.push(touches_edge);
                next_label += 1;
            }
            ctx.progress.progress((row as f64 + 1.0) / rows.max(1) as f64 * 0.35);
        }

        let mut output = input.clone();

        // For each removable hole clump, fill with the most common neighbouring foreground value.
        for label in 1..next_label {
            let lidx = label as usize;
            if clump_touches_edge[lidx] || clump_sizes[lidx] >= threshold {
                continue;
            }

            let mut value_counts: std::collections::HashMap<u64, (f64, usize)> =
                std::collections::HashMap::new();
            let mut cells: Vec<(isize, isize)> = Vec::with_capacity(clump_sizes[lidx]);

            for row in 0..rows {
                for col in 0..cols {
                    let idx = row as usize * input.cols + col as usize;
                    if labels[idx] != label {
                        continue;
                    }
                    cells.push((row, col));
                    for n in 0..8 {
                        let ndx = [1isize, 1, 1, 0, -1, -1, -1, 0][n];
                        let ndy = [-1isize, 0, 1, 1, 1, 0, -1, -1][n];
                        let rn = row + ndy;
                        let cn = col + ndx;
                        if rn < 0 || rn >= rows || cn < 0 || cn >= cols {
                            continue;
                        }
                        let n_idx = rn as usize * input.cols + cn as usize;
                        if labels[n_idx] == label {
                            continue;
                        }
                        let v = output.get(0, rn, cn);
                        if is_bg(v) {
                            continue;
                        }
                        let key = v.to_bits();
                        let entry = value_counts.entry(key).or_insert((v, 0));
                        entry.1 += 1;
                    }
                }
            }

            let fill_value = if let Some((_, (v, _))) = value_counts
                .iter()
                .max_by_key(|(_, (_, count))| *count)
            {
                *v
            } else {
                1.0
            };

            for (row, col) in cells {
                output
                    .set(0, row, col, fill_value)
                    .map_err(|e| ToolError::Execution(format!("failed filling hole cell: {e}")))?;
            }
        }

        ctx.progress.progress(1.0);
        write_raster_output(output, output_path, ctx)
    }
}

impl Tool for CsvPointsToVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "csv_points_to_vector",
            display_name: "CsvPointsToVector",
            summary: "Imports point records from a CSV file into a point vector layer.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_file", description: "Input CSV file path.", required: true },
                ToolParamSpec { name: "x_field_num", description: "Zero-based index for X coordinate field. Defaults to 0.", required: false },
                ToolParamSpec { name: "y_field_num", description: "Zero-based index for Y coordinate field. Defaults to 1.", required: false },
                ToolParamSpec { name: "epsg", description: "Optional EPSG code for output CRS.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input_file".to_string(), json!("points.csv"));
        defaults.insert("x_field_num".to_string(), json!(0));
        defaults.insert("y_field_num".to_string(), json!(1));
        defaults.insert("output".to_string(), json!("points.geojson"));

        let mut example = ToolArgs::new();
        example.insert("input_file".to_string(), json!("samples.csv"));
        example.insert("x_field_num".to_string(), json!(2));
        example.insert("y_field_num".to_string(), json!(3));
        example.insert("epsg".to_string(), json!(4326));
        example.insert("output".to_string(), json!("samples_points.gpkg"));

        ToolManifest {
            id: "csv_points_to_vector".to_string(),
            display_name: "CsvPointsToVector".to_string(),
            summary: "Imports point records from a CSV file into a point vector layer.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input_file".to_string(), description: "Input CSV file path.".to_string(), required: true },
                ToolParamDescriptor { name: "x_field_num".to_string(), description: "Zero-based index for X coordinate field. Defaults to 0.".to_string(), required: false },
                ToolParamDescriptor { name: "y_field_num".to_string(), description: "Zero-based index for Y coordinate field. Defaults to 1.".to_string(), required: false },
                ToolParamDescriptor { name: "epsg".to_string(), description: "Optional EPSG code for output CRS.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "csv_import".to_string(),
                description: "Import CSV points with explicit X/Y fields.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "csv".to_string(), "vector".to_string(), "points".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_optional_string(args, "input_file")?
            .ok_or_else(|| ToolError::Validation("parameter 'input_file' is required".to_string()))?;
        let _ = parse_vector_path_arg(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_file = parse_optional_string(args, "input_file")?
            .ok_or_else(|| ToolError::Validation("parameter 'input_file' is required".to_string()))?;
        let output_path = PathBuf::from(parse_vector_path_arg(args, "output")?);
        let x_field = parse_optional_usize(args, "x_field_num")?.unwrap_or(0);
        let y_field = parse_optional_usize(args, "y_field_num")?.unwrap_or(1);
        let epsg = parse_optional_usize(args, "epsg")?.and_then(|v| u32::try_from(v).ok());

        let (headers, rows) = parse_csv_table(input_file)?;
        if headers.is_empty() || rows.is_empty() {
            return Err(ToolError::Validation("csv file does not contain data rows".to_string()));
        }
        if x_field >= headers.len() || y_field >= headers.len() {
            return Err(ToolError::Validation(
                "x_field_num or y_field_num index out of bounds".to_string(),
            ));
        }

        let mut field_types: Vec<FieldType> = Vec::with_capacity(headers.len());
        for (idx, _) in headers.iter().enumerate() {
            if idx == x_field || idx == y_field {
                field_types.push(FieldType::Float);
            } else {
                let samples = rows.iter().map(|r| r[idx].clone()).collect::<Vec<_>>();
                field_types.push(infer_field_type(&samples));
            }
        }

        let mut output = Layer::new(
            Path::new(input_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("csv_points"),
        )
        .with_geom_type(GeometryType::Point);
        if let Some(epsg_code) = epsg {
            output.set_crs_epsg(Some(epsg_code));
        }

        for (name, ty) in headers.iter().zip(field_types.iter()) {
            output.add_field(FieldDef::new(name.clone(), *ty));
        }

        for row in &rows {
            let x = row[x_field].trim().parse::<f64>().map_err(|_| {
                ToolError::Validation(format!("failed parsing X coordinate '{}': expected numeric", row[x_field]))
            })?;
            let y = row[y_field].trim().parse::<f64>().map_err(|_| {
                ToolError::Validation(format!("failed parsing Y coordinate '{}': expected numeric", row[y_field]))
            })?;

            let attrs: Vec<(&str, FieldValue)> = headers
                .iter()
                .enumerate()
                .map(|(i, h)| (h.as_str(), parse_typed_value(&row[i], field_types[i])))
                .collect();

            output
                .add_feature(Some(Geometry::point(x, y)), &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding output feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for ExportTableToCsvTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "export_table_to_csv",
            display_name: "ExportTableToCsv",
            summary: "Exports a vector attribute table to a CSV file.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "output_csv_file", description: "Output CSV file path.", required: true },
                ToolParamSpec { name: "headers", description: "Include header row in output. Defaults to true.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.gpkg"));
        defaults.insert("output_csv_file".to_string(), json!("table.csv"));
        defaults.insert("headers".to_string(), json!(true));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("parcels.geojson"));
        example.insert("output_csv_file".to_string(), json!("parcels_table.csv"));
        example.insert("headers".to_string(), json!(true));

        ToolManifest {
            id: "export_table_to_csv".to_string(),
            display_name: "ExportTableToCsv".to_string(),
            summary: "Exports a vector attribute table to a CSV file.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "output_csv_file".to_string(), description: "Output CSV file path.".to_string(), required: true },
                ToolParamDescriptor { name: "headers".to_string(), description: "Include header row in output. Defaults to true.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "export_csv".to_string(),
                description: "Export attribute table to CSV.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "csv".to_string(), "attributes".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_string(args, "output_csv_file")?
            .ok_or_else(|| ToolError::Validation("parameter 'output_csv_file' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let mut output_csv = parse_optional_string(args, "output_csv_file")?
            .ok_or_else(|| ToolError::Validation("parameter 'output_csv_file' is required".to_string()))?
            .to_string();
        if !output_csv.to_ascii_lowercase().ends_with(".csv") {
            output_csv.push_str(".csv");
        }
        let output_path = PathBuf::from(output_csv);
        ensure_parent_dir(&output_path)?;
        let headers = args.get("headers").and_then(|v| v.as_bool()).unwrap_or(true);

        let input = read_vector_layer(&input_path, "input")?;
        let file = File::create(&output_path)
            .map_err(|e| ToolError::Execution(format!("failed creating csv output: {e}")))?;
        let mut writer = BufWriter::new(file);

        if headers {
            let line = input
                .schema
                .fields()
                .iter()
                .map(|f| format!("\"{}\"", f.name.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",");
            writer
                .write_all(format!("{}\n", line).as_bytes())
                .map_err(|e| ToolError::Execution(format!("failed writing csv header: {e}")))?;
        }

        for feature in &input.features {
            let row = input
                .schema
                .fields()
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let v = feature.attributes.get(i).unwrap_or(&FieldValue::Null);
                    field_value_to_csv(v)
                })
                .collect::<Vec<_>>()
                .join(",");
            writer
                .write_all(format!("{}\n", row).as_bytes())
                .map_err(|e| ToolError::Execution(format!("failed writing csv row: {e}")))?;
        }

        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_path.to_string_lossy().to_string()));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for JoinTablesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "join_tables",
            display_name: "JoinTables",
            summary: "Joins attributes from a foreign vector table to a primary vector table using key fields.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "primary_vector", description: "Primary vector path to receive joined fields.", required: true },
                ToolParamSpec { name: "primary_key_field", description: "Primary key field in primary vector.", required: true },
                ToolParamSpec { name: "foreign_vector", description: "Foreign vector path containing source fields.", required: true },
                ToolParamSpec { name: "foreign_key_field", description: "Foreign key field in foreign vector.", required: true },
                ToolParamSpec { name: "import_field", description: "Optional single field to import; defaults to all non-key, non-FID fields.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("primary_vector".to_string(), json!("countries.gpkg"));
        defaults.insert("primary_key_field".to_string(), json!("COUNTRY"));
        defaults.insert("foreign_vector".to_string(), json!("country_stats.gpkg"));
        defaults.insert("foreign_key_field".to_string(), json!("COUNTRY"));

        let mut example = ToolArgs::new();
        example.insert("primary_vector".to_string(), json!("countries.gpkg"));
        example.insert("primary_key_field".to_string(), json!("COUNTRY"));
        example.insert("foreign_vector".to_string(), json!("country_stats.gpkg"));
        example.insert("foreign_key_field".to_string(), json!("COUNTRY"));
        example.insert("import_field".to_string(), json!("POPULATION"));
        example.insert("output".to_string(), json!("countries_joined.gpkg"));

        ToolManifest {
            id: "join_tables".to_string(),
            display_name: "JoinTables".to_string(),
            summary: "Joins attributes from a foreign vector table to a primary vector table using key fields.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "primary_vector".to_string(), description: "Primary vector path to receive joined fields.".to_string(), required: true },
                ToolParamDescriptor { name: "primary_key_field".to_string(), description: "Primary key field in primary vector.".to_string(), required: true },
                ToolParamDescriptor { name: "foreign_vector".to_string(), description: "Foreign vector path containing source fields.".to_string(), required: true },
                ToolParamDescriptor { name: "foreign_key_field".to_string(), description: "Foreign key field in foreign vector.".to_string(), required: true },
                ToolParamDescriptor { name: "import_field".to_string(), description: "Optional single field to import; defaults to all non-key, non-FID fields.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. Defaults to a derived GeoJSON path beside primary input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "join_pop".to_string(),
                description: "Join one field by country key.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "table".to_string(), "join".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "primary_vector")?;
        let _ = parse_vector_path_arg(args, "foreign_vector")?;
        let _ = parse_optional_string(args, "primary_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'primary_key_field' is required".to_string()))?;
        let _ = parse_optional_string(args, "foreign_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_key_field' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let primary_path = parse_vector_path_arg(args, "primary_vector")?;
        let foreign_path = parse_vector_path_arg(args, "foreign_vector")?;
        let primary_key = parse_optional_string(args, "primary_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'primary_key_field' is required".to_string()))?;
        let foreign_key = parse_optional_string(args, "foreign_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_key_field' is required".to_string()))?;
        let import_field = parse_optional_string(args, "import_field")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&primary_path, "joined"));

        ctx.progress.info("running join_tables");
        let primary = read_vector_layer(&primary_path, "primary_vector")?;
        let foreign = read_vector_layer(&foreign_path, "foreign_vector")?;

        let primary_key_idx = primary.schema.field_index(primary_key).ok_or_else(|| {
            ToolError::Validation(format!("primary key field '{}' not found", primary_key))
        })?;
        let foreign_key_idx = foreign.schema.field_index(foreign_key).ok_or_else(|| {
            ToolError::Validation(format!("foreign key field '{}' not found", foreign_key))
        })?;

        let fields_to_append: Vec<FieldDef> = if let Some(import_name) = import_field {
            vec![foreign
                .schema
                .field(import_name)
                .ok_or_else(|| ToolError::Validation(format!("import field '{}' not found", import_name)))?
                .clone()]
        } else {
            foreign
                .schema
                .fields()
                .iter()
                .enumerate()
                .filter(|(i, f)| *i != foreign_key_idx && f.name.to_ascii_lowercase() != "fid")
                .map(|(_, f)| f.clone())
                .collect()
        };

        let mut map: HashMap<String, Vec<FieldValue>> = HashMap::new();
        for feat in &foreign.features {
            let key = feat
                .attributes
                .get(foreign_key_idx)
                .cloned()
                .unwrap_or(FieldValue::Null)
                .to_string();
            let vals = fields_to_append
                .iter()
                .map(|f| {
                    let idx = foreign.schema.field_index(&f.name).unwrap_or(usize::MAX);
                    if idx == usize::MAX {
                        FieldValue::Null
                    } else {
                        feat.attributes.get(idx).cloned().unwrap_or(FieldValue::Null)
                    }
                })
                .collect::<Vec<_>>();
            map.insert(key, vals);
        }

        let mut output = Layer::new(primary.name.clone());
        output.geom_type = primary.geom_type;
        apply_input_crs_to_layer(&primary, &mut output);
        for f in primary.schema.fields() {
            output.add_field(f.clone());
        }
        for f in &fields_to_append {
            output.add_field(f.clone());
        }

        for feature in &primary.features {
            let mut attrs = clone_feature_attrs(&primary, feature);
            let key = feature
                .attributes
                .get(primary_key_idx)
                .cloned()
                .unwrap_or(FieldValue::Null)
                .to_string();
            if let Some(extra_vals) = map.get(&key) {
                for (field_def, value) in fields_to_append.iter().zip(extra_vals.iter()) {
                    attrs.push((field_def.name.as_str(), value.clone()));
                }
            } else {
                for field_def in &fields_to_append {
                    attrs.push((field_def.name.as_str(), FieldValue::Null));
                }
            }

            output
                .add_feature(feature.geometry.clone(), &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding joined feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for MergeTableWithCsvTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "merge_table_with_csv",
            display_name: "MergeTableWithCsv",
            summary: "Merges attributes from a CSV table into a vector attribute table by key fields.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "primary_vector", description: "Primary vector path to receive merged fields.", required: true },
                ToolParamSpec { name: "primary_key_field", description: "Primary key field in primary vector.", required: true },
                ToolParamSpec { name: "foreign_csv_filename", description: "CSV file containing foreign table.", required: true },
                ToolParamSpec { name: "foreign_key_field", description: "Foreign key field name in CSV header.", required: true },
                ToolParamSpec { name: "import_field", description: "Optional single CSV field to import; defaults to all non-key fields.", required: false },
                ToolParamSpec { name: "output", description: "Output vector path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("primary_vector".to_string(), json!("countries.gpkg"));
        defaults.insert("primary_key_field".to_string(), json!("COUNTRY"));
        defaults.insert("foreign_csv_filename".to_string(), json!("country_stats.csv"));
        defaults.insert("foreign_key_field".to_string(), json!("COUNTRY"));

        let mut example = ToolArgs::new();
        example.insert("primary_vector".to_string(), json!("countries.gpkg"));
        example.insert("primary_key_field".to_string(), json!("COUNTRY"));
        example.insert("foreign_csv_filename".to_string(), json!("country_stats.csv"));
        example.insert("foreign_key_field".to_string(), json!("COUNTRY"));
        example.insert("import_field".to_string(), json!("GDP"));
        example.insert("output".to_string(), json!("countries_merged.gpkg"));

        ToolManifest {
            id: "merge_table_with_csv".to_string(),
            display_name: "MergeTableWithCsv".to_string(),
            summary: "Merges attributes from a CSV table into a vector attribute table by key fields.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "primary_vector".to_string(), description: "Primary vector path to receive merged fields.".to_string(), required: true },
                ToolParamDescriptor { name: "primary_key_field".to_string(), description: "Primary key field in primary vector.".to_string(), required: true },
                ToolParamDescriptor { name: "foreign_csv_filename".to_string(), description: "CSV file containing foreign table.".to_string(), required: true },
                ToolParamDescriptor { name: "foreign_key_field".to_string(), description: "Foreign key field name in CSV header.".to_string(), required: true },
                ToolParamDescriptor { name: "import_field".to_string(), description: "Optional single CSV field to import; defaults to all non-key fields.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Output vector path. Defaults to a derived GeoJSON path beside primary input.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "merge_csv_field".to_string(),
                description: "Merge one CSV field into vector table by key.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "table".to_string(), "csv".to_string(), "join".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "primary_vector")?;
        let _ = parse_optional_string(args, "primary_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'primary_key_field' is required".to_string()))?;
        let _ = parse_optional_string(args, "foreign_csv_filename")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_csv_filename' is required".to_string()))?;
        let _ = parse_optional_string(args, "foreign_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_key_field' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let primary_path = parse_vector_path_arg(args, "primary_vector")?;
        let primary_key = parse_optional_string(args, "primary_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'primary_key_field' is required".to_string()))?;
        let csv_path = parse_optional_string(args, "foreign_csv_filename")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_csv_filename' is required".to_string()))?;
        let foreign_key = parse_optional_string(args, "foreign_key_field")?
            .ok_or_else(|| ToolError::Validation("parameter 'foreign_key_field' is required".to_string()))?;
        let import_field = parse_optional_string(args, "import_field")?;
        let output_path = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| derived_vector_output_path(&primary_path, "merged_csv"));

        ctx.progress.info("running merge_table_with_csv");
        let primary = read_vector_layer(&primary_path, "primary_vector")?;
        let primary_key_idx = primary.schema.field_index(primary_key).ok_or_else(|| {
            ToolError::Validation(format!("primary key field '{}' not found", primary_key))
        })?;

        let (headers, rows) = parse_csv_table(csv_path)?;
        let foreign_key_idx = headers
            .iter()
            .position(|h| h == foreign_key)
            .ok_or_else(|| ToolError::Validation(format!("foreign key field '{}' not found in csv", foreign_key)))?;

        let append_indices: Vec<usize> = if let Some(import_name) = import_field {
            vec![headers
                .iter()
                .position(|h| h == import_name)
                .ok_or_else(|| ToolError::Validation(format!("import field '{}' not found in csv", import_name)))?]
        } else {
            headers
                .iter()
                .enumerate()
                .filter(|(i, h)| *i != foreign_key_idx && h.to_ascii_lowercase() != "fid")
                .map(|(i, _)| i)
                .collect()
        };

        let mut append_types: Vec<FieldType> = Vec::new();
        for idx in &append_indices {
            let samples = rows.iter().map(|r| r[*idx].clone()).collect::<Vec<_>>();
            append_types.push(infer_field_type(&samples));
        }

        let mut map: HashMap<String, Vec<FieldValue>> = HashMap::new();
        for row in &rows {
            let key = row[foreign_key_idx].trim().to_string();
            let vals = append_indices
                .iter()
                .zip(append_types.iter())
                .map(|(i, ty)| parse_typed_value(&row[*i], *ty))
                .collect::<Vec<_>>();
            map.insert(key, vals);
        }

        let mut output = Layer::new(primary.name.clone());
        output.geom_type = primary.geom_type;
        apply_input_crs_to_layer(&primary, &mut output);
        for f in primary.schema.fields() {
            output.add_field(f.clone());
        }
        for (i, idx) in append_indices.iter().enumerate() {
            output.add_field(FieldDef::new(headers[*idx].clone(), append_types[i]));
        }

        for feature in &primary.features {
            let mut attrs = clone_feature_attrs(&primary, feature);
            let key = feature
                .attributes
                .get(primary_key_idx)
                .cloned()
                .unwrap_or(FieldValue::Null)
                .to_string();
            if let Some(extra_vals) = map.get(&key) {
                for (i, idx) in append_indices.iter().enumerate() {
                    attrs.push((headers[*idx].as_str(), extra_vals[i].clone()));
                }
            } else {
                for idx in &append_indices {
                    attrs.push((headers[*idx].as_str(), FieldValue::Null));
                }
            }
            output
                .add_feature(feature.geometry.clone(), &attrs)
                .map_err(|e| ToolError::Execution(format!("failed adding merged feature: {e}")))?;
        }

        write_vector_output(&output, &output_path)
    }
}

impl Tool for VectorPolygonsToRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "vector_polygons_to_raster",
            display_name: "VectorPolygonsToRaster",
            summary: "Rasterizes polygon vectors to a grid, supporting attribute-driven burn values.",
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input polygon vector path.", required: true },
                ToolParamSpec { name: "field", description: "Optional numeric field name for burn values (defaults to FID).", required: false },
                ToolParamSpec { name: "zero_background", description: "When true, initializes output background to 0 instead of nodata.", required: false },
                ToolParamSpec { name: "cell_size", description: "Output cell size when 'base' is not supplied.", required: false },
                ToolParamSpec { name: "base", description: "Optional base raster path defining output grid and extent.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, output remains in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("polygons.gpkg"));
        defaults.insert("field".to_string(), json!("FID"));
        defaults.insert("cell_size".to_string(), json!(10.0));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("landcover_polys.gpkg"));
        example.insert("field".to_string(), json!("CLASS_ID"));
        example.insert("base".to_string(), json!("base.tif"));
        example.insert("output".to_string(), json!("landcover_raster.tif"));

        ToolManifest {
            id: "vector_polygons_to_raster".to_string(),
            display_name: "VectorPolygonsToRaster".to_string(),
            summary: "Rasterizes polygon vectors to a grid, supporting attribute-driven burn values.".to_string(),
            category: ToolCategory::Conversion,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input polygon vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Optional numeric field name for burn values (defaults to FID).".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "When true, initializes output background to 0 instead of nodata.".to_string(), required: false },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output cell size when 'base' is not supplied.".to_string(), required: false },
                ToolParamDescriptor { name: "base".to_string(), description: "Optional base raster path defining output grid and extent.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path. If omitted, output remains in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "polygons_to_grid".to_string(),
                description: "Rasterize polygons to a base grid.".to_string(),
                args: example,
            }],
            tags: vec!["data-tools".to_string(), "vector".to_string(), "raster".to_string(), "polygons".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_f64(args, "cell_size")?;
        let _ = parse_optional_string(args, "base")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let field_name = parse_optional_string(args, "field")?.unwrap_or("FID");
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let cell_size = parse_optional_f64(args, "cell_size")?.unwrap_or(0.0);
        let base_path = parse_optional_string(args, "base")?;

        if base_path.is_none() && cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "either 'cell_size' (> 0) or 'base' must be provided".to_string(),
            ));
        }

        ctx.progress.info("running vector_polygons_to_raster");
        let input = read_vector_layer(&input_path, "input")?;
        let gt = input.geom_type.unwrap_or(GeometryType::Polygon);
        if gt != GeometryType::Polygon && gt != GeometryType::MultiPolygon {
            return Err(ToolError::Validation(
                "input vector must contain polygon or multipolygon geometries".to_string(),
            ));
        }

        let nodata = -32768.0;
        let background = if zero_background { 0.0 } else { nodata };
        let use_fid = field_name.eq_ignore_ascii_case("FID");
        let field_idx = if use_fid {
            None
        } else {
            input.schema.field_index(field_name)
        };
        let data_type = if use_fid { DataType::I32 } else { DataType::F32 };

        let mut output = if let Some(base) = base_path {
            let base_raster = Raster::read(base)
                .map_err(|e| ToolError::Execution(format!("failed reading base raster: {e}")))?;
            Raster::new(RasterConfig {
                cols: base_raster.cols,
                rows: base_raster.rows,
                bands: 1,
                x_min: base_raster.x_min,
                y_min: base_raster.y_min,
                cell_size: base_raster.cell_size_x,
                cell_size_y: Some(base_raster.cell_size_y),
                nodata,
                data_type,
                crs: base_raster.crs.clone(),
                metadata: base_raster.metadata.clone(),
            })
        } else {
            let mut input_for_bbox = input.clone();
            let bbox = input_for_bbox
                .bbox()
                .ok_or_else(|| ToolError::Validation("input vector has no geometry extent".to_string()))?;
            let cols = ((bbox.max_x - bbox.min_x) / cell_size).ceil().max(1.0) as usize;
            let rows = ((bbox.max_y - bbox.min_y) / cell_size).ceil().max(1.0) as usize;
            Raster::new(RasterConfig {
                cols,
                rows,
                bands: 1,
                x_min: bbox.min_x,
                y_min: bbox.min_y,
                cell_size,
                cell_size_y: Some(cell_size),
                nodata,
                data_type,
                crs: vector_to_raster_crs(&input),
                metadata: Vec::new(),
            })
        };

        if background != nodata {
            output.fill(background);
        }

        let total = input.features.len().max(1) as f64;
        let coalescer = PercentCoalescer::new(1, 99);
        for (feat_idx, feature) in input.features.iter().enumerate() {
            let Some(geometry) = &feature.geometry else {
                continue;
            };
            let burn = if use_fid {
                feat_idx as f64 + 1.0
            } else if let Some(idx) = field_idx {
                feature
                    .attributes
                    .get(idx)
                    .and_then(field_value_as_f64)
                    .unwrap_or(feat_idx as f64 + 1.0)
            } else {
                feat_idx as f64 + 1.0
            };

            let polygons: Vec<(Ring, Vec<Ring>)> = match geometry {
                Geometry::Polygon { exterior, interiors } => vec![(exterior.clone(), interiors.clone())],
                Geometry::MultiPolygon(polys) => polys.clone(),
                _ => Vec::new(),
            };

            for (exterior, holes) in polygons {
                if exterior.0.is_empty() {
                    continue;
                }
                let min_x = exterior.0.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
                let max_x = exterior.0.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
                let min_y = exterior.0.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
                let max_y = exterior.0.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);

                let (min_col, min_row) = output.world_to_pixel(min_x, max_y).unwrap_or((0, 0));
                let (max_col, max_row) = output
                    .world_to_pixel(max_x, min_y)
                    .unwrap_or((output.cols as isize - 1, output.rows as isize - 1));

                let c0 = min_col.clamp(0, output.cols as isize - 1);
                let c1 = max_col.clamp(0, output.cols as isize - 1);
                let r0 = min_row.clamp(0, output.rows as isize - 1);
                let r1 = max_row.clamp(0, output.rows as isize - 1);

                let ext = exterior.0.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>();
                let hole_rings = holes
                    .iter()
                    .map(|h| h.0.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>())
                    .collect::<Vec<_>>();

                for row in r0..=r1 {
                    let y = output.row_center_y(row);
                    for col in c0..=c1 {
                        let x = output.col_center_x(col);
                        if !point_in_ring((x, y), &ext) {
                            continue;
                        }
                        let mut in_hole = false;
                        for h in &hole_rings {
                            if point_in_ring((x, y), h) {
                                in_hole = true;
                                break;
                            }
                        }
                        if !in_hole {
                            output
                                .set(0, row, col, burn)
                                .map_err(|e| ToolError::Execution(format!("failed raster write: {e}")))?;
                        }
                    }
                }
            }

            coalescer.emit_unit_fraction(ctx.progress, (feat_idx + 1) as f64 / total);
        }

        write_raster_output(output, output_path, ctx)
    }
}
