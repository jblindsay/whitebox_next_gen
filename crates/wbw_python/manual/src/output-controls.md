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

### Raster Write Option Reference

The raster `options` dictionary currently supports these top-level keys:

- `compress`: boolean convenience switch for GeoTIFF-family outputs.
- `strict_format_options`: boolean validation switch.
- `geotiff`: nested dictionary for GeoTIFF / BigTIFF / COG-specific controls.
- `jpeg2000`: nested dictionary for JPEG2000 (`.jp2`) controls.

#### `compress`

`compress` is a convenience flag. It applies only to GeoTIFF-family outputs.

- `True`: maps to GeoTIFF `deflate` compression.
- `False`: maps to uncompressed GeoTIFF output.

Default behavior:

- `compress` is unset by default (not `True` and not `False`).
- For GeoTIFF-family outputs, an unset `compress` falls through to the
    GeoTIFF writer default compression, which is `deflate`.
- For non-GeoTIFF outputs, `compress` has no effect.

If `geotiff.compression` is also supplied, the explicit
`geotiff.compression` value takes precedence over `compress`.

#### `strict_format_options`

`strict_format_options` accepts `True` or `False`.

- `False` (default): GeoTIFF-specific options are ignored when writing a
    non-GeoTIFF output such as `.jp2`, `.gpkg`, or `.zarr`; JPEG2000-specific
    options are ignored when writing non-`.jp2` outputs.
- `True`: using GeoTIFF-specific options with a non-GeoTIFF output path, or
    JPEG2000-specific options with a non-`.jp2` output path, raises an error.

#### `jpeg2000`

The `jpeg2000` dictionary supports these keys:

- `compression`: string compression mode.
- `quality_db`: numeric quality target in dB used when
    `compression='lossy'`.
- `decomp_levels`: non-negative integer decode-resolution level hint
    (`0` to `255`).
- `color_space`: string color-space override.

Default values when keys are omitted:

- `compression`: `lossy`.
- `quality_db`: `35.0` when `compression='lossy'` and no explicit
    `quality_db` is provided.
- `decomp_levels`: writer default (unset).
- `color_space`: writer default (unset); inferred from band layout when
    possible.

Supported `jpeg2000.compression` values:

- `lossless`
- `lossy`

Supported `jpeg2000.color_space` values:

- `greyscale` (aliases: `grayscale`, `gray`, `grey`)
- `srgb` (alias: `rgb`)
- `ycbcr` (alias: `ycb`)
- `multiband` (aliases: `multi_band`, `multi`)

Notes:

- `quality_db` is optional; when omitted with `compression='lossy'`, the
    writer default quality is used.

#### `geotiff`

The `geotiff` dictionary supports these keys:

- `compression`: string compression codec.
- `bigtiff`: boolean.
- `layout`: string layout name.
- `rows_per_strip`: positive integer used when `layout='stripped'`.
- `tile_width`: positive integer used when `layout='tiled'`.
- `tile_height`: positive integer used when `layout='tiled'`.
- `tile_size`: positive integer shortcut used by `layout='cog'`, and also
    accepted as a shortcut for both tile width and tile height when
    `layout='tiled'`.
- `cog_tile_size`: positive integer alias for `tile_size` when `layout='cog'`.

Default values when keys are omitted:

- `compression`: `deflate`.
- `bigtiff`: `False`.
- `layout`: `standard`.
- `rows_per_strip`: `1` when `layout='stripped'`.
- `tile_width`: `512` when `layout='tiled'`.
- `tile_height`: defaults to `tile_width` when `layout='tiled'`.
- `tile_size` / `cog_tile_size`: `512` when `layout='cog'`.

Supported `geotiff.compression` values:

- `none`
- `off`
- `uncompressed`
- `deflate`
- `zip`
- `lzw`
- `packbits`
- `pack_bits`
- `jpeg`
- `webp`
- `web_p`
- `jpegxl`
- `jpeg_xl`
- `jxl`

These names are aliases for the same underlying codecs:

- `none`, `off`, and `uncompressed`
- `deflate` and `zip`
- `packbits` and `pack_bits`
- `webp` and `web_p`
- `jpegxl`, `jpeg_xl`, and `jxl`

Supported `geotiff.layout` values:

- `standard`: default GeoTIFF writer behavior.
- `stripped`: strip-organized GeoTIFF.
- `striped`: alias for `stripped`.
- `tiled`: tiled GeoTIFF.
- `cog`: Cloud-Optimized GeoTIFF.

Layout-specific parameter behavior:

- `layout='standard'`: ignores strip/tile size keys.
- `layout='stripped'` or `layout='striped'`: uses `rows_per_strip`.
    Default is `1` if omitted.
- `layout='tiled'`: uses `tile_width` and `tile_height`.
    If omitted, `tile_width` defaults to `512`.
    `tile_height` defaults to `tile_width`.
    If `tile_size` is supplied, it is accepted as a shortcut for both dimensions.
- `layout='cog'`: uses `tile_size` or `cog_tile_size`.
    Default is `512` if neither is supplied.

#### Extensionless Raster Outputs

If you omit the output extension, raster writes default to `.tif`. In that
case the wrapper also defaults the GeoTIFF layout to COG unless you explicitly
set a different layout.

```python
# Writes my_surface.tif and defaults to COG-style layout.
wbe.write_raster(r, 'my_surface')
```

#### Practical Patterns

```python
# Explicit uncompressed standard GeoTIFF
wbe.write_raster(r, 'out_standard_uncompressed.tif', options={
        'compress': False,
        'geotiff': {'layout': 'standard'},
})

# Stripped GeoTIFF with 64 rows per strip
wbe.write_raster(r, 'out_stripped.tif', options={
        'geotiff': {'layout': 'stripped', 'rows_per_strip': 64},
})

# Tiled GeoTIFF using a single tile_size shortcut
wbe.write_raster(r, 'out_tiled.tif', options={
        'geotiff': {'layout': 'tiled', 'tile_size': 256},
})

# COG with explicit codec and BigTIFF toggle
wbe.write_raster(r, 'out_cog.tif', options={
        'strict_format_options': True,
        'geotiff': {
                'layout': 'cog',
                'tile_size': 512,
                'compression': 'deflate',
                'bigtiff': False,
        },
})
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

### Vector Write Option Reference

For vector writes, the `options` dictionary supports these keys:

- `strict_format_options`: boolean validation switch.
- `geoparquet`: nested dictionary used for GeoParquet-specific controls.

#### `strict_format_options`

- `False` (default): format-specific options are ignored when they do not apply
    to the selected output format.
- `True`: format-specific options on an incompatible output path raise an
    error.

#### `geoparquet`

The `geoparquet` dictionary supports:

- `max_rows_per_group`: positive integer.
- `data_page_size_limit`: positive integer.
- `write_batch_size`: positive integer.
- `data_page_row_count_limit`: positive integer.
- `compression`: string codec name.

Default values when keys are omitted:

- `max_rows_per_group`: `1_048_576`.
- `data_page_size_limit`: Parquet library default page size.
- `write_batch_size`: Parquet library default write batch size.
- `data_page_row_count_limit`: Parquet library default row-count limit.
- `compression`: Parquet library default compression codec.

Supported `geoparquet.compression` values:

- `none`
- `snappy`
- `gzip`
- `lz4`
- `zstd`
- `brotli`

Notes:

- GeoParquet controls are only applied for `.parquet` outputs.
- If no output extension is provided, vector writes default to `.gpkg`.

```python
# GeoParquet with explicit row-group and compression controls
wbe.write_vector(v, 'roads.parquet', options={
        'strict_format_options': True,
        'geoparquet': {
                'compression': 'zstd',
                'max_rows_per_group': 250000,
                'data_page_size_limit': 1048576,
                'write_batch_size': 8192,
                'data_page_row_count_limit': 20000,
        },
})
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

### Lidar Write Option Reference

For lidar writes, the `options` dictionary supports these top-level keys:

- `laz`: nested dictionary with LAZ writer controls.
- `copc`: nested dictionary with COPC hierarchy controls.

#### `laz`

Supported keys:

- `chunk_size`: positive integer.
- `compression_level`: integer compression level.

Default values when keys are omitted:

- `chunk_size`: `50000`.
- `compression_level`: `6`.

`compression_level` is typically used in the 0-9 range for LAZ workflows.

#### `copc`

Supported keys:

- `max_points_per_node`: positive integer.
- `max_depth`: positive integer.
- `node_point_ordering`: string ordering mode.

Default values when keys are omitted:

- `max_points_per_node`: `100000`.
- `max_depth`: `8`.
- `node_point_ordering`: `auto`.

Supported `copc.node_point_ordering` values:

- `auto`
- `morton`
- `hilbert`

Notes:

- If no output extension is provided, lidar writes default to `.copc.laz`.
- COPC options are relevant when output format is COPC (`.copc.laz` or
  `.copc.las`).

```python
# Extensionless output defaults to COPC
wbe.write_lidar(l, 'survey_out', options={
    'copc': {
        'max_points_per_node': 75000,
        'max_depth': 8,
        'node_point_ordering': 'hilbert',
    },
})

# Explicit LAZ controls
wbe.write_lidar(l, 'survey_out.laz', options={
    'laz': {
        'chunk_size': 25000,
        'compression_level': 7,
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
