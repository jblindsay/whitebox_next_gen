//! TIFF and BigTIFF Image File Directory (IFD) reading and writing utilities.
//!
//! This module handles the low-level TIFF structures for both classic TIFF
//! (32-bit offsets, magic=42) and BigTIFF (64-bit offsets, magic=43).
//!
//! ## Classic TIFF vs BigTIFF structural differences
//!
//! | Property          | Classic TIFF  | BigTIFF        |
//! |-------------------|---------------|----------------|
//! | Magic number      | 42            | 43             |
//! | Header size       | 8 bytes       | 16 bytes       |
//! | Offset width      | 4 bytes (u32) | 8 bytes (u64)  |
//! | IFD entry count   | u16 (2 bytes) | u64 (8 bytes)  |
//! | IFD entry size    | 12 bytes      | 20 bytes       |
//! | Inline value size | 4 bytes       | 8 bytes        |

#![allow(dead_code)]

use std::io::{Read, Seek, SeekFrom};

use super::error::{GeoTiffError, Result};
use super::tags::DataType;

// ── Byte order ────────────────────────────────────────────────────────────────

/// TIFF byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// Little-endian (Intel, "II").
    LittleEndian,
    /// Big-endian (Motorola, "MM").
    BigEndian,
}

impl ByteOrder {
    pub(crate) fn read_u16(self, buf: &[u8; 2]) -> u16 {
        match self { Self::LittleEndian => u16::from_le_bytes(*buf), Self::BigEndian => u16::from_be_bytes(*buf) }
    }
    pub(crate) fn read_u32(self, buf: &[u8; 4]) -> u32 {
        match self { Self::LittleEndian => u32::from_le_bytes(*buf), Self::BigEndian => u32::from_be_bytes(*buf) }
    }
    pub(crate) fn read_u64(self, buf: &[u8; 8]) -> u64 {
        match self { Self::LittleEndian => u64::from_le_bytes(*buf), Self::BigEndian => u64::from_be_bytes(*buf) }
    }
    pub(crate) fn read_i16(self, buf: &[u8; 2]) -> i16 {
        match self { Self::LittleEndian => i16::from_le_bytes(*buf), Self::BigEndian => i16::from_be_bytes(*buf) }
    }
    pub(crate) fn read_i32(self, buf: &[u8; 4]) -> i32 {
        match self { Self::LittleEndian => i32::from_le_bytes(*buf), Self::BigEndian => i32::from_be_bytes(*buf) }
    }
    pub(crate) fn read_i64(self, buf: &[u8; 8]) -> i64 {
        match self { Self::LittleEndian => i64::from_le_bytes(*buf), Self::BigEndian => i64::from_be_bytes(*buf) }
    }
    pub(crate) fn read_f32(self, buf: &[u8; 4]) -> f32 {
        match self { Self::LittleEndian => f32::from_le_bytes(*buf), Self::BigEndian => f32::from_be_bytes(*buf) }
    }
    pub(crate) fn read_f64(self, buf: &[u8; 8]) -> f64 {
        match self { Self::LittleEndian => f64::from_le_bytes(*buf), Self::BigEndian => f64::from_be_bytes(*buf) }
    }

    /// Encode a u16 in this byte order.
    pub fn u16_bytes(self, v: u16) -> [u8; 2] {
        match self { Self::LittleEndian => v.to_le_bytes(), Self::BigEndian => v.to_be_bytes() }
    }
    /// Encode a u32 in this byte order.
    pub fn u32_bytes(self, v: u32) -> [u8; 4] {
        match self { Self::LittleEndian => v.to_le_bytes(), Self::BigEndian => v.to_be_bytes() }
    }
    /// Encode a u64 in this byte order.
    pub fn u64_bytes(self, v: u64) -> [u8; 8] {
        match self { Self::LittleEndian => v.to_le_bytes(), Self::BigEndian => v.to_be_bytes() }
    }
    /// Encode a f32 in this byte order.
    pub fn f32_bytes(self, v: f32) -> [u8; 4] {
        match self { Self::LittleEndian => v.to_le_bytes(), Self::BigEndian => v.to_be_bytes() }
    }
    /// Encode a f64 in this byte order.
    pub fn f64_bytes(self, v: f64) -> [u8; 8] {
        match self { Self::LittleEndian => v.to_le_bytes(), Self::BigEndian => v.to_be_bytes() }
    }
}

// ── TiffVariant ───────────────────────────────────────────────────────────────

/// Distinguishes classic TIFF (32-bit) from BigTIFF (64-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TiffVariant {
    /// Classic TIFF: magic=42, 4-byte offsets, 4-byte inline, 12-byte entries.
    Classic,
    /// BigTIFF: magic=43, 8-byte offsets, 8-byte inline, 20-byte entries.
    BigTiff,
}

impl TiffVariant {
    /// Size of each IFD entry in bytes.
    pub fn ifd_entry_size(self) -> u64 {
        match self { Self::Classic => 12, Self::BigTiff => 20 }
    }
    /// Size of the inline value / offset field in an IFD entry.
    pub fn inline_size(self) -> usize {
        match self { Self::Classic => 4, Self::BigTiff => 8 }
    }
    /// True for BigTIFF.
    pub fn is_bigtiff(self) -> bool { self == Self::BigTiff }
}

// ── IfdValue ──────────────────────────────────────────────────────────────────

/// Decoded value of one IFD entry.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum IfdValue {
    Bytes(Vec<u8>),
    Shorts(Vec<u16>),
    Longs(Vec<u32>),
    Long8s(Vec<u64>),
    Rationals(Vec<(u32, u32)>),
    SBytes(Vec<i8>),
    SShorts(Vec<i16>),
    SLongs(Vec<i32>),
    SLong8s(Vec<i64>),
    SRationals(Vec<(i32, i32)>),
    Floats(Vec<f32>),
    Doubles(Vec<f64>),
    Ascii(String),
    Undefined(Vec<u8>),
}

impl IfdValue {
    /// First numeric value as u64.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Bytes(v)   => v.first().map(|&x| x as u64),
            Self::Shorts(v)  => v.first().map(|&x| x as u64),
            Self::Longs(v)   => v.first().map(|&x| x as u64),
            Self::Long8s(v)  => v.first().copied(),
            Self::SBytes(v)  => v.first().map(|&x| x as u64),
            Self::SShorts(v) => v.first().map(|&x| x as u64),
            Self::SLongs(v)  => v.first().map(|&x| x as u64),
            Self::SLong8s(v) => v.first().map(|&x| x as u64),
            _ => None,
        }
    }

    /// All integer values as `Vec<u64>`.
    pub fn as_u64_vec(&self) -> Option<Vec<u64>> {
        match self {
            Self::Bytes(v)  => Some(v.iter().map(|&x| x as u64).collect()),
            Self::Shorts(v) => Some(v.iter().map(|&x| x as u64).collect()),
            Self::Longs(v)  => Some(v.iter().map(|&x| x as u64).collect()),
            Self::Long8s(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// All values as `Vec<f64>`.
    pub fn as_f64_vec(&self) -> Option<Vec<f64>> {
        match self {
            Self::Doubles(v)   => Some(v.clone()),
            Self::Floats(v)    => Some(v.iter().map(|&x| x as f64).collect()),
            Self::Rationals(v) => Some(v.iter().map(|&(n,d)| if d==0 {0.0} else {n as f64/d as f64}).collect()),
            Self::Shorts(v)    => Some(v.iter().map(|&x| x as f64).collect()),
            Self::Longs(v)     => Some(v.iter().map(|&x| x as f64).collect()),
            Self::Long8s(v)    => Some(v.iter().map(|&x| x as f64).collect()),
            _ => None,
        }
    }

    /// As &str for ASCII entries.
    pub fn as_str(&self) -> Option<&str> {
        match self { Self::Ascii(s) => Some(s.as_str()), _ => None }
    }

    /// Raw bytes for Byte/Undefined entries.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self { Self::Bytes(v) | Self::Undefined(v) => Some(v.as_slice()), _ => None }
    }

    /// Short (u16) slice.
    pub fn as_u16_vec(&self) -> Option<&[u16]> {
        match self { Self::Shorts(v) => Some(v.as_slice()), _ => None }
    }
}

// ── IfdEntry ─────────────────────────────────────────────────────────────────

/// One parsed IFD entry.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct IfdEntry {
    pub tag:       u16,
    pub data_type: DataType,
    /// Value count (u64 in BigTIFF; u32 in classic, promoted here).
    pub count:     u64,
    pub value:     IfdValue,
}

// ── Ifd ──────────────────────────────────────────────────────────────────────

/// A decoded TIFF / BigTIFF Image File Directory.
#[derive(Debug, Clone, Default)]
#[allow(missing_docs)]
pub struct Ifd {
    pub entries:         Vec<IfdEntry>,
    /// Next IFD offset (0 = none). u64 to cover both classic and BigTIFF.
    pub next_ifd_offset: u64,
}

impl Ifd {
    #[allow(missing_docs)]
    pub fn get(&self, tag_code: u16) -> Option<&IfdEntry> {
        self.entries.iter().find(|e| e.tag == tag_code)
    }

    #[allow(missing_docs)]
    pub fn require_u64(&self, tag_code: u16, name: &'static str) -> Result<u64> {
        self.get(tag_code).and_then(|e| e.value.as_u64())
            .ok_or(GeoTiffError::MissingTag { tag: name, code: tag_code })
    }

    #[allow(missing_docs)]
    pub fn require_u64_vec(&self, tag_code: u16, name: &'static str) -> Result<Vec<u64>> {
        self.get(tag_code).and_then(|e| e.value.as_u64_vec())
            .ok_or(GeoTiffError::MissingTag { tag: name, code: tag_code })
    }
}

// ── TiffReader ────────────────────────────────────────────────────────────────

/// Low-level reader for both classic TIFF and BigTIFF.
#[allow(missing_docs)]
pub struct TiffReader<R: Read + Seek> {
    reader:            R,
    pub byte_order:    ByteOrder,
    pub variant:       TiffVariant,
    pub first_ifd_offset: u64,
}

impl<R: Read + Seek> TiffReader<R> {
    /// Parse the file header and detect classic vs BigTIFF.
    pub fn new(mut reader: R) -> Result<Self> {
        let mut hdr4 = [0u8; 4];
        reader.read_exact(&mut hdr4).map_err(GeoTiffError::Io)?;

        let byte_order = match &hdr4[0..2] {
            b"II" => ByteOrder::LittleEndian,
            b"MM" => ByteOrder::BigEndian,
            other => return Err(GeoTiffError::InvalidTiff(
                format!("Unknown byte order marker: {:02X?}", other))),
        };

        let magic = byte_order.read_u16(hdr4[2..4].try_into().unwrap());

        let (variant, first_ifd_offset) = match magic {
            42 => {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf).map_err(GeoTiffError::Io)?;
                (TiffVariant::Classic, byte_order.read_u32(&buf) as u64)
            }
            43 => {
                // BigTIFF extended header: offsetsize(2) + always0(2) + ifd_offset(8)
                let mut ext = [0u8; 12];
                reader.read_exact(&mut ext).map_err(GeoTiffError::Io)?;
                let offset_size = byte_order.read_u16(ext[0..2].try_into().unwrap());
                if offset_size != 8 {
                    return Err(GeoTiffError::InvalidTiff(
                        format!("BigTIFF: expected offset size 8, got {}", offset_size)));
                }
                let off = byte_order.read_u64(ext[4..12].try_into().unwrap());
                (TiffVariant::BigTiff, off)
            }
            m => return Err(GeoTiffError::InvalidTiff(format!("Bad TIFF magic: {}", m))),
        };

        Ok(Self { reader, byte_order, variant, first_ifd_offset })
    }

    // ── Primitive reads ───────────────────────────────────────────────────────

    fn read_u16(&mut self) -> std::io::Result<u16> { let mut b=[0u8;2]; self.reader.read_exact(&mut b)?; Ok(self.byte_order.read_u16(&b)) }
    fn read_u32(&mut self) -> std::io::Result<u32> { let mut b=[0u8;4]; self.reader.read_exact(&mut b)?; Ok(self.byte_order.read_u32(&b)) }
    fn read_u64(&mut self) -> std::io::Result<u64> { let mut b=[0u8;8]; self.reader.read_exact(&mut b)?; Ok(self.byte_order.read_u64(&b)) }

    fn read_offset(&mut self) -> std::io::Result<u64> {
        match self.variant { TiffVariant::Classic => self.read_u32().map(|v| v as u64), TiffVariant::BigTiff => self.read_u64() }
    }

    // ── IFD reading ───────────────────────────────────────────────────────────

    #[allow(missing_docs)]
    pub fn read_all_ifds(&mut self) -> Result<Vec<Ifd>> {
        let mut ifds = Vec::new();
        let mut offset = self.first_ifd_offset;
        while offset != 0 {
            let ifd = self.read_ifd(offset)?;
            offset = ifd.next_ifd_offset;
            ifds.push(ifd);
        }
        Ok(ifds)
    }

    #[allow(missing_docs)]
    pub fn read_ifd(&mut self, offset: u64) -> Result<Ifd> {
        self.reader.seek(SeekFrom::Start(offset)).map_err(GeoTiffError::Io)?;

        let num_entries: u64 = match self.variant {
            TiffVariant::Classic => self.read_u16().map_err(GeoTiffError::Io)? as u64,
            TiffVariant::BigTiff => self.read_u64().map_err(GeoTiffError::Io)?,
        };

        let inline = self.variant.inline_size();
        let mut raw: Vec<(u16, u16, u64, Vec<u8>)> = Vec::with_capacity(num_entries as usize);

        for _ in 0..num_entries {
            let tag_code  = self.read_u16().map_err(GeoTiffError::Io)?;
            let type_code = self.read_u16().map_err(GeoTiffError::Io)?;
            let count: u64 = match self.variant {
                TiffVariant::Classic => self.read_u32().map_err(GeoTiffError::Io)? as u64,
                TiffVariant::BigTiff => self.read_u64().map_err(GeoTiffError::Io)?,
            };
            let mut vbuf = vec![0u8; inline];
            self.reader.read_exact(&mut vbuf).map_err(GeoTiffError::Io)?;
            raw.push((tag_code, type_code, count, vbuf));
        }

        let next_ifd_offset = self.read_offset().map_err(GeoTiffError::Io)?;

        let mut entries = Vec::with_capacity(raw.len());
        for (tag_code, type_code, count, vbuf) in raw {
            let Some(data_type) = DataType::from_u16(type_code) else { continue };
            let value = self.decode_value(data_type, count, &vbuf)?;
            entries.push(IfdEntry { tag: tag_code, data_type, count, value });
        }

        Ok(Ifd { entries, next_ifd_offset })
    }

    fn decode_value(&mut self, data_type: DataType, count: u64, inline_buf: &[u8]) -> Result<IfdValue> {
        let total_bytes = data_type.byte_size().saturating_mul(count as usize);
        let inline_size = self.variant.inline_size();

        let data: Vec<u8> = if total_bytes <= inline_size {
            inline_buf[..total_bytes].to_vec()
        } else {
            let offset: u64 = match self.variant {
                TiffVariant::Classic => self.byte_order.read_u32(inline_buf[..4].try_into().unwrap()) as u64,
                TiffVariant::BigTiff => self.byte_order.read_u64(inline_buf[..8].try_into().unwrap()),
            };
            let pos = self.reader.stream_position().map_err(GeoTiffError::Io)?;
            self.reader.seek(SeekFrom::Start(offset)).map_err(GeoTiffError::Io)?;
            let mut buf = vec![0u8; total_bytes];
            self.reader.read_exact(&mut buf).map_err(GeoTiffError::Io)?;
            self.reader.seek(SeekFrom::Start(pos)).map_err(GeoTiffError::Io)?;
            buf
        };

        let bo = self.byte_order;
        Ok(match data_type {
            DataType::Byte      => IfdValue::Bytes(data),
            DataType::Ascii     => IfdValue::Ascii(String::from_utf8_lossy(&data).trim_end_matches('\0').to_owned()),
            DataType::Short     => IfdValue::Shorts(data.chunks_exact(2).map(|c| bo.read_u16(c.try_into().unwrap())).collect()),
            DataType::Long      => IfdValue::Longs(data.chunks_exact(4).map(|c| bo.read_u32(c.try_into().unwrap())).collect()),
            DataType::Long8     => IfdValue::Long8s(data.chunks_exact(8).map(|c| bo.read_u64(c.try_into().unwrap())).collect()),
            DataType::Rational  => IfdValue::Rationals(data.chunks_exact(8).map(|c| (bo.read_u32(c[0..4].try_into().unwrap()), bo.read_u32(c[4..8].try_into().unwrap()))).collect()),
            DataType::SByte     => IfdValue::SBytes(data.iter().map(|&b| b as i8).collect()),
            DataType::Undefined => IfdValue::Undefined(data),
            DataType::SShort    => IfdValue::SShorts(data.chunks_exact(2).map(|c| bo.read_i16(c.try_into().unwrap())).collect()),
            DataType::SLong     => IfdValue::SLongs(data.chunks_exact(4).map(|c| bo.read_i32(c.try_into().unwrap())).collect()),
            DataType::SLong8    => IfdValue::SLong8s(data.chunks_exact(8).map(|c| bo.read_i64(c.try_into().unwrap())).collect()),
            DataType::SRational => IfdValue::SRationals(data.chunks_exact(8).map(|c| (bo.read_i32(c[0..4].try_into().unwrap()), bo.read_i32(c[4..8].try_into().unwrap()))).collect()),
            DataType::Float     => IfdValue::Floats(data.chunks_exact(4).map(|c| bo.read_f32(c.try_into().unwrap())).collect()),
            DataType::Double    => IfdValue::Doubles(data.chunks_exact(8).map(|c| bo.read_f64(c.try_into().unwrap())).collect()),
        })
    }

    // ── Raw reads ─────────────────────────────────────────────────────────────

    #[allow(missing_docs)]
    pub fn read_bytes_at(&mut self, offset: u64, len: usize) -> Result<Vec<u8>> {
        self.reader.seek(SeekFrom::Start(offset)).map_err(GeoTiffError::Io)?;
        let mut buf = vec![0u8; len];
        self.reader.read_exact(&mut buf).map_err(GeoTiffError::Io)?;
        Ok(buf)
    }

    #[allow(missing_docs)]
    pub fn read_u16_vec_at(&mut self, offset: u64, count: usize) -> Result<Vec<u16>> {
        let bytes = self.read_bytes_at(offset, count * 2)?;
        let bo = self.byte_order;
        Ok(bytes.chunks_exact(2).map(|c| bo.read_u16(c.try_into().unwrap())).collect())
    }

    #[allow(missing_docs)]
    pub fn read_f64_vec_at(&mut self, offset: u64, count: usize) -> Result<Vec<f64>> {
        let bytes = self.read_bytes_at(offset, count * 8)?;
        let bo = self.byte_order;
        Ok(bytes.chunks_exact(8).map(|c| bo.read_f64(c.try_into().unwrap())).collect())
    }

    #[allow(missing_docs)]
    pub fn inner_mut(&mut self) -> &mut R { &mut self.reader }
}
