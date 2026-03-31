//! Fahey projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const TOL: f64 = 1e-6;

pub(super) struct FaheyProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl FaheyProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(FaheyProj {
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

impl ProjectionImpl for FaheyProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let t = (0.5 * lat).tan();
        let y = 1.819_152 * t;
        let rad = (1.0 - t * t).max(0.0);
        let x = 0.819_152 * lon_rel * rad.sqrt();

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let mut yn = (y - self.fn_) / self.a;

        yn /= 1.819_152;
        let lat = 2.0 * yn.atan();
        let rad = 1.0 - yn * yn;
        let lon_rel = if rad.abs() < TOL {
            0.0
        } else {
            xn / (0.819_152 * rad.max(0.0).sqrt())
        };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
