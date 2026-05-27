# Hydrology Tools

This document covers hydrology tools currently ported into the backend, including DEM depression removal/conditioning and flow-accumulation workflows.

## Hydrology (Depression Removal)

These tools prepare DEMs for downstream flow-routing by removing pits, flats, or larger enclosed depressions. The short version is:

- Prefer `breach_depressions_least_cost` when you want the lowest-impact correction and realistic cuts through barriers such as roads or embankments.
- Use `fill_depressions` when you need a robust full-fill solution and are comfortable modifying all enclosed depressions up to their spill elevations.
- Use `fill_pits` or `breach_single_cell_pits` as lightweight preprocessing for single-cell artifacts, not as a replacement for full depression conditioning.
- Treat the Wang-and-Liu and Planchon-and-Darboux variants mainly as compatibility methods when you need those specific historical formulations.

### Tool Index

- `breach_depressions_least_cost`
- `breach_single_cell_pits`
- `fill_depressions`
- `fill_depressions_planchon_and_darboux`
- `fill_depressions_wang_and_liu`
- `fill_pits`
- `depth_in_sink`
- `sink`

## Hydrology (Flow Accumulation)

### Tool Index

- `d8_pointer`
- `d8_flow_accum`
- `dinf_pointer`
- `dinf_flow_accum`
- `fd8_pointer`
- `fd8_flow_accum`
- `rho8_pointer`
- `rho8_flow_accum`
- `mdinf_flow_accum`
- `qin_flow_accumulation`
- `quinn_flow_accumulation`
- `minimal_dispersion_flow_algorithm`
- `flow_accum_full_workflow`
- `d8_mass_flux`
- `dinf_mass_flux`

## Hydrology (Diagnostics)

### Tool Index

- `find_noflow_cells`
- `num_inflowing_neighbours`
- `find_parallel_flow`
- `edge_contamination`
- `flow_length_diff`
- `downslope_flowpath_length`
- `max_upslope_flowpath_length`
- `average_upslope_flowpath_length`
- `elevation_above_stream`
- `elevation_above_stream_euclidean`
- `downslope_distance_to_stream`
- `average_flowpath_slope`
- `max_upslope_value`
- `longest_flowpath`
- `depth_to_water`
- `fill_burn`
- `burn_streams_at_roads`
- `trace_downslope_flowpaths`
- `flood_order`
- `insert_dams`
- `raise_walls`
- `topological_breach_burn`
- `stochastic_depression_analysis`
- `unnest_basins`
- `upslope_depression_storage`
- `flatten_lakes`
- `hydrologic_connectivity`
- `impoundment_size_index`

## Hydrology (Watersheds and Basins)

### Tool Index

- `basins`
- `watershed_from_raster_pour_points`
- `watershed`
- `jenson_snap_pour_points`
- `snap_pour_points`
- `subbasins`
- `hillslopes`
- `strahler_order_basins`
- `isobasins`

### mdinf_flow_accum

```
mdinf_flow_accum(dem, out_type="sca", exponent=1.1, threshold=None, log=False, clip=False, output_path=None, callback=None)
```

Computes MD-Infinity triangular multiple-flow-direction accumulation from a DEM.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `out_type` | string | no | One of `"cells"`, `"ca"`, `"sca"` (default). |
| `exponent` | float | no | Slope weighting exponent (default `1.1`). |
| `threshold` | float\|None | no | Optional convergence threshold in cells. If provided and exceeded, routing becomes convergent. |
| `log` | bool | no | If true, log-transform output values. |
| `clip` | bool | no | Compatibility flag accepted by the API. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.mdinf_flow_accum(
    dem,
    out_type="value",
    exponent=1.0,
    threshold=1.0,
    output_path="result.tif",
)
```

### qin_flow_accumulation

```
qin_flow_accumulation(dem, out_type="sca", exponent=10.0, max_slope=45.0, threshold=None, log=False, clip=False, output_path=None, callback=None)
```

Computes Qin MFD flow accumulation from a DEM using a gradient-dependent dynamic exponent.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `out_type` | string | no | One of `"cells"`, `"ca"`, `"sca"` (default). |
| `exponent` | float | no | Upper-bound exponent parameter (default `10.0`). |
| `max_slope` | float | no | Upper-bound slope in degrees used by the dynamic exponent function (default `45.0`). |
| `threshold` | float\|None | no | Optional convergence threshold in cells. |
| `log` | bool | no | If true, log-transform output values. |
| `clip` | bool | no | Compatibility flag accepted by the API. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.qin_flow_accumulation(
    dem,
    out_type="value",
    exponent=1.0,
    max_slope=1.0,
    threshold=1.0,
    output_path="result.tif",
)
```

### quinn_flow_accumulation

```
quinn_flow_accumulation(dem, out_type="sca", exponent=1.1, threshold=None, log=False, clip=False, output_path=None, callback=None)
```

Computes Quinn MFD flow accumulation from a DEM using accumulation-dependent convergence.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `out_type` | string | no | One of `"cells"`, `"ca"`, `"sca"` (default). |
| `exponent` | float | no | Exponent parameter (default `1.1`). |
| `threshold` | float\|None | no | Optional convergence threshold in cells. |
| `log` | bool | no | If true, log-transform output values. |
| `clip` | bool | no | Compatibility flag accepted by the API. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.quinn_flow_accumulation(
    dem,
    out_type="value",
    exponent=1.0,
    threshold=1.0,
    output_path="result.tif",
)
```

### minimal_dispersion_flow_algorithm

```
minimal_dispersion_flow_algorithm(raster, out_type="sca", path_corrected_direction_preference=0.0, log_transform=False, clip=False, esri_pntr=False, flow_dir_output_path=None, output_path=None, callback=None)
```

Computes the Minimal Dispersion Flow Algorithm (MDFA) from a DEM and returns both a flow-direction raster and flow-accumulation raster as a tuple.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `raster` | Raster | yes | Input depressionless DEM raster. |
| `out_type` | string | no | One of `"cells"`, `"ca"`, `"sca"` (default). |
| `path_corrected_direction_preference` | float | no | Preference parameter `p` in `[0, 1]`; `1.0` is fully non-dispersive. |
| `log_transform` | bool | no | If true, log-transform accumulation values. |
| `clip` | bool | no | Compatibility flag accepted by the API. |
| `esri_pntr` | bool | no | If true, encode flow-direction output in Esri pointer style. |
| `flow_dir_output_path` | string | no | Optional output path for the flow-direction raster. |
| `output_path` | string | no | Optional output path for the flow-accumulation raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, Raster]` in this order:

- `flow_dir`: `Raster`
- `result`: `Raster`

**WbEnvironment usage**

```python
raster_1, raster_2 = wbe.hydrology.flow_routing.minimal_dispersion_flow_algorithm(
    raster,
    out_type="value",
    path_corrected_direction_preference=1.0,
    flow_dir_output_path="flow_dir.tif",
    output_path="result.tif",
)
```

### flow_accum_full_workflow

```
flow_accum_full_workflow(dem, out_type="sca", log_transform=False, clip=False, esri_pntr=False, breached_dem_output_path=None, flow_dir_output_path=None, output_path=None, callback=None)
```

Runs a full non-divergent flow workflow in one call and returns a tuple containing a depressionless DEM, a D8 pointer raster, and a D8 accumulation raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `out_type` | string | no | One of `"cells"`, `"ca"`, `"sca"` (default). |
| `log_transform` | bool | no | If true, log-transform accumulation values. |
| `clip` | bool | no | If true, clip accumulation display maximum (compatibility behavior). |
| `esri_pntr` | bool | no | If true, encode flow-direction output in Esri pointer style. |
| `breached_dem_output_path` | string | no | Optional output path for the depressionless DEM raster. |
| `flow_dir_output_path` | string | no | Optional output path for the flow-direction raster. |
| `output_path` | string | no | Optional output path for the flow-accumulation raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, Raster, Raster]` in this order:

- `breached_dem`: `Raster`
- `flow_dir`: `Raster`
- `result`: `Raster`

**WbEnvironment usage**

```python
raster_1, raster_2, raster_3 = wbe.hydrology.flow_routing.flow_accum_full_workflow(
    dem,
    out_type="value",
    breached_dem_output_path="breached_dem.tif",
    flow_dir_output_path="flow_dir.tif",
    output_path="result.tif",
)
```

### find_noflow_cells

```
find_noflow_cells(dem, output_path=None, callback=None, interior_only=False)
```

Finds DEM cells that have no lower D8 neighbour. On a fully conditioned DEM this should usually be limited to valid edge-drainage cases; interior hits often indicate remaining pits or flats.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `interior_only` | bool | no | If true, only flags true interior no-flow cells (excluding raster-border and NoData-adjacent outlets). |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.find_noflow_cells(
    dem,
    output_path="result.tif",
)
```

### dinf_mass_flux

```
dinf_mass_flux(dem, loading, efficiency, absorption, output_path=None, callback=None)
```

Routes mass downslope using D-Infinity flow-routing, accumulating `loading` while applying per-cell `efficiency` and `absorption` losses.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `loading` | Raster | yes | Input loading raster. |
| `efficiency` | Raster | yes | Input efficiency raster (`0-1` or percent values). |
| `absorption` | Raster | yes | Input absorption raster in loading units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.dinf_mass_flux(
    dem,
    loading,
    efficiency,
    absorption,
    output_path="result.tif",
)
```

### trace_downslope_flowpaths

```
trace_downslope_flowpaths(seed_points, d8_pntr, esri_pntr=False, zero_background=False, output_path=None, callback=None)
```

Traces downslope D8 flowpaths from seed points to no-flow cells or the raster edge. Output values are visit counts where overlapping traces occur.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `seed_points` | Vector | yes | Input point vector of seed locations. |
| `d8_pntr` | Any | yes | Input D8 pointer raster. |
| `esri_pntr` | bool | no | If true, interpret D8 pointers with ESRI encoding. |
| `zero_background` | bool | no | If true, background is `0`; otherwise background is NoData. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.trace_downslope_flowpaths(
    seed_points,
    d8_pointer,
    output_path="result.tif",
)
```

### flood_order

```
flood_order(dem, output_path=None, callback=None)
```

Computes flood order from a DEM using a priority-flood traversal from edges inward, assigning each valid cell its visitation sequence.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.flood_order(
    dem,
    output_path="result.tif",
)
```

### flatten_lakes

```
flatten_lakes(dem, lakes, output_path=None, callback=None)
```

Flattens lake polygons in a DEM by setting each lake interior to its minimum perimeter elevation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `lakes` | Vector | yes | Input polygon vector of lake features. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.flatten_lakes(
    dem,
    lakes,
    output_path="result.tif",
)
```

### insert_dams

```
insert_dams(dem, dam_points, dam_length, output_path=None, callback=None)
```

Inserts localized dam embankments at specified point locations using profile-based crest selection constrained by maximum dam length.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `dam_points` | Vector | yes | Input point vector of dam locations. |
| `dam_length` | float | yes | Maximum dam length in map units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.insert_dams(
    dem,
    dam_points,
    dam_length=1.0,
    output_path="result.tif",
)
```

### raise_walls

```
raise_walls(dem, walls, breach_lines=None, wall_height=100.0, output_path=None, callback=None)
```

Raises DEM elevations along wall features by a specified height increment, with optional breach lines used to carve openings through raised walls.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `walls` | Vector | yes | Input line or polygon vector defining wall segments. |
| `breach_lines` | Vector | no | Optional vector defining breach locations. |
| `wall_height` | float | no | Elevation increment applied to wall cells. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.raise_walls(
    dem,
    walls,
    breach_lines,
    wall_height=1.0,
    output_path="result.tif",
)
```

### topological_breach_burn

```
topological_breach_burn(streams, dem, snap_distance=0.001, out_streams_path=None, out_dem_path=None, out_dir_path=None, out_fa_path=None, callback=None)
```

Performs topological stream burning using a stream vector and DEM, producing stream raster, burned/conditioned DEM, D8 pointer, and D8 accumulation outputs.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `streams` | Vector | yes | Input stream network vector. |
| `dem` | Raster | yes | Input DEM raster. |
| `snap_distance` | float | no | Optional stream snapping distance used in burn-depth scaling. |
| `out_streams_path` | string | no | Optional output path for rasterized streams. |
| `out_dem_path` | string | no | Optional output path for burned/conditioned DEM. |
| `out_dir_path` | string | no | Optional output path for D8 pointer raster. |
| `out_fa_path` | string | no | Optional output path for flow-accumulation raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, Raster, Raster, Raster]` in this order:

- `raster_1`: `Raster`
- `raster_2`: `Raster`
- `raster_3`: `Raster`
- `raster_4`: `Raster`

**WbEnvironment usage**

```python
raster_1, raster_2, raster_3, raster_4 = wbe.hydrology.depressions_storage.topological_breach_burn(
    dem,
    streams,
    snap_distance=1.0,
    output_streams_path="output_streams.dat",
    output_dem_path="output_dem.dat",
    output_dir_path="output_dir.dat",
    output_flow_accum_path="output_flow_accum.dat",
)
```

### stochastic_depression_analysis

```
stochastic_depression_analysis(dem, rmse, range, iterations=100, output_path=None, callback=None)
```

Estimates depression-membership probability for each DEM cell using Monte Carlo perturbation of elevation error and repeated depression filling.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `rmse` | float | yes | Elevation RMSE used for Gaussian perturbation. |
| `range` | float | yes | Error autocorrelation range in map units. |
| `iterations` | int | no | Number of Monte Carlo iterations. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.stochastic_depression_analysis(
    dem,
    rmse=1.0,
    range=1.0,
    iterations=1,
    output_path="result.tif",
)
```

### unnest_basins

```
unnest_basins(d8_pointer, pour_points, esri_pntr=False, output_path=None, callback=None)
```

Delineates full nested basins for pour points over a D8 pointer raster, producing one raster per nesting level.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Raster | yes | Input D8 pointer raster. |
| `pour_points` | Vector | yes | Input point vector of outlets/pour points. |
| `esri_pntr` | bool | no | If true, interpret pointer values with ESRI encoding. |
| `output_path` | string | no | Optional base output path used to write numbered outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `list[Raster]`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.unnest_basins(
    d8_pointer,
    pour_points,
    output_path="result.tif",
)
```

### upslope_depression_storage

```
upslope_depression_storage(dem, output_path=None, callback=None)
```

Estimates average upslope depression-storage depth by conditioning depressions and routing storage depth downslope.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.upslope_depression_storage(
    dem,
    output_path="result.tif",
)
```

### hydrologic_connectivity

```
hydrologic_connectivity(dem, exponent=1.1, convergence_threshold=0.0, z_factor=1.0, output1_path=None, output2_path=None, callback=None)
```

Computes two hydrologic-connectivity indices from a DEM: downslope unsaturated length (DUL) and upslope disconnected saturated area (UDSA).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `exponent` | float | no | Compatibility parameter for dispersion control. |
| `convergence_threshold` | float | no | Optional stream-initiation threshold in contributing cells. |
| `z_factor` | float | no | Optional vertical scaling factor. |
| `output1_path` | string | no | Optional output path for DUL raster. |
| `output2_path` | string | no | Optional output path for UDSA raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, Raster]` in this order:

- `output1`: `Raster`
- `output2`: `Raster`

**WbEnvironment usage**

```python
raster_1, raster_2 = wbe.hydrology.hydrologic_indices.hydrologic_connectivity(
    dem,
    exponent=1.0,
    convergence_threshold=1.0,
    z_factor=1.0,
    output1="value",
    output2="value",
)
```

### impoundment_size_index

```
impoundment_size_index(dem, max_dam_length, output_mean=False, output_max=False, output_volume=False, output_area=False, output_height=False, callback=None)
```

Estimates impoundment metrics for potential dams of a given maximum length at each DEM cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `max_dam_length` | float | yes | Maximum dam length in map units. |
| `output_mean` | bool | no | Include mean flooded-depth raster in output tuple. |
| `output_max` | bool | no | Include max flooded-depth raster in output tuple. |
| `output_volume` | bool | no | Include flooded-volume raster in output tuple. |
| `output_area` | bool | no | Include flooded-area raster in output tuple. |
| `output_height` | bool | no | Include dam-height raster in output tuple. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster | None, Raster | None, Raster | None, Raster | None, Raster | None]` in this order:

- `mean`: `Raster | None`
- `max`: `Raster | None`
- `volume`: `Raster | None`
- `area`: `Raster | None`
- `dam_height`: `Raster | None`

**WbEnvironment usage**

```python
raster_1, raster_2, raster_3, raster_4, raster_5 = wbe.hydrology.depressions_storage.impoundment_size_index(
    dem,
    max_dam_length=1.0,
    mean_output_path="mean.tif",
    max_output_path="max.tif",
    volume_output_path="volume.tif",
    area_output_path="area.tif",
    dam_height_output_path="dam_height.tif",
)
```

### num_inflowing_neighbours

```
num_inflowing_neighbours(dem, output_path=None, callback=None)
```

Counts the number of inflowing D8 neighbours for each DEM cell by deriving a D8 flow field from the DEM internally.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.num_inflowing_neighbours(
    dem,
    output_path="result.tif",
)
```

### find_parallel_flow

```
find_parallel_flow(d8_pointer, streams=None, output_path=None, callback=None)
```

Flags stream cells that have neighboring stream cells with the same local D8 flow direction, which can indicate D8 directional bias and suspect parallel channel routing.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Raster | yes | Input D8 pointer raster. |
| `streams` | Raster | no | Optional stream raster mask. If omitted, all valid cells are considered. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.find_parallel_flow(
    d8_pointer,
    streams,
    output_path="result.tif",
)
```

### edge_contamination

```
edge_contamination(dem, flow_type="mfd", z_factor=-1.0, output_path=None, callback=None)
```

Identifies edge-contaminated cells, i.e., cells whose upslope contributing area extends beyond the DEM boundary or boundary-connected NoData areas.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `flow_type` | string | no | Routing method to use: one of `"d8"`, `"mfd"`/`"fd8"`, or `"dinf"`. |
| `z_factor` | float | no | Optional vertical scaling factor. Values `<= 0` are treated as `1.0`. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.edge_contamination(
    dem,
    flow_type="value",
    z_factor=1.0,
    output_path="result.tif",
)
```

### flow_length_diff

```
flow_length_diff(d8_pointer, esri_pntr=False, log_transform=False, output_path=None, callback=None)
```

Calculates the local maximum absolute difference in downslope flowpath length, which is useful for highlighting drainage divides and ridgelines.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | Input D8 pointer raster. |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI D8 encoding. |
| `log_transform` | Any | yes | If true, apply natural-log transform to the output. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.flow_length_diff(
    d8_pntr,
    output_path="result.tif",
)
```

### downslope_flowpath_length

```
downslope_flowpath_length(d8_pointer, watersheds=None, weights=None, esri_pntr=False, output_path=None, callback=None)
```

Computes downslope flowpath length from each cell in a D8 pointer raster to its outlet. Optionally constrains paths within watershed IDs and applies per-cell distance weighting.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | Input D8 pointer raster. |
| `watersheds` | Raster | no | Optional watershed raster. When supplied, flowpath accumulation is truncated at watershed boundaries. |
| `weights` | Raster | no | Optional raster multiplier applied to each traversed step length. |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI D8 encoding. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.downslope_flowpath_length(
    d8_pntr,
    output_path="result.tif",
    watersheds,
    weights,
)
```

### max_upslope_flowpath_length

```
max_upslope_flowpath_length(dem, output_path=None, callback=None)
```

Computes the maximum upslope flowpath length passing through each DEM cell using D8 routing derived from the input DEM.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.max_upslope_flowpath_length(
    dem,
    output_path="result.tif",
)
```

### average_upslope_flowpath_length

```
average_upslope_flowpath_length(dem, output_path=None, callback=None)
```

Computes the average upslope flowpath length passing through each DEM cell using D8 routing derived from the input DEM.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.average_upslope_flowpath_length(
    dem,
    output_path="result.tif",
)
```

### elevation_above_stream

```
elevation_above_stream(dem, streams, output_path=None, callback=None)
```

Computes elevation above nearest stream measured along downslope D8 flowpaths (a HAND-like terrain index).

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `streams` | Raster | yes | Input stream raster; stream cells are values `> 0` and not NoData. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.elevation_above_stream(
    dem,
    streams,
    output_path="result.tif",
)
```

### elevation_above_stream_euclidean

```
elevation_above_stream_euclidean(dem, streams, output_path=None, callback=None)
```

Computes elevation above nearest stream using Euclidean proximity to assign each cell to the nearest stream cell, then subtracts stream elevation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `streams` | Raster | yes | Input stream raster; stream cells are values `> 0` and not NoData. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.elevation_above_stream_euclidean(
    dem,
    streams,
    output_path="result.tif",
)
```

### downslope_distance_to_stream

```
downslope_distance_to_stream(dem, streams, dinf=False, output_path=None, callback=None)
```

Computes distance from each cell to the nearest stream along downslope flowpaths. Supports D8 routing by default and optional D-infinity routing when `dinf=True`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `streams` | Raster | yes | Input stream raster; stream cells are values `> 0` and not NoData. |
| `dinf` | bool | no | If true, use D-infinity routing; otherwise uses D8 routing. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.downslope_distance_to_stream(
    dem,
    streams,
    output_path="result.tif",
)
```

### average_flowpath_slope

```
average_flowpath_slope(dem, output_path=None, callback=None)
```

Calculates average slope gradient in degrees for flowpaths passing through each DEM cell, using D8 flow routing derived from the DEM.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.average_flowpath_slope(
    dem,
    output_path="result.tif",
)
```

### max_upslope_value

```
max_upslope_value(dem, values, output_path=None, callback=None)
```

Propagates the maximum upslope value along D8 flowpaths over a DEM. Useful for carrying source characteristics downslope while preserving maxima.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `values` | Raster | yes | Input values raster to propagate. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.max_upslope_value(
    dem,
    values,
    output_path="result.tif",
)
```

### longest_flowpath

```
longest_flowpath(dem, basins, output_path, callback=None)
```

Delineates one longest downslope flowpath polyline for each basin in a basin raster. Output includes basin ID, upstream/downstream elevation, flowpath length, and average slope.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster. |
| `basins` | Raster | yes | Input basin raster with non-zero IDs for basin cells. |
| `output_path` | string | yes | Output vector path (required). |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.longest_flowpath(
    dem,
    basins,
    output_path="result.tif",
)
```

### depth_to_water

```
depth_to_water(dem, streams=None, lakes=None, output_path=None, callback=None)
```

Computes cartographic depth-to-water (DTW) by least-cost accumulation from mapped surface-water source features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `streams` | Vector | no | Optional stream vector layer (line or multiline). |
| `lakes` | Vector | no | Optional waterbody vector layer (polygon or multipolygon). |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.hydrologic_indices.depth_to_water(
    dem,
    streams,
    lakes,
    output_path="result.tif",
)
```

### fill_burn

```
fill_burn(dem, streams, output_path=None, callback=None)
```

Creates a hydro-enforced DEM by burning in stream locations and then filling depressions.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `streams` | Vector | yes | Input streams vector layer. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.fill_burn(
    dem,
    streams,
    output_path="result.tif",
)
```

### burn_streams_at_roads

```
burn_streams_at_roads(dem, streams, roads, road_width, output_path=None, callback=None)
```

Lowers stream elevations near stream-road intersections to breach embankment effects in a DEM.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `streams` | Vector | yes | Stream vector layer. |
| `roads` | Vector | yes | Road vector layer. |
| `road_width` | Any | yes | Maximum embankment width in map units used to set burn reach along the stream. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.burn_streams_at_roads(
    dem,
    streams,
    roads,
    output_path="result.tif",
    width=1.0,
)
```

### d8_mass_flux

```
d8_mass_flux(dem, loading, efficiency, absorption, output_path=None, callback=None)
```

Performs D8-based mass-flux routing, suitable for modeling movement of sediment, nutrients, or contaminants over a DEM-defined flow network.

The routed mass per cell follows:

$$
	ext{outflow} = (\text{loading} - \text{absorption} + \text{inflow}) \times \text{efficiency}
$$

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input depressionless DEM raster used to derive D8 flow directions. |
| `loading` | Raster | yes | Raster of initial mass loading values. |
| `efficiency` | Raster | yes | Raster of transfer efficiency values, either in `[0, 1]` or percent. |
| `absorption` | Raster | yes | Raster of per-cell mass losses in loading units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.flow_routing.d8_mass_flux(
    dem,
    loading,
    efficiency,
    absorption,
    output_path="result.tif",
)
```

### basins

```
basins(d8_pointer, esri_pntr=False, output_path=None, callback=None)
```

Delineates all drainage basins in a D8 pointer raster by assigning each valid cell to the edge-draining outlet basin reached along its D8 flow path.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | Input D8 pointer raster. |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI D8 encoding. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.basins(
    d8_pntr,
    output_path="result.tif",
)
```

### watershed_from_raster_pour_points

```
watershed_from_raster_pour_points(d8_pointer, pour_points, esri_pntr=False, output_path=None, callback=None)
```

Delineates watersheds from a D8 pointer raster and a raster of pour-point outlet IDs. Each non-zero, non-NoData cell in `pour_points` is treated as an outlet; its cell value becomes the watershed ID for all cells that drain to it.

Algorithm notes:
- Same two-pass flow-path labeling as `basins`, but seeded from user-supplied pour points rather than edge outlets.
- Watershed IDs are inherited directly from the pour-points raster values, making it easy to use stream-link or lake ID rasters as pour-point inputs.
- Cells where the D8 pointer is NoData are propagated as NoData in the output.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Raster | yes | Input D8 pointer raster. |
| `pour_points` | Raster | yes | Pour-points raster; non-zero, non-NoData cell values become outlet IDs. |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI D8 encoding. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.watershed_from_raster_pour_points(d8_pointer, pour_points)
```

### watershed

```
watershed(d8_pointer, pour_pts, esri_pntr=False, output_path=None, callback=None)
```

Delineates watersheds from a D8 pointer raster and a vector point file of pour points. Each vector feature is assigned a sequential 1-based watershed ID.

Algorithm notes:
- Pour-point coordinates are converted to raster row/col via the pointer raster's geotransform.
- Watershed IDs are 1-based sequential integers in feature insertion order.
- Same two-pass flow-path labeling as `watershed_from_raster_pour_points`.
- Only the first coordinate of each feature is used; MultiPoint features use their first point.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | Input D8 pointer raster. |
| `pour_pts` | Vector | yes | Input vector file of pour points (point or multipoint geometries). |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI D8 encoding. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.watershed(
    d8_pntr,
    pour_pts,
    output_path="result.tif",
)
```

### jenson_snap_pour_points

```
jenson_snap_pour_points(pour_pts, streams, snap_dist=0.0, output_path=None, callback=None)
```

Moves each input pour point to the nearest stream cell within a configurable search radius, snapping it onto the stream network. Preserves all input feature attributes.

Algorithm notes:
- For each point, a square window of `floor((snap_dist / cell_size) / 2)` cells is searched around the point's raster position.
- The nearest stream cell (value > 0 and not NoData) by squared Euclidean distance is chosen.
- If no stream cell is found within the window, the point is emitted at its original location.
- Points outside the raster extent are passed through unchanged.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `pour_pts` | Vector | yes | Input vector file of pour points (point or multipoint geometries). |
| `streams` | Raster | yes | Input stream-network raster where stream cells have value > 0 and are not NoData. |
| `snap_dist` | float | no | Maximum search radius in map units. Defaults to one cell width when omitted or zero. |
| `output_path` | string | no | Output path for the snapped pour-point vector file (required; defaults to `snapped_pour_points.geojson` in the working directory when not supplied to the wrapper). |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.jenson_snap_pour_points(
    pour_pts,
    streams,
    snap_dist=1.0,
    output_path="result.tif",
)
```

### snap_pour_points

```
snap_pour_points(pour_pts, flow_accum, snap_dist=0.0, output_path=None, callback=None)
```

Moves each pour point to the highest flow-accumulation cell within a local search window. Preserves all input feature attributes.

Algorithm notes:
- For each point, the tool scans a square search window centered on the point's raster position.
- The output location is set to the cell center of the maximum `flow_accum` value found in that window.
- If no valid cell exists in the search window (for example, all NoData), the point is emitted unchanged.
- Points outside the raster extent are emitted unchanged.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `pour_pts` | Vector | yes | Input vector file of pour points (point or multipoint geometries). |
| `flow_accum` | Raster | yes | Input flow-accumulation raster. |
| `snap_dist` | float | no | Maximum search radius in map units. Defaults to one cell width when omitted or zero. |
| `output_path` | string | no | Output path for the snapped pour-point vector file (required; defaults to `snapped_pour_points.geojson` in the working directory when not supplied to the wrapper). |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.snap_pour_points(
    pour_pts,
    flow_accum,
    snap_dist=1.0,
    output_path="result.tif",
)
```

### subbasins

```
subbasins(d8_pointer, streams, esri_pntr=False, output_path=None, callback=None)
```

Identifies the catchment area of each stream link in a D8 stream network, producing a raster where every cell is labelled with the ID of the sub-basin it drains to.

Algorithm notes:
- Performs a stream-link ID operation followed by a watershed operation.
- Headwater stream cells receive a unique link ID. At each downstream confluence (cell with more than one inflowing stream neighbour) a new link ID is assigned.
- All non-stream cells are labelled by walking downstream to the nearest stream-link seed.
- Differs from `hillslopes` in that stream cells themselves are also labelled (not zeroed) and no left/right bank separation is applied.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | D8 pointer raster produced by `d8_pointer`. |
| `streams` | Raster | yes | Stream-network raster where stream cells have value > 0 and are not NoData. |
| `esri_pntr` | bool | no | If true, interpret pointer values using ESRI encoding. Default `False`. |
| `output_path` | string | no | Optional output raster path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.subbasins(
    d8_pntr,
    streams,
    output_path="result.tif",
)
```

### hillslopes

```
hillslopes(d8_pointer, streams, esri_pntr=False, output_path=None, callback=None)
```

Identifies hillslope regions draining to each stream link, distinguishing left-bank and right-bank areas. Stream cells themselves are set to 0.

Algorithm notes:
- Performs the same stream-link ID and watershed labeling as `subbasins`.
- After labeling, all stream cells are zeroed.
- A flood-fill clump pass re-numbers spatially connected regions that share the same sub-basin ID, separating left- and right-bank hillslopes.
- Diagonal clump expansion is blocked when both adjacent cardinal cells are stream cells, preventing hillslopes from merging across the stream.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | D8 pointer raster. |
| `streams` | Raster | yes | Stream-network raster where stream cells have value > 0 and are not NoData. |
| `esri_pntr` | bool | no | ESRI pointer encoding flag. |
| `output_path` | string | no | Optional output raster path. |
| `callback` | function | no | Optional progress callback. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.hillslopes(
    d8_pntr,
    streams,
    output_path="result.tif",
)
```

### strahler_order_basins

```
strahler_order_basins(d8_pointer, streams, esri_pntr=False, output_path=None, callback=None)
```

Delineates watershed basins whose cells are labelled by the Horton-Strahler order of the stream link that drains them.

Algorithm notes:
- Assigns Strahler stream orders to all stream cells: headwaters receive order 1; at a confluence where two or more inflowing streams share the same order, the downstream order is incremented by 1.
- All non-stream cells are then labelled with the Strahler order of the stream link they drain into, using the same two-pass watershed labeling as `watershed`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `d8_pointer` | Any | yes | D8 pointer raster. |
| `streams` | Raster | yes | Stream-network raster where stream cells have value > 0 and are not NoData. |
| `esri_pntr` | bool | no | ESRI pointer encoding flag. |
| `output_path` | string | no | Optional output raster path. |
| `callback` | function | no | Optional progress callback. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.streams.ordering_metrics.strahler_order_basins(
    d8_pntr,
    streams,
    output_path="result.tif",
)
```

### isobasins

```
isobasins(dem, target_size, output_path=None, callback=None)
```

Divides a landscape into approximately equal-sized watersheds (isobasins) by placing pour points wherever flow accumulation first exceeds a target threshold.

Algorithm notes:
- Computes D8 flow direction internally from the DEM using steepest descent.
- Accumulates flow cell-by-cell in topological order from headwaters downstream.
- When a cell's cumulative upstream area reaches `target_size`, a pour point is created at that cell or (if it produces a closer-to-target split) at its largest-accumulation inflowing neighbour.
- All cells are then traced downstream to their nearest pour point and assigned that basin's ID.
- The DEM must have been hydrologically conditioned (depressions filled or breached) before use.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input hydrologically-conditioned DEM raster. |
| `target_size` | float | yes | Target isobasin area in number of grid cells (positive integer or float). |
| `output_path` | string | no | Optional output raster path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.watersheds_basins.isobasins(
    dem,
    target_size=1.0,
    output_path="result.tif",
)
```

### breach_depressions_least_cost

```
breach_depressions_least_cost(dem, max_cost=inf, max_dist=100, flat_increment=None, fill_deps=False, minimize_dist=False, output=None)
```

Breaches depressions using a constrained least-cost pathway search from pit cells.

Algorithm notes:
- Searches outward from pit cells for a lower outlet cell and cuts the least-cost breach channel through intervening terrain.
- Usually alters the DEM less than full filling because it carves narrow channels instead of raising entire depressions.
- Well suited to artificial barriers such as roads, berms, and embankments where a culvert-like breach is more realistic than filling.
- `fill_deps=True` is useful when a small number of depressions remain unresolved after breaching.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `max_cost` | float\|None | no | Maximum allowed breach cost. |
| `max_dist` | int | no | Maximum search distance in cells. |
| `flat_increment` | float\|None | no | Optional monotonic decrement increment. |
| `fill_deps` | bool | no | If true, fill unresolved depressions after breaching. |
| `minimize_dist` | bool | no | If true, distance-weight breach costs. |
| `output` | string | no | Optional output path. |

When to use:
- First-choice preprocessing for hydrologic conditioning in many LiDAR-derived DEM workflows.
- Best when preserving surrounding terrain is more important than enforcing a pure fill solution.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.breach_depressions_least_cost(
    dem,
    max_cost=1.0,
    max_dist=1,
    flat_increment=1.0,
    output_path="result.tif",
)
```

### breach_single_cell_pits

```
breach_single_cell_pits(dem, output=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Breaches single-cell pits by carving local one-cell channels toward lower second-ring neighbors.

Algorithm notes:
- Only targets isolated one-cell pit artifacts.
- Adjusts an adjacent cell to create a local breach path toward a lower cell in the surrounding 5x5 neighborhood.
- Very fast, but intentionally limited in scope; it does not solve larger depressions.

When to use:
- Cheap cleanup pass before a more complete breaching or filling step.
- Useful when DEM artifacts are dominated by isolated single-cell pits.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.breach_single_cell_pits(
    dem,
    output_path="result.tif",
)
```

### fill_depressions

```
fill_depressions(dem, fix_flats=True, flat_increment=0.0001, flat_resolution="garbrecht_martz", max_depth=inf, output=None)
```

Fills depressions using a priority-flood strategy with optional flat resolution and optional maximum fill depth.

Algorithm notes:
- Identifies depressions, raises them to spill elevation, and optionally imposes a very small gradient across resulting flats.
- `fix_flats=True` applies a small downstream gradient so later flow-routing tools do not stall on large flat surfaces.
- `flat_resolution="garbrecht_martz"` is the default and produces a more structured, less meandering flat-resolution pattern for D8-style routing. Use `"natural"` to recover the older natural-path behaviour that follows original within-flat topography more closely.
- `max_depth` can limit how much vertical filling is allowed, which is useful when deep excavations or reservoirs should not be completely removed.
- More aggressive than breaching because every enclosed depression is raised rather than selectively cut.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `fix_flats` | bool | no | If true, impose a small gradient across filled flats. |
| `flat_increment` | float\|None | no | Flat increment (default `0.0001`). |
| `flat_resolution` | Literal["garbrecht_martz", "natural"]\|None | no | Flat-resolution mode. One of `"garbrecht_martz"` or `"natural"`. |
| `max_depth` | float\|None | no | Maximum allowed fill depth. |
| `output` | string | no | Optional output path. |

When to use:
- Good general-purpose fill workflow when a complete depressionless DEM is required.
- Appropriate when breaching would create unrealistic long cuts or when a full-fill surface is preferred.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.fill_depressions(
    dem,
    flat_increment=1.0,
    max_depth=1.0,
    flat_resolution,
    output_path="result.tif",
)
```

### fill_depressions_planchon_and_darboux

```
fill_depressions_planchon_and_darboux(dem, fix_flats=True, flat_increment=0.0001, output=None)
```

Planchon-and-Darboux-compatible interface using the shared optimized fill backend.

Algorithm notes:
- Compatibility-oriented interface for the classic Planchon and Darboux depression-filling formulation.
- Included mainly for parity with legacy workflows rather than because it is the preferred modern option.
- In practice, `fill_depressions` or `breach_depressions_least_cost` will often be the better first choice.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `fix_flats` | bool | no | If true, impose a small gradient across filled flats. |
| `flat_increment` | float\|None | no | Flat increment (default `0.0001`). |
| `output` | string | no | Optional output path. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.fill_depressions_planchon_and_darboux(
    dem,
    flat_increment=1.0,
    output_path="result.tif",
)
```

### fill_depressions_wang_and_liu

```
fill_depressions_wang_and_liu(dem, fix_flats=True, flat_increment=0.0001, output=None)
```

Wang-and-Liu-compatible interface using the shared optimized fill backend.

Algorithm notes:
- Compatibility-oriented interface for the Wang and Liu priority-queue depression-filling method.
- Processes cells by spill elevation and is historically important, but is not the preferred default in this port.
- Best used when reproducing older Wang-and-Liu-based workflows or published methods.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `fix_flats` | bool | no | If true, impose a small gradient across filled flats. |
| `flat_increment` | float\|None | no | Flat increment (default `0.0001`). |
| `output` | string | no | Optional output path. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.fill_depressions_wang_and_liu(
    dem,
    flat_increment=1.0,
    output_path="result.tif",
)
```

### fill_pits

```
fill_pits(dem, output=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Fills single-cell pits by raising pit cells to the minimum neighboring elevation plus a small increment.

Algorithm notes:
- Only removes isolated one-cell pits.
- Leaves larger depressions unchanged.
- Minimal and fast, but much less complete than full depression filling or breaching.

When to use:
- Very lightweight preprocessing for obvious single-cell artifacts.
- A quick first pass before running `fill_depressions` or `breach_depressions_least_cost`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.fill_pits(
    dem,
    output_path="result.tif",
)
```

### depth_in_sink

```
depth_in_sink(dem, zero_background=False, output_path=None, callback=None)
```

Measures the vertical depth of each cell within topographic depressions by differencing a depression-filled DEM and the original DEM.

Algorithm notes:
- Internally generates a filled DEM surface and computes `filled_dem - dem` for each valid cell.
- Positive values indicate depression depth.
- Non-sink cells are assigned NoData by default, or `0.0` when `zero_background=True`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `zero_background` | bool | no | If true, assign `0.0` to cells outside sinks; otherwise assign NoData. |
| `output_path` | string | no | Optional output raster path. |
| `callback` | function | no | Optional progress callback. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.depth_in_sink(
    dem,
    output_path="result.tif",
)
```

### sink

```
sink(dem, zero_background=False, output_path=None, callback=None)
```

Creates a binary raster identifying cells that belong to topographic depressions.

Algorithm notes:
- Uses the same filled-vs-original DEM differencing approach as `depth_in_sink`.
- Cells with positive depth are classified as sink cells (`1`).
- Non-sink cells are assigned NoData by default, or `0.0` when `zero_background=True`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `zero_background` | bool | no | If true, assign `0.0` to cells outside sinks; otherwise assign NoData. |
| `output_path` | string | no | Optional output raster path. |
| `callback` | function | no | Optional progress callback. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.hydrology.depressions_storage.sink(
    dem,
    output_path="result.tif",
)
```

