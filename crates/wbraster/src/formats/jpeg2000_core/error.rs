//! Error types for the GeoJP2 library.

use thiserror::Error;

/// Result type alias for GeoJP2 operations.
pub type Result<T> = std::result::Result<T, Jp2Error>;

/// All errors that can occur when reading or writing JPEG 2000 / GeoJP2 files.
#[derive(Debug, Error)]
pub enum Jp2Error {
    /// Underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The file does not start with the JP2 signature box.
    #[error("Not a JPEG 2000 file: {0}")]
    NotJp2(String),

    /// A required JP2 box is missing or malformed.
    #[error("Invalid JP2 box '{box_type}': {message}")]
    InvalidBox {
        /// Four-character box type (e.g. "ftyp", "jp2h")
        box_type: String,
        /// Human-readable description of the problem
        message: String,
    },

    /// The JPEG 2000 codestream contains an unexpected or invalid marker.
    #[error("Invalid codestream at offset {offset}: {message}")]
    InvalidCodestream {
        /// Byte offset in the codestream where the error was detected
        offset: usize,
        /// Description of the problem
        message: String,
    },

    /// Image dimensions are zero or otherwise invalid.
    #[error("Invalid image dimensions: {width}×{height}×{components}")]
    InvalidDimensions {
        width:      u32,
        height:     u32,
        components: u16,
    },

    /// Requested band index is out of range.
    #[error("Component {index} out of range (image has {components} components)")]
    ComponentOutOfRange {
        index:      usize,
        components: usize,
    },

    /// The requested or detected bit depth is not supported.
    #[error("Unsupported bit depth: {0}")]
    UnsupportedBitDepth(u8),

    /// The requested or detected sample format is not supported.
    #[error("Unsupported sample format: signed={signed}, bits={bits}")]
    UnsupportedSampleFormat {
        signed: bool,
        bits:   u8,
    },

    /// The DWT or entropy coding configuration is unsupported.
    #[error("Unsupported coding parameter: {0}")]
    UnsupportedCodingParam(String),

    /// A DWT coefficient buffer is the wrong size.
    #[error("DWT buffer size mismatch: expected {expected}, got {actual}")]
    DwtSizeMismatch {
        expected: usize,
        actual:   usize,
    },

    /// Arithmetic overflow in tile / codestream calculations.
    #[error("Arithmetic overflow: {0}")]
    Overflow(String),

    /// Data buffer size does not match declared image geometry.
    #[error("Data size mismatch: expected {expected} samples, got {actual}")]
    DataSizeMismatch {
        expected: usize,
        actual:   usize,
    },

    /// The GeoJP2 UUID box contains invalid GeoTIFF metadata.
    #[error("Invalid GeoJP2 metadata: {0}")]
    InvalidGeoMetadata(String),

    /// A feature required to decode this file is not yet implemented.
    #[error("Not yet implemented: {0}")]
    NotImplemented(String),
}
