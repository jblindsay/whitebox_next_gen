# HDF Module Boundaries

Date: 2026-05-31
Owner: wbhdf core

## Purpose

Define clear internal boundaries between the HDF5-first decoder path and the HDF4/HDF-EOS2 feasibility path so integration can advance without coupling unrelated format concerns.

## Boundary Model

### HDF5 Path (Primary Runtime Decode)

Modules:
- `superblock.rs`
- `object_header.rs`
- `btree.rs`
- `dataset.rs`
- `datatypes.rs`
- `filters.rs`
- `compare.rs`

Responsibilities:
- Parse HDF5 container structures (superblocks, object headers, chunk indexes).
- Decode payload bytes for supported datatype/layout/filter combinations.
- Apply fill/nodata mapping for validated numeric paths.
- Provide reference-comparison helpers for fixture validation.

Current state:
- Active decode path for targeted ATL08 and GEDI checks.
- VIIRS baseline checkpoint currently validated through contiguous HDF5 dimension data.

### HDF4/HDF-EOS2 Path (Feasibility + Staged Expansion)

Module:
- `hdf4.rs`

Responsibilities:
- Validate HDF4 file signature for candidate MODIS products.
- Parse embedded EOS metadata (`StructMetadata.0`) for grid and data-field enumeration.
- Expose structured metadata summaries (`GridName`, `DataFieldName`, datatype, dimension list) used to drive staged SDS decode implementation.

Current state:
- Metadata-only feasibility parser and fixture-backed assertions for MOD09/MOD11/MOD13 families.
- No SDS payload-array decode yet.

## Shared Contracts

- Unified crate-level error/result types from `error.rs` (`WbhdfError`, `WbhdfResult`).
- Fixture discovery through `fixtures.rs` environment-based roots:
  - `WBHDF_FIXTURE_DIR` (GEDI/ATL08)
  - `WBHDF_VIIRS_FIXTURE_DIR`
  - `WBHDF_MODIS_FIXTURE_DIR`
- Integration tests in `tests/integration_tests.rs` are format-partitioned by fixture family and should remain independently skippable when fixtures are absent.

## Integration Intent

- `wbhdf` remains the container decode core.
- `wbraster` and `wblidar` should consume validated format-specific adapters rather than directly coupling to raw parser internals.
- HDF4 SDS decode capability can be added incrementally in `hdf4.rs` while preserving stability of the existing HDF5 decode path.
