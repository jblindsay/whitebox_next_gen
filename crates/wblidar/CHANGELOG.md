# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [0.1.1] - 2026-04-11
### Added
- Added format-aware LiDAR write options in the unified frontend API:
	- `LidarWriteOptions`
	- `LazWriteOptions`
	- `CopcWriteOptions`
- Added option-aware write functions:
	- `write_with_options(...)`
	- `write_auto_with_options(...)`
- Added option-aware `PointCloud` methods:
	- `PointCloud::write_with_options(...)`
	- `PointCloud::write_as_with_options(...)`

### Changed
- LAZ output now applies optional `chunk_size` and `compression_level` controls
	when provided through the frontend write options.
- COPC output now applies optional `max_points_per_node`, `max_depth`, and
	`node_point_ordering` controls when provided through the frontend write
	options.
- Export surface in `lib.rs` now re-exports write-option types and functions
	so downstream crates can consume the new API directly.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
