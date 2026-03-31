//! LAS public file header for versions 1.1 – 1.5.

use std::io::{Read, Seek, SeekFrom, Write};
use crate::io::le;
use crate::{Error, Result};

/// The four-byte file signature of every LAS file.
pub const SIGNATURE: &[u8; 4] = b"LASF";

/// Global encoding bitmask (LAS 1.2+).
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalEncoding(pub u16);

impl GlobalEncoding {
    /// GPS time is Adjusted Standard GPS time when this bit is set.
    pub const GPS_TIME_TYPE: u16 = 0x0001;
    /// LAZ / DEFLATE waveform data is stored internally.
    pub const WAVEFORM_DATA_INTERNAL: u16 = 0x0002;
    /// LAZ / DEFLATE waveform data is stored externally.
    pub const WAVEFORM_DATA_EXTERNAL: u16 = 0x0004;
    /// Return numbers are synthetized.
    pub const SYNTHETIC_RETURN_NUMBERS: u16 = 0x0008;
    /// Coordinate Reference System is stored as WKT (LAS 1.4).
    pub const WKT: u16 = 0x0010;

    /// Test a bit flag.
    pub fn is_set(self, flag: u16) -> bool { self.0 & flag != 0 }
}

/// Point-data record format identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PointDataFormat {
    /// PDRF 0 – X/Y/Z + intensity + return/classification/scan.
    Pdrf0 = 0,
    /// PDRF 1 – PDRF 0 + GPS time.
    Pdrf1 = 1,
    /// PDRF 2 – PDRF 0 + RGB.
    Pdrf2 = 2,
    /// PDRF 3 – PDRF 0 + GPS time + RGB.
    Pdrf3 = 3,
    /// PDRF 4 – PDRF 1 + waveform.
    Pdrf4 = 4,
    /// PDRF 5 – PDRF 3 + waveform.
    Pdrf5 = 5,
    /// PDRF 6 – LAS 1.4 base (X/Y/Z + return bits + classification + scan angle + GPS time).
    Pdrf6 = 6,
    /// PDRF 7 – PDRF 6 + RGB.
    Pdrf7 = 7,
    /// PDRF 8 – PDRF 7 + NIR.
    Pdrf8 = 8,
    /// PDRF 9 – PDRF 6 + waveform.
    Pdrf9 = 9,
    /// PDRF 10 – PDRF 7 + waveform.
    Pdrf10 = 10,
    /// PDRF 11 – LAS 1.5 (PDRF 6 base, no extra fields). Core size: 30 bytes.
    Pdrf11 = 11,
    /// PDRF 12 – LAS 1.5 (PDRF 6 + 16-bit RGB). Core size: 36 bytes.
    Pdrf12 = 12,
    /// PDRF 13 – LAS 1.5 (PDRF 6 + 16-bit RGB + 16-bit NIR + ThermalRGB). Core size: 46 bytes.
    Pdrf13 = 13,
    /// PDRF 14 – LAS 1.5 (PDRF 6 + 16-bit RGB + waveform). Core size: 65 bytes.
    Pdrf14 = 14,
    /// PDRF 15 – LAS 1.5 (PDRF 6 + 16-bit RGB + 16-bit NIR + waveform + ThermalRGB). Core size: 75 bytes.
    Pdrf15 = 15,
}

impl PointDataFormat {
    /// Parse from a raw `u8`.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Pdrf0),
            1 => Some(Self::Pdrf1),
            2 => Some(Self::Pdrf2),
            3 => Some(Self::Pdrf3),
            4 => Some(Self::Pdrf4),
            5 => Some(Self::Pdrf5),
            6 => Some(Self::Pdrf6),
            7 => Some(Self::Pdrf7),
            8 => Some(Self::Pdrf8),
            9 => Some(Self::Pdrf9),
            10 => Some(Self::Pdrf10),
            11 => Some(Self::Pdrf11),
            12 => Some(Self::Pdrf12),
            13 => Some(Self::Pdrf13),
            14 => Some(Self::Pdrf14),
            15 => Some(Self::Pdrf15),
            _ => None,
        }
    }

    /// Core byte size of a point record in this format (without extra bytes).
    pub fn core_size(self) -> u16 {
        match self {
            Self::Pdrf0 => 20,
            Self::Pdrf1 => 28,
            Self::Pdrf2 => 26,
            Self::Pdrf3 => 34,
            Self::Pdrf4 => 57,
            Self::Pdrf5 => 63,
            Self::Pdrf6 => 30,
            Self::Pdrf7 => 36,
            Self::Pdrf8 => 38,
            Self::Pdrf9 => 59,
            Self::Pdrf10 => 67,
            Self::Pdrf11 => 30,  // LAS 1.5: PDRF 6 base (no extras)
            Self::Pdrf12 => 36,  // LAS 1.5: 30 base + 6 RGB
            Self::Pdrf13 => 46,  // LAS 1.5: 30 base + 6 RGB + 2 NIR + 8 ThermalRGB
            Self::Pdrf14 => 65,  // LAS 1.5: 30 base + 6 RGB + 29 waveform
            Self::Pdrf15 => 75,  // LAS 1.5: 30 base + 6 RGB + 2 NIR + 29 waveform + 8 ThermalRGB
        }
    }

    /// Whether this PDRF carries GPS time.
    pub fn has_gps_time(self) -> bool {
        !matches!(self, Self::Pdrf0 | Self::Pdrf2)
    }

    /// Whether this PDRF carries RGB colour.
    pub fn has_rgb(self) -> bool {
        matches!(self, Self::Pdrf2 | Self::Pdrf3 | Self::Pdrf5 | Self::Pdrf7 | Self::Pdrf8 | Self::Pdrf10
                      | Self::Pdrf12 | Self::Pdrf13 | Self::Pdrf14 | Self::Pdrf15)
    }

    /// Whether this PDRF carries NIR.
    pub fn has_nir(self) -> bool { matches!(self, Self::Pdrf8 | Self::Pdrf13 | Self::Pdrf15) }

    /// Whether this PDRF carries waveform data.
    pub fn has_waveform(self) -> bool {
        matches!(self, Self::Pdrf4 | Self::Pdrf5 | Self::Pdrf9 | Self::Pdrf10 | Self::Pdrf14 | Self::Pdrf15)
    }

    /// Whether this is a LAS 1.4 PDRF (6–10).
    pub fn is_v14(self) -> bool { (6..=10).contains(&(self as u8)) }

    /// Whether this is a LAS 1.5 PDRF (11–15).
    pub fn is_v15(self) -> bool { (11..=15).contains(&(self as u8)) }

    /// Whether this PDRF uses extended 16-bit RGB (LAS 1.5 PDRFs 12–15).
    pub fn has_extended_rgb(self) -> bool {
        matches!(self, Self::Pdrf12 | Self::Pdrf13 | Self::Pdrf14 | Self::Pdrf15)
    }
}

/// The full LAS Public File Header (all versions).
#[derive(Debug, Clone)]
pub struct LasHeader {
    // Common (all versions)
    /// Major version (always 1).
    pub version_major: u8,
    /// Minor version (1, 2, 3, or 4).
    pub version_minor: u8,
    /// System identifier (32 bytes).
    pub system_identifier: String,
    /// Generating software (32 bytes).
    pub generating_software: String,
    /// File creation day of year.
    pub file_creation_day: u16,
    /// File creation year.
    pub file_creation_year: u16,
    /// Size of the public file header in bytes.
    pub header_size: u16,
    /// Byte offset from the start of the file to the first point record.
    pub offset_to_point_data: u32,
    /// Number of VLRs.
    pub number_of_vlrs: u32,
    /// Point-data record format.
    pub point_data_format: PointDataFormat,
    /// Point-data record length in bytes (may include extra bytes).
    pub point_data_record_length: u16,
    /// Global encoding flags (LAS 1.2+).
    pub global_encoding: GlobalEncoding,
    /// Project ID GUID (16 bytes).
    pub project_id: [u8; 16],

    // Scale / offset
    /// X scale factor.
    pub x_scale: f64,
    /// Y scale factor.
    pub y_scale: f64,
    /// Z scale factor.
    pub z_scale: f64,
    /// X offset.
    pub x_offset: f64,
    /// Y offset.
    pub y_offset: f64,
    /// Z offset.
    pub z_offset: f64,

    // Bounding box
    /// Maximum X.
    pub max_x: f64,
    /// Minimum X.
    pub min_x: f64,
    /// Maximum Y.
    pub max_y: f64,
    /// Minimum Y.
    pub min_y: f64,
    /// Maximum Z.
    pub max_z: f64,
    /// Minimum Z.
    pub min_z: f64,

    // Point counts
    /// Legacy 32-bit total point count (LAS 1.1 – 1.3; 0 for 1.4 if >u32::MAX).
    pub legacy_point_count: u32,
    /// Legacy 32-bit per-return counts (5 entries).
    pub legacy_point_count_by_return: [u32; 5],

    // LAS 1.3 / 1.4
    /// Byte offset to waveform data (LAS 1.3+).
    pub waveform_data_packet_offset: Option<u64>,
    /// Byte offset to first EVLR (LAS 1.4).
    pub start_of_first_evlr: Option<u64>,
    /// Number of EVLRs (LAS 1.4).
    pub number_of_evlrs: Option<u32>,
    /// 64-bit total point count (LAS 1.4).
    pub point_count_64: Option<u64>,
    /// 64-bit per-return counts (15 entries, LAS 1.4).
    pub point_count_by_return_64: Option<[u64; 15]>,

    /// Number of extra bytes appended to each point record.
    pub extra_bytes_count: u16,
}

impl LasHeader {
    /// Read the public file header from a seekable reader.
    pub fn read<R: Read + Seek>(r: &mut R) -> Result<Self> {
        r.seek(SeekFrom::Start(0))?;

        // Signature
        let mut sig = [0u8; 4];
        r.read_exact(&mut sig)?;
        if &sig != SIGNATURE {
            return Err(Error::InvalidSignature { format: "LAS", found: sig.to_vec() });
        }

        let file_source_id = le::read_u16(r)?;
        let _ = file_source_id;
        let global_encoding = GlobalEncoding(le::read_u16(r)?);

        let mut project_id = [0u8; 16];
        r.read_exact(&mut project_id)?;

        let version_major = le::read_u8(r)?;
        let version_minor = le::read_u8(r)?;

        let mut sys_id = [0u8; 32];
        r.read_exact(&mut sys_id)?;
        let mut gen_sw = [0u8; 32];
        r.read_exact(&mut gen_sw)?;

        let file_creation_day  = le::read_u16(r)?;
        let file_creation_year = le::read_u16(r)?;
        let header_size        = le::read_u16(r)?;
        let offset_to_point_data = le::read_u32(r)?;
        let number_of_vlrs     = le::read_u32(r)?;
        let raw_pdrf           = le::read_u8(r)?;

        let point_data_format = PointDataFormat::from_u8(raw_pdrf & 0x7F)
            .ok_or_else(|| Error::InvalidValue {
                field: "point_data_format_id",
                detail: format!("unknown PDRF {raw_pdrf}"),
            })?;

        let point_data_record_length = le::read_u16(r)?;
        let legacy_point_count       = le::read_u32(r)?;
        let mut legacy_returns       = [0u32; 5];
        for v in &mut legacy_returns { *v = le::read_u32(r)?; }

        // Scale and offset
        let x_scale = le::read_f64(r)?;
        let y_scale = le::read_f64(r)?;
        let z_scale = le::read_f64(r)?;
        let x_offset = le::read_f64(r)?;
        let y_offset = le::read_f64(r)?;
        let z_offset = le::read_f64(r)?;

        let max_x = le::read_f64(r)?;
        let min_x = le::read_f64(r)?;
        let max_y = le::read_f64(r)?;
        let min_y = le::read_f64(r)?;
        let max_z = le::read_f64(r)?;
        let min_z = le::read_f64(r)?;

        // Version-specific fields
        let (waveform_offset, evlr_start, evlr_count, pc64, pcr64) =
            if version_minor >= 3 && version_minor <= 5 {
                let wf = le::read_u64(r)?;

                let (evlr_s, evlr_c, pc, pcr) = if version_minor >= 4 {
                    let es = le::read_u64(r)?;
                    let ec = le::read_u32(r)?;
                    let pc = le::read_u64(r)?;
                    let mut ret = [0u64; 15];
                    for v in &mut ret { *v = le::read_u64(r)?; }
                    (Some(es), Some(ec), Some(pc), Some(ret))
                } else {
                    (None, None, None, None)
                };
                (Some(wf), evlr_s, evlr_c, pc, pcr)
            } else {
                (None, None, None, None, None)
            };

        let extra_bytes_count = point_data_record_length
            .saturating_sub(point_data_format.core_size());

        Ok(LasHeader {
            version_major, version_minor,
            system_identifier: null_padded_str(&sys_id),
            generating_software: null_padded_str(&gen_sw),
            file_creation_day, file_creation_year,
            header_size,
            offset_to_point_data, number_of_vlrs,
            point_data_format,
            point_data_record_length,
            global_encoding, project_id,
            x_scale, y_scale, z_scale,
            x_offset, y_offset, z_offset,
            max_x, min_x, max_y, min_y, max_z, min_z,
            legacy_point_count,
            legacy_point_count_by_return: legacy_returns,
            waveform_data_packet_offset: waveform_offset,
            start_of_first_evlr: evlr_start,
            number_of_evlrs: evlr_count,
            point_count_64: pc64,
            point_count_by_return_64: pcr64,
            extra_bytes_count,
        })
    }

    /// Total point count: prefers 64-bit field when available.
    pub fn point_count(&self) -> u64 {
        self.point_count_64.unwrap_or(u64::from(self.legacy_point_count))
    }

    /// Write the LAS public file header (375 bytes for versions 1.0 – 1.5).
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(SIGNATURE)?;
        le::write_u16(w, 0)?; // file_source_id
        le::write_u16(w, self.global_encoding.0)?;
        w.write_all(&self.project_id)?;
        le::write_u8(w, self.version_major)?;
        le::write_u8(w, self.version_minor)?;
        write_fixed_str(w, &self.system_identifier, 32)?;
        write_fixed_str(w, &self.generating_software, 32)?;
        le::write_u16(w, self.file_creation_day)?;
        le::write_u16(w, self.file_creation_year)?;
        le::write_u16(w, 375)?; // LAS 1.4 header size
        le::write_u32(w, self.offset_to_point_data)?;
        le::write_u32(w, self.number_of_vlrs)?;
        le::write_u8(w, self.point_data_format as u8)?;
        le::write_u16(w, self.point_data_record_length)?;
        le::write_u32(w, self.legacy_point_count.min(u32::MAX))?;
        for v in &self.legacy_point_count_by_return { le::write_u32(w, *v)?; }
        le::write_f64(w, self.x_scale)?;
        le::write_f64(w, self.y_scale)?;
        le::write_f64(w, self.z_scale)?;
        le::write_f64(w, self.x_offset)?;
        le::write_f64(w, self.y_offset)?;
        le::write_f64(w, self.z_offset)?;
        le::write_f64(w, self.max_x)?;
        le::write_f64(w, self.min_x)?;
        le::write_f64(w, self.max_y)?;
        le::write_f64(w, self.min_y)?;
        le::write_f64(w, self.max_z)?;
        le::write_f64(w, self.min_z)?;
        // LAS 1.3 waveform offset
        le::write_u64(w, self.waveform_data_packet_offset.unwrap_or(0))?;
        // LAS 1.4 EVLRs
        le::write_u64(w, self.start_of_first_evlr.unwrap_or(0))?;
        le::write_u32(w, self.number_of_evlrs.unwrap_or(0))?;
        // LAS 1.4 point counts
        le::write_u64(w, self.point_count_64.unwrap_or(u64::from(self.legacy_point_count)))?;
        let pcr = self.point_count_by_return_64.unwrap_or([0u64; 15]);
        for v in &pcr { le::write_u64(w, *v)?; }
        Ok(())
    }
}

fn null_padded_str(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn write_fixed_str<W: Write>(w: &mut W, s: &str, n: usize) -> Result<()> {
    let mut buf = vec![0u8; n];
    let bytes = s.as_bytes();
    let len = bytes.len().min(n);
    buf[..len].copy_from_slice(&bytes[..len]);
    Ok(w.write_all(&buf)?)
}
