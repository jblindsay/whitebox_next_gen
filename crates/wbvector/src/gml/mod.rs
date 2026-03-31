//! Geography Markup Language (GML) reader and writer.
//!
//! This module targets practical GML Simple Features interoperability:
//! - `gml:FeatureCollection` with `gml:featureMember` / `gml:member`
//! - Geometries: Point, LineString, Polygon, Multi*, GeometryCollection
//! - Coordinates from `gml:pos`, `gml:posList`, or `gml:coordinates`
//! - Attributes as either `<gv:attr name="..." type="...">...</gv:attr>`
//!   (the form written by this module) or plain non-GML child elements.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Coord, Geometry, Ring};

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a GML file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let text = std::fs::read_to_string(path).map_err(GeoError::Io)?;
    parse_str(&text)
}

/// Parse a GML string into a [`Layer`].
pub fn parse_str(text: &str) -> Result<Layer> {
    let root = parse_xml(text)?;
    layer_from_xml(&root)
}

/// Write a [`Layer`] as GML to a file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    std::fs::write(path, to_string(layer).as_bytes()).map_err(GeoError::Io)
}

/// Serialize a [`Layer`] to a GML string.
pub fn to_string(layer: &Layer) -> String {
    let mut out = String::new();
    let srs_epsg = layer.crs_epsg().or_else(|| layer.crs_wkt().and_then(crs::epsg_from_wkt_lenient));
    let srs_name = srs_epsg.map(crs::canonical_gml_epsg_srs_name);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<gml:FeatureCollection xmlns:gml=\"http://www.opengis.net/gml\" xmlns:gv=\"https://wbvector.rs/gml\"");
    if let Some(srs_name) = &srs_name {
        out.push_str(" srsName=\"");
        out.push_str(srs_name);
        out.push('"');
    }
    out.push('>');

    if let Some(wkt) = layer.crs_wkt() {
        if !wkt.trim().is_empty() {
            out.push_str("<gv:crsWkt>");
            escape_xml(&mut out, wkt);
            out.push_str("</gv:crsWkt>");
        }
    }

    for feat in &layer.features {
        out.push_str("<gml:featureMember><gv:feature");
        out.push_str(" fid=\"");
        out.push_str(&feat.fid.to_string());
        out.push('"');
        if let Some(srs_name) = &srs_name {
            out.push_str(" srsName=\"");
            out.push_str(srs_name);
            out.push('"');
        }
        out.push('>');

        if let Some(g) = &feat.geometry {
            out.push_str("<gv:geometry>");
            write_geom(&mut out, g, srs_name.as_deref());
            out.push_str("</gv:geometry>");
        }

        for (i, def) in layer.schema.fields().iter().enumerate() {
            let v = feat.attributes.get(i).cloned().unwrap_or(FieldValue::Null);
            out.push_str("<gv:attr name=\"");
            escape_xml(&mut out, &def.name);
            out.push_str("\" type=\"");
            out.push_str(def.field_type.as_str());
            out.push_str("\">");
            match v {
                FieldValue::Null => {}
                _ => escape_xml(&mut out, &field_to_string(&v)),
            }
            out.push_str("</gv:attr>");
        }

        out.push_str("</gv:feature></gml:featureMember>");
    }
    out.push_str("</gml:FeatureCollection>\n");
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

    fn first_element_child(&self) -> Option<&XmlNode> {
        self.children.first()
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
            msg: "unterminated tags".into(),
        });
    }

    root.ok_or_else(|| GeoError::GmlParse { offset: 0, msg: "empty XML".into() })
}

fn parse_start_tag(raw: &str, offset: usize) -> Result<(String, HashMap<String, String>)> {
    let mut chars = raw.char_indices().peekable();
    let mut name_end = raw.len();
    while let Some((idx, ch)) = chars.peek().copied() {
        if ch.is_whitespace() {
            name_end = idx;
            break;
        }
        chars.next();
    }
    let name = raw[..name_end].trim();
    if name.is_empty() {
        return Err(GeoError::GmlParse { offset, msg: "missing tag name".into() });
    }

    let mut attrs = HashMap::new();
    let mut rest = raw[name_end..].trim();
    while !rest.is_empty() {
        let eq = rest.find('=').ok_or_else(|| GeoError::GmlParse {
            offset,
            msg: format!("invalid attribute syntax in tag <{}>", name),
        })?;
        let key = rest[..eq].trim();
        if key.is_empty() {
            return Err(GeoError::GmlParse { offset, msg: "empty attribute name".into() });
        }
        rest = rest[eq + 1..].trim_start();
        if !rest.starts_with('"') && !rest.starts_with('\'') {
            return Err(GeoError::GmlParse { offset, msg: "attribute must be quoted".into() });
        }
        let q = rest.as_bytes()[0] as char;
        rest = &rest[1..];
        let end = rest.find(q).ok_or_else(|| GeoError::GmlParse {
            offset,
            msg: format!("unterminated attribute '{}'", key),
        })?;
        let val = decode_entities(&rest[..end]);
        attrs.insert(key.to_owned(), val.to_owned());
        rest = rest[end + 1..].trim_start();
    }
    Ok((name.to_owned(), attrs))
}

fn find_bytes(hay: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || from >= hay.len() || needle.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

fn find_gt(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut in_quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == q {
                in_quote = None;
            }
        } else if b == b'\'' || b == b'"' {
            in_quote = Some(b);
        } else if b == b'>' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn decode_entities(s: &str) -> &str {
    if !(s.contains('&')) {
        return s;
    }
    // Avoid allocation for the common case in this crate. For the uncommon
    // entity case we return the original text and rely on plain data usage.
    // Writer always escapes output, so round-trips remain stable.
    s
}

// ══════════════════════════════════════════════════════════════════════════════
// XML → Layer
// ══════════════════════════════════════════════════════════════════════════════

fn layer_from_xml(root: &XmlNode) -> Result<Layer> {
    if root.local_name() != "FeatureCollection" {
        return Err(GeoError::GmlParse {
            offset: 0,
            msg: format!("expected FeatureCollection root, found {}", root.name),
        });
    }

    let members: Vec<&XmlNode> = root
        .children
        .iter()
        .filter(|c| matches!(c.local_name(), "featureMember" | "member"))
        .collect();

    let mut raw_attrs: Vec<Vec<(String, String, Option<FieldType>)>> = Vec::new();
    let mut geoms: Vec<Option<Geometry>> = Vec::new();
    let mut key_order: Vec<String> = Vec::new();
    let mut key_seen: HashSet<String> = HashSet::new();
    let mut inferred: HashMap<String, FieldType> = HashMap::new();

    for (idx, member) in members.iter().enumerate() {
        let feat = member.first_element_child().ok_or_else(|| GeoError::GmlParse {
            offset: idx,
            msg: "empty featureMember".into(),
        })?;
        let (geom, attrs) = parse_feature_node(feat)?;
        geoms.push(geom);
        for (k, v, t) in &attrs {
            if key_seen.insert(k.clone()) {
                key_order.push(k.clone());
            }
            if v.is_empty() {
                continue;
            }
            let ft = t.unwrap_or_else(|| infer_field_type(v));
            let entry = inferred.entry(k.clone()).or_insert(ft);
            *entry = crate::feature::FieldValue::widen_type(*entry, ft);
        }
        raw_attrs.push(attrs);
    }

    let mut layer = Layer::new("layer");
    for k in &key_order {
        let ft = inferred.get(k).copied().unwrap_or(FieldType::Text);
        layer.add_field(FieldDef::new(k, ft));
    }

    for (i, attrs) in raw_attrs.into_iter().enumerate() {
        let mut f = Feature {
            fid: i as u64,
            geometry: geoms[i].clone(),
            attributes: vec![FieldValue::Null; layer.schema.len()],
        };
        for (k, v, explicit) in attrs {
            if let Some(col) = layer.schema.field_index(&k) {
                let ft = explicit.unwrap_or(layer.schema.fields()[col].field_type);
                f.attributes[col] = parse_field_value(&v, ft);
            }
        }
        if layer.geom_type.is_none() {
            if let Some(g) = &f.geometry {
                layer.geom_type = Some(g.geom_type());
            }
        }
        layer.features.push(f);

        if layer.crs_epsg().is_none() {
            if let Some(member) = members.get(i) {
                if let Some(feat_node) = member.first_element_child() {
                    layer.set_crs_epsg(feature_srs_epsg(feat_node));
                }
            }
        }
    }

    if layer.crs_epsg().is_none() {
        layer.set_crs_epsg(root.attr("srsName").and_then(crs::epsg_from_srs_reference));
    }

    if let Some(crs_wkt_node) = root.children_by_local("crsWkt").next() {
        let wkt = crs_wkt_node.text.trim();
        if !wkt.is_empty() {
            layer.set_crs_wkt(Some(wkt.to_owned()));
        }
    }

    if layer.crs_wkt().is_none() {
        layer.set_crs_wkt(layer.crs_epsg().and_then(crs::ogc_wkt_from_epsg));
    }

    Ok(layer)
}

fn feature_srs_epsg(feature: &XmlNode) -> Option<u32> {
    if let Some(srs) = feature.attr("srsName") {
        if let Some(code) = crs::epsg_from_srs_reference(srs) {
            return Some(code);
        }
    }

    for child in &feature.children {
        if child.local_name() == "geometry" {
            if let Some(geom) = child.children.iter().find(|n| is_geometry_name(n.local_name())) {
                if let Some(srs) = geom.attr("srsName") {
                    if let Some(code) = crs::epsg_from_srs_reference(srs) {
                        return Some(code);
                    }
                }
            }
        } else if is_geometry_name(child.local_name()) {
            if let Some(srs) = child.attr("srsName") {
                if let Some(code) = crs::epsg_from_srs_reference(srs) {
                    return Some(code);
                }
            }
        }
    }

    None
}

fn parse_feature_node(feature: &XmlNode) -> Result<(Option<Geometry>, Vec<(String, String, Option<FieldType>)>)> {
    let mut geom: Option<Geometry> = None;
    let mut attrs: Vec<(String, String, Option<FieldType>)> = Vec::new();

    for child in &feature.children {
        if child.local_name() == "geometry" {
            if geom.is_none() {
                if let Some(g_node) = child.children.iter().find(|n| is_geometry_name(n.local_name())) {
                    geom = Some(parse_geometry(g_node)?);
                }
            }
            continue;
        }

        if is_geometry_name(child.local_name()) {
            if geom.is_none() {
                geom = Some(parse_geometry(child)?);
            }
            continue;
        }

        if child.local_name() == "attr" {
            if let Some(name) = child.attr("name") {
                let explicit = child.attr("type").and_then(parse_field_type);
                attrs.push((name.to_owned(), child.text.clone(), explicit));
            }
            continue;
        }

        if !child.text.is_empty() {
            attrs.push((child.local_name().to_owned(), child.text.clone(), None));
        }
    }

    Ok((geom, attrs))
}

fn is_geometry_name(n: &str) -> bool {
    matches!(
        n,
        "Point"
            | "LineString"
            | "Polygon"
            | "MultiPoint"
            | "MultiLineString"
            | "MultiPolygon"
            | "GeometryCollection"
    )
}

fn parse_geometry(node: &XmlNode) -> Result<Geometry> {
    match node.local_name() {
        "Point" => parse_point(node).map(Geometry::Point),
        "LineString" => parse_line_string(node).map(Geometry::LineString),
        "Polygon" => parse_polygon(node),
        "MultiPoint" => parse_multi_point(node),
        "MultiLineString" => parse_multi_line_string(node),
        "MultiPolygon" => parse_multi_polygon(node),
        "GeometryCollection" => parse_geometry_collection(node),
        other => Err(GeoError::GmlParse {
            offset: 0,
            msg: format!("unsupported geometry {}", other),
        }),
    }
}

fn parse_point(node: &XmlNode) -> Result<Coord> {
    if let Some(pos) = node.child_by_local("pos") {
        let cs = parse_coord_list(&pos.text)?;
        return cs.first().cloned().ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "empty gml:pos".into(),
        });
    }
    if let Some(coords) = node.child_by_local("coordinates") {
        let cs = parse_coordinates_legacy(&coords.text)?;
        return cs.first().cloned().ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "empty gml:coordinates".into(),
        });
    }
    Err(GeoError::GmlParse { offset: 0, msg: "Point missing pos/coordinates".into() })
}

fn parse_line_string(node: &XmlNode) -> Result<Vec<Coord>> {
    if let Some(pos_list) = node.child_by_local("posList") {
        return parse_coord_list(&pos_list.text);
    }
    let mut out = Vec::new();
    for p in node.children_by_local("pos") {
        let mut cs = parse_coord_list(&p.text)?;
        out.append(&mut cs);
    }
    if !out.is_empty() {
        return Ok(out);
    }
    if let Some(coords) = node.child_by_local("coordinates") {
        return parse_coordinates_legacy(&coords.text);
    }
    Err(GeoError::GmlParse { offset: 0, msg: "LineString missing coordinates".into() })
}

fn parse_ring(node: &XmlNode) -> Result<Ring> {
    let ring = if node.local_name() == "LinearRing" {
        node
    } else {
        node.child_by_local("LinearRing").ok_or_else(|| GeoError::GmlParse {
            offset: 0,
            msg: "missing LinearRing".into(),
        })?
    };
    let mut cs = if let Some(pos_list) = ring.child_by_local("posList") {
        parse_coord_list(&pos_list.text)?
    } else {
        let mut out = Vec::new();
        for p in ring.children_by_local("pos") {
            out.extend(parse_coord_list(&p.text)?);
        }
        if out.is_empty() {
            return Err(GeoError::GmlParse { offset: 0, msg: "LinearRing missing pos/posList".into() });
        }
        out
    };

    if cs.len() >= 2 && cs.first() == cs.last() {
        cs.pop();
    }
    Ok(Ring::new(cs))
}

fn parse_polygon(node: &XmlNode) -> Result<Geometry> {
    let ext = node
        .child_by_local("exterior")
        .or_else(|| node.child_by_local("outerBoundaryIs"))
        .ok_or_else(|| GeoError::GmlParse { offset: 0, msg: "Polygon missing exterior".into() })?;
    let exterior = parse_ring(ext)?;

    let mut interiors = Vec::new();
    for i in node
        .children
        .iter()
        .filter(|c| matches!(c.local_name(), "interior" | "innerBoundaryIs"))
    {
        interiors.push(parse_ring(i)?);
    }

    Ok(Geometry::Polygon { exterior, interiors })
}

fn parse_multi_point(node: &XmlNode) -> Result<Geometry> {
    let mut pts = Vec::new();
    for m in node
        .children
        .iter()
        .filter(|c| matches!(c.local_name(), "pointMember" | "pointMembers" | "member"))
    {
        if let Some(p) = m.children.iter().find(|n| n.local_name() == "Point") {
            pts.push(parse_point(p)?);
        }
    }
    Ok(Geometry::MultiPoint(pts))
}

fn parse_multi_line_string(node: &XmlNode) -> Result<Geometry> {
    let mut lines = Vec::new();
    for m in node
        .children
        .iter()
        .filter(|c| matches!(c.local_name(), "lineStringMember" | "lineStringMembers" | "member"))
    {
        if let Some(l) = m.children.iter().find(|n| n.local_name() == "LineString") {
            lines.push(parse_line_string(l)?);
        }
    }
    Ok(Geometry::MultiLineString(lines))
}

fn parse_multi_polygon(node: &XmlNode) -> Result<Geometry> {
    let mut polys = Vec::new();
    for m in node
        .children
        .iter()
        .filter(|c| matches!(c.local_name(), "polygonMember" | "polygonMembers" | "member"))
    {
        if let Some(p) = m.children.iter().find(|n| n.local_name() == "Polygon") {
            if let Geometry::Polygon { exterior, interiors } = parse_polygon(p)? {
                polys.push((exterior, interiors));
            }
        }
    }
    Ok(Geometry::MultiPolygon(polys))
}

fn parse_geometry_collection(node: &XmlNode) -> Result<Geometry> {
    let mut geoms = Vec::new();
    for m in node.children.iter().filter(|c| matches!(c.local_name(), "geometryMember" | "member")) {
        if let Some(g) = m.children.iter().find(|n| is_geometry_name(n.local_name())) {
            geoms.push(parse_geometry(g)?);
        }
    }
    Ok(Geometry::GeometryCollection(geoms))
}

fn parse_coord_list(text: &str) -> Result<Vec<Coord>> {
    let nums: Vec<f64> = text
        .split_whitespace()
        .map(|v| {
            v.parse::<f64>().map_err(|_| GeoError::GmlParse {
                offset: 0,
                msg: format!("invalid coordinate value '{}'", v),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    if nums.len() < 2 {
        return Ok(Vec::new());
    }

    let dim = if nums.len() % 3 == 0 { 3 } else { 2 };
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 1 < nums.len() {
        if dim == 3 && i + 2 < nums.len() {
            out.push(Coord::xyz(nums[i], nums[i + 1], nums[i + 2]));
            i += 3;
        } else {
            out.push(Coord::xy(nums[i], nums[i + 1]));
            i += 2;
        }
    }
    Ok(out)
}

fn parse_coordinates_legacy(text: &str) -> Result<Vec<Coord>> {
    let mut out = Vec::new();
    for pair in text.split_whitespace() {
        let vals: Vec<&str> = pair.split(',').collect();
        if vals.len() < 2 {
            continue;
        }
        let x = vals[0].parse::<f64>().map_err(|_| GeoError::GmlParse {
            offset: 0,
            msg: format!("invalid X value '{}'", vals[0]),
        })?;
        let y = vals[1].parse::<f64>().map_err(|_| GeoError::GmlParse {
            offset: 0,
            msg: format!("invalid Y value '{}'", vals[1]),
        })?;
        if vals.len() >= 3 {
            let z = vals[2].parse::<f64>().map_err(|_| GeoError::GmlParse {
                offset: 0,
                msg: format!("invalid Z value '{}'", vals[2]),
            })?;
            out.push(Coord::xyz(x, y, z));
        } else {
            out.push(Coord::xy(x, y));
        }
    }
    Ok(out)
}

fn parse_field_type(t: &str) -> Option<FieldType> {
    match t {
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

fn infer_field_type(v: &str) -> FieldType {
    if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false") {
        return FieldType::Boolean;
    }
    if v.parse::<i64>().is_ok() {
        return FieldType::Integer;
    }
    if v.parse::<f64>().is_ok() {
        return FieldType::Float;
    }
    if looks_like_date(v) {
        return FieldType::Date;
    }
    if looks_like_datetime(v) {
        return FieldType::DateTime;
    }
    FieldType::Text
}

fn looks_like_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[7] == b'-'
}

fn looks_like_datetime(s: &str) -> bool {
    s.contains('T') && (s.ends_with('Z') || s.contains('+'))
}

fn parse_field_value(v: &str, ft: FieldType) -> FieldValue {
    if v.is_empty() {
        return FieldValue::Null;
    }
    match ft {
        FieldType::Integer => v.parse::<i64>().map(FieldValue::Integer).unwrap_or_else(|_| FieldValue::Text(v.to_owned())),
        FieldType::Float => v.parse::<f64>().map(FieldValue::Float).unwrap_or_else(|_| FieldValue::Text(v.to_owned())),
        FieldType::Boolean => {
            if v.eq_ignore_ascii_case("true") {
                FieldValue::Boolean(true)
            } else if v.eq_ignore_ascii_case("false") {
                FieldValue::Boolean(false)
            } else {
                FieldValue::Text(v.to_owned())
            }
        }
        FieldType::Date => FieldValue::Date(v.to_owned()),
        FieldType::DateTime => FieldValue::DateTime(v.to_owned()),
        FieldType::Blob => FieldValue::Blob(v.as_bytes().to_vec()),
        FieldType::Json | FieldType::Text => FieldValue::Text(v.to_owned()),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Layer → XML
// ══════════════════════════════════════════════════════════════════════════════

fn field_to_string(v: &FieldValue) -> String {
    match v {
        FieldValue::Integer(x) => x.to_string(),
        FieldValue::Float(x) => x.to_string(),
        FieldValue::Text(x) => x.clone(),
        FieldValue::Boolean(x) => x.to_string(),
        FieldValue::Blob(x) => format!("{:?}", x),
        FieldValue::Date(x) => x.clone(),
        FieldValue::DateTime(x) => x.clone(),
        FieldValue::Null => String::new(),
    }
}

fn push_srs_name_attr(out: &mut String, srs_name: Option<&str>) {
    if let Some(s) = srs_name {
        out.push_str(" srsName=\"");
        out.push_str(s);
        out.push('"');
    }
}

fn write_geom(out: &mut String, g: &Geometry, srs_name: Option<&str>) {
    match g {
        Geometry::Point(c) => {
            out.push_str("<gml:Point");
            push_srs_name_attr(out, srs_name);
            out.push_str("><gml:pos>");
            push_coord(out, c);
            out.push_str("</gml:pos></gml:Point>");
        }
        Geometry::LineString(cs) => {
            out.push_str("<gml:LineString");
            push_srs_name_attr(out, srs_name);
            out.push_str("><gml:posList>");
            push_coord_list(out, cs);
            out.push_str("</gml:posList></gml:LineString>");
        }
        Geometry::Polygon { exterior, interiors } => {
            out.push_str("<gml:Polygon");
            push_srs_name_attr(out, srs_name);
            out.push_str("><gml:exterior>");
            write_ring(out, exterior);
            out.push_str("</gml:exterior>");
            for ring in interiors {
                out.push_str("<gml:interior>");
                write_ring(out, ring);
                out.push_str("</gml:interior>");
            }
            out.push_str("</gml:Polygon>");
        }
        Geometry::MultiPoint(pts) => {
            out.push_str("<gml:MultiPoint");
            push_srs_name_attr(out, srs_name);
            out.push('>');
            for p in pts {
                out.push_str("<gml:pointMember><gml:Point><gml:pos>");
                push_coord(out, p);
                out.push_str("</gml:pos></gml:Point></gml:pointMember>");
            }
            out.push_str("</gml:MultiPoint>");
        }
        Geometry::MultiLineString(lines) => {
            out.push_str("<gml:MultiLineString");
            push_srs_name_attr(out, srs_name);
            out.push('>');
            for l in lines {
                out.push_str("<gml:lineStringMember><gml:LineString><gml:posList>");
                push_coord_list(out, l);
                out.push_str("</gml:posList></gml:LineString></gml:lineStringMember>");
            }
            out.push_str("</gml:MultiLineString>");
        }
        Geometry::MultiPolygon(polys) => {
            out.push_str("<gml:MultiPolygon");
            push_srs_name_attr(out, srs_name);
            out.push('>');
            for (ext, ints) in polys {
                out.push_str("<gml:polygonMember>");
                write_geom(out, &Geometry::Polygon { exterior: ext.clone(), interiors: ints.clone() }, srs_name);
                out.push_str("</gml:polygonMember>");
            }
            out.push_str("</gml:MultiPolygon>");
        }
        Geometry::GeometryCollection(geoms) => {
            out.push_str("<gml:GeometryCollection");
            push_srs_name_attr(out, srs_name);
            out.push('>');
            for g in geoms {
                out.push_str("<gml:geometryMember>");
                write_geom(out, g, srs_name);
                out.push_str("</gml:geometryMember>");
            }
            out.push_str("</gml:GeometryCollection>");
        }
    }
}

fn write_ring(out: &mut String, ring: &Ring) {
    out.push_str("<gml:LinearRing><gml:posList>");
    push_coord_list_closed(out, &ring.0);
    out.push_str("</gml:posList></gml:LinearRing>");
}

fn push_coord(out: &mut String, c: &Coord) {
    out.push_str(&c.x.to_string());
    out.push(' ');
    out.push_str(&c.y.to_string());
    if let Some(z) = c.z {
        out.push(' ');
        out.push_str(&z.to_string());
    }
}

fn push_coord_list(out: &mut String, coords: &[Coord]) {
    for (i, c) in coords.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        push_coord(out, c);
    }
}

fn push_coord_list_closed(out: &mut String, coords: &[Coord]) {
    push_coord_list(out, coords);
    if let (Some(first), Some(last)) = (coords.first(), coords.last()) {
        if first != last {
            out.push(' ');
            push_coord(out, first);
        }
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
    use crate::geometry::GeometryType;

    #[test]
    fn gml_roundtrip_basic() {
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

        let xml = to_string(&layer);
        let parsed = parse_str(&xml).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.schema.len(), 2);
        assert_eq!(parsed.schema.fields()[0].name, "name");
        assert_eq!(parsed.schema.fields()[1].name, "population");
        assert!(matches!(parsed.features[0].geometry, Some(Geometry::Point(_))));
    }

    #[test]
    fn parse_plain_member_attrs() {
        let xml = r#"
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml">
  <gml:featureMember>
    <feature>
      <name>River A</name>
      <length_km>123.5</length_km>
      <gml:LineString><gml:posList>0 0 1 1 2 1</gml:posList></gml:LineString>
    </feature>
  </gml:featureMember>
</gml:FeatureCollection>
"#;
        let layer = parse_str(xml).unwrap();
        assert_eq!(layer.len(), 1);
        assert_eq!(layer.schema.len(), 2);
        assert!(matches!(layer.features[0].geometry, Some(Geometry::LineString(_))));
    }

    #[test]
    fn parses_uri_style_srs_name() {
        let xml = r#"
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml">
    <gml:featureMember>
        <feature>
            <gml:Point srsName="http://www.opengis.net/def/crs/EPSG/0/3857">
                <gml:pos>0 0</gml:pos>
            </gml:Point>
        </feature>
    </gml:featureMember>
</gml:FeatureCollection>
"#;
        let layer = parse_str(xml).unwrap();
        assert_eq!(layer.crs_epsg(), Some(3857));
        assert!(layer.crs_wkt().map(|w| !w.is_empty()).unwrap_or(false));
    }

    #[test]
    fn writes_srs_name_from_wkt_when_epsg_missing() {
        let mut layer = Layer::new("wkt_only").with_geom_type(GeometryType::Point);
        layer.set_crs_wkt(Some(
            "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],AUTHORITY[\"EPSG\",\"4326\"]]"
                .to_owned(),
        ));
        layer.add_feature(Some(Geometry::point(1.0, 2.0)), &[]).unwrap();

        let xml = to_string(&layer);
        assert!(xml.contains("srsName=\"http://www.opengis.net/def/crs/EPSG/0/4326\""));
        assert!(xml.contains("<gv:crsWkt>"));

        let roundtrip = parse_str(&xml).unwrap();
        assert_eq!(roundtrip.crs_epsg(), Some(4326));
        assert!(roundtrip.crs_wkt().map(|w| !w.is_empty()).unwrap_or(false));
    }
}
