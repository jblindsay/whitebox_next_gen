//! Putnins P5 projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const C: f64 = 1.013_46;
const D: f64 = 1.215_854_2;
const A: f64 = 2.0;
const B: f64 = 1.0;

pub(super) struct PutninsP5Proj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl PutninsP5Proj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(Self {
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

impl ProjectionImpl for PutninsP5Proj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let k = A - B * (1.0 + D * lat * lat).sqrt();
        let x = self.a * C * lon_rel * k + self.fe;
        let y = self.a * C * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / (self.a * C);
        let denom = C * (A - B * (1.0 + D * lat * lat).sqrt());
        let lon_rel = if denom.abs() < 1e-15 { 0.0 } else { (x - self.fe) / (self.a * denom) };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
