//! Vertical offset grid support for height-reference conversions.
//!
//! This module provides a lightweight in-memory registry for named vertical
//! offset grids and bilinear interpolation utilities.

use std::collections::HashMap;
use std::io::{BufRead, Read};
use std::sync::{OnceLock, RwLock};

use crate::error::{ProjectionError, Result};

/// A regular-lattice vertical offset grid in meters.
#[derive(Debug, Clone, PartialEq)]
pub struct VerticalOffsetGrid {
    /// Grid identifier used by vertical offset providers.
    pub name: String,
    /// Westernmost longitude (degrees).
    pub lon_min: f64,
    /// Southernmost latitude (degrees).
    pub lat_min: f64,
    /// Longitude spacing (degrees).
    pub lon_step: f64,
    /// Latitude spacing (degrees).
    pub lat_step: f64,
    /// Number of columns.
    pub width: usize,
    /// Number of rows.
    pub height: usize,
    /// Row-major vertical offsets in meters of size width * height.
    pub offsets_m: Vec<f64>,
}

impl VerticalOffsetGrid {
    /// Create a regular-lattice vertical offset grid.
    pub fn new(
        name: impl Into<String>,
        lon_min: f64,
        lat_min: f64,
        lon_step: f64,
        lat_step: f64,
        width: usize,
        height: usize,
        offsets_m: Vec<f64>,
    ) -> Result<Self> {
        if width < 2 || height < 2 {
            return Err(ProjectionError::DatumError(
                "vertical offset grid must be at least 2x2 for bilinear interpolation".to_string(),
            ));
        }
        if lon_step <= 0.0 || lat_step <= 0.0 {
            return Err(ProjectionError::DatumError(
                "vertical offset grid step must be positive".to_string(),
            ));
        }
        if offsets_m.len() != width * height {
            return Err(ProjectionError::DatumError(format!(
                "vertical offset sample count mismatch: expected {}, got {}",
                width * height,
                offsets_m.len()
            )));
        }

        Ok(Self {
            name: name.into(),
            lon_min,
            lat_min,
            lon_step,
            lat_step,
            width,
            height,
            offsets_m,
        })
    }

    fn lon_max(&self) -> f64 {
        self.lon_min + self.lon_step * (self.width as f64 - 1.0)
    }

    fn lat_max(&self) -> f64 {
        self.lat_min + self.lat_step * (self.height as f64 - 1.0)
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Bilinearly interpolate a vertical offset at lon/lat in degrees.
    pub fn sample(&self, lon_deg: f64, lat_deg: f64) -> Result<f64> {
        if lon_deg < self.lon_min
            || lon_deg > self.lon_max()
            || lat_deg < self.lat_min
            || lat_deg > self.lat_max()
        {
            return Err(ProjectionError::DatumError(format!(
                "coordinate ({lon_deg}, {lat_deg}) outside vertical grid '{}' extent",
                self.name
            )));
        }

        let fx = (lon_deg - self.lon_min) / self.lon_step;
        let fy = (lat_deg - self.lat_min) / self.lat_step;

        let mut ix = fx.floor() as usize;
        let mut iy = fy.floor() as usize;

        if ix >= self.width - 1 {
            ix = self.width - 2;
        }
        if iy >= self.height - 1 {
            iy = self.height - 2;
        }

        let tx = fx - ix as f64;
        let ty = fy - iy as f64;

        let z00 = self.offsets_m[self.idx(ix, iy)];
        let z10 = self.offsets_m[self.idx(ix + 1, iy)];
        let z01 = self.offsets_m[self.idx(ix, iy + 1)];
        let z11 = self.offsets_m[self.idx(ix + 1, iy + 1)];

        let z0 = z00 * (1.0 - tx) + z10 * tx;
        let z1 = z01 * (1.0 - tx) + z11 * tx;

        Ok(z0 * (1.0 - ty) + z1 * ty)
    }
}

static VERTICAL_GRID_REGISTRY: OnceLock<RwLock<HashMap<String, VerticalOffsetGrid>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, VerticalOffsetGrid>> {
    VERTICAL_GRID_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register or replace a named vertical offset grid.
pub fn register_vertical_offset_grid(grid: VerticalOffsetGrid) -> Result<()> {
    let mut m = registry()
        .write()
        .map_err(|_| ProjectionError::DatumError("vertical grid registry lock poisoned".to_string()))?;
    m.insert(grid.name.clone(), grid);
    Ok(())
}

/// Remove a named vertical offset grid.
pub fn unregister_vertical_offset_grid(name: &str) -> Result<bool> {
    let mut m = registry()
        .write()
        .map_err(|_| ProjectionError::DatumError("vertical grid registry lock poisoned".to_string()))?;
    Ok(m.remove(name).is_some())
}

/// Returns true if a named vertical offset grid is currently registered.
pub fn has_vertical_offset_grid(name: &str) -> Result<bool> {
    let m = registry()
        .read()
        .map_err(|_| ProjectionError::DatumError("vertical grid registry lock poisoned".to_string()))?;
    Ok(m.contains_key(name))
}

/// Fetch a registered vertical offset grid by name.
pub fn get_vertical_offset_grid(name: &str) -> Result<Option<VerticalOffsetGrid>> {
    let m = registry()
        .read()
        .map_err(|_| ProjectionError::DatumError("vertical grid registry lock poisoned".to_string()))?;
    Ok(m.get(name).cloned())
}

/// Load a [`VerticalOffsetGrid`] from ISG 2.0 format data.
///
/// ISG (International Service for the Geoid) format is widely used for geoid
/// undulation models including EGM2008, EGM96, and regional geoid models.
///
/// The following header fields are required: `lat_min`, `lon_min`,
/// `delta_lat` (or `lat_step`), `delta_lon` (or `lon_step`), `nrows`, `ncols`.
/// If `nrows`/`ncols` are absent, `lat_max`/`lon_max` are used to derive them.
/// The default data ordering is assumed N-to-S, W-to-E unless the
/// `data_ordering` header specifies `S-to-N`.
///
/// # Example
///
/// ```no_run
/// use std::{fs::File, io::BufReader};
/// use wbprojection::{load_vertical_grid_from_isg, register_vertical_offset_grid};
///
/// let f = File::open("egm2008.isg").unwrap();
/// let grid = load_vertical_grid_from_isg(BufReader::new(f), "egm2008").unwrap();
/// register_vertical_offset_grid(grid).unwrap();
/// ```
pub fn load_vertical_grid_from_isg<R: BufRead>(reader: R, name: impl Into<String>) -> Result<VerticalOffsetGrid> {
    let name = name.into();
    let mut lon_min: Option<f64> = None;
    let mut lat_min: Option<f64> = None;
    let mut lat_max: Option<f64> = None;
    let mut lon_max: Option<f64> = None;
    let mut lon_step: Option<f64> = None;
    let mut lat_step: Option<f64> = None;
    let mut nrows: Option<usize> = None;
    let mut ncols: Option<usize> = None;
    let mut nodata: Option<f64> = None;
    let mut north_to_south = true; // ISG default is N-to-S
    let mut in_header = true;
    let mut raw_values: Vec<f64> = Vec::new();

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| ProjectionError::DatumError(format!("ISG read error: {e}")))? ;
        let trimmed = line.trim();

        if in_header {
            if trimmed.to_lowercase().starts_with("end_of_head") {
                in_header = false;
                continue;
            }
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_lowercase();
                let val = trimmed[pos + 1..].trim();
                match key.as_str() {
                    "delta_lat" | "lat_step" => { lat_step = val.parse().ok(); }
                    "delta_lon" | "lon_step" => { lon_step = val.parse().ok(); }
                    "lat_min" => { lat_min = val.parse().ok(); }
                    "lat_max" => { lat_max = val.parse().ok(); }
                    "lon_min" => { lon_min = val.parse().ok(); }
                    "lon_max" => { lon_max = val.parse().ok(); }
                    "nrows" => { nrows = val.parse().ok(); }
                    "ncols" => { ncols = val.parse().ok(); }
                    "nodata" => { nodata = val.parse().ok(); }
                    "data_ordering" => {
                        if val.to_lowercase().contains("s-to-n") {
                            north_to_south = false;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            for token in trimmed.split_ascii_whitespace() {
                if let Ok(v) = token.parse::<f64>() {
                    raw_values.push(v);
                }
            }
        }
    }

    let lon_min = lon_min.ok_or_else(|| ProjectionError::DatumError("ISG: missing lon_min".into()))?;
    let lat_min = lat_min.ok_or_else(|| ProjectionError::DatumError("ISG: missing lat_min".into()))?;
    let lon_step = lon_step.ok_or_else(|| ProjectionError::DatumError("ISG: missing delta_lon".into()))?;
    let lat_step = lat_step.ok_or_else(|| ProjectionError::DatumError("ISG: missing delta_lat".into()))?;

    // Prefer explicit nrows/ncols; fall back to computing from extent + step.
    let width = ncols.or_else(|| {
        lon_max.map(|lmax| ((lmax - lon_min) / lon_step).round() as usize + 1)
    }).ok_or_else(|| ProjectionError::DatumError("ISG: missing ncols or lon_max".into()))?;
    let height = nrows.or_else(|| {
        lat_max.map(|lmax| ((lmax - lat_min) / lat_step).round() as usize + 1)
    }).ok_or_else(|| ProjectionError::DatumError("ISG: missing nrows or lat_max".into()))?;

    if raw_values.len() != width * height {
        return Err(ProjectionError::DatumError(format!(
            "ISG: expected {} values ({height} rows × {width} cols), parsed {}",
            width * height,
            raw_values.len()
        )));
    }

    let nodata_replace = |v: f64| -> f64 {
        match nodata {
            Some(nd) if (v - nd).abs() < 0.5 => 0.0,
            _ => v,
        }
    };

    let mut offsets_m: Vec<f64> = Vec::with_capacity(width * height);
    if north_to_south {
        // ISG row 0 = lat_max; flip to S-to-N (row 0 = lat_min)
        for row in (0..height).rev() {
            for col in 0..width {
                offsets_m.push(nodata_replace(raw_values[row * width + col]));
            }
        }
    } else {
        for &v in &raw_values {
            offsets_m.push(nodata_replace(v));
        }
    }

    VerticalOffsetGrid::new(name, lon_min, lat_min, lon_step, lat_step, width, height, offsets_m)
}

/// Load a [`VerticalOffsetGrid`] from a simple header + data text format.
///
/// The format is a sequence of `key = value` header lines (one per line),
/// optionally interspersed with `#` comment lines, followed by space-separated
/// floating-point offset values.  The transition from header to data occurs at
/// the first non-comment, non-`key=value` line.
///
/// Required header keys: `lon_min`, `lat_min`, `lon_step`, `lat_step`,
/// `width`, `height`.  Data rows are ordered S-to-N (first row = `lat_min`),
/// W-to-E (first column = `lon_min`).
///
/// # Example format
///
/// ```text
/// # my geoid grid
/// lon_min = 140.0
/// lat_min = -40.0
/// lon_step = 0.5
/// lat_step = 0.5
/// width = 3
/// height = 3
/// 28.0 28.1 28.2
/// 28.3 28.4 28.5
/// 28.6 28.7 28.8
/// ```
///
/// # Example
///
/// ```no_run
/// use std::{fs::File, io::BufReader};
/// use wbprojection::{load_vertical_grid_from_simple_header_grid, register_vertical_offset_grid};
///
/// let f = File::open("my_geoid.grid").unwrap();
/// let grid = load_vertical_grid_from_simple_header_grid(BufReader::new(f), "my_geoid").unwrap();
/// register_vertical_offset_grid(grid).unwrap();
/// ```
pub fn load_vertical_grid_from_simple_header_grid<R: BufRead>(reader: R, name: impl Into<String>) -> Result<VerticalOffsetGrid> {
    let name = name.into();
    let mut lon_min: Option<f64> = None;
    let mut lat_min: Option<f64> = None;
    let mut lon_step: Option<f64> = None;
    let mut lat_step: Option<f64> = None;
    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut offsets_m: Vec<f64> = Vec::new();
    let mut in_header = true;

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| ProjectionError::DatumError(format!("grid read error: {e}")))? ;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if in_header {
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_lowercase();
                let val = trimmed[pos + 1..].trim();
                match key.as_str() {
                    "lon_min" => { lon_min = val.parse().ok(); }
                    "lat_min" => { lat_min = val.parse().ok(); }
                    "lon_step" => { lon_step = val.parse().ok(); }
                    "lat_step" => { lat_step = val.parse().ok(); }
                    "width"   => { width  = val.parse().ok(); }
                    "height"  => { height = val.parse().ok(); }
                    _ => {}
                }
                continue;
            }
            // First non-comment, non-key=value line marks start of data.
            in_header = false;
        }

        for token in trimmed.split_ascii_whitespace() {
            if let Ok(v) = token.parse::<f64>() {
                offsets_m.push(v);
            }
        }
    }

    let lon_min = lon_min.ok_or_else(|| ProjectionError::DatumError("simple grid: missing lon_min".into()))?;
    let lat_min = lat_min.ok_or_else(|| ProjectionError::DatumError("simple grid: missing lat_min".into()))?;
    let lon_step = lon_step.ok_or_else(|| ProjectionError::DatumError("simple grid: missing lon_step".into()))?;
    let lat_step = lat_step.ok_or_else(|| ProjectionError::DatumError("simple grid: missing lat_step".into()))?;
    let width  = width .ok_or_else(|| ProjectionError::DatumError("simple grid: missing width".into()))?;
    let height = height.ok_or_else(|| ProjectionError::DatumError("simple grid: missing height".into()))?;

    VerticalOffsetGrid::new(name, lon_min, lat_min, lon_step, lat_step, width, height, offsets_m)
}

#[derive(Debug, Clone, Copy)]
enum GtxEndian {
    Little,
    Big,
}

fn read_f64_at(buf: &[u8], off: usize, endian: GtxEndian) -> f64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&buf[off..off + 8]);
    match endian {
        GtxEndian::Little => f64::from_le_bytes(a),
        GtxEndian::Big => f64::from_be_bytes(a),
    }
}

fn read_i32_at(buf: &[u8], off: usize, endian: GtxEndian) -> i32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&buf[off..off + 4]);
    match endian {
        GtxEndian::Little => i32::from_le_bytes(a),
        GtxEndian::Big => i32::from_be_bytes(a),
    }
}

fn read_f32_at(buf: &[u8], off: usize, endian: GtxEndian) -> f32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&buf[off..off + 4]);
    match endian {
        GtxEndian::Little => f32::from_le_bytes(a),
        GtxEndian::Big => f32::from_be_bytes(a),
    }
}

fn parse_gtx(buf: &[u8], endian: GtxEndian, name: &str) -> Result<VerticalOffsetGrid> {
    if buf.len() < 40 {
        return Err(ProjectionError::DatumError(
            "GTX file too short to contain header".to_string(),
        ));
    }

    let lat0 = read_f64_at(buf, 0, endian);
    let lon0 = read_f64_at(buf, 8, endian);
    let dlat = read_f64_at(buf, 16, endian);
    let dlon = read_f64_at(buf, 24, endian);
    let rows_i32 = read_i32_at(buf, 32, endian);
    let cols_i32 = read_i32_at(buf, 36, endian);

    if rows_i32 < 2 || cols_i32 < 2 {
        return Err(ProjectionError::DatumError(
            "GTX rows/cols must be >= 2".to_string(),
        ));
    }
    if !dlat.is_finite() || !dlon.is_finite() || dlat == 0.0 || dlon == 0.0 {
        return Err(ProjectionError::DatumError(
            "GTX invalid cell spacing values".to_string(),
        ));
    }

    let height = rows_i32 as usize;
    let width = cols_i32 as usize;
    let expected = 40usize
        .checked_add(
            width
                .checked_mul(height)
                .and_then(|n| n.checked_mul(4))
                .ok_or_else(|| ProjectionError::DatumError("GTX dimensions overflow".to_string()))?,
        )
        .ok_or_else(|| ProjectionError::DatumError("GTX dimensions overflow".to_string()))?;

    if buf.len() != expected {
        return Err(ProjectionError::DatumError(format!(
            "GTX size mismatch: expected {expected} bytes, got {}",
            buf.len()
        )));
    }

    let lon_step = dlon.abs();
    let lat_step = dlat.abs();

    let lon_min = if dlon > 0.0 {
        lon0
    } else {
        lon0 + dlon * (width as f64 - 1.0)
    };
    let lat_min = if dlat > 0.0 {
        lat0
    } else {
        lat0 + dlat * (height as f64 - 1.0)
    };

    let mut offsets_m = vec![0.0; width * height];

    for src_row in 0..height {
        for src_col in 0..width {
            let src_idx = src_row * width + src_col;
            let off = 40 + src_idx * 4;
            let v = read_f32_at(buf, off, endian) as f64;

            let dst_row = if dlat > 0.0 { src_row } else { height - 1 - src_row };
            let dst_col = if dlon > 0.0 { src_col } else { width - 1 - src_col };
            offsets_m[dst_row * width + dst_col] = v;
        }
    }

    VerticalOffsetGrid::new(
        name.to_string(),
        lon_min,
        lat_min,
        lon_step,
        lat_step,
        width,
        height,
        offsets_m,
    )
}

/// Load a [`VerticalOffsetGrid`] from a binary PROJ/NOAA GTX geoid file.
///
/// The loader auto-detects little-endian or big-endian encoding, normalizes
/// to S-to-N / W-to-E internal order, and preserves raw offset values.
pub fn load_vertical_grid_from_gtx<R: Read>(mut reader: R, name: impl Into<String>) -> Result<VerticalOffsetGrid> {
    let name = name.into();
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .map_err(|e| ProjectionError::DatumError(format!("GTX read error: {e}")))?;

    let le = parse_gtx(&buf, GtxEndian::Little, &name);
    if le.is_ok() {
        return le;
    }

    let be = parse_gtx(&buf, GtxEndian::Big, &name);
    if be.is_ok() {
        return be;
    }

    Err(ProjectionError::DatumError(format!(
        "failed to parse GTX grid '{}' as little-endian or big-endian",
        name
    )))
}

#[cfg(test)]
mod tests {
    use super::VerticalOffsetGrid;

    fn make_gtx_bytes_le(
        lat0: f64,
        lon0: f64,
        dlat: f64,
        dlon: f64,
        rows: i32,
        cols: i32,
        data: &[f32],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&lat0.to_le_bytes());
        out.extend_from_slice(&lon0.to_le_bytes());
        out.extend_from_slice(&dlat.to_le_bytes());
        out.extend_from_slice(&dlon.to_le_bytes());
        out.extend_from_slice(&rows.to_le_bytes());
        out.extend_from_slice(&cols.to_le_bytes());
        for &v in data {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    fn make_gtx_bytes_be(
        lat0: f64,
        lon0: f64,
        dlat: f64,
        dlon: f64,
        rows: i32,
        cols: i32,
        data: &[f32],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&lat0.to_be_bytes());
        out.extend_from_slice(&lon0.to_be_bytes());
        out.extend_from_slice(&dlat.to_be_bytes());
        out.extend_from_slice(&dlon.to_be_bytes());
        out.extend_from_slice(&rows.to_be_bytes());
        out.extend_from_slice(&cols.to_be_bytes());
        for &v in data {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out
    }

    #[test]
    fn bilinear_vertical_offset_sample_midpoint() {
        let grid = VerticalOffsetGrid::new(
            "test_vertical",
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![0.0, 2.0, 2.0, 4.0],
        )
        .unwrap();

        let z = grid.sample(0.5, 0.5).unwrap();
        assert!((z - 2.0).abs() < 1e-12);
    }

    #[test]
    fn load_isg_north_to_south_flips_rows() {
        // 2×2 grid, N-to-S ordering.
        // Stored rows: [3,4] at lat=1, then [1,2] at lat=0.
        // After flip (to S-to-N): row0=[1,2] at lat=0, row1=[3,4] at lat=1.
        let src = [
            "begin_of_head ========================",
            "lat_min = 0.0",
            "lon_min = 0.0",
            "delta_lat = 1.0",
            "delta_lon = 1.0",
            "nrows = 2",
            "ncols = 2",
            "data_ordering = N-to-S, W-to-E",
            "end_of_head ========================",
            "3.0 4.0",
            "1.0 2.0",
        ]
        .join("\n");
        let grid = super::load_vertical_grid_from_isg(src.as_bytes(), "isg_test").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 4.0).abs() < 1e-9);
        assert!((grid.sample(0.5, 0.5).unwrap() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn load_isg_south_to_north_preserves_order() {
        let src = [
            "begin_of_head ========================",
            "lat_min = 0.0",
            "lon_min = 0.0",
            "delta_lat = 1.0",
            "delta_lon = 1.0",
            "nrows = 2",
            "ncols = 2",
            "data_ordering = S-to-N, W-to-E",
            "end_of_head ========================",
            "1.0 2.0",
            "3.0 4.0",
        ]
        .join("\n");
        let grid = super::load_vertical_grid_from_isg(src.as_bytes(), "isg_sn").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn load_isg_derives_dims_from_extent() {
        // nrows/ncols absent; derived from lat_max/lon_max.
        let src = [
            "begin_of_head ========================",
            "lat_min = 0.0",
            "lat_max = 1.0",
            "lon_min = 0.0",
            "lon_max = 1.0",
            "delta_lat = 1.0",
            "delta_lon = 1.0",
            "data_ordering = N-to-S, W-to-E",
            "end_of_head ========================",
            "3.0 4.0",
            "1.0 2.0",
        ]
        .join("\n");
        let grid = super::load_vertical_grid_from_isg(src.as_bytes(), "isg_ext").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn load_isg_replaces_nodata_with_zero() {
        let src = [
            "begin_of_head ========================",
            "lat_min = 0.0",
            "lon_min = 0.0",
            "delta_lat = 1.0",
            "delta_lon = 1.0",
            "nrows = 2",
            "ncols = 2",
            "nodata = 9999.0",
            "data_ordering = S-to-N, W-to-E",
            "end_of_head ========================",
            "1.0 9999.0",
            "3.0 4.0",
        ]
        .join("\n");
        let grid = super::load_vertical_grid_from_isg(src.as_bytes(), "isg_nd").unwrap();
        assert!((grid.sample(1.0, 0.0).unwrap() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn load_simple_header_grid_parses_values() {
        let src = [
            "lon_min = 0.0",
            "lat_min = 0.0",
            "lon_step = 1.0",
            "lat_step = 1.0",
            "width = 2",
            "height = 2",
            "1.0 2.0",
            "3.0 4.0",
        ]
        .join("\n");
        let grid =
            super::load_vertical_grid_from_simple_header_grid(src.as_bytes(), "simple").unwrap();
        // row 0 (lat=0): [1, 2]; row 1 (lat=1): [3, 4]
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 4.0).abs() < 1e-9);
        assert!((grid.sample(0.5, 0.5).unwrap() - 2.5).abs() < 1e-9);
    }

    #[test]
    fn load_simple_header_grid_handles_comments() {
        let src = [
            "# my test grid",
            "lon_min = 0.0",
            "lat_min = 0.0",
            "# another comment mid-header",
            "lon_step = 1.0",
            "lat_step = 1.0",
            "width = 2",
            "height = 2",
            "# data below",
            "0.0 1.0",
            "2.0 3.0",
        ]
        .join("\n");
        let grid =
            super::load_vertical_grid_from_simple_header_grid(src.as_bytes(), "simple_comments").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 0.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn load_gtx_little_endian_with_negative_steps_reorders_axes() {
        // Header starts at NE corner with negative steps.
        let bytes = make_gtx_bytes_le(
            1.0,
            1.0,
            -1.0,
            -1.0,
            2,
            2,
            &[4.0, 3.0, 2.0, 1.0],
        );

        let grid = super::load_vertical_grid_from_gtx(bytes.as_slice(), "gtx_le").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn load_gtx_big_endian_parses_ok() {
        let bytes = make_gtx_bytes_be(0.0, 0.0, 1.0, 1.0, 2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let grid = super::load_vertical_grid_from_gtx(bytes.as_slice(), "gtx_be").unwrap();
        assert!((grid.sample(0.0, 0.0).unwrap() - 1.0).abs() < 1e-9);
        assert!((grid.sample(1.0, 1.0).unwrap() - 4.0).abs() < 1e-9);
    }
}
