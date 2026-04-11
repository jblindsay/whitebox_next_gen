//! Zarr v3 support (filesystem store focus).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{RasterError, Result};
use crate::raster::{DataType, RasterConfig};
use crate::raster::Raster;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ZarrV3Root {
    pub zarr_format: u8,
    pub node_type: String,
    pub shape: Option<Vec<usize>>,
    pub data_type: Option<Value>,
    pub chunk_grid: Option<Value>,
    pub chunk_key_encoding: Option<Value>,
    pub codecs: Option<Vec<Value>>,
    pub fill_value: Option<Value>,
    pub attributes: Option<Value>,
    pub dimension_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChunkGrid {
    name: String,
    configuration: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct ChunkKeyEncoding {
    name: String,
    configuration: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct Codec {
    name: String,
    configuration: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
enum Endian {
    Little,
    Big,
    NativeOneByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationMode {
    Strict,
    Lenient,
}

#[derive(Debug, Clone)]
enum TransposeSpec {
    /// No transpose codec present in the pipeline.
    None,
    /// `"C"` order — identity permutation, decoded as-is.
    C,
    /// `"F"` order — reverse all axes (resolved to concrete indices once ndim is known).
    F,
    /// Explicit permutation array, e.g. `[1, 0]` or `[2, 1, 0]`.
    Explicit(Vec<usize>),
}

pub(crate) fn is_v3_store(dir: &Path) -> bool {
    dir.join("zarr.json").exists()
}

/// Returns `true` if `dir` contains a v3 `zarr.json` whose `node_type` is
/// `"group"` (as opposed to `"array"`).  Returns `false` on any parse error
/// so that callers can fall through to the normal array path.
pub(crate) fn is_v3_group(dir: &Path) -> bool {
    let Ok(s) = fs::read_to_string(dir.join("zarr.json")) else { return false; };
    let Ok(v) = serde_json::from_str::<Value>(&s) else { return false; };
    v.get("node_type").and_then(Value::as_str) == Some("group")
}

/// Discover the ordered list of array sub-paths that make up a multi-scale
/// pyramid rooted at `dir`.
///
/// Resolution order: finest (`0`) first, coarsest last — matching the
/// OME-NGFF convention (`multiscales[0].datasets[].path`).
///
/// Falls back to scanning for numeric sub-directories (`0/`, `1/`, ...) when
/// no `multiscales` attribute block is present, so plain "hand-rolled" pyramid
/// groups also work.
pub(crate) fn discover_multiscale_levels_v3(dir: &Path) -> Vec<std::path::PathBuf> {
    // Try OME-NGFF attributes: zarr.json -> attributes.multiscales[0].datasets[].path
    if let Ok(s) = fs::read_to_string(dir.join("zarr.json")) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            if let Some(datasets) = v
                .get("attributes")
                .and_then(|a| a.get("multiscales"))
                .and_then(Value::as_array)
                .and_then(|ms| ms.first())
                .and_then(|m| m.get("datasets"))
                .and_then(Value::as_array)
            {
                let paths: Vec<_> = datasets
                    .iter()
                    .filter_map(|d| d.get("path").and_then(Value::as_str))
                    .map(|p| dir.join(p))
                    .filter(|p| p.join("zarr.json").exists())
                    .collect();
                if !paths.is_empty() {
                    return paths;
                }
            }
        }
    }

    // Fallback: scan for consecutive numeric sub-dirs that look like v3 arrays.
    let mut levels = Vec::new();
    for i in 0usize.. {
        let candidate = dir.join(i.to_string());
        if candidate.join("zarr.json").exists() {
            levels.push(candidate);
        } else {
            break;
        }
    }
    levels
}

pub(crate) fn read_from_dir(dir: &Path) -> Result<Raster> {
    let root = parse_root(dir)?;
    validate_root(&root)?;

    let shape = root
        .shape
        .as_ref()
        .ok_or_else(|| RasterError::CorruptData("zarr.json missing required 'shape'".into()))?;
    let (bands, rows, cols) = if shape.len() == 3 {
        (shape[0], shape[1], shape[2])
    } else {
        (1, shape[0], shape[1])
    };

    let data_type = root
        .data_type
        .as_ref()
        .ok_or_else(|| RasterError::CorruptData("zarr.json missing required 'data_type'".into()))?;
    let (dtype, default_endian) = parse_v3_data_type(data_type)?;

    let chunk_shape = parse_regular_chunk_shape(
        root.chunk_grid
            .as_ref()
            .ok_or_else(|| RasterError::CorruptData("zarr.json missing required 'chunk_grid'".into()))?,
    )?;
    if chunk_shape.len() != shape.len() {
        return Err(RasterError::CorruptData(format!(
            "chunk_shape has {} dimension(s) but shape has {}; they must match",
            chunk_shape.len(),
            shape.len()
        )));
    }
    let (chunk_bands, chunk_rows, chunk_cols) = if chunk_shape.len() == 3 {
        (chunk_shape[0].max(1), chunk_shape[1].max(1), chunk_shape[2].max(1))
    } else {
        (1, chunk_shape[0].max(1), chunk_shape[1].max(1))
    };

    let (encoding_name, encoding_sep) = parse_chunk_key_encoding(root.chunk_key_encoding.as_ref())?;
    let codecs = root
        .codecs
        .as_ref()
        .ok_or_else(|| RasterError::CorruptData("zarr.json missing required 'codecs'".into()))?;
    let (codec_endian, compressor, transpose_spec) = parse_codec_pipeline(codecs, default_endian)?;
    let ndim = if bands > 1 { 3 } else { 2 };
    let transpose_order: Option<Vec<usize>> = match transpose_spec {
        TransposeSpec::None | TransposeSpec::C => None,
        TransposeSpec::F => {
            let v: Vec<usize> = (0..ndim).rev().collect();
            if v.iter().enumerate().all(|(i, &p)| p == i) { None } else { Some(v) }
        }
        TransposeSpec::Explicit(v) => {
            if v.len() != ndim {
                return Err(RasterError::CorruptData(format!(
                    "transpose order length {} does not match array ndim {}",
                    v.len(), ndim
                )));
            }
            let mut seen = vec![false; ndim];
            for &p in &v {
                if p >= ndim {
                    return Err(RasterError::CorruptData(format!(
                        "transpose order contains out-of-range axis index {p} for {ndim}D array"
                    )));
                }
                if seen[p] {
                    return Err(RasterError::CorruptData(format!(
                        "transpose order contains duplicate axis index {p}"
                    )));
                }
                seen[p] = true;
            }
            if v.iter().enumerate().all(|(i, &p)| p == i) { None } else { Some(v) }
        }
    };

    let early_attrs = root.attributes.as_ref().and_then(Value::as_object);
    let early_mode = parse_validation_mode_from_attrs(early_attrs);
    validate_dimension_names(root.dimension_names.as_deref(), shape.len(), early_mode)?;
    let nodata = early_attrs
        .and_then(|a| a.get("nodata")).and_then(Value::as_f64)
        .or_else(|| early_attrs.and_then(|a| a.get("_FillValue")).and_then(Value::as_f64))
        .or_else(|| early_attrs.and_then(|a| a.get("missing_value")).and_then(Value::as_f64))
        .unwrap_or_else(|| fill_value_to_f64(root.fill_value.as_ref()).unwrap_or(-9999.0));

    let mut data = vec![nodata; bands * rows * cols];
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);
    let band_plane_len = rows * cols;
    let cb_block_len = chunk_bands * band_plane_len;
    data.par_chunks_mut(cb_block_len)
        .enumerate()
        .try_for_each(|(cb, data_cb)| -> Result<()> {
            for cr in 0..n_chunk_rows {
                for cc in 0..n_chunk_cols {
                    let this_bands = (bands - cb * chunk_bands).min(chunk_bands);
                    let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
                    let this_cols = (cols - cc * chunk_cols).min(chunk_cols);
                    let chunk_path = if bands > 1 {
                        resolve_chunk_path(dir, &encoding_name, &encoding_sep, &[cb, cr, cc])
                    } else {
                        resolve_chunk_path(dir, &encoding_name, &encoding_sep, &[cr, cc])
                    };

                    let chunk_data = if chunk_path.exists() {
                        let chunk_bytes = fs::read(&chunk_path)?;
                        let raw = decompress_bytes(&compressor, &chunk_bytes)?;
                        if let Some(ref order) = transpose_order {
                            let bpp = dtype.size_bytes();
                            let this_size = this_bands * this_rows * this_cols;
                            let full_size = chunk_bands * chunk_rows * chunk_cols;
                            let n_stored = raw.len() / bpp;
                            let this_shape: Vec<usize> = if bands > 1 {
                                vec![this_bands, this_rows, this_cols]
                            } else {
                                vec![this_rows, this_cols]
                            };
                            let full_shape: Vec<usize> = if bands > 1 {
                                vec![chunk_bands, chunk_rows, chunk_cols]
                            } else {
                                vec![chunk_rows, chunk_cols]
                            };
                            if n_stored == full_size {
                                // Standard padded chunk (spec-compliant producers always pad
                                // boundary chunks to the full chunk shape before encoding).
                                let decoded = decode_typed_buffer(&raw, full_size, dtype, codec_endian)?;
                                let untransposed = apply_inverse_transpose(&decoded, order, &full_shape);
                                if this_size == full_size {
                                    untransposed
                                } else {
                                    extract_valid_subchunk(&untransposed, &full_shape, &this_shape)
                                }
                            } else if n_stored == this_size {
                                // Unpadded boundary chunk (non-spec-compliant producer but
                                // encountered in the wild; handle gracefully).
                                let decoded = decode_typed_buffer(&raw, this_size, dtype, codec_endian)?;
                                apply_inverse_transpose(&decoded, order, &this_shape)
                            } else {
                                return Err(RasterError::CorruptData(format!(
                                    "v3 chunk size mismatch with transpose: \
                                     expected {} or {} values ({}bpp), got {} bytes",
                                    full_size, this_size, bpp, raw.len()
                                )));
                            }
                        } else {
                            decode_typed_buffer(
                                &raw,
                                this_bands * this_rows * this_cols,
                                dtype,
                                codec_endian,
                            )?
                        }
                    } else {
                        vec![nodata; this_bands * this_rows * this_cols]
                    };

                    for bb in 0..this_bands {
                        for rr in 0..this_rows {
                            for cc2 in 0..this_cols {
                                let src_i = bb * this_rows * this_cols + rr * this_cols + cc2;
                                let dst_row = cr * chunk_rows + rr;
                                let dst_col = cc * chunk_cols + cc2;
                                data_cb[bb * band_plane_len + dst_row * cols + dst_col] = chunk_data[src_i];
                            }
                        }
                    }
                }
            }
            Ok(())
        })?;

    if data.iter().all(|v| v.is_nan()) && !nodata.is_nan() {
        data.fill(nodata);
    }

    let attrs_obj = root
        .attributes
        .as_ref()
        .and_then(Value::as_object);
    let validation_mode = parse_validation_mode_from_attrs(attrs_obj);
    let transform = parse_transform_from_attrs(attrs_obj, validation_mode)?;
    let x_min = attrs_obj
        .and_then(|a| a.get("x_min"))
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[0]))
        .unwrap_or(0.0);
    let cell_size = attrs_obj
        .and_then(|a| a.get("cell_size_x"))
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[1].abs()))
        .unwrap_or(1.0);
    let cell_size_y = attrs_obj
        .and_then(|a| a.get("cell_size_y"))
        .and_then(Value::as_f64)
        .or_else(|| transform.map(|t| t[5].abs()));
    let y_min = attrs_obj
        .and_then(|a| a.get("y_min"))
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
        attrs_obj,
        transform,
        rows,
        cell_size,
        cell_size_y,
        validation_mode,
    )?;

    let crs = crate::crs_info::CrsInfo {
        epsg: attrs_obj
            .and_then(|a| a.get("crs_epsg"))
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .or_else(|| {
                attrs_obj
                    .and_then(|a| a.get("epsg"))
                    .and_then(Value::as_u64)
                    .map(|v| v as u32)
            })
            .or_else(|| attrs_obj.and_then(parse_epsg_from_crs_value_from_attrs))
            .or_else(|| attrs_obj.and_then(parse_epsg_from_grid_mapping_attrs)),
        wkt: attrs_obj
            .and_then(|a| a.get("crs_wkt"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                attrs_obj
                    .and_then(|a| a.get("spatial_ref"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .or_else(|| attrs_obj.and_then(parse_wkt_from_grid_mapping_attrs)),
        proj4: attrs_obj
            .and_then(|a| a.get("crs_proj4"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                attrs_obj
                    .and_then(|a| a.get("proj4"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    })
                    .or_else(|| attrs_obj.and_then(parse_proj4_from_grid_mapping_attrs)),
    };

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
            ("zarr_version".into(), "3".into()),
            ("zarr_chunk_key_encoding".into(), encoding_name),
            ("zarr_dimension_separator".into(), encoding_sep),
        ],
    };
    Raster::from_data(cfg, data)
}

pub(crate) fn write_to_dir(raster: &Raster, dir: &Path) -> Result<()> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    let bands = raster.bands;
    let rows = raster.rows;
    let cols = raster.cols;
    let chunk_bands = metadata_usize(raster, "zarr_chunk_bands")
        .unwrap_or(1)
        .clamp(1, bands.max(1));
    let chunk_rows = metadata_usize(raster, "zarr_chunk_rows")
        .unwrap_or(rows.clamp(1, 256))
        .clamp(1, rows.max(1));
    let chunk_cols = metadata_usize(raster, "zarr_chunk_cols")
        .unwrap_or(cols.clamp(1, 256))
        .clamp(1, cols.max(1));

    let (encoding_name, encoding_sep) = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "zarr_chunk_key_encoding")
        .map(|(_, v)| {
            let n = v.to_ascii_lowercase();
            if n == "v2" {
                (
                    "v2".to_owned(),
                    raster
                        .metadata
                        .iter()
                        .find(|(k, _)| k == "zarr_dimension_separator" || k == "zarr_chunk_separator")
                        .map(|(_, v)| if v == "/" { "/".to_owned() } else { ".".to_owned() })
                        .unwrap_or_else(|| ".".to_owned()),
                )
            } else {
                (
                    "default".to_owned(),
                    raster
                        .metadata
                        .iter()
                        .find(|(k, _)| k == "zarr_dimension_separator" || k == "zarr_chunk_separator")
                        .map(|(_, v)| if v == "." { ".".to_owned() } else { "/".to_owned() })
                        .unwrap_or_else(|| "/".to_owned()),
                )
            }
        })
        .unwrap_or_else(|| ("default".to_owned(), "/".to_owned()));

    let compressor_name = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "zarr_compressor")
        .map(|(_, v)| v.to_ascii_lowercase())
        .unwrap_or_else(|| "zlib".to_owned());
    let compressor_level = raster
        .metadata
        .iter()
        .find(|(k, _)| k == "zarr_compression_level")
        .and_then(|(_, v)| v.parse::<i32>().ok());

    let mut codecs = vec![json!({
        "name": "bytes",
        "configuration": { "endian": "little" }
    })];
    if compressor_name != "none" {
        let mut compressor_obj = json!({ "name": compressor_name });
        if let Some(level) = compressor_level {
            compressor_obj["configuration"] = json!({ "level": level });
        }
        codecs.push(compressor_obj);
    }

    let mut attrs = json!({
        "x_min": raster.x_min,
        "y_min": raster.y_min,
        "cell_size_x": raster.cell_size_x,
        "cell_size_y": raster.cell_size_y,
        "nodata": raster.nodata,
        "data_type": raster.data_type.as_str(),
        "_ARRAY_DIMENSIONS": if bands > 1 { json!(["band", "y", "x"]) } else { json!(["y", "x"]) },
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
    if let Some(obj) = attrs.as_object_mut() {
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

    let chunk_key_encoding = if encoding_name == "v2" {
        json!({
            "name": "v2",
            "configuration": { "separator": encoding_sep }
        })
    } else {
        json!({
            "name": "default",
            "configuration": { "separator": encoding_sep }
        })
    };

    let root = json!({
        "zarr_format": 3,
        "node_type": "array",
        "shape": if bands > 1 { json!([bands, rows, cols]) } else { json!([rows, cols]) },
        "data_type": raster.data_type.as_str(),
        "chunk_grid": {
            "name": "regular",
            "configuration": {
                "chunk_shape": if bands > 1 {
                    json!([chunk_bands, chunk_rows, chunk_cols])
                } else {
                    json!([chunk_rows, chunk_cols])
                }
            }
        },
        "chunk_key_encoding": chunk_key_encoding,
        "codecs": codecs,
        "fill_value": raster.nodata,
        "dimension_names": if bands > 1 { json!(["band", "y", "x"]) } else { json!(["y", "x"]) },
        "attributes": attrs,
    });
    fs::write(
        dir.join("zarr.json"),
        serde_json::to_string_pretty(&root)
            .map_err(|e| RasterError::Other(format!("failed to serialize zarr.json: {e}")))?,
    )?;

    let compressor = if compressor_name == "none" {
        None
    } else {
        Some((compressor_name, compressor_level))
    };

    let n_chunk_bands = bands.div_ceil(chunk_bands);
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);
    for cb in 0..n_chunk_bands {
        for cr in 0..n_chunk_rows {
            for cc in 0..n_chunk_cols {
                let this_bands = (bands - cb * chunk_bands).min(chunk_bands);
                let this_rows = (rows - cr * chunk_rows).min(chunk_rows);
                let this_cols = (cols - cc * chunk_cols).min(chunk_cols);

                let mut raw = Vec::with_capacity(this_bands * this_rows * this_cols * raster.data_type.size_bytes());
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

                let bytes = compress_bytes(&compressor, &raw)?;

                let chunk_path = if bands > 1 {
                    resolve_chunk_path(dir, &encoding_name, &encoding_sep, &[cb, cr, cc])
                } else {
                    resolve_chunk_path(dir, &encoding_name, &encoding_sep, &[cr, cc])
                };
                if let Some(parent) = chunk_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut f = File::create(chunk_path)?;
                f.write_all(&bytes)?;
            }
        }
    }

    Ok(())
}

fn parse_root(dir: &Path) -> Result<ZarrV3Root> {
    let p = dir.join("zarr.json");
    let s = fs::read_to_string(&p)?;
    serde_json::from_str(&s)
        .map_err(|e| RasterError::CorruptData(format!("invalid zarr.json metadata: {e}")))
}

fn validate_root(root: &ZarrV3Root) -> Result<()> {
    if root.zarr_format != 3 {
        return Err(RasterError::UnsupportedDataType(format!(
            "zarr_format={} in zarr.json (expected 3)",
            root.zarr_format
        )));
    }
    if root.node_type != "array" {
        return Err(RasterError::UnsupportedDataType(format!(
            "zarr v3 node_type '{}' is not supported (array required)",
            root.node_type
        )));
    }

    let shape = root.shape.as_ref().ok_or_else(|| {
        RasterError::CorruptData("zarr.json missing required 'shape'".into())
    })?;
    if shape.len() != 2 && shape.len() != 3 {
        return Err(RasterError::UnsupportedDataType(format!(
            "only 2D or 3D [band,y,x] zarr v3 arrays are supported (got {}D)",
            shape.len()
        )));
    }
    if shape.iter().any(|&d| d == 0) {
        return Err(RasterError::CorruptData(format!(
            "zarr.json 'shape' contains a zero dimension: {shape:?}"
        )));
    }

    if root.data_type.is_none() {
        return Err(RasterError::CorruptData(
            "zarr.json missing required 'data_type'".into(),
        ));
    }
    if root.chunk_grid.is_none() {
        return Err(RasterError::CorruptData(
            "zarr.json missing required 'chunk_grid'".into(),
        ));
    }
    if root.codecs.is_none() {
        return Err(RasterError::CorruptData(
            "zarr.json missing required 'codecs'".into(),
        ));
    }

    let _ = (
        &root.chunk_key_encoding,
        &root.fill_value,
        &root.attributes,
        &root.dimension_names,
    );
    Ok(())
}

fn parse_v3_data_type(v: &Value) -> Result<(DataType, Endian)> {
    if let Some(s) = v.as_str() {
        if let Some(dt) = DataType::from_str(s) {
            return Ok((dt, Endian::Little));
        }
        return parse_zarr_dtype_string(s);
    }

    if let Some(obj) = v.as_object() {
        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| RasterError::CorruptData("v3 data_type object missing 'name'".into()))?;
        if let Some(dt) = DataType::from_str(name) {
            let endian = obj
                .get("configuration")
                .and_then(Value::as_object)
                .and_then(|cfg| cfg.get("endian"))
                .and_then(Value::as_str)
                .map(|e| match e {
                    "big" => Endian::Big,
                    "little" => Endian::Little,
                    _ => Endian::Little,
                })
                .unwrap_or(Endian::Little);
            return Ok((dt, endian));
        }
        return parse_zarr_dtype_string(name);
    }

    Err(RasterError::UnsupportedDataType(
        "unsupported v3 data_type representation".into(),
    ))
}

fn parse_zarr_dtype_string(dtype: &str) -> Result<(DataType, Endian)> {
    let mut chars = dtype.chars();
    let first = chars
        .next()
        .ok_or_else(|| RasterError::CorruptData("empty dtype".into()))?;
    let (endian, rest) = match first {
        '<' => (Endian::Little, chars.as_str()),
        '>' => (Endian::Big, chars.as_str()),
        '|' => (Endian::NativeOneByte, chars.as_str()),
        _ => (Endian::Little, dtype),
    };

    let mut it = rest.chars();
    let kind = it
        .next()
        .ok_or_else(|| RasterError::CorruptData(format!("invalid dtype '{dtype}'")))?;
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

fn parse_regular_chunk_shape(v: &Value) -> Result<Vec<usize>> {
    let grid: ChunkGrid = serde_json::from_value(v.clone())
        .map_err(|e| RasterError::CorruptData(format!("invalid chunk_grid: {e}")))?;
    if grid.name != "regular" {
        return Err(RasterError::UnsupportedDataType(format!(
            "unsupported chunk_grid name '{}' (regular required)",
            grid.name
        )));
    }
    let arr = grid
        .configuration
        .as_object()
        .and_then(|c| c.get("chunk_shape"))
        .and_then(Value::as_array)
        .ok_or_else(|| RasterError::CorruptData("chunk_grid.configuration.chunk_shape missing".into()))?;
    if arr.len() != 2 && arr.len() != 3 {
        return Err(RasterError::UnsupportedDataType(format!(
            "only 2D or 3D chunk_shape supported (got {}D)",
            arr.len()
        )));
    }
    let mut out = Vec::with_capacity(arr.len());
    for (i, v) in arr.iter().enumerate() {
        let n = v
            .as_u64()
            .ok_or_else(|| RasterError::CorruptData(format!("invalid chunk_shape[{i}]")))?
            as usize;
        out.push(n.max(1));
    }
    Ok(out)
}

fn parse_chunk_key_encoding(v: Option<&Value>) -> Result<(String, String)> {
    let Some(raw) = v else {
        return Ok(("default".into(), "/".into()));
    };
    let encoding: ChunkKeyEncoding = serde_json::from_value(raw.clone())
        .map_err(|e| RasterError::CorruptData(format!("invalid chunk_key_encoding: {e}")))?;

    let sep = encoding
        .configuration
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|cfg| cfg.get("separator"))
        .and_then(Value::as_str)
        .map(|s| if s == "." { ".".to_owned() } else { "/".to_owned() })
        .unwrap_or_else(|| {
            if encoding.name == "v2" {
                ".".to_owned()
            } else {
                "/".to_owned()
            }
        });
    match encoding.name.as_str() {
        "default" | "v2" => Ok((encoding.name, sep)),
        other => Err(RasterError::UnsupportedDataType(format!(
            "unsupported chunk_key_encoding '{other}'"
        ))),
    }
}

type CompressorSpec = Option<(String, Option<i32>)>;

fn parse_codec_pipeline(codecs: &[Value], default_endian: Endian) -> Result<(Endian, CompressorSpec, TransposeSpec)> {
    let parsed: Vec<Codec> = codecs
        .iter()
        .cloned()
        .map(|v| {
            serde_json::from_value(v)
                .map_err(|e| RasterError::CorruptData(format!("invalid codec entry: {e}")))
        })
        .collect::<Result<_>>()?;

    let mut endian = default_endian;
    let mut compressor: Option<(String, Option<i32>)> = None;
    let mut transpose = TransposeSpec::None;
    for codec in parsed {
        match codec.name.as_str() {
            "bytes" => {
                if let Some(e) = codec
                    .configuration
                    .as_ref()
                    .and_then(Value::as_object)
                    .and_then(|cfg| cfg.get("endian"))
                    .and_then(Value::as_str)
                {
                    endian = if e == "big" { Endian::Big } else { Endian::Little };
                }
            }
            "zlib" | "gzip" | "gz" | "zstd" | "lz4" => {
                let level = codec
                    .configuration
                    .as_ref()
                    .and_then(Value::as_object)
                    .and_then(|cfg| cfg.get("level"))
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                compressor = Some((codec.name, level));
            }
            "transpose" => {
                let order_val = codec
                    .configuration
                    .as_ref()
                    .and_then(Value::as_object)
                    .and_then(|cfg| cfg.get("order"));
                transpose = match order_val {
                    None => TransposeSpec::C,
                    Some(Value::String(s)) if s.eq_ignore_ascii_case("C") => TransposeSpec::C,
                    Some(Value::String(s)) if s.eq_ignore_ascii_case("F") => TransposeSpec::F,
                    Some(Value::Array(arr)) => {
                        let perm: Option<Vec<usize>> = arr
                            .iter()
                            .map(|v| v.as_u64().map(|n| n as usize))
                            .collect();
                        match perm {
                            Some(p) => TransposeSpec::Explicit(p),
                            None => return Err(RasterError::CorruptData(
                                "transpose codec 'order' array contains non-integer values".into(),
                            )),
                        }
                    }
                    _ => return Err(RasterError::CorruptData(
                        "transpose codec has unrecognized 'order' configuration".into(),
                    )),
                };
            }
            other => {
                return Err(RasterError::UnsupportedDataType(format!(
                    "unsupported zarr v3 codec '{other}' in pipeline; cannot safely decode"
                )));
            }
        }
    }

    Ok((endian, compressor, transpose))
}

fn resolve_chunk_path(
    dir: &Path,
    encoding_name: &str,
    separator: &str,
    chunk_indices: &[usize],
) -> std::path::PathBuf {
    let key = if encoding_name == "v2" {
        let sep = if separator == "/" { "/" } else { "." };
        chunk_indices
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    } else if separator == "." {
        format!(
            "c.{}",
            chunk_indices
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(".")
        )
    } else {
        format!(
            "c/{}",
            chunk_indices
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join("/")
        )
    };
    dir.join(key)
}

fn metadata_usize(raster: &Raster, key: &str) -> Option<usize> {
    raster
        .metadata
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<usize>().ok())
}

fn decode_typed_buffer(raw: &[u8], n_values: usize, dtype: DataType, endian: Endian) -> Result<Vec<f64>> {
    let bpp = dtype.size_bytes();
    let expected = n_values * bpp;
    if raw.len() != expected {
        return Err(RasterError::CorruptData(format!(
            "v3 chunk size mismatch: expected {expected}, got {}",
            raw.len()
        )));
    }
    let mut out = Vec::with_capacity(n_values);
    for i in 0..n_values {
        let src = &raw[i * bpp..(i + 1) * bpp];
        out.push(decode_sample(src, dtype, endian)?);
    }
    Ok(out)
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

fn parse_transform_tuple_from_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<[f64; 6]> {
    let arr = attrs.get("transform")?.as_array()?;
    if arr.len() < 6 {
        return None;
    }
    let mut out = [0.0f64; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = arr[i].as_f64()?;
    }
    Some(out)
}

fn parse_geotransform_string_from_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<[f64; 6]> {
    let s = attrs.get("GeoTransform")?.as_str()?.trim();
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

fn parse_epsg_from_crs_value_from_attrs(
    attrs: &serde_json::Map<String, Value>,
) -> Option<u32> {
    parse_epsg_from_crs_json(attrs.get("crs")?)
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

fn parse_transform_from_attrs_strict(
    attrs_obj: Option<&serde_json::Map<String, Value>>,
) -> Result<Option<[f64; 6]>> {
    let Some(attrs) = attrs_obj else {
        return Ok(None);
    };

    let transform = if attrs.get("transform").is_some() {
        Some(parse_transform_tuple_from_attrs(attrs).ok_or_else(|| {
            RasterError::CorruptData("invalid geospatial metadata: 'transform' must contain at least 6 numeric values".into())
        })?)
    } else {
        None
    };

    let geotransform = if attrs.get("GeoTransform").is_some() {
        Some(parse_geotransform_string_from_attrs(attrs).ok_or_else(|| {
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

fn parse_transform_from_attrs_lenient(
    attrs_obj: Option<&serde_json::Map<String, Value>>,
) -> Option<[f64; 6]> {
    let attrs = attrs_obj?;
    let transform = parse_transform_tuple_from_attrs(attrs);
    let geotransform = parse_geotransform_string_from_attrs(attrs);
    transform.or(geotransform)
}

fn parse_transform_from_attrs(
    attrs_obj: Option<&serde_json::Map<String, Value>>,
    mode: ValidationMode,
) -> Result<Option<[f64; 6]>> {
    match mode {
        ValidationMode::Strict => parse_transform_from_attrs_strict(attrs_obj),
        ValidationMode::Lenient => Ok(parse_transform_from_attrs_lenient(attrs_obj)),
    }
}

fn same_transform(a: &[f64; 6], b: &[f64; 6]) -> bool {
    const TOL: f64 = 1e-9;
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() <= TOL)
}

fn validate_georef_consistency(
    attrs_obj: Option<&serde_json::Map<String, Value>>,
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
    let (Some(attrs), Some(t)) = (attrs_obj, transform) else {
        return Ok(());
    };

    if let Some(x) = attrs.get("x_min").and_then(Value::as_f64) {
        if (x - t[0]).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'x_min' disagrees with transform".into(),
            ));
        }
    }

    if let Some(dx) = attrs.get("cell_size_x").and_then(Value::as_f64) {
        if (dx - t[1].abs()).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'cell_size_x' disagrees with transform".into(),
            ));
        }
    }

    if let Some(dy) = attrs.get("cell_size_y").and_then(Value::as_f64) {
        if (dy - t[5].abs()).abs() > TOL {
            return Err(RasterError::CorruptData(
                "conflicting geospatial metadata: 'cell_size_y' disagrees with transform".into(),
            ));
        }
    }

    if let Some(y) = attrs.get("y_min").and_then(Value::as_f64) {
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

fn parse_validation_mode_from_attrs(
    attrs_obj: Option<&serde_json::Map<String, Value>>,
) -> ValidationMode {
    let mode = attrs_obj
        .and_then(|attrs| attrs.get("zarr_validation_mode"))
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_lowercase());
    match mode.as_deref() {
        Some("lenient") => ValidationMode::Lenient,
        _ => ValidationMode::Strict,
    }
}

/// Validates that `dimension_names` (when present) do not declare an axis ordering
/// that is incompatible with the reader's band-first assumption for 3D arrays.
///
/// In strict mode, a spatial-first / band-last layout (`["y","x","band"]` etc.) is
/// rejected with a clear error.  In lenient mode the reader proceeds with its
/// default axis interpretation (band-first), which may produce incorrect results for
/// such stores.  2D stores and stores with unrecognized names are always accepted.
fn validate_dimension_names(
    names: Option<&[String]>,
    ndim: usize,
    mode: ValidationMode,
) -> Result<()> {
    let Some(names) = names else { return Ok(()); };
    // Only meaningful for 3D arrays whose dimension count matches the shape.
    if names.len() != ndim || ndim != 3 {
        return Ok(());
    }

    const SPATIAL_Y: &[&str] = &["y", "lat", "latitude", "row", "northing"];
    const SPATIAL_X: &[&str] = &["x", "lon", "longitude", "col", "easting"];
    const BAND_LIKE: &[&str] = &[
        "band", "time", "level", "depth", "pressure",
        "wavelength", "channel", "z",
    ];

    let d0 = names[0].to_ascii_lowercase();
    let d1 = names[1].to_ascii_lowercase();
    let d2 = names[2].to_ascii_lowercase();

    // Detected incompatible layout: spatial axes first, band-like axis last.
    // Without a transpose codec this would produce an incorrect band/row/col mapping.
    if SPATIAL_Y.contains(&d0.as_str())
        && SPATIAL_X.contains(&d1.as_str())
        && BAND_LIKE.contains(&d2.as_str())
    {
        if mode == ValidationMode::Strict {
            return Err(RasterError::UnsupportedDataType(format!(
                "zarr dimension_names {:?} places spatial axes before the band axis; \
                 this layout requires axis reordering which is not yet supported. \
                 Add a 'transpose' codec to reorder axes, or set \
                 zarr_validation_mode to 'lenient' to attempt a best-effort read",
                names
            )));
        }
        // Lenient: fall through and attempt the read with the default (band-first) axis
        // interpretation.  The caller is responsible for any resulting misinterpretation.
    }

    Ok(())
}

fn decode_sample(src: &[u8], dtype: DataType, endian: Endian) -> Result<f64> {
    let v = match dtype {
        DataType::U8 => src[0] as f64,
        DataType::I8 => (src[0] as i8) as f64,
        DataType::U16 => {
            let b: [u8; 2] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad u16 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u16::from_le_bytes(b) as f64,
                Endian::Big => u16::from_be_bytes(b) as f64,
            }
        }
        DataType::I16 => {
            let b: [u8; 2] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad i16 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i16::from_le_bytes(b) as f64,
                Endian::Big => i16::from_be_bytes(b) as f64,
            }
        }
        DataType::U32 => {
            let b: [u8; 4] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad u32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u32::from_le_bytes(b) as f64,
                Endian::Big => u32::from_be_bytes(b) as f64,
            }
        }
        DataType::I32 => {
            let b: [u8; 4] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad i32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i32::from_le_bytes(b) as f64,
                Endian::Big => i32::from_be_bytes(b) as f64,
            }
        }
        DataType::U64 => {
            let b: [u8; 8] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad u64 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => u64::from_le_bytes(b) as f64,
                Endian::Big => u64::from_be_bytes(b) as f64,
            }
        }
        DataType::I64 => {
            let b: [u8; 8] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad i64 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => i64::from_le_bytes(b) as f64,
                Endian::Big => i64::from_be_bytes(b) as f64,
            }
        }
        DataType::F32 => {
            let b: [u8; 4] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad f32 sample size".into()))?;
            match endian {
                Endian::Little | Endian::NativeOneByte => f32::from_le_bytes(b) as f64,
                Endian::Big => f32::from_be_bytes(b) as f64,
            }
        }
        DataType::F64 => {
            let b: [u8; 8] = src
                .try_into()
                .map_err(|_| RasterError::CorruptData("bad f64 sample size".into()))?;
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

/// Undo a zarr v3 `transpose` codec on decoded chunk values.
///
/// `order` is the permutation stored in `zarr.json` (e.g. `[1,0]` for 2D F-order).
/// `chunk_shape` is the original (C-order) shape of the decoded chunk.
/// The input `data` contains values in the encoded (transposed + C-order) layout;
/// the output contains the same values rearranged to the canonical C-order layout.
fn apply_inverse_transpose(data: &[f64], order: &[usize], chunk_shape: &[usize]) -> Vec<f64> {
    let ndim = order.len();
    debug_assert_eq!(ndim, chunk_shape.len());

    // stored_shape[i] = chunk_shape[order[i]]
    let stored_shape: Vec<usize> = (0..ndim).map(|i| chunk_shape[order[i]]).collect();

    // Strides for stored C-order array.
    let mut stored_strides = vec![1usize; ndim];
    for d in (0..ndim.saturating_sub(1)).rev() {
        stored_strides[d] = stored_strides[d + 1] * stored_shape[d + 1];
    }

    // Strides for output C-order array (chunk_shape).
    let mut orig_strides = vec![1usize; ndim];
    for d in (0..ndim.saturating_sub(1)).rev() {
        orig_strides[d] = orig_strides[d + 1] * chunk_shape[d + 1];
    }

    let n = data.len();
    let mut out = vec![0.0f64; n];
    for k in 0..n {
        let mut rem = k;
        let mut dest = 0usize;
        for i in 0..ndim {
            let coord_i = rem / stored_strides[i];
            rem %= stored_strides[i];
            // stored axis i corresponds to original axis order[i]
            dest += coord_i * orig_strides[order[i]];
        }
        out[dest] = data[k];
    }
    out
}

/// Extract a valid sub-region from a fully-padded chunk after inverse transpose.
///
/// Used for boundary chunks where `full_shape` is the complete chunk shape but only
/// `valid_shape` elements (the intersection of the chunk with the array extents) are needed.
fn extract_valid_subchunk(data: &[f64], full_shape: &[usize], valid_shape: &[usize]) -> Vec<f64> {
    let ndim = full_shape.len();
    debug_assert_eq!(ndim, valid_shape.len());

    // Strides for the full (padded) C-order array.
    let mut full_strides = vec![1usize; ndim];
    for d in (0..ndim.saturating_sub(1)).rev() {
        full_strides[d] = full_strides[d + 1] * full_shape[d + 1];
    }

    let n_valid: usize = valid_shape.iter().product();
    let mut out = Vec::with_capacity(n_valid);
    let mut coords = vec![0usize; ndim];
    loop {
        let src: usize = coords.iter().zip(full_strides.iter()).map(|(&c, &s)| c * s).sum();
        out.push(data[src]);
        // Advance coords in C-order (last axis fastest).
        let mut carry = true;
        for d in (0..ndim).rev() {
            if carry {
                coords[d] += 1;
                if coords[d] < valid_shape[d] {
                    carry = false;
                } else {
                    coords[d] = 0;
                }
            }
        }
        if carry { break; }
    }
    out
}

fn compress_bytes(compressor: &Option<(String, Option<i32>)>, raw: &[u8]) -> Result<Vec<u8>> {
    match compressor {
        None => Ok(raw.to_vec()),
        Some((name, level)) => match name.to_ascii_lowercase().as_str() {
            "zlib" => {
                let mut enc =
                    ZlibEncoder::new(Vec::new(), Compression::new(level.unwrap_or(6).clamp(0, 9) as u32));
                enc.write_all(raw)?;
                enc.finish().map_err(RasterError::Io)
            }
            "gzip" | "gz" => {
                let mut enc =
                    GzEncoder::new(Vec::new(), Compression::new(level.unwrap_or(6).clamp(0, 9) as u32));
                enc.write_all(raw)?;
                enc.finish().map_err(RasterError::Io)
            }
            "zstd" => encode_zstd(raw, level.unwrap_or(3)),
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

fn decompress_bytes(compressor: &Option<(String, Option<i32>)>, bytes: &[u8]) -> Result<Vec<u8>> {
    match compressor {
        None => Ok(bytes.to_vec()),
        Some((name, _level)) => match name.to_ascii_lowercase().as_str() {
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
