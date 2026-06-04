// Diagnostics and pre-flight checks for spatial regression

use crate::weights::SpatialWeightsGraph;
use nalgebra::{DMatrix, SVD};
use super::{PreFlightDiagnostics, ResidualSummary};

/// Perform comprehensive pre-flight diagnostic checks before fitting
pub fn preflight_check(
    y: &[f64],
    x: &DMatrix<f64>,
    weights: &SpatialWeightsGraph,
) -> Result<PreFlightDiagnostics, String> {
    let mut design_warnings = Vec::new();
    let mut response_warnings = Vec::new();
    let mut weights_warnings = Vec::new();

    let n_obs = y.len();
    let n_vars = x.ncols();

    // Check response variable
    let y_mean = y.iter().sum::<f64>() / n_obs as f64;
    let y_var = y.iter().map(|v| (v - y_mean).powi(2)).sum::<f64>() / n_obs as f64;

    if y_var < 1e-12 {
        return Err("Response variable has zero or near-zero variance".to_string());
    }

    if y_var.is_nan() || y_var.is_infinite() {
        response_warnings.push("Response variance is NaN or infinite".to_string());
    }

    // Check design matrix
    if n_vars > n_obs {
        return Err(format!(
            "More variables ({}) than observations ({}); underdetermined system",
            n_vars, n_obs
        ));
    }

    // Compute condition number via SVD
    let svd = SVD::new(x.clone(), true, true);
    let singular_values = svd.singular_values;

    if singular_values.len() < n_vars {
        return Err("Design matrix rank-deficient; cannot proceed".to_string());
    }

    let sv_max = singular_values[0];
    let sv_min = singular_values[n_vars - 1];
    let condition_number = if sv_min > 1e-14 {
        sv_max / sv_min
    } else {
        f64::INFINITY
    };

    let design_matrix_rank = singular_values
        .iter()
        .filter(|sv| **sv > 1e-10 * sv_max)
        .count();

    // Condition number assessment
    if condition_number > 1e10 {
        return Err(format!(
            "Design matrix is numerically singular (condition number: {:.2e}); remove redundant variables or collect more data",
            condition_number
        ));
    }

    if condition_number > 1e6 {
        design_warnings.push(format!(
            "Design matrix is ill-conditioned (κ = {:.2e}); results may be unstable; consider variable screening or standardization",
            condition_number
        ));
    }

    if design_matrix_rank < n_vars {
        design_warnings.push(format!(
            "Design matrix rank ({}) is less than number of variables ({}); multicollinearity likely",
            design_matrix_rank, n_vars
        ));
    }

    // Check weights matrix
    let isolated_features: Vec<usize> = weights
        .neighbors
        .iter()
        .enumerate()
        .filter_map(|(i, neighbors)| if neighbors.is_empty() { Some(i) } else { None })
        .collect();

    if !isolated_features.is_empty() {
        if isolated_features.len() <= 10 {
            weights_warnings.push(format!(
                "Found {} isolated features (no neighbors): {:?}; will be dropped from inference",
                isolated_features.len(),
                isolated_features
            ));
        } else {
            weights_warnings.push(format!(
                "Found {} isolated features (no neighbors); will be dropped from inference",
                isolated_features.len()
            ));
        }
    }

    let can_proceed = design_warnings.is_empty()
        && response_warnings.is_empty()
        && weights_warnings.is_empty();

    Ok(PreFlightDiagnostics {
        design_matrix_condition_number: condition_number,
        design_matrix_rank,
        response_variance: y_var,
        design_warnings,
        response_warnings,
        weights_warnings,
        can_proceed,
    })
}

/// Compute residual summary with spatial autocorrelation diagnostics
pub fn compute_residual_summary(
    residuals: &[f64],
    weights: &SpatialWeightsGraph,
) -> Result<ResidualSummary, String> {
    if residuals.is_empty() {
        return Err("No residuals provided".to_string());
    }

    let n = residuals.len();
    let mean = residuals.iter().sum::<f64>() / n as f64;
    let variance = residuals.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    // Percentiles
    let mut sorted = residuals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[n - 1];
    let q25 = sorted[n / 4];
    let median = if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    };
    let q75 = sorted[(3 * n) / 4];

    // Moran's I on residuals
    let morans_i = compute_morans_i(residuals, weights)?;
    let morans_i_pvalue = if morans_i.is_finite() {
        crate::weights::two_tailed_normal_p(morans_i)
    } else {
        1.0
    };

    // Generate interpretation
    let interpretation = if morans_i_pvalue < 0.05 {
        if morans_i > 0.0 {
            format!(
                "Significant positive spatial autocorrelation in residuals (I={:.4}, p={:.4}); model may be missing spatial structure",
                morans_i, morans_i_pvalue
            )
        } else {
            format!(
                "Significant negative spatial autocorrelation in residuals (I={:.4}, p={:.4}); overdispersion",
                morans_i, morans_i_pvalue
            )
        }
    } else {
        format!(
            "No significant spatial autocorrelation in residuals (I={:.4}, p={:.4}); spatial structure adequately captured",
            morans_i, morans_i_pvalue
        )
    };

    Ok(ResidualSummary {
        mean,
        std_dev,
        min,
        q25,
        median,
        q75,
        max,
        morans_i,
        morans_i_pvalue,
        interpretation,
    })
}

/// Compute Moran's I for residuals (simplified, no island handling)
fn compute_morans_i(
    residuals: &[f64],
    weights: &SpatialWeightsGraph,
) -> Result<f64, String> {
    let n = residuals.len() as f64;
    let mean = residuals.iter().sum::<f64>() / n;
    let centered: Vec<f64> = residuals.iter().map(|r| r - mean).collect();
    let s2: f64 = centered.iter().map(|z| z * z).sum::<f64>() / n;

    if s2 <= 0.0 {
        return Ok(0.0); // No variation, no autocorrelation
    }

    let mut numerator = 0.0;
    let mut sum_weights = 0.0;

    for (i, neighbors) in weights.neighbors.iter().enumerate() {
        for (j, weight) in neighbors {
            numerator += weight * centered[i] * centered[*j];
            sum_weights += weight;
        }
    }

    if sum_weights == 0.0 {
        return Ok(0.0);
    }

    let morans_i = (n / sum_weights) * (numerator / (s2 * n));
    Ok(morans_i)
}

/// Compute Akaike Information Criterion with small-sample correction (AICc)
pub fn compute_aicc(log_likelihood: f64, k: usize, n: usize) -> f64 {
    let aic = 2.0 * k as f64 - 2.0 * log_likelihood;
    let correction = if n > k + 1 {
        (2.0 * k as f64 * (k as f64 + 1.0)) / (n as f64 - k as f64 - 1.0)
    } else {
        // For small samples, use large penalty
        1000.0
    };
    aic + correction
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn test_aicc_computation() {
        // Basic test: AICc should be larger than AIC for small N
        let ll = -50.0;
        let k = 5;
        let n_large = 1000;
        let n_small = 10;

        let aicc_large = compute_aicc(ll, k, n_large);
        let aicc_small = compute_aicc(ll, k, n_small);
        let aic = 2.0 * k as f64 - 2.0 * ll;

        assert!(aicc_large > aic); // AICc > AIC
        assert!(aicc_small > aicc_large); // Smaller N has larger penalty
    }
}
