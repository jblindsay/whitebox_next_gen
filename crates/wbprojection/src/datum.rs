//! Geodetic datum definitions and datum transformations.
//!
//! A datum ties an ellipsoid to the actual Earth by defining its position
//! and orientation. Datum transformations allow converting coordinates
//! between different datums.

use crate::ellipsoid::Ellipsoid;
use crate::error::{ProjectionError, Result};
use crate::grid_shift::get_grid;
use crate::grid_formats::resolve_ntv2_hierarchy_grid;
use wide::f64x4;

/// A geodetic datum.
#[derive(Debug, Clone, PartialEq)]
pub struct Datum {
    /// Human-readable name.
    pub name: &'static str,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// Transformation method from this datum to WGS84.
    pub transform: DatumTransform,
}

/// Datum transformation strategy to/from WGS84.
#[derive(Debug, Clone, PartialEq)]
pub enum DatumTransform {
    /// No datum shift is required (or datum is treated as equivalent to WGS84).
    None,
    /// Helmert 3-parameter translation to WGS84.
    Helmert3(HelmertParams),
    /// Helmert 7-parameter similarity transform to WGS84.
    Helmert7(HelmertParams),
    /// Grid-shift transform to WGS84 (reserved for future implementation).
    GridShift {
        /// Registered grid dataset name.
        grid_name: &'static str,
    },
    /// NTv2 multi-subgrid hierarchy dataset.
    Ntv2Hierarchy {
        /// Registered NTv2 hierarchy dataset identifier.
        dataset_name: &'static str,
    },
}

/// Policy for handling datum-transform edge cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatumTransformPolicy {
    /// Return an error when a transform cannot be applied exactly.
    Strict,
    /// For grid-shift failures (missing grid / out-of-extent), fall back to identity.
    FallbackToIdentityGridShift,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DatumGeodeticTrace {
    /// Geodetic latitude in radians.
    pub lat_rad: f64,
    /// Geodetic longitude in radians.
    pub lon_rad: f64,
    /// Ellipsoidal height in meters.
    pub h: f64,
    /// Optional selected NTv2 subgrid name.
    pub selected_grid: Option<String>,
}

/// Helmert 7-parameter similarity transformation.
///
/// Transforms from source datum to WGS84:
/// ```text
/// X_wgs84 = tx + (1 + ds) * (X + rz*Y - ry*Z)
/// Y_wgs84 = ty + (1 + ds) * (-rz*X + Y + rx*Z)
/// Z_wgs84 = tz + (1 + ds) * (ry*X - rx*Y + Z)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HelmertParams {
    /// Translation in X (meters).
    pub tx: f64,
    /// Translation in Y (meters).
    pub ty: f64,
    /// Translation in Z (meters).
    pub tz: f64,
    /// Rotation around X axis (arc-seconds).
    pub rx: f64,
    /// Rotation around Y axis (arc-seconds).
    pub ry: f64,
    /// Rotation around Z axis (arc-seconds).
    pub rz: f64,
    /// Scale change (parts per million).
    pub ds: f64,
}

impl HelmertParams {
    /// 3-parameter translation (Molodensky-style).
    pub fn translation(tx: f64, ty: f64, tz: f64) -> Self {
        HelmertParams { tx, ty, tz, rx: 0.0, ry: 0.0, rz: 0.0, ds: 0.0 }
    }

    /// Apply this Helmert transform to ECEF coordinates, returning WGS84 ECEF.
    pub fn apply(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);
        let rx = self.rx * ARCSEC_TO_RAD;
        let ry = self.ry * ARCSEC_TO_RAD;
        let rz = self.rz * ARCSEC_TO_RAD;
        let ds = self.ds * 1e-6;
        let scale = 1.0 + ds;

        let xw = self.tx + scale * (x + rz * y - ry * z);
        let yw = self.ty + scale * (-rz * x + y + rx * z);
        let zw = self.tz + scale * (ry * x - rx * y + z);
        (xw, yw, zw)
    }

    /// Apply the inverse transform (WGS84 ECEF → source datum ECEF).
    pub fn apply_inverse(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);
        let rx = -self.rx * ARCSEC_TO_RAD;
        let ry = -self.ry * ARCSEC_TO_RAD;
        let rz = -self.rz * ARCSEC_TO_RAD;
        let ds = -self.ds * 1e-6;
        let tx = -self.tx;
        let ty = -self.ty;
        let tz = -self.tz;
        let scale = 1.0 + ds;

        let xd = tx + scale * (x + rz * y - ry * z);
        let yd = ty + scale * (-rz * x + y + rx * z);
        let zd = tz + scale * (ry * x - rx * y + z);
        (xd, yd, zd)
    }

    /// Apply this Helmert transform to a batch of 4 ECEF coordinate tuples using SIMD.
    /// Input: 4 tuples of (x, y, z) packed in arrays x4, y4, z4.
    /// Returns: 4 tuples of transformed (x, y, z) in arrays.
    pub fn apply_simd_batch4(&self, x4: &[f64; 4], y4: &[f64; 4], z4: &[f64; 4]) -> ([f64; 4], [f64; 4], [f64; 4]) {
        const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);
        let rx = self.rx * ARCSEC_TO_RAD;
        let ry = self.ry * ARCSEC_TO_RAD;
        let rz = self.rz * ARCSEC_TO_RAD;
        let scale = 1.0 + self.ds * 1e-6;

        let rx_v = f64x4::splat(rx);
        let ry_v = f64x4::splat(ry);
        let rz_v = f64x4::splat(rz);
        let neg_rx_v = f64x4::splat(-rx);
        let neg_rz_v = f64x4::splat(-rz);
        let scale_v = f64x4::splat(scale);
        let tx_v = f64x4::splat(self.tx);
        let ty_v = f64x4::splat(self.ty);
        let tz_v = f64x4::splat(self.tz);

        let x_v = f64x4::new(*x4);
        let y_v = f64x4::new(*y4);
        let z_v = f64x4::new(*z4);

        let xw_v = tx_v + scale_v * (x_v + rz_v * y_v - ry_v * z_v);
        let yw_v = ty_v + scale_v * (neg_rz_v * x_v + y_v + rx_v * z_v);
        let zw_v = tz_v + scale_v * (ry_v * x_v + neg_rx_v * y_v + z_v);

        (<[f64; 4]>::from(xw_v), <[f64; 4]>::from(yw_v), <[f64; 4]>::from(zw_v))
    }

    /// Apply the inverse Helmert transform to a batch of 4 ECEF coordinate tuples using SIMD.
    pub fn apply_inverse_simd_batch4(&self, x4: &[f64; 4], y4: &[f64; 4], z4: &[f64; 4]) -> ([f64; 4], [f64; 4], [f64; 4]) {
        const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3600.0);
        let rx = -self.rx * ARCSEC_TO_RAD;
        let ry = -self.ry * ARCSEC_TO_RAD;
        let rz = -self.rz * ARCSEC_TO_RAD;
        let scale = 1.0 - self.ds * 1e-6;

        let rx_v = f64x4::splat(rx);
        let ry_v = f64x4::splat(ry);
        let rz_v = f64x4::splat(rz);
        let neg_rx_v = f64x4::splat(-rx);
        let neg_rz_v = f64x4::splat(-rz);
        let scale_v = f64x4::splat(scale);
        let tx_v = f64x4::splat(-self.tx);
        let ty_v = f64x4::splat(-self.ty);
        let tz_v = f64x4::splat(-self.tz);

        let x_v = f64x4::new(*x4);
        let y_v = f64x4::new(*y4);
        let z_v = f64x4::new(*z4);

        let xd_v = tx_v + scale_v * (x_v + rz_v * y_v - ry_v * z_v);
        let yd_v = ty_v + scale_v * (neg_rz_v * x_v + y_v + rx_v * z_v);
        let zd_v = tz_v + scale_v * (ry_v * x_v + neg_rx_v * y_v + z_v);

        (<[f64; 4]>::from(xd_v), <[f64; 4]>::from(yd_v), <[f64; 4]>::from(zd_v))
    }
}

/// Convert geodetic (lat, lon, height) to ECEF Cartesian (X, Y, Z).
pub fn geodetic_to_ecef(lat_rad: f64, lon_rad: f64, h: f64, ellipsoid: &Ellipsoid) -> (f64, f64, f64) {
    let n = ellipsoid.normal_radius(lat_rad);
    let cos_lat = lat_rad.cos();
    let sin_lat = lat_rad.sin();
    let cos_lon = lon_rad.cos();
    let sin_lon = lon_rad.sin();
    let x = (n + h) * cos_lat * cos_lon;
    let y = (n + h) * cos_lat * sin_lon;
    let z = (n * (1.0 - ellipsoid.e2) + h) * sin_lat;
    (x, y, z)
}

/// Convert ECEF Cartesian (X, Y, Z) to geodetic (lat, lon, height).
/// Uses the iterative Bowring method.
pub fn ecef_to_geodetic(x: f64, y: f64, z: f64, ellipsoid: &Ellipsoid) -> (f64, f64, f64) {
    let a = ellipsoid.a;
    let b = ellipsoid.b;
    let e2 = ellipsoid.e2;
    let ep2 = ellipsoid.ep2;

    let lon = y.atan2(x);
    let p = (x * x + y * y).sqrt();
    let theta = (z * a).atan2(p * b);

    let lat = (z + ep2 * b * theta.sin().powi(3))
        .atan2(p - e2 * a * theta.cos().powi(3));

    let n = ellipsoid.normal_radius(lat);
    let h = if lat.cos().abs() > 1e-10 {
        p / lat.cos() - n
    } else {
        z.abs() / lat.sin() - n * (1.0 - e2)
    };

    (lat, lon, h)
}

/// Common datums.
impl Datum {
    /// Return a copy of this datum that uses a named grid-shift transform.
    pub fn with_grid_shift(mut self, grid_name: &'static str) -> Self {
        self.transform = DatumTransform::GridShift { grid_name };
        self
    }

    /// Return a copy of this datum that uses an NTv2 hierarchy dataset.
    pub fn with_ntv2_hierarchy(mut self, dataset_name: &'static str) -> Self {
        self.transform = DatumTransform::Ntv2Hierarchy { dataset_name };
        self
    }

    fn apply_grid_shift_to_wgs84(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        grid_name: &str,
    ) -> Result<(f64, f64, f64)> {
        let grid = get_grid(grid_name)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "grid-shift transform '{grid_name}' not registered"
            ))
        })?;

        let lon_deg = lon_rad.to_degrees();
        let lat_deg = lat_rad.to_degrees();
        let (dlon_deg, dlat_deg) = grid.sample_shift_degrees(lon_deg, lat_deg)?;

        Ok((
            (lat_deg + dlat_deg).to_radians(),
            (lon_deg + dlon_deg).to_radians(),
            h,
        ))
    }

    fn apply_grid_shift_from_wgs84(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        grid_name: &str,
    ) -> Result<(f64, f64, f64)> {
        let grid = get_grid(grid_name)?.ok_or_else(|| {
            ProjectionError::DatumError(format!(
                "grid-shift transform '{grid_name}' not registered"
            ))
        })?;

        let target_lon = lon_rad.to_degrees();
        let target_lat = lat_rad.to_degrees();

        let mut src_lon = target_lon;
        let mut src_lat = target_lat;

        for _ in 0..8 {
            let (dlon_deg, dlat_deg) = grid.sample_shift_degrees(src_lon, src_lat)?;
            let pred_lon = src_lon + dlon_deg;
            let pred_lat = src_lat + dlat_deg;
            src_lon += target_lon - pred_lon;
            src_lat += target_lat - pred_lat;
        }

        Ok((src_lat.to_radians(), src_lon.to_radians(), h))
    }

    /// Transform geodetic coordinates from this datum to WGS84 with policy control.
    pub fn to_wgs84_geodetic_with_policy(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        policy: DatumTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let trace = self.to_wgs84_geodetic_with_policy_and_trace(lat_rad, lon_rad, h, policy)?;
        Ok((trace.lat_rad, trace.lon_rad, trace.h))
    }

    pub(crate) fn to_wgs84_geodetic_with_policy_and_trace(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        policy: DatumTransformPolicy,
    ) -> Result<DatumGeodeticTrace> {
        match &self.transform {
            DatumTransform::None => Ok(DatumGeodeticTrace {
                lat_rad,
                lon_rad,
                h,
                selected_grid: None,
            }),
            DatumTransform::Helmert3(_) | DatumTransform::Helmert7(_) => {
                let (x, y, z) = geodetic_to_ecef(lat_rad, lon_rad, h, &self.ellipsoid);
                let (xw, yw, zw) = self.to_wgs84_ecef(x, y, z)?;
                let (lat, lon, hgt) = ecef_to_geodetic(xw, yw, zw, &Ellipsoid::WGS84);
                Ok(DatumGeodeticTrace {
                    lat_rad: lat,
                    lon_rad: lon,
                    h: hgt,
                    selected_grid: None,
                })
            }
            DatumTransform::GridShift { grid_name } => {
                let shifted = self.apply_grid_shift_to_wgs84(lat_rad, lon_rad, h, grid_name);
                match (shifted, policy) {
                    (Ok((lat, lon, hgt)), _) => Ok(DatumGeodeticTrace {
                        lat_rad: lat,
                        lon_rad: lon,
                        h: hgt,
                        selected_grid: Some((*grid_name).to_string()),
                    }),
                    (Err(e), DatumTransformPolicy::Strict) => Err(e),
                    (Err(_), DatumTransformPolicy::FallbackToIdentityGridShift) => {
                        Ok(DatumGeodeticTrace {
                            lat_rad,
                            lon_rad,
                            h,
                            selected_grid: None,
                        })
                    }
                }
            }
            DatumTransform::Ntv2Hierarchy { dataset_name } => {
                let lon_deg = lon_rad.to_degrees();
                let lat_deg = lat_rad.to_degrees();
                let resolved = resolve_ntv2_hierarchy_grid(dataset_name, lon_deg, lat_deg)?;
                let selected_grid = resolved.clone();
                let shifted = match resolved {
                    Some(grid_name) => {
                        self.apply_grid_shift_to_wgs84(lat_rad, lon_rad, h, &grid_name)
                    }
                    None => Err(ProjectionError::DatumError(format!(
                        "NTv2 hierarchy dataset '{dataset_name}' has no matching subgrid for ({lon_deg}, {lat_deg})"
                    ))),
                };

                match (shifted, policy) {
                    (Ok((lat, lon, hgt)), _) => Ok(DatumGeodeticTrace {
                        lat_rad: lat,
                        lon_rad: lon,
                        h: hgt,
                        selected_grid,
                    }),
                    (Err(e), DatumTransformPolicy::Strict) => Err(e),
                    (Err(_), DatumTransformPolicy::FallbackToIdentityGridShift) => {
                        Ok(DatumGeodeticTrace {
                            lat_rad,
                            lon_rad,
                            h,
                            selected_grid: None,
                        })
                    }
                }
            }
        }
    }

    /// Transform geodetic coordinates from WGS84 into this datum with policy control.
    pub fn from_wgs84_geodetic_with_policy(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        policy: DatumTransformPolicy,
    ) -> Result<(f64, f64, f64)> {
        let trace = self.from_wgs84_geodetic_with_policy_and_trace(lat_rad, lon_rad, h, policy)?;
        Ok((trace.lat_rad, trace.lon_rad, trace.h))
    }

    pub(crate) fn from_wgs84_geodetic_with_policy_and_trace(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
        policy: DatumTransformPolicy,
    ) -> Result<DatumGeodeticTrace> {
        match &self.transform {
            DatumTransform::None => Ok(DatumGeodeticTrace {
                lat_rad,
                lon_rad,
                h,
                selected_grid: None,
            }),
            DatumTransform::Helmert3(_) | DatumTransform::Helmert7(_) => {
                let (x, y, z) = geodetic_to_ecef(lat_rad, lon_rad, h, &Ellipsoid::WGS84);
                let (xt, yt, zt) = self.from_wgs84_ecef(x, y, z)?;
                let (lat, lon, hgt) = ecef_to_geodetic(xt, yt, zt, &self.ellipsoid);
                Ok(DatumGeodeticTrace {
                    lat_rad: lat,
                    lon_rad: lon,
                    h: hgt,
                    selected_grid: None,
                })
            }
            DatumTransform::GridShift { grid_name } => {
                let shifted = self.apply_grid_shift_from_wgs84(lat_rad, lon_rad, h, grid_name);
                match (shifted, policy) {
                    (Ok((lat, lon, hgt)), _) => Ok(DatumGeodeticTrace {
                        lat_rad: lat,
                        lon_rad: lon,
                        h: hgt,
                        selected_grid: Some((*grid_name).to_string()),
                    }),
                    (Err(e), DatumTransformPolicy::Strict) => Err(e),
                    (Err(_), DatumTransformPolicy::FallbackToIdentityGridShift) => {
                        Ok(DatumGeodeticTrace {
                            lat_rad,
                            lon_rad,
                            h,
                            selected_grid: None,
                        })
                    }
                }
            }
            DatumTransform::Ntv2Hierarchy { dataset_name } => {
                let lon_deg = lon_rad.to_degrees();
                let lat_deg = lat_rad.to_degrees();
                let resolved = resolve_ntv2_hierarchy_grid(dataset_name, lon_deg, lat_deg)?;
                let selected_grid = resolved.clone();
                let shifted = match resolved {
                    Some(grid_name) => {
                        self.apply_grid_shift_from_wgs84(lat_rad, lon_rad, h, &grid_name)
                    }
                    None => Err(ProjectionError::DatumError(format!(
                        "NTv2 hierarchy dataset '{dataset_name}' has no matching subgrid for ({lon_deg}, {lat_deg})"
                    ))),
                };

                match (shifted, policy) {
                    (Ok((lat, lon, hgt)), _) => Ok(DatumGeodeticTrace {
                        lat_rad: lat,
                        lon_rad: lon,
                        h: hgt,
                        selected_grid,
                    }),
                    (Err(e), DatumTransformPolicy::Strict) => Err(e),
                    (Err(_), DatumTransformPolicy::FallbackToIdentityGridShift) => {
                        Ok(DatumGeodeticTrace {
                            lat_rad,
                            lon_rad,
                            h,
                            selected_grid: None,
                        })
                    }
                }
            }
        }
    }

    /// Transform geodetic coordinates from this datum to WGS84.
    pub fn to_wgs84_geodetic(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
    ) -> Result<(f64, f64, f64)> {
        self.to_wgs84_geodetic_with_policy(lat_rad, lon_rad, h, DatumTransformPolicy::Strict)
    }

    /// Transform geodetic coordinates from WGS84 into this datum.
    pub fn from_wgs84_geodetic(
        &self,
        lat_rad: f64,
        lon_rad: f64,
        h: f64,
    ) -> Result<(f64, f64, f64)> {
        self.from_wgs84_geodetic_with_policy(lat_rad, lon_rad, h, DatumTransformPolicy::Strict)
    }

    /// Transform ECEF coordinates from this datum into WGS84 ECEF.
    pub fn to_wgs84_ecef(&self, x: f64, y: f64, z: f64) -> Result<(f64, f64, f64)> {
        match &self.transform {
            DatumTransform::None => Ok((x, y, z)),
            DatumTransform::Helmert3(params) | DatumTransform::Helmert7(params) => {
                Ok(params.apply(x, y, z))
            }
            DatumTransform::GridShift { grid_name } | DatumTransform::Ntv2Hierarchy { dataset_name: grid_name } => Err(ProjectionError::DatumError(
                format!("grid-shift transform '{grid_name}' not implemented"),
            )),
        }
    }

    pub(crate) fn supports_ecef_batch_simd(&self) -> bool {
        match self.transform {
            DatumTransform::Helmert3(_) | DatumTransform::Helmert7(_) => true,
            DatumTransform::None => self.ellipsoid == Ellipsoid::WGS84,
            DatumTransform::GridShift { .. } | DatumTransform::Ntv2Hierarchy { .. } => false,
        }
    }

    pub(crate) fn to_wgs84_ecef_batch4(
        &self,
        x4: &[f64; 4],
        y4: &[f64; 4],
        z4: &[f64; 4],
    ) -> Result<([f64; 4], [f64; 4], [f64; 4])> {
        match &self.transform {
            DatumTransform::None => Ok((*x4, *y4, *z4)),
            DatumTransform::Helmert3(params) | DatumTransform::Helmert7(params) => {
                Ok(params.apply_simd_batch4(x4, y4, z4))
            }
            DatumTransform::GridShift { grid_name }
            | DatumTransform::Ntv2Hierarchy {
                dataset_name: grid_name,
            } => Err(ProjectionError::DatumError(format!(
                "grid-shift transform '{grid_name}' not implemented"
            ))),
        }
    }

    /// Transform ECEF coordinates from WGS84 into this datum's ECEF.
    pub fn from_wgs84_ecef(&self, x: f64, y: f64, z: f64) -> Result<(f64, f64, f64)> {
        match &self.transform {
            DatumTransform::None => Ok((x, y, z)),
            DatumTransform::Helmert3(params) | DatumTransform::Helmert7(params) => {
                Ok(params.apply_inverse(x, y, z))
            }
            DatumTransform::GridShift { grid_name } | DatumTransform::Ntv2Hierarchy { dataset_name: grid_name } => Err(ProjectionError::DatumError(
                format!("grid-shift transform '{grid_name}' not implemented"),
            )),
        }
    }

    pub(crate) fn from_wgs84_ecef_batch4(
        &self,
        x4: &[f64; 4],
        y4: &[f64; 4],
        z4: &[f64; 4],
    ) -> Result<([f64; 4], [f64; 4], [f64; 4])> {
        match &self.transform {
            DatumTransform::None => Ok((*x4, *y4, *z4)),
            DatumTransform::Helmert3(params) | DatumTransform::Helmert7(params) => {
                Ok(params.apply_inverse_simd_batch4(x4, y4, z4))
            }
            DatumTransform::GridShift { grid_name }
            | DatumTransform::Ntv2Hierarchy {
                dataset_name: grid_name,
            } => Err(ProjectionError::DatumError(format!(
                "grid-shift transform '{grid_name}' not implemented"
            ))),
        }
    }

    /// WGS 84 – World Geodetic System 1984 (GPS standard).
    pub const WGS84: Datum = Datum {
        name: "WGS 84",
        ellipsoid: Ellipsoid::WGS84,
        transform: DatumTransform::None,
    };

    /// NAD 83 – North American Datum 1983.
    pub const NAD83: Datum = Datum {
        name: "NAD 83",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 0.9956, ty: -1.9013, tz: -0.5215,
            rx: 0.025915, ry: 0.009426, rz: 0.011599,
            ds: -0.00062,
        }),
    };

    /// NAD83 (CSRS) – Canadian Spatial Reference System.
    pub const NAD83_CSRS: Datum = Datum {
        name: "NAD83 (CSRS)",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: -0.991,
            ty: 1.9072,
            tz: 0.5129,
            rx: -1.25033e-07,
            ry: -4.6785e-08,
            rz: -5.6529e-08,
            ds: 0.0,
        }),
    };

    /// NAD83(NSRS2007) – National Spatial Reference System 2007 realization.
    ///
    /// Treated as equivalent to GRS80 with no additional transform in this engine.
    pub const NAD83_NSRS2007: Datum = Datum {
        name: "NAD83(NSRS2007)",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// NAD83(HARN) – High Accuracy Reference Network realization.
    ///
    /// Treated as equivalent to GRS80 with no additional transform in this engine.
    pub const NAD83_HARN: Datum = Datum {
        name: "NAD83(HARN)",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// NAD 27 – North American Datum 1927.
    pub const NAD27: Datum = Datum {
        name: "NAD 27",
        ellipsoid: Ellipsoid::CLARKE1866,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -8.0, ty: 160.0, tz: 176.0,
            rx: 0.0, ry: 0.0, rz: 0.0,
            ds: 0.0,
        }),
    };

    /// ETRS89 – European Terrestrial Reference System 1989.
    pub const ETRS89: Datum = Datum {
        name: "ETRS 89",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// ED50 – European Datum 1950.
    pub const ED50: Datum = Datum {
        name: "ED 50",
        ellipsoid: Ellipsoid::INTERNATIONAL,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -87.0, ty: -98.0, tz: -121.0,
            rx: 0.0, ry: 0.0, rz: 0.0,
            ds: 0.0,
        }),
    };

    /// GDA94 – Geocentric Datum of Australia 1994.
    ///
    /// Treated as equivalent to WGS84 for now; a realization/epoch-aware transform
    /// can be added later for sub-meter geodetic workflows.
    pub const GDA94: Datum = Datum {
        name: "GDA94",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// GDA2020 – Geocentric Datum of Australia 2020.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const GDA2020: Datum = Datum {
        name: "GDA2020",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// CGCS2000 – China Geodetic Coordinate System 2000.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const CGCS2000: Datum = Datum {
        name: "CGCS2000",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// SIRGAS2000 – Sistema de Referencia Geocentrico para las Americas 2000.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const SIRGAS2000: Datum = Datum {
        name: "SIRGAS2000",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// New Beijing datum.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const NEW_BEIJING: Datum = Datum {
        name: "New Beijing",
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::None,
    };

    /// Xian 1980 geodetic datum.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const XIAN_1980: Datum = Datum {
        name: "Xian 1980",
        ellipsoid: Ellipsoid::IAU1976,
        transform: DatumTransform::None,
    };

    /// Antigua 1943 geodetic datum.
    pub const ANTIGUA_1943: Datum = Datum {
        name: "Antigua 1943",
        ellipsoid: Ellipsoid::CLARKE1880_RGS,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -255.0,
            ty: -15.0,
            tz: 71.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Dominica 1945 geodetic datum.
    pub const DOMINICA_1945: Datum = Datum {
        name: "Dominica 1945",
        ellipsoid: Ellipsoid::CLARKE1880_RGS,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 725.0,
            ty: 685.0,
            tz: 536.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Grenada 1953 geodetic datum.
    pub const GRENADA_1953: Datum = Datum {
        name: "Grenada 1953",
        ellipsoid: Ellipsoid::CLARKE1880_RGS,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 72.0,
            ty: 213.7,
            tz: 93.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Montserrat 1958 geodetic datum.
    pub const MONTSERRAT_1958: Datum = Datum {
        name: "Montserrat 1958",
        ellipsoid: Ellipsoid::CLARKE1880_RGS,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 174.0,
            ty: 359.0,
            tz: 365.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// St. Kitts 1955 geodetic datum.
    pub const ST_KITTS_1955: Datum = Datum {
        name: "St. Kitts 1955",
        ellipsoid: Ellipsoid::CLARKE1880_RGS,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 9.0,
            ty: 183.0,
            tz: 236.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// NZGD2000 – New Zealand Geodetic Datum 2000.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const NZGD2000: Datum = Datum {
        name: "NZGD2000",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// JGD2000 – Japanese Geodetic Datum 2000.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const JGD2000: Datum = Datum {
        name: "JGD2000",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// JGD2011 – Japanese Geodetic Datum 2011.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const JGD2011: Datum = Datum {
        name: "JGD2011",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// RDN2008 – Rete Dinamica Nazionale 2008.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const RDN2008: Datum = Datum {
        name: "RDN2008",
        ellipsoid: Ellipsoid::GRS80,
        transform: DatumTransform::None,
    };

    /// VN-2000 – Vietnam geodetic datum.
    ///
    /// Treated as equivalent to WGS84 for now.
    pub const VN2000: Datum = Datum {
        name: "VN-2000",
        ellipsoid: Ellipsoid::WGS84,
        transform: DatumTransform::None,
    };

    /// OSGB36 – Ordnance Survey Great Britain 1936.
    ///
    /// Uses a commonly cited 7-parameter Helmert approximation to WGS84.
    pub const OSGB36: Datum = Datum {
        name: "OSGB36",
        ellipsoid: Ellipsoid::AIRY1830,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 446.448,
            ty: -125.157,
            tz: 542.060,
            rx: 0.1502,
            ry: 0.2470,
            rz: 0.8421,
            ds: -20.4894,
        }),
    };

    /// DHDN (Potsdam) – Deutsches Hauptdreiecksnetz.
    ///
    /// Uses a commonly cited 7-parameter Helmert approximation to WGS84.
    pub const DHDN: Datum = Datum {
        name: "DHDN",
        ellipsoid: Ellipsoid::BESSEL,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 598.1,
            ty: 73.7,
            tz: 418.2,
            rx: 0.202,
            ry: 0.045,
            rz: -2.455,
            ds: 6.7,
        }),
    };

    /// Pulkovo 1942 (58) – historical realization.
    ///
    /// Uses a commonly used 3-parameter approximation to WGS84.
    pub const PULKOVO1942_58: Datum = Datum {
        name: "Pulkovo 1942(58)",
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 28.0,
            ty: -130.0,
            tz: -95.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Pulkovo 1942 (83) – historical realization.
    ///
    /// Uses a commonly used 3-parameter approximation to WGS84.
    pub const PULKOVO1942_83: Datum = Datum {
        name: "Pulkovo 1942(83)",
        ellipsoid: Ellipsoid::KRASSOWSKY1940,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 24.0,
            ty: -123.0,
            tz: -94.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// S-JTSK – Czech/Slovak geodetic datum.
    ///
    /// Uses a commonly used 3-parameter approximation to WGS84.
    pub const S_JTSK: Datum = Datum {
        name: "S-JTSK",
        ellipsoid: Ellipsoid::BESSEL,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 589.0,
            ty: 76.0,
            tz: 480.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Belge 1972 – Belgian national datum.
    ///
    /// Uses a commonly cited 7-parameter Helmert approximation to WGS84.
    pub const BELGE1972: Datum = Datum {
        name: "Belge 1972",
        ellipsoid: Ellipsoid::INTERNATIONAL,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 106.868628,
            ty: -52.297783,
            tz: 103.723893,
            rx: 0.33657,
            ry: -0.456955,
            rz: 1.842183,
            ds: -1.2747,
        }),
    };

    /// Amersfoort – Dutch national datum.
    ///
    /// Uses a commonly cited 7-parameter Helmert approximation to WGS84.
    pub const AMERSFOORT: Datum = Datum {
        name: "Amersfoort",
        ellipsoid: Ellipsoid::BESSEL,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 565.4171,
            ty: 50.3319,
            tz: 465.5524,
            rx: 1.9342,
            ry: -1.6677,
            rz: 9.1019,
            ds: 4.0725,
        }),
    };

    /// TM65 – Irish historical datum.
    ///
    /// Uses a commonly cited 7-parameter Helmert approximation to WGS84.
    pub const TM65: Datum = Datum {
        name: "TM65",
        ellipsoid: Ellipsoid::AIRY1830_MOD,
        transform: DatumTransform::Helmert7(HelmertParams {
            tx: 482.53,
            ty: -130.596,
            tz: 564.557,
            rx: -1.042,
            ry: -0.214,
            rz: -0.631,
            ds: 8.15,
        }),
    };

    /// Katanga 1955 – DR Congo regional datum.
    ///
    /// Uses a coarse 3-parameter approximation to WGS84.
    pub const KATANGA1955: Datum = Datum {
        name: "Katanga 1955",
        ellipsoid: Ellipsoid::CLARKE1866,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -150.0,
            ty: 40.0,
            tz: -200.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Cape – South African historical datum.
    ///
    /// Uses a commonly used 3-parameter approximation to WGS84.
    pub const CAPE: Datum = Datum {
        name: "Cape",
        ellipsoid: Ellipsoid::CLARKE1866,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -136.0,
            ty: -108.0,
            tz: -292.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// Puerto Rico 1927 – legacy Puerto Rico local frame.
    ///
    /// Uses a coarse regional approximation inherited from NAD27-style shifts.
    pub const PUERTO_RICO_1927: Datum = Datum {
        name: "Puerto Rico 1927",
        ellipsoid: Ellipsoid::CLARKE1866,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -8.0,
            ty: 160.0,
            tz: 176.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// St. Croix – legacy local frame for the US Virgin Islands.
    ///
    /// Uses a coarse regional approximation inherited from NAD27-style shifts.
    pub const ST_CROIX: Datum = Datum {
        name: "St. Croix",
        ellipsoid: Ellipsoid::CLARKE1866,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: -8.0,
            ty: 160.0,
            tz: 176.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// CH1903 – Swiss legacy geodetic datum.
    pub const CH1903: Datum = Datum {
        name: "CH1903",
        ellipsoid: Ellipsoid::BESSEL,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 674.374,
            ty: 15.056,
            tz: 405.346,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// CH1903+ – Swiss updated local frame.
    pub const CH1903_PLUS: Datum = Datum {
        name: "CH1903+",
        ellipsoid: Ellipsoid::BESSEL,
        transform: DatumTransform::Helmert3(HelmertParams {
            tx: 674.374,
            ty: 15.056,
            tz: 405.346,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            ds: 0.0,
        }),
    };

    /// South East Island 1943 datum.
    ///
    /// Currently treated as WGS84-equivalent pending authoritative local transform parameters.
    pub const SOUTH_EAST_ISLAND_1943: Datum = Datum {
        name: "South East Island 1943",
        ellipsoid: Ellipsoid::WGS84,
        transform: DatumTransform::None,
    };

    /// SVY21 – Singapore geodetic frame used with Singapore TM.
    ///
    /// Currently treated as WGS84-equivalent for this library's supported scope.
    pub const SVY21: Datum = Datum {
        name: "SVY21",
        ellipsoid: Ellipsoid::WGS84,
        transform: DatumTransform::None,
    };
}

impl Default for Datum {
    fn default() -> Self {
        Datum::WGS84.clone()
    }
}

impl std::fmt::Display for Datum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.ellipsoid.name)
    }
}
