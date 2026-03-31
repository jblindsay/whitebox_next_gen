//! Idrisi / TerrSet Raster format (`.rst` / `.rdc`).
//!
//! Two files per raster:
//! - `<n>.rdc` — plain-text header ("Resource Documentation")
//! - `<n>.rst` — binary data (little-endian, real32 by default)
//!
//! Reference: Clark Labs Idrisi format specification, and GDAL source
//! (`frmts/idrisi/IdrisiDataset.cpp`).
//!
//! Key `.rdc` fields:
//! ```text
//! file format : IDRISI Raster A.1
//! file title  : <title>
//! data type   : real   (byte|integer|real)
//! file type   : binary
//! columns     : <int>
//! rows        : <int>
//! ref. system : latlong / utm / plane
//! ref. units  : deg / m
//! unit dist.  : 1.0000000
//! min. X      : <f64>   (left edge)
//! max. X      : <f64>   (right edge)
//! min. Y      : <f64>   (bottom edge)
//! max. Y      : <f64>   (top edge)
//! pos'n error : unknown
//! resolution  : <f64>   (cell size)
//! min. value  : <f64>
//! max. value  : <f64>
//! display min : <f64>
//! display max : <f64>
//! value units : unspecified
//! value error : unknown
//! flag value  : -9999
//! flag def'n  : missing data
//! legend cats : 0
//! ```

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::*;
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an Idrisi/TerrSet raster. `path` may be the `.rdc` or `.rst` file.
pub fn read(path: &str) -> Result<Raster> {
    let rdc_path = if path.ends_with(".rst") {
        with_extension(path, "rdc")
    } else {
        path.to_string()
    };
    read_from_header(&rdc_path)
}

/// Write an Idrisi/TerrSet raster. `path` should be the `.rdc` path (the `.rst` is written alongside it).
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let rdc_path = if path.ends_with(".rst") {
        with_extension(path, "rdc")
    } else {
        path.to_string()
    };
        if raster.bands != 1 {
            return Err(RasterError::UnsupportedDataType(
                "Idrisi writer currently supports single-band rasters only".into(),
            ));
        }
    let rst_path = with_extension(&rdc_path, "rst");
    write_rdc(raster, &rdc_path)?;
    write_rst(raster, &rst_path)?;
    write_ref_sidecar(raster, &rdc_path)
}

// ─── Header ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct IdrisiHeader {
    data_type: DataType,
    byte_order: IdrisiByteOrder,
    cols: usize,
    rows: usize,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    nodata: f64,
    title: String,
    ref_system: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdrisiByteOrder {
    Little,
    Big,
}

fn parse_rdc(path: &str) -> Result<IdrisiHeader> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut data_type = DataType::F32;
    let mut cols: Option<usize> = None;
    let mut rows: Option<usize> = None;
    let mut byte_order = IdrisiByteOrder::Little;
    let mut x_min: Option<f64> = None;
    let mut x_max: Option<f64> = None;
    let mut y_min: Option<f64> = None;
    let mut y_max: Option<f64> = None;
    let mut nodata = -9999.0_f64;
    let mut title = String::new();
    let mut ref_system = String::from("latlong");

    for line in reader.lines() {
        let line = line?;
        // Idrisi uses `: ` as separator
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') { continue; }
        let colon_pos = match line.find(':') {
            Some(p) => p,
            None => continue,
        };
        let key = line[..colon_pos].trim().to_ascii_lowercase();
        let val = line[colon_pos + 1..].trim().to_string();

        match key.as_str() {
            "file title"   => title = val,
            "data type"    => data_type = idrisi_data_type(&val),
            "columns"      => cols = Some(parse_usize("columns", &val)?),
            "rows"         => rows = Some(parse_usize("rows", &val)?),
            "min. x"       => x_min = Some(parse_f64("min. x", &val)?),
            "max. x"       => x_max = Some(parse_f64("max. x", &val)?),
            "min. y"       => y_min = Some(parse_f64("min. y", &val)?),
            "max. y"       => y_max = Some(parse_f64("max. y", &val)?),
            "flag value"   => nodata = parse_f64("flag value", &val).unwrap_or(-9999.0),
            "ref. system"  => ref_system = val,
            "byteorder"    => byte_order = parse_byte_order(&val),
            _ => {}
        }
    }

    let cols  = cols .ok_or_else(|| RasterError::MissingField("columns".into()))?;
    let rows  = rows .ok_or_else(|| RasterError::MissingField("rows".into()))?;
    let x_min = x_min.ok_or_else(|| RasterError::MissingField("min. X".into()))?;
    let x_max = x_max.ok_or_else(|| RasterError::MissingField("max. X".into()))?;
    let y_min = y_min.ok_or_else(|| RasterError::MissingField("min. Y".into()))?;
    let y_max = y_max.ok_or_else(|| RasterError::MissingField("max. Y".into()))?;

    Ok(IdrisiHeader { data_type, byte_order, cols, rows, x_min, x_max, y_min, y_max, nodata, title, ref_system })
}

fn idrisi_data_type(s: &str) -> DataType {
    match s.trim().to_ascii_lowercase().as_str() {
        "byte"    => DataType::U8,
        "integer" => DataType::I16,
        "real"    => DataType::F32,
        "rgb24"   => DataType::U32,
        _         => DataType::F32,
    }
}

fn idrisi_data_type_str(dt: DataType) -> Result<&'static str> {
    match dt {
        DataType::U8 => Ok("byte"),
        DataType::I16 => Ok("integer"),
        DataType::F32 => Ok("real"),
        DataType::U32 => Ok("RGB24"),
        _ => Err(RasterError::UnsupportedDataType(
            format!("Idrisi/TerrSet writer does not support {:?}", dt),
        )),
    }
}

fn parse_byte_order(s: &str) -> IdrisiByteOrder {
    let lower = s.trim().to_ascii_lowercase();
    if lower.contains("big") || lower.contains("msb") {
        IdrisiByteOrder::Big
    } else {
        IdrisiByteOrder::Little
    }
}

fn byte_order_str(byte_order: IdrisiByteOrder) -> &'static str {
    match byte_order {
        IdrisiByteOrder::Little => "LITTLE_ENDIAN",
        IdrisiByteOrder::Big => "BIG_ENDIAN",
    }
}

// ─── Read ─────────────────────────────────────────────────────────────────────

fn read_from_header(rdc_path: &str) -> Result<Raster> {
    let hdr = parse_rdc(rdc_path)?;
    let rst_path = with_extension(rdc_path, "rst");
    let data = read_rst(&rst_path, &hdr)?;

    let cell_size = (hdr.x_max - hdr.x_min) / hdr.cols as f64;
    let cell_size_y = (hdr.y_max - hdr.y_min) / hdr.rows as f64;

    let ref_text = read_ref_sidecar(rdc_path);
    let mut metadata = vec![
        ("title".to_string(), hdr.title),
        ("ref_system".to_string(), hdr.ref_system),
        (
            "color_interpretation".to_string(),
            if hdr.data_type == DataType::U32 {
                "packed_rgb".to_string()
            } else {
                "unknown".to_string()
            },
        ),
    ];
    let crs = if let Some(ref text) = ref_text {
        metadata.push(("idrisi_ref_text".to_string(), text.clone()));
        if wkt_like(text) {
            CrsInfo::from_wkt(text.clone())
        } else {
            CrsInfo::default()
        }
    } else {
        CrsInfo::default()
    };

    let cfg = RasterConfig {
        cols: hdr.cols,
        rows: hdr.rows,
        x_min: hdr.x_min,
        y_min: hdr.y_min,
        cell_size,
        cell_size_y: Some(cell_size_y),
        nodata: hdr.nodata,
        data_type: hdr.data_type,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

fn read_ref_sidecar(rdc_path: &str) -> Option<String> {
    let ref_path = with_extension(rdc_path, "ref");
    std::fs::read_to_string(ref_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_ref_sidecar(raster: &Raster, rdc_path: &str) -> Result<()> {
    let ref_text = raster
        .metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("idrisi_ref_text"))
        .map(|(_, v)| v.as_str())
        .or(raster.crs.wkt.as_deref());

    if let Some(text) = ref_text {
        let ref_path = with_extension(rdc_path, "ref");
        std::fs::write(ref_path, text)?;
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

fn read_rst(path: &str, hdr: &IdrisiHeader) -> Result<Vec<f64>> {
    use std::io::Read;
    let mut file = BufReader::with_capacity(512 * 1024, File::open(path)?);
    let n = hdr.cols * hdr.rows;
    let mut data = Vec::with_capacity(n);

    // Idrisi stores rows north-to-south (top to bottom) — same as our layout.
    match hdr.data_type {
        DataType::U8 => {
            let mut buf = vec![0u8; n];
            file.read_exact(&mut buf)?;
            data.extend(buf.iter().map(|&b| b as f64));
        }
        DataType::I16 => {
            for _ in 0..n {
                let value = match hdr.byte_order {
                    IdrisiByteOrder::Little => read_i16_le_stream(&mut file)?,
                    IdrisiByteOrder::Big => read_i16_be_stream(&mut file)?,
                };
                data.push(value as f64);
            }
        }
        DataType::F32 => {
            for _ in 0..n {
                let value = match hdr.byte_order {
                    IdrisiByteOrder::Little => read_f32_le_stream(&mut file)?,
                    IdrisiByteOrder::Big => read_f32_be_stream(&mut file)?,
                };
                data.push(value as f64);
            }
        }
        DataType::U32 => {
            let mut bgr = vec![0u8; n * 3];
            file.read_exact(&mut bgr)?;
            for i in 0..n {
                let b = bgr[i * 3] as u32;
                let g = bgr[i * 3 + 1] as u32;
                let r = bgr[i * 3 + 2] as u32;
                let argb = (255u32 << 24) | (r << 16) | (g << 8) | b;
                data.push(argb as f64);
            }
        }
        _ => {
            return Err(RasterError::UnsupportedDataType(
                format!("Idrisi/TerrSet reader does not support {:?}", hdr.data_type),
            ));
        }
    }
    Ok(data)
}

// ─── Write ────────────────────────────────────────────────────────────────────

fn write_rdc(raster: &Raster, rdc_path: &str) -> Result<()> {
    let title = raster.metadata.iter()
        .find(|(k, _)| k == "title")
        .map(|(_, v)| v.as_str())
        .unwrap_or("untitled");
    let ref_system = raster.metadata.iter()
        .find(|(k, _)| k == "ref_system")
        .map(|(_, v)| v.as_str())
        .unwrap_or("plane");
    let byte_order = raster.metadata.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("byteorder"))
        .map(|(_, v)| parse_byte_order(v))
        .unwrap_or(IdrisiByteOrder::Little);
    let stats = raster.statistics();

    let mut w = BufWriter::new(File::create(rdc_path)?);
    writeln!(w, "file format : IDRISI Raster A.1")?;
    writeln!(w, "file title  : {title}")?;
    writeln!(w, "data type   : {}", idrisi_data_type_str(raster.data_type)?)?;
    writeln!(w, "file type   : binary")?;
    writeln!(w, "columns     : {}", raster.cols)?;
    writeln!(w, "rows        : {}", raster.rows)?;
    writeln!(w, "ref. system : {ref_system}")?;
    writeln!(w, "ref. units  : m")?;
    writeln!(w, "unit dist.  : 1.0000000")?;
    writeln!(w, "min. X      : {}", format_float(raster.x_min, 7))?;
    writeln!(w, "max. X      : {}", format_float(raster.x_max(), 7))?;
    writeln!(w, "min. Y      : {}", format_float(raster.y_min, 7))?;
    writeln!(w, "max. Y      : {}", format_float(raster.y_max(), 7))?;
    writeln!(w, "pos'n error : unknown")?;
    writeln!(w, "resolution  : {}", format_float(raster.cell_size_x, 7))?;
    writeln!(w, "min. value  : {}", format_float(stats.min, 6))?;
    writeln!(w, "max. value  : {}", format_float(stats.max, 6))?;
    writeln!(w, "display min : {}", format_float(stats.min, 6))?;
    writeln!(w, "display max : {}", format_float(stats.max, 6))?;
    writeln!(w, "value units : unspecified")?;
    writeln!(w, "value error : unknown")?;
    writeln!(w, "flag value  : {}", format_float(raster.nodata, 6))?;
    writeln!(w, "flag def'n  : missing data")?;
    writeln!(w, "legend cats : 0")?;
    writeln!(w, "byteorder   : {}", byte_order_str(byte_order))?;
    w.flush()?;
    Ok(())
}

fn write_rst(raster: &Raster, rst_path: &str) -> Result<()> {
    let mut w = BufWriter::with_capacity(512 * 1024, File::create(rst_path)?);
    let byte_order = raster.metadata.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("byteorder"))
        .map(|(_, v)| parse_byte_order(v))
        .unwrap_or(IdrisiByteOrder::Little);
    // Idrisi: row-major, north to south (row 0 first) — same as our layout.
    match raster.data_type {
        DataType::U8 => {
            for v in raster.data.iter_f64() {
                w.write_all(&[v as u8])?;
            }
        }
        DataType::I16 => {
            for v in raster.data.iter_f64() {
                let bytes = match byte_order {
                    IdrisiByteOrder::Little => (v as i16).to_le_bytes(),
                    IdrisiByteOrder::Big => (v as i16).to_be_bytes(),
                };
                w.write_all(&bytes)?;
            }
        }
        DataType::F32 => {
            for v in raster.data.iter_f64() {
                let bytes = match byte_order {
                    IdrisiByteOrder::Little => (v as f32).to_le_bytes(),
                    IdrisiByteOrder::Big => (v as f32).to_be_bytes(),
                };
                w.write_all(&bytes)?;
            }
        }
        DataType::U32 => {
            // RGB24 stored as B, G, R bytes interleaved by pixel.
            for v in raster.data.iter_f64() {
                let rgba = v as u32;
                let b = (rgba & 0x0000_00FF) as u8;
                let g = ((rgba >> 8) & 0x0000_00FF) as u8;
                let r = ((rgba >> 16) & 0x0000_00FF) as u8;
                w.write_all(&[b, g, r])?;
            }
        }
        _ => {
            return Err(RasterError::UnsupportedDataType(
                format!("Idrisi/TerrSet writer does not support {:?}", raster.data_type),
            ));
        }
    }
    w.flush()?;
    Ok(())
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
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(suffix: &str) -> String {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        temp_dir().join(format!("idrisi_test_{ts}{suffix}")).to_string_lossy().into_owned()
    }

    #[test]
    fn idrisi_roundtrip() {
        let rdc = tmp(".rdc");
        let cfg = RasterConfig {
            cols: 4, rows: 3, cell_size: 10.0, x_min: 0.0, y_min: 0.0,
            nodata: -9999.0, data_type: DataType::F32, ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &rdc).unwrap();
        let r2 = read(&rdc).unwrap();
        assert_eq!(r2.cols, 4);
        assert_eq!(r2.rows, 3);
        assert!((r2.get(0, 0, 0) - 0.0).abs() < 1e-4);
        assert!((r2.get(0, 2, 3) - 11.0).abs() < 1e-4);
        let _ = std::fs::remove_file(&rdc);
        let _ = std::fs::remove_file(with_extension(&rdc, "rst"));
    }

    #[test]
    fn idrisi_roundtrip_i16() {
        let rdc = tmp(".rdc");
        let cfg = RasterConfig {
            cols: 3,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -9999.0,
            data_type: DataType::I16,
            ..Default::default()
        };
        let data: Vec<f64> = vec![-10.0, -1.0, 0.0, 1.0, 2.0, 327.0];
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &rdc).unwrap();
        let r2 = read(&rdc).unwrap();
        assert_eq!(r2.data_type, DataType::I16);
        assert_eq!(r2.cols, 3);
        assert_eq!(r2.rows, 2);
        assert_eq!(r2.get(0, 0, 0), -10.0);
        assert_eq!(r2.get(0, 1, 2), 327.0);
        let _ = std::fs::remove_file(&rdc);
        let _ = std::fs::remove_file(with_extension(&rdc, "rst"));
    }

    #[test]
    fn idrisi_reads_big_endian_f32() {
        let rdc = tmp(".rdc");
        let rst = with_extension(&rdc, "rst");

        let mut hw = BufWriter::new(File::create(&rdc).unwrap());
        writeln!(hw, "file format : IDRISI Raster A.1").unwrap();
        writeln!(hw, "file title  : be").unwrap();
        writeln!(hw, "data type   : real").unwrap();
        writeln!(hw, "file type   : binary").unwrap();
        writeln!(hw, "columns     : 2").unwrap();
        writeln!(hw, "rows        : 2").unwrap();
        writeln!(hw, "ref. system : plane").unwrap();
        writeln!(hw, "ref. units  : m").unwrap();
        writeln!(hw, "unit dist.  : 1.0000000").unwrap();
        writeln!(hw, "min. X      : 0").unwrap();
        writeln!(hw, "max. X      : 2").unwrap();
        writeln!(hw, "min. Y      : 0").unwrap();
        writeln!(hw, "max. Y      : 2").unwrap();
        writeln!(hw, "flag value  : -9999").unwrap();
        writeln!(hw, "byteorder   : BIG_ENDIAN").unwrap();
        hw.flush().unwrap();

        let vals = [1.25_f32, -2.5_f32, 3.75_f32, 4.0_f32];
        let mut dw = BufWriter::new(File::create(&rst).unwrap());
        for v in vals {
            dw.write_all(&v.to_be_bytes()).unwrap();
        }
        dw.flush().unwrap();

        let r = read(&rdc).unwrap();
        assert!((r.get(0, 0, 0) - 1.25).abs() < 1e-6);
        assert!((r.get(0, 0, 1) - (-2.5)).abs() < 1e-6);
        assert!((r.get(0, 1, 0) - 3.75).abs() < 1e-6);
        assert!((r.get(0, 1, 1) - 4.0).abs() < 1e-6);

        let _ = std::fs::remove_file(&rdc);
        let _ = std::fs::remove_file(&rst);
    }

    #[test]
    fn idrisi_writes_ref_sidecar_when_srs_wkt_present() {
        let rdc = tmp(".rdc");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            crs: CrsInfo::from_wkt("GEOGCS[\"WGS 84\"]"),
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &rdc).unwrap();

        let ref_path = with_extension(&rdc, "ref");
        let ref_text = std::fs::read_to_string(&ref_path).unwrap();
        assert_eq!(ref_text.trim(), "GEOGCS[\"WGS 84\"]");

        let _ = std::fs::remove_file(&rdc);
        let _ = std::fs::remove_file(with_extension(&rdc, "rst"));
        let _ = std::fs::remove_file(ref_path);
    }

    #[test]
    fn idrisi_reads_ref_sidecar_into_srs() {
        let rdc = tmp(".rdc");
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            cell_size: 1.0,
            x_min: 0.0,
            y_min: 0.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &rdc).unwrap();

        let ref_path = with_extension(&rdc, "ref");
        let wkt = "GEOGCS[\"WGS 84\"]";
        std::fs::write(&ref_path, wkt).unwrap();

        let r2 = read(&rdc).unwrap();
        assert_eq!(r2.crs.wkt.as_deref(), Some(wkt));
        assert_eq!(
            r2.metadata
                .iter()
                .find(|(k, _)| k == "idrisi_ref_text")
                .map(|(_, v)| v.as_str()),
            Some(wkt)
        );

        let _ = std::fs::remove_file(&rdc);
        let _ = std::fs::remove_file(with_extension(&rdc, "rst"));
        let _ = std::fs::remove_file(ref_path);
    }
}
