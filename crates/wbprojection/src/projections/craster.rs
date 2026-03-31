//! Craster Parabolic (Putnins P4) projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const XM: f64 = 0.977_205_023_805_839_8;
const RXM: f64 = 1.023_326_707_946_488_5;
const YM: f64 = 3.069_980_123_839_465_5;
const RYM: f64 = 0.325_735_007_935_279_95;
const THIRD: f64 = 1.0 / 3.0;

pub(super) struct CrasterProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl CrasterProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(CrasterProj {
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

impl ProjectionImpl for CrasterProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg).clamp(-FRAC_PI_2, FRAC_PI_2);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let phi = lat * THIRD;
        let x = self.a * XM * lon_rel * (2.0 * (2.0 * phi).cos() - 1.0) + self.fe;
        let y = self.a * YM * phi.sin() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let lat = (3.0 * (yn * RYM).clamp(-1.0, 1.0).asin()).clamp(-FRAC_PI_2, FRAC_PI_2);
        let denom = 2.0 * (2.0 * lat * THIRD).cos() - 1.0;
        let lon_rel = if denom.abs() < 1e-15 { 0.0 } else { xn * RXM / denom };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
