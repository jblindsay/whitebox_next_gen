//! Stereographic projection.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct StereographicProj {
    lon0: f64,
    a: f64,
    k0: f64,
    e: f64,
    fe: f64,
    fn_: f64,
    chi0: f64,
    sin_chi0: f64,
    cos_chi0: f64,
}

fn conformal_lat(e: f64, lat: f64) -> f64 {
    let esin = e * lat.sin();
    let t = (std::f64::consts::FRAC_PI_4 + 0.5 * lat).tan()
        * ((1.0 - esin) / (1.0 + esin)).powf(e / 2.0);
    2.0 * t.atan() - std::f64::consts::FRAC_PI_2
}

impl StereographicProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        let e = p.ellipsoid.e;
        let chi0 = conformal_lat(e, lat0);
        Ok(StereographicProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            k0: p.scale,
            e,
            fe: p.false_easting,
            fn_: p.false_northing,
            chi0,
            sin_chi0: chi0.sin(),
            cos_chi0: chi0.cos(),
        })
    }
}

impl ProjectionImpl for StereographicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let chi = conformal_lat(self.e, lat);
        let dlon = lon - self.lon0;
        let cos_dlon = dlon.cos();
        let sin_chi = chi.sin();
        let cos_chi = chi.cos();

        let a_k = 2.0 * self.k0 * self.a
            / (1.0 + self.sin_chi0 * sin_chi + self.cos_chi0 * cos_chi * cos_dlon);

        let x = a_k * cos_chi * dlon.sin() + self.fe;
        let y = a_k * (self.cos_chi0 * sin_chi - self.sin_chi0 * cos_chi * cos_dlon) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;
        let rho = (x * x + y * y).sqrt();
        let c = 2.0 * (rho / (2.0 * self.k0 * self.a)).atan();
        let cos_c = c.cos();
        let sin_c = c.sin();

        let chi = if rho < 1e-12 {
            self.chi0
        } else {
            (cos_c * self.sin_chi0 + y * sin_c * self.cos_chi0 / rho).asin()
        };

        let lon = self.lon0 + (x * sin_c)
            .atan2(rho * self.cos_chi0 * cos_c - y * self.sin_chi0 * sin_c);

        // Iterate conformal lat → geodetic lat
        let e = self.e;
        let mut phi = chi;
        for _ in 0..20 {
            let esin = e * phi.sin();
            let t = (std::f64::consts::FRAC_PI_4 + 0.5 * chi).tan()
                * ((1.0 + esin) / (1.0 - esin)).powf(e / 2.0);
            let phi_new = 2.0 * t.atan() - std::f64::consts::FRAC_PI_2;
            if (phi_new - phi).abs() < 1e-12 {
                phi = phi_new;
                break;
            }
            phi = phi_new;
        }

        Ok((to_degrees(lon), to_degrees(phi)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    const TOL_DEGREES: f64 = 1e-8;

    fn round_trip(lon: f64, lat: f64, lon0: f64, lat0: f64) {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::Stereographic)
                .with_lon0(lon0)
                .with_lat0(lat0),
        )
        .unwrap();
        let (x, y) = proj.forward(lon, lat).unwrap();
        let (lon2, lat2) = proj.inverse(x, y).unwrap();
        assert!((lon2 - lon).abs() < TOL_DEGREES, "lon: {lon} → {lon2}");
        assert!((lat2 - lat).abs() < TOL_DEGREES, "lat: {lat} → {lat2}");
    }

    #[test]
    fn round_trip_polar_north() {
        round_trip(0.0, 75.0, 0.0, 90.0);
        round_trip(45.0, 80.0, 0.0, 90.0);
    }

    #[test]
    fn round_trip_oblique() {
        round_trip(10.0, 45.0, 10.0, 45.0);
        round_trip(12.0, 47.0, 10.0, 45.0);
    }
}
