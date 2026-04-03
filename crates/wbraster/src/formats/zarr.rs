//! Zarr format (`.zarr`) support (v2, filesystem store).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};

/// Read a Zarr raster from a `.zarr` directory or from a `.zarray` file.
///
/// When the target directory is a multi-scale group (OME-NGFF or plain
/// numeric pyramid), the full-resolution level (`0`) is opened by default.
/// To read a different level, point `path` directly at the sub-array
/// directory (e.g. `"store.zarr/1"`).
pub fn read(path: &str) -> Result<Raster> {
    let dir = resolve_zarr_dir(path)?;

    // ── v3 group? ─────────────────────────────────────────────────────────
    if crate::formats::zarr_v3::is_v3_group(&dir) {
        let levels = crate::formats::zarr_v3::discover_multiscale_levels_v3(&dir);
        if levels.is_empty() {
            return Err(RasterError::CorruptData(
                "zarr v3 group contains no recognisable array sub-directories".into(),
            ));
        }
        let level_dir = select_level(&levels);
        return crate::formats::zarr_v3::read_from_dir(level_dir);
    }

    // ── v3 array? ─────────────────────────────────────────────────────────
    if crate::formats::zarr_v3::is_v3_store(&dir) {
        return crate::formats::zarr_v3::read_from_dir(&dir);
    }

    // ── v2 group? ─────────────────────────────────────────────────────────
    if is_v2_group(&dir) {
        let levels = discover_multiscale_levels_v2(&dir);
        if levels.is_empty() {
            return Err(RasterError::CorruptData(
                "zarr v2 group contains no recognisable array sub-directories".into(),
            ));
        }
        let level_dir = select_level(&levels);
        return read_from_dir(level_dir);
    }

    // ── v2 array (default) ────────────────────────────────────────────────
    read_from_dir(&dir)
}

/// Write a raster to a Zarr v2 directory.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    let dir = resolve_target_dir(path);
    if requested_zarr_version(raster) == 3 {
        return crate::formats::zarr_v3::write_to_dir(raster, &dir);
    }
    write_to_dir(raster, &dir)
}

fn requested_zarr_version(raster: &Raster) -> u8 {
    raster
        .metadata
        .iter()
        .find(|(k, _)| k == "zarr_version")
        .and_then(|(_, v)| v.parse::<u8>().ok())
        .unwrap_or(2)
}

    fn metadata_usize(raster: &Raster, key: &str) -> Option<usize> {
        raster
        .metadata
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<usize>().ok())
    }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZarrArrayMeta {
    zarr_format: u8,
    shape: Vec<usize>,
    chunks: Vec<usize>,
    dtype: String,
    compressor: Option<CompressorSpec>,
    fill_value: Option<Value>,
    order: String,
    filters: Option<Value>,
    dimension_separator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressorSpec {
    id: String,
    level: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationMode {
    Strict,
    Lenient,
}

/// Returns `true` if `dir` is a v2 Zarr group (has `.zgroup` file).
fn is_v2_group(dir: &Path) -> bool {
    dir.join(".zgroup").exists()
}

/// Discover the ordered list of array sub-paths for a v2 multi-scale group.
///
/// Tries the OME-NGFF `.zattrs` `multiscales[0].datasets[].path` convention
/// first; falls back to scanning consecutive numeric sub-directories.
pub(crate) fn discover_multiscale_levels_v2(dir: &Path) -> Vec<PathBuf> {
    // Try OME-NGFF .zattrs: multiscales[0].datasets[].path
    if let Ok(s) = fs::read_to_string(dir.join(".zattrs")) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            if let Some(datasets) = v
                .get("multiscales")
                .and_then(Value::as_array)
                .and_then(|ms| ms.first())
                .and_then(|m| m.get("datasets"))
                .and_then(Value::as_array)
            {
                let paths: Vec<_> = datasets
                    .iter()
                    .filter_map(|d| d.get("path").and_then(Value::as_str))
                    .map(|p| dir.join(p))
                    .filter(|p| p.join(".zarray").exists())
                    .collect();
                if !paths.is_empty() {
                    return paths;
                }
            }
        }
    }

    // Fallback: scan consecutive numeric sub-dirs that look like v2 arrays.
    let mut levels = Vec::new();
    for i in 0usize.. {
        let candidate = dir.join(i.to_string());
        if candidate.join(".zarray").exists() {
            levels.push(candidate);
        } else {
            break;
        }
    }
    levels
}

/// Select which level to read from a discovered level list.
///
/// Honours an optional `zarr_level` raster-metadata hint (not applicable on
/// read, so we always return index 0 here — the full-resolution level).
/// A caller that wants a coarser level can point the path directly at the
/// sub-array directory instead of the group root.
fn select_level(levels: &[PathBuf]) -> &Path {
    // Always return the first (finest / full-resolution) level.
    levels[0].as_path()
}

fn resolve_zarr_dir(path: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    if p.is_dir() {
        return Ok(p.to_path_buf());
    }
    if p.is_file() {
        if p.file_name().and_then(|n| n.to_str()) == Some(".zarray") {
            return p
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| RasterError::Other("invalid .zarray path".into()));
        }
        return Err(RasterError::UnknownFormat(path.to_owned()));
    }
    Err(RasterError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("zarr path not found: {path}"),
    )))
}

fn resolve_target_dir(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.file_name().and_then(|n| n.to_str()) == Some(".zarray") {
        p.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    } else {
        p.to_path_buf()
    }
}

fn read_from_dir(dir: &Path) -> Result<Raster> {
    let zarray_path = dir.join(".zarray");
    let meta_text = fs::read_to_string(&zarray_path)?;
    let meta: ZarrArrayMeta = serde_json::from_str(&meta_text)
        .map_err(|e| RasterError::CorruptData(format!("invalid .zarray JSON: {e}")))?;

    if meta.zarr_format != 2 {
        return Err(RasterError::UnsupportedDataType(format!(
            "zarr_format={} (only v2 supported)",
            meta.zarr_format
        )));
    }
    if meta.shape.len() != meta.chunks.len() {
        return Err(RasterError::CorruptData(format!(
            "shape/chunks rank mismatch: {} vs {}",
            meta.shape.len(),
            meta.chunks.len()
        )));
    }
    if meta.shape.len() != 2 && meta.shape.len() != 3 {
        return Err(RasterError::UnsupportedDataType(
            "only 2D or 3D [band,y,x] Zarr arrays are supported".into(),
        ));
    }
    if meta.order != "C" {
        return Err(RasterError::UnsupportedDataType(
            "only C-order Zarr arrays are supported".into(),
        ));
    }

    let (bands, rows, cols, chunk_bands, chunk_rows, chunk_cols) = if meta.shape.len() == 3 {
        (
            meta.shape[0],
            meta.shape[1],
            meta.shape[2],
            meta.chunks[0].max(1),
            meta.chunks[1].max(1),
            meta.chunks[2].max(1),
        )
    } else {
        (1, meta.shape[0], meta.shape[1], 1, meta.chunks[0].max(1), meta.chunks[1].max(1))
    };
    let (dtype, endian) = parse_zarr_dtype(&meta.dtype)?;
    let bpp = dtype.size_bytes();

    let attrs = read_zattrs(dir)?;
    let validation_mode = parse_validation_mode_from_attrs(&attrs);
    let transform = parse_transform_from_attrs(&attrs, validation_mode)?;
    let x_min = attrs
        .get("x_min")
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[0]))
        .unwrap_or(0.0);
    let cell_size = attrs
        .get("cell_size_x")
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[1].abs()))
        .unwrap_or(1.0);
    let cell_size_y = attrs
        .get("cell_size_y")
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[5].abs()));
    let y_min = attrs
        .get("y_min")
        .and_then(Value::as_f64)
        .or_else(|| {
            transform.map(|t| {
                let y_top = t[3];
                let dy = t[5];
                if dy == 0.0 {
                    y_top - cell_size_y.unwrap_or(cell_size) * rows as f64
                } else {
                    y_top + dy * rows as f64
                }
            })
        })
        .unwrap_or(0.0);

    validate_georef_consistency(
        &attrs,
        transform,
        rows,
        cell_size,
        cell_size_y,
        validation_mode,
    )?;
    let nodata = attrs.get("nodata").and_then(Value::as_f64)
        .or_else(|| attrs.get("_FillValue").and_then(Value::as_f64))
        .or_else(|| attrs.get("missing_value").and_then(Value::as_f64))
        .unwrap_or_else(|| fill_value_to_f64(meta.fill_value.as_ref()).unwrap_or(-9999.0));

    let attrs_obj = attrs.as_object();
    let crs = crate::crs_info::CrsInfo {
        epsg: attrs
            .get("crs_epsg")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .or_else(|| attrs.get("epsg").and_then(Value::as_u64).map(|v| v as u32))
            .or_else(|| parse_epsg_from_crs_value(attrs.get("crs")))
            .or_else(|| attrs_obj.and_then(parse_epsg_from_grid_mapping_attrs)),
        wkt: attrs
            .get("crs_wkt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                attrs
                    .get("spatial_ref")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    })
                    .or_else(|| attrs_obj.and_then(parse_wkt_from_grid_mapping_attrs)),
        proj4: attrs
            .get("crs_proj4")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
                    .or_else(|| attrs.get("proj4").and_then(Value::as_str).map(ToOwned::to_owned))
                    .or_else(|| attrs_obj.and_then(parse_proj4_from_grid_mapping_attrs)),
    };

    let sep = meta.dimension_separator.as_deref().unwrap_or(".");
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);

    let mut data = vec![nodata; bands * rows * cols];
    let band_plane_len = rows * cols;
    let cb_block_len = chunk_bands * band_plane_len;
    data.par_chunks_mut(cb_block_len)
        .enumerate()
        .try_for_each(|(cb, data_cb)| -> Result<()> {
            for cr in 0..n_chunk_rows {
                for cc in 0..n_chunk_cols {
                    let key = if bands > 1 {
                        chunk_key(&[cb, cr, cc], sep)
                    } else {
                        chunk_key(&[cr, cc], sep)
                    };
                    let chunk_path = dir.join(key);

                    let this_bands = (bands - cb * chunk_bands).min(chunk_bands);
                    let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
                    let this_cols = (cols - cc * chunk_cols).min(chunk_cols);
                    let expected_bytes = this_bands * this_rows * this_cols * bpp;

                    let raw = if chunk_path.exists() {
                        let compressed = fs::read(&chunk_path)?;
                        decompress_bytes(&meta.compressor, &compressed)?
                    } else {
                        let fv = fill_value_to_f64(meta.fill_value.as_ref()).unwrap_or(nodata);
                        encode_fill_chunk(this_bands * this_rows * this_cols, dtype, endian, fv)
                    };

                    if raw.len() != expected_bytes {
                        return Err(RasterError::CorruptData(format!(
                            "chunk {cb},{cr},{cc} size mismatch: expected {expected_bytes}, got {}",
                            raw.len()
                        )));
                    }

                    for bb in 0..this_bands {
                        for rr in 0..this_rows {
                            for cc2 in 0..this_cols {
                                let i_chunk = bb * this_rows * this_cols + rr * this_cols + cc2;
                                let src = &raw[i_chunk * bpp..(i_chunk + 1) * bpp];
                                let v = decode_sample(src, dtype, endian)?;
                                let row = cr * chunk_rows + rr;
                                let col = cc * chunk_cols + cc2;
                                data_cb[bb * band_plane_len + row * cols + col] = v;
                            }
                        }
                    }
                }
            }
            Ok(())
        })?;

    let cfg = RasterConfig {
        cols,
        rows,
        bands,
        x_min,
        y_min,
        cell_size,
        cell_size_y,
        nodata,
        data_type: dtype,
        crs: crs,        metadata: vec![
            ("zarr_version".into(), "2".into()),
            ("zarr_dimension_separator".into(), sep.to_owned()),
        ],
    };
    Raster::from_data(cfg, data)
}

fn write_to_dir(raster: &Raster, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;

    let bands = raster.bands;
    let rows = raster.rows;
    let cols = raster.cols;
    let chunk_bands = metadata_usize(raster, "zarr_chunk_bands")
        .unwrap_or(1)
        .clamp(1, bands.max(1));
    let chunk_rows = metadata_usize(raster, "zarr_chunk_rows")
        .unwrap_or_else(|| rows.clamp(1, 256))
        .clamp(1, rows.max(1));
    let chunk_cols = metadata_usize(raster, "zarr_chunk_cols")
        .unwrap_or_else(|| cols.clamp(1, 256))
        .clamp(1, cols.max(1));

    let dtype = data_type_to_zarr_dtype(raster.data_type);
    let dim_sep = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "zarr_dimension_separator" || k == "zarr_chunk_separator")
        .map(|(_, v)| v.as_str())
        .unwrap_or(".");
    let dim_sep = if dim_sep == "/" { "/" } else { "." };

    let compressor = Some(CompressorSpec {
        id: "zlib".to_owned(),
        level: Some(6),
    });

    let meta = ZarrArrayMeta {
        zarr_format: 2,
        shape: if bands > 1 { vec![bands, rows, cols] } else { vec![rows, cols] },
        chunks: if bands > 1 {
            vec![chunk_bands, chunk_rows, chunk_cols]
        } else {
            vec![chunk_rows, chunk_cols]
        },
        dtype: dtype.to_owned(),
        compressor: compressor.clone(),
        fill_value: Some(json!(raster.nodata)),
        order: "C".to_owned(),
        filters: None,
        dimension_separator: Some(dim_sep.to_owned()),
    };

    let zarray_text = serde_json::to_string_pretty(&meta)
        .map_err(|e| RasterError::Other(format!("failed to serialize .zarray: {e}")))?;
    fs::write(dir.join(".zarray"), zarray_text.as_bytes())?;

    let mut zattrs = json!({
        "x_min": raster.x_min,
        "y_min": raster.y_min,
        "cell_size_x": raster.cell_size_x,
        "cell_size_y": raster.cell_size_y,
        "nodata": raster.nodata,
        "data_type": raster.data_type.as_str(),
        "_ARRAY_DIMENSIONS": if bands > 1 { json!(["band", "y", "x"]) } else { json!(["y", "x"]) },
        // GDAL/rioxarray-friendly affine transform tuple:
        // [x_origin, pixel_width, rot_x, y_origin_top, rot_y, pixel_height_neg]
        "transform": [
            raster.x_min,
            raster.cell_size_x,
            0.0,
            raster.y_max(),
            0.0,
            -raster.cell_size_y,
        ],
        "grid_mapping": "spatial_ref",
    });

    if let Some(obj) = zattrs.as_object_mut() {
        if let Some(epsg) = raster.crs.epsg {
            obj.insert("crs_epsg".into(), json!(epsg));
            obj.insert("epsg".into(), json!(epsg));
            obj.insert("crs".into(), json!(format!("EPSG:{epsg}")));
        }
        if let Some(wkt) = &raster.crs.wkt {
            obj.insert("crs_wkt".into(), json!(wkt));
            obj.insert("spatial_ref".into(), json!(wkt));
        }
        if let Some(proj4) = &raster.crs.proj4 {
            obj.insert("crs_proj4".into(), json!(proj4));
            obj.insert("proj4".into(), json!(proj4));
        }
    }
    let zattrs_text = serde_json::to_string_pretty(&zattrs)
        .map_err(|e| RasterError::Other(format!("failed to serialize .zattrs: {e}")))?;
    fs::write(dir.join(".zattrs"), zattrs_text.as_bytes())?;

    let bpp = raster.data_type.size_bytes();
    let sep = dim_sep;
    let n_chunk_bands = bands.div_ceil(chunk_bands);
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);

    for cb in 0..n_chunk_bands {
        for cr in 0..n_chunk_rows {
            for cc in 0..n_chunk_cols {
                let this_bands = (bands - cb * chunk_bands).min(chunk_bands);
                let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
                let this_cols = (cols - cc * chunk_cols).min(chunk_cols);
                let mut raw = Vec::with_capacity(this_bands * this_rows * this_cols * bpp);

                for bb in 0..this_bands {
                    for rr in 0..this_rows {
                        for cc2 in 0..this_cols {
                            let band = cb * chunk_bands + bb;
                            let row = cr * chunk_rows + rr;
                            let col = cc * chunk_cols + cc2;
                            let v = raster
                                .get_raw(band as isize, row as isize, col as isize)
                                .unwrap_or(raster.nodata);
                            encode_sample(v, raster.data_type, &mut raw);
                        }
                    }
                }

                let compressed = compress_bytes(&compressor, &raw)?;
                let key = if bands > 1 {
                    chunk_key(&[cb, cr, cc], sep)
                } else {
                    chunk_key(&[cr, cc], sep)
                };
                let path = dir.join(key);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut f = File::create(path)?;
                f.write_all(&compressed)?;
            }
        }
    }

    Ok(())
}

fn read_zattrs(dir: &Path) -> Result<Value> {
    let p = dir.join(".zattrs");
    if !p.exists() {
        return Ok(json!({}));
    }
    let s = fs::read_to_string(p)?;
    serde_json::from_str(&s).map_err(|e| RasterError::CorruptData(format!("invalid .zattrs JSON: {e}")))
}

#[derive(Debug, Clone, Copy)]
enum Endian {
    Little,
    Big,
    NativeOneByte,
}

fn parse_zarr_dtype(dtype: &str) -> Result<(DataType, Endian)> {
    let mut chars = dtype.chars();
    let first = chars.next().ok_or_else(|| RasterError::CorruptData("empty dtype".into()))?;
    let (endian, rest) = match first {
        '<' => (Endian::Little, chars.as_str()),
        '>' => (Endian::Big, chars.as_str()),
        '|' => (Endian::NativeOneByte, chars.as_str()),
        _ => (Endian::Little, dtype),
    };
    let mut it = rest.chars();
    let kind = it.next().ok_or_else(|| RasterError::CorruptData(format!("invalid dtype '{dtype}'")))?;
    let size: usize = it
        .as_str()
        .parse()
        .map_err(|_| RasterError::CorruptData(format!("invalid dtype size in '{dtype}'")))?;

    let dt = match (kind, size) {
        ('u', 1) => DataType::U8,
        ('i', 1) => DataType::I8,
        ('u', 2) => DataType::U16,
        ('i', 2) => DataType::I16,
        ('u', 4) => DataType::U32,
        ('i', 4) => DataType::I32,
        ('u', 8) => DataType::U64,
        ('i', 8) => DataType::I64,
        ('f', 4) => DataType::F32,
        ('f', 8) => DataType::F64,
        _ => {
            return Err(RasterError::UnsupportedDataType(format!(
                "unsupported zarr dtype '{dtype}'"
            )))
        }
    };
    Ok((dt, endian))
}

fn data_type_to_zarr_dtype(dt: DataType) -> &'static str {
    match dt {
        DataType::U8 => "|u1",
        DataType::I8 => "|i1",
        DataType::U16 => "<u2",
        DataType::I16 => "<i2",
        DataType::U32 => "<u4",
        DataType::I32 => "<i4",
        DataType::U64 => "<u8",
        DataType::I64 => "<i8",
        DataType::F32 => "<f4",
        DataType::F64 => "<f8",
    }
}

fn decode_sample(src: &[u8], dtype: DataType, endian: Endian) -> Result<f64> {
    let v = match dtype {
        DataType::U8 => src[0] as f64,
        DataType::I8 => (src[0] as i8) as f64,
        DataType::U16 => {
            let b: [u8; 2] = src.try_into().map_err(|_| RasterError::CorruptData("bad u16 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u16::from_le_bytes(b) as f64,
                Endian::Big => u16::from_be_bytes(b) as f64,
            }
        }
        DataType::I16 => {
            let b: [u8; 2] = src.try_into().map_err(|_| RasterError::CorruptData("bad i16 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i16::from_le_bytes(b) as f64,
                Endian::Big => i16::from_be_bytes(b) as f64,
            }
        }
        DataType::U32 => {
            let b: [u8; 4] = src.try_into().map_err(|_| RasterError::CorruptData("bad u32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u32::from_le_bytes(b) as f64,
                Endian::Big => u32::from_be_bytes(b) as f64,
            }
        }
        DataType::I32 => {
            let b: [u8; 4] = src.try_into().map_err(|_| RasterError::CorruptData("bad i32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i32::from_le_bytes(b) as f64,
                Endian::Big => i32::from_be_bytes(b) as f64,
            }
        }
        DataType::U64 => {
            let b: [u8; 8] = src.try_into().map_err(|_| RasterError::CorruptData("bad u64 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u64::from_le_bytes(b) as f64,
                Endian::Big => u64::from_be_bytes(b) as f64,
            }
        }
        DataType::I64 => {
            let b: [u8; 8] = src.try_into().map_err(|_| RasterError::CorruptData("bad i64 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i64::from_le_bytes(b) as f64,
                Endian::Big => i64::from_be_bytes(b) as f64,
            }
        }
        DataType::F32 => {
            let b: [u8; 4] = src.try_into().map_err(|_| RasterError::CorruptData("bad f32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => f32::from_le_bytes(b) as f64,
                Endian::Big => f32::from_be_bytes(b) as f64,
            }
        }
        DataType::F64 => {
            let b: [u8; 8] = src.try_into().map_err(|_| RasterError::CorruptData("bad f64 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => f64::from_le_bytes(b),
                Endian::Big => f64::from_be_bytes(b),
            }
        }
    };
    Ok(v)
}

fn encode_sample(v: f64, dtype: DataType, out: &mut Vec<u8>) {
    match dtype {
        DataType::U8 => out.push(v as u8),
        DataType::I8 => out.push((v as i8) as u8),
        DataType::U16 => out.extend_from_slice(&(v as u16).to_le_bytes()),
        DataType::I16 => out.extend_from_slice(&(v as i16).to_le_bytes()),
        DataType::U32 => out.extend_from_slice(&(v as u32).to_le_bytes()),
        DataType::I32 => out.extend_from_slice(&(v as i32).to_le_bytes()),
        DataType::U64 => out.extend_from_slice(&(v as u64).to_le_bytes()),
        DataType::I64 => out.extend_from_slice(&(v as i64).to_le_bytes()),
        DataType::F32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
        DataType::F64 => out.extend_from_slice(&v.to_le_bytes()),
    }
}

fn encode_fill_chunk(n: usize, dtype: DataType, _endian: Endian, fill: f64) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * dtype.size_bytes());
    for _ in 0..n {
        encode_sample(fill, dtype, &mut out);
    }
    out
}

fn fill_value_to_f64(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Null => None,
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            if s.eq_ignore_ascii_case("nan") {
                Some(f64::NAN)
            } else {
                s.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

fn parse_transform_tuple(v: Option<&Value>) -> Option<[f64; 6]> {
    let arr = v?.as_array()?;
    if arr.len() < 6 {
        return None;
    }
    let mut out = [0.0f64; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = arr[i].as_f64()?;
    }
    Some(out)
}

fn parse_transform_from_attrs_strict(attrs: &Value) -> Result<Option<[f64; 6]>> {
    let Some(obj) = attrs.as_object() else {
        return Ok(None);
    };

    let transform = if let Some(v) = obj.get("transform") {
        Some(parse_transform_tuple(Some(v)).ok_or_else(|| {
            RasterError::CorruptData("invalid geospatial metadata: 'transform' must contain at least 6 numeric values".into())
        })?)
    } else {
        None
    };

    let geotransform = if let Some(v) = obj.get("GeoTransform") {
        Some(parse_geotransform_string(Some(v)).ok_or_else(|| {
            RasterError::CorruptData("invalid geospatial metadata: 'GeoTransform' must contain 6 numeric values".into())
        })?)
    } else {
        None
    };

    match (transform, geotransform) {
        (Some(a), Some(b)) => {
            if !same_transform(&a, &b) {
                return Err(RasterError::CorruptData(
                    "conflicting geospatial metadata: 'transform' and 'GeoTransform' disagree".into(),
                ));
            }
            Ok(Some(a))
        }
        (Some(a), None) => Ok(Some(a)),
        (None, Some(b)) => Ok(Some(b)),
        (None, None) => Ok(None),
    }
}

fn parse_transform_from_attrs_lenient(attrs: &Value) -> Option<[f64; 6]> {
    let Some(obj) = attrs.as_object() else {
        return None;
    };
    let transform = obj
        .get("transform")
        .and_then(|v| parse_transform_tuple(Some(v)));
    let geotransform = obj
        .get("GeoTransform")
        .and_then(|v| parse_geotransform_string(Some(v)));
    transform.or(geotransform)
}

fn parse_transform_from_attrs(attrs: &Value, mode: ValidationMode) -> Result<Option<[f64; 6]>> {
    match mode {
        ValidationMode::Strict => parse_transform_from_attrs_strict(attrs),
        ValidationMode::Lenient => Ok(parse_transform_from_attrs_lenient(attrs)),
    }
}

fn same_transform(a: &[f64; 6], b: &[f64; 6]) -> bool {
    const TOL: f64 = 1e-9;
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= TOL)
}

fn validate_georef_consistency(
    attrs: &Value,
    transform: Option<[f64; 6]>,
    rows: usize,
    cell_size: f64,
    cell_size_y: Option<f64>,
    mode: ValidationMode,
) -> Result<()> {
    if mode == ValidationMode::Lenient {
        return Ok(());
    }

    const TOL: f64 = 1e-9;
    let Some(obj) = attrs.as_object() else {
        return Ok(());
    };
    let Some(t) = transform else {
        return Ok(());
    };

    if let Some(x) = obj.get("x_min").and_then(Value::as_f64) {
        if (x - t[0]).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'x_min' disagrees with transform".into(),
            ));
        }
    }

    if let Some(dx) = obj.get("cell_size_x").and_then(Value::as_f64) {
        if (dx - t[1].abs()).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'cell_size_x' disagrees with transform".into(),
            ));
        }
    }

    if let Some(dy) = obj.get("cell_size_y").and_then(Value::as_f64) {
        if (dy - t[5].abs()).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'cell_size_y' disagrees with transform".into(),
            ));
        }
    }

    if let Some(y) = obj.get("y_min").and_then(Value::as_f64) {
        let y_from_t = if t[5] == 0.0 {
            t[3] - cell_size_y.unwrap_or(cell_size) * rows as f64
        } else {
            t[3] + t[5] * rows as f64
        };
        if (y - y_from_t).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'y_min' disagrees with transform".into(),
            ));
        }
    }

    Ok(())
}

fn parse_validation_mode_from_attrs(attrs: &Value) -> ValidationMode {
    let mode = attrs
        .as_object()
        .and_then(|obj| obj.get("zarr_validation_mode"))
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase());
    match mode.as_deref() {
        Some("lenient") => ValidationMode::Lenient,
        _ => ValidationMode::Strict,
    }
}

fn parse_geotransform_string(v: Option<&Value>) -> Option<[f64; 6]> {
    let s = v?.as_str()?.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() < 6 {
        return None;
    }
    let mut out = [0.0f64; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = parts[i].parse::<f64>().ok()?;
    }
    Some(out)
}

fn parse_epsg_from_crs_str(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Common simple form: EPSG:XXXX
    let upper = trimmed.to_ascii_uppercase();
    if upper.starts_with("EPSG:") {
        let (_, code) = upper.split_once(":")?;
        return code.parse::<u32>().ok();
    }

    // Common OGC forms seen in external metadata, e.g.:
    // - urn:ogc:def:crs:EPSG::3857
    // - http://www.opengis.net/def/crs/EPSG/0/3857
    // - https://www.opengis.net/def/crs/EPSG/0/3857
    let digits: String = upper
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    if !digits.is_empty()
        && (upper.contains(":EPSG::")
            || upper.contains("/EPSG/")
            || upper.contains(":EPSG:"))
    {
        return digits.parse::<u32>().ok();
    }

    None
}

fn parse_epsg_authority_code_obj(obj: &serde_json::Map<String, Value>) -> Option<u32> {
    let authority = obj.get("authority")?.as_str()?.trim().to_ascii_uppercase();
    if authority != "EPSG" {
        return None;
    }
    let code_v = obj.get("code")?;
    match code_v {
        Value::String(s) => s.trim().parse::<u32>().ok(),
        Value::Number(n) => n.as_u64().and_then(|x| u32::try_from(x).ok()),
        _ => None,
    }
}

fn parse_epsg_from_crs_json(v: &Value) -> Option<u32> {
    match v {
        Value::String(s) => parse_epsg_from_crs_str(s),
        Value::Number(n) => n.as_u64().and_then(|x| u32::try_from(x).ok()),
        Value::Object(obj) => {
            if let Some(epsg) = obj.get("epsg").and_then(parse_epsg_from_crs_json) {
                return Some(epsg);
            }
            if let Some(code) = parse_epsg_authority_code_obj(obj) {
                return Some(code);
            }
            if let Some(name) = obj.get("name").and_then(parse_epsg_from_crs_json) {
                return Some(name);
            }
            if let Some(props) = obj.get("properties").and_then(parse_epsg_from_crs_json) {
                return Some(props);
            }
            if let Some(id) = obj.get("id").and_then(parse_epsg_from_crs_json) {
                return Some(id);
            }
            None
        }
        _ => None,
    }
}

fn parse_epsg_from_crs_value(v: Option<&Value>) -> Option<u32> {
    parse_epsg_from_crs_json(v?)
}

fn grid_mapping_object_from_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<&serde_json::Map<String, Value>> {
    match attrs.get("grid_mapping") {
        Some(Value::Object(obj)) => Some(obj),
        Some(Value::String(name)) => attrs.get(name).and_then(Value::as_object),
        _ => None,
    }
}

fn parse_epsg_from_grid_mapping_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<u32> {
    let gm = grid_mapping_object_from_attrs(attrs)?;
    gm.get("epsg")
        .and_then(parse_epsg_from_crs_json)
        .or_else(|| gm.get("epsg_code").and_then(parse_epsg_from_crs_json))
        .or_else(|| gm.get("crs").and_then(parse_epsg_from_crs_json))
        .or_else(|| gm.get("id").and_then(parse_epsg_from_crs_json))
        .or_else(|| gm.get("spatial_ref").and_then(parse_epsg_from_crs_json))
}

fn parse_wkt_from_grid_mapping_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<String> {
    let gm = grid_mapping_object_from_attrs(attrs)?;
    gm.get("crs_wkt")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| gm.get("spatial_ref").and_then(Value::as_str).map(ToOwned::to_owned))
        .or_else(|| gm.get("wkt").and_then(Value::as_str).map(ToOwned::to_owned))
}

fn parse_proj4_from_grid_mapping_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<String> {
    let gm = grid_mapping_object_from_attrs(attrs)?;
    gm.get("crs_proj4")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| gm.get("proj4").and_then(Value::as_str).map(ToOwned::to_owned))
}

fn chunk_key(indices: &[usize], sep: &str) -> String {
    let s = if sep == "/" { "/" } else { "." };
    indices
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(s)
}

fn compress_bytes(compressor: &Option<CompressorSpec>, raw: &[u8]) -> Result<Vec<u8>> {
    match compressor {
        None => Ok(raw.to_vec()),
        Some(c) => match c.id.to_ascii_lowercase().as_str() {
            "zlib" => {
                let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(c.level.unwrap_or(6).clamp(0, 9) as u32));
                enc.write_all(raw)?;
                enc.finish().map_err(RasterError::Io)
            }
            "gzip" | "gz" => {
                let mut enc = GzEncoder::new(Vec::new(), Compression::new(c.level.unwrap_or(6).clamp(0, 9) as u32));
                enc.write_all(raw)?;
                enc.finish().map_err(RasterError::Io)
            }
            "zstd" => encode_zstd(raw, c.level.unwrap_or(3)),
            "lz4" => {
                let mut enc = lz4_flex::frame::FrameEncoder::new(Vec::new());
                enc.write_all(raw)?;
                enc.finish()
                    .map_err(|e| RasterError::Other(format!("lz4 encode error: {e}")))
            }
            other => Err(RasterError::UnsupportedDataType(format!(
                "unsupported zarr compressor '{other}'"
            ))),
        },
    }
}

fn decompress_bytes(compressor: &Option<CompressorSpec>, bytes: &[u8]) -> Result<Vec<u8>> {
    match compressor {
        None => Ok(bytes.to_vec()),
        Some(c) => match c.id.to_ascii_lowercase().as_str() {
            "zlib" => {
                let mut dec = ZlibDecoder::new(bytes);
                let mut out = Vec::new();
                dec.read_to_end(&mut out)?;
                Ok(out)
            }
            "gzip" | "gz" => {
                let mut dec = GzDecoder::new(bytes);
                let mut out = Vec::new();
                dec.read_to_end(&mut out)?;
                Ok(out)
            }
            "zstd" => decode_zstd(bytes),
            "lz4" => {
                let mut dec = lz4_flex::frame::FrameDecoder::new(bytes);
                let mut out = Vec::new();
                dec.read_to_end(&mut out)?;
                Ok(out)
            }
            other => Err(RasterError::UnsupportedDataType(format!(
                "unsupported zarr compressor '{other}'"
            ))),
        },
    }
}

fn encode_zstd(raw: &[u8], level: i32) -> Result<Vec<u8>> {
    #[cfg(feature = "zstd-native")]
    {
        return zstd::stream::encode_all(raw, level)
            .map_err(|e| RasterError::Other(format!("zstd encode error: {e}")));
    }

    #[cfg(not(feature = "zstd-native"))]
    {
        let _ = (raw, level);
        Err(RasterError::UnsupportedDataType(
            "zstd encode requires the 'zstd-native' feature".to_string(),
        ))
    }
}

fn decode_zstd(bytes: &[u8]) -> Result<Vec<u8>> {
    #[cfg(feature = "zstd-native")]
    {
        return zstd::stream::decode_all(bytes)
            .map_err(|e| RasterError::Other(format!("zstd decode error: {e}")));
    }

    #[cfg(all(not(feature = "zstd-native"), feature = "zstd-pure-rust-decode"))]
    {
        let mut source = bytes;
        let mut decoder = ruzstd::decoding::StreamingDecoder::new(&mut source)
            .map_err(|e| RasterError::Other(format!("zstd decode error: {e}")))?;
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| RasterError::Other(format!("zstd decode error: {e}")))?;
        return Ok(out);
    }

    #[cfg(all(not(feature = "zstd-native"), not(feature = "zstd-pure-rust-decode")))]
    {
        let _ = bytes;
        Err(RasterError::UnsupportedDataType(
            "zstd decode requires either 'zstd-native' or 'zstd-pure-rust-decode'"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::RasterConfig;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        temp_dir().join(format!("zarr_test_{pid}_{ts}_{n}.zarr"))
    }

    #[test]
    fn zarr_roundtrip() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            crs: crate::crs_info::CrsInfo::from_epsg(4326),
            ..Default::default()
        };
        let mut data: Vec<f64> = (0..40).map(|i| i as f64 * 0.25).collect();
        data[7] = -9999.0;
        let mut r = Raster::from_data(cfg, data).unwrap();
        r.metadata.push(("zarr_dimension_separator".into(), "/".into()));

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        // Slash-separated chunk keys should be present.
        assert!(dir.join("0").join("0").exists());

        let r2 = read_from_dir(&dir).unwrap();

        assert_eq!(r.cols, r2.cols);
        assert_eq!(r.rows, r2.rows);
        assert!((r.x_min - r2.x_min).abs() < 1e-10);
        assert!((r.y_min - r2.y_min).abs() < 1e-10);
        assert_eq!(r2.crs.epsg, Some(4326));
        for row in 0..r.rows {
            for col in 0..r.cols {
                let a = r.get_raw(0, row as isize, col as isize).unwrap();
                let b = r2.get_raw(0, row as isize, col as isize).unwrap();
                if r.is_nodata(a) {
                    assert!(r2.is_nodata(b));
                } else {
                    assert!((a - b).abs() < 1e-5);
                }
            }
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_roundtrip_multiband() {
        let cfg = RasterConfig {
            cols: 6,
            rows: 4,
            bands: 2,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
            .map(|i| i as f64)
            .collect();
        let mut r = Raster::from_data(cfg, data).unwrap();
        r.metadata.push(("zarr_dimension_separator".into(), "/".into()));
        r.metadata.push(("zarr_chunk_bands".into(), "1".into()));

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();
        let r2 = read_from_dir(&dir).unwrap();

        assert_eq!(r2.bands, 2);
        assert_eq!(r2.get_raw(0, 0, 0), Some(0.0));
        assert_eq!(r2.get_raw(1, 0, 0), Some(24.0));
        assert_eq!(r2.get_raw(1, 3, 5), Some(47.0));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_writer_respects_chunk_row_col_metadata() {
        let cfg = RasterConfig {
            cols: 9,
            rows: 7,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..(cfg.cols * cfg.rows)).map(|i| i as f64).collect();
        let mut r = Raster::from_data(cfg, data).unwrap();
        r.metadata.push(("zarr_chunk_rows".into(), "3".into()));
        r.metadata.push(("zarr_chunk_cols".into(), "4".into()));

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let zarray_text = fs::read_to_string(dir.join(".zarray")).unwrap();
        let zarray_json: serde_json::Value = serde_json::from_str(&zarray_text).unwrap();
        let chunks = zarray_json
            .get("chunks")
            .and_then(serde_json::Value::as_array)
            .expect(".zarray must contain chunks array");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].as_u64(), Some(3));
        assert_eq!(chunks[1].as_u64(), Some(4));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_uses_transform_when_origin_and_cellsize_missing() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("x_min");
        obj.remove("y_min");
        obj.remove("cell_size_x");
        obj.remove("cell_size_y");
        obj.insert(
            "transform".into(),
            json!([
                10.0,
                2.0,
                0.0,
                30.0,
                0.0,
                -2.0
            ]),
        );
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert!((r2.x_min - 10.0).abs() < 1e-10);
        assert!((r2.y_min - 20.0).abs() < 1e-10);
        assert!((r2.cell_size_x - 2.0).abs() < 1e-10);
        assert!((r2.cell_size_y - 2.0).abs() < 1e-10);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_aliases_from_external_style_attrs() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            crs: crate::crs_info::CrsInfo::from_epsg(4326),
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("crs_wkt");
        obj.remove("crs_proj4");
        obj.insert("epsg".into(), json!(4326));
        obj.insert(
            "spatial_ref".into(),
            json!("GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\"]]"),
        );
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(4326));
        assert!(r2.crs.wkt.as_deref().unwrap_or_default().contains("WGS 84"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_epsg_string_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.insert("crs".into(), json!("EPSG:3857"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(3857));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_epsg_object_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.insert(
            "crs".into(),
            json!({
                "type": "name",
                "properties": {
                    "name": "EPSG:3395"
                }
            }),
        );
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(3395));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_authority_code_object_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.insert(
            "crs".into(),
            json!({
                "id": {
                    "authority": "EPSG",
                    "code": 3035
                }
            }),
        );
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(3035));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_ogc_urn_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.insert("crs".into(), json!("urn:ogc:def:crs:EPSG::32617"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(32617));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_ogc_url_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.insert("crs".into(), json!("https://www.opengis.net/def/crs/EPSG/0/3857"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(3857));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_crs_from_grid_mapping_named_object_attr() {
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("crs_epsg");
        obj.remove("epsg");
        obj.remove("crs_wkt");
        obj.remove("spatial_ref");
        obj.remove("crs_proj4");
        obj.remove("proj4");
        obj.insert("grid_mapping".into(), json!("spatial_ref"));
        obj.insert(
            "spatial_ref".into(),
            json!({
                "epsg_code": "EPSG:32617",
                "crs_wkt": "PROJCS[\"WGS 84 / UTM zone 17N\",GEOGCS[\"WGS 84\"]]",
                "proj4": "+proj=utm +zone=17 +datum=WGS84 +units=m +no_defs"
            }),
        );
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert_eq!(r2.crs.epsg, Some(32617));
        assert!(r2.crs.wkt.as_deref().unwrap_or_default().contains("UTM zone 17N"));
        assert!(r2.crs.proj4.as_deref().unwrap_or_default().contains("+proj=utm"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_uses_geotransform_string_when_transform_missing() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("x_min");
        obj.remove("y_min");
        obj.remove("cell_size_x");
        obj.remove("cell_size_y");
        obj.remove("transform");
        obj.insert("GeoTransform".into(), json!("10 2 0 30 0 -2"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).unwrap();
        assert!((r2.x_min - 10.0).abs() < 1e-10);
        assert!((r2.y_min - 20.0).abs() < 1e-10);
        assert!((r2.cell_size_x - 2.0).abs() < 1e-10);
        assert!((r2.cell_size_y - 2.0).abs() < 1e-10);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_fails_on_conflicting_xmin_and_transform() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.insert("x_min".into(), json!(123.0));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let err = read_from_dir(&dir).expect_err("expected conflicting metadata error");
        assert!(
            format!("{err}").contains("conflicting geospatial metadata"),
            "unexpected error message: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_fails_on_invalid_geotransform_string() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("transform");
        obj.remove("x_min");
        obj.remove("y_min");
        obj.remove("cell_size_x");
        obj.remove("cell_size_y");
        obj.insert("GeoTransform".into(), json!("10 2 0 bad 0 -2"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let err = read_from_dir(&dir).expect_err("expected invalid geotransform error");
        assert!(
            format!("{err}").contains("invalid geospatial metadata"),
            "unexpected error message: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_lenient_mode_allows_conflicting_georef_metadata() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.insert("x_min".into(), json!(123.0));
        obj.insert("zarr_validation_mode".into(), json!("lenient"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).expect("lenient mode should not fail on conflicts");
        assert!((r2.x_min - 123.0).abs() < 1e-10);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_lenient_mode_allows_invalid_geotransform_string() {
        let cfg = RasterConfig {
            cols: 8,
            rows: 5,
            x_min: 10.0,
            y_min: 20.0,
            cell_size: 2.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("transform");
        obj.remove("x_min");
        obj.remove("y_min");
        obj.remove("cell_size_x");
        obj.remove("cell_size_y");
        obj.insert("GeoTransform".into(), json!("10 2 0 bad 0 -2"));
        obj.insert("zarr_validation_mode".into(), json!("lenient"));
        fs::write(
            dir.join(".zattrs"),
            serde_json::to_string_pretty(&zattrs).unwrap(),
        )
        .unwrap();

        let r2 = read_from_dir(&dir).expect("lenient mode should ignore invalid geotransform");
        assert_eq!(r2.rows, 5);
        assert_eq!(r2.cols, 8);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_nodata_from_cf_fill_value_attr() {
        // A producer that writes _FillValue instead of nodata (e.g. xarray-style CF output).
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..16).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        // Replace `nodata` with the CF-style `_FillValue` key.
        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("nodata");
        obj.insert("_FillValue".into(), json!(-32768.0_f64));
        fs::write(dir.join(".zattrs"), serde_json::to_string_pretty(&zattrs).unwrap()).unwrap();

        let r2 = read_from_dir(&dir).expect("should read nodata from _FillValue");
        assert!(
            (r2.nodata - (-32768.0)).abs() < 1e-6,
            "expected nodata=-32768; got {}",
            r2.nodata
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_read_nodata_from_missing_value_attr() {
        // A producer that writes missing_value (another CF convention).
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..16).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.remove("nodata");
        obj.insert("missing_value".into(), json!(-1.0_f64));
        fs::write(dir.join(".zattrs"), serde_json::to_string_pretty(&zattrs).unwrap()).unwrap();

        let r2 = read_from_dir(&dir).expect("should read nodata from missing_value");
        assert!(
            (r2.nodata - (-1.0)).abs() < 1e-6,
            "expected nodata=-1; got {}",
            r2.nodata
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zarr_v2_explicit_nodata_takes_precedence_over_cf_fill_value() {
        // When both `nodata` and `_FillValue` are present, explicit `nodata` wins.
        let cfg = RasterConfig {
            cols: 4,
            rows: 4,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..16).map(|i| i as f64).collect();
        let r = Raster::from_data(cfg, data).unwrap();

        let dir = tmp_dir();
        write_to_dir(&r, &dir).unwrap();

        // Add a _FillValue that differs from nodata; nodata should still win.
        let mut zattrs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join(".zattrs")).unwrap()).unwrap();
        let obj = zattrs.as_object_mut().unwrap();
        obj.insert("_FillValue".into(), json!(0.0_f64));
        fs::write(dir.join(".zattrs"), serde_json::to_string_pretty(&zattrs).unwrap()).unwrap();

        let r2 = read_from_dir(&dir).expect("should read OK");
        assert!(
            (r2.nodata - (-9999.0)).abs() < 1e-6,
            "expected explicit nodata=-9999 to take precedence; got {}",
            r2.nodata
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
