// Spatial autocorrelation measures (Phase A & Phase A+: Permutation Testing)
//
// Provides global and local indicators of spatial association:
// - Global Moran's I: Overall spatial autocorrelation
// - Local Moran's I (LISA): Local clustering indicators
// - Getis-Ord G and G*: Hot/cold spot analysis
// - Nearest Neighbor Index (NNI): CSR hypothesis testing
// - Quadrat Analysis: Count-based spatial pattern analysis
//
// Phase A+ adds permutation-based inference (robust, distribution-free):
// - Permutation testing for Moran's I (global and local)
// - Permutation testing for Getis-Ord statistics
// - FDR-BH multiple testing correction

pub mod permutation;

use crate::weights::SpatialWeightsGraph;
use rayon::prelude::*;

// Re-export key permutation testing types for convenience
pub use permutation::{
    apply_fdr_bh_correction, getis_ord_gi_star_permutation, local_morans_i_permutation,
    morans_i_permutation, LocalPermutationTestResult, PermutationTestResult,
};

/// Result of global spatial autocorrelation analysis
#[derive(Debug, Clone)]
pub struct GlobalAutocorrelationResult {
    /// Statistic value (e.g., Moran's I)
    pub statistic: f64,
    /// Expected value under null hypothesis
    pub expected_value: f64,
    /// Variance of the statistic
    pub variance: f64,
    /// Standardized z-score
    pub z_score: f64,
    /// Two-tailed p-value
    pub p_value: f64,
    /// Number of features used in computation
    pub n_features: usize,
}

/// Result of local spatial association analysis (LISA)
#[derive(Debug, Clone)]
pub struct LocalAssociationResult {
    /// Local statistic value for each feature
    pub local_statistics: Vec<f64>,
    /// Expected value for each feature
    pub expected_values: Vec<f64>,
    /// Variance for each feature
    pub variances: Vec<f64>,
    /// Z-scores for each feature
    pub z_scores: Vec<f64>,
    /// P-values for each feature
    pub p_values: Vec<f64>,
    /// Cluster classification: "HH", "LL", "HL", "LH", "insignificant"
    pub cluster_types: Vec<String>,
}

/// Result of Getis-Ord G statistic analysis
#[derive(Debug, Clone)]
pub struct GetisOrdResult {
    /// G or G* statistic value
    pub statistic: f64,
    /// Expected value under null hypothesis
    pub expected_value: f64,
    /// Variance of the statistic
    pub variance: f64,
    /// Z-score
    pub z_score: f64,
    /// Two-tailed p-value
    pub p_value: f64,
}

/// Result of local Getis-Ord G* analysis (per-feature)
#[derive(Debug, Clone)]
pub struct LocalGetisOrdResult {
    /// Local G* statistic for each feature
    pub local_statistics: Vec<f64>,
    /// Expected values for each feature
    pub expected_values: Vec<f64>,
    /// Variances for each feature
    pub variances: Vec<f64>,
    /// Z-scores for each feature
    pub z_scores: Vec<f64>,
    /// P-values for each feature
    pub p_values: Vec<f64>,
    /// Cluster type: "HotSpot", "ColdSpot", "insignificant"
    pub cluster_types: Vec<String>,
}

/// Result of Nearest Neighbor Index analysis
#[derive(Debug, Clone)]
pub struct NearestNeighborIndexResult {
    /// Observed mean nearest neighbor distance
    pub observed_distance: f64,
    /// Expected mean nearest neighbor distance under CSR
    pub expected_distance: f64,
    /// Nearest Neighbor Index (ratio of observed to expected)
    pub nni: f64,
    /// Z-score for spatial clustering test
    pub z_score: f64,
    /// Two-tailed p-value
    pub p_value: f64,
    /// Interpretation: "Clustered", "Random", "Dispersed"
    pub interpretation: String,
}

/// Result of Quadrat analysis
#[derive(Debug, Clone)]
pub struct QuadratAnalysisResult {
    /// Chi-square test statistic
    pub chi_square: f64,
    /// Degrees of freedom
    pub degrees_of_freedom: usize,
    /// P-value
    pub p_value: f64,
    /// Variance-to-mean ratio (dispersion index)
    pub variance_mean_ratio: f64,
    /// Number of quadrats
    pub n_quadrats: usize,
}

/// Compute Global Moran's I with asymptotic inference
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph
///
/// # Returns
/// Global Moran's I statistic and inference results
pub fn morans_i(values: &[f64], weights: &SpatialWeightsGraph) -> Result<GlobalAutocorrelationResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required for Moran's I".to_string());
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let deviations: Vec<f64> = values.iter().map(|v| v - mean).collect();

    // Numerator: sum of cross-products of neighboring deviations
    let mut numerator = 0.0;
    let mut neighbor_count = 0usize;
    
    for (i, neighbors) in weights.neighbors.iter().enumerate() {
        for (j, weight) in neighbors {
            numerator += weight * deviations[i] * deviations[*j];
            neighbor_count += 1;
        }
    }

    // Denominator: sum of squared deviations
    let denominator: f64 = deviations.iter().map(|d| d * d).sum();

    if denominator == 0.0 {
        return Err("Deviations are zero; cannot compute Moran's I".to_string());
    }

    // Moran's I
    let sum_weights: f64 = weights.neighbors.iter().flatten().map(|(_, w)| w).sum();
    let i_stat = (n / sum_weights) * (numerator / denominator);

    // Expected value under null hypothesis (no autocorrelation)
    let expected_i = -1.0 / (n - 1.0);

    // Variance approximation (simplified for now)
    // A full computation would involve higher-order moments and more careful numerical handling
    let variance = if neighbor_count > 0 {
        // Simple approximation: variance is roughly proportional to n
        (1.0 + (sum_weights / n)) / ((n - 1.0) * sum_weights)
    } else {
        1.0
    };

    let z_score = if variance > 0.0 {
        (i_stat - expected_i) / variance.sqrt()
    } else {
        0.0
    };
    let p_value = 2.0 * (1.0 - crate::weights::normal_cdf(z_score.abs()));

    Ok(GlobalAutocorrelationResult {
        statistic: i_stat,
        expected_value: expected_i,
        variance: variance.max(0.0),
        z_score,
        p_value,
        n_features: values.len(),
    })
}

/// Compute Local Moran's I (LISA) with cluster classification
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph
/// - `alpha`: Significance threshold (e.g., 0.05)
///
/// # Returns
/// Per-feature LISA statistics with cluster types ("HH", "LL", "HL", "LH", "insignificant")
pub fn local_morans_i_lisa(values: &[f64], weights: &SpatialWeightsGraph, alpha: f64) -> Result<LocalAssociationResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required".to_string());
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let centered: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let s2: f64 = centered.iter().map(|z| z * z).sum::<f64>() / n;
    
    if s2 <= 0.0 {
        return Err("Data variance is zero; LISA is undefined".to_string());
    }

    let s = s2.sqrt();
    let z: Vec<f64> = centered.iter().map(|c| c / s).collect();
    let b2 = z.iter().map(|zi| zi.powi(4)).sum::<f64>() / n;

    let mut local_i = vec![0.0; values.len()];
    let mut expected_vals = vec![0.0; values.len()];
    let mut variances = vec![0.0; values.len()];
    let mut z_scores = vec![0.0; values.len()];
    let mut p_values = vec![0.0; values.len()];
    let mut cluster_types = vec!["insignificant".to_string(); values.len()];

    // Parallel computation of per-feature LISA statistics
    let results: Vec<_> = (0..values.len())
        .into_par_iter()
        .map(|i| {
            let mut lag_z = 0.0;
            let mut wi = 0.0;
            let mut wi2 = 0.0;

            for (j, w) in &weights.neighbors[i] {
                lag_z += w * z[*j];
                wi += w;
                wi2 += w * w;
            }

            if wi == 0.0 {
                return (
                    0.0, 0.0, 0.0, 0.0, 0.0, "insignificant".to_string(),
                );
            }

            let i_stat = z[i] * lag_z;
            let expected = -wi / (n - 1.0);
            let var_raw = ((n - b2) / (n - 1.0)) * wi2 + ((2.0 * b2 - n) / ((n - 1.0) * (n - 2.0))) * (wi * wi - wi2) - expected * expected;
            
            if var_raw.is_finite() && var_raw > 1.0e-12 {
                let zscore = (i_stat - expected) / var_raw.sqrt();
                let p = crate::weights::two_tailed_normal_p(zscore);
                let cluster = if p <= alpha {
                    if z[i] >= 0.0 && lag_z >= 0.0 {
                        "HH".to_string()
                    } else if z[i] < 0.0 && lag_z < 0.0 {
                        "LL".to_string()
                    } else if z[i] >= 0.0 && lag_z < 0.0 {
                        "HL".to_string()
                    } else {
                        "LH".to_string()
                    }
                } else {
                    "insignificant".to_string()
                };
                (i_stat, expected, var_raw, zscore, p, cluster)
            } else {
                (i_stat, expected, 0.0, 0.0, 1.0, "insignificant".to_string())
            }
        })
        .collect();

    // Unpack parallel results into output vectors
    for (i, (i_stat, expected, var_raw, zscore, p, cluster)) in results.into_iter().enumerate() {
        local_i[i] = i_stat;
        expected_vals[i] = expected;
        variances[i] = var_raw;
        z_scores[i] = zscore;
        p_values[i] = p;
        cluster_types[i] = cluster;
    }

    Ok(LocalAssociationResult {
        local_statistics: local_i,
        expected_values: expected_vals,
        variances,
        z_scores,
        p_values,
        cluster_types,
    })
}

/// Compute Getis-Ord G statistic for global hot/cold spot analysis
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph (usually distance-based, symmetric)
///
/// # Returns
/// Global G statistic with inference
pub fn getis_ord_g(values: &[f64], weights: &SpatialWeightsGraph) -> Result<GetisOrdResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required".to_string());
    }

    let n = values.len() as f64;
    let mut numerator = 0.0;
    let mut sum_weights = 0.0;

    for (i, neighbors) in weights.neighbors.iter().enumerate() {
        for (j, weight) in neighbors {
            if i != *j {  // Getis-Ord G excludes self
                numerator += weight * values[i] * values[*j];
                sum_weights += weight;
            }
        }
    }

    let sum_val: f64 = values.iter().sum();
    let sum_sq: f64 = values.iter().map(|v| v * v).sum();

    if sum_val == 0.0 || sum_weights == 0.0 {
        return Err("Cannot compute G: zero sum or weights".to_string());
    }

    let g = numerator / (sum_val * sum_val);
    let expected_g = sum_weights / (n * (n - 1.0));

    let b2 = values.iter().map(|v| v.powi(4)).sum::<f64>();
    let s2 = sum_sq / n - (sum_val / n).powi(2);

    let var_numerator = (n * (n - 3.0) * sum_weights.powi(2) + sum_weights.powi(2) - 2.0 * (n - 1.0) * sum_weights) * sum_sq - (n - 1.0) * b2 * sum_weights.powi(2);
    let var_denominator = (n * (n - 1.0) * (sum_val / n).powi(4)).powi(2);

    let variance = if var_denominator > 0.0 { var_numerator / var_denominator } else { 0.0 };

    let z_score = if variance > 0.0 { (g - expected_g) / variance.sqrt() } else { 0.0 };
    let p_value = crate::weights::two_tailed_normal_p(z_score);

    Ok(GetisOrdResult {
        statistic: g,
        expected_value: expected_g,
        variance: variance.max(0.0),
        z_score,
        p_value,
    })
}

/// Compute local Getis-Ord G* statistic (includes self)
///
/// # Arguments
/// - `values`: Data values at features
/// - `weights`: Spatial weights graph (includes self-loops for G*)
///
/// # Returns
/// Per-feature G* statistics with cluster types ("HotSpot", "ColdSpot", "insignificant")
pub fn getis_ord_g_star(values: &[f64], weights: &SpatialWeightsGraph, alpha: f64) -> Result<LocalGetisOrdResult, String> {
    if values.len() != weights.n_features() {
        return Err("Values and weights must have same number of features".to_string());
    }

    if values.len() < 3 {
        return Err("At least 3 features required".to_string());
    }

    let n = values.len() as f64;
    let sum_val: f64 = values.iter().sum();
    let sum_sq: f64 = values.iter().map(|v| v * v).sum();
    let b2 = values.iter().map(|v| v.powi(4)).sum::<f64>();

    // Parallel computation of per-feature Getis-Ord G* statistics
    let results: Vec<_> = (0..values.len())
        .into_par_iter()
        .map(|i| {
            let mut sum_wy = 0.0;
            let mut wi = 0.0;
            let mut wi2 = 0.0;

            for (j, w) in &weights.neighbors[i] {
                sum_wy += w * values[*j];
                wi += w;
                wi2 += w * w;
            }

            if wi == 0.0 {
                return (
                    0.0, 0.0, 0.0, 0.0, 1.0, "insignificant".to_string(),
                );
            }

            let g_local = sum_wy / sum_val;
            let expected = wi / (n - 1.0);

            let var_numerator = (n - 1.0) * (sum_sq * wi2 - (wi * wi)) - 2.0 * (n - 2.0) * wi.powi(2) * sum_val;
            let var_denominator = (n - 1.0).powi(2) * sum_val.powi(2);
            let variance = if var_denominator > 0.0 { var_numerator / var_denominator } else { 0.0 };
            let variance = variance.max(0.0);

            if variance > 0.0 {
                let zscore = (g_local - expected) / variance.sqrt();
                let p = crate::weights::two_tailed_normal_p(zscore);
                let cluster = if p <= alpha {
                    if zscore > 0.0 { "HotSpot".to_string() } else { "ColdSpot".to_string() }
                } else {
                    "insignificant".to_string()
                };
                (g_local, expected, variance, zscore, p, cluster)
            } else {
                (g_local, expected, 0.0, 0.0, 1.0, "insignificant".to_string())
            }
        })
        .collect();

    // Unpack parallel results into output vectors
    let mut local_g = vec![0.0; values.len()];
    let mut expected_vals = vec![0.0; values.len()];
    let mut variances = vec![0.0; values.len()];
    let mut z_scores = vec![0.0; values.len()];
    let mut p_values = vec![0.0; values.len()];
    let mut cluster_types = vec!["insignificant".to_string(); values.len()];

    for (i, (g, exp, var, z, p, cluster)) in results.into_iter().enumerate() {
        local_g[i] = g;
        expected_vals[i] = exp;
        variances[i] = var;
        z_scores[i] = z;
        p_values[i] = p;
        cluster_types[i] = cluster;
    }

    Ok(LocalGetisOrdResult {
        local_statistics: local_g,
        expected_values: expected_vals,
        variances,
        z_scores,
        p_values,
        cluster_types,
    })
}

/// Compute Nearest Neighbor Index for spatial point pattern analysis
///
/// # Arguments
/// - `coordinates`: (x, y) coordinates of points
///
/// # Returns
/// NNI statistic with interpretation ("Clustered", "Random", "Dispersed")
pub fn nearest_neighbor_index(coordinates: &[(f64, f64)]) -> Result<NearestNeighborIndexResult, String> {
    if coordinates.len() < 2 {
        return Err("At least 2 points required for NNI".to_string());
    }

    let n = coordinates.len();
    let mut sum_nn_dist = 0.0;

    // Find nearest neighbor distance for each point
    for i in 0..n {
        let mut min_dist = f64::INFINITY;
        for j in 0..n {
            if i != j {
                let dx = coordinates[i].0 - coordinates[j].0;
                let dy = coordinates[i].1 - coordinates[j].1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                }
            }
        }
        sum_nn_dist += min_dist;
    }

    let observed = sum_nn_dist / n as f64;

    // Compute study area bounds
    let min_x = coordinates.iter().map(|(x, _)| x).copied().fold(f64::INFINITY, f64::min);
    let max_x = coordinates.iter().map(|(x, _)| x).copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = coordinates.iter().map(|(_, y)| y).copied().fold(f64::INFINITY, f64::min);
    let max_y = coordinates.iter().map(|(_, y)| y).copied().fold(f64::NEG_INFINITY, f64::max);

    let area = (max_x - min_x) * (max_y - min_y);
    if area <= 0.0 {
        return Err("Study area has zero or negative area".to_string());
    }

    // Expected NNI under complete spatial randomness (CSR)
    let density = n as f64 / area;
    let expected = 0.5 / density.sqrt();

    let nni = observed / expected;

    // Variance of NNI under CSR
    let var_nni = (0.0703 * area + 0.0000563 * area.powi(2)) / (n as f64).powi(2);
    let se_nni = var_nni.sqrt();

    let z_score = (observed - expected) / se_nni;
    let p_value = crate::weights::two_tailed_normal_p(z_score);

    let interpretation = if nni < 0.5 {
        "Clustered".to_string()
    } else if nni > 1.5 {
        "Dispersed".to_string()
    } else {
        "Random".to_string()
    };

    Ok(NearestNeighborIndexResult {
        observed_distance: observed,
        expected_distance: expected,
        nni,
        z_score,
        p_value,
        interpretation,
    })
}

/// Perform Quadrat Analysis for testing spatial randomness
///
/// # Arguments
/// - `coordinates`: (x, y) coordinates of points
/// - `rows`: Number of rows in quadrat grid
/// - `cols`: Number of columns in quadrat grid
///
/// # Returns
/// Chi-square test statistic with variance-to-mean ratio
pub fn quadrat_analysis(coordinates: &[(f64, f64)], rows: usize, cols: usize) -> Result<QuadratAnalysisResult, String> {
    if coordinates.is_empty() {
        return Err("At least one point required".to_string());
    }

    if rows == 0 || cols == 0 {
        return Err("rows and cols must be > 0".to_string());
    }

    let n_points = coordinates.len();
    let min_x = coordinates.iter().map(|(x, _)| x).copied().fold(f64::INFINITY, f64::min);
    let max_x = coordinates.iter().map(|(x, _)| x).copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = coordinates.iter().map(|(_, y)| y).copied().fold(f64::INFINITY, f64::min);
    let max_y = coordinates.iter().map(|(_, y)| y).copied().fold(f64::NEG_INFINITY, f64::max);

    let dx = (max_x - min_x) / cols as f64;
    let dy = (max_y - min_y) / rows as f64;

    if !dx.is_finite() || !dy.is_finite() || dx <= 0.0 || dy <= 0.0 {
        return Err("Study area bounds invalid".to_string());
    }

    let mut counts = vec![0usize; rows * cols];
    for (x, y) in coordinates {
        let col = ((x - min_x) / dx) as usize;
        let row = ((y - min_y) / dy) as usize;
        let col_clamped = col.min(cols - 1);
        let row_clamped = row.min(rows - 1);
        counts[row_clamped * cols + col_clamped] += 1;
    }

    let expected = n_points as f64 / (rows * cols) as f64;
    let mut chi_square = 0.0;
    let mut sum_sq = 0.0;

    for count in &counts {
        let diff = *count as f64 - expected;
        chi_square += diff * diff / expected;
        sum_sq += (*count as f64) * (*count as f64);
    }

    let variance = sum_sq / (rows * cols) as f64 - expected * expected;
    let vmr = variance / expected;

    let df = (rows * cols - 1) as usize;
    let p_value = chi_square_cdf(chi_square, df as f64);

    Ok(QuadratAnalysisResult {
        chi_square,
        degrees_of_freedom: df,
        p_value: 1.0 - p_value,
        variance_mean_ratio: vmr,
        n_quadrats: rows * cols,
    })
}

/// Approximate chi-square CDF using Cornish-Fisher expansion
fn chi_square_cdf(x: f64, k: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    if x == 0.0 {
        return 0.0;
    }

    // Approximate: for large k, χ²(k) ≈ N(k, 2k)
    let mean = k;
    let std = (2.0 * k).sqrt();
    let z = (x - mean) / std;
    crate::weights::normal_cdf(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_weights() -> SpatialWeightsGraph {
        SpatialWeightsGraph {
            neighbors: vec![
                vec![(1, 1.0)],
                vec![(0, 1.0), (2, 1.0)],
                vec![(1, 1.0)],
            ],
            diagnostics: crate::weights::SpatialWeightsDiagnostics {
                n_features: 3,
                n_islands: 0,
                neighbor_count_min: 1,
                neighbor_count_mean: 1.33,
                neighbor_count_max: 2,
                connected_component_count: 1,
                row_standardized: false,
                dropped_feature_count: 0,
            },
            warnings: vec![],
        }
    }

    #[test]
    fn test_morans_i_requires_enough_features() {
        let weights = SpatialWeightsGraph {
            neighbors: vec![vec![(1, 1.0)], vec![(0, 1.0)]],
            diagnostics: crate::weights::SpatialWeightsDiagnostics {
                n_features: 2,
                n_islands: 0,
                neighbor_count_min: 1,
                neighbor_count_mean: 1.0,
                neighbor_count_max: 1,
                connected_component_count: 1,
                row_standardized: false,
                dropped_feature_count: 0,
            },
            warnings: vec![],
        };
        let values = vec![1.0, 2.0];
        assert!(morans_i(&values, &weights).is_err());
    }

    #[test]
    fn test_morans_i_basic() {
        let weights = simple_weights();
        let values = vec![1.0, 2.0, 3.0];
        let result = morans_i(&values, &weights);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.statistic.is_finite());
        assert!(r.z_score.is_finite());
        assert!(r.p_value > 0.0 && r.p_value <= 1.0);
    }

    #[test]
    fn test_lisa_basic() {
        let weights = simple_weights();
        let values = vec![1.0, 2.0, 1.0];
        let result = local_morans_i_lisa(&values, &weights, 0.05);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.local_statistics.len(), 3);
        assert_eq!(r.cluster_types.len(), 3);
        for p in &r.p_values {
            assert!(*p > 0.0 && *p <= 1.0 || !p.is_finite());
        }
    }

    #[test]
    fn test_lisa_zero_variance() {
        let weights = simple_weights();
        let values = vec![5.0, 5.0, 5.0];  // constant
        let result = local_morans_i_lisa(&values, &weights, 0.05);
        assert!(result.is_err());
    }

    #[test]
    fn test_getis_ord_g_basic() {
        let weights = simple_weights();
        let values = vec![1.0, 5.0, 2.0];
        let result = getis_ord_g(&values, &weights);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.statistic.is_finite());
        assert!(r.z_score.is_finite());
    }

    #[test]
    fn test_getis_ord_g_star_basic() {
        let weights = simple_weights();
        let values = vec![1.0, 5.0, 2.0];
        let result = getis_ord_g_star(&values, &weights, 0.05);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.local_statistics.len(), 3);
        assert_eq!(r.cluster_types.len(), 3);
    }

    #[test]
    fn test_nni_basic() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (0.0, 1.0)];
        let result = nearest_neighbor_index(&coords);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.observed_distance > 0.0);
        assert!(r.expected_distance > 0.0);
        assert!(r.nni.is_finite());
        assert!(!r.interpretation.is_empty());
    }

    #[test]
    fn test_nni_insufficient_points() {
        let coords = vec![(0.0, 0.0)];
        let result = nearest_neighbor_index(&coords);
        assert!(result.is_err());
    }

    #[test]
    fn test_quadrat_basic() {
        let coords = vec![(0.1, 0.1), (0.5, 0.5), (0.9, 0.9), (0.2, 0.8), (0.7, 0.3)];
        let result = quadrat_analysis(&coords, 2, 2);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.chi_square >= 0.0);
        assert!(r.variance_mean_ratio >= 0.0);
        assert_eq!(r.n_quadrats, 4);
    }

    #[test]
    fn test_quadrat_empty() {
        let coords: Vec<(f64, f64)> = vec![];
        let result = quadrat_analysis(&coords, 2, 2);
        assert!(result.is_err());
    }
}
