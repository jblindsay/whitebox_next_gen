//! DTED (Digital Terrain Elevation Data) format (`.dt0`, `.dt1`, `.dt2`).
//!
//! DTED files contain elevation in metres as big-endian signed 16-bit integers.
//! Each file covers a 1° × 1° geographic tile.
//!
//! ## File structure
//!
//! | Section | Size |
//! |---------|------|
//! | UHL – User Header Label | 80 bytes |
//! | DSI – Data Set Identification | 648 bytes |
//! | ACC – Accuracy Description | 2700 bytes |
//! | Data records (one per longitude column) | variable |
//!
//! Each data record:
//! - Byte 0: sentinel `0xAA`
//! - Bytes 1-3: block count (3-byte big-endian unsigned)
//! - Bytes 4-5: longitude count (column index, big-endian u16)
//! - Bytes 6-7: latitude count (always 0, big-endian u16)
//! - Bytes 8 … 8+num_rows×2-1: elevations (big-endian i16, south→north)
//! - Last 4 bytes: checksum (big-endian u32, sum of all data bytes)
//!
//! ## Special elevation values
//! - `-32767` (`0x8001`) — DTED void / missing data → mapped to nodata
//! - `-32768` (`0x8000`) — null value → mapped to nodata
//!
//! ## Coordinate encoding in UHL
//! Coordinates are encoded as `DDDMMSSh` (8 ASCII chars):
//! - DDD: degrees (zero-padded)
//! - MM: minutes (zero-padded)
//! - SS: seconds (zero-padded)
//! - h: hemisphere (`E`/`W` for longitude, `N`/`S` for latitude)
//!
//! Reference: MIL-PRF-89020B, DTED Level 0, 1, and 2 specification.

use std::fs::File;
use std::io::{BufWriter, Read, Write};

use crate::crs_info::CrsInfo;
use crate::error::{Result, RasterError};
use crate::raster::{DataType, Raster, RasterConfig};

// ─── Constants ────────────────────────────────────────────────────────────────

const UHL_SIZE: usize = 80;
const DSI_SIZE: usize = 648;
const ACC_SIZE: usize = 2700;
const HEADER_SIZE: usize = UHL_SIZE + DSI_SIZE + ACC_SIZE; // 3428 bytes

/// Elevation value used as nodata in DTED files.
const DTED_VOID: i16 = -32767;
/// Secondary null / fill value found in some DTED files.
const DTED_NULL: i16 = -32768_i16; // = i16::MIN

/// Our internal nodata sentinel (exported to raster consumers).
const NODATA: f64 = -32768.0;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read a DTED elevation file (`.dt0`, `.dt1`, or `.dt2`).
pub fn read(path: &str) -> Result<Raster> {
    let mut file = File::open(path)?;
    let mut raw: Vec<u8> = Vec::new();
    file.read_to_end(&mut raw)?;

    if raw.len() < HEADER_SIZE {
        return Err(RasterError::CorruptData(format!(
            "DTED file too short: {} bytes (minimum {HEADER_SIZE})",
            raw.len()
        )));
    }

    // ── UHL parsing ───────────────────────────────────────────────────────
    let uhl = &raw[..UHL_SIZE];
    if &uhl[0..4] != b"UHL1" {
        return Err(RasterError::CorruptData(
            "DTED: UHL record does not begin with 'UHL1'".into(),
        ));
    }

    // SW-corner longitude (bytes 4-11) and latitude (bytes 12-19)
    let lon_sw = parse_dted_coord(&uhl[4..12])?;
    let lat_sw = parse_dted_coord(&uhl[12..20])?;

    // Arc intervals in tenths of arc-seconds (bytes 20-23 lon, 24-27 lat)
    let lon_interval_tenths = parse_4digit_ascii(&uhl[20..24])?;
    let lat_interval_tenths = parse_4digit_ascii(&uhl[24..28])?;
    let lon_interval = lon_interval_tenths as f64 / 36000.0; // degrees
    let lat_interval = lat_interval_tenths as f64 / 36000.0;

    if lon_interval <= 0.0 || lat_interval <= 0.0 {
        return Err(RasterError::CorruptData(
            "DTED: zero arc interval in UHL".into(),
        ));
    }

    // Number of longitude lines (columns) at bytes 47-50; lat points at 51-54.
    let num_cols = parse_4digit_ascii(&uhl[47..51])? as usize;
    let num_rows = parse_4digit_ascii(&uhl[51..55])? as usize;

    if num_cols == 0 || num_rows == 0 {
        return Err(RasterError::InvalidDimensions {
            cols: num_cols,
            rows: num_rows,
        });
    }

    // ── Data records ──────────────────────────────────────────────────────
    // Each record: 1 sentinel + 3 block-count + 2 lon-count + 2 lat-count
    //              + num_rows×2 elevations + 4 checksum = 12 + num_rows×2 bytes.
    let record_size = 12 + num_rows * 2;
    let expected_data = num_cols * record_size;
    if raw.len() < HEADER_SIZE + expected_data {
        return Err(RasterError::CorruptData(format!(
            "DTED: file too short for declared grid ({num_cols}×{num_rows}): \
             expected {} bytes, got {}",
            HEADER_SIZE + expected_data,
            raw.len()
        )));
    }

    // Grid is stored column-by-column, W→E; within a column S→N.
    // Raster rows are N→S (row 0 = northernmost).
    let mut data = vec![NODATA; num_cols * num_rows];

    for col in 0..num_cols {
        let rec_start = HEADER_SIZE + col * record_size;
        let rec = &raw[rec_start..rec_start + record_size];

        if rec[0] != 0xAA {
            return Err(RasterError::CorruptData(format!(
                "DTED: missing 0xAA sentinel at column {col}"
            )));
        }

        // Read num_rows big-endian i16 elevations starting at byte 8.
        let elev_start = 8;
        for lat_idx in 0..num_rows {
            let off = elev_start + lat_idx * 2;
            let raw_elev = i16::from_be_bytes([rec[off], rec[off + 1]]);
            let z = if raw_elev == DTED_VOID || raw_elev == DTED_NULL {
                NODATA
            } else {
                raw_elev as f64
            };
            // lat_idx 0 = southernmost row → raster row (num_rows - 1 - lat_idx)
            let raster_row = num_rows - 1 - lat_idx;
            data[raster_row * num_cols + col] = z;
        }
    }

    // CRS: DTED is always WGS-84 geographic (EPSG:4326).
    let crs = CrsInfo {
        epsg: Some(4326),
        ..Default::default()
    };

    // Cell size: use the longitude interval (assumes square cells for the cell_size field).
    // Actual lat interval stored in cell_size_y via RasterConfig if they differ.
    let cell_size = lon_interval;
    // SW corner of the grid (corner, not centre; DTED posts are at integer arc-seconds).
    // The first post is at the SW corner coordinates; subtract half a cell.
    let x_min = lon_sw - cell_size * 0.5;
    let y_min = lat_sw - lat_interval * 0.5;

    let cfg = RasterConfig {
        cols: num_cols,
        rows: num_rows,
        x_min,
        y_min,
        cell_size,
        cell_size_y: Some(-lat_interval),
        nodata: NODATA,
        data_type: DataType::I16,
        crs,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

/// Write a raster as a DTED file.
///
/// The raster **must** be in WGS-84 geographic coordinates (degrees).  Values
/// are rounded to the nearest metre and clamped to the valid DTED range
/// `[-32766, 32767]`.  Nodata cells are written as the DTED void value
/// (`-32767`).
///
/// `path` extension (`.dt0`, `.dt1`, `.dt2`) is purely cosmetic; the writer
/// does not enforce grid dimensions based on the DTED level.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "DTED writer supports single-band rasters only".into(),
        ));
    }

    let cols = raster.cols;
    let rows = raster.rows;
    let cs = raster.cell_size_x; // degrees
    let lat_cs = raster.cell_size_y.abs();

    // SW corner post coordinates (centre of SW cell).
    let lon_sw = raster.x_min + cs * 0.5;
    let lat_sw = raster.y_min + lat_cs * 0.5;

    // Arc interval in tenths of arc-seconds.
    let lon_interval_tenths = (cs * 36000.0).round() as u32;
    let lat_interval_tenths = (lat_cs * 36000.0).round() as u32;

    let file = File::create(path)?;
    let mut w = BufWriter::with_capacity((HEADER_SIZE + cols * (12 + rows * 2)).max(65536), file);

    // ── UHL ───────────────────────────────────────────────────────────────
    let mut uhl = [b' '; UHL_SIZE];
    uhl[0..4].copy_from_slice(b"UHL1");
    encode_dted_coord(&mut uhl[4..12], lon_sw, false);
    encode_dted_coord(&mut uhl[12..20], lat_sw, true);
    write_4digit(&mut uhl[20..24], lon_interval_tenths);
    write_4digit(&mut uhl[24..28], lat_interval_tenths);
    uhl[28..32].copy_from_slice(b"    "); // vertical accuracy – unknown
    uhl[32..35].copy_from_slice(b"U  "); // security code
    uhl[35..47].copy_from_slice(b"            "); // unique reference
    write_4digit(&mut uhl[47..51], cols as u32);
    write_4digit(&mut uhl[51..55], rows as u32);
    uhl[55] = b'0'; // multiple accuracy flag
    // bytes 56-79 remain spaces
    w.write_all(&uhl)?;

    // ── DSI (648 bytes, mostly informational) ─────────────────────────────
    let mut dsi = [b' '; DSI_SIZE];
    dsi[0..3].copy_from_slice(b"DSI");
    dsi[3] = b' '; // security code
    // Data edition and maintenance fields left as spaces.
    w.write_all(&dsi)?;

    // ── ACC (2700 bytes, accuracy record) ─────────────────────────────────
    let mut acc = [b' '; ACC_SIZE];
    acc[0..3].copy_from_slice(b"ACC");
    w.write_all(&acc)?;

    // ── Data records ──────────────────────────────────────────────────────
    let mut block_count: u32 = 0;
    for col in 0..cols {
        let mut rec_payload: Vec<u8> = Vec::with_capacity(4 + rows * 2);
        // Longitude count (column index, big-endian u16)
        rec_payload.extend_from_slice(&(col as u16).to_be_bytes());
        // Latitude count (always 0 per spec)
        rec_payload.extend_from_slice(&0u16.to_be_bytes());
        // Elevation data: south → north (raster row num_rows-1 down to 0)
        let mut checksum: u32 = 0;
        for lat_idx in 0..rows {
            let raster_row = (rows - 1 - lat_idx) as isize;
            let z = raster.get(0, raster_row, col as isize);
            let elev: i16 = if z == raster.nodata || z.is_nan() {
                DTED_VOID
            } else {
                z.round().clamp(-32766.0, 32767.0) as i16
            };
            rec_payload.extend_from_slice(&elev.to_be_bytes());
        }
        // Compute checksum over the 4-byte header + elevation bytes.
        for b in &rec_payload {
            checksum = checksum.wrapping_add(*b as u32);
        }

        // Write: sentinel + 3-byte block count + payload + checksum
        w.write_all(&[0xAA])?;
        w.write_all(&block_count.to_be_bytes()[1..4])?; // low 3 bytes
        w.write_all(&rec_payload)?;
        w.write_all(&checksum.to_be_bytes())?;
        block_count += 1;
    }

    Ok(())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Parse a DTED coordinate field of 8 ASCII bytes: `DDDMMSSh`.
fn parse_dted_coord(bytes: &[u8]) -> Result<f64> {
    if bytes.len() < 8 {
        return Err(RasterError::CorruptData(
            "DTED: coordinate field too short".into(),
        ));
    }
    let s = std::str::from_utf8(&bytes[..8]).map_err(|_| {
        RasterError::CorruptData("DTED: coordinate field not valid ASCII".into())
    })?;
    let deg: f64 = s[0..3].trim().parse().map_err(|_| {
        RasterError::ParseError {
            field: "DTED coord degrees".into(),
            value: s[0..3].into(),
            expected: "integer".into(),
        }
    })?;
    let min: f64 = s[3..5].trim().parse().map_err(|_| {
        RasterError::ParseError {
            field: "DTED coord minutes".into(),
            value: s[3..5].into(),
            expected: "integer".into(),
        }
    })?;
    let sec: f64 = s[5..7].trim().parse().map_err(|_| {
        RasterError::ParseError {
            field: "DTED coord seconds".into(),
            value: s[5..7].into(),
            expected: "integer".into(),
        }
    })?;
    let hemi = bytes[7].to_ascii_uppercase();
    let decimal = deg + min / 60.0 + sec / 3600.0;
    let signed = match hemi {
        b'W' | b'S' => -decimal,
        _ => decimal,
    };
    Ok(signed)
}

/// Parse a 4-character zero-padded decimal integer from ASCII bytes.
fn parse_4digit_ascii(bytes: &[u8]) -> Result<u32> {
    let s = std::str::from_utf8(bytes).map_err(|_| {
        RasterError::CorruptData("DTED: non-ASCII field".into())
    })?;
    s.trim().parse::<u32>().map_err(|_| RasterError::ParseError {
        field: "DTED 4-digit field".into(),
        value: s.into(),
        expected: "non-negative integer".into(),
    })
}

/// Encode a decimal degree value as `DDDMMSSh` into an 8-byte ASCII slice.
fn encode_dted_coord(out: &mut [u8], decimal: f64, is_latitude: bool) {
    let abs_val = decimal.abs();
    let deg = abs_val.trunc() as u32;
    let min_f = (abs_val - deg as f64) * 60.0;
    let min = min_f.trunc() as u32;
    let sec = ((min_f - min as f64) * 60.0).round() as u32;
    let hemi = if is_latitude {
        if decimal >= 0.0 { b'N' } else { b'S' }
    } else {
        if decimal >= 0.0 { b'E' } else { b'W' }
    };
    let s = format!("{deg:03}{min:02}{sec:02}");
    out[..7].copy_from_slice(s.as_bytes());
    out[7] = hemi;
}

/// Write a 4-digit zero-padded integer into an ASCII byte slice.
fn write_4digit(out: &mut [u8], v: u32) {
    let s = format!("{:04}", v.min(9999));
    out[..4].copy_from_slice(s.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_dted_coord_positive() {
        // 075°00'00"E
        let bytes = b"0750000E";
        let v = parse_dted_coord(bytes).unwrap();
        assert!((v - 75.0).abs() < 1e-9, "got {v}");
    }

    #[test]
    fn parse_dted_coord_negative_west() {
        // 075°30'30"W → -(75 + 30/60 + 30/3600)
        let bytes = b"0753030W";
        let v = parse_dted_coord(bytes).unwrap();
        let expected = -(75.0 + 30.0 / 60.0 + 30.0 / 3600.0);
        assert!((v - expected).abs() < 1e-9, "got {v}");
    }

    #[test]
    fn encode_decode_coord_roundtrip() {
        for &(decimal, is_lat) in &[
            (43.0_f64, true),
            (-75.0_f64, false),
            (0.0_f64, false),
            (-1.0_f64, true),
        ] {
            let mut buf = [b' '; 8];
            encode_dted_coord(&mut buf, decimal, is_lat);
            let decoded = parse_dted_coord(&buf).unwrap();
            assert!(
                (decoded - decimal).abs() < 0.001,
                "coord roundtrip failed for {decimal}: encoded {:?}, decoded {decoded}",
                std::str::from_utf8(&buf)
            );
        }
    }

    #[test]
    fn roundtrip_write_read() {
        // Create a tiny synthetic 2×3 raster (2 cols, 3 rows) in WGS-84.
        let cfg = RasterConfig {
            cols: 2,
            rows: 3,
            x_min: -75.0 - 0.5 / 36000.0 * 10.0, // SW corner
            y_min: 43.0 - 0.5 / 36000.0 * 10.0,
            cell_size: 10.0 / 36000.0, // 10" in degrees
            nodata: NODATA,
            data_type: DataType::I16,
            crs: CrsInfo { epsg: Some(4326), ..Default::default() },
            ..Default::default()
        };
        let elev = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0];
        let raster = Raster::from_data(cfg, elev.clone()).unwrap();

        let tmp = NamedTempFile::new().unwrap();
        let dt1_path = tmp.path().with_extension("dt1");
        let dt1_str = dt1_path.to_str().unwrap();

        write(&raster, dt1_str).unwrap();
        let loaded = read(dt1_str).unwrap();

        assert_eq!(loaded.cols, 2);
        assert_eq!(loaded.rows, 3);
        assert_eq!(loaded.crs.epsg, Some(4326));

        for row in 0..3isize {
            for col in 0..2isize {
                let orig = raster.get(0, row, col);
                let back = loaded.get(0, row, col);
                assert!(
                    (back - orig).abs() < 1.0,
                    "mismatch at ({row},{col}): {back} vs {orig}"
                );
            }
        }
    }

    #[test]
    fn void_elevation_maps_to_nodata() {
        // Build a minimal DTED buffer with one void elevation.
        // Manually construct a 1×1 DTED (1 col, 1 row) file.
        let mut buf = vec![0u8; HEADER_SIZE + 12 + 2];
        buf[0..4].copy_from_slice(b"UHL1");
        // lon = 075°00'00"E
        buf[4..12].copy_from_slice(b"0750000E");
        // lat = 043°00'00"N
        buf[12..20].copy_from_slice(b"0430000N");
        // intervals: 3000 tenths-of-arcsec = 5' (arbitrary)
        buf[20..24].copy_from_slice(b"3000");
        buf[24..28].copy_from_slice(b"3000");
        // cols = 1, rows = 1
        buf[47..51].copy_from_slice(b"0001");
        buf[51..55].copy_from_slice(b"0001");
        // DSI + ACC stay zero
        let rec = HEADER_SIZE;
        buf[rec] = 0xAA; // sentinel
        // block count (3 bytes)
        buf[rec + 1..rec + 4].copy_from_slice(&[0, 0, 0]);
        // lon count (2 bytes) = 0
        // lat count (2 bytes) = 0
        // elevation = DTED_VOID = 0x8001
        buf[rec + 8] = 0x80;
        buf[rec + 9] = 0x01;
        // checksum (4 bytes) — skip validation for this test

        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().with_extension("dt0");
        std::fs::write(&path, &buf).unwrap();

        let r = read(path.to_str().unwrap()).unwrap();
        assert_eq!(r.rows, 1);
        assert_eq!(r.cols, 1);
        assert_eq!(r.get(0, 0, 0), NODATA);
    }
}
