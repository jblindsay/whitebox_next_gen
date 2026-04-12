# Supported Data Formats

This chapter summarizes practical format support exposed through WbW-R.

Choosing formats is a workflow architecture decision, not just a file-extension
choice. You should evaluate read/write support together with ecosystem fit,
compression behavior, and long-term maintainability of outputs shared across
toolchains.

Backend support comes from core crates:
- Raster: `wbraster`
- Vector: `wbvector`
- Lidar: `wblidar`

## Raster Formats

Exhaustive raster format support in the current WbW-R build:

| Format | Read (`wbw_read_raster`) | Write (`wbw_write_raster`) | Common extensions / path rules |
|---|---|---|---|
| Esri ASCII Grid | Yes | Yes | `.asc` (and `.grd` when detected as ASCII) |
| Esri Binary Grid workspace | Yes | Backend-only | Esri Binary workspace directory (`hdr.adf` + `w001001.adf`) or `.adf` |
| GRASS ASCII Raster | Yes | Yes | `.txt` / `.asc` when GRASS header keys are detected |
| Surfer GRD | Yes | Yes | `.grd` (DSAA / DSRB signatures) |
| PCRaster | Yes | Yes | `.map` (CSF signature) |
| SAGA Binary Grid | Yes | Yes | `.sdat`, `.sgrd` |
| Idrisi / TerrSet Raster | Yes | Yes | `.rst`, `.rdc` |
| ER Mapper | Yes | Yes | `.ers` |
| ENVI HDR-labelled raster | Yes | Yes | `.hdr`, or data files (`.img`, `.dat`, `.bin`, `.raw`, `.bil`, `.bsq`, `.bip`) with `.hdr` sidecar |
| GeoTIFF / BigTIFF / COG | Yes | Yes | `.tif`, `.tiff` |
| GeoPackage raster | Yes | Yes | `.gpkg` |
| JPEG2000 / GeoJP2 | Yes | Yes | `.jp2` |
| Zarr | Yes | Yes | `.zarr` store (directory / suffix) |

Typical pattern:

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
wbw_write_raster(r, 'dem_out.tif')
```

## Vector Formats

Exhaustive vector format support in the current WbW-R build:

| Format | Read (`wbw_read_vector`) | Write (`wbw_write_vector`) | Extensions / notes |
|---|---|---|---|
| FlatGeobuf | Yes | Yes | `.fgb` |
| GeoJSON | Yes | Yes | `.geojson`, `.json` |
| GeoPackage | Yes | Yes | `.gpkg` |
| GeoParquet | Yes | Yes | `.parquet` |
| GML | Yes | Yes | `.gml` |
| GPX | Yes | Yes | `.gpx` |
| KML | Yes | Yes | `.kml` |
| KMZ | Yes | Yes | `.kmz` |
| MapInfo Interchange | Yes | Yes | `.mif` with `.mid` sidecar |
| OSM PBF | Yes | No | `.osm.pbf` (read workflows only) |
| ESRI Shapefile | Yes | Yes | `.shp` plus dataset sidecars |

When `wbw_write_vector(...)` is called without an extension, WbW-R defaults output to `.gpkg`.

Typical pattern:

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
wbw_write_vector(v, 'roads_out.gpkg')
```

## Lidar Formats

Exhaustive lidar format support in the current WbW-R build:

| Format | Read (`wbw_read_lidar`) | Write (`wbw_write_lidar`) | Extensions / notes |
|---|---|---|---|
| LAS | Yes | Yes | `.las` |
| LAZ | Yes | Yes | `.laz` |
| COPC | Yes | Yes | `.copc.las`, `.copc.laz` |
| PLY | Yes | Yes | `.ply` |
| E57 | Yes | Yes | `.e57` |

When `wbw_write_lidar(...)` is called without an extension, WbW-R defaults output to `.copc.laz`.

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

Bundle inputs may be either extracted directories or supported archives:
- `.zip`
- `.tar`
- `.tar.gz`
- `.tgz`

See [Working with Sensor Bundles](./working-with-sensor-bundles.md) for family-specific examples.

## Validation Guidance

1. Prefer stable interchange formats (`.tif`, `.gpkg`, `.copc.laz`) for production pipelines.
2. Re-open outputs and verify metadata after write operations.
3. Use explicit options where format behavior must be reproducible.