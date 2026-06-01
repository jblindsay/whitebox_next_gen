use crate::btree::{
    lookup_chunk_address, read_chunk_payload_in_file, read_chunked_storage_records_bounded_in_file,
    ChunkIndex,
};
use crate::datatypes::{
    decode_f32_slice, decode_f64_slice, decode_i16_slice, decode_u16_slice, Endianness,
};
use crate::error::{WbhdfError, WbhdfResult};
use crate::filters::decompress_zlib;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Minimal dataset descriptor used during early scaffolding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDescriptor {
    pub path: String,
}

/// Dataset-level chunk locator wiring B-tree lookup into dataset reads.
#[derive(Debug, Clone)]
pub struct DatasetChunkLocator {
    descriptor: DatasetDescriptor,
    chunk_index: ChunkIndex,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FillMappedF32 {
    pub values: Vec<f32>,
    pub nodata_value: f32,
    pub valid_count: usize,
    pub nodata_count: usize,
}

/// Resolves a dataset descriptor from a canonical dataset path.
pub fn resolve_dataset(path: &str) -> WbhdfResult<DatasetDescriptor> {
    if !path.starts_with('/') {
        return Err(WbhdfError::InvalidInput(
            "dataset path must start with '/'".to_string(),
        ));
    }

    Ok(DatasetDescriptor {
        path: path.to_string(),
    })
}

/// Resolves a dataset descriptor and verifies that the path is discoverable in a container file.
pub fn resolve_dataset_in_file(container_path: &Path, dataset_path: &str) -> WbhdfResult<DatasetDescriptor> {
    let descriptor = resolve_dataset(dataset_path)?;
    let bytes = fs::read(container_path)?;

    if bytes_contain_marker(&bytes, &descriptor.path)
        || path_components_are_discoverable(&bytes, &descriptor.path)
    {
        Ok(descriptor)
    } else {
        Err(WbhdfError::DatasetPathNotFound(descriptor.path))
    }
}

/// Reads a contiguous f32 window directly from a container file at a known byte offset.
pub fn read_contiguous_f32_window_in_file(
    container_path: &Path,
    byte_offset: u64,
    element_count: usize,
    endianness: Endianness,
) -> WbhdfResult<Vec<f32>> {
    let byte_len = element_count.checked_mul(4).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "contiguous f32 window size overflow: element_count={element_count}"
        ))
    })?;

    let start = usize::try_from(byte_offset).map_err(|_| {
        WbhdfError::InvalidInput(format!(
            "contiguous f32 window offset does not fit usize: offset={byte_offset}"
        ))
    })?;
    let end = start.checked_add(byte_len).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "contiguous f32 window end overflow: offset={byte_offset}, elements={element_count}"
        ))
    })?;

    let bytes = fs::read(container_path)?;
    if end > bytes.len() {
        return Err(WbhdfError::InvalidInput(format!(
            "contiguous f32 window out of bounds: offset={byte_offset}, elements={element_count}, file_size={}",
            bytes.len()
        )));
    }

    decode_f32_slice(&bytes[start..end], endianness)
        .map_err(|err| WbhdfError::InvalidInput(format!("contiguous f32 decode failed: {err}")))
}

/// Reads a contiguous f64 window directly from a container file at a known byte offset.
pub fn read_contiguous_f64_window_in_file(
    container_path: &Path,
    byte_offset: u64,
    element_count: usize,
    endianness: Endianness,
) -> WbhdfResult<Vec<f64>> {
    let byte_len = element_count.checked_mul(8).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "contiguous f64 window size overflow: element_count={element_count}"
        ))
    })?;

    let start = usize::try_from(byte_offset).map_err(|_| {
        WbhdfError::InvalidInput(format!(
            "contiguous f64 window offset does not fit usize: offset={byte_offset}"
        ))
    })?;
    let end = start.checked_add(byte_len).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "contiguous f64 window end overflow: offset={byte_offset}, elements={element_count}"
        ))
    })?;

    let bytes = fs::read(container_path)?;
    if end > bytes.len() {
        return Err(WbhdfError::InvalidInput(format!(
            "contiguous f64 window out of bounds: offset={byte_offset}, elements={element_count}, file_size={}",
            bytes.len()
        )));
    }

    decode_f64_slice(&bytes[start..end], endianness)
        .map_err(|err| WbhdfError::InvalidInput(format!("contiguous f64 decode failed: {err}")))
}

/// Decodes a bounded i16 prefix from a chunk selected by row offset in a chunked HDF5 dataset.
///
/// This helper traverses a v1 chunk-index B-tree at `chunk_index_address`, selects the first chunk
/// whose `row_dimension_index` offset equals `row_offset`, inflates the chunk payload using zlib,
/// and decodes i16 values using the supplied endianness.
pub fn decode_chunked_i16_row_prefix_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_offset: u64,
    max_values: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<i16>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked i16 row-prefix decode requires num_dimensions >= 1".to_string(),
        ));
    }
    if row_dimension_index >= num_dimensions {
        return Err(WbhdfError::InvalidInput(format!(
            "row_dimension_index out of bounds: index={row_dimension_index}, num_dimensions={num_dimensions}"
        )));
    }
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked i16 row-prefix decode requires max_values >= 1".to_string(),
        ));
    }

    let records = read_chunked_storage_records_bounded_in_file(
        container_path,
        chunk_index_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
    )?;

    let Some(record) = records
        .iter()
        .find(|record| record.chunk_offsets[row_dimension_index] == row_offset)
    else {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("row_dim={row_dimension_index}, row_offset={row_offset}")),
            file_offset: chunk_index_address,
            detail: "no chunk record matched requested row offset".to_string(),
        });
    };

    let compressed = read_chunk_payload_in_file(container_path, record.chunk_address, record.chunk_size)?;
    let decompressed = decompress_zlib(&compressed).map_err(|err| match err {
        WbhdfError::UnsupportedFilter(detail) => WbhdfError::FilterFailure {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
            file_offset: record.chunk_address,
            filter: "deflate/zlib".to_string(),
            detail,
        },
        other => other,
    })?;

    let mut values = decode_i16_slice(&decompressed, endianness).map_err(|detail| WbhdfError::InvalidChunk {
        dataset_path: dataset_path.to_string(),
        chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
        file_offset: record.chunk_address,
        detail: format!("chunk i16 decode failed: {detail}"),
    })?;

    if values.len() > max_values {
        values.truncate(max_values);
    }
    Ok(values)
}

/// Decodes a bounded row-major 2D i16 window from a chunked HDF5 dataset.
///
/// This helper currently targets products where each chunk resolves to one logical row of i16
/// values (e.g. VNP13 NDVI/EVI/EVI2). It composes row slices by repeatedly resolving row chunks
/// and extracting `[col_start, col_start + col_count)` from each decoded row.
pub fn decode_chunked_i16_row_major_window_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_start: u64,
    row_count: usize,
    col_start: usize,
    col_count: usize,
    row_width: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<i16>> {
    if row_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked i16 row-major window decode requires row_count >= 1".to_string(),
        ));
    }
    if col_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked i16 row-major window decode requires col_count >= 1".to_string(),
        ));
    }
    if row_width == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked i16 row-major window decode requires row_width >= 1".to_string(),
        ));
    }
    let col_end = col_start.checked_add(col_count).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked i16 row-major window column end overflow: col_start={col_start}, col_count={col_count}"
        ))
    })?;
    if col_end > row_width {
        return Err(WbhdfError::InvalidInput(format!(
            "chunked i16 row-major window exceeds row width: col_start={col_start}, col_count={col_count}, row_width={row_width}"
        )));
    }

    let mut out = Vec::<i16>::with_capacity(row_count.saturating_mul(col_count));
    for row_delta in 0..row_count {
        let row_offset = row_start
            .checked_add(row_delta as u64)
            .ok_or_else(|| WbhdfError::InvalidInput("row offset overflow".to_string()))?;

        let row_values = decode_chunked_i16_row_prefix_in_file(
            container_path,
            dataset_path,
            chunk_index_address,
            num_dimensions,
            row_dimension_index,
            row_offset,
            row_width,
            endianness,
            max_leaf_nodes,
            max_records,
        )?;

        if row_values.len() < col_end {
            return Err(WbhdfError::InvalidChunk {
                dataset_path: dataset_path.to_string(),
                chunk_coordinate: Some(format!("row_dim={row_dimension_index}, row_offset={row_offset}")),
                file_offset: chunk_index_address,
                detail: format!(
                    "decoded row too short for requested window: decoded_len={}, col_end={col_end}",
                    row_values.len()
                ),
            });
        }

        out.extend_from_slice(&row_values[col_start..col_end]);
    }

    Ok(out)
}

/// Decodes a bounded u16 prefix from a chunk selected by row offset in a chunked HDF5 dataset.
pub fn decode_chunked_u16_row_prefix_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_offset: u64,
    max_values: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<u16>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-prefix decode requires num_dimensions >= 1".to_string(),
        ));
    }
    if row_dimension_index >= num_dimensions {
        return Err(WbhdfError::InvalidInput(format!(
            "row_dimension_index out of bounds: index={row_dimension_index}, num_dimensions={num_dimensions}"
        )));
    }
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-prefix decode requires max_values >= 1".to_string(),
        ));
    }

    let records = read_chunked_storage_records_bounded_in_file(
        container_path,
        chunk_index_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
    )?;

    let Some(record) = records
        .iter()
        .find(|record| record.chunk_offsets[row_dimension_index] == row_offset)
    else {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("row_dim={row_dimension_index}, row_offset={row_offset}")),
            file_offset: chunk_index_address,
            detail: "no chunk record matched requested row offset".to_string(),
        });
    };

    let compressed = read_chunk_payload_in_file(container_path, record.chunk_address, record.chunk_size)?;
    let decompressed = decompress_zlib(&compressed).map_err(|err| match err {
        WbhdfError::UnsupportedFilter(detail) => WbhdfError::FilterFailure {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
            file_offset: record.chunk_address,
            filter: "deflate/zlib".to_string(),
            detail,
        },
        other => other,
    })?;

    let mut values = decode_u16_slice(&decompressed, endianness).map_err(|detail| WbhdfError::InvalidChunk {
        dataset_path: dataset_path.to_string(),
        chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
        file_offset: record.chunk_address,
        detail: format!("chunk u16 decode failed: {detail}"),
    })?;

    if values.len() > max_values {
        values.truncate(max_values);
    }
    Ok(values)
}

/// Decodes a bounded row-major 2D u16 window from a chunked HDF5 dataset.
pub fn decode_chunked_u16_row_major_window_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_start: u64,
    row_count: usize,
    col_start: usize,
    col_count: usize,
    row_width: usize,
    chunk_row_height: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<u16>> {
    if row_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-major window decode requires row_count >= 1".to_string(),
        ));
    }
    if col_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-major window decode requires col_count >= 1".to_string(),
        ));
    }
    if row_width == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-major window decode requires row_width >= 1".to_string(),
        ));
    }
    if chunk_row_height == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u16 row-major window decode requires chunk_row_height >= 1".to_string(),
        ));
    }
    let col_end = col_start.checked_add(col_count).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked u16 row-major window column end overflow: col_start={col_start}, col_count={col_count}"
        ))
    })?;
    if col_end > row_width {
        return Err(WbhdfError::InvalidInput(format!(
            "chunked u16 row-major window exceeds row width: col_start={col_start}, col_count={col_count}, row_width={row_width}"
        )));
    }

    let chunk_values = row_width.checked_mul(chunk_row_height).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked u16 row-major window chunk value count overflow: row_width={row_width}, chunk_row_height={chunk_row_height}"
        ))
    })?;

    let mut chunk_cache = BTreeMap::<u64, Vec<u16>>::new();
    let mut out = Vec::<u16>::with_capacity(row_count.saturating_mul(col_count));
    for row_delta in 0..row_count {
        let absolute_row = row_start
            .checked_add(row_delta as u64)
            .ok_or_else(|| WbhdfError::InvalidInput("row offset overflow".to_string()))?;

        let chunk_row_base = (absolute_row / chunk_row_height as u64) * chunk_row_height as u64;
        let chunk_rows = if let Some(values) = chunk_cache.get(&chunk_row_base) {
            values
        } else {
            let values = decode_chunked_u16_row_prefix_in_file(
                container_path,
                dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                chunk_row_base,
                chunk_values,
                endianness,
                max_leaf_nodes,
                max_records,
            )?;
            chunk_cache.insert(chunk_row_base, values);
            chunk_cache
                .get(&chunk_row_base)
                .expect("inserted chunk row should be retrievable")
        };

        let local_row = (absolute_row - chunk_row_base) as usize;
        let row_value_start = local_row.checked_mul(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked u16 row-major window row start overflow: local_row={local_row}, row_width={row_width}"
            ))
        })?;
        let row_value_end = row_value_start.checked_add(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked u16 row-major window row end overflow: row_start={row_value_start}, row_width={row_width}"
            ))
        })?;

        if chunk_rows.len() < row_value_end {
            return Err(WbhdfError::InvalidChunk {
                dataset_path: dataset_path.to_string(),
                chunk_coordinate: Some(format!(
                    "row_dim={row_dimension_index}, chunk_row_base={chunk_row_base}, absolute_row={absolute_row}"
                )),
                file_offset: chunk_index_address,
                detail: format!(
                    "decoded chunk too short for requested row: decoded_len={}, required_end={row_value_end}",
                    chunk_rows.len()
                ),
            });
        }

        let row_slice = &chunk_rows[row_value_start..row_value_end];
        out.extend_from_slice(&row_slice[col_start..col_end]);
    }

    Ok(out)
}

/// Decodes a bounded u8 prefix from a chunk selected by row offset in a chunked HDF5 dataset.
pub fn decode_chunked_u8_row_prefix_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_offset: u64,
    max_values: usize,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<u8>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-prefix decode requires num_dimensions >= 1".to_string(),
        ));
    }
    if row_dimension_index >= num_dimensions {
        return Err(WbhdfError::InvalidInput(format!(
            "row_dimension_index out of bounds: index={row_dimension_index}, num_dimensions={num_dimensions}"
        )));
    }
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-prefix decode requires max_values >= 1".to_string(),
        ));
    }

    let records = read_chunked_storage_records_bounded_in_file(
        container_path,
        chunk_index_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
    )?;

    let Some(record) = records
        .iter()
        .find(|record| record.chunk_offsets[row_dimension_index] == row_offset)
    else {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("row_dim={row_dimension_index}, row_offset={row_offset}")),
            file_offset: chunk_index_address,
            detail: "no chunk record matched requested row offset".to_string(),
        });
    };

    let compressed = read_chunk_payload_in_file(container_path, record.chunk_address, record.chunk_size)?;
    let mut decompressed = decompress_zlib(&compressed).map_err(|err| match err {
        WbhdfError::UnsupportedFilter(detail) => WbhdfError::FilterFailure {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
            file_offset: record.chunk_address,
            filter: "deflate/zlib".to_string(),
            detail,
        },
        other => other,
    })?;

    if decompressed.len() > max_values {
        decompressed.truncate(max_values);
    }
    Ok(decompressed)
}

/// Decodes a bounded row-major 2D u8 window from a chunked HDF5 dataset.
pub fn decode_chunked_u8_row_major_window_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_start: u64,
    row_count: usize,
    col_start: usize,
    col_count: usize,
    row_width: usize,
    chunk_row_height: usize,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<u8>> {
    if row_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-major window decode requires row_count >= 1".to_string(),
        ));
    }
    if col_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-major window decode requires col_count >= 1".to_string(),
        ));
    }
    if row_width == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-major window decode requires row_width >= 1".to_string(),
        ));
    }
    if chunk_row_height == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked u8 row-major window decode requires chunk_row_height >= 1".to_string(),
        ));
    }
    let col_end = col_start.checked_add(col_count).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked u8 row-major window column end overflow: col_start={col_start}, col_count={col_count}"
        ))
    })?;
    if col_end > row_width {
        return Err(WbhdfError::InvalidInput(format!(
            "chunked u8 row-major window exceeds row width: col_start={col_start}, col_count={col_count}, row_width={row_width}"
        )));
    }

    let chunk_values = row_width.checked_mul(chunk_row_height).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked u8 row-major window chunk value count overflow: row_width={row_width}, chunk_row_height={chunk_row_height}"
        ))
    })?;

    let mut chunk_cache = BTreeMap::<u64, Vec<u8>>::new();
    let mut out = Vec::<u8>::with_capacity(row_count.saturating_mul(col_count));
    for row_delta in 0..row_count {
        let absolute_row = row_start
            .checked_add(row_delta as u64)
            .ok_or_else(|| WbhdfError::InvalidInput("row offset overflow".to_string()))?;

        let chunk_row_base = (absolute_row / chunk_row_height as u64) * chunk_row_height as u64;
        let chunk_rows = if let Some(values) = chunk_cache.get(&chunk_row_base) {
            values
        } else {
            let values = decode_chunked_u8_row_prefix_in_file(
                container_path,
                dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                chunk_row_base,
                chunk_values,
                max_leaf_nodes,
                max_records,
            )?;
            chunk_cache.insert(chunk_row_base, values);
            chunk_cache
                .get(&chunk_row_base)
                .expect("inserted chunk row should be retrievable")
        };

        let local_row = (absolute_row - chunk_row_base) as usize;
        let row_value_start = local_row.checked_mul(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked u8 row-major window row start overflow: local_row={local_row}, row_width={row_width}"
            ))
        })?;
        let row_value_end = row_value_start.checked_add(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked u8 row-major window row end overflow: row_start={row_value_start}, row_width={row_width}"
            ))
        })?;

        if chunk_rows.len() < row_value_end {
            return Err(WbhdfError::InvalidChunk {
                dataset_path: dataset_path.to_string(),
                chunk_coordinate: Some(format!(
                    "row_dim={row_dimension_index}, chunk_row_base={chunk_row_base}, absolute_row={absolute_row}"
                )),
                file_offset: chunk_index_address,
                detail: format!(
                    "decoded chunk too short for requested row: decoded_len={}, required_end={row_value_end}",
                    chunk_rows.len()
                ),
            });
        }

        let row_slice = &chunk_rows[row_value_start..row_value_end];
        out.extend_from_slice(&row_slice[col_start..col_end]);
    }

    Ok(out)
}

/// Decodes a bounded f32 prefix from a chunk selected by row offset in a chunked HDF5 dataset.
pub fn decode_chunked_f32_row_prefix_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_offset: u64,
    max_values: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<f32>> {
    if num_dimensions == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-prefix decode requires num_dimensions >= 1".to_string(),
        ));
    }
    if row_dimension_index >= num_dimensions {
        return Err(WbhdfError::InvalidInput(format!(
            "row_dimension_index out of bounds: index={row_dimension_index}, num_dimensions={num_dimensions}"
        )));
    }
    if max_values == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-prefix decode requires max_values >= 1".to_string(),
        ));
    }

    let records = read_chunked_storage_records_bounded_in_file(
        container_path,
        chunk_index_address,
        num_dimensions,
        max_leaf_nodes,
        max_records,
    )?;

    let Some(record) = records
        .iter()
        .find(|record| record.chunk_offsets[row_dimension_index] == row_offset)
    else {
        return Err(WbhdfError::InvalidChunk {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("row_dim={row_dimension_index}, row_offset={row_offset}")),
            file_offset: chunk_index_address,
            detail: "no chunk record matched requested row offset".to_string(),
        });
    };

    let compressed = read_chunk_payload_in_file(container_path, record.chunk_address, record.chunk_size)?;
    let decompressed = decompress_zlib(&compressed).map_err(|err| match err {
        WbhdfError::UnsupportedFilter(detail) => WbhdfError::FilterFailure {
            dataset_path: dataset_path.to_string(),
            chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
            file_offset: record.chunk_address,
            filter: "deflate/zlib".to_string(),
            detail,
        },
        other => other,
    })?;

    let mut values = decode_f32_slice(&decompressed, endianness).map_err(|detail| WbhdfError::InvalidChunk {
        dataset_path: dataset_path.to_string(),
        chunk_coordinate: Some(format!("{:?}", record.chunk_offsets)),
        file_offset: record.chunk_address,
        detail: format!("chunk f32 decode failed: {detail}"),
    })?;

    if values.len() > max_values {
        values.truncate(max_values);
    }
    Ok(values)
}

/// Decodes a bounded row-major 2D f32 window from a chunked HDF5 dataset.
pub fn decode_chunked_f32_row_major_window_in_file(
    container_path: &Path,
    dataset_path: &str,
    chunk_index_address: u64,
    num_dimensions: usize,
    row_dimension_index: usize,
    row_start: u64,
    row_count: usize,
    col_start: usize,
    col_count: usize,
    row_width: usize,
    chunk_row_height: usize,
    endianness: Endianness,
    max_leaf_nodes: usize,
    max_records: usize,
) -> WbhdfResult<Vec<f32>> {
    if row_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-major window decode requires row_count >= 1".to_string(),
        ));
    }
    if col_count == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-major window decode requires col_count >= 1".to_string(),
        ));
    }
    if row_width == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-major window decode requires row_width >= 1".to_string(),
        ));
    }
    if chunk_row_height == 0 {
        return Err(WbhdfError::InvalidInput(
            "chunked f32 row-major window decode requires chunk_row_height >= 1".to_string(),
        ));
    }
    let col_end = col_start.checked_add(col_count).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked f32 row-major window column end overflow: col_start={col_start}, col_count={col_count}"
        ))
    })?;
    if col_end > row_width {
        return Err(WbhdfError::InvalidInput(format!(
            "chunked f32 row-major window exceeds row width: col_start={col_start}, col_count={col_count}, row_width={row_width}"
        )));
    }

    let chunk_values = row_width.checked_mul(chunk_row_height).ok_or_else(|| {
        WbhdfError::InvalidInput(format!(
            "chunked f32 row-major window chunk value count overflow: row_width={row_width}, chunk_row_height={chunk_row_height}"
        ))
    })?;

    let mut chunk_cache = BTreeMap::<u64, Vec<f32>>::new();
    let mut out = Vec::<f32>::with_capacity(row_count.saturating_mul(col_count));
    for row_delta in 0..row_count {
        let absolute_row = row_start
            .checked_add(row_delta as u64)
            .ok_or_else(|| WbhdfError::InvalidInput("row offset overflow".to_string()))?;

        let chunk_row_base = (absolute_row / chunk_row_height as u64) * chunk_row_height as u64;
        let chunk_rows = if let Some(values) = chunk_cache.get(&chunk_row_base) {
            values
        } else {
            let values = decode_chunked_f32_row_prefix_in_file(
                container_path,
                dataset_path,
                chunk_index_address,
                num_dimensions,
                row_dimension_index,
                chunk_row_base,
                chunk_values,
                endianness,
                max_leaf_nodes,
                max_records,
            )?;
            chunk_cache.insert(chunk_row_base, values);
            chunk_cache
                .get(&chunk_row_base)
                .expect("inserted chunk row should be retrievable")
        };

        let local_row = (absolute_row - chunk_row_base) as usize;
        let row_value_start = local_row.checked_mul(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked f32 row-major window row start overflow: local_row={local_row}, row_width={row_width}"
            ))
        })?;
        let row_value_end = row_value_start.checked_add(row_width).ok_or_else(|| {
            WbhdfError::InvalidInput(format!(
                "chunked f32 row-major window row end overflow: row_start={row_value_start}, row_width={row_width}"
            ))
        })?;

        if chunk_rows.len() < row_value_end {
            return Err(WbhdfError::InvalidChunk {
                dataset_path: dataset_path.to_string(),
                chunk_coordinate: Some(format!(
                    "row_dim={row_dimension_index}, chunk_row_base={chunk_row_base}, absolute_row={absolute_row}"
                )),
                file_offset: chunk_index_address,
                detail: format!(
                    "decoded chunk too short for requested row: decoded_len={}, required_end={row_value_end}",
                    chunk_rows.len()
                ),
            });
        }

        let row_slice = &chunk_rows[row_value_start..row_value_end];
        out.extend_from_slice(&row_slice[col_start..col_end]);
    }

    Ok(out)
}

fn bytes_contain_marker(bytes: &[u8], marker: &str) -> bool {
    bytes
        .windows(marker.len())
        .any(|window| window == marker.as_bytes())
}

fn path_components_are_discoverable(bytes: &[u8], dataset_path: &str) -> bool {
    dataset_path
        .split('/')
        .filter(|component| !component.is_empty())
        .all(|component| bytes_contain_marker(bytes, component))
}

impl DatasetChunkLocator {
    /// Constructs a locator from known key -> chunk address mappings.
    pub fn with_known_addresses(
        dataset_path: &str,
        key_to_address: &[(u64, u64)],
    ) -> WbhdfResult<Self> {
        let descriptor = resolve_dataset(dataset_path)?;
        let mut chunk_index = ChunkIndex::new(&descriptor.path);

        for (key, address) in key_to_address {
            chunk_index.insert(*key, *address);
        }

        Ok(Self {
            descriptor,
            chunk_index,
        })
    }

    /// Locates the chunk address for the supplied dataset coordinates.
    pub fn locate_chunk_address(&self, coords: &[u64]) -> WbhdfResult<u64> {
        lookup_chunk_address(&self.chunk_index, &self.descriptor.path, coords)
    }

    /// Returns the dataset path bound to this locator.
    pub fn dataset_path(&self) -> &str {
        &self.descriptor.path
    }
}

/// Applies deterministic fill-value to nodata mapping for f32 payloads.
pub fn apply_fill_value_mapping_f32(
    values: &[f32],
    fill_value: Option<f32>,
    nodata_value: f32,
) -> FillMappedF32 {
    let mut mapped = Vec::with_capacity(values.len());
    let mut valid_count = 0usize;
    let mut nodata_count = 0usize;

    match fill_value {
        Some(fill) => {
            let fill_bits = fill.to_bits();
            for value in values {
                if value.to_bits() == fill_bits {
                    mapped.push(nodata_value);
                    nodata_count += 1;
                } else {
                    mapped.push(*value);
                    valid_count += 1;
                }
            }
        }
        None => {
            mapped.extend_from_slice(values);
            valid_count = values.len();
        }
    }

    FillMappedF32 {
        values: mapped,
        nodata_value,
        valid_count,
        nodata_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_fill_value_mapping_f32, read_contiguous_f32_window_in_file,
        read_contiguous_f64_window_in_file, resolve_dataset_in_file, DatasetChunkLocator,
    };
    use crate::datatypes::Endianness;
    use std::fs;

    #[test]
    fn locator_returns_known_addresses() {
        let locator = DatasetChunkLocator::with_known_addresses(
            "/GEDI04_B/BEAM0000/rh100",
            &[(0, 4000), (1, 4500), (2, 5000)],
        )
        .expect("locator should construct");

        assert_eq!(locator.dataset_path(), "/GEDI04_B/BEAM0000/rh100");
        assert_eq!(locator.locate_chunk_address(&[0]).unwrap(), 4000);
        assert_eq!(locator.locate_chunk_address(&[2]).unwrap(), 5000);
    }

    #[test]
    fn locator_reports_missing_known_key() {
        let locator = DatasetChunkLocator::with_known_addresses(
            "/GEDI04_B/BEAM0000/rh100",
            &[(3, 7000)],
        )
        .expect("locator should construct");

        let err = locator.locate_chunk_address(&[1]).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("chunk address not found"));
    }

    #[test]
    fn resolve_dataset_in_file_finds_dataset_marker() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-dataset-marker-test.h5");
        fs::write(&file_path, b"prefix /gt1l/heights/h_canopy suffix").unwrap();

        let descriptor = resolve_dataset_in_file(&file_path, "/gt1l/heights/h_canopy").unwrap();
        assert_eq!(descriptor.path, "/gt1l/heights/h_canopy");

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn resolve_dataset_in_file_reports_missing_marker() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-dataset-missing-test.h5");
        fs::write(&file_path, b"prefix /gt1l/heights/other suffix").unwrap();

        let err = resolve_dataset_in_file(&file_path, "/gt1l/heights/h_canopy").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("dataset path not found"));

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn resolve_dataset_in_file_accepts_split_path_markers() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-dataset-split-test.h5");
        fs::write(
            &file_path,
            b"gt1l something land_segments something canopy something h_canopy",
        )
        .unwrap();

        let descriptor = resolve_dataset_in_file(&file_path, "/gt1l/land_segments/canopy/h_canopy")
            .unwrap();
        assert_eq!(descriptor.path, "/gt1l/land_segments/canopy/h_canopy");

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn fill_mapping_replaces_fill_values_and_reports_counts() {
        let mapped = apply_fill_value_mapping_f32(
            &[1.0, f32::MAX, 2.0, f32::MAX],
            Some(f32::MAX),
            -9999.0,
        );

        assert_eq!(mapped.values, vec![1.0, -9999.0, 2.0, -9999.0]);
        assert_eq!(mapped.valid_count, 2);
        assert_eq!(mapped.nodata_count, 2);
    }

    #[test]
    fn fill_mapping_without_fill_definition_keeps_values_valid() {
        let mapped = apply_fill_value_mapping_f32(&[1.0, 2.0, 3.0], None, -9999.0);

        assert_eq!(mapped.values, vec![1.0, 2.0, 3.0]);
        assert_eq!(mapped.valid_count, 3);
        assert_eq!(mapped.nodata_count, 0);
    }

    #[test]
    fn read_contiguous_f32_window_decodes_little_endian_values() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-contiguous-f32-window-test.bin");
        fs::write(
            &file_path,
            [
                0u8, 0, 128, 63, // 1.0
                0, 0, 0, 64, // 2.0
                0, 0, 64, 64, // 3.0
            ],
        )
        .unwrap();

        let values =
            read_contiguous_f32_window_in_file(&file_path, 4, 2, Endianness::Little).unwrap();
        assert_eq!(values, vec![2.0, 3.0]);

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn read_contiguous_f32_window_reports_bounds_errors() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-contiguous-f32-window-bounds-test.bin");
        fs::write(&file_path, [0u8, 0, 128, 63]).unwrap();

        let err =
            read_contiguous_f32_window_in_file(&file_path, 4, 1, Endianness::Little).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("out of bounds"));

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn read_contiguous_f64_window_decodes_little_endian_values() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-contiguous-f64-window-test.bin");
        fs::write(
            &file_path,
            [
                0u8, 0, 0, 0, 0, 0, 240, 63, // 1.0
                0, 0, 0, 0, 0, 0, 0, 64, // 2.0
                0, 0, 0, 0, 0, 0, 8, 64, // 3.0
            ],
        )
        .unwrap();

        let values =
            read_contiguous_f64_window_in_file(&file_path, 8, 2, Endianness::Little).unwrap();
        assert_eq!(values, vec![2.0, 3.0]);

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn read_contiguous_f64_window_reports_bounds_errors() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("wbhdf-contiguous-f64-window-bounds-test.bin");
        fs::write(&file_path, [0u8, 0, 0, 0, 0, 0, 240, 63]).unwrap();

        let err =
            read_contiguous_f64_window_in_file(&file_path, 8, 1, Endianness::Little).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("out of bounds"));

        let _ = fs::remove_file(file_path);
    }
}
