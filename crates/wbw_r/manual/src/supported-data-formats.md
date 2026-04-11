# Supported Data Formats

This chapter summarizes practical format support exposed through WbW-R.

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

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
wbw_write_raster(r, 'dem_out.tif')
```

## Vector Formats

Commonly used vector formats include:
- GeoPackage (`.gpkg`)
- Shapefile (`.shp` and sidecar dataset files)
- GeoJSON (`.geojson`, `.json`)
- GeoParquet (`.parquet`)
- OSM PBF read workflows (`.osm.pbf`)

Typical pattern:

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
wbw_write_vector(v, 'roads_out.gpkg')
```

## Lidar Formats

Commonly used lidar formats include:
- LAS (`.las`)
- LAZ (`.laz`)
- COPC (`.copc.laz`)

Typical pattern:

```r
library(whiteboxworkflows)

l <- wbw_read_lidar('survey.las')
wbw_write_lidar(l, 'survey_out.copc.laz')
```

## Sensor Bundle Families

Supported bundle readers include:
- `wbw_read_bundle(...)` (auto-detect)
- `wbw_read_landsat(...)`
- `wbw_read_sentinel1(...)`
- `wbw_read_sentinel2(...)`
- `wbw_read_planetscope(...)`
- `wbw_read_iceye(...)`
- `wbw_read_dimap(...)`
- `wbw_read_maxar_worldview(...)`
- `wbw_read_radarsat2(...)`
- `wbw_read_rcm(...)`

See [Working with Sensor Bundles](./working-with-sensor-bundles.md) for family-specific examples.

## Validation Guidance

1. Prefer stable interchange formats (`.tif`, `.gpkg`, `.copc.laz`) for production pipelines.
2. Re-open outputs and verify metadata after write operations.
3. Use explicit options where format behavior must be reproducible.