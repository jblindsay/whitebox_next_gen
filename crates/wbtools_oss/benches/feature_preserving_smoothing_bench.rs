use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use wbcore::{AllowAllCapabilities, ProgressSink, ToolArgs, ToolContext, ToolRuntimeRegistry};
use wbraster::{Raster, RasterConfig};
use wbtools_oss::{memory_store, register_default_tools, ToolRegistry};

struct NoopProgress;
impl ProgressSink for NoopProgress {}

fn make_ctx() -> ToolContext<'static> {
    static PROGRESS: NoopProgress = NoopProgress;
    static CAPS: AllowAllCapabilities = AllowAllCapabilities;
    ToolContext {
        progress: &PROGRESS,
        capabilities: &CAPS,
    }
}

/// Generates a synthetic DEM using a sum of sine waves to approximate a
/// realistic terrain surface with ridges and valleys.
fn make_dem(rows: usize, cols: usize) -> String {
    let cfg = RasterConfig {
        rows,
        cols,
        bands: 1,
        nodata: -9999.0,
        ..Default::default()
    };
    let mut raster = Raster::new(cfg);

    let rows_f = rows as f64;
    let cols_f = cols as f64;

    for r in 0..rows as isize {
        for c in 0..cols as isize {
            let x = c as f64 / cols_f;
            let y = r as f64 / rows_f;
            // Layered sinusoids create ridge/valley terrain with varying curvature
            let z = 200.0
                + 80.0 * (std::f64::consts::TAU * x * 3.0).sin()
                + 60.0 * (std::f64::consts::TAU * y * 2.5).cos()
                + 30.0 * (std::f64::consts::TAU * (x + y) * 5.0).sin()
                + 10.0 * (std::f64::consts::TAU * x * 11.0).cos()
                + 5.0 * (std::f64::consts::TAU * y * 13.0).sin();
            raster
                .set(0, r, c, z)
                .expect("failed to populate DEM benchmark raster");
        }
    }

    let id = memory_store::put_raster(raster);
    memory_store::make_raster_memory_path(&id)
}

fn make_args(input_path: &str, filter_size: u64, normal_diff_threshold: f64, iterations: u64) -> ToolArgs {
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path));
    args.insert("filter_size".to_string(), json!(filter_size));
    args.insert("normal_diff_threshold".to_string(), json!(normal_diff_threshold));
    args.insert("iterations".to_string(), json!(iterations));
    args
}

fn bench_feature_preserving_smoothing(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let mut group = c.benchmark_group("wbtools_oss/feature_preserving_smoothing");
    group.sample_size(10);

    // --- Grid size scaling ---
    for size in [128usize, 256, 512] {
        let input = make_dem(size, size);
        let args = make_args(&input, 11, 8.0, 3);
        group.bench_with_input(
            BenchmarkId::new("default_params", size),
            &size,
            |b, _| {
                b.iter(|| {
                    let out = registry
                        .run_tool(
                            "feature_preserving_smoothing",
                            black_box(&args),
                            black_box(&ctx),
                        )
                        .expect("feature_preserving_smoothing benchmark run failed");
                    black_box(out);
                })
            },
        );
    }

    // --- Iteration count sensitivity (fixed 256×256 grid) ---
    let input_256 = make_dem(256, 256);
    for iters in [1u64, 3, 8] {
        let args = make_args(&input_256, 11, 8.0, iters);
        group.bench_with_input(
            BenchmarkId::new("iters", iters),
            &iters,
            |b, _| {
                b.iter(|| {
                    let out = registry
                        .run_tool(
                            "feature_preserving_smoothing",
                            black_box(&args),
                            black_box(&ctx),
                        )
                        .expect("feature_preserving_smoothing benchmark run failed");
                    black_box(out);
                })
            },
        );
    }

    // --- Filter size sensitivity (fixed 256×256 grid, 3 iters) ---
    let input_256b = make_dem(256, 256);
    for fs in [3u64, 7, 11, 15] {
        let args = make_args(&input_256b, fs, 8.0, 3);
        group.bench_with_input(
            BenchmarkId::new("filter_size", fs),
            &fs,
            |b, _| {
                b.iter(|| {
                    let out = registry
                        .run_tool(
                            "feature_preserving_smoothing",
                            black_box(&args),
                            black_box(&ctx),
                        )
                        .expect("feature_preserving_smoothing benchmark run failed");
                    black_box(out);
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_feature_preserving_smoothing);
criterion_main!(benches);
