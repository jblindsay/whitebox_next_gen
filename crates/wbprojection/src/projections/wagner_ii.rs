//! Wagner II projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const C_X: f64 = 0.924_83;
const C_Y: f64 = 1.387_25;
const C_P1: f64 = 0.880_22;
const C_P2: f64 = 0.885_50;

pub(super) struct WagnerIiProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl WagnerIiProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(Self {
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

impl ProjectionImpl for WagnerIiProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let phi = (C_P1 * (C_P2 * lat).sin()).clamp(-1.0, 1.0).asin();
        let x = self.a * C_X * lon_rel * phi.cos() + self.fe;
        let y = self.a * C_Y * phi + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let phi = (y - self.fn_) / (self.a * C_Y);
        let lon_rel = (x - self.fe) / (self.a * C_X * phi.cos());
        let lat = ((phi.sin() / C_P1).clamp(-1.0, 1.0).asin()) / C_P2;
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
