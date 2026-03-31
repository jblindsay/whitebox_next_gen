//! Nell-Hammer projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const NITER: usize = 9;
const EPS: f64 = 1e-7;

pub(super) struct NellHammerProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl NellHammerProj {
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

impl ProjectionImpl for NellHammerProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let x = 0.5 * lam * (1.0 + phi.cos());
        let y = 2.0 * (phi - (0.5 * phi).tan());
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let p = 0.5 * yn;
        let mut phi = p;
        let mut i = NITER;
        while i > 0 {
            let c = (0.5 * phi).cos();
            let v = (phi - (0.5 * phi).tan() - p) / (1.0 - 0.5 / (c * c));
            phi -= v;
            if v.abs() < EPS {
                break;
            }
            i -= 1;
        }

        let lam = if i == 0 {
            phi = if p < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
            2.0 * xn
        } else {
            2.0 * xn / (1.0 + phi.cos())
        };

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
