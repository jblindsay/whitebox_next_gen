//! SIMD optimization benchmarks for wbprojection.
//!
//! Measures performance of the public Helmert SIMD kernel and compares it to the
//! current batch CRS wrapper.
//! Run with: cargo run --release --example simd_batch_transform

use wbprojection::datum::HelmertParams;
use wbprojection::{Crs, Datum, Ellipsoid, Projection, ProjectionKind, ProjectionParams};

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

fn benchmark_helmert_scalar(params: &HelmertParams, iterations: usize) -> std::time::Duration {
    let samples = [
        (3_987_654.25, 766_432.5, 4_966_789.0),
        (4_112_345.5, 612_345.75, 4_844_321.25),
        (3_854_210.0, 854_321.0, 5_102_468.5),
        (4_034_567.25, 701_234.5, 4_923_456.75),
    ];
    let start = std::time::Instant::now();
    let mut sink = 0.0;
    for _ in 0..iterations {
        for (x, y, z) in samples {
            let (xo, yo, zo) = params.apply(x, y, z);
            sink += xo + yo + zo;
        }
    }
    std::hint::black_box(sink);
    start.elapsed()
}

fn benchmark_helmert_simd(params: &HelmertParams, iterations: usize) -> std::time::Duration {
    let x4 = [3_987_654.25, 4_112_345.5, 3_854_210.0, 4_034_567.25];
    let y4 = [766_432.5, 612_345.75, 854_321.0, 701_234.5];
    let z4 = [4_966_789.0, 4_844_321.25, 5_102_468.5, 4_923_456.75];
    let start = std::time::Instant::now();
    let mut sink = 0.0;
    for _ in 0..iterations {
        let (xo, yo, zo) = params.apply_simd_batch4(&x4, &y4, &z4);
        sink += xo[0] + yo[1] + zo[2] + xo[3];
    }
    std::hint::black_box(sink);
    start.elapsed()
}

fn main() {
    println!("=== wbprojection SIMD Benchmark ===\n");

    let helmert = HelmertParams {
        tx: -115.0,
        ty: 118.0,
        tz: 426.0,
        rx: 0.0,
        ry: 0.0,
        rz: 0.814,
        ds: -3.6,
    };

    println!("Benchmark 1: Helmert kernel (4 ECEF points per batch)");
    let iterations = 1_000_000;
    let scalar_elapsed = benchmark_helmert_scalar(&helmert, iterations);
    let simd_elapsed = benchmark_helmert_simd(&helmert, iterations);
    println!("  Scalar: {:.2}ms", scalar_elapsed.as_secs_f64() * 1000.0);
    println!("  SIMD:   {:.2}ms", simd_elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Speedup: {:.2}x",
        scalar_elapsed.as_secs_f64() / simd_elapsed.as_secs_f64()
    );

    // Setup: Create a source and target CRS
    let source = Crs::from_epsg(4326).expect("WGS84 load failed");
    let target = Crs::from_epsg(32632).expect("UTM32N load failed");

    // Benchmark 2: Current CRS batch wrapper timing
    println!("\nBenchmark 2: Current CRS batch API (100 coordinates)");
    let mut coords_100: Vec<(f64, f64)> = (0..100)
        .map(|i| (9.0 + (i as f64 * 0.01), 48.0 + (i as f64 * 0.01)))
        .collect();
    let original_100 = coords_100.clone();

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        coords_100 = original_100.clone();
        let _errors = source.transform_to_batch(&mut coords_100, &target);
    }
    let elapsed_100 = start.elapsed();
    println!("  100 coords × 1000 iterations: {:.2}ms", elapsed_100.as_secs_f64() * 1000.0);
    println!("  Per-coordinate: {:.2}μs", (elapsed_100.as_micros() as f64) / (100.0 * 1000.0));

    // Benchmark 3: Current CRS batch wrapper timing (larger batch)
    println!("\nBenchmark 3: Current CRS batch API (1000 coordinates)");
    let mut coords_1000: Vec<(f64, f64)> = (0..1000)
        .map(|i| (9.0 + (i as f64 * 0.01), 48.0 + (i as f64 * 0.01)))
        .collect();
    let original_1000 = coords_1000.clone();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        coords_1000 = original_1000.clone();
        let _errors = source.transform_to_batch(&mut coords_1000, &target);
    }
    let elapsed_1000 = start.elapsed();
    println!("  1000 coords × 100 iterations: {:.2}ms", elapsed_1000.as_secs_f64() * 1000.0);
    println!("  Per-coordinate: {:.2}μs", (elapsed_1000.as_micros() as f64) / (1000.0 * 100.0));

    // Benchmark 4: Current CRS 3D batch wrapper timing
    println!("\nBenchmark 4: Current CRS 3D batch API (100 coordinates)");
    let mut coords3d_100: Vec<(f64, f64, f64)> = (0..100)
        .map(|i| (9.0 + (i as f64 * 0.01), 48.0 + (i as f64 * 0.01), 1000.0))
        .collect();
    let original_3d_100 = coords3d_100.clone();

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        coords3d_100 = original_3d_100.clone();
        let _errors = source.transform_to_3d_batch(&mut coords3d_100, &target);
    }
    let elapsed_3d_100 = start.elapsed();
    println!("  100 coords × 1000 iterations: {:.2}ms", elapsed_3d_100.as_secs_f64() * 1000.0);
    println!("  Per-coordinate: {:.2}μs", (elapsed_3d_100.as_micros() as f64) / (100.0 * 1000.0));

    println!("\nBenchmark 5: Geocentric CRS 3D batch API (SIMD fast path)");
    let source_geoc = helmert_geocentric(Datum::ED50, "ED50 geocentric benchmark CRS");
    let target_geoc = wgs84_geocentric();
    let mut geoc_coords: Vec<(f64, f64, f64)> = (0..1000)
        .map(|i| {
            let base = i as f64 * 25.0;
            (3_987_654.25 + base, 766_432.5 + base, 4_966_789.0 + base)
        })
        .collect();
    let original_geoc = geoc_coords.clone();

    let start = std::time::Instant::now();
    for _ in 0..100 {
        geoc_coords = original_geoc.clone();
        let _errors = source_geoc.transform_to_3d_batch(&mut geoc_coords, &target_geoc);
    }
    let elapsed_geoc = start.elapsed();
    println!("  1000 coords × 100 iterations: {:.2}ms", elapsed_geoc.as_secs_f64() * 1000.0);
    println!("  Per-coordinate: {:.2}μs", (elapsed_geoc.as_micros() as f64) / (1000.0 * 100.0));

    // Correctness check: verify Helmert scalar and SIMD results align.
    println!("\n=== Correctness Validation ===");
    let scalar_points = [
        helmert.apply(3_987_654.25, 766_432.5, 4_966_789.0),
        helmert.apply(4_112_345.5, 612_345.75, 4_844_321.25),
        helmert.apply(3_854_210.0, 854_321.0, 5_102_468.5),
        helmert.apply(4_034_567.25, 701_234.5, 4_923_456.75),
    ];
    let (simd_x, simd_y, simd_z) = helmert.apply_simd_batch4(
        &[3_987_654.25, 4_112_345.5, 3_854_210.0, 4_034_567.25],
        &[766_432.5, 612_345.75, 854_321.0, 701_234.5],
        &[4_966_789.0, 4_844_321.25, 5_102_468.5, 4_923_456.75],
    );
    for (idx, ((sx, sy, sz), (&vx, &vy, &vz))) in scalar_points
        .iter()
        .zip(simd_x.iter().zip(simd_y.iter()).zip(simd_z.iter()).map(|((x, y), z)| (x, y, z)))
        .enumerate()
    {
        println!(
            "  Kernel sample {} match: {}",
            idx,
            (sx - vx).abs() < 1e-9 && (sy - vy).abs() < 1e-9 && (sz - vz).abs() < 1e-9
        );
    }

    let mut test_coords = vec![(0.0, 0.0), (10.0, 50.0), (-5.0, 60.0)];
    let original_test = test_coords.clone();
    let _errors = source.transform_to_batch(&mut test_coords, &target);
    
    println!("Sample transformation results:");
    for (i, ((orig_x, orig_y), (new_x, new_y))) in original_test.iter().zip(test_coords.iter()).enumerate() {
        println!("  Point {}: ({:.6}, {:.6}) → ({:.1}, {:.1})",
                 i, orig_x, orig_y, new_x, new_y);
    }

    let mut geoc_test_batch = vec![
        (3_987_654.25, 766_432.5, 4_966_789.0),
        (4_112_345.5, 612_345.75, 4_844_321.25),
        (3_854_210.0, 854_321.0, 5_102_468.5),
        (4_034_567.25, 701_234.5, 4_923_456.75),
    ];
    let geoc_expected: Vec<(f64, f64, f64)> = geoc_test_batch
        .iter()
        .map(|&(x, y, z)| source_geoc.transform_to_3d(x, y, z, &target_geoc).unwrap())
        .collect();
    let _ = source_geoc.transform_to_3d_batch(&mut geoc_test_batch, &target_geoc);
    println!("Geocentric batch path matches scalar: {}",
        geoc_expected
            .iter()
            .zip(geoc_test_batch.iter())
            .all(|(lhs, rhs)| {
                (lhs.0 - rhs.0).abs() < 1e-3
                    && (lhs.1 - rhs.1).abs() < 1e-3
                    && (lhs.2 - rhs.2).abs() < 1e-3
            })
    );

    println!("\n=== Benchmark Complete ===");
}
