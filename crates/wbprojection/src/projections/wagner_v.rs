//! Wagner V projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const LOOP_TOL: f64 = 1e-7;
const MAX_ITER: usize = 30;

pub(super) struct WagnerVProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    c_x: f64,
    c_y: f64,
    c_p: f64,
}

impl WagnerVProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c_x: 0.909_77,
            c_y: 1.650_14,
            c_p: 3.008_96,
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

    fn solve_theta(lat: f64, c_p: f64) -> f64 {
        if lat.abs() >= FRAC_PI_2 {
            return lat.signum() * FRAC_PI_2;
        }

        let mut phi = lat;
        let k = c_p * lat.sin();
        for _ in 0..MAX_ITER {
            let d = (phi + phi.sin() - k) / (1.0 + phi.cos());
            phi -= d;
            if d.abs() < LOOP_TOL {
                break;
            }
        }
        phi * 0.5
    }
}

impl ProjectionImpl for WagnerVProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg).clamp(-FRAC_PI_2, FRAC_PI_2);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let theta = Self::solve_theta(lat, self.c_p);
        let x = self.a * self.c_x * lon_rel * theta.cos() + self.fe;
        let y = self.a * self.c_y * theta.sin() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let theta_arg = ((y - self.fn_) / self.a / self.c_y).clamp(-1.0, 1.0);
        let theta = theta_arg.asin();
        let cos_theta = theta.cos();
        if cos_theta.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds(
                "coordinate outside Wagner V inverse domain",
            ));
        }

        let lon_rel = (x - self.fe) / (self.a * self.c_x * cos_theta);
        if lon_rel.abs() >= PI {
            return Err(ProjectionError::out_of_bounds(
                "coordinate outside Wagner V inverse domain",
            ));
        }

        let two_theta = 2.0 * theta;
        let lat = ((two_theta + two_theta.sin()) / self.c_p)
            .clamp(-1.0, 1.0)
            .asin();
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
