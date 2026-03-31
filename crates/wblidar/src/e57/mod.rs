//! ASTM E2807 (E57) reader and writer.
//!
//! E57 stores:
//! * A binary section header (48 bytes) at offset 0.
//! * An XML section describing the data structure.
//! * Binary data pages (1024 bytes each) containing point data, with a
//!   CRC-32 checksum of the first 1020 bytes stored in the last 4 bytes.
//!
//! This implementation supports:
//! * Reading Cartesian (x/y/z) and spherical (range/azimuth/elevation) point clouds.
//! * Reading intensity, colour (r/g/b), row/column indices.
//! * Writing Cartesian point clouds with optional colour and intensity.
//! * CRC-32 validation of all binary pages.

pub mod crc;
pub mod page;
pub mod reader;
pub mod writer;
pub mod xml;

pub use reader::E57Reader;
pub use writer::{E57Writer, E57WriterConfig};

/// E57 file signature.
pub const E57_SIGNATURE: &[u8; 8] = b"ASTM-E57";
/// Binary page size (bytes).
pub const PAGE_SIZE: usize = 1024;
/// Usable bytes per page (PAGE_SIZE minus the 4-byte CRC).
pub const PAGE_PAYLOAD: usize = PAGE_SIZE - 4;
