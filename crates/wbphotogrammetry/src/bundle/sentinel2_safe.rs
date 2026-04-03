use std::path::Path;

use wbraster::{
    detect_sensor_bundle_family,
    Sentinel2SafePackage,
    SensorBundleFamily,
};

use crate::bundle::{ResolvedOpticalBundle, SensorBundleProvider};
use crate::error::{PhotogrammetryError, Result};

pub struct Sentinel2SafeBundleProvider;

impl SensorBundleProvider for Sentinel2SafeBundleProvider {
    fn sensor_name(&self) -> &'static str {
        "sentinel2_safe"
    }

    fn can_handle(&self, bundle_root: &Path) -> bool {
        if !bundle_root.is_dir() {
            return false;
        }

        // Sentinel-2 SAFE roots conventionally end with .SAFE.
        // Fallback to attempting package open in resolve_optical_bundle.
        bundle_root
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_uppercase().ends_with(".SAFE"))
            .unwrap_or(false)
    }

    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        let family = detect_sensor_bundle_family(bundle_root).map_err(|e| {
            PhotogrammetryError::Bundle(format!(
                "failed detecting sensor bundle family for '{}': {e}",
                bundle_root.display()
            ))
        })?;
        if family != SensorBundleFamily::Sentinel2Safe {
            return Err(PhotogrammetryError::Bundle(format!(
                "bundle '{}' is not a Sentinel-2 optical SAFE package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = Sentinel2SafePackage::open(bundle_root)
            .map_err(|e| PhotogrammetryError::Bundle(format!(
                "failed opening Sentinel-2 SAFE bundle '{}': {e}",
                bundle_root.display()
            )))?;

        Ok(ResolvedOpticalBundle {
            sensor_name: self.sensor_name().to_string(),
            red_path: package
                .band_path("B04")
                .map(|p| p.to_string_lossy().to_string()),
            nir_path: package
                .band_path("B08")
                .map(|p| p.to_string_lossy().to_string()),
            green_path: package
                .band_path("B03")
                .map(|p| p.to_string_lossy().to_string()),
            blue_path: package
                .band_path("B02")
                .map(|p| p.to_string_lossy().to_string()),
            qa_scl_path: package
                .qa_path("SCL")
                .map(|p| p.to_string_lossy().to_string()),
            qa_qa60_path: package
                .qa_path("QA60")
                .map(|p| p.to_string_lossy().to_string()),
            acquisition_datetime_utc: package.acquisition_datetime_utc.clone(),
            mean_solar_zenith_deg: package.mean_solar_zenith_deg,
            mean_solar_azimuth_deg: package.mean_solar_azimuth_deg,
        })
    }
}
