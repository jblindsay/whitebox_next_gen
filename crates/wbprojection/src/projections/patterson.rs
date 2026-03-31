//! Patterson cylindrical projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const K1: f64 = 1.0148;
const K2: f64 = 0.23185;
const K3: f64 = -0.14499;
const K4: f64 = 0.02406;
const C1: f64 = K1;
const C2: f64 = 5.0 * K2;
const C3: f64 = 7.0 * K3;
const C4: f64 = 9.0 * K4;
const EPS11: f64 = 1e-11;
const MAX_Y: f64 = 1.790_857_183;
const MAX_ITER: usize = 100;

pub(super) struct PattersonProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl PattersonProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(PattersonProj {
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

impl ProjectionImpl for PattersonProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let phi2 = lat * lat;
        let x = lon_rel;
        let y = lat * (K1 + phi2 * phi2 * (K2 + phi2 * (K3 + K4 * phi2)));

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;
        let y_clamped = yn.clamp(-MAX_Y, MAX_Y);

        let mut yc = y_clamped;
        for _ in 0..MAX_ITER {
            let y2 = yc * yc;
            let f = yc * (K1 + y2 * y2 * (K2 + y2 * (K3 + K4 * y2))) - y_clamped;
            let fder = C1 + y2 * y2 * (C2 + y2 * (C3 + C4 * y2));
            if fder.abs() < 1e-15 {
                break;
            }
            let tol = f / fder;
            yc -= tol;
            if tol.abs() < EPS11 {
                let lon = Self::wrap_lon(self.lon0 + xn);
                return Ok((to_degrees(lon), to_degrees(yc)));
            }
        }

        Err(ProjectionError::ConvergenceFailure {
            iterations: MAX_ITER,
        })
    }
}
