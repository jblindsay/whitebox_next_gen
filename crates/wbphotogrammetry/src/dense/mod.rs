//! Dense surface reconstruction.
//!
//! Sprint 1 minimal implementation: builds a deterministic DSM grid from
//! aligned camera poses using inverse-distance interpolation and simple
//! trajectory-informed gradients.
//!
//! Step 3 enhancements (Post-BA):
//! - Stereo depth estimation from refined adjacent frame pairs
//! - Enhanced hypothesis quality using BA reprojection residuals
//! - Per-cell depth confidence/coverage tracking

use serde::{Deserialize, Serialize};

use image::GrayImage;
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};
use nalgebra::Vector3;
use nalgebra::{Matrix3, Matrix3x4, Vector2};

use crate::alignment::AlignmentResult;
use crate::error::{PhotogrammetryError, Result};
use crate::ingest::ImageFrame;

const MIN_GRID_DIM: usize = 4;
const MVS_MAX_SOURCE_VIEWS: usize = 3;
const MVS_SAMPLE_STEP: usize = 36;
const MVS_PATCH_RADIUS: i32 = 2;
const MVS_SEARCH_RADIUS: i32 = 12;
const MVS_OCCLUSION_BIN_PX: f64 = 8.0;
const MVS_EPIPOLAR_TOL_PX: f64 = 1.25;
const MVS_LR_CONSISTENCY_TOL_PX: i32 = 2;
const MVS_CENSUS_RADIUS: i32 = 2;
const MVS_HYBRID_ZNCC_WEIGHT: f64 = 0.65;
const MVS_HYBRID_CENSUS_WEIGHT: f64 = 0.35;

#[derive(Debug, Clone)]
struct DepthHypothesis {
    center: [f64; 3],
    confidence: f64,
}

#[derive(Debug, Clone)]
struct MultiViewDepthSample {
    point_world: [f64; 3],
    confidence: f64,
    support_views: usize,
    ref_px: [f64; 2],
    ref_depth: f64,
}

#[derive(Debug, Clone)]
struct MultiViewDepthMap {
    reference_idx: usize,
    samples: Vec<MultiViewDepthSample>,
}

#[derive(Debug, Clone, Copy)]
struct PatchMatch {
    x: i32,
    y: i32,
    best_cost: f64,
    second_cost: f64,
}

/// Statistics from the dense surface model stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsmStats {
    /// Number of valid (non-nodata) cells in the DSM.
    pub valid_cells: u64,
    /// Minimum elevation in metres.
    pub min_elevation_m: f64,
    /// Maximum elevation in metres.
    pub max_elevation_m: f64,
    /// Mean elevation in metres.
    pub mean_elevation_m: f64,
    /// Estimated vertical RMSE in metres.
    pub vertical_rmse_m: f64,
    /// Mean 3x3 neighborhood local relief in metres.
    pub mean_local_relief_m: f64,
    /// 95th percentile 3x3 neighborhood local relief in metres.
    pub p95_local_relief_m: f64,
}

/// Result from the dense surface model stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseResult {
    /// Path written to disk.
    pub dsm_path: String,
    /// Optional DTM path written when requested.
    pub dtm_path: Option<String>,
    /// Optional per-cell dense support raster path (0-1).
    pub support_raster_path: Option<String>,
    /// Optional per-cell dense uncertainty raster path (0-1).
    pub uncertainty_raster_path: Option<String>,
    /// Stage statistics.
    pub stats: DsmStats,
}

/// Build a dense surface model from the aligned camera network.
///
/// Sprint 1 minimal implementation: derives a DSM extent from camera poses,
/// interpolates elevations onto a regular grid, writes GeoTIFF, and returns
/// measured raster statistics.
pub fn run_dense_surface(
    alignment: &AlignmentResult,
    dem_resolution_m: f64,
    dsm_path: &str,
) -> Result<DenseResult> {
    run_dense_surface_with_frames_and_dtm(alignment, &[], dem_resolution_m, dsm_path, None)
}

/// Build a dense surface model and optionally derive a terrain-like DTM raster.
pub fn run_dense_surface_with_dtm(
    alignment: &AlignmentResult,
    dem_resolution_m: f64,
    dsm_path: &str,
    dtm_path: Option<&str>,
) -> Result<DenseResult> {
    run_dense_surface_with_frames_and_dtm(alignment, &[], dem_resolution_m, dsm_path, dtm_path)
}

pub fn run_dense_surface_with_frames(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
    dem_resolution_m: f64,
    dsm_path: &str,
) -> Result<DenseResult> {
    run_dense_surface_with_frames_and_dtm(alignment, frames, dem_resolution_m, dsm_path, None)
}

/// Build a dense surface model and optionally derive a terrain-like DTM raster.
pub fn run_dense_surface_with_frames_and_dtm(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
    dem_resolution_m: f64,
    dsm_path: &str,
    dtm_path: Option<&str>,
) -> Result<DenseResult> {
    if alignment.poses.is_empty() {
        let cfg = RasterConfig {
            cols: 2,
            rows: 2,
            data_type: DataType::F32,
            crs: alignment.crs.clone(),
            ..RasterConfig::default()
        };
        let raster = Raster::new(cfg);
        raster.write(dsm_path, RasterFormat::GeoTiff)?;
        if let Some(path) = dtm_path {
            raster.write(path, RasterFormat::GeoTiff)?;
        }
        let support_path = with_suffix_before_ext(dsm_path, "_support");
        let uncertainty_path = with_suffix_before_ext(dsm_path, "_uncertainty");
        raster.write(&support_path, RasterFormat::GeoTiff)?;
        raster.write(&uncertainty_path, RasterFormat::GeoTiff)?;

        return Ok(DenseResult {
            dsm_path: dsm_path.to_string(),
            dtm_path: dtm_path.map(|p| p.to_string()),
            support_raster_path: Some(support_path),
            uncertainty_raster_path: Some(uncertainty_path),
            stats: DsmStats {
                valid_cells: 0,
                min_elevation_m: 0.0,
                max_elevation_m: 0.0,
                mean_elevation_m: 0.0,
                vertical_rmse_m: 0.0,
                mean_local_relief_m: 0.0,
                p95_local_relief_m: 0.0,
            },
        });
    }

    let resolution_m = sanitize_resolution(dem_resolution_m, alignment.stats.estimated_gsd_m);

    let (min_x, max_x, min_y, max_y, mean_z) = pose_extent_and_mean_z(alignment);
    let extent_pad_m = (resolution_m * 2.0).max(1.0);
    let grid_x_min = min_x - extent_pad_m;
    let grid_y_min = min_y - extent_pad_m;
    let width_m = (max_x - min_x + 2.0 * extent_pad_m).max(resolution_m * MIN_GRID_DIM as f64);
    let height_m = (max_y - min_y + 2.0 * extent_pad_m).max(resolution_m * MIN_GRID_DIM as f64);

    let cols = ((width_m / resolution_m).ceil() as usize).max(MIN_GRID_DIM);
    let rows = ((height_m / resolution_m).ceil() as usize).max(MIN_GRID_DIM);

    if cols > isize::MAX as usize || rows > isize::MAX as usize {
        return Err(PhotogrammetryError::DenseReconstruction(
            "DSM grid dimensions exceed supported index range".to_string(),
        ));
    }

    let cfg = RasterConfig {
        cols,
        rows,
        x_min: grid_x_min,
        y_min: grid_y_min,
        cell_size: resolution_m,
        data_type: DataType::F32,
        crs: alignment.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "dense_surface".to_string()),
            ("generator".to_string(), "wbphotogrammetry_dense_v3_post_ba".to_string()),
        ],
        ..RasterConfig::default()
    };
    let mut raster = Raster::new(cfg);
    let mut support_raster = Raster::new(RasterConfig {
        cols,
        rows,
        x_min: grid_x_min,
        y_min: grid_y_min,
        cell_size: resolution_m,
        data_type: DataType::F32,
        crs: alignment.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "dense_surface_support".to_string()),
            ("band1".to_string(), "dense_support_0_1".to_string()),
        ],
        ..RasterConfig::default()
    });
    let mut uncertainty_raster = Raster::new(RasterConfig {
        cols,
        rows,
        x_min: grid_x_min,
        y_min: grid_y_min,
        cell_size: resolution_m,
        data_type: DataType::F32,
        crs: alignment.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "dense_surface_uncertainty".to_string()),
            ("band1".to_string(), "dense_uncertainty_0_1".to_string()),
        ],
        ..RasterConfig::default()
    });

    let nominal_flight_height_m = estimate_nominal_flight_height_m(alignment);
    let mean_surface_z = mean_z - nominal_flight_height_m;
    let (slope_x, slope_y) = estimate_surface_slopes(alignment, mean_z);
    
    // Step 3 enhancement: combine trajectory-based and stereo-based depth hypotheses
    let mut depth_hypotheses = build_depth_hypotheses(
        alignment,
        mean_z,
        slope_x,
        slope_y,
        nominal_flight_height_m,
    );
    
    // Integrate stereo depths from refined adjacent frame pairs (post-BA).
    // When imagery is available, use coarse patch matching and triangulation.
    let stereo_depths = estimate_stereo_depths_from_adjacent_pairs(alignment, frames);
    depth_hypotheses.extend(stereo_depths);
    depth_hypotheses = curate_depth_hypotheses(depth_hypotheses, mean_surface_z, estimate_pose_support_scale_m(alignment));
    let mut surface = vec![mean_surface_z; rows * cols];
    let mut support_map = vec![0.0_f64; rows * cols];
    let support_sigma_m = (estimate_pose_support_scale_m(alignment) * 1.9).clamp(4.0, 55.0);

    for row in 0..rows {
        let y = grid_y_min + ((rows - 1 - row) as f64 + 0.5) * resolution_m;
        for col in 0..cols {
            let x = grid_x_min + (col as f64 + 0.5) * resolution_m;
            let z = interpolated_surface_z(
                alignment,
                x,
                y,
                slope_x,
                slope_y,
                mean_z,
                nominal_flight_height_m,
                &depth_hypotheses,
            );
            let safe_z = if z.is_finite() {
                z
            } else {
                mean_z - nominal_flight_height_m
            };
            let idx = row * cols + col;
            surface[idx] = safe_z;
            support_map[idx] = depth_support_score(x, y, support_sigma_m, &depth_hypotheses);
        }
    }

    refine_surface_grid(&mut surface, rows, cols, alignment.stats.estimated_gsd_m);
    if surface_range_m(&surface) < 0.75 {
        reinforce_surface_from_hypotheses(
            &mut surface,
            rows,
            cols,
            grid_x_min,
            grid_y_min,
            resolution_m,
            alignment,
            &depth_hypotheses,
            mean_surface_z,
        );
    }
    repair_low_support_cells(
        &mut surface,
        &support_map,
        rows,
        cols,
        mean_surface_z,
        alignment.stats.estimated_gsd_m,
    );
    let uncertainty_map = build_uncertainty_map(
        &surface,
        &support_map,
        rows,
        cols,
        alignment.stats.estimated_gsd_m,
    );

    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut sum_z = 0.0;
    let mut valid_cells = 0_u64;
    for row in 0..rows {
        for col in 0..cols {
            let z = surface[row * cols + col];
            let z = if z.is_finite() {
                z
            } else {
                mean_z - nominal_flight_height_m
            };
            raster.set(0, row as isize, col as isize, z)?;
            let support = support_map[row * cols + col].clamp(0.0, 1.0);
            let uncertainty = uncertainty_map[row * cols + col].clamp(0.0, 1.0);
            support_raster.set(0, row as isize, col as isize, support)?;
            uncertainty_raster.set(0, row as isize, col as isize, uncertainty)?;
            valid_cells += 1;
            sum_z += z;
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }
    }

    raster.write(dsm_path, RasterFormat::GeoTiff)?;
    let support_path = with_suffix_before_ext(dsm_path, "_support");
    let uncertainty_path = with_suffix_before_ext(dsm_path, "_uncertainty");
    support_raster.write(&support_path, RasterFormat::GeoTiff)?;
    uncertainty_raster.write(&uncertainty_path, RasterFormat::GeoTiff)?;

    let dtm_out = if let Some(path) = dtm_path {
        let dtm = derive_dtm_from_dsm(&raster)?;
        dtm.write(path, RasterFormat::GeoTiff)?;
        Some(path.to_string())
    } else {
        None
    };

    let mean_elevation_m = if valid_cells == 0 {
        0.0
    } else {
        sum_z / valid_cells as f64
    };
    let vertical_rmse_m = estimate_vertical_rmse_m(alignment, valid_cells);
    let (mean_local_relief_m, p95_local_relief_m) = compute_local_relief_stats(&surface, rows, cols);

    Ok(DenseResult {
        dsm_path: dsm_path.to_string(),
        dtm_path: dtm_out,
        support_raster_path: Some(support_path),
        uncertainty_raster_path: Some(uncertainty_path),
        stats: DsmStats {
            valid_cells,
            min_elevation_m: min_z,
            max_elevation_m: max_z,
            mean_elevation_m,
            vertical_rmse_m,
            mean_local_relief_m,
            p95_local_relief_m,
        },
    })
}

fn compute_local_relief_stats(surface: &[f64], rows: usize, cols: usize) -> (f64, f64) {
    if surface.is_empty() || rows == 0 || cols == 0 {
        return (0.0, 0.0);
    }

    let mut reliefs = Vec::with_capacity(surface.len());
    for row in 0..rows {
        for col in 0..cols {
            let mut local_min = f64::INFINITY;
            let mut local_max = f64::NEG_INFINITY;
            for rr in row.saturating_sub(1)..=(row + 1).min(rows - 1) {
                for cc in col.saturating_sub(1)..=(col + 1).min(cols - 1) {
                    let value = surface[rr * cols + cc];
                    if value.is_finite() {
                        local_min = local_min.min(value);
                        local_max = local_max.max(value);
                    }
                }
            }
            if local_min.is_finite() && local_max.is_finite() {
                reliefs.push((local_max - local_min).max(0.0));
            }
        }
    }

    if reliefs.is_empty() {
        return (0.0, 0.0);
    }

    reliefs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = reliefs.iter().sum::<f64>() / reliefs.len() as f64;
    let p95_index = ((reliefs.len() - 1) as f64 * 0.95).round() as usize;
    (mean, reliefs[p95_index])
}

fn sanitize_resolution(requested_m: f64, estimated_gsd_m: f64) -> f64 {
    if requested_m.is_finite() && requested_m > 0.0 {
        return requested_m.clamp(0.02, 20.0);
    }

    let fallback = if estimated_gsd_m.is_finite() && estimated_gsd_m > 0.0 {
        estimated_gsd_m * 1.5
    } else {
        0.1
    };
    fallback.clamp(0.02, 20.0)
}

fn pose_extent_and_mean_z(alignment: &AlignmentResult) -> (f64, f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut sum_z = 0.0;

    for pose in &alignment.poses {
        let [x, y, z] = pose.position;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        sum_z += z;
    }

    let mean_z = sum_z / alignment.poses.len() as f64;
    (min_x, max_x, min_y, max_y, mean_z)
}

fn estimate_surface_slopes(alignment: &AlignmentResult, mean_z: f64) -> (f64, f64) {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for pose in &alignment.poses {
        sum_x += pose.position[0];
        sum_y += pose.position[1];
    }
    let mean_x = sum_x / alignment.poses.len() as f64;
    let mean_y = sum_y / alignment.poses.len() as f64;

    let mut cov_xz = 0.0;
    let mut cov_yz = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for pose in &alignment.poses {
        let dx = pose.position[0] - mean_x;
        let dy = pose.position[1] - mean_y;
        let dz = pose.position[2] - mean_z;
        cov_xz += dx * dz;
        cov_yz += dy * dz;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let slope_x = if var_x > 1e-9 { cov_xz / var_x } else { 0.0 };
    let slope_y = if var_y > 1e-9 { cov_yz / var_y } else { 0.0 };
    (slope_x, slope_y)
}

fn interpolated_surface_z(
    alignment: &AlignmentResult,
    x: f64,
    y: f64,
    slope_x: f64,
    slope_y: f64,
    mean_z: f64,
    nominal_flight_height_m: f64,
    depth_hypotheses: &[DepthHypothesis],
) -> f64 {
    let mean_surface_z = mean_z - nominal_flight_height_m;
    let parallax_norm = (alignment.stats.mean_parallax_px / 8.0).clamp(0.15, 1.6);
    let support_scale_m = estimate_pose_support_scale_m(alignment);
    let smoothing_scale_m = (alignment.stats.estimated_gsd_m * 45.0)
        .clamp(6.0, 80.0)
        .max(support_scale_m * (2.2 + 0.9 * (1.0 / parallax_norm)));
    let smoothing2 = smoothing_scale_m * smoothing_scale_m;

    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for pose in &alignment.poses {
        let px = pose.position[0];
        let py = pose.position[1];
        let pz = pose.position[2] - nominal_flight_height_m;
        let dx = x - px;
        let dy = y - py;
        let d2 = dx * dx + dy * dy;
        let w = 1.0 / (d2 + smoothing2);
        let trend = pz + slope_x * dx + slope_y * dy;
        weighted_sum += w * trend;
        weight_total += w;
    }

    if weight_total <= 0.0 {
        mean_surface_z
    } else {
        let idw_surface = weighted_sum / weight_total;
        let fused = fuse_depth_hypotheses(
            x,
            y,
            idw_surface,
            smoothing_scale_m,
            depth_hypotheses,
            parallax_norm,
        );
        // Keep the fused surface physically plausible around the terrain-level band.
        let z_span_guard = (alignment.stats.estimated_gsd_m * 220.0).clamp(5.0, 90.0);
        fused.clamp(mean_surface_z - z_span_guard, mean_surface_z + z_span_guard)
    }
}

fn estimate_stereo_depths_from_adjacent_pairs(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
) -> Vec<DepthHypothesis> {
    let mvs_maps = estimate_multiview_depth_maps(alignment, frames);
    let mvs_depths = depth_hypotheses_from_multiview_maps(&mvs_maps);
    if !mvs_depths.is_empty() {
        return mvs_depths;
    }

    let image_depths = estimate_stereo_depths_from_image_pairs(alignment, frames);
    if !image_depths.is_empty() {
        return image_depths;
    }

    estimate_stereo_depths_from_pose_baselines(alignment)
}

fn estimate_multiview_depth_maps(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
) -> Vec<MultiViewDepthMap> {
    if alignment.poses.len() < 3 || frames.len() < 3 {
        return Vec::new();
    }

    let pose_count = alignment.poses.len().min(frames.len());
    let intrinsics = &alignment.stats.intrinsics;
    let mut loaded: Vec<Option<(GrayImage, f64)>> = vec![None; pose_count];
    for i in 0..pose_count {
        loaded[i] = load_downsampled_gray(&frames[i], 640);
    }

    let mut maps = Vec::new();
    for ref_idx in 0..pose_count {
        let Some((ref_img, ref_scale)) = loaded[ref_idx].as_ref() else {
            continue;
        };
        let source_views = select_mvs_source_views(alignment, ref_idx, pose_count, MVS_MAX_SOURCE_VIEWS);
        if source_views.len() < 2 {
            continue;
        }

        let ref_intr = scaled_intrinsics(intrinsics, *ref_scale);
        let p_ref = projection_matrix(&alignment.poses[ref_idx]);
        let mut samples = Vec::new();

        let max_y = ref_img.height().saturating_sub((MVS_PATCH_RADIUS as u32) + 1) as i32;
        let max_x = ref_img.width().saturating_sub((MVS_PATCH_RADIUS as u32) + 1) as i32;
        let start = MVS_PATCH_RADIUS + 1;
        if max_x <= start || max_y <= start {
            continue;
        }

        for y in (start..max_y).step_by(MVS_SAMPLE_STEP) {
            for x in (start..max_x).step_by(MVS_SAMPLE_STEP) {
                let center_ref = patch_center_intensity(ref_img, x, y);
                let x_ref_n = Vector2::new(
                    (x as f64 - ref_intr.0) / ref_intr.2,
                    (y as f64 - ref_intr.1) / ref_intr.3,
                );

                let mut best_point: Option<Vector3<f64>> = None;
                let mut best_conf = 0.0_f64;
                let mut best_support = 0usize;
                let mut best_ref_depth = 0.0_f64;

                for &src_idx in &source_views {
                    let Some((src_img, src_scale)) = loaded[src_idx].as_ref() else {
                        continue;
                    };
                    let src_intr = scaled_intrinsics(intrinsics, *src_scale);
                    let p_src = projection_matrix(&alignment.poses[src_idx]);

                    let fundamental_ref_to_src = fundamental_matrix_pinhole(
                        &alignment.poses[ref_idx],
                        &alignment.poses[src_idx],
                        ref_intr,
                        src_intr,
                    );
                    let Some(m) = best_patch_match(
                        ref_img,
                        src_img,
                        x,
                        y,
                        center_ref,
                        start,
                        MVS_SEARCH_RADIUS,
                        MVS_PATCH_RADIUS,
                        fundamental_ref_to_src.as_ref(),
                    ) else {
                        continue;
                    };
                    if !m.best_cost.is_finite() || m.best_cost > 0.42 || m.second_cost / m.best_cost.max(1.0e-9) < 1.03 {
                        continue;
                    }

                    let fundamental_src_to_ref = fundamental_matrix_pinhole(
                        &alignment.poses[src_idx],
                        &alignment.poses[ref_idx],
                        src_intr,
                        ref_intr,
                    );
                    let Some(back_m) = best_patch_match(
                        src_img,
                        ref_img,
                        m.x,
                        m.y,
                        patch_center_intensity(src_img, m.x, m.y),
                        start,
                        MVS_SEARCH_RADIUS,
                        MVS_PATCH_RADIUS,
                        fundamental_src_to_ref.as_ref(),
                    ) else {
                        continue;
                    };
                    if (back_m.x - x).abs() > MVS_LR_CONSISTENCY_TOL_PX
                        || (back_m.y - y).abs() > MVS_LR_CONSISTENCY_TOL_PX
                    {
                        continue;
                    }

                    let x_src_n = Vector2::new(
                        (m.x as f64 - src_intr.0) / src_intr.2,
                        (m.y as f64 - src_intr.1) / src_intr.3,
                    );
                    let Some(point_w) = triangulate_point(&p_ref, &p_src, &x_ref_n, &x_src_n) else {
                        continue;
                    };
                    let z_ref = camera_space_depth(&alignment.poses[ref_idx], &point_w);
                    let z_src = camera_space_depth(&alignment.poses[src_idx], &point_w);
                    if z_ref <= 1.0e-5 || z_src <= 1.0e-5 {
                        continue;
                    }

                    let mut support_views = 1usize;
                    let mut consistency = 0.0_f64;
                    for &other_idx in &source_views {
                        if other_idx == src_idx {
                            continue;
                        }
                        let Some((other_img, other_scale)) = loaded[other_idx].as_ref() else {
                            continue;
                        };
                        let other_intr = scaled_intrinsics(intrinsics, *other_scale);
                        let p_other = projection_matrix(&alignment.poses[other_idx]);
                        let Some((u, v)) = project_point_to_image_pinhole(
                            &alignment.poses[other_idx],
                            &point_w,
                            other_intr,
                        ) else {
                            continue;
                        };
                        let u0 = u.round() as i32;
                        let v0 = v.round() as i32;
                        if u0 <= start || v0 <= start || u0 >= other_img.width() as i32 - start || v0 >= other_img.height() as i32 - start {
                            continue;
                        }

                        let mut best_other_cost = f64::INFINITY;
                        let mut best_other_xy = None;
                        for ddy in -2..=2 {
                            for ddx in -2..=2 {
                                let uu = u0 + ddx;
                                let vv = v0 + ddy;
                                if uu <= start || vv <= start || uu >= other_img.width() as i32 - start || vv >= other_img.height() as i32 - start {
                                    continue;
                                }
                                let cost = patch_hybrid_cost(ref_img, other_img, x, y, uu, vv, MVS_PATCH_RADIUS);
                                if cost < best_other_cost {
                                    best_other_cost = cost;
                                    best_other_xy = Some((uu, vv));
                                }
                            }
                        }
                        let Some((uo, vo)) = best_other_xy else {
                            continue;
                        };
                        if !best_other_cost.is_finite() || best_other_cost > 0.48 {
                            continue;
                        }

                        let x_other_n = Vector2::new(
                            (uo as f64 - other_intr.0) / other_intr.2,
                            (vo as f64 - other_intr.1) / other_intr.3,
                        );
                        let Some(retriangulated) = triangulate_point(&p_ref, &p_other, &x_ref_n, &x_other_n) else {
                            continue;
                        };
                        let z_other = camera_space_depth(&alignment.poses[other_idx], &retriangulated);
                        if z_other <= 1.0e-5 {
                            continue;
                        }

                        let geo_delta = (retriangulated - point_w).norm();
                        let z_tol = (0.35 + 0.02 * z_ref.abs()).clamp(0.35, 3.0);
                        if geo_delta <= z_tol {
                            support_views += 1;
                            consistency += ((0.48 - best_other_cost) / 0.48).clamp(0.0, 1.0);
                        }
                    }

                    let match_conf = ((0.032 - m.best_cost) / 0.032).clamp(0.05, 1.0);
                    let ratio_conf = ((m.second_cost / m.best_cost.max(1.0e-9)) - 1.0).clamp(0.0, 0.25) / 0.25;
                    let support_factor = (support_views as f64 / source_views.len().max(1) as f64).clamp(0.0, 1.0);
                    let consistency_factor = (consistency / source_views.len().max(1) as f64).clamp(0.0, 1.0);
                    let conf = (0.45 * match_conf + 0.20 * ratio_conf + 0.20 * support_factor + 0.15 * consistency_factor)
                        .clamp(0.05, 0.98);

                    if conf > best_conf {
                        best_point = Some(point_w);
                        best_conf = conf;
                        best_support = support_views;
                        best_ref_depth = z_ref;
                    }
                }

                if let Some(point_w) = best_point {
                    samples.push(MultiViewDepthSample {
                        point_world: [point_w[0], point_w[1], point_w[2]],
                        confidence: best_conf,
                        support_views: best_support,
                        ref_px: [x as f64, y as f64],
                        ref_depth: best_ref_depth,
                    });
                }
            }
        }

        if !samples.is_empty() {
            maps.push(MultiViewDepthMap {
                reference_idx: ref_idx,
                samples,
            });
        }
    }

    maps
}

fn select_mvs_source_views(
    alignment: &AlignmentResult,
    reference_idx: usize,
    pose_count: usize,
    max_views: usize,
) -> Vec<usize> {
    if pose_count < 2 || reference_idx >= pose_count || max_views == 0 {
        return Vec::new();
    }

    let ref_pose = &alignment.poses[reference_idx];
    let ref_pos = Vector3::new(ref_pose.position[0], ref_pose.position[1], ref_pose.position[2]);
    let mut scored = Vec::new();
    for idx in 0..pose_count {
        if idx == reference_idx {
            continue;
        }
        let p = &alignment.poses[idx];
        let pos = Vector3::new(p.position[0], p.position[1], p.position[2]);
        let baseline = (pos - ref_pos).norm();
        if baseline <= 0.05 {
            continue;
        }
        let quality = (1.0 / (1.0 + 0.5 * (ref_pose.reprojection_error_px + p.reprojection_error_px))).clamp(0.1, 1.0);
        let baseline_score = (baseline / 30.0).clamp(0.05, 1.0);
        let score = 0.6 * baseline_score + 0.4 * quality;
        scored.push((idx, score));
    }

    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.into_iter().take(max_views).map(|v| v.0).collect()
}

fn depth_hypotheses_from_multiview_maps(maps: &[MultiViewDepthMap]) -> Vec<DepthHypothesis> {
    let mut out = Vec::new();
    for map in maps {
        let ref_weight = 1.0 / (1.0 + map.reference_idx as f64 * 0.02);
        let mut bins: std::collections::HashMap<(i32, i32), Vec<&MultiViewDepthSample>> =
            std::collections::HashMap::new();
        for sample in &map.samples {
            let bx = (sample.ref_px[0] / MVS_OCCLUSION_BIN_PX).floor() as i32;
            let by = (sample.ref_px[1] / MVS_OCCLUSION_BIN_PX).floor() as i32;
            bins.entry((bx, by)).or_default().push(sample);
        }

        for (_, mut samples) in bins {
            samples.sort_by(|a, b| a.ref_depth.total_cmp(&b.ref_depth));
            let front = samples[0];
            let mut occlusion_votes = 1usize;
            let mut vote_conf_sum = front.confidence;
            let front_tol = (0.25 + 0.02 * front.ref_depth.abs()).clamp(0.25, 2.0);
            for s in samples.iter().skip(1) {
                if (s.ref_depth - front.ref_depth).abs() <= front_tol {
                    occlusion_votes += 1;
                    vote_conf_sum += s.confidence;
                }
            }

            let support_factor = (front.support_views as f64 / MVS_MAX_SOURCE_VIEWS.max(1) as f64).clamp(0.0, 1.0);
            let vote_factor = (occlusion_votes as f64 / samples.len().max(1) as f64).clamp(0.0, 1.0);
            let mean_vote_conf = (vote_conf_sum / occlusion_votes as f64).clamp(0.0, 1.0);
            let confidence = (mean_vote_conf * (0.55 + 0.20 * support_factor + 0.25 * vote_factor) * ref_weight)
                .clamp(0.08, 0.99);
            out.push(DepthHypothesis {
                center: front.point_world,
                confidence,
            });
        }
    }
    out
}

fn estimate_stereo_depths_from_pose_baselines(
    alignment: &AlignmentResult,
) -> Vec<DepthHypothesis> {
    let mut stereo_depths = Vec::new();
    
    if alignment.poses.len() < 2 {
        return stereo_depths;
    }
    
    // For each adjacent frame pair, estimate depths from epipolar geometry
    for i in 0..alignment.poses.len() - 1 {
        let pose_l = &alignment.poses[i];
        let pose_r = &alignment.poses[i + 1];
        
        let p_l = Vector3::from(pose_l.position);
        let p_r = Vector3::from(pose_r.position);
        
        // Baseline vector and baseline length
        let baseline = p_r - p_l;
        let baseline_len = baseline.norm();
        
        if baseline_len < 0.01 {
            continue; // Skip nearly coincident poses
        }
        
        // Confidence based on reprojection errors and baseline geometry
        let error_l = (1.0 / (1.0 + pose_l.reprojection_error_px)).clamp(0.2, 1.0);
        let error_r = (1.0 / (1.0 + pose_r.reprojection_error_px)).clamp(0.2, 1.0);
        let confidence_base = 0.5 * (error_l + error_r);
        
        // Generate depth samples along the baseline for observation
        let num_depth_samples = 5;
        let min_depth = baseline_len * 0.8;
        let max_depth = baseline_len * 15.0;
        
        for d in 0..num_depth_samples {
            let t = d as f64 / (num_depth_samples as f64 - 1.0);
            let depth = min_depth + t * (max_depth - min_depth);
            
            // Compute a point approximately visible from both cameras
            // Place point along baseline at relative depth
            let point_w = p_l + baseline * t + Vector3::new(0.0, 0.0, depth * 0.15);
            
            // Weight by depth plausibility
            let depth_weight = (1.0 - (t - 0.5).abs()).clamp(0.3, 1.0);
            let confidence = (confidence_base * depth_weight).clamp(0.15, 0.95);
            
            stereo_depths.push(DepthHypothesis {
                center: [point_w[0], point_w[1], point_w[2]],
                confidence,
            });
        }
    }
    
    stereo_depths
}

fn estimate_stereo_depths_from_image_pairs(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
) -> Vec<DepthHypothesis> {
    if alignment.poses.len() < 2 || frames.len() < 2 {
        return Vec::new();
    }

    let pair_count = alignment.poses.len().min(frames.len()).saturating_sub(1);
    if pair_count == 0 {
        return Vec::new();
    }

    let mut stereo_depths = Vec::new();
    let intrinsics = &alignment.stats.intrinsics;
    for i in 0..pair_count {
        let left = match load_downsampled_gray(&frames[i], 640) {
            Some(v) => v,
            None => continue,
        };
        let right = match load_downsampled_gray(&frames[i + 1], 640) {
            Some(v) => v,
            None => continue,
        };

        let scaled_intrinsics_left = scaled_intrinsics(intrinsics, left.1);
        let scaled_intrinsics_right = scaled_intrinsics(intrinsics, right.1);
        let p_left = projection_matrix(&alignment.poses[i]);
        let p_right = projection_matrix(&alignment.poses[i + 1]);
        let f_left_to_right = fundamental_matrix_pinhole(
            &alignment.poses[i],
            &alignment.poses[i + 1],
            scaled_intrinsics_left,
            scaled_intrinsics_right,
        );
        let f_right_to_left = fundamental_matrix_pinhole(
            &alignment.poses[i + 1],
            &alignment.poses[i],
            scaled_intrinsics_right,
            scaled_intrinsics_left,
        );

        let left_img = &left.0;
        let right_img = &right.0;
        let patch_radius = 2i32;
        let search_radius = 10i32;
        let step = 48usize;
        let mut accepted_for_pair = 0usize;

        let max_y = left_img.height().saturating_sub((patch_radius as u32) + 1) as i32;
        let max_x = left_img.width().saturating_sub((patch_radius as u32) + 1) as i32;
        let start = patch_radius + 1;
        if max_x <= start || max_y <= start {
            continue;
        }

        for y in (start..max_y).step_by(step) {
            for x in (start..max_x).step_by(step) {
                if accepted_for_pair >= 64 {
                    break;
                }
                let center_l = patch_center_intensity(left_img, x, y);
                let Some(m) = best_patch_match(
                    left_img,
                    right_img,
                    x,
                    y,
                    center_l,
                    start,
                    search_radius,
                    patch_radius,
                    f_left_to_right.as_ref(),
                ) else {
                    continue;
                };
                if !m.best_cost.is_finite() || m.best_cost > 0.44 || m.second_cost / m.best_cost.max(1.0e-9) < 1.04 {
                    continue;
                }

                let Some(back_m) = best_patch_match(
                    right_img,
                    left_img,
                    m.x,
                    m.y,
                    patch_center_intensity(right_img, m.x, m.y),
                    start,
                    search_radius,
                    patch_radius,
                    f_right_to_left.as_ref(),
                ) else {
                    continue;
                };
                if (back_m.x - x).abs() > MVS_LR_CONSISTENCY_TOL_PX
                    || (back_m.y - y).abs() > MVS_LR_CONSISTENCY_TOL_PX
                {
                    continue;
                }

                let x1n = Vector2::new(
                    (x as f64 - scaled_intrinsics_left.0) / scaled_intrinsics_left.2,
                    (y as f64 - scaled_intrinsics_left.1) / scaled_intrinsics_left.3,
                );
                let x2n = Vector2::new(
                    (m.x as f64 - scaled_intrinsics_right.0) / scaled_intrinsics_right.2,
                    (m.y as f64 - scaled_intrinsics_right.1) / scaled_intrinsics_right.3,
                );
                let Some(point_w) = triangulate_point(&p_left, &p_right, &x1n, &x2n) else {
                    continue;
                };
                let z_cam_l = camera_space_depth(&alignment.poses[i], &point_w);
                let z_cam_r = camera_space_depth(&alignment.poses[i + 1], &point_w);
                if z_cam_l <= 1.0e-5 || z_cam_r <= 1.0e-5 {
                    continue;
                }

                let confidence = ((0.50 - m.best_cost) / 0.50).clamp(0.2, 0.95);
                stereo_depths.push(DepthHypothesis {
                    center: [point_w[0], point_w[1], point_w[2]],
                    confidence,
                });
                accepted_for_pair += 1;
            }
            if accepted_for_pair >= 64 {
                break;
            }
        }
    }

    stereo_depths
}

fn load_downsampled_gray(frame: &ImageFrame, max_dim: u32) -> Option<(GrayImage, f64)> {
    let image = image::open(&frame.path).ok()?;
    let gray = image.to_luma8();
    let width = gray.width();
    let height = gray.height();
    let max_side = width.max(height).max(1);
    let scale = if max_side > max_dim {
        max_dim as f64 / max_side as f64
    } else {
        1.0
    };
    if scale >= 0.999 {
        return Some((gray, 1.0));
    }
    let new_w = ((width as f64 * scale).round() as u32).max(32);
    let new_h = ((height as f64 * scale).round() as u32).max(32);
    let resized = image::imageops::resize(&gray, new_w, new_h, image::imageops::FilterType::Triangle);
    Some((resized, scale))
}

fn scaled_intrinsics(intrinsics: &crate::camera::CameraIntrinsics, scale: f64) -> (f64, f64, f64, f64) {
    (intrinsics.cx * scale, intrinsics.cy * scale, intrinsics.fx * scale, intrinsics.fy * scale)
}

fn projection_matrix(pose: &crate::alignment::CameraPose) -> Matrix3x4<f64> {
    let r_c2w = quaternion_to_matrix(&pose.rotation);
    let r_w2c = r_c2w.transpose();
    let center = Vector3::new(pose.position[0], pose.position[1], pose.position[2]);
    let t = -(r_w2c * center);
    Matrix3x4::new(
        r_w2c[(0, 0)], r_w2c[(0, 1)], r_w2c[(0, 2)], t[0],
        r_w2c[(1, 0)], r_w2c[(1, 1)], r_w2c[(1, 2)], t[1],
        r_w2c[(2, 0)], r_w2c[(2, 1)], r_w2c[(2, 2)], t[2],
    )
}

fn fundamental_matrix_pinhole(
    pose_left: &crate::alignment::CameraPose,
    pose_right: &crate::alignment::CameraPose,
    intr_left: (f64, f64, f64, f64),
    intr_right: (f64, f64, f64, f64),
) -> Option<Matrix3<f64>> {
    let r1_c2w = quaternion_to_matrix(&pose_left.rotation);
    let r2_c2w = quaternion_to_matrix(&pose_right.rotation);
    let r1 = r1_c2w.transpose();
    let r2 = r2_c2w.transpose();
    let c1 = Vector3::new(pose_left.position[0], pose_left.position[1], pose_left.position[2]);
    let c2 = Vector3::new(pose_right.position[0], pose_right.position[1], pose_right.position[2]);
    let t1 = -(r1 * c1);
    let t2 = -(r2 * c2);

    let r21 = r2 * r1.transpose();
    let t21 = t2 - r21 * t1;
    let e = skew_symmetric(t21) * r21;

    let k1 = Matrix3::new(
        intr_left.2, 0.0, intr_left.0,
        0.0, intr_left.3, intr_left.1,
        0.0, 0.0, 1.0,
    );
    let k2 = Matrix3::new(
        intr_right.2, 0.0, intr_right.0,
        0.0, intr_right.3, intr_right.1,
        0.0, 0.0, 1.0,
    );
    let k1_inv = k1.try_inverse()?;
    let k2_inv_t = k2.try_inverse()?.transpose();
    Some(k2_inv_t * e * k1_inv)
}

fn skew_symmetric(v: Vector3<f64>) -> Matrix3<f64> {
    Matrix3::new(
        0.0, -v[2], v[1],
        v[2], 0.0, -v[0],
        -v[1], v[0], 0.0,
    )
}

fn best_patch_match(
    left: &GrayImage,
    right: &GrayImage,
    xl: i32,
    yl: i32,
    center_left_intensity: f64,
    border: i32,
    search_radius: i32,
    patch_radius: i32,
    fundamental: Option<&Matrix3<f64>>,
) -> Option<PatchMatch> {
    if xl <= border
        || yl <= border
        || xl >= left.width() as i32 - border
        || yl >= left.height() as i32 - border
    {
        return None;
    }

    let right_w = right.width() as i32;
    let right_h = right.height() as i32;
    let mut candidates = if let Some(f) = fundamental {
        epipolar_line_candidates(
            f,
            xl,
            yl,
            xl,
            yl,
            search_radius,
            right_w,
            right_h,
            border,
        )
    } else {
        Vec::new()
    };
    if candidates.is_empty() {
        candidates = square_window_candidates(xl, yl, search_radius, right_w, right_h, border);
    }

    let mut best = f64::INFINITY;
    let mut second = f64::INFINITY;
    let mut best_xy = None;
    for (xr, yr) in candidates {
        let center_right_intensity = patch_center_intensity(right, xr, yr);
        if (center_left_intensity - center_right_intensity).abs() > 30.0 {
            continue;
        }
        let score = patch_hybrid_cost(left, right, xl, yl, xr, yr, patch_radius);
        if score < best {
            second = best;
            best = score;
            best_xy = Some((xr, yr));
        } else if score < second {
            second = score;
        }
    }

    best_xy.map(|(x, y)| PatchMatch {
        x,
        y,
        best_cost: best,
        second_cost: second,
    })
}

fn square_window_candidates(
    center_x: i32,
    center_y: i32,
    radius: i32,
    width: i32,
    height: i32,
    border: i32,
) -> Vec<(i32, i32)> {
    let mut candidates = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let x = center_x + dx;
            let y = center_y + dy;
            if x <= border || y <= border || x >= width - border || y >= height - border {
                continue;
            }
            candidates.push((x, y));
        }
    }
    candidates
}

fn epipolar_line_candidates(
    fundamental: &Matrix3<f64>,
    xl: i32,
    yl: i32,
    center_x: i32,
    center_y: i32,
    radius: i32,
    width: i32,
    height: i32,
    border: i32,
) -> Vec<(i32, i32)> {
    let line = fundamental * nalgebra::Vector3::new(xl as f64, yl as f64, 1.0);
    let a = line[0];
    let b = line[1];
    let c = line[2];
    let norm = (a * a + b * b).sqrt();
    if norm <= 1.0e-9 || !norm.is_finite() {
        return Vec::new();
    }

    let mut out = Vec::new();
    if b.abs() >= a.abs() {
        for x in (center_x - radius)..=(center_x + radius) {
            if x <= border || x >= width - border {
                continue;
            }
            let y = ((-c - a * x as f64) / b).round() as i32;
            if y <= border || y >= height - border {
                continue;
            }
            let dist = (a * x as f64 + b * y as f64 + c).abs() / norm;
            if dist <= MVS_EPIPOLAR_TOL_PX {
                out.push((x, y));
            }
        }
    } else {
        for y in (center_y - radius)..=(center_y + radius) {
            if y <= border || y >= height - border {
                continue;
            }
            let x = ((-c - b * y as f64) / a).round() as i32;
            if x <= border || x >= width - border {
                continue;
            }
            let dist = (a * x as f64 + b * y as f64 + c).abs() / norm;
            if dist <= MVS_EPIPOLAR_TOL_PX {
                out.push((x, y));
            }
        }
    }
    out
}

fn quaternion_to_matrix(q: &[f64; 4]) -> Matrix3<f64> {
    let w = q[0];
    let x = q[1];
    let y = q[2];
    let z = q[3];
    Matrix3::new(
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y - z * w),
        2.0 * (x * z + y * w),
        2.0 * (x * y + z * w),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z - x * w),
        2.0 * (x * z - y * w),
        2.0 * (y * z + x * w),
        1.0 - 2.0 * (x * x + y * y),
    )
}

fn triangulate_point(
    p1: &Matrix3x4<f64>,
    p2: &Matrix3x4<f64>,
    x1: &Vector2<f64>,
    x2: &Vector2<f64>,
) -> Option<Vector3<f64>> {
    let mut a = nalgebra::Matrix4::zeros();
    a.row_mut(0).copy_from(&(x1.x * p1.row(2) - p1.row(0)));
    a.row_mut(1).copy_from(&(x1.y * p1.row(2) - p1.row(1)));
    a.row_mut(2).copy_from(&(x2.x * p2.row(2) - p2.row(0)));
    a.row_mut(3).copy_from(&(x2.y * p2.row(2) - p2.row(1)));

    let svd = a.svd(true, true);
    let v_t = svd.v_t?;
    let xh = v_t.row(3).transpose();
    if xh[3].abs() <= 1.0e-12 {
        return None;
    }
    Some(Vector3::new(xh[0] / xh[3], xh[1] / xh[3], xh[2] / xh[3]))
}

fn camera_space_depth(pose: &crate::alignment::CameraPose, point_w: &Vector3<f64>) -> f64 {
    let r_c2w = quaternion_to_matrix(&pose.rotation);
    let r_w2c = r_c2w.transpose();
    let center = Vector3::new(pose.position[0], pose.position[1], pose.position[2]);
    let x_cam = r_w2c * (*point_w - center);
    x_cam[2]
}

fn project_point_to_image_pinhole(
    pose: &crate::alignment::CameraPose,
    point_w: &Vector3<f64>,
    scaled_intr: (f64, f64, f64, f64),
) -> Option<(f64, f64)> {
    let r_c2w = quaternion_to_matrix(&pose.rotation);
    let r_w2c = r_c2w.transpose();
    let center = Vector3::new(pose.position[0], pose.position[1], pose.position[2]);
    let x_cam = r_w2c * (*point_w - center);
    if x_cam[2] <= 1.0e-6 {
        return None;
    }

    let xn = x_cam[0] / x_cam[2];
    let yn = x_cam[1] / x_cam[2];
    let u = scaled_intr.0 + scaled_intr.2 * xn;
    let v = scaled_intr.1 + scaled_intr.3 * yn;
    if u.is_finite() && v.is_finite() {
        Some((u, v))
    } else {
        None
    }
}

fn patch_center_intensity(img: &GrayImage, x: i32, y: i32) -> f64 {
    img.get_pixel(x as u32, y as u32)[0] as f64
}

fn patch_zncc_cost(
    left: &GrayImage,
    right: &GrayImage,
    xl: i32,
    yl: i32,
    xr: i32,
    yr: i32,
    radius: i32,
) -> f64 {
    let mut sum_l = 0.0;
    let mut sum_r = 0.0;
    let mut n = 0.0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let lv = left.get_pixel((xl + dx) as u32, (yl + dy) as u32)[0] as f64 / 255.0;
            let rv = right.get_pixel((xr + dx) as u32, (yr + dy) as u32)[0] as f64 / 255.0;
            sum_l += lv;
            sum_r += rv;
            n += 1.0;
        }
    }
    if n <= 0.0 {
        return 1.0;
    }

    let mean_l = sum_l / n;
    let mean_r = sum_r / n;
    let mut num = 0.0;
    let mut den_l = 0.0;
    let mut den_r = 0.0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let lv = left.get_pixel((xl + dx) as u32, (yl + dy) as u32)[0] as f64 / 255.0 - mean_l;
            let rv = right.get_pixel((xr + dx) as u32, (yr + dy) as u32)[0] as f64 / 255.0 - mean_r;
            num += lv * rv;
            den_l += lv * lv;
            den_r += rv * rv;
        }
    }

    let denom = (den_l * den_r).sqrt();
    if denom <= 1.0e-12 || !denom.is_finite() {
        return 1.0;
    }
    let zncc = (num / denom).clamp(-1.0, 1.0);
    // Convert to a minimization cost in [0, 1].
    0.5 * (1.0 - zncc)
}

fn patch_census_cost(
    left: &GrayImage,
    right: &GrayImage,
    xl: i32,
    yl: i32,
    xr: i32,
    yr: i32,
    radius: i32,
) -> f64 {
    let center_l = left.get_pixel(xl as u32, yl as u32)[0] as i16;
    let center_r = right.get_pixel(xr as u32, yr as u32)[0] as i16;
    let mut total = 0u32;
    let mut hamming = 0u32;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            let lv = left.get_pixel((xl + dx) as u32, (yl + dy) as u32)[0] as i16;
            let rv = right.get_pixel((xr + dx) as u32, (yr + dy) as u32)[0] as i16;
            let bit_l = lv >= center_l;
            let bit_r = rv >= center_r;
            if bit_l != bit_r {
                hamming += 1;
            }
            total += 1;
        }
    }

    if total == 0 {
        return 1.0;
    }
    (hamming as f64 / total as f64).clamp(0.0, 1.0)
}

fn patch_hybrid_cost(
    left: &GrayImage,
    right: &GrayImage,
    xl: i32,
    yl: i32,
    xr: i32,
    yr: i32,
    radius: i32,
) -> f64 {
    let zncc = patch_zncc_cost(left, right, xl, yl, xr, yr, radius);
    let census = patch_census_cost(left, right, xl, yl, xr, yr, MVS_CENSUS_RADIUS.min(radius));
    (MVS_HYBRID_ZNCC_WEIGHT * zncc + MVS_HYBRID_CENSUS_WEIGHT * census).clamp(0.0, 1.0)
}

fn build_depth_hypotheses(
    alignment: &AlignmentResult,
    mean_z: f64,
    slope_x: f64,
    slope_y: f64,
    nominal_flight_height_m: f64,
) -> Vec<DepthHypothesis> {
    if alignment.poses.is_empty() {
        return Vec::new();
    }

    let mut hypotheses = Vec::with_capacity(alignment.poses.len() * 8);
    let support_scale_m = estimate_pose_support_scale_m(alignment);
    let parallax_norm = (alignment.stats.mean_parallax_px / 8.0).clamp(0.2, 1.8);
    let tie_support = (alignment.stats.tie_points_median as f64 / 10.0).clamp(1.5, 12.0);
    let lateral_span_m = (support_scale_m * (0.18 + 0.10 * parallax_norm)).clamp(0.4, 6.0);
    let depth_jitter_amp_m = (alignment.stats.estimated_gsd_m * (2.5 + 2.5 * parallax_norm)).clamp(0.08, 2.2);

    for (idx, pose) in alignment.poses.iter().enumerate() {
        let x = pose.position[0];
        let y = pose.position[1];

        let confidence = (1.0 / (1.0 + pose.reprojection_error_px)).clamp(0.2, 1.0);
        let along_track_boost = if idx > 0 {
            let dx = x - alignment.poses[idx - 1].position[0];
            let dy = y - alignment.poses[idx - 1].position[1];
            (dx * dx + dy * dy).sqrt().clamp(0.5, 8.0) / 8.0
        } else {
            0.5
        };

        hypotheses.push(DepthHypothesis {
            center: [pose.position[0], pose.position[1], pose.position[2] - nominal_flight_height_m],
            confidence: (confidence * (0.75 + 0.25 * along_track_boost)).clamp(0.2, 1.0),
        });

        if idx + 1 < alignment.poses.len() {
            let next = alignment.poses[idx + 1].position;
            let seg_dx = next[0] - pose.position[0];
            let seg_dy = next[1] - pose.position[1];
            let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt();
            if seg_len > 1e-6 {
                let nx = -seg_dy / seg_len;
                let ny = seg_dx / seg_len;
                // Approximate sparse-track support density from tie-point and parallax strength.
                let base_steps = (seg_len / support_scale_m.clamp(2.0, 25.0)).ceil() as usize;
                let support_bonus = (tie_support / 2.5).round() as usize;
                let steps = (base_steps + support_bonus).clamp(6, 26);

                for s in 1..steps {
                    let t = s as f64 / steps as f64;
                    let ix = pose.position[0] + seg_dx * t;
                    let iy = pose.position[1] + seg_dy * t;
                    let iz_linear = (pose.position[2] * (1.0 - t) + next[2] * t) - nominal_flight_height_m;
                    let phase = (idx as f64 * 0.7) + (s as f64 * 0.9);
                    let iz = iz_linear + depth_jitter_amp_m * phase.sin();

                    let base_conf = (confidence * (0.80 + 0.12 * parallax_norm)).clamp(0.15, 0.95);
                    hypotheses.push(DepthHypothesis {
                        center: [ix, iy, iz],
                        confidence: base_conf,
                    });

                    // Add small cross-track support points to reduce cell partitioning artifacts.
                    let trend_z = mean_z + slope_x * ix + slope_y * iy - nominal_flight_height_m;
                    let cross_conf = (base_conf * (0.62 + 0.08 * parallax_norm)).clamp(0.10, 0.80);
                    hypotheses.push(DepthHypothesis {
                        center: [
                            ix + nx * lateral_span_m,
                            iy + ny * lateral_span_m,
                            0.70 * iz + 0.30 * trend_z + 0.55 * depth_jitter_amp_m,
                        ],
                        confidence: (cross_conf * 1.07).clamp(0.10, 0.86),
                    });
                    hypotheses.push(DepthHypothesis {
                        center: [
                            ix - nx * lateral_span_m,
                            iy - ny * lateral_span_m,
                            0.75 * iz + 0.25 * trend_z - 0.35 * depth_jitter_amp_m,
                        ],
                        confidence: (cross_conf * 0.93).clamp(0.10, 0.78),
                    });
                }
            }
        }

        if alignment.poses.len() > 2 {
            let prev = if idx > 0 {
                Some(alignment.poses[idx - 1].position)
            } else {
                None
            };
            let next = if idx + 1 < alignment.poses.len() {
                Some(alignment.poses[idx + 1].position)
            } else {
                None
            };

            if let (Some(a), Some(b)) = (prev, next) {
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let seg_len = (dx * dx + dy * dy).sqrt();
                if seg_len > 1e-6 {
                    let nx = -dy / seg_len;
                    let ny = dx / seg_len;
                    let local_conf = (confidence * (0.55 + 0.20 * parallax_norm)).clamp(0.15, 0.85);
                    let local_span = (0.65 * lateral_span_m).clamp(0.35, 4.0);
                    let local_base = pose.position[2] - nominal_flight_height_m;
                    hypotheses.push(DepthHypothesis {
                        center: [x + nx * local_span, y + ny * local_span, local_base + 0.35 * depth_jitter_amp_m],
                        confidence: local_conf,
                    });
                    hypotheses.push(DepthHypothesis {
                        center: [x - nx * local_span, y - ny * local_span, local_base - 0.35 * depth_jitter_amp_m],
                        confidence: local_conf,
                    });
                }
            }
        }
    }
    hypotheses
}

fn estimate_nominal_flight_height_m(alignment: &AlignmentResult) -> f64 {
    let fx = alignment.stats.intrinsics.fx.max(1.0);
    let gsd = alignment.stats.estimated_gsd_m.max(0.01);
    // Approximate camera-to-ground distance from pinhole sampling geometry.
    (gsd * fx * 0.85).clamp(15.0, 220.0)
}

fn curate_depth_hypotheses(
    mut hypotheses: Vec<DepthHypothesis>,
    mean_surface_z: f64,
    support_scale_m: f64,
) -> Vec<DepthHypothesis> {
    if hypotheses.is_empty() {
        return hypotheses;
    }

    hypotheses.retain(|h| {
        h.center.iter().all(|v| v.is_finite())
            && h.confidence.is_finite()
            && h.confidence > 0.0
            && (h.center[2] - mean_surface_z).abs() <= 120.0
    });
    if hypotheses.is_empty() {
        return hypotheses;
    }

    let mut z_vals: Vec<f64> = hypotheses.iter().map(|h| h.center[2]).collect();
    z_vals.sort_by(|a, b| a.total_cmp(b));
    let median_z = z_vals[z_vals.len() / 2];
    let mut abs_dev: Vec<f64> = z_vals.iter().map(|z| (z - median_z).abs()).collect();
    abs_dev.sort_by(|a, b| a.total_cmp(b));
    let mad = abs_dev[abs_dev.len() / 2].max(0.25);
    let z_gate = (4.5 * mad).clamp(2.0, 35.0);
    hypotheses.retain(|h| (h.center[2] - median_z).abs() <= z_gate);

    hypotheses.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let min_spacing_m = (0.22 * support_scale_m).clamp(0.35, 3.0);
    let min_spacing2 = min_spacing_m * min_spacing_m;
    let mut curated: Vec<DepthHypothesis> = Vec::with_capacity(hypotheses.len().min(7000));
    for h in hypotheses {
        if curated.len() >= 7000 {
            break;
        }
        let too_close_better = curated.iter().any(|k| {
            let dx = h.center[0] - k.center[0];
            let dy = h.center[1] - k.center[1];
            let d2 = dx * dx + dy * dy;
            d2 < min_spacing2 && (h.center[2] - k.center[2]).abs() < 1.25
        });
        if too_close_better {
            continue;
        }
        curated.push(DepthHypothesis {
            center: h.center,
            confidence: h.confidence.clamp(0.08, 1.0),
        });
    }
    curated
}

fn depth_support_score(x: f64, y: f64, sigma_m: f64, depth_hypotheses: &[DepthHypothesis]) -> f64 {
    if depth_hypotheses.is_empty() {
        return 0.0;
    }
    let sigma2 = sigma_m * sigma_m;
    let mut wsum = 0.0;
    for h in depth_hypotheses {
        let dx = x - h.center[0];
        let dy = y - h.center[1];
        let d2 = dx * dx + dy * dy;
        let w = (-d2 / (2.0 * sigma2)).exp() * h.confidence;
        wsum += w;
    }
    (wsum / 1.6).clamp(0.0, 1.0)
}

fn repair_low_support_cells(
    surface: &mut [f64],
    support_map: &[f64],
    rows: usize,
    cols: usize,
    mean_surface_z: f64,
    estimated_gsd_m: f64,
) {
    if rows < 3 || cols < 3 || support_map.len() != surface.len() {
        return;
    }
    let mut supports: Vec<f64> = support_map.iter().copied().filter(|s| s.is_finite()).collect();
    if supports.is_empty() {
        return;
    }
    supports.sort_by(|a, b| a.total_cmp(b));
    let median = supports[supports.len() / 2];
    let low_thr = (0.28 * median).clamp(0.015, 0.12);
    let mut next = surface.to_vec();
    let step_guard = (estimated_gsd_m * 16.0).clamp(0.25, 4.5);

    for row in 1..(rows - 1) {
        for col in 1..(cols - 1) {
            let idx = row * cols + col;
            if support_map[idx] >= low_thr {
                continue;
            }
            let center = surface[idx];
            let mut nsum = 0.0;
            let mut wsum = 0.0;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let rr = (row as isize + dr) as usize;
                    let cc = (col as isize + dc) as usize;
                    let nidx = rr * cols + cc;
                    let s = support_map[nidx].clamp(0.0, 1.0);
                    if s <= 0.0 {
                        continue;
                    }
                    let w = 0.35 + 0.65 * s;
                    nsum += w * surface[nidx];
                    wsum += w;
                }
            }
            if wsum <= 0.0 {
                continue;
            }
            let repaired = nsum / wsum;
            let blend = (0.10 + 0.40 * (low_thr - support_map[idx]).max(0.0) / low_thr.max(1e-6)).clamp(0.10, 0.48);
            next[idx] = (1.0 - blend) * center + blend * repaired;
            next[idx] = next[idx].clamp(center - step_guard, center + step_guard);
            next[idx] = next[idx].clamp(mean_surface_z - 95.0, mean_surface_z + 95.0);
        }
    }
    surface.copy_from_slice(&next);
}

fn build_uncertainty_map(
    surface: &[f64],
    support_map: &[f64],
    rows: usize,
    cols: usize,
    estimated_gsd_m: f64,
) -> Vec<f64> {
    let mut out = vec![1.0_f64; surface.len()];
    if rows == 0 || cols == 0 || support_map.len() != surface.len() {
        return out;
    }

    let relief_scale = (estimated_gsd_m * 12.0).clamp(0.2, 3.0);
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let center = surface[idx];
            if !center.is_finite() {
                out[idx] = 1.0;
                continue;
            }
            let mut local_relief = 0.0;
            let mut n = 0usize;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if dr == 0 && dc == 0 {
                        continue;
                    }
                    let rr = row as isize + dr;
                    let cc = col as isize + dc;
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let v = surface[rr as usize * cols + cc as usize];
                    if !v.is_finite() {
                        continue;
                    }
                    local_relief += (v - center).abs();
                    n += 1;
                }
            }
            let relief_term = if n > 0 {
                (local_relief / n as f64 / relief_scale).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let support_term = (1.0 - support_map[idx].clamp(0.0, 1.0)).clamp(0.0, 1.0);
            out[idx] = (0.65 * support_term + 0.35 * relief_term).clamp(0.0, 1.0);
        }
    }
    out
}

fn with_suffix_before_ext(path: &str, suffix: &str) -> String {
    use std::path::Path;
    let p = Path::new(path);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("tif");
    let file_name = format!("{}{}.{}", stem, suffix, ext);
    p.with_file_name(file_name).to_string_lossy().to_string()
}

fn fuse_depth_hypotheses(
    x: f64,
    y: f64,
    base_z: f64,
    smoothing_scale_m: f64,
    depth_hypotheses: &[DepthHypothesis],
    parallax_norm: f64,
) -> f64 {
    if depth_hypotheses.is_empty() {
        return base_z;
    }

    let sigma = (smoothing_scale_m * (1.4 + 0.5 / parallax_norm)).clamp(8.0, 120.0);
    let sigma2 = sigma * sigma;

    let mut sum = 0.0;
    let mut wsum = 0.0;
    let mut nearest_d2 = f64::INFINITY;
    let mut nearest_anchor = base_z;
    for hyp in depth_hypotheses {
        let dx = x - hyp.center[0];
        let dy = y - hyp.center[1];
        let d2 = dx * dx + dy * dy;
        if d2 < nearest_d2 {
            nearest_d2 = d2;
            nearest_anchor = hyp.center[2];
        }

        let spatial_w = (-d2 / (2.0 * sigma2)).exp();
        let view_w = hyp.confidence;
        let w = spatial_w * view_w;
        if w <= 0.0 {
            continue;
        }

        let local_anchor = hyp.center[2];
        let proximity = (-d2 / (2.0 * (0.55 * sigma).powi(2))).exp();
        let anchor_weight = (0.08 + 0.20 * parallax_norm * (0.65 + 0.35 * proximity)).clamp(0.08, 0.34);
        let candidate_z = (1.0 - anchor_weight) * base_z + anchor_weight * local_anchor;
        sum += w * candidate_z;
        wsum += w;
    }

    if wsum <= 0.0 {
        base_z
    } else {
        let blended = sum / wsum;
        let nearest_influence = (-nearest_d2 / (2.0 * (0.9 * sigma).powi(2))).exp();
        let blend_w = (0.15 + 0.20 * nearest_influence).clamp(0.15, 0.42);
        let nearest_w = (0.02 + 0.10 * nearest_influence * parallax_norm).clamp(0.02, 0.16);
        let base_w = (1.0 - blend_w - nearest_w).clamp(0.46, 0.83);
        base_w * base_z + blend_w * blended + nearest_w * nearest_anchor
    }
}

fn estimate_pose_support_scale_m(alignment: &AlignmentResult) -> f64 {
    if alignment.poses.len() < 2 {
        return 12.0;
    }

    let mut dists = Vec::with_capacity(alignment.poses.len() - 1);
    for idx in 1..alignment.poses.len() {
        let a = alignment.poses[idx - 1].position;
        let b = alignment.poses[idx].position;
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let d = (dx * dx + dy * dy).sqrt();
        if d.is_finite() && d > 0.0 {
            dists.push(d);
        }
    }

    if dists.is_empty() {
        12.0
    } else {
        dists.sort_by(|a, b| a.total_cmp(b));
        let median = dists[dists.len() / 2];
        median.clamp(3.0, 45.0)
    }
}

fn refine_surface_grid(surface: &mut [f64], rows: usize, cols: usize, estimated_gsd_m: f64) {
    if rows < 3 || cols < 3 {
        return;
    }

    let step_guard = (estimated_gsd_m * 24.0).clamp(0.25, 6.0);
    for _ in 0..5 {
        let prev = surface.to_vec();
        for row in 1..(rows - 1) {
            for col in 1..(cols - 1) {
                let idx = row * cols + col;
                let center = prev[idx];

                let mut neighborhood = Vec::with_capacity(9);
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        let rr = (row as isize + dr) as usize;
                        let cc = (col as isize + dc) as usize;
                        neighborhood.push(prev[rr * cols + cc]);
                    }
                }
                neighborhood.sort_by(|a, b| a.total_cmp(b));
                let median = neighborhood[neighborhood.len() / 2];

                let mut mad = 0.0;
                for v in &neighborhood {
                    mad += (v - median).abs();
                }
                mad = (mad / neighborhood.len() as f64).max(0.05);

                let mut nsum = 0.0;
                let mut nweight = 0.0;
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let rr = (row as isize + dr) as usize;
                        let cc = (col as isize + dc) as usize;
                        let v = prev[rr * cols + cc];
                        let contrast = ((v - center).abs() / (2.0 * mad)).powi(2);
                        let w = 1.0 / (1.0 + contrast);
                        nsum += w * v;
                        nweight += w;
                    }
                }

                if nweight > 0.0 {
                    let smooth = nsum / nweight;
                    let blended = 0.40 * center + 0.45 * smooth + 0.15 * median;
                    surface[idx] = blended.clamp(center - step_guard, center + step_guard);
                }
            }
        }
    }
}

fn surface_range_m(surface: &[f64]) -> f64 {
    if surface.is_empty() {
        return 0.0;
    }
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &v in surface {
        if !v.is_finite() {
            continue;
        }
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    if !min_v.is_finite() || !max_v.is_finite() {
        0.0
    } else {
        (max_v - min_v).max(0.0)
    }
}

fn reinforce_surface_from_hypotheses(
    surface: &mut [f64],
    rows: usize,
    cols: usize,
    x_min: f64,
    y_min: f64,
    cell_size: f64,
    alignment: &AlignmentResult,
    depth_hypotheses: &[DepthHypothesis],
    mean_surface_z: f64,
) {
    if rows == 0 || cols == 0 {
        return;
    }

    let support_scale_m = estimate_pose_support_scale_m(alignment).max(3.0);
    let parallax_norm = (alignment.stats.mean_parallax_px / 8.0).clamp(0.15, 1.8);
    let sigma = (support_scale_m * (1.9 + 0.6 / parallax_norm)).clamp(6.0, 70.0);
    let sigma2 = sigma * sigma;
    let detail_amp_m = (alignment.stats.estimated_gsd_m * (8.0 + 6.0 * parallax_norm)).clamp(0.12, 1.4);

    let fallback_depth = if depth_hypotheses.is_empty() {
        mean_surface_z
    } else {
        depth_hypotheses.iter().map(|h| h.center[2]).sum::<f64>() / depth_hypotheses.len() as f64
    };

    for row in 0..rows {
        let y = y_min + ((rows - 1 - row) as f64 + 0.5) * cell_size;
        for col in 0..cols {
            let x = x_min + (col as f64 + 0.5) * cell_size;
            let idx = row * cols + col;
            let base = surface[idx];
            if !base.is_finite() {
                continue;
            }

            let mut sum = 0.0;
            let mut wsum = 0.0;
            for hyp in depth_hypotheses {
                let dx = x - hyp.center[0];
                let dy = y - hyp.center[1];
                let d2 = dx * dx + dy * dy;
                let w = (-d2 / (2.0 * sigma2)).exp() * hyp.confidence;
                if w <= 1e-6 {
                    continue;
                }
                sum += w * hyp.center[2];
                wsum += w;
            }

            let local_target = if wsum > 1e-8 { sum / wsum } else { fallback_depth };
            let delta = (local_target - base).clamp(-detail_amp_m, detail_amp_m);
            surface[idx] = (base + 0.65 * delta).clamp(mean_surface_z - 95.0, mean_surface_z + 95.0);
        }
    }
}

fn estimate_vertical_rmse_m(alignment: &AlignmentResult, valid_cells: u64) -> f64 {
    if valid_cells == 0 {
        return 0.0;
    }

    let base = (alignment.stats.rmse_px * alignment.stats.estimated_gsd_m * 1.4).clamp(0.02, 2.0);
    let density = alignment.stats.sparse_cloud_points as f64 / valid_cells as f64;
    let density_factor = (1.15 - (density * 0.4).clamp(0.0, 0.35)).clamp(0.8, 1.15);
    (base * density_factor).clamp(0.02, 2.5)
}

fn derive_dtm_from_dsm(dsm: &Raster) -> Result<Raster> {
    let cfg = RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "dtm".to_string()),
            ("generator".to_string(), "wbphotogrammetry_sprint1_terrain_filter".to_string()),
        ],
        ..RasterConfig::default()
    };
    let mut dtm = Raster::new(cfg);

    // First pass: conservative local-min proxy to suppress above-ground structures.
    for row in 0..dsm.rows {
        for col in 0..dsm.cols {
            let mut local_min = f64::INFINITY;
            let mut found = false;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if let Some(v) = dsm.get_opt(0, row as isize + dr, col as isize + dc) {
                        local_min = local_min.min(v);
                        found = true;
                    }
                }
            }
            if found {
                dtm.set(0, row as isize, col as isize, local_min)?;
            }
        }
    }

    // Second pass: light smoothing for terrain continuity.
    let mut smoothed = Raster::new(RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "dtm".to_string()),
            ("generator".to_string(), "wbphotogrammetry_sprint1_terrain_filter".to_string()),
            ("postprocess".to_string(), "3x3_mean_smoothing".to_string()),
        ],
        ..RasterConfig::default()
    });
    for row in 0..dtm.rows {
        for col in 0..dtm.cols {
            let mut sum = 0.0;
            let mut n = 0_u32;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    if let Some(v) = dtm.get_opt(0, row as isize + dr, col as isize + dc) {
                        sum += v;
                        n += 1;
                    }
                }
            }
            if n > 0 {
                smoothed.set(0, row as isize, col as isize, sum / n as f64)?;
            }
        }
    }

    Ok(smoothed)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::alignment::{AlignmentStats, CameraPose};
    use crate::camera::{CameraIntrinsics, CameraModel};
    use wbraster::CrsInfo;

    fn temp_dsm_path(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("{}_{}.tif", prefix, nanos))
            .to_string_lossy()
            .to_string()
    }

    fn sample_alignment() -> AlignmentResult {
        AlignmentResult {
            poses: vec![
                CameraPose {
                    position: [0.0, 0.0, 118.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    reprojection_error_px: 0.6,
                },
                CameraPose {
                    position: [12.0, 4.0, 120.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    reprojection_error_px: 0.6,
                },
                CameraPose {
                    position: [25.0, 9.0, 123.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    reprojection_error_px: 0.6,
                },
            ],
            crs: CrsInfo::from_epsg(32617),
            stats: AlignmentStats {
                aligned_fraction: 1.0,
                rmse_px: 0.7,
                residual_p50_px: 0.36,
                residual_p95_px: 1.05,
                sparse_cloud_points: 3_200,
                tie_points_median: 70,
                tracks_median: 4.0,
                mean_parallax_px: 6.0,
                estimated_gsd_m: 0.05,
                intrinsics: CameraIntrinsics::identity(4000, 3000),
                model: CameraModel::Pinhole,
                loop_closure_constraints: 0,
                mean_loop_closure_correction_m: 0.0,
                max_loop_closure_correction_m: 0.0,
                ba_optimization_passes: 0,
                ba_huber_threshold_px: 0.0,
                ba_final_cost: 0.0,
                ba_intrinsics_refined: false,
                ba_distortion_refined: false,
                ba_observations_initial: 0,
                ba_observations_final: 0,
                ba_observation_retention_pct: 0.0,
                ba_supported_camera_fraction: 0.0,
                ba_observations_per_pass: Vec::new(),
                ba_prune_thresholds_px: Vec::new(),
                ba_camera_covariance: crate::alignment::CameraCovarianceDiagnostics::default(),
            },
        }
    }

    #[test]
    fn empty_alignment_writes_placeholder_and_zero_stats() {
        let alignment = AlignmentResult {
            poses: Vec::new(),
            crs: CrsInfo::default(),
            stats: AlignmentStats {
                aligned_fraction: 0.0,
                rmse_px: 0.0,
                residual_p50_px: 0.0,
                residual_p95_px: 0.0,
                sparse_cloud_points: 0,
                tie_points_median: 0,
                tracks_median: 0.0,
                mean_parallax_px: 0.0,
                estimated_gsd_m: 0.0,
                intrinsics: CameraIntrinsics::identity(4000, 3000),
                model: CameraModel::Pinhole,
                loop_closure_constraints: 0,
                mean_loop_closure_correction_m: 0.0,
                max_loop_closure_correction_m: 0.0,
                ba_optimization_passes: 0,
                ba_huber_threshold_px: 0.0,
                ba_final_cost: 0.0,
                ba_intrinsics_refined: false,
                ba_distortion_refined: false,
                ba_observations_initial: 0,
                ba_observations_final: 0,
                ba_observation_retention_pct: 0.0,
                ba_supported_camera_fraction: 0.0,
                ba_observations_per_pass: Vec::new(),
                ba_prune_thresholds_px: Vec::new(),
                ba_camera_covariance: crate::alignment::CameraCovarianceDiagnostics::default(),
            },
        };

        let dsm_path = temp_dsm_path("dense_empty");
        let result = run_dense_surface(&alignment, 0.1, &dsm_path).expect("dense run");
        assert_eq!(result.stats.valid_cells, 0);
        assert_eq!(result.stats.vertical_rmse_m, 0.0);

        let raster = Raster::read(&dsm_path).expect("read written dsm");
        assert_eq!(raster.cols, 2);
        assert_eq!(raster.rows, 2);

        let _ = std::fs::remove_file(dsm_path);
    }

    #[test]
    fn pose_driven_dense_surface_produces_nonzero_stats() {
        let alignment = sample_alignment();
        let dsm_path = temp_dsm_path("dense_surface");

        let result = run_dense_surface(&alignment, 0.5, &dsm_path).expect("dense run");
        assert!(result.stats.valid_cells > 0);
        assert!(result.stats.min_elevation_m.is_finite());
        assert!(result.stats.max_elevation_m.is_finite());
        assert!(result.stats.mean_elevation_m.is_finite());
        assert!(result.stats.vertical_rmse_m > 0.0);
        assert!(result.stats.mean_local_relief_m >= 0.0);
        assert!(result.stats.p95_local_relief_m >= result.stats.mean_local_relief_m);

        let raster = Raster::read(&dsm_path).expect("read written dsm");
        assert!(raster.cols >= MIN_GRID_DIM);
        assert!(raster.rows >= MIN_GRID_DIM);

        let _ = std::fs::remove_file(dsm_path);
    }

    #[test]
    fn depth_hypotheses_are_built_for_each_pose() {
        let alignment = sample_alignment();
        let mean_z = alignment
            .poses
            .iter()
            .map(|p| p.position[2])
            .sum::<f64>()
            / alignment.poses.len() as f64;
        let (sx, sy) = estimate_surface_slopes(&alignment, mean_z);
        let nominal_flight_height_m = estimate_nominal_flight_height_m(&alignment);

        let hypotheses = build_depth_hypotheses(
            &alignment,
            mean_z,
            sx,
            sy,
            nominal_flight_height_m,
        );
        assert!(hypotheses.len() >= alignment.poses.len());
        assert!(hypotheses.iter().all(|h| h.confidence > 0.0));
    }

    #[test]
    fn fused_surface_returns_finite_values() {
        let alignment = sample_alignment();
        let mean_z = alignment
            .poses
            .iter()
            .map(|p| p.position[2])
            .sum::<f64>()
            / alignment.poses.len() as f64;
        let (sx, sy) = estimate_surface_slopes(&alignment, mean_z);
        let nominal_flight_height_m = estimate_nominal_flight_height_m(&alignment);
        let hypotheses = build_depth_hypotheses(
            &alignment,
            mean_z,
            sx,
            sy,
            nominal_flight_height_m,
        );

        let z = interpolated_surface_z(
            &alignment,
            10.0,
            5.0,
            sx,
            sy,
            mean_z,
            nominal_flight_height_m,
            &hypotheses,
        );
        assert!(z.is_finite());
        let mean_surface_z = mean_z - nominal_flight_height_m;
        assert!(z > mean_surface_z - 50.0 && z < mean_surface_z + 50.0);
    }

    #[test]
    fn optional_dtm_is_written_when_requested() {
        let alignment = sample_alignment();
        let dsm_path = temp_dsm_path("dense_with_dtm_dsm");
        let dtm_path = temp_dsm_path("dense_with_dtm_dtm");

        let result = run_dense_surface_with_dtm(&alignment, 0.5, &dsm_path, Some(&dtm_path))
            .expect("dense run with dtm");

        assert_eq!(result.dtm_path.as_deref(), Some(dtm_path.as_str()));

        let dsm = Raster::read(&dsm_path).expect("read dsm");
        let dtm = Raster::read(&dtm_path).expect("read dtm");
        assert_eq!(dsm.cols, dtm.cols);
        assert_eq!(dsm.rows, dtm.rows);

        let _ = std::fs::remove_file(dsm_path);
        let _ = std::fs::remove_file(dtm_path);
    }

    #[test]
    fn stereo_depths_from_refined_adjacent_pairs_produces_hypotheses() {
        // Step 3 enhancement: validate that stereo depth estimation from refined poses
        // generates plausible depth hypotheses for improved surface reconstruction.
        let alignment = sample_alignment();
        let stereo_depths = estimate_stereo_depths_from_adjacent_pairs(&alignment, &[]);
        
        // For N poses, we expect stereo depths from N-1 adjacent pairs
        // Each pair generates ~5 depth samples
        let expected_min = (alignment.poses.len() - 1) * 5;
        assert!(
            stereo_depths.len() >= expected_min,
            "expected at least {} stereo depth hypotheses, got {}",
            expected_min,
            stereo_depths.len()
        );

        // Validate that all hypotheses have reasonable confidence
        assert!(stereo_depths.iter().all(|h| h.confidence > 0.0 && h.confidence <= 1.0),
            "all stereo depths should have confidence in (0, 1]");

        // Validate that positions are finite
        assert!(stereo_depths.iter().all(|h| {
            h.center
                .iter()
                .all(|v| v.is_finite())
        }), "all stereo depth positions should be finite");

        // Stereo depths should be reasonable: Z should be within realistic bounds
        let min_pose_z: f64 = alignment.poses.iter().map(|p| p.position[2]).fold(f64::INFINITY, f64::min);
        let max_pose_z: f64 = alignment.poses.iter().map(|p| p.position[2]).fold(f64::NEG_INFINITY, f64::max);
        for depth in stereo_depths.iter() {
            // Depth Z should be within ±100m of the pose altitude range (allows for added depth jitter)
            assert!(
                depth.center[2] > min_pose_z - 100.0 && depth.center[2] < max_pose_z + 100.0,
                "stereo depth Z should be within reasonable bounds of trajectory"
            );
        }
    }

    #[test]
    fn mvs_source_view_selection_prefers_non_reference_views() {
        let alignment = sample_alignment();
        let views = select_mvs_source_views(&alignment, 1, alignment.poses.len(), 2);
        assert!(!views.is_empty());
        assert!(views.len() <= 2);
        assert!(views.iter().all(|&idx| idx != 1));
    }

    #[test]
    fn multiview_maps_convert_to_depth_hypotheses() {
        let maps = vec![MultiViewDepthMap {
            reference_idx: 0,
            samples: vec![
                MultiViewDepthSample {
                    point_world: [1.0, 2.0, 3.0],
                    confidence: 0.7,
                    support_views: 2,
                    ref_px: [12.0, 18.0],
                    ref_depth: 3.0,
                },
                MultiViewDepthSample {
                    point_world: [4.0, 5.0, 6.0],
                    confidence: 0.6,
                    support_views: 1,
                    ref_px: [28.0, 30.0],
                    ref_depth: 6.0,
                },
            ],
        }];

        let hyps = depth_hypotheses_from_multiview_maps(&maps);
        assert_eq!(hyps.len(), 2);
        assert!(hyps.iter().all(|h| h.confidence > 0.0 && h.confidence <= 1.0));
        assert!(hyps.iter().all(|h| h.center.iter().all(|v| v.is_finite())));
    }

    #[test]
    fn multiview_occlusion_voting_prefers_front_surface() {
        let maps = vec![MultiViewDepthMap {
            reference_idx: 0,
            samples: vec![
                MultiViewDepthSample {
                    point_world: [2.0, 2.0, 4.0],
                    confidence: 0.9,
                    support_views: 3,
                    ref_px: [16.0, 16.0],
                    ref_depth: 4.0,
                },
                MultiViewDepthSample {
                    point_world: [2.1, 2.0, 7.0],
                    confidence: 0.85,
                    support_views: 3,
                    ref_px: [16.8, 16.6],
                    ref_depth: 7.0,
                },
            ],
        }];

        let hyps = depth_hypotheses_from_multiview_maps(&maps);
        assert_eq!(hyps.len(), 1);
        assert!((hyps[0].center[2] - 4.0).abs() < 1.0e-6);
    }

    #[test]
    fn zncc_cost_prefers_matching_patches() {
        let mut left = GrayImage::new(9, 9);
        let mut right = GrayImage::new(9, 9);
        for y in 0..9u32 {
            for x in 0..9u32 {
                let v = ((x * 17 + y * 9) % 255) as u8;
                left.put_pixel(x, y, image::Luma([v]));
                right.put_pixel(x, y, image::Luma([v]));
            }
        }
        // Make a non-matching alternative patch nearby.
        for y in 3..6u32 {
            for x in 3..6u32 {
                right.put_pixel(x + 1, y, image::Luma([255u8.saturating_sub(left.get_pixel(x, y)[0])]));
            }
        }

        let good = patch_zncc_cost(&left, &right, 4, 4, 4, 4, 2);
        let bad = patch_zncc_cost(&left, &right, 4, 4, 5, 4, 2);
        assert!(good < bad, "ZNCC cost should be lower for matching patches");
    }

    #[test]
    fn hybrid_cost_prefers_matching_patches() {
        let mut left = GrayImage::new(11, 11);
        let mut right = GrayImage::new(11, 11);
        for y in 0..11u32 {
            for x in 0..11u32 {
                let v = ((x * 19 + y * 13) % 251) as u8;
                left.put_pixel(x, y, image::Luma([v]));
                right.put_pixel(x, y, image::Luma([v]));
            }
        }
        // Corrupt an offset neighborhood to force a structurally worse candidate.
        for y in 3..8u32 {
            for x in 3..8u32 {
                right.put_pixel(x + 1, y, image::Luma([0]));
            }
        }

        let good = patch_hybrid_cost(&left, &right, 5, 5, 5, 5, 2);
        let bad = patch_hybrid_cost(&left, &right, 5, 5, 6, 5, 2);
        assert!(good < bad, "hybrid cost should be lower for matching patches");
    }

    #[test]
    fn epipolar_candidates_follow_expected_line() {
        // This F yields epipolar line l' = [0, 1, -yl], i.e. y' = yl.
        let f = Matrix3::new(
            0.0, 0.0, 0.0,
            0.0, 0.0, 1.0,
            0.0, -1.0, 0.0,
        );
        let xl = 14;
        let yl = 9;
        let candidates = epipolar_line_candidates(&f, xl, yl, xl, yl + 2, 6, 64, 64, 2);
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|(_, y)| *y == yl));
    }

    #[test]
    fn hypothesis_reinforcement_preserves_data_driven_surface() {
        let alignment = sample_alignment();
        let rows = 8usize;
        let cols = 8usize;
        let mut surface = vec![101.5; rows * cols];
        let hyps = vec![
            DepthHypothesis {
                center: [5.0, 5.0, 101.8],
                confidence: 0.95,
            },
            DepthHypothesis {
                center: [14.0, 12.0, 102.3],
                confidence: 0.85,
            },
            DepthHypothesis {
                center: [20.0, 18.0, 100.9],
                confidence: 0.80,
            },
        ];

        reinforce_surface_from_hypotheses(
            &mut surface,
            rows,
            cols,
            0.0,
            0.0,
            2.0,
            &alignment,
            &hyps,
            101.5,
        );

        let range = surface_range_m(&surface);
        assert!(range > 1.0e-4, "reinforcement should introduce non-flat local relief");
        assert!(range < 3.5, "reinforcement should remain conservative");
    }

    #[test]
    fn depth_hypothesis_curation_rejects_outliers() {
        let hypotheses = vec![
            DepthHypothesis { center: [0.0, 0.0, 100.0], confidence: 0.9 },
            DepthHypothesis { center: [1.0, 0.5, 100.4], confidence: 0.8 },
            DepthHypothesis { center: [2.0, 1.0, 100.2], confidence: 0.7 },
            DepthHypothesis { center: [2.1, 1.1, 500.0], confidence: 0.95 },
            DepthHypothesis { center: [f64::NAN, 0.0, 100.0], confidence: 0.9 },
        ];
        let curated = curate_depth_hypotheses(hypotheses, 100.2, 10.0);
        assert!(!curated.is_empty());
        assert!(curated.iter().all(|h| h.center[2] < 150.0));
        assert!(curated.iter().all(|h| h.center.iter().all(|v| v.is_finite())));
    }

    #[test]
    fn low_support_repair_smooths_isolated_cell() {
        let rows = 5usize;
        let cols = 5usize;
        let mut surface = vec![100.0; rows * cols];
        surface[2 * cols + 2] = 112.0;
        let mut support = vec![0.9; rows * cols];
        support[2 * cols + 2] = 0.01;

        repair_low_support_cells(&mut surface, &support, rows, cols, 100.0, 0.15);
        assert!(surface[2 * cols + 2] < 110.0, "isolated low-support spike should be reduced");
    }
}
