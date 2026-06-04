/// Build configuration
/// When building for Python via maturin, this is handled automatically
/// For regular Rust builds, no special configuration needed

fn main() {
    // pyo3-build-config is only included when python feature is enabled
    #[cfg(feature = "python")]
    {
        // PyO3 automatically configures Python build settings
        pyo3_build_config::add_extension_module_link_args();
    }
}
