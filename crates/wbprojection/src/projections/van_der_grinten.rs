//! Van der Grinten I projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) struct VanDerGrintenProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl VanDerGrintenProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(VanDerGrintenProj {
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

    fn normalized_forward(lon_rel: f64, lat: f64) -> Result<(f64, f64)> {
        let eps = 1e-14;
        if !(-FRAC_PI_2..=FRAC_PI_2).contains(&lat) {
            return Err(ProjectionError::out_of_bounds(
                "latitude outside valid range [-90, 90]",
            ));
        }

        if lat.abs() < eps {
            return Ok((lon_rel, 0.0));
        }

        let abs_lat = lat.abs();
        let abs_lon = lon_rel.abs();

        let theta = ((2.0 * abs_lat / PI).clamp(0.0, 1.0)).asin();
        let sin_t = theta.sin();

        if abs_lon < eps || (abs_lat - FRAC_PI_2).abs() < eps {
            let y = lat.signum() * PI * (0.5 * theta).tan();
            return Ok((0.0, y));
        }

        if sin_t.abs() < eps {
            return Ok((lon_rel, 0.0));
        }

        let cos_t = theta.cos();
        let a_term = 0.5 * ((PI / abs_lon) - (abs_lon / PI)).abs();
        let g = cos_t / (sin_t + cos_t - 1.0);
        let p = g * (2.0 / sin_t - 1.0);
        let q = a_term * a_term + g;

        let p2 = p * p;
        let a2 = a_term * a_term;
        let den = p2 + a2;

        let rad_x = a2 * (g - p2) * (g - p2) - (p2 + a2) * (g * g - p2);
        let sqrt_x = rad_x.max(0.0).sqrt();
        let x = lon_rel.signum() * PI * (a_term * (g - p2) + sqrt_x) / den;

        let rad_y = (a2 + 1.0) * den - q * q;
        let sqrt_y = rad_y.max(0.0).sqrt();
        let y = lat.signum() * PI * (p * q - a_term * sqrt_y) / den;

        Ok((x, y))
    }
}

impl ProjectionImpl for VanDerGrintenProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let (x_norm, y_norm) = Self::normalized_forward(lon_rel, lat)?;
        Ok((self.a * x_norm + self.fe, self.a * y_norm + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x_target = (x - self.fe) / self.a;
        let y_target = (y - self.fn_) / self.a;

        let rho = (x_target * x_target + y_target * y_target).sqrt();
        if rho > PI + 1e-10 {
            return Err(ProjectionError::out_of_bounds(
                "point outside Van der Grinten valid world disk",
            ));
        }

        let mut lat = y_target.clamp(-FRAC_PI_2, FRAC_PI_2);
        let mut lon_rel = x_target.clamp(-PI, PI);

        for _ in 0..60 {
            let (fx, fy) = Self::normalized_forward(lon_rel, lat)?;
            let r1 = fx - x_target;
            let r2 = fy - y_target;

            if r1.abs() < 1e-13 && r2.abs() < 1e-13 {
                let lon = Self::wrap_lon(self.lon0 + lon_rel);
                return Ok((to_degrees(lon), to_degrees(lat)));
            }

            let h = 1e-7;
            let lat_h = (lat + h).clamp(-FRAC_PI_2, FRAC_PI_2);
            let lon_h = Self::wrap_lon(lon_rel + h);

            let (fx_lon_h, fy_lon_h) = Self::normalized_forward(lon_h, lat)?;
            let (fx_lat_h, fy_lat_h) = Self::normalized_forward(lon_rel, lat_h)?;

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

        Err(ProjectionError::ConvergenceFailure { iterations: 60 })
    }
}
