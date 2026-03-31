//! Foucaut projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct FoucautProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    c_x: f64,
    c_y: f64,
    c_p: f64,
    tan_mode: bool,
}

impl FoucautProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let proj_p = 2.0;
        let proj_q = 2.0;
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c_x: proj_q / proj_p,
            c_y: proj_p,
            c_p: 1.0 / proj_q,
            tan_mode: true,
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

impl ProjectionImpl for FoucautProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let mut x = self.c_x * lon_rel * lat.cos();
        let mut y = self.c_y;
        let lat_q = lat * self.c_p;
        let c = lat_q.cos();
        if self.tan_mode {
            x *= c * c;
            y *= lat_q.tan();
        } else {
            x /= c;
            y *= lat_q.sin();
        }
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let mut yn = (y - self.fn_) / self.a;

        yn /= self.c_y;
        let mut lat = if self.tan_mode { yn.atan() } else { yn.clamp(-1.0, 1.0).asin() };
        let c = lat.cos();
        lat /= self.c_p;
        let mut lon_rel = xn / (self.c_x * lat.cos());
        if self.tan_mode {
            lon_rel /= c * c;
        } else {
            lon_rel *= c;
        }
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
