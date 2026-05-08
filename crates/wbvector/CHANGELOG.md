# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- `VectorReprojectOptions::warn_on_area_of_use_mismatch` (default `false`) and
	`VectorReprojectOptions::with_area_of_use_warning(bool)` to emit non-fatal
	warnings when sampled layer extent points appear outside source/destination
	CRS area of use definitions during vector reprojection.

## [0.1.2] - 2026-05-07

### Added
- Introduced a new in-process vector memory store module at `wbvector::memory_store` for zero-disk vector handoff between components.
- Added memory-path API for vectors with `memory://vector/<id>` handles, including:
	`VECTOR_MEMORY_PREFIX`, `vector_is_memory_path`, `vector_path_to_id`, and `make_vector_memory_path`.
- Added vector store lifecycle API:
	`put_vector`, `put_vector_arc`, `get_vector_arc_by_id`, `get_vector_arc_by_path`, `get_vector_by_id`,
	`replace_vector_by_id`, `replace_vector_by_path`, `remove_vector_by_id`, `remove_vector_by_path`,
	`clear_vectors`, and `vector_count`.
- Added unit tests covering vector memory-store put/get/remove/clear behavior.

### Fixed
- GeoPackage writer now treats `fid`/`FID` as reserved and ignores user-added schema fields with that name when creating table columns.
- Prevents malformed GeoPackage outputs caused by duplicate `fid` columns, so tools that auto-add a `FID` attribute can write valid `.gpkg` layers without per-tool changes.
- Fixed a severe GeoPackage write-time performance issue in the pure-Rust SQLite engine where each `INSERT` re-scanned the full table to compute the next rowid.
- GeoPackage writes now use a cached per-table next-rowid value, removing quadratic insert growth for large layers (e.g., dense dissolved polygon outputs).

## [0.1.1] - 2026-04-14

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
