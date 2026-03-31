//! Werenskiold I projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const C_X: f64 = 1.0;
const C_Y: f64 = 4.442_882_938;
const S1: f64 = 0.883_883_476_483_184_4;
const RS1: f64 = 1.131_370_849_898_476;

pub(super) struct WerenskioldIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl WerenskioldIProj {
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

impl ProjectionImpl for WerenskioldIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds("latitude outside valid range [-90, 90]"));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let phi1 = (S1 * lat.sin()).clamp(-1.0, 1.0).asin();
        let mut x = C_X * lon_rel * phi1.cos();
        let phi2 = phi1 / 3.0;
        let c = phi2.cos();
        if c.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds("Werenskiold I forward undefined at pole"));
        }
        x /= c;
        let y = C_Y * phi2.sin();

        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut phi = (yn / C_Y).clamp(-1.0, 1.0).asin();
        let c = phi.cos();
        if c.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds("Werenskiold I inverse undefined at pole"));
        }

        let mut lon_rel = xn * c / C_X;
        phi *= 3.0;
        let c3 = phi.cos();
        if c3.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds("Werenskiold I inverse undefined at pole"));
        }
        lon_rel /= c3;
        let lat = (RS1 * phi.sin()).clamp(-1.0, 1.0).asin();
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
