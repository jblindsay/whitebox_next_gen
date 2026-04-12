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

Use `wbw_run_tool(...)` for reprojection pipelines and then reopen typed objects.

This pattern keeps reprojection explicit and testable as a standalone step.

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_run_tool(
  'reproject_raster',
  args = list(input = 'dem.tif', output = 'dem_utm.tif', epsg = 32618, method = 'bilinear'),
  session = s
)

dem_utm <- wbw_read_raster('dem_utm.tif')
print(dem_utm$crs_epsg())
```

## Vector Reprojection Pattern

Use this when downstream geometry processing depends on a specific projected CRS.

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_run_tool(
  'reproject_vector',
  args = list(input = 'roads.gpkg', output = 'roads_utm.gpkg', epsg = 32618),
  session = s
)

roads_utm <- wbw_read_vector('roads_utm.gpkg')
print(roads_utm$crs_epsg())
```

## Lidar Reprojection Pattern

Use this when point-cloud alignment and metric operations require a target CRS.

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_run_tool(
  'lidar_reproject',
  args = list(input = 'survey.las', output = 'survey_utm.laz', epsg = 32618),
  session = s
)

survey_utm <- wbw_read_lidar('survey_utm.laz')
print(survey_utm$crs_epsg())
```

## Best Practices

- Confirm source CRS before transformation.
- Use interpolation appropriate to data type for raster reprojection.
- Re-open outputs and verify CRS metadata.
- Keep transform arguments explicit in reproducible scripts.
