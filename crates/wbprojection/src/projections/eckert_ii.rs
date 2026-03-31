//! Eckert II projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const FXC: f64 = 0.460_658_865_961_780_66;
const FYC: f64 = 1.447_202_509_116_535_3;
const C13: f64 = 1.0 / 3.0;
const ONEEPS: f64 = 1.000_000_1;

pub(super) struct EckertIiProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertIiProj {
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

impl ProjectionImpl for EckertIiProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg).clamp(-FRAC_PI_2, FRAC_PI_2);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let t = (4.0 - 3.0 * lat.abs().sin()).sqrt();
        let x = self.a * FXC * lon_rel * t + self.fe;
        let mut y = self.a * FYC * (2.0 - t);
        if lat < 0.0 {
            y = -y;
        }
        y += self.fn_;

        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let yn = (y - self.fn_) / self.a;

        let mut t = 2.0 - yn.abs() / FYC;
        let lon_rel = if t.abs() < 1e-15 {
            0.0
        } else {
            (x - self.fe) / (self.a * FXC * t)
        };

        t = (4.0 - t * t) * C13;
        let lat = if t.abs() >= 1.0 {
            if t.abs() > ONEEPS {
                return Err(ProjectionError::out_of_bounds(
                    "coordinate outside Eckert II inverse domain",
                ));
            }
            if t < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 }
        } else {
            t.asin()
        };

        let lat = if yn < 0.0 { -lat } else { lat };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
