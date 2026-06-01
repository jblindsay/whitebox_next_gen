# HDF LiDAR Integration Plan (GEDI + ICESat-2)

## Intent

Define a stable, incremental integration path for HDF LiDAR products in `wblidar` using
`wbhdf` as the low-level decode backend.

Current validated Tier 1 scope:
- GEDI L2B canopy-style path (`/BEAM0000/elev_lowestmode`)
- ICESat-2 ATL08 canopy-style path (`/gt*/land_segments/canopy/h_canopy`)

## Architecture Decision

Yes, the integration should be **similar in design spirit** to the `wbraster` sensor-bundle
pattern, but not identical in shape.

Shared pattern with `wbraster` sensor bundles:
- family detection API
- provider trait abstraction
- ordered provider registry with defaults
- canonical resolved product descriptor
- unified dispatch entrypoint that hides family-specific details

Key difference from `wbraster` bundles:
- `wbraster` sensor bundles detect/open **directory package roots**.
- HDF LiDAR in `wblidar` detects/opens **single HDF files + dataset paths**.

So this is best modeled as a "product provider registry" rather than a directory-bundle opener.

## Layering

1. `wbhdf` (decode/runtime layer)
- container parsing
- dataset-path resolution
- chunk/layout/filter decode
- diagnostics and error taxonomy

2. `wblidar::hdf_adapter` (family-specific read helpers)
- concrete, bounded reads for validated Tier 1 paths
- product-specific path and offset/object-header assumptions

3. `wblidar::hdf_products` (family detection + dispatch)
- provider trait + registry
- canonical product-family resolution
- unified read dispatch for canopy windows

## Current State

Implemented in `wblidar`:
- `hdf_adapter` concrete Tier 1 helpers for GEDI and ATL08
- `hdf_products` registry-style dispatch API for product-family detection and unified reads

Implemented in `wbhdf`:
- HDF5 + HDF4 scoped parsing and bounded decode support
- structured decode errors (`DatatypeMismatch`, `InvalidChunk`, `FilterFailure`)

## Next Milestones

1. Generalize ATL08 object-header/index discovery
- Completed (initial): removed fixed object-header offset dependency by introducing bounded
	dynamic v1 object-header discovery + ranking in `wblidar::hdf_adapter`.
- Completed (refinement): ranking now incorporates resolved beam-path marker affinity.
- Follow-up: add explicit beam-group linkage extraction (name/object mapping) when available.

2. Add additional Tier 1 providers
- ATL03 canopy/height path (or additional ATL08 beams)
- GEDI alternative beams/variables as provider variants

3. Add runtime diagnostics counters
- Completed (initial): added unified dispatch counters through
	`HdfLidarReadDiagnostics` + `read_hdf_lidar_canopy_f32_window_with_diagnostics(...)`
	for chunk traversal/decode and key failure classes.
- Follow-up: surface diagnostics from deeper `wbhdf` decode internals (for example,
	candidate headers scanned and per-filter-attempt counters).

4. Add bounded-memory read safeguards
- Completed (initial): ATL08 canopy chunk read path now enforces explicit compressed and
	decompressed chunk-size caps with deterministic limit-exceeded diagnostics.
- Follow-up: align cap strategy across all future HDF LiDAR providers (ATL03, additional GEDI).

5. Stabilize canonical product descriptors
- include units, expected nodata semantics, and confidence annotations
- ensure bindings can consume descriptor metadata without product-specific code

## Why this is the right fit

This approach keeps `wblidar` API ergonomics close to `wbraster`'s provider/registry model
while preserving the real HDF constraint that the dispatch target is file+dataset, not a package
directory.
