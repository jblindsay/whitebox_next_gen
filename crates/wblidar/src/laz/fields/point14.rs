//! Point format 6/7/8 raw layout helpers for LASzip-based streams.

use crate::las::header::PointDataFormat;
use crate::point::{GpsTime, PointRecord, Rgb16};

/// Raw LAS 1.4 point base (PDRF 6) layout.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RawPoint14 {
    /// Quantized X coordinate integer.
    pub xi: i32,
    /// Quantized Y coordinate integer.
    pub yi: i32,
    /// Quantized Z coordinate integer.
    pub zi: i32,
    /// Return intensity.
    pub intensity: u16,
    /// Packed return-number and number-of-returns byte.
    pub return_byte: u8,
    /// Packed classification/scanner flags byte.
    pub flags_byte: u8,
    /// ASPRS classification code.
    pub classification: u8,
    /// User data byte.
    pub user_data: u8,
    /// Scan angle value.
    pub scan_angle: i16,
    /// Point source identifier.
    pub point_source_id: u16,
    /// GPS time value.
    pub gps_time: f64,
    /// RGB channels for PDRF 7/8.
    pub rgb: Option<Rgb16>,
    /// NIR channel for PDRF 8.
    pub nir: Option<u16>,
}

impl RawPoint14 {
    /// Decode raw bytes for PDRF 6/7/8 (and v1.5 equivalents 11/12/13) into a raw point struct.
    pub fn from_bytes(raw: &[u8], format: PointDataFormat) -> Option<Self> {
        let expected = match format {
            PointDataFormat::Pdrf6 | PointDataFormat::Pdrf11 => 30,
            PointDataFormat::Pdrf7 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf14 => 36,
            PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15 => 38,
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
            return_byte: raw[14],
            flags_byte: raw[15],
            classification: raw[16],
            user_data: raw[17],
            scan_angle: i16::from_le_bytes(raw[18..20].try_into().ok()?),
            point_source_id: u16::from_le_bytes(raw[20..22].try_into().ok()?),
            gps_time: f64::from_le_bytes(raw[22..30].try_into().ok()?),
            rgb: None,
            nir: None,
        };

        if matches!(format, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf14 | PointDataFormat::Pdrf15) {
            out.rgb = Some(Rgb16 {
                red: u16::from_le_bytes(raw[30..32].try_into().ok()?),
                green: u16::from_le_bytes(raw[32..34].try_into().ok()?),
                blue: u16::from_le_bytes(raw[34..36].try_into().ok()?),
            });
        }
        if matches!(format, PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15) {
            out.nir = Some(u16::from_le_bytes(raw[36..38].try_into().ok()?));
        }

        Some(out)
    }

    /// Encode this raw point to bytes for PDRF 6/7/8 (and v1.5 equivalents 11/12/13).
    pub fn to_bytes(&self, format: PointDataFormat) -> Option<Vec<u8>> {
        let expected = match format {
            PointDataFormat::Pdrf6 | PointDataFormat::Pdrf11 => 30,
            PointDataFormat::Pdrf7 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf14 => 36,
            PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15 => 38,
            _ => return None,
        };

        let mut out = vec![0u8; expected];
        out[0..4].copy_from_slice(&self.xi.to_le_bytes());
        out[4..8].copy_from_slice(&self.yi.to_le_bytes());
        out[8..12].copy_from_slice(&self.zi.to_le_bytes());
        out[12..14].copy_from_slice(&self.intensity.to_le_bytes());
        out[14] = self.return_byte;
        out[15] = self.flags_byte;
        out[16] = self.classification;
        out[17] = self.user_data;
        out[18..20].copy_from_slice(&self.scan_angle.to_le_bytes());
        out[20..22].copy_from_slice(&self.point_source_id.to_le_bytes());
        out[22..30].copy_from_slice(&self.gps_time.to_le_bytes());

        if matches!(format, PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 | PointDataFormat::Pdrf12 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf14 | PointDataFormat::Pdrf15) {
            let rgb = self.rgb?;
            out[30..32].copy_from_slice(&rgb.red.to_le_bytes());
            out[32..34].copy_from_slice(&rgb.green.to_le_bytes());
            out[34..36].copy_from_slice(&rgb.blue.to_le_bytes());
        }
        if matches!(format, PointDataFormat::Pdrf8 | PointDataFormat::Pdrf13 | PointDataFormat::Pdrf15) {
            out[36..38].copy_from_slice(&self.nir?.to_le_bytes());
        }

        Some(out)
    }

    /// Convert to full `PointRecord` using LAS scale/offset.
    pub fn to_point_record(self, scales: [f64; 3], offsets: [f64; 3]) -> PointRecord {
        let return_number = self.return_byte & 0x0F;
        let number_of_returns = (self.return_byte >> 4) & 0x0F;

        PointRecord {
            x: self.xi as f64 * scales[0] + offsets[0],
            y: self.yi as f64 * scales[1] + offsets[1],
            z: self.zi as f64 * scales[2] + offsets[2],
            intensity: self.intensity,
            color: self.rgb,
            nir: self.nir,
            classification: self.classification,
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            flags: self.flags_byte & 0x1F,
            return_number,
            number_of_returns,
            scan_direction_flag: (self.flags_byte & 0x40) != 0,
            edge_of_flight_line: (self.flags_byte & 0x80) != 0,
            scan_angle: self.scan_angle,
            gps_time: Some(GpsTime(self.gps_time)),
            ..PointRecord::default()
        }
    }

    /// Build raw representation from a scaled `PointRecord`.
    pub fn from_point_record(p: PointRecord, format: PointDataFormat, scales: [f64; 3], offsets: [f64; 3]) -> Option<Self> {
        match format {
            PointDataFormat::Pdrf6 | PointDataFormat::Pdrf7 | PointDataFormat::Pdrf8 => {}
            _ => return None,
        }

        let xi = ((p.x - offsets[0]) / scales[0]).round() as i32;
        let yi = ((p.y - offsets[1]) / scales[1]).round() as i32;
        let zi = ((p.z - offsets[2]) / scales[2]).round() as i32;

        let mut flags = p.flags & 0x3F;
        if p.scan_direction_flag {
            flags |= 0x40;
        }
        if p.edge_of_flight_line {
            flags |= 0x80;
        }

        let out = Self {
            xi,
            yi,
            zi,
            intensity: p.intensity,
            return_byte: (p.return_number & 0x0F) | ((p.number_of_returns & 0x0F) << 4),
            flags_byte: flags,
            classification: p.classification,
            user_data: p.user_data,
            scan_angle: p.scan_angle,
            point_source_id: p.point_source_id,
            gps_time: p.gps_time.map_or(0.0, |v| v.0),
            rgb: p.color,
            nir: p.nir,
        };

        match format {
            PointDataFormat::Pdrf6 => {
                if out.rgb.is_some() || out.nir.is_some() {
                    return None;
                }
            }
            PointDataFormat::Pdrf7 => {
                if out.rgb.is_none() || out.nir.is_some() {
                    return None;
                }
            }
            PointDataFormat::Pdrf8 => {
                if out.rgb.is_none() || out.nir.is_none() {
                    return None;
                }
            }
            _ => return None,
        }

        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::las::header::PointDataFormat;
    use crate::point::{GpsTime, PointRecord, Rgb16};

    use super::RawPoint14;

    #[test]
    fn raw_point14_pdrf8_round_trip() {
        let p = PointRecord {
            x: 100.25,
            y: -22.75,
            z: 10.5,
            intensity: 1234,
            color: Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }),
            nir: Some(4096),
            classification: 2,
            user_data: 9,
            point_source_id: 77,
            flags: 0x12,
            return_number: 2,
            number_of_returns: 3,
            scan_direction_flag: true,
            edge_of_flight_line: false,
            scan_angle: -45,
            gps_time: Some(GpsTime(12345.0)),
            ..PointRecord::default()
        };

        let scales = [0.01, 0.01, 0.01];
        let offsets = [0.0, 0.0, 0.0];

        let raw = RawPoint14::from_point_record(p, PointDataFormat::Pdrf8, scales, offsets)
            .expect("raw conversion should succeed");
        let bytes = raw.to_bytes(PointDataFormat::Pdrf8).expect("bytes should serialize");
        let decoded_raw = RawPoint14::from_bytes(&bytes, PointDataFormat::Pdrf8)
            .expect("raw bytes should parse");
        let decoded = decoded_raw.to_point_record(scales, offsets);

        assert_eq!(decoded.intensity, p.intensity);
        assert_eq!(decoded.color.unwrap().red, 1000);
        assert_eq!(decoded.nir, Some(4096));
        assert_eq!(decoded.classification, p.classification);
        assert_eq!(decoded.return_number, p.return_number);
        assert_eq!(decoded.number_of_returns, p.number_of_returns);
        assert!((decoded.x - p.x).abs() <= 0.01);
        assert!((decoded.y - p.y).abs() <= 0.01);
        assert!((decoded.z - p.z).abs() <= 0.01);
    }
}
