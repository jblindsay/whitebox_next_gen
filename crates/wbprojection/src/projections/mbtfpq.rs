//! McBryde-Thomas Flat-Polar Quartic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const NITER: usize = 20;
const EPS: f64 = 1e-7;
const ONETOL: f64 = 1.000001;
const C: f64 = 1.70710678118654752440;
const RC: f64 = 0.58578643762690495119;
const FYC: f64 = 1.87475828462269495505;
const RYC: f64 = 0.53340209679417701685;
const FXC: f64 = 0.31245971410378249250;
const RXC: f64 = 3.20041258076506210122;

pub(super) struct MbtfpqProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl MbtfpqProj {
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

impl ProjectionImpl for MbtfpqProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let c = C * phi.sin();
        let mut i = NITER;
        while i > 0 {
            let th1 = ((0.5 * phi).sin() + phi.sin() - c)
                / (0.5 * (0.5 * phi).cos() + phi.cos());
            phi -= th1;
            if th1.abs() < EPS {
                break;
            }
            i -= 1;
        }

        let x = FXC * lam * (1.0 + 2.0 * phi.cos() / (0.5 * phi).cos());
        let y = FYC * (0.5 * phi).sin();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let t;
        let mut phi = RYC * yn;
        if phi.abs() > 1.0 {
            if phi.abs() > ONETOL {
                phi = phi.signum();
                t = phi;
            } else if phi < 0.0 {
                t = -1.0;
                phi = -PI;
            } else {
                t = 1.0;
                phi = PI;
            }
        } else {
            t = phi;
            phi = 2.0 * phi.asin();
        }

        let lam = RXC * xn / (1.0 + 2.0 * phi.cos() / (0.5 * phi).cos());
        let mut lat = RC * (t + phi.sin());
        if lat.abs() > 1.0 {
            lat = if lat.abs() > ONETOL {
                lat.signum()
            } else if lat < 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            };
        } else {
            lat = lat.asin();
        }

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
