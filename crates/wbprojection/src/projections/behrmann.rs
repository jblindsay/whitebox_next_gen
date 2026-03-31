//! Behrmann projection (equal-area cylindrical with standard parallel 30°).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;

pub(super) struct BehrmannProj {
    inner: super::cylindrical_equal_area::CylindricalEqualAreaProj,
}

impl BehrmannProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(BehrmannProj {
            inner: super::cylindrical_equal_area::CylindricalEqualAreaProj::new(p, 30.0)?,
        })
    }
}

impl ProjectionImpl for BehrmannProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        self.inner.forward(lon_deg, lat_deg)
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        self.inner.inverse(x, y)
    }
}
