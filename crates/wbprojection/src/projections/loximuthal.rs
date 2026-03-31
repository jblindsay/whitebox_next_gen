//! Loximuthal projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const EPS: f64 = 1e-8;

pub(super) struct LoximuthalProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    phi1: f64,
    cosphi1: f64,
    tanphi1: f64,
}

impl LoximuthalProj {
    pub fn new(p: &ProjectionParams, lat1: f64) -> Result<Self> {
        let phi1 = to_radians(lat1);
        let cosphi1 = phi1.cos();
        let tanphi1 = (FRAC_PI_4 + 0.5 * phi1).tan();
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            phi1,
            cosphi1,
            tanphi1,
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

impl ProjectionImpl for LoximuthalProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let y = phi - self.phi1;
        let x = if y.abs() < EPS {
            lam * self.cosphi1
        } else {
            let t = FRAC_PI_4 + 0.5 * phi;
            if t.abs() < EPS || (t.abs() - FRAC_PI_2).abs() < EPS {
                0.0
            } else {
                lam * y / ((t.tan() / self.tanphi1).ln())
            }
        };

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let phi = yn + self.phi1;
        let lam = if yn.abs() < EPS {
            xn / self.cosphi1
        } else {
            let t = FRAC_PI_4 + 0.5 * phi;
            if t.abs() < EPS || (t.abs() - FRAC_PI_2).abs() < EPS {
                0.0
            } else {
                xn * ((t.tan() / self.tanphi1).ln()) / yn
            }
        };

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
