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

This example shows continuous-surface reprojection with explicit resampling and
resolution controls.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')

dem_utm = wbe.reproject_raster(
    dem,
    dst_epsg=32618,
    resample='bilinear',
    x_res=10.0,
    y_res=10.0,
)

wbe.write_raster(dem_utm, 'dem_utm_10m.tif')
```

## Vector Reprojection

This pattern is appropriate when geometry validity and failure policy need to be
explicit.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

roads_utm = wbe.reproject_vector(
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

las_utm = wbe.reproject_lidar(
    las,
    dst_epsg=32618,
    use_3d_transform=False,
    failure_policy='error',
)

wbe.write_lidar(las_utm, 'survey_utm.copc.laz')
```

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
