# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

### Added
- Reference-tolerance formalization artifacts:
	- `wbhdf::compare` now includes `compare_f64_exact` and
	  `compare_f64_with_tolerance` with unit coverage,
	- VIIRS `XDim` reference validation now uses reusable `f64` tolerance helpers,
	- `docs/internal/HDF_REFERENCE_TOLERANCE_MATRIX.md` documents explicit tolerance
	  contracts for currently validated GEDI/VIIRS reference paths.
- Explicit MODIS scope-boundary runbook in
	`docs/internal/HDF_MODIS_SCOPE_BOUNDARY.md` defining named in-scope companion
	families (`MOD09`/`MYD09`, `MOD11`/`MYD11`, `MOD13`/`MYD13`) and default-enable
	wording/diagnostic constraints.
- Repeatable default-enable smoke script
	`scripts/run_default_enable_smoke.sh` to run Tier 1 HDF dispatch checks,
	representative non-HDF `wbraster` regressions, and core `wbhdf` multilevel
	traversal regressions from a single command path.
- Additional multilevel traversal hardening in `src/btree.rs`:
	- recursion-path cycle detection for internal-node traversal,
	- explicit `UnsupportedLayout` diagnostics when internal-node cycles are detected,
	- regression coverage in `btree::tests::reports_internal_node_cycle_as_unsupported`.
- Default-enable roadmap gate evidence updates in
	`docs/internal/HDF5_SCOPED_READER_ROADMAP.md` marking completion evidence for:
	- Tier 1 supported-layout matrix documented/tested,
	- fail-fast unsupported-layout diagnostics coverage,
	- direct `wblidar` no-preconversion validation for currently validated Tier 1 paths.
- Fresh smoke-matrix execution evidence recorded in
	`docs/internal/HDF5_SCOPED_READER_ROADMAP.md`:
	- end-to-end pass of `scripts/run_default_enable_smoke.sh` after traversal hardening,
	- non-HDF regression gate item marked complete from current run output.
- Added a second real-fixture multilevel traversal probe in
	`tests/integration_tests.rs`:
	- `viirs_vnp21_lst_bounded_chunk_index_probe_returns_expected_chunk_records`
	  validates non-leaf root detection, bounded chunk-record retrieval, and direct
	  chunk-payload decode viability for VNP21 LST chunk-index traversal.
- Added a third real-fixture multilevel traversal probe in
	`tests/integration_tests.rs`:
	- `viirs_vnp21_latitude_bounded_chunk_index_probe_returns_expected_chunk_records`
	  validates non-leaf root detection, bounded non-origin chunk-record retrieval,
	  and direct chunk-payload decode viability/in-range geolocation values for
	  VNP21 latitude chunk-index traversal.
- Bounded chunked multilevel traversal hardening in `src/btree.rs`:
	- fail-fast `UnsupportedLayout` diagnostics for invalid internal child addresses
	  (`0` or `u64::MAX`) before recursive descent,
	- regression coverage in
	  `btree::tests::reports_invalid_internal_child_address_as_unsupported`, and
	- targeted non-regression confirmation for existing multilevel internal-fanout and
	  budget-exhaustion traversal paths.
- Default-enable rollback runbook in
	`docs/internal/HDF_DEFAULT_ENABLE_ROLLBACK_PLAN.md` defining:
	- fast operational guardrail rollback for `wbraster` HDF URI dispatch,
	- hard scope-clamp fallback for HDF5/NetCDF URI materialization,
	- post-rollback verification matrix, and
	- incident logging + re-enable criteria.
- Initial crate scaffolding for `wbhdf`.
- Module skeletons for error handling, superblock, object headers, datasets, chunk indexing, filters, datatypes, and attributes.
- Explicit `Endianness`-aware decode helpers in `src/datatypes.rs` for `F32`, `F64`, and `I16` scalar/slice payload decoding, with synthetic little-endian and big-endian test coverage.
- Contiguous layout payload reading in `src/object_header.rs` plus fixed-string decode support in `src/datatypes.rs`, with real ATL08 validation of the first object-header chain through payload bytes and XML prefix decoding.
- Real GZIP filter decompression in `src/filters.rs` using a pure-Rust decoder, with synthetic success/failure test coverage.
- Placeholder unit and integration test harnesses.
- Internal design and format-notes documentation stubs.
- Canonical fixture helpers in `src/fixtures.rs` for external fixture directory resolution and smoke fixture detection.
- Initial fixture manifest at `tests/fixtures/manifest.toml` for GEDI, ICESat-2, VIIRS, and MODIS target paths, with the current local GEDI and ATL08 Tier 1 samples wired as the active external references.
- Real-fixture validation evidence for the metadata smoke path using a local ICESat-2 ATL08 `.h5` sample via `WBHDF_SMOKE_FILE`.
- Real-fixture integration tests that resolve ATL08 and GEDI samples from `WBHDF_FIXTURE_DIR` and assert heuristic group discovery on both families.
- A minimal file-backed dataset-path probe in `src/dataset.rs` with:
	- contiguous marker detection,
	- split path-component fallback for ATL08-style layouts, and
	- real-fixture assertions for ATL08 canopy and GEDI `BEAM0000` dataset markers.
- Object-header signature probing in `src/object_header.rs`:
	- `OHDR` signature-offset discovery for raw bytes and file-backed probes,
	- v2 object-header prefix decoding (version/flags/chunk0 size) where signatures are valid,
	- bounded dataspace and datatype body decoding for first-chunk messages,
	- bounded first-chunk message-header decoding for parsed v2 headers,
	- header-continuation body decoding into `(address, size)` targets,
	- bounded `OCHK` continuation-chunk parsing for decoded continuation targets,
	- bounded contiguous layout-message decoding from continuation chunks,
	- unit coverage for positive and negative parse paths,
	- real-fixture assertions confirming ATL08 has parsable v2 object-header prefixes with dataspace/datatype bodies, first-chunk message IDs `[0x01, 0x03, 0x05, 0x10]`, first continuation target `(144130, 52)`, first continuation-chunk message IDs `[0x10, 0x15]`, and chunk-2 contiguous layout `(8500138, 38726)`, while GEDI has discoverable `OHDR` markers.
- Fixture-backed integration tests now pin ATL08 and GEDI assertions to the exact sample filenames recorded in `tests/fixtures/manifest.toml`, avoiding nondeterministic `read_dir()` ordering across multiple ATL08 granules.
- Day 2 metadata smoke-path probe API in `superblock`:
	- HDF5 signature validation,
	- minimal superblock version extraction,
	- heuristic top-level group discovery.
- Day 3 B-tree v1 kickoff implementation in `src/btree.rs`:
	- typed node header / internal record / leaf record parsing,
	- key-range child routing helper,
	- deterministic chunk-address lookup API with explicit error reporting.
- Synthetic B-tree unit tests and integration smoke test that gracefully skips when fixtures are unavailable.
- Day 4 dataset chunk-lookup validation path:
	- added `DatasetChunkLocator` in `src/dataset.rs` to wire B-tree lookup into dataset-level flow,
	- added known-address validation tests for deterministic key->address expectations,
	- documented initial chunk-key/routing assumptions in `docs/FORMAT_NOTES.md`.
- Bounded HDF5 v1 object-header parsing in `src/object_header.rs` for chunked numeric datasets:
	- v1 message-header parsing,
	- bounded dataspace/datatype/filter-pipeline/chunked-layout/continuation body decoding,
	- file-backed v1 parse entrypoint for known object-header offsets, and
	- synthetic + real-fixture assertions for ATL08 `h_canopy` metadata (`offset=328097`, deflate filter level 6, chunk index address 326001, chunk dims `{10000, 4}`).
- Initial bounded chunk decode pipeline for ATL08 `h_canopy`:
	- chunked-storage v1 B-tree first-leaf record parsing in `src/btree.rs`,
	- file-backed compressed chunk payload reads by decoded `(address, size)`,
	- zlib/deflate decompression support in `src/filters.rs` for HDF5 deflate-filter chunks,
	- real-fixture integration coverage decoding the first `h_canopy` chunk end-to-end to `10000` little-endian `f32` values.
- Bounded v1 fill-value message parsing in `src/object_header.rs` with fixture-backed ATL08 `h_canopy` assertions:
	- decoded fill metadata fields (`version`, allocation/fill timing, defined flag, value size),
	- decoded raw fill payload bytes (`ff ff 7f 7f`), and
	- validated little-endian `f32` fill sentinel value (`f32::MAX`).
- Deterministic `f32` fill-to-nodata mapping in `src/dataset.rs`:
	- new dataset materialization helper `apply_fill_value_mapping_f32` with explicit `nodata_value`,
	- deterministic fill matching by bit-pattern comparison,
	- returned `valid_count` and `nodata_count` for downstream diagnostics,
	- synthetic mapping unit tests and real ATL08 first-chunk integration assertions (`valid=3640`, `nodata=6360`).
- Day 3 comparison harness foundation in `src/compare.rs`:
	- `compare_f32_exact` for strict reference checks,
	- `compare_f32_with_tolerance` for toleranced `f32` validation,
	- structured mismatch summaries (`mismatches`, `max_abs_diff`, first mismatch index),
	- unit coverage for exact/toleranced match cases plus mismatch and invalid-input paths.
- First GEDI fixture-backed reference-comparison checkpoint:
	- new dataset utility `read_contiguous_f32_window_in_file` in `src/dataset.rs` for bounded contiguous `f32` window reads,
	- integration test `gedi_elev_lowestmode_contiguous_window_matches_h5dump_reference` validating `/BEAM0000/elev_lowestmode` first-window values against `h5dump` reference output,
	- corresponding layout/validation notes captured in `docs/FORMAT_NOTES.md`.
- First VIIRS fixture-backed reference checkpoint:
	- added `WBHDF_VIIRS_FIXTURE_DIR` support in `src/fixtures.rs` for dedicated VIIRS sample roots,
	- new dataset utility `read_contiguous_f64_window_in_file` in `src/dataset.rs` for bounded contiguous `f64` window reads,
	- integration test `viirs_vnp13_xdim_contiguous_window_matches_h5dump_reference` validating VNP13 X-dimension reference values from
	  `/HDFEOS/GRIDS/VIIRS_Grid_8Day_VI_500m/XDim`,
	- fixture manifest VIIRS entry now points to a concrete available HDF5 product path and dataset marker.
- MODIS fixture intake and HDF4 feasibility smoke coverage:
	- added `WBHDF_MODIS_FIXTURE_DIR` support in `src/fixtures.rs`,
	- integration smoke tests verify local MODIS fixtures for `MOD09A1`, `MOD11A2`, `MOD13A1`, `MYD11A2`, and `MYD13A1` exist and carry HDF4 magic signature bytes,
	- fixture manifest now includes concrete local `MOD09A1`/`MOD13A1`/`MOD11A2` entries for targeted follow-up decode work.
- Minimal HDF4 EOS metadata probe foundation in `src/hdf4.rs`:
	- validates HDF4 signature,
	- enumerates structured EOS metadata from embedded `StructMetadata.0` text:
	  - `GridName`
	  - `DataFieldName`
	  - `DataType`
	  - `DimList`,
	  - grid dimension sizes from `XDim=`/`YDim=` tokens,
	  - grid georeferencing tokens from `UpperLeftPointMtrs`, `LowerRightMtrs`, `Projection`, `ProjParams`, and `SphereCode`,
	- adds `resolve_hdf4_grid_field(...)` for resolving field descriptors into inferred array shape,
	- adds `resolve_hdf4_dataset_path(...)` for canonical path resolution (`/GridName/DataFieldName`) into the same resolved field descriptor,
	- adds `enumerate_hdf4_dataset_paths(...)` for sorted canonical path inventories derived from parsed grid metadata,
	- adds `prepare_hdf4_sds_decode_attempt(...)` to preflight canonical MODIS-style SDS paths into explicit decode-attempt descriptors,
	- adds `decode_hdf4_sds_i16_in_file(...)` as a first payload-read bridge entrypoint that currently emits deterministic `UnsupportedLayout` diagnostics with resolved field context while HDF4 SDS byte decode is pending,
	- adds `derive_hdf4_grid_geometry(...)` and `derive_hdf4_grid_geometry_for_dataset(...)` for deriving raster rows/cols, pixel spacing, and GDAL-style geotransform from resolved HDF4 metadata,
	- adds `assess_hdf4_sds_i16_decode_readiness(...)` for structured preflight readiness reports (resolved metadata, optional geometry, blocker list),
	- adds HDF4 DD table traversal (`parse_hdf4_data_descriptors*`) and expected-size payload candidate discovery (`find_hdf4_sds_i16_payload_candidates*`) for SDS i16 decode preflight,
	- adds `assess_hdf4_sds_i16_decode_readiness_in_file(...)` to enrich blockers with descriptor-scan outcomes from real files,
	- adds ranked payload-candidate inspection (`rank_hdf4_sds_i16_payload_candidates*`) including length-delta scoring, signature hints, and preview bytes for descriptor-to-field mapping diagnostics,
	- adds heuristic descriptor-to-field mapping (`map_hdf4_sds_i16_descriptor_heuristic*`) with deterministic selection rationale to support SDS decode targeting,
	- enriches in-file readiness blockers with heuristic mapping summaries when exact descriptor-to-field mapping is unresolved,
	- adds bounded payload probe APIs (`probe_hdf4_sds_i16_payload_window*`) to classify heuristic-selected descriptor payloads and provide i16 preview vectors when binary bytes are directly decodable,
	- extends payload probes with bounded gzip/zlib decode attempts for compressed-signature candidates, including deterministic failure diagnostics when compressed probe decode is not successful,
	- adds `attempt_decode_hdf4_sds_i16_window_in_file(...)` for bounded little-endian i16 window decode returns when probe decoding succeeds,
	- adds heuristic confidence labels (`high`/`medium`/`low`/`none`) to descriptor mapping diagnostics for clearer decode targeting,
	- returns a probe-derived little-endian preview window from `decode_hdf4_sds_i16_in_file(...)` when the mapped payload can be decoded,
	- compares little-endian and big-endian previews with a plausibility score and returns the more realistic window when both are available,
	- aligns `attempt_decode_hdf4_sds_i16_window_in_file(...)` with the same preview plausibility selection logic used by `decode_hdf4_sds_i16_in_file(...)`, avoiding contradictory endian assumptions across decode entrypoints,
	- adds `decode_hdf4_sds_i16_window_at_in_file(path, dataset_path, start_value, max_values)` for bounded descriptor-mapped SDS window extraction with start-offset support,
	- routes `attempt_decode_hdf4_sds_i16_window_in_file(...)` through the new start-offset decode path to keep decode behavior centralized and consistent,
	- adds fixture-backed integration coverage for deterministic out-of-bounds diagnostics when offset-window start indices exceed decoded payload length,
	- adds fixture-backed integration coverage for deterministic invalid-input diagnostics when offset-window calls use `max_values=0`,
	- adds synthetic unit coverage for deterministic compressed-payload limit diagnostics when offset-window decode encounters compressed descriptor candidates larger than `MAX_COMPRESSED_WINDOW_DECODE_BYTES`,
	- updates offset-window decode to try deterministic exact-length in-bounds descriptor candidates before heuristic fallback, reducing heuristic dependence when multiple candidate descriptors exist,
	- adds unit coverage for exact-length candidate preference (binary payload selected over textual payload when both exact-size candidates exist),
	- adds deterministic exact-candidate phase failure diagnostics (`attempted=N/M`) to improve troubleshooting when all exact-length decode candidates fail,
	- enriches readiness and unsupported decode diagnostics with probe status/rationale and preview context,
	- enriches unsupported SDS decode diagnostics with derived geotransform context when available,
	- propagates parsed grid georeferencing metadata onto resolved-field outputs for downstream raster geometry wiring,
	- unit coverage for extraction and invalid-signature handling,
	- integration coverage on local MODIS fixtures (`MOD09A1`, `MOD11A2`, `MOD13A1`) asserting expected grid/data-field discovery, canonical path resolution, inferred shapes, parsed georeferencing metadata, derived geometry values, descriptor enumeration/candidate-scan behavior, structured SDS-readiness blockers, and deterministic unsupported SDS-decode diagnostics for `sur_refl_b01`.
- Internal design note `docs/internal/HDF_MODULE_BOUNDARIES.md` added to define HDF5 vs HDF4
	module responsibilities and integration contracts for downstream consumers.
- Error taxonomy hardening for decode paths:
	- `WbhdfError` now includes explicit `DatatypeMismatch`, `InvalidChunk`, and `FilterFailure`
	  variants in addition to `UnsupportedLayout` for clearer decode failure classification,
	- HDF4 SDS bounded-window decode now emits these variants with structured context fields
	  (`dataset_path`, `chunk_coordinate`, `file_offset`) on datatype mismatch, descriptor-boundary,
	  and filter decode failure paths,
	- exact-candidate decode diagnostics now preserve attempted-candidate context for structured
	  chunk/filter failures.
- Added concise crate-level targeted-read API examples in `src/lib.rs` for:
	- contiguous `f32` window reads with known byte offsets, and
	- bounded HDF4 SDS i16 window reads by canonical dataset path.

- Expanded VNP21 swath-field payload validation coverage with fixture-backed
	bounded-window decode checks for:
	- `View_angle`,
	- geolocation `latitude`/`longitude` (`f32` row-major helper path),
	- `Emis_ASTER`,
	- `PWV`,
	- `QC`, `oceanpix`,
	- `Emis_14`/`Emis_15`/`Emis_16`, and
	- `Emis_14_err`/`Emis_15_err`/`Emis_16_err`.
- Added cross-field VNP21 semantic contract tests for thermal and emissivity
	families, including fill/valid-range/scale behavior guards.
- Added VNP21 QA-focused cross-field regressions for `QC`/`oceanpix`:
	- stable bit-pattern contracts,
	- observed bitfield-interpretation contracts,
	- observed-state histogram contracts, and
	- row-alignment contracts.
- Added an additional bounded non-origin VNP21 `QC`/`oceanpix` reference window
	(`start=(1000,1007), count=(2,10)`) that broadens observed QA-state coverage
	to include `QC=64192` against explicit `h5dump` references.
- Added observed-bit semantics regression coverage for that additional oceanpix=0
	slice (`viirs_vnp21_qc_observed_bits_in_additional_oceanpix0_window_are_stable`),
	asserting stable decoded bit-components across observed `QC=64192` and
	`QC=65216` states.
- Added cross-window observed-profile classification regression for VNP21 QC
	(`viirs_vnp21_qc_observed_profile_classification_is_stable_across_windows`),
	validating stable fixture-level QC profile classes and paired oceanpix counts
	across multiple bounded windows.
- Added origin-baseline-inclusive observed-profile classification regression for
	VNP21 QC
	(`viirs_vnp21_qc_observed_profile_classification_with_origin_baseline_is_stable`),
	tightening profile/pairing contracts across three bounded windows.
- Added deterministic known/unknown profile-classifier regression for VNP21 QC
	(`viirs_vnp21_qc_observed_profile_classifier_maps_known_and_unknown_states`),
	locking stable mappings for known observed QC states and explicit unknown
	fallback behavior.
- Added inland-water QA profile regression for VNP21 QC/oceanpix
	(`viirs_vnp21_qc_oceanpix_inland_window_profile_contract_is_stable`),
	validating an additional bounded window with `QC=7` paired to
	`oceanpix=1` and extending observed profile-category coverage.
- Added exhaustive multi-window QC/oceanpix profile-contract regression
	(`viirs_vnp21_qc_oceanpix_profile_contract_is_exhaustive_across_key_windows`),
	asserting stable profile/oceanpix pairing counts across key bounded windows
	with explicit no-unknown-classification guardrails.
- Added cross-window QC profile-to-bit invariant regression
	(`viirs_vnp21_qc_profile_bit_invariants_are_stable_across_key_windows`),
	enforcing stable observed bit semantics (`high_byte`, `low_byte`, and low-bit flags)
	for each known fixture profile and rejecting unknown-profile drift.
- Added raw QC-state whitelist-by-category regression for VNP21
	(`viirs_vnp21_qc_raw_state_whitelist_by_oceanpix_is_stable_across_key_windows`),
	enforcing stable raw `QC` value whitelists for each observed `oceanpix` class and
	rejecting category drift in the validated key-window set.
- Added non-overlapping QC/oceanpix profile-cluster regression for VNP21
	(`viirs_vnp21_qc_nonoverlapping_window_cluster_profile_contract_is_stable`),
	locking stable inland-known (`QC=7`, `oceanpix=1`) and land-unknown
	(`QC=15/50`, `oceanpix=0`) profile behavior in a second bounded window cluster.
- Added documented QA/category vocabulary discoverability regression for VNP21
	(`viirs_vnp21_qc_oceanpix_documented_semantics_vocabulary_is_discoverable_and_consistent`),
	asserting documented metadata vocabulary visibility and bounded-window consistency
	with the full documented `oceanpix` value space (`0..2`).
- Added documented QC vocabulary + observed bitfield-family consistency regression
	for VNP21 (`viirs_vnp21_qc_documented_semantics_vocabulary_and_observed_bitfield_families_are_consistent`),
	coupling documented QC metadata vocabulary discoverability with stable bounded-window
	high/low-byte family expectations for observed QC states.
- Added reusable dataset metadata-text assertion utilities
	(`dataset_metadata_contains_text_in_file`, `dataset_metadata_contains_all_texts_in_file`)
	in `wbhdf::attributes` and migrated VNP21 documented-semantics regressions to use
	utility-backed metadata assertions instead of direct fixture-byte scans.
- Added `dataset_metadata_missing_texts_in_file` utility in `wbhdf::attributes`
	and upgraded spec-aligned VNP21 metadata assertions to require empty missing-term
	lists, yielding explicit missing-vocabulary diagnostics on failure.
- Added report-style metadata utility
	(`dataset_metadata_text_report_in_file`) that returns present and missing terms,
	and upgraded spec-aligned VNP21 semantics assertions to emit present/missing
	vocabulary diagnostics directly in failure messages.
- Added report-based documented field-vocabulary regression for VNP13
	(`viirs_vnp13_documented_vi_field_vocabulary_is_discoverable_with_reports`),
	validating NDVI/EVI/EVI2 metadata vocabulary discoverability with explicit
	present/missing diagnostics.
- Added report-based documented field-vocabulary regressions for MODIS core
	fixtures (`modis_mod09_documented_field_vocabulary_is_discoverable_with_reports`,
	`modis_mod11_documented_field_vocabulary_is_discoverable_with_reports`,
	`modis_mod13_documented_field_vocabulary_is_discoverable_with_reports`),
	validating MOD09/MOD11/MOD13 grid/field metadata vocabulary discoverability
	with explicit present/missing diagnostics.
- Added report-based documented field-vocabulary regressions for MODIS Aqua
	companions (`myd09_documented_field_vocabulary_is_discoverable_with_reports`,
	`myd11_documented_field_vocabulary_is_discoverable_with_reports`,
	`myd13_documented_field_vocabulary_is_discoverable_with_reports`), validating
	MYD09/MYD11/MYD13 grid/field metadata vocabulary discoverability with explicit
	present/missing diagnostics.
- Added report-based documented swath-vocabulary regression for VNP09
	(`viirs_vnp09_documented_swath_vocabulary_is_discoverable_with_reports`),
	validating I-band/M-band/QF metadata vocabulary discoverability with explicit
	present/missing diagnostics.
- Updated metadata-text utilities to support non-slash swath metadata token
	anchors in addition to slash-prefixed dataset paths, enabling report-based
	diagnostics on HDF4 swath fixtures that do not expose enumerated dataset paths.
- Added report-based documented field-vocabulary regressions for VIIRS HDF5
	fixtures (`viirs_m3_documented_field_vocabulary_is_discoverable_with_reports`,
	`viirs_i4_documented_field_vocabulary_is_discoverable_with_reports`),
	validating M3/I4 science and geolocation metadata vocabulary discoverability
	with explicit present/missing diagnostics.
- Added report-based documented field-vocabulary regressions for ATL08/GEDI
	fixture probes (`atl08_documented_field_vocabulary_is_discoverable_with_reports`,
	`gedi_documented_field_vocabulary_is_discoverable_with_reports`), validating
	canonical beam/segment/science metadata vocabulary discoverability with
	explicit present/missing diagnostics.
- Added a roadmap plan-alignment checkpoint in
	`docs/internal/HDF5_SCOPED_READER_ROADMAP.md` to clarify current Phase/Week
	position and the practical interpretation of readiness percentages.
