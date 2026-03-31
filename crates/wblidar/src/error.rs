//! Unified error type for all wblidar operations.

use std::fmt;
use std::io;

/// Alias for `std::result::Result<T, crate::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that can originate from wblidar.
#[derive(Debug)]
pub enum Error {
    /// Underlying I/O error.
    Io(io::Error),
    /// File signature / magic bytes did not match the expected format.
    InvalidSignature {
        /// Short format label (e.g. "LAS", "E57").
        format: &'static str,
        /// Raw signature bytes that were actually found.
        found: Vec<u8>,
    },
    /// A numeric field held a value outside its legal range.
    InvalidValue {
        /// Field name where validation failed.
        field: &'static str,
        /// Human-readable explanation of the invalid value.
        detail: String,
    },
    /// The combination of version + point-data-format-id is not supported.
    UnsupportedVersion {
        /// LAS major version.
        major: u8,
        /// LAS minor version.
        minor: u8,
        /// Point Data Record Format identifier.
        pdrf: u8,
    },
    /// A feature is defined in the spec but not yet implemented.
    Unimplemented(&'static str),
    /// CRC-32 mismatch (E57 and COPC integrity checks).
    CrcMismatch {
        /// CRC value stored in the file.
        expected: u32,
        /// CRC value computed from payload bytes.
        computed: u32,
    },
    /// Variable-length record or chunk had an unexpected size.
    SizeMismatch {
        /// Logical context for the size check.
        context: &'static str,
        /// Expected byte count.
        expected: usize,
        /// Actual byte count.
        actual: usize,
    },
    /// UTF-8 decode failure inside a text field.
    Utf8(std::string::FromUtf8Error),
    /// Compression / decompression error from the flate2 layer.
    Compression(String),
    /// Coordinate transformation / CRS resolution error.
    Projection(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::InvalidSignature { format, found } =>
                write!(f, "invalid {format} signature: {found:?}"),
            Error::InvalidValue { field, detail } =>
                write!(f, "invalid value for field '{field}': {detail}"),
            Error::UnsupportedVersion { major, minor, pdrf } =>
                write!(f, "unsupported LAS {major}.{minor} PDRF {pdrf}"),
            Error::Unimplemented(msg) => write!(f, "not implemented: {msg}"),
            Error::CrcMismatch { expected, computed } =>
                write!(f, "CRC-32 mismatch: expected 0x{expected:08X}, computed 0x{computed:08X}"),
            Error::SizeMismatch { context, expected, actual } =>
                write!(f, "size mismatch in '{context}': expected {expected}, got {actual}"),
            Error::Utf8(e) => write!(f, "UTF-8 error: {e}"),
            Error::Compression(msg) => write!(f, "compression error: {msg}"),
            Error::Projection(msg) => write!(f, "projection error: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Error::Io(e) => Some(e), Error::Utf8(e) => Some(e), _ => None }
    }
}

impl From<io::Error> for Error { fn from(e: io::Error) -> Self { Error::Io(e) } }
impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self { Error::Utf8(e) }
}
