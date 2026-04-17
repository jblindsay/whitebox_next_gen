# Terrain Analysis and Geomorphometry

Terrain analysis — or geomorphometry — is the quantitative characterization of
land-surface form from digital elevation models (DEMs). It is one of the
original strengths of the Whitebox platform and covers a broad spectrum:
from simple first-order derivatives like slope and aspect, through
classification of terrain form, to advanced multiscale and visibility analyses.

This chapter moves from foundational concepts through intermediate and advanced
workflows, with example scripts throughout.

---

## What is a DEM?

A digital elevation model (DEM) is a raster where each cell stores the
elevation of the land surface at that location. DEMs underpin nearly every
terrain analysis: slope gradients, drainage patterns, landform classification,
solar radiation modelling, and viewshed analysis all begin with a DEM.

Common DEM sources include:
- LiDAR-derived bare-earth surfaces (highest resolution and accuracy)
- Photogrammetric DEMs from drone or aerial surveys
- SRTM (global 30 m coverage)
- ASTER GDEM and Copernicus DEM (global 30 m or 90 m)

DEM quality — vertical accuracy, spatial resolution, systematic artefacts from
interpolation or acquisition — directly limits the quality of derivative
products, so always inspect your DEM before computing derivatives.

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

## First-Order Terrain Derivatives

First-order derivatives measure the rate of change of elevation over space.

### Slope

Slope is the magnitude of the first derivative of elevation: the maximum rate
of elevation change per unit horizontal distance. It is typically expressed in
degrees (0–90) or as a percentage (rise over run × 100).

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem = wbe.read_raster('dem.tif')

# Degrees is the default units
slope_deg = wbe.slope(dem, units='degrees')
wbe.write_raster(slope_deg, 'slope_degrees.tif')

# Percentage slope useful for agricultural and road applications
slope_pct = wbe.slope(dem, units='percent')
wbe.write_raster(slope_pct, 'slope_percent.tif')
```

High values indicate steep terrain; low values indicate flat terrain. Slope
is widely used in landslide hazard mapping, erosion modelling, habitat
suitability analysis, and infrastructure routing.

### Aspect

Aspect is the compass direction that a slope faces, measured clockwise from
north (0°–360°). Flat cells conventionally receive a value of -1 or nodata.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem = wbe.read_raster('dem.tif')
aspect = wbe.aspect(dem)
wbe.write_raster(aspect, 'aspect.tif')
```

Aspect controls solar insolation, snow persistence, soil moisture, and
vegetation structure. South-facing slopes in the northern hemisphere receive
more direct solar radiation and tend to be warmer and drier than north-facing
slopes.

### Hillshade

Hillshading simulates how the terrain would appear under a directional light
source and is primarily a visualization aid rather than an analytical
derivative.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem = wbe.read_raster('dem.tif')

# Single directional hillshade — azimuth and altitude in degrees
hs = wbe.hillshade(dem, azimuth=315.0, altitude=45.0)
wbe.write_raster(hs, 'hillshade.tif')

# Multidirectional hillshade (more even illumination, no shadow artefacts)
mdhs = wbe.multidirectional_hillshade(dem)
wbe.write_raster(mdhs, 'hillshade_multidirectional.tif')

# Hypsometrically tinted hillshade blends elevation colour and shading
hyp = wbe.hypsometrically_tinted_hillshade(dem)
wbe.write_raster(hyp, 'hillshade_hypsometric.tif')
```

---

## Curvature

Curvature characterizes how the terrain surface bends in space. While slope
measures the inclination of a surface, curvature measures how that inclination
is changing — the "shape" of the surface rather than its steepness alone.

Whitebox provides a range of curvature types, each capturing a different
geometric property.

### Profile Curvature

Profile curvature is the curvature in the direction of steepest descent.
Positive values correspond to convex terrain (accelerating flow); negative
values to concave terrain (decelerating flow, area of convergence).

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

profile_curv = wbe.profile_curvature(dem)
wbe.write_raster(profile_curv, 'profile_curvature.tif')
```

### Plan Curvature

Plan curvature is measured in the horizontal plane, perpendicular to the
direction of slope. It indicates flow convergence (negative values) or flow
divergence (positive values) and is a useful predictor of soil moisture, run-on
areas, and erosion.

```python
plan_curv = wbe.plan_curvature(dem)
wbe.write_raster(plan_curv, 'plan_curvature.tif')
```

### Full Curvature Suite

For detailed geomorphometric characterisation compute the full family. Each
captures a slightly different geometric facet of the surface:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

curv_types = {
    'mean_curvature':       wbe.mean_curvature(dem),
    'gaussian_curvature':   wbe.gaussian_curvature(dem),
    'minimal_curvature':    wbe.minimal_curvature(dem),
    'maximal_curvature':    wbe.maximal_curvature(dem),
    'casorati_curvature':   wbe.casorati_curvature(dem),
    'accumulation_curvature': wbe.accumulation_curvature(dem),
    'difference_curvature': wbe.difference_curvature(dem),
    'generating_function':  wbe.generating_function(dem),
    'curvedness':           wbe.curvedness(dem),
}

for name, raster in curv_types.items():
    wbe.write_raster(raster, f'{name}.tif')
```

**When to use each:**
- *Mean curvature*: general shape descriptor; used in hydrological and
  geomorphological studies.
- *Gaussian curvature*: distinguishes synclinal, anteclinal, and saddle-point
  forms; positive on entirely convex surfaces, negative on saddle shapes.
- *Minimal/maximal curvature*: principal curvatures; describe the extremes of
  bending in orthogonal directions.
- *Casorati curvature*: root-mean-square of principal curvatures; a rotationally
  invariant roughness descriptor.
- *Accumulation curvature*: amplifies high-curvature terrain features; useful
  for ridge–valley extraction.

---

## Terrain Position and Landform Classification

Terrain position describes where a location sits relative to its surrounding
landscape — ridge crest, upper slope, flat, valley floor, and so on. It forms
the backbone of many landform classification workflows.

### Topographic Position Index (TPI)

TPI compares the elevation of a cell to the mean elevation of its neighbourhood.
Positive values indicate elevated positions (ridges, hilltops); negative values
indicate depressed positions (valleys, channels); values near zero indicate
planar slopes or mid-slope positions.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

# Inner and outer radii in cells
tpi = wbe.relative_topographic_position(dem, filterx=11, filtery=11)
wbe.write_raster(tpi, 'tpi.tif')
```

### Deviation from Mean Elevation

Similar to TPI but expressed as a Z-score: how many standard deviations the
local elevation is from the neighbourhood mean.

```python
dev = wbe.deviation_from_mean_elevation(dem, filterx=11, filtery=11)
wbe.write_raster(dev, 'deviation_from_mean_elev.tif')
```

### Geomorphons

Geomorphons classify the local terrain into ten fundamental landform elements
by analysing the directional horizon profile in eight compass directions:
flat, peak, ridge, shoulder, spur, slope, hollow, footslope, valley, and pit.
The approach is descriptive and does not require thresholds for individual
derivatives.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

geons = wbe.geomorphons(dem, search=50, threshold=1.0, flat=1.0, forms=True)
wbe.write_raster(geons, 'geomorphons.tif')
```

The `search` parameter sets the look-ahead distance in cells. Increasing it
produces a more generalised classification that emphasises regional landform
context; smaller values capture finer-scale topographic features.

---

## Multiscale Terrain Analysis

Many geomorphic properties are scale-dependent: a feature that appears as a
ridge at one scale is part of a larger plateau at another. Multiscale analysis
explores how terrain derivatives vary with the scale (neighbourhood size) of
computation.

### Multiscale Roughness

Roughness quantifies the local complexity of the terrain surface. The
multiscale variant computes roughness at a range of neighbourhood sizes and
returns both the maximum roughness and the scale at which it occurs.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

roughness, roughness_scale = wbe.multiscale_roughness(
    dem,
    min_scale=1,
    max_scale=100,
    step=1
)

wbe.write_raster(roughness, 'multiscale_roughness.tif')
wbe.write_raster(roughness_scale, 'roughness_scale_of_max.tif')
```

The scale-of-maximum raster describes the spatial grain of the terrain texture:
alluvial fans and smooth glacial valleys show large dominant scales; deeply
dissected badlands and karst terrain show small scales.

### Multiscale Elevation Percentile

Measures how often a cell's elevation is higher than nearby cells across a
range of scales, identifying persistent topographic prominence regardless of
local noise.

```python
mep = wbe.multiscale_elevation_percentile(
    dem,
    min_scale=1,
    max_scale=100,
    step=2,
    sig_digits=3
)
wbe.write_raster(mep, 'multiscale_elevation_percentile.tif')
```

### Multiscale Curvatures

Computes a suite of curvature metrics at multiple scales and returns the value
at the scale of maximum local curvature for each cell.

```python
multiscale_mean_curv = wbe.multiscale_curvatures(
    dem,
    min_scale=1,
    max_scale=30,
    step=1
)
wbe.write_raster(multiscale_mean_curv, 'multiscale_curvatures.tif')
```

### Multiscale Topographic Position Image

Produces an RGB composite where colour encodes topographic position at three
nested scales, providing an intuitive visual summary of multi-level terrain
structure.

```python
mtp = wbe.multiscale_topographic_position_image(
    dem,
    local=1,
    meso=11,
    broad=101
)
wbe.write_raster(mtp, 'multiscale_topo_position.tif')
```

---

## Visibility Analysis

Visibility analysis determines which parts of the landscape can be seen from a
given viewpoint or set of viewpoints.

### Viewshed

A viewshed identifies the set of cells visible from an observer location.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem = wbe.read_raster('dem.tif')
observer_points = wbe.read_vector('observer_locations.shp')

viewshed = wbe.viewshed(dem, observer_points, height=1.8)
wbe.write_raster(viewshed, 'viewshed.tif')
```

### Horizon Angle and Openness

Horizon angle measures the elevation angle to the skyline in a given direction,
useful for solar modelling and local climate studies. Terrain openness (positive
and negative) quantifies how open or enclosed a location is relative to
its surroundings.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

# Positive openness: how "exposed" each cell is
openness_pos = wbe.openness(dem, dist=50, pos_openness=True)
wbe.write_raster(openness_pos, 'openness_positive.tif')

# Negative openness: how "enclosed" each cell is
openness_neg = wbe.openness(dem, dist=50, pos_openness=False)
wbe.write_raster(openness_neg, 'openness_negative.tif')
```

### Sky View Factor

Sky view factor measures the fraction of the sky hemisphere that is visible
from a point on the surface, accounting for topographic obstruction. Values
range from 0 (fully enclosed depression) to 1 (completely open flat surface).
It is used in urban heat island modelling, long-wave radiation estimation, and
snowmelt modelling.

```python
svf = wbe.sky_view_factor(dem, num_directions=16, max_dist=200.0)
wbe.write_raster(svf, 'sky_view_factor.tif')
```

---

## Terrain Smoothing

Raw DEMs often contain artefacts from acquisition and interpolation:
striping, pitting, noisy micro-relief. Smoothing prior to derivative
computation reduces these effects while ideally preserving genuine terrain
features.

### Feature-Preserving Smoothing (Multiscale)

Whitebox Next Gen now includes a newer multiscale feature-preserving smoother
that is better suited to terrain-analysis workflows than the earlier
single-scale smoother when you need to suppress short-wavelength DEM noise
without flattening broader terrain form. The multiscale method works through a
coarse-to-fine pyramid, smoothing at larger scales first and then refining the
surface back toward the source DEM with explicit fidelity and edge-preservation
controls.

At the moment this tool is most directly accessed through the generic
tool-execution surface in WbW-Py.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('raw_dem.tif')

# Coarse-to-fine multiscale smoothing with explicit controls.
dem_smooth = wbe.run_tool(
  'feature_preserving_smoothing_multiscale',
  {
    'input': dem,
    'smoothing_amount': 0.65,
    'edge_preservation': 0.80,
    'scale_levels': 4,
    'fidelity': 0.45,
    'z_factor': 1.0,
  }
)
wbe.write_raster(dem_smooth, 'dem_smooth_multiscale.tif')
```

Use this before curvature, terrain-position, and landform-classification
workflows, especially where acquisition artefacts or interpolation roughness
would otherwise dominate second-derivative products.

---

## WbW-Pro Spotlight: Terrain Constraint and Conflict Analysis

- **Problem:** Screen terrain constraints early for siting and corridor
  decisions.
- **Tool:** `terrain_constraint_and_conflict_analysis`
- **Typical inputs:** DEM, optional wetness, optional flood-risk surface,
  optional land-cover penalty, slope threshold.
- **Typical outputs:** Terrain-conflict score raster, conflict classes, and
  summary outputs.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

result = wbe.run_tool(
  'terrain_constraint_and_conflict_analysis',
  {
    'dem': 'dem.tif',
    'wetness': 'wetness_index_norm.tif',
    'flood_risk': 'flood_risk_norm.tif',
    'landcover_penalty': 'landcover_penalty_norm.tif',
    'slope_limit_deg': 15.0,
    'output_prefix': 'terrain_conflict_corridor_a'
  }
)
print(result)
```

> **Note:** This workflow requires a `WbEnvironment` initialized with a valid
> Pro licence.

---

## Complete Terrain Analysis Pipeline

The following script assembles a typical geomorphometric analysis workflow
moving from raw DEM conditioning through a suite of first- and second-order
derivatives and terrain classification outputs.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.verbose = True

# --- 1. Read and inspect input ---
dem = wbe.read_raster('dem_raw.tif')
print(dem.metadata())

# --- 2. Fill missing data (voids from nodata gaps) ---
dem_filled = wbe.fill_missing_data(dem, filter_size=11)

# --- 3. Multiscale feature-preserving smoothing ---
dem_smooth = wbe.run_tool(
  'feature_preserving_smoothing_multiscale',
  {
    'input': dem_filled,
    'smoothing_amount': 0.65,
    'edge_preservation': 0.80,
    'scale_levels': 4,
    'fidelity': 0.45,
  }
)
wbe.write_raster(dem_smooth, 'dem_smooth_multiscale.tif')

# --- 4. First-order derivatives ---
slope   = wbe.slope(dem_smooth, units='degrees')
aspect  = wbe.aspect(dem_smooth)
hillshade = wbe.multidirectional_hillshade(dem_smooth)

wbe.write_raster(slope,     'slope.tif')
wbe.write_raster(aspect,    'aspect.tif')
wbe.write_raster(hillshade, 'hillshade.tif')

# --- 5. Curvatures ---
prof_curv = wbe.profile_curvature(dem_smooth)
plan_curv = wbe.plan_curvature(dem_smooth)
mean_curv = wbe.mean_curvature(dem_smooth)

wbe.write_raster(prof_curv, 'curvature_profile.tif')
wbe.write_raster(plan_curv, 'curvature_plan.tif')
wbe.write_raster(mean_curv, 'curvature_mean.tif')

# --- 6. Terrain position ---
dev = wbe.deviation_from_mean_elevation(dem_smooth, filterx=11, filtery=11)
wbe.write_raster(dev, 'deviation_from_mean_elev.tif')

# --- 7. Landform classification ---
geomorphons = wbe.geomorphons(dem_smooth, search=50, threshold=1.0, flat=1.0)
wbe.write_raster(geomorphons, 'geomorphons.tif')

# --- 8. Multiscale roughness ---
roughness, roughness_scale = wbe.multiscale_roughness(
    dem_smooth, min_scale=1, max_scale=50, step=1
)
wbe.write_raster(roughness, 'multiscale_roughness.tif')

# --- 9. Visibility ---
svf = wbe.sky_view_factor(dem_smooth, num_directions=16, max_dist=200.0)
wbe.write_raster(svf, 'sky_view_factor.tif')

print("Terrain analysis complete.")
```

---

## Ridges and Valleys

Topographic ridges and channels are fundamental landform elements. Whitebox
provides tools to extract them directly as vector features.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem_smooth.tif')

# Extract ridges
ridges = wbe.find_ridges(dem, line_thin=True)
wbe.write_raster(ridges, 'ridges.tif')

# Extract ridge and valley lines as vectors
ridge_lines = wbe.ridge_and_valley_vectors(dem)
wbe.write_vector(ridge_lines, 'ridge_valley_lines.gpkg')
```

For hydrographic applications, channel extraction is usually better served
through the hydrology toolset (flow accumulation-based extraction), but
ridge-line extraction complements watershed delineation by explicitly mapping
the divide network.

---

## Embankment and Road Mapping

In agricultural, infrastructure, and floodplain contexts, anthropogenic
embankments (dykes, levees, road fills) appear as linear elevated features
on DEMs. Whitebox provides dedicated tools for their detection:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem_lidar.tif')

# Map elevated linear features such as road embankments and dykes
embankments = wbe.embankment_mapping(
    dem,
    road_vec='roads.shp',
    search_dist=2.5,
    min_road_width=6.0,
    typical_width=30.0,
    max_height=2.5,
    spillout_slope=4.0,
    max_remove_fs_slope=0.1,
    min_bench_width=6.0,
    clean_up=True,
    output_type='filtered DEM'
)
wbe.write_raster(embankments, 'dem_embankments_filtered.tif')
```

---

## Summary

Terrain analysis in WbW-Py follows a layered depth model:
- **First-order work**: slope, aspect, hillshade are fast and interpretable.
- **Curvature analysis**: adds information about surface bending and is
  critical for hydrological modelling, mass movement susceptibility, and
  landform process inference.
- **Classification**: geomorphons and multiscale position methods provide
  stable, interpretable landform maps without manually tuned thresholds.
- **Multiscale methods**: reveal scale-dependent structure that single-scale
  derivatives miss.
- **Visibility and openness**: support solar, ecological, and landscape
  planning applications.

For most projects the right progression is: smooth → first-order derivatives →
curvatures → classification → application-specific outputs.

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
