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
pub mod io;
pub mod las;
pub mod laz;
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
