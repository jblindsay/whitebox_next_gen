# WbW-Py Interoperability Behavior Matrix (Phase 1)

Purpose:
- Capture practical interoperability behavior for first-pass documentation.
- Provide a stable source for future user-manual generation.
- Track where WbW-R should mirror documentation structure.

## Matrix

| Bridge | Entry points | What is preserved | What can drift | Copy/view notes | Verification checklist |
|---|---|---|---|---|---|
| NumPy | `Raster.to_numpy()`, `Raster.from_numpy(array, base, ...)` | Numeric cell values; base raster geospatial context on import | dtype conversion effects, nodata representation if explicitly cast | Materialized array transfer at boundary | Verify rows/cols, nodata, min/max, selected sample values |
| Rasterio | `write_raster` + Rasterio read/write + `read_raster` | CRS, transform, dimensions via raster file metadata | Driver/profile changes if Rasterio profile updated | File boundary (copy) | Verify CRS EPSG/WKT, shape, nodata, dtype |
| GeoPandas | `write_vector` + `gpd.read_file` + `read_vector` | Geometries, CRS, attributes (driver-dependent) | Field typing/width nuances by driver | File boundary (copy) | Verify feature count, field names/types, CRS, sample attributes |
| Shapely | Usually through GeoPandas geometry ops | Geometry results produced by Shapely ops | Topology precision/simplification effects from chosen operations | In-memory geometry object semantics | Validate geometry validity and expected relation checks |
| xarray/rioxarray | `write_raster` + `rxr.open_rasterio` + `.rio.to_raster` + `read_raster` | CRS/transform and raster grid metadata through rioxarray | Dask/chunk/lazy-eval pipeline choices | File boundary for wbw import/export | Verify CRS, transform, dimensions, nodata, sample cell deltas |
| pyproj | `metadata().crs_epsg()`, `pyproj.CRS`, pyproj transformers | CRS interpretation and transform definitions | EPSG resolution for unusual CRS strings | No payload copy unless combined with file/array exchange | Verify EPSG identification and representative coordinate transforms |

## Copy-vs-view guidance

- Treat all ecosystem boundaries as copy boundaries unless explicitly documented otherwise.
- Prefer explicit dtype handling (`to_numpy(dtype=...)`) when downstream tools are strict.
- Re-validate metadata after round trips with `metadata()` before chaining analysis tools.

## Recommended stable exchange formats

- Raster: GeoTIFF (`.tif`) for broad tool compatibility.
- Vector: GeoPackage (`.gpkg`) for robust schema + CRS round-trips.
- Lidar: use native wbw I/O unless external pipeline requirements demand format conversion.

## WbW-R Parallelization Notes

- Status: `parallel now` for documentation structure and matrix concept.
- Proposed WbW-R follow-up:
  - Add an R-facing interoperability matrix with equivalent bridge categories.
  - Mirror copy-boundary language and verification checklists.
  - Keep language-specific examples but align preservation/drift semantics.

## Follow-up tasks

- Add round-trip smoke tests matching the verification checklist above.
- Add explicit examples for metadata drift handling (dtype changes, field width limits).
- Link this matrix from any generated user-manual pipeline source index.
