//! Maxar/WorldView scene bundle reader.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed Maxar/WorldView scene bundle.
#[derive(Debug, Clone)]
pub struct MaxarWorldViewBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Metadata file path (`.IMD` or XML) when found.
    pub metadata_path: Option<PathBuf>,
    /// Platform/satellite id when present.
    pub satellite: Option<String>,
    /// Catalog/image id when present.
    pub scene_id: Option<String>,
    /// Acquisition datetime UTC when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Cloud cover percentage when present.
    pub cloud_cover_percent: Option<f64>,
    /// Mean sun azimuth in degrees when present.
    pub sun_azimuth_deg: Option<f64>,
    /// Mean sun elevation in degrees when present.
    pub sun_elevation_deg: Option<f64>,
    /// Mean off-nadir view angle in degrees when present.
    pub off_nadir_angle_deg: Option<f64>,
    /// Canonical band key -> path.
    pub bands: BTreeMap<String, PathBuf>,
    /// Profile key -> (canonical band key -> path).
    pub profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>>,
}

impl MaxarWorldViewBundle {
    /// Open and parse a Maxar/WorldView scene directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "Maxar/WorldView bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;
        files.sort();

        let mut metadata_path = files.iter().find(|p| has_imd_ext(p)).cloned();
        if metadata_path.is_none() {
            metadata_path = files.iter().find(|p| has_xml_ext(p)).cloned();
        }

        let mut bands = BTreeMap::new();
        let mut profile_bands: BTreeMap<String, BTreeMap<String, PathBuf>> = BTreeMap::new();
        for p in &files {
            if !has_raster_ext(p) {
                continue;
            }
            if let Some(k) = canonical_band_key(p) {
                let profile = detect_maxar_profile(p);
                profile_bands
                    .entry(profile)
                    .or_default()
                    .insert(k.clone(), p.clone());
                bands.insert(k, p.clone());
            }
        }

        if bands.is_empty() {
            return Err(RasterError::MissingField(
                "no Maxar/WorldView raster assets found in bundle".to_string(),
            ));
        }

        let mut satellite = None;
        let mut scene_id = None;
        let mut acquisition_datetime_utc = None;
        let mut cloud_cover_percent = None;
        let mut sun_azimuth_deg = None;
        let mut sun_elevation_deg = None;
        let mut off_nadir_angle_deg = None;
        if let Some(meta) = metadata_path.as_ref() {
            if let Ok(text) = fs::read_to_string(meta) {
                satellite = extract_assignment(&text, "satId")
                    .or_else(|| extract_assignment(&text, "SATID"))
                    .or_else(|| extract_xml_tag(&text, "SATID"));
                scene_id = extract_assignment(&text, "CATID")
                    .or_else(|| extract_assignment(&text, "IMAGEID"))
                    .or_else(|| extract_xml_tag(&text, "CATID"));
                acquisition_datetime_utc = extract_first_assignment(
                    &text,
                    &["earliestAcqTime", "FIRSTLINETIME", "ACQUISITIONDATETIME"],
                )
                .or_else(|| extract_first_xml_tag(&text, &["FIRSTLINETIME", "ACQUISITIONDATETIME"]));
                cloud_cover_percent = extract_first_assignment_number(
                    &text,
                    &["cloudCover", "CLOUDCOVER"],
                )
                .or_else(|| extract_first_xml_tag_number(&text, &["CLOUDCOVER", "CLOUD_COVER"]));
                sun_azimuth_deg = extract_first_assignment_number(
                    &text,
                    &["meanSunAz", "MEANSUNAZ"],
                )
                .or_else(|| extract_first_xml_tag_number(&text, &["MEANSUNAZ", "SUN_AZIMUTH"]));
                sun_elevation_deg = extract_first_assignment_number(
                    &text,
                    &["meanSunEl", "MEANSUNEL"],
                )
                .or_else(|| extract_first_xml_tag_number(&text, &["MEANSUNEL", "SUN_ELEVATION"]));
                off_nadir_angle_deg = extract_first_assignment_number(
                    &text,
                    &["meanOffNadirViewAngle", "MEANOFFNADIRVIEWANGLE"],
                )
                .or_else(|| {
                    extract_first_xml_tag_number(
                        &text,
                        &["MEANOFFNADIRVIEWANGLE", "OFF_NADIR_ANGLE"],
                    )
                });
            }
        }

        Ok(Self {
            bundle_root,
            metadata_path,
            satellite,
            scene_id,
            acquisition_datetime_utc,
            cloud_cover_percent,
            sun_azimuth_deg,
            sun_elevation_deg,
            off_nadir_angle_deg,
            bands,
            profile_bands,
        })
    }

    /// List canonical band keys.
    pub fn list_band_keys(&self) -> Vec<String> {
        self.bands.keys().cloned().collect()
    }

    /// List profile keys (e.g. `MS`, `PAN`, `PSH`, `SWIR`).
    pub fn list_profiles(&self) -> Vec<String> {
        self.profile_bands.keys().cloned().collect()
    }

    /// Resolve the preferred default profile key when available.
    pub fn default_profile(&self) -> Option<&str> {
        for p in ["MS", "PSH", "PAN", "SWIR"] {
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
            RasterError::MissingField(format!("Maxar/WorldView band '{}' not found", key))
        })?;
        Raster::read(p)
    }

    /// Read canonical band raster for a specific profile key.
    pub fn read_band_for_profile(&self, profile: &str, key: &str) -> Result<Raster> {
        let p = self.band_path_for_profile(profile, key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "Maxar/WorldView band '{}' not found in profile '{}'",
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
            ext.eq_ignore_ascii_case("tif")
                || ext.eq_ignore_ascii_case("tiff")
                || ext.eq_ignore_ascii_case("jp2")
        })
        .unwrap_or(false)
}

fn has_imd_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("imd"))
        .unwrap_or(false)
}

fn has_xml_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
}

fn canonical_band_key(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    let tokens = split_tokens(&stem);
    if has_any_token(&tokens, &["P", "PAN", "BANDP"]) {
        return Some("PAN".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "C"]) || has_any_token(&tokens, &["BANDC", "COAST", "COASTAL"]) {
        return Some("B1".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "B"]) || has_any_token(&tokens, &["BANDB", "BLUE", "B2"]) {
        return Some("B2".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "G"]) || has_any_token(&tokens, &["BANDG", "GREEN", "B3"]) {
        return Some("B3".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "R"]) || has_any_token(&tokens, &["BANDR", "RED", "B4"]) {
        return Some("B4".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "N"]) || has_any_token(&tokens, &["BANDN", "NIR", "NIR1", "B5"]) {
        return Some("B5".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "RE"]) || has_any_token(&tokens, &["BANDRE", "RE", "REDEDGE", "B6"]) {
        return Some("RE".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "Y"]) || has_any_token(&tokens, &["Y", "YELLOW", "BANDY"]) {
        return Some("Y".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "N2"]) || has_any_token(&tokens, &["N2", "NIR2", "BANDN2", "B8"]) {
        return Some("N2".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "S1"]) || has_any_token(&tokens, &["SWIR1", "BANDS1", "S1"]) {
        return Some("SWIR1".to_string());
    }
    if has_all_tokens(&tokens, &["BAND", "S2"]) || has_any_token(&tokens, &["SWIR2", "BANDS2", "S2"]) {
        return Some("SWIR2".to_string());
    }
    if has_any_token(&tokens, &["SWIR"]) {
        return Some("SWIR".to_string());
    }
    None
}

fn detect_maxar_profile(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    if stem.contains("PSH") || stem.contains("PANSHARP") {
        return "PSH".to_string();
    }
    if stem.contains("SWIR") || stem.contains("S1") || stem.contains("S2") {
        return "SWIR".to_string();
    }
    if stem.contains("PAN") || stem.contains("BAND_P") || stem.contains("_P") {
        return "PAN".to_string();
    }
    if stem.contains("MS") || stem.contains("BAND") {
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

fn has_all_tokens(tokens: &[&str], required: &[&str]) -> bool {
    required.iter().all(|r| {
        tokens
            .iter()
            .any(|t| t.eq_ignore_ascii_case(r))
    })
}

fn extract_assignment(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let mut parts = line.splitn(2, '=');
        let lhs = parts.next()?.trim();
        let rhs = parts.next()?.trim().trim_matches('"');
        if lhs.eq_ignore_ascii_case(key) && !rhs.is_empty() {
            return Some(rhs.to_string());
        }
    }
    None
}

fn extract_first_assignment(text: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = extract_assignment(text, key) {
            return Some(v);
        }
    }
    None
}

fn extract_first_assignment_number(text: &str, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(v) = extract_assignment(text, key) {
            if let Some(n) = parse_first_number(&v) {
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

fn extract_first_xml_tag(xml: &str, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(v) = extract_xml_tag(xml, tag) {
            return Some(v);
        }
    }
    None
}

fn extract_first_xml_tag_number(xml: &str, tags: &[&str]) -> Option<f64> {
    for tag in tags {
        if let Some(v) = extract_xml_tag(xml, tag) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_maxar_bundle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("MAXAR");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("20JAN01120000-M1BS-000000000010_01_P001.IMD"),
            "satId = \"WV03\"\ncatId = \"104001007ABCDE00\"\n",
        )
        .expect("imd");
        fs::write(root.join("IMG_BAND_R.TIF"), b"").expect("red");
        fs::write(root.join("IMG_BAND_G.TIF"), b"").expect("green");
        fs::write(root.join("IMG_BAND_B.TIF"), b"").expect("blue");

        let b = MaxarWorldViewBundle::open(&root).expect("open");
        assert_eq!(b.satellite.as_deref(), Some("WV03"));
        assert_eq!(b.scene_id.as_deref(), Some("104001007ABCDE00"));
        assert!(!b.list_profiles().is_empty());
        assert!(b.band_path("B2").is_some());
        assert!(b.band_path("B3").is_some());
        assert!(b.band_path("B4").is_some());
    }

    #[test]
    fn opens_real_maxar_worldview_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_MAXAR_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let b = MaxarWorldViewBundle::open(&root).expect("open real maxar/worldview sample");
        assert!(!b.list_band_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_MAXAR_SAMPLE_EXPECT_PROFILES",
            b.list_profiles(),
            "Maxar/WorldView profile",
        );
    }

    #[test]
    fn maps_worldview_multispectral_variants() {
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_C.TIF")).as_deref(), Some("B1"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_B.TIF")).as_deref(), Some("B2"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_G.TIF")).as_deref(), Some("B3"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_R.TIF")).as_deref(), Some("B4"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_N.TIF")).as_deref(), Some("B5"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_RE.TIF")).as_deref(), Some("RE"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_Y.TIF")).as_deref(), Some("Y"));
        assert_eq!(canonical_band_key(Path::new("IMG_BAND_N2.TIF")).as_deref(), Some("N2"));
    }

    #[test]
    fn parses_extended_maxar_metadata_from_imd() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("MAXAR_EXT");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(
            root.join("scene.IMD"),
            "satId = \"WV03\"\ncatId = \"104001007ABCDE00\"\nearliestAcqTime = \"2026-04-01T12:34:56Z\"\ncloudCover = 3.4\nmeanSunAz = 149.2\nmeanSunEl = 43.1\nmeanOffNadirViewAngle = 12.7\n",
        )
        .expect("imd");
        fs::write(root.join("IMG_BAND_R.TIF"), b"").expect("red");

        let b = MaxarWorldViewBundle::open(&root).expect("open");
        assert_eq!(b.acquisition_datetime_utc.as_deref(), Some("2026-04-01T12:34:56Z"));
        assert_eq!(b.cloud_cover_percent, Some(3.4));
        assert_eq!(b.sun_azimuth_deg, Some(149.2));
        assert_eq!(b.sun_elevation_deg, Some(43.1));
        assert_eq!(b.off_nadir_angle_deg, Some(12.7));
    }

    #[test]
    fn groups_assets_by_maxar_profile() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("MAXAR_GROUPS");
        fs::create_dir_all(&root).expect("mkdir");

        fs::write(root.join("scene.IMD"), "satId = \"WV03\"").expect("imd");
        fs::write(root.join("IMG_BAND_R.TIF"), b"").expect("ms");
        fs::write(root.join("IMG_PAN.TIF"), b"").expect("pan");

        let b = MaxarWorldViewBundle::open(&root).expect("open");
        assert!(b.list_profiles().iter().any(|p| p == "MS"));
        assert!(b.list_profiles().iter().any(|p| p == "PAN"));
        assert!(b.band_path_for_profile("MS", "B4").is_some());
        assert!(b.band_path_for_profile("PAN", "PAN").is_some());
    }
}
