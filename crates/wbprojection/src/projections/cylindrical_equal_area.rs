//! Lambert Cylindrical Equal-Area projection.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

pub(super) struct CylindricalEqualAreaProj {
    lon0: f64,
    a: f64,
    e: f64,
    e2: f64,
    k0: f64,
    fe: f64,
    fn_: f64,
    qp: f64,
}

fn qsfn(e: f64, sinphi: f64) -> f64 {
    if e.abs() < 1e-15 {
        return 2.0 * sinphi;
    }
    let esin = e * sinphi;
    (1.0 - e * e)
        * (sinphi / (1.0 - esin * esin)
            - (1.0 / (2.0 * e)) * ((1.0 - esin) / (1.0 + esin)).ln())
}

impl CylindricalEqualAreaProj {
    pub fn new(p: &ProjectionParams, lat_ts_deg: f64) -> Result<Self> {
        let lat_ts = to_radians(lat_ts_deg);
        let sin_lat_ts = lat_ts.sin();
        let e2 = p.ellipsoid.e2;
        let e = p.ellipsoid.e;

        let k0 = if e.abs() < 1e-15 {
            lat_ts.cos()
        } else {
            lat_ts.cos() / (1.0 - e2 * sin_lat_ts * sin_lat_ts).sqrt()
        };

        if k0.abs() < 1e-15 {
            return Err(ProjectionError::invalid_param(
                "lat_ts",
                "standard parallel too close to ±90° for cylindrical equal area",
            ));
        }

        let qp = qsfn(e, 1.0);

        Ok(Self {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            e,
            e2,
            k0,
            fe: p.false_easting,
            fn_: p.false_northing,
            qp,
        })
    }
}

impl ProjectionImpl for CylindricalEqualAreaProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);

        let x = self.a * self.k0 * (lon - self.lon0) + self.fe;
        let y = if self.e.abs() < 1e-15 {
            self.a * lat.sin() / self.k0 + self.fn_
        } else {
            self.a * 0.5 * qsfn(self.e, lat.sin()) / self.k0 + self.fn_
        };

        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let lon = (x - self.fe) / (self.a * self.k0) + self.lon0;

        let lat = if self.e.abs() < 1e-15 {
            let t = (y - self.fn_) * self.k0 / self.a;
            if t.abs() > 1.0 + 1e-12 {
                return Err(ProjectionError::out_of_bounds(
                    "Cylindrical Equal Area inverse outside valid domain",
                ));
            }
            t.clamp(-1.0, 1.0).asin()
        } else {
            let q = 2.0 * (y - self.fn_) * self.k0 / self.a;
            let target = q;
            let mut phi = (target / self.qp).clamp(-1.0, 1.0).asin();

            for _ in 0..25 {
                let sinphi = phi.sin();
                let cosphi = phi.cos();
                let qphi = qsfn(self.e, sinphi);
                let denom = (1.0 - self.e2 * sinphi * sinphi).powi(2);
                if denom.abs() < 1e-18 || cosphi.abs() < 1e-18 {
                    break;
                }
                let dqdphi = 2.0 * (1.0 - self.e2) * cosphi / denom;
                let step = (qphi - target) / dqdphi;
                phi -= step;
                if step.abs() < 1e-12 {
                    break;
                }
            }
            phi
        };

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    fn cea_wgs84(lat_ts: f64) -> Projection {
        Projection::new(
            ProjectionParams::new(ProjectionKind::CylindricalEqualArea { lat_ts })
                .with_lon0(0.0)
                .with_lat0(0.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_global_ease() {
        let proj = cea_wgs84(30.0);
        let (x, y) = proj.forward(20.0, 45.0).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - 20.0).abs() < 1e-7);
        assert!((lat - 45.0).abs() < 1e-7);
    }

    #[test]
    fn round_trip_southern() {
        let proj = cea_wgs84(30.0);
        let (x, y) = proj.forward(-75.0, -60.0).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - -75.0).abs() < 1e-7);
        assert!((lat - -60.0).abs() < 1e-7);
    }
}
