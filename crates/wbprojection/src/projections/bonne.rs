//! Bonne projection (spherical, standard parallel fixed at 45°).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) struct BonneProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    phi1: f64,
    cot_phi1: f64,
}

impl BonneProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let phi1 = to_radians(45.0);
        Ok(BonneProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            phi1,
            cot_phi1: 1.0 / phi1.tan(),
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

impl ProjectionImpl for BonneProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds("latitude outside valid range [-90, 90]"));
        }
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let rho = self.cot_phi1 + self.phi1 - lat;
        if rho.abs() < 1e-15 {
            return Ok((self.fe, self.fn_));
        }

        let e = lon_rel * lat.cos() / rho;
        let x = self.a * rho * e.sin() + self.fe;
        let y = self.a * (self.cot_phi1 - rho * e.cos()) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = (x - self.fe) / self.a;
        let y = (y - self.fn_) / self.a;

        let rho = (x * x + (self.cot_phi1 - y).powi(2)).sqrt();
        let lat = self.cot_phi1 + self.phi1 - rho;
        let cos_lat = lat.cos();

        let lon_rel = if cos_lat.abs() < 1e-15 {
            0.0
        } else {
            rho * x.atan2(self.cot_phi1 - y) / cos_lat
        };

        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
