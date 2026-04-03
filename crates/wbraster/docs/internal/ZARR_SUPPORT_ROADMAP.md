# Zarr Support Roadmap for wbraster

## Purpose

Define a practical path for advancing `wbraster` Zarr support from its current MVP state to a meaningfully broader, more interoperable implementation without turning `wbraster` into a distributed object-store framework.

This document is intended to guide implementation sequencing, testing, and documentation updates.

---

## Current State

`wbraster` currently supports:

- Local filesystem Zarr stores only
- Zarr v2 and Zarr v3 read/write
- 2D arrays and 3D arrays in canonical `(band, y, x)` form
- Regular chunk grids
- C-order chunk traversal
- `bytes` codec plus optional compressors:
  - `zlib`
  - `gzip`
  - `zstd`
  - `lz4`
- Basic geospatial metadata propagation through ad hoc metadata keys
- Synthetic and Python-style interoperability tests for several v3 combinations

This is already a solid MVP, but interoperability is still intentionally narrow.

---

## Main Limitations Today

### Implementation limitations

- Zarr v3 codec pipelines are only partially supported.
- Some advanced/extension codec combinations remain unsupported.
- Unknown or extension codecs are not handled in a spec-forward way.
- Support is limited to local filesystem stores.
- Arbitrary N-dimensional arrays are not supported.
- Metadata interoperability is still based on a small, Whitebox-oriented set of conventions.
- Strict-vs-lenient validation mode is not yet implemented.

### Practical interoperability limitations

- Limited coverage against real-world external producers beyond synthetic fixtures.
- No explicit strict-vs-lenient validation modes.
- No support for higher-level geospatial Zarr conventions such as richer multi-scale metadata structures.

---

## Design Constraints

These should continue to govern the work:

1. Keep `wbraster` focused on raster I/O, not distributed processing.
2. Prioritize real interoperability wins over broad-but-shallow spec coverage.
3. Preserve clean `Raster` semantics as the public data model.
4. Prefer deterministic filesystem-first behavior before introducing store abstractions.
5. Add validation and diagnostics as support broadens so failures stay understandable.

---

## Recommended Roadmap

## Phase 1: Interoperability Hardening

### 1. Implement Zarr v3 `transpose` codec support

This is the single highest-value next step.

Why:
- The README already calls this out as the next interoperability target.
- The current implementation explicitly rejects it.
- More external Zarr producers become readable once this is supported.

Primary files:
- `crates/wbraster/src/formats/zarr_v3.rs`
- `crates/wbraster/tests/integration.rs`

Acceptance criteria:
- Reader can ingest v3 stores whose codec pipeline contains `bytes -> transpose -> compressor`.
- Tests cover both little-endian and big-endian byte representations where practical.
- Clear error messages remain for unsupported transpose configurations beyond the initial implementation.

### 2. Expand real-world fixture coverage

Add conformance fixtures created outside `wbraster`, especially from:
- Python `zarr`
- `xarray`
- `rioxarray`
- any geospatial Zarr writer that uses v3 metadata and codec pipelines in realistic ways

Why:
- Synthetic fixtures are necessary but not sufficient.
- Real producers tend to expose metadata and codec details that synthetic tests miss.

Acceptance criteria:
- Fixture-backed tests cover at least several external producer combinations.
- README claims remain aligned with what is actually verified.

Status update (2026-04-01):
- Added env-gated external fixture smoke tests in `tests/integration.rs`:
  - `WBRASTER_EXTERNAL_ZARR_V2_FIXTURE`
  - `WBRASTER_EXTERNAL_ZARR_V3_FIXTURE`
- Added optional parity env vars for stronger external checks:
  - `*_EXPECT_ROWS`, `*_EXPECT_COLS`, `*_EXPECT_NODATA`, `*_EXPECT_CELL`
- Added candidate source tracker: `docs/internal/zarr_external_fixture_candidates.md`.
- Verified public geospatial v2 source options (AORC on AWS Open Data, IPFS-hosted geospatial examples).
- Added tiny local v2 and v3 fixtures for env-gated parity execution under `tests/fixtures_external/`.
- Verified both external parity tests pass locally in one run with expectations enabled:
  - `external_zarr_v2_fixture_smoke_local_path`
  - `external_zarr_v3_fixture_smoke_local_path`
- Still pending for full closure: stable anonymous/public geospatial v3 source selection and repeatable fixture acquisition (checked-in tiny external fixtures or scripted downloads with provenance).

### 3. Tighten validation and diagnostics

Add explicit checks and better errors for:
- shape and chunk-shape inconsistencies
- unsupported `node_type`
- unsupported chunk-grid variants
- unsupported dimension arrangements
- conflicting geospatial metadata
- unsupported codec pipelines

Acceptance criteria:
- Unsupported stores fail early with actionable messages.
- Corrupt metadata and unsupported metadata are distinguished cleanly.

Status update (2026-04-01):
- Implemented explicit root/shape/chunk-shape/node-type validation and actionable errors in `src/formats/zarr_v3.rs`.
- Added codec-pipeline validation with explicit unsupported-codec failures.
- Added transpose-order structural validation (length, range, uniqueness).

---

## Phase 2: Writer and Metadata Improvements

### 1. Add v2 writer chunk controls

The v3 writer already supports configurable chunk rows and columns; v2 should gain equivalent user control.

Suggested metadata keys:
- `zarr_chunk_rows`
- `zarr_chunk_cols`
- continue supporting `zarr_chunk_bands`

Why:
- Improves symmetry across v2 and v3.
- Lets users choose chunking appropriate to access patterns and file size.

Acceptance criteria:
- v2 writer respects user-specified chunk rows/cols.
- Defaults remain backward compatible.
- Tests verify both default and overridden chunking.

Status update (2026-04-01):
- Implemented in `src/formats/zarr.rs` via metadata keys:
  - `zarr_chunk_rows`
  - `zarr_chunk_cols`
  - `zarr_chunk_bands`
- Covered by tests including `zarr_v2_writer_respects_chunk_row_col_metadata`.

### 2. Improve geospatial metadata interoperability

Current geospatial metadata support is serviceable, but broader compatibility should be added carefully.

Potential improvements:
- recognize more CRS aliases and transform metadata layouts
- preserve more source metadata rather than normalizing away information
- add optional metadata pass-through for known external conventions

Acceptance criteria:
- External fixtures with alternate but common CRS metadata representations are read correctly or fail clearly.

Status update (2026-04-01):
- Implemented CRS alias handling for `crs_epsg`/`epsg`, `crs_wkt`/`spatial_ref`, and `crs_proj4`/`proj4` in v2 and v3 readers.
- Added affine `transform` fallback for origin/cellsize inference when `x_min`/`y_min`/cell-size keys are absent (v2 and v3 readers).
- Added fixture-backed v2 tests for alternate CRS alias keys and transform-driven origin/cellsize recovery.
- Added fixture-backed v3 integration tests for alternate CRS alias keys and transform-only georeferencing metadata.
- Added EPSG string alias parsing from `crs` values like `"EPSG:3857"` (v2 and v3 readers).
- Added object-style CRS EPSG parsing from `crs` JSON values (e.g., nested `{"properties": {"name": "EPSG:32618"}}`) in v2 and v3 readers.
- Added authority/code CRS object parsing (e.g., `{"id": {"authority": "EPSG", "code": "3035"}}`) in v2 and v3 readers.
- Added OGC URN/URL CRS string parsing for EPSG extraction (e.g., `urn:ogc:def:crs:EPSG::26917`, `https://www.opengis.net/def/crs/EPSG/0/3395`) in v2 and v3 readers.
- Added `GeoTransform` string fallback support for georeferencing recovery (v2 and v3 readers).
- Added strict negative-path validation/tests for malformed and conflicting geospatial metadata (clear failures for invalid `GeoTransform` and for disagreements between explicit georef keys and transform metadata).
- Added fixture-backed coverage for object-style CRS metadata in both v2 unit tests and v3 producer-style integration tests.
- Added fixture-backed coverage for authority/code CRS object metadata in both v2 unit tests and v3 producer-style integration tests.
- Added fixture-backed coverage for OGC URN/URL CRS string metadata in both v2 unit tests and v3 producer-style integration tests.
- Added fixture-backed coverage for CF/grid-mapping named-object CRS layouts (EPSG/WKT/proj4 extraction from a mapping object referenced by `grid_mapping`) in both v2 unit tests and v3 producer-style integration tests.
- Phase 2 Step 2 status: complete for the current interoperability target set (aliases, transform/geotransform fallback, object-style CRS, authority/code CRS, OGC URN/URL CRS, CF/grid-mapping named-object CRS, and strict malformed/conflict handling).
- Future metadata breadth can continue in Phase 3 as additional producer conventions are encountered.

### 3. Add strict and lenient validation modes

Suggested behavior:
- strict mode: fail on unsupported codecs or ambiguous metadata
- lenient mode: ignore non-critical unsupported metadata and read what is safely readable

Why:
- Production users often want deterministic enforcement.
- Exploratory users often want best-effort reads.

Acceptance criteria:
- Validation mode is explicit and documented.
- Error behavior is predictable and test-covered.

Status update (2026-04-01):
- Implemented explicit validation-mode control via metadata key `zarr_validation_mode` in Zarr attributes (`strict` default, optional `lenient`) for v2 and v3 readers.
- Strict mode preserves existing fail-fast behavior for malformed/conflicting geospatial metadata.
- Lenient mode performs best-effort geospatial metadata handling (ignores conflicting/invalid transform metadata instead of failing read).
- Added v2 unit tests and v3 integration tests for lenient-mode conflict and invalid-geotransform scenarios.

Status update (2026-04-01, close-out):
- Audited all remaining `RasterError` return sites in both v2 and v3 readers for mode-gating candidates.
- All remaining hard validation errors (unsupported codec, unsupported compressor, unsupported chunk_key_encoding, unsupported chunk_grid, unsupported data_type, zarr_format mismatch, malformed transpose order) are structural prerequisites for decoding: the chunk data is literally unreadable without them. There is no safe lenient fallback.
- The "extend to non-geospatial categories" follow-on has no concrete safe targets. Geospatial metadata soft cases were the correct and complete scope.
- **Phase 2 Step 3 status: complete.**

---

## Phase 3: Broader Format Capability

### 1. Support selected higher-level metadata conventions

Candidates:
- richer v3 metadata interoperability
- selected multi-scale conventions where they can be mapped cleanly to `Raster`
- more explicit dimension metadata handling

Caution:
- Multi-scale groups should not be added until the crate has a clear internal model for selecting one resolution level as a `Raster`.

Status update (2026-04-01):
- Added CF-style `_FillValue` and `missing_value` attribute keys as nodata fallbacks in both v2 and v3 readers.
  Lookup precedence: explicit `nodata` attr → `_FillValue` → `missing_value` → Zarr `fill_value` → default -9999.
  Covered by 6 tests (3 v2 unit, 3 v3 integration).
- Added `dimension_names` semantic validation in the v3 reader.
  In strict mode a 3D array whose `dimension_names` place spatial axes before the band axis
  (`["y","x","band"]` etc.) is rejected with an actionable error directing the user to add a
  transpose codec or set lenient mode.  All 2D layouts and unrecognized names are accepted.
  In lenient mode the check is bypassed and a best-effort read proceeds.
  Covered by 5 integration tests (standard 2D, lat/lon 2D, unrecognized 2D, band-last strict
  fails, band-last lenient succeeds).
- Implemented multi-scale group read support (OME-NGFF 0.5) for both v2 and v3 stores.
  Level selection policy (Option A): `zarr::read()` always opens level 0 (full-resolution) by
  default; callers that need a coarser level point the path directly at the sub-array directory
  (e.g. `store.zarr/1`).
  - `is_v3_group()` / `is_v2_group()` — lightweight group-root detectors.
  - `discover_multiscale_levels_v3()` / `discover_multiscale_levels_v2()` — level path
    discovery: OME-NGFF `multiscales[0].datasets[].path` with a consecutive-numeric-subdir
    fallback for plain groups without explicit OME attributes.
  - `select_level()` — returns `levels[0]` (finest resolution, index 0 per OME-NGFF convention).
  - `zarr::read()` entry-point rewritten with 4-branch dispatch:
    v3 group → discover + select → `zarr_v3::read_from_dir`;
    v3 array → `zarr_v3::read_from_dir`;
    v2 group → discover + select → `read_from_dir`;
    v2 array (default) → `read_from_dir`.
  Covered by 8 integration tests:
    default full-res group open (v2 + v3), level-1 via direct path (v2 + v3),
    numeric-fallback when OME attrs absent (v2 + v3), empty group returns error (v3),
    plain array unaffected by group-detection (v3 regression guard).
  All 80 tests pass.
- Remaining P3/S1 candidates: dimension-name-guided axis permutation for 3D stores without a
  codec-level transpose.

### 2. Consider a store abstraction only if it solves a real near-term problem

Examples that may justify later work:
- HTTP-backed readonly access for cloud-hosted Zarr
- object-store access through a narrow abstraction layer

This should be deferred until local filesystem interoperability is much stronger.

### 3. Optional performance pass

Likely targets:
- reduce allocation churn during chunk decode/encode
- parallelize chunk read/write work where it is clearly beneficial
- improve large multichunk read throughput

This should follow interoperability work, not precede it.

---

## Priority Order

Implementation order (✅ = complete):

1. ✅ v3 `transpose` codec support
2. ✅ Real external fixture coverage (env-gated smoke tests + tiny local fixtures)
3. ✅ Validation and diagnostic hardening
4. ✅ v2 writer chunk controls
5. ✅ Broader geospatial metadata interoperability
6. ✅ Strict/lenient validation mode
7. ✅ Higher-level metadata conventions (CF nodata, `dimension_names` validation, OME-NGFF multi-scale groups)
8. Store abstraction or remote access, if still justified later (deferred — FUSE mount is the recommended workaround today)

---

## Suggested Concrete Work Items

### Work item A: Transpose codec MVP — ✅ complete

Scope:
- implement the common transpose case used by real producer pipelines
- keep initial implementation narrow and explicit
- reject unsupported transpose orders with precise errors

Outcome:
- F-order, C-order, and explicit permutation supported in v3 reader.
- Covered by 5 integration tests (C-order noop, F-order string single-chunk, F-order multichunk, explicit permutation, big-endian + gzip combination).

### Work item B: External producer fixture suite — ✅ complete

Scope:
- add fixture generator scripts or checked-in minimal fixtures
- verify read parity against expected raster values and metadata

Outcome:
- Env-gated smoke tests: `WBRASTER_EXTERNAL_ZARR_V2_FIXTURE` and `WBRASTER_EXTERNAL_ZARR_V3_FIXTURE`.
- Optional parity env vars: `*_EXPECT_ROWS`, `*_EXPECT_COLS`, `*_EXPECT_NODATA`, `*_EXPECT_CELL`.
- Tiny local v2 and v3 fixtures under `tests/fixtures_external/`.
- External fixture candidate tracker in `docs/internal/zarr_external_fixture_candidates.md`.

### Work item C: v2 writer chunk controls — ✅ complete

Scope:
- read metadata keys `zarr_chunk_rows` and `zarr_chunk_cols` in v2 writer
- preserve existing default behavior when unspecified

Outcome:
- `zarr_chunk_rows`, `zarr_chunk_cols`, `zarr_chunk_bands` honoured in v2 writer.
- Covered by `zarr_v2_writer_respects_chunk_row_col_metadata` integration test.

### Work item D: Validation mode — ✅ complete

Scope:
- add internal configuration surface for strict vs lenient behavior
- keep default behavior conservative and backward compatible

Outcome:
- `zarr_validation_mode` key accepted in `.zattrs` (v2) and `zarr.json` attributes (v3).
- Strict mode (default) preserves all existing fail-fast behavior.
- Lenient mode bypasses soft geospatial metadata checks and `dimension_names` validation.
- Covered by v2 unit tests and v3 integration tests for conflict and invalid-GeoTransform scenarios.

### Work item E: Multi-scale group support — ✅ complete

Scope:
- detect OME-NGFF group roots for both v2 and v3
- select finest resolution level by default; allow direct sub-array path as escape hatch

Outcome:
- `zarr::read()` 4-branch dispatch: v3 group → v3 array → v2 group → v2 array.
- `discover_multiscale_levels_v2/v3`: OME-NGFF `multiscales[0].datasets[].path` with
  consecutive-numeric-subdir fallback.
- `select_level()` always returns `levels[0]` (finest/index-0 per OME-NGFF convention).
- Covered by 8 integration tests (default open, direct path, no-OME fallback, empty-group error,
  regression guard — both v2 and v3).

---

## Files Likely to Change

Core implementation:
- `crates/wbraster/src/formats/zarr.rs`
- `crates/wbraster/src/formats/zarr_v3.rs`
- `crates/wbraster/src/formats/mod.rs`
- optionally `crates/wbraster/src/raster.rs` if any metadata/config plumbing is needed

Tests:
- `crates/wbraster/tests/integration.rs`
- possibly dedicated Zarr-specific test helpers or fixture utilities

Docs:
- `crates/wbraster/README.md`
- `crates/wbraster/CHANGELOG.md`

---

## Recommended Documentation Updates When Work Starts

When the first substantive Zarr phase lands, update:

1. README Zarr Notes
- expand supported codec pipeline details
- document newly supported metadata conventions
- document any new validation or chunk-control behavior

2. README Known Limitations
- remove items that are no longer true
- keep unsupported remote-store and arbitrary-ND limitations explicit if still true

3. CHANGELOG
- record specific codec, metadata, and fixture-conformance improvements

---

## Assessment of Existing `docs/internal` Files

Current files in `docs/internal` do not appear to overlap with this Zarr roadmap.

Conclusion:
- No existing file in `docs/internal` needs to be updated for this Zarr planning document.
- No existing file in `docs/internal` should be deleted as part of this work.

Rationale:
- There is no pre-existing Zarr plan in that folder.
- The existing documents are about other topics and do not conflict with this roadmap.
- Deletion should only happen if a document is clearly obsolete or superseded, and that is not established here.

One possible future housekeeping item:
- `SENTINEL2_SAFE_READER_PLAN.md` may eventually be better moved to an `archive/` subfolder once its implementation is fully complete and no longer serves as a live plan.
- That is unrelated to the Zarr work and should be a separate cleanup decision, not bundled into this change.

---

## Current Focus and Remaining Work

Phases 1 and 2 are complete. Phase 3 Step 1 is substantially complete. The remaining open items are:

### In-scope P3 continuation

- **Dimension-name-guided axis permutation** — auto-permuting a 3D `(y, x, band)` store without
  a codec-level transpose (medium complexity; needs design work on safe axis discovery heuristics).
- **Stable public external fixture** — selecting a repeatable, citable public geospatial v3 Zarr
  source to anchor the external parity test (currently uses tiny local fixtures).

### Deferred

- **P3/S2 — Store abstraction**: Deferred until local-filesystem interoperability is much stronger.
  A `ChunkSource` trait threading through all metadata and chunk I/O sites in both readers would be
  the minimum structural change; this is a real refactor, not an addition. The FUSE-mount path
  (`rclone mount`) costs nothing and covers most real cloud-access use cases today.
- **P3/S3 — Performance pass**: Reduce allocation churn, parallelize chunk I/O, improve large
  multichunk throughput. Should follow interoperability work.

### README and CHANGELOG

- README Zarr Notes has been updated to reflect current capabilities (all phases).
- CHANGELOG entries for each completed work item should be added before the next crate release.
