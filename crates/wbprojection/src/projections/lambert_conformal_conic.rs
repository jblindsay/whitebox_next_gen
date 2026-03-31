//! Lambert Conformal Conic projection (1SP and 2SP variants).

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

pub(super) struct LccProj {
    lon0: f64,
    a: f64,
    e: f64,
    n: f64,   // cone constant
    f: f64,   // series constant
    rho0: f64, // ρ at lat0
    fe: f64,
    fn_: f64,
}

/// Compute t for conformal latitude
fn lcc_t(e: f64, lat: f64) -> f64 {
    let esin = e * lat.sin();
    ((FRAC_PI_4 - lat / 2.0).tan()) * ((1.0 + esin) / (1.0 - esin)).powf(e / 2.0)
}

impl LccProj {
    pub fn new(p: &ProjectionParams, lat1_deg: f64, lat2_deg: Option<f64>) -> Result<Self> {
        let lat1 = to_radians(lat1_deg);
        let lat2 = lat2_deg.map(to_radians).unwrap_or(lat1);
        let lat0 = to_radians(p.lat0);
        let a = p.ellipsoid.a;
        let e = p.ellipsoid.e;
        let e2 = p.ellipsoid.e2;

        // Compute m for a given latitude
        let m_fn = |lat: f64| {
            let sin_lat = lat.sin();
            lat.cos() / (1.0 - e2 * sin_lat * sin_lat).sqrt()
        };

        let m1 = m_fn(lat1);
        let m2 = m_fn(lat2);
        let t1 = lcc_t(e, lat1);
        let t2 = lcc_t(e, lat2);
        let t0 = lcc_t(e, lat0);

        let n = if (lat1 - lat2).abs() < 1e-10 {
            lat1.sin()
        } else {
            (m1.ln() - m2.ln()) / (t1.ln() - t2.ln())
        };

        let f = m1 / (n * t1.powf(n));
        let rho0 = a * f * t0.powf(n);

        Ok(LccProj {
            lon0: to_radians(p.lon0),
            a,
            e,
            n,
            f,
            rho0,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for LccProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);

        if lat.abs() > FRAC_PI_2 + 1e-10 {
            return Err(ProjectionError::out_of_bounds("latitude out of range"));
        }

        let t = lcc_t(self.e, lat);
        let rho = self.a * self.f * t.powf(self.n);
        let theta = self.n * (lon - self.lon0);

        let x = rho * theta.sin() + self.fe;
        let y = self.rho0 - rho * theta.cos() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = self.rho0 - (y - self.fn_);

        let rho = (x * x + y * y).sqrt() * self.n.signum();
        let theta = x.atan2(y);

        let t = (rho / (self.a * self.f)).powf(1.0 / self.n);

        // Iterate to find latitude
        let mut phi = FRAC_PI_2 - 2.0 * t.atan();
        let e = self.e;
        for _ in 0..20 {
            let esin = e * phi.sin();
            let phi_new = FRAC_PI_2
                - 2.0 * (t * ((1.0 - esin) / (1.0 + esin)).powf(e / 2.0)).atan();
            if (phi_new - phi).abs() < 1e-12 {
                phi = phi_new;
                break;
            }
            phi = phi_new;
        }

        let lon = theta / self.n + self.lon0;
        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
