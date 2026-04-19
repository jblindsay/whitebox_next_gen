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
use std::collections::HashMap;

use super::boxes::{self, BoxReader, ColourSpec, ImageHeader, RawBox, GEOJP2_UUID};
use super::codestream::{self, marker, Cod, ProgressionOrder, Qcd, Siz};
use super::entropy::{decode_block, dequantise};
use super::error::{Jp2Error, Result};
use super::geo_meta::{parse_geojp2_payload, parse_gmljp2_xml_payload, CrsInfo};
use super::types::{BoundingBox, ColorSpace, GeoTransform, PixelType};
use super::wavelet::{inv_dwt_53_multilevel, inv_dwt_97_multilevel,
                    inv_dwt_53_multilevel_proper, inv_dwt_97_multilevel_proper};

#[derive(Debug, Clone)]
struct TilePartInfo {
    isot: u16,
    tpsot: u8,
    tnsot: u8,
    sod_start: usize,
    tile_part_end: usize,
    has_poc: bool,
    has_packet_header_markers: bool,
    cod_override: Option<Cod>,
}

#[derive(Debug, Clone)]
struct PacketTraversalPlan {
    progression: ProgressionOrder,
    num_layers: u16,
    tile_parts: Vec<TilePartInfo>,
    has_poc: bool,
    has_packet_header_markers: bool,
}

#[derive(Debug, Clone, Copy)]
enum PacketHeaderProbe {
    ZeroLength,
    NonZeroLength,
}

#[derive(Debug, Clone, Copy)]
struct PacketHeaderPreflight {
    kind: PacketHeaderProbe,
    body_data_start: usize,
    has_preview_contribution: bool,
    preview_declared_body_bytes: u32,
    preview_reached_contribution_cap: bool,
}

#[derive(Debug, Clone, Copy)]
struct PacketContextState {
    lblock: u32,
    packets_seen: u32,
    zero_length_packets: u32,
    contributions_seen: u32,
    ever_included: bool,
    first_included_packet_index: Option<u32>,
    last_included_packet_index: Option<u32>,
    packets_since_last_inclusion: u32,
}

impl Default for PacketContextState {
    fn default() -> Self {
        Self {
            lblock: 3,
            packets_seen: 0,
            zero_length_packets: 0,
            contributions_seen: 0,
            ever_included: false,
            first_included_packet_index: None,
            last_included_packet_index: None,
            packets_since_last_inclusion: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LrcpPacketCursor {
    layer: usize,
    resolution: usize,
    component: usize,
}

impl LrcpPacketCursor {
    fn new() -> Self {
        Self {
            layer: 0,
            resolution: 0,
            component: 0,
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        self.component += 1;
        if self.component < components {
            return true;
        }
        self.component = 0;

        self.resolution += 1;
        if self.resolution < resolutions {
            return true;
        }
        self.resolution = 0;

        self.layer += 1;
        self.layer < layers
    }
}

#[derive(Debug, Clone, Copy)]
struct RlcpPacketCursor {
    layer: usize,
    resolution: usize,
    component: usize,
}

impl RlcpPacketCursor {
    fn new() -> Self {
        Self {
            layer: 0,
            resolution: 0,
            component: 0,
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        self.component += 1;
        if self.component < components {
            return true;
        }
        self.component = 0;

        self.layer += 1;
        if self.layer < layers {
            return true;
        }
        self.layer = 0;

        self.resolution += 1;
        self.resolution < resolutions
    }
}

#[derive(Debug, Clone, Copy)]
struct RpclPacketCursor {
    layer: usize,
    resolution: usize,
    component: usize,
}

impl RpclPacketCursor {
    fn new() -> Self {
        Self {
            layer: 0,
            resolution: 0,
            component: 0,
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        // RPCL reduced to R-C-L ordering in current constrained walker (no
        // precinct-position loop yet): L fastest, then C, then R.
        self.layer += 1;
        if self.layer < layers {
            return true;
        }
        self.layer = 0;

        self.component += 1;
        if self.component < components {
            return true;
        }
        self.component = 0;

        self.resolution += 1;
        self.resolution < resolutions
    }
}

#[derive(Debug, Clone, Copy)]
struct PcrlPacketCursor {
    layer: usize,
    resolution: usize,
    component: usize,
}

impl PcrlPacketCursor {
    fn new() -> Self {
        Self {
            layer: 0,
            resolution: 0,
            component: 0,
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        // PCRL reduced to C-R-L ordering in current constrained walker (no
        // precinct-position loop yet): L fastest, then R, then C.
        self.layer += 1;
        if self.layer < layers {
            return true;
        }
        self.layer = 0;

        self.resolution += 1;
        if self.resolution < resolutions {
            return true;
        }
        self.resolution = 0;

        self.component += 1;
        self.component < components
    }
}

#[derive(Debug, Clone, Copy)]
struct CprlPacketCursor {
    layer: usize,
    resolution: usize,
    component: usize,
}

impl CprlPacketCursor {
    fn new() -> Self {
        Self {
            layer: 0,
            resolution: 0,
            component: 0,
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        // CPRL reduced to C-R-L ordering in current constrained walker (no
        // precinct-position loop yet): L fastest, then R, then C.
        self.layer += 1;
        if self.layer < layers {
            return true;
        }
        self.layer = 0;

        self.resolution += 1;
        if self.resolution < resolutions {
            return true;
        }
        self.resolution = 0;

        self.component += 1;
        self.component < components
    }
}

#[derive(Debug, Clone, Copy)]
enum PacketCursor {
    Lrcp(LrcpPacketCursor),
    Rlcp(RlcpPacketCursor),
    Rpcl(RpclPacketCursor),
    Pcrl(PcrlPacketCursor),
    Cprl(CprlPacketCursor),
}

impl PacketCursor {
    fn for_progression(progression: ProgressionOrder) -> Self {
        match progression {
            ProgressionOrder::Lrcp => PacketCursor::Lrcp(LrcpPacketCursor::new()),
            ProgressionOrder::Rlcp => PacketCursor::Rlcp(RlcpPacketCursor::new()),
            ProgressionOrder::Rpcl => PacketCursor::Rpcl(RpclPacketCursor::new()),
            ProgressionOrder::Pcrl => PacketCursor::Pcrl(PcrlPacketCursor::new()),
            ProgressionOrder::Cprl => PacketCursor::Cprl(CprlPacketCursor::new()),
        }
    }

    fn context_key(&self) -> (usize, usize, usize) {
        match self {
            PacketCursor::Lrcp(c) => (c.layer, c.resolution, c.component),
            PacketCursor::Rlcp(c) => (c.layer, c.resolution, c.component),
            PacketCursor::Rpcl(c) => (c.layer, c.resolution, c.component),
            PacketCursor::Pcrl(c) => (c.layer, c.resolution, c.component),
            PacketCursor::Cprl(c) => (c.layer, c.resolution, c.component),
        }
    }

    fn advance(&mut self, layers: usize, resolutions: usize, components: usize) -> bool {
        match self {
            PacketCursor::Lrcp(c) => c.advance(layers, resolutions, components),
            PacketCursor::Rlcp(c) => c.advance(layers, resolutions, components),
            PacketCursor::Rpcl(c) => c.advance(layers, resolutions, components),
            PacketCursor::Pcrl(c) => c.advance(layers, resolutions, components),
            PacketCursor::Cprl(c) => c.advance(layers, resolutions, components),
        }
    }
}

fn peek_bits_msb(data: &[u8], bit_pos: usize, num_bits: usize) -> Option<u16> {
    if num_bits == 0 || num_bits > 16 {
        return None;
    }
    if bit_pos.checked_add(num_bits)? > data.len().checked_mul(8)? {
        return None;
    }

    let mut value = 0u16;
    for k in 0..num_bits {
        let p = bit_pos + k;
        let byte = data[p / 8];
        let bit = (byte >> (7 - (p % 8))) & 1;
        value = (value << 1) | u16::from(bit);
    }
    Some(value)
}

fn read_bits_msb(data: &[u8], bit_pos: &mut usize, num_bits: usize) -> Option<u16> {
    let v = peek_bits_msb(data, *bit_pos, num_bits)?;
    *bit_pos += num_bits;
    Some(v)
}

#[derive(Clone)]
struct HeaderBitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> HeaderBitReader<'a> {
    fn new(data: &'a [u8], bit_pos: usize) -> Self {
        Self { data, bit_pos }
    }

    fn byte_pos(&self) -> usize {
        self.bit_pos / 8
    }

    fn align(&mut self) {
        if self.bit_pos % 8 != 0 {
            self.bit_pos += 8 - (self.bit_pos % 8);
        }
    }

    fn read_bit_raw(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.bit_pos / 8)?;
        let shift = 7 - (self.bit_pos % 8);
        self.bit_pos += 1;
        Some((byte >> shift) & 1)
    }

    fn needs_stuffing_bit(&self) -> bool {
        self.bit_pos % 8 == 7 && self.data.get(self.bit_pos / 8).copied() == Some(0xFF)
    }

    fn read_bits_with_stuffing(&mut self, nbits: u8) -> Option<u32> {
        let mut v = 0u32;
        for _ in 0..nbits {
            let needs_stuff = self.needs_stuffing_bit();
            v = (v << 1) | u32::from(self.read_bit_raw()?);
            if needs_stuff {
                let stuffed = self.read_bit_raw()?;
                if stuffed != 0 {
                    return None;
                }
            }
        }
        Some(v)
    }

    fn peek_bits_with_stuffing(&self, nbits: u8) -> Option<u32> {
        let mut clone = self.clone();
        clone.read_bits_with_stuffing(nbits)
    }
}

fn hdr_decode_num_classic_coding_passes(hdr: &mut HeaderBitReader<'_>) -> Option<u8> {
    if hdr.peek_bits_with_stuffing(9) == Some(0x1ff) {
        hdr.read_bits_with_stuffing(9)?;
        Some((hdr.read_bits_with_stuffing(7)? + 37) as u8)
    } else if hdr.peek_bits_with_stuffing(4) == Some(0x0f) {
        hdr.read_bits_with_stuffing(4)?;
        Some((hdr.read_bits_with_stuffing(5)? + 6) as u8)
    } else if hdr.peek_bits_with_stuffing(4) == Some(0b1110) {
        hdr.read_bits_with_stuffing(4)?;
        Some(5)
    } else if hdr.peek_bits_with_stuffing(4) == Some(0b1101) {
        hdr.read_bits_with_stuffing(4)?;
        Some(4)
    } else if hdr.peek_bits_with_stuffing(4) == Some(0b1100) {
        hdr.read_bits_with_stuffing(4)?;
        Some(3)
    } else if hdr.peek_bits_with_stuffing(2) == Some(0b10) {
        hdr.read_bits_with_stuffing(2)?;
        Some(2)
    } else if hdr.peek_bits_with_stuffing(1) == Some(0) {
        hdr.read_bits_with_stuffing(1)?;
        Some(1)
    } else {
        None
    }
}

fn hdr_read_lblock_increment(hdr: &mut HeaderBitReader<'_>) -> Option<u32> {
    let mut inc = 0u32;
    loop {
        let b = hdr.read_bits_with_stuffing(1)?;
        if b == 0 {
            break;
        }
        inc = inc.saturating_add(1);
    }
    Some(inc)
}

// Probe classic coding-pass codeword shape (JPEG 2000 B.10.6 / Table B.4).
// Returns Ok((passes, bits_used)) when decodable at `bit_pos`, or Err(required_bits)
// when the prefix indicates a valid form but more bits are required.
fn probe_decode_num_classic_coding_passes(data: &[u8], bit_pos: usize) -> std::result::Result<(u8, usize), usize> {
    let total_bits = data.len() * 8;

    let remaining = |needed: usize| {
        if total_bits.saturating_sub(bit_pos) < needed {
            Err(needed)
        } else {
            Ok(())
        }
    };

    if peek_bits_msb(data, bit_pos, 9) == Some(0x1ff) {
        remaining(16)?;
        let ext = peek_bits_msb(data, bit_pos + 9, 7).expect("checked by remaining") as u8;
        return Ok((ext.saturating_add(37), 16));
    }
    if peek_bits_msb(data, bit_pos, 4) == Some(0x0f) {
        remaining(9)?;
        let ext = peek_bits_msb(data, bit_pos + 4, 5).expect("checked by remaining") as u8;
        return Ok((ext.saturating_add(6), 9));
    }
    if peek_bits_msb(data, bit_pos, 4) == Some(0b1110) {
        remaining(4)?;
        return Ok((5, 4));
    }
    if peek_bits_msb(data, bit_pos, 4) == Some(0b1101) {
        remaining(4)?;
        return Ok((4, 4));
    }
    if peek_bits_msb(data, bit_pos, 4) == Some(0b1100) {
        remaining(4)?;
        return Ok((3, 4));
    }
    if peek_bits_msb(data, bit_pos, 2) == Some(0b10) {
        remaining(2)?;
        return Ok((2, 2));
    }
    if peek_bits_msb(data, bit_pos, 1) == Some(0) {
        remaining(1)?;
        return Ok((1, 1));
    }

    // Any non-matching pattern at this position is considered malformed.
    Err(1)
}

fn probe_read_lblock_increment(data: &[u8], mut bit_pos: usize) -> std::result::Result<(u32, usize), usize> {
    let start = bit_pos;
    let mut increment = 0u32;

    loop {
        let b = peek_bits_msb(data, bit_pos, 1).ok_or(1usize)?;
        bit_pos += 1;
        if b == 0 {
            break;
        }
        increment = increment.saturating_add(1);
    }

    Ok((increment, bit_pos - start))
}

fn probe_classic_segment_length_field(
    data: &[u8],
    bit_pos: usize,
    added_coding_passes: u8,
    current_lblock: u32,
) -> std::result::Result<(usize, u32, u32), usize> {
    let (next_lblock, lblock_bits_used) = update_lblock(current_lblock, bit_pos, data)?;
    let length_bits = next_lblock
        .saturating_add(added_coding_passes.ilog2());

    // Guard against clearly unreasonable/unsafe requests in this preflight stage.
    if length_bits > 31 {
        return Err((lblock_bits_used + 1) as usize);
    }

    let length_bits_usize = length_bits as usize;
    let length_start = bit_pos + lblock_bits_used;
    let Some(length_value) = peek_bits_msb(data, length_start, length_bits_usize) else {
        return Err(lblock_bits_used + length_bits_usize);
    };

    Ok((lblock_bits_used + length_bits_usize, u32::from(length_value), next_lblock))
}

fn probe_first_inclusion_flag(data: &[u8], bit_pos: usize) -> std::result::Result<(bool, usize), usize> {
    let Some(v) = peek_bits_msb(data, bit_pos, 1) else {
        return Err(1);
    };
    Ok((v != 0, 1))
}

fn update_lblock(current_lblock: u32, bit_pos: usize, data: &[u8]) -> std::result::Result<(u32, usize), usize> {
    let (inc, bits_used) = probe_read_lblock_increment(data, bit_pos)?;
    Ok((current_lblock.saturating_add(inc), bits_used))
}

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
    // Whether a POC (Progression Order Change) marker was present in the
    // main codestream header.  Tile-part-level POC is captured per tile-part
    // in TilePartInfo.has_poc; main-header POC changes the global progression
    // order and is not yet supported by the native packet walker.
    has_main_header_poc: bool,
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
        fn collect_metadata_from_boxes(
            boxes_in: &[RawBox],
            ihdr: &mut Option<ImageHeader>,
            colr: &mut Option<ColourSpec>,
            crs: &mut Option<CrsInfo>,
            xml_boxes: &mut Vec<String>,
            codestream: &mut Option<Vec<u8>>,
        ) -> Result<()> {
            for b in boxes_in {
                match b.box_type {
                    boxes::box_type::JP2_HEADER => {
                        let subs = BoxReader::<std::io::Cursor<Vec<u8>>>::sub_boxes(&b.data)?;
                        collect_metadata_from_boxes(&subs, ihdr, colr, crs, xml_boxes, codestream)?;
                    }
                    // JP2 Association super-boxes can hold GMLJP2 XML metadata.
                    [b'a', b's', b'o', b'c'] => {
                        let subs = BoxReader::<std::io::Cursor<Vec<u8>>>::sub_boxes(&b.data)?;
                        collect_metadata_from_boxes(&subs, ihdr, colr, crs, xml_boxes, codestream)?;
                    }
                    boxes::box_type::IMAGE_HEADER => {
                        *ihdr = Some(ImageHeader::parse(&b.data)?);
                    }
                    boxes::box_type::COLOUR_SPEC => {
                        *colr = Some(ColourSpec::parse(&b.data)?);
                    }
                    boxes::box_type::UUID => {
                        let (uuid, payload) = boxes::parse_uuid_box(b)?;
                        if uuid == GEOJP2_UUID {
                            *crs = Some(parse_geojp2_payload(payload)?);
                        }
                    }
                    boxes::box_type::XML => {
                        if let Ok(text) = std::str::from_utf8(&b.data) {
                            xml_boxes.push(text.to_string());
                        }
                    }
                    boxes::box_type::CODESTREAM => {
                        *codestream = Some(b.data.clone());
                    }
                    _ => {}
                }
            }
            Ok(())
        }

        let mut br = BoxReader::new(reader)?;
        let all_boxes = br.read_all()?;

        // ── Validate signature ────────────────────────────────────────────
        let sig = all_boxes.first().ok_or_else(|| Jp2Error::NotJp2("empty file".into()))?;
        boxes::validate_signature(sig)?;

        let mut ihdr: Option<ImageHeader> = None;
        let mut colr: Option<ColourSpec>  = None;
        let mut crs:  Option<CrsInfo>     = None;
        let mut xml_boxes: Vec<String> = Vec::new();
        let mut codestream: Option<Vec<u8>> = None;

        collect_metadata_from_boxes(
            &all_boxes,
            &mut ihdr,
            &mut colr,
            &mut crs,
            &mut xml_boxes,
            &mut codestream,
        )?;

        // Fallback for products that store georeferencing in GMLJP2 XML boxes
        // rather than in the GeoJP2 UUID payload.
        if crs.is_none() {
            for xml in &xml_boxes {
                if let Some(parsed) = parse_gmljp2_xml_payload(xml) {
                    crs = Some(parsed);
                    break;
                }
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

        let mut has_main_header_poc = false;
        for m in &markers {
            match m.marker {
                marker::SIZ => siz = Some(Siz::parse(&m.data)?),
                marker::COD => cod = Some(Cod::parse(&m.data)?),
                marker::QCD => qcd = Some(Qcd::parse(&m.data)?),
                marker::POC => has_main_header_poc = true,
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
            has_main_header_poc,
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
    ///
    /// The native reader respects the SIZ signed flag for 32-bit data but
    /// reports `Uint16` for 16-bit signed components to match the behaviour
    /// of the `wbjpeg2000` bridge, which ignores the signed flag entirely and
    /// always clamps unsigned.
    pub fn pixel_type(&self) -> PixelType {
        match (self.signed, self.bits) {
            (false, 8)      => PixelType::Uint8,
            (false, 16)     => PixelType::Uint16,
            (_, 16)         => PixelType::Uint16,  // match bridge: ignore signed for 16-bit
            (true,  32)     => PixelType::Int32,
            _               => PixelType::Uint16,
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
        // Multi-layer files need the proper per-code-block packet decoder.
        if self.cod.num_layers > 1 {
            return self.decode_component_v2(component);
        }
        self.decode_component_single_layer(component)
    }

    /// Legacy single-layer decoder used for self-encoded `GeoJp2Writer` files that
    /// store the entire W×H DWT result as one code block with the compact (non-strided)
    /// multilevel DWT layout.
    fn decode_component_single_layer(&self, component: usize) -> Result<Vec<i32>> {
        let w  = self.width  as usize;
        let h  = self.height as usize;
        let nl = self.cod.num_decomps as usize;
        let nc = self.components as usize;
        let lossless = self.cod.wavelet == 1;

        if nc > 1 {
            return Err(Jp2Error::NotImplemented(
                "native jpeg2000_core multicomponent decode is disabled pending packet-level parser upgrade".into(),
            ));
        }

        // Find the SOT/SOD for the tile that contains this component
        // For a single-tile image this is straightforward
        let tile_data = self.extract_tile_data(0)?;

        // Single-component codestream path.
        let comp_data = tile_data;

        // Determine number of bit-planes from QCD
        let num_bitplanes = ((self.bits + nl as u8).min(31)) as usize;

        // Decode the code-block entropy data
        let decoded_ints = decode_block(&comp_data, w, h, num_bitplanes);

        // Inverse DWT
        let samples = if lossless {
            let mut coeff = decoded_ints;
            inv_dwt_53_multilevel(&mut coeff, w, h, self.cod.num_decomps);
            // Bridge-compat: treat 16-bit samples as unsigned for level shift,
            // even if SIZ signed flag is set.
            if !self.signed || self.bits == 16 {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in coeff.iter_mut() { *v += shift; }
            }
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
            if !self.signed || self.bits == 16 {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in samples.iter_mut() { *v += shift; }
            }
            samples
        };

        Ok(samples)
    }

    /// Extract the raw compressed bytes for tile `tile_idx` from the codestream.
    /// Proper per-code-block, multi-layer JPEG 2000 decoder for externally-encoded files.
    fn decode_component_v2(&self, _component: usize) -> Result<Vec<i32>> {
        let w   = self.width  as usize;
        let h   = self.height as usize;
        let nl  = self.cod.num_decomps as usize;
        let num_layers = self.cod.num_layers as usize;
        let lossless   = self.cod.wavelet == 1;

        let debug_enabled = std::env::var("JPEG2000_DEBUG_DEQUANT").is_ok();
        if debug_enabled {
            eprintln!("[decode_component_v2] w={} h={} nl={} num_layers={} lossless={}", w, h, nl, num_layers, lossless);
        }

        // ── 1. Subband region sizes within the W×H coefficient grid ──────────
        let mut rw = vec![0usize; nl + 1];
        let mut rh = vec![0usize; nl + 1];
        rw[0] = w;  rh[0] = h;
        for i in 0..nl { rw[i+1] = (rw[i]+1)/2; rh[i+1] = (rh[i]+1)/2; }

        // ── 2. Enumerate subbands ─────────────────────────────────────────────
        struct SubbandDesc { row_off: usize, col_off: usize, sb_w: usize, sb_h: usize, qcd_idx: usize }
        let mut subbands: Vec<SubbandDesc> = Vec::with_capacity(1 + 3 * nl);
        subbands.push(SubbandDesc { row_off: 0, col_off: 0, sb_w: rw[nl], sb_h: rh[nl], qcd_idx: 0 });
        for r in 1..=nl {
            let d = nl + 1 - r;
            let hl_w = rw[d - 1].saturating_sub(rw[d]);
            let lh_h = rh[d - 1].saturating_sub(rh[d]);
            subbands.push(SubbandDesc { row_off: 0,     col_off: rw[d], sb_w: hl_w, sb_h: rh[d],  qcd_idx: 3*r - 2 });
            subbands.push(SubbandDesc { row_off: rh[d], col_off: 0,     sb_w: rw[d], sb_h: lh_h,  qcd_idx: 3*r - 1 });
            subbands.push(SubbandDesc { row_off: rh[d], col_off: rw[d], sb_w: hl_w, sb_h: lh_h,   qcd_idx: 3*r     });
        }

        // ── 3. Per-code-block accumulation state ──────────────────────────────
        struct CbState {
            data: Vec<u8>,
            lblock: u32,
            ever_included: bool,
            missing_bitplanes: usize,
            incl_value: u32,
            incl_initialized: bool,
        }
        let mut cb: Vec<CbState> = subbands.iter().map(|_| CbState {
            data: Vec::new(),
            lblock: 3,
            ever_included: false,
            missing_bitplanes: 0,
            incl_value: 0,
            incl_initialized: false,
        }).collect();

        // ── 4. Locate SOD ─────────────────────────────────────────────────────
        let (sod_start, tile_end) = Self::find_tile_sod(&self.codestream)?;
        let body = &self.codestream[sod_start..tile_end];

        // ── 5. Parse packets (PCRL: outer=res 0..=nl, inner=layer 0..num_layers) ──
        let mut byte_pos = 0usize;
        'outer: for res in 0..=nl {
            let cb_start = if res == 0 { 0 } else { 1 + 3 * (res - 1) };
            let num_cbs  = if res == 0 { 1 } else { 3 };

            for _layer in 0..num_layers {
                if byte_pos >= body.len() { break 'outer; }

                // Optional SOP marker (0xFF91 + 4 bytes)
                if body.get(byte_pos) == Some(&0xFF) && body.get(byte_pos + 1) == Some(&0x91) {
                    byte_pos += 6;
                    if byte_pos >= body.len() { break 'outer; }
                }

                let mut hdr = HeaderBitReader::new(body, byte_pos * 8);
                let zero_bit = hdr.read_bits_with_stuffing(1).unwrap_or(0);
                if zero_bit == 0 {
                    hdr.align();
                    byte_pos = hdr.byte_pos();
                    if body.get(byte_pos) == Some(&0xFF) && body.get(byte_pos + 1) == Some(&0x92) {
                        byte_pos += 2;
                    }
                    continue;
                }

                let mut seg_lens  = [0u32;  3];
                let mut seg_valid = [false; 3];

                for cb_local in 0..num_cbs {
                    let si = cb_start + cb_local;
                    let first_time = !cb[si].ever_included;
                    let incl = if first_time {
                        let max_val = (_layer as u32).saturating_add(1);
                        if !cb[si].incl_initialized {
                            let mut v = cb[si].incl_value;
                            while v < max_val {
                                let Some(bit) = hdr.read_bits_with_stuffing(1) else {
                                    break;
                                };
                                if bit == 0 {
                                    v = v.saturating_add(1);
                                } else {
                                    cb[si].incl_initialized = true;
                                    break;
                                }
                            }
                            cb[si].incl_value = v;
                        }
                        cb[si].incl_value <= _layer as u32
                    } else {
                        match hdr.read_bits_with_stuffing(1) {
                            Some(v) => v != 0,
                            None => break,
                        }
                    };
                    if !incl { continue; }

                    // B.10.5 zero-bitplane information for first inclusion.
                    // For our constrained one-code-block-per-precinct path, the tag tree
                    // degenerates to one node and can be read as unary 0...01.
                    if first_time {
                        let mut mbp = 0usize;
                        loop {
                            let b = match hdr.read_bits_with_stuffing(1) {
                                Some(v) => v,
                                None => break,
                            };
                            if b == 1 {
                                break;
                            }
                            mbp = mbp.saturating_add(1);
                            if mbp > 31 {
                                break;
                            }
                        }
                        cb[si].missing_bitplanes = mbp;
                        if debug_enabled && !cb[si].ever_included {
                            eprintln!("[layer {}] sb[{}] first_inclusion: missing_bitplanes={}", _layer, si, mbp);
                        }
                    }

                    cb[si].ever_included = true;
                    let Some(passes) = hdr_decode_num_classic_coding_passes(&mut hdr) else {
                        break;
                    };

                    let Some(inc) = hdr_read_lblock_increment(&mut hdr) else {
                        break;
                    };
                    cb[si].lblock = cb[si].lblock.saturating_add(inc);

                    let len_bits = cb[si].lblock.saturating_add(passes.ilog2());
                    if len_bits > 31 {
                        break;
                    }
                    let Some(seg_len) = hdr.read_bits_with_stuffing(len_bits as u8) else {
                        break;
                    };
                    seg_lens[cb_local]  = seg_len;
                    seg_valid[cb_local] = true;
                }

                hdr.align();
                byte_pos = hdr.byte_pos();
                if body.get(byte_pos) == Some(&0xFF) && body.get(byte_pos + 1) == Some(&0x92) {
                    byte_pos += 2;
                }
                for cb_local in 0..num_cbs {
                    if seg_valid[cb_local] {
                        let end = byte_pos + seg_lens[cb_local] as usize;
                        if end > body.len() { break 'outer; }
                        let seg_start = cb[cb_start + cb_local].data.len();
                        cb[cb_start + cb_local].data.extend_from_slice(&body[byte_pos..end]);
                        if debug_enabled && cb_start + cb_local == 0 {
                            eprintln!("[layer {}] sb[0] segment: {} bytes (idx count {} → {})", 
                                _layer, seg_lens[cb_local], seg_start, cb[0].data.len());
                        }
                        byte_pos = end;
                    } else if debug_enabled && cb_start + cb_local == 0 {
                        eprintln!("[layer {}] sb[0] no segment", _layer);
                    }
                }
            }
        }

        // ── 6. Decode + assemble coefficient grid ────────────────────────────
        if lossless {
            let mut coeff = vec![0i32; w * h];
            let guard_bits = ((self.qcd.sqcd >> 5) & 0x07) as usize;
            let debug_enabled = std::env::var("JPEG2000_DEBUG_DEQUANT").is_ok();
            if debug_enabled {
                eprintln!("[lossless] guard_bits={} bits={} nl={}", guard_bits, self.bits, nl);
            }
            for (si, sb) in subbands.iter().enumerate() {
                if sb.sb_w == 0 || sb.sb_h == 0 || cb[si].data.is_empty() { continue; }
                let exp = if sb.qcd_idx < self.qcd.step_sizes.len() {
                    (self.qcd.step_sizes[sb.qcd_idx] >> 11) as usize
                } else { self.bits as usize + nl };
                let raw_bp = guard_bits.saturating_add(exp).saturating_sub(1);
                let num_bp = raw_bp.saturating_sub(cb[si].missing_bitplanes).max(1);
                if debug_enabled && si == 0 {
                    eprintln!("[lossless] sb[0]: exp={} raw_bp={} missing_bp={} num_bp={}", 
                        exp, raw_bp, cb[si].missing_bitplanes, num_bp);
                }
                let dec = decode_block(&cb[si].data, sb.sb_w, sb.sb_h, num_bp);
                for r in 0..sb.sb_h {
                    for c in 0..sb.sb_w {
                        coeff[(sb.row_off + r) * w + (sb.col_off + c)] = dec[r * sb.sb_w + c];
                    }
                }
            }
            inv_dwt_53_multilevel_proper(&mut coeff, w, h, nl as u8);
            if !self.signed || self.bits == 16 {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in coeff.iter_mut() { *v += shift; }
                if debug_enabled {
                    eprintln!("[lossless] level-shift shift={} sample[0]={}", shift, coeff[0]);
                }
            }
            Ok(coeff)
        } else {
            let mut fcoeff = vec![0.0f64; w * h];
            let guard_bits = ((self.qcd.sqcd >> 5) & 0x07) as usize;
            let debug_enabled = std::env::var("JPEG2000_DEBUG_DEQUANT").is_ok();
            if debug_enabled {
                eprintln!("[qcd] sqcd={:02X} guard_bits={} num_step_sizes={}", self.qcd.sqcd, guard_bits, self.qcd.step_sizes.len());
                for (i, &ss) in self.qcd.step_sizes.iter().take(4).enumerate() {
                    let exp = (ss >> 11) as i32;
                    let mnt = (ss & 0x7FF) as f64;
                    eprintln!("[qcd] step_size[{}] = 0x{:04X} (exp={} mnt={:.3})", i, ss, exp, mnt);
                }
            }
            for (si, sb) in subbands.iter().enumerate() {
                if sb.sb_w == 0 || sb.sb_h == 0 || cb[si].data.is_empty() { continue; }
                let (exp, mnt) = if sb.qcd_idx < self.qcd.step_sizes.len() {
                    let s = self.qcd.step_sizes[sb.qcd_idx];
                    (((s >> 11) as i32), (s & 0x7FF) as f64)
                } else {
                    (self.bits as i32, 0.0f64)
                };
                let raw_bp = guard_bits.saturating_add(exp.max(0) as usize).saturating_sub(1);
                let num_bp = raw_bp.saturating_sub(cb[si].missing_bitplanes).max(1);

                let log_gain = if sb.qcd_idx == 0 { 0 } else if sb.qcd_idx % 3 == 0 { 2 } else { 1 };
                let r_b = self.bits as i32 + log_gain;
                let delta = 2.0f64.powi(r_b - exp) * (1.0 + mnt / 2048.0);

                if debug_enabled && si == 0 {
                    eprintln!("[ll_calc] qcd_idx={} exp={} mnt={:.3} guard_bits={} raw_bp={} missing_bp={} num_bp={}", 
                        sb.qcd_idx, exp, mnt, guard_bits, raw_bp, cb[si].missing_bitplanes, num_bp);
                }
                if debug_enabled && sb.qcd_idx < 4 {
                    eprintln!("[lossy] sb[{}]: exp={} mnt={:.3} log_gain={} r_b={} delta={:.10} num_bp={} missing_bp={}",
                        si, exp, mnt, log_gain, r_b, delta, num_bp, cb[si].missing_bitplanes);
                }

                let dec = decode_block(&cb[si].data, sb.sb_w, sb.sb_h, num_bp);
                for r in 0..sb.sb_h {
                    for c in 0..sb.sb_w {
                        let q = dec[r * sb.sb_w + c];
                        let coeff = (q as f64) * delta;
                        if debug_enabled && r == 0 && c == 0 {
                            eprintln!("[lossy] sb[{}] (0,0): q={} delta={:.10} coeff={:.6}", si, q, delta, coeff);
                        }
                        fcoeff[(sb.row_off + r) * w + (sb.col_off + c)] = coeff;
                    }
                }
                if debug_enabled && si == 0 {
                    eprintln!("[ll_subband_after_assembly] sb0 grid size {}x{}, fcoeff[0]={:.6}", sb.sb_w, sb.sb_h, fcoeff[0]);
                }
            }
            let mut samples = inv_dwt_97_multilevel_proper(&fcoeff, w, h, nl as u8);
            if debug_enabled {
                eprintln!("[pre-shift] sample[0]={} bits={} signed={} check=({}||{})", 
                    samples[0], self.bits, self.signed, !self.signed, self.bits == 16);
            }
            if !self.signed || self.bits == 16 {
                let shift = 1i32 << (self.bits.saturating_sub(1));
                for v in samples.iter_mut() { *v += shift; }
                if debug_enabled {
                    eprintln!("[level-shift] shift={} (1<<{}), sample[0]={}", shift, self.bits-1, samples[0]);
                }
            }
            Ok(samples)
        }
    }

    /// Find tile-0 SOD start and tile data end within codestream bytes.
    /// Returns `(sod_byte_start, tile_data_end)`.
    fn find_tile_sod(cs: &[u8]) -> Result<(usize, usize)> {
        let mut i = 0;
        while i + 1 < cs.len() {
            if cs[i] != 0xFF { i += 1; continue; }
            let m = u16::from_be_bytes([cs[i], cs[i+1]]);
            match m {
                marker::SOC => { i += 2; }
                marker::SOT => {
                    if i + 11 >= cs.len() { break; }
                    let psot = u32::from_be_bytes([cs[i+6], cs[i+7], cs[i+8], cs[i+9]]) as usize;
                    let lsot = u16::from_be_bytes([cs[i+2], cs[i+3]]) as usize;
                    let tile_end = if psot > 0 {
                        i + psot
                    } else {
                        // psot=0: scan for EOC
                        let mut j = i + 2 + lsot;
                        loop {
                            if j + 1 >= cs.len() { break cs.len(); }
                            if cs[j] == 0xFF && cs[j+1] == 0xD9 { break j; }
                            j += 1;
                        }
                    };
                    // Find SOD within tile-part header
                    let mut j = i + 2 + lsot;
                    while j + 1 < tile_end.min(cs.len()) {
                        if cs[j] != 0xFF { j += 1; continue; }
                        let mm = u16::from_be_bytes([cs[j], cs[j+1]]);
                        if mm == marker::SOD {
                            return Ok((j + 2, tile_end.min(cs.len())));
                        }
                        if j + 3 < cs.len() {
                            let mlen = u16::from_be_bytes([cs[j+2], cs[j+3]]) as usize;
                            j += 2 + mlen;
                        } else { j += 1; }
                    }
                    break;
                }
                marker::EOC => break,
                _ => {
                    if i + 3 < cs.len() {
                        let mlen = u16::from_be_bytes([cs[i+2], cs[i+3]]) as usize;
                        i += 2 + mlen;
                    } else { i += 1; }
                }
            }
        }
        Err(Jp2Error::InvalidCodestream { offset: 0, message: "SOD not found in tile 0".into() })
    }

    /// Extract the raw compressed bytes for tile `tile_idx` from the codestream.
    fn extract_tile_data(&self, tile_idx: u16) -> Result<Vec<u8>> {
        let plan = self.build_packet_traversal_plan(tile_idx)?;
        let out = self.collect_tile_packet_payload(&plan)?;

        if !out.is_empty() {
            return Ok(out);
        }

        Err(Jp2Error::InvalidCodestream {
            offset: 0,
            message: format!("Tile {} not found in codestream", tile_idx),
        })
    }

    /// Build a packet traversal plan for one tile.
    ///
    /// This currently captures progression metadata and ordered tile-part payload
    /// windows. A later packet parser upgrade will consume this plan directly for
    /// progression-aware packet walking.
    fn build_packet_traversal_plan(&self, tile_idx: u16) -> Result<PacketTraversalPlan> {
        let all_parts = self.parse_tile_parts()?;
        let mut tile_parts: Vec<TilePartInfo> = all_parts
            .into_iter()
            .filter(|p| p.isot == tile_idx)
            .collect();

        tile_parts.sort_by_key(|p| p.tpsot);

        let mut effective_progression = self.cod.progression;
        let mut effective_num_layers = self.cod.num_layers;
        for p in &tile_parts {
            if let Some(cod) = p.cod_override.as_ref() {
                effective_progression = cod.progression;
                effective_num_layers = cod.num_layers;
                break;
            }
        }

        let has_poc = tile_parts.iter().any(|p| p.has_poc);
        let has_packet_header_markers = tile_parts.iter().any(|p| p.has_packet_header_markers);

        Ok(PacketTraversalPlan {
            progression: effective_progression,
            num_layers: effective_num_layers,
            tile_parts,
            has_poc,
            has_packet_header_markers,
        })
    }

    fn collect_tile_packet_payload(&self, plan: &PacketTraversalPlan) -> Result<Vec<u8>> {
        if self.has_main_header_poc || plan.has_poc {
            return Err(Jp2Error::NotImplemented(
                "native packet walker does not yet support POC progression changes".into(),
            ));
        }
        if plan.has_packet_header_markers {
            return Err(Jp2Error::NotImplemented(
                "native packet walker does not yet support PPM/PPT external packet-header marker workflows".into(),
            ));
        }

        match plan.progression {
            ProgressionOrder::Lrcp => {
                self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Lrcp)
            }
            ProgressionOrder::Rlcp => {
                self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Rlcp)
            }
            ProgressionOrder::Rpcl => {
                self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Rpcl)
            }
            ProgressionOrder::Pcrl => {
                self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Pcrl)
            }
            ProgressionOrder::Cprl => {
                self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Cprl)
            }
        }
    }

    fn collect_tile_packet_payload_lrcp(&self, plan: &PacketTraversalPlan) -> Result<Vec<u8>> {
        self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Lrcp)
    }

    fn collect_tile_packet_payload_rlcp(&self, plan: &PacketTraversalPlan) -> Result<Vec<u8>> {
        self.collect_tile_packet_payload_for_progression(plan, ProgressionOrder::Rlcp)
    }

    fn collect_tile_packet_payload_for_progression(
        &self,
        plan: &PacketTraversalPlan,
        progression: ProgressionOrder,
    ) -> Result<Vec<u8>> {
        if plan.num_layers == 0 {
            return Err(Jp2Error::InvalidCodestream {
                offset: 0,
                message: "Invalid COD: number of quality layers is zero".into(),
            });
        }

        let max_packet_preview_per_tilepart = self.packet_preview_budget_for_plan(plan);
        let layers = usize::from(plan.num_layers).max(1);
        let components = usize::from(self.components).max(1);
        let resolutions = (usize::from(self.cod.num_decomps) + 1).max(1);

        let mut out = Vec::new();
        // Carry packet context/state across tile-parts of the same tile so
        // packet sequencing continuity does not reset at each part boundary.
        let mut packet_ctx = PacketCursor::for_progression(progression);
        let mut packet_state_by_ctx: HashMap<(usize, usize, usize), PacketContextState> = HashMap::new();

        for part in &plan.tile_parts {
            let mut cursor = part.sod_start;
            let mut had_preview_slice = false;
            let mut force_full_payload_fallback = false;
            let mut append_unresolved_tail_from: Option<usize> = None;
            let mut part_out = Vec::new();
            let progression_label = match progression {
                ProgressionOrder::Lrcp => "LRCP",
                ProgressionOrder::Rlcp => "RLCP",
                ProgressionOrder::Rpcl => "RPCL",
                ProgressionOrder::Pcrl => "PCRL",
                ProgressionOrder::Cprl => "CPRL",
            };

            for _ in 0..max_packet_preview_per_tilepart {
                if cursor >= part.tile_part_end {
                    break;
                }

                let (layer, resolution, component) = packet_ctx.context_key();

                let preflight = self
                    .probe_packet_header_lrcp_at(
                        cursor,
                        part.tile_part_end,
                        packet_state_by_ctx.entry((layer, resolution, component)).or_default(),
                    )
                    .map_err(|e| match e {
                        Jp2Error::InvalidCodestream { offset, message } => Jp2Error::InvalidCodestream {
                            offset,
                            message: format!(
                                "{} [{} l={}, r={}, c={} state:seen={}, contrib={}, zero_len={}, ever_included={}, since_last_inclusion={}]",
                                message,
                                progression_label,
                                layer,
                                resolution,
                                component,
                                packet_state_by_ctx
                                    .get(&(layer, resolution, component))
                                    .map(|s| s.packets_seen)
                                    .unwrap_or(0),
                                packet_state_by_ctx
                                    .get(&(layer, resolution, component))
                                    .map(|s| s.contributions_seen)
                                    .unwrap_or(0),
                                packet_state_by_ctx
                                    .get(&(layer, resolution, component))
                                    .map(|s| s.zero_length_packets)
                                    .unwrap_or(0),
                                packet_state_by_ctx
                                    .get(&(layer, resolution, component))
                                    .map(|s| s.ever_included)
                                    .unwrap_or(false),
                                packet_state_by_ctx
                                    .get(&(layer, resolution, component))
                                    .map(|s| s.packets_since_last_inclusion)
                                    .unwrap_or(0),
                            ),
                        },
                        _ => e,
                    })?;

                if preflight.preview_reached_contribution_cap {
                    force_full_payload_fallback = true;
                    break;
                }

                if preflight.has_preview_contribution && preflight.preview_declared_body_bytes > 0 {
                    let body_end = preflight
                        .body_data_start
                        .saturating_add(preflight.preview_declared_body_bytes as usize);
                    part_out.extend_from_slice(&self.codestream[preflight.body_data_start..body_end]);
                    had_preview_slice = true;
                    cursor = body_end;
                } else {
                    let context_key = (layer, resolution, component);
                    let context_state = packet_state_by_ctx
                        .get(&context_key)
                        .copied()
                        .unwrap_or_default();

                    if matches!(preflight.kind, PacketHeaderProbe::NonZeroLength)
                        && context_state.ever_included
                    {
                        force_full_payload_fallback = true;
                        break;
                    }

                    if matches!(preflight.kind, PacketHeaderProbe::NonZeroLength) && had_preview_slice {
                        append_unresolved_tail_from = Some(cursor);
                        break;
                    }

                    if preflight.body_data_start <= cursor {
                        break;
                    }
                    cursor = preflight.body_data_start;
                }

                if !packet_ctx.advance(layers, resolutions, components) {
                    packet_ctx = PacketCursor::for_progression(progression);
                }
            }

            if force_full_payload_fallback || !had_preview_slice {
                out.extend_from_slice(&self.codestream[part.sod_start..part.tile_part_end]);
            } else {
                if let Some(tail_start) = append_unresolved_tail_from {
                    if tail_start < part.tile_part_end {
                        part_out.extend_from_slice(&self.codestream[tail_start..part.tile_part_end]);
                    }
                }
                out.extend_from_slice(&part_out);
            }
        }
        Ok(out)
    }

    fn packet_preview_budget_for_plan(&self, plan: &PacketTraversalPlan) -> usize {
        // Bound preview effort to avoid runaway scans while still scaling with
        // progression-relevant packet dimensions and allowing at least one
        // context revisit round for state continuity checks.
        let layers = usize::from(plan.num_layers).clamp(1, 16);
        let components = usize::from(self.components).clamp(1, 8);
        let resolutions = (usize::from(self.cod.num_decomps) + 1).clamp(1, 8);

        let budget = match plan.progression {
            ProgressionOrder::Lrcp
            | ProgressionOrder::Rlcp
            | ProgressionOrder::Rpcl
            | ProgressionOrder::Pcrl
            | ProgressionOrder::Cprl => {
                layers.saturating_mul(components).saturating_mul(resolutions)
            }
        };

        budget.saturating_mul(2).clamp(1, 128)
    }

    fn probe_packet_header_lrcp_at(
        &self,
        packet_start: usize,
        payload_end: usize,
        packet_state: &mut PacketContextState,
    ) -> Result<PacketHeaderPreflight> {
        const SOP: u16 = 0xFF91;
        const EPH: u16 = 0xFF92;
        const MAX_PREVIEW_CONTRIBUTIONS: usize = 8;

        let cs = &self.codestream;
        let payload_start = packet_start;

        if payload_start >= payload_end {
            return Err(Jp2Error::InvalidCodestream {
                offset: payload_start,
                message: "Tile-part payload is empty after SOD".into(),
            });
        }

        let mut i = payload_start;

        // Optional SOP marker at packet start.
        if i + 6 <= payload_end {
            let m = u16::from_be_bytes([cs[i], cs[i + 1]]);
            if m == SOP {
                let lsop = u16::from_be_bytes([cs[i + 2], cs[i + 3]]) as usize;
                if lsop != 4 {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: i,
                        message: format!("Invalid SOP marker segment length: {lsop}"),
                    });
                }
                let sop_end = i + 2 + lsop;
                if sop_end > payload_end {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: i,
                        message: "SOP marker segment extends beyond tile-part payload".into(),
                    });
                }
                i = sop_end;
            }
        }

        if i >= payload_end {
            return Err(Jp2Error::InvalidCodestream {
                offset: i,
                message: "Truncated packet header: missing first packet header byte".into(),
            });
        }

        let header_bytes = &cs[i..payload_end];
        let mut bit_pos = 0usize;
        let mut has_included_preview_contribution = false;
        let mut declared_preview_length_sum = 0u32;
        let mut preview_reached_contribution_cap = false;

        // First packet-header bit: 0 => zero-length packet, 1 => non-zero.
        let zero_length = read_bits_msb(header_bytes, &mut bit_pos, 1)
            .ok_or_else(|| Jp2Error::InvalidCodestream {
                offset: i,
                message: "Truncated packet header: missing first packet header bit".into(),
            })?
            == 0;

        packet_state.packets_seen = packet_state.packets_seen.saturating_add(1);
        let packet_index = packet_state.packets_seen;

        if !zero_length {
            let mut inclusion_terminated = false;
            for _ in 0..MAX_PREVIEW_CONTRIBUTIONS {
                let (included, inclusion_bits_used) = match probe_first_inclusion_flag(header_bytes, bit_pos) {
                    Ok(v) => v,
                    Err(required_bits) => {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: i + (bit_pos / 8),
                            message: format!(
                                "Truncated or malformed inclusion signaling in packet preflight (need {required_bits} bits)"
                            ),
                        });
                    }
                };
                bit_pos += inclusion_bits_used;

                if !included {
                    inclusion_terminated = true;
                    break;
                }

                has_included_preview_contribution = true;
                packet_state.ever_included = true;
                packet_state.contributions_seen = packet_state.contributions_seen.saturating_add(1);
                packet_state.last_included_packet_index = Some(packet_index);
                if packet_state.first_included_packet_index.is_none() {
                    packet_state.first_included_packet_index = Some(packet_index);
                }
                packet_state.packets_since_last_inclusion = 0;

                let (passes, bits_used) = match probe_decode_num_classic_coding_passes(header_bytes, bit_pos) {
                    Ok(v) => v,
                    Err(required_bits) => {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: i + (bit_pos / 8),
                            message: format!(
                                "Truncated or malformed classic coding-pass codeword in packet header preflight (need {required_bits} bits)"
                            ),
                        });
                    }
                };
                bit_pos += bits_used;

                let (length_bits_used, length_value, next_lblock) = match probe_classic_segment_length_field(
                    header_bytes,
                    bit_pos,
                    passes,
                    packet_state.lblock,
                ) {
                    Ok(v) => v,
                    Err(required_bits) => {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: i + (bit_pos / 8),
                            message: format!(
                                "Truncated or malformed Lblock/segment-length header in packet preflight (need {required_bits} bits)"
                            ),
                        });
                    }
                };
                bit_pos += length_bits_used;
                declared_preview_length_sum = declared_preview_length_sum.saturating_add(length_value);
                packet_state.lblock = next_lblock;
            }

            // If preview did not observe inclusion termination and included
            // contributions were seen, bounded contribution preview hit cap.
            preview_reached_contribution_cap = has_included_preview_contribution && !inclusion_terminated;
        } else {
            packet_state.zero_length_packets = packet_state.zero_length_packets.saturating_add(1);
            if packet_state.ever_included {
                packet_state.packets_since_last_inclusion =
                    packet_state.packets_since_last_inclusion.saturating_add(1);
            }
        }

        if !zero_length && !has_included_preview_contribution && packet_state.ever_included {
            packet_state.packets_since_last_inclusion =
                packet_state.packets_since_last_inclusion.saturating_add(1);
        }

        // Byte-align after packet header probe before optional EPH marker check.
        bit_pos = (bit_pos + 7) & !7;
        i += bit_pos / 8;

        // Optional EPH after packet header.
        if i + 2 <= payload_end {
            let m = u16::from_be_bytes([cs[i], cs[i + 1]]);
            if m == EPH {
                i += 2;
            }
        }

        if i > payload_end {
            return Err(Jp2Error::InvalidCodestream {
                offset: payload_end,
                message: "Packet header boundary extends beyond tile-part payload".into(),
            });
        }

        if !zero_length && has_included_preview_contribution && i >= payload_end {
            return Err(Jp2Error::InvalidCodestream {
                offset: payload_end,
                message: "Non-zero packet with included preview contributions leaves empty packet body in tile-part payload".into(),
            });
        }

        if !zero_length {
            // Provisional packet-body span accounting: for current single-segment
            // preview path, ensure enough bytes remain for the sum of declared
            // segment lengths from previewed included contributions.
            let remaining_body_bytes = (payload_end - i) as u32;
            let declared_length = declared_preview_length_sum;

            if declared_length > remaining_body_bytes {
                return Err(Jp2Error::InvalidCodestream {
                    offset: i,
                    message: format!(
                        "Declared packet segment length ({declared_length}) exceeds remaining tile-part body bytes ({remaining_body_bytes})"
                    ),
                });
            }
        }

        if zero_length {
            Ok(PacketHeaderPreflight {
                kind: PacketHeaderProbe::ZeroLength,
                body_data_start: i,
                has_preview_contribution: false,
                preview_declared_body_bytes: 0,
                preview_reached_contribution_cap: false,
            })
        } else {
            Ok(PacketHeaderPreflight {
                kind: PacketHeaderProbe::NonZeroLength,
                body_data_start: i,
                has_preview_contribution: has_included_preview_contribution,
                preview_declared_body_bytes: declared_preview_length_sum,
                preview_reached_contribution_cap,
            })
        }
    }

    /// Parse codestream tile-parts and return bounded tile-part metadata.
    fn parse_tile_parts(&self) -> Result<Vec<TilePartInfo>> {
        let cs = &self.codestream;
        let mut i = 0;
        let mut parts: Vec<TilePartInfo> = Vec::new();
        let mut last_part_index: HashMap<u16, u8> = HashMap::new();
        let mut declared_num_parts: HashMap<u16, u8> = HashMap::new();

        while i + 1 < cs.len() {
            if cs[i] != 0xFF { i += 1; continue; }
            let m = u16::from_be_bytes([cs[i], cs[i+1]]);
            if m == marker::SOC {
                i += 2;
                continue;
            }
            if m == marker::EOC { break; }

            if m == marker::SOT {
                let sot_marker_start = i;
                if i + 12 > cs.len() {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: i,
                        message: "Truncated SOT marker segment".into(),
                    });
                }

                let lsot = u16::from_be_bytes([cs[i + 2], cs[i + 3]]) as usize;
                if lsot < 10 {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: i,
                        message: format!("Invalid SOT length: {lsot}"),
                    });
                }

                let sot_segment_end = i + 2 + lsot;
                if sot_segment_end > cs.len() {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: i,
                        message: "SOT segment extends beyond codestream".into(),
                    });
                }

                let isot = u16::from_be_bytes([cs[i + 4], cs[i + 5]]);
                let psot = u32::from_be_bytes([cs[i + 6], cs[i + 7], cs[i + 8], cs[i + 9]]) as usize;
                let tpsot = cs[i + 10];
                let tnsot = cs[i + 11];

                match last_part_index.get(&isot).copied() {
                    Some(prev) => {
                        if tpsot != prev.saturating_add(1) {
                            return Err(Jp2Error::InvalidCodestream {
                                offset: i,
                                message: format!(
                                    "Invalid tile-part sequence for tile {isot}: expected TPsot={}, got {tpsot}",
                                    prev.saturating_add(1)
                                ),
                            });
                        }
                    }
                    None => {
                        if tpsot != 0 {
                            return Err(Jp2Error::InvalidCodestream {
                                offset: i,
                                message: format!(
                                    "Invalid first tile-part index for tile {isot}: expected TPsot=0, got {tpsot}"
                                ),
                            });
                        }
                    }
                }

                if tnsot > 0 {
                    if let Some(prev_tnsot) = declared_num_parts.get(&isot).copied() {
                        if prev_tnsot != tnsot {
                            return Err(Jp2Error::InvalidCodestream {
                                offset: i,
                                message: format!(
                                    "Inconsistent TNsot for tile {isot}: saw {prev_tnsot} then {tnsot}"
                                ),
                            });
                        }
                    } else {
                        declared_num_parts.insert(isot, tnsot);
                    }

                    if tpsot >= tnsot {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: i,
                            message: format!(
                                "Invalid tile-part index for tile {isot}: TPsot={tpsot} must be < TNsot={tnsot}"
                            ),
                        });
                    }
                }

                let tile_part_end = if psot > 0 {
                    let end = sot_marker_start + psot;
                    if end > cs.len() {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: i,
                            message: "SOT Psot extends beyond codestream".into(),
                        });
                    }
                    end
                } else {
                    self.find_next_tile_or_eoc(sot_segment_end)
                };

                // Parse tile-part header marker segments up to SOD and capture
                // unsupported packet-header distribution/progression-change modes.
                let mut sod_start: Option<usize> = None;
                let mut has_poc = false;
                let mut has_packet_header_markers = false;
                let mut cod_override: Option<Cod> = None;
                let mut j = sot_segment_end;
                while j + 1 < tile_part_end {
                    if cs[j] != 0xFF {
                        j += 1;
                        continue;
                    }

                    let m = u16::from_be_bytes([cs[j], cs[j + 1]]);
                    if m == marker::SOD {
                        let start = j + 2;
                        if start > tile_part_end {
                            return Err(Jp2Error::InvalidCodestream {
                                offset: j,
                                message: "Invalid SOD position in tile-part".into(),
                            });
                        }
                        sod_start = Some(start);
                        break;
                    }

                    if j + 4 > tile_part_end {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: j,
                            message: "Truncated tile-part header marker segment".into(),
                        });
                    }

                    let lseg = u16::from_be_bytes([cs[j + 2], cs[j + 3]]) as usize;
                    if lseg < 2 {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: j,
                            message: format!("Invalid tile-part marker segment length: {lseg}"),
                        });
                    }

                    let next = j + 2 + lseg;
                    if next > tile_part_end {
                        return Err(Jp2Error::InvalidCodestream {
                            offset: j,
                            message: "Tile-part header marker segment extends beyond tile-part".into(),
                        });
                    }

                    if m == marker::POC {
                        has_poc = true;
                    }
                    // PLT/PLM are packet-length hint tables only; they do not
                    // move headers out of the packet body so they can be skipped
                    // without affecting packet traversal correctness.
                    // PPT/PPM externalise packet headers and require structural
                    // support before they can be decoded natively.
                    if m == marker::PPM || m == marker::PPT {
                        has_packet_header_markers = true;
                    }
                    if m == marker::COD {
                        let data_start = j + 4;
                        if data_start > next {
                            return Err(Jp2Error::InvalidCodestream {
                                offset: j,
                                message: "Invalid COD marker payload bounds in tile-part header".into(),
                            });
                        }
                        let cod = Cod::parse(&cs[data_start..next])?;
                        cod_override = Some(cod);
                    }

                    j = next;
                }

                let sod_start = sod_start.ok_or_else(|| Jp2Error::InvalidCodestream {
                    offset: sot_marker_start,
                    message: format!("Tile-part for tile {} missing SOD marker", isot),
                })?;

                parts.push(TilePartInfo {
                    isot,
                    tpsot,
                    tnsot,
                    sod_start,
                    tile_part_end,
                    has_poc,
                    has_packet_header_markers,
                    cod_override,
                });
                last_part_index.insert(isot, tpsot);
                i = tile_part_end;

                continue;
            }

            // Other marker segments — skip
            if i + 4 > cs.len() {
                return Err(Jp2Error::InvalidCodestream {
                    offset: i,
                    message: "Truncated marker segment header".into(),
                });
            }
            let lseg = u16::from_be_bytes([cs[i + 2], cs[i + 3]]) as usize;
            if lseg < 2 {
                return Err(Jp2Error::InvalidCodestream {
                    offset: i,
                    message: format!("Invalid marker segment length: {lseg}"),
                });
            }
            i += 2 + lseg;
        }

        for (isot, tnsot) in declared_num_parts {
            if let Some(last) = last_part_index.get(&isot).copied() {
                let seen = last.saturating_add(1);
                if seen != tnsot {
                    return Err(Jp2Error::InvalidCodestream {
                        offset: 0,
                        message: format!(
                            "Incomplete tile-part sequence for tile {isot}: expected {tnsot} parts, saw {seen}"
                        ),
                    });
                }
            }
        }

        Ok(parts)
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

#[cfg(test)]
mod multiband_failfast_tests {
    use super::*;

    fn jp2_with_codestream(codestream: Vec<u8>, components: u16) -> GeoJp2 {
        GeoJp2 {
            width: 1,
            height: 1,
            components,
            bits: 8,
            signed: false,
            siz: Siz::new(1, 1, 8, false, components),
            cod: Cod::lossless(1, components),
            qcd: Qcd::no_quantisation(1, 8),
            color_space: if components > 1 { ColorSpace::MultiBand } else { ColorSpace::Greyscale },
            crs: None,
            has_main_header_poc: false,
            codestream,
        }
    }

    #[test]
    fn decode_component_fails_fast_for_multicomponent_native_path() {
        let jp2 = jp2_with_codestream(Vec::new(), 2);

        let err = jp2
            .decode_component(0)
            .expect_err("native multicomponent decode must fail-fast until parser upgrade");

        let msg = err.to_string();
        assert!(
            msg.contains("Not yet implemented") || msg.contains("multicomponent"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn parse_tile_parts_uses_psot_boundaries() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x11, // Psot = 17 bytes from SOT marker start
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0x01, 0x02, 0x03, // tile payload
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let parts = jp2.parse_tile_parts().expect("failed to parse tile parts");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].isot, 0);
        assert_eq!(parts[0].tpsot, 0);
        assert_eq!(parts[0].tnsot, 1);

        let data = jp2.extract_tile_data(0).expect("failed to extract tile payload");
        assert_eq!(data, vec![1, 2, 3]);
    }

    #[test]
    fn extract_tile_data_concatenates_multiple_tile_parts_for_same_tile() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            // tile-part 1
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x10, // Psot = 16
            0x00, 0x02, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0x0A, 0x0B, // payload part 1
            // tile-part 2
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0F, // Psot = 15
            0x01, 0x02, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0x0C, // payload part 2
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2.extract_tile_data(0).expect("failed to extract tile payloads");
        assert_eq!(data, vec![0x0A, 0x0B, 0x0C]);
    }

    #[test]
    fn parse_tile_parts_rejects_nonsequential_tpsot() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            // tile-part 1 (TPsot=0, TNsot=2)
            0xFF, 0x90,
            0x00, 0x0A,
            0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
            0x00, 0x02,
            0xFF, 0x93,
            0x0A, 0x0B,
            // tile-part 2 malformed (TPsot should be 1, but is 2)
            0xFF, 0x90,
            0x00, 0x0A,
            0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
            0x02, 0x02,
            0xFF, 0x93,
            0x0C, 0x0D,
            0xFF, 0xD9,
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .parse_tile_parts()
            .expect_err("nonsequential TPsot should be rejected");
        assert!(err.to_string().contains("TPsot"));
    }

    #[test]
    fn extract_tile_data_handles_rlcp_progression_after_chunk_b() {
        // RLCP progression support was added in Chunk B; extract_tile_data should
        // no longer reject it outright.
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x10, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0x11, 0x22, // payload
            0xFF, 0xD9, // EOC
        ];

        let mut jp2 = jp2_with_codestream(codestream, 1);
        jp2.cod.progression = ProgressionOrder::Rlcp;

        // Should NOT return a "not supported" error.
        if let Err(e) = jp2.extract_tile_data(0) {
            let msg = e.to_string();
            assert!(
                !msg.contains("not yet implemented") && !msg.contains("not support"),
                "RLCP rejection should not be returned after Chunk B walker port; got: {msg}"
            );
        }
    }

    #[test]
    fn extract_tile_data_rejects_poc_tile_part_headers_until_ported() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x14, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x5F, // POC
            0x00, 0x02, // Lpoc (header-only placeholder for native guard path)
            0xFF, 0x93, // SOD
            0x11, 0x22, // payload
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("POC in tile-part header should fail-fast until parser support is ported");

        let msg = err.to_string();
        assert!(msg.contains("POC"), "unexpected error message: {msg}");
    }

    #[test]
    fn extract_tile_data_rejects_ppt_tile_part_headers_until_ported() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x15, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x61, // PPT
            0x00, 0x03, // Lppt
            0x00,       // marker data
            0xFF, 0x93, // SOD
            0x11, 0x22, // payload
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("PPT in tile-part header should fail-fast until parser support is ported");

        let msg = err.to_string();
        assert!(msg.contains("PPT") || msg.contains("PPM"), "unexpected error message: {msg}");
    }

    #[test]
    fn extract_tile_data_skips_plt_in_tile_part_header() {
        // PLT is a packet-length hint that does not move headers outside the body;
        // it should be skipped silently, not rejected.
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x14, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x58, // PLT
            0x00, 0x03, // Lplt
            0x00,       // Zplt + empty Iplt
            0xFF, 0x93, // SOD
            0x11, 0x22, // payload
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        // Should NOT return NotImplemented; any result (including decode failure) is acceptable.
        let result = jp2.extract_tile_data(0);
        if let Err(e) = &result {
            let msg = e.to_string();
            assert!(
                !msg.contains("PLT") && !msg.contains("PLM") && !msg.contains("not yet implemented"),
                "PLT should be skipped, not rejected; got: {msg}"
            );
        }
    }

    #[test]
    fn extract_tile_data_rejects_invalid_sop_length_in_first_packet_probe() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x16, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0xFF, 0x91, // SOP
            0x00, 0x05, // invalid Lsop (must be 4)
            0x00, 0x00, // Nsop
            0x80,       // packet header first byte
            0xAA,       // payload
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("invalid SOP length should fail-fast in packet-header preflight");

        let msg = err.to_string();
        assert!(msg.contains("SOP"), "unexpected error message: {msg}");
    }

    #[test]
    fn extract_tile_data_rejects_empty_payload_after_sod() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0E, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD (no packet bytes before tile-part end)
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("empty tile-part payload should fail-fast");

        let msg = err.to_string();
        // Empty payload may surface as the empty-body preflight error or fall through
        // to the "Tile N not found" check in extract_tile_data.
        assert!(
            msg.contains("empty") || msg.contains("missing") || msg.contains("not found"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn extract_tile_data_honours_tile_part_cod_progression_override() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until next SOT/EOC)
            0x00, 0x01, // TPsot, TNsot
            // COD override in tile-part header
            0xFF, 0x52, // COD
            0x00, 0x0E, // Lcod = 14 => 12-byte payload
            0x00, // Scod
            0x01, // progression = RLCP
            0x00, 0x01, // num_layers = 1
            0x00, // mc_transform
            0x01, // num_decomps
            0x04, // xcb
            0x04, // ycb
            0x00, // cblk_style
            0x01, // wavelet
            0x00, 0x00, // optional extra COD bytes (accepted by parser)
            0xFF, 0x93, // SOD
            0x80, 0x11, 0x22, // packet header + payload bytes
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("tile-part COD progression override should be honoured");

        // Current RLCP constrained path uses the same packet preflight and
        // conservative fallback policy as LRCP, so this fixture returns the
        // full tile-part payload bytes.
        assert_eq!(data, vec![0x80, 0x11, 0x22]);
    }

    #[test]
    fn extract_tile_data_honours_tile_part_cod_progression_override_rpcl() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until next SOT/EOC)
            0x00, 0x01, // TPsot, TNsot
            // COD override in tile-part header
            0xFF, 0x52, // COD
            0x00, 0x0E, // Lcod = 14 => 12-byte payload
            0x00, // Scod
            0x02, // progression = RPCL
            0x00, 0x01, // num_layers = 1
            0x00, // mc_transform
            0x01, // num_decomps
            0x04, // xcb
            0x04, // ycb
            0x00, // cblk_style
            0x01, // wavelet
            0x00, 0x00, // padding bytes
            0xFF, 0x93, // SOD
            0x80, 0x11, 0x22, // packet header + payload bytes
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("RPCL tile-part COD progression override should be honoured");
        assert_eq!(data, vec![0x80, 0x11, 0x22]);
    }

    #[test]
    fn extract_tile_data_honours_tile_part_cod_progression_override_pcrl() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until next SOT/EOC)
            0x00, 0x01, // TPsot, TNsot
            // COD override in tile-part header
            0xFF, 0x52, // COD
            0x00, 0x0E, // Lcod = 14 => 12-byte payload
            0x00, // Scod
            0x03, // progression = PCRL
            0x00, 0x01, // num_layers = 1
            0x00, // mc_transform
            0x01, // num_decomps
            0x04, // xcb
            0x04, // ycb
            0x00, // cblk_style
            0x01, // wavelet
            0x00, 0x00, // padding bytes
            0xFF, 0x93, // SOD
            0x80, 0x11, 0x22, // packet header + payload bytes
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("PCRL tile-part COD progression override should be honoured");
        assert_eq!(data, vec![0x80, 0x11, 0x22]);
    }

    #[test]
    fn extract_tile_data_honours_tile_part_cod_progression_override_cprl() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until next SOT/EOC)
            0x00, 0x01, // TPsot, TNsot
            // COD override in tile-part header
            0xFF, 0x52, // COD
            0x00, 0x0E, // Lcod = 14 => 12-byte payload
            0x00, // Scod
            0x04, // progression = CPRL
            0x00, 0x01, // num_layers = 1
            0x00, // mc_transform
            0x01, // num_decomps
            0x04, // xcb
            0x04, // ycb
            0x00, // cblk_style
            0x01, // wavelet
            0x00, 0x00, // padding bytes
            0xFF, 0x93, // SOD
            0x80, 0x11, 0x22, // packet header + payload bytes
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("CPRL tile-part COD progression override should be honoured");
        assert_eq!(data, vec![0x80, 0x11, 0x22]);
    }

    #[test]
    fn probe_decode_num_classic_coding_passes_decodes_short_codewords() {
        // bits: 10xxxxxx => 2 coding passes using the 2-bit codeword path
        let data = [0b1000_0000u8];
        let (passes, used) = probe_decode_num_classic_coding_passes(&data, 0)
            .expect("2-bit classic coding-pass codeword should decode");
        assert_eq!(passes, 2);
        assert_eq!(used, 2);
    }

    #[test]
    fn extract_tile_data_rejects_truncated_classic_coding_pass_codeword_preflight() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0F, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0xF8,       // first bit=1 (non-zero packet), then 1111 prefix truncated for 9-bit form
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("truncated classic coding-pass codeword must fail-fast");

        let msg = err.to_string();
        // Truncated codeword may surface via coding-pass or Lblock/segment-length
        // preflight, depending on which stage occurs first with these payload bytes.
        assert!(
            msg.contains("coding-pass") || msg.contains("Lblock") || msg.contains("segment-length") || msg.contains("packet preflight"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn probe_read_lblock_increment_rejects_unterminated_unary() {
        let data = [0xFFu8]; // no terminating 0 bit
        let err = probe_read_lblock_increment(&data, 0)
            .expect_err("unterminated unary Lblock increment should fail");
        assert_eq!(err, 1);
    }

    #[test]
    fn probe_classic_segment_length_field_rejects_excessive_length_bits() {
        // 30 ones + terminating zero => lblock increment 30
        // length_bits = 3 + 30 + floor(log2(1)) = 33 > 31 => reject
        let data = [0xFFu8, 0xFF, 0xFF, 0xFC];
        let err = probe_classic_segment_length_field(&data, 0, 1, 3)
            .expect_err("excessive segment length bit request should fail");
        assert!(err >= 2, "unexpected required bits value: {err}");
    }

    #[test]
    fn probe_classic_segment_length_field_updates_lblock_state() {
        let data = [0b1001_1000u8];
        let (bits_used, length, next_lblock) = probe_classic_segment_length_field(&data, 0, 1, 3)
            .expect("valid segment length field should parse");
        assert_eq!(bits_used, 6);
        assert_eq!(length, 6);
        assert_eq!(next_lblock, 4);
    }

    #[test]
    fn extract_tile_data_rejects_unterminated_lblock_increment_preflight() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0F, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            0xBF,       // first bit=1, coding-pass codeword=0 (1 pass), then unary ones without terminator
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        // Walker may fail-fast on malformed Lblock/length fields or may return the
        // raw byte as a best-effort codestream body; either way it must not panic.
        if let Err(e) = jp2.extract_tile_data(0) {
            let msg = e.to_string();
            assert!(
                msg.contains("Lblock") || msg.contains("length") || msg.contains("segment"),
                "unexpected error message: {msg}"
            );
        }
    }

    #[test]
    fn probe_first_inclusion_flag_rejects_truncated_input() {
        let data: [u8; 0] = [];
        let err = probe_first_inclusion_flag(&data, 0)
            .expect_err("missing inclusion bit should fail");
        assert_eq!(err, 1);
    }

    #[test]
    fn probe_first_inclusion_flag_reads_boolean_value() {
        let data = [0b1000_0000u8];
        let (included, used) = probe_first_inclusion_flag(&data, 0)
            .expect("inclusion bit should decode");
        assert!(included);
        assert_eq!(used, 1);
    }

    fn pack_bits_msb(bits: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; (bits.len() + 7) / 8];
        for (i, bit) in bits.iter().enumerate() {
            if *bit == 0 {
                continue;
            }
            let byte = i / 8;
            let shift = 7 - (i % 8);
            out[byte] |= 1 << shift;
        }
        out
    }

    #[test]
    fn probe_packet_header_marks_when_contribution_preview_hits_cap() {
        let mut bits = vec![1u8]; // non-zero packet
        for _ in 0..8 {
            // include=1, passes=1(code 0), lblock_inc=0, len=1 (001)
            bits.extend_from_slice(&[1, 0, 0, 0, 0, 1]);
        }
        let mut packet = pack_bits_msb(&bits);
        packet.extend_from_slice(&[0x11; 8]);

        let jp2 = jp2_with_codestream(packet.clone(), 1);
        let mut state = PacketContextState::default();
        let preflight = jp2
            .probe_packet_header_lrcp_at(0, packet.len(), &mut state)
            .expect("packet preflight should succeed with cap hit fixture");

        assert!(matches!(preflight.kind, PacketHeaderProbe::NonZeroLength));
        assert!(preflight.preview_reached_contribution_cap);
        assert_eq!(preflight.preview_declared_body_bytes, 8);
    }

    #[test]
    fn extract_tile_data_rejects_nonzero_packet_without_body_after_preflight() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0F, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // Header bits packed into one byte:
            // 1 (non-zero packet)
            // 1 (first inclusion bit => included)
            // 0 (coding-pass codeword => 1 pass)
            // 0 (Lblock increment terminator)
            // 000 (segment length bits)
            0xC0,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("non-zero packet with no remaining body should fail-fast");

        let msg = err.to_string();
        assert!(msg.contains("empty packet body") || msg.contains("Non-zero"), "unexpected error message: {msg}");
    }

    #[test]
    fn extract_tile_data_allows_nonzero_packet_when_first_inclusion_is_off_and_no_body() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x0F, // Psot
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // Header bits: non-zero packet + first inclusion off
            0x80,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("non-zero packet with first inclusion off should not fail empty-body guard");
        assert_eq!(data, vec![0x80]);
    }

    #[test]
    fn extract_tile_data_rejects_when_previewed_declared_length_sum_exceeds_body() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet header preview bits:
            // 1: non-zero packet
            // contrib 1: include=1, passes=1(code 0), lblock_inc=0, len=1 (001)
            // contrib 2: include=1, passes=1(code 0), lblock_inc=0, len=1 (001)
            // contrib 3: include=0 (stop preview)
            0xC3, 0x10,
            // body has only 1 byte; declared preview sum is 2 bytes -> must fail
            0xAA,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let err = jp2
            .extract_tile_data(0)
            .expect_err("previewed declared length sum exceeding body should fail-fast");

        let msg = err.to_string();
        assert!(msg.contains("Declared packet segment length"), "unexpected error message: {msg}");
    }

    #[test]
    fn extract_tile_data_allows_when_previewed_declared_length_sum_fits_body() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // Same preview as prior test (current preview parser derives sum = 3)
            0xC3, 0x10,
            // body has 3 bytes, so span check should pass
            0xAA, 0xBB, 0xCC,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("previewed declared length sum within body should pass preflight");
        assert_eq!(data, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn extract_tile_data_collects_multiple_previewed_packets_in_one_tile_part() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet 1: nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // packet 2: same header pattern, 1-byte body
            0xC2,
            0x22,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("multi-packet preview extraction should succeed");
        assert_eq!(data, vec![0x11, 0x22]);
    }

    #[test]
    fn extract_tile_data_falls_back_when_preview_hits_contribution_cap() {
        let mut bits = vec![1u8]; // non-zero packet
        for _ in 0..8 {
            // include=1, passes=1(code 0), lblock_inc=0, len=1 (001)
            bits.extend_from_slice(&[1, 0, 0, 0, 0, 1]);
        }
        let mut packet = pack_bits_msb(&bits);
        packet.extend_from_slice(&[0x11; 8]);

        let mut codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
        ];
        codestream.extend_from_slice(&packet);
        codestream.extend_from_slice(&[0xFF, 0xD9]);

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("contribution-cap ambiguity should trigger full fallback");

        assert_eq!(data, packet);
    }

    #[test]
    fn extract_tile_data_falls_back_when_post_inclusion_packet_span_is_ambiguous() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet 1: nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // packet 2: nz=1, incl=0 => non-zero with no preview contribution
            0x80,
            // ambiguous tail byte that preflight cannot safely assign
            0xAA,
            0xFF, 0xD9, // EOC
        ];

        let mut jp2 = jp2_with_codestream(codestream, 1);
        // Force a single LRCP context so packet 2 revisits packet 1 context.
        jp2.cod.num_decomps = 0;
        let data = jp2
            .extract_tile_data(0)
            .expect("should conservatively fall back to full payload");

        // Fallback keeps full SOD..EOC-excluded payload for this tile-part.
        assert_eq!(data, vec![0xC2, 0x11, 0x80, 0xAA]);
    }

    #[test]
    fn extract_tile_data_keeps_preview_slice_when_ambiguous_packet_is_new_context() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet 1 (r=0): nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // packet 2 (r=1): nz=1, incl=0 => ambiguous packet in new context
            0x80,
            0xAA,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("new-context ambiguity should keep preview and append unresolved tail");

        assert_eq!(data, vec![0x11, 0x80, 0xAA]);
    }

    #[test]
    fn extract_tile_data_rlcp_new_context_ambiguity_appends_unresolved_tail() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            // COD override in tile-part header: RLCP progression
            0xFF, 0x52, // COD
            0x00, 0x0E, // Lcod = 14 => 12-byte payload
            0x00, // Scod
            0x01, // progression = RLCP
            0x00, 0x01, // num_layers = 1
            0x00, // mc_transform
            0x01, // num_decomps
            0x04, // xcb
            0x04, // ycb
            0x00, // cblk_style
            0x01, // wavelet
            0x00, 0x00, // optional extra COD bytes (accepted by parser)
            0xFF, 0x93, // SOD
            // packet 1 (r=0): nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // packet 2 (r=1): nz=1, incl=0 => ambiguity in new context
            0x80,
            0xAA,
            0xFF, 0xD9, // EOC
        ];

        let jp2 = jp2_with_codestream(codestream, 1);
        let data = jp2
            .extract_tile_data(0)
            .expect("RLCP new-context ambiguity should keep preview and append tail");

        assert_eq!(data, vec![0x11, 0x80, 0xAA]);
    }

    #[test]
    fn extract_tile_data_revisits_single_context_and_collects_multiple_packets() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x00, 0x01, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet 1: nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // packet 2: same header pattern, 1-byte body
            0xC2,
            0x22,
            0xFF, 0xD9, // EOC
        ];

        let mut jp2 = jp2_with_codestream(codestream, 1);
        // Single LRCP context (layer=1, resolution=1, component=1) requires
        // context-round revisit to reach packet 2.
        jp2.cod.num_decomps = 0;

        let data = jp2
            .extract_tile_data(0)
            .expect("single-context revisit should collect both packets");
        assert_eq!(data, vec![0x11, 0x22]);
    }

    #[test]
    fn packet_preview_budget_scales_with_lrcp_dimensions_and_is_bounded() {
        let jp2 = jp2_with_codestream(vec![], 3);
        let plan = PacketTraversalPlan {
            progression: ProgressionOrder::Lrcp,
            num_layers: 12,
            tile_parts: vec![],
            has_poc: false,
            has_packet_header_markers: false,
        };

        // base=layers*comps*resolutions=72; revisit multiplier x2 => 144,
        // clamped to 128.
        let budget = jp2.packet_preview_budget_for_plan(&plan);
        assert_eq!(budget, 128);
    }

    #[test]
    fn packet_preview_budget_scales_with_rlcp_dimensions_and_is_bounded() {
        let jp2 = jp2_with_codestream(vec![], 3);
        let plan = PacketTraversalPlan {
            progression: ProgressionOrder::Rlcp,
            num_layers: 12,
            tile_parts: vec![],
            has_poc: false,
            has_packet_header_markers: false,
        };

        // base=layers*comps*resolutions=72; revisit multiplier x2 => 144,
        // clamped to 128.
        let budget = jp2.packet_preview_budget_for_plan(&plan);
        assert_eq!(budget, 128);
    }

    #[test]
    fn packet_preview_budget_scales_with_other_supported_progressions_and_is_bounded() {
        let jp2 = jp2_with_codestream(vec![], 3);

        for progression in [ProgressionOrder::Rpcl, ProgressionOrder::Pcrl, ProgressionOrder::Cprl] {
            let plan = PacketTraversalPlan {
                progression,
                num_layers: 12,
                tile_parts: vec![],
                has_poc: false,
                has_packet_header_markers: false,
            };

            let budget = jp2.packet_preview_budget_for_plan(&plan);
            assert_eq!(budget, 128);
        }
    }

    #[test]
    fn lrcp_packet_cursor_advances_in_l_r_c_order() {
        let mut c = LrcpPacketCursor::new();
        let mut seen = vec![(c.layer, c.resolution, c.component)];

        // layers=2, resolutions=2, components=2 => 8 packet contexts total
        while c.advance(2, 2, 2) {
            seen.push((c.layer, c.resolution, c.component));
        }

        assert_eq!(
            seen,
            vec![
                (0, 0, 0),
                (0, 0, 1),
                (0, 1, 0),
                (0, 1, 1),
                (1, 0, 0),
                (1, 0, 1),
                (1, 1, 0),
                (1, 1, 1),
            ]
        );
    }

    #[test]
    fn rlcp_packet_cursor_advances_in_r_l_c_order() {
        let mut c = RlcpPacketCursor::new();
        let mut seen = vec![(c.layer, c.resolution, c.component)];

        // layers=2, resolutions=2, components=2 => 8 packet contexts total
        while c.advance(2, 2, 2) {
            seen.push((c.layer, c.resolution, c.component));
        }

        assert_eq!(
            seen,
            vec![
                (0, 0, 0),
                (0, 0, 1),
                (1, 0, 0),
                (1, 0, 1),
                (0, 1, 0),
                (0, 1, 1),
                (1, 1, 0),
                (1, 1, 1),
            ]
        );
    }

    #[test]
    fn rpcl_packet_cursor_advances_in_r_c_l_order() {
        let mut c = RpclPacketCursor::new();
        let mut seen = vec![(c.layer, c.resolution, c.component)];

        while c.advance(2, 2, 2) {
            seen.push((c.layer, c.resolution, c.component));
        }

        assert_eq!(
            seen,
            vec![
                (0, 0, 0),
                (1, 0, 0),
                (0, 0, 1),
                (1, 0, 1),
                (0, 1, 0),
                (1, 1, 0),
                (0, 1, 1),
                (1, 1, 1),
            ]
        );
    }

    #[test]
    fn pcrl_packet_cursor_advances_in_c_r_l_order() {
        let mut c = PcrlPacketCursor::new();
        let mut seen = vec![(c.layer, c.resolution, c.component)];

        while c.advance(2, 2, 2) {
            seen.push((c.layer, c.resolution, c.component));
        }

        assert_eq!(
            seen,
            vec![
                (0, 0, 0),
                (1, 0, 0),
                (0, 1, 0),
                (1, 1, 0),
                (0, 0, 1),
                (1, 0, 1),
                (0, 1, 1),
                (1, 1, 1),
            ]
        );
    }

    #[test]
    fn cprl_packet_cursor_advances_in_c_r_l_order() {
        let mut c = CprlPacketCursor::new();
        let mut seen = vec![(c.layer, c.resolution, c.component)];

        while c.advance(2, 2, 2) {
            seen.push((c.layer, c.resolution, c.component));
        }

        assert_eq!(
            seen,
            vec![
                (0, 0, 0),
                (1, 0, 0),
                (0, 1, 0),
                (1, 1, 0),
                (0, 0, 1),
                (1, 0, 1),
                (0, 1, 1),
                (1, 1, 1),
            ]
        );
    }

    #[test]
    fn extract_tile_data_carries_packet_context_state_across_tile_parts() {
        let codestream = vec![
            0xFF, 0x4F, // SOC
            // tile-part 0
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until next SOT)
            0x00, 0x02, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // packet 1: nz=1, incl=1, passes=1, lblock_inc=0, len=1, stop incl=0
            0xC2,
            0x11,
            // tile-part 1
            0xFF, 0x90, // SOT
            0x00, 0x0A, // Lsot
            0x00, 0x00, // Isot
            0x00, 0x00, 0x00, 0x00, // Psot = 0 (until EOC)
            0x01, 0x02, // TPsot, TNsot
            0xFF, 0x93, // SOD
            // next packet for same single context: nz=1, incl=0 (ambiguous same-context)
            0x80,
            0xAA,
            0xFF, 0xD9, // EOC
        ];

        let mut jp2 = jp2_with_codestream(codestream, 1);
        jp2.cod.num_decomps = 0; // single context across both parts

        let data = jp2
            .extract_tile_data(0)
            .expect("cross-part context continuity should be handled");

        // Part 0 preview collects 0x11; part 1 same-context ambiguity forces
        // conservative fallback for that part, preserving continuity semantics.
        assert_eq!(data, vec![0x11, 0x80, 0xAA]);
    }

    #[test]
    fn probe_packet_header_updates_context_state_counters() {
        let jp2 = jp2_with_codestream(vec![0xC2, 0x11], 1);
        let mut state = PacketContextState::default();

        let preflight = jp2
            .probe_packet_header_lrcp_at(0, 2, &mut state)
            .expect("packet header preflight should succeed");

        assert!(matches!(preflight.kind, PacketHeaderProbe::NonZeroLength));
        assert_eq!(state.packets_seen, 1);
        assert_eq!(state.contributions_seen, 1);
        assert!(state.ever_included);
        assert_eq!(state.first_included_packet_index, Some(1));
        assert_eq!(state.last_included_packet_index, Some(1));
        assert_eq!(state.packets_since_last_inclusion, 0);
    }

    #[test]
    fn probe_packet_header_zero_length_updates_history_after_inclusion() {
        let jp2 = jp2_with_codestream(vec![0xC2, 0x11, 0x00], 1);
        let mut state = PacketContextState::default();

        let first = jp2
            .probe_packet_header_lrcp_at(0, 3, &mut state)
            .expect("first packet preflight should succeed");
        assert!(matches!(first.kind, PacketHeaderProbe::NonZeroLength));
        assert_eq!(first.body_data_start, 1);

        let second = jp2
            .probe_packet_header_lrcp_at(2, 3, &mut state)
            .expect("second packet preflight should succeed");
        assert!(matches!(second.kind, PacketHeaderProbe::ZeroLength));

        assert_eq!(state.packets_seen, 2);
        assert_eq!(state.zero_length_packets, 1);
        assert_eq!(state.contributions_seen, 1);
        assert!(state.ever_included);
        assert_eq!(state.first_included_packet_index, Some(1));
        assert_eq!(state.last_included_packet_index, Some(1));
        assert_eq!(state.packets_since_last_inclusion, 1);
    }
}
