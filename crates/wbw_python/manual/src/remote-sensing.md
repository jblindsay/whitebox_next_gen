# Remote Sensing Analysis

Remote sensing uses satellite imagery, aerial photography, and drone captures to characterise the Earth's surface. Whitebox Workflows for Python (WbW-Py) provides a comprehensive toolkit for image processing, spectral analysis, dimensionality reduction, supervised and unsupervised classification, change detection, and sensor-specific workflows. Because all tools operate directly on in-memory raster objects, complex multi-step image analysis pipelines can be written as concise Python scripts.

> **Sensor Bundle First**: When working with industry-standard satellite products (Sentinel-2, Landsat, PlanetScope, SAR, and others), open the scene as a **sensor bundle** rather than loading individual band files by hand. Bundles provide automatic metadata discovery, key-based band access, and one-call true/false-colour composites that work across sensor families without hardcoding file names.

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

## Sensor Bundles — Cross-Family Scene Ingestion

WbW-Py's `Bundle` class wraps a product folder (zip archive or extracted directory) and exposes a consistent API regardless of sensor family. This is the preferred starting point for any scene-level workflow.

### Opening a Bundle

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

# Works with Sentinel-2, Landsat 8/9, PlanetScope, SPOT, SAR, etc.
bundle = wbe.read_bundle('/data/S2B_MSIL2A_20240601T105619_N0510_R094_T32TPT_20240601T142845.SAFE')

# Or from a zip archive — WbW extracts it automatically
bundle = wbe.read_bundle('/data/LC09_L2SP_017030_20240610_20240612_02_T1.tar')
```

### Discovering Available Layers

`Bundle` surfaces keys for bands, derived measurements, QA/cloud masks, auxiliary layers, and additional assets:

```python
print(bundle.family)               # e.g. 'sentinel2', 'landsat_c2', 'planetscope'
print(bundle.mission)              # e.g. 'Sentinel-2B'
print(bundle.tile_id())            # e.g. '32TPT'
print(bundle.processing_level())   # e.g. 'L2A', 'L2SP'
print(bundle.cloud_cover_percent())
print(bundle.acquisition_datetime_utc())

print(bundle.list_band_keys())        # spectral bands, e.g. ['B02', 'B03', 'B04', 'B08', ...]
print(bundle.list_measurement_keys()) # derived measurements, e.g. ['ndvi', 'ndwi']
print(bundle.list_qa_keys())          # quality/cloud layers, e.g. ['SCL', 'QA_PIXEL']
print(bundle.list_aux_keys())         # auxiliary rasters, e.g. ['AOT', 'WVP']
print(bundle.list_asset_keys())       # additional assets (angles, browse images, etc.)
```

### Reading Bands by Key

```python
# Keys follow sensor-native naming — no need to memorise file paths
blue  = bundle.read_band('B02')   # Sentinel-2 blue
green = bundle.read_band('B03')
red   = bundle.read_band('B04')
nir   = bundle.read_band('B08')
swir1 = bundle.read_band('B11')
swir2 = bundle.read_band('B12')

# Landsat 9 uses different key names
# blue  = bundle.read_band('SR_B2')
# green = bundle.read_band('SR_B3')
# red   = bundle.read_band('SR_B4')
# nir   = bundle.read_band('SR_B5')

# QA / cloud mask
cloud_mask = bundle.read_qa_layer('SCL')      # Sentinel-2 scene classification
# cloud_mask = bundle.read_qa_layer('QA_PIXEL')  # Landsat Collection 2
```

### Quick-Look Composites

Bundles know which keys correspond to red/green/blue/NIR bands for their sensor family, so composites require no band-order bookkeeping:

```python
# Write a true-colour GeoTIFF (auto-enhanced by default)
bundle.true_colour_composite(wbe, output_path='true_colour.tif')

# False-colour (NIR-Red-Green) composite
bundle.false_colour_composite(wbe, output_path='false_colour.tif')
```

### Multi-Sensor Workflow Without Hardcoded Paths

The bundle API is deliberately family-agnostic. The same code body works on Sentinel-2 and Landsat scenes because both expose their bands by semantic key:

```python
def compute_ndvi_from_bundle(wbe, bundle_path, output_path):
    b = wbe.read_bundle(bundle_path)
    if 'ndvi' in b.list_measurement_keys():
        ndvi = b.read_measurement('ndvi')
    else:
        # Fall back to band-based NDVI — works for any optical sensor
        red_band = b.read_band('B04') if b.family == 'sentinel2' else b.read_band('SR_B4')
        nir_band = b.read_band('B08') if b.family == 'sentinel2' else b.read_band('SR_B5')
        ndvi = wbe.normalized_difference_index(nir_band, red_band)
    wbe.write_raster(ndvi, output_path, compress=True)

compute_ndvi_from_bundle(wbe, '/data/S2B_MSIL2A_20240601.SAFE', 'ndvi_s2.tif')
compute_ndvi_from_bundle(wbe, '/data/LC09_L2SP_017030.tar',     'ndvi_l9.tif')
```

---

## Working with Individual Band Files

When bundles are not available (e.g., reprojected mosaics, custom composites, legacy archives), load individual band files in the conventional way. All raster objects returned by `read_band()` and `read_raster()` are interchangeable.

```python
wbe.working_directory = '/data/sentinel2'

blue   = wbe.read_raster('B02_10m.tif')
green  = wbe.read_raster('B03_10m.tif')
red    = wbe.read_raster('B04_10m.tif')
nir    = wbe.read_raster('B08_10m.tif')

# Sentinel-2 bands 11/12 are 20 m; resample to match 10 m bands before analysis
swir1 = wbe.resample(wbe.read_raster('B11_20m.tif'), base_raster=red, method='bilinear')
swir2 = wbe.resample(wbe.read_raster('B12_20m.tif'), base_raster=red, method='bilinear')
```

---

## Cloud and No-Data Masking

Before any analysis, mask clouds, cloud shadows, and saturated pixels.

```python
# Sentinel-2 SCL classes: 3=cloud shadow, 8=medium cloud, 9=high cloud, 10=thin cirrus
scl = bundle.read_qa_layer('SCL')
cloud_free_red = wbe.raster_calculator(
    "if('scl' == 3 or 'scl' == 8 or 'scl' == 9 or 'scl' == 10, nodata, 'red')",
    [scl, red]
)

# Landsat Collection 2 QA_PIXEL — dilated cloud = bit 1, cloud = bit 3, shadow = bit 4
# qa = bundle.read_qa_layer('QA_PIXEL')
# Use raster_calculator with bitwise masking expressions to isolate cloud/shadow pixels
```

---

## Spectral Indices

Spectral indices are band-ratio transformations that suppress illumination variation and amplify specific surface properties. WbW-Py's `normalized_difference_index()` is a general-purpose tool for any normalised ratio, while `raster_calculator()` accommodates arbitrary expressions.

### Normalised Difference Vegetation Index (NDVI)

NDVI measures photosynthetically active green biomass:

$$NDVI = \frac{NIR - Red}{NIR + Red}$$

Values range from −1 to +1; healthy vegetation typically exceeds 0.3.

```python
ndvi = wbe.normalized_difference_index(nir, red)
wbe.write_raster(ndvi, 'ndvi.tif', compress=True)
```

### Common Water, Snow, and Urban Indices

```python
# NDWI (McFeeters 1996) — open water; Green/NIR
ndwi  = wbe.normalized_difference_index(green, nir)

# MNDWI — better for urban areas; Green/SWIR1
mndwi = wbe.normalized_difference_index(green, swir1)

# NBR — fire severity and post-fire recovery; NIR/SWIR2
nbr   = wbe.normalized_difference_index(nir, swir2)

# NDSI — snow and ice; Green/SWIR1
ndsi  = wbe.normalized_difference_index(green, swir1)

# NDBI — normalised built-up index; SWIR1/NIR
ndbi  = wbe.normalized_difference_index(swir1, nir)
```

### Enhanced Vegetation Index (EVI)

EVI reduces soil background noise and atmospheric effects that affect NDVI in dense canopies:

$$EVI = 2.5 \cdot \frac{NIR - Red}{NIR + 6 \cdot Red - 7.5 \cdot Blue + 1}$$

```python
evi = wbe.raster_calculator(
    expression="2.5 * ('nir' - 'red') / ('nir' + 6.0 * 'red' - 7.5 * 'blue' + 1.0)",
    input_rasters=[nir, red, blue]
)
wbe.write_raster(evi, 'evi.tif', compress=True)
```

### Soil Adjusted Vegetation Index (SAVI)

SAVI introduces a soil brightness correction factor *L* (commonly 0.5):

$$SAVI = \frac{(NIR - Red) \cdot (1 + L)}{NIR + Red + L}$$

```python
L = 0.5
savi = wbe.raster_calculator(
    expression="('nir' - 'red') * (1.0 + 0.5) / ('nir' + 'red' + 0.5)",
    input_rasters=[nir, red]
)
```

---

## Image Enhancement and Contrast Stretching

Raw digital numbers often have narrow histogram ranges that make visualisation difficult. WbW-Py provides several enhancement methods:

```python
# Percentage contrast stretch — clips top and bottom 2% of values
enhanced = wbe.percentage_contrast_stretch(red, clip_tail=2.0)

# Standard deviation stretch — maps ±2σ to display range
sd_stretched = wbe.standard_deviation_contrast_stretch(red, num_std_dev=2.0)

# Gaussian contrast stretch
gauss = wbe.gaussian_contrast_stretch(red, num_std_dev=2.0)

# Sigmoidal stretch — soft S-curve useful for optical imagery
sig = wbe.sigmoidal_contrast_stretch(red, cutoff=0.4, gain=4.0)

# Min-max linear stretch
linear = wbe.min_max_contrast_stretch(red, min_val=0.0, max_val=255.0)

# Gamma correction — lighten (gamma<1) or darken (gamma>1) an image
gamma = wbe.gamma_correction(red, gamma=0.6)
```

Histogram matching normalises a source image to match the distribution of a reference image — invaluable when mosaicking scenes acquired on different dates or under different atmospheric conditions:

```python
# Match the histogram of a target scene to a reference scene
matched = wbe.histogram_matching(source_scene, reference_scene)

# You can also match between two images from the same sensor
matched2 = wbe.histogram_matching_two_images(img1, img2)
```

### IHS Colour Space Transformations

Intensity-Hue-Saturation (IHS) space separates brightness from colour, enabling selective sharpening or enhancement without hue distortion:

```python
# Convert RGB composite to IHS
(intensity, hue, saturation) = wbe.rgb_to_ihs(red, green, blue)

# Enhance the intensity channel
enhanced_i = wbe.standard_deviation_contrast_stretch(intensity, num_std_dev=2.0)

# Convert back to RGB
(r2, g2, b2) = wbe.ihs_to_rgb(enhanced_i, hue, saturation)
```

### Panchromatic Sharpening (Pan-Sharpening)

When a high-resolution panchromatic band accompanies lower-resolution multispectral data, IHS pan-sharpening fuses the two:

```python
panchromatic = bundle.read_band('B8A')   # Sentinel-2 red-edge as panchromatic proxy
# Landsat: panchromatic = bundle.read_band('PAN')   # 15 m Landsat panchromatic

(sharp_r, sharp_g, sharp_b) = wbe.panchromatic_sharpening(
    pan=panchromatic,
    red=red,
    green=green,
    blue=blue
)
wbe.write_raster(sharp_r, 'pan_sharp_r.tif')
wbe.write_raster(sharp_g, 'pan_sharp_g.tif')
wbe.write_raster(sharp_b, 'pan_sharp_b.tif')
```

---

## Image Filtering

Spatial filters are used to suppress noise, enhance edges, or smooth imagery before classification.

```python
# Gaussian smoothing — reduces sensor noise
smoothed = wbe.gaussian_filter(red, sigma=1.0)

# Bilateral filter — edge-preserving smoothing
bilateral = wbe.bilateral_filter(red, sigma_dist=2.0, sigma_int=25.0)

# High-pass filter — enhances fine texture
texture = wbe.high_pass_filter(red, filter_size_x=5, filter_size_y=5)

# Unsharp masking — sharpens blurred imagery
sharp = wbe.unsharp_masking(red, sigma=3.0, amount=1.0, threshold=0)

# Edge detection — Sobel gradient magnitude
edges_h = wbe.sobel_filter(red, variant='3x3')

# Canny edge detector — more precise edge localisation
canny_edges = wbe.canny_edge_detection(red, sigma=0.5,
                                        low_threshold=5.0, high_threshold=15.0)
```

---

## Principal Component Analysis (PCA)

PCA is an essential step in multispectral and hyperspectral analysis. It rotates correlated bands into decorrelated principal components (PCs) ordered by descending variance. The first few PCs typically capture most scene variance and are used to reduce data dimensionality before classification.

```python
# Run PCA on all six bands
bands = [blue, green, red, nir, swir1]
pca_result = wbe.principal_component_analysis(
    input_rasters=bands,
    output_html_file='pca_report.html',
    num_comp=4,
    standardize=True   # unit-variance standardisation recommended when bands
                       # have very different value ranges
)
# pca_result is a list of component rasters ordered PC1, PC2, ...
pc1, pc2, pc3, pc4 = pca_result[0], pca_result[1], pca_result[2], pca_result[3]
wbe.write_raster(pc1, 'pc1.tif')
wbe.write_raster(pc2, 'pc2.tif')
```

The HTML report includes the eigenvalue table, percentage of variance explained per component, and the component loadings matrix. PC1 often correlates strongly with overall brightness. PC2 frequently captures vegetation/non-vegetation contrast. Higher PCs capture moisture, soil, and urban differences.

### Inverse PCA

Apply classification or enhancements to individual PCs and then reconstruct back to original band space:

```python
# Modify pc1, then reconstruct
pc1_modified = wbe.standard_deviation_contrast_stretch(pc1, num_std_dev=2.0)
modified_components = [pc1_modified, pc2, pc3, pc4]

reconstructed_bands = wbe.inverse_pca(
    modified_components,
    original_rasters=bands
)
```

---

## Image Segmentation and Object-Based Analysis

Object-based image analysis (OBIA) groups pixels into meaningful spatial units (segments) before classifying them. This is superior to pixel-based classification for high-resolution imagery where individual objects span many pixels.

```python
# Segment the image into homogeneous regions
# threshold controls merge aggressiveness (lower = more segments)
segments = wbe.image_segmentation(
    input_rasters=[red, green, nir],
    threshold=15.0,
    min_size=20
)
wbe.write_raster(segments, 'segments.tif')
```

After segmentation, extract per-object statistics using `zonal_statistics()` and then classify based on the object-mean spectral signature.

---

## Unsupervised Classification

Unsupervised classification groups pixels by spectral similarity without training data, making it useful for exploratory analysis.

### K-Means Clustering

```python
bands = [blue, green, red, nir, swir1]

kmeans_result = wbe.k_means_clustering(
    input_rasters=bands,
    num_classes=10,
    max_iterations=25,
    class_change_threshold=2.0,
    initialize='random'
)
wbe.write_raster(kmeans_result, 'kmeans_10class.tif')
```

K-means with 10–15 classes followed by manual merging of semantically similar classes is a common workflow. Visualise the clusters against known reference areas to assign land-cover labels.

### Modified K-Means

Modified k-means iteratively merges clusters that are too small or spectrally indistinct, and splits clusters that are too dispersed:

```python
mod_kmeans = wbe.modified_k_means_clustering(
    input_rasters=bands,
    start_num_classes=20,
    merge_distance=2.0,
    max_iterations=25
)
wbe.write_raster(mod_kmeans, 'modified_kmeans.tif')
```

### DBSCAN Clustering

DBSCAN is a density-based algorithm that does not require specifying the number of clusters and handles irregularly shaped spectral clusters well:

```python
dbscan_result = wbe.dbscan(
    input_rasters=bands,
    scaling_method='min_max',
    search_distance=0.5,
    min_points=5
)
wbe.write_raster(dbscan_result, 'dbscan_clusters.tif')
```

---

## Supervised Classification

Supervised classification uses labelled training areas to build a model that is then applied across the full image. WbW-Py supports several classifiers.

### Evaluating Training Sites

Before training, check that each class is spectrally separable. Poor separability leads to confused outputs regardless of classifier choice:

```python
training_polys = wbe.read_vector('training_polygons.shp')
wbe.evaluate_training_sites(
    input_rasters=bands,
    training_polygons=training_polys,
    class_field_name='CLASS',
    output_html_file='separability_report.html'
)
```

The HTML report shows histograms and box-plots per band per class. Classes whose histograms overlap substantially should be merged or split by adding more nuanced training polygons.

### Minimum Distance Classification

The simplest linear classifier assigns each pixel to the nearest class mean in feature space:

```python
classified_md = wbe.min_dist_classification(
    input_rasters=bands,
    polys=training_polys,
    class_field=True,
    class_field_name='CLASS',
    threshold=5.0  # optional: do not classify if z-score distance > 5
)
wbe.write_raster(classified_md, 'classified_min_dist.tif')
```

### Parallelepiped Classification

The parallelepiped classifier assigns pixels that fall within all class-mean ± n×σ boxes. It is fast but can leave pixels unclassified when they fall outside all boxes:

```python
classified_pp = wbe.parallelepiped_classification(
    input_rasters=bands,
    polys=training_polys,
    class_field_name='CLASS'
)
wbe.write_raster(classified_pp, 'classified_parallelepiped.tif')
```

### K-Nearest Neighbour Classification

KNN classification is a non-parametric method well-suited to non-linear class boundaries. The `k` parameter is the number of neighbours to consider:

```python
training_pts = wbe.read_vector('training_points.shp')

knn_model = wbe.knn_classification(
    input_rasters=bands,
    training_data=training_pts,
    field_name='CLASS',
    scaling_method='z_score',
    k=7,
    distance_weighting=True,
    test_proportion=0.2,
    create_output=True
)
wbe.write_raster(knn_model, 'classified_knn.tif')
```

### Random Forest Classification

Random forest is an ensemble tree classifier that is robust to noise, handles multi-class problems, and produces out-of-bag accuracy estimates. All formerly Pro-tier classifier tools are now open-source:

```python
# Step 1: fit the model and save it
rf_model = wbe.random_forest_classification_fit(
    input_rasters=bands,
    training_data=training_polys,
    class_field_name='CLASS',
    n_trees=500,
    min_samples_leaf=1,
    test_proportion=0.2
)
# rf_model is a path to the saved model file

# Step 2: predict on the full image (can be a different image/date)
classified_rf = wbe.random_forest_classification_predict(
    input_rasters=bands,
    model=rf_model
)
wbe.write_raster(classified_rf, 'classified_rf.tif')
```

The fit step prints an accuracy report including the confusion matrix, Kappa coefficient, and overall accuracy.

### Support Vector Machine (SVM) Classification

SVMs find the maximum-margin hyperplane separating classes and are particularly effective with high-dimensional data and small training sets:

```python
classified_svm = wbe.svm_classification(
    input_rasters=bands,
    training=training_polys,
    field='CLASS',
    scaling_method='z_score',
    test_proportion=0.2,
    create_output=True
)
wbe.write_raster(classified_svm, 'classified_svm.tif')
```

### Logistic Regression Classification

Logistic regression is a fast and interpretable baseline classifier, useful as a benchmark before applying more complex models:

```python
classified_lr = wbe.logistic_regression(
    input_rasters=bands,
    training_data=training_polys,
    class_field_name='CLASS',
    scaling_method='z_score',
    test_proportion=0.2,
    create_output=True
)
```

### Accuracy Assessment

Compare a classified raster against a validation point dataset:

```python
# Kappa index of agreement between classified image and reference data
accuracy = wbe.kappa_index(
    classified=classified_rf,
    reference=wbe.read_raster('reference_classification.tif'),
    output_html_file='accuracy_report.html'
)
```

---

## Generalising Classification Outputs

Classified rasters often contain salt-and-pepper noise — isolated pixels assigned to a class inconsistent with their neighbours. Two tools remove this:

```python
# Remove small patches by merging them into surrounding majority class
generalised = wbe.generalize_classified_raster(
    classified=classified_rf,
    min_class_size=5  # minimum patch size in cells
)

# Alternative: merge small patches into most spectrally similar neighbour
generalised2 = wbe.generalize_with_similarity(
    input_rasters=bands,
    classified=classified_rf,
    min_class_size=5
)

# Remove holes (background pixels encircled by foreground)
no_holes = wbe.remove_raster_polygon_holes(
    input_raster=classified_rf,
    threshold=25          # cells
)
wbe.write_raster(generalised, 'classified_rf_clean.tif')
```

---

## Change Detection

Multi-temporal analysis detects land-cover change between two acquisitions.

### Image Differencing

The simplest approach differences co-registered images. NDVI differencing, for example, highlights vegetation gain and loss:

```python
ndvi_t1 = wbe.normalized_difference_index(nir_t1, red_t1)
ndvi_t2 = wbe.normalized_difference_index(nir_t2, red_t2)
ndvi_change = wbe.raster_calculator("'t2' - 't1'", [ndvi_t2, ndvi_t1])
wbe.write_raster(ndvi_change, 'ndvi_change.tif', compress=True)
```

### Change Vector Analysis (CVA)

CVA computes the magnitude and direction of change in multidimensional feature space between two dates. The output direction angle indicates the type of change (e.g., vegetation gain vs. urban growth) while magnitude reflects its intensity:

```python
magnitude, direction = wbe.change_vector_analysis(
    input_rasters_t1=[red_t1, nir_t1, swir1_t1],
    input_rasters_t2=[red_t2, nir_t2, swir1_t2]
)
wbe.write_raster(magnitude, 'cva_magnitude.tif')
wbe.write_raster(direction, 'cva_direction.tif')
```

### Post-Classification Comparison

Classify both dates independently and difference the class maps. This preserves the semantics of each date's land-cover map:

```python
# Train on t1 and apply to t2 using the same saved RF model
classified_t1 = wbe.random_forest_classification_predict(bands_t1, rf_model)
classified_t2 = wbe.random_forest_classification_predict(bands_t2, rf_model)

# Produce a change raster (unique code for each from-to class transition)
change_map = wbe.raster_calculator(
    "'t1' * 100 + 't2'",
    [classified_t1, classified_t2]
)
wbe.write_raster(change_map, 'change_map.tif')
```

---

## Water Extraction and River Mapping

Combining NDWI thresholding with morphological post-processing extracts water bodies:

```python
# Threshold MNDWI — water pixels where mndwi > 0
water = wbe.raster_calculator("if('mndwi' > 0.0, 1.0, nodata)", [mndwi])

# Remove small spurious water patches
water_clean = wbe.sieve(water, threshold=50)

# Remove islands in water polygons
water_filled = wbe.remove_raster_polygon_holes(water_clean, threshold=100)

# Extract river centrelines from the water raster
river_lines = wbe.river_centerlines(water_filled, min_length=5, search_radius=9)
wbe.write_vector(river_lines, 'river_centerlines.shp')
```

---

## Image Correlation and Regression

WbW-Py supports several image correlation and regression tools useful for sensor cross-calibration, quality control, and change analysis:

```python
# Pearson correlation matrix between all bands
wbe.image_correlation(bands, output_html_file='correlation_matrix.html')

# Spatial autocorrelation (Moran's I) per band
wbe.image_autocorrelation(bands, contiguity='Rooks', output_html_file='autocorr.html')

# Neighbourhood correlation analysis between two images
wbe.image_correlation_neighbourhood_analysis(
    img1=ndvi_t1, img2=ndvi_t2,
    filter_size=11,
    output_html_file='neighbourhood_corr.html'
)

# Simple linear regression of one image on another
slope, intercept, r2 = wbe.image_regression(img1=ndvi_t1, img2=ndvi_t2,
                                              output_html_file='regression.html')
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

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

result = wbe.run_tool(
    'in_season_crop_stress_intervention_planning',
    {
        'ndvi': 'ndvi_latest.tif',
        'canopy_temperature': 'lst_latest.tif',
        'soil_moisture': 'soil_moisture_latest.tif',
        'output_prefix': 'field_07_stress'
    }
)
print(result)
```

> **Note:** This workflow requires a `WbEnvironment` initialized with a valid
> Pro licence.

---

## Complete Land-Cover Classification Workflow

The following end-to-end example uses a sensor bundle for ingestion and a random-forest classifier for land-cover mapping:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.verbose = True

# 1. Open scene as a bundle — works with S2, Landsat 9, PlanetScope, etc.
bundle = wbe.read_bundle('/data/S2B_MSIL2A_20240601T105619_N0510_R094_T32TPT.SAFE')
print(f"Family: {bundle.family}, tile: {bundle.tile_id()}, "
      f"clouds: {bundle.cloud_cover_percent():.1f}%")

# Skip scenes with excessive cloud cover
if bundle.cloud_cover_percent() > 20.0:
    raise RuntimeError("Too cloudy for classification")

# 2. Read bands by key (family-agnostic)
blue  = bundle.read_band('B02')
green = bundle.read_band('B03')
red   = bundle.read_band('B04')
nir   = bundle.read_band('B08')
swir1 = wbe.resample(bundle.read_band('B11'), base_raster=red, method='bilinear')
swir2 = wbe.resample(bundle.read_band('B12'), base_raster=red, method='bilinear')
bands = [blue, green, red, nir, swir1, swir2]

# 3. Mask clouds using SCL
scl = bundle.read_qa_layer('SCL')
cloud_mask = wbe.raster_calculator(
    "if('scl' == 3 or 'scl' == 8 or 'scl' == 9 or 'scl' == 10, 1.0, 0.0)", [scl]
)

# 4. Compute indices
ndvi  = wbe.normalized_difference_index(nir, red)
mndwi = wbe.normalized_difference_index(green, swir1)
nbr   = wbe.normalized_difference_index(nir, swir2)

# 5. Build PCA decorrelation on 6 spectral bands
pc_rasters = wbe.principal_component_analysis(
    input_rasters=bands,
    output_html_file='pca.html',
    num_comp=4,
    standardize=True
)

# 6. Combine spectral bands + indices + PCs into feature stack
feature_stack = bands + [ndvi, mndwi, nbr] + pc_rasters

# 7. Evaluate training separability
training = wbe.read_vector('training_polygons.shp')
wbe.evaluate_training_sites(feature_stack, training, 'CLASS', 'separability.html')

# 8. Train random forest
rf_model_path = wbe.random_forest_classification_fit(
    input_rasters=feature_stack,
    training_data=training,
    class_field_name='CLASS',
    n_trees=500,
    test_proportion=0.25
)

# 9. Predict
classified = wbe.random_forest_classification_predict(feature_stack, rf_model_path)

# 10. Generalise
classified_clean = wbe.generalize_classified_raster(classified, min_class_size=9)
classified_clean = wbe.remove_raster_polygon_holes(classified_clean, threshold=25)

# 11. Save and assess accuracy
wbe.write_raster(classified_clean, 'classified_final.tif', compress=True)
wbe.kappa_index(classified_clean,
                wbe.read_raster('validation_reference.tif'),
                'accuracy_report.html')

# 12. Save a quick-look composite for QC
bundle.true_colour_composite(wbe, output_path='quicklook_truecolour.tif')

print('Land-cover classification complete.')
```

---

## Tips for Effective Remote Sensing Workflows

## Tips

- **Use bundles for scene-level ingestion**: `wbe.read_bundle()` eliminates hardcoded band filenames and works identically across Sentinel-2, Landsat, PlanetScope, and other supported families.
- **Screen cloud cover before processing**: Check `bundle.cloud_cover_percent()` before loading all bands to skip unusable scenes early in a batch workflow.
- **Atmospheric correction is iterative**: Raw digital numbers and even surface reflectance products may still carry illumination and atmospheric artefacts. Histogram matching across scenes is a simple relative normalisation that works well when no reference spectrum is available.
- **Align band resolution before stacking**: Sentinel-2 delivers 10 m (Blue, Green, Red, NIR) and 20 m (Red Edge, SWIR) bands. Upscale the 20 m bands to 10 m using bilinear resampling before combining them in a feature stack.
- **Always mask clouds and cloud shadows**: Use quality bands (Sentinel-2 SCL or Landsat QA_PIXEL) and apply bitwise tests via `raster_calculator()` — e.g. `"('qa' & 0b1000) != 0"` — to isolate valid pixels.
- **Balance training data**: Training datasets often over-represent dominant classes (e.g., forest) relative to rare classes (e.g., wetland). Sample training polygons proportional to expected class area, or oversample rare classes to improve model accuracy.
- **Check spectral separability**: If the evaluate_training_sites report shows Jeffries-Matusita distance < 1.5 between any two classes, consider merging classes or collecting more spectrally diverse training polygons.
- **Use time-series techniques for change detection**: Multi-date classifications can show spurious change due to atmospheric variation. Spectral indices are more robust; consider computing NDVI or NDWI time series and detecting change directly in index space.
- **Memory is your bottleneck**: Large multispectral stacks fill RAM quickly. Use `read_raster_band_by_band()` or out-of-memory processing for scenes > 1 GB.
