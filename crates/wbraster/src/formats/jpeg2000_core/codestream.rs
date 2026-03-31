//! JPEG 2000 codestream markers and marker segment parsing / writing.
//!
//! The JPEG 2000 codestream begins with `SOC` (0xFF90) and ends with `EOC` (0xFFD9).
//! Between them are header marker segments and tile-part data.
//!
//! Marker layout:
//! ```text
//! SOC                     – Start of codestream
//! SIZ                     – Image and tile size
//! COD                     – Coding style (DWT levels, progression, etc.)
//! [COC]                   – Per-component coding style override (optional)
//! QCD                     – Quantisation default
//! [QCC]                   – Per-component quantisation override (optional)
//! [POC]                   – Progression order change (optional)
//! [TLM]                   – Tile-part lengths (optional)
//! [PLT]                   – Packet length list (optional)
//! [PPM/PPT]               – Packed packet headers (optional)
//! [COM]                   – Comment
//! SOT                     – Start of tile-part
//!   SOD                   – Start of data (tile-part body follows)
//! … (more tile-parts)
//! EOC                     – End of codestream
//! ```

use std::io::Write;
use super::error::{Jp2Error, Result};

// ── Marker constants ──────────────────────────────────────────────────────────

pub mod marker {
    pub const SOC: u16 = 0xFF4F; // Start of codestream
    pub const SOT: u16 = 0xFF90; // Start of tile-part
    pub const SOD: u16 = 0xFF93; // Start of data
    pub const EOC: u16 = 0xFFD9; // End of codestream
    pub const SIZ: u16 = 0xFF51; // Image and tile size
    pub const COD: u16 = 0xFF52; // Coding style default
    pub const COC: u16 = 0xFF53; // Coding style component
    pub const RGN: u16 = 0xFF5E; // Region of interest
    pub const QCD: u16 = 0xFF5C; // Quantisation default
    pub const QCC: u16 = 0xFF5D; // Quantisation component
    pub const POC: u16 = 0xFF5F; // Progression order change
    pub const TLM: u16 = 0xFF55; // Tile-part lengths
    pub const PLM: u16 = 0xFF57; // Packet length main
    pub const PLT: u16 = 0xFF58; // Packet length tile-part
    pub const PPM: u16 = 0xFF60; // Packed packet main
    pub const PPT: u16 = 0xFF61; // Packed packet tile
    pub const CME: u16 = 0xFF64; // Comment
    pub const CRG: u16 = 0xFF63; // Component registration
}

// ── SIZ: Image and tile size ─────────────────────────────────────────────────

/// SIZ marker segment — image dimensions, component info.
#[derive(Debug, Clone)]
pub struct Siz {
    /// Capabilities (Rsiz): 0=baseline, 1=profile 0, 2=profile 1.
    pub rsiz: u16,
    /// Reference grid width.
    pub xsiz: u32,
    /// Reference grid height.
    pub ysiz: u32,
    /// Image area origin X.
    pub x_osiz: u32,
    /// Image area origin Y.
    pub y_osiz: u32,
    /// Tile width.
    pub x_tsiz: u32,
    /// Tile height.
    pub y_tsiz: u32,
    /// Tile origin X.
    pub xt_osiz: u32,
    /// Tile origin Y.
    pub yt_osiz: u32,
    /// Per-component: (Ssiz, XRsiz, YRsiz)
    /// Ssiz: (sign << 7) | (bit_depth - 1)
    pub components: Vec<SizComponent>,
}

#[derive(Debug, Clone, Copy)]
pub struct SizComponent {
    pub ssiz:   u8, // (signed << 7) | (bits - 1)
    pub xrsiz:  u8, // horizontal separation
    pub yrsiz:  u8, // vertical separation
}

impl SizComponent {
    pub fn new(bits: u8, signed: bool) -> Self {
        let ssiz = if signed { 0x80 | (bits - 1) } else { bits - 1 };
        Self { ssiz, xrsiz: 1, yrsiz: 1 }
    }
    pub fn bits(&self) -> u8   { (self.ssiz & 0x7F) + 1 }
    pub fn signed(&self) -> bool { self.ssiz & 0x80 != 0 }
}

impl Siz {
    pub fn new(width: u32, height: u32, bits: u8, signed: bool, num_components: u16) -> Self {
        Self {
            rsiz: 0,
            xsiz: width, ysiz: height,
            x_osiz: 0, y_osiz: 0,
            x_tsiz: width, y_tsiz: height,
            xt_osiz: 0, yt_osiz: 0,
            components: (0..num_components)
                .map(|_| SizComponent::new(bits, signed))
                .collect(),
        }
    }

    /// With explicit tile size.
    pub fn with_tiles(mut self, tw: u32, th: u32) -> Self {
        self.x_tsiz = tw; self.y_tsiz = th; self
    }

    /// Number of tiles in X direction.
    pub fn tiles_x(&self) -> u32 {
        (self.xsiz - self.xt_osiz + self.x_tsiz - 1) / self.x_tsiz
    }
    /// Number of tiles in Y direction.
    pub fn tiles_y(&self) -> u32 {
        (self.ysiz - self.yt_osiz + self.y_tsiz - 1) / self.y_tsiz
    }
    /// Total number of tiles.
    pub fn num_tiles(&self) -> u32 { self.tiles_x() * self.tiles_y() }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 38 {
            return Err(Jp2Error::InvalidCodestream { offset: 0, message: "SIZ too short".into() });
        }
        let rsiz   = u16::from_be_bytes(data[0..2].try_into().unwrap());
        let xsiz   = u32::from_be_bytes(data[2..6].try_into().unwrap());
        let ysiz   = u32::from_be_bytes(data[6..10].try_into().unwrap());
        let x_osiz = u32::from_be_bytes(data[10..14].try_into().unwrap());
        let y_osiz = u32::from_be_bytes(data[14..18].try_into().unwrap());
        let x_tsiz = u32::from_be_bytes(data[18..22].try_into().unwrap());
        let y_tsiz = u32::from_be_bytes(data[22..26].try_into().unwrap());
        let xt_osiz = u32::from_be_bytes(data[26..30].try_into().unwrap());
        let yt_osiz = u32::from_be_bytes(data[30..34].try_into().unwrap());
        let csiz   = u16::from_be_bytes(data[34..36].try_into().unwrap());

        if data.len() < 36 + csiz as usize * 3 {
            return Err(Jp2Error::InvalidCodestream { offset: 0, message: "SIZ component data truncated".into() });
        }
        let mut components = Vec::with_capacity(csiz as usize);
        for i in 0..csiz as usize {
            let off = 36 + i * 3;
            components.push(SizComponent { ssiz: data[off], xrsiz: data[off+1], yrsiz: data[off+2] });
        }
        Ok(Self { rsiz, xsiz, ysiz, x_osiz, y_osiz, x_tsiz, y_tsiz, xt_osiz, yt_osiz, components })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let csiz = self.components.len() as u16;
        let lsiz = 38u16 + csiz * 3;
        w.write_all(&marker::SIZ.to_be_bytes())?;
        w.write_all(&lsiz.to_be_bytes())?;
        w.write_all(&self.rsiz.to_be_bytes())?;
        w.write_all(&self.xsiz.to_be_bytes())?;
        w.write_all(&self.ysiz.to_be_bytes())?;
        w.write_all(&self.x_osiz.to_be_bytes())?;
        w.write_all(&self.y_osiz.to_be_bytes())?;
        w.write_all(&self.x_tsiz.to_be_bytes())?;
        w.write_all(&self.y_tsiz.to_be_bytes())?;
        w.write_all(&self.xt_osiz.to_be_bytes())?;
        w.write_all(&self.yt_osiz.to_be_bytes())?;
        w.write_all(&csiz.to_be_bytes())?;
        for c in &self.components {
            w.write_all(&[c.ssiz, c.xrsiz, c.yrsiz])?;
        }
        Ok(())
    }
}

// ── COD: Coding style default ─────────────────────────────────────────────────

/// Progression orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ProgressionOrder {
    /// Layer–Resolution–Component–Position (most common for GIS).
    #[default]
    Lrcp = 0,
    /// Resolution–Layer–Component–Position.
    Rlcp = 1,
    /// Resolution–Position–Component–Layer.
    Rpcl = 2,
    /// Position–Component–Resolution–Layer.
    Pcrl = 3,
    /// Component–Position–Resolution–Layer.
    Cprl = 4,
}

impl ProgressionOrder {
    pub fn from_u8(v: u8) -> Self {
        match v { 1=>Self::Rlcp, 2=>Self::Rpcl, 3=>Self::Pcrl, 4=>Self::Cprl, _=>Self::Lrcp }
    }
}

/// COD marker segment.
#[derive(Debug, Clone)]
pub struct Cod {
    /// Scod: bit0=entropy_coder_bypass, bit1=reset_ctx, bit2=term_each_pass,
    ///       bit3=vert_causal_ctx, bit4=predictable_termination, bit5=seg_markers
    pub scod: u8,
    pub progression: ProgressionOrder,
    /// Number of quality layers (1 for lossless).
    pub num_layers: u16,
    /// Multiple component transformation: 0=none, 1=RCT (lossless), 2=ICT (lossy).
    pub mc_transform: u8,
    /// Number of DWT decomposition levels (0..=32).
    pub num_decomps: u8,
    /// Code-block width exponent (2..=10).  Block width = 2^(xcb+2).
    pub xcb: u8,
    /// Code-block height exponent (2..=10).
    pub ycb: u8,
    /// Code-block style: bit4=lazy_coding, bit3=reset_ctx, etc.
    pub cblk_style: u8,
    /// Wavelet transformation: 0=9/7 (lossy), 1=5/3 (lossless).
    pub wavelet: u8,
    /// Precinct sizes per resolution level (empty = default 2^15 × 2^15).
    pub precincts: Vec<u8>,
}

impl Cod {
    pub fn lossless(num_decomps: u8, num_components: u16) -> Self {
        Self {
            scod: 0,
            progression: ProgressionOrder::Lrcp,
            num_layers: 1,
            mc_transform: if num_components == 3 { 1 } else { 0 }, // RCT for RGB
            num_decomps,
            xcb: 4, // 64-sample code blocks
            ycb: 4,
            cblk_style: 0,
            wavelet: 1, // 5/3 lossless
            precincts: Vec::new(),
        }
    }

    pub fn lossy(num_decomps: u8, num_components: u16) -> Self {
        Self {
            scod: 0,
            progression: ProgressionOrder::Lrcp,
            num_layers: 1,
            mc_transform: if num_components == 3 { 2 } else { 0 }, // ICT for RGB
            num_decomps,
            xcb: 4,
            ycb: 4,
            cblk_style: 0,
            wavelet: 0, // 9/7 lossy
            precincts: Vec::new(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(Jp2Error::InvalidCodestream { offset: 0, message: "COD too short".into() });
        }
        let scod        = data[0];
        let progression = ProgressionOrder::from_u8(data[1]);
        let num_layers  = u16::from_be_bytes(data[2..4].try_into().unwrap());
        let mc_transform = data[4];
        let num_decomps = data[5];
        let xcb         = data[6];
        let ycb         = data[7];
        let cblk_style  = data[8];
        let wavelet     = data[9];
        let precincts   = if scod & 0x01 != 0 { data[10..].to_vec() } else { Vec::new() };
        Ok(Self { scod, progression, num_layers, mc_transform, num_decomps, xcb, ycb, cblk_style, wavelet, precincts })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let has_precincts = !self.precincts.is_empty();
        let scod = if has_precincts { self.scod | 0x01 } else { self.scod & !0x01 };
        let len = 12u16 + self.precincts.len() as u16;
        w.write_all(&marker::COD.to_be_bytes())?;
        w.write_all(&len.to_be_bytes())?;
        w.write_all(&[scod, self.progression as u8])?;
        w.write_all(&self.num_layers.to_be_bytes())?;
        w.write_all(&[self.mc_transform, self.num_decomps, self.xcb, self.ycb, self.cblk_style, self.wavelet])?;
        w.write_all(&self.precincts)?;
        Ok(())
    }
}

// ── QCD: Quantisation default ─────────────────────────────────────────────────

/// QCD marker segment.
#[derive(Debug, Clone)]
pub struct Qcd {
    /// Sqcd: quantisation style (0=none/scalar-derived, 1=scalar-expounded, 2=scalar-expounded-tilepart).
    pub sqcd: u8,
    /// Step-size exponent/mantissa values.  For lossless (sqcd=0) these are shift values only.
    pub step_sizes: Vec<u16>,
}

impl Qcd {
    /// No quantisation (lossless): scalar derived from dynamic range.
    pub fn no_quantisation(num_decomps: u8, bit_depth: u8) -> Self {
        // For lossless, sqcd=0 (no quantisation), step sizes are (exp << 11) values.
        let num_subbands = 3 * num_decomps as usize + 1;
        let mut step_sizes = Vec::with_capacity(num_subbands);
        for i in 0..=num_decomps as usize {
            let exp = (bit_depth + num_decomps as u8).saturating_sub(i as u8);
            if i == 0 {
                step_sizes.push((exp as u16) << 11);
            } else {
                for _ in 0..3 {
                    let e = exp.saturating_sub(1);
                    step_sizes.push((e as u16) << 11);
                }
            }
        }
        Self { sqcd: 0, step_sizes }
    }

    /// Scalar-expounded quantisation for lossy compression.
    /// Step sizes encode (exp, mantissa) as (exp << 11) | mantissa.
    pub fn scalar_expounded(num_decomps: u8, bit_depth: u8, quality_db: f32) -> Self {
        let num_subbands = 3 * num_decomps as usize + 1;
        let base_step = (2.0f32.powf(-(quality_db / 20.0).max(0.1))) as f64;
        let mut step_sizes = Vec::with_capacity(num_subbands);
        for i in 0..=num_decomps as usize {
            let level_step = base_step * (2.0f64.powi(i as i32));
            let exp = level_step.log2().floor() as i32 + bit_depth as i32;
            let exp = exp.clamp(0, 31) as u16;
            let mantissa = ((level_step / 2.0f64.powi(exp as i32 - bit_depth as i32 + 1)) * 2048.0) as u16 & 0x7FF;
            if i == 0 {
                step_sizes.push((exp << 11) | mantissa);
            } else {
                for _ in 0..3 { step_sizes.push((exp << 11) | mantissa); }
            }
        }
        Self { sqcd: 2, step_sizes }
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Jp2Error::InvalidCodestream { offset: 0, message: "QCD empty".into() });
        }
        let sqcd = data[0];
        let step_sizes: Vec<u16> = match sqcd & 0x1F {
            0 => data[1..].iter().map(|&b| (b as u16) << 8).collect(),  // no quantisation
            _ => data[1..].chunks_exact(2).map(|c| u16::from_be_bytes(c.try_into().unwrap())).collect(),
        };
        Ok(Self { sqcd, step_sizes })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let is_noquant = self.sqcd & 0x1F == 0;
        let body_len = if is_noquant { self.step_sizes.len() } else { self.step_sizes.len() * 2 };
        let len = (2 + body_len) as u16;
        w.write_all(&marker::QCD.to_be_bytes())?;
        w.write_all(&len.to_be_bytes())?;
        w.write_all(&[self.sqcd])?;
        if is_noquant {
            for &s in &self.step_sizes { w.write_all(&[(s >> 8) as u8])?; }
        } else {
            for &s in &self.step_sizes { w.write_all(&s.to_be_bytes())?; }
        }
        Ok(())
    }
}

// ── SOT: Start of tile-part ───────────────────────────────────────────────────

/// SOT marker segment.
#[derive(Debug, Clone, Copy)]
pub struct Sot {
    /// Tile index (0-based).
    pub isot: u16,
    /// Length of tile-part in bytes (includes SOT marker segment itself).
    /// 0 = unknown (last tile-part).
    pub psot: u32,
    /// Tile-part index.
    pub tpsot: u8,
    /// Total number of tile-parts for this tile (0 = unknown).
    pub tnsot: u8,
}

impl Sot {
    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&marker::SOT.to_be_bytes())?;
        w.write_all(&10u16.to_be_bytes())?; // Lsot = 10
        w.write_all(&self.isot.to_be_bytes())?;
        w.write_all(&self.psot.to_be_bytes())?;
        w.write_all(&[self.tpsot, self.tnsot])?;
        Ok(())
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(Jp2Error::InvalidCodestream { offset: 0, message: "SOT too short".into() });
        }
        Ok(Self {
            isot:  u16::from_be_bytes(data[0..2].try_into().unwrap()),
            psot:  u32::from_be_bytes(data[2..6].try_into().unwrap()),
            tpsot: data[6],
            tnsot: data[7],
        })
    }
}

// ── CME: Comment ──────────────────────────────────────────────────────────────

pub fn write_comment<W: Write>(w: &mut W, text: &str) -> std::io::Result<()> {
    let bytes = text.as_bytes();
    let len = (4 + bytes.len()) as u16;
    w.write_all(&marker::CME.to_be_bytes())?;
    w.write_all(&len.to_be_bytes())?;
    w.write_all(&1u16.to_be_bytes())?; // Rcom=1 (Latin/ISO 8859-1 text)
    w.write_all(bytes)?;
    Ok(())
}

// ── Codestream marker reader ──────────────────────────────────────────────────

/// A single parsed marker segment from a codestream.
#[derive(Debug)]
pub struct MarkerSegment {
    pub marker: u16,
    /// Segment data (excludes the 2-byte marker code and 2-byte length field).
    pub data: Vec<u8>,
    /// Byte offset in the codestream (position of the 0xFF byte).
    pub offset: usize,
}

/// Read all marker segments from a raw codestream byte slice.
pub fn parse_codestream_markers(cs: &[u8]) -> Result<Vec<MarkerSegment>> {
    let mut segments = Vec::new();
    let mut i = 0;

    // Verify SOC
    if cs.len() < 2 || cs[0] != 0xFF || cs[1] != 0x4F {
        return Err(Jp2Error::InvalidCodestream {
            offset: 0,
            message: "Missing SOC marker".into(),
        });
    }
    segments.push(MarkerSegment { marker: marker::SOC, data: Vec::new(), offset: 0 });
    i = 2;

    while i < cs.len() {
        if cs[i] != 0xFF { i += 1; continue; }
        if i + 1 >= cs.len() { break; }

        let m = u16::from_be_bytes([cs[i], cs[i+1]]);
        i += 2;

        // Markers without a length field (standalone)
        match m {
            marker::SOC | marker::SOD | marker::EOC => {
                segments.push(MarkerSegment { marker: m, data: Vec::new(), offset: i - 2 });
                if m == marker::SOD { break; } // tile data follows; stop scanning
                continue;
            }
            _ => {}
        }

        if i + 2 > cs.len() { break; }
        let lseg = u16::from_be_bytes([cs[i], cs[i+1]]) as usize;
        if lseg < 2 || i + lseg > cs.len() {
            return Err(Jp2Error::InvalidCodestream {
                offset: i,
                message: format!("Marker 0x{:04X} has invalid length {}", m, lseg),
            });
        }
        let data = cs[i+2..i+lseg].to_vec();
        segments.push(MarkerSegment { marker: m, data, offset: i - 2 });
        i += lseg;
    }

    Ok(segments)
}
