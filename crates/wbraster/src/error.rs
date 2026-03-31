//! Error types for wbraster.

use std::fmt;
use std::io;

/// A convenient alias for `Result<T, RasterError>`.
pub type Result<T> = std::result::Result<T, RasterError>;

/// All errors that can be produced by this library.
#[derive(Debug)]
pub enum RasterError {
    /// An I/O error from the standard library.
    Io(io::Error),

    /// The file format could not be determined from the file extension or magic bytes.
    UnknownFormat(String),

    /// A required header field is missing.
    MissingField(String),

    /// A header value could not be parsed.
    ParseError {
        /// Which field failed to parse.
        field: String,
        /// The raw value that caused the failure.
        value: String,
        /// A description of what was expected.
        expected: String,
    },

    /// The raster dimensions are invalid (e.g. zero rows/cols).
    InvalidDimensions {
        /// Number of columns provided.
        cols: usize,
        /// Number of rows provided.
        rows: usize,
    },

    /// The requested pixel is outside the raster bounds.
    OutOfBounds {
        /// Requested band index.
        band: isize,
        /// Requested column index.
        col: isize,
        /// Requested row index.
        row: isize,
        /// Raster band count.
        bands: usize,
        /// Raster width in columns.
        cols: usize,
        /// Raster height in rows.
        rows: usize,
    },

    /// The data type requested is not supported by the target format.
    UnsupportedDataType(String),

    /// The file contains data that violates the format spec.
    CorruptData(String),

    /// Generic message for all other cases.
    Other(String),
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RasterError::Io(e) => write!(f, "I/O error: {e}"),
            RasterError::UnknownFormat(s) => write!(f, "unknown raster format: {s}"),
            RasterError::MissingField(s) => write!(f, "missing required header field: {s}"),
            RasterError::ParseError { field, value, expected } => {
                write!(f, "parse error in field '{field}': got '{value}', expected {expected}")
            }
            RasterError::InvalidDimensions { cols, rows } => {
                write!(f, "invalid dimensions: {cols}×{rows}")
            }
            RasterError::OutOfBounds { band, col, row, bands, cols, rows } => {
                write!(f, "pixel ({band},{col},{row}) is out of bounds ({bands}×{cols}×{rows})")
            }
            RasterError::UnsupportedDataType(s) => {
                write!(f, "data type not supported by this format: {s}")
            }
            RasterError::CorruptData(s) => write!(f, "corrupt data: {s}"),
            RasterError::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for RasterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RasterError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for RasterError {
    fn from(e: io::Error) -> Self {
        RasterError::Io(e)
    }
}

impl From<std::num::ParseIntError> for RasterError {
    fn from(e: std::num::ParseIntError) -> Self {
        RasterError::Other(format!("integer parse error: {e}"))
    }
}

impl From<std::num::ParseFloatError> for RasterError {
    fn from(e: std::num::ParseFloatError) -> Self {
        RasterError::Other(format!("float parse error: {e}"))
    }
}
