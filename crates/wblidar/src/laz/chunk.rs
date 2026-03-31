//! LAZ chunk-table reading and writing.
//!
//! The chunk table sits immediately before the compressed data and records the
//! byte offset of each chunk relative to the start of the first chunk.  This
//! enables O(1) random access to any chunk by index.

use std::io::{Read, Write};
use crate::io::le;
use crate::Result;

/// Upper bound for parsed chunk-table entries to avoid pathological allocations.
const MAX_CHUNK_COUNT: usize = 10_000_000;
/// Upper bound for a single compressed chunk size in bytes.
const MAX_COMPRESSED_CHUNK_BYTES: usize = 256 * 1024 * 1024; // 256 MiB

/// Version marker written at the start of the chunk table.
pub const CHUNK_TABLE_VERSION: u32 = 0;

/// A parsed LAZ chunk table.
#[derive(Debug, Clone, Default)]
pub struct ChunkTable {
    /// Byte offsets of each chunk (relative to the start of the first chunk).
    pub offsets: Vec<u64>,
}

impl ChunkTable {
    /// Read the chunk table from the current position.
    pub fn read<R: Read>(r: &mut R) -> Result<Self> {
        let _version    = le::read_u32(r)?;
        let chunk_count = le::read_u32(r)? as usize;
        if chunk_count > MAX_CHUNK_COUNT {
            return Err(crate::Error::InvalidValue {
                field: "laz_chunk_table.chunk_count",
                detail: format!(
                    "chunk_count {chunk_count} exceeds safety limit {MAX_CHUNK_COUNT}"
                ),
            });
        }

        let mut offsets = Vec::new();
        offsets
            .try_reserve(chunk_count)
            .map_err(|e| crate::Error::InvalidValue {
                field: "laz_chunk_table.chunk_count",
                detail: format!("failed reserving offset table for {chunk_count} chunks: {e}"),
            })?;
        for _ in 0..chunk_count {
            offsets.push(le::read_u64(r)?);
        }
        Ok(ChunkTable { offsets })
    }

    /// Write the chunk table at the current position.
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        le::write_u32(w, CHUNK_TABLE_VERSION)?;
        le::write_u32(w, self.offsets.len() as u32)?;
        for &off in &self.offsets { le::write_u64(w, off)?; }
        Ok(())
    }

    /// Byte size of this chunk table when serialised.
    pub fn serialised_size(&self) -> usize { 8 + self.offsets.len() * 8 }
}

/// Serialise a single compressed chunk: write a u64 chunk-body size prefix
/// then the compressed bytes.
pub fn write_compressed_chunk<W: Write>(w: &mut W, compressed: &[u8]) -> Result<u64> {
    let size = compressed.len() as u64;
    le::write_u64(w, size)?;
    w.write_all(compressed)?;
    Ok(8 + size) // bytes written (size prefix + body)
}

/// Read the u64 size prefix and return the compressed chunk bytes.
pub fn read_compressed_chunk<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let size = le::read_u64(r)? as usize;
    if size > MAX_COMPRESSED_CHUNK_BYTES {
        return Err(crate::Error::InvalidValue {
            field: "laz_chunk.size",
            detail: format!(
                "compressed chunk size {size} exceeds safety limit {MAX_COMPRESSED_CHUNK_BYTES}"
            ),
        });
    }

    let mut buf = Vec::new();
    buf.try_reserve_exact(size)
        .map_err(|e| crate::Error::InvalidValue {
            field: "laz_chunk.size",
            detail: format!("failed reserving compressed chunk buffer of {size} bytes: {e}"),
        })?;
    buf.resize(size, 0u8);
    r.read_exact(&mut buf)?;
    Ok(buf)
}
