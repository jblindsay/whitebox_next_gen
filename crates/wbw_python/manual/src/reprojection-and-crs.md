# Reprojection and CRS

This chapter covers raster, vector, and lidar reprojection in WbW-Py.

## CRS Inspection

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

## Raster Reprojection

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
