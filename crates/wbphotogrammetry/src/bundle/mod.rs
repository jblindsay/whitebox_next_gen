//! Sensor-bundle provider abstraction for optical remote-sensing workflows.

mod dimap;
mod landsat;
mod sentinel2_safe;

use std::path::Path;

use crate::error::{PhotogrammetryError, Result};

pub use dimap::DimapBundleProvider;
pub use landsat::LandsatBundleProvider;
pub use sentinel2_safe::Sentinel2SafeBundleProvider;

#[derive(Debug, Clone, Default)]
pub struct ResolvedOpticalBundle {
    pub sensor_name: String,
    pub red_path: Option<String>,
    pub nir_path: Option<String>,
    pub green_path: Option<String>,
    pub blue_path: Option<String>,
    pub qa_scl_path: Option<String>,
    pub qa_qa60_path: Option<String>,
    pub acquisition_datetime_utc: Option<String>,
    pub mean_solar_zenith_deg: Option<f64>,
    pub mean_solar_azimuth_deg: Option<f64>,
}

pub trait SensorBundleProvider: Send + Sync {
    fn sensor_name(&self) -> &'static str;
    fn can_handle(&self, bundle_root: &Path) -> bool;
    fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle>;
}

#[derive(Default)]
pub struct SensorBundleRegistry {
    providers: Vec<Box<dyn SensorBundleProvider>>,
}

impl SensorBundleRegistry {
    pub fn new() -> Self {
        Self { providers: Vec::new() }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(Sentinel2SafeBundleProvider));
        registry.register(Box::new(LandsatBundleProvider));
        registry.register(Box::new(DimapBundleProvider));
        registry
    }

    pub fn register(&mut self, provider: Box<dyn SensorBundleProvider>) {
        self.providers.push(provider);
    }

    pub fn resolve_optical_bundle(&self, bundle_root: &Path) -> Result<ResolvedOpticalBundle> {
        for provider in &self.providers {
            if provider.can_handle(bundle_root) {
                return provider.resolve_optical_bundle(bundle_root);
            }
        }

        Err(PhotogrammetryError::Bundle(format!(
            "unsupported sensor bundle root: {}",
            bundle_root.display()
        )))
    }
}
