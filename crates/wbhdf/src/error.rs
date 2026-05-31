use thiserror::Error;

pub type WbhdfResult<T> = Result<T, WbhdfError>;

#[derive(Debug, Error)]
pub enum WbhdfError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing dataset selector in URI")]
    MissingDatasetSelector,
    #[error("dataset path not found: {0}")]
    DatasetPathNotFound(String),
    #[error("unsupported container layout: {0}")]
    UnsupportedLayout(String),
    #[error("unsupported filter: {0}")]
    UnsupportedFilter(String),
    #[error("chunk address not found for dataset '{dataset_path}' and key {key}")]
    ChunkAddressNotFound { dataset_path: String, key: u64 },
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
