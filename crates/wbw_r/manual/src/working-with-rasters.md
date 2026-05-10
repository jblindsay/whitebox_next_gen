# Working with Rasters

This chapter documents practical raster workflows in WbW-R with emphasis on
inspection, iteration, modification, and persistence.

Raster processing usually alternates between backend tools for heavy
transformations and array-level edits for custom logic. The key design choice is
to use tool operations for scale and consistency, then reserve manual array
passes for domain-specific adjustments that are hard to express with existing
tools.

## Raster Lifecycle

This lifecycle makes assumptions explicit before computation and keeps your
workflow reviewable.

Typical lifecycle:
1. Read raster.
2. Inspect metadata.
3. Transform values (tool-driven or array-driven).
4. Persist outputs with explicit options when needed.

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
meta <- r$metadata()

print(meta$rows)
print(meta$columns)
print(meta$nodata)
```

## Memory-Backed Rasters for Pipeline Efficiency

For workflows that chain multiple tool operations, memory-backed rasters eliminate
disk I/O between steps. This is especially valuable when processing large rasters
in complex pipelines. Rasters remain in process memory, accessible to subsequent
tools without writing to disk.

Load a raster into memory with `file_mode = "m"`:

```r
library(whiteboxworkflows)

# Read directly into memory; no disk path required for subsequent ops
r1 <- wbw_read_raster('dem.tif', file_mode = "m")
r2 <- wbw_read_raster('slope.tif', file_mode = "m")

# Both rasters are now memory-backed. Chain operations without disk:
result <- r1 + r2
print(result$file_path())  # prints: memory://raster/...
```

Memory-backed paths are compatible with all downstream raster operations:

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif', file_mode = "m")

# Inspect and transform
meta <- r$metadata()
scaled <- wbw_multiply(input = r$file_path(), multiplier = 1.5)

# Export to disk when ready
wbw_write_raster(scaled, 'dem_scaled_1p5x.tif')
```

### Memory Lifecycle and Cleanup

Memory-backed rasters persist in the process store until explicitly removed or
cleared. For long-running jobs, manage memory explicitly to avoid accumulation:

```r
library(whiteboxworkflows)

# Check current memory usage
count_before <- wbw_raster_memory_count()
bytes_before <- wbw_raster_memory_bytes()
cat('Rasters in memory:', count_before, '\n')
cat('Bytes used:', bytes_before, '\n')

# Read two rasters
r1 <- wbw_read_raster('large1.tif', file_mode = "m")
r2 <- wbw_read_raster('large2.tif', file_mode = "m")

cat('After reads:', wbw_raster_memory_count(), '\n')

# Remove one raster when done
wbw_remove_raster_from_memory(r1)
cat('After remove:', wbw_raster_memory_count(), '\n')

# Or clear all rasters at once
wbw_clear_raster_memory()
cat('After clear:', wbw_raster_memory_count(), '\n')
```

Best practices:
- Use `file_mode = "m"` for intermediate results in tool chains.
- Export memory-backed rasters to disk with `write()` when persisting results.
- Call `remove_raster_from_memory()` after a raster is no longer needed.
- Use `clear_raster_memory()` between independent job phases.
- Use `wbw_clear_memory()` when resetting all in-process raster/vector/lidar stores together.
- Monitor `raster_memory_count()` and `raster_memory_bytes()` in large pipelines.

## Iterating Through Grid Cells

Use this only for logic that cannot be expressed through existing tools or
vectorized operations.

Use `to_array()` for cell-level logic.

```r
library(whiteboxworkflows)

r <- wbw_read_raster('dem.tif')
a <- r$to_array()
meta <- r$metadata()

nr <- dim(a)[1]
nc <- dim(a)[2]

for (row in seq_len(nr)) {
  for (col in seq_len(nc)) {
    z <- a[row, col]
    if (!is.na(z) && z != meta$nodata) {
      if (z < 0) {
        a[row, col] <- 0
      }
    }
  }
}
```

## Fast Random Cell Access with Pinning

For neighbourhood logic or flow-path traversal that performs frequent random
cell reads/writes, use pinned raster views. Pinning loads the raster values
once and avoids repeated conversion overhead inside tight loops.

Single-raster pinning:

```r
library(whiteboxworkflows)

r <- wbw_read_raster("dem.tif")
p <- wbw_pinned_raster(r)

v <- p$get_value(100, 200)
p$set_value(100, 200, v + 1)
p$close()
```

Two-raster read/write scan loop:

```r
library(whiteboxworkflows)

src <- wbw_read_raster("D8Pointer.tif")
dst <- wbw_read_raster("output_template.tif")

wbw_with_pinned_rasters(src, dst, .f = function(srcp, dstp) {
  meta <- src$metadata()
  for (row in seq_len(meta$rows)) {
    for (col in seq_len(meta$columns)) {
      value <- srcp$get_value(row, col)
      dstp$set_value(row, col, value)
    }
  }
})
```

Notes:
- `wbw_with_pinned_rasters()` closes all pins automatically.
- `wbw_pin_rasters(...)` is available when manual close control is preferred.
- Pinned write-back currently targets file-backed rasters in wbw_r.

## Writing Modified Data Back

This demonstrates creating a derived raster while preserving base geospatial
context.

```r
library(whiteboxworkflows)

base <- wbw_read_raster('dem.tif')
a <- base$to_array()
a <- a * 1.05

out <- wbw_array_to_raster(a, base, output_path = 'dem_scaled.tif')
print(out)
```

Supported raster write keys and valid values are documented in
[Output Controls](output-controls.md#raster-write-option-reference).

## Multi-Band Iteration

Use this when transform rules depend on band identity or per-band thresholds.

```r
library(whiteboxworkflows)

r <- wbw_read_raster('multiband.tif')
a <- r$to_array()

# If array is [rows, cols, bands], loop accordingly.
d <- dim(a)
if (length(d) == 3) {
  nr <- d[1]
  nc <- d[2]
  nb <- d[3]

  for (b in seq_len(nb)) {
    for (row in seq_len(nr)) {
      for (col in seq_len(nc)) {
        v <- a[row, col, b]
        if (!is.na(v) && v < 0) {
          a[row, col, b] <- 0
        }
      }
    }
  }
}

wbw_array_to_raster(a, r, output_path = 'multiband_clamped.tif')
```

## Tool-First Raster Processing

Prefer this pattern for heavy processing: let optimized tools do most work,
then apply targeted custom edits.

```r
library(whiteboxworkflows)

s <- wbw_session()
dem <- wbw_read_raster('dem.tif')

wbw_fill_depressions(dem = dem$file_path(), output = 'filled.tif')
filled <- wbw_read_raster('filled.tif')

slope <- filled$square()  # placeholder unary op example
slope$write('slope_out.tif', overwrite = TRUE)
```

## NoData and Performance Guidance

- Always account for `metadata()$nodata` in per-cell loops.
- Prefer tool operations for heavy transforms.
- Use array loops only for custom logic not available as tools.

## Raster Object Method Reference

### Metadata and Conversion

| Method | Description |
|---|---|
| `metadata` | Return raster metadata (dimensions, extent, nodata, CRS fields). |
| `file_path` | Return the backing raster path. |
| `band_count` | Return raster band count. |
| `active_band` | Return active band index. |
| `get_value`, `set_value` | Read/write single cell values using 1-based row/column/band indices. |
| `crs_epsg`, `crs_wkt` | Inspect CRS metadata. |
| `to_array` | Convert raster to in-memory array for custom processing. |
| `to_stars` | Convert raster to a `stars` object (requires `terra` support path). |
| `pin` | Create a pinned view for low-overhead random cell access in tight loops. |

### Arithmetic and Unary Transform Methods

| Method | Description |
|---|---|
| `add`, `subtract`, `multiply`, `divide` | Cellwise binary arithmetic with another raster/path operand. |
| `abs`, `ceil`, `floor`, `round`, `square`, `sqrt` | Common unary numeric transforms. |
| `log10`, `log2`, `exp`, `exp2` | Log and exponential transforms. |
| `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh` | Trigonometric and hyperbolic transforms. |

### Persistence

| Method | Description |
|---|---|
| `deep_copy` | Copy raster to a new output path and return a raster object. |
| `write` | Write raster to disk with optional output options. |
