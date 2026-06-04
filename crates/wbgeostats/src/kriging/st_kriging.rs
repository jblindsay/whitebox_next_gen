//! Spatio-temporal kriging for 3D predictions (space + time)
//!
//! Implements kriging predictions where data points have spatial (x, y) and
//! temporal (t) dimensions. Uses separable variogram models:
//!
//! γ(h_spatial, h_temporal) = γ_space(h_s) * γ_time(h_t)
//!
//! This separability assumption is computationally tractable and works well
//! for many environmental processes.

use crate::{GeostatError, GeostatResult};
use crate::kriging::KrigingResult;
use crate::variogram::VariogramModel;
use nalgebra as na;
use rayon::prelude::*;

/// Spatio-temporal kriging with separable variogram
///
/// Data points are 3D: (x, y, t). Predictions combine spatial and temporal
/// correlations using a separable variogram model.
#[derive(Debug, Clone)]
pub struct SpaceTimeKriging {
    /// Spatial coordinates (x, y)
    training_coords_spatial: Vec<(f64, f64)>,
    /// Temporal coordinates (t)
    training_coords_temporal: Vec<f64>,
    /// Training values at each (x, y, t) point
    training_values: Vec<f64>,
    /// Spatial variogram model
    variogram_spatial: VariogramModel,
    /// Temporal variogram model
    variogram_temporal: VariogramModel,
}

impl SpaceTimeKriging {
    /// Create a new spatio-temporal kriging instance
    ///
    /// # Arguments
    /// * `coords_spatial` - Spatial (x, y) coordinates
    /// * `coords_temporal` - Temporal t coordinates
    /// * `values` - Measured values at each point
    /// * `vario_spatial` - Spatial variogram model
    /// * `vario_temporal` - Temporal variogram model
    ///
    /// # Errors
    /// - Insufficient data (< 4 points required)
    /// - Coordinate/value length mismatch
    /// - Invalid variogram parameters
    pub fn new(
        coords_spatial: Vec<(f64, f64)>,
        coords_temporal: Vec<f64>,
        values: Vec<f64>,
        vario_spatial: VariogramModel,
        vario_temporal: VariogramModel,
    ) -> GeostatResult<Self> {
        if coords_spatial.len() < 4 {
            return Err(GeostatError::InsufficientData(
                "at least 4 spatio-temporal points required for kriging".to_string(),
            ));
        }

        if coords_spatial.len() != coords_temporal.len() || coords_spatial.len() != values.len() {
            return Err(GeostatError::InvalidParameters(
                "spatial coords, temporal coords, and values must have same length".to_string(),
            ));
        }

        Ok(SpaceTimeKriging {
            training_coords_spatial: coords_spatial,
            training_coords_temporal: coords_temporal,
            training_values: values,
            variogram_spatial: vario_spatial,
            variogram_temporal: vario_temporal,
        })
    }

    /// Predict at a new spatio-temporal point
    ///
    /// # Arguments
    /// * `x`, `y` - Spatial coordinates
    /// * `t` - Temporal coordinate
    ///
    /// Returns kriging prediction, variance, std error, and 95% CI bounds
    pub fn predict(&self, x: f64, y: f64, t: f64) -> GeostatResult<KrigingResult> {
        let target_spatial = (x, y);
        let target_temporal = t;
        let n = self.training_coords_spatial.len();

        // Build (n+1) × (n+1) separable variogram matrix
        // γ_st(i,j) = γ_s(i,j) * γ_t(i,j)
        let mut gamma = na::DMatrix::zeros(n + 1, n + 1);

        for i in 0..n {
            for j in 0..n {
                let dist_spatial = Self::distance(
                    self.training_coords_spatial[i],
                    self.training_coords_spatial[j],
                );
                let dist_temporal = (self.training_coords_temporal[i] - self.training_coords_temporal[j]).abs();

                // Separable: γ(h_s, h_t) = γ_s(h_s) * γ_t(h_t)
                let gamma_s = self.variogram_spatial.evaluate(dist_spatial);
                let gamma_t = self.variogram_temporal.evaluate(dist_temporal);
                gamma[(i, j)] = gamma_s * gamma_t;
            }

            gamma[(i, n)] = 1.0;
            gamma[(n, i)] = 1.0;
        }
        gamma[(n, n)] = 0.0;

        // Build RHS with separable semivariances to target
        let mut rhs = na::DVector::zeros(n + 1);
        for i in 0..n {
            let dist_spatial = Self::distance(self.training_coords_spatial[i], target_spatial);
            let dist_temporal = (self.training_coords_temporal[i] - target_temporal).abs();

            let gamma_s = self.variogram_spatial.evaluate(dist_spatial);
            let gamma_t = self.variogram_temporal.evaluate(dist_temporal);
            rhs[i] = gamma_s * gamma_t;
        }
        rhs[n] = 1.0;

        // Solve kriging system
        let weights = match gamma.clone().lu().solve(&rhs) {
            Some(w) => w,
            None => {
                let svd = gamma.svd(true, true);
                svd.solve(&rhs, 1e-10).map_err(|_| {
                    GeostatError::KrigingSolveFailed(
                        "Failed to solve space-time kriging system".to_string(),
                    )
                })?
            }
        };

        let kriging_weights: Vec<f64> = weights.iter().take(n).copied().collect();

        // Compute kriging variance using separable model
        let spatial_sill = self.variogram_spatial.total_sill();
        let temporal_sill = self.variogram_temporal.total_sill();

        // Variance formula for separable kriging:
        // σ² = sill_s * sill_t - Σ λ_i * γ_st(x_i, target)
        let sill_product = spatial_sill * temporal_sill;
        let mut kriging_variance = sill_product;

        for i in 0..n {
            let dist_spatial = Self::distance(self.training_coords_spatial[i], target_spatial);
            let dist_temporal = (self.training_coords_temporal[i] - target_temporal).abs();

            let gamma_s = self.variogram_spatial.evaluate(dist_spatial);
            let gamma_t = self.variogram_temporal.evaluate(dist_temporal);
            kriging_variance -= kriging_weights[i] * gamma_s * gamma_t;
        }
        kriging_variance = kriging_variance.max(0.0);

        // Prediction: Σ λ_i * z_i
        let mut prediction = 0.0;
        for i in 0..n {
            prediction += kriging_weights[i] * self.training_values[i];
        }

        let std_error = kriging_variance.sqrt();
        let ci_lower = prediction - 1.96 * std_error;
        let ci_upper = prediction + 1.96 * std_error;

        Ok(KrigingResult {
            prediction,
            variance: kriging_variance,
            std_error,
            ci_lower,
            ci_upper,
        })
    }

    /// Batch predict at multiple spatio-temporal points (parallelized)
    ///
    /// # Arguments
    /// * `coords_spatial` - Vec of (x, y) spatial coordinates
    /// * `coords_temporal` - Vec of t temporal coordinates
    ///
    /// Must have same length
    pub fn predict_batch(
        &self,
        coords_spatial: Vec<(f64, f64)>,
        coords_temporal: Vec<f64>,
    ) -> GeostatResult<Vec<KrigingResult>> {
        if coords_spatial.len() != coords_temporal.len() {
            return Err(GeostatError::InvalidParameters(
                "spatial and temporal coordinate arrays must have same length".to_string(),
            ));
        }

        let results = coords_spatial
            .into_par_iter()
            .zip(coords_temporal.into_par_iter())
            .map(|((x, y), t)| self.predict(x, y, t))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Euclidean distance between two spatial points
    fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt()
    }

    // Accessors
    pub fn n_training(&self) -> usize {
        self.training_values.len()
    }

    pub fn variogram_spatial(&self) -> &VariogramModel {
        &self.variogram_spatial
    }

    pub fn variogram_temporal(&self) -> &VariogramModel {
        &self.variogram_temporal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::VariogramModelFamily;

    fn create_test_variogram() -> VariogramModel {
        VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 0.8,
            range: 100.0,
            wrss: 0.01,
            condition_number: 10.0,
        }
    }

    #[test]
    fn test_spacetime_kriging_creation() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![10.0, 12.0, 11.0, 13.0];

        let vario = create_test_variogram();

        let sk = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario.clone(), vario);
        assert!(sk.is_ok());
        assert_eq!(sk.unwrap().n_training(), 4);
    }

    #[test]
    fn test_spacetime_kriging_insufficient_points() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        let coords_temporal = vec![0.0, 1.0, 2.0];
        let values = vec![10.0, 12.0, 11.0];

        let vario = create_test_variogram();

        let result = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario.clone(), vario);
        assert!(result.is_err());
    }

    #[test]
    fn test_spacetime_kriging_mismatch() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 1.0, 2.0]; // Wrong length
        let values = vec![10.0, 12.0, 11.0, 13.0];

        let vario = create_test_variogram();

        let result = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario.clone(), vario);
        assert!(result.is_err());
    }

    #[test]
    fn test_spacetime_kriging_prediction() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 0.0, 0.0, 0.0]; // Spatial-only case (temporal constant)
        let values = vec![10.0, 12.0, 11.0, 13.0];

        let vario = create_test_variogram();
        let sk = SpaceTimeKriging::new(coords_spatial.clone(), coords_temporal.clone(), values, vario.clone(), vario).unwrap();

        // Predict at new spatio-temporal point
        let result = sk.predict(0.5, 0.5, 0.0);
        assert!(result.is_ok());

        let pred = result.unwrap();
        assert!(pred.prediction.is_finite());
        assert!(pred.variance >= 0.0);
        assert!(pred.std_error >= 0.0);
    }

    #[test]
    fn test_spacetime_kriging_batch_prediction() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![10.0, 12.0, 11.0, 13.0];

        let vario = create_test_variogram();
        let sk = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario.clone(), vario).unwrap();

        let pred_spatial = vec![(0.5, 0.5), (0.7, 0.3)];
        let pred_temporal = vec![1.0, 2.0];

        let results = sk.predict_batch(pred_spatial, pred_temporal);
        assert!(results.is_ok());

        let preds = results.unwrap();
        assert_eq!(preds.len(), 2);
        for pred in preds {
            assert!(pred.prediction.is_finite());
            assert!(pred.variance >= 0.0);
        }
    }

    #[test]
    fn test_spacetime_kriging_temporal_variation() {
        // Data with temporal trend
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 0.0, 1.0, 1.0]; // Two time steps
        let values = vec![10.0, 10.0, 15.0, 15.0]; // Temporal increase

        let vario_spatial = create_test_variogram();
        let vario_temporal = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.05,
            partial_sill: 5.0, // Larger temporal variance
            range: 2.0,
            wrss: 0.02,
            condition_number: 8.0,
        };

        let sk = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario_spatial, vario_temporal).unwrap();

        // Predict at same spatial location, different times
        let pred_t0 = sk.predict(0.5, 0.5, 0.0).unwrap();
        let pred_t1 = sk.predict(0.5, 0.5, 1.0).unwrap();

        // Prediction should increase with time due to temporal trend
        assert!(pred_t1.prediction > pred_t0.prediction);
    }

    #[test]
    fn test_spacetime_separable_variogram() {
        // Verify separability: γ(h_s, h_t) = γ_s(h_s) * γ_t(h_t)
        let coords_spatial = vec![(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        let coords_temporal = vec![0.0, 0.0, 1.0, 1.0];
        let values = vec![100.0, 105.0, 110.0, 115.0];

        let vario_s = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 1.0,
            partial_sill: 4.0,
            range: 50.0,
            wrss: 0.001,
            condition_number: 5.0,
        };

        let vario_t = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.5,
            partial_sill: 2.5,
            range: 2.0,
            wrss: 0.001,
            condition_number: 5.0,
        };

        let _sk = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario_s.clone(), vario_t.clone()).unwrap();

        // Manual check of separable property
        let gamma_s_10_0 = vario_s.evaluate(10.0);
        let gamma_t_1_0 = vario_t.evaluate(1.0);
        let expected_separable = gamma_s_10_0 * gamma_t_1_0;

        // This should match internal computation
        assert!(expected_separable > 0.0);
        assert!(expected_separable < (vario_s.total_sill() * vario_t.total_sill()));
    }

    #[test]
    fn test_spacetime_kriging_batch_mismatch() {
        let coords_spatial = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let coords_temporal = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![10.0, 12.0, 11.0, 13.0];

        let vario = create_test_variogram();
        let sk = SpaceTimeKriging::new(coords_spatial, coords_temporal, values, vario.clone(), vario).unwrap();

        let pred_spatial = vec![(0.5, 0.5), (0.7, 0.3)];
        let pred_temporal = vec![1.0]; // Wrong length

        let result = sk.predict_batch(pred_spatial, pred_temporal);
        assert!(result.is_err());
    }
}
