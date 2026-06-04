//! Variogram estimation and model fitting

pub mod empirical;
pub mod model;
pub mod robust;
pub mod directional;
pub mod cross_variogram;

pub use empirical::{EmpiricalVariogram, EmpiricalVariogramBuilder};
pub use model::{VariogramModel, VariogramModelFamily, VariogramFitter};
pub use robust::{RobustVariogramFitter, RobustLossFunction};
pub use directional::{DirectionalVariogramBin, AnisotropyModel, compute_directional_variogram, fit_anisotropy};
pub use cross_variogram::{CrossVariogramBin, CrossVariogramModel, compute_cross_variogram, fit_cross_variogram_model};

use serde::{Deserialize, Serialize};
use std::fmt;

/// Direction of semivariogram (isotropic for MVP)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VariogramDirection {
    Isotropic,
    // Anisotropic(f64) // angle in degrees, deferred to Phase B+
}

/// Lag bin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagBin {
    /// Center distance of lag bin
    pub distance: f64,
    /// Semivariogram value at this lag
    pub semivariance: f64,
    /// Number of pairs in this lag bin
    pub pair_count: usize,
}

impl fmt::Display for LagBin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LagBin {{ dist={:.2}, gamma={:.4}, pairs={} }}",
            self.distance, self.semivariance, self.pair_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lag_bin_display() {
        let bin = LagBin {
            distance: 100.0,
            semivariance: 0.5,
            pair_count: 42,
        };
        let output = format!("{}", bin);
        assert!(output.contains("100.00"));
        assert!(output.contains("0.5000"));
        assert!(output.contains("42"));
    }
}
