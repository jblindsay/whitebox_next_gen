use crate::error::{WbhdfError, WbhdfResult};

/// Placeholder for object header message parsing state.
#[derive(Debug, Default)]
pub struct ObjectHeader;

impl ObjectHeader {
    /// Parses object header messages for a dataset or group.
    pub fn parse(_bytes: &[u8]) -> WbhdfResult<Self> {
        Err(WbhdfError::UnsupportedLayout(
            "object header parsing not implemented".to_string(),
        ))
    }
}
