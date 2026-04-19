//! GeoJP2 geolocation metadata: reading and writing the GeoJP2 UUID box.
//!
//! The GeoJP2 specification (OGC 05-047r2) stores CRS information inside a
//! `uuid` box whose 16-byte UUID is `B14BF8BD-083D-4B43-A5AE-8CD7D5A6CE03`.
//! The payload after the UUID is a minimal in-memory TIFF structure containing
//! only the GeoTIFF tag set (ModelPixelScaleTag, ModelTiepointTag, or
//! ModelTransformationTag, GeoKeyDirectoryTag, etc.).
//!
//! This module constructs and parses that embedded mini-TIFF independently of
//! any file I/O, so it can be embedded directly into a JP2 box.

use super::error::{Jp2Error, Result};
use super::types::{BoundingBox, GeoTransform};

// ── GeoTIFF tag codes (re-declared locally to keep module self-contained) ────

mod gtag {
    pub const MODEL_PIXEL_SCALE:    u16 = 33550;
    pub const MODEL_TIEPOINT:       u16 = 33922;
    pub const MODEL_TRANSFORMATION: u16 = 34264;
    pub const GEO_KEY_DIRECTORY:    u16 = 34735;
    pub const GEO_DOUBLE_PARAMS:    u16 = 34736;
    pub const GEO_ASCII_PARAMS:     u16 = 34737;
    pub const GDAL_NODATA:          u16 = 42113;
}

/// GeoKey IDs used in the GeoKeyDirectory.
mod geokey {
    pub const GT_MODEL_TYPE:         u16 = 1024;
    pub const GT_RASTER_TYPE:        u16 = 1025;
    pub const GEOGRAPHIC_TYPE:       u16 = 2048;
    pub const PROJECTED_CS_TYPE:     u16 = 3072;
    pub const PROJECTION:            u16 = 3074;
    pub const PROJ_LINEAR_UNITS:     u16 = 3076;
    pub const GEOG_ANGULAR_UNITS:    u16 = 2054;
    pub const VERTICAL_CS_TYPE:      u16 = 4096;
}

// ── CrsInfo ───────────────────────────────────────────────────────────────────

/// Coordinate reference system information parsed from a GeoJP2 UUID box.
#[derive(Debug, Clone, Default)]
pub struct CrsInfo {
    /// EPSG code (geographic or projected).
    pub epsg: Option<u16>,
    /// ModelType: 1=projected, 2=geographic, 3=geocentric.
    pub model_type: Option<u16>,
    /// RasterType: 1=PixelIsArea, 2=PixelIsPoint.
    pub raster_type: Option<u16>,
    /// Affine geo-transform (from ModelPixelScale + ModelTiepoint or ModelTransformation).
    pub geo_transform: Option<GeoTransform>,
    /// GDAL-style NODATA value.
    pub no_data: Option<f64>,
    /// Raw GeoKey directory entries (key_id, tiff_tag_location, count, value_or_index).
    pub raw_geokeys: Vec<[u16; 4]>,
}

impl CrsInfo {
    /// Bounding box derived from the geo-transform and image dimensions.
    pub fn bounding_box(&self, width: u32, height: u32) -> Option<BoundingBox> {
        let gt = self.geo_transform.as_ref()?;
        let (x0, y0) = gt.pixel_to_geo(0.0, 0.0);
        let (x1, y1) = gt.pixel_to_geo(width as f64, height as f64);
        Some(BoundingBox::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
    }
}

// ── Mini-TIFF writer (for the UUID box payload) ───────────────────────────────

/// Build the GeoJP2 UUID box payload: a minimal little-endian TIFF containing
/// only the GeoTIFF tag set, with no actual image data.
pub fn build_geojp2_payload(
    geo_transform: Option<&GeoTransform>,
    epsg: Option<u16>,
    model_type: u16,       // 1=projected, 2=geographic
    no_data: Option<f64>,
) -> Vec<u8> {
    // We write a tiny TIFF structure:
    //   8-byte TIFF header  (II, 42, ifd_offset=8)
    //   IFD with GeoTIFF tags
    //   Extra data (doubles, shorts for geokeys, ASCII)

    let mut tags: Vec<MiniTag> = Vec::new();

    // ModelPixelScale + ModelTiepoint (from geo_transform)
    if let Some(gt) = geo_transform {
        let scale: Vec<f64> = vec![gt.pixel_width, -gt.pixel_height, 0.0];
        let tiepoint: Vec<f64> = vec![0.0, 0.0, 0.0, gt.x_origin, gt.y_origin, 0.0];
        tags.push(MiniTag::doubles(gtag::MODEL_PIXEL_SCALE, scale));
        tags.push(MiniTag::doubles(gtag::MODEL_TIEPOINT, tiepoint));
    }

    // GeoKeyDirectory
    let mut geokeys: Vec<[u16; 4]> = Vec::new();
    // Header: version=1, revision=1, minor=0, num_keys=?
    geokeys.push([1, 1, 0, 0]); // placeholder count

    geokeys.push([geokey::GT_MODEL_TYPE, 0, 1, model_type]);
    geokeys.push([geokey::GT_RASTER_TYPE, 0, 1, 1]); // PixelIsArea

    if let Some(code) = epsg {
        if model_type == 2 {
            geokeys.push([geokey::GEOGRAPHIC_TYPE, 0, 1, code]);
        } else {
            geokeys.push([geokey::PROJECTED_CS_TYPE, 0, 1, code]);
        }
    }

    // Patch key count
    geokeys[0][3] = (geokeys.len() - 1) as u16;

    let dir_shorts: Vec<u16> = geokeys.iter().flat_map(|k| k.iter().copied()).collect();
    tags.push(MiniTag::shorts(gtag::GEO_KEY_DIRECTORY, dir_shorts));

    if let Some(nd) = no_data {
        let s = format!("{}", nd);
        tags.push(MiniTag::ascii(gtag::GDAL_NODATA, s));
    }

    tags.sort_by_key(|t| t.code);
    serialise_mini_tiff(&tags)
}

// ── Mini-TIFF parser (from UUID box payload) ─────────────────────────────────

/// Parse a GeoJP2 UUID box payload (mini-TIFF) into `CrsInfo`.
pub fn parse_geojp2_payload(data: &[u8]) -> Result<CrsInfo> {
    if data.len() < 8 {
        return Err(Jp2Error::InvalidGeoMetadata("payload too short".into()));
    }

    // Detect byte order
    let le = match &data[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(Jp2Error::InvalidGeoMetadata("unknown byte order in mini-TIFF".into())),
    };

    let read_u16 = |d: &[u8], off: usize| -> u16 {
        let b = [d[off], d[off+1]];
        if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) }
    };
    let read_u32 = |d: &[u8], off: usize| -> u32 {
        let b = [d[off], d[off+1], d[off+2], d[off+3]];
        if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
    };
    let read_f64 = |d: &[u8], off: usize| -> f64 {
        let b: [u8;8] = d[off..off+8].try_into().unwrap_or([0;8]);
        if le { f64::from_le_bytes(b) } else { f64::from_be_bytes(b) }
    };

    let ifd_off = read_u32(data, 4) as usize;
    if ifd_off + 2 > data.len() {
        return Err(Jp2Error::InvalidGeoMetadata("IFD offset out of range".into()));
    }

    let num_entries = read_u16(data, ifd_off) as usize;

    let mut pixel_scale: Option<Vec<f64>> = None;
    let mut tiepoint: Option<Vec<f64>> = None;
    let mut geo_key_dir: Option<Vec<u16>> = None;
    let mut no_data: Option<f64> = None;

    for i in 0..num_entries {
        let base = ifd_off + 2 + i * 12;
        if base + 12 > data.len() { break; }
        let tag   = read_u16(data, base);
        let dtype = read_u16(data, base + 2);
        let count = read_u32(data, base + 4) as usize;
        let voff  = base + 8;

        // Get data — inline (≤4 bytes) or at offset
        let get_bytes = |count: usize, bytes_each: usize| -> Vec<u8> {
            let total = count * bytes_each;
            if total <= 4 {
                data[voff..voff + total].to_vec()
            } else {
                let off = read_u32(data, voff) as usize;
                if off + total <= data.len() { data[off..off + total].to_vec() } else { Vec::new() }
            }
        };

        match (tag, dtype) {
            (t, 12) if t == gtag::MODEL_PIXEL_SCALE || t == gtag::MODEL_TIEPOINT || t == gtag::MODEL_TRANSFORMATION => {
                let raw = get_bytes(count, 8);
                let vals: Vec<f64> = raw.chunks_exact(8)
                    .map(|c| if le { f64::from_le_bytes(c.try_into().unwrap()) } else { f64::from_be_bytes(c.try_into().unwrap()) })
                    .collect();
                if t == gtag::MODEL_PIXEL_SCALE {
                    pixel_scale = Some(vals);
                } else if t == gtag::MODEL_TIEPOINT {
                    tiepoint = Some(vals);
                }
            }
            (t, 3) if t == gtag::GEO_KEY_DIRECTORY => {
                let raw = get_bytes(count, 2);
                let shorts: Vec<u16> = raw.chunks_exact(2)
                    .map(|c| if le { u16::from_le_bytes(c.try_into().unwrap()) } else { u16::from_be_bytes(c.try_into().unwrap()) })
                    .collect();
                geo_key_dir = Some(shorts);
            }
            (t, 2) if t == gtag::GDAL_NODATA => {
                let raw = get_bytes(count, 1);
                if let Ok(s) = std::str::from_utf8(&raw) {
                    no_data = s.trim_end_matches('\0').parse::<f64>().ok();
                }
            }
            _ => {}
        }
    }

    // Build GeoTransform from scale + tiepoint
    let geo_transform = if let (Some(sc), Some(tp)) = (&pixel_scale, &tiepoint) {
        if sc.len() >= 2 && tp.len() >= 6 {
            Some(GeoTransform::new(
                tp[3] - tp[0] * sc[0],
                sc[0], 0.0,
                tp[4] + tp[1] * sc[1],
                0.0,
                -sc[1],
            ))
        } else { None }
    } else { None };

    // Parse GeoKey directory for EPSG
    let mut epsg = None;
    let mut model_type = None;
    let mut raster_type = None;
    let mut raw_geokeys = Vec::new();

    if let Some(ref dir) = geo_key_dir {
        if dir.len() >= 4 {
            let num_keys = dir[3] as usize;
            for k in 0..num_keys {
                let base = 4 + k * 4;
                if base + 4 > dir.len() { break; }
                let key_id = dir[base];
                let loc    = dir[base + 1];
                let cnt    = dir[base + 2];
                let val    = dir[base + 3];
                raw_geokeys.push([key_id, loc, cnt, val]);
                match key_id {
                    k if k == geokey::GEOGRAPHIC_TYPE    => epsg = Some(val),
                    k if k == geokey::PROJECTED_CS_TYPE  => epsg = Some(val),
                    k if k == geokey::GT_MODEL_TYPE      => model_type = Some(val),
                    k if k == geokey::GT_RASTER_TYPE     => raster_type = Some(val),
                    _ => {}
                }
            }
        }
    }

    Ok(CrsInfo { epsg, model_type, raster_type, geo_transform, no_data, raw_geokeys })
}

/// Parse a GMLJP2 XML payload into `CrsInfo` when a GeoJP2 UUID box is absent.
///
/// This is a best-effort fallback for products (e.g. Sentinel-2 JP2 assets)
/// that encode georeferencing in `xml ` boxes instead of GeoJP2 UUID metadata.
pub fn parse_gmljp2_xml_payload(xml: &str) -> Option<CrsInfo> {
    fn extract_first(text: &str, start: &str, end: &str) -> Option<String> {
        let s = text.find(start)? + start.len();
        let tail = &text[s..];
        let e = tail.find(end)?;
        Some(tail[..e].trim().to_string())
    }

    fn extract_attr_value(tag_fragment: &str, attr: &str) -> Option<String> {
        let token = format!("{attr}=\"");
        let s = tag_fragment.find(&token)? + token.len();
        let tail = &tag_fragment[s..];
        let e = tail.find('"')?;
        Some(tail[..e].trim().to_string())
    }

    fn parse_two_f64(text: &str) -> Option<(f64, f64)> {
        let mut it = text.split_whitespace();
        let a = it.next()?.parse::<f64>().ok()?;
        let b = it.next()?.parse::<f64>().ok()?;
        Some((a, b))
    }

    fn parse_two_i64(text: &str) -> Option<(i64, i64)> {
        let mut it = text.split_whitespace();
        let a = it.next()?.parse::<i64>().ok()?;
        let b = it.next()?.parse::<i64>().ok()?;
        Some((a, b))
    }

    fn parse_epsg(s: &str) -> Option<u16> {
        let digits_rev: String = s
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits_rev.is_empty() {
            return None;
        }
        let digits: String = digits_rev.chars().rev().collect();
        digits.parse::<u16>().ok()
    }

    // Quick reject for non-GMLJP2 XML boxes.
    if !xml.contains("RectifiedGrid") {
        return None;
    }

    // Grid low indices (if present) are commonly 1-based in GMLJP2.
    let (low_x, low_y) = extract_first(xml, "<gml:low>", "</gml:low>")
        .and_then(|s| parse_two_i64(&s))
        .unwrap_or((1, 1));

    let origin_pos = extract_first(xml, "<gml:pos>", "</gml:pos>")
        .and_then(|s| parse_two_f64(&s))?;

    // Parse the first two offset vectors.
    let mut offsets: Vec<(f64, f64)> = Vec::new();
    let mut search_from = 0usize;
    while offsets.len() < 2 {
        let rel = xml[search_from..].find("<gml:offsetVector")?;
        let abs = search_from + rel;
        let after_tag = xml[abs..].find('>')? + abs + 1;
        let end = xml[after_tag..].find("</gml:offsetVector>")? + after_tag;
        let text = xml[after_tag..end].trim();
        offsets.push(parse_two_f64(text)?);
        search_from = end + "</gml:offsetVector>".len();
    }

    let (off1x, off1y) = offsets[0];
    let (off2x, off2y) = offsets[1];

    // Treat origin as the centre of the (low_x, low_y) grid point and convert
    // to GDAL-style upper-left corner of pixel (0,0).
    let low_dx = (low_x - 1) as f64;
    let low_dy = (low_y - 1) as f64;
    let x_origin = origin_pos.0 + low_dx * off1x + low_dy * off2x - 0.5 * (off1x + off2x);
    let y_origin = origin_pos.1 + low_dx * off1y + low_dy * off2y - 0.5 * (off1y + off2y);

    let gt = GeoTransform::new(x_origin, off1x, off2x, y_origin, off1y, off2y);

    let srs_name = extract_first(xml, "srsName=\"", "\"")
        .or_else(|| {
            let p0 = xml.find("<gml:Point")?;
            let p1 = xml[p0..].find('>')? + p0;
            extract_attr_value(&xml[p0..p1], "srsName")
        });

    let epsg = srs_name.as_deref().and_then(parse_epsg);

    Some(CrsInfo {
        epsg,
        model_type: epsg.map(|e| if (4000..5000).contains(&e) { 2 } else { 1 }),
        raster_type: Some(1),
        geo_transform: Some(gt),
        no_data: None,
        raw_geokeys: Vec::new(),
    })
}

// ── Mini-TIFF serialiser ──────────────────────────────────────────────────────

struct MiniTag {
    code:      u16,
    dtype:     u16,   // 3=short, 12=double, 2=ascii
    count:     u32,
    payload:   Vec<u8>,
}

impl MiniTag {
    fn doubles(code: u16, vals: Vec<f64>) -> Self {
        let payload: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
        Self { code, dtype: 12, count: vals.len() as u32, payload }
    }
    fn shorts(code: u16, vals: Vec<u16>) -> Self {
        let payload: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
        Self { code, dtype: 3, count: vals.len() as u32, payload }
    }
    fn ascii(code: u16, mut s: String) -> Self {
        s.push('\0');
        let count = s.len() as u32;
        Self { code, dtype: 2, count, payload: s.into_bytes() }
    }
}

fn serialise_mini_tiff(tags: &[MiniTag]) -> Vec<u8> {
    let ifd_offset: u32 = 8;
    let ifd_bytes: u32 = 2 + tags.len() as u32 * 12 + 4;

    // Layout: assign extra-data offsets
    let mut extra_offsets: Vec<u32> = Vec::with_capacity(tags.len());
    let mut cur = ifd_offset + ifd_bytes;
    for t in tags {
        if t.payload.len() > 4 {
            extra_offsets.push(cur);
            cur += t.payload.len() as u32;
            if cur % 2 != 0 { cur += 1; }
        } else {
            extra_offsets.push(0);
        }
    }

    let mut out: Vec<u8> = Vec::new();

    // TIFF header
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&ifd_offset.to_le_bytes());

    // IFD
    out.extend_from_slice(&(tags.len() as u16).to_le_bytes());
    for (t, &ex) in tags.iter().zip(extra_offsets.iter()) {
        out.extend_from_slice(&t.code.to_le_bytes());
        out.extend_from_slice(&t.dtype.to_le_bytes());
        out.extend_from_slice(&t.count.to_le_bytes());
        if t.payload.len() <= 4 {
            let mut buf = [0u8; 4];
            buf[..t.payload.len()].copy_from_slice(&t.payload);
            out.extend_from_slice(&buf);
        } else {
            out.extend_from_slice(&ex.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes()); // next IFD

    // Extra data
    for t in tags {
        if t.payload.len() > 4 {
            out.extend_from_slice(&t.payload);
            if t.payload.len() % 2 != 0 { out.push(0); }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_geo_meta() {
        let gt = GeoTransform::north_up(10.0, 0.001, 49.0, -0.001);
        let payload = build_geojp2_payload(Some(&gt), Some(4326), 2, Some(-9999.0));
        let crs = parse_geojp2_payload(&payload).unwrap();

        let parsed_gt = crs.geo_transform.unwrap();
        assert!((parsed_gt.x_origin - 10.0).abs() < 1e-9, "x_origin mismatch");
        assert!((parsed_gt.y_origin - 49.0).abs() < 1e-9, "y_origin mismatch");
        assert!((parsed_gt.pixel_width - 0.001).abs() < 1e-9, "pixel_width mismatch");
        assert!((parsed_gt.pixel_height - (-0.001)).abs() < 1e-9, "pixel_height mismatch");

        assert_eq!(crs.epsg, Some(4326));
        assert_eq!(crs.no_data, Some(-9999.0));
    }

    #[test]
    fn roundtrip_no_transform() {
        let payload = build_geojp2_payload(None, Some(32632), 1, None);
        let crs = parse_geojp2_payload(&payload).unwrap();
        assert_eq!(crs.epsg, Some(32632));
        assert!(crs.geo_transform.is_none());
    }

        #[test]
        fn parse_gmljp2_xml_fallback_rectified_grid() {
                let xml = r#"<?xml version='1.0'?>
<gml:FeatureCollection xmlns:gml="http://www.opengis.net/gml">
    <gml:featureMember>
        <gml:RectifiedGridCoverage>
            <gml:rectifiedGridDomain>
                <gml:RectifiedGrid dimension="2">
                    <gml:limits><gml:GridEnvelope><gml:low>1 1</gml:low><gml:high>1830 1830</gml:high></gml:GridEnvelope></gml:limits>
                    <gml:origin>
                        <gml:Point srsName="urn:ogc:def:crs:EPSG::32617"><gml:pos>500010 4899990</gml:pos></gml:Point>
                    </gml:origin>
                    <gml:offsetVector srsName="urn:ogc:def:crs:EPSG::32617">60 0</gml:offsetVector>
                    <gml:offsetVector srsName="urn:ogc:def:crs:EPSG::32617">0 -60</gml:offsetVector>
                </gml:RectifiedGrid>
            </gml:rectifiedGridDomain>
        </gml:RectifiedGridCoverage>
    </gml:featureMember>
</gml:FeatureCollection>
"#;

                let crs = parse_gmljp2_xml_payload(xml).expect("expected GMLJP2 fallback parse");
                assert_eq!(crs.epsg, Some(32617));
                let gt = crs.geo_transform.expect("expected geotransform");
                assert!((gt.x_origin - 499980.0).abs() < 1e-6);
                assert!((gt.y_origin - 4900020.0).abs() < 1e-6);
                assert!((gt.pixel_width - 60.0).abs() < 1e-9);
                assert!((gt.pixel_height + 60.0).abs() < 1e-9);
        }
}
