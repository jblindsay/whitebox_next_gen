//! Scoped HDF container reader for Whitebox Next Gen.
//!
//! The crate targets known product layouts first and does not attempt full
//! HDF4/HDF5 coverage.
//!
//! # Examples
//!
//! Read a bounded contiguous f32 window when a known byte offset is available:
//! ```rust,no_run
//! use std::path::Path;
//! use wbhdf::dataset::{read_contiguous_f32_window_in_file, resolve_dataset_in_file};
//! use wbhdf::datatypes::Endianness;
//!
//! let file_path = Path::new("/data/sample.h5");
//! let _descriptor = resolve_dataset_in_file(file_path, "/BEAM0000/elev_lowestmode")?;
//! let values = read_contiguous_f32_window_in_file(file_path, 1_012_683, 4, Endianness::Little)?;
//! assert_eq!(values.len(), 4);
//! # Ok::<(), wbhdf::WbhdfError>(())
//! ```
//!
//! Read a bounded i16 window from an HDF4 SDS dataset path:
//! ```rust,no_run
//! use std::path::Path;
//! use wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file;
//!
//! let file_path = Path::new("/data/MOD09A1.example.hdf");
//! let values = decode_hdf4_sds_i16_window_at_in_file(
//!     file_path,
//!     "/mod09a1_sur_refl/Data Fields/sur_refl_b01",
//!     0,
//!     256,
//! )?;
//! assert!(!values.is_empty());
//! # Ok::<(), wbhdf::WbhdfError>(())
//! ```

pub mod attributes;
pub mod btree;
pub mod compare;
pub mod dataset;
pub mod datatypes;
pub mod error;
pub mod fixtures;
pub mod filters;
pub mod hdf4;
pub mod object_header;
pub mod superblock;

pub use crate::error::{WbhdfError, WbhdfResult};

/// A placeholder reader entry point for early integration plumbing.
#[derive(Debug, Default)]
pub struct Reader;

impl Reader {
    /// Creates a new reader instance.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;

    #[test]
    fn reader_can_be_constructed() {
        let _reader = Reader::new();
    }
}
