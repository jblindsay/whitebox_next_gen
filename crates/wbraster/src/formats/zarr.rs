//! Zarr format (`.zarr`) support (v2, filesystem store).

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};

/// Read a Zarr raster from a `.zarr` directory or from a `.zarray` file.
pub fn read(path: &str) -> Result<Raster> {
    let dir = resolve_zarr_dir(path)?;
    if crate::formats::zarr_v3::is_v3_store(&dir) {
        return crate::formats::zarr_v3::read_from_dir(&dir);
    }
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
    let x_min = attrs.get("x_min").and_then(Value::as_f64).unwrap_or(0.0);
    let y_min = attrs.get("y_min").and_then(Value::as_f64).unwrap_or(0.0);
    let cell_size = attrs.get("cell_size_x").and_then(Value::as_f64).unwrap_or(1.0);
    let cell_size_y = attrs.get("cell_size_y").and_then(Value::as_f64);
    let nodata = attrs.get("nodata").and_then(Value::as_f64).unwrap_or_else(|| fill_value_to_f64(meta.fill_value.as_ref()).unwrap_or(-9999.0));

    let crs = crate::crs_info::CrsInfo {
        epsg: attrs
            .get("crs_epsg")
            .and_then(Value::as_u64)
            .map(|v| v as u32)
            .or_else(|| attrs.get("epsg").and_then(Value::as_u64).map(|v| v as u32)),
        wkt: attrs
            .get("crs_wkt")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                attrs
                    .get("spatial_ref")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            }),
        proj4: attrs
            .get("crs_proj4")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| attrs.get("proj4").and_then(Value::as_str).map(ToOwned::to_owned)),
    };

    let sep = meta.dimension_separator.as_deref().unwrap_or(".");
    let n_chunk_bands = bands.div_ceil(chunk_bands);
    let n_chunk_rows = rows.div_ceil(chunk_rows);
    let n_chunk_cols = cols.div_ceil(chunk_cols);

    let mut data = vec![nodata; bands * rows * cols];
    for cb in 0..n_chunk_bands {
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
                            let band = cb * chunk_bands + bb;
                            let row = cr * chunk_rows + rr;
                            let col = cc * chunk_cols + cc2;
                            data[band * rows * cols + row * cols + col] = v;
                        }
                    }
                }
            }
        }
    }

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
    let chunk_rows = rows.clamp(1, 256);
    let chunk_cols = cols.clamp(1, 256);

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
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir() -> PathBuf {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        temp_dir().join(format!("zarr_test_{ts}.zarr"))
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
}
