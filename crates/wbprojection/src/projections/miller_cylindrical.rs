//! Miller cylindrical projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct MillerCylindricalProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl MillerCylindricalProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(MillerCylindricalProj {
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

impl ProjectionImpl for MillerCylindricalProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);

        if lat.abs() >= std::f64::consts::FRAC_PI_2 {
            return Err(ProjectionError::out_of_bounds(
                "latitude ±90° is a singularity for Miller cylindrical",
            ));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let x = self.a * lon_rel + self.fe;
        let y = self.a * 1.25 * (std::f64::consts::FRAC_PI_4 + 0.4 * lat).tan().ln() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lon_rel = (x - self.fe) / self.a;
        let lat = 2.5 * ((0.8 * (y - self.fn_) / self.a).exp()).atan() - 0.625 * PI;
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
