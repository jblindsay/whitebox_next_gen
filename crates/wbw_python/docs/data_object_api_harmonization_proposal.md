# Data Object API Harmonization Proposal

Date: 2026-04-08
Scope: whitebox_next_gen Python API in wbw_python

## Problem Summary

The current object model is functional but asymmetric:

- Raster exposes metadata through Raster.configs().
- Vector and Lidar expose metadata through several ad hoc methods.
- Creation helpers exist for Raster, but not for Vector and Lidar.
- Write and deep_copy behavior for Vector and Lidar are path-copy oriented and format-agnostic.

This asymmetry makes the API harder to learn and reason about.

## Current State (As Implemented)

### Read

- wbe.read_raster(path) -> Raster
- wbe.read_vector(path) -> Vector
- wbe.read_lidar(path) -> Lidar

This is already harmonized and should remain.

### Metadata

- Raster: Raster.configs() returns RasterConfigs snapshot with grid and stats fields.
- Vector: metadata is split across feature_count(), attribute_fields(), attribute_field_names(), crs_wkt(), crs_epsg(), file methods.
- Lidar: metadata is split across crs_wkt(), crs_epsg(), file methods.

### Vector Attributes (Current)

Current vector attribute access is already fairly capable, but naming and flow are not yet part of a harmonized cross-type data API.

Read-side methods:

- feature_count()
- attribute_field_names()
- attribute_fields()
- get_attributes(feature_index)
- get_attribute(feature_index, field_name)

Write-side methods:

- set_attribute(feature_index, field_name, value)
- set_attributes(feature_index, values_dict)
- add_attribute_field(name, field_type, nullable, width, precision, default_value)

Behavior note:

- set_attribute/set_attributes/add_attribute_field perform in-place edits to the current vector path by reading the layer, mutating it, and writing it back to the same dataset path.

### Create

- Raster has create-from-base helpers:
  - wbe.new_raster_from_base_raster(...)
  - wbe.new_raster_from_base_vector(...)
- Vector and Lidar currently have no equivalent generic constructors in WbEnvironment.

### Write and Copy

- Raster write supports memory-backed and file-backed cases via write_raster.
- Vector write currently copies one path to another path.
- Lidar write currently copies one path to another path.
- Vector.deep_copy also currently copies one path.

## Why Shapefile Export Is Currently Incomplete

A shapefile dataset is a multi-file dataset, commonly including:

- .shp geometry
- .shx index
- .dbf attributes
- .prj projection
- optional sidecars (.cpg, .sbn, .sbx, and others)

Current vector write/deep_copy code path is single-path copy behavior. If the input is a shapefile and only the .shp path is copied, sidecar files are not guaranteed to be copied with it. This can lead to:

- missing attributes (if .dbf is missing),
- missing projection info (if .prj is missing),
- reduced interoperability in GIS tools.

In short: single-file vector formats are fine with this behavior, but shapefile exports need dataset-aware copy/write logic.

## Vector Format Copy Risk Matrix

The current write_vector/deep_copy implementation is a single-path file copy. Risk therefore depends on whether a format is truly single-file.

Lower risk with current path-copy approach (typically single-file):

- FlatGeobuf (.fgb)
- GeoJSON (.geojson/.json)
- GeoPackage (.gpkg)
- GeoParquet (.parquet, when feature enabled)
- GML (.gml)
- GPX (.gpx)
- KML (.kml)
- KMZ (.kmz, when feature enabled)
- OSM PBF (.osm.pbf, when feature enabled)

Higher risk with current path-copy approach (multi-file dataset semantics):

- Shapefile (.shp + sidecars)
- MapInfo MIF/MID pair (.mif + .mid)

Design implication:

- write_vector/deep_copy must detect dataset-style formats and execute format-aware export/copy behavior rather than raw single-path filesystem copy.
- For true single-file formats, raw copy remains an acceptable fast-path.

## Harmonized API Direction

### 1) Standardize Metadata Access

Introduce a single, discoverable metadata method on each object:

- Raster.metadata() -> RasterMetadata
- Vector.metadata() -> VectorMetadata
- Lidar.metadata() -> LidarMetadata

Guidelines:

- Keep object-specific fields where needed.
- Include a shared core shape:
  - file_path, file_name, exists, file_size_bytes, modified_unix_seconds,
  - crs_wkt, crs_epsg.
- Preserve Raster.configs() for compatibility initially, but deprecate it in docs in favor of metadata().

### 1b) Standardize Vector Attribute API

Keep existing vector attribute methods for compatibility, and add a clearer, grouped surface:

- vector.schema() -> VectorSchemaMetadata
- vector.attributes(feature_index) -> dict
- vector.attribute(feature_index, field_name) -> scalar
- vector.update_attributes(feature_index, values_dict) -> None
- vector.update_attribute(feature_index, field_name, value) -> None
- vector.add_field(...) -> None

Compatibility mapping:

- attributes() delegates to get_attributes()
- attribute() delegates to get_attribute()
- update_attributes() delegates to set_attributes()
- update_attribute() delegates to set_attribute()
- add_field() delegates to add_attribute_field()

This keeps users productive immediately while giving the API a more coherent naming system.

### 2) Keep Reading Unified

Retain current read entry points:

- wbe.read_raster
- wbe.read_vector
- wbe.read_lidar

Optional future convenience:

- wbe.read(path) returning a typed object by extension/inspection.

### 3) Harmonize Creation Patterns

Keep specialized raster creation methods, but add generic factory naming:

- wbe.create_raster(...)
- wbe.create_vector(...)
- wbe.create_lidar(...)

For first iteration:

- create_raster can delegate to existing base-raster/base-vector workflows.
- create_vector and create_lidar can start with minimal empty/template-from-source variants.

### 4) Harmonize Write Behavior

Unify object-level save semantics:

- obj.save(output_path, options...)

And environment-level wrappers:

- wbe.write_raster(raster, output_path, ...)
- wbe.write_vector(vector, output_path, ...)
- wbe.write_lidar(lidar, output_path, ...)

But for Vector, make writing format-aware:

- If source or destination is shapefile, perform dataset-aware export/copy including required sidecars.
- For other single-file formats, path copy can remain valid for deep copy cases.

## Proposed Naming Conventions

- Metadata retrieval:
  - metadata() on each data object type.
- Creation:
  - create_* for generic constructors.
  - keep specialized new_raster_from_* as explicit advanced helpers.
- Writing:
  - save() on objects plus write_* wrappers on environment.

This gives users one mental model:

- read -> metadata -> process -> save

## Migration Strategy

Phase 1 (non-breaking):

- Add metadata() for Raster, Vector, Lidar.
- Keep Raster.configs() unchanged.
- Add docs that metadata() is preferred.
- Add vector attribute aliases (attribute/attributes/update_attribute/update_attributes/add_field/schema) that wrap existing methods.

Phase 2 (behavior hardening):

- Make Vector write/deep_copy dataset-aware for shapefiles.
- Make Vector write/deep_copy dataset-aware for MapInfo MIF/MID pairs as well.
- Add tests for sidecar-preserving behavior.

Phase 3 (ergonomics):

- Add create_vector and create_lidar minimal constructors.
- Optionally add object-level save() methods and keep write_* wrappers.

## Test Matrix (Minimum)

Metadata:

- Raster.metadata fields match existing configs/grid/CRS expectations.
- Vector.metadata includes feature_count and schema summary.
- Lidar.metadata includes point_count (if available), bounds (if available), CRS and file info.

Vector attributes:

- Read one value and one row by index.
- Update one value and multiple values for a row.
- Add a new field with default value and verify persistence.
- Verify alias methods and legacy methods return identical results.

Write:

- GeoTIFF write round-trip.
- GeoJSON/GPKG write round-trip.
- Shapefile write preserves .shp/.shx/.dbf/.prj at minimum.
- MapInfo MIF write preserves the required .mif/.mid pair.
- LAS/LAZ write round-trip.

Compatibility:

- Existing scripts using Raster.configs() keep working.

## Recommended Immediate Next Changes

1. Add metadata() methods and metadata classes for all three object types.
2. Update the .pyi stubs so metadata() is strongly typed.
3. Add shapefile-aware vector write and vector deep_copy behavior.
4. Keep configs() on Raster as compatibility alias for now.
