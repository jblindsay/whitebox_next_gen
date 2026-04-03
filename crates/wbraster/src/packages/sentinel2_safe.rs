//! Sentinel-2 SAFE package reader.
//!
//! This module provides package-level discovery and metadata parsing for
//! Sentinel-2 SAFE products while delegating actual pixel decoding to existing
//! raster readers (typically JPEG2000/GeoJP2).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Sentinel-2 product level parsed from SAFE metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sentinel2ProductLevel {
    /// Level-1C top-of-atmosphere reflectance product.
    L1C,
    /// Level-2A surface reflectance product.
    L2A,
    /// Product level could not be determined.
    Unknown,
}

/// A parsed Sentinel-2 SAFE package.
#[derive(Debug, Clone)]
pub struct Sentinel2SafePackage {
    /// Root SAFE directory path.
    pub safe_root: PathBuf,
    /// Sentinel-2 product level.
    pub product_level: Sentinel2ProductLevel,
    /// Acquisition datetime in UTC when present in metadata.
    pub acquisition_datetime_utc: Option<String>,
    /// Mean solar zenith angle in degrees when present in metadata.
    pub mean_solar_zenith_deg: Option<f64>,
    /// Mean solar azimuth angle in degrees when present in metadata.
    pub mean_solar_azimuth_deg: Option<f64>,
    /// Tile identifier when present.
    pub tile_id: Option<String>,
    /// Cloud coverage percentage (0–100) when present in product metadata.
    pub cloud_coverage_assessment: Option<f64>,
    /// ESA processing baseline version string when present (e.g. `"05.09"`).
    pub processing_baseline: Option<String>,
    /// Canonical spectral band key -> resolved raster path.
    pub bands: BTreeMap<String, PathBuf>,
    /// Canonical QA layer key -> resolved raster path (SCL, QA60).
    pub qa_layers: BTreeMap<String, PathBuf>,
    /// Canonical L2A auxiliary layer key -> resolved raster path (AOT, WVP, TCI).
    pub aux_layers: BTreeMap<String, PathBuf>,
}

impl Sentinel2SafePackage {
    /// Open and parse a Sentinel-2 SAFE package directory.
    pub fn open(safe_root: impl AsRef<Path>) -> Result<Self> {
        let safe_root = safe_root.as_ref().to_path_buf();
        if !safe_root.is_dir() {
            return Err(RasterError::Other(format!(
                "SAFE root is not a directory: {}",
                safe_root.display()
            )));
        }

        let product_xml = find_product_xml(&safe_root)?.ok_or_else(|| {
            RasterError::MissingField("SAFE product metadata XML (MTD_MSI*.xml)".to_string())
        })?;
        let product_xml_text = fs::read_to_string(&product_xml)?;

        let product_level = infer_product_level(&product_xml_text);
        let acquisition_datetime_utc = extract_first_text(
            &product_xml_text,
            &[
                "DATATAKE_SENSING_START",
                "SENSING_TIME",
                "PRODUCT_START_TIME",
                "GENERATION_TIME",
            ],
        );
        let mean_solar_zenith_deg = extract_first_number(
            &product_xml_text,
            &["ZENITH_ANGLE", "Mean_Sun_Angle_Zenith", "SUN_ZENITH"],
        );
        let mean_solar_azimuth_deg = extract_first_number(
            &product_xml_text,
            &["AZIMUTH_ANGLE", "Mean_Sun_Angle_Azimuth", "SUN_AZIMUTH"],
        );
        let tile_id = extract_first_text(&product_xml_text, &["TILE_ID", "MGRS_TILE"]);

        let mut bands: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut qa_layers: BTreeMap<String, PathBuf> = BTreeMap::new();
        let cloud_coverage_assessment = extract_first_number(
            &product_xml_text,
            &["CLOUD_COVERAGE_ASSESSMENT", "Cloud_Coverage_Assessment"],
        );
        let processing_baseline = extract_first_text(
            &product_xml_text,
            &["PROCESSING_BASELINE", "Processing_Baseline"],
        );
        let mut aux_layers: BTreeMap<String, PathBuf> = BTreeMap::new();

        let mut files = Vec::new();
        collect_files_recursive(&safe_root, &mut files)?;
        for path in files {
            if !has_jp2_ext(&path) {
                continue;
            }
            if let Some(aux_key) = canonical_aux_key_for_jp2(&path) {
                upsert_best_resolution(&mut aux_layers, aux_key, path);
                continue;
            }
            if let Some((key, is_qa)) = canonical_key_for_jp2(&path) {
                if is_qa {
                    upsert_best_resolution(&mut qa_layers, key, path);
                } else {
                    upsert_best_resolution(&mut bands, key, path);
                }
            }
        }

        Ok(Self {
            safe_root,
            product_level,
            acquisition_datetime_utc,
            mean_solar_zenith_deg,
            mean_solar_azimuth_deg,
            tile_id,
            cloud_coverage_assessment,
            processing_baseline,
            bands,
            qa_layers,
            aux_layers,
        })
    }

    /// List canonical spectral band keys available in this package.
    pub fn list_band_keys(&self) -> Vec<String> {
        self.bands.keys().cloned().collect()
    }

    /// List canonical QA layer keys available in this package.
    pub fn list_qa_keys(&self) -> Vec<String> {
        self.qa_layers.keys().cloned().collect()
    }

    /// Resolve a canonical spectral band key to a raster file path.
    pub fn band_path(&self, key: &str) -> Option<&Path> {
        self.bands.get(&key.to_ascii_uppercase()).map(PathBuf::as_path)
    }

    /// Resolve a canonical QA layer key to a raster file path.
    pub fn qa_path(&self, key: &str) -> Option<&Path> {
        self.qa_layers
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical band directly as a [`Raster`].
    pub fn read_band(&self, key: &str) -> Result<Raster> {
        let p = self.band_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("band '{}' not found in SAFE package", key))
        })?;
        Raster::read(p)
    }

    /// List canonical L2A auxiliary layer keys (AOT, WVP, TCI) available in this package.
    pub fn list_aux_keys(&self) -> Vec<String> {
        self.aux_layers.keys().cloned().collect()
    }

    /// Resolve a canonical auxiliary layer key to a raster file path.
    pub fn aux_path(&self, key: &str) -> Option<&Path> {
        self.aux_layers
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical L2A auxiliary layer (AOT, WVP, TCI) as a [`Raster`].
    pub fn read_aux_layer(&self, key: &str) -> Result<Raster> {
        let p = self.aux_path(key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "auxiliary layer '{}' not found in SAFE package",
                key
            ))
        })?;
        Raster::read(p)
    }
}

fn has_jp2_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("jp2"))
        .unwrap_or(false)
}

fn find_product_xml(safe_root: &Path) -> Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(safe_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let upper = name.to_ascii_uppercase();
        if upper.starts_with("MTD_MSI") && upper.ends_with(".XML") {
            candidates.push(path);
        }
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
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

fn infer_product_level(xml: &str) -> Sentinel2ProductLevel {
    let u = xml.to_ascii_uppercase();
    if u.contains("MSIL2A") || u.contains("LEVEL-2A") {
        Sentinel2ProductLevel::L2A
    } else if u.contains("MSIL1C") || u.contains("LEVEL-1C") {
        Sentinel2ProductLevel::L1C
    } else {
        Sentinel2ProductLevel::Unknown
    }
}

fn extract_first_text(xml: &str, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(v) = extract_tag_text(xml, tag) {
            return Some(v);
        }
    }
    None
}

fn extract_first_number(xml: &str, tags: &[&str]) -> Option<f64> {
    for tag in tags {
        if let Some(v) = extract_tag_text(xml, tag).and_then(|s| parse_first_number(&s)) {
            return Some(v);
        }
    }
    None
}

fn extract_tag_text(xml: &str, tag_name: &str) -> Option<String> {
    let mut i = 0usize;
    let bytes = xml.as_bytes();
    while i < bytes.len() {
        let rel_start = xml[i..].find('<')?;
        let start = i + rel_start;
        let rel_end = xml[start..].find('>')?;
        let end = start + rel_end;
        let header = &xml[start + 1..end];
        if !header.starts_with('/') && header_contains_tag_name(header, tag_name) {
            let close = xml[end + 1..].find('<')?;
            let text = xml[end + 1..end + 1 + close].trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
        i = end + 1;
    }
    None
}

fn header_contains_tag_name(header: &str, tag_name: &str) -> bool {
    let name = header
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    if name.eq_ignore_ascii_case(tag_name) {
        return true;
    }
    if let Some(idx) = name.rfind(':') {
        return name[idx + 1..].eq_ignore_ascii_case(tag_name);
    }
    false
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

fn canonical_key_for_jp2(path: &Path) -> Option<(String, bool)> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();

    // Aux layers are handled separately; prevent them from matching band names.
    if stem.contains("_AOT") || stem.contains("_WVP") || stem.contains("_TCI") {
        return None;
    }

    if stem.contains("_SCL") || stem == "SCL" {
        return Some(("SCL".to_string(), true));
    }
    if stem.contains("_QA60") || stem == "QA60" {
        return Some(("QA60".to_string(), true));
    }
    if stem.contains("MSK_CLDPRB") {
        return Some(("MSK_CLDPRB".to_string(), true));
    }
    if stem.contains("MSK_SNWPRB") {
        return Some(("MSK_SNWPRB".to_string(), true));
    }
    if stem.contains("MSK_CLASSI") {
        return Some(("MSK_CLASSI".to_string(), true));
    }
    if stem.contains("MSK_DETFOO") {
        return Some(("MSK_DETFOO".to_string(), true));
    }
    if stem.contains("MSK_QUALIT") {
        return Some(("MSK_QUALIT".to_string(), true));
    }

    const BAND_KEYS: [&str; 13] = [
        "B8A", "B01", "B02", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "B10", "B11", "B12",
    ];

    for key in BAND_KEYS {
        if stem.contains(&format!("_{key}")) || stem.ends_with(key) {
            return Some((key.to_string(), false));
        }
    }

    None
}

fn resolution_score(path: &Path) -> i32 {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    if name.contains("_10M") {
        10
    } else if name.contains("_20M") {
        20
    } else if name.contains("_60M") {
        60
    } else {
        999
    }
}

fn upsert_best_resolution(map: &mut BTreeMap<String, PathBuf>, key: String, new_path: PathBuf) {
    let take_new = match map.get(&key) {
        None => true,
        Some(existing) => resolution_score(&new_path) < resolution_score(existing),
    };
    if take_new {
        map.insert(key, new_path);
    }
}

fn canonical_aux_key_for_jp2(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    if stem.contains("_AOT") {
        return Some("AOT".to_string());
    }
    if stem.contains("_WVP") {
        return Some("WVP".to_string());
    }
    if stem.contains("_TCI") {
        return Some("TCI".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_safe_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let safe = tmp.path().join("S2A_TEST_MSIL2A.SAFE");
        fs::create_dir_all(&safe).unwrap();

        let xml = r#"
            <n1:Level-2A_User_Product>
              <General_Info>
                <Product_Info>
                  <PRODUCT_START_TIME>2026-03-31T15:20:00.000Z</PRODUCT_START_TIME>
                                    <PROCESSING_BASELINE>05.09</PROCESSING_BASELINE>
                </Product_Info>
                <Product_Image_Characteristics>
                  <Mean_Sun_Angle>
                    <ZENITH_ANGLE unit=\"deg\">34.2</ZENITH_ANGLE>
                    <AZIMUTH_ANGLE unit=\"deg\">158.7</AZIMUTH_ANGLE>
                  </Mean_Sun_Angle>
                                    <CLOUD_COVERAGE_ASSESSMENT>12.5</CLOUD_COVERAGE_ASSESSMENT>
                </Product_Image_Characteristics>
              </General_Info>
            </n1:Level-2A_User_Product>
        "#;
        fs::write(safe.join("MTD_MSIL2A.xml"), xml).unwrap();

        let img = safe.join("GRANULE").join("T32ABC_001").join("IMG_DATA").join("R10m");
        fs::create_dir_all(&img).unwrap();
        fs::write(img.join("T32ABC_20260331T152000_B04_10m.jp2"), b"").unwrap();
        fs::write(img.join("T32ABC_20260331T152000_B08_10m.jp2"), b"").unwrap();
        fs::write(img.join("T32ABC_20260331T152000_AOT_10m.jp2"), b"").unwrap();
        fs::write(img.join("T32ABC_20260331T152000_TCI_10m.jp2"), b"").unwrap();

        let qa = safe
            .join("GRANULE")
            .join("T32ABC_001")
            .join("IMG_DATA")
            .join("R20m");
        fs::create_dir_all(&qa).unwrap();
        fs::write(qa.join("T32ABC_20260331T152000_SCL_20m.jp2"), b"").unwrap();
        fs::write(qa.join("T32ABC_20260331T152000_WVP_20m.jp2"), b"").unwrap();

        let pkg = Sentinel2SafePackage::open(&safe).unwrap();
        assert_eq!(pkg.product_level, Sentinel2ProductLevel::L2A);
        assert!(pkg.band_path("B04").is_some());
        assert!(pkg.band_path("B08").is_some());
        assert!(pkg.qa_path("SCL").is_some());
        assert!(!pkg.list_qa_keys().is_empty());
        assert_eq!(pkg.mean_solar_zenith_deg, Some(34.2));
        assert_eq!(pkg.mean_solar_azimuth_deg, Some(158.7));
        assert!(pkg.acquisition_datetime_utc.is_some());
        assert_eq!(pkg.cloud_coverage_assessment, Some(12.5));
        assert_eq!(pkg.processing_baseline.as_deref(), Some("05.09"));
        assert!(pkg.aux_path("AOT").is_some(), "AOT aux layer missing");
        assert!(pkg.aux_path("WVP").is_some(), "WVP aux layer missing");
        assert!(pkg.aux_path("TCI").is_some(), "TCI aux layer missing");
        assert!(pkg.band_path("AOT").is_none(), "AOT must not bleed into bands");
        assert!(pkg.band_path("TCI").is_none(), "TCI must not bleed into bands");
    }

    #[test]
    fn qa_mask_variants_are_classified_as_qa_layers() {
        let tmp = tempfile::tempdir().unwrap();
        let safe = tmp.path().join("S2A_TEST_MSIL2A.SAFE");
        fs::create_dir_all(&safe).unwrap();

        fs::write(
            safe.join("MTD_MSIL2A.xml"),
            "<Level-2A_User_Product><General_Info><Product_Info><PRODUCT_START_TIME>2026-01-01T00:00:00Z</PRODUCT_START_TIME></Product_Info></General_Info></Level-2A_User_Product>",
        )
        .unwrap();

        let qid = safe
            .join("GRANULE")
            .join("T17TNJ_001")
            .join("QI_DATA");
        fs::create_dir_all(&qid).unwrap();
        fs::write(qid.join("MSK_CLDPRB_20m.jp2"), b"").unwrap();
        fs::write(qid.join("MSK_SNWPRB_20m.jp2"), b"").unwrap();
        fs::write(qid.join("MSK_CLASSI_B00.jp2"), b"").unwrap();
        fs::write(qid.join("MSK_DETFOO_B02.jp2"), b"").unwrap();
        fs::write(qid.join("MSK_QUALIT_B02.jp2"), b"").unwrap();

        let img = safe.join("GRANULE").join("T17TNJ_001").join("IMG_DATA").join("R10m");
        fs::create_dir_all(&img).unwrap();
        fs::write(img.join("T17TNJ_20260101T000000_B04_10m.jp2"), b"").unwrap();

        let pkg = Sentinel2SafePackage::open(&safe).unwrap();
        assert!(pkg.qa_path("MSK_CLDPRB").is_some());
        assert!(pkg.qa_path("MSK_SNWPRB").is_some());
        assert!(pkg.qa_path("MSK_CLASSI").is_some());
        assert!(pkg.qa_path("MSK_DETFOO").is_some());
        assert!(pkg.qa_path("MSK_QUALIT").is_some());
        assert!(pkg.band_path("B04").is_some());
    }

    #[test]
    fn opens_real_s2_safe_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_S2_SAFE_SAMPLE") else {
            return;
        };
        let safe_root = PathBuf::from(path);
        if !safe_root.is_dir() {
            return;
        }

        let pkg = Sentinel2SafePackage::open(&safe_root).expect("open real SAFE sample");
        assert_eq!(pkg.product_level, Sentinel2ProductLevel::L2A);
        assert!(pkg.band_path("B04").is_some() || pkg.band_path("B03").is_some());
        assert!(!pkg.list_qa_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_S2_SAFE_SAMPLE_EXPECT_KEYS",
            pkg.list_band_keys()
                .into_iter()
                .chain(pkg.list_qa_keys())
                .chain(pkg.list_aux_keys()),
            "SAFE canonical key",
        );
    }
}
