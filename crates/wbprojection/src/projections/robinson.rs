//! Robinson pseudocylindrical compromise projection.
//! Uses lookup table interpolation as originally defined by Robinson (1963).

use crate::error::Result;
use crate::{to_degrees, to_radians};
use super::{ProjectionImpl, ProjectionParams};

/// Robinson table: (PLEN, PDFE) values at 5° latitude intervals 0..90.
/// PLEN = X interpolation factor, PDFE = Y interpolation factor.
#[rustfmt::skip]
const ROBINSON_TABLE: &[(f64, f64)] = &[
    (1.0000, 0.0000), // 0°
    (0.9986, 0.0620), // 5°
    (0.9954, 0.1240), // 10°
    (0.9900, 0.1860), // 15°
    (0.9822, 0.2480), // 20°
    (0.9730, 0.3100), // 25°
    (0.9600, 0.3720), // 30°
    (0.9427, 0.4340), // 35°
    (0.9216, 0.4958), // 40°
    (0.8962, 0.5571), // 45°
    (0.8679, 0.6176), // 50°
    (0.8350, 0.6769), // 55°
    (0.7986, 0.7346), // 60°
    (0.7597, 0.7903), // 65°
    (0.7186, 0.8435), // 70°
    (0.6732, 0.8936), // 75°
    (0.6213, 0.9394), // 80°
    (0.5722, 0.9761), // 85°
    (0.5322, 1.0000), // 90°
];

fn robinson_interpolate(lat_abs_deg: f64) -> (f64, f64) {
    let idx = (lat_abs_deg / 5.0) as usize;
    let idx = idx.min(ROBINSON_TABLE.len() - 2);
    let frac = (lat_abs_deg - idx as f64 * 5.0) / 5.0;
    let (p0, d0) = ROBINSON_TABLE[idx];
    let (p1, d1) = ROBINSON_TABLE[idx + 1];
    (p0 + frac * (p1 - p0), d0 + frac * (d1 - d0))
}

pub(super) struct RobinsonProj {
    lon0: f64,
    a: f64,
    fe: f64,
    fn_: f64,
}

impl RobinsonProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        Ok(RobinsonProj {
            lon0: to_radians(p.lon0),
            a: p.ellipsoid.a,
            fe: p.false_easting,
            fn_: p.false_northing,
        })
    }
}

const ROBINSON_SCALE: f64 = 0.8487;
const ROBINSON_C2: f64 = 1.3523;

impl ProjectionImpl for RobinsonProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat_abs = lat_deg.abs().min(90.0);
        let sign = lat_deg.signum();
        let (plen, pdfe) = robinson_interpolate(lat_abs);
        let x = self.a * ROBINSON_SCALE * plen * (lon - self.lon0) + self.fe;
        let y = self.a * ROBINSON_SCALE * ROBINSON_C2 * pdfe * sign + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let yy = (y - self.fn_) / (self.a * ROBINSON_SCALE * ROBINSON_C2);
        let sign = yy.signum();
        let yy_abs = yy.abs().min(1.0);

        // Find latitude by inverse interpolation of PDFE
        let mut lat_abs = yy_abs * 90.0;
        for _ in 0..20 {
            let idx = ((lat_abs / 5.0) as usize).min(ROBINSON_TABLE.len() - 2);
            let frac = (lat_abs - idx as f64 * 5.0) / 5.0;
            let (_, d0) = ROBINSON_TABLE[idx];
            let (_, d1) = ROBINSON_TABLE[idx + 1];
            let d_interp = d0 + frac * (d1 - d0);
            let dd_dlat = (d1 - d0) / 5.0;
            let delta = (yy_abs - d_interp) / dd_dlat;
            lat_abs += delta;
            lat_abs = lat_abs.clamp(0.0, 90.0);
            if delta.abs() < 1e-10 {
                break;
            }
        }

        let (plen, _) = robinson_interpolate(lat_abs);
        let lon = (x - self.fe) / (self.a * ROBINSON_SCALE * plen) + self.lon0;

        Ok((to_degrees(lon), lat_abs * sign))
    }
}
