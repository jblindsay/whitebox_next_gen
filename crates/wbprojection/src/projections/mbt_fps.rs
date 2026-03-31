//! McBryde-Thomas Flat-Pole Sine (No. 2) projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const MAX_ITER: usize = 10;
const LOOP_TOL: f64 = 1e-7;
const C1: f64 = 0.45503;
const C2: f64 = 1.36509;
const C3: f64 = 1.41546;
const C_X: f64 = 0.22248;
const C_Y: f64 = 1.44492;
const C1_2: f64 = 1.0 / 3.0;

pub(super) struct MbtFpsProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl MbtFpsProj {
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

impl ProjectionImpl for MbtFpsProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let k = C3 * phi.sin();
        let mut i = MAX_ITER;
        while i > 0 {
            let t = phi / C2;
            let v = (C1 * t.sin() + phi.sin() - k) / (C1_2 * t.cos() + phi.cos());
            phi -= v;
            if v.abs() < LOOP_TOL {
                break;
            }
            i -= 1;
        }

        let t = phi / C2;
        let x = C_X * lam * (1.0 + 3.0 * phi.cos() / t.cos());
        let y = C_Y * t.sin();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let t = (yn / C_Y).clamp(-1.0, 1.0).asin();
        let phi_t = C2 * t;
        let lam = xn / (C_X * (1.0 + 3.0 * phi_t.cos() / t.cos()));
        let phi = ((C1 * t.sin() + phi_t.sin()) / C3).clamp(-1.0, 1.0).asin();

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
