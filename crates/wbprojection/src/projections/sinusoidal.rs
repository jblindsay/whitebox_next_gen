//! Sinusoidal (Sanson-Flamsteed) equal-area pseudocylindrical projection.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct SinusoidalProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl SinusoidalProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(SinusoidalProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for SinusoidalProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let x = self.a * (lon - self.lon0) * lat.cos() + self.fe;
        let y = self.a * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / self.a;
        let cos_lat = lat.cos();
        let lon = if cos_lat.abs() < 1e-12 {
            self.lon0
        } else {
            (x - self.fe) / (self.a * cos_lat) + self.lon0
        };
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
