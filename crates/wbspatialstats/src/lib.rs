//! `wbspatialstats`: Unified spatial statistics library
//!
//! Provides kriging, variography, spatial autocorrelation, and spatial regression tools
//! for interpolation, analysis, and modeling of spatially correlated data.
//!
//! # Modules
//!
//! - `variogram`: Empirical and modeled semivariograms (geostatistics)
//! - `kriging`: Ordinary, Local, Simple, Universal, and Spatio-Temporal Kriging predictions
//! - `cv`: Cross-validation diagnostics and metrics
//! - `weights`: Spatial weights matrices and neighborhood construction (shared by Phase A & C)
//! - `autocorrelation`: Global and local spatial autocorrelation measures (Phase A)
//! - `python`: Python bindings via PyO3 (requires python feature)

pub mod variogram;
pub mod kriging;
pub mod cv;
pub mod weights;
pub mod autocorrelation;

// Re-export key types for convenience
pub use kriging::{OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, UniversalKriging, SpaceTimeKriging, KrigingResult};
pub use variogram::{VariogramModel, VariogramModelFamily, VariogramFitter, RobustVariogramFitter, RobustLossFunction};
pub use weights::{SpatialWeightsGraph, SpatialWeightsMode, IslandPolicy, SpatialWeightsDiagnostics};

#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "r")]
pub mod r;

use thiserror::Error;

/// Geostatistics library error type
#[derive(Error, Debug)]
pub enum GeostatError {
    #[error("Invalid variogram: {0}")]
    InvalidVariogram(String),

    #[error("Kriging solve failed: {0}")]
    KrigingSolveFailed(String),

    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

pub type GeostatResult<T> = Result<T, GeostatError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_propagation() {
        let err: GeostatResult<()> = Err(GeostatError::InvalidVariogram(
            "test".to_string(),
        ));
        assert!(err.is_err());
    }
}
