//! Hammer (Hammer-Aitoff) equal-area projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

pub(super) struct HammerProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl HammerProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(HammerProj {
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

impl ProjectionImpl for HammerProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);

        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds(
                "latitude outside valid range [-90, 90]",
            ));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let denom = (1.0 + lat.cos() * (0.5 * lon_rel).cos()).sqrt();
        if denom <= 1e-14 {
            return Err(ProjectionError::out_of_bounds(
                "point maps to infinity in Hammer projection",
            ));
        }

        let x = self.a * (2.0 * SQRT_2 * lat.cos() * (0.5 * lon_rel).sin() / denom) + self.fe;
        let y = self.a * (SQRT_2 * lat.sin() / denom) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = (x - self.fe) / self.a;
        let y = (y - self.fn_) / self.a;

        let t = 1.0 - (x * x) / 16.0 - (y * y) / 4.0;
        if t < -1e-14 {
            return Err(ProjectionError::out_of_bounds(
                "point outside Hammer projection bounds",
            ));
        }

        let z = t.max(0.0).sqrt();
        let lon_rel = 2.0 * (z * x).atan2(2.0 * (2.0 * z * z - 1.0));
        let lat = (z * y).clamp(-1.0, 1.0).asin();

        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
