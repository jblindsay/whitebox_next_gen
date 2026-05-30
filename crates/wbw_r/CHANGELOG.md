# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [2.0.5] - 2026-05-30

### Added
- Added schema-aware metadata emission path for R-side tool discovery payloads, including support for canonical per-parameter `schema` objects.

### Changed
- Updated `list_tools_json(...)` and `get_tool_metadata_json(...)` serialization to consume explicit backend schema maps for migrated tools.
- Updated manifest-parameter reconstruction for empty manifests to preserve backend metadata ordering across the OSS/PRO catalog, with explicit legacy ordering overrides for flow-family tools.
- Updated metadata enrichment to backfill missing parameter descriptions/required flags from tool metadata when doc-derived maps are absent or incomplete.

### Fixed
- Fixed stream-tool metadata typing drift for pilot tools (`extract_streams`, `vector_stream_network_analysis`) by consuming backend-authored typed schemas.
- Fixed `d8_flow_accum` metadata parameter ordering regression so frontend and binding consumers receive legacy-logical ordering (`input`, `output`, then processing options).
- Fixed generic parameter-description fallback regressions (for example ambiguous `input`) by ensuring exported metadata includes domain-specific descriptions where available.

### Release Checklist (WbW-R)
- [ ] Document user-visible API changes (new session helpers, wrapper methods, signature changes).
- [ ] Document discovery/catalog changes (search/list/describe behavior, metadata schema fields).
- [ ] Document R package facade/NAMESPACE export changes.
- [ ] Document typed object wrapper updates (`wbw_raster`, `wbw_vector`, `wbw_lidar`, `wbw_sensor_bundle`).
- [ ] Document compatibility/migration notes for renamed behavior or removed aliases.
- [ ] Record validation performed (for example `cargo check -p wbw_r`, package smoke checks).

## [2.0.3] - 2026-05-27

### Added
- Added crate-level changelog tracking for `wbw_r` with a repeatable release checklist.
- Added canonical metadata/info discovery APIs at the runtime layer:
  - `get_tool_metadata_json(...)`
  - `get_tool_info_json(...)`
  - `get_tool_metadata_json_with_options(...)`
  - `get_tool_info_json_with_options(...)`

### Changed
- Aligned runtime tool-manifest payloads with schema-first frontend consumption by exposing canonical manifest metadata via `manifest_with_io_schema_json(...)`.
- Aligned R package facade exposure with runtime metadata APIs so package consumers can call `get_tool_metadata_json(...)` and `get_tool_info_json(...)` directly.
- Updated R session helper behavior to provide `session$get_tool_metadata_json(...)` and `session$get_tool_info_json(...)` through the same discovery path used by `wbw_describe_tool(...)`.
- Updated canonical R manuals to include discovery/metadata info API references and schema-aware guidance.

### Fixed
- Reduced Python/R/QGIS metadata drift by keeping R-side metadata/info discovery aligned with the runtime schema used by frontend consumers.
