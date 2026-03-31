//! Geostationary Satellite View projection.
//!
//! This is a spherical geostationary projection suitable for typical
//! weather-satellite style navigation products. The implementation supports
//! both sweep-axis conventions used in practice:
//! - `sweep_x = true`  (GOES-style)
//! - `sweep_x = false` (Meteosat-style)

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

pub(super) struct GeostationaryProj {
    a: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
    sweep_x: bool,
    h_total: f64,
}

impl GeostationaryProj {
    pub fn new(p: &ProjectionParams, satellite_height: f64, sweep_x: bool) -> Result<Self> {
        if satellite_height <= 0.0 {
            return Err(ProjectionError::invalid_param(
                "satellite_height",
                "must be positive",
            ));
        }

        let a = p.ellipsoid.a;
        Ok(Self {
            a,
            lon0: to_radians(p.lon0),
            fe: p.false_easting,
            fn_: p.false_northing,
            sweep_x,
            h_total: a + satellite_height,
        })
    }
}

impl ProjectionImpl for GeostationaryProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lam = lon - self.lon0;

        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let cos_lam = lam.cos();
        let sin_lam = lam.sin();

        let r1 = self.h_total - self.a * cos_lat * cos_lam;
        let r2 = self.a * cos_lat * sin_lam;
        let r3 = self.a * sin_lat;

        // Visible hemisphere test from satellite viewpoint.
        if self.h_total * (self.h_total - self.a * cos_lat * cos_lam) < r2 * r2 + r3 * r3 {
            return Err(ProjectionError::out_of_bounds(
                "point is outside geostationary visible hemisphere",
            ));
        }

        let (x_ang, y_ang) = if self.sweep_x {
            (r2.atan2(r1), r3.atan2((r1 * r1 + r2 * r2).sqrt()))
        } else {
            (r2.atan2((r1 * r1 + r3 * r3).sqrt()), r3.atan2(r1))
        };

        Ok((self.a * x_ang + self.fe, self.a * y_ang + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x_ang = (x - self.fe) / self.a;
        let y_ang = (y - self.fn_) / self.a;

        let tx = x_ang.tan();
        let ty = y_ang.tan();

        let c = (1.0 + tx * tx) * (1.0 + ty * ty);
        let disc = self.h_total * self.h_total - c * (self.h_total * self.h_total - self.a * self.a);
        if disc < 0.0 {
            return Err(ProjectionError::out_of_bounds(
                "projected point is outside geostationary domain",
            ));
        }

        let r1 = (self.h_total - disc.sqrt()) / c;

        let (r2, r3) = if self.sweep_x {
            (r1 * tx, r1 * (1.0 + tx * tx).sqrt() * ty)
        } else {
            (r1 * (1.0 + ty * ty).sqrt() * tx, r1 * ty)
        };

        let xg = self.h_total - r1;
        let yg = r2;
        let zg = r3;

        let lon = self.lon0 + yg.atan2(xg);
        let lat = zg.atan2((xg * xg + yg * yg).sqrt());

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    #[test]
    fn round_trip_geos_sweep_x() {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::Geostationary {
                satellite_height: 35_786_023.0,
                sweep_x: true,
            })
            .with_lon0(-75.0),
        )
        .unwrap();

        let (x, y) = proj.forward(-80.0, 20.0).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon + 80.0).abs() < 1e-6);
        assert!((lat - 20.0).abs() < 1e-6);
    }

    #[test]
    fn round_trip_geos_sweep_y() {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::Geostationary {
                satellite_height: 35_786_023.0,
                sweep_x: false,
            })
            .with_lon0(0.0),
        )
        .unwrap();

        let (x, y) = proj.forward(5.0, 15.0).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - 5.0).abs() < 1e-6);
        assert!((lat - 15.0).abs() < 1e-6);
    }
}
