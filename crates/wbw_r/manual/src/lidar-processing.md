# LiDAR Processing

LiDAR point cloud processing in WbW-R covers the full pipeline from raw flight-line data through to classified point clouds and derived raster products. All processing tools run in the Whitebox backend; R handles session management, file discovery, and result validation.

---

## Core Concepts

Before processing LiDAR data, understand these foundational terms:

- **Return number**: Which reflection from a single laser pulse. Pulse 1 is the first (often canopy top); pulse 2–5 capture midstory and ground returns.
- **Point classification**: ASPRS standard categories — ground (2), low veg (3), medium veg (4), high veg (5), buildings (6), noise (7), overlap (12), and others.
- **Intensity**: Reflectance value (0–65535) proportional to target brightness. Useful for vegetation density estimation and water detection.
- **Ground filtering**: Separating terrain points (classification 2) from vegetation and buildings; critical for accurate digital terrain models (DTMs).
- **Digital terrain model (DTM)**: Raster surface of bare earth, computed from ground returns only. Used for hydrology, geomorphometry, and flood modelling.
- **Digital surface model (DSM)**: Raster surface of highest returns (canopy top). Used for building detection and volume calculations.
- **Canopy height model (CHM)**: DSM minus DTM; represents vegetation height above ground. Standard input for tree detection and segmentation.
- **Point density**: Points per square unit (typically points/m²). Higher density enables finer segmentation; lower density requires smoothing.
- **Normalization**: Converting raw Z-values to height-above-ground by subtracting DTM, creating a normalized point cloud for structural analysis.

---

## Session Setup and File Discovery

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/lidar')

# List all LAS/LAZ files in the project folder
las_files <- list.files('.', pattern = '\\.(las|laz)$', full.names = TRUE,
                         recursive = TRUE)
cat('Found', length(las_files), 'point cloud files.\n')
```

---

## Reading and Inspecting LiDAR Files

```r
las <- wbw_read_lidar('survey.las')
meta <- las$metadata()

cat('Point count:', meta$number_of_points, '\n')
cat('X range:', meta$min_x, 'to', meta$max_x, '\n')
cat('Y range:', meta$min_y, 'to', meta$max_y, '\n')
cat('Z range:', meta$min_z, 'to', meta$max_z, '\n')
cat('Point density per m²:', meta$point_density, '\n')
```

---

## Point Cloud Filtering and Outlier Removal

```r
# Remove isolated low and high points
wbw_run_tool('lidar_elevation_slice', args = list(
  i       = 'survey.las',
  output  = 'survey_sliced.las',
  minz    = 0.0,
  maxz    = 200.0,
  cls     = FALSE
), session = s)

# Statistical outlier removal
wbw_run_tool('lidar_remove_outliers', args = list(
  i       = 'survey.las',
  output  = 'survey_clean.las',
  radius  = 2.0,
  elev_diff = 25.0,
  use_median = FALSE
), session = s)
```

---

## Ground Point Classification

### Progressive Morphological Filter

```r
wbw_run_tool('lidar_ground_point_filter', args = list(
  i        = 'survey_clean.las',
  output   = 'survey_classified.las',
  radius   = 2.0,
  min_elev_diff = 0.15,
  max_elev_diff = 1.3,
  num_iter = 5,
  threshold = 0.15,
  slope_threshold = 0.15,
  height_threshold = 1.0,
  classify  = TRUE,
  slope       = FALSE,
  height_diff = TRUE,
  filter_return_all = FALSE
), session = s)
```

---

## Digital Elevation Model Interpolation

### Tin Gridding (TIN Interpolation)

```r
wbw_run_tool('tin_gridding', args = list(
  i          = 'survey_classified.las',
  output     = 'dtm.tif',
  returns    = 'last',
  resolution = 1.0,
  exclude_cls = '1,3,4,5,6,7',
  minz       = -50.0,
  maxz       = 250.0
), session = s)
```

### LiDAR IDW Interpolation

```r
wbw_run_tool('lidar_idw_interpolation', args = list(
  i          = 'survey_classified.las',
  output     = 'dtm_idw.tif',
  parameter  = 'elevation',
  returns    = 'last',
  resolution = 1.0,
  weight     = 1.0,
  radius     = 2.5,
  exclude_cls = '1,3,4,5,6,7'
), session = s)
```

### Normalised Height Above Ground

```r
wbw_run_tool('normalize_lidar', args = list(
  i          = 'survey_classified.las',
  ground     = 'survey_classified.las',
  output     = 'survey_normalised.las',
  ignore_ground_distance = FALSE
), session = s)
```

---

## Canopy Height Model (CHM) and DSM

```r
# First return DSM
wbw_run_tool('lidar_idw_interpolation', args = list(
  i          = 'survey_classified.las',
  output     = 'dsm.tif',
  parameter  = 'elevation',
  returns    = 'first',
  resolution = 1.0,
  weight     = 1.0,
  radius     = 2.5,
  exclude_cls = '7'
), session = s)

# Canopy Height Model = DSM - DTM
wbw_run_tool('subtract', args = list(
  input1 = 'dsm.tif',
  input2 = 'dtm.tif',
  output = 'chm.tif'
), session = s)
```

---

## Point Density and Distribution Analysis

```r
wbw_run_tool('lidar_point_stats', args = list(
  i          = 'survey.las',
  resolution = 1.0,
  num_points = TRUE,
  num_pulses = FALSE
), session = s)

wbw_run_tool('lidar_density', args = list(
  i          = 'survey.las',
  output     = 'density.tif',
  resolution = 1.0,
  returns    = 'all',
  exclude_cls = '7'
), session = s)
```

---

## Scan-Angle and Return Analysis

```r
# Filter by scan angle
wbw_run_tool('filter_lidar_scan_angles', args = list(
  i         = 'survey.las',
  output    = 'survey_nadir.las',
  threshold = 15.0
), session = s)

# Intensity image
wbw_run_tool('lidar_idw_interpolation', args = list(
  i          = 'survey_nadir.las',
  output     = 'intensity.tif',
  parameter  = 'intensity',
  returns    = 'all',
  resolution = 1.0,
  weight     = 1.0,
  radius     = 2.5
), session = s)
```

---

## Tile Management

Working with large surveys requires tiling:

```r
# Tile large dataset
wbw_run_tool('lidar_tile', args = list(
  i        = 'full_survey.las',
  width    = 500.0,
  height   = 500.0,
  origin_x = 0.0,
  origin_y = 0.0
), session = s)

# Add a buffer overlap to each tile
wbw_run_tool('lidar_tile_footprint', args = list(
  i      = 'tile_000_000.las',
  output = 'tile_000_000_footprint.shp',
  hull   = FALSE
), session = s)
```

---

## Segmentation and Vegetation Analysis

```r
# Individual tree segmentation from normalised LiDAR
wbw_run_tool('individual_tree_detection', args = list(
  i          = 'survey_normalised.las',
  output     = 'tree_tops.shp',
  min_search_radius = 1.0,
  min_height  = 2.0,
  max_search_radius = 5.0,
  max_height  = 40.0
), session = s)

# Canopy cover (fraction first return above height threshold)
wbw_run_tool('lidar_segmentation_based_filter', args = list(
  i            = 'survey_classified.las',
  output       = 'survey_seg_filtered.las',
  slope_threshold = 15.0,
  max_edge_length = 0.5,
  classify     = TRUE
), session = s)
```

---

## WbW-Pro Spotlight: LiDAR Change and Disturbance Analysis

- **Problem:** Compare repeat LiDAR epochs to detect disturbance in a
  repeatable way.
- **Tool:** `lidar_change_and_disturbance_analysis`
- **Typical inputs:** Baseline tile set, monitoring tile set, output
  resolution, minimum change threshold.
- **Typical outputs:** Change rasters plus summary metrics for affected area,
  hotspot intensity, and QA review.

```r
result <- s$lidar_change_and_disturbance_analysis(
  baseline_tiles = '/data/lidar_epoch_2022/',
  monitor_tiles  = '/data/lidar_epoch_2025/',
  resolution     = 1.0,
  min_change_m   = 0.5,
  output_prefix  = 'disturbance_2025_vs_2022'
)

print(result)
```

> **Note:** This workflow requires a session initialized with a valid Pro
> licence.

---

## Complete LiDAR Processing Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/lidar_project')

las_file <- 'raw_survey.las'

# 1. Remove outliers
wbw_run_tool('lidar_remove_outliers', args = list(
  i = las_file, output = 'clean.las', radius = 2.0, elev_diff = 25.0), session = s)

# 2. Ground classification
wbw_run_tool('lidar_ground_point_filter', args = list(
  i = 'clean.las', output = 'classified.las', radius = 2.0,
  min_elev_diff = 0.15, max_elev_diff = 1.3, num_iter = 5,
  threshold = 0.15, slope_threshold = 0.15, height_threshold = 1.0,
  classify = TRUE, slope = FALSE, height_diff = TRUE,
  filter_return_all = FALSE), session = s)

# 3. DTM
wbw_run_tool('tin_gridding', args = list(
  i = 'classified.las', output = 'dtm.tif', returns = 'last',
  resolution = 1.0, exclude_cls = '1,3,4,5,6,7'), session = s)

# 4. DSM and CHM
wbw_run_tool('lidar_idw_interpolation', args = list(
  i = 'classified.las', output = 'dsm.tif', parameter = 'elevation',
  returns = 'first', resolution = 1.0, weight = 1.0, radius = 2.5,
  exclude_cls = '7'), session = s)
wbw_run_tool('subtract', args = list(
  input1 = 'dsm.tif', input2 = 'dtm.tif', output = 'chm.tif'), session = s)

# 5. Normalise for vegetation analysis
wbw_run_tool('normalize_lidar', args = list(
  i = 'classified.las', ground = 'classified.las',
  output = 'normalised.las', ignore_ground_distance = FALSE), session = s)

# 6. Tree detection
wbw_run_tool('individual_tree_detection', args = list(
  i = 'normalised.las', output = 'tree_tops.shp',
  min_search_radius = 1.0, min_height = 2.0,
  max_search_radius = 5.0, max_height = 40.0), session = s)

# 7. Point density
wbw_run_tool('lidar_density', args = list(
  i = 'classified.las', output = 'density.tif',
  resolution = 1.0, returns = 'all', exclude_cls = '7'), session = s)

cat('LiDAR processing complete.\n')
```

---

## Tips

- **Choose your format wisely**: LAS is universal and compact; LAZ adds compression and is ideal for archival or transmission. COPC is cloud-optimized and best for remote HTTP range-request access. Use LAZ or LAS for terrestrial/airborne surveys; COPC for cloud-native workflows.
- **Always validate classifications**: Use `lidar_histogram()` and `lidar_info()` to inspect point distributions by return and classification. Misclassified ground points silently corrupt DTMs and downstream hydrology.
- **DTM vs. DSM vs. CHM**: Generate all three at the same resolution so derivatives align. A common pitfall is mixing DTM and DSM resolution.
- **Ground filtering is critical**: Outliers and noise (classification 7) should be excluded before gridding. Use `lidar_filter_for_ground()` to remove spikes and erratic points.
- **Normalization enables vegetation analysis**: Always normalize point clouds (subtract DTM) before individual tree detection or canopy structural metrics.
- **Monitor memory for large surveys**: Point clouds are memory-intensive. For datasets > 1 GB, use streaming APIs (`lidar_read_chunked()`) rather than loading the entire tile at once.
- **Coordinate reference systems matter**: LAS headers carry CRS as WKT. Verify WKT matches your project CRS before gridding or merging tiles.
- **Density and grid resolution**: If point density is < 0.5 pts/m², consider upsampling or smoothing the output grid to avoid isolated pits or peaks.
```
