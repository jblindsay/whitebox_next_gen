# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

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
