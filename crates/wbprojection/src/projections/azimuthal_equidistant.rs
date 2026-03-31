//! Azimuthal Equidistant projection.
//! Distances and directions are correct from the center point.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct AzimuthalEquidistantProj {
    lon0: f64,
    lat0: f64,
    sin_lat0: f64,
    cos_lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl AzimuthalEquidistantProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        Ok(AzimuthalEquidistantProj {
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

impl ProjectionImpl for AzimuthalEquidistantProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);

        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let dlon = lon - self.lon0;

        let cos_c = self.sin_lat0 * sin_lat + self.cos_lat0 * cos_lat * dlon.cos();
        let c = cos_c.acos();

        if c.abs() < 1e-12 {
            return Ok((self.fe, self.fn_));
        }

        let k = if c.sin().abs() < 1e-12 {
            1.0
        } else {
            c / c.sin()
        };

        let x = self.a * k * cos_lat * dlon.sin() + self.fe;
        let y = self.a * k * (self.cos_lat0 * sin_lat - self.sin_lat0 * cos_lat * dlon.cos()) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;

        let rho = (x * x + y * y).sqrt();
        let c = rho / self.a;

        if c < 1e-12 {
            return Ok((to_degrees(self.lon0), to_degrees(self.lat0)));
        }

        let lat = (c.cos() * self.sin_lat0 + y * c.sin() * self.cos_lat0 / rho).asin();
        let lon = self.lon0 + (x * c.sin())
            .atan2(rho * self.cos_lat0 * c.cos() - y * self.sin_lat0 * c.sin());

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
