# LiDAR Processing

LiDAR point clouds are the highest-resolution elevation data source available
for most practitioners. WbW-QGIS exposes the full Whitebox LiDAR pipeline —
from quality assurance through ground classification, surface modelling, and
height normalisation — directly in the QGIS Processing Toolbox.

This chapter walks through a complete bare-earth and canopy-height workflow
starting from a raw LAS/LAZ file.

---

## Key Concepts

- **Point cloud**: A set of 3-D coordinates (X, Y, Z) plus attributes
  (intensity, return number, classification, scan angle, etc.) acquired by
  laser scanning from airborne or terrestrial platforms.
- **LAS/LAZ**: The industry-standard binary format for point clouds. LAZ is
  a losslessly compressed variant. COPC (Cloud-Optimised Point Cloud) is a
  tiled LAZ variant for efficient streaming.
- **Classification codes**: Numeric labels assigned to points indicating
  surface type (1 = unclassified, 2 = ground, 3–5 = vegetation, 6 = building,
  etc. per ASPRS convention).
- **DTM**: Digital Terrain Model — a raster surface interpolated from
  ground-classified points only.
- **DSM**: Digital Surface Model — a raster surface from first returns,
  representing the tops of all objects (vegetation, buildings).
- **CHM**: Canopy Height Model — DSM minus DTM, representing object height
  above ground.
- **Height above ground (HAG)**: Per-point elevation relative to the
  interpolated ground surface. Enables classification of vegetation returns by
  height tier.

---

## End-to-End Workflow: DTM, DSM, and Canopy Height Model

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `cloud.laz` | LAZ point cloud | Any ASPRS LAS version 1.0–1.4 |

---

### Step 1 — Point Cloud Quality Check

**Processing Toolbox → Whitebox Workflows → LiDAR →
`LiDAR Point Stats`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud.laz` |
| Output | `cloud_stats.html` (HTML report) |

Review the report for:
- Point density (pts/m²)
- Classification distribution (% ground, vegetation, unclassified)
- Z range and intensity histogram
- Scan angle range (should be ±20° for most airborne missions)

---

### Step 2 — Thin High-Density Files (Optional)

For files > 50 pts/m², thinning reduces processing time with minimal accuracy
loss.

**Processing Toolbox → Whitebox Workflows → LiDAR →
`LiDAR Thin`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud.laz` |
| Resolution | `0.5` (metres) |
| Retain ground points | ✓ enabled |
| Output | `cloud_thin.laz` |

---

### Step 3 — Classify Ground Points

If the input file has all points as unclassified (class 1), classify ground
returns before surface modelling.

**Processing Toolbox → Whitebox Workflows → LiDAR →
`LiDAR Ground Point Filter`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud.laz` (or `cloud_thin.laz`) |
| Radius (m) | `2.0` |
| Minimum slope (°) | `5.0` |
| Maximum slope (°) | `85.0` |
| Terrain type | `Normal` |
| Output | `cloud_classified.laz` |

> For complex terrain (steep slopes, dense vegetation), increase radius to
> `4.0` and reduce minimum slope to `2.0`.

---

### Step 4 — Build DTM from Ground Points

**Processing Toolbox → Whitebox Workflows → LiDAR →
`LiDAR IDW Interpolation`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud_classified.laz` |
| IDW weight | `2.0` |
| Search radius (m) | `2.5` |
| Minimum number of points | `3` |
| Exclusion classes | *(leave empty to use ground points only)* |
| Returns | `Last` |
| Point classes included | `2` (ground) |
| Grid resolution | `0.5` |
| Output | `dtm.tif` |

Alternatively, use **`LiDAR TIN Gridding`** for faster interpolation on
uniformly distributed clouds.

---

### Step 5 — Build DSM from First Returns

**Processing Toolbox → Whitebox Workflows → LiDAR →
`LiDAR IDW Interpolation`** (second pass)

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud_classified.laz` |
| Returns | `First` |
| Point classes included | *(all — leave blank)* |
| Grid resolution | `0.5` |
| Output | `dsm.tif` |

---

### Step 6 — Canopy Height Model

Subtract DTM from DSM using the QGIS Raster Calculator, or use the dedicated
CHM tool.

**Processing Toolbox → Whitebox Workflows → LiDAR →
`Canopy Height Model`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud_classified.laz` |
| DTM raster | `dtm.tif` |
| Grid resolution | `0.5` |
| Output | `chm.tif` |

Apply a stretch from 0 to the 98th percentile height value. Negative cells
(< 0) indicate DTM–DSM interpolation artefacts; clamp to 0 in post-processing.

---

### Step 7 — Height Above Ground Normalisation

Assign a HAG value to every point for per-return vertical stratification
analysis.

**Processing Toolbox → Whitebox Workflows → LiDAR →
`Height Above Ground`**

| Parameter | Recommended value |
|-----------|------------------|
| Input LiDAR file | `cloud_classified.laz` |
| Output | `cloud_hag.laz` |

The tool sets the Z coordinate of each point to its height above the
interpolated ground surface. Ground points are set to 0.

---

## Python Console Equivalent

```python
import processing

cloud = '/data/cloud.laz'

# Step 1: stats / QA
processing.run('whitebox_workflows:lidar_point_stats', {
    'input': cloud,
    'output': '/data/cloud_stats.html',
})

# Step 3: ground classification
processing.run('whitebox_workflows:lidar_ground_point_filter', {
    'input': cloud,
    'radius': 2.0,
    'min_slope': 5.0,
    'max_slope': 85.0,
    'terrain_type': 'Normal',
    'output': '/data/cloud_classified.laz',
})

# Step 4: DTM
processing.run('whitebox_workflows:lidar_idw_interpolation', {
    'input': '/data/cloud_classified.laz',
    'parameter': 'elevation',
    'returns': 'Last',
    'classes_included': '2',
    'weight': 2.0,
    'radius': 2.5,
    'min_points': 3,
    'resolution': 0.5,
    'output': '/data/dtm.tif',
})

# Step 5: DSM
processing.run('whitebox_workflows:lidar_idw_interpolation', {
    'input': '/data/cloud_classified.laz',
    'parameter': 'elevation',
    'returns': 'First',
    'classes_included': '',
    'weight': 2.0,
    'radius': 2.5,
    'min_points': 3,
    'resolution': 0.5,
    'output': '/data/dsm.tif',
})

# Step 6: CHM
processing.run('whitebox_workflows:canopy_height_model', {
    'input': '/data/cloud_classified.laz',
    'dtm': '/data/dtm.tif',
    'resolution': 0.5,
    'output': '/data/chm.tif',
})

# Step 7: HAG
processing.run('whitebox_workflows:height_above_ground', {
    'input': '/data/cloud_classified.laz',
    'output': '/data/cloud_hag.laz',
})

print("LiDAR pipeline complete.")
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| DTM has large NoData holes | Ground point density too low | Increase IDW search radius or use TIN gridding |
| CHM has negative values | DTM higher than DSM in flat/water areas | Clamp CHM ≥ 0 with Raster Calculator after generation |
| Classification codes all zero after processing | Input was LAS point format 6–10 and legacy writer bug (see known issues) | Use WbW Next Gen pipeline — classification is preserved correctly |
| Ground filter over-segments in steep terrain | Slope parameters too restrictive | Increase max slope to 85° and radius to 4 m |
| LiDAR stats report extreme Z values | Outlier high/low points present | Run `LiDAR Remove Outliers` before classification |

---

## Validation Checklist

- [ ] Point stats report shows expected classification distribution (> 5 % ground points).
- [ ] DTM has no large NoData holes in vegetated areas.
- [ ] DTM is smooth with no pits deeper than 1–2 m.
- [ ] DSM ≥ DTM across the entire overlap extent.
- [ ] CHM values are 0 on roads and bare ground.
- [ ] HAG-normalised cloud has all ground points at Z ≈ 0 (±0.1 m).
