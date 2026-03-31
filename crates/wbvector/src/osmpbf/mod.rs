//! OpenStreetMap PBF reader (read-only).
//!
//! This module is available when built with the `osmpbf` feature.
//! It reads OSM ways (and their dependent nodes) from `.osm.pbf` and returns a
//! single `Layer` containing `LineString` / `Polygon` geometries.
//!
//! Notes:
//! - Writer support is intentionally not provided.
//! - Layer CRS is set to EPSG:4326.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, Tags};

use crate::error::{GeoError, Result};
use crate::feature::{FieldDef, FieldType, FieldValue, Feature, Layer};
use crate::geometry::{Coord, Geometry, GeometryType, Ring};

/// Filtering options for OSM PBF reads.
#[derive(Debug, Clone, Default)]
pub struct OsmPbfReadOptions {
    /// Keep only ways with a `highway=*` tag.
    pub highways_only: bool,
    /// Keep only ways with a non-empty `name=*` tag.
    pub named_ways_only: bool,
    /// Keep only polygonized area ways.
    pub polygons_only: bool,
    /// Optional whitelist of tag keys to keep in the `osm_tags` JSON field.
    ///
    /// When `None`, all tags are included.
    pub include_tag_keys: Option<Vec<String>>,
}

impl OsmPbfReadOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_highways_only(mut self, yes: bool) -> Self {
        self.highways_only = yes;
        self
    }

    pub fn with_named_ways_only(mut self, yes: bool) -> Self {
        self.named_ways_only = yes;
        self
    }

    pub fn with_polygons_only(mut self, yes: bool) -> Self {
        self.polygons_only = yes;
        self
    }

    pub fn with_include_tag_keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let keys_vec: Vec<String> = keys.into_iter().map(Into::into).filter(|k| !k.is_empty()).collect();
        self.include_tag_keys = if keys_vec.is_empty() { None } else { Some(keys_vec) };
        self
    }
}

/// Read OSM PBF from a file path.
pub fn read<P: AsRef<Path>>(path: P) -> Result<Layer> {
    read_with_options(path, &OsmPbfReadOptions::default())
}

/// Read OSM PBF from a file path with filtering options.
pub fn read_with_options<P: AsRef<Path>>(path: P, options: &OsmPbfReadOptions) -> Result<Layer> {
    let file = File::open(path).map_err(GeoError::Io)?;
    read_from_reader_with_options(file, options)
}

/// Read OSM PBF from any reader.
pub fn read_from_reader<R: Read + Seek>(reader: R) -> Result<Layer> {
    read_from_reader_with_options(reader, &OsmPbfReadOptions::default())
}

/// Read OSM PBF from any reader with filtering options.
pub fn read_from_reader_with_options<R: Read + Seek>(
    reader: R,
    options: &OsmPbfReadOptions,
) -> Result<Layer> {
    let mut pbf = OsmPbfReader::new(reader);
    let objs = pbf
        .get_objs_and_deps(|obj| matches!(obj, OsmObj::Way(_)))
        .map_err(|e| GeoError::OsmPbf(e.to_string()))?;
    layer_from_objs(&objs, options)
}

fn layer_from_objs(objs: &BTreeMap<OsmId, OsmObj>, options: &OsmPbfReadOptions) -> Result<Layer> {
    let mut layer = Layer::new("osm_ways").with_epsg(4326);
    layer.add_field(FieldDef::new("osm_id", FieldType::Integer));
    layer.add_field(FieldDef::new("name", FieldType::Text));
    layer.add_field(FieldDef::new("highway", FieldType::Text));
    layer.add_field(FieldDef::new("building", FieldType::Text));
    layer.add_field(FieldDef::new("osm_tags", FieldType::Text));

    let mut declared_geom: Option<GeometryType> = None;
    let mut mixed_geom = false;

    for obj in objs.values() {
        let OsmObj::Way(way) = obj else { continue };

        let mut coords = Vec::<Coord>::new();
        for node_id in &way.nodes {
            if let Some(OsmObj::Node(node)) = objs.get(&OsmId::Node(*node_id)) {
                coords.push(Coord::xy(node.lon(), node.lat()));
            }
        }

        if coords.len() < 2 {
            continue;
        }

        let is_closed = coords.first() == coords.last();
        let is_polygon = is_closed && way_is_area(&way.tags);

        if !way_matches_options(&way.tags, is_polygon, options) {
            continue;
        }

        let geom = if is_polygon {
            let mut ring = coords;
            ensure_ring_closed(&mut ring);
            Geometry::Polygon {
                exterior: Ring::new(ring),
                interiors: vec![],
            }
        } else {
            Geometry::LineString(coords)
        };

        let gt = geom.geom_type();
        if let Some(prev) = declared_geom {
            if prev != gt {
                mixed_geom = true;
            }
        } else {
            declared_geom = Some(gt);
        }

        let name = way.tags.get("name").map(|s| s.to_string()).unwrap_or_default();
        let highway = way.tags.get("highway").map(|s| s.to_string()).unwrap_or_default();
        let building = way.tags.get("building").map(|s| s.to_string()).unwrap_or_default();

        layer.push(Feature {
            fid: way.id.0 as u64,
            geometry: Some(geom),
            attributes: vec![
                FieldValue::Integer(way.id.0),
                if name.is_empty() { FieldValue::Null } else { FieldValue::Text(name) },
                if highway.is_empty() { FieldValue::Null } else { FieldValue::Text(highway) },
                if building.is_empty() { FieldValue::Null } else { FieldValue::Text(building) },
                FieldValue::Text(tags_to_json_filtered(&way.tags, options.include_tag_keys.as_deref())),
            ],
        });
    }

    layer.geom_type = if mixed_geom {
        Some(GeometryType::GeometryCollection)
    } else {
        declared_geom
    };

    Ok(layer)
}

fn ensure_ring_closed(ring: &mut Vec<Coord>) {
    if ring.len() < 3 {
        return;
    }
    if let (Some(first), Some(last)) = (ring.first().cloned(), ring.last()) {
        if &first != last {
            ring.push(first);
        }
    }
}

fn way_is_area(tags: &Tags) -> bool {
    if let Some(v) = tags.get("area") {
        let lower = v.to_ascii_lowercase();
        if lower == "yes" || lower == "1" || lower == "true" {
            return true;
        }
    }

    [
        "building",
        "landuse",
        "natural",
        "amenity",
        "leisure",
        "water",
        "waterway",
    ]
    .iter()
    .any(|k| tags.contains_key(*k))
}

fn way_matches_options(tags: &Tags, is_polygon: bool, options: &OsmPbfReadOptions) -> bool {
    if options.highways_only && !tags.contains_key("highway") {
        return false;
    }

    if options.named_ways_only {
        let has_name = tags
            .get("name")
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_name {
            return false;
        }
    }

    if options.polygons_only && !is_polygon {
        return false;
    }

    true
}

fn tags_to_json_filtered(tags: &Tags, include_keys: Option<&[String]>) -> String {
    let mut out = String::from("{");
    let mut first = true;
    for (k, v) in tags.iter() {
        if let Some(keys) = include_keys {
            if !keys.iter().any(|kk| kk == k) {
                continue;
            }
        }

        if !first {
            out.push(',');
        }
        first = false;

        out.push('"');
        out.push_str(&json_escape(k));
        out.push_str("\":\"");
        out.push_str(&json_escape(v));
        out.push('"');
    }
    out.push('}');
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_json_escapes_quotes() {
        let mut tags = Tags::new();
        tags.insert("name".into(), "A \"quoted\" value".into());
        let s = tags_to_json_filtered(&tags, None);
        assert!(s.contains("\\\"quoted\\\""));
    }

    #[test]
    fn tags_json_filter_keeps_only_whitelist() {
        let mut tags = Tags::new();
        tags.insert("name".into(), "Main".into());
        tags.insert("highway".into(), "residential".into());
        tags.insert("surface".into(), "gravel".into());

        let keep = vec!["name".to_string(), "highway".to_string()];
        let s = tags_to_json_filtered(&tags, Some(&keep));

        assert!(s.contains("\"name\""));
        assert!(s.contains("\"highway\""));
        assert!(!s.contains("\"surface\""));
    }

    #[test]
    fn area_tag_detection() {
        let mut tags = Tags::new();
        tags.insert("building".into(), "yes".into());
        assert!(way_is_area(&tags));
    }

    #[test]
    fn options_filter_logic() {
        let mut tags = Tags::new();
        tags.insert("name".into(), "Main Street".into());
        tags.insert("highway".into(), "residential".into());

        let opts = OsmPbfReadOptions::new()
            .with_highways_only(true)
            .with_named_ways_only(true)
            .with_polygons_only(false);
        assert!(way_matches_options(&tags, false, &opts));

        let opts_poly = OsmPbfReadOptions::new().with_polygons_only(true);
        assert!(!way_matches_options(&tags, false, &opts_poly));
        assert!(way_matches_options(&tags, true, &opts_poly));
    }
}
