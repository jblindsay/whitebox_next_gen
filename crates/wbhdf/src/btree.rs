use crate::error::{WbhdfError, WbhdfResult};
use byteorder::{BigEndian, ByteOrder};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedStorageLeafRecord {
    pub chunk_size: u64,
    pub chunk_offsets: Vec<u64>,
    pub chunk_address: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedStorageInternalRecord {
    pub upper_bound_offsets: Vec<u64>,
    pub child_address: u64,
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

/// Parses the first leaf record from a v1 chunked-storage B-tree node.
///
/// This is a bounded parser for the node shape observed in ATL08 `h_canopy`:
/// a leaf node (`node_type=1`, `node_level=0`) with at least one entry.
pub fn parse_first_chunked_storage_leaf_record(
    bytes: &[u8],
    num_dimensions: usize,
) -> WbhdfResult<ChunkedStorageLeafRecord> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked-storage node parsing requires num_dimensions > 0".to_string(),
        ));
    }
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

    let node_type = bytes[4];
    let node_level = bytes[5];
    let entries_used = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if node_type != 1 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "expected chunked-storage B-tree node type 1, found {node_type}"
        )));
    }
    if node_level != 0 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "expected leaf chunk node (level 0), found level {node_level}"
        )));
    }
    if entries_used == 0 {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage leaf node has no records".to_string(),
        ));
    }

    let key_len = 8usize
        .checked_mul(num_dimensions + 1)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk key length overflow".to_string()))?;
    let first_record_key_start = NODE_HEADER_LEN;
    let first_record_key_end = first_record_key_start
        .checked_add(key_len)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk key range overflow".to_string()))?;
    let first_record_addr_end = first_record_key_end
        .checked_add(8)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk address range overflow".to_string()))?;

    if first_record_addr_end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node record extends beyond provided bytes".to_string(),
        ));
    }

    let chunk_size = u64::from_le_bytes(
        bytes[first_record_key_start..first_record_key_start + 8]
            .try_into()
            .map_err(|_| WbhdfError::UnsupportedLayout("failed to parse chunk size".to_string()))?,
    );

    let mut chunk_offsets = Vec::with_capacity(num_dimensions);
    for i in 0..num_dimensions {
        let start = first_record_key_start + 8 + i * 8;
        let end = start + 8;
        let value = u64::from_le_bytes(
            bytes[start..end].try_into().map_err(|_| {
                WbhdfError::UnsupportedLayout("failed to parse chunk offset".to_string())
            })?,
        );
        chunk_offsets.push(value);
    }

    let chunk_address = u64::from_le_bytes(
        bytes[first_record_key_end..first_record_addr_end]
            .try_into()
            .map_err(|_| {
                WbhdfError::UnsupportedLayout("failed to parse chunk address".to_string())
            })?,
    );

    Ok(ChunkedStorageLeafRecord {
        chunk_size,
        chunk_offsets,
        chunk_address,
    })
}

/// Parses all leaf records from a v1 chunked-storage B-tree leaf node.
///
/// This is still intentionally bounded: it only handles a single leaf node that
/// already contains all records of interest and does not traverse sibling nodes.
pub fn parse_chunked_storage_leaf_records(
    bytes: &[u8],
    num_dimensions: usize,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked-storage node parsing requires num_dimensions > 0".to_string(),
        ));
    }
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

    let node_type = bytes[4];
    let node_level = bytes[5];
    let entries_used = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if node_type != 1 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "expected chunked-storage B-tree node type 1, found {node_type}"
        )));
    }
    if node_level != 0 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "expected leaf chunk node (level 0), found level {node_level}"
        )));
    }
    if entries_used == 0 {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage leaf node has no records".to_string(),
        ));
    }

    let key_len = 8usize
        .checked_mul(num_dimensions + 1)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk key length overflow".to_string()))?;
    let record_len = key_len
        .checked_add(8)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk record length overflow".to_string()))?;
    let required_len = NODE_HEADER_LEN
        .checked_add(
            record_len
                .checked_mul(entries_used)
                .ok_or_else(|| WbhdfError::InvalidInput("chunk record byte count overflow".to_string()))?,
        )
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?;
    if required_len > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node records extend beyond provided bytes".to_string(),
        ));
    }

    let mut records = Vec::with_capacity(entries_used);
    let mut cursor = NODE_HEADER_LEN;
    for _ in 0..entries_used {
        let chunk_size = u64::from_le_bytes(
            bytes[cursor..cursor + 8]
                .try_into()
                .map_err(|_| WbhdfError::UnsupportedLayout("failed to parse chunk size".to_string()))?,
        );
        cursor += 8;

        let mut chunk_offsets = Vec::with_capacity(num_dimensions);
        for _ in 0..num_dimensions {
            let value = u64::from_le_bytes(
                bytes[cursor..cursor + 8].try_into().map_err(|_| {
                    WbhdfError::UnsupportedLayout("failed to parse chunk offset".to_string())
                })?,
            );
            cursor += 8;
            chunk_offsets.push(value);
        }

        let chunk_address = u64::from_le_bytes(
            bytes[cursor..cursor + 8].try_into().map_err(|_| {
                WbhdfError::UnsupportedLayout("failed to parse chunk address".to_string())
            })?,
        );
        cursor += 8;

        records.push(ChunkedStorageLeafRecord {
            chunk_size,
            chunk_offsets,
            chunk_address,
        });
    }

    Ok(records)
}

/// Parses all records from a bounded chunked-storage internal node.
///
/// This staged parser treats each internal record as a vector of upper-bound
/// chunk offsets followed by a child node address, matching the bounded
/// synthetic/internal-root fixtures used for raster-like traversal.
pub fn parse_chunked_storage_internal_records(
    bytes: &[u8],
    num_dimensions: usize,
) -> WbhdfResult<Vec<ChunkedStorageInternalRecord>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked-storage internal parsing requires num_dimensions > 0".to_string(),
        ));
    }
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

    let node_type = bytes[4];
    let node_level = bytes[5];
    let entries_used = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    if node_type != 1 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "expected chunked-storage B-tree node type 1, found {node_type}"
        )));
    }
    if node_level == 0 {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage internal parser requires node level > 0".to_string(),
        ));
    }
    if entries_used == 0 {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage internal node has no records".to_string(),
        ));
    }

    fn parse_with_key_components(
        bytes: &[u8],
        num_dimensions: usize,
        entries_used: usize,
        key_components: usize,
    ) -> WbhdfResult<Vec<ChunkedStorageInternalRecord>> {
        let key_len = 8usize
            .checked_mul(key_components)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk internal key length overflow".to_string()))?;
        let record_len = key_len
            .checked_add(8)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk internal record length overflow".to_string()))?;
        let required_len = NODE_HEADER_LEN
            .checked_add(
                record_len
                    .checked_mul(entries_used)
                    .ok_or_else(|| WbhdfError::InvalidInput("chunk internal byte count overflow".to_string()))?,
            )
            .ok_or_else(|| WbhdfError::InvalidInput("chunk internal node length overflow".to_string()))?;
        if required_len > bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(
                "chunked-storage internal node records extend beyond provided bytes".to_string(),
            ));
        }

        let mut records = Vec::with_capacity(entries_used);
        let mut cursor = NODE_HEADER_LEN;
        for _ in 0..entries_used {
            if key_components == num_dimensions + 1 {
                let _chunk_size_upper_bound = u64::from_le_bytes(
                    bytes[cursor..cursor + 8].try_into().map_err(|_| {
                        WbhdfError::UnsupportedLayout(
                            "failed to parse internal chunk-size upper bound".to_string(),
                        )
                    })?,
                );
                cursor += 8;
            }

            let mut upper_bound_offsets = Vec::with_capacity(num_dimensions);
            for _ in 0..num_dimensions {
                let value = u64::from_le_bytes(
                    bytes[cursor..cursor + 8].try_into().map_err(|_| {
                        WbhdfError::UnsupportedLayout(
                            "failed to parse internal upper-bound offset".to_string(),
                        )
                    })?,
                );
                cursor += 8;
                upper_bound_offsets.push(value);
            }

            let child_address = u64::from_le_bytes(
                bytes[cursor..cursor + 8].try_into().map_err(|_| {
                    WbhdfError::UnsupportedLayout("failed to parse internal child address".to_string())
                })?,
            );
            cursor += 8;

            records.push(ChunkedStorageInternalRecord {
                upper_bound_offsets,
                child_address,
            });
        }

        Ok(records)
    }

    fn score_records(records: &[ChunkedStorageInternalRecord]) -> usize {
        records
            .iter()
            .map(|record| {
                let addr_score = usize::from(record.child_address != 0 && record.child_address != u64::MAX);
                let offset_score = usize::from(record.upper_bound_offsets.iter().any(|value| *value != 0));
                addr_score + offset_score
            })
            .sum()
    }

    // Prefer parsing keys with a leading chunk-size bound, but keep compatibility
    // with legacy synthetic fixtures that only encode per-dimension bounds.
    let with_chunk_size = parse_with_key_components(bytes, num_dimensions, entries_used, num_dimensions + 1);
    let without_chunk_size = parse_with_key_components(bytes, num_dimensions, entries_used, num_dimensions);

    match (with_chunk_size, without_chunk_size) {
        (Ok(a), Ok(b)) => {
            if score_records(&a) >= score_records(&b) {
                Ok(a)
            } else {
                Ok(b)
            }
        }
        (Ok(a), Err(_)) => Ok(a),
        (Err(_), Ok(b)) => Ok(b),
        (Err(err_a), Err(_err_b)) => Err(err_a),
    }
}

pub fn read_first_chunked_storage_leaf_record_in_file(
    path: &Path,
    node_address: u64,
    num_dimensions: usize,
) -> WbhdfResult<ChunkedStorageLeafRecord> {
    let bytes = fs::read(path)?;
    let start = node_address as usize;
    if start + NODE_HEADER_LEN > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node header extends beyond file bytes".to_string(),
        ));
    }

    // The first record and its first key are enough to bound this slice.
    let key_len = 8usize
        .checked_mul(num_dimensions + 1)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk key length overflow".to_string()))?;
    let min_node_len = NODE_HEADER_LEN
        .checked_add(key_len)
        .and_then(|n| n.checked_add(8))
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?;
    let end = start
        .checked_add(min_node_len)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node range overflow".to_string()))?;

    if end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node record extends beyond file bytes".to_string(),
        ));
    }

    parse_first_chunked_storage_leaf_record(&bytes[start..end], num_dimensions)
}

pub fn read_chunked_storage_leaf_records_in_file(
    path: &Path,
    node_address: u64,
    num_dimensions: usize,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    let bytes = fs::read(path)?;
    let start = node_address as usize;
    if start + NODE_HEADER_LEN > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node header extends beyond file bytes".to_string(),
        ));
    }

    let entries_used = u16::from_le_bytes([bytes[start + 6], bytes[start + 7]]) as usize;
    let key_len = 8usize
        .checked_mul(num_dimensions + 1)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk key length overflow".to_string()))?;
    let record_len = key_len
        .checked_add(8)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk record length overflow".to_string()))?;
    let node_len = NODE_HEADER_LEN
        .checked_add(
            record_len
                .checked_mul(entries_used)
                .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?,
        )
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?;
    let end = start
        .checked_add(node_len)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node range overflow".to_string()))?;
    if end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node records extend beyond file bytes".to_string(),
        ));
    }

    parse_chunked_storage_leaf_records(&bytes[start..end], num_dimensions)
}

fn parse_chunked_storage_node_header(bytes: &[u8]) -> WbhdfResult<BTreeNodeHeader> {
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
        entries_used: u16::from_le_bytes([bytes[6], bytes[7]]),
        left_sibling: u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| WbhdfError::UnsupportedLayout("failed to parse left sibling".to_string()))?,
        ),
        right_sibling: u64::from_le_bytes(
            bytes[16..24]
                .try_into()
                .map_err(|_| WbhdfError::UnsupportedLayout("failed to parse right sibling".to_string()))?,
        ),
    })
}

pub fn read_chunked_storage_leaf_chain_records_in_file(
    path: &Path,
    start_node_address: u64,
    num_dimensions: usize,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    if max_leaf_nodes == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked-storage leaf chain requires max_leaf_nodes >= 1".to_string(),
        ));
    }
    if max_records == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked-storage leaf chain requires max_records >= 1".to_string(),
        ));
    }

    let bytes = fs::read(path)?;
    let mut all_records = Vec::<ChunkedStorageLeafRecord>::new();
    let mut current_address = start_node_address;

    for _ in 0..max_leaf_nodes {
        let start = current_address as usize;
        let header_end = start
            .checked_add(NODE_HEADER_LEN)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk node header range overflow".to_string()))?;
        if header_end > bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(
                "chunked-storage node header extends beyond file bytes".to_string(),
            ));
        }

        let header = parse_chunked_storage_node_header(&bytes[start..header_end])?;
        if header.node_type != 1 {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "expected chunked-storage B-tree node type 1, found {}",
                header.node_type
            )));
        }
        if header.node_level != 0 {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "bounded leaf-chain traversal does not yet support internal chunk nodes (found level {})",
                header.node_level
            )));
        }

        let key_len = 8usize
            .checked_mul(num_dimensions + 1)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk key length overflow".to_string()))?;
        let record_len = key_len
            .checked_add(8)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk record length overflow".to_string()))?;
        let node_len = NODE_HEADER_LEN
            .checked_add(
                record_len
                    .checked_mul(header.entries_used as usize)
                    .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?,
            )
            .ok_or_else(|| WbhdfError::InvalidInput("chunk node length overflow".to_string()))?;
        let end = start
            .checked_add(node_len)
            .ok_or_else(|| WbhdfError::InvalidInput("chunk node range overflow".to_string()))?;
        if end > bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(
                "chunked-storage node records extend beyond file bytes".to_string(),
            ));
        }

        let mut records = parse_chunked_storage_leaf_records(&bytes[start..end], num_dimensions)?;
        let remaining_capacity = max_records.saturating_sub(all_records.len());
        if records.len() > remaining_capacity {
            records.truncate(remaining_capacity);
        }
        all_records.extend(records);
        if all_records.len() >= max_records {
            break;
        }

        if header.right_sibling == u64::MAX {
            break;
        }
        current_address = header.right_sibling;
    }

    Ok(all_records)
}

pub fn read_chunked_storage_records_bounded_in_file(
    path: &Path,
    start_node_address: u64,
    num_dimensions: usize,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    let bytes = fs::read(path)?;
    let start = start_node_address as usize;
    let header_end = start
        .checked_add(NODE_HEADER_LEN)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node header range overflow".to_string()))?;
    if header_end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node header extends beyond file bytes".to_string(),
        ));
    }

    let header = parse_chunked_storage_node_header(&bytes[start..header_end])?;
    read_chunked_storage_records_bounded_at_level(
        path,
        start_node_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
        header.node_level as usize,
    )
}

fn read_chunked_storage_records_bounded_at_level(
    path: &Path,
    start_node_address: u64,
    num_dimensions: usize,
    max_leaf_nodes: usize,
    max_records: usize,
    remaining_internal_levels: usize,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    let mut traversal_path = Vec::<u64>::new();
    read_chunked_storage_records_bounded_at_level_with_path(
        path,
        start_node_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
        remaining_internal_levels,
        &mut traversal_path,
    )
}

fn read_chunked_storage_records_bounded_at_level_with_path(
    path: &Path,
    start_node_address: u64,
    num_dimensions: usize,
    max_leaf_nodes: usize,
    max_records: usize,
    remaining_internal_levels: usize,
    traversal_path: &mut Vec<u64>,
) -> WbhdfResult<Vec<ChunkedStorageLeafRecord>> {
    if max_leaf_nodes == 0 {
        return Err(WbhdfError::InvalidInput(
            "bounded chunked traversal requires max_leaf_nodes >= 1".to_string(),
        ));
    }
    if max_records == 0 {
        return Err(WbhdfError::InvalidInput(
            "bounded chunked traversal requires max_records >= 1".to_string(),
        ));
    }

    let bytes = fs::read(path)?;
    let start = start_node_address as usize;
    let header_end = start
        .checked_add(NODE_HEADER_LEN)
        .ok_or_else(|| WbhdfError::InvalidInput("chunk node header range overflow".to_string()))?;
    if header_end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunked-storage node header extends beyond file bytes".to_string(),
        ));
    }

    let header = parse_chunked_storage_node_header(&bytes[start..header_end])?;
    if header.node_level == 0 {
        return read_chunked_storage_leaf_chain_records_in_file(
            path,
            start_node_address,
            num_dimensions,
            max_leaf_nodes,
            max_records,
        );
    }
    if traversal_path.contains(&start_node_address) {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "bounded chunked traversal detected internal-node cycle at address {}",
            start_node_address
        )));
    }

    if remaining_internal_levels == 0 {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "bounded chunked traversal exhausted internal-level budget at level {}",
            header.node_level
        )));
    }

    traversal_path.push(start_node_address);

    let result = (|| {
        let internal_records = parse_chunked_storage_internal_records(&bytes[start..], num_dimensions)?;
        let mut all_records = Vec::<ChunkedStorageLeafRecord>::new();
        for internal_record in internal_records {
            if all_records.len() >= max_records {
                break;
            }
            if internal_record.child_address == 0 || internal_record.child_address == u64::MAX {
                return Err(WbhdfError::UnsupportedLayout(
                    "chunked-storage internal record has invalid child address".to_string(),
                ));
            }
            let remaining = max_records - all_records.len();
            let child_records = read_chunked_storage_records_bounded_at_level_with_path(
                path,
                internal_record.child_address,
                num_dimensions,
                max_leaf_nodes,
                remaining,
                remaining_internal_levels - 1,
                traversal_path,
            )?;
            all_records.extend(child_records);
        }

        if all_records.len() > max_records {
            all_records.truncate(max_records);
        }
        Ok(all_records)
    })();

    traversal_path.pop();
    result
}

pub fn read_chunk_payload_in_file(
    path: &Path,
    chunk_address: u64,
    chunk_size: u64,
) -> WbhdfResult<Vec<u8>> {
    let bytes = fs::read(path)?;
    let start = chunk_address as usize;
    let size = chunk_size as usize;
    let end = start.saturating_add(size);
    if end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "chunk payload extends beyond file bytes".to_string(),
        ));
    }

    Ok(bytes[start..end].to_vec())
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
        lookup_chunk_address, parse_chunked_storage_internal_records,
        parse_chunked_storage_leaf_records, read_chunked_storage_leaf_chain_records_in_file,
        read_chunked_storage_records_bounded_at_level,
        read_chunked_storage_records_bounded_in_file,
        parse_first_chunked_storage_leaf_record, parse_internal_records, parse_leaf_records,
        parse_node_header, route_child_for_key, ChunkIndex, InternalRecord,
    };
    use std::fs;
    use tempfile::NamedTempFile;

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

    #[test]
    fn parses_first_chunked_storage_leaf_record() {
        let bytes = [
            b'T', b'R', b'E', b'E', 0x01, 0x00, 0x01, 0x00,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xb6, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xe5, 0xcc, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let record = parse_first_chunked_storage_leaf_record(&bytes, 2)
            .expect("chunked storage leaf record should parse");
        assert_eq!(record.chunk_size, 13_494);
        assert_eq!(record.chunk_offsets, vec![0, 0]);
        assert_eq!(record.chunk_address, 9_489_637);
    }

    #[test]
    fn parses_multiple_chunked_storage_leaf_records() {
        let bytes = [
            b'T', b'R', b'E', b'E', 0x01, 0x00, 0x02, 0x00,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x88, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x98, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let records = parse_chunked_storage_leaf_records(&bytes, 2)
            .expect("chunked storage leaf records should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chunk_size, 16);
        assert_eq!(records[0].chunk_offsets, vec![0, 0]);
        assert_eq!(records[0].chunk_address, 5000);
        assert_eq!(records[1].chunk_offsets, vec![0, 2]);
        assert_eq!(records[1].chunk_address, 5016);
    }

    #[test]
    fn reads_chained_chunked_storage_leaf_records() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let first_offset = 128usize;
        let second_offset = 216usize;
        let mut bytes = vec![0u8; 304];

        bytes[first_offset..first_offset + 4].copy_from_slice(b"TREE");
        bytes[first_offset + 4] = 1;
        bytes[first_offset + 5] = 0;
        bytes[first_offset + 6..first_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_offset + 8..first_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_offset + 16..first_offset + 24].copy_from_slice(&(second_offset as u64).to_le_bytes());
        let mut cursor = first_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_offset..second_offset + 4].copy_from_slice(b"TREE");
        bytes[second_offset + 4] = 1;
        bytes[second_offset + 5] = 0;
        bytes[second_offset + 6..second_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_offset + 8..second_offset + 16].copy_from_slice(&(first_offset as u64).to_le_bytes());
        bytes[second_offset + 16..second_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let records = read_chunked_storage_leaf_chain_records_in_file(tmp.path(), first_offset as u64, 2, 4, 2)
            .expect("leaf chain records should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chunk_offsets, vec![0, 0]);
        assert_eq!(records[1].chunk_offsets, vec![0, 2]);
    }

    #[test]
    fn parses_chunked_storage_internal_records() {
        let bytes = [
            b'T', b'R', b'E', b'E', 0x01, 0x01, 0x02, 0x00,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x88, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x98, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let records = parse_chunked_storage_internal_records(&bytes, 2)
            .expect("chunked storage internal records should parse");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].upper_bound_offsets, vec![0, 2]);
        assert_eq!(records[0].child_address, 5000);
        assert_eq!(records[1].upper_bound_offsets, vec![0, 4]);
        assert_eq!(records[1].child_address, 5016);
    }

    #[test]
    fn reads_bounded_chunked_records_through_internal_root() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let first_leaf_offset = 216usize;
        let second_leaf_offset = 304usize;
        let mut bytes = vec![0u8; 392];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 1;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_leaf_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_leaf_offset as u64).to_le_bytes());

        bytes[first_leaf_offset..first_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[first_leaf_offset + 4] = 1;
        bytes[first_leaf_offset + 5] = 0;
        bytes[first_leaf_offset + 6..first_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_leaf_offset + 8..first_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_leaf_offset + 16..first_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_leaf_offset..second_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[second_leaf_offset + 4] = 1;
        bytes[second_leaf_offset + 5] = 0;
        bytes[second_leaf_offset + 6..second_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_leaf_offset + 8..second_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_leaf_offset + 16..second_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let records = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 4, 2)
            .expect("bounded chunked traversal should read leaf records through internal root");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chunk_offsets, vec![0, 0]);
        assert_eq!(records[1].chunk_offsets, vec![0, 2]);
    }

    #[test]
    fn reports_malformed_multilevel_root_as_unsupported() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let mut bytes = vec![0u8; 192];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 2;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(160u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let err = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 4, 2)
            .expect_err("multi-level root should be rejected by bounded traversal");
        let msg = format!("{err}");
        assert!(msg.contains("B-tree node is missing TREE signature"));
    }

    #[test]
    fn reads_bounded_chunked_records_through_multilevel_root() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let internal_offset = 216usize;
        let first_leaf_offset = 304usize;
        let second_leaf_offset = 392usize;
        let mut bytes = vec![0u8; 480];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 2;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(internal_offset as u64).to_le_bytes());

        bytes[internal_offset..internal_offset + 4].copy_from_slice(b"TREE");
        bytes[internal_offset + 4] = 1;
        bytes[internal_offset + 5] = 1;
        bytes[internal_offset + 6..internal_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[internal_offset + 8..internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[internal_offset + 16..internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_leaf_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_leaf_offset as u64).to_le_bytes());

        bytes[first_leaf_offset..first_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[first_leaf_offset + 4] = 1;
        bytes[first_leaf_offset + 5] = 0;
        bytes[first_leaf_offset + 6..first_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_leaf_offset + 8..first_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_leaf_offset + 16..first_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_leaf_offset..second_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[second_leaf_offset + 4] = 1;
        bytes[second_leaf_offset + 5] = 0;
        bytes[second_leaf_offset + 6..second_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_leaf_offset + 8..second_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_leaf_offset + 16..second_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let records = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 4, 2)
            .expect("bounded chunked traversal should read records through multilevel root");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chunk_offsets, vec![0, 0]);
        assert_eq!(records[1].chunk_offsets, vec![0, 2]);
    }

    #[test]
    fn reads_bounded_chunked_records_through_multilevel_internal_fanout() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let first_internal_offset = 216usize;
        let second_internal_offset = 304usize;
        let first_leaf_offset = 392usize;
        let second_leaf_offset = 480usize;
        let third_leaf_offset = 568usize;
        let mut bytes = vec![0u8; 656];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 2;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_internal_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(6u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_internal_offset as u64).to_le_bytes());

        bytes[first_internal_offset..first_internal_offset + 4].copy_from_slice(b"TREE");
        bytes[first_internal_offset + 4] = 1;
        bytes[first_internal_offset + 5] = 1;
        bytes[first_internal_offset + 6..first_internal_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[first_internal_offset + 8..first_internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_internal_offset + 16..first_internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_leaf_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_leaf_offset as u64).to_le_bytes());

        bytes[second_internal_offset..second_internal_offset + 4].copy_from_slice(b"TREE");
        bytes[second_internal_offset + 4] = 1;
        bytes[second_internal_offset + 5] = 1;
        bytes[second_internal_offset + 6..second_internal_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_internal_offset + 8..second_internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_internal_offset + 16..second_internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(6u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(third_leaf_offset as u64).to_le_bytes());

        bytes[first_leaf_offset..first_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[first_leaf_offset + 4] = 1;
        bytes[first_leaf_offset + 5] = 0;
        bytes[first_leaf_offset + 6..first_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_leaf_offset + 8..first_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_leaf_offset + 16..first_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_leaf_offset..second_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[second_leaf_offset + 4] = 1;
        bytes[second_leaf_offset + 5] = 0;
        bytes[second_leaf_offset + 6..second_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_leaf_offset + 8..second_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_leaf_offset + 16..second_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        bytes[third_leaf_offset..third_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[third_leaf_offset + 4] = 1;
        bytes[third_leaf_offset + 5] = 0;
        bytes[third_leaf_offset + 6..third_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[third_leaf_offset + 8..third_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[third_leaf_offset + 16..third_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = third_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5032u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let records = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 6, 3)
            .expect("bounded chunked traversal should read records through multilevel internal fanout");
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].chunk_offsets, vec![0, 0]);
        assert_eq!(records[1].chunk_offsets, vec![0, 2]);
        assert_eq!(records[2].chunk_offsets, vec![0, 4]);
    }

    #[test]
    fn reports_malformed_multilevel_internal_fanout_as_unsupported() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let first_internal_offset = 216usize;
        let second_internal_offset = 304usize;
        let first_leaf_offset = 392usize;
        let second_leaf_offset = 480usize;
        let mut bytes = vec![0u8; 568];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 2;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_internal_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(6u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_internal_offset as u64).to_le_bytes());

        bytes[first_internal_offset..first_internal_offset + 4].copy_from_slice(b"TREE");
        bytes[first_internal_offset + 4] = 1;
        bytes[first_internal_offset + 5] = 1;
        bytes[first_internal_offset + 6..first_internal_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[first_internal_offset + 8..first_internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_internal_offset + 16..first_internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_leaf_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_leaf_offset as u64).to_le_bytes());

        bytes[first_leaf_offset..first_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[first_leaf_offset + 4] = 1;
        bytes[first_leaf_offset + 5] = 0;
        bytes[first_leaf_offset + 6..first_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_leaf_offset + 8..first_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_leaf_offset + 16..first_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_leaf_offset..second_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[second_leaf_offset + 4] = 1;
        bytes[second_leaf_offset + 5] = 0;
        bytes[second_leaf_offset + 6..second_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_leaf_offset + 8..second_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_leaf_offset + 16..second_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let err = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 6, 3)
            .expect_err("malformed multilevel internal fanout should fail explicitly");
        assert!(format!("{err}").contains("B-tree node is missing TREE signature"));
    }

    #[test]
    fn reports_multilevel_internal_fanout_budget_exhaustion() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let first_internal_offset = 216usize;
        let second_internal_offset = 304usize;
        let first_leaf_offset = 392usize;
        let second_leaf_offset = 480usize;
        let third_leaf_offset = 568usize;
        let mut bytes = vec![0u8; 656];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 2;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_internal_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(6u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_internal_offset as u64).to_le_bytes());

        bytes[first_internal_offset..first_internal_offset + 4].copy_from_slice(b"TREE");
        bytes[first_internal_offset + 4] = 1;
        bytes[first_internal_offset + 5] = 1;
        bytes[first_internal_offset + 6..first_internal_offset + 8].copy_from_slice(&(2u16).to_le_bytes());
        bytes[first_internal_offset + 8..first_internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_internal_offset + 16..first_internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(first_leaf_offset as u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(second_leaf_offset as u64).to_le_bytes());

        bytes[second_internal_offset..second_internal_offset + 4].copy_from_slice(b"TREE");
        bytes[second_internal_offset + 4] = 1;
        bytes[second_internal_offset + 5] = 1;
        bytes[second_internal_offset + 6..second_internal_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_internal_offset + 8..second_internal_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_internal_offset + 16..second_internal_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_internal_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(6u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(third_leaf_offset as u64).to_le_bytes());

        bytes[first_leaf_offset..first_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[first_leaf_offset + 4] = 1;
        bytes[first_leaf_offset + 5] = 0;
        bytes[first_leaf_offset + 6..first_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[first_leaf_offset + 8..first_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[first_leaf_offset + 16..first_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = first_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5000u64).to_le_bytes());

        bytes[second_leaf_offset..second_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[second_leaf_offset + 4] = 1;
        bytes[second_leaf_offset + 5] = 0;
        bytes[second_leaf_offset + 6..second_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[second_leaf_offset + 8..second_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[second_leaf_offset + 16..second_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = second_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5016u64).to_le_bytes());

        bytes[third_leaf_offset..third_leaf_offset + 4].copy_from_slice(b"TREE");
        bytes[third_leaf_offset + 4] = 1;
        bytes[third_leaf_offset + 5] = 0;
        bytes[third_leaf_offset + 6..third_leaf_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[third_leaf_offset + 8..third_leaf_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[third_leaf_offset + 16..third_leaf_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = third_leaf_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&(16u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(4u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(5032u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let err = read_chunked_storage_records_bounded_at_level(
            tmp.path(),
            root_offset as u64,
            2,
            6,
            3,
            1,
        )
        .expect_err("insufficient internal-level budget should fail explicitly");
        assert!(format!("{err}").contains("exhausted internal-level budget at level 1"));
    }

    #[test]
    fn reports_internal_node_cycle_as_unsupported() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let mut bytes = vec![0u8; 256];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 1;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(root_offset as u64).to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let err = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 4, 2)
            .expect_err("internal-node cycle should fail explicitly");
        assert!(
            format!("{err}").contains("internal-node cycle"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn reports_invalid_internal_child_address_as_unsupported() {
        let tmp = NamedTempFile::new().expect("temp file should be created");
        let root_offset = 128usize;
        let mut bytes = vec![0u8; 256];

        bytes[root_offset..root_offset + 4].copy_from_slice(b"TREE");
        bytes[root_offset + 4] = 1;
        bytes[root_offset + 5] = 1;
        bytes[root_offset + 6..root_offset + 8].copy_from_slice(&(1u16).to_le_bytes());
        bytes[root_offset + 8..root_offset + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes[root_offset + 16..root_offset + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        let mut cursor = root_offset + 24;
        bytes[cursor..cursor + 8].copy_from_slice(&0u64.to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&(2u64).to_le_bytes());
        cursor += 8;
        bytes[cursor..cursor + 8].copy_from_slice(&u64::MAX.to_le_bytes());

        fs::write(tmp.path(), &bytes).expect("temp bytes should be writable");

        let err = read_chunked_storage_records_bounded_in_file(tmp.path(), root_offset as u64, 2, 4, 2)
            .expect_err("internal node with invalid child address should fail explicitly");
        assert!(
            format!("{err}").contains("invalid child address"),
            "unexpected error: {err}"
        );
    }
}
