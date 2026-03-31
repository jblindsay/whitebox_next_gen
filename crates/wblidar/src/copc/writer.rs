//! COPC writer — builds an octree and writes the COPC structure.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
#[cfg(feature = "copc-parallel")]
use std::sync::OnceLock;
use crate::copc::hierarchy::{CopcEntry, CopcHierarchy, CopcInfo, VoxelKey};
use crate::copc::{COPC_HIERARCHY_RECORD_ID, COPC_INFO_RECORD_ID, COPC_USER_ID};
use crate::crs::{ogc_wkt_from_epsg, Crs};
use crate::io::{le, PointWriter};
use crate::las::header::{GlobalEncoding, LasHeader};
use crate::las::vlr::{Vlr, VlrKey, LASF_PROJECTION_USER_ID, OGC_WKT_RECORD_ID};
use crate::las::writer::WriterConfig;
use crate::laz::laszip_chunk_table::{write_laszip_chunk_table, LaszipChunkTableEntry};
use crate::laz::standard_point14::encode_standard_layered_chunk_point14_v3_constant_attributes;
use crate::laz::build_laszip_vlr_for_format;
use crate::point::PointRecord;
use crate::Error;
use crate::Result;
use wide::f64x4;
#[cfg(feature = "copc-parallel")]
use rayon::prelude::*;

const HIERARCHY_PAGE_MAX_ENTRIES: usize = 512;
#[cfg(feature = "copc-parallel")]
// Thresholds tuned via automated sweep on representative LAS files (1.3GB):
// p4_high_gate config yielded 1.3753x speedup with deterministic tie-break sorting.
const DEFAULT_PARALLEL_NODE_ENCODE_MIN_NODES: usize = 16;
#[cfg(feature = "copc-parallel")]
const DEFAULT_PARALLEL_SORT_MIN_POINTS: usize = 80_000;
#[cfg(feature = "copc-parallel")]
const DEFAULT_PARALLEL_NODE_ENCODE_MIN_POINTS: usize = 400_000;

#[cfg(feature = "copc-parallel")]
fn parse_env_usize(var: &str, default: usize) -> usize {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

#[cfg(feature = "copc-parallel")]
fn parallel_node_encode_min_nodes() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| {
        parse_env_usize(
            "WBLIDAR_COPC_PARALLEL_MIN_NODES",
            DEFAULT_PARALLEL_NODE_ENCODE_MIN_NODES,
        )
    })
}

#[cfg(feature = "copc-parallel")]
fn parallel_sort_min_points() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| {
        parse_env_usize(
            "WBLIDAR_COPC_PARALLEL_SORT_MIN_POINTS",
            DEFAULT_PARALLEL_SORT_MIN_POINTS,
        )
    })
}

#[cfg(feature = "copc-parallel")]
fn parallel_node_encode_min_points() -> usize {
    static VALUE: OnceLock<usize> = OnceLock::new();
    *VALUE.get_or_init(|| {
        parse_env_usize(
            "WBLIDAR_COPC_PARALLEL_MIN_POINTS",
            DEFAULT_PARALLEL_NODE_ENCODE_MIN_POINTS,
        )
    })
}

struct EncodedNodeChunk {
    key: VoxelKey,
    compressed: Vec<u8>,
    point_count: usize,
}

/// Node-local point ordering policy used before Point14 compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopcNodePointOrdering {
    /// Preserve current behavior: GPS time when present, otherwise Morton.
    #[default]
    Auto,
    /// Always use Morton (Z-order) spatial ordering.
    Morton,
    /// Always use Hilbert spatial ordering.
    Hilbert,
}

/// Configuration for the COPC writer.
#[derive(Debug, Clone)]
pub struct CopcWriterConfig {
    /// Base LAS writer config.
    pub las: WriterConfig,
    /// Spatial root center X.
    pub center_x: f64,
    /// Spatial root center Y.
    pub center_y: f64,
    /// Spatial root center Z.
    pub center_z: f64,
    /// Spatial root half-size.
    pub halfsize: f64,
    /// Target grid spacing at the root level.
    pub spacing: f64,
    /// Maximum octree depth.
    pub max_depth: u32,
    /// Maximum number of points to keep in a single node before subdividing.
    pub max_points_per_node: usize,
    /// Ordering policy for points within each COPC node before compression.
    ///
    /// Default is [`CopcNodePointOrdering::Auto`], which sorts by GPS time
    /// when present and otherwise falls back to Morton ordering.
    pub node_point_ordering: CopcNodePointOrdering,
    /// Compression level 0 (store) – 9 (best effort). Default 6.
    /// 
    /// Note: Currently affects only wb-native DEFLATE. Point14 arithmetic
    /// encoding uses fixed model sizes independent of this setting.
    /// Future versions may use this to control arithmetic model aggressiveness.
    pub compression_level: u32,
}

impl Default for CopcWriterConfig {
    fn default() -> Self {
        CopcWriterConfig {
            las: WriterConfig::default(),
            center_x: 0.0, center_y: 0.0, center_z: 0.0,
            halfsize: 1000.0, spacing: 10.0,
            max_depth: 8,
            // Favor larger leaves by default to reduce hierarchy overhead and
            // improve Point14 arithmetic context reuse inside each node.
            max_points_per_node: 100_000,
            node_point_ordering: CopcNodePointOrdering::Auto,
            compression_level: 6,
        }
    }
}

/// Builds an in-memory COPC octree and writes the final file.
///
/// All points are buffered in memory, then the octree is built and written.
/// For very large point clouds a streaming tile-based approach is recommended.
pub struct CopcWriter<W: Write + Seek> {
    inner: W,
    config: CopcWriterConfig,
    /// All input points, to be organised into nodes on `finish()`.
    points: Vec<PointRecord>,
}

impl<W: Write + Seek> CopcWriter<W> {
    /// Create a new COPC writer.  The actual file is not written until
    /// `finish()` is called.
    pub fn new(inner: W, config: CopcWriterConfig) -> Self {
        CopcWriter { inner, config, points: Vec::new() }
    }
}

fn promote_to_copc_point_format(
    fmt: crate::las::header::PointDataFormat,
    points: &[PointRecord],
) -> crate::las::header::PointDataFormat {
    use crate::las::header::PointDataFormat;
    match fmt {
        PointDataFormat::Pdrf6 | PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 => return fmt,
        PointDataFormat::Pdrf9 => return PointDataFormat::Pdrf6,
        PointDataFormat::Pdrf10 => return PointDataFormat::Pdrf7,
        PointDataFormat::Pdrf11 => return PointDataFormat::Pdrf6,
        PointDataFormat::Pdrf12 => return PointDataFormat::Pdrf7,
        PointDataFormat::Pdrf13 => return PointDataFormat::Pdrf8,
        PointDataFormat::Pdrf14 => return PointDataFormat::Pdrf7,
        PointDataFormat::Pdrf15 => return PointDataFormat::Pdrf8,
        _ => {}
    }

    if points.iter().any(|p| p.nir.is_some()) {
        PointDataFormat::Pdrf8
    } else if points.iter().any(|p| p.color.is_some()) {
        PointDataFormat::Pdrf7
    } else {
        PointDataFormat::Pdrf6
    }
}

impl<W: Write + Seek> PointWriter for CopcWriter<W> {
    fn write_point(&mut self, p: &PointRecord) -> Result<()> {
        self.points.push(*p);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let fmt = promote_to_copc_point_format(self.config.las.point_data_format, &self.points);

        let cx = self.config.center_x;
        let cy = self.config.center_y;
        let cz = self.config.center_z;
        let hs = self.config.halfsize;
        let (sx, sy, sz) = (
            self.config.las.x_scale,
            self.config.las.y_scale,
            self.config.las.z_scale,
        );
        let (ox, oy, oz) = (
            self.config.las.x_offset,
            self.config.las.y_offset,
            self.config.las.z_offset,
        );

        // ── Write LAS 1.4 header + COPC info VLR ──────────────────────────
        let total_points = self.points.len() as u64;
        let (gps_time_minimum, gps_time_maximum) = gps_time_range(&self.points);
        let (legacy_point_count_by_return, point_count_by_return_64) =
            return_histograms(&self.points);

        // Compute bounding box
        let (min_x, max_x, min_y, max_y, min_z, max_z) =
            bounding_box(&self.points);

        // ── Partition points into octree nodes ─────────────────────────────
        let input_points = std::mem::take(&mut self.points);
        let nodes = partition_points_into_nodes(
            input_points,
            cx,
            cy,
            cz,
            hs,
            &self.config,
            (sx, sy, sz),
            (ox, oy, oz),
        );

        // COPC info VLR placeholder
        let copc_info_placeholder = CopcInfo {
            center_x: cx, center_y: cy, center_z: cz,
            halfsize: hs, spacing: self.config.spacing,
            hierarchy_root_offset: 0, // back-patched later
            hierarchy_root_size: 0,
            gps_time_minimum,
            gps_time_maximum,
        };

        let copc_info_vlr = Vlr {
            key: VlrKey { user_id: COPC_USER_ID.to_owned(), record_id: COPC_INFO_RECORD_ID },
            description: "COPC info".to_owned(),
            data: copc_info_placeholder.to_bytes(),
            extended: false,
        };

        let mut vlrs = self.config.las.vlrs.clone();
        let laszip_vlr = build_laszip_vlr_for_format(fmt, u32::MAX);
        append_projection_vlrs(&mut vlrs, self.config.las.crs.as_ref());
        let mut all_vlrs = Vec::with_capacity(vlrs.len() + 2);
        all_vlrs.push(copc_info_vlr);
        all_vlrs.push(laszip_vlr);
        all_vlrs.extend(vlrs);
        let global_encoding = global_encoding_for_vlrs(&all_vlrs);

        // Compute VLR sizes for offset_to_point_data
        let vlr_size: usize = all_vlrs.iter().map(Vlr::serialised_size).sum();
        let vlr_size_u32 = u32::try_from(vlr_size).unwrap_or(u32::MAX);
        let offset_to_point_data = 375u32.saturating_add(vlr_size_u32);

        let number_of_vlrs = u32::try_from(all_vlrs.len()).unwrap_or(u32::MAX);

        let record_length = fmt.core_size() + self.config.las.extra_bytes_per_point;

        let las_hdr = LasHeader {
            version_major: 1, version_minor: 4,
            system_identifier: self.config.las.system_identifier.clone(),
            generating_software: self.config.las.generating_software.clone(),
            file_creation_day: 1, file_creation_year: 2024,
            header_size: 375,
            offset_to_point_data,
            number_of_vlrs,
            point_data_format: fmt,
            point_data_record_length: record_length,
            global_encoding,
            project_id: [0u8; 16],
            x_scale: self.config.las.x_scale,
            y_scale: self.config.las.y_scale,
            z_scale: self.config.las.z_scale,
            x_offset: self.config.las.x_offset,
            y_offset: self.config.las.y_offset,
            z_offset: self.config.las.z_offset,
            max_x, min_x, max_y, min_y, max_z, min_z,
            legacy_point_count: u32::try_from(total_points.min(u64::from(u32::MAX))).unwrap_or(u32::MAX),
            legacy_point_count_by_return,
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0), // back-patched
            number_of_evlrs: Some(1),
            point_count_64: Some(total_points),
            point_count_by_return_64: Some(point_count_by_return_64),
            extra_bytes_count: self.config.las.extra_bytes_per_point,
        };

        las_hdr.write(&mut self.inner)?;

        // LAZ files must set the compressed bit (bit 7) in the PDRF byte.
        // `PointDataFormat` stores only the base ID, so patch the on-disk header.
        let after_header_pos = self.inner.stream_position()?;
        self.inner.seek(SeekFrom::Start(104))?;
        le::write_u8(&mut self.inner, (fmt as u8) | 0x80)?;
        self.inner.seek(SeekFrom::Start(after_header_pos))?;

        // Record where the COPC info VLR data starts (for back-patching).
        let copc_info_data_pos = 375u64 + 54; // header + VLR header before data
        for vlr in &all_vlrs {
            vlr.write(&mut self.inner)?;
        }

        // ── LASzip chunk-table pointer placeholder ───────────────────────
        // Standard LASzip stores an i64/u64 pointer at offset_to_point_data.
        // It points to a chunk table near EOF (after compressed chunks).
        let chunk_table_ptr_pos = self.inner.stream_position()?;
        le::write_u64(&mut self.inner, 0)?;

        let mut sorted_keys: Vec<VoxelKey> = nodes.keys().copied().collect();
        sorted_keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));

        // ── Build compressed node chunks in deterministic key order ───────
        // The optional parallel path only parallelizes this CPU-bound step.
        // Final file writes remain serial and deterministic.
        let encoded_chunks = encode_all_node_chunks(
            &sorted_keys,
            &nodes,
            self.config.node_point_ordering,
            fmt,
            (sx, sy, sz),
            (ox, oy, oz),
        )?;

        // ── Write compressed node chunks in DFS order ──────────────────────
        let mut entries = Vec::with_capacity(sorted_keys.len());
        // `first_chunk_pos` is the file offset of the first compressed chunk
        // (immediately after the 8-byte chunk-table pointer field).
        let first_chunk_pos = self.inner.stream_position()?;
        let mut cumulative_chunk_bytes: u64 = 0;
        let mut standard_chunk_entries: Vec<LaszipChunkTableEntry> = Vec::with_capacity(sorted_keys.len());

        for chunk in encoded_chunks {
            let compressed = chunk.compressed;

            let byte_size = compressed.len() as i32;
            // CopcEntry::offset is the absolute file position of this chunk.
            let chunk_abs_offset = first_chunk_pos + cumulative_chunk_bytes;
            self.inner.write_all(&compressed)?;
            cumulative_chunk_bytes += compressed.len() as u64;
            standard_chunk_entries.push(LaszipChunkTableEntry {
                point_count: chunk.point_count as u64,
                byte_count: compressed.len() as u64,
            });

            entries.push(CopcEntry {
                key: chunk.key,
                offset: chunk_abs_offset,
                byte_size,
                point_count: chunk.point_count as i32,
            });
        }

        // Write the standard LASzip chunk table after the compressed payload.
        let after_chunks_pos = self.inner.stream_position()?;
        let chunk_table_offset = after_chunks_pos;
        write_laszip_chunk_table(&mut self.inner, &standard_chunk_entries, true)?;

        // Back-patch the pointer at offset_to_point_data to chunk_table_offset.
        let end_after_table = self.inner.stream_position()?;
        self.inner.seek(SeekFrom::Start(chunk_table_ptr_pos))?;
        le::write_u64(&mut self.inner, chunk_table_offset)?;
        self.inner.seek(SeekFrom::Start(end_after_table))?;

        // Ensure deep leaves are reachable from root by explicitly materializing
        // missing ancestor keys in the hierarchy key space.
        entries = close_hierarchy_with_ancestors(entries);

        // ── Write hierarchy EVLR (+ optional sub-pages) ───────────────────
        let hier_evlr_offset = self.inner.stream_position()?;
        let hier_data_offset = hier_evlr_offset.saturating_add(60);
        let (hier_bytes, subpage_bytes) = build_hierarchy_pages(entries, hier_data_offset)?;
        let hier_size = hier_bytes.len() as u64;

        // Write minimal EVLR header (60 bytes)
        let hierarchy_evlr = Vlr {
            key: VlrKey { user_id: COPC_USER_ID.to_owned(), record_id: COPC_HIERARCHY_RECORD_ID },
            description: "COPC hierarchy".to_owned(),
            data: hier_bytes,
            extended: true,
        };
        hierarchy_evlr.write_extended(&mut self.inner)?;

        // Additional hierarchy pages are stored as raw bytes and referenced
        // by offset/size entries from the root hierarchy page.
        for page in &subpage_bytes {
            self.inner.write_all(page)?;
        }

        // ── Back-patch COPC info VLR with hierarchy offset/size ───────────
        let updated_info = CopcInfo {
            center_x: cx, center_y: cy, center_z: cz,
            halfsize: hs, spacing: self.config.spacing,
            hierarchy_root_offset: hier_data_offset,
            hierarchy_root_size: hier_size,
            gps_time_minimum,
            gps_time_maximum,
        };
        let end_pos = self.inner.stream_position()?;
        self.inner.seek(SeekFrom::Start(copc_info_data_pos))?;
        self.inner.write_all(&updated_info.to_bytes())?;

        // Back-patch start_of_first_evlr in the LAS header (field at offset 235 in 1.4 header)
        self.inner.seek(SeekFrom::Start(235))?;
        le::write_u64(&mut self.inner, hier_evlr_offset)?;

        self.inner.seek(SeekFrom::Start(end_pos))?;
        Ok(())
    }
}

fn encode_node_chunk(
    key: VoxelKey,
    node_points: &[PointRecord],
    ordering: CopcNodePointOrdering,
    point_data_format: crate::las::header::PointDataFormat,
    scale: (f64, f64, f64),
    offset: (f64, f64, f64),
) -> Result<EncodedNodeChunk> {
    let (sx, sy, sz) = scale;
    let (ox, oy, oz) = offset;
    let inv_sx = 1.0 / sx;
    let inv_sy = 1.0 / sy;
    let inv_sz = 1.0 / sz;

    let mut pts = node_points.to_vec();
    order_node_points(&mut pts, ordering, sx, sy, ox, oy);

    // Convert to integer-scaled points for the Point14 arithmetic codec.
    let mut scaled: Vec<PointRecord> = Vec::with_capacity(pts.len());

    let mut i = 0usize;
    let ox4 = f64x4::splat(ox);
    let oy4 = f64x4::splat(oy);
    let oz4 = f64x4::splat(oz);
    let inv_sx4 = f64x4::splat(inv_sx);
    let inv_sy4 = f64x4::splat(inv_sy);
    let inv_sz4 = f64x4::splat(inv_sz);

    while i + 4 <= pts.len() {
        let px = f64x4::from([
            pts[i].x,
            pts[i + 1].x,
            pts[i + 2].x,
            pts[i + 3].x,
        ]);
        let py = f64x4::from([
            pts[i].y,
            pts[i + 1].y,
            pts[i + 2].y,
            pts[i + 3].y,
        ]);
        let pz = f64x4::from([
            pts[i].z,
            pts[i + 1].z,
            pts[i + 2].z,
            pts[i + 3].z,
        ]);

        let qx: [f64; 4] = ((px - ox4) * inv_sx4).round().into();
        let qy: [f64; 4] = ((py - oy4) * inv_sy4).round().into();
        let qz: [f64; 4] = ((pz - oz4) * inv_sz4).round().into();

        for lane in 0..4 {
            let mut s = pts[i + lane];
            s.x = qx[lane];
            s.y = qy[lane];
            s.z = qz[lane];
            scaled.push(s);
        }
        i += 4;
    }

    while i < pts.len() {
        let p = pts[i];
        let mut s = p;
        s.x = (p.x - ox).mul_add(inv_sx, 0.0).round();
        s.y = (p.y - oy).mul_add(inv_sy, 0.0).round();
        s.z = (p.z - oz).mul_add(inv_sz, 0.0).round();
        scaled.push(s);
        i += 1;
    }

    let compressed = match encode_standard_layered_chunk_point14_v3_constant_attributes(
        &scaled,
        point_data_format,
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    ) {
        Ok(bytes) => bytes,
        Err(Error::Unimplemented(_)) => {
            return Err(crate::Error::Unimplemented(
                "COPC writer Point14 standards encoder could not encode this point set",
            ));
        }
        Err(err) => return Err(err),
    };

    Ok(EncodedNodeChunk {
        key,
        compressed,
        point_count: pts.len(),
    })
}

#[cfg(not(feature = "copc-parallel"))]
fn encode_all_node_chunks(
    sorted_keys: &[VoxelKey],
    nodes: &HashMap<VoxelKey, Vec<PointRecord>>,
    ordering: CopcNodePointOrdering,
    point_data_format: crate::las::header::PointDataFormat,
    scale: (f64, f64, f64),
    offset: (f64, f64, f64),
) -> Result<Vec<EncodedNodeChunk>> {
    let mut chunks = Vec::with_capacity(sorted_keys.len());
    for &key in sorted_keys {
        chunks.push(encode_node_chunk(
            key,
            &nodes[&key],
            ordering,
            point_data_format,
            scale,
            offset,
        )?);
    }
    Ok(chunks)
}

#[cfg(feature = "copc-parallel")]
fn encode_all_node_chunks(
    sorted_keys: &[VoxelKey],
    nodes: &HashMap<VoxelKey, Vec<PointRecord>>,
    ordering: CopcNodePointOrdering,
    point_data_format: crate::las::header::PointDataFormat,
    scale: (f64, f64, f64),
    offset: (f64, f64, f64),
) -> Result<Vec<EncodedNodeChunk>> {
    let total_points: usize = sorted_keys.iter().map(|k| nodes[k].len()).sum();
    let thread_count = rayon::current_num_threads().max(1);
    let adaptive_min_nodes = parallel_node_encode_min_nodes().max(thread_count * 2);

    if sorted_keys.len() < adaptive_min_nodes || total_points < parallel_node_encode_min_points() {
        let mut chunks = Vec::with_capacity(sorted_keys.len());
        for &key in sorted_keys {
            chunks.push(encode_node_chunk(
                key,
                &nodes[&key],
                ordering,
                point_data_format,
                scale,
                offset,
            )?);
        }
        return Ok(chunks);
    }

    // `par_iter()` over a slice is indexed; collect preserves input order.
    sorted_keys
        .par_iter()
        .map(|&key| {
            encode_node_chunk(
                key,
                &nodes[&key],
                ordering,
                point_data_format,
                scale,
                offset,
            )
        })
        .collect::<Result<Vec<_>>>()
}

fn gps_time_range(points: &[PointRecord]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for p in points {
        if let Some(t) = p.gps_time {
            min = min.min(t.0);
            max = max.max(t.0);
        }
    }
    if min.is_finite() && max.is_finite() {
        (min, max)
    } else {
        (0.0, 0.0)
    }
}

fn return_histograms(points: &[PointRecord]) -> ([u32; 5], [u64; 15]) {
    let mut legacy = [0u32; 5];
    let mut full = [0u64; 15];

    for p in points {
        if p.return_number == 0 {
            continue;
        }
        let idx = usize::from(p.return_number.saturating_sub(1));
        if idx < full.len() {
            full[idx] = full[idx].saturating_add(1);
        }
        if idx < legacy.len() {
            legacy[idx] = legacy[idx].saturating_add(1);
        }
    }

    (legacy, full)
}

/// Compute a Morton (Z-order) code for spatial ordering, using quantized X/Y.
/// This preserves spatial locality better than GPS-time sorting, improving LAZ compression.
fn morton_code(x: f64, y: f64, scale_x: f64, scale_y: f64, offset_x: f64, offset_y: f64) -> u64 {
    // Quantize to integer domain, then extract lower 16 bits from each.
    let xi = ((x - offset_x) / scale_x).round() as i32;
    let yi = ((y - offset_y) / scale_y).round() as i32;
    let xu = u64::from(xi.cast_unsigned());
    let yu = u64::from(yi.cast_unsigned());
    
    // Interleave bits: result[2i] = x[i], result[2i+1] = y[i], for i=0..15
    let mut code = 0u64;
    for i in 0..16 {
        code |= ((xu >> i) & 1) << (2 * i);
        code |= ((yu >> i) & 1) << (2 * i + 1);
    }
    code
}

/// Compute a Hilbert curve distance for spatial ordering.
/// Hilbert curves preserve spatial locality better than Morton order (Z-order),
/// potentially improving compression through better predictor performance.
fn hilbert_distance(x: f64, y: f64, scale_x: f64, scale_y: f64, offset_x: f64, offset_y: f64) -> u64 {
    // Map quantized coordinates to a 16-bit domain used by the Hilbert walk.
    // This mirrors the Morton ordering path, keeping behavior deterministic
    // for large integer coordinates and avoiding overflow during quadrant flips.
    let xi = (((x - offset_x) / scale_x).round() as i32).cast_unsigned() & 0xFFFF;
    let yi = (((y - offset_y) / scale_y).round() as i32).cast_unsigned() & 0xFFFF;
    
    // Compute Hilbert distance for 16-bit coordinates (up to 2^32 values).
    let mut hd = 0u64;
    let n = 1u32 << 16;
    let mut s = 1u32 << 15; // Start at 2^15
    let mut x_cur = xi;
    let mut y_cur = yi;
    
    while s > 0 {
        let rx = (x_cur & s) > 0;
        let ry = (y_cur & s) > 0;
        hd += ((3 * u64::from(rx)) ^ u64::from(ry)) * (u64::from(s) * u64::from(s));
        
        // Rotate quadrant (standard Hilbert transform step).
        if !ry {
            if rx {
                x_cur = n - 1 - x_cur;
                y_cur = n - 1 - y_cur;
            }
            (x_cur, y_cur) = (y_cur, x_cur);
        }
        s >>= 1;
    }
    hd
}

fn sort_points_by_morton_order(
    points: &mut [PointRecord],
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
) {
    // Pre-compute Morton codes to avoid re-computing during sort.
    let mut codes: Vec<(u64, usize)> = points
        .iter()
        .enumerate()
        .map(|(idx, p)| (morton_code(p.x, p.y, scale_x, scale_y, offset_x, offset_y), idx))
        .collect();
    #[cfg(feature = "copc-parallel")]
    {
        if points.len() >= parallel_sort_min_points() {
            // Sort by (code, then index) for deterministic tie-breaking across serial/parallel paths.
            codes.par_sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        } else {
            codes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        }
    }
    #[cfg(not(feature = "copc-parallel"))]
    {
        codes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    }

    // Reorder in place from sorted "new_pos -> old_idx" mapping.
    apply_sorted_index_reorder(points, &codes);
}

fn sort_points_by_hilbert_order(
    points: &mut [PointRecord],
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
) {
    // Pre-compute Hilbert distances.
    let mut codes: Vec<(u64, usize)> = points
        .iter()
        .enumerate()
        .map(|(idx, p)| (hilbert_distance(p.x, p.y, scale_x, scale_y, offset_x, offset_y), idx))
        .collect();
    #[cfg(feature = "copc-parallel")]
    {
        if points.len() >= parallel_sort_min_points() {
            // Sort by (code, then index) for deterministic tie-breaking across serial/parallel paths.
            codes.par_sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        } else {
            codes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        }
    }
    #[cfg(not(feature = "copc-parallel"))]
    {
        codes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    }

    // Reorder in place from sorted "new_pos -> old_idx" mapping.
    apply_sorted_index_reorder(points, &codes);
}

fn apply_sorted_index_reorder(points: &mut [PointRecord], codes: &[(u64, usize)]) {
    if points.len() <= 1 {
        return;
    }

    // Map old index -> new index.
    let mut target_pos = vec![0usize; points.len()];
    for (new_idx, &(_, old_idx)) in codes.iter().enumerate() {
        target_pos[old_idx] = new_idx;
    }

    // Apply permutation in-place by swapping both data and mapping.
    for i in 0..points.len() {
        while target_pos[i] != i {
            let j = target_pos[i];
            points.swap(i, j);
            target_pos.swap(i, j);
        }
    }
}

fn order_node_points(
    points: &mut [PointRecord],
    ordering: CopcNodePointOrdering,
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
) {
    match ordering {
        CopcNodePointOrdering::Auto => {
            // GPS-time order tends to improve Point14 attribute compression
            // because scan-line-adjacent points have similar metadata.
            let has_gps = points.iter().any(|p| p.gps_time.is_some());
            if has_gps {
                sort_points_by_gps_time(points);
            } else {
                sort_points_by_morton_order(points, scale_x, scale_y, offset_x, offset_y);
            }
        }
        CopcNodePointOrdering::Morton => {
            sort_points_by_morton_order(points, scale_x, scale_y, offset_x, offset_y);
        }
        CopcNodePointOrdering::Hilbert => {
            sort_points_by_hilbert_order(points, scale_x, scale_y, offset_x, offset_y);
        }
    }
}

// Legacy GPS-time sort, kept for backward compatibility and testing.
fn sort_points_by_gps_time(points: &mut [PointRecord]) {
    points.sort_by(|a, b| {
        let ta = a.gps_time.map_or(f64::NEG_INFINITY, |t| t.0);
        let tb = b.gps_time.map_or(f64::NEG_INFINITY, |t| t.0);
        ta.total_cmp(&tb)
    });
}

fn close_hierarchy_with_ancestors(entries: Vec<CopcEntry>) -> Vec<CopcEntry> {
    let mut by_key: HashMap<VoxelKey, CopcEntry> = HashMap::with_capacity(entries.len());
    for entry in entries {
        by_key.insert(entry.key, entry);
    }

    let mut frontier: Vec<VoxelKey> = by_key.keys().copied().collect();
    while let Some(mut key) = frontier.pop() {
        while key.level > 0 {
            key = VoxelKey {
                level: key.level - 1,
                x: key.x / 2,
                y: key.y / 2,
                z: key.z / 2,
            };
            if let std::collections::hash_map::Entry::Vacant(v) = by_key.entry(key) {
                v.insert(CopcEntry {
                    key,
                    offset: 0,
                    byte_size: 0,
                    point_count: 0,
                });
            }
        }
    }

    by_key.entry(VoxelKey::ROOT).or_insert(CopcEntry {
        key: VoxelKey::ROOT,
        offset: 0,
        byte_size: 0,
        point_count: 0,
    });

    let mut closed: Vec<CopcEntry> = by_key.into_values().collect();
    closed.sort_by_key(|e| (e.key.level, e.key.x, e.key.y, e.key.z));
    closed
}

fn build_hierarchy_pages(
    mut entries: Vec<CopcEntry>,
    hierarchy_data_offset: u64,
) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
    #[derive(Clone)]
    struct HierPage {
        entries: Vec<CopcEntry>,
        // (entry_index_in_page, child_page_index)
        refs: Vec<(usize, usize)>,
    }

    fn is_descendant_or_self(key: VoxelKey, ancestor: VoxelKey) -> bool {
        if key.level < ancestor.level {
            return false;
        }
        let shift = (key.level - ancestor.level) as u32;
        (key.x >> shift) == ancestor.x
            && (key.y >> shift) == ancestor.y
            && (key.z >> shift) == ancestor.z
    }

    fn subtree_keys(root: VoxelKey, keys: &[VoxelKey]) -> Vec<VoxelKey> {
        let mut out: Vec<VoxelKey> = keys
            .iter()
            .copied()
            .filter(|k| is_descendant_or_self(*k, root))
            .collect();
        out.sort_by_key(|k| (k.level, k.x, k.y, k.z));
        out
    }

    fn build_page_for_subtree(
        root: VoxelKey,
        keys: &[VoxelKey],
        by_key: &HashMap<VoxelKey, CopcEntry>,
        children: &HashMap<VoxelKey, Vec<VoxelKey>>,
        subtree_size: &HashMap<VoxelKey, usize>,
        pages: &mut Vec<HierPage>,
    ) -> Result<usize> {
        let this_size = *subtree_size.get(&root).ok_or(Error::InvalidValue {
            field: "copc.hierarchy",
            detail: format!("missing subtree size for key {:?}", root),
        })?;

        if this_size <= HIERARCHY_PAGE_MAX_ENTRIES {
            let mut page_entries: Vec<CopcEntry> = subtree_keys(root, keys)
                .into_iter()
                .map(|k| by_key.get(&k).copied().ok_or(Error::InvalidValue {
                    field: "copc.hierarchy",
                    detail: format!("missing entry for key {:?}", k),
                }))
                .collect::<Result<Vec<_>>>()?;
            page_entries.sort_by_key(|e| (e.key.level, e.key.x, e.key.y, e.key.z));
            let idx = pages.len();
            pages.push(HierPage {
                entries: page_entries,
                refs: Vec::new(),
            });
            return Ok(idx);
        }

        let root_entry = by_key.get(&root).copied().ok_or(Error::InvalidValue {
            field: "copc.hierarchy",
            detail: format!("missing root entry for subtree key {:?}", root),
        })?;

        let child_keys = children.get(&root).cloned().unwrap_or_default();
        let mut refs: Vec<(VoxelKey, usize)> = Vec::new();
        for child in child_keys {
            let child_page = build_page_for_subtree(
                child,
                keys,
                by_key,
                children,
                subtree_size,
                pages,
            )?;
            refs.push((child, child_page));
        }
        refs.sort_by_key(|(k, _)| (k.level, k.x, k.y, k.z));

        let mut page_entries = Vec::with_capacity(1 + refs.len());
        page_entries.push(root_entry);
        let mut page_refs = Vec::with_capacity(refs.len());
        for (child_key, child_page_idx) in refs {
            let mut ref_entry = by_key.get(&child_key).copied().ok_or(Error::InvalidValue {
                field: "copc.hierarchy",
                detail: format!("missing child entry for key {:?}", child_key),
            })?;
            ref_entry.offset = 0;
            ref_entry.byte_size = 0;
            ref_entry.point_count = -1;
            let entry_idx = page_entries.len();
            page_entries.push(ref_entry);
            page_refs.push((entry_idx, child_page_idx));
        }

        if page_entries.len() > HIERARCHY_PAGE_MAX_ENTRIES {
            return Err(Error::Unimplemented(
                "hierarchy page exceeds max entries after subtree pagination",
            ));
        }

        let idx = pages.len();
        pages.push(HierPage {
            entries: page_entries,
            refs: page_refs,
        });
        Ok(idx)
    }

    entries.sort_by_key(|e| (e.key.level, e.key.x, e.key.y, e.key.z));

    if entries.len() <= HIERARCHY_PAGE_MAX_ENTRIES {
        let root = CopcHierarchy { entries };
        return Ok((root.to_bytes()?, Vec::new()));
    }

    let by_key: HashMap<VoxelKey, CopcEntry> = entries
        .iter()
        .copied()
        .map(|e| (e.key, e))
        .collect();
    if !by_key.contains_key(&VoxelKey::ROOT) {
        return Err(Error::InvalidValue {
            field: "copc.hierarchy",
            detail: "missing root entry in hierarchy".to_owned(),
        });
    }

    let keys: Vec<VoxelKey> = by_key.keys().copied().collect();
    let mut children: HashMap<VoxelKey, Vec<VoxelKey>> = HashMap::new();
    for key in &keys {
        if *key == VoxelKey::ROOT {
            continue;
        }
        let parent = VoxelKey {
            level: key.level - 1,
            x: key.x / 2,
            y: key.y / 2,
            z: key.z / 2,
        };
        children.entry(parent).or_default().push(*key);
    }
    for child_list in children.values_mut() {
        child_list.sort_by_key(|k| (k.level, k.x, k.y, k.z));
    }

    fn compute_subtree_size(
        key: VoxelKey,
        children: &HashMap<VoxelKey, Vec<VoxelKey>>,
        memo: &mut HashMap<VoxelKey, usize>,
    ) -> usize {
        if let Some(v) = memo.get(&key) {
            return *v;
        }
        let mut total = 1usize;
        if let Some(kids) = children.get(&key) {
            for child in kids {
                total = total.saturating_add(compute_subtree_size(*child, children, memo));
            }
        }
        memo.insert(key, total);
        total
    }

    let mut subtree_size: HashMap<VoxelKey, usize> = HashMap::new();
    compute_subtree_size(VoxelKey::ROOT, &children, &mut subtree_size);

    let mut pages: Vec<HierPage> = Vec::new();
    let root_page_idx = build_page_for_subtree(
        VoxelKey::ROOT,
        &keys,
        &by_key,
        &children,
        &subtree_size,
        &mut pages,
    )?;

    if root_page_idx != pages.len().saturating_sub(1) {
        return Err(Error::InvalidValue {
            field: "copc.hierarchy",
            detail: "internal hierarchy builder produced unexpected root page index".to_owned(),
        });
    }

    // Root page must be serialized first because CopcInfo points to it directly.
    pages.swap(0, root_page_idx);
    for page in &mut pages {
        for (_, child_idx) in &mut page.refs {
            if *child_idx == 0 {
                *child_idx = root_page_idx;
            } else if *child_idx == root_page_idx {
                *child_idx = 0;
            }
        }
    }

    let page_sizes: Vec<u64> = pages
        .iter()
        .map(|p| (p.entries.len() * CopcEntry::SIZE) as u64)
        .collect();
    let mut page_offsets = vec![0u64; pages.len()];
    let mut cursor = hierarchy_data_offset;
    for (idx, size) in page_sizes.iter().enumerate() {
        page_offsets[idx] = cursor;
        cursor = cursor.saturating_add(*size);
    }

    for page in &mut pages {
        for (entry_idx, child_page_idx) in &page.refs {
            let child_idx = *child_page_idx;
            page.entries[*entry_idx].offset = page_offsets[child_idx];
            page.entries[*entry_idx].byte_size = page_sizes[child_idx] as i32;
            page.entries[*entry_idx].point_count = -1;
        }
    }

    let root_bytes = CopcHierarchy {
        entries: pages[0].entries.clone(),
    }
    .to_bytes()?;
    let mut subpage_bytes: Vec<Vec<u8>> = Vec::with_capacity(pages.len().saturating_sub(1));
    for page in pages.iter().skip(1) {
        subpage_bytes.push(CopcHierarchy {
            entries: page.entries.clone(),
        }
        .to_bytes()?);
    }

    Ok((root_bytes, subpage_bytes))
}

fn append_projection_vlrs(vlrs: &mut Vec<Vlr>, crs: Option<&Crs>) {
    let Some(crs) = crs else { return; };

    let has_wkt = vlrs.iter().any(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
    });

    if !has_wkt {
        if let Some(wkt) = crs.wkt.as_deref().map(ToOwned::to_owned).or_else(|| {
            crs.epsg.and_then(ogc_wkt_from_epsg)
        }) {
            vlrs.push(Vlr::ogc_wkt(&wkt));
        }
    }

    // Do not auto-add GeoKeyDirectory from CRS defaults.
    // Some external validators/viewers are fragile with minimal geokey-only
    // metadata. Callers can still provide explicit geokey VLRs via config.vlrs.
}

fn global_encoding_for_vlrs(vlrs: &[Vlr]) -> GlobalEncoding {
    let mut bits = GlobalEncoding::GPS_TIME_TYPE;
    let has_wkt = vlrs.iter().any(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
    });
    if has_wkt {
        bits |= GlobalEncoding::WKT;
    }
    GlobalEncoding(bits)
}

// ── Spatial utilities ────────────────────────────────────────────────────────

fn quantize_world_coordinates(
    p: PointRecord,
    scale: (f64, f64, f64),
    offset: (f64, f64, f64),
) -> (f64, f64, f64) {
    let (sx, sy, sz) = scale;
    let (ox, oy, oz) = offset;
    (
        ((p.x - ox) / sx).round() * sx + ox,
        ((p.y - oy) / sy).round() * sy + oy,
        ((p.z - oz) / sz).round() * sz + oz,
    )
}

fn partition_points_into_nodes(
    points: Vec<PointRecord>,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    halfsize: f64,
    config: &CopcWriterConfig,
    scale: (f64, f64, f64),
    offset: (f64, f64, f64),
) -> HashMap<VoxelKey, Vec<PointRecord>> {
    const INTERNAL_NODE_KEEP_POINTS: usize = 512;

    fn recurse(
        points: Vec<PointRecord>,
        key: VoxelKey,
        center_x: f64,
        center_y: f64,
        center_z: f64,
        halfsize: f64,
        level: u32,
        config: &CopcWriterConfig,
        scale: (f64, f64, f64),
        offset: (f64, f64, f64),
        nodes: &mut HashMap<VoxelKey, Vec<PointRecord>>,
    ) {
        if points.is_empty() {
            return;
        }

        let reached_spacing = halfsize * 2.0 <= config.spacing;
        let reached_depth = level >= config.max_depth;
        let small_enough = points.len() <= config.max_points_per_node;
        if reached_spacing || reached_depth || small_enough {
            nodes.insert(key, points);
            return;
        }

        // Retain a small representative payload in each subdivided node so
        // root/internal nodes are non-empty for LOD-oriented COPC consumers.
        let keep_target = INTERNAL_NODE_KEEP_POINTS.min(points.len().saturating_sub(1));
        let mut keep_here: Vec<PointRecord> = Vec::new();
        let mut pass_down: Vec<PointRecord> = Vec::with_capacity(points.len().saturating_sub(keep_target));

        if keep_target > 0 {
            let stride = (points.len() / keep_target).max(1);
            for (idx, point) in points.into_iter().enumerate() {
                if keep_here.len() < keep_target && idx % stride == 0 {
                    keep_here.push(point);
                } else {
                    pass_down.push(point);
                }
            }

            // Fill any shortfall (e.g. due to stride rounding) from the front
            // to keep node occupancy deterministic.
            while keep_here.len() < keep_target {
                if let Some(point) = pass_down.pop() {
                    keep_here.push(point);
                } else {
                    break;
                }
            }
        } else {
            pass_down = points;
        }

        if !keep_here.is_empty() {
            nodes.insert(key, keep_here);
        }

        let child_halfsize = halfsize * 0.5;
        let mut children: [Vec<PointRecord>; 8] = std::array::from_fn(|_| Vec::new());
        for point in pass_down {
            let (qx, qy, qz) = quantize_world_coordinates(point, scale, offset);
            let nx = usize::from(qx >= center_x);
            let ny = usize::from(qy >= center_y);
            let nz = usize::from(qz >= center_z);
            let child_idx = nx | (ny << 1) | (nz << 2);
            children[child_idx].push(point);
        }

        for (child_idx, child_points) in children.into_iter().enumerate() {
            if child_points.is_empty() {
                continue;
            }
            let nx = (child_idx & 1) as i32;
            let ny = ((child_idx >> 1) & 1) as i32;
            let nz = ((child_idx >> 2) & 1) as i32;
            let child_key = VoxelKey {
                level: key.level + 1,
                x: key.x * 2 + nx,
                y: key.y * 2 + ny,
                z: key.z * 2 + nz,
            };
            let child_center_x = center_x + if nx == 1 { child_halfsize } else { -child_halfsize };
            let child_center_y = center_y + if ny == 1 { child_halfsize } else { -child_halfsize };
            let child_center_z = center_z + if nz == 1 { child_halfsize } else { -child_halfsize };
            recurse(
                child_points,
                child_key,
                child_center_x,
                child_center_y,
                child_center_z,
                child_halfsize,
                level + 1,
                config,
                scale,
                offset,
                nodes,
            );
        }
    }

    let mut nodes: HashMap<VoxelKey, Vec<PointRecord>> = HashMap::new();
    recurse(
        points,
        VoxelKey::ROOT,
        center_x,
        center_y,
        center_z,
        halfsize,
        0,
        config,
        scale,
        offset,
        &mut nodes,
    );
    nodes
}

/// Determine the deepest voxel key for a point given the root cube.
#[cfg(test)]
fn classify_point(
    px: f64, py: f64, pz: f64,
    cx: f64, cy: f64, cz: f64,
    hs: f64,
    max_depth: u32,
    spacing: f64,
) -> VoxelKey {
    let mut lx = 0i32;
    let mut ly = 0i32;
    let mut lz = 0i32;
    let mut cx_cur = cx;
    let mut cy_cur = cy;
    let mut cz_cur = cz;
    let mut cur_hs = hs;
    let mut out = VoxelKey { level: 0, x: 0, y: 0, z: 0 };

    for level in 0..max_depth {
        // If the current voxel spacing is fine enough, stop here.
        if cur_hs * 2.0 <= spacing { break; }

        cur_hs *= 0.5;

        let nx = if px >= cx_cur { 1 } else { 0 };
        let ny = if py >= cy_cur { 1 } else { 0 };
        let nz = if pz >= cz_cur { 1 } else { 0 };

        lx = lx * 2 + nx;
        ly = ly * 2 + ny;
        lz = lz * 2 + nz;

        cx_cur += if nx == 1 { cur_hs } else { -cur_hs };
        cy_cur += if ny == 1 { cur_hs } else { -cur_hs };
        cz_cur += if nz == 1 { cur_hs } else { -cur_hs };

        out = VoxelKey { level: level as i32 + 1, x: lx, y: ly, z: lz };
    }
    out
}

fn bounding_box(pts: &[PointRecord]) -> (f64, f64, f64, f64, f64, f64) {
    if pts.is_empty() {
        return (0., 0., 0., 0., 0., 0.);
    }
    // Branchless SIMD min/max accumulation over [x, y, z, _] lanes.
    let mut mins = f64x4::splat(f64::INFINITY);
    let mut maxs = f64x4::splat(f64::NEG_INFINITY);
    for p in pts {
        let coords = f64x4::new([p.x, p.y, p.z, 0.0]);
        mins = mins.min(coords);
        maxs = maxs.max(coords);
    }
    let min_arr: [f64; 4] = mins.into();
    let max_arr: [f64; 4] = maxs.into();
    (min_arr[0], max_arr[0], min_arr[1], max_arr[1], min_arr[2], max_arr[2])
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};
    use super::{
        classify_point,
        close_hierarchy_with_ancestors,
        order_node_points,
        sort_points_by_gps_time,
        sort_points_by_hilbert_order,
        sort_points_by_morton_order,
        CopcNodePointOrdering,
    };
    use crate::copc::hierarchy::{CopcEntry, CopcHierarchy, VoxelKey};
    use crate::copc::reader::{CopcReader, CopcReaderMode};
    use crate::copc::writer::{CopcWriter, CopcWriterConfig};
    use crate::crs::Crs;
    use crate::io::PointWriter;
    use crate::las::header::GlobalEncoding;
    use crate::las::reader::LasReader;
    use crate::las::vlr::{
        find_epsg, find_ogc_wkt, Vlr, GEOKEY_DIRECTORY_RECORD_ID,
        LASF_PROJECTION_USER_ID, OGC_WKT_RECORD_ID,
    };
    use crate::point::PointRecord;
    use crate::point::GpsTime;

    #[test]
    fn copc_emits_projection_vlrs_from_crs() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.crs = Some(Crs::from_epsg(4326));

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            let point = PointRecord { x: -80.0, y: 43.0, z: 300.0, ..PointRecord::default() };
            writer.write_point(&point)?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let reader = LasReader::new(&mut cursor)?;

        let wkt = find_ogc_wkt(reader.vlrs()).unwrap_or_default();
        assert!(wkt.contains("4326") || wkt.to_ascii_uppercase().contains("WGS"));
        assert!(reader.header().global_encoding.is_set(GlobalEncoding::WKT));
        Ok(())
    }

    #[test]
    fn copc_does_not_duplicate_projection_vlrs() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.crs = Some(Crs::from_epsg(4326));
        cfg.las.vlrs.push(Vlr::ogc_wkt("GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]"));
        cfg.las.vlrs.push(Vlr::geokey_directory_for_epsg(4326).expect("valid epsg for geokey"));

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            let point = PointRecord { x: -80.0, y: 43.0, z: 300.0, ..PointRecord::default() };
            writer.write_point(&point)?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let reader = LasReader::new(&mut cursor)?;

        let wkt_count = reader.vlrs().iter().filter(|v| {
            v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
        }).count();
        let geokey_count = reader.vlrs().iter().filter(|v| {
            v.key.user_id == LASF_PROJECTION_USER_ID
                && v.key.record_id == GEOKEY_DIRECTORY_RECORD_ID
        }).count();

        assert_eq!(wkt_count, 1);
        assert_eq!(geokey_count, 1);
        assert_eq!(find_epsg(reader.vlrs()), Some(4326));
        assert!(find_ogc_wkt(reader.vlrs()).is_some());
        Ok(())
    }

    #[test]
    fn copc_wkt_global_encoding_bit_requires_wkt_vlr() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.crs = None;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            let point = PointRecord { x: -80.0, y: 43.0, z: 300.0, ..PointRecord::default() };
            writer.write_point(&point)?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let reader = LasReader::new(&mut cursor)?;
        assert!(!reader.header().global_encoding.is_set(GlobalEncoding::WKT));
        Ok(())
    }

    #[test]
    fn classify_point_reaches_requested_depth() {
        let key = classify_point(7.0, 7.0, 7.0, 0.0, 0.0, 0.0, 8.0, 4, 0.5);
        assert_eq!(key.level, 4);
    }

    #[test]
    fn occupancy_partition_keeps_small_cloud_in_single_node() {
        let points = vec![
            PointRecord { x: 0.1, y: 0.1, z: 0.1, ..PointRecord::default() },
            PointRecord { x: 0.2, y: 0.2, z: 0.2, ..PointRecord::default() },
            PointRecord { x: 0.3, y: 0.3, z: 0.3, ..PointRecord::default() },
        ];

        let mut cfg = CopcWriterConfig::default();
        cfg.max_depth = 8;
        cfg.spacing = 0.000_001;
        cfg.max_points_per_node = 8;

        let nodes = super::partition_points_into_nodes(
            points,
            0.0,
            0.0,
            0.0,
            8.0,
            &cfg,
            (0.01, 0.01, 0.01),
            (0.0, 0.0, 0.0),
        );

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes.get(&VoxelKey::ROOT).map(Vec::len), Some(3));
    }

    #[test]
    fn occupancy_partition_keeps_root_payload_when_subdividing() {
        let points: Vec<PointRecord> = (0..20)
            .map(|i| PointRecord {
                x: f64::from(i % 5) * 0.1,
                y: f64::from((i / 5) % 2) * 0.1,
                z: f64::from(i / 10) * 0.1,
                ..PointRecord::default()
            })
            .collect();

        let mut cfg = CopcWriterConfig::default();
        cfg.max_depth = 8;
        cfg.spacing = 0.000_001;
        cfg.max_points_per_node = 2;

        let nodes = super::partition_points_into_nodes(
            points,
            0.0,
            0.0,
            0.0,
            8.0,
            &cfg,
            (0.01, 0.01, 0.01),
            (0.0, 0.0, 0.0),
        );

        assert!(nodes.get(&VoxelKey::ROOT).map(Vec::len).unwrap_or(0) > 0);
    }

    #[test]
    fn node_points_are_sorted_by_gps_time() {
        let mut pts = vec![
            PointRecord { gps_time: Some(GpsTime(5.0)), ..PointRecord::default() },
            PointRecord { gps_time: Some(GpsTime(2.0)), ..PointRecord::default() },
            PointRecord { gps_time: Some(GpsTime(3.0)), ..PointRecord::default() },
        ];

        sort_points_by_gps_time(&mut pts);
        let got: Vec<f64> = pts
            .iter()
            .map(|p| p.gps_time.map_or(-1.0, |t| t.0))
            .collect();
        assert_eq!(got, vec![2.0, 3.0, 5.0]);
    }

    #[test]
    fn order_node_points_auto_prefers_gps_time_when_available() {
        let mut pts = vec![
            PointRecord {
                x: 10.0,
                y: 0.0,
                gps_time: Some(GpsTime(5.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 1.0,
                y: 0.0,
                gps_time: Some(GpsTime(2.0)),
                ..PointRecord::default()
            },
            PointRecord {
                x: 5.0,
                y: 0.0,
                gps_time: Some(GpsTime(3.0)),
                ..PointRecord::default()
            },
        ];

        order_node_points(
            &mut pts,
            CopcNodePointOrdering::Auto,
            0.01,
            0.01,
            0.0,
            0.0,
        );

        let got: Vec<f64> = pts
            .iter()
            .map(|p| p.gps_time.map_or(-1.0, |t| t.0))
            .collect();
        assert_eq!(got, vec![2.0, 3.0, 5.0]);
    }

    #[test]
    fn order_node_points_auto_falls_back_to_morton_without_gps() {
        let mut pts = vec![
            PointRecord { x: 10.0, y: 1.0, gps_time: None, ..PointRecord::default() },
            PointRecord { x: 1.0, y: 20.0, gps_time: None, ..PointRecord::default() },
            PointRecord { x: 5.0, y: 6.0, gps_time: None, ..PointRecord::default() },
        ];
        let mut expected = pts.clone();
        sort_points_by_morton_order(&mut expected, 0.01, 0.01, 0.0, 0.0);

        order_node_points(
            &mut pts,
            CopcNodePointOrdering::Auto,
            0.01,
            0.01,
            0.0,
            0.0,
        );

        let got_xy: Vec<(f64, f64)> = pts.iter().map(|p| (p.x, p.y)).collect();
        let expected_xy: Vec<(f64, f64)> = expected.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(got_xy, expected_xy);
    }

    #[test]
    fn order_node_points_explicit_morton_matches_morton_sort() {
        let mut pts = vec![
            PointRecord { x: 8.0, y: 1.0, gps_time: Some(GpsTime(30.0)), ..PointRecord::default() },
            PointRecord { x: 2.0, y: 9.0, gps_time: Some(GpsTime(10.0)), ..PointRecord::default() },
            PointRecord { x: 4.0, y: 3.0, gps_time: Some(GpsTime(20.0)), ..PointRecord::default() },
        ];
        let mut expected = pts.clone();
        sort_points_by_morton_order(&mut expected, 0.01, 0.01, 0.0, 0.0);

        order_node_points(
            &mut pts,
            CopcNodePointOrdering::Morton,
            0.01,
            0.01,
            0.0,
            0.0,
        );

        let got_xy: Vec<(f64, f64)> = pts.iter().map(|p| (p.x, p.y)).collect();
        let expected_xy: Vec<(f64, f64)> = expected.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(got_xy, expected_xy);
    }

    #[test]
    fn order_node_points_explicit_hilbert_matches_hilbert_sort() {
        let mut pts = vec![
            PointRecord { x: 8.0, y: 1.0, gps_time: Some(GpsTime(30.0)), ..PointRecord::default() },
            PointRecord { x: 2.0, y: 9.0, gps_time: Some(GpsTime(10.0)), ..PointRecord::default() },
            PointRecord { x: 4.0, y: 3.0, gps_time: Some(GpsTime(20.0)), ..PointRecord::default() },
        ];
        let mut expected = pts.clone();
        sort_points_by_hilbert_order(&mut expected, 0.01, 0.01, 0.0, 0.0);

        order_node_points(
            &mut pts,
            CopcNodePointOrdering::Hilbert,
            0.01,
            0.01,
            0.0,
            0.0,
        );

        let got_xy: Vec<(f64, f64)> = pts.iter().map(|p| (p.x, p.y)).collect();
        let expected_xy: Vec<(f64, f64)> = expected.iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(got_xy, expected_xy);
    }

    #[test]
    fn hierarchy_closure_adds_missing_ancestors() {
        let leaf = CopcEntry {
            key: VoxelKey { level: 3, x: 5, y: 2, z: 7 },
            offset: 100,
            byte_size: 25,
            point_count: 9,
        };
        let closed = close_hierarchy_with_ancestors(vec![leaf]);

        assert!(closed.iter().any(|e| e.key == VoxelKey::ROOT));
        assert!(closed.iter().any(|e| e.key == VoxelKey { level: 1, x: 1, y: 0, z: 1 }));
        assert!(closed.iter().any(|e| e.key == VoxelKey { level: 2, x: 2, y: 1, z: 3 }));

        let leaf_entry = closed.iter().find(|e| e.key == leaf.key).expect("leaf preserved");
        assert_eq!(leaf_entry.offset, 100);
        assert_eq!(leaf_entry.byte_size, 25);
        assert_eq!(leaf_entry.point_count, 9);
    }

    #[test]
    fn gps_time_range_uses_present_values() {
        let pts = vec![
            PointRecord { gps_time: Some(crate::point::GpsTime(100.0)), ..PointRecord::default() },
            PointRecord { gps_time: None, ..PointRecord::default() },
            PointRecord { gps_time: Some(crate::point::GpsTime(250.5)), ..PointRecord::default() },
        ];
        let (min, max) = super::gps_time_range(&pts);
        assert_eq!(min, 100.0);
        assert_eq!(max, 250.5);
    }

    #[test]
    fn return_histograms_track_legacy_and_full_bins() {
        let pts = vec![
            PointRecord { return_number: 1, ..PointRecord::default() },
            PointRecord { return_number: 5, ..PointRecord::default() },
            PointRecord { return_number: 8, ..PointRecord::default() },
            PointRecord { return_number: 0, ..PointRecord::default() },
        ];
        let (legacy, full) = super::return_histograms(&pts);
        assert_eq!(legacy[0], 1);
        assert_eq!(legacy[4], 1);
        assert_eq!(full[0], 1);
        assert_eq!(full[4], 1);
        assert_eq!(full[7], 1);
    }

    #[test]
    fn hierarchy_is_paginated_when_entry_count_exceeds_limit() -> crate::Result<()> {
        let leaves: Vec<CopcEntry> = (0..(super::HIERARCHY_PAGE_MAX_ENTRIES + 5))
            .map(|i| CopcEntry {
                key: VoxelKey { level: 10, x: i as i32, y: 0, z: 0 },
                offset: 1000 + i as u64,
                byte_size: 10,
                point_count: 1,
            })
            .collect();

        let entries = super::close_hierarchy_with_ancestors(leaves);

        let (root_bytes, subpages) = super::build_hierarchy_pages(entries, 200)?;
        let root = CopcHierarchy::from_bytes(&root_bytes)?;

        assert!(!subpages.is_empty());
        assert!(root.entries.len() <= super::HIERARCHY_PAGE_MAX_ENTRIES);
        assert!(root.entries.iter().any(|e| e.key == VoxelKey::ROOT));
        assert!(root.entries.iter().any(|e| e.point_count < 0 && e.byte_size > 0));

        for page_bytes in &subpages {
            let page = CopcHierarchy::from_bytes(page_bytes)?;
            assert!(!page.entries.is_empty());
        }
        Ok(())
    }

    #[test]
    fn writer_roundtrip_preserves_count_bounds_and_crs() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf7;
        cfg.las.crs = Some(Crs::from_epsg(4326));

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord {
                x: -80.125,
                y: 43.25,
                z: 300.5,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(crate::point::GpsTime(12.5)),
                color: Some(crate::point::Rgb16 {
                    red: 1000,
                    green: 2000,
                    blue: 3000,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let bytes = cursor.into_inner();

        let mut strict_cur = Cursor::new(bytes.clone());
        let mut copc_reader = CopcReader::new_with_mode(&mut strict_cur, CopcReaderMode::Strict)?;
        assert_eq!(copc_reader.header().point_count(), 1);
        assert!((copc_reader.header().min_x - (-80.125)).abs() < 1e-9);
        assert!((copc_reader.header().max_x - (-80.125)).abs() < 1e-9);
        assert!((copc_reader.header().min_y - 43.25).abs() < 1e-9);
        assert!((copc_reader.header().max_y - 43.25).abs() < 1e-9);
        assert!((copc_reader.header().min_z - 300.5).abs() < 1e-9);
        assert!((copc_reader.header().max_z - 300.5).abs() < 1e-9);

        let points = copc_reader.read_all_nodes()?;
        assert_eq!(points.len(), 1);
        assert!((points[0].x - (-80.125)).abs() < 1e-9);
        assert!((points[0].y - 43.25).abs() < 1e-9);
        assert!((points[0].z - 300.5).abs() < 1e-9);

        let mut las_cur = Cursor::new(bytes);
        let las_reader = LasReader::new(&mut las_cur)?;
        let wkt = find_ogc_wkt(las_reader.vlrs()).unwrap_or_default();
        assert!(wkt.contains("4326") || wkt.to_ascii_uppercase().contains("WGS"));

        Ok(())
    }

    #[test]
    fn strict_mode_rejects_non_encodable_point14_points() {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        let err = {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 1,
                    return_number: 1,
                    number_of_returns: 1,
                    classification: 1,
                    color: Some(crate::point::Rgb16 {
                        red: 1200,
                        green: 2200,
                        blue: 3200,
                    }),
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            match writer.finish() {
                Ok(_) => panic!("strict mode should reject non-encodable Point14 point set"),
                Err(err) => err,
            }
        };

        assert!(format!("{err}").contains("point cannot be represented in requested Point14 format"));
    }

    #[test]
    fn strict_mode_accepts_multipoint_subset() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 4.0,
                y: 5.0,
                z: 6.0,
                intensity: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        Ok(())
    }

    #[test]
    fn default_mode_accepts_multipoint_standard_point14_encoding() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 4.0,
                y: 5.0,
                z: 6.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Tolerant)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        Ok(())
    }

    #[test]
    fn default_mode_rejects_point14_non_representable_points() {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf7;
        cfg.max_depth = 0; // force all points into one node

        let err = {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            match writer.finish() {
                Ok(_) => panic!("default mode should reject non-representable Point14 point set"),
                Err(err) => err,
            }
        };

        assert!(format!("{err}").contains("point cannot be represented in requested Point14 format"));
    }

    #[test]
    fn promotes_non_point14_input_format_to_point14_family() {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf3;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                })
                .expect("point should buffer");
            writer.finish().expect("finish should succeed with promoted format");
        }

        cursor.set_position(0);
        let las_reader = crate::las::reader::LasReader::new(&mut cursor)
            .expect("header should read");
        let header = las_reader.header().clone();
        assert_eq!(header.point_data_format, crate::las::header::PointDataFormat::Pdrf6);
    }

    #[test]
    fn promotes_non_point14_with_color_to_pdrf7() {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf3;
        cfg.max_depth = 0;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    return_number: 1,
                    number_of_returns: 1,
                    color: Some(crate::point::Rgb16 {
                        red: 100,
                        green: 200,
                        blue: 300,
                    }),
                    ..PointRecord::default()
                })
                .expect("point should buffer");
            writer.finish().expect("finish should succeed");
        }

        cursor.set_position(0);
        let las_reader = crate::las::reader::LasReader::new(&mut cursor)
            .expect("header should read");
        let header = las_reader.header().clone();
        assert_eq!(header.point_data_format, crate::las::header::PointDataFormat::Pdrf7);
    }

    #[test]
    fn promotes_waveform_point14_input_to_non_waveform_copc_format() {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf9;
        cfg.max_depth = 0;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    return_number: 1,
                    number_of_returns: 1,
                    waveform: Some(crate::point::WaveformPacket {
                        descriptor_index: 1,
                        byte_offset: 2,
                        packet_size: 3,
                        return_point_location: 0.4,
                        dx: 0.1,
                        dy: 0.2,
                        dz: 0.3,
                    }),
                    ..PointRecord::default()
                })
                .expect("point should buffer");
            writer.finish().expect("finish should succeed");
        }

        cursor.set_position(0);
        let las_reader = crate::las::reader::LasReader::new(&mut cursor)
            .expect("header should read");
        let header = las_reader.header().clone();
        assert_eq!(header.point_data_format, crate::las::header::PointDataFormat::Pdrf6);
    }

    #[test]
    fn promotes_v15_input_formats_to_copc_point14_family() {
        use crate::las::header::PointDataFormat;

        let mapping = [
            (PointDataFormat::Pdrf11, PointDataFormat::Pdrf6),
            (PointDataFormat::Pdrf12, PointDataFormat::Pdrf7),
            (PointDataFormat::Pdrf13, PointDataFormat::Pdrf8),
            (PointDataFormat::Pdrf14, PointDataFormat::Pdrf7),
            (PointDataFormat::Pdrf15, PointDataFormat::Pdrf8),
        ];

        for (input_fmt, expected_fmt) in mapping {
            let promoted = super::promote_to_copc_point_format(input_fmt, &[]);
            assert_eq!(promoted, expected_fmt, "failed mapping for {:?}", input_fmt);
        }
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_intensity_change() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 110,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].intensity, 100);
        assert_eq!(points[1].intensity, 110);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_classification_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 5,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].classification, 2);
        assert_ne!(points[1].classification, points[0].classification);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_user_data_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 21,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].user_data, 11);
        assert_ne!(points[1].user_data, points[0].user_data);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_scan_angle_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 12,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].scan_angle, 3);
        assert_ne!(points[1].scan_angle, points[0].scan_angle);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_point_source_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 77,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].point_source_id, 10);
        assert_ne!(points[1].point_source_id, points[0].point_source_id);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_gps_time_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    gps_time: Some(crate::point::GpsTime(1000.0)),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    gps_time: Some(crate::point::GpsTime(1001.5)),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].gps_time.map(|t| t.0), Some(1000.0));
        assert_eq!(points[1].gps_time.map(|t| t.0), Some(1001.5));
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_nir_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf8;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    nir: Some(100),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    nir: Some(140),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].nir, Some(100));
        assert_ne!(points[1].nir, points[0].nir);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_rgb_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf8;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    nir: Some(100),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1200,
                        green: 2400,
                        blue: 3600,
                    }),
                    nir: Some(100),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].color.map(|c| c.red), Some(1000));
        assert_ne!(points[1].color, points[0].color);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_rgb_change_pdrf7(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf7;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1200,
                        green: 2200,
                        blue: 3200,
                    }),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].color.map(|c| c.red), Some(1000));
        assert_ne!(points[1].color, points[0].color);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_flags_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    return_number: 1,
                    number_of_returns: 1,
                    scan_direction_flag: true,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert!(!points[0].scan_direction_flag);
        assert!(points[1].scan_direction_flag);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_scanner_channel_with_return_fields_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf6;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    return_number: 1,
                    number_of_returns: 2,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    return_number: 2,
                    number_of_returns: 2,
                    flags: 0x10,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].return_number, 1);
        assert_eq!(points[0].number_of_returns, 2);
        assert_eq!(points[1].return_number, 2);
        assert_eq!(points[1].number_of_returns, 2);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_multipoint_rgb_nir_change(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf8;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    nir: Some(100),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1200,
                        green: 2400,
                        blue: 3600,
                    }),
                    nir: Some(180),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].color.map(|c| c.red), Some(1000));
        assert_ne!(points[1].color, points[0].color);
        assert_eq!(points[0].nir, Some(100));
        assert_ne!(points[1].nir, points[0].nir);
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_multipoint_rgb_change_pdrf7(
    ) -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = crate::las::header::PointDataFormat::Pdrf7;
        cfg.max_depth = 0; // force all points into one node

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer
                .write_point(&PointRecord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1000,
                        green: 2000,
                        blue: 3000,
                    }),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("first point should buffer");
            writer
                .write_point(&PointRecord {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                    intensity: 100,
                    classification: 2,
                    user_data: 11,
                    scan_angle: 3,
                    point_source_id: 10,
                    color: Some(crate::point::Rgb16 {
                        red: 1200,
                        green: 2400,
                        blue: 3600,
                    }),
                    return_number: 1,
                    number_of_returns: 1,
                    flags: 0x00,
                    ..PointRecord::default()
                })
                .expect("second point should buffer");

            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].color.map(|c| c.red), Some(1000));
        assert_ne!(points[1].color, points[0].color);
        Ok(())
    }
}
