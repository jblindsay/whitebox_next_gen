# LiDAR Processing Parity Checklist

Reference legacy source:
- /Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing

Scope:
- New backend implementations in wbtools_oss + wbtools_pro
- Python wrapper parity in wbw_python

## Status Legend
- done: implemented and compiled in new backend
- partial: present but still behaviorally simplified vs legacy
- todo: not yet parity-complete

## Baseline Inventory Note
- Legacy LiDAR source files total 64 tools (`*.rs`, excluding `mod.rs`) under `src/tools/lidar_processing`.
- Legacy module exports currently list 63 active tools in `src/tools/lidar_processing/mod.rs`.
- The discrepancy is the unexported file `lidar_dem_full_workflow.rs` (present on disk, not listed in `mod.rs`).
- Parity tracking should use:
	- 63 for active module-parity accounting.
	- 64 only when intentionally including dormant/unexported legacy tools.

## Current Scope Note
- `lidar_dem_full_workflow` is excluded from parity tracking because it is not treated as a complete active tool target for the new backend.

## Phase Priorities
- Phase 1: LiDAR-to-raster interpolation and gridding tools (highest user impact)
- Phase 2: LiDAR-to-LiDAR filtering/classification/transforms
- Phase 3: LiDAR-to-vector, diagnostics, and specialized analysis tools

## Tool-by-Tool Status

### Phase 1: LiDAR-to-Raster (CRS-critical)
- lidar_nearest_neighbour_gridding: done
- lidar_idw_interpolation: done
- lidar_tin_gridding: done
- lidar_radial_basis_function_interpolation: done
- lidar_sibson_interpolation: done
- lidar_digital_surface_model: done
- lidar_block_maximum: done
- lidar_block_minimum: done
- lidar_point_density: done
- lidar_hillshade: done

### Phase 2: LiDAR-to-LiDAR Processing
- normalize_lidar: done
- height_above_ground: done
- lidar_ground_point_filter: done
- filter_lidar: done
- filter_lidar_classes: done
- filter_lidar_noise: done
- filter_lidar_scan_angles: done
- filter_lidar_by_percentile: done
- filter_lidar_by_reference_surface: done
- lidar_thin: done
- lidar_thin_high_density: done
- remove_duplicates: done
- modify_lidar: done
- lidar_shift: done
- sort_lidar: done
- split_lidar: done
- lidar_join: done
- lidar_tile: done
- classify_lidar: done
- classify_overlap_points: done
- classify_buildings_in_lidar: done
- lidar_classify_subset: done
- lidar_remove_outliers: done
- lidar_segmentation: done
- lidar_segmentation_based_filter: done
- clip_lidar_to_polygon: done
- erase_polygon_from_lidar: done
- select_tiles_by_polygon: done
- lidar_colourize: done
- colourize_based_on_class: done
- colourize_based_on_point_returns: done
- ascii_to_las: done
- las_to_ascii: done

### Phase 3: LiDAR-to-Vector / Analysis / Diagnostics
- lidar_contour: done
- lidar_construct_vector_tin: done
- lidar_hex_bin: done
- lidar_tile_footprint: done
- las_to_shapefile: done
- lidar_info: done
- lidar_histogram: done
- lidar_point_stats: done
- lidar_point_return_analysis: done
- flightline_overlap: done
- recover_flightline_info: done
- find_flightline_edge_points: done
- lidar_ransac_planes: done
- lidar_rooftop_analysis: done
- lidar_elevation_slice: done
- lidar_eigenvalue_features: done
- lidar_tophat_transform: done
- normal_vectors: done
- individual_tree_detection: done
- lidar_kappa: done

## Cross-Cutting CRS Requirements
- Any tool that outputs raster(s) from LiDAR input must assign output raster CRS from input LiDAR CRS metadata (EPSG and/or WKT).
- Multi-input tools must align all LiDAR datasets to a single reference CRS before interpolation/gridding.
- If CRS metadata is missing or ambiguous and alignment cannot be resolved, the tool should fail with a clear validation error.
- Regression tests must verify CRS propagation for EPSG-based and WKT-only source metadata.

## Validation Standard (Per Tool)
- Registry discoverability test.
- Runtime smoke test with small fixture(s).
- CRS behavior tests when applicable.
- Wrapper method exposure in wbw_python and docs entry added.
- `cargo check` for touched crates.

## Recent Changes Implemented
- 2026-03-23: Created LiDAR parity checklist and phase grouping with CRS policy gate for raster outputs.
- 2026-03-23: Implemented Batch A backend tools (`lidar_nearest_neighbour_gridding`, `lidar_idw_interpolation`, `lidar_tin_gridding`) and wired default registry exports.
- 2026-03-23: Added legacy-style parameter/filter support (`returns` alias, `excluded_classes`, `min_elev`/`max_elev`, interpolation parameter selection) in Batch A tools.
- 2026-03-23: Added integration coverage for Batch A registry presence plus EPSG and WKT-only CRS propagation.
- 2026-03-23: Added Python `WbEnvironment` convenience methods for Batch A LiDAR tools and documented current API signatures.
- 2026-03-23: Added parity-oriented regression tests for IDW and TIN legacy filtering aliases and thresholds.
- 2026-03-23: Ported `lidar_radial_basis_function_interpolation` and `lidar_sibson_interpolation` into wbtools_oss with CRS propagation and filter support.
- 2026-03-23: Added Python `WbEnvironment` convenience methods and docs coverage for RBF and Sibson LiDAR interpolation tools.
- 2026-03-23: Replaced the initial `lidar_sibson_interpolation` approximation with a true Sibson/natural-neighbour implementation backed by new `wbtopology` prepared interpolation support.
- 2026-03-23: Completed interpolation-parameter parity for Phase-1 interpolation tools by adding `time` and `rgb` support and associated regression coverage.
- 2026-03-23: Completed functional `poly_order` support (`none`, `constant`, `quadratic`) for `lidar_radial_basis_function_interpolation` with regression tests.
- 2026-03-23: Phase-1 interpolation batch (`lidar_nearest_neighbour_gridding`, `lidar_idw_interpolation`, `lidar_tin_gridding`, `lidar_radial_basis_function_interpolation`, `lidar_sibson_interpolation`) now marked done.
- 2026-03-23: Ported next Phase-1 LiDAR tools (`lidar_block_maximum`, `lidar_block_minimum`, `lidar_point_density`, `lidar_digital_surface_model`, `lidar_hillshade`) with registry wiring, wrappers, and CRS/runtime regression coverage.
- 2026-03-23: Started legacy batch-mode parity rollout (optional input scans working directory and per-tile parallel processing) for `lidar_block_maximum` and `lidar_block_minimum` in backend; Python wrapper/API parity for no-input batch mode is still pending.
- 2026-03-23: Extended backend batch-mode rollout to `lidar_point_density` and `lidar_digital_surface_model`, including no-input integration coverage for batch tile discovery and auto-written outputs.
- 2026-03-23: Extended backend batch-mode rollout to interpolation tools (`lidar_nearest_neighbour_gridding`, `lidar_idw_interpolation`, `lidar_tin_gridding`, `lidar_radial_basis_function_interpolation`, `lidar_sibson_interpolation`) with no-input integration coverage and documented wrapper-facing behavior.
- 2026-03-23: Added interpolation batch edge-effect reduction support by including neighboring batch-tile points during per-tile interpolation while preserving each output tile extent.
- 2026-03-23: Updated batch-mode output contract to return only a single placeholder raster path (first written tile) while writing all generated rasters directly to disk, and updated Python LiDAR wrappers to accept `input=None` for batch execution.
- 2026-03-23: Started Phase-2 LiDAR-to-LiDAR ports for `filter_lidar_classes`, `lidar_shift`, and `remove_duplicates` with backend implementations, batch-mode support, and placeholder-return semantics.
- 2026-03-23: Added Python `WbEnvironment` wrappers and LiDAR processing docs coverage for `filter_lidar_classes`, `lidar_shift`, and `remove_duplicates`.
- 2026-03-23: Fixed pre-existing bug in `wblidar` LAS 1.4 PDRF 6+ reader (`decode_flags_v14`): classification and subsequent fields were read at the wrong byte offsets, causing all classifications to read back as 0. Fix aligns offsets with LAS 1.4 spec.
- 2026-03-23: Ported additional Phase-2 tools (`filter_lidar_scan_angles`, `filter_lidar_noise`, `lidar_thin`, `lidar_elevation_slice`) in wbtools_oss with registry wiring, Python wrappers, batch-mode suffix outputs, and integration test coverage.
- 2026-03-23: Closed parity checklist for all previously partial tools by finalizing behavior/test/docs coverage, including `lidar_thin` filtered-output support and `lidar_hillshade` batch-mode call-shape compatibility.
- 2026-03-23: Started next Phase-2 batch by porting `lidar_join`, `lidar_thin_high_density`, `lidar_tile`, `sort_lidar`, and `filter_lidar_by_percentile` with registry wiring, Python wrappers, and initial integration coverage.
- 2026-03-23: Continued Phase-2 rollout with `split_lidar` and `lidar_remove_outliers`, including backend implementations, Python wrappers, and integration coverage for single-input and batch modes.
- 2026-03-23: Continued Phase-2 rollout with `normalize_lidar` and `height_above_ground`, including backend implementations, Python wrappers, and initial integration coverage.
- 2026-03-23: Continued Phase-2 rollout with `lidar_ground_point_filter` and `filter_lidar`, including backend implementations, Python wrappers, and initial integration coverage.
- 2026-03-23: Added hardening/edge-case validation for `normalize_lidar`, `height_above_ground`, `lidar_ground_point_filter`, and `filter_lidar`; promoted these tools to done.
- 2026-03-23: Continued Phase-2 rollout with `filter_lidar_by_reference_surface` and `classify_lidar`, including backend implementations, Python wrappers, and initial integration coverage.
- 2026-03-23: Deepened `classify_lidar` parity with multi-stage RANSAC geometry, erosion-dilation residuals, cluster segmentation, roof/facade refinement, and rooftop clutter reassignment; kept status partial pending further legacy-equivalence verification.
- 2026-03-23: Hardened and promoted to done: `sort_lidar`, `filter_lidar_by_percentile`, and `lidar_remove_outliers` after adding targeted alias/extreme-percentile/classification edge-case regression tests.
- 2026-03-23: Hardened and promoted to done: `lidar_join`, `lidar_tile`, and `lidar_thin_high_density` after adding targeted join-order/tile-threshold/filtered-output regression tests.
- 2026-03-23: Hardened and promoted to done: `filter_lidar_by_reference_surface`, `split_lidar`, and `classify_lidar` after adding targeted classify/preserve-class semantics, split `num_pts` output-count, and classify batch-mode regression tests.
- 2026-03-23: Implemented and promoted to done: `lidar_classify_subset`, `clip_lidar_to_polygon`, and `erase_polygon_from_lidar` with registry wiring, Python wrappers, and end-to-end integration coverage.
- 2026-03-23: Implemented `classify_overlap_points` (done) and added first-pass compatible implementations for `lidar_segmentation` and `lidar_segmentation_based_filter` (partial), with wrappers and regression coverage.
- 2026-03-23: Implemented and promoted to done: `lidar_colourize`, `colourize_based_on_class`, and `colourize_based_on_point_returns` with wrapper/docs updates and end-to-end integration coverage.
- 2026-03-23: Implemented and promoted to done: `classify_buildings_in_lidar`, `ascii_to_las`, and `las_to_ascii` with registry wiring, wrapper/docs updates, and integration coverage.
- 2026-03-23: Implemented and promoted to done: `select_tiles_by_polygon` with directory-selection/copy behavior, registry and wrapper wiring, and integration coverage.
- 2026-03-23: Implemented and promoted to done: `modify_lidar`; added additional segmentation hardening coverage and promoted `lidar_segmentation` and `lidar_segmentation_based_filter` to done.
- 2026-03-23: Implemented and promoted to done: `lidar_info`, `lidar_histogram`, and `lidar_point_stats` with registry/wrapper wiring, report/raster outputs, and integration coverage.
- 2026-03-23: Implemented and promoted to done: `lidar_contour`, `lidar_tile_footprint`, and `las_to_shapefile` with registry/wrapper wiring, vector outputs, and integration coverage.
- 2026-03-23: Implemented and promoted to done: `lidar_construct_vector_tin`, `lidar_hex_bin`, and `lidar_point_return_analysis` with registry/wrapper wiring, vector/report outputs, and integration coverage.

## Remaining High-Value Parity Work
- Build shared LiDAR CRS helper utilities in backend before large tool batches.
- Continue with remaining Phase-3 LiDAR tools (vector/analysis/diagnostics).
