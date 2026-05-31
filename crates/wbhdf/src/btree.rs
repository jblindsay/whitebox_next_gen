use crate::error::{WbhdfError, WbhdfResult};
use byteorder::{BigEndian, ByteOrder};
use std::collections::BTreeMap;

pub const NODE_HEADER_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BTreeNodeHeader {
    pub node_type: u8,
    pub node_level: u8,
    pub entries_used: u16,
    pub left_sibling: u64,
    pub right_sibling: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalRecord {
    pub key: u64,
    pub child_address: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafRecord {
    pub key: u64,
    pub chunk_address: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ChunkIndex {
    dataset_path: String,
    by_key: BTreeMap<u64, u64>,
}

impl ChunkIndex {
    pub fn new(dataset_path: &str) -> Self {
        Self {
            dataset_path: dataset_path.to_string(),
            by_key: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: u64, chunk_address: u64) {
        self.by_key.insert(key, chunk_address);
    }
}

pub fn parse_node_header(bytes: &[u8]) -> WbhdfResult<BTreeNodeHeader> {
    if bytes.len() < NODE_HEADER_LEN {
        return Err(WbhdfError::InvalidInput(format!(
            "B-tree node header requires at least {NODE_HEADER_LEN} bytes"
        )));
    }

    if &bytes[0..4] != b"TREE" {
        return Err(WbhdfError::UnsupportedLayout(
            "B-tree node is missing TREE signature".to_string(),
        ));
    }

    Ok(BTreeNodeHeader {
        node_type: bytes[4],
        node_level: bytes[5],
        entries_used: BigEndian::read_u16(&bytes[6..8]),
        left_sibling: BigEndian::read_u64(&bytes[8..16]),
        right_sibling: BigEndian::read_u64(&bytes[16..24]),
    })
}

pub fn parse_internal_records(bytes: &[u8], count: usize) -> WbhdfResult<Vec<InternalRecord>> {
    let required = count.checked_mul(16).ok_or_else(|| {
        WbhdfError::InvalidInput("internal record byte count overflow".to_string())
    })?;
    if bytes.len() < required {
        return Err(WbhdfError::InvalidInput(format!(
            "internal record buffer too short: expected {required} bytes"
        )));
    }

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * 16;
        out.push(InternalRecord {
            key: BigEndian::read_u64(&bytes[start..start + 8]),
            child_address: BigEndian::read_u64(&bytes[start + 8..start + 16]),
        });
    }
    Ok(out)
}

pub fn parse_leaf_records(bytes: &[u8], count: usize) -> WbhdfResult<Vec<LeafRecord>> {
    let required = count.checked_mul(16).ok_or_else(|| {
        WbhdfError::InvalidInput("leaf record byte count overflow".to_string())
    })?;
    if bytes.len() < required {
        return Err(WbhdfError::InvalidInput(format!(
            "leaf record buffer too short: expected {required} bytes"
        )));
    }

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = i * 16;
        out.push(LeafRecord {
            key: BigEndian::read_u64(&bytes[start..start + 8]),
            chunk_address: BigEndian::read_u64(&bytes[start + 8..start + 16]),
        });
    }
    Ok(out)
}

/// Returns the child address that should be followed for a lookup key.
pub fn route_child_for_key(records: &[InternalRecord], key: u64) -> WbhdfResult<u64> {
    if records.is_empty() {
        return Err(WbhdfError::InvalidInput(
            "cannot route key in empty internal record set".to_string(),
        ));
    }

    for rec in records {
        if key <= rec.key {
            return Ok(rec.child_address);
        }
    }

    Ok(records[records.len() - 1].child_address)
}

/// Deterministic lookup against a prebuilt chunk index.
pub fn lookup_chunk_address(
    index: &ChunkIndex,
    dataset_path: &str,
    coords: &[u64],
) -> WbhdfResult<u64> {
    if dataset_path != index.dataset_path {
        return Err(WbhdfError::DatasetPathNotFound(dataset_path.to_string()));
    }
    if coords.is_empty() {
        return Err(WbhdfError::InvalidInput(
            "coords must contain at least one index value".to_string(),
        ));
    }

    let key = coords[0];
    index
        .by_key
        .get(&key)
        .copied()
        .ok_or_else(|| WbhdfError::ChunkAddressNotFound {
            dataset_path: dataset_path.to_string(),
            key,
        })
}

#[cfg(test)]
mod tests {
    use super::{
        lookup_chunk_address, parse_internal_records, parse_leaf_records, parse_node_header,
        route_child_for_key, ChunkIndex, InternalRecord,
    };

    #[test]
    fn parse_tree_node_header_succeeds() {
        let mut bytes = vec![0u8; 24];
        bytes[0..4].copy_from_slice(b"TREE");
        bytes[4] = 1;
        bytes[5] = 0;
        bytes[6] = 0;
        bytes[7] = 3;
        bytes[15] = 9;
        bytes[23] = 42;

        let hdr = parse_node_header(&bytes).expect("header should parse");
        assert_eq!(hdr.node_type, 1);
        assert_eq!(hdr.entries_used, 3);
        assert_eq!(hdr.left_sibling, 9);
        assert_eq!(hdr.right_sibling, 42);
    }

    #[test]
    fn route_child_uses_first_upper_bound_or_last() {
        let records = vec![
            InternalRecord {
                key: 10,
                child_address: 100,
            },
            InternalRecord {
                key: 20,
                child_address: 200,
            },
            InternalRecord {
                key: 30,
                child_address: 300,
            },
        ];

        assert_eq!(route_child_for_key(&records, 7).unwrap(), 100);
        assert_eq!(route_child_for_key(&records, 20).unwrap(), 200);
        assert_eq!(route_child_for_key(&records, 55).unwrap(), 300);
    }

    #[test]
    fn parse_internal_and_leaf_records_succeeds() {
        let mut bytes = vec![0u8; 32];
        bytes[7] = 5;
        bytes[15] = 10;
        bytes[23] = 6;
        bytes[31] = 11;

        let internal = parse_internal_records(&bytes, 2).expect("internal records parse");
        let leaf = parse_leaf_records(&bytes, 2).expect("leaf records parse");
        assert_eq!(internal[0].key, 5);
        assert_eq!(internal[0].child_address, 10);
        assert_eq!(leaf[1].key, 6);
        assert_eq!(leaf[1].chunk_address, 11);
    }

    #[test]
    fn lookup_chunk_address_is_deterministic() {
        let mut index = ChunkIndex::new("/group/dataset");
        index.insert(1, 111);
        index.insert(2, 222);

        assert_eq!(
            lookup_chunk_address(&index, "/group/dataset", &[2]).unwrap(),
            222
        );

        let err = lookup_chunk_address(&index, "/group/dataset", &[99]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("chunk address not found"));
    }
}
