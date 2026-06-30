# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [2.0.7] - 2026-06-30

### Added
- Field parameter schemas are now exposed through Python bindings via `list_tool_catalog_json_with_options(...)` and `get_tool_metadata_json(...)`. Downstream consumers (QGIS plugin, Python scripts, Jupyter notebooks) can introspect field parameters and their parent layer references for dynamic UI rendering or validation.
- Schema JSON includes `kind: "field"` and `parent: <layer_name>` for all 40+ tools with field parameter support, enabling dropdown widgets and validation.

### Fixed
- Fixed `longest_flowpath` tool output type metadata schema. The tool now correctly exposes `output_vector_any()` schema type (instead of raster) through Python bindings. QGIS and downstream metadata consumers will correctly identify the output as a vector layer.
- Fixed `polygons_to_lines` output through Python bindings. The tool now correctly produces closed polygon rings (appends closing vertex if needed). Python and downstream consumers will receive properly closed line strings.

## [2.0.6] - 2026-06-14

### Fixed
- Fixed `quantiles` raster classification tool failing catastrophically on rasters with extreme positive skewness (e.g., 80% of pixels in range 0–2000 with outliers at 66M+). The histogram-based quantile calculation used a fixed 10,000-bin count regardless of data distribution, resulting in coarse bin widths that collapsed all quantile boundaries into a single bin; all valid pixels were then assigned the highest class. The tool now uses an adaptive-bin histogram that scales bin count with valid cell count (up to 4M bins, capped at 32 MB), ensuring quantile boundaries resolve correctly for any data distribution.

## [2.0.5] - 2026-06-09

### Added
- Added `is_pip_available()` utility function to detect when pip module is missing in the Python environment. Used for platform-aware diagnostics, particularly for detecting OSGeo4W installations on Windows.
- Added `get_osgeo4w_pip_error_message()` helper function to generate platform-specific error guidance for pip bootstrap issues. On Windows/OSGeo4W, provides step-by-step instructions for `ensurepip` or direct pip bootstrap script; on Unix-like systems provides generic pip installation guidance.

### Changed
- Enhanced error reporting in `install_or_upgrade_whitebox_workflows()` to provide helpful, context-aware error messages when pip is not available. Previously raised a generic error; now detects the pip missing case and returns structured guidance text suitable for user-facing dialogs.
- Improved plugin integration architecture to decouple tool-specific error handling from backend bootstrap logic. New diagnostic utilities (`is_pip_available`, `get_osgeo4w_pip_error_message`) enable frontend clients (QGIS, R, Python scripts) to implement context-appropriate error recovery strategies.

### Fixed
- Fixed ImportError handling in `install_or_upgrade_whitebox_workflows()` to catch and re-report pip import failures with user-friendly remediation steps, particularly for Windows OSGeo4W environments where pip is not included by default.

## [2.0.4] - 2026-06-07

### Added
- Added 28 new spatial statistics tools across four capability groups, accessible
  via `wbe.vector.spatial_statistics.*` and `wbe.raster.spatial_statistics.*`:
  - **Spatial autocorrelation (vector):** `global_morans_i`, `local_morans_i_lisa`,
    `getis_ord_gi_star`, `nearest_neighbour_index`, `quadrat_count_test`.
  - **Spatial autocorrelation (raster):** `local_morans_i_lisa_raster`,
    `getis_ord_gi_star_raster`.
  - **Variography and kriging (raster):** `ordinary_kriging`, `local_kriging`,
    `simple_kriging`, `universal_kriging`, `spacetime_kriging`, `ordinary_cokriging`.
  - **Variogram estimation (vector):** `estimate_variogram`, `fit_variogram`,
    `directional_variogram`, `kriging_cross_validation`.
  - **Spatial regression (vector):** `spatial_lag_regression`,
    `spatial_error_regression`, `geographically_weighted_regression`.
  - **Spatial regression (raster):** `spatial_lag_regression_raster`,
    `spatial_error_regression_raster`,
    `geographically_weighted_regression_raster`.
  - **Point pattern analysis (vector):** `ripleys_k_test`, `envelope_test`,
    `point_process_residuals`, `ripleys_k_function`, `point_pattern_envelope`,
    `inhomogeneous_intensity_raster`.
- Added `baseline_matching_and_diagnostics_assessment` to the terrain
  `workflow_products` subcategory.

### Changed
- Added epoch-aware reprojection parameters to Python-facing raster/vector/lidar
  reprojection methods, including `coordinate_epoch`, optional source/target
  reference epochs, explicit `operation_code`, `prefer_official_operation`, and
  `epoch_policy` routing control.
- Updated `whitebox_workflows.pyi` reprojection signatures (single and batch)
  to include the new epoch-aware parameters.
- Updated `tool_taxonomy.toml` with `raster.spatial_statistics` and
  `vector.spatial_statistics` subcategory sections; regenerated
  `tool_taxonomy.resolved.json`.

## [2.0.3] - 2026-05-30

### Added
- Added schema-aware metadata emission path that can include canonical per-parameter `schema` objects in tool catalog and metadata payloads.

### Changed
- Updated tool catalog metadata serialization to consume explicit schema maps for migrated tools before falling back to coarse compatibility fields.
- Updated manifest-parameter reconstruction for empty manifests to preserve backend metadata ordering across the full OSS/PRO catalog, with explicit legacy ordering overrides for flow-family tools.
- Updated metadata enrichment to backfill missing parameter descriptions/required flags from tool metadata when doc-derived maps are absent or incomplete.

### Fixed
- Fixed stream-tool metadata typing drift for pilot tools (`extract_streams`, `vector_stream_network_analysis`) by consuming backend-authored typed schemas.
- Fixed `d8_flow_accum` metadata parameter ordering regression so frontend and binding consumers receive legacy-logical ordering (`input`, `output`, then processing options).
- Fixed generic parameter-description fallback regressions (for example ambiguous `input`) by ensuring exported metadata includes domain-specific descriptions where available.

## [2.0.2] - 2026-05-27

### Added
- Added `get_tool_info_json(...)` and `get_tool_info_json_with_options(...)` as canonical aliases for metadata retrieval, matching runtime/schema terminology used by plugin/frontend consumers.
- Added internal Phase 1 execution and API-governance docs to make API cleanup and parity decisions repeatable:
  - `docs/internal/wbw_py_phase1_execution_checklist.md`
  - `docs/internal/wbw_py_alias_inventory.md`
  - `docs/internal/wbw_py_wbw_r_parity_ledger.md`
  - `docs/internal/wbw_py_interop_behavior_matrix.md`
  - `docs/internal/wbw_py_canonical_api_style_guide.md`
- Added `examples/interop_roundtrip_smoke_test.py` for optional NumPy/Rasterio/GeoPandas/Shapely/pyproj round-trip checks.

### Changed
- Updated Python stubs (`whitebox_workflows.pyi`) to include `get_tool_info_json` variants on top-level helpers, `RuntimeSession`, and `WbEnvironment`.
- Expanded typed category stubs for recent vector/network additions, including linear-referencing and schema-editing tools plus CSV/report output wrappers.
- Improved README discovery and workflow guidance with intent-driven entry points, canonical workflows, and recommended-vs-advanced notes around option-heavy paths.
- Removed high-confusion pre-release aliases in favor of canonical names (`metadata`, `attributes`/`attribute`, `update_attributes`/`update_attribute`, `add_field`, and canonical category properties).
- Refined docs/planning process to prioritize pre-release API clarity and explicit WbW-Py/WbW-R parity decisions.

### Fixed
- Corrected OSS tool tier classification in runtime/catalog metadata for: `assess_route`, `breakline_mapping`, `local_hypsometric_analysis`, `low_points_on_headwater_divides`, `shadow_animation`, `shadow_image`, `skyline_analysis`, `smooth_vegetation_residual`, `topo_render`, `topographic_hachures`, and `topographic_position_animation`.
- Re-synced resolved taxonomy exports after tier corrections so Python/R/QGIS frontend consumers see consistent license-tier and catalog metadata.

### Release Checklist (WbW-Py)
- [ ] Document user-visible API changes (new methods, removed aliases, signature changes).
- [ ] Document tool-catalog changes (added/removed tools, category moves, tier changes).
- [ ] Document typing/stub updates (`whitebox_workflows.pyi`) when signatures or options changed.
- [ ] Document frontend alignment work when taxonomy/runtime exports were regenerated (Python/R/QGIS).
- [ ] Document compatibility/migration notes for renamed behavior or default changes.
- [ ] Record validation performed (for example `cargo check -p wbtools_oss`, taxonomy sync run, smoke tests).