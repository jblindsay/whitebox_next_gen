//! E57 writer.
//!
//! Accumulates points in memory, then on `finish()`:
//! 1. Writes the 48-byte file header.
//! 2. Writes the binary point data in CRC-protected 1024-byte pages.
//! 3. Writes the XML section in CRC-protected pages.
//! 4. Back-patches the file header with correct offsets and lengths.

use std::io::{Seek, SeekFrom, Write};
use crate::crs::{ogc_wkt_from_epsg, Crs};
use crate::e57::page::PageWriter;
use crate::e57::xml::build_xml;
use crate::e57::{E57_SIGNATURE, PAGE_SIZE};
use crate::io::{le, PointWriter};
use crate::point::PointRecord;
use crate::Result;

/// Configuration for the E57 writer.
#[derive(Debug, Clone)]
pub struct E57WriterConfig {
    /// Point cloud name embedded in the XML.
    pub name: String,
    /// GUID for the point cloud (leave empty to auto-generate a placeholder).
    pub guid: String,
    /// Include intensity in the output.
    pub has_intensity: bool,
    /// Include RGB colour in the output.
    pub has_color: bool,
    /// Optional CRS description per ASTM E2807 §8.4.6 `coordinateMetadata`.
    /// Typically an OGC WKT2 string or an EPSG authority string such as
    /// `"EPSG:32617"`.  When `None` the element is omitted.
    pub coordinate_metadata: Option<String>,
    /// Optional CRS.  When set and `coordinate_metadata` is `None`, the WKT
    /// string (or `"EPSG:<code>"` fallback) is used as `coordinateMetadata`.
    /// Mirrors the CRS pattern in the LAS/COPC writers.
    pub crs: Option<Crs>,
}

impl Default for E57WriterConfig {
    fn default() -> Self {
        E57WriterConfig {
            name: "Point Cloud".to_owned(),
            guid: "{00000000-0000-0000-0000-000000000000}".to_owned(),
            has_intensity: false,
            has_color: false,
            coordinate_metadata: None,
            crs: None,
        }
    }
}

/// An E57 writer.  All points are buffered until `finish()` is called.
pub struct E57Writer<W: Write + Seek> {
    inner: W,
    config: E57WriterConfig,
    points: Vec<PointRecord>,
}

impl<W: Write + Seek> E57Writer<W> {
    /// Create a new E57 writer.
    pub fn new(inner: W, config: E57WriterConfig) -> Self {
        E57Writer { inner, config, points: Vec::new() }
    }
}

impl<W: Write + Seek> PointWriter for E57Writer<W> {
    fn write_point(&mut self, p: &PointRecord) -> Result<()> {
        self.points.push(*p);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        let point_count = self.points.len() as u64;
        let has_i = self.config.has_intensity;
        let has_c = self.config.has_color;

        // ── Step 1: Write placeholder file header (48 bytes) ─────────────
        self.inner.seek(SeekFrom::Start(0))?;
        self.inner.write_all(E57_SIGNATURE)?;
        le::write_u32(&mut self.inner, 1)?; // version major
        le::write_u32(&mut self.inner, 0)?; // version minor
        le::write_u64(&mut self.inner, 0)?; // file_length placeholder
        le::write_u64(&mut self.inner, 0)?; // xml_offset placeholder
        le::write_u64(&mut self.inner, 0)?; // xml_length placeholder
        le::write_u64(&mut self.inner, PAGE_SIZE as u64)?; // page_width
        le::write_u64(&mut self.inner, PAGE_SIZE as u64)?; // page_length

        // ── Step 2: Write binary point data in pages ──────────────────────
        let binary_offset = self.inner.stream_position()?;
        let record_size = point_record_size(has_i, has_c);
        let _ = record_size;

        let mut page_writer = PageWriter::new(&mut self.inner);
        for p in &self.points {
            let bytes = encode_point(p, has_i, has_c);
            page_writer.write_bytes(&bytes)?;
        }
        page_writer.flush_final()?;

        // ── Step 3: Write XML in pages ────────────────────────────────────
        let xml_offset = self.inner.stream_position()?;
        // Resolve coordinate_metadata: explicit string wins; then CRS WKT;
        // then EPSG authority string; then omit.
        let crs_string: Option<String> = self.config.coordinate_metadata.clone()
            .or_else(|| self.config.crs.as_ref().and_then(|c| c.wkt.clone()))
            .or_else(|| self.config.crs.as_ref().and_then(|c| c.epsg)
                .and_then(ogc_wkt_from_epsg)
            )
            .or_else(|| self.config.crs.as_ref().and_then(|c| c.epsg)
                .map(|e| format!("EPSG:{e}"))
            );

        let xml = build_xml(
            point_count, binary_offset,
            has_i, has_c,
            &self.config.guid, &self.config.name,
            crs_string.as_deref(),
        );
        let xml_bytes = xml.as_bytes();
        let xml_length = xml_bytes.len() as u64;

        let mut xml_page_writer = PageWriter::new(&mut self.inner);
        xml_page_writer.write_bytes(xml_bytes)?;
        xml_page_writer.flush_final()?;

        // ── Step 4: Back-patch file header ────────────────────────────────
        let file_length = self.inner.stream_position()?;
        self.inner.seek(SeekFrom::Start(0))?;
        self.inner.write_all(E57_SIGNATURE)?;
        le::write_u32(&mut self.inner, 1)?;
        le::write_u32(&mut self.inner, 0)?;
        le::write_u64(&mut self.inner, file_length)?;
        le::write_u64(&mut self.inner, xml_offset)?;
        le::write_u64(&mut self.inner, xml_length)?;
        le::write_u64(&mut self.inner, PAGE_SIZE as u64)?;
        le::write_u64(&mut self.inner, PAGE_SIZE as u64)?;

        self.inner.seek(SeekFrom::Start(file_length))?;
        Ok(())
    }
}

// ── Point encoding helpers ────────────────────────────────────────────────────

fn encode_point(p: &PointRecord, has_i: bool, has_c: bool) -> Vec<u8> {
    let mut v = Vec::with_capacity(32);
    v.extend_from_slice(&p.x.to_le_bytes());
    v.extend_from_slice(&p.y.to_le_bytes());
    v.extend_from_slice(&p.z.to_le_bytes());
    if has_i {
        let intensity = f32::from(p.intensity) / 65535.0;
        v.extend_from_slice(&intensity.to_le_bytes());
    }
    if has_c {
        let r = p.color.map_or(0, |c| (c.red >> 8) as u8);
        let g = p.color.map_or(0, |c| (c.green >> 8) as u8);
        let b = p.color.map_or(0, |c| (c.blue >> 8) as u8);
        v.push(r); v.push(g); v.push(b);
    }
    v
}

fn point_record_size(has_i: bool, has_c: bool) -> usize {
    let mut size = 24; // 3 × f64
    if has_i { size += 4; } // f32 intensity
    if has_c { size += 3; } // u8 r/g/b
    size
}
