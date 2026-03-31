use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wbprojection::datum::HelmertParams;
use wbprojection::{Crs, Datum, Ellipsoid, Projection, ProjectionKind, ProjectionParams};

fn make_helmert_batches(n: usize) -> Vec<([f64; 4], [f64; 4], [f64; 4])> {
    (0..n)
        .map(|i| {
            let base = i as f64 * 25.0;
            (
                [
                    3_987_654.25 + base,
                    4_112_345.5 + base,
                    3_854_210.0 + base,
                    4_034_567.25 + base,
                ],
                [
                    766_432.5 + base,
                    612_345.75 + base,
                    854_321.0 + base,
                    701_234.5 + base,
                ],
                [
                    4_966_789.0 + base,
                    4_844_321.25 + base,
                    5_102_468.5 + base,
                    4_923_456.75 + base,
                ],
            )
        })
        .collect()
}

fn make_coords(len: usize) -> Vec<(f64, f64)> {
    (0..len)
        .map(|i| (9.0 + (i as f64 * 0.01), 48.0 + (i as f64 * 0.01)))
        .collect()
}

fn make_geographic_coords(len: usize) -> Vec<(f64, f64)> {
    (0..len)
        .map(|i| {
            let lon = -5.0 + (i as f64 * 0.01);
            let lat = 40.0 + (i as f64 * 0.005);
            (lon, lat)
        })
        .collect()
}

fn make_geocentric_coords(len: usize) -> Vec<(f64, f64, f64)> {
    (0..len)
        .map(|i| {
            let base = i as f64 * 25.0;
            (3_987_654.25 + base, 766_432.5 + base, 4_966_789.0 + base)
        })
        .collect()
}

fn wgs84_geocentric() -> Crs {
    Crs {
        name: "WGS 84 geocentric benchmark CRS".to_string(),
        datum: Datum::WGS84,
        projection: Projection::new(
            ProjectionParams::new(ProjectionKind::Geocentric).with_ellipsoid(Ellipsoid::WGS84),
        )
        .expect("projection creation failed"),
    }
}

fn helmert_geocentric(datum: Datum, name: &str) -> Crs {
    let ellipsoid = datum.ellipsoid.clone();
    Crs {
        name: name.to_string(),
        datum,
        projection: Projection::new(
            ProjectionParams::new(ProjectionKind::Geocentric).with_ellipsoid(ellipsoid),
        )
        .expect("projection creation failed"),
    }
}

fn bench_helmert_kernels(c: &mut Criterion) {
    let params = HelmertParams {
        tx: -115.0,
        ty: 118.0,
        tz: 426.0,
        rx: 0.0,
        ry: 0.0,
        rz: 0.814,
        ds: -3.6,
    };
    let batches = make_helmert_batches(1024);
    let mut group = c.benchmark_group("helmert_kernel");

    group.bench_function("scalar_batch4_equivalent", |b| {
        b.iter(|| {
            let mut sink = 0.0;
            for (x4, y4, z4) in &batches {
                for lane in 0..4 {
                    let (xo, yo, zo) = params.apply(x4[lane], y4[lane], z4[lane]);
                    sink += xo + yo + zo;
                }
            }
            black_box(sink)
        })
    });

    group.bench_function("simd_batch4", |b| {
        b.iter(|| {
            let mut sink = 0.0;
            for (x4, y4, z4) in &batches {
                let (xo, yo, zo) = params.apply_simd_batch4(x4, y4, z4);
                sink += xo[0] + yo[1] + zo[2] + xo[3];
            }
            black_box(sink)
        })
    });

    group.finish();
}

fn bench_crs_batch_api(c: &mut Criterion) {
    let source = Crs::from_epsg(4326).expect("WGS84 load failed");
    let target = Crs::from_epsg(32632).expect("UTM32N load failed");
    let source_geoc = helmert_geocentric(Datum::ED50, "ED50 geocentric benchmark CRS");
    let target_geoc = wgs84_geocentric();
    let source_geo = Crs::from_epsg(4230).expect("ED50 geographic load failed");
    let target_geo = Crs::from_epsg(4326).expect("WGS84 geographic load failed");
    let mut group = c.benchmark_group("crs_batch_api");

    for &len in &[100usize, 1000usize] {
        let original = make_coords(len);
        group.bench_with_input(BenchmarkId::new("transform_to_batch", len), &len, |b, _| {
            b.iter(|| {
                let mut coords = original.clone();
                black_box(source.transform_to_batch(&mut coords, &target));
                black_box(coords)
            })
        });
    }

    for &len in &[100usize, 1000usize] {
        let original = make_geocentric_coords(len);
        group.bench_with_input(BenchmarkId::new("transform_to_3d_scalar_loop_geocentric", len), &len, |b, _| {
            b.iter(|| {
                let transformed: Vec<(f64, f64, f64)> = original
                    .iter()
                    .map(|&(x, y, z)| source_geoc.transform_to_3d(x, y, z, &target_geoc).unwrap())
                    .collect();
                black_box(transformed)
            })
        });
        group.bench_with_input(BenchmarkId::new("transform_to_3d_batch_geocentric", len), &len, |b, _| {
            b.iter(|| {
                let mut coords = original.clone();
                black_box(source_geoc.transform_to_3d_batch(&mut coords, &target_geoc));
                black_box(coords)
            })
        });
    }

    for &len in &[100usize, 1000usize] {
        let original = make_geographic_coords(len);
        group.bench_with_input(BenchmarkId::new("transform_to_scalar_loop_geographic", len), &len, |b, _| {
            b.iter(|| {
                let transformed: Vec<(f64, f64)> = original
                    .iter()
                    .map(|&(x, y)| source_geo.transform_to(x, y, &target_geo).unwrap())
                    .collect();
                black_box(transformed)
            })
        });
        group.bench_with_input(BenchmarkId::new("transform_to_batch_geographic", len), &len, |b, _| {
            b.iter(|| {
                let mut coords = original.clone();
                black_box(source_geo.transform_to_batch(&mut coords, &target_geo));
                black_box(coords)
            })
        });
    }

    // Projected CRS batch fast path: ED50 UTM 32N → WGS84 UTM 32N (cross-datum Helmert)
    let source_proj = Crs::from_epsg(23032).expect("ED50 UTM 32N load failed");
    let target_proj = Crs::from_epsg(32632).expect("WGS84 UTM 32N load failed");
    let make_projected_coords = |n: usize| -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| (490_000.0 + i as f64 * 10.0, 5_490_000.0 + i as f64 * 5.0))
            .collect()
    };

    for &len in &[100usize, 1000usize] {
        let original = make_projected_coords(len);
        group.bench_with_input(BenchmarkId::new("transform_to_scalar_loop_projected", len), &len, |b, _| {
            b.iter(|| {
                let transformed: Vec<(f64, f64)> = original
                    .iter()
                    .map(|&(x, y)| source_proj.transform_to(x, y, &target_proj).unwrap())
                    .collect();
                black_box(transformed)
            })
        });
        group.bench_with_input(BenchmarkId::new("transform_to_batch_projected", len), &len, |b, _| {
            b.iter(|| {
                let mut coords = original.clone();
                black_box(source_proj.transform_to_batch(&mut coords, &target_proj));
                black_box(coords)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_helmert_kernels, bench_crs_batch_api);
criterion_main!(benches);
