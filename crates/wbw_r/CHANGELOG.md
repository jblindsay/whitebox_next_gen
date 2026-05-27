# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- None yet.

### Changed
- None yet.

### Fixed
- None yet.

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
