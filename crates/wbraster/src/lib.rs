//! # wbraster
//!
//! A pure-Rust library for reading and writing common raster GIS formats.
//!
//! ## Supported Formats
//!
//! | Format | Read | Write | Extension(s) |
//! |---|---|---|---|
//! | Esri ASCII Grid | ✓ | ✓ | `.asc`, `.grd` |
//! | Esri Binary Grid | ✓ | ✓ | `.adf` (workspace) |
//! | GRASS ASCII Raster | ✓ | ✓ | `.asc`, `.txt` |
//! | Surfer GRD | ✓ | ✓ | `.grd` |
//! | PCRaster | ✓ | ✓ | `.map` |
//! | SAGA Binary Grid | ✓ | ✓ | `.sdat` / `.sgrd` |
//! | Idrisi/TerrSet Raster | ✓ | ✓ | `.rst` / `.rdc` |
//! | ER Mapper | ✓ | ✓ | `.ers` / `.ers` data |
//! | ENVI HDR Labelled Raster | ✓ | ✓ | `.hdr` + `.img/.dat/.bin/.raw/.bil/.bsq/.bip` |
//! | GeoTIFF / BigTIFF / COG | ✓ | ✓ | `.tif` / `.tiff` |
//! | GeoPackage Raster (Phase 4) | ✓ | ✓ | `.gpkg` |
//! | JPEG 2000 / GeoJP2 | ✓ | ✓ | `.jp2` |
//! | Zarr v2/v3 (MVP) | ✓ | ✓ | `.zarr` |
//!
//! ## Quick Start
//!
//! ```rust
//! use wbraster::{Raster, RasterFormat, RasterConfig};
//!
//! // Create a new raster in memory
//! let mut r = Raster::new(RasterConfig {
//!     cols: 100,
//!     rows: 100,
//!     bands: 1,
//!     x_min: 0.0,
//!     y_min: 0.0,
//!     cell_size: 1.0,
//!     nodata: -9999.0,
//!     ..Default::default()
//! });
//!
//! // Set some values
//! r.set(0, 50isize, 50isize, 42.0).unwrap();
//! assert_eq!(r.get(0, 50isize, 50isize), 42.0);
//!
//! // Write to disk
//! r.write("output.asc", RasterFormat::EsriAscii).unwrap();
//!
//! // Read back
//! let r2 = Raster::read("output.asc").unwrap();
//! assert_eq!(r2.get(0, 50isize, 50isize), 42.0);
//! ```
//!
//! ## COG-first write API
//!
//! ```rust,no_run
//! use wbraster::{CogWriteOptions, GeoTiffCompression, Raster};
//!
//! let raster = Raster::read("input.tif").unwrap();
//!
//! // Fast convenience defaults (deflate, tile=512, bigtiff=false)
//! raster.write_cog("output_default.cog.tif").unwrap();
//!
//! // Convenience defaults with custom tile size
//! raster
//!     .write_cog_with_tile_size("output_tile256.cog.tif", 256)
//!     .unwrap();
//!
//! // COG-focused typed options (compression + bigtiff + tile size)
//! let opts = CogWriteOptions {
//!     compression: Some(GeoTiffCompression::Deflate),
//!     bigtiff: Some(false),
//!     tile_size: Some(256),
//! };
//! raster
//!     .write_cog_with_options("output_opts.cog.tif", &opts)
//!     .unwrap();
//! ```
//!
//! ## JPEG2000 default lossy quality
//!
//! ```rust,no_run
//! use wbraster::{
//!     Jpeg2000Compression,
//!     Jpeg2000WriteOptions,
//!     JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
//!     Raster,
//!     RasterFormat,
//! };
//!
//! let raster = Raster::read("input.tif").unwrap();
//! let opts = Jpeg2000WriteOptions {
//!     compression: Some(Jpeg2000Compression::Lossy {
//!         quality_db: JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
//!     }),
//!     decomp_levels: Some(5),
//!     color_space: None,
//! };
//! raster.write_jpeg2000_with_options("output.jp2", &opts).unwrap();
//! raster.write("output_default.jp2", RasterFormat::Jpeg2000).unwrap();
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]

pub mod color_math;
pub mod error;
/// In-process raster memory store for passing rasters between tools without disk I/O.
pub mod memory_store;
pub mod raster;
pub mod formats;
pub mod io_utils;
pub mod crs_info;

pub use error::{RasterError, Result};
pub use raster::{
	Raster,
	RasterConfig,
	DataType,
	NoData,
	Statistics,
	StatisticsComputationMode,
	Extent,
	ResampleMethod,
	NodataPolicy,
	AntimeridianPolicy,
	GridSizePolicy,
	DestinationFootprint,
	ReprojectOptions,
};
pub use formats::RasterFormat;
pub use formats::geotiff::{
	CogWriteOptions,
	GeoTiffCompression,
	GeoTiffLayout,
	GeoTiffWriteOptions,
};
pub use formats::jpeg2000::{
	Jpeg2000Compression,
	Jpeg2000WriteOptions,
	JPEG2000_DEFAULT_LOSSY_QUALITY_DB,
};
pub use color_math::{hsi2value, hsi_to_rgb_norm, rgb_to_hsi_norm, value2hsi, value2i};
pub use crs_info::CrsInfo;
