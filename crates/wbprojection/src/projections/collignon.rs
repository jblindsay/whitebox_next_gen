//! Collignon projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const FXC: f64 = 1.128_379_167_095_512_6;
const FYC: f64 = 1.772_453_850_905_516;
const ONEEPS: f64 = 1.000_000_1;

pub(super) struct CollignonProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl CollignonProj {
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

impl ProjectionImpl for CollignonProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg).clamp(-FRAC_PI_2, FRAC_PI_2);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let mut t = 1.0 - lat.sin();
        if t <= 0.0 {
            t = 0.0;
        } else {
            t = t.sqrt();
        }

        let x = self.a * FXC * lon_rel * t + self.fe;
        let y = self.a * FYC * (1.0 - t) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let yn = (y - self.fn_) / self.a;
        let mut t = yn / FYC - 1.0;
        t = 1.0 - t * t;

        let lat = if t.abs() < 1.0 {
            t.asin()
        } else if t.abs() > ONEEPS {
            return Err(ProjectionError::out_of_bounds(
                "coordinate outside Collignon inverse domain",
            ));
        } else if t < 0.0 {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        };

        let mut d = 1.0 - lat.sin();
        let lon_rel = if d <= 0.0 {
            0.0
        } else {
            d = d.sqrt();
            (x - self.fe) / (self.a * FXC * d)
        };

        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
