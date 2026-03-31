//! Eckert V projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const XF: f64 = 0.441_012_771_724_551_5;
const RXF: f64 = 2.267_508_027_238_226_5;
const YF: f64 = 0.882_025_543_449_103;
const RYF: f64 = 1.133_754_013_619_113_2;

pub(super) struct EckertVProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertVProj {
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

impl ProjectionImpl for EckertVProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let x = self.a * XF * (1.0 + lat.cos()) * lon_rel + self.fe;
        let y = self.a * YF * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) * RYF / self.a;
        let denom = 1.0 + lat.cos();
        let lon_rel = if denom.abs() < 1e-15 {
            0.0
        } else {
            (x - self.fe) * RXF / (self.a * denom)
        };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
