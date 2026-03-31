//! # GeoTIFF
//!
//! A pure Rust library for reading and writing GeoTIFF files, including
//! BigTIFF (>4 GB), tiled layouts, and Cloud Optimized GeoTIFF (COG).
//!
//! ## Feature summary
//!
//! | Feature | API |
//! |---|---|
//! | Read stripped/tiled GeoTIFF | [`GeoTiff`] |
//! | Read BigTIFF (64-bit offsets) | [`GeoTiff`] (auto-detected) |
//! | Write stripped GeoTIFF | [`GeoTiffWriter`] |
//! | Write tiled GeoTIFF | [`GeoTiffWriter`] + [`WriteLayout::Tiled`] |
//! | Write BigTIFF | [`GeoTiffWriter::bigtiff`] |
//! | Write Cloud Optimized GeoTIFF | [`CogWriter`] |
//! | JPEG / LZW / Deflate / PackBits / None | [`Compression`] |
//!
//! ## Reading
//! ```rust,ignore
//! use wbraster::formats::geotiff_core::GeoTiff;
//! let tiff = GeoTiff::open("dem.tif").unwrap();
//! println!("{}×{} BigTIFF={}", tiff.width(), tiff.height(), tiff.is_bigtiff);
//! let data: Vec<f32> = tiff.read_band_f32(0).unwrap();
//! ```
//!
//! ## Writing (tiled + BigTIFF)
//! ```rust,ignore
//! use wbraster::formats::geotiff_core::{GeoTiffWriter, WriteLayout, Compression, SampleFormat, GeoTransform};
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
//! use wbraster::formats::geotiff_core::{CogWriter, Compression, GeoTransform};
//! CogWriter::new(4096, 4096, 1)
//!     .compression(Compression::Deflate)
//!     .tile_size(512)
//!     .geo_transform(GeoTransform::north_up(-180.0, 0.0879, 90.0, -0.0879))
//!     .epsg(4326)
//!     .write_f32("output.cog.tif", &vec![0.0f32; 4096 * 4096])
//!     .unwrap();
//! ```

#![warn(missing_docs)]
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
pub use reader::GeoTiff;
pub use tags::{PhotometricInterpretation, PlanarConfig, SampleFormat};
pub use types::GeoTransform;
pub use writer::{GeoTiffWriter, WriteLayout};
