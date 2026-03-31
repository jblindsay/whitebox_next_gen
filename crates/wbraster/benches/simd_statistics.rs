use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wbraster::{
    CrsInfo, DataType, NodataPolicy, Raster, RasterConfig, ReprojectOptions,
    ResampleMethod, StatisticsComputationMode,
};

fn make_raster(cols: usize, rows: usize) -> Raster {
    let nodata = -32768.0;
    let data: Vec<f64> = (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                if (row + col) % 10 == 0 {
                    nodata
                } else {
                    ((row * cols + col) % 500) as f64
                }
            })
        })
        .collect();

    let cfg = RasterConfig {
        cols,
        rows,
        bands: 1,
        x_min: 0.0,
        y_min: 0.0,
        cell_size: 1.0,
        cell_size_y: Some(-1.0),
        nodata,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    };

    Raster::from_data(cfg, data).expect("failed to build benchmark raster")
}

fn bench_full_statistics(c: &mut Criterion) {
    let raster = make_raster(1000, 1000);
    let mut group = c.benchmark_group("full_statistics");

    for mode in [
        StatisticsComputationMode::Scalar,
        StatisticsComputationMode::Simd,
    ] {
        group.bench_with_input(BenchmarkId::new("mode", format!("{:?}", mode)), &mode, |b, mode| {
            b.iter(|| black_box(raster.statistics_with_mode(*mode)))
        });
    }

    group.finish();
}

fn bench_band_statistics(c: &mut Criterion) {
    let raster = make_raster(1000, 1000);
    let mut group = c.benchmark_group("band_statistics");

    for mode in [
        StatisticsComputationMode::Scalar,
        StatisticsComputationMode::Simd,
    ] {
        group.bench_with_input(BenchmarkId::new("mode", format!("{:?}", mode)), &mode, |b, mode| {
            b.iter(|| black_box(raster.statistics_band_with_mode(0, *mode).expect("band statistics failed")))
        });
    }

    group.finish();
}

fn bench_bilinear_sampling(c: &mut Criterion) {
    let raster = make_raster(1024, 1024);
    let samples: Vec<(f64, f64)> = (0..10_000)
        .map(|i| {
            let x = 2.25 + ((i % 500) as f64 * 0.5);
            let y = 2.75 + (((i / 500) % 20) as f64 * 0.5);
            (x, y)
        })
        .collect();
    let mut group = c.benchmark_group("bilinear_sampling");

    group.bench_function("sample_world_bilinear_strict", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &(x, y) in &samples {
                sum += raster
                    .sample_world(0, x, y, ResampleMethod::Bilinear, NodataPolicy::Strict)
                    .unwrap_or(0.0);
            }
            black_box(sum)
        })
    });

    group.bench_function("scalar_equivalent_bilinear", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &(x, y) in &samples {
                let col_f = (x - raster.x_min) / raster.cell_size_x - 0.5;
                let row_f = (raster.y_max() - y) / raster.cell_size_y - 0.5;
                let c0 = col_f.floor() as isize;
                let r0 = row_f.floor() as isize;
                let c1 = c0 + 1;
                let r1 = r0 + 1;
                let (Some(q00), Some(q10), Some(q01), Some(q11)) = (
                    raster.get_opt(0, r0, c0),
                    raster.get_opt(0, r0, c1),
                    raster.get_opt(0, r1, c0),
                    raster.get_opt(0, r1, c1),
                ) else {
                    continue;
                };
                let tx = col_f - c0 as f64;
                let ty = row_f - r0 as f64;
                let a = q00 * (1.0 - tx) + q10 * tx;
                let b2 = q01 * (1.0 - tx) + q11 * tx;
                sum += a * (1.0 - ty) + b2 * ty;
            }
            black_box(sum)
        })
    });

    group.finish();
}

fn lanczos_kernel(x: f64, a: f64) -> f64 {
    if x.abs() < 1e-12 {
        return 1.0;
    }
    if x.abs() >= a {
        return 0.0;
    }
    let pix = std::f64::consts::PI * x;
    let pix_over_a = pix / a;
    (pix.sin() / pix) * (pix_over_a.sin() / pix_over_a)
}

fn lanczos3_weights(sample_f: f64, floor_idx: isize) -> [f64; 6] {
    let mut w = [0.0_f64; 6];
    for (i, wi) in w.iter_mut().enumerate() {
        let idx = floor_idx + i as isize - 2;
        let dx = sample_f - idx as f64;
        *wi = lanczos_kernel(dx, 3.0);
    }
    w
}

fn bench_lanczos_sampling(c: &mut Criterion) {
    let raster = make_raster(1024, 1024);
    let samples: Vec<(f64, f64)> = (0..6_000)
        .map(|i| {
            let x = 5.25 + ((i % 300) as f64 * 0.75);
            let y = 5.75 + (((i / 300) % 20) as f64 * 0.75);
            (x, y)
        })
        .collect();
    let mut group = c.benchmark_group("lanczos_sampling");

    group.bench_function("sample_world_lanczos_strict", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &(x, y) in &samples {
                sum += raster
                    .sample_world(0, x, y, ResampleMethod::Lanczos, NodataPolicy::Strict)
                    .unwrap_or(0.0);
            }
            black_box(sum)
        })
    });

    group.bench_function("scalar_equivalent_lanczos", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for &(x, y) in &samples {
                let col_f = (x - raster.x_min) / raster.cell_size_x - 0.5;
                let row_f = (raster.y_max() - y) / raster.cell_size_y - 0.5;
                let c0 = col_f.floor() as isize;
                let r0 = row_f.floor() as isize;

                if c0 - 2 < 0 || r0 - 2 < 0 || c0 + 3 >= raster.cols as isize || r0 + 3 >= raster.rows as isize {
                    continue;
                }

                let wx = lanczos3_weights(col_f, c0);
                let wy = lanczos3_weights(row_f, r0);
                let mut local_sum = 0.0;
                let mut wsum = 0.0;
                let mut valid = true;
                for (j, wyj) in wy.iter().enumerate() {
                    let rr = r0 + j as isize - 2;
                    for (i, wxi) in wx.iter().enumerate() {
                        let cc = c0 + i as isize - 2;
                        let Some(v) = raster.get_opt(0, rr, cc) else {
                            valid = false;
                            break;
                        };
                        let w = *wxi * *wyj;
                        local_sum += v * w;
                        wsum += w;
                    }
                    if !valid {
                        break;
                    }
                }
                if valid && wsum.abs() > 1e-12 {
                    sum += local_sum / wsum;
                }
            }
            black_box(sum)
        })
    });

    group.finish();
}

/// Build a projected raster in EPSG 32632 (WGS84 UTM Zone 32N).
///
/// Covers a `cols × rows` area at 100 m/pixel centred near 9°E, 52°N.
fn make_utm_raster(cols: usize, rows: usize) -> Raster {
    let nodata = -9999.0_f64;
    let data: Vec<f64> = (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                if (row + col) % 20 == 0 {
                    nodata
                } else {
                    ((row * cols + col) % 1000) as f64
                }
            })
        })
        .collect();

    let cell = 100.0_f64; // 100 m/pixel
    let cfg = RasterConfig {
        cols,
        rows,
        bands: 1,
        x_min: 500_000.0,
        y_min: 5_750_000.0,
        cell_size: cell,
        cell_size_y: Some(cell),
        nodata,
        data_type: DataType::F64,
        crs: Default::default(),
        metadata: Vec::new(),
    };
    let mut r = Raster::from_data(cfg, data).expect("failed to build bench raster");
    r.crs = CrsInfo::from_epsg(32632);
    r
}

fn bench_reproject(c: &mut Criterion) {
    let mut group = c.benchmark_group("reproject_batch");

    // Benchmark sizes: 128×128 (warm-up scale) and 512×512 (real workload).
    for size in [128_usize, 512] {
        let raster = make_utm_raster(size, size);

        // Same-datum: WGS84 UTM32N → WGS84 UTM33N (only projection math, no Helmert).
        group.bench_with_input(
            BenchmarkId::new("utm32n_to_utm33n_same_datum", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(
                        raster
                            .reproject_with_options(
                                &ReprojectOptions::new(32633, ResampleMethod::Bilinear),
                            )
                            .expect("reproject failed"),
                    )
                });
            },
        );

        // Cross-datum: WGS84 UTM32N → ED50 UTM32N (ECEF Helmert leg is vectorized).
        group.bench_with_input(
            BenchmarkId::new("utm32n_wgs84_to_utm32n_ed50_cross_datum", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(
                        raster
                            .reproject_with_options(
                                &ReprojectOptions::new(23032, ResampleMethod::Bilinear),
                            )
                            .expect("reproject failed"),
                    )
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_full_statistics, bench_band_statistics, bench_bilinear_sampling, bench_lanczos_sampling, bench_reproject);
criterion_main!(benches);