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

## Memory Lifecycle and Cleanup

For workflows that use memory-backed rasters, vectors, and lidar objects
(`file_mode='m'`), explicit lifecycle management prevents unbounded memory
growth in long-running jobs.

### When to use memory mode

Memory mode is most valuable when:
- Chaining multiple operations on the same data without disk I/O.
- Processing intermediate results that are never persisted to disk.
- Running batched analysis where you load, process, and clear per batch.
- Working with smaller datasets where memory is not a constraint.

Avoid memory mode when:
- Working with data larger than available RAM.
- Processing single operations on large files.
- Running unattended long-running jobs without explicit cleanup.

### Explicit cleanup in long-running pipelines

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

# Long-running batch analysis
for tile_id in range(1, 1001):
    print(f'Processing tile {tile_id}')
    
    # Read data into memory for this tile
    dem = wbe.read_raster(f'tile_{tile_id}.tif', file_mode='m')
    bounds = wbe.read_vector(f'bounds_{tile_id}.gpkg', file_mode='m')
    
    # Process
    result = wbe.run_tool('clip_raster_by_polygon', {
        'input': dem.file_path,
        'polygon': bounds.file_path,
        'output': f'clipped_{tile_id}.tif'
    })
    
    # Explicit cleanup before next iteration
    wbe.remove_raster_from_memory(dem)
    wbe.remove_vector_from_memory(bounds)
```

### Monitoring memory usage

For production scripts, track memory explicitly:

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

print(f"Initial raster memory: {wbe.raster_memory_bytes() / 1e6:.1f} MB")
print(f"Initial vector memory: {wbe.vector_memory_bytes() / 1e6:.1f} MB")

# ... run operations ...

# Before returning or starting new phase
print(f"Final raster count: {wbe.raster_memory_count()}")
print(f"Final raster memory: {wbe.raster_memory_bytes() / 1e6:.1f} MB")

# Explicit reset if needed
wbe.clear_memory()
print(f"After clear: {wbe.raster_memory_count()}")
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
