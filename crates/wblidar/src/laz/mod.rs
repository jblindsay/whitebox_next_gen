//! LAZ (LASzip) reader and writer.
//!
//! LAZ stores points in variable-size *chunks* of typically 50 000 points.
//! Each chunk is independently DEFLATE-compressed with a per-field integer
//! delta predictor, making random access to any chunk practical.
//!
//! This implementation covers:
//! * LASzip chunk table (version 3 / 4 compatible).
//! * Delta-predictor for XYZ, intensity, flags, classification, scan angle,
//!   source ID, GPS time, and RGB.
//! * DEFLATE compression via `flate2`.

pub mod arithmetic_decoder;
pub mod arithmetic_encoder;
pub mod arithmetic_model;
pub mod chunk;
pub mod codec;
pub mod fields;
pub mod integer_codec;
pub mod laszip_chunk_table;
pub mod reader;
pub mod standard_point10;
pub mod standard_point10_write;
pub mod standard_point14;
pub mod writer;

pub use reader::LazReader;
pub use writer::{LazWriter, LazWriterConfig};

use crate::las::header::PointDataFormat;
use crate::las::vlr::{Vlr, VlrKey};

/// Well-known LASzip VLR user ID.
pub const LASZIP_USER_ID: &str = "laszip encoded";
/// Well-known LASzip VLR record ID.
pub const LASZIP_RECORD_ID: u16 = 22204;
/// Default LAZ chunk size (points per chunk).
pub const DEFAULT_CHUNK_SIZE: u32 = 50_000;

/// LASzip compressor organization declared by the LASzip VLR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaszipCompressorType {
    /// Uncompressed stream.
    None,
    /// Single chunk with all points.
    PointWise,
    /// Point-wise chunked stream.
    PointWiseChunked,
    /// Layered chunked stream.
    LayeredChunked,
    /// Unknown compressor type code.
    Unknown(u16),
}

impl LaszipCompressorType {
    fn from_u16(value: u16) -> Self {
        match value {
            0 => Self::None,
            1 => Self::PointWise,
            2 => Self::PointWiseChunked,
            3 => Self::LayeredChunked,
            other => Self::Unknown(other),
        }
    }
}

/// A single LASzip item declaration from the LASzip VLR payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaszipItemSpec {
    /// LASzip item type code.
    pub item_type: u16,
    /// Item byte size.
    pub item_size: u16,
    /// Item codec version.
    pub item_version: u16,
}

/// Parsed LASzip VLR metadata used by reader/writer dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaszipVlrInfo {
    /// Compressor organization.
    pub compressor: LaszipCompressorType,
    /// Coder ID (0 is arithmetic coder in LASzip).
    pub coder: u16,
    /// Points per fixed-size chunk.
    pub chunk_size: u32,
    /// Per-item LASzip codec layout.
    pub items: Vec<LaszipItemSpec>,
}

impl LaszipVlrInfo {
    /// True when this VLR declares arithmetic coding.
    pub fn uses_arithmetic_coder(&self) -> bool {
        self.coder == 0
    }

    /// Returns true when item list contains Point14 core item.
    pub fn has_point14_item(&self) -> bool {
        self.items.iter().any(|it| it.item_type == 10)
    }

    /// Returns true when item list contains Point10 core item.
    pub fn has_point10_item(&self) -> bool {
        self.items.iter().any(|it| it.item_type == 6)
    }

    /// Returns true when item list contains RGB14 item.
    pub fn has_rgb14_item(&self) -> bool {
        self.items.iter().any(|it| it.item_type == 11)
    }

    /// Returns true when item list contains RGBNIR14 item.
    pub fn has_rgbnir14_item(&self) -> bool {
        self.items.iter().any(|it| it.item_type == 12)
    }

    /// Returns true when item list carries NIR data for Point14-family formats.
    pub fn has_nir14_item(&self) -> bool {
        self.has_rgbnir14_item()
    }
}

/// Parse the LASzip VLR payload into structured metadata.
///
/// Returns `None` if no LASzip VLR is present or the payload is malformed.
pub fn parse_laszip_vlr(vlrs: &[crate::las::Vlr]) -> Option<LaszipVlrInfo> {
    let vlr = vlrs.iter().find(|v| {
        v.key.user_id == LASZIP_USER_ID && v.key.record_id == LASZIP_RECORD_ID
    })?;

    // Fixed LASzip VLR header size up to and including num_items.
    if vlr.data.len() < 34 {
        return None;
    }

    let compressor_raw = u16::from_le_bytes([vlr.data[0], vlr.data[1]]);
    let coder = u16::from_le_bytes([vlr.data[2], vlr.data[3]]);
    let chunk_size = u32::from_le_bytes([vlr.data[12], vlr.data[13], vlr.data[14], vlr.data[15]]);
    let num_items = u16::from_le_bytes([vlr.data[32], vlr.data[33]]) as usize;

    let expected_items_bytes = num_items.checked_mul(6)?;
    let expected_total = 34usize.checked_add(expected_items_bytes)?;
    if vlr.data.len() < expected_total {
        return None;
    }

    let mut items = Vec::with_capacity(num_items);
    let mut offset = 34usize;
    for _ in 0..num_items {
        let item_type = u16::from_le_bytes([vlr.data[offset], vlr.data[offset + 1]]);
        let item_size = u16::from_le_bytes([vlr.data[offset + 2], vlr.data[offset + 3]]);
        let item_version = u16::from_le_bytes([vlr.data[offset + 4], vlr.data[offset + 5]]);
        items.push(LaszipItemSpec {
            item_type,
            item_size,
            item_version,
        });
        offset += 6;
    }

    Some(LaszipVlrInfo {
        compressor: LaszipCompressorType::from_u16(compressor_raw),
        coder,
        chunk_size,
        items,
    })
}

/// Extract the chunk size written in a LASzip VLR payload.
///
/// The VLR layout (all little-endian) is:
/// - u16 compressor, u16 coder, u8 ver_major, u8 ver_minor, u16 ver_revision
/// - u32 options, **u32 chunk_size** (bytes 12–15), …
///
/// Returns `None` if no LASzip VLR is present or the payload is too short.
pub fn parse_vlr_chunk_size(vlrs: &[crate::las::Vlr]) -> Option<u32> {
    parse_laszip_vlr(vlrs).map(|v| v.chunk_size)
}

/// Build a LASzip VLR payload for a target LAS point format.
///
/// For LAS 1.4 point formats (PDRF 6/7/8), this emits a Point14-family
/// layered declaration (`compressor=3`) with v3 item specs.
/// Legacy point formats keep the existing Point10-family chunked declaration.
pub fn build_laszip_vlr_for_format(point_data_format: PointDataFormat, chunk_size: u32) -> Vlr {
    build_laszip_vlr_for_format_with_extra_bytes(point_data_format, chunk_size, 0)
}

/// Build a LASzip VLR payload for a target LAS point format, including
/// optional BYTE14 extra-bytes declaration for Point14-family formats.
pub fn build_laszip_vlr_for_format_with_extra_bytes(
    point_data_format: PointDataFormat,
    chunk_size: u32,
    extra_bytes_per_point: u16,
) -> Vlr {
    let mut data = Vec::with_capacity(64);

    let is_point14_family = point_data_format.is_v14() || point_data_format.is_v15();
    let compressor = if is_point14_family { 3u16 } else { 2u16 };

    // compressor
    data.extend_from_slice(&compressor.to_le_bytes());
    // coder (0 = arithmetic)
    data.extend_from_slice(&0u16.to_le_bytes());
    // LASzip version 2.2.0
    data.push(2);
    data.push(2);
    data.extend_from_slice(&0u16.to_le_bytes());
    // options
    data.extend_from_slice(&0u32.to_le_bytes());
    // chunk size
    data.extend_from_slice(&chunk_size.to_le_bytes());
    // special EVLRs
    data.extend_from_slice(&(-1i64).to_le_bytes());
    data.extend_from_slice(&(-1i64).to_le_bytes());

    let waveform_bytes_per_point = if point_data_format.has_waveform() {
        29u16
    } else {
        0u16
    };

    let mut items: Vec<(u16, u16, u16)> = if is_point14_family {
        vec![(10, 30, 3)]
    } else {
        vec![(6, 20, 2)]
    };

    if is_point14_family {
        if point_data_format.has_rgb() {
            if point_data_format.has_nir() {
                items.push((12, 8, 3));
            } else {
                items.push((11, 6, 3));
            }
        }
        let byte14_total = extra_bytes_per_point.saturating_add(waveform_bytes_per_point);
        if byte14_total > 0 {
            items.push((14, byte14_total, 3));
        }
    } else {
        if point_data_format.has_gps_time() {
            items.push((7, 8, 2));
        }
        if point_data_format.has_rgb() {
            items.push((8, 6, 2));
        }
        let byte_total = extra_bytes_per_point.saturating_add(waveform_bytes_per_point);
        if byte_total > 0 {
            items.push((0, byte_total, 2));
        }
    }

    data.extend_from_slice(&(items.len() as u16).to_le_bytes());
    for (item_type, item_size, item_version) in items {
        data.extend_from_slice(&item_type.to_le_bytes());
        data.extend_from_slice(&item_size.to_le_bytes());
        data.extend_from_slice(&item_version.to_le_bytes());
    }

    Vlr {
        key: VlrKey {
            user_id: LASZIP_USER_ID.to_owned(),
            record_id: LASZIP_RECORD_ID,
        },
        description: "LASzip by Martin Isenburg".to_owned(),
        data,
        extended: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_laszip_vlr_for_format,
        build_laszip_vlr_for_format_with_extra_bytes,
        parse_laszip_vlr,
        LaszipCompressorType,
    };
    use crate::las::header::PointDataFormat;
    use crate::las::vlr::{Vlr, VlrKey};
    use crate::laz::{LASZIP_RECORD_ID, LASZIP_USER_ID};

    fn build_test_laszip_vlr_data() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&2u16.to_le_bytes()); // PointWiseChunked
        data.extend_from_slice(&0u16.to_le_bytes()); // arithmetic coder
        data.push(2); // ver major
        data.push(2); // ver minor
        data.extend_from_slice(&0u16.to_le_bytes()); // ver revision
        data.extend_from_slice(&0u32.to_le_bytes()); // options
        data.extend_from_slice(&50_000u32.to_le_bytes()); // chunk_size
        data.extend_from_slice(&(-1i64).to_le_bytes()); // num special evlrs
        data.extend_from_slice(&(-1i64).to_le_bytes()); // special evlr offset
        data.extend_from_slice(&2u16.to_le_bytes()); // num items
        // Point14
        data.extend_from_slice(&10u16.to_le_bytes());
        data.extend_from_slice(&30u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        // RGB14
        data.extend_from_slice(&11u16.to_le_bytes());
        data.extend_from_slice(&6u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data
    }

    #[test]
    fn parses_laszip_vlr_core_fields() {
        let vlrs = vec![Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "LASzip by Martin Isenburg".to_string(),
            data: build_test_laszip_vlr_data(),
            extended: false,
        }];

        let parsed = parse_laszip_vlr(&vlrs).expect("expected parsed LASzip VLR");
        assert_eq!(parsed.compressor, LaszipCompressorType::PointWiseChunked);
        assert!(parsed.uses_arithmetic_coder());
        assert_eq!(parsed.chunk_size, 50_000);
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items[0].item_type, 10);
        assert_eq!(parsed.items[1].item_type, 11);
        assert!(!parsed.has_point10_item());
        assert!(parsed.has_point14_item());
        assert!(parsed.has_rgb14_item());
        assert!(!parsed.has_nir14_item());
    }

    #[test]
    fn rejects_truncated_laszip_vlr() {
        let vlrs = vec![Vlr {
            key: VlrKey {
                user_id: LASZIP_USER_ID.to_string(),
                record_id: LASZIP_RECORD_ID,
            },
            description: "truncated".to_string(),
            data: vec![0u8; 20],
            extended: false,
        }];
        assert!(parse_laszip_vlr(&vlrs).is_none());
    }

    #[test]
    fn builds_point14_family_laszip_vlr_for_pdrf8() {
        let vlr = build_laszip_vlr_for_format(PointDataFormat::Pdrf8, 50_000);
        let parsed = parse_laszip_vlr(&[vlr]).expect("expected parsed LASzip VLR");

        assert_eq!(parsed.compressor, LaszipCompressorType::LayeredChunked);
        assert!(parsed.has_point14_item());
        assert!(!parsed.has_rgb14_item());
        assert!(parsed.has_rgbnir14_item());
        assert!(parsed.has_nir14_item());
    }

    #[test]
    fn builds_point10_family_laszip_vlr_for_pdrf3_with_extra_bytes() {
        let vlr = build_laszip_vlr_for_format_with_extra_bytes(PointDataFormat::Pdrf3, 50_000, 2);
        let parsed = parse_laszip_vlr(&[vlr]).expect("expected parsed LASzip VLR");

        assert_eq!(parsed.compressor, LaszipCompressorType::PointWiseChunked);
        assert!(parsed.has_point10_item());
        assert!(parsed.items.iter().any(|item| item.item_type == 7 && item.item_size == 8));
        assert!(parsed.items.iter().any(|item| item.item_type == 8 && item.item_size == 6));
        assert!(parsed.items.iter().any(|item| item.item_type == 0 && item.item_size == 2));
    }
}
