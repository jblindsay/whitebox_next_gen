use std::path::{Path, PathBuf};

pub const EXTERNAL_FIXTURE_DIR_ENV: &str = "WBHDF_FIXTURE_DIR";
pub const VIIRS_FIXTURE_DIR_ENV: &str = "WBHDF_VIIRS_FIXTURE_DIR";
pub const MODIS_FIXTURE_DIR_ENV: &str = "WBHDF_MODIS_FIXTURE_DIR";
pub const SMOKE_FIXTURE_FILE_ENV: &str = "WBHDF_SMOKE_FILE";

/// Returns the configured external fixture directory, if present.
pub fn external_fixture_dir() -> Option<PathBuf> {
    std::env::var_os(EXTERNAL_FIXTURE_DIR_ENV).map(PathBuf::from)
}

/// Returns the configured VIIRS fixture directory, if present.
pub fn external_viirs_fixture_dir() -> Option<PathBuf> {
    std::env::var_os(VIIRS_FIXTURE_DIR_ENV).map(PathBuf::from)
}

/// Returns the configured MODIS fixture directory, if present.
pub fn external_modis_fixture_dir() -> Option<PathBuf> {
    std::env::var_os(MODIS_FIXTURE_DIR_ENV).map(PathBuf::from)
}

/// Resolves a fixture path relative to `WBHDF_FIXTURE_DIR`.
pub fn resolve_external_fixture(relative_path: &str) -> Option<PathBuf> {
    external_fixture_dir().map(|root| root.join(relative_path))
}

/// Returns true if the fixture path exists and is a file.
pub fn fixture_is_available(path: &Path) -> bool {
    path.is_file()
}

/// Returns the optional smoke-test fixture path from env.
pub fn smoke_fixture_file() -> Option<PathBuf> {
    std::env::var_os(SMOKE_FIXTURE_FILE_ENV).map(PathBuf::from)
}
