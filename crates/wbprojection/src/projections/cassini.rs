//! Cassini-Soldner projection.

use super::{ProjectionImpl, ProjectionParams};
use crate::error::Result;
use crate::{to_degrees, to_radians};

pub(super) struct CassiniProj {
    a: f64,
    e2: f64,
    ep2: f64,
    lon0: f64,
    fe: f64,
    fn_: f64,
    m0: f64,
}

fn meridional_arc(a: f64, e2: f64, lat: f64) -> f64 {
    let e4 = e2 * e2;
    let e6 = e4 * e2;
    a * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * lat
        - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * lat).sin()
        + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * lat).sin()
        - (35.0 * e6 / 3072.0) * (6.0 * lat).sin())
}

impl CassiniProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        Ok(Self {
            a: p.ellipsoid.a,
            e2: p.ellipsoid.e2,
            ep2: p.ellipsoid.ep2,
            lon0: to_radians(p.lon0),
            fe: p.false_easting,
            fn_: p.false_northing,
            m0: meridional_arc(p.ellipsoid.a, p.ellipsoid.e2, lat0),
        })
    }
}

impl ProjectionImpl for CassiniProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let dlon = lon - self.lon0;

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let tan_lat = lat.tan();

        let n = self.a / (1.0 - self.e2 * sin_lat * sin_lat).sqrt();
        let t = tan_lat * tan_lat;
        let c = self.ep2 * cos_lat * cos_lat;
        let a = dlon * cos_lat;
        let m = meridional_arc(self.a, self.e2, lat);

        let x = n * (a - t * a.powi(3) / 6.0 - (8.0 - t + 8.0 * c) * t * a.powi(5) / 120.0) + self.fe;

        let y = (m - self.m0)
            + n * tan_lat * (a.powi(2) / 2.0 + (5.0 - t + 6.0 * c) * a.powi(4) / 24.0)
            + self.fn_;

        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let e2 = self.e2;
        let e4 = e2 * e2;
        let e6 = e4 * e2;

        let m1 = self.m0 + (y - self.fn_);
        let mu = m1 / (self.a * (1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0));

        let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
        let phi1 = mu
            + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
            + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
            + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
            + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();

        let sin_phi1 = phi1.sin();
        let cos_phi1 = phi1.cos();
        let tan_phi1 = phi1.tan();

        let n1 = self.a / (1.0 - e2 * sin_phi1 * sin_phi1).sqrt();
        let r1 = self.a * (1.0 - e2) / (1.0 - e2 * sin_phi1 * sin_phi1).powf(1.5);
        let t1 = tan_phi1 * tan_phi1;
        let d = (x - self.fe) / n1;

        let lat = phi1 - (n1 * tan_phi1 / r1) * (d * d / 2.0 - (1.0 + 3.0 * t1) * d.powi(4) / 24.0);

        let lon = self.lon0 + (d - t1 * d.powi(3) / 3.0 + (1.0 + 3.0 * t1) * t1 * d.powi(5) / 15.0) / cos_phi1;

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}
