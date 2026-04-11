# Reprojection and CRS

This chapter covers reprojection and CRS validation workflows in WbW-R.

## Inspect CRS

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
v <- wbw_read_vector('roads.gpkg')
l <- wbw_read_lidar('survey.las')

print(r$crs_epsg())
print(v$crs_epsg())
print(l$crs_epsg())
```

## Raster Reprojection Pattern

Use `wbw_run_tool(...)` for reprojection pipelines and then reopen typed objects.

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
