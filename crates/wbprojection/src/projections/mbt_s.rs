//! McBryde-Thomas Flat-Polar Sine (No. 1) projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct MbtSProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    c_x: f64,
    c_y: f64,
    c_p: f64,
}

impl MbtSProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let proj_p = 1.48875;
        let proj_q = 1.36509;
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c_x: proj_q / proj_p,
            c_y: proj_p,
            c_p: 1.0 / proj_q,
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

impl ProjectionImpl for MbtSProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let lat_q = lat * self.c_p;
        let c = lat_q.cos();
        let x = self.c_x * lon_rel * lat.cos() / c;
        let y = self.c_y * lat_q.sin();

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let lat_q = (yn / self.c_y).clamp(-1.0, 1.0).asin();
        let c = lat_q.cos();
        let lat = lat_q / self.c_p;
        let lon_rel = xn * c / (self.c_x * lat.cos());
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
