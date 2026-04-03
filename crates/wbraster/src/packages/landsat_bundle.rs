//! Landsat Collection bundle reader.
//!
//! This module provides package-level discovery and metadata parsing for
//! Landsat Collection scene bundles. Pixel decoding is delegated to existing
//! raster readers (typically GeoTIFF/COG).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Landsat spacecraft mission identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandsatMission {
    /// Landsat 4.
    Landsat4,
    /// Landsat 5.
    Landsat5,
    /// Landsat 7.
    Landsat7,
    /// Landsat 8.
    Landsat8,
    /// Landsat 9.
    Landsat9,
    /// Mission could not be inferred from metadata.
    Unknown,
}

/// Landsat processing level parsed from bundle metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandsatProcessingLevel {
    /// Level-1 product (e.g. L1TP).
    L1,
    /// Level-2 product (e.g. L2SP).
    L2,
    /// Processing level could not be inferred.
    Unknown,
}

/// Parsed Landsat Collection scene bundle.
#[derive(Debug, Clone)]
pub struct LandsatBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Path to MTL metadata file used for parsing.
    pub mtl_path: PathBuf,
    /// Mission identifier.
    pub mission: LandsatMission,
    /// Processing level identifier.
    pub processing_level: LandsatProcessingLevel,
    /// Product identifier when present.
    pub product_id: Option<String>,
    /// Collection number when present.
    pub collection_number: Option<String>,
    /// Scene acquisition date in UTC when present.
    pub acquisition_date_utc: Option<String>,
    /// Scene center time in UTC when present.
    pub scene_center_time_utc: Option<String>,
    /// WRS path/row when present.
    pub path_row: Option<(u16, u16)>,
    /// Cloud cover percentage when present.
    pub cloud_cover_percent: Option<f64>,
    /// Sun azimuth angle in degrees when present.
    pub sun_azimuth_deg: Option<f64>,
    /// Sun elevation angle in degrees when present.
    pub sun_elevation_deg: Option<f64>,
    /// Canonical spectral band key -> resolved raster path.
    pub bands: BTreeMap<String, PathBuf>,
    /// Canonical QA layer key -> resolved raster path.
    pub qa_layers: BTreeMap<String, PathBuf>,
    /// Canonical auxiliary layer key -> resolved raster path.
    pub aux_layers: BTreeMap<String, PathBuf>,
}

impl LandsatBundle {
    /// Open and parse a Landsat Collection scene bundle directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "Landsat bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mtl_path = find_mtl_file(&bundle_root)?.ok_or_else(|| {
            RasterError::MissingField("Landsat metadata text file (*_MTL.txt)".to_string())
        })?;
        let mtl_text = fs::read_to_string(&mtl_path)?;
        let kv = parse_mtl_key_values(&mtl_text);

        let mission = infer_mission(&kv);
        let processing_level = infer_processing_level(&kv);
        let product_id = get_text(&kv, &["LANDSAT_PRODUCT_ID", "PRODUCT_ID"]);
        let collection_number = get_text(&kv, &["COLLECTION_NUMBER"]);
        let acquisition_date_utc = get_text(&kv, &["DATE_ACQUIRED", "ACQUISITION_DATE"]);
        let scene_center_time_utc = get_text(&kv, &["SCENE_CENTER_TIME", "SCENE_TIME"]);
        let path_row = parse_path_row(&kv);
        let cloud_cover_percent = get_number(&kv, &["CLOUD_COVER", "CLOUD_COVER_LAND"]);
        let sun_azimuth_deg = get_number(&kv, &["SUN_AZIMUTH"]);
        let sun_elevation_deg = get_number(&kv, &["SUN_ELEVATION"]);

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;

        let mut bands = BTreeMap::new();
        let mut qa_layers = BTreeMap::new();
        let mut aux_layers = BTreeMap::new();

        for p in files {
            if !has_tiff_ext(&p) {
                continue;
            }
            if let Some(key) = canonical_qa_key_for_tiff(&p) {
                qa_layers.insert(key, p);
                continue;
            }
            if let Some(key) = canonical_aux_key_for_tiff(&p) {
                aux_layers.insert(key, p);
                continue;
            }
            if let Some(key) = canonical_band_key_for_tiff(&p) {
                bands.insert(key, p);
            }
        }

        if bands.is_empty() {
            return Err(RasterError::MissingField(
                "no Landsat band TIFF assets found in bundle".to_string(),
            ));
        }

        Ok(Self {
            bundle_root,
            mtl_path,
            mission,
            processing_level,
            product_id,
            collection_number,
            acquisition_date_utc,
            scene_center_time_utc,
            path_row,
            cloud_cover_percent,
            sun_azimuth_deg,
            sun_elevation_deg,
            bands,
            qa_layers,
            aux_layers,
        })
    }

    /// List canonical spectral band keys available in this bundle.
    pub fn list_band_keys(&self) -> Vec<String> {
        self.bands.keys().cloned().collect()
    }

    /// List canonical QA layer keys available in this bundle.
    pub fn list_qa_keys(&self) -> Vec<String> {
        self.qa_layers.keys().cloned().collect()
    }

    /// List canonical auxiliary layer keys available in this bundle.
    pub fn list_aux_keys(&self) -> Vec<String> {
        self.aux_layers.keys().cloned().collect()
    }

    /// Resolve a canonical band key to a raster file path.
    pub fn band_path(&self, key: &str) -> Option<&Path> {
        self.bands
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Resolve a canonical QA key to a raster file path.
    pub fn qa_path(&self, key: &str) -> Option<&Path> {
        self.qa_layers
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Resolve a canonical auxiliary key to a raster file path.
    pub fn aux_path(&self, key: &str) -> Option<&Path> {
        self.aux_layers
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical band directly as a [`Raster`].
    pub fn read_band(&self, key: &str) -> Result<Raster> {
        let p = self.band_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("band '{}' not found in Landsat bundle", key))
        })?;
        Raster::read(p)
    }

    /// Read a canonical QA layer directly as a [`Raster`].
    pub fn read_qa_layer(&self, key: &str) -> Result<Raster> {
        let p = self.qa_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("QA layer '{}' not found in Landsat bundle", key))
        })?;
        Raster::read(p)
    }

    /// Read a canonical auxiliary layer directly as a [`Raster`].
    pub fn read_aux_layer(&self, key: &str) -> Result<Raster> {
        let p = self.aux_path(key).ok_or_else(|| {
            RasterError::MissingField(format!(
                "aux layer '{}' not found in Landsat bundle",
                key
            ))
        })?;
        Raster::read(p)
    }
}

fn find_mtl_file(bundle_root: &Path) -> Result<Option<PathBuf>> {
    let mut files = Vec::new();
    collect_files_recursive(bundle_root, &mut files)?;
    let mut candidates: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_ascii_uppercase().ends_with("_MTL.TXT"))
                .unwrap_or(false)
        })
        .collect();
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

fn has_tiff_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let ext = e.to_string_lossy();
            ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
        })
        .unwrap_or(false)
}

fn parse_mtl_key_values(mtl_text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for raw_line in mtl_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else {
            continue;
        };
        let key = line[..eq].trim().to_ascii_uppercase();
        let mut value = line[eq + 1..].trim().to_string();
        if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
            value = value[1..value.len() - 1].to_string();
        }
        out.insert(key, value);
    }
    out
}

fn get_text(kv: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = kv.get(&key.to_ascii_uppercase()) {
            return Some(v.trim().to_string());
        }
    }
    None
}

fn get_number(kv: &BTreeMap<String, String>, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(v) = kv.get(&key.to_ascii_uppercase()) {
            if let Ok(n) = v.trim().parse::<f64>() {
                return Some(n);
            }
        }
    }
    None
}

fn parse_path_row(kv: &BTreeMap<String, String>) -> Option<(u16, u16)> {
    let path = get_text(kv, &["WRS_PATH"])?.parse::<u16>().ok()?;
    let row = get_text(kv, &["WRS_ROW"])?.parse::<u16>().ok()?;
    Some((path, row))
}

fn infer_mission(kv: &BTreeMap<String, String>) -> LandsatMission {
    let s = get_text(kv, &["SPACECRAFT_ID", "SPACECRAFT"]).unwrap_or_default();
    let u = s.to_ascii_uppercase();
    if u.contains('4') {
        LandsatMission::Landsat4
    } else if u.contains('5') {
        LandsatMission::Landsat5
    } else if u.contains('7') {
        LandsatMission::Landsat7
    } else if u.contains('8') {
        LandsatMission::Landsat8
    } else if u.contains('9') {
        LandsatMission::Landsat9
    } else {
        LandsatMission::Unknown
    }
}

fn infer_processing_level(kv: &BTreeMap<String, String>) -> LandsatProcessingLevel {
    let s = get_text(kv, &["PROCESSING_LEVEL", "DATA_TYPE"]).unwrap_or_default();
    let u = s.to_ascii_uppercase();
    if u.starts_with("L2") {
        LandsatProcessingLevel::L2
    } else if u.starts_with("L1") {
        LandsatProcessingLevel::L1
    } else {
        LandsatProcessingLevel::Unknown
    }
}

fn canonical_band_key_for_tiff(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();

    // Exclude known non-band families.
    if stem.contains("_QA_") || stem.contains("_SAA") || stem.contains("_SZA") {
        return None;
    }
    if stem.contains("_VAA") || stem.contains("_VZA") || stem.contains("_AOT") {
        return None;
    }

    // Prefer an exact `_B##` or `_B#` suffix token where available.
    for token in stem.split('_') {
        if let Some(rest) = token.strip_prefix('B') {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("B{}", rest));
            }
        }
    }

    None
}

fn canonical_qa_key_for_tiff(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    if stem.ends_with("_BQA") || stem.contains("_BQA_") {
        return Some("BQA".to_string());
    }
    if stem.contains("QA_PIXEL") {
        return Some("QA_PIXEL".to_string());
    }
    if stem.contains("QA_RADSAT") {
        return Some("QA_RADSAT".to_string());
    }
    if stem.contains("SR_QA_AEROSOL") {
        return Some("SR_QA_AEROSOL".to_string());
    }
    if stem.contains("SR_CLOUD_QA") {
        return Some("SR_CLOUD_QA".to_string());
    }
    if stem.contains("ST_QA") {
        return Some("ST_QA".to_string());
    }
    None
}

fn canonical_aux_key_for_tiff(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy().to_ascii_uppercase();
    if stem.contains("_SAA") {
        return Some("SAA".to_string());
    }
    if stem.contains("_SZA") {
        return Some("SZA".to_string());
    }
    if stem.contains("_VAA") {
        return Some("VAA".to_string());
    }
    if stem.contains("_VZA") {
        return Some("VZA".to_string());
    }
    if stem.contains("_AOT") {
        return Some("AOT".to_string());
    }
    if stem.contains("SR_ATMOS_OPACITY") {
        return Some("SR_ATMOS_OPACITY".to_string());
    }
    if stem.contains("ST_ATRAN") {
        return Some("ST_ATRAN".to_string());
    }
    if stem.contains("ST_DRAD") {
        return Some("ST_DRAD".to_string());
    }
    if stem.contains("ST_TRAD") {
        return Some("ST_TRAD".to_string());
    }
    if stem.contains("ST_URAD") {
        return Some("ST_URAD".to_string());
    }
    if stem.contains("ST_EMIS") {
        return Some("ST_EMIS".to_string());
    }
    if stem.contains("ST_EMSD") {
        return Some("ST_EMSD".to_string());
    }
    if stem.contains("ST_CDIST") {
        return Some("ST_CDIST".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_landsat_bundle_structure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("LC09_TEST_BUNDLE");
        fs::create_dir_all(&root).expect("create root");

        let mtl = r#"
GROUP = METADATA_FILE_INFO
  LANDSAT_PRODUCT_ID = "LC09_L2SP_018030_20240202_20240210_02_T1"
  SPACECRAFT_ID = "LANDSAT_9"
  COLLECTION_NUMBER = 2
  PROCESSING_LEVEL = "L2SP"
  DATE_ACQUIRED = 2024-02-02
  SCENE_CENTER_TIME = "16:42:31.1234560Z"
  WRS_PATH = 18
  WRS_ROW = 30
  CLOUD_COVER = 12.34
  SUN_AZIMUTH = 145.2
  SUN_ELEVATION = 38.6
END_GROUP = METADATA_FILE_INFO
END
"#;
        fs::write(
            root.join("LC09_L2SP_018030_20240202_20240210_02_T1_MTL.txt"),
            mtl,
        )
        .expect("write mtl");

        fs::write(root.join("LC09_L2SP_018030_20240202_20240210_02_T1_SR_B2.TIF"), b"")
            .expect("band b2");
        fs::write(root.join("LC09_L2SP_018030_20240202_20240210_02_T1_SR_B4.TIF"), b"")
            .expect("band b4");
        fs::write(root.join("LC09_L2SP_018030_20240202_20240210_02_T1_ST_B10.TIF"), b"")
            .expect("band b10");
        fs::write(
            root.join("LC09_L2SP_018030_20240202_20240210_02_T1_QA_PIXEL.TIF"),
            b"",
        )
        .expect("qa pixel");
        fs::write(
            root.join("LC09_L2SP_018030_20240202_20240210_02_T1_QA_RADSAT.TIF"),
            b"",
        )
        .expect("qa radsat");
        fs::write(root.join("LC09_L2SP_018030_20240202_20240210_02_T1_SAA.TIF"), b"")
            .expect("aux saa");

        let bundle = LandsatBundle::open(&root).expect("open landsat bundle");

        assert_eq!(bundle.mission, LandsatMission::Landsat9);
        assert_eq!(bundle.processing_level, LandsatProcessingLevel::L2);
        assert_eq!(bundle.product_id.as_deref(), Some("LC09_L2SP_018030_20240202_20240210_02_T1"));
        assert_eq!(bundle.collection_number.as_deref(), Some("2"));
        assert_eq!(bundle.acquisition_date_utc.as_deref(), Some("2024-02-02"));
        assert_eq!(bundle.scene_center_time_utc.as_deref(), Some("16:42:31.1234560Z"));
        assert_eq!(bundle.path_row, Some((18, 30)));
        assert_eq!(bundle.cloud_cover_percent, Some(12.34));
        assert_eq!(bundle.sun_azimuth_deg, Some(145.2));
        assert_eq!(bundle.sun_elevation_deg, Some(38.6));

        assert!(bundle.band_path("B2").is_some());
        assert!(bundle.band_path("B4").is_some());
        assert!(bundle.band_path("B10").is_some());
        assert!(bundle.qa_path("QA_PIXEL").is_some());
        assert!(bundle.qa_path("QA_RADSAT").is_some());
        assert!(bundle.aux_path("SAA").is_some());
    }

    #[test]
    fn rejects_bundle_without_bands() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("L8_EMPTY");
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("LC08_X_MTL.txt"), "SPACECRAFT_ID = \"LANDSAT_8\"")
            .expect("write mtl");

        let err = LandsatBundle::open(&root).expect_err("should fail due to missing bands");
        let msg = format!("{err}");
        assert!(msg.contains("no Landsat band TIFF assets"), "{msg}");
    }

    #[test]
    fn indexes_landsat7_l2_sr_st_qa_variants() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("LE07_SAMPLE");
        fs::create_dir_all(&root).expect("create root");

        fs::write(
            root.join("LE07_L2SP_018030_20010706_20200917_02_T1_MTL.txt"),
            "SPACECRAFT_ID = \"LANDSAT_7\"\nPROCESSING_LEVEL = \"L2SP\"\n",
        )
        .expect("write mtl");

        // Representative Collection 2 L2 scene assets observed in real bundles.
        for name in [
            "LE07_L2SP_018030_20010706_20200917_02_T1_SR_B1.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_SR_B2.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_B6.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_QA_PIXEL.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_QA_RADSAT.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_SR_CLOUD_QA.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_SR_ATMOS_OPACITY.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_QA.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_ATRAN.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_DRAD.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_TRAD.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_URAD.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_EMIS.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_EMSD.TIF",
            "LE07_L2SP_018030_20010706_20200917_02_T1_ST_CDIST.TIF",
        ] {
            fs::write(root.join(name), b"").expect("write sample tiff");
        }

        let bundle = LandsatBundle::open(&root).expect("open landsat bundle");

        assert_eq!(bundle.mission, LandsatMission::Landsat7);
        assert_eq!(bundle.processing_level, LandsatProcessingLevel::L2);

        assert!(bundle.band_path("B1").is_some());
        assert!(bundle.band_path("B2").is_some());
        assert!(bundle.band_path("B6").is_some());

        assert!(bundle.qa_path("QA_PIXEL").is_some());
        assert!(bundle.qa_path("QA_RADSAT").is_some());
        assert!(bundle.qa_path("SR_CLOUD_QA").is_some());
        assert!(bundle.qa_path("ST_QA").is_some());

        assert!(bundle.aux_path("SR_ATMOS_OPACITY").is_some());
        assert!(bundle.aux_path("ST_ATRAN").is_some());
        assert!(bundle.aux_path("ST_DRAD").is_some());
        assert!(bundle.aux_path("ST_TRAD").is_some());
        assert!(bundle.aux_path("ST_URAD").is_some());
        assert!(bundle.aux_path("ST_EMIS").is_some());
        assert!(bundle.aux_path("ST_EMSD").is_some());
        assert!(bundle.aux_path("ST_CDIST").is_some());
    }

    #[test]
    fn opens_real_landsat_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_LANDSAT_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let bundle = LandsatBundle::open(&root).expect("open real landsat sample");
        assert!(!bundle.list_band_keys().is_empty());
        assert!(bundle.qa_path("QA_PIXEL").is_some() || bundle.qa_path("BQA").is_some());
        assert_expected_csv_tokens_present(
            "WBRASTER_LANDSAT_SAMPLE_EXPECT_KEYS",
            bundle
                .list_band_keys()
                .into_iter()
                .chain(bundle.list_qa_keys())
                .chain(bundle.list_aux_keys()),
            "Landsat canonical key",
        );
    }
}
