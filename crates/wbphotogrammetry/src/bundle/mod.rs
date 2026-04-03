//! Sensor-bundle resolution for optical remote-sensing workflows.
//!
//! The core types ([`ResolvedOpticalBundle`], [`SensorBundleProvider`],
//! [`SensorBundleRegistry`]) and all built-in provider implementations now live
//! in `wbraster::optical`, where they can be reused by any crate that needs
//! normalised sensor-band paths without depending on `wbphotogrammetry`.
//!
//! This module re-exports them for backwards-compatibility and convenience.

pub use wbraster::{
    DimapBundleProvider,
    LandsatBundleProvider,
    ResolvedOpticalBundle,
    SensorBundleProvider,
    SensorBundleRegistry,
    Sentinel2SafeBundleProvider,
};
