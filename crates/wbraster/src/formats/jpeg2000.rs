//! JPEG 2000 / GeoJP2 adapter for wbraster.

use super::jpeg2000_core as jp2;
use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig};
use crate::crs_info::CrsInfo;

/// Default target quality used for JPEG2000 lossy output when no compression
/// option is provided.
pub const JPEG2000_DEFAULT_LOSSY_QUALITY_DB: f32 = 35.0;

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
    let jp2f = jp2::GeoJp2::open(path)
        .map_err(|e| RasterError::Other(format!("JPEG2000 read error: {e}")))?;

    let cols = jp2f.width() as usize;
    let rows = jp2f.height() as usize;
    let bands = jp2f.component_count() as usize;

    let all_chunky = jp2f
        .read_all_components()
        .map_err(|e| RasterError::Other(format!("JPEG2000 decode error: {e}")))?;
    let npix = rows * cols;
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
    let data_type = map_data_type(jp2f.pixel_type())?;
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

/// Write JPEG2000 / GeoJP2 to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    write_with_options(raster, path, &Jpeg2000WriteOptions::default())
}

/// Write JPEG2000 / GeoJP2 to `path` with typed options.
pub fn write_with_options(raster: &Raster, path: &str, opts: &Jpeg2000WriteOptions) -> Result<()> {
    let width = raster.cols as u32;
    let height = raster.rows as u32;
    let bands = raster.bands as u16;

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

fn raster_to_chunky_u8(r: &Raster) -> Vec<u8> {
    let npix = r.rows * r.cols;
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
        _ => Err(RasterError::UnsupportedDataType(format!(
            "JPEG2000 writer does not currently support {} output",
            raster.data_type
        ))),
    }
}
