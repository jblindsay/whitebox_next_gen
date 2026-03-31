//! Tobler-Mercator projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS: f64 = 1e-12;

pub(super) struct ToblerMercatorProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    k0: f64,
}

impl ToblerMercatorProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            k0: p.scale,
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

impl ProjectionImpl for ToblerMercatorProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        if phi.abs() >= FRAC_PI_2 - EPS {
            return Err(ProjectionError::out_of_bounds(
                "Tobler-Mercator is undefined at the poles",
            ));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let cosphi = phi.cos();
        let x = self.k0 * lon_rel * cosphi * cosphi;
        let y = self.k0 * phi.tan().asinh();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let phi = (yn / self.k0).sinh().atan();
        let cosphi = phi.cos();
        if cosphi.abs() < EPS {
            return Err(ProjectionError::out_of_bounds(
                "inverse unstable near poles",
            ));
        }
        let lon_rel = xn / (self.k0 * cosphi * cosphi);
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
