//! Variogram model fitting (Spherical, Exponential, Gaussian)

use crate::{GeostatError, GeostatResult};
use serde::{Deserialize, Serialize};

use super::LagBin;
use super::robust::{RobustVariogramFitter, RobustLossFunction};

/// Supported variogram model families
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VariogramModelFamily {
    Spherical,
    Exponential,
    Gaussian,
}

impl std::fmt::Display for VariogramModelFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariogramModelFamily::Spherical => write!(f, "Spherical"),
            VariogramModelFamily::Exponential => write!(f, "Exponential"),
            VariogramModelFamily::Gaussian => write!(f, "Gaussian"),
        }
    }
}

/// Fitted variogram model with nugget, sill, range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariogramModel {
    pub family: VariogramModelFamily,
    /// Nugget effect (variance at distance 0)
    pub nugget: f64,
    /// Partial sill (total sill - nugget)
    pub partial_sill: f64,
    /// Range parameter (practical range)
    pub range: f64,
    /// Model fit quality (weighted residual sum of squares)
    pub wrss: f64,
    /// Condition number of fit system
    pub condition_number: f64,
}

impl VariogramModel {
    /// Evaluate variogram at distance h
    pub fn evaluate(&self, h: f64) -> f64 {
        if h == 0.0 {
            return self.nugget;
        }

        let gamma_part = match self.family {
            VariogramModelFamily::Spherical => {
                if h >= self.range {
                    self.partial_sill
                } else {
                    let ratio = h / self.range;
                    self.partial_sill * (1.5 * ratio - 0.5 * ratio.powi(3))
                }
            }
            VariogramModelFamily::Exponential => {
                self.partial_sill * (1.0 - (-3.0 * h / self.range).exp())
            }
            VariogramModelFamily::Gaussian => {
                self.partial_sill * (1.0 - (-3.0 * (h / self.range).powi(2)).exp())
            }
        };

        self.nugget + gamma_part
    }

    /// Total sill (nugget + partial sill)
    pub fn total_sill(&self) -> f64 {
        self.nugget + self.partial_sill
    }

    pub fn summary(&self) -> String {
        format!(
            "{} model: nugget={:.4}, sill={:.4}, range={:.2}, wrss={:.6}, κ={:.2e}",
            self.family,
            self.nugget,
            self.total_sill(),
            self.range,
            self.wrss,
            self.condition_number
        )
    }
}

/// Variogram model fitter
pub struct VariogramFitter;

impl VariogramFitter {
    /// Fit variogram model to empirical lags using weighted least-squares
    ///
    /// Minimizes: Σ w_i * (model(h_i) - empirical(h_i))^2
    /// where w_i = pair_count_i
    pub fn fit(
        lags: &[LagBin],
        family: VariogramModelFamily,
    ) -> GeostatResult<VariogramModel> {
        if lags.len() < 3 {
            return Err(GeostatError::InsufficientData(
                "at least 3 lag bins required for fitting".to_string(),
            ));
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
        )?;

        // Compute fit quality
        let (wrss, condition_number) = Self::compute_fit_metrics(lags, family, nugget, partial_sill, range);

        Ok(VariogramModel {
            family,
            nugget: nugget.max(0.0), // Ensure non-negative nugget
            partial_sill: partial_sill.max(0.0), // Ensure non-negative partial sill
            range: range.max(0.1), // Ensure positive range
            wrss,
            condition_number,
        })
    }

    /// Fit variogram model using L¹ loss (robust to outliers)
    ///
    /// Minimizes: Σ w_i * |model(h_i) - empirical(h_i)|
    /// where w_i = sqrt(pair_count_i)
    pub fn fit_l1(
        lags: &[LagBin],
        family: VariogramModelFamily,
    ) -> GeostatResult<VariogramModel> {
        RobustVariogramFitter::fit(lags, family, RobustLossFunction::L1)
    }

    /// Fit variogram model using Huber loss (smooth robust alternative)
    ///
    /// Combines L² for small residuals and L¹ for large residuals with smooth transition.
    /// Delta parameter controls the transition threshold (default 0.5).
    ///
    /// # Arguments
    /// * `delta` - Huber loss transition threshold (must be > 0)
    pub fn fit_huber(
        lags: &[LagBin],
        family: VariogramModelFamily,
        delta: f64,
    ) -> GeostatResult<VariogramModel> {
        RobustVariogramFitter::fit(lags, family, RobustLossFunction::Huber(delta))
    }

    /// Optimize model parameters (simplified Gauss-Newton iteration)
    fn optimize_parameters(
        lags: &[LagBin],
        family: VariogramModelFamily,
        nugget_init: f64,
        partial_sill_init: f64,
        range_init: f64,
    ) -> GeostatResult<(f64, f64, f64)> {
        let nugget = nugget_init;
        let partial_sill = partial_sill_init;
        let mut range = range_init;

        const MAX_ITERATIONS: usize = 50;
        const TOL: f64 = 1e-6;

        for _iter in 0..MAX_ITERATIONS {
            // Compute weighted least-squares residuals
            let mut residuals = Vec::new();
            let mut weights = Vec::new();

            for lag in lags {
                let gamma_model = Self::evaluate_model(lag.distance, family, nugget, partial_sill, range);
                let residual = gamma_model - lag.semivariance;
                let weight = (lag.pair_count as f64).sqrt(); // sqrt for weighted LS

                residuals.push(residual);
                weights.push(weight);
            }

            // Check convergence
            let wrss: f64 = residuals.iter().zip(&weights).map(|(r, w)| r * r * w * w).sum();
            if wrss < TOL {
                break;
            }

            // Simple line search on range (most sensitive parameter)
            let mut best_wrss = wrss;
            let mut best_range = range;

            for delta_range in &[-range * 0.1, -range * 0.05, range * 0.05, range * 0.1] {
                let test_range = (range + delta_range).max(0.1);
                let test_wrss: f64 = lags
                    .iter()
                    .map(|lag| {
                        let gamma_model =
                            Self::evaluate_model(lag.distance, family, nugget, partial_sill, test_range);
                        let residual = gamma_model - lag.semivariance;
                        residual * residual * (lag.pair_count as f64)
                    })
                    .sum();

                if test_wrss < best_wrss {
                    best_wrss = test_wrss;
                    best_range = test_range;
                }
            }

            range = best_range;
        }

        Ok((nugget, partial_sill, range))
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

    /// Compute weighted residual sum of squares and condition number estimate
    fn compute_fit_metrics(
        lags: &[LagBin],
        family: VariogramModelFamily,
        nugget: f64,
        partial_sill: f64,
        range: f64,
    ) -> (f64, f64) {
        let mut wrss = 0.0;
        for lag in lags {
            let gamma_model = Self::evaluate_model(lag.distance, family, nugget, partial_sill, range);
            let residual = gamma_model - lag.semivariance;
            wrss += residual * residual * (lag.pair_count as f64);
        }

        // Rough condition number estimate (range/nugget+partial_sill)
        let total_sill = nugget + partial_sill;
        let condition_number = if total_sill > 0.0 {
            range / total_sill.max(1e-10)
        } else {
            1e10
        };

        (wrss, condition_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variogram_model_spherical() {
        let model = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 0.8,
            range: 100.0,
            wrss: 0.01,
            condition_number: 10.0,
        };

        assert_eq!(model.evaluate(0.0), 0.1); // At origin = nugget
        assert!(model.evaluate(100.0) > 0.8); // At range ≈ sill
        assert_eq!(model.evaluate(200.0), model.total_sill()); // Beyond range = sill
    }

    #[test]
    fn test_variogram_model_exponential() {
        let model = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.05,
            partial_sill: 0.9,
            range: 100.0,
            wrss: 0.01,
            condition_number: 20.0,
        };

        assert_eq!(model.evaluate(0.0), 0.05);
        assert!(model.evaluate(100.0) < model.total_sill());
        assert!(model.evaluate(300.0) > model.evaluate(100.0)); // Monotonic
    }

    #[test]
    fn test_variogram_model_gaussian() {
        let model = VariogramModel {
            family: VariogramModelFamily::Gaussian,
            nugget: 0.0,
            partial_sill: 1.0,
            range: 100.0,
            wrss: 0.005,
            condition_number: 15.0,
        };

        assert_eq!(model.evaluate(0.0), 0.0);
        assert!(model.evaluate(50.0) < model.evaluate(100.0)); // Monotonic
    }

    #[test]
    fn test_variogram_fit_insufficient_lags() {
        let lags = vec![LagBin {
            distance: 100.0,
            semivariance: 0.5,
            pair_count: 20,
        }];

        let result = VariogramFitter::fit(&lags, VariogramModelFamily::Spherical);
        assert!(result.is_err());
    }

    #[test]
    fn test_variogram_fit_simple() {
        let lags = vec![
            LagBin { distance: 50.0, semivariance: 0.3, pair_count: 50 },
            LagBin { distance: 100.0, semivariance: 0.6, pair_count: 45 },
            LagBin { distance: 150.0, semivariance: 0.85, pair_count: 40 },
        ];

        let model = VariogramFitter::fit(&lags, VariogramModelFamily::Spherical);
        assert!(model.is_ok());
        let m = model.unwrap();
        assert!(m.nugget >= 0.0);
        assert!(m.partial_sill >= 0.0);
        assert!(m.range > 0.0);
    }
}
