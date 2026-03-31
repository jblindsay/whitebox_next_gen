//! PLY (Polygon File Format / Stanford Triangle Format) reader and writer.
//!
//! Supports:
//! * ASCII, binary little-endian, and binary big-endian encodings.
//! * Reading x/y/z, intensity, r/g/b, nx/ny/nz, and the most common scalar
//!   properties.
//! * Writing in binary little-endian by default (fastest).

pub mod reader;
pub mod writer;

pub use reader::PlyReader;
pub use writer::PlyWriter;

/// The PLY magic string.
pub const PLY_MAGIC: &str = "ply\n";

/// Encoding of the PLY file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlyEncoding {
    /// Binary little-endian (default, fastest).
    #[default]
    BinaryLittleEndian,
    /// Binary big-endian.
    BinaryBigEndian,
    /// ASCII (slowest but human-readable).
    Ascii,
}
