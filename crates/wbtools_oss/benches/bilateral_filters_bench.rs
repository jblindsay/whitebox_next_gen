use criterion::{black_box, criterion_group, criterion_main, Criterion};
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

fn make_continuous_input(rows: usize, cols: usize) -> String {
    let cfg = RasterConfig {
        rows,
        cols,
        bands: 1,
        nodata: -9999.0,
        ..Default::default()
    };
    let mut raster = Raster::new(cfg);

    for r in 0..rows as isize {
        for c in 0..cols as isize {
            let value = ((r * 29 + c * 41).rem_euclid(2000) as f64) / 20.0;
            raster
                .set(0, r, c, value)
                .expect("failed to populate bilateral benchmark raster");
        }
    }

    let id = memory_store::put_raster(raster);
    memory_store::make_raster_memory_path(&id)
}

fn make_args(input_path: &str, sigma_dist: f64, sigma_int: f64) -> ToolArgs {
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path));
    args.insert("sigma_dist".to_string(), json!(sigma_dist));
    args.insert("sigma_int".to_string(), json!(sigma_int));
    args
}

fn bench_bilateral_filters(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let continuous_input = make_continuous_input(256, 256);
    let bilateral_args = make_args(&continuous_input, 1.5, 12.0);
    let high_pass_args = make_args(&continuous_input, 1.5, 12.0);

    let mut group = c.benchmark_group("wbtools_oss/bilateral_filters");
    group.sample_size(15);

    group.bench_function("bilateral_sigma1.5_12.0", |b| {
        b.iter(|| {
            let out = registry
                .run_tool("bilateral_filter", black_box(&bilateral_args), black_box(&ctx))
                .expect("bilateral_filter benchmark run failed");
            black_box(out);
        })
    });

    group.bench_function("high_pass_bilateral_sigma1.5_12.0", |b| {
        b.iter(|| {
            let out = registry
                .run_tool(
                    "high_pass_bilateral_filter",
                    black_box(&high_pass_args),
                    black_box(&ctx),
                )
                .expect("high_pass_bilateral_filter benchmark run failed");
            black_box(out);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_bilateral_filters);
criterion_main!(benches);
