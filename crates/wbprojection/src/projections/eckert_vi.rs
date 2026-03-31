//! Eckert VI projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) struct EckertViProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertViProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(EckertViProj {
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

impl ProjectionImpl for EckertViProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds("latitude outside valid range [-90, 90]"));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let k = 1.0 + PI / 2.0;
        let mut theta = lat;
        for _ in 0..40 {
            let f = theta + theta.sin() - k * lat.sin();
            let fp = 1.0 + theta.cos();
            if fp.abs() < 1e-15 {
                break;
            }
            let d = -f / fp;
            theta += d;
            if d.abs() < 1e-13 {
                break;
            }
        }

        let c = (2.0 + PI).sqrt();
        let x = self.a * lon_rel * (1.0 + theta.cos()) / c + self.fe;
        let y = self.a * (2.0 * theta) / c + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let c = (2.0 + PI).sqrt();
        let theta = (y - self.fn_) * c / (2.0 * self.a);
        let k = 1.0 + PI / 2.0;
        let sin_lat = ((theta + theta.sin()) / k).clamp(-1.0, 1.0);
        let lat = sin_lat.asin();

        let denom = 1.0 + theta.cos();
        let lon_rel = if denom.abs() < 1e-15 { 0.0 } else { (x - self.fe) * c / (self.a * denom) };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
