//! Equirectangular (Plate Carrée) projection.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct EquirectangularProj {
    lon0: f64,
    lat0: f64,
    a: f64,
    cos_lat_ts: f64,
    fe: f64,
    fn_: f64,
}

impl EquirectangularProj {
    pub fn new(p: &ProjectionParams, lat_ts_deg: f64) -> Result<Self> {
        Ok(EquirectangularProj {
            lon0: to_radians(p.lon0),
            lat0: to_radians(p.lat0),
            a: p.ellipsoid.a,
            cos_lat_ts: to_radians(lat_ts_deg).cos(),
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for EquirectangularProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let x = self.a * (lon - self.lon0) * self.cos_lat_ts + self.fe;
        let y = self.a * (lat - self.lat0) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / self.a + self.lat0;
        let lon = if self.cos_lat_ts.abs() < 1e-12 {
            self.lon0
        } else {
            (x - self.fe) / (self.a * self.cos_lat_ts) + self.lon0
        };
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
