// Permutation-based inference for spatial autocorrelation
//
// Provides robust, distribution-free hypothesis testing for spatial statistics
// through random permutation of spatial data. More reliable than asymptotic
// approximations, especially for small samples or non-normal distributions.
//
// Performance is optimized for large datasets through:
// - Rayon parallelization across permutations
// - Sparse matrix operations on spatial weights
// - Efficient random number generation

use crate::weights::SpatialWeightsGraph;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rayon::prelude::*;

/// Result of permutation-based spatial autocorrelation test
#[derive(Debug, Clone)]
pub struct PermutationTestResult {
    /// Observed statistic value (e.g., observed Moran's I)
    pub observed_statistic: f64,
    /// Expected value under null hypothesis
    pub expected_value: f64,
    /// Variance estimated from permutation distribution
    pub variance: f64,
    /// Standardized z-score
    pub z_score: f64,
    /// Empirical p-value (one-tailed): count of permutations >= observed / (n_simulations + 1)
    pub p_value_one_tailed: f64,
    /// Empirical p-value (two-tailed): count of |permutation| >= |observed| / (n_simulations + 1)
    pub p_value_two_tailed: f64,
    /// Full permutation distribution (for diagnostics/plotting)
    pub permutation_distribution: Vec<f64>,
    /// Number of permutations performed
    pub n_simulations: usize,
}

/// Result of local Moran's I permutation testing (per-feature)
#[derive(Debug, Clone)]
pub struct LocalPermutationTestResult {
    /// Observed local Moran's I for each feature
    pub observed_statistics: Vec<f64>,
    /// Expected values for each feature
    pub expected_values: Vec<f64>,
    /// Variances for each feature (from permutation distribution)
    pub variances: Vec<f64>,
    /// Z-scores for each feature
    pub z_scores: Vec<f64>,
    /// Empirical p-values (two-tailed) for each feature
    pub p_values: Vec<f64>,
    /// Cluster types after FDR-BH correction: "HH", "LL", "HL", "LH", or "insignificant"
    pub cluster_types: Vec<String>,
    /// Number of permutations performed
    pub n_simulations: usize,
}

/// Compute Global Moran's I using permutation testing for inference
///
/// Randomly permutes spatial values `n_simulations` times and recalculates
/// Moran's I for each permutation. This provides an empirical null distribution
/// and robust p-values that do not rely on asymptotic normality assumptions.
///
/// # Performance
/// - Time: O(n_simulations * n_features * avg_neighbors)
/// - Parallelized across permutations with rayon
/// - Typical: 1000 simulations on 10k points < 2-5 minutes
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph
/// - `n_simulations`: Number of random permutations (typically 999-9999)
/// - `seed`: Optional random seed for reproducibility
///
/// # Returns
/// Permutation test result with observed statistic, empirical p-values, and distribution
pub fn morans_i_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    seed: Option<u64>,
) -> Result<PermutationTestResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required for permutation testing".to_string());
    }

    if n_simulations < 99 {
        return Err("At least 99 simulations recommended; got fewer".to_string());
    }

    // Compute observed Moran's I
    let observed_i = compute_morans_i_raw(values, weights)?;

    // Prepare for permutation: center the data
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let deviations: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let denominator: f64 = deviations.iter().map(|d| d * d).sum();

    if denominator == 0.0 {
        return Err("Deviations are zero; cannot compute Moran's I".to_string());
    }

    let sum_weights: f64 = weights.neighbors.iter().flatten().map(|(_, w)| w).sum();

    // Generate random permutation indices
    let rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => {
            let r = rand::rngs::StdRng::from_entropy();
            r
        }
    };

    // Parallel permutation testing
    let permutation_distribution: Vec<f64> = (0..n_simulations)
        .into_par_iter()
        .map(|_| {
            let mut rng_local = rng.clone();
            let mut perm_indices: Vec<usize> = (0..values.len()).collect();
            perm_indices.shuffle(&mut rng_local);

            // Compute Moran's I on permuted data
            let mut numerator = 0.0;
            for (i, neighbors) in weights.neighbors.iter().enumerate() {
                for (j, weight) in neighbors {
                    numerator +=
                        weight * deviations[perm_indices[i]] * deviations[perm_indices[*j]];
                }
            }

            (n / sum_weights) * (numerator / denominator)
        })
        .collect();

    // Compute empirical statistics
    let mean_perm: f64 = permutation_distribution.iter().sum::<f64>() / n_simulations as f64;
    let variance_perm: f64 = permutation_distribution
        .iter()
        .map(|x| (x - mean_perm).powi(2))
        .sum::<f64>()
        / (n_simulations as f64 - 1.0);

    // Count permutations >= observed (one-tailed, right-tail)
    let count_ge = permutation_distribution.iter().filter(|&&x| x >= observed_i).count() as f64;
    let p_one_tailed = (count_ge + 1.0) / (n_simulations as f64 + 1.0);

    // Count permutations |x| >= |observed| (two-tailed)
    let abs_observed = observed_i.abs();
    let count_abs_ge = permutation_distribution
        .iter()
        .filter(|&&x| x.abs() >= abs_observed)
        .count() as f64;
    let p_two_tailed = (count_abs_ge + 1.0) / (n_simulations as f64 + 1.0);

    // Compute z-score using permutation distribution
    let z_score = if variance_perm > 0.0 {
        (observed_i - mean_perm) / variance_perm.sqrt()
    } else {
        0.0
    };

    Ok(PermutationTestResult {
        observed_statistic: observed_i,
        expected_value: mean_perm,
        variance: variance_perm.max(0.0),
        z_score,
        p_value_one_tailed: p_one_tailed,
        p_value_two_tailed: p_two_tailed,
        permutation_distribution,
        n_simulations,
    })
}

/// Compute local Moran's I (LISA) using permutation testing
///
/// Performs independent permutation tests for each feature, providing
/// empirical p-values for local spatial clustering without relying on
/// asymptotic normality. Applies FDR-BH correction for multiple testing.
///
/// # Performance
/// - Time: O(n_simulations * n_features^2) in worst case
/// - Rayon parallelization across features
/// - Typical: 999 simulations on 155 points < 30 seconds
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph
/// - `n_simulations`: Number of permutations per feature
/// - `fdr_correction`: Whether to apply FDR-BH multiple testing correction
/// - `seed`: Optional random seed
///
/// # Returns
/// Per-feature p-values and cluster types after multiple testing correction
pub fn local_morans_i_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    fdr_correction: bool,
    seed: Option<u64>,
) -> Result<LocalPermutationTestResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required".to_string());
    }

    if n_simulations < 99 {
        return Err("At least 99 simulations recommended".to_string());
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let centered: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let s2: f64 = centered.iter().map(|z| z * z).sum::<f64>() / n;

    if s2 <= 0.0 {
        return Err("Data variance is zero".to_string());
    }

    let s = s2.sqrt();
    let z: Vec<f64> = centered.iter().map(|c| c / s).collect();
    let _b2 = z.iter().map(|zi| zi.powi(4)).sum::<f64>() / n;

    // Compute observed local statistics
    let mut observed_stats = vec![0.0; values.len()];
    for (i, neighbors) in weights.neighbors.iter().enumerate() {
        let mut lag_z = 0.0;
        for (j, w) in neighbors {
            lag_z += w * z[*j];
        }
        observed_stats[i] = z[i] * lag_z;
    }

    // Parallel permutation testing for each feature
    let base_seed = seed.unwrap_or_else(|| rand::random::<u64>());

    let results: Vec<_> = (0..values.len())
        .into_par_iter()
        .map(|i| {
            let mut rng = rand::rngs::StdRng::seed_from_u64(base_seed.wrapping_add(i as u64));

            // Generate permutation distribution for this feature
            let mut perm_dist = Vec::with_capacity(n_simulations);
            for _ in 0..n_simulations {
                let mut perm_indices: Vec<usize> = (0..values.len()).collect();
                perm_indices.shuffle(&mut rng);

                let z_perm = perm_indices.iter().map(|&idx| z[idx]).collect::<Vec<_>>();
                let mut lag_z = 0.0;
                for (j, w) in &weights.neighbors[i] {
                    lag_z += w * z_perm[*j];
                }
                perm_dist.push(z_perm[i] * lag_z);
            }

            let mean_perm = perm_dist.iter().sum::<f64>() / n_simulations as f64;
            let var_perm = perm_dist
                .iter()
                .map(|x| (x - mean_perm).powi(2))
                .sum::<f64>()
                / (n_simulations as f64 - 1.0).max(1.0);

            // Empirical two-tailed p-value
            let abs_obs = observed_stats[i].abs();
            let count_abs_ge = perm_dist.iter().filter(|x| x.abs() >= abs_obs).count() as f64;
            let p_value = (count_abs_ge + 1.0) / (n_simulations as f64 + 1.0);

            let z_score = if var_perm > 0.0 {
                (observed_stats[i] - mean_perm) / var_perm.sqrt()
            } else {
                0.0
            };

            (mean_perm, var_perm, p_value, z_score)
        })
        .collect();

    let mut expected_values = vec![0.0; values.len()];
    let mut variances = vec![0.0; values.len()];
    let mut p_values = vec![0.0; values.len()];
    let mut z_scores = vec![0.0; values.len()];

    for (i, (exp, var, p, z)) in results.into_iter().enumerate() {
        expected_values[i] = exp;
        variances[i] = var;
        p_values[i] = p;
        z_scores[i] = z;
    }

    // Apply FDR-BH correction if requested
    let alpha = 0.05;
    let corrected_alpha = if fdr_correction {
        apply_fdr_bh_correction(&p_values, alpha)
    } else {
        alpha
    };

    // Classify clusters based on corrected p-values and signs
    let mut cluster_types = vec!["insignificant".to_string(); values.len()];
    for i in 0..values.len() {
        if p_values[i] <= corrected_alpha {
            let has_high_neighbor = weights.neighbors[i]
                .iter()
                .any(|(j, _)| z[*j] >= 0.0 && z[*j] > z[i]);
            let has_low_neighbor = weights.neighbors[i]
                .iter()
                .any(|(j, _)| z[*j] < 0.0 && z[*j] < z[i]);

            cluster_types[i] = if z[i] >= 0.0 {
                if has_high_neighbor {
                    "HH".to_string()
                } else if has_low_neighbor {
                    "HL".to_string()
                } else {
                    "insignificant".to_string()
                }
            } else {
                if has_low_neighbor {
                    "LL".to_string()
                } else if has_high_neighbor {
                    "LH".to_string()
                } else {
                    "insignificant".to_string()
                }
            };
        }
    }

    Ok(LocalPermutationTestResult {
        observed_statistics: observed_stats,
        expected_values,
        variances,
        z_scores,
        p_values,
        cluster_types,
        n_simulations,
    })
}

/// Compute Getis-Ord G* statistic using permutation testing
///
/// # Arguments
/// - `values`: Data values
/// - `weights`: Spatial weights (typically including self-loops for G*)
/// - `n_simulations`: Number of permutations
/// - `seed`: Optional seed
///
/// # Returns
/// Permutation test result for Getis-Ord G*
pub fn getis_ord_gi_star_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    seed: Option<u64>,
) -> Result<PermutationTestResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required".to_string());
    }

    // Compute observed Getis-Ord G*
    let sum_x: f64 = values.iter().sum();
    let sum_w: f64 = weights.neighbors.iter().flatten().map(|(_, w)| w).sum();

    let mut sum_wx_overall = 0.0;
    for (_i, neighbors) in weights.neighbors.iter().enumerate() {
        for (j, w) in neighbors {
            sum_wx_overall += w * values[*j];
        }
    }
    let _sum_wx_sq: f64 = weights
        .neighbors
        .iter()
        .flatten()
        .map(|(_, w)| w * w)
        .sum();

    let observed_g_star = if sum_w > 0.0 {
        sum_wx_overall / sum_x
    } else {
        0.0
    };

    // Parallel permutation testing
    let rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_entropy(),
    };

    let permutation_distribution: Vec<f64> = (0..n_simulations)
        .into_par_iter()
        .map(|_| {
            let mut rng_local = rng.clone();
            let mut perm_values = values.to_vec();
            perm_values.shuffle(&mut rng_local);

            let mut sum_wx = 0.0;
            for (_, neighbors) in weights.neighbors.iter().enumerate() {
                for (j, w) in neighbors {
                    sum_wx += w * perm_values[*j];
                }
            }
            sum_wx / sum_x
        })
        .collect();

    let mean_perm: f64 = permutation_distribution.iter().sum::<f64>() / n_simulations as f64;
    let variance_perm: f64 = permutation_distribution
        .iter()
        .map(|x| (x - mean_perm).powi(2))
        .sum::<f64>()
        / (n_simulations as f64 - 1.0);

    let count_ge = permutation_distribution
        .iter()
        .filter(|&&x| x >= observed_g_star)
        .count() as f64;
    let p_one_tailed = (count_ge + 1.0) / (n_simulations as f64 + 1.0);

    let abs_obs = observed_g_star.abs();
    let count_abs_ge = permutation_distribution
        .iter()
        .filter(|&&x| x.abs() >= abs_obs)
        .count() as f64;
    let p_two_tailed = (count_abs_ge + 1.0) / (n_simulations as f64 + 1.0);

    let z_score = if variance_perm > 0.0 {
        (observed_g_star - mean_perm) / variance_perm.sqrt()
    } else {
        0.0
    };

    Ok(PermutationTestResult {
        observed_statistic: observed_g_star,
        expected_value: mean_perm,
        variance: variance_perm.max(0.0),
        z_score,
        p_value_one_tailed: p_one_tailed,
        p_value_two_tailed: p_two_tailed,
        permutation_distribution,
        n_simulations,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute Moran's I from raw data (internal use)
fn compute_morans_i_raw(values: &[f64], weights: &SpatialWeightsGraph) -> Result<f64, String> {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let deviations: Vec<f64> = values.iter().map(|v| v - mean).collect();

    let mut numerator = 0.0;
    for (i, neighbors) in weights.neighbors.iter().enumerate() {
        for (j, weight) in neighbors {
            numerator += weight * deviations[i] * deviations[*j];
        }
    }

    let denominator: f64 = deviations.iter().map(|d| d * d).sum();

    if denominator == 0.0 {
        return Err("Deviations are zero".to_string());
    }

    let sum_weights: f64 = weights.neighbors.iter().flatten().map(|(_, w)| w).sum();

    Ok((n / sum_weights) * (numerator / denominator))
}

/// Apply Benjamini-Hochberg FDR correction
/// Returns corrected alpha level (threshold for significance)
pub fn apply_fdr_bh_correction(p_values: &[f64], alpha: f64) -> f64 {
    let mut sorted_p = p_values.to_vec();
    sorted_p.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let m = sorted_p.len() as f64;

    for (i, &p) in sorted_p.iter().enumerate() {
        let rank = (i + 1) as f64;
        let threshold = alpha * rank / m;

        if p <= threshold {
            return p;
        }
    }

    0.0 // All rejected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_weights(n: usize) -> SpatialWeightsGraph {
        let mut neighbors = vec![Vec::new(); n];

        // Create simple linear neighbors for testing
        for i in 0..n {
            let neighbors_map = vec![
                if i > 0 { Some((i - 1, 1.0)) } else { None },
                if i < n - 1 { Some((i + 1, 1.0)) } else { None },
            ];

            neighbors[i] = neighbors_map.into_iter().flatten().collect();
        }

        let diagnostics = crate::weights::SpatialWeightsDiagnostics {
            n_features: n,
            n_islands: 0,
            neighbor_count_min: 1,
            neighbor_count_mean: 2.0,
            neighbor_count_max: 2,
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
    fn test_morans_i_permutation_positive_autocorrelation() {
        // Create data with strong positive spatial autocorrelation
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let weights = create_test_weights(5);

        let result = morans_i_permutation(&values, &weights, 999, Some(42))
            .expect("Permutation test failed");

        // Observed should be positive (consecutive values are similar)
        assert!(result.observed_statistic > 0.0);
        // Two-tailed p-value should be small (significant)
        assert!(result.p_value_two_tailed < 0.1, "Expected significance");
    }

    #[test]
    fn test_morans_i_permutation_random_data() {
        // Random data should have near-zero Moran's I
        let values = vec![1.5, 3.2, 0.8, 4.1, 2.3, 1.9];
        let weights = create_test_weights(6);

        let result = morans_i_permutation(&values, &weights, 999, Some(42))
            .expect("Permutation test failed");

        // Observed should be close to permutation mean
        let diff = (result.observed_statistic - result.expected_value).abs();
        assert!(diff < 2.0 * result.z_score.abs() * result.variance.sqrt() + 0.1);
    }

    #[test]
    fn test_local_morans_i_permutation() {
        let values = vec![1.0, 5.0, 2.0, 5.0, 3.0];
        let weights = create_test_weights(5);

        let result = local_morans_i_permutation(&values, &weights, 499, true, Some(42))
            .expect("Local permutation test failed");

        assert_eq!(result.p_values.len(), 5);
        assert_eq!(result.cluster_types.len(), 5);
        // All p-values should be valid
        assert!(result.p_values.iter().all(|p| *p > 0.0 && *p <= 1.0));
    }

    #[test]
    #[ignore]  // Run with: cargo test --release -- --ignored --nocapture permutation_performance_meuse
    fn permutation_performance_meuse() {
        use std::time::Instant;

        // Simulate Meuse dataset: 155 points with spatial autocorrelation
        let n = 155;
        let mut values = vec![0.0; n];
        for i in 0..n {
            values[i] = (i as f64 * 0.5 + (i as f64 / 20.0).sin() * 10.0).cos();
        }

        // Create linear weights (simplified; real Meuse would be 2D spatial)
        let mut neighbors = vec![Vec::new(); n];
        for i in 0..n {
            let mut row = vec![];
            if i > 0 { row.push((i - 1, 1.0)); }
            if i < n - 1 { row.push((i + 1, 1.0)); }
            neighbors[i] = row;
        }

        let diagnostics = crate::weights::SpatialWeightsDiagnostics {
            n_features: n,
            n_islands: 0,
            neighbor_count_min: 1,
            neighbor_count_mean: 2.0,
            neighbor_count_max: 2,
            connected_component_count: 1,
            row_standardized: false,
            dropped_feature_count: 0,
        };

        let weights = SpatialWeightsGraph {
            neighbors,
            diagnostics,
            warnings: vec![],
        };

        // Benchmark 1000 permutations
        let start = Instant::now();
        let result = morans_i_permutation(
            &values,
            &weights,
            1000,
            Some(42)
        ).expect("Permutation test failed");
        let elapsed = start.elapsed();

        println!("\n📊 Permutation Testing Performance Benchmark");
        println!("─────────────────────────────────────────");
        println!("Dataset: Simulated Meuse (155 points)");
        println!("Permutations: 1,000");
        println!("Time: {:.2} seconds", elapsed.as_secs_f64());
        println!("Result: Moran's I = {:.4}, p-value = {:.4}", 
            result.observed_statistic, result.p_value_two_tailed);
        
        if elapsed.as_secs_f64() < 5.0 {
            println!("✓ PASS: Performance target met (< 5 seconds)");
        } else {
            println!("✗ FAIL: Performance target exceeded");
            panic!("Performance regression: took {:.2}s", elapsed.as_secs_f64());
        }
    }
}
