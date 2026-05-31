//! Scoped HDF container reader for Whitebox Next Gen.
//!
//! The crate targets known product layouts first and does not attempt full
//! HDF4/HDF5 coverage.

pub mod attributes;
pub mod btree;
pub mod dataset;
pub mod datatypes;
pub mod error;
pub mod fixtures;
pub mod filters;
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
