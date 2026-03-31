//! SIMD optimization benchmarks for wbraster.
//!
//! Measures performance of statistics computation with SIMD optimizations.
//! Run with: cargo run --release --example simd_stats_compute

use wbraster::{DataType, Raster, RasterConfig, Statistics, StatisticsComputationMode};

fn benchmark_statistics_mode(
    raster: &Raster,
    mode: StatisticsComputationMode,
    iterations: usize,
) -> (Statistics, std::time::Duration) {
    let start = std::time::Instant::now();
    let mut last = raster.statistics_with_mode(mode);
    for _ in 1..iterations {
        last = raster.statistics_with_mode(mode);
    }
    (last, start.elapsed())
}

fn benchmark_band_statistics_mode(
    raster: &Raster,
    mode: StatisticsComputationMode,
    iterations: usize,
) -> (Statistics, std::time::Duration) {
    let start = std::time::Instant::now();
    let mut last = raster
        .statistics_band_with_mode(0, mode)
        .expect("band statistics failed");
    for _ in 1..iterations {
        last = raster
            .statistics_band_with_mode(0, mode)
            .expect("band statistics failed");
    }
    (last, start.elapsed())
}

fn stats_match(lhs: Statistics, rhs: Statistics) -> bool {
    (lhs.min - rhs.min).abs() < 1e-9
        && (lhs.max - rhs.max).abs() < 1e-9
        && (lhs.mean - rhs.mean).abs() < 1e-9
        && (lhs.std_dev - rhs.std_dev).abs() < 1e-9
        && lhs.valid_count == rhs.valid_count
        && lhs.nodata_count == rhs.nodata_count
}

fn main() {
    println!("=== wbraster SIMD Benchmark ===\n");

    // Create test data: 1000×1000 raster with varying values and some nodata
    let mut data = Vec::with_capacity(1000 * 1000);
    let nodata = -32768.0;
    
    for i in 0..1000 {
        for j in 0..1000 {
            let value = (i as f64 * 10.0 + j as f64) % 500.0;
            // ~10% nodata
            if (i + j) % 10 == 0 {
                data.push(nodata);
            } else {
                data.push(value);
            }
        }
    }

    let config = RasterConfig {
        cols: 1000,
        rows: 1000,
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

    let raster = Raster::from_data(config, data).expect("Failed to create raster");

    // Benchmark 1: full-raster scalar vs SIMD statistics
    println!("Benchmark 1: Full raster statistics (1000×1000 with ~10% nodata)");
    let iterations = 100;
    let (scalar_stats, elapsed_scalar) = benchmark_statistics_mode(
        &raster,
        StatisticsComputationMode::Scalar,
        iterations,
    );
    let (simd_stats, elapsed_simd) = benchmark_statistics_mode(
        &raster,
        StatisticsComputationMode::Simd,
        iterations,
    );
    println!("  Scalar: {:.2}ms", elapsed_scalar.as_secs_f64() * 1000.0);
    println!("  SIMD:   {:.2}ms", elapsed_simd.as_secs_f64() * 1000.0);
    println!(
        "  Speedup: {:.2}x",
        elapsed_scalar.as_secs_f64() / elapsed_simd.as_secs_f64()
    );
    println!(
        "  SIMD throughput: {:.2}M pixels/sec",
        (1000.0 * 1000.0 * iterations as f64) / (elapsed_simd.as_secs_f64() * 1_000_000.0)
    );

    // Benchmark 2: band statistics scalar vs SIMD
    println!("\nBenchmark 2: Band statistics computation");
    let (scalar_band_stats, elapsed_band_scalar) = benchmark_band_statistics_mode(
        &raster,
        StatisticsComputationMode::Scalar,
        iterations,
    );
    let (simd_band_stats, elapsed_band_simd) = benchmark_band_statistics_mode(
        &raster,
        StatisticsComputationMode::Simd,
        iterations,
    );
    println!("  Scalar: {:.2}ms", elapsed_band_scalar.as_secs_f64() * 1000.0);
    println!("  SIMD:   {:.2}ms", elapsed_band_simd.as_secs_f64() * 1000.0);
    println!(
        "  Speedup: {:.2}x",
        elapsed_band_scalar.as_secs_f64() / elapsed_band_simd.as_secs_f64()
    );
    println!(
        "  SIMD throughput: {:.2}M pixels/sec",
        (1000.0 * 1000.0 * iterations as f64) / (elapsed_band_simd.as_secs_f64() * 1_000_000.0)
    );

    // Correctness check
    println!("\n=== Correctness Validation ===");
    let stats = raster.statistics();
    println!("Raster statistics (1000×1000 with nodata masking):");
    println!("  Min: {:.2}", stats.min);
    println!("  Max: {:.2}", stats.max);
    println!("  Mean: {:.2}", stats.mean);
    println!("  StdDev: {:.2}", stats.std_dev);
    println!("  Valid cells: {}", stats.valid_count);
    println!("  Nodata cells: {}", stats.nodata_count);
    println!(
        "  Full-raster scalar/SIMD match: {}",
        stats_match(scalar_stats, simd_stats)
    );
    println!(
        "  Band scalar/SIMD match: {}",
        stats_match(scalar_band_stats, simd_band_stats)
    );

    println!("\n=== Benchmark Complete ===");
}
