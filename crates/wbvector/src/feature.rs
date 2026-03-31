//! The common feature model used by every format driver.
//!
//! The data model follows OGC Simple Features:
//!
//! * A [`Layer`] is a named collection of [`Feature`]s sharing a [`Schema`].
//! * A [`Feature`] has an optional [`Geometry`] and a vector of [`FieldValue`]s.
//! * A [`Schema`] is an ordered list of [`FieldDef`]s.
//!
//! All format drivers convert their native records to/from these types,
//! so cross-format conversion is simply: `read(src) → Layer → write(dst)`.

use std::collections::HashMap;
use crate::error::{GeoError, Result};
use crate::geometry::{BBox, Geometry, GeometryType};

// ══════════════════════════════════════════════════════════════════════════════
// FieldType
// ══════════════════════════════════════════════════════════════════════════════

/// Storage type of one attribute field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// 64-bit signed integer field.
    Integer,   // i64
    /// 64-bit floating-point numeric field.
    Float,     // f64
    /// UTF-8 text field.
    Text,      // UTF-8 string
    /// Boolean field.
    Boolean,   // bool
    /// Raw byte payload field.
    Blob,      // raw bytes
    /// Date field encoded as `YYYY-MM-DD` text.
    Date,      // YYYY-MM-DD string
    /// Datetime field encoded as ISO-8601 text.
    DateTime,  // ISO-8601 datetime string
    /// JSON text field.
    Json,      // JSON text
}

impl FieldType {
    /// Returns the stable display/storage name of this field type.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Integer  => "Integer",
            Self::Float    => "Float",
            Self::Text     => "Text",
            Self::Boolean  => "Boolean",
            Self::Blob     => "Blob",
            Self::Date     => "Date",
            Self::DateTime => "DateTime",
            Self::Json     => "Json",
        }
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// FieldDef
// ══════════════════════════════════════════════════════════════════════════════

/// Metadata for a single attribute field.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name (case-sensitive).
    pub name: String,
    /// Storage type.
    pub field_type: FieldType,
    /// Whether null values are permitted.
    pub nullable: bool,
    /// Max byte length for Text fields (0 = unlimited).
    pub width: usize,
    /// Decimal places for Float fields.
    pub precision: usize,
}

impl FieldDef {
    /// Creates a new field definition with default nullability/width/precision.
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self { name: name.into(), field_type, nullable: true, width: 0, precision: 0 }
    }
    /// Marks this field as non-nullable.
    pub fn not_null(mut self)          -> Self { self.nullable  = false; self }
    /// Sets the maximum text width (0 means unlimited).
    pub fn width(mut self, w: usize)   -> Self { self.width     = w;     self }
    /// Sets numeric precision metadata for floating-point fields.
    pub fn precision(mut self, p: usize) -> Self { self.precision = p;   self }
}

// ══════════════════════════════════════════════════════════════════════════════
// FieldValue
// ══════════════════════════════════════════════════════════════════════════════

/// A typed attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// 64-bit signed integer value.
    Integer(i64),
    /// 64-bit floating-point value.
    Float(f64),
    /// UTF-8 text value.
    Text(String),
    /// Boolean value.
    Boolean(bool),
    /// Raw bytes value.
    Blob(Vec<u8>),
    /// Date value encoded as `YYYY-MM-DD` text.
    Date(String),
    /// Datetime value encoded as ISO-8601 text.
    DateTime(String),
    /// SQL-style null value.
    Null,
}

impl FieldValue {
    /// Returns `true` if this value is [`FieldValue::Null`].
    pub fn is_null(&self) -> bool { matches!(self, Self::Null) }

    /// Converts to `i64` when possible.
    pub fn as_i64(&self) -> Option<i64> {
        match self { Self::Integer(v) => Some(*v), Self::Float(v) => Some(*v as i64), _ => None }
    }
    /// Converts to `f64` when possible.
    pub fn as_f64(&self) -> Option<f64> {
        match self { Self::Float(v) => Some(*v), Self::Integer(v) => Some(*v as f64), _ => None }
    }
    /// Returns string-like contents for text/date/datetime values.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) | Self::Date(s) | Self::DateTime(s) => Some(s.as_str()),
            _ => None,
        }
    }
    /// Converts to `bool` when the value is boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self { Self::Boolean(b) => Some(*b), _ => None }
    }
    /// Returns the underlying byte slice for blob values.
    pub fn as_blob(&self) -> Option<&[u8]> {
        match self { Self::Blob(b) => Some(b.as_slice()), _ => None }
    }

    /// Widening type promotion used during schema inference.
    ///
    /// | self      | other    | result  |
    /// |-----------|----------|---------|
    /// | Integer   | Float    | Float   |
    /// | any       | any (≠)  | Text    |
    pub fn widen_type(a: FieldType, b: FieldType) -> FieldType {
        if a == b { return a; }
        if matches!((a, b), (FieldType::Integer, FieldType::Float) | (FieldType::Float, FieldType::Integer)) {
            return FieldType::Float;
        }
        FieldType::Text
    }
}

// Convenient From impls
impl From<i64>    for FieldValue { fn from(v: i64)    -> Self { Self::Integer(v) } }
impl From<i32>    for FieldValue { fn from(v: i32)    -> Self { Self::Integer(v as i64) } }
impl From<f64>    for FieldValue { fn from(v: f64)    -> Self { Self::Float(v) } }
impl From<f32>    for FieldValue { fn from(v: f32)    -> Self { Self::Float(v as f64) } }
impl From<bool>   for FieldValue { fn from(v: bool)   -> Self { Self::Boolean(v) } }
impl From<String> for FieldValue { fn from(v: String) -> Self { Self::Text(v) } }
impl From<&str>   for FieldValue { fn from(v: &str)   -> Self { Self::Text(v.to_owned()) } }

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(v)  => write!(f, "{v}"),
            Self::Float(v)    => write!(f, "{v}"),
            Self::Text(v)     => write!(f, "{v}"),
            Self::Boolean(v)  => write!(f, "{v}"),
            Self::Blob(v)     => write!(f, "<blob {} bytes>", v.len()),
            Self::Date(v)     => write!(f, "{v}"),
            Self::DateTime(v) => write!(f, "{v}"),
            Self::Null        => write!(f, "NULL"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Schema
// ══════════════════════════════════════════════════════════════════════════════

/// Ordered list of field definitions with fast name→index lookup.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    fields: Vec<FieldDef>,
    index:  HashMap<String, usize>,
}

impl Schema {
    /// Creates an empty schema.
    pub fn new() -> Self { Self::default() }

    /// Add a field.  Duplicate names are silently ignored.
    pub fn add_field(&mut self, def: FieldDef) {
        if self.index.contains_key(&def.name) { return; }
        let i = self.fields.len();
        self.index.insert(def.name.clone(), i);
        self.fields.push(def);
    }

    /// Returns all field definitions in schema order.
    pub fn fields(&self)          -> &[FieldDef]       { &self.fields }
    /// Returns the number of fields in the schema.
    pub fn len(&self)             -> usize             { self.fields.len() }
    /// Returns `true` when the schema has no fields.
    pub fn is_empty(&self)        -> bool              { self.fields.is_empty() }
    /// Returns the positional index for a field name.
    pub fn field_index(&self, name: &str) -> Option<usize> { self.index.get(name).copied() }
    /// Returns the field definition for a field name.
    pub fn field(&self, name: &str) -> Option<&FieldDef>   { self.index.get(name).map(|&i| &self.fields[i]) }
}

// ══════════════════════════════════════════════════════════════════════════════
// Feature
// ══════════════════════════════════════════════════════════════════════════════

/// A single geographic feature: optional geometry + attribute values.
///
/// Attribute values are stored positionally to match the layer's [`Schema`].
#[derive(Debug, Clone)]
pub struct Feature {
    /// Feature identifier (0 if the format doesn't provide one).
    pub fid: u64,
    /// The geometry, if present.
    pub geometry: Option<Geometry>,
    /// Attribute values in schema order.
    pub attributes: Vec<FieldValue>,
}

impl Feature {
    /// Creates an empty feature with no geometry and no attributes.
    pub fn new() -> Self { Self { fid: 0, geometry: None, attributes: Vec::new() } }

    /// Creates a feature with geometry and pre-sized null attribute array.
    pub fn with_geometry(fid: u64, geom: Geometry, n_fields: usize) -> Self {
        Self { fid, geometry: Some(geom), attributes: vec![FieldValue::Null; n_fields] }
    }

    // ── Attribute access ─────────────────────────────────────────────────────

    /// Get a value by positional index.
    pub fn get_by_index(&self, idx: usize) -> Option<&FieldValue> {
        self.attributes.get(idx)
    }

    /// Set a value by positional index (extends the vector if needed).
    pub fn set_by_index(&mut self, idx: usize, val: FieldValue) {
        if idx >= self.attributes.len() {
            self.attributes.resize(idx + 1, FieldValue::Null);
        }
        self.attributes[idx] = val;
    }

    /// Get a value by field name (requires the layer schema).
    pub fn get(&self, schema: &Schema, name: &str) -> Result<&FieldValue> {
        let idx = schema.field_index(name)
            .ok_or_else(|| GeoError::FieldNotFound(name.to_owned()))?;
        Ok(self.attributes.get(idx).unwrap_or(&FieldValue::Null))
    }

    /// Set a value by field name (requires the layer schema).
    pub fn set(&mut self, schema: &Schema, name: &str, val: FieldValue) -> Result<()> {
        let idx = schema.field_index(name)
            .ok_or_else(|| GeoError::FieldNotFound(name.to_owned()))?;
        self.set_by_index(idx, val);
        Ok(())
    }

    /// Build a name→value map (useful for serialisation and debugging).
    pub fn attrs_map<'a>(&'a self, schema: &'a Schema) -> HashMap<&'a str, &'a FieldValue> {
        schema.fields().iter().enumerate()
            .filter_map(|(i, fd)| self.attributes.get(i).map(|v| (fd.name.as_str(), v)))
            .collect()
    }
}

impl Default for Feature { fn default() -> Self { Self::new() } }

// ══════════════════════════════════════════════════════════════════════════════
// CRS
// ══════════════════════════════════════════════════════════════════════════════

/// Coordinate reference system metadata attached to a [`Layer`].
#[derive(Debug, Clone, Default)]
pub struct Crs {
    /// EPSG code of the spatial reference system.
    pub epsg: Option<u32>,
    /// WKT definition of the spatial reference system.
    pub wkt: Option<String>,
}

impl Crs {
    /// Creates an empty CRS metadata object.
    pub fn new() -> Self { Self::default() }
    /// Sets EPSG code metadata.
    pub fn with_epsg(mut self, epsg: u32) -> Self { self.epsg = Some(epsg); self }
    /// Sets WKT CRS metadata.
    pub fn with_wkt(mut self, wkt: impl Into<String>) -> Self { self.wkt = Some(wkt.into()); self }
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer
// ══════════════════════════════════════════════════════════════════════════════

/// A named collection of features sharing a geometry type and attribute schema.
///
/// This is the central data structure that all format drivers produce and consume.
///
/// # Example
/// ```rust,ignore
/// use wbvector::feature::{Layer, FieldDef, FieldType};
/// use wbvector::geometry::{Geometry, GeometryType};
///
/// let mut layer = Layer::new("cities")
///     .with_geom_type(GeometryType::Point)
///     .with_epsg(4326);
///
/// layer.add_field(FieldDef::new("name",       FieldType::Text));
/// layer.add_field(FieldDef::new("population", FieldType::Integer));
///
/// layer.add_feature(Some(Geometry::point(-0.12, 51.5)), &[
///     ("name", "London".into()),
///     ("population", 9_000_000i64.into()),
/// ])?;
/// ```
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer / table name.
    pub name: String,
    /// Declared geometry type (all features should share this type).
    pub geom_type: Option<GeometryType>,
    /// Coordinate reference system metadata.
    pub crs: Option<Crs>,
    /// Attribute field schema.
    pub schema: Schema,
    /// Features in insertion order.
    pub features: Vec<Feature>,
    /// Cached bounding box (populated on first call to [`Layer::bbox`]).
    pub extent: Option<BBox>,
}

impl Layer {
    /// Creates an empty layer with the provided name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name:     name.into(),
            geom_type: None,
            crs:      None,
            schema:   Schema::new(),
            features: Vec::new(),
            extent:   None,
        }
    }

    // ── Builder methods ───────────────────────────────────────────────────────

    /// Sets the layer geometry type metadata.
    pub fn with_geom_type(mut self, gt: GeometryType) -> Self { self.geom_type = Some(gt); self }
    /// Alias for [`Layer::with_crs_epsg`].
    pub fn with_epsg(self, epsg: u32) -> Self { self.with_crs_epsg(epsg) }
    /// Sets complete CRS metadata.
    pub fn with_crs(mut self, crs: Crs) -> Self { self.crs = Some(crs); self }
    /// Sets EPSG metadata on the layer CRS.
    pub fn with_crs_epsg(mut self, epsg: u32) -> Self {
        self.ensure_crs_mut().epsg = Some(epsg);
        self
    }
    /// Sets WKT metadata on the layer CRS.
    pub fn with_crs_wkt(mut self, wkt: impl Into<String>) -> Self {
        self.ensure_crs_mut().wkt = Some(wkt.into());
        self
    }

    /// Returns the CRS EPSG code metadata, if available.
    pub fn crs_epsg(&self) -> Option<u32> {
        self.crs.as_ref().and_then(|c| c.epsg)
    }

    /// Returns the CRS WKT metadata, if available.
    pub fn crs_wkt(&self) -> Option<&str> {
        self.crs.as_ref().and_then(|c| c.wkt.as_deref())
    }

    /// Updates the CRS EPSG metadata (or clears it when `None`).
    pub fn set_crs_epsg(&mut self, epsg: Option<u32>) {
        if let Some(epsg_code) = epsg {
            self.ensure_crs_mut().epsg = Some(epsg_code);
        } else if let Some(crs) = self.crs.as_mut() {
            crs.epsg = None;
            if crs.wkt.is_none() {
                self.crs = None;
            }
        }
    }

    /// Updates the CRS WKT metadata (or clears it when `None`).
    pub fn set_crs_wkt(&mut self, wkt: Option<String>) {
        if let Some(wkt_text) = wkt {
            self.ensure_crs_mut().wkt = Some(wkt_text);
        } else if let Some(crs) = self.crs.as_mut() {
            crs.wkt = None;
            if crs.epsg.is_none() {
                self.crs = None;
            }
        }
    }

    /// Assign a CRS to this layer using an EPSG code.
    ///
    /// Replaces the entire `crs` struct with a new `Crs` containing only the EPSG code.
    /// Any existing `wkt` field is cleared to ensure CRS consistency.
    pub fn assign_crs_epsg(&mut self, epsg: u32) {
        self.crs = Some(Crs {
            epsg: Some(epsg),
            wkt: None,
        });
    }

    /// Assign a CRS to this layer using WKT text.
    ///
    /// Replaces the entire `crs` struct with a new `Crs` containing only the WKT definition.
    /// Any existing `epsg` field is cleared to ensure CRS consistency.
    pub fn assign_crs_wkt(&mut self, wkt: &str) {
        self.crs = Some(Crs {
            epsg: None,
            wkt: Some(wkt.to_string()),
        });
    }

    /// Reproject this layer to a destination EPSG code.
    ///
    /// Source CRS is read from `self.crs_epsg()`.
    pub fn reproject_to_epsg(&self, dst_epsg: u32) -> Result<Self> {
        crate::reproject::layer_to_epsg(self, dst_epsg)
    }

    /// Reproject this layer to a destination EPSG code with options.
    pub fn reproject_to_epsg_with_options(
        &self,
        dst_epsg: u32,
        options: &crate::reproject::VectorReprojectOptions,
    ) -> Result<Self> {
        crate::reproject::layer_to_epsg_with_options(self, dst_epsg, options)
    }

    /// Reproject this layer between explicit source/destination EPSG codes.
    pub fn reproject_from_to_epsg(&self, src_epsg: u32, dst_epsg: u32) -> Result<Self> {
        crate::reproject::layer_from_to_epsg(self, src_epsg, dst_epsg)
    }

    /// Reproject this layer between explicit source/destination EPSG codes with options.
    pub fn reproject_from_to_epsg_with_options(
        &self,
        src_epsg: u32,
        dst_epsg: u32,
        options: &crate::reproject::VectorReprojectOptions,
    ) -> Result<Self> {
        crate::reproject::layer_from_to_epsg_with_options(self, src_epsg, dst_epsg, options)
    }

    fn ensure_crs_mut(&mut self) -> &mut Crs {
        self.crs.get_or_insert_with(Crs::new)
    }

    // ── Schema ────────────────────────────────────────────────────────────────

    /// Adds a field definition to the layer schema.
    pub fn add_field(&mut self, def: FieldDef) { self.schema.add_field(def); }

    // ── Feature access ────────────────────────────────────────────────────────

    /// Appends a feature to the layer.
    pub fn push(&mut self, f: Feature) { self.features.push(f); }
    /// Returns number of features.
    pub fn len(&self)     -> usize    { self.features.len() }
    /// Returns `true` when there are no features.
    pub fn is_empty(&self) -> bool    { self.features.is_empty() }
    /// Iterates over features.
    pub fn iter(&self) -> impl Iterator<Item = &Feature> { self.features.iter() }
    /// Iterates mutably over features.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Feature> { self.features.iter_mut() }

    /// Convenience: build a feature from `(name, value)` pairs and push it.
    pub fn add_feature(
        &mut self,
        geom: Option<Geometry>,
        attrs: &[(&str, FieldValue)],
    ) -> Result<()> {
        let n   = self.schema.len();
        let fid = self.features.len() as u64;
        let mut f = Feature { fid, geometry: geom, attributes: vec![FieldValue::Null; n] };
        for (name, val) in attrs { f.set(&self.schema, name, val.clone())?; }
        self.features.push(f);
        Ok(())
    }

    // ── Bounding box ──────────────────────────────────────────────────────────

    /// Compute (or return cached) bounding box over all feature geometries.
    pub fn bbox(&mut self) -> Option<BBox> {
        if self.extent.is_some() { return self.extent.clone(); }
        let mut bb: Option<BBox> = None;
        for f in &self.features {
            if let Some(g) = &f.geometry {
                if let Some(fb) = g.bbox() {
                    bb = Some(match bb { None => fb, Some(mut e) => { e.expand_to(&fb); e } });
                }
            }
        }
        self.extent = bb.clone();
        bb
    }

    /// Filter features whose geometry bbox intersects `query`.
    pub fn features_in_bbox(&self, query: &BBox) -> Vec<&Feature> {
        self.features.iter()
            .filter(|f| f.geometry.as_ref()
                .and_then(|g| g.bbox())
                .map_or(false, |b| b.intersects(query)))
            .collect()
    }
}

impl std::ops::Index<usize> for Layer {
    type Output = Feature;
    fn index(&self, i: usize) -> &Feature { &self.features[i] }
}

impl std::ops::IndexMut<usize> for Layer {
    fn index_mut(&mut self, i: usize) -> &mut Feature { &mut self.features[i] }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_layer() -> Layer {
        let mut l = Layer::new("test").with_geom_type(GeometryType::Point).with_epsg(4326);
        l.add_field(FieldDef::new("name",  FieldType::Text));
        l.add_field(FieldDef::new("value", FieldType::Float));
        l.add_feature(
            Some(Geometry::point(10.0, 20.0)),
            &[("name", "alpha".into()), ("value", 3.14f64.into())],
        ).unwrap();
        l.add_feature(
            Some(Geometry::point(11.0, 21.0)),
            &[("name", "beta".into()), ("value", 2.72f64.into())],
        ).unwrap();
        l
    }

    use crate::geometry::Geometry;

    #[test] fn layer_len() { assert_eq!(make_layer().len(), 2); }

    #[test] fn get_field_by_name() {
        let l = make_layer();
        let v = l[0].get(&l.schema, "name").unwrap();
        assert_eq!(v, &FieldValue::Text("alpha".into()));
    }

    #[test] fn get_field_by_index() {
        let l = make_layer();
        assert!(l[0].get_by_index(1).unwrap().as_f64().unwrap() - 3.14 < 1e-9);
    }

    #[test] fn bbox() {
        let mut l = make_layer();
        let b = l.bbox().unwrap();
        assert_eq!(b.min_x, 10.0); assert_eq!(b.max_x, 11.0);
    }

    #[test] fn widen_type_int_float() {
        assert_eq!(FieldValue::widen_type(FieldType::Integer, FieldType::Float), FieldType::Float);
    }

    #[test] fn widen_type_mismatch() {
        assert_eq!(FieldValue::widen_type(FieldType::Integer, FieldType::Text), FieldType::Text);
    }
}
