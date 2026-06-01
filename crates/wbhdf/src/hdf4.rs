use crate::error::{WbhdfError, WbhdfResult};
use crate::filters::{decompress_gzip, decompress_zlib};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const HDF4_MAGIC: [u8; 4] = [0x0E, 0x03, 0x13, 0x01];
const MAX_COMPRESSED_PROBE_BYTES: usize = 8 * 1024 * 1024;
const MAX_COMPRESSED_WINDOW_DECODE_BYTES: usize = 32 * 1024 * 1024;
const DEFAULT_HDF4_SDS_I16_WINDOW_VALUES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf4DataFieldSummary {
    pub name: String,
    pub data_type: Option<String>,
    pub dim_list: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4GridSummary {
    pub name: String,
    pub dim_sizes: BTreeMap<String, usize>,
    pub projection: Option<String>,
    pub proj_params: Vec<f64>,
    pub sphere_code: Option<i32>,
    pub upper_left_mtrs: Option<(f64, f64)>,
    pub lower_right_mtrs: Option<(f64, f64)>,
    pub data_fields: Vec<Hdf4DataFieldSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4ResolvedFieldSummary {
    pub grid_name: String,
    pub field_name: String,
    pub data_type: Option<String>,
    pub dim_list: Vec<String>,
    pub shape: Vec<usize>,
    pub projection: Option<String>,
    pub proj_params: Vec<f64>,
    pub sphere_code: Option<i32>,
    pub upper_left_mtrs: Option<(f64, f64)>,
    pub lower_right_mtrs: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4SdsDecodeAttempt {
    pub dataset_path: String,
    pub resolved_field: Hdf4ResolvedFieldSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4GridGeometry {
    pub rows: usize,
    pub cols: usize,
    pub upper_left_mtrs: (f64, f64),
    pub lower_right_mtrs: (f64, f64),
    pub pixel_size_x: f64,
    pub pixel_size_y: f64,
    pub geotransform: [f64; 6],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf4DataDescriptor {
    pub tag: u16,
    pub reference: u16,
    pub offset: u32,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4SdsDecodeReadiness {
    pub dataset_path: String,
    pub resolved_field: Hdf4ResolvedFieldSummary,
    pub geometry: Option<Hdf4GridGeometry>,
    pub payload_candidates: Vec<Hdf4DataDescriptor>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf4RankedPayloadCandidate {
    pub descriptor: Hdf4DataDescriptor,
    pub expected_length: usize,
    pub length_delta: usize,
    pub signature_hint: String,
    pub preview_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf4HeuristicDescriptorMapping {
    pub dataset_path: String,
    pub expected_length: usize,
    pub selected: Option<Hdf4RankedPayloadCandidate>,
    pub rationale: String,
    pub considered_count: usize,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdf4SdsPayloadProbe {
    pub dataset_path: String,
    pub status: String,
    pub rationale: String,
    pub candidate: Option<Hdf4RankedPayloadCandidate>,
    pub little_endian_preview: Vec<i16>,
    pub big_endian_preview: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hdf4EosMetadataSummary {
    pub struct_metadata_markers: usize,
    pub grid_names: Vec<String>,
    pub data_field_names: Vec<String>,
    pub grids: Vec<Hdf4GridSummary>,
}

pub fn probe_hdf4_eos_metadata_in_file(path: &Path) -> WbhdfResult<Hdf4EosMetadataSummary> {
    let bytes = fs::read(path)?;
    probe_hdf4_eos_metadata(&bytes)
}

pub fn probe_hdf4_eos_metadata(bytes: &[u8]) -> WbhdfResult<Hdf4EosMetadataSummary> {
    if bytes.len() < HDF4_MAGIC.len() {
        return Err(WbhdfError::InvalidInput(
            "HDF4 probe requires at least 4 bytes".to_string(),
        ));
    }

    if bytes[..4] != HDF4_MAGIC {
        return Err(WbhdfError::InvalidInput(
            "HDF4 probe requires HDF4 magic signature".to_string(),
        ));
    }

    let text = String::from_utf8_lossy(bytes);
    let struct_metadata_markers = text.matches("StructMetadata.0").count();

    let mut grid_names = Vec::<String>::new();
    let mut data_field_names = Vec::<String>::new();
    let mut grids = Vec::<Hdf4GridSummary>::new();
    let mut current_grid_name: Option<String> = None;
    let mut current_field_name: Option<String> = None;

    for line in text.lines() {
        if let Some(name) = extract_quoted_value(line, "GridName=") {
            push_unique(&mut grid_names, name.clone());
            ensure_grid(&mut grids, &name);
            current_grid_name = Some(name);
            current_field_name = None;
        }

        if let Some((dim_name, dim_size)) = extract_dim_size(line) {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    grid.dim_sizes.entry(dim_name).or_insert(dim_size);
                }
            }
        }

        if let Some(projection) = extract_unquoted_value(line, "Projection=") {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    grid.projection.get_or_insert(projection);
                }
            }
        }

        if let Some(proj_params) = extract_f64_list(line, "ProjParams=(") {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    if grid.proj_params.is_empty() {
                        grid.proj_params = proj_params;
                    }
                }
            }
        }

        if let Some(sphere_code) = extract_i32_value(line, "SphereCode=") {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    grid.sphere_code.get_or_insert(sphere_code);
                }
            }
        }

        if let Some(upper_left_mtrs) = extract_f64_pair(line, "UpperLeftPointMtrs=(") {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    grid.upper_left_mtrs.get_or_insert(upper_left_mtrs);
                }
            }
        }

        if let Some(lower_right_mtrs) = extract_f64_pair(line, "LowerRightMtrs=(") {
            if let Some(grid_name) = current_grid_name.as_deref() {
                if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
                    grid.lower_right_mtrs.get_or_insert(lower_right_mtrs);
                }
            }
        }

        if let Some(name) = extract_quoted_value(line, "DataFieldName=") {
            push_unique(&mut data_field_names, name.clone());
            if let Some(grid_name) = current_grid_name.as_deref() {
                ensure_data_field(&mut grids, grid_name, &name);
                current_field_name = Some(name);
            }
        }

        if let Some(data_type) = extract_unquoted_value(line, "DataType=") {
            if let (Some(grid_name), Some(field_name)) =
                (current_grid_name.as_deref(), current_field_name.as_deref())
            {
                if let Some(field) = find_data_field_mut(&mut grids, grid_name, field_name) {
                    field.data_type.get_or_insert(data_type);
                }
            }
        }

        if let Some(dim_list) = extract_dim_list(line) {
            if let (Some(grid_name), Some(field_name)) =
                (current_grid_name.as_deref(), current_field_name.as_deref())
            {
                if let Some(field) = find_data_field_mut(&mut grids, grid_name, field_name) {
                    if field.dim_list.is_empty() {
                        field.dim_list = dim_list;
                    }
                }
            }
        }
    }

    Ok(Hdf4EosMetadataSummary {
        struct_metadata_markers,
        grid_names,
        data_field_names,
        grids,
    })
}

pub fn resolve_hdf4_grid_field(
    summary: &Hdf4EosMetadataSummary,
    grid_name: &str,
    field_name: &str,
) -> WbhdfResult<Hdf4ResolvedFieldSummary> {
    let grid = summary
        .grids
        .iter()
        .find(|grid| grid.name == grid_name)
        .ok_or_else(|| WbhdfError::DatasetPathNotFound(format!("{grid_name}/{field_name}")))?;

    let field = grid
        .data_fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| WbhdfError::DatasetPathNotFound(format!("{grid_name}/{field_name}")))?;

    let mut shape = Vec::with_capacity(field.dim_list.len());
    for dim in &field.dim_list {
        let size = grid.dim_sizes.get(dim).copied().ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "missing dimension size for dim '{dim}' in grid '{grid_name}'"
            ))
        })?;
        shape.push(size);
    }

    Ok(Hdf4ResolvedFieldSummary {
        grid_name: grid.name.clone(),
        field_name: field.name.clone(),
        data_type: field.data_type.clone(),
        dim_list: field.dim_list.clone(),
        shape,
        projection: grid.projection.clone(),
        proj_params: grid.proj_params.clone(),
        sphere_code: grid.sphere_code,
        upper_left_mtrs: grid.upper_left_mtrs,
        lower_right_mtrs: grid.lower_right_mtrs,
    })
}

pub fn resolve_hdf4_dataset_path(
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Hdf4ResolvedFieldSummary> {
    if !dataset_path.starts_with('/') {
        return Err(WbhdfError::InvalidInput(
            "HDF4 dataset path must start with '/'".to_string(),
        ));
    }

    let mut parts = dataset_path.split('/').filter(|part| !part.is_empty());
    let grid_name = parts.next().ok_or_else(|| {
        WbhdfError::InvalidInput("HDF4 dataset path must include grid name".to_string())
    })?;
    let field_name = parts.next().ok_or_else(|| {
        WbhdfError::InvalidInput("HDF4 dataset path must include data field name".to_string())
    })?;

    if parts.next().is_some() {
        return Err(WbhdfError::InvalidInput(
            "HDF4 dataset path currently supports exactly two segments: /GridName/DataFieldName"
                .to_string(),
        ));
    }

    resolve_hdf4_grid_field(summary, grid_name, field_name)
}

pub fn enumerate_hdf4_dataset_paths(summary: &Hdf4EosMetadataSummary) -> Vec<String> {
    let mut paths = Vec::<String>::new();
    for grid in &summary.grids {
        for field in &grid.data_fields {
            paths.push(format!("/{}/{}", grid.name, field.name));
        }
    }
    paths.sort();
    paths
}

pub fn prepare_hdf4_sds_decode_attempt(
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Hdf4SdsDecodeAttempt> {
    let resolved_field = resolve_hdf4_dataset_path(summary, dataset_path)?;
    Ok(Hdf4SdsDecodeAttempt {
        dataset_path: dataset_path.to_string(),
        resolved_field,
    })
}

pub fn derive_hdf4_grid_geometry(
    resolved: &Hdf4ResolvedFieldSummary,
) -> WbhdfResult<Hdf4GridGeometry> {
    if resolved.shape.len() != 2 {
        return Err(WbhdfError::InvalidInput(format!(
            "HDF4 geometry derivation requires 2D shape, found rank {} for '{}'",
            resolved.shape.len(),
            resolved.field_name
        )));
    }

    let rows = resolved.shape[0];
    let cols = resolved.shape[1];
    if rows == 0 || cols == 0 {
        return Err(WbhdfError::InvalidInput(format!(
            "HDF4 geometry derivation requires non-zero dimensions, found shape {:?} for '{}'",
            resolved.shape, resolved.field_name
        )));
    }

    let (ulx, uly) = resolved.upper_left_mtrs.ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "missing UpperLeftPointMtrs metadata for '{}/{}'",
            resolved.grid_name, resolved.field_name
        ))
    })?;
    let (lrx, lry) = resolved.lower_right_mtrs.ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "missing LowerRightMtrs metadata for '{}/{}'",
            resolved.grid_name, resolved.field_name
        ))
    })?;

    let pixel_size_x = (lrx - ulx) / cols as f64;
    let pixel_size_y = (lry - uly) / rows as f64;
    let geotransform = [ulx, pixel_size_x, 0.0, uly, 0.0, pixel_size_y];

    Ok(Hdf4GridGeometry {
        rows,
        cols,
        upper_left_mtrs: (ulx, uly),
        lower_right_mtrs: (lrx, lry),
        pixel_size_x,
        pixel_size_y,
        geotransform,
    })
}

pub fn derive_hdf4_grid_geometry_for_dataset(
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Hdf4GridGeometry> {
    let resolved = resolve_hdf4_dataset_path(summary, dataset_path)?;
    derive_hdf4_grid_geometry(&resolved)
}

pub fn assess_hdf4_sds_i16_decode_readiness(
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Hdf4SdsDecodeReadiness> {
    let attempt = prepare_hdf4_sds_decode_attempt(summary, dataset_path)?;
    let resolved = attempt.resolved_field;
    let mut blockers = Vec::<String>::new();

    if resolved.data_type.as_deref() != Some("DFNT_INT16") {
        blockers.push(format!(
            "unsupported data type for i16 decode: {:?}",
            resolved.data_type
        ));
    }

    let geometry = match derive_hdf4_grid_geometry(&resolved) {
        Ok(g) => Some(g),
        Err(err) => {
            blockers.push(format!("missing/invalid geometry metadata: {err}"));
            None
        }
    };

    blockers.push("HDF4 SDS payload decode backend not yet implemented".to_string());

    Ok(Hdf4SdsDecodeReadiness {
        dataset_path: dataset_path.to_string(),
        resolved_field: resolved,
        geometry,
        payload_candidates: Vec::new(),
        blockers,
    })
}

pub fn parse_hdf4_data_descriptors(bytes: &[u8]) -> WbhdfResult<Vec<Hdf4DataDescriptor>> {
    if bytes.len() < 10 {
        return Err(WbhdfError::InvalidInput(
            "HDF4 descriptor parse requires at least 10 bytes".to_string(),
        ));
    }
    if bytes[..4] != HDF4_MAGIC {
        return Err(WbhdfError::InvalidInput(
            "HDF4 descriptor parse requires HDF4 magic signature".to_string(),
        ));
    }

    let mut descriptors = Vec::<Hdf4DataDescriptor>::new();
    let mut block_offset = 4usize;
    let mut visited_blocks = std::collections::BTreeSet::<usize>::new();
    let mut block_count = 0usize;
    const MAX_DD_BLOCKS: usize = 8192;

    loop {
        if block_count >= MAX_DD_BLOCKS {
            return Err(WbhdfError::UnsupportedLayout(
                "HDF4 descriptor parse exceeded DD block traversal limit".to_string(),
            ));
        }
        if !visited_blocks.insert(block_offset) {
            return Err(WbhdfError::UnsupportedLayout(
                "HDF4 descriptor parse detected DD block cycle".to_string(),
            ));
        }
        if block_offset + 6 > bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "HDF4 DD block header out of bounds at offset {block_offset}"
            )));
        }

        let descriptor_count = BigEndian::read_u16(&bytes[block_offset..block_offset + 2]);
        let next_block_offset = BigEndian::read_u32(&bytes[block_offset + 2..block_offset + 6]);
        let table_offset = block_offset + 6;
        let table_len = descriptor_count as usize * 12;
        if table_offset + table_len > bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "HDF4 DD table out of bounds at offset {table_offset} with {descriptor_count} descriptors"
            )));
        }

        for i in 0..descriptor_count as usize {
            let base = table_offset + i * 12;
            let tag = BigEndian::read_u16(&bytes[base..base + 2]);
            let reference = BigEndian::read_u16(&bytes[base + 2..base + 4]);
            let offset = BigEndian::read_u32(&bytes[base + 4..base + 8]);
            let length = BigEndian::read_u32(&bytes[base + 8..base + 12]);
            descriptors.push(Hdf4DataDescriptor {
                tag,
                reference,
                offset,
                length,
            });
        }

        block_count += 1;
        if next_block_offset == 0 {
            break;
        }
        let next = next_block_offset as usize;
        if next >= bytes.len() {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "HDF4 next DD block offset out of bounds: {next}"
            )));
        }
        block_offset = next;
    }

    Ok(descriptors)
}

pub fn parse_hdf4_data_descriptors_in_file(path: &Path) -> WbhdfResult<Vec<Hdf4DataDescriptor>> {
    let bytes = fs::read(path)?;
    parse_hdf4_data_descriptors(&bytes)
}

pub fn find_hdf4_sds_i16_payload_candidates(
    bytes: &[u8],
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Vec<Hdf4DataDescriptor>> {
    let resolved = resolve_hdf4_dataset_path(summary, dataset_path)?;
    if resolved.shape.len() != 2 {
        return Ok(Vec::new());
    }
    let rows = resolved.shape[0];
    let cols = resolved.shape[1];
    let expected_bytes = rows
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "shape overflow while computing expected i16 payload bytes for '{}': {:?}",
                dataset_path, resolved.shape
            ))
        })?;

    let descriptors = parse_hdf4_data_descriptors(bytes)?;
    let mut candidates = Vec::<Hdf4DataDescriptor>::new();
    for dd in descriptors {
        if dd.length as usize != expected_bytes {
            continue;
        }
        let end = dd.offset as usize + dd.length as usize;
        if end <= bytes.len() {
            candidates.push(dd);
        }
    }
    Ok(candidates)
}

pub fn rank_hdf4_sds_i16_payload_candidates(
    bytes: &[u8],
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
    limit: usize,
) -> WbhdfResult<Vec<Hdf4RankedPayloadCandidate>> {
    let resolved = resolve_hdf4_dataset_path(summary, dataset_path)?;
    if resolved.shape.len() != 2 {
        return Ok(Vec::new());
    }
    let rows = resolved.shape[0];
    let cols = resolved.shape[1];
    let expected_length = rows
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "shape overflow while computing expected i16 payload bytes for '{}': {:?}",
                dataset_path, resolved.shape
            ))
        })?;

    let descriptors = parse_hdf4_data_descriptors(bytes)?;
    let mut ranked = Vec::<Hdf4RankedPayloadCandidate>::new();
    for descriptor in descriptors {
        if descriptor.length == 0 {
            continue;
        }
        let start = descriptor.offset as usize;
        let end = start.saturating_add(descriptor.length as usize);
        if start >= bytes.len() || end > bytes.len() {
            continue;
        }
        let actual_length = descriptor.length as usize;
        let length_delta = actual_length.abs_diff(expected_length);
        let preview = &bytes[start..usize::min(start + 8, bytes.len())];
        ranked.push(Hdf4RankedPayloadCandidate {
            descriptor,
            expected_length,
            length_delta,
            signature_hint: classify_payload_signature(preview),
            preview_hex: to_hex_preview(preview),
        });
    }

    ranked.sort_by_key(|entry| {
        (
            entry.length_delta,
            entry.descriptor.tag,
            entry.descriptor.reference,
        )
    });
    if ranked.len() > limit {
        ranked.truncate(limit);
    }
    Ok(ranked)
}

pub fn rank_hdf4_sds_i16_payload_candidates_in_file(
    path: &Path,
    dataset_path: &str,
    limit: usize,
) -> WbhdfResult<Vec<Hdf4RankedPayloadCandidate>> {
    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    rank_hdf4_sds_i16_payload_candidates(&bytes, &summary, dataset_path, limit)
}

pub fn map_hdf4_sds_i16_descriptor_heuristic(
    bytes: &[u8],
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
) -> WbhdfResult<Hdf4HeuristicDescriptorMapping> {
    let ranked = rank_hdf4_sds_i16_payload_candidates(bytes, summary, dataset_path, 64)?;
    let expected_length = resolve_hdf4_dataset_path(summary, dataset_path)?
        .shape
        .into_iter()
        .try_fold(2usize, |acc, dim| acc.checked_mul(dim))
        .ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "shape overflow while computing expected i16 payload bytes for '{}'",
                dataset_path
            ))
        })?;

    if ranked.is_empty() {
        return Ok(Hdf4HeuristicDescriptorMapping {
            dataset_path: dataset_path.to_string(),
            expected_length,
            selected: None,
            rationale: "no in-bounds ranked descriptor candidates available".to_string(),
            considered_count: 0,
            confidence: "none".to_string(),
        });
    }

    if let Some(exact) = ranked.iter().find(|entry| entry.length_delta == 0) {
        return Ok(Hdf4HeuristicDescriptorMapping {
            dataset_path: dataset_path.to_string(),
            expected_length,
            selected: Some(exact.clone()),
            rationale: "selected exact-length descriptor candidate".to_string(),
            considered_count: ranked.len(),
            confidence: "high".to_string(),
        });
    }

    let mut selected = ranked[0].clone();
    let mut rationale = "selected nearest descriptor by length delta".to_string();
    let mut confidence = if selected.length_delta <= selected.expected_length / 4 {
        "medium".to_string()
    } else {
        "low".to_string()
    };

    if let Some(compressed_like) = ranked.iter().find(|entry| {
        (entry.signature_hint == "gzip" || entry.signature_hint == "zlib")
            && entry.length_delta <= expected_length
    }) {
        selected = compressed_like.clone();
        rationale = "selected nearest compressed-like descriptor candidate".to_string();
        confidence = if selected.length_delta <= expected_length / 8 {
            "medium".to_string()
        } else {
            "low".to_string()
        };
    }

    Ok(Hdf4HeuristicDescriptorMapping {
        dataset_path: dataset_path.to_string(),
        expected_length,
        selected: Some(selected),
        rationale,
        considered_count: ranked.len(),
        confidence,
    })
}

pub fn map_hdf4_sds_i16_descriptor_heuristic_in_file(
    path: &Path,
    dataset_path: &str,
) -> WbhdfResult<Hdf4HeuristicDescriptorMapping> {
    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    map_hdf4_sds_i16_descriptor_heuristic(&bytes, &summary, dataset_path)
}

pub fn probe_hdf4_sds_i16_payload_window(
    bytes: &[u8],
    summary: &Hdf4EosMetadataSummary,
    dataset_path: &str,
    max_values: usize,
) -> WbhdfResult<Hdf4SdsPayloadProbe> {
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "payload probe requires max_values >= 1".to_string(),
        ));
    }

    let mapping = map_hdf4_sds_i16_descriptor_heuristic(bytes, summary, dataset_path)?;
    let Some(candidate) = mapping.selected else {
        return Ok(Hdf4SdsPayloadProbe {
            dataset_path: dataset_path.to_string(),
            status: "no_candidate".to_string(),
            rationale: mapping.rationale,
            candidate: None,
            little_endian_preview: Vec::new(),
            big_endian_preview: Vec::new(),
        });
    };

    let descriptor = &candidate.descriptor;
    let start = descriptor.offset as usize;
    let end = start.saturating_add(descriptor.length as usize);
    if start >= bytes.len() || end > bytes.len() {
        return Ok(Hdf4SdsPayloadProbe {
            dataset_path: dataset_path.to_string(),
            status: "candidate_out_of_bounds".to_string(),
            rationale: "selected descriptor bytes are out of file bounds".to_string(),
            candidate: Some(candidate),
            little_endian_preview: Vec::new(),
            big_endian_preview: Vec::new(),
        });
    }

    let payload = &bytes[start..end];
    if candidate.signature_hint == "ascii" {
        return Ok(Hdf4SdsPayloadProbe {
            dataset_path: dataset_path.to_string(),
            status: "textual_payload".to_string(),
            rationale: "selected descriptor appears textual/ascii".to_string(),
            candidate: Some(candidate),
            little_endian_preview: Vec::new(),
            big_endian_preview: Vec::new(),
        });
    }

    if candidate.signature_hint == "gzip" || candidate.signature_hint == "zlib" {
        if payload.len() > MAX_COMPRESSED_PROBE_BYTES {
            return Ok(Hdf4SdsPayloadProbe {
                dataset_path: dataset_path.to_string(),
                status: "compressed_payload".to_string(),
                rationale: format!(
                    "selected descriptor appears compressed ({}) but exceeds bounded probe limit ({} bytes)",
                    candidate.signature_hint,
                    MAX_COMPRESSED_PROBE_BYTES
                ),
                candidate: Some(candidate),
                little_endian_preview: Vec::new(),
                big_endian_preview: Vec::new(),
            });
        }

        let decompressed = if candidate.signature_hint == "gzip" {
            decompress_gzip(payload)
        } else {
            decompress_zlib(payload)
        };

        return match decompressed {
            Ok(decoded_payload) => {
                let (little, big) = decode_i16_previews(&decoded_payload, max_values);
                if little.is_empty() {
                    Ok(Hdf4SdsPayloadProbe {
                        dataset_path: dataset_path.to_string(),
                        status: "insufficient_bytes".to_string(),
                        rationale: format!(
                            "decompressed {} payload but it is too small for i16 preview",
                            candidate.signature_hint
                        ),
                        candidate: Some(candidate),
                        little_endian_preview: Vec::new(),
                        big_endian_preview: Vec::new(),
                    })
                } else {
                    Ok(Hdf4SdsPayloadProbe {
                        dataset_path: dataset_path.to_string(),
                        status: "decoded_preview".to_string(),
                        rationale: format!(
                            "decoded {} i16 preview values from {}-compressed payload",
                            little.len(),
                            candidate.signature_hint
                        ),
                        candidate: Some(candidate),
                        little_endian_preview: little,
                        big_endian_preview: big,
                    })
                }
            }
            Err(err) => Ok(Hdf4SdsPayloadProbe {
                dataset_path: dataset_path.to_string(),
                status: "compressed_payload".to_string(),
                rationale: format!(
                    "selected descriptor appears compressed ({}) but decode probe failed: {}",
                    candidate.signature_hint, err
                ),
                candidate: Some(candidate),
                little_endian_preview: Vec::new(),
                big_endian_preview: Vec::new(),
            }),
        };
    }

    let (little, big) = decode_i16_previews(payload, max_values);
    if little.is_empty() {
        return Ok(Hdf4SdsPayloadProbe {
            dataset_path: dataset_path.to_string(),
            status: "insufficient_bytes".to_string(),
            rationale: "selected descriptor payload too small for i16 preview".to_string(),
            candidate: Some(candidate),
            little_endian_preview: Vec::new(),
            big_endian_preview: Vec::new(),
        });
    }

    Ok(Hdf4SdsPayloadProbe {
        dataset_path: dataset_path.to_string(),
        status: "decoded_preview".to_string(),
        rationale: format!(
            "decoded {} i16 preview values from selected binary descriptor",
            little.len()
        ),
        candidate: Some(candidate),
        little_endian_preview: little,
        big_endian_preview: big,
    })
}

pub fn probe_hdf4_sds_i16_payload_window_in_file(
    path: &Path,
    dataset_path: &str,
    max_values: usize,
) -> WbhdfResult<Hdf4SdsPayloadProbe> {
    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    probe_hdf4_sds_i16_payload_window(&bytes, &summary, dataset_path, max_values)
}

pub fn find_hdf4_sds_i16_payload_candidates_in_file(
    path: &Path,
    dataset_path: &str,
) -> WbhdfResult<Vec<Hdf4DataDescriptor>> {
    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    find_hdf4_sds_i16_payload_candidates(&bytes, &summary, dataset_path)
}

pub fn assess_hdf4_sds_i16_decode_readiness_in_file(
    path: &Path,
    dataset_path: &str,
) -> WbhdfResult<Hdf4SdsDecodeReadiness> {
    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    let mut readiness = assess_hdf4_sds_i16_decode_readiness(&summary, dataset_path)?;
    match find_hdf4_sds_i16_payload_candidates(&bytes, &summary, dataset_path) {
        Ok(candidates) => {
            readiness.payload_candidates = candidates;
            if readiness.payload_candidates.is_empty() {
                readiness.blockers.push(
                    "no in-bounds HDF4 descriptor candidates matched expected SDS i16 payload length"
                        .to_string(),
                );
                if let Ok(nearby) =
                    rank_hdf4_sds_i16_payload_candidates(&bytes, &summary, dataset_path, 3)
                {
                    if !nearby.is_empty() {
                        let nearest = nearby
                            .iter()
                            .map(|entry| {
                                format!(
                                    "tag=0x{:04X} ref={} len={} delta={} hint={} preview={}",
                                    entry.descriptor.tag,
                                    entry.descriptor.reference,
                                    entry.descriptor.length,
                                    entry.length_delta,
                                    entry.signature_hint,
                                    entry.preview_hex
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" | ");
                        readiness
                            .blockers
                            .push(format!("nearest payload descriptors: {nearest}"));
                    }
                }
                if let Ok(mapping) =
                    map_hdf4_sds_i16_descriptor_heuristic(&bytes, &summary, dataset_path)
                {
                    if let Some(selected) = mapping.selected {
                        readiness.blockers.push(format!(
                            "heuristic descriptor mapping: confidence={} tag=0x{:04X} ref={} len={} delta={} hint={} rationale='{}'",
                            mapping.confidence,
                            selected.descriptor.tag,
                            selected.descriptor.reference,
                            selected.descriptor.length,
                            selected.length_delta,
                            selected.signature_hint,
                            mapping.rationale
                        ));
                    } else {
                        readiness.blockers.push(format!(
                            "heuristic descriptor mapping unavailable: confidence={} rationale={}",
                            mapping.confidence, mapping.rationale
                        ));
                    }
                }
                if let Ok(probe) =
                    probe_hdf4_sds_i16_payload_window(&bytes, &summary, dataset_path, 8)
                {
                    readiness.blockers.push(format!(
                        "payload probe status={} rationale='{}'",
                        probe.status, probe.rationale
                    ));
                }
            } else {
                readiness.blockers.push(format!(
                    "descriptor-to-field mapping is not yet implemented ({} candidates with matching payload length)",
                    readiness.payload_candidates.len()
                ));
            }
        }
        Err(err) => {
            readiness.blockers.push(format!(
                "descriptor candidate scan failed: {err}"
            ));
        }
    }

    Ok(readiness)
}

pub fn decode_hdf4_sds_i16_in_file(path: &Path, dataset_path: &str) -> WbhdfResult<Vec<i16>> {
    let readiness = assess_hdf4_sds_i16_decode_readiness_in_file(path, dataset_path)?;
    let resolved = &readiness.resolved_field;
    let probe = probe_hdf4_sds_i16_payload_window_in_file(
        path,
        dataset_path,
        DEFAULT_HDF4_SDS_I16_WINDOW_VALUES,
    )
    .ok();

    if resolved.data_type.as_deref() != Some("DFNT_INT16") {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "HDF4 SDS i16 decode requires DFNT_INT16 but found {:?} at '{}'; resolved shape={:?}",
            resolved.data_type,
            readiness.dataset_path,
            resolved.shape
        )));
    }

    if let Ok(values) = decode_hdf4_sds_i16_window_at_in_file(
        path,
        dataset_path,
        0,
        DEFAULT_HDF4_SDS_I16_WINDOW_VALUES,
    ) {
        return Ok(values);
    }

    if let Some(probe) = &probe {
        if probe.status == "decoded_preview" && !probe.little_endian_preview.is_empty() {
            return Ok(select_preferred_i16_preview(
                &probe.little_endian_preview,
                &probe.big_endian_preview,
            )
            .to_vec());
        }
    }

    Err(WbhdfError::UnsupportedLayout(format!(
        "HDF4 SDS payload decode is not yet implemented for '{}'; resolved grid='{}', field='{}', data_type={:?}, shape={:?}, projection={:?}, geotransform={:?}, blockers={:?}",
        readiness.dataset_path,
        resolved.grid_name,
        resolved.field_name,
        resolved.data_type,
        resolved.shape,
        resolved.projection,
        readiness.geometry.as_ref().map(|g| g.geotransform),
        if let Some(probe) = &probe {
            let mut blockers = readiness.blockers.clone();
            blockers.push(format!(
                "payload probe: status={} rationale='{}' le_preview={:?} be_preview={:?}",
                probe.status, probe.rationale, probe.little_endian_preview, probe.big_endian_preview
            ));
            blockers
        } else {
            readiness.blockers
        }
    )))
}

pub fn attempt_decode_hdf4_sds_i16_window_in_file(
    path: &Path,
    dataset_path: &str,
    max_values: usize,
) -> WbhdfResult<Vec<i16>> {
    decode_hdf4_sds_i16_window_at_in_file(path, dataset_path, 0, max_values)
}

pub fn decode_hdf4_sds_i16_window_at_in_file(
    path: &Path,
    dataset_path: &str,
    start_value: usize,
    max_values: usize,
) -> WbhdfResult<Vec<i16>> {
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "window decode requires max_values >= 1".to_string(),
        ));
    }

    let bytes = fs::read(path)?;
    let summary = probe_hdf4_eos_metadata(&bytes)?;
    let resolved = resolve_hdf4_dataset_path(&summary, dataset_path)?;
    if resolved.data_type.as_deref() != Some("DFNT_INT16") {
        return Err(WbhdfError::DatatypeMismatch {
            dataset_path: dataset_path.to_string(),
            expected: "DFNT_INT16".to_string(),
            actual: resolved
                .data_type
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string()),
        });
    }

    let exact_candidates = find_hdf4_sds_i16_payload_candidates(&bytes, &summary, dataset_path)?;
    if !exact_candidates.is_empty() {
        let mut ranked_exact = Vec::<(Hdf4DataDescriptor, String)>::new();
        for descriptor in exact_candidates {
            let start = descriptor.offset as usize;
            let end = start.saturating_add(descriptor.length as usize);
            if start >= bytes.len() || end > bytes.len() {
                continue;
            }
            let preview = &bytes[start..usize::min(start + 8, end)];
            ranked_exact.push((descriptor, classify_payload_signature(preview)));
        }

        ranked_exact.sort_by_key(|(descriptor, signature_hint)| {
            (
                signature_preference(signature_hint),
                descriptor.tag,
                descriptor.reference,
                descriptor.offset,
                descriptor.length,
            )
        });

        let mut first_error: Option<WbhdfError> = None;
        let mut attempted_exact_candidates = 0usize;
        for (descriptor, signature_hint) in &ranked_exact {
            attempted_exact_candidates += 1;
            match decode_i16_window_from_descriptor(
                &bytes,
                descriptor,
                signature_hint,
                dataset_path,
                start_value,
                max_values,
            ) {
                Ok(values) => return Ok(values),
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
            }
        }

        if let Some(err) = first_error {
            return Err(annotate_exact_candidate_failure(
                err,
                dataset_path,
                attempted_exact_candidates,
                ranked_exact.len(),
            ));
        }
    }

    let mapping = map_hdf4_sds_i16_descriptor_heuristic(&bytes, &summary, dataset_path)?;
    let Some(candidate) = mapping.selected else {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "HDF4 SDS i16 window decode attempt unavailable for '{}': status=no_candidate rationale='{}'",
            dataset_path, mapping.rationale
        )));
    };

    decode_i16_window_from_descriptor(
        &bytes,
        &candidate.descriptor,
        &candidate.signature_hint,
        dataset_path,
        start_value,
        max_values,
    )
}

fn decode_i16_window_from_descriptor(
    bytes: &[u8],
    descriptor: &Hdf4DataDescriptor,
    signature_hint: &str,
    dataset_path: &str,
    start_value: usize,
    max_values: usize,
) -> WbhdfResult<Vec<i16>> {
    let chunk_coordinate = descriptor_chunk_coordinate(descriptor);
    let start = descriptor.offset as usize;
    let end = start.saturating_add(descriptor.length as usize);
    if start >= bytes.len() || end > bytes.len() {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(chunk_coordinate.clone()),
            file_offset: descriptor.offset as u64,
            detail: "selected descriptor bytes are out of file bounds".to_string(),
        });
    }

    let payload = &bytes[start..end];
    if signature_hint == "ascii" {
        return Err(WbhdfError::UnsupportedLayout(format!(
            "HDF4 SDS i16 window decode attempt unavailable for '{}': status=textual_payload rationale='selected descriptor appears textual/ascii'",
            dataset_path
        )));
    }

    let decoded_payload = if signature_hint == "gzip" || signature_hint == "zlib" {
        if payload.len() > MAX_COMPRESSED_WINDOW_DECODE_BYTES {
            return Err(WbhdfError::UnsupportedLayout(format!(
                "HDF4 SDS i16 window decode attempt unavailable for '{}': status=compressed_payload rationale='selected descriptor appears compressed ({}) but exceeds bounded decode limit ({} bytes)'",
                dataset_path,
                signature_hint,
                MAX_COMPRESSED_WINDOW_DECODE_BYTES
            )));
        }

        if signature_hint == "gzip" {
            decompress_gzip(payload).map_err(|err| {
                WbhdfError::FilterFailure {
                    dataset_path: dataset_path.to_string(),
                    chunk_coordinate: Some(chunk_coordinate.clone()),
                    file_offset: descriptor.offset as u64,
                    filter: "gzip".to_string(),
                    detail: err.to_string(),
                }
            })?
        } else {
            decompress_zlib(payload).map_err(|err| {
                WbhdfError::FilterFailure {
                    dataset_path: dataset_path.to_string(),
                    chunk_coordinate: Some(chunk_coordinate.clone()),
                    file_offset: descriptor.offset as u64,
                    filter: "zlib".to_string(),
                    detail: err.to_string(),
                }
            })?
        }
    } else {
        payload.to_vec()
    };

    let total_values = decoded_payload.len() / 2;
    if total_values == 0 {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(chunk_coordinate.clone()),
            file_offset: descriptor.offset as u64,
            detail: "selected descriptor payload too small for i16 decode".to_string(),
        });
    }

    if start_value >= total_values {
        return Err(WbhdfError::InvalidInput(format!(
            "window start index {} is out of bounds for {} decoded i16 values",
            start_value, total_values
        )));
    }

    let decode_count = usize::min(max_values, total_values - start_value);
    let (little, big) = decode_i16_window_previews(&decoded_payload, start_value, decode_count);
    if little.is_empty() {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(chunk_coordinate),
            file_offset: descriptor.offset as u64,
            detail: "selected descriptor payload too small for requested i16 window".to_string(),
        });
    }

    Ok(select_preferred_i16_preview(&little, &big).to_vec())
}

fn extract_quoted_value(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let first_quote = rest.find('"')?;
    let after_quote = &rest[first_quote + 1..];
    let end_quote = after_quote.find('"')?;
    Some(after_quote[..end_quote].to_string())
}

fn descriptor_chunk_coordinate(descriptor: &Hdf4DataDescriptor) -> String {
    format!("tag={},ref={}", descriptor.tag, descriptor.reference)
}

fn extract_unquoted_value(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn extract_dim_list(line: &str) -> Option<Vec<String>> {
    let start = line.find("DimList=(")? + "DimList=(".len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    let inner = &rest[..end];
    let values = inner
        .split(',')
        .map(|part| part.trim().trim_matches('"').to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    Some(values)
}

fn extract_dim_size(line: &str) -> Option<(String, usize)> {
    let (name, value) = line.split_once('=')?;
    let dim_name = name.trim();
    if !dim_name.ends_with("Dim") {
        return None;
    }
    let size = value.trim().parse::<usize>().ok()?;
    Some((dim_name.to_string(), size))
}

fn extract_i32_value(line: &str, key: &str) -> Option<i32> {
    let start = line.find(key)? + key.len();
    let rest = line[start..].trim();
    rest.parse::<i32>().ok()
}

fn extract_f64_pair(line: &str, key: &str) -> Option<(f64, f64)> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    let mut parts = rest[..end].split(',').map(str::trim);
    let first = parts.next()?.parse::<f64>().ok()?;
    let second = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((first, second))
}

fn extract_f64_list(line: &str, key: &str) -> Option<Vec<f64>> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(')')?;
    let values = rest[..end]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(values)
}

fn ensure_grid(grids: &mut Vec<Hdf4GridSummary>, name: &str) {
    if !grids.iter().any(|grid| grid.name == name) {
        grids.push(Hdf4GridSummary {
            name: name.to_string(),
            dim_sizes: BTreeMap::new(),
            projection: None,
            proj_params: Vec::new(),
            sphere_code: None,
            upper_left_mtrs: None,
            lower_right_mtrs: None,
            data_fields: Vec::new(),
        });
    }
}

fn ensure_data_field(grids: &mut Vec<Hdf4GridSummary>, grid_name: &str, field_name: &str) {
    ensure_grid(grids, grid_name);
    if let Some(grid) = grids.iter_mut().find(|grid| grid.name == grid_name) {
        if !grid.data_fields.iter().any(|field| field.name == field_name) {
            grid.data_fields.push(Hdf4DataFieldSummary {
                name: field_name.to_string(),
                data_type: None,
                dim_list: Vec::new(),
            });
        }
    }
}

fn find_data_field_mut<'a>(
    grids: &'a mut [Hdf4GridSummary],
    grid_name: &str,
    field_name: &str,
) -> Option<&'a mut Hdf4DataFieldSummary> {
    grids
        .iter_mut()
        .find(|grid| grid.name == grid_name)
        .and_then(|grid| grid.data_fields.iter_mut().find(|field| field.name == field_name))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn classify_payload_signature(preview: &[u8]) -> String {
    if preview.len() >= 2 {
        if preview[0] == 0x1F && preview[1] == 0x8B {
            return "gzip".to_string();
        }
        if preview[0] == 0x78 {
            return "zlib".to_string();
        }
    }
    if !preview.is_empty()
        && preview
            .iter()
            .all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
    {
        return "ascii".to_string();
    }
    "binary".to_string()
}

fn to_hex_preview(preview: &[u8]) -> String {
    if preview.is_empty() {
        return "empty".to_string();
    }
    preview
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join("")
}

fn signature_preference(signature_hint: &str) -> u8 {
    match signature_hint {
        "binary" => 0,
        "gzip" => 1,
        "zlib" => 2,
        "ascii" => 3,
        _ => 4,
    }
}

fn annotate_exact_candidate_failure(
    err: WbhdfError,
    dataset_path: &str,
    attempted_exact_candidates: usize,
    total_ranked_exact_candidates: usize,
) -> WbhdfError {
    match err {
        WbhdfError::UnsupportedLayout(msg) => WbhdfError::UnsupportedLayout(format!(
            "HDF4 SDS i16 window decode exact-candidate phase failed for '{}': attempted={}/{}; {}",
            dataset_path, attempted_exact_candidates, total_ranked_exact_candidates, msg
        )),
        WbhdfError::InvalidChunk {
            dataset_path,
            chunk_coordinate,
            file_offset,
            detail,
        } => WbhdfError::InvalidChunk {
            dataset_path,
            chunk_coordinate,
            file_offset,
            detail: format!(
                "exact-candidate phase attempted={attempted_exact_candidates}/{total_ranked_exact_candidates}; {detail}"
            ),
        },
        WbhdfError::FilterFailure {
            dataset_path,
            chunk_coordinate,
            file_offset,
            filter,
            detail,
        } => WbhdfError::FilterFailure {
            dataset_path,
            chunk_coordinate,
            file_offset,
            filter,
            detail: format!(
                "exact-candidate phase attempted={attempted_exact_candidates}/{total_ranked_exact_candidates}; {detail}"
            ),
        },
        other => other,
    }
}

fn decode_i16_previews(payload: &[u8], max_values: usize) -> (Vec<i16>, Vec<i16>) {
    let count = usize::min(max_values, payload.len() / 2);
    if count == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut little = Vec::<i16>::with_capacity(count);
    let mut big = Vec::<i16>::with_capacity(count);
    for i in 0..count {
        let base = i * 2;
        little.push(LittleEndian::read_i16(&payload[base..base + 2]));
        big.push(BigEndian::read_i16(&payload[base..base + 2]));
    }
    (little, big)
}

fn decode_i16_window_previews(
    payload: &[u8],
    start_value: usize,
    max_values: usize,
) -> (Vec<i16>, Vec<i16>) {
    if max_values == 0 {
        return (Vec::new(), Vec::new());
    }

    let total_values = payload.len() / 2;
    if start_value >= total_values {
        return (Vec::new(), Vec::new());
    }

    let count = usize::min(max_values, total_values - start_value);
    let mut little = Vec::<i16>::with_capacity(count);
    let mut big = Vec::<i16>::with_capacity(count);
    for i in 0..count {
        let base = (start_value + i) * 2;
        little.push(LittleEndian::read_i16(&payload[base..base + 2]));
        big.push(BigEndian::read_i16(&payload[base..base + 2]));
    }
    (little, big)
}

fn score_i16_preview(values: &[i16]) -> i64 {
    if values.is_empty() {
        return i64::MIN;
    }

    let mut score = 0i64;
    for value in values {
        let value = *value as i32;
        let magnitude = i64::from(value.abs());
        if magnitude <= 10_000 {
            score += 2;
        } else if magnitude <= 30_000 {
            score += 1;
        } else {
            score -= 2;
        }

        if value == 0 {
            score -= 1;
        }

        if magnitude % 256 == 0 {
            score -= 1;
        }
    }

    score
}

fn select_preferred_i16_preview<'a>(
    little_endian_preview: &'a [i16],
    big_endian_preview: &'a [i16],
) -> &'a [i16] {
    let little_score = score_i16_preview(little_endian_preview);
    let big_score = score_i16_preview(big_endian_preview);
    if big_score > little_score {
        big_endian_preview
    } else {
        little_endian_preview
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assess_hdf4_sds_i16_decode_readiness, attempt_decode_hdf4_sds_i16_window_in_file,
        HDF4_MAGIC, MAX_COMPRESSED_WINDOW_DECODE_BYTES,
        decode_hdf4_sds_i16_window_at_in_file,
        decode_hdf4_sds_i16_in_file, derive_hdf4_grid_geometry,
        derive_hdf4_grid_geometry_for_dataset, enumerate_hdf4_dataset_paths,
        find_hdf4_sds_i16_payload_candidates, parse_hdf4_data_descriptors,
        map_hdf4_sds_i16_descriptor_heuristic,
        probe_hdf4_sds_i16_payload_window,
        prepare_hdf4_sds_decode_attempt, probe_hdf4_eos_metadata, resolve_hdf4_dataset_path,
        resolve_hdf4_grid_field, rank_hdf4_sds_i16_payload_candidates,
        select_preferred_i16_preview,
    };

    #[test]
    fn extracts_grid_and_datafield_names_from_struct_metadata_text() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"MOD_Grid_500m_Surface_Reflectance\"\nXDim=2400\nYDim=2400\nUpperLeftPointMtrs=(-15567307.275333,0.000000)\nLowerRightMtrs=(-14455356.755667,-1111950.519667)\nProjection=GCTP_SNSOID\nProjParams=(6371007.181000,0,0,0,0,0,0,0,0,0,0,0,0)\nSphereCode=-1\nDataFieldName=\"sur_refl_b01\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\nDataFieldName=\"sur_refl_b02\"\nDataType=DFNT_UINT16\nDimList=(\"YDim\",\"XDim\")\n",
        );

        let summary = probe_hdf4_eos_metadata(&bytes).expect("HDF4 metadata probe should parse");
        assert_eq!(summary.struct_metadata_markers, 1);
        assert_eq!(summary.grid_names, vec!["MOD_Grid_500m_Surface_Reflectance"]);
        assert_eq!(summary.data_field_names, vec!["sur_refl_b01", "sur_refl_b02"]);
        assert_eq!(summary.grids.len(), 1);
        assert_eq!(summary.grids[0].name, "MOD_Grid_500m_Surface_Reflectance");
        assert_eq!(summary.grids[0].dim_sizes.get("XDim"), Some(&2400));
        assert_eq!(summary.grids[0].dim_sizes.get("YDim"), Some(&2400));
        assert_eq!(summary.grids[0].projection.as_deref(), Some("GCTP_SNSOID"));
        assert_eq!(summary.grids[0].sphere_code, Some(-1));
        assert_eq!(summary.grids[0].upper_left_mtrs, Some((-15567307.275333, 0.0)));
        assert_eq!(
            summary.grids[0].lower_right_mtrs,
            Some((-14455356.755667, -1111950.519667))
        );
        assert_eq!(summary.grids[0].proj_params.len(), 13);
        assert_eq!(summary.grids[0].proj_params[0], 6371007.181000);
        assert_eq!(summary.grids[0].data_fields.len(), 2);
        assert_eq!(summary.grids[0].data_fields[0].name, "sur_refl_b01");
        assert_eq!(
            summary.grids[0].data_fields[0].data_type.as_deref(),
            Some("DFNT_INT16")
        );
        assert_eq!(summary.grids[0].data_fields[0].dim_list, vec!["YDim", "XDim"]);

        let resolved = resolve_hdf4_grid_field(
            &summary,
            "MOD_Grid_500m_Surface_Reflectance",
            "sur_refl_b01",
        )
        .expect("field should resolve");
        assert_eq!(resolved.shape, vec![2400, 2400]);
        assert_eq!(resolved.projection.as_deref(), Some("GCTP_SNSOID"));
        assert_eq!(resolved.sphere_code, Some(-1));
        assert_eq!(resolved.upper_left_mtrs, Some((-15567307.275333, 0.0)));
        assert_eq!(
            resolved.lower_right_mtrs,
            Some((-14455356.755667, -1111950.519667))
        );

        let resolved_from_path =
            resolve_hdf4_dataset_path(&summary, "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01")
                .expect("dataset path should resolve");
        assert_eq!(resolved_from_path.shape, vec![2400, 2400]);

        let geometry = derive_hdf4_grid_geometry(&resolved_from_path)
            .expect("geometry should be derivable from parsed metadata");
        assert_eq!(geometry.rows, 2400);
        assert_eq!(geometry.cols, 2400);
        assert!((geometry.pixel_size_x - 463.3127165275).abs() < 1e-9);
        assert!((geometry.pixel_size_y + 463.31271652791664).abs() < 1e-9);

        let paths = enumerate_hdf4_dataset_paths(&summary);
        assert_eq!(
            paths,
            vec![
                "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01",
                "/MOD_Grid_500m_Surface_Reflectance/sur_refl_b02",
            ]
        );
    }

    #[test]
    fn rejects_non_hdf4_signature() {
        let err = probe_hdf4_eos_metadata(b"not-hdf4").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("HDF4"));
    }

    #[test]
    fn rejects_invalid_hdf4_dataset_paths() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");

        let err = resolve_hdf4_dataset_path(&summary, "GridA/FieldA").unwrap_err();
        assert!(format!("{err}").contains("must start"));

        let err = resolve_hdf4_dataset_path(&summary, "/GridA").unwrap_err();
        assert!(format!("{err}").contains("include data field"));

        let err = resolve_hdf4_dataset_path(&summary, "/GridA/FieldA/extra").unwrap_err();
        assert!(format!("{err}").contains("two segments"));
    }

    #[test]
    fn prepares_decode_attempt_for_canonical_hdf4_dataset_path() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");

        let attempt = prepare_hdf4_sds_decode_attempt(&summary, "/GridA/FieldA")
            .expect("decode attempt should resolve canonical path");
        assert_eq!(attempt.dataset_path, "/GridA/FieldA");
        assert_eq!(attempt.resolved_field.shape, vec![2, 2]);
        assert_eq!(attempt.resolved_field.data_type.as_deref(), Some("DFNT_INT16"));
    }

    #[test]
    fn reports_unimplemented_decode_for_hdf4_sds_i16_payload_path() {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().expect("temp file should be created");
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nProjection=GCTP_SNSOID\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        file.write_all(&bytes).expect("temp file should be written");

        let err = decode_hdf4_sds_i16_in_file(file.path(), "/GridA/FieldA")
            .expect_err("decode should currently report unsupported layout");
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
        assert!(msg.contains("/GridA/FieldA"));
        assert!(msg.contains("DFNT_INT16"));
        assert!(msg.contains("geotransform"));
        assert!(msg.contains("blockers"));
    }

    #[test]
    fn reports_structured_decode_readiness() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nUpperLeftPointMtrs=(0,10)\nLowerRightMtrs=(20,0)\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let readiness = assess_hdf4_sds_i16_decode_readiness(&summary, "/GridA/FieldA")
            .expect("readiness should evaluate");

        assert_eq!(readiness.dataset_path, "/GridA/FieldA");
        assert_eq!(readiness.resolved_field.shape, vec![2, 2]);
        assert!(readiness.geometry.is_some());
        assert_eq!(readiness.blockers.len(), 1);
        assert!(readiness.blockers[0].contains("not yet implemented"));
    }

    #[test]
    fn parses_hdf4_descriptor_blocks() {
        // Magic + DD block header + 2 descriptors, no next block.
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x02]); // descriptor count
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // next block
        // Descriptor 1: tag=0x02BD, ref=1, offset=100, length=200
        bytes.extend_from_slice(&[0x02, 0xBD, 0x00, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0xC8]);
        // Descriptor 2: tag=0x02BE, ref=2, offset=300, length=400
        bytes.extend_from_slice(&[0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x01, 0x2C, 0x00, 0x00, 0x01, 0x90]);

        let descriptors = parse_hdf4_data_descriptors(&bytes).expect("descriptor parse should succeed");
        assert_eq!(descriptors.len(), 2);
        assert_eq!(descriptors[0].tag, 0x02BD);
        assert_eq!(descriptors[0].reference, 1);
        assert_eq!(descriptors[0].offset, 100);
        assert_eq!(descriptors[0].length, 200);
        assert_eq!(descriptors[1].tag, 0x02BE);
        assert_eq!(descriptors[1].reference, 2);
    }

    #[test]
    fn finds_matching_payload_length_candidates() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x02]); // descriptor count
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // next block
        // Candidate length = 8 bytes for 2x2 i16
        bytes.extend_from_slice(&[0x02, 0xBE, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x08]);
        // Non-candidate
        bytes.extend_from_slice(&[0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x10]);
        bytes.resize(0x80, 0);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        // Keep bytes as valid HDF4 with descriptor block at start and metadata text later.
        bytes.extend_from_slice(&metadata[4..]);
        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let candidates = find_hdf4_sds_i16_payload_candidates(&bytes, &summary, "/GridA/FieldA")
            .expect("candidate search should succeed");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].length, 8);
    }

    #[test]
    fn ranks_nearest_payload_candidates_with_signature_hints() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x03]); // descriptor count
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // next block
        // length 12 (delta 4 from expected 8)
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x0C,
        ]);
        // length 9 (delta 1 from expected 8)
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x09,
        ]);
        // length 8 exact
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x03, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        bytes.resize(0x90, 0);
        // descriptor payload previews
        bytes[0x40..0x48].copy_from_slice(b"ABCD1234");
        bytes[0x60..0x68].copy_from_slice(&[0x78, 0x9C, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        bytes[0x80..0x88].copy_from_slice(&[0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let ranked = rank_hdf4_sds_i16_payload_candidates(&bytes, &summary, "/GridA/FieldA", 3)
            .expect("ranking should succeed");
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].descriptor.reference, 3);
        assert_eq!(ranked[0].length_delta, 0);
        assert_eq!(ranked[0].signature_hint, "gzip");
        assert_eq!(ranked[1].descriptor.reference, 2);
        assert_eq!(ranked[1].signature_hint, "zlib");
        assert_eq!(ranked[2].descriptor.reference, 1);
        assert_eq!(ranked[2].signature_hint, "ascii");
    }

    #[test]
    fn heuristic_mapping_prefers_exact_length_candidate_when_present() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x02]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // exact length candidate (8 bytes)
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x03, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        // near candidate (9 bytes)
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x09,
        ]);
        bytes.resize(0x90, 0);
        bytes[0x80..0x88].copy_from_slice(&[0x1F, 0x8B, 0x08, 0, 0, 0, 0, 0]);
        bytes[0x60..0x68].copy_from_slice(&[0x78, 0x9C, 1, 2, 3, 4, 5, 6]);
        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let mapping = map_hdf4_sds_i16_descriptor_heuristic(&bytes, &summary, "/GridA/FieldA")
            .expect("heuristic mapping should succeed");
        let selected = mapping.selected.expect("mapping should choose a candidate");
        assert_eq!(selected.descriptor.reference, 3);
        assert_eq!(selected.length_delta, 0);
        assert_eq!(mapping.confidence, "high");
    }

    #[test]
    fn payload_probe_decodes_i16_preview_for_binary_candidate() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // exact-length candidate for 2x2 i16 -> 8 bytes
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x03, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        bytes.resize(0x90, 0);
        // Little-endian i16 values: 1, 2, -3, 256
        bytes[0x80..0x88].copy_from_slice(&[0x01, 0x00, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x01]);
        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let probe = probe_hdf4_sds_i16_payload_window(&bytes, &summary, "/GridA/FieldA", 4)
            .expect("payload probe should succeed");
        assert_eq!(probe.status, "decoded_preview");
        assert_eq!(probe.little_endian_preview, vec![1, 2, -3, 256]);
        assert_eq!(probe.big_endian_preview.len(), 4);
    }

    #[test]
    fn payload_probe_decodes_i16_preview_from_zlib_candidate() {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        // Little-endian i16 values: 10, -20, 30, -40
        encoder
            .write_all(&[0x0A, 0x00, 0xEC, 0xFF, 0x1E, 0x00, 0xD8, 0xFF])
            .expect("zlib payload should be writable");
        let compressed = encoder.finish().expect("zlib payload should finish");

        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        bytes.extend_from_slice(&[
            0x02,
            0xBE,
            0x00,
            0x03,
            0x00,
            0x00,
            0x00,
            0x80,
            ((compressed.len() >> 24) & 0xFF) as u8,
            ((compressed.len() >> 16) & 0xFF) as u8,
            ((compressed.len() >> 8) & 0xFF) as u8,
            (compressed.len() & 0xFF) as u8,
        ]);
        bytes.resize(0x80, 0);
        bytes.extend_from_slice(&compressed);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");
        let probe = probe_hdf4_sds_i16_payload_window(&bytes, &summary, "/GridA/FieldA", 4)
            .expect("payload probe should succeed");
        assert_eq!(probe.status, "decoded_preview");
        assert_eq!(probe.little_endian_preview, vec![10, -20, 30, -40]);
        assert_eq!(probe.big_endian_preview.len(), 4);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");
        let decoded = attempt_decode_hdf4_sds_i16_window_in_file(tmp.path(), "/GridA/FieldA", 4)
            .expect("window decode attempt should succeed for decoded preview status");
        assert_eq!(decoded, vec![10, -20, 30, -40]);
    }

    #[test]
    fn window_decode_attempt_uses_preferred_endianness() {
        use std::io::Write;

        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // exact-length candidate for 2x2 i16 -> 8 bytes
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x03, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        bytes.resize(0x90, 0);
        // Big-endian i16 values: 10, -20, 30, -40
        bytes[0x80..0x88].copy_from_slice(&[0x00, 0x0A, 0xFF, 0xEC, 0x00, 0x1E, 0xFF, 0xD8]);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");

        let decoded = attempt_decode_hdf4_sds_i16_window_in_file(tmp.path(), "/GridA/FieldA", 4)
            .expect("window decode attempt should succeed for decoded preview status");
        assert_eq!(decoded, vec![10, -20, 30, -40]);
    }

    #[test]
    fn window_decode_at_supports_start_offset() {
        use std::io::Write;

        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x01]);
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // exact-length candidate for 2x4 i16 -> 16 bytes
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x03, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x10,
        ]);
        bytes.resize(0x90, 0);
        // Little-endian i16 values: 1..=8
        bytes[0x80..0x90].copy_from_slice(&[
            0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00,
            0x05, 0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00,
        ]);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=4\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");

        let decoded = decode_hdf4_sds_i16_window_at_in_file(tmp.path(), "/GridA/FieldA", 2, 3)
            .expect("offset-based window decode should succeed");
        assert_eq!(decoded, vec![3, 4, 5]);
    }

    #[test]
    fn window_decode_at_rejects_oversized_compressed_payload_candidate() {
        use std::io::Write;

        let oversized_len = MAX_COMPRESSED_WINDOW_DECODE_BYTES + 1;
        let descriptor_offset = 0x80usize;
        let total_len = descriptor_offset + oversized_len;
        let mut bytes = vec![0u8; total_len.max(256)];

        bytes[0..4].copy_from_slice(&HDF4_MAGIC);
        bytes[4..6].copy_from_slice(&[0x00, 0x01]); // one descriptor
        bytes[6..10].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]); // no next DD block

        // Descriptor: tag=0x02BE, ref=3, offset=0x80, length=oversized
        bytes[10..22].copy_from_slice(&[
            0x02,
            0xBE,
            0x00,
            0x03,
            0x00,
            0x00,
            0x00,
            0x80,
            ((oversized_len >> 24) & 0xFF) as u8,
            ((oversized_len >> 16) & 0xFF) as u8,
            ((oversized_len >> 8) & 0xFF) as u8,
            (oversized_len & 0xFF) as u8,
        ]);

        // Mark payload as gzip-like so the compressed-size guardrail path is exercised.
        bytes[descriptor_offset] = 0x1F;
        bytes[descriptor_offset + 1] = 0x8B;

        let metadata = b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n";
        bytes.extend_from_slice(metadata);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");

        let err = decode_hdf4_sds_i16_window_at_in_file(tmp.path(), "/GridA/FieldA", 0, 4)
            .expect_err("oversized compressed payload candidate should be rejected");
        let msg = format!("{err}");
        assert!(msg.contains("compressed_payload"));
        assert!(msg.contains("exceeds bounded decode limit"));
    }

    #[test]
    fn window_decode_at_prefers_binary_exact_candidate_over_ascii_exact_candidate() {
        use std::io::Write;

        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x02]); // two descriptors
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Exact-length candidate 1: ascii payload
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        // Exact-length candidate 2: binary i16 payload
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x08,
        ]);

        bytes.resize(0xA0, 0);
        bytes[0x80..0x88].copy_from_slice(b"ABCDEFGH");
        bytes[0x90..0x98].copy_from_slice(&[0x01, 0x00, 0x02, 0x00, 0xFD, 0xFF, 0x04, 0x00]);

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");

        let decoded = decode_hdf4_sds_i16_window_at_in_file(tmp.path(), "/GridA/FieldA", 0, 4)
            .expect("decode should use binary exact candidate even when ascii exact candidate exists");
        assert_eq!(decoded, vec![1, 2, -3, 4]);
    }

    #[test]
    fn window_decode_at_reports_exact_candidate_attempt_counts_on_failure() {
        use std::io::Write;

        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(&[0x00, 0x02]); // two descriptors
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // Two exact-length textual candidates so exact-candidate decode phase fails deterministically.
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x01, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x08,
        ]);
        bytes.extend_from_slice(&[
            0x02, 0xBE, 0x00, 0x02, 0x00, 0x00, 0x00, 0x90, 0x00, 0x00, 0x00, 0x08,
        ]);

        bytes.resize(0xA0, 0);
        bytes[0x80..0x88].copy_from_slice(b"ABCDEFGH");
        bytes[0x90..0x98].copy_from_slice(b"12345678");

        let mut metadata = vec![0x0E, 0x03, 0x13, 0x01];
        metadata.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        bytes.extend_from_slice(&metadata[4..]);

        let mut tmp = tempfile::NamedTempFile::new().expect("temp file should be created");
        tmp.write_all(&bytes).expect("temp file should be written");

        let err = decode_hdf4_sds_i16_window_at_in_file(tmp.path(), "/GridA/FieldA", 0, 4)
            .expect_err("decode should fail for textual exact candidates");
        let msg = format!("{err}");
        assert!(msg.contains("exact-candidate phase failed"));
        assert!(msg.contains("attempted=2/2"));
    }

    #[test]
    fn selects_more_plausible_i16_preview_by_score() {
        let little = [12, -18, 24, -30];
        let big = [12_288, -16_384, 24_576, -28_672];

        let selected = select_preferred_i16_preview(&little, &big);
        assert_eq!(selected, &little);
    }

    #[test]
    fn geometry_derivation_requires_corner_coordinates() {
        let mut bytes = vec![0x0E, 0x03, 0x13, 0x01];
        bytes.extend_from_slice(
            b"\nStructMetadata.0\nGridName=\"GridA\"\nXDim=2\nYDim=2\nDataFieldName=\"FieldA\"\nDataType=DFNT_INT16\nDimList=(\"YDim\",\"XDim\")\n",
        );
        let summary = probe_hdf4_eos_metadata(&bytes).expect("metadata should parse");

        let err = derive_hdf4_grid_geometry_for_dataset(&summary, "/GridA/FieldA")
            .expect_err("geometry derivation should fail without corner metadata");
        assert!(format!("{err}").contains("UpperLeftPointMtrs"));
    }
}
