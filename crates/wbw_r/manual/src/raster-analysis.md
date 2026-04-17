# Raster Analysis

Raster analysis in WbW-R covers the end-to-end workflow of reading, transforming, and writing gridded data — from simple cell-value arithmetic through multi-layer overlays, proximity operations, interpolation, and statistical testing. All heavy computation runs in the Whitebox backend.

---

## Core Concepts

Raster analysis requires understanding these fundamental terms:

- **Cell (pixel)**: The smallest unit of a raster. Each cell stores a single value (integer or floating-point) and has a uniform spatial extent (e.g. 10 m × 10 m).
- **Data type**: Integer (Int8, Int16, Int32) for categorical data; Float32 or Float64 for continuous measurements. Data type affects precision, file size, and computation speed.
- **NoData value**: Sentinel value representing missing or invalid data (e.g. -9999 or NaN). Critical for masking water, clouds, or invalid measurements in focal operations.
- **Spatial reference (CRS)**: Coordinate system and projection. Mismatched CRS between rasters causes silent misalignment; always verify before overlay operations.
- **Extent**: The bounding box (xmin, ymin, xmax, ymax) of the raster in real-world coordinates.
- **Cell size (resolution)**: Cell width and height in map units. Coarser resolution is faster but loses detail; finer resolution requires more memory and computation.
- **Focal operation**: Uses neighbourhood values (e.g. 3×3 kernel) to compute output. Examples: moving average, Sobel edge detection, local extrema.
- **Zonal operation**: Aggregates grid values by zone (polygon or categorical layer). Examples: mean by land-cover class, sum by administrative boundary.
- **Reclassification**: Reassigning cell values according to lookup rules. Common for categorizing continuous data (e.g. slope classes) or remapping land-cover codes.
- **Resampling**: Changing cell size or alignment. Methods: nearest-neighbour (preserves categories), bilinear (smooth), cubic (smoother).

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/raster')
```

---

## Reading and Inspecting Rasters

```r
r <- wbw_read_raster('input.tif')
meta <- r$metadata()

cat('Rows:', meta$rows, '  Cols:', meta$columns, '\n')
cat('Cell size:', meta$resolution_x, 'x', meta$resolution_y, '\n')
cat('EPSG:', meta$epsg, '\n')
cat('NoData:', meta$nodata, '\n')
cat('Data type:', meta$data_type, '\n')
```

---

## Raster Calculator

`raster_calculator` evaluates an algebraic expression that combines one or more raster inputs:

```r
# Single-raster expression — multiply by constant
wbw_run_tool('raster_calculator', args = list(
  statement = "'elevation.tif' * 3.28084",
  output    = 'elevation_ft.tif'
), session = s)

# Multi-raster NDVI expression
wbw_run_tool('raster_calculator', args = list(
  statement = "('nir.tif' - 'red.tif') / ('nir.tif' + 'red.tif')",
  output    = 'ndvi.tif'
), session = s)

# Conditional expression using isnull() and nodata()
wbw_run_tool('raster_calculator', args = list(
  statement = "if(isnull('input.tif'), nodata(), 'input.tif' + 100.0)",
  output    = 'result.tif'
), session = s)
```

Special tokens available in statements: `nodata()`, `isnull()`, `if()`, `abs()`, `sqrt()`, `log()`, `log2()`, `exp()`, `min()`, `max()`, `pi`, integer constants, and floating-point constants.

---

## Reclassification

```r
# Reclassify using from-to-becomes triplets
# Format: "from;to;new;from;to;new;..."
wbw_run_tool('reclass', args = list(
  i         = 'slope.tif',
  output    = 'slope_class.tif',
  reclass_vals = '0;5;1;5;15;2;15;30;3;30;45;4;45;90;5',
  assign_mode = FALSE
), session = s)

# Equal-interval reclassification
wbw_run_tool('reclass_equal_interval', args = list(
  i         = 'ndvi.tif',
  output    = 'ndvi_class.tif',
  interval  = 0.1,
  start_val = -1.0,
  end_val   = 1.0
), session = s)

# Reclassify from a CSV lookup table
wbw_run_tool('reclass_from_file', args = list(
  i          = 'landcover.tif',
  reclass_file = 'reclass_table.txt',
  output     = 'landcover_reclassed.tif'
), session = s)
```

---

## Focal Statistics (Moving Windows)

```r
# Gaussian filter
wbw_run_tool('gaussian_filter', args = list(
  i = 'dem.tif', output = 'dem_gauss.tif', sigma = 1.5), session = s)

# Median filter (feature-preserving)
wbw_run_tool('median_filter', args = list(
  i = 'dem.tif', output = 'dem_median.tif', filterx = 5, filtery = 5,
  sig_digits = 2), session = s)

# Feature-preserving smoothing (Zhang et al.)
wbw_run_tool('feature_preserving_smoothing', args = list(
  dem = 'dem.tif', output = 'dem_fps.tif', filter = 11,
  norm_diff = 8.0, num_iter = 3, max_diff = 0.5, zfactor = 1.0), session = s)

# Standard deviation in a window
wbw_run_tool('standard_dev_filter', args = list(
  i = 'dem.tif', output = 'dem_sd.tif', filterx = 11, filtery = 11), session = s)

# Percentile filter
wbw_run_tool('percentile_filter', args = list(
  i = 'dem.tif', output = 'dem_pct75.tif', filterx = 11, filtery = 11,
  sig_digits = 2), session = s)
```

---

## Morphological Operations

```r
# Morphological closing (fills gaps in foreground)
wbw_run_tool('closing', args = list(
  i = 'binary.tif', output = 'binary_close.tif', filterx = 3, filtery = 3), session = s)

# Morphological opening (removes small foreground blobs)
wbw_run_tool('opening', args = list(
  i = 'binary.tif', output = 'binary_open.tif', filterx = 3, filtery = 3), session = s)

# Top-hat transform (white)
wbw_run_tool('tophat_transform', args = list(
  i = 'raster.tif', output = 'tophat.tif', filterx = 11, filtery = 11,
  variant = 'white'), session = s)
```

---

## Global and Zonal Statistics

```r
# Global statistics (summary of entire raster)
wbw_run_tool('raster_histogram', args = list(
  i = 'dem.tif', output = 'dem_histogram.html'), session = s)

# Zonal statistics — mean elevation per watershed zone
wbw_run_tool('zonal_statistics', args = list(
  i         = 'dem.tif',
  features  = 'watersheds.tif',
  output    = 'watershed_stats.html',
  stat      = 'mean',
  out_raster = 'watershed_mean_elev.tif'
), session = s)
```

---

## Raster Overlay Operations

```r
# Weighted sum (multi-criteria suitability)
wbw_run_tool('weighted_sum', args = list(
  inputs  = 'soil.tif;slope.tif;distance.tif',
  weights = '0.3;0.5;0.2',
  output  = 'suitability.tif'
), session = s)

# Weighted overlay (MCE) with constraint
wbw_run_tool('weighted_overlay', args = list(
  inputs      = 'factor1.tif;factor2.tif;factor3.tif',
  weights     = '0.4;0.4;0.2',
  output      = 'suitability_mce.tif',
  scale_max   = 5.0,
  scale_min   = 0.0,
  scale_factor  = 1.0
), session = s)
```

---

## Resampling and Aggregation

```r
wbw_run_tool('resample', args = list(
  inputs      = 'dem.tif',
  output      = 'dem_10m.tif',
  cell_size   = 10.0,
  method      = 'bilinear'
), session = s)

wbw_run_tool('aggregate_raster', args = list(
  i = 'dem.tif', output = 'dem_agg.tif', agg_factor = 5,
  type = 'mean'), session = s)
```

---

## Proximity Analysis

```r
# Euclidean distance
wbw_run_tool('euclidean_distance', args = list(
  i = 'sources.tif', output = 'euclidean_dist.tif'), session = s)

# Cost-distance accumulation
wbw_run_tool('cost_distance', args = list(
  source = 'sources.tif',
  cost   = 'friction.tif',
  out_accum = 'cost_accum.tif',
  out_backlink = 'cost_backlink.tif'
), session = s)

# Least-cost path
wbw_run_tool('cost_pathway', args = list(
  destination = 'destinations.tif',
  backlink    = 'cost_backlink.tif',
  output      = 'least_cost_path.tif',
  zero_background = FALSE
), session = s)

# Raster buffer
wbw_run_tool('buffer_raster', args = list(
  i = 'features.tif', output = 'buffered.tif',
  size = 250.0, gridcells = FALSE), session = s)
```

---

## Raster Object Analysis

```r
# Label connected patches (foreground = non-zero)
wbw_run_tool('clump', args = list(
  i = 'binary.tif', output = 'patches.tif',
  diag = TRUE, zero_back = TRUE), session = s)

# Remove small patches below area threshold (10 000 m²)
wbw_run_tool('remove_spurs', args = list(
  i = 'patches.tif', output = 'patches_clean.tif',
  iterations = 10), session = s)

# Raster area of each patch value
wbw_run_tool('raster_area', args = list(
  i = 'patches.tif', output = 'patch_area.tif',
  out_text = FALSE, units = 'map units', zero_back = TRUE), session = s)
```

---

## Interpolation from Points

```r
pts <- wbw_read_vector('sample_points.shp')

# IDW
wbw_run_tool('idw_interpolation', args = list(
  i         = pts$file_path(),
  field     = 'ELEV',
  output    = 'idw_surf.tif',
  use_z     = FALSE,
  weight    = 2.0,
  radius    = 2.5,
  min_points = 2,
  cell_size  = 5.0
), session = s)

# Natural Neighbour
wbw_run_tool('natural_neighbour_interpolation', args = list(
  i        = pts$file_path(),
  field    = 'ELEV',
  output   = 'nn_surf.tif',
  use_z    = FALSE,
  cell_size = 5.0
), session = s)

# Radial Basis Function
wbw_run_tool('radial_basis_function_interpolation', args = list(
  i         = pts$file_path(),
  field     = 'ELEV',
  output    = 'rbf_surf.tif',
  num_points = 8,
  cell_size  = 5.0,
  func_type  = 'ThinPlateSpline',
  poly_order = 1,
  weight     = 0.1
), session = s)
```

---

## Statistical Tests

```r
# Kolmogorov-Smirnov normality test
wbw_run_tool('ks_test_for_normality', args = list(
  i = 'dem.tif', output = 'ks_normality.html'), session = s)

# Two-raster paired samples t-test
wbw_run_tool('paired_sample_t_test', args = list(
  input1 = 'dem_2010.tif', input2 = 'dem_2020.tif',
  output = 'ttest.html', num_samples = 1000), session = s)
```

---

## Contour Generation

```r
wbw_run_tool('contours_from_raster', args = list(
  i          = 'dem.tif',
  output     = 'contours.shp',
  interval   = 10.0,
  base       = 0.0,
  smooth     = 5,
  tolerance  = 10.0
), session = s)
```

---

## WbW-Pro Spotlight: Field Trafficability and Operation Planning

- **Problem:** Plan equipment timing and field access under variable moisture
  and weather conditions.
- **Tool:** `field_trafficability_and_operation_planning`
- **Typical inputs:** DEM, normalized soil-moisture raster, optional
  rainfall-risk raster.
- **Typical outputs:** Trafficability score raster, operation-class raster,
  and summary outputs.

```r
result <- s$field_trafficability_and_operation_planning(
  dem               = 'field_dem.tif',
  soil_moisture     = 'soil_moisture_norm.tif',
  rainfall_forecast = 'rainfall_risk_norm.tif',
  output_prefix     = 'field_12_trafficability'
)

print(result)
```

> **Note:** This workflow requires a session initialized with a valid Pro
> licence.

---

## Complete Raster Analysis Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/raster_workflow')

dem <- wbw_read_raster('dem.tif')

# 1. Smooth DEM
wbw_run_tool('feature_preserving_smoothing', args = list(
  dem = dem$file_path(), output = 'dem_smooth.tif', filter = 11,
  norm_diff = 8.0, num_iter = 3, max_diff = 0.5), session = s)

# 2. Slope
wbw_run_tool('slope', args = list(
  dem = 'dem_smooth.tif', output = 'slope.tif', units = 'degrees'), session = s)

# 3. Reclassify slope into erosion risk classes
wbw_run_tool('reclass', args = list(
  i = 'slope.tif', output = 'slope_risk.tif',
  reclass_vals = '0;5;1;5;15;2;15;30;3;30;90;4'), session = s)

# 4. Euclidean distance to water
wbw_run_tool('euclidean_distance', args = list(
  i = 'water_bodies.tif', output = 'dist_water.tif'), session = s)

# 5. Multi-criteria suitability overlay
wbw_run_tool('weighted_sum', args = list(
  inputs  = paste('slope_risk.tif', 'dist_water.tif', 'soil_type.tif', sep=';'),
  weights = '0.5;0.3;0.2',
  output  = 'suitability.tif'
), session = s)

# 6. Reclassify to binary mask (suitability > threshold)
wbw_run_tool('raster_calculator', args = list(
  statement = "if('suitability.tif' >= 3.0, 1.0, 0.0)",
  output    = 'suitable_areas.tif'
), session = s)

# 7. Generate contours
wbw_run_tool('contours_from_raster', args = list(
  i = dem$file_path(), output = 'contours_10m.shp',
  interval = 10.0, base = 0.0, smooth = 3), session = s)

cat('Raster analysis complete.\n')
```

---

## Tips

- **Choose your data type**: Use integers for categorical data (land cover, classifications) to minimize file size and computation time. Use floating-point (Float32 or Float64) only for continuous measurements (elevation, temperature, probability).
- **Set NoData explicitly**: Ensure your source rasters carry a valid NoData value. Missing NoData declarations can corrupt statistics and focal operations by including invalid pixels as zeros or false elevations.
- **Compress carefully**: LZW and DEFLATE compression work well for most data; avoid if you need rapid random access to interior tiles. Use COMPRESS=JPEG for photographic data only (lossy, unsuitable for analysis).
- **Focal operations require buffering**: Cells at raster edges cannot compute full neighbourhoods. Use `expand()` or accept edge effects; never assume borders are valid in derivative rasters.
- **Zonal statistics are only as good as your zones**: Ensure zone boundaries are topologically clean (no overlaps, no gaps). Overlapping zones cause double-counting; gaps cause NoData regions in output.
- **Reclassification is fast but risky**: Always validate output distributions (histogram) after reclassification. Off-by-one errors in class boundaries can silently produce wrong land-cover or suitability classes.
- **Memory is the constraint for large rasters**: Tiles > 2 GB require out-of-core or streaming processing. Use `read_by_block()` for large files; avoid loading entire rasters into memory if they exceed available RAM.
- **Upsampling introduces artifacts**: Never upsample (finer resolution) without a valid interpolation method. Nearest-neighbour upsampling creates blocky artefacts; bilinear is smoother but may violate data range (e.g. probability values > 1.0).
```
