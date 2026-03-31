//! Winkel II projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const MAX_ITER: usize = 20;
const LOOP_TOL: f64 = 1e-10;

pub(super) struct WinkelIIProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
    cosphi1: f64,
}

impl WinkelIIProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat1 = 50.467;
        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            cosphi1: to_radians(lat1).cos(),
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

    fn solve_phi_aux(phi: f64) -> f64 {
        let k = PI * phi.sin();
        let mut t = 1.8 * phi;
        for _ in 0..MAX_ITER {
            let v = (t + t.sin() - k) / (1.0 + t.cos());
            t -= v;
            if v.abs() < 1e-7 {
                return 0.5 * t;
            }
        }
        if t < 0.0 {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        }
    }

    fn y_of_phi(phi: f64) -> f64 {
        let phi_aux = Self::solve_phi_aux(phi);
        FRAC_PI_4 * (phi_aux.sin() + 2.0 * phi / PI)
    }
}

impl ProjectionImpl for WinkelIIProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let phi = to_radians(lat_deg);
        let lon_rel = Self::wrap_lon(lon - self.lon0);

        let phi_aux = Self::solve_phi_aux(phi);
        let x = 0.5 * lon_rel * (phi_aux.cos() + self.cosphi1);
        let y = FRAC_PI_4 * (phi_aux.sin() + 2.0 * phi / PI);
        Ok((self.a * x + self.fe, self.a * y + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let xn = (x - self.fe) / self.a;
        let yn = (y - self.fn_) / self.a;

        let mut phi = yn.clamp(-FRAC_PI_2, FRAC_PI_2);
        for _ in 0..MAX_ITER {
            let f = Self::y_of_phi(phi) - yn;
            if f.abs() < LOOP_TOL {
                break;
            }
            let h = 1e-8;
            let fp = (Self::y_of_phi((phi + h).clamp(-FRAC_PI_2, FRAC_PI_2))
                - Self::y_of_phi((phi - h).clamp(-FRAC_PI_2, FRAC_PI_2)))
                / (2.0 * h);
            if fp.abs() < 1e-14 {
                break;
            }
            phi = (phi - f / fp).clamp(-FRAC_PI_2, FRAC_PI_2);
        }

        let phi_aux = Self::solve_phi_aux(phi);
        let denom = 0.5 * (phi_aux.cos() + self.cosphi1);
        let lon_rel = if denom.abs() < 1e-14 { 0.0 } else { xn / denom };
        let lon = Self::wrap_lon(self.lon0 + lon_rel);
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
