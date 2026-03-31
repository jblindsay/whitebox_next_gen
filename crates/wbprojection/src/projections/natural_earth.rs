//! Natural Earth projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct NaturalEarthProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl NaturalEarthProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(NaturalEarthProj {
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

    fn l(phi: f64) -> f64 {
        let p2 = phi * phi;
        let p4 = p2 * p2;
        let p10 = p4 * p4 * p2;
        let p12 = p10 * p2;
        0.870700 - 0.131979 * p2 - 0.013791 * p4 + 0.003971 * p10 - 0.001529 * p12
    }

    fn d(phi: f64) -> f64 {
        let p2 = phi * phi;
        let p6 = p2 * p2 * p2;
        let p8 = p6 * p2;
        let p10 = p8 * p2;
        phi * (1.007226 + 0.015085 * p2 - 0.044475 * p6 + 0.028874 * p8 - 0.005916 * p10)
    }
}

impl ProjectionImpl for NaturalEarthProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let x = self.a * Self::l(lat) * lon_rel + self.fe;
        let y = self.a * Self::d(lat) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x_norm = (x - self.fe) / self.a;
        let y_norm = (y - self.fn_) / self.a;

        let mut lat = y_norm.clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
        for _ in 0..40 {
            let f = Self::d(lat) - y_norm;
            let h = 1e-7;
            let fp = (Self::d((lat + h).clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2)) - Self::d(lat)) / h;
            if fp.abs() < 1e-15 {
                break;
            }
            let dlat = -f / fp;
            lat = (lat + dlat).clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
            if dlat.abs() < 1e-13 {
                let l = Self::l(lat);
                if l.abs() < 1e-15 {
                    return Err(ProjectionError::out_of_bounds("Natural Earth inverse longitude undefined at pole"));
                }
                let lon_rel = x_norm / l;
                let lon = Self::wrap_lon(self.lon0 + lon_rel);
                return Ok((to_degrees(lon), to_degrees(lat)));
            }
        }

        Err(ProjectionError::ConvergenceFailure { iterations: 40 })
    }
}
