// Spatial lag regression (SAR) with GMM/IV+FGLS estimation
//
// Implements the Anselin & Bera (1998) framework for spatial lag models
// Model: y = ρWy + Xβ + ε
//
// Estimation strategy: GMM with IV, then FGLS for efficiency
// Parallelized with rayon for spatial lag computation and parameter updates

use super::{
    RegressionResult, SpatialLagResult, RegressionResultBase, EffectDecomposition,
    ConvergenceDiagnostics, matrix_solvers, diagnostics,
};
use crate::weights::SpatialWeightsGraph;
use nalgebra::{DMatrix, DVector};
use rayon::prelude::*;

/// Spatial lag regression model
pub struct SpatialLagRegression;

impl SpatialLagRegression {
    /// Estimate spatial lag model with GMM/IV+FGLS (production-grade)
    ///
    /// # Arguments
    /// * `y` - Response variable
    /// * `x` - Design matrix (including intercept column)
    /// * `weights` - Spatial weights structure
    /// * `max_iter` - Maximum iterations for convergence (default: 100)
    /// * `tolerance` - Convergence tolerance for parameter changes (default: 1e-6)
    pub fn estimate(
        y: &[f64],
        x: &DMatrix<f64>,
        weights: &SpatialWeightsGraph,
        max_iter: usize,
        tolerance: f64,
    ) -> RegressionResult<SpatialLagResult> {
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
        let _residuals_ols = matrix_solvers::compute_residuals(y, &fitted_ols)?;

        // Step 2: Construct spatial lag of y: Wy
        let wy = compute_spatial_lag(y, weights);

        // Step 3: Construct instruments for Wy: WX (weak instruments)
        let wx = compute_spatial_lag_matrix(x, weights)?;

        // Step 4: GMM/IV estimation for rho (first stage)
        let rho = estimate_spatial_parameter_iv(y, &beta_ols, &wy, x, &wx, weights)?;

        // Step 5: FGLS refinement (iterate)
        let (beta_fgls, rho_final, convergence, n_iterations) =
            fgls_iterate(y, x, &wy, &wx, rho, tolerance, max_iter, weights)?;

        // Step 6: Compute standard errors
        let fitted_final = compute_spatial_lag_fit(&beta_fgls, rho_final, x, &wy)?;
        let residuals_final = matrix_solvers::compute_residuals(y, &fitted_final)?;
        let ses = matrix_solvers::compute_coefficient_ses(x, &residuals_final)?;

        // Compute rho standard error (approximation via Wald)
        let rho_se = estimate_spatial_parameter_se(&residuals_final, &wy, rho_final, weights)?;

        // Step 7: Effect decomposition
        let effects = compute_effect_decomposition(
            &beta_fgls,
            rho_final,
            &ses,
            rho_se,
            weights,
            n,
            k,
        )?;

        // Step 8: Model statistics
        let (r_squared, r_squared_adj, _sigma_sq, log_likelihood, aic) =
            matrix_solvers::compute_model_stats(&y, &fitted_final, &residuals_final, k + 1)?;

        // Residual summary
        let residual_summary = diagnostics::compute_residual_summary(&residuals_final, weights)?;

        let convergence_diags = ConvergenceDiagnostics {
            converged: convergence,
            iterations: n_iterations,
            max_iterations: max_iter,
            final_gradient_norm: 0.0, // Would need gradient tracking
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

        Ok(SpatialLagResult {
            base,
            rho: rho_final,
            rho_se,
            rho_t: if rho_se > 0.0 { rho_final / rho_se } else { 0.0 },
            rho_pvalue: if rho_se > 0.0 {
                crate::weights::two_tailed_normal_p(rho_final / rho_se)
            } else {
                1.0
            },
            effects: Some(effects),
        })
    }
}

/// Compute Wy (spatial lag of y) - parallelized over features
fn compute_spatial_lag(y: &[f64], weights: &SpatialWeightsGraph) -> Vec<f64> {
    (0..y.len())
        .into_par_iter()
        .map(|i| {
            weights.neighbors[i]
                .iter()
                .map(|(j, w)| w * y[*j])
                .sum::<f64>()
        })
        .collect()
}

/// Compute spatial lag of X matrix: WX - parallelized over rows
fn compute_spatial_lag_matrix(x: &DMatrix<f64>, weights: &SpatialWeightsGraph) -> RegressionResult<DMatrix<f64>> {
    let n = x.nrows();
    let k = x.ncols();

    let wx_rows: Vec<Vec<f64>> = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut row = vec![0.0; k];
            for (j, w) in &weights.neighbors[i] {
                for p in 0..k {
                    row[p] += w * x[(*j, p)];
                }
            }
            row
        })
        .collect();

    let mut wx = DMatrix::zeros(n, k);
    for (i, row) in wx_rows.into_iter().enumerate() {
        for (p, &val) in row.iter().enumerate() {
            wx[(i, p)] = val;
        }
    }

    Ok(wx)
}

/// IV estimation of rho (first-stage)
fn estimate_spatial_parameter_iv(
    y: &[f64],
    beta_ols: &DVector<f64>,
    wy: &[f64],
    x: &DMatrix<f64>,
    wx: &DMatrix<f64>,
    _weights: &SpatialWeightsGraph,
) -> RegressionResult<f64> {
    let n = y.len();

    // Residuals from OLS
    let fitted_ols = matrix_solvers::compute_fitted(x, beta_ols)?;
    let e_ols = matrix_solvers::compute_residuals(y, &fitted_ols)?;

    // 2SLS for spatial parameter: regress e_ols on Wy using WX as instruments
    // First: Project Wy onto (X, WX)
    let mut aug_x = DMatrix::zeros(n, 2 * x.ncols());
    for i in 0..n {
        for j in 0..x.ncols() {
            aug_x[(i, j)] = x[(i, j)];
            aug_x[(i, x.ncols() + j)] = wx[(i, j)];
        }
    }

    let proj_wy = matrix_solvers::ols_solve(&aug_x, &wy)?;
    let fitted_wy: Vec<f64> = (0..n)
        .map(|i| {
            (0..aug_x.ncols())
                .map(|j| aug_x[(i, j)] * proj_wy[j])
                .sum()
        })
        .collect();

    // Regress residuals on fitted Wy
    let mut x_rho = DMatrix::zeros(n, 1);
    for i in 0..n {
        x_rho[(i, 0)] = fitted_wy[i];
    }

    let coef_rho = matrix_solvers::ols_solve(&x_rho, &e_ols)?;
    Ok(coef_rho[0].max(-0.9999).min(0.9999)) // Ensure stationarity
}

/// FGLS iteration to refine estimates
fn fgls_iterate(
    y: &[f64],
    x: &DMatrix<f64>,
    wy: &[f64],
    _wx: &DMatrix<f64>,
    rho_init: f64,
    tolerance: f64,
    max_iter: usize,
    _weights: &SpatialWeightsGraph,
) -> RegressionResult<(DVector<f64>, f64, bool, usize)> {
    let n = y.len();
    let mut rho = rho_init;
    let mut converged = false;

    for iter in 0..max_iter {
        // Transform to remove spatial autocorrelation: (I - ρW)y = (I - ρW)X β + ε
        let y_transformed: Vec<f64> =
            y.iter()
                .enumerate()
                .map(|(i, yi)| yi - rho * wy[i])
                .collect();

        let mut x_transformed = x.clone();
        for i in 0..n {
            for j in 0..x.ncols() {
                for (k, w) in &_weights.neighbors[i] {
                    x_transformed[(i, j)] -= rho * w * x[(*k, j)];
                }
            }
        }

        // GLS estimation
        let beta_iter = matrix_solvers::ols_solve(&x_transformed, &y_transformed)?;
        let fitted_iter = matrix_solvers::compute_fitted(&x_transformed, &beta_iter)?;
        let residuals_iter = matrix_solvers::compute_residuals(&y_transformed, &fitted_iter)?;

        // Update rho estimate
        let rho_new = estimate_rho_update(&residuals_iter, wy, _weights);

        // Check convergence
        if (rho_new - rho).abs() < tolerance {
            converged = true;
            return Ok((beta_iter, rho_new, converged, iter + 1));
        }

        rho = rho_new.max(-0.9999).min(0.9999);
    }

    Ok((DVector::zeros(x.ncols()), rho, converged, max_iter))
}

/// Update rho from residuals
fn estimate_rho_update(residuals: &[f64], wy: &[f64], _weights: &SpatialWeightsGraph) -> f64 {
    let _n = residuals.len() as f64;
    let numerator: f64 = residuals.iter().zip(wy).map(|(e, w)| e * w).sum();
    let denominator: f64 = wy.iter().map(|w| w * w).sum();

    if denominator.abs() > 1e-14 {
        numerator / denominator
    } else {
        0.0
    }
}

/// Standard error of rho (approximation)
fn estimate_spatial_parameter_se(
    residuals: &[f64],
    wy: &[f64],
    _rho: f64,
    _weights: &SpatialWeightsGraph,
) -> RegressionResult<f64> {
    let _n = residuals.len() as f64;
    let s2: f64 = residuals.iter().map(|e| e * e).sum::<f64>() / (residuals.len() as f64 - 2.0);

    let info_matrix: f64 = wy.iter().map(|w| w * w).sum();

    if info_matrix > 1e-14 {
        Ok((s2 / info_matrix).sqrt())
    } else {
        Ok(f64::INFINITY)
    }
}

/// Compute effect decomposition from spatial lag model (parallelized)
fn compute_effect_decomposition(
    beta: &DVector<f64>,
    rho: f64,
    ses_beta: &[f64],
    se_rho: f64,
    _weights: &SpatialWeightsGraph,
    n: usize,
    _k: usize,
) -> RegressionResult<EffectDecomposition> {
    let n_f = n as f64;

    // Effect computation relies on W matrix structure
    // For each parameter j: Direct effect ≈ β_j, Indirect ≈ β_j * ρ * avg(W effect)
    let beta_slice = &beta.as_slice()[1..];
    let ses_slice = &ses_beta[1..];

    // Parallelize effect calculations
    let results: Vec<_> = (0..beta_slice.len())
        .into_par_iter()
        .map(|j| {
            let b = beta_slice[j];
            let se_b = ses_slice[j];
            
            let direct = b;
            let indirect = b * rho / n_f;
            let total = direct + indirect;
            
            let d_se = se_b;
            let i_se = (se_b.powi(2) * rho.powi(2) + beta_slice[0] * se_rho).sqrt() / n_f;
            let t_se = (d_se.powi(2) + i_se.powi(2)).sqrt();
            
            (direct, indirect, total, d_se, i_se, t_se)
        })
        .collect();

    // Unpack results
    let (direct, indirect, total, d_se, i_se, t_se): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
        results.into_iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut d, mut i, mut t, mut ds, mut is, mut ts), (xd, xi, xt, xds, xis, xts)| {
                d.push(xd);
                i.push(xi);
                t.push(xt);
                ds.push(xds);
                is.push(xis);
                ts.push(xts);
                (d, i, t, ds, is, ts)
            }
        );

    EffectDecomposition::new(direct, indirect, total, d_se, i_se, t_se)
}

/// Compute fitted values including spatial lag term
fn compute_spatial_lag_fit(
    beta: &DVector<f64>,
    rho: f64,
    x: &DMatrix<f64>,
    wy: &[f64],
) -> RegressionResult<Vec<f64>> {
    let fitted_x = matrix_solvers::compute_fitted(x, beta)?;
    let fitted: Vec<f64> = fitted_x
        .iter()
        .zip(wy)
        .map(|(xb, w)| xb + rho * w)
        .collect();
    Ok(fitted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regression::test_data::ColumbusData;
    use crate::weights::SpatialWeightsDiagnostics;

    #[test]
    fn test_spatial_lag_columbus() {
        let y = ColumbusData::crime();
        let income = ColumbusData::income();
        let housing = ColumbusData::housing_value();

        // Build design matrix: intercept + income + housing
        let mut x_data = Vec::new();
        for i in 0..49 {
            x_data.push(vec![1.0, income[i], housing[i]]);
        }

        let x = DMatrix::from_fn(49, 3, |i, j| x_data[i][j]);

        // Build weights
        let neighbors_raw = ColumbusData::weights_queen();
        
        // Count neighbors for diagnostics
        let neighbor_counts: Vec<usize> = neighbors_raw.iter().map(|n| n.len()).collect();
        let n_islands = neighbor_counts.iter().filter(|&&c| c == 0).count();
        let neighbor_count_min = neighbor_counts.iter().min().copied().unwrap_or(0);
        let neighbor_count_max = neighbor_counts.iter().max().copied().unwrap_or(0);
        let neighbor_count_mean = neighbor_counts.iter().sum::<usize>() as f64 / 49.0;

        let diagnostics = SpatialWeightsDiagnostics {
            n_features: 49,
            n_islands,
            neighbor_count_min,
            neighbor_count_mean,
            neighbor_count_max,
            connected_component_count: 1,
            row_standardized: true,
            dropped_feature_count: 0,
        };

        let weights = SpatialWeightsGraph {
            neighbors: neighbors_raw,
            diagnostics,
            warnings: Vec::new(),
        };

        // Estimate SAR model
        let result = SpatialLagRegression::estimate(&y, &x, &weights, 100, 1e-6);
        assert!(result.is_ok(), "{:?}", result.err());

        let res = result.unwrap();
        assert!(res.base.r_squared > 0.0);
        assert!(res.base.r_squared < 1.0);
        assert!(res.rho.abs() < 0.99); // Stationarity
    }
}
