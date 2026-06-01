//! Minimal HDF adapter interfaces for early `wblidar` <-> `wbhdf` integration.
//!
//! This module intentionally exposes a narrow API surface for bounded dataset-window
//! reads while the broader Tier 1 ingestion paths are still being implemented.

use std::fs;
use std::path::Path;

use wbhdf::btree::{read_chunk_payload_in_file, read_first_chunked_storage_leaf_record_in_file};
use wbhdf::dataset::apply_fill_value_mapping_f32;
use wbhdf::datatypes::Endianness;
use wbhdf::filters::decompress_zlib;
use wbhdf::object_header::{discover_v1_object_headers, ObjectHeaderV1};

/// Canonical GEDI dataset-path marker used by the initial Tier 1 mapping slice.
pub const GEDI_L2B_CANOPY_STYLE_DATASET_PATH: &str = "/BEAM0000/elev_lowestmode";
/// Current known byte offset for the validated GEDI fixture's contiguous payload.
pub const GEDI_L2B_CANOPY_STYLE_KNOWN_BYTE_OFFSET: u64 = 1_012_683;
/// Candidate ATL08 beam groups for dynamic h_canopy dataset-path discovery.
pub const ICESAT2_ATL08_BEAM_GROUP_CANDIDATES: [&str; 6] =
    ["gt1l", "gt1r", "gt2l", "gt2r", "gt3l", "gt3r"];
/// Canonical ATL08 canopy subpath appended to discovered beam groups.
pub const ICESAT2_ATL08_CANOPY_SUBPATH: &str = "/land_segments/canopy/h_canopy";
/// Default nodata sentinel used for ATL08 canopy-style fill mapping in adapter reads.
pub const ICESAT2_ATL08_CANOPY_NODATA_VALUE: f32 = -9999.0;
/// Upper bound for compressed ATL08 chunk bytes accepted by bounded decode flow.
pub const ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES: usize = 16 * 1024 * 1024;
/// Upper bound for decompressed ATL08 chunk bytes accepted by bounded decode flow.
pub const ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES: usize = 64 * 1024 * 1024;

/// Adapter-level result type using `wbhdf` error semantics directly.
pub type HdfAdapterResult<T> = std::result::Result<T, wbhdf::WbhdfError>;

/// A bounded i16 window-read request against a canonical HDF dataset path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HdfI16WindowRequest<'a> {
    /// Source HDF file path.
    pub file_path: &'a Path,
    /// Canonical dataset path (for example `/GridName/DataFieldName`).
    pub dataset_path: &'a str,
    /// Start offset, in values, within the decoded dataset stream.
    pub start_value: usize,
    /// Maximum number of i16 values to decode.
    pub max_values: usize,
}

/// Minimal provider trait for bounded HDF i16 window reads.
pub trait HdfDatasetProvider {
    /// Reads a bounded i16 value window from the requested dataset path.
    fn read_i16_window(&self, request: &HdfI16WindowRequest<'_>) -> HdfAdapterResult<Vec<i16>>;

    /// Reads a bounded `f32` window from the initial GEDI canopy-style Tier 1 mapping path.
    fn read_gedi_l2b_canopy_style_f32_window(
        &self,
        file_path: &Path,
        start_value: usize,
        max_values: usize,
    ) -> HdfAdapterResult<Vec<f32>>;

    /// Reads a bounded `f32` window from the initial ICESat-2 ATL08 canopy-height path.
    fn read_icesat2_atl08_h_canopy_f32_window(
        &self,
        file_path: &Path,
        start_value: usize,
        max_values: usize,
    ) -> HdfAdapterResult<Vec<f32>>;
}

/// Default provider backed by `wbhdf` decode entrypoints.
#[derive(Debug, Default, Clone, Copy)]
pub struct WbhdfDatasetProvider;

impl HdfDatasetProvider for WbhdfDatasetProvider {
    fn read_i16_window(&self, request: &HdfI16WindowRequest<'_>) -> HdfAdapterResult<Vec<i16>> {
        wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file(
            request.file_path,
            request.dataset_path,
            request.start_value,
            request.max_values,
        )
    }

    fn read_gedi_l2b_canopy_style_f32_window(
        &self,
        file_path: &Path,
        start_value: usize,
        max_values: usize,
    ) -> HdfAdapterResult<Vec<f32>> {
        read_gedi_l2b_canopy_style_f32_window_in_file(file_path, start_value, max_values)
    }

    fn read_icesat2_atl08_h_canopy_f32_window(
        &self,
        file_path: &Path,
        start_value: usize,
        max_values: usize,
    ) -> HdfAdapterResult<Vec<f32>> {
        read_icesat2_atl08_h_canopy_f32_window_in_file(file_path, start_value, max_values)
    }
}

/// Reads a bounded `f32` window from the current GEDI canopy-style Tier 1 mapping path.
///
/// Current scope note:
/// - This mapping uses the fixture-validated contiguous payload offset for
///   `/BEAM0000/elev_lowestmode` while broader object-header-driven offset resolution
///   is still in progress.
pub fn read_gedi_l2b_canopy_style_f32_window_in_file(
    file_path: &Path,
    start_value: usize,
    max_values: usize,
) -> HdfAdapterResult<Vec<f32>> {
    if max_values == 0 {
        return Err(wbhdf::WbhdfError::InvalidInput(
            "GEDI canopy-style window read requires max_values >= 1".to_string(),
        ));
    }

    wbhdf::dataset::resolve_dataset_in_file(file_path, GEDI_L2B_CANOPY_STYLE_DATASET_PATH)?;

    let start_byte_offset = start_value
        .checked_mul(4)
        .and_then(|offset| u64::try_from(offset).ok())
        .and_then(|offset| GEDI_L2B_CANOPY_STYLE_KNOWN_BYTE_OFFSET.checked_add(offset))
        .ok_or_else(|| {
            wbhdf::WbhdfError::InvalidInput(format!(
                "GEDI canopy-style window start overflow: start_value={start_value}"
            ))
        })?;

    wbhdf::dataset::read_contiguous_f32_window_in_file(
        file_path,
        start_byte_offset,
        max_values,
        Endianness::Little,
    )
}

/// Resolves the first discoverable ATL08 `h_canopy` dataset path by enumerating known beam groups.
pub fn resolve_icesat2_atl08_h_canopy_path_in_file(file_path: &Path) -> HdfAdapterResult<String> {
    for beam in ICESAT2_ATL08_BEAM_GROUP_CANDIDATES {
        let candidate = format!("/{beam}{ICESAT2_ATL08_CANOPY_SUBPATH}");
        if wbhdf::dataset::resolve_dataset_in_file(file_path, &candidate).is_ok() {
            return Ok(candidate);
        }
    }

    Err(wbhdf::WbhdfError::DatasetPathNotFound(format!(
        "ATL08 h_canopy path not found under beam candidates {:?} with subpath '{}'",
        ICESAT2_ATL08_BEAM_GROUP_CANDIDATES,
        ICESAT2_ATL08_CANOPY_SUBPATH
    )))
}

/// Reads a bounded `f32` window from the ATL08 `h_canopy` first validated Tier 1 path.
///
/// Current scope note:
/// - Object-header selection uses bounded dynamic v1 discovery plus beam-aware ranking
///   derived from the resolved ATL08 dataset path.
pub fn read_icesat2_atl08_h_canopy_f32_window_in_file(
    file_path: &Path,
    start_value: usize,
    max_values: usize,
) -> HdfAdapterResult<Vec<f32>> {
    if max_values == 0 {
        return Err(wbhdf::WbhdfError::InvalidInput(
            "ATL08 h_canopy window read requires max_values >= 1".to_string(),
        ));
    }

    let parsed = resolve_icesat2_atl08_h_canopy_object_header_in_file(file_path)?;
    let layout = parsed
        .chunked_layouts
        .first()
        .ok_or_else(|| {
            wbhdf::WbhdfError::UnsupportedLayout(
                "ATL08 h_canopy decode requires a v1 chunked layout message".to_string(),
            )
        })?;
    let first_record = read_first_chunked_storage_leaf_record_in_file(
        file_path,
        layout.index_address,
        layout.num_dimensions as usize,
    )?;
    let compressed_size = first_record.chunk_size as usize;
    if compressed_size > ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES {
        return Err(wbhdf::WbhdfError::UnsupportedLayout(format!(
            "ATL08 h_canopy compressed chunk exceeds bounded-memory limit: chunk_size={}, max={} bytes",
            compressed_size, ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES
        )));
    }

    let compressed = read_chunk_payload_in_file(file_path, first_record.chunk_address, first_record.chunk_size)?;
    let decompressed = decompress_zlib(&compressed)?;
    if decompressed.len() > ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES {
        return Err(wbhdf::WbhdfError::UnsupportedLayout(format!(
            "ATL08 h_canopy decompressed chunk exceeds bounded-memory limit: chunk_size={}, max={} bytes",
            decompressed.len(),
            ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES
        )));
    }
    let values = wbhdf::datatypes::decode_f32_slice(&decompressed, Endianness::Little)
        .map_err(|err| wbhdf::WbhdfError::InvalidInput(format!("ATL08 h_canopy f32 decode failed: {err}")))?;

    let fill = parsed
        .fill_values
        .first()
        .ok_or_else(|| {
            wbhdf::WbhdfError::UnsupportedLayout(
                "ATL08 h_canopy decode requires a fill-value message".to_string(),
            )
        })?
        .value_bytes
        .clone();
    if fill.len() != 4 {
        return Err(wbhdf::WbhdfError::UnsupportedLayout(format!(
            "ATL08 h_canopy fill-value size unsupported: expected 4 bytes, found {}",
            fill.len()
        )));
    }
    let fill_value = wbhdf::datatypes::decode_f32(
        fill.try_into().map_err(|_| {
            wbhdf::WbhdfError::UnsupportedLayout(
                "ATL08 h_canopy fill-value bytes could not be converted to [u8; 4]".to_string(),
            )
        })?,
        Endianness::Little,
    );

    let mapped = apply_fill_value_mapping_f32(&values, Some(fill_value), ICESAT2_ATL08_CANOPY_NODATA_VALUE);
    if start_value >= mapped.values.len() {
        return Err(wbhdf::WbhdfError::InvalidInput(format!(
            "ATL08 h_canopy window start index {} is out of bounds for {} decoded values",
            start_value,
            mapped.values.len()
        )));
    }

    let decode_count = usize::min(max_values, mapped.values.len() - start_value);
    Ok(mapped.values[start_value..start_value + decode_count].to_vec())
}

/// Resolves a likely ATL08 `h_canopy` v1 object header by bounded discovery + ranking.
pub fn resolve_icesat2_atl08_h_canopy_object_header_in_file(
    file_path: &Path,
) -> HdfAdapterResult<ObjectHeaderV1> {
    let resolved_path = resolve_icesat2_atl08_h_canopy_path_in_file(file_path)?;
    let beam = extract_icesat2_beam_from_dataset_path(&resolved_path).ok_or_else(|| {
        wbhdf::WbhdfError::UnsupportedLayout(format!(
            "ATL08 canopy path '{}' did not contain a valid beam group",
            resolved_path
        ))
    })?;

    let bytes = fs::read(file_path)?;
    let candidates = discover_v1_object_headers(&bytes, 4096)?;
    let beam_marker_offsets = collect_ascii_marker_offsets(&bytes, beam);
    let path_marker_offsets = collect_ascii_marker_offsets(&bytes, &resolved_path);

    let mut ranked = candidates
        .into_iter()
        .filter_map(|header| {
            score_atl08_h_canopy_header(&header, &beam_marker_offsets, &path_marker_offsets)
                .map(|(score, distance)| (score, distance, header))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|(score_a, dist_a, header_a), (score_b, dist_b, header_b)| {
        score_b
            .cmp(score_a)
            .then(dist_a.cmp(dist_b))
            .then(header_a.offset.cmp(&header_b.offset))
    });

    ranked
        .into_iter()
        .next()
        .map(|(_, _, header)| header)
        .ok_or_else(|| {
            wbhdf::WbhdfError::UnsupportedLayout(
                "ATL08 h_canopy object-header discovery found no suitable v1 chunked canopy candidate"
                    .to_string(),
            )
        })
}

fn score_atl08_h_canopy_header(
    header: &ObjectHeaderV1,
    beam_marker_offsets: &[usize],
    path_marker_offsets: &[usize],
) -> Option<(u32, usize)> {
    let has_chunked = header.chunked_layouts.iter().any(|layout| {
        layout.layout_class == 2
            && layout.num_dimensions >= 2
            && !layout.chunk_dimensions.is_empty()
            && layout.chunk_dimensions.last() == Some(&4)
    });
    if !has_chunked {
        return None;
    }

    let mut score = 10_u32;
    if header.datatypes.iter().any(|datatype| datatype.size == 4) {
        score += 4;
    }
    if header
        .filter_pipelines
        .iter()
        .any(|pipeline| pipeline.filters.iter().any(|filter| filter.id == 1))
    {
        score += 3;
    }
    if header.fill_values.iter().any(|fill| {
        fill.value_size == 4 && fill.value_bytes == [0xff, 0xff, 0x7f, 0x7f]
    }) {
        score += 3;
    }
    if header
        .dataspaces
        .iter()
        .any(|dataspace| dataspace.rank >= 1 && !dataspace.dimensions.is_empty())
    {
        score += 2;
    }

    let nearest_path = nearest_marker_distance(header.offset, path_marker_offsets);
    let nearest_beam = nearest_marker_distance(header.offset, beam_marker_offsets);
    let nearest = usize::min(nearest_path, nearest_beam);
    if nearest_path <= 65_536 {
        score += 4;
    }
    if nearest_beam <= 131_072 {
        score += 2;
    }

    Some((score, nearest))
}

fn extract_icesat2_beam_from_dataset_path(dataset_path: &str) -> Option<&str> {
    let mut parts = dataset_path.split('/').filter(|part| !part.is_empty());
    let beam = parts.next()?;
    if beam.len() == 4 && beam.starts_with("gt") {
        Some(beam)
    } else {
        None
    }
}

fn collect_ascii_marker_offsets(bytes: &[u8], marker: &str) -> Vec<usize> {
    if marker.is_empty() || marker.len() > bytes.len() {
        return Vec::new();
    }

    bytes
        .windows(marker.len())
        .enumerate()
        .filter_map(|(idx, window)| (window == marker.as_bytes()).then_some(idx))
        .collect()
}

fn nearest_marker_distance(offset: usize, marker_offsets: &[usize]) -> usize {
    if marker_offsets.is_empty() {
        return usize::MAX / 2;
    }

    marker_offsets
        .iter()
        .map(|marker| offset.abs_diff(*marker))
        .min()
        .unwrap_or(usize::MAX / 2)
}

#[cfg(test)]
mod tests {
    use super::{
        read_gedi_l2b_canopy_style_f32_window_in_file,
        read_icesat2_atl08_h_canopy_f32_window_in_file,
        resolve_icesat2_atl08_h_canopy_object_header_in_file,
        resolve_icesat2_atl08_h_canopy_path_in_file,
        HdfDatasetProvider,
        HdfI16WindowRequest,
        ICESAT2_ATL08_CANOPY_NODATA_VALUE,
        WbhdfDatasetProvider,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn provider_delegates_to_wbhdf_with_deterministic_errors() {
        let provider = WbhdfDatasetProvider;
        let request = HdfI16WindowRequest {
            file_path: Path::new("/tmp/does-not-exist.hdf"),
            dataset_path: "/Grid/Field",
            start_value: 0,
            max_values: 8,
        };

        let err = provider
            .read_i16_window(&request)
            .expect_err("missing file should produce deterministic read error");
        assert!(
            format!("{err}").contains("No such file") || format!("{err}").contains("os error"),
            "error should surface file-read failure from wbhdf"
        );
    }

    #[test]
    fn gedi_canopy_style_window_matches_reference_when_fixture_available() {
        let Some(root) = std::env::var_os("WBHDF_FIXTURE_DIR").map(PathBuf::from) else {
            return;
        };
        let path = root.join("GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5");
        if !path.is_file() {
            return;
        }

        let actual = read_gedi_l2b_canopy_style_f32_window_in_file(&path, 0, 4)
            .expect("GEDI canopy-style mapping read should succeed");
        let expected = [7373.83593750, 7373.16357422, 7373.08935547, 7373.01513672];

        assert_eq!(actual.len(), expected.len());
        for (idx, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            let diff = (a - e).abs();
            assert!(
                diff <= 1e-5,
                "GEDI canopy-style mismatch at index {idx}: actual={a}, expected={e}, abs_diff={diff}"
            );
        }
    }

    #[test]
    fn atl08_h_canopy_path_resolution_reports_missing_path() {
        let file_path = std::env::temp_dir().join(format!(
            "wblidar-atl08-missing-path-{}-{}.h5",
            std::process::id(),
            std::thread::current().name().unwrap_or("thread")
        ));
        fs::write(&file_path, b"not-an-atl08-file").expect("temp file should be writable");

        let err = resolve_icesat2_atl08_h_canopy_path_in_file(&file_path)
            .expect_err("missing ATL08 path should return deterministic error");
        let _ = fs::remove_file(&file_path);

        match err {
            wbhdf::WbhdfError::DatasetPathNotFound(msg) => {
                assert!(
                    msg.contains("ATL08 h_canopy path not found"),
                    "missing-path error should include ATL08 context"
                );
            }
            other => panic!("expected DatasetPathNotFound, found {other}"),
        }
    }

    #[test]
    fn atl08_h_canopy_first_chunk_window_decodes_when_fixture_available() {
        let Some(root) = std::env::var_os("WBHDF_FIXTURE_DIR").map(PathBuf::from) else {
            return;
        };
        let path = root.join("ATL08_20181120185605_08120102_007_01.h5");
        if !path.is_file() {
            return;
        }

        let resolved = resolve_icesat2_atl08_h_canopy_path_in_file(&path)
            .expect("ATL08 canopy path should resolve under known beam candidates");
        assert!(
            resolved.ends_with("/land_segments/canopy/h_canopy"),
            "resolved ATL08 canopy path should point to h_canopy dataset"
        );

        let header = resolve_icesat2_atl08_h_canopy_object_header_in_file(&path)
            .expect("ATL08 canopy object header should be discoverable");
        assert_eq!(
            header.offset, 328_097,
            "beam-aware ATL08 header discovery should select known canopy header offset"
        );
        assert!(
            !header.chunked_layouts.is_empty(),
            "discovered ATL08 canopy object header should include chunked layout"
        );

        let actual = read_icesat2_atl08_h_canopy_f32_window_in_file(&path, 0, 10_000)
            .expect("ATL08 canopy-style mapping read should succeed");
        assert_eq!(actual.len(), 10_000);

        let nodata_bits = ICESAT2_ATL08_CANOPY_NODATA_VALUE.to_bits();
        let nodata_count = actual
            .iter()
            .filter(|value| value.to_bits() == nodata_bits)
            .count();
        assert_eq!(nodata_count, 6_360);
        assert_eq!(actual.len() - nodata_count, 3_640);
    }
}
