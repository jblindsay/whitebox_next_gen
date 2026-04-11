# Whitebox Workflows for Python

Whitebox Workflows for Python is the Python interface for the Whitebox backend runtime.

The API is in active modernization, with emphasis on:
- clearer data-object ergonomics,
- better discoverability and IntelliSense,
- memory-first workflows,
- stronger interoperability with Python data tooling.

## Table of contents

- [Current API highlights](#current-api-highlights)
- [Migration quick map](#migration-quick-map)
- [Tool reference docs](#tool-reference-docs)
- [Development install](#development-install)
- [Quick smoke test](#quick-smoke-test)
- [Recommended examples](#recommended-examples)
- [Recommended API pattern](#recommended-api-pattern)
- [Quick start examples by data type](#quick-start-examples-by-data-type)
- [Raster output controls](#raster-output-controls)
- [Vector output controls](#vector-output-controls)
- [Lidar output controls](#lidar-output-controls)
- [Memory-first execution model](#memory-first-execution-model)
- [Reprojection patterns](#reprojection-patterns)
- [NumPy interoperability](#numpy-interoperability)
- [Rasterio interoperability](#rasterio-interoperability)
- [GeoPandas interoperability](#geopandas-interoperability)
- [Shapely interoperability](#shapely-interoperability)
- [xarray/rioxarray interoperability](#xarrayrioxarray-interoperability)
- [pyproj interoperability](#pyproj-interoperability)
- [Supported file formats](#supported-file-formats)
- [Licensing overview](#licensing-overview)
- [Licensing and Pro workflows](#licensing-and-pro-workflows)
- [Discovery APIs](#discovery-apis)
- [IntelliSense in VS Code](#intellisense-in-vs-code)

## Current API highlights

- Harmonized metadata access:
  - `Raster.metadata()`
  - `Vector.metadata()`
  - `Lidar.metadata()`
- Backward compatibility remains for `Raster.configs()`.
- Vector attribute readability aliases:
  - `schema()`, `attributes()`, `attribute()`
  - `update_attributes()`, `update_attribute()`, `add_field()`
- Dataset-aware vector write/copy for multifile formats.
- Raster and NumPy bridge:
  - `Raster.to_numpy(...)`
  - `Raster.from_numpy(...)`

## Migration quick map

Common updates from legacy style to the harmonized API:

| Legacy style | Current style |
|---|---|
| `r.configs()` | `r.metadata()` |
| `v.attribute_fields()` | `v.schema()` |
| `v.get_attributes(i)` | `v.attributes(i)` |
| `v.get_attribute(i, field)` | `v.attribute(i, field)` |
| `v.set_attributes(i, values)` | `v.update_attributes(i, values)` |
| `v.set_attribute(i, field, value)` | `v.update_attribute(i, field, value)` |
| `v.add_attribute_field(...)` | `v.add_field(...)` |

Notes:
- Legacy method names remain available for compatibility.
- New aliases are intended to improve readability and consistency across object types.

## Tool reference docs

- [TOOLS.md](TOOLS.md)
- [docs/tools_hydrology.md](docs/tools_hydrology.md)
- [docs/tools_gis.md](docs/tools_gis.md)
- [docs/tools_remote_sensing.md](docs/tools_remote_sensing.md)
- [docs/tools_geomorphometry.md](docs/tools_geomorphometry.md)
- [docs/tools_agriculture.md](docs/tools_agriculture.md)
- [docs/tools_lidar_processing.md](docs/tools_lidar_processing.md)
- [docs/tools_stream_network_analysis.md](docs/tools_stream_network_analysis.md)

Design and migration notes:
- [docs/v2_migration_guide.md](docs/v2_migration_guide.md)
- [docs/data_object_api_harmonization_proposal.md](docs/data_object_api_harmonization_proposal.md)
- [docs/api_redesign_rfc_draft.md](docs/api_redesign_rfc_draft.md)

## Development install

From workspace root:

```bash
./scripts/dev_python_install.sh
```

To build the Python extension with Pro support compiled in:

```bash
./scripts/dev_python_install.sh --pro
```

You can also enable the same behavior with an environment variable:

```bash
WBW_PYTHON_ENABLE_PRO=1 ./scripts/dev_python_install.sh
```

This performs an editable install via maturin for the wbw_python crate.

## Quick smoke test

```bash
python3 crates/wbw_python/examples/python_import_smoke_test.py
```

## Recommended examples

**Suggested run order for new users:**

| Order | Script | Focus |
|---|---|---|
| 1 | [examples/quickstart_harmonized_api.py](examples/quickstart_harmonized_api.py) | Raster/vector/lidar metadata quickstart |
| 2 | [examples/current_api_data_handling_demo.py](examples/current_api_data_handling_demo.py) | End-to-end object read/process/write |
| 3 | [examples/sensor_bundle_overview.py](examples/sensor_bundle_overview.py) | Supported sensor bundle families, band/measurement access, and preview outputs |
| 4 | [examples/vector_attributes_harmonized_api.py](examples/vector_attributes_harmonized_api.py) | Vector schema + attribute access aliases |
| 5 | [examples/vector_multifile_write_demo.py](examples/vector_multifile_write_demo.py) | Shapefile/MapInfo dataset-aware outputs |
| 6 | [examples/raster_numpy_roundtrip.py](examples/raster_numpy_roundtrip.py) | 2D NumPy roundtrip |
| 7 | [examples/raster_numpy_multiband_roundtrip.py](examples/raster_numpy_multiband_roundtrip.py) | 3D NumPy roundtrip (bands-first and rows-cols-bands) |
| — | [examples/licensing_offline_example.py](examples/licensing_offline_example.py) | Offline signed entitlement startup |
| — | [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py) | Floating license startup |

Run from this directory:

```bash
python examples/quickstart_harmonized_api.py
```

## Recommended API pattern

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
wbe.working_directory = '/path/to/data'

dem = wbe.read_raster('dem.tif')
meta = dem.metadata()
print(meta.rows, meta.columns, meta.nodata)

filled = wbe.hydrology.fill_depressions(dem)
accum = wbe.hydrology.d8_flow_accum(filled)
wbe.write_raster(accum, 'flow_accum.tif')
```

## Quick start examples by data type

### Raster

```python
# Read and inspect
dem = wbe.read_raster('dem.tif')
meta = dem.metadata()
print(f'Size: {meta.rows} x {meta.columns}, CRS: {meta.crs_epsg()}')

# Apply a tool
slope = wbe.terrain.slope(dem)

# Write result (default GeoTIFF behavior uses backend defaults)
wbe.write_raster(slope, 'slope_default.tif')

# Extensionless path defaults to COG-style GeoTIFF
wbe.write_raster(slope, 'slope_default')  # writes slope_default.tif

# Write with explicit GeoTIFF/COG controls
wbe.write_raster(
  slope,
  'slope_cog.tif',
  options={
    'compress': True,
    'strict_format_options': True,
    'geotiff': {
      'compression': 'deflate',
      'bigtiff': False,
      'layout': 'cog',
      'tile_size': 512,
    },
  },
)

# Batch write with one option profile
wbe.write_rasters(
  [slope],
  ['slope_tiled.tif'],
  options={
    'compress': False,
    'geotiff': {
      'layout': 'tiled',
      'tile_width': 256,
      'tile_height': 256,
    },
  },
)
```

## Raster output controls

`WbEnvironment.write_raster(...)` and `WbEnvironment.write_rasters(...)` accept an
`options` dictionary for output control.

Supported keys:

- `compress` (`True`/`False`): convenience toggle for GeoTIFF compression.
  - `True` maps to `deflate`.
  - `False` maps to uncompressed GeoTIFF.
- `strict_format_options` (`True`/`False`): when `True`, using GeoTIFF options on
  non-GeoTIFF outputs raises an error.
- `geotiff` (dict): GeoTIFF/COG-specific controls.
  - `compression`: `none`, `deflate`, `lzw`, `packbits`, `jpeg`, `webp`, `jpegxl`
  - `bigtiff`: `True` or `False`
  - `layout`: `standard`, `stripped`, `tiled`, `cog`
  - `rows_per_strip` (for `stripped`)
  - `tile_width`, `tile_height` (for `tiled`)
  - `tile_size` (for `cog`)

Notes:

- For GeoTIFF outputs (`.tif`, `.tiff`), options are applied directly.
- For non-GeoTIFF outputs, GeoTIFF-specific options are ignored unless
  `strict_format_options=True`.
- Backend GeoTIFF default compression is Deflate when no explicit override is
  provided.

- If no output extension is provided (for example `"my_file"`),
  `write_raster(...)` defaults to COG-style GeoTIFF output at
  `"my_file.tif"`.

### Common output profiles

```python
# 1) Standard GeoTIFF (default backend behavior)
wbe.write_raster(result, 'out_standard.tif')

# 2) Explicit stripped GeoTIFF
wbe.write_raster(
  result,
  'out_stripped.tif',
  options={
    'geotiff': {
      'layout': 'stripped',
      'rows_per_strip': 32,
    },
  },
)

# 3) Explicit tiled GeoTIFF
wbe.write_raster(
  result,
  'out_tiled.tif',
  options={
    'geotiff': {
      'layout': 'tiled',
      'tile_width': 256,
      'tile_height': 256,
    },
  },
)

# 4) Cloud-Optimized GeoTIFF (COG)
wbe.write_raster(
  result,
  'out_cog.tif',
  options={
    'compress': True,
    'geotiff': {
      'layout': 'cog',
      'tile_size': 512,
      'bigtiff': False,
    },
  },
)
```

### Extensionless defaults (all data objects)

When `output_path` has no extension:

- `write_raster(...)` writes COG-style GeoTIFF to `*.tif`
- `write_vector(...)` writes GeoPackage to `*.gpkg`
- `write_lidar(...)` writes COPC to `*.copc.laz`

`write_lidar(...)` also accepts optional per-format write controls through
`options={...}`.

Examples:

```python
wbe.write_raster(raster, 'my_file')  # my_file.tif (COG-style default)
wbe.write_vector(vector, 'my_file')  # my_file.gpkg
wbe.write_lidar(lidar, 'my_file')    # my_file.copc.laz
```

### Sensor bundle (Sentinel-2)

```python
# Open a Sentinel-2 SAFE bundle
s2 = wbe.read_sentinel2('S2A_MSIL2A_20250714T160911_N0511_R097_T17TNH_20250714T221309.SAFE')

# Inspect bundle metadata
print(s2.family)
print(s2.tile_id(), s2.processing_level(), s2.cloud_cover_percent())
print(s2.list_band_keys())

# Read individual bands by key
red = s2.read_band('B04')
green = s2.read_band('B03')
blue = s2.read_band('B02')

# Build and persist composites using the bundle-aware helpers
rgb = wbe.true_colour_composite(s2.bundle_root, output='sentinel2_rgb.tif')
nir = wbe.false_colour_composite(s2.bundle_root, output='sentinel2_nir.tif')

# Or use the Bundle convenience delegates (same result)
rgb = s2.true_colour_composite(wbe, output='sentinel2_rgb.tif')
```

For a broader multi-family example, see [examples/sensor_bundle_overview.py](examples/sensor_bundle_overview.py).

### Vector

```python
# Read and inspect
roads = wbe.read_vector('roads.shp')
schema = roads.schema()
print(f'Geometry: {schema.geometry_type}, Fields: {len(schema.fields)}')

# Access attributes
for i in range(min(3, roads.num_records())):
    attrs = roads.attributes(i)
    print(f'Record {i}: {attrs}')

# Process and persist
buffered = wbe.gis.buffer_vector(roads, distance=10)
wbe.write_vector(buffered, 'roads_buffer.shp')

# Extensionless path defaults to GeoPackage
wbe.write_vector(buffered, 'roads_buffer')  # writes roads_buffer.gpkg
```

## Vector output controls

`WbEnvironment.write_vector(...)`, `WbEnvironment.read_vector(...)`, and
`WbEnvironment.read_vectors(...)` accept optional `options` dictionaries.

Supported keys:

- `strict_format_options` (`True`/`False`): when `True`, using format-specific
  vector options on non-matching formats raises an error.
- `geoparquet` (dict): GeoParquet write controls.
  - `compression`: `none`, `snappy`, `gzip`, `lz4`, `zstd`, `brotli`
  - `max_rows_per_group`
  - `data_page_size_limit`
  - `write_batch_size`
  - `data_page_row_count_limit`
- `osmpbf` (dict): OSM PBF read controls.
  - `highways_only` (`True`/`False`)
  - `named_ways_only` (`True`/`False`)
  - `polygons_only` (`True`/`False`)
  - `include_tag_keys` (list of strings)

Notes:

- GeoParquet options are applied only when writing `.parquet` outputs.
- OSM PBF options are applied only when reading `.osm.pbf` inputs.
- When `strict_format_options=False`, non-applicable vector options are ignored.

### Common vector option profiles

```python
# 1) GeoParquet write with explicit compression/row-group controls
wbe.write_vector(
  roads,
  'roads.parquet',
  options={
    'strict_format_options': True,
    'geoparquet': {
      'compression': 'zstd',
      'max_rows_per_group': 250000,
      'write_batch_size': 8192,
    },
  },
)

# 2) OSM PBF read with filtering controls
roads_from_osm = wbe.read_vector(
  'region.osm.pbf',
  options={
    'osmpbf': {
      'highways_only': True,
      'named_ways_only': True,
      'include_tag_keys': ['name', 'highway', 'maxspeed'],
    },
  },
)
```

### Lidar

```python
# Read and inspect
las = wbe.read_lidar('survey.las')
meta = las.metadata()
print(f'Points: {meta.num_points}, CRS: {meta.crs_epsg()}')

# Apply a tool
norms = wbe.lidar.calculate_point_normals(las)

# Write result
wbe.write_lidar(norms, 'survey_normals.las')

# Extensionless path defaults to COPC
wbe.write_lidar(norms, 'survey_normals')  # writes survey_normals.copc.laz

# Optional format-specific write controls
wbe.write_lidar(
  norms,
  'survey_normals.copc.laz',
  options={
    'copc': {
      'max_points_per_node': 75000,
      'max_depth': 8,
      'node_point_ordering': 'hilbert',
    },
  },
)

wbe.write_lidar(
  norms,
  'survey_normals.laz',
  options={
    'laz': {
      'chunk_size': 25000,
      'compression_level': 7,
    },
  },
)
```

## Lidar output controls

`WbEnvironment.write_lidar(...)` accepts optional `options` dictionaries.

Supported keys:

- `laz` (dict): LAZ write controls.
  - `chunk_size` (positive integer)
  - `compression_level` (`0`-`9`)
- `copc` (dict): COPC write controls.
  - `max_points_per_node` (positive integer)
  - `max_depth` (positive integer)
  - `node_point_ordering`: `auto`, `morton`, `hilbert`

Notes:

- LAZ options are applied when writing `.laz` outputs.
- COPC options are applied when writing `.copc.laz` outputs.
- Non-applicable lidar options are ignored for other output formats.

### Common lidar option profiles

```python
# 1) LAZ write with tuned compression/chunking
wbe.write_lidar(
  norms,
  'survey_normals.laz',
  options={
    'laz': {
      'chunk_size': 25000,
      'compression_level': 7,
    },
  },
)

# 2) COPC write with octree controls
wbe.write_lidar(
  norms,
  'survey_normals.copc.laz',
  options={
    'copc': {
      'max_points_per_node': 75000,
      'max_depth': 8,
      'node_point_ordering': 'hilbert',
    },
  },
)
```

### Progress and feedback

Long-running tools can report progress via a callback function:

```python
filled = wbe.hydrology.fill_depressions(
  input_dem=dem.file_path,
  callback=wb.callbacks.print_progress,
)
```

You can also import the root-level alias:

```python
filled = wbe.hydrology.fill_depressions(
  input_dem=dem.file_path,
  callback=wb.print_progress,
)
```

For custom verbosity, use the callback factory:

```python
progress_cb = wb.callbacks.make_progress_printer(
  min_increment=5,
  show_messages=True,
)

filled = wbe.hydrology.fill_depressions(
  input_dem=dem.file_path,
  callback=progress_cb,
)
```

The standard callback also parses percentages embedded in message text when an
explicit numeric progress field is missing (for example:
`"Progress (loop 1 of 2): 50%"`).

You can also wrap progress in a more structured way (e.g., with a progress bar).
If you implement your own callback, handle all event payload shapes
(JSON string, dictionary, or object attributes):

```python
import json
import re
from tqdm import tqdm

PERCENT_IN_MESSAGE = re.compile(r"(-?\d+(?:\.\d+)?)\s*%")

def normalize_event(event):
  parsed = event
  if isinstance(event, str):
    try:
      parsed = json.loads(event)
    except json.JSONDecodeError:
      return None, None, event

  if isinstance(parsed, dict):
    return parsed.get("type"), parsed.get("percent"), parsed.get("message")

  return (
    getattr(parsed, "type", None),
    getattr(parsed, "percent", None),
    getattr(parsed, "message", None),
  )

def infer_percent_from_message(message):
  if message is None:
    return None
  m = PERCENT_IN_MESSAGE.search(str(message))
  return float(m.group(1)) if m else None

class ProgressTracker:
  def __init__(self):
    self.pbar = tqdm(total=100, desc="Running")
    self.last = 0

  def __call__(self, event):
    event_type, raw_percent, message = normalize_event(event)
    if raw_percent is None and message:
      raw_percent = infer_percent_from_message(message)

    if event_type == "progress" and raw_percent is not None:
      percent = float(raw_percent)
      if percent <= 1.0:
        percent *= 100.0
      pct = max(0, min(100, int(percent)))
      self.pbar.update(max(0, pct - self.last))
      self.last = max(self.last, pct)

    if message:
      self.pbar.set_postfix_str(str(message), refresh=False)

    if self.last >= 100:
      self.pbar.close()

tracker = ProgressTracker()
result = wbe.hydrology.fill_depressions(input_dem=dem.file_path, callback=tracker)
```

## Memory-first execution model

Default behavior is memory-first for intermediates:

- If an output path is omitted, operations return memory-backed objects.
- Persist only when needed with `write_raster`, `write_vector`, or `write_lidar`.
- This reduces unnecessary disk I/O in chained workflows.

Example:

```python
tmp = wbe.raster.sqrt(dem)
out = wbe.raster.log10(tmp)
wbe.write_raster(out, 'sqrt_log10.tif')
```

### ArcPy-style I/O vs wbw_python memory-first chaining

ArcPy workflows often write each intermediate to disk as `input -> tool -> output`,
which can increase I/O overhead in multi-step pipelines.

In wbw_python, intermediate objects are memory-backed unless an explicit output path
is provided. This means you can chain operations in memory and persist only final
artifacts, reducing unnecessary disk writes.

## Reprojection patterns

```python
dst_epsg = 32618

dem_utm = wbe.reproject_raster(dem, dst_epsg=dst_epsg)
roads = wbe.read_vector('roads.shp')
roads_utm = wbe.reproject_vector(roads, dst_epsg=dst_epsg)

wbe.write_raster(dem_utm, 'dem_utm.tif')
wbe.write_vector(roads_utm, 'roads_utm.shp')
```

### Reprojection best practices

- **Preserve precision**: Use high-precision resampling (`'bilinear'` or `'cubic'`) for continuous data; `'nearest'` for categorical.
- **Verify CRS**: Always inspect `crs_epsg()` before and after reprojection to confirm the transform.
- **CRS mismatch**: If input CRS is unknown or incorrect, call `set_crs_epsg()` before reprojection.
- **Memory-first chaining**: Reprojection returns memory-backed objects; persist with `write_raster` or `write_vector`.
- **Coordinate order**: EPSG defines lat/lon order; Whitebox always uses lon/lat internal. Transforms are applied automatically.

## NumPy interoperability

```python
arr = dem.to_numpy(dtype='float64')
arr = arr + 1.0
new_dem = wb.Raster.from_numpy(arr, dem, output_path='dem_plus1.tif')
```

Multiband workflows support both `(bands, rows, cols)` and `(rows, cols, bands)` 3D arrays.

## Rasterio interoperability

Rasterio interoperability is best approached through GeoTIFF exchange when you want
to reuse Rasterio profiles, windows, masking, or block-aware I/O.

```python
import rasterio

# Export a wbw raster to a Rasterio-friendly format
wbe.write_raster(dem, 'dem_for_rasterio.tif')

with rasterio.open('dem_for_rasterio.tif') as src:
  arr = src.read(1)
  profile = src.profile

# Example Rasterio-side processing
arr = arr * 1.05

profile.update(dtype='float32', count=1)
with rasterio.open('dem_rasterio_processed.tif', 'w', **profile) as dst:
  dst.write(arr.astype('float32'), 1)

# Bring result back into wbw_python
dem_processed = wbe.read_raster('dem_rasterio_processed.tif')
```

## GeoPandas interoperability

For vector workflows, write to a GeoPandas-friendly dataset (e.g., GeoPackage),
process with GeoPandas/Shapely, then read back into wbw_python.

```python
import geopandas as gpd

wbe.write_vector(roads, 'roads_for_gpd.gpkg')
gdf = gpd.read_file('roads_for_gpd.gpkg')

# Example GeoPandas processing
gdf['length_m'] = gdf.length
gdf = gdf[gdf['length_m'] > 25.0]

gdf.to_file('roads_gpd_filtered.gpkg', driver='GPKG')
roads_filtered = wbe.read_vector('roads_gpd_filtered.gpkg')
```

## Shapely interoperability

Shapely integrates naturally with GeoPandas geometry columns; this is a convenient
path for advanced geometry operations before returning data to wbw_python.

```python
import geopandas as gpd
from shapely import simplify

wbe.write_vector(streams, 'streams_for_shapely.gpkg')
gdf = gpd.read_file('streams_for_shapely.gpkg')

# Example Shapely operation
gdf['geometry'] = gdf.geometry.apply(lambda geom: simplify(geom, tolerance=2.0))

gdf.to_file('streams_simplified.gpkg', driver='GPKG')
streams_simplified = wbe.read_vector('streams_simplified.gpkg')
```

## xarray/rioxarray interoperability

Use rioxarray for labeled raster workflows (coordinates, lazy loading, xarray ops),
then write results back to GeoTIFF for wbw_python ingestion.

```python
import rioxarray as rxr

wbe.write_raster(dem, 'dem_for_xarray.tif')
da = rxr.open_rasterio('dem_for_xarray.tif').squeeze(drop=True)

# Example xarray computation
da_smooth = da.rolling(x=3, y=3, center=True).mean()

da_smooth.rio.to_raster('dem_xarray_smoothed.tif')
dem_smoothed = wbe.read_raster('dem_xarray_smoothed.tif')
```

## pyproj interoperability

Use pyproj when you need explicit CRS inspection, custom transformation pipelines,
or CRS comparisons alongside wbw_python metadata.

```python
from pyproj import CRS

src_epsg = dem.metadata().crs_epsg()
src_crs = CRS.from_epsg(src_epsg)
dst_crs = CRS.from_epsg(32618)

print('Source:', src_crs.to_string())
print('Destination:', dst_crs.to_string())

dem_utm = wbe.reproject_raster(dem, dst_epsg=dst_crs.to_epsg())
```

### Interoperability strategy

- In-memory numeric exchange: use `to_numpy(...)` / `from_numpy(...)`.
- Rich raster ecosystem tools: exchange via GeoTIFF (`write_raster` / `read_raster`).
- Rich vector ecosystem tools: exchange via GeoPackage/Shapefile (`write_vector` / `read_vector`).
- Keep wbw_python as the geoprocessing engine and use ecosystem libraries where they are strongest.

## Supported file formats

The Python API format support comes from backend crates:

- Raster formats come from [`wbraster`](../wbraster).
- Vector formats come from [`wbvector`](../wbvector).
- LiDAR formats come from [`wblidar`](../wblidar).

### Raster (via wbraster)

Read/write support includes:

- GeoTIFF / BigTIFF / COG (`.tif`, `.tiff`)
- JPEG2000 / GeoJP2 (`.jp2`)
- GeoPackage raster (`.gpkg`)
- ENVI (`.hdr` with sidecar data files)
- ER Mapper (`.ers`)
- Esri ASCII (`.asc`, `.grd`)
- Esri Binary Grid (`.adf` workspace)
- GRASS ASCII (`.asc`, `.txt`)
- Idrisi (`.rdc`, `.rst`)
- PCRaster (`.map`)
- SAGA (`.sgrd`, `.sdat`)
- Surfer GRD (`.grd`)
- Zarr (`.zarr`)

#### Satellite sensor bundles (read-only)

`wbraster` also supports read-only satellite sensor bundle ingestion. These are
package-level readers (bundle metadata + band/measurement/asset resolution), not
generic raster write targets.

Supported bundle families:

- Sentinel-2 SAFE
- Sentinel-1 SAFE
- Landsat Collection bundles
- ICEYE bundles
- PlanetScope bundles
- SPOT/Pleiades DIMAP bundles
- Maxar/WorldView bundles
- RADARSAT-2 bundles
- RCM bundles

### Vector (via wbvector)

Read/write support includes:

- Shapefile (`.shp` + sidecars)
- GeoPackage (`.gpkg`)
- GeoJSON (`.geojson`)
- FlatGeobuf (`.fgb`)
- GML (`.gml`)
- GPX (`.gpx`)
- KML (`.kml`)
- MapInfo Interchange (`.mif` + `.mid`)

Additional feature-gated formats in `wbvector`:

- GeoParquet (`.parquet`)
- KMZ (`.kmz`)
- OSM PBF (`.osm.pbf`, read-only)

### LiDAR (via wblidar)

Read/write support includes:

- LAS
- LAZ
- COPC
- PLY
- E57

## Licensing overview

The runtime supports open and licensed modes.

- Open mode: instantiate `WbEnvironment()` directly.
- Signed entitlement mode: bootstrap from signed JSON or file.
- Floating license mode: online provider-verified activation using `from_floating_license_id(...)`.

See:
- [examples/licensing_offline_example.py](examples/licensing_offline_example.py)
- [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py)

## Licensing and Pro workflows

This section focuses on day-to-day patterns for integrating licensing into production
scripts, notebooks, services, and plugin-style applications.

### 1) Choose a startup mode

- Open mode: best for open-tier workflows and development where Pro tools are not required.
- Signed entitlement mode: best when users can provide a signed offline entitlement.
- Floating license mode: best when you want online lease activation against the provider service.

### 2) Keep initialization centralized

Use a single startup function that creates and validates the environment once, then pass
that `WbEnvironment` instance through your pipeline.  Choose the factory that matches
your deployment:

```python
import whitebox_workflows as wb

# ---- Open mode (no license required) ----
wbe = wb.WbEnvironment()                    # include_pro=False, tier='open'

# ---- Floating license (online lease) ----
wbe = wb.WbEnvironment.from_floating_license_id(
  floating_license_id='fl_12345',
  include_pro=True,
  fallback_tier='open',
  provider_url='https://license.example.com',
  machine_id='machine-01',
  customer_id='customer-abc',
)
# Tip: provider_url can also be supplied by environment variable WBW_LICENSE_PROVIDER_URL.

# ---- Signed entitlement (offline, from file) ----
wbe = wb.WbEnvironment.from_signed_entitlement_file(
    entitlement_file='./signed_entitlement.json',
    public_key_kid='k1',
    public_key_b64url='REPLACE_WITH_PROVIDER_PUBLIC_KEY',
    include_pro=True,
    fallback_tier='open',
)

# ---- Signed entitlement (offline, from string) ----
wbe = wb.WbEnvironment.from_signed_entitlement_json(
    signed_entitlement_json=my_entitlement_json_string,
    public_key_kid='k1',
    public_key_b64url='REPLACE_WITH_PROVIDER_PUBLIC_KEY',
    include_pro=True,
    fallback_tier='open',
)
```

For complete runnable examples see:
- [examples/licensing_offline_example.py](examples/licensing_offline_example.py)
- [examples/licensing_floating_online_example.py](examples/licensing_floating_online_example.py)

## Recent Checkpoints

- 2026-04-10: Restored package loading by aligning the compiled module path with the package layout (`whitebox_workflows.whitebox_workflows`).
- 2026-04-10: Restored memory-first object workflows for dynamic category calls, including object inputs, working-directory-relative outputs, memory-backed unary raster chaining, and typed single-output returns.
- 2026-04-10: Added typed multi-output coercion for dynamic category calls so multi-output raster/vector/lidar tools return data objects instead of raw output-path dictionaries.

### 3) Check Pro tool visibility at startup

Verify that expected Pro tools are actually available before entering a Pro workflow
branch.  Use this as a guard when `include_pro=True` but the entitlement may have
fallen back to open tier.

```python
pro_tools = {'raster_power', 'sar_coregistration'}   # tools that require Pro
available = set(wbe.list_tools())
missing = pro_tools - available
if missing:
    raise RuntimeError(
        f'Pro entitlement active but required tools are missing: {sorted(missing)}'
    )
```

### 4) Gate Pro workflows explicitly

Treat Pro execution as a deliberate branch in your application logic.  Return a result
from both sides so callers do not need to know which path was taken.

```python
def run_backscatter_correction(wbe, image, use_pro: bool):
    if use_pro:
        # Pro branch: full refined radiometric correction.
        coregistered = wbe.sar_tools.sar_coregistration(image)
        return wbe.sar_tools.refined_lee_filter(coregistered)
    # Open fallback: basic spatial filter only.
    return wbe.image_tools.lee_filter(image)
```

Keeping the fallback explicit makes it easy to audit which capabilities require a license
and to test open-mode coverage in CI without a Pro entitlement.

### 5) Operational recommendations

- Keep secrets and signed entitlement payloads out of source control.
- Prefer configuration-driven startup (env vars or config file) over hard-coded license values.
- In CI, run open-mode smoke tests by default and isolate Pro tests to approved environments.
- For long-running jobs with floating licenses, include retry and renewal-aware error handling.

## Discovery APIs

```python
tools = wbe.list_tools()
categories = wbe.categories()
rs_tools = wbe.remote_sensing.list_tools()
info = wbe.describe_tool('slope')
matches = wbe.search_tools('flow accumulation')
```

### Subcategory Browsing (Autocomplete-Friendly)

Large categories expose optional subcategory groupings for easier discovery in editors:

```python
# Category -> subcategory -> tool
out1 = wbe.remote_sensing.filters.canny_edge_detection(input='image.tif')
out2 = wbe.raster.overlay_math.add(input1='a.tif', input2='b.tif')
out3 = wbe.terrain.derivatives.slope(dem='dem.tif', units='degrees')

# Introspection helpers
print(wbe.remote_sensing.list_subcategories())
print(wbe.terrain.derivatives.list_tools())
```

Compatibility note: direct category tool access still works (for example,
`wbe.terrain.slope(...)`).

`other` remains available as `wbe.other`, but `wbe.categories()` omits it when
there are no currently visible tools in that bucket.

## IntelliSense in VS Code

If completions are stale, ensure VS Code uses the same interpreter where wbw_python is installed.

1. Run Python: Select Interpreter.
2. Select your .venv-wbw interpreter.
3. Reload window.
4. Restart language server.

Optional workspace pin in .vscode/settings.json:

```json
{
  "python.defaultInterpreterPath": "/Users/<you>/Documents/programming/Rust/whitebox_next_gen/.venv-wbw/bin/python"
}
```

If needed, remove legacy global path overrides such as:
- `python.analysis.extraPaths`
- `python.autoComplete.extraPaths`
