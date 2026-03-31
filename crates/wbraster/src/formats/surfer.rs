//! Surfer GRD format.
//!
//! Supported variants:
//! - `DSAA` (Surfer ASCII grid)
//! - `DSRB` (Surfer 7 binary grid)

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use crate::error::{RasterError, Result};
use crate::io_utils::{format_float, with_extension};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

const SURFER_NODATA_ASCII: f64 = 1.71041e38;
const SURFER_NODATA_BINARY: f64 = 1.70141e38;

/// Read a Surfer GRD raster from `path`.
pub fn read(path: &str) -> Result<Raster> {
    let mut f = File::open(path)?;
    let mut sig = [0u8; 4];
    f.read_exact(&mut sig)?;

    if &sig == b"DSAA" {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(128 * 1024, file);
        let mut raster = read_dsaa(reader)?;
        apply_prj_sidecar(path, &mut raster);
        return Ok(raster);
    }

    if i32::from_le_bytes(sig) == 0x4252_5344 {
        let mut raster = read_dsrb(path)?;
        apply_prj_sidecar(path, &mut raster);
        return Ok(raster);
    }

    Err(RasterError::UnknownFormat(
        "unsupported Surfer GRD signature (expected DSAA or DSRB)".into(),
    ))
}

/// Write Surfer GRD.
///
/// Default write format is `DSAA` (ASCII).
/// Set metadata key `surfer_format=dsrb` to write `DSRB` (Surfer 7 binary).
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "Surfer GRD writer currently supports single-band rasters only".into(),
        ));
    }

    let write_dsrb = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("surfer_format"))
        .map(|(_, v)| v.eq_ignore_ascii_case("dsrb") || v.eq_ignore_ascii_case("surfer7"))
        .unwrap_or(false);

    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    if write_dsrb {
        write_dsrb_binary(&mut writer, raster)
    } else {
        write_dsaa(&mut writer, raster)
    }?;
    write_prj_sidecar(raster, path)
}

fn apply_prj_sidecar(path: &str, raster: &mut Raster) {
    if let Some(text) = read_prj_sidecar(path) {
        raster
            .metadata
            .push(("surfer_prj_text".to_string(), text.clone()));
        if wkt_like(&text) {
            raster.crs = CrsInfo::from_wkt(text);
        }
    }
}

fn read_prj_sidecar(path: &str) -> Option<String> {
    let prj_path = with_extension(path, "prj");
    std::fs::read_to_string(prj_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_prj_sidecar(raster: &Raster, path: &str) -> Result<()> {
    let prj_text = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("surfer_prj_text"))
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

fn read_dsaa<R: BufRead>(reader: R) -> Result<Raster> {
    let mut lines = reader.lines();

    let line1 = next_data_line(&mut lines)?;
    if !line1.eq_ignore_ascii_case("DSAA") {
        return Err(RasterError::CorruptData(
            "invalid DSAA header magic".into(),
        ));
    }

    let dims = next_data_line(&mut lines)?;
    let dims: Vec<&str> = dims.split_ascii_whitespace().collect();
    if dims.len() != 2 {
        return Err(RasterError::CorruptData(
            "invalid DSAA dimension line".into(),
        ));
    }
    let cols = parse_usize("cols", dims[0])?;
    let rows = parse_usize("rows", dims[1])?;

    let xline = next_data_line(&mut lines)?;
    let xs: Vec<&str> = xline.split_ascii_whitespace().collect();
    if xs.len() != 2 {
        return Err(RasterError::CorruptData("invalid DSAA X-range line".into()));
    }
    let west = parse_f64("x_min", xs[0])?;
    let east = parse_f64("x_max", xs[1])?;

    let yline = next_data_line(&mut lines)?;
    let ys: Vec<&str> = yline.split_ascii_whitespace().collect();
    if ys.len() != 2 {
        return Err(RasterError::CorruptData("invalid DSAA Y-range line".into()));
    }
    let south = parse_f64("y_min", ys[0])?;
    let north = parse_f64("y_max", ys[1])?;

    let _zline = next_data_line(&mut lines)?;

    if cols == 0 || rows == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }

    let mut data = vec![SURFER_NODATA_ASCII; rows * cols];
    let mut count = 0usize;

    for line in lines {
        let line = line?;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        for tok in t.split_ascii_whitespace() {
            if count >= rows * cols {
                break;
            }
            let v = parse_f64("value", tok)?;
            let south_row = count / cols;
            let col = count % cols;
            let north_row = rows - 1 - south_row;
            let idx = north_row * cols + col;
            data[idx] = v;
            count += 1;
        }
        if count >= rows * cols {
            break;
        }
    }

    if count < rows * cols {
        return Err(RasterError::CorruptData(format!(
            "expected {} values in DSAA grid, got {count}",
            rows * cols
        )));
    }

    let cfg = RasterConfig {
        cols,
        rows,
        x_min: west,
        y_min: south,
        cell_size: (east - west) / cols as f64,
        cell_size_y: Some((north - south) / rows as f64),
        nodata: SURFER_NODATA_ASCII,
        data_type: DataType::F64,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

fn read_dsrb(path: &str) -> Result<Raster> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 96 {
        return Err(RasterError::CorruptData(
            "DSRB file too small for header".into(),
        ));
    }

    let mut off = 0usize;
    let hdr_id = read_i32_le_at(&bytes, &mut off)?;
    if hdr_id != 0x4252_5344 {
        return Err(RasterError::CorruptData("invalid DSRB header id".into()));
    }
    let hdr_sz = read_i32_le_at(&bytes, &mut off)?;
    if hdr_sz != 4 {
        return Err(RasterError::CorruptData("invalid DSRB header size".into()));
    }
    let _version = read_i32_le_at(&bytes, &mut off)?;

    let grid_id = read_i32_le_at(&bytes, &mut off)?;
    if grid_id != 0x4449_5247 {
        return Err(RasterError::CorruptData("missing GRID section in DSRB".into()));
    }
    let grid_sz = read_i32_le_at(&bytes, &mut off)?;
    if grid_sz != 72 {
        return Err(RasterError::CorruptData("unexpected DSRB GRID section size".into()));
    }

    let rows = read_i32_le_at(&bytes, &mut off)? as usize;
    let cols = read_i32_le_at(&bytes, &mut off)? as usize;
    let west = read_f64_le_at(&bytes, &mut off)?;
    let south = read_f64_le_at(&bytes, &mut off)?;
    let dx = read_f64_le_at(&bytes, &mut off)?;
    let dy = read_f64_le_at(&bytes, &mut off)?;
    let _zmin = read_f64_le_at(&bytes, &mut off)?;
    let _zmax = read_f64_le_at(&bytes, &mut off)?;
    let _rotation = read_f64_le_at(&bytes, &mut off)?;
    let nodata = read_f64_le_at(&bytes, &mut off)?;

    if off + 8 <= bytes.len() {
        let maybe_data = i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        if maybe_data == 0x4154_4144 {
            off += 4;
            let _section_bytes = read_i32_le_at(&bytes, &mut off)?;
        }
    }

    if rows == 0 || cols == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }

    let mut data = vec![nodata; rows * cols];
    for south_row in 0..rows {
        for col in 0..cols {
            let v = read_f64_le_at(&bytes, &mut off)?;
            let north_row = rows - 1 - south_row;
            let idx = north_row * cols + col;
            data[idx] = if v >= nodata { nodata } else { v };
        }
    }

    let cfg = RasterConfig {
        cols,
        rows,
        x_min: west,
        y_min: south,
        cell_size: dx,
        cell_size_y: Some(dy),
        nodata,
        data_type: DataType::F64,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

fn write_dsaa<W: Write>(w: &mut W, raster: &Raster) -> Result<()> {
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for v in raster.data.iter_f64() {
        if raster.is_nodata(v) {
            continue;
        }
        if v < zmin {
            zmin = v;
        }
        if v > zmax {
            zmax = v;
        }
    }
    if !zmin.is_finite() || !zmax.is_finite() {
        zmin = SURFER_NODATA_ASCII;
        zmax = SURFER_NODATA_ASCII;
    }

    writeln!(w, "DSAA")?;
    writeln!(w, "{} {}", raster.cols, raster.rows)?;
    writeln!(w, "{} {}", format_float(raster.x_min, 10), format_float(raster.x_max(), 10))?;
    writeln!(w, "{} {}", format_float(raster.y_min, 10), format_float(raster.y_max(), 10))?;
    writeln!(w, "{} {}", format_float(zmin, 10), format_float(zmax, 10))?;

    for row in (0..raster.rows).rev() {
        let slice = raster.row_slice(0, row as isize);
        let mut first = true;
        for v in slice {
            if !first {
                write!(w, " ")?;
            }
            first = false;
            if raster.is_nodata(v) {
                write!(w, "1.71041e38")?;
            } else {
                write!(w, "{}", format_float(v, 6))?;
            }
        }
        writeln!(w)?;
    }
    Ok(())
}

fn write_dsrb_binary<W: Write>(w: &mut W, raster: &Raster) -> Result<()> {
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for v in raster.data.iter_f64() {
        if raster.is_nodata(v) {
            continue;
        }
        if v < zmin {
            zmin = v;
        }
        if v > zmax {
            zmax = v;
        }
    }
    if !zmin.is_finite() || !zmax.is_finite() {
        zmin = SURFER_NODATA_BINARY;
        zmax = SURFER_NODATA_BINARY;
    }

    // Header
    w.write_all(&0x4252_5344_i32.to_le_bytes())?; // DSRB
    w.write_all(&4_i32.to_le_bytes())?;
    w.write_all(&2_i32.to_le_bytes())?; // version

    // GRID section
    w.write_all(&0x4449_5247_i32.to_le_bytes())?; // GRID
    w.write_all(&72_i32.to_le_bytes())?;
    w.write_all(&(raster.rows as i32).to_le_bytes())?;
    w.write_all(&(raster.cols as i32).to_le_bytes())?;
    w.write_all(&raster.x_min.to_le_bytes())?;
    w.write_all(&raster.y_min.to_le_bytes())?;
    w.write_all(&raster.cell_size_x.to_le_bytes())?;
    w.write_all(&raster.cell_size_y.to_le_bytes())?;
    w.write_all(&zmin.to_le_bytes())?;
    w.write_all(&zmax.to_le_bytes())?;
    w.write_all(&0.0_f64.to_le_bytes())?; // rotation
    w.write_all(&SURFER_NODATA_BINARY.to_le_bytes())?;

    // DATA section
    let section_bytes = (raster.rows * raster.cols * 8) as i32;
    w.write_all(&0x4154_4144_i32.to_le_bytes())?; // DATA
    w.write_all(&section_bytes.to_le_bytes())?;

    // Data are stored south-to-north, west-to-east.
    for row in (0..raster.rows).rev() {
        let slice = raster.row_slice(0, row as isize);
        for v in slice {
            let out = if raster.is_nodata(v) { SURFER_NODATA_BINARY } else { v };
            w.write_all(&out.to_le_bytes())?;
        }
    }

    Ok(())
}

fn next_data_line<R: BufRead>(lines: &mut std::io::Lines<R>) -> Result<String> {
    for line in lines {
        let line = line?;
        let t = line.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    Err(RasterError::CorruptData(
        "unexpected EOF while parsing Surfer grid".into(),
    ))
}

fn parse_usize(field: &str, val: &str) -> Result<usize> {
    val.parse::<usize>().map_err(|_| RasterError::ParseError {
        field: field.into(),
        value: val.into(),
        expected: "positive integer".into(),
    })
}

fn parse_f64(field: &str, val: &str) -> Result<f64> {
    val.parse::<f64>().map_err(|_| RasterError::ParseError {
        field: field.into(),
        value: val.into(),
        expected: "floating-point number".into(),
    })
}

fn read_i32_le_at(buf: &[u8], off: &mut usize) -> Result<i32> {
    if *off + 4 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF in DSRB i32".into()));
    }
    let out = i32::from_le_bytes(buf[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(out)
}

fn read_f64_le_at(buf: &[u8], off: &mut usize) -> Result<f64> {
    if *off + 8 > buf.len() {
        return Err(RasterError::CorruptData("unexpected EOF in DSRB f64".into()));
    }
    let out = f64::from_le_bytes(buf[*off..*off + 8].try_into().unwrap());
    *off += 8;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::io::Cursor;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SAMPLE_DSAA: &str = "\
DSAA
4 3
0 4
0 3
1 12
9 10 11 12
5 6 1.71041e38 8
1 2 3 4
";

    #[test]
    fn parse_dsaa() {
        let r = read_dsaa(BufReader::new(Cursor::new(SAMPLE_DSAA))).unwrap();
        assert_eq!(r.cols, 4);
        assert_eq!(r.rows, 3);
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert!(r.is_nodata(r.get(0, 1, 2)));
        assert_eq!(r.get(0, 2, 3), 12.0);
    }

    #[test]
    fn roundtrip_dsaa() {
        let r = read_dsaa(BufReader::new(Cursor::new(SAMPLE_DSAA))).unwrap();
        let mut out = Vec::new();
        write_dsaa(&mut out, &r).unwrap();
        let r2 = read_dsaa(BufReader::new(Cursor::new(out))).unwrap();
        assert_eq!(r2.cols, 4);
        assert_eq!(r2.rows, 3);
        assert_eq!(r2.get(0, 0, 0), 1.0);
        assert!(r2.is_nodata(r2.get(0, 1, 2)));
    }

    fn tmp(suffix: &str) -> String {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        temp_dir().join(format!("surfer_test_{ts}{suffix}")).to_string_lossy().into_owned()
    }

    #[test]
    fn writer_defaults_to_dsaa() {
        let path = tmp(".grd");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"DSAA");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn writer_supports_dsrb_via_metadata() {
        let path = tmp(".grd");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let mut r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        r.metadata.push(("surfer_format".into(), "dsrb".into()));
        write(&r, &path).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(i32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0x4252_5344);

        let r2 = read(&path).unwrap();
        assert_eq!(r2.cols, 2);
        assert_eq!(r2.rows, 2);
        assert!((r2.get(0, 0, 0) - 1.0).abs() < 1e-9);
        assert!((r2.get(0, 1, 1) - 4.0).abs() < 1e-9);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn surfer_writes_and_reads_prj_sidecar() {
        let path = tmp(".grd");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let mut r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\"]]";
        r.crs = CrsInfo::from_wkt(wkt);

        write(&r, &path).unwrap();

        let prj = with_extension(&path, "prj");
        let txt = std::fs::read_to_string(&prj).unwrap();
        assert_eq!(txt.trim(), wkt);

        let r2 = read(&path).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt));
        assert!(r2
            .metadata
            .iter()
            .any(|(k, v)| k == "surfer_prj_text" && v.trim() == wkt));

        let _ = std::fs::remove_file(&path);
        if Path::new(&prj).exists() {
            let _ = std::fs::remove_file(&prj);
        }
    }
}
