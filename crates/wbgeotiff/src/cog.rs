//! Cloud Optimized GeoTIFF (COG) writer.
//!
//! A COG is a regular GeoTIFF that follows a specific layout to enable efficient
//! HTTP range-request based access:
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  TIFF/BigTIFF header                        │
//! │  Ghost metadata block ("GDAL_STRUCTURAL_METADATA") │
//! │  Overview IFDs  (lowest-res → highest-res)  │
//! │  Full-resolution IFD                        │
//! │  Overview tile data (lowest → highest res)  │
//! │  Full-resolution tile data                  │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! Key properties:
//! - All image data is **tiled** (default 512×512).
//! - Overview (reduced-resolution) levels are embedded.
//! - The full-resolution data comes **last** so range requests can efficiently
//!   stream any tile without reading the entire file.
//! - A "ghost" ASCII metadata block at the start lets clients detect COG layout
//!   without parsing all IFDs.
//!
//! # Example
//! ```rust,ignore
//! use wbraster::formats::geotiff_core::{CogWriter, Compression, GeoTransform};
//!
//! let data: Vec<f32> = vec![0.0; 4096 * 4096];
//! CogWriter::new(4096, 4096, 1)
//!     .compression(Compression::Deflate)
//!     .tile_size(512)
//!     .geo_transform(GeoTransform::north_up(-180.0, 0.0879, 90.0, -0.0879))
//!     .epsg(4326)
//!     .write_f32("output.cog.tif", &data)
//!     .unwrap();
//! ```

#![allow(dead_code)]

use std::io::{Seek, Write};
use std::path::Path;
use std::fs::File;

use super::compression;
use super::error::{GeoTiffError, Result};
use super::geo_keys::{GeoKeyBuilder, GeoKeyDirectory};
use super::ifd::ByteOrder;
use super::tags::{tag, Compression, PhotometricInterpretation, SampleFormat};
use super::types::GeoTransform;
use super::writer::{
    push_ascii, push_doubles, push_long, push_longs,
    push_short, push_shorts, push_shorts_u16, TiffTag,
};

const BO: ByteOrder = ByteOrder::LittleEndian;

// ── Overview resampling ───────────────────────────────────────────────────────

/// Overview resampling algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resampling {
    /// Nearest-neighbour (fast, good for categorical data).
    #[default]
    Nearest,
    /// 2×2 average (good for continuous data like elevation / imagery).
    Average,
}

// ── CogWriter ────────────────────────────────────────────────────────────────

/// Builder for writing Cloud Optimized GeoTIFF files.
pub struct CogWriter {
    width:           u32,
    height:          u32,
    bands:           u16,
    bits_per_sample: u16,
    sample_format:   SampleFormat,
    compression:     Compression,
    photometric:     PhotometricInterpretation,
    tile_size:       u32,
    geo_transform:   Option<GeoTransform>,
    geo_keys:        Option<GeoKeyDirectory>,
    no_data:         Option<f64>,
    jpeg_quality:    u8,
    resampling:      Resampling,
    /// Explicit overview levels; `None` = auto-generate power-of-two levels.
    overview_levels: Option<Vec<u32>>,
    bigtiff:         bool,
}

#[allow(missing_docs)]
impl CogWriter {
    /// Create a new COG writer for a `width × height × bands` raster.
    pub fn new(width: u32, height: u32, bands: u16) -> Self {
        Self {
            width, height, bands,
            bits_per_sample: 8,
            sample_format:   SampleFormat::Uint,
            compression:     Compression::Deflate,
            photometric:     PhotometricInterpretation::MinIsBlack,
            tile_size:       512,
            geo_transform:   None,
            geo_keys:        None,
            no_data:         None,
            jpeg_quality:    85,
            resampling:      Resampling::Average,
            overview_levels: None,
            bigtiff:         false,
        }
    }

    // ── Builder setters ───────────────────────────────────────────────────────

    pub fn compression(mut self, c: Compression) -> Self { self.compression = c; self }
    pub fn sample_format(mut self, sf: SampleFormat) -> Self { self.sample_format = sf; self }
    pub fn bits_per_sample(mut self, bps: u16) -> Self { self.bits_per_sample = bps; self }
    pub fn photometric(mut self, p: PhotometricInterpretation) -> Self { self.photometric = p; self }
    pub fn tile_size(mut self, sz: u32) -> Self { self.tile_size = sz; self }
    pub fn geo_transform(mut self, gt: GeoTransform) -> Self { self.geo_transform = Some(gt); self }
    pub fn geo_key_directory(mut self, gkd: GeoKeyDirectory) -> Self { self.geo_keys = Some(gkd); self }
    pub fn no_data(mut self, v: f64) -> Self { self.no_data = Some(v); self }
    /// Set JPEG quality in range 1..=100 (used when `Compression::Jpeg` is selected).
    pub fn jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = quality.clamp(1, 100);
        self
    }
    /// Set the overview resampling method used when generating reduced-resolution levels.
    pub fn resampling(mut self, r: Resampling) -> Self { self.resampling = r; self }
    /// Enable or disable BigTIFF output (8-byte offsets).
    pub fn bigtiff(mut self, b: bool) -> Self { self.bigtiff = b; self }
    /// Override the automatically-derived overview levels.
    pub fn overview_levels(mut self, levels: Vec<u32>) -> Self { self.overview_levels = Some(levels); self }

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

    // ── Typed write entry points ──────────────────────────────────────────────

    /// Write `u8` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_u8<P: AsRef<Path>>(mut self, path: P, data: &[u8]) -> Result<()> {
        self.bits_per_sample = 8; self.sample_format = SampleFormat::Uint;
        let bytes = data.to_vec();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `i8` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_i8<P: AsRef<Path>>(mut self, path: P, data: &[i8]) -> Result<()> {
        self.bits_per_sample = 8; self.sample_format = SampleFormat::Int;
        let bytes: Vec<u8> = data.iter().map(|v| *v as u8).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `u16` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_u16<P: AsRef<Path>>(mut self, path: P, data: &[u16]) -> Result<()> {
        self.bits_per_sample = 16; self.sample_format = SampleFormat::Uint;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `i16` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_i16<P: AsRef<Path>>(mut self, path: P, data: &[i16]) -> Result<()> {
        self.bits_per_sample = 16; self.sample_format = SampleFormat::Int;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `u32` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_u32<P: AsRef<Path>>(mut self, path: P, data: &[u32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::Uint;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `u64` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_u64<P: AsRef<Path>>(mut self, path: P, data: &[u64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::Uint;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `i32` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_i32<P: AsRef<Path>>(mut self, path: P, data: &[i32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::Int;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `i64` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_i64<P: AsRef<Path>>(mut self, path: P, data: &[i64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::Int;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `f32` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_f32<P: AsRef<Path>>(mut self, path: P, data: &[f32]) -> Result<()> {
        self.bits_per_sample = 32; self.sample_format = SampleFormat::IeeeFloat;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    /// Write `f64` raster data as a Cloud Optimized GeoTIFF.
    pub fn write_f64<P: AsRef<Path>>(mut self, path: P, data: &[f64]) -> Result<()> {
        self.bits_per_sample = 64; self.sample_format = SampleFormat::IeeeFloat;
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut f = File::create(path).map_err(GeoTiffError::Io)?;
        self.write_cog(&mut f, bytes)
    }

    // ── Core COG writer ───────────────────────────────────────────────────────

    fn write_cog<W: Write + Seek>(&self, w: &mut W, pixel_bytes: Vec<u8>) -> Result<()> {
        let bps_b = (self.bits_per_sample as usize + 7) / 8;
        let spp   = self.bands as usize;
        let ts    = self.tile_size as usize;

        self.validate_compression_settings(spp)?;

        // ── 1. Generate overview pixel data ───────────────────────────────────
        let levels = self.compute_overview_levels();
        let mut overview_pixels: Vec<(u32, u32, Vec<u8>)> = Vec::new(); // (w, h, bytes)
        let mut prev_w = self.width;
        let mut prev_h = self.height;
        let mut prev_bytes: &[u8] = &pixel_bytes;
        let mut storage: Vec<Vec<u8>> = Vec::new();

        for &factor in &levels {
            let ov_w = (prev_w + factor - 1) / factor;
            let ov_h = (prev_h + factor - 1) / factor;
            let resampled = self.resample_overview(
                prev_bytes, prev_w, prev_h, spp, bps_b, ov_w, ov_h, factor,
            )?;
            overview_pixels.push((ov_w, ov_h, resampled.clone()));
            storage.push(resampled);
            prev_w = ov_w;
            prev_h = ov_h;
            prev_bytes = &storage[storage.len() - 1];
        }

        // ── 2. Encode tiles for each level ────────────────────────────────────
        // Write full-resolution first (IFD0), then overviews.
        let mut encoded_levels: Vec<EncodedLevel> = Vec::new();

        // Full resolution (first)
        let full_tiles = self.encode_tile_set(&pixel_bytes, self.width, self.height, spp, bps_b, ts)?;
        encoded_levels.push(EncodedLevel {
            width: self.width, height: self.height, tiles: full_tiles, is_overview: false,
        });

        // Overviews (lowest-res → highest-res)
        for (ov_w, ov_h, ov_bytes) in overview_pixels.iter().rev() {
            let tiles = self.encode_tile_set(ov_bytes, *ov_w, *ov_h, spp, bps_b, ts)?;
            encoded_levels.push(EncodedLevel { width: *ov_w, height: *ov_h, tiles, is_overview: true });
        }

        // ── 3. Serialise to a temporary buffer so we know all offsets ─────────
        let geo_keys_enc = self.geo_keys.as_ref().map(|gk| gk.encode());
        let mut buf: Vec<u8> = Vec::new();
        self.serialise_cog(&mut buf, &encoded_levels, &geo_keys_enc, spp)?;

        // ── 4. Write to output ────────────────────────────────────────────────
        w.write_all(&buf).map_err(GeoTiffError::Io)?;
        w.flush().map_err(GeoTiffError::Io)
    }

    fn serialise_cog(
        &self,
        buf: &mut Vec<u8>,
        levels: &[EncodedLevel],
        geo_keys_enc: &Option<(Vec<u16>, Vec<f64>, String)>,
        spp: usize,
    ) -> Result<()> {
        // ── Ghost metadata block ──────────────────────────────────────────────
        // GDAL-compatible structural metadata at byte 0 offset 16 (after header).
        // Classic TIFF header = 8 bytes; we place ghost block right after.
        // We need to write the header first (placeholder), then ghost, then IFDs.

        let ghost = self.ghost_metadata_block(levels);

        // ── Compute full file layout ──────────────────────────────────────────
        let header_size: u64 = if self.bigtiff { 16 } else { 8 };
        let ghost_size   = ghost.len() as u64;

        // Build IFD list: full-res first (IFD0), then overviews
        let num_ifds = levels.len();

        // Each IFD is variable size; we need a pass to compute it.
        // We build all IFDs as byte blobs and compute offsets.

        let bps = self.bits_per_sample;

        // Phase 1: build IFD blobs without tile data offsets
        let mut ifd_blobs: Vec<Vec<u8>> = Vec::with_capacity(num_ifds);
        for level in levels.iter() {
            let blob = self.build_ifd_blob(
                level, bps, spp, geo_keys_enc,
                !level.is_overview,       // only full-res gets geo tags
                level.is_overview,
                0, // tile offsets placeholder
                0, // next IFD placeholder
                0,
            );
            ifd_blobs.push(blob);
        }

        // Phase 2: compute offsets
        let mut cursor = header_size + ghost_size;
        let mut ifd_offsets: Vec<u64> = Vec::with_capacity(num_ifds);
        for blob in &ifd_blobs {
            ifd_offsets.push(cursor);
            cursor += blob.len() as u64;
            if cursor % 2 != 0 { cursor += 1; }
        }

        // Tile data section follows `levels` order.
        let mut tile_start_offsets: Vec<Vec<u64>> = Vec::with_capacity(num_ifds);
        for level in levels.iter() {
            let mut offsets = Vec::with_capacity(level.tiles.len());
            for tile in &level.tiles {
                offsets.push(cursor);
                cursor += tile.len() as u64;
            }
            tile_start_offsets.push(offsets);
        }

        // Phase 3: re-build IFD blobs with real offsets
        ifd_blobs.clear();
        for (i, level) in levels.iter().enumerate() {
            let is_last_ifd = i == num_ifds - 1;
            let next_ifd = if is_last_ifd { 0 } else { ifd_offsets[i + 1] };
            let blob = self.build_ifd_blob_full(
                level, bps, spp, geo_keys_enc,
                !level.is_overview,
                level.is_overview,
                &tile_start_offsets[i],
                next_ifd,
                self.bigtiff,
                ifd_offsets[i],
            );
            ifd_blobs.push(blob);
        }

        // ── Write header ──────────────────────────────────────────────────────
        if self.bigtiff {
            buf.extend_from_slice(b"II");
            buf.extend_from_slice(&BO.u16_bytes(43));
            buf.extend_from_slice(&BO.u16_bytes(8));
            buf.extend_from_slice(&BO.u16_bytes(0));
            buf.extend_from_slice(&BO.u64_bytes(header_size + ghost_size));
        } else {
            buf.extend_from_slice(b"II");
            buf.extend_from_slice(&BO.u16_bytes(42));
            buf.extend_from_slice(&BO.u32_bytes((header_size + ghost_size) as u32));
        }

        // ── Write ghost block ─────────────────────────────────────────────────
        buf.extend_from_slice(&ghost);

        // ── Write IFD blobs ───────────────────────────────────────────────────
        for blob in &ifd_blobs {
            buf.extend_from_slice(blob);
            if buf.len() % 2 != 0 { buf.push(0); }
        }

        // ── Write tile data ───────────────────────────────────────────────────
        for level in levels.iter() {
            for tile in &level.tiles {
                buf.extend_from_slice(tile);
            }
        }

        Ok(())
    }

    // ── Ghost metadata block ──────────────────────────────────────────────────

    fn ghost_metadata_block(&self, levels: &[EncodedLevel]) -> Vec<u8> {
        // GDAL writes a raw ASCII block that looks like:
        //   GDAL_STRUCTURAL_METADATA_SIZE=NNNNNN bytes\n
        //   LAYOUT=COG\n
        //   OVERVIEW_LEVELS=N\n
        //   COMPRESSION=name\n
        //   TILE_SIZE=NNN\n
        // The size field refers to the total length of this block and is
        // padded so the IFD starts on a word boundary.

        let num_overviews = levels.iter().filter(|l| l.is_overview).count();
        let codec_name = self.compression.name().to_uppercase();

        let inner = format!(
            "LAYOUT=COG\nOVERVIEW_COUNT={}\nCOMPRESSION={}\nTILE_SIZE={}\n",
            num_overviews,
            codec_name,
            self.tile_size,
        );
        // Header line includes total size (header line itself + inner)
        let size_field_placeholder = inner.len() + 40; // rough size for the header line
        let header_line = format!("GDAL_STRUCTURAL_METADATA_SIZE={:06} bytes\n", size_field_placeholder);
        let total = header_line.len() + inner.len();
        // Re-do with accurate size
        let header_line = format!("GDAL_STRUCTURAL_METADATA_SIZE={:06} bytes\n", total);

        let mut block = header_line.into_bytes();
        block.extend_from_slice(inner.as_bytes());
        // Pad to even length
        if block.len() % 2 != 0 { block.push(0); }
        block
    }

    // ── IFD blob builder (full, with real offsets) ────────────────────────────

    fn build_ifd_blob(
        &self,
        level: &EncodedLevel,
        bps: u16,
        spp: usize,
        geo_keys_enc: &Option<(Vec<u16>, Vec<f64>, String)>,
        include_geo: bool,
        is_overview: bool,
        _placeholder_tile_offset: u64,
        _next_ifd: u64,
        ifd_base_offset: u64,
    ) -> Vec<u8> {
        // Minimal blob for sizing — real one built by build_ifd_blob_full
        self.build_ifd_blob_full(
            level, bps, spp, geo_keys_enc, include_geo, is_overview,
            &vec![0u64; level.tiles.len()], 0, self.bigtiff, ifd_base_offset,
        )
    }

    fn build_ifd_blob_full(
        &self,
        level: &EncodedLevel,
        bps: u16,
        spp: usize,
        geo_keys_enc: &Option<(Vec<u16>, Vec<f64>, String)>,
        include_geo: bool,
        is_overview: bool,
        tile_offsets: &[u64],
        next_ifd_offset: u64,
        bigtiff: bool,
        ifd_base_offset: u64,
    ) -> Vec<u8> {
        let mut tags: Vec<TiffTag> = Vec::new();
        let ts = self.tile_size;
        let tile_bc_u32: Vec<u32> = level.tiles.iter().map(|t| t.len() as u32).collect();

        if is_overview {
            push_long(&mut tags, tag::NewSubFileType, 1); // reduced-resolution
        }

        push_long(&mut tags, tag::ImageWidth,  level.width);
        push_long(&mut tags, tag::ImageLength, level.height);
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
        push_short(&mut tags, tag::SamplesPerPixel, spp as u32);
        push_long(&mut tags, tag::TileWidth,  ts);
        push_long(&mut tags, tag::TileLength, ts);

        // TileOffsets (LONG8 for bigtiff, LONG for classic)
        if bigtiff {
            let bytes: Vec<u8> = tile_offsets.iter().flat_map(|&v| BO.u64_bytes(v)).collect();
            tags.push(TiffTag { code: tag::TileOffsets, data_type: 16, count: tile_offsets.len() as u32, extra_data: bytes, extra_offset64: 0 });
            let bc_bytes: Vec<u8> = level.tiles.iter().flat_map(|t| BO.u64_bytes(t.len() as u64)).collect();
            tags.push(TiffTag { code: tag::TileByteCounts, data_type: 16, count: level.tiles.len() as u32, extra_data: bc_bytes, extra_offset64: 0 });
        } else {
            let longs: Vec<u32> = tile_offsets.iter().map(|&v| v as u32).collect();
            push_longs(&mut tags, tag::TileOffsets, &longs);
            push_longs(&mut tags, tag::TileByteCounts, &tile_bc_u32);
        }
        push_short(&mut tags, tag::PlanarConfiguration, 1);
        push_short(&mut tags, tag::SampleFormat, self.sample_format.tag_value() as u32);

        if include_geo {
            if let Some(gt) = &self.geo_transform {
                push_doubles(&mut tags, tag::ModelPixelScaleTag, &gt.to_pixel_scale());
                push_doubles(&mut tags, tag::ModelTiepointTag,   &gt.to_tiepoint());
            }
            if let Some((dir, dbl, asc)) = geo_keys_enc {
                push_shorts_u16(&mut tags, tag::GeoKeyDirectoryTag, dir);
                if !dbl.is_empty() { push_doubles(&mut tags, tag::GeoDoubleParamsTag, dbl); }
                if !asc.is_empty() { push_ascii(&mut tags, tag::GeoAsciiParamsTag, asc); }
            }
            if let Some(nd) = self.no_data {
                push_ascii(&mut tags, tag::GdalNodata, &format!("{}", nd));
            }
        }

        tags.sort_by_key(|t| t.code);

        // Serialise IFD into a byte blob
        let mut blob: Vec<u8> = Vec::new();

        if bigtiff {
            blob.extend_from_slice(&BO.u64_bytes(tags.len() as u64));
        } else {
            blob.extend_from_slice(&BO.u16_bytes(tags.len() as u16));
        }

        // First pass: compute extra-data offsets relative to blob start
        let header_len = if bigtiff { 8u64 } else { 2u64 };
        let entry_len  = if bigtiff { 20u64 } else { 12u64 };
        let footer_len = if bigtiff { 8u64 } else { 4u64 };
        let inline_max = if bigtiff { 8usize } else { 4usize };

        let ifd_header_bytes = header_len + tags.len() as u64 * entry_len + footer_len;
        let mut extra_cur: u64 = ifd_header_bytes;

        let mut extra_offsets: Vec<u64> = Vec::with_capacity(tags.len());
        for t in &tags {
            if t.extra_data.len() > inline_max {
                extra_offsets.push(extra_cur + ifd_base_offset);
                extra_cur += t.extra_data.len() as u64;
                if extra_cur % 2 != 0 { extra_cur += 1; }
            } else {
                extra_offsets.push(0);
            }
        }

        // Write IFD entries
        // For COG: tile offsets must be absolute file offsets, which the caller
        // already placed in extra_data.  But the extra_offset in TiffTag is
        // relative to blob-start here; we pass the file offset directly in
        // extra_data for TileOffsets so it will be written inline if it fits,
        // or we trust the caller set it correctly.

        for (t, &ex_off) in tags.iter().zip(extra_offsets.iter()) {
            if bigtiff {
                blob.extend_from_slice(&BO.u16_bytes(t.code));
                blob.extend_from_slice(&BO.u16_bytes(t.data_type));
                blob.extend_from_slice(&BO.u64_bytes(t.count as u64));
                if t.extra_data.len() <= 8 {
                    let mut b = [0u8; 8];
                    b[..t.extra_data.len()].copy_from_slice(&t.extra_data);
                    blob.extend_from_slice(&b);
                } else {
                    blob.extend_from_slice(&BO.u64_bytes(ex_off));
                }
            } else {
                blob.extend_from_slice(&BO.u16_bytes(t.code));
                blob.extend_from_slice(&BO.u16_bytes(t.data_type));
                blob.extend_from_slice(&BO.u32_bytes(t.count));
                if t.extra_data.len() <= 4 {
                    let mut b = [0u8; 4];
                    b[..t.extra_data.len()].copy_from_slice(&t.extra_data);
                    blob.extend_from_slice(&b);
                } else {
                    blob.extend_from_slice(&BO.u32_bytes(ex_off as u32));
                }
            }
        }

        // Next IFD offset
        if bigtiff {
            blob.extend_from_slice(&BO.u64_bytes(next_ifd_offset));
        } else {
            blob.extend_from_slice(&BO.u32_bytes(next_ifd_offset as u32));
        }

        // Extra data
        for (t, &_ex_off) in tags.iter().zip(extra_offsets.iter()) {
            if t.extra_data.len() > inline_max {
                blob.extend_from_slice(&t.extra_data);
                if t.extra_data.len() % 2 != 0 { blob.push(0); }
            }
        }

        blob
    }

    // ── Tile encoder ──────────────────────────────────────────────────────────

    fn encode_tile_set(
        &self,
        pixel_bytes: &[u8],
        width: u32,
        height: u32,
        spp: usize,
        bps_b: usize,
        ts: usize,
    ) -> Result<Vec<Vec<u8>>> {
        let w = width as usize;
        let h = height as usize;
        let tiles_x = (w + ts - 1) / ts;
        let tiles_y = (h + ts - 1) / ts;
        let tile_raw = ts * ts * spp * bps_b;

        let mut out = Vec::with_capacity(tiles_x * tiles_y);
        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let img_x0 = tx * ts;
                let img_y0 = ty * ts;
                let copy_w = ts.min(w.saturating_sub(img_x0));
                let copy_h = ts.min(h.saturating_sub(img_y0));

                let mut tile = vec![0u8; tile_raw];
                for row in 0..copy_h {
                    let src_off = ((img_y0 + row) * w + img_x0) * spp * bps_b;
                    let dst_off = row * ts * spp * bps_b;
                    let len = copy_w * spp * bps_b;
                    if src_off + len <= pixel_bytes.len() {
                        tile[dst_off..dst_off + len]
                            .copy_from_slice(&pixel_bytes[src_off..src_off + len]);
                    }
                }
                out.push(self.compress_chunk(&tile, ts as u32, ts as u32, spp)?);
            }
        }
        Ok(out)
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

    // ── Overview level computation ────────────────────────────────────────────

    fn compute_overview_levels(&self) -> Vec<u32> {
        if let Some(ref explicit) = self.overview_levels {
            return explicit.clone();
        }
        // Generate power-of-two levels until the overview fits in one tile
        let min_dim = self.width.min(self.height) as f64;
        let ts = self.tile_size as f64;
        let num_levels = (min_dim / ts).log2().ceil() as u32;
        (0..num_levels).map(|i| 2u32.pow(i + 1)).collect()
    }

    // ── Overview resampling ───────────────────────────────────────────────────

    fn resample_overview(
        &self,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        spp: usize,
        bps_b: usize,
        dst_w: u32,
        dst_h: u32,
        _factor: u32,
    ) -> Result<Vec<u8>> {
        let sw = src_w as usize;
        let sh = src_h as usize;
        let dw = dst_w as usize;
        let dh = dst_h as usize;
        let mut out = vec![0u8; dw * dh * spp * bps_b];

        match self.resampling {
            Resampling::Nearest => {
                for dy in 0..dh {
                    for dx in 0..dw {
                        let sx = (dx * sw + sw / 2) / dw;
                        let sy = (dy * sh + sh / 2) / dh;
                        let sx = sx.min(sw - 1);
                        let sy = sy.min(sh - 1);
                        let src_off = (sy * sw + sx) * spp * bps_b;
                        let dst_off = (dy * dw + dx) * spp * bps_b;
                        let len = spp * bps_b;
                        out[dst_off..dst_off + len].copy_from_slice(&src[src_off..src_off + len]);
                    }
                }
            }
            Resampling::Average => {
                // 2x2 box average using f64 arithmetic
                for dy in 0..dh {
                    for dx in 0..dw {
                        let x0 = dx * sw / dw;
                        let y0 = dy * sh / dh;
                        let x1 = ((dx + 1) * sw / dw).min(sw);
                        let y1 = ((dy + 1) * sh / dh).min(sh);
                        let count = ((x1 - x0) * (y1 - y0)).max(1) as f64;

                        for band in 0..spp {
                            let mut acc = 0f64;
                            for sy in y0..y1 {
                                for sx in x0..x1 {
                                    let off = (sy * sw + sx) * spp * bps_b + band * bps_b;
                                    acc += bytes_to_f64(&src[off..off + bps_b], self.sample_format);
                                }
                            }
                            let avg = acc / count;
                            let dst_off = (dy * dw + dx) * spp * bps_b + band * bps_b;
                            f64_to_bytes(avg, self.sample_format, &mut out[dst_off..dst_off + bps_b]);
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

// ── EncodedLevel ─────────────────────────────────────────────────────────────

struct EncodedLevel {
    width:       u32,
    height:      u32,
    tiles:       Vec<Vec<u8>>,
    is_overview: bool,
}

// ── Sample byte converters ────────────────────────────────────────────────────

fn bytes_to_f64(bytes: &[u8], fmt: SampleFormat) -> f64 {
    match (fmt, bytes.len()) {
        (SampleFormat::Uint,      1) => bytes[0] as f64,
        (SampleFormat::Uint,      2) => u16::from_le_bytes(bytes.try_into().unwrap_or([0;2])) as f64,
        (SampleFormat::Uint,      4) => u32::from_le_bytes(bytes.try_into().unwrap_or([0;4])) as f64,
        (SampleFormat::Uint,      8) => u64::from_le_bytes(bytes.try_into().unwrap_or([0;8])) as f64,
        (SampleFormat::Int,       1) => bytes[0] as i8 as f64,
        (SampleFormat::Int,       2) => i16::from_le_bytes(bytes.try_into().unwrap_or([0;2])) as f64,
        (SampleFormat::Int,       4) => i32::from_le_bytes(bytes.try_into().unwrap_or([0;4])) as f64,
        (SampleFormat::Int,       8) => i64::from_le_bytes(bytes.try_into().unwrap_or([0;8])) as f64,
        (SampleFormat::IeeeFloat, 4) => f32::from_le_bytes(bytes.try_into().unwrap_or([0;4])) as f64,
        (SampleFormat::IeeeFloat, 8) => f64::from_le_bytes(bytes.try_into().unwrap_or([0;8])),
        _ => 0.0,
    }
}

fn f64_to_bytes(v: f64, fmt: SampleFormat, out: &mut [u8]) {
    match (fmt, out.len()) {
        (SampleFormat::Uint,      1) => out[0] = v.clamp(0.0, 255.0) as u8,
        (SampleFormat::Uint,      2) => out.copy_from_slice(&(v.clamp(0.0, 65535.0) as u16).to_le_bytes()),
        (SampleFormat::Uint,      4) => out.copy_from_slice(&(v.clamp(0.0, u32::MAX as f64) as u32).to_le_bytes()),
        (SampleFormat::Uint,      8) => out.copy_from_slice(&(v.clamp(0.0, u64::MAX as f64) as u64).to_le_bytes()),
        (SampleFormat::Int,       1) => out[0] = v.clamp(-128.0, 127.0) as i8 as u8,
        (SampleFormat::Int,       2) => out.copy_from_slice(&(v.clamp(-32768.0, 32767.0) as i16).to_le_bytes()),
        (SampleFormat::Int,       4) => out.copy_from_slice(&(v.clamp(i32::MIN as f64, i32::MAX as f64) as i32).to_le_bytes()),
        (SampleFormat::Int,       8) => out.copy_from_slice(&(v.clamp(i64::MIN as f64, i64::MAX as f64) as i64).to_le_bytes()),
        (SampleFormat::IeeeFloat, 4) => out.copy_from_slice(&(v as f32).to_le_bytes()),
        (SampleFormat::IeeeFloat, 8) => out.copy_from_slice(&v.to_le_bytes()),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::reader::GeoTiff;
    use super::super::types::GeoTransform;
    use tempfile::NamedTempFile;

    #[test]
    fn cog_readable() {
        let w = 256u32;
        let h = 256u32;
        let data: Vec<f32> = (0..(w * h)).map(|i| i as f32 * 0.01).collect();

        // Use a unique temp file
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        CogWriter::new(w, h, 1)
            .compression(Compression::None)
            .tile_size(64)
            .geo_transform(GeoTransform::north_up(0.0, 1.0, h as f64, -1.0))
            .epsg(4326)
            .write_f32(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert!(tiff.width() > 0);
        assert!(tiff.height() > 0);
        let read_back = tiff.read_band_f32(0).unwrap();
        assert_eq!(read_back.len(), (tiff.width() * tiff.height()) as usize);
    }

    #[test]
    fn overview_levels_computed() {
        let cog = CogWriter::new(4096, 4096, 1).tile_size(512);
        let levels = cog.compute_overview_levels();
        // Should give [2, 4, 8] for a 4096×4096 image with 512 tiles
        assert!(!levels.is_empty());
        assert!(levels.iter().all(|&l| l > 1 && l.is_power_of_two()));
    }

    #[test]
    fn cog_jpeg_u8_readable() {
        let w = 256u32;
        let h = 256u32;
        let data: Vec<u8> = (0..(w * h)).map(|i| ((i * 13) % 256) as u8).collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        CogWriter::new(w, h, 1)
            .compression(Compression::Jpeg)
            .jpeg_quality(80)
            .tile_size(128)
            .write_u8(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert_eq!(tiff.compression(), Compression::Jpeg);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), data.len());
    }

    #[test]
    fn cog_webp_u8_readable() {
        let w = 256u32;
        let h = 256u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 17) % 256) as u8;
                [v, v.wrapping_add(19), v.wrapping_add(37)]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        CogWriter::new(w, h, 3)
            .compression(Compression::WebP)
            .jpeg_quality(80)
            .tile_size(128)
            .write_u8(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert_eq!(tiff.compression(), Compression::WebP);
        assert_eq!(tiff.band_count(), 3);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn cog_webp_u8_rgba_readable() {
        let w = 128u32;
        let h = 128u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 11) % 256) as u8;
                [v, v.wrapping_add(21), v.wrapping_add(43), 180]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        CogWriter::new(w, h, 4)
            .compression(Compression::WebP)
            .jpeg_quality(82)
            .tile_size(128)
            .write_u8(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert_eq!(tiff.compression(), Compression::WebP);
        assert_eq!(tiff.band_count(), 4);
        let read_back = tiff.read_band_u8(3).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn cog_jpegxl_u8_readable() {
        let w = 256u32;
        let h = 256u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 5) % 256) as u8;
                [v, v.wrapping_add(11), v.wrapping_add(23)]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        CogWriter::new(w, h, 3)
            .compression(Compression::JpegXl)
            .jpeg_quality(88)
            .tile_size(128)
            .write_u8(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert_eq!(tiff.compression(), Compression::JpegXl);
        assert_eq!(tiff.band_count(), 3);
        let read_back = tiff.read_band_u8(0).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn cog_jpegxl_u8_rgba_readable() {
        let w = 128u32;
        let h = 128u32;
        let data: Vec<u8> = (0..(w * h))
            .flat_map(|i| {
                let v = ((i * 7) % 256) as u8;
                [v, v.wrapping_add(17), v.wrapping_add(29), 200]
            })
            .collect();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        CogWriter::new(w, h, 4)
            .compression(Compression::JpegXl)
            .jpeg_quality(92)
            .tile_size(128)
            .write_u8(&path, &data)
            .unwrap();

        let tiff = GeoTiff::open(&path).unwrap();
        assert_eq!(tiff.compression(), Compression::JpegXl);
        assert_eq!(tiff.band_count(), 4);
        let read_back = tiff.read_band_u8(3).unwrap();
        assert_eq!(read_back.len(), (w * h) as usize);
    }

    #[test]
    fn cog_integer_roundtrip_i16_u32_i32() {
        let file_i16 = NamedTempFile::new().unwrap();
        let path_i16 = file_i16.path().to_path_buf();
        let data_i16: Vec<i16> = vec![-32768, -1000, -1, 0, 1, 1000, 32767, 42, -42];
        CogWriter::new(3, 3, 1)
            .compression(Compression::Deflate)
            .tile_size(64)
            .write_i16(&path_i16, &data_i16)
            .unwrap();
        let tiff_i16 = GeoTiff::open(&path_i16).unwrap();
        assert_eq!(tiff_i16.read_band_i16(0).unwrap(), data_i16);

        let file_u32 = NamedTempFile::new().unwrap();
        let path_u32 = file_u32.path().to_path_buf();
        let data_u32: Vec<u32> = vec![0, 1, 255, 65_535, 100_000, 1_000_000, u32::MAX - 1, u32::MAX, 77];
        CogWriter::new(3, 3, 1)
            .compression(Compression::Deflate)
            .tile_size(64)
            .write_u32(&path_u32, &data_u32)
            .unwrap();
        let tiff_u32 = GeoTiff::open(&path_u32).unwrap();
        assert_eq!(tiff_u32.read_band_u32(0).unwrap(), data_u32);

        let file_i32 = NamedTempFile::new().unwrap();
        let path_i32 = file_i32.path().to_path_buf();
        let data_i32: Vec<i32> = vec![i32::MIN, -1_000_000, -32_768, -1, 0, 1, 32_767, 1_000_000, i32::MAX];
        CogWriter::new(3, 3, 1)
            .compression(Compression::Deflate)
            .tile_size(64)
            .write_i32(&path_i32, &data_i32)
            .unwrap();
        let tiff_i32 = GeoTiff::open(&path_i32).unwrap();
        assert_eq!(tiff_i32.read_band_i32(0).unwrap(), data_i32);

        let file_u64 = NamedTempFile::new().unwrap();
        let path_u64 = file_u64.path().to_path_buf();
        let data_u64: Vec<u64> = vec![0, 1, 255, 65_535, 1_000_000, 9_007_199_254_740_991, u64::MAX - 1, u64::MAX, 77];
        CogWriter::new(3, 3, 1)
            .compression(Compression::Deflate)
            .tile_size(64)
            .write_u64(&path_u64, &data_u64)
            .unwrap();
        let tiff_u64 = GeoTiff::open(&path_u64).unwrap();
        assert_eq!(tiff_u64.read_band_u64(0).unwrap(), data_u64);

        let file_i64 = NamedTempFile::new().unwrap();
        let path_i64 = file_i64.path().to_path_buf();
        let data_i64: Vec<i64> = vec![i64::MIN, -1_000_000, -32_768, -1, 0, 1, 32_767, 9_007_199_254_740_991, i64::MAX];
        CogWriter::new(3, 3, 1)
            .compression(Compression::Deflate)
            .tile_size(64)
            .write_i64(&path_i64, &data_i64)
            .unwrap();
        let tiff_i64 = GeoTiff::open(&path_i64).unwrap();
        assert_eq!(tiff_i64.read_band_i64(0).unwrap(), data_i64);
    }
}
