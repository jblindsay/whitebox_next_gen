# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Fixed
- Read GeoPackage layers correctly when table schemas use bracket-quoted identifiers and uppercase `FID` column names.
- Preserve GeoPackage feature IDs by falling back to SQLite rowids when `INTEGER PRIMARY KEY` payload values are stored as `NULL` aliases.
- Resolves false geometry-loss symptoms in real parcel fixtures where valid multipolygon GeoPackage layers were previously read with missing geometries and synthetic fallback feature IDs.
- Populate `Layer.geom_type` when reading Shapefiles by mapping the `.shp` header shape type to `GeometryType`.
- Infer and set `Layer.geom_type` when reading GeoJSON `FeatureCollection` layers from the first feature with geometry.
- Set `Layer.geom_type` for GeoJSON single `Feature` and bare-geometry inputs.
- Infer and set `Layer.geom_type` when reading GeoParquet layers from the first feature geometry.
- Resolves downstream false validation failures in tools that require line geometries (e.g., network readiness workflows) when inputs are valid line-based datasets.

### Tests
- Extended `real_mississauga_parcel_fixture_decodes_when_enabled` GeoPackage smoke test to assert that decoded multipolygon ring coordinates produce non-zero shoelace areas (100/100 sample), confirming end-to-end geometry fidelity for downstream calibration workflows.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
