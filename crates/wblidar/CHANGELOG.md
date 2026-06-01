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
- Added initial `hdf_adapter` module with a minimal provider trait (`HdfDatasetProvider`) and
  `WbhdfDatasetProvider` implementation for bounded i16 dataset-window reads delegated to
  `wbhdf::hdf4::decode_hdf4_sds_i16_window_at_in_file(...)`.
- Added first concrete Tier 1 GEDI mapping helper in `hdf_adapter`:
  `read_gedi_l2b_canopy_style_f32_window_in_file(...)`, which currently targets
  `/BEAM0000/elev_lowestmode` using the validated contiguous offset path while generalized
  object-header-driven offset resolution is still in progress.
- Added fixture-backed `wblidar` adapter test coverage for the GEDI Tier 1 mapping helper,
  validating the first reference window against known expected values when `WBHDF_FIXTURE_DIR`
  provides the GEDI sample fixture.
- Added second Tier 1 ingestion path in `hdf_adapter` for ICESat-2 ATL08 canopy data:
  `read_icesat2_atl08_h_canopy_f32_window_in_file(...)`, including deterministic fill-to-nodata
  mapping and bounded window extraction for the first validated chunk path.
- Added dynamic ATL08 beam-group path discovery helper
  `resolve_icesat2_atl08_h_canopy_path_in_file(...)` with candidate enumeration across
  `gt1l/gt1r/gt2l/gt2r/gt3l/gt3r` and deterministic missing-path error semantics.
- Added fixture-backed ATL08 adapter tests validating first-chunk decode counts
  (`valid=3640`, `nodata=6360`) plus explicit missing-path behavior coverage.
- Added `hdf_products` provider-registry layer for HDF LiDAR family dispatch:
  - `HdfLidarProductProvider` trait + `HdfLidarProductRegistry::with_defaults()`
    (ATL08 + GEDI providers),
  - canonical family resolution APIs (`detect_hdf_lidar_product_family`,
    `resolve_hdf_lidar_product`),
  - unified canopy-window dispatch entrypoint
    (`read_hdf_lidar_canopy_f32_window_in_file(...)`) that routes to product-specific
    adapter reads.
- Replaced fixed ATL08 `h_canopy` object-header offset dependency with bounded dynamic
  v1 object-header discovery + ranking (`resolve_icesat2_atl08_h_canopy_object_header_in_file(...)`),
  so canopy reads no longer require a hardcoded fixture-specific header address.
- Tightened ATL08 dynamic header ranking to incorporate resolved beam-path affinity
  (marker proximity from the selected `/<gt*>/land_segments/canopy/h_canopy` path), reducing
  ambiguity when multiple chunked v1 object-header candidates are present.
- Added runtime diagnostics counters for bounded HDF canopy reads via
  `read_hdf_lidar_canopy_f32_window_with_diagnostics(...)` and
  `HdfLidarReadDiagnostics` (`chunks_visited`, `chunks_decoded`, `filter_failures`,
  `unsupported_layout_failures`, `invalid_chunk_failures`, `dataset_resolution_failures`).
- Added bounded-memory safeguards for ATL08 canopy chunk decode flow with explicit
  compressed/decompressed size caps (`ICESAT2_ATL08_MAX_COMPRESSED_CHUNK_BYTES`,
  `ICESAT2_ATL08_MAX_DECOMPRESSED_CHUNK_BYTES`) and deterministic `UnsupportedLayout`
  diagnostics when limits are exceeded.
- Added malformed/partial-corruption regression coverage for ATL08-like HDF inputs in
  unified dispatch diagnostics tests, asserting deterministic unsupported-layout failure
  classification and counters.

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
