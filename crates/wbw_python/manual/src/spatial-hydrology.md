# Spatial Hydrology

Spatial hydrology covers the analysis of water movement and accumulation across
the land surface using DEMs. It is one of Whitebox's deepest specializations
and spans DEM conditioning, flow routing, watershed delineation, stream network
extraction, and advanced connectivity and storage modelling.

This chapter progresses from fundamental concepts through a full watershed
analysis pipeline, including advanced topics such as probabilistic depression
analysis and hydrologic connectivity.

---

## Core Concepts

### The Hydrologic DEM Problem

Most raw DEMs contain surface depressions — cells or groups of cells enclosed
by higher neighbours — that are real topographic basins, artefacts of
acquisition noise, or pits introduced by interpolation. Standard single-flow-
direction routing (D8) cannot route water out of these depressions. Before
flow routing, the DEM must be **conditioned** to remove spurious depressions
while preserving genuine ones (such as lakes and wetlands).

The two principal conditioning strategies are:

**Filling**: raises cells within each depression up to the spill elevation.
This guarantees flat, drained surfaces but can significantly alter elevations
and produce large flat areas that require secondary gradient enforcement.

**Breaching**: cuts a narrow channel of minimum cost through the barrier
surrounding each depression, routing water to the nearest outside outlet.
This preserves more of the original elevation field and is usually preferred
when breachable barriers exist (roads, levees, embankments).

In practice, a hybrid approach — breach where possible, fill remaining
depressions — is often optimal.

### Flow Direction Algorithms

After DEM conditioning, a flow direction raster encodes which direction water
flows from each cell. Whitebox supports multiple algorithms, each with
different properties:

| Algorithm | Flow type | Best use |
|-----------|-----------|----------|
| D8 | Single (deterministic) | Channel delineation, simple watersheds |
| Rho8 | Single (stochastic) | Reduces D8 directional bias |
| D-infinity (DInf) | Multiple (proportional split) | Hillslope flux, dispersive flow |
| FD8 | Multiple (proportional) | Shallow overland flow modelling |
| MD-infinity | Multiple | Subsurface/soil moisture modelling |
| Quinn et al. (FD8 variant) | Multiple | Wetness index, slope-area workflows |
| Minimal Dispersion Flow Algorithm | Adaptive | Balance between D8 and DInf |

---

## DEM Conditioning

### Breach Depressions

`breach_depressions_least_cost` is the recommended first-pass conditioning
tool. It minimises the total vertical cost of breaching while constraining
each breach path to a maximum length and depth.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem = wbe.read_raster('dem_raw.tif')

dem_breached = wbe.breach_depressions_least_cost(
    dem,
    dist=50,           # maximum breach channel length in cells
    max_cost=None,     # no cost ceiling (breach wherever needed)
    min_dist=True,     # prefer shorter breach paths
    flat_increment=None,
    fill_deps=True     # fill any remaining unbreachable depressions
)
wbe.write_raster(dem_breached, 'dem_conditioned.tif')
```

If your study area contains roads or embankments that act as real barriers to
flow, consider mapping them as embankments first and then burning them into
the DEM:

```python
dem_burned = wbe.topological_breach_burn(dem, streams='streams_mapped.shp')
dem_conditioned = wbe.breach_depressions_least_cost(dem_burned, dist=50, fill_deps=True)
```

### Fill Depressions

When you need guaranteed flat-free surfaces or are processing coarse-resolution
DEMs where breaching introduces artefacts, full filling is appropriate:

```python
dem_filled = wbe.fill_depressions(dem, flat_increment=0.001)
wbe.write_raster(dem_filled, 'dem_filled.tif')
```

The `flat_increment` adds a tiny gradient across flat areas to enable
downstream flow routing.

### Single-Cell Pit Removal

For DEMs that are mostly clean but contain isolated single-cell pits (from
radiometric noise in LiDAR interpolation), a lightweight pre-pass avoids
unnecessary full conditioning:

```python
dem_pitless = wbe.fill_pits(dem)
dem_conditioned = wbe.breach_depressions_least_cost(dem_pitless, dist=50, fill_deps=True)
```

---

## Flow Direction

### D8 Flow Pointer

D8 assigns each cell a direction code to its steepest neighbour. The result
is an integer pointer raster used by many downstream tools.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem_conditioned.tif')

d8_pntr = wbe.d8_pointer(dem)
wbe.write_raster(d8_pntr, 'd8_pointer.tif')
```

### D-Infinity Flow Pointer

DInf partitions flow between two downslope neighbours according to the
slope angle, producing a continuous flow direction that reduces the
directional bias of D8:

```python
dinf_pntr = wbe.dinf_pointer(dem)
wbe.write_raster(dinf_pntr, 'dinf_pointer.tif')
```

---

## Flow Accumulation

Flow accumulation counts (or accumulates weighted values of) the upstream
contributing area draining to each cell. It is the primary tool for
identifying stream channels, delineating watersheds, and computing
topographic wetness indices.

### Basic D8 Flow Accumulation

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem_conditioned.tif')

d8_pntr = wbe.d8_pointer(dem)

# Cell-count accumulation
flow_accum = wbe.d8_flow_accum(d8_pntr, out_type='cells')
wbe.write_raster(flow_accum, 'flow_accum_d8.tif')

# Log-transform for visualisation (high dynamic range)
import math
flow_accum_log = flow_accum.log2()   # WbW raster operator
wbe.write_raster(flow_accum_log, 'flow_accum_d8_log.tif')
```

### Specific Contributing Area (D-Infinity)

Specific contributing area normalises by cell width and is used in wetness
index and erosion models:

```python
dinf_pntr = wbe.dinf_pointer(dem)
sca = wbe.dinf_flow_accum(dinf_pntr, input_is_pointer=True, out_type='sca')
wbe.write_raster(sca, 'specific_contributing_area.tif')
```

### Topographic Wetness Index (TWI)

TWI = ln(SCA / tan(slope)) is a widely used proxy for soil moisture and
saturated area:

```python
import whitebox_workflows as wbw
import math

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem_conditioned.tif')

dinf_pntr = wbe.dinf_pointer(dem)
sca       = wbe.dinf_flow_accum(dinf_pntr, out_type='sca')
slope_rad = wbe.slope(dem, units='radians')

# Avoid log(0) by clamping minimum SCA
sca_clamped = sca.max(0.001)
slope_clamped = slope_rad.max(0.001)

twi = (sca_clamped / slope_clamped.tan()).log()
wbe.write_raster(twi, 'twi.tif')
```

---

## Stream Network Extraction

Stream channels appear in the flow accumulation raster as cells with very
large contributing areas. Thresholding the accumulation surface reveals the
channel network.

### Simple Threshold Extraction

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

d8_pntr   = wbe.read_raster('d8_pointer.tif')
flow_accum = wbe.read_raster('flow_accum_d8.tif')

# Extract stream cells above threshold (contributing area in cells)
streams = wbe.extract_streams(flow_accum, threshold=1000.0)
wbe.write_raster(streams, 'streams_raster.tif')

# Convert to vector lines
stream_vec = wbe.raster_streams_to_vector(streams, d8_pntr)
wbe.write_vector(stream_vec, 'streams.gpkg')
```

### Stream Link Tools

Stream links divide the network into segments between junctions, enabling
per-segment statistics and Strahler order computation:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
streams   = wbe.read_raster('streams_raster.tif')
d8_pntr   = wbe.read_raster('d8_pointer.tif')

# Assign Strahler order to each channel cell
strahler = wbe.strahler_stream_order(d8_pntr, streams)
wbe.write_raster(strahler, 'strahler_order.tif')

# Hack stream order (less sensitive to stubs)
hack = wbe.hack_stream_order(d8_pntr, streams)
wbe.write_raster(hack, 'hack_order.tif')

# Horton's drainage composition ratios
horton_ratios = wbe.horton_ratios(d8_pntr, streams)
print(horton_ratios)   # returns bifurcation ratio, length ratio, area ratio
```

### Shreve and Topological Orders

For large network analysis requiring consistent junction-based numbering:

```python
shreve = wbe.shreve_stream_magnitude(d8_pntr, streams)
wbe.write_raster(shreve, 'shreve_magnitude.tif')

topo_order = wbe.topological_stream_order(d8_pntr, streams)
wbe.write_raster(topo_order, 'topological_order.tif')
```

---

## Watershed Delineation

### Single-Outlet Watershed

Delineate the complete contributing area upstream of a single outlet point:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

d8_pntr   = wbe.read_raster('d8_pointer.tif')
outlet_pts = wbe.read_vector('outlet.shp')

watershed = wbe.watershed(d8_pntr, outlet_pts)
wbe.write_raster(watershed, 'watershed.tif')
```

### Multiple Outlet Points / Nested Basins

`unnest_basins` handles overlapping contributing areas when working with
multiple outlets in a nested configuration:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
d8_pntr   = wbe.read_raster('d8_pointer.tif')
outlets   = wbe.read_vector('outlets_multiple.shp')

# Each outlet gets a unique basin ID; nested basins properly accounted for
nested = wbe.unnest_basins(d8_pntr, outlets)
wbe.write_raster(nested, 'nested_watersheds.tif')
```

### Subbasin Delineation

Delineate subbasins for all channel junctions simultaneously:

```python
subbasins = wbe.subbasins(d8_pntr, streams)
wbe.write_raster(subbasins, 'subbasins.tif')
```

---

## Terrain Hydrologic Indices

### Elevation Above Stream

Measures the vertical distance of each cell above the nearest channel, useful
for floodplain mapping and valley-bottom delineation:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

dem     = wbe.read_raster('dem_conditioned.tif')
streams = wbe.read_raster('streams_raster.tif')
d8_pntr = wbe.read_raster('d8_pointer.tif')

height_above_stream = wbe.elevation_above_stream(dem, streams)
wbe.write_raster(height_above_stream, 'height_above_stream.tif')
```

Cells with values below 1.0 or 2.0 metres are candidate floodplain cells
under typical storm recurrence intervals.

### Downslope Distance to Stream

How far (path-following the flow network) is each cell from the nearest channel?

```python
dist_to_stream = wbe.downslope_distance_to_stream(
    d8_pntr, streams, dist_type='path'
)
wbe.write_raster(dist_to_stream, 'distance_to_stream.tif')
```

---

## Advanced: Stochastic Depression Analysis

Real DEMs have vertical uncertainty. A depression that appears in one DEM
may not appear once that uncertainty is accounted for. `stochastic_depression_
analysis` runs a Monte Carlo simulation across an ensemble of stochastically
perturbed DEMs to estimate the probability that each cell is part of a real
depression rather than a noise artefact.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem = wbe.read_raster('dem.tif')

# rmse: vertical uncertainty of the DEM (e.g. 0.15 m for good LiDAR)
# range: spatial autocorrelation range of the error field in map units
# iterations: number of Monte Carlo realisations
depression_prob = wbe.stochastic_depression_analysis(
    dem,
    rmse=0.15,
    range=5.0,
    iterations=100
)
wbe.write_raster(depression_prob, 'depression_probability.tif')
```

Cells with probability > 0.5 are more likely to be real basins than artefacts.
This output can drive a probability-weighted conditioned DEM or inform
uncertainty quantification in flood inundation modelling.

---

## Advanced: Hydrologic Connectivity

`hydrologic_connectivity` computes the probability (or frequency) that each
cell contributes runoff to the watershed outlet, accounting for temporary
storage and variable source areas:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem     = wbe.read_raster('dem_conditioned.tif')
streams = wbe.read_raster('streams_raster.tif')

connectivity = wbe.hydrologic_connectivity(dem, streams)
wbe.write_raster(connectivity, 'hydrologic_connectivity.tif')
```

Highly connected areas (near streams, with steep contributing slopes) respond
quickly to rainfall; low-connectivity areas (flats, depressions, wetlands)
buffer the hydrological response.

---

## Advanced: Impoundment Analysis

Impoundment modelling simulates the water surface extent and volume that would
result from a dam or weir at a specified location on a stream:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
dem     = wbe.read_raster('dem_conditioned.tif')
streams = wbe.read_raster('streams_raster.tif')

# impoundment height above stream bed in metres
impoundment_area = wbe.impoundment_size_index(
    dem, streams, damlength=1000.0
)
wbe.write_raster(impoundment_area, 'impoundment_size_index.tif')
```

---

## Full Watershed Analysis Script

This end-to-end script conditions a DEM, routes flow, extracts a stream
network, delineates a watershed, and produces a suite of hydrologic
derivatives in a single automated workflow.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.verbose = True

# --- Inputs ---
dem_path    = 'dem_raw.tif'
outlet_path = 'outlet_point.shp'
streams_threshold = 2000   # cells (adjust to DEM resolution)

# --- 1. Read DEM ---
dem = wbe.read_raster(dem_path)

# --- 2. Fill missing data ---
dem = wbe.fill_missing_data(dem, filter_size=11)

# --- 3. Smooth ---
dem = wbe.feature_preserving_smoothing(dem, filter_size=11, num_iter=2)
wbe.write_raster(dem, 'dem_smooth.tif')

# --- 4. Condition DEM ---
dem_cond = wbe.breach_depressions_least_cost(dem, dist=50, fill_deps=True)
wbe.write_raster(dem_cond, 'dem_conditioned.tif')

# --- 5. Flow direction ---
d8_pntr  = wbe.d8_pointer(dem_cond)
dinf_pntr = wbe.dinf_pointer(dem_cond)
wbe.write_raster(d8_pntr, 'd8_pointer.tif')

# --- 6. Flow accumulation ---
flow_accum = wbe.d8_flow_accum(d8_pntr, out_type='cells')
wbe.write_raster(flow_accum, 'flow_accum.tif')

# --- 7. Streams ---
streams = wbe.extract_streams(flow_accum, threshold=streams_threshold)
wbe.write_raster(streams, 'streams.tif')
stream_vec = wbe.raster_streams_to_vector(streams, d8_pntr)
wbe.write_vector(stream_vec, 'streams.gpkg')

# --- 8. Stream order ---
strahler = wbe.strahler_stream_order(d8_pntr, streams)
wbe.write_raster(strahler, 'strahler_order.tif')

# --- 9. Watershed ---
outlet_pts = wbe.read_vector(outlet_path)
watershed  = wbe.watershed(d8_pntr, outlet_pts)
wbe.write_raster(watershed, 'watershed.tif')

# --- 10. TWI ---
sca         = wbe.dinf_flow_accum(dinf_pntr, out_type='sca')
slope_rad   = wbe.slope(dem_cond, units='radians')
sca_c       = sca.max(0.001)
slope_c     = slope_rad.max(0.001)
twi         = (sca_c / slope_c.tan()).log()
wbe.write_raster(twi, 'twi.tif')

# --- 11. Height above stream ---
height_above = wbe.elevation_above_stream(dem_cond, streams)
wbe.write_raster(height_above, 'height_above_stream.tif')

print("Watershed analysis pipeline complete.")
```

---

## Summary

Hydrological analysis in WbW-Py follows a clear preparation → routing →
network → delineation progression:

1. **Condition** the DEM (breach + fill) to ensure topologically correct flow.
2. **Route flow** using the algorithm appropriate to your application
   (D8 for channels, DInf for hillslope flux).
3. **Accumulate** flow and extract the stream network by thresholding.
4. **Delineate** watersheds and subbasins from outlets or channel junctions.
5. **Compute** terrain hydrologic indices (TWI, height above stream, connectivity).
6. **Advance** to probabilistic or connectivity analysis for uncertainty
   quantification and dynamic source area modelling.
