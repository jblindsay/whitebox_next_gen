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
    #[error(
        "datatype mismatch for dataset '{dataset_path}': expected {expected}, actual {actual}"
    )]
    DatatypeMismatch {
        dataset_path: String,
        expected: String,
        actual: String,
    },
    #[error(
        "invalid chunk for dataset '{dataset_path}' (chunk_coordinate={chunk_coordinate:?}, file_offset={file_offset}): {detail}"
    )]
    InvalidChunk {
        dataset_path: String,
        chunk_coordinate: Option<String>,
        file_offset: u64,
        detail: String,
    },
    #[error(
        "filter failure for dataset '{dataset_path}' (chunk_coordinate={chunk_coordinate:?}, file_offset={file_offset}, filter={filter}): {detail}"
    )]
    FilterFailure {
        dataset_path: String,
        chunk_coordinate: Option<String>,
        file_offset: u64,
        filter: String,
        detail: String,
    },
    #[error("chunk address not found for dataset '{dataset_path}' and key {key}")]
    ChunkAddressNotFound { dataset_path: String, key: u64 },
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
