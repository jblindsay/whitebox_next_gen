//! Error types for the GeoTIFF library.

#![allow(dead_code)]

use thiserror::Error;

/// Result type alias for GeoTIFF operations.
pub type Result<T> = std::result::Result<T, GeoTiffError>;

/// Errors that can occur when reading or writing GeoTIFF files.
#[derive(Debug, Error)]
pub enum GeoTiffError {
    /// I/O error from the underlying file system.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The file is not a valid TIFF file (bad magic number or byte order).
    #[error("Invalid TIFF file: {0}")]
    InvalidTiff(String),

    /// An IFD (Image File Directory) entry is malformed.
    #[error("Invalid IFD entry: {0}")]
    InvalidIfd(String),

    /// A required TIFF tag is missing.
    #[error("Missing required tag: {tag} ({code})")]
    MissingTag {
        /// Human-readable tag name
        tag: &'static str,
        /// Numeric TIFF tag code
        code: u16,
    },

    /// A tag has an unexpected or unsupported value.
    #[error("Unsupported tag value for {tag}: {value}")]
    UnsupportedTagValue {
        /// Human-readable tag name
        tag: &'static str,
        /// The unsupported value
        value: u64,
    },

    /// Compression or decompression failed.
    #[error("Compression error ({codec}): {message}")]
    CompressionError {
        /// Name of the codec
        codec: &'static str,
        /// Description of the error
        message: String,
    },

    /// The requested compression format is not supported.
    #[error("Unsupported compression: {0}")]
    UnsupportedCompression(u16),

    /// The sample format or bit depth is not supported.
    #[error("Unsupported sample format: {bits_per_sample} bits, format={sample_format}")]
    UnsupportedSampleFormat {
        /// Bits per sample value
        bits_per_sample: u16,
        /// TIFF SampleFormat tag value
        sample_format: u16,
    },

    /// Image dimensions are zero or otherwise invalid.
    #[error("Invalid image dimensions: {width}x{height}x{bands}")]
    InvalidDimensions {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
        /// Number of bands/samples
        bands: u16,
    },

    /// Attempt to read a band index that doesn't exist.
    #[error("Band index {index} out of range (file has {bands} bands)")]
    BandOutOfRange {
        /// Requested band index
        index: usize,
        /// Available band count
        bands: usize,
    },

    /// Tile or strip data is truncated or corrupt.
    #[error("Corrupt image data at {location}: {message}")]
    CorruptData {
        /// Description of where the corruption was found
        location: String,
        /// Details of the issue
        message: String,
    },

    /// GeoKey directory is invalid or references unknown keys.
    #[error("Invalid GeoKey directory: {0}")]
    InvalidGeoKey(String),

    /// Buffer size mismatch during write.
    #[error("Data size mismatch: expected {expected} samples, got {actual}")]
    DataSizeMismatch {
        /// Expected number of samples
        expected: usize,
        /// Actual number of samples provided
        actual: usize,
    },
}
