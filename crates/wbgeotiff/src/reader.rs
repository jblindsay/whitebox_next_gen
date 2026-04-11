//! High-level GeoTIFF reader.

#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use super::compression;
use super::error::{GeoTiffError, Result};
use super::geo_keys::GeoKeyDirectory;
use super::ifd::{Ifd, IfdValue, TiffReader};
use super::tags::{tag, Compression, PhotometricInterpretation, PlanarConfig, SampleFormat};
use super::types::{BoundingBox, GeoTransform};

// ── ImageLayout ───────────────────────────────────────────────────────────────

/// Whether the image is organised as strips or tiles.
#[derive(Debug, Clone)]
enum ImageLayout {
    Stripped {
        rows_per_strip: u32,
        offsets: Vec<u64>,
        byte_counts: Vec<u64>,
    },
    Tiled {
        tile_width: u32,
        tile_height: u32,
        offsets: Vec<u64>,
        byte_counts: Vec<u64>,
    },
}

// ── ImageInfo ─────────────────────────────────────────────────────────────────

/// Decoded image metadata from the IFD.
#[derive(Debug, Clone)]
struct ImageInfo {
    width: u32,
    height: u32,
    samples_per_pixel: u16,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    compression: Compression,
    photometric: PhotometricInterpretation,
    planar_config: PlanarConfig,
    no_data: Option<f64>,
    layout: ImageLayout,
}

impl ImageInfo {
    /// Number of bytes per sample.
    fn bytes_per_sample(&self) -> usize {
        (self.bits_per_sample as usize + 7) / 8
    }

    /// Bytes needed for one uncompressed row (all bands, interleaved).
    fn row_bytes(&self) -> usize {
        self.width as usize * self.samples_per_pixel as usize * self.bytes_per_sample()
    }
}

// ── GeoTiff ───────────────────────────────────────────────────────────────────

/// A decoded GeoTIFF file, ready for data access.
///
/// # Example
/// ```rust,ignore
/// use wbraster::formats::geotiff_core::GeoTiff;
///
/// let tiff = GeoTiff::open("dem.tif").unwrap();
/// println!("{}x{} pixels, {} bands", tiff.width(), tiff.height(), tiff.band_count());
///
/// let elevation: Vec<f32> = tiff.read_band_f32(0).unwrap();
/// ```
pub struct GeoTiff {
    info: ImageInfo,
    geo_transform: Option<GeoTransform>,
    geo_keys: Option<GeoKeyDirectory>,
    value_transform: Option<ValueTransform>,
    /// True if the source file was BigTIFF (64-bit offsets).
    pub is_bigtiff: bool,
    /// The raw bytes of the file, loaded fully into memory for random access.
    data: Vec<u8>,
}

/// Optional linear value transform associated with raster samples.
///
/// This is intended for metadata-defined conversions such as
/// `physical_value = raw_value * scale + offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueTransform {
    /// Multiplicative scale term.
    pub scale: f64,
    /// Additive offset term.
    pub offset: f64,
}

impl GeoTiff {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Open a GeoTIFF file from disk.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(GeoTiffError::Io)?;
        Self::from_reader(BufReader::new(file))
    }

    /// Parse a GeoTIFF from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::from_reader(std::io::Cursor::new(bytes.to_vec()))
    }

    /// Parse a GeoTIFF from any `Read + Seek` reader.
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut tiff = TiffReader::new(reader)?;
        let is_bigtiff = tiff.variant.is_bigtiff();

        // Load entire file into memory for fast random access during tile/strip reads
        tiff.inner_mut().seek(std::io::SeekFrom::Start(0)).map_err(GeoTiffError::Io)?;
        let mut data = Vec::new();
        tiff.inner_mut().read_to_end(&mut data).map_err(GeoTiffError::Io)?;

        // Parse the first IFD (GeoTIFFs typically only have one)
        let ifd = tiff.read_ifd(tiff.first_ifd_offset)?;

        let info = Self::parse_image_info(&ifd)?;
        let geo_transform = Self::parse_geo_transform(&ifd);
        let geo_keys = Self::parse_geo_keys(&ifd)?;
        let value_transform = Self::parse_value_transform(&ifd);

        Ok(Self { info, geo_transform, geo_keys, value_transform, is_bigtiff, data })
    }

    // ── IFD parsing helpers ───────────────────────────────────────────────────

    fn parse_image_info(ifd: &Ifd) -> Result<ImageInfo> {
        let width = ifd.require_u64(tag::ImageWidth, "ImageWidth")? as u32;
        let height = ifd.require_u64(tag::ImageLength, "ImageLength")? as u32;

        let samples_per_pixel = ifd
            .get(tag::SamplesPerPixel)
            .and_then(|e| e.value.as_u64())
            .unwrap_or(1) as u16;

        let bits_per_sample = ifd
            .get(tag::BitsPerSample)
            .and_then(|e| e.value.as_u64())
            .unwrap_or(8) as u16;

        let sample_format = ifd
            .get(tag::SampleFormat)
            .and_then(|e| e.value.as_u64())
            .map(|v| SampleFormat::from_tag(v as u16))
            .unwrap_or_default();

        let compression = ifd
            .get(tag::Compression)
            .and_then(|e| e.value.as_u64())
            .map(|v| Compression::from_tag(v as u16))
            .unwrap_or(Compression::None);

        let photometric = ifd
            .get(tag::PhotometricInterpretation)
            .and_then(|e| e.value.as_u64())
            .map(|v| PhotometricInterpretation::from_tag(v as u16))
            .unwrap_or_default();

        let planar_config = ifd
            .get(tag::PlanarConfiguration)
            .and_then(|e| e.value.as_u64())
            .map(|v| PlanarConfig::from_tag(v as u16))
            .unwrap_or_default();

        let no_data = Self::parse_no_data(ifd);

        // Determine layout: tiled or stripped
        let layout = if let (Some(tw_entry), Some(th_entry)) = (
            ifd.get(tag::TileWidth),
            ifd.get(tag::TileLength),
        ) {
            let tile_width = tw_entry.value.as_u64().unwrap_or(256) as u32;
            let tile_height = th_entry.value.as_u64().unwrap_or(256) as u32;
            let offsets = ifd.require_u64_vec(tag::TileOffsets, "TileOffsets")?;
            let byte_counts = ifd.require_u64_vec(tag::TileByteCounts, "TileByteCounts")?;
            ImageLayout::Tiled { tile_width, tile_height, offsets, byte_counts }
        } else {
            let rows_per_strip = ifd
                .get(tag::RowsPerStrip)
                .and_then(|e| e.value.as_u64())
                .unwrap_or(height as u64) as u32;
            let offsets = ifd.require_u64_vec(tag::StripOffsets, "StripOffsets")?;
            let byte_counts = ifd.require_u64_vec(tag::StripByteCounts, "StripByteCounts")?;
            ImageLayout::Stripped { rows_per_strip, offsets, byte_counts }
        };

        if width == 0 || height == 0 || samples_per_pixel == 0 {
            return Err(GeoTiffError::InvalidDimensions {
                width,
                height,
                bands: samples_per_pixel,
            });
        }

        Ok(ImageInfo {
            width,
            height,
            samples_per_pixel,
            bits_per_sample,
            sample_format,
            compression,
            photometric,
            planar_config,
            no_data,
            layout,
        })
    }

    fn parse_geo_transform(ifd: &Ifd) -> Option<GeoTransform> {
        let scale = ifd.get(tag::ModelPixelScaleTag)?.value.as_f64_vec()?;
        let tiepoint = ifd.get(tag::ModelTiepointTag)?.value.as_f64_vec()?;
        GeoTransform::from_scale_tiepoint(&scale, &tiepoint)
    }

    fn parse_geo_keys(ifd: &Ifd) -> Result<Option<GeoKeyDirectory>> {
        let Some(dir_entry) = ifd.get(tag::GeoKeyDirectoryTag) else {
            return Ok(None);
        };
        let dir_words = match &dir_entry.value {
            IfdValue::Shorts(v) => v.clone(),
            _ => return Ok(None),
        };

        let doubles = ifd
            .get(tag::GeoDoubleParamsTag)
            .and_then(|e| e.value.as_f64_vec())
            .unwrap_or_default();

        let ascii = ifd
            .get(tag::GeoAsciiParamsTag)
            .and_then(|e| e.value.as_str().map(|s| s.to_owned()))
            .unwrap_or_default();

        Ok(Some(GeoKeyDirectory::parse(&dir_words, &doubles, &ascii)?))
    }

    fn parse_no_data(ifd: &Ifd) -> Option<f64> {
        if let Some(entry) = ifd.get(tag::GdalNodata) {
            if let Some(text) = entry.value.as_str() {
                if let Some(v) = Self::parse_no_data_text(text) {
                    return Some(v);
                }
            }
            if let Some(bytes) = entry.value.as_bytes() {
                let text = String::from_utf8_lossy(bytes);
                if let Some(v) = Self::parse_no_data_text(&text) {
                    return Some(v);
                }
            }
        }

        if let Some(meta) = ifd.get(tag::GdalMetadata).and_then(|e| e.value.as_str()) {
            if let Some(v) = Self::parse_no_data_text(meta) {
                return Some(v);
            }
        }

        None
    }

    fn parse_value_transform(ifd: &Ifd) -> Option<ValueTransform> {
        let metadata = ifd
            .get(tag::GdalMetadata)
            .and_then(|e| e.value.as_str().map(|s| s.to_owned()).or_else(|| {
                e.value
                    .as_bytes()
                    .map(|b| String::from_utf8_lossy(b).to_string())
            }));

        let metadata = metadata?;

        let scale = Self::parse_named_numeric(&metadata, &[
            "scale",
            "scalefactor",
            "scale_factor",
            "multiplicative_factor",
        ])
        .unwrap_or(1.0);
        let offset = Self::parse_named_numeric(&metadata, &[
            "offset",
            "add_offset",
            "data_offset",
        ])
        .unwrap_or(0.0);

        if !scale.is_finite() || !offset.is_finite() {
            return None;
        }

        if (scale - 1.0).abs() <= f64::EPSILON && offset.abs() <= f64::EPSILON {
            return None;
        }

        Some(ValueTransform { scale, offset })
    }

    fn parse_named_numeric(text: &str, names: &[&str]) -> Option<f64> {
        for name in names {
            if let Some(v) = Self::parse_xml_item_value(text, name) {
                return Some(v);
            }
            if let Some(v) = Self::parse_token_value(text, name) {
                return Some(v);
            }
        }
        None
    }

    fn parse_xml_item_value(text: &str, name: &str) -> Option<f64> {
        let lower = text.to_ascii_lowercase();
        let name_attr_double = format!("name=\"{}\"", name);
        let name_attr_single = format!("name='{}'", name);

        let mut cursor = 0usize;
        while let Some(rel_start) = lower[cursor..].find("<item") {
            let start = cursor + rel_start;
            let rel_tag_end = lower[start..].find('>')?;
            let tag_end = start + rel_tag_end;
            let rel_close = lower[tag_end + 1..].find("</item>")?;
            let close = tag_end + 1 + rel_close;

            let header = &lower[start..=tag_end];
            if header.contains(&name_attr_double) || header.contains(&name_attr_single) {
                let value_text = text[tag_end + 1..close].trim();
                if let Ok(v) = value_text.parse::<f64>() {
                    return Some(v);
                }
            }

            cursor = close + "</item>".len();
        }
        None
    }

    fn parse_token_value(text: &str, name: &str) -> Option<f64> {
        let lower_name = name.to_ascii_lowercase();
        for token in text.split(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == '|') {
            if token.is_empty() {
                continue;
            }

            let token_lower = token.to_ascii_lowercase();
            if let Some(value) = token_lower.strip_prefix(&(lower_name.clone() + "=")) {
                if let Ok(v) = value.trim_matches(|c| c == '"' || c == '\'' || c == '\0').parse::<f64>() {
                    return Some(v);
                }
            }
            if let Some(value) = token_lower.strip_prefix(&(lower_name.clone() + ":")) {
                if let Ok(v) = value.trim_matches(|c| c == '"' || c == '\'' || c == '\0').parse::<f64>() {
                    return Some(v);
                }
            }
        }
        None
    }

    fn parse_no_data_text(text: &str) -> Option<f64> {
        let cleaned = text.trim_matches(|c: char| c.is_whitespace() || c == '\0');
        if cleaned.is_empty() {
            return None;
        }

        if let Ok(v) = cleaned.parse::<f64>() {
            return Some(v);
        }

        for token in cleaned.split(|c: char| c == '|' || c == ',' || c == ';' || c.is_whitespace()) {
            let token = token.trim_matches('\0');
            if token.is_empty() {
                continue;
            }
            if let Ok(v) = token.parse::<f64>() {
                return Some(v);
            }
        }

        if let Some(pos) = cleaned.find("NODATA") {
            let tail = &cleaned[pos..];
            for token in tail.split(|c: char| c == '<' || c == '>' || c == '"' || c == '=' || c == '|' || c == ',' || c == ';' || c.is_whitespace()) {
                if let Ok(v) = token.parse::<f64>() {
                    return Some(v);
                }
            }
        }

        None
    }

    // ── Public metadata accessors ─────────────────────────────────────────────

    /// Image width in pixels.
    pub fn width(&self) -> u32 { self.info.width }

    /// Image height in pixels.
    pub fn height(&self) -> u32 { self.info.height }

    /// Number of bands (samples per pixel).
    pub fn band_count(&self) -> usize { self.info.samples_per_pixel as usize }

    /// Bits per sample.
    pub fn bits_per_sample(&self) -> u16 { self.info.bits_per_sample }

    /// Sample format.
    pub fn sample_format(&self) -> SampleFormat { self.info.sample_format }

    /// Compression codec used.
    pub fn compression(&self) -> Compression { self.info.compression }

    /// Photometric interpretation.
    pub fn photometric(&self) -> PhotometricInterpretation { self.info.photometric }

    /// No-data value (if set via GDAL NODATA tag).
    pub fn no_data(&self) -> Option<f64> { self.info.no_data }

    /// The affine geo-transform, if present.
    pub fn geo_transform(&self) -> Option<&GeoTransform> { self.geo_transform.as_ref() }

    /// The decoded GeoKey directory, if present.
    pub fn geo_keys(&self) -> Option<&GeoKeyDirectory> { self.geo_keys.as_ref() }

    /// EPSG code derived from the GeoKey directory.
    pub fn epsg(&self) -> Option<u16> {
        self.geo_keys.as_ref()?.epsg()
    }

    /// Optional linear value transform parsed from GDAL metadata.
    ///
    /// When present, data consumers may apply:
    /// `physical_value = raw_value * scale + offset`.
    pub fn value_transform(&self) -> Option<ValueTransform> {
        self.value_transform
    }

    /// Bounding box in geographic / projected coordinates.
    ///
    /// Returns `None` if no geo-transform is available.
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        let t = self.geo_transform.as_ref()?;
        let (x0, y0) = t.pixel_to_geo(0.0, 0.0);
        let (x1, y1) = t.pixel_to_geo(self.info.width as f64, self.info.height as f64);
        Some(BoundingBox::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
    }

    // ── Data access ───────────────────────────────────────────────────────────

    /// Read all pixel data for a single band as raw bytes.
    ///
    /// Bands are zero-indexed.  For multi-band chunky images the band samples
    /// are de-interleaved automatically.
    pub fn read_band_bytes(&self, band: usize) -> Result<Vec<u8>> {
        if band >= self.band_count() {
            return Err(GeoTiffError::BandOutOfRange {
                index: band,
                bands: self.band_count(),
            });
        }

        let raw = self.decode_all_pixels()?;
        self.extract_band_bytes(&raw, band)
    }

    /// Read a band as `u8` values.
    pub fn read_band_u8(&self, band: usize) -> Result<Vec<u8>> {
        self.validate_sample_type(SampleFormat::Uint, 8)?;
        self.read_band_bytes(band)
    }

    /// Read a band as `u16` values.
    pub fn read_band_u16(&self, band: usize) -> Result<Vec<u16>> {
        self.validate_sample_type(SampleFormat::Uint, 16)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `u32` values.
    pub fn read_band_u32(&self, band: usize) -> Result<Vec<u32>> {
        self.validate_sample_type(SampleFormat::Uint, 32)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `u64` values.
    pub fn read_band_u64(&self, band: usize) -> Result<Vec<u64>> {
        self.validate_sample_type(SampleFormat::Uint, 64)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `i8` values.
    pub fn read_band_i8(&self, band: usize) -> Result<Vec<i8>> {
        self.validate_sample_type(SampleFormat::Int, 8)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.into_iter().map(|v| v as i8).collect())
    }

    /// Read a band as `i16` values.
    pub fn read_band_i16(&self, band: usize) -> Result<Vec<i16>> {
        self.validate_sample_type(SampleFormat::Int, 16)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(2)
            .map(|c| i16::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `i32` values.
    pub fn read_band_i32(&self, band: usize) -> Result<Vec<i32>> {
        self.validate_sample_type(SampleFormat::Int, 32)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(4)
            .map(|c| i32::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `i64` values.
    pub fn read_band_i64(&self, band: usize) -> Result<Vec<i64>> {
        self.validate_sample_type(SampleFormat::Int, 64)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(8)
            .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `f32` values.
    pub fn read_band_f32(&self, band: usize) -> Result<Vec<f32>> {
        self.validate_sample_type(SampleFormat::IeeeFloat, 32)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read a band as `f64` values.
    pub fn read_band_f64(&self, band: usize) -> Result<Vec<f64>> {
        self.validate_sample_type(SampleFormat::IeeeFloat, 64)?;
        let bytes = self.read_band_bytes(band)?;
        Ok(bytes.chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    /// Read all bands, returned as a flat `[band0_px0, band1_px0, …, band0_px1, …]`
    /// (chunky/RGBRGB… order) vector of `f64`.
    pub fn read_all_f64(&self) -> Result<Vec<f64>> {
        let raw = self.decode_all_pixels()?;
        let bps = self.info.bytes_per_sample();
        let sf = self.info.sample_format;

        raw.chunks_exact(bps)
            .map(|c| sample_to_f64(c, sf))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| GeoTiffError::UnsupportedSampleFormat {
                bits_per_sample: self.info.bits_per_sample,
                sample_format: sf.tag_value(),
            })
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn validate_sample_type(&self, expected_fmt: SampleFormat, expected_bits: u16) -> Result<()> {
        if self.info.sample_format != expected_fmt || self.info.bits_per_sample != expected_bits {
            Err(GeoTiffError::UnsupportedSampleFormat {
                bits_per_sample: self.info.bits_per_sample,
                sample_format: self.info.sample_format.tag_value(),
            })
        } else {
            Ok(())
        }
    }

    /// Decode all strips or tiles into one contiguous byte buffer
    /// (full-image, row-major, all bands interleaved).
    fn decode_all_pixels(&self) -> Result<Vec<u8>> {
        let expected_total = self.info.row_bytes() * self.info.height as usize;

        match &self.info.layout {
            ImageLayout::Stripped { rows_per_strip, offsets, byte_counts } => {
                let mut out = Vec::with_capacity(expected_total);
                let rps = *rows_per_strip as usize;
                let row_bytes = self.info.row_bytes();

                for (i, (&off, &bc)) in offsets.iter().zip(byte_counts.iter()).enumerate() {
                    let start = off as usize;
                    let end = start + bc as usize;
                    if end > self.data.len() {
                        return Err(GeoTiffError::CorruptData {
                            location: format!("strip {}", i),
                            message: format!("offset {} + count {} > file size {}", off, bc, self.data.len()),
                        });
                    }
                    let compressed = &self.data[start..end];
                    let strip_rows = rps.min(self.info.height as usize - i * rps);
                    let expected_strip = strip_rows * row_bytes;
                    let decompressed =
                        compression::decompress(self.info.compression, compressed, expected_strip)?;
                    out.extend_from_slice(&decompressed[..decompressed.len().min(expected_strip)]);
                }
                Ok(out)
            }

            ImageLayout::Tiled { tile_width, tile_height, offsets, byte_counts } => {
                let tw = *tile_width as usize;
                let th = *tile_height as usize;
                let w = self.info.width as usize;
                let h = self.info.height as usize;
                let spp = self.info.samples_per_pixel as usize;
                let bps = self.info.bytes_per_sample();

                let tiles_x = (w + tw - 1) / tw;
                let tiles_y = (h + th - 1) / th;

                let mut out = vec![0u8; expected_total];
                let tile_bytes_raw = tw * th * spp * bps;

                for ty in 0..tiles_y {
                    for tx in 0..tiles_x {
                        let idx = ty * tiles_x + tx;
                        let off = offsets[idx] as usize;
                        let bc = byte_counts[idx] as usize;
                        if off + bc > self.data.len() {
                            return Err(GeoTiffError::CorruptData {
                                location: format!("tile ({}, {})", tx, ty),
                                message: "tile data out of range".into(),
                            });
                        }
                        let compressed = &self.data[off..off + bc];
                        let decompressed =
                            compression::decompress(self.info.compression, compressed, tile_bytes_raw)?;

                        // Copy tile into the output buffer, clipping at image edges
                        let img_x0 = tx * tw;
                        let img_y0 = ty * th;
                        let copy_w = tw.min(w - img_x0);
                        let copy_h = th.min(h - img_y0);

                        for row in 0..copy_h {
                            let src_off = row * tw * spp * bps;
                            let dst_off = ((img_y0 + row) * w + img_x0) * spp * bps;
                            let len = copy_w * spp * bps;
                            if src_off + len <= decompressed.len() {
                                out[dst_off..dst_off + len]
                                    .copy_from_slice(&decompressed[src_off..src_off + len]);
                            }
                        }
                    }
                }
                Ok(out)
            }
        }
    }

    /// Extract a single band from a chunky (interleaved) or planar byte buffer.
    fn extract_band_bytes(&self, all_bytes: &[u8], band: usize) -> Result<Vec<u8>> {
        let npix = (self.info.width * self.info.height) as usize;
        let bps = self.info.bytes_per_sample();
        let spp = self.info.samples_per_pixel as usize;
        let mut out = Vec::with_capacity(npix * bps);

        match self.info.planar_config {
            PlanarConfig::Chunky => {
                for p in 0..npix {
                    let off = (p * spp + band) * bps;
                    out.extend_from_slice(&all_bytes[off..off + bps]);
                }
            }
            PlanarConfig::Planar => {
                let plane_bytes = npix * bps;
                let start = band * plane_bytes;
                out.extend_from_slice(&all_bytes[start..start + plane_bytes]);
            }
        }
        Ok(out)
    }
}

// ── Helper: sample byte → f64 ─────────────────────────────────────────────────

fn sample_to_f64(bytes: &[u8], fmt: SampleFormat) -> Option<f64> {
    Some(match (fmt, bytes.len()) {
        (SampleFormat::Uint, 1) => bytes[0] as f64,
        (SampleFormat::Uint, 2) => u16::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::Uint, 4) => u32::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::Uint, 8) => u64::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::Int, 1) => bytes[0] as i8 as f64,
        (SampleFormat::Int, 2) => i16::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::Int, 4) => i32::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::Int, 8) => i64::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::IeeeFloat, 4) => f32::from_le_bytes(bytes.try_into().ok()?) as f64,
        (SampleFormat::IeeeFloat, 8) => f64::from_le_bytes(bytes.try_into().ok()?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::writer::GeoTiffWriter;
    use super::super::types::GeoTransform;
    use super::super::tags::Compression;
    use tempfile::NamedTempFile;

    fn make_tiff(compression: Compression) -> Vec<u8> {
        let data: Vec<f32> = (0..16u32).map(|i| i as f32 * 0.5).collect();
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        GeoTiffWriter::new(4, 4, 1)
            .compression(compression)
            .sample_format(SampleFormat::IeeeFloat)
            .geo_transform(GeoTransform::north_up(0.0, 1.0, 4.0, -1.0))
            .epsg(4326)
            .write_f32_to_writer(&mut cursor, &data)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn roundtrip_none() {
        let buf = make_tiff(Compression::None);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        assert_eq!(tiff.width(), 4);
        assert_eq!(tiff.height(), 4);
        let vals = tiff.read_band_f32(0).unwrap();
        assert!((vals[3] - 1.5f32).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_deflate() {
        let buf = make_tiff(Compression::Deflate);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        let vals = tiff.read_band_f32(0).unwrap();
        assert_eq!(vals.len(), 16);
        assert!((vals[0] - 0.0f32).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_packbits() {
        let buf = make_tiff(Compression::PackBits);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        let vals = tiff.read_band_f32(0).unwrap();
        assert_eq!(vals.len(), 16);
    }

    #[test]
    fn roundtrip_lzw() {
        let buf = make_tiff(Compression::Lzw);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        let vals = tiff.read_band_f32(0).unwrap();
        assert_eq!(vals.len(), 16);
        assert!((vals[15] - 7.5f32).abs() < 1e-6);
    }

    #[test]
    fn geo_transform_roundtrip() {
        let buf = make_tiff(Compression::None);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        let gt = tiff.geo_transform().unwrap();
        assert!((gt.x_origin - 0.0).abs() < 1e-9);
        assert!((gt.y_origin - 4.0).abs() < 1e-9);
        assert!((gt.pixel_width - 1.0).abs() < 1e-9);
        assert!((gt.pixel_height - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn epsg_roundtrip() {
        let buf = make_tiff(Compression::None);
        let tiff = GeoTiff::from_bytes(&buf).unwrap();
        assert_eq!(tiff.epsg(), Some(4326));
    }

    #[test]
    fn parse_scale_offset_from_gdal_xml_metadata() {
        let xml = "<GDALMetadata><Item name=\"scale\" sample=\"0\">0.1</Item><Item name=\"offset\" sample=\"0\">5</Item></GDALMetadata>";
        let scale = GeoTiff::parse_named_numeric(xml, &["scale"]).unwrap();
        let offset = GeoTiff::parse_named_numeric(xml, &["offset"]).unwrap();
        assert!((scale - 0.1).abs() < 1e-12);
        assert!((offset - 5.0).abs() < 1e-12);
    }

    #[test]
    fn parse_scale_offset_from_key_value_tokens() {
        let text = "SCALE=0.25 OFFSET=-12";
        let scale = GeoTiff::parse_named_numeric(text, &["scale"]).unwrap();
        let offset = GeoTiff::parse_named_numeric(text, &["offset"]).unwrap();
        assert!((scale - 0.25).abs() < 1e-12);
        assert!((offset + 12.0).abs() < 1e-12);
    }

    #[test]
    fn jpeg_u8_roundtrip() {
        let w = 64u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h)).map(|i| (i % 251) as u8).collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 1)
            .compression(Compression::Jpeg)
            .jpeg_quality(72)
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert_eq!(tiff.compression(), Compression::Jpeg);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), data.len());

        let max_abs_err = data.iter()
            .zip(read_back.iter())
            .map(|(a, b)| (*a as i16 - *b as i16).abs() as u8)
            .max()
            .unwrap_or(0);
        assert!(max_abs_err <= 40, "JPEG error too large: {}", max_abs_err);
    }

    #[test]
    fn jpeg_u8_bigtiff_roundtrip() {
        let w = 128u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h)).map(|i| ((i * 3) % 256) as u8).collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 1)
            .bigtiff(true)
            .compression(Compression::Jpeg)
            .jpeg_quality(90)
            .layout(super::super::writer::WriteLayout::Tiled { tile_width: 32, tile_height: 32 })
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert!(tiff.is_bigtiff);
        assert_eq!(tiff.compression(), Compression::Jpeg);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), data.len());
    }

    #[test]
    fn webp_u8_roundtrip() {
        let w = 64u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = (i % 251) as u8;
                [v, v.wrapping_add(23), v.wrapping_add(47)]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 3)
            .compression(Compression::WebP)
            .jpeg_quality(72)
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert_eq!(tiff.compression(), Compression::WebP);
        assert_eq!(tiff.band_count(), 3);
        let read_r = tiff.read_band_u8(0).unwrap();
        let read_g = tiff.read_band_u8(1).unwrap();
        let read_b = tiff.read_band_u8(2).unwrap();
        assert_eq!(read_r.len(), (w * h) as usize);
        assert_eq!(read_g.len(), (w * h) as usize);
        assert_eq!(read_b.len(), (w * h) as usize);
    }

    #[test]
    fn webp_u8_bigtiff_roundtrip() {
        let w = 128u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 5) % 256) as u8;
                [v, v.wrapping_add(31), v.wrapping_add(63)]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 3)
            .bigtiff(true)
            .compression(Compression::WebP)
            .jpeg_quality(85)
            .layout(super::super::writer::WriteLayout::Tiled { tile_width: 32, tile_height: 32 })
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert!(tiff.is_bigtiff);
        assert_eq!(tiff.compression(), Compression::WebP);
        assert_eq!(tiff.band_count(), 3);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn webp_u8_rgba_roundtrip() {
        let w = 64u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 7) % 256) as u8;
                [v, v.wrapping_add(29), v.wrapping_add(61), 200]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 4)
            .compression(Compression::WebP)
            .jpeg_quality(80)
            .layout(super::super::writer::WriteLayout::Tiled { tile_width: 32, tile_height: 32 })
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert_eq!(tiff.compression(), Compression::WebP);
        assert_eq!(tiff.band_count(), 4);

        let read_r = tiff.read_band_u8(0).unwrap();
        let read_a = tiff.read_band_u8(3).unwrap();
        assert_eq!(read_r.len(), (w * h) as usize);
        assert_eq!(read_a.len(), (w * h) as usize);
    }

    #[test]
    fn jpegxl_u8_roundtrip() {
        let w = 64u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = (i % 251) as u8;
                [v, v.wrapping_add(13), v.wrapping_add(27)]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 3)
            .compression(Compression::JpegXl)
            .jpeg_quality(84)
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert_eq!(tiff.compression(), Compression::JpegXl);
        assert_eq!(tiff.band_count(), 3);
        let read_r = tiff.read_band_u8(0).unwrap();
        let read_g = tiff.read_band_u8(1).unwrap();
        let read_b = tiff.read_band_u8(2).unwrap();
        assert_eq!(read_r.len(), (w * h) as usize);
        assert_eq!(read_g.len(), (w * h) as usize);
        assert_eq!(read_b.len(), (w * h) as usize);
    }

    #[test]
    fn jpegxl_u8_bigtiff_roundtrip() {
        let w = 128u32;
        let h = 64u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 9) % 256) as u8;
                [v, v.wrapping_add(33), v.wrapping_add(67), 220]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path();

        GeoTiffWriter::new(w, h, 4)
            .bigtiff(true)
            .compression(Compression::JpegXl)
            .jpeg_quality(90)
            .layout(super::super::writer::WriteLayout::Tiled { tile_width: 32, tile_height: 32 })
            .write_u8(path, &data)
            .unwrap();

        let tiff = GeoTiff::open(path).unwrap();
        assert!(tiff.is_bigtiff);
        assert_eq!(tiff.compression(), Compression::JpegXl);
        assert_eq!(tiff.band_count(), 4);
        let read_back = tiff.read_band_u8(3).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn integer_roundtrip_i8_u32_i32() {
        let file_i8 = NamedTempFile::new().unwrap();
        let path_i8 = file_i8.path();
        let data_i8: Vec<i8> = vec![-128, -5, -1, 0, 1, 5, 12, 127];
        GeoTiffWriter::new(4, 2, 1)
            .write_i8(path_i8, &data_i8)
            .unwrap();
        let tiff_i8 = GeoTiff::open(path_i8).unwrap();
        assert_eq!(tiff_i8.read_band_i8(0).unwrap(), data_i8);

        let file_u32 = NamedTempFile::new().unwrap();
        let path_u32 = file_u32.path();
        let data_u32: Vec<u32> = vec![0, 1, 255, 65_535, 100_000, 1_000_000, u32::MAX - 1, u32::MAX];
        GeoTiffWriter::new(4, 2, 1)
            .write_u32(path_u32, &data_u32)
            .unwrap();
        let tiff_u32 = GeoTiff::open(path_u32).unwrap();
        assert_eq!(tiff_u32.read_band_u32(0).unwrap(), data_u32);

        let file_i32 = NamedTempFile::new().unwrap();
        let path_i32 = file_i32.path();
        let data_i32: Vec<i32> = vec![i32::MIN, -1_000_000, -32_768, -1, 0, 1, 32_767, i32::MAX];
        GeoTiffWriter::new(4, 2, 1)
            .write_i32(path_i32, &data_i32)
            .unwrap();
        let tiff_i32 = GeoTiff::open(path_i32).unwrap();
        assert_eq!(tiff_i32.read_band_i32(0).unwrap(), data_i32);

        let file_u64 = NamedTempFile::new().unwrap();
        let path_u64 = file_u64.path();
        let data_u64: Vec<u64> = vec![0, 1, 255, 65_535, 1_000_000, 9_007_199_254_740_991, u64::MAX - 1, u64::MAX];
        GeoTiffWriter::new(4, 2, 1)
            .write_u64(path_u64, &data_u64)
            .unwrap();
        let tiff_u64 = GeoTiff::open(path_u64).unwrap();
        assert_eq!(tiff_u64.read_band_u64(0).unwrap(), data_u64);

        let file_i64 = NamedTempFile::new().unwrap();
        let path_i64 = file_i64.path();
        let data_i64: Vec<i64> = vec![i64::MIN, -9_007_199_254_740_991, -1_000_000, -1, 0, 1, 9_007_199_254_740_991, i64::MAX];
        GeoTiffWriter::new(4, 2, 1)
            .write_i64(path_i64, &data_i64)
            .unwrap();
        let tiff_i64 = GeoTiff::open(path_i64).unwrap();
        assert_eq!(tiff_i64.read_band_i64(0).unwrap(), data_i64);
    }
}
