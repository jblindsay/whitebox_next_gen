//! TopoJSON reader and writer.
//!
//! This module provides dependency-light TopoJSON I/O using wbvector's common
//! in-memory [`Layer`](crate::feature::Layer) model.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::{GeoError, Result};
use crate::feature::{Feature, FieldDef, FieldType, FieldValue, Layer};
use crate::geometry::{Coord, Geometry, Ring};

/// Writer options for TopoJSON serialization.
#[derive(Debug, Clone, Copy)]
pub struct TopoJsonWriteOptions {
    /// Optional quantization grid size. Values >= 2 enable transform + delta arcs.
    pub quantize: Option<u32>,
    /// Include a root `bbox` entry in output when true.
    pub include_bbox: bool,
}

impl Default for TopoJsonWriteOptions {
    fn default() -> Self {
        Self {
            quantize: None,
            include_bbox: false,
        }
    }
}

impl TopoJsonWriteOptions {
    /// Enable quantization with the specified grid size.
    pub fn with_quantize(mut self, grid_size: u32) -> Self {
        self.quantize = Some(grid_size);
        self
    }

    /// Toggle root bbox emission.
    pub fn with_bbox(mut self, include_bbox: bool) -> Self {
        self.include_bbox = include_bbox;
        self
    }
}

/// Read a TopoJSON file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let text = std::fs::read_to_string(path).map_err(GeoError::Io)?;
    parse_str(&text)
}

/// Parse TopoJSON text into a [`Layer`].
pub fn parse_str(text: &str) -> Result<Layer> {
    let mut p = Parser::new(text);
    let root = p.parse_value()?;
    layer_from_topology(&root, "layer")
}

/// Write a [`Layer`] to TopoJSON file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let text = to_string_with_options(layer, TopoJsonWriteOptions::default())?;
    std::fs::write(path, text).map_err(GeoError::Io)
}

/// Write a [`Layer`] to TopoJSON file with explicit options.
pub fn write_with_options<P: AsRef<Path>>(
    layer: &Layer,
    path: P,
    options: TopoJsonWriteOptions,
) -> Result<()> {
    let text = to_string_with_options(layer, options)?;
    std::fs::write(path, text).map_err(GeoError::Io)
}

/// Serialize a [`Layer`] to TopoJSON text.
pub fn to_string(layer: &Layer) -> Result<String> {
    to_string_with_options(layer, TopoJsonWriteOptions::default())
}

/// Serialize a [`Layer`] to TopoJSON text with explicit options.
pub fn to_string_with_options(layer: &Layer, options: TopoJsonWriteOptions) -> Result<String> {
    let mut writer = TopologyWriter::new();
    let root = writer.layer_to_topology(layer, options);
    Ok(jv_to_json(&root))
}

#[derive(Debug, Clone)]
enum Jv {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Jv>),
    Obj(Vec<(String, Jv)>),
}

impl Jv {
    fn get(&self, key: &str) -> Option<&Jv> {
        match self {
            Jv::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        if let Jv::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    fn as_f64(&self) -> Option<f64> {
        if let Jv::Num(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|v| v as i64)
    }

    fn as_arr(&self) -> Option<&[Jv]> {
        if let Jv::Arr(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            src: s.as_bytes(),
            pos: 0,
        }
    }

    fn err(&self, msg: &str) -> GeoError {
        GeoError::TopoJsonParse {
            offset: self.pos,
            msg: msg.to_owned(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, b: u8) -> Result<()> {
        self.skip_ws();
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!(
                "expected '{}' got {:?}",
                b as char,
                self.peek().map(|c| c as char)
            )))
        }
    }

    fn parse_value(&mut self) -> Result<Jv> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Jv::Str),
            Some(b't') => {
                self.pos += 4;
                Ok(Jv::Bool(true))
            }
            Some(b'f') => {
                self.pos += 5;
                Ok(Jv::Bool(false))
            }
            Some(b'n') => {
                self.pos += 4;
                Ok(Jv::Null)
            }
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(self.err(&format!("unexpected byte 0x{b:02X}"))),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.eat(b'"')?;
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => {
                            s.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            s.push('\\');
                            self.pos += 1;
                        }
                        Some(b'/') => {
                            s.push('/');
                            self.pos += 1;
                        }
                        Some(b'n') => {
                            s.push('\n');
                            self.pos += 1;
                        }
                        Some(b'r') => {
                            s.push('\r');
                            self.pos += 1;
                        }
                        Some(b't') => {
                            s.push('\t');
                            self.pos += 1;
                        }
                        Some(b'b') => {
                            s.push('\x08');
                            self.pos += 1;
                        }
                        Some(b'f') => {
                            s.push('\x0C');
                            self.pos += 1;
                        }
                        Some(b'u') => {
                            self.pos += 1;
                            if self.pos + 4 > self.src.len() {
                                return Err(self.err("truncated unicode escape"));
                            }
                            let hex = std::str::from_utf8(&self.src[self.pos..self.pos + 4])
                                .map_err(|_| self.err("invalid unicode escape"))?;
                            let cp = u32::from_str_radix(hex, 16)
                                .map_err(|_| self.err("invalid unicode codepoint"))?;
                            if let Some(ch) = char::from_u32(cp) {
                                s.push(ch);
                            }
                            self.pos += 4;
                        }
                        _ => s.push('\\'),
                    }
                }
                Some(b) => {
                    s.push(b as char);
                    self.pos += 1;
                }
            }
        }
        Ok(s)
    }

    fn parse_number(&mut self) -> Result<Jv> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err("invalid number bytes"))?;
        let n: f64 = s.parse().map_err(|_| self.err("invalid number"))?;
        Ok(Jv::Num(n))
    }

    fn parse_array(&mut self) -> Result<Jv> {
        self.eat(b'[')?;
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Jv::Arr(out));
        }
        loop {
            out.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
        Ok(Jv::Arr(out))
    }

    fn parse_object(&mut self) -> Result<Jv> {
        self.eat(b'{')?;
        let mut out = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Jv::Obj(out));
        }
        loop {
            self.skip_ws();
            let k = self.parse_string()?;
            self.eat(b':')?;
            let v = self.parse_value()?;
            out.push((k, v));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
        Ok(Jv::Obj(out))
    }
}

#[derive(Debug, Clone)]
struct TopoEntry {
    geometry: Option<Geometry>,
    properties: Option<Vec<(String, Jv)>>,
    id: Option<Jv>,
}

fn layer_from_topology(root: &Jv, layer_name: &str) -> Result<Layer> {
    let root_type = root
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GeoError::TopoJsonMissing("type".into()))?;
    if root_type != "Topology" {
        return Err(GeoError::TopoJsonType(root_type.to_owned()));
    }

    let arcs_jv = root
        .get("arcs")
        .and_then(|v| v.as_arr())
        .ok_or_else(|| GeoError::TopoJsonMissing("arcs".into()))?;

    let transform = parse_transform(root.get("transform"))?;

    let mut arcs = Vec::with_capacity(arcs_jv.len());
    for arc in arcs_jv {
        arcs.push(decode_arc(arc, transform.as_ref())?);
    }

    let objects = root
        .get("objects")
        .and_then(|v| v.as_arr().or_else(|| if matches!(v, Jv::Obj(_)) { Some(&[][..]) } else { None }));

    let objects_obj = root
        .get("objects")
        .ok_or_else(|| GeoError::TopoJsonMissing("objects".into()))?;

    let mut entries = Vec::<TopoEntry>::new();
    match objects_obj {
        Jv::Obj(pairs) => {
            for (_, obj) in pairs {
                extract_entries(obj, &arcs, &mut entries)?;
            }
        }
        _ => return Err(GeoError::TopoJsonTopology("objects must be an object".into())),
    }
    let _ = objects;

    let mut key_order = Vec::<String>::new();
    let mut seen = HashSet::<String>::new();
    let mut key_type = HashMap::<String, FieldType>::new();
    let mut has_id = false;

    for e in &entries {
        if e.id.is_some() {
            has_id = true;
        }
        if let Some(props) = &e.properties {
            for (k, v) in props {
                if seen.insert(k.clone()) {
                    key_order.push(k.clone());
                }
                if matches!(v, Jv::Null) {
                    continue;
                }
                let inferred = infer_type(v);
                let cur = key_type.entry(k.clone()).or_insert(inferred);
                *cur = FieldValue::widen_type(*cur, inferred);
            }
        }
    }

    let mut layer = Layer::new(layer_name);
    if has_id {
        layer.add_field(FieldDef::new("topo_id", FieldType::Text));
    }
    for k in &key_order {
        let ft = key_type.get(k).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(k, ft));
    }

    let id_index = layer.schema.field_index("topo_id");
    for (i, entry) in entries.into_iter().enumerate() {
        let mut attrs = vec![FieldValue::Null; layer.schema.len()];
        if let (Some(idx), Some(idv)) = (id_index, entry.id.as_ref()) {
            attrs[idx] = jv_to_field(idv, FieldType::Text);
        }
        if let Some(props) = entry.properties {
            for (k, v) in props {
                if let Some(idx) = layer.schema.field_index(&k) {
                    let ft = layer.schema.fields()[idx].field_type;
                    attrs[idx] = jv_to_field(&v, ft);
                }
            }
        }

        if layer.geom_type.is_none() {
            if let Some(g) = &entry.geometry {
                layer.geom_type = Some(g.geom_type());
            }
        }

        layer.push(Feature {
            fid: i as u64,
            geometry: entry.geometry,
            attributes: attrs,
        });
    }

    Ok(layer)
}

fn infer_type(v: &Jv) -> FieldType {
    match v {
        Jv::Bool(_) => FieldType::Boolean,
        Jv::Num(n) => {
            if n.fract() == 0.0 {
                FieldType::Integer
            } else {
                FieldType::Float
            }
        }
        Jv::Arr(_) | Jv::Obj(_) => FieldType::Json,
        Jv::Null => FieldType::Text,
        Jv::Str(_) => FieldType::Text,
    }
}

fn jv_to_field(v: &Jv, ft: FieldType) -> FieldValue {
    match (v, ft) {
        (Jv::Null, _) => FieldValue::Null,
        (Jv::Bool(b), _) => FieldValue::Boolean(*b),
        (Jv::Num(n), FieldType::Integer) => FieldValue::Integer(*n as i64),
        (Jv::Num(n), _) => FieldValue::Float(*n),
        (Jv::Str(s), FieldType::Date) => FieldValue::Date(s.clone()),
        (Jv::Str(s), FieldType::DateTime) => FieldValue::DateTime(s.clone()),
        (Jv::Str(s), _) => FieldValue::Text(s.clone()),
        (Jv::Arr(_), _) | (Jv::Obj(_), _) => FieldValue::Text(jv_to_json(v)),
    }
}

#[derive(Debug, Clone)]
struct Transform {
    scale: [f64; 2],
    translate: [f64; 2],
}

fn parse_transform(v: Option<&Jv>) -> Result<Option<Transform>> {
    let Some(v) = v else {
        return Ok(None);
    };
    let scale = v
        .get("scale")
        .and_then(|x| x.as_arr())
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.scale missing".into()))?;
    let trans = v
        .get("translate")
        .and_then(|x| x.as_arr())
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.translate missing".into()))?;

    if scale.len() < 2 || trans.len() < 2 {
        return Err(GeoError::TopoJsonTopology(
            "transform arrays must have at least 2 values".into(),
        ));
    }

    let s0 = scale[0]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.scale[0] invalid".into()))?;
    let s1 = scale[1]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.scale[1] invalid".into()))?;
    let t0 = trans[0]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.translate[0] invalid".into()))?;
    let t1 = trans[1]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("transform.translate[1] invalid".into()))?;

    Ok(Some(Transform {
        scale: [s0, s1],
        translate: [t0, t1],
    }))
}

fn decode_arc(v: &Jv, transform: Option<&Transform>) -> Result<Vec<Coord>> {
    let pts = v
        .as_arr()
        .ok_or_else(|| GeoError::TopoJsonTopology("arc must be array".into()))?;

    let mut out = Vec::<Coord>::with_capacity(pts.len());
    let mut acc_x = 0.0;
    let mut acc_y = 0.0;

    for p in pts {
        let xy = p
            .as_arr()
            .ok_or_else(|| GeoError::TopoJsonTopology("arc point must be coordinate array".into()))?;
        if xy.len() < 2 {
            return Err(GeoError::TopoJsonTopology(
                "arc coordinate must have x and y".into(),
            ));
        }
        let x = xy[0]
            .as_f64()
            .ok_or_else(|| GeoError::TopoJsonTopology("arc x coordinate invalid".into()))?;
        let y = xy[1]
            .as_f64()
            .ok_or_else(|| GeoError::TopoJsonTopology("arc y coordinate invalid".into()))?;

        let (rx, ry) = if let Some(t) = transform {
            acc_x += x;
            acc_y += y;
            (
                acc_x * t.scale[0] + t.translate[0],
                acc_y * t.scale[1] + t.translate[1],
            )
        } else {
            (x, y)
        };
        out.push(Coord::xy(rx, ry));
    }

    Ok(out)
}

fn extract_entries(obj: &Jv, arcs: &[Vec<Coord>], out: &mut Vec<TopoEntry>) -> Result<()> {
    let t = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GeoError::TopoJsonMissing("objects.*.type".into()))?;

    match t {
        "FeatureCollection" => {
            let features = obj
                .get("features")
                .and_then(|v| v.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("FeatureCollection.features".into()))?;
            for f in features {
                let g = f.get("geometry").unwrap_or(&Jv::Null);
                let geom = if matches!(g, Jv::Null) {
                    None
                } else {
                    Some(parse_topo_geometry(g, arcs)?)
                };
                let props = match f.get("properties") {
                    Some(Jv::Obj(p)) => Some(p.clone()),
                    _ => None,
                };
                let id = f.get("id").cloned();
                out.push(TopoEntry {
                    geometry: geom,
                    properties: props,
                    id,
                });
            }
        }
        "Feature" => {
            let g = obj.get("geometry").unwrap_or(&Jv::Null);
            let geom = if matches!(g, Jv::Null) {
                None
            } else {
                Some(parse_topo_geometry(g, arcs)?)
            };
            let props = match obj.get("properties") {
                Some(Jv::Obj(p)) => Some(p.clone()),
                _ => None,
            };
            let id = obj.get("id").cloned();
            out.push(TopoEntry {
                geometry: geom,
                properties: props,
                id,
            });
        }
        _ => {
            let geom = Some(parse_topo_geometry(obj, arcs)?);
            let props = match obj.get("properties") {
                Some(Jv::Obj(p)) => Some(p.clone()),
                _ => None,
            };
            let id = obj.get("id").cloned();
            out.push(TopoEntry {
                geometry: geom,
                properties: props,
                id,
            });
        }
    }

    Ok(())
}

fn parse_topo_geometry(v: &Jv, arcs: &[Vec<Coord>]) -> Result<Geometry> {
    let t = v
        .get("type")
        .and_then(|x| x.as_str())
        .ok_or_else(|| GeoError::TopoJsonMissing("geometry.type".into()))?;

    match t {
        "Point" => {
            let c = parse_one_coord(
                v.get("coordinates")
                    .ok_or_else(|| GeoError::TopoJsonMissing("coordinates".into()))?,
            )?;
            Ok(Geometry::Point(c))
        }
        "MultiPoint" => {
            let arr = v
                .get("coordinates")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("coordinates".into()))?;
            let mut pts = Vec::with_capacity(arr.len());
            for p in arr {
                pts.push(parse_one_coord(p)?);
            }
            Ok(Geometry::MultiPoint(pts))
        }
        "LineString" => {
            let refs = v
                .get("arcs")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("LineString.arcs".into()))?;
            Ok(Geometry::LineString(stitch_arc_refs(refs, arcs)?))
        }
        "MultiLineString" => {
            let lines = v
                .get("arcs")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("MultiLineString.arcs".into()))?;
            let mut out = Vec::with_capacity(lines.len());
            for l in lines {
                let refs = l
                    .as_arr()
                    .ok_or_else(|| GeoError::TopoJsonTopology("MultiLineString arc list invalid".into()))?;
                out.push(stitch_arc_refs(refs, arcs)?);
            }
            Ok(Geometry::MultiLineString(out))
        }
        "Polygon" => {
            let rings = v
                .get("arcs")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("Polygon.arcs".into()))?;
            let mut parsed = Vec::with_capacity(rings.len());
            for r in rings {
                let refs = r
                    .as_arr()
                    .ok_or_else(|| GeoError::TopoJsonTopology("Polygon ring arc list invalid".into()))?;
                parsed.push(strip_closed(stitch_arc_refs(refs, arcs)?));
            }
            let exterior = parsed.first().cloned().unwrap_or_default();
            let interiors = if parsed.len() > 1 {
                parsed[1..].to_vec()
            } else {
                Vec::new()
            };
            Ok(Geometry::polygon(exterior, interiors))
        }
        "MultiPolygon" => {
            let polys = v
                .get("arcs")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("MultiPolygon.arcs".into()))?;
            let mut out = Vec::with_capacity(polys.len());
            for p in polys {
                let rings = p
                    .as_arr()
                    .ok_or_else(|| GeoError::TopoJsonTopology("MultiPolygon polygon ring list invalid".into()))?;
                let mut parsed = Vec::with_capacity(rings.len());
                for r in rings {
                    let refs = r
                        .as_arr()
                        .ok_or_else(|| GeoError::TopoJsonTopology("MultiPolygon ring arc list invalid".into()))?;
                    parsed.push(strip_closed(stitch_arc_refs(refs, arcs)?));
                }
                let ext = parsed.first().cloned().unwrap_or_default();
                let holes = if parsed.len() > 1 {
                    parsed[1..].to_vec()
                } else {
                    Vec::new()
                };
                out.push((ext, holes));
            }
            Ok(Geometry::multi_polygon(out))
        }
        "GeometryCollection" => {
            let geoms = v
                .get("geometries")
                .and_then(|x| x.as_arr())
                .ok_or_else(|| GeoError::TopoJsonMissing("GeometryCollection.geometries".into()))?;
            let mut out = Vec::with_capacity(geoms.len());
            for g in geoms {
                out.push(parse_topo_geometry(g, arcs)?);
            }
            Ok(Geometry::GeometryCollection(out))
        }
        other => Err(GeoError::TopoJsonType(other.to_owned())),
    }
}

fn parse_one_coord(v: &Jv) -> Result<Coord> {
    let arr = v
        .as_arr()
        .ok_or_else(|| GeoError::TopoJsonTopology("coordinate must be array".into()))?;
    if arr.len() < 2 {
        return Err(GeoError::TopoJsonTopology(
            "coordinate must have x and y".into(),
        ));
    }
    let x = arr[0]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("coordinate x invalid".into()))?;
    let y = arr[1]
        .as_f64()
        .ok_or_else(|| GeoError::TopoJsonTopology("coordinate y invalid".into()))?;
    Ok(Coord::xy(x, y))
}

fn strip_closed(mut coords: Vec<Coord>) -> Vec<Coord> {
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop();
    }
    coords
}

fn stitch_arc_refs(refs: &[Jv], arcs: &[Vec<Coord>]) -> Result<Vec<Coord>> {
    let mut out = Vec::<Coord>::new();
    for r in refs {
        let idx = r
            .as_i64()
            .ok_or_else(|| GeoError::TopoJsonTopology("arc reference must be integer".into()))?;
        let arc = resolve_arc(idx, arcs)?;
        append_arc(&mut out, &arc);
    }
    Ok(out)
}

fn resolve_arc(idx: i64, arcs: &[Vec<Coord>]) -> Result<Vec<Coord>> {
    let (arc_idx, reverse) = if idx >= 0 {
        (idx as usize, false)
    } else {
        ((-idx - 1) as usize, true)
    };
    let arc = arcs
        .get(arc_idx)
        .ok_or_else(|| GeoError::TopoJsonTopology(format!("arc index out of range: {idx}")))?;
    if reverse {
        let mut rev = arc.clone();
        rev.reverse();
        Ok(rev)
    } else {
        Ok(arc.clone())
    }
}

fn append_arc(out: &mut Vec<Coord>, arc: &[Coord]) {
    if out.is_empty() {
        out.extend_from_slice(arc);
        return;
    }
    if arc.is_empty() {
        return;
    }
    let start = if out.last() == arc.first() { 1 } else { 0 };
    out.extend(arc.iter().skip(start).cloned());
}

struct TopologyWriter {
    arcs: Vec<Vec<Coord>>,
    arc_index: HashMap<Vec<(u64, u64)>, usize>,
}

impl TopologyWriter {
    fn new() -> Self {
        Self {
            arcs: Vec::new(),
            arc_index: HashMap::new(),
        }
    }

    fn layer_to_topology(&mut self, layer: &Layer, options: TopoJsonWriteOptions) -> Jv {
        let mut features = Vec::<Jv>::new();

        for (i, f) in layer.features.iter().enumerate() {
            let geom_jv = match &f.geometry {
                Some(g) => self.geometry_to_topo(g),
                None => Jv::Null,
            };

            let mut props = Vec::<(String, Jv)>::new();
            for (idx, fd) in layer.schema.fields().iter().enumerate() {
                let v = f.attributes.get(idx).unwrap_or(&FieldValue::Null);
                if matches!(v, FieldValue::Null) {
                    continue;
                }
                props.push((fd.name.clone(), field_to_jv(v)));
            }

            let feature_obj = Jv::Obj(vec![
                ("type".into(), Jv::Str("Feature".into())),
                ("id".into(), Jv::Num(f.fid as f64)),
                ("properties".into(), Jv::Obj(props)),
                ("geometry".into(), geom_jv),
            ]);
            let _ = i;
            features.push(feature_obj);
        }

        let (arcs_jv, transform_jv, bbox_jv) = if let Some(q) = options.quantize {
            match quantize_arcs(&self.arcs, q) {
                Some((arcs, transform, bbox)) => {
                    let arcs_jv = Jv::Arr(
                        arcs
                            .iter()
                            .map(|arc| {
                                Jv::Arr(
                                    arc.iter()
                                        .map(|(dx, dy)| {
                                            Jv::Arr(vec![Jv::Num(*dx as f64), Jv::Num(*dy as f64)])
                                        })
                                        .collect(),
                                )
                            })
                            .collect(),
                    );
                    let transform_jv = Some(Jv::Obj(vec![
                        (
                            "scale".into(),
                            Jv::Arr(vec![Jv::Num(transform.scale[0]), Jv::Num(transform.scale[1])]),
                        ),
                        (
                            "translate".into(),
                            Jv::Arr(vec![
                                Jv::Num(transform.translate[0]),
                                Jv::Num(transform.translate[1]),
                            ]),
                        ),
                    ]));
                    let bbox_jv = Some(Jv::Arr(vec![
                        Jv::Num(bbox[0]),
                        Jv::Num(bbox[1]),
                        Jv::Num(bbox[2]),
                        Jv::Num(bbox[3]),
                    ]));
                    (arcs_jv, transform_jv, bbox_jv)
                }
                None => (
                    Jv::Arr(
                        self.arcs
                            .iter()
                            .map(|arc| {
                                Jv::Arr(
                                    arc.iter()
                                        .map(|c| Jv::Arr(vec![Jv::Num(c.x), Jv::Num(c.y)]))
                                        .collect(),
                                )
                            })
                            .collect(),
                    ),
                    None,
                    None,
                ),
            }
        } else {
            (
                Jv::Arr(
                    self.arcs
                        .iter()
                        .map(|arc| {
                            Jv::Arr(
                                arc.iter()
                                    .map(|c| Jv::Arr(vec![Jv::Num(c.x), Jv::Num(c.y)]))
                                    .collect(),
                            )
                        })
                        .collect(),
                ),
                None,
                if options.include_bbox {
                    compute_bbox(&self.arcs).map(|bb| {
                        Jv::Arr(vec![
                            Jv::Num(bb[0]),
                            Jv::Num(bb[1]),
                            Jv::Num(bb[2]),
                            Jv::Num(bb[3]),
                        ])
                    })
                } else {
                    None
                },
            )
        };

        let mut root = vec![
            ("type".into(), Jv::Str("Topology".into())),
            (
                "objects".into(),
                Jv::Obj(vec![(
                    "layer".into(),
                    Jv::Obj(vec![
                        ("type".into(), Jv::Str("FeatureCollection".into())),
                        ("features".into(), Jv::Arr(features)),
                    ]),
                )]),
            ),
            ("arcs".into(), arcs_jv),
        ];

        if let Some(t) = transform_jv {
            root.push(("transform".into(), t));
        }
        if let Some(bb) = bbox_jv {
            root.push(("bbox".into(), bb));
        }

        Jv::Obj(root)
    }

    fn geometry_to_topo(&mut self, g: &Geometry) -> Jv {
        match g {
            Geometry::Point(c) => Jv::Obj(vec![
                ("type".into(), Jv::Str("Point".into())),
                ("coordinates".into(), Jv::Arr(vec![Jv::Num(c.x), Jv::Num(c.y)])),
            ]),
            Geometry::MultiPoint(cs) => Jv::Obj(vec![
                ("type".into(), Jv::Str("MultiPoint".into())),
                (
                    "coordinates".into(),
                    Jv::Arr(
                        cs.iter()
                            .map(|c| Jv::Arr(vec![Jv::Num(c.x), Jv::Num(c.y)]))
                            .collect(),
                    ),
                ),
            ]),
            Geometry::LineString(cs) => Jv::Obj(vec![
                ("type".into(), Jv::Str("LineString".into())),
                (
                    "arcs".into(),
                    Jv::Arr(vec![Jv::Num(self.register_arc(cs) as f64)]),
                ),
            ]),
            Geometry::MultiLineString(lines) => Jv::Obj(vec![
                ("type".into(), Jv::Str("MultiLineString".into())),
                (
                    "arcs".into(),
                    Jv::Arr(
                        lines
                            .iter()
                            .map(|l| Jv::Arr(vec![Jv::Num(self.register_arc(l) as f64)]))
                            .collect(),
                    ),
                ),
            ]),
            Geometry::Polygon { exterior, interiors } => {
                let mut rings = Vec::<Jv>::new();
                rings.push(Jv::Arr(vec![Jv::Num(self.register_ring(exterior) as f64)]));
                for h in interiors {
                    rings.push(Jv::Arr(vec![Jv::Num(self.register_ring(h) as f64)]));
                }
                Jv::Obj(vec![
                    ("type".into(), Jv::Str("Polygon".into())),
                    ("arcs".into(), Jv::Arr(rings)),
                ])
            }
            Geometry::MultiPolygon(polys) => {
                let mut out = Vec::<Jv>::new();
                for (ext, holes) in polys {
                    let mut rings = Vec::<Jv>::new();
                    rings.push(Jv::Arr(vec![Jv::Num(self.register_ring(ext) as f64)]));
                    for h in holes {
                        rings.push(Jv::Arr(vec![Jv::Num(self.register_ring(h) as f64)]));
                    }
                    out.push(Jv::Arr(rings));
                }
                Jv::Obj(vec![
                    ("type".into(), Jv::Str("MultiPolygon".into())),
                    ("arcs".into(), Jv::Arr(out)),
                ])
            }
            Geometry::GeometryCollection(gs) => Jv::Obj(vec![
                ("type".into(), Jv::Str("GeometryCollection".into())),
                (
                    "geometries".into(),
                    Jv::Arr(gs.iter().map(|x| self.geometry_to_topo(x)).collect()),
                ),
            ]),
        }
    }

    fn register_ring(&mut self, ring: &Ring) -> i64 {
        let mut coords = ring.0.clone();
        if !coords.is_empty() && coords.first() != coords.last() {
            coords.push(coords[0].clone());
        }
        let canonical = canonicalize_closed_ring(&coords);
        self.register_arc(&canonical)
    }

    fn register_arc(&mut self, coords: &[Coord]) -> i64 {
        let normalized = normalize_arc_coords(coords);
        let fwd = arc_key(&normalized);
        if let Some(idx) = self.arc_index.get(&fwd) {
            return *idx as i64;
        }

        let rev_coords: Vec<Coord> = normalized.iter().cloned().rev().collect();
        let rev = arc_key(&rev_coords);
        if let Some(idx) = self.arc_index.get(&rev) {
            return -(*idx as i64) - 1;
        }

        let idx = self.arcs.len();
        self.arcs.push(normalized);
        self.arc_index.insert(fwd, idx);
        idx as i64
    }
}

fn arc_key(coords: &[Coord]) -> Vec<(u64, u64)> {
    coords.iter().map(|c| (c.x.to_bits(), c.y.to_bits())).collect()
}

fn compute_bbox(arcs: &[Vec<Coord>]) -> Option<[f64; 4]> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_any = false;

    for arc in arcs {
        for c in arc {
            min_x = min_x.min(c.x);
            min_y = min_y.min(c.y);
            max_x = max_x.max(c.x);
            max_y = max_y.max(c.y);
            has_any = true;
        }
    }

    if has_any {
        Some([min_x, min_y, max_x, max_y])
    } else {
        None
    }
}

fn quantize_arcs(arcs: &[Vec<Coord>], grid_size: u32) -> Option<(Vec<Vec<(i64, i64)>>, Transform, [f64; 4])> {
    if grid_size < 2 {
        return None;
    }
    let bbox = compute_bbox(arcs)?;
    let [min_x, min_y, max_x, max_y] = bbox;
    let denom = (grid_size - 1) as f64;
    let scale_x = if (max_x - min_x).abs() <= f64::EPSILON {
        1.0
    } else {
        (max_x - min_x) / denom
    };
    let scale_y = if (max_y - min_y).abs() <= f64::EPSILON {
        1.0
    } else {
        (max_y - min_y) / denom
    };

    let transform = Transform {
        scale: [scale_x, scale_y],
        translate: [min_x, min_y],
    };

    let mut out = Vec::<Vec<(i64, i64)>>::with_capacity(arcs.len());
    for arc in arcs {
        let mut q_arc = Vec::<(i64, i64)>::with_capacity(arc.len());
        let mut prev_x = 0i64;
        let mut prev_y = 0i64;
        for (i, c) in arc.iter().enumerate() {
            let qx = ((c.x - min_x) / scale_x).round() as i64;
            let qy = ((c.y - min_y) / scale_y).round() as i64;
            if i == 0 {
                q_arc.push((qx, qy));
            } else {
                q_arc.push((qx - prev_x, qy - prev_y));
            }
            prev_x = qx;
            prev_y = qy;
        }
        out.push(q_arc);
    }

    Some((out, transform, bbox))
}

fn normalize_arc_coords(coords: &[Coord]) -> Vec<Coord> {
    let mut out = Vec::<Coord>::with_capacity(coords.len());
    for c in coords {
        if out.last() == Some(c) {
            continue;
        }
        out.push(c.clone());
    }
    out
}

fn canonicalize_closed_ring(coords: &[Coord]) -> Vec<Coord> {
    if coords.len() < 4 {
        return normalize_arc_coords(coords);
    }

    let mut open = normalize_arc_coords(coords);
    if open.first() == open.last() {
        open.pop();
    }
    if open.len() < 3 {
        return normalize_arc_coords(coords);
    }

    let fwd = canonical_open_ring_rotation(&open);
    let rev_open: Vec<Coord> = open.iter().cloned().rev().collect();
    let rev = canonical_open_ring_rotation(&rev_open);

    let mut fwd_closed = fwd.clone();
    fwd_closed.push(fwd[0].clone());
    let mut rev_closed = rev.clone();
    rev_closed.push(rev[0].clone());

    let fwd_key = arc_key(&fwd_closed);
    let rev_key = arc_key(&rev_closed);
    if rev_key < fwd_key {
        rev_closed
    } else {
        fwd_closed
    }
}

fn canonical_open_ring_rotation(open: &[Coord]) -> Vec<Coord> {
    let n = open.len();
    let mut best = 0usize;
    for i in 1..n {
        for j in 0..n {
            let a = &open[(i + j) % n];
            let b = &open[(best + j) % n];
            let ak = (a.x.to_bits(), a.y.to_bits());
            let bk = (b.x.to_bits(), b.y.to_bits());
            if ak < bk {
                best = i;
                break;
            }
            if ak > bk {
                break;
            }
        }
    }

    (0..n).map(|j| open[(best + j) % n].clone()).collect()
}

fn field_to_jv(v: &FieldValue) -> Jv {
    match v {
        FieldValue::Null => Jv::Null,
        FieldValue::Integer(n) => Jv::Num(*n as f64),
        FieldValue::Float(n) => Jv::Num(*n),
        FieldValue::Boolean(b) => Jv::Bool(*b),
        FieldValue::Text(s) | FieldValue::Date(s) | FieldValue::DateTime(s) => Jv::Str(s.clone()),
        FieldValue::Blob(b) => {
            let mut hex = String::new();
            for byte in b {
                use std::fmt::Write as _;
                let _ = write!(&mut hex, "{byte:02X}");
            }
            Jv::Str(hex)
        }
    }
}

fn jv_to_json(v: &Jv) -> String {
    match v {
        Jv::Null => "null".into(),
        Jv::Bool(b) => b.to_string(),
        Jv::Num(n) => fmt_number(*n),
        Jv::Str(s) => {
            let mut out = String::new();
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Jv::Arr(arr) => {
            let mut out = String::from("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&jv_to_json(item));
            }
            out.push(']');
            out
        }
        Jv::Obj(obj) => {
            let mut out = String::from("{");
            for (i, (k, val)) in obj.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&jv_to_json(&Jv::Str(k.clone())));
                out.push(':');
                out.push_str(&jv_to_json(val));
            }
            out.push('}');
            out
        }
    }
}

fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_SIMPLE_POINTS: &str =
        include_str!("../../tests/fixtures/topojson_io/simple_points.topojson");
    const FIXTURE_SHARED_BOUNDARY_POLYGONS: &str =
        include_str!("../../tests/fixtures/topojson_io/shared_boundary_polygons.topojson");
    const FIXTURE_MULTILINES_SHARED_ARCS: &str =
        include_str!("../../tests/fixtures/topojson_io/multilines_shared_arcs.topojson");
    const FIXTURE_TRANSFORM_QUANTIZED: &str =
        include_str!("../../tests/fixtures/topojson_io/transform_quantized_example.topojson");
    const FIXTURE_MIXED_GEOMETRY_COLLECTION: &str =
        include_str!("../../tests/fixtures/topojson_io/mixed_geometry_collection.topojson");
    const FIXTURE_OBJECT_MAP_MULTI_MEMBER: &str =
        include_str!("../../tests/fixtures/topojson_io/object_map_multi_member.topojson");
    const FIXTURE_FOREIGN_MEMBERS_BBOX: &str =
        include_str!("../../tests/fixtures/topojson_io/foreign_members_bbox.topojson");
    const FIXTURE_REVERSED_ARC_LINES: &str =
        include_str!("../../tests/fixtures/topojson_io/reversed_arc_lines.topojson");
    const FIXTURE_MAPSHAPER_LIKE_GEOMETRY_COLLECTION: &str = include_str!(
        "../../tests/fixtures/topojson_io/mapshaper_like_geometry_collection.topojson"
    );
    const FIXTURE_TOPOJSON_SERVER_QUANTIZED_LIKE: &str =
        include_str!("../../tests/fixtures/topojson_io/topojson_server_quantized_like.topojson");
    const FIXTURE_FEATURE_COLLECTION_WITH_NULL_GEOMETRY: &str = include_str!(
        "../../tests/fixtures/topojson_io/feature_collection_with_null_geometry.topojson"
    );
    const FIXTURE_PROVENANCE_MANIFEST: &str =
        include_str!("../../tests/fixtures/topojson_io/provenance_manifest.json");

    const SIMPLE: &str = r#"{
      "type": "Topology",
      "objects": {
        "layer": {
          "type": "FeatureCollection",
          "features": [
            {
              "type": "Feature",
              "id": 1,
              "properties": {"name": "road"},
              "geometry": {"type": "LineString", "arcs": [0]}
            }
          ]
        }
      },
      "arcs": [
        [[0,0],[1,0],[2,0]]
      ]
    }"#;

    #[test]
    fn parse_simple_topology() {
        let layer = parse_str(SIMPLE).unwrap();
        assert_eq!(layer.len(), 1);
        assert!(layer.schema.field("name").is_some());
        assert!(layer.schema.field("topo_id").is_some());
        assert!(matches!(layer[0].geometry, Some(Geometry::LineString(_))));
    }

    #[test]
    fn write_then_read_roundtrip() {
        let mut layer = Layer::new("test");
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.push(Feature {
            fid: 3,
            geometry: Some(Geometry::line_string(vec![
                Coord::xy(0.0, 0.0),
                Coord::xy(1.0, 0.0),
                Coord::xy(2.0, 0.0),
            ])),
            attributes: vec![FieldValue::Text("r1".into())],
        });

        let s = to_string(&layer).unwrap();
        let out = parse_str(&s).unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(out[0].geometry, Some(Geometry::LineString(_))));
    }

    #[test]
    fn file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.topojson");

        let mut layer = Layer::new("test");
        layer.push(Feature {
            fid: 1,
            geometry: Some(Geometry::point(1.0, 2.0)),
            attributes: vec![],
        });

        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn fixture_corpus_parses() {
        let fixtures = [
            FIXTURE_SIMPLE_POINTS,
            FIXTURE_SHARED_BOUNDARY_POLYGONS,
            FIXTURE_MULTILINES_SHARED_ARCS,
            FIXTURE_TRANSFORM_QUANTIZED,
            FIXTURE_MIXED_GEOMETRY_COLLECTION,
            FIXTURE_OBJECT_MAP_MULTI_MEMBER,
            FIXTURE_FOREIGN_MEMBERS_BBOX,
            FIXTURE_REVERSED_ARC_LINES,
            FIXTURE_MAPSHAPER_LIKE_GEOMETRY_COLLECTION,
            FIXTURE_TOPOJSON_SERVER_QUANTIZED_LIKE,
            FIXTURE_FEATURE_COLLECTION_WITH_NULL_GEOMETRY,
        ];

        for text in fixtures {
            let layer = parse_str(text).unwrap();
            assert!(!layer.features.is_empty());
        }
    }

    #[test]
    fn fixture_transform_quantized_decodes_expected_coords() {
        let layer = parse_str(FIXTURE_TRANSFORM_QUANTIZED).unwrap();
        assert_eq!(layer.len(), 1);
        let Some(Geometry::LineString(coords)) = &layer[0].geometry else {
            panic!("expected line string geometry");
        };
        assert_eq!(coords.len(), 3);
        assert!((coords[0].x - 100.0).abs() < 1e-12);
        assert!((coords[0].y - 200.0).abs() < 1e-12);
        assert!((coords[1].x - 101.0).abs() < 1e-12);
        assert!((coords[1].y - 200.0).abs() < 1e-12);
        assert!((coords[2].x - 102.0).abs() < 1e-12);
        assert!((coords[2].y - 201.0).abs() < 1e-12);
    }

    #[test]
    fn writer_canonicalizes_ring_rotation_and_orientation() {
        let ring_a = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(2.0, 0.0),
            Coord::xy(2.0, 2.0),
            Coord::xy(0.0, 2.0),
        ];
        let ring_b = vec![
            Coord::xy(2.0, 2.0),
            Coord::xy(2.0, 0.0),
            Coord::xy(0.0, 0.0),
            Coord::xy(0.0, 2.0),
        ];

        let mut layer = Layer::new("test");
        layer.push(Feature {
            fid: 1,
            geometry: Some(Geometry::polygon(ring_a, Vec::new())),
            attributes: vec![],
        });
        layer.push(Feature {
            fid: 2,
            geometry: Some(Geometry::polygon(ring_b, Vec::new())),
            attributes: vec![],
        });

        let text = to_string(&layer).unwrap();
        let mut parser = Parser::new(&text);
        let root = parser.parse_value().unwrap();
        let arcs = root.get("arcs").and_then(Jv::as_arr).unwrap();
        assert_eq!(arcs.len(), 1);
    }

    #[test]
    fn writer_quantize_emits_transform_and_bbox() {
        let mut layer = Layer::new("q");
        layer.push(Feature {
            fid: 1,
            geometry: Some(Geometry::line_string(vec![
                Coord::xy(10.0, 20.0),
                Coord::xy(11.0, 20.0),
                Coord::xy(12.0, 21.0),
            ])),
            attributes: vec![],
        });

        let text = to_string_with_options(
            &layer,
            TopoJsonWriteOptions::default().with_quantize(256).with_bbox(true),
        )
        .unwrap();

        let mut parser = Parser::new(&text);
        let root = parser.parse_value().unwrap();
        assert!(root.get("transform").is_some());
        assert!(root.get("bbox").is_some());
    }

    #[test]
    fn writer_quantize_roundtrip_preserves_shape_within_tolerance() {
        let mut layer = Layer::new("q");
        layer.push(Feature {
            fid: 1,
            geometry: Some(Geometry::line_string(vec![
                Coord::xy(10.0, 20.0),
                Coord::xy(11.25, 20.5),
                Coord::xy(12.0, 21.0),
            ])),
            attributes: vec![],
        });

        let text = to_string_with_options(
            &layer,
            TopoJsonWriteOptions::default().with_quantize(1024),
        )
        .unwrap();
        let out = parse_str(&text).unwrap();

        let Some(Geometry::LineString(coords)) = &out[0].geometry else {
            panic!("expected line string geometry");
        };
        assert_eq!(coords.len(), 3);
        assert!((coords[0].x - 10.0).abs() < 0.01);
        assert!((coords[0].y - 20.0).abs() < 0.01);
        assert!((coords[2].x - 12.0).abs() < 0.01);
        assert!((coords[2].y - 21.0).abs() < 0.01);
    }

    #[test]
    fn fixture_object_map_multi_member_flattens_objects() {
        let layer = parse_str(FIXTURE_OBJECT_MAP_MULTI_MEMBER).unwrap();
        assert_eq!(layer.len(), 2);
        assert!(layer.schema.field("name").is_some());

        let geometry_collection_count = layer
            .features
            .iter()
            .filter(|f| matches!(f.geometry, Some(Geometry::GeometryCollection(_))))
            .count();
        let polygon_count = layer
            .features
            .iter()
            .filter(|f| matches!(f.geometry, Some(Geometry::Polygon { .. })))
            .count();
        assert_eq!(geometry_collection_count, 1);
        assert_eq!(polygon_count, 1);
    }

    #[test]
    fn fixture_foreign_members_bbox_is_tolerated() {
        let layer = parse_str(FIXTURE_FOREIGN_MEMBERS_BBOX).unwrap();
        assert_eq!(layer.len(), 1);
        assert!(layer.schema.field("name").is_some());
        assert!(layer.schema.field("weight").is_some());
        assert!(layer.schema.field("active").is_some());
        assert!(layer.schema.field("meta").is_some());
        assert!(layer.schema.field("topo_id").is_some());
    }

    #[test]
    fn fixture_reversed_arc_lines_reverse_coordinates() {
        let layer = parse_str(FIXTURE_REVERSED_ARC_LINES).unwrap();
        assert_eq!(layer.len(), 2);

        let Some(Geometry::LineString(forward)) = &layer[0].geometry else {
            panic!("expected first feature to be a LineString");
        };
        let Some(Geometry::LineString(reverse)) = &layer[1].geometry else {
            panic!("expected second feature to be a LineString");
        };

        assert_eq!(forward.len(), reverse.len());
        assert_eq!(forward.first(), reverse.last());
        assert_eq!(forward.last(), reverse.first());
    }

    #[test]
    fn fixture_mapshaper_like_geometry_collection_parses_two_polygons() {
        let layer = parse_str(FIXTURE_MAPSHAPER_LIKE_GEOMETRY_COLLECTION).unwrap();
        assert_eq!(layer.len(), 1);
        let Some(Geometry::GeometryCollection(geoms)) = &layer[0].geometry else {
            panic!("expected a top-level GeometryCollection");
        };
        assert_eq!(geoms.len(), 2);
        assert!(matches!(geoms[0], Geometry::Polygon { .. }));
        assert!(matches!(geoms[1], Geometry::Polygon { .. }));
    }

    #[test]
    fn fixture_topojson_server_quantized_like_decodes_expected_endpoints() {
        let layer = parse_str(FIXTURE_TOPOJSON_SERVER_QUANTIZED_LIKE).unwrap();
        assert_eq!(layer.len(), 1);
        let Some(Geometry::GeometryCollection(geoms)) = &layer[0].geometry else {
            panic!("expected a top-level GeometryCollection");
        };
        assert_eq!(geoms.len(), 2);

        let Geometry::LineString(first) = &geoms[0] else {
            panic!("expected first geometry to be a line string");
        };
        assert!((first.first().unwrap().x - 10.0).abs() < 1e-12);
        assert!((first.first().unwrap().y - 20.0).abs() < 1e-12);

        let Geometry::LineString(second) = &geoms[1] else {
            panic!("expected second geometry to be a line string");
        };
        assert_eq!(second.first(), first.last());
    }

    #[test]
    fn fixture_feature_collection_with_null_geometry_preserves_feature_count() {
        let layer = parse_str(FIXTURE_FEATURE_COLLECTION_WITH_NULL_GEOMETRY).unwrap();
        assert_eq!(layer.len(), 2);
        assert!(layer.schema.field("name").is_some());
        assert!(layer.schema.field("topo_id").is_some());
        assert!(layer[0].geometry.is_none());
        assert!(matches!(layer[1].geometry, Some(Geometry::Point(_))));
    }

    #[test]
    fn fixture_provenance_manifest_matches_fixture_set() {
        let mut parser = Parser::new(FIXTURE_PROVENANCE_MANIFEST);
        let manifest = parser.parse_value().unwrap();
        let entries = manifest
            .get("entries")
            .and_then(Jv::as_arr)
            .expect("manifest must contain entries array");

        let mut listed = std::collections::HashSet::<String>::new();
        for entry in entries {
            let file_name = entry
                .get("file")
                .and_then(Jv::as_str)
                .expect("each manifest entry must contain file string");
            listed.insert(file_name.to_owned());
        }

        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("topojson_io");

        let mut discovered = std::collections::HashSet::<String>::new();
        for item in std::fs::read_dir(&fixture_dir).expect("fixture directory must exist") {
            let path = item.expect("directory entry must be readable").path();
            let is_topojson = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("topojson"))
                .unwrap_or(false);
            if !is_topojson {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("fixture file name must be valid utf-8")
                .to_owned();
            discovered.insert(name);
        }

        assert_eq!(listed, discovered);
    }
}
