# WbW-R Interoperability Behavior Matrix

Purpose:
- Capture practical R interoperability behavior for first-pass user documentation.
- Provide a stable source for future manual-generation or README sync workflows.
- Mirror the WbW-Py matrix structure while keeping R-specific bridge choices explicit.

## Matrix

| Bridge | Entry points | What is preserved | What can drift | Copy/view notes | Verification checklist |
|---|---|---|---|---|---|
| Base array / `terra` array bridge | `wbw_raster$to_array()`, `wbw_raster_to_array()`, `wbw_array_to_raster(x, template_path=...)` | Numeric cell values; template raster geospatial context on import | Type coercion, `NA`/nodata representation, dropped band labels if arrays are manually reshaped | Materialized array copy at the boundary | Verify dimensions, nodata, selected sample values, and min/max ranges |
| `stars` | `wbw_raster$to_stars()`, `wbw_raster_to_stars()`, `wbw_stars_to_raster()` | CRS, transform, extent, and grid structure through `stars` metadata | Lazy/proxy evaluation choices, attribute naming, value type changes after downstream transforms | File-backed or proxy-backed boundary depending on how `stars` is loaded | Verify CRS, dimensions, nodata, and a few representative cell values after round-trip |
| `terra` vector | `wbw_vector$to_terra()` plus `wbw_write_vector()` / `wbw_read_vector()` for persistence | Geometry, CRS, and attributes as supported by the source/target driver | Field type coercion, width/precision limits, factor-like handling by downstream packages | Read into an in-memory `SpatVector`; persistence is an explicit file boundary | Verify feature count, CRS, field names/types, and sample attribute values |
| `sf` | `wbw_vector$to_sf()` plus `sf::st_write()` / `wbw_read_vector()` when round-tripping | Geometry, CRS, and attributes as supported by the chosen driver | Driver-specific schema coercion, string width limits, geometry casting or simplification from downstream edits | In-memory `sf` object copy; round-trip persists through a file boundary | Verify feature count, CRS, geometry type, and sample attribute values |
| Stable file exchange | `wbw_raster$write()`, `wbw_write_raster()`, `wbw_vector$write()`, `wbw_write_vector()`, then re-read with `wbw_read_*()` | Backend-managed metadata and data layout for supported formats | Format-specific constraints such as GeoTIFF dtype limits or Shapefile field naming/width rules | Explicit file copy boundary by design | Prefer `.tif` for raster and `.gpkg` for vector; verify metadata immediately after re-ingest |

## Copy-vs-view guidance

- Treat all ecosystem boundaries as copy boundaries unless proxy/lazy behavior is explicitly documented.
- `wbw_raster$to_stars(proxy = TRUE)` can defer cell reads, but metadata should still be validated before chaining analysis.
- Prefer stable exchange containers when lossless round-trip matters: `.tif` for raster and `.gpkg` for vector.
- Re-check `metadata()`, `schema()`, or representative values after round-trips before continuing analysis.
- LiDAR currently has native wbw read/write workflows but no first-class R ecosystem bridge at the same maturity level as raster/vector.

## Recommended stable exchange formats

- Raster: GeoTIFF (`.tif`) for broad compatibility and predictable metadata round-trips.
- Vector: GeoPackage (`.gpkg`) for robust schema and CRS round-trips.
- LiDAR: prefer native wbw read/write workflows until a clearer R ecosystem bridge contract is defined.

## Relationship to WbW-Py

- Status: `parallel now` for structure and terminology alignment.
- Intention: keep bridge categories, preservation language, and verification checklist style aligned across languages.
- Allowed divergence: R-specific bridge entry points (`terra`, `stars`, `sf`) replace Python-specific libraries where appropriate.

## Follow-up tasks

- Add round-trip smoke tests that exercise the verification checklist for raster and vector bridges.
- Add explicit examples of metadata drift handling for field width/type coercion and raster dtype conversion.
- Extend this matrix when LiDAR or sensor-bundle interoperability with external R tooling becomes first-class.
