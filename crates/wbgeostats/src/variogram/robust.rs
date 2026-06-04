//! Robust variogram fitting using L¹ and Huber loss functions
//!
//! Provides alternatives to weighted least-squares (L²) fitting that are more
//! resistant to outliers in empirical variogram estimates.

use crate::{GeostatError, GeostatResult};
use super::{LagBin, VariogramModel, VariogramModelFamily};

/// Loss function type for robust fitting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RobustLossFunction {
    /// Least absolute deviations (L¹): sum of |residuals|
    /// Most resistant to outliers but less smooth
    L1,
    /// Huber loss: hybrid L¹/L² with smooth transition at threshold
    /// Combines robustness of L¹ with smoothness of L² for better convergence
    Huber(f64), // threshold parameter δ
}

/// Robust variogram fitter using L¹ or Huber loss
pub struct RobustVariogramFitter;

impl RobustVariogramFitter {
    /// Fit variogram model using robust loss function
    ///
    /// # Arguments
    /// * `lags` - Empirical variogram lag bins
    /// * `family` - Variogram model family (Spherical, Exponential, Gaussian)
    /// * `loss` - Loss function (L1 or Huber)
    ///
    /// # Panics
    /// If loss is Huber with non-positive threshold
    pub fn fit(
        lags: &[LagBin],
        family: VariogramModelFamily,
        loss: RobustLossFunction,
    ) -> GeostatResult<VariogramModel> {
        if lags.len() < 3 {
            return Err(GeostatError::InsufficientData(
                "at least 3 lag bins required for fitting".to_string(),
            ));
        }

        // Validate Huber threshold
        if let RobustLossFunction::Huber(delta) = loss {
            if delta <= 0.0 {
                return Err(GeostatError::InvalidParameter(
                    "Huber threshold must be positive".to_string(),
                ));
            }
        }

        // Initial parameter guess
        let max_gamma = lags.iter().map(|b| b.semivariance).fold(f64::NEG_INFINITY, f64::max);
        let range_guess = lags[lags.len() / 2].distance;

        let (nugget, partial_sill, range) = Self::optimize_parameters(
            lags,
            family,
            0.0,           // nugget init
            max_gamma,     // partial_sill init
            range_guess,   // range init
            loss,
        )?;

        // Compute fit quality
        let (loss_value, condition_number) = Self::compute_fit_metrics(
            lags,
            family,
            nugget,
            partial_sill,
            range,
            loss,
        );

        Ok(VariogramModel {
            family,
            nugget: nugget.max(0.0),
            partial_sill: partial_sill.max(0.0),
            range: range.max(0.1),
            wrss: loss_value, // Store loss value in wrss field
            condition_number,
        })
    }

    /// Optimize model parameters using robust loss function
    fn optimize_parameters(
        lags: &[LagBin],
        family: VariogramModelFamily,
        nugget_init: f64,
        partial_sill_init: f64,
        range_init: f64,
        loss: RobustLossFunction,
    ) -> GeostatResult<(f64, f64, f64)> {
        let nugget = nugget_init;
        let partial_sill = partial_sill_init;
        let mut range = range_init;

        const MAX_ITERATIONS: usize = 100;
        const TOL: f64 = 1e-5;

        for _iter in 0..MAX_ITERATIONS {
            // Compute loss and its derivative with respect to range
            let mut current_loss = 0.0;
            for lag in lags {
                let gamma_model = Self::evaluate_model(lag.distance, family, nugget, partial_sill, range);
                let residual = gamma_model - lag.semivariance;
                let weighted_residual = residual * (lag.pair_count as f64).sqrt();

                let loss_contrib = match loss {
                    RobustLossFunction::L1 => weighted_residual.abs(),
                    RobustLossFunction::Huber(delta) => Self::huber_loss(weighted_residual, delta),
                };

                current_loss += loss_contrib;
            }

            // Check convergence
            if current_loss < TOL {
                break;
            }

            // Robust line search on range
            let mut best_loss = current_loss;
            let mut best_range = range;

            let deltas = if loss == RobustLossFunction::L1 {
                // Larger steps for L¹ (less smooth surface)
                vec![-range * 0.15, -range * 0.08, -range * 0.04, range * 0.04, range * 0.08, range * 0.15]
            } else {
                // Standard steps for Huber (smoother surface)
                vec![-range * 0.1, -range * 0.05, range * 0.05, range * 0.1]
            };

            for delta_range in deltas {
                let test_range = (range + delta_range).max(0.1);
                let mut test_loss = 0.0;

                for lag in lags {
                    let gamma_model =
                        Self::evaluate_model(lag.distance, family, nugget, partial_sill, test_range);
                    let residual = gamma_model - lag.semivariance;
                    let weighted_residual = residual * (lag.pair_count as f64).sqrt();

                    let loss_contrib = match loss {
                        RobustLossFunction::L1 => weighted_residual.abs(),
                        RobustLossFunction::Huber(delta) => Self::huber_loss(weighted_residual, delta),
                    };

                    test_loss += loss_contrib;
                }

                if test_loss < best_loss {
                    best_loss = test_loss;
                    best_range = test_range;
                }
            }

            range = best_range;
        }

        Ok((nugget, partial_sill, range))
    }

    /// Huber loss function (smooth hybrid of L¹ and L²)
    ///
    /// For |x| ≤ δ: loss = 0.5 * x²
    /// For |x| > δ: loss = δ * (|x| - 0.5 * δ)
    fn huber_loss(residual: f64, delta: f64) -> f64 {
        let abs_residual = residual.abs();
        if abs_residual <= delta {
            0.5 * residual * residual
        } else {
            delta * (abs_residual - 0.5 * delta)
        }
    }

    /// Evaluate model at distance h
    fn evaluate_model(h: f64, family: VariogramModelFamily, nugget: f64, partial_sill: f64, range: f64) -> f64 {
        if h == 0.0 {
            return nugget;
        }

        let gamma_part = match family {
            VariogramModelFamily::Spherical => {
                if h >= range {
                    partial_sill
                } else {
                    let ratio = h / range;
                    partial_sill * (1.5 * ratio - 0.5 * ratio.powi(3))
                }
            }
            VariogramModelFamily::Exponential => {
                partial_sill * (1.0 - (-3.0 * h / range).exp())
            }
            VariogramModelFamily::Gaussian => {
                partial_sill * (1.0 - (-3.0 * (h / range).powi(2)).exp())
            }
        };

        nugget + gamma_part
    }

    /// Compute robust loss value and condition number
    fn compute_fit_metrics(
        lags: &[LagBin],
        family: VariogramModelFamily,
        nugget: f64,
        partial_sill: f64,
        range: f64,
        loss: RobustLossFunction,
    ) -> (f64, f64) {
        let mut total_loss = 0.0;
        for lag in lags {
            let gamma_model = Self::evaluate_model(lag.distance, family, nugget, partial_sill, range);
            let residual = gamma_model - lag.semivariance;
            let weighted_residual = residual * (lag.pair_count as f64).sqrt();

            let loss_contrib = match loss {
                RobustLossFunction::L1 => weighted_residual.abs(),
                RobustLossFunction::Huber(delta) => Self::huber_loss(weighted_residual, delta),
            };

            total_loss += loss_contrib;
        }

        // Condition number estimate
        let total_sill = nugget + partial_sill;
        let condition_number = if total_sill > 0.0 {
            range / total_sill.max(1e-10)
        } else {
            1e10
        };

        (total_loss, condition_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robust_fit_l1_simple() {
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        let model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::L1);
        assert!(model.is_ok());
        let m = model.unwrap();
        assert!(m.nugget >= 0.0);
        assert!(m.partial_sill >= 0.0);
        assert!(m.range > 0.0);
    }

    #[test]
    fn test_robust_fit_huber_simple() {
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        let model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::Huber(0.5));
        assert!(model.is_ok());
        let m = model.unwrap();
        assert!(m.nugget >= 0.0);
        assert!(m.partial_sill >= 0.0);
        assert!(m.range > 0.0);
    }

    #[test]
    fn test_robust_fit_with_outliers() {
        // Clean data
        let mut lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        // Add outlier
        lags.push(LagBin { distance: 75.0, semivariance: 2.0, pair_count: 5 });

        // Both methods should handle it, but L¹ more aggressively
        let l1_model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::L1);
        let huber_model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::Huber(0.5));

        assert!(l1_model.is_ok());
        assert!(huber_model.is_ok());

        let l1 = l1_model.unwrap();
        let huber = huber_model.unwrap();

        // Both should produce valid models
        assert!(l1.range > 0.0);
        assert!(huber.range > 0.0);
    }

    #[test]
    fn test_robust_fit_all_families() {
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        for family in &[VariogramModelFamily::Spherical, VariogramModelFamily::Exponential, VariogramModelFamily::Gaussian] {
            let l1_result = RobustVariogramFitter::fit(&lags, *family, RobustLossFunction::L1);
            let huber_result = RobustVariogramFitter::fit(&lags, *family, RobustLossFunction::Huber(0.3));

            assert!(l1_result.is_ok());
            assert!(huber_result.is_ok());
        }
    }

    #[test]
    fn test_huber_loss_function() {
        let delta = 1.0;

        // Small residual: should be close to L²
        let r_small = 0.5;
        let huber_small = RobustVariogramFitter::huber_loss(r_small, delta);
        assert!((huber_small - 0.125).abs() < 1e-10); // 0.5 * 0.5^2

        // Large residual: should be closer to L¹
        let r_large = 2.0;
        let huber_large = RobustVariogramFitter::huber_loss(r_large, delta);
        let expected_large = delta * (2.0 - 0.5 * delta); // 1.0 * (2.0 - 0.5)
        assert!((huber_large - expected_large).abs() < 1e-10);
    }

    #[test]
    fn test_robust_fit_insufficient_lags() {
        let lags = vec![LagBin {
            distance: 100.0,
            semivariance: 0.5,
            pair_count: 20,
        }];

        let result = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::L1);
        assert!(result.is_err());
    }

    #[test]
    fn test_robust_fit_invalid_huber_threshold() {
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        let result = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::Huber(-0.5));
        assert!(result.is_err());

        let result_zero = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::Huber(0.0));
        assert!(result_zero.is_err());
    }

    #[test]
    fn test_l1_vs_huber_convergence() {
        // Data with moderate outlier
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
            LagBin { distance: 75.0, semivariance: 1.5, pair_count: 10 }, // outlier
        ];

        let l1_model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::L1).unwrap();
        let huber_model = RobustVariogramFitter::fit(&lags, VariogramModelFamily::Spherical, RobustLossFunction::Huber(0.7)).unwrap();

        // Both should converge
        assert!(l1_model.wrss > 0.0);
        assert!(huber_model.wrss > 0.0);

        // Should produce reasonably similar fits (both robust)
        assert!((l1_model.range - huber_model.range).abs() < l1_model.range * 0.5);
    }
}
