# Terrain Analysis and Geomorphometry

Terrain analysis — or geomorphometry — is the quantitative characterisation of
land-surface form from digital elevation models (DEMs). It is one of the
original strengths of the Whitebox platform and covers first-order derivatives
(slope, aspect), curvature families, terrain position, roughness, multiscale
analysis, and visibility.

This chapter walks through a complete primary-derivative workflow in the QGIS
Processing Toolbox, followed by a Python console version for batch scripting.

---

## Key Concepts

- **DEM**: Raster where each cell stores surface elevation. All terrain
  derivatives begin here. Common sources: LiDAR bare-earth, drone
  photogrammetry, SRTM, Copernicus DEM.
- **Slope**: Maximum rate of elevation change per unit distance (degrees or
  percent). Core input for erosion, landslide, and routing models.
- **Aspect**: Compass direction a slope faces (0–360°, clockwise from north).
  Flat cells are assigned –1. Controls solar insolation and moisture.
- **Curvature**: Rate of change of slope. Profile curvature describes flow
  acceleration/deceleration; plan curvature describes flow convergence/divergence.
- **TPI / geomorphons**: Terrain position indices and landform classification
  assign cells to ridge, slope, valley, etc., without manual thresholds.
- **TWI**: Topographic Wetness Index — ln(upslope area / tan(slope)) —
  predicts persistent soil moisture and runoff zones.

---

## End-to-End Workflow: Primary Terrain Derivatives

This workflow takes a raw DEM through sink filling, then derives the most
commonly used terrain surfaces.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `dem.tif` | GeoTIFF raster | Projected CRS (e.g. UTM) strongly recommended |

---

### Step 1 — Fill Depressions

Sinks (isolated low cells) in a DEM cause flow-routing artifacts in all
downstream terrain derivatives. Fill them first.

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Fill Depressions`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem.tif` |
| Fix flats | ✓ enabled |
| Flat increment | `0.001` (one thousandth of the DEM z unit) |
| Output | `dem_filled.tif` |

> **Why fix flats?** Perfectly flat areas produce ambiguous flow directions.
> Adding a tiny gradient across flats ensures a routable surface.

---

### Step 2 — Slope

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Slope`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_filled.tif` |
| Output units | `Degrees` |
| Z conversion factor | `1.0` (set to `0.3048` if DEM z is in feet but CRS is metres) |
| Output | `slope.tif` |

Expected output range: 0° (flat) to ~85° (near-vertical cliff). Values above
70° often indicate interpolation artefacts — inspect those cells.

---

### Step 3 — Aspect

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Aspect`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_filled.tif` |
| Output | `aspect.tif` |

Flat cells receive –1. Apply a pseudocolor ramp (circular HSV) to aspect for
intuitive visualisation of slope direction.

---

### Step 4 — Hillshade

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Hillshade`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_filled.tif` |
| Azimuth (°) | `315` (NW sun — standard cartographic convention) |
| Altitude (°) | `45` |
| Z factor | `1.0` |
| Output | `hillshade.tif` |

Set the hillshade layer to **Multiply** blend mode in QGIS and overlay it on a
coloured DEM for a publication-quality relief map.

---

### Step 5 — Profile and Plan Curvature

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Profile Curvature`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_filled.tif` |
| Output | `profile_curv.tif` |

Repeat for **`Plan Curvature`** → `plan_curv.tif`.

Style both outputs with a diverging colour ramp centred on 0. Negative profile
curvature (blue) marks deceleration zones (valley bottoms); positive (red)
marks acceleration zones (ridge crests). Plan curvature negatives mark
convergent hollows; positives mark divergent noses.

---

### Step 6 — Topographic Wetness Index

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Wetness Index`**

| Parameter | Recommended value |
|-----------|------------------|
| Slope raster | `slope.tif` |
| Specific contributing area raster | *(run `D8 Flow Accumulation` first — see Spatial Hydrology chapter)* |
| Output | `twi.tif` |

High TWI values (> 8–10) indicate persistent moisture zones. Use as a
predictor variable in soil, flood, and habitat models.

---

## Python Console Equivalent

Paste the following into the QGIS Python Console
(**Plugins → Python Console**) or save as a Processing script to batch
multiple DEMs.

```python
import processing

dem = '/data/dem.tif'

# Step 1: fill depressions
processing.run('whitebox_workflows:fill_depressions', {
    'dem': dem,
    'fix_flats': True,
    'flat_increment': 0.001,
    'output': '/data/dem_filled.tif',
})

# Step 2: slope
processing.run('whitebox_workflows:slope', {
    'dem': '/data/dem_filled.tif',
    'units': 'Degrees',
    'zfactor': 1.0,
    'output': '/data/slope.tif',
})

# Step 3: aspect
processing.run('whitebox_workflows:aspect', {
    'dem': '/data/dem_filled.tif',
    'output': '/data/aspect.tif',
})

# Step 4: hillshade
processing.run('whitebox_workflows:hillshade', {
    'dem': '/data/dem_filled.tif',
    'azimuth': 315.0,
    'altitude': 45.0,
    'zfactor': 1.0,
    'output': '/data/hillshade.tif',
})

# Step 5: curvature
processing.run('whitebox_workflows:profile_curvature', {
    'dem': '/data/dem_filled.tif',
    'output': '/data/profile_curv.tif',
})
processing.run('whitebox_workflows:plan_curvature', {
    'dem': '/data/dem_filled.tif',
    'output': '/data/plan_curv.tif',
})

print("Terrain derivatives complete.")
```

---

## Advanced: Geomorphons Landform Classification

Geomorphons classify each cell into one of ten landform elements (peak, ridge,
shoulder, spur, slope, hollow, footslope, valley, pit, flat) by analysing
horizon profiles in eight compass directions.

**Processing Toolbox → Whitebox Workflows → Terrain Analysis →
`Geomorphons`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_filled.tif` |
| Search distance (cells) | `50` (adjust to DEM resolution and landscape scale) |
| Skip radius (cells) | `0` |
| Flatness threshold (°) | `1.0` |
| Output | `geomorphons.tif` |

The output is a categorical raster (1–10). Apply a predefined categorical
colour map — the Geomorphons palette is available in many QGIS style
repositories.

```python
processing.run('whitebox_workflows:geomorphons', {
    'dem': '/data/dem_filled.tif',
    'search': 50,
    'skip': 0,
    'threshold': 1.0,
    'output': '/data/geomorphons.tif',
})
```

---

## Pro Siting Sweep Diagnostics

For Pro terrain siting workflows (`wind_turbine_siting` and
`solar_site_suitability_analysis`), providing a sweep specification executes a
multi-run grid and emits extra diagnostics:

- `run_matrix_summary` (CSV)
- `sensitivity_report` (JSON)
- `sensitivity_report_html` (HTML)
- `stability_map` (GeoTIFF; `3=high`, `2=medium`, `1=low`)

Inside `sensitivity_report`, use these fields for quick robustness checks:

- `metrics.primary_metric`
- `metrics.primary_relative_span`
- `metrics.stability_class` (`high`, `medium`, `low`)

These outputs are intended for scenario comparison and shortlist stability
review before field validation.

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Slope values are unrealistically high (> 85°) | DEM has interpolation artefacts or NoData spikes | Run `Remove Off-terrain Objects` or inspect raw DEM |
| Flat areas produce zero slope everywhere | Depressions not filled before slope derivation | Run `Fill Depressions` first |
| Aspect shows –1 across large areas | Large flat regions in DEM | Expected for flat input; check DEM resolution |
| Curvature is noisy on fine-resolution DEMs | Sensor noise dominates at small spatial scales | Apply `Gaussian Filter` (σ ≈ 1–2 cells) before curvature |
| Units mismatch — Z factor warning | Horizontal CRS in metres but DEM z in feet | Set Z conversion factor to `0.3048` |

---

## Validation Checklist

- [ ] DEM uses a projected CRS (not geographic degrees).
- [ ] No unexpected flat artefacts introduced by depression filling.
- [ ] Slope range plausible for local relief (inspect histogram).
- [ ] Aspect –1 cells are spatially limited to genuine flats.
- [ ] Curvature raster has a near-symmetric distribution centred on 0.
- [ ] Hillshade visually matches known ridgelines and valley geometry.
