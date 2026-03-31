//! Eckert III projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const C_X: f64 = 0.422_238_200_315_771_2;
const C_Y: f64 = 0.844_476_400_631_542_4;
const A: f64 = 1.0;
const B: f64 = 0.405_284_734_569_351_1;

pub(super) struct EckertIiiProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertIiiProj {
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

impl ProjectionImpl for EckertIiiProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let y = self.a * C_Y * lat + self.fn_;
        let x = self.a * C_X * lon_rel * (A + (1.0 - B * lat * lat).sqrt()) + self.fe;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lat = (y - self.fn_) / (self.a * C_Y);
        let denom = C_X * (A + (1.0 - B * lat * lat).sqrt());
        let lon_rel = if denom.abs() < 1e-15 {
            0.0
        } else {
            (x - self.fe) / (self.a * denom)
        };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
