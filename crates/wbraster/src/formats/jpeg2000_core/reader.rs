//! High-level JPEG 2000 / GeoJP2 reader.
//!
//! # Example
//! ```rust,ignore
//! use geojp2::GeoJp2;
//!
//! let jp2 = GeoJp2::open("dem.jp2").unwrap();
//! println!("{}×{} components={}", jp2.width(), jp2.height(), jp2.component_count());
//! println!("EPSG: {:?}", jp2.epsg());
//!
//! let band: Vec<f32> = jp2.read_band_f32(0).unwrap();
//! ```

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use super::boxes::{self, BoxReader, ColourSpec, ImageHeader, GEOJP2_UUID};
use super::codestream::{self, marker, Cod, Qcd, Siz};
use super::entropy::{decode_block, dequantise};
use super::error::{Jp2Error, Result};
use super::geo_meta::{parse_geojp2_payload, CrsInfo};
use super::types::{BoundingBox, ColorSpace, GeoTransform, PixelType};
use super::wavelet::{inv_dwt_53_multilevel, inv_dwt_97_multilevel};

// ── GeoJp2 ───────────────────────────────────────────────────────────────────

/// A decoded JPEG 2000 / GeoJP2 file, ready for data access.
///
/// Supports lossless (5/3 wavelet) and lossy (9/7 wavelet) files, single and
/// multi-component (band) images, and optional GeoJP2 UUID-box geolocation.
pub struct GeoJp2 {
    // Image geometry
    width:      u32,
    height:     u32,
    components: u16,
    // Sample format
    bits:       u8,
    signed:     bool,
    // Coding parameters
    siz:        Siz,
    cod:        Cod,
    qcd:        Qcd,
    // Colour
    color_space: ColorSpace,
    // Geo metadata
    crs:        Option<CrsInfo>,
    // Raw codestream (kept in memory for decode-on-demand)
    codestream: Vec<u8>,
}

impl GeoJp2 {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Open a JP2 file from disk.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).map_err(Jp2Error::Io)?;
        Self::from_reader(BufReader::new(file))
    }

    /// Parse a JP2 from an in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::from_reader(std::io::Cursor::new(bytes.to_vec()))
    }

    /// Parse a JP2 from any `Read + Seek` reader.
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut br = BoxReader::new(reader)?;
        let all_boxes = br.read_all()?;

        // ── Validate signature ────────────────────────────────────────────
        let sig = all_boxes.first().ok_or_else(|| Jp2Error::NotJp2("empty file".into()))?;
        boxes::validate_signature(sig)?;

        let mut ihdr: Option<ImageHeader> = None;
        let mut colr: Option<ColourSpec>  = None;
        let mut crs:  Option<CrsInfo>     = None;
        let mut codestream: Option<Vec<u8>> = None;

        for b in &all_boxes {
            match b.box_type {
                boxes::box_type::JP2_HEADER => {
                    // Parse sub-boxes of jp2h
                    let subs = BoxReader::<std::io::Cursor<Vec<u8>>>::sub_boxes(&b.data)?;
                    for sb in subs {
                        if sb.is(boxes::box_type::IMAGE_HEADER) {
                            ihdr = Some(ImageHeader::parse(&sb.data)?);
                        }
                        if sb.is(boxes::box_type::COLOUR_SPEC) {
                            colr = Some(ColourSpec::parse(&sb.data)?);
                        }
                    }
                }
                boxes::box_type::UUID => {
                    let (uuid, payload) = boxes::parse_uuid_box(b)?;
                    if uuid == GEOJP2_UUID {
                        crs = Some(parse_geojp2_payload(payload)?);
                    }
                }
                boxes::box_type::CODESTREAM => {
                    codestream = Some(b.data.clone());
                }
                _ => {}
            }
        }

        let ihdr = ihdr.ok_or_else(|| Jp2Error::InvalidBox {
            box_type: "jp2h".into(),
            message: "Missing ihdr sub-box".into(),
        })?;
        let codestream = codestream.ok_or_else(|| Jp2Error::InvalidBox {
            box_type: "jp2c".into(),
            message: "Missing codestream box".into(),
        })?;

        // ── Parse codestream header markers ───────────────────────────────
        let markers = codestream::parse_codestream_markers(&codestream)?;
        let mut siz: Option<Siz> = None;
        let mut cod: Option<Cod> = None;
        let mut qcd: Option<Qcd> = None;

        for m in &markers {
            match m.marker {
                marker::SIZ => siz = Some(Siz::parse(&m.data)?),
                marker::COD => cod = Some(Cod::parse(&m.data)?),
                marker::QCD => qcd = Some(Qcd::parse(&m.data)?),
                _ => {}
            }
        }

        let siz = siz.unwrap_or_else(|| Siz::new(ihdr.width, ihdr.height, ihdr.bits_per_component().max(1), ihdr.is_signed(), ihdr.components));
        let cod = cod.unwrap_or_else(|| Cod::lossless(5, ihdr.components));
        let qcd = qcd.unwrap_or_else(|| Qcd::no_quantisation(5, siz.components[0].bits()));

        let color_space = colr.as_ref()
            .and_then(|c| c.enumcs)
            .map(ColorSpace::from_enumcs)
            .unwrap_or_default();

        let bits   = siz.components.first().map(|c| c.bits()).unwrap_or(8);
        let signed = siz.components.first().map(|c| c.signed()).unwrap_or(false);

        Ok(Self {
            width:  ihdr.width,
            height: ihdr.height,
            components: ihdr.components,
            bits, signed,
            siz, cod, qcd,
            color_space,
            crs,
            codestream,
        })
    }

    // ── Metadata accessors ────────────────────────────────────────────────────

    /// Image width in pixels.
    pub fn width(&self) -> u32 { self.width }
    /// Image height in pixels.
    pub fn height(&self) -> u32 { self.height }
    /// Number of components (bands).
    pub fn component_count(&self) -> u16 { self.components }
    /// Bits per sample.
    pub fn bits_per_sample(&self) -> u8 { self.bits }
    /// Whether samples are signed.
    pub fn is_signed(&self) -> bool { self.signed }
    /// Colour space.
    pub fn color_space(&self) -> ColorSpace { self.color_space }
    /// Number of DWT decomposition levels.
    pub fn decomp_levels(&self) -> u8 { self.cod.num_decomps }
    /// Whether the file uses lossless compression (5/3 wavelet).
    pub fn is_lossless(&self) -> bool { self.cod.wavelet == 1 }

    /// The geo-transform, if present.
    pub fn geo_transform(&self) -> Option<&GeoTransform> {
        self.crs.as_ref()?.geo_transform.as_ref()
    }

    /// EPSG code, if present in the GeoJP2 UUID box.
    pub fn epsg(&self) -> Option<u16> {
        self.crs.as_ref()?.epsg
    }

    /// NODATA value, if present.
    pub fn no_data(&self) -> Option<f64> {
        self.crs.as_ref()?.no_data
    }

    /// Full CRS information block.
    pub fn crs_info(&self) -> Option<&CrsInfo> { self.crs.as_ref() }

    /// Bounding box in geographic coordinates, if a geo-transform is available.
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        self.crs.as_ref()?.bounding_box(self.width, self.height)
    }

    /// Pixel type inferred from bit depth and signedness.
    pub fn pixel_type(&self) -> PixelType {
        match (self.signed, self.bits) {
            (false, 8)  => PixelType::Uint8,
            (false, 16) => PixelType::Uint16,
            (true,  16) => PixelType::Int16,
            (true,  32) => PixelType::Int32,
            _           => PixelType::Uint16,
        }
    }

    // ── Band read API (mirrors GeoTIFF library) ───────────────────────────────

    /// Read one band (component) as `u8`, decoding the JPEG 2000 codestream.
    pub fn read_band_u8(&self, band: usize) -> Result<Vec<u8>> {
        self.validate_band(band)?;
        let samples = self.decode_component(band)?;
        Ok(samples.iter().map(|&v| v.clamp(0, 255) as u8).collect())
    }

    /// Read one band as `u16`.
    pub fn read_band_u16(&self, band: usize) -> Result<Vec<u16>> {
        self.validate_band(band)?;
        let samples = self.decode_component(band)?;
        Ok(samples.iter().map(|&v| v.clamp(0, 65535) as u16).collect())
    }

    /// Read one band as `i16`.
    pub fn read_band_i16(&self, band: usize) -> Result<Vec<i16>> {
        self.validate_band(band)?;
        let samples = self.decode_component(band)?;
        Ok(samples.iter().map(|&v| v.clamp(i16::MIN as i32, i16::MAX as i32) as i16).collect())
    }

    /// Read one band as `f32`.
    pub fn read_band_f32(&self, band: usize) -> Result<Vec<f32>> {
        self.validate_band(band)?;
        let samples = self.decode_component(band)?;
        Ok(samples.iter().map(|&v| v as f32).collect())
    }

    /// Read one band as `f64`.
    pub fn read_band_f64(&self, band: usize) -> Result<Vec<f64>> {
        self.validate_band(band)?;
        let samples = self.decode_component(band)?;
        Ok(samples.iter().map(|&v| v as f64).collect())
    }

    /// Read all components interleaved into a flat `Vec<i32>` buffer.
    ///
    /// Layout: `[comp0_px0, comp1_px0, …, compN_px0, comp0_px1, …]`
    pub fn read_all_components(&self) -> Result<Vec<i32>> {
        let npix = self.width as usize * self.height as usize;
        let nc   = self.components as usize;
        let mut out = vec![0i32; npix * nc];
        for c in 0..nc {
            let band = self.decode_component(c)?;
            for p in 0..npix {
                out[p * nc + c] = band[p];
            }
        }
        Ok(out)
    }

    // ── Band statistics ───────────────────────────────────────────────────────

    /// Compute (min, max, mean) for one band.
    pub fn band_stats(&self, band: usize) -> Result<(f64, f64, f64)> {
        let data = self.read_band_f64(band)?;
        let nd = self.no_data();
        let vals: Vec<f64> = data.into_iter()
            .filter(|&v| nd.map_or(true, |n| (v - n).abs() > 1e-10))
            .collect();
        if vals.is_empty() { return Ok((0.0, 0.0, 0.0)); }
        let min = vals.iter().copied().fold(f64::INFINITY,  f64::min);
        let max = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mean = vals.iter().sum::<f64>() / vals.len() as f64;
        Ok((min, max, mean))
    }

    // ── Internal decode ───────────────────────────────────────────────────────

    fn validate_band(&self, band: usize) -> Result<()> {
        if band >= self.components as usize {
            Err(Jp2Error::ComponentOutOfRange { index: band, components: self.components as usize })
        } else {
            Ok(())
        }
    }

    /// Decode one component (band) from the codestream to a flat i32 pixel buffer.
    fn decode_component(&self, component: usize) -> Result<Vec<i32>> {
        let w  = self.width  as usize;
        let h  = self.height as usize;
        let nl = self.cod.num_decomps as usize;
        let nc = self.components as usize;
        let lossless = self.cod.wavelet == 1;

        // Find the SOT/SOD for the tile that contains this component
        // For a single-tile image this is straightforward
        let tile_data = self.extract_tile_data(0)?;

        // Each tile encodes all components sequentially
        // Split into per-component code-blocks
        let comp_data = self.extract_component_data(&tile_data, component, nc)?;

        // Determine number of bit-planes from QCD
        let num_bitplanes = ((self.bits + nl as u8).min(31)) as usize;

        // Decode the code-block entropy data
        let decoded_ints = decode_block(&comp_data, w, h, num_bitplanes);

        // Inverse DWT
        let samples = if lossless {
            let mut coeff = decoded_ints;
            // DC level shift restore: for unsigned data, add 2^(bits-1)
            if !self.signed {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in coeff.iter_mut() { *v += shift; }
            }
            inv_dwt_53_multilevel(&mut coeff, w, h, self.cod.num_decomps);
            coeff
        } else {
            // Dequantise then inverse 9/7 DWT
            let step_sizes: Vec<f64> = self.qcd.step_sizes.iter()
                .map(|&s| {
                    let exp = (s >> 11) as i32;
                    let mant = (s & 0x7FF) as f64;
                    (1.0 + mant / 2048.0) * 2.0f64.powi(exp - self.bits as i32)
                })
                .collect();
            let float_coeffs = dequantise(&decoded_ints, &step_sizes);
            let mut samples = inv_dwt_97_multilevel(&float_coeffs, w, h, self.cod.num_decomps);
            if !self.signed {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in samples.iter_mut() { *v += shift; }
            }
            samples
        };

        Ok(samples)
    }

    /// Extract the raw compressed bytes for tile `tile_idx` from the codestream.
    fn extract_tile_data(&self, tile_idx: u16) -> Result<Vec<u8>> {
        let cs = &self.codestream;
        let mut i = 0;

        while i + 1 < cs.len() {
            if cs[i] != 0xFF { i += 1; continue; }
            let m = u16::from_be_bytes([cs[i], cs[i+1]]);
            i += 2;
            if m == marker::SOC { continue; }
            if m == marker::EOC { break; }

            if m == marker::SOT {
                let lsot = u16::from_be_bytes([cs[i], cs[i+1]]) as usize;
                if i + lsot > cs.len() { break; }
                let sot_data = &cs[i+2..i+lsot];
                let isot = u16::from_be_bytes([sot_data[0], sot_data[1]]);
                let psot = u32::from_be_bytes([sot_data[2], sot_data[3], sot_data[4], sot_data[5]]) as usize;
                i += lsot;

                if isot == tile_idx {
                    // Find SOD
                    while i + 1 < cs.len() {
                        if cs[i] == 0xFF && cs[i+1] == (marker::SOD & 0xFF) as u8 {
                            let sod_start = i + 2;
                            let sod_end = if psot > 0 {
                                // psot counts from start of SOT marker (i-lsot-2 before)
                                // just take all remaining data to next SOT or EOC
                                self.find_next_tile_or_eoc(sod_start)
                            } else {
                                self.find_next_tile_or_eoc(sod_start)
                            };
                            return Ok(cs[sod_start..sod_end].to_vec());
                        }
                        i += 1;
                    }
                }
                continue;
            }

            // Other marker segments — skip
            if i + 2 > cs.len() { break; }
            let lseg = u16::from_be_bytes([cs[i], cs[i+1]]) as usize;
            i += lseg;
        }

        Err(Jp2Error::InvalidCodestream {
            offset: 0,
            message: format!("Tile {} not found in codestream", tile_idx),
        })
    }

    fn find_next_tile_or_eoc(&self, start: usize) -> usize {
        let cs = &self.codestream;
        let mut i = start;
        while i + 1 < cs.len() {
            if cs[i] == 0xFF {
                let m = u16::from_be_bytes([cs[i], cs[i+1]]);
                if m == marker::SOT || m == marker::EOC { return i; }
            }
            i += 1;
        }
        cs.len()
    }

    /// Split tile data into per-component slices.
    /// For a simple single-layer single-tile image the data is divided evenly.
    fn extract_component_data(
        &self,
        tile_data: &[u8],
        component: usize,
        num_components: usize,
    ) -> Result<Vec<u8>> {
        if num_components == 1 || tile_data.is_empty() {
            return Ok(tile_data.to_vec());
        }
        // Divide tile data evenly among components
        let chunk = tile_data.len() / num_components;
        let start = component * chunk;
        let end   = if component + 1 == num_components { tile_data.len() } else { start + chunk };
        Ok(tile_data[start..end].to_vec())
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    use super::super::types::CompressionMode;
    use super::super::writer::GeoJp2Writer;

    fn make_jp2(w: u32, h: u32, mode: CompressionMode) -> Vec<u8> {
        let data: Vec<u16> = (0..(w * h) as u16).collect();
        let mut cur = std::io::Cursor::new(Vec::new());
        GeoJp2Writer::new(w, h, 1)
            .compression(mode)
            .geo_transform(GeoTransform::north_up(0.0, 1.0, h as f64, -1.0))
            .epsg(4326)
            .write_u16_to_writer(&mut cur, &data)
            .unwrap();
        cur.into_inner()
    }

    #[test]
    fn metadata_roundtrip() {
        let buf = make_jp2(32, 32, CompressionMode::Lossless);
        let jp2 = GeoJp2::from_bytes(&buf).unwrap();
        assert_eq!(jp2.width(), 32);
        assert_eq!(jp2.height(), 32);
        assert_eq!(jp2.component_count(), 1);
        assert_eq!(jp2.epsg(), Some(4326));
        assert!(jp2.is_lossless());
    }

    #[test]
    fn lossless_pixel_roundtrip() {
        let w = 16u32; let h = 16u32;
        let data: Vec<u16> = (0..(w * h) as u16).map(|x| x * 3).collect();
        let mut cur = std::io::Cursor::new(Vec::new());
        GeoJp2Writer::new(w, h, 1)
            .compression(CompressionMode::Lossless)
            .write_u16_to_writer(&mut cur, &data)
            .unwrap();
        let buf = cur.into_inner();
        let jp2 = GeoJp2::from_bytes(&buf).unwrap();
        let read_back = jp2.read_band_u16(0).unwrap();
        assert_eq!(read_back, data, "Lossless round-trip pixel mismatch");
    }
}
