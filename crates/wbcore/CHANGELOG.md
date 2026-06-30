# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [0.2.0] - 2026-06-30

### Added
- Added canonical typed tool-parameter schema model types in `wbcore` (input/output/dataset/cardinality/scalar/vector-geometry/field/enum).
- Added `ToolFieldSchema` with parent layer reference and optional vector geometry constraints for field-parameter type-safety.
- Added ergonomic schema builder helpers (`ToolParamSchema::input_raster`, `input_vector`, `output_raster`, `field(parent, geometry)`, scalar and enum helpers) to reduce tool-authoring boilerplate.
- Added `manifest_with_param_schema_json(...)` for schema-aware metadata emission with compatibility fields.

### Changed
- Kept `manifest_with_io_schema_json(...)` backward-compatible by routing through the new schema-aware serializer with an empty schema map.

### Testing
- Schema model types and builders enable 40+ tools across wbtools_oss and wbspatialstats to define field parameter metadata for QGIS dropdown integration.