//! Cross-validation diagnostics for kriging

use crate::GeostatResult;
use serde::{Deserialize, Serialize};

/// Cross-validation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CVMetrics {
    /// Mean prediction error
    pub mean_error: f64,
    /// Root mean squared error
    pub rmse: f64,
    /// Mean standardized error
    pub mean_std_error: f64,
    /// Root mean squared standardized error
    pub rmsse: f64,
    /// Correlation between predicted and actual
    pub correlation: f64,
    /// Number of predictions
    pub sample_size: usize,
}

impl CVMetrics {
    pub fn summary(&self) -> String {
        format!(
            "CV: ME={:.4}, RMSE={:.4}, MSE={:.4}, RMSSE={:.4}, r={:.3}, n={}",
            self.mean_error, self.rmse, self.mean_std_error, self.rmsse, self.correlation, self.sample_size
        )
    }
}

/// Leave-One-Out Cross-Validation
pub struct LeaveOneOutCV;

impl LeaveOneOutCV {
    /// Perform LOOCV on training data (placeholder for Task 3)
    pub fn validate(
        _training_coords: &[(f64, f64)],
        _training_values: &[f64],
        _variogram: &crate::variogram::VariogramModel,
    ) -> GeostatResult<CVMetrics> {
        // Placeholder: will implement in Task 3
        Err(crate::GeostatError::KrigingSolveFailed(
            "LOOCV not yet implemented (Task 3)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
