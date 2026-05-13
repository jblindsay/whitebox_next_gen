# Remote Sensing Analysis

Remote sensing workflows in WbW-QGIS cover multispectral and hyperspectral
image analysis: spectral index computation, image enhancement, principal
component analysis (PCA), segmentation, and change detection.

This chapter is aligned with the Python and R manuals while staying focused on
QGIS Processing Toolbox execution patterns.

---

## Core Concepts You Should Know First

- Spectral bands: Wavelength-specific image channels (for example blue, red,
  NIR, SWIR) used to separate land-cover materials.
- Spectral indices: Band combinations that highlight specific targets, such as
  NDVI (vegetation), NDWI (water), and NBR (burn severity).
- Spatial resolution: Pixel size affects detail and detectability of features.
- Temporal resolution: Revisit interval controls change-detection sensitivity.
- Atmospheric and cloud effects: Compare like-with-like by masking clouds and
  shadows and using corrected imagery where possible.
- Change detection: Compare index or class outputs across acquisition dates.
- Dimensionality reduction: PCA reduces band redundancy before segmentation or
  classification.

---

## Typical Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| image_t1.tif | Multiband GeoTIFF | Time-1 scene, reflectance preferred |
| image_t2.tif | Multiband GeoTIFF | Time-2 scene, same sensor/preprocessing |
| cloud_mask_t1.tif | Raster | Optional cloud or QA-derived mask |
| cloud_mask_t2.tif | Raster | Optional cloud or QA-derived mask |

---

## End-to-End Workflow

### Step 1 - Quality Check and Harmonize Inputs

Before analysis:

- Confirm both scenes use the same CRS, grid, and pixel size.
- Confirm band order and data scale (for example 0-1 reflectance or scaled
  integer reflectance).
- Mask clouds/shadows using QA products or Raster Calculator conditions.

Use QGIS Raster menu tools as needed:

- Align Raster
- Warp (Reproject)
- Raster Calculator

---

### Step 2 - Build Key Spectral Indices

Process both dates with the same settings.

Processing Toolbox -> Whitebox Workflows -> Remote Sensing:

- NDVI
- Normalized Difference Index (for NDWI, NBR, NDSI, NDBI patterns)

Recommended outputs:

- ndvi_t1.tif, ndvi_t2.tif
- ndwi_t1.tif, ndwi_t2.tif
- nbr_t1.tif, nbr_t2.tif

Example NDVI run:

| Parameter | Value |
|-----------|-------|
| Input image | image_t1.tif |
| NIR band | sensor-specific (for example 4, 5, or 8 depending on product) |
| Red band | sensor-specific |
| Output | ndvi_t1.tif |

Repeat for time 2.

---

### Step 3 - Create Change Surfaces

Use QGIS Raster Calculator for differencing:

- NDVI change: ndvi_t2 - ndvi_t1
- NBR change: nbr_t2 - nbr_t1

Then classify into practical bins (loss, stable, gain) using:

- Whitebox Workflows -> Raster Analysis -> Reclass
- or Raster Calculator threshold expressions

Suggested interpretation for NDVI change:

| Class | Threshold |
|-------|-----------|
| Strong loss | < -0.20 |
| Moderate loss | -0.20 to -0.10 |
| Stable | -0.10 to 0.10 |
| Moderate gain | 0.10 to 0.20 |
| Strong gain | > 0.20 |

---

### Step 4 - Dimensionality Reduction (PCA)

Processing Toolbox -> Whitebox Workflows -> Remote Sensing ->
Principal Component Analysis

Use PCA when:

- Bands are highly correlated.
- You need compact inputs for segmentation or clustering.
- You are preparing a classification feature stack.

Inspect output variance/eigenvalue diagnostics and retain only the components
needed for most variance.

---

### Step 5 - Segmentation and Classification Prep

Processing Toolbox -> Whitebox Workflows -> Remote Sensing ->
Image Segmentation

Typical tuning:

- Lower threshold -> more, smaller segments
- Higher threshold -> fewer, larger segments
- Minimum segment size removes speckle

After segmentation:

- Optionally polygonize segment rasters in QGIS.
- Join zonal metrics from index layers.
- Use resulting segment features for training/validation workflows.

---

## QGIS Python Console Equivalent

Use this pattern for reproducible batch processing in QGIS:

```python
import processing

img_t1 = '/data/image_t1.tif'
img_t2 = '/data/image_t2.tif'

for label, img in [('t1', img_t1), ('t2', img_t2)]:
    processing.run('whitebox_workflows:ndvi', {
        'input': img,
        'nir_band': 4,
        'red_band': 3,
        'output': f'/data/ndvi_{label}.tif',
    })

processing.run('qgis:rastercalculator', {
    'EXPRESSION': '"ndvi_t2@1" - "ndvi_t1@1"',
    'LAYERS': ['/data/ndvi_t1.tif', '/data/ndvi_t2.tif'],
    'OUTPUT': '/data/ndvi_change.tif',
})

processing.run('whitebox_workflows:principal_component_analysis', {
    'input': '/data/image_t1.tif',
    'num_comp': 4,
    'output': '/data/pca_t1.tif',
})
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Index values are outside expected ranges | Wrong bands or scale mismatch | Verify band mapping and value scale |
| Apparent change is mostly cloud edges | Missing cloud/shadow masking | Mask QA classes before differencing |
| PCA output looks unstable | NoData included in stats | Mask NoData consistently |
| Segmentation over-merges features | Threshold too high | Lower threshold and retest |
| Change map is noisy | Different spatial grids or radiometry | Align rasters and normalize radiometry |

---

## Validation Checklist

- [ ] Inputs are co-registered and in a common CRS.
- [ ] Band assignments match sensor metadata.
- [ ] Cloud/shadow/no-data masking applied consistently across dates.
- [ ] Index histograms are plausible for local land cover.
- [ ] Change classes were reviewed visually against source imagery.
- [ ] PCA/segmentation parameters were documented for reproducibility.
