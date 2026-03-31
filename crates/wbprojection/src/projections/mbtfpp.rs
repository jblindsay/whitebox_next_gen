//! McBryde-Thomas Flat-Polar Parabolic projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const CSY: f64 = 0.95257934441568037152;
const FXC: f64 = 0.92582009977255146156;
const FYC: f64 = 3.40168025708304504493;
const C23: f64 = 2.0 / 3.0;
const C13: f64 = 1.0 / 3.0;
const ONEEPS: f64 = 1.0000001;

pub(super) struct MbtfppProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl MbtfppProj {
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

impl ProjectionImpl for MbtfppProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        phi = (CSY * phi.sin()).clamp(-1.0, 1.0).asin();
        let x = FXC * lam * (2.0 * (C23 * phi).cos() - 1.0);
        let y = FYC * (C13 * phi).sin();
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut phi = yn / FYC;
        if phi.abs() >= 1.0 {
            phi = if phi.abs() > ONEEPS {
                phi.signum()
            } else if phi < 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            };
        } else {
            phi = phi.asin();
        }

        phi *= 3.0;
        let lam = xn / (FXC * (2.0 * (C23 * phi).cos() - 1.0));
        let mut lat = phi.sin() / CSY;
        if lat.abs() >= 1.0 {
            lat = if lat.abs() > ONEEPS {
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
