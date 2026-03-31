//! Gnomonic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

pub(super) struct GnomonicProj {
    lon0: f64,
    lat0: f64,
    sin_lat0: f64,
    cos_lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl GnomonicProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        Ok(GnomonicProj {
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

impl ProjectionImpl for GnomonicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let dlon = lon - self.lon0;

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let cos_dlon = dlon.cos();
        let sin_dlon = dlon.sin();

        let cos_c = self.sin_lat0 * sin_lat + self.cos_lat0 * cos_lat * cos_dlon;
        if cos_c <= 1e-15 {
            return Err(ProjectionError::out_of_bounds(
                "point is on or beyond the gnomonic horizon",
            ));
        }

        let x = self.a * (cos_lat * sin_dlon) / cos_c + self.fe;
        let y = self.a * (self.cos_lat0 * sin_lat - self.sin_lat0 * cos_lat * cos_dlon) / cos_c + self.fn_;

        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;
        let rho = (x * x + y * y).sqrt();

        if rho < 1e-15 {
            return Ok((to_degrees(self.lon0), to_degrees(self.lat0)));
        }

        let c = (rho / self.a).atan();
        let sin_c = c.sin();
        let cos_c = c.cos();

        let lat = (cos_c * self.sin_lat0 + (y * sin_c * self.cos_lat0) / rho).asin();
        let lon = self.lon0
            + (x * sin_c).atan2(rho * self.cos_lat0 * cos_c - y * self.sin_lat0 * sin_c);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
