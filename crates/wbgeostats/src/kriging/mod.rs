//! Ordinary Kriging solver and predictor

#![allow(non_snake_case)] // Mathematical notation (A, b, U, Vt) is standard in numerical code

use crate::{GeostatError, GeostatResult};
use serde::{Deserialize, Serialize};
use nalgebra::{DMatrix, DVector};
use std::f64;

use crate::variogram::VariogramModel;

/// Ordinary Kriging prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KrigingResult {
    /// Predicted value at target location
    pub prediction: f64,
    /// Kriging variance (uncertainty)
    pub variance: f64,
    /// Standard error (sqrt of variance)
    pub std_error: f64,
    /// Lower 95% confidence interval
    pub ci_lower: f64,
    /// Upper 95% confidence interval
    pub ci_upper: f64,
}

impl KrigingResult {
    pub fn new(prediction: f64, variance: f64) -> Self {
        let std_error = variance.sqrt();
        let ci_margin = 1.96 * std_error; // 95% CI
        KrigingResult {
            prediction,
            variance,
            std_error,
            ci_lower: prediction - ci_margin,
            ci_upper: prediction + ci_margin,
        }
    }
}

/// Ordinary Kriging engine
pub struct OrdinaryKriging {
    /// Training point coordinates
    pub training_coords: Vec<(f64, f64)>,
    /// Training point values
    pub training_values: Vec<f64>,
    /// Fitted variogram model
    pub variogram: VariogramModel,
}

impl OrdinaryKriging {
    /// Create new kriging engine from training data and variogram
    pub fn new(
        training_coords: Vec<(f64, f64)>,
        training_values: Vec<f64>,
        variogram: VariogramModel,
    ) -> GeostatResult<Self> {
        if training_coords.len() != training_values.len() {
            return Err(GeostatError::InvalidParameters(
                "coordinates and values must have same length".to_string(),
            ));
        }

        if training_coords.len() < 3 {
            return Err(GeostatError::InsufficientData(
                "at least 3 training points required".to_string(),
            ));
        }

        Ok(OrdinaryKriging {
            training_coords,
            training_values,
            variogram,
        })
    }

    /// Predict at single target location
    pub fn predict(&self, target: (f64, f64)) -> GeostatResult<KrigingResult> {
        let n = self.training_coords.len();

        // Build kriging system matrix A (n+1) x (n+1)
        // Upper-left: semivariances between training points
        // Last row/col: constraint for Ordinary Kriging (sum of weights = 1)
        let mut A = DMatrix::<f64>::zeros(n + 1, n + 1);

        // Fill semivariance matrix
        for i in 0..n {
            for j in 0..n {
                let dist = Self::distance(self.training_coords[i], self.training_coords[j]);
                let gamma = self.variogram.evaluate(dist);
                A[(i, j)] = gamma;
            }
        }

        // Add Lagrange constraint: last row and column are 1 (except bottom-right corner = 0)
        for i in 0..n {
            A[(i, n)] = 1.0;
            A[(n, i)] = 1.0;
        }
        A[(n, n)] = 0.0;

        // Build right-hand side vector b (n+1)
        let mut b = DVector::<f64>::zeros(n + 1);

        // Compute semivariances from training points to target
        for i in 0..n {
            let dist = Self::distance(self.training_coords[i], target);
            let gamma = self.variogram.evaluate(dist);
            b[i] = gamma;
        }
        b[n] = 1.0; // Lagrange constraint: sum of weights = 1

        // Try to solve the system
        // Attempt 1: Regularized Cholesky
        let solution = match self.solve_regularized_cholesky(&A, &b) {
            Ok(x) => x,
            Err(_) => {
                // Fallback to SVD if Cholesky fails
                self.solve_svd(&A, &b)?
            }
        };

        // Extract kriging weights (first n elements)
        let weights: Vec<f64> = solution.iter().take(n).copied().collect();
        let lambda = solution[n]; // Lagrange multiplier

        // Compute prediction: sum of weights * training values
        let prediction: f64 = weights
            .iter()
            .zip(self.training_values.iter())
            .map(|(w, v)| w * v)
            .sum();

        // Compute kriging variance
        // σ²_OK = sum(weights_i * gamma_i) + lambda
        let mut variance = lambda;
        for i in 0..n {
            let dist = Self::distance(self.training_coords[i], target);
            let gamma = self.variogram.evaluate(dist);
            variance += weights[i] * gamma;
        }

        // Ensure non-negative variance (numerical errors can cause tiny negatives)
        variance = variance.max(0.0);

        Ok(KrigingResult::new(prediction, variance))
    }

    /// Batch predict at multiple locations (parallel with rayon)
    pub fn predict_batch(&self, targets: &[(f64, f64)]) -> GeostatResult<Vec<KrigingResult>> {
        use rayon::prelude::*;

        targets
            .par_iter()
            .map(|&t| self.predict(t))
            .collect()
    }

    /// Euclidean distance between two 2D points
    fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Solve kriging system using regularized Cholesky decomposition
    /// 
    /// Adds regularization to diagonal for numerical stability
    fn solve_regularized_cholesky(&self, A: &DMatrix<f64>, b: &DVector<f64>) -> GeostatResult<DVector<f64>> {
        let n = A.nrows();

        // Estimate regularization: 1e-10 * max diagonal value
        let max_diag = (0..n)
            .map(|i| A[(i, i)].abs())
            .fold(0.0, f64::max);

        let reg = 1e-10 * max_diag.max(1.0);

        // Create regularized matrix
        let mut A_reg = A.clone();
        for i in 0..n {
            A_reg[(i, i)] += reg;
        }

        // Compute Cholesky decomposition
        match A_reg.cholesky() {
            Some(chol) => {
                let x = chol.solve(b);
                Ok(x)
            }
            None => Err(GeostatError::KrigingSolveFailed(
                "Cholesky decomposition failed".to_string(),
            )),
        }
    }

    /// Solve kriging system using SVD (fallback for ill-conditioned systems)
    /// 
    /// Uses pseudo-inverse via SVD for robustness
    fn solve_svd(&self, A: &DMatrix<f64>, b: &DVector<f64>) -> GeostatResult<DVector<f64>> {
        use nalgebra::SVD;

        // Compute SVD
        let svd = SVD::new(A.clone(), true, true);

        // Get singular values (this is a field, not a method)
        let sigma = &svd.singular_values;
        let max_sigma = sigma[0];
        let threshold = 1e-10 * max_sigma;

        // Count non-negligible singular values
        let rank = sigma.iter().filter(|s| **s > threshold).count();

        if rank == 0 {
            return Err(GeostatError::NumericalInstability(
                "Matrix is numerically singular (all singular values below threshold)".to_string(),
            ));
        }

        // Get U and V^T from SVD
        let U = svd.u.as_ref().ok_or_else(|| GeostatError::NumericalInstability(
            "SVD U matrix not computed".to_string(),
        ))?;

        let Vt = svd.v_t.as_ref().ok_or_else(|| GeostatError::NumericalInstability(
            "SVD V^T matrix not computed".to_string(),
        ))?;

        // Build pseudo-inverse: V * Sigma^+ * U^T
        // Solve by: x = V * Sigma^+ * U^T * b
        // Which is: x = V * (Sigma^+ * (U^T * b))

        // Compute U^T * b
        let utb = U.transpose() * b;

        // Apply regularized inverse of singular values
        let mut sigma_inv_utb = DVector::<f64>::zeros(A.ncols());
        for i in 0..utb.len().min(sigma.len()) {
            if sigma[i] > threshold {
                sigma_inv_utb[i] = utb[i] / sigma[i];
            }
        }

        // Compute V * (Sigma^+ * U^T * b)
        // Since we have V^T, we need to transpose it
        let x = Vt.transpose() * sigma_inv_utb;

        Ok(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::{VariogramModel, VariogramModelFamily};

    #[test]
    fn test_kriging_result_ci_bounds() {
        let result = KrigingResult::new(10.0, 4.0);
        assert_eq!(result.prediction, 10.0);
        assert_eq!(result.variance, 4.0);
        assert_eq!(result.std_error, 2.0);
        assert!((result.ci_lower - 6.08).abs() < 0.01);
        assert!((result.ci_upper - 13.92).abs() < 0.01);
    }

    #[test]
    fn test_kriging_insufficient_data() {
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let coords = vec![(0.0, 0.0), (10.0, 10.0)];
        let values = vec![1.0, 2.0];

        let result = OrdinaryKriging::new(coords, values, vario);
        assert!(result.is_err());
    }

    #[test]
    fn test_kriging_valid_construction() {
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 0.8,
            range: 100.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        let coords = vec![(0.0, 0.0), (100.0, 0.0), (50.0, 50.0), (200.0, 200.0)];
        let values = vec![1.0, 2.5, 1.8, 4.0];

        let result = OrdinaryKriging::new(coords, values, vario);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kriging_prediction_simple() {
        // Simple linear field: z = 2*x
        let coords = vec![(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (0.0, 100.0)];
        let values = vec![0.0, 200.0, 400.0, 0.0];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.01,
            condition_number: 5.0,
        };

        let ok = OrdinaryKriging::new(coords, values, vario).unwrap();

        // Predict at (100, 0) - should be close to 200
        let result = ok.predict((100.0, 0.0)).unwrap();
        assert!(result.prediction > 0.0); // Should predict positive
        assert!(result.variance >= 0.0); // Variance must be non-negative
    }

    #[test]
    fn test_kriging_variance_positive() {
        let vario = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.05,
            partial_sill: 0.95,
            range: 150.0,
            wrss: 0.02,
            condition_number: 8.0,
        };

        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            (0.0, 100.0),
        ];
        let values = vec![1.0, 2.0, 1.5, 0.5];

        let ok = OrdinaryKriging::new(coords, values, vario).unwrap();
        let result = ok.predict((50.0, 25.0)).unwrap();

        // Kriging variance should always be non-negative
        assert!(result.variance >= 0.0);
        // Standard error should be sqrt of variance
        assert!((result.std_error - result.variance.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_kriging_batch_predict() {
        let vario = VariogramModel {
            family: VariogramModelFamily::Gaussian,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.005,
            condition_number: 6.0,
        };

        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            (50.0, -50.0),
        ];
        let values = vec![1.0, 2.0, 1.5, 1.8];

        let ok = OrdinaryKriging::new(coords, values, vario).unwrap();

        let targets = vec![(25.0, 25.0), (75.0, 0.0), (50.0, 0.0)];
        let results = ok.predict_batch(&targets).unwrap();

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.variance >= 0.0);
            assert!(result.std_error >= 0.0);
        }
    }

    #[test]
    fn test_kriging_interpolation_at_data_point() {
        // At a training point, prediction should be close to the training value
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.01,
            partial_sill: 0.99,
            range: 100.0,
            wrss: 0.001,
            condition_number: 4.0,
        };

        let coords = vec![(0.0, 0.0), (100.0, 0.0), (50.0, 50.0), (0.0, 100.0)];
        let values = vec![10.0, 20.0, 15.0, 12.0];

        let ok = OrdinaryKriging::new(coords.clone(), values.clone(), vario).unwrap();

        // Predict at first training point
        let result = ok.predict(coords[0]).unwrap();
        // Should be very close to training value (within nugget effect)
        assert!((result.prediction - values[0]).abs() < 5.0);
    }

    #[test]
    fn test_kriging_distance_function() {
        assert!((OrdinaryKriging::distance((0.0, 0.0), (3.0, 4.0)) - 5.0).abs() < 1e-10);
        assert!((OrdinaryKriging::distance((1.0, 1.0), (1.0, 1.0)) - 0.0).abs() < 1e-10);
        assert!((OrdinaryKriging::distance((0.0, 0.0), (1.0, 0.0)) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_kriging_multiple_models() {
        let coords = vec![(0.0, 0.0), (100.0, 0.0), (50.0, 50.0), (0.0, 100.0)];
        let values = vec![1.0, 2.0, 1.5, 0.8];

        // Test all three model families
        for family in [
            VariogramModelFamily::Spherical,
            VariogramModelFamily::Exponential,
            VariogramModelFamily::Gaussian,
        ] {
            let vario = VariogramModel {
                family,
                nugget: 0.1,
                partial_sill: 0.9,
                range: 100.0,
                wrss: 0.01,
                condition_number: 7.0,
            };

            let ok = OrdinaryKriging::new(coords.clone(), values.clone(), vario).unwrap();
            let result = ok.predict((50.0, 50.0)).unwrap();

            // All models should produce valid predictions
            assert!(!result.prediction.is_nan());
            assert!(!result.variance.is_nan());
            assert!(result.variance >= 0.0);
        }
    }
}
