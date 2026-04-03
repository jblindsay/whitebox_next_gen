//! Package-level readers that resolve multi-file geospatial products into raster assets.

pub mod safe_bundle;
pub mod sensor_bundle;
pub mod dimap_bundle;
pub mod iceye_bundle;
pub mod landsat_bundle;
pub mod maxar_worldview_bundle;
pub mod planetscope_bundle;
pub mod radarsat2_bundle;
pub mod rcm_bundle;
pub mod sentinel1_safe;
pub mod sentinel2_safe;

#[cfg(test)]
pub(crate) mod test_helpers;
