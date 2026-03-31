//! ER Mapper Raster format (`.ers` header + binary data).
//!
//! An ER Mapper raster consists of:
//! - A plain-text `.ers` header file  (UTF-8 / ASCII)
//! - A binary data file with the same name but without the `.ers` extension,
//!   or pointed to by the `DataFile` field in the header.
//!
//! The header uses a hierarchical brace syntax:
//! ```text
//! DatasetHeader Begin
//!     Version         = "6.0"
//!     Name            = "dem"
//!     LastUpdated     = ...
//!     DataFile        = ""
//!     HeaderOffset    = 0
//!     OldHeaderOffset = 0
//!     ByteOrder       = LSBFirst
//!     DataSetType     = ERStorage
//!     DataType        = Raster
//!     BytesPerCell    = 4
//!     CoordinateSpace Begin
//!         Datum           = "WGS84"
//!         Projection      = "GEOGRAPHIC"
//!         CoordinateType  = LL
//!         Rotation        = 0:0:0.000
//!     CoordinateSpace End
//!     RasterInfo Begin
//!         CellType        = IEEE4ByteReal
//!         NullCellValue   = -9999.0
//!         Xdimension      = 0.001
//!         Ydimension      = 0.001
//!         NrOfLines       = 200
//!         NrOfCellsPerLine = 300
//!         RegistrationCoord Begin
//!             Eastings        = 100.000
//!             Northings       = -30.000
//!         RegistrationCoord End
//!         RegistrationCellX = 0.5
//!         RegistrationCellY = 0.5
//!     RasterInfo End
//!     ...
//! DatasetHeader End
//! ```
//!
//! The `RegistrationCoord` refers to the cell-center of pixel (RegistrationCellX, RegistrationCellY)
//! (0-based).  Usually RegistrationCellX = RegistrationCellY = 0.5, meaning the upper-left corner
//! of the first pixel, which is the same as `(Eastings - Xdimension*0.5, Northings + Ydimension*0.5)`
//! for the upper-left corner.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::*;
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an ER Mapper raster. `path` should be the `.ers` header file.
pub fn read(path: &str) -> Result<Raster> {
    let hdr = parse_ers_header(path)?;
    let data_path = ers_data_path(path, &hdr.data_file);
    let data = read_data(&data_path, &hdr)?;
    let cell_size = hdr.x_dim; // assume square pixels

    // Registration: the RegistrationCoord is the cell-center of pixel (regX, regY).
    // Top-left corner of data:
    //   x_min = reg_easting  - regX * x_dim  (cell-center -> left edge of pixel -> then subtract regX cells)
    //   y_max = reg_northing + regY * y_dim
    let x_min = hdr.reg_easting  - hdr.reg_cell_x * hdr.x_dim;
    let y_max = hdr.reg_northing + hdr.reg_cell_y * hdr.y_dim;
    let y_min = y_max - hdr.rows as f64 * hdr.y_dim;

    let crs = if wkt_like(&hdr.datum) {
        CrsInfo::from_wkt(hdr.datum.clone())
    } else {
        CrsInfo::default()
    };

    let mut metadata = Vec::new();
    if !hdr.datum.is_empty() {
        metadata.push(("er_datum".to_string(), hdr.datum.clone()));
    }
    if !hdr.projection.is_empty() {
        metadata.push(("er_projection".to_string(), hdr.projection.clone()));
    }
    if !hdr.coordinate_type.is_empty() {
        metadata.push(("er_coordinate_type".to_string(), hdr.coordinate_type.clone()));
    }

    let cfg = RasterConfig {
        cols: hdr.cols,
        rows: hdr.rows,
        x_min,
        y_min,
        cell_size,
        nodata: hdr.nodata,
        data_type: hdr.cell_type,
        crs: crs,        metadata,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

/// Write an ER Mapper raster. `path` is the `.ers` header path.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "ER Mapper writer currently supports single-band rasters only".into(),
        ));
    }
    let ers_path = if !path.ends_with(".ers") {
        format!("{path}.ers")
    } else {
        path.to_string()
    };
    // Data file = ers path without the .ers suffix
    let data_path = ers_path.trim_end_matches(".ers").to_string();
    write_ers_header(raster, &ers_path, &data_path)?;
    write_data(raster, &data_path)
}

// ─── Header parsing ───────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct ErsHeader {
    data_file: String,
    byte_order_le: bool,
    cell_type: DataType,
    nodata: f64,
    x_dim: f64,
    y_dim: f64,
    rows: usize,
    cols: usize,
    reg_easting: f64,
    reg_northing: f64,
    reg_cell_x: f64,
    reg_cell_y: f64,
    datum: String,
    projection: String,
    coordinate_type: String,
}

fn parse_ers_header(path: &str) -> Result<ErsHeader> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut hdr = ErsHeader {
        byte_order_le: true,
        cell_type: DataType::F32,
        nodata: -9999.0,
        reg_cell_x: 0.5,
        reg_cell_y: 0.5,
        ..Default::default()
    };

    // Track nesting context
    let mut in_raster = false;
    let mut in_coord   = false;
    let mut in_reg     = false;

    for line_res in reader.lines() {
        let line = line_res?;
        let line = line.trim();
        if line.is_empty() { continue; }

        // Context pushes
        if line.eq_ignore_ascii_case("RasterInfo Begin")     { in_raster = true; continue; }
        if line.eq_ignore_ascii_case("RasterInfo End")       { in_raster = false; continue; }
        if line.eq_ignore_ascii_case("CoordinateSpace Begin"){ in_coord  = true; continue; }
        if line.eq_ignore_ascii_case("CoordinateSpace End")  { in_coord  = false; continue; }
        if line.eq_ignore_ascii_case("RegistrationCoord Begin") { in_reg = true; continue; }
        if line.eq_ignore_ascii_case("RegistrationCoord End")   { in_reg = false; continue; }
        if line.to_ascii_lowercase().ends_with("begin") || line.to_ascii_lowercase().ends_with("end") {
            continue;
        }

        let (key, val) = match parse_key_value(line) {
            Some(kv) => kv,
            None => continue,
        };
        // Strip surrounding quotes from values
        let val = val.trim_matches('"').to_string();

        if in_reg {
            match key.as_str() {
                "eastings"  => hdr.reg_easting  = parse_f64_h("Eastings", &val)?,
                "northings" => hdr.reg_northing = parse_f64_h("Northings", &val)?,
                _ => {}
            }
        } else if in_raster {
            match key.as_str() {
                "celltype"          => hdr.cell_type = parse_cell_type(&val),
                "nullcellvalue"     => hdr.nodata     = parse_f64_h("NullCellValue", &val).unwrap_or(-9999.0),
                "xdimension"        => hdr.x_dim       = parse_f64_h("Xdimension", &val)?,
                "ydimension"        => hdr.y_dim       = parse_f64_h("Ydimension", &val)?,
                "nroflines"         => hdr.rows         = parse_usize_h("NrOfLines", &val)?,
                "nrofcellsperline"  => hdr.cols         = parse_usize_h("NrOfCellsPerLine", &val)?,
                "registrationcellx" => hdr.reg_cell_x   = parse_f64_h("RegistrationCellX", &val).unwrap_or(0.5),
                "registrationcelly" => hdr.reg_cell_y   = parse_f64_h("RegistrationCellY", &val).unwrap_or(0.5),
                _ => {}
            }
        } else if in_coord {
            match key.as_str() {
                "datum"      => hdr.datum      = val,
                "projection" => hdr.projection = val,
                "coordinatetype" => hdr.coordinate_type = val,
                _ => {}
            }
        } else {
            match key.as_str() {
                "datafile"  => hdr.data_file   = val,
                "byteorder" => hdr.byte_order_le = !val.eq_ignore_ascii_case("MSBFirst"),
                _ => {}
            }
        }
    }

    if hdr.cols == 0 || hdr.rows == 0 {
        return Err(RasterError::MissingField("NrOfLines / NrOfCellsPerLine".into()));
    }
    if hdr.x_dim == 0.0 {
        return Err(RasterError::MissingField("Xdimension".into()));
    }
    Ok(hdr)
}

fn parse_cell_type(s: &str) -> DataType {
    match s.to_ascii_lowercase().as_str() {
        "ieee4bytereal" | "float" | "float32" => DataType::F32,
        "ieee8bytereal" | "double" | "float64" => DataType::F64,
        "unsignedinteger" | "uint8" | "byte"  => DataType::U8,
        "signedinteger16" | "int16" | "short" => DataType::I16,
        "signedinteger32" | "int32" | "int"   => DataType::I32,
        "unsignedinteger64" | "uint64" => DataType::U64,
        "signedinteger64" | "int64" => DataType::I64,
        _ => DataType::F32,
    }
}

fn cell_type_str(dt: DataType) -> &'static str {
    match dt {
        DataType::U8           => "Unsigned8BitInteger",
        DataType::I8           => "Signed8BitInteger",
        DataType::I16 | DataType::U16 => "Signed16BitInteger",
        DataType::I32 | DataType::U32 => "Signed32BitInteger",
        DataType::U64 => "Unsigned64BitInteger",
        DataType::I64 => "Signed64BitInteger",
        DataType::F32          => "IEEE4ByteReal",
        DataType::F64          => "IEEE8ByteReal",
    }
}

fn ers_data_path(ers_path: &str, data_file: &str) -> String {
    if !data_file.is_empty() {
        // Relative to the .ers directory
        let dir = std::path::Path::new(ers_path).parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        format!("{dir}/{data_file}")
    } else {
        // Default: strip .ers suffix
        ers_path.trim_end_matches(".ers").to_string()
    }
}

// ─── Read data ────────────────────────────────────────────────────────────────

fn read_data(path: &str, hdr: &ErsHeader) -> Result<Vec<f64>> {
    use std::io::Read;
    let mut file = BufReader::with_capacity(512 * 1024, File::open(path)?);
    let n = hdr.cols * hdr.rows;
    let le = hdr.byte_order_le;
    let mut data = Vec::with_capacity(n);

    match hdr.cell_type {
        DataType::U8 => {
            let mut buf = vec![0u8; n];
            file.read_exact(&mut buf)?;
            data.extend(buf.iter().map(|&b| b as f64));
        }
        DataType::I16 => {
            for _ in 0..n {
                let v = if le { read_i16_le_stream(&mut file)? } else { read_i16_be_stream(&mut file)? };
                data.push(v as f64);
            }
        }
        DataType::I32 => {
            for _ in 0..n {
                let v = if le { read_i32_le_stream(&mut file)? } else { read_i32_be_stream(&mut file)? };
                data.push(v as f64);
            }
        }
        DataType::F32 => {
            for _ in 0..n {
                let v = if le { read_f32_le_stream(&mut file)? } else { read_f32_be_stream(&mut file)? };
                data.push(v as f64);
            }
        }
        DataType::F64 => {
            for _ in 0..n {
                let v = if le { read_f64_le_stream(&mut file)? } else { read_f64_be_stream(&mut file)? };
                data.push(v);
            }
        }
        _ => return Err(RasterError::UnsupportedDataType(hdr.cell_type.to_string())),
    }
    Ok(data)
}

// ─── Write ────────────────────────────────────────────────────────────────────

fn write_ers_header(raster: &Raster, ers_path: &str, data_path: &str) -> Result<()> {
    let data_basename = std::path::Path::new(data_path)
        .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    // RegistrationCoord = upper-left corner of first pixel (center of first pixel).
    // We store as top-down, so y_max is the north edge.
    let reg_easting  = raster.x_min + raster.cell_size_x * 0.5;
    let reg_northing = raster.y_max() - raster.cell_size_y * 0.5;

    let md_val = |key: &str| -> Option<String> {
        raster
            .metadata
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    };

    let datum = md_val("er_datum")
        .or_else(|| raster.crs.wkt.clone())
        .unwrap_or_else(|| "WGS84".to_string());
    let projection = md_val("er_projection").unwrap_or_else(|| "GEOGRAPHIC".to_string());
    let coordinate_type = md_val("er_coordinate_type").unwrap_or_else(|| "LL".to_string());

    let mut w = BufWriter::new(File::create(ers_path)?);
    writeln!(w, "DatasetHeader Begin")?;
    writeln!(w, "\tVersion\t\t\t= \"6.0\"")?;
    writeln!(w, "\tDataFile\t\t= \"{data_basename}\"")?;
    writeln!(w, "\tHeaderOffset\t\t= 0")?;
    writeln!(w, "\tByteOrder\t\t= LSBFirst")?;
    writeln!(w, "\tDataSetType\t\t= ERStorage")?;
    writeln!(w, "\tDataType\t\t= Raster")?;
    writeln!(w, "\tCoordinateSpace Begin")?;
    writeln!(w, "\t\tDatum\t\t\t= \"{datum}\"")?;
    writeln!(w, "\t\tProjection\t\t= \"{projection}\"")?;
    writeln!(w, "\t\tCoordinateType\t= {coordinate_type}")?;
    writeln!(w, "\t\tRotation\t\t= 0:0:0.000")?;
    writeln!(w, "\tCoordinateSpace End")?;
    writeln!(w, "\tRasterInfo Begin")?;
    writeln!(w, "\t\tCellType\t\t= {}", cell_type_str(raster.data_type))?;
    writeln!(w, "\t\tNullCellValue\t= {}", format_float(raster.nodata, 6))?;
    writeln!(w, "\t\tXdimension\t\t= {}", format_float(raster.cell_size_x, 10))?;
    writeln!(w, "\t\tYdimension\t\t= {}", format_float(raster.cell_size_y, 10))?;
    writeln!(w, "\t\tNrOfLines\t\t= {}", raster.rows)?;
    writeln!(w, "\t\tNrOfCellsPerLine\t= {}", raster.cols)?;
    writeln!(w, "\t\tRegistrationCoord Begin")?;
    writeln!(w, "\t\t\tEastings\t\t= {}", format_float(reg_easting, 10))?;
    writeln!(w, "\t\t\tNorthings\t\t= {}", format_float(reg_northing, 10))?;
    writeln!(w, "\t\tRegistrationCoord End")?;
    writeln!(w, "\t\tRegistrationCellX\t= 0.5")?;
    writeln!(w, "\t\tRegistrationCellY\t= 0.5")?;
    writeln!(w, "\tRasterInfo End")?;
    writeln!(w, "DatasetHeader End")?;
    w.flush()?;
    Ok(())
}

fn wkt_like(s: &str) -> bool {
    let t = s.trim();
    let upper = t.to_ascii_uppercase();
    !t.is_empty() &&
    (upper.starts_with("GEOGCS[")
        || upper.starts_with("PROJCS[")
        || upper.starts_with("COMPOUNDCRS[")
        || upper.starts_with("GEODCRS[")
        || upper.starts_with("PROJCRS[")
        || upper.starts_with("VERTCRS["))
}

fn write_data(raster: &Raster, path: &str) -> Result<()> {
    let mut w = BufWriter::with_capacity(512 * 1024, File::create(path)?);
    // Little-endian F32 (most common ER Mapper convention)
    for v in raster.data.iter_f64() {
        w.write_all(&(v as f32).to_le_bytes())?;
    }
    w.flush()?;
    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_usize_h(field: &str, val: &str) -> Result<usize> {
    val.trim().parse::<usize>().map_err(|_| RasterError::ParseError {
        field: field.into(), value: val.into(), expected: "positive integer".into(),
    })
}

fn parse_f64_h(field: &str, val: &str) -> Result<f64> {
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
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let pid = std::process::id();
        temp_dir()
            .join(format!("ers_test_{pid}_{ts}{suffix}"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn ers_roundtrip() {
        let ers = tmp(".ers");
        let cfg = RasterConfig {
            cols: 4, rows: 3, cell_size: 0.01, x_min: 100.0, y_min: -30.03,
            nodata: -9999.0, data_type: DataType::F32, ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64 * 1.5).collect();
        let r = Raster::from_data(cfg, data).unwrap();
        write(&r, &ers).unwrap();
        let r2 = read(&ers).unwrap();
        assert_eq!(r2.cols, 4);
        assert_eq!(r2.rows, 3);
        assert!((r2.get(0, 0, 0) - 0.0).abs() < 1e-4, "got {:?}", r2.get(0, 0, 0));
        assert!((r2.get(0, 2, 3) - 16.5).abs() < 1e-3, "got {:?}", r2.get(0, 2, 3));
        let data_path = ers.trim_end_matches(".ers").to_string();
        let _ = std::fs::remove_file(&ers);
        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn ers_preserves_coordinate_space_metadata() {
        let ers = tmp(".ers");
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
        cfg.metadata.push(("er_datum".into(), "GDA94".into()));
        cfg.metadata.push(("er_projection".into(), "MGA55".into()));
        cfg.metadata.push(("er_coordinate_type".into(), "EN".into()));
        let r = Raster::from_data(cfg, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        write(&r, &ers).unwrap();

        let r2 = read(&ers).unwrap();
        let datum = r2
            .metadata
            .iter()
            .find(|(k, _)| k == "er_datum")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let proj = r2
            .metadata
            .iter()
            .find(|(k, _)| k == "er_projection")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let ctype = r2
            .metadata
            .iter()
            .find(|(k, _)| k == "er_coordinate_type")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        assert_eq!(datum, "GDA94");
        assert_eq!(proj, "MGA55");
        assert_eq!(ctype, "EN");
        assert!(r2.crs.wkt.is_none());

        let data_path = ers.trim_end_matches(".ers").to_string();
        let _ = std::fs::remove_file(&ers);
        let _ = std::fs::remove_file(&data_path);
    }

    #[test]
    fn ers_legacy_wkt_in_datum_populates_srs() {
        let ers = tmp(".ers");
        let data_path = ers.trim_end_matches(".ers").to_string();
        let wkt = "GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]]]";

        let mut w = BufWriter::new(File::create(&ers).unwrap());
        writeln!(w, "DatasetHeader Begin").unwrap();
        writeln!(w, "\tVersion\t\t\t= \"6.0\"").unwrap();
        writeln!(w, "\tDataFile\t\t= \"{}\"", std::path::Path::new(&data_path).file_name().unwrap().to_string_lossy()).unwrap();
        writeln!(w, "\tHeaderOffset\t\t= 0").unwrap();
        writeln!(w, "\tByteOrder\t\t= LSBFirst").unwrap();
        writeln!(w, "\tDataSetType\t\t= ERStorage").unwrap();
        writeln!(w, "\tDataType\t\t= Raster").unwrap();
        writeln!(w, "\tCoordinateSpace Begin").unwrap();
        writeln!(w, "\t\tDatum\t\t\t= \"{}\"", wkt).unwrap();
        writeln!(w, "\t\tProjection\t\t= \"GEOGRAPHIC\"").unwrap();
        writeln!(w, "\t\tCoordinateType\t= LL").unwrap();
        writeln!(w, "\t\tRotation\t\t= 0:0:0.000").unwrap();
        writeln!(w, "\tCoordinateSpace End").unwrap();
        writeln!(w, "\tRasterInfo Begin").unwrap();
        writeln!(w, "\t\tCellType\t\t= IEEE4ByteReal").unwrap();
        writeln!(w, "\t\tNullCellValue\t= -9999.0").unwrap();
        writeln!(w, "\t\tXdimension\t\t= 1").unwrap();
        writeln!(w, "\t\tYdimension\t\t= 1").unwrap();
        writeln!(w, "\t\tNrOfLines\t\t= 1").unwrap();
        writeln!(w, "\t\tNrOfCellsPerLine\t= 1").unwrap();
        writeln!(w, "\t\tRegistrationCoord Begin").unwrap();
        writeln!(w, "\t\t\tEastings\t\t= 0").unwrap();
        writeln!(w, "\t\t\tNorthings\t\t= 1").unwrap();
        writeln!(w, "\t\tRegistrationCoord End").unwrap();
        writeln!(w, "\t\tRegistrationCellX\t= 0.5").unwrap();
        writeln!(w, "\t\tRegistrationCellY\t= 0.5").unwrap();
        writeln!(w, "\tRasterInfo End").unwrap();
        writeln!(w, "DatasetHeader End").unwrap();
        w.flush().unwrap();

        let mut d = BufWriter::new(File::create(&data_path).unwrap());
        d.write_all(&(42.0_f32).to_le_bytes()).unwrap();
        d.flush().unwrap();

        let r = read(&ers).unwrap();
        assert_eq!(r.crs.wkt.as_deref(), Some(wkt));

        let _ = std::fs::remove_file(&ers);
        let _ = std::fs::remove_file(&data_path);
    }
}
