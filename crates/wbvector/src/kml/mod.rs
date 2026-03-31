//! KML (Keyhole Markup Language) reader and writer.
//!
//! This module targets practical KML 2.x interoperability:
//! - `kml` root with `Document`/`Folder` containers
//! - `Placemark` records
//! - Geometries: `Point`, `LineString`, `Polygon`, `MultiGeometry`
//! - Attributes from `<name>`, `<description>`, and `<ExtendedData>`
//!
//! CRS behavior:
//! - KML coordinates are interpreted as lon/lat (EPSG:4326)
//! - Parsed layers are assigned `crs_epsg = Some(4326)`

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Coord, Geometry, GeometryType, Ring};
use crate::reproject;

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a KML file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let text = std::fs::read_to_string(path).map_err(GeoError::Io)?;
    parse_str(&text)
}

/// Parse a KML string into a [`Layer`].
pub fn parse_str(text: &str) -> Result<Layer> {
    let root = parse_xml(text)?;
    layer_from_xml(&root)
}

/// Write a [`Layer`] as KML to a file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let out_layer = prepare_kml_layer(layer)?;
    std::fs::write(path, to_string(&out_layer).as_bytes()).map_err(GeoError::Io)
}

fn prepare_kml_layer(layer: &Layer) -> Result<Layer> {
    // KML requires WGS 84 lon/lat coordinates. If CRS metadata is present and
    // not already EPSG:4326, reproject on write.
    if layer.crs_epsg() == Some(4326) {
        return Ok(layer.clone());
    }

    if layer.crs_epsg().is_some() || layer.crs_wkt().is_some() {
        return reproject::layer_to_epsg(layer, 4326);
    }

    Ok(layer.clone())
}

/// Serialize a [`Layer`] to a KML string.
pub fn to_string(layer: &Layer) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n");
    out.push_str("  <Document>\n");
    out.push_str("    <name>");
    escape_xml(&mut out, &layer.name);
    out.push_str("</name>\n");

    let name_idx = layer.schema.field_index("name");
    let description_idx = layer.schema.field_index("description");

    for feat in &layer.features {
        out.push_str("    <Placemark>\n");

        if let Some(idx) = name_idx {
            if let Some(v) = feat.attributes.get(idx) {
                let s = field_value_text(v);
                if !s.is_empty() {
                    out.push_str("      <name>");
                    escape_xml(&mut out, &s);
                    out.push_str("</name>\n");
                }
            }
        }

        if let Some(idx) = description_idx {
            if let Some(v) = feat.attributes.get(idx) {
                let s = field_value_text(v);
                if !s.is_empty() {
                    out.push_str("      <description>");
                    escape_xml(&mut out, &s);
                    out.push_str("</description>\n");
                }
            }
        }

        let mut wrote_any_data = false;
        for (i, fd) in layer.schema.fields().iter().enumerate() {
            if Some(i) == name_idx || Some(i) == description_idx {
                continue;
            }
            let v = feat.attributes.get(i).cloned().unwrap_or(FieldValue::Null);
            if matches!(v, FieldValue::Null) {
                continue;
            }
            if !wrote_any_data {
                out.push_str("      <ExtendedData>\n");
                wrote_any_data = true;
            }
            out.push_str("        <Data name=\"");
            escape_xml(&mut out, &fd.name);
            out.push_str("\"><value>");
            escape_xml(&mut out, &field_value_text(&v));
            out.push_str("</value></Data>\n");
        }
        if wrote_any_data {
            out.push_str("      </ExtendedData>\n");
        }

        if let Some(g) = &feat.geometry {
            write_geom(&mut out, g, 6);
        }

        out.push_str("    </Placemark>\n");
    }

    out.push_str("  </Document>\n");
    out.push_str("</kml>\n");
    out
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
                return Err(GeoError::GmlParse { offset: i, msg: "unterminated comment".into() });
            }
            if i + 2 <= bytes.len() && bytes[i + 1] == b'?' {
                if let Some(end) = find_bytes(bytes, i + 2, b"?>") {
                    i = end + 2;
                    continue;
                }
                return Err(GeoError::GmlParse { offset: i, msg: "unterminated xml declaration".into() });
            }
            if i + 9 <= bytes.len() && &bytes[i..i + 9] == b"<![CDATA[" {
                if let Some(end) = find_bytes(bytes, i + 9, b"]]>") {
                    if let Some(cur) = stack.last_mut() {
                        cur.text.push_str(&input[i + 9..end]);
                    }
                    i = end + 3;
                    continue;
                }
                return Err(GeoError::GmlParse { offset: i, msg: "unterminated CDATA".into() });
            }
            if i + 2 <= bytes.len() && bytes[i + 1] == b'/' {
                let end = find_gt(bytes, i + 2).ok_or_else(|| GeoError::GmlParse {
                    offset: i,
                    msg: "unterminated end tag".into(),
                })?;
                let name = input[i + 2..end].trim();
                let node = stack.pop().ok_or_else(|| GeoError::GmlParse {
                    offset: i,
                    msg: "unexpected end tag".into(),
                })?;
                if local_name(name) != local_name(&node.name) {
                    return Err(GeoError::GmlParse {
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

            let end = find_gt(bytes, i + 1).ok_or_else(|| GeoError::GmlParse {
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
                    return Err(GeoError::GmlParse { offset: i, msg: "multiple root elements".into() });
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
        return Err(GeoError::GmlParse {
            offset: input.len(),
            msg: "unclosed tags".into(),
        });
    }

    root.ok_or_else(|| GeoError::GmlParse { offset: 0, msg: "no root element".into() })
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
            return Err(GeoError::GmlParse {
                offset,
                msg: format!("expected '=' after attribute '{key}'"),
            });
        }
        i += 1;

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return Err(GeoError::GmlParse { offset, msg: "truncated attribute value".into() });
        }

        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            return Err(GeoError::GmlParse {
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
            return Err(GeoError::GmlParse { offset, msg: "unterminated quoted attribute".into() });
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
// XML → Layer
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct PlacemarkRecord {
    geom: Option<Geometry>,
    attrs: HashMap<String, String>,
}

fn layer_from_xml(root: &XmlNode) -> Result<Layer> {
    let layer_name = document_name(root).unwrap_or_else(|| "layer".to_owned());
    let mut records = Vec::<PlacemarkRecord>::new();
    collect_placemarks(root, &mut records)?;

    let mut key_order: Vec<String> = Vec::new();
    let mut key_seen: HashSet<String> = HashSet::new();
    let mut key_type: HashMap<String, FieldType> = HashMap::new();

    for rec in &records {
        for (k, v) in &rec.attrs {
            if key_seen.insert(k.clone()) {
                key_order.push(k.clone());
            }
            if v.trim().is_empty() {
                continue;
            }
            let inferred = infer_field_type(v);
            let entry = key_type.entry(k.clone()).or_insert(inferred);
            *entry = FieldValue::widen_type(*entry, inferred);
        }
    }

    let mut layer = Layer::new(layer_name).with_epsg(4326);
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

fn document_name(root: &XmlNode) -> Option<String> {
    if root.local_name() == "Document" {
        if let Some(name) = root.child_by_local("name") {
            let s = name.text.trim();
            if !s.is_empty() {
                return Some(s.to_owned());
            }
        }
    }

    for child in root.element_children() {
        if let Some(v) = document_name(child) {
            return Some(v);
        }
    }
    None
}

fn collect_placemarks(node: &XmlNode, out: &mut Vec<PlacemarkRecord>) -> Result<()> {
    if node.local_name() == "Placemark" {
        out.push(parse_placemark(node)?);
    }
    for child in node.element_children() {
        collect_placemarks(child, out)?;
    }
    Ok(())
}

fn parse_placemark(pm: &XmlNode) -> Result<PlacemarkRecord> {
    let mut attrs = HashMap::<String, String>::new();

    if let Some(name) = pm.child_by_local("name") {
        let v = name.text.trim();
        if !v.is_empty() {
            attrs.insert("name".to_owned(), v.to_owned());
        }
    }

    if let Some(description) = pm.child_by_local("description") {
        let v = description.text.trim();
        if !v.is_empty() {
            attrs.insert("description".to_owned(), v.to_owned());
        }
    }

    if let Some(ed) = pm.child_by_local("ExtendedData") {
        parse_extended_data(ed, &mut attrs);
    }

    let mut geom: Option<Geometry> = None;
    for child in pm.element_children() {
        if is_geom_tag(child.local_name()) {
            geom = Some(parse_geometry(child)?);
            break;
        }
    }

    Ok(PlacemarkRecord { geom, attrs })
}

fn parse_extended_data(node: &XmlNode, out: &mut HashMap<String, String>) {
    for data in node.children_by_local("Data") {
        if let Some(name) = data.attr("name") {
            let value = data
                .child_by_local("value")
                .map(|v| v.text.trim().to_owned())
                .unwrap_or_default();
            out.insert(name.to_owned(), value);
        }
    }

    for schema_data in node.children_by_local("SchemaData") {
        for simple_data in schema_data.children_by_local("SimpleData") {
            if let Some(name) = simple_data.attr("name") {
                out.insert(name.to_owned(), simple_data.text.trim().to_owned());
            }
        }
    }

    for simple_data in node.children_by_local("SimpleData") {
        if let Some(name) = simple_data.attr("name") {
            out.insert(name.to_owned(), simple_data.text.trim().to_owned());
        }
    }
}

fn is_geom_tag(local: &str) -> bool {
    matches!(
        local,
        "Point"
            | "LineString"
            | "Polygon"
            | "MultiGeometry"
            | "MultiPoint"
            | "MultiLineString"
            | "MultiPolygon"
    )
}

fn parse_geometry(node: &XmlNode) -> Result<Geometry> {
    match node.local_name() {
        "Point" => parse_point(node),
        "LineString" => parse_line_string(node),
        "Polygon" => parse_polygon(node),
        "MultiGeometry" => parse_multi_geometry(node),
        "MultiPoint" => parse_multi_point(node),
        "MultiLineString" => parse_multi_line_string(node),
        "MultiPolygon" => parse_multi_polygon(node),
        other => Err(GeoError::NotImplemented(format!("KML geometry '{other}'"))),
    }
}

fn parse_point(node: &XmlNode) -> Result<Geometry> {
    let coords = node
        .child_by_local("coordinates")
        .ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "KML Point missing coordinates".into(),
        })?;

    let mut cs = parse_kml_coordinates(&coords.text)?;
    let first = cs.drain(..).next().ok_or_else(|| GeoError::GmlParse {
        offset: 0,
        msg: "KML Point has empty coordinates".into(),
    })?;
    Ok(Geometry::Point(first))
}

fn parse_line_string(node: &XmlNode) -> Result<Geometry> {
    let coords = node
        .child_by_local("coordinates")
        .ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "KML LineString missing coordinates".into(),
        })?;
    let cs = parse_kml_coordinates(&coords.text)?;
    Ok(Geometry::LineString(cs))
}

fn parse_polygon(node: &XmlNode) -> Result<Geometry> {
    let outer = node
        .child_by_local("outerBoundaryIs")
        .ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "KML Polygon missing outerBoundaryIs".into(),
        })?;
    let outer_ring = parse_ring(
        outer
            .child_by_local("LinearRing")
            .ok_or_else(|| GeoError::GmlParse {
                offset: 0,
                msg: "KML Polygon outerBoundaryIs missing LinearRing".into(),
            })?,
    )?;

    let mut holes = Vec::<Ring>::new();
    for inner in node.children_by_local("innerBoundaryIs") {
        if let Some(ring) = inner.child_by_local("LinearRing") {
            holes.push(parse_ring(ring)?);
        }
    }

    Ok(Geometry::Polygon {
        exterior: outer_ring,
        interiors: holes,
    })
}

fn parse_ring(linear_ring: &XmlNode) -> Result<Ring> {
    let coords = linear_ring
        .child_by_local("coordinates")
        .ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "KML LinearRing missing coordinates".into(),
        })?;
    let mut cs = parse_kml_coordinates(&coords.text)?;
    close_ring_if_needed(&mut cs);
    Ok(Ring::new(cs))
}

fn parse_multi_geometry(node: &XmlNode) -> Result<Geometry> {
    let mut geoms = Vec::<Geometry>::new();
    for child in node.element_children() {
        if is_geom_tag(child.local_name()) {
            geoms.push(parse_geometry(child)?);
        }
    }

    if geoms.is_empty() {
        return Ok(Geometry::GeometryCollection(vec![]));
    }

    if geoms.iter().all(|g| matches!(g, Geometry::Point(_))) {
        let pts = geoms
            .into_iter()
            .filter_map(|g| match g {
                Geometry::Point(c) => Some(c),
                _ => None,
            })
            .collect();
        return Ok(Geometry::MultiPoint(pts));
    }

    if geoms.iter().all(|g| matches!(g, Geometry::LineString(_))) {
        let lines = geoms
            .into_iter()
            .filter_map(|g| match g {
                Geometry::LineString(cs) => Some(cs),
                _ => None,
            })
            .collect();
        return Ok(Geometry::MultiLineString(lines));
    }

    if geoms.iter().all(|g| matches!(g, Geometry::Polygon { .. })) {
        let polys = geoms
            .into_iter()
            .filter_map(|g| match g {
                Geometry::Polygon { exterior, interiors } => Some((exterior, interiors)),
                _ => None,
            })
            .collect();
        return Ok(Geometry::MultiPolygon(polys));
    }

    Ok(Geometry::GeometryCollection(geoms))
}

fn parse_multi_point(node: &XmlNode) -> Result<Geometry> {
    let mut points = Vec::new();
    for child in node.element_children() {
        if child.local_name() == "Point" {
            if let Geometry::Point(c) = parse_point(child)? {
                points.push(c);
            }
        }
    }
    Ok(Geometry::MultiPoint(points))
}

fn parse_multi_line_string(node: &XmlNode) -> Result<Geometry> {
    let mut lines = Vec::new();
    for child in node.element_children() {
        if child.local_name() == "LineString" {
            if let Geometry::LineString(cs) = parse_line_string(child)? {
                lines.push(cs);
            }
        }
    }
    Ok(Geometry::MultiLineString(lines))
}

fn parse_multi_polygon(node: &XmlNode) -> Result<Geometry> {
    let mut polys = Vec::new();
    for child in node.element_children() {
        if child.local_name() == "Polygon" {
            if let Geometry::Polygon { exterior, interiors } = parse_polygon(child)? {
                polys.push((exterior, interiors));
            }
        }
    }
    Ok(Geometry::MultiPolygon(polys))
}

fn parse_kml_coordinates(text: &str) -> Result<Vec<Coord>> {
    let mut coords = Vec::<Coord>::new();
    for tuple in text.split_whitespace() {
        let parts: Vec<&str> = tuple.split(',').collect();
        if parts.len() < 2 {
            return Err(GeoError::GmlParse {
                offset: 0,
                msg: format!("invalid KML coordinate tuple '{tuple}'"),
            });
        }
        let x = parts[0].trim().parse::<f64>().map_err(|_| GeoError::GmlParse {
            offset: 0,
            msg: format!("invalid longitude '{}'", parts[0].trim()),
        })?;
        let y = parts[1].trim().parse::<f64>().map_err(|_| GeoError::GmlParse {
            offset: 0,
            msg: format!("invalid latitude '{}'", parts[1].trim()),
        })?;

        let z = if parts.len() >= 3 {
            let z_text = parts[2].trim();
            if z_text.is_empty() {
                None
            } else {
                Some(z_text.parse::<f64>().map_err(|_| GeoError::GmlParse {
                    offset: 0,
                    msg: format!("invalid elevation '{z_text}'"),
                })?)
            }
        } else {
            None
        };

        coords.push(Coord { x, y, z, m: None });
    }

    Ok(coords)
}

fn close_ring_if_needed(coords: &mut Vec<Coord>) {
    if coords.len() < 3 {
        return;
    }
    if let (Some(first), Some(last)) = (coords.first().cloned(), coords.last()) {
        if &first != last {
            coords.push(first);
        }
    }
}

fn infer_field_type(raw: &str) -> FieldType {
    let s = raw.trim();
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
    if is_yyyy_mm_dd(s) {
        return FieldType::Date;
    }
    if is_iso_datetime(s) {
        return FieldType::DateTime;
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
                "true" | "1" | "yes" => FieldValue::Boolean(true),
                "false" | "0" | "no" => FieldValue::Boolean(false),
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
    matches!(s.to_ascii_lowercase().as_str(), "true" | "false" | "1" | "0" | "yes" | "no")
}

fn is_yyyy_mm_dd(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let b = s.as_bytes();
    b[4] == b'-'
        && b[7] == b'-'
        && b[0..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
}

fn is_iso_datetime(s: &str) -> bool {
    // Practical heuristic to avoid false positives while keeping parser simple.
    s.contains('T') && s.contains('-') && (s.ends_with('Z') || s.contains('+') || s.matches(':').count() >= 2)
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer → KML
// ══════════════════════════════════════════════════════════════════════════════

fn write_geom(out: &mut String, geom: &Geometry, indent: usize) {
    let pad = " ".repeat(indent);
    match geom {
        Geometry::Point(c) => {
            out.push_str(&format!("{pad}<Point><coordinates>"));
            push_coord_tuple(out, c);
            out.push_str("</coordinates></Point>\n");
        }
        Geometry::LineString(cs) => {
            out.push_str(&format!("{pad}<LineString><coordinates>"));
            push_coord_tuples(out, cs);
            out.push_str("</coordinates></LineString>\n");
        }
        Geometry::Polygon { exterior, interiors } => {
            out.push_str(&format!("{pad}<Polygon>\n"));
            out.push_str(&format!("{pad}  <outerBoundaryIs><LinearRing><coordinates>"));
            push_coord_tuples_closed(out, &exterior.0);
            out.push_str("</coordinates></LinearRing></outerBoundaryIs>\n");
            for ring in interiors {
                out.push_str(&format!("{pad}  <innerBoundaryIs><LinearRing><coordinates>"));
                push_coord_tuples_closed(out, &ring.0);
                out.push_str("</coordinates></LinearRing></innerBoundaryIs>\n");
            }
            out.push_str(&format!("{pad}</Polygon>\n"));
        }
        Geometry::MultiPoint(pts) => {
            out.push_str(&format!("{pad}<MultiGeometry>\n"));
            for p in pts {
                out.push_str(&format!("{pad}  <Point><coordinates>"));
                push_coord_tuple(out, p);
                out.push_str("</coordinates></Point>\n");
            }
            out.push_str(&format!("{pad}</MultiGeometry>\n"));
        }
        Geometry::MultiLineString(lines) => {
            out.push_str(&format!("{pad}<MultiGeometry>\n"));
            for line in lines {
                out.push_str(&format!("{pad}  <LineString><coordinates>"));
                push_coord_tuples(out, line);
                out.push_str("</coordinates></LineString>\n");
            }
            out.push_str(&format!("{pad}</MultiGeometry>\n"));
        }
        Geometry::MultiPolygon(polys) => {
            out.push_str(&format!("{pad}<MultiGeometry>\n"));
            for (exterior, holes) in polys {
                out.push_str(&format!("{pad}  <Polygon>\n"));
                out.push_str(&format!("{pad}    <outerBoundaryIs><LinearRing><coordinates>"));
                push_coord_tuples_closed(out, &exterior.0);
                out.push_str("</coordinates></LinearRing></outerBoundaryIs>\n");
                for ring in holes {
                    out.push_str(&format!("{pad}    <innerBoundaryIs><LinearRing><coordinates>"));
                    push_coord_tuples_closed(out, &ring.0);
                    out.push_str("</coordinates></LinearRing></innerBoundaryIs>\n");
                }
                out.push_str(&format!("{pad}  </Polygon>\n"));
            }
            out.push_str(&format!("{pad}</MultiGeometry>\n"));
        }
        Geometry::GeometryCollection(gs) => {
            out.push_str(&format!("{pad}<MultiGeometry>\n"));
            for g in gs {
                write_geom(out, g, indent + 2);
            }
            out.push_str(&format!("{pad}</MultiGeometry>\n"));
        }
    }
}

fn push_coord_tuple(out: &mut String, c: &Coord) {
    out.push_str(&c.x.to_string());
    out.push(',');
    out.push_str(&c.y.to_string());
    if let Some(z) = c.z {
        out.push(',');
        out.push_str(&z.to_string());
    }
}

fn push_coord_tuples(out: &mut String, coords: &[Coord]) {
    for (i, c) in coords.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        push_coord_tuple(out, c);
    }
}

fn push_coord_tuples_closed(out: &mut String, coords: &[Coord]) {
    push_coord_tuples(out, coords);
    if let (Some(first), Some(last)) = (coords.first(), coords.last()) {
        if first != last {
            out.push(' ');
            push_coord_tuple(out, first);
        }
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
    fn kml_roundtrip_basic() {
        let mut layer = Layer::new("cities")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.add_field(FieldDef::new("population", FieldType::Integer));
        layer
            .add_feature(
                Some(Geometry::point(-0.1278, 51.5074)),
                &[("name", "London".into()), ("population", 9_000_000i64.into())],
            )
            .unwrap();

        let kml = to_string(&layer);
        let parsed = parse_str(&kml).unwrap();

        assert_eq!(parsed.name, "cities");
        assert_eq!(parsed.crs_epsg(), Some(4326));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.schema.len(), 2);
        assert!(matches!(parsed.features[0].geometry, Some(Geometry::Point(_))));
        assert_eq!(parsed.features[0].get(&parsed.schema, "name").unwrap().as_str(), Some("London"));
    }

    #[test]
    fn parses_extended_data_and_multi_geometry() {
        let kml = r#"
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <name>roads</name>
    <Placemark>
      <name>Main</name>
      <ExtendedData>
        <Data name="speed"><value>80</value></Data>
        <Data name="toll"><value>false</value></Data>
      </ExtendedData>
      <MultiGeometry>
        <LineString><coordinates>-1,50 0,51</coordinates></LineString>
        <LineString><coordinates>0,51 1,52</coordinates></LineString>
      </MultiGeometry>
    </Placemark>
  </Document>
</kml>
"#;

        let layer = parse_str(kml).unwrap();
        assert_eq!(layer.name, "roads");
        assert_eq!(layer.crs_epsg(), Some(4326));
        assert_eq!(layer.len(), 1);
        assert!(matches!(layer.features[0].geometry, Some(Geometry::MultiLineString(_))));
        assert_eq!(layer.features[0].get(&layer.schema, "speed").unwrap().as_i64(), Some(80));
        assert_eq!(layer.features[0].get(&layer.schema, "toll").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn parses_polygon_holes_and_closes_rings() {
        let kml = r#"
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    <Placemark>
      <Polygon>
        <outerBoundaryIs><LinearRing><coordinates>0,0 10,0 10,10 0,10</coordinates></LinearRing></outerBoundaryIs>
        <innerBoundaryIs><LinearRing><coordinates>2,2 3,2 3,3 2,3</coordinates></LinearRing></innerBoundaryIs>
      </Polygon>
    </Placemark>
  </Document>
</kml>
"#;

        let layer = parse_str(kml).unwrap();
        match &layer.features[0].geometry {
            Some(Geometry::Polygon { exterior, interiors }) => {
                assert_eq!(exterior.0.first(), exterior.0.last());
                assert_eq!(interiors.len(), 1);
                assert_eq!(interiors[0].0.first(), interiors[0].0.last());
            }
            _ => panic!("expected polygon"),
        }
    }

    #[test]
    fn write_reprojects_projected_layer_to_epsg4326() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reprojected.kml");

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
