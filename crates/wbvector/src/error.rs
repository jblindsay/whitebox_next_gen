//! Unified error type for all wbvector format drivers.

use thiserror::Error;

/// Standard result type used by `wbvector` APIs.
pub type Result<T> = std::result::Result<T, GeoError>;

/// Top-level error enum for vector I/O, parsing, and conversion operations.
#[derive(Debug, Error)]
pub enum GeoError {
    /// Wrapped filesystem or stream I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Unknown format or unsupported extension/content.
    #[error("Unknown or unsupported vector format: {0}")]
    UnknownFormat(String),

    // Shapefile
    #[error("Not a valid Shapefile: {0}")]
    /// File failed Shapefile header/structure validation.
    NotShapefile(String),
    #[error("Unsupported shape type code: {0}")]
    /// Encountered an unsupported Shapefile shape type code.
    UnsupportedShapeType(i32),
    #[error("Invalid DBF file: {0}")]
    /// DBF sidecar table failed parsing/validation.
    InvalidDbf(String),

    // GeoJSON
    #[error("GeoJSON parse error at offset {offset}: {msg}")]
    /// GeoJSON parse error with byte offset and parser detail.
    GeoJsonParse {
        /// Byte offset near where parsing failed.
        offset: usize,
        /// Human-readable parse failure message.
        msg: String,
    },
    #[error("GeoJSON missing required field '{0}'")]
    /// Required GeoJSON property or object member is missing.
    GeoJsonMissing(String),
    #[error("Unknown GeoJSON type '{0}'")]
    /// Unknown GeoJSON `type` token.
    GeoJsonType(String),

    // GML
    #[error("GML parse error at offset {offset}: {msg}")]
    /// GML parse error with byte offset and parser detail.
    GmlParse {
        /// Byte offset near where parsing failed.
        offset: usize,
        /// Human-readable parse failure message.
        msg: String,
    },

    // GPX
    #[error("GPX parse error at offset {offset}: {msg}")]
    /// GPX parse error with byte offset and parser detail.
    GpxParse {
        /// Byte offset near where parsing failed.
        offset: usize,
        /// Human-readable parse failure message.
        msg: String,
    },

    // KMZ
    #[error("KMZ error: {0}")]
    /// KMZ container or KML payload processing error.
    Kmz(String),

    // MapInfo MIF/MID
    #[error("MIF/MID parse error at line {line}: {msg}")]
    /// MapInfo MIF/MID parse error with line number and detail.
    MifParse {
        /// 1-based line number near where parsing failed.
        line: usize,
        /// Human-readable parse failure message.
        msg: String,
    },

    // OSM PBF
    #[error("OSM PBF error: {0}")]
    /// OSM PBF decoding/validation error.
    OsmPbf(String),

    // GeoParquet
    #[error("GeoParquet error: {0}")]
    /// GeoParquet read/write or schema conversion error.
    GeoParquet(String),

    // FlatGeobuf
    #[error("Not a valid FlatGeobuf file: {0}")]
    /// File failed FlatGeobuf signature/structure validation.
    NotFlatGeobuf(String),
    #[error("Invalid FlatGeobuf feature {index}: {msg}")]
    /// FlatGeobuf feature-level decoding/validation error.
    InvalidFgbFeature {
        /// Zero-based feature index.
        index: usize,
        /// Human-readable decode/validation message.
        msg: String,
    },

    // GeoPackage / SQLite
    #[error("Not a valid GeoPackage: {0}")]
    /// File failed GeoPackage container validation.
    NotGeoPackage(String),
    #[error("GeoPackage schema error: {0}")]
    /// GeoPackage schema/table/metadata error.
    GpkgSchema(String),
    #[error("SQLite error: {0}")]
    /// Internal SQLite engine error while reading/writing GeoPackage.
    Sqlite(String),
    #[error("Projection error: {0}")]
    /// CRS/projection transform error.
    Projection(String),
    #[error("Invalid WKB at offset {offset}: {msg}")]
    /// Invalid WKB payload with failing offset and message.
    InvalidWkb {
        /// Byte offset where WKB decoding failed.
        offset: usize,
        /// Human-readable WKB parse message.
        msg: String,
    },
    #[error("Unsupported WKB geometry type {0}")]
    /// Encountered an unsupported WKB geometry type code.
    UnsupportedWkbType(u32),

    // General
    #[error("Field '{0}' not found")]
    /// Requested field name does not exist in schema.
    FieldNotFound(String),
    #[error("Feature index {index} out of range (len={len})")]
    /// Feature/attribute index is outside valid bounds.
    OutOfRange {
        /// Requested index.
        index: usize,
        /// Collection length.
        len: usize,
    },
    #[error("Data size mismatch: expected {expected}, got {actual}")]
    /// Input/output data length does not match expected size.
    SizeMismatch {
        /// Expected item/byte count.
        expected: usize,
        /// Actual item/byte count.
        actual: usize,
    },
    #[error("Not implemented: {0}")]
    /// Feature exists in API surface but is not implemented yet.
    NotImplemented(String),
}
