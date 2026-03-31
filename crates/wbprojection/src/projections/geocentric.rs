//! Geocentric projection kind placeholder.
//!
//! Geocentric CRS uses 3D ECEF coordinates and is handled by `Crs::transform_to_3d`.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};

pub(super) struct GeocentricProj;

impl GeocentricProj {
    pub fn new(_p: &ProjectionParams) -> Result<Self> {
        Ok(Self)
    }
}

impl ProjectionImpl for GeocentricProj {
    fn forward(&self, _lon_deg: f64, _lat_deg: f64) -> Result<(f64, f64)> {
        Err(ProjectionError::UnsupportedProjection(
            "Geocentric CRS requires 3D transform API".to_string(),
        ))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64)> {
        Err(ProjectionError::UnsupportedProjection(
            "Geocentric CRS requires 3D transform API".to_string(),
        ))
    }
}
