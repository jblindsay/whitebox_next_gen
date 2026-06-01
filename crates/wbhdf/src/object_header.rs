use crate::error::{WbhdfError, WbhdfResult};
use std::fs;
use std::path::Path;

const OBJECT_HEADER_SIGNATURE: &[u8; 4] = b"OHDR";

/// Decoded fields from an HDF5 v2 object-header prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeaderV2 {
    pub offset: usize,
    pub version: u8,
    pub flags: u8,
    pub prefix_len: usize,
    pub chunk0_size: u64,
    pub messages: Vec<ObjectHeaderMessageHeader>,
    pub continuations: Vec<ObjectHeaderContinuation>,
    pub dataspaces: Vec<DataspaceMessage>,
    pub datatypes: Vec<DatatypeMessage>,
}

/// Minimal decoded object-header message header for the first chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeaderMessageHeader {
    pub type_id: u8,
    pub size: u16,
    pub flags: u8,
    pub header_offset: usize,
    pub data_offset: usize,
}

/// Minimal decoded header-continuation target from a v2 object header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeaderContinuation {
    pub address: u64,
    pub size: u64,
}

/// Minimal decoded continuation chunk carrying message headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationChunk {
    pub address: u64,
    pub size: u64,
    pub messages: Vec<ObjectHeaderMessageHeader>,
    pub continuations: Vec<ObjectHeaderContinuation>,
    pub layouts: Vec<LayoutMessage>,
}

/// Minimal decoded contiguous-layout message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutMessage {
    pub version: u8,
    pub layout_class: u8,
    pub data_address: u64,
    pub data_size: u64,
}

/// Minimal decoded dataspace message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataspaceMessage {
    pub version: u8,
    pub rank: u8,
    pub flags: u8,
    pub dimensions: Vec<u64>,
    pub max_dimensions: Vec<u64>,
}

/// Minimal decoded datatype message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatatypeMessage {
    pub version: u8,
    pub class: u8,
    pub bit_field_0: u8,
    pub bit_field_1: u8,
    pub bit_field_2: u8,
    pub size: u32,
}

/// Minimal object-header probe state used for early real-fixture traversal checks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ObjectHeader {
    pub signature_offsets: Vec<usize>,
    pub v2_headers: Vec<ObjectHeaderV2>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeaderV1 {
    pub offset: usize,
    pub total_header_size: u32,
    pub message_count: u16,
    pub messages: Vec<ObjectHeaderV1MessageHeader>,
    pub dataspaces: Vec<DataspaceMessage>,
    pub datatypes: Vec<DatatypeMessage>,
    pub fill_values: Vec<FillValueMessage>,
    pub filter_pipelines: Vec<FilterPipelineMessage>,
    pub chunked_layouts: Vec<ChunkedLayoutMessage>,
    pub continuations: Vec<ObjectHeaderContinuation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectHeaderV1MessageHeader {
    pub type_id: u16,
    pub size: u16,
    pub flags: u8,
    pub header_offset: usize,
    pub data_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillValueMessage {
    pub version: u8,
    pub allocation_time: u8,
    pub fill_time: u8,
    pub value_defined: u8,
    pub value_size: u32,
    pub value_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterPipelineMessage {
    pub version: u8,
    pub filters: Vec<FilterDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterDescription {
    pub id: u16,
    pub name: String,
    pub flags: u16,
    pub client_data: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedLayoutMessage {
    pub version: u8,
    pub layout_class: u8,
    pub num_dimensions: u8,
    pub index_address: u64,
    pub chunk_dimensions: Vec<u32>,
}

impl ObjectHeader {
    /// Scans raw bytes for HDF5 v2 object-header signatures (`OHDR`).
    pub fn parse(bytes: &[u8]) -> WbhdfResult<Self> {
        let signature_offsets = find_object_header_signatures(bytes);
        if signature_offsets.is_empty() {
            return Err(WbhdfError::UnsupportedLayout(
                "no object header signatures discovered".to_string(),
            ));
        }

        let v2_headers = signature_offsets
            .iter()
            .filter_map(|offset| parse_v2_header_at(bytes, *offset))
            .collect();

        Ok(Self {
            signature_offsets,
            v2_headers,
        })
    }
}

/// Probes object-header signature locations from a container file.
pub fn probe_file_object_headers(path: &Path) -> WbhdfResult<ObjectHeader> {
    let bytes = fs::read(path)?;
    ObjectHeader::parse(&bytes)
}

pub fn parse_v1_object_header_in_file(path: &Path, offset: usize) -> WbhdfResult<ObjectHeaderV1> {
    let bytes = fs::read(path)?;
    parse_v1_object_header(&bytes, offset)
}

/// Discovers parsable v1 object headers using a bounded scan over container bytes.
///
/// This utility is intentionally conservative and favors safety over exhaustive
/// parsing. It returns up to `max_results` candidate headers ordered by byte offset.
pub fn discover_v1_object_headers_in_file(path: &Path, max_results: usize) -> WbhdfResult<Vec<ObjectHeaderV1>> {
    let bytes = fs::read(path)?;
    discover_v1_object_headers(&bytes, max_results)
}

/// Discovers parsable v1 object headers from raw bytes using a bounded scan.
pub fn discover_v1_object_headers(bytes: &[u8], max_results: usize) -> WbhdfResult<Vec<ObjectHeaderV1>> {
    if max_results == 0 {
        return Err(WbhdfError::InvalidInput(
            "v1 object-header discovery requires max_results >= 1".to_string(),
        ));
    }
    if bytes.len() < 16 {
        return Err(WbhdfError::UnsupportedLayout(
            "container is too small for v1 object-header discovery".to_string(),
        ));
    }

    let mut discovered = Vec::<ObjectHeaderV1>::new();
    let scan_end = bytes.len().saturating_sub(16);
    for offset in 0..=scan_end {
        // Fast prefilter for plausible v1 headers before attempting full parse.
        if bytes[offset] != 1 {
            continue;
        }
        let message_count = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
        if message_count == 0 || message_count > 256 {
            continue;
        }
        let total_header_size = u32::from_le_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
        ]);
        if total_header_size < 16 || total_header_size > 1_048_576 {
            continue;
        }
        let chunk_end = offset.saturating_add(total_header_size as usize);
        if chunk_end > bytes.len() {
            continue;
        }

        if let Ok(parsed) = parse_v1_object_header(bytes, offset) {
            discovered.push(parsed);
            if discovered.len() >= max_results {
                break;
            }
        }
    }

    if discovered.is_empty() {
        return Err(WbhdfError::UnsupportedLayout(
            "no parsable v1 object headers discovered in bounded scan".to_string(),
        ));
    }

    Ok(discovered)
}

/// Parses a continuation chunk at a decoded object-header continuation target.
pub fn parse_continuation_chunk_in_file(
    path: &Path,
    continuation: &ObjectHeaderContinuation,
) -> WbhdfResult<ContinuationChunk> {
    let bytes = fs::read(path)?;
    parse_continuation_chunk(&bytes, continuation)
}

pub fn read_contiguous_layout_bytes_in_file(path: &Path, layout: &LayoutMessage) -> WbhdfResult<Vec<u8>> {
    let bytes = fs::read(path)?;
    let start = layout.data_address as usize;
    let size = layout.data_size as usize;
    let end = start.saturating_add(size);

    if end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "contiguous layout extends beyond container bytes".to_string(),
        ));
    }

    Ok(bytes[start..end].to_vec())
}

fn find_object_header_signatures(bytes: &[u8]) -> Vec<usize> {
    bytes
        .windows(OBJECT_HEADER_SIGNATURE.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == OBJECT_HEADER_SIGNATURE).then_some(offset))
        .collect()
}

fn parse_v2_header_at(bytes: &[u8], offset: usize) -> Option<ObjectHeaderV2> {
    // Signature + version + flags + chunk-size (at least 1 byte)
    if bytes.len() < offset + 7 {
        return None;
    }

    let version = bytes[offset + 4];
    if version != 2 {
        return None;
    }

    let flags = bytes[offset + 5];
    let size_len = match flags & 0b11 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => 8,
    };

    let prefix_metadata_len = timestamp_field_len(flags);
    let chunk_size_offset = offset + 6 + prefix_metadata_len;
    if bytes.len() < chunk_size_offset + size_len {
        return None;
    }

    let mut chunk0_size = 0_u64;
    for i in 0..size_len {
        chunk0_size |= (bytes[chunk_size_offset + i] as u64) << (8 * i);
    }

    let prefix_len = 6 + prefix_metadata_len + size_len;
    let messages = parse_v2_messages(bytes, offset, prefix_len, chunk0_size as usize);
    let continuations = parse_header_continuations(bytes, &messages);
    let dataspaces = parse_dataspace_messages(bytes, &messages);
    let datatypes = parse_datatype_messages(bytes, &messages);

    Some(ObjectHeaderV2 {
        offset,
        version,
        flags,
        prefix_len,
        chunk0_size,
        messages,
        continuations,
        dataspaces,
        datatypes,
    })
}

fn parse_v1_object_header(bytes: &[u8], offset: usize) -> WbhdfResult<ObjectHeaderV1> {
    if bytes.len() < offset + 16 {
        return Err(WbhdfError::UnsupportedLayout(
            "v1 object header is truncated".to_string(),
        ));
    }

    let version = bytes[offset];
    if version != 1 {
        return Err(WbhdfError::UnsupportedLayout(
            format!("unsupported v1 object header version byte: {version}"),
        ));
    }

    let message_count = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
    let total_header_size = u32::from_le_bytes([
        bytes[offset + 8],
        bytes[offset + 9],
        bytes[offset + 10],
        bytes[offset + 11],
    ]);
    let chunk0_end = offset.saturating_add(total_header_size as usize);
    if chunk0_end > bytes.len() {
        return Err(WbhdfError::UnsupportedLayout(
            "v1 object header chunk extends beyond container bytes".to_string(),
        ));
    }

    let messages = parse_v1_messages(bytes, offset + 16, chunk0_end, message_count as usize);
    let dataspaces = parse_v1_dataspace_messages(bytes, &messages);
    let datatypes = parse_v1_datatype_messages(bytes, &messages);
    let fill_values = parse_v1_fill_value_messages(bytes, &messages);
    let filter_pipelines = parse_v1_filter_pipeline_messages(bytes, &messages);
    let chunked_layouts = parse_v1_chunked_layout_messages(bytes, &messages);
    let continuations = parse_v1_continuations(bytes, &messages);

    Ok(ObjectHeaderV1 {
        offset,
        total_header_size,
        message_count,
        messages,
        dataspaces,
        datatypes,
        fill_values,
        filter_pipelines,
        chunked_layouts,
        continuations,
    })
}

fn parse_v1_fill_value_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<FillValueMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x0004 && message.size >= 8)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let allocation_time = bytes[message.data_offset + 1];
            let fill_time = bytes[message.data_offset + 2];
            let value_defined = bytes[message.data_offset + 3];
            let value_size = u32::from_le_bytes(
                bytes[message.data_offset + 4..message.data_offset + 8]
                    .try_into()
                    .ok()?,
            );

            let value_start = message.data_offset + 8;
            let value_end = value_start.saturating_add(value_size as usize);
            if value_end > body_end {
                return None;
            }

            Some(FillValueMessage {
                version,
                allocation_time,
                fill_time,
                value_defined,
                value_size,
                value_bytes: bytes[value_start..value_end].to_vec(),
            })
        })
        .collect()
}

fn timestamp_field_len(flags: u8) -> usize {
    if flags & 0x20 != 0 {
        16
    } else {
        0
    }
}
fn parse_v2_messages(
    bytes: &[u8],
    object_header_offset: usize,
    prefix_len: usize,
    chunk0_size: usize,
) -> Vec<ObjectHeaderMessageHeader> {
    let mut messages = Vec::new();
    let chunk_end = object_header_offset
        .saturating_add(prefix_len)
        .saturating_add(chunk0_size);
    let mut cursor = object_header_offset.saturating_add(prefix_len);

    while cursor + 4 <= bytes.len() && cursor + 4 <= chunk_end {
        let type_id = bytes[cursor];
        let size = u16::from_le_bytes([bytes[cursor + 1], bytes[cursor + 2]]);
        let flags = bytes[cursor + 3];
        let data_offset = cursor + 4;
        let next_cursor = data_offset.saturating_add(size as usize);

        if size == 0 && type_id == 0 {
            break;
        }
        if next_cursor > bytes.len() || next_cursor > chunk_end {
            break;
        }

        messages.push(ObjectHeaderMessageHeader {
            type_id,
            size,
            flags,
            header_offset: cursor,
            data_offset,
        });

        cursor = next_cursor;
    }

    messages
}

fn parse_v1_messages(
    bytes: &[u8],
    message_start: usize,
    chunk_end: usize,
    max_messages: usize,
) -> Vec<ObjectHeaderV1MessageHeader> {
    let mut messages = Vec::new();
    let mut cursor = message_start;

    while messages.len() < max_messages && cursor + 8 <= bytes.len() && cursor + 8 <= chunk_end {
        let type_id = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
        let size = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
        let flags = bytes[cursor + 4];
        let data_offset = cursor + 8;
        let next_cursor = data_offset.saturating_add(size as usize);

        if next_cursor > bytes.len() || next_cursor > chunk_end {
            break;
        }

        messages.push(ObjectHeaderV1MessageHeader {
            type_id,
            size,
            flags,
            header_offset: cursor,
            data_offset,
        });

        cursor = next_cursor;
    }

    messages
}

fn parse_header_continuations(
    bytes: &[u8],
    messages: &[ObjectHeaderMessageHeader],
) -> Vec<ObjectHeaderContinuation> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x10 && message.size >= 16)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let address = u64::from_le_bytes(
                bytes[message.data_offset..message.data_offset + 8]
                    .try_into()
                    .ok()?,
            );
            let size = u64::from_le_bytes(
                bytes[message.data_offset + 8..message.data_offset + 16]
                    .try_into()
                    .ok()?,
            );

            Some(ObjectHeaderContinuation { address, size })
        })
        .collect()
}

fn parse_continuation_chunk(
    bytes: &[u8],
    continuation: &ObjectHeaderContinuation,
) -> WbhdfResult<ContinuationChunk> {
    let chunk_start = continuation.address as usize;
    let chunk_size = continuation.size as usize;
    let chunk_end = chunk_start.saturating_add(chunk_size);

    if chunk_end > bytes.len() || chunk_size < 4 {
        return Err(WbhdfError::UnsupportedLayout(
            "continuation chunk extends beyond container bytes".to_string(),
        ));
    }

    if &bytes[chunk_start..chunk_start + 4] != b"OCHK" {
        return Err(WbhdfError::UnsupportedLayout(
            "continuation chunk missing OCHK signature".to_string(),
        ));
    }

    let messages = parse_chunk_messages(bytes, chunk_start + 4, chunk_end);
    let continuations = parse_header_continuations(bytes, &messages);
    let layouts = parse_layout_messages(bytes, &messages);

    Ok(ContinuationChunk {
        address: continuation.address,
        size: continuation.size,
        messages,
        continuations,
        layouts,
    })
}

fn parse_chunk_messages(
    bytes: &[u8],
    message_start: usize,
    chunk_end: usize,
) -> Vec<ObjectHeaderMessageHeader> {
    let mut messages = Vec::new();
    let mut cursor = message_start;

    while cursor + 4 <= bytes.len() && cursor + 4 <= chunk_end {
        let type_id = bytes[cursor];
        let size = u16::from_le_bytes([bytes[cursor + 1], bytes[cursor + 2]]);
        let flags = bytes[cursor + 3];
        let data_offset = cursor + 4;
        let next_cursor = data_offset.saturating_add(size as usize);

        if size == 0 && type_id == 0 {
            break;
        }
        if next_cursor > bytes.len() || next_cursor > chunk_end {
            break;
        }

        messages.push(ObjectHeaderMessageHeader {
            type_id,
            size,
            flags,
            header_offset: cursor,
            data_offset,
        });

        cursor = next_cursor;
    }

    messages
}

fn parse_layout_messages(bytes: &[u8], messages: &[ObjectHeaderMessageHeader]) -> Vec<LayoutMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x08 && message.size >= 18)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let layout_class = bytes[message.data_offset + 1];
            let data_address = u64::from_le_bytes(
                bytes[message.data_offset + 2..message.data_offset + 10]
                    .try_into()
                    .ok()?,
            );
            let data_size = u64::from_le_bytes(
                bytes[message.data_offset + 10..message.data_offset + 18]
                    .try_into()
                    .ok()?,
            );

            Some(LayoutMessage {
                version,
                layout_class,
                data_address,
                data_size,
            })
        })
        .collect()
}

fn parse_v1_dataspace_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<DataspaceMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x0001 && message.size >= 8)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let rank = bytes[message.data_offset + 1];
            let flags = bytes[message.data_offset + 2];
            let mut cursor = message.data_offset + 8;

            let mut dimensions = Vec::new();
            for _ in 0..rank {
                if cursor + 8 > body_end {
                    return None;
                }
                dimensions.push(u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?));
                cursor += 8;
            }

            let mut max_dimensions = Vec::new();
            if flags & 0x01 != 0 {
                for _ in 0..rank {
                    if cursor + 8 > body_end {
                        return None;
                    }
                    max_dimensions.push(u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?));
                    cursor += 8;
                }
            }

            Some(DataspaceMessage {
                version,
                rank,
                flags,
                dimensions,
                max_dimensions,
            })
        })
        .collect()
}

fn parse_v1_datatype_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<DatatypeMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x0003 && message.size >= 8)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let first = bytes[message.data_offset];
            Some(DatatypeMessage {
                version: first >> 4,
                class: first & 0x0f,
                bit_field_0: bytes[message.data_offset + 1],
                bit_field_1: bytes[message.data_offset + 2],
                bit_field_2: bytes[message.data_offset + 3],
                size: u32::from_le_bytes(
                    bytes[message.data_offset + 4..message.data_offset + 8]
                        .try_into()
                        .ok()?,
                ),
            })
        })
        .collect()
}

fn parse_v1_filter_pipeline_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<FilterPipelineMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x000b && message.size >= 8)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let num_filters = bytes[message.data_offset + 1] as usize;
            let mut cursor = message.data_offset + 8;
            let mut filters = Vec::new();

            for _ in 0..num_filters {
                if cursor + 8 > body_end {
                    return None;
                }
                let id = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
                let name_len = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]) as usize;
                let flags = u16::from_le_bytes([bytes[cursor + 4], bytes[cursor + 5]]);
                let num_cd = u16::from_le_bytes([bytes[cursor + 6], bytes[cursor + 7]]) as usize;
                cursor += 8;

                if cursor + name_len > body_end {
                    return None;
                }
                let name = String::from_utf8_lossy(&bytes[cursor..cursor + name_len])
                    .trim_end_matches('\0')
                    .to_string();
                cursor += name_len;
                if cursor % 8 != 0 {
                    cursor += 8 - (cursor % 8);
                }

                let mut client_data = Vec::new();
                for _ in 0..num_cd {
                    if cursor + 4 > body_end {
                        return None;
                    }
                    client_data.push(u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?));
                    cursor += 4;
                }
                if cursor % 8 != 0 {
                    cursor += 8 - (cursor % 8);
                }

                filters.push(FilterDescription {
                    id,
                    name,
                    flags,
                    client_data,
                });
            }

            Some(FilterPipelineMessage { version, filters })
        })
        .collect()
}

fn parse_v1_chunked_layout_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<ChunkedLayoutMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x0008 && message.size >= 11)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let layout_class = bytes[message.data_offset + 1];
            let num_dimensions = bytes[message.data_offset + 2];
            let index_address = u64::from_le_bytes(
                bytes[message.data_offset + 3..message.data_offset + 11]
                    .try_into()
                    .ok()?,
            );
            let mut cursor = message.data_offset + 11;
            let mut chunk_dimensions = Vec::new();
            for _ in 0..num_dimensions {
                if cursor + 4 > body_end {
                    return None;
                }
                chunk_dimensions.push(u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().ok()?));
                cursor += 4;
            }

            Some(ChunkedLayoutMessage {
                version,
                layout_class,
                num_dimensions,
                index_address,
                chunk_dimensions,
            })
        })
        .collect()
}

fn parse_v1_continuations(
    bytes: &[u8],
    messages: &[ObjectHeaderV1MessageHeader],
) -> Vec<ObjectHeaderContinuation> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x0010 && message.size >= 16)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let address = u64::from_le_bytes(bytes[message.data_offset..message.data_offset + 8].try_into().ok()?);
            let size = u64::from_le_bytes(bytes[message.data_offset + 8..message.data_offset + 16].try_into().ok()?);
            Some(ObjectHeaderContinuation { address, size })
        })
        .collect()
}

fn parse_dataspace_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderMessageHeader],
) -> Vec<DataspaceMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x01 && message.size >= 4)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let version = bytes[message.data_offset];
            let rank = bytes[message.data_offset + 1];
            let flags = bytes[message.data_offset + 2];
            let mut cursor = message.data_offset + 4;

            let mut dimensions = Vec::new();
            for _ in 0..rank {
                if cursor + 8 > body_end {
                    return None;
                }
                dimensions.push(u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?));
                cursor += 8;
            }

            let mut max_dimensions = Vec::new();
            if flags & 0x01 != 0 {
                for _ in 0..rank {
                    if cursor + 8 > body_end {
                        return None;
                    }
                    max_dimensions.push(u64::from_le_bytes(bytes[cursor..cursor + 8].try_into().ok()?));
                    cursor += 8;
                }
            }

            Some(DataspaceMessage {
                version,
                rank,
                flags,
                dimensions,
                max_dimensions,
            })
        })
        .collect()
}

fn parse_datatype_messages(
    bytes: &[u8],
    messages: &[ObjectHeaderMessageHeader],
) -> Vec<DatatypeMessage> {
    messages
        .iter()
        .filter(|message| message.type_id == 0x03 && message.size >= 8)
        .filter_map(|message| {
            let body_end = message.data_offset.saturating_add(message.size as usize);
            if body_end > bytes.len() {
                return None;
            }

            let first = bytes[message.data_offset];
            Some(DatatypeMessage {
                version: first >> 4,
                class: first & 0x0f,
                bit_field_0: bytes[message.data_offset + 1],
                bit_field_1: bytes[message.data_offset + 2],
                bit_field_2: bytes[message.data_offset + 3],
                size: u32::from_le_bytes(
                    bytes[message.data_offset + 4..message.data_offset + 8]
                        .try_into()
                        .ok()?,
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        find_object_header_signatures, parse_continuation_chunk, parse_v1_object_header,
        read_contiguous_layout_bytes_in_file, ChunkedLayoutMessage, DataspaceMessage,
        DatatypeMessage, FillValueMessage, FilterDescription, LayoutMessage, ObjectHeader,
        ObjectHeaderContinuation,
    };
    use std::fs;

    #[test]
    fn finds_object_header_signature_offsets() {
        let bytes = b"xxxxOHDRyyyOHDRzz";
        let offsets = find_object_header_signatures(bytes);
        assert_eq!(offsets, vec![4, 11]);
    }

    #[test]
    fn parse_rejects_missing_object_header_signatures() {
        let bytes = b"no signatures here";
        let err = ObjectHeader::parse(bytes).expect_err("parse should fail without OHDR signatures");
        let msg = format!("{err}");
        assert!(msg.contains("no object header signatures"));
    }

    #[test]
    fn parse_collects_valid_v2_headers_only() {
        let bytes = [
            b'x', b'x', b'x', b'x', b'O', b'H', b'D', b'R', 2, 0, 0x20, // valid v2
            b'O', b'H', b'D', b'R', 0x79, 0x08, 0x00, // invalid version, ignored
        ];

        let parsed = ObjectHeader::parse(&bytes).expect("parse should succeed with signatures");
        assert_eq!(parsed.signature_offsets, vec![4, 11]);
        assert_eq!(parsed.v2_headers.len(), 1);
        assert_eq!(parsed.v2_headers[0].offset, 4);
        assert_eq!(parsed.v2_headers[0].version, 2);
        assert_eq!(parsed.v2_headers[0].flags, 0);
        assert_eq!(parsed.v2_headers[0].prefix_len, 7);
        assert_eq!(parsed.v2_headers[0].chunk0_size, 0x20);
    }

    #[test]
    fn parse_extracts_v2_first_chunk_message_headers() {
        let bytes = [
            b'O', b'H', b'D', b'R', 2, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0x1c, // chunk0 size = 28 bytes (both message records after the prefix)
            0x01, 0x04, 0x00, 0x00, // message header: type 1, size 4, flags 0
            0xaa, 0xbb, 0xcc, 0xdd, // message body
            0x10, 0x10, 0x00, 0x01, // message header: continuation, size 16, flags 1
            1, 0, 0, 0, 0, 0, 0, 0, // continuation address
            8, 0, 0, 0, 0, 0, 0, 0, // continuation size
        ];

        let parsed = ObjectHeader::parse(&bytes).expect("parse should succeed");
        let header = &parsed.v2_headers[0];
        assert_eq!(header.prefix_len, 23);
        assert_eq!(header.chunk0_size, 28);
        assert_eq!(header.messages.len(), 2);
        assert_eq!(header.messages[0].type_id, 0x01);
        assert_eq!(header.messages[0].size, 4);
        assert_eq!(header.messages[0].flags, 0x00);
        assert_eq!(header.messages[0].header_offset, 23);
        assert_eq!(header.messages[0].data_offset, 27);
        assert_eq!(header.messages[1].type_id, 0x10);
        assert_eq!(header.continuations.len(), 1);
        assert_eq!(header.continuations[0].address, 1);
        assert_eq!(header.continuations[0].size, 8);
    }

    #[test]
    fn parse_extracts_dataspace_message() {
        let bytes = [
            b'O', b'H', b'D', b'R', 2, 0, 0x18,
            0x01, 0x14, 0x00, 0x00,
            0x02, 0x01, 0x01, 0x01,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let parsed = ObjectHeader::parse(&bytes).expect("parse should succeed");
        assert_eq!(parsed.v2_headers[0].dataspaces.len(), 1);
        assert_eq!(
            parsed.v2_headers[0].dataspaces[0],
            DataspaceMessage {
                version: 2,
                rank: 1,
                flags: 1,
                dimensions: vec![1],
                max_dimensions: vec![1],
            }
        );
    }

    #[test]
    fn parse_extracts_datatype_message() {
        let bytes = [
            b'O', b'H', b'D', b'R', 2, 0, 0x0c,
            0x03, 0x08, 0x00, 0x01,
            0x13, 0x01, 0x00, 0x00, 0x46, 0x97, 0x00, 0x00,
        ];

        let parsed = ObjectHeader::parse(&bytes).expect("parse should succeed");
        assert_eq!(parsed.v2_headers[0].datatypes.len(), 1);
        assert_eq!(
            parsed.v2_headers[0].datatypes[0],
            DatatypeMessage {
                version: 1,
                class: 3,
                bit_field_0: 0x01,
                bit_field_1: 0x00,
                bit_field_2: 0x00,
                size: 38_726,
            }
        );
    }

    #[test]
    fn parse_extracts_continuation_chunk_message_headers() {
        let bytes = [
            b'O', b'C', b'H', b'K',
            0x10, 0x10, 0x00, 0x00,
            0x07, 0x57, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x15, 0x12, 0x00, 0x04,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let continuation = ObjectHeaderContinuation {
            address: 0,
            size: bytes.len() as u64,
        };

        let chunk = parse_continuation_chunk(&bytes, &continuation).expect("chunk should parse");
        assert_eq!(chunk.messages.len(), 2);
        assert_eq!(chunk.messages[0].type_id, 0x10);
        assert_eq!(chunk.messages[1].type_id, 0x15);
        assert_eq!(chunk.continuations.len(), 1);
        assert_eq!(chunk.continuations[0].address, 0x25707);
        assert_eq!(chunk.continuations[0].size, 0xa0);
    }

    #[test]
    fn parse_extracts_layout_message_from_continuation_chunk() {
        let bytes = [
            b'O', b'C', b'H', b'K',
            0x08, 0x12, 0x00, 0x00,
            0x03, 0x01,
            0xaa, 0xb3, 0x81, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x46, 0x97, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let continuation = ObjectHeaderContinuation {
            address: 0,
            size: bytes.len() as u64,
        };

        let chunk = parse_continuation_chunk(&bytes, &continuation).expect("chunk should parse");
        assert_eq!(chunk.layouts.len(), 1);
        assert_eq!(
            chunk.layouts[0],
            LayoutMessage {
                version: 3,
                layout_class: 1,
                data_address: 8_500_138,
                data_size: 38_726,
            }
        );
    }

    #[test]
    fn reads_contiguous_layout_payload_bytes() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-layout-payload-test.bin");
        let mut bytes = vec![0_u8; 32];
        bytes[8..14].copy_from_slice(b"xml123");
        fs::write(&file_path, &bytes).unwrap();

        let payload = read_contiguous_layout_bytes_in_file(
            &file_path,
            &LayoutMessage {
                version: 3,
                layout_class: 1,
                data_address: 8,
                data_size: 6,
            },
        )
        .unwrap();
        assert_eq!(payload, b"xml123");

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn parse_extracts_v1_chunked_layout_and_filter_pipeline() {
        let bytes = [
            0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x00, 0x00, 0x00, 0, 0, 0, 0,
            0x0b, 0x00, 0x20, 0x00, 0x01, 0, 0, 0,
            0x01, 0x01, 0, 0, 0, 0, 0, 0,
            0x01, 0x00, 0x08, 0x00, 0x01, 0x00, 0x01, 0x00,
            b'd', b'e', b'f', b'l', b'a', b't', b'e', 0,
            0x06, 0x00, 0x00, 0x00, 0, 0, 0, 0,
            0x08, 0x00, 0x13, 0x00, 0x00, 0, 0, 0,
            0x03, 0x02, 0x02,
            0x71, 0xf9, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x10, 0x27, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00,
        ];

        let parsed = parse_v1_object_header(&bytes, 0).expect("v1 object header should parse");
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.filter_pipelines.len(), 1);
        assert_eq!(
            parsed.filter_pipelines[0].filters[0],
            FilterDescription {
                id: 1,
                name: "deflate".to_string(),
                flags: 1,
                client_data: vec![6],
            }
        );
        assert_eq!(parsed.chunked_layouts.len(), 1);
        assert_eq!(
            parsed.chunked_layouts[0],
            ChunkedLayoutMessage {
                version: 3,
                layout_class: 2,
                num_dimensions: 2,
                index_address: 326001,
                chunk_dimensions: vec![10000, 4],
            }
        );
    }

    #[test]
    fn parse_extracts_v1_fill_value_message() {
        let bytes = [
            0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0, 0, 0, 0,
            0x04, 0x00, 0x0c, 0x00, 0x01, 0, 0, 0,
            0x04, 0x02, 0x02, 0x01,
            0x04, 0x00, 0x00, 0x00,
            0xff, 0xff, 0x7f, 0x7f,
        ];

        let parsed = parse_v1_object_header(&bytes, 0).expect("v1 object header should parse");
        assert_eq!(parsed.fill_values.len(), 1);
        assert_eq!(
            parsed.fill_values[0],
            FillValueMessage {
                version: 4,
                allocation_time: 2,
                fill_time: 2,
                value_defined: 1,
                value_size: 4,
                value_bytes: vec![0xff, 0xff, 0x7f, 0x7f],
            }
        );
    }
}
