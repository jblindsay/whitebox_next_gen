# Output Controls

This chapter documents practical output controls for raster, vector, and lidar workflows in WbW-R.

Output configuration is where reproducibility is made explicit. Defaults are
useful during exploration, but production scripts should pin extensions and
format options so artifacts remain comparable across runs and environments.
Treat this chapter as the policy layer for how outputs are persisted.

## General Principles

- Start with defaults until explicit format constraints are required.
- Use explicit output extensions for reproducibility.
- Re-open outputs and validate metadata after writes.

## Raster Output Controls

Raster objects expose `write(...)` and `deep_copy(...)` with optional `options`.

Use explicit raster options when output layout and compression behavior must be
stable across environments.

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')

# Default write
r$write('dem_default.tif', overwrite = TRUE)

# Write with options list
r$write(
  'dem_cog.tif',
  overwrite = TRUE,
  options = list(
    compress = TRUE,
    strict_format_options = TRUE,
    geotiff = list(
      layout = 'cog',
      tile_size = 512,
      compression = 'deflate',
      bigtiff = FALSE
    )
  )
)
```

### Raster Write Option Reference

Raster write options are supplied as an R `list`. The current top-level keys
are:

- `compress`: logical convenience switch for GeoTIFF-family outputs.
- `strict_format_options`: logical validation switch.
- `geotiff`: nested list for GeoTIFF / BigTIFF / COG-specific controls.
- `jpeg2000`: nested list for JPEG2000 (`.jp2`) controls.

#### `compress`

`compress` is a convenience flag used for GeoTIFF-family outputs.

- `TRUE`: maps to GeoTIFF `deflate` compression.
- `FALSE`: maps to uncompressed GeoTIFF output.

Default behavior:

- `compress` is unset by default (not `TRUE` and not `FALSE`).
- For GeoTIFF-family outputs, an unset `compress` falls through to the
  GeoTIFF writer default compression, which is `deflate`.
- For non-GeoTIFF outputs, `compress` has no effect.

If `geotiff$compression` is also supplied, the explicit
`geotiff$compression` value takes precedence.

#### `strict_format_options`

`strict_format_options` must be `TRUE` or `FALSE`.

- `FALSE` (default): GeoTIFF-specific options are ignored when writing a
  non-GeoTIFF output; JPEG2000-specific options are ignored when writing a
  non-`.jp2` output.
- `TRUE`: GeoTIFF-specific options on a non-GeoTIFF output path, or
  JPEG2000-specific options on a non-`.jp2` output path, raise an error.

#### `jpeg2000`

The nested `jpeg2000` list supports these keys:

- `compression`: character scalar compression mode.
- `quality_db`: numeric quality target in dB used when
  `compression = 'lossy'`.
- `decomp_levels`: non-negative integer decode-resolution level hint
  (`0` to `255`).

Default values when keys are omitted:

- `compression`: `'lossy'`.
- `quality_db`: `35.0` when `compression = 'lossy'` and no explicit
  `quality_db` is provided.
- `decomp_levels`: writer default (unset).

Supported `jpeg2000$compression` values:

- `'lossless'`
- `'lossy'`

Notes:

- `quality_db` is optional; when omitted with `compression = 'lossy'`, the
  writer default quality is used.
- JPEG2000 color-space controls are not exposed in the R wrapper yet.

#### `geotiff`

The nested `geotiff` list supports these keys:

- `compression`: character scalar naming the compression codec.
- `bigtiff`: logical.
- `layout`: character scalar naming the layout.
- `rows_per_strip`: positive integer used when `layout = 'stripped'`.
- `tile_width`: positive integer used when `layout = 'tiled'`.
- `tile_height`: positive integer used when `layout = 'tiled'`.
- `tile_size`: positive integer shortcut for COG tile size, and also accepted
  as a shortcut for both tile width and tile height when `layout = 'tiled'`.
- `cog_tile_size`: positive integer alias for `tile_size` when
  `layout = 'cog'`.

Default values when keys are omitted:

- `compression`: `'deflate'`.
- `bigtiff`: `FALSE`.
- `layout`: `'standard'`.
- `rows_per_strip`: `1` when `layout = 'stripped'`.
- `tile_width`: `512` when `layout = 'tiled'`.
- `tile_height`: defaults to `tile_width` when `layout = 'tiled'`.
- `tile_size` / `cog_tile_size`: `512` when `layout = 'cog'`.

Supported `geotiff$compression` values:

- `'none'`
- `'off'`
- `'uncompressed'`
- `'deflate'`
- `'zip'`
- `'lzw'`
- `'packbits'`
- `'pack_bits'`
- `'jpeg'`
- `'webp'`
- `'web_p'`
- `'jpegxl'`
- `'jpeg_xl'`
- `'jxl'`

These are accepted aliases for the same underlying codecs:

- `'none'`, `'off'`, and `'uncompressed'`
- `'deflate'` and `'zip'`
- `'packbits'` and `'pack_bits'`
- `'webp'` and `'web_p'`
- `'jpegxl'`, `'jpeg_xl'`, and `'jxl'`

Supported `geotiff$layout` values:

- `'standard'`: default GeoTIFF writer behavior.
- `'stripped'`: strip-organized GeoTIFF.
- `'striped'`: alias for `'stripped'`.
- `'tiled'`: tiled GeoTIFF.
- `'cog'`: Cloud-Optimized GeoTIFF.

Layout-specific parameter behavior:

- `layout = 'standard'`: ignores strip/tile size keys.
- `layout = 'stripped'` or `'striped'`: uses `rows_per_strip`.
  Default is `1` if omitted.
- `layout = 'tiled'`: uses `tile_width` and `tile_height`.
  `tile_width` defaults to `512` if omitted.
  `tile_height` defaults to `tile_width`.
  `tile_size` is accepted as a shortcut for both dimensions.
- `layout = 'cog'`: uses `tile_size` or `cog_tile_size`.
  Default is `512` if neither is supplied.

#### Extensionless Raster Outputs

If you omit the output extension, raster writes default to `.tif`. In that
case the wrapper also defaults the layout to COG unless you explicitly specify
another layout.

```r
# Writes my_surface.tif and defaults to COG-style layout.
r$write('my_surface', overwrite = TRUE)
```

#### Practical Patterns

```r
# Explicit uncompressed standard GeoTIFF
r$write(
  'out_standard_uncompressed.tif',
  overwrite = TRUE,
  options = list(
    compress = FALSE,
    geotiff = list(layout = 'standard')
  )
)

# Stripped GeoTIFF with 64 rows per strip
r$write(
  'out_stripped.tif',
  overwrite = TRUE,
  options = list(
    geotiff = list(layout = 'stripped', rows_per_strip = 64)
  )
)

# Tiled GeoTIFF using a single tile_size shortcut
r$write(
  'out_tiled.tif',
  overwrite = TRUE,
  options = list(
    geotiff = list(layout = 'tiled', tile_size = 256)
  )
)

# COG with explicit codec and BigTIFF toggle
r$write(
  'out_cog.tif',
  overwrite = TRUE,
  options = list(
    strict_format_options = TRUE,
    geotiff = list(
      layout = 'cog',
      tile_size = 512,
      compression = 'deflate',
      bigtiff = FALSE
    )
  )
)
```

## Memory Lifecycle and Cleanup

For workflows that use memory-backed rasters, vectors, and lidar objects
(`file_mode = "m"`), explicit lifecycle management prevents unbounded memory
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

```r
library(whiteboxworkflows)

# Long-running batch analysis
for (tile_id in 1:1000) {
  cat('Processing tile', tile_id, '\n')
  
  # Read data into memory for this tile
  r <- wbw_read_raster(sprintf('tile_%d.tif', tile_id), file_mode = "m")
  v <- wbw_read_vector(sprintf('bounds_%d.gpkg', tile_id), file_mode = "m")
  
  # Process
  result <- wbw_clip_raster_by_polygon(input = r$file_path(), polygon = v$file_path(), 
                output = sprintf('clipped_%d.tif', tile_id))
  
  # Explicit cleanup before next iteration
  wbw_remove_raster_from_memory(r)
  wbw_remove_vector_from_memory(v)
}
```

### Monitoring memory usage

For production scripts, track memory explicitly:

```r
library(whiteboxworkflows)

cat('Initial raster memory:', wbw_raster_memory_bytes() / 1e6, 'MB\n')
cat('Initial vector memory:', wbw_vector_memory_bytes() / 1e6, 'MB\n')

# ... run operations ...

# Before returning or starting new phase
cat('Final raster count:', wbw_raster_memory_count(), '\n')
cat('Final raster memory:', wbw_raster_memory_bytes() / 1e6, 'MB\n')

# Explicit reset if needed
wbw_clear_memory()
cat('After clear:', wbw_raster_memory_count(), '\n')
```

## Lidar Output Controls

Lidar objects expose `write(...)` and `deep_copy(...)` with optional `options`.
The standalone `wbw_write_lidar()` function persists a `wbw_lidar` result
returned by a tool call directly to disk.

When a session method is called without an `output` argument the result is
stored in memory automatically; pass it to `wbw_write_lidar()` to persist it:

```r
library(whiteboxworkflows)

wbe <- wbw_make_session()
survey <- wbw_read_lidar('survey.laz')

# Omit output — result is memory-backed automatically
filtered <- wbe$filter_lidar_classes(input = survey$path, excluded_classes = list(7L))
cat(filtered$path, '\n')  # memory://lidar/...

# Persist when ready
wbw_write_lidar(filtered, 'survey_clean.copc.laz')
```

Write options for `wbw_write_lidar()` and `l$write()`:

Use these options to tune archive size, cloud-read behavior, and downstream
compatibility.

```r
library(whiteboxworkflows)

l <- wbw_read_lidar('survey.las')

# LAZ controls
l$write(
  'survey_out.laz',
  overwrite = TRUE,
  options = list(
    laz = list(
      chunk_size = 25000,
      compression_level = 7
    )
  )
)

# COPC controls
l$write(
  'survey_out.copc.laz',
  overwrite = TRUE,
  options = list(
    copc = list(
      max_points_per_node = 75000,
      max_depth = 8,
      node_point_ordering = 'hilbert'
    )
  )
)
```

### Lidar Write Option Reference

For lidar writes, the `options` list supports these top-level keys:

- `laz`: nested list with LAZ writer controls.
- `copc`: nested list with COPC hierarchy controls.

#### `laz`

Supported keys:

- `chunk_size`: positive integer (> 0).
- `compression_level`: integer in the range `0` to `9`.

Default values when keys are omitted:

- `chunk_size`: `50000`.
- `compression_level`: `6`.

#### `copc`

Supported keys:

- `max_points_per_node`: positive integer (> 0).
- `max_depth`: positive integer (> 0).
- `node_point_ordering`: one of:
  - `'auto'`
  - `'morton'`
  - `'hilbert'`

Default values when keys are omitted:

- `max_points_per_node`: `100000`.
- `max_depth`: `8`.
- `node_point_ordering`: `'auto'`.

Notes:

- If no output extension is provided, lidar writes default to `.copc.laz`.
- COPC options are relevant when writing COPC output (`.copc.laz` / `.copc.las`).

```r
# Extensionless output defaults to COPC
l$write(
  'survey_out',
  overwrite = TRUE,
  options = list(
    copc = list(
      max_points_per_node = 75000,
      max_depth = 8,
      node_point_ordering = 'hilbert'
    )
  )
)

# Explicit LAZ controls
l$write(
  'survey_out.laz',
  overwrite = TRUE,
  options = list(
    laz = list(
      chunk_size = 25000,
      compression_level = 7
    )
  )
)
```

## Vector Output Controls

`wbw_write_vector(...)` persists a `wbw_vector` object to disk. When a session
method is called without an `output` argument, the result is automatically
stored in memory and can be written to disk with `wbw_write_vector()`.

```r
library(whiteboxworkflows)

wbe <- wbw_make_session()
roads <- wbw_read_vector('roads.gpkg')

# Omit output — result is memory-backed automatically
centroids <- wbe$centroid_vector(input = roads$path)
cat(centroids$path, '\n')  # memory://vector/...

# Persist when ready
wbw_write_vector(centroids, 'roads_centroids.gpkg')

# Or supply output explicitly to write directly to disk
wbe$centroid_vector(input = roads$path, output = 'roads_centroids_direct.gpkg')
```

### Vector Write Option Reference

For vector writes, the `options` list supports these keys:

- `strict_format_options`: logical validation switch.
- `geoparquet`: nested list with GeoParquet writer controls.

#### `strict_format_options`

- `FALSE` (default): format-specific controls are ignored when they do not
  apply to the selected output format.
- `TRUE`: format-specific controls on incompatible output formats raise an
  error.

#### `geoparquet`

Supported keys:

- `max_rows_per_group`: positive integer.
- `data_page_size_limit`: positive integer.
- `write_batch_size`: positive integer.
- `data_page_row_count_limit`: positive integer.
- `compression`: character scalar.

Default values when keys are omitted:

- `max_rows_per_group`: `1_048_576`.
- `data_page_size_limit`: Parquet library default page size.
- `write_batch_size`: Parquet library default write batch size.
- `data_page_row_count_limit`: Parquet library default row-count limit.
- `compression`: Parquet library default compression codec.

Supported `geoparquet$compression` values:

- `'none'`
- `'snappy'`
- `'gzip'`
- `'lz4'`
- `'zstd'`
- `'brotli'`

Notes:

- GeoParquet controls are only applied for `.parquet` outputs.
- If no output extension is provided, vector writes default to `.gpkg`.

```r
wbw_write_vector(
  buffered,
  'roads.parquet',
  options = list(
    strict_format_options = TRUE,
    geoparquet = list(
      compression = 'zstd',
      max_rows_per_group = 250000,
      data_page_size_limit = 1048576,
      write_batch_size = 8192,
      data_page_row_count_limit = 20000
    )
  )
)
```

## Reproducibility Checklist

1. Pin output extension explicitly.
2. Capture option list values in scripts.
3. Verify metadata after write (`metadata()` and CRS values).
4. Keep source files immutable; write derived outputs separately.
