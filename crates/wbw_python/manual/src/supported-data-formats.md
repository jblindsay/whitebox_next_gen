# Supported Data Formats

This chapter summarizes practical format support exposed through WbW-Py.

Backend support comes from core crates:
- Raster: `wbraster`
- Vector: `wbvector`
- Lidar: `wblidar`

## Raster Formats

Commonly used raster formats include:
- GeoTIFF / BigTIFF / COG (`.tif`, `.tiff`)
- JPEG2000 / GeoJP2 (`.jp2`)
- GeoPackage raster (`.gpkg`)
- ENVI (`.hdr` with sidecar data)
- ER Mapper (`.ers`)

Typical pattern:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')
wbe.write_raster(r, 'dem_out.tif')
```

## Vector Formats

Commonly used vector formats include:
- GeoPackage (`.gpkg`)
- Shapefile (`.shp` and sidecar dataset files)
- GeoJSON (`.geojson`, `.json`)
- GeoParquet (`.parquet`)
- OSM PBF read workflows (`.osm.pbf`)

Typical pattern:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
v = wbe.read_vector('roads.gpkg')
wbe.write_vector(v, 'roads_out.gpkg')
```

## Lidar Formats

Commonly used lidar formats include:
- LAS (`.las`)
- LAZ (`.laz`)
- COPC (`.copc.laz`)

Typical pattern:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
l = wbe.read_lidar('survey.las')
wbe.write_lidar(l, 'survey_out.copc.laz')
```

## Sensor Bundle Families

Supported bundle readers include:
- `read_bundle(...)` (auto-detect)
- `read_landsat(...)`
- `read_sentinel1(...)`
- `read_sentinel2(...)`
- `read_planetscope(...)`
- `read_iceye(...)`
- `read_dimap(...)`
- `read_maxar_worldview(...)`
- `read_radarsat2(...)`
- `read_rcm(...)`

See [Working with Sensor Bundles](./working-with-sensor-bundles.md) for family-specific examples.

## Validation Guidance

1. Prefer stable interchange formats (`.tif`, `.gpkg`, `.copc.laz`) for production pipelines.
2. Re-open outputs and verify metadata after write operations.
3. Use explicit options where format behavior must be reproducible.