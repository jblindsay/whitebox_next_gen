//! XYZ ASCII raster format (`.xyz`).
//!
//! An XYZ file contains whitespace- or comma-delimited rows with three columns:
//!   X (easting/longitude)  Y (northing/latitude)  Z (value/elevation)
//!
//! The points are assumed to represent a regular grid. The reader infers grid
//! extent and cell size by collecting unique X and Y coordinates, checking for
//! uniform spacing, and filling the output raster accordingly.
//!
//! Write produces the same three-column text format, one point per line,
//! using the cell-centre coordinates, row-major from north to south.
//!
//! # Notes
//! - Lines beginning with `#` are treated as comments and skipped.
//! - An optional header row (`x,y,z` or `X Y Z` etc.) is detected and skipped.
//! - Delimiter is auto-detected (comma, tab, or whitespace).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::{Result, RasterError};
use crate::io_utils::format_float;
use crate::raster::{DataType, Raster, RasterConfig};

// Tolerance used when testing whether inferred cell-sizes are uniform.
const REL_TOL: f64 = 1e-6;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Read an XYZ ASCII raster from `path`.
pub fn read(path: &str) -> Result<Raster> {
    let file = File::open(path)?;
    let reader = BufReader::with_capacity(256 * 1024, file);
    parse(reader)
}

/// Write `raster` as an XYZ ASCII file to `path`.
///
/// Each cell is emitted as its centre coordinate. Nodata cells are written
/// with the raster's nodata value so round-trips preserve spatial coverage.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    if raster.bands != 1 {
        return Err(RasterError::UnsupportedDataType(
            "XYZ writer supports single-band rasters only".into(),
        ));
    }
    let file = File::create(path)?;
    let mut w = BufWriter::with_capacity(256 * 1024, file);
    let cs = raster.cell_size_x;
    let half = cs * 0.5;
    for row in 0..raster.rows as isize {
        // y increases northward; row 0 is the northernmost row.
        let y = raster.y_min + (raster.rows as f64 - 0.5 - row as f64) * cs;
        for col in 0..raster.cols as isize {
            let x = raster.x_min + (col as f64 + 0.5) * cs;
            let _ = half; // used above in concept; x/y already cell-centre
            let z = raster.get(0, row, col);
            writeln!(w, "{} {} {}", format_float(x, 10), format_float(y, 10), format_float(z, 6))?;
        }
    }
    Ok(())
}

// ─── Internal ─────────────────────────────────────────────────────────────────

fn parse<R: BufRead>(reader: R) -> Result<Raster> {
    let mut points: Vec<(f64, f64, f64)> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();
        // Skip blank lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Detect and skip a text header row (e.g. "x y z" or "X,Y,Z").
        let first_char = trimmed.chars().next().unwrap_or(' ');
        if first_char.is_ascii_alphabetic() {
            continue;
        }
        // Detect delimiter: comma, tab, or whitespace.
        let (x, y, z) = parse_xyz_line(trimmed)?;
        points.push((x, y, z));
    }

    if points.is_empty() {
        return Err(RasterError::CorruptData("XYZ file contains no data points".into()));
    }

    build_raster(points)
}

/// Parse one data line into `(x, y, z)`.
fn parse_xyz_line(line: &str) -> Result<(f64, f64, f64)> {
    // Try comma first, then tab, then whitespace.
    let tokens: Vec<&str> = if line.contains(',') {
        line.split(',').collect()
    } else if line.contains('\t') {
        line.split('\t').collect()
    } else {
        line.split_ascii_whitespace().collect()
    };

    if tokens.len() < 3 {
        return Err(RasterError::CorruptData(format!(
            "XYZ: expected at least 3 columns per line, got {}: '{}'",
            tokens.len(), line
        )));
    }

    let x = tokens[0].trim().parse::<f64>().map_err(|_| RasterError::ParseError {
        field: "X".into(),
        value: tokens[0].into(),
        expected: "number".into(),
    })?;
    let y = tokens[1].trim().parse::<f64>().map_err(|_| RasterError::ParseError {
        field: "Y".into(),
        value: tokens[1].into(),
        expected: "number".into(),
    })?;
    let z = tokens[2].trim().parse::<f64>().map_err(|_| RasterError::ParseError {
        field: "Z".into(),
        value: tokens[2].into(),
        expected: "number".into(),
    })?;
    Ok((x, y, z))
}

/// Reconstruct a regular grid from an unordered set of (x, y, z) points.
fn build_raster(mut points: Vec<(f64, f64, f64)>) -> Result<Raster> {
    // Collect unique X and Y coordinates using an ordered map keyed by
    // integer-rounded values so near-duplicates from float representation are merged.
    // We scale by a large factor before rounding to preserve sub-metre precision.
    const SCALE: f64 = 1e9;

    // Sort once so we can deduplicate with a simple linear scan.
    points.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap()
            .then(a.0.partial_cmp(&b.0).unwrap())
    });

    // Unique X values (sorted ascending).
    let mut x_map: BTreeMap<i64, f64> = BTreeMap::new();
    // Unique Y values (sorted ascending — we'll reverse for raster row ordering).
    let mut y_map: BTreeMap<i64, f64> = BTreeMap::new();

    for &(x, y, _) in &points {
        let xi = (x * SCALE).round() as i64;
        let yi = (y * SCALE).round() as i64;
        x_map.entry(xi).or_insert(x);
        y_map.entry(yi).or_insert(y);
    }

    let xs: Vec<f64> = x_map.values().copied().collect(); // ascending
    let ys_asc: Vec<f64> = y_map.values().copied().collect(); // ascending

    let cols = xs.len();
    let rows = ys_asc.len();

    if cols < 2 || rows < 2 {
        return Err(RasterError::CorruptData(format!(
            "XYZ: cannot construct a grid with fewer than 2 unique values in each axis \
             (found {cols} X values and {rows} Y values)"
        )));
    }

    // Infer cell size from X spacing (take median-ish: first gap then verify).
    let cs_x = xs[1] - xs[0];
    let cs_y = ys_asc[1] - ys_asc[0];
    if cs_x <= 0.0 || cs_y <= 0.0 {
        return Err(RasterError::CorruptData(
            "XYZ: non-positive inferred cell size".into(),
        ));
    }
    // Verify uniform X spacing.
    for w in xs.windows(2) {
        let gap = w[1] - w[0];
        if ((gap - cs_x) / cs_x).abs() > REL_TOL {
            return Err(RasterError::CorruptData(format!(
                "XYZ: irregular X spacing detected ({gap:.6} vs expected {cs_x:.6})"
            )));
        }
    }
    // Verify uniform Y spacing.
    for w in ys_asc.windows(2) {
        let gap = w[1] - w[0];
        if ((gap - cs_y) / cs_y).abs() > REL_TOL {
            return Err(RasterError::CorruptData(format!(
                "XYZ: irregular Y spacing detected ({gap:.6} vs expected {cs_y:.6})"
            )));
        }
    }
    // Require square pixels (standard for most raster tools).
    if ((cs_x - cs_y) / cs_x).abs() > REL_TOL * 10.0 {
        return Err(RasterError::CorruptData(format!(
            "XYZ: non-square cell spacing (dx={cs_x:.6}, dy={cs_y:.6})"
        )));
    }

    // SW corner (corner, not centre).
    let x_min = xs[0] - cs_x * 0.5;
    let y_min = ys_asc[0] - cs_y * 0.5;

    // Build index maps: coordinate key → column/row index.
    let x_idx: std::collections::HashMap<i64, usize> = x_map
        .keys()
        .enumerate()
        .map(|(i, k)| (*k, i))
        .collect();
    // Rows in raster are north-to-south: row 0 = largest Y.
    let y_idx: std::collections::HashMap<i64, usize> = y_map
        .keys()
        .rev()
        .enumerate()
        .map(|(i, k)| (*k, i))
        .collect();

    let nodata = -9999.0_f64;
    let mut data = vec![nodata; cols * rows];

    for (x, y, z) in points {
        let xi = (x * SCALE).round() as i64;
        let yi = (y * SCALE).round() as i64;
        if let (Some(&col), Some(&row)) = (x_idx.get(&xi), y_idx.get(&yi)) {
            data[row * cols + col] = z;
        }
    }

    let cfg = RasterConfig {
        cols,
        rows,
        x_min,
        y_min,
        cell_size: cs_x,
        nodata,
        data_type: DataType::F64,
        ..Default::default()
    };
    Raster::from_data(cfg, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // A small 3×3 grid, centre coordinates.
    // x: 5, 15, 25  y: 25, 15, 5  cellsize: 10  corner: (0,0)
    const SAMPLE_XYZ: &str = "\
5 25 1
15 25 2
25 25 3
5 15 4
15 15 5
25 15 6
5 5 7
15 5 8
25 5 9
";

    #[test]
    fn parse_grid_from_xyz() {
        let r = parse(BufReader::new(Cursor::new(SAMPLE_XYZ))).unwrap();
        assert_eq!(r.cols, 3);
        assert_eq!(r.rows, 3);
        assert!((r.cell_size_x - 10.0).abs() < 1e-9);
        assert!((r.x_min - 0.0).abs() < 1e-9);
        assert!((r.y_min - 0.0).abs() < 1e-9);
        // Row 0 is northernmost (y=25): values 1, 2, 3
        assert_eq!(r.get(0, 0, 0), 1.0);
        assert_eq!(r.get(0, 0, 1), 2.0);
        assert_eq!(r.get(0, 0, 2), 3.0);
        // Row 2 is southernmost (y=5): values 7, 8, 9
        assert_eq!(r.get(0, 2, 0), 7.0);
        assert_eq!(r.get(0, 2, 2), 9.0);
    }

    #[test]
    fn parse_skips_comments_and_header() {
        let src = "# this is a comment\nX Y Z\n5 5 99\n15 5 88\n5 15 77\n15 15 66\n";
        let r = parse(BufReader::new(Cursor::new(src))).unwrap();
        assert_eq!(r.cols, 2);
        assert_eq!(r.rows, 2);
        assert_eq!(r.get(0, 0, 0), 77.0); // y=15 → row 0
        assert_eq!(r.get(0, 1, 1), 88.0); // y=5  → row 1, x=15 → col 1
    }

    #[test]
    fn parse_comma_delimited() {
        let src = "5,5,10\n15,5,20\n5,15,30\n15,15,40\n";
        let r = parse(BufReader::new(Cursor::new(src))).unwrap();
        assert_eq!(r.cols, 2);
        assert_eq!(r.rows, 2);
        assert!((r.get(0, 0, 0) - 30.0).abs() < 1e-9); // y=15 → row 0, x=5 → col 0
    }

    #[test]
    fn roundtrip_xyz() {
        let r_in = parse(BufReader::new(Cursor::new(SAMPLE_XYZ))).unwrap();
        let mut buf = Vec::new();
        {
            let mut w = BufWriter::new(&mut buf);
            let cs = r_in.cell_size_x;
            for row in 0..r_in.rows as isize {
                let y = r_in.y_min + (r_in.rows as f64 - 0.5 - row as f64) * cs;
                for col in 0..r_in.cols as isize {
                    let x = r_in.x_min + (col as f64 + 0.5) * cs;
                    let z = r_in.get(0, row, col);
                    writeln!(w, "{x} {y} {z}").unwrap();
                }
            }
        }
        let text = String::from_utf8(buf).unwrap();
        let r_out = parse(BufReader::new(Cursor::new(text.as_str()))).unwrap();
        assert_eq!(r_out.cols, r_in.cols);
        assert_eq!(r_out.rows, r_in.rows);
        for row in 0..r_in.rows as isize {
            for col in 0..r_in.cols as isize {
                assert!(
                    (r_out.get(0, row, col) - r_in.get(0, row, col)).abs() < 1e-6,
                    "mismatch at ({row},{col})"
                );
            }
        }
    }
}
