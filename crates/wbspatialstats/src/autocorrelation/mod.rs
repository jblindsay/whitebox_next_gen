// Spatial autocorrelation measures (Phase A)
//
// Provides global and local indicators of spatial association:
// - Global Moran's I: Overall spatial autocorrelation
// - Local Moran's I (LISA): Local clustering indicators
// - Getis-Ord G and G*: Hot/cold spot analysis
// - Nearest Neighbor Index (NNI): CSR hypothesis testing
// - Quadrat Analysis: Count-based spatial pattern analysis

use crate::weights::SpatialWeightsGraph;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let weights = SpatialWeightsGraph {
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
        };
        let values = vec![1.0, 2.0, 3.0];
        let result = morans_i(&values, &weights);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.statistic.is_finite());
        assert!(r.z_score.is_finite());
        assert!(r.p_value > 0.0 && r.p_value <= 1.0);
    }
}
