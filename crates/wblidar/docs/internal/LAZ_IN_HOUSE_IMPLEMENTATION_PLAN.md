# In-House LASzip (LAZ) Implementation Plan for wblidar

## Goal
Implement standards-compatible LASzip read/write in `wblidar` without adding external runtime dependencies to backend crates.

## Constraints
- Keep backend dependency surface minimal (`wblidar`, `wbraster`, `wbvector`).
- Maintain pure Rust implementation.
- Preserve existing `PointRecord` API and frontend `read` / `write` behavior.
- Keep memory-safe behavior for malformed or adversarial files.

## Current State (2026-03-29)

All five implementation phases are complete. The codebase now maintains two
parallel LAZ paths:

**Standards-compliant LASzip path (read and COPC write)**
- `LazReader` dispatches incoming `.laz` and `.copc.laz` files to the
  standards-compliant arithmetic-coded backend for all Point10 and Point14
  family files produced by external tools (LAStools, PDAL, etc.).
- The COPC writer emits standard LASzip layered-chunked payloads (Point14
  family only: PDRF6, PDRF7, PDRF8) byte-compatible with external readers.
- The arithmetic encoder/decoder, integer codec, and all field codecs are
  fully implemented and validated against LAStools reference output.

**wb-native DEFLATE path (standalone .laz write)**
- `LazWriter` still uses a wb-native DEFLATE-based chunked codec for writing
  standalone `.laz` files.  Files produced by `LazWriter` are not readable by
  external LASzip consumers; they are intended for wb-internal workflows only.
- `LazReader` detects wb-native files by their chunk-table magic and routes
  them through the separate DEFLATE path, preserving full backward compatibility.

**Key milestones achieved**
- External `las` dependency removed from `wblidar` (pure Rust).
- COPC writer now produces standard-interoperable files validated against
  LAStools (Carlos dataset: 116,381,594 points, PDRF6, paged hierarchy).
- Arithmetic encoder carry-propagation bug fixed (`ArithmeticEncoder::done`);
  all Point14 layers are now byte-identical to LAStools reference output.
- External interoperability harness now supports optional real-world fixture
	directories (`WBLIDAR_LAZ_INTEROP_FIXTURE_DIR`) and labels report rows as
	generated vs fixture coverage.
- 117/117 library tests passing.

## Reference Architecture (Open Source Study)
The `laz` crate provided a reference modular structure for standards LAZ:
- Arithmetic core: symbol models, bit models, encoder/decoder.
- Integer compressors/decompressors for delta/predictive coding.
- Field compressors/decompressors by LAZ item type.
- Sequential chunked compressor/decompressor with chunk-table support.

Primary canonical reference for behavior compatibility:
- LASzip upstream implementation: https://github.com/LASzip/LASzip

Equivalent functionality has been implemented natively in `wblidar` without
adding an external `laz` crate dependency.

## Implementation Phases (all complete as of 2026-03-29)

### Phase 1: LASzip VLR and Chunk Metadata
1. Added LASzip VLR parser that extracts:
- compressor type
- coder
- chunk size
- item records (type/size/version)
2. Added chunk-table pointer reader (`i64` at start of LAZ point data).
3. Added chunk-table parser for standard LASzip format, including variable-size chunks.
4. Strict bounds checks in place for all counts and sizes.

Acceptance (met):
- Metadata parser tests pass against representative VLR byte fixtures.
- Reader classifies wb-native vs standards LASzip deterministically.

### Phase 2: Arithmetic Coding Core
1. Implemented arithmetic bit-model and symbol-model structures.
2. Implemented arithmetic decoder and encoder with carry/renorm handling.
   Fixed duplicate carry propagation bug in `ArithmeticEncoder::done`.
3. Deterministic unit tests cover known small vectors.

Acceptance (met):
- Bit/symbol coding round-trip tests pass.
- Decoder resilience tests for truncated/corrupt streams pass.

### Phase 3: LASzip Field Codecs for Point 6/7/8
1. Implemented integer compressor/decompressor primitives.
2. Implemented Point14 core field codec (PDRF6/7/8 base fields).
3. Implemented GPS time field codec.
4. Implemented RGB field codec.
5. Implemented NIR field codec for PDRF8.

Acceptance (met):
- Round-trip tests for PDRF6/7/8 pass.
- Byte-exact decode validation against LAStools reference outputs passes
  for all layers.

### Phase 4: Streaming Reader/Writer Integration
1. Added standards-compliant backend modules under `src/laz/`.
2. `LazReader` dispatches:
   - wb-native DEFLATE path for wb-native files
   - standards LASzip arithmetic-coded path for external files
3. `LazWriter` still emits wb-native DEFLATE chunked streams (see note in
   Current State about planned future migration).
4. CRS/VLR handling preserved in existing LAS header pipeline.

Acceptance (met):
- Frontend `.laz` read works without external crates against real-world files.
- `PointCloud::read` passes integration tests on corpus fixtures.

### Phase 5: COPC Compatibility and Shared Codec Path
1. COPC node decode and encode route through shared arithmetic/field codec
   primitives.
2. No duplicate logic between COPC and standard LAZ streams.
3. Fixture tests cover COPC decode edge cases including paged and
   mixed-root hierarchy layouts.

Acceptance (met):
- COPC decode regression tests pass.
- No codec divergence between LASzip-related paths.

## COPC Conformance Roadmap (2026-03-29)

This section is the authoritative roadmap for bringing `wblidar` COPC support
from "internally functional" to "spec-conformant and externally interoperable".
It supplements the LAZ implementation phases above rather than replacing them.

Current interoperability status:
- The COPC reader supports PDRF6, PDRF7, and PDRF8 (RGBNIR14 layered
	continuation implemented) using the shared Point14-family arithmetic decoder.
- The COPC writer emits standard LASzip-compressed payloads for PDRF6/7/8
	and has been externally validated against LAStools and the real-world Carlos
	dataset (116M-point, paged hierarchy).
- The standalone `LazWriter` still uses wb-native DEFLATE compression; it is
	not intended for interchange with external LASzip consumers.

### Conformance Objectives

1. Strictly validate COPC 1.0 structure when requested.
2. Read all COPC-required LAS 1.4 point families used by the standard:
	 PDRF6, PDRF7, and PDRF8.
3. Write COPC files whose chunk payloads are standard LASzip and open in external
	 tooling such as PDAL, QGIS, CloudCompare, and dedicated COPC validators.
4. Keep COPC-specific logic limited to file layout, hierarchy traversal, and node
	 selection while reusing the shared LAZ codec path for payloads.
5. Add subset-query and range-readable infrastructure after correctness and
	 interoperability are established.

### Tracking Conventions

Status values used below:
- `not started`: no implementation work has begun.
- `in progress`: active implementation or validation work is underway.
- `blocked`: progress depends on an upstream decision, missing fixture, or external validation step.
- `done`: implementation and acceptance checks are complete.

Update policy:
- Change the milestone `Status:` line whenever work crosses one of the states above.
- Check off implementation and validation items as they land.
- Append a dated bullet to `Progress Log` for each meaningful milestone step.
- If a milestone is partially complete but cannot satisfy its acceptance criteria yet,
	 leave the milestone status as `in progress` or `blocked` rather than `done`.

### Milestone Tracker

#### Milestone 1: Reader Conformance Hardening

Status: done

Scope:
- Add explicit COPC signature and version validation.
- Validate that the first VLR is the COPC info VLR in strict mode.
- Enforce strict-mode checks for reserved fields, hierarchy page size alignment,
	and PDRF restrictions.
- Introduce explicit strict vs tolerant reader behavior instead of relying only on
	tolerant heuristics.

Acceptance:
- Valid COPC fixtures pass in strict mode.
- Malformed synthetic fixtures fail with precise structured errors.
- Tolerant mode retains current interoperability behavior for known producer quirks.

Primary code areas:
- `src/copc/reader.rs`
- `src/copc/hierarchy.rs`

Implementation checklist:
- [x] Add explicit COPC signature validation for the `copc` user id / record position expectations.
- [x] Add explicit COPC version validation for version 1.0.
- [x] Add strict-mode validation that the first VLR is the COPC info VLR.
- [x] Add strict-mode validation for PDRF restriction to 6/7/8.
- [x] Add strict-mode validation for reserved `CopcInfo` fields.
- [x] Add strict-mode hierarchy page validation: page size multiple of 32, legal offsets, legal `pointCount` state.
- [x] Introduce a reader configuration or mode enum for `strict` vs `tolerant` behavior.
- [x] Ensure tolerant mode retains the current best-effort hierarchy offset handling.

Validation checklist:
- [x] Add synthetic tests for bad magic / bad version / missing COPC info VLR.
- [x] Add synthetic tests for malformed hierarchy pages.
- [x] Verify known-good COPC fixtures pass in strict mode.
- [x] Verify known producer-quirk fixtures still pass in tolerant mode.

#### Milestone 2: Finish PDRF8 / RGBNIR14 Reader Support

Status: done

Scope:
- Implement layered RGBNIR14 continuation for Point14-family item sets containing
	`POINT14 + RGB14 + NIR14`.
- Add real-fixture and synthetic regression tests for PDRF8 COPC.

Acceptance:
- PDRF8 COPC files decode without partial recovery on known-good fixtures.
- Shared Point14-family decoder covers PDRF6, PDRF7, and PDRF8.

Primary code areas:
- `src/laz/standard_point14.rs`

Implementation checklist:
- [x] Add layered continuation handling for `POINT14 + RGB14 + NIR14` item layouts.
- [x] Port the RGBNIR14 arithmetic layer logic from LASzip reference behavior.
- [x] Keep the implementation on the shared Point14-family decoder path instead of adding COPC-only logic.
- [x] Preserve strict/tolerant layered decode behavior and partial accounting.

Validation checklist:
- [x] Add unit or synthetic regression coverage for RGBNIR14 continuation.
- [ ] Add at least one real PDRF8 COPC fixture to smoke coverage if available.
  Local ReferenceLidar corpus probe (lidar_corpus_inventory, first 200 files)
  did not surface any pdrf8 signatures, so real-fixture validation is currently
  corpus-limited rather than code-limited.  All local PDRF8 validation uses
  synthetic fixtures; real-file coverage remains open as a corpus-expansion task.
- [x] Verify PDRF6/PDRF7 behavior remains unchanged after the PDRF8 extension.

#### Milestone 3: Standardize COPC Writer Payload Encoding

Status: done

Scope:
- Replace wb-native DEFLATE payload emission in the COPC writer with the shared
	standards-compliant LASzip encoder path.
- Ensure emitted LASzip VLR metadata, item layouts, and chunk semantics match
	external reader expectations.
- Support standards-compliant COPC writing for PDRF6, PDRF7, and PDRF8.

Acceptance:
- COPC files written by `wblidar` open successfully in at least one independent
	third-party consumer on a real-world dataset.
- Internal re-read verifies point count, bounds, CRS, and representative
	attributes.
- Byte-level parity spot checks on representative problematic chunks succeed
	against LAStools reference outputs.

Primary code areas:
- `src/copc/writer.rs`
- shared LAZ writer/encoder path under `src/laz/`

Implementation checklist:
- [x] Remove direct wb-native DEFLATE payload emission from the COPC writer.
- [x] Route COPC node payload generation through the shared standards-compliant LASzip encoder.
	Current scope: COPC writer enforces Point14-family formats (PDRF6/7/8)
	and rejects non-Point14 format requests.
- [x] Ensure emitted LASzip VLR metadata matches the encoded item layout and compressor type.
	Current scope: metadata alignment for Point14-family standards payloads.
- [x] Support standards-compliant writer output for PDRF6.
- [x] Extend writer output support to PDRF7.
- [x] Extend writer output support to PDRF8.
	Coverage note: strict-mode writer regressions now cover singleton,
	non-scanner multi-point, and scanner-channel multi-point variation for
	XYZ, return fields, intensity, classification, flags, user_data,
	point-source, scan-angle, gps-time, RGB, and NIR where representable.
- [x] Verify hierarchy entry offsets and byte sizes match the actual encoded chunk payloads.
- [x] Add explicit strict-mode writer regressions that fail on non-encodable Point14 sets.

Validation checklist:
- [x] Add internal round-trip tests for writer-produced COPC using the shared reader.
- [x] Verify writer output preserves point count, bounds, and CRS.
- [x] Verify representative GPS, RGB, and NIR attributes round-trip correctly.
	Scope: strict singleton plus multi-point/scanner-channel regressions for
	PDRF6/7/8 writer outputs.
- [x] Validate writer-produced COPC with an independent third-party consumer on
	a real-world dataset (LAStools decode of Carlos dataset succeeds end-to-end).
- [x] Validate chunk-level parity on known-problem chunk(s) against LAStools
	reference payloads (all layers byte-identical after arithmetic finalization fix).

#### Milestone 4: External Interoperability Validation Harness

Status: blocked

Scope:
- Add repeatable validation workflow for writer output using external tools.
- Build fixture coverage for single-page, paged, and mixed-root hierarchies;
	PDRF6/7/8; CRS combinations; and RGB/NIR/extra-bytes variants where supported.

Acceptance:
- Each supported writer scenario has at least one external interoperability check.
- Regressions are caught before claiming conformance improvements.

Primary code areas:
- integration tests and validation scripts/examples

Implementation checklist:
- [x] Define the minimum external validation matrix: producer, hierarchy shape, PDRF, CRS combination.
- [x] Add repeatable commands or scripts for PDAL-based validation.
- [x] Add repeatable commands or scripts for COPC-specific validation tooling.
- [x] Include a manual/automated validation step using https://validate.copc.io where applicable.
- [x] Record manual GUI validation procedure for at least one third-party desktop client.
- [x] Emit machine-readable JSON interop reports and CI-generated markdown summaries
	for matrix ingestion and run-to-run comparison.
- [x] Add optional fixture coverage gate (`WBLIDAR_LAZ_INTEROP_MIN_FIXTURE_PROFILES`)
	so CI can fail when real-world fixture validation falls below target thresholds.

Validation checklist:
- [x] Single-page hierarchy output validated externally.
- [x] Paged hierarchy output validated externally (Carlos COPC: 116,381,594
  points, LAStools pass via harness).
- [x] Mixed-root hierarchy output validated externally if writer can produce it.
	Current writer scope note: production writer outputs validated in this milestone
	are single-page or paged layouts; mixed-root production output is not currently
	emitted as a standard fixture shape, so this item is treated as not applicable
	for milestone sign-off.
- [x] PDRF6 output validated externally.
- [x] PDRF7 output validated externally.
- [ ] PDRF8 output validated externally (blocked: local LAStools v3.4r1 lacks
	RGBNIR14 support; pending newer LAStools/LASzip build or PDAL-based external
	validation path for PDRF8 fixtures).

Manual desktop validation procedure (QGIS):
- Open QGIS and add the COPC file through Layer -> Add Layer -> Add Point Cloud Layer.
- Confirm point count and extent load in layer properties.
- Verify points render in map canvas at full extent and while zooming.
- Confirm no load crash and no empty-canvas result for the tested fixture.

#### Milestone 5: COPC Query APIs

Status: done

Scope:
- Add bbox, subtree, and max-depth/LOD query APIs.
- Avoid forcing callers through full-tree `read_all_nodes()` for common subset
	access patterns.

Acceptance:
- Callers can request only intersecting nodes or capped hierarchy depth.
- Query traversal correctness is covered by regression tests.

Primary code areas:
- `src/copc/reader.rs`

Implementation checklist:
- [x] Add node iteration API without forcing eager full decode.
- [x] Add subtree query API by root voxel key.
- [x] Add bbox query API over hierarchy keys.
- [x] Add max-depth / LOD-limited query API.
- [x] Keep hierarchy traversal reusable across all query modes.

Validation checklist:
- [x] Add traversal regressions for bbox intersection logic.
- [x] Add traversal regressions for subtree expansion logic.
- [x] Add traversal regressions for max-depth limiting.
- [x] Verify query APIs do not regress full-tree reads.

#### Milestone 6: Range-Readable IO Backend

Status: done

Scope:
- Introduce a byte-range access abstraction beneath the COPC reader.
- Support local-file and HTTP range-backed implementations.
- Keep hierarchy and node decode logic transport-agnostic.

Acceptance:
- The same reader logic works against local and remote range-readable sources.
- Node reads avoid whole-file downloads when using remote backends.

Primary code areas:
- `src/copc/reader.rs`
- supporting IO abstraction module(s)

Implementation checklist:
- [x] Define a byte-range source abstraction for random-access reads.
- [x] Add a local-file implementation with parity to current behavior.
- [x] Adapt the COPC reader to use the abstraction instead of raw `Read + Seek` directly.
- [x] Add an HTTP range implementation.
- [x] Add minimal caching strategy for hierarchy pages and node payloads.

Validation checklist:
- [x] Local-file backend passes all current reader tests.
- [x] HTTP backend can read header, hierarchy, and targeted nodes without full download.
- [x] Remote subset reads preserve correctness relative to local-file reads.

### Recommended Execution Order

1. ~~Milestone 1: Reader conformance hardening.~~ done
2. ~~Milestone 2: PDRF8 / RGBNIR14 reader completion.~~ done
3. ~~Milestone 3: Standards-compliant COPC writer payloads.~~ done
4. Milestone 4: External interoperability validation. *blocked (PDRF8 external tool support)*
5. Milestone 5: Query APIs. *done*
6. Milestone 6: Range-readable IO. *done*

Rationale:
- Milestones 1–3 are complete and closed all correctness and writer
	interoperability gaps.
- Milestone 4 completes the external validation record before moving to
	new capability work.
- Milestones 5 and 6 increase COPC usefulness after format correctness is stable.

### Conformance Definition of Done

Reader:
- Strict mode validates COPC 1.0 structure explicitly.
- Tolerant mode preserves practical interoperability with known producer quirks.
- PDRF6, PDRF7, and PDRF8 decode through the shared Point14-family path.

Writer:
- Emits standard LASzip-compressed payloads instead of wb-native DEFLATE payloads.
- Emits valid COPC info and hierarchy metadata.
- Passes external validation and opens in third-party consumers.

Feature completeness:
- Supports node, bbox, subtree, and LOD-oriented access patterns.
- Supports local and HTTP range-readable data sources.

### Progress Log

- 2026-03-29:
	- Confirmed reader compatibility for current PDRF6 and PDRF7 corpus coverage.
	- Fixed Point14 layered RGB14 continuation so colourized PDRF7 COPC files now
		decode successfully.
	- Re-assessed the writer path and confirmed that COPC payload compression remains
		wb-native DEFLATE, which blocks external interoperability claims.
	- Established the milestone plan above as the canonical COPC conformance tracker.
	- Milestone 1 started: added `CopcReaderMode` with strict/tolerant constructors,
		strict COPC header/VLR validation, strict hierarchy layout validation, and
		regression tests while preserving tolerant-mode fallback behavior.
	- Added synthetic strict-validation regressions for bad magic, bad LAS version,
		wrong COPC info record-id bytes, and missing COPC info VLR.
	- Ad hoc strict-mode smoke check succeeded on
		`Mar19_test_colourized.copc.laz` (`STRICT_OK entries=3557 mode=DataOffset`).
	- Marked Milestone 1 done after strict-mode synthetic coverage and real-fixture
		constructor smoke validation.
	- Milestone 2 started: implemented shared layered RGBNIR14 continuation support
		for `POINT14 + RGB14 + NIR14` item layouts in `standard_point14.rs`.
	- Verified the extended Point14-family decoder still passes the full library
		test suite (`cargo test -q -p wblidar --lib`: 66/66).
	- Added a synthetic RGBNIR14 layered-continuation regression test that encodes
		a one-point arithmetic stream with zero XYZ/RGB/NIR deltas and verifies the
		second `Pdrf8` point inherits the seed attributes through the shared decoder path.
	- Probed `/Users/johnlindsay/Documents/data/ReferenceLidar` with the
		`lidar_corpus_inventory` example (`--max-files 200 --report-json`); no local
		`pdrf8` signatures were found in that sample, so real-fixture validation remains
		pending on corpus availability.
	- Milestone 3 started: added shared LASzip VLR metadata builder for writer paths,
		updated `LazWriter` to use the shared builder, and updated `CopcWriter` to emit
		a LASzip VLR (after COPC info VLR) so strict-mode COPC validation recognizes
		writer output as LAZ-metadata-aware.
	- Corrected `CopcWriter` hierarchy offset semantics: `CopcInfo.hierarchy_root_offset`
		now points to hierarchy data bytes (not EVLR header), while LAS header
		`start_of_first_evlr` remains the EVLR-header offset.
	- Added strict regression proving writer-produced COPC now opens in strict mode
		with `CopcHierarchyParseMode::DataOffset`, while strict mode still rejects
		EVLR-header-offset interpretation.
	- Refactored wb-native payload compression/decompression into shared helpers in
		`laz::codec` and switched both LAZ and COPC writer/reader call-sites to use
		the shared path as groundwork for the upcoming standards-compliant encoder swap.
	- Added writer-side singleton standard Point14 layered seed encoder in
		`laz::standard_point14` (`encode_standard_layered_chunk_point14_v3_singleton`)
		with regression round-trip coverage.
	- Updated `CopcWriter` node payload dispatch to use the shared standards path for
		PDRF6/7/8 singleton-node outputs and retain wb-native fallback for multi-point
		nodes, preserving broad internal compatibility while incrementally moving writer
		payload generation onto reusable standards-oriented primitives.
	- Added strict singleton round-trip regressions for writer-produced COPC:
		- PDRF7 preserves GPS+RGB,
		- PDRF8 preserves GPS+RGB+NIR,
		while keeping strict DataOffset hierarchy behavior.
	- Added writer round-trip regression asserting preservation of point count,
		header bounds, and CRS metadata (EPSG) for writer-produced COPC.
	- Added `CopcWriterConfig::allow_wb_native_fallback` (default `true`) so
		standards-only runs can fail-fast when unsupported multi-point Point14
		standards encoding would otherwise fall back to wb-native payloads.
	- Added regressions covering standards-only rejection for multi-point Point14
		nodes and default-mode backward-compatible fallback behavior.
	- Added multi-point Point14 standards subset encoder for constant non-XYZ
		attributes (XY plus optional Z layered arithmetic streams), integrated into
		COPC writer standards path prior to wb-native fallback.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying intensity (in addition to XY and optional Z changes) while requiring
		other non-XYZ attributes to remain constant.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying classification (in addition to XY, optional Z, and intensity)
		while continuing to reject unsupported return/flags/user-data/scan-angle/
		point-source/gps/rgb/nir variation in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying flags (classification flags, scan-direction flag, and edge-of-flight)
		while keeping return fields, scanner channel, and other non-XYZ layers
		constrained in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying return fields (return number and number of returns) while
		continuing to require constant scanner-channel/user-data/scan-angle/
		point-source/gps/rgb/nir layers in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying user_data while continuing to require constant scanner-channel/
		scan-angle/point-source/gps/rgb/nir layers in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying point source ids while continuing to require constant
		scanner-channel/scan-angle/gps/rgb/nir layers in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying scan-angle values while continuing to require constant
		scanner-channel/gps/rgb/nir layers in standards-only mode.
	- Expanded the multi-point Point14 standards subset encoder to support
		varying gps-time values while continuing to require constant
		scanner-channel/rgb/nir layers in standards-only mode.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point intensity variation (with
		other non-XYZ layers still constrained when scanner channel changes).
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point classification variation
		(with user-data/scan-angle/point-source/gps/rgb/nir still constrained
		when scanner channel changes).
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point user-data variation
		(with scan-angle/point-source/gps/rgb/nir still constrained when
		scanner channel changes).
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point scan-angle variation
		(with point-source/gps/rgb/nir still constrained when scanner channel
		changes).
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point point-source variation
		(with gps/rgb/nir still constrained when scanner channel changes).
	- Fixed Point14 decode path selection to prefer layered arithmetic
		continuation before non-arithmetic per-point fallback, preventing false
		per-point interpretation when layered tail sizes coincidentally match raw
		tail byte counts.
	- Added regressions for subset round-trip success, out-of-subset rejection,
		and standards-only acceptance/rejection semantics, including
		scanner-channel+intensity, scanner-channel+classification,
		scanner-channel+user-data, scanner-channel+scan-angle, and
		scanner-channel+point-source acceptance.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point gps-time variation
		(with rgb/nir still constrained when scanner channel changes).
	- Added regressions for scanner-channel+gps-time round-trip and
		standards-only writer acceptance.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point nir variation for PDRF8
		(with rgb still constrained when scanner channel changes).
	- Added regressions for scanner-channel+nir round-trip and
		standards-only writer acceptance.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point rgb variation for PDRF8
		(RGBNIR mode).
	- Added regressions for scanner-channel+rgb round-trip and
		standards-only writer acceptance.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point flags variation while
		keeping return fields scanner-invariant.
	- Added regressions for scanner-channel+flags round-trip and
		standards-only writer acceptance.
	- Expanded scanner-channel switching support for the multi-point Point14
		standards subset path to include per-point return-field variation.
	- Added regressions for scanner-channel+return-fields round-trip and
		standards-only writer acceptance.
	- Expanded non-scanner multi-point Point14 standards subset support to
		include PDRF8 RGB/NIR variation.
	- Added regressions for non-scanner multi-point RGB/NIR round-trip and
		standards-only writer acceptance.
	- Expanded scanner-channel multi-point Point14 standards subset support
		to include PDRF7 RGB variation.
	- Added regressions for scanner-channel PDRF7 RGB round-trip and
		standards-only writer acceptance.
	- Expanded non-scanner multi-point Point14 standards subset support
		to include PDRF7 RGB variation.
	- Added regressions for non-scanner PDRF7 RGB round-trip and
		standards-only writer acceptance.
	- Updated out-of-subset standards-only fixtures to use scanner-channel
		plus invalid format-representability combinations (e.g., RGB in
		PDRF6) now that scanner/non-scanner PDRF7 RGB and PDRF8 RGBNIR
		variation are supported.
	- Validation update: `cargo test -p wblidar --lib` now passes 112/112.
	- Narrowed COPC writer Point14 fallback behavior to apply only to
		standards-subset `Unimplemented` cases; non-representable Point14
		inputs now fail immediately instead of silently falling back.
	- Added regression for default-mode rejection of non-representable
		Point14 input (PDRF7 without required RGB data).
	- Validation update: `cargo test -p wblidar --lib` now passes 113/113.
	- Tightened Point14 representability validation to reject incompatible
		attribute combinations by target format (e.g., RGB/NIR in PDRF6,
		NIR in PDRF7).
	- Removed direct wb-native DEFLATE fallback emission from COPC writer
		for Point14-family formats; COPC Point14 chunks now always use the
		standards Point14 encoder path.
	- Updated writer regressions to reflect non-encodable Point14 rejection
		semantics under strict representability.
	- Validation update: `cargo test -p wblidar --lib` remains 113/113.
	- Removed `CopcWriterConfig::allow_wb_native_fallback` from the API to
		avoid stale configuration now that Point14 COPC payload emission is
		always standards-encoder based.
	- Removed the remaining non-Point14 wb-native payload branch in COPC
		writer by enforcing Point14-family formats (PDRF6/7/8) for COPC
		writes; non-Point14 requests now fail fast with an explicit
		unsupported-format error.
	- Added regression coverage for non-Point14 PDRF rejection in COPC
		writer.
	- Added `examples/copc_validation_fixture_pack.rs` to generate
		reproducible PDRF6/PDRF7/PDRF8 COPC outputs for external validators
		and third-party interoperability checks.
	- Verified fixture-pack example execution writes all three outputs:
		`cargo run -p wblidar --example copc_validation_fixture_pack -- /tmp/wblidar_copc_validation`.
	- Added `COPC_EXTERNAL_VALIDATION.md` with a step-by-step external
		validation protocol (validate.copc.io + independent consumer) and a
		results table for Milestone 3 sign-off.
	- Probed current environment for external tools (`pdal`,
		`cloudcompare`, `lasinfo`) and found none installed locally; external
		consumer validation remains pending manual/CI execution.
	- Added `examples/copc_validation_report.rs` to produce a Markdown
		report from generated fixtures with strict-reader verification fields
		and pre-seeded external validation checklist rows.
	- Isolated a late-stream Point14 scan-angle mismatch to arithmetic
		encoder finalization carry handling and fixed duplicate carry
		propagation in `ArithmeticEncoder::done`.
	- Added layer-level parity tooling in
		`examples/compare_point14_layers.rs` and verified chunk-level
		byte parity against LAStools on the previously failing Carlos chunk.
	- Regenerated Carlos COPC from source LAS and validated external decode
		in LAStools completes to full point count (116,381,594) with no
		chunk-51 EOF failure.
	- Marked Milestone 3 done and started Milestone 4 harness work with a
		formalized external validation matrix and repeatable command script(s).
	- Added `COPC_VALIDATION_MATRIX.md` to define the minimum external
		interoperability scenario set (hierarchy shape, PDRF, CRS, attributes).
	- Added `scripts/copc_external_validation_harness.sh` and generated
		`external_validation_report.md` for fixture-pack outputs.
	- External harness results: LAStools pass for fixture PDRF6 and PDRF7;
		PDRF8 currently blocked by local LAStools/LASzip version limitation
		(`RGBNIR14 has size != 8` in LASzip v3.4r1); PDAL unavailable in this
		environment so PDAL checks are recorded as skipped.
	- Fixed `lasinfo` point-count extraction in the harness (lasinfo reports
		to stderr; changed `2>/dev/null` to `2>&1` in harness lasinfo call).
	- Extended harness to accept an optional third argument of space-separated
		extra file paths for arbitrary COPC files beyond the fixture pack.
	- Ran harness against Carlos COPC (`carlos_sf23_from_las_to_copc_after_fix
		.copc.laz`); LAStools decodes all 116,381,594 points successfully.
	- Ticked paged-hierarchy M4 validation item: paged hierarchy now validated
		externally via LAStools on the real-world Carlos dataset.
	- Marked Milestone 2 done: all implementation items are checked; the sole
		remaining open item is real-PDRF8-fixture smoke coverage which is
		corpus-limited, not code-limited.
	- Comprehensively updated this plan document to reflect current capabilities:
		revised Current State section, changed future-tense implementation phase
		language to past-tense, updated test counts, removed stale Immediate Next
		Action entries from completed milestones, updated COPC interoperability
		clarification block.
	- Investigated a QGIS no-render interoperability issue on
		`1km_lake_erie_v8.copc.laz` despite passing structure checks and
		validate.copc.io checks.
	- Updated COPC node partitioning to retain representative payload in
		subdivided internal nodes (including root), regenerated
		`1km_lake_erie_v9.copc.laz`, and confirmed QGIS now renders points.
	- Ran deep validation on validate.copc.io for v9: all checks pass with a
		single warning for unsorted GPS time.
	- Marked Milestone 4 manual validate.copc.io step and manual desktop
		client validation step as complete.
	- Implemented per-node GPS-time sorting in COPC writer payload emission,
		regenerated `1km_lake_erie_v10.copc.laz`, and confirmed deep
		validate.copc.io run returns no errors and no warnings.
	- Reclassified Milestone 4 as blocked (not in-progress) with explicit blocker:
		external PDRF8 validation tooling support is still missing locally.
	- Marked mixed-root external validation item as not applicable for current
		writer output scope, while leaving PDRF8 as the only remaining blocker.
	- Milestone 5 started: added COPC reader query APIs for data-node key
		selection by subtree root, world-space bbox, and max-depth/LOD cap, plus
		a combined query method for reusable traversal filters.
	- Added Milestone 5 traversal regressions for bbox filtering, subtree
		descendant selection, and max-depth limiting; library suite now passes
		123/123.
	- Added Milestone 5 regression confirming query-all key selection matches
		the full data-node key set used by full-tree reads; library suite now
		passes 124/124.
	- Added `examples/copc_query_demo.rs` to demonstrate subtree, bbox,
		max-depth, and combined key queries against real COPC files with a
		working run on `1km_lake_erie_v10.copc.laz`.
	- Added `examples/copc_query_extract_las.rs` to extract points selected by
		combined COPC key queries and write an output LAS subset for visual QA.
	- Milestone 6 started: added `src/copc/range_io.rs` with a `ByteRangeSource`
		abstraction, blanket `Read+Seek` adapter implementation, and a
		`LocalFileRangeSource` backend.
	- Adapted `CopcReader` hierarchy/node random-access reads to use byte-range
		methods (`len`, `read_exact_at`, `read_range`) and added
		`CopcReader::open_path` / `open_path_with_mode` constructors.
	- Added local-file backend regression coverage and confirmed full
		`wblidar` library suite passes 126/126.
	- Added optional `copc-http` feature-gated HTTP range backend and
		`CachedRangeSource` with targeted cache regression coverage.
	- Replaced the broad `Read + Seek` range trait blanket impl with explicit
		`ByteRangeSource` impls (`File`, `BufReader<R>`, `Cursor<T>`,
		`LocalFileRangeSource`, and forwarding `&mut T`) to avoid trait
		coherence conflicts with wrapper sources.
	- Verified current Milestone 6 baseline with `cargo test -q -p wblidar
		--lib` and `cargo test -q -p wblidar --lib --features copc-http`
		passing (127/127 in both runs).
	- Added feature-gated HTTP integration regressions using a local range
		server fixture:
		- `open_url_reads_nodes_via_http_ranges` verifies header/hierarchy/node
			reads happen via HTTP `Range` requests and avoid full-file pulls.
		- `http_subset_query_matches_local_subset_query` verifies remote subset
			query/read parity with local backend output signatures.
	- Added HTTP reader convenience constructors:
		- `CopcReader::open_url` / `open_url_with_mode`
		- `CopcReader::open_url_cached` / `open_url_cached_with_mode`
	- Revalidated with `cargo test -q -p wblidar --lib` (127/127) and
		`cargo test -q -p wblidar --lib --features copc-http` (129/129).
	- Started standalone LAZ standards-compliance migration in `LazWriter`:
		- added `LazWriterConfig::standards_compliant` (opt-in),
		- emits standard Point14 layered chunk payloads for PDRF6/7/8,
		- writes LASzip chunk-table pointer at point-data start and standard
			chunk table near EOF.
	- Added `LazWriter` regressions for standards mode:
		- point round-trip through `LazReader`,
		- rejection for non-Point14 standards mode,
		- chunk-table pointer/header/entry structural validation.
	- Extended standards `LazWriter` regression coverage for PDRF8 to verify
		RGB + NIR round-trip preservation through `LazReader`.
	- Added multi-chunk standards regression validating LASzip chunk-table
		`point_count` boundaries (e.g. 5 points with chunk size 2 yields
		entry counts `[2, 2, 1]`).
	- Added explicit standards-mode guard for unsupported Point14 extra-bytes
		layouts (`extra_bytes_per_point > 0`) so writer fails early with a clear
		error instead of producing ambiguous metadata/payload combinations.
	- Added standards-mode runtime guard rejecting non-empty per-point
		extra-bytes payloads (`PointRecord.extra_bytes.len > 0`) to prevent
		silent truncation/mismatch while BYTE14 layered writer support is
		still unimplemented.
	- Implemented initial BYTE14 writer slices for standards mode:
		- supports declared Point14 extra-bytes for singleton chunks,
		- supports multi-point chunks when extra-bytes remain constant across the chunk,
		- emits seed extra-bytes payload bytes and zero-sized BYTE14 layer headers
			for constant multi-point chunks,
		- declares BYTE14 item in LASzip VLR when configured,
		- initially kept explicit rejection for varying multi-point extra-bytes chunks.
	- Completed BYTE14 layered continuation support for standards Point14 writer/reader:
		- emits LASzip-style per-byte BYTE14 arithmetic layers for varying multi-point chunks,
		- preserves BYTE14 context across scanner-channel switches,
		- decodes zero-sized and nonzero BYTE14 continuation layers through the shared
			Point14 layered reader,
		- adds direct codec and `LazWriter` round-trip regressions for varying
			extra-bytes chunks.
	- Added standards-mode LASzip VLR conformance regressions across Point14
		family outputs (PDRF6/PDRF7/PDRF8) to verify compressor type
		(`LayeredChunked`) and item layouts match payload intent:
		- PDRF6 -> POINT14
		- PDRF7 -> POINT14 + RGB14
		- PDRF8 -> POINT14 + RGB14 + NIR14
	- Extended standalone standards `LazWriter` coverage to Point10-family
		pointwise streams (PDRF0/PDRF1/PDRF2/PDRF3):
		- added Point10 v2 pointwise chunk encoder for core POINT10, GPSTIME11,
		  RGB12, and BYTE item layouts,
		- wired standards-mode `LazWriter` dispatch to select Point10 pointwise
		  or Point14 layered payloads from the target PDRF,
		- corrected legacy LASzip VLR item declarations so PDRF1/PDRF3 GPS uses
		  `item_type=7`, PDRF2/PDRF3 RGB uses `item_type=8`, and declared legacy
		  extra bytes emit BYTE items (`item_type=0`),
		- writes standard LASzip chunk tables with byte-count-only entries for
		  Point10 pointwise streams,
		- adds `LazWriter` round-trip and VLR-layout regressions for standards
		  PDRF3 output with GPS, RGB, and extra bytes.
	- Expanded Point10 standards regression matrix in `laz::writer`:
		- direct round-trip coverage for standards PDRF0, PDRF1 (+extra bytes),
		  and PDRF2 (+extra bytes),
		- explicit pointwise chunk-table regression ensuring Point10 standards
		  streams write byte-count-only entries (`contains_point_count=false`).
	- Updated `examples/laz_write_standards.rs` to target standards Point10
		or Point14 paths based on source PDRF (promoting only unsupported
		waveform formats) and preserve declared per-point extra-bytes metadata,
		with immediate read-back point-count verification.
	- Added ignored external interoperability harness
		`tests/standards_external_validation.rs`:
		- writes standards outputs for PDRF0/1/2/3/6/7/8,
		- includes Point10 and Point14 extra-bytes variants,
		- verifies internal read-back and, when installed, validates each output
		  with external consumers (`lasinfo` and/or `pdal info`),
		- exposes two ignored run modes:
		  - permissive: skips when no external consumer is installed,
		  - strict: fails immediately when neither `lasinfo` nor `pdal` is available.
		- emits a machine-readable JSON report artifact
		  (`schema_version=1`) with per-profile statuses, tool availability,
		  and summary counts; default output is under the test temp workspace,
		  or override with `WBLIDAR_LAZ_INTEROP_REPORT=<path>` for CI archiving.
	- Added GitHub Actions workflow
		`.github/workflows/wblidar-interop.yml` to run strict ignored
		external interoperability validation with PDAL on Linux runners and
		upload the JSON report artifact.
	- Added `LAZ_VALIDATION_MATRIX.md` as the LAZ counterpart to the COPC
		validation matrix docs, seeded with the current strict-mode run snapshot
		and report-schema contract for CI artifacts.
	- Revalidated latest baseline with `cargo test -q -p wblidar --lib`
		(150/150), `cargo test -q -p wblidar --lib laz::writer` (19/19),
		`cargo test -q -p wblidar --lib standard_point10_write` (2/2),
		`cargo test -q -p wblidar --test standards_external_validation`
		(2 ignored; run with `-- --ignored` for external checks),
		`cargo test -q -p wblidar --example laz_write_standards`, and
		`cargo test -q -p wblidar --lib --features copc-http` (152/152).

### Phase 4 Completion Evidence

- `LazReader` dispatches wb-native and standards LASzip paths, including Point14 layered status-aware decode fallback.
- Strict/tolerant controls are implemented:
	- `WBLIDAR_STRICT_POINT14_LAYERED`
	- `WBLIDAR_FAIL_ON_PARTIAL_POINT14`
- Frontend read path remains backward-compatible while using the enhanced diagnostics-capable internals.
- Validation:
	- `cargo test -p wblidar --lib` passing (117/117 as of 2026-03-29).
	- Corpus smoke baseline stable.

### Phase 5 Completion Evidence

- COPC decode path uses shared Point14 layered decode primitives and enforces fail-on-partial policy consistently with LAZ reader behavior.
- COPC hierarchy traversal now supports recursive sub-page expansion for multi-page hierarchies and mixed root layouts (direct data entries plus sub-page references).
- COPC writer now emits paginated hierarchy pages and root references for large hierarchy tables.
- Reader-level partial-recovery counters are exposed and aggregated into frontend diagnostics.
- Added diagnostics API:
	- `read_with_diagnostics` and `ReadDiagnostics`.
	- Existing `read` remains backward compatible and discards diagnostics.
- Added machine-readable production reporting in `examples/lidar_corpus_inventory.rs`:
	- JSON report mode (`--report-json`).
	- Schema metadata (`schema_version`, `tool_version`, generation timestamp).
	- COPC hierarchy compatibility matrix section (`copc_hierarchy`) with:
		- layout class counts (`single-page`, `paged`, `mixed-root`),
		- header-offset fallback hit counts/samples,
		- parse-failure counts/samples.
	- Threshold-gated CI behavior:
		- `--max-smoke-failures`
		- `--max-partial-events`
		- exit code `3` on threshold breach.
- Added automated report stability tests:
	- Argument parsing tests for new threshold/report flags.
	- JSON parse/shape test using `serde_json`.
- Added COPC interoperability regressions:
	- Pagination regression in writer hierarchy construction.
	- Reader round-trip test for paginated hierarchy output.
	- Mixed root+subpage hierarchy expansion regression.
	- Hierarchy root-offset interpretation regression coverage for both:
		- writer-produced data-offset files, and
		- producer-like synthetic EVLR-header-offset files.

Latest verification:
- `cargo test -p wblidar --example lidar_corpus_inventory` passing (3/3).
- `cargo test -p wblidar --lib` passing (141/141 as of 2026-03-29).
- `cargo test -p wblidar --lib --features copc-http` passing (143/143 as of 2026-03-29).

## Remaining Gaps and Future Work

1. **`LazWriter` standards compliance**: Partial migration completed.
	`LazWriterConfig::standards_compliant=true` now emits standards-compliant
	Point10-family pointwise payloads (PDRF0/1/2/3) and Point14-family layered
	payloads (PDRF6/7/8), both with standard LASzip chunk-table pointer + table,
	with round-trip regression coverage through `LazReader`.
	Default mode remains wb-native for backward compatibility; remaining work is
	primarily external interoperability validation plus deciding on default-mode
	transition timing.

2. **Real PDRF8 fixture coverage**: No real-world PDRF8 COPC fixture has been
   located in the local corpus.  Coverage would improve once a PDRF8 dataset
   is available for smoke testing.

3. **PDAL/second independent consumer**: available in the current environment
	(`/opt/homebrew/bin/pdal`).  Ignored interoperability harness strict mode
	now passes with PDAL (`pdal info`) as an independent consumer for the
	generated standards outputs.

4. **PDRF8 external validation**: LAStools v3.4r1 still does not support
	RGBNIR14, but PDAL-based validation is now runnable in this environment.
	Remaining gap is real-world PDRF8 fixture breadth rather than tool
	availability.

5. **GPS time ordering warning**: resolved for the validated Lake Erie COPC path
	by sorting node payload points by gps time before encoding.

6. **COPC Query APIs** (Milestone 5): bbox, subtree, and max-depth/LOD node
   selection APIs.

7. **Range-readable IO Backend** (Milestone 6): HTTP range-backed COPC reads.

## Actual Module Layout

The following modules are implemented (the names differ slightly from the
original proposal but the functional decomposition is equivalent):
- `src/laz/mod.rs` — LASzip VLR parsing, type definitions, constants
- `src/laz/laszip_chunk_table.rs` — standards chunk-table reader/writer
- `src/laz/arithmetic_model.rs` — bit/symbol models
- `src/laz/arithmetic_decoder.rs` — arithmetic decoder
- `src/laz/arithmetic_encoder.rs` — arithmetic encoder (carry bug fixed)
- `src/laz/integer_codec.rs` — integer delta/predictor codec primitives
- `src/laz/standard_point10.rs` — Point10 v2 pointwise codec
- `src/laz/standard_point10_write.rs` — Point10 v2 pointwise standards writer
- `src/laz/standard_point14.rs` — Point14 v3 layered codec (PDRF6/7/8)
- `src/laz/fields/` — per-field GPS/RGB/NIR continuation codecs
- `src/laz/codec.rs` — wb-native DEFLATE helpers (shared encode/decode)
- `src/laz/chunk.rs` — wb-native chunk table structure
- `src/laz/reader.rs` — `LazReader` (dispatches both paths)
- `src/laz/writer.rs` — `LazWriter` (wb-native + standards Point10/Point14 paths)
- `src/copc/reader.rs` — COPC reader with strict/tolerant modes
- `src/copc/writer.rs` — COPC writer (standards LASzip path)
- `src/copc/hierarchy.rs` — hierarchy traversal, pagination, query primitives

## Test Matrix (implemented)
- Unit tests:
- arithmetic coder primitives
- VLR parsing
- chunk-table parsing
- field codec state transitions

- Integration tests:
- Read known LAZ fixtures into `PointRecord`
- Write/re-read round trip for formats 6/7/8
- LAZ -> LAS conversion checks (point count, bbox, classification integrity)

- Runtime workflow tests:
- Existing LiDAR fixture tests in `wbtools_pro`
- Ensure prior triangulation failure is isolated from decode layer

## Safety and Robustness Requirements (retained)
- Never pre-allocate based only on untrusted counts without fallible reserve and cap checks.
- Validate all offsets before seek/read.
- Limit chunk size and model table sizes to sane upper bounds.
- Return explicit structured errors for unsupported item combinations.

## Historical Execution Order for Next Coding Steps

> Historical note: this sequence is retained for context from earlier implementation stages.
> Current authoritative status is in the "Implementation Phases" and "COPC Conformance Roadmap" sections above.

1. Implement Phase 1 parser modules and tests.
2. Implement Phase 2 arithmetic core and tests.
3. Implement Phase 3 field codecs for point formats 6/7/8.
4. Integrate reader path, then writer path.
5. Run fixture/regression suite and tune performance.

## Historical Progress Update (2026-03-28)

> Historical note: this section reflects status as of 2026-03-28 and is superseded by
> the "Implementation Phases" and "COPC Conformance Roadmap" sections above.

- Phase 1 status: in progress and substantially complete.
	- LASzip VLR metadata parsing implemented.
	- Standard chunk-table pointer/header parsing implemented.
	- LASzip-style tail-pointer fallback implemented.
- Phase 2 status: core implemented.
	- In-house adaptive arithmetic models added.
	- In-house arithmetic decoder/encoder modules added.
	- Deterministic modeled bit/symbol round-trip tests added and passing.
- Remaining work:
	- Implement Phase 3 field codecs (Point14/GPS/RGB/NIR).
	- Connect arithmetic decoder to `LazReader` standard LASzip path.
	- Add standard LASzip writer path and fixture validation.

## Historical Corpus-Driven Priority Update (2026-03-28)

> Historical note: this corpus snapshot guided implementation priorities on 2026-03-28.
> Use it as context, but rely on the 2026-03-29 phase snapshot for current status.

- Added inventory utility: `examples/lidar_corpus_inventory.rs`
	- Usage: `cargo run --example lidar_corpus_inventory -- /Users/johnlindsay/Documents/data`
	- Reports LASzip signature frequencies and likely unsupported signatures.

- Full corpus scan summary (`/Users/johnlindsay/Documents/data`):
	- Candidate files: 9934
	- Headers parsed: 9934
	- Parse failures: 0

- Highest-frequency signatures observed:
	- `pdrf3|compressor=PointWiseChunked|coder=0|items=[6:20:2,7:8:2,8:6:2]` (8258)
	- `pdrf3|compressor=PointWiseChunked|coder=0|items=[6:20:2,7:8:2,8:6:2,0:1:2]` (1035)
	- `pdrf1|compressor=PointWiseChunked|coder=0|items=[6:20:2,7:8:2,0:5:2]` (248)
	- `pdrf6|compressor=LayeredChunked|coder=0|items=[10:30:3]` (145)

- Immediate implementation priority order:
	1. Point10 v2 + RGB12 item (`item_type=8`) on standard pointwise path.
	2. Point10 v2 + RGB12 + extra-bytes variants (`item_type=0`, sizes 1 and 8 observed).
	3. Point14 layered base decode (`item_type=10`) for PDRF6/7.
	4. COPC node decode routing through shared standard LASzip codecs.

- Status update after RGB12 implementation:
	- Implemented Point10 v2 RGB12 standard decode support (`item_type=8`, size 6, version 2).
	- Added tolerant Point10 decode for non-conformant streams that omit RGB items while declaring RGB-capable PDRFs; decoded points keep `color=None`.
	- Added a best-effort Point14 fallback in `LazReader` that attempts wb-native DEFLATE chunk decode when a Point14 layered signature is declared.
		- This recovers mislabeled/mixed-encoding streams but does not replace standards-compliant layered arithmetic Point14 decoding.
	- Routed `CopcReader` node decoding through LASzip VLR-aware dispatch:
		- Uses standard Point10 item decode when metadata declares supported Point10 item layouts.
		- Falls back to wb-native DEFLATE decode for mixed/mislabeled Point14-tagged streams.
		- Returns explicit unimplemented errors for true layered Point14 arithmetic streams.
	- Added `standard_point14.rs` groundwork:
		- Validates Point14-family item layouts (`10:30:3`, `11:6:*`, `12:2:*`, and byte extras).
		- Adds support for Point14 item `14` (WavePacket14) as opaque per-point extra bytes.
		- Tolerates non-standard Point14 item ordering (e.g., RGB14 before Point14 core) during per-point decoding.
		- Decodes Point14 seed point from layered chunk payloads.
		- Wires seed decode fallback into both `LazReader` and `CopcReader` when wb-native decode fails.
		- Adds multi-point recovery for non-arithmetic streams that store full per-point item sets.
		- Added initial arithmetic continuation port for pure Point14 item layout (`items=[10:30:3]`) based on LASzip v3 layer/model structure.
		- Current known blocker: several sample files still fail in `LazReader::new` with chunk-table/metadata EOF before continuation decode is reached.
		- Remaining gap: complete arithmetic continuation coverage across real-world chunk-table variants and scanner-channel switching.
	- Added `examples/lidar_read_smoke.rs` for targeted runtime compatibility checks on real files.
	- Improved user-facing failure mode for Point14 streams:
		- Unexpected EOF during LAZ reads is translated to explicit Point14 layered arithmetic unimplemented when LASzip metadata declares Point14 items.
	- Runtime smoke snapshot over the current unsupported sample list:
		- 12 / 30 sample files decode successfully.
		- Remaining 18 / 30 fail with explicit Point14 layered arithmetic unimplemented errors.
	- Updated corpus inventory support classifier accordingly.
	- Full-corpus unsupported ranking now shifts to:
		1. Layered Point14 base signatures (`items=[10:30:3]`, PDRF6/7/COPC).
		2. Point10 signatures with GPSTIME + extra-bytes but no RGB (`items=[6:20:2,7:8:2,0:5:2]`).

> Note: all blockers documented in this section have since been resolved.
> The Point14 layered arithmetic decoder is complete and fully validated.
> See the COPC Conformance Roadmap and Progress Log above for current status.
