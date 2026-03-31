//! Central Conic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const EPS: f64 = 1e-10;

pub(super) struct CentralConicProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    phi1: f64,
    ctgphi1: f64,
    sinphi1: f64,
}

impl CentralConicProj {
    pub fn new(p: &ProjectionParams, lat1: f64) -> Result<Self> {
        let phi1 = to_radians(lat1);
        if phi1.abs() < EPS {
            return Err(ProjectionError::invalid_param(
                "lat1",
                "Central Conic requires |lat1| > 0",
            ));
        }
        let sinphi1 = phi1.sin();
        let cosphi1 = phi1.cos();
        let ctgphi1 = cosphi1 / sinphi1;

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            phi1,
            ctgphi1,
            sinphi1,
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

impl ProjectionImpl for CentralConicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let r = self.ctgphi1 - (lat - self.phi1).tan();
        let x = r * (lam * self.sinphi1).sin();
        let y = self.ctgphi1 - r * (lam * self.sinphi1).cos();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let yy = self.ctgphi1 - yn;
        let lat = self.phi1 - ((xn * xn + yy * yy).sqrt() - self.ctgphi1).atan();
        let lon_rel = xn.atan2(yy) / self.sinphi1;
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
