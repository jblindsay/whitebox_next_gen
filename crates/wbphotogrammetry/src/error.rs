//! Error type for photogrammetry operations.

use thiserror::Error;

/// Errors that can occur during photogrammetry processing.
#[derive(Debug, Error)]
pub enum PhotogrammetryError {
    /// The image set directory does not exist or cannot be read.
    #[error("image set error: {0}")]
    ImageSet(String),

    /// Feature detection or matching failed.
    #[error("feature matching error: {0}")]
    FeatureMatching(String),

    /// Camera alignment (bundle adjustment) failed.
    #[error("camera alignment error: {0}")]
    Alignment(String),

    /// Dense surface reconstruction failed.
    #[error("dense reconstruction error: {0}")]
    DenseReconstruction(String),

    /// Orthomosaic generation failed.
    #[error("orthomosaic error: {0}")]
    Orthomosaic(String),

    /// Raster I/O error.
    #[error("raster I/O error: {0}")]
    Raster(#[from] wbraster::RasterError),

    /// Sensor-bundle discovery or parsing failed.
    #[error("bundle error: {0}")]
    Bundle(String),

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A processing stage is not yet implemented (Sprint 1 stub marker).
    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// Convenience `Result` alias.
pub type Result<T> = std::result::Result<T, PhotogrammetryError>;
