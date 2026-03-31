//! Gall-Peters projection (equal-area cylindrical with standard parallel 45°).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;

pub(super) struct GallPetersProj {
    inner: super::cylindrical_equal_area::CylindricalEqualAreaProj,
}

impl GallPetersProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(GallPetersProj {
            inner: super::cylindrical_equal_area::CylindricalEqualAreaProj::new(p, 45.0)?,
        })
    }
}

impl ProjectionImpl for GallPetersProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        self.inner.forward(lon_deg, lat_deg)
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        self.inner.inverse(x, y)
    }
}
