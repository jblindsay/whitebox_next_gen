//! Equal Earth projection.

use crate::error::{ProjectionError, Result};
use crate::{to_degrees, to_radians};

use super::{ProjectionImpl, ProjectionParams};

const A1: f64 = 1.340_264;
const A2: f64 = -0.081_106;
const A3: f64 = 0.000_893;
const A4: f64 = 0.003_796;
const M: f64 = 0.866_025_403_784_438_6; // sqrt(3)/2
const MAX_Y: f64 = 1.317_362_759_157_4;
const EPS: f64 = 1e-11;
const MAX_ITER: usize = 12;

fn qsfn(e: f64, sinphi: f64) -> f64 {
    if e.abs() < 1e-15 {
        return 2.0 * sinphi;
    }
    let esin = e * sinphi;
    (1.0 - e * e)
        * (sinphi / (1.0 - esin * esin)
            - (1.0 / (2.0 * e)) * ((1.0 - esin) / (1.0 + esin)).ln())
}

fn authalic_lat_inverse(q_target: f64, e: f64) -> f64 {
    if e.abs() < 1e-15 {
        return (0.5 * q_target).clamp(-1.0, 1.0).asin();
    }

    let qp = qsfn(e, 1.0);
    let mut phi = (q_target / qp).clamp(-1.0, 1.0).asin();

    for _ in 0..25 {
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let qphi = qsfn(e, sinphi);
        let e2 = e * e;
        let denom = (1.0 - e2 * sinphi * sinphi).powi(2);
        if denom.abs() < 1e-18 || cosphi.abs() < 1e-18 {
            break;
        }
        let dqdphi = 2.0 * (1.0 - e2) * cosphi / denom;
        let step = (qphi - q_target) / dqdphi;
        phi -= step;
        if step.abs() < 1e-12 {
            break;
        }
    }

    phi
}

pub(super) struct EqualEarthProj {
    lon0: f64,
    fe: f64,
    fn_: f64,
    a: f64,
    e: f64,
    qp: f64,
    rqda: f64,
}

impl EqualEarthProj {
    pub fn new(p: &ProjectionParams) -> Result<Self> {
        let e = p.ellipsoid.e;
        let qp = if e.abs() < 1e-15 { 2.0 } else { qsfn(e, 1.0) };
        let rqda = if e.abs() < 1e-15 {
            1.0
        } else {
            (0.5 * qp).sqrt()
        };

        Ok(Self {
            lon0: to_radians(p.lon0),
            fe: p.false_easting,
            fn_: p.false_northing,
            a: p.ellipsoid.a,
            e,
            qp,
            rqda,
        })
    }
}

impl ProjectionImpl for EqualEarthProj {
    fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64)> {
        let lon = to_radians(lon_deg);
        let lat = to_radians(lat_deg);
        let lam = lon - self.lon0;

        let mut sbeta = lat.sin();
        if self.e.abs() > 1e-15 {
            sbeta = qsfn(self.e, sbeta) / self.qp;
            if sbeta.abs() > 1.0 {
                sbeta = sbeta.signum();
            }
        }

        let psi = (M * sbeta).asin();
        let psi2 = psi * psi;
        let psi6 = psi2 * psi2 * psi2;

        let x_norm = lam * psi.cos()
            / (M * (A1 + 3.0 * A2 * psi2 + psi6 * (7.0 * A3 + 9.0 * A4 * psi2)));
        let y_norm = psi * (A1 + A2 * psi2 + psi6 * (A3 + A4 * psi2));

        let x = self.a * self.rqda * x_norm + self.fe;
        let y = self.a * self.rqda * y_norm + self.fn_;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        let mut x_norm = (x - self.fe) / (self.a * self.rqda);
        let mut y_norm = (y - self.fn_) / (self.a * self.rqda);

        y_norm = y_norm.clamp(-MAX_Y, MAX_Y);
        let mut yc = y_norm;

        for _ in 0..MAX_ITER {
            let y2 = yc * yc;
            let y6 = y2 * y2 * y2;

            let f = yc * (A1 + A2 * y2 + y6 * (A3 + A4 * y2)) - y_norm;
            let fder = A1 + 3.0 * A2 * y2 + y6 * (7.0 * A3 + 9.0 * A4 * y2);

            let tol = f / fder;
            yc -= tol;
            if tol.abs() < EPS {
                break;
            }
        }

        let y2 = yc * yc;
        let y6 = y2 * y2 * y2;
        let denom = yc.cos();
        if denom.abs() < 1e-15 {
            return Err(ProjectionError::out_of_bounds(
                "Equal Earth inverse near singularity",
            ));
        }

        x_norm = M * x_norm * (A1 + 3.0 * A2 * y2 + y6 * (7.0 * A3 + 9.0 * A4 * y2)) / denom;

        let mut lat = (yc.sin() / M).clamp(-1.0, 1.0).asin();
        if self.e.abs() > 1e-15 {
            let q = self.qp * lat.sin();
            lat = authalic_lat_inverse(q, self.e);
        }

        let lon = x_norm + self.lon0;
        Ok((to_degrees(lon), to_degrees(lat)))
    }
}

#[cfg(test)]
mod tests {
    use crate::projections::{Projection, ProjectionKind, ProjectionParams};

    fn eqearth() -> Projection {
        Projection::new(
            ProjectionParams::new(ProjectionKind::EqualEarth)
                .with_lon0(0.0)
                .with_false_easting(0.0)
                .with_false_northing(0.0),
        )
        .unwrap()
    }

    #[test]
    fn round_trip_berlin() {
        let p = eqearth();
        let (x, y) = p.forward(13.4, 52.5).unwrap();
        let (lon, lat) = p.inverse(x, y).unwrap();
        assert!((lon - 13.4).abs() < 1e-7);
        assert!((lat - 52.5).abs() < 1e-7);
    }

    #[test]
    fn round_trip_southern() {
        let p = eqearth();
        let (x, y) = p.forward(151.2, -33.9).unwrap();
        let (lon, lat) = p.inverse(x, y).unwrap();
        assert!((lon - 151.2).abs() < 1e-7);
        assert!((lat - -33.9).abs() < 1e-7);
    }
}
