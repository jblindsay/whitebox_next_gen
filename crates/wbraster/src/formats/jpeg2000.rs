//! JPEG 2000 / GeoJP2 adapter for wbraster.

use super::jpeg2000_core as jp2;
use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

/// Default target quality used for JPEG2000 lossy output when no compression
/// option is provided.
pub const JPEG2000_DEFAULT_LOSSY_QUALITY_DB: f32 = 35.0;

/// Public JPEG2000 color-space type used by write options.
pub type Jpeg2000ColorSpace = jp2::ColorSpace;

fn color_interpretation_from_jpeg2000(
    color_space: jp2::ColorSpace,
    bands: usize,
    data_type: DataType,
) -> &'static str {
    match color_space {
        jp2::ColorSpace::Srgb => {
            if bands == 1 && data_type == DataType::U32 {
                "packed_rgb"
            } else {
                "rgb"
            }
        }
        jp2::ColorSpace::YCbCr => "ycbcr",
        jp2::ColorSpace::Greyscale => "gray",
        jp2::ColorSpace::MultiBand => "multiband",
    }
}

fn metadata_value_case_insensitive<'a>(
    metadata: &'a [(String, String)],
    key: &str,
) -> Option<&'a str> {
    metadata
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn raster_is_packed_rgb(raster: &Raster) -> bool {
    if raster.bands != 1 || raster.data_type != DataType::U32 {
        return false;
    }
    let color_interp =
        metadata_value_case_insensitive(&raster.metadata, "color_interpretation").unwrap_or("");
    color_interp.eq_ignore_ascii_case("packed_rgb")
}

/// Unpack a packed-RGB (`0xAABBGGRR`) single-band U32 raster into 3-band chunky U8 (R, G, B).
fn raster_to_chunky_u8_from_packed_rgb(r: &Raster) -> Vec<u8> {
    let npix = r.rows * r.cols;
    let mut out = Vec::with_capacity(npix * 3);
    if let Some(buf) = r.data_u32() {
        for &packed in buf.iter().take(npix) {
            out.push( (packed        & 0xFF) as u8);  // R
            out.push(((packed >>  8) & 0xFF) as u8);  // G
            out.push(((packed >> 16) & 0xFF) as u8);  // B
        }
    } else {
        // Fallback: raster backed by a non-native store (memory://f64 etc.)
        for p in 0..npix {
            let row = p / r.cols;
            let col = p % r.cols;
            let packed = r.get_raw(0, row as isize, col as isize).unwrap_or(0.0) as u32;
            out.push( (packed        & 0xFF) as u8);
            out.push(((packed >>  8) & 0xFF) as u8);
            out.push(((packed >> 16) & 0xFF) as u8);
        }
    }
    out
}

/// Typed compression choices for JPEG2000 writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Jpeg2000Compression {
    /// Reversible 5/3 wavelet compression.
    Lossless,
    /// Irreversible 9/7 wavelet compression with a target quality (dB).
    Lossy {
        /// Target quality in dB.
        quality_db: f32,
    },
}

impl Jpeg2000Compression {
    fn to_core(self) -> jp2::CompressionMode {
        match self {
            Self::Lossless => jp2::CompressionMode::Lossless,
            Self::Lossy { quality_db } => jp2::CompressionMode::Lossy { quality_db },
        }
    }
}

/// Typed write options for JPEG2000 / GeoJP2 output.
#[derive(Debug, Clone, Default)]
pub struct Jpeg2000WriteOptions {
    /// Compression mode.
    pub compression: Option<Jpeg2000Compression>,
    /// Number of decomposition levels.
    pub decomp_levels: Option<u8>,
    /// Optional color space override.
    pub color_space: Option<jp2::ColorSpace>,
}

/// Read JPEG2000 / GeoJP2 from `path`.
pub fn read(path: &str) -> Result<Raster> {
    eprintln!("[jpeg2000::read] path={}", path);
    let jp2f = jp2::GeoJp2::open(path)
        .map_err(|e| RasterError::Other(format!("JPEG2000 read error: {e}")))?;

    let cols = jp2f.width() as usize;
    let rows = jp2f.height() as usize;

    let (bands, data_type, data) = {
        eprintln!("[jpeg2000::read] Using native decoder");
        decode_samples_with_internal_reader(&jp2f, rows, cols)?
    };

    let mut x_min = 0.0;
    let mut y_min = 0.0;
    let mut cell_size = 1.0;
    let mut cell_size_y = Some(1.0);
    if let Some(gtx) = jp2f.geo_transform() {
        x_min = gtx.x_origin;
        cell_size = gtx.pixel_width.abs();
        cell_size_y = Some(gtx.pixel_height.abs());
        let y_max = gtx.y_origin;
        y_min = y_max + gtx.pixel_height * rows as f64;
    }

    let crs = CrsInfo {
        epsg: jp2f.epsg().map(u32::from),
        ..Default::default()
    };

    let nodata = jp2f.no_data().unwrap_or(-9999.0);
    let data_type = data_type;
    let color_space = jp2f.color_space();

    let metadata = vec![
        (
            "jpeg2000_compression".into(),
            if jp2f.is_lossless() {
                "lossless".into()
            } else {
                "lossy".into()
            },
        ),
        (
            "jpeg2000_color_space".into(),
            format!("{:?}", color_space).to_ascii_lowercase(),
        ),
        (
            "color_interpretation".into(),
            color_interpretation_from_jpeg2000(color_space, bands, data_type).to_string(),
        ),
    ];

    let cfg = RasterConfig {
        cols,
        rows,
        bands,
        x_min,
        y_min,
        cell_size,
        cell_size_y,
        nodata,
        data_type,
        crs: crs,        metadata,
    };

    Raster::from_data(cfg, data)
}

fn decode_samples_with_internal_reader(
    jp2f: &jp2::GeoJp2,
    rows: usize,
    cols: usize,
) -> Result<(usize, DataType, Vec<f64>)> {
    let bands = jp2f.component_count() as usize;
    let npix = rows * cols;

    let all_chunky = jp2f
        .read_all_components()
        .map_err(|e| RasterError::Other(format!("JPEG2000 decode error: {e}")))?;
    if all_chunky.len() != npix * bands {
        return Err(RasterError::CorruptData(format!(
            "JPEG2000 decoded sample count mismatch: expected {}, got {}",
            npix * bands,
            all_chunky.len()
        )));
    }

    let mut data = vec![0.0; npix * bands];
    for p in 0..npix {
        for b in 0..bands {
            data[b * npix + p] = all_chunky[p * bands + b] as f64;
        }
    }

    if std::env::var("JPEG2000_DEBUG_NATIVE_HEAD").is_ok() && npix > 0 {
        let head = 10.min(npix);
        eprintln!(
            "[native] first {} pixels (component 0): {:?}",
            head,
            &data[0..head]
        );
    }

    let data_type = map_data_type(jp2f.pixel_type())?;
    Ok((bands, data_type, data))
}

/// Write JPEG2000 / GeoJP2 to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    write_with_options(raster, path, &Jpeg2000WriteOptions::default())
}

/// Write JPEG2000 / GeoJP2 to `path` with typed options.
pub fn write_with_options(raster: &Raster, path: &str, opts: &Jpeg2000WriteOptions) -> Result<()> {
    let width = raster.cols as u32;
    let height = raster.rows as u32;
    let is_packed_rgb = raster_is_packed_rgb(raster);
    let bands = if is_packed_rgb { 3u16 } else { raster.bands as u16 };

    let compression = opts
        .compression
        .unwrap_or(Jpeg2000Compression::Lossy {
            quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
        })
        .to_core();

    let epsg = raster.crs.epsg.and_then(|v| u16::try_from(v).ok());
    let gt_xform = jp2::GeoTransform::north_up(
        raster.x_min,
        raster.cell_size_x,
        raster.y_max(),
        -raster.cell_size_y,
    );

    let mut writer = jp2::GeoJp2Writer::new(width, height, bands)
        .compression(compression)
        .geo_transform(gt_xform)
        .no_data(raster.nodata);

    if let Some(levels) = opts.decomp_levels {
        writer = writer.decomp_levels(levels);
    }
    if is_packed_rgb && opts.color_space.is_none() {
        writer = writer.color_space(jp2::ColorSpace::Srgb);
    }
    if let Some(color_space) = opts.color_space {
        writer = writer.color_space(color_space);
    }
    if let Some(code) = epsg {
        writer = writer.epsg(code);
    }

    write_with_writer(writer, path, raster)
}

fn map_data_type(pixel_type: jp2::PixelType) -> Result<DataType> {
    match pixel_type {
        jp2::PixelType::Uint8 => Ok(DataType::U8),
        jp2::PixelType::Uint16 => Ok(DataType::U16),
        jp2::PixelType::Int16 => Ok(DataType::I16),
        jp2::PixelType::Int32 => Ok(DataType::I32),
        jp2::PixelType::Float32 => Ok(DataType::F32),
        jp2::PixelType::Float64 => Ok(DataType::F64),
    }
}

fn interleave_band_major<T: Copy>(data: &[T], npix: usize, bands: usize) -> Vec<T> {
    if bands <= 1 {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    for p in 0..npix {
        for b in 0..bands {
            out.push(data[b * npix + p]);
        }
    }
    out
}

fn raster_to_chunky_u8(r: &Raster) -> Vec<u8> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_u8() {
        return interleave_band_major(data, npix, r.bands);
    }
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as u8);
        }
    }
    out
}

fn raster_to_chunky_u16(r: &Raster) -> Vec<u16> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_u16() {
        return interleave_band_major(data, npix, r.bands);
    }
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as u16);
        }
    }
    out
}

fn raster_to_chunky_i16(r: &Raster) -> Vec<i16> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_i16() {
        return interleave_band_major(data, npix, r.bands);
    }
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as i16);
        }
    }
    out
}

fn raster_to_chunky_f32(r: &Raster) -> Vec<f32> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_f32() {
        return interleave_band_major(data, npix, r.bands);
    }
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v as f32);
        }
    }
    out
}

fn raster_to_chunky_f64(r: &Raster) -> Vec<f64> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_f64() {
        return interleave_band_major(data, npix, r.bands);
    }
    let mut out = Vec::with_capacity(npix * r.bands);
    for p in 0..npix {
        let row = p / r.cols;
        let col = p % r.cols;
        for b in 0..r.bands {
            let v = r
                .get_raw(b as isize, row as isize, col as isize)
                .unwrap_or(r.nodata);
            out.push(v);
        }
    }
    out
}

fn write_with_writer(writer: jp2::GeoJp2Writer, path: &str, raster: &Raster) -> Result<()> {
    match raster.data_type {
        DataType::U8 => writer
            .write_u8(path, &raster_to_chunky_u8(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::U16 => writer
            .write_u16(path, &raster_to_chunky_u16(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::I16 => writer
            .write_i16(path, &raster_to_chunky_i16(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::F32 => writer
            .write_f32(path, &raster_to_chunky_f32(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::F64 => writer
            .write_f64(path, &raster_to_chunky_f64(raster))
            .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}"))),
        DataType::U32 => {
            if raster_is_packed_rgb(raster) {
                writer
                    .write_u8(path, &raster_to_chunky_u8_from_packed_rgb(raster))
                    .map_err(|e| RasterError::Other(format!("JPEG2000 write error: {e}")))
            } else {
                Err(RasterError::UnsupportedDataType(format!(
                    "JPEG2000 writer does not currently support {} output",
                    raster.data_type
                )))
            }
        }
        _ => Err(RasterError::UnsupportedDataType(format!(
            "JPEG2000 writer does not currently support {} output",
            raster.data_type
        ))),
    }
}



