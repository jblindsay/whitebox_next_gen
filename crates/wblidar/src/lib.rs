//! # wblidar
//!
//! A high-performance, pure-Rust library for reading and writing LiDAR point-cloud
//! data in the most common industry formats:
//!
//! | Format | Read | Write | Notes |
//! |--------|------|-------|-------|
//! | LAS    | ✓    | ✓     | versions 1.1 – 1.5; writes 1.4 R15 by default (v1.5 PDRF 11–15 also supported) |
//! | LAZ    | ✓    | ✓     | standards-compliant LASzip v2/v3; in-house implementation, no LASzip C dependency |
//! | COPC   | ✓    | ✓     | Cloud-Optimised Point Cloud (LAS 1.4 + EPT hierarchy) |
//! | PLY    | ✓    | ✓     | ASCII & binary little/big-endian |
//! | E57    | ✓    | ✓     | ASTM E2807; CRC-32 validation, optional zlib blobs |
//!
//! ## Design goals
//! * **Minimal allocations** – point records are streamed through a fixed-size
//!   `PointRecord` type; large arrays are written in a single pass.
//! * **Minimal external dependencies** – `flate2` (zlib-ng backend) for
//!   DEFLATE streams in LAZ/E57 and `wbprojection` for CRS transforms.
//! * **Reprojection helpers** – use `reproject::points_to_epsg` (source CRS
//!   metadata) or `reproject::points_from_to_epsg` (explicit EPSG codes).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod copc;
pub mod crs;
pub mod e57;
pub mod error;
pub mod frontend;
/// Minimal HDF adapter interfaces for incremental `wblidar` -> `wbhdf` integration.
pub mod hdf_adapter;
/// HDF LiDAR product-family detection and unified dispatch helpers.
pub mod hdf_products;
pub mod io;
pub mod las;
pub mod laz;
/// In-process LiDAR memory-store utilities and `memory://lidar/<id>` path helpers.
pub mod memory_store;
pub mod ply;
pub mod point;
pub mod reproject;

pub use error::{Error, Result};
pub use frontend::{
	read,
	read_columns,
	read_columns_chunked,
	read_point_count,
	read_with_diagnostics,
	rewrite_columns_chunked,
	write,
	write_auto,
	write_auto_with_options,
	write_with_options,
	CopcWriteOptions,
	LazWriteOptions,
	LidarFormat,
	LidarWriteOptions,
	PointField,
	PointColumnChunkReader,
	PointColumnChunkRewriter,
	PointCloud,
	ReadDiagnostics,
};
pub use crs::Crs;
pub use point::{Color, ExtraBytes, GpsTime, PointRecord, Rgb16, WaveformPacket};
pub use io::{PointReader, PointWriter, SeekableReader};
pub use hdf_adapter::{
	GEDI_L2B_CANOPY_STYLE_DATASET_PATH,
	GEDI_L2B_CANOPY_STYLE_KNOWN_BYTE_OFFSET,
	HdfAdapterResult,
	HdfDatasetProvider,
	HdfI16WindowRequest,
	ICESAT2_ATL08_BEAM_GROUP_CANDIDATES,
	ICESAT2_ATL08_CANOPY_NODATA_VALUE,
	ICESAT2_ATL08_CANOPY_SUBPATH,
	ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES,
	ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES,
	WbhdfDatasetProvider,
	read_gedi_l2b_canopy_style_f32_window_in_file,
	read_icesat2_atl08_h_canopy_f32_window_in_file,
	resolve_icesat2_atl08_h_canopy_object_header_in_file,
	resolve_icesat2_atl08_h_canopy_path_in_file,
};
pub use hdf_products::{
	GediL2bCanopyProvider,
	HdfLidarProductFamily,
	HdfLidarProductProvider,
	HdfLidarProductRegistry,
	HdfLidarReadDiagnostics,
	Icesat2Atl08CanopyProvider,
	ResolvedHdfLidarProduct,
	detect_hdf_lidar_product_family,
	icesat2_atl08_canopy_subpath,
	read_hdf_lidar_canopy_f32_window_in_file,
	read_hdf_lidar_canopy_f32_window_with_diagnostics,
	resolve_hdf_lidar_product,
};
