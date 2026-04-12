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

wbw_run_tool('fill_depressions', args = list(dem = dem$file_path(), output = 'filled.tif'), session = s)
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
| `crs_epsg`, `crs_wkt` | Inspect CRS metadata. |
| `to_array` | Convert raster to in-memory array for custom processing. |
| `to_stars` | Convert raster to a `stars` object (requires `terra` support path). |

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
