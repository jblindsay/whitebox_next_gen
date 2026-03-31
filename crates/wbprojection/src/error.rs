//! Error types for the geoproject library.

use thiserror::Error;

/// A specialized [`Result`] type for projection operations.
pub type Result<T> = std::result::Result<T, ProjectionError>;

/// Errors that can occur during projection operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProjectionError {
    /// The input coordinate is outside the valid domain of the projection.
    #[error("coordinate out of bounds: {0}")]
    OutOfBounds(String),

    /// A required parameter is missing or invalid.
    #[error("invalid parameter '{param}': {reason}")]
    InvalidParameter {
        /// Name of the invalid parameter.
        param: String,
        /// Reason why the parameter is invalid.
        reason: String,
    },

    /// The inverse projection did not converge.
    #[error("inverse projection failed to converge after {iterations} iterations")]
    ConvergenceFailure {
        /// Number of iterations attempted before aborting.
        iterations: usize,
    },

    /// The coordinate is at a singular point of the projection (e.g. a pole).
    #[error("singular point: {0}")]
    SingularPoint(String),

    /// The datum transformation failed.
    #[error("datum transformation error: {0}")]
    DatumError(String),

    /// An unsupported projection was requested.
    #[error("unsupported projection: {0}")]
    UnsupportedProjection(String),
}

impl ProjectionError {
    /// Convenience constructor for out-of-bounds errors.
    pub fn out_of_bounds(msg: impl Into<String>) -> Self {
        ProjectionError::OutOfBounds(msg.into())
    }

    /// Convenience constructor for invalid parameter errors.
    pub fn invalid_param(param: impl Into<String>, reason: impl Into<String>) -> Self {
        ProjectionError::InvalidParameter {
            param: param.into(),
            reason: reason.into(),
        }
    }
}
