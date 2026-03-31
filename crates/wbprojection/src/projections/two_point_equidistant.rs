//! Two-Point Equidistant projection (spherical formulation).

use super::{ProjectionImpl, ProjectionParams};
use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

pub(super) struct TwoPointEquidistantProj {
    a: f64,
    fe: f64,
    fn_: f64,
    c: f64,
    va: [f64; 3],
    vb: [f64; 3],
    n_ab: [f64; 3],
    lon_mid: f64,
    lat_mid: f64,
}

impl TwoPointEquidistantProj {
    pub fn new(p: &ProjectionParams, lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> Result<Self> {
        let lon1r = to_radians(lon1);
        let lat1r = to_radians(lat1);
        let lon2r = to_radians(lon2);
        let lat2r = to_radians(lat2);

        let va = sph_to_vec(lon1r, lat1r);
        let vb = sph_to_vec(lon2r, lat2r);
        let cos_c = dot(va, vb).clamp(-1.0, 1.0);
        let c = cos_c.acos();

        if c <= 1e-12 || (std::f64::consts::PI - c).abs() <= 1e-10 {
            return Err(ProjectionError::invalid_param(
                "TwoPointEquidistant",
                "control points must be distinct and non-antipodal",
            ));
        }

        let n_ab = normalize(cross(va, vb))?;
        let vm = normalize(add(va, vb))?;
        let (lon_mid, lat_mid) = vec_to_sph(vm);

        Ok(Self {
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
            c,
            va,
            vb,
            n_ab,
            lon_mid,
            lat_mid,
        })
    }

    fn forward_unit(&self, lon: f64, lat: f64) -> Result<(f64, f64)> {
        let vp = sph_to_vec(lon, lat);
        let d1 = dot(vp, self.va).clamp(-1.0, 1.0).acos();
        let d2 = dot(vp, self.vb).clamp(-1.0, 1.0).acos();

        let x = (d1 * d1 - d2 * d2) / (2.0 * self.c);
        let y2 = d1 * d1 - (x + self.c / 2.0) * (x + self.c / 2.0);
        let y_abs = y2.max(0.0).sqrt();
        let side = dot(self.n_ab, vp);
        let y = if side >= 0.0 { y_abs } else { -y_abs };

        Ok((x, y))
    }
}

impl ProjectionImpl for TwoPointEquidistantProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let (xu, yu) = self.forward_unit(lon, lat)?;
        Ok((self.a * xu + self.fe, self.a * yu + self.fn_))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        // Numerical inverse (Newton with finite-difference Jacobian).
        let target_x = (x - self.fe) / self.a;
        let target_y = (y - self.fn_) / self.a;

        let mut lon = self.lon_mid;
        let mut lat = self.lat_mid;

        for _ in 0..30 {
            let (fx, fy) = self.forward_unit(lon, lat)?;
            let rx = fx - target_x;
            let ry = fy - target_y;

            if rx.abs() + ry.abs() < 1e-13 {
                return Ok((to_degrees(lon), to_degrees(lat)));
            }

            let h = 1e-7;
            let (fx_lon, fy_lon) = self.forward_unit(lon + h, lat)?;
            let (fx_lat, fy_lat) = self.forward_unit(lon, lat + h)?;

            let j11 = (fx_lon - fx) / h;
            let j21 = (fy_lon - fy) / h;
            let j12 = (fx_lat - fx) / h;
            let j22 = (fy_lat - fy) / h;

            let det = j11 * j22 - j12 * j21;
            if det.abs() < 1e-18 {
                break;
            }

            let dlon = (-rx * j22 + ry * j12) / det;
            let dlat = (-ry * j11 + rx * j21) / det;

            lon += dlon;
            lat += dlat;

            lat = lat.clamp(
                -std::f64::consts::FRAC_PI_2 + 1e-10,
                std::f64::consts::FRAC_PI_2 - 1e-10,
            );
            if lon > std::f64::consts::PI {
                lon -= 2.0 * std::f64::consts::PI;
            } else if lon < -std::f64::consts::PI {
                lon += 2.0 * std::f64::consts::PI;
            }
        }

        Err(ProjectionError::ConvergenceFailure { iterations: 30 })
    }
}

fn sph_to_vec(lon: f64, lat: f64) -> [f64; 3] {
    let clat = lat.cos();
    [clat * lon.cos(), clat * lon.sin(), lat.sin()]
}

fn vec_to_sph(v: [f64; 3]) -> (f64, f64) {
    let lon = v[1].atan2(v[0]);
    let lat = v[2].clamp(-1.0, 1.0).asin();
    (lon, lat)
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

fn normalize(v: [f64; 3]) -> Result<[f64; 3]> {
    let n = dot(v, v).sqrt();
    if n <= 1e-15 {
        return Err(ProjectionError::invalid_param("TwoPointEquidistant", "degenerate basis"));
    }
    Ok([v[0] / n, v[1] / n, v[2] / n])
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    #[test]
    fn round_trip_two_point_equidistant() {
        let proj = Projection::new(
            ProjectionParams::new(ProjectionKind::TwoPointEquidistant {
                lon1: -10.0,
                lat1: 40.0,
                lon2: 20.0,
                lat2: 50.0,
            }),
        )
        .unwrap();

        let (x, y) = proj.forward(5.0, 45.0).unwrap();
        let (lon, lat) = proj.inverse(x, y).unwrap();
        assert!((lon - 5.0).abs() < 1e-6);
        assert!((lat - 45.0).abs() < 1e-6);
    }
}
