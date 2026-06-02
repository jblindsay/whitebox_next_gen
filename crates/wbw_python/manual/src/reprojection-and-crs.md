# Reprojection and CRS

This chapter covers raster, vector, and lidar reprojection in WbW-Py.

CRS handling is a semantic correctness issue, not just a metadata preference.
When coordinate assumptions are wrong, analyses may still run but yield invalid
spatial conclusions. The patterns here emphasize explicit source/destination CRS
checks, controlled reprojection settings, and immediate post-transform
verification before any downstream computation.

## CRS Inspection

Run this first to verify assumptions before any reprojection call.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')
roads = wbe.read_vector('roads.gpkg')
las = wbe.read_lidar('survey.las')

print('raster epsg:', dem.metadata().epsg_code)
print('vector epsg:', roads.metadata().crs_epsg)
print('lidar epsg:', las.metadata().crs_epsg)
```

## Assigning Projection Metadata

Use CRS assignment only when the coordinates are already in the correct
coordinate system but the file metadata is missing or wrong. Assignment does
not move coordinates; it only changes the declared CRS. If you need to change
the coordinate values themselves, use the reprojection methods shown later in
this chapter.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem_without_crs.tif')
roads = wbe.read_vector('roads_without_crs.gpkg')
las = wbe.read_lidar('survey_without_crs.las')

# Assign by EPSG when you know the coordinates already use that CRS.
dem.set_crs_epsg(26917)
roads.set_crs_epsg(26917)

# Assign by WKT when that is the CRS information you have available.
utm_wkt = wbe.projection.to_ogc_wkt(26917)
las.set_crs_wkt(utm_wkt)

print('raster epsg after assignment:', dem.crs_epsg())
print('vector epsg after assignment:', roads.crs_epsg())
print('lidar epsg after assignment:', las.crs_epsg())

# If metadata is wrong rather than missing, clear it before reassigning.
roads.clear_crs()
roads.set_crs_epsg(26917)
```

This pattern is especially useful for legacy rasters, sidecar-free vector
exports, and lidar files that have correct coordinates but incomplete CRS
metadata.

## Raster Reprojection

WbW-Py exposes six raster-object reprojection methods. Use the one that matches
your grid-control needs:

1. `raster.reproject(...)`:
General method with full control over extent, rows/cols, resolution, snap
origin, nodata policy, antimeridian policy, grid-size policy, and destination
footprint.
2. `raster.reproject_nearest(dst_epsg, ...)`:
Convenience wrapper for nearest-neighbour reprojection.
3. `raster.reproject_bilinear(dst_epsg, ...)`:
Convenience wrapper for bilinear reprojection.
4. `raster.reproject_to_match_grid(target_grid, ...)`:
Reprojects and snaps exactly to another raster's grid geometry (extent, rows,
cols, resolution).
5. `raster.reproject_to_match_resolution(reference_grid, ...)`:
Reprojects while matching the reference raster's resolution and snap behavior.
6. `raster.reproject_to_match_resolution_in_epsg(dst_epsg, reference_grid, ...)`:
Reprojects to a specified EPSG while deriving output resolution controls from a
reference raster.

### Available resampling methods (wbraster)

Use these values anywhere a reprojection method accepts `resample`:

- `nearest`: category-safe nearest-neighbour.
- `bilinear`: smooth linear interpolation.
- `cubic`: bicubic interpolation.
- `lanczos`: high-quality sinc-window interpolation.
- `average`: 3x3 mean statistic.
- `min`: 3x3 minimum statistic.
- `max`: 3x3 maximum statistic.
- `mode`: 3x3 modal statistic.
- `median`: 3x3 median statistic.
- `stddev`: 3x3 standard deviation statistic.

Practical defaults:

- Categorical/class rasters: `nearest` (or `mode` for smoothing by majority).
- Continuous surfaces (DEM, reflectance, temperature): `bilinear`, `cubic`, or
  `lanczos`.
- Thematic/statistical resamples: `average`, `min`, `max`, `median`, `stddev`.

### Example: full-control reprojection

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')

dem_utm = dem.reproject(
    dst_epsg=32618,
    resample='bilinear',
    x_res=10.0,
    y_res=10.0,
    nodata_policy='partial_kernel',
    antimeridian_policy='auto',
    grid_size_policy='expand',
    destination_footprint='none',
)

wbe.write_raster(dem_utm, 'dem_utm_10m.tif')
```

### Example: epoch-aware raster reprojection (advanced)

Use epoch-aware options when transforming between dynamic datums/realizations.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem_csrs_v3.tif')

dem_v8 = dem.reproject(
    dst_epsg=22818,
    resample='bilinear',
    coordinate_epoch=2020.0,
    source_reference_epoch=2010.0,
    target_reference_epoch=2020.0,
    prefer_official_operation=True,
    epoch_policy='strict',
)

wbe.write_raster(dem_v8, 'dem_csrs_v8_epoch2020.tif')
```

Supported optional epoch-routing arguments in Python reprojection calls:

- `coordinate_epoch`
- `source_reference_epoch`
- `target_reference_epoch`
- `operation_code`
- `prefer_official_operation`
- `epoch_policy` (`strict` or `allow_static_fallback`)

### CSRS operational status (current)

For NAD83(CSRS) realization-routing in current WbW builds:

- Active preferred-operation corridors (zone-matched UTM, zones 7-24):
    - v3 -> v8 (`223xx -> 228xx`), operation `10715`
    - v4 -> v8 (`224xx -> 228xx`), operation `10715`
    - v6 -> v8 (`226xx -> 228xx`), operation `10715`
    - v7 -> v8 (`227xx -> 228xx`), operation `10715`

For CRS pairs without a registered preferred operation mapping, standard
reprojection remains available via the baseline transform path.

### Query CSRS support status at runtime

Use the runtime capabilities payload to inspect active vs pending CSRS
realization corridors in the current environment.

```python
import json
import whitebox_workflows as wb

caps = json.loads(wb.get_runtime_capabilities_json_with_options(include_pro=False, tier='open'))
csrs = caps.get('projection_csrs_preferred_operation_support', {})

print('zone range:', csrs.get('zone_min'), 'to', csrs.get('zone_max'))

for pair in csrs.get('pairs', []):
    if pair.get('status') == 'active':
        print(
            f"{pair.get('source_realization')} -> {pair.get('target_realization')} "
            f"op={pair.get('preferred_operation_code')}"
        )
```

Expected active set in current builds:

| Source | Target | Status | Operation | Zones |
|---|---|---|---:|---|
| v3 | v8 | active | 10715 | 7-24 |
| v4 | v8 | active | 10715 | 7-24 |
| v6 | v8 | active | 10715 | 7-24 |
| v7 | v8 | active | 10715 | 7-24 |

### Example: grid-matching reprojection

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
src = wbe.read_raster('landcover_4326.tif')
target = wbe.read_raster('dem_utm_10m.tif')

# Categorical raster: nearest is typically required.
aligned = src.reproject_to_match_grid(target, resample='nearest')
wbe.write_raster(aligned, 'landcover_aligned.tif')
```

### Automatic reprojection in raster-stack tools

Several stack-based tools now support automatic stack alignment with explicit
controls:

- `auto_reproject` (default `true`)
- `auto_reproject_method` (optional override)

Current behavior:

1. `inputs[0]` is treated as the reference raster.
2. Any stack raster with mismatched CRS is auto-reprojected to match the
   reference grid when `auto_reproject=true`.
3. If `auto_reproject_method` is unset:
   - categorical rasters infer `nearest`
   - continuous rasters infer `bilinear`
4. If extents do not overlap after alignment, tools raise a hard error.

This is especially important for stack workflows (`input_rasters`/`inputs`) such
as overlay operations, weighted sums, PCA, inverse PCA, raster calculator,
image segmentation, and position-based stack selection.

### Example: stack tool with automatic reprojection

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

result = wbe.run_tool(
    'weighted_sum',
    {
        'input_rasters': ['slope_utm.tif', 'landcover_4326.tif', 'distance_utm.tif'],
        'weights': [0.4, 0.35, 0.25],
        'auto_reproject': True,
        'auto_reproject_method': '',  # empty -> infer nearest/bilinear per raster
        'output': 'weighted_sum.tif',
    },
)
print(result)
```

## Vector Reprojection

This pattern is appropriate when geometry validity and failure policy need to be
explicit.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

roads_utm = wbe.projection_georeferencing.general.reproject_vector(
    roads,
    dst_epsg=32618,
    failure_policy='error',
    topology_policy='none',
)

wbe.write_vector(roads_utm, 'roads_utm.gpkg')
```

### Example: epoch-aware vector reprojection (advanced)

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
stations = wbe.read_vector('stations_csrs_v3.gpkg')

stations_v8 = stations.reproject(
    dst_epsg=22818,
    coordinate_epoch=2020.0,
    prefer_official_operation=True,
    epoch_policy='strict',
)

wbe.write_vector(stations_v8, 'stations_csrs_v8_epoch2020.gpkg')
```

## Lidar Reprojection

Use explicit lidar reprojection settings to avoid silent dimensional or policy
defaults.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
las = wbe.read_lidar('survey.las')

las_utm = wbe.projection_georeferencing.general.reproject_lidar(
    las,
    dst_epsg=32618,
    use_3d_transform=False,
    failure_policy='error',
)

wbe.write_lidar(las_utm, 'survey_utm.copc.laz')
```

### Example: epoch-aware lidar reprojection (advanced)

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
las = wbe.read_lidar('survey_csrs_v3.las')

las_v8 = las.reproject(
    dst_epsg=22818,
    coordinate_epoch=2020.0,
    prefer_official_operation=True,
    epoch_policy='strict',
)

wbe.write_lidar(las_v8, 'survey_csrs_v8_epoch2020.laz')
```

## Georeference Raster from Control Points

Use this tool when an image/raster has no reliable georeferencing and you have
ground-control points (GCPs) linking image pixel coordinates to map coordinates.

Required CSV fields:

- `source_col`
- `source_row`
- `target_x`
- `target_y`

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

result = wbe.projection_georeferencing.general.georeference_raster_from_control_points(
    input_raster='historical_scan.tif',
    control_points_csv='historical_scan_gcps.csv',
    epsg=32618,
    resample='bilinear',
    output='historical_scan_georef.tif',
    report='historical_scan_georef_report.json',  # optional diagnostics JSON
)

print(result)
```

If you need raw runtime invocation style, the equivalent tool ID is
`georeference_raster_from_control_points`.

## Projection Utility Namespace

This namespace is useful for CRS diagnostics and point-level coordinate
transform tasks outside full dataset reprojection.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

wkt_3857 = wbe.projection.to_ogc_wkt(3857)
print('epsg from wkt:', wbe.projection.identify_epsg(wkt_3857))

pts = [{'x': -79.3832, 'y': 43.6532}]
pts_utm = wbe.projection.reproject_points(pts, src_epsg=4326, dst_epsg=32618)
print(pts_utm)
```

### Parse a PROJ string

Use `projection.from_proj_string` when you have a PROJ4-style string (e.g.,
read from a legacy file header or third-party metadata) and need to identify
the corresponding EPSG code or obtain an OGC WKT representation.

The method returns a dict with exactly one of these keys:

- `{'epsg': int}` — EPSG code identified
- `{'wkt': str}` — no EPSG match, WKT representation available
- `{'unknown': True}` — PROJ string parsed but CRS could not be identified further

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

proj_str = '+proj=utm +zone=17 +datum=NAD83 +units=m +no_defs'
result = wbe.projection.from_proj_string(proj_str)

if 'epsg' in result:
    print('identified EPSG:', result['epsg'])  # e.g. 26917
elif 'wkt' in result:
    print('WKT:', result['wkt'])
else:
    print('CRS unknown')
```

This is the recommended fallback for legacy data sources that carry only a
PROJ4 metadata string. WbW-Py itself uses this path internally when
reprojecting rasters whose CRS metadata does not include an EPSG code.

### Area-of-use bounding box

Use `projection.area_of_use` to retrieve the geographic bounding box of valid
use for an EPSG code. This is useful for validating that your data actually
falls within the intended CRS domain before or after reprojection.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

bbox = wbe.projection.area_of_use(32618)  # UTM Zone 18N
if bbox is not None:
    print(f"valid lon: {bbox['lon_min']} to {bbox['lon_max']}")
    print(f"valid lat: {bbox['lat_min']} to {bbox['lat_max']}")

# Returns None for codes with no registered bounding box.
print(wbe.projection.area_of_use(9999))  # None
```

You can also pass the bounding box check as a pre-reprojection guard:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')

dst_epsg = 32618
bbox = wbe.projection.area_of_use(dst_epsg)
if bbox is not None:
    ext = dem.metadata().extent
    # Quick geographic sanity check before committing to full reprojection.
    in_range = (
        ext.min_x >= bbox['lon_min'] and ext.max_x <= bbox['lon_max'] and
        ext.min_y >= bbox['lat_min'] and ext.max_y <= bbox['lat_max']
    )
    if not in_range:
        print('WARNING: DEM extent may fall outside area of use for EPSG:', dst_epsg)

dem_utm = dem.reproject(dst_epsg=dst_epsg, resample='bilinear')
wbe.write_raster(dem_utm, 'dem_utm.tif')
```

## Best Practices

- Confirm source CRS before any reprojection.
- Use `nearest` for categorical raster data, `bilinear`/`cubic` for continuous data.
- Re-open outputs and verify CRS metadata post-transform.
- Keep transform options explicit for reproducible pipelines.
