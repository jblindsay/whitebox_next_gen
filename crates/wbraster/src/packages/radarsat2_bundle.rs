//! RADARSAT-2 bundle reader.
//!
//! RADARSAT-2 deliveries typically include GeoTIFF assets and XML metadata
//! (`product.xml`). This module provides package-level indexing and metadata
//! extraction while delegating pixel decoding to existing raster readers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::error::{RasterError, Result};
use crate::raster::Raster;

/// Parsed RADARSAT-2 bundle.
#[derive(Debug, Clone)]
pub struct Radarsat2Bundle {
    /// Root bundle directory path.
    pub bundle_root: PathBuf,
    /// Product metadata XML path when present.
    pub product_xml_path: Option<PathBuf>,
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

impl Radarsat2Bundle {
    /// Open and parse a RADARSAT-2 bundle directory.
    pub fn open(bundle_root: impl AsRef<Path>) -> Result<Self> {
        let bundle_root = bundle_root.as_ref().to_path_buf();
        if !bundle_root.is_dir() {
            return Err(RasterError::Other(format!(
                "RADARSAT-2 bundle root is not a directory: {}",
                bundle_root.display()
            )));
        }

        let mut files = Vec::new();
        collect_files_recursive(&bundle_root, &mut files)?;

        let mut product_xml_path = None;
        let mut measurements = BTreeMap::new();
        for p in files {
            if is_product_xml(&p) {
                product_xml_path = Some(p);
                continue;
            }
            if has_tiff_ext(&p) {
                let key = canonical_measurement_key_with_collision_avoidance(&measurements, &p);
                measurements.insert(key, p);
            }
        }

        if measurements.is_empty() {
            return Err(RasterError::MissingField(
                "no RADARSAT-2 GeoTIFF assets found in bundle".to_string(),
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

        if let Some(xml_path) = product_xml_path.as_ref() {
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
            product_xml_path,
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
            RasterError::MissingField(format!(
                "RADARSAT-2 measurement '{}' not found",
                key
            ))
        })?;
        Raster::read(p)
    }

    /// Read all measurements matching a polarization code (e.g. `"HH"`, `"HV"`).
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

fn has_tiff_ext(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let ext = e.to_string_lossy();
            ext.eq_ignore_ascii_case("tif") || ext.eq_ignore_ascii_case("tiff")
        })
        .unwrap_or(false)
}

fn is_product_xml(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("product.xml"))
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
    fn parses_minimal_radarsat2_bundle_structure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RS2_TEST");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <productType>SLC</productType>
  <beamModeMnemonic>FQ</beamModeMnemonic>
  <rawDataStartTime>2026-04-01T11:00:00.000000Z</rawDataStartTime>
  <polarizations>HH HV</polarizations>
    <passDirection>ASCENDING</passDirection>
    <antennaPointing>RIGHT</antennaPointing>
    <incidenceAngleNearRange>20.1</incidenceAngleNearRange>
    <incidenceAngleFarRange>45.2</incidenceAngleFarRange>
    <sampledPixelSpacing>8.0</sampledPixelSpacing>
    <sampledLineSpacing>5.0</sampledLineSpacing>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("imagery_HH.tif"), b"").expect("write hh");
        fs::write(root.join("imagery_HV.tif"), b"").expect("write hv");

        let b = Radarsat2Bundle::open(&root).expect("open rs2");
        assert_eq!(b.product_type.as_deref(), Some("SLC"));
        assert_eq!(b.acquisition_mode.as_deref(), Some("FQ"));
        assert_eq!(b.acquisition_datetime_utc.as_deref(), Some("2026-04-01T11:00:00.000000Z"));
        assert_eq!(b.polarizations, vec!["HH", "HV"]);
        assert_eq!(b.orbit_direction.as_deref(), Some("ASCENDING"));
        assert_eq!(b.look_direction.as_deref(), Some("RIGHT"));
        assert_eq!(b.incidence_angle_near_deg, Some(20.1));
        assert_eq!(b.incidence_angle_far_deg, Some(45.2));
        assert_eq!(b.pixel_spacing_range_m, Some(8.0));
        assert_eq!(b.pixel_spacing_azimuth_m, Some(5.0));
        assert!(b.measurement_path("HH").is_some());
        assert!(b.measurement_path("HV").is_some());
    }

    #[test]
    fn canonical_measurement_key_extracts_polarization_across_name_variants() {
        assert_eq!(canonical_measurement_key(Path::new("imagery_HH.tif")), "HH");
        assert_eq!(canonical_measurement_key(Path::new("imagery-hv.tif")), "HV");
        assert_eq!(canonical_measurement_key(Path::new("rs2.vh.channel.tiff")), "VH");
        assert_eq!(canonical_measurement_key(Path::new("RS2__VV__SLC.tif")), "VV");
    }

    #[test]
    fn preserves_multiple_measurements_with_same_polarization_key() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RS2_DUP_POL");
        fs::create_dir_all(&root).expect("create root");

        fs::write(root.join("product.xml"), "<product><polarizations>HH</polarizations></product>")
            .expect("write xml");
        fs::write(root.join("imagery_HH.tif"), b"").expect("write hh 1");
        fs::write(root.join("imagery_cal_HH.tif"), b"").expect("write hh 2");

        let b = Radarsat2Bundle::open(&root).expect("open rs2");
        assert_eq!(b.measurements.len(), 2);

        let hh_keys = b
            .list_measurement_keys()
            .into_iter()
            .filter(|k| extract_polarization_token(k).as_deref() == Some("HH"))
            .count();
        assert_eq!(hh_keys, 2);
    }

    #[test]
    fn parses_numeric_values_with_units() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RS2_UNITS");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <incidenceAngleNearRange>20.1 deg</incidenceAngleNearRange>
  <incidenceAngleFarRange>45.2 deg</incidenceAngleFarRange>
  <sampledPixelSpacing>8.0 m</sampledPixelSpacing>
  <sampledLineSpacing>5.0 m</sampledLineSpacing>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("imagery_HH.tif"), b"").expect("write hh");

        let b = Radarsat2Bundle::open(&root).expect("open rs2");
        assert_eq!(b.incidence_angle_near_deg, Some(20.1));
        assert_eq!(b.incidence_angle_far_deg, Some(45.2));
        assert_eq!(b.pixel_spacing_range_m, Some(8.0));
        assert_eq!(b.pixel_spacing_azimuth_m, Some(5.0));
    }

    #[test]
    fn opens_real_radarsat2_sample_when_env_set() {
        let Ok(path) = std::env::var("WBRASTER_RADARSAT2_SAMPLE") else {
            return;
        };
        let root = PathBuf::from(path);
        if !root.is_dir() {
            return;
        }

        let b = Radarsat2Bundle::open(&root).expect("open real rs2 sample");
        assert!(!b.list_measurement_keys().is_empty());
        assert_expected_csv_tokens_present(
            "WBRASTER_RADARSAT2_SAMPLE_EXPECT_KEYS",
            b.list_measurement_keys(),
            "RADARSAT-2 canonical key",
        );
    }

    #[test]
    fn infers_polarizations_from_measurement_keys_when_xml_omits_them() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("RS2_INFER_POLS");
        fs::create_dir_all(&root).expect("create root");

        let xml = r#"
<product>
  <productType>SLC</productType>
  <beamModeMnemonic>FQ</beamModeMnemonic>
</product>
"#;
        fs::write(root.join("product.xml"), xml).expect("write xml");
        fs::write(root.join("imagery_HH.tif"), b"").expect("write hh");
        fs::write(root.join("imagery_HV.tif"), b"").expect("write hv");

        let b = Radarsat2Bundle::open(&root).expect("open rs2");
        assert_eq!(b.polarizations, vec!["HH", "HV"]);
    }
}
