//! Residual diagnostics for point process models
//!
//! Evaluates goodness-of-fit through standardized residuals and model adequacy checks.

use serde::{Deserialize, Serialize};

/// Type of residual computed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResidualType {
    /// Raw residual: observed - predicted
    Raw,
    /// Standardized residual: (observed - predicted) / sqrt(predicted)
    Standardized,
    /// Pearson residual: (observed - predicted) / sqrt(predicted) with continuity correction
    Pearson,
}

/// Point process residuals for diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointProcessResiduals {
    /// Point coordinates
    pub locations: Vec<(f64, f64)>,
    /// Observed counts or intensities
    pub observed: Vec<f64>,
    /// Predicted counts or intensities from model
    pub predicted: Vec<f64>,
    /// Residuals (type depends on computation)
    pub residuals: Vec<f64>,
    /// Type of residuals
    pub residual_type: ResidualType,
    /// Deviance contribution per observation
    pub deviance: Vec<f64>,
    /// Total deviance
    pub total_deviance: f64,
    /// AIC information criterion
    pub aic: f64,
}

impl PointProcessResiduals {
    /// Compute residuals for observed vs predicted patterns
    pub fn compute(
        locations: Vec<(f64, f64)>,
        observed: Vec<f64>,
        predicted: Vec<f64>,
        residual_type: ResidualType,
    ) -> Result<Self, String> {
        if locations.len() != observed.len() || observed.len() != predicted.len() {
            return Err("locations, observed, and predicted must have same length".to_string());
        }

        let n = observed.len();
        let mut residuals = Vec::with_capacity(n);
        let mut deviance = Vec::with_capacity(n);

        match residual_type {
            ResidualType::Raw => {
                for i in 0..n {
                    residuals.push(observed[i] - predicted[i]);
                    deviance.push((observed[i] - predicted[i]).powi(2));
                }
            }
            ResidualType::Standardized => {
                for i in 0..n {
                    let denom = predicted[i].sqrt().max(1e-10);
                    residuals.push((observed[i] - predicted[i]) / denom);
                    deviance.push(((observed[i] - predicted[i]) / denom).powi(2));
                }
            }
            ResidualType::Pearson => {
                for i in 0..n {
                    // Poisson deviance with continuity correction
                    let denom = (predicted[i] + 0.25).sqrt();
                    residuals.push((observed[i] - predicted[i]) / denom);

                    // Poisson deviance: 2 * [O*log(O/E) - (O-E)]
                    let o = observed[i].max(1e-10);
                    let e = predicted[i].max(1e-10);
                    let dev = 2.0 * (o * (o / e).ln() - (o - e));
                    deviance.push(dev);
                }
            }
        }

        let total_deviance: f64 = deviance.iter().sum();
        let df = (n - 1).max(1) as f64;
        let aic = total_deviance + 2.0 * df;

        Ok(PointProcessResiduals {
            locations,
            observed,
            predicted,
            residuals,
            residual_type,
            deviance,
            total_deviance,
            aic,
        })
    }

    /// Check for spatial clustering in residuals
    /// Returns mean absolute residual and spatial dispersion coefficient
    pub fn residual_clustering_score(&self) -> (f64, f64) {
        let mean_abs_residual = self
            .residuals
            .iter()
            .map(|r| r.abs())
            .sum::<f64>() / self.residuals.len() as f64;

        // Compute pairwise distances and residual covariance
        let n = self.locations.len();
        let mut spatial_var = 0.0;

        for i in 0..n {
            for j in i + 1..n {
                let dx = self.locations[i].0 - self.locations[j].0;
                let dy = self.locations[i].1 - self.locations[j].1;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > 0.0 && dist < 0.5 {
                    // Residual covariance for nearby points
                    spatial_var += self.residuals[i] * self.residuals[j] / (n as f64 * n as f64);
                }
            }
        }

        (mean_abs_residual, spatial_var)
    }

    /// Model adequacy: check if residuals are consistent with model assumptions
    /// Returns (good_fit: bool, diagnostics: String)
    pub fn adequacy_check(&self) -> (bool, String) {
        let (mar, spatial_cov) = self.residual_clustering_score();
        let mean_residual = self.residuals.iter().sum::<f64>() / self.residuals.len() as f64;

        let mut diagnostics = String::new();
        let mut is_adequate = true;

        // Check 1: Mean residual should be near zero
        if mean_residual.abs() > 0.1 {
            diagnostics.push_str(&format!(
                "WARNING: Mean residual = {:.4} (should be near 0)\n",
                mean_residual
            ));
            is_adequate = false;
        }

        // Check 2: Spatial correlation in residuals suggests model misfit
        if spatial_cov.abs() > 0.05 {
            diagnostics.push_str(&format!(
                "WARNING: Spatial autocorrelation in residuals = {:.4} (suggests model misfit)\n",
                spatial_cov
            ));
            is_adequate = false;
        }

        // Check 3: Large mean absolute residual indicates poor fit
        if mar > 0.5 {
            diagnostics.push_str(&format!(
                "WARNING: Large residuals (MAR = {:.4})\n",
                mar
            ));
            is_adequate = false;
        }

        if diagnostics.is_empty() {
            diagnostics.push_str("Model adequacy: PASS (residuals appear random and centered at 0)\n");
        }

        (is_adequate, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_residual_computation_raw() {
        let locations = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let observed = vec![10.0, 15.0, 20.0];
        let predicted = vec![9.0, 16.0, 19.0];

        let result = PointProcessResiduals::compute(
            locations,
            observed,
            predicted,
            ResidualType::Raw,
        );

        assert!(result.is_ok());
        let residuals = result.unwrap();

        assert_eq!(residuals.residuals[0], 1.0);
        assert_eq!(residuals.residuals[1], -1.0);
        assert_eq!(residuals.residuals[2], 1.0);
    }

    #[test]
    fn test_residual_computation_standardized() {
        let locations = vec![(0.0, 0.0), (1.0, 1.0)];
        let observed = vec![10.0, 20.0];
        let predicted = vec![9.0, 16.0];

        let result = PointProcessResiduals::compute(
            locations,
            observed,
            predicted,
            ResidualType::Standardized,
        );

        assert!(result.is_ok());
        let residuals = result.unwrap();

        assert!(residuals.residuals[0].abs() > 0.0);
        assert!(residuals.residuals[1].abs() > 0.0);
    }

    #[test]
    fn test_residual_length_mismatch() {
        let locations = vec![(0.0, 0.0), (1.0, 1.0)];
        let observed = vec![10.0];
        let predicted = vec![9.0, 16.0];

        let result = PointProcessResiduals::compute(
            locations,
            observed,
            predicted,
            ResidualType::Raw,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_residual_clustering_score() {
        let locations = vec![
            (0.0, 0.0),
            (0.1, 0.1),
            (1.0, 1.0),
            (1.1, 1.1),
        ];
        let observed = vec![10.0, 12.0, 15.0, 14.0];
        let predicted = vec![10.0, 10.0, 15.0, 15.0];

        let residuals = PointProcessResiduals::compute(
            locations,
            observed,
            predicted,
            ResidualType::Raw,
        )
        .unwrap();

        let (mar, _spatial_cov) = residuals.residual_clustering_score();
        assert!(mar > 0.0);
    }

    #[test]
    fn test_adequacy_check() {
        let locations = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let observed = vec![10.0, 15.0, 20.0];
        let predicted = vec![10.1, 15.1, 19.9]; // Good fit

        let residuals = PointProcessResiduals::compute(
            locations,
            observed,
            predicted,
            ResidualType::Raw,
        )
        .unwrap();

        let (is_adequate, _diagnostics) = residuals.adequacy_check();
        assert!(is_adequate);
    }
}
