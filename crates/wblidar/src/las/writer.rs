//! LAS 1.4 R15 writer (writes PDRF 6 / 7 / 8 by default).

use std::io::{Seek, SeekFrom, Write};
use wide::f64x4;
use crate::crs::{ogc_wkt_from_epsg, Crs};
use crate::io::le;
use crate::las::header::{GlobalEncoding, LasHeader, PointDataFormat};
use crate::las::vlr::{Vlr, LASF_PROJECTION_USER_ID, OGC_WKT_RECORD_ID};
use crate::point::PointRecord;
use crate::Result;
use crate::io::PointWriter;

/// Configuration for the LAS writer.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Point-data record format to write (default: auto-detect from first point).
    pub point_data_format: PointDataFormat,
    /// X scale factor (default 0.001).
    pub x_scale: f64,
    /// Y scale factor (default 0.001).
    pub y_scale: f64,
    /// Z scale factor (default 0.001).
    pub z_scale: f64,
    /// X offset (default 0.0).
    pub x_offset: f64,
    /// Y offset (default 0.0).
    pub y_offset: f64,
    /// Z offset (default 0.0).
    pub z_offset: f64,
    /// System identifier string (up to 32 chars).
    pub system_identifier: String,
    /// Generating-software string (up to 32 chars).
    pub generating_software: String,
    /// VLRs to include before the point data.
    pub vlrs: Vec<Vlr>,
    /// Optional CRS metadata to emit in LAS projection VLRs.
    pub crs: Option<Crs>,
    /// Number of extra bytes per point.
    pub extra_bytes_per_point: u16,
}

impl Default for WriterConfig {
    fn default() -> Self {
        WriterConfig {
            point_data_format: PointDataFormat::Pdrf6,
            x_scale: 0.001,
            y_scale: 0.001,
            z_scale: 0.001,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            system_identifier: String::new(),
            generating_software: String::from("wblidar"),
            vlrs: Vec::new(),
            crs: None,
            extra_bytes_per_point: 0,
        }
    }
}

/// A streaming LAS 1.4 R15 writer.
///
/// Call [`finish`](LasWriter::finish) after all points are written to
/// back-patch the header with the final point count and bounding box.
pub struct LasWriter<W: Write + Seek> {
    inner: W,
    config: WriterConfig,
    point_count: u64,
    per_return: [u64; 15],
    // Running bounding box
    min_x: f64, max_x: f64,
    min_y: f64, max_y: f64,
    min_z: f64, max_z: f64,
    // Byte offset of the start of the header (for back-patching).
    header_start: u64,
    // Byte offset of the first point record.
    point_data_start: u64,
}

impl<W: Write + Seek> LasWriter<W> {
    /// Create a new writer, emitting the header and VLRs immediately.
    pub fn new(mut inner: W, mut config: WriterConfig) -> Result<Self> {
        append_projection_vlrs(&mut config);
        let header_start = inner.seek(SeekFrom::Current(0))?;

        // Compute offset to point data: 375 (header) + sum of VLR sizes
        let vlr_total: usize = config.vlrs.iter().map(|v| v.serialised_size()).sum();
        let offset_to_point_data = 375u32 + vlr_total as u32;

        let record_length =
            config.point_data_format.core_size() + config.extra_bytes_per_point;
        let global_encoding = global_encoding_for_vlrs(&config.vlrs);

        // Auto-detect LAS version based on PDRF
        let version_minor = if config.point_data_format.is_v15() { 5 } else { 4 };

        // Build a skeleton header (bounding box and counts will be back-patched).
        let hdr = LasHeader {
            version_major: 1,
            version_minor,
            system_identifier: config.system_identifier.clone(),
            generating_software: config.generating_software.clone(),
            file_creation_day: day_of_year(),
            file_creation_year: current_year(),
            header_size: 375,
            offset_to_point_data,
            number_of_vlrs: config.vlrs.len() as u32,
            point_data_format: config.point_data_format,
            point_data_record_length: record_length,
            global_encoding,
            project_id: [0u8; 16],
            x_scale: config.x_scale,
            y_scale: config.y_scale,
            z_scale: config.z_scale,
            x_offset: config.x_offset,
            y_offset: config.y_offset,
            z_offset: config.z_offset,
            max_x: 0.0, min_x: 0.0,
            max_y: 0.0, min_y: 0.0,
            max_z: 0.0, min_z: 0.0,
            legacy_point_count: 0,
            legacy_point_count_by_return: [0u32; 5],
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(0),
            point_count_64: Some(0),
            point_count_by_return_64: Some([0u64; 15]),
            extra_bytes_count: config.extra_bytes_per_point,
        };

        hdr.write(&mut inner)?;
        for vlr in &config.vlrs { vlr.write(&mut inner)?; }

        let point_data_start = inner.seek(SeekFrom::Current(0))?;

        Ok(LasWriter {
            inner, config,
            point_count: 0,
            per_return: [0u64; 15],
            min_x: f64::MAX, max_x: f64::MIN,
            min_y: f64::MAX, max_y: f64::MIN,
            min_z: f64::MAX, max_z: f64::MIN,
            header_start, point_data_start,
        })
    }
}

fn append_projection_vlrs(config: &mut WriterConfig) {
    let Some(crs) = &config.crs else { return; };

    let has_wkt = config.vlrs.iter().any(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
    });

    if !has_wkt {
        if let Some(wkt) = crs.wkt.as_deref().map(ToOwned::to_owned).or_else(|| {
            crs.epsg.and_then(ogc_wkt_from_epsg)
        }) {
            config.vlrs.push(Vlr::ogc_wkt(&wkt));
        }
    }

    // Do not auto-add GeoKeyDirectory from CRS defaults.
    // Some external validators/viewers are fragile with minimal geokey-only
    // metadata. Callers can still provide explicit geokey VLRs via config.vlrs.
}

fn global_encoding_for_vlrs(vlrs: &[Vlr]) -> GlobalEncoding {
    let mut bits = GlobalEncoding::GPS_TIME_TYPE;
    let has_wkt = vlrs.iter().any(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
    });
    if has_wkt {
        bits |= GlobalEncoding::WKT;
    }
    GlobalEncoding(bits)
}

impl<W: Write + Seek> PointWriter for LasWriter<W> {
    fn write_point(&mut self, p: &PointRecord) -> Result<()> {
        let fmt = self.config.point_data_format;
        let cfg = &self.config;

        // Encode XYZ as scaled integers
        let coords = f64x4::new([p.x, p.y, p.z, 0.0]);
        let offsets = f64x4::new([cfg.x_offset, cfg.y_offset, cfg.z_offset, 0.0]);
        let scales = f64x4::new([cfg.x_scale, cfg.y_scale, cfg.z_scale, 1.0]);
        let quantized: [f64; 4] = ((coords - offsets) / scales).round().into();
        let xi = quantized[0] as i32;
        let yi = quantized[1] as i32;
        let zi = quantized[2] as i32;

        le::write_i32(&mut self.inner, xi)?;
        le::write_i32(&mut self.inner, yi)?;
        le::write_i32(&mut self.inner, zi)?;
        le::write_u16(&mut self.inner, p.intensity)?;

        if fmt.is_v14() || fmt.is_v15() {
            // LAS 1.4 and 1.5 layout (both use same core structure)
            let ret_byte = (p.return_number & 0x0F) | ((p.number_of_returns & 0x0F) << 4);
            let flg_byte = (p.flags & 0x1F)
                | (u8::from(p.scan_direction_flag) << 6)
                | (u8::from(p.edge_of_flight_line) << 7);
            le::write_u8(&mut self.inner, ret_byte)?;
            le::write_u8(&mut self.inner, flg_byte)?;
            le::write_u8(&mut self.inner, p.classification)?;
            le::write_u8(&mut self.inner, p.user_data)?;
            le::write_i16(&mut self.inner, p.scan_angle)?;
            le::write_u16(&mut self.inner, p.point_source_id)?;
            // GPS time (always present in PDRF 6-10 and 11-15)
            let gps = p.gps_time.map_or(0.0, |g| g.0);
            le::write_f64(&mut self.inner, gps)?;
        } else {
            // LAS 1.0-1.3 layout
            let flg = (p.return_number & 0x07)
                | ((p.number_of_returns & 0x07) << 3)
                | (u8::from(p.scan_direction_flag) << 6)
                | (u8::from(p.edge_of_flight_line) << 7);
            let cls = (p.classification & 0x1F) | ((p.flags & 0x07) << 5);
            le::write_u8(&mut self.inner, flg)?;
            le::write_u8(&mut self.inner, cls)?;
            le::write_u8(&mut self.inner, p.user_data)?;
            le::write_i8(&mut self.inner, (p.scan_angle as i8).clamp(-90, 90))?;
            le::write_u16(&mut self.inner, p.point_source_id)?;
        }

        // GPS time for PDRFs 1, 3, 4, 5
        if !fmt.is_v14() && !fmt.is_v15() && fmt.has_gps_time() {
            let gps = p.gps_time.map_or(0.0, |g| g.0);
            le::write_f64(&mut self.inner, gps)?;
        }

        // RGB
        if fmt.has_rgb() {
            let c = p.color.unwrap_or_default();
            le::write_u16(&mut self.inner, c.red)?;
            le::write_u16(&mut self.inner, c.green)?;
            le::write_u16(&mut self.inner, c.blue)?;
        }

        // NIR (PDRF 8 only)
        if fmt.has_nir() {
            le::write_u16(&mut self.inner, p.nir.unwrap_or(0))?;
        }

        // Waveform
        if fmt.has_waveform() {
            let wf = p.waveform.unwrap_or_default();
            le::write_u8(&mut self.inner, wf.descriptor_index)?;
            le::write_u64(&mut self.inner, wf.byte_offset)?;
            le::write_u32(&mut self.inner, wf.packet_size)?;
            le::write_f32(&mut self.inner, wf.return_point_location)?;
            le::write_f32(&mut self.inner, wf.dx)?;
            le::write_f32(&mut self.inner, wf.dy)?;
            le::write_f32(&mut self.inner, wf.dz)?;
        }

        // ThermalRGB (LAS 1.5 PDRFs 13, 15)
        if fmt.is_v15() && (fmt == PointDataFormat::Pdrf13 || fmt == PointDataFormat::Pdrf15) {
            let tr = p.thermal_rgb.unwrap_or_default();
            le::write_u16(&mut self.inner, tr.thermal)?;
            le::write_u16(&mut self.inner, tr.red)?;
            le::write_u16(&mut self.inner, tr.green)?;
            le::write_u16(&mut self.inner, tr.blue)?;
        }

        // Extra bytes (zero-padded if extra_bytes_per_point > extra_bytes.len)
        let extra_len = self.config.extra_bytes_per_point as usize;
        if extra_len > 0 {
            let src_len = p.extra_bytes.len as usize;
            let write_len = src_len.min(extra_len);
            self.inner.write_all(&p.extra_bytes.data[..write_len])?;
            // Pad remaining bytes with zeros
            for _ in write_len..extra_len {
                le::write_u8(&mut self.inner, 0)?;
            }
        }

        // Update bounding box using branchless SIMD min/max.
        let coords = f64x4::new([p.x, p.y, p.z, 0.0]);
        let mins = f64x4::new([self.min_x, self.min_y, self.min_z, f64::INFINITY]).min(coords);
        let maxs = f64x4::new([self.max_x, self.max_y, self.max_z, f64::NEG_INFINITY]).max(coords);
        let min_arr: [f64; 4] = mins.into();
        let max_arr: [f64; 4] = maxs.into();
        self.min_x = min_arr[0];
        self.min_y = min_arr[1];
        self.min_z = min_arr[2];
        self.max_x = max_arr[0];
        self.max_y = max_arr[1];
        self.max_z = max_arr[2];

        // Update per-return counts
        let ret = p.return_number as usize;
        if ret > 0 && ret <= 15 { self.per_return[ret - 1] += 1; }
        self.point_count += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        // Seek back to header start and rewrite with correct counts + bounds.
        self.inner.seek(SeekFrom::Start(self.header_start))?;

        let (min_x, max_x) = if self.point_count == 0 { (0.0, 0.0) } else { (self.min_x, self.max_x) };
        let (min_y, max_y) = if self.point_count == 0 { (0.0, 0.0) } else { (self.min_y, self.max_y) };
        let (min_z, max_z) = if self.point_count == 0 { (0.0, 0.0) } else { (self.min_z, self.max_z) };

        let legacy_count = self.point_count.min(u64::from(u32::MAX)) as u32;
        let mut legacy_per_return = [0u32; 5];
        for i in 0..5 { legacy_per_return[i] = self.per_return[i].min(u64::from(u32::MAX)) as u32; }

        let vlr_total: usize = self.config.vlrs.iter().map(|v| v.serialised_size()).sum();
        let record_length =
            self.config.point_data_format.core_size() + self.config.extra_bytes_per_point;
        let global_encoding = global_encoding_for_vlrs(&self.config.vlrs);

        let hdr = LasHeader {
            version_major: 1,
            version_minor: if self.config.point_data_format.is_v15() { 5 } else { 4 },
            system_identifier: self.config.system_identifier.clone(),
            generating_software: self.config.generating_software.clone(),
            file_creation_day: day_of_year(),
            file_creation_year: current_year(),
            header_size: 375,
            offset_to_point_data: 375 + vlr_total as u32,
            number_of_vlrs: self.config.vlrs.len() as u32,
            point_data_format: self.config.point_data_format,
            point_data_record_length: record_length,
            global_encoding,
            project_id: [0u8; 16],
            x_scale: self.config.x_scale,
            y_scale: self.config.y_scale,
            z_scale: self.config.z_scale,
            x_offset: self.config.x_offset,
            y_offset: self.config.y_offset,
            z_offset: self.config.z_offset,
            max_x, min_x, max_y, min_y, max_z, min_z,
            legacy_point_count: legacy_count,
            legacy_point_count_by_return: legacy_per_return,
            waveform_data_packet_offset: Some(0),
            start_of_first_evlr: Some(0),
            number_of_evlrs: Some(0),
            point_count_64: Some(self.point_count),
            point_count_by_return_64: Some(self.per_return),
            extra_bytes_count: self.config.extra_bytes_per_point,
        };

        hdr.write(&mut self.inner)?;
        // Seek back to end so further writes (if any) go to the right place.
        self.inner.seek(SeekFrom::Start(
            self.point_data_start + self.point_count * u64::from(record_length)
        ))?;
        Ok(())
    }
}

// ── Calendar helpers (no chrono dependency) ──────────────────────────────────

fn day_of_year() -> u16 {
    // Approximate; good enough for file metadata.
    // A full implementation would use std::time.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day_of_year_approx = ((secs / 86400) % 365) as u16 + 1;
    day_of_year_approx
}

fn current_year() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Approximate year from Unix timestamp
    let days = secs / 86400;
    let years_since_1970 = days / 365;
    (1970 + years_since_1970) as u16
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};
    use crate::crs::Crs;
    use crate::io::{PointReader, PointWriter};
    use crate::las::header::GlobalEncoding;
    use crate::las::reader::LasReader;
    use crate::las::vlr::{
        find_epsg, find_ogc_wkt, Vlr, GEOKEY_DIRECTORY_RECORD_ID,
        LASF_PROJECTION_USER_ID, OGC_WKT_RECORD_ID,
    };
    use crate::las::writer::{LasWriter, WriterConfig};
    use crate::point::PointRecord;

    #[test]
    fn las_does_not_duplicate_projection_vlrs() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = WriterConfig::default();
        cfg.crs = Some(Crs::from_epsg(4326));
        cfg.vlrs.push(Vlr::ogc_wkt("GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",\"4326\"]]"));
        cfg.vlrs.push(Vlr::geokey_directory_for_epsg(4326).expect("valid epsg for geokey"));

        {
            let mut writer = LasWriter::new(&mut cursor, cfg)?;
            let point = PointRecord { x: -80.0, y: 43.0, z: 300.0, ..PointRecord::default() };
            writer.write_point(&point)?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let mut reader = LasReader::new(&mut cursor)?;

        let wkt_count = reader.vlrs().iter().filter(|v| {
            v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
        }).count();
        let geokey_count = reader.vlrs().iter().filter(|v| {
            v.key.user_id == LASF_PROJECTION_USER_ID
                && v.key.record_id == GEOKEY_DIRECTORY_RECORD_ID
        }).count();

        assert_eq!(wkt_count, 1);
        assert_eq!(geokey_count, 1);
        assert_eq!(find_epsg(reader.vlrs()), Some(4326));
        assert!(find_ogc_wkt(reader.vlrs()).is_some());

        let mut p = PointRecord::default();
        assert!(reader.read_point(&mut p)?);
        Ok(())
    }

    #[test]
    fn las_wkt_global_encoding_bit_requires_wkt_vlr() -> crate::Result<()> {
        let mut cursor = Cursor::new(Vec::<u8>::new());

        let mut cfg = WriterConfig::default();
        cfg.crs = None;

        {
            let mut writer = LasWriter::new(&mut cursor, cfg)?;
            writer.write_point(&PointRecord::default())?;
            writer.finish()?;
        }

        cursor.seek(SeekFrom::Start(0))?;
        let reader = LasReader::new(&mut cursor)?;
        assert!(!reader.header().global_encoding.is_set(GlobalEncoding::WKT));
        Ok(())
    }
}
