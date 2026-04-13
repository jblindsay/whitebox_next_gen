# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Fixed
- Populate `Layer.geom_type` when reading Shapefiles by mapping the `.shp` header shape type to `GeometryType`.
- Infer and set `Layer.geom_type` when reading GeoJSON `FeatureCollection` layers from the first feature with geometry.
- Set `Layer.geom_type` for GeoJSON single `Feature` and bare-geometry inputs.
- Infer and set `Layer.geom_type` when reading GeoParquet layers from the first feature geometry.
- Resolves downstream false validation failures in tools that require line geometries (e.g., network readiness workflows) when inputs are valid line-based datasets.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
