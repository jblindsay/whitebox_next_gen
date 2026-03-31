//! Lagrange projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const TOL: f64 = 1e-10;

pub(super) struct LagrangeProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    a1: f64,
    a2: f64,
    hw: f64,
    hrw: f64,
    rw: f64,
    w: f64,
}

impl LagrangeProj {
    pub fn new(p: &ProjectionParams, lat1: f64, w: f64) -> Result<Self> {
        if w <= 0.0 {
            return Err(ProjectionError::invalid_param("w", "must be > 0"));
        }

        let phi1 = to_radians(lat1);
        let sin_phi1 = phi1.sin();
        if (sin_phi1.abs() - 1.0).abs() < TOL {
            return Err(ProjectionError::invalid_param("lat1", "|lat1| must be < 90°"));
        }

        let hw = 0.5 * w;
        let rw = 1.0 / w;
        let hrw = 0.5 * rw;
        let a1 = ((1.0 - sin_phi1) / (1.0 + sin_phi1)).powf(hrw);
        let a2 = a1 * a1;

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            a1,
            a2,
            hw,
            hrw,
            rw,
            w,
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

impl ProjectionImpl for LagrangeProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let mut lam = Self::wrap_lon(lon - self.lon0);

        let sin_phi = phi.sin();
        if (sin_phi.abs() - 1.0).abs() < TOL {
            let y = if phi < 0.0 { -2.0 } else { 2.0 };
            return Ok((self.fe, self.a * y + self.fn_));
        }

        let v = self.a1 * ((1.0 + sin_phi) / (1.0 - sin_phi)).powf(self.hrw);
        lam *= self.rw;
        let c = 0.5 * (v + 1.0 / v) + lam.cos();
        if c < TOL {
            return Err(ProjectionError::out_of_bounds("Lagrange forward outside domain"));
        }

        let x = 2.0 * lam.sin() / c;
        let y = (v - 1.0 / v) / c;
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        if (yn.abs() - 2.0).abs() < TOL {
            let lat = if yn < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
            return Ok((to_degrees(self.lon0), to_degrees(lat)));
        }

        let x2 = xn * xn;
        let y2p = 2.0 + yn;
        let y2m = 2.0 - yn;
        let c = y2p * y2m - x2;
        if c.abs() < TOL {
            return Err(ProjectionError::out_of_bounds("Lagrange inverse outside domain"));
        }

        let lat = 2.0
            * (((y2p * y2p + x2) / (self.a2 * (y2m * y2m + x2))).powf(self.hw)).atan()
            - FRAC_PI_2;
        let lon_rel = self.w * (4.0 * xn).atan2(c);
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
