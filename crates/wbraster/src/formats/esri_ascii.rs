//! Esri ASCII Grid format (`.asc` / `.grd`).
//!
//! Spec reference:
//! <https://desktop.arcgis.com/en/arcmap/latest/manage-data/raster-and-images/esri-ascii-raster-format.htm>
//!
//! Header keywords (case-insensitive):
//! - `ncols`, `nrows`
//! - `xllcorner` / `xllcenter`, `yllcorner` / `yllcenter`
//! - `cellsize`
//! - `nodata_value` (optional, default = -9999)
//!
//! Data is written row-by-row, north to south, space-separated.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::{format_float, parse_key_value, with_extension};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

/// Read an Esri ASCII Grid from `path`.
pub fn read(path: &str) -> Result<Raster> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    parse(reader, path)
}

/// Parse an ASCII Grid from any `BufRead`.
pub fn parse<R: BufRead>(reader: R, source: &str) -> Result<Raster> {
    let mut cols: Option<usize> = None;
    let mut rows: Option<usize> = None;
    let mut xll: Option<f64> = None;
    let mut yll: Option<f64> = None;
    let mut xll_is_corner = true;
    let mut yll_is_corner = true;
    let mut cell_size: Option<f64> = None;
    let mut nodata: f64 = -9999.0;
    let mut data: Vec<f64> = Vec::new();
    let mut header_lines = 0usize;

    let mut lines = reader.lines();

    // ── Header (up to 6 lines) ──────────────────────────────────────────
    loop {
        let raw = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => return Err(e.into()),
            None => break,
        };
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // If the first character is a digit, '-', or '+', we're in the data.
        let first = line.chars().next().unwrap_or(' ');
        if first.is_ascii_digit() || first == '-' || first == '+' {
            // Push this line as data
            parse_data_line(line, nodata, &mut data)?;
            break;
        }
        header_lines += 1;
        if let Some((key, val)) = parse_key_value(line) {
            match key.as_str() {
                "ncols"        => cols      = Some(parse_usize(&key, &val)?),
                "nrows"        => rows      = Some(parse_usize(&key, &val)?),
                "xllcorner"    => { xll = Some(parse_f64(&key, &val)?); xll_is_corner = true; }
                "xllcenter"    => { xll = Some(parse_f64(&key, &val)?); xll_is_corner = false; }
                "yllcorner"    => { yll = Some(parse_f64(&key, &val)?); yll_is_corner = true; }
                "yllcenter"    => { yll = Some(parse_f64(&key, &val)?); yll_is_corner = false; }
                "cellsize"     => cell_size = Some(parse_f64(&key, &val)?),
                "nodata_value" | "nodata" => nodata = parse_f64(&key, &val)?,
                _ => { /* unknown header field – ignore */ }
            }
        }
        // Stop after we've collected all mandatory fields
        if cols.is_some() && rows.is_some() && xll.is_some() && yll.is_some() && cell_size.is_some() && header_lines >= 5 {
            // Peek: if next line starts with a digit we're done
            // We can't easily peek with the iterator, but 6 header lines is the max
        }
    }

    let cols = cols.ok_or_else(|| RasterError::MissingField("ncols".into()))?;
    let rows = rows.ok_or_else(|| RasterError::MissingField("nrows".into()))?;
    let cs   = cell_size.ok_or_else(|| RasterError::MissingField("cellsize".into()))?;
    let xll  = xll.ok_or_else(|| RasterError::MissingField("xllcorner/xllcenter".into()))?;
    let yll  = yll.ok_or_else(|| RasterError::MissingField("yllcorner/yllcenter".into()))?;

    // Adjust corner vs. center
    let x_min = if xll_is_corner { xll } else { xll - cs * 0.5 };
    let y_min = if yll_is_corner { yll } else { yll - cs * 0.5 };

    if cols == 0 || rows == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }

    // ── Data ─────────────────────────────────────────────────────────────
    data.reserve(cols * rows - data.len().min(cols * rows));

    for line_result in lines {
        let line = line_result?;
        let line = line.trim();
        if line.is_empty() { continue; }
        parse_data_line(line, nodata, &mut data)?;
        if data.len() >= cols * rows {
            break;
        }
    }

    if data.len() < cols * rows {
        return Err(RasterError::CorruptData(format!(
            "expected {} values, got {}",
            cols * rows,
            data.len()
        )));
    }
    data.truncate(cols * rows);

    let prj_text = read_prj_sidecar(source);
    let mut metadata = Vec::new();
    let crs = if let Some(ref text) = prj_text {
        metadata.push(("esri_ascii_prj_text".to_string(), text.clone()));
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
        data_type: DataType::F64,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

/// Write an Esri ASCII Grid to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::with_capacity(256 * 1024, file);
    write_to(&mut w, raster)?;
    write_prj_sidecar(raster, path)
}

/// Write an ASCII Grid to any `Write`.
pub fn write_to<W: Write>(w: &mut W, raster: &Raster) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "Esri ASCII Grid writer currently supports single-band rasters only".into(),
        ));
    }
    writeln!(w, "ncols         {}", raster.cols)?;
    writeln!(w, "nrows         {}", raster.rows)?;
    writeln!(w, "xllcorner     {}", format_float(raster.x_min, 10))?;
    writeln!(w, "yllcorner     {}", format_float(raster.y_min, 10))?;
    writeln!(w, "cellsize      {}", format_float(raster.cell_size_x, 10))?;
    writeln!(w, "NODATA_value  {}", format_float(raster.nodata, 6))?;

    // Data — row-major, north to south (row 0 first).
    for row in 0..raster.rows {
        let slice = raster.row_slice(0, row as isize);
        let mut first = true;
        for v in slice {
            if !first { write!(w, " ")?; }
            first = false;
            write!(w, "{}", format_float(v, 6))?;
        }
        writeln!(w)?;
    }
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

/// Parse whitespace-delimited floats from a data line into `buf`.
fn parse_data_line(line: &str, _nodata: f64, buf: &mut Vec<f64>) -> Result<()> {
    for token in line.split_ascii_whitespace() {
        let v: f64 = token.parse().map_err(|_| RasterError::CorruptData(
            format!("invalid data token: '{token}'")
        ))?;
        buf.push(v);
    }
    Ok(())
}

fn read_prj_sidecar(source: &str) -> Option<String> {
    let prj_path = with_extension(source, "prj");
    std::fs::read_to_string(prj_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_prj_sidecar(raster: &Raster, path: &str) -> Result<()> {
    let prj_text = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("esri_ascii_prj_text"))
        .map(|(_, v)| v.as_str())
        .or(raster.crs.wkt.as_deref());

    if let Some(text) = prj_text {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let prj_path = with_extension(path, "prj");
            std::fs::write(prj_path, trimmed)?;
        }
    }
    Ok(())
}

fn wkt_like(s: &str) -> bool {
    let t = s.trim();
    let upper = t.to_ascii_uppercase();
    !t.is_empty()
        && (upper.starts_with("GEOGCS[")
            || upper.starts_with("PROJCS[")
            || upper.starts_with("COMPOUNDCRS[")
            || upper.starts_with("GEODCRS[")
            || upper.starts_with("PROJCRS[")
            || upper.starts_with("VERTCRS["))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::path::PathBuf;

    const SAMPLE_ASC: &str = "\
ncols         4
nrows         3
xllcorner     0.0
yllcorner     0.0
cellsize      10.0
NODATA_value  -9999
1 2 3 4
5 6 7 8
9 10 11 12
";

    #[test]
    fn roundtrip_ascii() {
        let r = parse(BufReader::new(Cursor::new(SAMPLE_ASC)), "test.asc").unwrap();
        assert_eq!(r.cols, 4);
        assert_eq!(r.rows, 3);
        assert_eq!(r.cell_size_x, 10.0);
        assert_eq!(r.x_min, 0.0);
        assert_eq!(r.y_min, 0.0);
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert_eq!(r.get(0, 2, 3), 12.0);
        assert_eq!(r.nodata, -9999.0);

        // Write and re-read
        let mut buf = Vec::new();
        write_to(&mut buf, &r).unwrap();
        let r2 = parse(BufReader::new(Cursor::new(&buf[..])), "test.asc").unwrap();
        assert_eq!(r2.cols, 4);
        assert_eq!(r2.get(0, 1, 2), 7.0);
    }

    #[test]
    fn nodata_handling() {
        let src = "\
ncols 2
nrows 2
xllcorner 0.0
yllcorner 0.0
cellsize 1.0
NODATA_value -9999
1.0 -9999.0
3.0 4.0
";
        let r = parse(BufReader::new(Cursor::new(src)), "nd.asc").unwrap();
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert!(r.is_nodata(r.get(0, 0, 1)));   // nodata
        assert_eq!(r.get(0, 1, 0), 3.0);
    }

    #[test]
    fn xllcenter_offset() {
        let src = "\
ncols 2
nrows 2
xllcenter 5.0
yllcenter 5.0
cellsize 10.0
NODATA_value -9999
1 2
3 4
";
        let r = parse(BufReader::new(Cursor::new(src)), "c.asc").unwrap();
        // xll_corner = 5.0 - 10.0*0.5 = 0.0
        assert_eq!(r.x_min, 0.0);
        assert_eq!(r.y_min, 0.0);
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            let path = std::env::temp_dir().join(format!("wbraster_esri_ascii_{ts}"));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn esri_ascii_writes_and_reads_prj_sidecar() {
        let td = TempDir::new();
        let asc = td.path().join("dem.asc");
        let asc_path = asc.to_string_lossy().to_string();

        let mut r = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            ..Default::default()
        });
        r.set(0, 0, 0, 1.0).unwrap();
        r.set(0, 0, 1, 2.0).unwrap();
        r.set(0, 1, 0, 3.0).unwrap();
        r.set(0, 1, 1, 4.0).unwrap();
        let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\"]]".to_string();
        r.crs = CrsInfo::from_wkt(wkt.clone());

        write(&r, &asc_path).unwrap();

        let prj_path = with_extension(&asc_path, "prj");
        let prj_text = std::fs::read_to_string(&prj_path).unwrap();
        assert_eq!(prj_text.trim(), wkt);

        let r2 = read(&asc_path).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt.as_str()));
        assert!(r2
            .metadata
            .iter()
            .any(|(k, v)| k == "esri_ascii_prj_text" && v.trim() == wkt));
    }
}
