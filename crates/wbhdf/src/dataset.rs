use crate::error::{WbhdfError, WbhdfResult};

/// Minimal dataset descriptor used during early scaffolding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDescriptor {
    pub path: String,
}

/// Resolves a dataset descriptor from a canonical dataset path.
pub fn resolve_dataset(path: &str) -> WbhdfResult<DatasetDescriptor> {
    if !path.starts_with('/') {
        return Err(WbhdfError::InvalidInput(
            "dataset path must start with '/'".to_string(),
        ));
    }

    Ok(DatasetDescriptor {
        path: path.to_string(),
    })
}
