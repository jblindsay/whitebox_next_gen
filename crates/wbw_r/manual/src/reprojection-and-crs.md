# Reprojection and CRS

This chapter covers reprojection and CRS validation workflows in WbW-R.

CRS operations are correctness-critical. A workflow can run successfully while
still producing invalid spatial interpretation if source and destination
reference systems are misunderstood. The patterns here prioritize explicit CRS
inspection, controlled reprojection calls, and immediate validation of outputs.

## Inspect CRS

Run this check before any reprojection to catch missing or incorrect source CRS
assumptions.

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
v <- wbw_read_vector('roads.gpkg')
l <- wbw_read_lidar('survey.las')

print(r$crs_epsg())
print(v$crs_epsg())
print(l$crs_epsg())
```

## Assigning Projection Metadata

Projection assignment and reprojection are different operations:

- Assignment updates CRS metadata only.
- Reprojection changes coordinate values.

In current WbW-R, direct object-level CRS assignment helpers are not exposed in
the same way as WbW-Py object methods. The practical assignment pattern is to
use R spatial libraries for metadata repair, then re-open data in WbW-R.

```r
library(whiteboxworkflows)
library(terra)
library(sf)

# Raster CRS assignment (metadata only)
r <- rast('dem_without_crs.tif')
crs(r) <- 'EPSG:26917'
writeRaster(r, 'dem_with_crs.tif', overwrite = TRUE)

# Vector CRS assignment (metadata only)
v <- st_read('roads_without_crs.gpkg', quiet = TRUE)
v <- st_set_crs(v, 26917)
st_write(v, 'roads_with_crs.gpkg', delete_dsn = TRUE, quiet = TRUE)

# Re-open in WbW-R for downstream analysis
r_wbw <- wbw_read_raster('dem_with_crs.tif')
v_wbw <- wbw_read_vector('roads_with_crs.gpkg')
print(r_wbw$crs_epsg())
print(v_wbw$crs_epsg())
```

If you need coordinate changes, use reprojection workflows in the next
sections rather than metadata assignment.

## Raster Reprojection Pattern

The core raster API provides six reprojection method patterns (documented here
so behavior is explicit across language bindings):

1. Full-options reprojection (`reproject`)
2. Nearest convenience (`reproject_nearest`)
3. Bilinear convenience (`reproject_bilinear`)
4. Reproject to match another raster grid (`reproject_to_match_grid`)
5. Reproject to match another raster resolution (`reproject_to_match_resolution`)
6. Reproject to target EPSG while matching a reference resolution
   (`reproject_to_match_resolution_in_epsg`)

In WbW-R, practical workflows typically call reprojection tools through
`wbw_<tool>(...)` wrappers (or `wbw_run_tool(...)` for dynamic tool-id workflows)
and then reopen outputs as typed objects.

### Available resampling methods (wbraster)

Use these method strings in reprojection workflows:

- `nearest`
- `bilinear`
- `cubic`
- `lanczos`
- `average`
- `min`
- `max`
- `mode`
- `median`
- `stddev`

Method guidance:

- Categorical/class rasters: `nearest` (or `mode` where majority behavior is desired).
- Continuous rasters: `bilinear`, `cubic`, or `lanczos`.
- Statistical downscaling/generalization: `average`, `min`, `max`, `median`, `stddev`.

### Example: explicit raster reprojection

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_reproject_raster(input = 'dem.tif',
    output = 'dem_utm.tif',
    epsg = 32618,
    method = 'bilinear')

dem_utm <- wbw_read_raster('dem_utm.tif')
print(dem_utm$crs_epsg())
```

### Example: match-grid categorical reprojection

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_reproject_raster(input = 'landcover_4326.tif',
    output = 'landcover_utm_aligned.tif',
    epsg = 32618,
    method = 'nearest')
```

### Automatic reprojection in raster-stack tools

Stack-based tools now support automatic alignment controls:

- `auto_reproject` (default `true`)
- `auto_reproject_method` (optional override)

Behavior for raster stacks:

1. `inputs[0]`/`input_rasters[0]` is the reference raster.
2. CRS-mismatched stack members are auto-reprojected to the reference grid when
   `auto_reproject=true`.
3. If `auto_reproject_method` is not set:
   - categorical rasters infer `nearest`
   - continuous rasters infer `bilinear`
4. Non-overlapping extents are treated as hard validation errors.

This matters most for tools that combine raster stacks (overlay, weighted sum,
PCA, inverse PCA, raster calculator, segmentation).

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_weighted_sum(input_rasters = c('slope_utm.tif', 'landcover_4326.tif', 'distance_utm.tif'),
    weights = c(0.4, 0.35, 0.25),
    auto_reproject = TRUE,
    auto_reproject_method = '',
    output = 'weighted_sum.tif')
```

## Vector Reprojection Pattern

Use this when downstream geometry processing depends on a specific projected CRS.

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_reproject_vector(input = 'roads.gpkg', output = 'roads_utm.gpkg', epsg = 32618)

roads_utm <- wbw_read_vector('roads_utm.gpkg')
print(roads_utm$crs_epsg())
```

## Lidar Reprojection Pattern

Use this when point-cloud alignment and metric operations require a target CRS.

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_reproject_lidar(input = 'survey.las', output = 'survey_utm.laz', epsg = 32618)

survey_utm <- wbw_read_lidar('survey_utm.laz')
print(survey_utm$crs_epsg())
```

## Georeference Raster from Control Points

Use this when a raster/image lacks reliable georeferencing and you have control
points mapping pixel coordinates to map coordinates.

Required CSV fields:

- `source_col`
- `source_row`
- `target_x`
- `target_y`

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_georeference_raster_from_control_points(input = 'historical_scan.tif',
    control_points = 'historical_scan_gcps.csv',
    epsg = 32618,
    resample = 'bilinear',
    output = 'historical_scan_georef.tif',
    report = 'historical_scan_georef_report.json')
```

## Best Practices

- Confirm source CRS before transformation.
- Use interpolation appropriate to data type for raster reprojection.
- Re-open outputs and verify CRS metadata.
- Keep transform arguments explicit in reproducible scripts.
