//! American Polyconic projection (spherical form).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

pub(super) struct PolyconicProj {
    lon0: f64,
    lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl PolyconicProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(PolyconicProj {
            lon0: to_radians(p.lon0),
            lat0: to_radians(p.lat0),
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

    fn normalized_forward(lon_rel: f64, lat: f64, lat0: f64) -> (f64, f64) {
        if lat.abs() < 1e-12 {
            return (lon_rel, -lat0);
        }
        let cot_lat = 1.0 / lat.tan();
        let e = lon_rel * lat.sin();
        let x = cot_lat * e.sin();
        let y = lat - lat0 + cot_lat * (1.0 - e.cos());
        (x, y)
    }
}

impl ProjectionImpl for PolyconicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);
        let (x, y) = Self::normalized_forward(lon_rel, lat, self.lat0);
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x_target = (x - self.fe) / self.a;
        let y_target = (y - self.fn_) / self.a;

        if (y_target + self.lat0).abs() < 1e-12 {
            let lon = Self::wrap_lon(self.lon0 + x_target);
            return Ok((to_degrees(lon), 0.0));
        }

        let mut lat = (y_target + self.lat0).clamp(-std::f64::consts::FRAC_PI_2 + 1e-8, std::f64::consts::FRAC_PI_2 - 1e-8);

        for _ in 0..80 {
            let sin_lat = lat.sin();
            let cos_lat = lat.cos();
            if sin_lat.abs() < 1e-14 || cos_lat.abs() < 1e-14 {
                break;
            }

            let t = (x_target * sin_lat / cos_lat).clamp(-1.0, 1.0);
            let e = t.asin();
            let f = lat - self.lat0 + (cos_lat / sin_lat) * (1.0 - e.cos()) - y_target;

            if f.abs() < 1e-13 {
                let lon_rel = e / sin_lat;
                let lon = Self::wrap_lon(self.lon0 + lon_rel);
                return Ok((to_degrees(lon), to_degrees(lat)));
            }

            let h = 1e-7;
            let lat_h = (lat + h).clamp(-std::f64::consts::FRAC_PI_2 + 1e-8, std::f64::consts::FRAC_PI_2 - 1e-8);
            let sin_h = lat_h.sin();
            let cos_h = lat_h.cos();
            let t_h = (x_target * sin_h / cos_h).clamp(-1.0, 1.0);
            let e_h = t_h.asin();
            let f_h = lat_h - self.lat0 + (cos_h / sin_h) * (1.0 - e_h.cos()) - y_target;
            let fp = (f_h - f) / h;
            if fp.abs() < 1e-15 {
                break;
            }
            let dlat = -f / fp;
            lat = (lat + dlat.clamp(-0.25, 0.25)).clamp(-std::f64::consts::FRAC_PI_2 + 1e-8, std::f64::consts::FRAC_PI_2 - 1e-8);
        }

        Err(ProjectionError::ConvergenceFailure { iterations: 80 })
    }
}
