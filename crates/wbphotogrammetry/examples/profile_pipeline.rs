use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use image::{GrayImage, Luma};
use serde_json::json;
use wbphotogrammetry::{
    AlignmentOptions, CameraModel, FeatureMethod, FrameMetadata, GpsCoordinate, ImageFrame,
    IntrinsicsRefinementPolicy, run_camera_alignment_with_options, run_dense_surface,
    run_feature_matching_with_method, run_orthomosaic,
};

#[derive(Debug, Clone)]
struct RunTimes {
    feature: Duration,
    alignment: Duration,
    dense: Duration,
    mosaic: Duration,
}

#[derive(Debug, Clone, Copy)]
struct StageSummary {
    mean_s: f64,
    p50_s: f64,
    p95_s: f64,
}

#[derive(Debug, Clone, Copy)]
struct PipelineSummary {
    feature: StageSummary,
    alignment: StageSummary,
    dense: StageSummary,
    mosaic: StageSummary,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let frame_count = parse_arg(&args, "--frames", 90_usize);
    let repeats = parse_arg(&args, "--repeats", 3_usize);
    let profile = parse_string_arg(&args, "--profile", "balanced");
    let feature_method = parse_string_arg(&args, "--feature-method", "rootsift")
        .parse::<FeatureMethod>()
        .unwrap_or_else(|e| {
            eprintln!("invalid --feature-method: {e}");
            std::process::exit(2);
        });
    let intrinsics_refinement = parse_intrinsics_refinement_policy(
        &parse_string_arg(&args, "--intrinsics-refinement", "auto"),
    );
    let resolution_m = parse_arg(&args, "--resolution", 0.12_f64);
    let json_out = parse_optional_string_arg(&args, "--json-out");

    let tmp_root = temp_workspace("wbphotogrammetry_profile");
    let images_dir = tmp_root.join("images");
    fs::create_dir_all(&images_dir).expect("failed to create images dir");

    println!("Generating {} synthetic frames in {}", frame_count, images_dir.display());
    let frames = generate_synthetic_frames(&images_dir, frame_count);

    let mut runs = Vec::with_capacity(repeats);
    for run_idx in 0..repeats {
        let dsm_path = tmp_root.join(format!("dsm_run_{}.tif", run_idx));
        let ortho_path = tmp_root.join(format!("ortho_run_{}.tif", run_idx));

        let start_feature = Instant::now();
        let match_stats = run_feature_matching_with_method(&frames, &profile, feature_method)
            .expect("feature stage");
        let feature_time = start_feature.elapsed();

        let start_alignment = Instant::now();
        let alignment = run_camera_alignment_with_options(
            &frames,
            &match_stats,
            CameraModel::Auto,
            AlignmentOptions {
                intrinsics_refinement,
            },
        )
        .expect("alignment stage");
        let alignment_time = start_alignment.elapsed();

        let start_dense = Instant::now();
        let _dense = run_dense_surface(&alignment, resolution_m, &dsm_path.to_string_lossy())
            .expect("dense stage");
        let dense_time = start_dense.elapsed();

        let start_mosaic = Instant::now();
        let _mosaic = run_orthomosaic(
            &alignment,
            &frames,
            &dsm_path.to_string_lossy(),
            &ortho_path.to_string_lossy(),
        )
        .expect("mosaic stage");
        let mosaic_time = start_mosaic.elapsed();

        println!(
            "Run {}: feature={:.3}s, alignment={:.3}s, dense={:.3}s, mosaic={:.3}s",
            run_idx + 1,
            feature_time.as_secs_f64(),
            alignment_time.as_secs_f64(),
            dense_time.as_secs_f64(),
            mosaic_time.as_secs_f64()
        );

        runs.push(RunTimes {
            feature: feature_time,
            alignment: alignment_time,
            dense: dense_time,
            mosaic: mosaic_time,
        });
    }

    let summary = summarize_pipeline(&runs);
    print_summary(&summary);

    if let Some(path) = json_out {
        let json_path = PathBuf::from(path);
        write_json_report(
            &json_path,
            frame_count,
            repeats,
            &profile,
            feature_method,
            resolution_m,
            &tmp_root,
            &runs,
            &summary,
        );
        println!("Wrote JSON profile report: {}", json_path.display());
    }

    println!("Temporary profiling workspace: {}", tmp_root.display());
    println!("Delete it when done if you want to reclaim disk space.");
}

fn summarize_pipeline(runs: &[RunTimes]) -> PipelineSummary {
    PipelineSummary {
        feature: summarize_stage(runs.iter().map(|r| r.feature).collect()),
        alignment: summarize_stage(runs.iter().map(|r| r.alignment).collect()),
        dense: summarize_stage(runs.iter().map(|r| r.dense).collect()),
        mosaic: summarize_stage(runs.iter().map(|r| r.mosaic).collect()),
    }
}

fn print_summary(summary: &PipelineSummary) {

    println!("\n--- Stage Timing Summary ---");
    println!(
        "feature:   mean={:.3}s p50={:.3}s p95={:.3}s",
        summary.feature.mean_s, summary.feature.p50_s, summary.feature.p95_s
    );
    println!(
        "alignment: mean={:.3}s p50={:.3}s p95={:.3}s",
        summary.alignment.mean_s, summary.alignment.p50_s, summary.alignment.p95_s
    );
    println!(
        "dense:     mean={:.3}s p50={:.3}s p95={:.3}s",
        summary.dense.mean_s, summary.dense.p50_s, summary.dense.p95_s
    );
    println!(
        "mosaic:    mean={:.3}s p50={:.3}s p95={:.3}s",
        summary.mosaic.mean_s, summary.mosaic.p50_s, summary.mosaic.p95_s
    );
}

fn summarize_stage(mut samples: Vec<Duration>) -> StageSummary {
    if samples.is_empty() {
        return StageSummary {
            mean_s: 0.0,
            p50_s: 0.0,
            p95_s: 0.0,
        };
    }
    samples.sort();
    let mean = samples.iter().map(|d| d.as_secs_f64()).sum::<f64>() / samples.len() as f64;
    let p50 = percentile(&samples, 0.50).as_secs_f64();
    let p95 = percentile(&samples, 0.95).as_secs_f64();
    StageSummary {
        mean_s: mean,
        p50_s: p50,
        p95_s: p95,
    }
}

fn write_json_report(
    output_path: &PathBuf,
    frame_count: usize,
    repeats: usize,
    profile: &str,
    feature_method: FeatureMethod,
    resolution_m: f64,
    tmp_root: &PathBuf,
    runs: &[RunTimes],
    summary: &PipelineSummary,
) {
    let generated_utc = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let run_entries = runs
        .iter()
        .enumerate()
        .map(|(idx, run)| {
            json!({
                "run_index": idx + 1,
                "feature_s": run.feature.as_secs_f64(),
                "alignment_s": run.alignment.as_secs_f64(),
                "dense_s": run.dense.as_secs_f64(),
                "mosaic_s": run.mosaic.as_secs_f64()
            })
        })
        .collect::<Vec<_>>();

    let payload = json!({
        "schema_version": "1.0.0",
        "generated_utc_epoch_s": generated_utc,
        "config": {
            "frame_count": frame_count,
            "repeats": repeats,
            "profile": profile,
            "feature_method": feature_method.as_str(),
            "resolution_m": resolution_m,
            "workspace": tmp_root.to_string_lossy(),
        },
        "summary": {
            "feature": {"mean_s": summary.feature.mean_s, "p50_s": summary.feature.p50_s, "p95_s": summary.feature.p95_s},
            "alignment": {"mean_s": summary.alignment.mean_s, "p50_s": summary.alignment.p50_s, "p95_s": summary.alignment.p95_s},
            "dense": {"mean_s": summary.dense.mean_s, "p50_s": summary.dense.p50_s, "p95_s": summary.dense.p95_s},
            "mosaic": {"mean_s": summary.mosaic.mean_s, "p50_s": summary.mosaic.p50_s, "p95_s": summary.mosaic.p95_s},
        },
        "runs": run_entries,
    });

    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let text = serde_json::to_string_pretty(&payload).expect("failed to serialize json report");
    fs::write(output_path, format!("{}\n", text)).expect("failed writing json profile report");
}

fn percentile(samples: &[Duration], p: f64) -> Duration {
    if samples.is_empty() {
        return Duration::from_secs(0);
    }
    let idx = ((samples.len() - 1) as f64 * p).round() as usize;
    samples[idx.min(samples.len() - 1)]
}

fn generate_synthetic_frames(images_dir: &PathBuf, frame_count: usize) -> Vec<ImageFrame> {
    let mut frames = Vec::with_capacity(frame_count);
    let base_lat = 45.000_000_f64;
    let base_lon = -81.000_000_f64;
    let base_alt = 120.0_f64;

    for i in 0..frame_count {
        let path = images_dir.join(format!("frame_{:04}.png", i));
        write_test_frame(&path, i as u32);

        let gps = GpsCoordinate {
            lat: base_lat + (i as f64 * 0.000_004),
            lon: base_lon + (i as f64 * 0.000_004),
            alt: base_alt + ((i as f64) * 0.02),
        };

        frames.push(ImageFrame {
            path: path.to_string_lossy().to_string(),
            width: 640,
            height: 480,
            metadata: FrameMetadata {
                gps: Some(gps),
                focal_length_mm: Some(8.8),
                sensor_width_mm: Some(13.2),
                image_width_px: 640,
                image_height_px: 480,
                timestamp: None,
                orientation_prior: None,
                blur_score: Some(0.08),
                has_rtk_gps: false,
            },
        });
    }

    frames
}

fn write_test_frame(path: &PathBuf, offset: u32) {
    let mut img = GrayImage::new(640, 480);
    for y in 0..480 {
        for x in 0..640 {
            let checker: u8 = if ((x / 16) + (y / 16)) % 2 == 0 { 35 } else { 220 };
            let stripe = (((x + offset * 2) / 13) % 3) as u8 * 8;
            let radial = (((x as i32 - 320).pow(2) + (y as i32 - 240).pow(2)) % 29) as u8;
            let value = checker.saturating_add(stripe).saturating_add(radial / 3);
            img.put_pixel(x, y, Luma([value]));
        }
    }

    let line_x = 60 + (offset % 40);
    for y in 40..440 {
        img.put_pixel(line_x, y, Luma([255]));
    }

    img.save(path).expect("failed writing profiling frame");
}

fn parse_arg<T: std::str::FromStr>(args: &[String], key: &str, default: T) -> T {
    args.windows(2)
        .find(|w| w[0] == key)
        .and_then(|w| w[1].parse::<T>().ok())
        .unwrap_or(default)
}

fn parse_string_arg(args: &[String], key: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.to_string())
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

    fn parse_optional_string_arg(args: &[String], key: &str) -> Option<String> {
        args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
    }

fn temp_workspace(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{}_{}", prefix, nanos));
    fs::create_dir_all(&dir).expect("failed to create temporary profiling workspace");
    dir
}
