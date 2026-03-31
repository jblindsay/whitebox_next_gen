//! Error types for wbtopology.

use std::fmt::{Display, Formatter};

/// Result alias for wbtopology operations.
pub type Result<T> = std::result::Result<T, TopologyError>;

/// Topology error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// Invalid geometry input.
    InvalidGeometry(String),
    /// Conversion failure from external geometry model.
    Conversion(String),
    /// I/O or format error from vector interoperability APIs.
    Io(String),
}

impl Display for TopologyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGeometry(msg) => write!(f, "invalid geometry: {msg}"),
            Self::Conversion(msg) => write!(f, "conversion error: {msg}"),
            Self::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

impl std::error::Error for TopologyError {}
