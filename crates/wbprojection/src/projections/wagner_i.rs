//! Wagner I (Kavrayskiy VI) projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const C_X: f64 = 0.877_382_675_3;
const CY: f64 = 1.139_753_528_477;
const N: f64 = 0.866_025_403_784_438_6;

pub(super) struct WagnerIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    c_y: f64,
}

impl WagnerIProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c_y: CY / N,
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

impl ProjectionImpl for WagnerIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        lat = (N * lat.sin()).clamp(-1.0, 1.0).asin();
        let x = self.a * C_X * lon_rel * lat.cos() + self.fe;
        let y = self.a * self.c_y * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let yn = (y - self.fn_) / self.a;
        let lat_prime = yn / self.c_y;
        let lat = (lat_prime.sin() / N).clamp(-1.0, 1.0).asin();
        let lon_rel = (x - self.fe) / (self.a * C_X * lat_prime.cos());
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
