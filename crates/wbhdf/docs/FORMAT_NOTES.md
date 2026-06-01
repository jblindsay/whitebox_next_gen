# FORMAT_NOTES

Working notes for discovered format variations and assumptions.

- Initial scope: HDF5 superblock versions 0/1 and GZIP-compressed targeted layouts.
- HDF4/HDF-EOS2 notes will be tracked under dedicated sections as they are validated.

## Day 4 Notes (Chunk Lookup Validation Path)

- The current synthetic chunk index model uses the first coordinate component as the key for
	deterministic lookup tests.
- Internal-node routing uses the first key upper-bound match, falling back to the last record
	when the lookup key exceeds all explicit bounds.
- Node sibling pointers are parsed in headers but are not yet traversed in this stage.
- Lookup errors are intentionally explicit:
	- unknown dataset path -> `DatasetPathNotFound`
	- missing key in known index -> `ChunkAddressNotFound`

These are temporary scaffolding assumptions for Week 1 validation and will be tightened against
real GEDI/ICESat-2 chunk-layout fixtures in subsequent phases.

## Day 4.5 Notes (Real Dataset Marker Discovery)

- `resolve_dataset_in_file()` is still a pre-traversal validation helper, not a true object-header
	resolver.
- The current heuristic accepts either:
	- a contiguous canonical dataset path marker embedded in the file bytes, or
	- all canonical path components discoverable as separate markers in the file bytes.
- This fallback exists because the ATL08 fixture exposes the real dataset path through the HDF
	hierarchy (`/gt1l/land_segments/canopy/h_canopy` via `h5ls`) but does not store that full path as
	one contiguous byte string in the container.
- GEDI `BEAM0000` dataset markers were observed as contiguous path strings in the current fixture,
	while ATL08 required split-component discovery.

## Day 4.6 Notes (Object Header Signature Probe)

- `object_header::ObjectHeader::parse()` now performs a concrete scan for HDF5 v2 object-header
	signatures (`OHDR`) and returns discovered byte offsets plus any successfully decoded v2 prefixes.
- `probe_file_object_headers(path)` is now available for fixture-backed smoke traversal checks.
- Real ATL08 fixture coverage now includes at least one decoded v2 object-header prefix.
- Current GEDI coverage confirms `OHDR` marker discovery but does not yet guarantee a parsable v2
	prefix at discovered offsets.
- This remains a prefix-level probe only; object-header message decoding and link traversal are
	still pending Week 2+ work.

## Day 4.7 Notes (ATL08 First-Chunk Message Headers)

- The first validated ATL08 v2 object header now decodes bounded first-chunk message headers.
- Current confirmed ATL08 first-chunk message IDs are:
	- `0x01` dataspace
	- `0x03` datatype
	- `0x05` fill_new
	- `0x10` header continuation
- `chunk0_size` is currently interpreted as the span after the v2 prefix, which matches the ATL08
	fixture and `h5debug` output used for validation.
- This is still first-chunk-only decoding; continuation-chunk traversal and full message-body
	decoding remain pending.

## Day 4.8 Notes (ATL08 Continuation Body Decode)

- Header-continuation messages (`0x10`) are now decoded into `(address, size)` targets when the
	message body is at least 16 bytes.
- The first ATL08 v2 object header now validates a continuation target of:
	- address `144130`
	- size `52`
- This matches `h5debug` output for the first continuation from chunk 0 to chunk 1.
- Actual traversal into continuation chunks is not implemented yet; this step only decodes the
	continuation target metadata.

## Day 4.9 Notes (ATL08 Continuation Chunk 1 Headers)

- The first ATL08 continuation chunk at address `144130` now parses as an `OCHK` chunk.
- Current confirmed chunk-1 message IDs are:
	- `0x10` header continuation
	- `0x15` ainfo
- This matches the first two chunk-1 messages reported by `h5debug` for the same object header.
- Parsing is still bounded to the chunk-local message headers; no continuation-chain walk or
	message-body decode for `ainfo` has been implemented yet.

## Day 4.10 Notes (ATL08 Dataspace, Datatype, and Layout Bodies)

- The first ATL08 object header now yields bounded body decodes for:
	- dataspace: version `2`, rank `1`, dims `{1}`, max dims `{1}`
	- datatype: version `1`, class `3` (string), size `38726`
- The second continuation chunk now yields a bounded contiguous layout decode:
	- version `3`
	- layout class `1` (contiguous)
	- data address `8500138`
	- data size `38726`
- Real-fixture tests are pinned to the documented ATL08 and GEDI filenames so field assertions are
	deterministic across runs.

## Day 4.11 Notes (GEDI First Reference Validation)

- First GEDI variable validation now uses `/BEAM0000/elev_lowestmode` from fixture
	`GEDI02_A_2025190205730_O37237_01_T04940_02_004_02_V002.h5`.
- `h5dump -pH` reports this dataset as:
	- datatype `H5T_IEEE_F32LE`
	- dataspace `{89634}`
	- storage `CONTIGUOUS`, `OFFSET 1012683`, `SIZE 358536`
	- filters `NONE`
- The first 12 values decoded from the contiguous payload now match `h5dump`
	reference values within absolute tolerance `1e-5`.
- This provides the first fixture-backed GEDI reference-comparison checkpoint and confirms the
	contiguous little-endian `f32` decode path for a Tier 1 GEDI variable.

## Day 4.12 Notes (VIIRS First Reference Validation Target)

- First VIIRS reference product selected from available local fixtures:
	`VNP13A4N.A2026150.h12v04.002.2026151015223.h5`.
- Initial validation path uses dataset
	`/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim` to establish a deterministic HDF5 baseline.
- `h5dump -pH` confirms this dataset is:
	- datatype `H5T_IEEE_F64LE`
	- dataspace `{2400}`
	- storage `CONTIGUOUS`, `OFFSET 78857`, `SIZE 19200`
	- filters `NONE`
- Integration test coverage now checks the first 8 decoded values against `h5dump` reference output
	with absolute tolerance `1e-8`.
- Additional VIIRS fields in this product (for example, `500 m 8 days EVI`) are chunked +
	deflate and remain follow-up targets for generalized chunked numeric decode paths.

## Day 4.13 Notes (MODIS Fixture Intake for HDF4 Feasibility)

- Local MODIS fixture intake now includes all three requested family variants:
	- `MOD09A1`/`MYD09A1`
	- `MOD13A1`/`MYD13A1`
	- `MOD11A2`/`MYD11A2`
- All currently ingested MODIS files are verified as HDF4 containers via magic signature bytes
	`0E 03 13 01`.
- This establishes concrete, repeatable local fixtures for the HDF4/HDF-EOS2 feasibility slice
	without prematurely claiming MODIS SDS decode support in the current HDF5-first parser path.

## Day 4.14 Notes (Minimal HDF4 EOS Metadata Enumeration)

- Added a minimal in-crate HDF4 probe (`wbhdf::hdf4::probe_hdf4_eos_metadata_in_file`) that:
	- validates HDF4 magic signature bytes,
	- counts `StructMetadata.0` markers,
	- extracts unique `GridName` and `DataFieldName` values from embedded EOS metadata text,
	- extracts field-level `DataType` and `DimList` descriptors for enumerated data fields,
	- extracts grid dimension sizes (`XDim=...`, `YDim=...`) and supports field-shape resolution,
	- extracts grid georeferencing tokens (`UpperLeftPointMtrs`, `LowerRightMtrs`, `Projection`,
	  `ProjParams`, `SphereCode`) into structured metadata.
- Fixture-backed checks now confirm expected metadata enumerations for:
	- `MOD09A1` (`MOD_Grid_500m_Surface_Reflectance`, `sur_refl_b01`, `sur_refl_state_500m`)
	- `MOD11A2` (`MODIS_Grid_8Day_1km_LST`, `LST_Day_1km`, `QC_Day`)
	- `MOD13A1` (`MODIS_Grid_16DAY_500m_VI`, `500m 16 days VI Quality`)
- This is intentionally a feasibility-stage metadata parser and does not yet decode HDF4 SDS data
	payloads; SDS/grid data decode remains subsequent work.

- `resolve_hdf4_grid_field(summary, grid_name, field_name)` now resolves:
	- field descriptor (name, data type, dim list)
	- inferred shape by mapping dim-list entries to parsed grid dimension sizes,
	- associated grid georeferencing metadata for downstream raster-geometry wiring.

- `resolve_hdf4_dataset_path(summary, dataset_path)` now supports canonical HDF4 path resolution
	for `"/GridName/DataFieldName"` paths, returning the same resolved field descriptor + shape.
	Current constraint: deeper multi-segment paths are intentionally rejected in this stage.

- `enumerate_hdf4_dataset_paths(summary)` now produces sorted canonical path inventories for
	all parsed HDF4 grid data fields, enabling dispatch/planning consumers to discover resolvable
	paths without scanning raw `StructMetadata` text.

## Day 4.15 Notes (HDF Module Boundaries Documentation)

- Added `docs/internal/HDF_MODULE_BOUNDARIES.md` to keep HDF5 runtime decode flow and HDF4
	feasibility flow explicitly separated.
- The document records:
	- active HDF5 decode modules,
	- staged HDF4 metadata-only module scope,
	- shared error/fixture contracts,
	- integration intent for `wbraster`/`wblidar` consumers.

## Day 4.16 Notes (HDF4 SDS Decode Attempt Scaffolding)

- Added a first payload-bridge API layer in `wbhdf::hdf4`:
	- `prepare_hdf4_sds_decode_attempt(summary, dataset_path)` resolves canonical
	  `"/GridName/DataFieldName"` paths into a structured decode-attempt descriptor.
	- `decode_hdf4_sds_i16_in_file(path, dataset_path)` performs metadata preflight for an
	  intended `DFNT_INT16` SDS read and emits deterministic `UnsupportedLayout` diagnostics
	  while byte-level HDF4 SDS decode remains unimplemented.
- The unsupported diagnostic now includes resolved context (path, grid, field, data type,
	shape, projection) so downstream dispatch plumbing can distinguish
	"path not found" from "recognized but not yet decodable".
- Fixture-backed integration coverage now pins this behavior on MOD09
	`/MOD_Grid_500m_Surface_Reflectance/sur_refl_b01`.

## Day 4.17 Notes (HDF4 Grid Geometry Derivation)

- Added geometry derivation helpers for resolved HDF4 fields:
	- `derive_hdf4_grid_geometry(resolved)`
	- `derive_hdf4_grid_geometry_for_dataset(summary, dataset_path)`
- For 2D MODIS-style grids with parsed UL/LR corner coordinates, this now computes:
	- rows/cols,
	- `pixel_size_x` / `pixel_size_y`,
	- GDAL-style geotransform `[ulx, pixel_size_x, 0, uly, 0, pixel_size_y]`.
- `decode_hdf4_sds_i16_in_file(...)` diagnostics now include derived geotransform context when
	available, improving downstream adapter observability while SDS payload decode is pending.

## Day 4.18 Notes (Structured SDS Decode Readiness)

- Added `assess_hdf4_sds_i16_decode_readiness(summary, dataset_path)` to provide a structured,
	programmatic readiness report for canonical MODIS-style SDS decode attempts.
- Readiness currently returns:
	- resolved field descriptor,
	- optional derived geometry,
	- explicit blockers list.
- For fixture-backed MOD09 `sur_refl_b01`, readiness confirms path + metadata + geometry are
	resolved, and surfaces only the remaining backend blocker
	("HDF4 SDS payload decode backend not yet implemented").

## Day 4.19 Notes (HDF4 DD Table Parsing + Payload Candidate Scan)

- Added low-level HDF4 Data Descriptor (DD) table traversal:
	- `parse_hdf4_data_descriptors(bytes)`
	- `parse_hdf4_data_descriptors_in_file(path)`
- Parser now walks chained DD blocks using observed MODIS layout (`descriptor_count` +
	`next_block_offset` header) and emits structured descriptor entries `(tag, ref, offset, length)`.
- Added SDS payload candidate discovery helpers:
	- `find_hdf4_sds_i16_payload_candidates(...)`
	- `find_hdf4_sds_i16_payload_candidates_in_file(...)`
- Readiness now has an in-file variant:
	- `assess_hdf4_sds_i16_decode_readiness_in_file(...)`
	- augments blockers with descriptor-scan outcomes:
	  - no exact-size in-bounds candidates, or
	  - descriptor-to-field mapping still pending when candidates exist.
- This pushes the payload bridge one layer deeper: we now parse real HDF4 descriptor metadata and
	report candidate-level decode blockers based on file contents, without over-claiming SDS decode.

## Day 4.20 Notes (Ranked Descriptor Candidate Inspection)

- Added ranked candidate inspection APIs for MODIS SDS decode targeting:
	- `rank_hdf4_sds_i16_payload_candidates(...)`
	- `rank_hdf4_sds_i16_payload_candidates_in_file(...)`
- Ranked output now includes:
	- descriptor metadata,
	- length delta from expected SDS payload bytes,
	- lightweight signature hints (`gzip`, `zlib`, `ascii`, `binary`),
	- payload preview hex.
- When exact-length candidates are absent, in-file readiness now appends nearest-descriptor
	context into blockers (`tag/ref/len/delta/hint/preview`) so mapping work can proceed from
	concrete evidence rather than generic failure messages.

## Day 4.21 Notes (Heuristic Descriptor-to-Field Mapping)

- Added heuristic mapping APIs:
	- `map_hdf4_sds_i16_descriptor_heuristic(...)`
	- `map_hdf4_sds_i16_descriptor_heuristic_in_file(...)`
- Mapping behavior:
	- prefer exact-length descriptor candidates,
	- otherwise prefer nearest compressed-like (`gzip`/`zlib`) candidate,
	- otherwise fall back to nearest length-delta candidate,
	- always return explicit rationale and considered-candidate count.
- In-file readiness now includes heuristic mapping summaries in blockers when exact-size matching
	candidates are absent, providing concrete provisional mapping evidence for next decode steps.

## Day 4.22 Notes (Bounded Payload Probe for Heuristic Candidate)

- Added bounded payload probe APIs:
	- `probe_hdf4_sds_i16_payload_window(...)`
	- `probe_hdf4_sds_i16_payload_window_in_file(...)`
- Probe behavior now classifies the heuristic-selected descriptor as one of:
	- `compressed_payload` (gzip/zlib signature),
	- `textual_payload`,
	- `decoded_preview` (binary i16 preview decoded in both little-endian and big-endian views),
	- `no_candidate`, `candidate_out_of_bounds`, or `insufficient_bytes`.
- In-file readiness blockers and decode diagnostics now include probe status + rationale, and when
	available, preview sample vectors to improve next-step descriptor-to-field mapping and decode
	backend planning.

- Probe now attempts bounded decompression for `gzip`/`zlib`-hinted candidates (up to
	`MAX_COMPRESSED_PROBE_BYTES`) and, on success, emits little-endian/big-endian i16 previews from
	the decompressed payload bytes.
- If compressed probe decode fails, diagnostics still return deterministic `compressed_payload`
	status with explicit decode-failure rationale.

- Added `attempt_decode_hdf4_sds_i16_window_in_file(...)` to convert successful probe-decoded
	previews into a direct bounded decode return path (little-endian view), while preserving
	deterministic unsupported diagnostics (`status` + `rationale`) when decode is unavailable.

## Day 4.26 Notes (Preview Endianness Consistency)

- `attempt_decode_hdf4_sds_i16_window_in_file(...)` now uses the same i16 preview plausibility
	selection as `decode_hdf4_sds_i16_in_file(...)`.
- This removes a behavior split where one API could return a hardcoded little-endian preview while
	the other selected between little/big previews based on score.
- Added unit coverage (`window_decode_attempt_uses_preferred_endianness`) to pin this behavior.

## Day 4.27 Notes (Offset-Based SDS Window Decode)

- Added `decode_hdf4_sds_i16_window_at_in_file(path, dataset_path, start_value, max_values)`
	for bounded window extraction from the heuristic-selected descriptor candidate.
- The new path supports start-offset window reads and handles both raw binary and bounded
	gzip/zlib payload decode before i16 interpretation.
- `attempt_decode_hdf4_sds_i16_window_in_file(...)` now delegates to this new API so window
	decode behavior is centralized.
- Added unit coverage (`window_decode_at_supports_start_offset`) to verify offset-based window
	reads on synthetic HDF4 descriptor payloads.

## Day 4.28 Notes (Offset-Window Bounds Diagnostics)

- Added fixture-backed integration coverage for out-of-bounds start handling on
	`decode_hdf4_sds_i16_window_at_in_file(...)`.
- MOD09 decode attempts with an intentionally invalid large start index now assert deterministic
	invalid-input diagnostics containing an out-of-bounds message.

## Day 4.29 Notes (Zero-Length Window Guardrails)

- Added fixture-backed integration coverage for `decode_hdf4_sds_i16_window_at_in_file(...)`
	with `max_values=0`.
- MOD09 decode attempts now explicitly assert deterministic invalid-input diagnostics for
	zero-length window requests.

## Day 4.30 Notes (Compressed Window Decode Limits)

- Added synthetic unit coverage for `decode_hdf4_sds_i16_window_at_in_file(...)` oversized
	compressed candidates (`> MAX_COMPRESSED_WINDOW_DECODE_BYTES`).
- The decode path now has pinned behavior for deterministic
	`status=compressed_payload` diagnostics when a selected compressed descriptor exceeds the bounded
	window-decode size limit.

## Day 4.31 Notes (wbraster HDF URI Dispatch Boundaries)

- `wbraster::Raster::read(...)` now checks for HDF dataset URIs before extension-based raster format
	detection.
- Current URI contract: `container.ext:///absolute/dataset/path`.
- Current `wbraster` dispatch scope:
	- HDF4 (`.hdf`/`.h4`): bounded raster materialization path for 2D `DFNT_INT16` SDS datasets.
	- HDF5/NetCDF (`.h5`/`.hdf5`/`.he5`/`.nc`): recognized and fails fast with deterministic
	  not-yet-implemented raster materialization diagnostics.
- Current HDF4 rasterization caveat:
	- Georeferencing is derived when HDF4 EOS metadata provides UL/LR grid corners; otherwise a
	  fallback identity-like grid (`x_min=0`, `y_min=0`, `cell_size=1`) is used.

## Day 4.32 Notes (HFA Test-Build Regression Root Cause)

- A temporary test-build failure in `wbraster::formats::hfa` was caused by stale test-helper
	symbol references (`ROOT_PTR_OFFSET`, `ENTRY_HDR_LEN_OFFSET`) after the production header-offset
	variant refactor to `*_V1`/`*_V2` plus shared resolver logic.
- Resolution was to route the helper through `read_root_and_entry_header_len(...)`, keeping tests
	aligned with production offset detection and avoiding duplicate offset assumptions.

## Day 4.33 Notes (First HDF5 Raster URI Materialization Path)

- `wbraster` now includes a first validated HDF5 raster materialization path for
	`*.h5:///BEAM0000/elev_lowestmode`.
- Current materialization details:
	- source decode: contiguous little-endian `f32` window via `wbhdf::dataset::read_contiguous_f32_window_in_file`
	- contiguous offset: `1_012_683`
	- element count: `89_634`
	- raster shape: `rows=1`, `cols=89_634`, `bands=1`, `DataType::F32`
- Current boundary: this is intentionally path-specific staged support; other HDF5/NetCDF dataset
	URIs are still recognized but fail with explicit limited-scope diagnostics.

## Day 4.34 Notes (Second HDF5 Raster URI Materialization Path)

- Added a second validated HDF5 raster URI materialization path for
	`*.h5:///HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim`.
- Current materialization details:
	- source decode: contiguous little-endian `f64` window via `wbhdf::dataset::read_contiguous_f64_window_in_file`
	- contiguous offset: `78_857`
	- element count: `2_400`
	- raster shape: `rows=1`, `cols=2_400`, `bands=1`, `DataType::F64`
- Boundary remains staged and explicit: other HDF5/NetCDF dataset URIs continue to fail with
	a clear limited-scope diagnostic.

## Day 4.31 Notes (Deterministic Exact-Candidate Window Decode)

- `decode_hdf4_sds_i16_window_at_in_file(...)` now first tries exact-length in-bounds descriptor
	candidates in deterministic order (signature preference + stable descriptor ordering) before
	falling back to heuristic mapping.
- This reduces heuristic dependence for MODIS/HDF4 SDS window extraction when multiple exact-size
	descriptors are present.
- Added unit coverage to confirm binary exact candidates are preferred over textual exact
	candidates for decode attempts.

## Day 4.32 Notes (Exact-Candidate Attempt Diagnostics)

- Added deterministic diagnostics for exact-candidate decode failures that now report how many
	ranked exact-length candidates were attempted before failure.
- Added unit coverage to verify failure messages include `attempted=N/M` context for rapid
	triage of descriptor-selection issues.

## Day 4.23 Notes (Heuristic Confidence Summary)

- Heuristic descriptor mapping now carries a confidence label:
	- `high` for exact-length candidates,
	- `medium` for near candidates with favorable length delta,
	- `low` for tentative nearest candidates,
	- `none` when no ranked candidates exist.
- Readiness blockers now surface the confidence label alongside the selected candidate summary so
	MODIS mapping decisions are explicitly qualified rather than implied.

## Day 4.24 Notes (Opportunistic Decode Preview)

- `decode_hdf4_sds_i16_in_file(...)` now returns the mapped probe's little-endian preview when the
	selected MODIS/HDF4 payload can be decoded, giving a real bounded i16 window instead of only an
	unsupported-layout diagnostic.
- The function still falls back to structured diagnostics when the probe cannot produce a decoded
	preview, so the read path remains deterministic for unresolved fixtures.

## Day 4.25 Notes (Preview Plausibility Selection)

- When both little-endian and big-endian i16 previews are available, the public decode path now
	uses a simple plausibility score to return the more realistic window instead of always preferring
	little-endian output.
- The score favors moderate-magnitude values and penalizes obviously byte-swapped patterns, which
	helps the MOD09 preview path surface a better real window without hard-coding a fixture-specific
	endian assumption.
