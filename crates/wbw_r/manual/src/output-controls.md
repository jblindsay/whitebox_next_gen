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
  result <- wbw_run_tool('clip_raster_by_polygon',
    args = list(input = r$file_path(), polygon = v$file_path(), 
                output = sprintf('clipped_%d.tif', tile_id))
  )
  
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
wbw_clear_raster_memory()
cat('After clear:', wbw_raster_memory_count(), '\n')
```

## Lidar Output Controls

Lidar objects expose `write(...)` and `deep_copy(...)` with optional `options`.

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

## Vector Output Pattern

This reflects the current R facade emphasis on tool-driven vector persistence.

Vector persistence is commonly tool-driven in current R workflows:

```r
library(whiteboxworkflows)

s <- wbw_session()
wbw_run_tool(
  'buffer_vector',
  args = list(input = 'roads.gpkg', output = 'roads_buffer.gpkg', distance = 10.0),
  session = s
)
```

## Reproducibility Checklist

1. Pin output extension explicitly.
2. Capture option list values in scripts.
3. Verify metadata after write (`metadata()` and CRS values).
4. Keep source files immutable; write derived outputs separately.
