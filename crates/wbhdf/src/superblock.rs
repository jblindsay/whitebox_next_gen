use crate::error::{WbhdfError, WbhdfResult};

/// Parsed superblock metadata required for container traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superblock {
    pub version: u8,
}

impl Superblock {
    /// Parses a superblock from raw bytes.
    pub fn parse(_bytes: &[u8]) -> WbhdfResult<Self> {
        Err(WbhdfError::UnsupportedLayout(
            "superblock parsing not implemented".to_string(),
        ))
    }
}
