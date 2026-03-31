//! Mollweide equal-area pseudocylindrical projection.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};
use std::f64::consts::PI;

pub(super) struct MollweideProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl MollweideProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(MollweideProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }

    /// Iteratively solve 2θ + sin(2θ) = π·sin(φ) for θ.
    fn solve_theta(lat: f64) -> f64 {
        if lat.abs() >= std::f64::consts::FRAC_PI_2 {
            return lat.signum() * std::f64::consts::FRAC_PI_2;
        }
        let target = PI * lat.sin();
        let mut theta = lat;
        for _ in 0..50 {
            let delta = -(2.0 * theta + (2.0 * theta).sin() - target)
                / (2.0 + 2.0 * (2.0 * theta).cos());
            theta += delta;
            if delta.abs() < 1e-12 {
                break;
            }
        }
        theta
    }
}

impl ProjectionImpl for MollweideProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let theta = MollweideProj::solve_theta(lat);
        let sqrt2 = 2.0f64.sqrt();
        let x = self.a * 2.0 * sqrt2 / PI * (lon - self.lon0) * theta.cos() + self.fe;
        let y = self.a * sqrt2 * theta.sin() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let sqrt2 = 2.0f64.sqrt();
        let theta = ((y - self.fn_) / (self.a * sqrt2)).asin();
        let lat = ((2.0 * theta + (2.0 * theta).sin()) / PI).asin();
        let cos_theta = theta.cos();
        let lon = if cos_theta.abs() < 1e-12 {
            self.lon0
        } else {
            self.lon0 + PI * (x - self.fe) / (2.0 * sqrt2 * self.a * cos_theta)
        };
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
