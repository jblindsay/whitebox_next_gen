//! SPOT/Pleiades DIMAP bundle reader.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed DIMAP scene bundle.
#[derive(Debug, Clone)]
pub struct DimapBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// DIMAP XML metadata path.
    pub metadata_xml_path: PathBuf,
    /// Mission/platform when detected.
    pub mission: Option<String>,
    /// Scene identifier when present.
    pub scene_id: Option<String>,
    /// Acquisition datetime UTC when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Processing/product level when present.
    pub processing_level: Option<String>,
    /// Cloud cover percentage when present.
    pub cloud_cover_percent: Option<f64>,
    /// Mean sun azimuth in degrees when present.
    pub sun_azimuth_deg: Option<f64>,
    /// Mean sun elevation in degrees when present.
    pub sun_elevation_deg: Option<f64>,
    /// Canonical band key -> path.
    pub bands: BTreeMap<String, PathBuf>,
    /// Profile key -> (canonical band key -> path).
    pub profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>>,
}

impl DimapBundle {
    /// Open and parse a DIMAP scene directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "DIMAP bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;
        files.sort();

        let metadata_xml_path = files
            .iter()
            .find(|p| is_dimap_xml(p))
            .cloned()
            .ok_or_else(|| RasterError::MissingField("DIMAP metadata XML not found".to_string()))?;

        let metadata_text = fs::read_to_string(&metadata_xml_path).unwrap_or_default();
        let mission = extract_tag_value(&metadata_text, "MISSION")
            .or_else(|| extract_tag_value(&metadata_text, "MISSION_INDEX"));
        let scene_id = extract_tag_value(&metadata_text, "DATASET_NAME")
            .or_else(|| extract_tag_value(&metadata_text, "SCENE_ID"));
        let acquisition_datetime_utc = combine_date_time(
            extract_first_tag_value(&metadata_text, &["IMAGING_DATE", "ACQUISITION_DATE"]),
            extract_first_tag_value(&metadata_text, &["IMAGING_TIME", "ACQUISITION_TIME"]),
        )
        .or_else(|| extract_first_tag_value(&metadata_text, &["ACQUISITION_DATETIME", "IMAGING_DATETIME"]));
        let processing_level = extract_first_tag_value(
            &metadata_text,
            &["PROCESSING_LEVEL", "PRODUCTION_LEVEL", "PRODUCT_LEVEL"],
        );
        let cloud_cover_percent = extract_first_tag_number(
            &metadata_text,
            &["CLOUD_COVER", "CLOUD_COVERAGE", "CLOUDCOVER"],
        );
        let sun_azimuth_deg = extract_first_tag_number(
            &metadata_text,
            &["SUN_AZIMUTH", "SUN_AZIMUTH_ANGLE"],
        );
        let sun_elevation_deg = extract_first_tag_number(
            &metadata_text,
            &["SUN_ELEVATION", "SUN_ELEVATION_ANGLE"],
        );

        let mut bands = BTreeMap::new();
        let mut profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>> = BTreeMap::new();
        for p in files {
            if !has_raster_ext(&p) {
                continue;
            }
            if let Some(k) = canonical_band_key(&p) {
                let profile = detect_dimap_profile(&p);
                profile_bands
                    .entry(profile)
                    .or_default()
                    .insert(k.clone(), p.clone());
                bands.insert(k, p);
            }
        }

        if bands.is_empty() {
            return Err(RasterError::MissingField(
                "no DIMAP raster assets found in bundle".to_string(),
            ));
        }

        Ok(Self {
            bundle_root,
            metadata_xml_path,
            mission,
            scene_id,
            acquisition_datetime_utc,
            processing_level,
            cloud_cover_percent,
            sun_azimuth_deg,
            sun_elevation_deg,
            bands,
            profile_bands,
        })
    }

    /// List canonical band keys.
    pub fn list_band_keys(&self) -> Vec<String> {
        self.bands.keys().cloned().collect()
    }

    /// List profile keys (e.g. `MS`, `PAN`, `PMS`, `PSH`, `SWIR`).
    pub fn list_profiles(&self) -> Vec<String> {
        self.profile_bands.keys().cloned().collect()
    }

    /// Resolve the preferred default profile key when available.
    pub fn default_profile(&self) -> Option<&str> {
        for p in ["MS", "PMS", "PSH", "PAN", "SWIR"] {
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

    /// Read canonical band raster.
    pub fn read_band(&self, key: &str) -> Result<Raster> {
        let p = self.band_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("DIMAP band '{}' not found", key))
        })?;
        Raster::read(p)
    }

    /// Read canonical band raster for a specific profile key.
    pub fn read_band_for_profile(&self, profile: &str, key: &str) -> Result<Raster> {
        let p = self.band_path_for_profile(profile, key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "DIMAP band '{}' not found in profile '{}'",
                key, profile
            ))
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

fn has_raster_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let ext = e.to_string_lossy();
            ext.eq_ignore_ascii_case("jp2")
                || ext.eq_ignore_ascii_case("tif")
                || ext.eq_ignore_ascii_case("tiff")
        })
        .unwrap_or(false)
}

fn is_dimap_xml(path: &Path) -> bool {
    if !path
        .extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
    {
        return false;
    }
    path.file_name()
        .map(|n| n.to_string_lossy().to_ascii_uppercase().starts_with("DIM_"))
        .unwrap_or(false)
}

fn canonical_band_key(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    let tokens = split_tokens(&stem);
    if has_any_token(&tokens, &["P", "PAN"]) {
        return Some("PAN".to_string());
    }
    if has_any_token(&tokens, &["B0", "COASTAL"]) {
        return Some("B0".to_string());
    }
    if has_any_token(&tokens, &["B1", "BLUE", "XS1"]) {
        return Some("B1".to_string());
    }
    if has_any_token(&tokens, &["B2", "GREEN", "XS2"]) {
        return Some("B2".to_string());
    }
    if has_any_token(&tokens, &["B3", "RED", "XS3"]) {
        return Some("B3".to_string());
    }
    if has_any_token(&tokens, &["B4", "NIR", "XS4"]) {
        return Some("B4".to_string());
    }
    if has_any_token(&tokens, &["B5", "RE", "REDEDGE"]) {
        return Some("B5".to_string());
    }
    if has_any_token(&tokens, &["SWIR1", "B6", "S1"]) {
        return Some("SWIR1".to_string());
    }
    if has_any_token(&tokens, &["SWIR2", "B7", "S2"]) {
        return Some("SWIR2".to_string());
    }
    if has_any_token(&tokens, &["SWIR"]) {
        return Some("SWIR".to_string());
    }
    None
}

fn detect_dimap_profile(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    if stem.contains("PSH") {
        return "PSH".to_string();
    }
    if stem.contains("PMS") {
        return "PMS".to_string();
    }
    if stem.contains("SWIR") {
        return "SWIR".to_string();
    }
    if stem.contains("PAN") || stem.contains("_P") {
        return "PAN".to_string();
    }
    if stem.contains("MS") || stem.contains("XS") || stem.contains("B") {
        return "MS".to_string();
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

fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
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

fn extract_first_tag_value(xml: &str, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(v) = extract_tag_value(xml, tag) {
            return Some(v);
        }
    }
    None
}

fn extract_first_tag_number(xml: &str, tags: &[&str]) -> Option<f64> {
    for tag in tags {
        if let Some(v) = extract_tag_value(xml, tag) {
            if let Some(n) = parse_first_number(&v) {
                return Some(n);
            }
        }
    }
    None
}

fn parse_first_number(text: &str) -> Option<f64> {
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E') {
            token.push(ch);
        } else if !token.is_empty() {
            if let Ok(v) = token.parse::<f64>() {
                return Some(v);
            }
            token.clear();
        }
    }
    if token.is_empty() {
        None
    } else {
        token.parse::<f64>().ok()
    }
}

fn combine_date_time(date: Option<String>, time: Option<String>) -> Option<String> {
    let d = date?;
    let t = time?;
    let tt = t.trim_end_matches('Z');
    Some(format!("{}T{}Z", d.trim(), tt.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_dimap_bundle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("DIMAP");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("DIM_PHR1A_PMS_001.XML"),
            "<Dimap_Document><MISSION>PLEIADES</MISSION><DATASET_NAME>SCENE_01</DATASET_NAME></Dimap_Document>",
        )
        .expect("xml");
        fs::write(root.join("IMG_B1.JP2"), b"").expect("b1");
        fs::write(root.join("IMG_B2.JP2"), b"").expect("b2");
        fs::write(root.join("IMG_P.JP2"), b"").expect("pan");

        let b = DimapBundle::open(&root).expect("open");
        assert_eq!(b.mission.as_deref(), Some("PLEIADES"));
        assert_eq!(b.scene_id.as_deref(), Some("SCENE_01"));
        assert!(!b.list_profiles().is_empty());
        assert!(b.band_path("B1").is_some());
        assert!(b.band_path("B2").is_some());
        assert!(b.band_path("PAN").is_some());
    }

    #[test]
    fn opens_real_dimap_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_DIMAP_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let b = DimapBundle::open(&root).expect("open real dimap sample");
        assert!(!b.list_band_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_DIMAP_SAMPLE_EXPECT_PROFILES",
            b.list_profiles(),
            "DIMAP profile",
        );
    }

    #[test]
    fn maps_xs_and_swir_variants() {
        assert_eq!(canonical_band_key(Path::new("IMG_XS1.JP2")).as_deref(), Some("B1"));
        assert_eq!(canonical_band_key(Path::new("IMG_XS2.JP2")).as_deref(), Some("B2"));
        assert_eq!(canonical_band_key(Path::new("IMG_XS3.JP2")).as_deref(), Some("B3"));
        assert_eq!(canonical_band_key(Path::new("IMG_XS4.JP2")).as_deref(), Some("B4"));
        assert_eq!(canonical_band_key(Path::new("IMG_SWIR1.TIF")).as_deref(), Some("SWIR1"));
        assert_eq!(canonical_band_key(Path::new("IMG_SWIR2.TIF")).as_deref(), Some("SWIR2"));
    }

    #[test]
    fn parses_extended_dimap_metadata() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("DIMAP_EXT");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("DIM_SPOT7_MS_001.XML"),
            "<Dimap_Document><MISSION>SPOT</MISSION><DATASET_NAME>SCENE_02</DATASET_NAME><IMAGING_DATE>2026-04-01</IMAGING_DATE><IMAGING_TIME>10:11:12.000</IMAGING_TIME><PROCESSING_LEVEL>L2A</PROCESSING_LEVEL><CLOUD_COVERAGE>7.5%</CLOUD_COVERAGE><SUN_AZIMUTH>145.0 deg</SUN_AZIMUTH><SUN_ELEVATION>41.2 deg</SUN_ELEVATION></Dimap_Document>",
        )
        .expect("xml");
        fs::write(root.join("IMG_XS1.JP2"), b"").expect("b1");

        let b = DimapBundle::open(&root).expect("open");
        assert_eq!(b.acquisition_datetime_utc.as_deref(), Some("2026-04-01T10:11:12.000Z"));
        assert_eq!(b.processing_level.as_deref(), Some("L2A"));
        assert_eq!(b.cloud_cover_percent, Some(7.5));
        assert_eq!(b.sun_azimuth_deg, Some(145.0));
        assert_eq!(b.sun_elevation_deg, Some(41.2));
    }

    #[test]
    fn groups_assets_by_dimap_profile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("DIMAP_GROUPS");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(root.join("DIM_X.XML"), "<Dimap_Document></Dimap_Document>").expect("xml");
        fs::write(root.join("IMG_PAN.JP2"), b"").expect("pan");
        fs::write(root.join("IMG_XS1.JP2"), b"").expect("xs1");

        let b = DimapBundle::open(&root).expect("open");
        assert!(b.list_profiles().iter().any(|p| p == "PAN"));
        assert!(b.list_profiles().iter().any(|p| p == "MS"));
        assert!(b.band_path_for_profile("PAN", "PAN").is_some());
        assert!(b.band_path_for_profile("MS", "B1").is_some());
    }
}
