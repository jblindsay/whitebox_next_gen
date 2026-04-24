# Spatial Hydrology

Spatial hydrology workflows extract hydrographic structure — flow directions,
stream networks, watersheds, and hydrologic indices — from a terrain model.
All hydrologic processing begins with a hydrologically conditioned DEM.
This chapter demonstrates a complete watershed delineation workflow from raw
DEM to labelled catchments.

---

## Key Concepts

- **Hydrologic conditioning**: Removing or breaching depressions so that flow
  can route from every cell to a basin outlet without interruption.
- **Flow direction**: Per-cell pointer indicating which of the eight cardinal
  and diagonal neighbours receives runoff (D8 model) or a fractional
  multi-direction model (D-infinity, MD8).
- **Flow accumulation**: Upslope contributing area (in cells or area units).
  High values mark channels; low values mark ridges.
- **Stream extraction threshold**: Minimum contributing area that defines a
  first-order channel. Smaller thresholds produce denser networks.
- **Watershed / catchment**: All cells draining to a common outlet.
  Delineated by tracing flow direction upstream from outlet points.
- **Strahler order**: Hierarchical stream ordering from headwaters (order 1)
  to main channel (highest order). Used to characterise drainage network
  complexity.

---

## End-to-End Workflow: Watershed Delineation

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `dem.tif` | GeoTIFF raster | Projected CRS, metres |
| `outlets.shp` | Point vector | One or more pour points |

---

### Step 1 — Breach Depressions (Preferred Conditioning Method)

Breaching cuts a narrow channel through depression rims rather than filling
them, preserving more of the original topography.

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Breach Depressions (Least Cost)`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem.tif` |
| Maximum search distance (cells) | `10` |
| Maximum breach depth | `2.0` (metres) |
| Flat increment | `0.001` |
| Fill remaining depressions | ✓ enabled |
| Output | `dem_conditioned.tif` |

> If the DEM has large lake or wetland depressions that should not be breached,
> use **`Fill Depressions`** instead.

---

### Step 2 — D8 Flow Direction

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`D8 Pointer`**

| Parameter | Recommended value |
|-----------|------------------|
| Input DEM | `dem_conditioned.tif` |
| Output | `d8_pointer.tif` |

The output is an integer raster (powers of 2: 1, 2, 4, 8, 16, 32, 64, 128)
encoding the direction to the steepest downslope neighbour.

---

### Step 3 — D8 Flow Accumulation

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`D8 Flow Accumulation`**

| Parameter | Recommended value |
|-----------|------------------|
| Input D8 pointer | `d8_pointer.tif` |
| Output type | `Cells` |
| Log-transform output | ☐ (disable for threshold-based channel extraction) |
| Output | `d8_accum.tif` |

Visualise with a logarithmic stretch. The highest values form the main channel
network.

---

### Step 4 — Extract Stream Network

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Extract Streams`**

| Parameter | Recommended value |
|-----------|------------------|
| Flow accumulation raster | `d8_accum.tif` |
| Threshold | `500` (cells — adjust for DEM resolution and drainage density) |
| Zero background | ✓ enabled |
| Output | `streams.tif` |

> Rule of thumb: for a 10 m DEM, a threshold of 500 cells ≈ 0.05 km²
> contributing area, producing a moderately dense first-order network.
> Halve or double the threshold to adjust density.

Convert to vector for display: **Processing → `Raster Streams to Vector`**
→ `streams.shp`.

---

### Step 5 — Snap Pour Points

Pour points must sit on the channel raster. Snap them to the nearest
high-accumulation cell to avoid off-channel watershed boundaries.

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Snap Pour Points`**

| Parameter | Recommended value |
|-----------|------------------|
| Pour points | `outlets.shp` |
| Flow accumulation | `d8_accum.tif` |
| Snap distance (map units) | `200` (metres — adjust to point accuracy) |
| Output | `outlets_snapped.shp` |

---

### Step 6 — Watershed Delineation

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Watershed`**

| Parameter | Recommended value |
|-----------|------------------|
| D8 pointer | `d8_pointer.tif` |
| Pour points | `outlets_snapped.shp` |
| Output | `watersheds.tif` |

Each outlet receives a unique integer ID; cells are assigned that ID. Use
**`Raster to Vector (Polygons)`** in QGIS to produce watershed boundary
polygons, then dissolve by ID if multiple raster cells share an outlet.

---

### Step 7 — Strahler Stream Order (Optional)

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Strahler Stream Order`**

| Parameter | Recommended value |
|-----------|------------------|
| D8 pointer | `d8_pointer.tif` |
| Streams raster | `streams.tif` |
| Output | `strahler.tif` |

---

## Python Console Equivalent

```python
import processing

dem = '/data/dem.tif'
outlets = '/data/outlets.shp'

# Step 1: condition DEM
processing.run('whitebox_workflows:breach_depressions_least_cost', {
    'dem': dem,
    'max_dist': 10,
    'max_depth': 2.0,
    'flat_increment': 0.001,
    'fill': True,
    'output': '/data/dem_conditioned.tif',
})

# Step 2: flow direction
processing.run('whitebox_workflows:d8_pointer', {
    'dem': '/data/dem_conditioned.tif',
    'output': '/data/d8_pointer.tif',
})

# Step 3: flow accumulation
processing.run('whitebox_workflows:d8_flow_accumulation', {
    'input': '/data/d8_pointer.tif',
    'output_type': 'Cells',
    'log': False,
    'output': '/data/d8_accum.tif',
})

# Step 4: extract streams
processing.run('whitebox_workflows:extract_streams', {
    'flow_accum': '/data/d8_accum.tif',
    'threshold': 500.0,
    'zero_background': True,
    'output': '/data/streams.tif',
})

# Step 5: snap pour points
processing.run('whitebox_workflows:snap_pour_points', {
    'pour_pts': outlets,
    'flow_accum': '/data/d8_accum.tif',
    'snap_dist': 200.0,
    'output': '/data/outlets_snapped.shp',
})

# Step 6: watershed
processing.run('whitebox_workflows:watershed', {
    'd8_pntr': '/data/d8_pointer.tif',
    'pour_pts': '/data/outlets_snapped.shp',
    'output': '/data/watersheds.tif',
})

print("Watershed delineation complete.")
```

---

## Advanced: Topographic Wetness Index

TWI requires the specific catchment area (flow accumulation in area units per
unit contour width) rather than the raw cell count.

```python
# SCA-based flow accumulation
processing.run('whitebox_workflows:d8_flow_accumulation', {
    'input': '/data/d8_pointer.tif',
    'output_type': 'Specific Contributing Area',
    'log': False,
    'output': '/data/sca.tif',
})

# Slope in radians (required for TWI)
processing.run('whitebox_workflows:slope', {
    'dem': '/data/dem_conditioned.tif',
    'units': 'Radians',
    'output': '/data/slope_rad.tif',
})

# TWI
processing.run('whitebox_workflows:wetness_index', {
    'sca': '/data/sca.tif',
    'slope': '/data/slope_rad.tif',
    'output': '/data/twi.tif',
})
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Watershed does not extend to expected ridgeline | Pour point not on channel raster | Run Snap Pour Points before Watershed |
| Parallel flow stripes in accumulation raster | Flat areas in conditioned DEM | Enable fix-flats during conditioning |
| Stream network is too sparse / too dense | Threshold too high / too low | Halve or double threshold and re-inspect |
| Watershed covers entire DEM | Pour point is at or near the DEM outlet cell | Check that outlet coordinates fall inside the DEM extent |
| TWI has very high values in flat areas | Slope is near-zero, causing division by tan(0) | Mask flat areas or apply a minimum slope floor (e.g. 0.001 rad) |

---

## Validation Checklist

- [ ] Conditioned DEM has no isolated flat areas (check flow direction raster for NoData).
- [ ] Flow accumulation values increase monotonically toward basin outlet.
- [ ] Extracted channels follow expected valley geometry in the DEM.
- [ ] Snapped pour points lie on the highest-accumulation cells within snap distance.
- [ ] Watershed boundary is a closed polygon that contains the pour point.
- [ ] Strahler orders are consistent with tributary junctions.
