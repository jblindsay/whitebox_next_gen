# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [0.1.3] - 2026-04-21

### Added
- Added an execution-tracked JPEG2000 bridge retirement implementation plan in
  `docs/internal/jpeg2000_bridge_retirement_plan.md`, including phased
  milestones, owner/checklist fields, weekly deliverables, risk controls, and
  time estimates for native decode parity and bridge decommissioning.

### Fixed
- Retired the vendored JPEG2000 decode bridge from active build wiring:
  `wbraster` now decodes through native `jpeg2000_core` only, the
  `jpeg2000-vendored-bridge` feature was removed from default/build paths, and
  `wbjpeg2000` was removed from workspace membership.
- Native `jpeg2000_core` packet walker now supports component-selective packet
  payload extraction for multicomponent LRCP traversal contexts via
  `extract_tile_data_for_component`, and includes regression tests that verify
  per-component packet-body filtering and out-of-range component guard
  behavior.
- Native `jpeg2000_core` single-layer decode now supports in-house
  multicomponent codestream payloads by decoding concatenated component
  code-block streams sequentially using consumed-byte tracking in the tier-1
  decoder. This replaces the immediate multicomponent `NotImplemented`
  fail-fast for that constrained class and adds a native writer/reader
  multiband roundtrip smoke test.
- `README.md` JPEG2000 notes now explicitly document the feature-gated vendored
  decode bridge (`wbjpeg2000` / `jpeg2000-vendored-bridge`) and acknowledge
  `dicom-toolkit-jpeg2000` lineage/licensing context, while clarifying
  long-term intent to retire the bridge as native `jpeg2000_core` reaches
  decode-parity coverage.
- GeoTIFF read-path sample-type mapping now accepts reduced-precision integer
  bit depths (`NBITS`) by promoting to the containing native integer type
  (e.g., unsigned 15-bit -> `u16`, signed 15-bit -> `i16`). This removes
  false "unsupported GeoTIFF sample format/bits combo" failures for common
  remote-sensing products stored with non-byte-aligned integer precision.
- JPEG2000 read-path now falls back to parsing GMLJP2 `xml ` boxes for CRS and
  affine geotransform when a GeoJP2 UUID payload is absent. This restores
  georeferencing for Sentinel-style JP2 scenes that previously loaded in pixel
  space (`EPSG=None`, unit-pixel extent), which could cause false
  "polygon extent does not overlap the input raster" failures in downstream
  clipping/overlay tools.
- JPEG2000 lossless decode now applies unsigned sample level-shift after
  inverse 5/3 wavelet reconstruction (image domain) rather than before IDWT
  (transform domain), preventing periodic lattice/checkerboard artifacts in
  decoded unsigned JP2 rasters.
- JPEG2000 read-path now uses a standards-compliant pure-Rust decoder for
  native sample decode (with internal decoder retained as fallback), while
  preserving existing GeoJP2/GMLJP2 georeferencing extraction. This removes
  severe decode corruption on some external JP2 products (e.g., periodic
  lattice/checkerboard artifacts on Sentinel-style scenes).
- The JP2 decode bridge is now feature-gated (`jpeg2000-vendored-bridge`) and
  backed by an in-repo vendored crate (`wbjpeg2000`) rather than a crates.io
  dependency, so decoding logic remains under Whitebox repository control while
  native `jpeg2000_core` improvements are developed.
- Native `jpeg2000_core` multicomponent decode path is now fail-fast disabled
  pending packet-level parser upgrades, removing a known silent-corruption
  risk from the legacy equal-split component extraction behavior.
- Native `jpeg2000_core` tile-part extraction now uses `SOT`/`Psot` bounded
  parsing to locate `SOD` payloads, replacing loose marker scanning and
  providing stricter codestream boundary validation as groundwork for packet-
  level parser upgrades.
- Native `jpeg2000_core` now validates tile-part sequence metadata
  (`TPsot`/`TNsot`) for per-tile ordering and completeness, rejecting malformed
  tile-part streams earlier in decode.
- Native `jpeg2000_core` now routes tile payload collection through an explicit
  packet-traversal plan and fail-fast rejects non-`LRCP` progression orders in
  the interim native packet walker, preventing silent decode attempts against
  unsupported progression layouts while packet-header parsing is still being
  ported.
- Native `jpeg2000_core` tile-part header parsing now inspects marker segments
  before `SOD` and fail-fast rejects unsupported progression/packet-header
  workflows (`POC`, `PLM`, `PLT`, `PPM`, `PPT`) in native packet traversal,
  preventing unsafe packet-body interpretation for codestreams that rely on
  not-yet-ported header-distribution logic.
- Native `jpeg2000_core` LRCP packet traversal now performs a bounded first
  packet-header preflight per tile-part (including optional `SOP`/`EPH`
  handling), with explicit fail-fast checks for empty post-`SOD` payloads,
  truncated first packet header bytes, and invalid `SOP` marker lengths.
- Native packet traversal planning now parses tile-part `COD` marker segments
  and uses tile-part coding-style overrides (progression order and layer
  count) when determining effective traversal behavior, avoiding incorrect
  reliance on main-header defaults when tile-part overrides are present.
- Native `jpeg2000_core` LRCP packet-header preflight now performs bounded
  post-flag bit parsing for non-zero packets and validates classic coding-pass
  codeword shape (Table B.4) for truncation/malformed-prefix conditions,
  fail-fast rejecting obviously incomplete packet-header bitstreams.
- Native `jpeg2000_core` non-zero packet-header preflight now also validates
  bounded `Lblock` unary increments and classic segment-length field bit-width
  requests, fail-fast rejecting unterminated unary increments and excessive/
  truncated segment-length bitfield reads before packet-body interpretation.
- Native LRCP packet-header preflight now validates first inclusion signaling
  bit availability and performs provisional packet-body span accounting,
  fail-fast rejecting non-zero packet headers that consume the entire tile-part
  payload without leaving packet body bytes.
- Native LRCP packet preflight now parses provisional classic segment-length
  values from packet-header bits and verifies declared segment lengths do not
  exceed remaining tile-part body bytes, adding explicit declared-span versus
  available-span consistency checks before packet-body interpretation.
- LRCP packet preflight ordering now checks first inclusion signaling before
  probing coding-pass/length fields for provisional contributions, reducing
  false rejects by only enforcing included-codeblock empty-body and declared-
  span checks when first-inclusion indicates contribution.
- LRCP packet preflight now performs a bounded multi-contribution preview
  (inclusion-gated classic coding-pass/length probes) and validates cumulative
  declared segment-length totals against remaining tile-part body bytes,
  tightening declared-span consistency checks beyond a single provisional
  contribution.
- Native LRCP tile-part extraction now consumes previewed packet body spans
  when provisional included contributions are detected (using preflight-derived
  body start and cumulative declared bytes), rather than always concatenating
  full tile-part payload windows in those cases.
- Native LRCP extraction now performs bounded position-aware packet-boundary
  walking within tile-parts, applying preview-derived body-span slicing across
  multiple packet starts (with conservative fallback when no previewed
  contributions are detected).
- LRCP packet-boundary preview iteration budget is now progression-aware,
  scaling with layer/component/resolution counts under bounded caps instead of
  a fixed packet-preview count.
- Native LRCP preview walking now advances with explicit packet-context
  scaffolding (`layer`, `resolution`, `component`) and annotates codestream
  preflight failures with that LRCP context for clearer diagnostics.
- Native LRCP packet preflight now tracks per-context `Lblock` state across
  packet walking and uses context-evolved `Lblock` values in classic segment-
  length parsing, improving packet-header state continuity across successive
  packet contexts.
- Native LRCP packet-context state now also tracks per-context packet and
  contribution history (`packets_seen`, `contributions_seen`, `ever_included`),
  with contextual diagnostics that include LRCP packet indices and state
  snapshots when packet-header preflight fails.
- Native LRCP packet-context history now records inclusion-transition metadata
  (`first_included_packet_index`, `last_included_packet_index`,
  `packets_since_last_inclusion`, `zero_length_packets`) and evolves it during
  packet preflight, providing stronger state continuity for future persistent
  inclusion/tag-tree migration work.
- Native LRCP tile-part payload assembly now buffers preview slices per
  tile-part and performs a conservative full tile-part fallback when a
  non-zero packet with no previewed contribution appears after previewed
  contributions have already been observed, avoiding unsafe packet-body span
  assumptions.
- The fallback path now retains a per-context inclusion-history hook, but
  remains tile-part conservative in current LRCP walker mode because packet
  contexts are traversed in a single pass (no context revisits yet).
- Native LRCP packet walker now performs bounded context-round revisits within
  each tile-part (instead of a strict single-pass), enabling repeated visits
  to the same `(layer,resolution,component)` context and preserving state
  continuity (e.g., `Lblock` and inclusion-history) across successive packet
  positions for that context.
- LRCP preview budget bounds were updated to allow revisit rounds while
  remaining bounded (`base_packet_context_budget * 2`, clamped to 128).
- Ambiguous non-zero/no-preview packets are now handled with context-aware
  policy during LRCP revisit walking: if the same context has prior inclusion
  history, conservative full tile-part fallback is triggered; otherwise, packet
  preview walking stops and already collected preview spans are preserved.
- Packet-header preflight now flags when bounded contribution preview reaches
  the contribution cap without observed inclusion termination, and extraction
  conservatively falls back to full tile-part payload in that ambiguous case to
  avoid partial packet-body span assumptions.
- Native packet traversal now supports constrained RLCP progression in addition
  to LRCP, including bounded context-round revisits and RLCP-context diagnostics
  around packet-header preflight failures.
- Tile-part COD progression overrides to RLCP are now honored by native packet
  traversal (instead of fail-fasting as unsupported progression).
- Packet-body accounting is now stricter for ambiguous new-context non-zero
  packets after previewed contributions: traversal preserves already collected
  preview slices and appends unresolved tail bytes from the ambiguity point
  rather than dropping remaining tile-part bytes.
- LRCP and constrained RLCP packet walkers now share a single progression-
  parameterized traversal core, reducing duplicated logic and keeping packet
  preflight/error-handling behavior consistent across both supported
  progression orders.
- Native packet traversal now supports all five JPEG2000 progression orders
  (`LRCP`, `RLCP`, `RPCL`, `PCRL`, `CPRL`) using the shared progression-aware
  traversal core.
- `RPCL`/`PCRL`/`CPRL` are currently implemented in constrained form (no
  precinct-position loop yet), with explicit cursor-order coverage and tile-part
  COD override tests.
- Packet-context/state continuity now carries across tile-parts for a tile,
  improving cross-part packet sequencing behavior for ambiguity/fallback logic.
- Writer-side parameter validation now enforces:
  - decomposition level upper bounds based on image dimensions,
  - color-space/component-count compatibility (`Greyscale`→1 component,
    `Srgb`/`YCbCr`→3 components).
- Writer defaults now auto-cap `decomp_levels` to image-supported limits,
  avoiding invalid default settings on small rasters.
- COD parsing now accepts standards-conformant 10-byte COD payloads (no
  legacy padding requirement), restoring writer/reader parse compatibility.
- Added active writer confidence tests for single-band encode/decode shape,
  multiband metadata roundtrip, decomposition bound rejection, and color-space
  compatibility rejection.
- Added writer confidence coverage for lossy mode encode/parse metadata behavior.
- Added writer geometadata roundtrip coverage (geo-transform, EPSG, NODATA).
- Writer now validates lossy quality hints (`quality_db` must be finite and > 0).
- Added a JPEG2000 differential corpus harness test (feature-gated under
  `jpeg2000-vendored-bridge`) that compares native decode vs bridge decode and
  classifies outcomes (`native_error`, `bridge_error`, `metadata_mismatch`,
  `sample_count_mismatch`, `sample_value_mismatch`, `ok`).
- Differential harness is controlled by env vars:
  - `JPEG2000_DIFF_FIXTURES`: fixture path list (`;`, `,`, or newline-separated),
  - `JPEG2000_DIFF_EPS`: absolute sample tolerance,
  - `JPEG2000_DIFF_ENFORCE=1`: fail test on any mismatch/error.
- Differential harness now supports machine-readable JSON reporting via
  `JPEG2000_DIFF_REPORT=<path>`.
- Differential harness now supports fixture-file input via
  `JPEG2000_DIFF_FIXTURE_FILE=<path>` (one JP2 path per line; `#` comments supported).
- Differential harness now supports threshold-based enforcement controls:
  - `JPEG2000_DIFF_MAX_NATIVE_ERROR`
  - `JPEG2000_DIFF_MAX_BRIDGE_ERROR`
  - `JPEG2000_DIFF_MAX_METADATA_MISMATCH`
  - `JPEG2000_DIFF_MAX_SAMPLE_COUNT_MISMATCH`
  - `JPEG2000_DIFF_MAX_SAMPLE_VALUE_MISMATCH`
  - `JPEG2000_DIFF_MIN_OK`
- Differential JSON summary now includes native blocker subtype
  `native_unsupported_packet_header_markers` for codestreams requiring
  `PPM`/`PPT` packet-header marker workflows (updated from the broader
  `PLM/PLT/PPM/PPT` label now that `PLT`/`PLM` are handled).
- Differential report writer now creates parent directories for
  `JPEG2000_DIFF_REPORT` output paths before writing JSON artifacts.
- `pixel_type()` now returns `Uint16` for 16-bit signed components (SIZ `Ssiz`
  signed flag), matching the `wbjpeg2000` bridge behaviour which ignores the
  signed flag entirely. Previously such components were mapped to `Int16`,
  causing a metadata mismatch in the differential corpus harness. The 32-bit
  signed case (`(true, 32) → Int32`) is still respected as a separate match.
- Main-header `POC` (Progression Order Change) marker is now detected during
  codestream construction and stored in `GeoJp2::has_main_header_poc`. If
  tile-part-level OR main-header-level POC is present, `collect_tile_packet_payload`
  returns a `NotImplemented` error immediately rather than silently producing
  incorrect packet sequences. A new `native_unsupported_poc` subtype counter
  is tracked in the differential corpus harness JSON report.
- Sentinel-2 Chunk F corpus run result: `metadata_mismatch=0` (was 2),
  `native_error=0`, `sample_value_mismatch=3`. The remaining 3 mismatches
  are due to the single-block MQ arithmetic decode approximation used by the
  native reader's `decode_block` path, which does not implement per-code-block
  segment parsing from packet headers. This is a Chunk G work item.
  marker segments are now silently skipped during native tile-part header
  scanning instead of triggering a `NotImplemented` rejection. These markers
  are length-hint optimizations only and do not alter the packet-body layout;
  skipping them is consistent with the reference `wbjpeg2000` implementation.
  `PPT` and `PPM` (which DO externalise packet headers from the body) continue
  to be rejected with a descriptive error until structural support is ported.
- Stale `multiband_failfast` unit tests updated to reflect current walker
  behaviour after Chunk B/C progression support additions:
  - `extract_tile_data_rejects_non_lrcp_progression_until_packet_port_is_ready`
    renamed and now verifies RLCP no longer returns a rejection error.
  - `extract_tile_data_rejects_empty_payload_after_sod` assertion broadened to
    also accept the `Tile N not found` catch-all error path.
  - `extract_tile_data_rejects_truncated_classic_coding_pass_codeword_preflight`
    assertion updated to match evolved preflight message text.
  - `extract_tile_data_rejects_unterminated_lblock_increment_preflight`
    converted to no-panic smoke check (walker now handles this byte pattern).
- Integration test `Jpeg2000WriteOptions` updated to use `decomp_levels: None`
  (auto-cap) for the 6×4 test raster, which supports at most 2 decomp levels
  after the Chunk C writer invariant checks were added.
- Sentinel-2 corpus baseline run updated: `native_error` reduced from 3 → 0
  after the PLT skip fix. Remaining issues recorded: data-type mismatch (native
  reports `I16` per SIZ signed flag; bridge ignores signed flag and reports
  `U16`) and sample-value differences indicating DC level-shift handling
  divergence between native and bridge paths.

## [0.1.2] - 2026-04-14

### Fixed
- Corrected GeoTIFF/COG write-path handling to use the common chunky conversion flow for single-band and multi-band rasters, preventing inconsistent layout behavior across data types.

### Changed
- Updated Zarr documentation terminology to remove stale "MVP" labels and
  reflect current v2/v3 local-store support more accurately.

## [0.1.1] - 2026-04-11

### Added
- Expanded Sentinel SAFE package support:
  - Sentinel-1: acquisition datetime parsing, spatial bounds parsing, canonical subswath-safe keying, calibration LUT parsing/interpolation, thermal noise LUT parsing/interpolation, calibrated and noise-corrected read helpers, dB-output helpers.
  - Sentinel-1: orbit-state vector parsing, geolocation-grid parsing/interpolation, SLC burst-list parsing, and multi-polarization batch read helpers.
  - Sentinel-2: QA key listing, L2A auxiliary-layer indexing (`AOT`, `WVP`, `TCI`), cloud-coverage metadata, and processing-baseline metadata.
- Added SAFE integration coverage for `open_safe_bundle` mission variants.
- Added new package readers for common non-SAFE remote-sensing bundles:
  - `LandsatBundle` (MTL + GeoTIFF assets)
  - `IceyeBundle` (XML + COG/GeoTIFF assets)
  - `PlanetScopeBundle` (JSON/XML + GeoTIFF assets)
  - `DimapBundle` (DIMAP XML + JP2/GeoTIFF assets for SPOT/Pleiades deliveries)
  - `MaxarWorldViewBundle` (`.IMD`/XML + JP2/GeoTIFF assets)
  - `Radarsat2Bundle` (product XML + GeoTIFF assets)
  - `RcmBundle` (XML + GeoTIFF assets)
- Added unified multi-family bundle detection/opening APIs:
  - `detect_sensor_bundle_family`
  - `open_sensor_bundle`
- Added archive-aware unified APIs supporting `.zip`, `.tar`, `.tar.gz`, and `.tgz` bundle paths:
  - `detect_sensor_bundle_family_path`
  - `open_sensor_bundle_path`
  - `OpenedSensorBundle` (exposes temporary extraction root for optional cleanup)
- Extended RADARSAT-2 and RCM bundle metadata extraction beyond MVP with:
  - orbit direction
  - look direction
  - near/far incidence angles
  - range/azimuth pixel spacing
- Extended ICEYE bundle metadata extraction beyond MVP with:
  - acquisition mode
  - orbit direction
  - look direction
  - near/far incidence angles
  - range/azimuth pixel spacing
- Added polarization-focused convenience read APIs:
  - `IceyeBundle::list_polarizations`
  - `IceyeBundle::read_assets_for_polarization`
  - `Radarsat2Bundle::read_measurements_for_polarization`
  - `RcmBundle::read_measurements_for_polarization`
- Improved channel key normalization across ICEYE, RADARSAT-2, and RCM readers to robustly detect polarization tokens across varied filename separators (`_`, `-`, `.`, and mixed patterns).
- Added compatibility-oriented filename variant tests for ICEYE, RADARSAT-2, and RCM key extraction.
- Added Landsat Collection compatibility hardening for common L2 SR/ST/QA variants, including `SR_CLOUD_QA`, `SR_ATMOS_OPACITY`, and `ST_*` auxiliary thermal products.
- Expanded Sentinel-2 L2A QA mask classification to include `MSK_CLDPRB`, `MSK_SNWPRB`, `MSK_CLASSI`, `MSK_DETFOO`, and `MSK_QUALIT` layers.
- Added opt-in real-sample smoke tests for Landsat and Sentinel-2 SAFE package openers (`WBRASTER_LANDSAT_SAMPLE`, `WBRASTER_S2_SAFE_SAMPLE`).
- Hardened ICEYE asset indexing to avoid silent overwrite when multiple same-polarization assets are present in one bundle.
- Improved ICEYE numeric metadata parsing to tolerate values with unit suffixes (e.g., `2.5 m`, `20.0 deg`).
- Added opt-in real-sample smoke test for ICEYE package opener (`WBRASTER_ICEYE_SAMPLE`).
- Hardened RADARSAT-2 and RCM measurement indexing to avoid silent overwrite when multiple same-polarization assets are present in one bundle.
- Improved RADARSAT-2 and RCM numeric metadata parsing to tolerate values with unit suffixes (e.g., `8.0 m`, `24.5 deg`).
- Added opt-in real-sample smoke tests for RADARSAT-2 and RCM package openers (`WBRASTER_RADARSAT2_SAMPLE`, `WBRASTER_RCM_SAMPLE`).
- Added RADARSAT-2 and RCM polarization fallback inference from measurement filenames when metadata polarization tags are missing or incomplete.
- Added ICEYE metadata JSON sidecar fallback parsing for core fields (product type, acquisition time, mode, polarization, orbit/look direction, incidence angles, and pixel spacing) when XML metadata is absent or partial.
- Added an explicit ICEYE Open Data opt-in smoke test path (`WBRASTER_ICEYE_OPEN_DATA_SAMPLE`) for validating public scene directories built from the open STAC catalog.
- Added opt-in real-sample smoke tests for the newly added PlanetScope, DIMAP, and Maxar/WorldView bundle readers (`WBRASTER_PLANETSCOPE_SAMPLE`, `WBRASTER_DIMAP_SAMPLE`, `WBRASTER_MAXAR_SAMPLE`).
- Hardened canonical band-key mapping for PlanetScope, DIMAP, and Maxar/WorldView bundles with token-aware filename parsing and expanded aliases (including SuperDove 8-band, DIMAP `XS*`/`SWIR*`, and WorldView multispectral variants).
- Expanded PlanetScope, DIMAP, and Maxar/WorldView metadata normalization with additional commonly used fields (acquisition datetime, cloud cover, sun geometry, and view/off-nadir angles where available in source metadata).
- Added profile-aware asset grouping and profile-targeted convenience APIs for PlanetScope, DIMAP, and Maxar/WorldView bundle readers (`list_profiles`, `default_profile`, `list_band_keys_for_profile`, `band_path_for_profile`, and `read_band_for_profile`).
- Extended PlanetScope, DIMAP, and Maxar/WorldView real-sample smoke tests with optional profile-conformance assertions via env vars (`*_EXPECT_PROFILES`).
- Extended Landsat, Sentinel-2 SAFE, ICEYE, ICEYE Open Data, RADARSAT-2, and RCM real-sample smoke tests with optional canonical-key conformance assertions via env vars (`*_EXPECT_KEYS`).
- Implemented Zarr v3 `transpose` codec support (array-to-array codec in the codec pipeline):
  - Supports `order: "C"` (identity, no-op), `order: "F"` (full axis reversal), and explicit integer permutation arrays.
  - Correctly handles both interior (full-size) chunks and boundary chunks for producers that pad boundary chunks to the full chunk shape.
  - Gracefully handles unpadded boundary chunks from non-spec-compliant producers.
  - Validated by four new integration tests: single-chunk F-order, explicit `[1,0]` permutation, C-order no-op, and multi-chunk with boundary tiles.
- Hardened Zarr v3 validation/diagnostics by:
  - rejecting unknown/unsupported codecs in the v3 codec pipeline with actionable error messages,
  - rejecting zero-dimension array shapes,
  - rejecting mismatched array-rank vs chunk-shape-rank metadata.
- Added Zarr v2 writer chunk-row/chunk-col controls via metadata keys (`zarr_chunk_rows`, `zarr_chunk_cols`) while keeping backward-compatible defaults.
- Added in-place parallel fill APIs:
  - `RasterData::par_fill_with(Fn(usize) -> f64)`
  - `Raster::par_fill_with(Fn(usize) -> f64)`
- Added typed in-place parallel write path that dispatches to native storage and avoids intermediate allocation buffers.
- Added `Raster::new_like(&Raster)` to construct metadata-equivalent output rasters without cloning source data buffers.
- Updated memory-store internals to Arc-backed raster storage while preserving existing `Raster`-returning APIs.
- Added shared-handle memory-store APIs:
  - `put_raster_arc(Arc<Raster>)`
  - `get_raster_arc_by_id(&str)`
  - `get_raster_arc_by_path(&str)`

### Changed
- GeoTIFF reads now apply metadata-defined linear value transforms
  (`value = raw * scale + offset`) when present, and normalize known vertical
  GeoKey units (international foot / US survey foot) to meters in-memory.
  Transformed GeoTIFF reads are materialized as `f64` raster data.
- Updated README with SAFE and non-SAFE bundle examples, including unified detection/opening workflows.
- Configured `zip` dependency for pure-Rust operation (`default-features = false`, `features = ["deflate"]`) to avoid `bzip2-sys`/`lzma-sys` native dependencies from default feature sets.
- Stabilized `memory_store` unit tests by serializing tests that mutate the shared global in-memory raster store.
- Raster write throughput improves for GeoTIFF/COG outputs through wbgeotiff's
  parallel strip/tile chunk encoding path.

## [0.1.0] - 2026-03-31
### Added
- Initial published release.
