//! ENVI HDR Labelled Raster format.
//!
//! Two files:
//! - `<name>.hdr` — plain-text header
//! - `<name>.img` (or `.dat`, `.bin`, etc.) — binary data
//!
//! ENVI header reference:
//! <https://www.l3harrisgeospatial.com/docs/enviheaderfiles.html>
//!
//! Key header fields:
//! ```text
//! ENVI
//! description = { <text> }
//! samples = 300
//! lines   = 200
//! bands   = 1
//! header offset = 0
//! file type = ENVI Standard
//! data type = 4              (1=byte,2=int16,3=int32,4=float32,5=float64,12=uint16,13=uint32)
//! interleave = bsq           (bsq|bil|bip)
//! sensor type = Unknown
//! byte order = 0             (0=little-endian, 1=big-endian)
//! map info = {Geographic Lat/Lon, 1, 1, 100.0, -30.0, 0.01, 0.01, WGS-84}
//! coordinate system string = {GEOGCS[...]}
//! data ignore value = -9999
//! ```
//!
//! The `map info` field layout (comma-separated, braces stripped):
//!   `{projection, ref_pixel_x, ref_pixel_y, ref_easting, ref_northing, x_pixel_size, y_pixel_size, [datum], [units]}`
//!
//! We support BSQ, BIL, and BIP interleaves.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::*;
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an ENVI raster. `path` may be the `.hdr` file or the data file.
pub fn read(path: &str) -> Result<Raster> {
    let hdr_path = if extension_lower(path) == "hdr" {
        path.to_string()
    } else {
        with_extension(path, "hdr")
    };
    read_from_header(&hdr_path)
}

/// Write an ENVI raster. `path` should be the `.hdr` path, or a base path.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let hdr_path = if extension_lower(path) == "hdr" {
        path.to_string()
    } else {
        with_extension(path, "hdr")
    };
    let data_path = envi_data_path(&hdr_path);
    write_header(raster, &hdr_path)?;
    write_data(raster, &data_path)
}

// ─── Header ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct EnviHeader {
    samples: usize,
    lines: usize,
    bands: usize,
    header_offset: u64,
    data_type: DataType,
    interleave: Interleave,
    byte_order_le: bool,
    x_min: f64,
    y_min: f64,
    cell_size_x: f64,
    cell_size_y: f64,
    nodata: f64,
    crs: CrsInfo,
    map_projection: Option<String>,
    map_datum: Option<String>,
    map_units: Option<String>,
    description: String,
    data_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Interleave { #[default] Bsq, Bil, Bip }

fn parse_envi_header(path: &str) -> Result<EnviHeader> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut samples: Option<usize> = None;
    let mut lines: Option<usize> = None;
    let mut bands = 1usize;
    let mut header_offset = 0u64;
    let mut data_type = DataType::F32;
    let mut interleave = Interleave::Bsq;
    let mut byte_order_le = true;
    let mut x_min = 0.0_f64;
    let mut y_min = 0.0_f64;
    let mut cell_size_x = 1.0_f64;
    let mut cell_size_y = 1.0_f64;
    let mut nodata = -9999.0_f64;
    let mut crs = CrsInfo::default();
    let mut map_projection: Option<String> = None;
    let mut map_datum: Option<String> = None;
    let mut map_units: Option<String> = None;
    let mut description = String::new();
    let mut data_file: Option<String> = None;

    // ENVI headers can have multi-line values wrapped in `{ ... }`
    let mut all_text = String::new();
    for line in reader.lines() {
        let l = line?;
        all_text.push_str(&l);
        all_text.push('\n');
    }

    // First line must be "ENVI"
    let first_line = all_text.lines().next().unwrap_or("").trim();
    if !first_line.eq_ignore_ascii_case("ENVI") {
        return Err(RasterError::CorruptData(
            format!("not an ENVI header: first line is '{first_line}'")
        ));
    }

    // Flatten multi-line brace values
    let flat = flatten_envi_braces(&all_text);

    for line in flat.lines() {
        let line = line.trim();
        if line.is_empty() || line.eq_ignore_ascii_case("ENVI") { continue; }

        let eq_pos = match line.find('=') {
            Some(p) => p,
            None => continue,
        };
        let key = line[..eq_pos].trim().to_ascii_lowercase();
        let val = line[eq_pos + 1..].trim().to_string();
        // Strip surrounding braces if present
        let val = if val.starts_with('{') && val.ends_with('}') {
            val[1..val.len()-1].trim().to_string()
        } else {
            val
        };

        match key.as_str() {
            "samples"             => samples       = Some(parse_usize_h("samples", &val)?),
            "lines"               => lines         = Some(parse_usize_h("lines", &val)?),
            "bands"               => bands         = parse_usize_h("bands", &val)?,
            "header offset"       => header_offset = val.trim().parse::<u64>().unwrap_or(0),
            "data type"           => data_type     = envi_data_type(&val),
            "interleave"          => interleave    = parse_interleave(&val),
            "byte order"          => byte_order_le = val.trim() == "0",
            "data ignore value"   => nodata        = val.trim().parse::<f64>().unwrap_or(-9999.0),
            "description"         => description   = val,
            "coordinate system string" => crs      = CrsInfo::from_wkt(&val),
            "data file"           => data_file     = Some(val),
            "map info"            => {
                parse_map_info(
                    &val,
                    &mut x_min,
                    &mut y_min,
                    &mut cell_size_x,
                    &mut cell_size_y,
                    &mut map_projection,
                    &mut map_datum,
                    &mut map_units,
                );
            }
            _ => {}
        }
    }

    let samples = samples.ok_or_else(|| RasterError::MissingField("samples".into()))?;
    let lines   = lines  .ok_or_else(|| RasterError::MissingField("lines".into()))?;

    Ok(EnviHeader {
        samples,
        lines,
        bands,
        header_offset,
        data_type,
        interleave,
        byte_order_le,
        x_min,
        y_min,
        cell_size_x,
        cell_size_y,
        nodata,
        crs: crs,        map_projection,
        map_datum,
        map_units,
        description,
        data_file,
    })
}

/// Collapse multi-line `{ ... }` blocks in an ENVI header into single lines.
fn flatten_envi_braces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_braces = false;
    for ch in text.chars() {
        match ch {
            '{' => { in_braces = true; out.push('{'); }
            '}' => { in_braces = false; out.push('}'); }
            '\n' if in_braces => { out.push(' '); } // fold line breaks inside braces
            c => { out.push(c); }
        }
    }
    out
}

fn envi_data_type(s: &str) -> DataType {
    match s.trim() {
        "1"  => DataType::U8,
        "2"  => DataType::I16,
        "3"  => DataType::I32,
        "4"  => DataType::F32,
        "5"  => DataType::F64,
        "12" => DataType::U16,
        "13" => DataType::U32,
        _    => DataType::F32,
    }
}

fn envi_data_type_code(dt: DataType) -> u8 {
    match dt {
        DataType::U8  => 1,
        DataType::I16 => 2,
        DataType::I32 => 3,
        DataType::F32 => 4,
        DataType::F64 => 5,
        DataType::U16 => 12,
        DataType::U32 => 13,
        DataType::I64 => 14,
        DataType::U64 => 15,
        DataType::I8  => 1, // promote
    }
}

fn parse_interleave(s: &str) -> Interleave {
    match s.trim().to_ascii_lowercase().as_str() {
        "bil" => Interleave::Bil,
        "bip" => Interleave::Bip,
        _     => Interleave::Bsq,
    }
}

/// Parse `map info = {projection, ref_x, ref_y, easting, northing, x_dim, y_dim, ...}`
#[allow(clippy::too_many_arguments)]
fn parse_map_info(
    val: &str,
    x_min: &mut f64,
    y_min: &mut f64,
    x_dim: &mut f64,
    y_dim: &mut f64,
    map_projection: &mut Option<String>,
    map_datum: &mut Option<String>,
    map_units: &mut Option<String>,
) {
    let parts: Vec<&str> = val.split(',').collect();
    // [0]=projection, [1]=ref_pixel_x (1-based), [2]=ref_pixel_y (1-based),
    // [3]=ref_easting, [4]=ref_northing, [5]=x_dim, [6]=y_dim
    if parts.len() < 7 { return; }
    let ref_px: f64 = parts[1].trim().parse().unwrap_or(1.0);
    let ref_py: f64 = parts[2].trim().parse().unwrap_or(1.0);
    let ref_e:  f64 = parts[3].trim().parse().unwrap_or(0.0);
    let ref_n:  f64 = parts[4].trim().parse().unwrap_or(0.0);
    let xd:     f64 = parts[5].trim().parse().unwrap_or(1.0);
    let yd:     f64 = parts[6].trim().parse().unwrap_or(1.0);
    // ENVI map info gives upper-left corner of ref pixel (1-based index)
    // x_min = ref_e - (ref_px - 1) * xd
    // y_max = ref_n + (ref_py - 1) * yd   (Northings increases upward)
    // y_min is computed later from y_max - rows*yd
    *x_dim = xd.abs();
    *y_dim = yd.abs();
    *x_min = ref_e - (ref_px - 1.0) * xd.abs();
    // Temporarily store y_max in y_min; will be corrected after reading rows
    *y_min = ref_n + (ref_py - 1.0) * yd.abs(); // this is y_max

    let projection = parts[0].trim();
    if !projection.is_empty() {
        *map_projection = Some(projection.to_string());
    }
    if parts.len() >= 8 {
        let datum = parts[7].trim();
        if !datum.is_empty() {
            *map_datum = Some(datum.to_string());
        }
    }
    if parts.len() >= 11 {
        let units = parts[10].trim();
        if !units.is_empty() {
            *map_units = Some(units.to_string());
        }
    }
}

// ─── Read ─────────────────────────────────────────────────────────────────────

fn envi_data_path(hdr_path: &str) -> String {
    // Try common data extensions
    let base = hdr_path.trim_end_matches(".hdr").trim_end_matches(".HDR");
    for ext in &["img", "dat", "bin", "raw", ""] {
        let candidate = if ext.is_empty() { base.to_string() } else { format!("{base}.{ext}") };
        if std::path::Path::new(&candidate).exists() {
            return candidate;
        }
    }
    format!("{base}.img") // default fallback
}

fn read_from_header(hdr_path: &str) -> Result<Raster> {
    let hdr = parse_envi_header(hdr_path)?;

    let data_path = if let Some(ref f) = hdr.data_file {
        let dir = std::path::Path::new(hdr_path).parent()
            .map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".into());
        format!("{dir}/{f}")
    } else {
        envi_data_path(hdr_path)
    };

    let data = read_data(&data_path, &hdr)?;

    // y_min was stored as y_max; correct it
    let y_max = hdr.y_min; // temporarily stored
    let y_min = y_max - hdr.lines as f64 * hdr.cell_size_y;

    let mut metadata: Vec<(String, String)> = vec![
        ("description".into(), hdr.description),
        (
            "envi_interleave".into(),
            match hdr.interleave {
                Interleave::Bsq => "bsq".into(),
                Interleave::Bil => "bil".into(),
                Interleave::Bip => "bip".into(),
            },
        ),
    ];
    if let Some(mp) = hdr.map_projection {
        metadata.push(("envi_map_projection".into(), mp));
    }
    if let Some(md) = hdr.map_datum {
        metadata.push(("envi_map_datum".into(), md));
    }
    if let Some(mu) = hdr.map_units {
        metadata.push(("envi_map_units".into(), mu));
    }
    if let Some(ref wkt) = hdr.crs.wkt {
        metadata.push(("envi_coordinate_system_string".into(), wkt.clone()));
    }

    let cfg = RasterConfig {
        cols: hdr.samples,
        rows: hdr.lines,
        bands: hdr.bands,
        x_min: hdr.x_min,
        y_min,
        cell_size: hdr.cell_size_x,
        cell_size_y: Some(hdr.cell_size_y),
        nodata: hdr.nodata,
        data_type: hdr.data_type,
        crs: hdr.crs,
        metadata,
    };
    Raster::from_data(cfg, data)
}

fn read_data(path: &str, hdr: &EnviHeader) -> Result<Vec<f64>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = BufReader::with_capacity(512 * 1024, File::open(path)?);
    if hdr.header_offset > 0 {
        file.seek(SeekFrom::Start(hdr.header_offset))?;
    }

    let n = hdr.samples * hdr.lines * hdr.bands;
    let le = hdr.byte_order_le;
    let mut linear = Vec::with_capacity(n);

    match hdr.data_type {
        DataType::U8 => {
            let mut buf = vec![0u8; n];
            file.read_exact(&mut buf)?;
            linear.extend(buf.iter().map(|&b| b as f64));
        }
        DataType::I16 => {
            for _ in 0..n {
                let v = if le { read_i16_le_stream(&mut file)? } else { read_i16_be_stream(&mut file)? };
                linear.push(v as f64);
            }
        }
        DataType::I32 => {
            for _ in 0..n {
                let v = if le { read_i32_le_stream(&mut file)? } else { read_i32_be_stream(&mut file)? };
                linear.push(v as f64);
            }
        }
        DataType::U16 => {
            for _ in 0..n {
                let mut b = [0u8; 2];
                file.read_exact(&mut b)?;
                let v = if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) };
                linear.push(v as f64);
            }
        }
        DataType::U32 => {
            for _ in 0..n {
                let mut b = [0u8; 4];
                file.read_exact(&mut b)?;
                let v = if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) };
                linear.push(v as f64);
            }
        }
        DataType::F32 => {
            for _ in 0..n {
                let v = if le { read_f32_le_stream(&mut file)? } else { read_f32_be_stream(&mut file)? };
                linear.push(v as f64);
            }
        }
        DataType::F64 => {
            for _ in 0..n {
                let v = if le { read_f64_le_stream(&mut file)? } else { read_f64_be_stream(&mut file)? };
                linear.push(v);
            }
        }
        _ => return Err(RasterError::UnsupportedDataType(hdr.data_type.to_string())),
    }

    let cols = hdr.samples;
    let rows = hdr.lines;
    let bands = hdr.bands;
    let mut data = vec![hdr.nodata; n];
    for band in 0..bands {
        for row in 0..rows {
            for col in 0..cols {
                let src_idx = match hdr.interleave {
                    Interleave::Bsq => band * rows * cols + row * cols + col,
                    Interleave::Bil => row * bands * cols + band * cols + col,
                    Interleave::Bip => row * cols * bands + col * bands + band,
                };
                let dst_idx = band * rows * cols + row * cols + col;
                data[dst_idx] = linear[src_idx];
            }
        }
    }

    Ok(data)
}

// ─── Write ────────────────────────────────────────────────────────────────────

fn write_header(raster: &Raster, hdr_path: &str) -> Result<()> {
    let data_basename = {
        let dp = envi_data_path(hdr_path);
        std::path::Path::new(&dp)
            .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    };
    let desc = raster.metadata.iter()
        .find(|(k, _)| k == "description")
        .map(|(_, v)| v.as_str()).unwrap_or("Created by gis_raster");
    let interleave = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "envi_interleave")
        .map(|(_, v)| v.to_ascii_lowercase())
        .and_then(|v| match v.as_str() {
            "bsq" => Some(Interleave::Bsq),
            "bil" => Some(Interleave::Bil),
            "bip" => Some(Interleave::Bip),
            _ => None,
        })
        .unwrap_or(Interleave::Bsq);

    let md = |key: &str| {
        raster
            .metadata
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    };

    // map info: upper-left corner of pixel (1,1) = (x_min, y_max)
    let y_max = raster.y_max();
    let projection = md("envi_map_projection").unwrap_or("Geographic Lat/Lon");
    let map_datum = md("envi_map_datum");
    let map_units = md("envi_map_units");

    let mut w = BufWriter::new(File::create(hdr_path)?);
    writeln!(w, "ENVI")?;
    writeln!(w, "description = {{{desc}}}")?;
    writeln!(w, "samples = {}", raster.cols)?;
    writeln!(w, "lines   = {}", raster.rows)?;
    writeln!(w, "bands   = {}", raster.bands)?;
    writeln!(w, "header offset = 0")?;
    writeln!(w, "file type = ENVI Standard")?;
    writeln!(w, "data type = {}", envi_data_type_code(raster.data_type))?;
    writeln!(
        w,
        "interleave = {}",
        match interleave {
            Interleave::Bsq => "bsq",
            Interleave::Bil => "bil",
            Interleave::Bip => "bip",
        }
    )?;
    writeln!(w, "sensor type = Unknown")?;
    writeln!(w, "byte order = 0")?;
    let mut map_info = format!(
        "{projection}, 1, 1, {}, {}, {}, {}",
        format_float(raster.x_min, 10),
        format_float(y_max, 10),
        format_float(raster.cell_size_x, 10),
        format_float(raster.cell_size_y, 10),
    );
    if let Some(d) = map_datum {
        map_info.push_str(", ");
        map_info.push_str(d);
        if let Some(u) = map_units {
            map_info.push_str(", units=");
            map_info.push_str(u);
        }
    }
    writeln!(w, "map info = {{{map_info}}}")?;
    if let Some(ref wkt) = raster.crs.wkt {
        writeln!(w, "coordinate system string = {{{wkt}}}")?;
    } else if let Some(cs) = md("envi_coordinate_system_string") {
        writeln!(w, "coordinate system string = {{{cs}}}")?;
    }
    writeln!(w, "data ignore value = {}", format_float(raster.nodata, 6))?;
    writeln!(w, "data file = {data_basename}")?;
    w.flush()?;
    Ok(())
}

fn write_data(raster: &Raster, path: &str) -> Result<()> {
    let mut w = BufWriter::with_capacity(512 * 1024, File::create(path)?);
    let interleave = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "envi_interleave")
        .map(|(_, v)| v.to_ascii_lowercase())
        .unwrap_or_else(|| "bsq".to_string());

    let bands = raster.bands;
    let rows = raster.rows;
    let cols = raster.cols;
    let mut out = Vec::with_capacity(rows * cols * bands * raster.data_type.size_bytes());
    match interleave.as_str() {
        "bil" => {
            for row in 0..rows {
                for band in 0..bands {
                    for col in 0..cols {
                        let v = raster
                            .get_raw(band as isize, row as isize, col as isize)
                            .unwrap_or(raster.nodata);
                        append_sample_le(&mut out, raster.data_type, v);
                    }
                }
            }
        }
        "bip" => {
            for row in 0..rows {
                for col in 0..cols {
                    for band in 0..bands {
                        let v = raster
                            .get_raw(band as isize, row as isize, col as isize)
                            .unwrap_or(raster.nodata);
                        append_sample_le(&mut out, raster.data_type, v);
                    }
                }
            }
        }
        _ => {
            for band in 0..bands {
                for row in 0..rows {
                    for col in 0..cols {
                        let v = raster
                            .get_raw(band as isize, row as isize, col as isize)
                            .unwrap_or(raster.nodata);
                        append_sample_le(&mut out, raster.data_type, v);
                    }
                }
            }
        }
    }
    w.write_all(&out)?;
    w.flush()?;
    Ok(())
}

fn append_sample_le(out: &mut Vec<u8>, dt: DataType, v: f64) {
    match dt {
        DataType::U8 => out.push(v as u8),
        DataType::I8 => out.push((v as i8) as u8),
        DataType::U16 => out.extend_from_slice(&(v as u16).to_le_bytes()),
        DataType::I16 => out.extend_from_slice(&(v as i16).to_le_bytes()),
        DataType::U32 => out.extend_from_slice(&(v as u32).to_le_bytes()),
        DataType::I32 => out.extend_from_slice(&(v as i32).to_le_bytes()),
        DataType::U64 => out.extend_from_slice(&(v as u64).to_le_bytes()),
        DataType::I64 => out.extend_from_slice(&(v as i64).to_le_bytes()),
        DataType::F32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
        DataType::F64 => out.extend_from_slice(&v.to_le_bytes()),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_usize_h(field: &str, val: &str) -> Result<usize> {
    val.trim().parse::<usize>().map_err(|_| RasterError::ParseError {
        field: field.into(), value: val.into(), expected: "positive integer".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::RasterConfig;
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(suffix: &str) -> String {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        temp_dir().join(format!("envi_test_{ts}{suffix}")).to_string_lossy().into_owned()
    }

    #[test]
    fn envi_roundtrip() {
        let hdr = tmp(".hdr");
        let cfg = RasterConfig {
            cols: 5, rows: 4, cell_size: 0.5, x_min: 10.0, y_min: 20.0,
            nodata: -9999.0, data_type: DataType::F32, ..Default::default()
        };
        let data: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &hdr).unwrap();
        let r2 = read(&hdr).unwrap();
        assert_eq!(r2.cols, 5);
        assert_eq!(r2.rows, 4);
        assert!((r2.get(0, 0, 0) - 0.0).abs() < 1e-4, "got {:?}", r2.get(0, 0, 0));
        assert!((r2.get(0, 3, 4) - 19.0).abs() < 1e-3, "got {:?}", r2.get(0, 3, 4));
        let img = with_extension(&hdr, "img");
        let _ = std::fs::remove_file(&hdr);
        let _ = std::fs::remove_file(&img);
    }

    #[test]
    fn envi_roundtrip_multiband_bil() {
        let hdr = tmp("_mb.hdr");
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            bands: 3,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
            .map(|i| i as f64)
            .collect();
        let mut r = Raster::from_data(cfg, data).unwrap();
        r.metadata.push(("envi_interleave".into(), "bil".into()));

        write(&r, &hdr).unwrap();
        let r2 = read(&hdr).unwrap();

        assert_eq!(r2.bands, 3);
        assert_eq!(r2.get_raw(0, 0, 0), Some(0.0));
        assert_eq!(r2.get_raw(1, 0, 0), Some(12.0));
        assert_eq!(r2.get_raw(2, 2, 3), Some(35.0));

        let img = with_extension(&hdr, "img");
        let _ = std::fs::remove_file(&hdr);
        let _ = std::fs::remove_file(&img);
    }

    #[test]
    fn envi_preserves_map_info_metadata() {
        let hdr = tmp("_map.hdr");
        let mut cfg = RasterConfig {
            cols: 3,
            rows: 2,
            cell_size: 2.0,
            x_min: 100.0,
            y_min: 200.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        cfg.metadata.push(("envi_map_projection".into(), "UTM".into()));
        cfg.metadata.push(("envi_map_datum".into(), "WGS-84".into()));
        cfg.metadata.push(("envi_map_units".into(), "Meters".into()));
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

        write(&r, &hdr).unwrap();
        let r2 = read(&hdr).unwrap();
        assert_eq!(
            r2.metadata
                .iter()
                .find(|(k, _)| k == "envi_map_projection")
                .map(|(_, v)| v.as_str()),
            Some("UTM")
        );
        assert_eq!(
            r2.metadata
                .iter()
                .find(|(k, _)| k == "envi_map_datum")
                .map(|(_, v)| v.as_str()),
            Some("WGS-84")
        );

        let img = with_extension(&hdr, "img");
        let _ = std::fs::remove_file(&hdr);
        let _ = std::fs::remove_file(&img);
    }

    #[test]
    fn envi_writes_coordinate_system_from_metadata_when_srs_empty() {
        let hdr = tmp("_cs.hdr");
        let mut cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let wkt = "GEOGCS[\"WGS 84\"]";
        cfg.metadata
            .push(("envi_coordinate_system_string".into(), wkt.into()));
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &hdr).unwrap();

        let text = std::fs::read_to_string(&hdr).unwrap();
        assert!(text.contains("coordinate system string = {GEOGCS[\"WGS 84\"]}"));

        let img = with_extension(&hdr, "img");
        let _ = std::fs::remove_file(&hdr);
        let _ = std::fs::remove_file(&img);
    }

    #[test]
    fn flatten_braces() {
        let text = "map info = {\nGeo,\n1, 1,\n100.0,\n-30.0\n}\n";
        let flat = flatten_envi_braces(text);
        assert!(!flat.contains('\n') || flat.lines().count() <= 3);
    }
}
