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

## Best Practices

- Confirm source CRS before any reprojection.
- Use `nearest` for categorical raster data, `bilinear`/`cubic` for continuous data.
- Re-open outputs and verify CRS metadata post-transform.
- Keep transform options explicit for reproducible pipelines.
