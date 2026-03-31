//! Winkel I projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct WinkelIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    cosphi1: f64,
}

impl WinkelIProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat_ts = 50.467;
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            cosphi1: to_radians(lat_ts).cos(),
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

impl ProjectionImpl for WinkelIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let x = 0.5 * lon_rel * (self.cosphi1 + lat.cos());
        let y = lat;

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;
        let lat = yn;

        let denom = self.cosphi1 + lat.cos();
        let lon_rel = if denom.abs() < f64::EPSILON {
            0.0
        } else {
            2.0 * xn / denom
        };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
