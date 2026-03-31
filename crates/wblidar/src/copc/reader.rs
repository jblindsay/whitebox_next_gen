//! COPC reader — reads points from individual hierarchy nodes.

use std::collections::HashSet;
use std::io::{Read, Seek};
use crate::copc::hierarchy::{CopcEntry, CopcHierarchy, CopcInfo, VoxelKey};
use crate::copc::range_io::{ByteRangeSource, LocalFileRangeSource};
use crate::copc::{COPC_INFO_RECORD_ID, COPC_USER_ID};
use crate::io::PointReader;
use crate::las::{LasHeader, LasReader};

use crate::laz::standard_point10::decode_standard_pointwise_chunk_point10_v2;
use crate::laz::standard_point14::decode_standard_layered_chunk_point14_v3_with_status;
use crate::laz::{parse_laszip_vlr, LaszipCompressorType, LaszipVlrInfo, LASZIP_RECORD_ID, LASZIP_USER_ID};
use crate::las::header::PointDataFormat;
use crate::point::PointRecord;
use crate::{Error, Result};

/// Indicates how the COPC hierarchy root offset was interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopcHierarchyParseMode {
    /// `hierarchy_root_offset` points directly to hierarchy data bytes.
    DataOffset,
    /// `hierarchy_root_offset` points to an EVLR header and data starts at `offset + 60`.
    EvlrHeaderOffset,
}

/// Reader behavior when validating COPC structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopcReaderMode {
    /// Default mode: tolerate known producer quirks where practical.
    #[default]
    Tolerant,
    /// Enforce COPC 1.0 structure more strictly.
    Strict,
}

/// Axis-aligned world-space bounding box used for hierarchy-key queries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopcBoundingBox {
    /// Minimum X coordinate.
    pub min_x: f64,
    /// Maximum X coordinate.
    pub max_x: f64,
    /// Minimum Y coordinate.
    pub min_y: f64,
    /// Maximum Y coordinate.
    pub max_y: f64,
    /// Minimum Z coordinate.
    pub min_z: f64,
    /// Maximum Z coordinate.
    pub max_z: f64,
}

/// A COPC reader that can fetch individual octree nodes.
pub struct CopcReader<R: Read + Seek + ByteRangeSource> {
    inner: R,
    header: LasHeader,
    laszip_info: Option<LaszipVlrInfo>,
    /// COPC metadata block parsed from the COPC info VLR.
    pub info: CopcInfo,
    /// In-memory hierarchy parsed from the COPC hierarchy EVLR.
    pub hierarchy: CopcHierarchy,
    /// Parsing mode used for the root hierarchy offset.
    pub hierarchy_parse_mode: CopcHierarchyParseMode,
    point14_partial_events: u64,
    point14_partial_decoded_points: u64,
    point14_partial_expected_points: u64,
    sequential_points: Option<Vec<PointRecord>>,
    sequential_pos: usize,
}

impl<R: Read + Seek + ByteRangeSource> CopcReader<R> {
    /// Open a COPC reader.  Parses the LAS header, locates the COPC info VLR
    /// and the hierarchy EVLR, and pre-loads the hierarchy.
    pub fn new(inner: R) -> Result<Self> {
        Self::new_with_mode(inner, CopcReaderMode::Tolerant)
    }

    /// Open a COPC reader with explicit strict/tolerant validation behavior.
    pub fn new_with_mode(mut inner: R, mode: CopcReaderMode) -> Result<Self> {
        let las_reader = LasReader::new(&mut inner)?;
        let header = las_reader.header().clone();
        let vlrs = las_reader.vlrs().to_vec();
        let laszip_info = parse_laszip_vlr(&vlrs);

        // Find COPC info VLR
        let info_vlr = vlrs.iter().find(|v| {
            v.key.user_id == COPC_USER_ID && v.key.record_id == COPC_INFO_RECORD_ID
        }).ok_or_else(|| Error::InvalidValue {
            field: "copc_info_vlr",
            detail: "COPC info VLR not found".to_owned(),
        })?;

        if info_vlr.data.len() < CopcInfo::SIZE {
            return Err(Error::SizeMismatch {
                context: "CopcInfo VLR",
                expected: CopcInfo::SIZE,
                actual: info_vlr.data.len(),
            });
        }
        if matches!(mode, CopcReaderMode::Strict) {
            validate_copc_header_strict(
                &mut inner,
                &header,
                &vlrs,
                laszip_info.as_ref(),
                info_vlr,
            )?;
        }
        let info = CopcInfo::from_bytes(&info_vlr.data[..CopcInfo::SIZE])?;
        if matches!(mode, CopcReaderMode::Strict) {
            validate_copc_info_strict(&info, &info_vlr.data[..CopcInfo::SIZE])?;
        }

        // Read hierarchy bytes. Different producers may store
        // hierarchy_root_offset as either:
        // 1) the start of hierarchy data, or
        // 2) the start of the hierarchy EVLR header (60 bytes before data).
        // Try both interpretations and select the plausible parse.
        let file_end = inner.len()?;
        let (hierarchy, hierarchy_parse_mode) = read_best_hierarchy(
            &mut inner,
            info.hierarchy_root_offset,
            info.hierarchy_root_size,
            file_end,
            mode,
        )?;

        if matches!(mode, CopcReaderMode::Strict)
            && hierarchy_parse_mode != CopcHierarchyParseMode::DataOffset
        {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy_root_offset",
                detail: "strict mode requires hierarchy_root_offset to point directly to hierarchy data".to_string(),
            });
        }

        Ok(CopcReader {
            inner,
            header,
            laszip_info,
            info,
            hierarchy,
            hierarchy_parse_mode,
            point14_partial_events: 0,
            point14_partial_decoded_points: 0,
            point14_partial_expected_points: 0,
            sequential_points: None,
            sequential_pos: 0,
        })
    }

    /// Read all points in a given voxel node into `out`.  Returns the number
    /// of points read.
    pub fn read_node(&mut self, key: VoxelKey, out: &mut Vec<PointRecord>) -> Result<usize> {
        let entry = self.hierarchy.find(key)
            .ok_or_else(|| Error::InvalidValue {
                field: "voxel_key",
                detail: format!("key ({},{},{},{}) not found", key.level, key.x, key.y, key.z),
            })?
            .clone();

        if entry.point_count <= 0 || entry.byte_size <= 0 { return Ok(0); }

        if let Ok(file_end) = self.inner.len() {
            let expected_end = entry.offset.saturating_add(entry.byte_size as u64);
            if expected_end > file_end {
                return Err(Error::InvalidValue {
                    field: "copc.node.bounds",
                    detail: format!(
                        "node key ({},{},{},{}) declares offset={} byte_size={} (end={}) beyond file size {}",
                        entry.key.level,
                        entry.key.x,
                        entry.key.y,
                        entry.key.z,
                        entry.offset,
                        entry.byte_size,
                        expected_end,
                        file_end
                    ),
                });
            }
        }

        let mut compressed = vec![0u8; entry.byte_size as usize];
        self.inner.read_exact_at(entry.offset, &mut compressed).map_err(|e| Error::InvalidValue {
            field: "copc.node.read",
            detail: format!(
                "failed to read node key ({},{},{},{}): offset={}, byte_size={}, point_count={}, error={}",
                entry.key.level,
                entry.key.x,
                entry.key.y,
                entry.key.z,
                entry.offset,
                entry.byte_size,
                entry.point_count,
                e
            ),
        })?;

        let (points, already_scaled) = self.decode_node_points(&compressed, entry.point_count as usize)?;

        // Apply scale/offset
        let (sx, sy, sz) = (self.header.x_scale, self.header.y_scale, self.header.z_scale);
        let (ox, oy, oz) = (self.header.x_offset, self.header.y_offset, self.header.z_offset);

        let start = out.len();
        if already_scaled {
            out.extend(points);
        } else {
            out.extend(points.into_iter().map(|mut p| {
                p.x = p.x * sx + ox;
                p.y = p.y * sy + oy;
                p.z = p.z * sz + oz;
                p
            }));
        }
        Ok(out.len() - start)
    }

    fn decode_node_points(
        &mut self,
        compressed: &[u8],
        point_count: usize,
    ) -> Result<(Vec<PointRecord>, bool)> {
        let _has_gps = self.header.point_data_format.has_gps_time();
        let _has_rgb = self.header.point_data_format.has_rgb();
        let scales = [self.header.x_scale, self.header.y_scale, self.header.z_scale];
        let offsets = [self.header.x_offset, self.header.y_offset, self.header.z_offset];

        if let Some(info) = self.laszip_info.as_ref() {
            let declared_standard = info.uses_arithmetic_coder()
                && matches!(
                    info.compressor,
                    LaszipCompressorType::PointWise
                        | LaszipCompressorType::PointWiseChunked
                        | LaszipCompressorType::LayeredChunked
                );

            if declared_standard && info.has_point10_item() && !info.has_point14_item() {
                if let Ok(points) = decode_standard_pointwise_chunk_point10_v2(
                    compressed,
                    point_count,
                    &info.items,
                    self.header.point_data_format,
                    self.header.extra_bytes_count as usize,
                    scales,
                    offsets,
                ) {
                    return Ok((points, true));
                }
            }

            if declared_standard && info.has_point14_item() {
                // Attempt standard Point14 layered decoding only.
                return decode_standard_layered_chunk_point14_v3_with_status(
                    compressed,
                    point_count,
                    &info.items,
                    self.header.point_data_format,
                    scales,
                    offsets,
                )
                .and_then(|(points, status)| {
                    if status.partial && fail_on_partial_point14() {
                        return Err(Error::InvalidValue {
                            field: "copc.point14.partial",
                            detail: format!(
                                "decoded {} of {} points in strict partial-check mode",
                                status.decoded_points, status.expected_points
                            ),
                        });
                    }
                    if status.partial {
                        self.point14_partial_events += 1;
                        self.point14_partial_decoded_points += status.decoded_points as u64;
                        self.point14_partial_expected_points += status.expected_points as u64;
                    }
                    Ok((points, true))
                });
            }
        }

        // Only standards-compliant LASzip v2/v3 Point10 encoding is supported as a fallback.
        // Legacy wb-native DEFLATE chunks are no longer supported.
        Err(Error::Unimplemented(
            "only standards-compliant LASzip v2/v3 Point10/Point14 encoding is supported",
        ))
    }

    /// Read all points from the root node and recurse to all children.
    pub fn read_all_nodes(&mut self) -> Result<Vec<PointRecord>> {
        let keys = self.data_node_keys();
        let total: usize = self.hierarchy.entries.iter()
            .filter(|e| e.point_count > 0)
            .map(|e| e.point_count as usize)
            .sum();
        let mut out = Vec::with_capacity(total);
        for key in keys {
            self.read_node(key, &mut out)?;
        }
        Ok(out)
    }

    /// Return the LAS header.
    pub fn header(&self) -> &LasHeader { &self.header }

    /// Return keys for all hierarchy entries that carry point payloads.
    pub fn data_node_keys(&self) -> Vec<VoxelKey> {
        self.hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0)
            .map(|e| e.key)
            .collect()
    }

    /// Return data-node keys capped at the requested octree depth.
    pub fn data_node_keys_max_depth(&self, max_depth: i32) -> Vec<VoxelKey> {
        self.hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0 && e.key.level <= max_depth)
            .map(|e| e.key)
            .collect()
    }

    /// Return data-node keys under a subtree root (including the root key itself).
    pub fn data_node_keys_subtree(&self, subtree_root: VoxelKey) -> Vec<VoxelKey> {
        self.hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0 && is_descendant_key(e.key, subtree_root))
            .map(|e| e.key)
            .collect()
    }

    /// Return data-node keys whose voxel bounds intersect a world-space bbox.
    pub fn data_node_keys_bbox(&self, bbox: CopcBoundingBox) -> Vec<VoxelKey> {
        self.hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0 && voxel_key_intersects_bbox(&self.info, e.key, bbox))
            .map(|e| e.key)
            .collect()
    }

    /// Return data-node keys matching optional subtree, bbox, and max-depth constraints.
    pub fn query_data_node_keys(
        &self,
        subtree_root: Option<VoxelKey>,
        bbox: Option<CopcBoundingBox>,
        max_depth: Option<i32>,
    ) -> Vec<VoxelKey> {
        self.hierarchy
            .entries
            .iter()
            .filter(|e| e.point_count > 0)
            .filter(|e| max_depth.map_or(true, |d| e.key.level <= d))
            .filter(|e| subtree_root.map_or(true, |r| is_descendant_key(e.key, r)))
            .filter(|e| bbox.map_or(true, |b| voxel_key_intersects_bbox(&self.info, e.key, b)))
            .map(|e| e.key)
            .collect()
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
}

impl CopcReader<LocalFileRangeSource> {
    /// Open a COPC reader backed by a local file path.
    pub fn open_path(path: &std::path::Path) -> Result<Self> {
        Self::open_path_with_mode(path, CopcReaderMode::Tolerant)
    }

    /// Open a COPC reader backed by a local file path with explicit mode.
    pub fn open_path_with_mode(path: &std::path::Path, mode: CopcReaderMode) -> Result<Self> {
        let src = LocalFileRangeSource::open(path)?;
        CopcReader::new_with_mode(src, mode)
    }
}

#[cfg(feature = "copc-http")]
impl CopcReader<crate::copc::range_io::HttpRangeSource> {
    /// Open a COPC reader backed by an HTTP byte-range source.
    pub fn open_url(url: &str) -> Result<Self> {
        Self::open_url_with_mode(url, CopcReaderMode::Tolerant)
    }

    /// Open an HTTP-backed COPC reader with explicit strict/tolerant mode.
    pub fn open_url_with_mode(url: &str, mode: CopcReaderMode) -> Result<Self> {
        let src = crate::copc::range_io::HttpRangeSource::new(url)?;
        CopcReader::new_with_mode(src, mode)
    }
}

#[cfg(feature = "copc-http")]
impl CopcReader<crate::copc::range_io::CachedRangeSource<crate::copc::range_io::HttpRangeSource>> {
    /// Open an HTTP-backed COPC reader with a fixed-size exact-range cache.
    pub fn open_url_cached(url: &str, max_cache_entries: usize) -> Result<Self> {
        Self::open_url_cached_with_mode(url, max_cache_entries, CopcReaderMode::Tolerant)
    }

    /// Open a cached HTTP-backed COPC reader with explicit strict/tolerant mode.
    pub fn open_url_cached_with_mode(
        url: &str,
        max_cache_entries: usize,
        mode: CopcReaderMode,
    ) -> Result<Self> {
        let src = crate::copc::range_io::HttpRangeSource::new(url)?;
        let cached = crate::copc::range_io::CachedRangeSource::new(src, max_cache_entries);
        CopcReader::new_with_mode(cached, mode)
    }
}

fn validate_copc_header_strict<R: Read + Seek + ByteRangeSource>(
    inner: &mut R,
    header: &LasHeader,
    vlrs: &[crate::las::Vlr],
    laszip_info: Option<&LaszipVlrInfo>,
    info_vlr: &crate::las::Vlr,
) -> Result<()> {
    if header.version_major != 1 || header.version_minor != 4 {
        return Err(Error::InvalidValue {
            field: "copc.version",
            detail: format!(
                "strict mode requires LAS 1.4, found {}.{}",
                header.version_major, header.version_minor
            ),
        });
    }

    if header.header_size != 375 {
        return Err(Error::InvalidValue {
            field: "copc.header_size",
            detail: format!(
                "strict mode requires LAS 1.4 header size 375, found {}",
                header.header_size
            ),
        });
    }

    if !matches!(header.point_data_format, PointDataFormat::Pdrf6 | PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8) {
        return Err(Error::InvalidValue {
            field: "copc.point_data_format",
            detail: format!(
                "strict mode requires COPC PDRF 6, 7, or 8, found {:?}",
                header.point_data_format
            ),
        });
    }

    if header.number_of_vlrs == 0 || vlrs.is_empty() {
        return Err(Error::InvalidValue {
            field: "copc.vlrs",
            detail: "strict mode requires at least one VLR and the first VLR must be COPC info".to_string(),
        });
    }

    let first_vlr = &vlrs[0];
    if first_vlr.key.user_id != COPC_USER_ID || first_vlr.key.record_id != COPC_INFO_RECORD_ID {
        return Err(Error::InvalidValue {
            field: "copc.info_vlr.first",
            detail: format!(
                "strict mode requires first VLR to be COPC info, found user_id='{}' record_id={}",
                first_vlr.key.user_id, first_vlr.key.record_id
            ),
        });
    }

    if info_vlr.data.len() != CopcInfo::SIZE {
        return Err(Error::SizeMismatch {
            context: "strict CopcInfo VLR",
            expected: CopcInfo::SIZE,
            actual: info_vlr.data.len(),
        });
    }

    if laszip_info.is_none() || !vlrs.iter().any(|v| v.key.user_id == LASZIP_USER_ID && v.key.record_id == LASZIP_RECORD_ID) {
        return Err(Error::InvalidValue {
            field: "copc.laszip_vlr",
            detail: "strict mode requires a valid LASzip VLR for LAZ payload metadata".to_string(),
        });
    }

    let mut copc_magic = [0u8; 4];
    inner.read_exact_at(377, &mut copc_magic)?;
    if &copc_magic != b"copc" {
        return Err(Error::InvalidValue {
            field: "copc.magic",
            detail: format!(
                "strict mode expected 'copc' at offset 377, found {:?}",
                copc_magic
            ),
        });
    }

    let mut record_id = [0u8; 2];
    inner.read_exact_at(393, &mut record_id)?;
    if record_id != COPC_INFO_RECORD_ID.to_le_bytes() {
        return Err(Error::InvalidValue {
            field: "copc.info_record_id",
            detail: format!(
                "strict mode expected COPC info record id {} at offset 393, found {}",
                COPC_INFO_RECORD_ID,
                u16::from_le_bytes(record_id)
            ),
        });
    }

    Ok(())
}

fn validate_copc_info_strict(info: &CopcInfo, raw_info_bytes: &[u8]) -> Result<()> {
    if raw_info_bytes.len() != CopcInfo::SIZE {
        return Err(Error::SizeMismatch {
            context: "strict CopcInfo bytes",
            expected: CopcInfo::SIZE,
            actual: raw_info_bytes.len(),
        });
    }

    if raw_info_bytes[72..].iter().any(|&b| b != 0) {
        return Err(Error::InvalidValue {
            field: "copc.info.reserved",
            detail: "strict mode requires CopcInfo reserved bytes to be zero".to_string(),
        });
    }

    if info.hierarchy_root_size == 0 || info.hierarchy_root_size % CopcEntry::SIZE as u64 != 0 {
        return Err(Error::InvalidValue {
            field: "copc.info.root_hierarchy_size",
            detail: format!(
                "strict mode requires root hierarchy size to be a non-zero multiple of {}, found {}",
                CopcEntry::SIZE,
                info.hierarchy_root_size
            ),
        });
    }

    Ok(())
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

fn read_hierarchy_bytes<R: ByteRangeSource>(
    inner: &mut R,
    data_offset: u64,
    data_size: u64,
) -> Result<Vec<u8>> {
    inner.read_range(data_offset, data_size as usize)
}

fn hierarchy_entries_plausible(entries: &[CopcEntry], file_end: u64, require_root: bool) -> bool {
    if entries.is_empty() {
        return false;
    }

    if require_root && !entries.iter().any(|e| e.key == VoxelKey::ROOT) {
        return false;
    }

    for e in entries {
        if e.key.level < 0 || e.key.level > 31 {
            return false;
        }
        if e.key.x < 0 || e.key.y < 0 || e.key.z < 0 {
            return false;
        }
        if e.byte_size < -1 || e.point_count < -1 {
            return false;
        }
        if e.byte_size > 0 {
            let end = e.offset.saturating_add(e.byte_size as u64);
            if end > file_end {
                return false;
            }
        }
    }

    true
}

fn validate_hierarchy_entries_strict(entries: &[CopcEntry], file_end: u64, require_root: bool) -> Result<()> {
    if entries.is_empty() {
        return Err(Error::InvalidValue {
            field: "copc.hierarchy.entries",
            detail: "strict mode requires at least one hierarchy entry".to_string(),
        });
    }

    if require_root && !entries.iter().any(|e| e.key == VoxelKey::ROOT) {
        return Err(Error::InvalidValue {
            field: "copc.hierarchy.root",
            detail: "strict mode requires a root hierarchy entry".to_string(),
        });
    }

    for e in entries {
        if e.key.level < 0 || e.key.x < 0 || e.key.y < 0 || e.key.z < 0 {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy.key",
                detail: format!(
                    "strict mode found invalid voxel key ({},{},{},{})",
                    e.key.level, e.key.x, e.key.y, e.key.z
                ),
            });
        }
        if e.point_count < -1 || e.byte_size < -1 {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy.entry",
                detail: format!(
                    "strict mode found invalid hierarchy state byte_size={} point_count={} for key ({},{},{},{})",
                    e.byte_size, e.point_count, e.key.level, e.key.x, e.key.y, e.key.z
                ),
            });
        }

        if e.point_count == -1 {
            if e.byte_size <= 0 || e.byte_size as usize % CopcEntry::SIZE != 0 {
                return Err(Error::InvalidValue {
                    field: "copc.hierarchy.subpage",
                    detail: format!(
                        "strict mode requires sub-page byte_size to be a positive multiple of {}, found {} for key ({},{},{},{})",
                        CopcEntry::SIZE,
                        e.byte_size,
                        e.key.level,
                        e.key.x,
                        e.key.y,
                        e.key.z
                    ),
                });
            }
        }

        if e.byte_size > 0 {
            let end = e.offset.saturating_add(e.byte_size as u64);
            if end > file_end {
                return Err(Error::InvalidValue {
                    field: "copc.hierarchy.bounds",
                    detail: format!(
                        "strict mode found hierarchy entry ending at {} beyond file size {} for key ({},{},{},{})",
                        end, file_end, e.key.level, e.key.x, e.key.y, e.key.z
                    ),
                });
            }
        }
    }

    Ok(())
}

fn hierarchy_subpage_refs(entries: &[CopcEntry]) -> Vec<(u64, u64)> {
    entries
        .iter()
        .filter_map(|e| {
            if e.point_count < 0 && e.byte_size > 0 {
                Some((e.offset, e.byte_size as u64))
            } else {
                None
            }
        })
        .collect()
}

fn upsert_entry(entries: &mut Vec<CopcEntry>, incoming: CopcEntry) {
    if let Some(idx) = entries.iter().position(|e| e.key == incoming.key) {
        let existing = entries[idx];
        let existing_data = existing.point_count > 0 && existing.byte_size > 0;
        let incoming_data = incoming.point_count > 0 && incoming.byte_size > 0;

        if !existing_data && incoming_data {
            entries[idx] = incoming;
        }
        return;
    }
    entries.push(incoming);
}

fn expand_hierarchy_subpages<R: ByteRangeSource>(
    inner: &mut R,
    hierarchy: &mut CopcHierarchy,
    file_end: u64,
    mode: CopcReaderMode,
) -> Result<()> {
    let mut visited: HashSet<(u64, u64)> = HashSet::new();
    let mut queue = hierarchy_subpage_refs(&hierarchy.entries);

    while let Some((offset, size)) = queue.pop() {
        if !visited.insert((offset, size)) {
            continue;
        }

        if matches!(mode, CopcReaderMode::Strict) && size % CopcEntry::SIZE as u64 != 0 {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy.subpage",
                detail: format!(
                    "strict mode requires sub-page size to be a multiple of {}, found {} at offset {}",
                    CopcEntry::SIZE,
                    size,
                    offset
                ),
            });
        }

        if offset.saturating_add(size) > file_end {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy.subpage",
                detail: format!(
                    "sub-page offset {} with size {} exceeds file size {}",
                    offset, size, file_end
                ),
            });
        }

        let bytes = read_hierarchy_bytes(inner, offset, size)?;
        let page = CopcHierarchy::from_bytes(&bytes)?;
        if matches!(mode, CopcReaderMode::Strict) {
            validate_hierarchy_entries_strict(&page.entries, file_end, false)?;
        } else if !hierarchy_entries_plausible(&page.entries, file_end, false) {
            return Err(Error::InvalidValue {
                field: "copc.hierarchy.subpage",
                detail: format!(
                    "sub-page at offset {} with size {} failed plausibility checks",
                    offset, size
                ),
            });
        }

        for entry in page.entries {
            if entry.point_count < 0 && entry.byte_size > 0 {
                queue.push((entry.offset, entry.byte_size as u64));
            }
            upsert_entry(&mut hierarchy.entries, entry);
        }
    }

    Ok(())
}

fn read_best_hierarchy<R: ByteRangeSource>(
    inner: &mut R,
    hierarchy_root_offset: u64,
    hierarchy_root_size: u64,
    file_end: u64,
    mode: CopcReaderMode,
) -> Result<(CopcHierarchy, CopcHierarchyParseMode)> {
    let mut best: Option<(CopcHierarchy, CopcHierarchyParseMode)> = None;

    if matches!(mode, CopcReaderMode::Strict)
        && (hierarchy_root_size == 0 || hierarchy_root_size % CopcEntry::SIZE as u64 != 0)
    {
        return Err(Error::InvalidValue {
            field: "copc.hierarchy.root_size",
            detail: format!(
                "strict mode requires root hierarchy size to be a non-zero multiple of {}, found {}",
                CopcEntry::SIZE,
                hierarchy_root_size
            ),
        });
    }

    // Candidate A: offset points directly to hierarchy data.
    if hierarchy_root_offset.saturating_add(hierarchy_root_size) <= file_end {
        if let Ok(bytes) = read_hierarchy_bytes(inner, hierarchy_root_offset, hierarchy_root_size) {
            if let Ok(h) = CopcHierarchy::from_bytes(&bytes) {
                let plausible = if matches!(mode, CopcReaderMode::Strict) {
                    validate_hierarchy_entries_strict(&h.entries, file_end, true).is_ok()
                } else {
                    hierarchy_entries_plausible(&h.entries, file_end, true)
                };
                if plausible {
                    best = Some((h, CopcHierarchyParseMode::DataOffset));
                }
            }
        }
    }

    // Candidate B: offset points to EVLR header, data starts 60 bytes later.
    let evlr_data_offset = hierarchy_root_offset.saturating_add(60);
    if !matches!(mode, CopcReaderMode::Strict)
        && evlr_data_offset.saturating_add(hierarchy_root_size) <= file_end
    {
        if let Ok(bytes) = read_hierarchy_bytes(inner, evlr_data_offset, hierarchy_root_size) {
            if let Ok(h) = CopcHierarchy::from_bytes(&bytes) {
                let plausible = if matches!(mode, CopcReaderMode::Strict) {
                    validate_hierarchy_entries_strict(&h.entries, file_end, true).is_ok()
                } else {
                    hierarchy_entries_plausible(&h.entries, file_end, true)
                };
                if plausible {
                    // Prefer candidate A if already valid; otherwise take B.
                    if best.is_none() {
                        best = Some((h, CopcHierarchyParseMode::EvlrHeaderOffset));
                    }
                }
            }
        }
    }

    if let Some((mut h, parse_mode)) = best {
        expand_hierarchy_subpages(inner, &mut h, file_end, mode)?;
        return Ok((h, parse_mode));
    }

    Err(Error::InvalidValue {
        field: "copc.hierarchy",
        detail: format!(
            "failed to parse plausible hierarchy at root offset {} with size {}",
            hierarchy_root_offset, hierarchy_root_size
        ),
    })
}

/// Sequential `PointReader` wrapper: reads nodes depth-first.
impl<R: Read + Seek + ByteRangeSource> PointReader for CopcReader<R> {
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool> {
        if self.sequential_points.is_none() {
            self.sequential_points = Some(self.read_all_nodes()?);
            self.sequential_pos = 0;
        }

        let points = self.sequential_points.as_ref().expect("sequential cache initialized");
        if self.sequential_pos >= points.len() {
            return Ok(false);
        }

        *out = points[self.sequential_pos];
        self.sequential_pos += 1;
        Ok(true)
    }
    fn point_count(&self) -> Option<u64> { Some(self.header.point_count()) }
}


fn is_descendant_key(key: VoxelKey, root: VoxelKey) -> bool {
    if key.level < root.level {
        return false;
    }
    let delta = (key.level - root.level) as u32;
    if delta == 0 {
        return key == root;
    }
    (key.x >> delta) == root.x && (key.y >> delta) == root.y && (key.z >> delta) == root.z
}

fn voxel_key_bounds(info: &CopcInfo, key: VoxelKey) -> (f64, f64, f64, f64, f64, f64) {
    let levels = 2f64.powi(key.level);
    let cell = (info.halfsize * 2.0) / levels;
    let root_min_x = info.center_x - info.halfsize;
    let root_min_y = info.center_y - info.halfsize;
    let root_min_z = info.center_z - info.halfsize;

    let min_x = root_min_x + f64::from(key.x) * cell;
    let min_y = root_min_y + f64::from(key.y) * cell;
    let min_z = root_min_z + f64::from(key.z) * cell;
    let max_x = min_x + cell;
    let max_y = min_y + cell;
    let max_z = min_z + cell;
    (min_x, max_x, min_y, max_y, min_z, max_z)
}

fn voxel_key_intersects_bbox(info: &CopcInfo, key: VoxelKey, bbox: CopcBoundingBox) -> bool {
    let (min_x, max_x, min_y, max_y, min_z, max_z) = voxel_key_bounds(info, key);
    max_x >= bbox.min_x
        && min_x <= bbox.max_x
        && max_y >= bbox.min_y
        && min_y <= bbox.max_y
        && max_z >= bbox.min_z
        && min_z <= bbox.max_z
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    use std::io::Cursor;
    use crate::las::vlr::{Vlr, VlrKey};
    use crate::io::{PointReader, PointWriter};
    use crate::copc::writer::{CopcWriter, CopcWriterConfig};
    use crate::point::PointRecord;
    #[cfg(feature = "copc-http")]
    use std::sync::{Arc, Mutex};
    #[cfg(feature = "copc-http")]
    use std::sync::mpsc;
    #[cfg(feature = "copc-http")]
    use std::thread;
    #[cfg(feature = "copc-http")]
    use std::time::Duration;

    #[cfg(feature = "copc-http")]
    fn parse_range_spec(spec: &str, total_len: usize) -> Option<(usize, usize)> {
        let raw = spec.strip_prefix("bytes=")?;
        let (start_s, end_s) = raw.split_once('-')?;
        let start = start_s.parse::<usize>().ok()?;
        let end = if end_s.is_empty() {
            total_len.checked_sub(1)?
        } else {
            end_s.parse::<usize>().ok()?
        };
        if start > end || end >= total_len {
            return None;
        }
        Some((start, end))
    }

    #[cfg(feature = "copc-http")]
    fn spawn_http_range_server(
        bytes: Vec<u8>,
    ) -> Result<(String, Arc<Mutex<Vec<String>>>, mpsc::Sender<()>, thread::JoinHandle<()>)> {
        use tiny_http::{Header, Method, Response, Server, StatusCode};

        let server = Server::http("127.0.0.1:0").map_err(|e| Error::InvalidValue {
            field: "copc.http.test_server",
            detail: e.to_string(),
        })?;
        let url = format!("http://{}", server.server_addr());
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let log_clone = Arc::clone(&log);
        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let handle = thread::spawn(move || {
            let total_len = bytes.len();
            loop {
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                let req_opt = match server.recv_timeout(Duration::from_millis(50)) {
                    Ok(v) => v,
                    Err(_) => break,
                };
                let Some(req) = req_opt else { continue; };

                let method = req.method().clone();
                let range_header = req
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Range"))
                    .map(|h| h.value.as_str().to_string());

                let mut status = StatusCode(200);
                let mut body = Vec::new();
                let mut content_range: Option<String> = None;

                match method {
                    Method::Head => {
                        body = Vec::new();
                    }
                    Method::Get => {
                        if let Some(spec) = range_header.as_deref() {
                            if let Some((start, end)) = parse_range_spec(spec, total_len) {
                                body.extend_from_slice(&bytes[start..=end]);
                                status = StatusCode(206);
                                content_range = Some(format!(
                                    "bytes {}-{}/{}",
                                    start,
                                    end,
                                    total_len
                                ));
                                if let Ok(mut l) = log_clone.lock() {
                                    l.push(spec.to_string());
                                }
                            } else {
                                status = StatusCode(416);
                            }
                        } else {
                            body.extend_from_slice(&bytes);
                            if let Ok(mut l) = log_clone.lock() {
                                l.push("<full-get>".to_string());
                            }
                        }
                    }
                    _ => {
                        status = StatusCode(405);
                    }
                }

                let body_len = body.len();
                let mut response = Response::from_data(body).with_status_code(status);
                if let Ok(h) = Header::from_bytes(
                    &b"Content-Length"[..],
                    body_len.to_string().as_bytes(),
                ) {
                    response = response.with_header(h);
                }
                if let Some(cr) = content_range {
                    if let Ok(h) = Header::from_bytes(&b"Content-Range"[..], cr.as_bytes()) {
                        response = response.with_header(h);
                    }
                }
                let _ = req.respond(response);
            }
        });

        Ok((url, log, stop_tx, handle))
    }

    #[cfg(feature = "copc-http")]
    fn point_xyz_signature(points: &[PointRecord]) -> (usize, f64, f64, f64) {
        points.iter().fold((0usize, 0.0f64, 0.0f64, 0.0f64), |acc, p| {
            (acc.0 + 1, acc.1 + p.x, acc.2 + p.y, acc.3 + p.z)
        })
    }

    #[test]
    fn expands_hierarchy_subpages() -> Result<()> {
        let subpage_entry = CopcEntry {
            key: VoxelKey { level: 1, x: 1, y: 0, z: 0 },
            offset: 200,
            byte_size: 20,
            point_count: 42,
        };

        let subpage_bytes = CopcHierarchy { entries: vec![subpage_entry] }.to_bytes()?;
        let subpage_offset = 64u64;
        let mut file = vec![0u8; 512];
        file[subpage_offset as usize..subpage_offset as usize + subpage_bytes.len()]
            .copy_from_slice(&subpage_bytes);

        let mut hierarchy = CopcHierarchy {
            entries: vec![
                CopcEntry {
                    key: VoxelKey::ROOT,
                    offset: 0,
                    byte_size: 0,
                    point_count: 0,
                },
                CopcEntry {
                    key: VoxelKey { level: 1, x: 0, y: 0, z: 0 },
                    offset: subpage_offset,
                    byte_size: subpage_bytes.len() as i32,
                    point_count: -1,
                },
            ],
        };

        let mut cur = Cursor::new(file);
        expand_hierarchy_subpages(&mut cur, &mut hierarchy, 512, CopcReaderMode::Tolerant)?;

        assert!(hierarchy.entries.iter().any(|e| {
            e.key == subpage_entry.key && e.point_count == subpage_entry.point_count
        }));
        Ok(())
    }

    #[test]
    fn expands_mixed_root_entries_and_subpages() -> Result<()> {
        let direct_entry = CopcEntry {
            key: VoxelKey { level: 1, x: 2, y: 0, z: 0 },
            offset: 300,
            byte_size: 32,
            point_count: 7,
        };
        let subpage_data_entry = CopcEntry {
            key: VoxelKey { level: 1, x: 3, y: 0, z: 0 },
            offset: 400,
            byte_size: 48,
            point_count: 9,
        };

        let subpage_bytes = CopcHierarchy { entries: vec![subpage_data_entry] }.to_bytes()?;
        let subpage_offset = 80u64;
        let mut file = vec![0u8; 512];
        file[subpage_offset as usize..subpage_offset as usize + subpage_bytes.len()]
            .copy_from_slice(&subpage_bytes);

        let mut hierarchy = CopcHierarchy {
            entries: vec![
                CopcEntry {
                    key: VoxelKey::ROOT,
                    offset: 0,
                    byte_size: 0,
                    point_count: 0,
                },
                direct_entry,
                CopcEntry {
                    key: VoxelKey { level: 1, x: 0, y: 0, z: 0 },
                    offset: subpage_offset,
                    byte_size: subpage_bytes.len() as i32,
                    point_count: -1,
                },
            ],
        };

        let mut cur = Cursor::new(file);
        expand_hierarchy_subpages(&mut cur, &mut hierarchy, 512, CopcReaderMode::Tolerant)?;

        assert!(hierarchy.entries.iter().any(|e| {
            e.key == direct_entry.key && e.point_count == direct_entry.point_count
        }));
        assert!(hierarchy.entries.iter().any(|e| {
            e.key == subpage_data_entry.key && e.point_count == subpage_data_entry.point_count
        }));
        Ok(())
    }

    #[test]
    fn sequential_read_point_works_for_copc() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        {
            let mut writer = CopcWriter::new(&mut cursor, CopcWriterConfig::default());
            writer.write_point(&PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 11.0,
                y: 21.0,
                z: 31.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = CopcReader::new(&mut cursor)?;
        let mut out = PointRecord::default();
        let mut n = 0usize;
        while reader.read_point(&mut out)? {
            n += 1;
        }
        assert_eq!(n, 2);
        Ok(())
    }

    #[test]
    fn reads_points_from_paginated_hierarchy_output() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.spacing = 0.0;
        cfg.max_depth = 8;

        let point_count = 600usize;
        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            let cell_size = 2000.0 / 256.0;
            for i in 0..point_count {
                let xi = (i % 256) as f64;
                let yi = ((i / 256) % 256) as f64;
                let zi = 128.0;
                let p = PointRecord {
                    x: -1000.0 + (xi + 0.5) * cell_size,
                    y: -1000.0 + (yi + 0.5) * cell_size,
                    z: -1000.0 + (zi + 0.5) * cell_size,
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                };
                writer.write_point(&p)?;
            }
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = CopcReader::new(&mut cursor)?;
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), point_count);
        Ok(())
    }

    #[test]
    fn writer_produced_hierarchy_offset_is_readable() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = CopcWriter::new(&mut cursor, CopcWriterConfig::default());
            writer.write_point(&PointRecord {
                x: 5.0,
                y: 6.0,
                z: 7.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let reader = CopcReader::new(&mut cursor)?;
        assert!(reader.hierarchy.entries.iter().any(|e| e.key == VoxelKey::ROOT));
        Ok(())
    }

    #[test]
    fn synthetic_evlr_header_offset_is_readable() -> Result<()> {
        let hierarchy = CopcHierarchy {
            entries: vec![
                CopcEntry {
                    key: VoxelKey::ROOT,
                    offset: 0,
                    byte_size: 0,
                    point_count: 0,
                },
                CopcEntry {
                    key: VoxelKey { level: 1, x: 1, y: 0, z: 0 },
                    offset: 256,
                    byte_size: 32,
                    point_count: 4,
                },
            ],
        };
        let data = hierarchy.to_bytes()?;
        let evlr_header_offset = 100u64;
        let data_offset = evlr_header_offset + 60;

        let mut file = vec![0u8; 512];
        // Use intentionally non-plausible bytes in the EVLR header region so
        // candidate-A parse fails and candidate-B (offset+60) is exercised.
        for b in &mut file[evlr_header_offset as usize..data_offset as usize] {
            *b = 0xFF;
        }
        file[data_offset as usize..data_offset as usize + data.len()].copy_from_slice(&data);

        let file_end = file.len() as u64;
        let mut cur = Cursor::new(file);
        let (parsed, mode) = read_best_hierarchy(
            &mut cur,
            evlr_header_offset,
            data.len() as u64,
            file_end,
            CopcReaderMode::Tolerant,
        )?;

        assert_eq!(mode, CopcHierarchyParseMode::EvlrHeaderOffset);
        assert!(parsed.entries.iter().any(|e| e.key == VoxelKey::ROOT));
        assert!(parsed.entries.iter().any(|e| {
            e.key == VoxelKey { level: 1, x: 1, y: 0, z: 0 } && e.point_count == 4
        }));
        Ok(())
    }

    #[test]
    fn strict_mode_accepts_writer_copc_with_laszip_vlr_and_data_offset_hierarchy() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = CopcWriter::new(&mut cursor, CopcWriterConfig::default());
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        assert_eq!(reader.hierarchy_parse_mode, CopcHierarchyParseMode::DataOffset);
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 1);
        Ok(())
    }

    #[test]
    fn strict_mode_roundtrip_writer_pdrf7_singleton_preserves_rgb_and_gps() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = PointDataFormat::Pdrf7;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord {
                x: 11.0,
                y: 22.0,
                z: 33.0,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(crate::point::GpsTime(1234.5)),
                color: Some(crate::point::Rgb16 {
                    red: 100,
                    green: 200,
                    blue: 300,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        assert_eq!(reader.hierarchy_parse_mode, CopcHierarchyParseMode::DataOffset);
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].gps_time.map(|g| g.0), Some(1234.5));
        let color = points[0].color.expect("expected rgb in pdrf7 roundtrip");
        assert_eq!(color.red, 100);
        assert_eq!(color.green, 200);
        assert_eq!(color.blue, 300);
        Ok(())
    }

    #[test]
    fn strict_mode_roundtrip_writer_pdrf8_singleton_preserves_rgb_nir_and_gps() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.las.point_data_format = PointDataFormat::Pdrf8;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord {
                x: -11.0,
                y: 2.0,
                z: 8.0,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(crate::point::GpsTime(9876.25)),
                color: Some(crate::point::Rgb16 {
                    red: 111,
                    green: 222,
                    blue: 333,
                }),
                nir: Some(444),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let mut reader = CopcReader::new_with_mode(&mut cursor, CopcReaderMode::Strict)?;
        assert_eq!(reader.hierarchy_parse_mode, CopcHierarchyParseMode::DataOffset);
        let points = reader.read_all_nodes()?;
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].gps_time.map(|g| g.0), Some(9876.25));
        let color = points[0].color.expect("expected rgb in pdrf8 roundtrip");
        assert_eq!(color.red, 111);
        assert_eq!(color.green, 222);
        assert_eq!(color.blue, 333);
        assert_eq!(points[0].nir, Some(444));
        Ok(())
    }

    #[test]
    fn query_max_depth_limits_results() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 3;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: -10.0, y: -10.0, z: -10.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 10.0, y: 10.0, z: 10.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let reader = CopcReader::new(&mut cursor)?;
        let keys = reader.data_node_keys_max_depth(0);
        assert!(keys.iter().all(|k| k.level <= 0));
        Ok(())
    }

    #[test]
    fn query_subtree_returns_only_descendants() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 3;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: -10.0, y: -10.0, z: -10.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 10.0, y: 10.0, z: 10.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let reader = CopcReader::new(&mut cursor)?;
        let root_child = VoxelKey { level: 1, x: 1, y: 1, z: 1 };
        let keys = reader.data_node_keys_subtree(root_child);
        assert!(!keys.is_empty());
        assert!(keys.iter().all(|k| is_descendant_key(*k, root_child)));
        Ok(())
    }

    #[test]
    fn query_bbox_filters_keys() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 2;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: 100.0, y: 100.0, z: 100.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: -100.0, y: -100.0, z: -100.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let reader = CopcReader::new(&mut cursor)?;
        let left_bbox = CopcBoundingBox {
            min_x: -1000.0,
            max_x: reader.info.center_x,
            min_y: -1000.0,
            max_y: reader.info.center_y,
            min_z: -1000.0,
            max_z: reader.info.center_z,
        };
        let keys = reader.data_node_keys_bbox(left_bbox);
        assert!(!keys.is_empty());
        assert!(keys.iter().any(|k| k.level >= 1 && k.x == 0));
        Ok(())
    }

    #[test]
    fn query_all_matches_full_data_node_key_set() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 3;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: -100.0, y: -50.0, z: -20.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 50.0, y: 75.0, z: 20.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 120.0, y: -120.0, z: 60.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        cursor.set_position(0);
        let reader = CopcReader::new(&mut cursor)?;
        let mut from_full = reader.data_node_keys();
        let mut from_query = reader.query_data_node_keys(None, None, None);
        from_full.sort_by_key(|k| (k.level, k.x, k.y, k.z));
        from_query.sort_by_key(|k| (k.level, k.x, k.y, k.z));
        assert_eq!(from_full, from_query);
        Ok(())
    }

    #[test]
    fn open_path_uses_local_file_range_backend() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut writer = CopcWriter::new(&mut cursor, CopcWriterConfig::default());
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        path.push(format!("wblidar_copc_open_path_{stamp}.copc.laz"));

        std::fs::write(&path, cursor.into_inner()).map_err(Error::Io)?;
        let mut reader = CopcReader::open_path(&path)?;
        let pts = reader.read_all_nodes()?;
        let _ = remove_file(&path);
        assert_eq!(pts.len(), 1);
        Ok(())
    }

    #[cfg(feature = "copc-http")]
    #[test]
    fn open_url_reads_nodes_via_http_ranges() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 3;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: -10.0, y: -20.0, z: -30.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 10.0, y: 20.0, z: 30.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        let bytes = cursor.into_inner();
        let total_len = bytes.len();
        let (url, log, stop_tx, handle) = spawn_http_range_server(bytes)?;

        let test_result = (|| -> Result<()> {
            let mut reader = CopcReader::open_url(&url)?;
            let keys = reader.data_node_keys();
            assert!(!keys.is_empty());
            let mut pts = Vec::new();
            let _ = reader.read_node(keys[0], &mut pts)?;
            assert!(!pts.is_empty());

            let ranges = log
                .lock()
                .map_err(|_| Error::InvalidValue {
                    field: "copc.http.test_log",
                    detail: "range log poisoned".to_string(),
                })?
                .clone();
            assert!(!ranges.is_empty());
            assert!(ranges.iter().all(|r| r.starts_with("bytes=")));
            assert!(!ranges.iter().any(|r| {
                parse_range_spec(r, total_len)
                    .map(|(s, e)| s == 0 && (e + 1) >= total_len)
                    .unwrap_or(false)
            }));
            Ok(())
        })();

        let _ = stop_tx.send(());
        let _ = handle.join();
        test_result
    }

    #[cfg(feature = "copc-http")]
    #[test]
    fn http_subset_query_matches_local_subset_query() -> Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = CopcWriterConfig::default();
        cfg.max_points_per_node = 1;
        cfg.spacing = 0.0;
        cfg.max_depth = 4;

        {
            let mut writer = CopcWriter::new(&mut cursor, cfg);
            writer.write_point(&PointRecord { x: -100.0, y: -100.0, z: -100.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: -50.0, y: -50.0, z: -50.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 50.0, y: 50.0, z: 50.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.write_point(&PointRecord { x: 100.0, y: 100.0, z: 100.0, return_number: 1, number_of_returns: 1, ..PointRecord::default() })?;
            writer.finish()?;
        }

        let bytes = cursor.into_inner();
        let (url, _log, stop_tx, handle) = spawn_http_range_server(bytes.clone())?;

        let test_result = (|| -> Result<()> {
            let mut local = CopcReader::new(Cursor::new(bytes))?;
            let mut remote = CopcReader::open_url_cached(&url, 16)?;

            let bbox = CopcBoundingBox {
                min_x: -120.0,
                max_x: 0.0,
                min_y: -120.0,
                max_y: 0.0,
                min_z: -120.0,
                max_z: 0.0,
            };

            let mut local_keys = local.query_data_node_keys(None, Some(bbox), Some(3));
            let mut remote_keys = remote.query_data_node_keys(None, Some(bbox), Some(3));
            local_keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));
            remote_keys.sort_by_key(|k| (k.level, k.x, k.y, k.z));
            assert_eq!(local_keys, remote_keys);

            let mut local_pts = Vec::new();
            for key in &local_keys {
                let _ = local.read_node(*key, &mut local_pts)?;
            }
            let mut remote_pts = Vec::new();
            for key in &remote_keys {
                let _ = remote.read_node(*key, &mut remote_pts)?;
            }

            assert_eq!(point_xyz_signature(&local_pts), point_xyz_signature(&remote_pts));
            Ok(())
        })();

        let _ = stop_tx.send(());
        let _ = handle.join();
        test_result
    }

    #[test]
    fn strict_mode_rejects_evlr_header_offset_interpretation() -> Result<()> {
        let hierarchy = CopcHierarchy {
            entries: vec![
                CopcEntry {
                    key: VoxelKey::ROOT,
                    offset: 0,
                    byte_size: 0,
                    point_count: 0,
                },
                CopcEntry {
                    key: VoxelKey { level: 1, x: 1, y: 0, z: 0 },
                    offset: 256,
                    byte_size: 32,
                    point_count: 4,
                },
            ],
        };
        let data = hierarchy.to_bytes()?;
        let evlr_header_offset = 100u64;
        let data_offset = evlr_header_offset + 60;

        let mut file = vec![0u8; 512];
        for b in &mut file[evlr_header_offset as usize..data_offset as usize] {
            *b = 0xFF;
        }
        file[data_offset as usize..data_offset as usize + data.len()].copy_from_slice(&data);

        let file_end = file.len() as u64;
        let mut cur = Cursor::new(file);
        let err = read_best_hierarchy(
            &mut cur,
            evlr_header_offset,
            data.len() as u64,
            file_end,
            CopcReaderMode::Strict,
        )
        .expect_err("strict mode should reject EVLR-header-offset interpretation");
        assert!(format!("{err}").contains("failed to parse plausible hierarchy")
            || format!("{err}").contains("strict mode requires"));
        Ok(())
    }

    #[test]
    fn strict_info_validation_rejects_nonzero_reserved_bytes() {
        let info = CopcInfo {
            hierarchy_root_offset: 128,
            hierarchy_root_size: 32,
            ..CopcInfo::default()
        };
        let mut bytes = info.to_bytes();
        bytes[72] = 1;
        let err = validate_copc_info_strict(&info, &bytes)
            .expect_err("non-zero reserved bytes should be rejected");
        assert!(format!("{err}").contains("reserved bytes"));
    }

    #[test]
    fn strict_header_validation_accepts_valid_first_vlr_bytes() -> Result<()> {
        let mut file = vec![0u8; 512];
        file[377..381].copy_from_slice(b"copc");
        file[393..395].copy_from_slice(&COPC_INFO_RECORD_ID.to_le_bytes());

        let header = LasHeader {
            version_major: 1,
            version_minor: 4,
            system_identifier: String::new(),
            generating_software: String::new(),
            file_creation_day: 0,
            file_creation_year: 0,
            header_size: 375,
            offset_to_point_data: 589,
            number_of_vlrs: 2,
            point_data_format: PointDataFormat::Pdrf6,
            point_data_record_length: 30,
            global_encoding: Default::default(),
            project_id: [0; 16],
            x_scale: 0.01,
            y_scale: 0.01,
            z_scale: 0.01,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            max_x: 0.0,
            min_x: 0.0,
            max_y: 0.0,
            min_y: 0.0,
            max_z: 0.0,
            min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(1),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0; 15]),
            extra_bytes_count: 0,
        };

        let info_vlr = Vlr {
            key: VlrKey {
                user_id: COPC_USER_ID.to_string(),
                record_id: COPC_INFO_RECORD_ID,
            },
            description: "COPC info".to_string(),
            data: CopcInfo {
                hierarchy_root_offset: 128,
                hierarchy_root_size: 32,
                ..CopcInfo::default()
            }
            .to_bytes(),
            extended: false,
        };

        let laszip_vlr = Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: {
                let mut data = Vec::new();
                data.extend_from_slice(&3u16.to_le_bytes());
                data.extend_from_slice(&0u16.to_le_bytes());
                data.push(3);
                data.push(0);
                data.extend_from_slice(&0u16.to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&50_000u32.to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&10u16.to_le_bytes());
                data.extend_from_slice(&30u16.to_le_bytes());
                data.extend_from_slice(&3u16.to_le_bytes());
                data
            },
            extended: false,
        };

        let mut cur = Cursor::new(file);
        validate_copc_header_strict(
            &mut cur,
            &header,
            &[info_vlr.clone(), laszip_vlr.clone()],
            parse_laszip_vlr(&[info_vlr, laszip_vlr]).as_ref(),
            &Vlr {
                key: VlrKey {
                    user_id: COPC_USER_ID.to_string(),
                    record_id: COPC_INFO_RECORD_ID,
                },
                description: "COPC info".to_string(),
                data: CopcInfo {
                    hierarchy_root_offset: 128,
                    hierarchy_root_size: 32,
                    ..CopcInfo::default()
                }.to_bytes(),
                extended: false,
            },
        )?;
        Ok(())
    }

    #[test]
    fn strict_header_validation_rejects_bad_magic() {
        let mut file = vec![0u8; 512];
        file[377..381].copy_from_slice(b"nope");
        file[393..395].copy_from_slice(&COPC_INFO_RECORD_ID.to_le_bytes());

        let header = LasHeader {
            version_major: 1,
            version_minor: 4,
            system_identifier: String::new(),
            generating_software: String::new(),
            file_creation_day: 0,
            file_creation_year: 0,
            header_size: 375,
            offset_to_point_data: 589,
            number_of_vlrs: 2,
            point_data_format: PointDataFormat::Pdrf6,
            point_data_record_length: 30,
            global_encoding: Default::default(),
            project_id: [0; 16],
            x_scale: 0.01,
            y_scale: 0.01,
            z_scale: 0.01,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            max_x: 0.0,
            min_x: 0.0,
            max_y: 0.0,
            min_y: 0.0,
            max_z: 0.0,
            min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(1),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0; 15]),
            extra_bytes_count: 0,
        };

        let info_vlr = Vlr {
            key: VlrKey {
                user_id: COPC_USER_ID.to_string(),
                record_id: COPC_INFO_RECORD_ID,
            },
            description: "COPC info".to_string(),
            data: CopcInfo {
                hierarchy_root_offset: 128,
                hierarchy_root_size: 32,
                ..CopcInfo::default()
            }
            .to_bytes(),
            extended: false,
        };

        let laszip_vlr = Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: {
                let mut data = Vec::new();
                data.extend_from_slice(&3u16.to_le_bytes());
                data.extend_from_slice(&0u16.to_le_bytes());
                data.push(3);
                data.push(0);
                data.extend_from_slice(&0u16.to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&50_000u32.to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&10u16.to_le_bytes());
                data.extend_from_slice(&30u16.to_le_bytes());
                data.extend_from_slice(&3u16.to_le_bytes());
                data
            },
            extended: false,
        };

        let mut cur = Cursor::new(file);
        let err = validate_copc_header_strict(
            &mut cur,
            &header,
            &[info_vlr.clone(), laszip_vlr.clone()],
            parse_laszip_vlr(&[info_vlr, laszip_vlr]).as_ref(),
            &Vlr {
                key: VlrKey {
                    user_id: COPC_USER_ID.to_string(),
                    record_id: COPC_INFO_RECORD_ID,
                },
                description: "COPC info".to_string(),
                data: CopcInfo {
                    hierarchy_root_offset: 128,
                    hierarchy_root_size: 32,
                    ..CopcInfo::default()
                }.to_bytes(),
                extended: false,
            },
        )
        .expect_err("bad magic should be rejected");
        assert!(format!("{err}").contains("copc.magic"));
    }

    #[test]
    fn strict_header_validation_rejects_non_las14() {
        let mut file = vec![0u8; 512];
        file[377..381].copy_from_slice(b"copc");
        file[393..395].copy_from_slice(&COPC_INFO_RECORD_ID.to_le_bytes());

        let header = LasHeader {
            version_major: 1,
            version_minor: 3,
            system_identifier: String::new(),
            generating_software: String::new(),
            file_creation_day: 0,
            file_creation_year: 0,
            header_size: 235,
            offset_to_point_data: 589,
            number_of_vlrs: 2,
            point_data_format: PointDataFormat::Pdrf6,
            point_data_record_length: 30,
            global_encoding: Default::default(),
            project_id: [0; 16],
            x_scale: 0.01,
            y_scale: 0.01,
            z_scale: 0.01,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            max_x: 0.0,
            min_x: 0.0,
            max_y: 0.0,
            min_y: 0.0,
            max_z: 0.0,
            min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(1),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0; 15]),
            extra_bytes_count: 0,
        };

        let info_vlr = Vlr {
            key: VlrKey {
                user_id: COPC_USER_ID.to_string(),
                record_id: COPC_INFO_RECORD_ID,
            },
            description: "COPC info".to_string(),
            data: CopcInfo {
                hierarchy_root_offset: 128,
                hierarchy_root_size: 32,
                ..CopcInfo::default()
            }
            .to_bytes(),
            extended: false,
        };

        let laszip_vlr = Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: {
                let mut data = Vec::new();
                data.extend_from_slice(&3u16.to_le_bytes());
                data.extend_from_slice(&0u16.to_le_bytes());
                data.push(3);
                data.push(0);
                data.extend_from_slice(&0u16.to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&50_000u32.to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&10u16.to_le_bytes());
                data.extend_from_slice(&30u16.to_le_bytes());
                data.extend_from_slice(&3u16.to_le_bytes());
                data
            },
            extended: false,
        };

        let mut cur = Cursor::new(file);
        let err = validate_copc_header_strict(
            &mut cur,
            &header,
            &[info_vlr.clone(), laszip_vlr.clone()],
            parse_laszip_vlr(&[info_vlr, laszip_vlr]).as_ref(),
            &Vlr {
                key: VlrKey {
                    user_id: COPC_USER_ID.to_string(),
                    record_id: COPC_INFO_RECORD_ID,
                },
                description: "COPC info".to_string(),
                data: CopcInfo {
                    hierarchy_root_offset: 128,
                    hierarchy_root_size: 32,
                    ..CopcInfo::default()
                }.to_bytes(),
                extended: false,
            },
        )
        .expect_err("non-LAS14 file should be rejected");
        assert!(format!("{err}").contains("strict mode requires LAS 1.4"));
    }

    #[test]
    fn strict_header_validation_rejects_wrong_info_record_id_bytes() {
        let mut file = vec![0u8; 512];
        file[377..381].copy_from_slice(b"copc");
        file[393..395].copy_from_slice(&999u16.to_le_bytes());

        let header = LasHeader {
            version_major: 1,
            version_minor: 4,
            system_identifier: String::new(),
            generating_software: String::new(),
            file_creation_day: 0,
            file_creation_year: 0,
            header_size: 375,
            offset_to_point_data: 589,
            number_of_vlrs: 2,
            point_data_format: PointDataFormat::Pdrf6,
            point_data_record_length: 30,
            global_encoding: Default::default(),
            project_id: [0; 16],
            x_scale: 0.01,
            y_scale: 0.01,
            z_scale: 0.01,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            max_x: 0.0,
            min_x: 0.0,
            max_y: 0.0,
            min_y: 0.0,
            max_z: 0.0,
            min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(1),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0; 15]),
            extra_bytes_count: 0,
        };

        let info_vlr = Vlr {
            key: VlrKey {
                user_id: COPC_USER_ID.to_string(),
                record_id: COPC_INFO_RECORD_ID,
            },
            description: "COPC info".to_string(),
            data: CopcInfo {
                hierarchy_root_offset: 128,
                hierarchy_root_size: 32,
                ..CopcInfo::default()
            }
            .to_bytes(),
            extended: false,
        };

        let laszip_vlr = Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: {
                let mut data = Vec::new();
                data.extend_from_slice(&3u16.to_le_bytes());
                data.extend_from_slice(&0u16.to_le_bytes());
                data.push(3);
                data.push(0);
                data.extend_from_slice(&0u16.to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&50_000u32.to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&10u16.to_le_bytes());
                data.extend_from_slice(&30u16.to_le_bytes());
                data.extend_from_slice(&3u16.to_le_bytes());
                data
            },
            extended: false,
        };

        let mut cur = Cursor::new(file);
        let err = validate_copc_header_strict(
            &mut cur,
            &header,
            &[info_vlr.clone(), laszip_vlr.clone()],
            parse_laszip_vlr(&[info_vlr, laszip_vlr]).as_ref(),
            &Vlr {
                key: VlrKey {
                    user_id: COPC_USER_ID.to_string(),
                    record_id: COPC_INFO_RECORD_ID,
                },
                description: "COPC info".to_string(),
                data: CopcInfo {
                    hierarchy_root_offset: 128,
                    hierarchy_root_size: 32,
                    ..CopcInfo::default()
                }.to_bytes(),
                extended: false,
            },
        )
        .expect_err("wrong COPC info record-id bytes should be rejected");
        assert!(format!("{err}").contains("copc.info_record_id"));
    }

    #[test]
    fn strict_header_validation_rejects_missing_copc_info_vlr() {
        let mut file = vec![0u8; 512];
        file[377..381].copy_from_slice(b"copc");
        file[393..395].copy_from_slice(&COPC_INFO_RECORD_ID.to_le_bytes());

        let header = LasHeader {
            version_major: 1,
            version_minor: 4,
            system_identifier: String::new(),
            generating_software: String::new(),
            file_creation_day: 0,
            file_creation_year: 0,
            header_size: 375,
            offset_to_point_data: 589,
            number_of_vlrs: 1,
            point_data_format: PointDataFormat::Pdrf6,
            point_data_record_length: 30,
            global_encoding: Default::default(),
            project_id: [0; 16],
            x_scale: 0.01,
            y_scale: 0.01,
            z_scale: 0.01,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            max_x: 0.0,
            min_x: 0.0,
            max_y: 0.0,
            min_y: 0.0,
            max_z: 0.0,
            min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(1),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0; 15]),
            extra_bytes_count: 0,
        };

        let laszip_vlr = Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: {
                let mut data = Vec::new();
                data.extend_from_slice(&3u16.to_le_bytes());
                data.extend_from_slice(&0u16.to_le_bytes());
                data.push(3);
                data.push(0);
                data.extend_from_slice(&0u16.to_le_bytes());
                data.extend_from_slice(&0u32.to_le_bytes());
                data.extend_from_slice(&50_000u32.to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&(-1i64).to_le_bytes());
                data.extend_from_slice(&1u16.to_le_bytes());
                data.extend_from_slice(&10u16.to_le_bytes());
                data.extend_from_slice(&30u16.to_le_bytes());
                data.extend_from_slice(&3u16.to_le_bytes());
                data
            },
            extended: false,
        };

        let mut cur = Cursor::new(file);
        let err = validate_copc_header_strict(
            &mut cur,
            &header,
            &[laszip_vlr.clone()],
            parse_laszip_vlr(&[laszip_vlr]).as_ref(),
            &Vlr {
                key: VlrKey {
                    user_id: COPC_USER_ID.to_string(),
                    record_id: COPC_INFO_RECORD_ID,
                },
                description: "COPC info".to_string(),
                data: CopcInfo {
                    hierarchy_root_offset: 128,
                    hierarchy_root_size: 32,
                    ..CopcInfo::default()
                }.to_bytes(),
                extended: false,
            },
        )
        .expect_err("missing COPC info VLR should be rejected");
        assert!(format!("{err}").contains("first VLR to be COPC info"));
    }
}
