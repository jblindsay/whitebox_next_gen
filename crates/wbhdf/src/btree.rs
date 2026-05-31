use crate::error::{WbhdfError, WbhdfResult};

/// Placeholder chunk index API for B-tree-backed chunk lookups.
pub fn lookup_chunk_address(_dataset_path: &str, _coords: &[u64]) -> WbhdfResult<u64> {
    Err(WbhdfError::UnsupportedLayout(
        "B-tree chunk lookup not implemented".to_string(),
    ))
}
