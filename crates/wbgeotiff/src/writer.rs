//! GeoTIFF writer supporting stripped, tiled, and BigTIFF output.
//!
//! Use [`GeoTiffWriter`] for a simple builder-style API.  For Cloud Optimised
//! GeoTIFF output see the [`cog`](super::cog) module.

#![allow(dead_code)]

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::mem;
use std::path::Path;
use std::slice;

use rayon::prelude::*;

use super::compression;
use super::error::{GeoTiffError, Result};
use super::geo_keys::{GeoKeyBuilder, GeoKeyDirectory};
use super::ifd::ByteOrder;
use super::tags::{tag, Compression, PhotometricInterpretation, PlanarConfig, SampleFormat};
use super::types::GeoTransform;

/// Always write little-endian.
const BO: ByteOrder = ByteOrder::LittleEndian;
const WRITE_BUFFER_CAPACITY: usize = 1 << 20;

trait LeByteEncode {
    const WIDTH: usize;

    fn append_le_bytes(&self, out: &mut Vec<u8>);
}

macro_rules! impl_le_byte_encode {
    ($($t:ty),+ $(,)?) => {
        $(
            impl LeByteEncode for $t {
                const WIDTH: usize = mem::size_of::<$t>();

                fn append_le_bytes(&self, out: &mut Vec<u8>) {
                    out.extend_from_slice(&self.to_le_bytes());
                }
            }
        )+
    };
}

impl_le_byte_encode!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

#[inline]
fn slice_as_le_bytes<T: LeByteEncode>(data: &[T]) -> Cow<'_, [u8]> {
    #[cfg(target_endian = "little")]
    {
        Cow::Borrowed(unsafe {
            slice::from_raw_parts(data.as_ptr() as *const u8, mem::size_of_val(data))
        })
    }

    #[cfg(not(target_endian = "little"))]
    {
        let mut out = Vec::with_capacity(data.len() * T::WIDTH);
        for value in data {
            value.append_le_bytes(&mut out);
        }
        Cow::Owned(out)
    }
}

#[inline]
fn new_output_writer<P: AsRef<Path>>(path: P) -> Result<BufWriter<File>> {
    let file = File::create(path).map_err(GeoTiffError::Io)?;
    Ok(BufWriter::with_capacity(WRITE_BUFFER_CAPACITY, file))
}

// ── WriteLayout ───────────────────────────────────────────────────────────────

/// Whether to organise image data as strips or tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteLayout {
    /// Horizontal strips, `rows_per_strip` rows each (default: 256).
    Stripped {
        /// Number of raster rows stored per strip.
        rows_per_strip: u32,
    },
    /// Fixed-size tiles, typically 256×256 or 512×512.
    Tiled {
        /// Tile width in pixels.
        tile_width: u32,
        /// Tile height in pixels.
        tile_height: u32,
    },
}

impl Default for WriteLayout {
    fn default() -> Self { Self::Stripped { rows_per_strip: 256 } }
}

// ── GeoTiffWriter ─────────────────────────────────────────────────────────────

/// Builder for writing GeoTIFF files (stripped, tiled, or BigTIFF).
///
/// # Examples
///
/// ```rust,ignore
/// use wbraster::formats::geotiff_core::{GeoTiffWriter, GeoTransform, Compression, SampleFormat, WriteLayout};
///
/// // Tiled f32 GeoTIFF with LZW compression
/// let data: Vec<f32> = vec![1.0; 1024 * 1024];
/// GeoTiffWriter::new(1024, 1024, 1)
///     .compression(Compression::Lzw)
///     .sample_format(SampleFormat::IeeeFloat)
///     .layout(WriteLayout::Tiled { tile_width: 256, tile_height: 256 })
///     .geo_transform(GeoTransform::north_up(0.0, 0.001, 1.0, -0.001))
///     .epsg(4326)
///     .write_f32("tiled.tif", &data)
///     .unwrap();
///
/// // BigTIFF strip output for very large files
/// GeoTiffWriter::new(100_000, 100_000, 1)
///     .bigtiff(true)
///     .compression(Compression::Deflate)
///     .write_f32("big.tif", &vec![0.0f32; 100_000 * 100_000])
///     .unwrap();
/// ```
pub struct GeoTiffWriter {
    pub(crate) width:        u32,
    pub(crate) height:       u32,
    pub(crate) bands:        u16,
    pub(crate) bits_per_sample: u16,
    pub(crate) sample_format:   SampleFormat,
    pub(crate) compression:     Compression,
    pub(crate) photometric:     PhotometricInterpretation,
    pub(crate) planar_config:   PlanarConfig,
    pub(crate) layout:          WriteLayout,
    pub(crate) geo_transform:   Option<GeoTransform>,
    pub(crate) geo_keys:        Option<GeoKeyDirectory>,
    pub(crate) no_data:         Option<f64>,
    pub(crate) software:        Option<String>,
    pub(crate) jpeg_quality:    u8,
    /// Write as BigTIFF (magic=43, 8-byte offsets).  Recommended for files >4 GB.
    pub(crate) bigtiff:         bool,
    /// SubFileType tag value (0 = main image, 1 = overview).
    pub(crate) sub_file_type:   u32,
}

#[allow(missing_docs)]
impl GeoTiffWriter {
    /// Create a new writer for a `width × height × bands` raster.
    pub fn new(width: u32, height: u32, bands: u16) -> Self {
        Self {
            width, height, bands,
            bits_per_sample: 8,
            sample_format:   SampleFormat::Uint,
            compression:     Compression::None,
            photometric:     PhotometricInterpretation::MinIsBlack,
            planar_config:   PlanarConfig::Chunky,
            layout:          WriteLayout::default(),
            geo_transform:   None,
            geo_keys:        None,
            no_data:         None,
            software:        Some("geotiff-rs".into()),
            jpeg_quality:    85,
            bigtiff:         false,
            sub_file_type:   0,
        }
    }

    // ── Builder setters ───────────────────────────────────────────────────────

    pub fn compression(mut self, c: Compression)              -> Self { self.compression = c; self }
    pub fn sample_format(mut self, sf: SampleFormat)          -> Self { self.sample_format = sf; self }
    pub fn bits_per_sample(mut self, bps: u16)                -> Self { self.bits_per_sample = bps; self }
    pub fn photometric(mut self, p: PhotometricInterpretation) -> Self { self.photometric = p; self }
    pub fn layout(mut self, l: WriteLayout)                   -> Self { self.layout = l; self }
    pub fn geo_transform(mut self, gt: GeoTransform)          -> Self { self.geo_transform = Some(gt); self }
    pub fn geo_key_directory(mut self, gkd: GeoKeyDirectory)  -> Self { self.geo_keys = Some(gkd); self }
    pub fn no_data(mut self, v: f64)                          -> Self { self.no_data = Some(v); self }
    pub fn software(mut self, s: impl Into<String>)            -> Self { self.software = Some(s.into()); self }
    /// Set JPEG quality in range 1..=100 (used when `Compression::Jpeg` is selected).
    pub fn jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = quality.clamp(1, 100);
        self
    }
    /// Enable BigTIFF output (required for files > ~4 GB).
    pub fn bigtiff(mut self, b: bool)                         -> Self { self.bigtiff = b; self }

    /// Rows-per-strip shortcut (sets `layout` to Stripped).
    pub fn rows_per_strip(mut self, rps: u32) -> Self {
        self.layout = WriteLayout::Stripped { rows_per_strip: rps.max(1) };
        self
    }

    /// Tile size shortcut (sets `layout` to Tiled).
    pub fn tile_size(mut self, tw: u32, th: u32) -> Self {
        self.layout = WriteLayout::Tiled { tile_width: tw, tile_height: th };
        self
    }

    /// Convenience: set EPSG code.
    pub fn epsg(mut self, epsg: u16) -> Self {
        let gkd = if epsg / 1000 == 4 {
            GeoKeyBuilder::new().geographic_epsg(epsg).build()
        } else {
            GeoKeyBuilder::new().projected_epsg(epsg).build()
        };
        self.geo_keys = Some(gkd);
        self
    }

    // ── Typed write methods ───────────────────────────────────────────────────

    pub fn write_u8<P: AsRef<Path>>(mut self, path: P, data: &[u8]) -> Result<()> {
        self.bits_per_sample = 8; self.sample_format = SampleFormat::Uint;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_i8<P: AsRef<Path>>(mut self, path: P, data: &[i8]) -> Result<()> {
        self.bits_per_sample = 8; self.sample_format = SampleFormat::Int;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_u16<P: AsRef<Path>>(mut self, path: P, data: &[u16]) -> Result<()> {
        self.bits_per_sample = 16; self.sample_format = SampleFormat::Uint;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_u32<P: AsRef<Path>>(mut self, path: P, data: &[u32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::Uint;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_u64<P: AsRef<Path>>(mut self, path: P, data: &[u64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::Uint;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_i16<P: AsRef<Path>>(mut self, path: P, data: &[i16]) -> Result<()> {
        self.bits_per_sample = 16; self.sample_format = SampleFormat::Int;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_i32<P: AsRef<Path>>(mut self, path: P, data: &[i32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::Int;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_i64<P: AsRef<Path>>(mut self, path: P, data: &[i64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::Int;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_f32<P: AsRef<Path>>(mut self, path: P, data: &[f32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::IeeeFloat;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    pub fn write_f64<P: AsRef<Path>>(mut self, path: P, data: &[f64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::IeeeFloat;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(new_output_writer(path)?, bytes.as_ref())
    }

    /// Write f32 data into any `Write + Seek` (useful for in-memory buffers / COG).
    pub fn write_f32_to_writer<W: Write + Seek>(mut self, w: W, data: &[f32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::IeeeFloat;
        self.validate(data.len())?;
        let bytes = slice_as_le_bytes(data);
        self.write_raw(w, bytes.as_ref())
    }

    // ── Validation ────────────────────────────────────────────────────────────

    fn validate(&self, n: usize) -> Result<()> {
        let expected = self.width as usize * self.height as usize * self.bands as usize;
        if n != expected { Err(GeoTiffError::DataSizeMismatch { expected, actual: n }) } else { Ok(()) }
    }

    // ── Core writer ───────────────────────────────────────────────────────────

    pub(crate) fn write_raw<W: Write + Seek>(&self, mut w: W, pixel_bytes: &[u8]) -> Result<()> {
        let bps   = self.bits_per_sample;
        let bps_b = (bps as usize + 7) / 8;   // bytes per sample
        let spp   = self.bands as usize;

        self.validate_compression_settings(spp)?;

        // ── Encode data chunks (strips or tiles) ──────────────────────────────
        let (chunk_data, chunk_layout) = match self.layout {
            WriteLayout::Stripped { rows_per_strip } => {
                self.encode_strips(pixel_bytes, bps_b, spp, rows_per_strip)?
            }
            WriteLayout::Tiled { tile_width, tile_height } => {
                self.encode_tiles(pixel_bytes, bps_b, spp, tile_width, tile_height)?
            }
        };

        // ── Build IFD tags ────────────────────────────────────────────────────
        let geo_keys_encoded = self.geo_keys.as_ref().map(|gk| gk.encode());

        let tags = self.build_tags(
            bps,
            spp,
            &chunk_layout,
            &geo_keys_encoded,
        );

        if self.bigtiff {
            self.write_bigtiff(&mut w, tags, chunk_data)
        } else {
            self.write_classic(&mut w, tags, chunk_data)
        }
    }

    // ── Strip encoder ─────────────────────────────────────────────────────────

    fn encode_strips(
        &self,
        pixel_bytes: &[u8],
        bps_b: usize,
        spp: usize,
        rows_per_strip: u32,
    ) -> Result<(Vec<Vec<u8>>, ChunkLayout)> {
        let row_bytes = self.width as usize * spp * bps_b;
        let rps = rows_per_strip.min(self.height) as usize;
        let num_strips = (self.height as usize + rps - 1) / rps;

        let chunks: Result<Vec<Vec<u8>>> = (0..num_strips)
            .into_par_iter()
            .map(|s| {
            let row_start = s * rps;
            let row_end = (row_start + rps).min(self.height as usize);
            let raw = &pixel_bytes[row_start * row_bytes..row_end * row_bytes];
            let chunk_h = (row_end - row_start) as u32;
            self.compress_chunk(raw, self.width, chunk_h, spp)
            })
            .collect();
        let chunks = chunks?;

        Ok((chunks, ChunkLayout::Stripped { rows_per_strip: rps as u32 }))
    }

    // ── Tile encoder ──────────────────────────────────────────────────────────

    fn encode_tiles(
        &self,
        pixel_bytes: &[u8],
        bps_b: usize,
        spp: usize,
        tile_width: u32,
        tile_height: u32,
    ) -> Result<(Vec<Vec<u8>>, ChunkLayout)> {
        let w   = self.width as usize;
        let h   = self.height as usize;
        let tw  = tile_width as usize;
        let th  = tile_height as usize;

        let tiles_x = (w + tw - 1) / tw;
        let tiles_y = (h + th - 1) / th;
        let tile_raw_bytes = tw * th * spp * bps_b;

        let num_tiles = tiles_x * tiles_y;
        let chunks: Result<Vec<Vec<u8>>> = (0..num_tiles)
            .into_par_iter()
            .map(|tile_index| {
                let ty = tile_index / tiles_x;
                let tx = tile_index % tiles_x;
                let img_x0 = tx * tw;
                let img_y0 = ty * th;
                let copy_w = tw.min(w - img_x0);
                let copy_h = th.min(h - img_y0);

                // Build a full tile buffer (padded with zeros if at image edge)
                let mut tile = vec![0u8; tile_raw_bytes];
                for row in 0..copy_h {
                    let src_off = ((img_y0 + row) * w + img_x0) * spp * bps_b;
                    let dst_off = row * tw * spp * bps_b;
                    let len = copy_w * spp * bps_b;
                    tile[dst_off..dst_off + len]
                        .copy_from_slice(&pixel_bytes[src_off..src_off + len]);
                }
                self.compress_chunk(&tile, tile_width, tile_height, spp)
            })
            .collect();
        let chunks = chunks?;

        Ok((chunks, ChunkLayout::Tiled { tile_width, tile_height }))
    }

    // ── IFD tag builder ───────────────────────────────────────────────────────

    fn build_tags(
        &self,
        bps: u16,
        spp: usize,
        layout: &ChunkLayout,
        geo_keys_encoded: &Option<(Vec<u16>, Vec<f64>, String)>,
    ) -> Vec<TiffTag> {
        let mut tags: Vec<TiffTag> = Vec::new();

        if self.sub_file_type != 0 {
            push_long(&mut tags, tag::NewSubFileType, self.sub_file_type);
        }

        push_long(&mut tags, tag::ImageWidth,  self.width);
        push_long(&mut tags, tag::ImageLength, self.height);
        push_shorts(&mut tags, tag::BitsPerSample, &vec![bps; spp]);
        push_short(&mut tags, tag::Compression, self.compression.tag_value() as u32);
        push_short(
            &mut tags,
            tag::PhotometricInterpretation,
            self.effective_photometric(spp).tag_value() as u32,
        );
        if (self.compression == Compression::WebP || self.compression == Compression::JpegXl) && spp == 4 {
            push_short(&mut tags, tag::ExtraSamples, 2);
        }

        match layout {
            ChunkLayout::Stripped { rows_per_strip } => {
                // Placeholder — real offsets patched after layout
                push_longs(&mut tags, tag::StripOffsets,    &[]);
                push_short(&mut tags, tag::SamplesPerPixel, spp as u32);
                push_long(&mut tags, tag::RowsPerStrip,    *rows_per_strip);
                push_longs(&mut tags, tag::StripByteCounts, &[]);
            }
            ChunkLayout::Tiled { tile_width, tile_height } => {
                push_short(&mut tags, tag::SamplesPerPixel, spp as u32);
                push_long(&mut tags, tag::TileWidth,  *tile_width);
                push_long(&mut tags, tag::TileLength, *tile_height);
                push_longs(&mut tags, tag::TileOffsets,    &[]);
                push_longs(&mut tags, tag::TileByteCounts, &[]);
            }
        }

        push_rational(&mut tags, tag::XResolution, 72, 1);
        push_rational(&mut tags, tag::YResolution,  72, 1);
        push_short(&mut tags, tag::PlanarConfiguration, self.planar_config.tag_value() as u32);
        push_short(&mut tags, tag::SampleFormat, self.sample_format.tag_value() as u32);

        if let Some(sw) = &self.software {
            push_ascii(&mut tags, tag::Software, sw);
        }

        if let Some(gt) = &self.geo_transform {
            push_doubles(&mut tags, tag::ModelPixelScaleTag, &gt.to_pixel_scale());
            push_doubles(&mut tags, tag::ModelTiepointTag,   &gt.to_tiepoint());
        }

        if let Some((dir, dbl, asc)) = geo_keys_encoded {
            push_shorts_u16(&mut tags, tag::GeoKeyDirectoryTag, dir);
            if !dbl.is_empty() { push_doubles(&mut tags, tag::GeoDoubleParamsTag, dbl); }
            if !asc.is_empty() { push_ascii(&mut tags, tag::GeoAsciiParamsTag, asc); }
        }

        if let Some(nd) = self.no_data {
            push_ascii(&mut tags, tag::GdalNodata, &format!("{}", nd));
        }

        tags.sort_by_key(|t| t.code);
        tags
    }

    fn validate_compression_settings(&self, spp: usize) -> Result<()> {
        if self.compression == Compression::Jpeg {
            if self.sample_format != SampleFormat::Uint || self.bits_per_sample != 8 {
                return Err(GeoTiffError::UnsupportedSampleFormat {
                    bits_per_sample: self.bits_per_sample,
                    sample_format: self.sample_format.tag_value(),
                });
            }
            if spp != 1 && spp != 3 {
                return Err(GeoTiffError::UnsupportedTagValue {
                    tag: "SamplesPerPixel",
                    value: spp as u64,
                });
            }
        }
        if self.compression == Compression::WebP {
            if self.sample_format != SampleFormat::Uint || self.bits_per_sample != 8 {
                return Err(GeoTiffError::UnsupportedSampleFormat {
                    bits_per_sample: self.bits_per_sample,
                    sample_format: self.sample_format.tag_value(),
                });
            }
            if spp != 3 && spp != 4 {
                return Err(GeoTiffError::UnsupportedTagValue {
                    tag: "SamplesPerPixel",
                    value: spp as u64,
                });
            }
        }
        if self.compression == Compression::JpegXl {
            if self.sample_format != SampleFormat::Uint || self.bits_per_sample != 8 {
                return Err(GeoTiffError::UnsupportedSampleFormat {
                    bits_per_sample: self.bits_per_sample,
                    sample_format: self.sample_format.tag_value(),
                });
            }
            if spp != 1 && spp != 3 && spp != 4 {
                return Err(GeoTiffError::UnsupportedTagValue {
                    tag: "SamplesPerPixel",
                    value: spp as u64,
                });
            }
        }
        Ok(())
    }

    fn compress_chunk(&self, raw: &[u8], chunk_w: u32, chunk_h: u32, spp: usize) -> Result<Vec<u8>> {
        if self.compression == Compression::Jpeg {
            let width = u16::try_from(chunk_w).map_err(|_| GeoTiffError::CompressionError {
                codec: "JPEG",
                message: format!("chunk width {} exceeds JPEG u16 limit", chunk_w),
            })?;
            let height = u16::try_from(chunk_h).map_err(|_| GeoTiffError::CompressionError {
                codec: "JPEG",
                message: format!("chunk height {} exceeds JPEG u16 limit", chunk_h),
            })?;
            compression::compress_jpeg(raw, width, height, spp, self.jpeg_quality)
        } else if self.compression == Compression::WebP {
            compression::compress_webp(raw, chunk_w, chunk_h, spp, self.jpeg_quality as f32)
        } else if self.compression == Compression::JpegXl {
            compression::compress_jpegxl(raw, chunk_w, chunk_h, spp, self.jpeg_quality)
        } else {
            compression::compress(self.compression, raw)
        }
    }

    fn effective_photometric(&self, spp: usize) -> PhotometricInterpretation {
        if self.compression == Compression::Jpeg {
            if spp == 3 {
                return match self.photometric {
                    PhotometricInterpretation::YCbCr => PhotometricInterpretation::YCbCr,
                    _ => PhotometricInterpretation::Rgb,
                };
            }
            if spp == 1 {
                return PhotometricInterpretation::MinIsBlack;
            }
        }
        if self.compression == Compression::WebP {
            return PhotometricInterpretation::Rgb;
        }
        if self.compression == Compression::JpegXl {
            if spp == 1 {
                return PhotometricInterpretation::MinIsBlack;
            }
            return PhotometricInterpretation::Rgb;
        }
        self.photometric
    }

    // ── Classic TIFF writer ───────────────────────────────────────────────────

    fn write_classic<W: Write + Seek>(
        &self,
        w: &mut W,
        mut tags: Vec<TiffTag>,
        chunk_data: Vec<Vec<u8>>,
    ) -> Result<()> {
        let ifd_offset: u32 = 8;
        let ifd_bytes: u32  = 2 + (tags.len() as u32) * 12 + 4;

        // Layout pass (run 3× to converge with accurate offset sizes)
        for _ in 0..3 {
            let mut cur = ifd_offset + ifd_bytes;
            // Extra data blocks
            for t in tags.iter_mut() {
                if t.extra_data.len() > 4 {
                    t.extra_offset64 = cur as u64;
                    cur += t.extra_data.len() as u32;
                    if cur % 2 != 0 { cur += 1; }
                } else {
                    t.extra_offset64 = 0;
                }
            }
            // Chunk offsets
            let mut off = cur as u64;
            let chunk_offsets: Vec<u64> = chunk_data.iter().map(|c| { let o = off; off += c.len() as u64; o }).collect();
            let chunk_bc: Vec<u32> = chunk_data.iter().map(|c| c.len() as u32).collect();

            // Patch StripOffsets / TileOffsets and byte counts
            for t in tags.iter_mut() {
                if t.code == tag::StripOffsets || t.code == tag::TileOffsets {
                    t.extra_data = chunk_offsets.iter().flat_map(|&v| BO.u32_bytes(v as u32)).collect();
                    t.count = chunk_offsets.len() as u32;
                }
                if t.code == tag::StripByteCounts || t.code == tag::TileByteCounts {
                    t.extra_data = chunk_bc.iter().flat_map(|&v| BO.u32_bytes(v)).collect();
                    t.count = chunk_bc.len() as u32;
                }
            }
        }

        // Write header
        w.write_all(b"II").map_err(GeoTiffError::Io)?;
        w.write_all(&BO.u16_bytes(42)).map_err(GeoTiffError::Io)?;
        w.write_all(&BO.u32_bytes(ifd_offset)).map_err(GeoTiffError::Io)?;

        // Write IFD
        w.write_all(&BO.u16_bytes(tags.len() as u16)).map_err(GeoTiffError::Io)?;
        for t in &tags {
            w.write_all(&BO.u16_bytes(t.code)).map_err(GeoTiffError::Io)?;
            w.write_all(&BO.u16_bytes(t.data_type)).map_err(GeoTiffError::Io)?;
            w.write_all(&BO.u32_bytes(t.count)).map_err(GeoTiffError::Io)?;
            if t.extra_data.len() <= 4 {
                let mut b = [0u8; 4];
                b[..t.extra_data.len()].copy_from_slice(&t.extra_data);
                w.write_all(&b).map_err(GeoTiffError::Io)?;
            } else {
                w.write_all(&BO.u32_bytes(t.extra_offset64 as u32)).map_err(GeoTiffError::Io)?;
            }
        }
        w.write_all(&BO.u32_bytes(0)).map_err(GeoTiffError::Io)?; // next IFD

        // Write extra data
        for t in &tags {
            if t.extra_data.len() > 4 {
                w.write_all(&t.extra_data).map_err(GeoTiffError::Io)?;
                if t.extra_data.len() % 2 != 0 { w.write_all(&[0u8]).map_err(GeoTiffError::Io)?; }
            }
        }

        // Write chunks
        for chunk in &chunk_data {
            w.write_all(chunk).map_err(GeoTiffError::Io)?;
        }
        w.flush().map_err(GeoTiffError::Io)
    }

    // ── BigTIFF writer ────────────────────────────────────────────────────────
    //
    // BigTIFF header (16 bytes):
    //   "II"              – byte order
    //   43 (u16)          – magic
    //   8  (u16)          – bigtiff offset bytesize
    //   0  (u16)          – reserved
    //   first_ifd (u64)   – offset of first IFD
    //
    // BigTIFF IFD:
    //   num_entries (u64)
    //   per entry (20 bytes): tag(u16) type(u16) count(u64) value_or_offset(u64 inline)
    //   next_ifd (u64)

    fn write_bigtiff<W: Write + Seek>(
        &self,
        w: &mut W,
        mut tags: Vec<TiffTag>,
        chunk_data: Vec<Vec<u8>>,
    ) -> Result<()> {
        let header_size: u64  = 16;
        let ifd_offset:  u64  = header_size;
        let ifd_header:  u64  = 8; // u64 entry count
        let ifd_entries: u64  = tags.len() as u64 * 20;
        let ifd_footer:  u64  = 8; // u64 next ifd offset
        let ifd_total:   u64  = ifd_header + ifd_entries + ifd_footer;

        for _ in 0..3 {
            let mut cur = ifd_offset + ifd_total;
            for t in tags.iter_mut() {
                if t.extra_data.len() > 8 {
                    t.extra_offset64 = cur;
                    cur += t.extra_data.len() as u64;
                    if cur % 2 != 0 { cur += 1; }
                } else {
                    t.extra_offset64 = 0;
                }
            }
            let mut off = cur;
            let chunk_offsets: Vec<u64> = chunk_data.iter().map(|c| { let o = off; off += c.len() as u64; o }).collect();
            let chunk_bc: Vec<u64> = chunk_data.iter().map(|c| c.len() as u64).collect();

            for t in tags.iter_mut() {
                if t.code == tag::StripOffsets || t.code == tag::TileOffsets {
                    t.extra_data = chunk_offsets.iter().flat_map(|&v| BO.u64_bytes(v)).collect();
                    t.data_type = 16; // LONG8
                    t.count = chunk_offsets.len() as u32;
                }
                if t.code == tag::StripByteCounts || t.code == tag::TileByteCounts {
                    t.extra_data = chunk_bc.iter().flat_map(|&v| BO.u64_bytes(v)).collect();
                    t.data_type = 16; // LONG8
                    t.count = chunk_bc.len() as u32;
                }
            }
        }

        // Write BigTIFF header
        w.write_all(b"II").map_err(GeoTiffError::Io)?;
        w.write_all(&BO.u16_bytes(43)).map_err(GeoTiffError::Io)?;   // magic
        w.write_all(&BO.u16_bytes(8)).map_err(GeoTiffError::Io)?;    // offset size
        w.write_all(&BO.u16_bytes(0)).map_err(GeoTiffError::Io)?;    // reserved
        w.write_all(&BO.u64_bytes(ifd_offset)).map_err(GeoTiffError::Io)?;

        // Write IFD
        w.write_all(&BO.u64_bytes(tags.len() as u64)).map_err(GeoTiffError::Io)?;
        for t in &tags {
            w.write_all(&BO.u16_bytes(t.code)).map_err(GeoTiffError::Io)?;
            w.write_all(&BO.u16_bytes(t.data_type)).map_err(GeoTiffError::Io)?;
            w.write_all(&BO.u64_bytes(t.count as u64)).map_err(GeoTiffError::Io)?;
            if t.extra_data.len() <= 8 {
                let mut b = [0u8; 8];
                b[..t.extra_data.len()].copy_from_slice(&t.extra_data);
                w.write_all(&b).map_err(GeoTiffError::Io)?;
            } else {
                w.write_all(&BO.u64_bytes(t.extra_offset64)).map_err(GeoTiffError::Io)?;
            }
        }
        w.write_all(&BO.u64_bytes(0)).map_err(GeoTiffError::Io)?; // next IFD

        // Write extra data
        for t in &tags {
            if t.extra_data.len() > 8 {
                w.write_all(&t.extra_data).map_err(GeoTiffError::Io)?;
                if t.extra_data.len() % 2 != 0 { w.write_all(&[0u8]).map_err(GeoTiffError::Io)?; }
            }
        }

        // Write chunks
        for chunk in &chunk_data { w.write_all(chunk).map_err(GeoTiffError::Io)?; }
        w.flush().map_err(GeoTiffError::Io)
    }
}

// ── ChunkLayout (internal) ────────────────────────────────────────────────────

/// Describes how encoded chunks relate to the image layout.
pub(crate) enum ChunkLayout {
    Stripped { rows_per_strip: u32 },
    Tiled    { tile_width: u32, tile_height: u32 },
}

// ── TiffTag (internal) ────────────────────────────────────────────────────────

pub(crate) struct TiffTag {
    pub code:         u16,
    pub data_type:    u16,
    pub count:        u32,
    pub extra_data:   Vec<u8>,
    pub extra_offset64: u64,
}

// ── Tag builder helpers ───────────────────────────────────────────────────────

pub(crate) fn push_short(tags: &mut Vec<TiffTag>, code: u16, v: u32) {
    tags.push(TiffTag { code, data_type: 3, count: 1, extra_data: BO.u16_bytes(v as u16).to_vec(), extra_offset64: 0 });
}

pub(crate) fn push_long(tags: &mut Vec<TiffTag>, code: u16, v: u32) {
    tags.push(TiffTag { code, data_type: 4, count: 1, extra_data: BO.u32_bytes(v).to_vec(), extra_offset64: 0 });
}

pub(crate) fn push_shorts(tags: &mut Vec<TiffTag>, code: u16, vals: &[u16]) {
    let bytes: Vec<u8> = vals.iter().flat_map(|&v| BO.u16_bytes(v)).collect();
    tags.push(TiffTag { code, data_type: 3, count: vals.len() as u32, extra_data: bytes, extra_offset64: 0 });
}

pub(crate) fn push_shorts_u16(tags: &mut Vec<TiffTag>, code: u16, vals: &[u16]) {
    push_shorts(tags, code, vals);
}

pub(crate) fn push_longs(tags: &mut Vec<TiffTag>, code: u16, vals: &[u32]) {
    let bytes: Vec<u8> = vals.iter().flat_map(|&v| BO.u32_bytes(v)).collect();
    tags.push(TiffTag { code, data_type: 4, count: vals.len() as u32, extra_data: bytes, extra_offset64: 0 });
}

pub(crate) fn push_rational(tags: &mut Vec<TiffTag>, code: u16, num: u32, den: u32) {
    let mut bytes = BO.u32_bytes(num).to_vec();
    bytes.extend_from_slice(&BO.u32_bytes(den));
    tags.push(TiffTag { code, data_type: 5, count: 1, extra_data: bytes, extra_offset64: 0 });
}

pub(crate) fn push_doubles(tags: &mut Vec<TiffTag>, code: u16, vals: &[f64]) {
    let bytes: Vec<u8> = vals.iter().flat_map(|&v| BO.f64_bytes(v)).collect();
    tags.push(TiffTag { code, data_type: 12, count: vals.len() as u32, extra_data: bytes, extra_offset64: 0 });
}

pub(crate) fn push_ascii(tags: &mut Vec<TiffTag>, code: u16, s: &str) {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    tags.push(TiffTag { code, data_type: 2, count: bytes.len() as u32, extra_data: bytes, extra_offset64: 0 });
}
