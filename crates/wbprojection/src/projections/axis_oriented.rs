use crate::error::Result;

use super::ProjectionImpl;

/// Wraps a projection and flips one or both projected axes about false origin.
///
/// This is used for EPSG methods with south/west-oriented axis conventions,
/// while preserving the underlying map projection mathematics.
pub(super) struct AxisOrientedProj {
    inner: Box<dyn ProjectionImpl>,
    fe: f64,
    fn_: f64,
    flip_x: bool,
    flip_y: bool,
}

impl AxisOrientedProj {
    pub fn new(
        inner: Box<dyn ProjectionImpl>,
        fe: f64,
        fn_: f64,
        flip_x: bool,
        flip_y: bool,
    ) -> Self {
        Self {
            inner,
            fe,
            fn_,
            flip_x,
            flip_y,
        }
    }

    fn apply_axis_orientation(&self, x: f64, y: f64) -> (f64, f64) {
        let xo = if self.flip_x { self.fe - (x - self.fe) } else { x };
        let yo = if self.flip_y { self.fn_ - (y - self.fn_) } else { y };
        (xo, yo)
    }

    fn remove_axis_orientation(&self, x: f64, y: f64) -> (f64, f64) {
        // Reflection is its own inverse.
        self.apply_axis_orientation(x, y)
    }
}

impl ProjectionImpl for AxisOrientedProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let (x, y) = self.inner.forward(lon_deg, lat_deg)?;
        Ok(self.apply_axis_orientation(x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let (xu, yu) = self.remove_axis_orientation(x, y);
        self.inner.inverse(xu, yu)
    }
}
