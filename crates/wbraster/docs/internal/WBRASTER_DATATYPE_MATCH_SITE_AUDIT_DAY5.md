# wbraster DataType Match-Site Audit (Day 5)

Date: 2026-05-31
Method: grep inventory of DataType match/variant usage under src + tests

## Classification Legend

- behavior-sensitive: Changes can affect runtime decoding/encoding semantics.
- compile-only update: Add enum variants and update exhaustive matches; no intended behavior change.
- deferred update: Not required in first complex enablement pass; track for later validation.

## behavior-sensitive files

- crates/wbraster/src/raster.rs
- crates/wbraster/src/formats/dted.rs
- crates/wbraster/src/formats/envi.rs
- crates/wbraster/src/formats/er_mapper.rs
- crates/wbraster/src/formats/esri_ascii.rs
- crates/wbraster/src/formats/esri_binary.rs
- crates/wbraster/src/formats/esri_float.rs
- crates/wbraster/src/formats/geopackage.rs
- crates/wbraster/src/formats/geotiff.rs
- crates/wbraster/src/formats/grass_ascii.rs
- crates/wbraster/src/formats/hfa.rs
- crates/wbraster/src/formats/idrisi.rs
- crates/wbraster/src/formats/jpeg2000.rs
- crates/wbraster/src/formats/pcraster.rs
- crates/wbraster/src/formats/png_jpeg.rs
- crates/wbraster/src/formats/saga.rs
- crates/wbraster/src/formats/surfer.rs
- crates/wbraster/src/formats/xyz.rs
- crates/wbraster/src/formats/zarr.rs
- crates/wbraster/src/formats/zarr_v3.rs

Rationale:
- These modules map file format type metadata to DataType and encode/decode payload bytes.
- Incorrect handling can silently corrupt values or mis-map nodata/fill semantics.

## compile-only update candidates

- crates/wbraster/src/memory_store.rs
- crates/wbraster/tests/integration.rs

Rationale:
- Predominantly construction/default type uses and assertions.
- Expect straightforward variant additions and non-semantic compile fixes.

## deferred update candidates

- crates/wbraster/src/packages/sentinel1_safe.rs

Rationale:
- Current paths are strongly F32-oriented and tied to sensor-specific behavior.
- Complex support in this package should be addressed in a dedicated sensor pathway
  review after core raster complex storage/accessor support lands.

## Notes

- This audit is file-level (not per-match-line) to keep Day 5 planning actionable.
- During implementation, behavior-sensitive files should be updated first with explicit
  tests to avoid accidental scalar-regression behavior changes.
