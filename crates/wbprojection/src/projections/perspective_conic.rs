//! Perspective Conic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS: f64 = 1e-10;

pub(super) struct PerspectiveConicProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    n: f64,
    c1: f64,
    c2: f64,
    sig: f64,
    rho_0: f64,
}

impl PerspectiveConicProj {
    pub fn new(p: &ProjectionParams, lat1: f64, lat2: f64) -> Result<Self> {
        let phi1 = to_radians(lat1);
        let phi2 = to_radians(lat2);
        let lat0 = to_radians(p.lat0);
        let del = 0.5 * (phi2 - phi1);
        let sig = 0.5 * (phi2 + phi1);

        if del.abs() < EPS || sig.abs() < EPS {
            return Err(ProjectionError::InvalidParameter {
                param: "lat1/lat2".to_string(),
                reason: "Perspective Conic requires |lat1-lat2|>0 and |lat1+lat2|>0"
                    .to_string(),
            });
        }

        let n = sig.sin();
        let c2 = del.cos();
        let c1 = 1.0 / sig.tan();
        let del0 = lat0 - sig;
        if del0.abs() >= FRAC_PI_2 - EPS {
            return Err(ProjectionError::InvalidParameter {
                param: "lat0/lat1/lat2".to_string(),
                reason:
                    "Perspective Conic requires |lat0 - 0.5*(lat1+lat2)| < 90°".to_string(),
            });
        }
        let rho_0 = c2 * (c1 - del0.tan());

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            n,
            c1,
            c2,
            sig,
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

impl ProjectionImpl for PerspectiveConicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0) * self.n;

        let rho = self.c2 * (self.c1 - (phi - self.sig).tan());
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
        let phi = (self.c1 - rho / self.c2).atan() + self.sig;
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
