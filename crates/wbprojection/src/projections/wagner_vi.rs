//! Wagner VI projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct WagnerViProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl WagnerViProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(WagnerViProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }

    fn wrap_lon(mut lon: f64) -> f64 {
        while lon > PI {
            lon -= 2.0 * PI;
        }
        while lon < -PI {
            lon += 2.0 * PI;
        }
        lon
    }
}

impl ProjectionImpl for WagnerViProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let factor = (1.0 - 3.0 * (lat / PI).powi(2)).max(0.0).sqrt();
        let x = self.a * lon_rel * factor + self.fe;
        let y = self.a * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / self.a;
        let factor = (1.0 - 3.0 * (lat / PI).powi(2)).max(0.0).sqrt();
        let lon_rel = if factor.abs() < 1e-15 { 0.0 } else { (x - self.fe) / (self.a * factor) };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
