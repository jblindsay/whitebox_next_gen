use crate::btree::{lookup_chunk_address, ChunkIndex};
use crate::error::{WbhdfError, WbhdfResult};

/// Minimal dataset descriptor used during early scaffolding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDescriptor {
    pub path: String,
}

/// Dataset-level chunk locator wiring B-tree lookup into dataset reads.
#[derive(Debug, Clone)]
pub struct DatasetChunkLocator {
    descriptor: DatasetDescriptor,
    chunk_index: ChunkIndex,
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

impl DatasetChunkLocator {
    /// Constructs a locator from known key -> chunk address mappings.
    pub fn with_known_addresses(
        dataset_path: &str,
        key_to_address: &[(u64, u64)],
    ) -> WbhdfResult<Self> {
        let descriptor = resolve_dataset(dataset_path)?;
        let mut chunk_index = ChunkIndex::new(&descriptor.path);

        for (key, address) in key_to_address {
            chunk_index.insert(*key, *address);
        }

        Ok(Self {
            descriptor,
            chunk_index,
        })
    }

    /// Locates the chunk address for the supplied dataset coordinates.
    pub fn locate_chunk_address(&self, coords: &[u64]) -> WbhdfResult<u64> {
        lookup_chunk_address(&self.chunk_index, &self.descriptor.path, coords)
    }

    /// Returns the dataset path bound to this locator.
    pub fn dataset_path(&self) -> &str {
        &self.descriptor.path
    }
}

#[cfg(test)]
mod tests {
    use super::DatasetChunkLocator;

    #[test]
    fn locator_returns_known_addresses() {
        let locator = DatasetChunkLocator::with_known_addresses(
            "/GEDI04_B/BEAM0000/rh100",
            &[(0, 4000), (1, 4500), (2, 5000)],
        )
        .expect("locator should construct");

        assert_eq!(locator.dataset_path(), "/GEDI04_B/BEAM0000/rh100");
        assert_eq!(locator.locate_chunk_address(&[0]).unwrap(), 4000);
        assert_eq!(locator.locate_chunk_address(&[2]).unwrap(), 5000);
    }

    #[test]
    fn locator_reports_missing_known_key() {
        let locator = DatasetChunkLocator::with_known_addresses(
            "/GEDI04_B/BEAM0000/rh100",
            &[(3, 7000)],
        )
        .expect("locator should construct");

        let err = locator.locate_chunk_address(&[1]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("chunk address not found"));
    }
}
