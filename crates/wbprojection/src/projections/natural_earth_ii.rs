//! Natural Earth II projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const A0: f64 = 0.84719;
const A1: f64 = -0.13063;
const A2: f64 = -0.04515;
const A3: f64 = 0.05494;
const A4: f64 = -0.02326;
const A5: f64 = 0.00331;
const B0: f64 = 1.01183;
const B1: f64 = -0.02625;
const B2: f64 = 0.01926;
const B3: f64 = -0.00396;
const C0: f64 = B0;
const C1: f64 = 9.0 * B1;
const C2: f64 = 11.0 * B2;
const C3: f64 = 13.0 * B3;
const EPS: f64 = 1e-11;
const MAX_Y: f64 = 0.84719 * 0.535117535153096 * PI;

pub(super) struct NaturalEarthIIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl NaturalEarthIIProj {
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

impl ProjectionImpl for NaturalEarthIIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let phi2 = phi * phi;
        let phi4 = phi2 * phi2;
        let phi6 = phi2 * phi4;

        let x = lam * (A0 + A1 * phi2 + phi6 * phi6 * (A2 + A3 * phi2 + A4 * phi4 + A5 * phi6));
        let y = phi * (B0 + phi4 * phi4 * (B1 + B2 * phi2 + B3 * phi4));
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let mut yn = (y - self.fn_) / self.a;

        yn = yn.clamp(-MAX_Y, MAX_Y);

        let mut yc = yn;
        for _ in 0..100 {
            let y2 = yc * yc;
            let y4 = y2 * y2;
            let f = yc * (B0 + y4 * y4 * (B1 + B2 * y2 + B3 * y4)) - yn;
            let fder = C0 + y4 * y4 * (C1 + C2 * y2 + C3 * y4);
            let tol = f / fder;
            yc -= tol;
            if tol.abs() < EPS {
                break;
            }
        }

        let y2 = yc * yc;
        let y4 = y2 * y2;
        let y6 = y2 * y4;
        let lon_rel = xn / (A0 + A1 * y2 + y6 * y6 * (A2 + A3 * y2 + A4 * y4 + A5 * y6));
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(yc)))
    }
}
