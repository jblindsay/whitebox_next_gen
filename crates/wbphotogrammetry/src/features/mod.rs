//! Feature detection and matching.
//!
//! First-pass implementation: detects FAST-style corners, computes
//! deterministic BRIEF descriptors, and performs symmetric Hamming matching
//! with a simple translation-consistency filter.

use image::{DynamicImage, GrayImage, imageops::FilterType};
use nalgebra::{DMatrix, Matrix3, Vector2, Vector3};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use crate::error::{PhotogrammetryError, Result};
use crate::ingest::ImageFrame;

const FAST_EDGE_RADIUS_PX: u32 = 17;
const BRIEF_PATCH_DIAMETER_PX: u32 = 31;
const BRIEF_WORDS: usize = 4;
const BRIEF_AVERAGE_RADIUS_PX: i32 = 2;
const ORB_ORIENTATION_RADIUS_PX: i32 = 9;
const ORB_DESCRIPTOR_RADIUS_PX: i32 = 24;
const ORB_PYRAMID_LEVELS: usize = 4;
const ORB_SCALE_FACTOR: f64 = 1.2;
const SIFT_SCALES_PER_OCTAVE: usize = 3;
const SIFT_EXTRA_LEVELS: usize = 3;
const SIFT_BASE_SIGMA: f32 = 1.6;
const SIFT_CONTRAST_THRESHOLD: f32 = 0.018;
const SIFT_EDGE_RATIO_THRESHOLD: f32 = 10.0;
const SIFT_ORIENTATION_BINS: usize = 36;
const SIFT_DESCRIPTOR_CELLS: usize = 4;
const SIFT_DESCRIPTOR_BINS: usize = 8;
const SIFT_DESCRIPTOR_LEN: usize =
    SIFT_DESCRIPTOR_CELLS * SIFT_DESCRIPTOR_CELLS * SIFT_DESCRIPTOR_BINS;
const DESCRIPTOR_TEXTURE_RADIUS_PX: i32 = 4;
const MIN_DESCRIPTOR_STDDEV: f64 = 8.0;
const MIN_CORNER_SPACING_PX: i32 = 5;
const NON_MAX_SUPPRESSION_RADIUS_PX: i32 = 1;
const MIN_VERIFIED_PAIR_INLIERS: usize = 2;
const FUNDAMENTAL_RANSAC_MIN_INLIERS: usize = 8;
const SPATIAL_GRID_COLS: usize = 4;
const SPATIAL_GRID_ROWS: usize = 4;
const SPATIAL_MAX_MATCHES_PER_CELL: usize = 32;
const SPATIAL_MIN_OCCUPIED_CELLS: usize = 2;
const SPATIAL_MIN_AXIS_SPREAD_FRACTION: f64 = 0.18;
const HARRIS_WINDOW_RADIUS: i32 = 3;
const HARRIS_K: f64 = 0.04;
const GPS_DEFAULT_SENSOR_WIDTH_MM: f64 = 13.2;
const GPS_DEFAULT_HFOV_DEG: f64 = 84.0;
const GPS_FILTER_DEBUG_ENV: &str = "WBPHOTOGRAMMETRY_DEBUG_GPS_FILTER";
const PAIR_EDGE_MIN_INLIERS_ADJACENT: usize = 5;
const PAIR_EDGE_MIN_INLIERS_NON_ADJACENT: usize = 8;
const PAIR_EDGE_MIN_SPATIAL_PENALTY_ADJACENT: f64 = 0.20;
const PAIR_EDGE_MIN_SPATIAL_PENALTY_NON_ADJACENT: f64 = 0.28;
const PAIR_EDGE_MIN_MEDIAN_WEIGHT_NON_ADJACENT: f64 = 0.16;
const PAIR_EDGE_MEDIAN_WEIGHT_SOFT_MARGIN_INLIERS: usize = 4;
const PAIR_EDGE_SMALL_MISSION_FRAME_LIMIT: usize = 12;
const PAIR_EDGE_SMALL_MISSION_INLIER_RELAXATION: usize = 2;
const PAIR_EDGE_SMALL_MISSION_SPATIAL_RELAXATION: f64 = 0.08;
const PAIR_EDGE_SMALL_MISSION_MEDIAN_WEIGHT_RELAXATION: f64 = 0.03;
const WEAK_PAIR_EXAMPLE_LIMIT: usize = 12;

const FAST_CIRCLE_OFFSETS: [(i32, i32); 16] = [
    (0, -3),
    (1, -3),
    (2, -2),
    (3, -1),
    (3, 0),
    (3, 1),
    (2, 2),
    (1, 3),
    (0, 3),
    (-1, 3),
    (-2, 2),
    (-3, 1),
    (-3, 0),
    (-3, -1),
    (-2, -2),
    (-1, -3),
];

/// Summary statistics from the feature-matching stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchStats {
    /// Number of frames submitted.
    pub frame_count: usize,
    /// Total keypoints detected across all frames.
    pub total_keypoints: u64,
    /// Total accepted matches across all image pairs.
    pub total_matches: u64,
    /// Fraction of image pairs that share at least one track (0–1).
    pub connectivity: f64,
    /// Mean number of matches per image pair.
    pub mean_matches_per_pair: f64,
    /// Mean robust inter-image parallax (pixels) across connected pairs.
    pub mean_parallax_px: f64,
    /// Number of pair evaluations attempted after pair-candidate prefiltering.
    pub pair_attempt_count: u64,
    /// Number of pairs accepted into the final match graph.
    pub pair_connected_count: u64,
    /// Number of attempted pairs rejected by geometric-quality gating.
    pub pair_rejected_count: u64,
    /// Robust adjacent-pair image-plane motion estimates for correspondence-driven alignment.
    pub adjacent_pair_motions: Vec<AdjacentPairMotion>,
    /// Per-pair inlier correspondences in native image pixels, for essential matrix estimation.
    pub pair_correspondences: Vec<PairCorrespondences>,
    /// Explicit feature-stage failure or degradation reasons.
    pub failure_reasons: Vec<String>,
    /// Machine-readable feature-stage failure/degradation codes.
    pub failure_codes: Vec<String>,
    /// Example weak/rejected pair labels with reason codes.
    pub weak_pair_examples: Vec<String>,
}

/// Feature detection and description method used by the matching stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureMethod {
    /// FAST + BRIEF binary descriptors.
    Brief,
    /// ORB-style rotated BRIEF binary descriptors.
    Orb,
    /// Scale-space detector with floating-point SIFT descriptors.
    Sift,
    /// SIFT descriptors with L1-sqrt RootSIFT post-normalization.
    RootSift,
    /// Learned SuperPoint keypoints and floating-point descriptors.
    SuperPoint,
}

impl FeatureMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Brief => "brief",
            Self::Orb => "orb",
            Self::Sift => "sift",
            Self::RootSift => "rootsift",
            Self::SuperPoint => "superpoint",
        }
    }

    pub const fn descriptor_metric(self) -> FeatureDistanceMetric {
        match self {
            Self::Brief | Self::Orb => FeatureDistanceMetric::Hamming,
            Self::Sift | Self::RootSift => FeatureDistanceMetric::EuclideanL2,
            Self::SuperPoint => FeatureDistanceMetric::Cosine,
        }
    }

    pub const fn uses_floating_point_descriptors(self) -> bool {
        matches!(self, Self::Sift | Self::RootSift | Self::SuperPoint)
    }

    pub const fn is_implemented(self) -> bool {
        matches!(self, Self::Brief | Self::Orb | Self::Sift | Self::RootSift)
    }
}

impl fmt::Display for FeatureMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FeatureMethod {
    type Err = PhotogrammetryError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "brief" => Ok(Self::Brief),
            "orb" => Ok(Self::Orb),
            "sift" => Ok(Self::Sift),
            "rootsift" | "root_sift" => Ok(Self::RootSift),
            "superpoint" | "super_point" => Ok(Self::SuperPoint),
            _ => Err(PhotogrammetryError::FeatureMatching(format!(
                "unsupported feature method '{}'; expected one of: brief, orb, sift, rootsift, superpoint",
                value
            ))),
        }
    }
}

/// Distance metric required by a feature descriptor family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureDistanceMetric {
    Hamming,
    EuclideanL2,
    Cosine,
}

/// Public options for the feature matching stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureMatchingOptions {
    /// Built-in tuning profile (`fast`, `balanced`, `survey`).
    pub profile: String,
    /// Keypoint and descriptor method to use.
    pub method: FeatureMethod,
}

impl FeatureMatchingOptions {
    pub fn new(profile: impl Into<String>, method: FeatureMethod) -> Self {
        Self {
            profile: profile.into(),
            method,
        }
    }
}

impl Default for FeatureMatchingOptions {
    fn default() -> Self {
        Self::new("balanced", FeatureMethod::RootSift)
    }
}

/// Robust motion estimate between adjacent frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacentPairMotion {
    /// Index of the left frame in the input list.
    pub left_idx: usize,
    /// Index of the right frame in the input list.
    pub right_idx: usize,
    /// Robust translation model in image x (pixels).
    pub model_dx_px: f64,
    /// Robust translation model in image y (pixels).
    pub model_dy_px: f64,
    /// Number of inlier matches supporting this motion.
    pub inlier_count: usize,
    /// Median inlier displacement magnitude in pixels.
    pub median_displacement_px: f64,
}

/// Inlier point correspondences for a matched image pair, for use in pose estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairCorrespondences {
    /// Index of the left frame in the input list.
    pub left_frame_idx: usize,
    /// Index of the right frame in the input list.
    pub right_frame_idx: usize,
    /// Inlier correspondences in native image pixels: [left_x, left_y, right_x, right_y].
    pub points: Vec<[f64; 4]>,
    /// Relative confidence weight per inlier correspondence.
    pub confidence_weights: Vec<f64>,
}

#[derive(Debug, Clone, Copy)]
struct FeatureProfile {
    max_features_per_image: usize,
    fast_threshold: u8,
    match_distance_threshold: u32,
    float_match_distance_threshold: f64,
    ratio_test_threshold: f64,
    geometric_tolerance_px: f64,
    max_image_dimension_px: u32,
    gps_pair_footprint_multiplier: f64,
}

#[derive(Debug, Clone, Copy)]
enum FeatureAlgorithm {
    Brief,
    Orb,
    Sift,
    RootSift,
}

impl FeatureProfile {
    fn from_profile(profile: &str) -> Self {
        match profile {
            "fast" => Self {
                max_features_per_image: 450,
                fast_threshold: 28,
                match_distance_threshold: 96,
                float_match_distance_threshold: 0.95,
                ratio_test_threshold: 0.98,
                geometric_tolerance_px: 18.0,
                max_image_dimension_px: 1200,
                gps_pair_footprint_multiplier: 2.2,
            },
            "survey" => Self {
                max_features_per_image: 1_400,
                fast_threshold: 13,
                match_distance_threshold: 60,
                float_match_distance_threshold: 0.74,
                ratio_test_threshold: 0.92,
                geometric_tolerance_px: 11.5,
                max_image_dimension_px: 1800,
                gps_pair_footprint_multiplier: 1.6,
            },
            _ => Self {
                max_features_per_image: 1_100,
                fast_threshold: 16,
                match_distance_threshold: 78,
                float_match_distance_threshold: 0.82,
                ratio_test_threshold: 0.96,
                geometric_tolerance_px: 15.0,
                max_image_dimension_px: 1600,
                gps_pair_footprint_multiplier: 1.8,
            },
        }
    }

    fn tuned_for_orb(self) -> Self {
        let boosted_features = ((self.max_features_per_image as f64) * 1.35).round() as usize;
        Self {
            max_features_per_image: boosted_features.max(self.max_features_per_image + 200),
            fast_threshold: self.fast_threshold.saturating_sub(4).max(8),
            match_distance_threshold: (self.match_distance_threshold + 36).min(160),
            float_match_distance_threshold: (self.float_match_distance_threshold + 0.08).min(1.15),
            ratio_test_threshold: (self.ratio_test_threshold + 0.02).min(0.995),
            geometric_tolerance_px: (self.geometric_tolerance_px + 8.0).min(30.0),
            max_image_dimension_px: (self.max_image_dimension_px + 300).min(2200),
            gps_pair_footprint_multiplier: self.gps_pair_footprint_multiplier,
        }
    }

    fn tuned_for_sift(self) -> Self {
        Self {
            max_features_per_image: self.max_features_per_image.saturating_add(250),
            fast_threshold: self.fast_threshold,
            match_distance_threshold: self.match_distance_threshold,
            float_match_distance_threshold: (self.float_match_distance_threshold - 0.06).max(0.58),
            ratio_test_threshold: self.ratio_test_threshold.min(0.86),
            geometric_tolerance_px: (self.geometric_tolerance_px + 2.0).min(24.0),
            max_image_dimension_px: self.max_image_dimension_px,
            gps_pair_footprint_multiplier: self.gps_pair_footprint_multiplier,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TestPair {
    p0: (i32, i32),
    p1: (i32, i32),
}

#[derive(Debug, Clone, Copy)]
struct Keypoint {
    x: u32,
    y: u32,
    score: u32,
}

#[derive(Clone)]
struct BriefDescriptor {
    corner: Keypoint,
    words: [u64; BRIEF_WORDS],
    texture_stddev: f64,
    octave: u8,
}

#[derive(Clone)]
struct FloatDescriptor {
    corner: Keypoint,
    values: [f32; SIFT_DESCRIPTOR_LEN],
    texture_stddev: f64,
    octave: u8,
}

#[derive(Clone)]
struct FloatImage {
    width: usize,
    height: usize,
    data: Vec<f32>,
}

impl FloatImage {
    fn from_gray(image: &GrayImage) -> Self {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let mut data = Vec::with_capacity(width * height);
        for y in 0..image.height() {
            for x in 0..image.width() {
                data.push(image.get_pixel(x, y)[0] as f32 / 255.0);
            }
        }
        Self { width, height, data }
    }

    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0.0; width * height],
        }
    }

    fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }

    fn get_clamped(&self, x: i32, y: i32) -> f32 {
        let cx = x.clamp(0, self.width.saturating_sub(1) as i32) as usize;
        let cy = y.clamp(0, self.height.saturating_sub(1) as i32) as usize;
        self.get(cx, cy)
    }

    fn sample_bilinear(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let v00 = self.get_clamped(x0, y0);
        let v10 = self.get_clamped(x0 + 1, y0);
        let v01 = self.get_clamped(x0, y0 + 1);
        let v11 = self.get_clamped(x0 + 1, y0 + 1);

        let top = v00 * (1.0 - tx) + v10 * tx;
        let bottom = v01 * (1.0 - tx) + v11 * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

#[derive(Clone)]
struct FrameFeatures {
    keypoint_count: usize,
    binary_descriptors: Vec<BriefDescriptor>,
    float_descriptors: Vec<FloatDescriptor>,
    native_scale_px: f64,
    image_width_px: u32,
    image_height_px: u32,
}

#[derive(Debug, Clone, Copy)]
struct DescriptorMatch {
    left_idx: usize,
    right_idx: usize,
    best_dist: f64,
    second_dist: Option<f64>,
    metric: FeatureDistanceMetric,
}

#[derive(Debug, Clone, Copy)]
struct PairMatchCandidate {
    point: [f64; 4],
    descriptor_confidence: f64,
    texture_confidence: f64,
}

#[derive(Debug, Clone, Copy)]
struct PairInlierStats {
    inlier_count: usize,
    median_displacement_px: f64,
    model_dx_px: f64,
    model_dy_px: f64,
}

#[derive(Debug, Clone)]
struct PairMatchEvaluation {
    left_idx: usize,
    right_idx: usize,
    pair_stats: PairInlierStats,
    points: Vec<[f64; 4]>,
    weights: Vec<f64>,
    pair_native_scale: f64,
    accepted: bool,
    rejection_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct PairGateThresholds {
    min_inliers_adjacent: usize,
    min_inliers_non_adjacent: usize,
    min_spatial_penalty_adjacent: f64,
    min_spatial_penalty_non_adjacent: f64,
    min_median_weight_non_adjacent: f64,
}

#[derive(Debug, Clone, Copy)]
struct GpsPairPrior {
    baseline_ratio: f64,
    bearing_rad: f64,
}

fn gps_filter_debug_enabled() -> bool {
    std::env::var_os(GPS_FILTER_DEBUG_ENV).is_some()
}

fn gps_filter_debug_line(message: String) {
    if gps_filter_debug_enabled() {
        eprintln!("[gps-filter] {message}");
    }
}

fn pair_label_from_paths(left_path: &str, right_path: &str) -> String {
    let left_name = std::path::Path::new(left_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(left_path);
    let right_name = std::path::Path::new(right_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(right_path);
    format!("{left_name}|{right_name}")
}

fn pair_gate_thresholds(frame_count: usize) -> PairGateThresholds {
    if frame_count <= PAIR_EDGE_SMALL_MISSION_FRAME_LIMIT {
        PairGateThresholds {
            min_inliers_adjacent: PAIR_EDGE_MIN_INLIERS_ADJACENT
                .saturating_sub(PAIR_EDGE_SMALL_MISSION_INLIER_RELAXATION),
            min_inliers_non_adjacent: PAIR_EDGE_MIN_INLIERS_NON_ADJACENT
                .saturating_sub(PAIR_EDGE_SMALL_MISSION_INLIER_RELAXATION),
            min_spatial_penalty_adjacent: (PAIR_EDGE_MIN_SPATIAL_PENALTY_ADJACENT
                - PAIR_EDGE_SMALL_MISSION_SPATIAL_RELAXATION)
                .max(0.0),
            min_spatial_penalty_non_adjacent: (PAIR_EDGE_MIN_SPATIAL_PENALTY_NON_ADJACENT
                - PAIR_EDGE_SMALL_MISSION_SPATIAL_RELAXATION)
                .max(0.0),
            min_median_weight_non_adjacent: (PAIR_EDGE_MIN_MEDIAN_WEIGHT_NON_ADJACENT
                - PAIR_EDGE_SMALL_MISSION_MEDIAN_WEIGHT_RELAXATION)
                .max(0.0),
        }
    } else {
        PairGateThresholds {
            min_inliers_adjacent: PAIR_EDGE_MIN_INLIERS_ADJACENT,
            min_inliers_non_adjacent: PAIR_EDGE_MIN_INLIERS_NON_ADJACENT,
            min_spatial_penalty_adjacent: PAIR_EDGE_MIN_SPATIAL_PENALTY_ADJACENT,
            min_spatial_penalty_non_adjacent: PAIR_EDGE_MIN_SPATIAL_PENALTY_NON_ADJACENT,
            min_median_weight_non_adjacent: PAIR_EDGE_MIN_MEDIAN_WEIGHT_NON_ADJACENT,
        }
    }
}

fn evaluate_pair_edge_acceptance(
    adjacent: bool,
    inlier_count: usize,
    spatial_penalty: f64,
    median_weight: f64,
    thresholds: PairGateThresholds,
) -> (bool, Option<&'static str>) {
    let min_inliers = if adjacent {
        thresholds.min_inliers_adjacent
    } else {
        thresholds.min_inliers_non_adjacent
    };
    let min_spatial_penalty = if adjacent {
        thresholds.min_spatial_penalty_adjacent
    } else {
        thresholds.min_spatial_penalty_non_adjacent
    };

    if inlier_count < min_inliers {
        return (false, Some("low_inliers"));
    }
    if spatial_penalty < min_spatial_penalty {
        return (false, Some("poor_spatial_diversity"));
    }
    if !adjacent
        && median_weight < thresholds.min_median_weight_non_adjacent
        && inlier_count < (min_inliers + PAIR_EDGE_MEDIAN_WEIGHT_SOFT_MARGIN_INLIERS)
    {
        return (false, Some("low_median_confidence"));
    }

    (true, None)
}

/// Run feature detection and matching on `frames`.
///
/// Uses FAST-style corners and BRIEF descriptors. Matching is deterministic
/// across runs because a fixed BRIEF test-pattern is used.
pub fn run_feature_matching(frames: &[ImageFrame], profile: &str) -> Result<MatchStats> {
    run_feature_matching_brief(frames, profile)
}

/// Run feature matching using an explicit method selection.
pub fn run_feature_matching_with_method(
    frames: &[ImageFrame],
    profile: &str,
    method: FeatureMethod,
) -> Result<MatchStats> {
    let algorithm = feature_algorithm_from_method(method)?;
    run_feature_matching_with_algorithm(frames, profile, algorithm)
}

/// Run feature matching using a structured options object.
pub fn run_feature_matching_with_options(
    frames: &[ImageFrame],
    options: &FeatureMatchingOptions,
) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, &options.profile, options.method)
}

/// Run feature detection and matching using the legacy BRIEF descriptor path.
pub fn run_feature_matching_brief(frames: &[ImageFrame], profile: &str) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, profile, FeatureMethod::Brief)
}

/// Run feature detection and matching using ORB-style rotated BRIEF descriptors.
pub fn run_feature_matching_orb(frames: &[ImageFrame], profile: &str) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, profile, FeatureMethod::Orb)
}

/// Reserved public API hook for future floating-point SIFT descriptors.
pub fn run_feature_matching_sift(frames: &[ImageFrame], profile: &str) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, profile, FeatureMethod::Sift)
}

/// Run feature matching using SIFT keypoints with RootSIFT descriptor normalization.
pub fn run_feature_matching_rootsift(frames: &[ImageFrame], profile: &str) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, profile, FeatureMethod::RootSift)
}

/// Reserved public API hook for future learned SuperPoint descriptors.
pub fn run_feature_matching_superpoint(
    frames: &[ImageFrame],
    profile: &str,
) -> Result<MatchStats> {
    run_feature_matching_with_method(frames, profile, FeatureMethod::SuperPoint)
}

fn feature_algorithm_from_method(method: FeatureMethod) -> Result<FeatureAlgorithm> {
    match method {
        FeatureMethod::Brief => Ok(FeatureAlgorithm::Brief),
        FeatureMethod::Orb => Ok(FeatureAlgorithm::Orb),
        FeatureMethod::Sift => Ok(FeatureAlgorithm::Sift),
        FeatureMethod::RootSift => Ok(FeatureAlgorithm::RootSift),
        FeatureMethod::SuperPoint => Err(PhotogrammetryError::NotImplemented(
            "feature method 'superpoint' has been scaffolded in the public API, but learned inference, weight loading, and floating-point descriptor matching are not implemented yet".to_string(),
        )),
    }
}

fn run_feature_matching_with_algorithm(
    frames: &[ImageFrame],
    profile: &str,
    algorithm: FeatureAlgorithm,
) -> Result<MatchStats> {
    let n = frames.len();
    if n == 0 {
        return Ok(MatchStats {
            frame_count: 0,
            total_keypoints: 0,
            total_matches: 0,
            connectivity: 0.0,
            mean_matches_per_pair: 0.0,
            mean_parallax_px: 0.0,
            pair_attempt_count: 0,
            pair_connected_count: 0,
            pair_rejected_count: 0,
            adjacent_pair_motions: Vec::new(),
            pair_correspondences: Vec::new(),
            failure_reasons: vec!["No frames available for feature matching.".to_string()],
            failure_codes: vec!["no_frames".to_string()],
            weak_pair_examples: Vec::new(),
        });
    }

    let base_profile = FeatureProfile::from_profile(profile);
    let feature_profile = match algorithm {
        FeatureAlgorithm::Brief => base_profile,
        FeatureAlgorithm::Orb => base_profile.tuned_for_orb(),
        FeatureAlgorithm::Sift => base_profile.tuned_for_sift(),
        FeatureAlgorithm::RootSift => base_profile.tuned_for_sift(),
    };
    let test_pairs = match algorithm {
        FeatureAlgorithm::Brief => build_deterministic_test_pairs(256),
        FeatureAlgorithm::Orb => build_orb_rbrief_test_pairs(256),
        FeatureAlgorithm::Sift => Vec::new(),
        FeatureAlgorithm::RootSift => Vec::new(),
    };
    let octave_constraint: Option<u8> = match algorithm {
        FeatureAlgorithm::Brief => None,
        FeatureAlgorithm::Orb => Some(2),
        FeatureAlgorithm::Sift => Some(1),
        FeatureAlgorithm::RootSift => Some(1),
    };

    let frame_features: Vec<FrameFeatures> = frames
        .par_iter()
        .map(|frame| extract_frame_features(frame, feature_profile, &test_pairs, algorithm))
        .collect::<Result<Vec<_>>>()?;

    let total_keypoints = frame_features
        .iter()
        .map(|features| features.keypoint_count as u64)
        .sum::<u64>();

    let total_pairs = n.saturating_sub(1) * n / 2;

    let gate_thresholds = pair_gate_thresholds(n);

    // Run all-pairs matching in parallel, collecting per-pair stats and evaluating
    // graph-edge quality before accepting each pair into the final match graph.
    let all_pair_results: Vec<PairMatchEvaluation> =
        (0..frame_features.len())
        .into_par_iter()
        .flat_map(|i| {
            let mut local: Vec<PairMatchEvaluation> = Vec::new();
            for j in (i + 1)..frame_features.len() {
                if !should_attempt_pair(
                    &frames[i],
                    &frames[j],
                    i,
                    j,
                    feature_profile.gps_pair_footprint_multiplier,
                ) {
                    continue;
                }
                let pair_native_scale = 0.5
                    * (frame_features[i].native_scale_px + frame_features[j].native_scale_px);
                let (pair_stats, pts, weights) = verify_pair_matches(
                    &frames[i],
                    &frames[j],
                    &frame_features[i],
                    &frame_features[j],
                    feature_profile.match_distance_threshold,
                    feature_profile.float_match_distance_threshold,
                    feature_profile.ratio_test_threshold,
                    feature_profile.geometric_tolerance_px,
                    octave_constraint,
                );

                let adjacent = j == i + 1;

                let spatial_penalty = spatial_distribution_penalty(
                    &pts,
                    frame_features[i].image_width_px,
                    frame_features[i].image_height_px,
                    frame_features[j].image_width_px,
                    frame_features[j].image_height_px,
                );
                let mut sorted_weights = weights.clone();
                sorted_weights.sort_by(|a, b| a.total_cmp(b));
                let median_weight = if sorted_weights.is_empty() {
                    0.0
                } else {
                    sorted_weights[sorted_weights.len() / 2]
                };
                let (accepted, rejection_reason) = evaluate_pair_edge_acceptance(
                    adjacent,
                    pair_stats.inlier_count,
                    spatial_penalty,
                    median_weight,
                    gate_thresholds,
                );

                local.push(PairMatchEvaluation {
                    left_idx: i,
                    right_idx: j,
                    pair_stats,
                    points: pts,
                    weights,
                    pair_native_scale,
                    accepted,
                    rejection_reason,
                });
            }
            local
        })
        .collect();

    let attempted_pairs = all_pair_results.len() as u64;
    let connected_pairs = all_pair_results.iter().filter(|r| r.accepted).count() as u64;
    let rejected_pairs = attempted_pairs.saturating_sub(connected_pairs);
    let total_matches: u64 = all_pair_results
        .iter()
        .filter(|r| r.accepted)
        .map(|r| r.pair_stats.inlier_count as u64)
        .sum();
    let pair_parallax_sum: f64 = all_pair_results
        .iter()
        .filter(|r| r.accepted)
        .map(|r| r.pair_stats.median_displacement_px * r.pair_native_scale)
        .sum();

    let connectivity = if total_pairs > 0 {
        connected_pairs as f64 / total_pairs as f64
    } else {
        0.0
    };
    let mean_matches_per_pair = if total_pairs > 0 {
        total_matches as f64 / total_pairs as f64
    } else {
        0.0
    };
    let mean_parallax_px = if connected_pairs > 0 {
        pair_parallax_sum / connected_pairs as f64
    } else {
        0.0
    };

    // Build adjacent-pair motions and full correspondence list from the collected results.
    let mut adjacent_pair_motions = Vec::new();
    let mut pair_correspondences = Vec::new();
    let mut weak_pair_examples = Vec::new();
    for result in all_pair_results {
        if !result.accepted {
            if weak_pair_examples.len() < WEAK_PAIR_EXAMPLE_LIMIT {
                let left_name = std::path::Path::new(&frames[result.left_idx].path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("left");
                let right_name = std::path::Path::new(&frames[result.right_idx].path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("right");
                weak_pair_examples.push(format!(
                    "{}|{}:{}(inliers={})",
                    left_name,
                    right_name,
                    result.rejection_reason.unwrap_or("rejected"),
                    result.pair_stats.inlier_count
                ));
            }
            continue;
        }

        // Scale inlier points from downsampled to native image pixels.
        let native_pts: Vec<[f64; 4]> = result.points.iter()
            .map(|p| [
                p[0] * result.pair_native_scale,
                p[1] * result.pair_native_scale,
                p[2] * result.pair_native_scale,
                p[3] * result.pair_native_scale,
            ])
            .collect();
        pair_correspondences.push(PairCorrespondences {
            left_frame_idx: result.left_idx,
            right_frame_idx: result.right_idx,
            points: native_pts,
            confidence_weights: result.weights,
        });
        if result.right_idx == result.left_idx + 1 {
            adjacent_pair_motions.push(AdjacentPairMotion {
                left_idx: result.left_idx,
                right_idx: result.right_idx,
                model_dx_px: result.pair_stats.model_dx_px * result.pair_native_scale,
                model_dy_px: result.pair_stats.model_dy_px * result.pair_native_scale,
                inlier_count: result.pair_stats.inlier_count,
                median_displacement_px: result.pair_stats.median_displacement_px * result.pair_native_scale,
            });
        }
    }

    let (mut failure_reasons, mut failure_codes) = infer_failure_diagnostics(
        n,
        total_keypoints,
        total_matches,
        connectivity,
    );
    if rejected_pairs > 0 {
        failure_reasons.push(format!(
            "{} pair(s) were rejected by pair-graph quality gates (inlier strength, spatial diversity, or confidence).",
            rejected_pairs
        ));
        failure_codes.push("pair_graph_rejections".to_string());
    }

    Ok(MatchStats {
        frame_count: n,
        total_keypoints,
        total_matches,
        connectivity,
        mean_matches_per_pair,
        mean_parallax_px,
        pair_attempt_count: attempted_pairs,
        pair_connected_count: connected_pairs,
        pair_rejected_count: rejected_pairs,
        adjacent_pair_motions,
        pair_correspondences,
        failure_reasons,
        failure_codes,
        weak_pair_examples,
    })
}

fn infer_failure_diagnostics(
    frame_count: usize,
    total_keypoints: u64,
    total_matches: u64,
    connectivity: f64,
) -> (Vec<String>, Vec<String>) {
    let mut reasons = Vec::new();
    let mut codes = Vec::new();
    if frame_count < 2 {
        reasons.push("At least two frames are required for pairwise feature matching.".to_string());
        codes.push("insufficient_frames".to_string());
    }
    if total_keypoints == 0 {
        reasons.push("No keypoints were detected; imagery may be low texture, overexposed, or too blurry.".to_string());
        codes.push("no_keypoints".to_string());
    } else if total_keypoints < frame_count as u64 * 80 {
        reasons.push("Low keypoint density detected across frames.".to_string());
        codes.push("low_keypoint_density".to_string());
    }

    if total_matches == 0 {
        reasons.push("No cross-image feature matches were verified.".to_string());
        codes.push("no_verified_matches".to_string());
    } else if connectivity < 0.2 {
        reasons.push("Feature graph connectivity is weak; overlap may be insufficient.".to_string());
        codes.push("weak_connectivity".to_string());
    }

    (reasons, codes)
}

fn extract_frame_features(
    frame: &ImageFrame,
    profile: FeatureProfile,
    test_pairs: &[TestPair],
    algorithm: FeatureAlgorithm,
) -> Result<FrameFeatures> {
    let image = image::open(&frame.path).map_err(|e| {
        PhotogrammetryError::FeatureMatching(format!(
            "failed reading image '{}': {}",
            frame.path, e
        ))
    })?;
    let gray = downsample_to_gray(image, profile.max_image_dimension_px);
    let min_dim = gray.width().min(gray.height());
    let descriptor_diameter = match algorithm {
        FeatureAlgorithm::Brief => BRIEF_PATCH_DIAMETER_PX,
        FeatureAlgorithm::Orb => (ORB_DESCRIPTOR_RADIUS_PX as u32 * 2 + 1)
            .max(ORB_ORIENTATION_RADIUS_PX as u32 * 2 + 1),
        FeatureAlgorithm::Sift => 33,
        FeatureAlgorithm::RootSift => 33,
    };
    let required_dim = (FAST_EDGE_RADIUS_PX * 2 + 1).max(descriptor_diameter);
    if min_dim < required_dim {
        return Ok(FrameFeatures {
            keypoint_count: 0,
            binary_descriptors: Vec::new(),
            float_descriptors: Vec::new(),
            native_scale_px: 1.0,
            image_width_px: gray.width(),
            image_height_px: gray.height(),
        });
    }

    let (binary_descriptors, float_descriptors, keypoint_count) = match algorithm {
        FeatureAlgorithm::Brief => {
            let keypoints = detect_fast_corners(
                &gray,
                profile.fast_threshold,
                profile.max_features_per_image,
            );
            if keypoints.is_empty() {
                (Vec::new(), Vec::new(), 0)
            } else {
                let descriptors = compute_brief_descriptors(&gray, &keypoints, test_pairs);
                let count = descriptors.len();
                (descriptors, Vec::new(), count)
            }
        }
        FeatureAlgorithm::Orb => {
            let descriptors = extract_orb_pyramid_descriptors(&gray, profile, test_pairs);
            let count = descriptors.len();
            (descriptors, Vec::new(), count)
        }
        FeatureAlgorithm::Sift => {
            let descriptors = extract_sift_descriptors(&gray, profile, false);
            let count = descriptors.len();
            (Vec::new(), descriptors, count)
        }
        FeatureAlgorithm::RootSift => {
            let descriptors = extract_sift_descriptors(&gray, profile, true);
            let count = descriptors.len();
            (Vec::new(), descriptors, count)
        }
    };

    Ok(FrameFeatures {
        keypoint_count,
        binary_descriptors,
        float_descriptors,
        native_scale_px: (frame.width.max(1) as f64 / gray.width().max(1) as f64)
            .max(1.0),
        image_width_px: gray.width(),
        image_height_px: gray.height(),
    })
}

fn extract_orb_pyramid_descriptors(
    base_image: &GrayImage,
    profile: FeatureProfile,
    test_pairs: &[TestPair],
) -> Vec<BriefDescriptor> {
    let pyramid = build_orb_pyramid(base_image);
    if pyramid.is_empty() {
        return Vec::new();
    }
    let per_level_budget = distribute_orb_feature_budget(
        profile.max_features_per_image,
        pyramid.len(),
    );

    let mut pooled = Vec::new();
    for (level_idx, (level_image, scale_to_base)) in pyramid.iter().enumerate() {
        let budget = per_level_budget[level_idx];
        if budget == 0 {
            continue;
        }

        // Over-select before descriptor filtering to avoid sparse high-octave levels.
        let fast_threshold = orb_fast_threshold_for_level(profile.fast_threshold, level_idx);
        let mut keypoints = detect_fast_corners(level_image, fast_threshold, budget.saturating_mul(2));
        if keypoints.is_empty() {
            continue;
        }
        rerank_keypoints_with_harris(level_image, &mut keypoints);
        if keypoints.is_empty() {
            continue;
        }

        let mut level_descriptors = compute_orb_descriptors(level_image, &keypoints, test_pairs, level_idx as u8);
        for descriptor in &mut level_descriptors {
            let x_base = ((descriptor.corner.x as f64) * *scale_to_base)
                .round()
                .clamp(0.0, (base_image.width().saturating_sub(1)) as f64) as u32;
            let y_base = ((descriptor.corner.y as f64) * *scale_to_base)
                .round()
                .clamp(0.0, (base_image.height().saturating_sub(1)) as f64) as u32;
            descriptor.corner.x = x_base;
            descriptor.corner.y = y_base;
        }
        pooled.extend(level_descriptors);
    }

    if pooled.is_empty() {
        return Vec::new();
    }

    pooled.sort_by(|left, right| {
        right
            .corner
            .score
            .cmp(&left.corner.score)
            .then_with(|| right.texture_stddev.total_cmp(&left.texture_stddev))
    });

    let mut accepted: Vec<BriefDescriptor> = Vec::with_capacity(profile.max_features_per_image.min(pooled.len()));
    for candidate in pooled {
        if accepted.len() >= profile.max_features_per_image {
            break;
        }
        let too_close = accepted.iter().any(|other| {
            let dx = other.corner.x as i32 - candidate.corner.x as i32;
            let dy = other.corner.y as i32 - candidate.corner.y as i32;
            dx * dx + dy * dy <= MIN_CORNER_SPACING_PX * MIN_CORNER_SPACING_PX
        });
        if !too_close {
            accepted.push(candidate);
        }
    }

    accepted
}

fn extract_sift_descriptors(
    base_image: &GrayImage,
    profile: FeatureProfile,
    rootsift: bool,
) -> Vec<FloatDescriptor> {
    let base = FloatImage::from_gray(base_image);
    let pyramid = build_sift_pyramid(&base);
    if pyramid.is_empty() {
        return Vec::new();
    }

    let base_max_x = base_image.width().saturating_sub(1) as f32;
    let base_max_y = base_image.height().saturating_sub(1) as f32;
    let mut descriptors = Vec::with_capacity(profile.max_features_per_image.saturating_mul(2));
    for (octave_idx, octave) in pyramid.iter().enumerate() {
        let octave_scale = 2.0_f32.powi(octave_idx as i32);
        let sigma_octave_scale = SIFT_BASE_SIGMA * octave_scale;
        let mut sigma_by_scale_idx = Vec::with_capacity(octave.dogs.len().saturating_sub(2));
        for scale_idx in 1..octave.dogs.len().saturating_sub(1) {
            sigma_by_scale_idx.push(
                sigma_octave_scale
                    * 2.0_f32.powf(scale_idx as f32 / SIFT_SCALES_PER_OCTAVE as f32),
            );
        }

        for scale_idx in 1..octave.dogs.len().saturating_sub(1) {
            let dog = &octave.dogs[scale_idx];
            if dog.width < 17 || dog.height < 17 {
                continue;
            }
            let sigma = sigma_by_scale_idx[scale_idx - 1];
            let gaussian = &octave.gaussians[scale_idx + 1];
            for y in 8..(dog.height - 8) {
                for x in 8..(dog.width - 8) {
                    let value = dog.get(x, y);
                    if value.abs() < SIFT_CONTRAST_THRESHOLD {
                        continue;
                    }
                    if !is_sift_scale_space_extremum(octave, scale_idx, x, y, value) {
                        continue;
                    }
                    if sift_edge_like_response(dog, x, y) {
                        continue;
                    }

                    let Some((angle, response)) = sift_dominant_orientation(gaussian, x as f32, y as f32, sigma) else {
                        continue;
                    };
                    let texture = sift_local_stddev(gaussian, x as i32, y as i32, 4);
                    if texture < (MIN_DESCRIPTOR_STDDEV as f32 / 255.0) {
                        continue;
                    }
                    let Some(mut values) = compute_sift_descriptor(gaussian, x as f32, y as f32, sigma, angle) else {
                        continue;
                    };
                    if rootsift {
                        let Some(()) = normalize_rootsift_descriptor(&mut values) else {
                            continue;
                        };
                    }

                    let base_x = (x as f32 * octave_scale).round().clamp(0.0, base_max_x) as u32;
                    let base_y = (y as f32 * octave_scale).round().clamp(0.0, base_max_y) as u32;
                    let score = (response * 1_000.0).clamp(0.0, u32::MAX as f32) as u32;
                    descriptors.push(FloatDescriptor {
                        corner: Keypoint {
                            x: base_x,
                            y: base_y,
                            score,
                        },
                        values,
                        texture_stddev: texture as f64 * 255.0,
                        octave: octave_idx as u8,
                    });
                }
            }
        }
    }

    descriptors.sort_by(|left, right| {
        right
            .corner
            .score
            .cmp(&left.corner.score)
            .then_with(|| right.texture_stddev.total_cmp(&left.texture_stddev))
    });

    let mut accepted: Vec<FloatDescriptor> =
        Vec::with_capacity(profile.max_features_per_image.min(descriptors.len()));
    for candidate in descriptors {
        if accepted.len() >= profile.max_features_per_image {
            break;
        }
        let too_close = accepted.iter().any(|other| {
            let dx = other.corner.x as i32 - candidate.corner.x as i32;
            let dy = other.corner.y as i32 - candidate.corner.y as i32;
            dx * dx + dy * dy <= (MIN_CORNER_SPACING_PX * 2) * (MIN_CORNER_SPACING_PX * 2)
        });
        if !too_close {
            accepted.push(candidate);
        }
    }

    accepted
}

struct SiftOctave {
    gaussians: Vec<FloatImage>,
    dogs: Vec<FloatImage>,
}

fn build_sift_pyramid(base: &FloatImage) -> Vec<SiftOctave> {
    let mut octaves = Vec::with_capacity(6);
    let mut current = gaussian_blur_float(base, SIFT_BASE_SIGMA.max(0.8));
    let levels_per_octave = SIFT_SCALES_PER_OCTAVE + SIFT_EXTRA_LEVELS;
    let k = 2.0_f32.powf(1.0 / SIFT_SCALES_PER_OCTAVE as f32);

    while current.width >= 24 && current.height >= 24 && octaves.len() < 6 {
        let mut gaussians = Vec::with_capacity(levels_per_octave);
        gaussians.push(current.clone());

        let mut sigma_prev = SIFT_BASE_SIGMA;
        for level_idx in 1..levels_per_octave {
            let sigma_total = SIFT_BASE_SIGMA * k.powf(level_idx as f32);
            let sigma_diff = (sigma_total * sigma_total - sigma_prev * sigma_prev).max(0.01).sqrt();
            let blurred = gaussian_blur_float(gaussians.last().unwrap(), sigma_diff);
            gaussians.push(blurred);
            sigma_prev = sigma_total;
        }

        let mut dogs = Vec::with_capacity(gaussians.len().saturating_sub(1));
        for idx in 1..gaussians.len() {
            dogs.push(subtract_float_images(&gaussians[idx], &gaussians[idx - 1]));
        }
        octaves.push(SiftOctave { gaussians, dogs });

        current = downsample_half_float_image(&octaves.last().unwrap().gaussians[SIFT_SCALES_PER_OCTAVE]);
    }

    octaves
}

fn gaussian_blur_float(image: &FloatImage, sigma: f32) -> FloatImage {
    let radius = (sigma * 3.0).ceil().max(1.0) as i32;
    let kernel = gaussian_kernel_1d(sigma.max(0.1), radius);

    let mut horizontal = FloatImage::new(image.width, image.height);
    for y in 0..image.height {
        for x in 0..image.width {
            let mut sum = 0.0;
            for (k_idx, weight) in kernel.iter().enumerate() {
                let offset = k_idx as i32 - radius;
                sum += image.get_clamped(x as i32 + offset, y as i32) * *weight;
            }
            horizontal.data[y * image.width + x] = sum;
        }
    }

    let mut output = FloatImage::new(image.width, image.height);
    for y in 0..image.height {
        for x in 0..image.width {
            let mut sum = 0.0;
            for (k_idx, weight) in kernel.iter().enumerate() {
                let offset = k_idx as i32 - radius;
                sum += horizontal.get_clamped(x as i32, y as i32 + offset) * *weight;
            }
            output.data[y * image.width + x] = sum;
        }
    }
    output
}

fn gaussian_kernel_1d(sigma: f32, radius: i32) -> Vec<f32> {
    let mut kernel = Vec::with_capacity((radius * 2 + 1) as usize);
    let mut sum = 0.0;
    for offset in -radius..=radius {
        let value = (-((offset * offset) as f32) / (2.0 * sigma * sigma)).exp();
        kernel.push(value);
        sum += value;
    }
    if sum > 0.0 {
        for value in &mut kernel {
            *value /= sum;
        }
    }
    kernel
}

fn subtract_float_images(left: &FloatImage, right: &FloatImage) -> FloatImage {
    let mut output = FloatImage::new(left.width, left.height);
    for idx in 0..left.data.len() {
        output.data[idx] = left.data[idx] - right.data[idx];
    }
    output
}

fn downsample_half_float_image(image: &FloatImage) -> FloatImage {
    let width = (image.width / 2).max(1);
    let height = (image.height / 2).max(1);
    let mut output = FloatImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let sx = x * 2;
            let sy = y * 2;
            let v00 = image.get(sx.min(image.width - 1), sy.min(image.height - 1));
            let v10 = image.get((sx + 1).min(image.width - 1), sy.min(image.height - 1));
            let v01 = image.get(sx.min(image.width - 1), (sy + 1).min(image.height - 1));
            let v11 = image.get((sx + 1).min(image.width - 1), (sy + 1).min(image.height - 1));
            output.data[y * width + x] = 0.25 * (v00 + v10 + v01 + v11);
        }
    }
    output
}

fn is_sift_scale_space_extremum(octave: &SiftOctave, scale_idx: usize, x: usize, y: usize, value: f32) -> bool {
    let is_max = value > 0.0;
    for ds in -1isize..=1 {
        let dog = &octave.dogs[(scale_idx as isize + ds) as usize];
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if ds == 0 && dx == 0 && dy == 0 {
                    continue;
                }
                let neighbor = dog.get((x as isize + dx) as usize, (y as isize + dy) as usize);
                if is_max {
                    if neighbor >= value {
                        return false;
                    }
                } else if neighbor <= value {
                    return false;
                }
            }
        }
    }
    true
}

fn sift_edge_like_response(image: &FloatImage, x: usize, y: usize) -> bool {
    let x = x as i32;
    let y = y as i32;
    let dxx = image.get_clamped(x + 1, y) + image.get_clamped(x - 1, y) - 2.0 * image.get_clamped(x, y);
    let dyy = image.get_clamped(x, y + 1) + image.get_clamped(x, y - 1) - 2.0 * image.get_clamped(x, y);
    let dxy = (image.get_clamped(x + 1, y + 1) - image.get_clamped(x + 1, y - 1)
        - image.get_clamped(x - 1, y + 1) + image.get_clamped(x - 1, y - 1))
        * 0.25;
    let trace = dxx + dyy;
    let det = dxx * dyy - dxy * dxy;
    if det <= 1.0e-8 {
        return true;
    }
    (trace * trace / det) > ((SIFT_EDGE_RATIO_THRESHOLD + 1.0).powi(2) / SIFT_EDGE_RATIO_THRESHOLD)
}

fn sift_dominant_orientation(image: &FloatImage, x: f32, y: f32, sigma: f32) -> Option<(f32, f32)> {
    let radius = (3.0 * 1.5 * sigma).ceil() as i32;
    if radius < 1 {
        return None;
    }
    let mut histogram = [0.0_f32; SIFT_ORIENTATION_BINS];
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let fx = x + dx as f32;
            let fy = y + dy as f32;
            let gx = image.sample_bilinear(fx + 1.0, fy) - image.sample_bilinear(fx - 1.0, fy);
            let gy = image.sample_bilinear(fx, fy + 1.0) - image.sample_bilinear(fx, fy - 1.0);
            let mag = (gx * gx + gy * gy).sqrt();
            if mag <= 1.0e-6 {
                continue;
            }
            let angle = gy.atan2(gx);
            let weight = (-(dx * dx + dy * dy) as f32 / (2.0 * (1.5 * sigma) * (1.5 * sigma))).exp();
            let mut bin = (((angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI))
                * SIFT_ORIENTATION_BINS as f32)
                .floor() as isize;
            bin = bin.rem_euclid(SIFT_ORIENTATION_BINS as isize);
            histogram[bin as usize] += mag * weight;
        }
    }
    let (best_bin, best_value) = histogram
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    if best_value <= 1.0e-6 {
        return None;
    }
    let angle = ((best_bin as f32 + 0.5) / SIFT_ORIENTATION_BINS as f32)
        * 2.0
        * std::f32::consts::PI
        - std::f32::consts::PI;
    Some((angle, best_value))
}

fn compute_sift_descriptor(
    image: &FloatImage,
    x: f32,
    y: f32,
    sigma: f32,
    angle: f32,
) -> Option<[f32; SIFT_DESCRIPTOR_LEN]> {
    let mut descriptor = [0.0_f32; SIFT_DESCRIPTOR_LEN];
    let cos_theta = angle.cos();
    let sin_theta = angle.sin();
    let window_half = (sigma * 8.0).max(8.0);
    let sample_step = (2.0 * window_half) / 16.0;

    for sample_y in 0..16 {
        for sample_x in 0..16 {
            let local_x = (sample_x as f32 + 0.5) * sample_step - window_half;
            let local_y = (sample_y as f32 + 0.5) * sample_step - window_half;
            let rot_x = cos_theta * local_x - sin_theta * local_y;
            let rot_y = sin_theta * local_x + cos_theta * local_y;
            let fx = x + rot_x;
            let fy = y + rot_y;

            if fx < 1.0 || fy < 1.0 || fx >= (image.width - 2) as f32 || fy >= (image.height - 2) as f32 {
                continue;
            }

            let gx = image.sample_bilinear(fx + 1.0, fy) - image.sample_bilinear(fx - 1.0, fy);
            let gy = image.sample_bilinear(fx, fy + 1.0) - image.sample_bilinear(fx, fy - 1.0);
            let mag = (gx * gx + gy * gy).sqrt();
            if mag <= 1.0e-6 {
                continue;
            }

            let mut rel_angle = gy.atan2(gx) - angle;
            while rel_angle < 0.0 {
                rel_angle += 2.0 * std::f32::consts::PI;
            }
            while rel_angle >= 2.0 * std::f32::consts::PI {
                rel_angle -= 2.0 * std::f32::consts::PI;
            }

            let cell_x = sample_x / 4;
            let cell_y = sample_y / 4;
            let bin = ((rel_angle / (2.0 * std::f32::consts::PI)) * SIFT_DESCRIPTOR_BINS as f32)
                .floor()
                .clamp(0.0, (SIFT_DESCRIPTOR_BINS - 1) as f32) as usize;
            let gaussian_weight = (-(local_x * local_x + local_y * local_y)
                / (2.0 * (0.5 * 16.0 * sigma).max(1.0).powi(2)))
                .exp();
            let idx = (cell_y * SIFT_DESCRIPTOR_CELLS + cell_x) * SIFT_DESCRIPTOR_BINS + bin;
            descriptor[idx] += mag * gaussian_weight;
        }
    }

    normalize_sift_descriptor(&mut descriptor)?;
    Some(descriptor)
}

fn normalize_sift_descriptor(descriptor: &mut [f32; SIFT_DESCRIPTOR_LEN]) -> Option<()> {
    let mut norm = descriptor.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= 1.0e-8 {
        return None;
    }
    for value in descriptor.iter_mut() {
        *value /= norm;
        if *value > 0.2 {
            *value = 0.2;
        }
    }
    norm = descriptor.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= 1.0e-8 {
        return None;
    }
    for value in descriptor.iter_mut() {
        *value /= norm;
    }
    Some(())
}

fn normalize_rootsift_descriptor(descriptor: &mut [f32; SIFT_DESCRIPTOR_LEN]) -> Option<()> {
    let l1_norm = descriptor.iter().map(|v| v.abs()).sum::<f32>();
    if l1_norm <= 1.0e-8 {
        return None;
    }
    for value in descriptor.iter_mut() {
        *value = (*value / l1_norm).max(0.0).sqrt();
    }

    let l2_norm = descriptor.iter().map(|v| v * v).sum::<f32>().sqrt();
    if l2_norm <= 1.0e-8 {
        return None;
    }
    for value in descriptor.iter_mut() {
        *value /= l2_norm;
    }
    Some(())
}

fn sift_local_stddev(image: &FloatImage, x: i32, y: i32, radius: i32) -> f32 {
    let mut sum = 0.0_f32;
    let mut sum_sq = 0.0_f32;
    let mut count = 0.0_f32;
    for py in (y - radius)..=(y + radius) {
        for px in (x - radius)..=(x + radius) {
            let value = image.get_clamped(px, py);
            sum += value;
            sum_sq += value * value;
            count += 1.0;
        }
    }
    if count <= 0.0 {
        return 0.0;
    }
    let mean = sum / count;
    (sum_sq / count - mean * mean).max(0.0).sqrt()
}

fn build_orb_pyramid(base_image: &GrayImage) -> Vec<(GrayImage, f64)> {
    let mut levels = Vec::with_capacity(ORB_PYRAMID_LEVELS);
    let min_level_dim = (FAST_EDGE_RADIUS_PX * 2 + 1)
        .max(ORB_DESCRIPTOR_RADIUS_PX as u32 * 2 + 1)
        .max(ORB_ORIENTATION_RADIUS_PX as u32 * 2 + 1);

    levels.push((base_image.clone(), 1.0));
    for level_idx in 1..ORB_PYRAMID_LEVELS {
        let scale_to_base = ORB_SCALE_FACTOR.powi(level_idx as i32);
        let target_width = ((base_image.width() as f64) / scale_to_base)
            .round()
            .max(1.0) as u32;
        let target_height = ((base_image.height() as f64) / scale_to_base)
            .round()
            .max(1.0) as u32;
        if target_width < min_level_dim || target_height < min_level_dim {
            break;
        }
        let level_img = image::imageops::resize(
            base_image,
            target_width,
            target_height,
            FilterType::Triangle,
        );
        levels.push((level_img, scale_to_base));
    }

    levels
}

fn distribute_orb_feature_budget(total_features: usize, levels: usize) -> Vec<usize> {
    if levels == 0 || total_features == 0 {
        return vec![0; levels];
    }

    let mut weights = Vec::with_capacity(levels);
    let mut weight_sum = 0.0_f64;
    for level_idx in 0..levels {
        // Favor fine scales while still allocating enough budget to coarser octaves.
        let weight = 1.0 / ORB_SCALE_FACTOR.powi(level_idx as i32);
        weights.push(weight);
        weight_sum += weight;
    }

    let mut quotas = vec![0usize; levels];
    let mut assigned = 0usize;
    for (level_idx, weight) in weights.iter().enumerate() {
        let q = ((total_features as f64) * (*weight / weight_sum)).floor() as usize;
        quotas[level_idx] = q;
        assigned += q;
    }

    // Distribute remainders from fine to coarse levels.
    let mut level_idx = 0usize;
    while assigned < total_features {
        quotas[level_idx % levels] += 1;
        assigned += 1;
        level_idx += 1;
    }

    quotas
}

fn orb_fast_threshold_for_level(base_threshold: u8, level_idx: usize) -> u8 {
    let relaxed = (base_threshold as i32 - level_idx as i32 * 2).max(6);
    relaxed as u8
}

fn compute_orb_descriptors(
    image: &GrayImage,
    keypoints: &[Keypoint],
    test_pairs: &[TestPair],
    octave: u8,
) -> Vec<BriefDescriptor> {
    keypoints
        .par_iter()
        .filter_map(|keypoint| {
            let cx = keypoint.x as i32;
            let cy = keypoint.y as i32;
            if patch_stddev(image, cx, cy, DESCRIPTOR_TEXTURE_RADIUS_PX) < MIN_DESCRIPTOR_STDDEV {
                return None;
            }
            if !is_patch_inside(image, cx, cy, ORB_DESCRIPTOR_RADIUS_PX)
                || !is_patch_inside(image, cx, cy, ORB_ORIENTATION_RADIUS_PX)
            {
                return None;
            }

            let angle = orb_keypoint_orientation(image, cx, cy);
            let cos_theta = angle.cos();
            let sin_theta = angle.sin();

            let mut words = [0_u64; BRIEF_WORDS];
            for (bit_idx, pair) in test_pairs.iter().enumerate() {
                let (x0, y0) = rotate_point(pair.p0.0, pair.p0.1, cos_theta, sin_theta);
                let (x1, y1) = rotate_point(pair.p1.0, pair.p1.1, cos_theta, sin_theta);
                let p0 = patch_average(image, cx + x0, cy + y0);
                let p1 = patch_average(image, cx + x1, cy + y1);
                if p0 < p1 {
                    words[bit_idx / 64] |= 1_u64 << (bit_idx % 64);
                }
            }

            Some(BriefDescriptor {
                corner: *keypoint,
                words,
                texture_stddev: patch_stddev(image, cx, cy, DESCRIPTOR_TEXTURE_RADIUS_PX),
                octave,
            })
        })
        .collect()
}

fn is_patch_inside(image: &GrayImage, cx: i32, cy: i32, radius: i32) -> bool {
    let width = image.width() as i32;
    let height = image.height() as i32;
    cx - radius - BRIEF_AVERAGE_RADIUS_PX >= 0
        && cy - radius - BRIEF_AVERAGE_RADIUS_PX >= 0
        && cx + radius + BRIEF_AVERAGE_RADIUS_PX < width
        && cy + radius + BRIEF_AVERAGE_RADIUS_PX < height
}

fn orb_keypoint_orientation(image: &GrayImage, cx: i32, cy: i32) -> f64 {
    let mut m01 = 0.0_f64;
    let mut m10 = 0.0_f64;
    for dy in -ORB_ORIENTATION_RADIUS_PX..=ORB_ORIENTATION_RADIUS_PX {
        for dx in -ORB_ORIENTATION_RADIUS_PX..=ORB_ORIENTATION_RADIUS_PX {
            if dx * dx + dy * dy > ORB_ORIENTATION_RADIUS_PX * ORB_ORIENTATION_RADIUS_PX {
                continue;
            }
            let x = (cx + dx) as u32;
            let y = (cy + dy) as u32;
            let intensity = image.get_pixel(x, y)[0] as f64;
            m10 += dx as f64 * intensity;
            m01 += dy as f64 * intensity;
        }
    }
    m01.atan2(m10)
}

fn rotate_point(dx: i32, dy: i32, cos_theta: f64, sin_theta: f64) -> (i32, i32) {
    let rx = cos_theta * dx as f64 - sin_theta * dy as f64;
    let ry = sin_theta * dx as f64 + cos_theta * dy as f64;
    (rx.round() as i32, ry.round() as i32)
}

fn detect_fast_corners(
    image: &GrayImage,
    threshold: u8,
    max_features: usize,
) -> Vec<Keypoint> {
    let width = image.width();
    let height = image.height();
    if width <= FAST_EDGE_RADIUS_PX * 2 || height <= FAST_EDGE_RADIUS_PX * 2 {
        return Vec::new();
    }

    let row_hits: Vec<Vec<(usize, Keypoint)>> =
        (FAST_EDGE_RADIUS_PX..(height - FAST_EDGE_RADIUS_PX))
            .into_par_iter()
            .map(|y| {
                let mut local = Vec::new();
                for x in FAST_EDGE_RADIUS_PX..(width - FAST_EDGE_RADIUS_PX) {
                    if let Some(score) = fast_corner_score(image, x as i32, y as i32, threshold) {
                        local.push(((y * width + x) as usize, Keypoint { x, y, score }));
                    }
                }
                local
            })
            .collect();

    let mut candidates = Vec::new();
    let mut score_map = vec![0_u32; (width * height) as usize];
    for row in row_hits {
        for (idx, candidate) in row {
            score_map[idx] = candidate.score;
            candidates.push(candidate);
        }
    }

    candidates.retain(|candidate| {
        is_local_maximum(
            &score_map,
            width,
            height,
            candidate.x as i32,
            candidate.y as i32,
            candidate.score,
            NON_MAX_SUPPRESSION_RADIUS_PX,
        )
    });

    candidates.sort_by(|left, right| right.score.cmp(&left.score));

    let mut accepted: Vec<Keypoint> = Vec::with_capacity(max_features.min(candidates.len()));
    for candidate in candidates {
        if accepted.len() >= max_features {
            break;
        }
        if accepted.iter().any(|other| {
            let dx = other.x as i32 - candidate.x as i32;
            let dy = other.y as i32 - candidate.y as i32;
            dx * dx + dy * dy <= MIN_CORNER_SPACING_PX * MIN_CORNER_SPACING_PX
        }) {
            continue;
        }
        accepted.push(candidate);
    }

    accepted
}

fn is_local_maximum(
    score_map: &[u32],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    score: u32,
    radius: i32,
) -> bool {
    for ny in (y - radius)..=(y + radius) {
        for nx in (x - radius)..=(x + radius) {
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            if nx == x && ny == y {
                continue;
            }

            let neighbor_score = score_map[(ny as u32 * width + nx as u32) as usize];
            if neighbor_score > score {
                return false;
            }
            if neighbor_score == score && (ny < y || (ny == y && nx < x)) {
                return false;
            }
        }
    }
    true
}

fn fast_corner_score(image: &GrayImage, x: i32, y: i32, threshold: u8) -> Option<u32> {
    let center = image.get_pixel(x as u32, y as u32)[0] as i16;
    let threshold = threshold as i16;
    let mut diffs = [0_i16; FAST_CIRCLE_OFFSETS.len()];
    for (idx, (dx, dy)) in FAST_CIRCLE_OFFSETS.iter().enumerate() {
        diffs[idx] = image.get_pixel((x + dx) as u32, (y + dy) as u32)[0] as i16 - center;
    }

    let bright = diffs.iter().map(|diff| *diff >= threshold).collect::<Vec<_>>();
    let dark = diffs.iter().map(|diff| *diff <= -threshold).collect::<Vec<_>>();
    if !(has_contiguous_run(&bright, 9) || has_contiguous_run(&dark, 9)) {
        return None;
    }

    Some(diffs.iter().map(|diff| diff.unsigned_abs() as u32).sum())
}

fn has_contiguous_run(flags: &[bool], required_len: usize) -> bool {
    let mut run = 0_usize;
    for idx in 0..(flags.len() * 2) {
        if flags[idx % flags.len()] {
            run += 1;
            if run >= required_len {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn compute_brief_descriptors(
    image: &GrayImage,
    keypoints: &[Keypoint],
    test_pairs: &[TestPair],
) -> Vec<BriefDescriptor> {
    keypoints
        .par_iter()
        .filter_map(|keypoint| {
            if patch_stddev(
                image,
                keypoint.x as i32,
                keypoint.y as i32,
                DESCRIPTOR_TEXTURE_RADIUS_PX,
            ) < MIN_DESCRIPTOR_STDDEV
            {
                return None;
            }
            let mut words = [0_u64; BRIEF_WORDS];
            for (bit_idx, pair) in test_pairs.iter().enumerate() {
                let p0 = patch_average(
                    image,
                    keypoint.x as i32 + pair.p0.0,
                    keypoint.y as i32 + pair.p0.1,
                );
                let p1 = patch_average(
                    image,
                    keypoint.x as i32 + pair.p1.0,
                    keypoint.y as i32 + pair.p1.1,
                );
                if p0 < p1 {
                    words[bit_idx / 64] |= 1_u64 << (bit_idx % 64);
                }
            }
            Some(BriefDescriptor {
                corner: *keypoint,
                words,
                texture_stddev: patch_stddev(
                    image,
                    keypoint.x as i32,
                    keypoint.y as i32,
                    DESCRIPTOR_TEXTURE_RADIUS_PX,
                ),
                octave: 0,
            })
        })
        .collect()
}

fn patch_average(image: &GrayImage, x: i32, y: i32) -> u32 {
    let mut sum = 0_u32;
    let mut count = 0_u32;
    let width = image.width() as i32;
    let height = image.height() as i32;
    for py in (y - BRIEF_AVERAGE_RADIUS_PX)..=(y + BRIEF_AVERAGE_RADIUS_PX) {
        for px in (x - BRIEF_AVERAGE_RADIUS_PX)..=(x + BRIEF_AVERAGE_RADIUS_PX) {
            if px < 0 || py < 0 || px >= width || py >= height {
                continue;
            }
            sum += image.get_pixel(px as u32, py as u32)[0] as u32;
            count += 1;
        }
    }
    if count == 0 {
        return 0;
    }
    sum / count
}

fn patch_stddev(image: &GrayImage, x: i32, y: i32, radius: i32) -> f64 {
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut count = 0.0_f64;
    let width = image.width() as i32;
    let height = image.height() as i32;
    for py in (y - radius)..=(y + radius) {
        for px in (x - radius)..=(x + radius) {
            if px < 0 || py < 0 || px >= width || py >= height {
                continue;
            }
            let value = image.get_pixel(px as u32, py as u32)[0] as f64;
            sum += value;
            sum_sq += value * value;
            count += 1.0;
        }
    }
    if count <= 0.0 {
        return 0.0;
    }
    let mean = sum / count;
    let variance = (sum_sq / count) - mean * mean;
    variance.max(0.0).sqrt()
}

fn downsample_to_gray(image: DynamicImage, max_dim: u32) -> GrayImage {
    if image.width().max(image.height()) <= max_dim {
        return image.to_luma8();
    }
    image
        .resize(max_dim, max_dim, FilterType::Triangle)
        .to_luma8()
}

fn build_deterministic_test_pairs(length: usize) -> Vec<TestPair> {
    // Use a deterministic PRNG and uniqueness checks so descriptor bits are diverse
    // and avoid the short modulo period that collapsed the previous pattern.
    let mut pairs = Vec::with_capacity(length);
    let mut seen = HashSet::with_capacity(length * 2);
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;

    while pairs.len() < length {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x0 = ((state % 31) as i32) - 15;
        let y0 = (((state >> 5) % 31) as i32) - 15;

        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x1 = ((state % 31) as i32) - 15;
        let y1 = (((state >> 5) % 31) as i32) - 15;

        if x0 == x1 && y0 == y1 {
            continue;
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx * dx + dy * dy < 9 {
            continue;
        }

        let key = (x0, y0, x1, y1);
        let reverse_key = (x1, y1, x0, y0);
        if seen.contains(&key) || seen.contains(&reverse_key) {
            continue;
        }
        seen.insert(key);
        pairs.push(TestPair {
            p0: (x0, y0),
            p1: (x1, y1),
        });
    }
    pairs
}

fn build_orb_rbrief_test_pairs(length: usize) -> Vec<TestPair> {
    // Sample test point locations from a 2D Gaussian (sigma ~9 px) using the
    // Box-Muller transform applied to an LCG stream, matching the rBRIEF
    // construction in the ORB paper. sigma is scaled proportionally to the
    // descriptor half-radius (0.4 × 21 px ≈ 9) so that test pairs cover the
    // full patch while remaining centre-weighted.
    let sigma = 9.0_f64;
    let max_offset = (ORB_DESCRIPTOR_RADIUS_PX - BRIEF_AVERAGE_RADIUS_PX - 1) as f64;
    let mut pairs = Vec::with_capacity(length);
    let mut seen = HashSet::with_capacity(length * 2);
    let mut state = 0xDEAD_BEEF_CAFE_1337_u64;

    while pairs.len() < length {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let va = ((state >> 11) as f64 / (1u64 << 53) as f64).max(1e-10);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vb = (state >> 11) as f64 / (1u64 << 53) as f64;
        let x0 = ((-2.0 * va.ln()).sqrt() * (2.0 * std::f64::consts::PI * vb).cos() * sigma)
            .round().clamp(-max_offset, max_offset) as i32;

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vc = ((state >> 11) as f64 / (1u64 << 53) as f64).max(1e-10);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vd = (state >> 11) as f64 / (1u64 << 53) as f64;
        let y0 = ((-2.0 * vc.ln()).sqrt() * (2.0 * std::f64::consts::PI * vd).cos() * sigma)
            .round().clamp(-max_offset, max_offset) as i32;

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let ve = ((state >> 11) as f64 / (1u64 << 53) as f64).max(1e-10);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vf = (state >> 11) as f64 / (1u64 << 53) as f64;
        let x1 = ((-2.0 * ve.ln()).sqrt() * (2.0 * std::f64::consts::PI * vf).cos() * sigma)
            .round().clamp(-max_offset, max_offset) as i32;

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vg = ((state >> 11) as f64 / (1u64 << 53) as f64).max(1e-10);
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let vh = (state >> 11) as f64 / (1u64 << 53) as f64;
        let y1 = ((-2.0 * vg.ln()).sqrt() * (2.0 * std::f64::consts::PI * vh).cos() * sigma)
            .round().clamp(-max_offset, max_offset) as i32;

        if x0 == x1 && y0 == y1 {
            continue;
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx * dx + dy * dy < 9 {
            continue;
        }
        let key = (x0, y0, x1, y1);
        let reverse_key = (x1, y1, x0, y0);
        if seen.contains(&key) || seen.contains(&reverse_key) {
            continue;
        }
        seen.insert(key);
        pairs.push(TestPair { p0: (x0, y0), p1: (x1, y1) });
    }
    pairs
}

fn harris_response(image: &GrayImage, x: i32, y: i32) -> f64 {
    let width = image.width() as i32;
    let height = image.height() as i32;
    let mut ixx = 0.0_f64;
    let mut ixy = 0.0_f64;
    let mut iyy = 0.0_f64;
    for dy in -HARRIS_WINDOW_RADIUS..=HARRIS_WINDOW_RADIUS {
        for dx in -HARRIS_WINDOW_RADIUS..=HARRIS_WINDOW_RADIUS {
            let px = x + dx;
            let py = y + dy;
            if px <= 0 || py <= 0 || px >= width - 1 || py >= height - 1 {
                continue;
            }
            let ix = image.get_pixel((px + 1) as u32, py as u32)[0] as f64
                - image.get_pixel((px - 1) as u32, py as u32)[0] as f64;
            let iy = image.get_pixel(px as u32, (py + 1) as u32)[0] as f64
                - image.get_pixel(px as u32, (py - 1) as u32)[0] as f64;
            ixx += ix * ix;
            ixy += ix * iy;
            iyy += iy * iy;
        }
    }
    let det = ixx * iyy - ixy * ixy;
    let trace = ixx + iyy;
    det - HARRIS_K * trace * trace
}

fn rerank_keypoints_with_harris(image: &GrayImage, keypoints: &mut Vec<Keypoint>) {
    for kp in keypoints.iter_mut() {
        let response = harris_response(image, kp.x as i32, kp.y as i32);
        kp.score = if response > 0.0 {
            (response.sqrt().min(u32::MAX as f64)) as u32
        } else {
            0
        };
    }
    keypoints.retain(|kp| kp.score > 0);
    keypoints.sort_by(|a, b| b.score.cmp(&a.score));
}

fn verify_pair_matches(
    left_frame: &ImageFrame,
    right_frame: &ImageFrame,
    left: &FrameFeatures,
    right: &FrameFeatures,
    distance_threshold: u32,
    float_distance_threshold: f64,
    ratio_threshold: f64,
    geometric_tolerance_px: f64,
    max_octave_diff: Option<u8>,
) -> (PairInlierStats, Vec<[f64; 4]>, Vec<f64>) {
    let empty = PairInlierStats { inlier_count: 0, median_displacement_px: 0.0, model_dx_px: 0.0, model_dy_px: 0.0 };
    if (left.binary_descriptors.is_empty() || right.binary_descriptors.is_empty())
        && (left.float_descriptors.is_empty() || right.float_descriptors.is_empty())
    {
        return (empty, Vec::new(), Vec::new());
    }

    let (candidate_matches, candidates) = if !left.binary_descriptors.is_empty() && !right.binary_descriptors.is_empty() {
        let matches = match_binary_descriptors(
            &left.binary_descriptors,
            &right.binary_descriptors,
            distance_threshold,
            ratio_threshold,
            max_octave_diff,
        );
        let candidates: Vec<PairMatchCandidate> = matches
            .iter()
            .map(|m| {
                let l = &left.binary_descriptors[m.left_idx];
                let r = &right.binary_descriptors[m.right_idx];
                PairMatchCandidate {
                    point: [
                        l.corner.x as f64,
                        l.corner.y as f64,
                        r.corner.x as f64,
                        r.corner.y as f64,
                    ],
                    descriptor_confidence: descriptor_match_confidence(*m),
                    texture_confidence: texture_confidence(l.texture_stddev, r.texture_stddev),
                }
            })
            .collect();
        (matches, candidates)
    } else {
        let matches = match_float_descriptors(
            &left.float_descriptors,
            &right.float_descriptors,
            float_distance_threshold,
            ratio_threshold,
            max_octave_diff,
        );
        let candidates: Vec<PairMatchCandidate> = matches
            .iter()
            .map(|m| {
                let l = &left.float_descriptors[m.left_idx];
                let r = &right.float_descriptors[m.right_idx];
                PairMatchCandidate {
                    point: [
                        l.corner.x as f64,
                        l.corner.y as f64,
                        r.corner.x as f64,
                        r.corner.y as f64,
                    ],
                    descriptor_confidence: descriptor_match_confidence(*m),
                    texture_confidence: texture_confidence(l.texture_stddev, r.texture_stddev),
                }
            })
            .collect();
        (matches, candidates)
    };
    if candidate_matches.is_empty() {
        return (empty, Vec::new(), Vec::new());
    }
    if candidate_matches.len() < MIN_VERIFIED_PAIR_INLIERS {
        return (empty, Vec::new(), Vec::new());
    }
    let match_points: Vec<((f64, f64), (f64, f64))> = candidates
        .iter()
        .map(|c| ((c.point[0], c.point[1]), (c.point[2], c.point[3])))
        .collect();
    let gps_prior = build_gps_pair_prior(left_frame, right_frame);
    let pair_label = pair_label_from_paths(&left_frame.path, &right_frame.path);

    if let Some((fundamental_inliers, residuals)) =
        filter_matches_with_fundamental_ransac(&match_points, geometric_tolerance_px)
    {
        let spatial = spatially_filter_inlier_indices(
            &candidates,
            &fundamental_inliers,
            left.image_width_px,
            left.image_height_px,
            right.image_width_px,
            right.image_height_px,
        );
        let orientation = orientation_parallax_sanity_filter(
            &candidates,
            &spatial,
            gps_prior.as_ref(),
            left.image_width_px,
            left.image_height_px,
            &pair_label,
        );
        if orientation.len() >= FUNDAMENTAL_RANSAC_MIN_INLIERS {
            return summarize_inlier_candidates(
                &candidates,
                &orientation,
                Some(&residuals),
                geometric_tolerance_px,
                left.image_width_px,
                left.image_height_px,
                right.image_width_px,
                right.image_height_px,
            );
        }
    }

    let (stats, inlier_indices, residuals) = robust_translation_inliers(&match_points, geometric_tolerance_px);
    let spatial = spatially_filter_inlier_indices(
        &candidates,
        &inlier_indices,
        left.image_width_px,
        left.image_height_px,
        right.image_width_px,
        right.image_height_px,
    );
    let orientation = orientation_parallax_sanity_filter(
        &candidates,
        &spatial,
        gps_prior.as_ref(),
        left.image_width_px,
        left.image_height_px,
        &pair_label,
    );
    if orientation.len() >= MIN_VERIFIED_PAIR_INLIERS {
        let (_fallback_stats, points, weights) = summarize_inlier_candidates(
            &candidates,
            &orientation,
            Some(&residuals),
            geometric_tolerance_px,
            left.image_width_px,
            left.image_height_px,
            right.image_width_px,
            right.image_height_px,
        );
        if points.len() < MIN_VERIFIED_PAIR_INLIERS {
            return (empty, Vec::new(), Vec::new());
        }

        let (median_displacement_px, model_dx_px, model_dy_px) = summarize_displacements(&points);
        let mut final_stats = stats;
        final_stats.inlier_count = points.len();
        final_stats.median_displacement_px = median_displacement_px;
        final_stats.model_dx_px = model_dx_px;
        final_stats.model_dy_px = model_dy_px;
        (final_stats, points, weights)
    } else {
        (empty, Vec::new(), Vec::new())
    }
}

fn match_binary_descriptors(
    left: &[BriefDescriptor],
    right: &[BriefDescriptor],
    distance_threshold: u32,
    ratio_threshold: f64,
    max_octave_diff: Option<u8>,
) -> Vec<DescriptorMatch> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }

    let best_right_for_left = left
        .par_iter()
        .map(|left_descriptor| {
            best_two_binary_matches(left_descriptor, right, max_octave_diff).and_then(
                |(best_idx, best_dist, second_dist)| {
                    if best_dist > distance_threshold as f64 {
                        return None;
                    }
                    if !passes_ratio_test(best_dist, second_dist, ratio_threshold, right.len()) {
                        return None;
                    }
                    Some((best_idx, best_dist, second_dist))
                },
            )
        })
        .collect::<Vec<_>>();

    let best_left_for_right = right
        .par_iter()
        .map(|right_descriptor| {
            best_two_binary_matches(right_descriptor, left, max_octave_diff).and_then(
                |(best_idx, best_dist, second_dist)| {
                    if best_dist > distance_threshold as f64 {
                        return None;
                    }
                    if !passes_ratio_test(best_dist, second_dist, ratio_threshold, left.len()) {
                        return None;
                    }
                    Some((best_idx, best_dist, second_dist))
                },
            )
        })
        .collect::<Vec<_>>();

    let mut matches = Vec::new();
    for (left_idx, candidate) in best_right_for_left.iter().enumerate() {
        let Some((right_idx, best_dist, second_dist)) = candidate else {
            continue;
        };
        if let Some((back_left_idx, _, _)) = best_left_for_right[*right_idx] {
            if back_left_idx == left_idx {
                matches.push(DescriptorMatch {
                    left_idx,
                    right_idx: *right_idx,
                    best_dist: *best_dist,
                    second_dist: *second_dist,
                    metric: FeatureDistanceMetric::Hamming,
                });
            }
        }
    }

    matches
}

fn match_float_descriptors(
    left: &[FloatDescriptor],
    right: &[FloatDescriptor],
    distance_threshold: f64,
    ratio_threshold: f64,
    max_octave_diff: Option<u8>,
) -> Vec<DescriptorMatch> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }

    let best_right_for_left = left
        .par_iter()
        .map(|left_descriptor| {
            best_two_float_matches(left_descriptor, right, max_octave_diff).and_then(
                |(best_idx, best_dist, second_dist)| {
                    if best_dist > distance_threshold {
                        return None;
                    }
                    if !passes_ratio_test(best_dist, second_dist, ratio_threshold, right.len()) {
                        return None;
                    }
                    Some((best_idx, best_dist, second_dist))
                },
            )
        })
        .collect::<Vec<_>>();

    let best_left_for_right = right
        .par_iter()
        .map(|right_descriptor| {
            best_two_float_matches(right_descriptor, left, max_octave_diff).and_then(
                |(best_idx, best_dist, second_dist)| {
                    if best_dist > distance_threshold {
                        return None;
                    }
                    if !passes_ratio_test(best_dist, second_dist, ratio_threshold, left.len()) {
                        return None;
                    }
                    Some((best_idx, best_dist, second_dist))
                },
            )
        })
        .collect::<Vec<_>>();

    let mut matches = Vec::new();
    for (left_idx, candidate) in best_right_for_left.iter().enumerate() {
        let Some((right_idx, best_dist, second_dist)) = candidate else {
            continue;
        };
        if let Some((back_left_idx, _, _)) = best_left_for_right[*right_idx] {
            if back_left_idx == left_idx {
                matches.push(DescriptorMatch {
                    left_idx,
                    right_idx: *right_idx,
                    best_dist: *best_dist,
                    second_dist: *second_dist,
                    metric: FeatureDistanceMetric::EuclideanL2,
                });
            }
        }
    }

    matches
}

fn best_two_binary_matches(
    query: &BriefDescriptor,
    candidates: &[BriefDescriptor],
    max_octave_diff: Option<u8>,
) -> Option<(usize, f64, Option<f64>)> {
    if candidates.is_empty() {
        return None;
    }

    let mut best_idx = usize::MAX;
    let mut best_dist = f64::INFINITY;
    let mut second_dist = f64::INFINITY;

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(max_diff) = max_octave_diff {
            if query.octave.abs_diff(candidate.octave) > max_diff {
                continue;
            }
        }
        let dist = hamming_distance(query, candidate) as f64;
        if dist < best_dist {
            second_dist = best_dist;
            best_dist = dist;
            best_idx = idx;
        } else if dist < second_dist {
            second_dist = dist;
        }
    }

    if best_idx == usize::MAX {
        return None;
    }

    let second = if !second_dist.is_finite() {
        None
    } else {
        Some(second_dist)
    };
    Some((best_idx, best_dist, second))
}

fn best_two_float_matches(
    query: &FloatDescriptor,
    candidates: &[FloatDescriptor],
    max_octave_diff: Option<u8>,
) -> Option<(usize, f64, Option<f64>)> {
    if candidates.is_empty() {
        return None;
    }

    let mut best_idx = usize::MAX;
    let mut best_dist = f64::INFINITY;
    let mut second_dist = f64::INFINITY;

    for (idx, candidate) in candidates.iter().enumerate() {
        if let Some(max_diff) = max_octave_diff {
            if query.octave.abs_diff(candidate.octave) > max_diff {
                continue;
            }
        }
        let dist = euclidean_descriptor_distance(query, candidate);
        if dist < best_dist {
            second_dist = best_dist;
            best_dist = dist;
            best_idx = idx;
        } else if dist < second_dist {
            second_dist = dist;
        }
    }

    if best_idx == usize::MAX {
        return None;
    }

    let second = if !second_dist.is_finite() {
        None
    } else {
        Some(second_dist)
    };
    Some((best_idx, best_dist, second))
}

fn passes_ratio_test(
    best_dist: f64,
    second_dist: Option<f64>,
    ratio_threshold: f64,
    candidate_count: usize,
) -> bool {
    let Some(second) = second_dist else {
        return candidate_count == 1 && best_dist <= 1.0e-9;
    };
    if second <= 1.0e-9 {
        return best_dist <= 1.0e-9;
    }
    (best_dist / second) <= ratio_threshold
}

fn hamming_distance(left: &BriefDescriptor, right: &BriefDescriptor) -> u32 {
    (left.words[0] ^ right.words[0]).count_ones()
        + (left.words[1] ^ right.words[1]).count_ones()
        + (left.words[2] ^ right.words[2]).count_ones()
        + (left.words[3] ^ right.words[3]).count_ones()
}

fn euclidean_descriptor_distance(left: &FloatDescriptor, right: &FloatDescriptor) -> f64 {
    let mut sum = 0.0_f64;
    for idx in 0..SIFT_DESCRIPTOR_LEN {
        let diff = left.values[idx] as f64 - right.values[idx] as f64;
        sum += diff * diff;
    }
    sum.sqrt()
}

fn robust_translation_inliers(
    match_points: &[((f64, f64), (f64, f64))],
    geometric_tolerance_px: f64,
) -> (PairInlierStats, Vec<usize>, Vec<f64>) {
    let empty_stats = PairInlierStats { inlier_count: 0, median_displacement_px: 0.0, model_dx_px: 0.0, model_dy_px: 0.0 };
    if match_points.len() < 2 {
        return (empty_stats, Vec::new(), Vec::new());
    }

    let max_hypotheses = 256_usize;
    let pair_count = match_points.len();
    let stride = ((pair_count * pair_count) / max_hypotheses).max(1);
    let mut best_count = 0_usize;
    let mut best_error_sum = f64::INFINITY;
    let mut best_model = SimilarityModel {
        a: 1.0,
        b: 0.0,
        tx: 0.0,
        ty: 0.0,
    };

    let mut pair_idx = 0usize;
    for i in 0..pair_count {
        for j in (i + 1)..pair_count {
            if pair_idx % stride != 0 {
                pair_idx += 1;
                continue;
            }
            pair_idx += 1;
            let Some(model) = similarity_model_from_two_pairs(match_points[i], match_points[j]) else {
                continue;
            };

            let mut count = 0_usize;
            let mut error_sum = 0.0_f64;
            for point in match_points {
                let err = similarity_residual(model, *point);
                if err <= geometric_tolerance_px {
                    count += 1;
                    error_sum += err;
                }
            }

            if count > best_count || (count == best_count && error_sum < best_error_sum) {
                best_count = count;
                best_error_sum = error_sum;
                best_model = model;
            }
        }
    }

    if best_count == 0 {
        return (empty_stats, Vec::new(), Vec::new());
    }

    let mut inlier_indices = Vec::with_capacity(best_count);
    let mut residuals = vec![f64::INFINITY; match_points.len()];
    for (idx, ((plx, ply), (prx, pry))) in match_points.iter().enumerate() {
        let residual = similarity_residual(best_model, ((*plx, *ply), (*prx, *pry)));
        if residual <= geometric_tolerance_px {
            inlier_indices.push(idx);
            residuals[idx] = residual;
        }
    }
    let points: Vec<[f64; 4]> = inlier_indices
        .iter()
        .map(|&idx| {
            let ((x1, y1), (x2, y2)) = match_points[idx];
            [x1, y1, x2, y2]
        })
        .collect();
    let (stats, _, _) = summarize_displacements(&points);
    (
        PairInlierStats {
            inlier_count: inlier_indices.len(),
            median_displacement_px: stats,
            model_dx_px: summarize_displacements(&points).1,
            model_dy_px: summarize_displacements(&points).2,
        },
        inlier_indices,
        residuals,
    )
}
fn summarize_displacements(points: &[[f64; 4]]) -> (f64, f64, f64) {
    let mut inlier_displacements = Vec::with_capacity(points.len());
    let mut inlier_dx = Vec::with_capacity(points.len());
    let mut inlier_dy = Vec::with_capacity(points.len());
    for p in points {
        let dx = p[2] - p[0];
        let dy = p[3] - p[1];
        inlier_displacements.push((dx * dx + dy * dy).sqrt());
        inlier_dx.push(dx);
        inlier_dy.push(dy);
    }
    inlier_displacements.sort_by(|a, b| a.total_cmp(b));
    inlier_dx.sort_by(|a, b| a.total_cmp(b));
    inlier_dy.sort_by(|a, b| a.total_cmp(b));

    (
        inlier_displacements[inlier_displacements.len() / 2],
        inlier_dx[inlier_dx.len() / 2],
        inlier_dy[inlier_dy.len() / 2],
    )
}

fn summarize_inlier_candidates(
    candidates: &[PairMatchCandidate],
    indices: &[usize],
    residuals: Option<&[f64]>,
    residual_threshold: f64,
    left_width: u32,
    left_height: u32,
    right_width: u32,
    right_height: u32,
) -> (PairInlierStats, Vec<[f64; 4]>, Vec<f64>) {
    let empty_stats = PairInlierStats { inlier_count: 0, median_displacement_px: 0.0, model_dx_px: 0.0, model_dy_px: 0.0 };
    if indices.is_empty() {
        return (empty_stats, Vec::new(), Vec::new());
    }
    let points: Vec<[f64; 4]> = indices.iter().map(|&idx| candidates[idx].point).collect();
    let (median_displacement_px, model_dx_px, model_dy_px) = summarize_displacements(&points);
    let spatial_penalty = spatial_distribution_penalty(
        &points,
        left_width,
        left_height,
        right_width,
        right_height,
    );
    let (dominant_angle, median_mag) = dominant_motion_summary(candidates, indices);
    let weights: Vec<f64> = indices
        .iter()
        .map(|&idx| {
            let descriptor = candidates[idx].descriptor_confidence;
            let texture = candidates[idx].texture_confidence;
            let geometric = residuals
                .and_then(|vals| vals.get(idx).copied())
                .filter(|r| r.is_finite())
                .map(|r| (1.0 - r / residual_threshold.max(1e-6)).clamp(0.2, 1.0))
                .unwrap_or(0.6);
            let orientation = orientation_confidence(candidates[idx].point, dominant_angle, median_mag);
            (descriptor * texture * geometric * orientation * spatial_penalty).clamp(0.08, 1.0)
        })
        .collect();

    let mut sorted_weights = weights.clone();
    sorted_weights.sort_by(|a, b| a.total_cmp(b));
    let median_weight = sorted_weights[sorted_weights.len() / 2];
    let min_required_inliers = adaptive_min_pair_inliers(spatial_penalty, median_weight);
    if points.len() < min_required_inliers {
        return (empty_stats, Vec::new(), Vec::new());
    }

    (
        PairInlierStats {
            inlier_count: points.len(),
            median_displacement_px,
            model_dx_px,
            model_dy_px,
        },
        points,
        weights,
    )
}

fn adaptive_min_pair_inliers(spatial_penalty: f64, median_weight: f64) -> usize {
    let spatial_floor = if spatial_penalty < 0.42 {
        3
    } else if spatial_penalty < 0.62 {
        2
    } else {
        2
    };

    let confidence_floor = if median_weight < 0.22 {
        3
    } else if median_weight < 0.38 {
        2
    } else {
        2
    };

    spatial_floor.max(confidence_floor)
}

fn spatially_filter_inlier_indices(
    candidates: &[PairMatchCandidate],
    indices: &[usize],
    left_width: u32,
    left_height: u32,
    right_width: u32,
    right_height: u32,
) -> Vec<usize> {
    if indices.len() < MIN_VERIFIED_PAIR_INLIERS {
        return Vec::new();
    }

    let cell_count = SPATIAL_GRID_COLS * SPATIAL_GRID_ROWS;
    let mut left_cell_counts = vec![0usize; cell_count];
    let mut right_cell_counts = vec![0usize; cell_count];
    let mut kept = Vec::with_capacity(indices.len());

    let mut left_min_x = f64::INFINITY;
    let mut left_max_x = f64::NEG_INFINITY;
    let mut left_min_y = f64::INFINITY;
    let mut left_max_y = f64::NEG_INFINITY;
    let mut right_min_x = f64::INFINITY;
    let mut right_max_x = f64::NEG_INFINITY;
    let mut right_min_y = f64::INFINITY;
    let mut right_max_y = f64::NEG_INFINITY;

    for &idx in indices {
        let p = candidates[idx].point;
        let left_cell = spatial_cell_index(p[0], p[1], left_width, left_height);
        let right_cell = spatial_cell_index(p[2], p[3], right_width, right_height);
        if left_cell_counts[left_cell] >= SPATIAL_MAX_MATCHES_PER_CELL
            || right_cell_counts[right_cell] >= SPATIAL_MAX_MATCHES_PER_CELL
        {
            continue;
        }

        left_cell_counts[left_cell] += 1;
        right_cell_counts[right_cell] += 1;
        left_min_x = left_min_x.min(p[0]);
        left_max_x = left_max_x.max(p[0]);
        left_min_y = left_min_y.min(p[1]);
        left_max_y = left_max_y.max(p[1]);
        right_min_x = right_min_x.min(p[2]);
        right_max_x = right_max_x.max(p[2]);
        right_min_y = right_min_y.min(p[3]);
        right_max_y = right_max_y.max(p[3]);
        kept.push(idx);
    }

    if kept.len() < MIN_VERIFIED_PAIR_INLIERS {
        return Vec::new();
    }

    kept
}

fn orientation_parallax_sanity_filter(
    candidates: &[PairMatchCandidate],
    indices: &[usize],
    gps_prior: Option<&GpsPairPrior>,
    left_width_px: u32,
    left_height_px: u32,
    pair_label: &str,
) -> Vec<usize> {
    if indices.len() < MIN_VERIFIED_PAIR_INLIERS {
        return Vec::new();
    }

    let mut dxs = Vec::with_capacity(indices.len());
    let mut dys = Vec::with_capacity(indices.len());
    let mut mags = Vec::with_capacity(indices.len());
    for &idx in indices {
        let p = candidates[idx].point;
        let dx = p[2] - p[0];
        let dy = p[3] - p[1];
        dxs.push(dx);
        dys.push(dy);
        mags.push((dx * dx + dy * dy).sqrt());
    }
    dxs.sort_by(|a, b| a.total_cmp(b));
    dys.sort_by(|a, b| a.total_cmp(b));
    mags.sort_by(|a, b| a.total_cmp(b));
    let median_dx = dxs[dxs.len() / 2];
    let median_dy = dys[dys.len() / 2];
    let median_mag = mags[mags.len() / 2].max(1.0);
    let dominant_angle = median_dy.atan2(median_dx);

    let mut angular_diffs = Vec::with_capacity(indices.len());
    for &idx in indices {
        let p = candidates[idx].point;
        let dx = p[2] - p[0];
        let dy = p[3] - p[1];
        let angle = dy.atan2(dx);
        let mut diff = (angle - dominant_angle).abs();
        while diff > std::f64::consts::PI {
            diff -= std::f64::consts::PI;
        }
        angular_diffs.push(diff);
    }
    angular_diffs.sort_by(|a, b| a.total_cmp(b));
    let median_ang_diff = angular_diffs[angular_diffs.len() / 2];
    let mut mad_vals: Vec<f64> = angular_diffs
        .iter()
        .map(|d| (d - median_ang_diff).abs())
        .collect();
    mad_vals.sort_by(|a, b| a.total_cmp(b));
    let mad = mad_vals[mad_vals.len() / 2].max(1.0e-6);
    let p75_ang_diff = angular_diffs[(angular_diffs.len() * 3) / 4];
    // For large-rotation / cross-flightline pairs, valid correspondences span a
    // broad range of directions — they converge to an epipole rather than being
    // parallel. Delegate to a dedicated convergence-point filter for that case.
    let mut has_strong_directional_consensus = p75_ang_diff <= 0.65 || median_ang_diff <= 0.35;
    if let Some(prior) = gps_prior {
        // GPS baseline helps distinguish translation-dominant pairs from
        // cross-flightline pairs where convergence should dominate.
        if prior.baseline_ratio >= 0.95 && p75_ang_diff > 0.45 {
            has_strong_directional_consensus = false;
        } else if prior.baseline_ratio <= 0.35 && median_ang_diff <= 0.55 {
            has_strong_directional_consensus = true;
        }
    }
    if !has_strong_directional_consensus {
        gps_filter_debug_line(format!(
            "pair={} branch=convergence n={} median_ang_diff={:.3} p75_ang_diff={:.3} gps_baseline_ratio={}",
            pair_label,
            indices.len(),
            median_ang_diff,
            p75_ang_diff,
            gps_prior
                .map(|p| format!("{:.3}", p.baseline_ratio))
                .unwrap_or_else(|| "none".to_string())
        ));
        return convergence_point_filter(
            candidates,
            indices,
            gps_prior,
            left_width_px,
            left_height_px,
            pair_label,
        );
    }

    let n = indices.len();
    let base_angle_cap = if n <= 20 {
        0.40
    } else if n <= 40 {
        0.55
    } else {
        0.75
    };
    let mut adaptive_angle_cap = (median_ang_diff + 2.6 * mad)
        .clamp(base_angle_cap, 0.95);
    if let Some(prior) = gps_prior {
        // Use GPS airbase bearing as a weak prior on dominant displacement
        // direction (pi-periodic because line direction is unsigned).
        let bearing_diff = angle_diff_pi(dominant_angle, prior.bearing_rad);
        if prior.baseline_ratio <= 0.60 && bearing_diff <= 0.45 {
            adaptive_angle_cap = (adaptive_angle_cap * 0.88).max(base_angle_cap * 0.85);
        } else if prior.baseline_ratio <= 0.60 && bearing_diff >= 1.05 {
            adaptive_angle_cap = (adaptive_angle_cap * 1.12).min(1.05);
        }
    }
    let (min_mag_ratio, max_mag_ratio) = if n <= 20 {
        (0.35, 2.8)
    } else if n <= 40 {
        (0.28, 3.4)
    } else {
        (0.22, 4.0)
    };

    let kept: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&idx| {
            let p = candidates[idx].point;
            let dx = p[2] - p[0];
            let dy = p[3] - p[1];
            let mag = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx);
            let mut diff = (angle - dominant_angle).abs();
            while diff > std::f64::consts::PI {
                diff -= std::f64::consts::PI;
            }
            let mag_ratio = mag / median_mag;
            diff <= adaptive_angle_cap && (min_mag_ratio..=max_mag_ratio).contains(&mag_ratio)
        })
        .collect();

    gps_filter_debug_line(format!(
        "pair={} branch=parallel n={} kept={} angle_cap={:.3} mag_ratio=[{:.2},{:.2}] gps_baseline_ratio={} bearing_diff={}",
        pair_label,
        n,
        kept.len(),
        adaptive_angle_cap,
        min_mag_ratio,
        max_mag_ratio,
        gps_prior
            .map(|p| format!("{:.3}", p.baseline_ratio))
            .unwrap_or_else(|| "none".to_string()),
        gps_prior
            .map(|p| format!("{:.3}", angle_diff_pi(dominant_angle, p.bearing_rad)))
            .unwrap_or_else(|| "none".to_string())
    ));

    if kept.len() >= MIN_VERIFIED_PAIR_INLIERS {
        kept
    } else {
        let mut ranked: Vec<(usize, f64)> = indices
            .iter()
            .copied()
            .map(|idx| {
                let p = candidates[idx].point;
                let dx = p[2] - p[0];
                let dy = p[3] - p[1];
                let angle = dy.atan2(dx);
                let mut diff = (angle - dominant_angle).abs();
                while diff > std::f64::consts::PI {
                    diff -= std::f64::consts::PI;
                }
                (idx, diff)
            })
            .collect();
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
        ranked
            .into_iter()
            .take(indices.len().min(MIN_VERIFIED_PAIR_INLIERS))
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// Filter matches for cross-flightline (large-rotation) pairs by checking
/// that each match's displacement line passes near a common convergence point.
///
/// When two images share significant rotation rather than pure translation the
/// epipole (projection of one camera centre into the other image) lies at a
/// finite location.  Valid match vectors all point *toward* that epipole, so
/// the lines defined by (left_keypoint, displacement_direction) converge to a
/// single point.  Outliers whose lines deviate substantially from that point
/// are rejected.
///
/// Algorithm (all in left-image coordinate space):
///   1. Represent each match as a line through `(lx, ly)` with direction
///      `(dx, dy) = (rx − lx, ry − ly)`.
///   2. Fit the convergence point C = (cx, cy) with 2 × 2 least squares on the
///      normal-form line equations.
///   3. Keep matches whose perpendicular distance from C is within
///      `median_dist + 3 × MAD`, plus a magnitude-ratio sanity check.
fn convergence_point_filter(
    candidates: &[PairMatchCandidate],
    indices: &[usize],
    gps_prior: Option<&GpsPairPrior>,
    left_width_px: u32,
    left_height_px: u32,
    pair_label: &str,
) -> Vec<usize> {
    // Accumulate AᵀA and Aᵀb for the system of normal-form line equations.
    // Line i: (-dy)·x + (dx)·y = (-dy)·lx + (dx)·ly
    let mut ata00 = 0.0_f64;
    let mut ata01 = 0.0_f64;
    let mut ata11 = 0.0_f64;
    let mut atb0 = 0.0_f64;
    let mut atb1 = 0.0_f64;
    for &idx in indices {
        let p = candidates[idx].point;
        let (lx, ly) = (p[0], p[1]);
        let dx = p[2] - lx;
        let dy = p[3] - ly;
        if dx * dx + dy * dy < 1.0 {
            continue;
        }
        let a0 = -dy; // normal component x
        let a1 = dx;  // normal component y
        let b = a0 * lx + a1 * ly;
        ata00 += a0 * a0;
        ata01 += a0 * a1;
        ata11 += a1 * a1;
        atb0 += a0 * b;
        atb1 += a1 * b;
    }
    let det = ata00 * ata11 - ata01 * ata01;
    if det.abs() < 1.0e-4 {
        // Near-singular: lines are nearly parallel despite weak angular consensus.
        // This is unusual but fall back to returning all indices.
        return indices.to_vec();
    }
    let cx = (atb0 * ata11 - atb1 * ata01) / det;
    let cy = (ata00 * atb1 - ata01 * atb0) / det;

    // Perpendicular distance of each match line from the convergence point.
    let mut dist_pairs: Vec<(usize, f64)> = indices
        .iter()
        .copied()
        .map(|idx| {
            let p = candidates[idx].point;
            let (lx, ly) = (p[0], p[1]);
            let dx = p[2] - lx;
            let dy = p[3] - ly;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            // dist = |cross(d, lp − C)| / |d|
            let dist = (dx * (ly - cy) - dy * (lx - cx)).abs() / len;
            (idx, dist)
        })
        .collect();
    dist_pairs.sort_by(|a, b| a.1.total_cmp(&b.1));
    let n = dist_pairs.len();
    let median_dist = dist_pairs[n / 2].1;

    // MAD-based adaptive threshold (floor of 3 px so tiny images still work).
    let mut mad_vals: Vec<f64> = dist_pairs
        .iter()
        .map(|(_, d)| (d - median_dist).abs())
        .collect();
    mad_vals.sort_by(|a, b| a.total_cmp(b));
    let mad_dist = mad_vals[n / 2].max(3.0);
    let mut dist_threshold = median_dist + 3.0 * mad_dist;

    if let Some(prior) = gps_prior {
        // GPS-derived airbase ratio indicates whether finite convergence is
        // plausible. Use it to tighten or relax the epipole consistency gate.
        let cx_ref = left_width_px as f64 * 0.5;
        let cy_ref = left_height_px as f64 * 0.5;
        let diag = ((left_width_px as f64).hypot(left_height_px as f64)).max(1.0);
        let epipole_dist_norm = ((cx - cx_ref).hypot(cy - cy_ref)) / diag;
        if prior.baseline_ratio >= 0.60 {
            if epipole_dist_norm > 6.0 {
                dist_threshold *= 0.82;
            } else if epipole_dist_norm < 2.5 {
                dist_threshold *= 1.08;
            }
        } else if epipole_dist_norm < 0.30 {
            dist_threshold *= 0.82;
        }
    }

    // Magnitude consistency (wide bounds — scale difference is expected across
    // flightlines but should still be roughly uniform across all matches).
    let mut sorted_mags: Vec<f64> = indices
        .iter()
        .map(|&idx| {
            let p = candidates[idx].point;
            let dx = p[2] - p[0];
            let dy = p[3] - p[1];
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    sorted_mags.sort_by(|a, b| a.total_cmp(b));
    let median_mag = sorted_mags[n / 2].max(1.0);

    let kept: Vec<usize> = dist_pairs
        .iter()
        .filter(|(idx, dist)| {
            let p = candidates[*idx].point;
            let dx = p[2] - p[0];
            let dy = p[3] - p[1];
            let mag_ratio = (dx * dx + dy * dy).sqrt() / median_mag;
            *dist <= dist_threshold && (0.12..=7.5).contains(&mag_ratio)
        })
        .map(|(idx, _)| *idx)
        .collect();

    gps_filter_debug_line(format!(
        "pair={} branch=convergence-fit n={} kept={} epipole=({:.1},{:.1}) dist_thresh={:.2} median_dist={:.2} gps_baseline_ratio={}",
        pair_label,
        n,
        kept.len(),
        cx,
        cy,
        dist_threshold,
        median_dist,
        gps_prior
            .map(|p| format!("{:.3}", p.baseline_ratio))
            .unwrap_or_else(|| "none".to_string())
    ));

    if kept.len() >= MIN_VERIFIED_PAIR_INLIERS {
        kept
    } else {
        indices.to_vec()
    }
}

fn dominant_motion_summary(
    candidates: &[PairMatchCandidate],
    indices: &[usize],
) -> (f64, f64) {
    let mut dxs = Vec::with_capacity(indices.len());
    let mut dys = Vec::with_capacity(indices.len());
    let mut mags = Vec::with_capacity(indices.len());
    for &idx in indices {
        let p = candidates[idx].point;
        let dx = p[2] - p[0];
        let dy = p[3] - p[1];
        dxs.push(dx);
        dys.push(dy);
        mags.push((dx * dx + dy * dy).sqrt());
    }
    dxs.sort_by(|a, b| a.total_cmp(b));
    dys.sort_by(|a, b| a.total_cmp(b));
    mags.sort_by(|a, b| a.total_cmp(b));
    let median_dx = dxs[dxs.len() / 2];
    let median_dy = dys[dys.len() / 2];
    let median_mag = mags[mags.len() / 2].max(1.0);
    (median_dy.atan2(median_dx), median_mag)
}

fn build_gps_pair_prior(left: &ImageFrame, right: &ImageFrame) -> Option<GpsPairPrior> {
    let left_gps = left.metadata.gps.as_ref()?;
    let right_gps = right.metadata.gps.as_ref()?;

    let baseline_m = great_circle_distance_m(left_gps.lat, left_gps.lon, right_gps.lat, right_gps.lon);
    let avg_footprint = 0.5 * (estimate_frame_footprint_width_m(left) + estimate_frame_footprint_width_m(right));
    let baseline_ratio = baseline_m / avg_footprint.max(1.0);
    let bearing_rad = initial_bearing_rad(left_gps.lat, left_gps.lon, right_gps.lat, right_gps.lon);

    Some(GpsPairPrior {
        baseline_ratio,
        bearing_rad,
    })
}

fn angle_diff_pi(a: f64, b: f64) -> f64 {
    let mut diff = (a - b).abs();
    while diff > std::f64::consts::PI {
        diff -= std::f64::consts::PI;
    }
    if diff > std::f64::consts::FRAC_PI_2 {
        std::f64::consts::PI - diff
    } else {
        diff
    }
}

fn should_attempt_pair(
    left: &ImageFrame,
    right: &ImageFrame,
    left_idx: usize,
    right_idx: usize,
    gps_pair_footprint_multiplier: f64,
) -> bool {
    // Always keep adjacent-frame attempts; this preserves the strongest
    // sequential motion constraints even when GPS is noisy.
    if right_idx == left_idx + 1 {
        return true;
    }

    let (left_gps, right_gps) = match (left.metadata.gps.as_ref(), right.metadata.gps.as_ref()) {
        (Some(a), Some(b)) => (a, b),
        _ => return true, // Fallback: no GPS on either frame means no filtering.
    };

    let spacing_m = great_circle_distance_m(left_gps.lat, left_gps.lon, right_gps.lat, right_gps.lon);
    let left_footprint = estimate_frame_footprint_width_m(left);
    let right_footprint = estimate_frame_footprint_width_m(right);
    let max_spacing_m = gps_pair_footprint_multiplier * 0.5 * (left_footprint + right_footprint);

    spacing_m <= max_spacing_m
}

fn estimate_frame_footprint_width_m(frame: &ImageFrame) -> f64 {
    let altitude_m = frame
        .metadata
        .gps
        .as_ref()
        .map(|gps| gps.alt.abs())
        .unwrap_or(1.0)
        .max(1.0);

    if let Some(focal_mm) = frame.metadata.focal_length_mm {
        if focal_mm > 0.0 {
            let sensor_width_mm = frame
                .metadata
                .sensor_width_mm
                .unwrap_or(GPS_DEFAULT_SENSOR_WIDTH_MM)
                .max(1.0);
            return (altitude_m * (sensor_width_mm / focal_mm)).max(1.0);
        }
    }

    (2.0 * altitude_m * (0.5 * GPS_DEFAULT_HFOV_DEG.to_radians()).tan()).max(1.0)
}

fn great_circle_distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_378_137.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let a = (dlat * 0.5).sin().powi(2)
        + lat1r.cos() * lat2r.cos() * (dlon * 0.5).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

fn initial_bearing_rad(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1r = lat1.to_radians();
    let lat2r = lat2.to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let y = dlon.sin() * lat2r.cos();
    let x = lat1r.cos() * lat2r.sin() - lat1r.sin() * lat2r.cos() * dlon.cos();
    y.atan2(x)
}

fn orientation_confidence(point: [f64; 4], dominant_angle: f64, median_mag: f64) -> f64 {
    let dx = point[2] - point[0];
    let dy = point[3] - point[1];
    let mag = (dx * dx + dy * dy).sqrt().max(1.0e-6);
    let angle = dy.atan2(dx);
    let mut diff = (angle - dominant_angle).abs();
    while diff > std::f64::consts::PI {
        diff -= std::f64::consts::PI;
    }
    let angle_penalty = (1.0 - diff / 1.25).clamp(0.35, 1.0);
    let mag_ratio = mag / median_mag.max(1.0e-6);
    let mag_penalty = if mag_ratio < 0.35 {
        (mag_ratio / 0.35).clamp(0.35, 1.0)
    } else if mag_ratio > 2.8 {
        (2.8 / mag_ratio).clamp(0.35, 1.0)
    } else {
        1.0
    };
    (angle_penalty * mag_penalty).clamp(0.2, 1.0)
}

fn spatial_distribution_penalty(
    points: &[[f64; 4]],
    left_width: u32,
    left_height: u32,
    right_width: u32,
    right_height: u32,
) -> f64 {
    if points.is_empty() {
        return 0.3;
    }

    let cell_count = SPATIAL_GRID_COLS * SPATIAL_GRID_ROWS;
    let mut left_cells = vec![0usize; cell_count];
    let mut right_cells = vec![0usize; cell_count];
    let mut left_min_x = f64::INFINITY;
    let mut left_max_x = f64::NEG_INFINITY;
    let mut left_min_y = f64::INFINITY;
    let mut left_max_y = f64::NEG_INFINITY;
    let mut right_min_x = f64::INFINITY;
    let mut right_max_x = f64::NEG_INFINITY;
    let mut right_min_y = f64::INFINITY;
    let mut right_max_y = f64::NEG_INFINITY;

    for p in points {
        left_cells[spatial_cell_index(p[0], p[1], left_width, left_height)] += 1;
        right_cells[spatial_cell_index(p[2], p[3], right_width, right_height)] += 1;
        left_min_x = left_min_x.min(p[0]);
        left_max_x = left_max_x.max(p[0]);
        left_min_y = left_min_y.min(p[1]);
        left_max_y = left_max_y.max(p[1]);
        right_min_x = right_min_x.min(p[2]);
        right_max_x = right_max_x.max(p[2]);
        right_min_y = right_min_y.min(p[3]);
        right_max_y = right_max_y.max(p[3]);
    }

    let left_occ = left_cells.iter().filter(|&&c| c > 0).count() as f64 / SPATIAL_MIN_OCCUPIED_CELLS as f64;
    let right_occ = right_cells.iter().filter(|&&c| c > 0).count() as f64 / SPATIAL_MIN_OCCUPIED_CELLS as f64;
    let left_spread_x = (left_max_x - left_min_x) / left_width.max(1) as f64;
    let left_spread_y = (left_max_y - left_min_y) / left_height.max(1) as f64;
    let right_spread_x = (right_max_x - right_min_x) / right_width.max(1) as f64;
    let right_spread_y = (right_max_y - right_min_y) / right_height.max(1) as f64;
    let spread_support = (left_spread_x / SPATIAL_MIN_AXIS_SPREAD_FRACTION)
        .min(left_spread_y / SPATIAL_MIN_AXIS_SPREAD_FRACTION)
        .min(right_spread_x / SPATIAL_MIN_AXIS_SPREAD_FRACTION)
        .min(right_spread_y / SPATIAL_MIN_AXIS_SPREAD_FRACTION)
        .clamp(0.0, 1.4);
    let occupancy_support = left_occ.min(right_occ).clamp(0.0, 1.4);
    (0.45 + 0.35 * occupancy_support.min(1.0) + 0.20 * spread_support.min(1.0)).clamp(0.45, 1.0)
}

fn descriptor_match_confidence(m: DescriptorMatch) -> f64 {
    let distance_quality = match m.metric {
        FeatureDistanceMetric::Hamming => 1.0 - (m.best_dist / (BRIEF_WORDS as f64 * 64.0)),
        FeatureDistanceMetric::EuclideanL2 => 1.0 - (m.best_dist / 1.4143),
        FeatureDistanceMetric::Cosine => 1.0 - (m.best_dist / 2.0),
    };
    let distinctiveness = m
        .second_dist
        .map(|second| {
            if second <= 1.0e-9 {
                1.0
            } else {
                (1.0 - (m.best_dist / second)).clamp(0.0, 1.0)
            }
        })
        .unwrap_or(1.0);
    (0.55 * distance_quality + 0.45 * distinctiveness).clamp(0.1, 1.0)
}

fn texture_confidence(left_stddev: f64, right_stddev: f64) -> f64 {
    (((left_stddev + right_stddev) * 0.5) / 24.0).clamp(0.2, 1.0)
}

fn spatial_cell_index(x: f64, y: f64, width: u32, height: u32) -> usize {
    let col = ((x / width.max(1) as f64) * SPATIAL_GRID_COLS as f64)
        .floor()
        .clamp(0.0, (SPATIAL_GRID_COLS - 1) as f64) as usize;
    let row = ((y / height.max(1) as f64) * SPATIAL_GRID_ROWS as f64)
        .floor()
        .clamp(0.0, (SPATIAL_GRID_ROWS - 1) as f64) as usize;
    row * SPATIAL_GRID_COLS + col
}

fn filter_matches_with_fundamental_ransac(
    match_points: &[((f64, f64), (f64, f64))],
    geometric_tolerance_px: f64,
) -> Option<(Vec<usize>, Vec<f64>)> {
    if match_points.len() < 8 {
        return None;
    }

    let threshold = (0.35 * geometric_tolerance_px).powi(2).max(1.5);
    let iterations = (match_points.len().max(16) * 3).min(220);
    let mut best_inliers: Vec<usize> = Vec::new();
    let mut best_f = Matrix3::identity();

    for iter in 0..iterations {
        let Some(sample) = deterministic_sample_indices(match_points.len(), 8, iter as u64 + 29) else {
            continue;
        };
        let sample_points: Vec<[(f64, f64); 2]> = sample
            .iter()
            .map(|&idx| {
                let ((x1, y1), (x2, y2)) = match_points[idx];
                [(x1, y1), (x2, y2)]
            })
            .collect();
        let Some(f) = estimate_fundamental_from_correspondences(&sample_points) else {
            continue;
        };
        let inliers = sampson_inliers_fundamental(&f, match_points, threshold);
        if inliers.len() > best_inliers.len() {
            best_inliers = inliers;
            best_f = f;
        }
    }

    if best_inliers.len() < FUNDAMENTAL_RANSAC_MIN_INLIERS {
        return None;
    }

    let refined_points: Vec<[(f64, f64); 2]> = best_inliers
        .iter()
        .map(|&idx| {
            let ((x1, y1), (x2, y2)) = match_points[idx];
            [(x1, y1), (x2, y2)]
        })
        .collect();
    let refined_f = estimate_fundamental_from_correspondences(&refined_points).unwrap_or(best_f);
    let refined_inliers = sampson_inliers_fundamental(&refined_f, match_points, threshold);
    let mut residuals = vec![f64::INFINITY; match_points.len()];
    for &idx in &refined_inliers {
        residuals[idx] = sampson_residual_fundamental(&refined_f, match_points[idx]);
    }
    Some((refined_inliers, residuals))
}

fn estimate_fundamental_from_correspondences(
    points: &[[(f64, f64); 2]],
) -> Option<Matrix3<f64>> {
    if points.len() < 8 {
        return None;
    }

    let left_pts: Vec<Vector2<f64>> = points.iter().map(|p| Vector2::new(p[0].0, p[0].1)).collect();
    let right_pts: Vec<Vector2<f64>> = points.iter().map(|p| Vector2::new(p[1].0, p[1].1)).collect();
    let (norm_left, t_left) = normalize_points_2d(&left_pts)?;
    let (norm_right, t_right) = normalize_points_2d(&right_pts)?;

    let mut a = DMatrix::zeros(points.len(), 9);
    for i in 0..points.len() {
        let p1 = norm_left[i];
        let p2 = norm_right[i];
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
    let f_vec = v_t.row(v_t.nrows() - 1);
    let mut f_hat = Matrix3::zeros();
    for r in 0..3 {
        for c in 0..3 {
            f_hat[(r, c)] = f_vec[r * 3 + c];
        }
    }

    let svd_f = f_hat.svd(true, true);
    let u = svd_f.u?;
    let v_t = svd_f.v_t?;
    let mut singular = svd_f.singular_values;
    singular[2] = 0.0;
    let sigma = Matrix3::new(
        singular[0], 0.0, 0.0,
        0.0, singular[1], 0.0,
        0.0, 0.0, singular[2],
    );
    let f_rank2 = u * sigma * v_t;
    let f_denorm = t_right.transpose() * f_rank2 * t_left;
    let norm = f_denorm.norm();
    if norm <= 1.0e-12 {
        None
    } else {
        Some(f_denorm / norm)
    }
}

fn normalize_points_2d(points: &[Vector2<f64>]) -> Option<(Vec<Vector2<f64>>, Matrix3<f64>)> {
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
    let scale = if mean_dist <= 1.0e-12 {
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

fn sampson_inliers_fundamental(
    f: &Matrix3<f64>,
    points: &[((f64, f64), (f64, f64))],
    threshold: f64,
) -> Vec<usize> {
    let mut inliers = Vec::new();
    for (idx, ((x1, y1), (x2, y2))) in points.iter().enumerate() {
        let p1 = Vector3::new(*x1, *y1, 1.0);
        let p2 = Vector3::new(*x2, *y2, 1.0);
        let fp1 = f * p1;
        let ft_p2 = f.transpose() * p2;
        let numer = p2.dot(&fp1);
        let denom = fp1.x * fp1.x + fp1.y * fp1.y + ft_p2.x * ft_p2.x + ft_p2.y * ft_p2.y;
        if denom <= 1.0e-12 {
            continue;
        }
        let d = (numer * numer) / denom;
        if d <= threshold {
            inliers.push(idx);
        }
    }
    inliers
}

fn sampson_residual_fundamental(
    f: &Matrix3<f64>,
    point: ((f64, f64), (f64, f64)),
) -> f64 {
    let ((x1, y1), (x2, y2)) = point;
    let p1 = Vector3::new(x1, y1, 1.0);
    let p2 = Vector3::new(x2, y2, 1.0);
    let fp1 = f * p1;
    let ft_p2 = f.transpose() * p2;
    let numer = p2.dot(&fp1);
    let denom = fp1.x * fp1.x + fp1.y * fp1.y + ft_p2.x * ft_p2.x + ft_p2.y * ft_p2.y;
    if denom <= 1.0e-12 {
        f64::INFINITY
    } else {
        (numer * numer) / denom
    }
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

#[derive(Debug, Clone, Copy)]
struct SimilarityModel {
    a: f64,
    b: f64,
    tx: f64,
    ty: f64,
}

fn similarity_model_from_two_pairs(
    first: ((f64, f64), (f64, f64)),
    second: ((f64, f64), (f64, f64)),
) -> Option<SimilarityModel> {
    let ((lx0, ly0), (rx0, ry0)) = first;
    let ((lx1, ly1), (rx1, ry1)) = second;
    let dpx = lx1 - lx0;
    let dpy = ly1 - ly0;
    let dqx = rx1 - rx0;
    let dqy = ry1 - ry0;
    let denom = dpx * dpx + dpy * dpy;
    if denom <= 1.0e-9 {
        return None;
    }

    let a = (dqx * dpx + dqy * dpy) / denom;
    let b = (dqy * dpx - dqx * dpy) / denom;
    let tx = rx0 - (a * lx0 - b * ly0);
    let ty = ry0 - (b * lx0 + a * ly0);

    if !a.is_finite() || !b.is_finite() || !tx.is_finite() || !ty.is_finite() {
        return None;
    }
    Some(SimilarityModel { a, b, tx, ty })
}

fn similarity_residual(
    model: SimilarityModel,
    point: ((f64, f64), (f64, f64)),
) -> f64 {
    let ((lx, ly), (rx, ry)) = point;
    let pred_x = model.a * lx - model.b * ly + model.tx;
    let pred_y = model.b * lx + model.a * ly + model.ty;
    let ex = pred_x - rx;
    let ey = pred_y - ry;
    (ex * ex + ey * ey).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    use crate::ingest::FrameMetadata;

    fn make_frame(path: &std::path::Path) -> ImageFrame {
        ImageFrame {
            path: path.to_string_lossy().to_string(),
            width: 160,
            height: 160,
            metadata: FrameMetadata {
                gps: None,
                focal_length_mm: None,
                sensor_width_mm: None,
                image_width_px: 160,
                image_height_px: 160,
                timestamp: None,
                orientation_prior: None,
                blur_score: None,
                has_rtk_gps: false,
            },
        }
    }

    #[test]
    fn empty_frame_list_returns_zero_stats() {
        let stats = run_feature_matching(&[], "balanced").expect("empty run should succeed");
        assert_eq!(stats.frame_count, 0);
        assert_eq!(stats.total_keypoints, 0);
        assert_eq!(stats.total_matches, 0);
        assert_eq!(stats.connectivity, 0.0);
    }

    #[test]
    fn tiny_images_return_zero_matches_without_panicking() {
        let tmp = std::env::temp_dir().join(format!(
            "wbphotogrammetry_feature_tiny_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("failed creating temp dir");

        let tiny_path = tmp.join("tiny.png");
        let tiny = GrayImage::from_pixel(8, 8, Luma([120]));
        tiny.save(&tiny_path).expect("failed writing tiny frame");

        let stats = run_feature_matching(&[make_frame(&tiny_path)], "balanced")
            .expect("tiny-image matching should succeed");

        assert_eq!(stats.frame_count, 1);
        assert_eq!(stats.total_keypoints, 0);
        assert_eq!(stats.total_matches, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn non_max_suppression_rejects_weaker_and_tie_keypoints() {
        let width = 7_u32;
        let height = 7_u32;
        let mut score_map = vec![0_u32; (width * height) as usize];

        // Stronger neighbor should suppress current point.
        score_map[(3 * width + 3) as usize] = 10;
        score_map[(3 * width + 4) as usize] = 12;
        assert!(!is_local_maximum(
            &score_map,
            width,
            height,
            3,
            3,
            10,
            NON_MAX_SUPPRESSION_RADIUS_PX,
        ));

        // Equal-score ties are broken deterministically by raster order.
        score_map[(3 * width + 4) as usize] = 10;
        assert!(is_local_maximum(
            &score_map,
            width,
            height,
            3,
            3,
            10,
            NON_MAX_SUPPRESSION_RADIUS_PX,
        ));
        assert!(!is_local_maximum(
            &score_map,
            width,
            height,
            4,
            3,
            10,
            NON_MAX_SUPPRESSION_RADIUS_PX,
        ));
    }

    #[test]
    fn full_detector_path_returns_only_local_maxima() {
        let mut img = GrayImage::new(96, 96);
        for y in 0..96 {
            for x in 0..96 {
                let value = if ((x / 8) + (y / 8)) % 2 == 0 { 20 } else { 230 };
                img.put_pixel(x, y, Luma([value]));
            }
        }

        // Add a couple of stronger structures so corners are consistently found.
        for y in 16..80 {
            img.put_pixel(20, y, Luma([255]));
            img.put_pixel(68, y, Luma([0]));
        }
        for x in 16..80 {
            img.put_pixel(x, 24, Luma([255]));
            img.put_pixel(x, 72, Luma([0]));
        }

        let threshold = 20_u8;
        let keypoints = detect_fast_corners(&img, threshold, 300);
        assert!(!keypoints.is_empty(), "expected detector to find corners");

        let width = img.width();
        let height = img.height();
        let mut score_map = vec![0_u32; (width * height) as usize];
        for y in FAST_EDGE_RADIUS_PX..(height - FAST_EDGE_RADIUS_PX) {
            for x in FAST_EDGE_RADIUS_PX..(width - FAST_EDGE_RADIUS_PX) {
                if let Some(score) = fast_corner_score(&img, x as i32, y as i32, threshold) {
                    score_map[(y * width + x) as usize] = score;
                }
            }
        }

        for keypoint in keypoints {
            assert!(is_local_maximum(
                &score_map,
                width,
                height,
                keypoint.x as i32,
                keypoint.y as i32,
                keypoint.score,
                NON_MAX_SUPPRESSION_RADIUS_PX,
            ));
        }
    }

    #[test]
    fn sift_backend_finds_matches_on_translated_texture() {
        let tmp = std::env::temp_dir().join(format!(
            "wbphotogrammetry_feature_sift_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("failed creating temp dir");

        let left_path = tmp.join("left.png");
        let right_path = tmp.join("right.png");

        let mut left = GrayImage::new(160, 160);
        let mut right = GrayImage::new(160, 160);
        for y in 0..160 {
            for x in 0..160 {
                let base = if ((x / 10) + (y / 10)) % 2 == 0 { 35 } else { 220 };
                left.put_pixel(x, y, Luma([base]));

                let sx = x.saturating_sub(4);
                let sy = y.saturating_sub(3);
                let shifted = if ((sx / 10) + (sy / 10)) % 2 == 0 { 35 } else { 220 };
                right.put_pixel(x, y, Luma([shifted]));
            }
        }

        for y in 32..128 {
            left.put_pixel(48, y, Luma([255]));
            right.put_pixel(52, y, Luma([255]));
        }
        for x in 24..136 {
            left.put_pixel(x, 64, Luma([0]));
            right.put_pixel(x.saturating_add(4).min(159), 67, Luma([0]));
        }

        left.save(&left_path).expect("failed writing left frame");
        right.save(&right_path).expect("failed writing right frame");

        let frames = vec![make_frame(&left_path), make_frame(&right_path)];
        let stats = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::Sift)
            .expect("sift matching should succeed");

        assert!(stats.total_keypoints > 0, "expected sift keypoints on synthetic texture");
        assert!(stats.total_matches > 0, "expected sift matches on translated texture");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rootsift_backend_finds_matches_on_translated_texture() {
        let tmp = std::env::temp_dir().join(format!(
            "wbphotogrammetry_feature_rootsift_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("failed creating temp dir");

        let left_path = tmp.join("left.png");
        let right_path = tmp.join("right.png");

        let mut left = GrayImage::new(160, 160);
        let mut right = GrayImage::new(160, 160);
        for y in 0..160 {
            for x in 0..160 {
                let base = if ((x / 9) + (y / 9)) % 2 == 0 { 28 } else { 226 };
                left.put_pixel(x, y, Luma([base]));

                let sx = x.saturating_sub(5);
                let sy = y.saturating_sub(2);
                let shifted = if ((sx / 9) + (sy / 9)) % 2 == 0 { 28 } else { 226 };
                right.put_pixel(x, y, Luma([shifted]));
            }
        }
        for y in 36..124 {
            left.put_pixel(44, y, Luma([255]));
            right.put_pixel(49, y, Luma([255]));
        }
        for x in 26..138 {
            left.put_pixel(x, 82, Luma([0]));
            right.put_pixel(x.saturating_add(5).min(159), 84, Luma([0]));
        }

        left.save(&left_path).expect("failed writing left frame");
        right.save(&right_path).expect("failed writing right frame");

        let frames = vec![make_frame(&left_path), make_frame(&right_path)];
        let stats = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::RootSift)
            .expect("rootsift matching should succeed");

        assert!(stats.total_keypoints > 0, "expected rootsift keypoints on synthetic texture");
        assert!(stats.total_matches > 0, "expected rootsift matches on translated texture");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sift_and_rootsift_match_under_photometric_perturbation() {
        let tmp = std::env::temp_dir().join(format!(
            "wbphotogrammetry_feature_photometric_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("failed creating temp dir");

        let left_path = tmp.join("left.png");
        let right_path = tmp.join("right.png");

        let mut left = GrayImage::new(160, 160);
        let mut right = GrayImage::new(160, 160);
        for y in 0..160 {
            for x in 0..160 {
                let checker = if ((x / 8) + (y / 8)) % 2 == 0 { 42.0 } else { 212.0 };
                left.put_pixel(x, y, Luma([checker as u8]));

                let sx = x.saturating_sub(4);
                let sy = y.saturating_sub(3);
                let shifted_checker = if ((sx / 8) + (sy / 8)) % 2 == 0 { 42.0 } else { 212.0 };
                let deterministic_noise = (((x * 17 + y * 29) % 7) as f32) - 3.0;
                let photometric = (shifted_checker * 0.90) + 12.0 + deterministic_noise;
                let clamped = photometric.clamp(0.0, 255.0) as u8;
                right.put_pixel(x, y, Luma([clamped]));
            }
        }

        for y in 24..136 {
            left.put_pixel(52, y, Luma([255]));
            right.put_pixel(56, y, Luma([240]));
        }
        for x in 18..142 {
            left.put_pixel(x, 88, Luma([0]));
            right.put_pixel(x.saturating_add(4).min(159), 91, Luma([8]));
        }

        left.save(&left_path).expect("failed writing left frame");
        right.save(&right_path).expect("failed writing right frame");

        let frames = vec![make_frame(&left_path), make_frame(&right_path)];
        let sift = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::Sift)
            .expect("sift matching should succeed");
        let rootsift = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::RootSift)
            .expect("rootsift matching should succeed");

        assert!(sift.total_keypoints > 0, "expected sift keypoints on photometric perturbation");
        assert!(rootsift.total_keypoints > 0, "expected rootsift keypoints on photometric perturbation");
        assert!(sift.total_matches > 0, "expected sift matches on photometric perturbation");
        assert!(rootsift.total_matches > 0, "expected rootsift matches on photometric perturbation");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rootsift_match_count_remains_same_order_as_sift_under_photometric_perturbation() {
        let tmp = std::env::temp_dir().join(format!(
            "wbphotogrammetry_feature_photometric_ratio_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("failed creating temp dir");

        let left_path = tmp.join("left.png");
        let right_path = tmp.join("right.png");

        let mut left = GrayImage::new(160, 160);
        let mut right = GrayImage::new(160, 160);
        for y in 0..160 {
            for x in 0..160 {
                let checker = if ((x / 10) + (y / 10)) % 2 == 0 { 36.0 } else { 224.0 };
                left.put_pixel(x, y, Luma([checker as u8]));

                let sx = x.saturating_sub(5);
                let sy = y.saturating_sub(2);
                let shifted_checker = if ((sx / 10) + (sy / 10)) % 2 == 0 { 36.0 } else { 224.0 };
                let deterministic_noise = (((x * 31 + y * 11) % 9) as f32) - 4.0;
                let photometric = (shifted_checker * 0.88) + 14.0 + deterministic_noise;
                let clamped = photometric.clamp(0.0, 255.0) as u8;
                right.put_pixel(x, y, Luma([clamped]));
            }
        }

        for y in 30..130 {
            left.put_pixel(46, y, Luma([255]));
            right.put_pixel(51, y, Luma([242]));
        }
        for x in 24..136 {
            left.put_pixel(x, 74, Luma([0]));
            right.put_pixel(x.saturating_add(5).min(159), 76, Luma([6]));
        }

        left.save(&left_path).expect("failed writing left frame");
        right.save(&right_path).expect("failed writing right frame");

        let frames = vec![make_frame(&left_path), make_frame(&right_path)];
        let sift = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::Sift)
            .expect("sift matching should succeed");
        let rootsift = run_feature_matching_with_method(&frames, "balanced", FeatureMethod::RootSift)
            .expect("rootsift matching should succeed");

        assert!(sift.total_matches > 0, "expected sift matches for ratio comparison");
        assert!(rootsift.total_matches > 0, "expected rootsift matches for ratio comparison");
        assert!(
            rootsift.total_matches * 4 >= sift.total_matches,
            "rootsift matches ({}) should remain in same order as sift ({}) under photometric perturbation",
            rootsift.total_matches,
            sift.total_matches
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rootsift_normalization_is_scale_invariant_and_unit_l2() {
        let mut a = [0.0_f32; SIFT_DESCRIPTOR_LEN];
        let mut b = [0.0_f32; SIFT_DESCRIPTOR_LEN];
        for i in 0..SIFT_DESCRIPTOR_LEN {
            let value = ((i % 17) as f32 + 1.0) * 0.5;
            a[i] = value;
            b[i] = value * 13.0;
        }

        normalize_rootsift_descriptor(&mut a).expect("rootsift normalization should succeed");
        normalize_rootsift_descriptor(&mut b).expect("rootsift normalization should succeed");

        let norm_a = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        let norm_b = b.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm_a - 1.0).abs() < 1.0e-4, "normalized descriptor should have unit L2 norm");
        assert!((norm_b - 1.0).abs() < 1.0e-4, "normalized descriptor should have unit L2 norm");

        let max_delta = a
            .iter()
            .zip(b.iter())
            .map(|(lhs, rhs)| (lhs - rhs).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_delta < 1.0e-4,
            "rootsift normalization should be scale invariant; max delta={max_delta}"
        );
    }

    #[test]
    fn rootsift_normalization_rejects_zero_descriptor() {
        let mut descriptor = [0.0_f32; SIFT_DESCRIPTOR_LEN];
        assert!(
            normalize_rootsift_descriptor(&mut descriptor).is_none(),
            "zero descriptor should not be normalizable"
        );
    }

    #[test]
    fn feature_method_parser_accepts_public_names() {
        assert_eq!(FeatureMethod::from_str("brief").unwrap(), FeatureMethod::Brief);
        assert_eq!(FeatureMethod::from_str("orb").unwrap(), FeatureMethod::Orb);
        assert_eq!(FeatureMethod::from_str("sift").unwrap(), FeatureMethod::Sift);
        assert_eq!(FeatureMethod::from_str("rootsift").unwrap(), FeatureMethod::RootSift);
        assert_eq!(
            FeatureMethod::from_str("super_point").unwrap(),
            FeatureMethod::SuperPoint
        );
    }

    #[test]
    fn default_feature_matching_options_use_balanced_rootsift() {
        let options = FeatureMatchingOptions::default();
        assert_eq!(options.profile, "balanced");
        assert_eq!(options.method, FeatureMethod::RootSift);
    }

    #[test]
    fn unimplemented_floating_point_methods_fail_cleanly() {
        let sift = run_feature_matching_with_method(&[], "balanced", FeatureMethod::Sift)
            .expect("sift empty input should succeed");
        assert_eq!(sift.frame_count, 0);
        assert_eq!(sift.total_keypoints, 0);

        let err = run_feature_matching_superpoint(&[], "balanced")
            .expect_err("superpoint should return a not implemented error");
        assert!(matches!(err, PhotogrammetryError::NotImplemented(message) if message.contains("superpoint")));
    }

    #[test]
    fn ratio_test_rejects_ambiguous_matches() {
        let query = BriefDescriptor {
            corner: Keypoint {
                x: 0,
                y: 0,
                score: 1,
            },
            words: [0, 0, 0, 0],
            texture_stddev: 12.0,
            octave: 0,
        };
        let left = vec![query.clone()];
        let ambiguous_right = vec![
            BriefDescriptor {
                corner: Keypoint {
                    x: 1,
                    y: 0,
                    score: 1,
                },
                words: [0, 0, 0, 1],
                texture_stddev: 12.0,
                octave: 0,
            },
            BriefDescriptor {
                corner: Keypoint {
                    x: 2,
                    y: 0,
                    score: 1,
                },
                words: [0, 0, 0, 2],
                texture_stddev: 12.0,
                octave: 0,
            },
        ];

        let strict = match_binary_descriptors(&left, &ambiguous_right, 32, 0.8, None);
        assert!(
            strict.is_empty(),
            "ambiguous duplicate best matches should fail ratio filtering"
        );

        let unambiguous_right = vec![
            BriefDescriptor {
                corner: Keypoint {
                    x: 1,
                    y: 0,
                    score: 1,
                },
                words: [0, 0, 0, 0],
                texture_stddev: 12.0,
                octave: 0,
            },
            BriefDescriptor {
                corner: Keypoint {
                    x: 2,
                    y: 0,
                    score: 1,
                },
                words: [u64::MAX, u64::MAX, u64::MAX, u64::MAX],
                texture_stddev: 12.0,
                octave: 0,
            },
        ];
        let accepted = match_binary_descriptors(&left, &unambiguous_right, 256, 0.8, None);
        assert_eq!(accepted.len(), 1, "clear nearest neighbor should be retained");

        let singleton_right = vec![BriefDescriptor {
            corner: Keypoint {
                x: 3,
                y: 0,
                score: 1,
            },
            words: [0, 0, 0, 0],
            texture_stddev: 12.0,
            octave: 0,
        }];
        let accepted_singleton = match_binary_descriptors(&left, &singleton_right, 256, 0.8, None);
        assert_eq!(
            accepted_singleton.len(),
            1,
            "singleton exact match should be allowed only when distance is exact"
        );

        assert!(
            !passes_ratio_test(5.0, None, 0.8, 3),
            "missing second-best must fail ratio filtering when candidate set is not singleton"
        );
    }

    #[test]
    fn robust_translation_keeps_dominant_motion_cluster() {
        let mut points = Vec::new();
        for idx in 0..12 {
            let lx = idx as f64 * 10.0;
            let ly = idx as f64 * 2.0;
            points.push(((lx, ly), (lx + 5.0, ly - 3.0)));
        }
        points.push(((10.0, 50.0), (120.0, -20.0)));
        points.push(((40.0, 20.0), (-60.0, 88.0)));

        let (inliers, _indices, _residuals) = robust_translation_inliers(&points, 1.0);
        assert_eq!(inliers.inlier_count, 12, "expected dominant translation cluster to be selected");
        assert!((inliers.median_displacement_px - (34.0_f64).sqrt()).abs() < 0.25);
    }

    #[test]
    fn low_texture_keypoints_are_rejected_before_descriptor_build() {
        let image = GrayImage::from_pixel(96, 96, Luma([128]));
        let keypoints = vec![Keypoint {
            x: 48,
            y: 48,
            score: 100,
        }];
        let pairs = build_deterministic_test_pairs(256);
        let descriptors = compute_brief_descriptors(&image, &keypoints, &pairs);
        assert!(descriptors.is_empty(), "uniform patches should not yield descriptors");
    }

    #[test]
    fn spatial_distribution_filter_caps_clustered_matches() {
        let candidates = vec![
            PairMatchCandidate { point: [8.0, 8.0, 9.0, 8.5], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [10.0, 7.0, 11.0, 7.5], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [12.0, 9.0, 13.0, 9.5], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [11.0, 11.0, 12.0, 11.5], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [9.0, 10.0, 10.0, 10.5], descriptor_confidence: 0.8, texture_confidence: 0.8 },
        ];
        let indices: Vec<usize> = (0..candidates.len()).collect();
        let filtered = spatially_filter_inlier_indices(&candidates, &indices, 100, 100, 100, 100);
        assert!(
            filtered.len() <= candidates.len(),
            "clustered matches should not grow after spatial balancing"
        );
        let penalty = spatial_distribution_penalty(
            &filtered.iter().map(|&idx| candidates[idx].point).collect::<Vec<_>>(),
            100,
            100,
            100,
            100,
        );
        assert!(penalty < 0.8, "clustered matches should receive a meaningful spatial penalty");
    }

    #[test]
    fn spatial_distribution_filter_keeps_spread_matches() {
        let candidates = vec![
            PairMatchCandidate { point: [8.0, 8.0, 10.0, 9.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [32.0, 14.0, 34.0, 15.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [68.0, 22.0, 70.0, 23.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [20.0, 54.0, 22.0, 55.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [56.0, 62.0, 58.0, 63.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
            PairMatchCandidate { point: [82.0, 78.0, 84.0, 79.0], descriptor_confidence: 0.8, texture_confidence: 0.8 },
        ];
        let indices: Vec<usize> = (0..candidates.len()).collect();
        let filtered = spatially_filter_inlier_indices(&candidates, &indices, 100, 100, 100, 100);
        assert_eq!(filtered.len(), candidates.len(), "spatially spread matches should be kept");
    }

    #[test]
    fn deterministic_brief_pattern_has_high_uniqueness() {
        let pairs = build_deterministic_test_pairs(256);
        assert_eq!(pairs.len(), 256);

        let mut seen = HashSet::new();
        for pair in &pairs {
            let key = (pair.p0.0, pair.p0.1, pair.p1.0, pair.p1.1);
            let reverse_key = (pair.p1.0, pair.p1.1, pair.p0.0, pair.p0.1);
            assert!(!seen.contains(&key), "duplicate BRIEF test pair generated");
            assert!(
                !seen.contains(&reverse_key),
                "mirror-duplicate BRIEF test pair generated"
            );
            seen.insert(key);
        }
    }

    #[test]
    fn adaptive_pair_gate_rejects_clustered_low_support_inliers() {
        let candidates = vec![
            PairMatchCandidate { point: [8.0, 8.0, 9.0, 8.6], descriptor_confidence: 0.45, texture_confidence: 0.40 },
            PairMatchCandidate { point: [9.0, 8.5, 10.0, 9.1], descriptor_confidence: 0.44, texture_confidence: 0.42 },
        ];
        let indices = vec![0usize, 1usize];
        let residuals = vec![0.8_f64, 0.9_f64];

        let (stats, points, weights) = summarize_inlier_candidates(
            &candidates,
            &indices,
            Some(&residuals),
            2.0,
            100,
            100,
            100,
            100,
        );
        assert_eq!(stats.inlier_count, 0);
        assert!(points.is_empty());
        assert!(weights.is_empty());
    }

    #[test]
    fn adaptive_pair_gate_keeps_well_supported_inliers() {
        let candidates = vec![
            PairMatchCandidate { point: [8.0, 8.0, 12.0, 10.0], descriptor_confidence: 0.95, texture_confidence: 0.92 },
            PairMatchCandidate { point: [86.0, 84.0, 90.0, 86.0], descriptor_confidence: 0.93, texture_confidence: 0.94 },
        ];
        let indices = vec![0usize, 1usize];
        let residuals = vec![0.15_f64, 0.18_f64];

        let (stats, points, weights) = summarize_inlier_candidates(
            &candidates,
            &indices,
            Some(&residuals),
            2.0,
            100,
            100,
            100,
            100,
        );
        assert_eq!(stats.inlier_count, 2);
        assert_eq!(points.len(), 2);
        assert_eq!(weights.len(), 2);
    }
}
