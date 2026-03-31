//! Minimal E57 XML parser / builder.
//!
//! E57 embeds a UTF-8 XML document in the file to describe the structure.
//! This module implements a lightweight pull parser just sufficient to:
//! * Locate `<vectorChild>` elements for each point cloud.
//! * Extract `<points>` binary section metadata (fileOffset, recordCount).
//! * Extract prototype field names and types.
//!
//! For writing, it builds the minimal XML document describing a Cartesian
//! point cloud with optional colour and intensity.

/// Describes one scalar field in an E57 point record.
#[derive(Debug, Clone)]
pub struct E57Field {
    /// Field name (e.g. `"cartesianX"`, `"colorRed"`).
    pub name: String,
    /// E57 data type tag.
    pub dtype: E57FieldType,
    /// For scaled integer fields: scale factor.
    pub scale: f64,
    /// For scaled integer fields: offset.
    pub offset: f64,
    /// Minimum integer value (for `ScaledInteger` and `Integer`).
    pub minimum: i64,
    /// Maximum integer value.
    pub maximum: i64,
}

/// E57 scalar field types relevant to point clouds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E57FieldType {
    /// 64-bit IEEE 754 floating point.
    Float,
    /// 32-bit IEEE 754 floating point.
    Float32,
    /// Scaled integer (integer bits × scale + offset = physical value).
    ScaledInteger,
    /// Raw integer.
    Integer,
}

impl E57FieldType {
    /// Byte width of the raw storage representation.
    pub fn byte_width(self, minimum: i64, maximum: i64) -> usize {
        let range = (maximum - minimum) as u64;
        match self {
            E57FieldType::Float    => 8,
            E57FieldType::Float32  => 4,
            E57FieldType::ScaledInteger | E57FieldType::Integer => {
                if range <= 0xFF       { 1 }
                else if range <= 0xFFFF     { 2 }
                else if range <= 0xFFFF_FFFF { 4 }
                else { 8 }
            }
        }
    }
}

/// Metadata for one point cloud section extracted from the E57 XML.
#[derive(Debug, Clone, Default)]
pub struct PointCloudMeta {
    /// GUID of this data3D entry.
    pub guid: String,
    /// Human-readable name.
    pub name: String,
    /// CRS description string per ASTM E2807 §8.4.6 `coordinateMetadata`.
    /// In practice this is OGC WKT2 or an EPSG authority string.
    pub coordinate_metadata: Option<String>,
    /// Byte offset of the binary section (from start of file).
    pub file_offset: u64,
    /// Number of point records.
    pub record_count: u64,
    /// Prototype field descriptors.
    pub fields: Vec<E57Field>,
}

// ── Minimal XML pull parser ───────────────────────────────────────────────────

/// Parse one or more `<data3D>` entries from the E57 XML string.
pub fn parse_point_clouds(xml: &str) -> Vec<PointCloudMeta> {
    // Very lightweight: we scan for known tags rather than building a DOM.
    let mut results = Vec::new();
    let mut pos = 0;

    while let Some(start) = find_tag(xml, "<data3D", pos) {
        let end = find_closing(xml, "data3D", start).unwrap_or(xml.len());
        let section = &xml[start..end];
        results.push(parse_data3d(section));
        pos = end;
    }
    results
}

fn parse_data3d(s: &str) -> PointCloudMeta {
    let mut meta = PointCloudMeta::default();
    meta.guid = extract_text(s, "guid").unwrap_or_default();
    meta.name = extract_text(s, "name").unwrap_or_default();
    meta.coordinate_metadata = extract_text(s, "coordinateMetadata");

    // Find <points> element
    if let Some(pts_start) = find_tag(s, "<points", 0) {
        if let Some(pts_end) = find_closing(s, "points", pts_start) {
            let pts_section = &s[pts_start..pts_end];
            meta.file_offset  = attr_u64(pts_section, "fileOffset").unwrap_or(0);
            meta.record_count = attr_u64(pts_section, "recordCount").unwrap_or(0);

            // Parse prototype fields
            for tag in &["Float", "ScaledInteger", "Integer"] {
                let open = format!("<{tag}");
                let mut fp = 0;
                while let Some(fstart) = find_tag(pts_section, &open, fp) {
                    let field = parse_field_element(pts_section, fstart, tag);
                    if let Some(f) = field { meta.fields.push(f); }
                    fp = fstart + 1;
                }
            }
        }
    }
    meta
}

fn parse_field_element(s: &str, start: usize, tag: &str) -> Option<E57Field> {
    let end = s[start..].find('>').map(|i| start + i + 1)?;
    let elem = &s[start..end];
    let name = attr_str(elem, "name")?;
    let dtype = match tag {
        "Float"          => E57FieldType::Float,
        "ScaledInteger"  => E57FieldType::ScaledInteger,
        "Integer"        => E57FieldType::Integer,
        _                => return None,
    };
    Some(E57Field {
        name,
        dtype,
        scale:   attr_f64(elem, "scale").unwrap_or(1.0),
        offset:  attr_f64(elem, "offset").unwrap_or(0.0),
        minimum: attr_i64(elem, "minimum").unwrap_or(i64::MIN),
        maximum: attr_i64(elem, "maximum").unwrap_or(i64::MAX),
    })
}

// ── XML builder ───────────────────────────────────────────────────────────────

/// Build the minimal E57 XML document for a Cartesian point cloud.
///
/// `coordinate_metadata` is an optional OGC WKT2 string (or other EPSG
/// authority string) stored in the `<coordinateMetadata>` element of the
/// `Data3D` section per ASTM E2807 §8.4.6.
pub fn build_xml(
    point_count: u64,
    file_offset: u64,
    has_intensity: bool,
    has_color: bool,
    guid: &str,
    name: &str,
    coordinate_metadata: Option<&str>,
) -> String {
    let mut xml = String::with_capacity(2048);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<e57Root type="Structure" xmlns="http://www.astm.org/COMMIT/E57/2010-e57-v1.0">"#);
    xml.push('\n');
    xml.push_str(r#"  <formatName type="String"><![CDATA[ASTM E57 3D Imaging Data File]]></formatName>"#);
    xml.push('\n');
    xml.push_str(&format!(r#"  <guid type="String"><![CDATA[{guid}]]></guid>"#));
    xml.push('\n');
    xml.push_str(r#"  <versionMajor type="Integer">1</versionMajor>"#);
    xml.push('\n');
    xml.push_str(r#"  <versionMinor type="Integer">0</versionMinor>"#);
    xml.push('\n');
    xml.push_str(r#"  <data3D type="Vector" allowHeterogeneousChildren="1">"#);
    xml.push('\n');
    xml.push_str(r#"    <vectorChild type="Structure">"#);
    xml.push('\n');
    xml.push_str(&format!(r#"      <name type="String"><![CDATA[{name}]]></name>"#));
    xml.push('\n');
    if let Some(crs) = coordinate_metadata {
        // Escape the CRS string inside a CDATA section so arbitrary WKT2
        // content (brackets, quotes, backslashes) is preserved verbatim.
        xml.push_str(&format!(
            r#"      <coordinateMetadata type="String"><![CDATA[{crs}]]></coordinateMetadata>"#
        ));
        xml.push('\n');
    }
    xml.push_str(&format!(
        r#"      <points type="CompressedVector" fileOffset="{file_offset}" recordCount="{point_count}">"#
    ));
    xml.push('\n');
    xml.push_str(r#"        <prototype type="Structure">"#);
    xml.push('\n');
    for axis in &["X", "Y", "Z"] {
        xml.push_str(&format!(
            r#"          <cartesian{axis} type="Float" precision="double"/>"#
        ));
        xml.push('\n');
    }
    if has_intensity {
        xml.push_str(r#"          <intensity type="Float" precision="single"/>"#);
        xml.push('\n');
    }
    if has_color {
        for ch in &["Red", "Green", "Blue"] {
            xml.push_str(&format!(
                r#"          <color{ch} type="Integer" minimum="0" maximum="255"/>"#
            ));
            xml.push('\n');
        }
    }
    xml.push_str(r#"        </prototype>"#); xml.push('\n');
    xml.push_str(r#"        <codecs type="Vector"/>"#); xml.push('\n');
    xml.push_str(r#"      </points>"#); xml.push('\n');
    xml.push_str(r#"    </vectorChild>"#); xml.push('\n');
    xml.push_str(r#"  </data3D>"#); xml.push('\n');
    xml.push_str(r#"</e57Root>"#); xml.push('\n');
    xml
}

// ── Helper parsers ────────────────────────────────────────────────────────────

fn find_tag(s: &str, tag: &str, from: usize) -> Option<usize> {
    s[from..].find(tag).map(|i| from + i)
}

fn find_closing(s: &str, tag: &str, from: usize) -> Option<usize> {
    let close = format!("</{tag}>");
    s[from..].find(&close).map(|i| from + i + close.len())
}

fn extract_text(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = s.find(&open)?;
    let gt = s[start..].find('>')? + start + 1;
    let end = s[gt..].find(&close)? + gt;
    // Strip CDATA wrapper if present
    let inner = s[gt..end].trim();
    if inner.starts_with("<![CDATA[") && inner.ends_with("]]>") {
        Some(inner[9..inner.len()-3].to_owned())
    } else {
        Some(inner.to_owned())
    }
}

fn attr_str(elem: &str, attr: &str) -> Option<String> {
    let key = format!("{attr}=\"");
    let start = elem.find(&key)? + key.len();
    let end = elem[start..].find('"')? + start;
    Some(elem[start..end].to_owned())
}

fn attr_u64(elem: &str, attr: &str) -> Option<u64> {
    attr_str(elem, attr)?.parse().ok()
}
fn attr_i64(elem: &str, attr: &str) -> Option<i64> {
    attr_str(elem, attr)?.parse().ok()
}
fn attr_f64(elem: &str, attr: &str) -> Option<f64> {
    attr_str(elem, attr)?.parse().ok()
}
