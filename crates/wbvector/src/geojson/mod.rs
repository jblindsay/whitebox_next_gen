//! GeoJSON (RFC 7946) reader and writer.
//!
//! The parser is hand-rolled with no external JSON library.  It covers the
//! GeoJSON subset of JSON precisely — numbers, strings (including `\uXXXX`),
//! arrays, and objects.
//!
//! ## Schema inference
//! Property keys are collected across all features.  Column types are inferred
//! by scanning every value:
//! - All integers → `Integer`
//! - Mix of integer + float → `Float`
//! - Any object or array → `Json`
//! - YYYY-MM-DD strings → `Date`
//! - Otherwise → `Text`

use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer, Schema};
use crate::geometry::{Coord, Geometry, Ring};
use crate::reproject;

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a GeoJSON file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let text = std::fs::read_to_string(path).map_err(GeoError::Io)?;
    parse_str(&text)
}

/// Parse a GeoJSON string into a [`Layer`].
pub fn parse_str(text: &str) -> Result<Layer> {
    let val = Parser::new(text).parse_value()?;
    layer_from_value(val, "layer")
}

/// Write a [`Layer`] as a GeoJSON `FeatureCollection` to a file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let out_layer = prepare_rfc7946_layer(layer)?;
    std::fs::write(path, to_string(&out_layer).as_bytes()).map_err(GeoError::Io)
}

/// Serialise a [`Layer`] as a compact GeoJSON string.
pub fn to_string(layer: &Layer) -> String {
    let mut s = String::new();
    write_feature_collection(&mut s, layer);
    s
}

fn prepare_rfc7946_layer(layer: &Layer) -> Result<Layer> {
    // RFC 7946 requires WGS 84 lon/lat coordinates. If CRS metadata is
    // present and not already EPSG:4326, reproject on write.
    if layer.crs_epsg() == Some(4326) {
        return Ok(layer.clone());
    }

    if layer.crs_epsg().is_some() || layer.crs_wkt().is_some() {
        return reproject::layer_to_epsg(layer, 4326);
    }

    Ok(layer.clone())
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal JSON value
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
enum Jv {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Jv>),
    Obj(Vec<(String, Jv)>),  // preserve insertion order
}

impl Jv {
    fn get(&self, key: &str) -> Option<&Jv> {
        if let Jv::Obj(pairs) = self { pairs.iter().find(|(k,_)| k == key).map(|(_,v)| v) }
        else { None }
    }
    fn as_str(&self) -> Option<&str>  { if let Jv::Str(s) = self { Some(s) } else { None } }
    fn as_f64(&self) -> Option<f64>   { if let Jv::Num(n) = self { Some(*n) } else { None } }
    fn as_arr(&self) -> Option<&[Jv]> { if let Jv::Arr(a) = self { Some(a) } else { None } }
}

// ══════════════════════════════════════════════════════════════════════════════
// Minimal JSON parser
// ══════════════════════════════════════════════════════════════════════════════

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self { Self { src: s.as_bytes(), pos: 0 } }

    fn err(&self, msg: &str) -> GeoError {
        GeoError::GeoJsonParse { offset: self.pos, msg: msg.to_owned() }
    }

    fn peek(&self) -> Option<u8> { self.src.get(self.pos).copied() }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' '|b'\t'|b'\n'|b'\r')) { self.pos += 1; }
    }

    fn eat(&mut self, b: u8) -> Result<()> {
        self.skip_ws();
        if self.peek() == Some(b) { self.pos += 1; Ok(()) }
        else { Err(self.err(&format!("expected '{}' got {:?}", b as char, self.peek().map(|b| b as char)))) }
    }

    pub fn parse_value(&mut self) -> Result<Jv> {
        self.skip_ws();
        match self.peek() {
            Some(b'"')                   => self.parse_string().map(Jv::Str),
            Some(b'{')                   => self.parse_object(),
            Some(b'[')                   => self.parse_array(),
            Some(b't')                   => { self.pos += 4; Ok(Jv::Bool(true))  }
            Some(b'f')                   => { self.pos += 5; Ok(Jv::Bool(false)) }
            Some(b'n')                   => { self.pos += 4; Ok(Jv::Null)        }
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b)                      => Err(self.err(&format!("unexpected byte 0x{b:02X}"))),
            None                         => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.eat(b'"')?;
        let mut s = String::new();
        loop {
            match self.peek() {
                None        => return Err(self.err("unterminated string")),
                Some(b'"')  => { self.pos += 1; break; }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"')  => { s.push('"');   self.pos += 1; }
                        Some(b'\\') => { s.push('\\');  self.pos += 1; }
                        Some(b'/')  => { s.push('/');   self.pos += 1; }
                        Some(b'n')  => { s.push('\n');  self.pos += 1; }
                        Some(b'r')  => { s.push('\r');  self.pos += 1; }
                        Some(b't')  => { s.push('\t');  self.pos += 1; }
                        Some(b'b')  => { s.push('\x08'); self.pos += 1; }
                        Some(b'f')  => { s.push('\x0C'); self.pos += 1; }
                        Some(b'u')  => {
                            self.pos += 1;
                            if self.pos + 4 > self.src.len() {
                                return Err(self.err("truncated \\u escape"));
                            }
                            let hex = std::str::from_utf8(&self.src[self.pos..self.pos+4])
                                .map_err(|_| self.err("invalid \\u escape"))?;
                            let cp = u32::from_str_radix(hex, 16)
                                .map_err(|_| self.err("invalid \\u codepoint"))?;
                            if let Some(ch) = char::from_u32(cp) { s.push(ch); }
                            self.pos += 4;
                        }
                        _ => s.push('\\'),
                    }
                }
                Some(b) => { s.push(b as char); self.pos += 1; }
            }
        }
        Ok(s)
    }

    fn parse_number(&mut self) -> Result<Jv> {
        let start = self.pos;
        if self.peek() == Some(b'-') { self.pos += 1; }
        while matches!(self.peek(), Some(b'0'..=b'9')) { self.pos += 1; }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) { self.pos += 1; }
        }
        if matches!(self.peek(), Some(b'e'|b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+'|b'-')) { self.pos += 1; }
            while matches!(self.peek(), Some(b'0'..=b'9')) { self.pos += 1; }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err("invalid number bytes"))?;
        let n: f64 = s.parse().map_err(|_| self.err("invalid number"))?;
        Ok(Jv::Num(n))
    }

    fn parse_array(&mut self) -> Result<Jv> {
        self.eat(b'[')?;
        let mut arr = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') { self.pos += 1; return Ok(Jv::Arr(arr)); }
        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => { self.pos += 1; }
                Some(b']') => { self.pos += 1; break; }
                _          => return Err(self.err("expected ',' or ']'")),
            }
        }
        Ok(Jv::Arr(arr))
    }

    fn parse_object(&mut self) -> Result<Jv> {
        self.eat(b'{')?;
        let mut pairs: Vec<(String, Jv)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') { self.pos += 1; return Ok(Jv::Obj(pairs)); }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.eat(b':')?;
            let val = self.parse_value()?;
            pairs.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => { self.pos += 1; }
                Some(b'}') => { self.pos += 1; break; }
                _          => return Err(self.err("expected ',' or '}'")),
            }
        }
        Ok(Jv::Obj(pairs))
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// JSON value → Layer
// ══════════════════════════════════════════════════════════════════════════════

fn layer_from_value(val: Jv, name: &str) -> Result<Layer> {
    let type_s = val.get("type").and_then(|v| v.as_str()).unwrap_or("").to_owned();
    match type_s.as_str() {
        "FeatureCollection" => parse_feature_collection(val, name),
        "Feature" => {
            let mut layer = Layer::new(name);
            if let Some(f) = build_feature(&val, &layer.schema, 0)? {
                if let Some(geom) = &f.geometry {
                    layer.geom_type = Some(geom.geom_type());
                }
                layer.push(f);
            }
            Ok(layer)
        }
        _ => {
            // Bare geometry
            let geom = parse_geometry(&val)?;
            let mut layer = Layer::new(name);
            layer.geom_type = Some(geom.geom_type());
            layer.push(Feature { fid: 0, geometry: Some(geom), attributes: vec![] });
            Ok(layer)
        }
    }
}

fn parse_feature_collection(val: Jv, name: &str) -> Result<Layer> {
    let features_arr = val.get("features").and_then(|v| v.as_arr())
        .ok_or_else(|| GeoError::GeoJsonMissing("features".into()))?;

    // ── two-pass schema inference ─────────────────────────────────────────────
    let mut key_order: Vec<String>       = Vec::new();
    let mut key_seen:  HashSet<String>   = HashSet::new();
    let mut key_type:  HashMap<String, FieldType> = HashMap::new();

    for feat in features_arr {
        if let Some(Jv::Obj(props)) = feat.get("properties") {
            for (k, v) in props {
                if key_seen.insert(k.clone()) { key_order.push(k.clone()); }
                if matches!(v, Jv::Null) { continue; }
                let inferred = infer_type(v);
                let entry = key_type.entry(k.clone()).or_insert(inferred);
                *entry = FieldValue::widen_type(*entry, inferred);
            }
        }
    }

    let mut layer = Layer::new(name);
    for k in &key_order {
        let ft = key_type.get(k).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(k, ft));
    }

    for (idx, feat_val) in features_arr.iter().enumerate() {
        if let Some(f) = build_feature(feat_val, &layer.schema, idx as u64)? {
            // Infer layer geometry type from first feature with geometry
            if layer.geom_type.is_none() {
                if let Some(geom) = &f.geometry {
                    layer.geom_type = Some(geom.geom_type());
                }
            }
            layer.push(f);
        }
    }

    Ok(layer)
}

fn infer_type(v: &Jv) -> FieldType {
    match v {
        Jv::Bool(_)   => FieldType::Boolean,
        Jv::Num(n)    => if n.fract() == 0.0 { FieldType::Integer } else { FieldType::Float },
        Jv::Null      => FieldType::Text,           // conservative
        Jv::Arr(_) | Jv::Obj(_) => FieldType::Json,
        Jv::Str(s)    => if looks_like_date(s) { FieldType::Date } else { FieldType::Text },
    }
}

fn looks_like_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-'
}

fn build_feature(val: &Jv, schema: &Schema, fid: u64) -> Result<Option<Feature>> {
    let type_s = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if type_s != "Feature" {
        return Err(GeoError::GeoJsonType(type_s.to_owned()));
    }

    let geom = match val.get("geometry") {
        Some(Jv::Null) | None => None,
        Some(g) => Some(parse_geometry(g)?),
    };

    let mut attrs = vec![FieldValue::Null; schema.len()];
    if let Some(Jv::Obj(props)) = val.get("properties") {
        for (k, v) in props {
            if let Some(idx) = schema.field_index(k) {
                let ft = schema.fields()[idx].field_type;
                attrs[idx] = jv_to_field(v, ft);
            }
        }
    }

    Ok(Some(Feature { fid, geometry: geom, attributes: attrs }))
}

fn jv_to_field(v: &Jv, ft: FieldType) -> FieldValue {
    match (v, ft) {
        (Jv::Null, _)                       => FieldValue::Null,
        (Jv::Bool(b), _)                    => FieldValue::Boolean(*b),
        (Jv::Num(n), FieldType::Integer)    => FieldValue::Integer(*n as i64),
        (Jv::Num(n), _)                     => FieldValue::Float(*n),
        (Jv::Str(s), FieldType::Date)       => FieldValue::Date(s.clone()),
        (Jv::Str(s), FieldType::DateTime)   => FieldValue::DateTime(s.clone()),
        (Jv::Str(s), _)                     => FieldValue::Text(s.clone()),
        (Jv::Arr(_), _) | (Jv::Obj(_), _)  => FieldValue::Text(jv_to_json_str(v)),
    }
}

fn jv_to_json_str(v: &Jv) -> String {
    match v {
        Jv::Null      => "null".into(),
        Jv::Bool(b)   => b.to_string(),
        Jv::Num(n)    => fmt_number(*n),
        Jv::Str(s)    => format!("\"{}\"", s.replace('"', "\\\"")),
        Jv::Arr(a)    => format!("[{}]", a.iter().map(jv_to_json_str).collect::<Vec<_>>().join(",")),
        Jv::Obj(o)    => {
            let pairs: Vec<String> = o.iter().map(|(k,v)| format!("\"{}\":{}", k, jv_to_json_str(v))).collect();
            format!("{{{}}}", pairs.join(","))
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// GeoJSON geometry parsing
// ══════════════════════════════════════════════════════════════════════════════

fn parse_geometry(val: &Jv) -> Result<Geometry> {
    let type_s = val.get("type").and_then(|v| v.as_str())
        .ok_or_else(|| GeoError::GeoJsonMissing("geometry.type".into()))?;
    let coords = val.get("coordinates");

    match type_s {
        "Point" => {
            let c = parse_one_coord(coords.ok_or_else(|| GeoError::GeoJsonMissing("coordinates".into()))?)?;
            Ok(Geometry::Point(c))
        }
        "LineString" => {
            let cs = parse_coord_ring(coords.ok_or_else(|| GeoError::GeoJsonMissing("coordinates".into()))?)?;
            Ok(Geometry::LineString(cs))
        }
        "Polygon" => {
            let rings = coords.and_then(|v| v.as_arr())
                .ok_or_else(|| GeoError::GeoJsonMissing("polygon coordinates".into()))?;
            let mut parsed: Vec<Vec<Coord>> = rings.iter()
                .map(|r| parse_coord_ring(r).map(strip_closed_ring))
                .collect::<Result<_>>()?;
            let exterior = parsed.drain(..1).next().unwrap_or_default();
            Ok(Geometry::polygon(exterior, parsed))
        }
        "MultiPoint" => {
            let cs = parse_coord_ring(coords.ok_or_else(|| GeoError::GeoJsonMissing("coordinates".into()))?)?;
            Ok(Geometry::MultiPoint(cs))
        }
        "MultiLineString" => {
            let lines = coords.and_then(|v| v.as_arr())
                .ok_or_else(|| GeoError::GeoJsonMissing("MultiLineString coordinates".into()))?;
            let ls: Vec<Vec<Coord>> = lines.iter().map(|l| parse_coord_ring(l)).collect::<Result<_>>()?;
            Ok(Geometry::MultiLineString(ls))
        }
        "MultiPolygon" => {
            let polys = coords.and_then(|v| v.as_arr())
                .ok_or_else(|| GeoError::GeoJsonMissing("MultiPolygon coordinates".into()))?;
            let ps: Vec<(Vec<Coord>, Vec<Vec<Coord>>)> = polys.iter().map(|poly| {
                let rings = poly.as_arr().ok_or_else(|| GeoError::GeoJsonMissing("polygon rings".into()))?;
                let mut parsed: Vec<Vec<Coord>> = rings.iter()
                    .map(|r| parse_coord_ring(r).map(strip_closed_ring))
                    .collect::<Result<_>>()?;
                let ext = parsed.drain(..1).next().unwrap_or_default();
                Ok((ext, parsed))
            }).collect::<Result<_>>()?;
            Ok(Geometry::multi_polygon(ps))
        }
        "GeometryCollection" => {
            let geoms = val.get("geometries").and_then(|v| v.as_arr())
                .ok_or_else(|| GeoError::GeoJsonMissing("geometries".into()))?;
            let gs: Vec<Geometry> = geoms.iter().map(|g| parse_geometry(g)).collect::<Result<_>>()?;
            Ok(Geometry::GeometryCollection(gs))
        }
        other => Err(GeoError::GeoJsonType(other.to_owned())),
    }
}

fn parse_one_coord(v: &Jv) -> Result<Coord> {
    let a = v.as_arr().ok_or_else(|| GeoError::GeoJsonParse { offset: 0, msg: "coordinate must be array".into() })?;
    let x = a.get(0).and_then(|v| v.as_f64()).ok_or_else(|| GeoError::GeoJsonParse { offset: 0, msg: "missing x".into() })?;
    let y = a.get(1).and_then(|v| v.as_f64()).ok_or_else(|| GeoError::GeoJsonParse { offset: 0, msg: "missing y".into() })?;
    let z = a.get(2).and_then(|v| v.as_f64());
    Ok(Coord { x, y, z, m: None })
}

fn parse_coord_ring(v: &Jv) -> Result<Vec<Coord>> {
    let arr = v.as_arr().ok_or_else(|| GeoError::GeoJsonParse { offset: 0, msg: "expected coord array".into() })?;
    arr.iter().map(|c| parse_one_coord(c)).collect()
}

fn strip_closed_ring(mut coords: Vec<Coord>) -> Vec<Coord> {
    if coords.len() > 1 {
        let first = coords.first().cloned();
        let last = coords.last().cloned();
        if first == last {
            coords.pop();
        }
    }
    coords
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer → GeoJSON string
// ══════════════════════════════════════════════════════════════════════════════

fn write_feature_collection(s: &mut String, layer: &Layer) {
    s.push_str(r#"{"type":"FeatureCollection","features":["#);
    for (i, f) in layer.features.iter().enumerate() {
        if i > 0 { s.push(','); }
        write_feature(s, f, &layer.schema);
    }
    s.push_str("]}");
}

fn write_feature(s: &mut String, f: &Feature, schema: &Schema) {
    s.push_str(r#"{"type":"Feature","geometry":"#);
    match &f.geometry { None => s.push_str("null"), Some(g) => write_geom(s, g) }
    s.push_str(r#","properties":"#);
    write_props(s, f, schema);
    s.push('}');
}

fn write_geom(s: &mut String, g: &Geometry) {
    match g {
        Geometry::Point(c) => {
            s.push_str(r#"{"type":"Point","coordinates":"#);
            write_coord(s, c); s.push('}');
        }
        Geometry::LineString(cs) => {
            s.push_str(r#"{"type":"LineString","coordinates":"#);
            write_coord_arr(s, cs); s.push('}');
        }
        Geometry::Polygon { exterior, interiors } => {
            s.push_str(r#"{"type":"Polygon","coordinates":["#);
            write_ring_arr(s, exterior);
            for r in interiors { s.push(','); write_ring_arr(s, r); }
            s.push_str("]}");
        }
        Geometry::MultiPoint(cs) => {
            s.push_str(r#"{"type":"MultiPoint","coordinates":"#);
            write_coord_arr(s, cs); s.push('}');
        }
        Geometry::MultiLineString(ls) => {
            s.push_str(r#"{"type":"MultiLineString","coordinates":["#);
            for (i, l) in ls.iter().enumerate() { if i>0 {s.push(',');} write_coord_arr(s, l); }
            s.push_str("]}");
        }
        Geometry::MultiPolygon(ps) => {
            s.push_str(r#"{"type":"MultiPolygon","coordinates":["#);
            for (i, (e, hs)) in ps.iter().enumerate() {
                if i>0 {s.push(',');} s.push('[');
                write_ring_arr(s, e);
                for h in hs { s.push(','); write_ring_arr(s, h); }
                s.push(']');
            }
            s.push_str("]}");
        }
        Geometry::GeometryCollection(gs) => {
            s.push_str(r#"{"type":"GeometryCollection","geometries":["#);
            for (i, g) in gs.iter().enumerate() { if i>0 {s.push(',');} write_geom(s, g); }
            s.push_str("]}");
        }
    }
}

fn write_coord(s: &mut String, c: &Coord) {
    s.push('[');
    s.push_str(&fmt_number(c.x)); s.push(','); s.push_str(&fmt_number(c.y));
    if let Some(z) = c.z { s.push(','); s.push_str(&fmt_number(z)); }
    s.push(']');
}

fn write_coord_arr(s: &mut String, cs: &[Coord]) {
    s.push('[');
    for (i, c) in cs.iter().enumerate() { if i>0 {s.push(',');} write_coord(s, c); }
    s.push(']');
}

fn write_ring_arr(s: &mut String, ring: &Ring) {
    s.push('[');
    for (i, c) in ring.0.iter().enumerate() { if i>0 {s.push(',');} write_coord(s, c); }
    // close ring
    if !ring.0.is_empty() { s.push(','); write_coord(s, &ring.0[0]); }
    s.push(']');
}

fn write_props(s: &mut String, f: &Feature, schema: &Schema) {
    if schema.is_empty() { s.push_str("null"); return; }
    s.push('{');
    let mut first = true;
    for (i, fd) in schema.fields().iter().enumerate() {
        if !first { s.push(','); }
        first = false;
        write_json_str(s, &fd.name);
        s.push(':');
        let val = f.attributes.get(i).unwrap_or(&FieldValue::Null);
        write_field_value(s, val);
    }
    s.push('}');
}

fn write_field_value(s: &mut String, val: &FieldValue) {
    match val {
        FieldValue::Null         => s.push_str("null"),
        FieldValue::Integer(v)   => s.push_str(&v.to_string()),
        FieldValue::Float(v)     => s.push_str(&fmt_number(*v)),
        FieldValue::Boolean(v)   => s.push_str(if *v { "true" } else { "false" }),
        FieldValue::Text(v) | FieldValue::Date(v) | FieldValue::DateTime(v) => write_json_str(s, v),
        FieldValue::Blob(b)      => {
            // Encode as hex string
            s.push('"');
            for byte in b { s.push_str(&format!("{byte:02X}")); }
            s.push('"');
        }
    }
}

fn write_json_str(s: &mut String, v: &str) {
    s.push('"');
    for ch in v.chars() {
        match ch {
            '"'  => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c    => s.push(c),
        }
    }
    s.push('"');
}

fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) }
    else { format!("{n}") }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "type": "FeatureCollection",
        "features": [
            {"type":"Feature",
             "geometry":{"type":"Point","coordinates":[10.5,20.0]},
             "properties":{"name":"alpha","count":7,"score":3.14}},
            {"type":"Feature",
             "geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]},
             "properties":{"name":"beta","count":2,"score":null}}
        ]
    }"#;

    #[test]
    fn parse_fc() {
        let l = parse_str(SAMPLE).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l.schema.len(), 3);
    }

    #[test]
    fn parse_point() {
        let l = parse_str(SAMPLE).unwrap();
        if let Some(Geometry::Point(c)) = &l[0].geometry {
            assert!((c.x - 10.5).abs() < 1e-9);
            assert!((c.y - 20.0).abs() < 1e-9);
        } else { panic!("expected Point"); }
    }

    #[test]
    fn parse_polygon() {
        let l = parse_str(SAMPLE).unwrap();
        if let Some(Geometry::Polygon { exterior, interiors }) = &l[1].geometry {
            assert_eq!(exterior.len(), 4); // closing point stripped
            assert!(interiors.is_empty());
        } else { panic!("expected Polygon"); }
    }

    #[test]
    fn field_types() {
        let l = parse_str(SAMPLE).unwrap();
        let f = l.schema.field("name").unwrap();
        assert_eq!(f.field_type, FieldType::Text);
        let f = l.schema.field("count").unwrap();
        assert_eq!(f.field_type, FieldType::Integer);
        let f = l.schema.field("score").unwrap();
        // null in one row widens Integer→Float here because 3.14 has fract
        assert_eq!(f.field_type, FieldType::Float);
    }

    #[test]
    fn roundtrip() {
        let l1 = parse_str(SAMPLE).unwrap();
        let json = to_string(&l1);
        let l2 = parse_str(&json).unwrap();
        assert_eq!(l1.len(), l2.len());
        assert_eq!(l1.schema.len(), l2.schema.len());
    }

    #[test]
    fn null_geometry() {
        let text = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":null,"properties":{"id":1}}]}"#;
        let l = parse_str(text).unwrap();
        assert!(l[0].geometry.is_none());
    }

    #[test]
    fn geometry_collection() {
        let text = r#"{"type":"GeometryCollection","geometries":[
            {"type":"Point","coordinates":[0,0]},
            {"type":"LineString","coordinates":[[0,0],[1,1]]}]}"#;
        let l = parse_str(text).unwrap();
        assert!(matches!(l[0].geometry, Some(Geometry::GeometryCollection(_))));
    }

    #[test]
    fn file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.geojson");
        let l1 = parse_str(SAMPLE).unwrap();
        write(&l1, &path).unwrap();
        let l2 = read(&path).unwrap();
        assert_eq!(l2.len(), 2);
    }

    #[test]
    fn write_reprojects_projected_layer_to_epsg4326() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reprojected.geojson");

        let mut layer = Layer::new("mercator").with_crs_epsg(3857);
        layer.push(Feature {
            fid: 0,
            geometry: Some(Geometry::point(111319.49079327357, 0.0)),
            attributes: vec![],
        });

        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();

        if let Some(Geometry::Point(c)) = &out[0].geometry {
            assert!((c.x - 1.0).abs() < 1.0e-5);
            assert!(c.y.abs() < 1.0e-9);
        } else {
            panic!("expected Point geometry");
        }
    }
}
