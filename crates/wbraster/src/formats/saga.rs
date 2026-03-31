//! SAGA GIS Binary Grid format (`.sdat` / `.sgrd`).
//!
//! A SAGA grid consists of two files:
//! - `<name>.sgrd` — plain-text header
//! - `<name>.sdat` — binary data (little-endian by default)
//!
//! Reference: SAGA GIS source (`saga_core/saga_api/grid_io.cpp`)
//!
//! Header fields (subset used here):
//! ```text
//! NAME       = <name>
//! DESCRIPTION=
//! UNIT       =
//! DATAFORMAT = FLOAT          (BYTE|SHORT|DWORD|INT|FLOAT|DOUBLE)
//! DATAFILE_OFFSET = 0
//! BYTEORDER_BIG = FALSE
//! POSITION_XMIN = <f64>       (cell-center)
//! POSITION_YMIN = <f64>       (cell-center)
//! CELLCOUNT_X = <int>
//! CELLCOUNT_Y = <int>
//! CELLSIZE = <f64>
//! Z_FACTOR = 1.000000
//! Z_MIN = <f64>
//! Z_MAX = <f64>
//! NODATA_VALUE = -99999.0
//! TOPTOBOTTOM = FALSE
//! ```

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::*;
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read a SAGA Grid. `path` may be the `.sgrd` or `.sdat` file.
pub fn read(path: &str) -> Result<Raster> {
    let sgrd_path = if path.ends_with(".sdat") {
        with_extension(path, "sgrd")
    } else {
        path.to_string()
    };
    read_from_header(&sgrd_path)
}

/// Write a SAGA Grid. `path` should be the desired `.sgrd` path.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let sgrd_path = if path.ends_with(".sdat") {
        with_extension(path, "sgrd")
    } else {
        path.to_string()
    };
        if raster.bands != 1 {
            return Err(RasterError::UnsupportedDataType(
                "SAGA writer currently supports single-band rasters only".into(),
            ));
        }
        let big_endian = saga_write_big_endian(raster);
    let sdat_path = with_extension(&sgrd_path, "sdat");
        write_header(raster, &sgrd_path, big_endian)?;
        write_data(raster, &sdat_path, big_endian)?;
        write_prj_sidecar(raster, &sgrd_path)
}

// ─── Header ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SagaHeader {
    name: String,
    data_format: DataType,
    data_offset: u64,
    big_endian: bool,
    x_min_center: f64,  // cell-center X of first column
    y_min_center: f64,  // cell-center Y of first row (bottom-most when toptobottom=false)
    cols: usize,
    rows: usize,
    cell_size: f64,
    nodata: f64,
    top_to_bottom: bool,
}

fn parse_header(path: &str) -> Result<SagaHeader> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut name = String::new();
    let mut data_format = DataType::F32;
    let mut data_offset: u64 = 0;
    let mut big_endian = false;
    let mut x_min_center: Option<f64> = None;
    let mut y_min_center: Option<f64> = None;
    let mut cols: Option<usize> = None;
    let mut rows: Option<usize> = None;
    let mut cell_size: Option<f64> = None;
    let mut nodata = -99999.0_f64;
    let mut top_to_bottom = false;

    for line in reader.lines() {
        let line = line?;
        let (key, val) = match parse_key_value(&line) {
            Some(kv) => kv,
            None => continue,
        };
        match key.as_str() {
            "name"             => name = val,
            "dataformat"       => {
                data_format = saga_data_type(&val).ok_or_else(|| RasterError::ParseError {
                    field: "DATAFORMAT".into(),
                    value: val.clone(),
                    expected: "BYTE|SHORT|DWORD|INT|FLOAT|DOUBLE".into(),
                })?;
            }
            "datafile_offset"  => data_offset = val.trim().parse::<u64>().unwrap_or(0),
            "byteorder_big"    => big_endian = val.trim().eq_ignore_ascii_case("true"),
            "position_xmin"    => x_min_center = Some(parse_f64("position_xmin", &val)?),
            "position_ymin"    => y_min_center = Some(parse_f64("position_ymin", &val)?),
            "cellcount_x"      => cols = Some(parse_usize("cellcount_x", &val)?),
            "cellcount_y"      => rows = Some(parse_usize("cellcount_y", &val)?),
            "cellsize"         => cell_size = Some(parse_f64("cellsize", &val)?),
            "nodata_value"     => nodata = parse_f64("nodata_value", &val)?,
            "toptobottom"      => top_to_bottom = val.trim().eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    let cols = cols.ok_or_else(|| RasterError::MissingField("CELLCOUNT_X".into()))?;
    let rows = rows.ok_or_else(|| RasterError::MissingField("CELLCOUNT_Y".into()))?;
    let cell_size = cell_size.ok_or_else(|| RasterError::MissingField("CELLSIZE".into()))?;
    let x_min_center = x_min_center.ok_or_else(|| RasterError::MissingField("POSITION_XMIN".into()))?;
    let y_min_center = y_min_center.ok_or_else(|| RasterError::MissingField("POSITION_YMIN".into()))?;

    Ok(SagaHeader {
        name,
        data_format,
        data_offset,
        big_endian,
        x_min_center,
        y_min_center,
        cols,
        rows,
        cell_size,
        nodata,
        top_to_bottom,
    })
}

fn saga_data_type(s: &str) -> Option<DataType> {
    match s.trim().to_ascii_uppercase().as_str() {
        "BYTE" | "BYTE_UNSIGNED" => Some(DataType::U8),
        "SHORT" | "SHORTINT" => Some(DataType::I16),
        "WORD" | "SHORT_UNSIGNED" | "SHORTINT_UNSIGNED" => Some(DataType::U16),
        "DWORD" | "INTEGER_UNSIGNED" => Some(DataType::U32),
        "INT" | "INTEGER" => Some(DataType::I32),
        "FLOAT" => Some(DataType::F32),
        "DOUBLE" => Some(DataType::F64),
        _        => None,
    }
}

fn saga_data_type_str(dt: DataType) -> Result<&'static str> {
    match dt {
        DataType::U8  => Ok("BYTE"),
        DataType::I16 => Ok("SHORT"),
        DataType::U16 => Ok("WORD"),
        DataType::U32 => Ok("DWORD"),
        DataType::I32 => Ok("INT"),
        DataType::F32 => Ok("FLOAT"),
        DataType::F64 => Ok("DOUBLE"),
        _ => Err(RasterError::UnsupportedDataType(format!(
            "SAGA writer does not support {:?}",
            dt
        ))),
    }
}

// ─── Read ─────────────────────────────────────────────────────────────────────

fn read_from_header(sgrd_path: &str) -> Result<Raster> {
    let hdr = parse_header(sgrd_path)?;
    let sdat_path = with_extension(sgrd_path, "sdat");
    let data = read_data(&sdat_path, &hdr)?;
    let prj_text = read_prj_sidecar(sgrd_path);
    let crs = prj_text
        .as_deref()
        .map(CrsInfo::from_wkt)
        .unwrap_or_default();

    // SAGA stores POSITION_XMIN/YMIN as cell *centers* of the bottom-left cell.
    let x_min = hdr.x_min_center - hdr.cell_size * 0.5;
    let y_min = hdr.y_min_center - hdr.cell_size * 0.5;

    let mut metadata = vec![
        ("name".to_string(), hdr.name),
        (
            "saga_byteorder_big".to_string(),
            if hdr.big_endian { "true".to_string() } else { "false".to_string() },
        ),
    ];
    if let Some(ref text) = prj_text {
        metadata.push(("saga_prj_text".to_string(), text.clone()));
    }

    let cfg = RasterConfig {
        cols: hdr.cols,
        rows: hdr.rows,
        x_min,
        y_min,
        cell_size: hdr.cell_size,
        nodata: hdr.nodata,
        data_type: hdr.data_format,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

fn read_prj_sidecar(sgrd_path: &str) -> Option<String> {
    let prj_path = with_extension(sgrd_path, "prj");
    match std::fs::read_to_string(prj_path) {
        Ok(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Err(_) => None,
    }
}

fn write_prj_sidecar(raster: &Raster, sgrd_path: &str) -> Result<()> {
    let wkt = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("saga_prj_text"))
        .or_else(|| {
            raster
                .metadata
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("saga_prj_wkt"))
        })
        .map(|(_, v)| v.as_str())
        .or(raster.crs.wkt.as_deref());

    if let Some(wkt) = wkt {
        let trimmed = wkt.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let prj_path = with_extension(sgrd_path, "prj");
        std::fs::write(prj_path, trimmed)?;
    }
    Ok(())
}

fn read_data(path: &str, hdr: &SagaHeader) -> Result<Vec<f64>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = BufReader::with_capacity(512 * 1024, File::open(path)?);
    if hdr.data_offset > 0 {
        file.seek(SeekFrom::Start(hdr.data_offset))?;
    }

    let n = hdr.cols * hdr.rows;
    let be = hdr.big_endian;
    let mut data: Vec<f64> = Vec::with_capacity(n);

    macro_rules! read_all {
        ($read_fn:expr) => {{
            for _ in 0..n {
                data.push($read_fn(&mut file)? as f64);
            }
        }};
    }

    match hdr.data_format {
        DataType::U8 => {
            let mut buf = vec![0u8; n];
            file.read_exact(&mut buf)?;
            data.extend(buf.iter().map(|&b| b as f64));
        }
        DataType::I16 => {
            if be { for _ in 0..n { data.push(read_i16_be_stream(&mut file)? as f64); } }
            else  { for _ in 0..n { data.push(read_i16_le_stream(&mut file)? as f64); } }
        }
        DataType::I32 => {
            if be { for _ in 0..n { data.push(read_i32_be_stream(&mut file)? as f64); } }
            else  { for _ in 0..n { data.push(read_i32_le_stream(&mut file)? as f64); } }
        }
        DataType::U32 => {
            if be {
                for _ in 0..n {
                    let mut b = [0u8; 4];
                    file.read_exact(&mut b)?;
                    data.push(u32::from_be_bytes(b) as f64);
                }
            } else {
                for _ in 0..n {
                    let mut b = [0u8; 4];
                    file.read_exact(&mut b)?;
                    data.push(u32::from_le_bytes(b) as f64);
                }
            }
        }
        DataType::F32 => {
            if be { read_all!(read_f32_be_stream) }
            else  { read_all!(read_f32_le_stream) }
        }
        DataType::F64 => {
            if be { read_all!(read_f64_be_stream) }
            else  { read_all!(read_f64_le_stream) }
        }
        _ => return Err(RasterError::UnsupportedDataType(hdr.data_format.to_string())),
    }

    // If TOPTOBOTTOM = FALSE (the default), data is stored south-to-north.
    // We store internally top-to-bottom, so flip.
    if !hdr.top_to_bottom {
        flip_rows(&mut data, hdr.cols, hdr.rows);
    }

    Ok(data)
}

fn flip_rows(data: &mut [f64], cols: usize, rows: usize) {
    for r in 0..rows / 2 {
        let top = r * cols;
        let bot = (rows - 1 - r) * cols;
        for c in 0..cols {
            data.swap(top + c, bot + c);
        }
    }
}

// ─── Write ────────────────────────────────────────────────────────────────────

fn write_header(raster: &Raster, sgrd_path: &str, big_endian: bool) -> Result<()> {
    let name = raster.metadata.iter()
        .find(|(k, _)| k == "name")
        .map(|(_, v)| v.as_str())
        .unwrap_or("unnamed");
    let stats = raster.statistics();
    let sdat_name = std::path::Path::new(sgrd_path)
        .file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    let mut w = BufWriter::new(File::create(sgrd_path)?);
    writeln!(w, "NAME\t\t\t= {name}")?;
    writeln!(w, "DESCRIPTION\t\t=")?;
    writeln!(w, "UNIT\t\t\t=")?;
    writeln!(w, "DATAFORMAT\t\t= {}", saga_data_type_str(raster.data_type)?)?;
    writeln!(w, "DATAFILE_OFFSET\t\t= 0")?;
    writeln!(w, "BYTEORDER_BIG\t\t= {}", if big_endian { "TRUE" } else { "FALSE" })?;
    writeln!(w, "POSITION_XMIN\t\t= {}", format_float(raster.x_min + raster.cell_size_x * 0.5, 10))?;
    writeln!(w, "POSITION_YMIN\t\t= {}", format_float(raster.y_min + raster.cell_size_y * 0.5, 10))?;
    writeln!(w, "CELLCOUNT_X\t\t= {}", raster.cols)?;
    writeln!(w, "CELLCOUNT_Y\t\t= {}", raster.rows)?;
    writeln!(w, "CELLSIZE\t\t= {}", format_float(raster.cell_size_x, 10))?;
    writeln!(w, "Z_FACTOR\t\t= 1.000000")?;
    writeln!(w, "Z_MIN\t\t\t= {}", format_float(stats.min, 6))?;
    writeln!(w, "Z_MAX\t\t\t= {}", format_float(stats.max, 6))?;
    writeln!(w, "NODATA_VALUE\t\t= {}", format_float(raster.nodata, 6))?;
    writeln!(w, "TOPTOBOTTOM\t\t= FALSE")?;
    writeln!(w, "DATAFILE_NAME\t\t= {sdat_name}.sdat")?;
    w.flush()?;
    Ok(())
}

fn write_data(raster: &Raster, sdat_path: &str, big_endian: bool) -> Result<()> {
    let mut w = BufWriter::with_capacity(512 * 1024, File::create(sdat_path)?);
    // We always write TOPTOBOTTOM=FALSE, so flip rows (write south-to-north).
    match raster.data_type {
        DataType::U8 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    w.write_all(&[v as u8])?;
                }
            }
        }
        DataType::I16 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        (v as i16).to_be_bytes()
                    } else {
                        (v as i16).to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        DataType::U16 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        (v as u16).to_be_bytes()
                    } else {
                        (v as u16).to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        DataType::I32 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        (v as i32).to_be_bytes()
                    } else {
                        (v as i32).to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        DataType::U32 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        (v as u32).to_be_bytes()
                    } else {
                        (v as u32).to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        DataType::F32 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        (v as f32).to_be_bytes()
                    } else {
                        (v as f32).to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        DataType::F64 => {
            for row in (0..raster.rows).rev() {
                let slice = raster.row_slice(0, row as isize);
                for v in slice {
                    let bytes = if big_endian {
                        v.to_be_bytes()
                    } else {
                        v.to_le_bytes()
                    };
                    w.write_all(&bytes)?;
                }
            }
        }
        _ => {
            return Err(RasterError::UnsupportedDataType(format!(
                "SAGA writer does not support {:?}",
                raster.data_type
            )));
        }
    }
    w.flush()?;
    Ok(())
}

fn saga_write_big_endian(raster: &Raster) -> bool {
    raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("saga_byteorder_big") || k.eq_ignore_ascii_case("byteorder_big"))
        .map(|(_, v)| {
            let t = v.trim().to_ascii_lowercase();
            matches!(t.as_str(), "true" | "1" | "yes" | "y" | "big" | "msb" | "msbfirst")
        })
        .unwrap_or(false)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_usize(field: &str, val: &str) -> Result<usize> {
    val.trim().parse::<usize>().map_err(|_| RasterError::ParseError {
        field: field.into(), value: val.into(), expected: "positive integer".into(),
    })
}

fn parse_f64(field: &str, val: &str) -> Result<f64> {
    val.trim().parse::<f64>().map_err(|_| RasterError::ParseError {
        field: field.into(), value: val.into(), expected: "float".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::RasterConfig;
    use crate::crs_info::CrsInfo;
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(suffix: &str) -> String {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        temp_dir().join(format!("saga_test_{ts}{suffix}")).to_string_lossy().into_owned()
    }

    #[test]
    fn saga_roundtrip() {
        let sgrd = tmp_path(".sgrd");
        let cfg = RasterConfig {
            cols: 3, rows: 2, cell_size: 5.0, x_min: 10.0, y_min: 20.0,
            nodata: -99999.0, data_type: DataType::F32, ..Default::default()
        };
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &sgrd).unwrap();
        let r2 = read(&sgrd).unwrap();
        assert_eq!(r2.cols, 3);
        assert_eq!(r2.rows, 2);
        assert!((r2.get(0, 0, 0) - 1.0).abs() < 1e-4, "got {:?}", r2.get(0, 0, 0));
        assert!((r2.get(0, 1, 2) - 6.0).abs() < 1e-4, "got {:?}", r2.get(0, 1, 2));
        // Clean up
        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
    }

    #[test]
    fn saga_roundtrip_i16() {
        let sgrd = tmp_path(".sgrd");
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            cell_size: 5.0,
            x_min: 10.0,
            y_min: 20.0,
            nodata: -32768.0,
            data_type: DataType::I16,
            ..Default::default()
        };
        let data = vec![-10.0, -2.0, 0.0, 1.0, 2.0, 300.0];
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &sgrd).unwrap();
        let r2 = read(&sgrd).unwrap();
        assert_eq!(r2.data_type, DataType::I16);
        assert_eq!(r2.get(0, 0, 0), -10.0);
        assert_eq!(r2.get(0, 1, 2), 300.0);
        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
    }

    #[test]
    fn saga_roundtrip_f32_big_endian() {
        let sgrd = tmp_path(".sgrd");
        let mut cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -99999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        cfg.metadata.push(("saga_byteorder_big".to_string(), "true".to_string()));
        let data = vec![1.25, 2.5, 3.75, 4.0];
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &sgrd).unwrap();

        let hdr_text = std::fs::read_to_string(&sgrd).unwrap();
        assert!(hdr_text.contains("BYTEORDER_BIG\t\t= TRUE"));

        let r2 = read(&sgrd).unwrap();
        assert!((r2.get(0, 0, 0) - 1.25).abs() < 1e-6);
        assert!((r2.get(0, 0, 1) - 2.5).abs() < 1e-6);
        assert!((r2.get(0, 1, 0) - 3.75).abs() < 1e-6);
        assert!((r2.get(0, 1, 1) - 4.0).abs() < 1e-6);

        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
    }

    #[test]
    fn saga_writes_prj_sidecar_when_srs_present() {
        let sgrd = tmp_path(".sgrd");
        let mut cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -99999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let wkt = "GEOGCS[\"WGS 84\"]";
        cfg.crs = CrsInfo::from_wkt(wkt);
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &sgrd).unwrap();

        let prj = with_extension(&sgrd, "prj");
        let text = std::fs::read_to_string(&prj).unwrap();
        assert_eq!(text.trim(), wkt);

        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
        let _ = std::fs::remove_file(prj);
    }

    #[test]
    fn saga_reads_prj_sidecar_into_srs() {
        let sgrd = tmp_path(".sgrd");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -99999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &sgrd).unwrap();

        let prj = with_extension(&sgrd, "prj");
        let wkt = "GEOGCS[\"WGS 84\"]";
        std::fs::write(&prj, wkt).unwrap();

        let r2 = read(&sgrd).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt));

        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
        let _ = std::fs::remove_file(prj);
    }

    #[test]
    fn saga_writes_prj_sidecar_from_metadata_when_srs_unknown() {
        let sgrd = tmp_path(".sgrd");
        let mut cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -99999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let wkt = "GEOGCS[\"WGS 84\"]";
        cfg.metadata
            .push(("saga_prj_text".to_string(), wkt.to_string()));

        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &sgrd).unwrap();

        let prj = with_extension(&sgrd, "prj");
        let text = std::fs::read_to_string(&prj).unwrap();
        assert_eq!(text.trim(), wkt);

        let r2 = read(&sgrd).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt));
        assert!(r2
            .metadata
            .iter()
            .any(|(k, v)| k == "saga_prj_text" && v.trim() == wkt));

        let _ = std::fs::remove_file(&sgrd);
        let _ = std::fs::remove_file(with_extension(&sgrd, "sdat"));
        let _ = std::fs::remove_file(prj);
    }
}
