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

fn make_dem(rows: usize, cols: usize) -> String {
    let cfg = RasterConfig {
        rows,
        cols,
        bands: 1,
        nodata: -9999.0,
        cell_size: 10.0,
        ..Default::default()
    };
    let mut raster = Raster::new(cfg);

    let rows_f = rows as f64;
    let cols_f = cols as f64;
    for r in 0..rows as isize {
        for c in 0..cols as isize {
            let x = c as f64 / cols_f;
            let y = r as f64 / rows_f;
            let z = 400.0
                + 90.0 * (std::f64::consts::TAU * x * 2.0).sin()
                + 70.0 * (std::f64::consts::TAU * y * 3.0).cos()
                + 22.0 * (std::f64::consts::TAU * (x + y) * 7.0).sin()
                + 8.0 * (std::f64::consts::TAU * x * 11.0).cos();
            raster
                .set(0, r, c, z)
                .expect("failed to populate geomorphons benchmark raster");
        }
    }

    let id = memory_store::put_raster(raster);
    memory_store::make_raster_memory_path(&id)
}

fn make_args(input_path: &str, search_distance: u64, output_forms: bool) -> ToolArgs {
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path));
    args.insert("search_distance".to_string(), json!(search_distance));
    args.insert("flatness_threshold".to_string(), json!(1.0));
    args.insert("flatness_distance".to_string(), json!(0));
    args.insert("skip_distance".to_string(), json!(0));
    args.insert("output_forms".to_string(), json!(output_forms));
    args
}

fn bench_geomorphons(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let mut group = c.benchmark_group("wbtools_oss/geomorphons");
    group.sample_size(10);

    for size in [256usize, 512] {
        let input = make_dem(size, size);
        for search in [25u64, 50] {
            let forms_args = make_args(&input, search, true);
            group.bench_with_input(
                BenchmarkId::new("forms", format!("{size}x{size}_s{search}")),
                &(size, search),
                |b, _| {
                    b.iter(|| {
                        let out = registry
                            .run_tool("geomorphons", black_box(&forms_args), black_box(&ctx))
                            .expect("geomorphons forms benchmark run failed");
                        black_box(out);
                    })
                },
            );

            let raw_args = make_args(&input, search, false);
            group.bench_with_input(
                BenchmarkId::new("raw_code", format!("{size}x{size}_s{search}")),
                &(size, search),
                |b, _| {
                    b.iter(|| {
                        let out = registry
                            .run_tool("geomorphons", black_box(&raw_args), black_box(&ctx))
                            .expect("geomorphons raw-code benchmark run failed");
                        black_box(out);
                    })
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_geomorphons);
criterion_main!(benches);
