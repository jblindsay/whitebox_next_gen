//! Tissot conic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const EPS: f64 = 1e-10;

pub(super) struct TissotProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    n: f64,
    rho_c: f64,
    rho_0: f64,
}

impl TissotProj {
    pub fn new(p: &ProjectionParams, lat1: f64, lat2: f64) -> Result<Self> {
        let phi1 = to_radians(lat1);
        let phi2 = to_radians(lat2);
        let lat0 = to_radians(p.lat0);
        let del = 0.5 * (phi2 - phi1);
        let sig = 0.5 * (phi2 + phi1);
        if del.abs() < EPS || sig.abs() < EPS {
            return Err(ProjectionError::InvalidParameter {
                param: "lat1/lat2".to_string(),
                reason: "Tissot requires |lat1-lat2|>0 and |lat1+lat2|>0".to_string(),
            });
        }

        let cs = sig.cos();
        let n = sig.sin();
        let rho_c = n / cs + cs / n;
        let rho_0 = ((rho_c - 2.0 * lat0.sin()) / n).sqrt();

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            n,
            rho_c,
            rho_0,
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

impl ProjectionImpl for TissotProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0) * self.n;

        let rho_term = self.rho_c - 2.0 * phi.sin() / self.n;
        if rho_term < 0.0 {
            return Err(ProjectionError::out_of_bounds(
                "latitude is outside the valid domain for this Tissot configuration",
            ));
        }
        let rho = rho_term.sqrt();
        let x = rho * lam.sin();
        let y = self.rho_0 - rho * lam.cos();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let mut xn = (x - self.fe) / self.a;
        let mut yn = self.rho_0 - (y - self.fn_) / self.a;
        let mut rho = (xn * xn + yn * yn).sqrt();
        if self.n < 0.0 {
            rho = -rho;
            xn = -xn;
            yn = -yn;
        }

        let lon_rel = xn.atan2(yn) / self.n;
        let arg = 0.5 * (self.rho_c - rho * rho) * self.n;
        let phi = arg.clamp(-1.0, 1.0).asin();
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
