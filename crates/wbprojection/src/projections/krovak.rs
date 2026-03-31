//! Krovak projection (ellipsoidal).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

const EPS: f64 = 1e-15;
const UQ: f64 = 1.042_168_563_804_74; // 59°42'42.69689"
const S0: f64 = 1.370_083_462_815_55; // 78°30'00"
const MAX_ITER: usize = 100;

pub(super) struct KrovakProj {
    lon0: f64,
    a: f64,
    e: f64,
    alpha: f64,
    k: f64,
    n: f64,
    rho0: f64,
    ad: f64,
    fe: f64,
    fn_: f64,
}

impl KrovakProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        let e2 = p.ellipsoid.e2;
        let e = p.ellipsoid.e;

        let cos_lat0 = lat0.cos();
        let sin_lat0 = lat0.sin();

        let alpha = (1.0 + (e2 * cos_lat0.powi(4)) / (1.0 - e2)).sqrt();
        let u0 = (sin_lat0 / alpha).asin();
        let g = ((1.0 + e * sin_lat0) / (1.0 - e * sin_lat0)).powf(alpha * e / 2.0);

        let tan_half_lat0_plus_pi_4 = (lat0 / 2.0 + std::f64::consts::FRAC_PI_4).tan();
        if tan_half_lat0_plus_pi_4.abs() < 1e-18 {
            return Err(ProjectionError::invalid_param(
                "lat0",
                "lat0/2 + PI/4 must not approach 0",
            ));
        }

        let k = (u0 / 2.0 + std::f64::consts::FRAC_PI_4).tan()
            / tan_half_lat0_plus_pi_4.powf(alpha)
            * g;

        let n0 = (1.0 - e2).sqrt() / (1.0 - e2 * sin_lat0 * sin_lat0);
        let n = S0.sin();
        let rho0 = p.scale * n0 / S0.tan();
        let ad = std::f64::consts::FRAC_PI_2 - UQ;

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            e,
            alpha,
            k,
            n,
            rho0,
            ad,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for KrovakProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lam = lon - self.lon0;

        let gfi = ((1.0 + self.e * lat.sin()) / (1.0 - self.e * lat.sin()))
            .powf(self.alpha * self.e / 2.0);

        let u = 2.0
            * ((self.k * (lat / 2.0 + std::f64::consts::FRAC_PI_4).tan().powf(self.alpha) / gfi)
                .atan()
                - std::f64::consts::FRAC_PI_4);

        let deltav = -lam * self.alpha;

        let s = (self.ad.cos() * u.sin() + self.ad.sin() * u.cos() * deltav.cos()).asin();
        let cos_s = s.cos();
        if cos_s.abs() < 1e-12 {
            return Err(ProjectionError::SingularPoint(
                "Krovak forward singularity near cone pole".into(),
            ));
        }

        let d = (u.cos() * deltav.sin() / cos_s).asin();
        let eps = self.n * d;

        let rho = self.rho0 * (S0 / 2.0 + std::f64::consts::FRAC_PI_4).tan().powf(self.n)
            / (s / 2.0 + std::f64::consts::FRAC_PI_4).tan().powf(self.n);

        let southing = rho * eps.cos() * self.a;
        let westing = rho * eps.sin() * self.a;

        let x = -westing + self.fe;
        let y = -southing + self.fn_;

        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let southing = -(y - self.fn_);
        let westing = -(x - self.fe);

        let x_norm = southing / self.a;
        let y_norm = westing / self.a;

        let rho = (x_norm * x_norm + y_norm * y_norm).sqrt();
        let eps = y_norm.atan2(x_norm);

        let d = eps / S0.sin();
        let s = if rho == 0.0 {
            std::f64::consts::FRAC_PI_2
        } else {
            2.0
                * ((self.rho0 / rho).powf(1.0 / self.n)
                    * (S0 / 2.0 + std::f64::consts::FRAC_PI_4).tan())
                .atan()
                - std::f64::consts::FRAC_PI_4 * 2.0
        };

        let u = (self.ad.cos() * s.sin() - self.ad.sin() * s.cos() * d.cos()).asin();
        let deltav = (s.cos() * d.sin() / u.cos()).asin();

        let lon = self.lon0 - deltav / self.alpha;

        let mut fi1 = u;
        let inv_alpha = 1.0 / self.alpha;

        for _ in 0..MAX_ITER {
            let ratio = ((1.0 + self.e * fi1.sin()) / (1.0 - self.e * fi1.sin())).powf(self.e / 2.0);
            let lat = 2.0
                * ((self.k.powf(-inv_alpha)
                    * (u / 2.0 + std::f64::consts::FRAC_PI_4).tan().powf(inv_alpha)
                    * ratio)
                    .atan()
                    - std::f64::consts::FRAC_PI_4);

            if (fi1 - lat).abs() < EPS {
                return Ok((to_degrees(lon), to_degrees(lat)));
            }
            fi1 = lat;
        }

        Err(ProjectionError::ConvergenceFailure {
            iterations: MAX_ITER,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ellipsoid::Ellipsoid;
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    fn krovak_proj() -> Projection {
        Projection::new(
            ProjectionParams::new(ProjectionKind::Krovak)
                .with_lat0(49.5)
                .with_lon0(7.166_666_666_666_667)
                .with_scale(0.9999)
                .with_false_easting(0.0)
                .with_false_northing(0.0)
                .with_ellipsoid(Ellipsoid::BESSEL),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_prague() {
        let proj = krovak_proj();
        let (x, y) = proj.forward(14.42, 50.09).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - 14.42).abs() < 1e-7);
        assert!((lat - 50.09).abs() < 1e-7);
    }

    #[test]
    fn round_trip_brno() {
        let proj = krovak_proj();
        let (x, y) = proj.forward(16.61, 49.20).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - 16.61).abs() < 1e-7);
        assert!((lat - 49.20).abs() < 1e-7);
    }
}
