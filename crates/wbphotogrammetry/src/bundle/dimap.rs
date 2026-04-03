use std::path::Path;

use wbraster::{
    detect_sensor_bundle_family,
    DimapBundle,
    SensorBundleFamily,
};

use crate::bundle::{ResolvedOpticalBundle, SensorBundleProvider};
use crate::error::{PhotogrammetryError, Result};

pub struct DimapBundleProvider;

impl SensorBundleProvider for DimapBundleProvider {
    fn sensor_name(&self) -> &'static str {
        "dimap"
    }

    fn can_handle(&self, bundle_root: &Path) -> bool {
        if !bundle_root.is_dir() {
            return false;
        }

        std::fs::read_dir(bundle_root)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_uppercase()))
                    .any(|name| name.starts_with("DIM_") && name.ends_with(".XML"))
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
        if family != SensorBundleFamily::Dimap {
            return Err(PhotogrammetryError::Bundle(format!(
                "bundle '{}' is not a DIMAP optical package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = DimapBundle::open(bundle_root).map_err(|e| {
            PhotogrammetryError::Bundle(format!(
                "failed opening DIMAP bundle '{}': {e}",
                bundle_root.display()
            ))
        })?;

        // DIMAP multispectral mapping is commonly: B1=blue, B2=green, B3=red, B4=nir.
        let red_path = package
            .band_path("B3")
            .map(|p| p.to_string_lossy().to_string());
        let nir_path = package
            .band_path("B4")
            .map(|p| p.to_string_lossy().to_string());

        if red_path.is_none() || nir_path.is_none() {
            return Err(PhotogrammetryError::Bundle(format!(
                "DIMAP bundle '{}' does not contain required multispectral bands (expected red='B3', nir='B4')",
                bundle_root.display()
            )));
        }

        let mean_solar_zenith_deg = package
            .sun_elevation_deg
            .map(|e| (90.0 - e).clamp(0.0, 90.0));

        Ok(ResolvedOpticalBundle {
            sensor_name: self.sensor_name().to_string(),
            red_path,
            nir_path,
            green_path: package
                .band_path("B2")
                .map(|p| p.to_string_lossy().to_string()),
            blue_path: package
                .band_path("B1")
                .or_else(|| package.band_path("B0"))
                .map(|p| p.to_string_lossy().to_string()),
            qa_scl_path: None,
            qa_qa60_path: None,
            acquisition_datetime_utc: package.acquisition_datetime_utc.clone(),
            mean_solar_zenith_deg,
            mean_solar_azimuth_deg: package.sun_azimuth_deg,
        })
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

    #[test]
    fn resolves_dimap_multispectral_core_bands() {
        let root = unique_temp_dir("wbphotogrammetry_dimap");
        fs::create_dir_all(&root).expect("create root");
        fs::write(
            root.join("DIM_SPOT7_MS_001.XML"),
            "<Dimap_Document><MISSION>SPOT</MISSION><IMAGING_DATE>2026-04-01</IMAGING_DATE><IMAGING_TIME>10:11:12.000</IMAGING_TIME><SUN_AZIMUTH>145.0</SUN_AZIMUTH><SUN_ELEVATION>41.2</SUN_ELEVATION></Dimap_Document>",
        )
        .expect("write xml");
        fs::write(root.join("IMG_XS1.JP2"), b"").expect("b1");
        fs::write(root.join("IMG_XS2.JP2"), b"").expect("b2");
        fs::write(root.join("IMG_XS3.JP2"), b"").expect("b3");
        fs::write(root.join("IMG_XS4.JP2"), b"").expect("b4");

        let provider = DimapBundleProvider;
        let resolved = provider
            .resolve_optical_bundle(&root)
            .expect("resolve dimap bundle");

        assert_eq!(resolved.sensor_name, "dimap");
        assert!(resolved.red_path.as_deref().unwrap_or_default().contains("XS3"));
        assert!(resolved.nir_path.as_deref().unwrap_or_default().contains("XS4"));
        assert!(resolved.green_path.as_deref().unwrap_or_default().contains("XS2"));
        assert!(resolved.blue_path.as_deref().unwrap_or_default().contains("XS1"));
        assert_eq!(resolved.mean_solar_azimuth_deg, Some(145.0));
        assert_eq!(resolved.mean_solar_zenith_deg, Some(48.8));

        let _ = fs::remove_dir_all(&root);
    }
}
