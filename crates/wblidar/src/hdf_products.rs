//! HDF LiDAR product-family detection and dispatch.
//!
//! This module provides a lightweight provider registry similar in spirit to
//! `wbraster` sensor-bundle dispatch, but specialized for file-scoped HDF LiDAR
//! products (GEDI / ICESat-2) rather than directory package roots.

use std::path::Path;

use crate::hdf_adapter::{
    HdfAdapterResult,
    GEDI_L2B_CANOPY_STYLE_DATASET_PATH,
    ICESAT2_ATL08_CANOPY_SUBPATH,
    read_gedi_l2b_canopy_style_f32_window_in_file,
    read_icesat2_atl08_h_canopy_f32_window_in_file,
    resolve_icesat2_atl08_h_canopy_path_in_file,
};

/// Supported HDF LiDAR product families for the current integration slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdfLidarProductFamily {
    /// GEDI L2B-style canopy-height dataset path.
    GediL2bCanopy,
    /// ICESat-2 ATL08 canopy-height dataset path.
    Icesat2Atl08Canopy,
    /// No currently supported HDF LiDAR family was detected.
    Unknown,
}

/// Canonical resolved product path for a supported HDF LiDAR family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHdfLidarProduct {
    /// Product family selected by provider detection.
    pub family: HdfLidarProductFamily,
    /// Canonical dataset path used for targeted reads.
    pub dataset_path: String,
}

/// Runtime diagnostics counters for bounded HDF LiDAR canopy reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HdfLidarReadDiagnostics {
    /// Product family selected by dispatch.
    pub family: HdfLidarProductFamily,
    /// Number of chunk-like payload units visited during bounded read execution.
    pub chunks_visited: usize,
    /// Number of chunk-like payload units successfully decoded.
    pub chunks_decoded: usize,
    /// Count of filter decode failures.
    pub filter_failures: usize,
    /// Count of unsupported-layout failures.
    pub unsupported_layout_failures: usize,
    /// Count of invalid-chunk failures.
    pub invalid_chunk_failures: usize,
    /// Count of dataset-path or family-resolution failures.
    pub dataset_resolution_failures: usize,
}

impl Default for HdfLidarReadDiagnostics {
    fn default() -> Self {
        Self {
            family: HdfLidarProductFamily::Unknown,
            chunks_visited: 0,
            chunks_decoded: 0,
            filter_failures: 0,
            unsupported_layout_failures: 0,
            invalid_chunk_failures: 0,
            dataset_resolution_failures: 0,
        }
    }
}

/// Provider abstraction for HDF LiDAR family detection and path resolution.
pub trait HdfLidarProductProvider: Send + Sync {
    /// Product family resolved by this provider.
    fn family(&self) -> HdfLidarProductFamily;

    /// Returns `true` when this provider can resolve the given file.
    fn can_handle(&self, file_path: &Path) -> bool;

    /// Resolves canonical dataset path information for this product family.
    fn resolve(&self, file_path: &Path) -> HdfAdapterResult<ResolvedHdfLidarProduct>;
}

/// Ordered registry of HDF LiDAR product providers.
#[derive(Default)]
pub struct HdfLidarProductRegistry {
    providers: Vec<Box<dyn HdfLidarProductProvider>>,
}

impl HdfLidarProductRegistry {
    /// Creates an empty provider registry.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Creates a registry pre-populated with built-in providers.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(Icesat2Atl08CanopyProvider));
        registry.register(Box::new(GediL2bCanopyProvider));
        registry
    }

    /// Appends a provider to the probe order.
    pub fn register(&mut self, provider: Box<dyn HdfLidarProductProvider>) {
        self.providers.push(provider);
    }

    /// Resolves the first matching provider and returns canonical product details.
    pub fn resolve(&self, file_path: &Path) -> HdfAdapterResult<ResolvedHdfLidarProduct> {
        for provider in &self.providers {
            if provider.can_handle(file_path) {
                return provider.resolve(file_path);
            }
        }

        Err(wbhdf::WbhdfError::DatasetPathNotFound(format!(
            "no supported HDF LiDAR product family detected for '{}'",
            file_path.display()
        )))
    }
}

/// Built-in provider for GEDI L2B canopy-style reads.
#[derive(Debug, Default, Clone, Copy)]
pub struct GediL2bCanopyProvider;

impl HdfLidarProductProvider for GediL2bCanopyProvider {
    fn family(&self) -> HdfLidarProductFamily {
        HdfLidarProductFamily::GediL2bCanopy
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        wbhdf::dataset::resolve_dataset_in_file(file_path, GEDI_L2B_CANOPY_STYLE_DATASET_PATH).is_ok()
    }

    fn resolve(&self, file_path: &Path) -> HdfAdapterResult<ResolvedHdfLidarProduct> {
        wbhdf::dataset::resolve_dataset_in_file(file_path, GEDI_L2B_CANOPY_STYLE_DATASET_PATH)?;
        Ok(ResolvedHdfLidarProduct {
            family: self.family(),
            dataset_path: GEDI_L2B_CANOPY_STYLE_DATASET_PATH.to_string(),
        })
    }
}

/// Built-in provider for ICESat-2 ATL08 canopy-style reads.
#[derive(Debug, Default, Clone, Copy)]
pub struct Icesat2Atl08CanopyProvider;

impl HdfLidarProductProvider for Icesat2Atl08CanopyProvider {
    fn family(&self) -> HdfLidarProductFamily {
        HdfLidarProductFamily::Icesat2Atl08Canopy
    }

    fn can_handle(&self, file_path: &Path) -> bool {
        resolve_icesat2_atl08_h_canopy_path_in_file(file_path).is_ok()
    }

    fn resolve(&self, file_path: &Path) -> HdfAdapterResult<ResolvedHdfLidarProduct> {
        let dataset_path = resolve_icesat2_atl08_h_canopy_path_in_file(file_path)?;
        Ok(ResolvedHdfLidarProduct {
            family: self.family(),
            dataset_path,
        })
    }
}

/// Detects product family with the default provider registry.
pub fn detect_hdf_lidar_product_family(file_path: &Path) -> HdfLidarProductFamily {
    let registry = HdfLidarProductRegistry::with_defaults();
    match registry.resolve(file_path) {
        Ok(resolved) => resolved.family,
        Err(_) => HdfLidarProductFamily::Unknown,
    }
}

/// Resolves canonical product path with the default provider registry.
pub fn resolve_hdf_lidar_product(file_path: &Path) -> HdfAdapterResult<ResolvedHdfLidarProduct> {
    HdfLidarProductRegistry::with_defaults().resolve(file_path)
}

/// Reads a bounded canopy-height `f32` window using default product dispatch.
pub fn read_hdf_lidar_canopy_f32_window_in_file(
    file_path: &Path,
    start_value: usize,
    max_values: usize,
) -> HdfAdapterResult<Vec<f32>> {
    read_hdf_lidar_canopy_f32_window_with_diagnostics(file_path, start_value, max_values).0
}

/// Reads a bounded canopy-height `f32` window and returns runtime diagnostics counters.
pub fn read_hdf_lidar_canopy_f32_window_with_diagnostics(
    file_path: &Path,
    start_value: usize,
    max_values: usize,
) -> (HdfAdapterResult<Vec<f32>>, HdfLidarReadDiagnostics) {
    let mut diagnostics = HdfLidarReadDiagnostics::default();

    let resolved = match resolve_hdf_lidar_product(file_path) {
        Ok(resolved) => resolved,
        Err(err) => {
            apply_error_counters(&mut diagnostics, &err);
            return (Err(err), diagnostics);
        }
    };

    diagnostics.family = resolved.family;
    match resolved.family {
        HdfLidarProductFamily::GediL2bCanopy => {
            diagnostics.chunks_visited = 1;
            let result =
                read_gedi_l2b_canopy_style_f32_window_in_file(file_path, start_value, max_values);
            if result.is_ok() {
                diagnostics.chunks_decoded = 1;
            } else if let Err(err) = &result {
                apply_error_counters(&mut diagnostics, err);
            }
            (result, diagnostics)
        }
        HdfLidarProductFamily::Icesat2Atl08Canopy => {
            diagnostics.chunks_visited = 1;
            let result =
                read_icesat2_atl08_h_canopy_f32_window_in_file(file_path, start_value, max_values);
            if result.is_ok() {
                diagnostics.chunks_decoded = 1;
            } else if let Err(err) = &result {
                apply_error_counters(&mut diagnostics, err);
            }
            (result, diagnostics)
        }
        HdfLidarProductFamily::Unknown => {
            let err = wbhdf::WbhdfError::DatasetPathNotFound(
                "no supported canopy-family dispatch available".to_string(),
            );
            apply_error_counters(&mut diagnostics, &err);
            (Err(err), diagnostics)
        }
    }
}

/// Returns the current ATL08 canopy subpath for API-level discoverability.
pub fn icesat2_atl08_canopy_subpath() -> &'static str {
    ICESAT2_ATL08_CANOPY_SUBPATH
}

fn apply_error_counters(diagnostics: &mut HdfLidarReadDiagnostics, err: &wbhdf::WbhdfError) {
    match err {
        wbhdf::WbhdfError::FilterFailure { .. } | wbhdf::WbhdfError::UnsupportedFilter(_) => {
            diagnostics.filter_failures += 1;
        }
        wbhdf::WbhdfError::UnsupportedLayout(_) | wbhdf::WbhdfError::DatatypeMismatch { .. } => {
            diagnostics.unsupported_layout_failures += 1;
        }
        wbhdf::WbhdfError::InvalidChunk { .. } => {
            diagnostics.invalid_chunk_failures += 1;
        }
        wbhdf::WbhdfError::DatasetPathNotFound(_) => {
            diagnostics.dataset_resolution_failures += 1;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HdfLidarReadDiagnostics,
        HdfLidarProductFamily,
        detect_hdf_lidar_product_family,
        icesat2_atl08_canopy_subpath,
        read_hdf_lidar_canopy_f32_window_in_file,
        read_hdf_lidar_canopy_f32_window_with_diagnostics,
        resolve_hdf_lidar_product,
    };
    use std::fs;
    use std::path::PathBuf;

    fn unique_temp_file(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{prefix}-{}-{}.h5",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos()
        ));
        path
    }

    #[test]
    fn detects_gedi_family_from_canonical_marker() {
        let path = unique_temp_file("wblidar-gedi-detect");
        fs::write(&path, b"prefix /BEAM0000/elev_lowestmode suffix")
            .expect("temp GEDI marker file should be writable");

        let detected = detect_hdf_lidar_product_family(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(detected, HdfLidarProductFamily::GediL2bCanopy);
    }

    #[test]
    fn detects_atl08_family_from_component_markers() {
        let path = unique_temp_file("wblidar-atl08-detect");
        fs::write(
            &path,
            b"gt1l something land_segments something canopy something h_canopy",
        )
        .expect("temp ATL08 marker file should be writable");

        let detected = detect_hdf_lidar_product_family(&path);
        let resolved = resolve_hdf_lidar_product(&path).expect("ATL08 path should resolve");
        let _ = fs::remove_file(&path);

        assert_eq!(detected, HdfLidarProductFamily::Icesat2Atl08Canopy);
        assert_eq!(resolved.dataset_path, "/gt1l/land_segments/canopy/h_canopy");
    }

    #[test]
    fn unknown_family_is_reported_for_unmarked_files() {
        let path = unique_temp_file("wblidar-hdf-unknown");
        fs::write(&path, b"plain-binary-content").expect("temp file should be writable");

        let detected = detect_hdf_lidar_product_family(&path);
        let err = resolve_hdf_lidar_product(&path).expect_err("unmarked file should fail resolution");
        let _ = fs::remove_file(&path);

        assert_eq!(detected, HdfLidarProductFamily::Unknown);
        assert!(format!("{err}").contains("no supported HDF LiDAR product family detected"));
    }

    #[test]
    fn atl08_subpath_accessor_is_stable() {
        assert_eq!(icesat2_atl08_canopy_subpath(), "/land_segments/canopy/h_canopy");
    }

    #[test]
    fn unified_read_dispatch_surfaces_missing_product_error() {
        let path = unique_temp_file("wblidar-hdf-read-missing");
        fs::write(&path, b"not-a-supported-product").expect("temp file should be writable");

        let err = read_hdf_lidar_canopy_f32_window_in_file(&path, 0, 8)
            .expect_err("unsupported file should fail with deterministic resolution error");
        let _ = fs::remove_file(&path);

        assert!(format!("{err}").contains("no supported HDF LiDAR product family detected"));
    }

    #[test]
    fn diagnostics_counter_reports_resolution_failure_for_unsupported_file() {
        let path = unique_temp_file("wblidar-hdf-read-diag-missing");
        fs::write(&path, b"not-a-supported-product").expect("temp file should be writable");

        let (result, diagnostics) = read_hdf_lidar_canopy_f32_window_with_diagnostics(&path, 0, 8);
        let _ = fs::remove_file(&path);

        assert!(result.is_err(), "unsupported file should fail read dispatch");
        assert_eq!(
            diagnostics,
            HdfLidarReadDiagnostics {
                family: HdfLidarProductFamily::Unknown,
                chunks_visited: 0,
                chunks_decoded: 0,
                filter_failures: 0,
                unsupported_layout_failures: 0,
                invalid_chunk_failures: 0,
                dataset_resolution_failures: 1,
            }
        );
    }

    #[test]
    fn diagnostics_counter_reports_malformed_atl08_like_file_as_unsupported_layout() {
        let path = unique_temp_file("wblidar-hdf-read-diag-malformed-atl08");
        fs::write(
            &path,
            b"gt1l land_segments canopy h_canopy (truncated file without object headers)",
        )
        .expect("temp ATL08-like malformed file should be writable");

        let (result, diagnostics) = read_hdf_lidar_canopy_f32_window_with_diagnostics(&path, 0, 8);
        let _ = fs::remove_file(&path);

        assert!(result.is_err(), "malformed ATL08-like file should fail read dispatch");
        let err_text = format!(
            "{}",
            result.expect_err("error result should be present for malformed ATL08-like file")
        );
        assert!(
            err_text.contains("object-header discovery") || err_text.contains("v1 object headers"),
            "error should indicate object-header discovery failure"
        );

        assert_eq!(diagnostics.family, HdfLidarProductFamily::Icesat2Atl08Canopy);
        assert_eq!(diagnostics.chunks_visited, 1);
        assert_eq!(diagnostics.chunks_decoded, 0);
        assert_eq!(diagnostics.filter_failures, 0);
        assert_eq!(diagnostics.unsupported_layout_failures, 1);
        assert_eq!(diagnostics.invalid_chunk_failures, 0);
        assert_eq!(diagnostics.dataset_resolution_failures, 0);
    }
}
