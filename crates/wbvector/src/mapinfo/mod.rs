//! MapInfo Interchange Format (MIF/MID) reader and writer.
//!
//! This driver supports practical MIF/MID interoperability for:
//! - Point, Line, Pline, Region, Multipoint, and None geometries
//! - Attribute schemas via `Columns` + `.mid` rows
//! - Common field types: Integer, Float, Text, Boolean, Date, DateTime
//!
//! Notes:
//! - MIF/MID stores geometry in `.mif` and attributes in partner `.mid`.
//! - CRS support is intentionally lightweight in this phase:
//!   `CoordSys Earth Projection 1` is interpreted as EPSG:4326.

use std::path::{Path, PathBuf};

use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Coord, Geometry, GeometryType, Ring};

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Read a MIF/MID dataset into a [`Layer`].
///
/// `path` may be either:
/// - a `.mif` path, or
/// - a base path without extension.
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    let (mif_path, mid_path) = resolve_pair_paths(path.as_ref());
    let mif_text = std::fs::read_to_string(&mif_path).map_err(GeoError::Io)?;

    let mid_text = match std::fs::read_to_string(&mid_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(GeoError::Io(e)),
    };

    parse_pair_str(&mif_text, &mid_text)
}

/// Parse MIF + MID strings into a [`Layer`].
pub fn parse_pair_str(mif_text: &str, mid_text: &str) -> Result<Layer> {
    let parsed = parse_mif(mif_text)?;

    let rows = if parsed.columns.is_empty() {
        Vec::new()
    } else {
        parse_mid_rows(mid_text, parsed.delimiter)?
    };

    if !parsed.columns.is_empty() && rows.len() < parsed.geometries.len() {
        return Err(GeoError::MifParse {
            line: 0,
            msg: format!(
                "MID row count ({}) is less than geometry count ({})",
                rows.len(),
                parsed.geometries.len()
            ),
        });
    }

    let mut layer = Layer::new(parsed.layer_name);
    if let Some(epsg) = parsed.epsg {
        layer = layer.with_epsg(epsg);
    }

    for col in &parsed.columns {
        layer.add_field(col.clone());
    }

    let mut declared_geom: Option<GeometryType> = None;
    let mut mixed_geom = false;

    for (fid, geom) in parsed.geometries.into_iter().enumerate() {
        if let Some(g) = &geom {
            let gt = g.geom_type();
            if let Some(prev) = declared_geom {
                if prev != gt {
                    mixed_geom = true;
                }
            } else {
                declared_geom = Some(gt);
            }
        }

        let attrs = if parsed.columns.is_empty() {
            vec![]
        } else {
            let row = rows.get(fid).ok_or_else(|| GeoError::MifParse {
                line: 0,
                msg: format!("missing MID row for feature {fid}"),
            })?;
            row_to_values(row, &parsed.columns)
        };

        layer.push(Feature {
            fid: fid as u64,
            geometry: geom,
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

/// Write a [`Layer`] as a MIF/MID dataset.
///
/// `path` may be either:
/// - a `.mif` path, or
/// - a base path without extension.
pub fn write<P: AsRef<Path>>(layer: &Layer, path: P) -> Result<()> {
    let (mif_path, mid_path) = resolve_pair_paths(path.as_ref());
    let (mif_text, mid_text) = to_pair_string(layer)?;
    std::fs::write(&mif_path, mif_text.as_bytes()).map_err(GeoError::Io)?;
    std::fs::write(&mid_path, mid_text.as_bytes()).map_err(GeoError::Io)
}

/// Serialize a [`Layer`] to MIF + MID strings.
pub fn to_pair_string(layer: &Layer) -> Result<(String, String)> {
    let delimiter = ',';

    let mut mif = String::new();
    mif.push_str("Version 300\n");
    mif.push_str("Charset \"Neutral\"\n");
    mif.push_str(&format!("Delimiter \"{}\"\n", delimiter));
    mif.push_str(&coordsys_line_for_layer(layer));

    mif.push_str(&format!("Columns {}\n", layer.schema.len()));
    for fd in layer.schema.fields() {
        mif.push_str("  ");
        mif.push_str(&fd.name);
        mif.push(' ');
        mif.push_str(field_type_to_mif(fd));
        mif.push('\n');
    }

    mif.push_str("Data\n");

    let mut mid = String::new();

    for feat in &layer.features {
        write_geometry_mif(&mut mif, feat.geometry.as_ref())?;
        write_mid_row(&mut mid, &feat.attributes, layer.schema.fields(), delimiter);
    }

    Ok((mif, mid))
}

fn coordsys_line_for_layer(layer: &Layer) -> String {
    if let Some(epsg) = layer.crs_epsg() {
        if epsg == 4326 {
            return "CoordSys Earth Projection 1, 0\n".to_owned();
        }

        // MapInfo MIF supports Transverse Mercator through Earth Projection 8.
        // For common UTM EPSG families, encode equivalent TM parameters.
        if let Some((zone, south)) = utm_zone_from_epsg(epsg) {
            let lon0 = -183.0 + 6.0 * zone as f64;
            let false_northing = if south { 10_000_000.0 } else { 0.0 };
            return format!(
                "CoordSys Earth Projection 8, 104, \"m\", {lon0}, 0, 0.9996, 500000, {false_northing}\n"
            );
        }

        // Keep projected coordinates as projected even when we do not have a
        // specific MIF projection mapping for this EPSG.
        if is_projected_epsg(epsg) {
            return "CoordSys NonEarth Units \"m\"\n".to_owned();
        }
    }

    // Legacy default for unknown CRS metadata.
    "CoordSys Earth Projection 1, 0\n".to_owned()
}

fn utm_zone_from_epsg(epsg: u32) -> Option<(u8, bool)> {
    match epsg {
        // WGS84 / UTM North
        32601..=32660 => Some(((epsg - 32600) as u8, false)),
        // WGS84 / UTM South
        32701..=32760 => Some(((epsg - 32700) as u8, true)),
        // NAD83 / UTM North America zones
        26901..=26923 => Some(((epsg - 26900) as u8, false)),
        // NAD83(CSRS) / UTM zones 14N..19N (includes EPSG:2958 zone 17N)
        2955..=2960 => Some(((epsg - 2941) as u8, false)),
        _ => None,
    }
}

fn is_projected_epsg(epsg: u32) -> bool {
    crate::crs::ogc_wkt_from_epsg(epsg)
        .map(|wkt| wkt.contains("PROJCS[") || wkt.contains("PROJCRS["))
        .unwrap_or(false)
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal parsed representation
// ══════════════════════════════════════════════════════════════════════════════

struct ParsedMif {
    layer_name: String,
    delimiter: char,
    epsg: Option<u32>,
    columns: Vec<FieldDef>,
    geometries: Vec<Option<Geometry>>,
}

fn resolve_pair_paths(path: &Path) -> (PathBuf, PathBuf) {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.eq_ignore_ascii_case("mif") {
            let mut mid = path.to_path_buf();
            mid.set_extension("mid");
            return (path.to_path_buf(), mid);
        }
    }

    let mut mif = path.to_path_buf();
    if mif.extension().is_none() {
        mif.set_extension("mif");
    }
    let mut mid = mif.clone();
    mid.set_extension("mid");
    (mif, mid)
}

fn parse_mif(text: &str) -> Result<ParsedMif> {
    let mut lines = text.lines().enumerate().peekable();

    let mut delimiter = ',';
    let mut epsg: Option<u32> = None;
    let mut columns = Vec::<FieldDef>::new();
    let mut data_started = false;

    while let Some((line_no, raw_line)) = lines.peek().cloned() {
        let line = raw_line.trim();
        if line.is_empty() {
            lines.next();
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if lower.starts_with("delimiter") {
            delimiter = parse_delimiter(line, line_no + 1)?;
            lines.next();
            continue;
        }
        if lower.starts_with("coordsys") {
            epsg = parse_coordsys_epsg(line);
            lines.next();
            continue;
        }
        if lower.starts_with("columns") {
            let (_, count) = parse_columns_header(line, line_no + 1)?;
            lines.next();
            for i in 0..count {
                let (col_line_no, col_line_raw) = lines.next().ok_or_else(|| GeoError::MifParse {
                    line: line_no + 1,
                    msg: format!("expected {count} column lines, found only {i}"),
                })?;
                let fd = parse_column_def(col_line_raw.trim(), col_line_no + 1)?;
                columns.push(fd);
            }
            continue;
        }
        if lower == "data" {
            data_started = true;
            lines.next();
            break;
        }

        lines.next();
    }

    if !data_started {
        return Err(GeoError::MifParse {
            line: 0,
            msg: "missing Data section".into(),
        });
    }

    let mut geometries = Vec::<Option<Geometry>>::new();

    while let Some((line_no, raw_line)) = lines.next() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let geom = parse_geometry_line(line, line_no + 1, &mut lines)?;
        geometries.push(geom);
    }

    Ok(ParsedMif {
        layer_name: "layer".into(),
        delimiter,
        epsg,
        columns,
        geometries,
    })
}

fn parse_delimiter(line: &str, line_no: usize) -> Result<char> {
    let rest = line
        .split_once(' ')
        .map(|(_, r)| r.trim())
        .ok_or_else(|| GeoError::MifParse {
            line: line_no,
            msg: "Delimiter requires a quoted character".into(),
        })?;

    let q = rest.trim();
    if q.len() >= 3 && q.starts_with('"') && q.ends_with('"') {
        let inner = &q[1..q.len() - 1];
        return inner.chars().next().ok_or_else(|| GeoError::MifParse {
            line: line_no,
            msg: "empty Delimiter".into(),
        });
    }

    Err(GeoError::MifParse {
        line: line_no,
        msg: "Delimiter must be quoted".into(),
    })
}

fn parse_coordsys_epsg(line: &str) -> Option<u32> {
    // Practical minimal mapping for common WGS84 MIF.
    let lower = line.to_ascii_lowercase();
    if lower.contains("earth projection 1") {
        Some(4326)
    } else {
        None
    }
}

fn parse_columns_header(line: &str, line_no: usize) -> Result<(&str, usize)> {
    let mut parts = line.split_whitespace();
    let kw = parts.next().unwrap_or("");
    let n_text = parts.next().ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: "Columns requires a count".into(),
    })?;
    let n = n_text.parse::<usize>().map_err(|_| GeoError::MifParse {
        line: line_no,
        msg: format!("invalid column count '{n_text}'"),
    })?;
    Ok((kw, n))
}

fn parse_column_def(line: &str, line_no: usize) -> Result<FieldDef> {
    let mut parts = line.split_whitespace();
    let name = parts.next().ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: "column line missing name".into(),
    })?;
    let ty_text = parts.collect::<Vec<_>>().join(" ");
    if ty_text.is_empty() {
        return Err(GeoError::MifParse {
            line: line_no,
            msg: format!("column '{name}' missing type"),
        });
    }

    let (field_type, width, precision) = mif_type_to_field_type(&ty_text);
    let mut fd = FieldDef::new(name, field_type);
    fd.width = width;
    fd.precision = precision;
    Ok(fd)
}

fn mif_type_to_field_type(ty_text: &str) -> (FieldType, usize, usize) {
    let lower = ty_text.to_ascii_lowercase();

    if lower.starts_with("char") {
        let width = parse_single_paren_int(&lower).unwrap_or(0);
        return (FieldType::Text, width, 0);
    }
    if lower.starts_with("integer") || lower.starts_with("smallint") {
        return (FieldType::Integer, 0, 0);
    }
    if lower.starts_with("decimal") {
        let (w, p) = parse_two_paren_ints(&lower).unwrap_or((0, 0));
        return (FieldType::Float, w, p);
    }
    if lower.starts_with("float") {
        return (FieldType::Float, 0, 0);
    }
    if lower.starts_with("date ") || lower == "date" {
        return (FieldType::Date, 0, 0);
    }
    if lower.starts_with("datetime") {
        return (FieldType::DateTime, 0, 0);
    }
    if lower.starts_with("logical") || lower.starts_with("bool") {
        return (FieldType::Boolean, 0, 0);
    }

    (FieldType::Text, 0, 0)
}

fn parse_single_paren_int(s: &str) -> Option<usize> {
    let a = s.find('(')?;
    let b = s[a + 1..].find(')')? + a + 1;
    s[a + 1..b].trim().parse::<usize>().ok()
}

fn parse_two_paren_ints(s: &str) -> Option<(usize, usize)> {
    let a = s.find('(')?;
    let b = s[a + 1..].find(')')? + a + 1;
    let inner = &s[a + 1..b];
    let mut it = inner.split(',').map(|x| x.trim());
    let w = it.next()?.parse::<usize>().ok()?;
    let p = it.next()?.parse::<usize>().ok()?;
    Some((w, p))
}

fn parse_geometry_line<'a, I>(
    line: &str,
    line_no: usize,
    lines: &mut std::iter::Peekable<I>,
) -> Result<Option<Geometry>>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let lower = line.to_ascii_lowercase();

    if lower == "none" {
        return Ok(None);
    }

    if lower.starts_with("point") {
        let nums = parse_numbers_after_keyword(line, "point", line_no)?;
        if nums.len() < 2 {
            return Err(GeoError::MifParse {
                line: line_no,
                msg: "Point requires x y".into(),
            });
        }
        let c = Coord { x: nums[0], y: nums[1], z: None, m: None };
        return Ok(Some(Geometry::Point(c)));
    }

    if lower.starts_with("line") {
        let nums = parse_numbers_after_keyword(line, "line", line_no)?;
        if nums.len() < 4 {
            return Err(GeoError::MifParse {
                line: line_no,
                msg: "Line requires x1 y1 x2 y2".into(),
            });
        }
        let cs = vec![
            Coord::xy(nums[0], nums[1]),
            Coord::xy(nums[2], nums[3]),
        ];
        return Ok(Some(Geometry::LineString(cs)));
    }

    if lower.starts_with("multipoint") {
        let n = parse_count_after_keyword(line, "multipoint", line_no)?;
        let cs = read_coord_lines(n, lines, line_no)?;
        return Ok(Some(Geometry::MultiPoint(cs)));
    }

    if lower.starts_with("pline") {
        return parse_pline(line, line_no, lines).map(Some);
    }

    if lower.starts_with("region") {
        return parse_region(line, line_no, lines).map(Some);
    }

    Err(GeoError::NotImplemented(format!(
        "MIF geometry statement not supported at line {line_no}: {line}"
    )))
}

fn parse_pline<'a, I>(
    line: &str,
    line_no: usize,
    lines: &mut std::iter::Peekable<I>,
) -> Result<Geometry>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("pline multiple") {
        let n_parts = parse_count_after_keyword(&line[5..], "multiple", line_no)?;
        let mut parts = Vec::<Vec<Coord>>::new();
        for _ in 0..n_parts {
            let (n_line_no, n_line_raw) = lines.next().ok_or_else(|| GeoError::MifParse {
                line: line_no,
                msg: "Pline Multiple missing part point count".into(),
            })?;
            let n = n_line_raw.trim().parse::<usize>().map_err(|_| GeoError::MifParse {
                line: n_line_no + 1,
                msg: format!("invalid Pline part point count '{}': expected integer", n_line_raw.trim()),
            })?;
            let cs = read_coord_lines(n, lines, n_line_no + 1)?;
            parts.push(cs);
        }
        return Ok(Geometry::MultiLineString(parts));
    }

    let n = parse_count_after_keyword(line, "pline", line_no)?;
    let cs = read_coord_lines(n, lines, line_no)?;
    Ok(Geometry::LineString(cs))
}

fn parse_region<'a, I>(
    line: &str,
    line_no: usize,
    lines: &mut std::iter::Peekable<I>,
) -> Result<Geometry>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let n_rings = parse_count_after_keyword(line, "region", line_no)?;
    if n_rings == 0 {
        return Ok(Geometry::MultiPolygon(vec![]));
    }

    let mut rings = Vec::<Ring>::new();
    for _ in 0..n_rings {
        let (n_line_no, n_line_raw) = lines.next().ok_or_else(|| GeoError::MifParse {
            line: line_no,
            msg: "Region missing ring point count".into(),
        })?;
        let n = n_line_raw.trim().parse::<usize>().map_err(|_| GeoError::MifParse {
            line: n_line_no + 1,
            msg: format!("invalid Region ring point count '{}': expected integer", n_line_raw.trim()),
        })?;
        let mut cs = read_coord_lines(n, lines, n_line_no + 1)?;
        close_ring_if_needed(&mut cs);
        rings.push(Ring::new(cs));
    }

    if rings.is_empty() {
        return Ok(Geometry::MultiPolygon(vec![]));
    }

    // MIF Region ring semantics can represent multiple outers and holes, but
    // does not encode explicit hole-parent relationships in a simple way.
    // For this minimal implementation, we preserve all rings by creating one
    // polygon with first ring as exterior and remaining rings as interiors.
    let exterior = rings[0].clone();
    let interiors = rings[1..].to_vec();
    Ok(Geometry::Polygon { exterior, interiors })
}

fn parse_numbers_after_keyword(line: &str, kw: &str, line_no: usize) -> Result<Vec<f64>> {
    let rest = line
        .split_once(char::is_whitespace)
        .map(|(_, r)| r.trim())
        .ok_or_else(|| GeoError::MifParse {
            line: line_no,
            msg: format!("{kw} requires numeric values"),
        })?;

    let mut nums = Vec::new();
    for tok in rest.split_whitespace() {
        let n = tok.parse::<f64>().map_err(|_| GeoError::MifParse {
            line: line_no,
            msg: format!("invalid number '{tok}'"),
        })?;
        nums.push(n);
    }
    Ok(nums)
}

fn parse_count_after_keyword(line: &str, kw: &str, line_no: usize) -> Result<usize> {
    let lower = line.to_ascii_lowercase();
    let pos = lower.find(kw).ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: format!("missing keyword '{kw}'"),
    })?;
    let rest = line[pos + kw.len()..].trim();
    let first = rest.split_whitespace().next().ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: format!("{kw} requires a count"),
    })?;
    first.parse::<usize>().map_err(|_| GeoError::MifParse {
        line: line_no,
        msg: format!("invalid count '{first}'"),
    })
}

fn read_coord_lines<'a, I>(
    n: usize,
    lines: &mut std::iter::Peekable<I>,
    line_no: usize,
) -> Result<Vec<Coord>>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (c_line_no, c_line_raw) = lines.next().ok_or_else(|| GeoError::MifParse {
            line: line_no,
            msg: format!("expected {n} coordinate lines"),
        })?;
        let c = parse_coord_line(c_line_raw.trim(), c_line_no + 1)?;
        out.push(c);
    }
    Ok(out)
}

fn parse_coord_line(line: &str, line_no: usize) -> Result<Coord> {
    let mut it = line.split_whitespace();
    let x_text = it.next().ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: "coordinate line missing x".into(),
    })?;
    let y_text = it.next().ok_or_else(|| GeoError::MifParse {
        line: line_no,
        msg: "coordinate line missing y".into(),
    })?;

    let x = x_text.parse::<f64>().map_err(|_| GeoError::MifParse {
        line: line_no,
        msg: format!("invalid x '{x_text}'"),
    })?;
    let y = y_text.parse::<f64>().map_err(|_| GeoError::MifParse {
        line: line_no,
        msg: format!("invalid y '{y_text}'"),
    })?;

    Ok(Coord::xy(x, y))
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

fn parse_mid_rows(text: &str, delimiter: char) -> Result<Vec<Vec<String>>> {
    let mut out = Vec::<Vec<String>>::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end();
        if line.is_empty() {
            out.push(Vec::new());
            continue;
        }
        out.push(parse_mid_row(line, delimiter, i + 1)?);
    }
    Ok(out)
}

fn parse_mid_row(line: &str, delimiter: char, line_no: usize) -> Result<Vec<String>> {
    let mut vals = Vec::<String>::new();
    let mut buf = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    buf.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                buf.push(ch);
            }
            continue;
        }

        if ch == '"' {
            in_quotes = true;
            continue;
        }

        if ch == delimiter {
            vals.push(buf.trim().to_owned());
            buf.clear();
            continue;
        }

        buf.push(ch);
    }

    if in_quotes {
        return Err(GeoError::MifParse {
            line: line_no,
            msg: "unterminated quoted MID field".into(),
        });
    }

    vals.push(buf.trim().to_owned());
    Ok(vals)
}

fn row_to_values(row: &[String], columns: &[FieldDef]) -> Vec<FieldValue> {
    let mut out = vec![FieldValue::Null; columns.len()];

    for (i, fd) in columns.iter().enumerate() {
        let raw = row.get(i).map(String::as_str).unwrap_or("").trim();
        if raw.is_empty() {
            continue;
        }
        out[i] = parse_value_as_type(raw, fd.field_type);
    }

    out
}

fn parse_value_as_type(raw: &str, ty: FieldType) -> FieldValue {
    match ty {
        FieldType::Integer => raw
            .parse::<i64>()
            .map(FieldValue::Integer)
            .unwrap_or_else(|_| FieldValue::Text(raw.to_owned())),
        FieldType::Float => raw
            .parse::<f64>()
            .map(FieldValue::Float)
            .unwrap_or_else(|_| FieldValue::Text(raw.to_owned())),
        FieldType::Boolean => {
            let lower = raw.to_ascii_lowercase();
            match lower.as_str() {
                "true" | "t" | "1" | "yes" => FieldValue::Boolean(true),
                "false" | "f" | "0" | "no" => FieldValue::Boolean(false),
                _ => FieldValue::Text(raw.to_owned()),
            }
        }
        FieldType::Date => FieldValue::Date(raw.to_owned()),
        FieldType::DateTime => FieldValue::DateTime(raw.to_owned()),
        _ => FieldValue::Text(raw.to_owned()),
    }
}

fn field_type_to_mif(fd: &FieldDef) -> &'static str {
    match fd.field_type {
        FieldType::Integer => "Integer",
        FieldType::Float => "Float",
        FieldType::Boolean => "Logical",
        FieldType::Date => "Date",
        FieldType::DateTime => "DateTime",
        _ => "Char(254)",
    }
}

fn write_geometry_mif(out: &mut String, geom: Option<&Geometry>) -> Result<()> {
    match geom {
        None => {
            out.push_str("None\n");
        }
        Some(Geometry::Point(c)) => {
            out.push_str(&format!("Point {} {}\n", c.x, c.y));
        }
        Some(Geometry::LineString(cs)) => {
            if cs.len() == 2 {
                out.push_str(&format!(
                    "Line {} {} {} {}\n",
                    cs[0].x, cs[0].y, cs[1].x, cs[1].y
                ));
            } else {
                out.push_str(&format!("Pline {}\n", cs.len()));
                for c in cs {
                    out.push_str(&format!("{} {}\n", c.x, c.y));
                }
            }
        }
        Some(Geometry::Polygon { exterior, interiors }) => {
            let ring_count = 1 + interiors.len();
            out.push_str(&format!("Region {}\n", ring_count));

            write_ring_mif(out, &exterior.0);
            for ring in interiors {
                write_ring_mif(out, &ring.0);
            }
        }
        Some(Geometry::MultiPoint(pts)) => {
            out.push_str(&format!("Multipoint {}\n", pts.len()));
            for c in pts {
                out.push_str(&format!("{} {}\n", c.x, c.y));
            }
        }
        Some(Geometry::MultiLineString(lines)) => {
            out.push_str(&format!("Pline Multiple {}\n", lines.len()));
            for line in lines {
                out.push_str(&format!("{}\n", line.len()));
                for c in line {
                    out.push_str(&format!("{} {}\n", c.x, c.y));
                }
            }
        }
        Some(Geometry::MultiPolygon(polys)) => {
            let mut rings = Vec::<&[Coord]>::new();
            for (ext, holes) in polys {
                rings.push(&ext.0);
                for hole in holes {
                    rings.push(&hole.0);
                }
            }

            out.push_str(&format!("Region {}\n", rings.len()));
            for ring in rings {
                write_ring_mif(out, ring);
            }
        }
        Some(Geometry::GeometryCollection(_)) => {
            out.push_str("None\n");
        }
    }

    Ok(())
}

fn write_ring_mif(out: &mut String, coords: &[Coord]) {
    let mut ring = coords.to_vec();
    close_ring_if_needed(&mut ring);
    out.push_str(&format!("{}\n", ring.len()));
    for c in ring {
        out.push_str(&format!("{} {}\n", c.x, c.y));
    }
}

fn write_mid_row(out: &mut String, attrs: &[FieldValue], schema: &[FieldDef], delimiter: char) {
    for i in 0..schema.len() {
        if i > 0 {
            out.push(delimiter);
        }

        let v = attrs.get(i).cloned().unwrap_or(FieldValue::Null);
        let text = match v {
            FieldValue::Null => String::new(),
            FieldValue::Integer(n) => n.to_string(),
            FieldValue::Float(n) => n.to_string(),
            FieldValue::Boolean(b) => {
                if b { "T".into() } else { "F".into() }
            }
            FieldValue::Date(s) | FieldValue::DateTime(s) | FieldValue::Text(s) => {
                quote_mid(&s)
            }
            FieldValue::Blob(b) => quote_mid(&format!("<blob {} bytes>", b.len())),
        };

        out.push_str(&text);
    }
    out.push('\n');
}

fn quote_mid(s: &str) -> String {
    let escaped = s.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mif_mid_roundtrip_basic() {
        let mut layer = Layer::new("roads")
            .with_geom_type(GeometryType::LineString)
            .with_epsg(4326);
        layer.add_field(FieldDef::new("name", FieldType::Text));
        layer.add_field(FieldDef::new("speed", FieldType::Integer));

        layer
            .add_feature(
                Some(Geometry::line_string(vec![
                    Coord::xy(-1.0, 50.0),
                    Coord::xy(0.0, 51.0),
                    Coord::xy(1.0, 52.0),
                ])),
                &[("name", "A-Road".into()), ("speed", 80i64.into())],
            )
            .unwrap();

        let (mif, mid) = to_pair_string(&layer).unwrap();
        let parsed = parse_pair_str(&mif, &mid).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.schema.len(), 2);
        assert_eq!(parsed.crs_epsg(), Some(4326));
        assert!(matches!(parsed.features[0].geometry, Some(Geometry::LineString(_))));
        assert_eq!(parsed.features[0].get(&parsed.schema, "name").unwrap().as_str(), Some("A-Road"));
        assert_eq!(parsed.features[0].get(&parsed.schema, "speed").unwrap().as_i64(), Some(80));
    }

    #[test]
    fn parses_pline_multiple() {
        let mif = r#"
Version 300
Charset "Neutral"
Delimiter ","
CoordSys Earth Projection 1, 0
Columns 1
  id Integer
Data
Pline Multiple 2
2
0 0
1 1
2
1 1
2 2
"#;
        let mid = "1\n";

        let layer = parse_pair_str(mif, mid).unwrap();
        assert_eq!(layer.crs_epsg(), Some(4326));
        assert!(matches!(layer.features[0].geometry, Some(Geometry::MultiLineString(_))));
    }

    #[test]
    fn parses_region_polygon() {
        let mif = r#"
Version 300
Charset "Neutral"
Delimiter ","
CoordSys Earth Projection 1, 0
Columns 1
  name Char(20)
Data
Region 2
4
0 0
10 0
10 10
0 10
4
2 2
3 2
3 3
2 3
"#;
        let mid = "\"poly\"\n";

        let layer = parse_pair_str(mif, mid).unwrap();
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
    fn write_uses_projected_coordsys_for_epsg2958() {
        let mut layer = Layer::new("subset").with_epsg(2958);
        layer.add_field(FieldDef::new("id", FieldType::Integer));
        layer
            .add_feature(
                Some(Geometry::point(560000.0, 4_820_000.0)),
                &[("id", 1i64.into())],
            )
            .unwrap();

        let (mif, _mid) = to_pair_string(&layer).unwrap();
        assert!(
            mif.contains("CoordSys Earth Projection 8, 104, \"m\", -81, 0, 0.9996, 500000, 0"),
            "unexpected CoordSys line in MIF header:\n{mif}"
        );
    }

    #[test]
    fn write_uses_nonearth_for_other_projected_epsg() {
        let mut layer = Layer::new("mercator").with_epsg(3857);
        layer.add_field(FieldDef::new("id", FieldType::Integer));
        layer
            .add_feature(
                Some(Geometry::point(1_000_000.0, 5_000_000.0)),
                &[("id", 1i64.into())],
            )
            .unwrap();

        let (mif, _mid) = to_pair_string(&layer).unwrap();
        assert!(
            mif.contains("CoordSys NonEarth Units \"m\""),
            "expected NonEarth CoordSys for projected EPSG without explicit map, got:\n{mif}"
        );
    }
}
