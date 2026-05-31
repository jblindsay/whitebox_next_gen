use crate::error::{WbhdfError, WbhdfResult};

/// Decompresses GZIP payload bytes.
pub fn decompress_gzip(_compressed: &[u8]) -> WbhdfResult<Vec<u8>> {
    Err(WbhdfError::UnsupportedFilter(
        "GZIP decode path not implemented".to_string(),
    ))
}
