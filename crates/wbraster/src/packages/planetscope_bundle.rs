//! PlanetScope scene bundle reader.
//!
//! PlanetScope deliveries commonly include analytic GeoTIFF assets, UDM/UDM2
//! masks, and JSON/XML metadata sidecars.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed PlanetScope scene bundle.
#[derive(Debug, Clone)]
pub struct PlanetScopeBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Metadata JSON path when present.
    pub metadata_json_path: Option<PathBuf>,
    /// Metadata XML path when present.
    pub metadata_xml_path: Option<PathBuf>,
    /// Scene identifier when present.
    pub scene_id: Option<String>,
    /// Acquisition datetime UTC when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Product type when present.
    pub product_type: Option<String>,
    /// Cloud cover percentage when present.
    pub cloud_cover_percent: Option<f64>,
    /// Mean sun azimuth in degrees when present.
    pub sun_azimuth_deg: Option<f64>,
    /// Mean sun elevation in degrees when present.
    pub sun_elevation_deg: Option<f64>,
    /// View angle in degrees when present.
    pub view_angle_deg: Option<f64>,
    /// Off-nadir angle in degrees when present.
    pub off_nadir_angle_deg: Option<f64>,
    /// Canonical band key -> path.
    pub bands: BTreeMap<String, PathBuf>,
    /// Profile key -> (canonical band key -> path).
    pub profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>>,
    /// Canonical QA key -> path.
    pub qa_layers: BTreeMap<String, PathBuf>,
}

impl PlanetScopeBundle {
    /// Open and parse a PlanetScope scene directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "PlanetScope bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;
        files.sort();

        let mut metadata_json_path = None;
        let mut metadata_xml_path = None;
        let mut bands = BTreeMap::new();
        let mut profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>> = BTreeMap::new();
        let mut qa_layers = BTreeMap::new();

        for p in files {
            if has_json_ext(&p) && metadata_json_path.is_none() {
                metadata_json_path = Some(p.clone());
            }
            if has_xml_ext(&p) && metadata_xml_path.is_none() {
                metadata_xml_path = Some(p.clone());
            }
            if !has_tiff_ext(&p) {
                continue;
            }

            if let Some(q) = canonical_qa_key(&p) {
                qa_layers.insert(q, p);
                continue;
            }
            if let Some(k) = canonical_band_key(&p) {
                let profile = detect_planetscope_profile(&p);
                profile_bands
                    .entry(profile)
                    .or_default()
                    .insert(k.clone(), p.clone());
                bands.insert(k, p);
            }
        }

        if bands.is_empty() && qa_layers.is_empty() {
            return Err(RasterError::MissingField(
                "no PlanetScope TIFF assets found in bundle".to_string(),
            ));
        }

        let mut scene_id = None;
        let mut acquisition_datetime_utc = None;
        let mut product_type = None;
        let mut cloud_cover_percent = None;
        let mut sun_azimuth_deg = None;
        let mut sun_elevation_deg = None;
        let mut view_angle_deg = None;
        let mut off_nadir_angle_deg = None;

        if let Some(json_path) = metadata_json_path.as_ref() {
            if let Ok(text) = fs::read_to_string(json_path) {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    scene_id = extract_first_json_text(
                        &v,
                        &["id", "scene_id", "properties.id", "properties.item_id"],
                    );
                    acquisition_datetime_utc = extract_first_json_text(
                        &v,
                        &["datetime", "properties.datetime", "acquired", "properties.acquired"],
                    );
                    product_type = extract_first_json_text(
                        &v,
                        &["properties.item_type", "item_type", "product_type", "properties.product_type"],
                    );
                    cloud_cover_percent = extract_first_json_number(
                        &v,
                        &["eo:cloud_cover", "cloud_cover", "properties.cloud_cover"],
                    );
                    sun_azimuth_deg = extract_first_json_number(
                        &v,
                        &["view:sun_azimuth", "sun_azimuth", "properties.sun_azimuth"],
                    );
                    sun_elevation_deg = extract_first_json_number(
                        &v,
                        &["view:sun_elevation", "sun_elevation", "properties.sun_elevation"],
                    );
                    view_angle_deg = extract_first_json_number(
                        &v,
                        &["view:incidence_angle", "view_angle", "properties.view_angle"],
                    );
                    off_nadir_angle_deg = extract_first_json_number(
                        &v,
                        &["view:off_nadir", "off_nadir", "properties.off_nadir"],
                    );
                }
            }
        }

        if let Some(xml_path) = metadata_xml_path.as_ref() {
            if let Ok(xml) = fs::read_to_string(xml_path) {
                if scene_id.is_none() {
                    scene_id = extract_first_xml_text(&xml, &["scene_id", "id"]);
                }
                if acquisition_datetime_utc.is_none() {
                    acquisition_datetime_utc = extract_first_xml_text(
                        &xml,
                        &["acquired", "acquisition_time", "acquisition_datetime"],
                    );
                }
                if product_type.is_none() {
                    product_type = extract_first_xml_text(&xml, &["item_type", "product_type"]);
                }
                if cloud_cover_percent.is_none() {
                    cloud_cover_percent = extract_first_xml_number(
                        &xml,
                        &["cloud_cover", "cloudcover", "eo:cloud_cover"],
                    );
                }
                if sun_azimuth_deg.is_none() {
                    sun_azimuth_deg = extract_first_xml_number(
                        &xml,
                        &["sun_azimuth", "sun_azimuth_angle"],
                    );
                }
                if sun_elevation_deg.is_none() {
                    sun_elevation_deg = extract_first_xml_number(
                        &xml,
                        &["sun_elevation", "sun_elevation_angle"],
                    );
                }
                if view_angle_deg.is_none() {
                    view_angle_deg = extract_first_xml_number(&xml, &["view_angle", "incidence_angle"]);
                }
                if off_nadir_angle_deg.is_none() {
                    off_nadir_angle_deg = extract_first_xml_number(&xml, &["off_nadir", "off_nadir_angle"]);
                }
            }
        }

        Ok(Self {
            bundle_root,
            metadata_json_path,
            metadata_xml_path,
            scene_id,
            acquisition_datetime_utc,
            product_type,
            cloud_cover_percent,
            sun_azimuth_deg,
            sun_elevation_deg,
            view_angle_deg,
            off_nadir_angle_deg,
            bands,
            profile_bands,
            qa_layers,
        })
    }

    /// List canonical band keys.
    pub fn list_band_keys(&self) -> Vec<String> {
        self.bands.keys().cloned().collect()
    }

    /// List canonical QA keys.
    pub fn list_qa_keys(&self) -> Vec<String> {
        self.qa_layers.keys().cloned().collect()
    }

    /// List profile keys (e.g. `ANALYTIC`, `ANALYTIC_SR`, `VISUAL`, `PAN`).
    pub fn list_profiles(&self) -> Vec<String> {
        self.profile_bands.keys().cloned().collect()
    }

    /// Resolve the preferred default profile key when available.
    pub fn default_profile(&self) -> Option<&str> {
        for p in ["ANALYTIC_SR", "ANALYTIC", "VISUAL", "PAN"] {
            if self.profile_bands.contains_key(p) {
                return Some(p);
            }
        }
        self.profile_bands.keys().next().map(String::as_str)
    }

    /// List canonical band keys for a given profile key.
    pub fn list_band_keys_for_profile(&self, profile: &str) -> Vec<String> {
        self.profile_bands
            .get(&profile.to_ascii_uppercase())
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Resolve canonical band path.
    pub fn band_path(&self, key: &str) -> Option<&Path> {
        self.bands.get(&key.to_ascii_uppercase()).map(PathBuf::as_path)
    }

    /// Resolve canonical band path for a specific profile key.
    pub fn band_path_for_profile(&self, profile: &str, key: &str) -> Option<&Path> {
        self.profile_bands
            .get(&profile.to_ascii_uppercase())
            .and_then(|m| m.get(&key.to_ascii_uppercase()))
            .map(PathBuf::as_path)
    }

    /// Resolve canonical QA path.
    pub fn qa_path(&self, key: &str) -> Option<&Path> {
        self.qa_layers
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read canonical band raster.
    pub fn read_band(&self, key: &str) -> Result<Raster> {
        let p = self.band_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("PlanetScope band '{}' not found", key))
        })?;
        Raster::read(p)
    }

    /// Read canonical band raster for a specific profile key.
    pub fn read_band_for_profile(&self, profile: &str, key: &str) -> Result<Raster> {
        let p = self.band_path_for_profile(profile, key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "PlanetScope band '{}' not found in profile '{}'",
                key, profile
            ))
        })?;
        Raster::read(p)
    }

    /// Read canonical QA raster.
    pub fn read_qa_layer(&self, key: &str) -> Result<Raster> {
        let p = self.qa_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("PlanetScope QA layer '{}' not found", key))
        })?;
        Raster::read(p)
    }
}

fn collect_files_recursive(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_files_recursive(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

fn has_tiff_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let ext = e.to_string_lossy();
            ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
        })
        .unwrap_or(false)
}

fn has_json_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("json"))
        .unwrap_or(false)
}

fn has_xml_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
}

fn canonical_qa_key(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    if stem.contains("UDM2") {
        return Some("UDM2".to_string());
    }
    if stem.contains("UDM") {
        return Some("UDM".to_string());
    }
    None
}

fn canonical_band_key(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    let tokens = split_tokens(&stem);
    if has_any_token(&tokens, &["B1", "BAND1", "COASTAL"]) {
        return Some("B1".to_string());
    }
    if has_any_token(&tokens, &["B2", "BAND2", "BLUE"]) {
        return Some("B2".to_string());
    }
    if has_any_token(&tokens, &["B3", "BAND3", "GREEN"]) {
        return Some("B3".to_string());
    }
    if has_any_token(&tokens, &["B4", "BAND4", "RED"]) {
        return Some("B4".to_string());
    }
    if has_any_token(&tokens, &["B5", "BAND5", "NIR", "NIR1"]) {
        return Some("B5".to_string());
    }
    if has_any_token(&tokens, &["B6", "BAND6", "RE", "REDEDGE"]) {
        return Some("B6".to_string());
    }
    if has_any_token(&tokens, &["B7", "BAND7", "YELLOW"]) {
        return Some("B7".to_string());
    }
    if has_any_token(&tokens, &["B8", "BAND8", "NIR2"]) {
        return Some("B8".to_string());
    }
    if stem.contains("ANALYTIC") {
        return Some("ANALYTIC".to_string());
    }
    None
}

fn detect_planetscope_profile(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    if stem.contains("ANALYTIC_SR") || stem.contains("SURFACE_REFLECTANCE") {
        return "ANALYTIC_SR".to_string();
    }
    if stem.contains("ANALYTIC") {
        return "ANALYTIC".to_string();
    }
    if stem.contains("VISUAL") {
        return "VISUAL".to_string();
    }
    if stem.contains("PAN") || stem.contains("PANCHROMATIC") {
        return "PAN".to_string();
    }
    "UNKNOWN".to_string()
}

fn split_tokens(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect()
}

fn has_any_token(tokens: &[&str], candidates: &[&str]) -> bool {
    tokens
        .iter()
        .any(|t| candidates.iter().any(|c| t.eq_ignore_ascii_case(c)))
}

fn extract_first_json_text(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = find_json_value_by_path(value, key) {
            if let Some(s) = v.as_str() {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

fn extract_first_json_number(value: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(v) = find_json_value_by_path(value, key) {
            if let Some(n) = v.as_f64() {
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.trim().parse::<f64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_first_xml_text(xml: &str, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(v) = extract_xml_tag(xml, tag) {
            return Some(v);
        }
    }
    None
}

fn extract_first_xml_number(xml: &str, tags: &[&str]) -> Option<f64> {
    for tag in tags {
        if let Some(v) = extract_xml_tag(xml, tag) {
            if let Ok(n) = v.trim().parse::<f64>() {
                return Some(n);
            }
        }
    }
    None
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let tail = &xml[start..];
    let end_rel = tail.find(&close)?;
    let value = tail[..end_rel].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn find_json_value_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = value;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_planetscope_bundle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("PLANET");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("metadata.json"),
            r#"{"id":"PSScene_01","properties":{"datetime":"2026-04-01T10:00:00Z","item_type":"PSScene"}}"#,
        )
        .expect("json");
        fs::write(root.join("scene_analytic_b3.tif"), b"").expect("b3");
        fs::write(root.join("scene_analytic_b4.tif"), b"").expect("b4");
        fs::write(root.join("scene_udm2.tif"), b"").expect("udm2");

        let b = PlanetScopeBundle::open(&root).expect("open");
        assert_eq!(b.scene_id.as_deref(), Some("PSScene_01"));
        assert_eq!(b.acquisition_datetime_utc.as_deref(), Some("2026-04-01T10:00:00Z"));
        assert_eq!(b.product_type.as_deref(), Some("PSScene"));
        assert!(b.band_path("B3").is_some());
        assert!(b.band_path("B4").is_some());
        assert!(!b.list_profiles().is_empty());
        assert!(b.qa_path("UDM2").is_some());
    }

    #[test]
    fn opens_real_planetscope_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_PLANETSCOPE_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let b = PlanetScopeBundle::open(&root).expect("open real planetscope sample");
        assert!(!b.list_band_keys().is_empty() || !b.list_qa_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_PLANETSCOPE_SAMPLE_EXPECT_PROFILES",
            b.list_profiles(),
            "PlanetScope profile",
        );
    }

    #[test]
    fn maps_superdove_style_band_tokens() {
        assert_eq!(
            canonical_band_key(Path::new("scene_analytic_band1.tif")).as_deref(),
            Some("B1")
        );
        assert_eq!(
            canonical_band_key(Path::new("scene_analytic_band6.tif")).as_deref(),
            Some("B6")
        );
        assert_eq!(
            canonical_band_key(Path::new("scene_analytic_band8.tif")).as_deref(),
            Some("B8")
        );
        assert_eq!(
            canonical_band_key(Path::new("scene_analytic_nir2.tif")).as_deref(),
            Some("B8")
        );
    }

    #[test]
    fn parses_extended_planetscope_metadata_from_json() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("PLANET_EXT");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("metadata.json"),
            r#"{"id":"PSScene_02","properties":{"datetime":"2026-04-01T11:00:00Z","item_type":"PSScene","cloud_cover":12.5,"sun_azimuth":135.2,"sun_elevation":49.1,"view_angle":3.2,"off_nadir":2.1}}"#,
        )
        .expect("json");
        fs::write(root.join("scene_analytic_b4.tif"), b"").expect("b4");

        let b = PlanetScopeBundle::open(&root).expect("open");
        assert_eq!(b.cloud_cover_percent, Some(12.5));
        assert_eq!(b.sun_azimuth_deg, Some(135.2));
        assert_eq!(b.sun_elevation_deg, Some(49.1));
        assert_eq!(b.view_angle_deg, Some(3.2));
        assert_eq!(b.off_nadir_angle_deg, Some(2.1));
    }

    #[test]
    fn groups_assets_by_planetscope_profile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("PLANET_GROUPS");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(root.join("a_analytic_b4.tif"), b"").expect("analytic");
        fs::write(root.join("b_analytic_sr_b4.tif"), b"").expect("analytic sr");

        let b = PlanetScopeBundle::open(&root).expect("open");
        assert!(b.list_profiles().iter().any(|p| p == "ANALYTIC"));
        assert!(b.list_profiles().iter().any(|p| p == "ANALYTIC_SR"));
        assert!(b.band_path_for_profile("ANALYTIC", "B4").is_some());
        assert!(b.band_path_for_profile("ANALYTIC_SR", "B4").is_some());
    }
}
