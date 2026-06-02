//! Grid-shift support for datum transformations.
//!
//! This module provides a lightweight in-memory registry for named geodetic
//! shift grids and bilinear interpolation utilities.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::error::{ProjectionError, Result};

/// One dynamic shift sample in arc-seconds.
///
/// `*_0_arcsec` are reference-epoch offsets and `*_rate_arcsec_per_year` are
/// linear rates per decimal year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicGridShiftSample {
    /// Reference-epoch longitude offset in arc-seconds.
    pub dlon0_arcsec: f64,
    /// Reference-epoch latitude offset in arc-seconds.
    pub dlat0_arcsec: f64,
    /// Longitude offset rate in arc-seconds per year.
    pub dlon_rate_arcsec_per_year: f64,
    /// Latitude offset rate in arc-seconds per year.
    pub dlat_rate_arcsec_per_year: f64,
}

impl DynamicGridShiftSample {
    /// Construct a dynamic shift sample in arc-seconds and arc-seconds/year.
    pub fn new(
        dlon0_arcsec: f64,
        dlat0_arcsec: f64,
        dlon_rate_arcsec_per_year: f64,
        dlat_rate_arcsec_per_year: f64,
    ) -> Self {
        Self {
            dlon0_arcsec,
            dlat0_arcsec,
            dlon_rate_arcsec_per_year,
            dlat_rate_arcsec_per_year,
        }
    }

    /// Evaluate this dynamic sample at `epoch_decimal_year`, returning arc-seconds.
    pub fn as_arcseconds_at_epoch(
        self,
        reference_epoch_decimal_year: f64,
        epoch_decimal_year: f64,
    ) -> (f64, f64) {
        let dt_years = epoch_decimal_year - reference_epoch_decimal_year;
        (
            self.dlon0_arcsec + self.dlon_rate_arcsec_per_year * dt_years,
            self.dlat0_arcsec + self.dlat_rate_arcsec_per_year * dt_years,
        )
    }

    /// Evaluate this dynamic sample at `epoch_decimal_year`, returning degrees.
    pub fn as_degrees_at_epoch(
        self,
        reference_epoch_decimal_year: f64,
        epoch_decimal_year: f64,
    ) -> (f64, f64) {
        let (dlon_arcsec, dlat_arcsec) =
            self.as_arcseconds_at_epoch(reference_epoch_decimal_year, epoch_decimal_year);
        (dlon_arcsec / 3600.0, dlat_arcsec / 3600.0)
    }
}

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

/// A regular-lattice geodetic dynamic grid-shift model.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicGridShiftGrid {
    /// Grid identifier used by datum definitions.
    pub name: String,
    /// Reference epoch for base shift values (decimal year).
    pub reference_epoch_decimal_year: f64,
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
    pub samples: Vec<DynamicGridShiftSample>,
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

impl DynamicGridShiftGrid {
    /// Create a regular-lattice dynamic grid.
    pub fn new(
        name: impl Into<String>,
        reference_epoch_decimal_year: f64,
        lon_min: f64,
        lat_min: f64,
        lon_step: f64,
        lat_step: f64,
        width: usize,
        height: usize,
        samples: Vec<DynamicGridShiftSample>,
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
        if !reference_epoch_decimal_year.is_finite() {
            return Err(ProjectionError::DatumError(
                "reference epoch must be finite".to_string(),
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
            reference_epoch_decimal_year,
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

    /// Bilinearly interpolate dynamic offsets (arc-seconds) at lon/lat and epoch.
    pub fn sample_shift_arcseconds_at_epoch(
        &self,
        lon_deg: f64,
        lat_deg: f64,
        epoch_decimal_year: f64,
    ) -> Result<(f64, f64)> {
        if !epoch_decimal_year.is_finite() {
            return Err(ProjectionError::DatumError(
                "epoch must be finite".to_string(),
            ));
        }

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

        let s00 = self.samples[self.idx(ix, iy)]
            .as_arcseconds_at_epoch(self.reference_epoch_decimal_year, epoch_decimal_year);
        let s10 = self.samples[self.idx(ix + 1, iy)]
            .as_arcseconds_at_epoch(self.reference_epoch_decimal_year, epoch_decimal_year);
        let s01 = self.samples[self.idx(ix, iy + 1)]
            .as_arcseconds_at_epoch(self.reference_epoch_decimal_year, epoch_decimal_year);
        let s11 = self.samples[self.idx(ix + 1, iy + 1)]
            .as_arcseconds_at_epoch(self.reference_epoch_decimal_year, epoch_decimal_year);

        let dlon0 = s00.0 * (1.0 - tx) + s10.0 * tx;
        let dlon1 = s01.0 * (1.0 - tx) + s11.0 * tx;
        let dlat0 = s00.1 * (1.0 - tx) + s10.1 * tx;
        let dlat1 = s01.1 * (1.0 - tx) + s11.1 * tx;

        Ok((
            dlon0 * (1.0 - ty) + dlon1 * ty,
            dlat0 * (1.0 - ty) + dlat1 * ty,
        ))
    }

    /// Bilinearly interpolate dynamic offsets (degrees) at lon/lat and epoch.
    pub fn sample_shift_degrees_at_epoch(
        &self,
        lon_deg: f64,
        lat_deg: f64,
        epoch_decimal_year: f64,
    ) -> Result<(f64, f64)> {
        let (dlon_arcsec, dlat_arcsec) =
            self.sample_shift_arcseconds_at_epoch(lon_deg, lat_deg, epoch_decimal_year)?;
        Ok((dlon_arcsec / 3600.0, dlat_arcsec / 3600.0))
    }
}

static GRID_REGISTRY: OnceLock<RwLock<HashMap<String, GridShiftGrid>>> = OnceLock::new();
static DYNAMIC_GRID_REGISTRY: OnceLock<RwLock<HashMap<String, DynamicGridShiftGrid>>> =
    OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, GridShiftGrid>> {
    GRID_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn dynamic_registry() -> &'static RwLock<HashMap<String, DynamicGridShiftGrid>> {
    DYNAMIC_GRID_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
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

/// Register or replace a named dynamic grid-shift model.
pub fn register_dynamic_grid(grid: DynamicGridShiftGrid) -> Result<()> {
    let mut m = dynamic_registry().write().map_err(|_| {
        ProjectionError::DatumError("dynamic grid registry lock poisoned".to_string())
    })?;
    m.insert(grid.name.clone(), grid);
    Ok(())
}

/// Remove a named dynamic grid-shift model.
pub fn unregister_dynamic_grid(name: &str) -> Result<bool> {
    let mut m = dynamic_registry().write().map_err(|_| {
        ProjectionError::DatumError("dynamic grid registry lock poisoned".to_string())
    })?;
    Ok(m.remove(name).is_some())
}

/// Returns true if a named dynamic grid is currently registered.
pub fn has_dynamic_grid(name: &str) -> Result<bool> {
    let m = dynamic_registry().read().map_err(|_| {
        ProjectionError::DatumError("dynamic grid registry lock poisoned".to_string())
    })?;
    Ok(m.contains_key(name))
}

/// Fetch a registered dynamic grid by name.
pub fn get_dynamic_grid(name: &str) -> Result<Option<DynamicGridShiftGrid>> {
    let m = dynamic_registry().read().map_err(|_| {
        ProjectionError::DatumError("dynamic grid registry lock poisoned".to_string())
    })?;
    Ok(m.get(name).cloned())
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicGridShiftGrid, DynamicGridShiftSample, GridShiftGrid, GridShiftSample,
        get_dynamic_grid, has_dynamic_grid, register_dynamic_grid, unregister_dynamic_grid,
    };

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

    #[test]
    fn dynamic_sample_zero_delta_time_returns_base_shift() {
        let grid = DynamicGridShiftGrid::new(
            "dyn_test",
            2020.0,
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![
                DynamicGridShiftSample::new(10.0, -20.0, 1.5, -2.5),
                DynamicGridShiftSample::new(10.0, -20.0, 1.5, -2.5),
                DynamicGridShiftSample::new(10.0, -20.0, 1.5, -2.5),
                DynamicGridShiftSample::new(10.0, -20.0, 1.5, -2.5),
            ],
        )
        .unwrap();

        let (dlon_deg, dlat_deg) = grid.sample_shift_degrees_at_epoch(0.5, 0.5, 2020.0).unwrap();
        assert!((dlon_deg - (10.0 / 3600.0)).abs() < 1e-12);
        assert!((dlat_deg - (-20.0 / 3600.0)).abs() < 1e-12);
    }

    #[test]
    fn dynamic_sample_nonzero_delta_time_applies_rate() {
        let grid = DynamicGridShiftGrid::new(
            "dyn_test_rate",
            2020.0,
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![
                DynamicGridShiftSample::new(0.0, 0.0, 2.0, -4.0),
                DynamicGridShiftSample::new(0.0, 0.0, 2.0, -4.0),
                DynamicGridShiftSample::new(0.0, 0.0, 2.0, -4.0),
                DynamicGridShiftSample::new(0.0, 0.0, 2.0, -4.0),
            ],
        )
        .unwrap();

        // dt = +3 years => (6, -12) arcsec
        let (dlon_deg, dlat_deg) = grid.sample_shift_degrees_at_epoch(0.25, 0.75, 2023.0).unwrap();
        assert!((dlon_deg - (6.0 / 3600.0)).abs() < 1e-12);
        assert!((dlat_deg - (-12.0 / 3600.0)).abs() < 1e-12);
    }

    #[test]
    fn dynamic_grid_registry_round_trip() {
        let grid = DynamicGridShiftGrid::new(
            "dyn_registry_test",
            2020.0,
            0.0,
            0.0,
            1.0,
            1.0,
            2,
            2,
            vec![
                DynamicGridShiftSample::new(1.0, 1.0, 0.1, 0.1),
                DynamicGridShiftSample::new(1.0, 1.0, 0.1, 0.1),
                DynamicGridShiftSample::new(1.0, 1.0, 0.1, 0.1),
                DynamicGridShiftSample::new(1.0, 1.0, 0.1, 0.1),
            ],
        )
        .unwrap();

        register_dynamic_grid(grid).unwrap();
        assert!(has_dynamic_grid("dyn_registry_test").unwrap());

        let loaded = get_dynamic_grid("dyn_registry_test").unwrap();
        assert!(loaded.is_some());
        assert!((loaded.unwrap().reference_epoch_decimal_year - 2020.0).abs() < 1e-12);

        assert!(unregister_dynamic_grid("dyn_registry_test").unwrap());
        assert!(!has_dynamic_grid("dyn_registry_test").unwrap());
    }
}