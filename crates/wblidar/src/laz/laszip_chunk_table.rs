//! LASzip chunk-table locator/parser helpers.
//!
//! Standard LASzip streams store an 8-byte pointer at the beginning of LAZ point
//! data that references the chunk table location. The table itself starts with
//! a version and chunk count, followed by arithmetic-coded entry payload.

use std::io::{Read, Seek, SeekFrom, Write};

use crate::io::le;
use crate::laz::arithmetic_encoder::ArithmeticEncoder;
use crate::laz::arithmetic_decoder::ArithmeticDecoder;
use crate::laz::integer_codec::{IntegerCompressor, IntegerDecompressor};
use crate::Result;

/// Safety cap for chunk counts declared in standard LASzip chunk tables.
const MAX_STANDARD_CHUNK_COUNT: u32 = 100_000_000;

/// Parsed pointer metadata from the start of LAZ point data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaszipChunkTablePointer {
    /// File offset where the first compressed chunk payload starts.
    pub data_start: u64,
    /// Absolute file offset to the chunk table.
    pub chunk_table_offset: u64,
}

/// Parsed chunk-table header metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaszipChunkTableHeader {
    /// LASzip chunk-table version.
    pub version: u32,
    /// Number of chunk entries encoded in the table.
    pub chunk_count: u32,
}

/// A decoded standard LASzip chunk-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LaszipChunkTableEntry {
    /// Number of points in this chunk (when present in table encoding).
    pub point_count: u64,
    /// Number of compressed bytes in this chunk.
    pub byte_count: u64,
}

/// Write a standard LASzip chunk table at the current stream position.
///
/// Layout:
/// - `u32 version` (currently `0`)
/// - `u32 chunk_count`
/// - arithmetic-coded entry payload
pub fn write_laszip_chunk_table<W: Write>(
    writer: &mut W,
    entries: &[LaszipChunkTableEntry],
    contains_point_count: bool,
) -> Result<()> {
    if entries.len() > MAX_STANDARD_CHUNK_COUNT as usize {
        return Err(crate::Error::InvalidValue {
            field: "laz.chunk_count",
            detail: format!(
                "standard LASzip chunk_count {} exceeds safety limit {MAX_STANDARD_CHUNK_COUNT}",
                entries.len()
            ),
        });
    }

    for e in entries {
        if e.byte_count > u32::MAX as u64 {
            return Err(crate::Error::InvalidValue {
                field: "laz.chunk_byte_count",
                detail: format!("chunk byte_count {} exceeds u32::MAX", e.byte_count),
            });
        }
        if contains_point_count && e.point_count > u32::MAX as u64 {
            return Err(crate::Error::InvalidValue {
                field: "laz.chunk_point_count",
                detail: format!("chunk point_count {} exceeds u32::MAX", e.point_count),
            });
        }
    }

    le::write_u32(writer, 0)?;
    le::write_u32(writer, entries.len() as u32)?;

    if entries.is_empty() {
        return Ok(());
    }

    let mut encoded = Vec::<u8>::new();
    {
        let mut ar = ArithmeticEncoder::new(&mut encoded);
        let mut int_enc = IntegerCompressor::new(32, 2, 8, 0);
        let mut prev = LaszipChunkTableEntry::default();

        for e in entries {
            if contains_point_count {
                int_enc.compress(&mut ar, prev.point_count as i32, e.point_count as i32, 0)?;
            }
            int_enc.compress(&mut ar, prev.byte_count as i32, e.byte_count as i32, 1)?;
            prev = *e;
        }
        let _ = ar.done()?;
    }

    writer.write_all(&encoded)?;
    Ok(())
}

/// Read the LASzip chunk-table pointer from the start of point data.
///
/// Returns `Ok(None)` when the pointer is absent or clearly invalid.
pub fn read_laszip_chunk_table_pointer<R: Read + Seek>(
    reader: &mut R,
    data_start: u64,
    file_len: u64,
) -> Result<Option<LaszipChunkTablePointer>> {
    if file_len < 8 {
        return Ok(None);
    }

    reader.seek(SeekFrom::Start(data_start))?;
    let raw_ptr_start = i64::from_le_bytes(le::read_u64(reader)?.to_le_bytes());

    // LASzip writers may leave the start pointer unset and place a valid
    // chunk-table pointer in the last 8 bytes of the LAZ payload instead.
    let raw_ptr = if raw_ptr_start > data_start as i64 && (raw_ptr_start as u64) < file_len {
        raw_ptr_start
    } else {
        reader.seek(SeekFrom::Start(file_len - 8))?;
        let tail = i64::from_le_bytes(le::read_u64(reader)?.to_le_bytes());
        if tail <= data_start as i64 || (tail as u64) >= file_len {
            return Ok(None);
        }
        tail
    };

    if raw_ptr <= data_start as i64 {
        return Ok(None);
    }

    Ok(Some(LaszipChunkTablePointer {
        data_start,
        chunk_table_offset: raw_ptr as u64,
    }))
}

/// Read and validate the fixed chunk-table header at `chunk_table_offset`.
pub fn read_laszip_chunk_table_header<R: Read + Seek>(
    reader: &mut R,
    chunk_table_offset: u64,
    file_len: u64,
) -> Result<LaszipChunkTableHeader> {
    if chunk_table_offset + 8 > file_len {
        return Err(crate::Error::InvalidValue {
            field: "laz.chunk_table_offset",
            detail: format!(
                "chunk table offset {chunk_table_offset} leaves insufficient bytes for header"
            ),
        });
    }

    reader.seek(SeekFrom::Start(chunk_table_offset))?;
    let version = le::read_u32(reader)?;
    let chunk_count = le::read_u32(reader)?;

    if chunk_count > MAX_STANDARD_CHUNK_COUNT {
        return Err(crate::Error::InvalidValue {
            field: "laz.chunk_count",
            detail: format!(
                "standard LASzip chunk_count {chunk_count} exceeds safety limit {MAX_STANDARD_CHUNK_COUNT}"
            ),
        });
    }

    Ok(LaszipChunkTableHeader {
        version,
        chunk_count,
    })
}

/// Decode chunk-table entries at the current stream position.
///
/// Stream position must be immediately after `(version, chunk_count)`.
/// If `contains_point_count` is false, `point_count` fields are left as zero.
pub fn read_laszip_chunk_table_entries<R: Read>(
    reader: &mut R,
    chunk_count: u32,
    contains_point_count: bool,
) -> Result<Vec<LaszipChunkTableEntry>> {
    if chunk_count == 0 {
        return Ok(Vec::new());
    }

    // Check if chunk_count is unreasonably large  
    if chunk_count > 1_000_000 {
        return Err(crate::Error::InvalidValue {
            field: "laz.chunk_count",
            detail: format!(
                "chunk_count {chunk_count} is unreasonably large"
            ),
        });
    }

    let mut dec = ArithmeticDecoder::new(reader);
    dec.read_init_bytes().map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            crate::Error::InvalidValue {
                field: "laz.chunk_table_entries",
                detail: "insufficient bytes to read chunk table entries (arithmetic decoder init failed)".to_string(),
            }
        } else {
            crate::Error::Io(e)
        }
    })?;

    let mut int_dec = IntegerDecompressor::new(32, 2, 8, 0);
    let mut prev = LaszipChunkTableEntry::default();
    let mut out = Vec::with_capacity(chunk_count as usize);

    for _ in 0..chunk_count {
        let mut cur = LaszipChunkTableEntry::default();

        if contains_point_count {
            let v = int_dec.decompress(&mut dec, prev.point_count as i32, 0)?;
            cur.point_count = u64::from_le(v as u32 as u64);
        }

        let b = int_dec.decompress(&mut dec, prev.byte_count as i32, 1)?;
        cur.byte_count = u64::from_le(b as u32 as u64);

        out.push(cur);
        prev = cur;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::laz::arithmetic_encoder::ArithmeticEncoder;
    use crate::laz::integer_codec::IntegerCompressor;
    use super::{
        read_laszip_chunk_table_entries,
        read_laszip_chunk_table_header,
        read_laszip_chunk_table_pointer,
        LaszipChunkTableEntry,
        LaszipChunkTableHeader,
        LaszipChunkTablePointer,
    };

    #[test]
    fn parses_valid_pointer() {
        let mut bytes = vec![0u8; 128];
        bytes[16..24].copy_from_slice(&(96i64).to_le_bytes());
        let mut cur = Cursor::new(bytes);
        let ptr = read_laszip_chunk_table_pointer(&mut cur, 16, 128)
            .expect("pointer parse should succeed")
            .expect("pointer should be present");

        assert_eq!(
            ptr,
            LaszipChunkTablePointer {
                data_start: 16,
                chunk_table_offset: 96,
            }
        );
    }

    #[test]
    fn rejects_non_forward_pointer() {
        let mut bytes = vec![0u8; 64];
        bytes[8..16].copy_from_slice(&(8i64).to_le_bytes());
        let mut cur = Cursor::new(bytes);
        let ptr = read_laszip_chunk_table_pointer(&mut cur, 8, 64)
            .expect("pointer parse should succeed");
        assert!(ptr.is_none());
    }

    #[test]
    fn parses_tail_pointer_when_start_pointer_invalid() {
        let mut bytes = vec![0u8; 128];
        // Invalid at start (not forward from data_start).
        bytes[16..24].copy_from_slice(&(16i64).to_le_bytes());
        // Valid tail pointer.
        bytes[120..128].copy_from_slice(&(96i64).to_le_bytes());

        let mut cur = Cursor::new(bytes);
        let ptr = read_laszip_chunk_table_pointer(&mut cur, 16, 128)
            .expect("pointer parse should succeed")
            .expect("tail pointer should be detected");

        assert_eq!(
            ptr,
            LaszipChunkTablePointer {
                data_start: 16,
                chunk_table_offset: 96,
            }
        );
    }

    #[test]
    fn parses_chunk_table_header() {
        let mut bytes = vec![0u8; 128];
        bytes[80..84].copy_from_slice(&0u32.to_le_bytes());
        bytes[84..88].copy_from_slice(&42u32.to_le_bytes());
        let mut cur = Cursor::new(bytes);

        let header = read_laszip_chunk_table_header(&mut cur, 80, 128)
            .expect("chunk table header parse should succeed");
        assert_eq!(
            header,
            LaszipChunkTableHeader {
                version: 0,
                chunk_count: 42,
            }
        );
    }

    #[test]
    fn decodes_chunk_table_entries_with_point_counts() {
        let entries = vec![
            LaszipChunkTableEntry {
                point_count: 100,
                byte_count: 4096,
            },
            LaszipChunkTableEntry {
                point_count: 80,
                byte_count: 3500,
            },
            LaszipChunkTableEntry {
                point_count: 120,
                byte_count: 5000,
            },
        ];

        let mut encoded = Cursor::new(Vec::<u8>::new());
        {
            let mut ar = ArithmeticEncoder::new(&mut encoded);
            let mut int_enc = IntegerCompressor::new(32, 2, 8, 0);
            let mut prev = LaszipChunkTableEntry::default();
            for e in &entries {
                int_enc
                    .compress(&mut ar, prev.point_count as i32, e.point_count as i32, 0)
                    .expect("encode point_count");
                int_enc
                    .compress(&mut ar, prev.byte_count as i32, e.byte_count as i32, 1)
                    .expect("encode byte_count");
                prev = *e;
            }
            let _ = ar.done().expect("done arithmetic stream");
        }

        encoded.set_position(0);
        let decoded = read_laszip_chunk_table_entries(
            &mut encoded,
            entries.len() as u32,
            true,
        )
        .expect("decode chunk table entries");

        assert_eq!(decoded, entries);
    }

    #[test]
    fn decodes_chunk_table_entries_without_point_counts() {
        let byte_counts = [1200u64, 1400u64, 900u64, 2200u64];

        let mut encoded = Cursor::new(Vec::<u8>::new());
        {
            let mut ar = ArithmeticEncoder::new(&mut encoded);
            let mut int_enc = IntegerCompressor::new(32, 2, 8, 0);
            let mut prev = 0i32;
            for b in byte_counts {
                int_enc
                    .compress(&mut ar, prev, b as i32, 1)
                    .expect("encode byte_count");
                prev = b as i32;
            }
            let _ = ar.done().expect("done arithmetic stream");
        }

        encoded.set_position(0);
        let decoded = read_laszip_chunk_table_entries(
            &mut encoded,
            byte_counts.len() as u32,
            false,
        )
        .expect("decode chunk table entries");

        let got: Vec<u64> = decoded.iter().map(|e| e.byte_count).collect();
        assert_eq!(got, byte_counts);
        assert!(decoded.iter().all(|e| e.point_count == 0));
    }
}
