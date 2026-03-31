//! Polar Stereographic projection (ellipsoidal).
//!
//! Handles three ESRI WKT variants:
//! - **Variant A** (`Polar_Stereographic_Variant_A`, EPSG method 9810):
//!   scale factor k0 at the pole.
//! - **Variant B / North-South Pole** (`Stereographic_North_Pole` /
//!   `Stereographic_South_Pole`): standard parallel φ_ts where scale = 1.
//!
//! Both variants reduce to the same forward/inverse kernel once `rho_coeff` is
//! precomputed.

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

pub(super) struct PolarStereographicProj {
    /// true = North Pole origin, false = South Pole origin.
    north: bool,
    lon0: f64,
    fe: f64,
    fn_: f64,
    e: f64,
    /// ρ = rho_coeff × t(φ).
    rho_coeff: f64,
}

/// t-factor at latitude φ for the north-pole convention.
/// Uses the identity t_south(φ) = t_north(-φ), so we always call
/// t_north with |φ| internally.
fn t_north(e: f64, phi: f64) -> f64 {
    let esin = e * phi.sin();
    (std::f64::consts::FRAC_PI_4 - phi / 2.0).tan()
        * ((1.0 + esin) / (1.0 - esin)).powf(e / 2.0)
}

/// m-factor: cos(φ) / sqrt(1 - e² sin²φ), used when deriving k0 from lat_ts.
fn m_factor(e: f64, phi: f64) -> f64 {
    let esin = e * phi.sin();
    phi.cos() / (1.0 - esin * esin).sqrt()
}

impl PolarStereographicProj {
    /// `lat_ts_deg` — standard parallel in degrees (where scale = 1).
    /// When `None`, `params.scale` is treated as the scale factor at the pole.
    pub fn new(p: &ProjectionParams, north: bool, lat_ts_deg: Option<f64>) -> Result<Self> {
        let e = p.ellipsoid.e;
        let a = p.ellipsoid.a;
        let lon0 = to_radians(p.lon0);

        let rho_coeff = match lat_ts_deg {
            None => {
                // Variant A: ρ = 2 a k0 t / C
                // where C = sqrt( (1+e)^(1+e) × (1-e)^(1-e) )
                let k0 = p.scale;
                let c = ((1.0 + e).powf(1.0 + e) * (1.0 - e).powf(1.0 - e)).sqrt();
                2.0 * a * k0 / c
            }
            Some(lat_ts_deg) => {
                // Variant B/C: ρ = a m_ts t / t_ts
                // Use absolute value: same formula for north and south
                let phi_ts = to_radians(lat_ts_deg.abs());
                let m_ts = m_factor(e, phi_ts);
                let t_ts = t_north(e, phi_ts); // t_north(|lat|) == t_south(-|lat|)
                if t_ts.abs() < 1e-15 {
                    // Standard parallel at pole — degenerate; default to zero scale
                    0.0
                } else {
                    a * m_ts / t_ts
                }
            }
        };

        Ok(PolarStereographicProj {
            north,
            lon0,
            fe: p.false_easting,
            fn_: p.false_northing,
            e,
            rho_coeff,
        })
    }
}

impl ProjectionImpl for PolarStereographicProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let dlon = lon - self.lon0;

        let t = if self.north {
            t_north(self.e, lat)
        } else {
            t_north(self.e, -lat) // t_south(lat) = t_north(-lat)
        };
        let rho = self.rho_coeff * t;

        let e = self.fe + rho * dlon.sin();
        let n = if self.north {
            self.fn_ - rho * dlon.cos()
        } else {
            self.fn_ + rho * dlon.cos()
        };
        Ok((e, n))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let dx = x - self.fe;
        let dy = y - self.fn_;
        let rho = (dx * dx + dy * dy).sqrt();

        if rho < 1e-10 {
            // At the pole itself
            let lat_deg = if self.north { 90.0 } else { -90.0 };
            return Ok((to_degrees(self.lon0), lat_deg));
        }

        let t = rho / self.rho_coeff;

        // Recover longitude
        let lon = if self.north {
            self.lon0 + dx.atan2(-dy)
        } else {
            self.lon0 + dx.atan2(dy)
        };

        // Recover latitude iteratively from t (Snyder eq 7-9)
        let e = self.e;
        // Always iterate using the north-pole form (positive phi), then negate
        // for south-pole projections.  If we iterated with a negative phi the
        // eccentricity correction factor would invert and the series diverges.
        let mut phi = std::f64::consts::FRAC_PI_2 - 2.0 * t.atan();
        for _ in 0..20 {
            let esin = e * phi.sin();
            let phi_new = std::f64::consts::FRAC_PI_2
                - 2.0 * (t * ((1.0 - esin) / (1.0 + esin)).powf(e / 2.0)).atan();
            if (phi_new - phi).abs() < 1e-12 {
                phi = phi_new;
                break;
            }
            phi = phi_new;
        }
        if !self.north {
            phi = -phi;
        }

        Ok((to_degrees(lon), to_degrees(phi)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};
    use crate::ellipsoid::Ellipsoid;

    const TOL_M: f64 = 1e-3; // 1 mm
    const TOL_DEG: f64 = 1e-8;

    /// UPS North round-trip (EPSG:5041 parameters)
    #[test]
    fn ups_north_round_trip() {
        let p = ProjectionParams {
            kind: ProjectionKind::PolarStereographic { north: true, lat_ts: None },
            lon0: 0.0,
            lat0: 90.0,
            false_easting: 2_000_000.0,
            false_northing: 2_000_000.0,
            scale: 0.994,
            ellipsoid: Ellipsoid::WGS84,
            ..Default::default()
        };
        let proj = Projection::new(p).unwrap();
        for &(lon, lat) in &[(0.0_f64, 85.0_f64), (90.0, 80.0), (-45.0, 75.0)] {
            let (x, y) = proj.forward(lon, lat).unwrap();
            let (lon2, lat2) = proj.inverse(x, y).unwrap();
            assert!((lon2 - lon).abs() < TOL_DEG, "lon round-trip fail: {lon} → {lon2}");
            assert!((lat2 - lat).abs() < TOL_DEG, "lat round-trip fail: {lat} → {lat2}");
        }
    }

    /// UPS South round-trip (EPSG:5042 parameters)
    #[test]
    fn ups_south_round_trip() {
        let p = ProjectionParams {
            kind: ProjectionKind::PolarStereographic { north: false, lat_ts: None },
            lon0: 0.0,
            lat0: -90.0,
            false_easting: 2_000_000.0,
            false_northing: 2_000_000.0,
            scale: 0.994,
            ellipsoid: Ellipsoid::WGS84,
            ..Default::default()
        };
        let proj = Projection::new(p).unwrap();
        for &(lon, lat) in &[(0.0_f64, -85.0_f64), (90.0, -80.0), (-45.0, -75.0)] {
            let (x, y) = proj.forward(lon, lat).unwrap();
            let (lon2, lat2) = proj.inverse(x, y).unwrap();
            assert!((lon2 - lon).abs() < TOL_DEG, "lon round-trip fail: {lon} → {lon2}");
            assert!((lat2 - lat).abs() < TOL_DEG, "lat round-trip fail: {lat} → {lat2}");
        }
    }

    /// SCAR-style south-pole stereographic with standard parallel
    #[test]
    fn scar_south_pole_round_trip() {
        let p = ProjectionParams {
            kind: ProjectionKind::PolarStereographic { north: false, lat_ts: Some(-80.0) },
            lon0: -165.0,
            lat0: -90.0,
            false_easting: 0.0,
            false_northing: 0.0,
            scale: 1.0,
            ellipsoid: Ellipsoid::WGS84,
            ..Default::default()
        };
        let proj = Projection::new(p).unwrap();
        for &(lon, lat) in &[(-165.0_f64, -85.0_f64), (-100.0, -80.0), (-170.0, -78.0)] {
            let (x, y) = proj.forward(lon, lat).unwrap();
            let (lon2, lat2) = proj.inverse(x, y).unwrap();
            assert!((lon2 - lon).abs() < TOL_DEG, "lon round-trip fail: {lon} → {lon2}");
            assert!((lat2 - lat).abs() < TOL_DEG, "lat round-trip fail: {lat} → {lat2}");
        }
    }

    /// North pole: point at the pole maps to (FE, FN)
    #[test]
    fn pole_maps_to_false_origin() {
        let p = ProjectionParams {
            kind: ProjectionKind::PolarStereographic { north: true, lat_ts: None },
            lon0: 0.0,
            lat0: 90.0,
            false_easting: 2_000_000.0,
            false_northing: 2_000_000.0,
            scale: 0.994,
            ellipsoid: Ellipsoid::WGS84,
            ..Default::default()
        };
        let proj = Projection::new(p).unwrap();
        let (x, y) = proj.forward(0.0, 90.0).unwrap();
        assert!((x - 2_000_000.0).abs() < TOL_M);
        assert!((y - 2_000_000.0).abs() < TOL_M);
    }
}
