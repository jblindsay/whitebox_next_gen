//! ICEYE bundle reader.
//!
//! ICEYE deliveries commonly package one or more COG/GeoTIFF assets with XML
//! metadata. This module resolves those assets and exposes basic package-level
//! metadata while delegating raster decoding to existing readers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde_json::Value;

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed ICEYE scene bundle.
#[derive(Debug, Clone)]
pub struct IceyeBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Metadata XML path when present.
    pub metadata_xml_path: Option<PathBuf>,
    /// Product type parsed from XML when present.
    pub product_type: Option<String>,
    /// Acquisition time in UTC parsed from XML when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Polarization parsed from XML when present.
    pub polarization: Option<String>,
    /// Acquisition mode parsed from XML when present.
    pub acquisition_mode: Option<String>,
    /// Orbit direction when present (ASCENDING / DESCENDING).
    pub orbit_direction: Option<String>,
    /// Look direction when present (RIGHT / LEFT).
    pub look_direction: Option<String>,
    /// Incidence angle at near range in degrees when present.
    pub incidence_angle_near_deg: Option<f64>,
    /// Incidence angle at far range in degrees when present.
    pub incidence_angle_far_deg: Option<f64>,
    /// Ground-range pixel spacing in metres when present.
    pub pixel_spacing_range_m: Option<f64>,
    /// Azimuth pixel spacing in metres when present.
    pub pixel_spacing_azimuth_m: Option<f64>,
    /// Canonical asset key -> COG/GeoTIFF path.
    pub assets: BTreeMap<String, PathBuf>,
}

impl IceyeBundle {
    /// Open and parse an ICEYE bundle directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "ICEYE bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;

        let mut xml_candidates = Vec::new();
        let mut json_candidates = Vec::new();
        let mut assets = BTreeMap::new();
        for p in files {
            if has_xml_ext(&p) {
                xml_candidates.push(p);
                continue;
            }
            if has_json_ext(&p) {
                json_candidates.push(p);
                continue;
            }
            if has_tiff_ext(&p) {
                let key = canonical_asset_key_with_collision_avoidance(&assets, &p);
                assets.insert(key, p);
            }
        }

        if assets.is_empty() {
            return Err(RasterError::MissingField(
                "no ICEYE COG/GeoTIFF assets found in bundle".to_string(),
            ));
        }

        xml_candidates.sort();
        let metadata_xml_path = xml_candidates.into_iter().next();
        json_candidates.sort();
        let metadata_json_path = json_candidates.into_iter().next();

        let mut product_type = None;
        let mut acquisition_datetime_utc = None;
        let mut polarization = None;
        let mut acquisition_mode = None;
        let mut orbit_direction = None;
        let mut look_direction = None;
        let mut incidence_angle_near_deg = None;
        let mut incidence_angle_far_deg = None;
        let mut pixel_spacing_range_m = None;
        let mut pixel_spacing_azimuth_m = None;
        if let Some(xml_path) = metadata_xml_path.as_ref() {
            let xml = fs::read_to_string(xml_path)?;
            product_type = extract_first_text(&xml, &["product_type", "productType"]);
            acquisition_datetime_utc = extract_first_text(
                &xml,
                &["acquisition_start_utc", "acquisitionStartUTC", "acquisition_time"],
            );
            polarization = extract_first_text(&xml, &["polarization", "polarisation"])
                .map(|s| s.to_ascii_uppercase());
            acquisition_mode = extract_first_text(
                &xml,
                &["acquisition_mode", "acquisitionMode", "imaging_mode", "imagingMode"],
            );
            orbit_direction = extract_first_text(
                &xml,
                &["orbit_direction", "orbitDirection", "passDirection"],
            )
            .map(|s| s.to_ascii_uppercase());
            look_direction = extract_first_text(
                &xml,
                &["look_side", "lookSide", "lookDirection", "antennaPointing"],
            )
            .map(|s| s.to_ascii_uppercase());
            incidence_angle_near_deg = extract_first_number(
                &xml,
                &["incidence_angle_near", "incidenceAngleNear", "incidenceAngleNearRange"],
            );
            incidence_angle_far_deg = extract_first_number(
                &xml,
                &["incidence_angle_far", "incidenceAngleFar", "incidenceAngleFarRange"],
            );
            pixel_spacing_range_m = extract_first_number(
                &xml,
                &["range_spacing", "rangeSpacing", "rangePixelSpacing", "sampledPixelSpacing"],
            );
            pixel_spacing_azimuth_m = extract_first_number(
                &xml,
                &["azimuth_spacing", "azimuthSpacing", "azimuthPixelSpacing", "sampledLineSpacing"],
            );
        }

        if let Some(json_path) = metadata_json_path.as_ref() {
            if let Ok(json_text) = fs::read_to_string(json_path) {
                if let Ok(json_value) = serde_json::from_str::<Value>(&json_text) {
                    if product_type.is_none() {
                        product_type = extract_first_json_text(
                            &json_value,
                            &["sar:product_type", "product_type", "productType"],
                        );
                    }
                    if acquisition_datetime_utc.is_none() {
                        acquisition_datetime_utc = extract_first_json_text(
                            &json_value,
                            &[
                                "datetime",
                                "start_datetime",
                                "acquisition_start_utc",
                                "acquisitionStartUTC",
                                "acquisition_time",
                            ],
                        );
                    }
                    if polarization.is_none() {
                        polarization = extract_first_json_text(
                            &json_value,
                            &["polarization", "polarisation"],
                        )
                        .map(|s| s.to_ascii_uppercase());
                    }
                    if polarization.is_none() {
                        polarization = extract_first_json_array_text(
                            &json_value,
                            &["sar:polarizations", "sar:polarisation"],
                        )
                        .map(|s| s.to_ascii_uppercase());
                    }
                    if acquisition_mode.is_none() {
                        acquisition_mode = extract_first_json_text(
                            &json_value,
                            &[
                                "sar:instrument_mode",
                                "acquisition_mode",
                                "acquisitionMode",
                                "imaging_mode",
                                "imagingMode",
                                "iceye:processing_mode",
                            ],
                        );
                    }
                    if orbit_direction.is_none() {
                        orbit_direction = extract_first_json_text(
                            &json_value,
                            &["sat:orbit_state", "orbit_direction", "orbitDirection"],
                        )
                        .map(|s| s.to_ascii_uppercase());
                    }
                    if look_direction.is_none() {
                        look_direction = extract_first_json_text(
                            &json_value,
                            &[
                                "sar:observation_direction",
                                "look_side",
                                "lookSide",
                                "lookDirection",
                                "antennaPointing",
                            ],
                        )
                        .map(|s| s.to_ascii_uppercase());
                    }
                    if incidence_angle_near_deg.is_none() {
                        incidence_angle_near_deg = extract_first_json_number(
                            &json_value,
                            &[
                                "iceye:incidence_angle_near",
                                "incidence_angle_near",
                                "incidenceAngleNear",
                                "incidenceAngleNearRange",
                            ],
                        );
                    }
                    if incidence_angle_far_deg.is_none() {
                        incidence_angle_far_deg = extract_first_json_number(
                            &json_value,
                            &[
                                "iceye:incidence_angle_far",
                                "incidence_angle_far",
                                "incidenceAngleFar",
                                "incidenceAngleFarRange",
                            ],
                        );
                    }
                    if pixel_spacing_range_m.is_none() {
                        pixel_spacing_range_m = extract_first_json_number(
                            &json_value,
                            &[
                                "sar:pixel_spacing_range",
                                "range_spacing",
                                "rangeSpacing",
                                "rangePixelSpacing",
                                "sampledPixelSpacing",
                            ],
                        );
                    }
                    if pixel_spacing_azimuth_m.is_none() {
                        pixel_spacing_azimuth_m = extract_first_json_number(
                            &json_value,
                            &[
                                "sar:pixel_spacing_azimuth",
                                "azimuth_spacing",
                                "azimuthSpacing",
                                "azimuthPixelSpacing",
                                "sampledLineSpacing",
                            ],
                        );
                    }
                }
            }
        }

        Ok(Self {
            bundle_root,
            metadata_xml_path,
            product_type,
            acquisition_datetime_utc,
            polarization,
            acquisition_mode,
            orbit_direction,
            look_direction,
            incidence_angle_near_deg,
            incidence_angle_far_deg,
            pixel_spacing_range_m,
            pixel_spacing_azimuth_m,
            assets,
        })
    }

    /// List canonical asset keys in this bundle.
    pub fn list_asset_keys(&self) -> Vec<String> {
        self.assets.keys().cloned().collect()
    }

    /// Resolve a canonical asset key to a raster file path.
    pub fn asset_path(&self, key: &str) -> Option<&Path> {
        self.assets
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical asset directly as a [`Raster`].
    pub fn read_asset(&self, key: &str) -> Result<Raster> {
        let p = self.asset_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("ICEYE asset '{}' not found", key))
        })?;
        Raster::read(p)
    }

    /// List unique polarization codes present in this bundle.
    ///
    /// Values are inferred from metadata and/or canonical asset keys.
    pub fn list_polarizations(&self) -> Vec<String> {
        let mut pols = Vec::new();
        if let Some(pol) = self.polarization.as_ref() {
            pols.push(pol.to_ascii_uppercase());
        }
        for key in self.assets.keys() {
            if let Some(pol) = extract_polarization_token(key) {
                pols.push(pol);
            }
        }
        pols.sort();
        pols.dedup();
        pols
    }

    /// Read all assets whose canonical key matches `pol` case-insensitively.
    pub fn read_assets_for_polarization(&self, pol: &str) -> Result<BTreeMap<String, Raster>> {
        let pol_upper = pol.to_ascii_uppercase();
        let keys: Vec<String> = self
            .list_asset_keys()
            .into_iter()
            .filter(|key| {
                let key_pol = extract_polarization_token(key).unwrap_or_default();
                key.eq_ignore_ascii_case(&pol_upper) || key_pol.eq_ignore_ascii_case(&pol_upper)
            })
            .collect();
        let entries: Vec<(String, Raster)> = keys
            .into_par_iter()
            .map(|key| Ok((key.clone(), self.read_asset(&key)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(entries.into_iter().collect())
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

fn has_xml_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("xml"))
        .unwrap_or(false)
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

fn canonical_asset_key(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_else(|| "ICEYE_ASSET".to_string());

    if let Some(pol) = extract_polarization_token(&stem) {
        return pol;
    }

    // Prefer the final token if the filename is heavily prefixed.
    let tokens: Vec<&str> = stem.split('_').collect();
    if let Some(last) = tokens.last() {
        if !last.is_empty() && last.chars().all(|c| c.is_ascii_alphanumeric()) {
            return (*last).to_string();
        }
    }

    stem
}

fn canonical_asset_key_with_collision_avoidance(
    existing: &BTreeMap<String, PathBuf>,
    path: &Path,
) -> String {
    let base = canonical_asset_key(path);
    if !existing.contains_key(&base) {
        return base;
    }

    // Preserve polarization token in the prefix while avoiding silent overwrite
    // when multiple same-pol assets are delivered (e.g. GRD + calibrated).
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_else(|| "ASSET".to_string());
    let mut candidate = format!("{base}_{stem}");
    if !existing.contains_key(&candidate) {
        return candidate;
    }

    let mut idx = 2usize;
    loop {
        candidate = format!("{base}_{idx}");
        if !existing.contains_key(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}

fn extract_polarization_token(stem: &str) -> Option<String> {
    let mut token = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
        } else if !token.is_empty() {
            if matches!(token.as_str(), "HH" | "HV" | "VH" | "VV") {
                return Some(token);
            }
            token.clear();
        }
    }
    if !token.is_empty() && matches!(token.as_str(), "HH" | "HV" | "VH" | "VV") {
        return Some(token);
    }
    None
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
        if let Some(text) = extract_tag_text(xml, tag) {
            if let Some(v) = parse_first_number(&text) {
                return Some(v);
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

fn extract_first_json_text(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = find_json_value_by_key(value, key) {
            if let Some(s) = json_value_to_text(v) {
                return Some(s);
            }
        }
    }
    None
}

fn extract_first_json_array_text(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(Value::Array(arr)) = find_json_value_by_key(value, key) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let t = s.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_first_json_number(value: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(v) = find_json_value_by_key(value, key) {
            if let Some(n) = json_value_to_number(v) {
                return Some(n);
            }
        }
    }
    None
}

fn find_json_value_by_key<'a>(value: &'a Value, wanted: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if k.eq_ignore_ascii_case(wanted) {
                    return Some(v);
                }
                if let Some(found) = find_json_value_by_key(v, wanted) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_json_value_by_key(item, wanted) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn json_value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn json_value_to_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => parse_first_number(s),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::test_helpers::assert_expected_csv_tokens_present;

    #[test]
    fn parses_minimal_iceye_bundle_structure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("ICEYE_TEST");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <product_type>GRD</product_type>
    <acquisition_mode>STRIPMAP</acquisition_mode>
  <acquisition_start_utc>2026-04-01T10:15:00.000000Z</acquisition_start_utc>
  <polarization>VV</polarization>
    <orbit_direction>DESCENDING</orbit_direction>
    <look_side>RIGHT</look_side>
    <incidence_angle_near>20.0</incidence_angle_near>
    <incidence_angle_far>36.0</incidence_angle_far>
    <range_spacing>2.5</range_spacing>
    <azimuth_spacing>2.5</azimuth_spacing>
</product>
"#;
        fs::write(root.join("metadata.xml"), xml).expect("write xml");
        fs::write(root.join("ICEYE_TEST_GRD_VV.tif"), b"").expect("write tiff");

        let bundle = IceyeBundle::open(&root).expect("open iceye bundle");
        assert_eq!(bundle.product_type.as_deref(), Some("GRD"));
        assert_eq!(
            bundle.acquisition_datetime_utc.as_deref(),
            Some("2026-04-01T10:15:00.000000Z")
        );
        assert_eq!(bundle.polarization.as_deref(), Some("VV"));
        assert_eq!(bundle.acquisition_mode.as_deref(), Some("STRIPMAP"));
        assert_eq!(bundle.orbit_direction.as_deref(), Some("DESCENDING"));
        assert_eq!(bundle.look_direction.as_deref(), Some("RIGHT"));
        assert_eq!(bundle.incidence_angle_near_deg, Some(20.0));
        assert_eq!(bundle.incidence_angle_far_deg, Some(36.0));
        assert_eq!(bundle.pixel_spacing_range_m, Some(2.5));
        assert_eq!(bundle.pixel_spacing_azimuth_m, Some(2.5));
        assert_eq!(bundle.assets.len(), 1);
        assert!(bundle.asset_path("VV").is_some());
        assert_eq!(bundle.list_polarizations(), vec!["VV"]);
    }

    #[test]
    fn canonical_asset_key_extracts_polarization_across_name_variants() {
        let p1 = Path::new("ICEYE_XYZ_GRD_VV.tif");
        let p2 = Path::new("iceye-xyz-grd-hv.tiff");
        let p3 = Path::new("scene__VH__calibrated.tif");
        let p4 = Path::new("scene_product_hh_001.tif");

        assert_eq!(canonical_asset_key(p1), "VV");
        assert_eq!(canonical_asset_key(p2), "HV");
        assert_eq!(canonical_asset_key(p3), "VH");
        assert_eq!(canonical_asset_key(p4), "HH");
    }

    #[test]
    fn preserves_multiple_assets_with_same_polarization_key() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("ICEYE_DUP_POL");
        fs::create_dir_all(&root).expect("create root");

        fs::write(root.join("metadata.xml"), "<product><polarization>VV</polarization></product>")
            .expect("write xml");
        fs::write(root.join("ICEYE_SCENE_GRD_VV.tif"), b"").expect("write vv 1");
        fs::write(root.join("ICEYE_SCENE_CAL_VV.tif"), b"").expect("write vv 2");

        let bundle = IceyeBundle::open(&root).expect("open iceye bundle");
        assert_eq!(bundle.assets.len(), 2);
        assert_eq!(bundle.list_polarizations(), vec!["VV"]);

        let vv_keys = bundle
            .list_asset_keys()
            .into_iter()
            .filter(|k| extract_polarization_token(k).as_deref() == Some("VV"))
            .count();
        assert_eq!(vv_keys, 2);
    }

    #[test]
    fn parses_numeric_values_with_units() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("ICEYE_UNITS");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <incidence_angle_near>20.0 deg</incidence_angle_near>
  <incidence_angle_far>36.0 deg</incidence_angle_far>
  <range_spacing>2.5 m</range_spacing>
  <azimuth_spacing>2.5 m</azimuth_spacing>
</product>
"#;
        fs::write(root.join("metadata.xml"), xml).expect("write xml");
        fs::write(root.join("ICEYE_TEST_GRD_VV.tif"), b"").expect("write tif");

        let bundle = IceyeBundle::open(&root).expect("open iceye bundle");
        assert_eq!(bundle.incidence_angle_near_deg, Some(20.0));
        assert_eq!(bundle.incidence_angle_far_deg, Some(36.0));
        assert_eq!(bundle.pixel_spacing_range_m, Some(2.5));
        assert_eq!(bundle.pixel_spacing_azimuth_m, Some(2.5));
    }

    #[test]
    fn opens_real_iceye_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_ICEYE_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let bundle = IceyeBundle::open(&root).expect("open real iceye sample");
        assert!(!bundle.list_asset_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_ICEYE_SAMPLE_EXPECT_KEYS",
            bundle.list_asset_keys(),
            "ICEYE canonical key",
        );
    }

    #[test]
    fn opens_real_iceye_open_data_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_ICEYE_OPEN_DATA_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let bundle = IceyeBundle::open(&root).expect("open real iceye open-data sample");
        assert!(!bundle.list_asset_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_ICEYE_OPEN_DATA_SAMPLE_EXPECT_KEYS",
            bundle.list_asset_keys(),
            "ICEYE open-data canonical key",
        );
    }

        #[test]
        fn parses_metadata_from_json_sidecar_when_xml_missing() {
                let tmp = tempfile::tempdir().expect("tempdir");
                let root = tmp.path().join("ICEYE_JSON_ONLY");
                fs::create_dir_all(&root).expect("create root");

                let json = r#"
{
    "datetime": "2025-06-27T11:24:12.843Z",
    "sar:product_type": "SLC-COG",
    "sar:instrument_mode": "spotlight",
    "sar:polarizations": ["VV"],
    "sat:orbit_state": "descending",
    "sar:observation_direction": "right",
    "sar:pixel_spacing_range": 0.19,
    "sar:pixel_spacing_azimuth": 0.09,
    "iceye:incidence_angle_near": 31.84,
    "iceye:incidence_angle_far": 32.05
}
"#;
                fs::write(root.join("metadata.json"), json).expect("write json");
                fs::write(root.join("ICEYE_TEST_SLC_VV.tif"), b"").expect("write tif");

                let bundle = IceyeBundle::open(&root).expect("open iceye bundle");
                assert_eq!(bundle.product_type.as_deref(), Some("SLC-COG"));
                assert_eq!(bundle.acquisition_datetime_utc.as_deref(), Some("2025-06-27T11:24:12.843Z"));
                assert_eq!(bundle.acquisition_mode.as_deref(), Some("spotlight"));
                assert_eq!(bundle.polarization.as_deref(), Some("VV"));
                assert_eq!(bundle.orbit_direction.as_deref(), Some("DESCENDING"));
                assert_eq!(bundle.look_direction.as_deref(), Some("RIGHT"));
                assert_eq!(bundle.pixel_spacing_range_m, Some(0.19));
                assert_eq!(bundle.pixel_spacing_azimuth_m, Some(0.09));
                assert_eq!(bundle.incidence_angle_near_deg, Some(31.84));
                assert_eq!(bundle.incidence_angle_far_deg, Some(32.05));
        }

        #[test]
        fn xml_values_take_precedence_over_json_fallback() {
                let tmp = tempfile::tempdir().expect("tempdir");
                let root = tmp.path().join("ICEYE_XML_OVER_JSON");
                fs::create_dir_all(&root).expect("create root");

                let xml = r#"
<product>
    <product_type>GRD</product_type>
    <acquisition_start_utc>2026-01-01T00:00:00.000Z</acquisition_start_utc>
    <polarization>VH</polarization>
    <acquisition_mode>STRIPMAP</acquisition_mode>
</product>
"#;
                let json = r#"
{
    "sar:product_type": "SLC-COG",
    "datetime": "2025-06-27T11:24:12.843Z",
    "sar:polarizations": ["VV"],
    "sar:instrument_mode": "spotlight"
}
"#;

                fs::write(root.join("metadata.xml"), xml).expect("write xml");
                fs::write(root.join("metadata.json"), json).expect("write json");
                fs::write(root.join("ICEYE_TEST_GRD_VH.tif"), b"").expect("write tif");

                let bundle = IceyeBundle::open(&root).expect("open iceye bundle");
                assert_eq!(bundle.product_type.as_deref(), Some("GRD"));
                assert_eq!(bundle.acquisition_datetime_utc.as_deref(), Some("2026-01-01T00:00:00.000Z"));
                assert_eq!(bundle.polarization.as_deref(), Some("VH"));
                assert_eq!(bundle.acquisition_mode.as_deref(), Some("STRIPMAP"));
        }
}
