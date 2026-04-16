//! Shapefile (`.shp` / `.shx` / `.dbf` / `.prj`) reader and writer.
//!
//! The Shapefile format is defined by ESRI's whitepaper
//! "ESRI Shapefile Technical Description" (July 1998).
//!
//! ## File layout
//! * `.shp` – geometry records; big-endian file / record headers, LE coordinates
//! * `.shx` – index (offset + length in 16-bit words) for each record
//! * `.dbf` – dBASE III+ attribute table
//! * `.prj` – WKT CRS string (optional, read-only by this driver)
//!
//! ## Shape type codes
//! ```text
//!  0 Null         1 Point        3 PolyLine    5 Polygon
//!  8 MultiPoint  11 PointZ      13 PolyLineZ  15 PolygonZ
//! 18 MultiPointZ 21 PointM      23 PolyLineM  25 PolygonM
//! ```

use std::path::{Path, PathBuf};
use crate::crs;
use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{BBox, Coord, Geometry, GeometryType, Ring};

// ── shape-type constants ─────────────────────────────────────────────────────

const SHP_NULL:         i32 =  0;
const SHP_POINT:        i32 =  1;
const SHP_POLYLINE:     i32 =  3;
const SHP_POLYGON:      i32 =  5;
const SHP_MULTIPOINT:   i32 =  8;
const SHP_POINT_Z:      i32 = 11;
const SHP_POLYLINE_Z:   i32 = 13;
const SHP_POLYGON_Z:    i32 = 15;
const SHP_MULTIPOINT_Z: i32 = 18;
const SHP_POINT_M:      i32 = 21;
const SHP_POLYLINE_M:   i32 = 23;
const SHP_POLYGON_M:    i32 = 25;
const SHP_MULTIPOINT_M: i32 = 28;

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a Shapefile set into a [`Layer`].
///
/// `path` may be the `.shp` file or the base name without extension.
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let base = base_path(path.as_ref());

    let shp = std::fs::read(base.with_extension("shp")).map_err(GeoError::Io)?;
    let dbf = std::fs::read(base.with_extension("dbf")).map_err(GeoError::Io)?;
    let prj = std::fs::read_to_string(base.with_extension("prj")).ok();

    let name = base.file_stem().and_then(|s| s.to_str()).unwrap_or("layer").to_owned();
    let mut layer = Layer::new(name);
    if let Some(wkt) = prj {
        let trimmed = wkt.trim().to_owned();
        layer.set_crs_epsg(crs::epsg_from_wkt_lenient(&trimmed));
        layer.set_crs_wkt(Some(trimmed));
    }

    // ── parse .shp header (100 bytes) ────────────────────────────────────────
    if shp.len() < 100 {
        return Err(GeoError::NotShapefile("file too short".into()));
    }
    let file_code = i32_be(&shp, 0);
    if file_code != 9994 {
        return Err(GeoError::NotShapefile(format!("file code {file_code} ≠ 9994")));
    }

    // Extract shape type from .shp header (offset 32, little-endian)
    // and set layer.geom_type accordingly
    let file_shape_type = i32_le(&shp, 32);
    layer.geom_type = match file_shape_type {
        SHP_POINT | SHP_POINT_M | SHP_POINT_Z => Some(GeometryType::Point),
        SHP_POLYLINE | SHP_POLYLINE_M | SHP_POLYLINE_Z => Some(GeometryType::LineString),
        SHP_POLYGON | SHP_POLYGON_M | SHP_POLYGON_Z => Some(GeometryType::Polygon),
        SHP_MULTIPOINT | SHP_MULTIPOINT_M | SHP_MULTIPOINT_Z => Some(GeometryType::Point),
        _ => None,
    };

    // ── parse .dbf ────────────────────────────────────────────────────────────
    let (schema, dbf_rows) = read_dbf(&dbf)?;
    for fd in schema.fields() { layer.add_field(fd.clone()); }

    // ── parse .shp records ────────────────────────────────────────────────────
    let mut pos  = 100usize;
    let mut ridx = 0usize;

    while pos + 8 <= shp.len() {
        let _rec_num     = i32_be(&shp, pos);
        let content_len  = i32_be(&shp, pos + 4) as usize * 2; // in 16-bit words
        pos += 8;

        if pos + content_len > shp.len() { break; }
        let rec = &shp[pos..pos + content_len];
        pos += content_len;

        let geom = if content_len >= 4 {
            let rec_type = i32_le(rec, 0);
            if rec_type == SHP_NULL { None } else { Some(parse_shape(rec)?) }
        } else { None };

        let attrs = if ridx < dbf_rows.len() {
            dbf_rows[ridx].clone()
        } else {
            vec![FieldValue::Null; schema.len()]
        };

        layer.push(Feature { fid: ridx as u64, geometry: geom, attributes: attrs });
        ridx += 1;
    }

    Ok(layer)
}

/// Write a [`Layer`] as a Shapefile set (`.shp`, `.shx`, `.dbf`, `.prj`).
///
/// `path` may include `.shp` or be the base name.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let base = base_path(path.as_ref());

    let shape_type = infer_shape_type(layer);

    // ── compute overall bbox ─────────────────────────────────────────────────
    let mut bb = BBox::new(f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for f in &layer.features {
        if let Some(fb) = f.geometry.as_ref().and_then(|g| g.bbox()) { bb.expand_to(&fb); }
    }
    if bb.min_x == f64::INFINITY { bb = BBox::new(0., 0., 0., 0.); }

    // ── encode records ────────────────────────────────────────────────────────
    let mut shp: Vec<u8> = Vec::new();
    let mut shx: Vec<u8> = Vec::new();
    shp_header(&mut shp, shape_type, &bb, 0); // file length patched below
    shp_header(&mut shx, shape_type, &bb, 0);

    for (idx, feat) in layer.features.iter().enumerate() {
        let off_words = (shp.len() / 2) as u32;
        let rec_body  = encode_shape(feat.geometry.as_ref(), shape_type)?;
        let len_words = (rec_body.len() / 2) as u32;

        // SHX entry
        shx.extend_from_slice(&off_words.to_be_bytes());
        shx.extend_from_slice(&len_words.to_be_bytes());

        // SHP record header + body
        shp.extend_from_slice(&((idx + 1) as u32).to_be_bytes());
        shp.extend_from_slice(&len_words.to_be_bytes());
        shp.extend_from_slice(&rec_body);
    }

    // Patch file lengths
    let shp_words = (shp.len() / 2) as u32;
    shp[24..28].copy_from_slice(&shp_words.to_be_bytes());
    let shx_words = (shx.len() / 2) as u32;
    shx[24..28].copy_from_slice(&shx_words.to_be_bytes());

    // ── write .dbf ────────────────────────────────────────────────────────────
    let dbf = build_dbf(layer)?;

    // ── write files ───────────────────────────────────────────────────────────
    std::fs::write(base.with_extension("shp"), &shp).map_err(GeoError::Io)?;
    std::fs::write(base.with_extension("shx"), &shx).map_err(GeoError::Io)?;
    std::fs::write(base.with_extension("dbf"), &dbf).map_err(GeoError::Io)?;

    if let Some(wkt) = layer.crs_wkt() {
        std::fs::write(base.with_extension("prj"), wkt.as_bytes()).map_err(GeoError::Io)?;
    } else if let Some(epsg) = layer.crs_epsg() {
        let wkt = crs::ogc_wkt_from_epsg(epsg)
            .unwrap_or_else(|| default_prj(epsg).to_owned());
        std::fs::write(base.with_extension("prj"), wkt.as_bytes()).map_err(GeoError::Io)?;
    }

    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// SHP geometry parsing
// ══════════════════════════════════════════════════════════════════════════════

fn parse_shape(data: &[u8]) -> Result<Geometry> {
    let shape_type = i32_le(data, 0);

    // helpers to read at byte offset
    let rd = |off: usize| -> f64 { f64_le(data, off) };
    let ri = |off: usize| -> i32 { i32_le(data, off) };

    match shape_type {
        SHP_POINT | SHP_POINT_M => {
            if data.len() < 20 { return Err(GeoError::NotShapefile("Point too short".into())); }
            Ok(Geometry::point(rd(4), rd(12)))
        }
        SHP_POINT_Z => {
            if data.len() < 28 { return Err(GeoError::NotShapefile("PointZ too short".into())); }
            Ok(Geometry::point_z(rd(4), rd(12), rd(20)))
        }
        SHP_POLYLINE | SHP_POLYLINE_M | SHP_POLYLINE_Z => {
            if data.len() < 44 { return Err(GeoError::NotShapefile("PolyLine too short".into())); }
            let (parts, points) = read_parts_points(data, 36, 40, 44)?;
            if parts.len() == 1 {
                Ok(Geometry::LineString(points))
            } else {
                Ok(Geometry::MultiLineString(split_parts(&points, &parts)))
            }
        }
        SHP_POLYGON | SHP_POLYGON_M | SHP_POLYGON_Z => {
            if data.len() < 44 { return Err(GeoError::NotShapefile("Polygon too short".into())); }
            let (parts, points) = read_parts_points(data, 36, 40, 44)?;
            let rings: Vec<Ring> = split_parts(&points, &parts)
                .into_iter()
                .map(|mut cs| {
                    // remove closing point
                    if cs.len() > 1 && cs.first() == cs.last() { cs.pop(); }
                    Ring::new(cs)
                })
                .collect();
            // Shapefile: exterior = CW (negative signed area), holes = CCW
            let (exteriors, holes) = partition_rings(rings);
            if exteriors.len() <= 1 {
                let exterior = exteriors.into_iter().next().unwrap_or_default();
                Ok(Geometry::Polygon { exterior, interiors: holes })
            } else {
                // Multiple exterior rings → MultiPolygon (simplified: no hole assignment)
                let polys = exteriors.into_iter().map(|e| (e, vec![])).collect();
                Ok(Geometry::MultiPolygon(polys))
            }
        }
        SHP_MULTIPOINT | SHP_MULTIPOINT_M | SHP_MULTIPOINT_Z => {
            if data.len() < 40 { return Err(GeoError::NotShapefile("MultiPoint too short".into())); }
            let n   = ri(36) as usize;
            let pts = (0..n)
                .map(|i| { let off = 40 + i * 16; Coord::xy(rd(off), rd(off + 8)) })
                .collect();
            Ok(Geometry::MultiPoint(pts))
        }
        other => Err(GeoError::UnsupportedShapeType(other)),
    }
}

fn read_parts_points(data: &[u8], np_off: usize, npt_off: usize, arr_off: usize) -> Result<(Vec<usize>, Vec<Coord>)> {
    let num_parts  = i32_le(data, np_off)  as usize;
    let num_points = i32_le(data, npt_off) as usize;
    let pts_off    = arr_off + num_parts * 4;

    if data.len() < pts_off + num_points * 16 {
        return Err(GeoError::NotShapefile("record truncated".into()));
    }

    let parts: Vec<usize> = (0..num_parts)
        .map(|i| i32_le(data, arr_off + i * 4) as usize)
        .collect();

    let points: Vec<Coord> = (0..num_points)
        .map(|i| { let off = pts_off + i * 16; Coord::xy(f64_le(data, off), f64_le(data, off + 8)) })
        .collect();

    Ok((parts, points))
}

fn split_parts(points: &[Coord], parts: &[usize]) -> Vec<Vec<Coord>> {
    let n = points.len();
    parts.iter().enumerate().map(|(p, &start)| {
        let end = if p + 1 < parts.len() { parts[p + 1] } else { n };
        points[start..end.min(n)].to_vec()
    }).collect()
}

/// Separate rings into exterior (CW = negative area) and holes (CCW = positive).
fn partition_rings(rings: Vec<Ring>) -> (Vec<Ring>, Vec<Ring>) {
    let mut exts  = Vec::new();
    let mut holes = Vec::new();
    for r in rings {
        if r.signed_area() <= 0.0 { exts.push(r); } else { holes.push(r); }
    }
    (exts, holes)
}

// ══════════════════════════════════════════════════════════════════════════════
// SHP geometry encoding
// ══════════════════════════════════════════════════════════════════════════════

fn encode_shape(geom: Option<&Geometry>, _shape_type: i32) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let Some(geom) = geom else {
        buf.extend_from_slice(&SHP_NULL.to_le_bytes());
        return Ok(buf);
    };

    match geom {
        Geometry::Point(c) => {
            buf.extend_from_slice(&SHP_POINT.to_le_bytes());
            buf.extend_from_slice(&c.x.to_le_bytes());
            buf.extend_from_slice(&c.y.to_le_bytes());
        }
        Geometry::LineString(cs) => {
            encode_polyline(&mut buf, &[cs.as_slice()]);
        }
        Geometry::Polygon { exterior, interiors } => {
            // ESRI Shapefile polygon convention: exterior rings CW, holes CCW.
            let mut parts: Vec<Vec<Coord>> = vec![ring_closed_oriented(exterior, true)];
            for r in interiors { parts.push(ring_closed_oriented(r, false)); }
            let slices: Vec<&[Coord]> = parts.iter().map(|p| p.as_slice()).collect();
            encode_polygon(&mut buf, &slices);
        }
        Geometry::MultiPoint(cs) => {
            encode_multipoint(&mut buf, cs);
        }
        Geometry::MultiLineString(ls) => {
            let slices: Vec<&[Coord]> = ls.iter().map(|l| l.as_slice()).collect();
            encode_polyline(&mut buf, &slices);
        }
        Geometry::MultiPolygon(ps) => {
            let mut parts: Vec<Vec<Coord>> = Vec::new();
            for (ext, holes) in ps {
                parts.push(ring_closed_oriented(ext, true));
                for h in holes { parts.push(ring_closed_oriented(h, false)); }
            }
            let slices: Vec<&[Coord]> = parts.iter().map(|p| p.as_slice()).collect();
            encode_polygon(&mut buf, &slices);
        }
        Geometry::GeometryCollection(_) => {
            // Write as null – GeometryCollection is not a native Shapefile type
            buf.extend_from_slice(&SHP_NULL.to_le_bytes());
        }
    }
    Ok(buf)
}

fn ring_closed(ring: &Ring) -> Vec<Coord> {
    let mut v = ring.0.clone();
    if !v.is_empty() && v.first() != v.last() { v.push(v[0].clone()); }
    v
}

fn ring_closed_oriented(ring: &Ring, want_cw: bool) -> Vec<Coord> {
    let mut v = ring_closed(ring);
    if v.len() < 4 {
        return v;
    }

    let area = ring.signed_area();
    let is_cw = area < 0.0;
    if is_cw != want_cw {
        // Preserve closure while reversing orientation.
        v.pop();
        v.reverse();
        if !v.is_empty() {
            v.push(v[0].clone());
        }
    }

    v
}

fn parts_bbox(parts: &[&[Coord]]) -> BBox {
    let mut bb = BBox::new(f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &p in parts {
        for c in p { bb.min_x = bb.min_x.min(c.x); bb.max_x = bb.max_x.max(c.x);
                     bb.min_y = bb.min_y.min(c.y); bb.max_y = bb.max_y.max(c.y); }
    }
    if bb.min_x == f64::INFINITY { bb = BBox::new(0.,0.,0.,0.); }
    bb
}

fn encode_polyline(buf: &mut Vec<u8>, parts: &[&[Coord]]) {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    let bb = parts_bbox(parts);
    buf.extend_from_slice(&SHP_POLYLINE.to_le_bytes());
    push_bbox(buf, &bb);
    buf.extend_from_slice(&(parts.len() as i32).to_le_bytes());
    buf.extend_from_slice(&(total as i32).to_le_bytes());
    let mut off = 0i32;
    for p in parts { buf.extend_from_slice(&off.to_le_bytes()); off += p.len() as i32; }
    for p in parts { for c in *p { push_xy(buf, c); } }
}

fn encode_polygon(buf: &mut Vec<u8>, parts: &[&[Coord]]) {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    let bb = parts_bbox(parts);
    buf.extend_from_slice(&SHP_POLYGON.to_le_bytes());
    push_bbox(buf, &bb);
    buf.extend_from_slice(&(parts.len() as i32).to_le_bytes());
    buf.extend_from_slice(&(total as i32).to_le_bytes());
    let mut off = 0i32;
    for p in parts { buf.extend_from_slice(&off.to_le_bytes()); off += p.len() as i32; }
    for p in parts { for c in *p { push_xy(buf, c); } }
}

fn encode_multipoint(buf: &mut Vec<u8>, pts: &[Coord]) {
    let bb = parts_bbox(&[pts]);
    buf.extend_from_slice(&SHP_MULTIPOINT.to_le_bytes());
    push_bbox(buf, &bb);
    buf.extend_from_slice(&(pts.len() as i32).to_le_bytes());
    for c in pts { push_xy(buf, c); }
}

fn push_xy(buf: &mut Vec<u8>, c: &Coord) {
    buf.extend_from_slice(&c.x.to_le_bytes());
    buf.extend_from_slice(&c.y.to_le_bytes());
}

fn push_bbox(buf: &mut Vec<u8>, bb: &BBox) {
    buf.extend_from_slice(&bb.min_x.to_le_bytes());
    buf.extend_from_slice(&bb.min_y.to_le_bytes());
    buf.extend_from_slice(&bb.max_x.to_le_bytes());
    buf.extend_from_slice(&bb.max_y.to_le_bytes());
}

// ══════════════════════════════════════════════════════════════════════════════
// SHP file header
// ══════════════════════════════════════════════════════════════════════════════

fn shp_header(buf: &mut Vec<u8>, shape_type: i32, bb: &BBox, file_len_words: u32) {
    buf.extend_from_slice(&9994i32.to_be_bytes());   // file code
    buf.extend_from_slice(&[0u8; 20]);               // unused
    buf.extend_from_slice(&file_len_words.to_be_bytes()); // patched later
    buf.extend_from_slice(&1000i32.to_le_bytes());   // version
    buf.extend_from_slice(&shape_type.to_le_bytes());
    push_bbox(buf, bb);
    buf.extend_from_slice(&0.0f64.to_le_bytes()); // Zmin
    buf.extend_from_slice(&0.0f64.to_le_bytes()); // Zmax
    buf.extend_from_slice(&0.0f64.to_le_bytes()); // Mmin
    buf.extend_from_slice(&0.0f64.to_le_bytes()); // Mmax
}

// ══════════════════════════════════════════════════════════════════════════════
// dBASE III+ (.dbf) reader
// ══════════════════════════════════════════════════════════════════════════════

fn read_dbf(data: &[u8]) -> Result<(crate::feature::Schema, Vec<Vec<FieldValue>>)> {
    use crate::feature::Schema;

    if data.len() < 32 {
        return Err(GeoError::InvalidDbf("file too short".into()));
    }
    let num_records  = u32_le(data, 4) as usize;
    let header_bytes = u16_le(data, 8) as usize;
    let record_len   = u16_le(data, 10) as usize;

    // Each field descriptor is 32 bytes; header ends with 0x0D terminator
    let num_fields = header_bytes.saturating_sub(33) / 32;

    let mut schema = Schema::new();
    // (offset-in-record, byte-length, type-char, decimals)
    let mut field_meta: Vec<(usize, usize, u8, u8)> = Vec::new();
    let mut col_off = 1usize; // byte 0 = deletion flag

    for i in 0..num_fields {
        let base = 32 + i * 32;
        if base + 32 > data.len() { break; }

        // name: 11 bytes, NUL-padded
        let name_end = data[base..base+11].iter().position(|&b| b == 0).unwrap_or(11);
        let name = String::from_utf8_lossy(&data[base..base + name_end]).to_string();
        let ftype    = data[base + 11];
        let length   = data[base + 16] as usize;
        let decimals = data[base + 17];

        let field_type = match ftype {
            b'C' => FieldType::Text,
            b'N' | b'F' => if decimals > 0 { FieldType::Float } else { FieldType::Integer },
            b'D' => FieldType::Date,
            b'L' => FieldType::Boolean,
            b'M' => FieldType::Blob,
            b'T' => FieldType::DateTime,
            _    => FieldType::Text,
        };

        field_meta.push((col_off, length, ftype, decimals));
        col_off += length;

        schema.add_field(FieldDef::new(name, field_type).width(length).precision(decimals as usize));
    }

    // Parse records
    let data_start = header_bytes;
    let mut rows: Vec<Vec<FieldValue>> = Vec::with_capacity(num_records);

    for r in 0..num_records {
        let rec_off = data_start + r * record_len;
        if rec_off + record_len > data.len() { break; }
        let rec = &data[rec_off..rec_off + record_len];

        if rec[0] == 0x2A { continue; } // deleted record

        let mut row: Vec<FieldValue> = Vec::with_capacity(field_meta.len());
        for &(off, len, ftype, decimals) in &field_meta {
            let end = (off + len).min(rec.len());
            let raw = if off < rec.len() { &rec[off..end] } else { b"" };
            let s   = String::from_utf8_lossy(raw).trim().to_string();

            let val = if s.is_empty() || s.bytes().all(|b| b == 0) {
                FieldValue::Null
            } else {
                match ftype {
                    b'C' => FieldValue::Text(s),
                    b'D' => FieldValue::Date(s),
                    b'T' => FieldValue::DateTime(s),
                    b'L' => match s.to_ascii_uppercase().as_str() {
                        "T" | "Y" | "1" | "TRUE" | "YES" => FieldValue::Boolean(true),
                        _                                  => FieldValue::Boolean(false),
                    },
                    b'N' | b'F' => {
                        if decimals > 0 {
                            s.parse::<f64>().map(FieldValue::Float).unwrap_or(FieldValue::Null)
                        } else {
                            s.parse::<i64>()
                             .or_else(|_| s.parse::<f64>().map(|f| f as i64))
                             .map(FieldValue::Integer)
                             .unwrap_or(FieldValue::Null)
                        }
                    }
                    _ => FieldValue::Text(s),
                }
            };
            row.push(val);
        }
        rows.push(row);
    }

    Ok((schema, rows))
}

// ══════════════════════════════════════════════════════════════════════════════
// dBASE III+ (.dbf) writer
// ══════════════════════════════════════════════════════════════════════════════

fn build_dbf(layer: &Layer) -> Result<Vec<u8>> {
    let fields    = layer.schema.fields();
    let n_fields  = fields.len();
    let n_records = layer.features.len() as u32;

    // Compute per-field byte length in the record
    let field_lens: Vec<usize> = fields.iter().map(|f| match f.field_type {
        FieldType::Text     => f.width.max(1).min(254),
        FieldType::Integer  => 11usize,
        FieldType::Float    => 14usize,
        FieldType::Boolean  => 1usize,
        FieldType::Date     => 8usize,
        FieldType::DateTime => 14usize,
        _                   => f.width.max(10).min(254),
    }).collect();

    let record_len: usize = 1 + field_lens.iter().sum::<usize>(); // 1 for deletion flag
    let header_size = 32 + n_fields * 32 + 1;                     // +1 for 0x0D terminator

    let mut buf: Vec<u8> = Vec::new();

    // File header
    buf.push(0x03);                                         // dBASE III version
    buf.extend_from_slice(&[0u8; 3]);                       // date YY MM DD
    buf.extend_from_slice(&n_records.to_le_bytes());
    buf.extend_from_slice(&(header_size as u16).to_le_bytes());
    buf.extend_from_slice(&(record_len  as u16).to_le_bytes());
    buf.extend_from_slice(&[0u8; 20]);                      // reserved

    // Field descriptor array
    for (i, f) in fields.iter().enumerate() {
        let mut nbuf = [0u8; 11];
        let nb = f.name.as_bytes();
        nbuf[..nb.len().min(10)].copy_from_slice(&nb[..nb.len().min(10)]);
        buf.extend_from_slice(&nbuf);

        let dtype = match f.field_type {
            FieldType::Text    => b'C',
            FieldType::Integer | FieldType::Float => b'N',
            FieldType::Boolean => b'L',
            FieldType::Date    => b'D',
            FieldType::DateTime => b'T',
            _                  => b'C',
        };
        buf.push(dtype);
        buf.extend_from_slice(&[0u8; 4]); // field data address (unused)
        buf.push(field_lens[i] as u8);
        let dec = match f.field_type { FieldType::Float => f.precision.min(15) as u8, _ => 0 };
        buf.push(dec);
        buf.extend_from_slice(&[0u8; 14]); // reserved
    }
    buf.push(0x0D); // header terminator

    // Records
    for feat in &layer.features {
        buf.push(0x20); // not deleted
        for (i, f) in fields.iter().enumerate() {
            let flen = field_lens[i];
            let val  = feat.attributes.get(i).unwrap_or(&FieldValue::Null);
            let cell = match val {
                FieldValue::Integer(v) => format!("{v:>width$}", width = flen),
                FieldValue::Float(v)   => {
                    let dec = f.precision.min(15);
                    format!("{v:>width$.prec$}", width = flen, prec = dec)
                }
                FieldValue::Text(s)    => {
                    let s = if s.len() > flen { &s[..flen] } else { s.as_str() };
                    format!("{s:<width$}", width = flen)
                }
                FieldValue::Boolean(b) => if *b { "T".into() } else { "F".into() },
                FieldValue::Date(s)    => format!("{:>8}", &s[..s.len().min(8)]),
                FieldValue::DateTime(s) => format!("{:<14}", &s[..s.len().min(14)]),
                FieldValue::Null       => " ".repeat(flen),
                _                      => " ".repeat(flen),
            };
            let bytes = cell.as_bytes();
            let copy  = bytes.len().min(flen);
            buf.extend_from_slice(&bytes[..copy]);
            for _ in copy..flen { buf.push(b' '); }
        }
    }
    buf.push(0x1A); // EOF
    Ok(buf)
}

// ══════════════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════════════

fn infer_shape_type(layer: &Layer) -> i32 {
    if let Some(gt) = layer.geom_type {
        return match gt {
            GeometryType::Point              => SHP_POINT,
            GeometryType::LineString         => SHP_POLYLINE,
            GeometryType::Polygon            => SHP_POLYGON,
            GeometryType::MultiPoint         => SHP_MULTIPOINT,
            GeometryType::MultiLineString    => SHP_POLYLINE,
            GeometryType::MultiPolygon       => SHP_POLYGON,
            GeometryType::GeometryCollection => SHP_NULL,
        };
    }
    for f in &layer.features {
        if let Some(g) = &f.geometry {
            return match g {
                Geometry::Point(_)            => SHP_POINT,
                Geometry::LineString(_)       => SHP_POLYLINE,
                Geometry::Polygon { .. }      => SHP_POLYGON,
                Geometry::MultiPoint(_)       => SHP_MULTIPOINT,
                Geometry::MultiLineString(_)  => SHP_POLYLINE,
                Geometry::MultiPolygon(_)     => SHP_POLYGON,
                Geometry::GeometryCollection(_) => SHP_NULL,
            };
        }
    }
    SHP_NULL
}

fn base_path(path: &Path) -> PathBuf {
    match path.extension().and_then(|e| e.to_str()) {
        Some("shp") | Some("shx") | Some("dbf") | Some("prj") => path.with_extension(""),
        _ => path.to_path_buf(),
    }
}

fn default_prj(epsg: u32) -> &'static str {
    match epsg {
        4326 => r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#,
        3857 => r#"PROJCS["WGS_1984_Web_Mercator_Auxiliary_Sphere",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Mercator_Auxiliary_Sphere"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Standard_Parallel_1",0.0],PARAMETER["Auxiliary_Sphere_Type",0.0],UNIT["Meter",1.0]]"#,
        _    => r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#,
    }
}

// ── byte-reading helpers ─────────────────────────────────────────────────────

fn i32_be(d: &[u8], off: usize) -> i32 { i32::from_be_bytes(d[off..off+4].try_into().unwrap()) }
fn i32_le(d: &[u8], off: usize) -> i32 { i32::from_le_bytes(d[off..off+4].try_into().unwrap()) }
fn f64_le(d: &[u8], off: usize) -> f64 { f64::from_le_bytes(d[off..off+8].try_into().unwrap()) }
fn u16_le(d: &[u8], off: usize) -> u16 { u16::from_le_bytes(d[off..off+2].try_into().unwrap()) }
fn u32_le(d: &[u8], off: usize) -> u32 { u32::from_le_bytes(d[off..off+4].try_into().unwrap()) }

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::{FieldDef, FieldType};

    fn point_layer() -> Layer {
        let mut l = Layer::new("pts")
            .with_geom_type(GeometryType::Point)
            .with_epsg(4326);
        l.add_field(FieldDef::new("name",  FieldType::Text).width(50));
        l.add_field(FieldDef::new("value", FieldType::Float).precision(4));
        l.add_feature(Some(Geometry::point( 10.0, 20.0)), &[("name", "alpha".into()), ("value", 1.5f64.into())]).unwrap();
        l.add_feature(Some(Geometry::point(-70.0, 42.5)), &[("name", "beta".into()),  ("value", 2.5f64.into())]).unwrap();
        l
    }

    fn polygon_layer() -> Layer {
        let mut l = Layer::new("polys").with_geom_type(GeometryType::Polygon).with_epsg(4326);
        l.add_field(FieldDef::new("id", FieldType::Integer));
        l.add_feature(
            Some(Geometry::polygon(
                vec![Coord::xy(0.,0.), Coord::xy(1.,0.), Coord::xy(1.,1.), Coord::xy(0.,1.)],
                vec![],
            )),
            &[("id", 1i64.into())],
        ).unwrap();
        l
    }

    #[test]
    fn roundtrip_points() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pts");
        let layer = point_layer();
        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();
        assert_eq!(out.len(), 2);
        if let Some(Geometry::Point(c)) = &out[0].geometry {
            assert!((c.x - 10.0).abs() < 1e-9);
            assert!((c.y - 20.0).abs() < 1e-9);
        } else { panic!("expected Point"); }
        let name = out[0].get(&out.schema, "name").unwrap();
        assert_eq!(name, &FieldValue::Text("alpha".into()));
    }

    #[test]
    fn roundtrip_polygon() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("polys");
        let layer = polygon_layer();
        write(&layer, &path).unwrap();
        let out = read(&path).unwrap();
        assert_eq!(out.len(), 1);
        assert!(matches!(&out[0].geometry, Some(Geometry::Polygon { .. })));
    }

    #[test]
    fn writes_prj_for_non_default_epsg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mercator_pts");
        let mut layer = point_layer();
        layer.set_crs_wkt(None);
        layer.set_crs_epsg(Some(3857));

        write(&layer, &path).unwrap();
        let prj = std::fs::read_to_string(path.with_extension("prj")).unwrap();

        assert!(!prj.trim().is_empty());
        assert!(prj.contains("PROJCS") || prj.contains("GEOGCS"));
    }
}
