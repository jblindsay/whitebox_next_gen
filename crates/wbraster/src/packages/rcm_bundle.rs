//! RADARSAT Constellation Mission (RCM) bundle reader.
//!
//! RCM deliveries commonly include GeoTIFF assets and XML metadata. This
//! module exposes package-level indexing and metadata extraction.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed RCM bundle.
#[derive(Debug, Clone)]
pub struct RcmBundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Metadata XML path when present.
    pub metadata_xml_path: Option<PathBuf>,
    /// Product type from metadata when present.
    pub product_type: Option<String>,
    /// Acquisition mode from metadata when present.
    pub acquisition_mode: Option<String>,
    /// Acquisition start time in UTC when present.
    pub acquisition_datetime_utc: Option<String>,
    /// Polarizations from metadata when present.
    pub polarizations: Vec<String>,
    /// Orbit direction when present (ASCENDING / DESCENDING).
    pub orbit_direction: Option<String>,
    /// Look direction when present (RIGHT / LEFT).
    pub look_direction: Option<String>,
    /// Incidence angle at near range in degrees when present.
    pub incidence_angle_near_deg: Option<f64>,
    /// Incidence angle at far range in degrees when present.
    pub incidence_angle_far_deg: Option<f64>,
    /// Ground-range sample spacing in metres when present.
    pub pixel_spacing_range_m: Option<f64>,
    /// Azimuth line spacing in metres when present.
    pub pixel_spacing_azimuth_m: Option<f64>,
    /// Canonical measurement key -> GeoTIFF path.
    pub measurements: BTreeMap<String, PathBuf>,
}

impl RcmBundle {
    /// Open and parse an RCM bundle directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "RCM bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;

        let mut metadata_xml_path = None;
        let mut measurements = BTreeMap::new();
        for p in files {
            if has_xml_ext(&p) {
                // Prefer product.xml if present, otherwise first XML candidate.
                let is_product = p
                    .file_name()
                    .map(|n| n.to_string_lossy().eq_ignore_ascii_case("product.xml"))
                    .unwrap_or(false);
                if is_product || metadata_xml_path.is_none() {
                    metadata_xml_path = Some(p);
                }
                continue;
            }
            if has_tiff_ext(&p) {
                let key = canonical_measurement_key_with_collision_avoidance(&measurements, &p);
                measurements.insert(key, p);
            }
        }

        if measurements.is_empty() {
            return Err(RasterError::MissingField(
                "no RCM GeoTIFF assets found in bundle".to_string(),
            ));
        }

        let mut product_type = None;
        let mut acquisition_mode = None;
        let mut acquisition_datetime_utc = None;
        let mut polarizations = Vec::new();
        let mut orbit_direction = None;
        let mut look_direction = None;
        let mut incidence_angle_near_deg = None;
        let mut incidence_angle_far_deg = None;
        let mut pixel_spacing_range_m = None;
        let mut pixel_spacing_azimuth_m = None;

        if let Some(xml_path) = metadata_xml_path.as_ref() {
            let xml = fs::read_to_string(xml_path)?;
            product_type = extract_first_text(&xml, &["productType"]);
            acquisition_mode = extract_first_text(&xml, &["beamModeMnemonic", "acquisitionType"]);
            acquisition_datetime_utc = extract_first_text(
                &xml,
                &["rawDataStartTime", "zeroDopplerTimeFirstLine", "startTime"],
            );
            if let Some(pol_text) = extract_first_text(&xml, &["polarizations", "polarization"]) {
                polarizations = pol_text
                    .split_whitespace()
                    .map(|s| s.to_ascii_uppercase())
                    .collect();
                polarizations.sort();
                polarizations.dedup();
            }
            orbit_direction = extract_first_text(&xml, &["passDirection", "orbitDirection"])
                .map(|s| s.to_ascii_uppercase());
            look_direction = extract_first_text(&xml, &["antennaPointing", "lookDirection"])
                .map(|s| s.to_ascii_uppercase());
            incidence_angle_near_deg = extract_first_number(
                &xml,
                &["incidenceAngleNearRange", "nearRangeIncidenceAngle"],
            );
            incidence_angle_far_deg = extract_first_number(
                &xml,
                &["incidenceAngleFarRange", "farRangeIncidenceAngle"],
            );
            pixel_spacing_range_m = extract_first_number(
                &xml,
                &["sampledPixelSpacing", "pixelSpacingRange", "rangePixelSpacing"],
            );
            pixel_spacing_azimuth_m = extract_first_number(
                &xml,
                &["sampledLineSpacing", "pixelSpacingAzimuth", "azimuthPixelSpacing"],
            );
        }

        // Fallback: infer available polarizations from measurement filenames.
        // This complements metadata parsing for minimal or non-standard product.xml variants.
        for key in measurements.keys() {
            if let Some(pol) = extract_polarization_token(key) {
                polarizations.push(pol);
            }
        }
        polarizations.sort();
        polarizations.dedup();

        Ok(Self {
            bundle_root,
            metadata_xml_path,
            product_type,
            acquisition_mode,
            acquisition_datetime_utc,
            polarizations,
            orbit_direction,
            look_direction,
            incidence_angle_near_deg,
            incidence_angle_far_deg,
            pixel_spacing_range_m,
            pixel_spacing_azimuth_m,
            measurements,
        })
    }

    /// List canonical measurement keys available in this bundle.
    pub fn list_measurement_keys(&self) -> Vec<String> {
        self.measurements.keys().cloned().collect()
    }

    /// Resolve a canonical measurement key to a raster file path.
    pub fn measurement_path(&self, key: &str) -> Option<&Path> {
        self.measurements
            .get(&key.to_ascii_uppercase())
            .map(PathBuf::as_path)
    }

    /// Read a canonical measurement directly as a [`Raster`].
    pub fn read_measurement(&self, key: &str) -> Result<Raster> {
        let p = self.measurement_path(key).ok_or_else(|| {
            RasterError::MissingField(format!("RCM measurement '{}' not found", key))
        })?;
        Raster::read(p)
    }

    /// Read all measurements matching a polarization code (e.g. `"VV"`, `"VH"`).
    pub fn read_measurements_for_polarization(
        &self,
        pol: &str,
    ) -> Result<BTreeMap<String, Raster>> {
        let pol_upper = pol.to_ascii_uppercase();
        let keys: Vec<String> = self
            .list_measurement_keys()
            .into_iter()
            .filter(|key| {
                let key_pol = extract_polarization_token(key).unwrap_or_default();
                key.eq_ignore_ascii_case(&pol_upper) || key_pol.eq_ignore_ascii_case(&pol_upper)
            })
            .collect();
        let entries: Vec<(String, Raster)> = keys
            .into_par_iter()
            .map(|key| Ok((key.clone(), self.read_measurement(&key)?)))
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

fn canonical_measurement_key(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_else(|| "MEASUREMENT".to_string());

    if let Some(pol) = extract_polarization_token(&stem) {
        return pol;
    }

    stem
}

fn canonical_measurement_key_with_collision_avoidance(
    existing: &BTreeMap<String, PathBuf>,
    path: &Path,
) -> String {
    let base = canonical_measurement_key(path);
    if !existing.contains_key(&base) {
        return base;
    }

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_else(|| "MEASUREMENT".to_string());
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
    fn parses_minimal_rcm_bundle_structure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RCM_TEST");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <productType>GRD</productType>
  <beamModeMnemonic>SC30</beamModeMnemonic>
  <rawDataStartTime>2026-04-01T12:30:00.000000Z</rawDataStartTime>
  <polarizations>VV VH</polarizations>
    <passDirection>DESCENDING</passDirection>
    <antennaPointing>RIGHT</antennaPointing>
    <incidenceAngleNearRange>24.5</incidenceAngleNearRange>
    <incidenceAngleFarRange>43.7</incidenceAngleFarRange>
    <sampledPixelSpacing>8.0</sampledPixelSpacing>
    <sampledLineSpacing>5.0</sampledLineSpacing>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("rcm_scene_VV.tif"), b"").expect("write vv");
        fs::write(root.join("rcm_scene_VH.tif"), b"").expect("write vh");

        let b = RcmBundle::open(&root).expect("open rcm");
        assert_eq!(b.product_type.as_deref(), Some("GRD"));
        assert_eq!(b.acquisition_mode.as_deref(), Some("SC30"));
        assert_eq!(b.acquisition_datetime_utc.as_deref(), Some("2026-04-01T12:30:00.000000Z"));
        assert_eq!(b.polarizations, vec!["VH", "VV"]);
        assert_eq!(b.orbit_direction.as_deref(), Some("DESCENDING"));
        assert_eq!(b.look_direction.as_deref(), Some("RIGHT"));
        assert_eq!(b.incidence_angle_near_deg, Some(24.5));
        assert_eq!(b.incidence_angle_far_deg, Some(43.7));
        assert_eq!(b.pixel_spacing_range_m, Some(8.0));
        assert_eq!(b.pixel_spacing_azimuth_m, Some(5.0));
        assert!(b.measurement_path("VV").is_some());
        assert!(b.measurement_path("VH").is_some());
    }

    #[test]
    fn canonical_measurement_key_extracts_polarization_across_name_variants() {
        assert_eq!(canonical_measurement_key(Path::new("rcm_scene_VV.tif")), "VV");
        assert_eq!(canonical_measurement_key(Path::new("rcm-scene-vh.tif")), "VH");
        assert_eq!(canonical_measurement_key(Path::new("RCM.HV.channel.tiff")), "HV");
        assert_eq!(canonical_measurement_key(Path::new("RCM__HH__SLC.tif")), "HH");
    }

    #[test]
    fn preserves_multiple_measurements_with_same_polarization_key() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RCM_DUP_POL");
        fs::create_dir_all(&root).expect("create root");

        fs::write(root.join("product.xml"), "<product><polarizations>VV</polarizations></product>")
            .expect("write xml");
        fs::write(root.join("rcm_scene_VV.tif"), b"").expect("write vv 1");
        fs::write(root.join("rcm_scene_cal_VV.tif"), b"").expect("write vv 2");

        let b = RcmBundle::open(&root).expect("open rcm");
        assert_eq!(b.measurements.len(), 2);

        let vv_keys = b
            .list_measurement_keys()
            .into_iter()
            .filter(|k| extract_polarization_token(k).as_deref() == Some("VV"))
            .count();
        assert_eq!(vv_keys, 2);
    }

    #[test]
    fn parses_numeric_values_with_units() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RCM_UNITS");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <incidenceAngleNearRange>24.5 deg</incidenceAngleNearRange>
  <incidenceAngleFarRange>43.7 deg</incidenceAngleFarRange>
  <sampledPixelSpacing>8.0 m</sampledPixelSpacing>
  <sampledLineSpacing>5.0 m</sampledLineSpacing>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("rcm_scene_VV.tif"), b"").expect("write vv");

        let b = RcmBundle::open(&root).expect("open rcm");
        assert_eq!(b.incidence_angle_near_deg, Some(24.5));
        assert_eq!(b.incidence_angle_far_deg, Some(43.7));
        assert_eq!(b.pixel_spacing_range_m, Some(8.0));
        assert_eq!(b.pixel_spacing_azimuth_m, Some(5.0));
    }

    #[test]
    fn opens_real_rcm_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_RCM_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let b = RcmBundle::open(&root).expect("open real rcm sample");
        assert!(!b.list_measurement_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_RCM_SAMPLE_EXPECT_KEYS",
            b.list_measurement_keys(),
            "RCM canonical key",
        );
    }

    #[test]
    fn infers_polarizations_from_measurement_keys_when_xml_omits_them() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RCM_INFER_POLS");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <productType>GRD</productType>
  <beamModeMnemonic>SC30</beamModeMnemonic>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("rcm_scene_VV.tif"), b"").expect("write vv");
        fs::write(root.join("rcm_scene_VH.tif"), b"").expect("write vh");

        let b = RcmBundle::open(&root).expect("open rcm");
        assert_eq!(b.polarizations, vec!["VH", "VV"]);
    }
}
