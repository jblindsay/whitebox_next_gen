//! GeoTIFF / BigTIFF / COG adapter for wbraster.

use wbgeotiff as gt;
use crate::error::{RasterError, Result};
use crate::raster::{DataType, Raster, RasterConfig, RasterData};
use crate::crs_info::CrsInfo;

fn geotiff_photometric_name(p: gt::PhotometricInterpretation) -> &'static str {
    match p {
        gt::PhotometricInterpretation::MinIsWhite => "min_is_white",
        gt::PhotometricInterpretation::MinIsBlack => "min_is_black",
        gt::PhotometricInterpretation::Rgb => "rgb",
        gt::PhotometricInterpretation::Palette => "palette",
        gt::PhotometricInterpretation::Mask => "mask",
        gt::PhotometricInterpretation::Separated => "separated",
        gt::PhotometricInterpretation::YCbCr => "ycbcr",
        gt::PhotometricInterpretation::CieLab => "cielab",
    }
}

fn color_interpretation_from_geotiff(
    photometric: gt::PhotometricInterpretation,
    bands: usize,
    data_type: DataType,
) -> &'static str {
    match photometric {
        gt::PhotometricInterpretation::Rgb => {
            // Common legacy packed RGB representation in Whitebox is a single U32 band
            // storing 0xAABBGGRR values.
            if bands == 1 && data_type == DataType::U32 {
                "packed_rgb"
            } else {
                "rgb"
            }
        }
        gt::PhotometricInterpretation::Palette => "palette",
        gt::PhotometricInterpretation::YCbCr => "ycbcr",
        gt::PhotometricInterpretation::MinIsBlack | gt::PhotometricInterpretation::MinIsWhite => {
            "gray"
        }
        _ => "unknown",
    }
}

/// Typed compression choices for GeoTIFF/COG writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoTiffCompression {
    /// Uncompressed output.
    None,
    /// LZW compression.
    Lzw,
    /// Deflate/ZIP compression.
    Deflate,
    /// PackBits compression.
    PackBits,
    /// JPEG compression.
    Jpeg,
    /// WebP compression.
    WebP,
    /// JPEG-XL compression.
    JpegXl,
}

impl GeoTiffCompression {
    fn to_vendor(self) -> gt::Compression {
        match self {
            Self::None => gt::Compression::None,
            Self::Lzw => gt::Compression::Lzw,
            Self::Deflate => gt::Compression::Deflate,
            Self::PackBits => gt::Compression::PackBits,
            Self::Jpeg => gt::Compression::Jpeg,
            Self::WebP => gt::Compression::WebP,
            Self::JpegXl => gt::Compression::JpegXl,
        }
    }
}

/// Typed layout/mode choices for GeoTIFF/COG writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoTiffLayout {
    /// Standard GeoTIFF writer with default layout behavior.
    Standard,
    /// Stripped GeoTIFF layout.
    Stripped {
        /// Rows per strip.
        rows_per_strip: u32,
    },
    /// Tiled GeoTIFF layout.
    Tiled {
        /// Tile width in pixels.
        tile_width: u32,
        /// Tile height in pixels.
        tile_height: u32,
    },
    /// Cloud-Optimized GeoTIFF (COG) writer.
    Cog {
        /// Tile size in pixels.
        tile_size: u32,
    },
}

/// Typed write options for GeoTIFF / BigTIFF / COG output.
///
/// Any field left as `None` falls back to legacy metadata keys (if present),
/// then to built-in defaults.
#[derive(Debug, Clone, Default)]
pub struct GeoTiffWriteOptions {
    /// Output compression codec.
    pub compression: Option<GeoTiffCompression>,
    /// Whether to force BigTIFF container.
    pub bigtiff: Option<bool>,
    /// Output layout/mode.
    pub layout: Option<GeoTiffLayout>,
}

/// Typed write options focused specifically on COG output.
///
/// Any field left as `None` uses COG convenience defaults.
#[derive(Debug, Clone, Default)]
pub struct CogWriteOptions {
    /// Output compression codec.
    pub compression: Option<GeoTiffCompression>,
    /// Whether to force BigTIFF container.
    pub bigtiff: Option<bool>,
    /// COG tile size in pixels.
    pub tile_size: Option<u32>,
}

/// Read GeoTIFF / BigTIFF / COG from `path`.
pub fn read(path: &str) -> Result<Raster> {
    let tiff = gt::GeoTiff::open(path)
        .map_err(|e| RasterError::Other(format!("GeoTIFF read error: {e}")))?;

    let cols = tiff.width() as usize;
    let rows = tiff.height() as usize;
    let bands = tiff.band_count();

    let mut x_min = 0.0;
    let mut y_min = 0.0;
    let mut cell_size = 1.0;
    let mut cell_size_y = Some(1.0);
    if let Some(gtx) = tiff.geo_transform() {
        x_min = gtx.x_origin;
        cell_size = gtx.pixel_width.abs();
        cell_size_y = Some(gtx.pixel_height.abs());
        let y_max = gtx.y_origin;
        y_min = y_max + gtx.pixel_height * rows as f64;
    }

    let crs = CrsInfo {
        epsg: tiff.epsg().map(|v| v as u32),
        ..Default::default()
    };

    let nodata = tiff.no_data().unwrap_or(-9999.0);
    let mut data_type = map_data_type(tiff.sample_format(), tiff.bits_per_sample())?;
    let mut data = read_native_data(&tiff, data_type, rows * cols, bands)?;
    let photometric = tiff.photometric();

    let mut metadata = vec![
        ("geotiff_is_bigtiff".into(), tiff.is_bigtiff.to_string()),
        (
            "geotiff_compression".into(),
            tiff.compression().name().to_ascii_lowercase(),
        ),
        (
            "geotiff_photometric".into(),
            geotiff_photometric_name(photometric).to_string(),
        ),
        (
            "color_interpretation".into(),
            color_interpretation_from_geotiff(photometric, bands, data_type).to_string(),
        ),
    ];
    if let Some(v) = metadata_bool_from_name(tiff.compression().name()) {
        metadata.push(("geotiff_is_cog_candidate".into(), v.to_string()));
    }

    let mut total_scale = 1.0f64;
    let mut total_offset = 0.0f64;
    let mut applied_transform = false;

    if let Some(vt) = tiff.value_transform() {
        total_scale *= vt.scale;
        total_offset = total_offset * vt.scale + vt.offset;
        metadata.push(("geotiff_value_scale".into(), vt.scale.to_string()));
        metadata.push(("geotiff_value_offset".into(), vt.offset.to_string()));
        applied_transform = true;
    }

    if let Some(vertical_scale) = vertical_units_to_meters_factor(&tiff) {
        if (vertical_scale - 1.0).abs() > f64::EPSILON {
            total_scale *= vertical_scale;
            total_offset *= vertical_scale;
            metadata.push((
                "geotiff_vertical_units_to_meters_scale".into(),
                vertical_scale.to_string(),
            ));
            metadata.push(("values_normalized_to_vertical_meters".into(), "true".into()));
            applied_transform = true;
        }
    }

    if applied_transform
        && ((total_scale - 1.0).abs() > f64::EPSILON || total_offset.abs() > f64::EPSILON)
    {
        data = apply_linear_value_transform(data, nodata, total_scale, total_offset);
        data_type = DataType::F64;
        metadata.push(("geotiff_values_normalized".into(), "true".into()));
        metadata.push(("geotiff_values_total_scale".into(), total_scale.to_string()));
        metadata.push(("geotiff_values_total_offset".into(), total_offset.to_string()));
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
        data_type,
        crs: crs,        metadata,
    };

    Raster::from_data_native(cfg, data)
}

/// Write GeoTIFF / BigTIFF / COG to `path`.
pub fn write(raster: &Raster, path: &str) -> Result<()> {
    write_with_options(raster, path, &GeoTiffWriteOptions::default())
}

/// Write GeoTIFF / BigTIFF / COG to `path` with typed options.
///
/// This is the only configuration path for non-default GeoTIFF/COG writes.
/// Metadata-based write controls are no longer consumed.
pub fn write_with_options(raster: &Raster, path: &str, opts: &GeoTiffWriteOptions) -> Result<()> {
    let width = raster.cols as u32;
    let height = raster.rows as u32;
    let bands = raster.bands as u16;

    let compression = opts
        .compression
        .map(GeoTiffCompression::to_vendor)
        .unwrap_or(gt::Compression::Deflate);
    let bigtiff = opts.bigtiff.unwrap_or(false);
    let layout = opts.layout.unwrap_or(GeoTiffLayout::Standard);

    let epsg = raster.crs.epsg.and_then(|v| u16::try_from(v).ok());
    let gt_xform = gt::GeoTransform::north_up(
        raster.x_min,
        raster.cell_size_x,
        raster.y_max(),
        -raster.cell_size_y,
    );

    if let GeoTiffLayout::Cog { tile_size } = layout {
        let mut writer = gt::CogWriter::new(width, height, bands)
            .compression(compression)
            .bigtiff(bigtiff)
            .geo_transform(gt_xform)
            .no_data(raster.nodata);
        writer = writer.tile_size(tile_size);
        if let Some(e) = epsg {
            writer = writer.epsg(e);
        }
        return write_with_cog(writer, path, raster);
    }

    let mut writer = gt::GeoTiffWriter::new(width, height, bands)
        .compression(compression)
        .bigtiff(bigtiff)
        .geo_transform(gt_xform)
        .no_data(raster.nodata);

    if let Some(e) = epsg {
        writer = writer.epsg(e);
    }

    match layout {
        GeoTiffLayout::Tiled {
            tile_width,
            tile_height,
        } => {
            writer = writer.layout(gt::WriteLayout::Tiled {
                tile_width,
                tile_height,
            });
        }
        GeoTiffLayout::Stripped { rows_per_strip } => {
            writer = writer.layout(gt::WriteLayout::Stripped { rows_per_strip });
        }
        GeoTiffLayout::Standard | GeoTiffLayout::Cog { .. } => {}
    }

    write_with_writer(writer, path, raster)
}

fn map_data_type(sample_format: gt::SampleFormat, bits: u16) -> Result<DataType> {
    match (sample_format, bits) {
        (gt::SampleFormat::Uint, 8) => Ok(DataType::U8),
        (gt::SampleFormat::Uint, 16) => Ok(DataType::U16),
        (gt::SampleFormat::Uint, 32) => Ok(DataType::U32),
        (gt::SampleFormat::Uint, 64) => Ok(DataType::U64),
        (gt::SampleFormat::Int, 8) => Ok(DataType::I8),
        (gt::SampleFormat::Int, 16) => Ok(DataType::I16),
        (gt::SampleFormat::Int, 32) => Ok(DataType::I32),
        (gt::SampleFormat::Int, 64) => Ok(DataType::I64),
        (gt::SampleFormat::IeeeFloat, 32) => Ok(DataType::F32),
        (gt::SampleFormat::IeeeFloat, 64) => Ok(DataType::F64),
        _ => Err(RasterError::UnsupportedDataType(format!(
            "unsupported GeoTIFF sample format/bits combo: format={:?}, bits={bits}",
            sample_format
        ))),
    }
}

fn read_native_data(tiff: &gt::GeoTiff, data_type: DataType, npix: usize, bands: usize) -> Result<RasterData> {
    match data_type {
        DataType::U8 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_u8(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::U8(out))
        }
        DataType::I8 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_i8(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::I8(out))
        }
        DataType::U16 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_u16(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::U16(out))
        }
        DataType::I16 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_i16(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::I16(out))
        }
        DataType::U32 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_u32(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::U32(out))
        }
        DataType::I32 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_i32(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::I32(out))
        }
        DataType::U64 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_u64(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::U64(out))
        }
        DataType::I64 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_i64(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::I64(out))
        }
        DataType::F32 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_f32(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::F32(out))
        }
        DataType::F64 => {
            let mut out = Vec::with_capacity(npix * bands);
            for band in 0..bands {
                out.extend(
                    tiff.read_band_f64(band)
                        .map_err(|e| RasterError::Other(format!("GeoTIFF decode error: {e}")))?,
                );
            }
            Ok(RasterData::F64(out))
        }
    }
}

fn metadata_bool_from_name(name: &str) -> Option<bool> {
    match name.to_ascii_lowercase().as_str() {
        "jpeg" | "deflate" | "lzw" | "packbits" | "webp" | "jpeg-xl" => Some(true),
        "none" => Some(false),
        _ => None,
    }
}

fn vertical_units_to_meters_factor(tiff: &gt::GeoTiff) -> Option<f64> {
    let keys = tiff.geo_keys()?;
    let gt::geo_keys::GeoKeyValue::Short(unit_code) =
        keys.get(gt::geo_keys::key::VerticalUnitsGeoKey)?
    else {
        return None;
    };

    // EPSG unit codes commonly used in GeoTIFF GeoKeys.
    let factor = match *unit_code {
        9001 => 1.0,                // metre
        9002 => 0.3048,             // foot (international)
        9003 => 1200.0 / 3937.0,    // US survey foot
        _ => return None,
    };
    Some(factor)
}

fn apply_linear_value_transform(data: RasterData, nodata: f64, scale: f64, offset: f64) -> RasterData {
    let nodata_is_nan = nodata.is_nan();
    let mut out = data.to_f64_vec();
    for value in &mut out {
        let is_nodata = if nodata_is_nan {
            value.is_nan()
        } else {
            (*value - nodata).abs() <= f64::EPSILON
        };
        if !is_nodata {
            *value = *value * scale + offset;
        }
    }
    RasterData::F64(out)
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

fn raster_to_chunky_i8(r: &Raster) -> Vec<i8> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_i8() {
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
            out.push(v as i8);
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

fn raster_to_chunky_u32(r: &Raster) -> Vec<u32> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_u32() {
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
            out.push(v as u32);
        }
    }
    out
}

fn raster_to_chunky_i32(r: &Raster) -> Vec<i32> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_i32() {
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
            out.push(v as i32);
        }
    }
    out
}

fn raster_to_chunky_u64(r: &Raster) -> Vec<u64> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_u64() {
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
            out.push(v as u64);
        }
    }
    out
}

fn raster_to_chunky_i64(r: &Raster) -> Vec<i64> {
    let npix = r.rows * r.cols;
    if let Some(data) = r.data_i64() {
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
            out.push(v as i64);
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

fn write_with_writer(writer: gt::GeoTiffWriter, path: &str, raster: &Raster) -> Result<()> {
    match raster.data_type {
        DataType::I8 => writer
            .write_i8(path, &raster_to_chunky_i8(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::U8 => writer
            .write_u8(path, &raster_to_chunky_u8(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::U16 => writer
            .write_u16(path, &raster_to_chunky_u16(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::U32 => writer
            .write_u32(path, &raster_to_chunky_u32(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::I16 => writer
            .write_i16(path, &raster_to_chunky_i16(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::I32 => writer
            .write_i32(path, &raster_to_chunky_i32(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::U64 => writer
            .write_u64(path, &raster_to_chunky_u64(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::I64 => writer
            .write_i64(path, &raster_to_chunky_i64(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::F32 => writer
            .write_f32(path, &raster_to_chunky_f32(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
        DataType::F64 => writer
            .write_f64(path, &raster_to_chunky_f64(raster))
            .map_err(|e| RasterError::Other(format!("GeoTIFF write error: {e}"))),
    }
}

fn write_with_cog(writer: gt::CogWriter, path: &str, raster: &Raster) -> Result<()> {
    match raster.data_type {
        DataType::I8 => writer
            .write_i8(path, &raster_to_chunky_i8(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::U8 => writer
            .write_u8(path, &raster_to_chunky_u8(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::U16 => writer
            .write_u16(path, &raster_to_chunky_u16(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::U32 => writer
            .write_u32(path, &raster_to_chunky_u32(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::I16 => writer
            .write_i16(path, &raster_to_chunky_i16(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::I32 => writer
            .write_i32(path, &raster_to_chunky_i32(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::U64 => writer
            .write_u64(path, &raster_to_chunky_u64(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::I64 => writer
            .write_i64(path, &raster_to_chunky_i64(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::F32 => writer
            .write_f32(path, &raster_to_chunky_f32(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
        DataType::F64 => writer
            .write_f64(path, &raster_to_chunky_f64(raster))
            .map_err(|e| RasterError::Other(format!("COG write error: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(suffix: &str) -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        temp_dir()
            .join(format!("geotiff_test_{ts}{suffix}"))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn geotiff_roundtrip_multiband() {
        let path = tmp(".tif");
        let cfg = RasterConfig {
            cols: 5,
            rows: 4,
            bands: 3,
            x_min: 100.0,
            y_min: -30.0,
            cell_size: 0.5,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        };
        let data: Vec<f64> = (0..(cfg.cols * cfg.rows * cfg.bands))
            .map(|i| if i == 7 { -9999.0 } else { i as f64 * 0.25 })
            .collect();
        let mut r = Raster::from_data(cfg, data).unwrap();
        r.metadata.push(("geotiff_compression".into(), "deflate".into()));

        write(&r, &path).unwrap();
        let r2 = read(&path).unwrap();

        assert_eq!(r2.cols, r.cols);
        assert_eq!(r2.rows, r.rows);
        assert_eq!(r2.bands, r.bands);
        for b in 0..r.bands {
            for row in 0..r.rows {
                for col in 0..r.cols {
                    let a = r.get_raw(b as isize, row as isize, col as isize).unwrap();
                    let b2 = r2.get_raw(b as isize, row as isize, col as isize).unwrap();
                    if r.is_nodata(a) {
                        assert!(r2.is_nodata(b2));
                    } else {
                        assert!((a - b2).abs() < 1e-4);
                    }
                }
            }
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn geotiff_roundtrip_i8() {
        let path = tmp("_i8.tif");
        let cfg = RasterConfig {
            cols: 4,
            rows: 3,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -128.0,
            data_type: DataType::I8,
            ..Default::default()
        };
        let data = vec![-128.0, -2.0, -1.0, 0.0, 1.0, 2.0, 7.0, 12.0, 25.0, 63.0, 64.0, 127.0];
        let r = Raster::from_data(cfg, data.clone()).unwrap();

        write(&r, &path).unwrap();
        let r2 = read(&path).unwrap();

        assert_eq!(r2.data_type, DataType::I8);
        for row in 0..r.rows {
            for col in 0..r.cols {
                let expected = data[row * r.cols + col];
                let actual = r2.get_raw(0, row as isize, col as isize).unwrap();
                assert_eq!(expected as i8, actual as i8);
            }
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn geotiff_roundtrip_u64() {
        let path = tmp("_u64.tif");
        let cfg = RasterConfig {
            cols: 4,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: 0.0,
            data_type: DataType::U64,
            ..Default::default()
        };
        let data = RasterData::U64(vec![0, 1, 255, 65_535, 1_000_000, 9_007_199_254_740_991, u64::MAX - 1, u64::MAX]);
        let r = Raster::from_data_native(cfg, data.clone()).unwrap();

        write(&r, &path).unwrap();
        let r2 = read(&path).unwrap();

        assert_eq!(r2.data_type, DataType::U64);
        assert_eq!(r2.data_u64().unwrap(), match &data { RasterData::U64(values) => values.as_slice(), _ => unreachable!() });

        let _ = std::fs::remove_file(path);
    }
}
