# Sensor Bundle Next-Candidate Roadmap (Internal)

Last updated: 2026-04-01 (revised 2026-04-01: Tier A complete)

## Purpose

This note ranks likely next sensor-bundle package readers for `wbraster` based on:

1. High remote-sensing community usage.
2. Fit with current `wbraster` I/O strengths (GeoTIFF/COG, JP2/GeoJP2, XML/JSON sidecars).
3. Fit with project constraints (pure Rust, minimal dependencies in core crates).
4. Likelihood of obtaining public or redistributable test fixtures.

Current package-level baseline already implemented:

- Sentinel-1 SAFE
- Sentinel-2 SAFE
- Landsat Collection bundles
- ICEYE bundles (standard + Open Data)
- RADARSAT-2 bundles
- RCM bundles
- PlanetScope scene bundles ✅ (Tier A — complete)
- SPOT/Pleiades DIMAP bundles ✅ (Tier A — complete)
- Maxar/WorldView bundles ✅ (Tier A — complete)

---

## Short Answer On Sentinel-3

Yes, your concern is valid.

Many Sentinel-3 products are distributed as SAFE directories containing NetCDF-4 files. NetCDF-4 typically relies on HDF5 internals. Supporting this well in a pure-Rust + minimal-dependency stack is materially harder than current bundle readers that mostly orchestrate TIFF/JP2 assets.

Practical implication:

- Sentinel-3 SAFE is still high-value scientifically, but should not be the immediate next addition unless we first make an explicit decision on NetCDF/HDF5 strategy.
- If pursued early, scope should be metadata/indexing-first (detect package, parse top-level XML/manifests), while deferring pixel read support for NetCDF assets.

---

## Ranked Next Additions

### Tier A — all complete ✅

1. PlanetScope scene bundles ✅
2. SPOT/Pleiades DIMAP ✅
3. Maxar/WorldView bundle families ✅

All Tier A readers are implemented with unit tests, opt-in real-sample smoke tests,
and conformance env vars (`WBRASTER_PLANETSCOPE_EXPECT_PROFILES`, etc.).

### Tier B — current frontier (high value, medium complexity)

4. Sentinel-3 SAFE (metadata/indexing phase)
5. TerraSAR-X / TanDEM-X package deliveries
6. ALOS PALSAR CEOS-style packages

Why Tier B:

- Valuable missions but with greater packaging variance or format friction.
- Sentinel-3 specifically intersects NetCDF-4/HDF5 constraints (see strategy section below).

### Tier C — defer until strategy/fixture access

7. MODIS/VIIRS package products (HDF/netCDF-heavy)
8. OPERA/HLS-style multi-file analysis bundles
9. Capella/Umbra commercial SAR bundles

Why Tier C:

- Either stronger dependency pressure (HDF/netCDF) or less reliable public fixture accessibility.

---

## Recommended Implementation Sequence

1. Sentinel-3 SAFE metadata/indexing reader (no NetCDF pixel decode yet)
2. TerraSAR-X / TanDEM-X reader
3. ALOS PALSAR CEOS-style reader
4. Full NetCDF pixel-read support (gated on explicit HDF5/NetCDF strategy decision)

This sequence maximizes user-visible value without forcing an early NetCDF/HDF5 architecture decision.

---

## Detailed Checklists For Tier B Candidates

## 1) Sentinel-3 SAFE — Metadata/Index Phase

### Typical package ingredients

- SAFE directory with top-level `xfdumanifest.xml`.
- `measurement/` subdirectory containing NetCDF-4 (`.nc`) assets.
- `granules/` and `indices/` subdirectories with supporting metadata.

### Proposed public API shape (Phase 1, metadata/index only)

- `Sentinel3Bundle`
- Fields:
  - `bundle_root`
  - `product_type` (e.g., `OL_1_EFR`, `SL_2_LST`, etc.)
  - `acquisition_start_utc`, `acquisition_stop_utc`
  - `mission` (`Sentinel3A`, `Sentinel3B`)
  - `assets: BTreeMap<String, PathBuf>` — opaque NetCDF asset entries

### Phase 1 methods

- `open(bundle_root)`
- `list_asset_keys() -> Vec<String>`
- `asset_path(key) -> Option<&Path>`
- `read_*` methods: return `Err(NotSupported)` with clear message until NetCDF pixel-read is implemented

### Detection heuristics

- SAFE directory with `xfdumanifest.xml` declaring `Sentinel-3` family.
- Product type prefix matching (`OL_`, `SL_`, `SY_`, `SR_`, etc.).

### Tests

- Synthetic metadata parsing (mock `xfdumanifest.xml`).
- Confirm `read_*` returns `Err` with useful message rather than panicking.

### Optional smoke test env var

- `WBRASTER_SENTINEL3_SAMPLE`

---

## Sentinel-3 Strategy Options (explicit)

If Sentinel-3 is prioritized soon, pick one of these before implementation:

1. Metadata/index-only phase (recommended first)
- Detect Sentinel-3 SAFE family.
- Parse mission/product metadata from XML manifests.
- Index NetCDF assets as opaque entries.
- Return clear `NotImplemented` for NetCDF pixel reads.

2. Full pixel-read phase
- Requires explicit NetCDF/HDF5 dependency strategy compatible with project policy.
- Should be isolated behind a crate feature gate if adopted.

Recommended now: Option 1 only, after Tier A readers.

---

## Acceptance Criteria For This Roadmap

We should treat Tier B work as the active frontier. This roadmap is considered current until:

1. Sentinel-3 SAFE metadata/indexing phase is implemented and tested.
2. At least one further Tier B reader (TerraSAR-X or ALOS PALSAR) is complete.
3. At that point, revise Tier B/C ranking and update this document again.
2. Unified sensor-bundle detection includes these families.
3. Sentinel-3 decision is documented as metadata-only phase or dependency-backed full read phase.
4. No core crate dependency policy violations are introduced.
