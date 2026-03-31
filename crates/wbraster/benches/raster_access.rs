use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wbraster::{DataType, Raster, RasterConfig};

fn make_raster(data_type: DataType, cols: usize, rows: usize) -> Raster {
    let cfg = RasterConfig {
        cols,
        rows,
        data_type,
        nodata: -9999.0,
        ..Default::default()
    };
    let data: Vec<f64> = (0..(cols * rows))
        .map(|i| ((i % 1000) as f64) * 0.25)
        .collect();
    Raster::from_data(cfg, data).expect("failed to build benchmark raster")
}

fn make_probe_indices(len: usize, n: usize) -> Vec<usize> {
    // Deterministic LCG so benchmarks are reproducible without RNG deps.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push((state as usize) % len);
    }
    out
}

fn bench_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("f32_access");
    for &(cols, rows) in &[(512usize, 512usize), (2048usize, 2048usize)] {
        let raster = make_raster(DataType::F32, cols, rows);

        group.bench_with_input(
            BenchmarkId::new("iter_f64", format!("{}x{}", cols, rows)),
            &raster,
            |b, r| {
                b.iter(|| {
                    let mut sum = 0.0_f64;
                    for v in r.data.iter_f64() {
                        sum += v;
                    }
                    black_box(sum)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("typed_f32_slice", format!("{}x{}", cols, rows)),
            &raster,
            |b, r| {
                b.iter(|| {
                    let mut sum = 0.0_f64;
                    let slice = r.data_f32().expect("expected f32 storage");
                    for &v in slice {
                        sum += v as f64;
                    }
                    black_box(sum)
                })
            },
        );
    }
    group.finish();
}

fn bench_u16(c: &mut Criterion) {
    let mut group = c.benchmark_group("u16_access");
    for &(cols, rows) in &[(512usize, 512usize), (2048usize, 2048usize)] {
        let raster = make_raster(DataType::U16, cols, rows);

        group.bench_with_input(
            BenchmarkId::new("iter_f64", format!("{}x{}", cols, rows)),
            &raster,
            |b, r| {
                b.iter(|| {
                    let mut sum = 0.0_f64;
                    for v in r.data.iter_f64() {
                        sum += v;
                    }
                    black_box(sum)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("typed_u16_slice", format!("{}x{}", cols, rows)),
            &raster,
            |b, r| {
                b.iter(|| {
                    let mut sum = 0.0_f64;
                    let slice = r.data_u16().expect("expected u16 storage");
                    for &v in slice {
                        sum += v as f64;
                    }
                    black_box(sum)
                })
            },
        );
    }
    group.finish();
}

fn bench_random_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_access");
    let cols = 2048usize;
    let rows = 2048usize;
    let probes = make_probe_indices(cols * rows, 200_000);

    let raster_f32 = make_raster(DataType::F32, cols, rows);
    group.bench_with_input(
        BenchmarkId::new("get_raw_f32", format!("{}x{}", cols, rows)),
        &raster_f32,
        |b, r| {
            b.iter(|| {
                let mut sum = 0.0_f64;
                for &idx in &probes {
                    let row = idx / cols;
                    let col = idx % cols;
                    sum += r.get_raw(0, col as isize, row as isize).unwrap_or(0.0);
                }
                black_box(sum)
            })
        },
    );
    group.bench_with_input(
        BenchmarkId::new("typed_f32_direct", format!("{}x{}", cols, rows)),
        &raster_f32,
        |b, r| {
            b.iter(|| {
                let mut sum = 0.0_f64;
                let s = r.data_f32().expect("expected f32 storage");
                for &idx in &probes {
                    sum += s[idx] as f64;
                }
                black_box(sum)
            })
        },
    );

    let raster_u16 = make_raster(DataType::U16, cols, rows);
    group.bench_with_input(
        BenchmarkId::new("get_raw_u16", format!("{}x{}", cols, rows)),
        &raster_u16,
        |b, r| {
            b.iter(|| {
                let mut sum = 0.0_f64;
                for &idx in &probes {
                    let row = idx / cols;
                    let col = idx % cols;
                    sum += r.get_raw(0, col as isize, row as isize).unwrap_or(0.0);
                }
                black_box(sum)
            })
        },
    );
    group.bench_with_input(
        BenchmarkId::new("typed_u16_direct", format!("{}x{}", cols, rows)),
        &raster_u16,
        |b, r| {
            b.iter(|| {
                let mut sum = 0.0_f64;
                let s = r.data_u16().expect("expected u16 storage");
                for &idx in &probes {
                    sum += s[idx] as f64;
                }
                black_box(sum)
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench_f32, bench_u16, bench_random_access);
criterion_main!(benches);
