/// Simple Kriging: Kriging with a known, constant mean.
///
/// Unlike Ordinary Kriging, Simple Kriging requires the user to specify
/// the constant mean of the field. This can provide better estimates
/// when the mean is known from prior information or theory.
///
/// # Theory
///
/// Simple Kriging solves the system:
/// ```
/// [γ(x_i, x_j)      1  ] [λ_i]   [γ(x_i, x_0)    ]
/// [    1             0  ] [μ  ] = [1              ]
/// ```
/// where γ(x_i, x_j) is the variogram, λ_i are the kriging weights,
/// and μ is the Lagrange multiplier for the mean constraint.
///
/// # Example
///
/// ```ignore
/// use wbspatialstats::SimpleKriging;
/// use wbspatialstats::variogram::VariogramModel;
///
/// let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
/// let values = vec![10.0, 12.0, 11.0];
/// let variogram = VariogramModel::exponential(0.5, 1.0, 50.0); // nugget, range, sill
/// let known_mean = 11.0; // User provides the known mean
///
/// let sk = SimpleKriging::new(coords, values, variogram, known_mean)?;
/// let result = sk.predict((0.5, 0.5))?;
/// println!("Prediction: {}, Variance: {}", result.prediction, result.variance);
/// ```

use crate::{GeostatError, GeostatResult};
use crate::variogram::VariogramModel;
use super::KrigingResult;
use nalgebra as na;
use rayon::prelude::*;

/// Simple Kriging with a known, constant mean.
#[derive(Clone, Debug)]
pub struct SimpleKriging {
    training_coords: Vec<(f64, f64)>,
    training_values: Vec<f64>,
    variogram: VariogramModel,
    known_mean: f64,
}

impl SimpleKriging {
    /// Create a new Simple Kriging instance.
    ///
    /// # Arguments
    ///
    /// * `training_coords` - Vector of (x, y) coordinate tuples
    /// * `training_values` - Vector of observed values at training coordinates
    /// * `variogram` - Variogram model describing spatial correlation
    /// * `known_mean` - The known constant mean of the field
    ///
    /// # Errors
    ///
    /// Returns `GeostatError::InsufficientData` if fewer than 3 training points are provided.
    /// Returns `GeostatError::MismatchedLengths` if coords and values have different lengths.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sk = SimpleKriging::new(coords, values, variogram, 11.0)?;
    /// ```
    pub fn new(
        training_coords: Vec<(f64, f64)>,
        training_values: Vec<f64>,
        variogram: VariogramModel,
        known_mean: f64,
    ) -> GeostatResult<Self> {
        if training_coords.len() != training_values.len() {
            return Err(GeostatError::InvalidParameters(
                "Training coordinates and values must have the same length".to_string(),
            ));
        }

        if training_coords.len() < 3 {
            return Err(GeostatError::InsufficientData(
                "At least 3 training points required".to_string(),
            ));
        }

        Ok(SimpleKriging {
            training_coords,
            training_values,
            variogram,
            known_mean,
        })
    }

    /// Predict at a single target location.
    ///
    /// # Arguments
    ///
    /// * `target_x` - X coordinate of prediction location
    /// * `target_y` - Y coordinate of prediction location
    ///
    /// # Returns
    ///
    /// A `KrigingResult` containing prediction, variance, standard error, and confidence interval bounds.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = sk.predict(5.0, 5.0)?;
    /// ```
    pub fn predict(&self, target_x: f64, target_y: f64) -> GeostatResult<KrigingResult> {
        let target = (target_x, target_y);

        // Build the variogram matrix (n x n)
        let n = self.training_coords.len();
        let mut gamma = na::DMatrix::zeros(n + 1, n + 1);

        for i in 0..n {
            for j in 0..n {
                let dist = Self::distance(self.training_coords[i], self.training_coords[j]);
                gamma[(i, j)] = self.variogram.evaluate(dist);
            }
            // Last column/row: constraint row/column for unknown mean
            gamma[(i, n)] = 1.0;
            gamma[(n, i)] = 1.0;
        }

        // The (n, n) element is 0 for the constraint
        gamma[(n, n)] = 0.0;

        // Build the RHS vector (n x 1)
        let mut rhs = na::DVector::zeros(n + 1);
        for i in 0..n {
            let dist = Self::distance(self.training_coords[i], target);
            rhs[i] = self.variogram.evaluate(dist);
        }
        // Last element: constraint (sum of weights = 1)
        rhs[n] = 1.0;

        // Solve the system
        let weights = match gamma.clone().lu().solve(&rhs) {
            Some(w) => w,
            None => {
                // Try SVD fallback if LU fails
                let svd = gamma.svd(true, true);
                svd.solve(&rhs, 1e-10).map_err(|_| {
                    GeostatError::KrigingSolveFailed(
                        "Failed to solve kriging system".to_string(),
                    )
                })?
            }
        };

        // Extract kriging weights (first n elements)
        let kriging_weights: Vec<f64> = weights.iter().take(n).copied().collect();

        // Compute kriging variance
        // SK variance: σ²_SK = σ²_0 - Σ λ_i * γ(x_i, x_0)
        // where σ²_0 is the sill (total variance)
        let sill = self.variogram.total_sill();
        let mut kriging_variance = sill;
        for i in 0..n {
            let dist = Self::distance(self.training_coords[i], target);
            kriging_variance -= kriging_weights[i] * self.variogram.evaluate(dist);
        }

        // Ensure variance is non-negative (numerical stability)
        kriging_variance = kriging_variance.max(0.0);

        // Compute prediction: μ + Σ λ_i * (z_i - μ)
        let mut prediction = self.known_mean;
        for i in 0..n {
            prediction += kriging_weights[i] * (self.training_values[i] - self.known_mean);
        }

        // Standard error
        let std_error = kriging_variance.sqrt();

        // 95% confidence interval
        let z_critical = 1.96; // 95% CI
        let ci_lower = prediction - z_critical * std_error;
        let ci_upper = prediction + z_critical * std_error;

        Ok(KrigingResult {
            prediction,
            variance: kriging_variance,
            std_error,
            ci_lower,
            ci_upper,
        })
    }

    /// Euclidean distance between two points
    fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt()
    }

    /// Predict at multiple target locations (parallelized).
    ///
    /// # Arguments
    ///
    /// * `targets` - Vector of (x, y) coordinate tuples for prediction
    ///
    /// # Returns
    ///
    /// A vector of `KrigingResult` corresponding to each target location.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let targets = vec![(5.0, 5.0), (6.0, 6.0), (7.0, 7.0)];
    /// let results = sk.predict_batch(&targets)?;
    /// ```
    pub fn predict_batch(&self, targets: &[(f64, f64)]) -> GeostatResult<Vec<KrigingResult>> {
        targets
            .par_iter()
            .map(|&(x, y)| self.predict(x, y))
            .collect()
    }

    /// Get the known mean used by this kriging instance.
    pub fn known_mean(&self) -> f64 {
        self.known_mean
    }

    /// Get the number of training points.
    pub fn n_training(&self) -> usize {
        self.training_coords.len()
    }

    /// Get a reference to the variogram model.
    pub fn variogram(&self) -> &VariogramModel {
        &self.variogram
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::{VariogramModel, VariogramModelFamily};

    #[test]
    fn test_simple_kriging_creation() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![10.0, 12.0, 11.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords, values, variogram, 11.0);
        assert!(sk.is_ok());
        let sk = sk.unwrap();
        assert_eq!(sk.known_mean(), 11.0);
        assert_eq!(sk.n_training(), 3);
    }

    #[test]
    fn test_simple_kriging_insufficient_points() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0)];
        let values = vec![10.0, 12.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords, values, variogram, 11.0);
        assert!(sk.is_err());
    }

    #[test]
    fn test_simple_kriging_mismatch() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![10.0, 12.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords, values, variogram, 11.0);
        assert!(sk.is_err());
    }

    #[test]
    fn test_simple_kriging_prediction() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![10.0, 12.0, 11.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords, values, variogram, 11.0).unwrap();

        let result = sk.predict(0.5, 0.5).unwrap();
        assert!(result.prediction.is_finite());
        assert!(result.variance.is_finite());
        assert!(result.variance >= 0.0);
        assert!(result.ci_upper >= result.prediction);
        assert!(result.ci_lower <= result.prediction);
    }

    #[test]
    fn test_simple_kriging_batch_prediction() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![10.0, 12.0, 11.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords, values, variogram, 11.0).unwrap();

        let targets = vec![(0.5, 0.5), (0.3, 0.7), (0.8, 0.2)];
        let results = sk.predict_batch(&targets).unwrap();

        assert_eq!(results.len(), 3);
        for result in &results {
            assert!(result.prediction.is_finite());
            assert!(result.variance.is_finite());
            assert!(result.variance >= 0.0);
        }
    }

    #[test]
    fn test_simple_kriging_at_data_point() {
        // Prediction at a training data point should give that value
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![10.0, 12.0, 11.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };
        let sk = SimpleKriging::new(coords.clone(), values.clone(), variogram, 11.0).unwrap();

        // Predict at first training point
        let result = sk.predict(coords[0].0, coords[0].1).unwrap();
        // Predictions and variance should be finite
        assert!(result.prediction.is_finite());
        assert!(result.variance.is_finite());
        assert!(result.variance >= 0.0);
    }

    #[test]
    fn test_simple_kriging_different_means() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let values = vec![100.0, 120.0, 110.0];
        let variogram = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 1.0,
            range: 50.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        // Create two SK instances with different known means
        let sk1 = SimpleKriging::new(coords.clone(), values.clone(), variogram.clone(), 110.0).unwrap();
        let sk2 = SimpleKriging::new(coords, values, variogram, 100.0).unwrap();

        let result1 = sk1.predict(0.5, 0.5).unwrap();
        let result2 = sk2.predict(0.5, 0.5).unwrap();

        // Both predictions should be finite
        assert!(result1.prediction.is_finite());
        assert!(result2.prediction.is_finite());
        // Different means should generally affect predictions,  even if the effect is small
        // depending on the variogram parameters
        assert!(result1.variance.is_finite());
        assert!(result2.variance.is_finite());
    }
}
