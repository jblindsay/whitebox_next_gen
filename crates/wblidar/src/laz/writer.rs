//! LAZ streaming writer.
//!
//! Accumulates points into a chunk buffer, DEFLATE-compresses each full chunk,
//! and records the byte offset in the chunk table.  On `finish()`, the chunk
//! table is back-patched to its placeholder location at the beginning of the
//! LAZ data block, then the LAS header is back-patched with final counts.

use std::io::{Seek, SeekFrom, Write};
use wide::f64x4;
use crate::io::{le, PointWriter};
use crate::las::header::PointDataFormat;
use crate::las::writer::{LasWriter, WriterConfig};
use crate::laz::chunk::ChunkTable;

use crate::laz::laszip_chunk_table::{write_laszip_chunk_table, LaszipChunkTableEntry};
use crate::laz::standard_point10_write::encode_standard_pointwise_chunk_point10_v2;
use crate::laz::standard_point14::encode_standard_layered_chunk_point14_v3_constant_attributes;
use crate::laz::{build_laszip_vlr_for_format_with_extra_bytes, DEFAULT_CHUNK_SIZE};
use crate::point::PointRecord;
use crate::Result;

/// Configuration for the LAZ writer.
#[derive(Debug, Clone)]
pub struct LazWriterConfig {
    /// Underlying LAS writer configuration.
    pub las: WriterConfig,
    /// Points per compressed chunk (default 50 000).
    pub chunk_size: u32,
    /// Compression tuning level 0 (fastest) - 9 (smallest).
    ///
    /// For Point10 payloads this is currently informational.
    /// For Point14-family payloads this controls the effective chunk target size
    /// (lower levels favor smaller chunks/faster writes; higher levels favor
    /// larger chunks/better compression ratio).
    pub compression_level: u32,
    /// Ignored. Writer always emits standards-compliant LASzip v2/v3 payloads.
    ///
    /// Kept for backward compatibility with code that sets this field.
    #[deprecated(since = "0.2.0", note = "field is ignored; writer always uses standards-compliant encoding")]
    pub standards_compliant: bool,
}

impl Default for LazWriterConfig {
    fn default() -> Self {
        #[allow(deprecated)]
        {
            LazWriterConfig {
                las: WriterConfig::default(),
                chunk_size: DEFAULT_CHUNK_SIZE,
                compression_level: 6,
                standards_compliant: false,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LazPayloadMode {
    StandardPoint10,
    StandardPoint14,
}

/// A streaming LAZ writer.
pub struct LazWriter<W: Write + Seek> {
    inner: W,
    config: LazWriterConfig,
    /// Points accumulated in the current chunk.
    chunk_buf: Vec<PointRecord>,
    #[allow(dead_code)]
    chunk_table: ChunkTable,
    /// Running byte offset from the start of the first chunk.
    #[allow(dead_code)]
    cumulative_offset: u64,
    payload_mode: LazPayloadMode,
    chunk_table_ptr_pos: Option<u64>,
    standard_chunk_entries: Vec<LaszipChunkTableEntry>,
    /// LAS header position (start of file).
    _las_header_pos: u64,
    total_points: u64,
    min_x: f64, max_x: f64,
    min_y: f64, max_y: f64,
    min_z: f64, max_z: f64,
}

impl<W: Write + Seek> LazWriter<W> {
    const LAS_POINT_DATA_FORMAT_OFFSET: u64 = 104;
    const LAS_COMPRESSED_POINT_FORMAT_BIT: u8 = 0x80;

    fn effective_chunk_size(&self) -> usize {
        let base = self.config.chunk_size.max(1);

        if !matches!(self.payload_mode, LazPayloadMode::StandardPoint14) {
            return base as usize;
        }

        let lvl = self.config.compression_level.min(9);
        let tuned = match lvl {
            0 => base / 2,
            1 => (base.saturating_mul(2)) / 3,
            2 => (base.saturating_mul(3)) / 4,
            3..=6 => base,
            7 => (base.saturating_mul(5)) / 4,
            8 => (base.saturating_mul(3)) / 2,
            _ => base.saturating_mul(2),
        };

        tuned.max(1) as usize
    }

    /// Create and initialise a new LAZ writer.  Writes the LAS header, the
    /// LASzip VLR, and a placeholder chunk table.
    pub fn new(mut inner: W, mut config: LazWriterConfig) -> Result<Self> {
        let las_header_pos = inner.seek(SeekFrom::Current(0))?;

        let payload_mode = if matches!(
            config.las.point_data_format,
            PointDataFormat::Pdrf0
                | PointDataFormat::Pdrf1
                | PointDataFormat::Pdrf2
                | PointDataFormat::Pdrf3
                | PointDataFormat::Pdrf4
                | PointDataFormat::Pdrf5
        ) {
            LazPayloadMode::StandardPoint10
        } else if config.las.point_data_format.is_v14() || config.las.point_data_format.is_v15() {
            LazPayloadMode::StandardPoint14
        } else {
            return Err(crate::Error::Unimplemented(
                "LazWriter supports PDRF0-10 and v1.5 PDRF11-15",
            ));
        };

        // Inject the LASzip VLR so readers know this is compressed.
        let laszip_vlr = build_laszip_vlr_for_format_with_extra_bytes(
            config.las.point_data_format,
            config.chunk_size,
            config.las.extra_bytes_per_point,
        );
        config.las.vlrs.push(laszip_vlr);

        // Write LAS header + VLRs using a temporary LasWriter.  We immediately
        // finish it (without points) so the header is in place.
        let mut las_writer = LasWriter::new(&mut inner, config.las.clone())?;
        las_writer.finish()?;

        // LAZ payloads require the compressed flag in the LAS point format byte.
        // Without this bit, external tools may treat payload bytes as uncompressed LAS points.
        let after_header = inner.seek(SeekFrom::Current(0))?;
        inner.seek(SeekFrom::Start(
            las_header_pos + Self::LAS_POINT_DATA_FORMAT_OFFSET,
        ))?;
        let raw_pdrf = config.las.point_data_format as u8;
        le::write_u8(&mut inner, raw_pdrf | Self::LAS_COMPRESSED_POINT_FORMAT_BIT)?;
        inner.seek(SeekFrom::Start(after_header))?;

        let chunk_table_ptr_pos = match payload_mode {
            LazPayloadMode::StandardPoint10 | LazPayloadMode::StandardPoint14 => {
                // Standard LASzip: offset_to_point_data stores pointer to chunk table near EOF.
                let pos = inner.seek(SeekFrom::Current(0))?;
                le::write_u64(&mut inner, 0)?;
                Some(pos)
            }
        };

        Ok(LazWriter {
            inner, config,
            chunk_buf: Vec::with_capacity(DEFAULT_CHUNK_SIZE as usize),
            chunk_table: ChunkTable::default(),
            cumulative_offset: 0,
            payload_mode,
            chunk_table_ptr_pos,
            standard_chunk_entries: Vec::new(),
            _las_header_pos: las_header_pos,
            total_points: 0,
            min_x: f64::MAX, max_x: f64::MIN,
            min_y: f64::MAX, max_y: f64::MIN,
            min_z: f64::MAX, max_z: f64::MIN,
        })
    }

    fn flush_chunk(&mut self) -> Result<()> {
        if self.chunk_buf.is_empty() { return Ok(()); }

        match self.payload_mode {
            LazPayloadMode::StandardPoint10 => {
                let compressed = encode_standard_pointwise_chunk_point10_v2(
                    &self.chunk_buf,
                    self.config.las.point_data_format,
                    self.config.las.extra_bytes_per_point as usize,
                    [1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0],
                )?;
                self.inner.write_all(&compressed)?;
                self.standard_chunk_entries.push(LaszipChunkTableEntry {
                    point_count: self.chunk_buf.len() as u64,
                    byte_count: compressed.len() as u64,
                });
            }
            LazPayloadMode::StandardPoint14 => {
                let compressed = encode_standard_layered_chunk_point14_v3_constant_attributes(
                    &self.chunk_buf,
                    self.config.las.point_data_format,
                    [1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0],
                )?;
                self.inner.write_all(&compressed)?;
                self.standard_chunk_entries.push(LaszipChunkTableEntry {
                    point_count: self.chunk_buf.len() as u64,
                    byte_count: compressed.len() as u64,
                });
            }
        }

        self.chunk_buf.clear();
        Ok(())
    }
}

impl<W: Write + Seek> PointWriter for LazWriter<W> {
    fn write_point(&mut self, p: &PointRecord) -> Result<()> {
        if matches!(
            self.payload_mode,
            LazPayloadMode::StandardPoint10 | LazPayloadMode::StandardPoint14
        ) {
            let declared = self.config.las.extra_bytes_per_point as usize;
            let actual = p.extra_bytes.len as usize;
            if declared == 0 {
                if actual > 0 {
                    return Err(crate::Error::Unimplemented(
                        "LazWriter standards_compliant mode does not yet support per-point extra-bytes payloads unless extra_bytes_per_point is declared",
                    ));
                }
            } else if actual != declared {
                return Err(crate::Error::InvalidValue {
                    field: "laz.standard_writer.extra_bytes",
                    detail: format!(
                        "point extra-bytes length {} does not match declared extra_bytes_per_point {}",
                        actual, declared
                    ),
                });
            }
        }

        // Track bounding box.
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
        self.total_points += 1;

        // Pre-quantize float coordinates to the integer domain that the codec
        // operates in (same as the LAS integer representation).
        let sx = self.config.las.x_scale;
        let sy = self.config.las.y_scale;
        let sz = self.config.las.z_scale;
        let ox = self.config.las.x_offset;
        let oy = self.config.las.y_offset;
        let oz = self.config.las.z_offset;
        let quantized = PointRecord {
            x: ((p.x - ox) / sx).round(),
            y: ((p.y - oy) / sy).round(),
            z: ((p.z - oz) / sz).round(),
            ..*p
        };
        self.chunk_buf.push(quantized);
        if self.chunk_buf.len() >= self.effective_chunk_size() {
            self.flush_chunk()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        // Flush last partial chunk.
        self.flush_chunk()?;

        if matches!(
            self.payload_mode,
            LazPayloadMode::StandardPoint10 | LazPayloadMode::StandardPoint14
        ) {
            let chunk_table_offset = self.inner.seek(SeekFrom::Current(0))?;
            write_laszip_chunk_table(
                &mut self.inner,
                &self.standard_chunk_entries,
                // Fixed chunk-size LAZ does not store per-chunk point counts in the
                // chunk table — only byte counts.  Variable-chunk streams (chunk_size
                // == u32::MAX, e.g. COPC) do include point counts, but those are
                // written by the COPC writer directly, not by LazWriter.
                false,
            )?;

            let file_end_after_table = self.inner.seek(SeekFrom::Current(0))?;
            let ptr_pos = self.chunk_table_ptr_pos.ok_or_else(|| crate::Error::InvalidValue {
                field: "laz.chunk_table_pointer",
                detail: "missing standard chunk-table pointer position".to_string(),
            })?;
            self.inner.seek(SeekFrom::Start(ptr_pos))?;
            le::write_u64(&mut self.inner, chunk_table_offset)?;
            self.inner.seek(SeekFrom::Start(file_end_after_table))?;
        }

        // ── Back-patch LAS 1.4 header with final counts and bounding box ──
        //
        // LAS 1.4 header byte offsets (little-endian, fixed 375-byte header):
        //   107: u32  legacy_point_count
        //   179: f64  max_x, 187: f64 min_x
        //   195: f64  max_y, 203: f64 min_y
        //   211: f64  max_z, 219: f64 min_z
        //   247: u64  point_count_64
        let file_end = self.inner.seek(SeekFrom::Current(0))?;
        let n = self.total_points;

        self.inner.seek(SeekFrom::Start(107))?;
        le::write_u32(&mut self.inner, n.min(u32::MAX as u64) as u32)?;

        self.inner.seek(SeekFrom::Start(179))?;
        le::write_f64(&mut self.inner, self.max_x)?;
        le::write_f64(&mut self.inner, self.min_x)?;
        le::write_f64(&mut self.inner, self.max_y)?;
        le::write_f64(&mut self.inner, self.min_y)?;
        le::write_f64(&mut self.inner, self.max_z)?;
        le::write_f64(&mut self.inner, self.min_z)?;

        self.inner.seek(SeekFrom::Start(247))?;
        le::write_u64(&mut self.inner, n)?;

        // Restore stream to end of file.
        self.inner.seek(SeekFrom::Start(file_end))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::le;
    use std::io::Cursor;
    use crate::io::{PointReader, PointWriter};
    use crate::las::reader::LasReader;
    use crate::laz::reader::LazReader;
    use crate::laz::laszip_chunk_table::read_laszip_chunk_table_entries;
    use crate::laz::{parse_laszip_vlr, LaszipCompressorType};
    use crate::point::{GpsTime, Rgb16, WaveformPacket};

    #[test]
    fn point14_compression_level_reduces_chunk_target_at_low_levels() {
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 100;
        cfg.compression_level = 0;
        cfg.las.point_data_format = PointDataFormat::Pdrf7;

        let writer = LazWriter {
            inner: Cursor::new(Vec::<u8>::new()),
            config: cfg,
            chunk_buf: Vec::new(),
            chunk_table: ChunkTable::default(),
            cumulative_offset: 0,
            payload_mode: LazPayloadMode::StandardPoint14,
            chunk_table_ptr_pos: None,
            standard_chunk_entries: Vec::new(),
            _las_header_pos: 0,
            total_points: 0,
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
            min_z: f64::MAX,
            max_z: f64::MIN,
        };

        assert_eq!(writer.effective_chunk_size(), 50);
    }

    #[test]
    fn point14_compression_level_increases_chunk_target_at_high_levels() {
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 100;
        cfg.compression_level = 9;
        cfg.las.point_data_format = PointDataFormat::Pdrf7;

        let writer = LazWriter {
            inner: Cursor::new(Vec::<u8>::new()),
            config: cfg,
            chunk_buf: Vec::new(),
            chunk_table: ChunkTable::default(),
            cumulative_offset: 0,
            payload_mode: LazPayloadMode::StandardPoint14,
            chunk_table_ptr_pos: None,
            standard_chunk_entries: Vec::new(),
            _las_header_pos: 0,
            total_points: 0,
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
            min_z: f64::MAX,
            max_z: f64::MIN,
        };

        assert_eq!(writer.effective_chunk_size(), 200);
    }

    #[test]
    fn point10_chunk_target_ignores_compression_level_for_now() {
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 123;
        cfg.compression_level = 9;
        cfg.las.point_data_format = PointDataFormat::Pdrf3;

        let writer = LazWriter {
            inner: Cursor::new(Vec::<u8>::new()),
            config: cfg,
            chunk_buf: Vec::new(),
            chunk_table: ChunkTable::default(),
            cumulative_offset: 0,
            payload_mode: LazPayloadMode::StandardPoint10,
            chunk_table_ptr_pos: None,
            standard_chunk_entries: Vec::new(),
            _las_header_pos: 0,
            total_points: 0,
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
            min_z: f64::MAX,
            max_z: f64::MIN,
        };

        assert_eq!(writer.effective_chunk_size(), 123);
    }

    #[test]
    fn standards_compliant_point14_roundtrip_reads_via_lazreader() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf7;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(42.0)),
                color: Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 4.0,
                y: 5.0,
                z: 6.0,
                intensity: 101,
                classification: 3,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(43.0)),
                color: Some(Rgb16 { red: 1100, green: 2100, blue: 3100 }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert!((out[0].x - 1.0).abs() < 1e-9);
        assert!((out[0].y - 2.0).abs() < 1e-9);
        assert!((out[0].z - 3.0).abs() < 1e-9);
        assert_eq!(out[0].classification, 2);
        assert_eq!(out[1].classification, 3);
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_roundtrip_reads_via_lazreader() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf3;
        cfg.las.extra_bytes_per_point = 2;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut p1 = PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(42.0)),
                color: Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }),
                ..PointRecord::default()
            };
            p1.extra_bytes.data[0] = 7;
            p1.extra_bytes.data[1] = 9;
            p1.extra_bytes.len = 2;
            writer.write_point(&p1)?;

            let mut p2 = p1;
            p2.x = 4.0;
            p2.y = 5.0;
            p2.z = 6.0;
            p2.intensity = 101;
            p2.classification = 3;
            p2.gps_time = Some(GpsTime(43.0));
            p2.color = Some(Rgb16 { red: 1100, green: 2100, blue: 3100 });
            p2.extra_bytes.data[0] = 8;
            writer.write_point(&p2)?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].classification, 2);
        assert_eq!(out[1].classification, 3);
        assert_eq!(out[0].color, Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }));
        assert_eq!(out[1].color, Some(Rgb16 { red: 1100, green: 2100, blue: 3100 }));
        assert_eq!(out[0].extra_bytes.len, 2);
        assert_eq!(out[1].extra_bytes.len, 2);
        assert_eq!(out[0].extra_bytes.data[0], 7);
        assert_eq!(out[1].extra_bytes.data[0], 8);
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_pdrf5_roundtrip_preserves_waveform() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf5;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(42.0)),
                color: Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }),
                waveform: Some(WaveformPacket {
                    descriptor_index: 3,
                    byte_offset: 1234,
                    packet_size: 56,
                    return_point_location: 0.25,
                    dx: 0.1,
                    dy: 0.2,
                    dz: 0.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].waveform.map(|w| w.descriptor_index), Some(3));
        assert_eq!(out[0].waveform.map(|w| w.packet_size), Some(56));
        assert_eq!(out[0].waveform.map(|w| w.byte_offset), Some(1234));
        Ok(())
    }

    #[test]
    fn standards_compliant_point14_pdrf10_roundtrip_preserves_waveform() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf10;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 200,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(142.0)),
                color: Some(Rgb16 { red: 2000, green: 3000, blue: 4000 }),
                waveform: Some(WaveformPacket {
                    descriptor_index: 7,
                    byte_offset: 98765,
                    packet_size: 88,
                    return_point_location: 0.5,
                    dx: 1.1,
                    dy: 1.2,
                    dz: 1.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].waveform.map(|w| w.descriptor_index), Some(7));
        assert_eq!(out[0].waveform.map(|w| w.packet_size), Some(88));
        assert_eq!(out[0].waveform.map(|w| w.byte_offset), Some(98765));
        Ok(())
    }

    #[test]
    fn standards_compliant_point14_pdrf9_roundtrip_preserves_waveform() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf9;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 15.0,
                y: 25.0,
                z: 35.0,
                intensity: 220,
                classification: 3,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(242.0)),
                waveform: Some(WaveformPacket {
                    descriptor_index: 9,
                    byte_offset: 34567,
                    packet_size: 99,
                    return_point_location: 0.6,
                    dx: 2.1,
                    dy: 2.2,
                    dz: 2.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].waveform.map(|w| w.descriptor_index), Some(9));
        assert_eq!(out[0].waveform.map(|w| w.packet_size), Some(99));
        assert_eq!(out[0].waveform.map(|w| w.byte_offset), Some(34567));
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_pdrf0_roundtrip_reads_via_lazreader() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf0;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 11.0,
                y: 21.0,
                z: 31.0,
                intensity: 11,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].intensity, 10);
        assert_eq!(out[1].intensity, 11);
        assert_eq!(out[0].classification, 1);
        assert_eq!(out[1].classification, 2);
        assert!(out[0].gps_time.is_none());
        assert!(out[0].color.is_none());
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_pdrf1_roundtrip_reads_via_lazreader() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf1;
        cfg.las.extra_bytes_per_point = 1;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut p1 = PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(50.0)),
                ..PointRecord::default()
            };
            p1.extra_bytes.data[0] = 3;
            p1.extra_bytes.len = 1;
            writer.write_point(&p1)?;

            let mut p2 = p1;
            p2.x = 11.0;
            p2.y = 21.0;
            p2.z = 31.0;
            p2.intensity = 101;
            p2.gps_time = Some(GpsTime(51.0));
            p2.extra_bytes.data[0] = 9;
            writer.write_point(&p2)?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].gps_time, Some(GpsTime(50.0)));
        assert_eq!(out[1].gps_time, Some(GpsTime(51.0)));
        assert!(out[0].color.is_none());
        assert_eq!(out[0].extra_bytes.len, 1);
        assert_eq!(out[1].extra_bytes.len, 1);
        assert_eq!(out[0].extra_bytes.data[0], 3);
        assert_eq!(out[1].extra_bytes.data[0], 9);
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_pdrf2_roundtrip_reads_via_lazreader() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf2;
        cfg.las.extra_bytes_per_point = 1;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut p1 = PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 100,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                color: Some(Rgb16 {
                    red: 1000,
                    green: 2000,
                    blue: 3000,
                }),
                ..PointRecord::default()
            };
            p1.extra_bytes.data[0] = 4;
            p1.extra_bytes.len = 1;
            writer.write_point(&p1)?;

            let mut p2 = p1;
            p2.x = 11.0;
            p2.y = 21.0;
            p2.z = 31.0;
            p2.intensity = 101;
            p2.color = Some(Rgb16 {
                red: 1001,
                green: 2001,
                blue: 3001,
            });
            p2.extra_bytes.data[0] = 8;
            writer.write_point(&p2)?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert!(out[0].gps_time.is_none());
        assert_eq!(
            out[0].color,
            Some(Rgb16 {
                red: 1000,
                green: 2000,
                blue: 3000,
            })
        );
        assert_eq!(
            out[1].color,
            Some(Rgb16 {
                red: 1001,
                green: 2001,
                blue: 3001,
            })
        );
        assert_eq!(out[0].extra_bytes.len, 1);
        assert_eq!(out[1].extra_bytes.len, 1);
        assert_eq!(out[0].extra_bytes.data[0], 4);
        assert_eq!(out[1].extra_bytes.data[0], 8);
        Ok(())
    }

    #[test]
    fn standards_compliant_supports_waveform_point10_formats() {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.las.point_data_format = PointDataFormat::Pdrf5;
        assert!(LazWriter::new(&mut sink, cfg).is_ok());
    }

    #[test]
    fn standards_compliant_supports_singleton_point14_extra_bytes_when_declared() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.las.point_data_format = PointDataFormat::Pdrf8;
        cfg.las.extra_bytes_per_point = 2;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut p = PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                color: Some(Rgb16 {
                    red: 100,
                    green: 200,
                    blue: 300,
                }),
                nir: Some(400),
                ..PointRecord::default()
            };
            p.extra_bytes.data[0] = 7;
            p.extra_bytes.data[1] = 9;
            p.extra_bytes.len = 2;
            writer.write_point(&p)?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].extra_bytes.len, 2);
        assert_eq!(out[0].extra_bytes.data[0], 7);
        assert_eq!(out[0].extra_bytes.data[1], 9);
        Ok(())
    }

    #[test]
    fn standards_compliant_rejects_point_payload_extra_bytes() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.las.point_data_format = PointDataFormat::Pdrf8;

        let mut writer = LazWriter::new(&mut sink, cfg)?;
        let mut p = PointRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            intensity: 10,
            classification: 1,
            return_number: 1,
            number_of_returns: 1,
            gps_time: Some(GpsTime(1.0)),
            color: Some(Rgb16 {
                red: 100,
                green: 200,
                blue: 300,
            }),
            nir: Some(400),
            ..PointRecord::default()
        };
        p.extra_bytes.data[0] = 42;
        p.extra_bytes.len = 1;

        let err = writer
            .write_point(&p)
            .expect_err("expected standards-mode payload extra-bytes rejection");
        assert!(format!("{err}").contains("unless extra_bytes_per_point is declared"));
        Ok(())
    }

    #[test]
    fn standards_compliant_supports_constant_multi_point_chunk_with_extra_bytes() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;
        cfg.las.extra_bytes_per_point = 2;

        let mut writer = LazWriter::new(&mut sink, cfg)?;
        let mut p1 = PointRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            intensity: 10,
            classification: 1,
            return_number: 1,
            number_of_returns: 1,
            gps_time: Some(GpsTime(1.0)),
            color: Some(Rgb16 {
                red: 100,
                green: 200,
                blue: 300,
            }),
            nir: Some(400),
            ..PointRecord::default()
        };
        p1.extra_bytes.data[0] = 1;
        p1.extra_bytes.data[1] = 2;
        p1.extra_bytes.len = 2;
        writer.write_point(&p1)?;

        let mut p2 = p1;
        p2.x = 4.0;
        p2.y = 5.0;
        p2.z = 6.0;
        writer.write_point(&p2)?;
        writer.finish()?;

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].extra_bytes.len, 2);
        assert_eq!(out[1].extra_bytes.len, 2);
        assert_eq!(out[0].extra_bytes.data[0], 1);
        assert_eq!(out[1].extra_bytes.data[0], 1);
        assert_eq!(out[0].extra_bytes.data[1], 2);
        assert_eq!(out[1].extra_bytes.data[1], 2);
        Ok(())
    }

    #[test]
    fn standards_compliant_supports_varying_multi_point_chunk_extra_bytes() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;
        cfg.las.extra_bytes_per_point = 2;

        let mut writer = LazWriter::new(&mut sink, cfg)?;
        let mut p1 = PointRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            intensity: 10,
            classification: 1,
            return_number: 1,
            number_of_returns: 1,
            gps_time: Some(GpsTime(1.0)),
            color: Some(Rgb16 {
                red: 100,
                green: 200,
                blue: 300,
            }),
            nir: Some(400),
            ..PointRecord::default()
        };
        p1.extra_bytes.data[0] = 1;
        p1.extra_bytes.data[1] = 2;
        p1.extra_bytes.len = 2;
        writer.write_point(&p1)?;

        let mut p2 = p1;
        p2.x = 4.0;
        p2.y = 5.0;
        p2.z = 6.0;
        p2.extra_bytes.data[0] = 9;
        writer.write_point(&p2)?;
        writer.finish()?;

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].extra_bytes.len, 2);
        assert_eq!(out[1].extra_bytes.len, 2);
        assert_eq!(out[0].extra_bytes.data[0], 1);
        assert_eq!(out[0].extra_bytes.data[1], 2);
        assert_eq!(out[1].extra_bytes.data[0], 9);
        assert_eq!(out[1].extra_bytes.data[1], 2);
        Ok(())
    }

    #[test]
    fn standards_compliant_writes_valid_chunk_table_pointer_and_entries() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf6;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 4.0,
                y: 5.0,
                z: 6.0,
                intensity: 11,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(2.0)),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 7.0,
                y: 8.0,
                z: 9.0,
                intensity: 12,
                classification: 3,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(3.0)),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let bytes = sink.into_inner();
        let las = LasReader::new(Cursor::new(bytes.clone()))?;
        let point_data_start = las.offset_to_point_data();

        let mut raw = Cursor::new(bytes);
        raw.set_position(point_data_start);
        let chunk_table_offset = le::read_u64(&mut raw)?;
        assert!(chunk_table_offset > point_data_start + 8);

        raw.set_position(chunk_table_offset);
        let version = le::read_u32(&mut raw)?;
        let chunk_count = le::read_u32(&mut raw)?;
        assert_eq!(version, 0);
        assert_eq!(chunk_count, 2);

        let entries = read_laszip_chunk_table_entries(&mut raw, chunk_count, false)?;
        assert_eq!(entries.len(), 2);
        // Fixed chunk-size tables do not store point counts; byte counts must be > 0.
        assert!(entries.iter().all(|e| e.byte_count > 0));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf8_roundtrip_preserves_rgb_nir() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 3;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 77,
                classification: 4,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(100.5)),
                color: Some(Rgb16 { red: 1200, green: 2200, blue: 3200 }),
                nir: Some(4200),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 11.0,
                y: 21.0,
                z: 31.0,
                intensity: 78,
                classification: 5,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(101.5)),
                color: Some(Rgb16 { red: 1300, green: 2300, blue: 3300 }),
                nir: Some(4300),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].nir, Some(4200));
        assert_eq!(out[1].nir, Some(4300));
        assert_eq!(out[0].color, Some(Rgb16 { red: 1200, green: 2200, blue: 3200 }));
        assert_eq!(out[1].color, Some(Rgb16 { red: 1300, green: 2300, blue: 3300 }));
        Ok(())
    }

    #[test]
    fn standards_compliant_chunk_table_point_counts_match_chunk_boundaries() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            for i in 0..5u16 {
                writer.write_point(&PointRecord {
                    x: i as f64,
                    y: 100.0 + i as f64,
                    z: 200.0 + i as f64,
                    intensity: 500 + i,
                    classification: (i % 5) as u8,
                    return_number: 1,
                    number_of_returns: 1,
                    gps_time: Some(GpsTime(10.0 + i as f64)),
                    color: Some(Rgb16 {
                        red: 1000 + i,
                        green: 2000 + i,
                        blue: 3000 + i,
                    }),
                    nir: Some(4000 + i),
                    ..PointRecord::default()
                })?;
            }
            writer.finish()?;
        }

        let bytes = sink.into_inner();
        let las = LasReader::new(Cursor::new(bytes.clone()))?;
        let point_data_start = las.offset_to_point_data();

        let mut raw = Cursor::new(bytes);
        raw.set_position(point_data_start);
        let chunk_table_offset = le::read_u64(&mut raw)?;
        raw.set_position(chunk_table_offset);
        let version = le::read_u32(&mut raw)?;
        let chunk_count = le::read_u32(&mut raw)?;
        assert_eq!(version, 0);
        assert_eq!(chunk_count, 3);

        let entries = read_laszip_chunk_table_entries(&mut raw, chunk_count, false)?;
        assert_eq!(entries.len(), 3);
        // Fixed chunk-size tables store only byte counts, not point counts.
        assert!(entries.iter().all(|e| e.byte_count > 0));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf8_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 12345;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 50,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(7.0)),
                color: Some(Rgb16 { red: 100, green: 200, blue: 300 }),
                nir: Some(400),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::LayeredChunked);
        assert_eq!(info.chunk_size, 12345);
        assert!(info.has_point14_item());
        assert!(!info.has_rgb14_item());
        assert!(info.has_rgbnir14_item());
        assert!(info.has_nir14_item());
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf3_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 321;
        cfg.las.point_data_format = PointDataFormat::Pdrf3;
        cfg.las.extra_bytes_per_point = 2;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut point = PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 50,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(7.0)),
                color: Some(Rgb16 { red: 100, green: 200, blue: 300 }),
                ..PointRecord::default()
            };
            point.extra_bytes.data[0] = 11;
            point.extra_bytes.data[1] = 22;
            point.extra_bytes.len = 2;
            writer.write_point(&point)?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::PointWiseChunked);
        assert_eq!(info.chunk_size, 321);
        assert!(info.has_point10_item());
        assert!(info.items.iter().any(|i| i.item_type == 7 && i.item_size == 8 && i.item_version == 2));
        assert!(info.items.iter().any(|i| i.item_type == 8 && i.item_size == 6 && i.item_version == 2));
        assert!(info.items.iter().any(|i| i.item_type == 0 && i.item_size == 2 && i.item_version == 2));
        Ok(())
    }

    #[test]
    fn standards_compliant_point10_writes_pointwise_chunk_table_entries() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf1;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            for i in 0..3 {
                writer.write_point(&PointRecord {
                    x: i as f64,
                    y: 100.0 + i as f64,
                    z: 200.0 + i as f64,
                    intensity: 10 + i as u16,
                    classification: 1,
                    return_number: 1,
                    number_of_returns: 1,
                    gps_time: Some(GpsTime(1.0 + i as f64)),
                    ..PointRecord::default()
                })?;
            }
            writer.finish()?;
        }

        let bytes = sink.into_inner();
        let las = LasReader::new(Cursor::new(bytes.clone()))?;
        let point_data_start = las.offset_to_point_data();

        let mut raw = Cursor::new(bytes);
        raw.set_position(point_data_start);
        let chunk_table_offset = le::read_u64(&mut raw)?;
        raw.set_position(chunk_table_offset);
        let version = le::read_u32(&mut raw)?;
        let chunk_count = le::read_u32(&mut raw)?;
        assert_eq!(version, 0);
        assert_eq!(chunk_count, 2);

        let entries = read_laszip_chunk_table_entries(&mut raw, chunk_count, false)?;
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.byte_count > 0));
        assert!(entries.iter().all(|e| e.point_count == 0));
        Ok(())
    }

    #[test]
    fn standards_compliant_declares_byte14_item_when_extra_bytes_configured() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 123;
        cfg.las.point_data_format = PointDataFormat::Pdrf8;
        cfg.las.extra_bytes_per_point = 2;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            let mut p = PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 50,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(7.0)),
                color: Some(Rgb16 { red: 100, green: 200, blue: 300 }),
                nir: Some(400),
                ..PointRecord::default()
            };
            p.extra_bytes.data[0] = 11;
            p.extra_bytes.data[1] = 22;
            p.extra_bytes.len = 2;
            writer.write_point(&p)?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert!(info.items.iter().any(|i| i.item_type == 14 && i.item_size == 2 && i.item_version == 3));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf6_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 777;
        cfg.las.point_data_format = PointDataFormat::Pdrf6;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::LayeredChunked);
        assert_eq!(info.chunk_size, 777);
        assert!(info.has_point14_item());
        assert!(!info.has_rgb14_item());
        assert!(!info.has_nir14_item());
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf7_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 888;
        cfg.las.point_data_format = PointDataFormat::Pdrf7;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                color: Some(Rgb16 {
                    red: 100,
                    green: 200,
                    blue: 300,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::LayeredChunked);
        assert_eq!(info.chunk_size, 888);
        assert!(info.has_point14_item());
        assert!(info.has_rgb14_item());
        assert!(!info.has_nir14_item());
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf4_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 444;
        cfg.las.point_data_format = PointDataFormat::Pdrf4;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                waveform: Some(WaveformPacket {
                    descriptor_index: 1,
                    byte_offset: 2,
                    packet_size: 3,
                    return_point_location: 0.4,
                    dx: 0.1,
                    dy: 0.2,
                    dz: 0.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::PointWiseChunked);
        assert_eq!(info.chunk_size, 444);
        assert!(info.has_point10_item());
        assert!(info.items.iter().any(|i| i.item_type == 7 && i.item_size == 8 && i.item_version == 2));
        assert!(info.items.iter().any(|i| i.item_type == 0 && i.item_size == 29 && i.item_version == 2));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf5_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 555;
        cfg.las.point_data_format = PointDataFormat::Pdrf5;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                color: Some(Rgb16 {
                    red: 100,
                    green: 200,
                    blue: 300,
                }),
                waveform: Some(WaveformPacket {
                    descriptor_index: 1,
                    byte_offset: 2,
                    packet_size: 3,
                    return_point_location: 0.4,
                    dx: 0.1,
                    dy: 0.2,
                    dz: 0.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::PointWiseChunked);
        assert_eq!(info.chunk_size, 555);
        assert!(info.has_point10_item());
        assert!(info.items.iter().any(|i| i.item_type == 7 && i.item_size == 8 && i.item_version == 2));
        assert!(info.items.iter().any(|i| i.item_type == 8 && i.item_size == 6 && i.item_version == 2));
        assert!(info.items.iter().any(|i| i.item_type == 0 && i.item_size == 29 && i.item_version == 2));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf9_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 999;
        cfg.las.point_data_format = PointDataFormat::Pdrf9;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                waveform: Some(WaveformPacket {
                    descriptor_index: 1,
                    byte_offset: 2,
                    packet_size: 3,
                    return_point_location: 0.4,
                    dx: 0.1,
                    dy: 0.2,
                    dz: 0.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::LayeredChunked);
        assert_eq!(info.chunk_size, 999);
        assert!(info.has_point14_item());
        assert!(!info.has_rgb14_item());
        assert!(!info.has_nir14_item());
        assert!(info.items.iter().any(|i| i.item_type == 14 && i.item_size == 29 && i.item_version == 3));
        Ok(())
    }

    #[test]
    fn standards_compliant_pdrf10_vlr_layout_matches_point_format() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 1010;
        cfg.las.point_data_format = PointDataFormat::Pdrf10;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 10,
                classification: 1,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1.0)),
                color: Some(Rgb16 {
                    red: 100,
                    green: 200,
                    blue: 300,
                }),
                waveform: Some(WaveformPacket {
                    descriptor_index: 1,
                    byte_offset: 2,
                    packet_size: 3,
                    return_point_location: 0.4,
                    dx: 0.1,
                    dy: 0.2,
                    dz: 0.3,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        let las = LasReader::new(Cursor::new(sink.into_inner()))?;
        let info = parse_laszip_vlr(las.vlrs()).ok_or_else(|| crate::Error::InvalidValue {
            field: "laz.vlr",
            detail: "LASzip VLR missing in standards output".to_string(),
        })?;

        assert_eq!(info.compressor, LaszipCompressorType::LayeredChunked);
        assert_eq!(info.chunk_size, 1010);
        assert!(info.has_point14_item());
        assert!(info.has_rgb14_item());
        assert!(!info.has_nir14_item());
        assert!(info.items.iter().any(|i| i.item_type == 14 && i.item_size == 29 && i.item_version == 3));
        Ok(())
    }

    // ── LAS 1.5 (PDRF 11-15) roundtrip tests ────────────────────────────────

    #[test]
    fn laz_pdrf11_roundtrip() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf11;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                intensity: 100,
                classification: 5,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(42.0)),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 4.0,
                y: 5.0,
                z: 6.0,
                intensity: 200,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(43.0)),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert!((out[0].x - 1.0).abs() < 1e-3);
        assert_eq!(out[0].classification, 5);
        assert!(out[0].color.is_none());
        assert!(out[0].waveform.is_none());
        assert_eq!(out[1].classification, 2);
        Ok(())
    }

    #[test]
    fn laz_pdrf12_roundtrip() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf12;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 10.0,
                y: 20.0,
                z: 30.0,
                intensity: 111,
                classification: 3,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1000.0)),
                color: Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 11.0,
                y: 21.0,
                z: 31.0,
                intensity: 222,
                classification: 4,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(1001.0)),
                color: Some(Rgb16 { red: 1100, green: 2100, blue: 3100 }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert!((out[0].x - 10.0).abs() < 1e-3);
        assert_eq!(out[0].classification, 3);
        assert_eq!(out[0].color, Some(Rgb16 { red: 1000, green: 2000, blue: 3000 }));
        assert_eq!(out[1].color, Some(Rgb16 { red: 1100, green: 2100, blue: 3100 }));
        Ok(())
    }

    #[test]
    fn laz_pdrf13_roundtrip_preserves_rgb_and_nir() -> Result<()> {
        // ThermalRGB is not preserved by the LAZ codec (handled outside the
        // compressed stream); only base fields + RGB + NIR are tested here.
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf13;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 5.0,
                y: 6.0,
                z: 7.0,
                intensity: 333,
                classification: 2,
                return_number: 1,
                number_of_returns: 2,
                gps_time: Some(GpsTime(500.0)),
                color: Some(Rgb16 { red: 5000, green: 6000, blue: 7000 }),
                nir: Some(8000),
                ..PointRecord::default()
            })?;
            writer.write_point(&PointRecord {
                x: 8.0,
                y: 9.0,
                z: 10.0,
                intensity: 444,
                classification: 3,
                return_number: 2,
                number_of_returns: 2,
                gps_time: Some(GpsTime(501.0)),
                color: Some(Rgb16 { red: 5100, green: 6100, blue: 7100 }),
                nir: Some(8100),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 2);
        assert_eq!(out[0].color, Some(Rgb16 { red: 5000, green: 6000, blue: 7000 }));
        assert_eq!(out[0].nir, Some(8000));
        assert_eq!(out[1].color, Some(Rgb16 { red: 5100, green: 6100, blue: 7100 }));
        assert_eq!(out[1].nir, Some(8100));
        Ok(())
    }

    #[test]
    fn laz_pdrf14_roundtrip_preserves_rgb_and_waveform() -> Result<()> {
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf14;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 20.0,
                y: 21.0,
                z: 22.0,
                intensity: 555,
                classification: 2,
                return_number: 1,
                number_of_returns: 1,
                gps_time: Some(GpsTime(200.0)),
                color: Some(Rgb16 { red: 2000, green: 3000, blue: 4000 }),
                waveform: Some(WaveformPacket {
                    descriptor_index: 5,
                    byte_offset: 55555,
                    packet_size: 100,
                    return_point_location: 0.75,
                    dx: 0.5,
                    dy: 0.6,
                    dz: 0.7,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].color, Some(Rgb16 { red: 2000, green: 3000, blue: 4000 }));
        assert!(out[0].nir.is_none());
        let wf = out[0].waveform.expect("waveform must survive LAZ roundtrip");
        assert_eq!(wf.descriptor_index, 5);
        assert_eq!(wf.byte_offset, 55555);
        assert_eq!(wf.packet_size, 100);
        Ok(())
    }

    #[test]
    fn laz_pdrf15_roundtrip_preserves_rgb_nir_and_waveform() -> Result<()> {
        // ThermalRGB is not preserved by the LAZ codec; only RGB + NIR + waveform are tested.
        let mut sink = Cursor::new(Vec::<u8>::new());
        let mut cfg = LazWriterConfig::default();
        cfg.chunk_size = 2;
        cfg.las.point_data_format = PointDataFormat::Pdrf15;

        {
            let mut writer = LazWriter::new(&mut sink, cfg)?;
            writer.write_point(&PointRecord {
                x: 50.0,
                y: 60.0,
                z: 70.0,
                intensity: 999,
                classification: 6,
                return_number: 2,
                number_of_returns: 3,
                gps_time: Some(GpsTime(3000.0)),
                color: Some(Rgb16 { red: 60000, green: 50000, blue: 40000 }),
                nir: Some(55000),
                waveform: Some(WaveformPacket {
                    descriptor_index: 9,
                    byte_offset: 77777,
                    packet_size: 256,
                    return_point_location: 0.1,
                    dx: 0.01,
                    dy: 0.02,
                    dz: 0.03,
                }),
                ..PointRecord::default()
            })?;
            writer.finish()?;
        }

        sink.set_position(0);
        let mut reader = LazReader::new(sink)?;
        let out = reader.read_all()?;

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].color, Some(Rgb16 { red: 60000, green: 50000, blue: 40000 }));
        assert_eq!(out[0].nir, Some(55000));
        let wf = out[0].waveform.expect("waveform must survive LAZ roundtrip");
        assert_eq!(wf.descriptor_index, 9);
        assert_eq!(wf.byte_offset, 77777);
        assert_eq!(wf.packet_size, 256);
        Ok(())
    }
}
