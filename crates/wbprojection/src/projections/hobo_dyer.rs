//! Hobo-Dyer projection (equal-area cylindrical with standard parallel 37.5°).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;

pub(super) struct HoboDyerProj {
    inner: super::cylindrical_equal_area::CylindricalEqualAreaProj,
}

impl HoboDyerProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(HoboDyerProj {
            inner: super::cylindrical_equal_area::CylindricalEqualAreaProj::new(p, 37.5)?,
        })
    }
}

impl ProjectionImpl for HoboDyerProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        self.inner.forward(lon_deg, lat_deg)
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        self.inner.inverse(x, y)
    }
}
