//! Orthographic projection – globe view from infinity.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct OrthographicProj {
    lon0: f64,
    lat0: f64,
    sin_lat0: f64,
    cos_lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl OrthographicProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        Ok(OrthographicProj {
            lon0: to_radians(p.lon0),
            lat0,
            sin_lat0: lat0.sin(),
            cos_lat0: lat0.cos(),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for OrthographicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let dlon = lon - self.lon0;

        // Check if on visible hemisphere
        let cos_c = self.sin_lat0 * lat.sin() + self.cos_lat0 * lat.cos() * dlon.cos();
        if cos_c < 0.0 {
            return Err(ProjectionError::out_of_bounds(
                "point is on the far side of the globe",
            ));
        }

        let x = self.a * lat.cos() * dlon.sin() + self.fe;
        let y = self.a * (self.cos_lat0 * lat.sin() - self.sin_lat0 * lat.cos() * dlon.cos()) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;
        let rho = (x * x + y * y).sqrt();

        if rho > self.a {
            return Err(ProjectionError::out_of_bounds("point outside orthographic projection bounds"));
        }

        let c = (rho / self.a).asin();
        let cos_c = c.cos();
        let sin_c = c.sin();

        let lat = if rho < 1e-12 {
            self.lat0
        } else {
            (cos_c * self.sin_lat0 + y * sin_c * self.cos_lat0 / rho).asin()
        };

        let lon = if rho < 1e-12 {
            self.lon0
        } else {
            self.lon0 + (x * sin_c)
                .atan2(rho * self.cos_lat0 * cos_c - y * self.sin_lat0 * sin_c)
        };

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
