//! Grid-shift support for datum transformations.
//!
//! This module provides a lightweight in-memory registry for named geodetic
//! shift grids and bilinear interpolation utilities.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::error::{ProjectionError, Result};

/// One shift sample in arc-seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridShiftSample {
    /// Longitude offset in arc-seconds.
    pub dlon_arcsec: f64,
    /// Latitude offset in arc-seconds.
    pub dlat_arcsec: f64,
}

impl GridShiftSample {
    /// Construct a shift sample in arc-seconds.
    pub fn new(dlon_arcsec: f64, dlat_arcsec: f64) -> Self {
        Self {
            dlon_arcsec,
            dlat_arcsec,
        }
    }

    /// Convert this sample to degree offsets.
    pub fn as_degrees(self) -> (f64, f64) {
        (self.dlon_arcsec / 3600.0, self.dlat_arcsec / 3600.0)
    }
}

/// A regular-lattice geodetic grid-shift model.
#[derive(Debug, Clone, PartialEq)]
pub struct GridShiftGrid {
    /// Grid identifier used by datum definitions.
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
    /// Row-major samples of size width * height.
    pub samples: Vec<GridShiftSample>,
}

impl GridShiftGrid {
    /// Create a regular-lattice grid.
    pub fn new(
        name: impl Into<String>,
        lon_min: f64,
        lat_min: f64,
        lon_step: f64,
        lat_step: f64,
        width: usize,
        height: usize,
        samples: Vec<GridShiftSample>,
    ) -> Result<Self> {
        if width < 2 || height < 2 {
            return Err(ProjectionError::DatumError(
                "grid must be at least 2x2 for bilinear interpolation".to_string(),
            ));
        }
        if lon_step <= 0.0 || lat_step <= 0.0 {
            return Err(ProjectionError::DatumError(
                "grid step must be positive".to_string(),
            ));
        }
        if samples.len() != width * height {
            return Err(ProjectionError::DatumError(format!(
                "grid sample count mismatch: expected {}, got {}",
                width * height,
                samples.len()
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
            samples,
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

    /// Bilinearly interpolate a shift sample at lon/lat in degrees.
    pub fn sample(&self, lon_deg: f64, lat_deg: f64) -> Result<GridShiftSample> {
        if lon_deg < self.lon_min
            || lon_deg > self.lon_max()
            || lat_deg < self.lat_min
            || lat_deg > self.lat_max()
        {
            return Err(ProjectionError::DatumError(format!(
                "coordinate ({lon_deg}, {lat_deg}) outside grid '{}' extent",
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

        let s00 = self.samples[self.idx(ix, iy)];
        let s10 = self.samples[self.idx(ix + 1, iy)];
        let s01 = self.samples[self.idx(ix, iy + 1)];
        let s11 = self.samples[self.idx(ix + 1, iy + 1)];

        let dlon0 = s00.dlon_arcsec * (1.0 - tx) + s10.dlon_arcsec * tx;
        let dlon1 = s01.dlon_arcsec * (1.0 - tx) + s11.dlon_arcsec * tx;
        let dlat0 = s00.dlat_arcsec * (1.0 - tx) + s10.dlat_arcsec * tx;
        let dlat1 = s01.dlat_arcsec * (1.0 - tx) + s11.dlat_arcsec * tx;

        Ok(GridShiftSample {
            dlon_arcsec: dlon0 * (1.0 - ty) + dlon1 * ty,
            dlat_arcsec: dlat0 * (1.0 - ty) + dlat1 * ty,
        })
    }

    /// Bilinearly interpolate degree offsets at lon/lat in degrees.
    pub fn sample_shift_degrees(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        Ok(self.sample(lon_deg, lat_deg)?.as_degrees())
    }
}

static GRID_REGISTRY: OnceLock<RwLock<HashMap<String, GridShiftGrid>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, GridShiftGrid>> {
    GRID_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register or replace a named grid-shift model.
pub fn register_grid(grid: GridShiftGrid) -> Result<()> {
    let mut m = registry().write().map_err(|_| {
        ProjectionError::DatumError("grid registry lock poisoned".to_string())
    })?;
    m.insert(grid.name.clone(), grid);
    Ok(())
}

/// Remove a named grid-shift model.
pub fn unregister_grid(name: &str) -> Result<bool> {
    let mut m = registry().write().map_err(|_| {
        ProjectionError::DatumError("grid registry lock poisoned".to_string())
    })?;
    Ok(m.remove(name).is_some())
}

/// Returns true if a named grid is currently registered.
pub fn has_grid(name: &str) -> Result<bool> {
    let m = registry().read().map_err(|_| {
        ProjectionError::DatumError("grid registry lock poisoned".to_string())
    })?;
    Ok(m.contains_key(name))
}

/// Fetch a registered grid by name.
pub fn get_grid(name: &str) -> Result<Option<GridShiftGrid>> {
    let m = registry().read().map_err(|_| {
        ProjectionError::DatumError("grid registry lock poisoned".to_string())
    })?;
    Ok(m.get(name).cloned())
}

#[cfg(test)]
mod tests {
    use super::{GridShiftGrid, GridShiftSample};

    #[test]
    fn bilinear_sample_midpoint() {
        let grid = GridShiftGrid::new(
            "test",
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![
                GridShiftSample::new(0.0, 0.0),
                GridShiftSample::new(2.0, 0.0),
                GridShiftSample::new(0.0, 2.0),
                GridShiftSample::new(2.0, 2.0),
            ],
        )
        .unwrap();

        let s = grid.sample(0.5, 0.5).unwrap();
        assert!((s.dlon_arcsec - 1.0).abs() < 1e-12);
        assert!((s.dlat_arcsec - 1.0).abs() < 1e-12);
    }
}