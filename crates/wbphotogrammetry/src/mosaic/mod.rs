//! Orthomosaic generation.
//!
//! Current implementation projects real RGB source imagery onto the DSM grid
//! using pinhole back-projection and weighted multi-view blending.

use image::RgbImage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::alignment::AlignmentResult;
use crate::camera::{CameraIntrinsics, CameraModel};
use crate::error::{PhotogrammetryError, Result};
use crate::ingest::ImageFrame;

#[derive(Debug, Clone)]
struct SourceImage {
    rgb: RgbImage,
    width_px: u32,
    height_px: u32,
    pose_xyz_m: [f64; 3],
    rotation: [f64; 4],
    fx: f64,
    fy: f64,
    cx: f64,
    cy: f64,
    rgb_gain: [f64; 3],
    quality_weight: f64,
    use_legacy_projection: bool,
    camera_model: CameraModel,
    k1: f64,
    k2: f64,
    p1: f64,
    p2: f64,
}

#[derive(Debug, Clone, Copy)]
struct BlendOutcome {
    rgb: [f64; 3],
    used_sources: u8,
    dominance_ratio: f64,
    confidence: f64,
    selected_source_idx: usize,
}

#[derive(Debug, Clone, Copy)]
struct RadiometricEdge {
    a: usize,
    b: usize,
    weight: f64,
    ratio_b_over_a: [f64; 3],
    sample_count: u32,
}

/// Statistics from the orthomosaic stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamStats {
    /// Number of seam segments blended.
    pub seam_segments: u64,
    /// Maximum colour difference across seams (0–1 normalised).
    pub max_seam_delta: f64,
    /// Mean colour difference across seams.
    pub mean_seam_delta: f64,
    /// Number of seam cells in the raw transition mask before path optimization.
    pub raw_seam_cells: u64,
    /// Number of seam cells retained after seamline path optimization.
    pub optimized_seam_cells: u64,
    /// Percent seam-cell reduction from raw mask to optimized path(s).
    pub seam_path_reduction_pct: f64,
    /// Number of adjacent image-pair overlap edges used in radiometric refinement.
    pub overlap_edge_count: u64,
    /// Number of overlap samples used to estimate pairwise radiometric ratios.
    pub overlap_edge_samples: u64,
}

/// Result from the orthomosaic generation stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicResult {
    /// Path of the orthomosaic GeoTIFF written to disk.
    pub ortho_path: String,
    /// Seam statistics.
    pub stats: SeamStats,
    /// Ground sampling distance of the output mosaic in metres.
    pub gsd_m: f64,
    /// Number of DSM-valid cells that received at least one image projection.
    pub projected_cells: u64,
    /// Number of DSM-valid cells with no valid projection (written as nodata).
    pub uncovered_cells: u64,
    /// Orthomosaic coverage percentage over DSM-valid cells.
    pub coverage_pct: f64,
    /// Optional raster path with per-cell mosaic confidence (0-1).
    pub confidence_raster_path: Option<String>,
    /// Optional raster path with support diagnostics (band1=source_count, band2=dominance_ratio).
    pub support_diagnostics_path: Option<String>,
    /// Optional raster path with selected source-image index per cell (1-based frame index).
    pub source_index_raster_path: Option<String>,
}

/// Build an orthomosaic from the aligned camera network and DSM.
///
/// The current model assumes nadir-dominant drone imagery and performs
/// pinhole projection from DSM cells into each image plane.
pub fn run_orthomosaic(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
    dsm_path: &str,
    ortho_path: &str,
) -> Result<MosaicResult> {
    run_orthomosaic_with_confidence(alignment, frames, dsm_path, ortho_path, None)
}

pub fn run_orthomosaic_with_confidence(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
    dsm_path: &str,
    ortho_path: &str,
    confidence_path: Option<&str>,
) -> Result<MosaicResult> {
    let dsm = Raster::read(dsm_path)?;
    let dense_support = load_dense_support_raster(dsm_path, dsm.rows, dsm.cols);

    let cfg = RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        bands: 3,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "orthomosaic".to_string()),
            ("generator".to_string(), "wbphotogrammetry_rgb_v1".to_string()),
        ],
        ..RasterConfig::default()
    };
    let mut ortho = Raster::new(cfg);
    let mut support_diag = Raster::new(RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        bands: 2,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "orthomosaic_support".to_string()),
            ("band1".to_string(), "source_count".to_string()),
            ("band2".to_string(), "dominance_ratio".to_string()),
        ],
        ..RasterConfig::default()
    });
    let mut source_index = Raster::new(RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        bands: 1,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "orthomosaic_source_index".to_string()),
            (
                "band1".to_string(),
                "selected_source_frame_index_1_based".to_string(),
            ),
        ],
        ..RasterConfig::default()
    });
    let mut confidence = Raster::new(RasterConfig {
        cols: dsm.cols,
        rows: dsm.rows,
        bands: 1,
        x_min: dsm.x_min,
        y_min: dsm.y_min,
        cell_size: dsm.cell_size_x,
        nodata: dsm.nodata,
        data_type: DataType::F32,
        crs: dsm.crs.clone(),
        metadata: vec![
            ("stage".to_string(), "orthomosaic_confidence".to_string()),
            ("band1".to_string(), "mosaic_confidence_0_1".to_string()),
        ],
        ..RasterConfig::default()
    });

    if alignment.poses.is_empty() || frames.is_empty() {
        ortho.write(ortho_path, RasterFormat::GeoTiff)?;
        let confidence_raster_path = if let Some(path) = confidence_path {
            let placeholder = Raster::new(RasterConfig {
                cols: dsm.cols,
                rows: dsm.rows,
                bands: 1,
                x_min: dsm.x_min,
                y_min: dsm.y_min,
                cell_size: dsm.cell_size_x,
                nodata: dsm.nodata,
                data_type: DataType::F32,
                crs: dsm.crs.clone(),
                ..RasterConfig::default()
            });
            placeholder.write(path, RasterFormat::GeoTiff)?;
            Some(path.to_string())
        } else {
            None
        };
        return Ok(MosaicResult {
            ortho_path: ortho_path.to_string(),
            stats: SeamStats {
                seam_segments: 0,
                max_seam_delta: 0.0,
                mean_seam_delta: 0.0,
                raw_seam_cells: 0,
                optimized_seam_cells: 0,
                seam_path_reduction_pct: 0.0,
                overlap_edge_count: 0,
                overlap_edge_samples: 0,
            },
            gsd_m: dsm.cell_size_x.max(0.001),
            projected_cells: 0,
            uncovered_cells: 0,
            coverage_pct: 0.0,
            confidence_raster_path,
            support_diagnostics_path: None,
            source_index_raster_path: None,
        });
    }

    let (_min_z, _max_z, valid_count) = dsm_min_max_count(&dsm);
    if valid_count == 0 {
        ortho.write(ortho_path, RasterFormat::GeoTiff)?;
        let confidence_raster_path = if let Some(path) = confidence_path {
            let placeholder = Raster::new(RasterConfig {
                cols: dsm.cols,
                rows: dsm.rows,
                bands: 1,
                x_min: dsm.x_min,
                y_min: dsm.y_min,
                cell_size: dsm.cell_size_x,
                nodata: dsm.nodata,
                data_type: DataType::F32,
                crs: dsm.crs.clone(),
                ..RasterConfig::default()
            });
            placeholder.write(path, RasterFormat::GeoTiff)?;
            Some(path.to_string())
        } else {
            None
        };
        return Ok(MosaicResult {
            ortho_path: ortho_path.to_string(),
            stats: SeamStats {
                seam_segments: 0,
                max_seam_delta: 0.0,
                mean_seam_delta: 0.0,
                raw_seam_cells: 0,
                optimized_seam_cells: 0,
                seam_path_reduction_pct: 0.0,
                overlap_edge_count: 0,
                overlap_edge_samples: 0,
            },
            gsd_m: dsm.cell_size_x.max(0.001),
            projected_cells: 0,
            uncovered_cells: 0,
            coverage_pct: 0.0,
            confidence_raster_path,
            support_diagnostics_path: None,
            source_index_raster_path: None,
        });
    }

    let mut sources = build_source_images(alignment, frames)?;
    if sources.is_empty() {
        return Err(PhotogrammetryError::Orthomosaic(
            "no aligned source images could be loaded for orthomosaic generation".to_string(),
        ));
    }
    let overlap_edges = build_pairwise_overlap_edges_from_projected_cells(&sources, &dsm);
    refine_pairwise_rgb_gains(&mut sources, &overlap_edges);

    let mut projected_cells = 0_u64;
    let mut uncovered_cells = 0_u64;
    let mut source_choice = vec![usize::MAX; dsm.rows * dsm.cols];
    let mut prev_row_choice = vec![usize::MAX; dsm.cols];
    for row in 0..dsm.rows {
        let mut curr_row_choice = vec![usize::MAX; dsm.cols];
        for col in 0..dsm.cols {
            let z = dsm.get(0, row as isize, col as isize);
            if (z - dsm.nodata).abs() <= f64::EPSILON {
                continue;
            }

            let x_m = dsm.x_min + (col as f64 + 0.5) * dsm.cell_size_x;
            let y_m = dsm.y_min + ((dsm.rows - 1 - row) as f64 + 0.5) * dsm.cell_size_y;
            let support_w = dense_support_weight(&dense_support, row, col);
            if support_w <= 0.03 {
                ortho.set(0, row as isize, col as isize, ortho.nodata)?;
                ortho.set(1, row as isize, col as isize, ortho.nodata)?;
                ortho.set(2, row as isize, col as isize, ortho.nodata)?;
                confidence.set(0, row as isize, col as isize, confidence.nodata)?;
                support_diag.set(0, row as isize, col as isize, support_diag.nodata)?;
                support_diag.set(1, row as isize, col as isize, support_diag.nodata)?;
                source_index.set(0, row as isize, col as isize, source_index.nodata)?;
                uncovered_cells += 1;
                continue;
            }

            let left_pref = if col > 0 {
                let v = curr_row_choice[col - 1];
                if v == usize::MAX { None } else { Some(v) }
            } else {
                None
            };
            let left_rgb = if col > 0 {
                rgb_at(&ortho, row as isize, (col - 1) as isize)
            } else {
                None
            };
            let up_pref = {
                let v = prev_row_choice[col];
                if v == usize::MAX { None } else { Some(v) }
            };
            let up_rgb = if row > 0 {
                rgb_at(&ortho, row as isize - 1, col as isize)
            } else {
                None
            };

            if let Some(outcome) = blend_sources_at(
                &sources,
                x_m,
                y_m,
                z,
                support_w,
                left_pref,
                up_pref,
                left_rgb,
                up_rgb,
            ) {
                ortho.set(0, row as isize, col as isize, outcome.rgb[0])?;
                ortho.set(1, row as isize, col as isize, outcome.rgb[1])?;
                ortho.set(2, row as isize, col as isize, outcome.rgb[2])?;
                let fused_confidence = (outcome.confidence * (0.35 + 0.65 * support_w)).clamp(0.0, 1.0);
                confidence.set(0, row as isize, col as isize, fused_confidence)?;
                support_diag.set(0, row as isize, col as isize, outcome.used_sources as f64)?;
                support_diag.set(1, row as isize, col as isize, outcome.dominance_ratio)?;
                source_index.set(
                    0,
                    row as isize,
                    col as isize,
                    (outcome.selected_source_idx + 1) as f64,
                )?;
                curr_row_choice[col] = outcome.selected_source_idx;
                source_choice[row * dsm.cols + col] = outcome.selected_source_idx;
                projected_cells += 1;
            } else {
                // Keep uncovered cells as nodata so sparse projection support is visible.
                ortho.set(0, row as isize, col as isize, ortho.nodata)?;
                ortho.set(1, row as isize, col as isize, ortho.nodata)?;
                ortho.set(2, row as isize, col as isize, ortho.nodata)?;
                confidence.set(0, row as isize, col as isize, confidence.nodata)?;
                support_diag.set(0, row as isize, col as isize, support_diag.nodata)?;
                support_diag.set(1, row as isize, col as isize, support_diag.nodata)?;
                source_index.set(0, row as isize, col as isize, source_index.nodata)?;
                uncovered_cells += 1;
            }
        }
        prev_row_choice = curr_row_choice;
    }

    let seam_mask_raw = build_seam_mask(&source_choice, dsm.rows, dsm.cols);
    let seam_costs = build_seam_cost_map(&ortho, &source_choice, &seam_mask_raw, &dense_support);
    let seam_mask = optimize_seamline_paths(&seam_mask_raw, dsm.rows, dsm.cols, &seam_costs);
    apply_seam_softening(&mut ortho, &seam_mask)?;
    let raw_seam_cells = seam_mask_raw.iter().filter(|v| **v).count() as u64;
    let optimized_seam_cells = seam_mask.iter().filter(|v| **v).count() as u64;
    let seam_path_reduction_pct = if raw_seam_cells > 0 {
        ((1.0 - (optimized_seam_cells as f64 / raw_seam_cells as f64)) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let overlap_edge_count = overlap_edges.len() as u64;
    let overlap_edge_samples = overlap_edges
        .iter()
        .map(|edge| edge.sample_count as u64)
        .sum::<u64>();

    let seam_deltas = seam_boundary_deltas(&ortho, &source_choice, &seam_mask);
    let seam_segments = seam_deltas.len() as u64;
    let max_seam_delta = seam_deltas.iter().copied().fold(0.0_f64, f64::max).clamp(0.0, 1.0);
    let mean_seam_delta = if seam_deltas.is_empty() {
        0.0
    } else {
        (seam_deltas.iter().sum::<f64>() / seam_deltas.len() as f64).clamp(0.0, 1.0)
    };

    ortho.write(ortho_path, RasterFormat::GeoTiff)?;
    let confidence_raster_path = if let Some(path) = confidence_path {
        confidence.write(path, RasterFormat::GeoTiff)?;
        Some(path.to_string())
    } else {
        None
    };
    let support_diag_path = with_suffix_before_ext(ortho_path, "_support");
    support_diag.write(&support_diag_path, RasterFormat::GeoTiff)?;
    let source_index_path = with_suffix_before_ext(ortho_path, "_source_index");
    source_index.write(&source_index_path, RasterFormat::GeoTiff)?;

    let coverage_pct = if valid_count > 0 {
        (projected_cells as f64 / valid_count as f64) * 100.0
    } else {
        0.0
    };

    Ok(MosaicResult {
        ortho_path: ortho_path.to_string(),
        stats: SeamStats {
            seam_segments,
            max_seam_delta,
            mean_seam_delta,
            raw_seam_cells,
            optimized_seam_cells,
            seam_path_reduction_pct,
            overlap_edge_count,
            overlap_edge_samples,
        },
        gsd_m: dsm.cell_size_x.max(0.001),
        projected_cells,
        uncovered_cells,
        coverage_pct,
        confidence_raster_path,
        support_diagnostics_path: Some(support_diag_path),
        source_index_raster_path: Some(source_index_path),
    })
}

fn load_dense_support_raster(dsm_path: &str, expected_rows: usize, expected_cols: usize) -> Option<Raster> {
    let support_path = with_suffix_before_ext(dsm_path, "_support");
    let Ok(raster) = Raster::read(&support_path) else {
        return None;
    };
    if raster.rows == expected_rows && raster.cols == expected_cols {
        Some(raster)
    } else {
        None
    }
}

fn dense_support_weight(dense_support: &Option<Raster>, row: usize, col: usize) -> f64 {
    let Some(r) = dense_support else {
        return 1.0;
    };
    let v = r.get(0, row as isize, col as isize);
    if !v.is_finite() || (v - r.nodata).abs() <= f64::EPSILON {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

fn build_source_images(
    alignment: &AlignmentResult,
    frames: &[ImageFrame],
) -> Result<Vec<SourceImage>> {
    let count = alignment.poses.len().min(frames.len());
    let mut out = Vec::with_capacity(count);
    let mut channel_means = Vec::with_capacity(count);

    for i in 0..count {
        let frame = &frames[i];
        let pose = &alignment.poses[i];

        let dyn_img = image::open(&frame.path).map_err(|e| {
            PhotogrammetryError::Orthomosaic(format!("failed to read image '{}': {e}", frame.path))
        })?;
        // Apply EXIF image orientation (0x0112 tag) to normalize the pixel data
        let oriented_img = frame.metadata.image_orientation.apply_to_image(dyn_img);
        let rgb = oriented_img.to_rgb8();
        let width_px = rgb.width();
        let height_px = rgb.height();
        if width_px < 2 || height_px < 2 {
            continue;
        }

        let (fx, fy, cx, cy) = intrinsics_for_source(alignment, frame, width_px, height_px);
        let quality_weight = 1.0 / (1.0 + pose.reprojection_error_px.max(0.0));
        let mean_channels = image_channel_means(&rgb);
        channel_means.push(mean_channels);

        out.push(SourceImage {
            rgb,
            width_px,
            height_px,
            pose_xyz_m: pose.position,
            rotation: pose.rotation,
            fx,
            fy,
            cx,
            cy,
            rgb_gain: [1.0, 1.0, 1.0],
            quality_weight,
            use_legacy_projection: false,
            camera_model: alignment.stats.model,
            k1: alignment.stats.intrinsics.k1,
            k2: alignment.stats.intrinsics.k2,
            p1: alignment.stats.intrinsics.p1,
            p2: alignment.stats.intrinsics.p2,
        });
    }

    if !out.is_empty() {
        let global_mean = global_channel_mean(&channel_means);
        for (idx, source) in out.iter_mut().enumerate() {
            source.rgb_gain = radiometric_rgb_gains(channel_means[idx], global_mean);
        }
    }

    Ok(out)
}

fn blend_sources_at(
    sources: &[SourceImage],
    x_m: f64,
    y_m: f64,
    z_m: f64,
    dense_support_w: f64,
    left_pref: Option<usize>,
    up_pref: Option<usize>,
    left_rgb: Option<[f64; 3]>,
    up_rgb: Option<[f64; 3]>,
) -> Option<BlendOutcome> {
    let mut candidates: Vec<([f64; 3], f64, f64, f64, f64, usize)> = Vec::new();

    for (source_idx, source) in sources.iter().enumerate() {
        let (px, py, view_cosine, distance_m, cam_z) = match project_world_to_image(source, x_m, y_m, z_m) {
            Some(v) => v,
            None => continue,
        };
        if px < 0.0
            || py < 0.0
            || px > source.width_px.saturating_sub(1) as f64
            || py > source.height_px.saturating_sub(1) as f64
        {
            continue;
        }

        let mut sample = bilinear_sample_rgb(&source.rgb, px, py);
        sample[0] = (sample[0] * source.rgb_gain[0]).clamp(0.0, 255.0);
        sample[1] = (sample[1] * source.rgb_gain[1]).clamp(0.0, 255.0);
        sample[2] = (sample[2] * source.rgb_gain[2]).clamp(0.0, 255.0);

        let edge_weight = image_edge_feather_weight(source, px, py);
        let angle_weight = view_cosine.clamp(0.0, 1.0);
        if angle_weight < 0.45 {
            continue;
        }
        let distance_weight = 1.0 / (1.0 + distance_m * 0.005);
        let weight = edge_weight * angle_weight * distance_weight * source.quality_weight;
        if weight < 0.02 {
            continue;
        }

        let texture_score = image_local_texture_score(source, px, py);

        candidates.push((sample, weight, cam_z, view_cosine, texture_score, source_idx));
    }

    if candidates.is_empty() {
        return None;
    }

    // Occlusion-aware visibility gate: keep only the front-most depth layer.
    candidates.sort_by(|a, b| a.2.total_cmp(&b.2));
    let primary_depth = candidates[0].2;
    let depth_tolerance = (primary_depth * 0.22).max(1.5);

    let mut filtered: Vec<([f64; 3], f64, f64, usize)> = candidates
        .into_iter()
        .filter(|(_, _, cam_z, view_cosine, _, _)| {
            *cam_z <= primary_depth + depth_tolerance || *view_cosine > 0.96
        })
        .map(|(sample, weight, _, _, texture_score, source_idx)| (sample, weight, texture_score, source_idx))
        .collect();

    filtered.sort_by(|a, b| b.1.total_cmp(&a.1));
    if filtered.is_empty() {
        return None;
    }

    let chosen_idx = choose_candidate_index(
        &filtered,
        dense_support_w,
        left_pref,
        up_pref,
        left_rgb,
        up_rgb,
    );

    let (chosen_rgb, chosen_w, _chosen_texture, chosen_src) = filtered[chosen_idx];
    // Find the best alternative source (may differ from filtered[1] after regularization).
    let alt = filtered.iter().copied().find(|(_, _, _, idx)| *idx != chosen_src);
    let alt_weight = alt.map(|(_, w, _, _)| w).unwrap_or(chosen_w * 0.05);
    let dominance_ratio = (chosen_w / alt_weight.max(1e-6)).clamp(1.0, 10.0);

    // Soft-blend the top two sources when they are genuinely competitive.
    // A dominance ratio above 1.5 means one source clearly wins; use it directly.
    // Below 1.5 the two sources are close — blend by weight to smooth seam transitions
    // without introducing 3-way ghosting.
    let low_support_penalty = (1.0 - dense_support_w.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let blend_threshold = 1.35 + 0.45 * low_support_penalty;
    let (rgb, used_sources) = if dominance_ratio > blend_threshold || alt.is_none() {
        (chosen_rgb, 1_u8)
    } else {
        let (alt_rgb, alt_w, _, _) = alt.unwrap();
        let total_w = chosen_w + alt_w;
        let alpha_raw = chosen_w / total_w;
        let alpha = if low_support_penalty > 0.35 {
            (0.25 + 0.5 * alpha_raw).clamp(0.35, 0.65)
        } else {
            alpha_raw
        };
        let blended = [
            chosen_rgb[0] * alpha + alt_rgb[0] * (1.0 - alpha),
            chosen_rgb[1] * alpha + alt_rgb[1] * (1.0 - alpha),
            chosen_rgb[2] * alpha + alt_rgb[2] * (1.0 - alpha),
        ];
        (blended, 2_u8)
    };

    let support_strength = (filtered.len() as f64 / 3.0).clamp(0.35, 1.0);
    let chosen_strength = (chosen_w / 0.30).clamp(0.0, 1.0);
    let dominance_strength = ((dominance_ratio - 1.0) / 2.5).clamp(0.0, 1.0);
    let blend_penalty = if used_sources > 1 { 0.92 - 0.08 * low_support_penalty } else { 1.0 };
    let confidence = ((0.45 * chosen_strength)
        + (0.35 * dominance_strength)
        + (0.20 * support_strength))
        * blend_penalty;

    Some(BlendOutcome {
        rgb,
        used_sources,
        dominance_ratio,
        confidence: confidence.clamp(0.05, 1.0),
        selected_source_idx: chosen_src,
    })
}

fn choose_candidate_index(
    filtered: &[([f64; 3], f64, f64, usize)],
    dense_support_w: f64,
    left_pref: Option<usize>,
    up_pref: Option<usize>,
    left_rgb: Option<[f64; 3]>,
    up_rgb: Option<[f64; 3]>,
) -> usize {
    if filtered.is_empty() {
        return 0;
    }

    let best_weight = filtered[0].1.max(1e-9);
    let low_support = (1.0 - dense_support_w.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if let Some(pref_src) = preferred_source_from_neighbors(left_pref, up_pref) {
        if let Some((pref_idx, (_, pref_weight, _, _))) = filtered
            .iter()
            .enumerate()
            .find(|(_, (_, _, _, src))| *src == pref_src)
        {
            let agreement_bonus = if left_pref.is_some() && up_pref == Some(pref_src) { 0.03 } else { 0.0 };
            // In low-support zones, keep the previous source unless a clear winner emerges.
            let hysteresis_margin = (0.04 + 0.12 * low_support + agreement_bonus).clamp(0.04, 0.18);
            let min_ratio = (1.0 - hysteresis_margin).clamp(0.82, 0.96);
            if *pref_weight >= best_weight * min_ratio {
                return pref_idx;
            }
        }
    }

    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;

    for (idx, (sample, weight, texture_score, source_idx)) in filtered.iter().enumerate() {
        let pref_boost = 0.02 + 0.07 * low_support;
        let mut pref_bonus = 1.0;
        if left_pref == Some(*source_idx) {
            pref_bonus += pref_boost;
        }
        if up_pref == Some(*source_idx) {
            pref_bonus += pref_boost;
        }

        let mut continuity_sum = 0.0;
        let mut continuity_n = 0.0;
        if let Some(rgb) = left_rgb {
            continuity_sum += neighbor_color_consistency(*sample, rgb);
            continuity_n += 1.0;
        }
        if let Some(rgb) = up_rgb {
            continuity_sum += neighbor_color_consistency(*sample, rgb);
            continuity_n += 1.0;
        }
        let continuity = if continuity_n > 0.0 {
            continuity_sum / continuity_n
        } else {
            0.7
        };

        // Only let seam cost influence near-tie candidates; clear geometric winners still win.
        let tie_strength = (*weight / best_weight).clamp(0.0, 1.0);
        let continuity_gain = 0.14 + 0.14 * low_support;
        let continuity_factor = 1.0 + ((continuity - 0.5) * continuity_gain * tie_strength);
        let texture_factor = 1.0 + ((*texture_score - 0.5) * 0.18 * tie_strength);
        let score = *weight * pref_bonus * continuity_factor * texture_factor;
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }

    best_idx
}

fn preferred_source_from_neighbors(left_pref: Option<usize>, up_pref: Option<usize>) -> Option<usize> {
    match (left_pref, up_pref) {
        (Some(l), Some(u)) if l == u => Some(l),
        (Some(l), _) => Some(l),
        (_, Some(u)) => Some(u),
        _ => None,
    }
}

fn neighbor_color_consistency(sample: [f64; 3], neighbor: [f64; 3]) -> f64 {
    let dr = sample[0] - neighbor[0];
    let dg = sample[1] - neighbor[1];
    let db = sample[2] - neighbor[2];
    let dist = (dr * dr + dg * dg + db * db).sqrt();
    let normalized = (dist / (255.0 * 3.0_f64.sqrt())).clamp(0.0, 1.0);
    (1.0 - normalized).clamp(0.1, 1.0)
}

fn rgb_at(raster: &Raster, row: isize, col: isize) -> Option<[f64; 3]> {
    let r = raster.get_opt(0, row, col)?;
    let g = raster.get_opt(1, row, col)?;
    let b = raster.get_opt(2, row, col)?;
    Some([r, g, b])
}

fn image_local_texture_score(source: &SourceImage, px: f64, py: f64) -> f64 {
    let step = 1.5_f64;
    let left = rgb_luminance(bilinear_sample_rgb(&source.rgb, px - step, py));
    let right = rgb_luminance(bilinear_sample_rgb(&source.rgb, px + step, py));
    let up = rgb_luminance(bilinear_sample_rgb(&source.rgb, px, py - step));
    let down = rgb_luminance(bilinear_sample_rgb(&source.rgb, px, py + step));

    let grad = ((right - left).abs() + (down - up).abs()) * 0.5;
    (grad / 48.0).clamp(0.1, 1.0)
}

fn rgb_luminance(rgb: [f64; 3]) -> f64 {
    0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2]
}

fn with_suffix_before_ext(path: &str, suffix: &str) -> String {
    if let Some(stripped) = path.strip_suffix(".tif") {
        format!("{}{}.tif", stripped, suffix)
    } else if let Some(stripped) = path.strip_suffix(".tiff") {
        format!("{}{}.tiff", stripped, suffix)
    } else {
        format!("{}{}", path, suffix)
    }
}

fn project_world_to_image(
    source: &SourceImage,
    x_m: f64,
    y_m: f64,
    z_m: f64,
) -> Option<(f64, f64, f64, f64, f64)> {
    let dx = x_m - source.pose_xyz_m[0];
    let dy = y_m - source.pose_xyz_m[1];
    let dz = z_m - source.pose_xyz_m[2];

    let (rx, ry, rz) = rotate_vector_by_quaternion_inverse(source.rotation, dx, dy, dz);

    let (_legacy_x, _legacy_y, _legacy_z) = if source.use_legacy_projection {
        (rx, -ry, -rz)
    } else {
        (rx, ry, rz)
    };
    let (cam_x, cam_y, cam_z) = (rx, ry, rz);
    if cam_z <= 1e-6 {
        return None;
    }

    let intrinsics = CameraIntrinsics {
        fx: source.fx,
        fy: source.fy,
        cx: source.cx,
        cy: source.cy,
        k1: source.k1,
        k2: source.k2,
        p1: source.p1,
        p2: source.p2,
    };
    let (px, py) = project_camera_ray_to_pixel(
        cam_x / cam_z,
        cam_y / cam_z,
        &intrinsics,
        source.camera_model,
    )?;
    let distance_m = (dx * dx + dy * dy + dz * dz).sqrt();
    let view_cosine = cam_z / distance_m.max(1e-6);

    Some((px, py, view_cosine, distance_m, cam_z))
}

fn rotate_vector_by_quaternion_inverse(q: [f64; 4], x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let w = q[0];
    let qx = q[1];
    let qy = q[2];
    let qz = q[3];

    // Inverse rotation for a unit quaternion is its conjugate.
    let ix = -qx;
    let iy = -qy;
    let iz = -qz;
    let iw = w;

    let tx = 2.0 * (iy * z - iz * y);
    let ty = 2.0 * (iz * x - ix * z);
    let tz = 2.0 * (ix * y - iy * x);

    let rx = x + iw * tx + (iy * tz - iz * ty);
    let ry = y + iw * ty + (iz * tx - ix * tz);
    let rz = z + iw * tz + (ix * ty - iy * tx);
    (rx, ry, rz)
}

fn project_camera_ray_to_pixel(
    xn: f64,
    yn: f64,
    intrinsics: &CameraIntrinsics,
    camera_model: CameraModel,
) -> Option<(f64, f64)> {
    let (x_d, y_d) = match camera_model {
        CameraModel::Pinhole | CameraModel::Auto => {
            let r2 = xn * xn + yn * yn;
            let radial = 1.0 + intrinsics.k1 * r2 + intrinsics.k2 * r2 * r2;
            let x_t = 2.0 * intrinsics.p1 * xn * yn + intrinsics.p2 * (r2 + 2.0 * xn * xn);
            let y_t = intrinsics.p1 * (r2 + 2.0 * yn * yn) + 2.0 * intrinsics.p2 * xn * yn;
            (xn * radial + x_t, yn * radial + y_t)
        }
        CameraModel::Fisheye => {
            let r = (xn * xn + yn * yn).sqrt();
            if r <= 1.0e-12 {
                (xn, yn)
            } else {
                let theta = r.atan();
                let theta2 = theta * theta;
                let theta_d = theta * (1.0 + intrinsics.k1 * theta2 + intrinsics.k2 * theta2 * theta2);
                let scale = theta_d / r;
                (xn * scale, yn * scale)
            }
        }
    };
    if !x_d.is_finite() || !y_d.is_finite() {
        return None;
    }

    let u = intrinsics.fx * x_d + intrinsics.cx;
    let v = intrinsics.fy * y_d + intrinsics.cy;
    if u.is_finite() && v.is_finite() {
        Some((u, v))
    } else {
        None
    }
}

fn intrinsics_for_source(
    alignment: &AlignmentResult,
    frame: &ImageFrame,
    width_px: u32,
    height_px: u32,
) -> (f64, f64, f64, f64) {
    let mut fx = alignment.stats.intrinsics.fx;
    let mut fy = alignment.stats.intrinsics.fy;
    let mut cx = alignment.stats.intrinsics.cx;
    let mut cy = alignment.stats.intrinsics.cy;

    if fx <= 0.0 || fy <= 0.0 {
        fx = width_px as f64 * 0.85;
        fy = height_px as f64 * 0.85;
    }
    if cx <= 0.0 || cy <= 0.0 {
        cx = width_px as f64 * 0.5;
        cy = height_px as f64 * 0.5;
    }

    let ref_w = frame.width.max(1) as f64;
    let ref_h = frame.height.max(1) as f64;
    let scale_x = width_px as f64 / ref_w;
    let scale_y = height_px as f64 / ref_h;

    (
        (fx * scale_x).max(1.0),
        (fy * scale_y).max(1.0),
        (cx * scale_x).clamp(0.0, width_px as f64),
        (cy * scale_y).clamp(0.0, height_px as f64),
    )
}

fn image_channel_means(img: &RgbImage) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for p in img.pixels() {
        let rgb = p.0;
        sum[0] += rgb[0] as f64;
        sum[1] += rgb[1] as f64;
        sum[2] += rgb[2] as f64;
    }
    let denom = img.width().max(1) as f64 * img.height().max(1) as f64;
    [sum[0] / denom, sum[1] / denom, sum[2] / denom]
}

fn global_channel_mean(channel_means: &[[f64; 3]]) -> [f64; 3] {
    if channel_means.is_empty() {
        return [128.0, 128.0, 128.0];
    }
    let mut sum = [0.0; 3];
    for means in channel_means {
        sum[0] += means[0];
        sum[1] += means[1];
        sum[2] += means[2];
    }
    let n = channel_means.len() as f64;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn radiometric_rgb_gains(source_mean: [f64; 3], global_mean: [f64; 3]) -> [f64; 3] {
    [
        (global_mean[0] / source_mean[0].max(1.0)).clamp(0.75, 1.25),
        (global_mean[1] / source_mean[1].max(1.0)).clamp(0.75, 1.25),
        (global_mean[2] / source_mean[2].max(1.0)).clamp(0.75, 1.25),
    ]
}

fn build_pairwise_overlap_edges_from_projected_cells(
    sources: &[SourceImage],
    dsm: &Raster,
) -> Vec<RadiometricEdge> {
    if sources.len() < 2 {
        return Vec::new();
    }
    let mut edges = Vec::new();

    let sample_stride = overlap_sampling_stride(dsm.rows, dsm.cols);
    for idx in 0..sources.len().saturating_sub(1) {
        let a = &sources[idx];
        let b = &sources[idx + 1];

        let mut sum_a = [0.0; 3];
        let mut sum_b = [0.0; 3];
        let mut sample_weight = 0.0;

        let mut row = 0;
        while row < dsm.rows {
            let mut col = 0;
            while col < dsm.cols {
                let z = dsm.get(0, row as isize, col as isize);
                if (z - dsm.nodata).abs() > f64::EPSILON {
                    let x_m = dsm.x_min + (col as f64 + 0.5) * dsm.cell_size_x;
                    let y_m = dsm.y_min + ((dsm.rows - 1 - row) as f64 + 0.5) * dsm.cell_size_y;

                    if let (Some(pa), Some(pb)) = (
                        project_world_to_image(a, x_m, y_m, z),
                        project_world_to_image(b, x_m, y_m, z),
                    ) {
                        let in_a = pa.0 >= 0.0
                            && pa.1 >= 0.0
                            && pa.0 <= a.width_px.saturating_sub(1) as f64
                            && pa.1 <= a.height_px.saturating_sub(1) as f64;
                        let in_b = pb.0 >= 0.0
                            && pb.1 >= 0.0
                            && pb.0 <= b.width_px.saturating_sub(1) as f64
                            && pb.1 <= b.height_px.saturating_sub(1) as f64;
                        if in_a && in_b {
                            let sa = bilinear_sample_rgb(&a.rgb, pa.0, pa.1);
                            let sb = bilinear_sample_rgb(&b.rgb, pb.0, pb.1);
                            let la = rgb_luminance(sa);
                            let lb = rgb_luminance(sb);
                            if la > 8.0 && lb > 8.0 {
                                let weight = pa.2.min(pb.2).clamp(0.3, 1.0);
                                for ch in 0..3 {
                                    sum_a[ch] += sa[ch] * weight;
                                    sum_b[ch] += sb[ch] * weight;
                                }
                                sample_weight += weight;
                            }
                        }
                    }
                }
                col += sample_stride;
            }
            row += sample_stride;
        }

        if sample_weight < 16.0 {
            continue;
        }
        let ratio_b_over_a = [
            (sum_b[0] / sum_a[0].max(1.0)).clamp(0.7, 1.43),
            (sum_b[1] / sum_a[1].max(1.0)).clamp(0.7, 1.43),
            (sum_b[2] / sum_a[2].max(1.0)).clamp(0.7, 1.43),
        ];
        let weight = (sample_weight / 128.0).clamp(0.15, 1.0);

        edges.push(RadiometricEdge {
            a: idx,
            b: idx + 1,
            weight,
            ratio_b_over_a,
            sample_count: sample_weight.round().max(0.0) as u32,
        });
    }

    edges
}

fn overlap_sampling_stride(rows: usize, cols: usize) -> usize {
    let total = (rows.max(1) * cols.max(1)) as f64;
    let target = 10_000.0;
    ((total / target).sqrt().ceil() as usize).max(1)
}

fn refine_pairwise_rgb_gains(sources: &mut [SourceImage], edges: &[RadiometricEdge]) {
    if sources.len() < 2 || edges.is_empty() {
        return;
    }

    let anchor_gains: Vec<[f64; 3]> = sources.iter().map(|s| s.rgb_gain).collect();
    let mut gains = anchor_gains.clone();

    for _ in 0..4 {
        let prev = gains.clone();
        for idx in 0..gains.len() {
            let mut neighbor_sum = [0.0; 3];
            let mut neighbor_weight = 0.0;

            for edge in edges {
                if edge.a == idx {
                    for channel in 0..3 {
                        neighbor_sum[channel] += edge.weight * (prev[edge.b][channel] * edge.ratio_b_over_a[channel]);
                    }
                    neighbor_weight += edge.weight;
                } else if edge.b == idx {
                    for channel in 0..3 {
                        neighbor_sum[channel] += edge.weight * (prev[edge.a][channel] / edge.ratio_b_over_a[channel].max(0.7));
                    }
                    neighbor_weight += edge.weight;
                }
            }

            for channel in 0..3 {
                let anchored = anchor_gains[idx][channel];
                let pairwise_target = if neighbor_weight > 0.0 {
                    neighbor_sum[channel] / neighbor_weight
                } else {
                    anchored
                };
                gains[idx][channel] = (anchored * 0.6 + pairwise_target * 0.4).clamp(0.75, 1.25);
            }
        }
    }

    for (source, gain) in sources.iter_mut().zip(gains) {
        source.rgb_gain = gain;
    }
}

fn image_edge_feather_weight(source: &SourceImage, px: f64, py: f64) -> f64 {
    let dist_left = px;
    let dist_right = source.width_px.saturating_sub(1) as f64 - px;
    let dist_top = py;
    let dist_bottom = source.height_px.saturating_sub(1) as f64 - py;
    let edge_dist = dist_left.min(dist_right).min(dist_top).min(dist_bottom);
    let feather_px = (source.width_px.min(source.height_px) as f64 * 0.08).max(4.0);
    (edge_dist / feather_px).clamp(0.05, 1.0)
}

fn bilinear_sample_rgb(img: &RgbImage, x: f64, y: f64) -> [f64; 3] {
    let max_x = img.width().saturating_sub(1) as f64;
    let max_y = img.height().saturating_sub(1) as f64;
    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);

    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(img.width().saturating_sub(1));
    let y1 = (y0 + 1).min(img.height().saturating_sub(1));

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;

    let mut out = [0.0_f64; 3];
    for b in 0..3 {
        let v00 = p00[b] as f64;
        let v10 = p10[b] as f64;
        let v01 = p01[b] as f64;
        let v11 = p11[b] as f64;

        let top = v00 * (1.0 - fx) + v10 * fx;
        let bot = v01 * (1.0 - fx) + v11 * fx;
        out[b] = top * (1.0 - fy) + bot * fy;
    }

    out
}

fn build_seam_mask(source_choice: &[usize], rows: usize, cols: usize) -> Vec<bool> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }

    let mut seam_mask = vec![false; rows * cols];
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let src = source_choice[idx];
            if src == usize::MAX {
                continue;
            }

            if col + 1 < cols {
                let right_idx = row * cols + (col + 1);
                let right_src = source_choice[right_idx];
                if right_src != usize::MAX && right_src != src {
                    seam_mask[idx] = true;
                    seam_mask[right_idx] = true;
                }
            }
            if row + 1 < rows {
                let down_idx = (row + 1) * cols + col;
                let down_src = source_choice[down_idx];
                if down_src != usize::MAX && down_src != src {
                    seam_mask[idx] = true;
                    seam_mask[down_idx] = true;
                }
            }
        }
    }

    seam_mask
}

fn build_seam_cost_map(
    ortho: &Raster,
    source_choice: &[usize],
    seam_mask: &[bool],
    dense_support: &Option<Raster>,
) -> Vec<f64> {
    let mut costs = vec![1.0; seam_mask.len()];
    if seam_mask.is_empty() {
        return costs;
    }

    for idx in 0..seam_mask.len() {
        if !seam_mask[idx] {
            continue;
        }
        let row = idx / ortho.cols;
        let col = idx % ortho.cols;
        let src = source_choice[idx];
        if src == usize::MAX {
            continue;
        }

        let mut transition_delta = 0.0;
        let mut transition_n = 0.0;
        let mut local_grad = 0.0;
        let mut grad_n = 0.0;
        let center = match luminance_at(ortho, row as isize, col as isize) {
            Some(v) => v,
            None => continue,
        };

        for (rr, cc) in [
            (row as isize - 1, col as isize),
            (row as isize + 1, col as isize),
            (row as isize, col as isize - 1),
            (row as isize, col as isize + 1),
        ] {
            if rr < 0 || cc < 0 || rr >= ortho.rows as isize || cc >= ortho.cols as isize {
                continue;
            }
            let nidx = rr as usize * ortho.cols + cc as usize;
            if let Some(v) = luminance_at(ortho, rr, cc) {
                let delta = ((center - v).abs() / 255.0).clamp(0.0, 1.0);
                local_grad += delta;
                grad_n += 1.0;
                if source_choice[nidx] != usize::MAX && source_choice[nidx] != src {
                    transition_delta += delta;
                    transition_n += 1.0;
                }
            }
        }

        let transition = if transition_n > 0.0 {
            transition_delta / transition_n
        } else {
            1.0
        };
        let gradient = if grad_n > 0.0 { local_grad / grad_n } else { 0.0 };
        let support_w = dense_support_weight(dense_support, row, col);
        let low_support = (1.0 - support_w).clamp(0.0, 1.0);
        let support_penalty = 1.0 + 0.60 * low_support;
        let cost = (0.55 + 0.45 * transition) * (1.10 - 0.35 * gradient).clamp(0.65, 1.10) * support_penalty;
        costs[idx] = cost.clamp(0.1, 2.0);
    }

    costs
}

fn optimize_seamline_paths(
    seam_mask: &[bool],
    rows: usize,
    cols: usize,
    costs: &[f64],
) -> Vec<bool> {
    if seam_mask.is_empty() || rows == 0 || cols == 0 || costs.len() != seam_mask.len() {
        return seam_mask.to_vec();
    }

    let mut optimized = vec![false; seam_mask.len()];
    let mut visited = vec![false; seam_mask.len()];

    for start in 0..seam_mask.len() {
        if !seam_mask[start] || visited[start] {
            continue;
        }

        let component = collect_component_indices(start, seam_mask, rows, cols, &mut visited);
        if component.len() < 3 {
            for idx in component {
                optimized[idx] = true;
            }
            continue;
        }

        let endpoints = seam_component_endpoints(&component, seam_mask, rows, cols);
        let (start_idx, end_idx) = if endpoints.len() >= 2 {
            (endpoints[0], endpoints[1])
        } else {
            farthest_component_pair(&component, cols)
        };

        let path = shortest_path_within_component(start_idx, end_idx, &component, rows, cols, costs);
        if path.len() >= 2 {
            for idx in path {
                optimized[idx] = true;
            }
        } else {
            for idx in component {
                optimized[idx] = true;
            }
        }
    }

    optimized
}

fn collect_component_indices(
    start: usize,
    seam_mask: &[bool],
    rows: usize,
    cols: usize,
    visited: &mut [bool],
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut q = VecDeque::new();
    visited[start] = true;
    q.push_back(start);

    while let Some(idx) = q.pop_front() {
        out.push(idx);
        let row = idx / cols;
        let col = idx % cols;
        for (rr, cc) in [
            (row as isize - 1, col as isize),
            (row as isize + 1, col as isize),
            (row as isize, col as isize - 1),
            (row as isize, col as isize + 1),
        ] {
            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                continue;
            }
            let nidx = rr as usize * cols + cc as usize;
            if seam_mask[nidx] && !visited[nidx] {
                visited[nidx] = true;
                q.push_back(nidx);
            }
        }
    }

    out
}

fn seam_component_endpoints(component: &[usize], seam_mask: &[bool], rows: usize, cols: usize) -> Vec<usize> {
    let mut endpoints = Vec::new();
    for idx in component {
        let row = *idx / cols;
        let col = *idx % cols;
        let mut degree = 0;
        for (rr, cc) in [
            (row as isize - 1, col as isize),
            (row as isize + 1, col as isize),
            (row as isize, col as isize - 1),
            (row as isize, col as isize + 1),
        ] {
            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                continue;
            }
            let nidx = rr as usize * cols + cc as usize;
            if seam_mask[nidx] {
                degree += 1;
            }
        }
        if degree <= 1 {
            endpoints.push(*idx);
        }
    }
    endpoints
}

fn farthest_component_pair(component: &[usize], cols: usize) -> (usize, usize) {
    let mut best = (component[0], component[0]);
    let mut best_dist = 0usize;
    for i in 0..component.len() {
        let ai = component[i];
        let ar = ai / cols;
        let ac = ai % cols;
        for bj in component.iter().skip(i + 1) {
            let br = *bj / cols;
            let bc = *bj % cols;
            let dist = ar.abs_diff(br) + ac.abs_diff(bc);
            if dist > best_dist {
                best_dist = dist;
                best = (ai, *bj);
            }
        }
    }
    best
}

fn shortest_path_within_component(
    start_idx: usize,
    end_idx: usize,
    component: &[usize],
    rows: usize,
    cols: usize,
    costs: &[f64],
) -> Vec<usize> {
    if start_idx == end_idx {
        return vec![start_idx];
    }

    let mut node_pos: HashMap<usize, usize> = HashMap::with_capacity(component.len());
    for (pos, idx) in component.iter().enumerate() {
        node_pos.insert(*idx, pos);
    }

    let mut dist = vec![f64::INFINITY; component.len()];
    let mut prev = vec![usize::MAX; component.len()];
    let mut used = vec![false; component.len()];

    let start_pos = match node_pos.get(&start_idx) {
        Some(v) => *v,
        None => return Vec::new(),
    };
    let end_pos = match node_pos.get(&end_idx) {
        Some(v) => *v,
        None => return Vec::new(),
    };
    dist[start_pos] = costs[start_idx];

    loop {
        let mut best_pos = usize::MAX;
        let mut best_dist = f64::INFINITY;
        for i in 0..component.len() {
            if !used[i] && dist[i] < best_dist {
                best_dist = dist[i];
                best_pos = i;
            }
        }
        if best_pos == usize::MAX || best_pos == end_pos {
            break;
        }

        used[best_pos] = true;
        let idx = component[best_pos];
        let row = idx / cols;
        let col = idx % cols;

        for (rr, cc) in [
            (row as isize - 1, col as isize),
            (row as isize + 1, col as isize),
            (row as isize, col as isize - 1),
            (row as isize, col as isize + 1),
        ] {
            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                continue;
            }
            let nidx = rr as usize * cols + cc as usize;
            let npos = match node_pos.get(&nidx) {
                Some(v) => *v,
                None => continue,
            };
            if used[npos] {
                continue;
            }
            let edge_cost = (costs[idx] + costs[nidx]) * 0.5;
            let cand = dist[best_pos] + edge_cost;
            if cand < dist[npos] {
                dist[npos] = cand;
                prev[npos] = best_pos;
            }
        }
    }

    if !dist[end_pos].is_finite() {
        return Vec::new();
    }

    let mut path = Vec::new();
    let mut cur = end_pos;
    loop {
        path.push(component[cur]);
        if cur == start_pos {
            break;
        }
        cur = prev[cur];
        if cur == usize::MAX {
            return Vec::new();
        }
    }
    path.reverse();
    path
}

fn apply_seam_softening(ortho: &mut Raster, seam_mask: &[bool]) -> Result<()> {
    if ortho.cols < 3 || ortho.rows == 0 || seam_mask.is_empty() {
        return Ok(());
    }

    let mut smoothed = vec![ortho.nodata; ortho.bands * ortho.rows * ortho.cols];
    for band in 0..ortho.bands {
        let band_idx = band as isize;
        for row in 0..ortho.rows {
            for col in 0..ortho.cols {
                let idx = row * ortho.cols + col;
                let out_idx = band * ortho.rows * ortho.cols + idx;
                let current = ortho.get(band_idx, row as isize, col as isize);
                if !seam_mask[idx] || (current - ortho.nodata).abs() <= f64::EPSILON {
                    smoothed[out_idx] = current;
                    continue;
                }

                let mut weighted_sum = 0.0;
                let mut weight_total = 0.0;
                for dr in -1..=1 {
                    for dc in -1..=1 {
                        let rr = row as isize + dr;
                        let cc = col as isize + dc;
                        if rr < 0 || cc < 0 || rr >= ortho.rows as isize || cc >= ortho.cols as isize {
                            continue;
                        }
                        let v = ortho.get(band_idx, rr, cc);
                        if (v - ortho.nodata).abs() <= f64::EPSILON {
                            continue;
                        }
                        let dist2 = (dr * dr + dc * dc) as f64;
                        let weight = match dist2 as i32 {
                            0 => 0.40,
                            1 => 0.15,
                            2 => 0.075,
                            _ => 0.0,
                        };
                        weighted_sum += weight * v;
                        weight_total += weight;
                    }
                }

                smoothed[out_idx] = if weight_total > 0.0 {
                    (weighted_sum / weight_total).clamp(0.0, 255.0)
                } else {
                    current
                };
            }
        }
    }

    for band in 0..ortho.bands {
        let band_idx = band as isize;
        for row in 0..ortho.rows {
            for col in 0..ortho.cols {
                let idx = band * ortho.rows * ortho.cols + row * ortho.cols + col;
                ortho.set(band_idx, row as isize, col as isize, smoothed[idx])?;
            }
        }
    }

    Ok(())
}

fn seam_boundary_deltas(ortho: &Raster, source_choice: &[usize], seam_mask: &[bool]) -> Vec<f64> {
    if ortho.cols < 2 || ortho.rows == 0 || seam_mask.is_empty() {
        return Vec::new();
    }

    let mut visited = vec![false; seam_mask.len()];
    let mut deltas = Vec::new();

    for start in 0..seam_mask.len() {
        if !seam_mask[start] || visited[start] {
            continue;
        }

        let mut q = VecDeque::new();
        q.push_back(start);
        visited[start] = true;
        let mut sum = 0.0;
        let mut n = 0_u64;

        while let Some(idx) = q.pop_front() {
            let row = idx / ortho.cols;
            let col = idx % ortho.cols;
            let src = source_choice[idx];
            if src == usize::MAX {
                continue;
            }

            let center = luminance_at(ortho, row as isize, col as isize);
            if let Some(a) = center {
                for (rr, cc) in [
                    (row as isize - 1, col as isize),
                    (row as isize + 1, col as isize),
                    (row as isize, col as isize - 1),
                    (row as isize, col as isize + 1),
                ] {
                    if rr < 0 || cc < 0 || rr >= ortho.rows as isize || cc >= ortho.cols as isize {
                        continue;
                    }
                    let nidx = rr as usize * ortho.cols + cc as usize;
                    if seam_mask[nidx] && !visited[nidx] {
                        visited[nidx] = true;
                        q.push_back(nidx);
                    }
                    let nsrc = source_choice[nidx];
                    if nsrc == usize::MAX || nsrc == src {
                        continue;
                    }
                    if let Some(b) = luminance_at(ortho, rr, cc) {
                        sum += ((a - b).abs() / 255.0).clamp(0.0, 1.0);
                        n += 1;
                    }
                }
            }
        }

        if n > 0 {
            deltas.push((sum / n as f64).clamp(0.0, 1.0));
        }
    }

    deltas
}

fn luminance_at(raster: &Raster, row: isize, col: isize) -> Option<f64> {
    let r = raster.get_opt(0, row, col)?;
    let g = raster.get_opt(1, row, col)?;
    let b = raster.get_opt(2, row, col)?;
    Some((0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0))
}

fn dsm_min_max_count(dsm: &Raster) -> (f64, f64, u64) {
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut count = 0_u64;

    for row in 0..dsm.rows {
        for col in 0..dsm.cols {
            let v = dsm.get(0, row as isize, col as isize);
            if (v - dsm.nodata).abs() <= f64::EPSILON {
                continue;
            }
            min_z = min_z.min(v);
            max_z = max_z.max(v);
            count += 1;
        }
    }

    (min_z, max_z, count)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::{ImageBuffer, Rgb};

    use super::*;
    use crate::alignment::{AlignmentResult, AlignmentStats, CameraPose};
    use crate::camera::{CameraIntrinsics, CameraModel};
    use wbraster::CrsInfo;

    fn temp_path(prefix: &str, suffix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("{}_{}.{}", prefix, nanos, suffix))
            .to_string_lossy()
            .to_string()
    }

    fn sample_alignment() -> AlignmentResult {
        AlignmentResult {
            poses: vec![
                CameraPose {
                    position: [0.0, 0.0, 120.0],
                    rotation: [1.0, 0.0, 0.0, 0.0],
                    reprojection_error_px: 0.7,
                },
                CameraPose {
                    position: [10.0, 2.0, 121.0],
                    rotation: [1.0, 0.0, 0.0, 0.05],
                    reprojection_error_px: 0.7,
                },
                CameraPose {
                    position: [20.0, 4.0, 122.0],
                    rotation: [1.0, 0.0, 0.0, 0.10],
                    reprojection_error_px: 0.7,
                },
            ],
            crs: CrsInfo::from_epsg(32617),
            stats: AlignmentStats {
                aligned_fraction: 1.0,
                rmse_px: 0.8,
                residual_p50_px: 0.40,
                residual_p95_px: 1.16,
                sparse_cloud_points: 4_200,
                tie_points_median: 82,
                tracks_median: 4.3,
                mean_parallax_px: 6.6,
                estimated_gsd_m: 0.5,
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

    fn sample_frames(count: usize) -> Vec<ImageFrame> {
        (0..count)
            .map(|i| {
                let path = temp_path(&format!("mosaic_img_{}", i), "png");
                let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(64, 48);
                for y in 0..48 {
                    for x in 0..64 {
                        let r = ((x as f64 / 63.0) * 255.0) as u8;
                        let g = ((y as f64 / 47.0) * 255.0) as u8;
                        let b = (40 + (i as u8 * 60)).min(255);
                        img.put_pixel(x, y, Rgb([r, g, b]));
                    }
                }
                img.save(&path).expect("save frame");

                ImageFrame {
                    path,
                    width: 64,
                    height: 48,
                    metadata: crate::ingest::FrameMetadata {
                        gps: None,
                        focal_length_mm: Some(8.8),
                        sensor_width_mm: Some(13.2),
                        image_width_px: 64,
                        image_height_px: 48,
                        timestamp: None,
                        orientation_prior: None,
                        blur_score: None,
                        has_rtk_gps: false,
                    },
                }
            })
            .collect()
    }

    fn write_sample_dsm(path: &str) {
        let mut dsm = Raster::new(RasterConfig {
            cols: 12,
            rows: 8,
            x_min: 100.0,
            y_min: 200.0,
            cell_size: 0.5,
            data_type: DataType::F32,
            crs: CrsInfo::from_epsg(32617),
            ..RasterConfig::default()
        });

        for row in 0..dsm.rows {
            for col in 0..dsm.cols {
                let z = 110.0 + row as f64 * 0.6 + col as f64 * 0.4;
                dsm.set(0, row as isize, col as isize, z).expect("set dsm");
            }
        }
        dsm.write(path, RasterFormat::GeoTiff).expect("write dsm");
    }

    #[test]
    fn empty_alignment_writes_rgb_placeholder() {
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

        let dsm_path = temp_path("mosaic_empty_dsm", "tif");
        let ortho_path = temp_path("mosaic_empty_ortho", "tif");
        write_sample_dsm(&dsm_path);

        let result = run_orthomosaic(&alignment, &[], &dsm_path, &ortho_path).expect("mosaic run");
        assert_eq!(result.stats.seam_segments, 0);

        let ortho = Raster::read(&ortho_path).expect("read ortho");
        assert_eq!(ortho.bands, 3);
        assert_eq!(ortho.cols, 12);
        assert_eq!(ortho.rows, 8);

        let _ = std::fs::remove_file(dsm_path);
        let _ = std::fs::remove_file(ortho_path);
    }

    #[test]
    fn image_projected_orthomosaic_reports_measured_stats() {
        let alignment = sample_alignment();
        let frames = sample_frames(3);
        let dsm_path = temp_path("mosaic_dsm", "tif");
        let ortho_path = temp_path("mosaic_ortho", "tif");
        write_sample_dsm(&dsm_path);

        let result = run_orthomosaic(&alignment, &frames, &dsm_path, &ortho_path).expect("mosaic run");
        assert!(result.stats.seam_segments <= (result.projected_cells + result.uncovered_cells));
        assert!(result.stats.max_seam_delta >= result.stats.mean_seam_delta);
        assert!((0.0..=1.0).contains(&result.stats.max_seam_delta));
        assert!(result.gsd_m > 0.0);
        assert!(result.projected_cells + result.uncovered_cells > 0);
        assert!((0.0..=100.0).contains(&result.coverage_pct));

        let ortho = Raster::read(&ortho_path).expect("read ortho");
        assert_eq!(ortho.cols, 12);
        assert_eq!(ortho.rows, 8);
        assert_eq!(ortho.bands, 3);
        assert_eq!(ortho.crs.epsg, Some(32617));

        assert!(result.uncovered_cells <= result.projected_cells + result.uncovered_cells);

        let conf_path = temp_path("mosaic_confidence", "tif");
        let result_with_conf = run_orthomosaic_with_confidence(
            &alignment,
            &frames,
            &dsm_path,
            &ortho_path,
            Some(&conf_path),
        )
        .expect("mosaic run with confidence");
        assert_eq!(result_with_conf.confidence_raster_path.as_deref(), Some(conf_path.as_str()));
        let conf = Raster::read(&conf_path).expect("read confidence mosaic");
        assert_eq!(conf.bands, 1);
        assert_eq!(conf.cols, 12);
        assert_eq!(conf.rows, 8);

        for frame in &frames {
            let _ = std::fs::remove_file(&frame.path);
        }
        let _ = std::fs::remove_file(dsm_path);
        let _ = std::fs::remove_file(ortho_path);
        let _ = std::fs::remove_file(conf_path);
    }

    #[test]
    fn bilinear_sampling_stays_in_range() {
        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                img.put_pixel(x, y, Rgb([x as u8 * 32, y as u8 * 64, 128]));
            }
        }

        let sample = bilinear_sample_rgb(&img, 1.5, 1.5);
        assert!((0.0..=255.0).contains(&sample[0]));
        assert!((0.0..=255.0).contains(&sample[1]));
        assert!((0.0..=255.0).contains(&sample[2]));
    }

    #[test]
    fn local_texture_score_distinguishes_flat_and_textured_regions() {
        let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(12, 12);
        for y in 0..12 {
            for x in 0..12 {
                let value = if x < 6 { 100 } else { ((x * 20 + y * 7) % 255) as u8 };
                img.put_pixel(x, y, Rgb([value, value, value]));
            }
        }

        let source = SourceImage {
            rgb: img,
            width_px: 12,
            height_px: 12,
            pose_xyz_m: [0.0, 0.0, 100.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            fx: 10.0,
            fy: 10.0,
            cx: 6.0,
            cy: 6.0,
            rgb_gain: [1.0, 1.0, 1.0],
            quality_weight: 1.0,
            use_legacy_projection: false,
            camera_model: CameraModel::Pinhole,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let flat = image_local_texture_score(&source, 2.0, 6.0);
        let textured = image_local_texture_score(&source, 9.0, 6.0);
        assert!(textured > flat);
    }

    #[test]
    fn source_transition_seams_drive_detection_and_stats() {
        let rows = 3;
        let cols = 4;
        let source_choice = vec![
            0, 0, 1, 1,
            0, 0, 1, 1,
            0, 0, 1, 1,
        ];
        let seam_mask = build_seam_mask(&source_choice, rows, cols);
        assert!(seam_mask.iter().any(|v| *v));

        let mut ortho = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 3,
            data_type: DataType::F32,
            ..RasterConfig::default()
        });
        for row in 0..rows {
            for col in 0..cols {
                let value = if col < 2 { 25.0 } else { 225.0 };
                for band in 0..3 {
                    ortho.set(band, row as isize, col as isize, value).expect("set ortho");
                }
            }
        }

        let deltas = seam_boundary_deltas(&ortho, &source_choice, &seam_mask);
        assert_eq!(deltas.len(), 1);
        assert!(deltas[0] > 0.5);

        apply_seam_softening(&mut ortho, &seam_mask).expect("soften seams");
        let left = luminance_at(&ortho, 1, 1).expect("left luminance");
        let right = luminance_at(&ortho, 1, 2).expect("right luminance");
        assert!(left > 25.0);
        assert!(right < 225.0);
    }

    #[test]
    fn seam_cost_selection_prefers_color_consistent_near_tie() {
        let candidates = vec![
            ([200.0, 200.0, 200.0], 0.98, 0.55, 0usize),
            ([32.0, 30.0, 28.0], 0.95, 0.55, 1usize),
        ];

        let chosen = choose_candidate_index(
            &candidates,
            1.0,
            Some(1),
            None,
            Some([30.0, 28.0, 26.0]),
            None,
        );
        assert_eq!(chosen, 1);

        let clear_winner = vec![
            ([200.0, 200.0, 200.0], 1.25, 0.55, 0usize),
            ([32.0, 30.0, 28.0], 0.75, 0.55, 1usize),
        ];
        let chosen_clear = choose_candidate_index(
            &clear_winner,
            1.0,
            Some(1),
            None,
            Some([30.0, 28.0, 26.0]),
            None,
        );
        assert_eq!(chosen_clear, 0);

        let texture_tie = vec![
            ([120.0, 120.0, 120.0], 0.97, 0.25, 0usize),
            ([122.0, 122.0, 122.0], 0.96, 0.95, 1usize),
        ];
        let chosen_textured = choose_candidate_index(
            &texture_tie,
            1.0,
            None,
            None,
            None,
            None,
        );
        assert_eq!(chosen_textured, 1);

        let hysteresis_near_tie = vec![
            ([100.0, 100.0, 100.0], 1.00, 0.55, 0usize),
            ([99.0, 99.0, 99.0], 0.88, 0.55, 1usize),
        ];
        let chosen_low_support = choose_candidate_index(
            &hysteresis_near_tie,
            0.10,
            Some(1),
            Some(0),
            Some([98.0, 98.0, 98.0]),
            Some([98.0, 98.0, 98.0]),
        );
        assert_eq!(chosen_low_support, 1);

        let chosen_high_support = choose_candidate_index(
            &hysteresis_near_tie,
            1.00,
            Some(1),
            Some(0),
            Some([98.0, 98.0, 98.0]),
            Some([98.0, 98.0, 98.0]),
        );
        assert_eq!(chosen_high_support, 0);
    }

    #[test]
    fn radiometric_rgb_gains_reduce_channel_bias_between_sources() {
        let cool_mean = [80.0, 100.0, 150.0];
        let warm_mean = [150.0, 110.0, 70.0];
        let global = global_channel_mean(&[cool_mean, warm_mean]);

        let cool_gain = radiometric_rgb_gains(cool_mean, global);
        let warm_gain = radiometric_rgb_gains(warm_mean, global);

        let cool_adjusted = [
            cool_mean[0] * cool_gain[0],
            cool_mean[1] * cool_gain[1],
            cool_mean[2] * cool_gain[2],
        ];
        let warm_adjusted = [
            warm_mean[0] * warm_gain[0],
            warm_mean[1] * warm_gain[1],
            warm_mean[2] * warm_gain[2],
        ];

        let before_diff = (cool_mean[0] - warm_mean[0]).abs()
            + (cool_mean[1] - warm_mean[1]).abs()
            + (cool_mean[2] - warm_mean[2]).abs();
        let after_diff = (cool_adjusted[0] - warm_adjusted[0]).abs()
            + (cool_adjusted[1] - warm_adjusted[1]).abs()
            + (cool_adjusted[2] - warm_adjusted[2]).abs();

        assert!(after_diff < before_diff);
        assert!(cool_gain.iter().all(|g| *g >= 0.75 && *g <= 1.25));
        assert!(warm_gain.iter().all(|g| *g >= 0.75 && *g <= 1.25));
    }

    #[test]
    fn pairwise_overlap_refinement_reduces_adjacent_bias() {
        let make_source = |rgb_gain: [f64; 3], x: f64| SourceImage {
            rgb: ImageBuffer::from_pixel(10, 10, Rgb([128, 128, 128])),
            width_px: 10,
            height_px: 10,
            pose_xyz_m: [x, 0.0, 100.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            fx: 10.0,
            fy: 10.0,
            cx: 5.0,
            cy: 5.0,
            rgb_gain,
            quality_weight: 1.0,
            use_legacy_projection: false,
            camera_model: CameraModel::Pinhole,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };

        let mut sources = vec![
            make_source([1.25, 1.10, 0.90], 0.0),
            make_source([0.85, 0.95, 1.20], 1.0),
            make_source([0.80, 0.90, 1.22], 2.0),
        ];
        let before = pairwise_gain_delta(&sources[0].rgb_gain, &sources[1].rgb_gain)
            + pairwise_gain_delta(&sources[1].rgb_gain, &sources[2].rgb_gain);
        let edges = vec![
            RadiometricEdge {
                a: 0,
                b: 1,
                weight: 0.7,
                ratio_b_over_a: [0.96, 0.98, 1.02],
                sample_count: 80,
            },
            RadiometricEdge {
                a: 1,
                b: 2,
                weight: 0.65,
                ratio_b_over_a: [0.99, 1.01, 1.03],
                sample_count: 72,
            },
        ];

        refine_pairwise_rgb_gains(&mut sources, &edges);

        let after = pairwise_gain_delta(&sources[0].rgb_gain, &sources[1].rgb_gain)
            + pairwise_gain_delta(&sources[1].rgb_gain, &sources[2].rgb_gain);
        assert!(after < before);
        assert!(sources
            .iter()
            .all(|source| source.rgb_gain.iter().all(|gain| *gain >= 0.75 && *gain <= 1.25)));
    }

    fn pairwise_gain_delta(a: &[f64; 3], b: &[f64; 3]) -> f64 {
        (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()
    }

    #[test]
    fn projected_overlap_edges_capture_pairwise_channel_ratio() {
        let mk_src = |rgb: [u8; 3], x: f64| SourceImage {
            rgb: ImageBuffer::from_pixel(28, 24, Rgb(rgb)),
            width_px: 28,
            height_px: 24,
            pose_xyz_m: [x, 0.0, 100.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
            fx: 28.0,
            fy: 24.0,
            cx: 14.0,
            cy: 12.0,
            rgb_gain: [1.0, 1.0, 1.0],
            quality_weight: 1.0,
            use_legacy_projection: true,
            camera_model: CameraModel::Pinhole,
            k1: 0.0,
            k2: 0.0,
            p1: 0.0,
            p2: 0.0,
        };
        let sources = vec![mk_src([100, 90, 80], 0.0), mk_src([120, 99, 72], 1.2)];

        let mut dsm = Raster::new(RasterConfig {
            cols: 20,
            rows: 20,
            x_min: -2.0,
            y_min: -2.0,
            cell_size: 0.5,
            data_type: DataType::F32,
            ..RasterConfig::default()
        });
        for row in 0..dsm.rows {
            for col in 0..dsm.cols {
                dsm.set(0, row as isize, col as isize, 92.0).expect("set dsm");
            }
        }

        let edges = build_pairwise_overlap_edges_from_projected_cells(&sources, &dsm);
        assert_eq!(edges.len(), 1);
        let ratio = edges[0].ratio_b_over_a;
        assert!(edges[0].sample_count > 0);
        assert!((ratio[0] - 1.20).abs() < 0.15);
        assert!((ratio[1] - 1.10).abs() < 0.15);
        assert!((ratio[2] - 0.90).abs() < 0.15);
    }

    #[test]
    fn seamline_optimization_reduces_component_to_explicit_path() {
        let rows = 6;
        let cols = 6;
        let mut seam_mask = vec![false; rows * cols];
        for row in 0..rows {
            for col in 2..4 {
                seam_mask[row * cols + col] = true;
            }
        }
        let mut costs = vec![1.0; rows * cols];
        for row in 0..rows {
            costs[row * cols + 2] = 0.2;
            costs[row * cols + 3] = 1.2;
        }

        let optimized = optimize_seamline_paths(&seam_mask, rows, cols, &costs);
        let seam_count = optimized.iter().filter(|v| **v).count();
        assert!(seam_count <= rows + 1);
        let mut low_cost_count = 0usize;
        let mut high_cost_count = 0usize;
        for row in 0..rows {
            if optimized[row * cols + 2] {
                low_cost_count += 1;
            }
            if optimized[row * cols + 3] {
                high_cost_count += 1;
            }
        }
        assert!(low_cost_count >= rows.saturating_sub(1));
        assert!(high_cost_count <= 1);
    }
}
