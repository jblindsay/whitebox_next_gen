# Spatial Hydrology

Hydrological analysis in WbW-R covers the full DEM-to-drainage workflow: depression removal, flow routing, stream extraction, watershed delineation, and hydrological indices. All heavy computation runs in the Whitebox backend via `wbw_<tool>(...)` wrappers (or `wbw_run_tool(...)` for dynamic tool-id workflows); R handles orchestration, conditional logic, and result inspection.

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
wbw_breach_depressions_least_cost(dem          = dem$file_path(),
  output       = 'dem_breached.tif',
  dist         = 500,        # maximum breach distance (cells)
  max_cost     = -1.0,       # -1 = no cost limit
  min_dist     = TRUE,
  flat_increment = 0.0001,
  fill_deps    = FALSE)
```

### Fill Depressions — Wang & Liu

Use fill after breach, or alone on smooth low-relief landscapes:

```r
wbw_fill_depressions_wang_and_liu(dem       = dem$file_path(),
  output    = 'dem_filled.tif',
  fix_flats = TRUE,
  flat_increment = 0.001,
  max_depth = -1.0)
```

### Burn-in Stream Network

If you have a mapped stream network, burn it into the DEM before filling to improve drainage coherence:

```r
streams <- wbw_read_vector('streams.shp')
wbw_burn_streams_at_roads(dem     = dem$file_path(),
  streams = streams$file_path(),
  roads   = 'roads.shp',
  output  = 'dem_burn.tif',
  width   = 6.0)
```

---

## Flow Routing

### D8 Flow Pointer and Accumulation

```r
wbw_d8_pointer(dem    = 'dem_breached.tif',
  output = 'd8_ptr.tif',
  esri_pntr = FALSE)

wbw_d8_flow_accum(i      = 'd8_ptr.tif',
  output = 'd8_acc.tif',
  out_type = 'cells',
  log    = FALSE,
  clip   = FALSE,
  pntr   = TRUE)
```

### D-Infinity Routing

Distributes flow across two cells proportionally:

```r
wbw_d_inf_pointer(dem    = 'dem_breached.tif',
  output = 'dinf_ptr.tif')

wbw_d_inf_flow_accum(dem      = 'dem_breached.tif',
  output   = 'dinf_sca.tif',
  out_type = 'sca',
  log      = FALSE)
```

### FD8 Multi-Flow

```r
wbw_fd8_flow_accum(dem      = 'dem_breached.tif',
  output   = 'fd8_acc.tif',
  out_type = 'cells',
  exponent = 1.1,
  threshold = 0.0,
  log      = FALSE,
  clip     = FALSE)
```

---

## Stream Extraction

```r
wbw_extract_streams(flow_accum = 'd8_acc.tif',
  output     = 'streams.tif',
  threshold  = 5000,
  zero_background = TRUE)
```

### Extract Valley Bottom by Region Growing

```r
wbw_extract_valley_bottoms(dem     = 'dem_breached.tif',
  output  = 'valley_bottoms.tif',
  threshold = 0.5,
  line_thin = TRUE)
```

---

## Watershed Delineation

### Snap Pour Points

```r
outlets <- wbw_read_vector('gauges.shp')
wbw_snap_pour_points(pour_pts   = outlets$file_path(),
  flow_accum = 'd8_acc.tif',
  output     = 'gauges_snapped.shp',
  snap_dist  = 200.0)
```

### Single and Multi-Watershed Delineation

```r
wbw_watershed(d8_pntr    = 'd8_ptr.tif',
  pour_pts   = 'gauges_snapped.shp',
  output     = 'watersheds.tif',
  esri_pntr  = FALSE)
```

### Unnest Basins

```r
wbw_unnest_basins(d8_pntr   = 'd8_ptr.tif',
  pour_pts  = 'gauges_snapped.shp',
  output    = 'unnested_basins.tif',
  esri_pntr = FALSE)
```

---

## Flow Path Analysis

```r
# Downslope flowpath length
wbw_downslope_flowpath_length(d8_pntr  = 'd8_ptr.tif',
  output   = 'ds_flowpath_len.tif',
  watersheds = '',
  weights  = '',
  esri_pntr = FALSE)

# Distance to stream outlet
wbw_distance_to_outlet(d8_pntr = 'd8_ptr.tif',
  streams = 'streams.tif',
  output  = 'dist_to_outlet.tif',
  esri_pntr = FALSE,
  zero_background = TRUE)

# HAND — Height Above Nearest Drainage
wbw_elevation_above_stream(dem     = 'dem_breached.tif',
  streams = 'streams.tif',
  output  = 'hand.tif')
```

---

## Hydrological Indices

### Topographic Wetness Index

```r
wbw_wetness_index(sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'twi.tif')
```

### Sediment Transport Index

```r
wbw_sediment_transport_index(sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'sti.tif',
  sca_exponent  = 0.4,
  slope_exponent = 1.3)
```

### Stream Power Index

```r
wbw_stream_power_index(sca    = 'dinf_sca.tif',
  slope  = 'slope.tif',
  output = 'spi.tif',
  exponent = 1.0)
```

---

## Isobasins — Equal-Area Basin Partitioning

```r
wbw_isobasins(dem          = 'dem_breached.tif',
  output       = 'isobasins.tif',
  size         = 5000,
  connections  = TRUE,
  csv_file     = 'isobasin_connectivity.csv')
```

---

## Stochastic Depression Analysis

```r
wbw_stochastic_depression_analysis(dem         = dem$file_path(),
  output      = 'prob_flooded.tif',
  rmse        = 0.18,
  range       = 20.0,
  iterations  = 1000)
```

---

## Complete Hydrological Analysis Workflow

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/hydro_workflow')

dem <- wbw_read_raster('dem.tif')

# 1. Breach depressions
wbw_breach_depressions_least_cost(dem = dem$file_path(), output = 'dem_b.tif', dist = 500,
  flat_increment = 0.0001, fill_deps = FALSE)

# 2. D8 routing
wbw_d8_pointer(dem = 'dem_b.tif', output = 'd8_ptr.tif')
wbw_d8_flow_accum(i = 'd8_ptr.tif', output = 'd8_acc.tif',
  out_type = 'cells', pntr = TRUE)

# 3. DInf SCA for TWI
wbw_d_inf_flow_accum(dem = 'dem_b.tif', output = 'sca.tif',
  out_type = 'sca')

# 4. Slope for indices
wbw_slope(dem = 'dem_b.tif', output = 'slope.tif',
  units = 'degrees')

# 5. Streams
wbw_extract_streams(flow_accum = 'd8_acc.tif',
  output = 'streams.tif', threshold = 3000, zero_background = TRUE)

# 6. Watershed
outlets <- wbw_read_vector('gauges.shp')
wbw_snap_pour_points(pour_pts = outlets$file_path(),
  flow_accum = 'd8_acc.tif', output = 'gauges_snap.shp', snap_dist = 100.0)
wbw_watershed(d8_pntr = 'd8_ptr.tif',
  pour_pts = 'gauges_snap.shp', output = 'watersheds.tif')

# 7. Indices
wbw_wetness_index(sca = 'sca.tif', slope = 'slope.tif',
  output = 'twi.tif')
wbw_elevation_above_stream(dem = 'dem_b.tif',
  streams = 'streams.tif', output = 'hand.tif')

cat('Hydrological analysis complete.\n')
```
