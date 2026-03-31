//! Wagner III projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const TWOTHIRD: f64 = 2.0 / 3.0;

pub(super) struct WagnerIiiProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    c_x: f64,
}

impl WagnerIiiProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat_ts = 0.0f64;
        let c_x = lat_ts.cos() / (2.0 * lat_ts / 3.0).cos();
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c_x,
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

impl ProjectionImpl for WagnerIiiProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let x = self.a * self.c_x * lon_rel * (TWOTHIRD * lat).cos() + self.fe;
        let y = self.a * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / self.a;
        let lon_rel = (x - self.fe) / (self.a * self.c_x * (TWOTHIRD * lat).cos());
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
