//! # wbgeotiff
//!
//! `wbgeotiff` is the shared low-level GeoTIFF engine used by Whitebox Rust crates.
//! It provides pure-Rust read/write support for GeoTIFF, BigTIFF, and COG.
//!
//! ## Feature Summary
//!
//! | Feature | API |
//! |---|---|
//! | Read GeoTIFF/BigTIFF | [`GeoTiff`] |
//! | Write stripped/tiled GeoTIFF | [`GeoTiffWriter`] + [`WriteLayout`] |
//! | Write COG | [`CogWriter`] |
//! | Compression selection | [`Compression`] |
//! | Georeferencing | [`GeoTransform`] + GeoKeys |
//!
//! ## Reading
//! ```rust,ignore
//! use wbgeotiff::GeoTiff;
//! let tiff = GeoTiff::open("dem.tif").unwrap();
//! println!("{}x{} BigTIFF={}", tiff.width(), tiff.height(), tiff.is_bigtiff);
//! let data: Vec<f32> = tiff.read_band_f32(0).unwrap();
//! ```
//!
//! ## Writing (tiled + BigTIFF)
//! ```rust,ignore
//! use wbgeotiff::{Compression, GeoTiffWriter, GeoTransform, SampleFormat, WriteLayout};
//! GeoTiffWriter::new(8192, 8192, 1)
//!     .layout(WriteLayout::Tiled { tile_width: 512, tile_height: 512 })
//!     .compression(Compression::Deflate)
//!     .sample_format(SampleFormat::IeeeFloat)
//!     .bigtiff(true)
//!     .geo_transform(GeoTransform::north_up(0.0, 0.001, 8.192, -0.001))
//!     .epsg(4326)
//!     .write_f32("large.tif", &vec![0.0f32; 8192 * 8192])
//!     .unwrap();
//! ```
//!
//! ## Cloud Optimized GeoTIFF
//! ```rust,ignore
//! use wbgeotiff::{CogWriter, Compression, GeoTransform, Resampling};
//! CogWriter::new(4096, 4096, 1)
//!     .compression(Compression::Deflate)
//!     .tile_size(512)
//!     .resampling(Resampling::Average)
//!     .geo_transform(GeoTransform::north_up(-180.0, 0.0879, 90.0, -0.0879))
//!     .epsg(4326)
//!     .write_f32("output.cog.tif", &vec![0.0f32; 4096 * 4096])
//!     .unwrap();
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]
#![allow(unused_imports)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::cast_abs_to_unsigned)]

pub mod cog;
pub mod compression;
pub mod error;
pub mod geo_keys;
pub mod ifd;
pub mod reader;
pub mod tags;
pub mod types;
pub mod writer;

pub use cog::{CogWriter, Resampling};
pub use tags::Compression;
pub use error::{GeoTiffError, Result};
pub use geo_keys::{GeoKeyDirectory, ModelType, RasterType};
pub use ifd::TiffVariant;
pub use reader::{GeoTiff, ValueTransform};
pub use tags::{PhotometricInterpretation, PlanarConfig, SampleFormat};
pub use types::GeoTransform;
pub use writer::{GeoTiffWriter, WriteLayout};
