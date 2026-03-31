//! # wbprojection
//!
//! A map projection library for Rust, inspired by the PROJ library.
//!
//! ## Overview
//!
//! `wbprojection` provides forward and inverse transformations between geographic
//! coordinates (longitude/latitude) and projected coordinates (easting/northing)
//! for a wide range of map projections and datums.
//!
//! ## Quick Start
//!
//! ```rust
//! use wbprojection::{Projection, ProjectionParams, Ellipsoid};
//!
//! // Create a UTM Zone 32N projection
//! let params = ProjectionParams::utm(32, false);
//! let proj = Projection::new(params).unwrap();
//!
//! // Forward: lon/lat (degrees) → easting/northing (meters)
//! let (easting, northing) = proj.forward(9.0, 48.0).unwrap();
//! println!("Easting: {:.2}, Northing: {:.2}", easting, northing);
//!
//! // Inverse: easting/northing → lon/lat
//! let (lon, lat) = proj.inverse(easting, northing).unwrap();
//! println!("Lon: {:.6}, Lat: {:.6}", lon, lat);
//! ```
//!
//! ## Supported Projections
//!
//! - **Mercator** – Standard and Web Mercator (EPSG:3857)
//! - **Transverse Mercator** – Foundation for UTM
//! - **UTM** – Universal Transverse Mercator (all zones)
//! - **Lambert Conformal Conic** – One and two standard parallels
//! - **Albers Equal-Area Conic** – Equal-area conic
//! - **Azimuthal Equidistant** – Distances true from center point
//! - **Lambert Azimuthal Equal-Area** – Equal-area azimuthal
//! - **Krovak** – Oblique conformal conic (Czech/Slovak systems)
//! - **Central Conic** – Conic projection with one standard parallel
//! - **Lagrange** – Conformal spherical projection
//! - **Loximuthal** – Rhumb-line based pseudocylindrical projection
//! - **Euler** – Conic projection with two standard parallels
//! - **Tissot** – Conic projection with two standard parallels
//! - **Murdoch I** – Conic projection with two standard parallels
//! - **Murdoch II** – Conic projection with two standard parallels
//! - **Murdoch III** – Conic projection with two standard parallels
//! - **Perspective Conic** – Conic projection with two standard parallels
//! - **Vitkovsky I** – Conic projection with two standard parallels
//! - **Tobler-Mercator** – Equal-area cylindrical projection
//! - **Winkel II** – Compromise pseudocylindrical world projection
//! - **Kavrayskiy V** – Pseudocylindrical world projection
//! - **Stereographic** – Polar and oblique variants
//! - **Orthographic** – Globe-view projection
//! - **Sinusoidal** – Equal-area pseudocylindrical
//! - **Mollweide** – Equal-area pseudocylindrical
//! - **McBryde-Thomas Flat-Pole Sine (No. 2)** – Pseudocylindrical projection
//! - **McBryde-Thomas Flat-Polar Sine (No. 1)** – Pseudocylindrical projection
//! - **McBryde-Thomas Flat-Polar Parabolic** – Pseudocylindrical projection
//! - **McBryde-Thomas Flat-Polar Quartic** – Pseudocylindrical projection
//! - **Nell** – Pseudocylindrical projection
//! - **Equal Earth** – Equal-area compromise world projection
//! - **Cylindrical Equal-Area** – Equal-area cylindrical
//! - **Equirectangular** (Plate Carrée) – Simple cylindrical
//! - **Robinson** – Compromise pseudocylindrical
//! - **Gnomonic** – Great circles map to straight lines
//! - **Aitoff** – Compromise world projection
//! - **Van der Grinten** – Circular world projection
//! - **Winkel Tripel** – National Geographic world projection
//! - **Hammer** – Equal-area world projection
//! - **Hatano** – Asymmetrical equal-area pseudocylindrical projection
//! - **Eckert I** – Pseudocylindrical world projection
//! - **Eckert II** – Pseudocylindrical world projection
//! - **Eckert III** – Pseudocylindrical world projection
//! - **Eckert IV** – Equal-area pseudocylindrical world projection
//! - **Eckert V** – Pseudocylindrical world projection
//! - **Miller Cylindrical** – Modified Mercator world projection
//! - **Gall Stereographic** – Cylindrical stereographic world projection
//! - **Gall-Peters** – Equal-area cylindrical world projection
//! - **Behrmann** – Equal-area cylindrical projection (30° standard parallel)
//! - **Hobo-Dyer** – Equal-area cylindrical projection (37.5° standard parallel)
//! - **Wagner I** – Pseudocylindrical world projection
//! - **Wagner II** – Pseudocylindrical world projection
//! - **Wagner III** – Pseudocylindrical world projection
//! - **Wagner IV** – Equal-area pseudocylindrical world projection
//! - **Wagner V** – Equal-area pseudocylindrical world projection
//! - **Natural Earth** – Compromise pseudocylindrical world projection
//! - **Natural Earth II** – Compromise pseudocylindrical world projection
//! - **Wagner VI** – Compromise pseudocylindrical world projection
//! - **Eckert VI** – Equal-area pseudocylindrical world projection
//! - **Transverse Cylindrical Equal Area** – Spherical equal-area cylindrical projection
//! - **Polyconic** – American polyconic projection
//! - **Bonne** – Equal-area pseudoconical projection
//! - **Craster** – Craster Parabolic (Putnins P4) projection
//! - **Putnins P4'** – Pseudocylindrical compromise projection
//! - **Fahey** – Pseudocylindrical projection
//! - **Times** – Cylindrical compromise projection
//! - **Patterson** – Cylindrical compromise projection
//! - **Putnins P3** – Pseudocylindrical compromise projection
//! - **Putnins P3'** – Modified pseudocylindrical compromise projection
//! - **Putnins P5** – Pseudocylindrical compromise projection
//! - **Putnins P5'** – Modified pseudocylindrical compromise projection
//! - **Putnins P1** – Pseudocylindrical projection
//! - **Putnins P2** – Pseudocylindrical projection
//! - **Putnins P6** – Pseudocylindrical projection
//! - **Putnins P6'** – Pseudocylindrical projection
//! - **Quartic Authalic** – Equal-area pseudocylindrical projection
//! - **Foucaut** – Pseudocylindrical projection
//! - **Winkel I** – Compromise pseudocylindrical world projection
//! - **Werenskiold I** – Pseudocylindrical projection
//! - **Collignon** – Equal-area pseudocylindrical projection
//! - **Nell-Hammer** – Pseudocylindrical projection
//! - **Kavrayskiy VII** – Pseudocylindrical world projection
//!
//! ## Coordinate Reference Systems
//!
//! Use the [`Crs`] type to perform datum transformations between common
//! coordinate reference systems including WGS84, NAD83, NAD27, and ETRS89.
//! Grid-shift workflows are also supported via [`datum::DatumTransform::GridShift`]
//! with loaders in [`grid_formats`] and runtime registration in [`grid_shift`].
//!
//! ## 3D CRS workflows
//!
//! `wbprojection` now supports geocentric (ECEF XYZ) and minimal vertical CRS workflows.
//!
//! - Use [`Crs::transform_to_3d`] for strict 3D transforms.
//!   - Geographic/Projected <-> Geocentric is supported.
//!   - Vertical <-> Vertical is supported as passthrough.
//!   - Mixed Vertical <-> Geographic/Projected is rejected in strict mode.
//! - Use [`Crs::transform_to_3d_preserve_horizontal`] for explicit mixed-mode
//!   Vertical <-> Geographic/Projected workflows where horizontal context should be
//!   preserved unchanged.
//! - Use [`Crs::transform_to_3d_preserve_horizontal_with_vertical_offsets`] (or its
//!   policy-aware variant) when vertical offsets are available from external models.
//! - Use [`Crs::transform_to_3d_preserve_horizontal_with_provider`] when offsets
//!   should be resolved dynamically from coordinate/CRS context.
//! - Use [`ConstantVerticalOffsetProvider`] for simple fixed-offset workflows.
//! - Use [`GridVerticalOffsetProvider`] with registered [`VerticalOffsetGrid`]
//!   models for native bilinear-sampled vertical offsets.
//!
//! ```rust
//! use wbprojection::Crs;
//!
//! // Geographic <-> Geocentric
//! let geog = Crs::from_epsg(7843).unwrap();
//! let geoc = Crs::from_epsg(7842).unwrap();
//! let (x, y, z) = geog.transform_to_3d(147.0, -35.0, 120.0, &geoc).unwrap();
//! let (_lon, _lat, _h) = geoc.transform_to_3d(x, y, z, &geog).unwrap();
//!
//! // Explicit mixed vertical workflow (preserve horizontal context)
//! let vertical = Crs::from_epsg(7841).unwrap();
//! let utm = Crs::from_epsg(7846).unwrap();
//! let (_x2, _y2, _z2) = utm
//!     .transform_to_3d_preserve_horizontal(500_000.0, 6_120_000.0, 42.0, &vertical)
//!     .unwrap();
//! ```

#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

#[cfg(test)]
mod tests;
pub mod compound_crs;
pub mod crs;
pub mod datum;
pub mod ellipsoid;
pub mod epsg;
pub mod error;
pub mod grid_formats;
pub mod grid_shift;
pub mod projections;
pub mod transform;
pub mod vertical_grid;
mod wkt;

pub use compound_crs::CompoundCrs;
pub use crs::{
    ConstantVerticalOffsetProvider,
    Crs,
    CrsTransformPolicy,
    CrsTransformTrace,
    GridVerticalOffsetProvider,
    VerticalOffsetProvider,
};
pub use datum::Datum;
pub use ellipsoid::Ellipsoid;
pub use epsg::{
    EpsgAliasEntry,
    EpsgIdentifyCandidate,
    EpsgIdentifyPolicy,
    EpsgIdentifyReport,
    EpsgResolution,
    EpsgResolutionPolicy,
    clear_runtime_epsg_aliases,
    epsg_alias_catalog,
    epsg_from_srs_reference,
    epsg_from_wkt,
    compound_from_wkt,
    from_epsg,
    from_epsg_with_catalog,
    from_epsg_with_policy,
    identify_epsg_from_crs,
    identify_epsg_from_crs_report,
    identify_epsg_from_crs_with_policy,
    identify_epsg_from_wkt,
    identify_epsg_from_wkt_report,
    identify_epsg_from_wkt_with_policy,
    from_wkt,
    register_epsg_alias,
    resolve_epsg_with_catalog,
    resolve_epsg_with_policy,
    runtime_epsg_aliases,
    to_esri_wkt,
    to_geotiff_info,
    to_ogc_wkt,
    unregister_epsg_alias,
    vertical_offset_grid_name,
};
pub use error::{ProjectionError, Result};
pub use grid_formats::{
    list_ntv2_subgrids, load_nadcon_ascii_pair, load_ntv2_gsb, load_ntv2_gsb_subgrid,
    register_nadcon_ascii_pair, register_ntv2_gsb, register_ntv2_gsb_hierarchy,
    register_ntv2_gsb_subgrid, resolve_ntv2_hierarchy_grid_name,
    resolve_ntv2_hierarchy_subgrid,
};
pub use grid_shift::{GridShiftGrid, GridShiftSample, get_grid, has_grid, register_grid, unregister_grid};
pub use projections::{Projection, ProjectionKind, ProjectionParams};
pub use transform::{CoordTransform, Point2D, Point3D};
pub use vertical_grid::{
    VerticalOffsetGrid,
    get_vertical_offset_grid,
    has_vertical_offset_grid,
    load_vertical_grid_from_gtx,
    load_vertical_grid_from_isg,
    load_vertical_grid_from_simple_header_grid,
    register_vertical_offset_grid,
    unregister_vertical_offset_grid,
};

/// Convert degrees to radians.
#[inline]
pub fn to_radians(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// Convert radians to degrees.
#[inline]
pub fn to_degrees(rad: f64) -> f64 {
    rad * 180.0 / std::f64::consts::PI
}

/// Normalize a longitude value to the range [-180, 180).
pub fn normalize_longitude(lon: f64) -> f64 {
    let mut l = lon % 360.0;
    if l > 180.0 {
        l -= 360.0;
    } else if l < -180.0 {
        l += 360.0;
    }
    l
}
