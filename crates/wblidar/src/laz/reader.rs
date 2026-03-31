//! LAZ streaming reader.

use std::io::{ErrorKind, Read, Seek, SeekFrom};

#[cfg(feature = "laz-parallel")]
use std::sync::OnceLock;

use crate::crs::Crs;
use crate::io::{le, PointReader};
use crate::las::PointDataFormat;
use crate::las::LasReader;
use crate::laz::chunk::{read_compressed_chunk, ChunkTable, CHUNK_TABLE_VERSION};
use crate::laz::laszip_chunk_table::{
    LaszipChunkTableEntry,
    read_laszip_chunk_table_entries,
    read_laszip_chunk_table_header,
    read_laszip_chunk_table_pointer,
};
use crate::laz::standard_point10::decode_standard_pointwise_chunk_point10_v2;
use crate::laz::standard_point14::decode_standard_layered_chunk_point14_v3_with_status;
use crate::laz::{
    parse_laszip_vlr,
    LaszipItemSpec,
    parse_vlr_chunk_size,
    LaszipCompressorType,
    DEFAULT_CHUNK_SIZE,
};
use crate::point::PointRecord;
use crate::Result;

#[cfg(feature = "laz-parallel")]
use rayon::prelude::*;

#[cfg(feature = "laz-parallel")]
// Thresholds tuned for multi-chunk parallel decompression: only spawn threads when cost is justified.
// Tuned via automated benchmarking on representative LAZ files (variable chunk counts).
const DEFAULT_PARALLEL_CHUNK_DECODE_MIN_CHUNKS: usize = 4;
#[cfg(feature = "laz-parallel")]
const DEFAULT_PARALLEL_CHUNK_DECODE_MIN_POINTS: usize = 200_000;

#[cfg(feature = "laz-parallel")]
fn parse_env_usize(var: &str, default: usize) -> usize {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

#[cfg(feature = "laz-parallel")]
fn parallel_chunk_decode_min_chunks() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| {
        parse_env_usize(
            "WBLIDAR_LAZ_PARALLEL_MIN_CHUNKS",
            DEFAULT_PARALLEL_CHUNK_DECODE_MIN_CHUNKS,
        )
    })
}

#[cfg(feature = "laz-parallel")]
fn parallel_chunk_decode_min_points() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| {
        parse_env_usize(
            "WBLIDAR_LAZ_PARALLEL_MIN_POINTS",
            DEFAULT_PARALLEL_CHUNK_DECODE_MIN_POINTS,
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StandardChunkTableContext {
    data_start: u64,
    chunk_table_offset: u64,
    file_len: u64,
    total_points: u64,
    chunk_size: u32,
}

fn validate_standard_chunk_table_entries(
    entries: &[LaszipChunkTableEntry],
    contains_point_count: bool,
    ctx: StandardChunkTableContext,
) -> bool {
    if entries.is_empty() {
        return false;
    }

    let payload_start = ctx.data_start.saturating_add(8);
    if payload_start > ctx.chunk_table_offset || ctx.chunk_table_offset > ctx.file_len {
        return false;
    }
    let payload_bytes = ctx.chunk_table_offset.saturating_sub(payload_start);
    if payload_bytes == 0 {
        return false;
    }

    let mut total_bytes = 0u64;
    let mut nonzero_chunks = 0usize;
    for entry in entries {
        if entry.byte_count > payload_bytes {
            return false;
        }
        total_bytes = match total_bytes.checked_add(entry.byte_count) {
            Some(v) => v,
            None => return false,
        };
        if total_bytes > payload_bytes {
            return false;
        }
        if entry.byte_count > 0 {
            nonzero_chunks += 1;
        }
    }

    if nonzero_chunks == 0 || total_bytes == 0 {
        return false;
    }

    if contains_point_count {
        let total_entry_points = entries.iter().map(|e| e.point_count).sum::<u64>();
        if total_entry_points == 0 {
            return false;
        }
        if ctx.total_points > 0 {
            let chunk_slack = u64::from(ctx.chunk_size.max(1));
            let min_expected = ctx.total_points.saturating_sub(chunk_slack);
            let max_expected = ctx.total_points.saturating_add(chunk_slack);
            if total_entry_points < min_expected || total_entry_points > max_expected {
                return false;
            }
        }
    }

    true
}

fn read_standard_chunk_table_entries_with_recovery<R: Read + Seek>(
    reader: &mut R,
    chunk_table_offset: u64,
    chunk_count: u32,
    preferred_contains_point_count: bool,
    ctx: StandardChunkTableContext,
) -> Option<Vec<LaszipChunkTableEntry>> {
    let decode = |reader: &mut R, contains_point_count: bool| -> Option<Vec<LaszipChunkTableEntry>> {
        reader.seek(SeekFrom::Start(chunk_table_offset + 8)).ok()?;
        let entries = read_laszip_chunk_table_entries(reader, chunk_count, contains_point_count).ok()?;
        validate_standard_chunk_table_entries(&entries, contains_point_count, ctx).then_some(entries)
    };

    decode(reader, preferred_contains_point_count)
        .or_else(|| decode(reader, !preferred_contains_point_count))
}

/// A streaming LAZ reader. Decompresses one chunk at a time on demand.
pub struct LazReader<R: Read + Seek> {
    las: LasReader<R>,
    chunk_table: ChunkTable,
    chunk_size: u32,
    /// Byte offset of the first compressed chunk (immediately after the chunk table).
    data_start: u64,
    /// Sequential-mode cursor: byte offset of the next chunk to read.
    /// Only used when `chunk_table.offsets` is empty (non-indexed / count=0 mode).
    next_chunk_offset: u64,
    /// Currently buffered decoded points from the last decompressed chunk.
    buffer: Vec<PointRecord>,
    buf_pos: usize,
    /// Lazy standard LASzip Point10 chunk decoding state.
    standard_point10_lazy: Option<StandardPoint10LazyState>,
    /// Fully decoded point cache for supported standard LASzip streams.
    standard_points: Option<Vec<PointRecord>>,
    /// Indicates LASzip metadata declares Point14 core item.
    declared_point14_standard: bool,
    standard_pos: usize,
    total_read: u64,
    point14_partial_events: u64,
    point14_partial_decoded_points: u64,
    point14_partial_expected_points: u64,
}

#[derive(Debug, Clone)]
struct StandardPoint10LazyState {
    entries: Vec<LaszipChunkTableEntry>,
    item_specs: Vec<LaszipItemSpec>,
    point_data_format: PointDataFormat,
    expected_extra_bytes_count: usize,
    scales: [f64; 3],
    offsets: [f64; 3],
    chunk_data_offset: u64,
    entry_index: usize,
}

impl<R: Read + Seek> LazReader<R> {
    /// Open a LAZ file, parse the header and VLRs, read chunk metadata, and
    /// position the reader ready to stream points.
    pub fn new(inner: R) -> Result<Self> {
        let mut las = LasReader::new(inner)?;
        let mut standard_points: Option<Vec<PointRecord>> = None;
        let mut standard_point10_lazy: Option<StandardPoint10LazyState> = None;
        let mut point14_partial_events = 0u64;
        let mut point14_partial_decoded_points = 0u64;
        let mut point14_partial_expected_points = 0u64;

        // Extract chunk size from LASzip VLR (fall back to standard default).
        let chunk_size = parse_vlr_chunk_size(las.vlrs()).unwrap_or(DEFAULT_CHUNK_SIZE);
        let laszip_info = parse_laszip_vlr(las.vlrs());

        // Cache immutable header values used in potential standard decode path.
        let total_points = las.header().point_count();
        let point_data_format = las.header().point_data_format;
        let expected_extra_bytes_count = las.header().extra_bytes_count as usize;
        let scales = [las.header().x_scale, las.header().y_scale, las.header().z_scale];
        let offsets = [las.header().x_offset, las.header().y_offset, las.header().z_offset];

        // After LasReader::new the inner stream is positioned at offset_to_point_data.
        let chunk_table_pos = las.offset_to_point_data();

        let file_len = {
            let inner = las.inner_mut();
            let cur = inner.stream_position()?;
            let end = inner.seek(SeekFrom::End(0))?;
            inner.seek(SeekFrom::Start(cur))?;
            end
        };

        let (chunk_table, data_start) = {
            let inner = las.inner_mut();
            inner.seek(SeekFrom::Start(chunk_table_pos))?;

            // Peek first 8 bytes and check wb-native chunk table prefix.
            let first_word = match (le::read_u32(inner), le::read_u32(inner)) {
                (Ok(version), Ok(chunk_count_u32)) => {
                    u64::from(version) | ((chunk_count_u32 as u64) << 32)
                }
                _ => {
                    // I/O error reading at chunk_table_pos - fall through to standard path
                    u64::MAX
                }
            };
            
            let (version, chunk_count) = (
                ((first_word & 0xFFFF_FFFF) as u32),
                ((first_word >> 32) as u32) as u64,
            );
            let table_bytes = 8u64.saturating_add(chunk_count.saturating_mul(8));
            let table_fits = chunk_table_pos.saturating_add(table_bytes) <= file_len;

            if version == CHUNK_TABLE_VERSION && table_fits && first_word != u64::MAX {
                inner.seek(SeekFrom::Start(chunk_table_pos))?;
                let table = ChunkTable::read(inner)?;
                let start = chunk_table_pos + table.serialised_size() as u64;
                (table, start)
            } else {
                // Attempt standard LASzip chunk-table pointer + chunk table parse.
                let standard_pointer =
                    read_laszip_chunk_table_pointer(inner, chunk_table_pos, file_len).ok().flatten();
                let parsed_standard_table = if let Some(ptr) = standard_pointer {
                    if let Ok(header) =
                        read_laszip_chunk_table_header(inner, ptr.chunk_table_offset, file_len)
                    {
                        // Standard LASzip only encodes per-chunk point counts in
                        // the chunk table when chunk_size == u32::MAX (variable-size
                        // chunks, e.g. COPC).  For fixed chunk-size streams the
                        // external LASzip decoder reads byte-counts only.
                        let contains_point_count = laszip_info
                            .as_ref()
                            .map(|info| info.chunk_size == u32::MAX)
                            .unwrap_or(false);

                        let entries = read_standard_chunk_table_entries_with_recovery(
                            inner,
                            ptr.chunk_table_offset,
                            header.chunk_count,
                            contains_point_count,
                            StandardChunkTableContext {
                                data_start: ptr.data_start,
                                chunk_table_offset: ptr.chunk_table_offset,
                                file_len,
                                total_points,
                                chunk_size,
                            },
                        );
                        entries.map(|e| (ptr, e))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let declared_standard_laszip = laszip_info
                    .as_ref()
                    .map(|info| {
                        info.uses_arithmetic_coder()
                            && matches!(
                                info.compressor,
                                LaszipCompressorType::PointWise
                                    | LaszipCompressorType::PointWiseChunked
                                    | LaszipCompressorType::LayeredChunked
                            )
                    })
                    .unwrap_or(false);

                if let Some((ptr, entries)) = parsed_standard_table.as_ref() {
                    if let Some(info) = laszip_info.as_ref() {
                        if !info.has_point14_item() && !info.has_point10_item() {
                            return Err(crate::Error::Unimplemented(
                                "standard LASzip LAZ stream detected, but only Point10/Point14 item layouts are currently targeted in wblidar standard backend",
                            ));
                        }

                        if info.has_point10_item() && !info.has_point14_item() {
                            standard_point10_lazy = Some(StandardPoint10LazyState {
                                entries: entries.clone(),
                                item_specs: info.items.clone(),
                                point_data_format,
                                expected_extra_bytes_count,
                                scales,
                                offsets,
                                chunk_data_offset: ptr.data_start + 8,
                                entry_index: 0,
                            });
                        } else if info.has_point14_item() {
                            // Attempt to read entire Point14 file at once using layered decoding.
                            let mut decoded = Vec::new();
                            if let Ok(hint) = usize::try_from(total_points) {
                                let _ = decoded.try_reserve(hint);
                            }
                            let mut chunk_data_offset = ptr.data_start + 8;
                            for entry in entries {
                                if total_points > 0 && decoded.len() as u64 >= total_points {
                                    break;
                                }
                                if entry.byte_count == 0 {
                                    continue;
                                }
                                let remaining = if total_points > 0 {
                                    total_points.saturating_sub(decoded.len() as u64)
                                } else {
                                    u64::from(chunk_size)
                                };
                                let points_in_chunk = if entry.point_count > 0 {
                                    entry.point_count.min(remaining) as usize
                                } else {
                                    u64::from(chunk_size).min(remaining) as usize
                                };
                                if points_in_chunk == 0 {
                                    break;
                                }

                                let chunk_byte_count = usize::try_from(entry.byte_count).map_err(|_| {
                                    crate::Error::InvalidValue {
                                        field: "laz.standard_chunk_byte_count",
                                        detail: format!("chunk byte_count {} does not fit into usize", entry.byte_count),
                                    }
                                })?;

                                let mut chunk_bytes = vec![0u8; chunk_byte_count];
                                inner.seek(SeekFrom::Start(chunk_data_offset))?;
                                inner.read_exact(&mut chunk_bytes)?;

                                let (mut chunk_points, already_scaled) = match decode_standard_layered_chunk_point14_v3_with_status(
                                    &chunk_bytes,
                                    points_in_chunk,
                                    &info.items,
                                    point_data_format,
                                    scales,
                                    offsets,
                                ) {
                                    Ok((points, status)) => {
                                        if status.partial && fail_on_partial_point14() {
                                            return Err(crate::Error::InvalidValue {
                                                field: "laz.point14.partial",
                                                detail: format!(
                                                    "decoded {} of {} points in strict partial-check mode",
                                                    status.decoded_points, status.expected_points
                                                ),
                                            });
                                        }
                                        if status.partial {
                                            point14_partial_events += 1;
                                            point14_partial_decoded_points += status.decoded_points as u64;
                                            point14_partial_expected_points += status.expected_points as u64;
                                        }
                                        (points, true)
                                    }
                                    Err(e) => return Err(e),
                                };

                                if !already_scaled {
                                    for pt in &mut chunk_points {
                                        pt.x = pt.x * scales[0] + offsets[0];
                                        pt.y = pt.y * scales[1] + offsets[1];
                                        pt.z = pt.z * scales[2] + offsets[2];
                                    }
                                }

                                decoded.append(&mut chunk_points);
                                chunk_data_offset = chunk_data_offset.saturating_add(entry.byte_count);
                            }

                            standard_points = Some(decoded);
                        } else {
                            return Err(crate::Error::Unimplemented(
                                "standard LASzip LAZ stream detected, but this item layout is not yet implemented in wblidar standard backend",
                            ));
                        }
                    } else {
                        return Err(crate::Error::Unimplemented(
                            "standard LASzip LAZ stream detected, but LASzip VLR metadata is missing",
                        ));
                    }

                    // Successfully parsed and decoded: return default table with data start offset
                    (ChunkTable::default(), ptr.data_start)
                } else if declared_standard_laszip && laszip_info.as_ref().map(|i| i.has_point14_item()).unwrap_or(false) {
                    // For LayeredChunked Point14 streams, even if we can't parse the chunk table,
                    // report the actual limitation (arithmetic decoding not implemented) rather than chunk-table error.
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip Point14 layered stream detected, but arithmetic layered decoding is not yet implemented in wblidar standard backend",
                    ));
                } else if declared_standard_laszip {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip LAZ stream detected but chunk table could not be parsed",
                    ));
                } else {
                    // Keep compatibility with legacy heuristic for malformed metadata.
                    let looks_like_standard_chunk_table_ptr =
                        first_word > chunk_table_pos && first_word < file_len;
                    if looks_like_standard_chunk_table_ptr {
                        return Err(crate::Error::Unimplemented(
                            "standard LASzip LAZ stream detected but could not be parsed",
                        ));
                    }

                    // No upfront chunk table: read wb-native chunks sequentially.
                    (ChunkTable::default(), chunk_table_pos)
                }
            }
        };

        Ok(LazReader {
            las,
            chunk_table,
            chunk_size,
            data_start,
            next_chunk_offset: data_start,
            buffer: Vec::new(),
            buf_pos: 0,
            standard_point10_lazy,
            standard_points,
            declared_point14_standard: laszip_info
                .as_ref()
                .map(|i| i.has_point14_item() && i.uses_arithmetic_coder())
                .unwrap_or(false),
            standard_pos: 0,
            total_read: 0,
            point14_partial_events,
            point14_partial_decoded_points,
            point14_partial_expected_points,
        })
    }

    /// Borrow CRS metadata extracted from the LAS projection VLRs.
    pub fn crs(&self) -> Option<&Crs> {
        self.las.crs()
    }

    /// Return aggregate Point14 partial-recovery counters for this reader.
    ///
    /// Tuple fields are `(events, decoded_points, expected_points)`.
    pub fn point14_partial_recovery_stats(&self) -> (u64, u64, u64) {
        (
            self.point14_partial_events,
            self.point14_partial_decoded_points,
            self.point14_partial_expected_points,
        )
    }

    /// Decode all chunks in parallel and return a vec of all points (Point10 lazy path only).
    #[cfg(feature = "laz-parallel")]
    pub fn read_all_points_parallel(&mut self) -> Result<Vec<PointRecord>> {
        // Only implemented for Point10 lazy streaming mode (standard LASzip).
        let state = self.standard_point10_lazy.as_ref()
            .ok_or_else(|| crate::Error::Unimplemented(
                "read_all_points_parallel only supports Point10 lazy (standard LASzip) streams"
            ))?;

        // Decide whether parallelization is worthwhile.
        let nonzero_chunks = state.entries.iter().filter(|e| e.byte_count > 0).count();
        let total_points: u64 = state.entries.iter().map(|e| e.point_count).sum();

        if nonzero_chunks < parallel_chunk_decode_min_chunks() 
            || total_points < parallel_chunk_decode_min_points() as u64
        {
            // Fall back to serial decoding.
            return self.read_all_points_serial();
        }

        // Read all chunk bytes into memory upfront.
        let mut chunk_bytes_list: Vec<Vec<u8>> = Vec::new();
        {
            for entry in &state.entries {
                if entry.byte_count == 0 {
                    chunk_bytes_list.push(Vec::new());
                } else {
                    let chunk_byte_count = usize::try_from(entry.byte_count).map_err(|_| {
                        crate::Error::InvalidValue {
                            field: "laz.parallel_chunk_byte_count",
                            detail: format!("chunk byte_count {} does not fit into usize", entry.byte_count),
                        }
                    })?;
                    let mut chunk_bytes = vec![0u8; chunk_byte_count];
                    self.las.inner_mut().seek(SeekFrom::Start(
                        state.chunk_data_offset + chunk_bytes_list.iter().map(|c| c.len() as u64).sum::<u64>()
                    ))?;
                    self.las.inner_mut().read_exact(&mut chunk_bytes)?;
                    chunk_bytes_list.push(chunk_bytes);
                }
            }
        }

        // Parallel decode of all chunks.
        let shared_state = (
            state.item_specs.clone(),
            state.point_data_format,
            state.expected_extra_bytes_count,
            state.scales,
            state.offsets,
        );

        let total_points_for_header = self.las.header().point_count();
        let chunk_size = self.chunk_size;

        let decoded_chunks: Result<Vec<Vec<PointRecord>>> = chunk_bytes_list
            .into_par_iter()
            .zip(state.entries.par_iter())
            .enumerate()
            .try_fold(
                || Vec::new(),
                |mut acc, (_, (chunk_bytes, entry))| {
                    if chunk_bytes.is_empty() {
                        acc.push(Vec::new());
                        Ok(acc)
                    } else {
                        let (item_specs, pdf, extra_bytes, scales, offsets) = &shared_state;
                        let remaining = if total_points_for_header > 0 {
                            total_points_for_header.saturating_sub(acc.len() as u64)
                        } else {
                            u64::from(chunk_size)
                        };
                        let points_in_chunk = if entry.point_count > 0 {
                            entry.point_count.min(remaining) as usize
                        } else {
                            u64::from(chunk_size).min(remaining) as usize
                        };

                        match decode_standard_pointwise_chunk_point10_v2(
                            &chunk_bytes,
                            points_in_chunk,
                            item_specs,
                            *pdf,
                            *extra_bytes,
                            *scales,
                            *offsets,
                        ) {
                            Ok(points) => {
                                acc.push(points);
                                Ok(acc)
                            }
                            Err(e) => Err(e),
                        }
                    }
                },
            )
            .try_reduce(
                || Vec::new(),
                |mut left, mut right| {
                    left.append(&mut right);
                    Ok(left)
                },
            );

        // Flatten chunks and collect all points in order.
        let mut all_points = Vec::new();
        for chunk_points in decoded_chunks? {
            all_points.extend(chunk_points);
        }

        Ok(all_points)
    }

    /// Serial fallback for read_all_points_parallel(). Decodes all chunks sequentially.
    #[cfg(feature = "laz-parallel")]
    fn read_all_points_serial(&mut self) -> Result<Vec<PointRecord>> {
        let state = self.standard_point10_lazy.as_ref()
            .ok_or_else(|| crate::Error::Unimplemented(
                "read_all_points_serial only supports Point10 lazy streams"
            ))?;

        let mut all_points = Vec::new();
        let mut chunk_data_offset = state.chunk_data_offset;
        let total_points_for_header = self.las.header().point_count();
        let chunk_size = self.chunk_size;

        for entry in &state.entries {
            if entry.byte_count == 0 {
                continue;
            }

            let chunk_byte_count = usize::try_from(entry.byte_count).map_err(|_| {
                crate::Error::InvalidValue {
                    field: "laz.serial_chunk_byte_count",
                    detail: format!("chunk byte_count {} does not fit into usize", entry.byte_count),
                }
            })?;

            let mut chunk_bytes = vec![0u8; chunk_byte_count];
            self.las.inner_mut().seek(SeekFrom::Start(chunk_data_offset))?;
            self.las.inner_mut().read_exact(&mut chunk_bytes)?;

            let remaining = if total_points_for_header > 0 {
                total_points_for_header.saturating_sub(all_points.len() as u64)
            } else {
                u64::from(chunk_size)
            };
            let points_in_chunk = if entry.point_count > 0 {
                entry.point_count.min(remaining) as usize
            } else {
                u64::from(chunk_size).min(remaining) as usize
            };

            let mut chunk_points = decode_standard_pointwise_chunk_point10_v2(
                &chunk_bytes,
                points_in_chunk,
                &state.item_specs,
                state.point_data_format,
                state.expected_extra_bytes_count,
                state.scales,
                state.offsets,
            )?;

            all_points.append(&mut chunk_points);
            chunk_data_offset = chunk_data_offset.saturating_add(entry.byte_count);
        }

        Ok(all_points)
    }
}


impl<R: Read + Seek> PointReader for LazReader<R> {
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool> {
        if let Some(points) = self.standard_points.as_ref() {
            if self.standard_pos >= points.len() {
                return Ok(false);
            }
            *out = points[self.standard_pos];
            self.standard_pos += 1;
            self.total_read += 1;
            return Ok(true);
        }

        if self.standard_point10_lazy.is_some() {
            if self.buf_pos >= self.buffer.len() {
                let total = self.las.header().point_count();
                if total > 0 && self.total_read >= total {
                    return Ok(false);
                }

                loop {
                    let (entry, point_data_format, expected_extra_bytes_count, scales, offsets, item_specs, chunk_data_offset) = {
                        let state = self.standard_point10_lazy.as_mut().expect("state checked above");

                        while state.entry_index < state.entries.len()
                            && state.entries[state.entry_index].byte_count == 0
                        {
                            state.entry_index += 1;
                        }

                        if state.entry_index >= state.entries.len() {
                            return Ok(false);
                        }

                        let entry = state.entries[state.entry_index];
                        state.entry_index += 1;

                        (
                            entry,
                            state.point_data_format,
                            state.expected_extra_bytes_count,
                            state.scales,
                            state.offsets,
                            state.item_specs.clone(),
                            state.chunk_data_offset,
                        )
                    };

                    let remaining = if total > 0 {
                        total.saturating_sub(self.total_read)
                    } else {
                        u64::from(self.chunk_size)
                    };
                    let points_in_chunk = if entry.point_count > 0 {
                        entry.point_count.min(remaining) as usize
                    } else {
                        u64::from(self.chunk_size).min(remaining) as usize
                    };

                    if points_in_chunk == 0 {
                        return Ok(false);
                    }

                    let chunk_byte_count = usize::try_from(entry.byte_count).map_err(|_| {
                        crate::Error::InvalidValue {
                            field: "laz.standard_chunk_byte_count",
                            detail: format!(
                                "chunk byte_count {} does not fit into usize",
                                entry.byte_count
                            ),
                        }
                    })?;

                    let mut chunk_bytes = vec![0u8; chunk_byte_count];
                    self.las
                        .inner_mut()
                        .seek(SeekFrom::Start(chunk_data_offset))?;
                    self.las.inner_mut().read_exact(&mut chunk_bytes)?;

                    self.buffer = decode_standard_pointwise_chunk_point10_v2(
                        &chunk_bytes,
                        points_in_chunk,
                        &item_specs,
                        point_data_format,
                        expected_extra_bytes_count,
                        scales,
                        offsets,
                    )?;

                    if let Some(state) = self.standard_point10_lazy.as_mut() {
                        state.chunk_data_offset = state.chunk_data_offset.saturating_add(entry.byte_count);
                    }

                    self.buf_pos = 0;
                    if !self.buffer.is_empty() {
                        break;
                    }
                }
            }

            if self.buf_pos >= self.buffer.len() {
                return Ok(false);
            }

            *out = self.buffer[self.buf_pos];
            self.buf_pos += 1;
            self.total_read += 1;
            return Ok(true);
        }

        // Refill the in-memory buffer when the current chunk is exhausted.
        if self.buf_pos >= self.buffer.len() {
            let total = self.las.header().point_count();
            if total > 0 && self.total_read >= total {
                return Ok(false);
            }

            let chunk_idx = (self.total_read / u64::from(self.chunk_size)) as usize;

            // Determine byte position of the compressed chunk to read.
            let chunk_byte_offset = if self.chunk_table.offsets.is_empty() {
                // Non-indexed (sequential) mode: advance through file linearly.
                self.next_chunk_offset
            } else {
                // Indexed mode: random-access via chunk table.
                if chunk_idx >= self.chunk_table.offsets.len() {
                    return Ok(false);
                }
                if chunk_idx == 0 {
                    self.data_start
                } else {
                    self.data_start + self.chunk_table.offsets[chunk_idx - 1]
                }
            };

            // Seek, read the u64-prefixed compressed block, then inflate.
            self.las.inner_mut().seek(SeekFrom::Start(chunk_byte_offset))?;
            let _compressed = match read_compressed_chunk(self.las.inner_mut()) {
                Ok(v) => v,
                Err(crate::Error::Io(e))
                    if self.declared_point14_standard
                        && e.kind() == ErrorKind::UnexpectedEof =>
                {
                    return Err(crate::Error::Unimplemented(
                        "standard LASzip Point14 layered stream detected, but arithmetic layered decoding is not yet implemented in wblidar standard backend",
                    ));
                }
                Err(e) => return Err(e),
            };

            // Legacy wb-native sequential reading support has been removed.
            // Only standards-compliant LASzip v2/v3 files are supported.
            if self.chunk_table.offsets.is_empty() {
                return Err(crate::Error::Unimplemented(
                    "sequential LAZ chunks (legacy wb-native format) are no longer supported; only standards-compliant LASzip v2/v3 files are supported",
                ));
            }
            // Point10 lazy should have been initialized if a valid chunk table exists.
            if self.standard_point10_lazy.is_none() {
                return Err(crate::Error::Unimplemented(
                    "LAZ chunk table detected but no valid pointwise encoding found; only standards-compliant LASzip v2/v3 Point10/Point14 encoding is supported",
                ));
            }
        }

        if self.buf_pos >= self.buffer.len() {
            return Ok(false);
        }

        *out = self.buffer[self.buf_pos];
        self.buf_pos += 1;
        self.total_read += 1;
        Ok(true)
    }

    fn point_count(&self) -> Option<u64> {
        Some(self.las.header().point_count())
    }
}

fn fail_on_partial_point14() -> bool {
    match std::env::var("WBLIDAR_FAIL_ON_PARTIAL_POINT14") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
   }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::LazReader;
    use crate::io::{PointReader, PointWriter};
    use crate::laz::writer::{LazWriter, LazWriterConfig};
    use crate::point::PointRecord;

    fn make_points(n: usize) -> Vec<PointRecord> {
        (0..n)
            .map(|i| PointRecord {
                x: i as f64 * 1.5,
                y: i as f64 * 2.5,
                z: i as f64 * 0.5,
                intensity: (i % 65536) as u16,
                classification: (i % 32) as u8,
                ..PointRecord::default()
            })
            .collect()
    }

    #[test]
    fn laz_round_trip_small() -> crate::Result<()> {
        let src = make_points(7);
        let mut cursor = Cursor::new(Vec::<u8>::new());

        {
            let cfg = LazWriterConfig::default();
            let mut writer = LazWriter::new(&mut cursor, cfg)?;
            for p in &src {
                writer.write_point(p)?;
            }
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = LazReader::new(&mut cursor)?;
        assert_eq!(reader.point_count(), Some(7));
        let got = reader.read_all()?;
        assert_eq!(got.len(), 7);
        for (a, b) in src.iter().zip(got.iter()) {
            assert!((a.x - b.x).abs() < 0.01, "x mismatch: {} vs {}", a.x, b.x);
            assert!((a.y - b.y).abs() < 0.01, "y mismatch: {} vs {}", a.y, b.y);
            assert!((a.z - b.z).abs() < 0.01, "z mismatch: {} vs {}", a.z, b.z);
            assert_eq!(a.intensity, b.intensity);
            assert_eq!(a.classification, b.classification);
        }
        Ok(())
    }

    /// A multi-chunk round-trip exercises chunk-boundary bookkeeping.
    #[test]
    fn laz_round_trip_multi_chunk() -> crate::Result<()> {
        let n = 200usize;
        let src = make_points(n);

        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 50;

        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = LazWriter::new(&mut cursor, cfg)?;
            for p in &src {
                writer.write_point(p)?;
            }
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = LazReader::new(&mut cursor)?;
        assert_eq!(reader.point_count(), Some(n as u64));
        let got = reader.read_all()?;
        assert_eq!(got.len(), n);
        for (i, (a, b)) in src.iter().zip(got.iter()).enumerate() {
            assert!((a.x - b.x).abs() < 0.01, "point {i}: x mismatch");
            assert!((a.y - b.y).abs() < 0.01, "point {i}: y mismatch");
            assert!((a.z - b.z).abs() < 0.01, "point {i}: z mismatch");
        }
        Ok(())
    }
}

