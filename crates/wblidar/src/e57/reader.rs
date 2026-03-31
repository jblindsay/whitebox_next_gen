//! E57 reader.

use std::io::{Read, Seek, SeekFrom};
use crate::e57::page::PageReader;
use crate::e57::xml::{parse_point_clouds, E57FieldType, PointCloudMeta};
use crate::e57::E57_SIGNATURE;
use crate::io::{le, PointReader};
use crate::point::{PointRecord, Rgb16};
use crate::{Error, Result};

/// File header (48 bytes) at offset 0 of every E57 file.
#[derive(Debug)]
struct E57FileHeader {
    xml_offset: u64,
    xml_length:  u64,
    _page_width:  u64,
    _page_length: u64,
}

impl E57FileHeader {
    fn read<R: Read>(r: &mut R) -> Result<Self> {
        let mut sig = [0u8; 8];
        r.read_exact(&mut sig)?;
        if &sig != E57_SIGNATURE {
            return Err(Error::InvalidSignature { format: "E57", found: sig.to_vec() });
        }
        let _major      = le::read_u32(r)?;
        let _minor      = le::read_u32(r)?;
        let _file_len   = le::read_u64(r)?;
        let xml_offset  = le::read_u64(r)?;
        let xml_length  = le::read_u64(r)?;
        let page_width  = le::read_u64(r)?;
        let page_length = le::read_u64(r)?;
        Ok(E57FileHeader { xml_offset, xml_length, _page_width: page_width, _page_length: page_length })
    }
}

/// An E57 sequential reader for the first point cloud in the file.
pub struct E57Reader<R: Read + Seek> {
    _inner: R,
    meta: PointCloudMeta,
    points_read: u64,
    _page_reader: Option<PageReader<std::io::Take<std::io::Cursor<Vec<u8>>>>>,
    raw_data: Vec<u8>,
    raw_pos: usize,
    record_size: usize,
}

impl<R: Read + Seek> E57Reader<R> {
    /// Open and parse an E57 file.  Loads the XML section and positions the
    /// reader at the start of the first point cloud's binary data.
    pub fn new(mut inner: R) -> Result<Self> {
        // Read file header at offset 0
        inner.seek(SeekFrom::Start(0))?;
        let file_hdr = E57FileHeader::read(&mut inner)?;

        // Read and validate XML section (pages)
        inner.seek(SeekFrom::Start(file_hdr.xml_offset))?;
        let xml = read_paged_xml(&mut inner, file_hdr.xml_length as usize)?;

        // Parse point clouds from XML
        let clouds = parse_point_clouds(&xml);
        let meta = clouds.into_iter().next().ok_or_else(|| Error::InvalidValue {
            field: "e57_data3D",
            detail: "no data3D point clouds found in E57 XML".to_owned(),
        })?;

        // Pre-load the binary section into memory for paged reading.
        inner.seek(SeekFrom::Start(meta.file_offset))?;
        let total_bytes = estimate_binary_size(&meta);
        let mut raw_data = Vec::with_capacity(total_bytes);
        // Read page-by-page, discarding CRCs
        let pages_needed = (total_bytes + crate::e57::PAGE_PAYLOAD - 1) / crate::e57::PAGE_PAYLOAD;
        for _ in 0..pages_needed {
            match crate::e57::page::read_page(&mut inner) {
                Ok(page) => raw_data.extend_from_slice(&page),
                Err(_) => break,
            }
        }

        let record_size = record_byte_size(&meta);

        Ok(E57Reader {
            _inner: inner, meta, points_read: 0,
            _page_reader: None,
            raw_data, raw_pos: 0,
            record_size,
        })
    }

    /// Return the point cloud metadata (fields, name, etc.).
    pub fn meta(&self) -> &PointCloudMeta { &self.meta }
}

impl<R: Read + Seek> PointReader for E57Reader<R> {
    fn read_point(&mut self, out: &mut PointRecord) -> Result<bool> {
        if self.points_read >= self.meta.record_count { return Ok(false); }

        let end = self.raw_pos + self.record_size;
        if end > self.raw_data.len() { return Ok(false); }

        let record = &self.raw_data[self.raw_pos..end];
        self.raw_pos = end;

        *out = PointRecord::default();
        let mut offset = 0;

        for field in &self.meta.fields {
            let bw = field.dtype.byte_width(field.minimum, field.maximum);
            if offset + bw > record.len() { break; }
            let raw_val = read_raw_int(&record[offset..offset + bw], bw);
            offset += bw;

            let physical = match field.dtype {
                E57FieldType::Float   => {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&record[offset-bw..offset]);
                    f64::from_le_bytes(b)
                }
                E57FieldType::Float32 => {
                    let mut b = [0u8; 4];
                    b.copy_from_slice(&record[offset-bw..offset]);
                    f64::from(f32::from_le_bytes(b))
                }
                E57FieldType::ScaledInteger => {
                    raw_val as f64 * field.scale + field.offset
                }
                E57FieldType::Integer => raw_val as f64,
            };

            match field.name.as_str() {
                "cartesianX" => out.x = physical,
                "cartesianY" => out.y = physical,
                "cartesianZ" => out.z = physical,
                "intensity"  => out.intensity = (physical * 65535.0).clamp(0.0, 65535.0) as u16,
                "colorRed"   => {
                    let c = out.color.get_or_insert(Rgb16::default());
                    c.red = (physical as u8 as u16) << 8;
                }
                "colorGreen" => {
                    let c = out.color.get_or_insert(Rgb16::default());
                    c.green = (physical as u8 as u16) << 8;
                }
                "colorBlue"  => {
                    let c = out.color.get_or_insert(Rgb16::default());
                    c.blue = (physical as u8 as u16) << 8;
                }
                "nor:normalX" | "normalX" => out.normal_x = Some(physical as f32),
                "nor:normalY" | "normalY" => out.normal_y = Some(physical as f32),
                "nor:normalZ" | "normalZ" => out.normal_z = Some(physical as f32),
                _ => {}
            }
        }

        self.points_read += 1;
        Ok(true)
    }

    fn point_count(&self) -> Option<u64> { Some(self.meta.record_count) }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_paged_xml<R: Read>(r: &mut R, xml_len: usize) -> Result<String> {
    let pages = (xml_len + crate::e57::PAGE_PAYLOAD - 1) / crate::e57::PAGE_PAYLOAD;
    let mut bytes = Vec::with_capacity(xml_len);
    for _ in 0..pages {
        let page = crate::e57::page::read_page(r)?;
        bytes.extend_from_slice(&page);
    }
    bytes.truncate(xml_len);
    String::from_utf8(bytes).map_err(|e| Error::Utf8(e))
}

fn record_byte_size(meta: &PointCloudMeta) -> usize {
    meta.fields.iter()
        .map(|f| f.dtype.byte_width(f.minimum, f.maximum))
        .sum()
}

fn estimate_binary_size(meta: &PointCloudMeta) -> usize {
    record_byte_size(meta) * meta.record_count as usize
}

fn read_raw_int(bytes: &[u8], width: usize) -> i64 {
    match width {
        1 => bytes[0] as i64,
        2 => i16::from_le_bytes(bytes.try_into().unwrap_or([0; 2])) as i64,
        4 => i32::from_le_bytes(bytes.try_into().unwrap_or([0; 4])) as i64,
        8 => i64::from_le_bytes(bytes.try_into().unwrap_or([0; 8])),
        _ => 0,
    }
}
