//! GRASS ASCII raster format.
//!
//! Common header fields (case-insensitive):
//! - `north`, `south`, `east`, `west`
//! - `rows`, `cols`
//! - `null` (optional, numeric or string token)
//! - `type` (`int`, `float`, `double`) (optional)

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{RasterError, Result};
use crate::io_utils::{format_float, with_extension};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

/// Read a GRASS ASCII raster from `path`.
pub fn read(path: &str) -> Result<Raster> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(128 * 1024, file);
    parse_with_source(reader, path)
}

/// Parse GRASS ASCII raster text from any buffered reader.
pub fn parse<R: BufRead>(reader: R) -> Result<Raster> {
    parse_with_source(reader, "")
}

fn parse_with_source<R: BufRead>(reader: R, source: &str) -> Result<Raster> {
    let mut rows: Option<usize> = None;
    let mut cols: Option<usize> = None;
    let mut north: Option<f64> = None;
    let mut south: Option<f64> = None;
    let mut east: Option<f64> = None;
    let mut west: Option<f64> = None;
    let mut cellsize: Option<f64> = None;
    let mut nodata = -9999.0f64;
    let mut nodata_token: Option<String> = None;
    let mut data_type = DataType::F64;

    let mut data: Vec<f64> = Vec::new();
    let mut in_data = false;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !in_data {
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_ascii_lowercase();
                let val = v.trim();
                match key.as_str() {
                    "rows" => rows = Some(parse_usize("rows", val)?),
                    "cols" => cols = Some(parse_usize("cols", val)?),
                    "north" => north = Some(parse_f64("north", val)?),
                    "south" => south = Some(parse_f64("south", val)?),
                    "east" => east = Some(parse_f64("east", val)?),
                    "west" => west = Some(parse_f64("west", val)?),
                    "cellsize" => cellsize = Some(parse_f64("cellsize", val)?),
                    "null" => {
                        if let Ok(v) = val.parse::<f64>() {
                            nodata = v;
                            nodata_token = None;
                        } else {
                            nodata_token = Some(val.to_string());
                        }
                    }
                    "type" => {
                        let t = val.to_ascii_lowercase();
                        data_type = if t.contains("double") {
                            DataType::F64
                        } else if t.contains("float") {
                            DataType::F32
                        } else {
                            DataType::I32
                        };
                    }
                    _ => {}
                }
                continue;
            }
            in_data = true;
        }

        for token in trimmed.split_ascii_whitespace() {
            if let Some(null_tok) = &nodata_token {
                if token == null_tok {
                    data.push(nodata);
                    continue;
                }
            }
            let v: f64 = token.parse().map_err(|_| {
                RasterError::CorruptData(format!("invalid data token in GRASS ASCII: '{token}'"))
            })?;
            data.push(v);
        }
    }

    let rows = rows.ok_or_else(|| RasterError::MissingField("rows".into()))?;
    let cols = cols.ok_or_else(|| RasterError::MissingField("cols".into()))?;
    let north = north.ok_or_else(|| RasterError::MissingField("north".into()))?;
    let south = south.ok_or_else(|| RasterError::MissingField("south".into()))?;
    let east = east.ok_or_else(|| RasterError::MissingField("east".into()))?;
    let west = west.ok_or_else(|| RasterError::MissingField("west".into()))?;

    if rows == 0 || cols == 0 {
        return Err(RasterError::InvalidDimensions { cols, rows });
    }

    let expected = rows * cols;
    if data.len() < expected {
        return Err(RasterError::CorruptData(format!(
            "expected {expected} values, got {}",
            data.len()
        )));
    }
    data.truncate(expected);

    let x_res = (east - west) / cols as f64;
    let y_res = (north - south) / rows as f64;
    let x_res = if let Some(cs) = cellsize { cs } else { x_res };
    let y_res = if let Some(cs) = cellsize { cs } else { y_res };

    let prj_text = read_prj_sidecar(source);
    let mut metadata = Vec::new();
    let crs = if let Some(ref text) = prj_text {
        metadata.push(("grass_ascii_prj_text".to_string(), text.clone()));
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
        x_min: west,
        y_min: south,
        cell_size: x_res,
        cell_size_y: Some(y_res),
        nodata,
        data_type,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

/// Write a GRASS ASCII raster to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);
    write_to(&mut writer, raster)?;
    write_prj_sidecar(raster, path)
}

/// Write GRASS ASCII raster text to any writer.
pub fn write_to<W: Write>(w: &mut W, raster: &Raster) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "GRASS ASCII writer currently supports single-band rasters only".into(),
        ));
    }

    writeln!(w, "north: {}", format_float(raster.y_max(), 10))?;
    writeln!(w, "south: {}", format_float(raster.y_min, 10))?;
    writeln!(w, "east: {}", format_float(raster.x_max(), 10))?;
    writeln!(w, "west: {}", format_float(raster.x_min, 10))?;
    writeln!(w, "rows: {}", raster.rows)?;
    writeln!(w, "cols: {}", raster.cols)?;
    writeln!(w, "null: {}", format_float(raster.nodata, 10))?;

    let typ = match raster.data_type {
        DataType::F64 => "double",
        DataType::F32 => "float",
        _ => "int",
    };
    writeln!(w, "type: {typ}")?;

    let decimals = match raster.data_type {
        DataType::F32 | DataType::F64 => 6,
        _ => 0,
    };

    for row in 0..raster.rows {
        let slice = raster.row_slice(0, row as isize);
        let mut first = true;
        for v in slice {
            if !first {
                write!(w, " ")?;
            }
            first = false;
            write!(w, "{}", format_float(v, decimals))?;
        }
        writeln!(w)?;
    }
    Ok(())
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

fn read_prj_sidecar(source: &str) -> Option<String> {
    if source.trim().is_empty() {
        return None;
    }
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
        .find(|(k, _)| k.eq_ignore_ascii_case("grass_ascii_prj_text"))
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
    use std::path::Path;

    const SAMPLE: &str = "\
north: 40
south: 10
east: 25
west: 5
rows: 3
cols: 4
null: -9999
type: float
1 2 3 4
5 -9999 7 8
9 10 11 12
";

    #[test]
    fn parse_grass_ascii() {
        let r = parse(BufReader::new(Cursor::new(SAMPLE))).unwrap();
        assert_eq!(r.cols, 4);
        assert_eq!(r.rows, 3);
        assert_eq!(r.x_min, 5.0);
        assert_eq!(r.y_min, 10.0);
        assert!((r.cell_size_x - 5.0).abs() < 1e-10);
        assert!((r.cell_size_y - 10.0).abs() < 1e-10);
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert!(r.is_nodata(r.get(0, 1, 1)));
    }

    #[test]
    fn roundtrip_grass_ascii() {
        let r = parse(BufReader::new(Cursor::new(SAMPLE))).unwrap();
        let mut out = Vec::new();
        write_to(&mut out, &r).unwrap();
        let r2 = parse(BufReader::new(Cursor::new(&out))).unwrap();
        assert_eq!(r2.cols, r.cols);
        assert_eq!(r2.rows, r.rows);
        assert_eq!(r2.get(0, 2, 3), 12.0);
        assert!(r2.is_nodata(r2.get(0, 1, 1)));
    }

    #[test]
    fn grass_ascii_writes_and_reads_prj_sidecar() {
        let path = std::env::temp_dir()
            .join("wbraster_grass_ascii_prj_unit_test.asc")
            .to_string_lossy()
            .into_owned();
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
            .any(|(k, v)| k == "grass_ascii_prj_text" && v.trim() == wkt));

        let _ = std::fs::remove_file(&path);
        if Path::new(&prj).exists() {
            let _ = std::fs::remove_file(&prj);
        }
    }
}
