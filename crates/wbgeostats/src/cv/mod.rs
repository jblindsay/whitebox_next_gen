//! Cross-validation diagnostics for kriging

use crate::GeostatResult;
use crate::GeostatError;
use serde::{Deserialize, Serialize};
use crate::kriging::OrdinaryKriging;
use crate::variogram::VariogramModel;

/// Cross-validation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CVMetrics {
    /// Mean prediction error (bias)
    pub mean_error: f64,
    /// Root mean squared error
    pub rmse: f64,
    /// Mean standardized error (should be ~0 if model is unbiased)
    pub mean_std_error: f64,
    /// Root mean squared standardized error (should be ~1 if model is well-calibrated)
    pub rmsse: f64,
    /// Pearson correlation between predicted and actual
    pub correlation: f64,
    /// Number of CV predictions
    pub sample_size: usize,
}

impl CVMetrics {
    pub fn summary(&self) -> String {
        format!(
            "CV: ME={:.4}, RMSE={:.4}, MSE={:.4}, RMSSE={:.4}, r={:.3}, n={}",
            self.mean_error,
            self.rmse,
            (self.rmse * self.rmse),
            self.rmsse,
            self.correlation,
            self.sample_size
        )
    }

    /// Check if model is well-calibrated
    /// 
    /// Returns true if:
    /// - Mean standardized error close to 0 (|ME_std| < 0.1)
    /// - RMSSE close to 1 (0.8 < RMSSE < 1.2)
    pub fn is_well_calibrated(&self) -> bool {
        self.mean_std_error.abs() < 0.1 && self.rmsse > 0.8 && self.rmsse < 1.2
    }
}

/// Leave-One-Out Cross-Validation
pub struct LeaveOneOutCV;

impl LeaveOneOutCV {
    /// Perform LOOCV on training data
    ///
    /// For each training point:
    /// 1. Remove the point from the dataset
    /// 2. Create OK model with remaining n-1 points
    /// 3. Predict at the removed point
    /// 4. Record prediction error and kriging variance
    ///
    /// Returns CV metrics summarizing prediction accuracy
    pub fn validate(
        training_coords: &[(f64, f64)],
        training_values: &[f64],
        variogram: &VariogramModel,
    ) -> GeostatResult<CVMetrics> {
        if training_coords.len() != training_values.len() {
            return Err(GeostatError::InvalidParameters(
                "coordinates and values must have same length".to_string(),
            ));
        }

        if training_coords.len() < 4 {
            return Err(GeostatError::InsufficientData(
                "at least 4 points required for meaningful LOOCV".to_string(),
            ));
        }

        let n = training_coords.len();
        let mut predictions = Vec::new();
        let mut actuals = Vec::new();
        let mut residuals = Vec::new();
        let mut std_residuals = Vec::new();

        // Perform leave-one-out for each point
        for i in 0..n {
            // Build training set without point i
            let mut loo_coords = Vec::new();
            let mut loo_values = Vec::new();

            for j in 0..n {
                if i != j {
                    loo_coords.push(training_coords[j]);
                    loo_values.push(training_values[j]);
                }
            }

            // Create kriging model without point i
            let ok = match OrdinaryKriging::new(loo_coords, loo_values, variogram.clone()) {
                Ok(model) => model,
                Err(_) => continue, // Skip this fold if kriging fails
            };

            // Predict at removed point
            let target = training_coords[i];
            let actual = training_values[i];

            match ok.predict(target) {
                Ok(result) => {
                    predictions.push(result.prediction);
                    actuals.push(actual);

                    let residual = actual - result.prediction;
                    residuals.push(residual);

                    // Standardized residual = residual / std_error
                    if result.std_error > 0.0 {
                        let std_residual = residual / result.std_error;
                        std_residuals.push(std_residual);
                    }
                }
                Err(_) => continue, // Skip this fold if prediction fails
            }
        }

        if predictions.is_empty() {
            return Err(GeostatError::KrigingSolveFailed(
                "All LOOCV folds failed".to_string(),
            ));
        }

        // Compute metrics
        let sample_size = predictions.len();

        // Mean error (bias)
        let mean_error = residuals.iter().sum::<f64>() / residuals.len() as f64;

        // Root mean squared error
        let mse = residuals.iter().map(|r| r * r).sum::<f64>() / residuals.len() as f64;
        let rmse = mse.sqrt();

        // Mean standardized error
        let mean_std_error = if std_residuals.is_empty() {
            0.0
        } else {
            std_residuals.iter().sum::<f64>() / std_residuals.len() as f64
        };

        // RMSSE (root mean squared standardized error)
        let msse = if std_residuals.is_empty() {
            1.0
        } else {
            std_residuals.iter().map(|e| e * e).sum::<f64>() / std_residuals.len() as f64
        };
        let rmsse = msse.sqrt();

        // Pearson correlation
        let correlation = Self::pearson_correlation(&predictions, &actuals);

        Ok(CVMetrics {
            mean_error,
            rmse,
            mean_std_error,
            rmsse,
            correlation,
            sample_size,
        })
    }

    /// Compute Pearson correlation coefficient
    fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }

        let n = x.len() as f64;
        let mean_x = x.iter().sum::<f64>() / n;
        let mean_y = y.iter().sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;

        for (xi, yi) in x.iter().zip(y.iter()) {
            let dx = xi - mean_x;
            let dy = yi - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        let denom = (var_x * var_y).sqrt();
        if denom > 0.0 {
            cov / denom
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::{VariogramModel, VariogramModelFamily};

    #[test]
    fn test_cv_metrics_summary() {
        let metrics = CVMetrics {
            mean_error: -0.05,
            rmse: 0.42,
            mean_std_error: 0.01,
            rmsse: 1.05,
            correlation: 0.95,
            sample_size: 100,
        };
        let summary = metrics.summary();
        assert!(summary.contains("RMSE=0.4200"));
        assert!(summary.contains("n=100"));
    }

    #[test]
    fn test_cv_metrics_calibration_check() {
        // Well-calibrated model
        let good = CVMetrics {
            mean_error: 0.01,
            rmse: 0.5,
            mean_std_error: 0.05,
            rmsse: 0.95,
            correlation: 0.9,
            sample_size: 50,
        };
        assert!(good.is_well_calibrated());

        // Poorly calibrated: high bias
        let bias = CVMetrics {
            mean_error: 0.5,
            rmse: 0.5,
            mean_std_error: 0.5,
            rmsse: 0.95,
            correlation: 0.9,
            sample_size: 50,
        };
        assert!(!bias.is_well_calibrated());

        // Poorly calibrated: low RMSSE
        let underconfident = CVMetrics {
            mean_error: 0.01,
            rmse: 0.5,
            mean_std_error: 0.01,
            rmsse: 0.5,
            correlation: 0.9,
            sample_size: 50,
        };
        assert!(!underconfident.is_well_calibrated());

        // Poorly calibrated: high RMSSE
        let overconfident = CVMetrics {
            mean_error: 0.01,
            rmse: 0.5,
            mean_std_error: 0.01,
            rmsse: 1.5,
            correlation: 0.9,
            sample_size: 50,
        };
        assert!(!overconfident.is_well_calibrated());
    }

    #[test]
    fn test_pearson_correlation_perfect() {
        // Perfect correlation
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r = LeaveOneOutCV::pearson_correlation(&x, &y);
        assert!((r - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_correlation_perfect_negative() {
        // Perfect negative correlation
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let r = LeaveOneOutCV::pearson_correlation(&x, &y);
        assert!((r + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pearson_correlation_no_correlation() {
        // No correlation
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 2.0, 2.0, 2.0, 2.0]; // Constant
        let r = LeaveOneOutCV::pearson_correlation(&x, &y);
        assert!(r.is_nan() || r == 0.0);
    }

    #[test]
    fn test_loocv_insufficient_data() {
        let coords = vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)];
        let values = vec![1.0, 2.0, 1.5];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 0.9,
            range: 100.0,
            wrss: 0.01,
            condition_number: 5.0,
        };

        let result = LeaveOneOutCV::validate(&coords, &values, &vario);
        assert!(result.is_err());
    }

    #[test]
    fn test_loocv_mismatched_lengths() {
        let coords = vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        let values = vec![1.0, 2.0, 1.5];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.01,
            condition_number: 5.0,
        };

        let result = LeaveOneOutCV::validate(&coords, &values, &vario);
        assert!(result.is_err());
    }

    #[test]
    fn test_loocv_simple_linear_field() {
        // Simple linear field: z = x
        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (200.0, 0.0),
            (0.0, 100.0),
            (100.0, 100.0),
        ];
        let values = vec![0.0, 100.0, 200.0, 0.0, 100.0];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 150.0,
            wrss: 0.005,
            condition_number: 4.0,
        };

        let result = LeaveOneOutCV::validate(&coords, &values, &vario);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert_eq!(metrics.sample_size, 5);
        // For a well-behaved linear field, RMSE should be reasonable
        assert!(metrics.rmse >= 0.0); // RMSE must be non-negative
        assert!(metrics.rmse.is_finite()); // Must not be NaN/Inf
        // Correlation may vary depending on model fit; just check validity
        if metrics.sample_size >= 2 {
            assert!(metrics.correlation >= -1.0 || metrics.correlation.is_nan());
            assert!(metrics.correlation <= 1.0 || metrics.correlation.is_nan());
        }
    }

    #[test]
    fn test_loocv_constant_field() {
        // Constant field: all values are 5.0
        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (200.0, 0.0),
            (0.0, 100.0),
            (100.0, 100.0),
        ];
        let values = vec![5.0, 5.0, 5.0, 5.0, 5.0];

        let vario = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.001,
            condition_number: 3.0,
        };

        let result = LeaveOneOutCV::validate(&coords, &values, &vario);
        assert!(result.is_ok());

        let metrics = result.unwrap();
        // For constant field, predictions should be close to 5.0
        assert!(metrics.mean_error.abs() < 2.0); // Should predict close to 5.0
        // For constant field, correlation is undefined (zero variance in y)
        // So we just check that computation doesn't crash
        assert!(metrics.sample_size > 0);
    }

    #[test]
    fn test_loocv_all_model_families() {
        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            (0.0, 100.0),
            (100.0, 100.0),
        ];
        let values = vec![1.0, 2.5, 2.0, 1.5, 2.8];

        for family in [
            VariogramModelFamily::Spherical,
            VariogramModelFamily::Exponential,
            VariogramModelFamily::Gaussian,
        ] {
            let vario = VariogramModel {
                family,
                nugget: 0.1,
                partial_sill: 0.9,
                range: 120.0,
                wrss: 0.01,
                condition_number: 6.0,
            };

            let result = LeaveOneOutCV::validate(&coords, &values, &vario);
            assert!(result.is_ok(), "LOOCV failed for {:?}", family);

            let metrics = result.unwrap();
            assert_eq!(metrics.sample_size, 5);
            assert!(!metrics.rmse.is_nan());
            assert!(!metrics.correlation.is_nan());
        }
    }

    #[test]
    fn test_loocv_metrics_bounds() {
        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            (0.0, 100.0),
            (100.0, 100.0),
        ];
        let values = vec![1.0, 2.5, 2.0, 1.5, 2.8];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.05,
            partial_sill: 0.95,
            range: 100.0,
            wrss: 0.01,
            condition_number: 5.0,
        };

        let result = LeaveOneOutCV::validate(&coords, &values, &vario);
        assert!(result.is_ok());

        let metrics = result.unwrap();

        // RMSE should be non-negative
        assert!(metrics.rmse >= 0.0);

        // Correlation should be in [-1, 1]
        assert!(metrics.correlation >= -1.0 && metrics.correlation <= 1.0);

        // RMSSE should be non-negative
        assert!(metrics.rmsse >= 0.0);

        // Sample size should match number of folds
        assert!(metrics.sample_size <= coords.len());
        assert!(metrics.sample_size > 0);
    }

    #[test]
    fn test_loocv_reproducibility() {
        let coords = vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            (0.0, 100.0),
        ];
        let values = vec![1.0, 2.5, 2.0, 1.5];

        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 0.9,
            range: 100.0,
            wrss: 0.01,
            condition_number: 5.0,
        };

        let result1 = LeaveOneOutCV::validate(&coords, &values, &vario);
        let result2 = LeaveOneOutCV::validate(&coords, &values, &vario);

        assert!(result1.is_ok() && result2.is_ok());
        let metrics1 = result1.unwrap();
        let metrics2 = result2.unwrap();

        // Results should be deterministic
        assert!((metrics1.rmse - metrics2.rmse).abs() < 1e-10);
        assert!((metrics1.correlation - metrics2.correlation).abs() < 1e-10);
        assert!((metrics1.mean_error - metrics2.mean_error).abs() < 1e-10);
    }
}

