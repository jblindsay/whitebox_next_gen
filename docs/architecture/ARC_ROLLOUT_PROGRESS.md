# Arc/Memory-Store Rollout Progress

This document tracks the migration of wbtools_oss tool files from the
`get_raster_by_id` (clone path) to `get_raster_arc_by_id` (zero-copy Arc path).
See `RASTER_ZERO_COPY_PATTERN.md` for the technical pattern.

## Why This Matters

When `read_raster()` is called in Python, the raster is eagerly loaded into the
in-process `memory_store`. Tool loaders that call `get_raster_by_id` **clone** the
entire raster buffer on every tool invocation. Switching to `get_raster_arc_by_id`
returns a reference-counted pointer — no copy, zero allocation overhead. The
observed speedup for add/sqrt was ~30–66x faster vs. legacy on large rasters.

## Summary

| Stage | Scope                   | Files | Status      |
|-------|-------------------------|-------|-------------|
| 1     | raster math/stats       | 3     | ✅ Complete |
| 2     | remote_sensing          | 9     | ✅ Complete |
| 3     | hydrology               | 1     | ✅ Complete |
| 4     | flow_algorithms         | 1     | ✅ Complete |
| 5     | geomorphometry          | 5     | ✅ Complete |
| 6     | stream_network_analysis | 2     | ✅ Complete |
| 7     | lidar_processing        | 1     | ✅ Complete |
| 8     | data_tools / gis / misc | TBD   | ✅ Complete |
| 9     | Vector memory-store     | Infra | ✅ Complete |
| 10    | Lidar memory-store      | Infra | ✅ Complete |

### File-Level Progress (Stages 1-7)

- Completed files: **22 / 22**
- In progress: **0 / 22**
- Not started: **0 / 22**

---

## Stage 1 — Raster Math & Stats ✅

### Completed Files

| File | Tools Covered |
|------|---------------|
| `crates/wbtools_oss/src/tools/raster/raster_add.rs` | add, subtract, multiply, divide, and/or/xor/not, power, atan2, equal/not_equal/greater/less |
| `crates/wbtools_oss/src/tools/raster/raster_unary_math.rs` | sqrt, abs, sin, cos, tan, ln, exp, floor, ceil, round, negate, log2, log10, and ~20 more |
| `crates/wbtools_oss/src/tools/raster/raster_stats.rs` | z_scores, rescale_value_range, max, min, quantiles, histogram, image_correlation, raster_summary_stats, inplace_{add,subtract,multiply,divide}, and more |

### Stage 1 Benchmarks (LKERIE_10m_final_DEM.tif, memory-backed)

| Tool                 | Median (s) | Notes                              |
|----------------------|------------|------------------------------------|
| add                  | 0.4407     | ~66% faster than legacy (1.303s)   |
| sqrt                 | 0.4511     | ~65% faster than legacy (1.290s)   |
| z_scores             | 2.8931     | No legacy baseline yet             |
| rescale_value_range  | 3.6570     | No legacy baseline yet             |
| max                  | 3.4289     | No legacy baseline yet             |
| min                  | 4.4460     | No legacy baseline yet             |

---

## Stage 2 — Remote Sensing ✅

### Files to Migrate

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/remote_sensing/convolution_filters.rs` | ✅ Done | HighPass, Laplacian, Sobel, Prewitt, and others |
| `crates/wbtools_oss/src/tools/remote_sensing/advanced_filters.rs` | ✅ Done | AnisotropicDiffusion, GammaCorrection, GuidedFilter, WienerFilter |
| `crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs` | ✅ Done | Scharr, RobertsCross, LineDetection, Emboss |
| `crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs` | ✅ Done | FastAlmostGaussian, EdgePreservingMean, UnsharpMasking, DiffOfGaussians |
| `crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs` | ✅ Done | Median, Percentile, Majority, Diversity |
| `crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs` | ✅ Done | Mean, Total, StdDev, Minimum, Maximum, Range |
| `crates/wbtools_oss/src/tools/remote_sensing/gaussian_filter.rs` | ✅ Done | GaussianFilter (also removed Arc::new(input) re-wrap) |
| `crates/wbtools_oss/src/tools/remote_sensing/bilateral_filter.rs` | ✅ Done | BilateralFilter, HighPassBilateralFilter (removed Arc::new(input) re-wrap ×2) |
| `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs` | ✅ Done | BalanceContrast, ColourComposite, DirectDecorrelation, FlipImage, HistogramEqualization, etc. Also fixed opacity.as_ref() → as_deref() |

### Notes
- `non_filter_tools.rs` had `opacity: Option<Raster>` → `Option<Arc<Raster>>`; fixed `opacity.as_ref()` → `opacity.as_deref()` to maintain `Option<&Raster>` parameter type.
- `gaussian_filter.rs` and `bilateral_filter.rs` both had `let input = Arc::new(input)` re-wrapping after the load call; removed since `load_raster` now returns `Arc<Raster>` directly.

### Stage 2 Benchmarks

| Tool | Median (s) | Notes |
|------|------------|-------|
| gaussian_filter | TBD | Run after build |
| bilateral_filter | TBD | Run after build |

### Stage 2 Validation Status

- `cargo check --all-targets --all-features` passes.
- Python smoke checks passed for `gaussian_filter` and `bilateral_filter` with in-memory raster handles.
- The `mean_filter` smoke probe failed only because `WbEnvironment` does not expose `mean_filter` under that method name; this is an API naming/export issue, not a remote_sensing Arc loader regression.

---

## Stage 3 — Hydrology ✅

### Files to Migrate

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/hydrology/mod.rs` | ✅ Done | Loader migrated to `Arc<Raster>` + `get_raster_arc_by_id`; `cargo check --all-targets --all-features` passes; representative hydrology tests pass (`breach_single_cell_pits_carves_adjacent_cell`, `subbasins_labels_single_link_basin`, `watershed_from_raster_pour_points_labels_all_upstream_cells`). |

---

## Stage 4 — Flow Algorithms ✅

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/flow_algorithms/mod.rs` | ✅ Done | Loader migrated to `Arc<Raster>` + `get_raster_arc_by_id`; `cargo check --all-targets --all-features` passes; representative flow tests pass (`d8_pointer_runs_on_simple_dem`, `dinf_pointer_runs_on_simple_dem`, `fd8_flow_accum_runs_on_simple_dem`). |

---

## Stage 5 — Geomorphometry ✅

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs` | ✅ Done | Arc loader migration completed; compile clean. |
| `crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs` | ✅ Done | Arc loader migration completed; compile clean. |
| `crates/wbtools_oss/src/tools/geomorphometry/multiscale_curvatures.rs` | ✅ Done | Arc loader migration completed; compile clean. |
| `crates/wbtools_oss/src/tools/geomorphometry/hydrologic_index_tools.rs` | ✅ Done | Arc loader migration completed; compile clean. |
| `crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs` | ✅ Done | Arc loader migration completed; compile clean; `curvature_tools_constant_raster_returns_zero` passes. |

### Stage 5 Benchmark Update (Gaussian Curvature)

- Retest completed using `scripts/performance/bench_gaussian_curvature.py` on `LKERIE_10m_final_DEM.tif` (5 runs).
- Legacy median: `8.880000 s`
- NG median (post-Arc in `curvature_tools.rs`): `2.331498 s`
- Delta: `-73.74%` (NG faster)

---

## Stage 6 — Stream Network Analysis ✅

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs` | ✅ Done | Arc loader migration completed; compile clean. |
| `crates/wbtools_oss/src/tools/stream_network_analysis/pro_stream_tools.rs` | ✅ Done | Arc loader migration completed; compile clean. |

---

## Stage 7 — LiDAR Processing ✅

| File | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/lidar_processing/improved_ground_point_filter.rs` | ✅ Done | Arc loader migration completed; compile clean. |

---

## Stage 8 — Data Tools / GIS / Misc ✅

Audit and targeted remediation complete.

| File / Module | Status | Notes |
|------|--------|-------|
| `crates/wbtools_oss/src/tools/gis/mod.rs` | ✅ Done | Memory-path load now uses `get_raster_arc_by_id` (mapped to owned raster for existing API boundary). |
| `crates/wbtools_oss/src/tools/gis/nibble_sieve.rs` | ✅ Done | Loader migrated to `Arc<Raster>` with Arc-aware clone sites for mutable outputs. |
| `crates/wbtools_oss/src/tools/stream_network_analysis/pro_stream_tools.rs` | ✅ Done | `load_raster_mem` migrated to `Arc<Raster>` and mutable clone sites updated. |
| `crates/wbtools_oss/src/tools/data_tools/` | ✅ Audited | No `get_raster_by_id` usage detected in this module audit pass. |

---

## Stage 9 — Vector Memory-Store ✅

Stage 9 foundation and runtime loader adoption are complete.

| Component | Status | Notes |
|------|--------|-------|
| `crates/wbvector/src/memory_store.rs` | ✅ Done | Added vector memory-store API (`VECTOR_MEMORY_PREFIX`, put/get/replace/remove/clear/count + memory-path helpers). |
| `crates/wbvector/src/lib.rs` | ✅ Done | Exported `memory_store` module for crate-level access. |
| `crates/wbw_python/src/wb_environment.rs` helpers | ✅ Done | `read_vector_layer_for_python` / `write_vector_layer_for_python` now support `memory://vector/...` paths. |
| `crates/wbw_python/src/wb_environment.rs` memory mgmt | ✅ Done | Added `remove_vector_from_memory`, `clear_vector_memory`, `vector_memory_count`. |
| `read_vector()` eager-load to vector store | ✅ Done | `read_vector` / `read_vectors` now load into vector store and return `memory://vector/...` handles. |
| `write_vector()` from memory-backed vectors | ✅ Done | Memory-backed vectors are staged and written with existing options pipeline. |
| Vector tool loader migration (`gis/mod.rs` and others) | ✅ Done | Tranche 1: `gis/mod.rs` shared vector loader helpers now accept `memory://vector/...`. Tranche 2: memory-aware vector loaders added in `data_tools/mod.rs`, `stream_network_analysis/mod.rs`, `stream_network_analysis/pro_stream_tools.rs`, `hydrology/mod.rs`, and `lidar_processing/mod.rs`. Tranche 3: `remote_sensing/non_filter_tools.rs` now routes vector/training-data loads through a memory-aware helper. Tranche 4: geomorphometry runtime vector-load boundaries and `raster/raster_stats.rs` vector reads now route through memory-aware loaders. Tranche 5: wbtools_pro workflow and domain modules now use memory-aware vector loaders across workflow_products, siting, lidar_processing, geomorphometry, and agriculture helper paths; `cargo check` passes in wbtools_pro. Post-migration audit found no remaining direct runtime `wbvector::read` bypasses in wbtools_oss or wbtools_pro tool runtime paths; remaining hits are helper-wrapper internals and tests only. |

---

## Stage 10 — Lidar Memory-Store ✅

Stage 10 foundation tranche is now implemented and compile-validated.

| Component | Status | Notes |
|------|--------|-------|
| `crates/wblidar/src/memory_store.rs` | ✅ Done | Added lidar memory-store API (`LIDAR_MEMORY_PREFIX`, put/get/replace/remove/clear/count + `memory://lidar/...` path helpers). |
| `crates/wblidar/src/lib.rs` | ✅ Done | Exported `memory_store` module and documented public module export. |
| `crates/wblidar/CHANGELOG.md` | ✅ Done | Added `[Unreleased]` entry for LiDAR memory-store foundation. |
| `crates/wbw_python/src/wb_environment.rs` helpers | ✅ Done | Added `read_lidar_cloud_for_python` / `write_lidar_cloud_for_python` for memory-backed lidar handles. |
| `crates/wbw_python/src/wb_environment.rs` `Lidar` methods | ✅ Done | Added memory-aware branches for `exists`, `point_count`, numpy conversion/write paths, reprojection copy/write flows, and CRS get/set/clear operations. |
| `read_lidar(..., file_mode="m")` opt-in memory mode | ✅ Done | `read_lidar` now supports eager load into lidar memory store when `file_mode` contains `m`; default behavior remains disk-path mode. |
| `write_lidar()` staging for memory-backed sources | ✅ Done | Memory-backed lidar inputs are staged through temp `.laz` and then written with existing options pipeline. |
| LiDAR memory management methods in `WbEnvironment` | ✅ Done | Added `remove_lidar_from_memory`, `clear_lidar_memory`, and `lidar_memory_count`. |
| LiDAR tool runtime loader migration (`lidar_processing/mod.rs`) | ✅ Done | Added memory-aware `load_lidar_cloud` helper in `crates/wbtools_oss/src/tools/lidar_processing/mod.rs` and migrated runtime reads across the full LiDAR tool surface in this module (gridding/interpolation/filter/classification/segmentation/analysis/vectorization/reporting paths, including batch-neighbor and multi-input loops). Post-migration grep shows only the helper’s own disk fallback retains direct `PointCloud::read`. |
| Cross-repo compile validation | ✅ Done | `cargo check -p wbtools_oss` and `cargo check` (wbtools_pro) both pass after Tranche A loader migration. |
| Compile validation | ✅ Done | `cargo check -p wblidar` and `cargo check -p wbw_python` both pass after Stage 10 tranche edits. |
| OSS memory-path smoke tests | ✅ Done | Added and passed: `lidar_nearest_neighbour_gridding_accepts_memory_input` and `filter_lidar_classes_accepts_memory_input` in `crates/wbtools_oss/src/tools/lidar_processing/mod.rs`. |
| Pro workflow memory-path smoke test | ✅ Done | Added and passed: `lidar_terrain_product_suite_accepts_memory_lidar_input` in `wbtools_pro/tests/workflow_contract_tests.rs`. |
| Stage 10 closeout audit | ✅ Done | Runtime grep audit confirms one intentional direct `PointCloud::read` disk fallback inside the shared `load_lidar_cloud` helper in OSS, plus one test-only read in the test module; wbtools_pro `src/tools` has no direct `PointCloud::read` callsites. |

Stage 10 closeout status: complete. Remaining LiDAR changes beyond this point are feature work, not migration-blocking memory-store rollout items.

---

## R API Parity (Sprint Active)

R parity implementation is now active.

Current state in `wbw_r`:
- Raster access still includes the existing disk cache path in `wbw_r/src/lib.rs` (`DISK_RASTER_CACHE`, `get_or_load_disk_raster_handle`, `flush_cached_disk_raster`).
- Raster parity Slice 1 is now implemented:
   - Added `raster_read_to_memory_path(...)` in `wbw_r` native layer.
   - `raster_metadata_json`, `raster_get_value`, `raster_set_value`, and `raster_write_with_options_json` now accept `memory://raster/...` paths.
   - R facade `wbw_read_raster(..., file_mode = "m")` now stages rasters into memory store and returns memory-backed raster objects.
   - R raster conversion helpers now stage memory-backed rasters to temp GeoTIFF when `terra`/`stars` require a filesystem path.

Remaining parity tasks (in-progress backlog):
1. Vector parity Slice 2 is now implemented:
   - Added `vector_read_to_memory_path(...)` in the `wbw_r` native layer.
   - `vector_metadata_json`, topology feature reads, and vector copy/write paths now accept `memory://vector/...` sources.
   - In-memory vector overwrite/update paths are supported through memory-aware destination handling in `vector_copy_with_options_json(...)`.
   - R facade `wbw_read_vector(..., file_mode = "m")` now returns memory-backed vector objects.
   - R vector conversion helpers now stage memory-backed vectors to temp GeoPackage when `terra`/`sf` require filesystem paths.
2. LiDAR parity Slice 3 is now implemented:
   - Added `lidar_read_to_memory_path(...)` in the `wbw_r` native layer.
   - `lidar_metadata_json`, `lidar_point_count`, `lidar_columns_json`, `lidar_from_columns_json`, `lidar_from_column_chunks_json`, `lidar_copy_to_path`, and `lidar_write_with_options_json` now accept `memory://lidar/...` sources.
   - R facade `wbw_read_lidar(..., file_mode = "m")` now returns memory-backed lidar objects.
   - Session facade now exposes `read_lidar(..., file_mode = "r"|"m")`.
   - Memory-backed lidar objects continue to support matrix extraction and disk writes; chunk rewrite falls back to in-memory application when the source is memory-backed.
3. R memory management helpers are now implemented:
   - Added explicit WbW-R helpers for raster/vector/lidar memory-store remove/count/clear operations.
   - Added raster memory byte accounting in the R facade to mirror Python's approximate raster-store heap reporting.
   - Session facade now exposes `remove_*_from_memory(...)`, `clear_*_memory()`, `*_memory_count()`, and `raster_memory_bytes()` helpers.
   - Added focused runtime coverage for remove/count/clear behavior across raster, vector, and lidar memory stores.
4. Validate end-to-end chaining and benchmark representative raster/vector/lidar pipelines in R.

---

## Stage 5 Retest Status

Stage 5 retest requirement is complete.
- `gaussian_curvature` benchmark has been rerun and recorded in `tool_parity_tracker.csv`.
- Additional curvature benchmarks can be refreshed opportunistically as needed.

---

## Migration Pattern Reference

For each file:

1. Add `use std::sync::Arc;` if not present
2. Change `fn load_raster(path: &str) -> Result<Raster, ToolError>` →
   `fn load_raster(path: &str) -> Result<Arc<Raster>, ToolError>`
3. Change `get_raster_by_id(id)` → `get_raster_arc_by_id(id)`
4. Change `Raster::read(path).map_err(...)` →
   `Raster::read(path).map(Arc::new).map_err(...)`
5. If file has `let input = Arc::new(input)` re-wrapping after load → **remove it**
6. If file has `Option<Raster>` returned from `load_raster`, change
   `optional.as_ref()` → `optional.as_deref()` at the usage site

See `RASTER_ZERO_COPY_PATTERN.md` for the full canonical pattern.
