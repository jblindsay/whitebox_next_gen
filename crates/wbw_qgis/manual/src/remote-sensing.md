# Remote Sensing Analysis

Remote sensing workflows transform multispectral and hyperspectral imagery
into interpretable thematic products: spectral indices, vegetation and soil
maps, change detection results, and classified land-cover rasters.
WbW-QGIS surfaces these tools in the Processing Toolbox alongside standard
QGIS raster operations.

This chapter demonstrates a complete vegetation-mapping workflow using
multispectral imagery.

---

## Key Concepts

- **Band**: A single-wavelength image channel. Common bands: Blue (B), Green
  (G), Red (R), Red-Edge (RE), Near Infrared (NIR), Short-Wave Infrared (SWIR).
- **Spectral index**: A band ratio or combination designed to highlight a
  target surface type. Examples: NDVI (vegetation), NDWI (water), NDSI (snow),
  SAVI (soil-adjusted vegetation).
- **Atmosphericaly corrected reflectance**: Raw DN values converted to
  surface reflectance. Required for reliable index thresholding and
  multi-date comparisons.
- **Image segmentation**: Partitioning an image into homogeneous regions.
  Segments can be used as classification units or for object-based analysis.
- **Change detection**: Comparing two images acquired at different times to
  identify areas of changed land cover.
- **Principal Component Analysis (PCA)**: Linear transform that concentrates
  variance into uncorrelated bands. Useful for compression and feature
  extraction before classification.

---

## End-to-End Workflow: NDVI, Thresholding, and Change Detection

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `image_t1.tif` | Multiband GeoTIFF | Surface reflectance, time 1 |
| `image_t2.tif` | Multiband GeoTIFF | Surface reflectance, time 2 |
| Band mapping | — | NIR = band 4, Red = band 3 (Sentinel-2 / Landsat convention) |

---

### Step 1 — Inspect Image Statistics

Before any analysis confirm that the input imagery has expected statistics.

**Processing Toolbox → Raster Analysis → `Raster Layer Statistics`** (QGIS
native)

Or use the QGIS **Layer Properties → Histogram** to inspect per-band
distribution. Reflectance imagery should have values in 0–1 (float) or
0–10000 (scaled integer). DN-only imagery needs atmospheric correction before
spectral indices.

---

### Step 2 — Compute NDVI

**Processing Toolbox → Whitebox Workflows → Remote Sensing →
`NDVI`**

| Parameter | Recommended value |
|-----------|------------------|
| Input image | `image_t1.tif` |
| NIR band | `4` |
| Red band | `3` |
| Output | `ndvi_t1.tif` |

NDVI output range: –1 to +1. Active vegetation: > 0.3. Bare soil: 0.1–0.2.
Water: < 0.

Repeat for `image_t2.tif` → `ndvi_t2.tif`.

---

### Step 3 — Threshold Vegetation

Convert the NDVI raster into a binary vegetation mask.

**Processing Toolbox → Whitebox Workflows → Raster Analysis →
`Reclass From File`** or use QGIS **Raster Calculator**:

```
"ndvi_t1@1" >= 0.3
```

Output: `vegetation_mask_t1.tif` (1 = vegetated, 0 = non-vegetated).

---

### Step 4 — Compute NDVI Change

Subtract time-1 NDVI from time-2 NDVI to produce a change magnitude raster.

**QGIS Raster Calculator:**

```
"ndvi_t2@1" - "ndvi_t1@1"
```

Output: `ndvi_change.tif`. Positive values indicate vegetation gain;
negative values indicate vegetation loss.

Apply a symmetric diverging colour ramp (red–white–green) centred on 0.

---

### Step 5 — Identify Significant Change Areas

**Processing Toolbox → Whitebox Workflows → Raster Analysis →
`Reclass`** (or Raster Calculator threshold)

| Class | NDVI change threshold | Interpretation |
|-------|----------------------|----------------|
| –2 | < –0.20 | Significant vegetation loss |
| –1 | –0.20 to –0.10 | Moderate vegetation loss |
| 0 | –0.10 to +0.10 | No significant change |
| +1 | +0.10 to +0.20 | Moderate vegetation gain |
| +2 | > +0.20 | Significant vegetation gain |

---

## Python Console Equivalent

```python
import processing

img_t1 = '/data/image_t1.tif'
img_t2 = '/data/image_t2.tif'

# NDVI for both dates
for label, img in [('t1', img_t1), ('t2', img_t2)]:
    processing.run('whitebox_workflows:ndvi', {
        'input': img,
        'nir_band': 4,
        'red_band': 3,
        'output': f'/data/ndvi_{label}.tif',
    })

# NDVI change via Raster Calculator
processing.run('qgis:rastercalculator', {
    'EXPRESSION': '"ndvi_t2@1" - "ndvi_t1@1"',
    'LAYERS': ['/data/ndvi_t1.tif', '/data/ndvi_t2.tif'],
    'OUTPUT': '/data/ndvi_change.tif',
})

print("Change detection complete.")
```

---

## Advanced: PCA for Dimensionality Reduction

PCA is useful before unsupervised classification to reduce correlated band
redundancy.

**Processing Toolbox → Whitebox Workflows → Remote Sensing →
`Principal Component Analysis`**

| Parameter | Recommended value |
|-----------|------------------|
| Input image | `image_t1.tif` |
| Number of components | `4` (captures > 95 % variance in most 8-band images) |
| Output | `pca_t1.tif` |

```python
processing.run('whitebox_workflows:principal_component_analysis', {
    'input': '/data/image_t1.tif',
    'num_comp': 4,
    'output': '/data/pca_t1.tif',
})
```

Inspect the eigenvalue table in the tool output log to confirm how many
components are needed to explain > 95 % of the variance.

---

## Advanced: Image Segmentation

Object-based analysis begins with segmentation into spectrally homogeneous
regions.

**Processing Toolbox → Whitebox Workflows → Remote Sensing →
`Image Segmentation`**

| Parameter | Recommended value |
|-----------|------------------|
| Input image | `image_t1.tif` (or PCA output) |
| Threshold | `25.0` (lower = smaller segments) |
| Minimum segment size (px) | `50` |
| Output | `segments_t1.tif` |

Use QGIS **Polygonize** to convert the integer segment raster to vector
polygons for attribute extraction and classification.

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| NDVI values outside –1 to +1 | DN imagery rather than reflectance | Convert DN to reflectance before computing index |
| Change map shows artefacts at swath edges | Different acquisition geometries or BRDF | Normalise images before differencing |
| PCA produces striped output | No-data mixed into statistics | Mask NoData before running PCA |
| Segmentation produces single giant segment | Threshold too high | Halve threshold and rerun |
| Band order mismatch | Tool expects specific band numbering | Verify band order in Layer Properties → Source |

---

## Validation Checklist

- [ ] Input imagery is atmospherically corrected (reflectance, not DN).
- [ ] Band assignments match sensor band order (confirm in metadata).
- [ ] NDVI histogram peaks in expected range for land cover type.
- [ ] Change map artefacts are not co-located with cloud/shadow masks.
- [ ] Segment boundaries visually match spectral boundaries in original image.
- [ ] PCA output eigenvalue table confirms component count captures > 95 % variance.
