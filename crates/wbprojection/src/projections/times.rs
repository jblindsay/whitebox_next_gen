//! Times projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct TimesProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl TimesProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(TimesProj {
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

impl ProjectionImpl for TimesProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let t = (0.5 * lat).tan();
        let s = (std::f64::consts::FRAC_PI_4 * t).sin();
        let s2 = s * s;
        let x = lon_rel * (0.744_82 - 0.345_88 * s2);
        let y = 1.707_11 * t;

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let t = yn / 1.707_11;
        let s = (std::f64::consts::FRAC_PI_4 * t).sin();
        let s2 = s * s;
        let denom = 0.744_82 - 0.345_88 * s2;
        if denom.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds("Times inverse denominator is zero"));
        }

        let lon_rel = xn / denom;
        let lat = 2.0 * t.atan();
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
