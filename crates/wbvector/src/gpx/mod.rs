//! GPX (GPS Exchange Format) reader and writer.
//!
//! This module targets practical GPX 1.1 interoperability:
//! - Waypoints (`wpt`) -> `Point`
//! - Routes (`rte` + `rtept`) -> `LineString`
//! - Tracks (`trk` + `trkseg` + `trkpt`) -> `LineString` / `MultiLineString`
//! - Common metadata fields (`name`, `desc`, `time`, `type`) and extension attrs
//!
//! CRS behavior:
//! - GPX coordinates are lon/lat WGS84
//! - Parsed layers are assigned `crs_epsg = Some(4326)`

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Coord, Geometry, GeometryType};

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a GPX file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let text = std::fs::read_to_string(path).map_err(GeoError::Io)?;
    parse_str(&text)
}

/// Parse a GPX string into a [`Layer`].
pub fn parse_str(text: &str) -> Result<Layer> {
    let root = parse_xml(text)?;
    layer_from_xml(&root)
}

/// Write a [`Layer`] as GPX to a file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    std::fs::write(path, to_string(layer)?.as_bytes()).map_err(GeoError::Io)
}

/// Serialize a [`Layer`] to GPX string.
pub fn to_string(layer: &Layer) -> Result<String> {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<gpx version=\"1.1\" creator=\"wbvector\" xmlns=\"http://www.topografix.com/GPX/1/1\" xmlns:wbv=\"https://wbvector.rs/gpx\">\n",
    );

    let name_idx = layer.schema.field_index("name");
    let desc_idx = layer.schema.field_index("desc");
    let time_idx = layer.schema.field_index("time");
    let type_idx = layer.schema.field_index("type");
    let kind_idx = layer.schema.field_index("gpx_type");

    for feat in &layer.features {
        let kind = kind_idx
            .and_then(|i| feat.attributes.get(i))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let write_as_track = matches!(feat.geometry, Some(Geometry::MultiLineString(_))) || kind.eq_ignore_ascii_case("trk");
        let write_as_waypoint = matches!(feat.geometry, Some(Geometry::Point(_))) && kind.eq_ignore_ascii_case("wpt");

        match feat.geometry.as_ref() {
            Some(Geometry::Point(c)) => {
                if write_as_waypoint {
                    out.push_str(&format!("  <wpt lat=\"{}\" lon=\"{}\">\n", c.y, c.x));
                    write_common_gpx_fields(&mut out, feat, layer, name_idx, desc_idx, time_idx, type_idx, 4);
                    write_extension_fields(&mut out, feat, layer, &[name_idx, desc_idx, time_idx, type_idx, kind_idx], 4);
                    out.push_str("  </wpt>\n");
                } else {
                    out.push_str("  <rte>\n");
                    write_common_gpx_fields(&mut out, feat, layer, name_idx, desc_idx, time_idx, type_idx, 4);
                    out.push_str(&format!("    <rtept lat=\"{}\" lon=\"{}\"/>\n", c.y, c.x));
                    write_extension_fields(&mut out, feat, layer, &[name_idx, desc_idx, time_idx, type_idx, kind_idx], 4);
                    out.push_str("  </rte>\n");
                }
            }
            Some(Geometry::LineString(cs)) => {
                if write_as_track {
                    out.push_str("  <trk>\n");
                    write_common_gpx_fields(&mut out, feat, layer, name_idx, desc_idx, time_idx, type_idx, 4);
                    out.push_str("    <trkseg>\n");
                    for c in cs {
                        out.push_str(&format!("      <trkpt lat=\"{}\" lon=\"{}\">", c.y, c.x));
                        if let Some(z) = c.z {
                            out.push_str("<ele>");
                            out.push_str(&z.to_string());
                            out.push_str("</ele>");
                        }
                        out.push_str("</trkpt>\n");
                    }
                    out.push_str("    </trkseg>\n");
                    write_extension_fields(&mut out, feat, layer, &[name_idx, desc_idx, time_idx, type_idx, kind_idx], 4);
                    out.push_str("  </trk>\n");
                } else {
                    out.push_str("  <rte>\n");
                    write_common_gpx_fields(&mut out, feat, layer, name_idx, desc_idx, time_idx, type_idx, 4);
                    for c in cs {
                        out.push_str(&format!("    <rtept lat=\"{}\" lon=\"{}\">", c.y, c.x));
                        if let Some(z) = c.z {
                            out.push_str("<ele>");
                            out.push_str(&z.to_string());
                            out.push_str("</ele>");
                        }
                        out.push_str("</rtept>\n");
                    }
                    write_extension_fields(&mut out, feat, layer, &[name_idx, desc_idx, time_idx, type_idx, kind_idx], 4);
                    out.push_str("  </rte>\n");
                }
            }
            Some(Geometry::MultiLineString(lines)) => {
                out.push_str("  <trk>\n");
                write_common_gpx_fields(&mut out, feat, layer, name_idx, desc_idx, time_idx, type_idx, 4);
                for line in lines {
                    out.push_str("    <trkseg>\n");
                    for c in line {
                        out.push_str(&format!("      <trkpt lat=\"{}\" lon=\"{}\">", c.y, c.x));
                        if let Some(z) = c.z {
                            out.push_str("<ele>");
                            out.push_str(&z.to_string());
                            out.push_str("</ele>");
                        }
                        out.push_str("</trkpt>\n");
                    }
                    out.push_str("    </trkseg>\n");
                }
                write_extension_fields(&mut out, feat, layer, &[name_idx, desc_idx, time_idx, type_idx, kind_idx], 4);
                out.push_str("  </trk>\n");
            }
            None => {}
            _ => {
                return Err(GeoError::NotImplemented(
                    "GPX writer supports Point, LineString, and MultiLineString geometries".into(),
                ));
            }
        }
    }

    out.push_str("</gpx>\n");
    Ok(out)
}

fn write_common_gpx_fields(
    out: &mut String,
    feat: &Feature,
    layer: &Layer,
    name_idx: Option<usize>,
    desc_idx: Option<usize>,
    time_idx: Option<usize>,
    type_idx: Option<usize>,
    indent: usize,
) {
    if let Some(i) = name_idx {
        if let Some(v) = feat.attributes.get(i) {
            let s = field_value_text(v);
            if !s.is_empty() {
                out.push_str(&" ".repeat(indent));
                out.push_str("<name>");
                escape_xml(out, &s);
                out.push_str("</name>\n");
            }
        }
    }
    if let Some(i) = desc_idx {
        if let Some(v) = feat.attributes.get(i) {
            let s = field_value_text(v);
            if !s.is_empty() {
                out.push_str(&" ".repeat(indent));
                out.push_str("<desc>");
                escape_xml(out, &s);
                out.push_str("</desc>\n");
            }
        }
    }
    if let Some(i) = time_idx {
        if let Some(v) = feat.attributes.get(i) {
            let s = field_value_text(v);
            if !s.is_empty() {
                out.push_str(&" ".repeat(indent));
                out.push_str("<time>");
                escape_xml(out, &s);
                out.push_str("</time>\n");
            }
        }
    }
    if let Some(i) = type_idx {
        if let Some(v) = feat.attributes.get(i) {
            let s = field_value_text(v);
            if !s.is_empty() {
                out.push_str(&" ".repeat(indent));
                out.push_str("<type>");
                escape_xml(out, &s);
                out.push_str("</type>\n");
            }
        }
    }

    let _ = layer; // keep signature aligned with future expansion
}

fn write_extension_fields(
    out: &mut String,
    feat: &Feature,
    layer: &Layer,
    skip_indices: &[Option<usize>],
    indent: usize,
) {
    let mut skip = HashSet::new();
    for idx in skip_indices.iter().flatten() {
        skip.insert(*idx);
    }

    let mut wrote_any = false;
    for (i, fd) in layer.schema.fields().iter().enumerate() {
        if skip.contains(&i) {
            continue;
        }
        let v = feat.attributes.get(i).cloned().unwrap_or(FieldValue::Null);
        if matches!(v, FieldValue::Null) {
            continue;
        }
        if !wrote_any {
            out.push_str(&" ".repeat(indent));
            out.push_str("<extensions>\n");
            wrote_any = true;
        }
        out.push_str(&" ".repeat(indent + 2));
        out.push_str("<wbv:attr name=\"");
        escape_xml(out, &fd.name);
        out.push_str("\">");
        escape_xml(out, &field_value_text(&v));
        out.push_str("</wbv:attr>\n");
    }

    if wrote_any {
        out.push_str(&" ".repeat(indent));
        out.push_str("</extensions>\n");
    }
}

fn field_value_text(v: &FieldValue) -> String {
    match v {
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Float(n) => n.to_string(),
        FieldValue::Text(s) => s.clone(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::Blob(b) => format!("<blob {} bytes>", b.len()),
        FieldValue::Date(s) => s.clone(),
        FieldValue::DateTime(s) => s.clone(),
        FieldValue::Null => String::new(),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// XML model + parser
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct XmlNode {
    name: String,
    attrs: HashMap<String, String>,
    children: Vec<XmlNode>,
    text: String,
}

impl XmlNode {
    fn local_name(&self) -> &str {
        local_name(&self.name)
    }

    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .get(key)
            .or_else(|| self.attrs.iter().find_map(|(k, v)| (local_name(k) == key).then_some(v)))
            .map(|s| s.as_str())
    }

    fn child_by_local(&self, name: &str) -> Option<&XmlNode> {
        self.children.iter().find(|c| c.local_name() == name)
    }

    fn children_by_local<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
        self.children.iter().filter(move |c| c.local_name() == name)
    }

    fn element_children(&self) -> impl Iterator<Item = &XmlNode> {
        self.children.iter()
    }
}

fn parse_xml(input: &str) -> Result<XmlNode> {
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if i + 4 <= bytes.len() && &bytes[i..i + 4] == b"<!--" {
                if let Some(end) = find_bytes(bytes, i + 4, b"-->") {
                    i = end + 3;
                    continue;
                }
                return Err(GeoError::GpxParse { offset: i, msg: "unterminated comment".into() });
            }
            if i + 2 <= bytes.len() && bytes[i + 1] == b'?' {
                if let Some(end) = find_bytes(bytes, i + 2, b"?>") {
                    i = end + 2;
                    continue;
                }
                return Err(GeoError::GpxParse { offset: i, msg: "unterminated xml declaration".into() });
            }
            if i + 9 <= bytes.len() && &bytes[i..i + 9] == b"<![CDATA[" {
                if let Some(end) = find_bytes(bytes, i + 9, b"]]>") {
                    if let Some(cur) = stack.last_mut() {
                        cur.text.push_str(&input[i + 9..end]);
                    }
                    i = end + 3;
                    continue;
                }
                return Err(GeoError::GpxParse { offset: i, msg: "unterminated CDATA".into() });
            }
            if i + 2 <= bytes.len() && bytes[i + 1] == b'/' {
                let end = find_gt(bytes, i + 2).ok_or_else(|| GeoError::GpxParse {
                    offset: i,
                    msg: "unterminated end tag".into(),
                })?;
                let name = input[i + 2..end].trim();
                let node = stack.pop().ok_or_else(|| GeoError::GpxParse {
                    offset: i,
                    msg: "unexpected end tag".into(),
                })?;
                if local_name(name) != local_name(&node.name) {
                    return Err(GeoError::GpxParse {
                        offset: i,
                        msg: format!("mismatched end tag: expected </{}>", node.name),
                    });
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
                i = end + 1;
                continue;
            }

            let end = find_gt(bytes, i + 1).ok_or_else(|| GeoError::GpxParse {
                offset: i,
                msg: "unterminated start tag".into(),
            })?;
            let mut raw = input[i + 1..end].trim();
            let self_close = raw.ends_with('/');
            if self_close {
                raw = raw[..raw.len() - 1].trim_end();
            }

            let (name, attrs) = parse_start_tag(raw, i)?;
            let node = XmlNode { name, attrs, children: Vec::new(), text: String::new() };
            if self_close {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.is_none() {
                    root = Some(node);
                } else {
                    return Err(GeoError::GpxParse { offset: i, msg: "multiple root elements".into() });
                }
            } else {
                stack.push(node);
            }
            i = end + 1;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            if let Some(cur) = stack.last_mut() {
                let t = decode_entities(&input[start..i]);
                if !t.trim().is_empty() {
                    if !cur.text.is_empty() {
                        cur.text.push(' ');
                    }
                    cur.text.push_str(t.trim());
                }
            }
        }
    }

    if !stack.is_empty() {
        return Err(GeoError::GpxParse {
            offset: input.len(),
            msg: "unclosed tags".into(),
        });
    }

    root.ok_or_else(|| GeoError::GpxParse { offset: 0, msg: "no root element".into() })
}

fn find_gt(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut in_quote = false;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quote = !in_quote,
            b'>' if !in_quote => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn find_bytes(hay: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    hay[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| start + p)
}

fn parse_start_tag(raw: &str, offset: usize) -> Result<(String, HashMap<String, String>)> {
    let mut i = 0usize;
    let bytes = raw.as_bytes();

    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let name = raw[..i].to_owned();
    let mut attrs = HashMap::new();

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let k0 = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key = raw[k0..i].trim().to_owned();

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            return Err(GeoError::GpxParse {
                offset,
                msg: format!("expected '=' after attribute '{key}'"),
            });
        }
        i += 1;

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(GeoError::GpxParse { offset, msg: "truncated attribute value".into() });
        }

        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            return Err(GeoError::GpxParse {
                offset,
                msg: format!("attribute '{key}' value must be quoted"),
            });
        }
        i += 1;
        let v0 = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(GeoError::GpxParse { offset, msg: "unterminated quoted attribute".into() });
        }
        let val = decode_entities(&raw[v0..i]);
        i += 1;
        attrs.insert(key, val);
    }

    Ok((name, attrs))
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

// ══════════════════════════════════════════════════════════════════════════════
// XML -> Layer
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct GpxRecord {
    geom: Option<Geometry>,
    attrs: HashMap<String, String>,
}

fn layer_from_xml(root: &XmlNode) -> Result<Layer> {
    let mut records = Vec::<GpxRecord>::new();
    collect_gpx_records(root, &mut records)?;

    let mut key_order = vec!["gpx_type".to_owned()];
    let mut key_seen = HashSet::from(["gpx_type".to_owned()]);
    let mut key_type = HashMap::<String, FieldType>::new();
    key_type.insert("gpx_type".to_owned(), FieldType::Text);

    for rec in &records {
        for (k, v) in &rec.attrs {
            if key_seen.insert(k.clone()) {
                key_order.push(k.clone());
            }
            if v.trim().is_empty() {
                continue;
            }
            let inferred = infer_field_type(v, k);
            let entry = key_type.entry(k.clone()).or_insert(inferred);
            *entry = FieldValue::widen_type(*entry, inferred);
        }
    }

    let mut layer = Layer::new("layer").with_epsg(4326);
    for k in &key_order {
        let ty = key_type.get(k).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(k, ty));
    }

    let mut declared_geom: Option<GeometryType> = None;
    let mut mixed_geom = false;

    for (fid, rec) in records.into_iter().enumerate() {
        if let Some(g) = &rec.geom {
            let gt = g.geom_type();
            if let Some(prev) = declared_geom {
                if prev != gt {
                    mixed_geom = true;
                }
            } else {
                declared_geom = Some(gt);
            }
        }

        let mut attrs = vec![FieldValue::Null; layer.schema.len()];
        for (i, fd) in layer.schema.fields().iter().enumerate() {
            if let Some(raw) = rec.attrs.get(&fd.name) {
                attrs[i] = parse_value_as_type(raw, fd.field_type);
            }
        }

        layer.push(Feature {
            fid: fid as u64,
            geometry: rec.geom,
            attributes: attrs,
        });
    }

    layer.geom_type = if mixed_geom {
        Some(GeometryType::GeometryCollection)
    } else {
        declared_geom
    };

    Ok(layer)
}

fn collect_gpx_records(node: &XmlNode, out: &mut Vec<GpxRecord>) -> Result<()> {
    match node.local_name() {
        "wpt" => out.push(parse_wpt(node)?),
        "rte" => out.push(parse_rte(node)?),
        "trk" => out.push(parse_trk(node)?),
        _ => {}
    }

    for child in node.element_children() {
        collect_gpx_records(child, out)?;
    }

    Ok(())
}

fn parse_wpt(node: &XmlNode) -> Result<GpxRecord> {
    let lon = node.attr("lon").ok_or_else(|| GeoError::GpxParse {
        offset: 0,
        msg: "wpt missing lon".into(),
    })?;
    let lat = node.attr("lat").ok_or_else(|| GeoError::GpxParse {
        offset: 0,
        msg: "wpt missing lat".into(),
    })?;

    let x = lon.parse::<f64>().map_err(|_| GeoError::GpxParse {
        offset: 0,
        msg: format!("invalid lon '{lon}'"),
    })?;
    let y = lat.parse::<f64>().map_err(|_| GeoError::GpxParse {
        offset: 0,
        msg: format!("invalid lat '{lat}'"),
    })?;

    let z = node
        .child_by_local("ele")
        .and_then(|n| n.text.trim().parse::<f64>().ok());

    let mut attrs = parse_common_attrs(node);
    attrs.insert("gpx_type".to_owned(), "wpt".to_owned());
    parse_extensions(node, &mut attrs);

    Ok(GpxRecord {
        geom: Some(Geometry::Point(Coord { x, y, z, m: None })),
        attrs,
    })
}

fn parse_rte(node: &XmlNode) -> Result<GpxRecord> {
    let mut coords = Vec::<Coord>::new();
    for pt in node.children_by_local("rtept") {
        coords.push(parse_lat_lon_point(pt, "rtept")?);
    }

    let mut attrs = parse_common_attrs(node);
    attrs.insert("gpx_type".to_owned(), "rte".to_owned());
    parse_extensions(node, &mut attrs);

    let geom = if coords.is_empty() {
        None
    } else {
        Some(Geometry::LineString(coords))
    };

    Ok(GpxRecord { geom, attrs })
}

fn parse_trk(node: &XmlNode) -> Result<GpxRecord> {
    let mut segments = Vec::<Vec<Coord>>::new();
    for seg in node.children_by_local("trkseg") {
        let mut coords = Vec::<Coord>::new();
        for pt in seg.children_by_local("trkpt") {
            coords.push(parse_lat_lon_point(pt, "trkpt")?);
        }
        if !coords.is_empty() {
            segments.push(coords);
        }
    }

    let mut attrs = parse_common_attrs(node);
    attrs.insert("gpx_type".to_owned(), "trk".to_owned());
    parse_extensions(node, &mut attrs);

    let geom = if segments.is_empty() {
        None
    } else if segments.len() == 1 {
        Some(Geometry::LineString(segments.remove(0)))
    } else {
        Some(Geometry::MultiLineString(segments))
    };

    Ok(GpxRecord { geom, attrs })
}

fn parse_lat_lon_point(node: &XmlNode, tag: &str) -> Result<Coord> {
    let lon = node.attr("lon").ok_or_else(|| GeoError::GpxParse {
        offset: 0,
        msg: format!("{tag} missing lon"),
    })?;
    let lat = node.attr("lat").ok_or_else(|| GeoError::GpxParse {
        offset: 0,
        msg: format!("{tag} missing lat"),
    })?;

    let x = lon.parse::<f64>().map_err(|_| GeoError::GpxParse {
        offset: 0,
        msg: format!("invalid lon '{lon}'"),
    })?;
    let y = lat.parse::<f64>().map_err(|_| GeoError::GpxParse {
        offset: 0,
        msg: format!("invalid lat '{lat}'"),
    })?;

    let z = node
        .child_by_local("ele")
        .and_then(|n| n.text.trim().parse::<f64>().ok());

    Ok(Coord { x, y, z, m: None })
}

fn parse_common_attrs(node: &XmlNode) -> HashMap<String, String> {
    let mut attrs = HashMap::<String, String>::new();
    for key in ["name", "desc", "time", "type"] {
        if let Some(ch) = node.child_by_local(key) {
            let v = ch.text.trim();
            if !v.is_empty() {
                attrs.insert(key.to_owned(), v.to_owned());
            }
        }
    }
    attrs
}

fn parse_extensions(node: &XmlNode, attrs: &mut HashMap<String, String>) {
    if let Some(ext) = node.child_by_local("extensions") {
        for ch in ext.element_children() {
            if let Some(name) = ch.attr("name") {
                attrs.insert(name.to_owned(), ch.text.trim().to_owned());
            } else {
                let key = ch.local_name().to_owned();
                if !key.is_empty() {
                    attrs.insert(key, ch.text.trim().to_owned());
                }
            }
        }
    }
}

fn infer_field_type(value: &str, key: &str) -> FieldType {
    if key.eq_ignore_ascii_case("time") && is_iso_datetime(value.trim()) {
        return FieldType::DateTime;
    }
    let s = value.trim();
    if s.is_empty() {
        return FieldType::Text;
    }
    if is_bool_text(s) {
        return FieldType::Boolean;
    }
    if s.parse::<i64>().is_ok() {
        return FieldType::Integer;
    }
    if s.parse::<f64>().is_ok() {
        return FieldType::Float;
    }
    FieldType::Text
}

fn parse_value_as_type(raw: &str, ty: FieldType) -> FieldValue {
    let s = raw.trim();
    if s.is_empty() {
        return FieldValue::Null;
    }

    match ty {
        FieldType::Boolean => {
            let lower = s.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "t" => FieldValue::Boolean(true),
                "false" | "0" | "no" | "f" => FieldValue::Boolean(false),
                _ => FieldValue::Text(raw.to_owned()),
            }
        }
        FieldType::Integer => s
            .parse::<i64>()
            .map(FieldValue::Integer)
            .unwrap_or_else(|_| FieldValue::Text(raw.to_owned())),
        FieldType::Float => s
            .parse::<f64>()
            .map(FieldValue::Float)
            .unwrap_or_else(|_| FieldValue::Text(raw.to_owned())),
        FieldType::Date => FieldValue::Date(s.to_owned()),
        FieldType::DateTime => FieldValue::DateTime(s.to_owned()),
        _ => FieldValue::Text(raw.to_owned()),
    }
}

fn is_bool_text(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "true" | "false" | "1" | "0" | "yes" | "no" | "t" | "f"
    )
}

fn is_iso_datetime(s: &str) -> bool {
    s.contains('T') && (s.ends_with('Z') || s.contains('+') || s.matches(':').count() >= 2)
}

fn escape_xml(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FieldDef, FieldType};

    #[test]
    fn gpx_roundtrip_waypoint() {
        let mut layer = Layer::new("waypoints")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        layer.add_field(FieldDef::new("gpx_type", FieldType::Text));
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.add_field(FieldDef::new("time", FieldType::DateTime));
        layer
            .add_feature(
                Some(Geometry::point(-79.38, 43.72)),
                &[
                    ("gpx_type", "wpt".into()),
                    ("name", "Bench".into()),
                    ("time", "2026-03-12T10:20:30Z".into()),
                ],
            )
            .unwrap();

        let text = to_string(&layer).unwrap();
        let parsed = parse_str(&text).unwrap();

        assert_eq!(parsed.crs_epsg(), Some(4326));
        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed.features[0].geometry, Some(Geometry::Point(_))));
        assert_eq!(
            parsed.features[0].get(&parsed.schema, "gpx_type").unwrap().as_str(),
            Some("wpt")
        );
    }

    #[test]
    fn parses_route_and_track() {
        let gpx = r#"
<gpx version="1.1" creator="x" xmlns="http://www.topografix.com/GPX/1/1">
  <rte>
    <name>route A</name>
    <rtept lat="43.70" lon="-79.40"/>
    <rtept lat="43.71" lon="-79.39"/>
  </rte>
  <trk>
    <name>track B</name>
    <trkseg>
      <trkpt lat="43.70" lon="-79.40"/>
      <trkpt lat="43.71" lon="-79.39"/>
    </trkseg>
    <trkseg>
      <trkpt lat="43.72" lon="-79.38"/>
      <trkpt lat="43.73" lon="-79.37"/>
    </trkseg>
  </trk>
</gpx>
"#;
        let layer = parse_str(gpx).unwrap();
        assert_eq!(layer.len(), 2);
        assert_eq!(layer.crs_epsg(), Some(4326));

        assert!(matches!(layer.features[0].geometry, Some(Geometry::LineString(_))));
        assert!(matches!(layer.features[1].geometry, Some(Geometry::MultiLineString(_))));
    }

    #[test]
    fn parses_extension_attrs() {
        let gpx = r#"
<gpx version="1.1" creator="x" xmlns="http://www.topografix.com/GPX/1/1" xmlns:wbv="https://wbvector.rs/gpx">
  <wpt lat="43.72" lon="-79.38">
    <name>Bench</name>
    <extensions>
      <wbv:attr name="surface">gravel</wbv:attr>
      <wbv:attr name="difficulty">2</wbv:attr>
    </extensions>
  </wpt>
</gpx>
"#;
        let layer = parse_str(gpx).unwrap();
        assert_eq!(layer.features[0].get(&layer.schema, "surface").unwrap().as_str(), Some("gravel"));
        assert_eq!(layer.features[0].get(&layer.schema, "difficulty").unwrap().as_i64(), Some(2));
    }
}
