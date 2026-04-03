//! Quality-assurance report types and status classification.

use serde::{Deserialize, Serialize};

use crate::alignment::AlignmentStats;
use crate::dense::DsmStats;
use crate::features::MatchStats;
use crate::mosaic::MosaicResult;

/// Processing profile — determines which threshold preset to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProcessingProfile {
    /// Fast: relaxed thresholds suitable for rapid preview.
    Fast,
    /// Balanced: production-quality defaults (default).
    #[default]
    Balanced,
    /// Survey: strict thresholds for mapping and measurement.
    Survey,
}

impl ProcessingProfile {
    /// Parse from a string arg value.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "fast" => Some(Self::Fast),
            "balanced" => Some(Self::Balanced),
            "survey" => Some(Self::Survey),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Survey => "survey",
        }
    }
}

/// Per-profile QA thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileThresholds {
    /// Minimum fraction of frames that must align successfully.
    pub min_aligned_fraction: f64,
    /// Maximum acceptable reprojection RMSE in pixels.
    pub max_rmse_px: f64,
    /// Minimum image connectivity (fraction of pairs sharing a track).
    pub min_connectivity: f64,
    /// Minimum mean accepted matches per image pair.
    pub min_mean_matches_per_pair: f64,
    /// Minimum mean robust inter-image parallax (pixels).
    pub min_mean_parallax_px: f64,
    /// Maximum seam colour delta.
    pub max_seam_delta: f64,
}

impl ProfileThresholds {
    /// Return the preset thresholds for the given profile.
    pub fn for_profile(profile: ProcessingProfile) -> Self {
        match profile {
            ProcessingProfile::Fast => Self {
                min_aligned_fraction: 0.70,
                max_rmse_px: 2.0,
                min_connectivity: 0.60,
                min_mean_matches_per_pair: 60.0,
                min_mean_parallax_px: 1.8,
                max_seam_delta: 0.16,
            },
            ProcessingProfile::Balanced => Self {
                min_aligned_fraction: 0.82,
                max_rmse_px: 1.3,
                min_connectivity: 0.75,
                min_mean_matches_per_pair: 90.0,
                min_mean_parallax_px: 2.8,
                max_seam_delta: 0.10,
            },
            ProcessingProfile::Survey => Self {
                min_aligned_fraction: 0.90,
                max_rmse_px: 0.95,
                min_connectivity: 0.84,
                min_mean_matches_per_pair: 120.0,
                min_mean_parallax_px: 3.8,
                max_seam_delta: 0.07,
            },
        }
    }
}

/// Overall QA status for a completed workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QaStatus {
    /// All thresholds satisfied — outputs are publication-ready.
    Pass,
    /// Outputs complete but one or more soft thresholds were not met.
    Review,
    /// Hard failure — one or more required stages could not complete.
    Fail,
}

impl QaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Review => "review",
            Self::Fail => "fail",
        }
    }
}

/// Structured QA report for a single workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaReport {
    /// Overall status.
    pub status: QaStatus,
    /// Human-readable actions to take if status is Review or Fail.
    pub recommended_actions: Vec<String>,
    /// Thresholds that were applied.
    pub thresholds: ProfileThresholds,
    /// Measured alignment fraction.
    pub aligned_fraction: f64,
    /// Measured reprojection RMSE in pixels.
    pub rmse_px: f64,
    /// Image connectivity score.
    pub connectivity: f64,
    /// Worst seam colour delta.
    pub max_seam_delta: f64,
    /// Estimated GSD in metres.
    pub estimated_gsd_m: f64,
}

/// Build a [`QaReport`] from the per-stage results.
#[allow(clippy::too_many_arguments)]
pub fn build_qa_report(
    match_stats: &MatchStats,
    alignment: &AlignmentStats,
    dsm: &DsmStats,
    mosaic: &MosaicResult,
    thresholds: ProfileThresholds,
) -> QaReport {
    let mut actions: Vec<String> = Vec::new();
    let mut hard_fail = false;

    apply_dsm_quality_rules(dsm, alignment, &mut actions, &mut hard_fail);
    apply_feature_failure_diagnostics(match_stats, &mut actions, &mut hard_fail);

    if alignment.aligned_fraction < thresholds.min_aligned_fraction {
        if alignment.aligned_fraction < 0.5 {
            hard_fail = true;
            actions.push(format!(
                "Only {:.0}% of frames aligned (minimum {:.0}%). Increase image overlap or use a slower profile.",
                alignment.aligned_fraction * 100.0,
                thresholds.min_aligned_fraction * 100.0,
            ));
        } else {
            actions.push(format!(
                "Frame alignment {:.0}% is below the {:.0}% threshold. Consider re-flying with greater overlap.",
                alignment.aligned_fraction * 100.0,
                thresholds.min_aligned_fraction * 100.0,
            ));
        }
    }

    if alignment.rmse_px > thresholds.max_rmse_px {
        actions.push(format!(
            "Reprojection RMSE {:.2}px exceeds {:.2}px limit. Check for blurry images.",
            alignment.rmse_px, thresholds.max_rmse_px,
        ));
    }

    if alignment.loop_closure_constraints > 0 {
        let max_corr_review_m = loop_closure_correction_review_threshold_m(alignment.estimated_gsd_m);
        if alignment.max_loop_closure_correction_m > max_corr_review_m {
            actions.push(format!(
                "Loop-closure correction {:.2}m exceeds {:.2}m review threshold. Verify trajectory drift and overlap quality.",
                alignment.max_loop_closure_correction_m, max_corr_review_m,
            ));
        }

        let mean_corr_review_m = (max_corr_review_m * 0.65).clamp(0.30, max_corr_review_m);
        if alignment.loop_closure_constraints >= 2
            && alignment.mean_loop_closure_correction_m > mean_corr_review_m
        {
            actions.push(format!(
                "Mean loop-closure correction {:.2}m is elevated (>{:.2}m). Consider tighter matching/overlap before survey use.",
                alignment.mean_loop_closure_correction_m, mean_corr_review_m,
            ));
        }
    }

    if match_stats.connectivity < thresholds.min_connectivity {
        actions.push(format!(
            "Image connectivity {:.0}% is below {:.0}% threshold.",
            match_stats.connectivity * 100.0,
            thresholds.min_connectivity * 100.0,
        ));
    }

    if match_stats.mean_matches_per_pair < thresholds.min_mean_matches_per_pair {
        let critical_floor = (thresholds.min_mean_matches_per_pair * 0.45).max(20.0);
        if match_stats.mean_matches_per_pair < critical_floor {
            hard_fail = true;
            actions.push(format!(
                "Mean matches/pair {:.1} is critically below {:.1}. Add overlap and reduce motion blur before rerun.",
                match_stats.mean_matches_per_pair,
                thresholds.min_mean_matches_per_pair,
            ));
        } else {
            actions.push(format!(
                "Mean matches/pair {:.1} is below {:.1} threshold.",
                match_stats.mean_matches_per_pair,
                thresholds.min_mean_matches_per_pair,
            ));
        }
    }

    if match_stats.mean_parallax_px < thresholds.min_mean_parallax_px {
        let critical_floor = (thresholds.min_mean_parallax_px * 0.45).max(0.8);
        if match_stats.mean_parallax_px < critical_floor {
            hard_fail = true;
            actions.push(format!(
                "Mean parallax {:.2}px is critically low (target {:.2}px). Increase altitude variation or crossing geometry.",
                match_stats.mean_parallax_px,
                thresholds.min_mean_parallax_px,
            ));
        } else {
            actions.push(format!(
                "Mean parallax {:.2}px is below {:.2}px threshold.",
                match_stats.mean_parallax_px,
                thresholds.min_mean_parallax_px,
            ));
        }
    }

    if mosaic.stats.max_seam_delta > thresholds.max_seam_delta {
        actions.push(format!(
            "Seam delta {:.3} exceeds {:.3} limit. Consider colour-balancing the input images.",
            mosaic.stats.max_seam_delta, thresholds.max_seam_delta,
        ));
    }

    let status = if hard_fail {
        QaStatus::Fail
    } else if actions.is_empty() {
        QaStatus::Pass
    } else {
        QaStatus::Review
    };

    QaReport {
        status,
        recommended_actions: actions,
        thresholds,
        aligned_fraction: alignment.aligned_fraction,
        rmse_px: alignment.rmse_px,
        connectivity: match_stats.connectivity,
        max_seam_delta: mosaic.stats.max_seam_delta,
        estimated_gsd_m: alignment.estimated_gsd_m,
    }
}

fn loop_closure_correction_review_threshold_m(estimated_gsd_m: f64) -> f64 {
    let gsd = estimated_gsd_m.max(0.01);
    (gsd * 30.0).clamp(0.60, 4.00)
}

fn apply_feature_failure_diagnostics(
    match_stats: &MatchStats,
    actions: &mut Vec<String>,
    hard_fail: &mut bool,
) {
    for reason in &match_stats.failure_reasons {
        if !actions.iter().any(|existing| existing == reason) {
            actions.push(reason.clone());
        }
    }

    for code in &match_stats.failure_codes {
        match code.as_str() {
            // Feature-stage hard failures: reconstruction cannot proceed reliably.
            "no_frames" | "insufficient_frames" | "no_keypoints" | "no_verified_matches" => {
                *hard_fail = true;
            }
            // Soft degradations: keep outputs but require review.
            "low_keypoint_density" | "weak_connectivity" => {}
            // Unknown future code: treat as soft-review signal.
            _ => {
                let msg = format!("Feature diagnostic code reported: {}", code);
                if !actions.iter().any(|existing| existing == &msg) {
                    actions.push(msg);
                }
            }
        }
    }
}

fn apply_dsm_quality_rules(
    dsm: &DsmStats,
    alignment: &AlignmentStats,
    actions: &mut Vec<String>,
    hard_fail: &mut bool,
) {
    if dsm.valid_cells == 0 {
        *hard_fail = true;
        actions.push(
            "Dense surface contains no valid cells. Verify overlap, altitude consistency, and image quality."
                .to_string(),
        );
        return;
    }

    let invalid_range = !dsm.min_elevation_m.is_finite()
        || !dsm.max_elevation_m.is_finite()
        || !dsm.mean_elevation_m.is_finite()
        || dsm.max_elevation_m < dsm.min_elevation_m;
    if invalid_range {
        *hard_fail = true;
        actions.push(
            "Dense surface elevation statistics are invalid. Re-run reconstruction and verify input geotags."
                .to_string(),
        );
        return;
    }

    let elev_eps = 1e-9;
    if dsm.mean_elevation_m + elev_eps < dsm.min_elevation_m
        || dsm.mean_elevation_m - elev_eps > dsm.max_elevation_m
    {
        actions.push(
            "Dense surface mean elevation falls outside [min, max]. Inspect DSM outliers and nodata handling."
                .to_string(),
        );
    }

    let gsd = alignment.estimated_gsd_m.max(0.01);
    let vertical_rmse_limit = (gsd * 6.0).clamp(0.08, 3.0);
    if dsm.vertical_rmse_m > vertical_rmse_limit * 1.8 {
        *hard_fail = true;
        actions.push(format!(
            "Vertical RMSE {:.3}m is critically above {:.3}m. Reconstruct with a stricter profile and better overlap.",
            dsm.vertical_rmse_m, vertical_rmse_limit,
        ));
    } else if dsm.vertical_rmse_m > vertical_rmse_limit {
        actions.push(format!(
            "Vertical RMSE {:.3}m exceeds {:.3}m target for current GSD.",
            dsm.vertical_rmse_m, vertical_rmse_limit,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{CameraIntrinsics, CameraModel};
    use crate::mosaic::SeamStats;

    fn base_inputs() -> (MatchStats, AlignmentStats, DsmStats, MosaicResult, ProfileThresholds) {
        (
            MatchStats {
                frame_count: 12,
                total_keypoints: 15_000,
                total_matches: 22_000,
                connectivity: 0.91,
                mean_matches_per_pair: 140.0,
                mean_parallax_px: 5.5,
                pair_attempt_count: 20,
                pair_connected_count: 18,
                pair_rejected_count: 2,
                adjacent_pair_motions: Vec::new(),
                pair_correspondences: Vec::new(),
                failure_reasons: Vec::new(),
                failure_codes: Vec::new(),
                weak_pair_examples: Vec::new(),
            },
            AlignmentStats {
                aligned_fraction: 0.95,
                rmse_px: 0.75,
                residual_p50_px: 0.38,
                residual_p95_px: 1.15,
                sparse_cloud_points: 30_000,
                tie_points_median: 96,
                tracks_median: 4.8,
                mean_parallax_px: 5.5,
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
            DsmStats {
                valid_cells: 25_000,
                min_elevation_m: 98.0,
                max_elevation_m: 126.0,
                mean_elevation_m: 111.0,
                vertical_rmse_m: 0.15,
                mean_local_relief_m: 0.38,
                p95_local_relief_m: 0.81,
            },
            MosaicResult {
                ortho_path: "orthomosaic.tif".to_string(),
                stats: SeamStats {
                    seam_segments: 11,
                    max_seam_delta: 0.04,
                    mean_seam_delta: 0.02,
                    raw_seam_cells: 18,
                    optimized_seam_cells: 11,
                    seam_path_reduction_pct: 38.9,
                    overlap_edge_count: 2,
                    overlap_edge_samples: 140,
                },
                gsd_m: 0.05,
                projected_cells: 24_000,
                uncovered_cells: 1_000,
                coverage_pct: 96.0,
                confidence_raster_path: None,
                support_diagnostics_path: None,
                source_index_raster_path: None,
            },
            ProfileThresholds::for_profile(ProcessingProfile::Balanced),
        )
    }

    #[test]
    fn qa_passes_with_consistent_dense_stats() {
        let (match_stats, alignment, dsm, mosaic, thresholds) = base_inputs();
        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Pass);
        assert!(report.recommended_actions.is_empty());
    }

    #[test]
    fn qa_fails_when_dense_has_no_valid_cells() {
        let (match_stats, alignment, mut dsm, mosaic, thresholds) = base_inputs();
        dsm.valid_cells = 0;
        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Fail);
        assert!(report.recommended_actions.iter().any(|msg| msg.contains("no valid cells")));
    }

    #[test]
    fn qa_reviews_when_vertical_rmse_exceeds_limit() {
        let (match_stats, alignment, mut dsm, mosaic, thresholds) = base_inputs();
        dsm.vertical_rmse_m = 0.45;
        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Review);
        assert!(report.recommended_actions.iter().any(|msg| msg.contains("Vertical RMSE")));
    }

    #[test]
    fn qa_fails_when_feature_failure_code_is_hard() {
        let (mut match_stats, alignment, dsm, mosaic, thresholds) = base_inputs();
        match_stats.failure_reasons = vec!["No cross-image feature matches were verified.".to_string()];
        match_stats.failure_codes = vec!["no_verified_matches".to_string()];

        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Fail);
        assert!(report.recommended_actions.iter().any(|m| m.contains("No cross-image feature matches")));
    }

    #[test]
    fn qa_reviews_when_feature_failure_code_is_soft() {
        let (mut match_stats, alignment, dsm, mosaic, thresholds) = base_inputs();
        match_stats.failure_reasons = vec!["Feature graph connectivity is weak; overlap may be insufficient.".to_string()];
        match_stats.failure_codes = vec!["weak_connectivity".to_string()];

        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Review);
        assert!(report.recommended_actions.iter().any(|m| m.contains("weak")));
    }

    #[test]
    fn qa_reviews_when_loop_closure_correction_is_large() {
        let (match_stats, mut alignment, dsm, mosaic, thresholds) = base_inputs();
        alignment.loop_closure_constraints = 3;
        alignment.mean_loop_closure_correction_m = 1.15;
        alignment.max_loop_closure_correction_m = 1.95;

        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Review);
        assert!(report
            .recommended_actions
            .iter()
            .any(|m| m.contains("Loop-closure correction")));
    }

    #[test]
    fn qa_reviews_when_feature_pair_density_is_low() {
        let (mut match_stats, alignment, dsm, mosaic, thresholds) = base_inputs();
        match_stats.mean_matches_per_pair = 72.0;

        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Review);
        assert!(report
            .recommended_actions
            .iter()
            .any(|m| m.contains("matches/pair")));
    }

    #[test]
    fn qa_fails_when_feature_parallax_is_critically_low() {
        let (mut match_stats, alignment, dsm, mosaic, thresholds) = base_inputs();
        match_stats.mean_parallax_px = 1.0;

        let report = build_qa_report(&match_stats, &alignment, &dsm, &mosaic, thresholds);
        assert_eq!(report.status, QaStatus::Fail);
        assert!(report
            .recommended_actions
            .iter()
            .any(|m| m.contains("critically low")));
    }

    #[test]
    fn profile_thresholds_follow_expected_strictness_order() {
        let fast = ProfileThresholds::for_profile(ProcessingProfile::Fast);
        let balanced = ProfileThresholds::for_profile(ProcessingProfile::Balanced);
        let survey = ProfileThresholds::for_profile(ProcessingProfile::Survey);

        assert!(fast.min_aligned_fraction < balanced.min_aligned_fraction);
        assert!(balanced.min_aligned_fraction < survey.min_aligned_fraction);

        assert!(fast.max_rmse_px > balanced.max_rmse_px);
        assert!(balanced.max_rmse_px > survey.max_rmse_px);

        assert!(fast.min_connectivity < balanced.min_connectivity);
        assert!(balanced.min_connectivity < survey.min_connectivity);

        assert!(fast.min_mean_matches_per_pair < balanced.min_mean_matches_per_pair);
        assert!(balanced.min_mean_matches_per_pair < survey.min_mean_matches_per_pair);

        assert!(fast.min_mean_parallax_px < balanced.min_mean_parallax_px);
        assert!(balanced.min_mean_parallax_px < survey.min_mean_parallax_px);

        assert!(fast.max_seam_delta > balanced.max_seam_delta);
        assert!(balanced.max_seam_delta > survey.max_seam_delta);
    }

    #[test]
    fn calibrated_thresholds_match_observed_synthetic_and_real_like_samples() {
        // Observed sample envelopes from Sprint 1 validation runs:
        // synthetic-clean and small real-like datasets.
        let synthetic_clean = MatchStats {
            frame_count: 16,
            total_keypoints: 31_500,
            total_matches: 44_000,
            connectivity: 0.93,
            mean_matches_per_pair: 165.0,
            mean_parallax_px: 7.4,
            pair_attempt_count: 30,
            pair_connected_count: 28,
            pair_rejected_count: 2,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };
        let real_like = MatchStats {
            frame_count: 14,
            total_keypoints: 22_000,
            total_matches: 28_500,
            connectivity: 0.81,
            mean_matches_per_pair: 102.0,
            mean_parallax_px: 4.1,
            pair_attempt_count: 24,
            pair_connected_count: 21,
            pair_rejected_count: 3,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: Vec::new(),
            failure_codes: Vec::new(),
            weak_pair_examples: Vec::new(),
        };

        let alignment_synth = AlignmentStats {
            aligned_fraction: 0.97,
            rmse_px: 0.72,
            residual_p50_px: 0.34,
            residual_p95_px: 1.05,
            sparse_cloud_points: 41_000,
            tie_points_median: 108,
            tracks_median: 5.2,
            mean_parallax_px: 7.4,
            estimated_gsd_m: 0.04,
            intrinsics: CameraIntrinsics::identity(4000, 3000),
            model: CameraModel::Pinhole,
            loop_closure_constraints: 2,
            mean_loop_closure_correction_m: 0.22,
            max_loop_closure_correction_m: 0.34,
            ba_optimization_passes: 3,
            ba_huber_threshold_px: 2.0,
            ba_final_cost: 0.82,
            ba_intrinsics_refined: true,
            ba_distortion_refined: true,
            ba_observations_initial: 320,
            ba_observations_final: 272,
            ba_observation_retention_pct: 85.0,
            ba_supported_camera_fraction: 0.92,
            ba_observations_per_pass: vec![305, 289, 278, 272],
            ba_prune_thresholds_px: vec![5.8, 5.2, 4.9, 4.6],
            ba_camera_covariance: crate::alignment::CameraCovarianceDiagnostics::default(),
        };
        let alignment_real_like = AlignmentStats {
            aligned_fraction: 0.86,
            rmse_px: 1.14,
            residual_p50_px: 0.58,
            residual_p95_px: 1.92,
            sparse_cloud_points: 24_000,
            tie_points_median: 62,
            tracks_median: 3.7,
            mean_parallax_px: 4.1,
            estimated_gsd_m: 0.06,
            intrinsics: CameraIntrinsics::identity(4000, 3000),
            model: CameraModel::Pinhole,
            loop_closure_constraints: 1,
            mean_loop_closure_correction_m: 0.31,
            max_loop_closure_correction_m: 0.31,
            ba_optimization_passes: 3,
            ba_huber_threshold_px: 2.0,
            ba_final_cost: 1.34,
            ba_intrinsics_refined: true,
            ba_distortion_refined: true,
            ba_observations_initial: 280,
            ba_observations_final: 224,
            ba_observation_retention_pct: 80.0,
            ba_supported_camera_fraction: 0.86,
            ba_observations_per_pass: vec![266, 247, 233, 224],
            ba_prune_thresholds_px: vec![6.2, 5.5, 5.0, 4.7],
            ba_camera_covariance: crate::alignment::CameraCovarianceDiagnostics::default(),
        };

        let dsm_good = DsmStats {
            valid_cells: 33_000,
            min_elevation_m: 84.0,
            max_elevation_m: 142.0,
            mean_elevation_m: 109.0,
            vertical_rmse_m: 0.19,
            mean_local_relief_m: 0.47,
            p95_local_relief_m: 0.96,
        };
        let dsm_real_like = DsmStats {
            valid_cells: 22_500,
            min_elevation_m: 91.0,
            max_elevation_m: 136.0,
            mean_elevation_m: 111.0,
            vertical_rmse_m: 0.31,
            mean_local_relief_m: 0.61,
            p95_local_relief_m: 1.24,
        };

        let mosaic_synth = MosaicResult {
            ortho_path: "orthomosaic.tif".to_string(),
            stats: SeamStats {
                seam_segments: 14,
                max_seam_delta: 0.052,
                mean_seam_delta: 0.028,
                raw_seam_cells: 22,
                optimized_seam_cells: 14,
                seam_path_reduction_pct: 36.4,
                overlap_edge_count: 3,
                overlap_edge_samples: 220,
            },
            gsd_m: 0.04,
            projected_cells: 32_000,
            uncovered_cells: 1_000,
            coverage_pct: 96.97,
            confidence_raster_path: None,
            support_diagnostics_path: None,
            source_index_raster_path: None,
        };
        let mosaic_real_like = MosaicResult {
            ortho_path: "orthomosaic.tif".to_string(),
            stats: SeamStats {
                seam_segments: 9,
                max_seam_delta: 0.092,
                mean_seam_delta: 0.049,
                raw_seam_cells: 16,
                optimized_seam_cells: 9,
                seam_path_reduction_pct: 43.8,
                overlap_edge_count: 2,
                overlap_edge_samples: 96,
            },
            gsd_m: 0.06,
            projected_cells: 20_000,
            uncovered_cells: 2_500,
            coverage_pct: 88.89,
            confidence_raster_path: None,
            support_diagnostics_path: None,
            source_index_raster_path: None,
        };

        let fast = ProfileThresholds::for_profile(ProcessingProfile::Fast);
        let balanced = ProfileThresholds::for_profile(ProcessingProfile::Balanced);
        let survey = ProfileThresholds::for_profile(ProcessingProfile::Survey);

        let synth_balanced = build_qa_report(
            &synthetic_clean,
            &alignment_synth,
            &dsm_good,
            &mosaic_synth,
            balanced.clone(),
        );
        assert_eq!(synth_balanced.status, QaStatus::Pass);

        let real_fast = build_qa_report(
            &real_like,
            &alignment_real_like,
            &dsm_real_like,
            &mosaic_real_like,
            fast,
        );
        assert_ne!(real_fast.status, QaStatus::Fail);

        let real_balanced = build_qa_report(
            &real_like,
            &alignment_real_like,
            &dsm_real_like,
            &mosaic_real_like,
            balanced,
        );
        assert_ne!(real_balanced.status, QaStatus::Fail);

        let real_survey = build_qa_report(
            &real_like,
            &alignment_real_like,
            &dsm_real_like,
            &mosaic_real_like,
            survey,
        );
        assert_eq!(real_survey.status, QaStatus::Review);
    }
}
