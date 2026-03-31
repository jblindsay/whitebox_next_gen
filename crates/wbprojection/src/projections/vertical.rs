//! Vertical projection kind placeholder.
//!
//! Vertical CRS is height-only; horizontal projection calls are unsupported.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};

pub(super) struct VerticalProj;

impl VerticalProj {
    pub fn new(_p: &ProjectionParams) -> Result<Self> {
        Ok(Self)
    }
}

impl ProjectionImpl for VerticalProj {
    fn forward(&self, _lon_deg: f64, _lat_deg: f64) -> Result<(f64, f64)> {
        Err(ProjectionError::UnsupportedProjection(
            "Vertical CRS requires height-only handling".to_string(),
        ))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64)> {
        Err(ProjectionError::UnsupportedProjection(
            "Vertical CRS requires height-only handling".to_string(),
        ))
    }
}
