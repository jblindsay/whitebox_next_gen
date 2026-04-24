# Raster Analysis

General raster analysis covers a broad set of cell-based operations: local
algebra on single rasters, focal statistics across neighbourhoods, zonal
summaries within polygon regions, reclassification, and suitability scoring.
These tools form the computational backbone of many GIS modelling workflows.

This chapter demonstrates an environmental suitability analysis that combines
multiple raster datasets through reclassification and weighted overlay.

---

## Key Concepts

- **Local operations**: Applied to each cell independently — arithmetic, logic,
  trigonometry, conditional assignment. Result depends only on the cell's own
  value (and corresponding cells in other input rasters).
- **Focal operations**: Applied to each cell using values within a spatial
  neighbourhood (kernel). Examples: focal mean, focal max, focal standard
  deviation.
- **Zonal statistics**: Aggregate raster values by zone boundaries defined by
  a second raster or vector polygon layer.
- **Reclassification**: Map old cell values to new values via a lookup table or
  range intervals. Core step in suitability and habitat modelling.
- **Weighted overlay**: Combine multiple reclassified factor rasters using
  factor-specific weights. The weighted sum produces a composite suitability
  score.
- **NoData / null handling**: Cells with NoData propagate through most local
  operations. Ensure all input rasters share the same extent, resolution, and
  NoData mask before combining them.

---

## End-to-End Workflow: Multi-Criteria Habitat Suitability

This workflow scores terrain for habitat suitability using slope, TWI, and
distance from water as factors.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `slope.tif` | GeoTIFF raster | Degrees, from terrain analysis |
| `twi.tif` | GeoTIFF raster | Topographic Wetness Index |
| `streams.tif` | GeoTIFF raster | Binary stream network |

All rasters must share the same projected CRS, extent, and cell size before
being combined. Use **`Snap Raster Extents`** or QGIS **`Warp (Reproject)`**
to align if needed.

---

### Step 1 — Compute Distance from Water

**Processing Toolbox → Whitebox Workflows → GIS Analysis →
`Euclidean Distance`**

| Parameter | Recommended value |
|-----------|------------------|
| Input feature | `streams.tif` |
| Output | `dist_water.tif` |

Cells closer to water receive smaller values. This will be reclassified
so that proximity = higher suitability.

---

### Step 2 — Reclassify Slope Factor

Assign suitability scores 1–5 to slope ranges.

**Processing Toolbox → Whitebox Workflows → Raster Analysis →
`Reclass`**

| Class | Slope range (°) | Suitability score |
|-------|----------------|------------------|
| 1 | > 30 | 1 (unsuitable) |
| 2 | 20–30 | 2 |
| 3 | 10–20 | 3 |
| 4 | 5–10 | 4 |
| 5 | 0–5 | 5 (most suitable) |

Use **`Reclass From File`** with a two-column table file (old value; new value)
or set up intervals in the tool dialogue.

| Parameter | Recommended value |
|-----------|------------------|
| Input raster | `slope.tif` |
| Reclass intervals file | `slope_reclass.txt` |
| Output | `slope_reclass.tif` |

---

### Step 3 — Reclassify TWI Factor

| Class | TWI range | Suitability score |
|-------|----------|------------------|
| 1 | < 4 | 1 |
| 2 | 4–6 | 2 |
| 3 | 6–8 | 3 |
| 4 | 8–10 | 4 |
| 5 | > 10 | 5 |

```
Processing Toolbox → Whitebox Workflows → Raster Analysis → Reclass
Input: twi.tif → Output: twi_reclass.tif
```

---

### Step 4 — Reclassify Distance from Water

Closer = more suitable:

| Class | Distance range (m) | Suitability score |
|-------|-------------------|------------------|
| 1 | > 500 | 1 |
| 2 | 300–500 | 2 |
| 3 | 100–300 | 3 |
| 4 | 50–100 | 4 |
| 5 | 0–50 | 5 |

```
Processing Toolbox → Whitebox Workflows → Raster Analysis → Reclass
Input: dist_water.tif → Output: dist_water_reclass.tif
```

---

### Step 5 — Weighted Overlay (Raster Calculator)

Combine the three factors using assigned weights (must sum to 1.0).

| Factor | Weight |
|--------|--------|
| Slope suitability | 0.4 |
| TWI suitability | 0.35 |
| Distance suitability | 0.25 |

**QGIS Raster Calculator:**

```
("slope_reclass@1" * 0.4) + ("twi_reclass@1" * 0.35) + ("dist_water_reclass@1" * 0.25)
```

Output: `suitability.tif` (range 1–5, continuous).

---

### Step 6 — Zonal Statistics (Optional)

Summarise suitability scores by catchment polygon.

**Processing Toolbox → QGIS → Vector Analysis →
`Zonal Statistics`** (QGIS native)

| Parameter | Recommended value |
|-----------|------------------|
| Input raster | `suitability.tif` |
| Vector layer | `watersheds.shp` |
| Statistics | Mean, Max, Std Dev |
| Output column prefix | `suit_` |

---

## Python Console Equivalent

```python
import processing

# Step 1: distance from water
processing.run('whitebox_workflows:euclidean_distance', {
    'input': '/data/streams.tif',
    'output': '/data/dist_water.tif',
})

# Step 2–4: reclassify each factor
for src, dst in [
    ('slope', 'slope_reclass'),
    ('twi', 'twi_reclass'),
    ('dist_water', 'dist_water_reclass'),
]:
    processing.run('whitebox_workflows:reclass_from_file', {
        'input': f'/data/{src}.tif',
        'reclass_vals': f'/data/{src}_reclass.txt',
        'output': f'/data/{dst}.tif',
    })

# Step 5: weighted overlay via Raster Calculator
processing.run('qgis:rastercalculator', {
    'EXPRESSION': '("slope_reclass@1" * 0.4) + ("twi_reclass@1" * 0.35) + ("dist_water_reclass@1" * 0.25)',
    'LAYERS': [
        '/data/slope_reclass.tif',
        '/data/twi_reclass.tif',
        '/data/dist_water_reclass.tif',
    ],
    'OUTPUT': '/data/suitability.tif',
})

print("Suitability analysis complete.")
```

---

## Advanced: Focal Statistics

Focal statistics smooth or enhance spatial patterns at a neighbourhood scale.

**Processing Toolbox → Whitebox Workflows → Raster Analysis →
`Mean Filter`**

| Parameter | Recommended value |
|-----------|------------------|
| Input raster | `suitability.tif` |
| Filter size X | `5` (cells) |
| Filter size Y | `5` |
| Output | `suitability_smooth.tif` |

Use `Standard Deviation Filter` to highlight areas of high local variability,
or `Percentile Filter` for rank-based neighbourhood smoothing.

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Raster Calculator outputs all NoData | Rasters have different extents or CRS | Clip/warp all inputs to common grid before combining |
| Reclass produces unexpected values | Range gaps or overlaps in reclass table | Verify that intervals are contiguous with no gap or overlap |
| Zonal statistics returns wrong polygon counts | Raster–vector CRS mismatch | Reproject vector to match raster CRS before running |
| Weighted overlay result > 5 | Weights do not sum to 1.0 | Recalculate weights so they sum to exactly 1.0 |
| Focal filter introduces edge NoData | Kernel extends beyond raster boundary | Pad raster with `Expand Raster` before filtering, or ignore edge cells |

---

## Validation Checklist

- [ ] All input rasters aligned (same CRS, extent, cell size, NoData value).
- [ ] Reclass table covers the full observed value range with no gaps.
- [ ] Weighted overlay weights sum to 1.0.
- [ ] Output suitability range matches expected 1–5 interval.
- [ ] Zonal statistics polygon CRS matches raster CRS.
- [ ] Focal filter kernel size is appropriate for target feature scale.
