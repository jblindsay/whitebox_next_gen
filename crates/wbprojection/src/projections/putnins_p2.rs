//! Putnins P2 projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const C_X: f64 = 1.89490;
const C_Y: f64 = 1.71848;
const C_P: f64 = 0.6141848493043784;
const EPS: f64 = 1e-10;
const NITER: usize = 10;
const PI_DIV_3: f64 = PI / 3.0;

pub(super) struct PutninsP2Proj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl PutninsP2Proj {
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

impl ProjectionImpl for PutninsP2Proj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let p = C_P * phi.sin();
        let phi2 = phi * phi;
        phi *= 0.615709 + phi2 * (0.00909953 + phi2 * 0.0046292);

        let mut i = NITER;
        while i > 0 {
            let c = phi.cos();
            let s = phi.sin();
            let v = (phi + s * (c - 1.0) - p) / (1.0 + c * (c - 1.0) - s * s);
            phi -= v;
            if v.abs() < EPS {
                break;
            }
            i -= 1;
        }

        if i == 0 {
            phi = if phi < 0.0 { -PI_DIV_3 } else { PI_DIV_3 };
        }

        let x = C_X * lon_rel * (phi.cos() - 0.5);
        let y = C_Y * phi.sin();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut phi = (yn / C_Y).clamp(-1.0, 1.0).asin();
        let c = phi.cos();
        let lon_rel = xn / (C_X * (c - 0.5));
        phi = ((phi + phi.sin() * (c - 1.0)) / C_P).clamp(-1.0, 1.0).asin();

        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
