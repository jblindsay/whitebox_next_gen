//! Eckert I projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const FC: f64 = 0.921_317_731_923_561_3;
const RP: f64 = 0.318_309_886_183_790_7;

pub(super) struct EckertIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl EckertIProj {
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

impl ProjectionImpl for EckertIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let x = self.a * FC * lon_rel * (1.0 - RP * lat.abs()) + self.fe;
        let y = self.a * FC * lat + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let yn = (y - self.fn_) / self.a;
        let lat = yn / FC;
        let denom = FC * (1.0 - RP * lat.abs());
        let lon_rel = if denom.abs() < 1e-15 {
            0.0
        } else {
            (x - self.fe) / (self.a * denom)
        };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
