use std::path::Path;

use wbraster::{
    detect_sensor_bundle_family,
    LandsatBundle,
    LandsatMission,
    SensorBundleFamily,
};

use crate::bundle::{ResolvedOpticalBundle, SensorBundleProvider};
use crate::error::{PhotogrammetryError, Result};

pub struct LandsatBundleProvider;

impl SensorBundleProvider for LandsatBundleProvider {
    fn sensor_name(&self) -> &'static str {
        "landsat"
    }

    fn can_handle(&self, bundle_root: &Path) -> bool {
        if !bundle_root.is_dir() {
            return false;
        }

        // Landsat bundles contain an MTL metadata file at root or nested.
        std::fs::read_dir(bundle_root)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_uppercase()))
                    .any(|name| name.ends_with("_MTL.TXT"))
            })
            .unwrap_or(false)
    }

    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        let family = detect_sensor_bundle_family(bundle_root).map_err(|e| {
            PhotogrammetryError::Bundle(format!(
                "failed detecting sensor bundle family for '{}': {e}",
                bundle_root.display()
            ))
        })?;
        if family != SensorBundleFamily::Landsat {
            return Err(PhotogrammetryError::Bundle(format!(
                "bundle '{}' is not a Landsat optical package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = LandsatBundle::open(bundle_root).map_err(|e| {
            PhotogrammetryError::Bundle(format!(
                "failed opening Landsat bundle '{}': {e}",
                bundle_root.display()
            ))
        })?;

        let (red_key, nir_key, green_key, blue_key) = mission_band_mapping(&package);
        let red_path = package
            .band_path(red_key)
            .map(|p| p.to_string_lossy().to_string());
        let nir_path = package
            .band_path(nir_key)
            .map(|p| p.to_string_lossy().to_string());

        if red_path.is_none() || nir_path.is_none() {
            return Err(PhotogrammetryError::Bundle(format!(
                "Landsat bundle '{}' does not contain required optical bands (expected red='{}', nir='{}')",
                bundle_root.display(),
                red_key,
                nir_key
            )));
        }

        let acquisition_datetime_utc = combine_landsat_datetime(
            package.acquisition_date_utc.as_deref(),
            package.scene_center_time_utc.as_deref(),
        );

        let mean_solar_zenith_deg = package
            .sun_elevation_deg
            .map(|e| (90.0 - e).clamp(0.0, 90.0));

        Ok(ResolvedOpticalBundle {
            sensor_name: self.sensor_name().to_string(),
            red_path,
            nir_path,
            green_path: package
                .band_path(green_key)
                .map(|p| p.to_string_lossy().to_string()),
            blue_path: package
                .band_path(blue_key)
                .map(|p| p.to_string_lossy().to_string()),
            qa_scl_path: None,
            qa_qa60_path: package
                .qa_path("QA_PIXEL")
                .or_else(|| package.qa_path("BQA"))
                .map(|p| p.to_string_lossy().to_string()),
            acquisition_datetime_utc,
            mean_solar_zenith_deg,
            mean_solar_azimuth_deg: package.sun_azimuth_deg,
        })
    }
}

fn mission_band_mapping(bundle: &LandsatBundle) -> (&'static str, &'static str, &'static str, &'static str) {
    // OLI/TIRS missions use red=B4 and NIR=B5. TM/ETM+ missions use red=B3 and NIR=B4.
    match bundle.mission {
        LandsatMission::Landsat8 | LandsatMission::Landsat9 => ("B4", "B5", "B3", "B2"),
        LandsatMission::Landsat4 | LandsatMission::Landsat5 | LandsatMission::Landsat7 => {
            ("B3", "B4", "B2", "B1")
        }
        LandsatMission::Unknown => {
            if bundle.band_path("B5").is_some() && bundle.band_path("B4").is_some() {
                ("B4", "B5", "B3", "B2")
            } else {
                ("B3", "B4", "B2", "B1")
            }
        }
    }
}

fn combine_landsat_datetime(date_utc: Option<&str>, scene_time_utc: Option<&str>) -> Option<String> {
    match (date_utc, scene_time_utc) {
        (Some(date), Some(time)) => {
            let t = time.trim();
            if t.contains('T') {
                Some(t.to_string())
            } else {
                Some(format!("{date}T{t}"))
            }
        }
        (Some(date), None) => Some(format!("{date}T00:00:00Z")),
        (None, Some(time)) => Some(time.trim().to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), ts))
    }

    fn write_landsat_fixture(root: &Path, mission: &str, band_stems: &[&str]) {
        fs::create_dir_all(root).expect("create fixture root");
        let mtl = format!(
            "SPACECRAFT_ID = \"{mission}\"\nDATE_ACQUIRED = 2024-02-02\nSCENE_CENTER_TIME = \"16:42:31.1234560Z\"\nSUN_AZIMUTH = 145.2\nSUN_ELEVATION = 38.6\n"
        );
        fs::write(root.join("SCENE_MTL.txt"), mtl).expect("write mtl");
        for stem in band_stems {
            fs::write(root.join(format!("{stem}.TIF")), b"").expect("write band");
        }
        fs::write(root.join("SCENE_QA_PIXEL.TIF"), b"").expect("write qa");
    }

    #[test]
    fn maps_landsat9_oli_band_set() {
        let root = unique_temp_dir("wbphotogrammetry_landsat9");
        write_landsat_fixture(
            &root,
            "LANDSAT_9",
            &["SCENE_SR_B2", "SCENE_SR_B3", "SCENE_SR_B4", "SCENE_SR_B5"],
        );

        let provider = LandsatBundleProvider;
        let resolved = provider
            .resolve_optical_bundle(&root)
            .expect("resolve landsat9 bundle");

        assert_eq!(resolved.sensor_name, "landsat");
        assert!(resolved.red_path.as_deref().unwrap_or_default().contains("_B4.TIF"));
        assert!(resolved.nir_path.as_deref().unwrap_or_default().contains("_B5.TIF"));
        assert!(resolved.green_path.as_deref().unwrap_or_default().contains("_B3.TIF"));
        assert!(resolved.blue_path.as_deref().unwrap_or_default().contains("_B2.TIF"));
        assert!(resolved.qa_qa60_path.as_deref().unwrap_or_default().contains("QA_PIXEL"));
        assert_eq!(resolved.mean_solar_azimuth_deg, Some(145.2));
        assert_eq!(resolved.mean_solar_zenith_deg, Some(51.4));
        assert_eq!(
            resolved.acquisition_datetime_utc.as_deref(),
            Some("2024-02-02T16:42:31.1234560Z")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn maps_landsat7_tm_style_band_set() {
        let root = unique_temp_dir("wbphotogrammetry_landsat7");
        write_landsat_fixture(
            &root,
            "LANDSAT_7",
            &["SCENE_SR_B1", "SCENE_SR_B2", "SCENE_SR_B3", "SCENE_SR_B4"],
        );

        let provider = LandsatBundleProvider;
        let resolved = provider
            .resolve_optical_bundle(&root)
            .expect("resolve landsat7 bundle");

        assert!(resolved.red_path.as_deref().unwrap_or_default().contains("_B3.TIF"));
        assert!(resolved.nir_path.as_deref().unwrap_or_default().contains("_B4.TIF"));
        assert!(resolved.green_path.as_deref().unwrap_or_default().contains("_B2.TIF"));
        assert!(resolved.blue_path.as_deref().unwrap_or_default().contains("_B1.TIF"));

        let _ = fs::remove_dir_all(&root);
    }
}
