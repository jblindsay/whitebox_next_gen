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
        ndvi = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir_band, red_band)
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
swir1 = wbe.remote_sensing.enhancement_contrast.resample(wbe.read_raster('B11_20m.tif'), base_raster=red, method='bilinear')
swir2 = wbe.remote_sensing.enhancement_contrast.resample(wbe.read_raster('B12_20m.tif'), base_raster=red, method='bilinear')
```

---

## Cloud and No-Data Masking

Before any analysis, mask clouds, cloud shadows, and saturated pixels.

```python
# Sentinel-2 SCL classes: 3=cloud shadow, 8=medium cloud, 9=high cloud, 10=thin cirrus
scl = bundle.read_qa_layer('SCL')
cloud_free_red = wbe.raster.general.raster_calculator(
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
ndvi = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir, red)
wbe.write_raster(ndvi, 'ndvi.tif', compress=True)
```

### Common Water, Snow, and Urban Indices

```python
# NDWI (McFeeters 1996) — open water; Green/NIR
ndwi  = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(green, nir)

# MNDWI — better for urban areas; Green/SWIR1
mndwi = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(green, swir1)

# NBR — fire severity and post-fire recovery; NIR/SWIR2
nbr   = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir, swir2)

# NDSI — snow and ice; Green/SWIR1
ndsi  = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(green, swir1)

# NDBI — normalised built-up index; SWIR1/NIR
ndbi  = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(swir1, nir)
```

### Enhanced Vegetation Index (EVI)

EVI reduces soil background noise and atmospheric effects that affect NDVI in dense canopies:

$$EVI = 2.5 \cdot \frac{NIR - Red}{NIR + 6 \cdot Red - 7.5 \cdot Blue + 1}$$

```python
evi = wbe.raster.general.raster_calculator(
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
savi = wbe.raster.general.raster_calculator(
    expression="('nir' - 'red') * (1.0 + 0.5) / ('nir' + 'red' + 0.5)",
    input_rasters=[nir, red]
)
```

---

## Image Enhancement and Contrast Stretching

Raw digital numbers often have narrow histogram ranges that make visualisation difficult. WbW-Py provides several enhancement methods:

```python
# Percentage contrast stretch — clips top and bottom 2% of values
enhanced = wbe.remote_sensing.enhancement_contrast.percentage_contrast_stretch(red, clip_tail=2.0)

# Standard deviation stretch — maps ±2σ to display range
sd_stretched = wbe.remote_sensing.enhancement_contrast.standard_deviation_contrast_stretch(red, num_std_dev=2.0)

# Gaussian contrast stretch
gauss = wbe.remote_sensing.enhancement_contrast.gaussian_contrast_stretch(red, num_std_dev=2.0)

# Sigmoidal stretch — soft S-curve useful for optical imagery
sig = wbe.remote_sensing.enhancement_contrast.sigmoidal_contrast_stretch(red, cutoff=0.4, gain=4.0)

# Min-max linear stretch
linear = wbe.remote_sensing.enhancement_contrast.min_max_contrast_stretch(red, min_val=0.0, max_val=255.0)

# Gamma correction — lighten (gamma<1) or darken (gamma>1) an image
gamma = wbe.remote_sensing.enhancement_contrast.gamma_correction(red, gamma=0.6)
```

Histogram matching normalises a source image to match the distribution of a reference image — invaluable when mosaicking scenes acquired on different dates or under different atmospheric conditions:

```python
# Match the histogram of a target scene to a reference scene
matched = wbe.remote_sensing.enhancement_contrast.histogram_matching(source_scene, reference_scene)

# You can also match between two images from the same sensor
matched2 = wbe.remote_sensing.enhancement_contrast.histogram_matching_two_images(img1, img2)
```

### IHS Colour Space Transformations

Intensity-Hue-Saturation (IHS) space separates brightness from colour, enabling selective sharpening or enhancement without hue distortion:

```python
# Convert RGB composite to IHS
(intensity, hue, saturation) = wbe.remote_sensing.enhancement_contrast.rgb_to_ihs(red, green, blue)

# Enhance the intensity channel
enhanced_i = wbe.remote_sensing.enhancement_contrast.standard_deviation_contrast_stretch(intensity, num_std_dev=2.0)

# Convert back to RGB
(r2, g2, b2) = wbe.remote_sensing.enhancement_contrast.ihs_to_rgb(enhanced_i, hue, saturation)
```

### Panchromatic Sharpening (Pan-Sharpening)

When a high-resolution panchromatic band accompanies lower-resolution multispectral data, IHS pan-sharpening fuses the two:

```python
panchromatic = bundle.read_band('B8A')   # Sentinel-2 red-edge as panchromatic proxy
# Landsat: panchromatic = bundle.read_band('PAN')   # 15 m Landsat panchromatic

(sharp_r, sharp_g, sharp_b) = wbe.remote_sensing.enhancement_contrast.panchromatic_sharpening(
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
smoothed = wbe.remote_sensing.filters.gaussian_filter(red, sigma=1.0)

# Bilateral filter — edge-preserving smoothing
bilateral = wbe.remote_sensing.filters.bilateral_filter(red, sigma_dist=2.0, sigma_int=25.0)

# High-pass filter — enhances fine texture
texture = wbe.remote_sensing.filters.high_pass_filter(red, filter_size_x=5, filter_size_y=5)

# Unsharp masking — sharpens blurred imagery
sharp = wbe.remote_sensing.filters.unsharp_masking(red, sigma=3.0, amount=1.0, threshold=0)

# Edge detection — Sobel gradient magnitude
edges_h = wbe.remote_sensing.edge_feature_detection.sobel_filter(red, variant='3x3')

# Canny edge detector — more precise edge localisation
canny_edges = wbe.remote_sensing.edge_feature_detection.canny_edge_detection(red, sigma=0.5,
                                        low_threshold=5.0, high_threshold=15.0)

# General-purpose GLCM texture (multiband output)
# Band names are recorded in raster metadata as band_1_name, band_2_name, ...
glcm = wbe.remote_sensing.filters.glcm_texture(
    red,
    window_size=9,
    distance=1,
    angles="0,45,90,135",
    features="contrast,homogeneity,entropy",
    direction_aggregation="mean",
    levels=32,
    output_path="glcm_texture.tif",
)
```

---

## Principal Component Analysis (PCA)

PCA is an essential step in multispectral and hyperspectral analysis. It rotates correlated bands into decorrelated principal components (PCs) ordered by descending variance. The first few PCs typically capture most scene variance and are used to reduce data dimensionality before classification.

```python
# Run PCA on all six bands
bands = [blue, green, red, nir, swir1]
pca_result = wbe.raster.general.principal_component_analysis(
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
pc1_modified = wbe.remote_sensing.enhancement_contrast.standard_deviation_contrast_stretch(pc1, num_std_dev=2.0)
modified_components = [pc1_modified, pc2, pc3, pc4]

reconstructed_bands = wbe.raster.general.inverse_pca(
    modified_components,
    original_rasters=bands
)
```

---

## Image Segmentation and Object-Based Analysis

Object-based image analysis (OBIA) groups pixels into meaningful spatial units (segments) before classifying them. This is superior to pixel-based classification for high-resolution imagery where individual objects span many pixels.

```python
# Inspect the dedicated OBIA grouping under remote sensing.
print(wbe.remote_sensing.obia.list_tools())

# 1) Create baseline segments (SLIC-like open-core baseline).
segments = wbe.remote_sensing.obia.segment_slic_superpixels(
    inputs=[red, green, nir],
    region_size=18,
    compactness=12.0,
    output='segments_slic.tif'
)

# 2) Merge small regions for cleaner object topology.
segments_clean = wbe.remote_sensing.obia.segments_merge_small_regions(
    segments=segments,
    min_size=12,
    method='longest',
    output='segments_clean.tif'
)

# 3) Extract object-level features.
spectral_csv = wbe.remote_sensing.obia.object_features_spectral_basic(
    segments=segments_clean,
    inputs=[red, green, nir],
    output='object_features_spectral.csv'
)

shape_csv = wbe.remote_sensing.obia.object_features_shape_basic(
    segments=segments_clean,
    output='object_features_shape.csv'
)

texture_csv = wbe.remote_sensing.obia.object_features_texture_glcm_basic(
    segments=segments_clean,
    input=nir,
    levels=16,
    output='object_features_texture.csv'
)

# 4) Train/apply object RF model using segment-level training labels.
pred_csv = wbe.remote_sensing.obia.classify_objects_random_forest(
    features='object_features_all.csv',
    training='training_segments.csv',
    class_field='class',
    output='object_predictions.csv'
)

# 5) Evaluate object-level accuracy.
report = wbe.remote_sensing.obia.evaluate_object_classification_accuracy(
    predictions=pred_csv,
    reference='validation_segments.csv',
    output='object_accuracy.json'
)

# Optional one-call baseline pipeline.
outputs = wbe.remote_sensing.obia.obia_pipeline_basic(
    inputs=[red, green, nir],
    training='training_segments.csv',
    output_prefix='obia_field01',
    segment_method='slic',
)
```

This baseline stack is designed to be reproducible and script-friendly. For many projects, the one-call `obia_pipeline_basic` run is the fastest path to a validated Phase 1 workflow.

### Advanced OBIA Capabilities (Open Tier)

The OBIA stack now includes advanced capabilities in open tier. These tools are available under `wbe.remote_sensing.obia.*` and are grouped here by workflow purpose.

Segmentation and scale control:
- `segment_watershed_markers`
- `segment_multiresolution_hierarchical`
- `segment_scale_parameter_optimizer`
- `segments_split_low_cohesion`

Object conversion and interoperability:
- `segments_to_polygons`
- `polygons_to_segments`

Advanced feature engineering:
- `object_features_context_neighbors`
- `object_features_topology_relations`

Advanced object classification:
- `classify_objects_svm`
- `classify_objects_ensemble_pro`
- `classify_objects_rules_basic`
- `classify_objects_rules_hierarchical`
- `object_class_probability_maps`
- `object_uncertainty_diagnostics_pro`

Hierarchy management and propagation:
- `build_object_hierarchy_multiscale`
- `propagate_labels_across_hierarchy`

Post-processing and quality:
- `objects_enforce_min_mapping_unit`
- `objects_boundary_refinement_pro`
- `evaluate_segmentation_quality_pro`

Workflow orchestration and reporting:
- `obia_batch_orchestrator_pro`
- `obia_audit_report_pro`

```python
# Build multi-scale objects and hierarchy links
hier = wbe.remote_sensing.obia.segment_multiresolution_hierarchical(
    inputs=[red, green, nir],
    coarse_k=900.0,
    fine_k=280.0,
    output_prefix='site01_hier'
)

# Add neighborhood + topology context features for difficult classes
context_csv = wbe.remote_sensing.obia.object_features_context_neighbors(
    segments=hier['segments_fine'],
    output='site01_context.csv'
)
topology_csv = wbe.remote_sensing.obia.object_features_topology_relations(
    segments=hier['segments_fine'],
    output='site01_topology.csv'
)

# Ensemble and rule-hierarchy options
ensemble_pred = wbe.remote_sensing.obia.classify_objects_ensemble_pro(
    features='site01_features_all.csv',
    training='site01_training_segments.csv',
    output='site01_pred_ensemble.csv'
)
rule_pred = wbe.remote_sensing.obia.classify_objects_rules_hierarchical(
    features='site01_features_all.csv',
    rules='site01_rules.csv',
    output='site01_pred_rules.csv'
)

# Probability and uncertainty diagnostics
prob_csv = wbe.remote_sensing.obia.object_class_probability_maps(
    predictions=ensemble_pred,
    output='site01_probabilities.csv'
)
unc_json = wbe.remote_sensing.obia.object_uncertainty_diagnostics_pro(
    probabilities=prob_csv,
    low_conf_threshold=0.7,
    output='site01_uncertainty.json'
)
```

```python
# Batch orchestration + audit report for multi-scene production runs
batch = wbe.remote_sensing.obia.obia_batch_orchestrator_pro(
    jobs=[
        {
            'inputs': ['s1_red.tif', 's1_green.tif', 's1_nir.tif'],
            'training': 's1_training.csv',
            'output_prefix': 'prod/s1',
            'segment_method': 'graph',
        },
        {
            'inputs': ['s2_red.tif', 's2_green.tif', 's2_nir.tif'],
            'training': 's2_training.csv',
            'output_prefix': 'prod/s2',
            'segment_method': 'slic',
        },
    ],
    output='prod/obia_batch_report.json'
)

audit = wbe.remote_sensing.obia.obia_audit_report_pro(
    artifacts=[
        'prod/s1_object_predictions.csv',
        'prod/s2_object_predictions.csv',
        'prod/obia_batch_report.json',
    ],
    output='prod/obia_audit.json'
)
```

The `segments_to_polygons` and `polygons_to_segments` conversion tools are also valuable in edit-and-return workflows where analysts refine object boundaries in vector space and then rasterize updated objects back to segment grids.

---

## Unsupervised Classification

Unsupervised classification groups pixels by spectral similarity without training data, making it useful for exploratory analysis.

### K-Means Clustering

```python
bands = [blue, green, red, nir, swir1]

kmeans_result = wbe.remote_sensing.classification.k_means_clustering(
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
mod_kmeans = wbe.remote_sensing.classification.modified_k_means_clustering(
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
dbscan_result = wbe.raster.general.dbscan(
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
wbe.remote_sensing.classification.evaluate_training_sites(
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
classified_md = wbe.remote_sensing.classification.min_dist_classification(
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
classified_pp = wbe.remote_sensing.classification.parallelepiped_classification(
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

knn_model = wbe.remote_sensing.classification.knn_classification(
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
rf_model = wbe.raster.general.random_forest_classification_fit(
    input_rasters=bands,
    training_data=training_polys,
    class_field_name='CLASS',
    n_trees=500,
    min_samples_leaf=1,
    test_proportion=0.2
)
# rf_model is a path to the saved model file

# Step 2: predict on the full image (can be a different image/date)
classified_rf = wbe.raster.general.random_forest_classification_predict(
    input_rasters=bands,
    model=rf_model
)
wbe.write_raster(classified_rf, 'classified_rf.tif')
```

The fit step prints an accuracy report including the confusion matrix, Kappa coefficient, and overall accuracy.

### Support Vector Machine (SVM) Classification

SVMs find the maximum-margin hyperplane separating classes and are particularly effective with high-dimensional data and small training sets:

```python
classified_svm = wbe.remote_sensing.classification.svm_classification(
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
classified_lr = wbe.remote_sensing.classification.logistic_regression(
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
accuracy = wbe.raster.general.kappa_index(
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
generalised = wbe.remote_sensing.classification.generalize_classified_raster(
    classified=classified_rf,
    min_class_size=5  # minimum patch size in cells
)

# Alternative: merge small patches into most spectrally similar neighbour
generalised2 = wbe.remote_sensing.classification.generalize_with_similarity(
    input_rasters=bands,
    classified=classified_rf,
    min_class_size=5
)

# Remove holes (background pixels encircled by foreground)
no_holes = wbe.conversion.raster_vector_conversion.remove_raster_polygon_holes(
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
ndvi_t1 = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir_t1, red_t1)
ndvi_t2 = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir_t2, red_t2)
ndvi_change = wbe.raster.general.raster_calculator("'t2' - 't1'", [ndvi_t2, ndvi_t1])
wbe.write_raster(ndvi_change, 'ndvi_change.tif', compress=True)
```

### Change Vector Analysis (CVA)

CVA computes the magnitude and direction of change in multidimensional feature space between two dates. The output direction angle indicates the type of change (e.g., vegetation gain vs. urban growth) while magnitude reflects its intensity:

```python
magnitude, direction = wbe.remote_sensing.change_detection.change_vector_analysis(
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
classified_t1 = wbe.raster.general.random_forest_classification_predict(bands_t1, rf_model)
classified_t2 = wbe.raster.general.random_forest_classification_predict(bands_t2, rf_model)

# Produce a change raster (unique code for each from-to class transition)
change_map = wbe.raster.general.raster_calculator(
    "'t1' * 100 + 't2'",
    [classified_t1, classified_t2]
)
wbe.write_raster(change_map, 'change_map.tif')
```

### Advanced Change Detection Tools

The sprint additions expose dedicated change tools that return richer diagnostics than simple raster subtraction:

```python
# Multiband image differencing with optional signed and mask outputs
diff_result = wbe.remote_sensing.change_detection.image_difference_change_detection(
    t1_inputs=[red_t1, nir_t1, swir1_t1],
    t2_inputs=[red_t2, nir_t2, swir1_t2],
    output='img_diff_mag.tif',
    options={
        'mode': 'magnitude',
        'threshold_sigma': 2.0,
        'output_signed': 'img_diff_signed.tif',
        'output_mask': 'img_diff_mask.tif'
    }
)

# Post-classification transition table + remap support
post_class_result = wbe.remote_sensing.change_detection.post_classification_change(
    t1_classified=classified_t1,
    t2_classified=classified_t2,
    output='post_class_transition.tif',
    options={
        'transition_scale': 1000,
        't1_class_remap': {'11': 1, '12': 1},
        't2_class_remap': {'41': 4, '42': 4}
    }
)

# PCA-based change with optional mask/report outputs
pca_change_result = wbe.remote_sensing.change_detection.pca_based_change_detection(
    t1_inputs=[red_t1, nir_t1, swir1_t1],
    t2_inputs=[red_t2, nir_t2, swir1_t2],
    output='pca_change_pc1.tif',
    options={
        'component': 1,
        'threshold_sigma': 2.0,
        'output_mask': 'pca_change_mask.tif',
        'output_report': 'pca_change_report.json'
    }
)
```

### Radiometric and Thermal Emissivity Tools

For physically grounded thermal workflows, run radiometric correction and emissivity estimation before LST:

```python
# Atmospheric haze reduction
dos_result = wbe.remote_sensing.radiometric_correction.dark_object_subtraction(
    inputs=[blue, green, red, nir],
    output='dos_stack.tif',
    options={'percentile': 1.0, 'output_diagnostic_offsets': 'dos_offsets.tif'}
)

# DN to TOA reflectance using bundle metadata where available
toa_result = wbe.remote_sensing.radiometric_correction.dn_to_toa_reflectance(
    inputs=[blue, green, red, nir],
    output='toa_stack.tif',
    options={'sensor_bundle_root': '/data/LC09_L1TP_017030_20240420_20240426_02_T1'}
)

# NDVI-based emissivity + LST products
emiss_result = wbe.remote_sensing.thermal_emissivity.ndvi_based_emissivity(
    red_input=red,
    nir_input=nir,
    output='emissivity.tif'
)

lst_sc_result = wbe.remote_sensing.thermal_emissivity.land_surface_temperature_single_channel(
    thermal_input=wbe.read_raster('LC09_B10.TIF'),
    output='lst_single_channel.tif',
    options={'sensor_bundle_root': '/data/LC09_L1TP_017030_20240420_20240426_02_T1'}
)

lst_sw_result = wbe.remote_sensing.thermal_emissivity.land_surface_temperature_split_window(
    thermal1_input=wbe.read_raster('LC09_B10.TIF'),
    thermal2_input=wbe.read_raster('LC09_B11.TIF'),
    output='lst_split_window.tif',
    options={'emissivity_mean_constant': 0.98, 'emissivity_delta_constant': 0.0}
)
```

### Spectral Analytics and PolSAR Decomposition

The new spectral analytics subcategory covers endmember-driven and denoising workflows:

```python
sam_result = wbe.remote_sensing.spectral_analytics.spectral_angle_mapper(
    inputs=[blue, green, red, nir],
    output='sam_classes.tif',
    options={
        'endmembers': [
            {'name': 'water', 'values': [0.03, 0.02, 0.01, 0.00]},
            {'name': 'veg', 'values': [0.05, 0.10, 0.06, 0.40]}
        ],
        'output_angle': 'sam_angle.tif'
    }
)

cont_result = wbe.remote_sensing.spectral_analytics.continuum_removal(
    inputs=[wbe.read_raster('hyp_b1.tif'), wbe.read_raster('hyp_b2.tif'), wbe.read_raster('hyp_b3.tif')],
    output='continuum_removed.tif',
    options={'wavelengths': [450.0, 550.0, 650.0]}
)

unmix_result = wbe.remote_sensing.spectral_analytics.linear_spectral_unmixing(
    inputs=[blue, green, red, nir],
    output='unmix_frac.tif',
    options={
        'endmembers': [
            {'name': 'soil', 'values': [0.18, 0.20, 0.22, 0.24]},
            {'name': 'veg', 'values': [0.05, 0.09, 0.06, 0.40]}
        ],
        'output_residual': 'unmix_residual.tif'
    }
)

mnf_result = wbe.remote_sensing.spectral_analytics.minimum_noise_fraction(
    inputs=[blue, green, red, nir],
    output='mnf_components.tif',
    options={'num_components': 3, 'output_inverse': 'mnf_inverse.tif'}
)

lib_result = wbe.remote_sensing.spectral_analytics.spectral_library_matching(
    inputs=[blue, green, red, nir],
    output='lib_class.tif',
    options={
        'metric': 'sam',
        'library': [
            {'name': 'water', 'values': [0.03, 0.02, 0.01, 0.00]},
            {'name': 'soil', 'values': [0.18, 0.20, 0.22, 0.24]}
        ],
        'output_score': 'lib_score.tif'
    }
)

# SAR decomposition tools are available under remote_sensing.sar
cp_result = wbe.remote_sensing.sar.cloude_pottier_decomposition(
    inputs=[wbe.read_raster('t11.tif'), wbe.read_raster('t22.tif'), wbe.read_raster('t33.tif')],
    output='cloude_pottier_haa.tif',
    options={'matrix_format': 'diag3'}
)

fd_result = wbe.remote_sensing.sar.freeman_durden_decomposition(
    inputs=[wbe.read_raster('c11.tif'), wbe.read_raster('c22.tif'), wbe.read_raster('c33.tif')],
    output='freeman_durden.tif',
    options={'matrix_format': 'diag3', 'output_clip_mask': 'freeman_clip.tif'}
)
```

---

## Water Extraction and River Mapping

Combining NDWI thresholding with morphological post-processing extracts water bodies:

```python
# Threshold MNDWI — water pixels where mndwi > 0
water = wbe.raster.general.raster_calculator("if('mndwi' > 0.0, 1.0, nodata)", [mndwi])

# Remove small spurious water patches
water_clean = wbe.raster.general.sieve(water, threshold=50)

# Remove islands in water polygons
water_filled = wbe.conversion.raster_vector_conversion.remove_raster_polygon_holes(water_clean, threshold=100)

# Extract river centrelines from the water raster
river_lines = wbe.streams.network_extraction.river_centerlines(water_filled, min_length=5, search_radius=9)
wbe.write_vector(river_lines, 'river_centerlines.shp')
```

---

## Image Correlation and Regression

WbW-Py supports several image correlation and regression tools useful for sensor cross-calibration, quality control, and change analysis:

```python
# Pearson correlation matrix between all bands
wbe.raster.general.image_correlation(bands, output_html_file='correlation_matrix.html')

# Spatial autocorrelation (Moran's I) per band
wbe.raster.general.image_autocorrelation(bands, contiguity='Rooks', output_html_file='autocorr.html')

# Neighbourhood correlation analysis between two images
wbe.raster.local_neighborhood.image_correlation_neighbourhood_analysis(
    img1=ndvi_t1, img2=ndvi_t2,
    filter_size=11,
    output_html_file='neighbourhood_corr.html'
)

# Simple linear regression of one image on another
slope, intercept, r2 = wbe.raster.general.image_regression(img1=ndvi_t1, img2=ndvi_t2,
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
swir1 = wbe.remote_sensing.enhancement_contrast.resample(bundle.read_band('B11'), base_raster=red, method='bilinear')
swir2 = wbe.remote_sensing.enhancement_contrast.resample(bundle.read_band('B12'), base_raster=red, method='bilinear')
bands = [blue, green, red, nir, swir1, swir2]

# 3. Mask clouds using SCL
scl = bundle.read_qa_layer('SCL')
cloud_mask = wbe.raster.general.raster_calculator(
    "if('scl' == 3 or 'scl' == 8 or 'scl' == 9 or 'scl' == 10, 1.0, 0.0)", [scl]
)

# 4. Compute indices
ndvi  = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir, red)
mndwi = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(green, swir1)
nbr   = wbe.remote_sensing.enhancement_contrast.normalized_difference_index(nir, swir2)

# 5. Build PCA decorrelation on 6 spectral bands
pc_rasters = wbe.raster.general.principal_component_analysis(
    input_rasters=bands,
    output_html_file='pca.html',
    num_comp=4,
    standardize=True
)

# 6. Combine spectral bands + indices + PCs into feature stack
feature_stack = bands + [ndvi, mndwi, nbr] + pc_rasters

# 7. Evaluate training separability
training = wbe.read_vector('training_polygons.shp')
wbe.remote_sensing.classification.evaluate_training_sites(feature_stack, training, 'CLASS', 'separability.html')

# 8. Train random forest
rf_model_path = wbe.raster.general.random_forest_classification_fit(
    input_rasters=feature_stack,
    training_data=training,
    class_field_name='CLASS',
    n_trees=500,
    test_proportion=0.25
)

# 9. Predict
classified = wbe.raster.general.random_forest_classification_predict(feature_stack, rf_model_path)

# 10. Generalise
classified_clean = wbe.remote_sensing.classification.generalize_classified_raster(classified, min_class_size=9)
classified_clean = wbe.conversion.raster_vector_conversion.remove_raster_polygon_holes(classified_clean, threshold=25)

# 11. Save and assess accuracy
wbe.write_raster(classified_clean, 'classified_final.tif', compress=True)
wbe.raster.general.kappa_index(classified_clean,
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
