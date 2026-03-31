//! Aitoff projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) struct AitoffProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl AitoffProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(AitoffProj {
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

    fn sinc(alpha: f64) -> f64 {
        if alpha.abs() < 1e-12 {
            1.0 - alpha * alpha / 6.0
        } else {
            alpha.sin() / alpha
        }
    }

    fn normalized_forward(lon_rel: f64, lat: f64) -> (f64, f64) {
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let half_lon = lon_rel * 0.5;
        let alpha = (cos_lat * half_lon.cos()).acos();
        let s = Self::sinc(alpha);

        let x = 2.0 * cos_lat * half_lon.sin() / s;
        let y = sin_lat / s;
        (x, y)
    }
}

impl ProjectionImpl for AitoffProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds(
                "latitude outside valid range [-90, 90]",
            ));
        }

        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let (x_norm, y_norm) = Self::normalized_forward(lon_rel, lat);
        Ok((self.a * x_norm + self.fe, self.a * y_norm + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x_target = (x - self.fe) / self.a;
        let y_target = (y - self.fn_) / self.a;

        let mut lat = y_target.clamp(-FRAC_PI_2, FRAC_PI_2);
        let mut lon_rel = (x_target * 0.5).clamp(-PI, PI);

        for _ in 0..50 {
            let (fx, fy) = Self::normalized_forward(lon_rel, lat);
            let r1 = fx - x_target;
            let r2 = fy - y_target;

            if r1.abs() < 1e-13 && r2.abs() < 1e-13 {
                let lon = Self::wrap_lon(self.lon0 + lon_rel);
                return Ok((to_degrees(lon), to_degrees(lat)));
            }

            let h = 1e-7;
            let lat_h = (lat + h).clamp(-FRAC_PI_2, FRAC_PI_2);
            let lon_h = Self::wrap_lon(lon_rel + h);

            let (fx_lon_h, fy_lon_h) = Self::normalized_forward(lon_h, lat);
            let (fx_lat_h, fy_lat_h) = Self::normalized_forward(lon_rel, lat_h);

            let j11 = (fx_lon_h - fx) / h;
            let j12 = (fx_lat_h - fx) / h;
            let j21 = (fy_lon_h - fy) / h;
            let j22 = (fy_lat_h - fy) / h;

            let det = j11 * j22 - j12 * j21;
            if det.abs() < 1e-16 {
                break;
            }

            let d_lon = (-r1 * j22 + r2 * j12) / det;
            let d_lat = (-j11 * r2 + j21 * r1) / det;

            lon_rel = Self::wrap_lon(lon_rel + d_lon.clamp(-0.5, 0.5));
            lat = (lat + d_lat.clamp(-0.5, 0.5)).clamp(-FRAC_PI_2, FRAC_PI_2);
        }

        Err(ProjectionError::ConvergenceFailure { iterations: 50 })
    }
}
