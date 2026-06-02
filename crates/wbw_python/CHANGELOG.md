# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Changed
- Added epoch-aware reprojection parameters to Python-facing raster/vector/lidar
  reprojection methods, including `coordinate_epoch`, optional source/target
  reference epochs, explicit `operation_code`, `prefer_official_operation`, and
  `epoch_policy` routing control.
- Updated `whitebox_workflows.pyi` reprojection signatures (single and batch)
  to include the new epoch-aware parameters.

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