//! Ordinary Kriging solver and predictor

use crate::{GeostatError, GeostatResult};
use serde::{Deserialize, Serialize};

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
    pub fn predict(&self, _target: (f64, f64)) -> GeostatResult<KrigingResult> {
        // Placeholder: will implement in Task 2
        // This will involve:
        // 1. Computing semivariances from training points to target
        // 2. Building kriging system matrix (n+1)x(n+1)
        // 3. Solving with regularized Cholesky
        // 4. Computing weights and prediction
        // 5. Computing kriging variance

        Err(GeostatError::KrigingSolveFailed(
            "OK solver not yet implemented (Task 2)".to_string(),
        ))
    }

    /// Batch predict at multiple locations (parallel with rayon)
    pub fn predict_batch(&self, targets: &[(f64, f64)]) -> GeostatResult<Vec<KrigingResult>> {
        // Placeholder: will parallelize in Task 2
        let results: GeostatResult<Vec<_>> = targets.iter().map(|t| self.predict(*t)).collect();
        results
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
}
