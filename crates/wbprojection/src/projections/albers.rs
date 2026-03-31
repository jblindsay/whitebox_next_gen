//! Albers Equal-Area Conic projection.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct AlbersProj {
    lon0: f64,
    a: f64,
    e2: f64,
    n: f64,
    c: f64,
    rho0: f64,
    fe: f64,
    fn_: f64,
}

fn alpha(e2: f64, lat: f64) -> f64 {
    let e = e2.sqrt();
    let sin_lat = lat.sin();
    let esin = e * sin_lat;
    (1.0 - e2) * (
        sin_lat / (1.0 - e2 * sin_lat * sin_lat)
        - (1.0 / (2.0 * e)) * ((1.0 - esin) / (1.0 + esin)).ln()
    )
}

fn m(e2: f64, lat: f64) -> f64 {
    let sin_lat = lat.sin();
    lat.cos() / (1.0 - e2 * sin_lat * sin_lat).sqrt()
}

impl AlbersProj {
    pub fn new(p: &ProjectionParams, lat1_deg: f64, lat2_deg: f64) -> Result<Self> {
        let lat1 = to_radians(lat1_deg);
        let lat2 = to_radians(lat2_deg);
        let lat0 = to_radians(p.lat0);
        let a = p.ellipsoid.a;
        let e2 = p.ellipsoid.e2;

        let m1 = m(e2, lat1);
        let m2 = m(e2, lat2);
        let a1 = alpha(e2, lat1);
        let a2 = alpha(e2, lat2);
        let a0 = alpha(e2, lat0);

        let n = (m1 * m1 - m2 * m2) / (a2 - a1);
        let c = m1 * m1 + n * a1;
        let rho0 = a * (c - n * a0).sqrt() / n;

        Ok(AlbersProj {
            lon0: to_radians(p.lon0),
            a,
            e2,
            n,
            c,
            rho0,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for AlbersProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);

        let a_val = alpha(self.e2, lat);
        let rho = self.a * (self.c - self.n * a_val).sqrt() / self.n;
        let theta = self.n * (lon - self.lon0);

        let x = rho * theta.sin() + self.fe;
        let y = self.rho0 - rho * theta.cos() + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let mut x = x - self.fe;
        let mut y = self.rho0 - (y - self.fn_);

        let mut rho = (x * x + y * y).sqrt();
        if rho != 0.0 && self.n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }

        let theta = x.atan2(y);
        let lon = theta / self.n + self.lon0;

        let q = (self.c - (rho * self.n / self.a).powi(2)) / self.n;
        let e2 = self.e2;
        let e = e2.sqrt();

        let phi = if e < 1e-15 {
            (q / 2.0).clamp(-1.0, 1.0).asin()
        } else {
            let mut phi = (q / 2.0).clamp(-1.0, 1.0).asin();
            let mut converged = false;
            for _ in 0..25 {
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();
                if cos_phi.abs() < 1e-15 {
                    break;
                }
                let esin = e * sin_phi;
                let one_minus = 1.0 - e2 * sin_phi * sin_phi;
                let phi_new = phi + one_minus * one_minus / (2.0 * cos_phi) * (
                    q / (1.0 - e2)
                    - sin_phi / one_minus
                    + (1.0 / (2.0 * e)) * ((1.0 - esin) / (1.0 + esin)).ln()
                );
                if (phi_new - phi).abs() < 1e-12 {
                    phi = phi_new;
                    converged = true;
                    break;
                }
                phi = phi_new;
            }
            if !converged {
                return Err(ProjectionError::ConvergenceFailure { iterations: 25 });
            }
            phi
        };

        Ok((to_degrees(lon), to_degrees(phi)))
    }
}
