//! Point format 0/1/2/3 raw layout helpers for LASzip legacy streams.

use crate::las::header::PointDataFormat;
use crate::point::{GpsTime, PointRecord, Rgb16};

/// Raw LAS <=1.3 point base (PDRF 0-family) layout.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RawPoint10 {
    /// Quantized X coordinate integer.
    pub xi: i32,
    /// Quantized Y coordinate integer.
    pub yi: i32,
    /// Quantized Z coordinate integer.
    pub zi: i32,
    /// Return intensity.
    pub intensity: u16,
    /// Legacy packed return/scan flags byte.
    pub return_flags: u8,
    /// Legacy packed classification flags byte.
    pub class_flags: u8,
    /// User data byte.
    pub user_data: u8,
    /// Scan angle rank (legacy i8 encoding).
    pub scan_angle_rank: i8,
    /// Point source identifier.
    pub point_source_id: u16,
    /// GPS time for formats with time.
    pub gps_time: Option<f64>,
    /// RGB channels for color formats.
    pub rgb: Option<Rgb16>,
}

impl RawPoint10 {
    /// Decode raw bytes for PDRF 0/1/2/3.
    pub fn from_bytes(raw: &[u8], format: PointDataFormat) -> Option<Self> {
        let expected = match format {
            PointDataFormat::Pdrf0 => 20,
            PointDataFormat::Pdrf1 => 28,
            PointDataFormat::Pdrf2 => 26,
            PointDataFormat::Pdrf3 => 34,
            _ => return None,
        };
        if raw.len() < expected {
            return None;
        }

        let mut out = Self {
            xi: i32::from_le_bytes(raw[0..4].try_into().ok()?),
            yi: i32::from_le_bytes(raw[4..8].try_into().ok()?),
            zi: i32::from_le_bytes(raw[8..12].try_into().ok()?),
            intensity: u16::from_le_bytes(raw[12..14].try_into().ok()?),
            return_flags: raw[14],
            class_flags: raw[15],
            user_data: raw[16],
            scan_angle_rank: raw[17] as i8,
            point_source_id: u16::from_le_bytes(raw[18..20].try_into().ok()?),
            gps_time: None,
            rgb: None,
        };

        match format {
            PointDataFormat::Pdrf1 => {
                out.gps_time = Some(f64::from_le_bytes(raw[20..28].try_into().ok()?));
            }
            PointDataFormat::Pdrf2 => {
                out.rgb = Some(Rgb16 {
                    red: u16::from_le_bytes(raw[20..22].try_into().ok()?),
                    green: u16::from_le_bytes(raw[22..24].try_into().ok()?),
                    blue: u16::from_le_bytes(raw[24..26].try_into().ok()?),
                });
            }
            PointDataFormat::Pdrf3 => {
                out.gps_time = Some(f64::from_le_bytes(raw[20..28].try_into().ok()?));
                out.rgb = Some(Rgb16 {
                    red: u16::from_le_bytes(raw[28..30].try_into().ok()?),
                    green: u16::from_le_bytes(raw[30..32].try_into().ok()?),
                    blue: u16::from_le_bytes(raw[32..34].try_into().ok()?),
                });
            }
            _ => {}
        }

        Some(out)
    }

    /// Convert to full `PointRecord` using LAS scale/offset.
    pub fn to_point_record(self, scales: [f64; 3], offsets: [f64; 3]) -> PointRecord {
        PointRecord {
            x: self.xi as f64 * scales[0] + offsets[0],
            y: self.yi as f64 * scales[1] + offsets[1],
            z: self.zi as f64 * scales[2] + offsets[2],
            intensity: self.intensity,
            color: self.rgb,
            classification: self.class_flags & 0x1F,
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            flags: (self.class_flags >> 5) & 0x07,
            return_number: self.return_flags & 0x07,
            number_of_returns: (self.return_flags >> 3) & 0x07,
            scan_direction_flag: (self.return_flags & 0x40) != 0,
            edge_of_flight_line: (self.return_flags & 0x80) != 0,
            scan_angle: self.scan_angle_rank as i16,
            gps_time: self.gps_time.map(GpsTime),
            ..PointRecord::default()
        }
    }

    /// Build raw representation from `PointRecord`.
    pub fn from_point_record(
        p: PointRecord,
        format: PointDataFormat,
        scales: [f64; 3],
        offsets: [f64; 3],
    ) -> Option<Self> {
        match format {
            PointDataFormat::Pdrf0
            | PointDataFormat::Pdrf1
            | PointDataFormat::Pdrf2
            | PointDataFormat::Pdrf3 => {}
            _ => return None,
        }

        let return_flags = (p.return_number & 0x07)
            | ((p.number_of_returns & 0x07) << 3)
            | ((u8::from(p.scan_direction_flag)) << 6)
            | ((u8::from(p.edge_of_flight_line)) << 7);
        let class_flags = (p.classification & 0x1F) | ((p.flags & 0x07) << 5);

        let out = Self {
            xi: ((p.x - offsets[0]) / scales[0]).round() as i32,
            yi: ((p.y - offsets[1]) / scales[1]).round() as i32,
            zi: ((p.z - offsets[2]) / scales[2]).round() as i32,
            intensity: p.intensity,
            return_flags,
            class_flags,
            user_data: p.user_data,
            scan_angle_rank: p.scan_angle as i8,
            point_source_id: p.point_source_id,
            gps_time: p.gps_time.map(|v| v.0),
            rgb: p.color,
        };

        if matches!(format, PointDataFormat::Pdrf1 | PointDataFormat::Pdrf3) && out.gps_time.is_none() {
            return None;
        }
        if matches!(format, PointDataFormat::Pdrf2 | PointDataFormat::Pdrf3) && out.rgb.is_none() {
            return None;
        }

        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::las::header::PointDataFormat;
    use crate::point::{GpsTime, PointRecord, Rgb16};

    use super::RawPoint10;

    #[test]
    fn raw_point10_pdrf3_round_trip() {
        let p = PointRecord {
            x: 250.0,
            y: 100.0,
            z: 12.0,
            intensity: 512,
            color: Some(Rgb16 { red: 12, green: 34, blue: 56 }),
            classification: 2,
            user_data: 7,
            point_source_id: 9,
            flags: 0x03,
            return_number: 1,
            number_of_returns: 2,
            scan_direction_flag: true,
            edge_of_flight_line: false,
            scan_angle: -12,
            gps_time: Some(GpsTime(42.5)),
            ..PointRecord::default()
        };

        let scales = [0.01, 0.01, 0.01];
        let offsets = [0.0, 0.0, 0.0];

        let raw = RawPoint10::from_point_record(p, PointDataFormat::Pdrf3, scales, offsets)
            .expect("raw conversion should succeed");
        let bytes = {
            // reuse parser layout check by reconstructing manually
            let mut b = vec![0u8; 34];
            b[0..4].copy_from_slice(&raw.xi.to_le_bytes());
            b[4..8].copy_from_slice(&raw.yi.to_le_bytes());
            b[8..12].copy_from_slice(&raw.zi.to_le_bytes());
            b[12..14].copy_from_slice(&raw.intensity.to_le_bytes());
            b[14] = raw.return_flags;
            b[15] = raw.class_flags;
            b[16] = raw.user_data;
            b[17] = raw.scan_angle_rank as u8;
            b[18..20].copy_from_slice(&raw.point_source_id.to_le_bytes());
            b[20..28].copy_from_slice(&raw.gps_time.expect("gps").to_le_bytes());
            let rgb = raw.rgb.expect("rgb");
            b[28..30].copy_from_slice(&rgb.red.to_le_bytes());
            b[30..32].copy_from_slice(&rgb.green.to_le_bytes());
            b[32..34].copy_from_slice(&rgb.blue.to_le_bytes());
            b
        };

        let decoded_raw = RawPoint10::from_bytes(&bytes, PointDataFormat::Pdrf3)
            .expect("raw bytes should parse");
        let decoded = decoded_raw.to_point_record(scales, offsets);

        assert_eq!(decoded.intensity, p.intensity);
        assert_eq!(decoded.classification, p.classification);
        assert_eq!(decoded.color.expect("color").green, 34);
        assert_eq!(decoded.gps_time.expect("gps").0, 42.5);
        assert_eq!(decoded.return_number, p.return_number);
        assert_eq!(decoded.number_of_returns, p.number_of_returns);
    }
}
