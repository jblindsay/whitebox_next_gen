# Terrain Analysis and Geomorphometry

Digital terrain analysis in WbW-R encompasses the full range of operations from surface derivatives (slope, aspect, curvature) through multi-scale geomorphometric indices and geomorphological classification. All computation is performed by the Whitebox backend; this chapter shows how to wire those tools into reproducible R workflows.

---

## Core Concepts

Terrain analysis relies on these fundamental concepts:

- **Slope**: The gradient of the land surface, typically expressed in degrees (0–90) or percent. High values indicate steep terrain; low values indicate flat terrain. Critical for erosion, landslide, and hydrology models.
- **Aspect**: Compass direction a slope faces (0–360°, measured clockwise from north). Flat cells are typically -1 or nodata. Controls solar radiation, drainage direction, and habitat quality.
- **Curvature**: Rate of change of slope (usually planform and profile separately). Positive profile curvature indicates acceleration zones (ridges); negative indicates deceleration zones (valleys).
- **Flow direction**: The steepest downslope direction from each cell. Forms the basis for flow accumulation, drainage network delineation, and watershed boundaries.
- **Flow accumulation**: Number of upslope cells draining to each cell, proportional to contributing area. High values indicate stream channels and valley bottoms.
- **Topographic Wetness Index (TWI)**: Ln(upslope area / tan(slope)), predicts persistent moisture. High TWI indicates probable saturated zones; used in soil moisture and flood risk mapping.
- **Landform classification**: Categorizing terrain into types (e.g. summits, ridges, valleys, footslopes) via multivariate analysis. Provides interpretable terrain structure without tuning thresholds.
- **Multiscale analysis**: Deriving terrain metrics at multiple scales. Single-scale derivatives can miss important structure; multiscale approaches reveal process-relevant scales.
- **Viewshed**: Set of cells visible from a vantage point. Used in landscape perception, military analysis, and wind farm siting.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()

# Set working directory for relative file paths
setwd('/data/terrain')
```

---

## Reading a DEM

```r
dem <- wbw_read_raster('dem.tif')
meta <- dem$metadata()
cat('Rows:', meta$rows, '  Cols:', meta$columns, '\n')
cat('Cell size:', meta$resolution_x, 'm\n')
cat('NoData:', meta$nodata, '\n')
```

---

## Surface Derivatives

### Slope

```r
# Slope in degrees
slope_deg <- wbw_slope(dem   = dem$file_path(),
  output = 'slope_deg.tif',
  units  = 'degrees',
  z_factor = 1.0)

slope <- wbw_read_raster('slope_deg.tif')
```

### Aspect

```r
wbw_aspect(dem    = dem$file_path(),
  output = 'aspect.tif',
  zero_aspect = FALSE)
```

### Hillshade

```r
wbw_hillshade(dem     = dem$file_path(),
  output  = 'hillshade.tif',
  azimuth = 315.0,
  altitude = 30.0)
```

### Profile Curvature, Plan Curvature, Tangential Curvature

```r
wbw_profile_curvature(dem = dem$file_path(), output = 'profc.tif')
wbw_plan_curvature(dem = dem$file_path(), output = 'planc.tif')
wbw_tangential_curvature(dem = dem$file_path(), output = 'tangc.tif')
```

### Mean Curvature and Other Modes

```r
wbw_mean_curvature(dem = dem$file_path(), output = 'meanc.tif')
wbw_minimal_curvature(dem = dem$file_path(), output = 'minc.tif')
wbw_maximal_curvature(dem = dem$file_path(), output = 'maxc.tif')
wbw_gaussian_curvature(dem = dem$file_path(), output = 'gaussc.tif')
```

---

## Topographic Wetness and Flow Accumulation

```r
wbw_wetness_index(sca    = 'sca.tif',
  slope  = 'slope_deg.tif',
  output = 'twi.tif')
```

Computing the specific contributing area (SCA) first:

```r
wbw_d_inf_flow_accum(dem     = dem$file_path(),
  output  = 'sca.tif',
  out_type = 'sca',
  threshold = 0.0,
  log      = FALSE,
  clip     = FALSE)
```

---

## Relief and Position Indices

### Relative Topographic Position

```r
wbw_relative_topographic_position(dem    = dem$file_path(),
  output = 'rtp.tif',
  filterx = 101,
  filtery = 101)
```

### TPI, Deviation from Mean, Scale Standardised Elevation

```r
wbw_topographic_position_index(dem    = dem$file_path(),
  output = 'tpi.tif',
  minrad = 1.0,
  maxrad = 25.0,
  steps  = 10,
  num_sig_digits = 3)

wbw_deviation_from_mean_elevation(dem    = dem$file_path(),
  output = 'dev_mean.tif',
  filterx = 11,
  filtery = 11)

wbw_elev_above_pit(dem    = dem$file_path(),
  output = 'elev_above_pit.tif')
```

---

## Multi-Scale Roughness and Complexity

```r
wbw_multiscale_roughness(dem    = dem$file_path(),
  out_mag  = 'ms_rough_mag.tif',
  out_scale = 'ms_rough_scale.tif',
  min_scale = 1,
  max_scale = 100,
  step = 1)

wbw_vector_ruggedness_measure(dem    = dem$file_path(),
  output = 'vrm.tif',
  filterx = 11,
  filtery = 11)

wbw_ruggedness_index(dem    = dem$file_path(),
  output = 'tri.tif')
```

---

## Terrain Smoothing

DEM-derived curvature, landform classes, and terrain-position indices can be
overly sensitive to short-range roughness. Whitebox Next Gen now includes a
multiscale feature-preserving smoother that is better aligned with modern
terrain-analysis workflows than relying only on the older single-scale tool.

```r
wbw_feature_preserving_smoothing_multiscale(input = dem$file_path(),
  output = 'dem_smooth_multiscale.tif',
  smoothing_amount = 0.65,
  edge_preservation = 0.80,
  scale_levels = 4,
  fidelity = 0.45,
  z_factor = 1.0)

dem_smooth <- wbw_read_raster('dem_smooth_multiscale.tif')
```

The key idea is coarse-to-fine smoothing: larger-scale terrain form is
stabilized first, then finer detail is reintroduced while preserving major
breaks-in-slope. This is especially useful before curvature, geomorphon, and
terrain-position workflows.

---

## Geomorphons — Landform Classification

```r
wbw_geomorphons(dem              = dem$file_path(),
  output           = 'geomorphons.tif',
  search           = 50,
  threshold        = 1.0,
  fdist            = 0,
  skip             = 0,
  forms            = TRUE,
  residuals        = FALSE)
```

Geomorphon class codes: 1=flat, 2=peak, 3=ridge, 4=shoulder, 5=spur, 6=slope, 7=hollow, 8=footslope, 9=valley, 10=pit.

---

## Multi-Scale Topographic Position

```r
wbw_multiscale_topographic_position_image(local  = 'tpi_local.tif',
  meso   = 'tpi_meso.tif',
  broad  = 'tpi_broad.tif',
  output = 'mstpi_rgb.tif',
  hillshade = 'hillshade.tif')
```

---

## Solar and Horizon Analysis

```r
wbw_time_in_daylight(dem        = dem$file_path(),
  output     = 'daylight_hrs.tif',
  lat        = 43.5,
  long       = -80.5,
  az_fraction = 10.0,
  max_dist   = 100.0,
  utc_offset  = '-5:00',
  start_day   = 172,
  end_day     = 172,
  start_time  = '06:00:00',
  end_time    = '20:00:00')
```

---

## WbW-Pro Spotlight: Terrain Constraint and Conflict Analysis

- **Problem:** Screen terrain constraints early for siting and corridor
  decisions.
- **Tool:** `terrain_constraint_and_conflict_analysis`
- **Typical inputs:** DEM, optional wetness, optional flood-risk surface,
  optional land-cover penalty, slope threshold.
- **Typical outputs:** Terrain-conflict score raster, conflict classes, and
  summary outputs.

```r
result <- s$terrain_constraint_and_conflict_analysis(
  dem               = 'dem.tif',
  wetness           = 'wetness_index_norm.tif',
  flood_risk        = 'flood_risk_norm.tif',
  landcover_penalty = 'landcover_penalty_norm.tif',
  slope_limit_deg   = 15.0,
  output_prefix     = 'terrain_conflict_corridor_a'
)

print(result)
```

> **Note:** This workflow requires a session initialized with a valid Pro
> licence.

### Pro Sweep Diagnostics for Siting Workflows

For scenario testing in Pro siting workflows, `wind_turbine_siting` and
`solar_site_suitability_analysis` accept a `sweep_spec` list and emit
additional sweep outputs:

- `run_matrix_summary` (CSV)
- `sensitivity_report` (JSON)
- `sensitivity_report_html` (HTML)
- `stability_map` (GeoTIFF; `3=high`, `2=medium`, `1=low`)

The sensitivity JSON includes a normalized span and stability classifier:

- `metrics.primary_metric`
- `metrics.primary_relative_span`
- `metrics.stability_class` (`high`, `medium`, `low`)

```r
s <- wbw_session()

sweep_spec <- list(
  schema_version = "1.0.0",
  sweep_mode = "grid",
  parameters = list(
    list(name = "candidate_threshold", values = list(0.65, 0.70, 0.75))
  )
)

result <- s$wind_turbine_siting(
  dem = "dem.tif",
  settlements = "settlements.gpkg",
  sweep_spec = sweep_spec,
  output_prefix = "wind_sweep"
)
```

---

## Complete Terrain Analysis Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/terrain_workflow')

dem <- wbw_read_raster('dem.tif')

# Smooth the DEM before derivative-heavy work.
wbw_feature_preserving_smoothing_multiscale(input = dem$file_path(),
  output = 'dem_smooth_multiscale.tif',
  smoothing_amount = 0.65,
  edge_preservation = 0.80,
  scale_levels = 4,
  fidelity = 0.45)

dem_smooth <- wbw_read_raster('dem_smooth_multiscale.tif')

# Derivatives
for (tool in c('slope', 'aspect')) {
  wbw_run_tool(tool, args = list(dem = dem_smooth$file_path(),
    output = paste0(tool, '.tif')), session = s)
}

wbw_profile_curvature(dem = dem_smooth$file_path(), output = 'profc.tif')
wbw_plan_curvature(dem = dem_smooth$file_path(), output = 'planc.tif')

# Hillshade for visualisation
wbw_hillshade(dem = dem_smooth$file_path(), output = 'hillshade.tif',
  azimuth = 315.0, altitude = 30.0)

# TWI
wbw_d_inf_flow_accum(dem = dem_smooth$file_path(), output = 'sca.tif',
  out_type = 'sca')
wbw_wetness_index(sca = 'sca.tif', slope = 'slope.tif',
  output = 'twi.tif')

# Roughness and classification
wbw_vector_ruggedness_measure(dem = dem_smooth$file_path(), output = 'vrm.tif', filterx = 11, filtery = 11)
wbw_geomorphons(dem = dem_smooth$file_path(), output = 'geomorphons.tif', search = 50, threshold = 1.0,
  fdist = 0, skip = 0, forms = TRUE, residuals = FALSE)

cat('Terrain analysis complete.\n')
```

---

## Tips

- **Pre-process your DEM**: Remove spikes, pits, and fill sinks before computing derivatives. Use `breach_depressions_least_cost()` (preserves real depressions) or `fill_depressions()` (infills) depending on your application.
- **Resolution matters**: Coarse DEMs (e.g. 30 m) are smoothed and may miss local process features. Fine DEMs (e.g. 1 m LiDAR) can be noisy. Choose resolution to match your process scale.
- **Flow direction algorithms**: D8 (8-directional) is faster but can cause artificial flow alignments. D-infinity and Dinf-Rho distribute flow more naturally and are preferred for continuous analyses.
- **Curvature is scale-dependent**: Always compute curvature at the scale matching your DEM resolution. A 10 m window on a 1 m DEM can overinterpret noise; a 1 m window on a 30 m DEM misses important structure.
- **Multiscale position classification**: Run classification at 3–5 scales (e.g. local, neighbourhood, regional) and examine layer coherence. Inconsistent multi-scale patterns suggest model overfitting.
- **Viewshed validation**: Viewshed results are sensitive to DEM quality and observer height assumptions. Always validate against ground observation or high-res ortho imagery.
- **Hydrological thresholds are empirical**: Contributing area thresholds for stream initiation vary by geology and climate (typically 0.5–5 km²). Calibrate against observed stream networks.
- **Openness and exposure**: Sky-view factor (SVF) and terrain openness support better hillshading and visibility assessment than raw slope or aspect. Use for visual interpretation and photogrammetry.
```
