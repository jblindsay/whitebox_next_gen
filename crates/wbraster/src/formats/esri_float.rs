//! Esri Binary Float Grid format (`.flt` + `.hdr`).
//!
//! An Esri Float Grid consists of two files:
//! - `<name>.hdr` — plain-text header with the same keywords as Esri ASCII Grid,
//!   plus an optional `BYTEORDER` field (`LSBFIRST` or `MSBFIRST`).
//! - `<name>.flt` — raw binary 32-bit IEEE 754 floats, row-major, north to south.
//!
//! The format is identical in spatial semantics to Esri ASCII Grid, only the
//! data encoding differs.
//!
//! Reference:
//! <https://desktop.arcgis.com/en/arcmap/latest/manage-data/raster-and-images/esri-float-raster-format.htm>

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use crate::crs_info::CrsInfo;
use crate::error::{Result, RasterError};
use crate::io_utils::{format_float, parse_key_value, with_extension};
use crate::raster::{DataType, Raster, RasterConfig};

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an Esri Float Grid. `path` may be either the `.flt` or `.hdr` file.
pub fn read(path: &str) -> Result<Raster> {
    let lower = path.to_ascii_lowercase();
    let hdr_path = if lower.ends_with(".hdr") {
        path.to_string()
    } else {
        with_extension(path, "hdr")
    };
    let flt_path = with_extension(&hdr_path, "flt");
    read_from_header(&hdr_path, &flt_path)
}

/// Write an Esri Float Grid. `path` should be the `.flt` or `.hdr` file path;
/// both sidecar files are written automatically.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "Esri Float Grid writer supports single-band rasters only".into(),
        ));
    }
    let lower = path.to_ascii_lowercase();
    let hdr_path = if lower.ends_with(".hdr") {
        path.to_string()
    } else {
        with_extension(path, "hdr")
    };
    let flt_path = with_extension(&hdr_path, "flt");
    write_header(raster, &hdr_path)?;
    write_data(raster, &flt_path)?;
    write_prj_sidecar(raster, &hdr_path)
}

// ─── Internal ─────────────────────────────────────────────────────────────────

fn read_from_header(hdr_path: &str, flt_path: &str) -> Result<Raster> {
    let file = File::open(hdr_path)?;
    let reader = BufReader::new(file);

    let mut cols: Option<usize> = None;
    let mut rows: Option<usize> = None;
    let mut xll: Option<f64> = None;
    let mut yll: Option<f64> = None;
    let mut xll_is_corner = true;
    let mut yll_is_corner = true;
    let mut cell_size: Option<f64> = None;
    let mut nodata: f64 = -9999.0;
    let mut little_endian = true; // LSBFIRST is the default

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, val)) = parse_key_value(trimmed) {
            match key.as_str() {
                "ncols" => cols = Some(parse_usize(&key, &val)?),
                "nrows" => rows = Some(parse_usize(&key, &val)?),
                "xllcorner" => {
                    xll = Some(parse_f64(&key, &val)?);
                    xll_is_corner = true;
                }
                "xllcenter" => {
                    xll = Some(parse_f64(&key, &val)?);
                    xll_is_corner = false;
                }
                "yllcorner" => {
                    yll = Some(parse_f64(&key, &val)?);
                    yll_is_corner = true;
                }
                "yllcenter" => {
                    yll = Some(parse_f64(&key, &val)?);
                    yll_is_corner = false;
                }
                "cellsize" => cell_size = Some(parse_f64(&key, &val)?),
                "nodata_value" | "nodata" => nodata = parse_f64(&key, &val)?,
                "byteorder" => {
                    little_endian = !val.trim().eq_ignore_ascii_case("MSBFIRST");
                }
                _ => {}
            }
        }
    }

    let cols = cols.ok_or_else(|| RasterError::MissingField("NCOLS".into()))?;
    let rows = rows.ok_or_else(|| RasterError::MissingField("NROWS".into()))?;
    let cs = cell_size.ok_or_else(|| RasterError::MissingField("CELLSIZE".into()))?;
    let xll = xll.ok_or_else(|| RasterError::MissingField("XLLCORNER/XLLCENTER".into()))?;
    let yll = yll.ok_or_else(|| RasterError::MissingField("YLLCORNER/YLLCENTER".into()))?;

    let x_min = if xll_is_corner { xll } else { xll - cs * 0.5 };
    let y_min = if yll_is_corner { yll } else { yll - cs * 0.5 };

    if cols == 0 || rows == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }

    // Read binary data — raw float32 values, row-major north-to-south.
    let expected_bytes = cols * rows * 4;
    let mut flt_file = File::open(flt_path)?;
    let mut raw = Vec::with_capacity(expected_bytes);
    flt_file.read_to_end(&mut raw)?;

    if raw.len() < expected_bytes {
        return Err(RasterError::CorruptData(format!(
            "Esri Float Grid: expected {} bytes, got {}",
            expected_bytes,
            raw.len()
        )));
    }

    let n = cols * rows;
    let mut data: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let off = i * 4;
        let bytes: [u8; 4] = raw[off..off + 4].try_into().unwrap();
        let v = if little_endian {
            f32::from_le_bytes(bytes)
        } else {
            f32::from_be_bytes(bytes)
        };
        data.push(v as f64);
    }

    let prj_text = read_prj_sidecar(hdr_path);
    let crs = if let Some(ref text) = prj_text {
        if wkt_like(text) {
            CrsInfo::from_wkt(text.clone())
        } else {
            CrsInfo::default()
        }
    } else {
        CrsInfo::default()
    };

    let cfg = RasterConfig {
        cols,
        rows,
        x_min,
        y_min,
        cell_size: cs,
        nodata,
        data_type: DataType::F32,
        crs,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

fn write_header(raster: &Raster, hdr_path: &str) -> Result<()> {
    let mut w = BufWriter::new(File::create(hdr_path)?);
    writeln!(w, "ncols         {}", raster.cols)?;
    writeln!(w, "nrows         {}", raster.rows)?;
    writeln!(w, "xllcorner     {}", format_float(raster.x_min, 10))?;
    writeln!(w, "yllcorner     {}", format_float(raster.y_min, 10))?;
    writeln!(w, "cellsize      {}", format_float(raster.cell_size_x, 10))?;
    writeln!(w, "NODATA_value  {}", format_float(raster.nodata, 6))?;
    writeln!(w, "BYTEORDER     LSBFIRST")?;
    Ok(())
}

fn write_data(raster: &Raster, flt_path: &str) -> Result<()> {
    let n = raster.cols * raster.rows;
    let mut w = BufWriter::with_capacity(n * 4, File::create(flt_path)?);
    for row in 0..raster.rows as isize {
        for col in 0..raster.cols as isize {
            let v = raster.get(0, row, col) as f32;
            w.write_all(&v.to_le_bytes())?;
        }
    }
    Ok(())
}

fn write_prj_sidecar(raster: &Raster, hdr_path: &str) -> Result<()> {
    let prj_text = raster.crs.wkt.as_deref();
    if let Some(text) = prj_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let prj_path = with_extension(hdr_path, "prj");
            std::fs::write(prj_path, trimmed)?;
        }
    }
    Ok(())
}

fn read_prj_sidecar(hdr_path: &str) -> Option<String> {
    let prj_path = with_extension(hdr_path, "prj");
    std::fs::read_to_string(prj_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn wkt_like(s: &str) -> bool {
    let upper = s.trim().to_ascii_uppercase();
    upper.starts_with("GEOGCS[")
        || upper.starts_with("PROJCS[")
        || upper.starts_with("COMPOUNDCRS[")
        || upper.starts_with("PROJCRS[")
        || upper.starts_with("VERTCRS[")
}

fn parse_usize(field: &str, val: &str) -> Result<usize> {
    val.trim().parse::<usize>().map_err(|_| RasterError::ParseError {
        field: field.into(),
        value: val.into(),
        expected: "positive integer".into(),
    })
}

fn parse_f64(field: &str, val: &str) -> Result<f64> {
    val.trim().parse::<f64>().map_err(|_| RasterError::ParseError {
        field: field.into(),
        value: val.into(),
        expected: "floating-point number".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::RasterConfig;
    use tempfile::NamedTempFile;

    #[test]
    fn roundtrip_esri_float_grid() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 100.0,
            y_min: 50.0,
            cell_size: 10.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64 * 1.5).collect();
        let raster = Raster::from_data(cfg, data.clone()).unwrap();

        let tmp = NamedTempFile::new().unwrap();
        let flt_path = tmp.path().with_extension("flt");
        let flt_str = flt_path.to_str().unwrap();

        write(&raster, flt_str).unwrap();
        let loaded = read(flt_str).unwrap();

        assert_eq!(loaded.cols, 4);
        assert_eq!(loaded.rows, 3);
        assert!((loaded.cell_size_x - 10.0).abs() < 1e-9);
        assert!((loaded.x_min - 100.0).abs() < 1e-9);
        assert!((loaded.y_min - 50.0).abs() < 1e-9);

        for i in 0..12 {
            let row = (i / 4) as isize;
            let col = (i % 4) as isize;
            let expected = data[i] as f32 as f64;
            let actual = loaded.get(0, row, col);
            assert!(
                (actual - expected).abs() < 1e-4,
                "mismatch at ({row},{col}): got {actual}, expected {expected}"
            );
        }
    }

    #[test]
    fn read_hdr_accepts_msbfirst() {
        // Build a big-endian binary data file and matching .hdr.
        let tmp = NamedTempFile::new().unwrap();
        let base = tmp.path().with_extension("");
        let hdr_path = base.with_extension("hdr");
        let flt_path = base.with_extension("flt");

        let hdr = "ncols 2\nnrows 2\nxllcorner 0\nyllcorner 0\ncellsize 1\nBYTEORDER MSBFIRST\n";
        std::fs::write(&hdr_path, hdr).unwrap();

        // Write 4 big-endian f32 values: 1.0, 2.0, 3.0, 4.0
        let mut raw = Vec::new();
        for v in [1.0f32, 2.0, 3.0, 4.0] {
            raw.extend_from_slice(&v.to_be_bytes());
        }
        std::fs::write(&flt_path, &raw).unwrap();

        let loaded = read(hdr_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.cols, 2);
        assert_eq!(loaded.rows, 2);
        assert!((loaded.get(0, 0, 0) - 1.0).abs() < 1e-6);
        assert!((loaded.get(0, 1, 1) - 4.0).abs() < 1e-6);
    }
}
