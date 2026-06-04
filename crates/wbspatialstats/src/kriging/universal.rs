/// Universal Kriging (UK) with polynomial trend component
///
/// Universal kriging extends ordinary kriging by explicitly modeling
/// a polynomial drift/trend in addition to spatial correlation.
///
/// ## Model
///
/// $$Z(x) = f(x) + \delta(x) + \varepsilon$$
///
/// where:
/// - f(x) is the polynomial trend (linear: β₀ + β₁x₁ + β₂x₂, or quadratic)
/// - δ(x) is the spatially correlated residual
/// - ε is measurement error
///
/// ## Algorithm
///
/// 1. Fit polynomial trend to training data via least squares
/// 2. Compute residuals: r_i = z_i - f(x_i)
/// 3. Estimate variogram of residuals
/// 4. Apply ordinary kriging to residuals at prediction location
/// 5. Add trend prediction: ẑ = f(x₀) + kriging_residual(r)
///
/// ## Supported Trend Degrees
///
/// - Degree 0: Constant (equivalent to ordinary kriging)
/// - Degree 1: Linear trend (most common)
/// - Degree 2: Quadratic trend
///
/// ## References
///
/// - Wackernagel, H. (2003). *Multivariate Geostatistics*, 3rd ed.
/// - Isaaks, E.H. & Srivastava, R.M. (1989). *An Introduction to Applied Geostatistics*

use super::OrdinaryKriging;
use crate::variogram::VariogramModel;
use crate::{GeostatError, GeostatResult};
use nalgebra as na;
use std::sync::Arc;
use rayon::prelude::*;

/// Result from universal kriging prediction
#[derive(Debug, Clone)]
pub struct UniversalKrigingResult {
    /// Predicted value at location
    pub prediction: f64,
    /// Kriging variance (uncertainty)
    pub variance: f64,
    /// Standard error (sqrt of variance)
    pub std_error: f64,
    /// Lower 95% confidence interval bound
    pub ci_lower: f64,
    /// Upper 95% confidence interval bound
    pub ci_upper: f64,
}

/// Universal kriging with polynomial trend
#[derive(Debug)]
pub struct UniversalKriging {
    training_coords: Vec<(f64, f64)>,
    #[allow(dead_code)]
    training_values: Vec<f64>,
    variogram: VariogramModel,
    trend_degree: usize,
    trend_coefficients: Vec<f64>,
    residuals: Vec<f64>,
    ordinary_kriging: Arc<OrdinaryKriging>,
}

impl UniversalKriging {
    /// Create a new universal kriging model
    ///
    /// # Arguments
    ///
    /// * `training_coords` - Training point coordinates [(x₁,y₁), (x₂,y₂), ...]
    /// * `training_values` - Training point values [z₁, z₂, ...]
    /// * `variogram` - Fitted variogram model for residuals
    /// * `trend_degree` - Polynomial degree (0, 1, or 2)
    ///
    /// # Returns
    ///
    /// A new UniversalKriging instance or error if data is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// use wbspatialstats::UniversalKriging;
    /// use wbspatialstats::variogram::VariogramModel;
    ///
    /// let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5)];
    /// let values = vec![1.0, 2.5, 1.8];
    /// let vario = VariogramModel::spherical(0.1, 1.0, 5.0);
    ///
    /// let uk = UniversalKriging::new(coords, values, vario, 1)?;
    /// let result = uk.predict(0.5, 0.5)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(
        training_coords: Vec<(f64, f64)>,
        training_values: Vec<f64>,
        variogram: VariogramModel,
        trend_degree: usize,
    ) -> GeostatResult<Self> {
        // Validation
        if training_coords.len() < 4 {
            return Err(GeostatError::InsufficientData(format!(
                "Minimum 4 training points required; got {}",
                training_coords.len()
            )));
        }
        if training_coords.len() != training_values.len() {
            return Err(GeostatError::InvalidParameters(format!(
                "Coordinate/value length mismatch: {} coords, {} values",
                training_coords.len(),
                training_values.len()
            )));
        }
        if trend_degree > 2 {
            return Err(GeostatError::InvalidParameters(format!(
                "Trend degree must be 0, 1, or 2; got {}",
                trend_degree
            )));
        }

        let _n = training_coords.len();

        // Fit polynomial trend
        let trend_coefficients = Self::fit_polynomial(&training_coords, &training_values, trend_degree)?;

        // Compute residuals
        let residuals = Self::compute_residuals(
            &training_coords,
            &training_values,
            &trend_coefficients,
            trend_degree,
        );

        // Create ordinary kriging model on residuals
        let ordinary_kriging = Arc::new(OrdinaryKriging::new(
            training_coords.clone(),
            residuals.clone(),
            variogram.clone(),
        )?);

        Ok(UniversalKriging {
            training_coords,
            training_values,
            variogram,
            trend_degree,
            trend_coefficients,
            residuals,
            ordinary_kriging,
        })
    }

    /// Predict at a single location
    ///
    /// Combines polynomial trend prediction with kriging of residuals
    pub fn predict(&self, x: f64, y: f64) -> GeostatResult<UniversalKrigingResult> {
        // Predict trend
        let trend_pred =
            Self::evaluate_polynomial(&[x, y], &self.trend_coefficients, self.trend_degree);

        // Predict residuals using ordinary kriging
        let residual_prediction = self.ordinary_kriging.predict((x, y))?;

        // Combine: z = trend + residual
        let prediction = trend_pred + residual_prediction.prediction;
        let variance = residual_prediction.variance;
        let std_error = variance.sqrt();

        Ok(UniversalKrigingResult {
            prediction,
            variance,
            std_error,
            ci_lower: prediction - 1.96 * std_error,
            ci_upper: prediction + 1.96 * std_error,
        })
    }

    /// Predict at multiple locations (parallelized with rayon)
    pub fn predict_batch(
        &self,
        coords: Vec<(f64, f64)>,
    ) -> GeostatResult<Vec<UniversalKrigingResult>> {
        if coords.is_empty() {
            return Err(GeostatError::InvalidParameters(
                "Empty coordinate array".to_string(),
            ));
        }

        coords
            .par_iter()
            .map(|(x, y)| self.predict(*x, *y))
            .collect()
    }

    /// Number of training points
    pub fn n_training(&self) -> usize {
        self.training_coords.len()
    }

    /// Polynomial trend degree
    pub fn trend_degree(&self) -> usize {
        self.trend_degree
    }

    /// Trend coefficients [β₀, β₁, β₂, ...] depending on degree
    pub fn trend_coefficients(&self) -> &[f64] {
        &self.trend_coefficients
    }

    /// Underlying variogram model (for residuals)
    pub fn variogram(&self) -> &VariogramModel {
        &self.variogram
    }

    /// Residuals from trend fit
    pub fn residuals(&self) -> &[f64] {
        &self.residuals
    }

    // ---- Helper methods ----

    /// Fit polynomial trend to data using least squares
    fn fit_polynomial(
        coords: &[(f64, f64)],
        values: &[f64],
        degree: usize,
    ) -> GeostatResult<Vec<f64>> {
        let n = coords.len();
        let num_coeffs = match degree {
            0 => 1,           // β₀
            1 => 3,           // β₀, β₁, β₂
            2 => 6,           // β₀, β₁, β₂, β₁₁, β₂₂, β₁₂
            _ => return Err(GeostatError::InvalidParameters(
                "Degree must be 0, 1, or 2".to_string()
            )),
        };

        // Build design matrix X
        let mut x_data = vec![];
        for (xi, yi) in coords {
            let mut row = vec![1.0]; // intercept
            if degree >= 1 {
                row.push(*xi);
                row.push(*yi);
            }
            if degree >= 2 {
                row.push(xi * xi);
                row.push(yi * yi);
                row.push(xi * yi);
            }
            x_data.extend(row);
        }

        let x = na::DMatrix::from_row_slice(n, num_coeffs, &x_data);
        let y = na::DVector::from_vec(values.to_vec());

        // Solve X*β = y via normal equations: (X^T*X)β = X^T*y
        let xt_x = x.transpose() * &x;
        let xt_y = x.transpose() * y;

        let coeffs = match xt_x.clone().lu().solve(&xt_y) {
            Some(beta) => beta.as_slice().to_vec(),
            None => {
                // Fallback to SVD
                let svd = xt_x.svd(true, true);
                svd.solve(&xt_y, 1e-10)
                    .map_err(|_| GeostatError::KrigingSolveFailed(
                        "Singular matrix in trend fitting".to_string()
                    ))?
                    .as_slice()
                    .to_vec()
            }
        };

        Ok(coeffs)
    }

    /// Evaluate polynomial at a point
    fn evaluate_polynomial(xy: &[f64], coeffs: &[f64], degree: usize) -> f64 {
        let mut result = coeffs[0]; // β₀

        if degree >= 1 && xy.len() >= 2 {
            result += coeffs[1] * xy[0]; // β₁*x
            result += coeffs[2] * xy[1]; // β₂*y
        }

        if degree >= 2 && xy.len() >= 2 && coeffs.len() >= 6 {
            result += coeffs[3] * xy[0] * xy[0]; // β₁₁*x²
            result += coeffs[4] * xy[1] * xy[1]; // β₂₂*y²
            result += coeffs[5] * xy[0] * xy[1]; // β₁₂*xy
        }

        result
    }

    /// Compute residuals from trend fit
    fn compute_residuals(
        coords: &[(f64, f64)],
        values: &[f64],
        coeffs: &[f64],
        degree: usize,
    ) -> Vec<f64> {
        coords
            .iter()
            .zip(values.iter())
            .map(|((x, y), z)| {
                let trend = Self::evaluate_polynomial(&[*x, *y], coeffs, degree);
                z - trend
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::VariogramFitter;

    #[test]
    fn test_universal_kriging_creation() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.5, 2.0, 3.5]; // slight linear trend
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.5,
            range: 3.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1);
        assert!(uk.is_ok());
        assert_eq!(uk.unwrap().n_training(), 4);
    }

    #[test]
    fn test_universal_kriging_insufficient_points() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5)];
        let values = vec![1.0, 2.5, 2.0];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.5,
            range: 3.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1);
        assert!(uk.is_err());
    }

    #[test]
    fn test_universal_kriging_mismatch() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.5, 2.0]; // mismatch
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.5,
            range: 3.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1);
        assert!(uk.is_err());
    }

    #[test]
    fn test_universal_kriging_degree_validation() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.5, 2.0, 3.5];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.5,
            range: 3.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 3); // degree 3 invalid
        assert!(uk.is_err());
    }

    #[test]
    fn test_universal_kriging_prediction() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.0, 2.2, 3.0];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.3,
            range: 2.5,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1).unwrap();
        let result = uk.predict(0.5, 0.5);

        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.prediction.is_finite());
        assert!(res.variance >= 0.0);
        assert!(res.std_error >= 0.0);
        assert!(res.ci_lower <= res.prediction);
        assert!(res.prediction <= res.ci_upper);
    }

    #[test]
    fn test_universal_kriging_trend_removal() {
        // Linear trend: z = 1.0 + 0.5*x + 0.3*y
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let values = vec![
            1.0,
            1.0 + 0.5,
            1.0 + 0.3,
            1.0 + 0.5 + 0.3,
        ];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.01,
            range: 10.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1).unwrap();

        // Trend coefficients should be approximately [1.0, 0.5, 0.3]
        let coeffs = uk.trend_coefficients();
        assert!((coeffs[0] - 1.0).abs() < 0.01);
        assert!((coeffs[1] - 0.5).abs() < 0.01);
        assert!((coeffs[2] - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_universal_kriging_batch_prediction() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.0, 2.2, 3.0];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.3,
            range: 2.5,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1).unwrap();
        let test_coords = vec![(0.5, 0.5), (1.0, 0.0), (0.0, 1.5)];

        let results = uk.predict_batch(test_coords);
        assert!(results.is_ok());
        let res = results.unwrap();
        assert_eq!(res.len(), 3);
        for r in res {
            assert!(r.prediction.is_finite());
            assert!(r.variance >= 0.0);
        }
    }

    #[test]
    fn test_universal_kriging_zero_degree() {
        // Degree 0: constant trend (equivalent to ordinary kriging)
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.5, 1.5, 1.5, 1.5]; // constant values
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.1,
            range: 3.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 0).unwrap();
        let result = uk.predict(0.5, 0.5);

        assert!(result.is_ok());
        let res = result.unwrap();
        // Prediction should be close to mean (1.5)
        assert!((res.prediction - 1.5).abs() < 0.5);
    }

    #[test]
    fn test_universal_kriging_two_degree() {
        // Quadratic trend: z = 1.0 + 0.1*x² + 0.2*y²
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let values = vec![
            1.0,
            1.0 + 0.1,
            1.0 + 0.2,
            1.0 + 0.1 + 0.2,
        ];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.01,
            range: 10.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 2).unwrap();

        // Trend should fit well
        let result = uk.predict(0.5, 0.5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_universal_kriging_residuals() {
        let coords = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5), (1.5, 2.0)];
        let values = vec![1.0, 2.0, 2.2, 3.0];
        let vario = VariogramModel {
            family: crate::variogram::VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 0.3,
            range: 2.5,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let uk = UniversalKriging::new(coords, values, vario, 1).unwrap();
        let residuals = uk.residuals();

        assert_eq!(residuals.len(), 4);
        // Residuals should have mean close to zero (trend removed)
        let mean: f64 = residuals.iter().sum::<f64>() / residuals.len() as f64;
        assert!(mean.abs() < 0.1);
    }
}
