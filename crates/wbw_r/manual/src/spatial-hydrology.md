# Spatial Hydrology

Hydrological analysis in WbW-R covers the full DEM-to-drainage workflow: depression removal, flow routing, stream extraction, watershed delineation, and hydrological indices. All heavy computation runs in the Whitebox backend via `wbw_run_tool()`; R handles orchestration, conditional logic, and result inspection.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/hydrology')

dem <- wbw_read_raster('dem.tif')
```

---

## Depression Removal

Raw DEMs often contain spurious pits that block modelled drainage. Choose the method appropriate to your landscape and accuracy requirements.

### Breach Depressions — Least-Cost

The preferred pre-treatment for most applications. Carves the minimum-width, minimum-depth channel through each depression barrier:

```r
wbw_run_tool('breach_depressions_least_cost', args = list(
  dem          = dem$file_path(),
  output       = 'dem_breached.tif',
  dist         = 500,        # maximum breach distance (cells)
  max_cost     = -1.0,       # -1 = no cost limit
  min_dist     = TRUE,
  flat_increment = 0.0001,
  fill_deps    = FALSE
), session = s)
```

### Fill Depressions — Wang & Liu

Use fill after breach, or alone on smooth low-relief landscapes:

```r
wbw_run_tool('fill_depressions_wang_and_liu', args = list(
  dem       = dem$file_path(),
  output    = 'dem_filled.tif',
  fix_flats = TRUE,
  flat_increment = 0.001,
  max_depth = -1.0
), session = s)
```

### Burn-in Stream Network

If you have a mapped stream network, burn it into the DEM before filling to improve drainage coherence:

```r
streams <- wbw_read_vector('streams.shp')
wbw_run_tool('burn_streams_at_roads', args = list(
  dem     = dem$file_path(),
  streams = streams$file_path(),
  roads   = 'roads.shp',
  output  = 'dem_burn.tif',
  width   = 6.0
), session = s)
```

---

## Flow Routing

### D8 Flow Pointer and Accumulation

```r
wbw_run_tool('d8_pointer', args = list(
  dem    = 'dem_breached.tif',
  output = 'd8_ptr.tif',
  esri_pntr = FALSE
), session = s)

wbw_run_tool('d8_flow_accum', args = list(
  i      = 'd8_ptr.tif',
  output = 'd8_acc.tif',
  out_type = 'cells',
  log    = FALSE,
  clip   = FALSE,
  pntr   = TRUE
), session = s)
```

### D-Infinity Routing

Distributes flow across two cells proportionally:

```r
wbw_run_tool('d_inf_pointer', args = list(
  dem    = 'dem_breached.tif',
  output = 'dinf_ptr.tif'
), session = s)

wbw_run_tool('d_inf_flow_accum', args = list(
  dem      = 'dem_breached.tif',
  output   = 'dinf_sca.tif',
  out_type = 'sca',
  log      = FALSE
), session = s)
```

### FD8 Multi-Flow

```r
wbw_run_tool('fd8_flow_accum', args = list(
  dem      = 'dem_breached.tif',
  output   = 'fd8_acc.tif',
  out_type = 'cells',
  exponent = 1.1,
  threshold = 0.0,
  log      = FALSE,
  clip     = FALSE
), session = s)
```

---

## Stream Extraction

```r
wbw_run_tool('extract_streams', args = list(
  flow_accum = 'd8_acc.tif',
  output     = 'streams.tif',
  threshold  = 5000,
  zero_background = TRUE
), session = s)
```

### Extract Valley Bottom by Region Growing

```r
wbw_run_tool('extract_valley_bottoms', args = list(
  dem     = 'dem_breached.tif',
  output  = 'valley_bottoms.tif',
  threshold = 0.5,
  line_thin = TRUE
), session = s)
```

---

## Watershed Delineation

### Snap Pour Points

```r
outlets <- wbw_read_vector('gauges.shp')
wbw_run_tool('snap_pour_points', args = list(
  pour_pts   = outlets$file_path(),
  flow_accum = 'd8_acc.tif',
  output     = 'gauges_snapped.shp',
  snap_dist  = 200.0
), session = s)
```

### Single and Multi-Watershed Delineation

```r
wbw_run_tool('watershed', args = list(
  d8_pntr    = 'd8_ptr.tif',
  pour_pts   = 'gauges_snapped.shp',
  output     = 'watersheds.tif',
  esri_pntr  = FALSE
), session = s)
```

### Unnest Basins

```r
wbw_run_tool('unnest_basins', args = list(
  d8_pntr   = 'd8_ptr.tif',
  pour_pts  = 'gauges_snapped.shp',
  output    = 'unnested_basins.tif',
  esri_pntr = FALSE
), session = s)
```

---

## Flow Path Analysis

```r
# Downslope flowpath length
wbw_run_tool('downslope_flowpath_length', args = list(
  d8_pntr  = 'd8_ptr.tif',
  output   = 'ds_flowpath_len.tif',
  watersheds = '',
  weights  = '',
  esri_pntr = FALSE
), session = s)

# Distance to stream outlet
wbw_run_tool('distance_to_outlet', args = list(
  d8_pntr = 'd8_ptr.tif',
  streams = 'streams.tif',
  output  = 'dist_to_outlet.tif',
  esri_pntr = FALSE,
  zero_background = TRUE
), session = s)

# HAND — Height Above Nearest Drainage
wbw_run_tool('elevation_above_stream', args = list(
  dem     = 'dem_breached.tif',
  streams = 'streams.tif',
  output  = 'hand.tif'
), session = s)
```

---

## Hydrological Indices

### Topographic Wetness Index

```r
wbw_run_tool('wetness_index', args = list(
  sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'twi.tif'
), session = s)
```

### Sediment Transport Index

```r
wbw_run_tool('sediment_transport_index', args = list(
  sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'sti.tif',
  sca_exponent  = 0.4,
  slope_exponent = 1.3
), session = s)
```

### Stream Power Index

```r
wbw_run_tool('stream_power_index', args = list(
  sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'spi.tif',
  exponent = 1.0
), session = s)
```

---

## Isobasins — Equal-Area Basin Partitioning

```r
wbw_run_tool('isobasins', args = list(
  dem          = 'dem_breached.tif',
  output       = 'isobasins.tif',
  size         = 5000,
  connections  = TRUE,
  csv_file     = 'isobasin_connectivity.csv'
), session = s)
```

---

## Stochastic Depression Analysis

```r
wbw_run_tool('stochastic_depression_analysis', args = list(
  dem         = dem$file_path(),
  output      = 'prob_flooded.tif',
  rmse        = 0.18,
  range       = 20.0,
  iterations  = 1000
), session = s)
```

---

## Complete Hydrological Analysis Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/hydro_workflow')

dem <- wbw_read_raster('dem.tif')

# 1. Breach depressions
wbw_run_tool('breach_depressions_least_cost', args = list(
  dem = dem$file_path(), output = 'dem_b.tif', dist = 500,
  flat_increment = 0.0001, fill_deps = FALSE), session = s)

# 2. D8 routing
wbw_run_tool('d8_pointer',   args = list(dem = 'dem_b.tif', output = 'd8_ptr.tif'), session = s)
wbw_run_tool('d8_flow_accum', args = list(i = 'd8_ptr.tif', output = 'd8_acc.tif',
  out_type = 'cells', pntr = TRUE), session = s)

# 3. DInf SCA for TWI
wbw_run_tool('d_inf_flow_accum', args = list(dem = 'dem_b.tif', output = 'sca.tif',
  out_type = 'sca'), session = s)

# 4. Slope for indices
wbw_run_tool('slope', args = list(dem = 'dem_b.tif', output = 'slope.tif',
  units = 'degrees'), session = s)

# 5. Streams
wbw_run_tool('extract_streams', args = list(flow_accum = 'd8_acc.tif',
  output = 'streams.tif', threshold = 3000, zero_background = TRUE), session = s)

# 6. Watershed
outlets <- wbw_read_vector('gauges.shp')
wbw_run_tool('snap_pour_points', args = list(pour_pts = outlets$file_path(),
  flow_accum = 'd8_acc.tif', output = 'gauges_snap.shp', snap_dist = 100.0), session = s)
wbw_run_tool('watershed', args = list(d8_pntr = 'd8_ptr.tif',
  pour_pts = 'gauges_snap.shp', output = 'watersheds.tif'), session = s)

# 7. Indices
wbw_run_tool('wetness_index', args = list(sca = 'sca.tif', slope = 'slope.tif',
  output = 'twi.tif'), session = s)
wbw_run_tool('elevation_above_stream', args = list(dem = 'dem_b.tif',
  streams = 'streams.tif', output = 'hand.tif'), session = s)

cat('Hydrological analysis complete.\n')
```
