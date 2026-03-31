//! Geographic lon/lat degree pass-through projection.

use crate::error::Result;
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct GeographicProj;

impl GeographicProj {
    pub fn new(_p: &ProjectionParams) -> Result<Self> {
        Ok(Self)
    }
}

impl ProjectionImpl for GeographicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        Ok((lon_deg, lat_deg))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        Ok((x, y))
    }
}
