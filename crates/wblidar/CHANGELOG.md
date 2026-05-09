# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]
### Fixed
- `default_las_config` (used by all `PointCloud::write` / `write_las` / `write_laz` paths) now
  auto-computes `x_offset`, `y_offset`, `z_offset` from `floor(min)` of the point cloud's
  bounding box instead of leaving them at `0.0`. The previous default caused silent i32 overflow
  when storing UTM northings (or other large coordinates) with `scale = 0.001`, because values
  such as 4 800 000 m exceeded the i32 range (~±2 147 483). The overflow saturated all affected
  coordinates to `i32::MAX`, collapsing every point to the same Y value and breaking all
  downstream triangulation-based tools (e.g. `improved_ground_point_filter`).
### Added
- Added in-process LiDAR memory-store foundation (`memory://lidar/<id>`) via new
	`memory_store` module with APIs for put/get/replace/remove/clear/count and
	memory-path helpers.

## [0.1.1] – 2026-05-09 (Reaffirmed)

### Testing
- Interop Phase B LiDAR cases: L01 (LAS 1.4), L02 (LAZ compressed), L03 (COPC) all passing.
- Python binding architecture decoupled from WbW-R; native wblidar write path now fully backend-native.

*Note: Version 0.1.1 is being published as-is as part of the interop release milestone (2026-05-09) to affirm Phase B validation.*

### Changed
- Added `PointCloud::apply_columns_range(...)` to support in-place updates over
	bounded point-index ranges, enabling chunk-by-chunk edit pipelines.
- Public exports in `lib.rs` now include chunked read/rewrite frontend types and
	helper functions for downstream crate reuse.
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
