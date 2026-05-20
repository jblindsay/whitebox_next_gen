use serde_json::json;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use wbprojection::Crs;
use wbcore::{
    LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest,
    ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};

pub struct DownloadOsmVectorTool;

// ── Filter presets ─────────────────────────────────────────────────────────────

const OSM_FILTER_PRESETS: &[&str] = &[
    "all",
    "roads",
    "buildings",
    "water",
    "landuse",
    "trails",
    "parks",
    "rail",
    "amenities",
    "boundaries",
    "transit",
    "poi",
];

fn is_valid_preset(preset: &str) -> bool {
    OSM_FILTER_PRESETS.iter().any(|p| *p == preset)
}

const FILTER_PRESET_DESCRIPTION: &str = "Feature class preset: all, roads, buildings, water, landuse, trails, parks, rail, amenities, boundaries, transit, poi (default all).";

fn preset_to_overpass_filters(preset: &str) -> Vec<String> {
    match preset {
        "roads"      => vec!["[highway]".to_string()],
        "buildings"  => vec!["[building]".to_string()],
        "water"      => vec!["[waterway]".to_string(), "[natural=water]".to_string()],
        "landuse"    => vec!["[landuse]".to_string()],
        "trails"     => vec!["[highway=path]".to_string(), "[highway=footway]".to_string(), "[highway=cycleway]".to_string(), "[highway=bridleway]".to_string()],
        "parks"      => vec!["[leisure=park]".to_string(), "[boundary=national_park]".to_string(), "[landuse=recreation_ground]".to_string(), "[leisure=nature_reserve]".to_string()],
        "rail"       => vec!["[railway]".to_string()],
        "amenities"  => vec!["[amenity]".to_string()],
        "boundaries" => vec!["[boundary]".to_string()],
        "transit"    => vec!["[public_transport]".to_string(), "[railway=station]".to_string(), "[highway=bus_stop]".to_string()],
        "poi"        => vec!["[amenity]".to_string(), "[tourism]".to_string(), "[shop]".to_string(), "[leisure]".to_string()],
        _ => vec![],  // "all" or unrecognised → no tag filter
    }
}

// ── Area-way heuristic ─────────────────────────────────────────────────────────

fn is_area_way(tags: &serde_json::Map<String, serde_json::Value>) -> bool {
    if tags.get("area").and_then(|v| v.as_str()) == Some("yes") {
        return true;
    }
    for key in &[
        "building", "landuse", "natural", "leisure", "amenity",
        "boundary", "sport", "place", "tourism",
    ] {
        if tags.contains_key(*key) {
            return true;
        }
    }
    false
}

// ── Primary classification key/value ──────────────────────────────────────────

fn primary_class(
    tags: Option<&serde_json::Map<String, serde_json::Value>>,
) -> (String, String) {
    const PRIORITY_KEYS: &[&str] = &[
        "highway", "building", "waterway", "natural", "landuse",
        "railway", "amenity", "shop", "leisure", "tourism", "boundary", "place",
    ];
    if let Some(t) = tags {
        for key in PRIORITY_KEYS {
            if let Some(val) = t.get(*key) {
                let val_str = val.as_str().unwrap_or("yes").to_string();
                return (key.to_string(), val_str);
            }
        }
    }
    (String::new(), String::new())
}

// ── Overpass query builder ─────────────────────────────────────────────────────

fn build_overpass_query(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    include_nodes: bool,
    include_ways: bool,
    include_relations: bool,
    filters: &[String],
    timeout: u64,
) -> String {
    // Overpass bbox order: south, west, north, east
    let bbox = format!("{:.7},{:.7},{:.7},{:.7}", south, west, north, east);
    let mut parts = Vec::new();
    if filters.is_empty() {
        if include_nodes {
            parts.push(format!("  node({});", bbox));
        }
        if include_ways {
            parts.push(format!("  way({});", bbox));
        }
        if include_relations {
            parts.push(format!("  relation({});", bbox));
        }
    } else {
        for filter in filters {
            if include_nodes {
                parts.push(format!("  node{}({});", filter, bbox));
            }
            if include_ways {
                parts.push(format!("  way{}({});", filter, bbox));
            }
            if include_relations {
                parts.push(format!("  relation{}({});", filter, bbox));
            }
        }
    }
    format!(
        "[out:json][timeout:{timeout}];\n(\n{parts}\n);\nout body;\n>;\nout skel qt;",
        timeout = timeout,
        parts = parts.join("\n"),
    )
}

// ── Percent-encode for application/x-www-form-urlencoded ──────────────────────

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(
                    char::from_digit((b >> 4) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit((b & 0xf) as u32, 16)
                        .unwrap_or('0')
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    out
}

// ── HTTP fetch ─────────────────────────────────────────────────────────────────

fn fetch_overpass(
    endpoint: &str,
    query: &str,
    timeout_secs: u64,
) -> Result<serde_json::Value, ToolError> {
    let body = format!("data={}", url_encode(query));
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout(timeout)
        .build();

    let do_request = |agent: &ureq::Agent| {
        agent
            .post(endpoint)
            .set("Content-Type", "application/x-www-form-urlencoded")
            .send_string(&body)
    };

    let mut last_error: Option<String> = None;
    let max_attempts = 4usize;
    for attempt in 0..max_attempts {
        match do_request(&agent) {
            Ok(resp) => {
                if resp.status() == 200 {
                    return resp.into_json::<serde_json::Value>().map_err(|e| {
                        ToolError::Execution(format!("failed parsing Overpass JSON: {}", e))
                    });
                }
                last_error = Some(format!("Overpass returned HTTP {}", resp.status()));
            }
            Err(ureq::Error::Status(code, _)) if code == 429 || code >= 500 => {
                last_error = Some(format!("Overpass returned HTTP {}", code));
            }
            Err(e) => {
                return Err(ToolError::Execution(format!("Overpass request failed: {}", e)));
            }
        }

        if attempt + 1 < max_attempts {
            let backoff_secs = 1u64 << attempt.min(3);
            std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
        }
    }

    let msg = last_error.unwrap_or_else(|| "Overpass request failed".to_string());
    Err(ToolError::Execution(format!(
        "{} after {} attempt(s)",
        msg, max_attempts
    )))
}

fn overpass_cache_file_path(cache_dir: &Path, endpoint: &str, query: &str) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    endpoint.hash(&mut hasher);
    query.hash(&mut hasher);
    let key = hasher.finish();
    cache_dir.join(format!("overpass_{:016x}.json", key))
}

fn try_read_cached_overpass(path: &Path, ttl_hours: u64) -> Option<serde_json::Value> {
    if !path.exists() {
        return None;
    }

    if ttl_hours > 0 {
        let max_age = std::time::Duration::from_secs(ttl_hours.saturating_mul(3600));
        let modified = std::fs::metadata(path).ok()?.modified().ok()?;
        let age = modified.elapsed().ok()?;
        if age > max_age {
            return None;
        }
    }

    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<serde_json::Value>(&raw).ok()
}

fn write_cached_overpass(path: &Path, value: &serde_json::Value) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ToolError::Execution(format!("failed creating cache directory: {}", e))
        })?;
    }
    let text = serde_json::to_string(value)
        .map_err(|e| ToolError::Execution(format!("failed serializing cache JSON: {}", e)))?;
    std::fs::write(path, text)
        .map_err(|e| ToolError::Execution(format!("failed writing cache file: {}", e)))?;
    Ok(())
}

fn fetch_overpass_with_optional_cache(
    endpoint: &str,
    query: &str,
    timeout_secs: u64,
    cache_dir: Option<&str>,
    cache_ttl_hours: u64,
) -> Result<(serde_json::Value, bool), ToolError> {
    if let Some(dir) = cache_dir {
        let cache_path = overpass_cache_file_path(Path::new(dir), endpoint, query);
        if let Some(cached) = try_read_cached_overpass(&cache_path, cache_ttl_hours) {
            return Ok((cached, true));
        }
        let fetched = fetch_overpass(endpoint, query, timeout_secs)?;
        let _ = write_cached_overpass(&cache_path, &fetched);
        Ok((fetched, false))
    } else {
        fetch_overpass(endpoint, query, timeout_secs).map(|v| (v, false))
    }
}

fn fetch_parse_chunk(
    endpoint: &str,
    bbox: (f64, f64, f64, f64),
    include_points: bool,
    include_lines: bool,
    include_polygons: bool,
    filters: &[String],
    timeout_secs: u64,
    max_elements: u64,
    cache_dir: Option<&str>,
    cache_ttl_hours: u64,
) -> Result<(ParseResult, bool), ToolError> {
    let (west, south, east, north) = bbox;
    let include_ways = include_lines || include_polygons;
    let query = build_overpass_query(
        west,
        south,
        east,
        north,
        include_points,
        include_ways,
        include_polygons,
        filters,
        timeout_secs,
    );

    let (json, used_cache) = fetch_overpass_with_optional_cache(
        endpoint,
        &query,
        timeout_secs,
        cache_dir,
        cache_ttl_hours,
    )?;
    let parsed = parse_overpass_response(
        &json,
        include_points,
        include_lines,
        include_polygons,
        max_elements,
    )?;
    Ok((parsed, used_cache))
}

// ── Overpass JSON → Layer ──────────────────────────────────────────────────────

struct ParseResult {
    layer: wbvector::Layer,
    skipped: usize,
}

fn parse_overpass_response(
    json: &serde_json::Value,
    include_points: bool,
    include_lines: bool,
    include_polygons: bool,
    max_elements: u64,
) -> Result<ParseResult, ToolError> {
    let elements = json
        .get("elements")
        .and_then(|e| e.as_array())
        .ok_or_else(|| {
            ToolError::Execution(
                "Overpass response missing 'elements' array".to_string(),
            )
        })?;

    if elements.len() as u64 > max_elements {
        return Err(ToolError::Execution(format!(
            "response contains {} elements, exceeding max_elements limit of {}; \
             reduce AOI or add a filter preset",
            elements.len(),
            max_elements
        )));
    }

    // Build node id → (lon, lat) lookup for way geometry resolution.
    let mut node_coords: HashMap<i64, (f64, f64)> = HashMap::with_capacity(elements.len());
    for el in elements {
        if el.get("type").and_then(|t| t.as_str()) != Some("node") {
            continue;
        }
        if let (Some(id), Some(lat), Some(lon)) = (
            el.get("id").and_then(|v| v.as_i64()),
            el.get("lat").and_then(|v| v.as_f64()),
            el.get("lon").and_then(|v| v.as_f64()),
        ) {
            node_coords.insert(id, (lon, lat));
        }
    }

    // Build way id -> coordinate sequence lookup so relation members can reuse way geometries.
    let mut way_coords: HashMap<i64, Vec<(f64, f64)>> = HashMap::new();
    for el in elements {
        if el.get("type").and_then(|t| t.as_str()) != Some("way") {
            continue;
        }
        let Some(way_id) = el.get("id").and_then(|v| v.as_i64()) else {
            continue;
        };
        let Some(node_ids) = el.get("nodes").and_then(|n| n.as_array()) else {
            continue;
        };
        let coords: Vec<(f64, f64)> = node_ids
            .iter()
            .filter_map(|nid| nid.as_i64().and_then(|i| node_coords.get(&i).copied()))
            .collect();
        if coords.len() >= 2 {
            way_coords.insert(way_id, coords);
        }
    }

    let ring_from_coords = |coords: &[(f64, f64)]| -> Vec<wbvector::Coord> {
        let mut out: Vec<wbvector::Coord> = coords
            .iter()
            .map(|(x, y)| wbvector::Coord::xy(*x, *y))
            .collect();
        if out.len() >= 3 {
            let first = out[0].clone();
            let last = out[out.len() - 1].clone();
            if (first.x - last.x).abs() > 1.0e-9 || (first.y - last.y).abs() > 1.0e-9 {
                out.push(first);
            }
        }
        out
    };

    let mut layer = wbvector::Layer::new("osm_download");
    layer.assign_crs_epsg(4326);
    layer.add_field(wbvector::FieldDef::new("osm_id",      wbvector::FieldType::Integer));
    layer.add_field(wbvector::FieldDef::new("osm_type",    wbvector::FieldType::Text));
    layer.add_field(wbvector::FieldDef::new("name",        wbvector::FieldType::Text));
    layer.add_field(wbvector::FieldDef::new("class_key",   wbvector::FieldType::Text));
    layer.add_field(wbvector::FieldDef::new("class_value", wbvector::FieldType::Text));
    layer.add_field(wbvector::FieldDef::new("osm_tags",    wbvector::FieldType::Text));

    let mut skipped = 0usize;

    for el in elements {
        let el_type = match el.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let id = el.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let tags_obj = el.get("tags").and_then(|t| t.as_object());
        let name = tags_obj
            .and_then(|t| t.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let (class_key, class_value) = primary_class(tags_obj);
        let tags_text = tags_obj
            .map(|t| serde_json::to_string(t).unwrap_or_default())
            .unwrap_or_default();

        let attrs: &[(&str, wbvector::FieldValue)] = &[
            ("osm_id",      wbvector::FieldValue::Integer(id)),
            ("osm_type",    wbvector::FieldValue::Text(el_type.to_string())),
            ("name",        wbvector::FieldValue::Text(name)),
            ("class_key",   wbvector::FieldValue::Text(class_key)),
            ("class_value", wbvector::FieldValue::Text(class_value)),
            ("osm_tags",    wbvector::FieldValue::Text(tags_text)),
        ];

        match el_type {
            "node" if include_points => {
                let lat = el.get("lat").and_then(|v| v.as_f64());
                let lon = el.get("lon").and_then(|v| v.as_f64());
                if let (Some(lat), Some(lon)) = (lat, lon) {
                    // Skip bare geometry nodes (no tags) used only as way members
                    let has_tags = tags_obj.map(|t| !t.is_empty()).unwrap_or(false);
                    if has_tags {
                        let geom = wbvector::Geometry::point(lon, lat);
                        let _ = layer.add_feature(Some(geom), attrs);
                    }
                }
            }
            "way" => {
                let node_ids = match el.get("nodes").and_then(|n| n.as_array()) {
                    Some(n) => n,
                    None => {
                        skipped += 1;
                        continue;
                    }
                };
                let coords: Vec<(f64, f64)> = node_ids
                    .iter()
                    .filter_map(|nid| nid.as_i64().and_then(|i| node_coords.get(&i).copied()))
                    .collect();
                if coords.len() < 2 {
                    skipped += 1;
                    continue;
                }
                let n = coords.len();
                let is_closed = n >= 4
                    && (coords[0].0 - coords[n - 1].0).abs() < 1e-9
                    && (coords[0].1 - coords[n - 1].1).abs() < 1e-9;
                let make_polygon = is_closed
                    && include_polygons
                    && tags_obj.map(is_area_way).unwrap_or(false);

                if make_polygon {
                    let ring: Vec<wbvector::Coord> = coords
                        .iter()
                        .map(|(x, y)| wbvector::Coord::xy(*x, *y))
                        .collect();
                    let geom = wbvector::Geometry::polygon(ring, vec![]);
                    let _ = layer.add_feature(Some(geom), attrs);
                } else if include_lines {
                    let wbcoords: Vec<wbvector::Coord> = coords
                        .iter()
                        .map(|(x, y)| wbvector::Coord::xy(*x, *y))
                        .collect();
                    let geom = wbvector::Geometry::line_string(wbcoords);
                    let _ = layer.add_feature(Some(geom), attrs);
                }
            }
            "relation" if include_polygons => {
                let rel_type = tags_obj
                    .and_then(|t| t.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if rel_type != "multipolygon" && rel_type != "boundary" {
                    continue;
                }

                let members = match el.get("members").and_then(|m| m.as_array()) {
                    Some(m) => m,
                    None => {
                        skipped += 1;
                        continue;
                    }
                };

                let mut outers: Vec<Vec<wbvector::Coord>> = Vec::new();
                let mut inners: Vec<Vec<wbvector::Coord>> = Vec::new();

                for m in members {
                    if m.get("type").and_then(|v| v.as_str()) != Some("way") {
                        continue;
                    }
                    let Some(way_id) = m.get("ref").and_then(|v| v.as_i64()) else {
                        continue;
                    };
                    let Some(coords) = way_coords.get(&way_id) else {
                        continue;
                    };
                    let ring = ring_from_coords(coords);
                    if ring.len() < 4 {
                        continue;
                    }
                    let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("");
                    if role == "inner" {
                        inners.push(ring);
                    } else {
                        outers.push(ring);
                    }
                }

                if outers.is_empty() {
                    skipped += 1;
                    continue;
                }

                // MVP relation handling: create one polygon feature per outer ring.
                // If exactly one outer exists, attach all discovered inner rings as holes.
                if outers.len() == 1 {
                    let geom = wbvector::Geometry::polygon(outers.remove(0), inners);
                    let _ = layer.add_feature(Some(geom), attrs);
                } else {
                    for outer in outers {
                        let geom = wbvector::Geometry::polygon(outer, vec![]);
                        let _ = layer.add_feature(Some(geom), attrs);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ParseResult { layer, skipped })
}

// ── Parameter helpers ──────────────────────────────────────────────────────────

fn parse_bbox(args: &ToolArgs) -> Result<(f64, f64, f64, f64), ToolError> {
    let get = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))
    };
    Ok((get("west")?, get("south")?, get("east")?, get("north")?))
}

fn validate_bbox(west: f64, south: f64, east: f64, north: f64) -> Result<(), ToolError> {
    if west >= east {
        return Err(ToolError::Validation(
            "'west' must be less than 'east'".to_string(),
        ));
    }
    if south >= north {
        return Err(ToolError::Validation(
            "'south' must be less than 'north'".to_string(),
        ));
    }
    if west < -180.0 || east > 180.0 || south < -90.0 || north > 90.0 {
        return Err(ToolError::Validation(
            "bbox coordinates must be in valid geographic range \
             (lon: -180..180, lat: -90..90)"
                .to_string(),
        ));
    }
    let area_deg2 = (east - west) * (north - south);
    if area_deg2 > 25.0 {
        return Err(ToolError::Validation(
            "bounding box is too large (> 25 square degrees); \
             use a smaller AOI to avoid overloading the Overpass endpoint"
                .to_string(),
        ));
    }
    Ok(())
}

fn resolve_overpass_endpoint(args: &ToolArgs) -> Result<String, ToolError> {
    let profile = args
        .get("overpass_profile")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_else(|| "main".to_string());

    let profile_url = match profile.as_str() {
        "main" => Some("https://overpass-api.de/api/interpreter"),
        "kumi" => Some("https://overpass.kumi.systems/api/interpreter"),
        "fr" => Some("https://overpass.openstreetmap.fr/api/interpreter"),
        "private" | "custom" => None,
        _ => {
            return Err(ToolError::Validation(format!(
                "invalid overpass_profile '{}'; expected one of: main, kumi, fr, custom",
                profile
            )))
        }
    };

    let explicit = args
        .get("overpass_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(explicit.unwrap_or_else(|| {
        profile_url
            .unwrap_or("https://overpass-api.de/api/interpreter")
            .to_string()
    }))
}

fn transform_bbox_to_wgs84(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    src_epsg: u32,
) -> Result<(f64, f64, f64, f64), ToolError> {
    let src = Crs::from_epsg(src_epsg)
        .map_err(|e| ToolError::Validation(format!("invalid input_extent_epsg {}: {}", src_epsg, e)))?;
    let dst = Crs::from_epsg(4326)
        .map_err(|e| ToolError::Execution(format!("failed loading EPSG:4326 CRS: {}", e)))?;

    let corners = [
        (west, south),
        (west, north),
        (east, south),
        (east, north),
    ];

    let mut out: Vec<(f64, f64)> = Vec::with_capacity(4);
    for (x, y) in corners {
        let (lon, lat) = src.transform_to(x, y, &dst).map_err(|e| {
            ToolError::Validation(format!(
                "failed transforming bbox corner ({}, {}) from EPSG:{} to EPSG:4326: {}",
                x, y, src_epsg, e
            ))
        })?;
        out.push((lon, lat));
    }

    let min_lon = out.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_lon = out.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let min_lat = out.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_lat = out.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

    Ok((min_lon, min_lat, max_lon, max_lat))
}

fn resolve_query_bbox(args: &ToolArgs) -> Result<(f64, f64, f64, f64), ToolError> {
    let (west, south, east, north) = parse_bbox(args)?;
    if west >= east {
        return Err(ToolError::Validation(
            "'west' must be less than 'east'".to_string(),
        ));
    }
    if south >= north {
        return Err(ToolError::Validation(
            "'south' must be less than 'north'".to_string(),
        ));
    }

    let input_extent_epsg = args
        .get("input_extent_epsg")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(4326);

    let (q_west, q_south, q_east, q_north) = if input_extent_epsg == 4326 {
        (west, south, east, north)
    } else {
        transform_bbox_to_wgs84(west, south, east, north, input_extent_epsg)?
    };

    validate_bbox(q_west, q_south, q_east, q_north)?;
    Ok((q_west, q_south, q_east, q_north))
}

fn plan_chunked_bboxes(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    max_tile_area_deg2: f64,
    max_chunk_count: usize,
) -> Result<Vec<(f64, f64, f64, f64)>, ToolError> {
    if max_tile_area_deg2 <= 0.0 {
        return Err(ToolError::Validation(
            "chunk_max_area_deg2 must be > 0".to_string(),
        ));
    }
    let width = east - west;
    let height = north - south;
    let area = width * height;
    if area <= max_tile_area_deg2 {
        return Ok(vec![(west, south, east, north)]);
    }

    let target_tiles = (area / max_tile_area_deg2).ceil().max(1.0);
    let aspect = if height.abs() > 1.0e-12 { width / height } else { 1.0 };
    let nx = ((target_tiles * aspect).sqrt().ceil().max(1.0)) as usize;
    let ny = (target_tiles / nx as f64).ceil().max(1.0) as usize;
    let total = nx.saturating_mul(ny);

    if total > max_chunk_count {
        return Err(ToolError::Validation(format!(
            "chunking would require {} tiles, exceeding max_chunk_count {}",
            total, max_chunk_count
        )));
    }

    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut out = Vec::with_capacity(total);
    for iy in 0..ny {
        let y0 = south + dy * iy as f64;
        let y1 = if iy + 1 == ny { north } else { south + dy * (iy + 1) as f64 };
        for ix in 0..nx {
            let x0 = west + dx * ix as f64;
            let x1 = if ix + 1 == nx { east } else { west + dx * (ix + 1) as f64 };
            out.push((x0, y0, x1, y1));
        }
    }
    Ok(out)
}

fn dedupe_layer_by_osm_identity(layer: &mut wbvector::Layer) {
    let Some(id_idx) = layer.schema.field_index("osm_id") else {
        return;
    };
    let Some(type_idx) = layer.schema.field_index("osm_type") else {
        return;
    };

    let mut seen: HashSet<String> = HashSet::new();
    layer.features.retain(|f| {
        let id_key = match f.attributes.get(id_idx) {
            Some(wbvector::FieldValue::Integer(v)) => v.to_string(),
            Some(wbvector::FieldValue::Text(v)) => v.clone(),
            Some(wbvector::FieldValue::Float(v)) => format!("{:.0}", v),
            _ => String::new(),
        };
        let type_key = match f.attributes.get(type_idx) {
            Some(wbvector::FieldValue::Text(v)) => v.clone(),
            _ => String::new(),
        };
        let key = format!("{}:{}", type_key, id_key);
        seen.insert(key)
    });
}

fn split_key_value(kv: &str) -> (&str, &str) {
    if let Some(pos) = kv.find('=') {
        (kv[..pos].trim(), kv[pos + 1..].trim())
    } else {
        (kv.trim(), "")
    }
}

fn parse_semicolon_list(raw: Option<&str>) -> Vec<String> {
    raw
        .map(|s| {
            s.split(';')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn build_filter_clauses(args: &ToolArgs, preset: &str) -> Vec<String> {
    let include_tags = parse_semicolon_list(
        args.get("include_tags").and_then(|v| v.as_str())
            .or_else(|| args.get("filter_key").and_then(|v| v.as_str())),
    );
    let include_key_values = parse_semicolon_list(
        args.get("include_key_values").and_then(|v| v.as_str())
            .or_else(|| args.get("filter_key_value").and_then(|v| v.as_str())),
    );

    if include_tags.is_empty() && include_key_values.is_empty() {
        return preset_to_overpass_filters(preset);
    }

    let mut filters = Vec::<String>::new();
    for key in include_tags {
        filters.push(format!("[{}]", key));
    }
    for kv in include_key_values {
        let (k, v) = split_key_value(&kv);
        if k.is_empty() {
            continue;
        }
        if v.is_empty() {
            filters.push(format!("[{}]", k));
        } else {
            filters.push(format!("[{}={}]", k, v));
        }
    }
    filters.sort();
    filters.dedup();
    filters
}

const CLIP_EPS: f64 = 1.0e-12;

fn point_inside_bbox(x: f64, y: f64, west: f64, south: f64, east: f64, north: f64) -> bool {
    x >= west - CLIP_EPS && x <= east + CLIP_EPS && y >= south - CLIP_EPS && y <= north + CLIP_EPS
}

fn out_code(x: f64, y: f64, west: f64, south: f64, east: f64, north: f64) -> u8 {
    let mut code = 0u8;
    if x < west { code |= 1; }
    if x > east { code |= 2; }
    if y < south { code |= 4; }
    if y > north { code |= 8; }
    code
}

fn clip_segment_to_bbox(
    mut x0: f64,
    mut y0: f64,
    mut x1: f64,
    mut y1: f64,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let mut c0 = out_code(x0, y0, west, south, east, north);
    let mut c1 = out_code(x1, y1, west, south, east, north);

    loop {
        if (c0 | c1) == 0 {
            return Some(((x0, y0), (x1, y1)));
        }
        if (c0 & c1) != 0 {
            return None;
        }

        let co = if c0 != 0 { c0 } else { c1 };
        let mut x = 0.0;
        let mut y = 0.0;

        if (co & 8) != 0 {
            if (y1 - y0).abs() < CLIP_EPS { return None; }
            x = x0 + (x1 - x0) * (north - y0) / (y1 - y0);
            y = north;
        } else if (co & 4) != 0 {
            if (y1 - y0).abs() < CLIP_EPS { return None; }
            x = x0 + (x1 - x0) * (south - y0) / (y1 - y0);
            y = south;
        } else if (co & 2) != 0 {
            if (x1 - x0).abs() < CLIP_EPS { return None; }
            y = y0 + (y1 - y0) * (east - x0) / (x1 - x0);
            x = east;
        } else if (co & 1) != 0 {
            if (x1 - x0).abs() < CLIP_EPS { return None; }
            y = y0 + (y1 - y0) * (west - x0) / (x1 - x0);
            x = west;
        }

        if co == c0 {
            x0 = x;
            y0 = y;
            c0 = out_code(x0, y0, west, south, east, north);
        } else {
            x1 = x;
            y1 = y;
            c1 = out_code(x1, y1, west, south, east, north);
        }
    }
}

fn clip_ring_sutherland_hodgman(
    ring: &[wbvector::Coord],
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> Vec<wbvector::Coord> {
    if ring.len() < 3 {
        return Vec::new();
    }

    let mut pts: Vec<(f64, f64)> = ring.iter().map(|c| (c.x, c.y)).collect();
    if let Some(first) = pts.first().copied() {
        if pts.last().copied() == Some(first) {
            pts.pop();
        }
    }

    let clip_edge = |input: &Vec<(f64, f64)>, inside: &dyn Fn((f64, f64)) -> bool, intersect: &dyn Fn((f64, f64), (f64, f64)) -> (f64, f64)| {
        if input.is_empty() {
            return Vec::new();
        }
        let mut output = Vec::<(f64, f64)>::new();
        let mut s = *input.last().unwrap_or(&(0.0, 0.0));
        for &e in input {
            let s_in = inside(s);
            let e_in = inside(e);
            if e_in {
                if !s_in {
                    output.push(intersect(s, e));
                }
                output.push(e);
            } else if s_in {
                output.push(intersect(s, e));
            }
            s = e;
        }
        output
    };

    let mut out = pts;
    out = clip_edge(&out, &|p| p.0 >= west - CLIP_EPS, &|s, e| {
        let t = if (e.0 - s.0).abs() < CLIP_EPS { 0.0 } else { (west - s.0) / (e.0 - s.0) };
        (west, s.1 + t * (e.1 - s.1))
    });
    out = clip_edge(&out, &|p| p.0 <= east + CLIP_EPS, &|s, e| {
        let t = if (e.0 - s.0).abs() < CLIP_EPS { 0.0 } else { (east - s.0) / (e.0 - s.0) };
        (east, s.1 + t * (e.1 - s.1))
    });
    out = clip_edge(&out, &|p| p.1 >= south - CLIP_EPS, &|s, e| {
        let t = if (e.1 - s.1).abs() < CLIP_EPS { 0.0 } else { (south - s.1) / (e.1 - s.1) };
        (s.0 + t * (e.0 - s.0), south)
    });
    out = clip_edge(&out, &|p| p.1 <= north + CLIP_EPS, &|s, e| {
        let t = if (e.1 - s.1).abs() < CLIP_EPS { 0.0 } else { (north - s.1) / (e.1 - s.1) };
        (s.0 + t * (e.0 - s.0), north)
    });

    if out.len() < 3 {
        return Vec::new();
    }
    if out.first() != out.last() {
        out.push(*out.first().unwrap_or(&(0.0, 0.0)));
    }

    out.into_iter().map(|(x, y)| wbvector::Coord::xy(x, y)).collect()
}

fn clip_layer_to_bbox(
    layer: &wbvector::Layer,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> wbvector::Layer {
    let mut out = layer.clone();
    out.features.clear();

    for f in &layer.features {
        let Some(g) = &f.geometry else {
            continue;
        };

        let clipped = match g {
            wbvector::Geometry::Point(c) => {
                if point_inside_bbox(c.x, c.y, west, south, east, north) {
                    Some(wbvector::Geometry::Point(c.clone()))
                } else {
                    None
                }
            }
            wbvector::Geometry::LineString(coords) => {
                if coords.len() < 2 {
                    None
                } else {
                    let mut parts = Vec::<Vec<wbvector::Coord>>::new();
                    let mut current = Vec::<wbvector::Coord>::new();
                    for win in coords.windows(2) {
                        let a = &win[0];
                        let b = &win[1];
                        if let Some(((x0, y0), (x1, y1))) = clip_segment_to_bbox(
                            a.x, a.y, b.x, b.y, west, south, east, north,
                        ) {
                            let c0 = wbvector::Coord::xy(x0, y0);
                            let c1 = wbvector::Coord::xy(x1, y1);
                            if current.is_empty() {
                                current.push(c0);
                                current.push(c1);
                            } else if current.last().map(|p| (p.x - c0.x).abs() <= CLIP_EPS && (p.y - c0.y).abs() <= CLIP_EPS).unwrap_or(false) {
                                current.push(c1);
                            } else {
                                if current.len() >= 2 {
                                    parts.push(current);
                                }
                                current = vec![c0, c1];
                            }
                        } else if current.len() >= 2 {
                            parts.push(current);
                            current = Vec::new();
                        }
                    }
                    if current.len() >= 2 {
                        parts.push(current);
                    }

                    if parts.is_empty() {
                        None
                    } else if parts.len() == 1 {
                        Some(wbvector::Geometry::line_string(parts.remove(0)))
                    } else {
                        Some(wbvector::Geometry::multi_line_string(parts))
                    }
                }
            }
            wbvector::Geometry::Polygon { exterior, interiors: _ } => {
                let ring = clip_ring_sutherland_hodgman(&exterior.0, west, south, east, north);
                if ring.len() >= 4 {
                    Some(wbvector::Geometry::Polygon {
                        exterior: wbvector::Ring(ring),
                        interiors: Vec::new(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new_geom) = clipped {
            let mut new_feat = f.clone();
            new_feat.geometry = Some(new_geom);
            out.features.push(new_feat);
        }
    }

    out
}

fn parse_vector_output_path(args: &ToolArgs) -> Result<String, ToolError> {
    if let Some(path) = args
        .get("output")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return Ok(path);
    }

    // For object-first wrapper calls, create a temporary output path.
    let mut temp = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    temp.push(format!("wbw_download_osm_vector_{}.gpkg", stamp));
    Ok(temp.to_string_lossy().to_string())
}

fn detect_vector_output_format(path: &str) -> Result<wbvector::VectorFormat, ToolError> {
    match wbvector::VectorFormat::detect(path) {
        Ok(fmt) => Ok(fmt),
        Err(_) => {
            if std::path::Path::new(path).extension().is_none() {
                Ok(wbvector::VectorFormat::Shapefile)
            } else {
                Err(ToolError::Validation(format!(
                    "could not determine vector output format from path '{}'",
                    path
                )))
            }
        }
    }
}

fn write_vector_output(layer: &wbvector::Layer, path: &str) -> Result<String, ToolError> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating output directory: {}", e))
            })?;
        }
    }
    let format = detect_vector_output_format(path)?;
    wbvector::write(layer, path, format)
        .map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;
    Ok(path.to_string())
}

fn split_layer_by_geometry(layer: &wbvector::Layer) -> (wbvector::Layer, wbvector::Layer, wbvector::Layer) {
    let mut points = layer.clone();
    let mut lines = layer.clone();
    let mut polygons = layer.clone();
    points.features.clear();
    lines.features.clear();
    polygons.features.clear();

    for f in &layer.features {
        match f.geometry.as_ref() {
            Some(wbvector::Geometry::Point(_)) | Some(wbvector::Geometry::MultiPoint(_)) => {
                points.features.push(f.clone());
            }
            Some(wbvector::Geometry::LineString(_)) | Some(wbvector::Geometry::MultiLineString(_)) => {
                lines.features.push(f.clone());
            }
            Some(wbvector::Geometry::Polygon { .. }) | Some(wbvector::Geometry::MultiPolygon(_)) => {
                polygons.features.push(f.clone());
            }
            _ => {}
        }
    }

    (points, lines, polygons)
}

fn derive_split_output_paths(output: &str) -> Result<(String, String, String), ToolError> {
    let p = std::path::Path::new(output);
    let ext = p.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        ToolError::Validation(
            "split_output_by_geometry requires an output filename extension, e.g. .geojson or .gpkg"
                .to_string(),
        )
    })?;
    let stem = p.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        ToolError::Validation("invalid output path for split_output_by_geometry".to_string())
    })?;
    let parent = p.parent().unwrap_or_else(|| std::path::Path::new(""));

    let mk = |suffix: &str| {
        parent
            .join(format!("{}_{}.{}", stem, suffix, ext))
            .to_string_lossy()
            .to_string()
    };

    Ok((mk("points"), mk("lines"), mk("polygons")))
}

fn write_json_sidecar(path: &str, value: &serde_json::Value) -> Result<String, ToolError> {
    let p = std::path::Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ToolError::Execution(format!("failed creating sidecar directory: {}", e))
            })?;
        }
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| ToolError::Execution(format!("failed serializing provenance JSON: {}", e)))?;
    std::fs::write(p, text)
        .map_err(|e| ToolError::Execution(format!("failed writing provenance JSON: {}", e)))?;
    Ok(path.to_string())
}

// ── Tool impl ──────────────────────────────────────────────────────────────────

impl Tool for DownloadOsmVectorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "download_osm_vector",
            display_name: "Download OSM Vector",
            summary: "Downloads OpenStreetMap features from the Overpass API for a bounding box \
                      and writes the result as a vector layer.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "west",  description: "West boundary longitude (EPSG:4326).",  required: true  },
                ToolParamSpec { name: "south", description: "South boundary latitude (EPSG:4326).",  required: true  },
                ToolParamSpec { name: "east",  description: "East boundary longitude (EPSG:4326).",  required: true  },
                ToolParamSpec { name: "north", description: "North boundary latitude (EPSG:4326).",  required: true  },
                ToolParamSpec { name: "input_extent_epsg", description: "EPSG code for west/south/east/north input extent (default 4326).",                     required: false },
                ToolParamSpec { name: "filter_preset",   description: FILTER_PRESET_DESCRIPTION,                                                                       required: false },
                ToolParamSpec { name: "include_tags",    description: "Semicolon-delimited tag keys filter list, e.g. amenity;shop.",                                         required: false },
                ToolParamSpec { name: "include_key_values",description: "Semicolon-delimited key=value filter list, e.g. amenity=school;building=yes.",                     required: false },
                ToolParamSpec { name: "filter_key",      description: "Legacy single tag-key filter (deprecated; use include_tags).",                                          required: false },
                ToolParamSpec { name: "filter_key_value",description: "Legacy single key=value filter (deprecated; use include_key_values).",                                 required: false },
                ToolParamSpec { name: "include_points",  description: "Include OSM node features with tags as Point geometries (default true).",                             required: false },
                ToolParamSpec { name: "include_lines",   description: "Include OSM way features as LineString geometries (default true).",                                   required: false },
                ToolParamSpec { name: "include_polygons",description: "Include closed OSM ways with area tags as Polygon geometries (default true).",                        required: false },
                ToolParamSpec { name: "clip_to_extent",  description: "Clip output geometries to the query bbox extent (default true).",                                      required: false },
                ToolParamSpec { name: "split_output_by_geometry", description: "Write separate outputs per geometry type (<output>_points|lines|polygons.ext).",              required: false },
                ToolParamSpec { name: "output_epsg",     description: "Reproject output to this EPSG code; omit to keep EPSG:4326.",                                         required: false },
                ToolParamSpec { name: "overpass_profile", description: "Overpass endpoint profile: main, kumi, fr, or custom (default main).",                  required: false },
                ToolParamSpec { name: "overpass_url",    description: "Overpass API endpoint URL (default https://overpass-api.de/api/interpreter).",                        required: false },
                ToolParamSpec { name: "timeout_seconds", description: "Query timeout in seconds (default 25).",                                                              required: false },
                ToolParamSpec { name: "max_elements",    description: "Maximum number of returned Overpass elements before an error is raised (default 50 000).",            required: false },
                ToolParamSpec { name: "chunk_large_aoi", description: "Automatically split larger AOIs into multiple Overpass requests (default true).",                      required: false },
                ToolParamSpec { name: "chunk_max_area_deg2", description: "Maximum area per chunk in square degrees when chunking (default 4.0).",                           required: false },
                ToolParamSpec { name: "max_chunk_count", description: "Hard cap on chunk count when chunking large AOIs (default 64).",                                       required: false },
                ToolParamSpec { name: "chunk_parallel_requests", description: "Maximum parallel chunk requests for large AOIs (default 1 = sequential).",                     required: false },
                ToolParamSpec { name: "cache_dir",       description: "Optional directory for caching Overpass JSON responses by endpoint+query hash.",                       required: false },
                ToolParamSpec { name: "cache_ttl_hours", description: "Cache time-to-live in hours (default 24; 0 means no expiry check).",                                  required: false },
                ToolParamSpec { name: "provenance_output", description: "Optional JSON path to write query/provenance metadata sidecar.",                                     required: false },
                ToolParamSpec { name: "output",          description: "Optional output vector path; when omitted, a temporary output is used.",                              required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("west".to_string(),            json!(-80.5));
        defaults.insert("south".to_string(),           json!(43.4));
        defaults.insert("east".to_string(),            json!(-80.4));
        defaults.insert("north".to_string(),           json!(43.5));
        defaults.insert("input_extent_epsg".to_string(),json!(4326));
        defaults.insert("filter_preset".to_string(),   json!("roads"));
        defaults.insert("include_tags".to_string(),    json!(""));
        defaults.insert("include_key_values".to_string(), json!(""));
        defaults.insert("include_points".to_string(),  json!(true));
        defaults.insert("include_lines".to_string(),   json!(true));
        defaults.insert("include_polygons".to_string(),json!(true));
        defaults.insert("clip_to_extent".to_string(),  json!(true));
        defaults.insert("split_output_by_geometry".to_string(), json!(false));
        defaults.insert("overpass_profile".to_string(),json!("main"));
        defaults.insert("overpass_url".to_string(),    json!("https://overpass-api.de/api/interpreter"));
        defaults.insert("timeout_seconds".to_string(), json!(25));
        defaults.insert("max_elements".to_string(),    json!(50_000));
        defaults.insert("chunk_large_aoi".to_string(), json!(true));
        defaults.insert("chunk_max_area_deg2".to_string(), json!(4.0));
        defaults.insert("max_chunk_count".to_string(), json!(64));
        defaults.insert("chunk_parallel_requests".to_string(), json!(1));
        defaults.insert("cache_ttl_hours".to_string(), json!(24));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("osm_roads.geojson"));

        ToolManifest {
            id: "download_osm_vector".to_string(),
            display_name: "Download OSM Vector".to_string(),
            summary: "Downloads OpenStreetMap features from the Overpass API for a bounding box \
                      and writes the result as a vector layer."
                .to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "west".to_string(),             description: "West boundary longitude (EPSG:4326).".to_string(),                                                                 required: true  },
                ToolParamDescriptor { name: "south".to_string(),            description: "South boundary latitude (EPSG:4326).".to_string(),                                                                 required: true  },
                ToolParamDescriptor { name: "east".to_string(),             description: "East boundary longitude (EPSG:4326).".to_string(),                                                                 required: true  },
                ToolParamDescriptor { name: "north".to_string(),            description: "North boundary latitude (EPSG:4326).".to_string(),                                                                 required: true  },
                ToolParamDescriptor { name: "input_extent_epsg".to_string(),description: "EPSG code for west/south/east/north input extent (default 4326).".to_string(),                     required: false },
                ToolParamDescriptor { name: "filter_preset".to_string(),    description: FILTER_PRESET_DESCRIPTION.to_string(),                                                            required: false },
                ToolParamDescriptor { name: "include_tags".to_string(),     description: "Semicolon-delimited tag keys filter list, e.g. amenity;shop.".to_string(),                                        required: false },
                ToolParamDescriptor { name: "include_key_values".to_string(), description: "Semicolon-delimited key=value filter list, e.g. amenity=school;building=yes.".to_string(),                     required: false },
                ToolParamDescriptor { name: "filter_key".to_string(),       description: "Legacy single tag-key filter (deprecated; use include_tags).".to_string(),                                         required: false },
                ToolParamDescriptor { name: "filter_key_value".to_string(), description: "Legacy single key=value filter (deprecated; use include_key_values).".to_string(),                                required: false },
                ToolParamDescriptor { name: "include_points".to_string(),   description: "Include OSM node features with tags as Point geometries (default true).".to_string(),                             required: false },
                ToolParamDescriptor { name: "include_lines".to_string(),    description: "Include OSM way features as LineString geometries (default true).".to_string(),                                   required: false },
                ToolParamDescriptor { name: "include_polygons".to_string(), description: "Include closed OSM ways with area tags as Polygon geometries (default true).".to_string(),                        required: false },
                ToolParamDescriptor { name: "clip_to_extent".to_string(),   description: "Clip output geometries to the query bbox extent (default true).".to_string(),                                     required: false },
                ToolParamDescriptor { name: "split_output_by_geometry".to_string(), description: "Write separate outputs per geometry type (<output>_points|lines|polygons.ext).".to_string(),              required: false },
                ToolParamDescriptor { name: "output_epsg".to_string(),      description: "Reproject output to this EPSG code; omit to keep EPSG:4326.".to_string(),                                         required: false },
                ToolParamDescriptor { name: "overpass_profile".to_string(), description: "Overpass endpoint profile: main, kumi, fr, or custom (default main).".to_string(),                  required: false },
                ToolParamDescriptor { name: "overpass_url".to_string(),     description: "Overpass API endpoint URL (default https://overpass-api.de/api/interpreter).".to_string(),                        required: false },
                ToolParamDescriptor { name: "timeout_seconds".to_string(),  description: "Query timeout in seconds (default 25).".to_string(),                                                              required: false },
                ToolParamDescriptor { name: "max_elements".to_string(),     description: "Maximum number of returned Overpass elements before an error is raised (default 50 000).".to_string(),            required: false },
                ToolParamDescriptor { name: "chunk_large_aoi".to_string(),  description: "Automatically split larger AOIs into multiple Overpass requests (default true).".to_string(),                      required: false },
                ToolParamDescriptor { name: "chunk_max_area_deg2".to_string(),description: "Maximum area per chunk in square degrees when chunking (default 4.0).".to_string(),                           required: false },
                ToolParamDescriptor { name: "max_chunk_count".to_string(),  description: "Hard cap on chunk count when chunking large AOIs (default 64).".to_string(),                                       required: false },
                ToolParamDescriptor { name: "chunk_parallel_requests".to_string(), description: "Maximum parallel chunk requests for large AOIs (default 1 = sequential).".to_string(),                     required: false },
                ToolParamDescriptor { name: "cache_dir".to_string(),        description: "Optional directory for caching Overpass JSON responses by endpoint+query hash.".to_string(),                       required: false },
                ToolParamDescriptor { name: "cache_ttl_hours".to_string(),  description: "Cache time-to-live in hours (default 24; 0 means no expiry check).".to_string(),                                  required: false },
                ToolParamDescriptor { name: "provenance_output".to_string(),description: "Optional JSON path to write query/provenance metadata sidecar.".to_string(),                                     required: false },
                ToolParamDescriptor { name: "output".to_string(),           description: "Optional output vector path; when omitted, a temporary output is used.".to_string(),  required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "download_osm_vector_roads".to_string(),
                description: "Downloads OSM road network for a small bounding box and writes to GeoJSON.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "vector".to_string(),
                "openstreetmap".to_string(),
                "osm".to_string(),
                "download".to_string(),
                "overpass".to_string(),
                "online".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let (west, south, east, north) = resolve_query_bbox(args)?;
        let filter_preset = args
            .get("filter_preset")
            .and_then(|v| v.as_str())
            .unwrap_or("all")
            .to_lowercase();
        if !is_valid_preset(&filter_preset) {
            return Err(ToolError::Validation(format!(
                "invalid filter_preset '{}'; expected one of: {}",
                filter_preset,
                OSM_FILTER_PRESETS.join(", ")
            )));
        }
        let chunk_large_aoi = args
            .get("chunk_large_aoi")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if chunk_large_aoi {
            let chunk_max_area_deg2 = args
                .get("chunk_max_area_deg2")
                .and_then(|v| v.as_f64())
                .unwrap_or(4.0);
            let max_chunk_count = args
                .get("max_chunk_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(64) as usize;
            let chunk_parallel_requests = args
                .get("chunk_parallel_requests")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;
            if max_chunk_count == 0 {
                return Err(ToolError::Validation("max_chunk_count must be > 0".to_string()));
            }
            if chunk_parallel_requests == 0 {
                return Err(ToolError::Validation(
                    "chunk_parallel_requests must be > 0".to_string(),
                ));
            }
            let _ = plan_chunked_bboxes(
                west,
                south,
                east,
                north,
                chunk_max_area_deg2,
                max_chunk_count,
            )?;
        }
        let _ = resolve_overpass_endpoint(args)?;
        let output = parse_vector_output_path(args)?;
        let split_output = args
            .get("split_output_by_geometry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if split_output {
            let _ = derive_split_output_paths(output.trim())?;
        }
        let _ = detect_vector_output_format(output.trim())?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (west, south, east, north) = resolve_query_bbox(args)?;
        let input_extent_epsg = args
            .get("input_extent_epsg")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(4326);

        let filter_preset = args
            .get("filter_preset")
            .and_then(|v| v.as_str())
            .unwrap_or("all")
            .to_lowercase();

        let include_points   = args.get("include_points").and_then(|v| v.as_bool()).unwrap_or(true);
        let include_lines    = args.get("include_lines").and_then(|v| v.as_bool()).unwrap_or(true);
        let include_polygons = args.get("include_polygons").and_then(|v| v.as_bool()).unwrap_or(true);
        let clip_to_extent   = args.get("clip_to_extent").and_then(|v| v.as_bool()).unwrap_or(true);
        let split_output_by_geometry = args
            .get("split_output_by_geometry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let output_epsg      = args.get("output_epsg").and_then(|v| v.as_u64()).map(|v| v as u32);
        let cache_dir = args
            .get("cache_dir")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let cache_ttl_hours = args
            .get("cache_ttl_hours")
            .and_then(|v| v.as_u64())
            .unwrap_or(24);
        let provenance_output = args
            .get("provenance_output")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let endpoint = resolve_overpass_endpoint(args)?;

        let timeout_secs = args
            .get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .max(5);

        let max_elements = args
            .get("max_elements")
            .and_then(|v| v.as_u64())
            .unwrap_or(50_000);
        let chunk_large_aoi = args
            .get("chunk_large_aoi")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let chunk_max_area_deg2 = args
            .get("chunk_max_area_deg2")
            .and_then(|v| v.as_f64())
            .unwrap_or(4.0);
        let max_chunk_count = args
            .get("max_chunk_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(64) as usize;
        let chunk_parallel_requests = args
            .get("chunk_parallel_requests")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let output_path = parse_vector_output_path(args)?;

        let filters = build_filter_clauses(args, &filter_preset);

        ctx.progress.info(&format!(
            "querying Overpass API at {} for bbox ({:.4},{:.4},{:.4},{:.4})",
            endpoint, west, south, east, north
        ));

        let chunk_bboxes = if chunk_large_aoi {
            plan_chunked_bboxes(
                west,
                south,
                east,
                north,
                chunk_max_area_deg2,
                max_chunk_count,
            )?
        } else {
            vec![(west, south, east, north)]
        };

        if chunk_bboxes.len() > 1 {
            ctx.progress.info(&format!(
                "large AOI detected; splitting request into {} chunk(s)",
                chunk_bboxes.len()
            ));
        }

        let mut merged_layer: Option<wbvector::Layer> = None;
        let mut total_skipped = 0usize;
        let mut downloaded_feature_count = 0usize;
        let mut any_cache_used = false;

        let parallel_chunks = chunk_bboxes.len() > 1 && chunk_parallel_requests > 1;
        if parallel_chunks {
            let workers = chunk_parallel_requests.min(chunk_bboxes.len());
            ctx.progress.info(&format!(
                "fetching chunks in parallel with {} worker(s)",
                workers
            ));
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .map_err(|e| ToolError::Execution(format!("failed creating chunk thread pool: {}", e)))?;
            let mut chunk_results = pool.install(|| {
                chunk_bboxes
                    .par_iter()
                    .enumerate()
                    .map(|(idx, bbox)| {
                        fetch_parse_chunk(
                            &endpoint,
                            *bbox,
                            include_points,
                            include_lines,
                            include_polygons,
                            &filters,
                            timeout_secs,
                            max_elements,
                            cache_dir.as_deref(),
                            cache_ttl_hours,
                        )
                        .map(|(parsed, used_cache)| (idx, parsed, used_cache))
                    })
                    .collect::<Result<Vec<_>, ToolError>>()
            })?;
            chunk_results.sort_by_key(|(idx, _, _)| *idx);

            for (_, parsed, used_cache_chunk) in chunk_results {
                if used_cache_chunk {
                    any_cache_used = true;
                }
                total_skipped += parsed.skipped;
                downloaded_feature_count += parsed.layer.len();

                if let Some(layer) = merged_layer.as_mut() {
                    layer.features.extend(parsed.layer.features.into_iter());
                } else {
                    merged_layer = Some(parsed.layer);
                }
            }
        } else {
            for (idx, bbox) in chunk_bboxes.iter().enumerate() {
                if chunk_bboxes.len() > 1 {
                    ctx.progress.info(&format!(
                        "fetching chunk {}/{}",
                        idx + 1,
                        chunk_bboxes.len()
                    ));
                }

                let (parsed, used_cache_chunk) = fetch_parse_chunk(
                    &endpoint,
                    *bbox,
                    include_points,
                    include_lines,
                    include_polygons,
                    &filters,
                    timeout_secs,
                    max_elements,
                    cache_dir.as_deref(),
                    cache_ttl_hours,
                )?;
                if used_cache_chunk {
                    any_cache_used = true;
                }

                total_skipped += parsed.skipped;
                downloaded_feature_count += parsed.layer.len();

                if let Some(layer) = merged_layer.as_mut() {
                    layer.features.extend(parsed.layer.features.into_iter());
                } else {
                    merged_layer = Some(parsed.layer);
                }
            }
        }

        let mut merged_layer = merged_layer.unwrap_or_else(|| {
            let mut l = wbvector::Layer::new("osm_download");
            l.assign_crs_epsg(4326);
            l
        });

        if chunk_bboxes.len() > 1 {
            dedupe_layer_by_osm_identity(&mut merged_layer);
        }

        if total_skipped > 0 {
            ctx.progress.info(&format!(
                "{} way/relation member(s) skipped due to missing node coordinates",
                total_skipped
            ));
        }

        ctx.progress.info(&format!(
            "downloaded {} feature(s) from OSM",
            downloaded_feature_count
        ));

        let mut output_layer = if clip_to_extent {
            clip_layer_to_bbox(&merged_layer, west, south, east, north)
        } else {
            merged_layer
        };

        if let Some(epsg) = output_epsg {
            output_layer = output_layer.reproject_to_epsg(epsg).map_err(|e| {
                ToolError::Execution(format!(
                    "reprojection to EPSG:{} failed: {}",
                    epsg, e
                ))
            })?;
        }

        let mut outputs = std::collections::BTreeMap::new();

        if split_output_by_geometry {
            let (points_path, lines_path, polygons_path) = derive_split_output_paths(output_path.trim())?;

            let (mut points_layer, mut lines_layer, mut polygons_layer) = split_layer_by_geometry(&output_layer);

            // Explicitly set geometry types for each output
            points_layer.geom_type = Some(wbvector::GeometryType::Point);
            lines_layer.geom_type = Some(wbvector::GeometryType::LineString);
            polygons_layer.geom_type = Some(wbvector::GeometryType::Polygon);

            if !points_layer.features.is_empty() {
                let loc = write_vector_output(&points_layer, &points_path)?;
                outputs.insert("points_path".to_string(), json!(loc));
            }
            if !lines_layer.features.is_empty() {
                let loc = write_vector_output(&lines_layer, &lines_path)?;
                outputs.insert("lines_path".to_string(), json!(loc));
            }
            if !polygons_layer.features.is_empty() {
                let loc = write_vector_output(&polygons_layer, &polygons_path)?;
                outputs.insert("polygons_path".to_string(), json!(loc));
            }

            if outputs.is_empty() {
                return Err(ToolError::Execution(
                    "no geometries were available to write for split_output_by_geometry".to_string(),
                ));
            }
        } else {
            let locator = write_vector_output(&output_layer, output_path.trim())?;
            outputs.insert("path".to_string(), json!(locator));
        }

        if let Some(prov_path) = provenance_output {
            let provenance = json!({
                "source": "openstreetmap_overpass",
                "endpoint": endpoint,
                "bbox": {
                    "west": west,
                    "south": south,
                    "east": east,
                    "north": north
                },
                "input_extent_epsg": input_extent_epsg,
                "filter_preset": filter_preset,
                "filters": filters,
                "include_points": include_points,
                "include_lines": include_lines,
                "include_polygons": include_polygons,
                "clip_to_extent": clip_to_extent,
                "output_epsg": output_epsg,
                "split_output_by_geometry": split_output_by_geometry,
                "cache_used": any_cache_used,
                "cache_dir": cache_dir,
                "cache_ttl_hours": cache_ttl_hours,
                "chunk_large_aoi": chunk_large_aoi,
                "chunk_max_area_deg2": chunk_max_area_deg2,
                "max_chunk_count": max_chunk_count,
                "chunk_parallel_requests": chunk_parallel_requests,
                "chunk_count": chunk_bboxes.len(),
                "downloaded_feature_count": downloaded_feature_count,
                "output_feature_count": output_layer.len(),
                "skipped_way_count": total_skipped,
            });
            let wrote = write_json_sidecar(&prov_path, &provenance)?;
            outputs.insert("provenance_path".to_string(), json!(wrote));
        }

        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_semicolon_list_splits_and_trims() {
        let values = parse_semicolon_list(Some(" amenity ; shop ;; building "));
        assert_eq!(values, vec!["amenity", "shop", "building"]);
    }

    #[test]
    fn build_filter_clauses_uses_design_args() {
        let mut args = ToolArgs::new();
        args.insert("include_tags".to_string(), json!("amenity;shop"));
        args.insert("include_key_values".to_string(), json!("building=yes;amenity=school"));

        let filters = build_filter_clauses(&args, "all");
        assert!(filters.contains(&"[amenity]".to_string()));
        assert!(filters.contains(&"[shop]".to_string()));
        assert!(filters.contains(&"[building=yes]".to_string()));
        assert!(filters.contains(&"[amenity=school]".to_string()));
    }

    #[test]
    fn build_filter_clauses_falls_back_to_preset() {
        let args = ToolArgs::new();
        let filters = build_filter_clauses(&args, "roads");
        assert_eq!(filters, vec!["[highway]".to_string()]);
    }

    #[test]
    fn build_filter_clauses_supports_trails_preset() {
        let args = ToolArgs::new();
        let filters = build_filter_clauses(&args, "trails");
        assert!(filters.contains(&"[highway=path]".to_string()));
        assert!(filters.contains(&"[highway=footway]".to_string()));
        assert!(filters.contains(&"[highway=cycleway]".to_string()));
    }

    #[test]
    fn build_filter_clauses_supports_parks_preset() {
        let args = ToolArgs::new();
        let filters = build_filter_clauses(&args, "parks");
        assert!(filters.contains(&"[leisure=park]".to_string()));
        assert!(filters.contains(&"[boundary=national_park]".to_string()));
    }

    #[test]
    fn build_overpass_query_includes_expected_clauses() {
        let filters = vec!["[highway]".to_string()];
        let q = build_overpass_query(-80.5, 43.4, -80.4, 43.5, true, true, true, &filters, 25);
        assert!(q.contains("[out:json][timeout:25]"));
        assert!(q.contains("node[highway](43.4000000,-80.5000000,43.5000000,-80.4000000);"));
        assert!(q.contains("way[highway](43.4000000,-80.5000000,43.5000000,-80.4000000);"));
        assert!(q.contains("relation[highway](43.4000000,-80.5000000,43.5000000,-80.4000000);"));
    }

    #[test]
    fn clip_layer_to_bbox_clips_linestring() {
        let mut layer = wbvector::Layer::new("test");
        let line = wbvector::Geometry::line_string(vec![
            wbvector::Coord::xy(-1.0, 0.5),
            wbvector::Coord::xy(2.0, 0.5),
        ]);
        layer.add_feature(Some(line), &[]).unwrap();

        let out = clip_layer_to_bbox(&layer, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(out.features.len(), 1);
        let g = out.features[0].geometry.as_ref().unwrap();
        match g {
            wbvector::Geometry::LineString(coords) => {
                assert_eq!(coords.len(), 2);
                assert!((coords[0].x - 0.0).abs() < 1.0e-9);
                assert!((coords[1].x - 1.0).abs() < 1.0e-9);
            }
            _ => panic!("expected linestring geometry"),
        }
    }

    #[test]
    fn clip_layer_to_bbox_clips_polygon() {
        let mut layer = wbvector::Layer::new("test_poly");
        let poly = wbvector::Geometry::polygon(
            vec![
                wbvector::Coord::xy(-1.0, -1.0),
                wbvector::Coord::xy(2.0, -1.0),
                wbvector::Coord::xy(2.0, 2.0),
                wbvector::Coord::xy(-1.0, 2.0),
                wbvector::Coord::xy(-1.0, -1.0),
            ],
            vec![],
        );
        layer.add_feature(Some(poly), &[]).unwrap();

        let out = clip_layer_to_bbox(&layer, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(out.features.len(), 1);
        let g = out.features[0].geometry.as_ref().unwrap();
        match g {
            wbvector::Geometry::Polygon { exterior, interiors: _ } => {
                assert!(exterior.0.len() >= 4);
                for c in &exterior.0 {
                    assert!(c.x >= -1.0e-9 && c.x <= 1.0 + 1.0e-9);
                    assert!(c.y >= -1.0e-9 && c.y <= 1.0 + 1.0e-9);
                }
            }
            _ => panic!("expected polygon geometry"),
        }
    }

    #[test]
    fn parse_overpass_response_builds_features_from_mock_json() {
        let mock = json!({
            "elements": [
                {
                    "type": "node",
                    "id": 1,
                    "lat": 43.45,
                    "lon": -80.49,
                    "tags": {"amenity": "school", "name": "Test School"}
                },
                {"type": "node", "id": 10, "lat": 43.40, "lon": -80.50},
                {"type": "node", "id": 11, "lat": 43.40, "lon": -80.48},
                {"type": "node", "id": 12, "lat": 43.42, "lon": -80.48},
                {"type": "node", "id": 13, "lat": 43.42, "lon": -80.50},
                {"type": "node", "id": 10, "lat": 43.40, "lon": -80.50},
                {
                    "type": "way",
                    "id": 100,
                    "nodes": [10, 11, 12, 13, 10],
                    "tags": {"building": "yes", "name": "Mock Building"}
                }
            ]
        });

        let parsed = parse_overpass_response(&mock, true, true, true, 10_000)
            .expect("mock response should parse");
        assert_eq!(parsed.skipped, 0);
        assert!(parsed.layer.features.len() >= 2);

        let has_point = parsed.layer.features.iter().any(|f| {
            matches!(f.geometry.as_ref(), Some(wbvector::Geometry::Point(_)))
        });
        let has_polygon = parsed.layer.features.iter().any(|f| {
            matches!(f.geometry.as_ref(), Some(wbvector::Geometry::Polygon { .. }))
        });
        assert!(has_point);
        assert!(has_polygon);
    }

    #[test]
    fn parse_overpass_response_builds_polygon_from_relation_member_ways() {
        let mock = json!({
            "elements": [
                {"type": "node", "id": 1, "lat": 43.0, "lon": -80.0},
                {"type": "node", "id": 2, "lat": 43.0, "lon": -79.9},
                {"type": "node", "id": 3, "lat": 43.1, "lon": -79.9},
                {"type": "node", "id": 4, "lat": 43.1, "lon": -80.0},
                {
                    "type": "way",
                    "id": 101,
                    "nodes": [1, 2, 3, 4, 1],
                    "tags": {"highway": "path"}
                },
                {
                    "type": "relation",
                    "id": 201,
                    "members": [
                        {"type": "way", "ref": 101, "role": "outer"}
                    ],
                    "tags": {"type": "multipolygon", "leisure": "park", "name": "Mock Park"}
                }
            ]
        });

        let parsed = parse_overpass_response(&mock, false, false, true, 10_000)
            .expect("relation response should parse");

        let polygon_count = parsed
            .layer
            .features
            .iter()
            .filter(|f| matches!(f.geometry.as_ref(), Some(wbvector::Geometry::Polygon { .. })))
            .count();

        assert!(polygon_count >= 1);
    }

    #[test]
    fn resolve_overpass_endpoint_uses_profile_default() {
        let mut args = ToolArgs::new();
        args.insert("overpass_profile".to_string(), json!("kumi"));
        let endpoint = resolve_overpass_endpoint(&args).expect("profile should resolve");
        assert_eq!(endpoint, "https://overpass.kumi.systems/api/interpreter");
    }

    #[test]
    fn resolve_overpass_endpoint_explicit_url_overrides_profile() {
        let mut args = ToolArgs::new();
        args.insert("overpass_profile".to_string(), json!("main"));
        args.insert("overpass_url".to_string(), json!("https://example.test/interpreter"));
        let endpoint = resolve_overpass_endpoint(&args).expect("explicit URL should override profile");
        assert_eq!(endpoint, "https://example.test/interpreter");
    }

    #[test]
    fn resolve_query_bbox_transforms_projected_extent() {
        let mut args = ToolArgs::new();
        args.insert("west".to_string(), json!(-222638.9816));
        args.insert("south".to_string(), json!(-111325.1429));
        args.insert("east".to_string(), json!(222638.9816));
        args.insert("north".to_string(), json!(111325.1429));
        args.insert("input_extent_epsg".to_string(), json!(3857));

        let (w, s, e, n) = resolve_query_bbox(&args).expect("projected bbox should transform");
        assert!(w < -1.0 && e > 1.0);
        assert!(s < -0.9 && n > 0.9);
    }

    #[test]
    fn derive_split_output_paths_appends_geometry_suffixes() {
        let (p, l, g) = derive_split_output_paths("out/osm.geojson").expect("paths should derive");
        assert!(p.ends_with("osm_points.geojson"));
        assert!(l.ends_with("osm_lines.geojson"));
        assert!(g.ends_with("osm_polygons.geojson"));
    }

    #[test]
    fn derive_split_output_paths_requires_extension() {
        let err = derive_split_output_paths("out/osm_no_ext").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("split_output_by_geometry requires an output filename extension"));
    }

    #[test]
    fn plan_chunked_bboxes_splits_large_extent() {
        let tiles = plan_chunked_bboxes(-80.5, 43.4, -79.5, 44.4, 0.2, 64)
            .expect("chunk plan should succeed");
        assert!(tiles.len() > 1);
        assert!(tiles.len() <= 64);
    }

    #[test]
    fn dedupe_layer_by_osm_identity_removes_duplicates() {
        let mut layer = wbvector::Layer::new("osm_download");
        layer.assign_crs_epsg(4326);
        layer.add_field(wbvector::FieldDef::new("osm_id", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("osm_type", wbvector::FieldType::Text));

        let geom = wbvector::Geometry::point(-80.0, 43.0);
        layer
            .add_feature(
                Some(geom.clone()),
                &[
                    ("osm_id", wbvector::FieldValue::Integer(1)),
                    ("osm_type", wbvector::FieldValue::Text("node".to_string())),
                ],
            )
            .unwrap();
        layer
            .add_feature(
                Some(geom),
                &[
                    ("osm_id", wbvector::FieldValue::Integer(1)),
                    ("osm_type", wbvector::FieldValue::Text("node".to_string())),
                ],
            )
            .unwrap();

        dedupe_layer_by_osm_identity(&mut layer);
        assert_eq!(layer.features.len(), 1);
    }
}
