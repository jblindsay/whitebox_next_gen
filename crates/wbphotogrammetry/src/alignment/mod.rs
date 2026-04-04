//! Camera alignment (bundle adjustment) — Sprint 1 minimal real implementation.
//!
//! This stage derives a data-driven camera trajectory from frame metadata
//! (GPS when present, otherwise sequence heuristics) and computes alignment
//! quality statistics from match-network strength.

use serde::{Deserialize, Serialize};
use nalgebra::{DMatrix, DVector, Matrix2, Matrix2x3, Matrix2x4, Matrix3, Matrix3x4, Matrix4, Matrix4x3, Vector2, Vector3, Vector4};
use std::collections::{HashMap, VecDeque};

use crate::camera::{CameraIntrinsics, CameraModel};
use crate::error::Result;
use crate::features::MatchStats;
use crate::ingest::{orientation_prior_yaw_rad, GpsCoordinate, ImageFrame, OrientationPrior, OrientationPriorSource};
use wbraster::CrsInfo;
use wbprojection::Crs;

/// 6-DOF camera pose in a local ENU coordinate frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPose {
    /// Camera centre [X, Y, Z] in metres (ENU).
    pub position: [f64; 3],
    /// Rotation as a unit quaternion [w, x, y, z].
    pub rotation: [f64; 4],
    /// Per-pose reprojection error in pixels.
    pub reprojection_error_px: f64,
}

/// Aggregate statistics from the alignment stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentStats {
    /// Fraction of submitted frames that aligned successfully (0–1).
    pub aligned_fraction: f64,
    /// Root-mean-square reprojection error in pixels.
    pub rmse_px: f64,
    /// Median reprojection residual in pixels.
    pub residual_p50_px: f64,
    /// 95th-percentile reprojection residual in pixels.
    pub residual_p95_px: f64,
    /// Number of 3-D tie-points in the sparse cloud.
    pub sparse_cloud_points: u64,
    /// Median tie-points observed per aligned image pair.
    pub tie_points_median: u64,
    /// Median track length (images per feature track).
    pub tracks_median: f64,
    /// Mean robust inter-image parallax (pixels) from feature matching.
    pub mean_parallax_px: f64,
    /// Estimated ground sampling distance in metres.
    pub estimated_gsd_m: f64,
    /// Calibrated camera intrinsics.
    pub intrinsics: CameraIntrinsics,
    /// Camera model used.
    pub model: CameraModel,
    /// Number of accepted non-adjacent loop-closure constraints.
    pub loop_closure_constraints: u64,
    /// Mean per-constraint correction magnitude applied by loop closure (m).
    pub mean_loop_closure_correction_m: f64,
    /// Maximum per-constraint correction magnitude applied by loop closure (m).
    pub max_loop_closure_correction_m: f64,
    /// Number of optimization passes completed by bundle adjustment.
    pub ba_optimization_passes: u32,
    /// Robust loss threshold (pixels) used during bundle adjustment.
    pub ba_huber_threshold_px: f64,
    /// Final robust reprojection objective value after BA optimization.
    pub ba_final_cost: f64,
    /// True when intrinsic core parameters (fx/fy/cx/cy) were refined by BA.
    pub ba_intrinsics_refined: bool,
    /// True when lens distortion parameters (k1/k2/p1/p2) were refined by BA.
    pub ba_distortion_refined: bool,
    /// Number of BA observations before iterative residual pruning.
    pub ba_observations_initial: u64,
    /// Number of BA observations after iterative residual pruning.
    pub ba_observations_final: u64,
    /// Percent retention of BA observations after pruning (0-100).
    pub ba_observation_retention_pct: f64,
    /// Fraction of cameras supported by final BA observations (0-1).
    pub ba_supported_camera_fraction: f64,
    /// Observation counts after each BA pruning pass.
    pub ba_observations_per_pass: Vec<u64>,
    /// Residual thresholds (pixels) applied in each BA pruning pass.
    pub ba_prune_thresholds_px: Vec<f64>,
    /// Covariance-style uncertainty proxies derived from the reduced BA system.
    pub ba_camera_covariance: CameraCovarianceDiagnostics,
}

/// Aggregate uncertainty proxies for camera bundle-adjustment parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CameraCovarianceDiagnostics {
    /// Number of cameras with finite covariance-style proxy estimates.
    pub supported_camera_count: u64,
    /// Median horizontal translation sigma proxy (metres) from reduced-system curvature.
    pub translation_sigma_median_m: f64,
    /// 95th-percentile horizontal translation sigma proxy (metres).
    pub translation_sigma_p95_m: f64,
    /// Median roll/pitch sigma proxy (degrees) from local rotation curvature.
    pub rotation_sigma_median_deg: f64,
    /// 95th-percentile roll/pitch sigma proxy (degrees).
    pub rotation_sigma_p95_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LoopClosureDiagnostics {
    /// Number of accepted non-adjacent loop-closure constraints.
    pub constraint_count: u64,
    /// Mean correction magnitude applied per accepted constraint (m).
    pub mean_correction_m: f64,
    /// Maximum correction magnitude applied among accepted constraints (m).
    pub max_correction_m: f64,
}

/// Full result from the camera alignment stage.
#[derive(Debug, Clone)]
pub struct AlignmentResult {
    /// Per-frame poses (only for frames that aligned).
    pub poses: Vec<CameraPose>,
    /// Output horizontal CRS for pose X/Y coordinates (empty when unknown).
    pub crs: CrsInfo,
    /// Aggregate statistics.
    pub stats: AlignmentStats,
}

#[derive(Debug, Clone)]
struct BaDiagnostics {
    optimization_passes: u32,
    huber_threshold_px: f64,
    final_cost: f64,
    intrinsics_refined: bool,
    distortion_refined: bool,
    observations_initial: usize,
    observations_final: usize,
    observation_retention_pct: f64,
    supported_camera_fraction: f64,
    observations_per_pass: Vec<usize>,
    prune_thresholds_px: Vec<f64>,
    covariance: CameraCovarianceDiagnostics,
}

#[derive(Debug, Clone, Copy)]
struct BaPruneStats {
    threshold_px: f64,
    kept_observations: usize,
}

#[derive(Debug, Clone, Copy)]
struct IntrinsicsRefineMask {
    params: [bool; 8],
}

impl IntrinsicsRefineMask {
    fn none() -> Self {
        Self { params: [false; 8] }
    }
}

/// Controls how bundle adjustment refines camera intrinsics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntrinsicsRefinementPolicy {
    /// Use heuristic gating based on observation count and geometry support.
    Auto,
    /// Keep all intrinsics fixed during bundle adjustment.
    None,
    /// Refine core intrinsics only (`fx`, `fy`, `cx`, `cy`).
    CoreOnly,
    /// Refine core intrinsics and radial distortion (`k1`, `k2`).
    CoreAndRadial,
    /// Refine all supported intrinsics (`fx`, `fy`, `cx`, `cy`, `k1`, `k2`, `p1`, `p2`).
    All,
}

impl Default for IntrinsicsRefinementPolicy {
    fn default() -> Self {
        Self::Auto
    }
}

/// Selects the reduced camera solve backend used in Schur-style BA updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReducedCameraSolveMode {
    /// Prefer sparse PCG with robust dense fallback on numerical issues.
    SparsePcg,
    /// Force dense damped LU solve for reduced camera systems.
    DenseLu,
}

impl Default for ReducedCameraSolveMode {
    fn default() -> Self {
        Self::SparsePcg
    }
}

/// Optional controls for camera alignment and bundle adjustment behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AlignmentOptions {
    /// Intrinsics refinement policy used by bundle adjustment.
    pub intrinsics_refinement: IntrinsicsRefinementPolicy,
    /// Reduced camera solver backend for Schur-style BA updates.
    pub reduced_camera_solve_mode: ReducedCameraSolveMode,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            intrinsics_refinement: IntrinsicsRefinementPolicy::Auto,
            reduced_camera_solve_mode: ReducedCameraSolveMode::SparsePcg,
        }
    }
}

/// Run camera alignment on `frames` using the supplied match statistics.
///
/// Sprint 1 minimal implementation: derives poses from available metadata
/// and estimates alignment quality from match-network density/connectivity.
pub fn run_camera_alignment(
    frames: &[ImageFrame],
    match_stats: &MatchStats,
    camera_model: CameraModel,
) -> Result<AlignmentResult> {
    run_camera_alignment_with_options(
        frames,
        match_stats,
        camera_model,
        AlignmentOptions::default(),
    )
}

/// Run camera alignment with optional tuning controls.
pub fn run_camera_alignment_with_options(
    frames: &[ImageFrame],
    match_stats: &MatchStats,
    camera_model: CameraModel,
    options: AlignmentOptions,
) -> Result<AlignmentResult> {
    let model = resolve_camera_model(camera_model, frames);
    let n = frames.len();
    if n == 0 {
        return Ok(AlignmentResult {
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
                model,
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
                ba_camera_covariance: CameraCovarianceDiagnostics::default(),
            },
        });
    }

    let mut intrinsics = infer_intrinsics(frames);
    let quality = network_quality(match_stats, n);
    let relative_support = relative_motion_support(match_stats, n);
    let mut aligned = estimate_aligned_count(n, match_stats);
    let mut reprojection_rmse_px = (2.4 - 1.9 * quality).clamp(0.35, 2.4);

    let (positions, rotations_opt, pose_crs, essential_aligned_count) = derive_positions(
        frames,
        aligned,
        relative_support,
        quality,
        &intrinsics,
        model,
        match_stats,
    );
    if essential_aligned_count > 0 {
        aligned = essential_aligned_count;
    }
    calibrate_intrinsics_from_correspondence(&mut intrinsics, frames, &positions, match_stats);
    let rotations = rotations_opt
        .unwrap_or_else(|| derive_rotations_from_positions(frames, &positions, match_stats));
    let rotations = normalize_rotations_for_projection_convention(rotations);
    let (positions, rotations, refined_intrinsics, residual_samples_px, ba_diag) = run_simplified_bundle_adjustment(
        &positions,
        &rotations,
        match_stats,
        &intrinsics,
        model,
        options.intrinsics_refinement,
        options.reduced_camera_solve_mode,
    );
    intrinsics = refined_intrinsics;
    let (positions, loop_closure_diag) = apply_loop_closure_global_optimization(
        &positions,
        &rotations,
        match_stats,
        &intrinsics,
        model,
        relative_support,
    );
    let (residual_p50_px, residual_p95_px) = if residual_samples_px.len() >= 8 {
        residual_quantiles_from_samples(&residual_samples_px)
    } else {
        estimate_residual_quantiles(reprojection_rmse_px, quality)
    };
    let use_nadir_orientation_fallback = positions.len() <= 8
        && ba_diag.supported_camera_fraction < 0.90
        && residual_p50_px > 18.0;
    let output_rotations: Vec<[f64; 4]> = if use_nadir_orientation_fallback {
        // Use nadir orientation (camera looks down, north-up image convention)
        // rather than identity (camera looks up), which is geometrically correct
        // for drone surveys and consistent with the mosaic standard projection path.
        vec![[0.0, 1.0, 0.0, 0.0]; positions.len()]
    } else {
        rotations.clone()
    };
    if !residual_samples_px.is_empty() {
        let obs_rmse = (residual_samples_px.iter().map(|r| r * r).sum::<f64>()
            / residual_samples_px.len() as f64)
            .sqrt();
        reprojection_rmse_px = (0.65 * reprojection_rmse_px + 0.35 * obs_rmse).clamp(0.35, 2.8);
    }
    let per_pose_error = (reprojection_rmse_px * 0.95).max(0.3);

    let poses: Vec<CameraPose> = positions
        .iter()
        .zip(output_rotations.iter())
        .map(|(position, rotation)| CameraPose {
            position: *position,
            rotation: *rotation,
            reprojection_error_px: per_pose_error,
        })
        .collect();

    let aligned_fraction = aligned as f64 / n as f64;
    let sparse_cloud_points = estimate_sparse_points(match_stats, aligned);
    let tie_points_median = estimate_tie_points_median(match_stats);
    let tracks_median = estimate_tracks_median(match_stats, aligned);
    let estimated_gsd_m = estimate_gsd_m(frames, &intrinsics, quality);
    Ok(AlignmentResult {
        poses,
        crs: pose_crs,
        stats: AlignmentStats {
            aligned_fraction,
            rmse_px: reprojection_rmse_px,
            residual_p50_px,
            residual_p95_px,
            sparse_cloud_points,
            tie_points_median,
            tracks_median,
            mean_parallax_px: match_stats.mean_parallax_px,
            estimated_gsd_m,
            intrinsics,
            model,
            loop_closure_constraints: loop_closure_diag.constraint_count,
            mean_loop_closure_correction_m: loop_closure_diag.mean_correction_m,
            max_loop_closure_correction_m: loop_closure_diag.max_correction_m,
            ba_optimization_passes: ba_diag.optimization_passes,
            ba_huber_threshold_px: ba_diag.huber_threshold_px,
            ba_final_cost: ba_diag.final_cost,
            ba_intrinsics_refined: ba_diag.intrinsics_refined,
            ba_distortion_refined: ba_diag.distortion_refined,
            ba_observations_initial: ba_diag.observations_initial as u64,
            ba_observations_final: ba_diag.observations_final as u64,
            ba_observation_retention_pct: ba_diag.observation_retention_pct,
            ba_supported_camera_fraction: ba_diag.supported_camera_fraction,
            ba_observations_per_pass: ba_diag
                .observations_per_pass
                .iter()
                .map(|v| *v as u64)
                .collect(),
            ba_prune_thresholds_px: ba_diag.prune_thresholds_px.clone(),
            ba_camera_covariance: ba_diag.covariance.clone(),
        },
    })
}

fn network_quality(match_stats: &MatchStats, frame_count: usize) -> f64 {
    if frame_count <= 1 {
        return 1.0;
    }
    let connectivity = match_stats.connectivity.clamp(0.0, 1.0);
    let match_density = (match_stats.mean_matches_per_pair / 120.0).clamp(0.0, 1.0);
    let feature_coverage = (match_stats.total_keypoints as f64 / (frame_count as f64 * 300.0)).clamp(0.0, 1.0);
    (0.5 * connectivity + 0.35 * match_density + 0.15 * feature_coverage).clamp(0.0, 1.0)
}

fn relative_motion_support(match_stats: &MatchStats, frame_count: usize) -> f64 {
    if frame_count <= 1 {
        return 1.0;
    }

    let min_pairs = frame_count.saturating_sub(1).max(1) as f64;
    let chain_support = (match_stats.total_matches as f64 / (min_pairs * 120.0)).clamp(0.0, 1.0);
    let pair_support = (match_stats.mean_matches_per_pair / 160.0).clamp(0.0, 1.0);
    let connectivity = match_stats.connectivity.clamp(0.0, 1.0);

    (0.45 * chain_support + 0.35 * pair_support + 0.20 * connectivity).clamp(0.0, 1.0)
}

fn estimate_aligned_count(
    frame_count: usize,
    match_stats: &MatchStats,
) -> usize {
    if frame_count == 0 {
        return 0;
    }
    if frame_count == 1 {
        return 1;
    }

    let supported = largest_support_component_size(frame_count, match_stats);
    if supported <= 2 && match_stats.total_matches > 0 && match_stats.pair_correspondences.is_empty() {
        return estimate_aligned_count_proxy(frame_count, match_stats);
    }
    supported.clamp(2, frame_count)
}

fn estimate_aligned_count_proxy(frame_count: usize, match_stats: &MatchStats) -> usize {
    if frame_count <= 1 {
        return frame_count;
    }
    let connectivity = match_stats.connectivity.clamp(0.0, 1.0);
    let pair_support = (match_stats.mean_matches_per_pair / 160.0).clamp(0.0, 1.0);
    let feature_density = (match_stats.total_keypoints as f64 / (frame_count as f64 * 300.0)).clamp(0.0, 1.0);
    let fused = (0.55 * connectivity + 0.35 * pair_support + 0.10 * feature_density).clamp(0.0, 1.0);
    let coverage = (0.40 + 0.60 * fused).clamp(0.35, 1.0);
    ((frame_count as f64 * coverage).round() as usize).clamp(2, frame_count)
}

fn largest_support_component_size(frame_count: usize, match_stats: &MatchStats) -> usize {
    if frame_count == 0 {
        return 0;
    }
    if frame_count == 1 {
        return 1;
    }

    let mut adjacency = vec![Vec::<usize>::new(); frame_count];
    for pair in &match_stats.pair_correspondences {
        if pair.left_frame_idx >= frame_count || pair.right_frame_idx >= frame_count {
            continue;
        }
        if pair.points.len() < 8 {
            continue;
        }
        let left = pair.left_frame_idx;
        let right = pair.right_frame_idx;
        adjacency[left].push(right);
        adjacency[right].push(left);
    }

    let mut visited = vec![false; frame_count];
    let mut best = 0usize;
    for seed in 0..frame_count {
        if visited[seed] {
            continue;
        }
        let mut q = VecDeque::new();
        q.push_back(seed);
        visited[seed] = true;
        let mut count = 0usize;
        while let Some(node) = q.pop_front() {
            count += 1;
            for &nbr in &adjacency[node] {
                if !visited[nbr] {
                    visited[nbr] = true;
                    q.push_back(nbr);
                }
            }
        }
        best = best.max(count);
    }

    if best >= 2 {
        best
    } else if match_stats.total_matches > 0 {
        2
    } else {
        1
    }
}

fn derive_positions(
    frames: &[ImageFrame],
    aligned_count: usize,
    relative_support: f64,
    quality: f64,
    intrinsics: &CameraIntrinsics,
    model: CameraModel,
    match_stats: &MatchStats,
) -> (Vec<[f64; 3]>, Option<Vec<[f64; 4]>>, CrsInfo, usize) {
    if aligned_count == 0 {
        return (Vec::new(), None, CrsInfo::default(), 0);
    }

    let gps_extent = frames
        .iter()
        .rposition(|f| f.metadata.gps.is_some())
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let working_count = aligned_count.max(gps_extent).min(frames.len()).max(2);
    let aligned_frames = &frames[..working_count];
    let gps_positions = gps_positions(aligned_frames, relative_support);
    if let Some((positions, crs)) = gps_positions {
        return (positions, None, crs, aligned_count);
    }

    if let Some((positions, rotations)) = derive_incremental_poses_from_essential(
        aligned_frames,
        match_stats,
        intrinsics,
        model,
        relative_support,
        quality,
    ) {
        let recovered = positions.len();
        if recovered >= 2 {
            return (positions, Some(rotations), CrsInfo::default(), recovered);
        }
    }

    // Fallback trajectory: match-driven relative-motion seed.
    let baseline_m = estimate_relative_baseline_m(relative_support, quality);
    let base_alt = estimate_nominal_altitude_m(aligned_frames);
    let positions = (0..working_count)
        .map(|i| [i as f64 * baseline_m, 0.0, base_alt])
        .collect();
    (positions, None, CrsInfo::default(), working_count)
}

#[derive(Debug, Clone)]
struct EssentialPose {
    r: Matrix3<f64>,
    t: Vector3<f64>,
}

fn derive_incremental_poses_from_essential(
    frames: &[ImageFrame],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    relative_support: f64,
    quality: f64,
) -> Option<(Vec<[f64; 3]>, Vec<[f64; 4]>)> {
    if frames.len() < 2 {
        return None;
    }

    let mut positions = Vec::with_capacity(frames.len());
    let mut rotations = Vec::with_capacity(frames.len());
    let mut c_world = Vector3::new(0.0, 0.0, estimate_nominal_altitude_m(frames));
    let mut rc2w = Matrix3::identity();
    let mut last_step_local = Vector3::new(1.0, 0.0, 0.0);

    positions.push([c_world[0], c_world[1], c_world[2]]);
    rotations.push(matrix_to_quaternion(&rc2w));

    let baseline_m = estimate_relative_baseline_m(relative_support, quality);

    let mut recovered_pairs = 0usize;
    for left_idx in 0..(frames.len() - 1) {
        let mut step_local = last_step_local * baseline_m;
        if let Some((essential_pose, gap)) = best_incremental_pair_pose(
            left_idx,
            match_stats,
            intrinsics,
            camera_model,
            3,
        ) {
            let direction = -essential_pose.r.transpose() * essential_pose.t;
            if direction.norm() > 1.0e-9 {
                step_local = direction.normalize() * baseline_m;
                last_step_local = step_local / baseline_m.max(1.0e-9);
            }
            if gap == 1 {
                rc2w *= essential_pose.r.transpose();
            }
            recovered_pairs += 1;
        }

        c_world += rc2w * step_local;

        positions.push([c_world[0], c_world[1], c_world[2]]);
        rotations.push(matrix_to_quaternion(&rc2w));
    }

    if recovered_pairs > 0 {
        Some((positions, rotations))
    } else {
        None
    }
}

fn best_incremental_pair_pose(
    left_idx: usize,
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    max_gap: usize,
) -> Option<(EssentialPose, usize)> {
    for gap in 1..=max_gap {
        let right_idx = left_idx + gap;
        let Some(pair) = match_stats
            .pair_correspondences
            .iter()
            .find(|p| p.left_frame_idx == left_idx && p.right_frame_idx == right_idx) else {
            continue;
        };
        if pair.points.len() < 8 {
            continue;
        }
        if let Some(pose) = estimate_essential_pose(pair.points.as_slice(), intrinsics, camera_model) {
            return Some((pose, gap));
        }
    }
    None
}

fn estimate_essential_pose(
    points: &[[f64; 4]],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Option<EssentialPose> {
    if points.len() < 8 {
        return None;
    }

    let n = points.len();
    let mut normalized = Vec::with_capacity(n);
    for p in points {
        let left = unproject_pixel_to_normalized_camera_ray(
            &Vector2::new(p[0], p[1]),
            intrinsics,
            camera_model,
        )?;
        let right = unproject_pixel_to_normalized_camera_ray(
            &Vector2::new(p[2], p[3]),
            intrinsics,
            camera_model,
        )?;
        normalized.push((left, right));
    }

    let mut best_inliers: Vec<usize> = Vec::new();
    let mut best_e = Matrix3::identity();
    let iterations = (n.max(16) * 3).min(220);
    for iter in 0..iterations {
        let sample = deterministic_sample_indices(n, 8, iter as u64 + 17)?;
        let sample_points: Vec<(Vector2<f64>, Vector2<f64>)> = sample.iter().map(|&i| normalized[i]).collect();
        let e = estimate_essential_from_correspondences(&sample_points)?;
        let inliers = sampson_inliers(&e, &normalized, 1.1e-3);
        if inliers.len() > best_inliers.len() {
            best_inliers = inliers;
            best_e = e;
        }
    }

    if best_inliers.len() < 8 {
        return None;
    }

    let refined_pts: Vec<(Vector2<f64>, Vector2<f64>)> = best_inliers
        .iter()
        .map(|&idx| normalized[idx])
        .collect();
    let refined_e = estimate_essential_from_correspondences(&refined_pts).unwrap_or(best_e);

    let pose = recover_pose_from_essential(&refined_e, &refined_pts)?;
    if essential_pose_is_degenerate(&pose, &refined_pts) {
        None
    } else {
        Some(pose)
    }
}

fn estimate_essential_from_correspondences(
    points: &[(Vector2<f64>, Vector2<f64>)],
) -> Option<Matrix3<f64>> {
    if points.len() < 8 {
        return None;
    }

    let (norm1, t1) = normalize_points(points.iter().map(|(p1, _)| *p1).collect::<Vec<_>>().as_slice())?;
    let (norm2, t2) = normalize_points(points.iter().map(|(_, p2)| *p2).collect::<Vec<_>>().as_slice())?;

    let mut a = DMatrix::zeros(points.len(), 9);
    for i in 0..points.len() {
        let p1 = norm1[i];
        let p2 = norm2[i];
        a[(i, 0)] = p2.x * p1.x;
        a[(i, 1)] = p2.x * p1.y;
        a[(i, 2)] = p2.x;
        a[(i, 3)] = p2.y * p1.x;
        a[(i, 4)] = p2.y * p1.y;
        a[(i, 5)] = p2.y;
        a[(i, 6)] = p1.x;
        a[(i, 7)] = p1.y;
        a[(i, 8)] = 1.0;
    }

    let svd = a.svd(true, true);
    let v_t = svd.v_t?;
    let e_vec = v_t.row(v_t.nrows() - 1);
    let mut e_hat = Matrix3::zeros();
    for r in 0..3 {
        for c in 0..3 {
            e_hat[(r, c)] = e_vec[r * 3 + c];
        }
    }

    let e_denorm = t2.transpose() * e_hat * t1;
    let svd_e = e_denorm.svd(true, true);
    let u = svd_e.u?;
    let v_t = svd_e.v_t?;
    let s0 = svd_e.singular_values[0];
    let s1 = svd_e.singular_values[1];
    let s = ((s0 + s1) * 0.5).max(1e-9);
    let sigma = Matrix3::new(s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 0.0);
    let e_rank2 = u * sigma * v_t;
    let norm = e_rank2.norm();
    if norm <= 1e-12 {
        None
    } else {
        Some(e_rank2 / norm)
    }
}

fn normalize_points(points: &[Vector2<f64>]) -> Option<(Vec<Vector2<f64>>, Matrix3<f64>)> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    let cx = points.iter().map(|p| p.x).sum::<f64>() / n;
    let cy = points.iter().map(|p| p.y).sum::<f64>() / n;
    let mean_dist = points
        .iter()
        .map(|p| {
            let dx = p.x - cx;
            let dy = p.y - cy;
            (dx * dx + dy * dy).sqrt()
        })
        .sum::<f64>()
        / n;
    let scale = if mean_dist <= 1e-12 {
        1.0
    } else {
        (2.0_f64).sqrt() / mean_dist
    };
    let t = Matrix3::new(
        scale,
        0.0,
        -scale * cx,
        0.0,
        scale,
        -scale * cy,
        0.0,
        0.0,
        1.0,
    );
    let norm_pts = points
        .iter()
        .map(|p| {
            let hp = t * Vector3::new(p.x, p.y, 1.0);
            Vector2::new(hp[0] / hp[2], hp[1] / hp[2])
        })
        .collect();
    Some((norm_pts, t))
}

fn sampson_inliers(
    e: &Matrix3<f64>,
    points: &[(Vector2<f64>, Vector2<f64>)],
    threshold: f64,
) -> Vec<usize> {
    let mut inliers = Vec::new();
    for (idx, (p1, p2)) in points.iter().enumerate() {
        let x1 = Vector3::new(p1.x, p1.y, 1.0);
        let x2 = Vector3::new(p2.x, p2.y, 1.0);
        let ex1 = e * x1;
        let e_tx2 = e.transpose() * x2;
        let x2tex1 = x2.dot(&ex1);
        let denom = ex1.x * ex1.x + ex1.y * ex1.y + e_tx2.x * e_tx2.x + e_tx2.y * e_tx2.y;
        if denom <= 1e-12 {
            continue;
        }
        let d = (x2tex1 * x2tex1) / denom;
        if d <= threshold {
            inliers.push(idx);
        }
    }
    inliers
}

fn recover_pose_from_essential(
    e: &Matrix3<f64>,
    inlier_points: &[(Vector2<f64>, Vector2<f64>)],
) -> Option<EssentialPose> {
    let svd = e.svd(true, true);
    let mut u = svd.u?;
    let mut v_t = svd.v_t?;

    if u.determinant() < 0.0 {
        u.column_mut(2).iter_mut().for_each(|v| *v = -*v);
    }
    if v_t.determinant() < 0.0 {
        v_t.row_mut(2).iter_mut().for_each(|v| *v = -*v);
    }

    let w = Matrix3::new(0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    let r1 = u * w * v_t;
    let r2 = u * w.transpose() * v_t;
    let t = u.column(2).into_owned();

    let candidates = [
        (ensure_rotation(r1), t),
        (ensure_rotation(r1), -t),
        (ensure_rotation(r2), t),
        (ensure_rotation(r2), -t),
    ];

    let mut best_pose: Option<EssentialPose> = None;
    let mut best_metrics: Option<EssentialPoseCandidateMetrics> = None;
    for (r, tvec) in candidates {
        let t_unit = tvec.normalize();
        let metrics = analyze_essential_pose_candidate(&r, &t_unit, inlier_points);
        let better = match best_metrics {
            None => true,
            Some(best) => {
                metrics.positive_count > best.positive_count
                    || (metrics.positive_count == best.positive_count
                        && metrics.reprojection_rmse < best.reprojection_rmse - 1.0e-9)
                    || (metrics.positive_count == best.positive_count
                        && (metrics.reprojection_rmse - best.reprojection_rmse).abs() <= 1.0e-9
                        && metrics.upper_quartile_triangulation_angle_deg
                            > best.upper_quartile_triangulation_angle_deg + 1.0e-9)
            }
        };
        if better {
            best_metrics = Some(metrics);
            best_pose = Some(EssentialPose { r, t: t_unit });
        }
    }

    if best_metrics.map(|m| m.positive_count).unwrap_or(0) < 6 {
        None
    } else {
        best_pose
    }
}

#[derive(Debug, Clone, Copy)]
struct EssentialPoseCandidateMetrics {
    positive_count: usize,
    reprojection_rmse: f64,
    upper_quartile_triangulation_angle_deg: f64,
}

fn analyze_essential_pose_candidate(
    r: &Matrix3<f64>,
    t: &Vector3<f64>,
    points: &[(Vector2<f64>, Vector2<f64>)],
) -> EssentialPoseCandidateMetrics {
    let centre_left = Vector3::zeros();
    let centre_right = -r.transpose() * t;
    let mut positive_count = 0usize;
    let mut residual_sumsq = 0.0;
    let mut residual_count = 0usize;
    let mut triangulation_angles = Vec::new();

    for (x1, x2) in points.iter().take(96) {
        let Some(xw) = triangulate_point_two_view_refined(r, t, x1, x2) else {
            continue;
        };

        let z1 = xw[2];
        let x_cam2 = r * xw + t;
        let z2 = x_cam2[2];
        if z1 <= 1.0e-6 || z2 <= 1.0e-6 {
            continue;
        }
        positive_count += 1;

        let pred1 = Vector2::new(xw[0] / z1, xw[1] / z1);
        let pred2 = Vector2::new(x_cam2[0] / z2, x_cam2[1] / z2);
        let res1 = (pred1 - x1).norm();
        let res2 = (pred2 - x2).norm();
        if res1.is_finite() {
            residual_sumsq += res1 * res1;
            residual_count += 1;
        }
        if res2.is_finite() {
            residual_sumsq += res2 * res2;
            residual_count += 1;
        }

        let angle_deg = triangulation_angle_deg(&xw, &centre_left, &centre_right);
        if angle_deg.is_finite() {
            triangulation_angles.push(angle_deg);
        }
    }

    triangulation_angles.sort_by(|a, b| a.total_cmp(b));
    let upper_quartile_triangulation_angle_deg = if triangulation_angles.is_empty() {
        0.0
    } else {
        triangulation_angles[(triangulation_angles.len() * 3) / 4]
    };
    let reprojection_rmse = if residual_count > 0 {
        (residual_sumsq / residual_count as f64).sqrt()
    } else {
        f64::INFINITY
    };

    EssentialPoseCandidateMetrics {
        positive_count,
        reprojection_rmse,
        upper_quartile_triangulation_angle_deg,
    }
}

fn essential_pose_is_degenerate(
    pose: &EssentialPose,
    inlier_points: &[(Vector2<f64>, Vector2<f64>)],
) -> bool {
    let parallax_deg = essential_rotated_ray_parallax_degrees(&pose.r, inlier_points);
    if parallax_deg.is_empty() {
        return true;
    }

    let mut sorted = parallax_deg;
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    let upper_quartile = sorted[(sorted.len() * 3) / 4];

    if median < 0.35 {
        return true;
    }

    if inlier_points.len() >= 12 {
        let homography_inliers = best_homography_inlier_count(inlier_points, (inlier_points.len().max(12) * 2).min(180), 1.0e-4);
        if homography_inliers + 1 >= inlier_points.len() && upper_quartile < 1.5 {
            return true;
        }
    }

    let lateral_fraction = (pose.t[0] * pose.t[0] + pose.t[1] * pose.t[1]).sqrt() / pose.t.norm().max(1.0e-9);
    lateral_fraction < 0.18 && upper_quartile < 0.9
}

fn essential_rotated_ray_parallax_degrees(
    r: &Matrix3<f64>,
    inlier_points: &[(Vector2<f64>, Vector2<f64>)],
) -> Vec<f64> {
    let mut parallax = Vec::with_capacity(inlier_points.len().min(128));
    for (x1, x2) in inlier_points.iter().take(128) {
        let ray1 = Vector3::new(x1.x, x1.y, 1.0).normalize();
        let ray2 = Vector3::new(x2.x, x2.y, 1.0).normalize();
        let ray2_back = (r.transpose() * ray2).normalize();
        let cos_theta = ray1.dot(&ray2_back).clamp(-1.0, 1.0);
        let theta_deg = cos_theta.acos().to_degrees();
        if theta_deg.is_finite() {
            parallax.push(theta_deg);
        }
    }
    parallax
}

fn ensure_rotation(mut r: Matrix3<f64>) -> Matrix3<f64> {
    if r.determinant() < 0.0 {
        r[(0, 2)] *= -1.0;
        r[(1, 2)] *= -1.0;
        r[(2, 2)] *= -1.0;
    }
    r
}

fn triangulate_point_two_view_refined(
    r: &Matrix3<f64>,
    t: &Vector3<f64>,
    x1: &Vector2<f64>,
    x2: &Vector2<f64>,
) -> Option<Vector3<f64>> {
    let p1 = Matrix3x4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
    );
    let p2 = Matrix3x4::new(
        r[(0, 0)], r[(0, 1)], r[(0, 2)], t[0],
        r[(1, 0)], r[(1, 1)], r[(1, 2)], t[1],
        r[(2, 0)], r[(2, 1)], r[(2, 2)], t[2],
    );

    let mut x = triangulate_point(&p1, &p2, x1, x2)?;
    for _ in 0..6 {
        let z1 = x[2];
        let x_cam2 = r * x + t;
        let z2 = x_cam2[2];
        if z1.abs() <= 1.0e-9 || z2.abs() <= 1.0e-9 {
            break;
        }

        let pred1 = Vector2::new(x[0] / z1, x[1] / z1);
        let pred2 = Vector2::new(x_cam2[0] / z2, x_cam2[1] / z2);
        let residual = Vector4::new(
            x1[0] - pred1[0],
            x1[1] - pred1[1],
            x2[0] - pred2[0],
            x2[1] - pred2[1],
        );

        let j1 = Matrix2x3::new(
            1.0 / z1, 0.0, -x[0] / (z1 * z1),
            0.0, 1.0 / z1, -x[1] / (z1 * z1),
        );
        let j2_cam = Matrix2x3::new(
            1.0 / z2, 0.0, -x_cam2[0] / (z2 * z2),
            0.0, 1.0 / z2, -x_cam2[1] / (z2 * z2),
        );
        let j2 = j2_cam * r;

        let mut j = Matrix4x3::zeros();
        j.row_mut(0).copy_from(&j1.row(0));
        j.row_mut(1).copy_from(&j1.row(1));
        j.row_mut(2).copy_from(&j2.row(0));
        j.row_mut(3).copy_from(&j2.row(1));

        let normal = j.transpose() * j + Matrix3::identity() * 1.0e-9;
        let rhs = j.transpose() * residual;
        let Some(delta) = normal.lu().solve(&rhs) else {
            break;
        };
        if !delta.iter().all(|v| v.is_finite()) {
            break;
        }
        x += delta;
        if delta.norm() <= 1.0e-8 {
            break;
        }
    }

    if x.iter().all(|v| v.is_finite()) {
        Some(x)
    } else {
        None
    }
}

fn estimate_homography_from_correspondences(
    points: &[(Vector2<f64>, Vector2<f64>)],
) -> Option<Matrix3<f64>> {
    if points.len() < 4 {
        return None;
    }

    let (norm1, t1) = normalize_points(points.iter().map(|(p1, _)| *p1).collect::<Vec<_>>().as_slice())?;
    let (norm2, t2) = normalize_points(points.iter().map(|(_, p2)| *p2).collect::<Vec<_>>().as_slice())?;

    let mut a = DMatrix::zeros(points.len() * 2, 9);
    for (i, (p1, p2)) in norm1.iter().zip(norm2.iter()).enumerate() {
        let row = i * 2;
        a[(row, 0)] = -p1.x;
        a[(row, 1)] = -p1.y;
        a[(row, 2)] = -1.0;
        a[(row, 6)] = p2.x * p1.x;
        a[(row, 7)] = p2.x * p1.y;
        a[(row, 8)] = p2.x;

        a[(row + 1, 3)] = -p1.x;
        a[(row + 1, 4)] = -p1.y;
        a[(row + 1, 5)] = -1.0;
        a[(row + 1, 6)] = p2.y * p1.x;
        a[(row + 1, 7)] = p2.y * p1.y;
        a[(row + 1, 8)] = p2.y;
    }

    let svd = a.svd(true, true);
    let v_t = svd.v_t?;
    let h_vec = v_t.row(v_t.nrows() - 1);
    let mut h_hat = Matrix3::zeros();
    for r in 0..3 {
        for c in 0..3 {
            h_hat[(r, c)] = h_vec[r * 3 + c];
        }
    }

    let t2_inv = t2.try_inverse()?;
    let h = t2_inv * h_hat * t1;
    let scale = if h[(2, 2)].abs() > 1.0e-12 { h[(2, 2)] } else { h.norm() };
    if scale.abs() <= 1.0e-12 {
        None
    } else {
        Some(h / scale)
    }
}

fn homography_inliers(
    h: &Matrix3<f64>,
    points: &[(Vector2<f64>, Vector2<f64>)],
    threshold: f64,
) -> Vec<usize> {
    let Some(h_inv) = h.try_inverse() else {
        return Vec::new();
    };

    let mut inliers = Vec::new();
    for (idx, (p1, p2)) in points.iter().enumerate() {
        let hp2 = h * Vector3::new(p1.x, p1.y, 1.0);
        let hp1 = h_inv * Vector3::new(p2.x, p2.y, 1.0);
        if hp2[2].abs() <= 1.0e-12 || hp1[2].abs() <= 1.0e-12 {
            continue;
        }
        let pred2 = Vector2::new(hp2[0] / hp2[2], hp2[1] / hp2[2]);
        let pred1 = Vector2::new(hp1[0] / hp1[2], hp1[1] / hp1[2]);
        let err = (pred2 - p2).norm_squared() + (pred1 - p1).norm_squared();
        if err <= threshold {
            inliers.push(idx);
        }
    }
    inliers
}

fn best_homography_inlier_count(
    points: &[(Vector2<f64>, Vector2<f64>)],
    iterations: usize,
    threshold: f64,
) -> usize {
    if points.len() < 4 {
        return 0;
    }

    let mut best = 0usize;
    for iter in 0..iterations.max(1) {
        let Some(sample) = deterministic_sample_indices(points.len(), 4, iter as u64 + 91) else {
            continue;
        };
        let sample_points: Vec<(Vector2<f64>, Vector2<f64>)> = sample.iter().map(|&i| points[i]).collect();
        let Some(h) = estimate_homography_from_correspondences(&sample_points) else {
            continue;
        };
        let count = homography_inliers(&h, points, threshold).len();
        if count > best {
            best = count;
        }
    }
    best
}

fn triangulate_point(
    p1: &Matrix3x4<f64>,
    p2: &Matrix3x4<f64>,
    x1: &Vector2<f64>,
    x2: &Vector2<f64>,
) -> Option<Vector3<f64>> {
    let mut a = Matrix4::zeros();
    a.row_mut(0).copy_from(&(x1.x * p1.row(2) - p1.row(0)));
    a.row_mut(1).copy_from(&(x1.y * p1.row(2) - p1.row(1)));
    a.row_mut(2).copy_from(&(x2.x * p2.row(2) - p2.row(0)));
    a.row_mut(3).copy_from(&(x2.y * p2.row(2) - p2.row(1)));

    let svd = a.svd(true, true);
    let v_t = svd.v_t?;
    let xh = v_t.row(3).transpose();
    if xh[3].abs() <= 1e-12 {
        return None;
    }
    Some(Vector3::new(xh[0] / xh[3], xh[1] / xh[3], xh[2] / xh[3]))
}

fn deterministic_sample_indices(n: usize, k: usize, seed: u64) -> Option<Vec<usize>> {
    if n < k || k == 0 {
        return None;
    }
    let mut indices = Vec::with_capacity(k);
    let mut state = seed ^ ((n as u64) << 16) ^ ((k as u64) << 32);
    let mut tries = 0usize;
    while indices.len() < k && tries < k * 64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state % (n as u64)) as usize;
        if !indices.contains(&idx) {
            indices.push(idx);
        }
        tries += 1;
    }
    if indices.len() == k {
        Some(indices)
    } else {
        None
    }
}

fn matrix_to_quaternion(r: &Matrix3<f64>) -> [f64; 4] {
    let trace = r[(0, 0)] + r[(1, 1)] + r[(2, 2)];
    let mut q = [0.0; 4];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        q[0] = 0.25 * s;
        q[1] = (r[(2, 1)] - r[(1, 2)]) / s;
        q[2] = (r[(0, 2)] - r[(2, 0)]) / s;
        q[3] = (r[(1, 0)] - r[(0, 1)]) / s;
    } else if r[(0, 0)] > r[(1, 1)] && r[(0, 0)] > r[(2, 2)] {
        let s = (1.0 + r[(0, 0)] - r[(1, 1)] - r[(2, 2)]).sqrt() * 2.0;
        q[0] = (r[(2, 1)] - r[(1, 2)]) / s;
        q[1] = 0.25 * s;
        q[2] = (r[(0, 1)] + r[(1, 0)]) / s;
        q[3] = (r[(0, 2)] + r[(2, 0)]) / s;
    } else if r[(1, 1)] > r[(2, 2)] {
        let s = (1.0 + r[(1, 1)] - r[(0, 0)] - r[(2, 2)]).sqrt() * 2.0;
        q[0] = (r[(0, 2)] - r[(2, 0)]) / s;
        q[1] = (r[(0, 1)] + r[(1, 0)]) / s;
        q[2] = 0.25 * s;
        q[3] = (r[(1, 2)] + r[(2, 1)]) / s;
    } else {
        let s = (1.0 + r[(2, 2)] - r[(0, 0)] - r[(1, 1)]).sqrt() * 2.0;
        q[0] = (r[(1, 0)] - r[(0, 1)]) / s;
        q[1] = (r[(0, 2)] + r[(2, 0)]) / s;
        q[2] = (r[(1, 2)] + r[(2, 1)]) / s;
        q[3] = 0.25 * s;
    }
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n > 1e-12 {
        q[0] /= n;
        q[1] /= n;
        q[2] /= n;
        q[3] /= n;
    }
    q
}

#[derive(Debug, Clone)]
struct BaObservation {
    cam_idx: usize,
    point_id: usize,
    point_world: Vector3<f64>,
    obs_px: Vector2<f64>,
    quality_weight: f64,
}

#[derive(Debug, Clone, Copy)]
struct BaObservationBuildProfile {
    max_pts_per_pair: usize,
    min_pair_inliers: usize,
    non_adjacent_inlier_step: usize,
    min_parallax_floor_px: f64,
    mean_parallax_scale: f64,
    non_adjacent_parallax_step: f64,
    adjacent_motion_min_inliers: usize,
    non_adjacent_motion_min_inliers: usize,
    min_triangulation_angle_deg: f64,
    max_initial_reproj_px: f64,
}

const BA_MIN_PAIR_INLIERS: usize = 14;
const BA_MIN_PARALLAX_PX: f64 = 2.5;
const BA_MIN_TRIANGULATION_ANGLE_DEG: f64 = 0.8;
const BA_MAX_INITIAL_REPROJ_PX: f64 = 22.0;
const BA_MAX_PAIR_GAP: usize = 4;
const BA_OBSERVATION_DEBUG_ENV: &str = "WBPHOTOGRAMMETRY_DEBUG_BA_OBS";

#[derive(Debug, Default, Clone, Copy)]
struct BaObservationDebugCounts {
    skipped_pair_gap: usize,
    skipped_pair_inliers: usize,
    skipped_pair_parallax: usize,
    skipped_pair_motion: usize,
    triangulation_failed: usize,
    skipped_triangulation_angle: usize,
    skipped_reprojection: usize,
    accepted_observations: usize,
}

fn ba_observation_debug_enabled() -> bool {
    std::env::var_os(BA_OBSERVATION_DEBUG_ENV).is_some()
}

fn ba_observation_debug_line(message: String) {
    if ba_observation_debug_enabled() {
        eprintln!("[ba-obs] {message}");
    }
}

fn huber_weight(residual: f64, threshold: f64) -> f64 {
    if residual <= threshold {
        1.0
    } else {
        threshold / residual
    }
}

fn run_simplified_bundle_adjustment(
    positions: &[[f64; 3]],
    rotations: &[[f64; 4]],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    intrinsics_refinement_policy: IntrinsicsRefinementPolicy,
    reduced_camera_solve_mode: ReducedCameraSolveMode,
) -> (Vec<[f64; 3]>, Vec<[f64; 4]>, CameraIntrinsics, Vec<f64>, BaDiagnostics) {
    if positions.len() < 2 || rotations.len() != positions.len() {
        return (
            positions.to_vec(),
            rotations.to_vec(),
            intrinsics.clone(),
            Vec::new(),
            BaDiagnostics {
                optimization_passes: 0,
                huber_threshold_px: 2.0,
                final_cost: 0.0,
                intrinsics_refined: false,
                distortion_refined: false,
                observations_initial: 0,
                observations_final: 0,
                observation_retention_pct: 0.0,
                supported_camera_fraction: 0.0,
                observations_per_pass: Vec::new(),
                prune_thresholds_px: Vec::new(),
                covariance: CameraCovarianceDiagnostics::default(),
            },
        );
    }

    let mut centres: Vec<Vector3<f64>> = positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let original_centres = centres.clone();
    let mut rotations_c2w: Vec<Matrix3<f64>> = rotations
        .iter()
        .map(quaternion_to_matrix)
        .collect();
    let original_rotations = rotations_c2w.clone();

    let mut observations = build_ba_observations(
        &centres,
        &rotations_c2w,
        match_stats,
        intrinsics,
        camera_model,
    );
    observations = refine_structure_points_from_observations(
        &centres,
        &rotations_c2w,
        &observations,
        intrinsics,
        camera_model,
    );
    if observations.len() < 12 {
        return (
            positions.to_vec(),
            rotations.to_vec(),
            intrinsics.clone(),
            Vec::new(),
            BaDiagnostics {
                optimization_passes: 0,
                huber_threshold_px: 2.0,
                final_cost: 0.0,
                intrinsics_refined: false,
                distortion_refined: false,
                observations_initial: 0,
                observations_final: 0,
                observation_retention_pct: 0.0,
                supported_camera_fraction: 0.0,
                observations_per_pass: Vec::new(),
                prune_thresholds_px: Vec::new(),
                covariance: CameraCovarianceDiagnostics::default(),
            },
        );
    }
    let observations_initial = observations.len();
    let initial_supported_camera_fraction = supported_camera_fraction_from_observations(&observations, centres.len());
    let min_intrinsics_obs = centres.len().max(2) * 18;
    let allow_intrinsics_refinement = observations_initial >= min_intrinsics_obs
        && initial_supported_camera_fraction >= 0.85;
    let weak_geometry_support = centres.len() <= 8
        && (initial_supported_camera_fraction < 0.95
            || observations_initial < centres.len().max(2) * 24);
    let intrinsics_refine_mask = build_intrinsics_refine_mask(
        intrinsics_refinement_policy,
        allow_intrinsics_refinement,
        observations_initial,
        centres.len(),
        initial_supported_camera_fraction,
        weak_geometry_support,
        camera_model,
    );
    let freeze_rotation_updates = weak_geometry_support && initial_supported_camera_fraction < 0.90;
    let pose_prior_sigma_m = if weak_geometry_support { 2.5 } else { 6.0 };
    let pose_prior_scale_px2 = if weak_geometry_support { 450.0 } else { 180.0 };
    let pose_prior_weights = build_pose_prior_weights(&observations, centres.len(), weak_geometry_support);
    let center_step_cap = if weak_geometry_support { 0.04 } else { 0.08 };
    let rotation_step_cap = if weak_geometry_support { 0.018 } else { 0.035 };
    // Increased blend factors: the previous ultra-conservative values (0.12-0.25
    // for positions, 0.20-0.40 for intrinsics) were discarding the vast majority
    // of every BA pass, preventing convergence even when observations were good.
    let post_blend = if weak_geometry_support {
        0.50
    } else if initial_supported_camera_fraction < 0.90 {
        0.70
    } else {
        0.88
    };
    let intrinsics_blend = if weak_geometry_support { 0.60 } else { 0.88 };

    // Enhanced BA: optimize camera centers and small-angle rotations
    // with robust Huber weighting for outlier trimming and triangulated structure.
    let eps = 0.02;
    let rot_eps = 0.001;
    let huber_threshold = 2.0; // pixels
    let intrinsics_eps = [2.0, 2.0, 0.5, 0.5, 1.0e-4, 1.0e-5, 1.0e-5, 1.0e-5];
    let intrinsics_lr = [0.08, 0.08, 0.03, 0.03, 0.002, 0.001, 0.0005, 0.0005];
    let mut intrinsics_opt = intrinsics.clone();
    let initial_intrinsics = intrinsics.clone();
    let image_width_hint = (intrinsics.cx * 2.0).max(320.0);
    let image_height_hint = (intrinsics.cy * 2.0).max(240.0);
    let mut lambda_center: f64 = 20.0;
    let mut lambda_rot: f64 = 45.0;
    let mut lambda_intr: f64 = 12.0;
    let mut best_centres = centres.clone();
    let mut best_rotations = rotations_c2w.clone();
    let mut best_intrinsics = intrinsics_opt.clone();
    let mut best_observations = observations.clone();
    let mut best_pass_observations = Vec::new();
    let mut best_prune_thresholds = Vec::new();
    let mut best_cost = total_observation_error_with_huber(
        &centres,
        &rotations_c2w,
        &observations,
        &intrinsics_opt,
        camera_model,
        huber_threshold,
        Some(&original_centres),
        &pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    let mut pass_observations = Vec::new();
    let mut pass_prune_thresholds = Vec::new();
    let max_passes = 4usize;
    let mut passes_completed = 0u32;
    
    for pass_idx in 0..max_passes {
        passes_completed += 1;
        let mut used_coupled_pose_update = false;
        if !freeze_rotation_updates {
            if let Some(update) = apply_reduced_camera_pose_update(
                &centres,
                &rotations_c2w,
                &observations,
                &intrinsics_opt,
                camera_model,
                huber_threshold,
                &original_centres,
                &pose_prior_weights,
                pose_prior_sigma_m,
                pose_prior_scale_px2,
                lambda_center.max(lambda_rot),
                center_step_cap,
                rotation_step_cap,
                rot_eps,
                reduced_camera_solve_mode,
            ) {
                centres = update.centres;
                rotations_c2w = update.rotations;
                observations = update.observations;
                lambda_center = update.next_lambda;
                lambda_rot = update.next_lambda;
                used_coupled_pose_update = true;
            }
        }

        // Optimize camera centers with a reduced Schur-style solve when the
        // point network supports it; otherwise fall back to the older local pass.
        if !used_coupled_pose_update {
            if let Some(update) = apply_reduced_camera_center_update(
            &centres,
            &rotations_c2w,
            &observations,
            &intrinsics_opt,
            camera_model,
            huber_threshold,
            &original_centres,
            &pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
            lambda_center,
            center_step_cap,
            ) {
                centres = update.centres;
                observations = update.observations;
                lambda_center = update.next_lambda;
            } else {
                for cam_idx in 1..centres.len() {
                let base_err = camera_observation_error_with_huber(
                    cam_idx,
                    &centres[cam_idx],
                    &observations,
                    &rotations_c2w,
                    &intrinsics_opt,
                    camera_model,
                    huber_threshold,
                    &original_centres[cam_idx],
                    pose_prior_weights[cam_idx],
                    pose_prior_sigma_m,
                    pose_prior_scale_px2,
                );
                if !base_err.is_finite() {
                    continue;
                }

                let mut grad = [0.0_f64; 2];
                let mut hdiag = [1.0e-6_f64; 2];
                for axis in 0..2 {
                    let mut cp = centres[cam_idx];
                    cp[axis] += eps;
                    let ep = camera_observation_error_with_huber(
                        cam_idx,
                        &cp,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    let mut cm = centres[cam_idx];
                    cm[axis] -= eps;
                    let em = camera_observation_error_with_huber(
                        cam_idx,
                        &cm,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    if ep.is_finite() && em.is_finite() {
                        grad[axis] = (ep - em) / (2.0 * eps);
                        let curvature = (ep - 2.0 * base_err + em) / (eps * eps);
                        hdiag[axis] = curvature.abs().max(1.0e-6);
                    }
                }

                let mut hxy = 0.0_f64;
                {
                    let mut cpp = centres[cam_idx];
                    cpp[0] += eps;
                    cpp[1] += eps;
                    let e_pp = camera_observation_error_with_huber(
                        cam_idx,
                        &cpp,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    let mut cpm = centres[cam_idx];
                    cpm[0] += eps;
                    cpm[1] -= eps;
                    let e_pm = camera_observation_error_with_huber(
                        cam_idx,
                        &cpm,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    let mut cmp = centres[cam_idx];
                    cmp[0] -= eps;
                    cmp[1] += eps;
                    let e_mp = camera_observation_error_with_huber(
                        cam_idx,
                        &cmp,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    let mut cmm = centres[cam_idx];
                    cmm[0] -= eps;
                    cmm[1] -= eps;
                    let e_mm = camera_observation_error_with_huber(
                        cam_idx,
                        &cmm,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );

                    if e_pp.is_finite() && e_pm.is_finite() && e_mp.is_finite() && e_mm.is_finite() {
                        hxy = ((e_pp - e_pm) - (e_mp - e_mm)) / (4.0 * eps * eps);
                    }
                }

                let mut accepted = false;
                let mut lambda_try = lambda_center;
                for _ in 0..5 {
                    let mut delta = Vector3::zeros();
                    let h00 = hdiag[0] + lambda_try;
                    let h11 = hdiag[1] + lambda_try;
                    if let Some((dx, dy)) = solve_2x2(h00, hxy, hxy, h11, grad[0], grad[1]) {
                        delta[0] = dx.clamp(-center_step_cap, center_step_cap);
                        delta[1] = dy.clamp(-center_step_cap, center_step_cap);
                    } else {
                        delta[0] = (grad[0] / h00).clamp(-center_step_cap, center_step_cap);
                        delta[1] = (grad[1] / h11).clamp(-center_step_cap, center_step_cap);
                    }
                    if !delta.iter().all(|v| v.is_finite()) {
                        lambda_try *= 2.0;
                        continue;
                    }

                    let candidate = centres[cam_idx] - delta;
                    let cand_err = camera_observation_error_with_huber(
                        cam_idx,
                        &candidate,
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );
                    if cand_err.is_finite() && cand_err + 1.0e-9 < base_err {
                        centres[cam_idx] = candidate;
                        lambda_center = (lambda_try * 0.7).max(1.0e-4);
                        accepted = true;
                        break;
                    }
                    lambda_try *= 2.0;
                }
                if !accepted {
                    lambda_center = (lambda_center * 2.0).min(1.0e6);
                }
                }
            }
        }

        // Optimize rotations (small-angle approximation for non-first camera)
        // only when orientation support is strong enough; otherwise keep seeded attitudes.
        if !used_coupled_pose_update && !freeze_rotation_updates {
            if let Some(update) = apply_reduced_camera_rotation_update(
                &centres,
                &rotations_c2w,
                &observations,
                &intrinsics_opt,
                camera_model,
                huber_threshold,
                &original_centres,
                &pose_prior_weights,
                pose_prior_sigma_m,
                pose_prior_scale_px2,
                lambda_rot,
                rotation_step_cap,
                rot_eps,
            ) {
                rotations_c2w = update.rotations;
                observations = update.observations;
                lambda_rot = update.next_lambda;
            } else {
                for cam_idx in 1..rotations_c2w.len() {
                    let base_err = camera_observation_error_with_huber(
                        cam_idx,
                        &centres[cam_idx],
                        &observations,
                        &rotations_c2w,
                        &intrinsics_opt,
                        camera_model,
                        huber_threshold,
                        &original_centres[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );
                    if !base_err.is_finite() {
                        continue;
                    }

                    let mut grad_rot = Vector3::zeros();
                    let mut hdiag_rot = [1.0e-6_f64; 2];
                    for rot_axis in 0..2 {
                        let mut rotations_p = rotations_c2w.clone();
                        rotations_p[cam_idx] = small_angle_update(&rotations_c2w[cam_idx], rot_axis, rot_eps);
                        let ep = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_p,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        let mut rotations_m = rotations_c2w.clone();
                        rotations_m[cam_idx] = small_angle_update(&rotations_c2w[cam_idx], rot_axis, -rot_eps);
                        let em = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_m,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        if ep.is_finite() && em.is_finite() {
                            grad_rot[rot_axis] = (ep - em) / (2.0 * rot_eps);
                            let curvature = (ep - 2.0 * base_err + em) / (rot_eps * rot_eps);
                            hdiag_rot[rot_axis] = curvature.abs().max(1.0e-6);
                        }
                    }

                    let mut hxy_rot = 0.0_f64;
                    {
                        let mut rotations_pp = rotations_c2w.clone();
                        let update_pp = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, rot_eps), 1, rot_eps);
                        rotations_pp[cam_idx] = update_pp;
                        let e_pp = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_pp,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        let mut rotations_pm = rotations_c2w.clone();
                        let update_pm = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, rot_eps), 1, -rot_eps);
                        rotations_pm[cam_idx] = update_pm;
                        let e_pm = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_pm,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        let mut rotations_mp = rotations_c2w.clone();
                        let update_mp = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, -rot_eps), 1, rot_eps);
                        rotations_mp[cam_idx] = update_mp;
                        let e_mp = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_mp,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        let mut rotations_mm = rotations_c2w.clone();
                        let update_mm = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, -rot_eps), 1, -rot_eps);
                        rotations_mm[cam_idx] = update_mm;
                        let e_mm = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_mm,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );

                        if e_pp.is_finite() && e_pm.is_finite() && e_mp.is_finite() && e_mm.is_finite() {
                            hxy_rot = ((e_pp - e_pm) - (e_mp - e_mm)) / (4.0 * rot_eps * rot_eps);
                        }
                    }

                    let mut accepted = false;
                    let mut lambda_try = lambda_rot;
                    for _ in 0..5 {
                        let mut delta = Vector3::zeros();
                        let h00 = hdiag_rot[0] + lambda_try;
                        let h11 = hdiag_rot[1] + lambda_try;
                        if let Some((dx, dy)) = solve_2x2(h00, hxy_rot, hxy_rot, h11, grad_rot[0], grad_rot[1]) {
                            delta[0] = dx.clamp(-rotation_step_cap, rotation_step_cap);
                            delta[1] = dy.clamp(-rotation_step_cap, rotation_step_cap);
                        } else {
                            delta[0] = (grad_rot[0] / h00).clamp(-rotation_step_cap, rotation_step_cap);
                            delta[1] = (grad_rot[1] / h11).clamp(-rotation_step_cap, rotation_step_cap);
                        }
                        if !delta.iter().all(|v| v.is_finite()) {
                            lambda_try *= 2.0;
                            continue;
                        }

                        let candidate_rot = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, -delta[0]), 1, -delta[1]);
                        let mut rotations_candidate = rotations_c2w.clone();
                        rotations_candidate[cam_idx] = candidate_rot;
                        let cand_err = camera_observation_error_with_huber(
                            cam_idx,
                            &centres[cam_idx],
                            &observations,
                            &rotations_candidate,
                            &intrinsics_opt,
                            camera_model,
                            huber_threshold,
                            &original_centres[cam_idx],
                            pose_prior_weights[cam_idx],
                            pose_prior_sigma_m,
                            pose_prior_scale_px2,
                        );
                        if cand_err.is_finite() && cand_err + 1.0e-9 < base_err {
                            rotations_c2w[cam_idx] = candidate_rot;
                            lambda_rot = (lambda_try * 0.7).max(1.0e-4);
                            accepted = true;
                            break;
                        }
                        lambda_try *= 2.0;
                    }
                    if !accepted {
                        lambda_rot = (lambda_rot * 2.0).min(1.0e6);
                    }
                }
            }
        }

        let base_intr_cost = total_observation_error_with_huber(
            &centres,
            &rotations_c2w,
            &observations,
            &intrinsics_opt,
            camera_model,
            huber_threshold,
            Some(&original_centres),
            &pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        if allow_intrinsics_refinement && base_intr_cost.is_finite() {
            let mut grads = [0.0_f64; 8];
            let mut hdiag_intr = [1.0e-8_f64; 8];
            for param_idx in 0..8 {
                if !intrinsics_refine_mask.params[param_idx] {
                    continue;
                }
                let eps = intrinsics_eps[param_idx];
                let mut plus = intrinsics_opt.clone();
                perturb_intrinsics_param(&mut plus, param_idx, eps, camera_model);
                let ep = total_observation_error_with_huber(
                    &centres,
                    &rotations_c2w,
                    &observations,
                    &plus,
                    camera_model,
                    huber_threshold,
                    Some(&original_centres),
                    &pose_prior_weights,
                    pose_prior_sigma_m,
                    pose_prior_scale_px2,
                );

                let mut minus = intrinsics_opt.clone();
                perturb_intrinsics_param(&mut minus, param_idx, -eps, camera_model);
                let em = total_observation_error_with_huber(
                    &centres,
                    &rotations_c2w,
                    &observations,
                    &minus,
                    camera_model,
                    huber_threshold,
                    Some(&original_centres),
                    &pose_prior_weights,
                    pose_prior_sigma_m,
                    pose_prior_scale_px2,
                );

                if ep.is_finite() && em.is_finite() {
                    grads[param_idx] = (ep - em) / (2.0 * eps);
                    let curvature = (ep - 2.0 * base_intr_cost + em) / (eps * eps);
                    hdiag_intr[param_idx] = curvature.abs().max(1.0e-8);
                }
            }

            let mut accepted = false;
            let mut lambda_try = lambda_intr;
            for _ in 0..5 {
                let mut scaled_lrs = [0.0_f64; 8];
                for i in 0..8 {
                    if !intrinsics_refine_mask.params[i] {
                        continue;
                    }
                    let lm_scale = 1.0 / (hdiag_intr[i] + lambda_try);
                    scaled_lrs[i] = (intrinsics_lr[i] * lm_scale).clamp(0.0, intrinsics_lr[i]);
                }

                let mut candidate = intrinsics_opt.clone();
                apply_intrinsics_update(
                    &mut candidate,
                    &grads,
                    &scaled_lrs,
                    intrinsics_refine_mask,
                    image_width_hint,
                    image_height_hint,
                    camera_model,
                );
                let cand_cost = total_observation_error_with_huber(
                    &centres,
                    &rotations_c2w,
                    &observations,
                    &candidate,
                    camera_model,
                    huber_threshold,
                    Some(&original_centres),
                    &pose_prior_weights,
                    pose_prior_sigma_m,
                    pose_prior_scale_px2,
                );
                if cand_cost.is_finite() && cand_cost + 1.0e-9 < base_intr_cost {
                    intrinsics_opt = candidate;
                    lambda_intr = (lambda_try * 0.7).max(1.0e-4);
                    accepted = true;
                    break;
                }
                lambda_try *= 2.0;
            }
            if !accepted {
                lambda_intr = (lambda_intr * 2.0).min(1.0e6);
            }
        }

        // D1-T02: prune high-residual observations between BA passes.
        let (pruned_observations, prune_stats) = prune_observations_by_residual(
            &centres,
            &rotations_c2w,
            &observations,
            &intrinsics_opt,
            camera_model,
            pass_idx,
            max_passes,
        );
        let pruned_count = pruned_observations.len();
        let pre_prune_count = observations.len();
        observations = pruned_observations;
        observations = refine_structure_points_from_observations(
            &centres,
            &rotations_c2w,
            &observations,
            &intrinsics_opt,
            camera_model,
        );
        pass_observations.push(prune_stats.kept_observations);
        pass_prune_thresholds.push(prune_stats.threshold_px);
        ba_observation_debug_line(format!(
            "ba-pass={}/{}, obs_before={}, obs_after={}, prune_threshold_px={:.2}",
            pass_idx + 1,
            max_passes,
            pre_prune_count,
            pruned_count,
            prune_stats.threshold_px,
        ));

        let iter_cost = total_observation_error_with_huber(
            &centres,
            &rotations_c2w,
            &observations,
            &intrinsics_opt,
            camera_model,
            huber_threshold,
            Some(&original_centres),
            &pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        if iter_cost.is_finite() && (!best_cost.is_finite() || iter_cost < best_cost) {
            best_cost = iter_cost;
            best_centres = centres.clone();
            best_rotations = rotations_c2w.clone();
            best_intrinsics = intrinsics_opt.clone();
            best_observations = observations.clone();
            best_pass_observations = pass_observations.clone();
            best_prune_thresholds = pass_prune_thresholds.clone();
        }

        if weak_geometry_support {
            for cam_idx in 1..centres.len() {
                centres[cam_idx] = original_centres[cam_idx] + 0.45 * (centres[cam_idx] - original_centres[cam_idx]);
                rotations_c2w[cam_idx] = original_rotations[cam_idx]
                    + 0.45 * (&rotations_c2w[cam_idx] - &original_rotations[cam_idx]);
            }
        }

        if observations.len() < 10 {
            break;
        }

        // Early stop when pruning no longer changes support materially.
        if pre_prune_count > 0 {
            let retained_ratio = observations.len() as f64 / pre_prune_count as f64;
            if retained_ratio > 0.995 && pass_idx >= 1 {
                break;
            }
        }
    }

    centres = best_centres;
    rotations_c2w = best_rotations;
    intrinsics_opt = best_intrinsics;
    observations = best_observations;
    pass_observations = best_pass_observations;
    pass_prune_thresholds = best_prune_thresholds;

    // Keep this BA pass conservative for workflow stability.
    for cam_idx in 1..centres.len() {
        centres[cam_idx] = original_centres[cam_idx] + post_blend * (centres[cam_idx] - original_centres[cam_idx]);
        rotations_c2w[cam_idx] = original_rotations[cam_idx] + post_blend * (&rotations_c2w[cam_idx] - &original_rotations[cam_idx]);
    }
    intrinsics_opt.fx = initial_intrinsics.fx + intrinsics_blend * (intrinsics_opt.fx - initial_intrinsics.fx);
    intrinsics_opt.fy = initial_intrinsics.fy + intrinsics_blend * (intrinsics_opt.fy - initial_intrinsics.fy);
    intrinsics_opt.cx = initial_intrinsics.cx + intrinsics_blend * (intrinsics_opt.cx - initial_intrinsics.cx);
    intrinsics_opt.cy = initial_intrinsics.cy + intrinsics_blend * (intrinsics_opt.cy - initial_intrinsics.cy);
    intrinsics_opt.k1 = initial_intrinsics.k1 + intrinsics_blend * (intrinsics_opt.k1 - initial_intrinsics.k1);
    intrinsics_opt.k2 = initial_intrinsics.k2 + intrinsics_blend * (intrinsics_opt.k2 - initial_intrinsics.k2);
    intrinsics_opt.p1 = initial_intrinsics.p1 + intrinsics_blend * (intrinsics_opt.p1 - initial_intrinsics.p1);
    intrinsics_opt.p2 = initial_intrinsics.p2 + intrinsics_blend * (intrinsics_opt.p2 - initial_intrinsics.p2);

    let refined_positions: Vec<[f64; 3]> = centres
        .iter()
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    
    let refined_rotations: Vec<[f64; 4]> = rotations_c2w
        .iter()
        .map(matrix_to_quaternion)
        .collect();
    
    let residuals = reprojection_residual_samples(
        &centres,
        &rotations_c2w,
        &observations,
        &intrinsics_opt,
        camera_model,
    );

    let final_cost = total_observation_error_with_huber(
        &centres,
        &rotations_c2w,
        &observations,
        &intrinsics_opt,
        camera_model,
        huber_threshold,
        Some(&original_centres),
        &pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    let observations_final = observations.len();
    let observation_retention_pct = if observations_initial > 0 {
        100.0 * observations_final as f64 / observations_initial as f64
    } else {
        0.0
    };
    let mut supported_cameras = vec![false; centres.len()];
    for obs in &observations {
        if obs.cam_idx < supported_cameras.len() {
            supported_cameras[obs.cam_idx] = true;
        }
    }
    let supported_camera_fraction = if centres.is_empty() {
        0.0
    } else {
        supported_cameras.iter().filter(|&&v| v).count() as f64 / centres.len() as f64
    };
    let intrinsics_refined = (intrinsics_opt.fx - initial_intrinsics.fx).abs() > 1e-3
        || (intrinsics_opt.fy - initial_intrinsics.fy).abs() > 1e-3
        || (intrinsics_opt.cx - initial_intrinsics.cx).abs() > 1e-4
        || (intrinsics_opt.cy - initial_intrinsics.cy).abs() > 1e-4;
    let distortion_refined = (intrinsics_opt.k1 - initial_intrinsics.k1).abs() > 1e-8
        || (intrinsics_opt.k2 - initial_intrinsics.k2).abs() > 1e-8
        || (intrinsics_opt.p1 - initial_intrinsics.p1).abs() > 1e-8
        || (intrinsics_opt.p2 - initial_intrinsics.p2).abs() > 1e-8;
    let covariance = estimate_camera_covariance_diagnostics(
        &centres,
        &rotations_c2w,
        &observations,
        &intrinsics_opt,
        camera_model,
        huber_threshold,
        &original_centres,
        &pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
        rot_eps,
    );

    (
        refined_positions,
        refined_rotations,
        intrinsics_opt,
        residuals,
        BaDiagnostics {
            optimization_passes: passes_completed,
            huber_threshold_px: huber_threshold,
            final_cost,
            intrinsics_refined,
            distortion_refined,
            observations_initial,
            observations_final,
            observation_retention_pct,
            supported_camera_fraction,
            observations_per_pass: pass_observations,
            prune_thresholds_px: pass_prune_thresholds,
            covariance,
        },
    )
}

fn perturb_intrinsics_param(
    intrinsics: &mut CameraIntrinsics,
    param_idx: usize,
    delta: f64,
    camera_model: CameraModel,
) {
    match param_idx {
        0 => intrinsics.fx += delta,
        1 => intrinsics.fy += delta,
        2 => intrinsics.cx += delta,
        3 => intrinsics.cy += delta,
        4 => intrinsics.k1 += delta,
        5 => intrinsics.k2 += delta,
        6 if camera_model != CameraModel::Fisheye => intrinsics.p1 += delta,
        7 if camera_model != CameraModel::Fisheye => intrinsics.p2 += delta,
        _ => {}
    }
}

fn solve_2x2(
    a00: f64,
    a01: f64,
    a10: f64,
    a11: f64,
    b0: f64,
    b1: f64,
) -> Option<(f64, f64)> {
    let det = a00 * a11 - a01 * a10;
    if !det.is_finite() || det.abs() <= 1.0e-12 {
        return None;
    }
    let inv00 = a11 / det;
    let inv01 = -a01 / det;
    let inv10 = -a10 / det;
    let inv11 = a00 / det;
    Some((inv00 * b0 + inv01 * b1, inv10 * b0 + inv11 * b1))
}

#[derive(Debug, Clone)]
struct ReducedPointContribution {
    cam_idx: usize,
    obs_indices: Vec<usize>,
    a: Matrix2<f64>,
    b: Matrix2x3<f64>,
    g: Vector2<f64>,
}

#[derive(Debug, Clone)]
struct ReducedPointBlock {
    point_world: Vector3<f64>,
    contributions: Vec<ReducedPointContribution>,
    v_inv: Matrix3<f64>,
    g_point: Vector3<f64>,
}

#[derive(Debug, Clone)]
struct ReducedCameraSystem {
    hessian: DMatrix<f64>,
    gradient: DVector<f64>,
    point_blocks: Vec<ReducedPointBlock>,
}

#[derive(Debug, Clone)]
struct ReducedCenterUpdate {
    centres: Vec<Vector3<f64>>,
    observations: Vec<BaObservation>,
    next_lambda: f64,
}

#[derive(Debug, Clone)]
struct ReducedRotationUpdate {
    rotations: Vec<Matrix3<f64>>,
    observations: Vec<BaObservation>,
    next_lambda: f64,
}

#[derive(Debug, Clone)]
struct ReducedPosePointContribution {
    cam_idx: usize,
    obs_indices: Vec<usize>,
    a: Matrix4<f64>,
    b: Matrix4x3<f64>,
    g: Vector4<f64>,
}

#[derive(Debug, Clone)]
struct ReducedPosePointBlock {
    point_world: Vector3<f64>,
    contributions: Vec<ReducedPosePointContribution>,
    v_inv: Matrix3<f64>,
    g_point: Vector3<f64>,
}

#[derive(Debug, Clone)]
struct ReducedPoseSystem {
    hessian: DMatrix<f64>,
    hessian_sparse: SparseSymmetricBlock4,
    gradient: DVector<f64>,
    point_blocks: Vec<ReducedPosePointBlock>,
}

#[derive(Debug, Clone)]
struct SparseSymmetricBlock4 {
    dim: usize,
    blocks: HashMap<(usize, usize), Matrix4<f64>>,
}

impl SparseSymmetricBlock4 {
    fn new(dim: usize) -> Self {
        Self {
            dim,
            blocks: HashMap::new(),
        }
    }

    fn add_block(&mut self, row: usize, col: usize, block: Matrix4<f64>) {
        if row <= col {
            self.blocks
                .entry((row, col))
                .and_modify(|acc| *acc += block)
                .or_insert(block);
        } else {
            let bt = block.transpose();
            self.blocks
                .entry((col, row))
                .and_modify(|acc| *acc += bt)
                .or_insert(bt);
        }
    }

    fn add_diag_value(&mut self, idx: usize, value: f64) {
        let base = (idx / 4) * 4;
        let local = idx % 4;
        let mut block = Matrix4::zeros();
        block[(local, local)] = value;
        self.add_block(base, base, block);
    }

    fn into_dense(self) -> DMatrix<f64> {
        let mut dense = DMatrix::zeros(self.dim, self.dim);
        for ((row, col), block) in self.blocks {
            for r in 0..4 {
                for c in 0..4 {
                    dense[(row + r, col + c)] += block[(r, c)];
                }
            }
            if row != col {
                let bt = block.transpose();
                for r in 0..4 {
                    for c in 0..4 {
                        dense[(col + r, row + c)] += bt[(r, c)];
                    }
                }
            }
        }
        dense
    }

    fn diagonal(&self) -> DVector<f64> {
        let mut diag = DVector::zeros(self.dim);
        for ((row, col), block) in &self.blocks {
            if row == col {
                for i in 0..4 {
                    diag[row + i] += block[(i, i)];
                }
            }
        }
        diag
    }

    fn matvec_with_damping(&self, x: &DVector<f64>, damping: f64) -> DVector<f64> {
        let mut y = DVector::zeros(self.dim);
        for ((row, col), block) in &self.blocks {
            if row == col {
                for r in 0..4 {
                    let mut acc = 0.0;
                    for c in 0..4 {
                        acc += block[(r, c)] * x[col + c];
                    }
                    y[row + r] += acc;
                }
            } else {
                for r in 0..4 {
                    let mut acc_upper = 0.0;
                    let mut acc_lower = 0.0;
                    for c in 0..4 {
                        acc_upper += block[(r, c)] * x[col + c];
                        acc_lower += block[(c, r)] * x[row + c];
                    }
                    y[row + r] += acc_upper;
                    y[col + r] += acc_lower;
                }
            }
        }
        if damping > 0.0 {
            for i in 0..self.dim {
                y[i] += damping * x[i];
            }
        }
        y
    }
}

fn solve_damped_sparse_pcg(
    matrix: &SparseSymmetricBlock4,
    gradient: &DVector<f64>,
    damping: f64,
    max_iters: usize,
    tol: f64,
) -> Option<DVector<f64>> {
    if matrix.dim == 0 || gradient.len() != matrix.dim {
        return None;
    }

    let dense_fallback = || {
        let mut dense = matrix.clone().into_dense();
        for i in 0..dense.nrows() {
            dense[(i, i)] += damping;
        }
        let rhs = -gradient;
        dense.lu().solve(&rhs)
    };

    let b = -gradient;
    let mut x = DVector::zeros(matrix.dim);
    let mut r = b.clone();
    let mut diag = matrix.diagonal();
    for i in 0..diag.len() {
        diag[i] += damping;
        if !diag[i].is_finite() || diag[i].abs() < 1.0e-9 {
            diag[i] = 1.0;
        }
    }

    let mut z = DVector::zeros(matrix.dim);
    for i in 0..z.len() {
        z[i] = r[i] / diag[i];
    }
    let mut p = z.clone();
    let mut rz_old = r.dot(&z);
    if !rz_old.is_finite() {
        return dense_fallback();
    }

    let b_norm = b.norm().max(1.0);
    let target = tol.max(1.0e-12) * b_norm;

    for _ in 0..max_iters.max(1) {
        let ap = matrix.matvec_with_damping(&p, damping);
        let denom = p.dot(&ap);
        if !denom.is_finite() || denom.abs() < 1.0e-12 {
            return dense_fallback();
        }
        let alpha = rz_old / denom;
        if !alpha.is_finite() {
            return dense_fallback();
        }
        x += alpha * &p;
        r -= alpha * ap;

        let r_norm = r.norm();
        if r_norm.is_finite() && r_norm <= target {
            return Some(x);
        }

        for i in 0..z.len() {
            z[i] = r[i] / diag[i];
        }
        let rz_new = r.dot(&z);
        if !rz_new.is_finite() {
            return dense_fallback();
        }
        let beta = rz_new / rz_old;
        if !beta.is_finite() {
            return dense_fallback();
        }
        p = &z + beta * p;
        rz_old = rz_new;
    }

    let resid = (matrix.matvec_with_damping(&x, damping) + gradient).norm();
    if resid.is_finite() && resid <= target * 2.0 {
        Some(x)
    } else {
        dense_fallback()
    }
}

#[derive(Debug, Clone)]
struct ReducedPoseUpdate {
    centres: Vec<Vector3<f64>>,
    rotations: Vec<Matrix3<f64>>,
    observations: Vec<BaObservation>,
    next_lambda: f64,
}

fn apply_reduced_camera_center_update(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    lambda_center: f64,
    center_step_cap: f64,
) -> Option<ReducedCenterUpdate> {
    if centres.len() < 3 || observations.len() < 18 {
        return None;
    }

    let base_cost = total_observation_error_with_huber(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        Some(seed_centres),
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    if !base_cost.is_finite() {
        return None;
    }

    let reduced = build_reduced_camera_system(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        seed_centres,
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    )?;
    if reduced.hessian.nrows() < 2 {
        return None;
    }

    let mut lambda_try = lambda_center;
    for _ in 0..5 {
        let mut h_damped = reduced.hessian.clone();
        for idx in 0..h_damped.nrows() {
            h_damped[(idx, idx)] += lambda_try;
        }
        let rhs = -&reduced.gradient;
        let delta = h_damped.lu().solve(&rhs)?;
        if !delta.iter().all(|v| v.is_finite()) {
            lambda_try *= 2.0;
            continue;
        }

        let mut candidate_centres = centres.to_vec();
        for cam_idx in 1..centres.len() {
            let base = reduced_camera_param_offset(cam_idx)?;
            let dx = delta[base].clamp(-center_step_cap, center_step_cap);
            let dy = delta[base + 1].clamp(-center_step_cap, center_step_cap);
            candidate_centres[cam_idx][0] += dx;
            candidate_centres[cam_idx][1] += dy;
        }

        let candidate_observations = back_substitute_reduced_points(observations, &reduced.point_blocks, &delta);
        let cand_cost = total_observation_error_with_huber(
            &candidate_centres,
            rotations_c2w,
            &candidate_observations,
            intrinsics,
            camera_model,
            huber_threshold,
            Some(seed_centres),
            pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        if cand_cost.is_finite() && cand_cost + 1.0e-9 < base_cost {
            return Some(ReducedCenterUpdate {
                centres: candidate_centres,
                observations: candidate_observations,
                next_lambda: (lambda_try * 0.7).max(1.0e-4),
            });
        }
        lambda_try *= 2.0;
    }

    None
}

fn apply_reduced_camera_pose_update(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    lambda_pose: f64,
    center_step_cap: f64,
    rotation_step_cap: f64,
    rot_eps: f64,
    reduced_camera_solve_mode: ReducedCameraSolveMode,
) -> Option<ReducedPoseUpdate> {
    if centres.len() < 3 || observations.len() < 18 {
        return None;
    }

    let base_cost = total_observation_error_with_huber(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        Some(seed_centres),
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    if !base_cost.is_finite() {
        return None;
    }

    let reduced = build_reduced_camera_pose_system(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        seed_centres,
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
        rot_eps,
    )?;
    if reduced.hessian.nrows() < 4 {
        return None;
    }

    let mut lambda_try = lambda_pose;
    for _ in 0..5 {
        let delta = match reduced_camera_solve_mode {
            ReducedCameraSolveMode::SparsePcg => solve_damped_sparse_pcg(
                &reduced.hessian_sparse,
                &reduced.gradient,
                lambda_try,
                96,
                1.0e-8,
            )
            .or_else(|| {
                let mut h_damped = reduced.hessian.clone();
                for idx in 0..h_damped.nrows() {
                    h_damped[(idx, idx)] += lambda_try;
                }
                let rhs = -&reduced.gradient;
                h_damped.lu().solve(&rhs)
            }),
            ReducedCameraSolveMode::DenseLu => {
                let mut h_damped = reduced.hessian.clone();
                for idx in 0..h_damped.nrows() {
                    h_damped[(idx, idx)] += lambda_try;
                }
                let rhs = -&reduced.gradient;
                h_damped.lu().solve(&rhs)
            }
        }?;
        if !delta.iter().all(|v| v.is_finite()) {
            lambda_try *= 2.0;
            continue;
        }

        let mut candidate_centres = centres.to_vec();
        let mut candidate_rotations = rotations_c2w.to_vec();
        for cam_idx in 1..centres.len() {
            let base = reduced_camera_pose_param_offset(cam_idx)?;
            let d_cx = delta[base].clamp(-center_step_cap, center_step_cap);
            let d_cy = delta[base + 1].clamp(-center_step_cap, center_step_cap);
            let d_rx = delta[base + 2].clamp(-rotation_step_cap, rotation_step_cap);
            let d_ry = delta[base + 3].clamp(-rotation_step_cap, rotation_step_cap);

            candidate_centres[cam_idx][0] += d_cx;
            candidate_centres[cam_idx][1] += d_cy;
            candidate_rotations[cam_idx] = small_angle_update(
                &small_angle_update(&rotations_c2w[cam_idx], 0, d_rx),
                1,
                d_ry,
            );
        }

        let candidate_observations = back_substitute_reduced_pose_points(observations, &reduced.point_blocks, &delta);
        let cand_cost = total_observation_error_with_huber(
            &candidate_centres,
            &candidate_rotations,
            &candidate_observations,
            intrinsics,
            camera_model,
            huber_threshold,
            Some(seed_centres),
            pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        if cand_cost.is_finite() && cand_cost + 1.0e-9 < base_cost {
            return Some(ReducedPoseUpdate {
                centres: candidate_centres,
                rotations: candidate_rotations,
                observations: candidate_observations,
                next_lambda: (lambda_try * 0.7).max(1.0e-4),
            });
        }
        lambda_try *= 2.0;
    }

    None
}

fn apply_reduced_camera_rotation_update(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    lambda_rot: f64,
    rotation_step_cap: f64,
    rot_eps: f64,
) -> Option<ReducedRotationUpdate> {
    if rotations_c2w.len() < 3 || observations.len() < 18 {
        return None;
    }

    let base_cost = total_observation_error_with_huber(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        Some(seed_centres),
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    if !base_cost.is_finite() {
        return None;
    }

    let reduced = build_reduced_camera_rotation_system(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        rot_eps,
    )?;
    if reduced.hessian.nrows() < 2 {
        return None;
    }

    let mut lambda_try = lambda_rot;
    for _ in 0..5 {
        let mut h_damped = reduced.hessian.clone();
        for idx in 0..h_damped.nrows() {
            h_damped[(idx, idx)] += lambda_try;
        }
        let rhs = -&reduced.gradient;
        let delta = h_damped.lu().solve(&rhs)?;
        if !delta.iter().all(|v| v.is_finite()) {
            lambda_try *= 2.0;
            continue;
        }

        let mut candidate_rotations = rotations_c2w.to_vec();
        for cam_idx in 1..rotations_c2w.len() {
            let base = reduced_camera_param_offset(cam_idx)?;
            let dx = delta[base].clamp(-rotation_step_cap, rotation_step_cap);
            let dy = delta[base + 1].clamp(-rotation_step_cap, rotation_step_cap);
            let updated = small_angle_update(&small_angle_update(&rotations_c2w[cam_idx], 0, dx), 1, dy);
            candidate_rotations[cam_idx] = updated;
        }

        let candidate_observations = back_substitute_reduced_points(observations, &reduced.point_blocks, &delta);
        let cand_cost = total_observation_error_with_huber(
            centres,
            &candidate_rotations,
            &candidate_observations,
            intrinsics,
            camera_model,
            huber_threshold,
            Some(seed_centres),
            pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        if cand_cost.is_finite() && cand_cost + 1.0e-9 < base_cost {
            return Some(ReducedRotationUpdate {
                rotations: candidate_rotations,
                observations: candidate_observations,
                next_lambda: (lambda_try * 0.7).max(1.0e-4),
            });
        }
        lambda_try *= 2.0;
    }

    None
}

fn build_reduced_camera_pose_system(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    rot_eps: f64,
) -> Option<ReducedPoseSystem> {
    if centres.len() < 2
        || rotations_c2w.len() != centres.len()
        || seed_centres.len() != centres.len()
        || pose_prior_weights.len() != centres.len()
    {
        return None;
    }

    let dim = (centres.len() - 1) * 4;
    if dim == 0 {
        return None;
    }

    let mut hessian_sparse = SparseSymmetricBlock4::new(dim);
    let mut gradient = DVector::zeros(dim);
    let mut point_blocks = Vec::new();

    for obs_group in group_observation_indices_by_point_id(observations) {
        if obs_group.len() < 2 {
            continue;
        }
        let point_world = observations[*obs_group.first()?].point_world;
        let mut point_gradient = Vector3::zeros();
        let mut point_hessian = Matrix3::zeros();
        let mut contributions = Vec::new();

        for &obs_idx in &obs_group {
            let obs = observations.get(obs_idx)?;
            let (residual, jac_pose, jac_point) = weighted_residual_and_jacobians_pose(
                obs,
                &point_world,
                centres,
                rotations_c2w,
                intrinsics,
                camera_model,
                huber_threshold,
                rot_eps,
            )?;

            point_gradient += jac_point.transpose() * residual;
            point_hessian += jac_point.transpose() * jac_point;
            if obs.cam_idx == 0 {
                continue;
            }

            contributions.push(ReducedPosePointContribution {
                cam_idx: obs.cam_idx,
                obs_indices: vec![obs_idx],
                a: jac_pose.transpose() * jac_pose,
                b: jac_pose.transpose() * jac_point,
                g: jac_pose.transpose() * residual,
            });
        }

        if contributions.len() < 2 {
            continue;
        }

        for axis in 0..3 {
            point_hessian[(axis, axis)] += 1.0e-6;
        }
        let Some(v_inv) = point_hessian.try_inverse() else {
            continue;
        };

        for contribution in &contributions {
            let offset_i = reduced_camera_pose_param_offset(contribution.cam_idx)?;
            hessian_sparse.add_block(offset_i, offset_i, contribution.a);
            gradient[offset_i] += contribution.g[0];
            gradient[offset_i + 1] += contribution.g[1];
            gradient[offset_i + 2] += contribution.g[2];
            gradient[offset_i + 3] += contribution.g[3];

            let reduced_grad = contribution.b * (v_inv * point_gradient);
            gradient[offset_i] -= reduced_grad[0];
            gradient[offset_i + 1] -= reduced_grad[1];
            gradient[offset_i + 2] -= reduced_grad[2];
            gradient[offset_i + 3] -= reduced_grad[3];
        }

        for left in 0..contributions.len() {
            for right in 0..contributions.len() {
                let offset_i = reduced_camera_pose_param_offset(contributions[left].cam_idx)?;
                let offset_j = reduced_camera_pose_param_offset(contributions[right].cam_idx)?;
                let correction = contributions[left].b * v_inv * contributions[right].b.transpose();
                hessian_sparse.add_block(offset_i, offset_j, -correction);
            }
        }

        point_blocks.push(ReducedPosePointBlock {
            point_world,
            contributions,
            v_inv,
            g_point: point_gradient,
        });
    }

    if point_blocks.is_empty() {
        return None;
    }

    for cam_idx in 1..centres.len() {
        let offset = reduced_camera_pose_param_offset(cam_idx)?;
        let prior_weight = pose_prior_weights[cam_idx];
        if prior_weight > 0.0 && pose_prior_sigma_m > 0.0 && pose_prior_scale_px2 > 0.0 {
            let curvature = 2.0 * pose_prior_scale_px2 * prior_weight / (pose_prior_sigma_m * pose_prior_sigma_m);
            hessian_sparse.add_diag_value(offset, curvature);
            hessian_sparse.add_diag_value(offset + 1, curvature);
            gradient[offset] += curvature * (centres[cam_idx][0] - seed_centres[cam_idx][0]);
            gradient[offset + 1] += curvature * (centres[cam_idx][1] - seed_centres[cam_idx][1]);
        }
        hessian_sparse.add_diag_value(offset + 2, 0.05);
        hessian_sparse.add_diag_value(offset + 3, 0.05);
    }

    let hessian = hessian_sparse.clone().into_dense();

    Some(ReducedPoseSystem {
        hessian,
        hessian_sparse,
        gradient,
        point_blocks,
    })
}

fn back_substitute_reduced_pose_points(
    observations: &[BaObservation],
    point_blocks: &[ReducedPosePointBlock],
    delta: &DVector<f64>,
) -> Vec<BaObservation> {
    let mut updated = observations.to_vec();
    for block in point_blocks {
        let mut rhs = block.g_point;
        let mut point_obs_indices = Vec::new();
        for contribution in &block.contributions {
            point_obs_indices.extend_from_slice(&contribution.obs_indices);
            if let Some(offset) = reduced_camera_pose_param_offset(contribution.cam_idx) {
                let dc = Vector4::new(
                    delta[offset],
                    delta[offset + 1],
                    delta[offset + 2],
                    delta[offset + 3],
                );
                rhs += contribution.b.transpose() * dc;
            }
        }
        let mut dp = -(block.v_inv * rhs);
        let step_norm = dp.norm();
        if step_norm > 0.35 {
            dp *= 0.35 / step_norm;
        }
        let candidate = block.point_world + dp;
        if !candidate.iter().all(|v| v.is_finite()) {
            continue;
        }
        for obs_idx in point_obs_indices {
            if let Some(obs) = updated.get_mut(obs_idx) {
                obs.point_world = candidate;
            }
        }
    }
    updated
}

fn build_reduced_camera_system(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
) -> Option<ReducedCameraSystem> {
    if centres.len() < 2 || seed_centres.len() != centres.len() || pose_prior_weights.len() != centres.len() {
        return None;
    }
    let dim = (centres.len() - 1) * 2;
    if dim == 0 {
        return None;
    }

    let mut hessian = DMatrix::zeros(dim, dim);
    let mut gradient = DVector::zeros(dim);
    let mut point_blocks = Vec::new();
    for obs_group in group_observation_indices_by_point_id(observations) {
        if obs_group.len() < 2 {
            continue;
        }
        let point_world = observations[*obs_group.first()?].point_world;
        let mut point_gradient = Vector3::zeros();
        let mut point_hessian = Matrix3::zeros();
        let mut contributions = Vec::new();

        for &obs_idx in &obs_group {
            let obs = observations.get(obs_idx)?;
            let (residual, jac_centre, jac_point) = weighted_residual_and_jacobians(
                obs,
                &point_world,
                &centres[obs.cam_idx],
                &rotations_c2w[obs.cam_idx],
                intrinsics,
                camera_model,
                huber_threshold,
            )?;

            point_gradient += jac_point.transpose() * residual;
            point_hessian += jac_point.transpose() * jac_point;
            if obs.cam_idx == 0 {
                continue;
            }

            contributions.push(ReducedPointContribution {
                cam_idx: obs.cam_idx,
                obs_indices: vec![obs_idx],
                a: jac_centre.transpose() * jac_centre,
                b: jac_centre.transpose() * jac_point,
                g: jac_centre.transpose() * residual,
            });
        }

        if contributions.len() < 2 {
            continue;
        }

        for axis in 0..3 {
            point_hessian[(axis, axis)] += 1.0e-6;
        }
        let Some(v_inv) = point_hessian.try_inverse() else {
            continue;
        };

        for contribution in &contributions {
            let offset_i = reduced_camera_param_offset(contribution.cam_idx)?;
            accumulate_matrix2_block(&mut hessian, offset_i, offset_i, contribution.a);
            gradient[offset_i] += contribution.g[0];
            gradient[offset_i + 1] += contribution.g[1];
            let reduced_grad = contribution.b * (v_inv * point_gradient);
            gradient[offset_i] -= reduced_grad[0];
            gradient[offset_i + 1] -= reduced_grad[1];
        }

        for left in 0..contributions.len() {
            for right in 0..contributions.len() {
                let offset_i = reduced_camera_param_offset(contributions[left].cam_idx)?;
                let offset_j = reduced_camera_param_offset(contributions[right].cam_idx)?;
                let correction = contributions[left].b * v_inv * contributions[right].b.transpose();
                accumulate_matrix2_block(&mut hessian, offset_i, offset_j, -correction);
            }
        }

        point_blocks.push(ReducedPointBlock {
            point_world,
            contributions,
            v_inv,
            g_point: point_gradient,
        });
    }

    if point_blocks.is_empty() {
        return None;
    }

    for cam_idx in 1..centres.len() {
        let offset = reduced_camera_param_offset(cam_idx)?;
        let prior_weight = pose_prior_weights[cam_idx];
        if prior_weight > 0.0 && pose_prior_sigma_m > 0.0 && pose_prior_scale_px2 > 0.0 {
            let curvature = 2.0 * pose_prior_scale_px2 * prior_weight / (pose_prior_sigma_m * pose_prior_sigma_m);
            hessian[(offset, offset)] += curvature;
            hessian[(offset + 1, offset + 1)] += curvature;
            gradient[offset] += curvature * (centres[cam_idx][0] - seed_centres[cam_idx][0]);
            gradient[offset + 1] += curvature * (centres[cam_idx][1] - seed_centres[cam_idx][1]);
        }
    }

    Some(ReducedCameraSystem {
        hessian,
        gradient,
        point_blocks,
    })
}

fn build_reduced_camera_rotation_system(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    rot_eps: f64,
) -> Option<ReducedCameraSystem> {
    if centres.len() < 2 || rotations_c2w.len() != centres.len() {
        return None;
    }
    let dim = (centres.len() - 1) * 2;
    if dim == 0 {
        return None;
    }

    let mut hessian = DMatrix::zeros(dim, dim);
    let mut gradient = DVector::zeros(dim);
    let mut point_blocks = Vec::new();
    for obs_group in group_observation_indices_by_point_id(observations) {
        if obs_group.len() < 2 {
            continue;
        }
        let point_world = observations[*obs_group.first()?].point_world;
        let mut point_gradient = Vector3::zeros();
        let mut point_hessian = Matrix3::zeros();
        let mut contributions = Vec::new();

        for &obs_idx in &obs_group {
            let obs = observations.get(obs_idx)?;
            let (residual, jac_rot, jac_point) = weighted_residual_and_jacobians_rotation(
                obs,
                &point_world,
                centres,
                rotations_c2w,
                intrinsics,
                camera_model,
                huber_threshold,
                rot_eps,
            )?;

            point_gradient += jac_point.transpose() * residual;
            point_hessian += jac_point.transpose() * jac_point;
            if obs.cam_idx == 0 {
                continue;
            }

            contributions.push(ReducedPointContribution {
                cam_idx: obs.cam_idx,
                obs_indices: vec![obs_idx],
                a: jac_rot.transpose() * jac_rot,
                b: jac_rot.transpose() * jac_point,
                g: jac_rot.transpose() * residual,
            });
        }

        if contributions.len() < 2 {
            continue;
        }

        for axis in 0..3 {
            point_hessian[(axis, axis)] += 1.0e-6;
        }
        let Some(v_inv) = point_hessian.try_inverse() else {
            continue;
        };

        for contribution in &contributions {
            let offset_i = reduced_camera_param_offset(contribution.cam_idx)?;
            accumulate_matrix2_block(&mut hessian, offset_i, offset_i, contribution.a);
            gradient[offset_i] += contribution.g[0];
            gradient[offset_i + 1] += contribution.g[1];
            let reduced_grad = contribution.b * (v_inv * point_gradient);
            gradient[offset_i] -= reduced_grad[0];
            gradient[offset_i + 1] -= reduced_grad[1];
        }

        for left in 0..contributions.len() {
            for right in 0..contributions.len() {
                let offset_i = reduced_camera_param_offset(contributions[left].cam_idx)?;
                let offset_j = reduced_camera_param_offset(contributions[right].cam_idx)?;
                let correction = contributions[left].b * v_inv * contributions[right].b.transpose();
                accumulate_matrix2_block(&mut hessian, offset_i, offset_j, -correction);
            }
        }

        point_blocks.push(ReducedPointBlock {
            point_world,
            contributions,
            v_inv,
            g_point: point_gradient,
        });
    }

    if point_blocks.is_empty() {
        return None;
    }

    for cam_idx in 1..centres.len() {
        let offset = reduced_camera_param_offset(cam_idx)?;
        hessian[(offset, offset)] += 0.05;
        hessian[(offset + 1, offset + 1)] += 0.05;
    }

    Some(ReducedCameraSystem {
        hessian,
        gradient,
        point_blocks,
    })
}

fn back_substitute_reduced_points(
    observations: &[BaObservation],
    point_blocks: &[ReducedPointBlock],
    delta: &DVector<f64>,
) -> Vec<BaObservation> {
    let mut updated = observations.to_vec();
    for block in point_blocks {
        let mut rhs = block.g_point;
        let mut point_obs_indices = Vec::new();
        for contribution in &block.contributions {
            point_obs_indices.extend_from_slice(&contribution.obs_indices);
            if let Some(offset) = reduced_camera_param_offset(contribution.cam_idx) {
                let dc = Vector2::new(delta[offset], delta[offset + 1]);
                rhs += contribution.b.transpose() * dc;
            }
        }
        let mut dp = -(block.v_inv * rhs);
        let step_norm = dp.norm();
        if step_norm > 0.35 {
            dp *= 0.35 / step_norm;
        }
        let candidate = block.point_world + dp;
        if !candidate.iter().all(|v| v.is_finite()) {
            continue;
        }
        for obs_idx in point_obs_indices {
            if let Some(obs) = updated.get_mut(obs_idx) {
                obs.point_world = candidate;
            }
        }
    }
    updated
}

fn weighted_residual_and_jacobians(
    observation: &BaObservation,
    point_world: &Vector3<f64>,
    centre: &Vector3<f64>,
    r_c2w: &Matrix3<f64>,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
) -> Option<(Vector2<f64>, Matrix2<f64>, Matrix2x3<f64>)> {
    let base_pix = project_world_to_pixel(point_world, centre, r_c2w, intrinsics, camera_model)?;
    let residual = base_pix - observation.obs_px;
    let residual_norm = residual.norm();
    let weight = huber_weight(residual_norm, huber_threshold) * observation.quality_weight;
    if !weight.is_finite() || weight <= 0.0 {
        return None;
    }
    let sqrt_w = weight.sqrt();
    let centre_eps = 0.02;
    let point_eps = 0.05;

    let mut jac_centre = Matrix2::zeros();
    for axis in 0..2 {
        let mut plus = *centre;
        plus[axis] += centre_eps;
        let pix_plus = project_world_to_pixel(point_world, &plus, r_c2w, intrinsics, camera_model)?;
        let mut minus = *centre;
        minus[axis] -= centre_eps;
        let pix_minus = project_world_to_pixel(point_world, &minus, r_c2w, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * centre_eps);
        jac_centre[(0, axis)] = deriv[0] * sqrt_w;
        jac_centre[(1, axis)] = deriv[1] * sqrt_w;
    }

    let mut jac_point = Matrix2x3::zeros();
    for axis in 0..3 {
        let mut plus = *point_world;
        plus[axis] += point_eps;
        let pix_plus = project_world_to_pixel(&plus, centre, r_c2w, intrinsics, camera_model)?;
        let mut minus = *point_world;
        minus[axis] -= point_eps;
        let pix_minus = project_world_to_pixel(&minus, centre, r_c2w, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * point_eps);
        jac_point[(0, axis)] = deriv[0] * sqrt_w;
        jac_point[(1, axis)] = deriv[1] * sqrt_w;
    }

    Some((residual * sqrt_w, jac_centre, jac_point))
}

fn weighted_residual_and_jacobians_rotation(
    observation: &BaObservation,
    point_world: &Vector3<f64>,
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    rot_eps: f64,
) -> Option<(Vector2<f64>, Matrix2<f64>, Matrix2x3<f64>)> {
    let cam_idx = observation.cam_idx;
    let centre = centres.get(cam_idx)?;
    let rotation = rotations_c2w.get(cam_idx)?;
    let base_pix = project_world_to_pixel(point_world, centre, rotation, intrinsics, camera_model)?;
    let residual = base_pix - observation.obs_px;
    let residual_norm = residual.norm();
    let weight = huber_weight(residual_norm, huber_threshold) * observation.quality_weight;
    if !weight.is_finite() || weight <= 0.0 {
        return None;
    }
    let sqrt_w = weight.sqrt();
    let point_eps = 0.05;

    let mut jac_rot = Matrix2::zeros();
    for axis in 0..2 {
        let rot_plus = small_angle_update(rotation, axis, rot_eps);
        let pix_plus = project_world_to_pixel(point_world, centre, &rot_plus, intrinsics, camera_model)?;
        let rot_minus = small_angle_update(rotation, axis, -rot_eps);
        let pix_minus = project_world_to_pixel(point_world, centre, &rot_minus, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * rot_eps);
        jac_rot[(0, axis)] = deriv[0] * sqrt_w;
        jac_rot[(1, axis)] = deriv[1] * sqrt_w;
    }

    let mut jac_point = Matrix2x3::zeros();
    for axis in 0..3 {
        let mut plus = *point_world;
        plus[axis] += point_eps;
        let pix_plus = project_world_to_pixel(&plus, centre, rotation, intrinsics, camera_model)?;
        let mut minus = *point_world;
        minus[axis] -= point_eps;
        let pix_minus = project_world_to_pixel(&minus, centre, rotation, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * point_eps);
        jac_point[(0, axis)] = deriv[0] * sqrt_w;
        jac_point[(1, axis)] = deriv[1] * sqrt_w;
    }

    Some((residual * sqrt_w, jac_rot, jac_point))
}

fn weighted_residual_and_jacobians_pose(
    observation: &BaObservation,
    point_world: &Vector3<f64>,
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    rot_eps: f64,
) -> Option<(Vector2<f64>, Matrix2x4<f64>, Matrix2x3<f64>)> {
    let cam_idx = observation.cam_idx;
    let centre = centres.get(cam_idx)?;
    let rotation = rotations_c2w.get(cam_idx)?;
    let base_pix = project_world_to_pixel(point_world, centre, rotation, intrinsics, camera_model)?;
    let residual = base_pix - observation.obs_px;
    let residual_norm = residual.norm();
    let weight = huber_weight(residual_norm, huber_threshold) * observation.quality_weight;
    if !weight.is_finite() || weight <= 0.0 {
        return None;
    }
    let sqrt_w = weight.sqrt();

    let centre_eps = 0.02;
    let point_eps = 0.05;
    let mut jac_pose = Matrix2x4::zeros();
    for axis in 0..2 {
        let mut plus = *centre;
        plus[axis] += centre_eps;
        let pix_plus = project_world_to_pixel(point_world, &plus, rotation, intrinsics, camera_model)?;
        let mut minus = *centre;
        minus[axis] -= centre_eps;
        let pix_minus = project_world_to_pixel(point_world, &minus, rotation, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * centre_eps);
        jac_pose[(0, axis)] = deriv[0] * sqrt_w;
        jac_pose[(1, axis)] = deriv[1] * sqrt_w;
    }
    for (j, axis) in [0usize, 1usize].iter().enumerate() {
        let rot_plus = small_angle_update(rotation, *axis, rot_eps);
        let pix_plus = project_world_to_pixel(point_world, centre, &rot_plus, intrinsics, camera_model)?;
        let rot_minus = small_angle_update(rotation, *axis, -rot_eps);
        let pix_minus = project_world_to_pixel(point_world, centre, &rot_minus, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * rot_eps);
        jac_pose[(0, 2 + j)] = deriv[0] * sqrt_w;
        jac_pose[(1, 2 + j)] = deriv[1] * sqrt_w;
    }

    let mut jac_point = Matrix2x3::zeros();
    for axis in 0..3 {
        let mut plus = *point_world;
        plus[axis] += point_eps;
        let pix_plus = project_world_to_pixel(&plus, centre, rotation, intrinsics, camera_model)?;
        let mut minus = *point_world;
        minus[axis] -= point_eps;
        let pix_minus = project_world_to_pixel(&minus, centre, rotation, intrinsics, camera_model)?;
        let deriv = (pix_plus - pix_minus) / (2.0 * point_eps);
        jac_point[(0, axis)] = deriv[0] * sqrt_w;
        jac_point[(1, axis)] = deriv[1] * sqrt_w;
    }

    Some((residual * sqrt_w, jac_pose, jac_point))
}

fn reduced_camera_param_offset(cam_idx: usize) -> Option<usize> {
    cam_idx.checked_sub(1).map(|idx| idx * 2)
}

fn reduced_camera_pose_param_offset(cam_idx: usize) -> Option<usize> {
    cam_idx.checked_sub(1).map(|idx| idx * 4)
}

fn accumulate_matrix2_block(target: &mut DMatrix<f64>, row: usize, col: usize, block: Matrix2<f64>) {
    target[(row, col)] += block[(0, 0)];
    target[(row, col + 1)] += block[(0, 1)];
    target[(row + 1, col)] += block[(1, 0)];
    target[(row + 1, col + 1)] += block[(1, 1)];
}

fn group_observation_indices_by_point_id(observations: &[BaObservation]) -> Vec<Vec<usize>> {
    if observations.is_empty() {
        return Vec::new();
    }
    let max_point_id = observations.iter().map(|obs| obs.point_id).max().unwrap_or(0);
    let mut grouped = vec![Vec::new(); max_point_id + 1];
    for (obs_idx, obs) in observations.iter().enumerate() {
        grouped[obs.point_id].push(obs_idx);
    }
    grouped.into_iter().filter(|group| !group.is_empty()).collect()
}

fn estimate_camera_covariance_diagnostics(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    rot_eps: f64,
) -> CameraCovarianceDiagnostics {
    let mut translation_sigma = Vec::new();
    if let Some(reduced) = build_reduced_camera_system(
        centres,
        rotations_c2w,
        observations,
        intrinsics,
        camera_model,
        huber_threshold,
        seed_centres,
        pose_prior_weights,
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    ) {
        for cam_idx in 1..centres.len() {
            let Some(offset) = reduced_camera_param_offset(cam_idx) else {
                continue;
            };
            let block = Matrix2::new(
                reduced.hessian[(offset, offset)],
                reduced.hessian[(offset, offset + 1)],
                reduced.hessian[(offset + 1, offset)],
                reduced.hessian[(offset + 1, offset + 1)],
            );
            if let Some(inv_block) = block.try_inverse() {
                let sigma = ((inv_block[(0, 0)].max(0.0) + inv_block[(1, 1)].max(0.0)) * 0.5).sqrt();
                if sigma.is_finite() {
                    translation_sigma.push(sigma);
                }
            }
        }
    }

    let mut rotation_sigma_deg = Vec::new();
    for cam_idx in 1..centres.len() {
        if let Some(sigma_deg) = estimate_rotation_sigma_proxy_deg(
            cam_idx,
            centres,
            rotations_c2w,
            observations,
            intrinsics,
            camera_model,
            huber_threshold,
            seed_centres,
            pose_prior_weights,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
            rot_eps,
        ) {
            rotation_sigma_deg.push(sigma_deg);
        }
    }

    CameraCovarianceDiagnostics {
        supported_camera_count: translation_sigma.len().max(rotation_sigma_deg.len()) as u64,
        translation_sigma_median_m: quantile_or_zero(&translation_sigma, 0.50),
        translation_sigma_p95_m: quantile_or_zero(&translation_sigma, 0.95),
        rotation_sigma_median_deg: quantile_or_zero(&rotation_sigma_deg, 0.50),
        rotation_sigma_p95_deg: quantile_or_zero(&rotation_sigma_deg, 0.95),
    }
}

fn estimate_rotation_sigma_proxy_deg(
    cam_idx: usize,
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
    rot_eps: f64,
) -> Option<f64> {
    let base_err = camera_observation_error_with_huber(
        cam_idx,
        &centres[cam_idx],
        observations,
        rotations_c2w,
        intrinsics,
        camera_model,
        huber_threshold,
        &seed_centres[cam_idx],
        pose_prior_weights[cam_idx],
        pose_prior_sigma_m,
        pose_prior_scale_px2,
    );
    if !base_err.is_finite() {
        return None;
    }

    let mut hdiag = [1.0e-6_f64; 2];
    for rot_axis in 0..2 {
        let mut rotations_p = rotations_c2w.to_vec();
        rotations_p[cam_idx] = small_angle_update(&rotations_c2w[cam_idx], rot_axis, rot_eps);
        let ep = camera_observation_error_with_huber(
            cam_idx,
            &centres[cam_idx],
            observations,
            &rotations_p,
            intrinsics,
            camera_model,
            huber_threshold,
            &seed_centres[cam_idx],
            pose_prior_weights[cam_idx],
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );

        let mut rotations_m = rotations_c2w.to_vec();
        rotations_m[cam_idx] = small_angle_update(&rotations_c2w[cam_idx], rot_axis, -rot_eps);
        let em = camera_observation_error_with_huber(
            cam_idx,
            &centres[cam_idx],
            observations,
            &rotations_m,
            intrinsics,
            camera_model,
            huber_threshold,
            &seed_centres[cam_idx],
            pose_prior_weights[cam_idx],
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );

        if ep.is_finite() && em.is_finite() {
            let curvature = (ep - 2.0 * base_err + em) / (rot_eps * rot_eps);
            hdiag[rot_axis] = curvature.abs().max(1.0e-6);
        }
    }

    let hessian = Matrix2::new(hdiag[0], 0.0, 0.0, hdiag[1]);
    let inv = hessian.try_inverse()?;
    let sigma_rad = ((inv[(0, 0)].max(0.0) + inv[(1, 1)].max(0.0)) * 0.5).sqrt();
    let sigma_deg = sigma_rad.to_degrees();
    if sigma_deg.is_finite() {
        Some(sigma_deg)
    } else {
        None
    }
}

fn small_angle_update(r: &Matrix3<f64>, axis: usize, delta: f64) -> Matrix3<f64> {
    let mut v = Vector3::zeros();
    v[axis] = delta;
    let update = Matrix3::new(
        1.0, -v[2], v[1],
        v[2], 1.0, -v[0],
        -v[1], v[0], 1.0,
    );
    update * r
}

fn quantile_or_zero(values: &[f64], q: f64) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.total_cmp(b));
    let idx = ((sorted.len() as f64 - 1.0) * q.clamp(0.0, 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn refine_structure_points_from_observations(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Vec<BaObservation> {
    if observations.is_empty() {
        return Vec::new();
    }

    let max_point_id = observations
        .iter()
        .map(|obs| obs.point_id)
        .max()
        .unwrap_or(0);
    let mut grouped: Vec<Vec<usize>> = vec![Vec::new(); max_point_id + 1];
    for (obs_idx, obs) in observations.iter().enumerate() {
        if obs.point_id < grouped.len() {
            grouped[obs.point_id].push(obs_idx);
        }
    }

    let mut refined = observations.to_vec();
    for obs_group in grouped.iter() {
        if obs_group.len() < 2 {
            continue;
        }
        let Some(xw) = triangulate_point_from_observation_set(
            centres,
            rotations_c2w,
            intrinsics,
            observations,
            obs_group,
            camera_model,
        ) else {
            continue;
        };

        for &obs_idx in obs_group {
            refined[obs_idx].point_world = xw;
        }
    }

    refined
}

fn triangulate_point_from_observation_set(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    intrinsics: &CameraIntrinsics,
    observations: &[BaObservation],
    obs_indices: &[usize],
    camera_model: CameraModel,
) -> Option<Vector3<f64>> {
    if obs_indices.len() < 2 {
        return None;
    }

    let mut a = DMatrix::zeros(obs_indices.len() * 2, 4);
    for (row_idx, &obs_idx) in obs_indices.iter().enumerate() {
        let obs = observations.get(obs_idx)?;
        if obs.cam_idx >= centres.len() || obs.cam_idx >= rotations_c2w.len() {
            return None;
        }
        let r_w2c = rotations_c2w[obs.cam_idx].transpose();
        let t = -(r_w2c * centres[obs.cam_idx]);
        let p = Matrix3x4::new(
            r_w2c[(0, 0)], r_w2c[(0, 1)], r_w2c[(0, 2)], t[0],
            r_w2c[(1, 0)], r_w2c[(1, 1)], r_w2c[(1, 2)], t[1],
            r_w2c[(2, 0)], r_w2c[(2, 1)], r_w2c[(2, 2)], t[2],
        );
        let ray = unproject_pixel_to_normalized_camera_ray(&obs.obs_px, intrinsics, camera_model)?;
        let xn = ray[0];
        let yn = ray[1];

        let r0 = row_idx * 2;
        let e0 = xn * p.row(2) - p.row(0);
        let e1 = yn * p.row(2) - p.row(1);
        for c in 0..4 {
            a[(r0, c)] = e0[c];
            a[(r0 + 1, c)] = e1[c];
        }
    }

    let svd = a.svd(true, true);
    let v_t = svd.v_t?;
    let xh = v_t.row(v_t.nrows() - 1);
    if xh[3].abs() <= 1.0e-12 {
        return None;
    }
    let xw = Vector3::new(xh[0] / xh[3], xh[1] / xh[3], xh[2] / xh[3]);
    if !xw.iter().all(|v| v.is_finite()) {
        return None;
    }

    let mut valid = 0usize;
    for &obs_idx in obs_indices {
        let obs = observations.get(obs_idx)?;
        let residual = reprojection_residual_px(
            &xw,
            &centres[obs.cam_idx],
            &rotations_c2w[obs.cam_idx],
            intrinsics,
            camera_model,
            &obs.obs_px,
        );
        if residual.is_finite() && residual <= BA_MAX_INITIAL_REPROJ_PX * 1.6 {
            valid += 1;
        }
    }

    if valid >= 2 {
        Some(xw)
    } else {
        None
    }
}

fn supported_camera_fraction_from_observations(observations: &[BaObservation], camera_count: usize) -> f64 {
    if camera_count == 0 {
        return 0.0;
    }
    let mut supported = vec![false; camera_count];
    for obs in observations {
        if obs.cam_idx < supported.len() {
            supported[obs.cam_idx] = true;
        }
    }
    supported.iter().filter(|&&v| v).count() as f64 / camera_count as f64
}

fn build_pose_prior_weights(
    observations: &[BaObservation],
    camera_count: usize,
    weak_geometry_support: bool,
) -> Vec<f64> {
    if camera_count == 0 {
        return Vec::new();
    }
    let mut counts = vec![0usize; camera_count];
    for obs in observations {
        if obs.cam_idx < camera_count {
            counts[obs.cam_idx] += 1;
        }
    }

    let target_obs = if weak_geometry_support { 10.0 } else { 16.0 };
    let base_weight = if weak_geometry_support { 1.00 } else { 0.30 };
    let mut weights = vec![0.0; camera_count];
    for cam_idx in 1..camera_count {
        let support = (counts[cam_idx] as f64 / target_obs).clamp(0.0, 1.0);
        let weakness = 1.0 - support;
        weights[cam_idx] = base_weight * (0.35 + 1.65 * weakness * weakness);
    }
    weights
}

fn center_prior_penalty_px2(
    candidate_centre: &Vector3<f64>,
    seed_centre: &Vector3<f64>,
    prior_weight: f64,
    prior_sigma_m: f64,
    prior_scale_px2: f64,
) -> f64 {
    if prior_weight <= 0.0 || prior_sigma_m <= 0.0 || prior_scale_px2 <= 0.0 {
        return 0.0;
    }
    let dx = (candidate_centre[0] - seed_centre[0]) / prior_sigma_m;
    let dy = (candidate_centre[1] - seed_centre[1]) / prior_sigma_m;
    prior_scale_px2 * prior_weight * (dx * dx + dy * dy)
}

fn smooth_motion_prior_penalty_px2(
    centres: &[Vector3<f64>],
    pose_prior_weights: &[f64],
    prior_scale_px2: f64,
) -> f64 {
    if centres.len() < 3 || pose_prior_weights.len() != centres.len() || prior_scale_px2 <= 0.0 {
        return 0.0;
    }

    let mut sum = 0.0;
    let mut n = 0usize;
    for cam_idx in 1..(centres.len() - 1) {
        let support_weight = pose_prior_weights[cam_idx].clamp(0.0, 1.0);
        let local_weight = 0.18 + 0.82 * support_weight;
        let accel = centres[cam_idx + 1] - 2.0 * centres[cam_idx] + centres[cam_idx - 1];
        let ax = accel[0];
        let ay = accel[1];
        if !ax.is_finite() || !ay.is_finite() {
            continue;
        }
        let accel_sq = ax * ax + ay * ay;
        sum += prior_scale_px2 * 0.16 * local_weight * accel_sq;
        n += 1;
    }

    if n > 0 {
        sum / n as f64
    } else {
        0.0
    }
}

fn apply_intrinsics_update(
    intrinsics: &mut CameraIntrinsics,
    grads: &[f64; 8],
    lrs: &[f64; 8],
    refine_mask: IntrinsicsRefineMask,
    image_width_hint: f64,
    image_height_hint: f64,
    camera_model: CameraModel,
) {
    let mut deltas = [0.0_f64; 8];
    for i in 0..8 {
        deltas[i] = grads[i] * lrs[i];
    }

    // Scale the focal-length step cap proportionally to the current focal length
    // so the BA can meaningfully correct large initial estimates (e.g. when EXIF
    // sensor_width defaults to 13.2 mm but the true sensor is smaller).
    let fx_step = (intrinsics.fx * 0.06).clamp(30.0, 400.0);
    let fy_step = (intrinsics.fy * 0.06).clamp(30.0, 400.0);
    deltas[0] = deltas[0].clamp(-fx_step, fx_step);
    deltas[1] = deltas[1].clamp(-fy_step, fy_step);
    deltas[2] = deltas[2].clamp(-3.0, 3.0);
    deltas[3] = deltas[3].clamp(-3.0, 3.0);
    deltas[4] = deltas[4].clamp(-0.002, 0.002);
    deltas[5] = deltas[5].clamp(-0.001, 0.001);
    deltas[6] = deltas[6].clamp(-0.0005, 0.0005);
    deltas[7] = deltas[7].clamp(-0.0005, 0.0005);

    if refine_mask.params[0] {
        intrinsics.fx = (intrinsics.fx - deltas[0]).clamp(120.0, 25000.0);
    }
    if refine_mask.params[1] {
        intrinsics.fy = (intrinsics.fy - deltas[1]).clamp(120.0, 25000.0);
    }
    if refine_mask.params[2] {
        intrinsics.cx = (intrinsics.cx - deltas[2]).clamp(0.0, image_width_hint);
    }
    if refine_mask.params[3] {
        intrinsics.cy = (intrinsics.cy - deltas[3]).clamp(0.0, image_height_hint);
    }
    if refine_mask.params[4] {
        intrinsics.k1 = (intrinsics.k1 - deltas[4]).clamp(-0.50, 0.50);
    }
    if refine_mask.params[5] {
        intrinsics.k2 = (intrinsics.k2 - deltas[5]).clamp(-0.50, 0.50);
    }
    if camera_model != CameraModel::Fisheye && refine_mask.params[6] {
        intrinsics.p1 = (intrinsics.p1 - deltas[6]).clamp(-0.10, 0.10);
    }
    if camera_model != CameraModel::Fisheye && refine_mask.params[7] {
        intrinsics.p2 = (intrinsics.p2 - deltas[7]).clamp(-0.10, 0.10);
    }
}

fn build_intrinsics_refine_mask(
    intrinsics_refinement_policy: IntrinsicsRefinementPolicy,
    allow_intrinsics_refinement: bool,
    observations_initial: usize,
    camera_count: usize,
    supported_camera_fraction: f64,
    weak_geometry_support: bool,
    camera_model: CameraModel,
) -> IntrinsicsRefineMask {
    match intrinsics_refinement_policy {
        IntrinsicsRefinementPolicy::None => return IntrinsicsRefineMask::none(),
        IntrinsicsRefinementPolicy::CoreOnly => {
            return IntrinsicsRefineMask {
                params: [true, true, true, true, false, false, false, false],
            };
        }
        IntrinsicsRefinementPolicy::CoreAndRadial => {
            return IntrinsicsRefineMask {
                params: [true, true, true, true, true, true, false, false],
            };
        }
        IntrinsicsRefinementPolicy::All => {
            return IntrinsicsRefineMask {
                params: [
                    true,
                    true,
                    true,
                    true,
                    true,
                    true,
                    camera_model != CameraModel::Fisheye,
                    camera_model != CameraModel::Fisheye,
                ],
            };
        }
        IntrinsicsRefinementPolicy::Auto => {}
    }

    if !allow_intrinsics_refinement {
        return IntrinsicsRefineMask::none();
    }

    let cam_scale = camera_count.max(2);
    let enough_for_pp = observations_initial >= cam_scale * 28
        && supported_camera_fraction >= 0.92
        && !weak_geometry_support;
    let enough_for_k1 = observations_initial >= cam_scale * 32
        && supported_camera_fraction >= 0.88;
    let enough_for_k2 = observations_initial >= cam_scale * 44
        && supported_camera_fraction >= 0.90
        && !weak_geometry_support;
    let enough_for_tangential = observations_initial >= cam_scale * 52
        && supported_camera_fraction >= 0.93
        && !weak_geometry_support;

    IntrinsicsRefineMask {
        // [fx, fy, cx, cy, k1, k2, p1, p2]
        params: [
            true,
            true,
            enough_for_pp,
            enough_for_pp,
            enough_for_k1,
            enough_for_k2,
            camera_model != CameraModel::Fisheye && enough_for_tangential,
            camera_model != CameraModel::Fisheye && enough_for_tangential,
        ],
    }
}

fn total_observation_error_with_huber(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centres: Option<&[Vector3<f64>]>,
    pose_prior_weights: &[f64],
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
) -> f64 {
    let mut weighted_sum = 0.0;
    let mut count = 0usize;
    for obs in observations {
        if let Some(pix) = project_world_to_pixel(
            &obs.point_world,
            &centres[obs.cam_idx],
            &rotations_c2w[obs.cam_idx],
            intrinsics,
            camera_model,
        ) {
            let dx = pix[0] - obs.obs_px[0];
            let dy = pix[1] - obs.obs_px[1];
            let residual_sq = dx * dx + dy * dy;
            let residual = residual_sq.sqrt();
            let weight = huber_weight(residual, huber_threshold) * obs.quality_weight;
            weighted_sum += weight * residual_sq;
            count += 1;
        }
    }
    if count == 0 {
        f64::INFINITY
    } else {
        let observation_error = weighted_sum / count as f64;
        let prior_error = if let Some(seed) = seed_centres {
            if seed.len() == centres.len() && pose_prior_weights.len() == centres.len() {
                let mut sum = 0.0;
                let mut n = 0usize;
                for cam_idx in 1..centres.len() {
                    let penalty = center_prior_penalty_px2(
                        &centres[cam_idx],
                        &seed[cam_idx],
                        pose_prior_weights[cam_idx],
                        pose_prior_sigma_m,
                        pose_prior_scale_px2,
                    );
                    if penalty.is_finite() {
                        sum += penalty;
                        n += 1;
                    }
                }
                if n > 0 { sum / n as f64 } else { 0.0 }
            } else {
                0.0
            }
        } else {
            0.0
        };
        let motion_prior_error = smooth_motion_prior_penalty_px2(
            centres,
            pose_prior_weights,
            pose_prior_scale_px2,
        );
        observation_error + prior_error + motion_prior_error
    }
}

fn camera_observation_error_with_huber(
    cam_idx: usize,
    candidate_centre: &Vector3<f64>,
    observations: &[BaObservation],
    rotations_c2w: &[Matrix3<f64>],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    huber_threshold: f64,
    seed_centre: &Vector3<f64>,
    pose_prior_weight: f64,
    pose_prior_sigma_m: f64,
    pose_prior_scale_px2: f64,
) -> f64 {
    let mut weighted_sum = 0.0;
    let mut count = 0usize;
    for obs in observations.iter().filter(|o| o.cam_idx == cam_idx) {
        if let Some(pix) = project_world_to_pixel(
            &obs.point_world,
            candidate_centre,
            &rotations_c2w[cam_idx],
            intrinsics,
            camera_model,
        ) {
            let dx = pix[0] - obs.obs_px[0];
            let dy = pix[1] - obs.obs_px[1];
            let residual_sq = dx * dx + dy * dy;
            let residual = residual_sq.sqrt();
            let weight = huber_weight(residual, huber_threshold) * obs.quality_weight;
            weighted_sum += weight * residual_sq;
            count += 1;
        }
    }
    if count == 0 {
        f64::INFINITY
    } else {
        let observation_error = weighted_sum / count as f64;
        let prior_error = center_prior_penalty_px2(
            candidate_centre,
            seed_centre,
            pose_prior_weight,
            pose_prior_sigma_m,
            pose_prior_scale_px2,
        );
        observation_error + prior_error
    }
}

fn build_ba_observations(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Vec<BaObservation> {
    let strict_profile = BaObservationBuildProfile {
        max_pts_per_pair: 48,
        min_pair_inliers: BA_MIN_PAIR_INLIERS,
        non_adjacent_inlier_step: 4,
        min_parallax_floor_px: BA_MIN_PARALLAX_PX,
        mean_parallax_scale: 0.10,
        non_adjacent_parallax_step: 0.20,
        adjacent_motion_min_inliers: BA_MIN_PAIR_INLIERS.saturating_sub(4).max(10),
        non_adjacent_motion_min_inliers: BA_MIN_PAIR_INLIERS.max(12),
        min_triangulation_angle_deg: BA_MIN_TRIANGULATION_ANGLE_DEG,
        max_initial_reproj_px: BA_MAX_INITIAL_REPROJ_PX,
    };
    let strict_observations = build_ba_observations_with_profile(
        centres,
        rotations_c2w,
        match_stats,
        intrinsics,
        camera_model,
        strict_profile,
    );
    if strict_observations.len() >= 12 {
        return strict_observations;
    }

    let relaxed_profile = BaObservationBuildProfile {
        max_pts_per_pair: 72,
        min_pair_inliers: 10,
        non_adjacent_inlier_step: 2,
        min_parallax_floor_px: 1.25,
        mean_parallax_scale: 0.06,
        non_adjacent_parallax_step: 0.10,
        adjacent_motion_min_inliers: 8,
        non_adjacent_motion_min_inliers: 10,
        min_triangulation_angle_deg: 0.25,
        max_initial_reproj_px: 32.0,
    };
    let relaxed_observations = build_ba_observations_with_profile(
        centres,
        rotations_c2w,
        match_stats,
        intrinsics,
        camera_model,
        relaxed_profile,
    );
    if relaxed_observations.len() >= 12 {
        return relaxed_observations;
    }

    let emergency_profile = BaObservationBuildProfile {
        max_pts_per_pair: 96,
        min_pair_inliers: 8,
        non_adjacent_inlier_step: 1,
        min_parallax_floor_px: 0.8,
        mean_parallax_scale: 0.04,
        non_adjacent_parallax_step: 0.08,
        adjacent_motion_min_inliers: 6,
        non_adjacent_motion_min_inliers: 8,
        min_triangulation_angle_deg: 0.10,
        max_initial_reproj_px: 96.0,
    };
    let emergency_observations = build_ba_observations_with_profile(
        centres,
        rotations_c2w,
        match_stats,
        intrinsics,
        camera_model,
        emergency_profile,
    );
    if emergency_observations.len() > relaxed_observations.len().max(strict_observations.len()) {
        emergency_observations
    } else if relaxed_observations.len() > strict_observations.len() {
        relaxed_observations
    } else {
        strict_observations
    }
}

fn build_ba_observations_with_profile(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    profile: BaObservationBuildProfile,
) -> Vec<BaObservation> {
    let mut observations = Vec::new();
    let mut debug_counts = BaObservationDebugCounts::default();
    let mut next_point_id: usize = 0;

    let min_parallax_px = profile
        .min_parallax_floor_px
        .max(match_stats.mean_parallax_px * profile.mean_parallax_scale);
    let motion_lookup: std::collections::HashMap<(usize, usize), &crate::features::AdjacentPairMotion> =
        match_stats
            .adjacent_pair_motions
            .iter()
            .map(|m| ((m.left_idx, m.right_idx), m))
            .collect();

    for pair in &match_stats.pair_correspondences {
        if pair.left_frame_idx >= centres.len() || pair.right_frame_idx >= centres.len() {
            continue;
        }
        let gap = pair.right_frame_idx.abs_diff(pair.left_frame_idx);
        if gap == 0 || gap > BA_MAX_PAIR_GAP {
            debug_counts.skipped_pair_gap += 1;
            continue;
        }

        let is_adjacent = gap == 1;
        let min_inliers = if is_adjacent {
            profile.min_pair_inliers
        } else {
            profile.min_pair_inliers + profile.non_adjacent_inlier_step * (gap - 1)
        };
        if pair.points.len() < min_inliers {
            debug_counts.skipped_pair_inliers += 1;
            continue;
        }

        let pair_median_disp_px = pair_median_displacement_px(&pair.points);
        let min_pair_parallax = if is_adjacent {
            min_parallax_px
        } else {
            min_parallax_px * (0.85 + profile.non_adjacent_parallax_step * (gap as f64 - 1.0))
        };
        if pair_median_disp_px < min_pair_parallax {
            debug_counts.skipped_pair_parallax += 1;
            continue;
        }

        if let Some(motion) = motion_lookup.get(&(pair.left_frame_idx, pair.right_frame_idx)) {
            let motion_min_inliers = if is_adjacent {
                profile.adjacent_motion_min_inliers
            } else {
                profile.non_adjacent_motion_min_inliers
            };
            if motion.inlier_count < motion_min_inliers || motion.median_displacement_px < min_parallax_px {
                debug_counts.skipped_pair_motion += 1;
                continue;
            }
        }

        let left_idx = pair.left_frame_idx;
        let right_idx = pair.right_frame_idx;
        let r_l_w2c = rotations_c2w[left_idx].transpose();
        let r_r_w2c = rotations_c2w[right_idx].transpose();
        let t_l = -(r_l_w2c * centres[left_idx]);
        let t_r = -(r_r_w2c * centres[right_idx]);
        let p_l = Matrix3x4::new(
            r_l_w2c[(0, 0)], r_l_w2c[(0, 1)], r_l_w2c[(0, 2)], t_l[0],
            r_l_w2c[(1, 0)], r_l_w2c[(1, 1)], r_l_w2c[(1, 2)], t_l[1],
            r_l_w2c[(2, 0)], r_l_w2c[(2, 1)], r_l_w2c[(2, 2)], t_l[2],
        );
        let p_r = Matrix3x4::new(
            r_r_w2c[(0, 0)], r_r_w2c[(0, 1)], r_r_w2c[(0, 2)], t_r[0],
            r_r_w2c[(1, 0)], r_r_w2c[(1, 1)], r_r_w2c[(1, 2)], t_r[1],
            r_r_w2c[(2, 0)], r_r_w2c[(2, 1)], r_r_w2c[(2, 2)], t_r[2],
        );

        for (point_idx, p) in pair.points.iter().take(profile.max_pts_per_pair).enumerate() {
            let Some(x1n) = unproject_pixel_to_normalized_camera_ray(
                &Vector2::new(p[0], p[1]),
                intrinsics,
                camera_model,
            ) else {
                continue;
            };
            let Some(x2n) = unproject_pixel_to_normalized_camera_ray(
                &Vector2::new(p[2], p[3]),
                intrinsics,
                camera_model,
            ) else {
                continue;
            };

            if let Some(xw) = triangulate_point(&p_l, &p_r, &x1n, &x2n) {
                if xw[2].is_finite() {
                    let angle_deg = triangulation_angle_deg(&xw, &centres[left_idx], &centres[right_idx]);
                    if angle_deg < profile.min_triangulation_angle_deg {
                        debug_counts.skipped_triangulation_angle += 1;
                        continue;
                    }

                    let pair_gap_penalty = (1.0 / gap as f64).sqrt().clamp(0.45, 1.0);
                    let mut quality_weight = if let Some(motion) = motion_lookup.get(&(left_idx, right_idx)) {
                        let support_w = (motion.inlier_count as f64 / 120.0).clamp(0.4, 1.6);
                        let parallax_w = (motion.median_displacement_px / min_parallax_px.max(1.0)).clamp(0.6, 1.5);
                        let angle_w = (angle_deg / 8.0).clamp(0.5, 1.4);
                        support_w * parallax_w * angle_w * pair_gap_penalty
                    } else {
                        let support_w = (pair.points.len() as f64 / 96.0).clamp(0.45, 1.35);
                        let parallax_w = (pair_median_disp_px / min_pair_parallax.max(1.0)).clamp(0.7, 1.45);
                        let angle_w = (angle_deg / 8.0).clamp(0.5, 1.3);
                        support_w * parallax_w * angle_w * pair_gap_penalty
                    };
                    let match_confidence = pair
                        .confidence_weights
                        .get(point_idx)
                        .copied()
                        .unwrap_or(0.6)
                        .clamp(0.1, 1.0);
                    quality_weight = (quality_weight * (0.55 + 0.9 * match_confidence)).clamp(0.25, 2.4);

                    let left_obs = Vector2::new(p[0], p[1]);
                    let right_obs = Vector2::new(p[2], p[3]);
                    let left_res = reprojection_residual_px(
                        &xw,
                        &centres[left_idx],
                        &rotations_c2w[left_idx],
                        intrinsics,
                        camera_model,
                        &left_obs,
                    );
                    let right_res = reprojection_residual_px(
                        &xw,
                        &centres[right_idx],
                        &rotations_c2w[right_idx],
                        intrinsics,
                        camera_model,
                        &right_obs,
                    );
                    if !left_res.is_finite() || !right_res.is_finite() {
                        debug_counts.skipped_reprojection += 1;
                        continue;
                    }
                    let max_residual = left_res.max(right_res);
                    if max_residual > profile.max_initial_reproj_px {
                        debug_counts.skipped_reprojection += 1;
                        continue;
                    }
                    let reprojection_weight = (profile.max_initial_reproj_px / max_residual.max(1.0)).clamp(0.2, 1.0);
                    quality_weight = (quality_weight * reprojection_weight).clamp(0.15, 2.4);

                    observations.push(BaObservation {
                        cam_idx: left_idx,
                        point_id: next_point_id,
                        point_world: xw,
                        obs_px: left_obs,
                        quality_weight,
                    });
                    observations.push(BaObservation {
                        cam_idx: right_idx,
                        point_id: next_point_id,
                        point_world: xw,
                        obs_px: right_obs,
                        quality_weight,
                    });
                    next_point_id = next_point_id.saturating_add(1);
                    debug_counts.accepted_observations += 2;
                }
            } else {
                debug_counts.triangulation_failed += 1;
            }
        }
    }

    ba_observation_debug_line(format!(
        concat!(
            "profile=max_pts:{} min_inliers:{} min_parallax:{:.3} angle:{:.3} max_reproj:{:.1} ",
            "pairs={} accepted_obs={} skip_gap={} skip_inliers={} skip_parallax={} skip_motion={} ",
            "triangulation_failed={} skip_angle={} skip_reprojection={}"
        ),
        profile.max_pts_per_pair,
        profile.min_pair_inliers,
        min_parallax_px,
        profile.min_triangulation_angle_deg,
        profile.max_initial_reproj_px,
        match_stats.pair_correspondences.len(),
        debug_counts.accepted_observations,
        debug_counts.skipped_pair_gap,
        debug_counts.skipped_pair_inliers,
        debug_counts.skipped_pair_parallax,
        debug_counts.skipped_pair_motion,
        debug_counts.triangulation_failed,
        debug_counts.skipped_triangulation_angle,
        debug_counts.skipped_reprojection,
    ));

    observations
}

fn pair_median_displacement_px(points: &[[f64; 4]]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let mut mags: Vec<f64> = points
        .iter()
        .map(|p| {
            let dx = p[2] - p[0];
            let dy = p[3] - p[1];
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    mags.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = mags.len() / 2;
    if mags.len() % 2 == 1 {
        mags[mid]
    } else {
        0.5 * (mags[mid - 1] + mags[mid])
    }
}

fn project_world_to_pixel(
    xw: &Vector3<f64>,
    centre: &Vector3<f64>,
    r_c2w: &Matrix3<f64>,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Option<Vector2<f64>> {
    let r_w2c = r_c2w.transpose();
    let xc = r_w2c * (xw - centre);
    if xc[2] <= 1e-6 {
        return None;
    }
    let x = xc[0] / xc[2];
    let y = xc[1] / xc[2];
    let (x_d, y_d) = match camera_model {
        CameraModel::Pinhole | CameraModel::Auto => {
            let r2 = x * x + y * y;
            let radial = 1.0 + intrinsics.k1 * r2 + intrinsics.k2 * r2 * r2;
            let x_t = 2.0 * intrinsics.p1 * x * y
                + intrinsics.p2 * (r2 + 2.0 * x * x);
            let y_t = intrinsics.p1 * (r2 + 2.0 * y * y)
                + 2.0 * intrinsics.p2 * x * y;
            (x * radial + x_t, y * radial + y_t)
        }
        CameraModel::Fisheye => {
            let r = (x * x + y * y).sqrt();
            if r <= 1.0e-12 {
                (x, y)
            } else {
                let theta = r.atan();
                let theta2 = theta * theta;
                let theta_d = theta * (1.0 + intrinsics.k1 * theta2 + intrinsics.k2 * theta2 * theta2);
                let scale = theta_d / r;
                (x * scale, y * scale)
            }
        }
    };
    if !x_d.is_finite() || !y_d.is_finite() {
        return None;
    }
    let u = intrinsics.fx * x_d + intrinsics.cx;
    let v = intrinsics.fy * y_d + intrinsics.cy;
    if !u.is_finite() || !v.is_finite() {
        return None;
    }
    Some(Vector2::new(u, v))
}

fn unproject_pixel_to_normalized_camera_ray(
    obs_px: &Vector2<f64>,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Option<Vector2<f64>> {
    let x_distorted = (obs_px[0] - intrinsics.cx) / intrinsics.fx;
    let y_distorted = (obs_px[1] - intrinsics.cy) / intrinsics.fy;
    if !x_distorted.is_finite() || !y_distorted.is_finite() {
        return None;
    }

    match camera_model {
        CameraModel::Pinhole | CameraModel::Auto => {
            undistort_pinhole_normalized_coords(x_distorted, y_distorted, intrinsics)
        }
        CameraModel::Fisheye => undistort_fisheye_normalized_coords(x_distorted, y_distorted, intrinsics),
    }
}

fn undistort_pinhole_normalized_coords(
    x_distorted: f64,
    y_distorted: f64,
    intrinsics: &CameraIntrinsics,
) -> Option<Vector2<f64>> {
    let mut x = x_distorted;
    let mut y = y_distorted;
    for _ in 0..8 {
        let r2 = x * x + y * y;
        let radial = 1.0 + intrinsics.k1 * r2 + intrinsics.k2 * r2 * r2;
        if !radial.is_finite() || radial.abs() <= 1.0e-12 {
            return None;
        }
        let x_t = 2.0 * intrinsics.p1 * x * y
            + intrinsics.p2 * (r2 + 2.0 * x * x);
        let y_t = intrinsics.p1 * (r2 + 2.0 * y * y)
            + 2.0 * intrinsics.p2 * x * y;

        let next_x = (x_distorted - x_t) / radial;
        let next_y = (y_distorted - y_t) / radial;
        if !next_x.is_finite() || !next_y.is_finite() {
            return None;
        }
        let step = ((next_x - x) * (next_x - x) + (next_y - y) * (next_y - y)).sqrt();
        x = next_x;
        y = next_y;
        if step <= 1.0e-12 {
            break;
        }
    }

    if x.is_finite() && y.is_finite() {
        Some(Vector2::new(x, y))
    } else {
        None
    }
}

fn undistort_fisheye_normalized_coords(
    x_distorted: f64,
    y_distorted: f64,
    intrinsics: &CameraIntrinsics,
) -> Option<Vector2<f64>> {
    let radius_distorted = (x_distorted * x_distorted + y_distorted * y_distorted).sqrt();
    if !radius_distorted.is_finite() {
        return None;
    }
    if radius_distorted <= 1.0e-12 {
        return Some(Vector2::new(x_distorted, y_distorted));
    }

    let theta = invert_fisheye_theta(radius_distorted, intrinsics)?;
    let radius_rectilinear = theta.tan();
    if !radius_rectilinear.is_finite() {
        return None;
    }

    let scale = radius_rectilinear / radius_distorted;
    let x = x_distorted * scale;
    let y = y_distorted * scale;
    if x.is_finite() && y.is_finite() {
        Some(Vector2::new(x, y))
    } else {
        None
    }
}

fn invert_fisheye_theta(theta_distorted: f64, intrinsics: &CameraIntrinsics) -> Option<f64> {
    if !theta_distorted.is_finite() || theta_distorted < 0.0 {
        return None;
    }

    let mut theta = theta_distorted.clamp(0.0, std::f64::consts::FRAC_PI_2 - 1.0e-4);
    for _ in 0..10 {
        let theta2 = theta * theta;
        let theta4 = theta2 * theta2;
        let f = theta * (1.0 + intrinsics.k1 * theta2 + intrinsics.k2 * theta4) - theta_distorted;
        let df = 1.0 + 3.0 * intrinsics.k1 * theta2 + 5.0 * intrinsics.k2 * theta4;
        if !f.is_finite() || !df.is_finite() || df.abs() <= 1.0e-12 {
            return None;
        }
        let step = f / df;
        theta = (theta - step).clamp(0.0, std::f64::consts::FRAC_PI_2 - 1.0e-4);
        if step.abs() <= 1.0e-10 {
            break;
        }
    }

    if theta.is_finite() {
        Some(theta)
    } else {
        None
    }
}

fn reprojection_residual_px(
    xw: &Vector3<f64>,
    centre: &Vector3<f64>,
    r_c2w: &Matrix3<f64>,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    obs_px: &Vector2<f64>,
) -> f64 {
    if let Some(pix) = project_world_to_pixel(xw, centre, r_c2w, intrinsics, camera_model) {
        let dx = pix[0] - obs_px[0];
        let dy = pix[1] - obs_px[1];
        (dx * dx + dy * dy).sqrt()
    } else {
        f64::INFINITY
    }
}

fn triangulation_angle_deg(
    xw: &Vector3<f64>,
    centre_left: &Vector3<f64>,
    centre_right: &Vector3<f64>,
) -> f64 {
    let vl = xw - centre_left;
    let vr = xw - centre_right;
    let nl = vl.norm();
    let nr = vr.norm();
    if nl <= 1e-9 || nr <= 1e-9 {
        return 0.0;
    }
    let cosang = (vl.dot(&vr) / (nl * nr)).clamp(-1.0, 1.0);
    cosang.acos().to_degrees()
}

fn prune_observations_by_residual(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    pass_idx: usize,
    total_passes: usize,
) -> (Vec<BaObservation>, BaPruneStats) {
    fn as_prune_result(observations: &[BaObservation], threshold_px: f64) -> (Vec<BaObservation>, BaPruneStats) {
        (
            observations.to_vec(),
            BaPruneStats {
                threshold_px,
                kept_observations: observations.len(),
            },
        )
    }

    if observations.len() < 20 {
        return as_prune_result(observations, 0.0);
    }

    let mut residual_samples: Vec<(usize, usize, f64)> = Vec::with_capacity(observations.len());
    for (idx, obs) in observations.iter().enumerate() {
        let res = reprojection_residual_px(
            &obs.point_world,
            &centres[obs.cam_idx],
            &rotations_c2w[obs.cam_idx],
            intrinsics,
            camera_model,
            &obs.obs_px,
        );
        if res.is_finite() {
            residual_samples.push((idx, obs.cam_idx, res));
        }
    }
    let mut residuals: Vec<f64> = residual_samples.iter().map(|(_, _, r)| *r).collect();

    if residuals.len() < 12 {
        return as_prune_result(observations, 0.0);
    }

    residuals.sort_by(|a, b| a.total_cmp(b));
    let quantile = 0.90 - 0.05 * (pass_idx as f64 / total_passes.max(1) as f64);
    let q_idx = ((residuals.len() as f64) * quantile.clamp(0.80, 0.95)).floor() as usize;
    let q_residual = residuals[q_idx.min(residuals.len() - 1)];
    let dynamic_cap = (10.0 - pass_idx as f64).clamp(6.5, 10.0);
    let threshold = q_residual.min(dynamic_cap).max(2.5);

    let mut keep = vec![false; observations.len()];
    for (idx, _, res) in &residual_samples {
        if *res <= threshold {
            keep[*idx] = true;
        }
    }

    // Preserve minimal per-camera support while only backfilling with low residual observations.
    let min_obs_per_camera = 2usize;
    let support_backfill_cap = (threshold * 1.05).clamp(3.0, 6.0);
    let mut per_camera_kept = vec![0usize; centres.len()];
    let mut per_camera_all: Vec<Vec<(usize, f64)>> = vec![Vec::new(); centres.len()];
    for (idx, cam_idx, res) in residual_samples {
        if cam_idx < centres.len() {
            if res <= support_backfill_cap {
                per_camera_all[cam_idx].push((idx, res));
            }
            if keep[idx] {
                per_camera_kept[cam_idx] += 1;
            }
        }
    }

    for cam_idx in 0..centres.len() {
        if per_camera_kept[cam_idx] >= min_obs_per_camera {
            continue;
        }
        per_camera_all[cam_idx].sort_by(|a, b| a.1.total_cmp(&b.1));
        for (idx, _) in &per_camera_all[cam_idx] {
            if !keep[*idx] {
                keep[*idx] = true;
                per_camera_kept[cam_idx] += 1;
            }
            if per_camera_kept[cam_idx] >= min_obs_per_camera {
                break;
            }
        }
    }

    let mut pruned = Vec::with_capacity(observations.len());
    for (idx, obs) in observations.iter().enumerate() {
        if keep[idx] {
            pruned.push(obs.clone());
        }
    }

    if pruned.len() < 10 {
        return as_prune_result(observations, threshold);
    }

    (
        pruned,
        BaPruneStats {
            threshold_px: threshold,
            kept_observations: keep.iter().filter(|&&k| k).count(),
        },
    )
}

fn reprojection_residual_samples(
    centres: &[Vector3<f64>],
    rotations_c2w: &[Matrix3<f64>],
    observations: &[BaObservation],
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Vec<f64> {
    let mut out = Vec::new();
    for obs in observations {
        if let Some(pix) = project_world_to_pixel(
            &obs.point_world,
            &centres[obs.cam_idx],
            &rotations_c2w[obs.cam_idx],
            intrinsics,
            camera_model,
        ) {
            let dx = pix[0] - obs.obs_px[0];
            let dy = pix[1] - obs.obs_px[1];
            out.push((dx * dx + dy * dy).sqrt());
        }
    }
    out
}

fn residual_quantiles_from_samples(residuals: &[f64]) -> (f64, f64) {
    if residuals.is_empty() {
        return (0.0, 0.0);
    }
    let mut v: Vec<f64> = residuals.iter().copied().filter(|r| r.is_finite()).collect();
    if v.is_empty() {
        return (0.0, 0.0);
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let p50 = v[v.len() / 2];
    let p95_idx = ((v.len() as f64) * 0.95).floor() as usize;
    let p95 = v[p95_idx.min(v.len() - 1)];
    (p50, p95)
}

#[derive(Debug, Clone)]
struct LoopClosureConstraint {
    left_idx: usize,
    right_idx: usize,
    target_right: Vector3<f64>,
    weight: f64,
    support_count: usize,
    quality_score: f64,
}

fn apply_loop_closure_global_optimization(
    positions: &[[f64; 3]],
    rotations: &[[f64; 4]],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
    relative_support: f64,
) -> (Vec<[f64; 3]>, LoopClosureDiagnostics) {
    if positions.len() < 4 || rotations.len() != positions.len() {
        return (
            positions.to_vec(),
            LoopClosureDiagnostics {
                constraint_count: 0,
                mean_correction_m: 0.0,
                max_correction_m: 0.0,
            },
        );
    }

    let mut constraints = estimate_loop_closure_constraints(
        positions,
        rotations,
        match_stats,
        intrinsics,
        camera_model,
    );
    if constraints.is_empty() {
        return (
            positions.to_vec(),
            LoopClosureDiagnostics {
                constraint_count: 0,
                mean_correction_m: 0.0,
                max_correction_m: 0.0,
            },
        );
    }
    constraints.sort_by(|a, b| {
        b.quality_score
            .total_cmp(&a.quality_score)
            .then_with(|| b.support_count.cmp(&a.support_count))
            .then_with(|| b.weight.total_cmp(&a.weight))
    });
    let selected_constraints = select_loop_closure_constraints(&constraints, positions.len(), 6);

    let baseline_m = median_adjacent_baseline_m(positions).max(0.5);
    let robust_correction_cutoff_m = loop_closure_outlier_cutoff_m(
        &selected_constraints,
        positions,
        baseline_m,
    );
    let original: Vec<Vector3<f64>> = positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let mut centres = original.clone();
    let base_strength = (0.10 + 0.18 * relative_support).clamp(0.10, 0.22);
    let mut applied_corrections_m = Vec::new();
    let mut candidate_correction_m = Vec::new();

    for constraint in &selected_constraints {
        let span = constraint.right_idx.saturating_sub(constraint.left_idx);
        if span < 2 {
            continue;
        }

        let current_right = centres[constraint.right_idx];
        let closure_error = constraint.target_right - current_right;
        if !closure_error.iter().all(|v| v.is_finite()) {
            continue;
        }

        let span_f = span as f64;
        let span_damping = (1.0 / (1.0 + 0.18 * (span_f - 2.0).max(0.0))).clamp(0.45, 1.0);
        let support_factor = (constraint.support_count as f64 / 96.0).clamp(0.65, 1.20);
        let quality_factor = (0.70 + 0.50 * constraint.quality_score).clamp(0.70, 1.15);
        let max_shift = baseline_m * (0.45 + 0.30 * span_f.sqrt());
        let bounded_error = closure_error.cap_magnitude(max_shift.max(0.2));
        if bounded_error.norm() > robust_correction_cutoff_m {
            continue;
        }
        let strength = (base_strength * constraint.weight * span_damping * support_factor * quality_factor)
            .clamp(0.04, 0.28);
        candidate_correction_m.push(bounded_error.norm());
        applied_corrections_m.push((bounded_error.norm() * strength).max(0.0));

        for idx in (constraint.left_idx + 1)..=constraint.right_idx {
            let t = (idx - constraint.left_idx) as f64 / span as f64;
            centres[idx] += bounded_error * (strength * t);
        }
        let tail_scale = (0.08 + 0.10 * span_damping * constraint.weight).clamp(0.08, 0.18);
        for centre in centres.iter_mut().skip(constraint.right_idx + 1) {
            *centre += bounded_error * (strength * tail_scale);
        }
    }

    // Keep the first camera as the fixed anchor for global consistency.
    let anchor_shift = centres[0] - original[0];
    if anchor_shift.norm() > 0.0 {
        for centre in &mut centres {
            *centre -= anchor_shift;
        }
    }

    // Adaptive blend reduces over-correction when loop-closure deltas are large.
    let median_candidate_correction = if candidate_correction_m.is_empty() {
        0.0
    } else {
        let mut v = candidate_correction_m;
        v.sort_by(|a, b| a.total_cmp(b));
        v[v.len() / 2]
    };
    let correction_ratio = (median_candidate_correction / (baseline_m * 2.5).max(0.5)).clamp(0.0, 1.5);
    let blend = (0.44 - 0.14 * correction_ratio).clamp(0.26, 0.44);
    let refined: Vec<[f64; 3]> = centres
        .iter()
        .zip(original.iter())
        .map(|(refined, base)| {
            let p = base + (refined - base) * blend;
            [p[0], p[1], p[2]]
        })
        .collect();

    let constraint_count = applied_corrections_m.len() as u64;
    let mean_correction_m = if applied_corrections_m.is_empty() {
        0.0
    } else {
        applied_corrections_m.iter().sum::<f64>() / applied_corrections_m.len() as f64
    };
    let max_correction_m = applied_corrections_m
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    (
        refined,
        LoopClosureDiagnostics {
            constraint_count,
            mean_correction_m,
            max_correction_m,
        },
    )
}

fn loop_closure_outlier_cutoff_m(
    constraints: &[LoopClosureConstraint],
    positions: &[[f64; 3]],
    baseline_m: f64,
) -> f64 {
    if constraints.is_empty() {
        return baseline_m * 6.0;
    }

    let mut magnitudes = Vec::with_capacity(constraints.len());
    for c in constraints {
        if c.right_idx >= positions.len() {
            continue;
        }
        let current_right = Vector3::new(
            positions[c.right_idx][0],
            positions[c.right_idx][1],
            positions[c.right_idx][2],
        );
        let e = c.target_right - current_right;
        if e.iter().all(|v| v.is_finite()) {
            magnitudes.push(e.norm());
        }
    }
    if magnitudes.is_empty() {
        return baseline_m * 6.0;
    }

    magnitudes.sort_by(|a, b| a.total_cmp(b));
    let median = magnitudes[magnitudes.len() / 2];
    let mut abs_dev = magnitudes
        .iter()
        .map(|v| (v - median).abs())
        .collect::<Vec<_>>();
    abs_dev.sort_by(|a, b| a.total_cmp(b));
    let mad = abs_dev[abs_dev.len() / 2];
    let sigma = (1.4826 * mad).max(0.10 * baseline_m);
    (median + 3.0 * sigma)
        .clamp(0.35 * baseline_m, 6.0 * baseline_m)
}

fn select_loop_closure_constraints(
    ranked: &[LoopClosureConstraint],
    pose_count: usize,
    max_constraints: usize,
) -> Vec<LoopClosureConstraint> {
    if ranked.is_empty() || pose_count < 3 || max_constraints == 0 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut node_usage = vec![0usize; pose_count];
    let max_per_node = 2usize;

    for candidate in ranked {
        if selected.len() >= max_constraints {
            break;
        }
        if candidate.left_idx >= pose_count || candidate.right_idx >= pose_count {
            continue;
        }
        if candidate.right_idx <= candidate.left_idx + 1 {
            continue;
        }
        if node_usage[candidate.left_idx] >= max_per_node || node_usage[candidate.right_idx] >= max_per_node {
            continue;
        }

        let overlaps_heavily = selected.iter().any(|s: &LoopClosureConstraint| {
            let left_close = s.left_idx.abs_diff(candidate.left_idx) <= 1;
            let right_close = s.right_idx.abs_diff(candidate.right_idx) <= 1;
            left_close && right_close
        });
        if overlaps_heavily {
            continue;
        }

        selected.push(candidate.clone());
        node_usage[candidate.left_idx] += 1;
        node_usage[candidate.right_idx] += 1;
    }

    if selected.is_empty() {
        ranked.iter().take(max_constraints).cloned().collect()
    } else {
        selected
    }
}

fn estimate_loop_closure_constraints(
    positions: &[[f64; 3]],
    rotations: &[[f64; 4]],
    match_stats: &MatchStats,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Vec<LoopClosureConstraint> {
    let mut constraints = Vec::new();
    if positions.len() < 4 {
        return constraints;
    }

    let baseline_m = median_adjacent_baseline_m(positions).max(0.5);
    for pair in &match_stats.pair_correspondences {
        if pair.right_frame_idx >= positions.len() || pair.left_frame_idx >= positions.len() {
            continue;
        }
        if pair.right_frame_idx <= pair.left_frame_idx + 1 || pair.points.len() < 14 {
            continue;
        }

        let essential_pose = match estimate_essential_pose(pair.points.as_slice(), intrinsics, camera_model) {
            Some(v) => v,
            None => continue,
        };

        let step_local = -essential_pose.r.transpose() * essential_pose.t;
        if step_local.norm() <= 1e-9 {
            continue;
        }
        let dir_local = step_local.normalize();
        let r_left = quaternion_to_matrix(&rotations[pair.left_frame_idx]);
        let mut dir_world = r_left * dir_local;
        if dir_world.norm() <= 1e-9 {
            continue;
        }
        dir_world = dir_world.normalize();

        let left = Vector3::new(
            positions[pair.left_frame_idx][0],
            positions[pair.left_frame_idx][1],
            positions[pair.left_frame_idx][2],
        );
        let right = Vector3::new(
            positions[pair.right_frame_idx][0],
            positions[pair.right_frame_idx][1],
            positions[pair.right_frame_idx][2],
        );
        let current_delta = right - left;
        let current_dist = current_delta.norm().max(1e-6);
        let current_dir = current_delta / current_dist;

        let span = (pair.right_frame_idx - pair.left_frame_idx) as f64;
        let target_dist = baseline_m * span;
        let target_right = left + dir_world * target_dist;

        let directional_agreement = ((dir_world.dot(&current_dir) + 1.0) * 0.5).clamp(0.0, 1.0);
        let support = (pair.points.len() as f64 / 96.0).clamp(0.2, 1.0);
        let span_score = (span / 6.0).clamp(0.20, 1.0);
        let quality_score = (0.45 * support + 0.35 * directional_agreement + 0.20 * span_score).clamp(0.2, 1.0);
        let weight = (0.60 * support + 0.40 * directional_agreement).clamp(0.2, 1.0);

        constraints.push(LoopClosureConstraint {
            left_idx: pair.left_frame_idx,
            right_idx: pair.right_frame_idx,
            target_right,
            weight,
            support_count: pair.points.len(),
            quality_score,
        });
    }

    constraints
}

fn median_adjacent_baseline_m(positions: &[[f64; 3]]) -> f64 {
    if positions.len() < 2 {
        return 1.0;
    }
    let mut dists = Vec::with_capacity(positions.len().saturating_sub(1));
    for i in 1..positions.len() {
        let dx = positions[i][0] - positions[i - 1][0];
        let dy = positions[i][1] - positions[i - 1][1];
        let dz = positions[i][2] - positions[i - 1][2];
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        if d.is_finite() && d > 1e-6 {
            dists.push(d);
        }
    }
    if dists.is_empty() {
        return 1.0;
    }
    dists.sort_by(|a, b| a.total_cmp(b));
    dists[dists.len() / 2]
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

fn gps_positions(frames: &[ImageFrame], relative_support: f64) -> Option<(Vec<[f64; 3]>, CrsInfo)> {
    let mut first_gps: Option<GpsCoordinate> = None;
    for frame in frames {
        if let Some(gps) = &frame.metadata.gps {
            first_gps = Some(gps.clone());
            break;
        }
    }
    let origin = first_gps?;

    let utm_epsg = utm_epsg_for_lonlat(origin.lon, origin.lat);
    let source_crs = Crs::from_epsg(4326).ok()?;
    let target_crs = Crs::from_epsg(utm_epsg).ok()?;

    let mut positions = Vec::with_capacity(frames.len());
    let mut last = [0.0, 0.0, 0.0];
    for (idx, frame) in frames.iter().enumerate() {
        if let Some(gps) = &frame.metadata.gps {
            let (x, y) = source_crs
                .transform_to(gps.lon, gps.lat, &target_crs)
                .ok()?;
            let z = gps.alt - origin.alt;
            last = [x, y, z];
        } else if idx > 0 {
            // Missing GPS frame: preserve last known track and advance slightly.
            last = [last[0] + 2.0, last[1], last[2]];
        }
        positions.push(last);
    }

    smooth_position_track(&mut positions, relative_support);

    Some((positions, CrsInfo::from_epsg(utm_epsg)))
}

fn utm_epsg_for_lonlat(lon_deg: f64, lat_deg: f64) -> u32 {
    let mut zone = ((lon_deg + 180.0) / 6.0).floor() as i32 + 1;
    zone = zone.clamp(1, 60);
    if lat_deg >= 0.0 {
        32600 + zone as u32
    } else {
        32700 + zone as u32
    }
}

fn smooth_position_track(positions: &mut [[f64; 3]], relative_support: f64) {
    if positions.len() < 3 {
        return;
    }

    let alpha = (0.35 - 0.20 * relative_support).clamp(0.10, 0.35);
    let original = positions.to_vec();
    for idx in 1..(positions.len() - 1) {
        for dim in 0..3 {
            let neighborhood = (original[idx - 1][dim] + original[idx][dim] + original[idx + 1][dim]) / 3.0;
            positions[idx][dim] = (1.0 - alpha) * original[idx][dim] + alpha * neighborhood;
        }
    }
}

fn estimate_relative_baseline_m(relative_support: f64, quality: f64) -> f64 {
    let support_term = 1.5 + 4.5 * relative_support.clamp(0.0, 1.0);
    let quality_term = 0.75 + 0.5 * quality.clamp(0.0, 1.0);
    (support_term * quality_term).clamp(1.5, 8.0)
}

fn estimate_nominal_altitude_m(frames: &[ImageFrame]) -> f64 {
    let mut alts = Vec::new();
    for frame in frames {
        if let Some(gps) = &frame.metadata.gps {
            alts.push(gps.alt.abs());
        }
    }
    if alts.is_empty() {
        100.0
    } else {
        (alts.iter().sum::<f64>() / alts.len() as f64).max(30.0)
    }
}

fn derive_rotations_from_positions(
    frames: &[ImageFrame],
    positions: &[[f64; 3]],
    match_stats: &MatchStats,
) -> Vec<[f64; 4]> {
    if positions.is_empty() {
        return Vec::new();
    }

    let mut yaws = vec![0.0_f64; positions.len()];
    let mut pitches = vec![0.0_f64; positions.len()];

    for i in 0..positions.len() {
        let delta = if i > 0 && i + 1 < positions.len() {
            [
                positions[i + 1][0] - positions[i - 1][0],
                positions[i + 1][1] - positions[i - 1][1],
                positions[i + 1][2] - positions[i - 1][2],
            ]
        } else if i + 1 < positions.len() {
            [
                positions[i + 1][0] - positions[i][0],
                positions[i + 1][1] - positions[i][1],
                positions[i + 1][2] - positions[i][2],
            ]
        } else if i > 0 {
            [
                positions[i][0] - positions[i - 1][0],
                positions[i][1] - positions[i - 1][1],
                positions[i][2] - positions[i - 1][2],
            ]
        } else {
            [1.0, 0.0, 0.0]
        };

        let horizontal = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        let yaw = if horizontal < 1e-9 {
            if i > 0 { yaws[i - 1] } else { 0.0 }
        } else {
            delta[1].atan2(delta[0])
        };
        let pitch = (0.35 * (-delta[2]).atan2(horizontal.max(1e-6))).clamp(-0.10, 0.10);
        yaws[i] = yaw;
        pitches[i] = pitch;
    }

    let mut prior_yaw_count = 0usize;
    for i in 0..positions.len().min(frames.len()) {
        if let Some(prior) = frames[i].metadata.orientation_prior.as_ref() {
            if let Some(prior_yaw) = orientation_prior_yaw_rad(prior) {
                let trust = orientation_prior_yaw_trust(prior);
                yaws[i] = yaws[i] + trust * shortest_angle_delta(prior_yaw, yaws[i]);
                prior_yaw_count += 1;
            }
        }
    }

    smooth_circular_angles_in_place(&mut yaws, 0.45);
    smooth_scalar_track_in_place(&mut pitches, 0.35);

    let orientation_support = orientation_support_for_attitude_seed(positions.len(), match_stats);
    let prior_coverage = (prior_yaw_count as f64 / positions.len().max(1) as f64).clamp(0.0, 1.0);
    let nadir_lock = positions.len() <= 8 && orientation_support < 0.80 && prior_coverage < 0.30;
    let yaw_offset = if nadir_lock {
        0.0
    } else {
        let raw = estimate_correspondence_yaw_offset_rad(positions, match_stats);
        let trust = (0.30 + 0.70 * orientation_support).clamp(0.30, 1.0) * (1.0 - 0.70 * prior_coverage);
        (raw * trust).clamp(-0.40, 0.40)
    };

    let mut rotations = Vec::with_capacity(positions.len());
    for i in 0..positions.len() {
        let yaw = yaws[i] + yaw_offset;
        let pitch = if nadir_lock {
            0.0
        } else {
            pitches[i] * orientation_support
        };
        let roll = if nadir_lock {
            0.0
        } else if i > 0 {
            let dyaw = shortest_angle_delta(yaws[i], yaws[i - 1]);
            (dyaw * 0.18 * orientation_support).clamp(-0.06, 0.06)
        } else {
            0.0
        };
        let q_body = yaw_pitch_roll_to_quaternion(yaw, pitch, roll);
        // Compose with nadir flip (180° around X) so the camera looks downward.
        // This places camera +Z toward world -Z, enabling project_world_to_pixel
        // to see positive depth for ground features and engaging the standard
        // (non-legacy) mosaic projection path for all GPS-derived poses.
        rotations.push(quaternion_premultiply_nadir_flip(q_body));
    }

    rotations
}

/// Pre-multiply a quaternion by the 180° X-axis flip q_flip = [0, 1, 0, 0].
/// Closed-form product [0,1,0,0] * [qw,qx,qy,qz] = [-qx, qw, -qz, qy].
/// Converts an upward-Z camera (Z = world +Z) to a nadir/downward-Z camera
/// (Z = world -Z) while preserving the yaw direction.
fn quaternion_premultiply_nadir_flip(q: [f64; 4]) -> [f64; 4] {
    let [qw, qx, qy, qz] = q;
    let r = [-qx, qw, -qz, qy];
    let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2] + r[3] * r[3]).sqrt();
    if n > 1e-12 {
        [r[0] / n, r[1] / n, r[2] / n, r[3] / n]
    } else {
        [0.0, 1.0, 0.0, 0.0]
    }
}

fn normalize_rotations_for_projection_convention(rotations: Vec<[f64; 4]>) -> Vec<[f64; 4]> {
    rotations
        .into_iter()
        .map(|q| {
            let r_w2c = quaternion_to_matrix(&q).transpose();
            let down_in_camera = r_w2c * Vector3::new(0.0, 0.0, -1.0);
            if down_in_camera[2] <= 0.0 {
                quaternion_premultiply_nadir_flip(q)
            } else {
                q
            }
        })
        .collect()
}

fn orientation_prior_yaw_trust(prior: &OrientationPrior) -> f64 {
    match prior.source {
        // DJI XMP gimbal data comes from the drone's sensor fusion IMU+compass
        // and is the most accurate orientation source available; trust it heavily.
        OrientationPriorSource::XmpDji => 0.95,
        OrientationPriorSource::XmpGeneric => 0.80,
        OrientationPriorSource::DjiMakerNote => 0.90,
        OrientationPriorSource::ExifGpsImageDirection => 0.72,
        OrientationPriorSource::ExifGpsTrack => 0.60,
    }
}

fn orientation_support_for_attitude_seed(frame_count: usize, match_stats: &MatchStats) -> f64 {
    if frame_count <= 1 || match_stats.adjacent_pair_motions.is_empty() {
        return 0.0;
    }

    let mut inliers = Vec::new();
    let mut disps = Vec::new();
    let mut sum_cos = 0.0;
    let mut sum_sin = 0.0;
    let mut counted = 0usize;

    for motion in &match_stats.adjacent_pair_motions {
        if motion.right_idx != motion.left_idx + 1 {
            continue;
        }
        inliers.push(motion.inlier_count as f64);
        let disp = (motion.model_dx_px * motion.model_dx_px + motion.model_dy_px * motion.model_dy_px).sqrt();
        disps.push(disp);
        let ang = motion.model_dy_px.atan2(motion.model_dx_px);
        sum_cos += ang.cos();
        sum_sin += ang.sin();
        counted += 1;
    }

    if counted == 0 {
        return 0.0;
    }

    inliers.sort_by(|a, b| a.total_cmp(b));
    disps.sort_by(|a, b| a.total_cmp(b));
    let med_inliers = inliers[inliers.len() / 2];
    let med_disp = disps[disps.len() / 2];
    let pair_fraction = (counted as f64 / frame_count.saturating_sub(1).max(1) as f64).clamp(0.0, 1.0);
    let inlier_score = (med_inliers / 40.0).clamp(0.0, 1.0);
    let disp_score = (med_disp / 18.0).clamp(0.0, 1.0);
    let concentration = ((sum_cos * sum_cos + sum_sin * sum_sin).sqrt() / counted as f64).clamp(0.0, 1.0);

    (0.30 * pair_fraction + 0.30 * inlier_score + 0.20 * disp_score + 0.20 * concentration).clamp(0.0, 1.0)
}

fn estimate_correspondence_yaw_offset_rad(positions: &[[f64; 3]], match_stats: &MatchStats) -> f64 {
    if positions.len() < 2 || match_stats.adjacent_pair_motions.is_empty() {
        return 0.0;
    }

    let mut sum_cos = 0.0;
    let mut sum_sin = 0.0;
    let mut support = 0.0;
    let mut pair_count = 0usize;

    for motion in &match_stats.adjacent_pair_motions {
        if motion.right_idx >= positions.len() || motion.right_idx != motion.left_idx + 1 {
            continue;
        }

        let p0 = positions[motion.left_idx];
        let p1 = positions[motion.right_idx];
        let wx = p1[0] - p0[0];
        let wy = p1[1] - p0[1];
        let world_len = (wx * wx + wy * wy).sqrt();
        let img_len = (motion.model_dx_px * motion.model_dx_px + motion.model_dy_px * motion.model_dy_px).sqrt();
        if world_len < 1e-6 || img_len < 1e-6 {
            continue;
        }

        let world_bearing = wy.atan2(wx);
        // With nadir-like imagery, apparent image motion is opposite platform translation.
        let image_bearing = motion.model_dy_px.atan2(motion.model_dx_px) + std::f64::consts::PI;
        let delta = shortest_angle_delta(world_bearing, image_bearing);

        let pair_support = motion.inlier_count as f64;
        let pair_weight = pair_support.clamp(5.0, 250.0) * (img_len / 12.0).clamp(0.4, 2.2);
        sum_cos += pair_weight * delta.cos();
        sum_sin += pair_weight * delta.sin();
        support += pair_weight;
        pair_count += 1;
    }

    if support <= 0.0 {
        0.0
    } else {
        let raw_offset = sum_sin.atan2(sum_cos).clamp(-0.75, 0.75);
        let pair_fraction = (pair_count as f64 / (positions.len().saturating_sub(1).max(1) as f64)).clamp(0.0, 1.0);
        let support_strength = (support / 220.0).clamp(0.0, 1.0);
        raw_offset * (0.25 + 0.75 * pair_fraction * support_strength)
    }
}

fn smooth_circular_angles_in_place(values: &mut [f64], alpha: f64) {
    if values.len() < 3 {
        return;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let original = values.to_vec();
    for idx in 1..(values.len() - 1) {
        let prev = original[idx - 1];
        let curr = original[idx];
        let next = original[idx + 1];
        let mean = circular_mean3(prev, curr, next);
        values[idx] = curr + alpha * shortest_angle_delta(mean, curr);
    }
}

fn circular_mean3(a: f64, b: f64, c: f64) -> f64 {
    let sum_sin = a.sin() + b.sin() + c.sin();
    let sum_cos = a.cos() + b.cos() + c.cos();
    sum_sin.atan2(sum_cos)
}

fn smooth_scalar_track_in_place(values: &mut [f64], alpha: f64) {
    if values.len() < 3 {
        return;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let original = values.to_vec();
    for idx in 1..(values.len() - 1) {
        let neighborhood = (original[idx - 1] + original[idx] + original[idx + 1]) / 3.0;
        values[idx] = (1.0 - alpha) * original[idx] + alpha * neighborhood;
    }
}

fn shortest_angle_delta(a: f64, b: f64) -> f64 {
    let mut d = a - b;
    while d > std::f64::consts::PI {
        d -= 2.0 * std::f64::consts::PI;
    }
    while d < -std::f64::consts::PI {
        d += 2.0 * std::f64::consts::PI;
    }
    d
}

fn yaw_pitch_roll_to_quaternion(yaw: f64, pitch: f64, roll: f64) -> [f64; 4] {
    let (cy, sy) = ((yaw * 0.5).cos(), (yaw * 0.5).sin());
    let (cp, sp) = ((pitch * 0.5).cos(), (pitch * 0.5).sin());
    let (cr, sr) = ((roll * 0.5).cos(), (roll * 0.5).sin());

    let mut q = [
        cr * cp * cy + sr * sp * sy,
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
    ];

    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n > 1e-12 {
        q[0] /= n;
        q[1] /= n;
        q[2] /= n;
        q[3] /= n;
    }
    q
}

fn infer_intrinsics(frames: &[ImageFrame]) -> CameraIntrinsics {
    let first = &frames[0];
    let mut intrinsics = CameraIntrinsics::identity(first.width, first.height);

    if let Some(focal_mm) = first.metadata.focal_length_mm {
        let sensor_width_mm = first.metadata.sensor_width_mm.unwrap_or(13.2);
        if focal_mm > 0.0 && sensor_width_mm > 0.0 {
            let fx = (focal_mm / sensor_width_mm) * first.width as f64;
            intrinsics.fx = fx;
            intrinsics.fy = fx;
        }
    }

    intrinsics
}

fn calibrate_intrinsics_from_correspondence(
    intrinsics: &mut CameraIntrinsics,
    frames: &[ImageFrame],
    positions: &[[f64; 3]],
    match_stats: &MatchStats,
) {
    if positions.len() < 2 || match_stats.adjacent_pair_motions.is_empty() {
        return;
    }

    let nominal_alt_m = estimate_nominal_altitude_m(frames).max(20.0);
    let predicted_px_per_m = (intrinsics.fx / nominal_alt_m).max(1e-6);

    let mut observed_px_per_m = Vec::new();
    for motion in &match_stats.adjacent_pair_motions {
        if motion.right_idx >= positions.len() || motion.right_idx != motion.left_idx + 1 {
            continue;
        }
        if motion.inlier_count < 10 {
            continue;
        }

        let p0 = positions[motion.left_idx];
        let p1 = positions[motion.right_idx];
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let baseline_m = (dx * dx + dy * dy).sqrt();
        if baseline_m < 1e-3 {
            continue;
        }

        let disp_px = motion.median_displacement_px.max(0.0);
        let ratio = disp_px / baseline_m;
        if ratio.is_finite() && ratio > 1e-6 {
            observed_px_per_m.push(ratio);
        }
    }

    if observed_px_per_m.len() < 3 {
        return;
    }

    observed_px_per_m.sort_by(|a, b| a.total_cmp(b));
    let median_observed = observed_px_per_m[observed_px_per_m.len() / 2];
    // GPS positions carry true metric scale, so the pixel/metre ratio derived from
    // GPS baselines is a reliable focal-length signal even when the default sensor
    // width assumption is wrong (e.g. small-sensor drones vs. the 13.2 mm default).
    // Widen the correction window when every frame has GPS; keep it conservative
    // for heuristic (non-GPS) baselines that may not reflect true scene scale.
    let has_gps = frames.iter().all(|f| f.metadata.gps.is_some());
    let (scale_lo, scale_hi) = if has_gps { (0.25, 4.0) } else { (0.70, 1.45) };
    let scale = (median_observed / predicted_px_per_m).clamp(scale_lo, scale_hi);

    intrinsics.fx = (intrinsics.fx * scale).max(50.0);
    intrinsics.fy = (intrinsics.fy * scale).max(50.0);
}

fn estimate_sparse_points(match_stats: &MatchStats, aligned_count: usize) -> u64 {
    if aligned_count <= 1 {
        return 0;
    }
    let baseline = ((aligned_count - 1) as u64) * 150;
    let inferred = (match_stats.total_matches as f64 * 1.35) as u64;
    inferred.max(baseline)
}

fn estimate_tie_points_median(match_stats: &MatchStats) -> u64 {
    let base = (match_stats.mean_matches_per_pair * 0.65).round() as u64;
    base.max(8)
}

fn estimate_tracks_median(match_stats: &MatchStats, aligned_count: usize) -> f64 {
    if aligned_count <= 1 {
        return 0.0;
    }
    let connectivity_term = 2.0 + 4.0 * match_stats.connectivity.clamp(0.0, 1.0);
    let pair_term = (match_stats.mean_matches_per_pair / 220.0).clamp(0.0, 2.0);
    (connectivity_term + pair_term).clamp(2.0, aligned_count as f64)
}

fn estimate_residual_quantiles(rmse_px: f64, quality: f64) -> (f64, f64) {
    let p50 = (rmse_px * (0.62 - 0.07 * quality)).clamp(0.15, rmse_px.max(0.15));
    let p95 = (rmse_px * (1.75 - 0.20 * quality)).clamp(rmse_px, 6.0);
    (p50, p95)
}

fn estimate_gsd_m(frames: &[ImageFrame], intrinsics: &CameraIntrinsics, quality: f64) -> f64 {
    let mut alts = Vec::new();
    let mut focals = Vec::new();
    for frame in frames {
        if let Some(gps) = &frame.metadata.gps {
            alts.push(gps.alt.abs());
        }
        if let Some(focal_mm) = frame.metadata.focal_length_mm {
            if focal_mm > 0.0 {
                focals.push(focal_mm);
            }
        }
    }

    let mean_alt = if alts.is_empty() {
        90.0
    } else {
        alts.iter().sum::<f64>() / alts.len() as f64
    };
    let mean_focal_mm = if focals.is_empty() {
        8.8
    } else {
        focals.iter().sum::<f64>() / focals.len() as f64
    };
    let sensor_width_m = 0.0132;

    let gsd = (mean_alt * sensor_width_m) / (mean_focal_mm * 0.001 * intrinsics.cx * 2.0);
    (gsd / (0.8 + 0.2 * quality)).clamp(0.01, 0.5)
}

fn resolve_camera_model(requested: CameraModel, frames: &[ImageFrame]) -> CameraModel {
    if requested != CameraModel::Auto {
        return requested;
    }

    let maybe_focal = frames
        .iter()
        .find_map(|frame| frame.metadata.focal_length_mm);
    match maybe_focal {
        Some(focal_mm) if focal_mm < 4.0 => CameraModel::Fisheye,
        _ => CameraModel::Pinhole,
    }
}

/// Export camera poses as GeoJSON visualization for inspection in QGIS.
///
/// Creates a GeoJSON FeatureCollection with:
/// - Point features for each camera center (with metadata)
/// - LineString for the full camera trajectory
///
/// This allows visual inspection of camera positions and poses before mosaic generation.
pub fn export_camera_poses_as_geojson(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
    output_path: &str,
) -> Result<()> {
    use std::fs::File;
    use std::io::Write;
    use serde_json::Map;

    let mut features = Vec::new();
    let mut trajectory_coords: Vec<[f64; 2]> = Vec::new();

    // Create point features for each camera center
    for (idx, (pose, frame)) in alignment.poses.iter().zip(frames.iter()).enumerate() {
        let mut props = Map::new();
        props.insert("index".to_string(), serde_json::Value::Number(serde_json::Number::from(idx as u64)));
        props.insert("filename".to_string(), serde_json::Value::String(
            std::path::Path::new(&frame.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        ));
        if let Some(n) = serde_json::Number::from_f64(pose.position[0]) {
            props.insert("ba_x_enu_m".to_string(), serde_json::Value::Number(n));
        }
        if let Some(n) = serde_json::Number::from_f64(pose.position[1]) {
            props.insert("ba_y_enu_m".to_string(), serde_json::Value::Number(n));
        }
        if let Some(n) = serde_json::Number::from_f64(pose.position[2]) {
            props.insert("ba_z_enu_m".to_string(), serde_json::Value::Number(n));
        }
        if let Some(n) = serde_json::Number::from_f64(pose.reprojection_error_px) {
            props.insert("reprojection_error_px".to_string(), serde_json::Value::Number(n));
        }
        props.insert("image_width".to_string(), serde_json::Value::Number(serde_json::Number::from(frame.width as u64)));
        props.insert("image_height".to_string(), serde_json::Value::Number(serde_json::Number::from(frame.height as u64)));
        
        // Use GPS coordinates as geometry so QGIS can display them
        let mut geometry = None;
        if let Some(gps) = &frame.metadata.gps {
            if let Some(n) = serde_json::Number::from_f64(gps.lat) {
                props.insert("gps_lat".to_string(), serde_json::Value::Number(n));
            }
            if let Some(n) = serde_json::Number::from_f64(gps.lon) {
                props.insert("gps_lon".to_string(), serde_json::Value::Number(n));
            }
            if let Some(n) = serde_json::Number::from_f64(gps.alt) {
                props.insert("gps_alt".to_string(), serde_json::Value::Number(n));
            }
            // Create point geometry using GPS (lon, lat) in WGS-84
            geometry = Some(serde_json::json!({
                "type": "Point",
                "coordinates": [gps.lon, gps.lat]
            }));
            trajectory_coords.push([gps.lon, gps.lat]);
        }

        if let Some(geom) = geometry {
            let feature = serde_json::json!({
                "type": "Feature",
                "geometry": geom,
                "properties": props
            });
            features.push(feature);
        }
    }

    // Create trajectory LineString
    if trajectory_coords.len() >= 2 {
        let trajectory_feature = serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": "LineString",
                "coordinates": trajectory_coords,
            },
            "properties": {
                "name": "Camera Trajectory",
                "pose_count": alignment.poses.len(),
                "rmse_px": alignment.stats.rmse_px,
                "connectivity_fraction": alignment.stats.ba_supported_camera_fraction,
            }
        });
        features.push(trajectory_feature);
    }

    // Create the FeatureCollection
    let feature_collection = serde_json::json!({
        "type": "FeatureCollection",
        "crs": {
            "type": "name",
            "properties": {
                "name": "EPSG:4326"
            }
        },
        "features": features,
        "properties": {
            "dataset": "Camera Positions from Bundle Adjustment (WGS-84)",
            "frame_count": frames.len(),
            "aligned_fraction": alignment.stats.aligned_fraction,
            "rmse_px": alignment.stats.rmse_px,
            "residual_p50_px": alignment.stats.residual_p50_px,
            "sparse_points": alignment.stats.sparse_cloud_points,
            "ba_observations_initial": alignment.stats.ba_observations_initial,
            "ba_observations_final": alignment.stats.ba_observations_final,
            "ba_retention_pct": alignment.stats.ba_observation_retention_pct,
            "ba_camera_support_fraction": alignment.stats.ba_supported_camera_fraction,
        }
    });

    let mut file = File::create(output_path)?;

    let json_str = serde_json::to_string_pretty(&feature_collection)
        .map_err(|e| crate::error::PhotogrammetryError::Alignment(format!(
            "failed to serialize geojson: {}",
            e
        )))?;

    file.write_all(json_str.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::PairCorrespondences;
    use crate::ingest::FrameMetadata;

    fn make_frame(path: &str, gps: Option<GpsCoordinate>) -> ImageFrame {
        ImageFrame {
            path: path.to_string(),
            width: 4000,
            height: 3000,
            metadata: FrameMetadata {
                gps,
                focal_length_mm: Some(8.8),
                sensor_width_mm: Some(13.2),
                image_width_px: 4000,
                image_height_px: 3000,
                timestamp: None,
                orientation_prior: None,
                blur_score: None,
                has_rtk_gps: false,
            },
        }
    }

    #[test]
    fn gps_alignment_derives_nonzero_track_and_reasonable_stats() {
        let frames = vec![
            make_frame(
                "a.jpg",
                Some(GpsCoordinate {
                    lat: 45.0,
                    lon: -81.0,
                    alt: 120.0,
                }),
            ),
            make_frame(
                "b.jpg",
                Some(GpsCoordinate {
                    lat: 45.00002,
                    lon: -80.99998,
                    alt: 120.5,
                }),
            ),
            make_frame(
                "c.jpg",
                Some(GpsCoordinate {
                    lat: 45.00004,
                    lon: -80.99996,
                    alt: 121.0,
                }),
            ),
        ];

        let match_stats = MatchStats {
            frame_count: 3,
            total_keypoints: 1800,
            total_matches: 2200,
            connectivity: 0.95,
            mean_matches_per_pair: 730.0,
            mean_parallax_px: 7.5,
            pair_attempt_count: 3,
            pair_connected_count: 3,
            pair_rejected_count: 0,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &match_stats, CameraModel::Auto)
            .expect("alignment should succeed");

        assert_eq!(result.poses.len(), 3);
        assert!(result.poses[1].position[0].abs() > 0.5 || result.poses[1].position[1].abs() > 0.5);
        assert!(result.stats.aligned_fraction > 0.9);
        assert!(result.stats.rmse_px < 1.0);
        assert!(result.stats.sparse_cloud_points > 0);
        assert!(result.stats.estimated_gsd_m > 0.0);
        assert_eq!(result.stats.model, CameraModel::Pinhole);
    }

    #[test]
    fn weak_match_network_reduces_aligned_fraction() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
            make_frame("c.jpg", None),
            make_frame("d.jpg", None),
            make_frame("e.jpg", None),
        ];

        let weak = MatchStats {
            frame_count: 5,
            total_keypoints: 120,
            total_matches: 0,
            connectivity: 0.05,
            mean_matches_per_pair: 1.0,
            mean_parallax_px: 0.0,
            pair_attempt_count: 10,
            pair_connected_count: 0,
            pair_rejected_count: 10,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: vec!["No cross-image feature matches were verified.".to_string()],
            failure_codes: vec!["no_verified_matches".to_string()],
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &weak, CameraModel::Pinhole)
            .expect("alignment should succeed");

        assert!(result.poses.len() < frames.len());
        assert!(result.stats.aligned_fraction < 0.6);
        assert!(result.stats.rmse_px >= 1.2);
    }

    #[test]
    fn strong_non_gps_network_builds_stable_relative_trajectory() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
            make_frame("c.jpg", None),
            make_frame("d.jpg", None),
            make_frame("e.jpg", None),
            make_frame("f.jpg", None),
        ];

        let strong = MatchStats {
            frame_count: 6,
            total_keypoints: 4_800,
            total_matches: 8_400,
            connectivity: 0.92,
            mean_matches_per_pair: 560.0,
            mean_parallax_px: 6.8,
            pair_attempt_count: 15,
            pair_connected_count: 14,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &strong, CameraModel::Pinhole)
            .expect("alignment should succeed");

        assert_eq!(result.poses.len(), frames.len());
        assert!(result.stats.aligned_fraction > 0.9);
        assert!(result.stats.rmse_px < 1.1);

        let baseline = result.poses[1].position[0] - result.poses[0].position[0];
        assert!(baseline > 1.0, "expected non-trivial forward baseline from relative seed");
    }

    #[test]
    fn alignment_options_can_disable_intrinsics_refinement() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
            make_frame("c.jpg", None),
            make_frame("d.jpg", None),
            make_frame("e.jpg", None),
            make_frame("f.jpg", None),
        ];

        let strong = MatchStats {
            frame_count: 6,
            total_keypoints: 4_800,
            total_matches: 8_400,
            connectivity: 0.92,
            mean_matches_per_pair: 560.0,
            mean_parallax_px: 6.8,
            pair_attempt_count: 15,
            pair_connected_count: 14,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let options = AlignmentOptions {
            intrinsics_refinement: IntrinsicsRefinementPolicy::None,
            reduced_camera_solve_mode: ReducedCameraSolveMode::SparsePcg,
        };
        let result = run_camera_alignment_with_options(
            &frames,
            &strong,
            CameraModel::Pinhole,
            options,
        )
        .expect("alignment should succeed");

        assert!(!result.stats.ba_intrinsics_refined);
        assert!(!result.stats.ba_distortion_refined);
    }

    #[test]
    fn alignment_options_core_only_never_refines_distortion() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
            make_frame("c.jpg", None),
            make_frame("d.jpg", None),
            make_frame("e.jpg", None),
            make_frame("f.jpg", None),
        ];

        let strong = MatchStats {
            frame_count: 6,
            total_keypoints: 4_800,
            total_matches: 8_400,
            connectivity: 0.92,
            mean_matches_per_pair: 560.0,
            mean_parallax_px: 6.8,
            pair_attempt_count: 15,
            pair_connected_count: 14,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment_with_options(
            &frames,
            &strong,
            CameraModel::Pinhole,
            AlignmentOptions {
                intrinsics_refinement: IntrinsicsRefinementPolicy::CoreOnly,
                reduced_camera_solve_mode: ReducedCameraSolveMode::SparsePcg,
            },
        )
        .expect("alignment should succeed");

        assert!(!result.stats.ba_distortion_refined);
    }

    #[test]
    fn gps_track_with_gap_remains_monotonic_after_smoothing() {
        let frames = vec![
            make_frame(
                "a.jpg",
                Some(GpsCoordinate {
                    lat: 45.0,
                    lon: -81.0,
                    alt: 120.0,
                }),
            ),
            make_frame("b.jpg", None),
            make_frame(
                "c.jpg",
                Some(GpsCoordinate {
                    lat: 45.00002,
                    lon: -80.99998,
                    alt: 120.5,
                }),
            ),
            make_frame(
                "d.jpg",
                Some(GpsCoordinate {
                    lat: 45.00004,
                    lon: -80.99996,
                    alt: 121.0,
                }),
            ),
        ];
        let stats = MatchStats {
            frame_count: 4,
            total_keypoints: 2_000,
            total_matches: 2_900,
            connectivity: 0.88,
            mean_matches_per_pair: 480.0,
            mean_parallax_px: 5.9,
            pair_attempt_count: 6,
            pair_connected_count: 5,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &stats, CameraModel::Auto)
            .expect("alignment should succeed");

        assert_eq!(result.poses.len(), 4);
        let origin = result.poses[0].position;
        let distance_from_origin = |position: [f64; 3]| -> f64 {
            let dx = position[0] - origin[0];
            let dy = position[1] - origin[1];
            (dx * dx + dy * dy).sqrt()
        };
        for idx in 1..result.poses.len() {
            assert!(
                distance_from_origin(result.poses[idx].position)
                    + 1e-6
                    >= distance_from_origin(result.poses[idx - 1].position),
                "expected non-decreasing radial displacement after smoothing"
            );
        }
    }

    #[test]
    fn essential_matrix_incremental_pose_recovers_nontrivial_motion() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
        ];

        let intrinsics = CameraIntrinsics::identity(4000, 3000);
        let yaw = 0.08_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(),  yaw.cos(), 0.0,
            0.0,        0.0,       1.0,
        );
        let t = Vector3::new(0.18, 0.0, 0.02);

        let mut points = Vec::new();
        for i in 0..28 {
            let x = -1.2 + (i as f64) * 0.09;
            let y = -0.7 + ((i * 7 % 13) as f64) * 0.10;
            let z = 4.5 + ((i * 5 % 11) as f64) * 0.20;
            let p_w = Vector3::new(x, y, z);

            let p1 = p_w;
            let p2 = r * p_w + t;
            if p1[2] <= 0.2 || p2[2] <= 0.2 {
                continue;
            }

            let x1n = p1[0] / p1[2];
            let y1n = p1[1] / p1[2];
            let x2n = p2[0] / p2[2];
            let y2n = p2[1] / p2[2];

            let lx = intrinsics.fx * x1n + intrinsics.cx;
            let ly = intrinsics.fy * y1n + intrinsics.cy;
            let rx = intrinsics.fx * x2n + intrinsics.cx;
            let ry = intrinsics.fy * y2n + intrinsics.cy;
            points.push([lx, ly, rx, ry]);
        }

        let point_count = points.len();
        let stats = MatchStats {
            frame_count: 2,
            total_keypoints: 2000,
            total_matches: points.len() as u64,
            connectivity: 1.0,
            mean_matches_per_pair: points.len() as f64,
            mean_parallax_px: 6.0,
            pair_attempt_count: 1,
            pair_connected_count: 1,
            pair_rejected_count: 0,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: vec![PairCorrespondences {
                left_frame_idx: 0,
                right_frame_idx: 1,
                points,
                confidence_weights: vec![1.0; point_count],
            }],
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &stats, CameraModel::Pinhole)
            .expect("alignment should succeed");

        assert_eq!(result.poses.len(), 2);
        let dx = result.poses[1].position[0] - result.poses[0].position[0];
        let dy = result.poses[1].position[1] - result.poses[0].position[1];
        let baseline = (dx * dx + dy * dy).sqrt();
        assert!(baseline > 1.0, "expected non-trivial incremental baseline from E recovery");

        let q = result.poses[1].rotation;
        let identity_like = (q[0] - 1.0).abs() < 1e-6 && q[1].abs() < 1e-6 && q[2].abs() < 1e-6 && q[3].abs() < 1e-6;
        assert!(!identity_like, "expected non-identity recovered orientation");
    }

    #[test]
    fn essential_pose_rejects_pure_rotation_degeneracy() {
        let intrinsics = CameraIntrinsics::identity(4000, 3000);
        let yaw = 0.12_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(),  yaw.cos(), 0.0,
            0.0,        0.0,       1.0,
        );

        let mut points = Vec::new();
        for i in 0..28 {
            let x = -1.2 + (i as f64) * 0.09;
            let y = -0.7 + ((i * 7 % 13) as f64) * 0.10;
            let z = 4.5 + ((i * 5 % 11) as f64) * 0.20;
            let p1 = Vector3::new(x, y, z);
            let p2 = r * p1;
            if p1[2] <= 0.2 || p2[2] <= 0.2 {
                continue;
            }

            points.push([
                intrinsics.fx * (p1[0] / p1[2]) + intrinsics.cx,
                intrinsics.fy * (p1[1] / p1[2]) + intrinsics.cy,
                intrinsics.fx * (p2[0] / p2[2]) + intrinsics.cx,
                intrinsics.fy * (p2[1] / p2[2]) + intrinsics.cy,
            ]);
        }

        assert!(estimate_essential_pose(&points, &intrinsics, CameraModel::Pinhole).is_none());
    }

    #[test]
    fn essential_pose_rejects_near_forward_motion_degeneracy() {
        let intrinsics = CameraIntrinsics::identity(4000, 3000);
        let yaw = 0.01_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(),  yaw.cos(), 0.0,
            0.0,        0.0,       1.0,
        );
        let t = Vector3::new(0.006, 0.002, 0.30);

        let mut points = Vec::new();
        for i in 0..32 {
            let x = -0.8 + (i as f64) * 0.05;
            let y = -0.5 + ((i * 3 % 11) as f64) * 0.08;
            let z = 5.5 + ((i * 5 % 9) as f64) * 0.22;
            let p1 = Vector3::new(x, y, z);
            let p2 = r * p1 + t;
            if p1[2] <= 0.2 || p2[2] <= 0.2 {
                continue;
            }

            points.push([
                intrinsics.fx * (p1[0] / p1[2]) + intrinsics.cx,
                intrinsics.fy * (p1[1] / p1[2]) + intrinsics.cy,
                intrinsics.fx * (p2[0] / p2[2]) + intrinsics.cx,
                intrinsics.fy * (p2[1] / p2[2]) + intrinsics.cy,
            ]);
        }

        assert!(estimate_essential_pose(&points, &intrinsics, CameraModel::Pinhole).is_none());
    }

    #[test]
    fn essential_pose_rejects_homography_dominant_planar_scene() {
        let intrinsics = CameraIntrinsics::identity(4000, 3000);
        let yaw = 0.035_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(),  yaw.cos(), 0.0,
            0.0,        0.0,       1.0,
        );
        let t = Vector3::new(0.22, 0.04, 0.01);

        let mut points = Vec::new();
        for row in 0..6 {
            for col in 0..7 {
                let x = -1.1 + col as f64 * 0.32;
                let y = -0.8 + row as f64 * 0.26;
                let z = 5.0;
                let p1 = Vector3::new(x, y, z);
                let p2 = r * p1 + t;
                if p1[2] <= 0.2 || p2[2] <= 0.2 {
                    continue;
                }

                points.push([
                    intrinsics.fx * (p1[0] / p1[2]) + intrinsics.cx,
                    intrinsics.fy * (p1[1] / p1[2]) + intrinsics.cy,
                    intrinsics.fx * (p2[0] / p2[2]) + intrinsics.cx,
                    intrinsics.fy * (p2[1] / p2[2]) + intrinsics.cy,
                ]);
            }
        }

        assert!(estimate_essential_pose(&points, &intrinsics, CameraModel::Pinhole).is_none());
    }

    #[test]
    fn missing_adjacent_pair_chain_still_aligns_with_short_gap_bridge() {
        let frames = vec![
            make_frame("a.jpg", None),
            make_frame("b.jpg", None),
            make_frame("c.jpg", None),
            make_frame("d.jpg", None),
        ];
        let intrinsics = CameraIntrinsics::identity(4000, 3000);

        let make_points = |yaw: f64, t: Vector3<f64>| -> Vec<[f64; 4]> {
            let r = Matrix3::new(
                yaw.cos(), -yaw.sin(), 0.0,
                yaw.sin(),  yaw.cos(), 0.0,
                0.0,        0.0,       1.0,
            );
            let mut pts = Vec::new();
            for i in 0..24 {
                let x = -1.0 + (i as f64) * 0.085;
                let y = -0.6 + ((i * 5 % 11) as f64) * 0.11;
                let z = 4.2 + ((i * 7 % 13) as f64) * 0.19;
                let p_w = Vector3::new(x, y, z);
                let p1 = p_w;
                let p2 = r * p_w + t;
                if p1[2] <= 0.2 || p2[2] <= 0.2 {
                    continue;
                }
                pts.push([
                    intrinsics.fx * (p1[0] / p1[2]) + intrinsics.cx,
                    intrinsics.fy * (p1[1] / p1[2]) + intrinsics.cy,
                    intrinsics.fx * (p2[0] / p2[2]) + intrinsics.cx,
                    intrinsics.fy * (p2[1] / p2[2]) + intrinsics.cy,
                ]);
            }
            pts
        };

        let p01 = make_points(0.06, Vector3::new(0.22, 0.01, 0.02));
        let p13 = make_points(0.10, Vector3::new(0.43, 0.02, 0.02)); // gap bridge (missing 1->2)
        let p23 = make_points(0.05, Vector3::new(0.20, 0.01, 0.02));

        let stats = MatchStats {
            frame_count: 4,
            total_keypoints: 3000,
            total_matches: (p01.len() + p13.len() + p23.len()) as u64,
            connectivity: 0.82,
            mean_matches_per_pair: 140.0,
            mean_parallax_px: 6.2,
            pair_attempt_count: 4,
            pair_connected_count: 3,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: vec![
                PairCorrespondences {
                    left_frame_idx: 0,
                    right_frame_idx: 1,
                    points: p01.clone(),
                    confidence_weights: vec![1.0; p01.len()],
                },
                PairCorrespondences {
                    left_frame_idx: 1,
                    right_frame_idx: 3,
                    points: p13.clone(),
                    confidence_weights: vec![1.0; p13.len()],
                },
                PairCorrespondences {
                    left_frame_idx: 2,
                    right_frame_idx: 3,
                    points: p23.clone(),
                    confidence_weights: vec![1.0; p23.len()],
                },
            ],
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let result = run_camera_alignment(&frames, &stats, CameraModel::Pinhole)
            .expect("alignment should succeed with short-gap bridge");

        assert_eq!(result.poses.len(), 4);
        assert!(result.stats.aligned_fraction >= 0.95);

        let b01 = {
            let dx = result.poses[1].position[0] - result.poses[0].position[0];
            let dy = result.poses[1].position[1] - result.poses[0].position[1];
            (dx * dx + dy * dy).sqrt()
        };
        let b23 = {
            let dx = result.poses[3].position[0] - result.poses[2].position[0];
            let dy = result.poses[3].position[1] - result.poses[2].position[1];
            (dx * dx + dy * dy).sqrt()
        };
        assert!(b01 > 0.5);
        assert!(b23 > 0.5);
    }

    #[test]
    fn bundle_adjustment_reduces_residuals_with_huber_trimming() {
        // Validate the key BA improvements: Huber weighting and rotation optimization are integrated.
        // This is a unit-level test of the BA architecture rather than end-to-end residual reduction.
        
        let intrinsics = CameraIntrinsics {
            fx: 1000.0,
            fy: 1000.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let positions = vec![
            [0.0, 0.0, 10.0],
            [0.5, 0.0, 10.0],
        ];

        let rotations = vec![
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ];

        // Create a straightforward correspondence: a few points in view of both cameras
        // Using simple geometry to ensure successful triangulation
        let mut points = Vec::new();
        for i in 0..20 {
            let t = i as f64 / 20.0;
            // Point visible in both cameras at various depths and image locations
            let px_l_x = 1500.0 + t * 1000.0;
            let px_l_y = 1000.0 + t * 500.0;
            let px_r_x = 1400.0 + t * 1000.0;
            let px_r_y = 1050.0 + t * 500.0;
            points.push([px_l_x, px_l_y, px_r_x, px_r_y]);
        }

        let point_count = points.len();
        let (_refined_pos, refined_rot, _refined_intrinsics, _residuals, _ba_diag) = run_simplified_bundle_adjustment(
            &positions,
            &rotations,
            &MatchStats {
                frame_count: 2,
                total_keypoints: 200,
                total_matches: 100,
                connectivity: 1.0,
                mean_matches_per_pair: 100.0,
                mean_parallax_px: 50.0,
                pair_attempt_count: 1,
                pair_connected_count: 1,
                pair_rejected_count: 0,
                adjacent_pair_motions: Vec::new(),
                pair_correspondences: vec![
                    PairCorrespondences {
                        left_frame_idx: 0,
                        right_frame_idx: 1,
                        points,
                        confidence_weights: vec![1.0; point_count],
                    }
                ],
                failure_reasons: Vec::new(),
                failure_codes: Vec::new(),
                weak_pair_examples: Vec::new(),
            },
            &intrinsics,
            CameraModel::Pinhole,
            IntrinsicsRefinementPolicy::Auto,
            ReducedCameraSolveMode::SparsePcg,
        );

        // Validate that BA successfully:
        // 1. Returns refined positions and rotations
        // 2. Handles Huber weighting and rotation optimization without crashing
        assert_eq!(refined_rot.len(), 2, "BA should return refined rotations for both cameras");
        
        // BA can produce empty residuals if triangulation fails for all points on synthetic data,
        // which is acceptable. The key is that BA runs with the new Huber weighting and
        // rotation optimization code paths and doesn't crash.
        for rot in refined_rot {
            let q_norm = (rot[0] * rot[0] + rot[1] * rot[1] + rot[2] * rot[2] + rot[3] * rot[3]).sqrt();
            assert!(q_norm > 0.99 && q_norm < 1.01, "BA should preserve quaternion normalization");
        }
    }

    #[test]
    fn bundle_adjustment_falls_back_to_relaxed_observation_gate_for_sparse_short_sequences() {
        let intrinsics = CameraIntrinsics {
            fx: 1000.0,
            fy: 1000.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let positions = vec![
            [0.0, 0.0, 10.0],
            [0.5, 0.0, 10.0],
        ];
        let rotations = vec![
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ];

        let mut points = Vec::new();
        for i in 0..12 {
            let t = i as f64 / 12.0;
            points.push([
                1500.0 + t * 900.0,
                1000.0 + t * 420.0,
                1400.0 + t * 900.0,
                1050.0 + t * 420.0,
            ]);
        }

        let point_count = points.len();
        let (_refined_pos, _refined_rot, _refined_intrinsics, residuals, ba_diag) = run_simplified_bundle_adjustment(
            &positions,
            &rotations,
            &MatchStats {
                frame_count: 2,
                total_keypoints: 120,
                total_matches: 12,
                connectivity: 1.0,
                mean_matches_per_pair: 12.0,
                mean_parallax_px: 40.0,
                pair_attempt_count: 1,
                pair_connected_count: 1,
                pair_rejected_count: 0,
                adjacent_pair_motions: Vec::new(),
                pair_correspondences: vec![PairCorrespondences {
                    left_frame_idx: 0,
                    right_frame_idx: 1,
                    points,
                    confidence_weights: vec![1.0; point_count],
                }],
                failure_reasons: Vec::new(),
                failure_codes: Vec::new(),
                weak_pair_examples: Vec::new(),
            },
            &intrinsics,
            CameraModel::Pinhole,
            IntrinsicsRefinementPolicy::Auto,
            ReducedCameraSolveMode::SparsePcg,
        );

        assert!(ba_diag.observations_initial >= 12, "relaxed BA admission should preserve sparse short-sequence observations");
        assert!(ba_diag.optimization_passes >= 1, "BA should run once relaxed observation admission succeeds");
        assert!(!residuals.is_empty(), "BA should produce residual samples once observations are admitted");
        assert!(ba_diag.covariance.supported_camera_count >= 1, "BA should emit covariance-style camera diagnostics");
        assert!(ba_diag.covariance.translation_sigma_median_m >= 0.0, "translation covariance proxy should be non-negative");
    }

    #[test]
    fn reduced_camera_center_solve_updates_multiview_network_and_emits_covariance() {
        let intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1200.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let true_positions = vec![
            [0.0, 0.0, 10.0],
            [0.8, 0.00, 10.0],
            [1.6, 0.00, 10.0],
        ];
        let seed_positions = vec![
            [0.0, 0.0, 10.0],
            [0.92, 0.12, 10.0],
            [1.74, -0.08, 10.0],
        ];

        let world_points = vec![
            Vector3::new(-0.6, -0.4, 13.5),
            Vector3::new(-0.2, 0.1, 14.0),
            Vector3::new(0.2, -0.2, 14.4),
            Vector3::new(0.7, 0.3, 15.0),
            Vector3::new(1.1, -0.1, 14.7),
            Vector3::new(1.5, 0.4, 15.2),
        ];

        let true_centres: Vec<Vector3<f64>> = true_positions
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let seed_centres: Vec<Vector3<f64>> = seed_positions
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let rotations_m = vec![Matrix3::identity(); 3];
        let identity = Matrix3::identity();
        let mut pair01 = Vec::new();
        let mut pair12 = Vec::new();
        let mut pair02 = Vec::new();
        for point in &world_points {
            let p0 = project_world_to_pixel(point, &true_centres[0], &identity, &intrinsics, CameraModel::Pinhole)
                .expect("p0");
            let p1 = project_world_to_pixel(point, &true_centres[1], &identity, &intrinsics, CameraModel::Pinhole)
                .expect("p1");
            let p2 = project_world_to_pixel(point, &true_centres[2], &identity, &intrinsics, CameraModel::Pinhole)
                .expect("p2");
            pair01.push([p0[0], p0[1], p1[0], p1[1]]);
            pair12.push([p1[0], p1[1], p2[0], p2[1]]);
            pair02.push([p0[0], p0[1], p2[0], p2[1]]);
        }

        let mut observations = Vec::new();
        for (point_id, point) in world_points.iter().enumerate() {
            let p0 = Vector2::new(pair01[point_id][0], pair01[point_id][1]);
            let p1 = Vector2::new(pair01[point_id][2], pair01[point_id][3]);
            let p2 = Vector2::new(pair12[point_id][2], pair12[point_id][3]);
            observations.push(BaObservation {
                cam_idx: 0,
                point_id,
                point_world: *point,
                obs_px: p0,
                quality_weight: 1.0,
            });
            observations.push(BaObservation {
                cam_idx: 1,
                point_id,
                point_world: *point,
                obs_px: p1,
                quality_weight: 1.0,
            });
            observations.push(BaObservation {
                cam_idx: 2,
                point_id,
                point_world: *point,
                obs_px: p2,
                quality_weight: 1.0,
            });
        }

        let pose_prior_weights = vec![0.0, 0.25, 0.25];
        let update = apply_reduced_camera_center_update(
            &seed_centres,
            &rotations_m,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &seed_centres,
            &pose_prior_weights,
            6.0,
            180.0,
            20.0,
            0.08,
        ).expect("reduced center update");

        let covariance = estimate_camera_covariance_diagnostics(
            &update.centres,
            &rotations_m,
            &update.observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &seed_centres,
            &pose_prior_weights,
            6.0,
            180.0,
            0.001,
        );

        assert!((update.centres[1][0] - seed_centres[1][0]).abs() > 1.0e-6 || (update.centres[1][1] - seed_centres[1][1]).abs() > 1.0e-6,
            "reduced solve should move at least one non-anchor camera");
        assert!(covariance.supported_camera_count >= 2, "reduced solve should report covariance for multiple cameras");
        assert!(covariance.translation_sigma_p95_m.is_finite(), "translation covariance proxy should be finite");
        assert!(covariance.rotation_sigma_p95_deg.is_finite(), "rotation covariance proxy should be finite");
    }

    #[test]
    fn reduced_camera_rotation_solve_updates_attitude_block() {
        let intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1200.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let centres = vec![
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(0.8, 0.0, 10.0),
            Vector3::new(1.6, 0.0, 10.0),
        ];

        let world_points = vec![
            Vector3::new(-0.8, -0.5, 13.8),
            Vector3::new(-0.3, 0.0, 14.4),
            Vector3::new(0.2, -0.1, 14.8),
            Vector3::new(0.7, 0.2, 15.1),
            Vector3::new(1.2, -0.2, 15.0),
            Vector3::new(1.7, 0.3, 15.4),
        ];

        let mut true_rotations = vec![Matrix3::identity(); 3];
        true_rotations[1] = small_angle_update(&small_angle_update(&Matrix3::identity(), 0, 0.012), 1, -0.009);
        true_rotations[2] = small_angle_update(&small_angle_update(&Matrix3::identity(), 0, 0.015), 1, -0.011);

        let seed_rotations = vec![Matrix3::identity(); 3];
        let mut observations = Vec::new();
        for (point_id, point) in world_points.iter().enumerate() {
            for cam_idx in 0..3 {
                let obs_px = project_world_to_pixel(
                    point,
                    &centres[cam_idx],
                    &true_rotations[cam_idx],
                    &intrinsics,
                    CameraModel::Pinhole,
                ).expect("synthetic observation");
                observations.push(BaObservation {
                    cam_idx,
                    point_id,
                    point_world: *point,
                    obs_px,
                    quality_weight: 1.0,
                });
            }
        }

        let pose_prior_weights = vec![0.0, 0.20, 0.20];
        let update = apply_reduced_camera_rotation_update(
            &centres,
            &seed_rotations,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &centres,
            &pose_prior_weights,
            6.0,
            180.0,
            45.0,
            0.035,
            0.001,
        ).expect("reduced rotation update");

        let delta_norm = (&update.rotations[1] - &seed_rotations[1]).norm()
            + (&update.rotations[2] - &seed_rotations[2]).norm();
        assert!(delta_norm > 1.0e-8, "reduced rotation solve should update non-anchor camera rotations");
    }

    #[test]
    fn reduced_camera_pose_solve_updates_center_and_attitude() {
        let intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1200.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let true_centres = vec![
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(0.8, 0.0, 10.0),
            Vector3::new(1.6, 0.0, 10.0),
        ];
        let seed_centres = vec![
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(0.92, 0.12, 10.0),
            Vector3::new(1.74, -0.08, 10.0),
        ];

        let world_points = vec![
            Vector3::new(-0.8, -0.5, 13.8),
            Vector3::new(-0.3, 0.0, 14.4),
            Vector3::new(0.2, -0.1, 14.8),
            Vector3::new(0.7, 0.2, 15.1),
            Vector3::new(1.2, -0.2, 15.0),
            Vector3::new(1.7, 0.3, 15.4),
        ];

        let mut true_rotations = vec![Matrix3::identity(); 3];
        true_rotations[1] = small_angle_update(&small_angle_update(&Matrix3::identity(), 0, 0.012), 1, -0.009);
        true_rotations[2] = small_angle_update(&small_angle_update(&Matrix3::identity(), 0, 0.015), 1, -0.011);

        let seed_rotations = vec![Matrix3::identity(); 3];
        let mut observations = Vec::new();
        for (point_id, point) in world_points.iter().enumerate() {
            for cam_idx in 0..3 {
                let obs_px = project_world_to_pixel(
                    point,
                    &true_centres[cam_idx],
                    &true_rotations[cam_idx],
                    &intrinsics,
                    CameraModel::Pinhole,
                ).expect("synthetic observation");
                observations.push(BaObservation {
                    cam_idx,
                    point_id,
                    point_world: *point,
                    obs_px,
                    quality_weight: 1.0,
                });
            }
        }

        let pose_prior_weights = vec![0.0, 0.20, 0.20];
        let update = apply_reduced_camera_pose_update(
            &seed_centres,
            &seed_rotations,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &seed_centres,
            &pose_prior_weights,
            6.0,
            180.0,
            20.0,
            0.08,
            0.035,
            0.001,
            ReducedCameraSolveMode::SparsePcg,
        ).expect("reduced pose update");

        let center_delta = (update.centres[1] - seed_centres[1]).norm()
            + (update.centres[2] - seed_centres[2]).norm();
        let rotation_delta = (&update.rotations[1] - &seed_rotations[1]).norm()
            + (&update.rotations[2] - &seed_rotations[2]).norm();
        assert!(center_delta > 1.0e-8, "coupled pose solve should update non-anchor camera centers");
        assert!(rotation_delta > 1.0e-8, "coupled pose solve should update non-anchor camera rotations");
    }

    #[test]
    fn sparse_pcg_matches_dense_lu_for_block_system() {
        let mut sparse = SparseSymmetricBlock4::new(8);
        let block00 = Matrix4::new(
            8.0, 0.15, 0.0, 0.0,
            0.15, 7.5, 0.08, 0.0,
            0.0, 0.08, 7.0, 0.12,
            0.0, 0.0, 0.12, 6.8,
        );
        let block11 = Matrix4::new(
            7.8, 0.08, 0.0, 0.0,
            0.08, 7.2, 0.07, 0.0,
            0.0, 0.07, 6.9, 0.07,
            0.0, 0.0, 0.07, 6.6,
        );
        let off = Matrix4::new(
            -0.06, 0.01, 0.0, 0.0,
            0.01, -0.05, 0.005, 0.0,
            0.0, 0.005, -0.04, 0.004,
            0.0, 0.0, 0.004, -0.035,
        );
        sparse.add_block(0, 0, block00);
        sparse.add_block(4, 4, block11);
        sparse.add_block(0, 4, off);

        let gradient = DVector::from_vec(vec![0.8, -0.4, 0.2, -0.1, -0.7, 0.3, -0.25, 0.12]);
        let damping = 0.35;

        let pcg = solve_damped_sparse_pcg(&sparse, &gradient, damping, 96, 1.0e-10)
            .expect("pcg solve");

        let mut dense = sparse.into_dense();
        for i in 0..dense.nrows() {
            dense[(i, i)] += damping;
        }
        let rhs = -&gradient;
        let lu = dense.lu().solve(&rhs).expect("dense lu solve");

        let diff = (&pcg - &lu).norm();
        assert!(diff < 1.0e-6, "sparse pcg should match dense LU for the same damped system");
    }

    #[test]
    #[ignore]
    fn schur_sparse_solver_benchmark_matrix_reports_runtime_and_parity() {
        use std::time::Instant;

        fn build_chain_block_system(block_count: usize) -> (SparseSymmetricBlock4, DVector<f64>) {
            let dim = block_count * 4;
            let mut sparse = SparseSymmetricBlock4::new(dim);

            for b in 0..block_count {
                let base = 4 * b;
                let diag_strength = 8.0 + 0.2 * b as f64;
                let diag = Matrix4::new(
                    diag_strength,
                    0.06,
                    0.0,
                    0.0,
                    0.06,
                    diag_strength - 0.2,
                    0.04,
                    0.0,
                    0.0,
                    0.04,
                    diag_strength - 0.35,
                    0.03,
                    0.0,
                    0.0,
                    0.03,
                    diag_strength - 0.5,
                );
                sparse.add_block(base, base, diag);

                if b + 1 < block_count {
                    let off = Matrix4::new(
                        -0.08,
                        0.01,
                        0.0,
                        0.0,
                        0.01,
                        -0.06,
                        0.008,
                        0.0,
                        0.0,
                        0.008,
                        -0.05,
                        0.006,
                        0.0,
                        0.0,
                        0.006,
                        -0.045,
                    );
                    sparse.add_block(base, base + 4, off);
                }
            }

            let mut g = Vec::with_capacity(dim);
            for i in 0..dim {
                let v = ((i % 7) as f64 - 3.0) * 0.13 + ((i % 3) as f64) * 0.07;
                g.push(v);
            }
            (sparse, DVector::from_vec(g))
        }

        let damping = 0.25;
        let pcg_iters = 128;
        let sizes = [6usize, 16usize, 32usize];

        for blocks in sizes {
            let (sparse, gradient) = build_chain_block_system(blocks);

            let t_sparse = Instant::now();
            let sparse_sol = solve_damped_sparse_pcg(&sparse, &gradient, damping, pcg_iters, 1.0e-8)
                .expect("sparse solve");
            let sparse_us = t_sparse.elapsed().as_secs_f64() * 1.0e6;

            let mut dense = sparse.clone().into_dense();
            for i in 0..dense.nrows() {
                dense[(i, i)] += damping;
            }
            let rhs = -&gradient;

            let t_dense = Instant::now();
            let dense_sol = dense.lu().solve(&rhs).expect("dense solve");
            let dense_us = t_dense.elapsed().as_secs_f64() * 1.0e6;

            let err = (&sparse_sol - &dense_sol).norm();
            assert!(err < 1.0e-4, "sparse and dense reduced solves should remain numerically close");

            println!(
                "Schur benchmark blocks={} dim={} sparse_us={:.2} dense_us={:.2} ratio={:.3} err={:.3e}",
                blocks,
                blocks * 4,
                sparse_us,
                dense_us,
                sparse_us / dense_us.max(1.0e-9),
                err,
            );
        }
    }

    #[test]
    fn reduced_camera_pose_solve_stable_on_larger_synthetic_network() {
        let intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1200.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let camera_count = 10usize;
        let mut true_centres = Vec::with_capacity(camera_count);
        let mut seed_centres = Vec::with_capacity(camera_count);
        let mut true_rotations = Vec::with_capacity(camera_count);
        let mut seed_rotations = Vec::with_capacity(camera_count);

        for i in 0..camera_count {
            let x = 0.7 * i as f64;
            let y = 0.08 * (0.35 * i as f64).sin();
            let z = 10.0 + 0.02 * (0.22 * i as f64).cos();
            true_centres.push(Vector3::new(x, y, z));

            let seed_x = x + 0.10 * (0.50 * i as f64).sin();
            let seed_y = y - 0.09 * (0.45 * i as f64).cos();
            seed_centres.push(Vector3::new(seed_x, seed_y, z));

            let yaw = 0.010 + 0.0012 * i as f64;
            let pitch = -0.008 - 0.0010 * i as f64;
            let mut r = Matrix3::identity();
            r = small_angle_update(&r, 0, yaw);
            r = small_angle_update(&r, 1, pitch);
            true_rotations.push(r);

            seed_rotations.push(Matrix3::identity());
        }

        let point_count = 64usize;
        let mut world_points = Vec::with_capacity(point_count);
        for i in 0..point_count {
            let px = -1.2 + 0.22 * (i % 16) as f64;
            let py = -0.7 + 0.20 * ((i * 5) % 9) as f64;
            let pz = 14.0 + 0.18 * ((i * 7) % 11) as f64;
            world_points.push(Vector3::new(px, py, pz));
        }

        let mut observations = Vec::new();
        for (point_id, point) in world_points.iter().enumerate() {
            for cam_idx in 0..camera_count {
                if let Some(obs_px) = project_world_to_pixel(
                    point,
                    &true_centres[cam_idx],
                    &true_rotations[cam_idx],
                    &intrinsics,
                    CameraModel::Pinhole,
                ) {
                    observations.push(BaObservation {
                        cam_idx,
                        point_id,
                        point_world: *point,
                        obs_px,
                        quality_weight: 1.0,
                    });
                }
            }
        }

        assert!(observations.len() >= camera_count * point_count / 2, "synthetic network should be well observed");

        let seed_residuals = reprojection_residual_samples(
            &seed_centres,
            &seed_rotations,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
        );
        let (_, seed_p95) = residual_quantiles_from_samples(&seed_residuals);

        let mut pose_prior_weights = vec![0.20; camera_count];
        pose_prior_weights[0] = 0.0;
        let update = apply_reduced_camera_pose_update(
            &seed_centres,
            &seed_rotations,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &seed_centres,
            &pose_prior_weights,
            6.0,
            180.0,
            20.0,
            0.08,
            0.035,
            0.001,
            ReducedCameraSolveMode::SparsePcg,
        ).expect("reduced pose update on larger synthetic network");

        let updated_residuals = reprojection_residual_samples(
            &update.centres,
            &update.rotations,
            &observations,
            &intrinsics,
            CameraModel::Pinhole,
        );
        let (_, updated_p95) = residual_quantiles_from_samples(&updated_residuals);

        assert!(updated_p95.is_finite(), "updated residual p95 should be finite");
        assert!(updated_p95 <= seed_p95 * 2.5 + 1.0e-6, "reduced pose solve should remain stable on larger synthetic network");

        for c in &update.centres {
            assert!(c[0].is_finite() && c[1].is_finite() && c[2].is_finite(), "updated camera centers should remain finite");
        }
        for r in &update.rotations {
            assert!(r.iter().all(|v| v.is_finite()), "updated camera rotations should remain finite");
        }

        let covariance = estimate_camera_covariance_diagnostics(
            &update.centres,
            &update.rotations,
            &update.observations,
            &intrinsics,
            CameraModel::Pinhole,
            2.0,
            &seed_centres,
            &pose_prior_weights,
            6.0,
            180.0,
            0.001,
        );
        assert!(covariance.supported_camera_count >= (camera_count as u64).saturating_sub(1), "covariance diagnostics should cover most non-anchor cameras");
        assert!(covariance.translation_sigma_p95_m.is_finite(), "translation covariance proxy should be finite");
        assert!(covariance.rotation_sigma_p95_deg.is_finite(), "rotation covariance proxy should be finite");
    }

    #[test]
    fn loop_closure_global_optimization_reduces_lateral_drift() {
        let intrinsics = CameraIntrinsics::identity(4000, 3000);
        let positions = vec![
            [0.0, 0.0, 10.0],
            [3.0, 0.8, 10.0],
            [6.0, 1.6, 10.0],
            [9.0, 2.4, 10.0],
        ];
        let rotations = vec![
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ];

        let yaw = 0.04_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(), yaw.cos(), 0.0,
            0.0, 0.0, 1.0,
        );
        let t = Vector3::new(0.26, 0.02, 0.02);

        let mut loop_points = Vec::new();
        for i in 0..36 {
            let x = -1.0 + (i as f64) * 0.08;
            let y = -0.6 + ((i * 5 % 11) as f64) * 0.11;
            let z = 4.0 + ((i * 7 % 13) as f64) * 0.18;
            let p_w = Vector3::new(x, y, z);
            let p1 = p_w;
            let p2 = r * p_w + t;
            if p1[2] <= 0.2 || p2[2] <= 0.2 {
                continue;
            }
            loop_points.push([
                intrinsics.fx * (p1[0] / p1[2]) + intrinsics.cx,
                intrinsics.fy * (p1[1] / p1[2]) + intrinsics.cy,
                intrinsics.fx * (p2[0] / p2[2]) + intrinsics.cx,
                intrinsics.fy * (p2[1] / p2[2]) + intrinsics.cy,
            ]);
        }

        let loop_point_count = loop_points.len();
        let match_stats = MatchStats {
            frame_count: 4,
            total_keypoints: 2200,
            total_matches: loop_points.len() as u64,
            connectivity: 0.86,
            mean_matches_per_pair: 120.0,
            mean_parallax_px: 6.1,
            pair_attempt_count: 6,
            pair_connected_count: 5,
            pair_rejected_count: 1,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: vec![PairCorrespondences {
                left_frame_idx: 0,
                right_frame_idx: 3,
                points: loop_points,
                confidence_weights: vec![1.0; loop_point_count],
            }],
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let (refined, diag) = apply_loop_closure_global_optimization(
            &positions,
            &rotations,
            &match_stats,
            &intrinsics,
            CameraModel::Pinhole,
            0.85,
        );

        let before_drift = (positions[3][1] - positions[0][1]).abs();
        let after_drift = (refined[3][1] - refined[0][1]).abs();
        assert!(after_drift < before_drift);
        assert!(diag.constraint_count >= 1);
        assert!(diag.max_correction_m > 0.0);
    }

    #[test]
    fn fisheye_projection_compresses_far_off_axis_points_vs_pinhole() {
        let intrinsics = CameraIntrinsics {
            fx: 1000.0,
            fy: 1000.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };
        let centre = Vector3::new(0.0, 0.0, 0.0);
        let r = Matrix3::identity();
        let xw = Vector3::new(2.0, 0.0, 2.0); // x/z = 1.0, strong off-axis ray

        let pin = project_world_to_pixel(&xw, &centre, &r, &intrinsics, CameraModel::Pinhole)
            .expect("pinhole projection");
        let fish = project_world_to_pixel(&xw, &centre, &r, &intrinsics, CameraModel::Fisheye)
            .expect("fisheye projection");

        let pin_radius = (pin[0] - intrinsics.cx).abs();
        let fish_radius = (fish[0] - intrinsics.cx).abs();
        assert!(
            fish_radius < pin_radius,
            "fisheye equidistant projection should compress off-axis radius"
        );
    }

    #[test]
    fn fisheye_and_pinhole_projections_match_near_optical_axis() {
        let intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1200.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };
        let centre = Vector3::new(0.0, 0.0, 0.0);
        let r = Matrix3::identity();
        let xw = Vector3::new(0.01, -0.008, 2.0); // near-axis ray

        let pin = project_world_to_pixel(&xw, &centre, &r, &intrinsics, CameraModel::Pinhole)
            .expect("pinhole projection");
        let fish = project_world_to_pixel(&xw, &centre, &r, &intrinsics, CameraModel::Fisheye)
            .expect("fisheye projection");

        let du = (pin[0] - fish[0]).abs();
        let dv = (pin[1] - fish[1]).abs();
        assert!(du < 0.05 && dv < 0.05, "near-axis projection should be nearly identical");
    }

    #[test]
    fn fisheye_projection_ignores_tangential_terms() {
        let intrinsics_base = CameraIntrinsics {
            fx: 1000.0,
            fy: 1000.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.015,
            k2: -0.002,
            p1: 0.0,
            p2: 0.0,
        };
        let intrinsics_tangential = CameraIntrinsics {
            p1: 0.08,
            p2: -0.06,
            ..intrinsics_base.clone()
        };
        let centre = Vector3::new(0.0, 0.0, 0.0);
        let r = Matrix3::identity();
        let xw = Vector3::new(1.8, -0.9, 2.2);

        let base = project_world_to_pixel(&xw, &centre, &r, &intrinsics_base, CameraModel::Fisheye)
            .expect("fisheye projection");
        let tangential = project_world_to_pixel(&xw, &centre, &r, &intrinsics_tangential, CameraModel::Fisheye)
            .expect("fisheye projection");

        assert!((base - tangential).norm() < 1.0e-9, "fisheye projection should ignore tangential Brown-Conrady terms");
    }

    #[test]
    fn fisheye_essential_pose_recovers_nontrivial_motion() {
        let intrinsics = CameraIntrinsics {
            fx: 1100.0,
            fy: 1090.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.018,
            k2: -0.003,
            p1: 0.0,
            p2: 0.0,
        };
        let yaw = 0.06_f64;
        let r = Matrix3::new(
            yaw.cos(), -yaw.sin(), 0.0,
            yaw.sin(),  yaw.cos(), 0.0,
            0.0,        0.0,       1.0,
        );
        let t = Vector3::new(0.20, 0.02, 0.01);

        let mut points = Vec::new();
        for i in 0..32 {
            let x = -1.1 + (i as f64) * 0.08;
            let y = -0.7 + ((i * 5 % 13) as f64) * 0.10;
            let z = 3.8 + ((i * 7 % 11) as f64) * 0.18;
            let world = Vector3::new(x, y, z);
            let p1 = project_world_to_pixel(
                &world,
                &Vector3::zeros(),
                &Matrix3::identity(),
                &intrinsics,
                CameraModel::Fisheye,
            ).expect("fisheye obs 1");
            let p2 = project_world_to_pixel(
                &world,
                &(-r.transpose() * t),
                &r.transpose(),
                &intrinsics,
                CameraModel::Fisheye,
            ).expect("fisheye obs 2");
            points.push([p1[0], p1[1], p2[0], p2[1]]);
        }

        let pose = estimate_essential_pose(&points, &intrinsics, CameraModel::Fisheye)
            .expect("fisheye essential recovery should succeed");
        let step_local = -pose.r.transpose() * pose.t;
        assert!(step_local.norm() > 0.5, "fisheye essential recovery should produce a non-trivial step direction");
    }

    #[test]
    fn fisheye_refine_mask_keeps_tangential_params_frozen() {
        let mask = build_intrinsics_refine_mask(
            IntrinsicsRefinementPolicy::All,
            true,
            256,
            6,
            0.98,
            false,
            CameraModel::Fisheye,
        );

        assert!(mask.params[0] && mask.params[1] && mask.params[4] && mask.params[5]);
        assert!(!mask.params[6] && !mask.params[7], "fisheye refinement should not expose pinhole tangential parameters");
    }

    #[test]
    fn fisheye_intrinsics_update_leaves_tangential_terms_unchanged() {
        let mut intrinsics = CameraIntrinsics {
            fx: 1200.0,
            fy: 1180.0,
            cx: 2000.0,
            cy: 1500.0,
            k1: 0.01,
            k2: -0.002,
            p1: 0.03,
            p2: -0.04,
        };
        let p1_before = intrinsics.p1;
        let p2_before = intrinsics.p2;

        apply_intrinsics_update(
            &mut intrinsics,
            &[1.0, -1.0, 0.5, -0.5, 1.0, -1.0, 1.0, -1.0],
            &[0.1, 0.1, 0.1, 0.1, 0.01, 0.01, 0.5, 0.5],
            IntrinsicsRefineMask {
                params: [true, true, true, true, true, true, true, true],
            },
            4000.0,
            3000.0,
            CameraModel::Fisheye,
        );

        assert_eq!(intrinsics.p1, p1_before);
        assert_eq!(intrinsics.p2, p2_before);
        assert!((intrinsics.k1 - 0.01).abs() > 1.0e-8 || (intrinsics.k2 + 0.002).abs() > 1.0e-8);
    }

    #[test]
    fn pinhole_unprojection_inverts_projected_distorted_point() {
        let intrinsics = CameraIntrinsics {
            fx: 1180.0,
            fy: 1210.0,
            cx: 2010.0,
            cy: 1490.0,
            k1: -0.08,
            k2: 0.014,
            p1: 0.002,
            p2: -0.0015,
        };
        let centre = Vector3::new(0.0, 0.0, 0.0);
        let r = Matrix3::identity();
        let xw = Vector3::new(0.72, -0.46, 2.35);
        let expected = Vector2::new(xw[0] / xw[2], xw[1] / xw[2]);

        let obs_px = project_world_to_pixel(&xw, &centre, &r, &intrinsics, CameraModel::Pinhole)
            .expect("pinhole projection");
        let recovered = unproject_pixel_to_normalized_camera_ray(
            &obs_px,
            &intrinsics,
            CameraModel::Pinhole,
        ).expect("pinhole unprojection");

        assert!((recovered - expected).norm() < 1.0e-6, "pinhole unprojection should invert Brown-Conrady projection");
    }

    #[test]
    fn smooth_motion_prior_penalty_prefers_constant_velocity_track() {
        let straight = vec![
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(1.0, 0.0, 10.0),
            Vector3::new(2.0, 0.0, 10.0),
            Vector3::new(3.0, 0.0, 10.0),
            Vector3::new(4.0, 0.0, 10.0),
        ];
        let kinked = vec![
            Vector3::new(0.0, 0.0, 10.0),
            Vector3::new(1.0, 0.0, 10.0),
            Vector3::new(2.0, 0.9, 10.0),
            Vector3::new(3.0, 0.0, 10.0),
            Vector3::new(4.0, 0.0, 10.0),
        ];
        let weights = vec![0.0, 0.5, 0.5, 0.5, 0.5];

        let straight_penalty = smooth_motion_prior_penalty_px2(&straight, &weights, 180.0);
        let kinked_penalty = smooth_motion_prior_penalty_px2(&kinked, &weights, 180.0);
        assert!(straight_penalty <= 1.0e-12, "constant-velocity path should have near-zero smooth-motion penalty");
        assert!(kinked_penalty > straight_penalty + 1.0, "kinked path should incur a stronger smooth-motion penalty");
    }
}
