//! FlatGeobuf (`.fgb`) reader and writer.
//!
//! This module writes and reads standards-compliant FlatGeobuf v3 datasets.
//!
//! File layout:
//!
//! ```text
//! [8 magic bytes]
//! [4-byte LE header size]
//! [header FlatBuffer]
//! [optional packed R-tree index]
//! [per-feature records: 4-byte LE size + feature FlatBuffer]
//! ```
//!
//! Magic: `\x66\x67\x62\x03\x66\x67\x62\x00`  ("fgb" + version 3 + "fgb" + NUL)
//!
//! The geometry payload uses FlatGeobuf geometry tables (`xy`, `ends`, `parts`).
//! Properties are encoded in standard FlatGeobuf property binary form.

use std::path::Path;
use flatbuffers::FlatBufferBuilder;
use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{BBox, Coord, Geometry, GeometryType, Ring};

#[allow(unused_imports, dead_code, non_snake_case, non_camel_case_types, clippy::all)]
mod header_generated;
#[allow(unused_imports, dead_code, non_snake_case, non_camel_case_types, clippy::all)]
mod feature_generated;

use self::feature_generated as fg;
use self::header_generated as hg;

// ── Magic ─────────────────────────────────────────────────────────────────────
/// FlatGeobuf v3 file signature (`fgb\x03fgb\0`).
pub const MAGIC: [u8; 8] = [0x66, 0x67, 0x62, 0x03, 0x66, 0x67, 0x62, 0x00];

// ── FlatGeobuf column-type codes ─────────────────────────────────────────────
mod ct {
    pub const BYTE:     u8 =  0;
    pub const UBYTE:    u8 =  1;
    pub const BOOL:     u8 =  2;
    pub const SHORT:    u8 =  3;
    pub const USHORT:   u8 =  4;
    pub const INT:      u8 =  5;
    pub const UINT:     u8 =  6;
    pub const LONG:     u8 =  7;
    pub const ULONG:    u8 =  8;
    pub const FLOAT:    u8 =  9;
    pub const DOUBLE:   u8 = 10;
    pub const STRING:   u8 = 11;
    pub const JSON:     u8 = 12;
    pub const DATETIME: u8 = 13;
    pub const BINARY:   u8 = 14;
}

// ── FlatGeobuf geometry-type codes ───────────────────────────────────────────
mod gt {
    #[allow(dead_code)]
    pub const UNKNOWN:            u8 = 0;
    pub const POINT:              u8 = 1;
    pub const LINESTRING:         u8 = 2;
    pub const POLYGON:            u8 = 3;
    pub const MULTIPOINT:         u8 = 4;
    pub const MULTILINESTRING:    u8 = 5;
    pub const MULTIPOLYGON:       u8 = 6;
    pub const GEOMETRYCOLLECTION: u8 = 7;
}

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a FlatGeobuf file into a [`Layer`].
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let data = std::fs::read(path).map_err(GeoError::Io)?;
    from_bytes(&data)
}

/// Parse FlatGeobuf from a byte slice.
pub fn from_bytes(data: &[u8]) -> Result<Layer> {
    if data.len() < 12 || &data[0..8] != MAGIC {
        return Err(GeoError::NotFlatGeobuf(
            format!("bad magic {:?}", &data[..8.min(data.len())])
        ));
    }
    let hdr_size  = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    if 12 + hdr_size > data.len() {
        return Err(GeoError::NotFlatGeobuf("header extends beyond EOF".into()));
    }
    let hdr_data = &data[12..12 + hdr_size];

    if let Ok(hdr) = hg::root_as_header(hdr_data) {
        if let Ok(layer) = from_bytes_standard(data, hdr_size, hdr) {
            return Ok(layer);
        }
    }

    from_bytes_legacy(data, hdr_size)
}

fn from_bytes_legacy(data: &[u8], hdr_size: usize) -> Result<Layer> {
    let hdr_data = &data[12..12 + hdr_size];
    let hdr = parse_header(hdr_data)?;

    let mut layer = Layer::new(if hdr.name.is_empty() { "layer" } else { &hdr.name });
    layer.geom_type = geom_type_from_code(hdr.geom_type);
    layer.set_crs_epsg(hdr.srs_epsg);
    layer.set_crs_wkt(layer.crs_epsg().and_then(crs::ogc_wkt_from_epsg));

    for col in &hdr.columns {
        let ft = col_type_to_field_type(col.col_type);
        layer.add_field(FieldDef::new(&col.name, ft).width(col.width as usize));
    }

    // Parse feature records
    let mut pos   = 12 + hdr_size;
    let mut fidx  = 0usize;

    while pos + 4 <= data.len() {
        let feat_size = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize;
        pos += 4;
        if feat_size == 0 || pos + feat_size > data.len() { break; }
        let feat_data = &data[pos..pos + feat_size];
        pos += feat_size;

        // Feature layout: [4 geom_size LE] [geom bytes] [props bytes]
        if feat_data.len() < 4 { fidx += 1; continue; }
        let geom_size = u32::from_le_bytes(feat_data[0..4].try_into().unwrap()) as usize;
        if 4 + geom_size > feat_data.len() { fidx += 1; continue; }

        let geom_bytes  = &feat_data[4..4 + geom_size];
        let props_bytes = &feat_data[4 + geom_size..];

        let geom = if geom_bytes.is_empty() { None }
                   else { decode_geom(geom_bytes, hdr.geom_type, hdr.has_z).ok() };

        let attrs = decode_props(props_bytes, &hdr.columns);

        layer.push(Feature { fid: fidx as u64, geometry: geom, attributes: attrs });
        fidx += 1;
    }

    Ok(layer)
}

fn from_bytes_standard(data: &[u8], hdr_size: usize, hdr: hg::Header<'_>) -> Result<Layer> {
    let mut layer = Layer::new(hdr.name().unwrap_or("layer"));
    layer.geom_type = geom_type_from_code(hdr.geometry_type().0);
    if let Some(fg_crs) = hdr.crs() {
        let code = fg_crs.code();
        if code > 0 {
            layer.set_crs_epsg(Some(code as u32));
        }
        if layer.crs_epsg().is_none() {
            if let Some(code_string) = fg_crs.code_string() {
                layer.set_crs_epsg(crs::epsg_from_srs_reference(code_string));
            }
        }
        if layer.crs_epsg().is_none() {
            if let Some(name) = fg_crs.name() {
                layer.set_crs_epsg(crs::epsg_from_srs_reference(name));
            }
        }
        if let Some(wkt) = fg_crs.wkt() {
            let trimmed = wkt.trim();
            if !trimmed.is_empty() {
                layer.set_crs_wkt(Some(trimmed.to_owned()));
            }
        }
    }
    if layer.crs_epsg().is_none() {
        layer.set_crs_epsg(layer.crs_wkt().and_then(crs::epsg_from_wkt_lenient));
    }
    if layer.crs_wkt().is_none() {
        layer.set_crs_wkt(layer.crs_epsg().and_then(crs::ogc_wkt_from_epsg));
    }

    let mut columns: Vec<FgbColumn> = Vec::new();
    if let Some(cols) = hdr.columns() {
        for i in 0..cols.len() {
            let c = cols.get(i);
            let name = c.name().to_string();
            let col_type = c.type_().0;
            columns.push(FgbColumn {
                name: name.clone(),
                col_type,
                _nullable: c.nullable(),
                width: c.width(),
            });
            let ft = col_type_to_field_type(col_type);
            let mut fd = FieldDef::new(name, ft).width(c.width().max(0) as usize);
            fd.nullable = c.nullable();
            if c.precision() > 0 {
                fd.precision = c.precision() as usize;
            }
            layer.add_field(fd);
        }
    }

    let mut pos = 12 + hdr_size;
    if hdr.index_node_size() > 0 && hdr.features_count() > 0 {
        let index_size = packed_index_size(hdr.features_count() as usize, hdr.index_node_size());
        pos = pos.saturating_add(index_size);
        if pos > data.len() {
            return Err(GeoError::NotFlatGeobuf("index extends beyond EOF".into()));
        }
    }

    let mut fidx = 0usize;
    while pos + 4 <= data.len() {
        let feat_size = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if feat_size == 0 || pos + feat_size > data.len() {
            break;
        }
        let feat_buf = &data[pos..pos + feat_size];
        pos += feat_size;

        let feat = fg::root_as_feature(feat_buf)
            .map_err(|e| GeoError::NotFlatGeobuf(format!("invalid feature buffer: {e}")))?;

        let geometry = feat.geometry().map(decode_geom_standard).transpose()?;
        let attrs = if let Some(props) = feat.properties() {
            let mut p = Vec::with_capacity(props.len());
            for i in 0..props.len() { p.push(props.get(i)); }
            decode_props(&p, &columns)
        } else {
            vec![FieldValue::Null; columns.len()]
        };

        layer.push(Feature { fid: fidx as u64, geometry, attributes: attrs });
        fidx += 1;
    }

    Ok(layer)
}

/// Write a [`Layer`] as a FlatGeobuf file.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    std::fs::write(path, to_bytes(layer)).map_err(GeoError::Io)
}

/// Serialise a [`Layer`] as FlatGeobuf bytes.
pub fn to_bytes(layer: &Layer) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);

    let hdr = build_standard_header(layer);
    out.extend_from_slice(&(hdr.len() as u32).to_le_bytes());
    out.extend_from_slice(&hdr);

    for feat in &layer.features {
        let feat_buf = build_standard_feature(feat, &layer.schema);
        out.extend_from_slice(&(feat_buf.len() as u32).to_le_bytes());
        out.extend_from_slice(&feat_buf);
    }

    out
}

fn build_standard_header(layer: &Layer) -> Vec<u8> {
    let mut fbb = FlatBufferBuilder::new();

    let mut cols = Vec::new();
    for fd in layer.schema.fields() {
        let name = fbb.create_string(&fd.name);
        let args = hg::ColumnArgs {
            name: Some(name),
            type_: std_col_type(field_type_to_col_type(fd.field_type)),
            width: if fd.width == 0 { -1 } else { fd.width as i32 },
            precision: if fd.precision == 0 { -1 } else { fd.precision as i32 },
            nullable: fd.nullable,
            ..Default::default()
        };
        cols.push(hg::Column::create(&mut fbb, &args));
    }
    let cols_off = if cols.is_empty() { None } else { Some(fbb.create_vector(&cols)) };

    let has_z = layer.features.iter().any(|f| f.geometry.as_ref().map_or(false, |g| g.has_z()));
    let name = fbb.create_string(&layer.name);
    let envelope = layer_bbox(layer).map(|bb| fbb.create_vector(&[bb.min_x, bb.min_y, bb.max_x, bb.max_y]));
    let epsg = layer.crs_epsg().or_else(|| layer.crs_wkt().and_then(crs::epsg_from_wkt_lenient));
    let srs_wkt = layer.crs_wkt().map(|w| w.to_owned()).or_else(|| epsg.and_then(crs::ogc_wkt_from_epsg));
    let crs_name = epsg.and_then(crs::crs_name_from_epsg);

    let crs = if epsg.is_none() && srs_wkt.is_none() {
        None
    } else {
        let org = epsg.map(|_| fbb.create_string("EPSG"));
        let code_string = epsg.map(|e| fbb.create_string(&crs::canonical_epsg_srs_name(e)));
        let name_text = crs_name.map(|v| fbb.create_string(&v));
        let wkt_text = srs_wkt.map(|v| fbb.create_string(&v));
        Some(hg::Crs::create(&mut fbb, &hg::CrsArgs {
            org,
            code: epsg.map(|e| e as i32).unwrap_or(0),
            name: name_text,
            description: None,
            wkt: wkt_text,
            code_string,
        }))
    };

    let args = hg::HeaderArgs {
        name: Some(name),
        envelope,
        geometry_type: std_geom_type(layer.geom_type.unwrap_or(GeometryType::GeometryCollection)),
        has_z,
        columns: cols_off,
        features_count: layer.features.len() as u64,
        index_node_size: 0,
        crs,
        ..Default::default()
    };
    let root = hg::Header::create(&mut fbb, &args);
    fbb.finish(root, None);
    fbb.finished_data().to_vec()
}

fn build_standard_feature(feat: &Feature, schema: &crate::feature::Schema) -> Vec<u8> {
    let mut fbb = FlatBufferBuilder::new();

    let geom = feat.geometry.as_ref().map(|g| encode_geom_standard(&mut fbb, g));
    let props = encode_props(feat, schema);
    let props_off = if props.is_empty() { None } else { Some(fbb.create_vector(&props)) };

    let args = fg::FeatureArgs {
        geometry: geom,
        properties: props_off,
        ..Default::default()
    };
    let root = fg::Feature::create(&mut fbb, &args);
    fbb.finish(root, None);
    fbb.finished_data().to_vec()
}

fn encode_geom_standard<'a>(fbb: &mut FlatBufferBuilder<'a>, geom: &Geometry) -> flatbuffers::WIPOffset<fg::Geometry<'a>> {
    match geom {
        Geometry::Point(c) => {
            let xy = fbb.create_vector(&[c.x, c.y]);
            let z = c.z.map(|v| fbb.create_vector(&[v]));
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::Point),
                xy: Some(xy),
                z,
                ..Default::default()
            })
        }
        Geometry::LineString(cs) => {
            let (xy, z) = flatten_coords(cs);
            let xy = fbb.create_vector(&xy);
            let z = z.map(|vals| fbb.create_vector(&vals));
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::LineString),
                xy: Some(xy),
                z,
                ..Default::default()
            })
        }
        Geometry::Polygon { exterior, interiors } => {
            let mut coords = Vec::new();
            let mut ends = Vec::new();
            append_ring(&mut coords, &mut ends, exterior);
            for r in interiors { append_ring(&mut coords, &mut ends, r); }
            let (xy, z) = flatten_coords(&coords);
            let xy = fbb.create_vector(&xy);
            let ends = fbb.create_vector(&ends);
            let z = z.map(|vals| fbb.create_vector(&vals));
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::Polygon),
                xy: Some(xy),
                z,
                ends: Some(ends),
                ..Default::default()
            })
        }
        Geometry::MultiPoint(cs) => {
            let (xy, z) = flatten_coords(cs);
            let xy = fbb.create_vector(&xy);
            let z = z.map(|vals| fbb.create_vector(&vals));
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::MultiPoint),
                xy: Some(xy),
                z,
                ..Default::default()
            })
        }
        Geometry::MultiLineString(lines) => {
            let mut coords = Vec::new();
            let mut ends = Vec::new();
            for l in lines {
                coords.extend_from_slice(l);
                ends.push(coords.len() as u32);
            }
            let (xy, z) = flatten_coords(&coords);
            let xy = fbb.create_vector(&xy);
            let ends = fbb.create_vector(&ends);
            let z = z.map(|vals| fbb.create_vector(&vals));
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::MultiLineString),
                xy: Some(xy),
                z,
                ends: Some(ends),
                ..Default::default()
            })
        }
        Geometry::MultiPolygon(polys) => {
            let mut parts = Vec::new();
            for (ext, holes) in polys {
                let pg = Geometry::Polygon { exterior: ext.clone(), interiors: holes.clone() };
                parts.push(encode_geom_standard(fbb, &pg));
            }
            let parts = fbb.create_vector(&parts);
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::MultiPolygon),
                parts: Some(parts),
                ..Default::default()
            })
        }
        Geometry::GeometryCollection(gs) => {
            let mut parts = Vec::new();
            for g in gs { parts.push(encode_geom_standard(fbb, g)); }
            let parts = fbb.create_vector(&parts);
            fg::Geometry::create(fbb, &fg::GeometryArgs {
                type_: std_geom_type(GeometryType::GeometryCollection),
                parts: Some(parts),
                ..Default::default()
            })
        }
    }
}

fn decode_geom_standard(geom: fg::Geometry<'_>) -> Result<Geometry> {
    let gtype = geom.type_().0;

    if let Some(parts) = geom.parts() {
        match gtype {
            gt::MULTIPOLYGON => {
                let mut polys = Vec::new();
                for i in 0..parts.len() {
                    let p = decode_geom_standard(parts.get(i))?;
                    match p {
                        Geometry::Polygon { exterior, interiors } => polys.push((exterior, interiors)),
                        Geometry::MultiPolygon(mut more) => polys.append(&mut more),
                        _ => {}
                    }
                }
                return Ok(Geometry::MultiPolygon(polys));
            }
            gt::GEOMETRYCOLLECTION => {
                let mut gs = Vec::new();
                for i in 0..parts.len() { gs.push(decode_geom_standard(parts.get(i))?); }
                return Ok(Geometry::GeometryCollection(gs));
            }
            _ => {}
        }
    }

    let xy = geom.xy().ok_or_else(|| GeoError::NotFlatGeobuf("geometry missing xy".into()))?;
    if xy.len() % 2 != 0 {
        return Err(GeoError::NotFlatGeobuf("invalid xy vector length".into()));
    }
    let z = geom.z();
    let n = xy.len() / 2;
    let mut coords = Vec::with_capacity(n);
    for i in 0..n {
        let x = xy.get(i * 2);
        let y = xy.get(i * 2 + 1);
        let zv = z.and_then(|zz| if i < zz.len() { Some(zz.get(i)) } else { None });
        coords.push(Coord { x, y, z: zv, m: None });
    }

    let ends: Vec<usize> = geom
        .ends()
        .map(|e| (0..e.len()).map(|i| e.get(i) as usize).collect())
        .unwrap_or_default();

    build_geometry(gtype, &coords, &if ends.is_empty() { vec![coords.len()] } else { ends })
}

fn flatten_coords(coords: &[Coord]) -> (Vec<f64>, Option<Vec<f64>>) {
    let mut xy = Vec::with_capacity(coords.len() * 2);
    let has_z = coords.iter().any(|c| c.z.is_some());
    let mut z = if has_z { Some(Vec::with_capacity(coords.len())) } else { None };
    for c in coords {
        xy.push(c.x);
        xy.push(c.y);
        if let Some(ref mut zv) = z {
            zv.push(c.z.unwrap_or(0.0));
        }
    }
    (xy, z)
}

fn append_ring(coords: &mut Vec<Coord>, ends: &mut Vec<u32>, ring: &Ring) {
    coords.extend_from_slice(&ring.0);
    if ring.0.len() > 1 && ring.0.first() != ring.0.last() {
        coords.push(ring.0[0].clone());
    }
    ends.push(coords.len() as u32);
}

fn layer_bbox(layer: &Layer) -> Option<BBox> {
    let mut bb: Option<BBox> = None;
    for f in &layer.features {
        if let Some(g) = &f.geometry {
            if let Some(gb) = g.bbox() {
                bb = Some(match bb {
                    None => gb,
                    Some(mut e) => {
                        e.expand_to(&gb);
                        e
                    }
                });
            }
        }
    }
    bb
}

fn packed_index_size(num_items: usize, node_size: u16) -> usize {
    if node_size < 2 || num_items == 0 {
        return 0;
    }
    let node_size_min = node_size.clamp(2, 65535) as usize;
    let mut n = num_items;
    let mut num_nodes = n;
    loop {
        n = n.div_ceil(node_size_min);
        num_nodes += n;
        if n == 1 { break; }
    }
    num_nodes * std::mem::size_of::<(f64, f64, f64, f64, u64)>()
}

// ══════════════════════════════════════════════════════════════════════════════
// Header
// ══════════════════════════════════════════════════════════════════════════════

struct FgbHeader {
    name:      String,
    geom_type: u8,
    has_z:     bool,
    srs_epsg:  Option<u32>,
    columns:   Vec<FgbColumn>,
}

#[derive(Debug, Clone)]
struct FgbColumn {
    name:     String,
    col_type: u8,
    _nullable: bool,
    width:    i32,
}

/// Parse the simplified binary header written by [`build_header`].
///
/// Layout:
/// ```text
/// [1]  geometry type
/// [1]  has_z flag
/// [1]  has_m flag (ignored)
/// [4]  srs_epsg (0 = absent)
/// [2]  num_columns
/// for each column:
///   [1]  name length
///   [N]  name bytes
///   [1]  column type code
///   [1]  nullable flag
///   [4]  width (i32 LE)
/// [4]  feature_count (u32 LE, informational)
/// ```
fn parse_header(data: &[u8]) -> Result<FgbHeader> {
    if data.len() < 9 {
        return Err(GeoError::NotFlatGeobuf("header too short".into()));
    }
    let geom_type = data[0];
    let has_z     = data[1] != 0;
    // data[2] = has_m (ignored)
    let srs_epsg_raw = u32::from_le_bytes(data[3..7].try_into().unwrap());
    let srs_epsg  = if srs_epsg_raw == 0 { None } else { Some(srs_epsg_raw) };
    let num_cols  = u16::from_le_bytes(data[7..9].try_into().unwrap()) as usize;

    let mut pos = 9usize;
    let mut columns = Vec::with_capacity(num_cols);

    for _ in 0..num_cols {
        if pos >= data.len() { break; }
        let name_len = data[pos] as usize; pos += 1;
        if pos + name_len + 6 > data.len() { break; }
        let name = String::from_utf8_lossy(&data[pos..pos+name_len]).to_string(); pos += name_len;
        let col_type = data[pos]; pos += 1;
        let nullable = data[pos] != 0; pos += 1;
        let width    = i32::from_le_bytes(data[pos..pos+4].try_into().unwrap()); pos += 4;
        columns.push(FgbColumn { name, col_type, _nullable: nullable, width });
    }

    // Skip feature_count (4 bytes, informational)
    Ok(FgbHeader { name: String::new(), geom_type, has_z, srs_epsg, columns })
}

#[allow(dead_code)]
fn build_header(layer: &Layer) -> Vec<u8> {
    let geom_type = layer.geom_type.map(geom_type_code).unwrap_or(gt::UNKNOWN);
    let has_z: u8 = if layer.features.iter().any(|f| f.geometry.as_ref().map_or(false, |g| g.has_z())) { 1 } else { 0 };
    let srs_epsg: u32 = layer.crs_epsg().unwrap_or(0);

    let mut buf = Vec::new();
    buf.push(geom_type);
    buf.push(has_z);
    buf.push(0u8); // has_m
    buf.extend_from_slice(&srs_epsg.to_le_bytes());
    buf.extend_from_slice(&(layer.schema.len() as u16).to_le_bytes());

    for fd in layer.schema.fields() {
        let col_type = field_type_to_col_type(fd.field_type);
        let name_b   = fd.name.as_bytes();
        buf.push(name_b.len() as u8);
        buf.extend_from_slice(name_b);
        buf.push(col_type);
        buf.push(if fd.nullable { 1 } else { 0 });
        buf.extend_from_slice(&(fd.width as i32).to_le_bytes());
    }

    buf.extend_from_slice(&(layer.features.len() as u32).to_le_bytes());
    buf
}

// ══════════════════════════════════════════════════════════════════════════════
// Geometry codec
// ══════════════════════════════════════════════════════════════════════════════
//
// Geometry bytes layout:
//   [1]  geometry type override (may differ from header for mixed collections)
//   [1]  has_z flag
//   [4]  n_pts (u32 LE)
//   [n_pts * stride * 8] coordinates (x0,y0[,z0],x1,y1[,z1],...)
//   [4]  n_ends (u32 LE)
//   [n_ends * 4]  ends (u32 LE) — cumulative point counts per part/ring

#[allow(dead_code)]
fn encode_geom(geom: &Geometry) -> Vec<u8> {
    let gt   = geom_type_code(geom.geom_type());
    let has_z = geom.has_z();

    let mut coords: Vec<Coord> = Vec::new();
    let mut ends:   Vec<u32>   = Vec::new();

    collect_coords(geom, &mut coords, &mut ends);

    let mut buf = Vec::new();
    buf.push(gt);
    buf.push(has_z as u8);
    buf.extend_from_slice(&(coords.len() as u32).to_le_bytes());
    for c in &coords {
        buf.extend_from_slice(&c.x.to_le_bytes());
        buf.extend_from_slice(&c.y.to_le_bytes());
        if has_z { buf.extend_from_slice(&c.z.unwrap_or(0.0).to_le_bytes()); }
    }
    buf.extend_from_slice(&(ends.len() as u32).to_le_bytes());
    for e in &ends { buf.extend_from_slice(&e.to_le_bytes()); }
    buf
}

#[allow(dead_code)]
fn collect_coords(geom: &Geometry, coords: &mut Vec<Coord>, ends: &mut Vec<u32>) {
    match geom {
        Geometry::Point(c) => coords.push(c.clone()),
        Geometry::LineString(cs) => {
            coords.extend_from_slice(cs);
            ends.push(coords.len() as u32);
        }
        Geometry::Polygon { exterior, interiors } => {
            push_closed_ring(coords, exterior);
            ends.push(coords.len() as u32);
            for r in interiors {
                push_closed_ring(coords, r);
                ends.push(coords.len() as u32);
            }
        }
        Geometry::MultiPoint(cs) => {
            coords.extend_from_slice(cs);
        }
        Geometry::MultiLineString(ls) => {
            for l in ls {
                coords.extend_from_slice(l);
                ends.push(coords.len() as u32);
            }
        }
        Geometry::MultiPolygon(ps) => {
            for (ext, holes) in ps {
                push_closed_ring(coords, ext);
                ends.push(coords.len() as u32);
                for h in holes {
                    push_closed_ring(coords, h);
                    ends.push(coords.len() as u32);
                }
            }
        }
        Geometry::GeometryCollection(_) => {} // skip nested for simplicity
    }
}

#[allow(dead_code)]
fn push_closed_ring(coords: &mut Vec<Coord>, ring: &Ring) {
    coords.extend_from_slice(&ring.0);
    if ring.0.len() > 1 { coords.push(ring.0[0].clone()); }
}

fn decode_geom(data: &[u8], _header_gt: u8, header_has_z: bool) -> Result<Geometry> {
    if data.len() < 6 {
        return Err(GeoError::InvalidFgbFeature { index: 0, msg: "geom data too short".into() });
    }
    let geom_type = data[0];
    let has_z     = data[1] != 0 || header_has_z;
    let stride    = if has_z { 3usize } else { 2 };

    let n_pts = u32::from_le_bytes(data[2..6].try_into().unwrap()) as usize;
    let coord_bytes = n_pts * stride * 8;
    let min_len = 6 + coord_bytes + 4;

    if data.len() < min_len {
        return Err(GeoError::InvalidFgbFeature { index: 0, msg: "geom data truncated".into() });
    }

    let mut coords = Vec::with_capacity(n_pts);
    for i in 0..n_pts {
        let off = 6 + i * stride * 8;
        let x = f64::from_le_bytes(data[off..off+8].try_into().unwrap());
        let y = f64::from_le_bytes(data[off+8..off+16].try_into().unwrap());
        let z = if has_z { Some(f64::from_le_bytes(data[off+16..off+24].try_into().unwrap())) } else { None };
        coords.push(Coord { x, y, z, m: None });
    }

    let ends_off  = 6 + coord_bytes;
    let n_ends    = u32::from_le_bytes(data[ends_off..ends_off+4].try_into().unwrap()) as usize;
    let ends: Vec<usize> = (0..n_ends).map(|i| {
        let off = ends_off + 4 + i * 4;
        u32::from_le_bytes(data[off..off+4].try_into().unwrap()) as usize
    }).collect();

    // default ends = whole coordinate array as one part
    let effective_ends: Vec<usize> = if ends.is_empty() { vec![coords.len()] } else { ends };

    build_geometry(geom_type, &coords, &effective_ends)
}

fn build_geometry(geom_type: u8, coords: &[Coord], ends: &[usize]) -> Result<Geometry> {
    match geom_type {
        gt::POINT => {
            let c = coords.first().cloned().unwrap_or(Coord::xy(0.0, 0.0));
            Ok(Geometry::Point(c))
        }
        gt::LINESTRING => Ok(Geometry::LineString(coords.to_vec())),
        gt::POLYGON => {
            let rings = ends_to_rings(coords, ends);
            let mut it = rings.into_iter();
            let ext  = it.next().unwrap_or_default();
            Ok(Geometry::Polygon { exterior: ext, interiors: it.collect() })
        }
        gt::MULTIPOINT => Ok(Geometry::MultiPoint(coords.to_vec())),
        gt::MULTILINESTRING => {
            let parts = ends_to_parts(coords, ends);
            Ok(Geometry::MultiLineString(parts))
        }
        gt::MULTIPOLYGON => {
            // Each polygon's rings are split by ends; we rebuild from the flat list.
            // Simplified: treat each ring as its own polygon (no hole association).
            let rings = ends_to_rings(coords, ends);
            let polys = rings.into_iter().map(|r| (r, vec![])).collect();
            Ok(Geometry::MultiPolygon(polys))
        }
        gt::GEOMETRYCOLLECTION => Ok(Geometry::GeometryCollection(vec![])),
        other => Err(GeoError::NotImplemented(format!("FGB geom type {other}"))),
    }
}

fn ends_to_rings(coords: &[Coord], ends: &[usize]) -> Vec<Ring> {
    let mut rings = Vec::new();
    let mut start = 0;
    for &end in ends {
        let end = end.min(coords.len());
        if end > start {
            let mut part = coords[start..end].to_vec();
            // drop closing point
            if part.len() > 1 && part.first().map(|c|(c.x,c.y)) == part.last().map(|c|(c.x,c.y)) {
                part.pop();
            }
            rings.push(Ring::new(part));
        }
        start = end;
    }
    rings
}

fn ends_to_parts(coords: &[Coord], ends: &[usize]) -> Vec<Vec<Coord>> {
    let mut parts = Vec::new();
    let mut start = 0;
    for &end in ends {
        let end = end.min(coords.len());
        if end > start { parts.push(coords[start..end].to_vec()); }
        start = end;
    }
    parts
}

// ══════════════════════════════════════════════════════════════════════════════
// Property codec
// ══════════════════════════════════════════════════════════════════════════════
//
// Binary property layout: repeated [col_idx: u16 LE] [value bytes]
// Value size is determined by the column type code.

fn encode_props(feat: &Feature, schema: &crate::feature::Schema) -> Vec<u8> {
    let mut buf = Vec::new();
    for (i, _fd) in schema.fields().iter().enumerate() {
        let val = feat.attributes.get(i).unwrap_or(&FieldValue::Null);
        if val.is_null() { continue; }
        buf.extend_from_slice(&(i as u16).to_le_bytes());
        match val {
            FieldValue::Boolean(v)  => buf.push(*v as u8),
            FieldValue::Integer(v)  => buf.extend_from_slice(&v.to_le_bytes()),
            FieldValue::Float(v)    => buf.extend_from_slice(&v.to_le_bytes()),
            FieldValue::Text(s) | FieldValue::Date(s) | FieldValue::DateTime(s) => {
                buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
                buf.extend_from_slice(s.as_bytes());
            }
            FieldValue::Blob(b)     => {
                buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
                buf.extend_from_slice(b);
            }
            FieldValue::Null => {}
        }
    }
    buf
}

fn decode_props(data: &[u8], columns: &[FgbColumn]) -> Vec<FieldValue> {
    let mut vals = vec![FieldValue::Null; columns.len()];
    let mut pos  = 0;

    while pos + 2 <= data.len() {
        let col_idx = u16::from_le_bytes(data[pos..pos+2].try_into().unwrap()) as usize;
        pos += 2;
        if col_idx >= columns.len() { break; }
        let col = &columns[col_idx];

        let (val, consumed) = match col.col_type {
            ct::BOOL => {
                if pos >= data.len() { break; }
                (FieldValue::Boolean(data[pos] != 0), 1)
            }
            ct::BYTE | ct::UBYTE => {
                if pos >= data.len() { break; }
                (FieldValue::Integer(data[pos] as i64), 1)
            }
            ct::SHORT | ct::USHORT => {
                if pos + 2 > data.len() { break; }
                (FieldValue::Integer(i16::from_le_bytes(data[pos..pos+2].try_into().unwrap()) as i64), 2)
            }
            ct::INT | ct::UINT => {
                if pos + 4 > data.len() { break; }
                (FieldValue::Integer(i32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as i64), 4)
            }
            ct::LONG | ct::ULONG => {
                if pos + 8 > data.len() { break; }
                (FieldValue::Integer(i64::from_le_bytes(data[pos..pos+8].try_into().unwrap())), 8)
            }
            ct::FLOAT => {
                if pos + 4 > data.len() { break; }
                (FieldValue::Float(f32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as f64), 4)
            }
            ct::DOUBLE => {
                if pos + 8 > data.len() { break; }
                (FieldValue::Float(f64::from_le_bytes(data[pos..pos+8].try_into().unwrap())), 8)
            }
            ct::STRING | ct::JSON => {
                if pos + 4 > data.len() { break; }
                let len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize; pos += 4;
                if pos + len > data.len() { break; }
                let s = String::from_utf8_lossy(&data[pos..pos+len]).to_string();
                (FieldValue::Text(s), len)
            }
            ct::DATETIME => {
                if pos + 4 > data.len() { break; }
                let len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize; pos += 4;
                if pos + len > data.len() { break; }
                let s = String::from_utf8_lossy(&data[pos..pos+len]).to_string();
                (FieldValue::DateTime(s), len)
            }
            ct::BINARY => {
                if pos + 4 > data.len() { break; }
                let len = u32::from_le_bytes(data[pos..pos+4].try_into().unwrap()) as usize; pos += 4;
                if pos + len > data.len() { break; }
                (FieldValue::Blob(data[pos..pos+len].to_vec()), len)
            }
            _ => break,
        };
        vals[col_idx] = val;
        pos += consumed;
    }
    vals
}

// ══════════════════════════════════════════════════════════════════════════════
// Type conversion helpers
// ══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn geom_type_code(gt: GeometryType) -> u8 {
    match gt {
        GeometryType::Point              => gt::POINT,
        GeometryType::LineString         => gt::LINESTRING,
        GeometryType::Polygon            => gt::POLYGON,
        GeometryType::MultiPoint         => gt::MULTIPOINT,
        GeometryType::MultiLineString    => gt::MULTILINESTRING,
        GeometryType::MultiPolygon       => gt::MULTIPOLYGON,
        GeometryType::GeometryCollection => gt::GEOMETRYCOLLECTION,
    }
}

fn geom_type_from_code(code: u8) -> Option<GeometryType> {
    match code {
        gt::POINT              => Some(GeometryType::Point),
        gt::LINESTRING         => Some(GeometryType::LineString),
        gt::POLYGON            => Some(GeometryType::Polygon),
        gt::MULTIPOINT         => Some(GeometryType::MultiPoint),
        gt::MULTILINESTRING    => Some(GeometryType::MultiLineString),
        gt::MULTIPOLYGON       => Some(GeometryType::MultiPolygon),
        gt::GEOMETRYCOLLECTION => Some(GeometryType::GeometryCollection),
        _                      => None,
    }
}

fn col_type_to_field_type(ct: u8) -> FieldType {
    match ct {
        ct::BOOL                                           => FieldType::Boolean,
        ct::BYTE|ct::UBYTE|ct::SHORT|ct::USHORT
        |ct::INT|ct::UINT|ct::LONG|ct::ULONG              => FieldType::Integer,
        ct::FLOAT|ct::DOUBLE                              => FieldType::Float,
        ct::DATETIME                                       => FieldType::DateTime,
        ct::BINARY                                         => FieldType::Blob,
        ct::JSON                                           => FieldType::Json,
        _                                                  => FieldType::Text,
    }
}

fn field_type_to_col_type(ft: FieldType) -> u8 {
    match ft {
        FieldType::Boolean  => ct::BOOL,
        FieldType::Integer  => ct::LONG,
        FieldType::Float    => ct::DOUBLE,
        FieldType::Text     => ct::STRING,
        FieldType::Date     => ct::STRING,
        FieldType::DateTime => ct::DATETIME,
        FieldType::Blob     => ct::BINARY,
        FieldType::Json     => ct::JSON,
    }
}

fn std_geom_type(gt: GeometryType) -> hg::GeometryType {
    match gt {
        GeometryType::Point              => hg::GeometryType::Point,
        GeometryType::LineString         => hg::GeometryType::LineString,
        GeometryType::Polygon            => hg::GeometryType::Polygon,
        GeometryType::MultiPoint         => hg::GeometryType::MultiPoint,
        GeometryType::MultiLineString    => hg::GeometryType::MultiLineString,
        GeometryType::MultiPolygon       => hg::GeometryType::MultiPolygon,
        GeometryType::GeometryCollection => hg::GeometryType::GeometryCollection,
    }
}

fn std_col_type(ct_code: u8) -> hg::ColumnType {
    match ct_code {
        ct::BYTE     => hg::ColumnType::Byte,
        ct::UBYTE    => hg::ColumnType::UByte,
        ct::BOOL     => hg::ColumnType::Bool,
        ct::SHORT    => hg::ColumnType::Short,
        ct::USHORT   => hg::ColumnType::UShort,
        ct::INT      => hg::ColumnType::Int,
        ct::UINT     => hg::ColumnType::UInt,
        ct::LONG     => hg::ColumnType::Long,
        ct::ULONG    => hg::ColumnType::ULong,
        ct::FLOAT    => hg::ColumnType::Float,
        ct::DOUBLE   => hg::ColumnType::Double,
        ct::STRING   => hg::ColumnType::String,
        ct::JSON     => hg::ColumnType::Json,
        ct::DATETIME => hg::ColumnType::DateTime,
        ct::BINARY   => hg::ColumnType::Binary,
        _            => hg::ColumnType::String,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FieldDef, FieldType};

    fn sample_layer() -> Layer {
        let mut l = Layer::new("rivers")
            .with_geom_type(GeometryType::LineString)
            .with_epsg(4326);
        l.add_field(FieldDef::new("name",   FieldType::Text));
        l.add_field(FieldDef::new("length", FieldType::Float));
        l.add_feature(
            Some(Geometry::line_string(vec![Coord::xy(0.,0.), Coord::xy(1.,1.), Coord::xy(2.,0.)])),
            &[("name", "Nile".into()), ("length", 6650.0f64.into())],
        ).unwrap();
        l.add_feature(
            Some(Geometry::line_string(vec![Coord::xy(-80., 0.), Coord::xy(-79., 1.)])),
            &[("name", "Amazon".into()), ("length", 6400.0f64.into())],
        ).unwrap();
        l
    }

    #[test]
    fn magic_preserved() {
        let bytes = to_bytes(&sample_layer());
        assert_eq!(&bytes[0..8], &MAGIC);
    }

    #[test]
    fn in_memory_roundtrip() {
        let l1 = sample_layer();
        let bytes = to_bytes(&l1);
        let l2 = from_bytes(&bytes).unwrap();
        assert_eq!(l2.len(), 2);
        assert_eq!(l2.schema.len(), 2);
    }

    #[test]
    fn geometry_preserved() {
        let l1 = sample_layer();
        let bytes = to_bytes(&l1);
        let l2 = from_bytes(&bytes).unwrap();
        if let Some(Geometry::LineString(cs)) = &l2[0].geometry {
            assert!((cs[0].x - 0.0).abs() < 1e-9);
            assert!((cs[1].x - 1.0).abs() < 1e-9);
        } else { panic!("expected LineString"); }
    }

    #[test]
    fn attributes_preserved() {
        let l1 = sample_layer();
        let bytes = to_bytes(&l1);
        let l2 = from_bytes(&bytes).unwrap();
        let name = l2[0].get(&l2.schema, "name").unwrap();
        assert_eq!(name, &FieldValue::Text("Nile".into()));
    }

    #[test]
    fn file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rivers.fgb");
        let l1 = sample_layer();
        write(&l1, &path).unwrap();
        let l2 = read(&path).unwrap();
        assert_eq!(l2.len(), 2);
    }

    #[test]
    fn polygon_roundtrip() {
        let mut l = Layer::new("polys").with_geom_type(GeometryType::Polygon);
        l.add_feature(
            Some(Geometry::polygon(
                vec![Coord::xy(0.,0.), Coord::xy(1.,0.), Coord::xy(1.,1.), Coord::xy(0.,1.)],
                vec![],
            )),
            &[],
        ).unwrap();
        let bytes = to_bytes(&l);
        let l2 = from_bytes(&bytes).unwrap();
        assert!(matches!(l2[0].geometry, Some(Geometry::Polygon { .. })));
    }

    #[test]
    fn crs_roundtrip_from_epsg_populates_wkt() {
        let mut l = sample_layer();
        l.set_crs_epsg(Some(3857));
        l.set_crs_wkt(None);

        let bytes = to_bytes(&l);
        let out = from_bytes(&bytes).unwrap();

        assert_eq!(out.crs_epsg(), Some(3857));
        assert!(out.crs_wkt().map(|w| !w.is_empty()).unwrap_or(false));
    }

    #[test]
    fn crs_roundtrip_from_wkt_infers_epsg() {
        let mut l = sample_layer();
        l.set_crs_epsg(None);
        l.set_crs_wkt(Some(
            "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],AUTHORITY[\"EPSG\",\"4326\"]]"
                .to_owned(),
        ));

        let bytes = to_bytes(&l);
        let out = from_bytes(&bytes).unwrap();

        assert_eq!(out.crs_epsg(), Some(4326));
        assert!(out.crs_wkt().map(|w| !w.is_empty()).unwrap_or(false));
    }
}
