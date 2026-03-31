use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use wbcore::{AllowAllCapabilities, ProgressSink, ToolArgs, ToolContext, ToolRuntimeRegistry};
use wbraster::{Raster, RasterConfig};
use wbtools_oss::{memory_store, register_default_tools, ToolRegistry};

#[derive(Clone, Copy)]
enum RankBenchOp {
    Percentile,
    Majority,
    Diversity,
}

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

fn make_input(rows: usize, cols: usize, categorical: bool) -> String {
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
            let value = if categorical {
                ((r * 17 + c * 31).rem_euclid(7) as f64) + 1.0
            } else {
                ((r * 37 + c * 19).rem_euclid(1000) as f64) / 10.0
            };
            raster.set(0, r, c, value).expect("failed to populate benchmark raster");
        }
    }

    let id = memory_store::put_raster(raster);
    memory_store::make_raster_memory_path(&id)
}

fn make_args(input_path: &str, filter_size: u64, sig_digits: Option<i64>) -> ToolArgs {
    let mut args = ToolArgs::new();
    args.insert("input".to_string(), json!(input_path));
    args.insert("filter_size_x".to_string(), json!(filter_size));
    args.insert("filter_size_y".to_string(), json!(filter_size));
    if let Some(digits) = sig_digits {
        args.insert("sig_digits".to_string(), json!(digits));
    }
    args
}

fn make_values(rows: usize, cols: usize, categorical: bool) -> Vec<f64> {
    let mut values = vec![0.0; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            values[r * cols + c] = if categorical {
                ((r * 17 + c * 31) % 7) as f64 + 1.0
            } else {
                ((r * 37 + c * 19) % 1000) as f64 / 10.0
            };
        }
    }
    values
}

fn run_rank_kernel_legacy(
    values: &[f64],
    rows: usize,
    cols: usize,
    filter_size: usize,
    sig_digits: i32,
    op: RankBenchOp,
) -> f64 {
    let half = (filter_size / 2) as isize;
    let multiplier_rank = 10.0f64.powi(sig_digits);
    let mut bins = Vec::<i64>::with_capacity(filter_size * filter_size);
    let mut sum_out = 0.0;

    for r in 0..rows as isize {
        for c in 0..cols as isize {
            bins.clear();
            for ny in (r - half)..=(r + half) {
                if ny < 0 || ny >= rows as isize {
                    continue;
                }
                for nx in (c - half)..=(c + half) {
                    if nx < 0 || nx >= cols as isize {
                        continue;
                    }
                    let z = values[ny as usize * cols + nx as usize];
                    match op {
                        RankBenchOp::Percentile => {
                            bins.push((z * multiplier_rank).floor() as i64);
                        }
                        RankBenchOp::Majority => {
                            bins.push((z * 100.0).floor() as i64);
                        }
                        RankBenchOp::Diversity => {
                            bins.push((z * 1000.0).floor() as i64);
                        }
                    }
                }
            }

            if bins.is_empty() {
                continue;
            }

            let out = match op {
                RankBenchOp::Percentile => {
                    let center = values[r as usize * cols + c as usize];
                    let center_bin = (center * multiplier_rank).floor() as i64;
                    let n_less = bins.iter().filter(|&&v| v < center_bin).count();
                    n_less as f64 / bins.len() as f64 * 100.0
                }
                RankBenchOp::Majority => {
                    let mut frequencies = HashMap::<i64, usize>::new();
                    for &bin in &bins {
                        *frequencies.entry(bin).or_insert(0) += 1;
                    }
                    let mut mode_bin = bins[0];
                    let mut mode_freq = 0usize;
                    for (bin, freq) in frequencies {
                        if freq > mode_freq {
                            mode_freq = freq;
                            mode_bin = bin;
                        }
                    }
                    mode_bin as f64 / 100.0
                }
                RankBenchOp::Diversity => {
                    let mut unique = HashSet::<i64>::new();
                    for &bin in &bins {
                        unique.insert(bin);
                    }
                    unique.len() as f64
                }
            };
            sum_out += out;
        }
    }

    sum_out
}

fn run_rank_kernel_optimized(
    values: &[f64],
    rows: usize,
    cols: usize,
    filter_size: usize,
    sig_digits: i32,
    op: RankBenchOp,
) -> f64 {
    let half = (filter_size / 2) as isize;
    let multiplier_rank = 10.0f64.powi(sig_digits);
    let mut bins = Vec::<i64>::with_capacity(filter_size * filter_size);
    let mut sum_out = 0.0;

    for r in 0..rows as isize {
        for c in 0..cols as isize {
            bins.clear();
            let center_bin_rank = (values[r as usize * cols + c as usize] * multiplier_rank).floor() as i64;
            let mut count = 0usize;
            let mut n_less = 0usize;

            for ny in (r - half)..=(r + half) {
                if ny < 0 || ny >= rows as isize {
                    continue;
                }
                for nx in (c - half)..=(c + half) {
                    if nx < 0 || nx >= cols as isize {
                        continue;
                    }
                    let z = values[ny as usize * cols + nx as usize];
                    match op {
                        RankBenchOp::Percentile => {
                            let q = (z * multiplier_rank).floor() as i64;
                            if q < center_bin_rank {
                                n_less += 1;
                            }
                            count += 1;
                        }
                        RankBenchOp::Majority => {
                            bins.push((z * 100.0).floor() as i64);
                        }
                        RankBenchOp::Diversity => {
                            bins.push((z * 1000.0).floor() as i64);
                        }
                    }
                }
            }

            if matches!(op, RankBenchOp::Percentile) {
                if count > 0 {
                    sum_out += n_less as f64 / count as f64 * 100.0;
                }
                continue;
            }

            if bins.is_empty() {
                continue;
            }

            let out = match op {
                RankBenchOp::Percentile => unreachable!(),
                RankBenchOp::Majority => {
                    bins.sort_unstable();
                    let mut mode_bin = bins[0];
                    let mut mode_freq = 1usize;
                    let mut run_bin = bins[0];
                    let mut run_freq = 1usize;

                    for &bin in bins.iter().skip(1) {
                        if bin == run_bin {
                            run_freq += 1;
                        } else {
                            if run_freq > mode_freq {
                                mode_freq = run_freq;
                                mode_bin = run_bin;
                            }
                            run_bin = bin;
                            run_freq = 1;
                        }
                    }

                    if run_freq > mode_freq {
                        mode_bin = run_bin;
                    }

                    mode_bin as f64 / 100.0
                }
                RankBenchOp::Diversity => {
                    bins.sort_unstable();
                    let mut unique = 1usize;
                    let mut prev = bins[0];
                    for &bin in bins.iter().skip(1) {
                        if bin != prev {
                            unique += 1;
                            prev = bin;
                        }
                    }
                    unique as f64
                }
            };
            sum_out += out;
        }
    }

    sum_out
}

fn bench_rank_filters_kernel_compare(c: &mut Criterion) {
    let rows = 160usize;
    let cols = 160usize;
    let filter = 21usize;
    let cont_values = make_values(rows, cols, false);
    let cat_values = make_values(rows, cols, true);

    let mut group = c.benchmark_group("wbtools_oss/rank_filters_kernel_compare");
    group.sample_size(12);

    group.bench_function("percentile_legacy_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_legacy(
                black_box(&cont_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Percentile),
            );
            black_box(out);
        })
    });

    group.bench_function("percentile_optimized_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_optimized(
                black_box(&cont_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Percentile),
            );
            black_box(out);
        })
    });

    group.bench_function("majority_legacy_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_legacy(
                black_box(&cat_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Majority),
            );
            black_box(out);
        })
    });

    group.bench_function("majority_optimized_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_optimized(
                black_box(&cat_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Majority),
            );
            black_box(out);
        })
    });

    group.bench_function("diversity_legacy_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_legacy(
                black_box(&cat_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Diversity),
            );
            black_box(out);
        })
    });

    group.bench_function("diversity_optimized_11x11", |b| {
        b.iter(|| {
            let out = run_rank_kernel_optimized(
                black_box(&cat_values),
                black_box(rows),
                black_box(cols),
                black_box(filter),
                black_box(2),
                black_box(RankBenchOp::Diversity),
            );
            black_box(out);
        })
    });

    group.finish();
}

fn bench_rank_filters(c: &mut Criterion) {
    let mut registry = ToolRegistry::new();
    register_default_tools(&mut registry);
    let ctx = make_ctx();

    let continuous_input = make_input(256, 256, false);
    let categorical_input = make_input(256, 256, true);

    let mut group = c.benchmark_group("wbtools_oss/rank_filters");
    group.sample_size(20);

    let median_args = make_args(&continuous_input, 11, Some(2));
    group.bench_function("median_11x11", |b| {
        b.iter(|| {
            let out = registry
                .run_tool("median_filter", black_box(&median_args), black_box(&ctx))
                .expect("median_filter benchmark run failed");
            black_box(out);
        })
    });

    let percentile_args = make_args(&continuous_input, 11, Some(2));
    group.bench_function("percentile_11x11", |b| {
        b.iter(|| {
            let out = registry
                .run_tool("percentile_filter", black_box(&percentile_args), black_box(&ctx))
                .expect("percentile_filter benchmark run failed");
            black_box(out);
        })
    });

    let majority_args = make_args(&categorical_input, 11, None);
    group.bench_function("majority_11x11", |b| {
        b.iter(|| {
            let out = registry
                .run_tool("majority_filter", black_box(&majority_args), black_box(&ctx))
                .expect("majority_filter benchmark run failed");
            black_box(out);
        })
    });

    let diversity_args = make_args(&categorical_input, 11, None);
    group.bench_function("diversity_11x11", |b| {
        b.iter(|| {
            let out = registry
                .run_tool("diversity_filter", black_box(&diversity_args), black_box(&ctx))
                .expect("diversity_filter benchmark run failed");
            black_box(out);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_rank_filters, bench_rank_filters_kernel_compare);
criterion_main!(benches);
