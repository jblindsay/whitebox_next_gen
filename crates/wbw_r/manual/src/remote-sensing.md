# Remote Sensing Analysis

Remote sensing workflows in WbW-R cover multispectral and hyperspectral image analysis: spectral index computation, image enhancement, principal component analysis, image segmentation, unsupervised and supervised classification, accuracy assessment, and change detection. All computation runs in the Whitebox backend.

> **Sensor Bundle First**: When working with Sentinel-2, Landsat, PlanetScope, or SAR product folders, open the scene as a **sensor bundle** using `wbw_sensor_bundle_from_path()`. Bundles provide automatic metadata discovery, key-based band access, and one-call true/false-colour composites without hardcoding file names.

---

## Core Concepts

Remote sensing image analysis requires familiarity with these core ideas:

- **Spectral bands**: Distinct wavelength ranges (e.g. blue 450–510 nm, red 620–750 nm, NIR 750–900 nm). Different materials reflect different bands, enabling material discrimination.
- **Spectral indices**: Normalized ratios of bands that isolate phenomena. NDVI (Normalized Difference Vegetation Index) uses NIR and red to measure greenness; NDWI uses NIR and SWIR for water content.
- **Spatial resolution**: Pixel size in meters (Sentinel-2: 10 m for visible/NIR, 20 m for SWIR; Landsat: 30 m; SAR: 5–30 m depending on mode).
- **Temporal resolution**: Revisit interval (Sentinel-2: 5 days; Landsat: 16 days; PlanetScope: daily or higher).
- **Atmospheric effects**: Raw radiance is affected by aerosols, water vapour, and ozone. Surface reflectance products are atmospherically corrected; still require relative normalization for multi-date analysis.
- **Cloud masking**: Cloud and cloud shadow pixels must be identified and excluded before classification or change detection using quality/QA bands.
- **Supervised classification**: Training polygons with known labels teach a model to assign classes to unlabelled pixels. Training quality directly controls classification accuracy.
- **Unsupervised classification**: Clustering algorithms (K-means, ISODATA) discover spectral clusters without training labels; useful for exploratory analysis.
- **Change detection**: Comparing indices or classifications across dates to identify land-use changes, disturbance, or phenological shifts.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/remote_sensing')
```

---

## Sensor Bundles — Cross-Family Scene Ingestion

`wbw_sensor_bundle_from_path()` wraps a product folder and returns a bundle object with a consistent interface across sensor families.

### Opening a Bundle

```r
# Works with Sentinel-2, Landsat 8/9, PlanetScope, SPOT, SAR, etc.
bundle <- wbw_sensor_bundle_from_path(
  '/data/S2B_MSIL2A_20240601T105619_N0510_R094_T32TPT.SAFE',
  session = s
)

# Or from a zip archive — WbW extracts it automatically
bundle <- wbw_sensor_bundle_from_path('/data/LC09_L2SP_017030_20240610.tar', session = s)
```

### Discovering Available Layers

```r
meta <- bundle$metadata()
cat('Family:   ', meta$family, '\n')
cat('Mission:  ', meta$mission, '\n')
cat('Tile:     ', meta$tile_id, '\n')
cat('Level:    ', meta$processing_level, '\n')
cat('Cloud %:  ', meta$cloud_cover_percent, '\n')
cat('Datetime: ', meta$acquisition_datetime_utc, '\n')

print(bundle$list_band_keys())        # e.g. c('B02', 'B03', 'B04', 'B08', ...)
print(bundle$list_measurement_keys()) # e.g. c('ndvi', 'ndwi')
print(bundle$list_qa_keys())          # e.g. c('SCL', 'QA_PIXEL')
print(bundle$list_aux_keys())         # e.g. c('AOT', 'WVP')
print(bundle$list_asset_keys())
```

### Reading Bands by Key

```r
# Keys follow sensor-native naming — no need to memorise file paths
blue  <- bundle$read_band('B02')   # Sentinel-2 blue
green <- bundle$read_band('B03')
red   <- bundle$read_band('B04')
nir   <- bundle$read_band('B08')
swir1 <- bundle$read_band('B11')
swir2 <- bundle$read_band('B12')

# Landsat 9 uses different key names
# blue  <- bundle$read_band('SR_B2')
# nir   <- bundle$read_band('SR_B5')

# QA / cloud mask
scl <- bundle$read_qa_layer('SCL')   # Sentinel-2 scene classification
```

### Quick-Look Composites

```r
# True-colour GeoTIFF (auto-enhanced by default)
bundle$write_true_colour(output_path = 'true_colour.tif')

# False-colour (NIR-Red-Green) composite
bundle$write_false_colour(output_path = 'false_colour.tif')
```

### Cross-Family NDVI Pattern

```r
compute_ndvi_from_bundle <- function(bundle_path, output_path, session = NULL) {
  b <- wbw_sensor_bundle_from_path(bundle_path, session = session)
  meta <- b$metadata()

  if ('ndvi' %in% b$list_measurement_keys()) {
    ndvi_r <- b$read_measurement('ndvi')
    wbw_write_raster(ndvi_r, output_path)
  } else {
    # Fall back to band-based NDVI
    if (meta$family == 'sentinel2') {
      red_r <- b$read_band('B04')
      nir_r <- b$read_band('B08')
    } else {
      red_r <- b$read_band('SR_B4')
      nir_r <- b$read_band('SR_B5')
    }
    wbw_run_tool('normalized_difference_index', args = list(
      input1 = nir_r$file_path(), input2 = red_r$file_path(),
      output = output_path
    ), session = session)
  }
}

compute_ndvi_from_bundle('/data/S2B_MSIL2A_20240601.SAFE', 'ndvi_s2.tif', s)
compute_ndvi_from_bundle('/data/LC09_L2SP_017030.tar',     'ndvi_l9.tif', s)
```

---

## Cloud and No-Data Masking

```r
# Sentinel-2 SCL: 3=cloud shadow, 8=med cloud, 9=high cloud, 10=thin cirrus
scl <- bundle$read_qa_layer('SCL')
wbw_run_tool('raster_calculator', args = list(
  output    = 'red_cloud_free.tif',
  statement = paste0("if('", scl$file_path(), "' == 3 or '", scl$file_path(), "' == 8 ",
                     "or '", scl$file_path(), "' == 9 or '", scl$file_path(), "' == 10, ",
                     "nodata, '", red$file_path(), "')")
), session = s)

# Landsat Collection 2 QA_PIXEL — use bitwise expressions via raster_calculator
# qa <- bundle$read_qa_layer('QA_PIXEL')
```

---

## Reading Multi-Band Imagery (Individual Files)

When bundles are not available (e.g., reprojected mosaics, custom composites, legacy archives), load individual band files in the conventional way:

```r
b2 <- wbw_read_raster('LC08_B2_blue.tif')
b3 <- wbw_read_raster('LC08_B3_green.tif')
b4 <- wbw_read_raster('LC08_B4_red.tif')
b5 <- wbw_read_raster('LC08_B5_nir.tif')
b6 <- wbw_read_raster('LC08_B6_swir1.tif')
b7 <- wbw_read_raster('LC08_B7_swir2.tif')

# Resample if bands have different resolutions
wbw_run_tool('resample', args = list(
  inputs   = b6$file_path(),
  output   = 'b6_10m.tif',
  cell_size = 0.0,
  base     = b4$file_path(),
  method   = 'bilinear'
), session = s)
```

---

## Spectral Indices

### Normalised Difference Vegetation Index (NDVI)

$$NDVI = \frac{NIR - Red}{NIR + Red}$$

```r
# Using bands read from a bundle
wbw_run_tool('normalized_difference_index', args = list(
  input1 = nir$file_path(),    # NIR
  input2 = red$file_path(),    # Red
  output = 'ndvi.tif'
), session = s)
```

### Common Water, Snow, and Urban Indices

```r
# Define index triplets: (output name, numerator band, denominator band)
indices <- list(
  list(name = 'ndwi',  b1 = green, b2 = nir),   # open water;  Green/NIR
  list(name = 'mndwi', b1 = green, b2 = swir1),  # urban water; Green/SWIR1
  list(name = 'nbr',   b1 = nir,   b2 = swir2),  # fire severity; NIR/SWIR2
  list(name = 'ndsi',  b1 = green, b2 = swir1),  # snow/ice;    Green/SWIR1
  list(name = 'ndbi',  b1 = swir1, b2 = nir)     # built-up;    SWIR1/NIR
)

for (idx in indices) {
  wbw_run_tool('normalized_difference_index', args = list(
    input1 = idx$b1$file_path(),
    input2 = idx$b2$file_path(),
    output = paste0(idx$name, '.tif')
  ), session = s)
}
```

### EVI (Enhanced Vegetation Index)

$$EVI = 2.5 \cdot \frac{NIR - Red}{NIR + 6 \cdot Red - 7.5 \cdot Blue + 1}$$

```r
wbw_run_tool('raster_calculator', args = list(
  output    = 'evi.tif',
  statement = paste0("2.5 * ('", nir$file_path(), "' - '", red$file_path(), "') / (",
                     "'", nir$file_path(), "' + 6.0 * '", red$file_path(), "' - 7.5 * '",
                     blue$file_path(), "' + 1.0)")
), session = s)
```

---

## Image Enhancement

### Percentage Linear Stretch

```r
wbw_run_tool('percentage_contrast_stretch', args = list(
  i      = b4$file_path(),
  output = 'b4_stretched.tif',
  clip   = 1.0,
  tail   = 'both',
  num_tones = 256
), session = s)
```

### Standard Deviation Stretch

```r
wbw_run_tool('standard_deviation_contrast_stretch', args = list(
  i         = b4$file_path(),
  output    = 'b4_sd_stretch.tif',
  stdev     = 2.0,
  num_tones = 256
), session = s)
```

### Gamma Correction

```r
wbw_run_tool('gamma_correction', args = list(
  i      = b4$file_path(),
  output = 'b4_gamma.tif',
  gamma  = 0.55
), session = s)
```

### Histogram Matching

```r
wbw_run_tool('histogram_matching', args = list(
  i          = 'image_target.tif',
  histo_file = 'reference_histogram.html',
  output     = 'image_matched.tif'
), session = s)
```

---

## IHS Colour Space and Pan-Sharpening

```r
wbw_run_tool('rgb_to_ihs', args = list(
  intensity = b4$file_path(),
  hue       = b3$file_path(),
  saturation = b2$file_path(),
  output    = 'ihs.tif'
), session = s)

# Pan-sharpen
wbw_run_tool('ihs_to_rgb', args = list(
  intensity  = 'pan_band.tif',
  hue        = 'ihs_hue.tif',
  saturation = 'ihs_sat.tif',
  output     = 'pansharpened.tif'
), session = s)
```

---

## Spatial Filtering

```r
# Gaussian smoothing
wbw_run_tool('gaussian_filter', args = list(
  i = b5$file_path(), output = 'b5_gauss.tif', sigma = 2.0), session = s)

# Edge detection — Canny
wbw_run_tool('canny_edge_detection', args = list(
  i = b4$file_path(), output = 'edges.tif', sigma = 0.5,
  low_threshold = 0.05, high_threshold = 0.15, add_back = FALSE), session = s)
```

---

## Principal Component Analysis

```r
# Supply a comma-separated list of band files
all_bands <- paste(c(b2$file_path(), b3$file_path(), b4$file_path(),
                     b5$file_path(), b6$file_path(), b7$file_path()),
                   collapse = ';')

wbw_run_tool('principal_component_analysis', args = list(
  inputs        = all_bands,
  output        = 'pca_loadings.html',
  num_comp      = 6,
  standardised  = FALSE
), session = s)

# PC scores are written as pc1.tif, pc2.tif, ... in the working directory
```

---

## Image Segmentation

```r
wbw_run_tool('image_segmentation', args = list(
  inputs       = all_bands,
  output       = 'segments.tif',
  threshold    = 30.0,
  steps        = 10,
  min_area     = 10
), session = s)
```

---

## Unsupervised Classification

### KMeans

```r
wbw_run_tool('modified_k_means_clustering', args = list(
  inputs        = all_bands,
  output        = 'kmeans.tif',
  out_html      = 'kmeans_report.html',
  start_clusters = 5,
  end_clusters   = 10,
  max_iterations = 25,
  class_change   = 2.0
), session = s)
```

### DBSCAN

```r
wbw_run_tool('dbscan', args = list(
  inputs   = all_bands,
  output   = 'dbscan.tif',
  search_dist = 2.5,
  min_points   = 10
), session = s)
```

---

## Supervised Classification

### Random Forest

```r
# 1. Fit the model
wbw_run_tool('random_forest_classification_fit', args = list(
  inputs  = all_bands,
  training = 'training_polygons.shp',
  field    = 'CLASS_ID',
  output   = 'rf_model.bin',
  n_trees  = 100,
  min_leaf_size = 1,
  max_features  = 0.0
), session = s)

# 2. Apply the model
wbw_run_tool('random_forest_classification_predict', args = list(
  inputs  = all_bands,
  model   = 'rf_model.bin',
  output  = 'rf_classification.tif'
), session = s)
```

### Support Vector Machine

```r
wbw_run_tool('svm_classification', args = list(
  inputs  = all_bands,
  training = 'training_polygons.shp',
  field    = 'CLASS_ID',
  output   = 'svm_classification.tif',
  c        = 200.0,
  gamma    = 50.0,
  cost     = 10.0,
  tolerance = 0.1,
  test_proportion = 0.2
), session = s)
```

### K-Nearest Neighbours

```r
wbw_run_tool('knn_classification', args = list(
  inputs  = all_bands,
  training = 'training_polygons.shp',
  field    = 'CLASS_ID',
  output   = 'knn_classification.tif',
  k        = 5,
  scaling  = TRUE,
  test_proportion = 0.2
), session = s)
```

---

## Accuracy Assessment

```r
wbw_run_tool('kappa_index', args = list(
  i1     = 'rf_classification.tif',
  i2     = 'reference_classification.tif',
  output = 'accuracy_report.html'
), session = s)
```

---

## Change Detection

### Image Differencing

```r
wbw_run_tool('change_vector_analysis', args = list(
  date1     = paste(c('t1_b2.tif','t1_b3.tif','t1_b4.tif','t1_b5.tif'), collapse=';'),
  date2     = paste(c('t2_b2.tif','t2_b3.tif','t2_b4.tif','t2_b5.tif'), collapse=';'),
  magnitude = 'cva_magnitude.tif',
  direction = 'cva_direction.tif'
), session = s)
```

---

## WbW-Pro Spotlight: In-Season Crop Stress Intervention Planning

- **Problem:** Turn in-season crop stress signals into actionable intervention
  priorities.
- **Tool:** `in_season_crop_stress_intervention_planning`
- **Typical inputs:** NDVI raster, canopy-temperature raster, soil-moisture
  raster.
- **Typical outputs:** Intervention-priority and intervention-class products
  with summary reporting.

```r
result <- s$in_season_crop_stress_intervention_planning(
  ndvi               = 'ndvi_latest.tif',
  canopy_temperature = 'lst_latest.tif',
  soil_moisture      = 'soil_moisture_latest.tif',
  output_prefix      = 'field_07_stress'
)

print(result)
```

> **Note:** This workflow requires a session initialized with a valid Pro
> licence.

---

## Complete Remote Sensing Workflow

The following end-to-end example uses a sensor bundle for ingestion and random forest for land-cover mapping:

```r
library(whiteboxworkflows)
s <- wbw_session()

# 1. Open scene as a bundle — works with S2, Landsat 9, PlanetScope, etc.
bundle <- wbw_sensor_bundle_from_path(
  '/data/S2B_MSIL2A_20240601T105619_N0510_R094_T32TPT.SAFE',
  session = s
)
meta <- bundle$metadata()
cat(sprintf('Family: %s, tile: %s, clouds: %.1f%%\n',
            meta$family, meta$tile_id, meta$cloud_cover_percent))

if (meta$cloud_cover_percent > 20) {
  stop('Too cloudy for classification')
}

# 2. Read bands by key (family-agnostic)
blue  <- bundle$read_band('B02')
green <- bundle$read_band('B03')
red   <- bundle$read_band('B04')
nir   <- bundle$read_band('B08')

wbw_run_tool('resample', args = list(
  inputs = bundle$read_band('B11')$file_path(), output = 'swir1_10m.tif',
  cell_size = 0.0, base = red$file_path(), method = 'bilinear'), session = s)
wbw_run_tool('resample', args = list(
  inputs = bundle$read_band('B12')$file_path(), output = 'swir2_10m.tif',
  cell_size = 0.0, base = red$file_path(), method = 'bilinear'), session = s)
swir1 <- wbw_read_raster('swir1_10m.tif')
swir2 <- wbw_read_raster('swir2_10m.tif')

bands <- list(blue, green, red, nir, swir1, swir2)
band_paths <- paste(sapply(bands, function(b) b$file_path()), collapse = ';')

# 3. Mask clouds using SCL
scl <- bundle$read_qa_layer('SCL')

# 4. Spectral indices
wbw_run_tool('normalized_difference_index', args = list(
  input1 = nir$file_path(), input2 = red$file_path(),
  output = 'ndvi.tif'), session = s)
wbw_run_tool('normalized_difference_index', args = list(
  input1 = green$file_path(), input2 = swir1$file_path(),
  output = 'mndwi.tif'), session = s)
wbw_run_tool('normalized_difference_index', args = list(
  input1 = nir$file_path(), input2 = swir2$file_path(),
  output = 'nbr.tif'), session = s)

ndvi  <- wbw_read_raster('ndvi.tif')
mndwi <- wbw_read_raster('mndwi.tif')
nbr   <- wbw_read_raster('nbr.tif')

# 5. PCA decorrelation
wbw_run_tool('principal_component_analysis', args = list(
  inputs       = band_paths,
  output       = 'pca_loadings.html',
  num_comp     = 4,
  standardised = TRUE
), session = s)
pc_paths <- paste(paste0('pc', 1:4, '.tif'), collapse = ';')

# 6. Feature stack = bands + indices + PCA scores
feature_paths <- paste(c(band_paths, 'ndvi.tif', 'mndwi.tif', 'nbr.tif', pc_paths),
                       collapse = ';')

# 7. Train random forest
wbw_run_tool('random_forest_classification_fit', args = list(
  inputs   = feature_paths,
  training = 'training_polygons.shp',
  field    = 'CLASS',
  output   = 'rf_model.bin',
  n_trees  = 500,
  test_proportion = 0.25
), session = s)

# 8. Predict
wbw_run_tool('random_forest_classification_predict', args = list(
  inputs = feature_paths,
  model  = 'rf_model.bin',
  output = 'lulc.tif'
), session = s)

# 9. Generalise
wbw_run_tool('generalize_classified_raster', args = list(
  i = 'lulc.tif', output = 'lulc_clean.tif', min_size = 9), session = s)

# 10. Accuracy assessment
wbw_run_tool('kappa_index', args = list(
  i1 = 'lulc_clean.tif', i2 = 'validation_reference.tif',
  output = 'accuracy.html'), session = s)

# 11. Quick-look composite for QC
bundle$write_true_colour(output_path = 'quicklook_truecolour.tif')

cat('Remote sensing workflow complete.\n')
```

---

## Tips

- **Use bundles for scene-level ingestion**: `wbw_sensor_bundle_from_path()` eliminates hardcoded band filenames and works identically across Sentinel-2, Landsat, PlanetScope, and other supported families.
- **Screen cloud cover before processing**: Check bundle metadata `cloud_cover_percent` before loading all bands to skip unusable scenes early in a batch workflow.
- **Atmospheric correction is iterative**: Raw digital numbers and even surface reflectance products may still carry illumination and atmospheric artefacts. Histogram matching across scenes is a simple relative normalisation that works well when no reference spectrum is available.
- **Align band resolution before stacking**: Sentinel-2 delivers 10 m (Blue, Green, Red, NIR) and 20 m (Red Edge, SWIR) bands. Upscale the 20 m bands to 10 m using bilinear resampling before combining them in a feature stack.
- **Always mask clouds and cloud shadows**: Use quality bands (Sentinel-2 SCL or Landsat QA_PIXEL) and apply bitwise tests via raster calculator to isolate valid pixels.
- **Balance training data**: Training datasets often over-represent dominant classes (e.g., forest) relative to rare classes (e.g., wetland). Sample training polygons proportional to expected class area, or oversample rare classes to improve model accuracy.
- **Check spectral separability**: If accuracy assessment reports Jeffries-Matusita distance < 1.5 between any two classes, consider merging classes or collecting more spectrally diverse training polygons.
- **Use time-series techniques for change detection**: Multi-date classifications can show spurious change due to atmospheric variation. Spectral indices are more robust; consider computing NDVI or NDWI time series and detecting change directly in index space.
- **Memory is your bottleneck**: Large multispectral stacks fill RAM quickly. Use band-by-band processing or out-of-memory methods for scenes > 1 GB.
```
