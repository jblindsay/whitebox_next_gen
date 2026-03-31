//! Transverse Cylindrical Equal Area projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const EPS: f64 = 1e-12;

pub(super) struct TransverseCylindricalEqualAreaProj {
    lon0: f64,
    lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    k0: f64,
}

impl TransverseCylindricalEqualAreaProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        if p.scale.abs() < EPS {
            return Err(ProjectionError::invalid_param("scale", "must be non-zero"));
        }
        Ok(Self {
            lon0: to_radians(p.lon0),
            lat0: to_radians(p.lat0),
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

impl ProjectionImpl for TransverseCylindricalEqualAreaProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let x = phi.cos() * lam.sin() / self.k0;
        let y = self.k0 * ((phi.tan()).atan2(lam.cos()) - self.lat0);
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let mut xn = (x - self.fe) / self.a;
        let mut yn = (y - self.fn_) / self.a;

        yn = yn / self.k0 + self.lat0;
        xn *= self.k0;
        let t = (1.0 - xn * xn).sqrt();
        let phi = (t * yn.sin()).clamp(-1.0, 1.0).asin();
        let lam = xn.atan2(t * yn.cos());
        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
