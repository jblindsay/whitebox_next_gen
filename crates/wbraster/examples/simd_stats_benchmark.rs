//! Compact scalar-vs-SIMD statistics benchmark for wbraster.
//!
//! Run with: cargo run --release --example simd_stats_benchmark

use wbraster::{DataType, Raster, RasterConfig, StatisticsComputationMode};

fn main() {
    println!("=== wbraster Compact SIMD Benchmark ===\n");

    let nodata = -9999.0;
    let data: Vec<f64> = (0..1000)
        .flat_map(|row| {
            (0..1000).map(move |col| {
                if (row + col) % 10 == 0 {
                    nodata
                } else {
                    ((row * 1000 + col) as f64) / 1000.0
                }
            })
        })
        .collect();

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
    let raster = Raster::from_data(config, data).expect("Raster creation failed");

    println!("Benchmark: statistics_band_with_mode on a 1000x1000 raster");
    let start = std::time::Instant::now();
    let scalar = raster
        .statistics_band_with_mode(0, StatisticsComputationMode::Scalar)
        .expect("scalar statistics failed");
    let scalar_elapsed = start.elapsed();

    let start = std::time::Instant::now();
    let simd = raster
        .statistics_band_with_mode(0, StatisticsComputationMode::Simd)
        .expect("simd statistics failed");
    let simd_elapsed = start.elapsed();

    println!("  Scalar: {:.2}ms", scalar_elapsed.as_secs_f64() * 1000.0);
    println!("  SIMD:   {:.2}ms", simd_elapsed.as_secs_f64() * 1000.0);
    println!(
        "  Speedup: {:.2}x",
        scalar_elapsed.as_secs_f64() / simd_elapsed.as_secs_f64()
    );
    println!("  Scalar valid cells: {}", scalar.valid_count);
    println!("  SIMD valid cells:   {}", simd.valid_count);
    println!("  Scalar min/max: {:.2} / {:.2}", scalar.min, scalar.max);
    println!("  SIMD min/max:   {:.2} / {:.2}", simd.min, simd.max);
    println!(
        "  Match: {}",
        (scalar.min - simd.min).abs() < 1e-9
            && (scalar.max - simd.max).abs() < 1e-9
            && (scalar.mean - simd.mean).abs() < 1e-9
            && (scalar.std_dev - simd.std_dev).abs() < 1e-9
            && scalar.valid_count == simd.valid_count
            && scalar.nodata_count == simd.nodata_count
    );

    println!("\n=== Benchmark Complete ===");
}
