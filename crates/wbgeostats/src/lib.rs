//! `wbgeostats`: Production-grade geostatistics library
//!
//! Provides kriging, variography, and spatial inference tools for interpolation
//! and analysis of spatially correlated data.
//!
//! # Modules
//!
//! - `variogram`: Empirical and modeled semivariograms
//! - `kriging`: Ordinary and Local Kriging predictions and variance estimation
//! - `cv`: Cross-validation diagnostics and metrics
//! - `python`: Python bindings via PyO3 (requires python feature)

pub mod variogram;
pub mod kriging;
pub mod cv;

// Re-export key types for convenience
pub use kriging::{OrdinaryKriging, LocalOrdinaryKriging, KrigingResult};

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
