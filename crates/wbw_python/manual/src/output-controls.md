# Output Controls

This chapter documents output controls for raster, vector, and lidar writes in WbW-Py.

Output settings are where reproducibility becomes explicit. Defaults are useful
for fast iteration, but production workflows should pin format, compression, and
layout choices so outputs remain comparable across runs and environments. Treat
this chapter as the policy layer for how artifacts are written, named, and
validated.

## General Principles

- Start with default output behavior unless you need strict reproducibility.
- Use `strict_format_options=True` when you want invalid option/format combinations
to fail instead of silently ignoring options.
- Prefer extensionless outputs for sensible defaults when prototyping.

## Raster Output Controls

`write_raster(...)` and `write_rasters(...)` support an `options` dictionary.

Use this when output layout and compression are part of a reproducibility or
distribution requirement.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
r = wbe.read_raster('dem.tif')

wbe.write_raster(r, 'out_default.tif')

wbe.write_raster(
    r,
    'out_cog.tif',
    options={
        'compress': True,
        'strict_format_options': True,
        'geotiff': {
            'layout': 'cog',
            'tile_size': 512,
            'compression': 'deflate',
            'bigtiff': False,
        },
    },
)
```

### Common Raster Profiles

These profiles illustrate common tradeoffs between compatibility and read
performance.

```python
# Stripped GeoTIFF
wbe.write_raster(r, 'out_stripped.tif', options={
    'geotiff': {'layout': 'stripped', 'rows_per_strip': 32},
})

# Tiled GeoTIFF
wbe.write_raster(r, 'out_tiled.tif', options={
    'geotiff': {'layout': 'tiled', 'tile_width': 256, 'tile_height': 256},
})
```

## Vector Output Controls

`write_vector(...)` supports format-specific options.

Use strict format options in production so incompatible settings fail fast.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
v = wbe.read_vector('roads.gpkg')

# Extensionless output defaults to GeoPackage
wbe.write_vector(v, 'roads_copy')

# GeoParquet with strict options
wbe.write_vector(
    v,
    'roads.parquet',
    options={
        'strict_format_options': True,
        'geoparquet': {
            'compression': 'zstd',
            'max_rows_per_group': 250000,
            'write_batch_size': 8192,
        },
    },
)
```

## Lidar Output Controls

`write_lidar(...)` supports LAZ and COPC option blocks.

Choose LAZ for compact archives and COPC when cloud-native spatial access is
important.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
l = wbe.read_lidar('survey.las')

# LAZ controls
wbe.write_lidar(l, 'survey_out.laz', options={
    'laz': {'chunk_size': 25000, 'compression_level': 7},
})

# COPC controls
wbe.write_lidar(l, 'survey_out.copc.laz', options={
    'copc': {
        'max_points_per_node': 75000,
        'max_depth': 8,
        'node_point_ordering': 'hilbert',
    },
})
```

## Extensionless Defaults

Extensionless writes are useful in prototyping, but pin extensions in production
for deterministic artifact naming.

When no extension is provided:
- raster -> `.tif` (COG-style GeoTIFF default)
- vector -> `.gpkg`
- lidar -> `.copc.laz`

```python
wbe.write_raster(r, 'my_raster')
wbe.write_vector(v, 'my_vector')
wbe.write_lidar(l, 'my_lidar')
```

## Reproducibility Checklist

1. Pin output extension explicitly.
2. Set `strict_format_options=True` when format mismatches must error.
3. Pin codec/layout/tile parameters for raster outputs.
4. For vector/lidar, pin format-specific compression and partitioning options.
5. Re-open output and validate metadata before downstream analysis.
