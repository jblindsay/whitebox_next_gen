// Matrix operations and linear algebra solvers for regression
//
// Provides OLS, GLS, and related operations using nalgebra
// All operations preserve numerical stability for condition numbers up to ~1e8

use nalgebra::{DMatrix, DVector, SVD};

/// Solve OLS regression: y = Xβ + ε
/// Returns coefficients β minimizing ||y - Xβ||²
pub fn ols_solve(x: &DMatrix<f64>, y: &[f64]) -> Result<DVector<f64>, String> {
    if x.nrows() != y.len() {
        return Err(format!(
            "Design matrix rows ({}) != observations ({})",
            x.nrows(),
            y.len()
        ));
    }

    let y_vec = DVector::from_row_slice(y);

    // Use SVD for numerical stability (better than X'X inversion)
    let svd = SVD::new(x.clone(), true, true);
    let u = svd.u.as_ref().ok_or("SVD decomposition failed (U matrix)")?;
    let v_t = svd.v_t.as_ref().ok_or("SVD decomposition failed (V^T matrix)")?;
    let s = &svd.singular_values;

    // Compute U'y
    let uty = u.transpose() * y_vec;

    // Solve S * c = U'y for c (diagonal system)
    let mut c = DVector::zeros(s.len());
    for i in 0..s.len() {
        if s[i] > 1e-14 {
            c[i] = uty[i] / s[i];
        }
    }

    // Beta = V * c (V = V^T^T)
    let beta = v_t.transpose() * c;
    Ok(beta)
}

/// Solve GLS regression: y = Xβ + ε with covariance structure Σ
/// Transforms to OLS: (Σ^-1/2)y = (Σ^-1/2)X β + (Σ^-1/2)ε
/// Returns coefficients β
pub fn gls_solve(
    x: &DMatrix<f64>,
    y: &[f64],
    omega_inv_sqrt: &DMatrix<f64>,
) -> Result<DVector<f64>, String> {
    if x.nrows() != y.len() {
        return Err(format!(
            "Design matrix rows ({}) != observations ({})",
            x.nrows(),
            y.len()
        ));
    }

    if omega_inv_sqrt.nrows() != y.len() || omega_inv_sqrt.ncols() != y.len() {
        return Err(format!(
            "Covariance matrix ({} x {}) incompatible with observations ({})",
            omega_inv_sqrt.nrows(),
            omega_inv_sqrt.ncols(),
            y.len()
        ));
    }

    let y_vec = DVector::from_row_slice(y);

    // Transform: X_transformed = Σ^-1/2 X, y_transformed = Σ^-1/2 y
    let x_transformed = omega_inv_sqrt * x;
    let y_transformed = omega_inv_sqrt * y_vec;

    // Solve via OLS on transformed data
    let y_slice: Vec<f64> = y_transformed.as_slice().to_vec();
    ols_solve(&x_transformed, &y_slice)
}

/// Compute fitted values: ŷ = X β
pub fn compute_fitted(x: &DMatrix<f64>, beta: &DVector<f64>) -> Result<Vec<f64>, String> {
    if x.ncols() != beta.len() {
        return Err(format!(
            "Design matrix has {} columns but {} coefficients",
            x.ncols(),
            beta.len()
        ));
    }
    let fitted = x * beta;
    Ok(fitted.as_slice().to_vec())
}

/// Compute residuals: e = y - ŷ
pub fn compute_residuals(y: &[f64], fitted: &[f64]) -> Result<Vec<f64>, String> {
    if y.len() != fitted.len() {
        return Err(format!(
            "Observations ({}) != fitted values ({})",
            y.len(),
            fitted.len()
        ));
    }
    Ok(y.iter().zip(fitted).map(|(obs, fit)| obs - fit).collect())
}

/// Compute coefficient standard errors from residuals
/// SE = sqrt(diag((X'X)^-1) * σ²)
pub fn compute_coefficient_ses(
    x: &DMatrix<f64>,
    residuals: &[f64],
) -> Result<Vec<f64>, String> {
    let n = residuals.len();
    let k = x.ncols();

    if x.nrows() != n {
        return Err(format!(
            "Design matrix rows ({}) != residuals ({})",
            x.nrows(),
            n
        ));
    }

    // Residual sum of squares
    let rss: f64 = residuals.iter().map(|e| e * e).sum();

    // Residual variance (unbiased)
    let rss_rss = if n > k { rss / (n - k) as f64 } else { 0.0 };

    // Compute (X'X)^-1 via SVD
    let svd = SVD::new(x.clone(), true, true);
    let v_t = svd.v_t.as_ref().ok_or("SVD decomposition failed")?;
    let s = &svd.singular_values;

    // Compute V * S^-2 * V' where V = V_T^T
    let v = v_t.transpose();
    let mut s_inv_sq = vec![0.0; s.len()];
    for i in 0..s.len() {
        if s[i] > 1e-14 {
            s_inv_sq[i] = 1.0 / (s[i] * s[i]);
        }
    }

    let mut xty_inv: DMatrix<f64> = DMatrix::zeros(k, k);
    for i in 0..k {
        for j in 0..k {
            for p in 0..s.len() {
                xty_inv[(i, j)] += v[(i, p)] * s_inv_sq[p] * v[(j, p)];
            }
        }
    }

    // Standard errors: sqrt(σ² * diag((X'X)^-1))
    let ses: Vec<f64> = (0..k)
        .map(|i| {
            let var: f64 = xty_inv[(i, i)] * rss_rss;
            if var >= 0.0 {
                var.sqrt()
            } else {
                0.0
            }
        })
        .collect();

    Ok(ses)
}

/// Compute model fit statistics
pub fn compute_model_stats(
    y: &[f64],
    fitted: &[f64],
    residuals: &[f64],
    n_params: usize,
) -> Result<(f64, f64, f64, f64, f64), String> {
    let n = y.len();

    if y.len() != fitted.len() || y.len() != residuals.len() {
        return Err("Input vectors must have equal length".to_string());
    }

    // Mean of y
    let y_mean = y.iter().sum::<f64>() / n as f64;

    // Sum of squares
    let tss: f64 = y.iter().map(|yi| (yi - y_mean).powi(2)).sum();
    let rss: f64 = residuals.iter().map(|e| e * e).sum();
    let ess = tss - rss; // Explained sum of squares

    // R²
    let r_squared = if tss > 0.0 { ess / tss } else { 0.0 };

    // Adjusted R²
    let r_squared_adj = if n > n_params {
        1.0 - ((1.0 - r_squared) * (n - 1) as f64 / (n - n_params) as f64)
    } else {
        0.0
    };

    // Residual variance
    let sigma_sq = if n > n_params {
        rss / (n - n_params) as f64
    } else {
        0.0
    };

    // Log-likelihood (assuming normal errors)
    let log_likelihood = -0.5 * ((n as f64 * (1.0 + (2.0 * std::f64::consts::PI).ln()))
        + (n as f64 * sigma_sq.ln())
        + rss);

    // AIC
    let aic = 2.0 * n_params as f64 - 2.0 * log_likelihood;

    Ok((r_squared, r_squared_adj, sigma_sq, log_likelihood, aic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ols_simple() {
        // Simple regression: y = 2 + 3*x + ε
        let x_vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let x = DMatrix::from_fn(5, 2, |i, j| {
            if j == 0 { 1.0 } else { x_vals[i] }
        });

        let y = vec![5.0, 8.0, 11.0, 14.0, 17.0]; // Perfect fit: y = 2 + 3*x

        let beta = ols_solve(&x, &y).unwrap();
        assert!((beta[0] - 2.0).abs() < 1e-10); // Intercept ≈ 2
        assert!((beta[1] - 3.0).abs() < 1e-10); // Slope ≈ 3
    }

    #[test]
    fn test_fitted_residuals() {
        let x = DMatrix::from_fn(3, 2, |i, j| {
            if j == 0 { 1.0 } else { (i + 1) as f64 }
        });

        let beta = DVector::from_row_slice(&[2.0, 3.0]);
        let y = vec![5.0, 8.0, 11.0];

        let fitted = compute_fitted(&x, &beta).unwrap();
        assert_eq!(fitted.len(), 3);
        assert!((fitted[0] - 5.0).abs() < 1e-10);

        let residuals = compute_residuals(&y, &fitted).unwrap();
        assert!(residuals.iter().all(|e| e.abs() < 1e-10));
    }
}
