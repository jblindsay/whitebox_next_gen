//! Eckert IV equal-area pseudocylindrical projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) struct EckertIvProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertIvProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(EckertIvProj {
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

    fn cx() -> f64 {
        2.0 / (PI * (4.0 + PI)).sqrt()
    }

    fn cy() -> f64 {
        2.0 * (PI / (4.0 + PI)).sqrt()
    }

    fn theta_for_lat(lat: f64) -> Result<f64> {
        let target = (2.0 + PI / 2.0) * lat.sin();
        let mut theta = lat;

        for _ in 0..30 {
            let sin_t = theta.sin();
            let cos_t = theta.cos();
            let f = theta + sin_t * cos_t + 2.0 * sin_t - target;
            let fp = 1.0 + cos_t * cos_t - sin_t * sin_t + 2.0 * cos_t;
            if fp.abs() < 1e-14 {
                break;
            }
            let d = -f / fp;
            theta += d;
            if d.abs() < 1e-12 {
                return Ok(theta);
            }
        }

        Err(ProjectionError::ConvergenceFailure { iterations: 30 })
    }
}

impl ProjectionImpl for EckertIvProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);

        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds(
                "latitude outside valid range [-90, 90]",
            ));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let theta = Self::theta_for_lat(lat)?;

        let x = self.a * Self::cx() * lon_rel * (1.0 + theta.cos()) + self.fe;
        let y = self.a * Self::cy() * theta.sin() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = (x - self.fe) / self.a;
        let y = (y - self.fn_) / self.a;

        let sin_theta = (y / Self::cy()).clamp(-1.0, 1.0);
        let theta = sin_theta.asin();
        let cos_theta = theta.cos();

        let lat_term = (theta + theta.sin() * cos_theta + 2.0 * theta.sin()) / (2.0 + PI / 2.0);
        let lat = lat_term.clamp(-1.0, 1.0).asin();

        let denom = Self::cx() * (1.0 + cos_theta);
        if denom.abs() < 1e-14 {
            return Err(ProjectionError::out_of_bounds(
                "point outside Eckert IV inverse domain",
            ));
        }
        let lon_rel = x / denom;
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
