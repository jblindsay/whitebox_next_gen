//! Lambert Azimuthal Equal-Area projection.
//! Preserves area globally on a sphere.

use crate::error::Result;
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

pub(super) struct LambertAzimuthalEqualAreaProj {
    lon0: f64,
    lat0: f64,
    sin_lat0: f64,
    cos_lat0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl LambertAzimuthalEqualAreaProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let lat0 = to_radians(p.lat0);
        Ok(Self {
            lon0: to_radians(p.lon0),
            lat0,
            sin_lat0: lat0.sin(),
            cos_lat0: lat0.cos(),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

impl ProjectionImpl for LambertAzimuthalEqualAreaProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let dlon = lon - self.lon0;

        let sin_lat = lat.sin();
        let cos_lat = lat.cos();
        let cos_dlon = dlon.cos();

        let denom = 1.0 + self.sin_lat0 * sin_lat + self.cos_lat0 * cos_lat * cos_dlon;
        let k = (2.0 / denom).sqrt();

        let x = self.a * k * cos_lat * dlon.sin() + self.fe;
        let y = self.a * k * (self.cos_lat0 * sin_lat - self.sin_lat0 * cos_lat * cos_dlon) + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x = x - self.fe;
        let y = y - self.fn_;

        let rho = (x * x + y * y).sqrt();
        if rho < 1e-12 {
            return Ok((to_degrees(self.lon0), to_degrees(self.lat0)));
        }

        let c = 2.0 * (rho / (2.0 * self.a)).asin();
        let sin_c = c.sin();
        let cos_c = c.cos();

        let lat = (cos_c * self.sin_lat0 + y * sin_c * self.cos_lat0 / rho).asin();
        let lon = self.lon0 + (x * sin_c)
            .atan2(rho * self.cos_lat0 * cos_c - y * self.sin_lat0 * sin_c);

        Ok((to_degrees(lon), to_degrees(lat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    const TOL_DEGREES: f64 = 1e-8;

    fn round_trip(lon: f64, lat: f64, lon0: f64, lat0: f64) {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::LambertAzimuthalEqualArea)
                .with_lon0(lon0)
                .with_lat0(lat0),
        )
        .unwrap();
        let (x, y) = proj.forward(lon, lat).unwrap();
        let (lon2, lat2) = proj.inverse(x, y).unwrap();
        assert!((lon2 - lon).abs() < TOL_DEGREES, "lon: {lon} -> {lon2}");
        assert!((lat2 - lat).abs() < TOL_DEGREES, "lat: {lat} -> {lat2}");
    }

    #[test]
    fn round_trip_oblique() {
        round_trip(10.0, 52.0, 10.0, 52.0);
        round_trip(15.0, 45.0, 10.0, 52.0);
    }

    #[test]
    fn round_trip_equatorial() {
        round_trip(5.0, 10.0, 0.0, 0.0);
        round_trip(-70.0, -20.0, 0.0, 0.0);
    }
}
