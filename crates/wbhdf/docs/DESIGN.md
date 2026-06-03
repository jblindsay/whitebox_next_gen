# DESIGN

Date: 2026-05-31
Status: Active design + implementation notes (updated)

This document is the high-level technical design for the scoped HDF ingestion stack,
covering how `wbhdf` is used by `wblidar` and `wbraster`, what is implemented now,
and what remains intentionally out of scope.

## 1) Design Intent

The stack is intentionally a targeted scientific-product decoder, not a full HDF library.

- Primary goal: remove common pre-conversion steps for high-value products.
- Primary architecture: `wbhdf` as shared decode core; `wblidar` and `wbraster` as consumers.
- Scope control: support only validated layouts and datatype/filter combinations.

## 2) Module Responsibilities

### `wbhdf` (decode core)

- `superblock.rs`
	- HDF5 superblock and root discovery primitives.
- `object_header.rs`
	- HDF5 object-header parsing.
	- v1/v2 message extraction, continuation chunk handling, dataspace/datatype/layout messages.
- `btree.rs`
	- B-tree v1 parsing and chunk-address lookup for chunked datasets.
- `dataset.rs`
	- Dataset-path resolution checks and contiguous bounded reads.
	- Fill-to-nodata helper for `f32` paths.
- `hdf4.rs`
	- Scoped HDF4/HDF-EOS2 metadata probing, dataset-path resolution, and bounded SDS decode paths.

### `wblidar` (Tier 1 lidar integration)

- Uses `wbhdf` for product-aware ingestion paths.
- Current validated paths include initial GEDI and ATL08 canopy-style reads.
- Includes provider/registry dispatch and bounded-memory safeguards.

### `wbraster` (raster-like integration)

- Parses canonical dataset URIs: `container#dataset=/absolute/path`.
- Keeps legacy `:///` alias for compatibility.
- HDF4 path: bounded 2D `DFNT_INT16` SDS materialization.
- HDF5/NetCDF path: metadata-driven contiguous scalar materialization
	(currently `f32`/`f64` widths only), using object-header layout metadata,
	plus bounded chunked recursive scalar fallback via v1 object-header + chunk-index metadata.

## 3) HDF5 Read Strategy (Current)

### 3.1 Metadata path

1. Parse dataset URI and resolve container + dataset path.
2. Probe object headers and continuation chunks.
3. Collect contiguous layout candidates (layout class 1).
4. Rank candidates by:
	 - datatype-width compatibility,
	 - dataspace dimension consistency,
	 - proximity to dataset-path markers.
5. Select best contiguous candidate and decode contiguous payload.
6. If contiguous selection fails, attempt bounded chunked recursive traversal fallback
	(v1 object-header + chunk-index leaf record + scalar decode).

### 3.2 Materialization boundaries

- Supported now in `wbraster`:
	- contiguous scalar datasets with 4-byte or 8-byte elements (`f32`/`f64` decode path).
	- chunked recursive traversal scalar datasets (`f32`/`f64`) through bounded fallback flow,
	  including raw and single-deflate-filter chunk payloads.
- Not supported yet in `wbraster`:
	- complex datatypes,
	- broad CF/NetCDF generalization.

Current unsupported boundary note:
- malformed recursive chunk trees and layouts that violate the staged internal-
	record assumptions are intentionally rejected with explicit diagnostics until
	those shapes are validated and implemented.

### 3.3 Why this shape

This provides immediate practical value for validated raster-like datasets while
avoiding premature expansion into full-layout generalization.

## 4) HDF4/HDF-EOS2 Strategy (Current)

- Focus: MODIS-targeted metadata + bounded SDS bridge behavior.
- Metadata coverage includes path resolution and grid geometry derivation.
- Decode path supports bounded `i16` SDS windows with deterministic diagnostics.
- Full general-purpose HDF4 support is explicitly out of scope.

## 5) Error and Diagnostics Contract

The stack emphasizes deterministic, actionable failure modes:

- missing dataset selector/path,
- unsupported layout/filter/datatype class,
- bounded read/input errors,
- chunk/decode failures with contextual metadata where available.

`wbraster` and `wblidar` should preserve this style at integration boundaries.

## 6) Interop Contract Across Crates

### Stable URI contract

- Canonical: `container_path#dataset=/absolute/path`
- Legacy alias accepted in parsing: `container_path:///absolute/path`

### Consumer separation

- `wblidar` owns lidar product ingestion semantics.
- `wbraster` owns raster-like dataset materialization only.

### CRS responsibility boundary

- `wbhdf` must remain projection-engine agnostic and should not depend on
	`wbprojection`.
- `wbhdf` may parse and expose CRS-relevant metadata tokens (for example WKT,
	EPSG hints, grid georeferencing fields), but must not perform CRS
	normalization, transform selection, or reprojection.
- CRS interpretation and coordinate transformation policy belong to downstream
	consumers (`wbraster` and `wblidar`), which already integrate with
	`wbprojection`.
- This boundary keeps `wbhdf` focused on deterministic container/layout decode,
	reduces coupling, and avoids projection-policy drift inside low-level HDF
	decode paths.

## 7) What Is Deliberately Not Claimed

- Not a full HDF5 spec implementation.
- Not a general-purpose HDF4 library.
- Not broad, default-on support for all scientific HDF/NetCDF products yet.

## 8) Immediate Next Design Step

The next high-value extension is hardening the bounded recursive chunked traversal
materialization to validated multi-chunk HDF5 raster-like materialization in
`wbraster` using existing `wbhdf` chunk-locator/decode primitives, while keeping
the same staged guardrails and deterministic unsupported-layout diagnostics.

## 9) B-tree v1 + Chunk Lookup Design (Concrete)

This section describes the currently implemented chunk-index primitives and how
they are used for bounded chunked reads and upcoming broader chunked raster materialization.

### 9.1 Core structures (implemented)

In `wbhdf::btree`:

- `BTreeNodeHeader`
	- parsed by `parse_node_header(...)` from `TREE` node bytes.
- `InternalRecord` and `LeafRecord`
	- parsed by `parse_internal_records(...)` and `parse_leaf_records(...)`.
- `ChunkedStorageLeafRecord`
	- parsed by `parse_first_chunked_storage_leaf_record(...)` and
		`read_first_chunked_storage_leaf_record_in_file(...)` for bounded chunked-node reads.
- `ChunkIndex`
	- deterministic key -> chunk-address map used by `lookup_chunk_address(...)`.

In `wbhdf::dataset`:

- `DatasetChunkLocator`
	- wrapper that binds a dataset path and `ChunkIndex` and exposes
		`locate_chunk_address(coords)`.

### 9.2 Current bounded chunked-read flow

For validated chunked paths (for example ATL08 helper usage and `wbraster`
bounded recursive traversal):

1. Discover dataset object header and chunked layout metadata.
2. Read first chunk-index leaf record with
	 `read_first_chunked_storage_leaf_record_in_file(...)`.
3. Read compressed payload via `read_chunk_payload_in_file(...)`.
4. Decompress with active filter path (currently deflate/zlib where validated).
5. Decode typed values and apply fill mapping where required.

This flow is intentionally bounded and now includes staged recursive internal-node traversal,
but is not yet general-purpose chunk-tree coverage.

### 9.3 Routing/traversal rules

- Internal-node key routing uses `route_child_for_key(records, key)`:
	- choose first child where `key <= record.key`,
	- otherwise fall back to last child.
- Lookup against `ChunkIndex` is deterministic and path-bound:
	- dataset path mismatch -> `DatasetPathNotFound`,
	- missing key -> `ChunkAddressNotFound`.

### 9.4 Safety invariants

The B-tree/chunk path must preserve these invariants:

- all byte-range reads are bounds-checked against file length,
- key/record length arithmetic uses checked math,
- unsupported node/layout shapes fail fast with explicit diagnostics,
- no unbounded full-file decode assumptions in chunk paths.

### 9.5 Integration plan for `wbraster` chunked materialization

Near-term chunked raster-like integration should follow this staged sequence:

1. Resolve dataset URI + object-header metadata in `wbraster`.
2. Detect chunked layout class and extract chunk index address + dimensions.
3. Use `wbhdf::btree` chunk locator primitives to retrieve one or more chunk addresses.
4. Decode chunk payloads through existing filter/datatype helpers.
5. Assemble output raster by chunk coordinate placement.
6. Preserve deterministic unsupported-layout errors for unhandled variants.

Current state note:
- Steps 1-4 are in place for bounded recursive scalar paths.
- Step 5 is partially realized for one-record, multi-record single-leaf, right-sibling
	leaf-chain, single-level internal-root, and staged multilevel internal traversal;
	real-fixture validation of broader chunk-tree shapes remains the main pending item.

### 9.6 Explicit non-goals for this stage

- no broad B-tree v2 implementation,
- no general-purpose chunk cache design yet,
- no claim of complete HDF5 chunk-layout coverage.

## 10) Documentation Completion Criteria (Design Deliverable)

The Phase 5 design-documentation item should be considered complete when this
document also includes:

- a full multi-chunk assembly algorithm section (not first-chunk only),
- explicit key-format notes for validated product families,
- documented assumptions for chunk-ordering and chunk-coordinate mapping,
- cross-links to regression tests that prove each stated invariant.
