//! Point-cloud reprojection helpers powered by `wbprojection`.

use wbprojection::Crs as ProjCrs;

use crate::crs::Crs;
use crate::error::{Error, Result};
use crate::point::PointRecord;

/// Behavior when a point fails reprojection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformFailurePolicy {
    /// Abort and return the first reprojection error.
    Error,
    /// Keep the point but set `x`/`y` to `NaN`.
    SetNaN,
    /// Drop the failed point from output.
    SkipPoint,
}

/// Options for LiDAR reprojection operations.
#[derive(Debug, Clone, Copy)]
pub struct LidarReprojectOptions {
    /// Policy applied when a point transformation fails.
    pub failure_policy: TransformFailurePolicy,
    /// When `true`, use 3D reprojection and update `z` values.
    pub use_3d_transform: bool,
}

impl Default for LidarReprojectOptions {
    fn default() -> Self {
        Self {
            failure_policy: TransformFailurePolicy::Error,
            use_3d_transform: false,
        }
    }
}

impl LidarReprojectOptions {
    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set transform failure policy.
    pub fn with_failure_policy(mut self, policy: TransformFailurePolicy) -> Self {
        self.failure_policy = policy;
        self
    }

    /// Enable/disable 3D reprojection (`x`, `y`, and `z`).
    ///
    /// Uses `wbprojection`'s preserve-horizontal 3D transform behavior.
    pub fn with_3d_transform(mut self, enabled: bool) -> Self {
        self.use_3d_transform = enabled;
        self
    }
}

/// Reproject points to a destination EPSG using source CRS metadata.
///
/// Source EPSG is read from `src_crs.epsg`.
pub fn points_to_epsg(points: &[PointRecord], src_crs: &Crs, dst_epsg: u32) -> Result<Vec<PointRecord>> {
    points_to_epsg_with_options(points, src_crs, dst_epsg, &LidarReprojectOptions::default())
}

/// Reproject points to a destination EPSG using source CRS metadata with options.
///
/// Source CRS is resolved from `src_crs.epsg` or `src_crs.wkt`.
pub fn points_to_epsg_with_options(
    points: &[PointRecord],
    src_crs: &Crs,
    dst_epsg: u32,
    options: &LidarReprojectOptions,
) -> Result<Vec<PointRecord>> {
    let src = source_proj_crs(src_crs, "points_to_epsg")?;
    let dst = ProjCrs::from_epsg(dst_epsg)
        .map_err(|e| Error::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;
    points_with_crs_options(points, &src, &dst, options)
}

/// Reproject points to a destination EPSG using source CRS metadata with
/// options and progress updates in the range [0, 1] as points are completed.
pub fn points_to_epsg_with_options_and_progress<F>(
    points: &[PointRecord],
    src_crs: &Crs,
    dst_epsg: u32,
    options: &LidarReprojectOptions,
    progress: F,
) -> Result<Vec<PointRecord>>
where
    F: Fn(f64) + Send + Sync,
{
    let src = source_proj_crs(src_crs, "points_to_epsg")?;
    let dst = ProjCrs::from_epsg(dst_epsg)
        .map_err(|e| Error::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;
    points_with_crs_options_internal(points, &src, &dst, options, Some(&progress))
}

/// Reproject points to a destination EPSG and return updated destination CRS metadata.
///
/// Source EPSG is read from `src_crs.epsg`.
pub fn points_to_epsg_with_output_crs(
    points: &[PointRecord],
    src_crs: &Crs,
    dst_epsg: u32,
) -> Result<(Vec<PointRecord>, Crs)> {
    points_to_epsg_with_output_crs_options(
        points,
        src_crs,
        dst_epsg,
        &LidarReprojectOptions::default(),
    )
}

/// Reproject points to a destination EPSG and return updated destination CRS metadata with options.
///
/// Source EPSG is read from `src_crs.epsg`.
pub fn points_to_epsg_with_output_crs_options(
    points: &[PointRecord],
    src_crs: &Crs,
    dst_epsg: u32,
    options: &LidarReprojectOptions,
) -> Result<(Vec<PointRecord>, Crs)> {
    let out = points_to_epsg_with_options(points, src_crs, dst_epsg, options)?;
    Ok((out, Crs::from_epsg(dst_epsg)))
}

/// Reproject points between explicit EPSG codes.
pub fn points_from_to_epsg(points: &[PointRecord], src_epsg: u32, dst_epsg: u32) -> Result<Vec<PointRecord>> {
    points_from_to_epsg_with_options(
        points,
        src_epsg,
        dst_epsg,
        &LidarReprojectOptions::default(),
    )
}

/// Reproject points between explicit EPSG codes with options.
pub fn points_from_to_epsg_with_options(
    points: &[PointRecord],
    src_epsg: u32,
    dst_epsg: u32,
    options: &LidarReprojectOptions,
) -> Result<Vec<PointRecord>> {
    let src = ProjCrs::from_epsg(src_epsg)
        .map_err(|e| Error::Projection(format!("invalid source EPSG {src_epsg}: {e}")))?;
    let dst = ProjCrs::from_epsg(dst_epsg)
        .map_err(|e| Error::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;
    points_with_crs_options(points, &src, &dst, options)
}

/// Reproject points between caller-supplied CRS objects.
pub fn points_with_crs(points: &[PointRecord], src: &ProjCrs, dst: &ProjCrs) -> Result<Vec<PointRecord>> {
    points_with_crs_options(points, src, dst, &LidarReprojectOptions::default())
}

/// Reproject points between caller-supplied CRS objects with options.
pub fn points_with_crs_options(
    points: &[PointRecord],
    src: &ProjCrs,
    dst: &ProjCrs,
    options: &LidarReprojectOptions,
) -> Result<Vec<PointRecord>> {
    points_with_crs_options_internal(points, src, dst, options, None)
}

/// Reproject points between caller-supplied CRS objects with options and
/// progress updates in the range [0, 1] as points are completed.
pub fn points_with_crs_options_and_progress<F>(
    points: &[PointRecord],
    src: &ProjCrs,
    dst: &ProjCrs,
    options: &LidarReprojectOptions,
    progress: F,
) -> Result<Vec<PointRecord>>
where
    F: Fn(f64) + Send + Sync,
{
    points_with_crs_options_internal(points, src, dst, options, Some(&progress))
}

fn points_with_crs_options_internal(
    points: &[PointRecord],
    src: &ProjCrs,
    dst: &ProjCrs,
    options: &LidarReprojectOptions,
    progress: Option<&(dyn Fn(f64) + Send + Sync)>,
) -> Result<Vec<PointRecord>> {
    let mut out = Vec::with_capacity(points.len());
    let total_points = points.len();

    for (index, p) in points.iter().enumerate() {
        let transformed = if options.use_3d_transform {
            src.transform_to_3d_preserve_horizontal(p.x, p.y, p.z, dst)
                .map(|(x, y, z)| (x, y, Some(z)))
        } else {
            src.transform_to(p.x, p.y, dst)
                .map(|(x, y)| (x, y, None))
        };

        match transformed {
            Ok((x, y, z_opt)) => {
                let mut q = *p;
                q.x = x;
                q.y = y;
                if let Some(z) = z_opt {
                    q.z = z;
                }
                out.push(q);
            }
            Err(err) => match options.failure_policy {
                TransformFailurePolicy::Error => {
                    return Err(Error::Projection(format!("point reprojection failed: {err}")));
                }
                TransformFailurePolicy::SetNaN => {
                    let mut q = *p;
                    q.x = f64::NAN;
                    q.y = f64::NAN;
                    if options.use_3d_transform {
                        q.z = f64::NAN;
                    }
                    out.push(q);
                }
                TransformFailurePolicy::SkipPoint => {}
            },
        }

        if let Some(progress_cb) = progress {
            progress_cb((index + 1) as f64 / total_points.max(1) as f64);
        }
    }

    if let Some(progress_cb) = progress {
        progress_cb(1.0);
    }

    Ok(out)
}

/// Reproject points in-place between explicit EPSG codes.
pub fn points_in_place_from_to_epsg(points: &mut [PointRecord], src_epsg: u32, dst_epsg: u32) -> Result<()> {
    let src = ProjCrs::from_epsg(src_epsg)
        .map_err(|e| Error::Projection(format!("invalid source EPSG {src_epsg}: {e}")))?;
    let dst = ProjCrs::from_epsg(dst_epsg)
        .map_err(|e| Error::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;

    for p in points.iter_mut() {
        let (x, y) = src
            .transform_to(p.x, p.y, &dst)
            .map_err(|e| Error::Projection(format!("point reprojection failed: {e}")))?;
        p.x = x;
        p.y = y;
    }
    Ok(())
}

/// Reproject points in-place to a destination EPSG using source CRS metadata,
/// then update that metadata to the destination CRS.
///
/// Source CRS is resolved from `src_crs.epsg` or `src_crs.wkt`.
pub fn points_in_place_to_epsg(
    points: &mut [PointRecord],
    src_crs: &mut Crs,
    dst_epsg: u32,
) -> Result<()> {
    points_in_place_to_epsg_with_options(
        points,
        src_crs,
        dst_epsg,
        &LidarReprojectOptions::default(),
    )
}

/// Reproject points in-place to a destination EPSG using source CRS metadata
/// and options, then update CRS metadata to the destination CRS.
pub fn points_in_place_to_epsg_with_options(
    points: &mut [PointRecord],
    src_crs: &mut Crs,
    dst_epsg: u32,
    options: &LidarReprojectOptions,
) -> Result<()> {
    let src = source_proj_crs(src_crs, "points_in_place_to_epsg")?;
    let dst = ProjCrs::from_epsg(dst_epsg)
        .map_err(|e| Error::Projection(format!("invalid destination EPSG {dst_epsg}: {e}")))?;

    for p in points.iter_mut() {
        let (x, y, z_opt) = if options.use_3d_transform {
            src.transform_to_3d_preserve_horizontal(p.x, p.y, p.z, &dst)
                .map(|(x, y, z)| (x, y, Some(z)))
                .map_err(|e| Error::Projection(format!("point reprojection failed: {e}")))?
        } else {
            src.transform_to(p.x, p.y, &dst)
                .map(|(x, y)| (x, y, None))
                .map_err(|e| Error::Projection(format!("point reprojection failed: {e}")))?
        };
        p.x = x;
        p.y = y;
        if let Some(z) = z_opt {
            p.z = z;
        }
    }

    *src_crs = Crs::from_epsg(dst_epsg);
    Ok(())
}

fn source_proj_crs(src_crs: &Crs, op_name: &str) -> Result<ProjCrs> {
    if let Some(src_epsg) = src_crs.epsg {
        return ProjCrs::from_epsg(src_epsg)
            .map_err(|e| Error::Projection(format!("invalid source EPSG {src_epsg}: {e}")));
    }

    if let Some(wkt) = src_crs.wkt.as_deref() {
        let trimmed = wkt.trim();
        if !trimmed.is_empty() {
            return wbprojection::from_wkt(trimmed)
                .map_err(|e| Error::Projection(format!("invalid source CRS WKT: {e}")));
        }
    }

    Err(Error::Projection(format!(
        "{op_name} requires source CRS metadata in src_crs (EPSG or WKT)"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn reprojects_4326_to_3857() {
        let points = vec![PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() }];
        let out = points_from_to_epsg(&points, 4326, 3857).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].x.abs() > 1000.0);
        assert!(out[0].y.abs() > 100.0);
    }

    #[test]
    fn points_to_epsg_requires_source_epsg() {
        let points = vec![PointRecord::default()];
        let src = Crs::new();
        let err = points_to_epsg(&points, &src, 3857).unwrap_err();
        assert!(matches!(err, Error::Projection(_)));
        assert!(err
            .to_string()
            .contains("points_to_epsg requires source CRS metadata"));
    }

    #[test]
    fn points_to_epsg_accepts_source_wkt_without_epsg() {
        let points = vec![PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() }];
        let src = Crs::new().with_wkt(
            "GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],CS[ellipsoidal,2],AXIS[\"Geodetic latitude (Lat)\",north],AXIS[\"Geodetic longitude (Lon)\",east],UNIT[\"degree\",0.0174532925199433],ID[\"EPSG\",4326]]"
        );
        let out = points_to_epsg(&points, &src, 3857).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].x.abs() > 1000.0);
        assert!(out[0].y.abs() > 100.0);
    }

    #[test]
    fn points_to_epsg_with_3d_option_preserves_z_path() {
        let points = vec![PointRecord { x: -2.0, y: -0.5, z: 123.4, ..PointRecord::default() }];
        let src = Crs::from_epsg(4326);
        let opts = LidarReprojectOptions::new().with_3d_transform(true);
        let out = points_to_epsg_with_options(&points, &src, 3857, &opts).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].x.abs() > 1000.0);
        assert!(out[0].y.abs() > 100.0);
        assert!(out[0].z.is_finite());
    }

    #[test]
    fn points_to_epsg_updates_coords() {
        let points = vec![PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() }];
        let src = Crs::from_epsg(4326);
        let out = points_to_epsg(&points, &src, 3857).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].x.abs() > 1000.0);
        assert!(out[0].y.abs() > 100.0);
    }

    #[test]
    fn points_to_epsg_with_output_crs_returns_dst_crs() {
        let points = vec![PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() }];
        let src = Crs::from_epsg(4326);
        let (out, dst_crs) = points_to_epsg_with_output_crs(&points, &src, 3857).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(dst_crs.epsg, Some(3857));
    }

    #[test]
    fn points_in_place_to_epsg_updates_points_and_crs() {
        let mut points = vec![PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() }];
        let mut crs = Crs::from_epsg(4326);
        points_in_place_to_epsg(&mut points, &mut crs, 3857).unwrap();
        assert_eq!(crs.epsg, Some(3857));
        assert!(points[0].x.abs() > 1000.0);
        assert!(points[0].y.abs() > 100.0);
    }

    #[test]
    fn points_in_place_to_epsg_requires_source_epsg() {
        let mut points = vec![PointRecord::default()];
        let mut crs = Crs::new();
        let err = points_in_place_to_epsg(&mut points, &mut crs, 3857).unwrap_err();
        assert!(matches!(err, Error::Projection(_)));
        assert!(err
            .to_string()
            .contains("points_in_place_to_epsg requires source CRS metadata"));
    }

    #[test]
    fn points_to_epsg_with_progress_emits_point_updates() {
        let points = vec![
            PointRecord { x: -2.0, y: -0.5, ..PointRecord::default() },
            PointRecord { x: -1.5, y: -0.4, ..PointRecord::default() },
            PointRecord { x: -1.0, y: -0.3, ..PointRecord::default() },
        ];
        let src = Crs::from_epsg(4326);

        let progress_values: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&progress_values);

        let out = points_to_epsg_with_options_and_progress(
            &points,
            &src,
            3857,
            &LidarReprojectOptions::default(),
            move |pct| {
                sink.lock().unwrap().push(pct);
            },
        )
        .unwrap();

        let values = progress_values.lock().unwrap();
        assert_eq!(out.len(), points.len());
        assert!(!values.is_empty());
        assert_eq!(values.len(), points.len() + 1);
        assert!(values.iter().all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0));
        assert!((values.last().copied().unwrap() - 1.0).abs() < 1e-12);
    }
}
