# Output Controls

This chapter documents practical output controls for raster, vector, and lidar workflows in WbW-R.

## General Principles

- Start with defaults until explicit format constraints are required.
- Use explicit output extensions for reproducibility.
- Re-open outputs and validate metadata after writes.

## Raster Output Controls

Raster objects expose `write(...)` and `deep_copy(...)` with optional `options`.

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

## Lidar Output Controls

Lidar objects expose `write(...)` and `deep_copy(...)` with optional `options`.

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
