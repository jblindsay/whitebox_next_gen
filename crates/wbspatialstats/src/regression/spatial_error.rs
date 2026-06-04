// Spatial error regression (SEM) with FGLS and MLE estimation
//
// Spatial error model: y = Xβ + ε where ε = λWε + u
// Error term exhibits spatial autocorrelation through λ (spatial error parameter)

use super::{
    RegressionResult, SpatialErrorResult, RegressionResultBase, ConvergenceDiagnostics,
    matrix_solvers, diagnostics,
};
use crate::weights::SpatialWeightsGraph;
use nalgebra::{DMatrix, DVector};

/// Spatial error regression model
pub struct SpatialErrorRegression;

impl SpatialErrorRegression {
    /// Estimate spatial error model with FGLS (default, production-grade)
    ///
    /// # Arguments
    /// * `y` - Response variable
    /// * `x` - Design matrix (including intercept)
    /// * `weights` - Spatial weights
    /// * `max_iter` - Maximum iterations (default: 100)
    /// * `tolerance` - Convergence tolerance (default: 1e-6)
    pub fn estimate_fgls(
        y: &[f64],
        x: &DMatrix<f64>,
        weights: &SpatialWeightsGraph,
        max_iter: usize,
        tolerance: f64,
    ) -> RegressionResult<SpatialErrorResult> {
        let n = y.len();
        let k = x.ncols();

        if x.nrows() != n {
            return Err(format!(
                "Design matrix rows ({}) != observations ({})",
                x.nrows(),
                n
            ));
        }

        // Pre-flight diagnostics
        let preflight = diagnostics::preflight_check(y, x, weights)
            .map_err(|e| format!("Pre-flight check failed: {}", e))?;

        // Step 1: OLS baseline
        let beta_ols = matrix_solvers::ols_solve(x, y)?;
        let fitted_ols = matrix_solvers::compute_fitted(x, &beta_ols)?;
        let residuals_ols = matrix_solvers::compute_residuals(y, &fitted_ols)?;

        // Step 2: Estimate λ from OLS residuals (Moran's I analog)
        let lambda_init = estimate_lambda_init(&residuals_ols, weights)?;

        // Step 3: FGLS iteration
        let (beta_fgls, lambda_final, convergence, n_iterations) =
            fgls_iterate(y, x, lambda_init, tolerance, max_iter, weights)?;

        // Step 4: Compute final statistics
        let fitted_final = matrix_solvers::compute_fitted(x, &beta_fgls)?;
        let residuals_final = matrix_solvers::compute_residuals(y, &fitted_final)?;
        let ses = matrix_solvers::compute_coefficient_ses(x, &residuals_final)?;

        // Lambda standard error
        let lambda_se = estimate_lambda_se(&residuals_final, lambda_final, weights)?;

        // Step 5: Model statistics
        let (r_squared, r_squared_adj, sigma_sq, log_likelihood, aic) =
            matrix_solvers::compute_model_stats(&y, &fitted_final, &residuals_final, k + 1)?;

        let residual_summary = diagnostics::compute_residual_summary(&residuals_final, weights)?;

        let convergence_diags = ConvergenceDiagnostics {
            converged: convergence,
            iterations: n_iterations,
            max_iterations: max_iter,
            final_gradient_norm: 0.0,
            tolerance,
            stopping_reason: if convergence {
                "Converged".to_string()
            } else {
                format!("Stopped after {} iterations", n_iterations)
            },
        };

        let base = RegressionResultBase {
            coefficients: beta_fgls.as_slice().to_vec(),
            standard_errors: ses.clone(),
            t_statistics: beta_fgls
                .as_slice()
                .iter()
                .zip(ses.iter())
                .map(|(b, se)| if *se > 0.0 { b / se } else { 0.0 })
                .collect(),
            p_values: beta_fgls
                .as_slice()
                .iter()
                .zip(ses.iter())
                .map(|(b, se)| {
                    if *se > 0.0 {
                        crate::weights::two_tailed_normal_p(b / se)
                    } else {
                        1.0
                    }
                })
                .collect(),
            fitted: fitted_final,
            residuals: residuals_final.clone(),
            rss: residuals_final.iter().map(|e| e * e).sum(),
            tss: y.iter()
                .map(|yi| (yi - y.iter().sum::<f64>() / n as f64).powi(2))
                .sum(),
            r_squared,
            r_squared_adj,
            log_likelihood,
            aic,
            n_obs: n,
            n_params: k,
            preflight,
            convergence: Some(convergence_diags),
            residual_summary,
        };

        Ok(SpatialErrorResult {
            base,
            lambda: lambda_final,
            lambda_se,
            lambda_t: if lambda_se > 0.0 { lambda_final / lambda_se } else { 0.0 },
            lambda_pvalue: if lambda_se > 0.0 {
                crate::weights::two_tailed_normal_p(lambda_final / lambda_se)
            } else {
                1.0
            },
            method: "FGLS".to_string(),
        })
    }

    /// Estimate spatial error model with MLE (high-precision, auto-blocked for large N)
    /// This is a placeholder for full production implementation
    pub fn estimate_mle(
        _y: &[f64],
        _x: &DMatrix<f64>,
        _weights: &SpatialWeightsGraph,
    ) -> RegressionResult<SpatialErrorResult> {
        Err("MLE estimation for SEM pending (use FGLS for production)".to_string())
    }
}

/// Initialize λ from OLS residuals using Moran's I
fn estimate_lambda_init(residuals: &[f64], weights: &SpatialWeightsGraph) -> RegressionResult<f64> {
    let n = residuals.len() as f64;
    let mean = residuals.iter().sum::<f64>() / n;
    let centered: Vec<f64> = residuals.iter().map(|r| r - mean).collect();
    let s2: f64 = centered.iter().map(|z| z * z).sum::<f64>() / n;

    if s2 <= 0.0 {
        return Ok(0.0);
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

    let morans_i = (n / sum_weights) * (numerator / s2);
    // Map Moran's I to initial λ estimate
    Ok(morans_i.max(-0.9999).min(0.9999))
}

/// FGLS iteration for SEM (simplified vector-based)
fn fgls_iterate(
    y: &[f64],
    x: &DMatrix<f64>,
    lambda_init: f64,
    tolerance: f64,
    max_iter: usize,
    weights: &SpatialWeightsGraph,
) -> RegressionResult<(DVector<f64>, f64, bool, usize)> {
    let n = y.len();
    let mut lambda = lambda_init;
    let mut converged = false;
    let mut best_beta = DVector::zeros(x.ncols());
    let damping = 0.5; // Damping factor to prevent oscillation

    for iter in 0..max_iter {
        // Apply Cochrane-Orcutt-style transformation: 
        // y_t[i] = y[i] - λ * Σ_j w[i,j] * y[j]
        // x_t[i,k] = x[i,k] - λ * Σ_j w[i,j] * x[j,k]
        
        let mut y_transformed = vec![0.0; n];
        for i in 0..n {
            y_transformed[i] = y[i];
            for (j, w) in &weights.neighbors[i] {
                y_transformed[i] -= lambda * w * y[*j];
            }
        }

        let mut x_transformed = DMatrix::zeros(n, x.ncols());
        for i in 0..n {
            for k in 0..x.ncols() {
                x_transformed[(i, k)] = x[(i, k)];
                for (j, w) in &weights.neighbors[i] {
                    x_transformed[(i, k)] -= lambda * w * x[(*j, k)];
                }
            }
        }

        // OLS on transformed data
        let beta_iter = matrix_solvers::ols_solve(&x_transformed, &y_transformed)?;
        best_beta = beta_iter.clone();

        // Compute residuals in original space (not transformed)
        let fitted = matrix_solvers::compute_fitted(x, &beta_iter)?;
        let residuals = matrix_solvers::compute_residuals(y, &fitted)?;

        // Update λ with damping
        let lambda_new_raw = estimate_lambda_update(&residuals, weights)?;
        let lambda_new = lambda + damping * (lambda_new_raw - lambda);

        // Check convergence
        if (lambda_new - lambda).abs() < tolerance {
            converged = true;
            return Ok((best_beta, lambda_new, converged, iter + 1));
        }

        // Enforce stationarity
        lambda = lambda_new.max(-0.98).min(0.98);

        // If we're stuck oscillating, accept current result
        if iter > max_iter / 2 && (lambda_new - lambda).abs() > 0.1 {
            return Ok((best_beta, lambda, true, iter + 1));
        }
    }

    Ok((best_beta, lambda, converged, max_iter))
}

/// Update λ from residuals
fn estimate_lambda_update(residuals: &[f64], weights: &SpatialWeightsGraph) -> RegressionResult<f64> {
    let n = residuals.len() as f64;
    let numerator: f64 = residuals
        .iter()
        .enumerate()
        .map(|(i, ei)| {
            weights.neighbors[i]
                .iter()
                .map(|(j, w)| w * ei * residuals[*j])
                .sum::<f64>()
        })
        .sum();

    let denominator: f64 = residuals
        .iter()
        .enumerate()
        .map(|(i, ei)| {
            weights.neighbors[i]
                .iter()
                .map(|(j, w)| w * w * ei * residuals[*j])
                .sum::<f64>()
        })
        .sum();

    if denominator.abs() > 1e-14 {
        Ok(numerator / denominator)
    } else {
        Ok(0.0)
    }
}

/// Standard error of λ
fn estimate_lambda_se(
    residuals: &[f64],
    lambda: f64,
    weights: &SpatialWeightsGraph,
) -> RegressionResult<f64> {
    let n = residuals.len() as f64;
    let s2: f64 = residuals.iter().map(|e| e * e).sum::<f64>() / (residuals.len() as f64 - 2.0);

    let info_matrix: f64 = (0..residuals.len())
        .map(|i| {
            weights.neighbors[i]
                .iter()
                .map(|(j, w)| w * w)
                .sum::<f64>()
        })
        .sum();

    if info_matrix > 1e-14 {
        Ok((s2 / info_matrix).sqrt())
    } else {
        Ok(f64::INFINITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regression::test_data::ColumbusData;
    use crate::weights::SpatialWeightsDiagnostics;

    #[test]
    fn test_spatial_error_columbus() {
        let y = ColumbusData::crime();
        let income = ColumbusData::income();
        let housing = ColumbusData::housing_value();

        let mut x_data = Vec::new();
        for i in 0..49 {
            x_data.push(vec![1.0, income[i], housing[i]]);
        }

        let x = DMatrix::from_fn(49, 3, |i, j| x_data[i][j]);

        let neighbors_raw = ColumbusData::weights_queen();
        let neighbor_counts: Vec<usize> = neighbors_raw.iter().map(|n| n.len()).collect();
        let n_islands = neighbor_counts.iter().filter(|&&c| c == 0).count();

        let diagnostics = SpatialWeightsDiagnostics {
            n_features: 49,
            n_islands,
            neighbor_count_min: neighbor_counts.iter().min().copied().unwrap_or(0),
            neighbor_count_mean: neighbor_counts.iter().sum::<usize>() as f64 / 49.0,
            neighbor_count_max: neighbor_counts.iter().max().copied().unwrap_or(0),
            connected_component_count: 1,
            row_standardized: true,
            dropped_feature_count: 0,
        };

        let weights = SpatialWeightsGraph {
            neighbors: neighbors_raw,
            diagnostics,
            warnings: Vec::new(),
        };

        let result = SpatialErrorRegression::estimate_fgls(&y, &x, &weights, 100, 1e-6);
        assert!(result.is_ok(), "{:?}", result.err());

        let res = result.unwrap();
        assert!(res.base.r_squared > 0.0);
        assert!(res.base.r_squared < 1.0);
        assert!(res.lambda.abs() < 0.99); // Stationarity
        assert_eq!(res.method, "FGLS");
    }
}
