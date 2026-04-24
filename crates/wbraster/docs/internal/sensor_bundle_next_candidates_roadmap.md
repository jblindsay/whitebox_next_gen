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

Recent outcome:

- Experimental HDF5/NetCDF bundle readers were removed after real-file
  validation showed the pure-Rust HDF5 approach was not viable enough for
  production support.
- Sentinel-3, MODIS, Sentinel-5P, SMAP, and VIIRS are not currently
  implemented bundle families in `wbraster`.

---

## Short Answer On HDF5/NetCDF Families

Many Sentinel-3 products are distributed as SAFE directories containing NetCDF-4 files. NetCDF-4 typically relies on HDF5 internals. Supporting this well in a pure-Rust + minimal-dependency stack is materially harder than current bundle readers that mostly orchestrate TIFF/JP2 assets.

Practical implication:

- HDF5/NetCDF-backed bundle families are deferred until there is an explicit,
  credible dependency strategy.
- Do not reintroduce Sentinel-3, MODIS, Sentinel-5P, SMAP, or VIIRS bundle
  readers under the current pure-Rust-only HDF5 assumption.

---

## Ranked Next Additions

### Tier A — all complete ✅

1. PlanetScope scene bundles ✅
2. SPOT/Pleiades DIMAP ✅
3. Maxar/WorldView bundle families ✅

All Tier A readers are implemented with unit tests, opt-in real-sample smoke tests,
and conformance env vars (`WBRASTER_PLANETSCOPE_EXPECT_PROFILES`, etc.).

### Tier B — current frontier (high value, medium complexity)

4. TerraSAR-X / TanDEM-X package deliveries
5. ALOS PALSAR CEOS-style packages
6. OPERA/HLS-style multi-file analysis bundles

Why Tier B:

- Valuable missions but with greater packaging variance or metadata friction.

### Tier C — defer until dependency strategy changes

7. Sentinel-3 SAFE / SEN3 products
8. MODIS, Sentinel-5P, SMAP, and VIIRS package products
9. Capella/Umbra commercial SAR bundles

Why Tier C:

- Either strong HDF/netCDF dependency pressure or less reliable public fixture accessibility.

---

## Recommended Implementation Sequence

1. TerraSAR-X / TanDEM-X reader
2. ALOS PALSAR CEOS-style reader
3. OPERA/HLS-style bundle reader
4. Revisit HDF5/NetCDF package support only after adopting a non-experimental strategy

This sequence maximizes user-visible value without reopening the HDF5/NetCDF dependency problem.

---

## HDF5/NetCDF Re-entry Criteria

Do not restart work on these families until all of the following are true:

1. A viable dependency strategy has been chosen and validated against real files.
2. The strategy supports NOAA/NASA-style HDF5 edge cases seen in production samples.
3. The policy impact on core crate dependencies is explicitly accepted.
4. Real-file smoke fixtures exist for each target family before implementation begins.

---

## Acceptance Criteria For This Roadmap

We should treat non-HDF Tier B work as the active frontier. This roadmap is considered current until:

1. At least one further Tier B reader (TerraSAR-X, ALOS PALSAR, or OPERA/HLS) is complete.
2. The Tier B/C ranking is revised after that delivery.
3. Any future HDF5/NetCDF proposal includes a validated dependency decision up front.
4. No core crate dependency policy violations are introduced.
