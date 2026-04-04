use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;
use wbphotogrammetry::{
    AlignmentOptions, CameraModel, FeatureMethod, IntrinsicsRefinementPolicy,
    ReducedCameraSolveMode,
    ProfileThresholds, ProcessingProfile, build_qa_report,
    ingest_image_set, run_camera_alignment_with_options, run_feature_matching_with_method,
    run_orthomosaic_with_confidence,
};

#[derive(Debug, Serialize)]
struct PipelineTiming {
    ingest_s: f64,
    feature_s: f64,
    alignment_s: f64,
    dense_s: f64,
    mosaic_s: f64,
}

#[derive(Debug, Serialize)]
struct PipelineSummary {
    images_dir: String,
    output_dir: String,
    profile: String,
    feature_method: String,
    camera_model: String,
    reduced_solver_mode: String,
    resolution_m: f64,
    frame_count: usize,
    outputs: PipelineOutputs,
    timing: PipelineTiming,
    match_stats: wbphotogrammetry::MatchStats,
    alignment_stats: wbphotogrammetry::AlignmentStats,
    dsm_stats: wbphotogrammetry::DsmStats,
    mosaic_stats: wbphotogrammetry::SeamStats,
    mosaic_coverage_pct: f64,
    qa: wbphotogrammetry::QaReport,
}

#[derive(Debug, Serialize)]
struct PipelineOutputs {
    dsm_path: String,
    support_path: Option<String>,
    uncertainty_path: Option<String>,
    confidence_path: String,
    ortho_path: String,
    mosaic_support_diagnostics_path: Option<String>,
    mosaic_source_index_path: Option<String>,
    camera_poses_geojson_path: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let images_dir = required_arg(&args, "--images-dir");
    let output_dir = PathBuf::from(required_arg(&args, "--out-dir"));
    let profile = optional_arg(&args, "--profile").unwrap_or_else(|| "balanced".to_string());
    let feature_method = optional_arg(&args, "--feature-method")
        .unwrap_or_else(|| "rootsift".to_string())
        .parse::<FeatureMethod>()
        .unwrap_or_else(|e| {
            eprintln!("invalid --feature-method: {e}");
            std::process::exit(2);
        });
    let camera_model = parse_camera_model(
        optional_arg(&args, "--camera-model").unwrap_or_else(|| "auto".to_string()).as_str(),
    );
    let intrinsics_refinement = parse_intrinsics_refinement_policy(
        optional_arg(&args, "--intrinsics-refinement")
            .unwrap_or_else(|| "auto".to_string())
            .as_str(),
    );
    let reduced_camera_solve_mode = parse_reduced_camera_solve_mode(
        optional_arg(&args, "--reduced-solver-mode")
            .unwrap_or_else(|| "sparse-pcg".to_string())
            .as_str(),
    );
    let resolution_m = optional_arg(&args, "--resolution")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.12);

    let processing_profile = ProcessingProfile::from_str(&profile).unwrap_or_else(|| {
        eprintln!("invalid --profile: {profile}. expected one of fast|balanced|survey");
        std::process::exit(2);
    });

    fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        eprintln!(
            "failed to create output directory '{}': {e}",
            output_dir.display()
        );
        std::process::exit(1);
    });

    let tag = format!("{}_{}", profile, feature_method.as_str());
    let dsm_path = output_dir.join(format!("{}_dsm.tif", tag));
    let ortho_path = output_dir.join(format!("{}_ortho.tif", tag));
    let confidence_path = output_dir.join(format!("{}_ortho_confidence.tif", tag));
    let report_path = output_dir.join(format!("{}_report.json", tag));
    let camera_poses_geojson_path = output_dir.join(format!("{}_camera_poses.geojson", tag));

    println!("Running wbphotogrammetry dataset pipeline");
    println!("images_dir: {images_dir}");
    println!("output_dir: {}", output_dir.display());
    println!("profile: {profile}");
    println!("feature_method: {}", feature_method.as_str());
    println!("camera_model: {}", camera_model_name(camera_model));
    println!(
        "intrinsics_refinement: {}",
        intrinsics_refinement_name(intrinsics_refinement)
    );
    println!(
        "reduced_solver_mode: {}",
        reduced_solver_mode_name(reduced_camera_solve_mode)
    );
    println!("resolution_m: {resolution_m:.3}");

    let ingest_start = Instant::now();
    let frames = ingest_image_set(&images_dir).unwrap_or_else(|e| {
        eprintln!("ingest failed: {e}");
        std::process::exit(1);
    });
    let ingest_s = ingest_start.elapsed().as_secs_f64();
    println!("ingested {} frames in {:.3}s", frames.len(), ingest_s);

    let feature_start = Instant::now();
    let match_stats = run_feature_matching_with_method(&frames, &profile, feature_method)
        .unwrap_or_else(|e| {
            eprintln!("feature stage failed: {e}");
            std::process::exit(1);
        });
    let feature_s = feature_start.elapsed().as_secs_f64();
    println!("feature stage done in {:.3}s", feature_s);

    let alignment_start = Instant::now();
    let alignment = run_camera_alignment_with_options(
        &frames,
        &match_stats,
        camera_model,
        AlignmentOptions {
            intrinsics_refinement,
            reduced_camera_solve_mode,
        },
    )
    .unwrap_or_else(|e| {
        eprintln!("alignment stage failed: {e}");
        std::process::exit(1);
    });
    let alignment_s = alignment_start.elapsed().as_secs_f64();
    println!("alignment stage done in {:.3}s", alignment_s);

    // Export camera poses as GeoJSON for visual inspection
    wbphotogrammetry::export_camera_poses_as_geojson(
        &alignment,
        &frames,
        &camera_poses_geojson_path.to_string_lossy(),
    )
    .unwrap_or_else(|e| {
        eprintln!("warning: failed to export camera poses geojson: {e}");
    });

    let dense_start = Instant::now();
    let dense = wbphotogrammetry::dense::run_dense_surface_with_frames(
        &alignment,
        &frames,
        resolution_m,
        &dsm_path.to_string_lossy(),
    )
    .unwrap_or_else(|e| {
        eprintln!("dense stage failed: {e}");
        std::process::exit(1);
    });
    let dense_s = dense_start.elapsed().as_secs_f64();
    println!("dense stage done in {:.3}s", dense_s);

    let mosaic_start = Instant::now();
    let mosaic = run_orthomosaic_with_confidence(
        &alignment,
        &frames,
        &dense.dsm_path,
        &ortho_path.to_string_lossy(),
        Some(&confidence_path.to_string_lossy()),
    )
    .unwrap_or_else(|e| {
        eprintln!("mosaic stage failed: {e}");
        std::process::exit(1);
    });
    let mosaic_s = mosaic_start.elapsed().as_secs_f64();
    println!("mosaic stage done in {:.3}s", mosaic_s);

    let thresholds = ProfileThresholds::for_profile(processing_profile);
    let qa = build_qa_report(
        &match_stats,
        &alignment.stats,
        &dense.stats,
        &mosaic,
        thresholds,
    );

    let summary = PipelineSummary {
        images_dir,
        output_dir: output_dir.to_string_lossy().to_string(),
        profile,
        feature_method: feature_method.as_str().to_string(),
        camera_model: camera_model_name(camera_model).to_string(),
        reduced_solver_mode: reduced_solver_mode_name(reduced_camera_solve_mode).to_string(),
        resolution_m,
        frame_count: frames.len(),
        outputs: PipelineOutputs {
            dsm_path: dense.dsm_path.clone(),
            support_path: dense.support_raster_path.clone(),
            uncertainty_path: dense.uncertainty_raster_path.clone(),
            confidence_path: confidence_path.to_string_lossy().to_string(),
            ortho_path: mosaic.ortho_path.clone(),
            mosaic_support_diagnostics_path: mosaic.support_diagnostics_path.clone(),
            mosaic_source_index_path: mosaic.source_index_raster_path.clone(),
            camera_poses_geojson_path: camera_poses_geojson_path.to_string_lossy().to_string(),
        },
        timing: PipelineTiming {
            ingest_s,
            feature_s,
            alignment_s,
            dense_s,
            mosaic_s,
        },
        match_stats,
        alignment_stats: alignment.stats,
        dsm_stats: dense.stats,
        mosaic_stats: mosaic.stats,
        mosaic_coverage_pct: mosaic.coverage_pct,
        qa,
    };

    let text = serde_json::to_string_pretty(&summary).unwrap_or_else(|e| {
        eprintln!("failed to serialize report json: {e}");
        std::process::exit(1);
    });
    fs::write(&report_path, format!("{}\n", text)).unwrap_or_else(|e| {
        eprintln!("failed writing report '{}': {e}", report_path.display());
        std::process::exit(1);
    });

    println!("report: {}", report_path.display());
    println!("ortho: {}", ortho_path.display());
    println!("dsm: {}", dsm_path.display());
    println!("camera_poses: {}", camera_poses_geojson_path.display());
    println!("qa_status: {}", summary.qa.status.as_str());
    println!("qa_actions: {}", summary.qa.recommended_actions.len());
    println!("coverage_pct: {:.2}", summary.mosaic_coverage_pct);
}

fn required_arg(args: &[String], key: &str) -> String {
    optional_arg(args, key).unwrap_or_else(|| {
        eprintln!("missing required argument: {key} <value>");
        std::process::exit(2);
    })
}

fn optional_arg(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find_map(|w| (w[0] == key).then(|| w[1].to_string()))
}

fn parse_camera_model(s: &str) -> CameraModel {
    match s {
        "auto" => CameraModel::Auto,
        "pinhole" => CameraModel::Pinhole,
        "fisheye" => CameraModel::Fisheye,
        _ => {
            eprintln!("invalid --camera-model: {s}. expected auto|pinhole|fisheye");
            std::process::exit(2);
        }
    }
}

fn camera_model_name(model: CameraModel) -> &'static str {
    match model {
        CameraModel::Auto => "auto",
        CameraModel::Pinhole => "pinhole",
        CameraModel::Fisheye => "fisheye",
    }
}

fn parse_intrinsics_refinement_policy(s: &str) -> IntrinsicsRefinementPolicy {
    match s {
        "auto" => IntrinsicsRefinementPolicy::Auto,
        "none" => IntrinsicsRefinementPolicy::None,
        "core-only" => IntrinsicsRefinementPolicy::CoreOnly,
        "core-and-radial" => IntrinsicsRefinementPolicy::CoreAndRadial,
        "all" => IntrinsicsRefinementPolicy::All,
        _ => {
            eprintln!(
                "invalid --intrinsics-refinement: {s}. expected auto|none|core-only|core-and-radial|all"
            );
            std::process::exit(2);
        }
    }
}

fn parse_reduced_camera_solve_mode(s: &str) -> ReducedCameraSolveMode {
    match s {
        "sparse-pcg" => ReducedCameraSolveMode::SparsePcg,
        "dense-lu" => ReducedCameraSolveMode::DenseLu,
        _ => {
            eprintln!("invalid --reduced-solver-mode: {s}. expected sparse-pcg|dense-lu");
            std::process::exit(2);
        }
    }
}

fn intrinsics_refinement_name(policy: IntrinsicsRefinementPolicy) -> &'static str {
    match policy {
        IntrinsicsRefinementPolicy::Auto => "auto",
        IntrinsicsRefinementPolicy::None => "none",
        IntrinsicsRefinementPolicy::CoreOnly => "core-only",
        IntrinsicsRefinementPolicy::CoreAndRadial => "core-and-radial",
        IntrinsicsRefinementPolicy::All => "all",
    }
}

fn reduced_solver_mode_name(mode: ReducedCameraSolveMode) -> &'static str {
    match mode {
        ReducedCameraSolveMode::SparsePcg => "sparse-pcg",
        ReducedCameraSolveMode::DenseLu => "dense-lu",
    }
}