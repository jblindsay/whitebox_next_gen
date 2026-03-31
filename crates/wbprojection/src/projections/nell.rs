//! Nell projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const MAX_ITER: usize = 10;
const LOOP_TOL: f64 = 1e-7;

pub(super) struct NellProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl NellProj {
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

impl ProjectionImpl for NellProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let k = 2.0 * phi.sin();
        let phi2 = phi * phi;
        phi *= 1.00371 + phi2 * (-0.0935382 + phi2 * -0.011412);
        let mut i = MAX_ITER;
        while i > 0 {
            let v = (phi + phi.sin() - k) / (1.0 + phi.cos());
            phi -= v;
            if v.abs() < LOOP_TOL {
                break;
            }
            i -= 1;
        }

        let x = 0.5 * lam * (1.0 + phi.cos());
        let y = phi;
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let lam = 2.0 * xn / (1.0 + yn.cos());
        let phi = (0.5 * (yn + yn.sin())).clamp(-1.0, 1.0).asin();

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
