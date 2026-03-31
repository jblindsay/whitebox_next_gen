//! High-level JPEG 2000 / GeoJP2 writer.
//!
//! # Example
//! ```rust,ignore
//! use geojp2::{GeoJp2Writer, CompressionMode, GeoTransform};
//!
//! let data: Vec<f32> = vec![0.0; 512 * 512];
//! GeoJp2Writer::new(512, 512, 1)
//!     .compression(CompressionMode::Lossless)
//!     .geo_transform(GeoTransform::north_up(10.0, 0.001, 49.0, -0.001))
//!     .epsg(4326)
//!     .no_data(-9999.0)
//!     .write_f32("output.jp2", &data)
//!     .unwrap();
//! ```

use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use super::boxes::{
    box_type, write_box, write_super_box, write_signature, write_file_type,
    write_uuid_box, write_xml_box, ColourSpec, ImageHeader, ResolutionBox, GEOJP2_UUID,
};
use super::codestream::{marker, write_comment, Cod, Qcd, Siz, Sot};
use super::entropy::encode_block;
use super::error::{Jp2Error, Result};
use super::geo_meta::build_geojp2_payload;
use super::types::{ColorSpace, CompressionMode, GeoTransform};
use super::wavelet::{fwd_dwt_53_multilevel, fwd_dwt_97_multilevel};

// ── GeoJp2Writer ─────────────────────────────────────────────────────────────

/// Builder for writing JPEG 2000 / GeoJP2 files.
///
/// The API closely mirrors [`geotiff::GeoTiffWriter`]:
///
/// ```rust,ignore
/// use geojp2::{GeoJp2Writer, CompressionMode, GeoTransform};
///
/// GeoJp2Writer::new(1024, 1024, 1)
///     .compression(CompressionMode::Lossless)
///     .decomp_levels(5)
///     .geo_transform(GeoTransform::north_up(-180.0, 0.352, 90.0, -0.352))
///     .epsg(4326)
///     .write_f32("output.jp2", &vec![0.0f32; 1024 * 1024])
///     .unwrap();
/// ```
pub struct GeoJp2Writer {
    width:        u32,
    height:       u32,
    components:   u16,
    bits:         u8,
    signed:       bool,
    compression:  CompressionMode,
    decomp_levels: u8,
    code_block_w: u8,   // log2 of code-block width  (default 4 → 16 samples)
    code_block_h: u8,
    color_space:  ColorSpace,
    geo_transform: Option<GeoTransform>,
    epsg:         Option<u16>,
    no_data:      Option<f64>,
    comment:      Option<String>,
    /// If set, embed an `xml ` box with this GML/XML string.
    xml_metadata: Option<String>,
}

impl GeoJp2Writer {
    /// Create a new writer for a `width × height × components` raster.
    pub fn new(width: u32, height: u32, components: u16) -> Self {
        Self {
            width, height, components,
            bits:         8,
            signed:       false,
            compression:  CompressionMode::Lossless,
            decomp_levels: 5,
            code_block_w: 4,
            code_block_h: 4,
            color_space:  if components == 3 { ColorSpace::Srgb } else if components == 1 { ColorSpace::Greyscale } else { ColorSpace::MultiBand },
            geo_transform: None,
            epsg:         None,
            no_data:      None,
            comment:      Some("Created by geojp2-rs".into()),
            xml_metadata: None,
        }
    }

    // ── Builder setters ───────────────────────────────────────────────────────

    /// Set the compression mode (lossless or lossy with target quality).
    pub fn compression(mut self, c: CompressionMode) -> Self { self.compression = c; self }
    /// Number of DWT decomposition levels (1–32; default 5).
    pub fn decomp_levels(mut self, n: u8) -> Self { self.decomp_levels = n.clamp(1, 15); self }
    /// Manually set bit depth (default inferred from data type).
    pub fn bits_per_sample(mut self, b: u8) -> Self { self.bits = b; self }
    /// Set the JP2 colour space.
    pub fn color_space(mut self, cs: ColorSpace) -> Self { self.color_space = cs; self }
    /// Set the affine geo-transform.
    pub fn geo_transform(mut self, gt: GeoTransform) -> Self { self.geo_transform = Some(gt); self }
    /// Set a codestream comment string.
    pub fn comment(mut self, c: impl Into<String>) -> Self { self.comment = Some(c.into()); self }
    /// Embed an `xml ` box with the provided GML/XML string.
    pub fn xml_metadata(mut self, xml: impl Into<String>) -> Self { self.xml_metadata = Some(xml.into()); self }
    /// Set the GDAL-style NODATA value.
    pub fn no_data(mut self, v: f64) -> Self { self.no_data = Some(v); self }
    /// Set the EPSG code for the CRS.
    pub fn epsg(mut self, code: u16) -> Self { self.epsg = Some(code); self }

    // ── Typed write entry points ──────────────────────────────────────────────

    /// Write `u8` samples to a JP2 file.
    pub fn write_u8<P: AsRef<Path>>(mut self, path: P, data: &[u8]) -> Result<()> {
        self.bits = 8; self.signed = false;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        let f = File::create(path).map_err(Jp2Error::Io)?;
        self.write_raw(BufWriter::new(f), &ints)
    }

    /// Write `u16` samples to a JP2 file.
    pub fn write_u16<P: AsRef<Path>>(mut self, path: P, data: &[u16]) -> Result<()> {
        self.bits = 16; self.signed = false;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        let f = File::create(path).map_err(Jp2Error::Io)?;
        self.write_raw(BufWriter::new(f), &ints)
    }

    /// Write `i16` samples to a JP2 file.
    pub fn write_i16<P: AsRef<Path>>(mut self, path: P, data: &[i16]) -> Result<()> {
        self.bits = 16; self.signed = true;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        let f = File::create(path).map_err(Jp2Error::Io)?;
        self.write_raw(BufWriter::new(f), &ints)
    }

    /// Write `f32` samples to a JP2 file (quantised to 32-bit integers internally).
    pub fn write_f32<P: AsRef<Path>>(mut self, path: P, data: &[f32]) -> Result<()> {
        self.bits = 32; self.signed = true;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        let f = File::create(path).map_err(Jp2Error::Io)?;
        self.write_raw(BufWriter::new(f), &ints)
    }

    /// Write `f64` samples to a JP2 file.
    pub fn write_f64<P: AsRef<Path>>(mut self, path: P, data: &[f64]) -> Result<()> {
        self.bits = 32; self.signed = true;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        let f = File::create(path).map_err(Jp2Error::Io)?;
        self.write_raw(BufWriter::new(f), &ints)
    }

    /// Write `u16` samples to any `Write` (for in-memory use / testing).
    pub fn write_u16_to_writer<W: Write + Seek>(mut self, w: W, data: &[u16]) -> Result<()> {
        self.bits = 16; self.signed = false;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        self.write_raw(w, &ints)
    }

    /// Write `f32` samples to any `Write` (for in-memory use / testing).
    pub fn write_f32_to_writer<W: Write + Seek>(mut self, w: W, data: &[f32]) -> Result<()> {
        self.bits = 32; self.signed = true;
        self.validate(data.len())?;
        let ints: Vec<i32> = data.iter().map(|&v| v as i32).collect();
        self.write_raw(w, &ints)
    }

    // ── Core writer ───────────────────────────────────────────────────────────

    fn validate(&self, n: usize) -> Result<()> {
        let expected = self.width as usize * self.height as usize * self.components as usize;
        if n != expected { Err(Jp2Error::DataSizeMismatch { expected, actual: n }) } else { Ok(()) }
    }

    fn write_raw<W: Write + Seek>(&self, mut w: W, pixels: &[i32]) -> Result<()> {
        // ── Build the JPEG 2000 codestream ────────────────────────────────
        let codestream = self.encode_codestream(pixels)?;

        // ── Build JP2 boxes ───────────────────────────────────────────────
        let mut buf = Vec::new();

        // 1. Signature
        write_signature(&mut buf).map_err(Jp2Error::Io)?;

        // 2. File type
        write_file_type(&mut buf).map_err(Jp2Error::Io)?;

        // 3. JP2 Header superbox
        let bpc = if self.signed { 0x80 | (self.bits - 1) } else { self.bits - 1 };
        let ihdr = ImageHeader {
            height: self.height, width: self.width,
            components: self.components,
            bpc, c: 7, unk_c: 0, ipr: 0,
        };
        let colr = ColourSpec::enumerated(self.color_space.enumcs());
        let mut jp2h_payload = Vec::new();
        ihdr.write(&mut jp2h_payload).map_err(Jp2Error::Io)?;
        colr.write(&mut jp2h_payload).map_err(Jp2Error::Io)?;

        // Optional: capture resolution (72 dpi)
        let res = ResolutionBox { vr_n: 72, vr_d: 1, hr_n: 72, hr_d: 1, vr_e: 0, hr_e: 0 };
        res.write(&mut jp2h_payload).map_err(Jp2Error::Io)?;

        write_super_box(&mut buf, box_type::JP2_HEADER, &jp2h_payload).map_err(Jp2Error::Io)?;

        // 4. GeoJP2 UUID box (if geo metadata available)
        if self.geo_transform.is_some() || self.epsg.is_some() {
            let model_type = if self.epsg.map_or(false, |e| e / 1000 == 4) { 2u16 } else { 1 };
            let payload = build_geojp2_payload(
                self.geo_transform.as_ref(),
                self.epsg,
                model_type,
                self.no_data,
            );
            write_uuid_box(&mut buf, &GEOJP2_UUID, &payload).map_err(Jp2Error::Io)?;
        }

        // 5. Optional XML box
        if let Some(ref xml) = self.xml_metadata {
            write_xml_box(&mut buf, xml).map_err(Jp2Error::Io)?;
        }

        // 6. Codestream box (jp2c)
        write_box(&mut buf, box_type::CODESTREAM, &codestream).map_err(Jp2Error::Io)?;

        w.write_all(&buf).map_err(Jp2Error::Io)?;
        w.flush().map_err(Jp2Error::Io)
    }

    // ── JPEG 2000 codestream encoder ──────────────────────────────────────────

    fn encode_codestream(&self, pixels: &[i32]) -> Result<Vec<u8>> {
        let w  = self.width  as usize;
        let h  = self.height as usize;
        let nc = self.components as usize;
        let nl = self.decomp_levels;
        let lossless = self.compression.is_lossless();

        let quality_db = match self.compression {
            CompressionMode::Lossy { quality_db } => quality_db,
            _ => 40.0,
        };

        let siz = Siz::new(self.width, self.height, self.bits, self.signed, self.components);
        let cod = if lossless { Cod::lossless(nl, self.components) } else { Cod::lossy(nl, self.components) };
        let qcd = if lossless {
            Qcd::no_quantisation(nl, self.bits)
        } else {
            Qcd::scalar_expounded(nl, self.bits, quality_db)
        };

        let mut cs: Vec<u8> = Vec::new();

        // SOC
        cs.extend_from_slice(&marker::SOC.to_be_bytes());

        // Main header markers
        siz.write(&mut cs).map_err(Jp2Error::Io)?;
        cod.write(&mut cs).map_err(Jp2Error::Io)?;
        qcd.write(&mut cs).map_err(Jp2Error::Io)?;

        // Comment
        if let Some(ref cmt) = self.comment {
            write_comment(&mut cs, cmt).map_err(Jp2Error::Io)?;
        }

        // ── Encode tiles ──────────────────────────────────────────────────
        // Single-tile encoding: all components in one tile-part
        let num_tiles = 1u32; // single-tile for simplicity

        for tile_idx in 0..num_tiles {
            // Gather tile pixel data for each component
            let mut tile_body: Vec<u8> = Vec::new();

            for c in 0..nc {
                // Extract component samples (un-shifted)
                let comp_pixels: Vec<i32> = (0..w * h)
                    .map(|p| {
                        let raw = pixels[p * nc + c];
                        // DC level shift for unsigned data: subtract 2^(bits-1)
                        if !self.signed {
                            raw - (1 << (self.bits.saturating_sub(1)) as i32)
                        } else {
                            raw
                        }
                    })
                    .collect();

                // Forward DWT
                let encoded_ints = if lossless {
                    let mut coeff = comp_pixels.clone();
                    fwd_dwt_53_multilevel(&mut coeff, w, h, nl);
                    coeff
                } else {
                    let float_coeffs = fwd_dwt_97_multilevel(&comp_pixels, w, h, nl);
                    // Quantise
                    let step_sizes: Vec<f64> = qcd.step_sizes.iter()
                        .map(|&s| {
                            let exp = (s >> 11) as i32;
                            let mant = (s & 0x7FF) as f64;
                            (1.0 + mant / 2048.0) * 2.0f64.powi(exp - self.bits as i32)
                        })
                        .collect();
                    super::entropy::quantise(&float_coeffs, &step_sizes)
                };

                // Entropy encode
                let compressed = encode_block(&encoded_ints, w, h);
                tile_body.extend_from_slice(&compressed);
            }

            // Write SOT
            let psot = (12 + 2 + tile_body.len()) as u32; // SOT(12) + SOD(2) + data
            let sot = Sot { isot: tile_idx as u16, psot, tpsot: 0, tnsot: 1 };
            sot.write(&mut cs).map_err(Jp2Error::Io)?;

            // SOD
            cs.extend_from_slice(&marker::SOD.to_be_bytes());

            // Tile data
            cs.extend_from_slice(&tile_body);
        }

        // EOC
        cs.extend_from_slice(&marker::EOC.to_be_bytes());

        Ok(cs)
    }
}
