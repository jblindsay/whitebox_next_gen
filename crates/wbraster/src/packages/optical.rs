//! Optical sensor-bundle provider abstraction.
//!
//! Provides a common trait and registry for resolving raw sensor packages (DIMAP, Landsat,
//! Sentinel-2 SAFE, …) into a canonical [`ResolvedOpticalBundle`] that upstream crates can
//! consume without depending on individual sensor-format internals.

use std::path::Path;

use crate::error::{RasterError, Result};
use super::dimap_bundle::DimapBundle;
use super::landsat_bundle::{LandsatBundle, LandsatMission};
use super::sensor_bundle::{detect_sensor_bundle_family, SensorBundleFamily};
use super::sentinel2_safe::Sentinel2SafePackage;

// ---------------------------------------------------------------------------
// Canonical output type
// ---------------------------------------------------------------------------

/// Normalised view of an optical remote-sensing scene bundle.
///
/// Paths are stored as `String` so callers do not need to hold a reference to the
/// originating bundle. All fields are optional to accommodate sensors that omit
/// particular bands or metadata.
#[derive(Debug, Clone, Default)]
pub struct ResolvedOpticalBundle {
    /// Short sensor family identifier (e.g. `"dimap"`, `"landsat"`, `"sentinel2_safe"`).
    pub sensor_name: String,
    /// Absolute path to the red band file, if present.
    pub red_path: Option<String>,
    /// Absolute path to the NIR band file, if present.
    pub nir_path: Option<String>,
    /// Absolute path to the green band file, if present.
    pub green_path: Option<String>,
    /// Absolute path to the blue band file, if present.
    pub blue_path: Option<String>,
    /// Absolute path to the Scene Classification Layer (SCL) QA band, if present.
    pub qa_scl_path: Option<String>,
    /// Absolute path to the QA60 / QA_PIXEL mask band, if present.
    pub qa_qa60_path: Option<String>,
    /// Scene acquisition timestamp in UTC, ISO-8601 format where available.
    pub acquisition_datetime_utc: Option<String>,
    /// Mean solar zenith angle in degrees above ground (derived from sun elevation).
    pub mean_solar_zenith_deg: Option<f64>,
    /// Mean solar azimuth angle in degrees, clockwise from north.
    pub mean_solar_azimuth_deg: Option<f64>,
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

/// Resolves a sensor bundle directory into a [`ResolvedOpticalBundle`].
///
/// Implementations are registered with a [`SensorBundleRegistry`] so callers
/// can probe an arbitrary directory without knowing its sensor family in advance.
pub trait SensorBundleProvider: Send + Sync {
    /// Short identifier for this sensor family (e.g. `"dimap"`).
    fn sensor_name(&self) -> &'static str;

    /// Returns `true` when `bundle_root` looks like a bundle this provider can open.
    fn can_handle(&self, bundle_root: &Path) -> bool;

    /// Resolves the bundle and returns the canonical optical description.
    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle>;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Ordered collection of [`SensorBundleProvider`]s.
///
/// Call [`SensorBundleRegistry::with_defaults`] to get a registry pre-populated
/// with DIMAP, Landsat, and Sentinel-2 providers.
#[derive(Default)]
pub struct SensorBundleRegistry {
    providers: Vec<Box<dyn SensorBundleProvider>>,
}

impl SensorBundleRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self { providers: Vec::new() }
    }

    /// Creates a registry pre-populated with the built-in optical providers.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(Sentinel2SafeBundleProvider));
        registry.register(Box::new(LandsatBundleProvider));
        registry.register(Box::new(DimapBundleProvider));
        registry
    }

    /// Appends a provider to the end of the probe list.
    pub fn register(&mut self, provider: Box<dyn SensorBundleProvider>) {
        self.providers.push(provider);
    }

    /// Probes `bundle_root` against all registered providers and returns the
    /// first successful resolution, or an error if no provider recognises it.
    pub fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        for provider in &self.providers {
            if provider.can_handle(bundle_root) {
                return provider.resolve_optical_bundle(bundle_root);
            }
        }
        Err(RasterError::Other(format!(
            "unsupported sensor bundle root: {}",
            bundle_root.display()
        )))
    }
}

// ---------------------------------------------------------------------------
// DIMAP provider (SPOT / Pléiades)
// ---------------------------------------------------------------------------

/// [`SensorBundleProvider`] for SPOT/Pléiades DIMAP optical packages.
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
            RasterError::Other(format!(
                "failed detecting sensor bundle family for '{}': {e}",
                bundle_root.display()
            ))
        })?;
        if family != SensorBundleFamily::Dimap {
            return Err(RasterError::Other(format!(
                "bundle '{}' is not a DIMAP optical package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = DimapBundle::open(bundle_root).map_err(|e| {
            RasterError::Other(format!(
                "failed opening DIMAP bundle '{}': {e}",
                bundle_root.display()
            ))
        })?;

        // DIMAP multispectral order: B0/B1=blue, B2=green, B3=red, B4=nir.
        let red_path = package
            .band_path("B3")
            .map(|p| p.to_string_lossy().to_string());
        let nir_path = package
            .band_path("B4")
            .map(|p| p.to_string_lossy().to_string());

        if red_path.is_none() || nir_path.is_none() {
            return Err(RasterError::Other(format!(
                "DIMAP bundle '{}' does not contain required multispectral bands \
                 (expected red='B3', nir='B4')",
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

// ---------------------------------------------------------------------------
// Landsat provider (Landsat 4-9)
// ---------------------------------------------------------------------------

/// [`SensorBundleProvider`] for Landsat Collection scene bundles (Landsat 4–9).
pub struct LandsatBundleProvider;

impl SensorBundleProvider for LandsatBundleProvider {
    fn sensor_name(&self) -> &'static str {
        "landsat"
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
                    .any(|name| name.ends_with("_MTL.TXT"))
            })
            .unwrap_or(false)
    }

    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        let family = detect_sensor_bundle_family(bundle_root).map_err(|e| {
            RasterError::Other(format!(
                "failed detecting sensor bundle family for '{}': {e}",
                bundle_root.display()
            ))
        })?;
        if family != SensorBundleFamily::Landsat {
            return Err(RasterError::Other(format!(
                "bundle '{}' is not a Landsat optical package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = LandsatBundle::open(bundle_root).map_err(|e| {
            RasterError::Other(format!(
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
            return Err(RasterError::Other(format!(
                "Landsat bundle '{}' does not contain required optical bands \
                 (expected red='{}', nir='{}')",
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

/// Band-key mapping for Landsat OLI (8/9) vs TM/ETM+ (4/5/7).
fn mission_band_mapping(bundle: &LandsatBundle) -> (&'static str, &'static str, &'static str, &'static str) {
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

// ---------------------------------------------------------------------------
// Sentinel-2 SAFE provider
// ---------------------------------------------------------------------------

/// [`SensorBundleProvider`] for Sentinel-2 Level-1C / Level-2A SAFE packages.
pub struct Sentinel2SafeBundleProvider;

impl SensorBundleProvider for Sentinel2SafeBundleProvider {
    fn sensor_name(&self) -> &'static str {
        "sentinel2_safe"
    }

    fn can_handle(&self, bundle_root: &Path) -> bool {
        if !bundle_root.is_dir() {
            return false;
        }
        bundle_root
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_ascii_uppercase().ends_with(".SAFE"))
            .unwrap_or(false)
    }

    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        let family = detect_sensor_bundle_family(bundle_root).map_err(|e| {
            RasterError::Other(format!(
                "failed detecting sensor bundle family for '{}': {e}",
                bundle_root.display()
            ))
        })?;
        if family != SensorBundleFamily::Sentinel2Safe {
            return Err(RasterError::Other(format!(
                "bundle '{}' is not a Sentinel-2 optical SAFE package (detected: {:?})",
                bundle_root.display(),
                family
            )));
        }

        let package = Sentinel2SafePackage::open(bundle_root).map_err(|e| {
            RasterError::Other(format!(
                "failed opening Sentinel-2 SAFE bundle '{}': {e}",
                bundle_root.display()
            ))
        })?;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    // --- DIMAP ---

    #[test]
    fn dimap_resolves_multispectral_core_bands() {
        let root = unique_temp_dir("wbraster_dimap");
        fs::create_dir_all(&root).expect("create root");
        fs::write(
            root.join("DIM_SPOT7_MS_001.XML"),
            "<Dimap_Document><MISSION>SPOT</MISSION>\
             <IMAGING_DATE>2026-04-01</IMAGING_DATE>\
             <IMAGING_TIME>10:11:12.000</IMAGING_TIME>\
             <SUN_AZIMUTH>145.0</SUN_AZIMUTH>\
             <SUN_ELEVATION>41.2</SUN_ELEVATION></Dimap_Document>",
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

    // --- Landsat ---

    fn write_landsat_fixture(root: &Path, mission: &str, band_stems: &[&str]) {
        fs::create_dir_all(root).expect("create fixture root");
        let mtl = format!(
            "SPACECRAFT_ID = \"{mission}\"\n\
             DATE_ACQUIRED = 2024-02-02\n\
             SCENE_CENTER_TIME = \"16:42:31.1234560Z\"\n\
             SUN_AZIMUTH = 145.2\n\
             SUN_ELEVATION = 38.6\n"
        );
        fs::write(root.join("SCENE_MTL.txt"), mtl).expect("write mtl");
        for stem in band_stems {
            fs::write(root.join(format!("{stem}.TIF")), b"").expect("write band");
        }
        fs::write(root.join("SCENE_QA_PIXEL.TIF"), b"").expect("write qa");
    }

    #[test]
    fn landsat_maps_oli_band_set() {
        let root = unique_temp_dir("wbraster_landsat9");
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
    fn landsat_maps_tm_style_band_set() {
        let root = unique_temp_dir("wbraster_landsat7");
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
