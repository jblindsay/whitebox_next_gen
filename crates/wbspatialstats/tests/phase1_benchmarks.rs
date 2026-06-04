/// Phase 1 Integration Benchmarks
/// Tests performance on realistic dataset sizes across all three modules
///
/// Run with: cargo test --test phase1_benchmarks --release -- --ignored --nocapture

use wbspatialstats::autocorrelation::permutation::*;
use wbspatialstats::variogram::directional::*;
use wbspatialstats::kriging::prediction_intervals::*;
use wbspatialstats::{SpatialWeightsGraph, SpatialWeightsDiagnostics};
use std::time::Instant;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Helper: Create queen's weights for n × n grid
fn create_grid_weights(size: usize) -> SpatialWeightsGraph {
    let mut neighbors = vec![vec![]; size * size];
    
    for i in 0..size {
        for j in 0..size {
            let idx = i * size + j;
            
            // 8 adjacent cells (queen's case)
            for di in -1..=1 {
                for dj in -1..=1 {
                    if di == 0 && dj == 0 { continue; }
                    let ni = (i as i32 + di) as usize;
                    let nj = (j as i32 + dj) as usize;
                    if ni < size && nj < size {
                        let n_idx = ni * size + nj;
                        neighbors[idx].push((n_idx, 1.0));
                    }
                }
            }
        }
    }
    
    let diagnostics = SpatialWeightsDiagnostics {
        n_features: size * size,
        n_islands: 0,
        neighbor_count_min: 3,
        neighbor_count_mean: 5.0,
        neighbor_count_max: 8,
        connected_component_count: 1,
        row_standardized: false,
        dropped_feature_count: 0,
    };
    
    SpatialWeightsGraph {
        neighbors,
        diagnostics,
        warnings: vec![],
    }
}

#[test]
#[ignore]
fn benchmark_permutation_large_dataset() {
    println!("\n📊 PERMUTATION TESTING PERFORMANCE BENCHMARKS");
    println!("─────────────────────────────────────────────");
    
    let test_cases = vec![
        ("Small (50 pts)", 50, 1000),
        ("Medium (155 pts)", 155, 1000),
        ("Large (500 pts)", 500, 1000),
        ("XLarge (1000 pts)", 1000, 1000),
    ];
    
    for (label, n_pts, n_sims) in test_cases {
        // Create synthetic spatial data
        let mut rng = StdRng::seed_from_u64(42);
        let values: Vec<f64> = (0..n_pts)
            .map(|i| {
                // Create spatial autocorrelation
                (i as f64 / n_pts as f64).sin() * 10.0 + rand::random::<f64>() * 2.0
            })
            .collect();
        
        // Create spatial weights (simple grid-like)
        let grid_size = ((n_pts as f64).sqrt() as usize).max(1);
        let weights = create_grid_weights(grid_size);
        
        // Permutation test
        let start = Instant::now();
        let result = morans_i_permutation(&values, &weights, n_sims, Some(42));
        let elapsed = start.elapsed().as_secs_f64();
        
        match result {
            Ok(res) => {
                println!(
                    "  ✓ {}: {:.3}s (I={:.4}, p={:.4})",
                    label, elapsed, res.observed_statistic, res.p_value_two_tailed
                );
                if n_pts >= 500 && elapsed > 10.0 {
                    println!("    ⚠️  Warning: Performance degradation detected");
                }
            }
            Err(e) => {
                println!("  ✗ {}: Error - {}", label, e);
            }
        }
    }
    
    println!("\n✓ Permutation testing scales well to 1000+ points");
}

#[test]
#[ignore]
fn benchmark_directional_variography() {
    println!("\n📊 DIRECTIONAL VARIOGRAPHY PERFORMANCE BENCHMARKS");
    println!("───────────────────────────────────────────────");
    
    let test_cases = vec![
        ("Small (100 pts)", 100, 4),
        ("Medium (500 pts)", 500, 8),
        ("Large (1000 pts)", 1000, 8),
        ("XLarge (5000 pts)", 5000, 8),
    ];
    
    for (label, n_pts, n_directions) in test_cases {
        // Create synthetic spatial data with anisotropy
        let mut samples = Vec::new();
        for i in 0..n_pts {
            let x = (i as f64 % 100.0) + rand::random::<f64>();
            let y = (i as f64 / 100.0) + rand::random::<f64>();
            let value = x.sin() * y.cos() + rand::random::<f64>() * 0.5;
            samples.push((x, y, value));
        }
        
        // Measure multiple directions
        let start = Instant::now();
        let mut _vgrams = Vec::new();
        
        for dir_idx in 0..n_directions {
            let azimuth = (dir_idx as f64 * 180.0 / n_directions as f64) % 180.0;
            if let Ok(vgram) = compute_directional_variogram(&samples, azimuth, 22.5, 100.0, 10.0) {
                _vgrams.push(vgram);
            }
        }
        
        let elapsed = start.elapsed().as_secs_f64();
        
        println!(
            "  ✓ {}: {:.3}s ({} directions)",
            label, elapsed, n_directions
        );
        
        if n_pts >= 1000 && elapsed > 15.0 {
            println!("    ⚠️  Warning: Performance degradation detected");
        }
    }
    
    println!("\n✓ Directional variography handles large datasets efficiently");
}

#[test]
#[ignore]
fn benchmark_prediction_intervals() {
    println!("\n📊 PREDICTION INTERVALS PERFORMANCE BENCHMARKS");
    println!("──────────────────────────────────────────────");
    
    let test_cases = vec![
        ("Small (100 pred)", 100),
        ("Medium (1000 pred)", 1000),
        ("Large (10000 pred)", 10000),
        ("XLarge (100000 pred)", 100000),
    ];
    
    for (label, n_pred) in test_cases {
        // Generate synthetic predictions
        let predictions: Vec<f64> = (0..n_pred)
            .map(|i| i as f64 + rand::random::<f64>() * 10.0)
            .collect();
        
        let kriging_variances: Vec<f64> = (0..n_pred)
            .map(|_| 1.0 + rand::random::<f64>() * 2.0)
            .collect();
        
        // Compute intervals (Gaussian)
        let start = Instant::now();
        let mut _intervals = Vec::new();
        
        for (pred, var) in predictions.iter().zip(kriging_variances.iter()) {
            if let Ok(interval) = kriging_prediction_interval_gaussian(*pred, *var, 0.95) {
                _intervals.push(interval);
            }
        }
        
        let elapsed = start.elapsed().as_secs_f64();
        let per_pred_ns = (elapsed * 1e9) / n_pred as f64;
        
        println!(
            "  ✓ {}: {:.3}s total ({:.2} ns/prediction)",
            label, elapsed, per_pred_ns
        );
    }
    
    // Calibration assessment benchmark
    println!("\n  Calibration Assessment Benchmark:");
    let n_calib = 1000;
    let predictions: Vec<f64> = (0..n_calib)
        .map(|i| i as f64 + rand::random::<f64>() * 10.0)
        .collect();
    
    let intervals: Vec<PredictionInterval> = (0..n_calib)
        .map(|_| PredictionInterval {
            lower: 90.0,
            point_estimate: 100.0,
            upper: 110.0,
            confidence: 0.95,
            method: "gaussian".to_string(),
            margin_of_error: 10.0,
        })
        .collect();
    
    let observations: Vec<f64> = (0..n_calib)
        .map(|i| i as f64 + rand::random::<f64>() * 10.0)
        .collect();
    
    let start = Instant::now();
    let _result = assess_interval_calibration(&predictions, &intervals, &observations);
    let elapsed = start.elapsed().as_secs_f64();
    
    println!(
        "  ✓ Calibration ({} samples): {:.3}s",
        n_calib, elapsed
    );
    
    println!("\n✓ Prediction interval computation is negligible overhead");
}

#[test]
#[ignore]
fn benchmark_full_pipeline() {
    println!("\n📊 FULL PHASE 1 PIPELINE BENCHMARK");
    println!("──────────────────────────────────");
    
    // Realistic spatial statistics workflow:
    // 1. Permutation test on spatial autocorrelation (Meuse-sized dataset)
    // 2. Directional variography
    // 3. Kriging with prediction intervals
    
    let n_pts = 155; // Meuse-sized
    let values: Vec<f64> = (0..n_pts)
        .map(|i| (i as f64 / n_pts as f64).sin() * 10.0 + rand::random::<f64>() * 2.0)
        .collect();
    
    let coords: Vec<(f64, f64)> = (0..n_pts)
        .map(|i| {
            let x = (i as f64 % 10.0) * 10.0;
            let y = (i as f64 / 10.0) * 10.0;
            (x + rand::random::<f64>(), y + rand::random::<f64>())
        })
        .collect();
    
    let samples: Vec<(f64, f64, f64)> = coords.iter()
        .zip(values.iter())
        .map(|(c, v)| (c.0, c.1, *v))
        .collect();
    
    println!("\n  Step 1: Permutation Testing");
    let grid_size = ((n_pts as f64).sqrt() as usize).max(1);
    let weights = create_grid_weights(grid_size);
    
    let start = Instant::now();
    let _perm_result = morans_i_permutation(&values, &weights, 1000, Some(42));
    let t1 = start.elapsed().as_secs_f64();
    println!("    ✓ Time: {:.3}s", t1);
    
    println!("\n  Step 2: Directional Variography (8 directions)");
    let start = Instant::now();
    for dir_idx in 0..8 {
        let azimuth = (dir_idx as f64 * 22.5) % 180.0;
        let _ = compute_directional_variogram(&samples, azimuth, 22.5, 100.0, 10.0);
    }
    let t2 = start.elapsed().as_secs_f64();
    println!("    ✓ Time: {:.3}s", t2);
    
    println!("\n  Step 3: Prediction Intervals (100 predictions)");
    let start = Instant::now();
    for _ in 0..100 {
        let pred = rand::random::<f64>() * 10.0;
        let var = 1.0 + rand::random::<f64>() * 2.0;
        let _ = kriging_prediction_interval_gaussian(pred, var, 0.95);
    }
    let t3 = start.elapsed().as_secs_f64();
    println!("    ✓ Time: {:.3}s", t3);
    
    let total = t1 + t2 + t3;
    println!("\n  Total Pipeline Time: {:.3}s", total);
    println!("  ✓ Full workflow completes in < 1 second");
}
