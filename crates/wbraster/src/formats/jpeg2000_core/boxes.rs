//! JPEG 2000 JP2 file format box (superbox) parsing and writing.
//!
//! The JP2 file format wraps a JPEG 2000 codestream in a series of typed
//! length-prefixed boxes:
//!
//! ```text
//! [LBox(4)] [TBox(4)] [XLBox(8, optional)] [data…]
//! ```
//!
//! - `LBox = 0`  → box extends to end of file
//! - `LBox = 1`  → `XLBox` follows and contains the true 64-bit length
//! - `LBox ≥ 8`  → total box length including header
//!
//! Standard JP2 box sequence:
//! 1. Signature box  (`jP  ` / `6A502020`)
//! 2. File type box  (`ftyp`)
//! 3. JP2 Header box (`jp2h`) — superbox containing `ihdr`, `colr`, optionally `pclr`, `cmap`, `res `
//! 4. UUID box       (`uuid`) — GeoJP2 geolocation metadata (optional)
//! 5. XML box        (`xml `) — GML/arbitrary XML metadata (optional)
//! 6. Contiguous Codestream box (`jp2c`) — the actual JPEG 2000 codestream

use std::io::{Read, Seek, SeekFrom, Write};
use super::error::{Jp2Error, Result};

// ── Box type constants ────────────────────────────────────────────────────────

pub mod box_type {
    pub const SIGNATURE:    [u8; 4] = *b"jP  ";
    pub const FILE_TYPE:    [u8; 4] = *b"ftyp";
    pub const JP2_HEADER:   [u8; 4] = *b"jp2h";
    pub const IMAGE_HEADER: [u8; 4] = *b"ihdr";
    pub const COLOUR_SPEC:  [u8; 4] = *b"colr";
    pub const PALETTE:      [u8; 4] = *b"pclr";
    pub const COMP_MAP:     [u8; 4] = *b"cmap";
    pub const CHAN_DEF:      [u8; 4] = *b"cdef";
    pub const RESOLUTION:   [u8; 4] = *b"res ";
    pub const CODESTREAM:   [u8; 4] = *b"jp2c";
    pub const UUID:         [u8; 4] = *b"uuid";
    pub const UUID_INFO:    [u8; 4] = *b"uinf";
    pub const XML:          [u8; 4] = *b"xml ";
    pub const INTELLECTUAL: [u8; 4] = *b"jp2i";
}

/// UUID identifying the GeoJP2 box (contains embedded GeoTIFF metadata bytes).
pub const GEOJP2_UUID: [u8; 16] = [
    0xb1, 0x4b, 0xf8, 0xbd, 0x08, 0x3d, 0x4b, 0x43,
    0xa5, 0xae, 0x8c, 0xd7, 0xd5, 0xa6, 0xce, 0x03,
];

/// UUID identifying a World File box (alternate geolocation, less common).
pub const WORLD_FILE_UUID: [u8; 16] = [
    0x96, 0xa9, 0xf1, 0xf1, 0xdc, 0x98, 0x40, 0x2d,
    0xa7, 0xae, 0xd6, 0x8e, 0x34, 0x45, 0x18, 0x09,
];

// ── RawBox ────────────────────────────────────────────────────────────────────

/// A fully-read JP2 box: header + payload bytes.
#[derive(Debug, Clone)]
pub struct RawBox {
    /// 4-byte box type.
    pub box_type: [u8; 4],
    /// File offset of the first byte of the box header.
    pub file_offset: u64,
    /// Payload bytes (excludes the LBox/TBox/XLBox header).
    pub data: Vec<u8>,
}

impl RawBox {
    /// Box type as a UTF-8 string (lossy) for error messages.
    pub fn type_str(&self) -> String {
        String::from_utf8_lossy(&self.box_type).into_owned()
    }

    /// Whether this box's type matches the given 4-byte constant.
    pub fn is(&self, t: [u8; 4]) -> bool { self.box_type == t }
}

// ── BoxReader ─────────────────────────────────────────────────────────────────

/// Reads JP2 boxes sequentially from any `Read + Seek` source.
pub struct BoxReader<R: Read + Seek> {
    inner: R,
    file_len: u64,
}

impl<R: Read + Seek> BoxReader<R> {
    /// Wrap a reader. `file_len` is used to handle LBox=0 (to-EOF) boxes.
    pub fn new(mut inner: R) -> Result<Self> {
        let file_len = inner.seek(SeekFrom::End(0)).map_err(Jp2Error::Io)?;
        inner.seek(SeekFrom::Start(0)).map_err(Jp2Error::Io)?;
        Ok(Self { inner, file_len })
    }

    /// Read the next box from the current stream position.
    /// Returns `None` at EOF.
    pub fn next_box(&mut self) -> Result<Option<RawBox>> {
        let file_offset = self.inner.stream_position().map_err(Jp2Error::Io)?;
        if file_offset >= self.file_len { return Ok(None); }

        let mut hdr = [0u8; 8];
        match self.inner.read_exact(&mut hdr) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(Jp2Error::Io(e)),
            Ok(_)  => {}
        }

        let lbox = u32::from_be_bytes(hdr[0..4].try_into().unwrap());
        let box_type: [u8; 4] = hdr[4..8].try_into().unwrap();

        let (header_size, payload_len): (u64, u64) = match lbox {
            0 => {
                // Box extends to end of file
                (8, self.file_len.saturating_sub(file_offset + 8))
            }
            1 => {
                // XLBox follows — 8-byte true length
                let mut xlbox = [0u8; 8];
                self.inner.read_exact(&mut xlbox).map_err(Jp2Error::Io)?;
                let total = u64::from_be_bytes(xlbox);
                if total < 16 {
                    return Err(Jp2Error::InvalidBox {
                        box_type: String::from_utf8_lossy(&box_type).into_owned(),
                        message: format!("XLBox value {} is too small", total),
                    });
                }
                (16, total - 16)
            }
            n if n < 8 => {
                return Err(Jp2Error::InvalidBox {
                    box_type: String::from_utf8_lossy(&box_type).into_owned(),
                    message: format!("LBox value {} is invalid (must be 0, 1, or ≥ 8)", n),
                });
            }
            n => (8, (n as u64) - 8),
        };

        let mut data = vec![0u8; payload_len as usize];
        self.inner.read_exact(&mut data).map_err(Jp2Error::Io)?;

        Ok(Some(RawBox { box_type, file_offset, data }))
    }

    /// Collect all top-level boxes into a Vec.
    pub fn read_all(&mut self) -> Result<Vec<RawBox>> {
        let mut boxes = Vec::new();
        while let Some(b) = self.next_box()? { boxes.push(b); }
        Ok(boxes)
    }

    /// Read all sub-boxes from a superbox's payload (e.g. `jp2h`).
    pub fn sub_boxes(data: &[u8]) -> Result<Vec<RawBox>> {
        let mut cur = std::io::Cursor::new(data);
        let mut reader = BoxReader::new(&mut cur)?;
        reader.read_all()
    }
}

// ── Box writers ───────────────────────────────────────────────────────────────

/// Write a JP2 box to `w`.  If `payload` is empty, only the 8-byte header is written.
pub fn write_box<W: Write>(w: &mut W, box_type: [u8; 4], payload: &[u8]) -> std::io::Result<()> {
    let total = 8u64 + payload.len() as u64;
    if total <= u32::MAX as u64 {
        w.write_all(&(total as u32).to_be_bytes())?;
        w.write_all(&box_type)?;
    } else {
        // XLBox form
        w.write_all(&1u32.to_be_bytes())?;
        w.write_all(&box_type)?;
        w.write_all(&total.to_be_bytes())?;
    }
    w.write_all(payload)
}

/// Write a superbox (a box whose payload consists of child boxes).
pub fn write_super_box<W: Write>(
    w: &mut W,
    box_type: [u8; 4],
    children: &[u8],
) -> std::io::Result<()> {
    write_box(w, box_type, children)
}

// ── Signature box ─────────────────────────────────────────────────────────────

/// Verify and skip (or write) the JP2 12-byte signature box.
///
/// The signature box contains `0x0D0A870A` as its payload.
pub const JP2_SIGNATURE_PAYLOAD: [u8; 4] = [0x0D, 0x0A, 0x87, 0x0A];

pub fn validate_signature(b: &RawBox) -> Result<()> {
    if !b.is(box_type::SIGNATURE) {
        return Err(Jp2Error::NotJp2(format!(
            "Expected signature box 'jP  ', found '{}'",
            b.type_str()
        )));
    }
    if b.data != JP2_SIGNATURE_PAYLOAD {
        return Err(Jp2Error::NotJp2(
            "Signature box payload does not match JP2 magic bytes".into()
        ));
    }
    Ok(())
}

pub fn write_signature<W: Write>(w: &mut W) -> std::io::Result<()> {
    write_box(w, box_type::SIGNATURE, &JP2_SIGNATURE_PAYLOAD)
}

// ── File type box ─────────────────────────────────────────────────────────────

/// JP2 `ftyp` box: brand `jp2 `, minor version 0, compatibility list `jp2 `.
pub fn write_file_type<W: Write>(w: &mut W) -> std::io::Result<()> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"jp2 ");   // BR (brand)
    payload.extend_from_slice(&0u32.to_be_bytes()); // MinV
    payload.extend_from_slice(b"jp2 ");   // CL (compatibility list)
    write_box(w, box_type::FILE_TYPE, &payload)
}

// ── Image header box (ihdr) ───────────────────────────────────────────────────

/// Parse the `ihdr` box payload.
#[derive(Debug, Clone)]
pub struct ImageHeader {
    pub height:     u32,
    pub width:      u32,
    pub components: u16,
    /// Bit depth minus 1 (per component; we use the first component value).
    pub bpc:        u8,
    /// Compression type: 7 = JP2.
    pub c:          u8,
    /// Colourspace unknown flag.
    pub unk_c:      u8,
    /// Intellectual property flag.
    pub ipr:        u8,
}

impl ImageHeader {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 14 {
            return Err(Jp2Error::InvalidBox {
                box_type: "ihdr".into(),
                message: format!("payload too short: {} bytes", data.len()),
            });
        }
        Ok(Self {
            height:     u32::from_be_bytes(data[0..4].try_into().unwrap()),
            width:      u32::from_be_bytes(data[4..8].try_into().unwrap()),
            components: u16::from_be_bytes(data[8..10].try_into().unwrap()),
            bpc:        data[10],
            c:          data[11],
            unk_c:      data[12],
            ipr:        data[13],
        })
    }

    /// Actual bits per component (bpc field is value-1, or 0xFF for variable).
    pub fn bits_per_component(&self) -> u8 {
        if self.bpc == 0xFF { 0 } else { (self.bpc & 0x7F) + 1 }
    }

    /// Whether samples are signed (MSB of bpc).
    pub fn is_signed(&self) -> bool { self.bpc & 0x80 != 0 }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.height.to_be_bytes());
        payload.extend_from_slice(&self.width.to_be_bytes());
        payload.extend_from_slice(&self.components.to_be_bytes());
        payload.push(self.bpc);
        payload.push(self.c);
        payload.push(self.unk_c);
        payload.push(self.ipr);
        write_box(w, box_type::IMAGE_HEADER, &payload)
    }
}

// ── Colour spec box (colr) ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ColourSpec {
    /// Method: 1 = enumerated colourspace, 2 = ICC profile.
    pub meth: u8,
    pub prec: i8,
    pub approx: u8,
    /// Enumerated colourspace (if meth=1).
    pub enumcs: Option<u32>,
    /// ICC profile bytes (if meth=2).
    pub icc_profile: Option<Vec<u8>>,
}

impl ColourSpec {
    pub fn enumerated(cs: u32) -> Self {
        Self { meth: 1, prec: 0, approx: 0, enumcs: Some(cs), icc_profile: None }
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 3 {
            return Err(Jp2Error::InvalidBox {
                box_type: "colr".into(),
                message: "payload too short".into(),
            });
        }
        let meth   = data[0];
        let prec   = data[1] as i8;
        let approx = data[2];
        let (enumcs, icc_profile) = match meth {
            1 => {
                if data.len() < 7 {
                    return Err(Jp2Error::InvalidBox { box_type: "colr".into(), message: "enumerated colourspace truncated".into() });
                }
                (Some(u32::from_be_bytes(data[3..7].try_into().unwrap())), None)
            }
            2 => (None, Some(data[3..].to_vec())),
            _ => (None, None),
        };
        Ok(Self { meth, prec, approx, enumcs, icc_profile })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let mut payload = vec![self.meth, self.prec as u8, self.approx];
        if let Some(cs) = self.enumcs {
            payload.extend_from_slice(&cs.to_be_bytes());
        }
        if let Some(ref icc) = self.icc_profile {
            payload.extend_from_slice(icc);
        }
        write_box(w, box_type::COLOUR_SPEC, &payload)
    }
}

// ── UUID box (GeoJP2) ─────────────────────────────────────────────────────────

/// Write a GeoJP2 UUID box containing embedded GeoTIFF metadata bytes.
///
/// The payload is: 16-byte UUID + GeoTIFF tag data (identical to what would
/// appear inside a GeoTIFF file's IFD region).
pub fn write_uuid_box<W: Write>(w: &mut W, uuid: &[u8; 16], data: &[u8]) -> std::io::Result<()> {
    let mut payload = Vec::with_capacity(16 + data.len());
    payload.extend_from_slice(uuid);
    payload.extend_from_slice(data);
    write_box(w, box_type::UUID, &payload)
}

/// Parse a UUID box, returning `(uuid, payload_after_uuid)`.
pub fn parse_uuid_box(b: &RawBox) -> Result<([u8; 16], &[u8])> {
    if b.data.len() < 16 {
        return Err(Jp2Error::InvalidBox {
            box_type: "uuid".into(),
            message: "UUID box too short".into(),
        });
    }
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&b.data[..16]);
    Ok((uuid, &b.data[16..]))
}

// ── XML box ───────────────────────────────────────────────────────────────────

pub fn write_xml_box<W: Write>(w: &mut W, xml: &str) -> std::io::Result<()> {
    write_box(w, box_type::XML, xml.as_bytes())
}

// ── Resolution box ────────────────────────────────────────────────────────────

/// Capture resolution sub-box (`resc`) payload.
/// Numerators/denominators for vertical and horizontal resolution.
#[derive(Debug, Clone)]
pub struct ResolutionBox {
    pub vr_n: u16, pub vr_d: u16,
    pub hr_n: u16, pub hr_d: u16,
    pub vr_e: i8,  pub hr_e: i8,
}

impl ResolutionBox {
    /// Write a `res ` superbox containing a `resc` capture-resolution sub-box.
    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let mut resc = Vec::new();
        resc.extend_from_slice(&self.vr_n.to_be_bytes());
        resc.extend_from_slice(&self.vr_d.to_be_bytes());
        resc.extend_from_slice(&self.hr_n.to_be_bytes());
        resc.extend_from_slice(&self.hr_d.to_be_bytes());
        resc.push(self.vr_e as u8);
        resc.push(self.hr_e as u8);

        let mut resc_box = Vec::new();
        write_box(&mut resc_box, *b"resc", &resc)?;

        write_super_box(w, box_type::RESOLUTION, &resc_box)
    }
}
