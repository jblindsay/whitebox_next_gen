//! Putnins P6' projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::PI;

const EPS: f64 = 1e-10;
const NITER: usize = 10;
const CON_POLE: f64 = 1.732050807568877;

const C_X: f64 = 0.44329;
const C_Y: f64 = 0.80404;
const A: f64 = 6.0;
const B: f64 = 5.61125;
const D: f64 = 3.0;

pub(super) struct PutninsP6pProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl PutninsP6pProj {
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

impl ProjectionImpl for PutninsP6pProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let p = B * phi.sin();
        phi *= 1.10265779;
        let mut i = NITER;
        while i > 0 {
            let r = (1.0 + phi * phi).sqrt();
            let v = ((A - r) * phi - (phi + r).ln() - p) / (A - 2.0 * r);
            phi -= v;
            if v.abs() < EPS {
                break;
            }
            i -= 1;
        }

        let sqrt_1_plus_phi2 = if i == 0 {
            phi = if p < 0.0 { -CON_POLE } else { CON_POLE };
            2.0
        } else {
            (1.0 + phi * phi).sqrt()
        };

        let x = C_X * lon_rel * (D - sqrt_1_plus_phi2);
        let y = C_Y * phi;
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut phi = yn / C_Y;
        let r = (1.0 + phi * phi).sqrt();
        let lon_rel = xn / (C_X * (D - r));
        phi = (((A - r) * phi - (phi + r).ln()) / B)
            .clamp(-1.0, 1.0)
            .asin();

        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
