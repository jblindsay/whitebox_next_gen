//! LAS sequential point reader (all versions and all PDRFs).

use std::io::{Read, Seek, SeekFrom};
use wide::f64x4;
use crate::crs::{epsg_from_wkt, Crs};
use crate::las::header::PointDataFormat;
use crate::las::vlr::{find_epsg, find_ogc_wkt, Vlr};
use crate::las::LasHeader;
use crate::point::{GpsTime, PointRecord, Rgb16, ThermalRgb, WaveformPacket};
use crate::{Error, Result};
use crate::io::PointReader;

/// A streaming LAS reader.
pub struct LasReader<R: Read + Seek> {
    inner: R,
    header: LasHeader,
    vlrs: Vec<Vlr>,
    crs: Option<Crs>,
    points_read: u64,
    raw_buf: Vec<u8>,
}

impl<R: Read + Seek> LasReader<R> {
    /// Open a reader, parse the header and all VLRs, then seek to the first
    /// point record.
    pub fn new(mut inner: R) -> Result<Self> {
        let header = LasHeader::read(&mut inner)?;

        // Seek past the fixed header to read VLRs
        inner.seek(SeekFrom::Start(u64::from(header.header_size)))?;
        let mut vlrs = Vec::with_capacity(header.number_of_vlrs as usize);
        for _ in 0..header.number_of_vlrs {
            vlrs.push(Vlr::read_vlr(&mut inner)?);
        }
        let crs = infer_crs_from_vlrs(&vlrs);

        // Seek to the first point
        inner.seek(SeekFrom::Start(u64::from(header.offset_to_point_data)))?;

        let raw_buf = vec![0u8; header.point_data_record_length as usize];
        Ok(Self { inner, header, vlrs, crs, points_read: 0, raw_buf })
    }

    /// Borrow the parsed header.
    pub fn header(&self) -> &LasHeader { &self.header }

    /// Borrow the VLR list.
    pub fn vlrs(&self) -> &[Vlr] { &self.vlrs }

    /// Borrow detected CRS metadata from LAS projection VLRs (if present).
    pub fn crs(&self) -> Option<&Crs> { self.crs.as_ref() }

    /// Mutable access to the underlying reader (required by `LazReader`).
    pub fn inner_mut(&mut self) -> &mut R { &mut self.inner }

    /// Byte offset of the first point record (or chunk table for LAZ).
    pub fn offset_to_point_data(&self) -> u64 { u64::from(self.header.offset_to_point_data) }
}

fn infer_crs_from_vlrs(vlrs: &[Vlr]) -> Option<Crs> {
    let wkt = find_ogc_wkt(vlrs);
    let epsg = find_epsg(vlrs).or_else(|| wkt.as_deref().and_then(epsg_from_wkt));
    if epsg.is_none() && wkt.is_none() {
        None
    } else {
        Some(Crs { epsg, wkt })
    }
}

impl<R: Read + Seek> PointReader for LasReader<R> {
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool> {
        let total = self.header.point_count();
        if total > 0 && self.points_read >= total {
            return Ok(false);
        }

        match self.inner.read_exact(&mut self.raw_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(false),
            Err(e) => return Err(Error::Io(e)),
        }

        decode_point(&self.raw_buf, out, &self.header)?;
        self.points_read += 1;
        Ok(true)
    }

    fn point_count(&self) -> Option<u64> { Some(self.header.point_count()) }
}

// ── Point decode dispatch ────────────────────────────────────────────────────

fn decode_point(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    *out = PointRecord::default();
    match hdr.point_data_format {
        PointDataFormat::Pdrf0 => decode_pdrf0(buf, out, hdr),
        PointDataFormat::Pdrf1 => decode_pdrf1(buf, out, hdr),
        PointDataFormat::Pdrf2 => decode_pdrf2(buf, out, hdr),
        PointDataFormat::Pdrf3 => decode_pdrf3(buf, out, hdr),
        PointDataFormat::Pdrf4 => decode_pdrf4(buf, out, hdr),
        PointDataFormat::Pdrf5 => decode_pdrf5(buf, out, hdr),
        PointDataFormat::Pdrf6 => decode_pdrf6(buf, out, hdr),
        PointDataFormat::Pdrf7 => decode_pdrf7(buf, out, hdr),
        PointDataFormat::Pdrf8 => decode_pdrf8(buf, out, hdr),
        PointDataFormat::Pdrf9 => decode_pdrf9(buf, out, hdr),
        PointDataFormat::Pdrf10 => decode_pdrf10(buf, out, hdr),
        PointDataFormat::Pdrf11 => decode_pdrf11(buf, out, hdr),
        PointDataFormat::Pdrf12 => decode_pdrf12(buf, out, hdr),
        PointDataFormat::Pdrf13 => decode_pdrf13(buf, out, hdr),
        PointDataFormat::Pdrf14 => decode_pdrf14(buf, out, hdr),
        PointDataFormat::Pdrf15 => decode_pdrf15(buf, out, hdr),
    }
}

// ── Helper: read XYZ from a little-endian buffer starting at offset 0 ────────

#[inline]
fn decode_xyz(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) {
    let xi = i32::from_le_bytes(buf[0..4].try_into().unwrap());
    let yi = i32::from_le_bytes(buf[4..8].try_into().unwrap());
    let zi = i32::from_le_bytes(buf[8..12].try_into().unwrap());
    // Compute x/y/z scale+offset in one f64x4 SIMD op (4th lane is unused).
    let ints    = f64x4::new([xi as f64, yi as f64, zi as f64, 0.0]);
    let scales  = f64x4::new([hdr.x_scale, hdr.y_scale, hdr.z_scale, 1.0]);
    let offsets = f64x4::new([hdr.x_offset, hdr.y_offset, hdr.z_offset, 0.0]);
    let result: [f64; 4] = (ints * scales + offsets).into();
    out.x = result[0];
    out.y = result[1];
    out.z = result[2];
    out.intensity = u16::from_le_bytes(buf[12..14].try_into().unwrap());
}

/// LAS 1.0–1.3 return/classification byte pack (1 byte flags + 1 byte class).
#[inline]
fn decode_flags_v13(buf: &[u8], off: usize, out: &mut PointRecord) {
    let flags = buf[off];
    out.return_number        = flags & 0x07;
    out.number_of_returns    = (flags >> 3) & 0x07;
    out.scan_direction_flag  = (flags >> 6) & 1 != 0;
    out.edge_of_flight_line  = (flags >> 7) & 1 != 0;
    let cls = buf[off + 1];
    out.classification       = cls & 0x1F;
    out.flags                = (cls >> 5) & 0x07; // synthetic/key/withheld
    out.user_data            = buf[off + 2];
    out.scan_angle           = i16::from(buf[off + 3] as i8); // raw i8 in v1.x
    out.point_source_id      = u16::from_le_bytes(buf[off+4..off+6].try_into().unwrap());
}

/// LAS 1.4 extended return/flag layout.
#[inline]
fn decode_flags_v14(buf: &[u8], off: usize, out: &mut PointRecord) {
    let ret_byte = buf[off];
    let flg_byte = buf[off + 1];
    out.return_number       = ret_byte & 0x0F;
    out.number_of_returns   = (ret_byte >> 4) & 0x0F;
    out.classification      = buf[off + 2];
    out.user_data           = buf[off + 3];
    out.scan_angle          = i16::from_le_bytes(buf[off+4..off+6].try_into().unwrap());
    out.point_source_id     = u16::from_le_bytes(buf[off+6..off+8].try_into().unwrap());
    out.flags               = flg_byte & 0x1F; // scanner channel + flags
    out.scan_direction_flag = (flg_byte >> 6) & 1 != 0;
    out.edge_of_flight_line = (flg_byte >> 7) != 0;
}

#[inline]
fn read_rgb(buf: &[u8], off: usize) -> Rgb16 {
    Rgb16 {
        red:   u16::from_le_bytes(buf[off..off+2].try_into().unwrap()),
        green: u16::from_le_bytes(buf[off+2..off+4].try_into().unwrap()),
        blue:  u16::from_le_bytes(buf[off+4..off+6].try_into().unwrap()),
    }
}

#[inline]
fn read_thermal_rgb(buf: &[u8], off: usize) -> ThermalRgb {
    ThermalRgb {
        thermal: u16::from_le_bytes(buf[off..off+2].try_into().unwrap()),
        red:     u16::from_le_bytes(buf[off+2..off+4].try_into().unwrap()),
        green:   u16::from_le_bytes(buf[off+4..off+6].try_into().unwrap()),
        blue:    u16::from_le_bytes(buf[off+6..off+8].try_into().unwrap()),
    }
}

#[inline]
fn read_gps_time(buf: &[u8], off: usize) -> GpsTime {
    GpsTime(f64::from_le_bytes(buf[off..off+8].try_into().unwrap()))
}

#[inline]
fn read_waveform(buf: &[u8], off: usize) -> WaveformPacket {
    WaveformPacket {
        descriptor_index:    buf[off],
        byte_offset:         u64::from_le_bytes(buf[off+1..off+9].try_into().unwrap()),
        packet_size:         u32::from_le_bytes(buf[off+9..off+13].try_into().unwrap()),
        return_point_location: f32::from_le_bytes(buf[off+13..off+17].try_into().unwrap()),
        dx: f32::from_le_bytes(buf[off+17..off+21].try_into().unwrap()),
        dy: f32::from_le_bytes(buf[off+21..off+25].try_into().unwrap()),
        dz: f32::from_le_bytes(buf[off+25..off+29].try_into().unwrap()),
    }
}

// ── PDRF decoders ─────────────────────────────────────────────────────────────

fn decode_pdrf0(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);                // 0..14
    decode_flags_v13(buf, 14, out);           // 14..20
    Ok(())
}

fn decode_pdrf1(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v13(buf, 14, out);
    out.gps_time = Some(read_gps_time(buf, 20));
    Ok(())
}

fn decode_pdrf2(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v13(buf, 14, out);
    out.color = Some(read_rgb(buf, 20));
    Ok(())
}

fn decode_pdrf3(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v13(buf, 14, out);
    out.gps_time = Some(read_gps_time(buf, 20));
    out.color    = Some(read_rgb(buf, 28));
    Ok(())
}

fn decode_pdrf4(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v13(buf, 14, out);
    out.gps_time = Some(read_gps_time(buf, 20));
    out.waveform = Some(read_waveform(buf, 28));
    Ok(())
}

fn decode_pdrf5(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v13(buf, 14, out);
    out.gps_time = Some(read_gps_time(buf, 20));
    out.color    = Some(read_rgb(buf, 28));
    out.waveform = Some(read_waveform(buf, 34));
    Ok(())
}

fn decode_pdrf6(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_xyz(buf, out, hdr);
    decode_flags_v14(buf, 14, out);
    out.gps_time = Some(read_gps_time(buf, 22));
    Ok(())
}

fn decode_pdrf7(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_pdrf6(buf, out, hdr)?;
    out.color = Some(read_rgb(buf, 30));
    Ok(())
}

fn decode_pdrf8(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_pdrf7(buf, out, hdr)?;
    out.nir = Some(u16::from_le_bytes(buf[36..38].try_into().unwrap()));
    Ok(())
}

fn decode_pdrf9(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_pdrf6(buf, out, hdr)?;
    out.waveform = Some(read_waveform(buf, 30));
    Ok(())
}

fn decode_pdrf10(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    decode_pdrf7(buf, out, hdr)?;
    out.waveform = Some(read_waveform(buf, 36));
    Ok(())
}

fn decode_pdrf11(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    // LAS 1.5: PDRF 6 base equivalent, no extra fields.
    decode_pdrf6(buf, out, hdr)
}

fn decode_pdrf12(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    // LAS 1.5: PDRF 6 + 16-bit RGB
    decode_pdrf6(buf, out, hdr)?;
    out.color = Some(read_rgb(buf, 30));
    Ok(())
}

fn decode_pdrf13(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    // LAS 1.5: PDRF 6 + 16-bit RGB + 16-bit NIR + ThermalRGB
    decode_pdrf6(buf, out, hdr)?;
    out.color = Some(read_rgb(buf, 30));
    out.nir = Some(u16::from_le_bytes(buf[36..38].try_into().unwrap()));
    out.thermal_rgb = Some(read_thermal_rgb(buf, 38));
    Ok(())
}

fn decode_pdrf14(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    // LAS 1.5: PDRF 6 + 16-bit RGB + waveform
    decode_pdrf6(buf, out, hdr)?;
    out.color = Some(read_rgb(buf, 30));
    out.waveform = Some(read_waveform(buf, 36));
    Ok(())
}

fn decode_pdrf15(buf: &[u8], out: &mut PointRecord, hdr: &LasHeader) -> Result<()> {
    // LAS 1.5: PDRF 6 + 16-bit RGB + 16-bit NIR + waveform + ThermalRGB
    // Layout: base(30) + RGB(6) + NIR(2) + waveform(29) + ThermalRGB(8) = 75 bytes
    decode_pdrf6(buf, out, hdr)?;
    out.color = Some(read_rgb(buf, 30));
    out.nir = Some(u16::from_le_bytes(buf[36..38].try_into().unwrap()));
    out.waveform = Some(read_waveform(buf, 38));
    out.thermal_rgb = Some(read_thermal_rgb(buf, 67));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};

    use crate::io::{PointReader, PointWriter};
    use crate::las::header::PointDataFormat;
    use crate::las::reader::LasReader;
    use crate::las::vlr::{find_epsg, Vlr};
    use crate::las::writer::{LasWriter, WriterConfig};
    use crate::point::{GpsTime, PointRecord, Rgb16, ThermalRgb, WaveformPacket};

    #[test]
    fn infers_epsg_from_authority_missing_wkt_vlr() -> crate::Result<()> {
        let legacy_wkt = "PROJCS[\"NAD83_CSRS_UTM_zone_17N\",GEOGCS[\"GCS_NAD83(CSRS)\",DATUM[\"D_North_American_1983_CSRS\",SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",-81],PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";

        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = WriterConfig::default();
        cfg.vlrs.push(Vlr::ogc_wkt(legacy_wkt));

        {
            let mut writer = LasWriter::new(&mut cursor, cfg)?;
            let point = PointRecord { x: -80.0, y: 43.0, z: 300.0, ..PointRecord::default() };
            writer.write_point(&point)?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = LasReader::new(&mut cursor)?;

        // This fixture intentionally omits GeoKey EPSG VLR so inference must use WKT.
        assert_eq!(find_epsg(reader.vlrs()), None);
        assert_eq!(reader.crs().and_then(|c| c.epsg), Some(2958));

        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        Ok(())
    }

    fn make_cursor_for_pdrf(fmt: PointDataFormat, point: PointRecord) -> crate::Result<Cursor<Vec<u8>>> {
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let mut cfg = WriterConfig::default();
        cfg.point_data_format = fmt;
        {
            let mut writer = LasWriter::new(&mut cursor, cfg)?;
            writer.write_point(&point)?;
            writer.finish()?;
        }
        cursor.seek(SeekFrom::Start(0))?;
        Ok(cursor)
    }

    #[test]
    fn las_pdrf11_roundtrip() -> crate::Result<()> {
        let input = PointRecord {
            x: 10.123,
            y: 20.456,
            z: 30.789,
            intensity: 1234,
            classification: 5,
            return_number: 2,
            number_of_returns: 3,
            gps_time: Some(GpsTime(12345.678)),
            ..PointRecord::default()
        };
        let mut cursor = make_cursor_for_pdrf(PointDataFormat::Pdrf11, input)?;
        let mut reader = LasReader::new(&mut cursor)?;
        assert_eq!(reader.header().version_minor, 5);
        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        assert!((p.x - input.x).abs() < 1e-3);
        assert!((p.y - input.y).abs() < 1e-3);
        assert!((p.z - input.z).abs() < 1e-3);
        assert_eq!(p.intensity, input.intensity);
        assert_eq!(p.classification, input.classification);
        assert_eq!(p.return_number, input.return_number);
        assert_eq!(p.number_of_returns, input.number_of_returns);
        assert!((p.gps_time.unwrap().0 - 12345.678).abs() < 1e-9);
        assert!(p.color.is_none());
        assert!(p.waveform.is_none());
        Ok(())
    }

    #[test]
    fn las_pdrf12_roundtrip() -> crate::Result<()> {
        let input = PointRecord {
            x: 11.0,
            y: 22.0,
            z: 33.0,
            intensity: 2048,
            classification: 2,
            return_number: 1,
            number_of_returns: 1,
            gps_time: Some(GpsTime(9999.001)),
            color: Some(Rgb16 { red: 5000, green: 6000, blue: 7000 }),
            ..PointRecord::default()
        };
        let mut cursor = make_cursor_for_pdrf(PointDataFormat::Pdrf12, input)?;
        let mut reader = LasReader::new(&mut cursor)?;
        assert_eq!(reader.header().version_minor, 5);
        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        assert!((p.x - input.x).abs() < 1e-3);
        assert!((p.y - input.y).abs() < 1e-3);
        assert!((p.z - input.z).abs() < 1e-3);
        assert_eq!(p.intensity, input.intensity);
        assert_eq!(p.classification, input.classification);
        assert_eq!(p.color, Some(Rgb16 { red: 5000, green: 6000, blue: 7000 }));
        assert!(p.waveform.is_none());
        assert!(p.thermal_rgb.is_none());
        Ok(())
    }

    #[test]
    fn las_pdrf13_roundtrip() -> crate::Result<()> {
        let input = PointRecord {
            x: 55.5,
            y: 66.6,
            z: 77.7,
            intensity: 3000,
            classification: 3,
            return_number: 1,
            number_of_returns: 2,
            gps_time: Some(GpsTime(1001.5)),
            color: Some(Rgb16 { red: 10000, green: 20000, blue: 30000 }),
            nir: Some(40000),
            thermal_rgb: Some(ThermalRgb { thermal: 1111, red: 2222, green: 3333, blue: 4444 }),
            ..PointRecord::default()
        };
        let mut cursor = make_cursor_for_pdrf(PointDataFormat::Pdrf13, input)?;
        let mut reader = LasReader::new(&mut cursor)?;
        assert_eq!(reader.header().version_minor, 5);
        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        assert!((p.x - input.x).abs() < 1e-3);
        assert!((p.y - input.y).abs() < 1e-3);
        assert!((p.z - input.z).abs() < 1e-3);
        assert_eq!(p.intensity, input.intensity);
        assert_eq!(p.classification, input.classification);
        assert_eq!(p.color, Some(Rgb16 { red: 10000, green: 20000, blue: 30000 }));
        assert_eq!(p.nir, Some(40000));
        assert_eq!(p.thermal_rgb, Some(ThermalRgb { thermal: 1111, red: 2222, green: 3333, blue: 4444 }));
        assert!(p.waveform.is_none());
        Ok(())
    }

    #[test]
    fn las_pdrf14_roundtrip() -> crate::Result<()> {
        let wf = WaveformPacket {
            descriptor_index: 4,
            byte_offset: 98765,
            packet_size: 128,
            return_point_location: 0.5,
            dx: 0.1,
            dy: 0.2,
            dz: 0.3,
        };
        let input = PointRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            intensity: 500,
            classification: 2,
            return_number: 1,
            number_of_returns: 1,
            gps_time: Some(GpsTime(777.777)),
            color: Some(Rgb16 { red: 100, green: 200, blue: 300 }),
            waveform: Some(wf),
            ..PointRecord::default()
        };
        let mut cursor = make_cursor_for_pdrf(PointDataFormat::Pdrf14, input)?;
        let mut reader = LasReader::new(&mut cursor)?;
        assert_eq!(reader.header().version_minor, 5);
        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        assert!((p.x - input.x).abs() < 1e-3);
        assert!((p.y - input.y).abs() < 1e-3);
        assert!((p.z - input.z).abs() < 1e-3);
        assert_eq!(p.color, Some(Rgb16 { red: 100, green: 200, blue: 300 }));
        assert!(p.nir.is_none());
        let pw = p.waveform.expect("waveform should be present");
        assert_eq!(pw.descriptor_index, 4);
        assert_eq!(pw.byte_offset, 98765);
        assert_eq!(pw.packet_size, 128);
        assert!((pw.return_point_location - 0.5).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn las_pdrf15_roundtrip() -> crate::Result<()> {
        let wf = WaveformPacket {
            descriptor_index: 7,
            byte_offset: 11111,
            packet_size: 64,
            return_point_location: 0.25,
            dx: 1.0,
            dy: 2.0,
            dz: 3.0,
        };
        let input = PointRecord {
            x: 100.0,
            y: 200.0,
            z: 300.0,
            intensity: 65535,
            classification: 6,
            return_number: 3,
            number_of_returns: 5,
            gps_time: Some(GpsTime(3600.0)),
            color: Some(Rgb16 { red: 60000, green: 50000, blue: 40000 }),
            nir: Some(55000),
            waveform: Some(wf),
            thermal_rgb: Some(ThermalRgb { thermal: 9999, red: 8888, green: 7777, blue: 6666 }),
            ..PointRecord::default()
        };
        let mut cursor = make_cursor_for_pdrf(PointDataFormat::Pdrf15, input)?;
        let mut reader = LasReader::new(&mut cursor)?;
        assert_eq!(reader.header().version_minor, 5);
        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        assert!((p.x - input.x).abs() < 1e-3);
        assert!((p.y - input.y).abs() < 1e-3);
        assert!((p.z - input.z).abs() < 1e-3);
        assert_eq!(p.intensity, input.intensity);
        assert_eq!(p.classification, input.classification);
        assert_eq!(p.color, Some(Rgb16 { red: 60000, green: 50000, blue: 40000 }));
        assert_eq!(p.nir, Some(55000));
        let pw = p.waveform.expect("waveform should be present");
        assert_eq!(pw.descriptor_index, 7);
        assert_eq!(pw.byte_offset, 11111);
        assert_eq!(pw.packet_size, 64);
        assert_eq!(p.thermal_rgb, Some(ThermalRgb { thermal: 9999, red: 8888, green: 7777, blue: 6666 }));
        Ok(())
    }
}
