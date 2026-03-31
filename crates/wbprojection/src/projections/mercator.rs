//! Mercator and Web Mercator projections.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

/// Standard Mercator cylindrical conformal projection.
pub(super) struct MercatorProj {
    lon0: f64,     // central longitude (radians)
    a: f64,        // semi-major axis
    e: f64,        // first eccentricity
    k0: f64,       // scale factor
    fe: f64,       // false easting
    fn_: f64,      // false northing
}

impl MercatorProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(MercatorProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            e: p.ellipsoid.e,
            k0: p.scale,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for MercatorProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);

        if lat.abs() >= std::f64::consts::FRAC_PI_2 {
            return Err(ProjectionError::out_of_bounds(
                "latitude ±90° is a singularity for Mercator",
            ));
        }

        let x = self.a * self.k0 * (lon - self.lon0) + self.fe;
        let y = if self.e < 1e-12 {
            // Spherical
            self.a * self.k0 * (std::f64::consts::FRAC_PI_4 + lat / 2.0).tan().ln() + self.fn_
        } else {
            // Ellipsoidal
            let e = self.e;
            let esin = e * lat.sin();
            let psi = (std::f64::consts::FRAC_PI_4 + lat / 2.0).tan()
                * ((1.0 - esin) / (1.0 + esin)).powf(e / 2.0);
            self.a * self.k0 * psi.ln() + self.fn_
        };
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lon = to_degrees((x - self.fe) / (self.a * self.k0) + self.lon0);
        let lat = if self.e < 1e-12 {
            // Spherical
            let t = (-(y - self.fn_) / (self.a * self.k0)).exp();
            to_degrees(std::f64::consts::FRAC_PI_2 - 2.0 * t.atan())
        } else {
            // Ellipsoidal: iterate
            let t = (-(y - self.fn_) / (self.a * self.k0)).exp();
            let e = self.e;
            let mut phi = std::f64::consts::FRAC_PI_2 - 2.0 * t.atan();
            for _ in 0..15 {
                let esin = e * phi.sin();
                let phi_new = std::f64::consts::FRAC_PI_2
                    - 2.0 * (t * ((1.0 - esin) / (1.0 + esin)).powf(e / 2.0)).atan();
                if (phi_new - phi).abs() < 1e-12 {
                    phi = phi_new;
                    break;
                }
                phi = phi_new;
            }
            to_degrees(phi)
        };
        Ok((lon, lat))
    }
}

/// Web Mercator (EPSG:3857) – spherical Mercator on WGS84 lat/lon.
pub(super) struct WebMercatorProj {
    a: f64,
}

impl WebMercatorProj {
    pub fn new(_p: &ProjectionParams) -> Result<Self> {
        Ok(WebMercatorProj { a: 6_378_137.0 })
    }
}

impl ProjectionImpl for WebMercatorProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        if lat_deg.abs() > 85.051_129 {
            return Err(ProjectionError::out_of_bounds(
                "Web Mercator only valid between ±85.05° latitude",
            ));
        }
        let lat = to_radians(lat_deg);
        let lon = to_radians(lon_deg);
        let x = self.a * lon;
        let y = self.a * (std::f64::consts::FRAC_PI_4 + lat / 2.0).tan().ln();
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lon = to_degrees(x / self.a);
        let lat = to_degrees(2.0 * (y / self.a).exp().atan() - std::f64::consts::FRAC_PI_2);
        Ok((lon, lat))
    }
}
