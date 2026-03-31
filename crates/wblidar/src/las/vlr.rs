//! Variable-Length Records (VLRs) and Extended VLRs (EVLRs).

use std::io::{Read, Write};
use crate::io::le;
use crate::Result;

/// Projection VLR user ID used by LAS files.
pub const LASF_PROJECTION_USER_ID: &str = "LASF_Projection";
/// OGC WKT CRS record ID.
pub const OGC_WKT_RECORD_ID: u16 = 2112;
/// GeoKeyDirectoryTag record ID.
pub const GEOKEY_DIRECTORY_RECORD_ID: u16 = 34735;

/// Identifies a VLR by (user_id, record_id) pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VlrKey {
    /// User identifier (16 ASCII bytes, null-padded).
    pub user_id: String,
    /// Record identifier.
    pub record_id: u16,
}

/// A parsed Variable-Length Record.
#[derive(Debug, Clone)]
pub struct Vlr {
    /// Identifying key.
    pub key: VlrKey,
    /// Human-readable description (32 ASCII bytes).
    pub description: String,
    /// Raw payload.
    pub data: Vec<u8>,
    /// True if this is an Extended VLR (64-bit length).
    pub extended: bool,
}

impl Vlr {
    /// Read a standard VLR (18-byte header + data).
    pub fn read_vlr<R: Read>(r: &mut R) -> Result<Self> {
        let _reserved = le::read_u16(r)?;
        let mut uid = [0u8; 16];
        r.read_exact(&mut uid)?;
        let record_id = le::read_u16(r)?;
        let record_length = le::read_u16(r)? as usize;
        let mut desc = [0u8; 32];
        r.read_exact(&mut desc)?;
        let mut data = vec![0u8; record_length];
        r.read_exact(&mut data)?;

        Ok(Vlr {
            key: VlrKey {
                user_id: null_terminated_string(&uid),
                record_id,
            },
            description: null_terminated_string(&desc),
            data,
            extended: false,
        })
    }

    /// Read an Extended VLR (60-byte header + 64-bit length + data).
    pub fn read_evlr<R: Read>(r: &mut R) -> Result<Self> {
        let _reserved = le::read_u16(r)?;
        let mut uid = [0u8; 16];
        r.read_exact(&mut uid)?;
        let record_id = le::read_u16(r)?;
        let record_length = le::read_u64(r)? as usize;
        let mut desc = [0u8; 32];
        r.read_exact(&mut desc)?;
        let mut data = vec![0u8; record_length];
        r.read_exact(&mut data)?;

        Ok(Vlr {
            key: VlrKey {
                user_id: null_terminated_string(&uid),
                record_id,
            },
            description: null_terminated_string(&desc),
            data,
            extended: true,
        })
    }

    /// Byte size of this VLR when serialised (header + data).
    pub fn serialised_size(&self) -> usize {
        if self.extended { 60 + self.data.len() } else { 54 + self.data.len() }
    }

    /// Write as a standard VLR.
    pub fn write<W: Write>(&self, w: &mut W) -> Result<()> {
        le::write_u16(w, 0)?; // reserved
        let mut uid = [0u8; 16];
        let bytes = self.key.user_id.as_bytes();
        let len = bytes.len().min(16);
        uid[..len].copy_from_slice(&bytes[..len]);
        w.write_all(&uid)?;
        le::write_u16(w, self.key.record_id)?;
        le::write_u16(w, self.data.len() as u16)?;
        let mut desc = [0u8; 32];
        let db = self.description.as_bytes();
        let dl = db.len().min(32);
        desc[..dl].copy_from_slice(&db[..dl]);
        w.write_all(&desc)?;
        w.write_all(&self.data)?;
        Ok(())
    }

    /// Write as an Extended VLR.
    pub fn write_extended<W: Write>(&self, w: &mut W) -> Result<()> {
        le::write_u16(w, 0)?;
        let mut uid = [0u8; 16];
        let bytes = self.key.user_id.as_bytes();
        uid[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
        w.write_all(&uid)?;
        le::write_u16(w, self.key.record_id)?;
        le::write_u64(w, self.data.len() as u64)?;
        let mut desc = [0u8; 32];
        let db = self.description.as_bytes();
        desc[..db.len().min(32)].copy_from_slice(&db[..db.len().min(32)]);
        w.write_all(&desc)?;
        w.write_all(&self.data)?;
        Ok(())
    }

    /// Build an OGC WKT projection VLR.
    pub fn ogc_wkt(wkt: &str) -> Self {
        let mut data = wkt.as_bytes().to_vec();
        if !data.ends_with(&[0]) {
            data.push(0);
        }
        Self {
            key: VlrKey {
                user_id: LASF_PROJECTION_USER_ID.to_owned(),
                record_id: OGC_WKT_RECORD_ID,
            },
            description: "OGC WKT".to_owned(),
            data,
            extended: false,
        }
    }

    /// Build a minimal GeoKeyDirectory projection VLR for an EPSG code.
    pub fn geokey_directory_for_epsg(epsg: u32) -> Option<Self> {
        let epsg_u16 = u16::try_from(epsg).ok()?;
        let key_id = if (4000..5000).contains(&epsg) { 2048u16 } else { 3072u16 };

        let values: [u16; 8] = [
            1, 1, 0, 1, // header: key-directory-version, key-revision, minor-revision, key-count
            key_id, 0, 1, epsg_u16, // key entry: key-id, tiff-tag-location, count, value-offset
        ];

        let mut data = Vec::with_capacity(values.len() * 2);
        for v in values {
            data.extend_from_slice(&v.to_le_bytes());
        }

        Some(Self {
            key: VlrKey {
                user_id: LASF_PROJECTION_USER_ID.to_owned(),
                record_id: GEOKEY_DIRECTORY_RECORD_ID,
            },
            description: "GeoKeyDirectoryTag".to_owned(),
            data,
            extended: false,
        })
    }
}

/// Find OGC WKT text in a LAS VLR list.
pub fn find_ogc_wkt(vlrs: &[Vlr]) -> Option<String> {
    let vlr = vlrs.iter().find(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == OGC_WKT_RECORD_ID
    })?;

    let end = vlr.data.iter().position(|&b| b == 0).unwrap_or(vlr.data.len());
    String::from_utf8(vlr.data[..end].to_vec()).ok().map(|s| s.trim().to_owned())
}

/// Find EPSG code in GeoKeyDirectoryTag VLR.
pub fn find_epsg(vlrs: &[Vlr]) -> Option<u32> {
    let vlr = vlrs.iter().find(|v| {
        v.key.user_id == LASF_PROJECTION_USER_ID && v.key.record_id == GEOKEY_DIRECTORY_RECORD_ID
    })?;

    if vlr.data.len() < 8 {
        return None;
    }

    let mut vals = Vec::<u16>::with_capacity(vlr.data.len() / 2);
    let mut i = 0usize;
    while i + 1 < vlr.data.len() {
        vals.push(u16::from_le_bytes([vlr.data[i], vlr.data[i + 1]]));
        i += 2;
    }
    if vals.len() < 4 {
        return None;
    }

    let key_count = vals[3] as usize;
    let mut pos = 4usize;
    for _ in 0..key_count {
        if pos + 3 >= vals.len() {
            break;
        }
        let key_id = vals[pos];
        let tiff_tag_location = vals[pos + 1];
        let value_offset = vals[pos + 3];

        if (key_id == 2048 || key_id == 3072) && tiff_tag_location == 0 {
            return Some(u32::from(value_offset));
        }

        pos += 4;
    }

    None
}

fn null_terminated_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wkt_roundtrip() {
        let v = Vlr::ogc_wkt("GEOGCS[\"WGS 84\"]");
        let parsed = find_ogc_wkt(&[v]).unwrap();
        assert!(parsed.contains("WGS 84"));
    }

    #[test]
    fn epsg_roundtrip() {
        let v = Vlr::geokey_directory_for_epsg(3857).unwrap();
        assert_eq!(find_epsg(&[v]), Some(3857));
    }
}
