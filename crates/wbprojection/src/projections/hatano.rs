//! Hatano Asymmetrical Equal Area projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, PI};

const NITER: usize = 20;
const EPS: f64 = 1e-7;
const ONETOL: f64 = 1.000001;
const CN: f64 = 2.67595;
const CS: f64 = 2.43763;
const RCN: f64 = 0.37369906014686373063;
const RCS: f64 = 0.41023453108141924738;
const FYCN: f64 = 1.75859;
const FYCS: f64 = 1.93052;
const RYCN: f64 = 0.56863737426006061674;
const RYCS: f64 = 0.51799515156538134803;
const FXC: f64 = 0.85;
const RXC: f64 = 1.17647058823529411764;

pub(super) struct HatanoProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl HatanoProj {
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

impl ProjectionImpl for HatanoProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let mut phi = to_radians(lat_deg);
        let lam = Self::wrap_lon(lon - self.lon0);

        let c = phi.sin() * if phi < 0.0 { CS } else { CN };
        let mut i = NITER;
        while i > 0 {
            let th1 = (phi + phi.sin() - c) / (1.0 + phi.cos());
            phi -= th1;
            if th1.abs() < EPS {
                break;
            }
            i -= 1;
        }

        phi *= 0.5;
        let x = FXC * lam * phi.cos();
        let y = phi.sin() * if phi < 0.0 { FYCS } else { FYCN };
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut th = yn * if yn < 0.0 { RYCS } else { RYCN };
        if th.abs() > 1.0 {
            th = if th.abs() > ONETOL {
                th.signum()
            } else if th > 0.0 {
                FRAC_PI_2
            } else {
                -FRAC_PI_2
            };
        } else {
            th = th.asin();
        }

        let lam = RXC * xn / th.cos();
        let th2 = th + th;
        let mut phi = (th2 + th2.sin()) * if yn < 0.0 { RCS } else { RCN };
        if phi.abs() > 1.0 {
            phi = if phi.abs() > ONETOL {
                phi.signum()
            } else if phi > 0.0 {
                FRAC_PI_2
            } else {
                -FRAC_PI_2
            };
        } else {
            phi = phi.asin();
        }

        let lon = Self::wrap_lon(self.lon0 + lam);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
