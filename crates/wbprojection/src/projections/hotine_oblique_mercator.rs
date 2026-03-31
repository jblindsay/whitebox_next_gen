//! Hotine Oblique Mercator (azimuth at projection center).
//!
//! This implementation uses a spherical oblique-Mercator construction anchored
//! at the projection center and azimuth, with ellipsoid semi-major axis as the
//! map radius. It is adequate for robust forward/inverse behavior for state
//! plane usage and preserves round-trip consistency.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

pub(super) struct HotineObliqueMercatorProj {
    a: f64,
    k0: f64,
    fe: f64,
    fn_: f64,
    cos_theta: f64,
    sin_theta: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    z_axis: [f64; 3],
}

impl HotineObliqueMercatorProj {
    pub fn new(p: &ProjectionParams, azimuth_deg: f64, rectified_grid_angle_deg: f64) -> Result<Self> {
        if p.scale <= 0.0 {
            return Err(ProjectionError::invalid_param("scale", "must be positive"));
        }

        let lonc = to_radians(p.lon0);
        let latc = to_radians(p.lat0);
        let az = to_radians(azimuth_deg);
        let theta = to_radians(rectified_grid_angle_deg - azimuth_deg);

        let up = lon_lat_to_vec(lonc, latc);
        let east = normalize(cross([0.0, 0.0, 1.0], up))?;
        let north = normalize(cross(up, east))?;

        let direction = normalize(add(scale(east, az.sin()), scale(north, az.cos())))?;
        let z_axis = normalize(cross(up, direction))?;
        let x_axis = normalize(sub(up, scale(z_axis, dot(up, z_axis))))?;
        let y_axis = normalize(cross(z_axis, x_axis))?;

        Ok(Self {
            a: p.ellipsoid.a,
            k0: p.scale,
            fe: p.false_easting,
            fn_: p.false_northing,
            cos_theta: theta.cos(),
            sin_theta: theta.sin(),
            x_axis,
            y_axis,
            z_axis,
        })
    }
}

impl ProjectionImpl for HotineObliqueMercatorProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let v = lon_lat_to_vec(lon, lat);

        let xr = dot(v, self.x_axis);
        let yr = dot(v, self.y_axis);
        let zr = dot(v, self.z_axis).clamp(-1.0, 1.0);

        let lat_r = zr.asin();
        if lat_r.abs() >= std::f64::consts::FRAC_PI_2 {
            return Err(ProjectionError::out_of_bounds(
                "latitude reaches oblique Mercator singularity",
            ));
        }
        let lon_r = yr.atan2(xr);

        let x = self.a * self.k0 * lon_r;
        let y = self.a * self.k0 * (std::f64::consts::FRAC_PI_4 + lat_r / 2.0).tan().ln();

        // Rectified skew orthomorphic methods apply an additional XY-plane
        // rotation between the projection center azimuth and rectified grid angle.
        let xr = x * self.cos_theta - y * self.sin_theta;
        let yr = x * self.sin_theta + y * self.cos_theta;

        Ok((xr + self.fe, yr + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let x0 = x - self.fe;
        let y0 = y - self.fn_;

        // Undo rectification rotation.
        let xu = x0 * self.cos_theta + y0 * self.sin_theta;
        let yu = -x0 * self.sin_theta + y0 * self.cos_theta;

        let lon_r = xu / (self.a * self.k0);
        let lat_r = 2.0 * (yu / (self.a * self.k0)).exp().atan() - std::f64::consts::FRAC_PI_2;

        let cos_lat_r = lat_r.cos();
        let v = add(
            add(
                scale(self.x_axis, cos_lat_r * lon_r.cos()),
                scale(self.y_axis, cos_lat_r * lon_r.sin()),
            ),
            scale(self.z_axis, lat_r.sin()),
        );
        let vn = normalize(v)?;

        let lon = to_degrees(vn[1].atan2(vn[0]));
        let lat = to_degrees(vn[2].clamp(-1.0, 1.0).asin());
        Ok((lon, lat))
    }
}

fn lon_lat_to_vec(lon: f64, lat: f64) -> [f64; 3] {
    let clat = lat.cos();
    [clat * lon.cos(), clat * lon.sin(), lat.sin()]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(v: [f64; 3], k: f64) -> [f64; 3] {
    [v[0] * k, v[1] * k, v[2] * k]
}

fn normalize(v: [f64; 3]) -> Result<[f64; 3]> {
    let norm = dot(v, v).sqrt();
    if norm <= 1e-15 {
        return Err(ProjectionError::invalid_param(
            "oblique_mercator",
            "degenerate projection orientation",
        ));
    }
    Ok([v[0] / norm, v[1] / norm, v[2] / norm])
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    #[test]
    fn round_trip_alaska_zone1_like() {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::HotineObliqueMercator {
                azimuth: 323.130_102_361_111,
                rectified_grid_angle: None,
            })
            .with_lat0(57.0)
            .with_lon0(-133.666_666_666_667)
            .with_scale(0.9999)
            .with_false_easting(5_000_000.0)
            .with_false_northing(-5_000_000.0),
        )
        .unwrap();

        let (x, y) = proj.forward(-134.3, 57.2).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon + 134.3).abs() < 1e-6);
        assert!((lat - 57.2).abs() < 1e-6);
    }
}
