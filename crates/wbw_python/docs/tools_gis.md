# GIS Tools

This document covers the GIS tools currently ported into the backend.

## GIS Workflow Products (Pro)

These workflow methods expose higher-level environmental monitoring and siting products.

### Workflow Product Index

- `wetland_hydrogeomorphic_classification`
- `urban_expansion_impact_assessment`
- `wind_turbine_siting`
- `solar_site_suitability_analysis`
- `corridor_mapping_intelligence`
- `landslide_susceptibility_assessment`
- `river_corridor_health_assessment`

### wetland_hydrogeomorphic_classification

```
wetland_hydrogeomorphic_classification(dem, wetland_mask, max_polygon_features=10000, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `wetland_mask` | Raster | yes | Input raster for `wetland_mask`. |
| `max_polygon_features` | int | no | Numeric parameter for `max_polygon_features`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(hgm_class_raster, wetland_polygons_vector, confidence_raster, summary_json_path)`.

**Outputs**

Returned as `tuple[Raster, Vector, Raster, str]` in this order:

- `cls`: `Raster`
- `polys`: `Vector`
- `conf`: `Raster`
- `summary`: `str`

Example:

```python
hgm, polygons, confidence, summary = wbe.terrain.workflow_products.wetland_hydrogeomorphic_classification(
	dem=dem,
	wetland_mask=wetland_mask,
)
```

### urban_expansion_impact_assessment

```
urban_expansion_impact_assessment(baseline_urban, scenario_urban, streams, habitat_sensitivity=None, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `baseline_urban` | Raster | yes | Input raster for `baseline_urban`. |
| `scenario_urban` | Raster | yes | Input raster for `scenario_urban`. |
| `streams` | Vector | yes | Input vector layer for `streams`. |
| `habitat_sensitivity` | Raster | no | Input raster for `habitat_sensitivity`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(impact_severity_raster, affected_streams_vector, habitat_loss_raster, summary_json_path)`.

**Outputs**

Returned as `tuple[Raster, Vector, Raster, str]` in this order:

- `impact`: `Raster`
- `affected`: `Vector`
- `habitat`: `Raster`
- `summary`: `str`

Example:

```python
impact, streams_out, habitat_loss, summary = wbe.terrain.workflow_products.urban_expansion_impact_assessment(
	baseline_urban=urban_2020,
	scenario_urban=urban_2035,
	streams=streams,
	habitat_sensitivity=habitat_sensitivity,
)
```

### wind_turbine_siting

```
wind_turbine_siting(dem, settlements, settlements_epsg=None, visibility_radius_meters=5000, min_slope_degrees=5.0, max_slope_degrees=35.0, profile="balanced", sweep_spec_json=None, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `settlements` | Vector | yes | Input vector layer for `settlements`. |
| `settlements_epsg` | int \|None | no | Numeric parameter for `settlements_epsg`. |
| `visibility_radius_meters` | int | no | Numeric parameter for `visibility_radius_meters`. |
| `min_slope_degrees` | float | no | Numeric parameter for `min_slope_degrees`. |
| `max_slope_degrees` | float | no | Numeric parameter for `max_slope_degrees`. |
| `profile` | string | no | String parameter for `profile`. |
| `sweep_spec_json` | string \|None | no | String parameter for `sweep_spec_json`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(siting_score_raster, confidence_raster, summary_json_path)`.

When `sweep_spec_json` is provided, the runtime also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map` outputs. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`).

**Outputs**

Returned as `tuple[Raster, Raster, str, str]` in this order:

- `score`: `Raster`
- `confidence`: `Raster`
- `summary`: `str`
- `threshold_sensitivity`: `str`

Example:

```python
score, confidence, summary = wbe.terrain.workflow_products.wind_turbine_siting(
	dem=dem,
	settlements=settlements,
	profile="balanced",
)
```

### solar_site_suitability_analysis

```
solar_site_suitability_analysis(dem, candidate_threshold=0.7, max_candidate_sites=200, sweep_spec_json=None, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `transmission_lines` | Vector | no | Input vector layer for `transmission_lines`. |
| `substations` | Vector | no | Input vector layer for `substations`. |
| `road_network` | Vector | no | Input vector layer for `road_network`. |
| `infra_weight_profile` | string | yes | String parameter for `infra_weight_profile`. |
| `candidate_threshold` | float | no | Numeric parameter for `candidate_threshold`. |
| `max_candidate_sites` | int | no | Numeric parameter for `max_candidate_sites`. |
| `sweep_spec_json` | string \|None | no | String parameter for `sweep_spec_json`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(suitability_score_raster, visual_impact_raster, candidate_sites_vector, summary_json_path)`.

When `sweep_spec_json` is provided, the runtime also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map` outputs. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`).

**Outputs**

Returned as `tuple[Raster, Raster, Vector, str]` in this order:

- `score`: `Raster`
- `impact`: `Raster`
- `sites`: `Vector`
- `summary`: `str`

Example:

```python
score, impact, sites, summary = wbe.terrain.workflow_products.solar_site_suitability_analysis(
	dem=dem,
	candidate_threshold=0.7,
)
```

### corridor_mapping_intelligence

```
corridor_mapping_intelligence(dem, start_features, end_features, constraints=None, cost_profile="slope_roughness", terminal_anchor_strategy="mixed", corridor_tolerance=0.15, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `start_features` | Vector | yes | Input vector layer for `start_features`. |
| `end_features` | Vector | yes | Input vector layer for `end_features`. |
| `constraints` | Vector | no | Input vector layer for `constraints`. |
| `cost_profile` | string | no | String parameter for `cost_profile`. |
| `terminal_anchor_strategy` | string | no | String parameter for `terminal_anchor_strategy`. |
| `corridor_tolerance` | float | no | Numeric parameter for `corridor_tolerance`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(cost_surface_raster, accumulated_cost_raster, optimal_route_vector, corridor_suitability_raster, summary_json_path)`.

Finds the terrain least-cost route and corridor suitability band for siting linear infrastructure (roads, pipelines, utility lines).
`start_features` and `end_features` are vector layers containing point and/or polygon features.
`cost_profile` is one of `"slope_only"`, `"slope_roughness"` (default), or `"conservative"`.
`terminal_anchor_strategy` is one of `"mixed"` (default), `"centroid_only"`, or `"boundary_only"`.
`corridor_tolerance` is the fractional cost margin above optimal for the suitability band (default 0.15).

Value proposition vs OSS least-cost building blocks:
- This is an end-to-end workflow product rather than a low-level routing primitive.
- It derives a terrain cost surface from DEM slope/roughness, runs least-cost routing, and emits both route geometry and corridor alternatives in one call.
- It supports polygon exclusion constraints and returns a summary JSON contract for reporting/automation.

Endpoint modeling note:
- Current API is vector-first for terminal modeling.
- Point features are used directly as candidate anchors.
- Polygon features contribute sampled boundary/centroid candidates, and the tool chooses a traversable anchor pair.
- `terminal_anchor_strategy` controls polygon anchor candidate generation.
- If start/end layers differ from DEM CRS, they are reprojected to the DEM CRS before routing.
- If constraints differ from DEM CRS, they are reprojected to the DEM CRS before exclusion masking.
- DEM, start/end layers, and optional constraints must include EPSG metadata for CRS harmonization.

QA-style outputs:
- `cost_surface_raster`, `accumulated_cost_raster`, and `corridor_suitability_raster` provide inspectable diagnostic surfaces.
- `optimal_route_vector` includes comparative route attributes (`ROUTE_LEN_M`, `MEAN_SLOPE`, `ROUTE_COST`, `PROFILE`).
- `summary_json_path` stores reproducible run metadata and key metrics.

**Outputs**

Returned as `tuple[Raster, Raster, Vector, Raster, str]` in this order:

- `cost_surface`: `Raster`
- `accumulated_cost`: `Raster`
- `optimal_route`: `Vector`
- `corridor_suitability`: `Raster`
- `summary`: `str`

Example:

```python
cost, acc_cost, route, suitability, summary = wbe.terrain.workflow_products.corridor_mapping_intelligence(
	dem=dem,
	start_features=start_features,
	end_features=end_features,
	cost_profile="slope_roughness",
	terminal_anchor_strategy="mixed",
	corridor_tolerance=0.15,
	output_prefix="output/access_road",
)
```

### landslide_susceptibility_assessment

```
landslide_susceptibility_assessment(dem, rainfall_intensity=None, profile="balanced", susceptibility_threshold=0.65, max_zone_features=5000, output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `rainfall_intensity` | Raster | no | Input raster for `rainfall_intensity`. |
| `profile` | string | no | String parameter for `profile`. |
| `susceptibility_threshold` | float | no | Numeric parameter for `susceptibility_threshold`. |
| `max_zone_features` | int | no | Numeric parameter for `max_zone_features`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(susceptibility_raster, trigger_pressure_raster, confidence_raster, risk_zones_vector, summary_json_path)`.

**Outputs**

Returned as `tuple[Raster, Raster, Raster, Vector, str]` in this order:

- `susceptibility`: `Raster`
- `trigger`: `Raster`
- `confidence`: `Raster`
- `zones`: `Vector`
- `summary`: `str`

Example:

```python
sus, trigger, confidence, zones, summary = wbe.terrain.workflow_products.landslide_susceptibility_assessment(
	dem=dem,
	rainfall_intensity=rainfall,
	profile="balanced",
)
```

### river_corridor_health_assessment

```
river_corridor_health_assessment(dem, streams, profile="balanced", output_prefix=None, callback=None)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `dem` | Raster | yes | Input DEM raster. |
| `streams` | Vector | yes | Input vector layer for `streams`. |
| `profile` | string | no | String parameter for `profile`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns `(erosion_pressure_raster, corridor_confidence_raster, stream_health_score_raster, restoration_zones_vector, summary_json_path)`.

**Outputs**

Returned as `tuple[Raster, Raster, Raster, Vector, str]` in this order:

- `erosion`: `Raster`
- `confidence`: `Raster`
- `health`: `Raster`
- `zones`: `Vector`
- `summary`: `str`

Example:

```python
erosion, confidence, health, restoration, summary = wbe.terrain.workflow_products.river_corridor_health_assessment(
	dem=dem,
	streams=streams,
	profile="balanced",
)
```

## GIS (Raster Overlay)

These tools compare or combine aligned raster stacks on a cell-by-cell basis. Use them when the rasters already share the same grid geometry and you want overlay-style summaries or index outputs.

### Overlay Tool Index

- `average_overlay`
- `count_if`
- `highest_position`
- `lowest_position`
- `max_absolute_overlay`
- `max_overlay`
- `min_absolute_overlay`
- `min_overlay`
- `multiply_overlay`
- `percent_equal_to`
- `percent_greater_than`
- `percent_less_than`
- `pick_from_list`
- `standard_deviation_overlay`
- `sum_overlay`
- `weighted_overlay`
- `weighted_sum`

## GIS (Raster Aggregation and Point-Block Rasterization)

These tools are GIS raster utilities but are not raster overlay operations. They aggregate rasters, create synthetic rasters, find extrema, interpolate from points, rasterize point blocks, or derive footprint geometries.

### Aggregation and Block Tool Index

- `aggregate_raster`
- `buffer_raster`
- `centroid_raster`
- `clump`
- `create_plane`
- `find_lowest_or_highest_points`
- `heat_map`
- `hexagonal_grid_from_raster_base`
- `hexagonal_grid_from_vector_base`
- `idw_interpolation`
- `layer_footprint_raster`
- `layer_footprint_vector`
- `map_features`
- `rectangular_grid_from_raster_base`
- `rectangular_grid_from_vector_base`
- `natural_neighbour_interpolation`
- `nearest_neighbour_interpolation`
- `modified_shepard_interpolation`
- `radial_basis_function_interpolation`
- `raster_cell_assignment`
- `nibble`
- `sieve`
- `tin_interpolation`
- `block_maximum`
- `block_minimum`

### find_lowest_or_highest_points

```
find_lowest_or_highest_points(input, output_type="lowest", output_path=None, callback=None)
```

Finds the lowest and/or highest raster cell locations and outputs them as vector points.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `output_type` | string | no | One of `lowest`, `highest`, or `both`. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived GeoJSON path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.find_lowest_or_highest_points(
    input,
    output_type="value",
    output_path="result.tif",
)
```

### aggregate_raster

```
aggregate_raster(input, aggregation_factor=2, aggregation_type="mean", output_path=None, callback=None)
```

Reduces raster resolution by aggregating fixed-size source blocks using `mean`, `sum`, `maximum`, `minimum`, or `range`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `aggregation_factor` | int | no | Integer block size in source cells. |
| `aggregation_type` | string | no | Aggregation statistic to compute. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.aggregate_raster(
    input,
    aggregation_factor=1,
    aggregation_type="value",
    output_path="result.tif",
)
```

### create_plane

```
create_plane(base, gradient, aspect, constant, output_path=None, callback=None)
```

Creates a raster from a planar equation using a base raster for output geometry.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Base raster providing output extent, resolution, and CRS. |
| `gradient` | float | yes | Plane slope gradient in degrees. |
| `aspect` | float | yes | Plane aspect in degrees. |
| `constant` | float | yes | Additive constant term. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.create_plane(
    base,
    gradient=1.0,
    aspect=1.0,
    constant=1.0,
    output_path="result.tif",
)
```

### centroid_raster

```
centroid_raster(input, output_path=None, callback=None)
```

Calculates centroid cells for positive patch IDs in a raster and returns both output raster and a textual report.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, str]` in this order:

- `result`: `Raster`
- `string_2`: `str`

**WbEnvironment usage**

```python
raster_1, string_2 = wbe.raster.centroid_raster(
    input,
    output_path="result.tif",
)
```

### buffer_raster

```
buffer_raster(input, buffer_size, grid_cell_units=False, output_path=None, callback=None)
```

Creates a binary buffer around non-zero, non-NoData raster cells.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster where non-zero cells are buffer targets. |
| `buffer_size` | float | yes | Buffer distance threshold. |
| `grid_cell_units` | bool | no | If `True`, interprets `buffer_size` in grid-cell units instead of map units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.distance_cost.buffer_raster(
    input,
    buffer_size=1.0,
    output_path="result.tif",
)
```

### clump

```
clump(input, diag=False, zero_background=False, output_path=None, callback=None)
```

Groups contiguous equal-valued raster cells into unique patch IDs.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input categorical raster. |
| `diag` | bool | no | If `True`, uses 8-neighbour connectivity; otherwise 4-neighbour. |
| `zero_background` | bool | no | If `True`, preserves zero-valued cells as background. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.clump(
    input,
    output_path="result.tif",
)
```

### nibble

```
nibble(input, mask, use_nodata=False, nibble_nodata=True, output_path=None, callback=None)
```

Fills background regions in a raster by propagating values from nearest foreground cells,
constrained by a mask raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster to fill. |
| `mask` | Raster | yes | Binary mask raster (non-zero cells are preserved/eligible). |
| `use_nodata` | bool | no | If `True`, treats input NoData as a class value during nibbling. |
| `nibble_nodata` | bool | no | If `True`, restores NoData behavior for masked NoData regions. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.nibble(
    input,
    mask,
    output_path="result.tif",
)
```

### sieve

```
sieve(input, threshold=1.0, zero_background=False, output_path=None, callback=None)
```

Removes small raster patches below a cell-count threshold by replacing them with neighbouring
larger-patch values.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input categorical raster. |
| `threshold` | float | no | Minimum patch size in grid cells to retain. |
| `zero_background` | bool | no | If `True`, preserves original zero-valued background as zero. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.sieve(
    input,
    threshold=1.0,
    output_path="result.tif",
)
```

### heat_map

```
heat_map(points, bandwidth, field_name=None, cell_size=None, base_raster=None, kernel_function="quartic", output_path=None, callback=None)
```

Generates a kernel-density heat map raster from point occurrences.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string\|None | no | Optional numeric weight field; if omitted, each point contributes weight `1`. |
| `bandwidth` | float | yes | Kernel bandwidth in map units. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `kernel_function` | string | no | Kernel function type such as `quartic`, `gaussian`, `triangular`, or `uniform`. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.heat_map(
    points,
    bandwidth=1.0,
    field_name="value",
    cell_size=1.0,
    base_raster,
    kernel_function="value",
    output_path="result.tif",
)
```

### idw_interpolation

```
idw_interpolation(points, field_name="FID", use_z=False, weight=2.0, radius=0.0, min_points=0, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using inverse-distance weighting.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `weight` | float | no | IDW distance exponent. |
| `radius` | float | no | Optional neighbourhood radius in map units. |
| `min_points` | int | no | Minimum number of neighbours to use. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.idw_interpolation(
    points,
    field_name="value",
    weight=1.0,
    radius=1.0,
    min_points=1,
    cell_size=1.0,
    base_raster,
    output_path="result.tif",
)
```

### layer_footprint_raster

```
layer_footprint_raster(input, output_path=None, callback=None)
```

Creates a rectangular polygon footprint from the full spatial extent of a raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived GeoJSON path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.layer_footprint_raster(
    input,
    output_path="result.tif",
)
```

### layer_footprint_vector

```
layer_footprint_vector(input, output_path=None, callback=None)
```

Creates a rectangular polygon footprint from the full bounding box of a vector layer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.layer_footprint_vector(
    input,
    output_path="result.tif",
)
```

### hexagonal_grid_from_raster_base

```
hexagonal_grid_from_raster_base(base, width, orientation="h", output_path=None, callback=None)
```

Creates a hexagonal polygon grid using the extent of a base raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Base raster controlling output extent. |
| `width` | float | yes | Hexagon width in map units. |
| `orientation` | string | no | Hexagon orientation (`"h"`/horizontal or `"v"`/vertical). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.hexagonal_grid_from_raster_base(
    base,
    width=1.0,
    orientation="value",
    output_path="result.tif",
)
```

### hexagonal_grid_from_vector_base

```
hexagonal_grid_from_vector_base(base, width, orientation="h", output_path=None, callback=None)
```

Creates a hexagonal polygon grid using the bounding extent of a base vector layer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Vector | yes | Base vector layer controlling output extent. |
| `width` | float | yes | Hexagon width in map units. |
| `orientation` | string | no | Hexagon orientation (`"h"`/horizontal or `"v"`/vertical). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.hexagonal_grid_from_vector_base(
    base,
    width=1.0,
    orientation="value",
    output_path="result.tif",
)
```

### rectangular_grid_from_raster_base

```
rectangular_grid_from_raster_base(base, width, height, x_origin=0.0, y_origin=0.0, output_path=None, callback=None)
```

Creates a rectangular polygon grid using the extent of a base raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Base raster controlling output extent. |
| `width` | float | yes | Grid cell width in map units. |
| `height` | float | yes | Grid cell height in map units. |
| `x_origin` | float | no | Optional x-origin used to align the grid. |
| `y_origin` | float | no | Optional y-origin used to align the grid. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.rectangular_grid_from_raster_base(
    base,
    width=1.0,
    height=1.0,
    x_origin=1.0,
    y_origin=1.0,
    output_path="result.tif",
)
```

### rectangular_grid_from_vector_base

```
rectangular_grid_from_vector_base(base, width, height, x_origin=0.0, y_origin=0.0, output_path=None, callback=None)
```

Creates a rectangular polygon grid using the bounding extent of a base vector layer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Vector | yes | Base vector layer controlling output extent. |
| `width` | float | yes | Grid cell width in map units. |
| `height` | float | yes | Grid cell height in map units. |
| `x_origin` | float | no | Optional x-origin used to align the grid. |
| `y_origin` | float | no | Optional y-origin used to align the grid. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.rectangular_grid_from_vector_base(
    base,
    width=1.0,
    height=1.0,
    x_origin=1.0,
    y_origin=1.0,
    output_path="result.tif",
)
```

### map_features

```
map_features(input, min_feature_height, min_feature_size=1, output_path=None, callback=None)
```

Labels discrete terrain features in a raster using descending-priority region growth and small-feature merging.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `min_feature_height` | float | yes | Minimum vertical separation required for separate features. |
| `min_feature_size` | int | no | Minimum retained feature size in cells. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.map_features(
    input,
    min_feature_height=1.0,
    min_feature_size=1,
    output_path="result.tif",
)
```

### natural_neighbour_interpolation

```
natural_neighbour_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, clip_to_hull=True, output_path=None, callback=None)
```

Interpolates a raster from point samples using Delaunay-neighbour weighted interpolation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `clip_to_hull` | bool | no | If `True`, limits interpolation to the points' convex hull. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.local_neighborhood.natural_neighbour_interpolation(
    points,
    field_name="value",
    cell_size=1.0,
    base_raster,
    output_path="result.tif",
)
```

### nearest_neighbour_interpolation

```
nearest_neighbour_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, max_dist=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using nearest-neighbour assignment.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `max_dist` | float\|None | no | Optional maximum search distance in map units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.local_neighborhood.nearest_neighbour_interpolation(
    points,
    field_name="value",
    cell_size=1.0,
    base_raster,
    max_dist=1.0,
    output_path="result.tif",
)
```

### modified_shepard_interpolation

```
modified_shepard_interpolation(points, field_name="FID", use_z=False, weight=2.0, radius=0.0, min_points=8, use_quadratic_basis=False, cell_size=None, base_raster=None, use_data_hull=False, output_path=None, callback=None)
```

Interpolates a raster from point samples using modified Shepard weighting.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `weight` | float | no | Shepard weight exponent. |
| `radius` | float | no | Optional neighbourhood radius in map units. |
| `min_points` | int | no | Minimum number of neighbours to use. |
| `use_quadratic_basis` | bool | no | Optional local basis flag (reserved for parity refinement). |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `use_data_hull` | bool | no | If `True`, limits interpolation to the points' convex hull. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.modified_shepard_interpolation(
    points,
    field_name="value",
    weight=1.0,
    radius=1.0,
    min_points=1,
    cell_size=1.0,
    base_raster,
    output_path="result.tif",
)
```

### radial_basis_function_interpolation

```
radial_basis_function_interpolation(points, field_name="FID", use_z=False, radius=0.0, min_points=16, cell_size=None, base_raster=None, func_type="thinplatespline", poly_order="none", weight=0.1, approximate_mode=True, output_path=None, callback=None)
```

Interpolates a raster from point samples using local radial-basis similarity weighting.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `radius` | float | no | Optional neighbourhood radius in map units. |
| `min_points` | int | no | Minimum number of neighbours to use. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `func_type` | Literal["thinplatespline", "polyharmonic", "gaussian", "multiquadric", "inversemultiquadric"] | no | Basis type (`thinplatespline`, `polyharmonic`, `gaussian`, `multiquadric`, `inversemultiquadric`). |
| `poly_order` | Literal["none", "constant", "quadratic"] | no | Polynomial order hint (`none`, `constant`, `quadratic`). |
| `weight` | float | no | Basis shape/exponent parameter. |
| `approximate_mode` | bool | no | If `True`, uses the NG approximate local neighborhood strategy; if `False`, uses legacy-style exhaustive evaluation. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.radial_basis_function_interpolation(
    points,
    field_name="value",
    radius=1.0,
    min_points=1,
    cell_size=1.0,
    base_raster,
    func_type,
    poly_order,
    weight=1.0,
    output_path="result.tif",
)
```

### tin_interpolation

```
tin_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, max_triangle_edge_length=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field; defaults to FID fallback. |
| `use_z` | bool | no | If `True`, uses point Z values instead of attributes. |
| `cell_size` | float\|None | no | Output cell size when `base_raster` is not provided. |
| `base_raster` | Raster | no | Optional base raster controlling output geometry. |
| `max_triangle_edge_length` | float\|None | no | Optional maximum allowed triangle edge length in map units. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.tin_interpolation(
    points,
    field_name="value",
    cell_size=1.0,
    base_raster,
    max_triangle_edge_length=1.0,
    output_path="result.tif",
)
```

### raster_cell_assignment

```
raster_cell_assignment(input, what_to_assign="column", output_path=None, callback=None)
```

Creates a raster from a base raster by assigning each cell its row number, column number, x coordinate, or y coordinate.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input base raster. |
| `what_to_assign` | Literal["column", "row", "x", "y"] | no | One of `column`, `row`, `x`, or `y`. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.raster_cell_assignment(
    input,
    what_to_assign,
    output_path="result.tif",
)
```

### block_maximum

```
block_maximum(points, field_name=None, use_z=False, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Rasterizes point features by assigning the maximum observed value within each output cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field. If omitted or unavailable, the tool falls back to feature IDs. |
| `use_z` | bool | no | When `True`, use point Z values instead of attributes. |
| `cell_size` | float | no | Output cell size when `base_raster` is not supplied. |
| `base_raster` | Raster | no | Optional raster supplying output geometry. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.block_maximum(
    points,
    field_name="value",
    cell_size=1.0,
    base_raster,
    output_path="result.tif",
)
```

### block_minimum

```
block_minimum(points, field_name=None, use_z=False, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Rasterizes point features by assigning the minimum observed value within each output cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `points` | Vector | yes | Input points vector layer. |
| `field_name` | string | no | Optional numeric attribute field. If omitted or unavailable, the tool falls back to feature IDs. |
| `use_z` | bool | no | When `True`, use point Z values instead of attributes. |
| `cell_size` | float | no | Output cell size when `base_raster` is not supplied. |
| `base_raster` | Raster | no | Optional raster supplying output geometry. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Bounding And Reclassification)

These tools create vector bounding geometries and reclassify labelled rasters.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.block_minimum(
    points,
    field_name="value",
    cell_size=1.0,
    base_raster,
    output_path="result.tif",
)
```

### Bounding And Reclassification Tool Index

- `minimum_convex_hull`
- `minimum_bounding_box`
- `minimum_bounding_circle`
- `minimum_bounding_envelope`
- `medoid`
- `reclass`
- `reclass_equal_interval`
- `filter_raster_features_by_area`

### medoid

```
medoid(input, output_path=None, callback=None)
```

Creates medoid point output from vector geometries; for point layers this returns one medoid for the full set, and for non-point layers one medoid per feature.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.medoid(
    input,
    output_path="result.tif",
)
```

### minimum_convex_hull

```
minimum_convex_hull(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates convex hull polygons around vector features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `individual_feature_hulls` | bool | no | If `True`, output one hull per input feature; if `False`, output one hull for the full layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.minimum_convex_hull(
    input,
    output_path="result.tif",
)
```

### minimum_bounding_box

```
minimum_bounding_box(input, min_criteria="area", individual_feature_hulls=True, output_path=None, callback=None)
```

Creates oriented minimum bounding box polygons around vector features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `min_criteria` | Literal["area", "perimeter", "length", "width"] | no | Optimization target (`"area"`, `"perimeter"`, `"length"`, or `"width"`). |
| `individual_feature_hulls` | bool | no | If `True`, output one box per input feature; if `False`, output one box for the full layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.minimum_bounding_box(
    input,
    min_criteria,
    output_path="result.tif",
)
```

### minimum_bounding_circle

```
minimum_bounding_circle(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates minimum enclosing circle polygons around vector features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `individual_feature_hulls` | bool | no | If `True`, output one circle per input feature; if `False`, output one circle for the full layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.minimum_bounding_circle(
    input,
    output_path="result.tif",
)
```

### minimum_bounding_envelope

```
minimum_bounding_envelope(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates axis-aligned minimum bounding envelope polygons around vector features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `individual_feature_hulls` | bool | no | If `True`, output one envelope per input feature; if `False`, output one envelope for the full layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.minimum_bounding_envelope(
    input,
    output_path="result.tif",
)
```

### reclass

```
reclass(input, reclass_values, assign_mode=False, output_path=None, callback=None)
```

Reclassifies raster values using either value ranges or exact assignment pairs.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `reclass_values` | list[list[float]] | yes | Reclassification rows. Use `[new, from, to_less_than]` for range mode, or `[new, old]` when `assign_mode=True`. |
| `assign_mode` | bool | no | If `True`, interpret `reclass_values` rows as exact assignment pairs. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.reclass_mask.reclass(
    input,
    [reclass_values_1, reclass_values_2],
    output_path="result.tif",
)
```

### reclass_equal_interval

```
reclass_equal_interval(input, interval_size, start_value=None, end_value=None, output_path=None, callback=None)
```

Reclassifies raster values into equal-width intervals.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `interval_size` | float | yes | Interval width used for binning. |
| `start_value` | float\|None | no | Optional lower bound of the reclassification range. |
| `end_value` | float\|None | no | Optional upper bound of the reclassification range. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.reclass_mask.reclass_equal_interval(
    input,
    interval_size=1.0,
    start_value=1.0,
    end_value=1.0,
    output_path="result.tif",
)
```

### filter_raster_features_by_area

```
filter_raster_features_by_area(input, threshold, zero_background=False, output_path=None, callback=None)
```

Removes integer-labelled raster features smaller than a cell-count threshold.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster containing integer-labelled features. |
| `threshold` | int | yes | Minimum feature size in cells to retain. |
| `zero_background` | bool | no | If `True`, removed features are assigned zero; otherwise they are assigned NoData. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Vector Overlay And Linework)

These tools perform vector overlay, line splitting/merging, and polygon generation from linework.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.filter_raster_features_by_area(
    input,
    threshold=1,
    output_path="result.tif",
)
```

### Vector Overlay And Linework Tool Index

- `centroid_vector`
- `clip`
- `difference`
- `dissolve`
- `eliminate_coincident_points`
- `erase`
- `extend_vector_lines`
- `extract_by_attribute`
- `snap_endnodes`
- `smooth_vectors`
- `split_vector_lines`
- `extract_nodes`
- `filter_vector_features_by_area`
- `intersect`
- `line_intersections`
- `merge_line_segments`
- `polygonize`
- `split_with_lines`
- `symmetrical_difference`
- `union`
- `voronoi_diagram`
- `travelling_salesman_problem`
- `construct_vector_tin`
- `vector_hex_binning`

### extract_by_attribute

```
extract_by_attribute(input, statement, output_path=None, callback=None)
```

Extracts vector features whose attributes satisfy a boolean expression.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `statement` | string | yes | Boolean expression evaluated against attribute field names. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.attribute_analysis.extract_by_attribute(
    input,
    statement="value",
    output_path="result.tif",
)
```

### extract_raster_values_at_points

```
extract_raster_values_at_points(rasters, points, output_path=None, callback=None)
```

Samples one or more rasters at point locations and writes the values to new `VALUE1`, `VALUE2`, ... fields on the output point layer. Returns `(vector, report_text)`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `rasters` | Raster | yes | List of input rasters to sample. |
| `points` | Vector | yes | Input points vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Vector, str]` in this order:

- `result`: `Vector`
- `string_2`: `str`

**WbEnvironment usage**

```python
vector_1, string_2 = wbe.vector.sampling_gridding.extract_raster_values_at_points(
    rasters,
    points,
    output_path="result.tif",
)
```

### centroid_vector

```
centroid_vector(input, output_path=None, callback=None)
```

Computes centroid points from vector features.

For point inputs, the output is one centroid point representing the mean location of all points.
For non-point inputs, the output contains one centroid point per input feature.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.centroid_vector(
    input,
    output_path="result.tif",
)
```

### clip

```
clip(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Clips input polygons to overlay polygon boundaries.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.clip(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### dissolve

```
dissolve(input, dissolve_field="", snap_tolerance=EPSILON, output_path=None, callback=None)
```

Removes shared polygon boundaries globally or within dissolve-field groups.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `dissolve_field` | string | no | Optional field name used to dissolve polygons within attribute groups. |
| `snap_tolerance` | float | no | Snapping tolerance used by topology operations. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.dissolve(
    input,
    dissolve_field="value",
    snap_tolerance=1.0,
    output_path="result.tif",
)
```

### extract_nodes

```
extract_nodes(input, output_path=None, callback=None)
```

Converts polyline or polygon vertices into output point features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polyline or polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.extract_nodes(
    input,
    output_path="result.tif",
)
```

### filter_vector_features_by_area

```
filter_vector_features_by_area(input, threshold, output_path=None, callback=None)
```

Removes polygon features smaller than the specified area threshold.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `threshold` | float | yes | Minimum polygon area to retain. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.attribute_analysis.filter_vector_features_by_area(
    input,
    threshold=1.0,
    output_path="result.tif",
)
```

### extend_vector_lines

```
extend_vector_lines(input, distance, extend_direction="both", output_path=None, callback=None)
```

Extends line features by moving the start endpoint, end endpoint, or both along the local line direction.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input line vector layer. |
| `distance` | float | yes | Extension distance in map units. |
| `extend_direction` | Literal["both", "start", "end"] | no | One of `"both"`, `"start"`, or `"end"`. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.extend_vector_lines(
    input,
    distance=1.0,
    extend_direction,
    output_path="result.tif",
)
```

### smooth_vectors

```
smooth_vectors(input, filter_size=3, output_path=None, callback=None)
```

Smooths polyline and polygon geometries using an odd-sized moving-average window.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polyline or polygon vector layer. |
| `filter_size` | int | no | Smoothing window size (odd integer >= 3; even values are adjusted to the next odd value). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.smooth_vectors(
    input,
    filter_size=1,
    output_path="result.tif",
)
```

### split_vector_lines

```
split_vector_lines(input, segment_length, output_path=None, callback=None)
```

Divides polyline features into segments of a maximum specified length. Each output segment becomes
a separate feature. The output attributes include `FID`, `PARENT_ID` (the 1-based index of the
originating input feature), and all other input attributes.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polyline vector layer. |
| `segment_length` | float | yes | Maximum segment length in map units (must be > 0). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.split_vector_lines(
    input,
    segment_length=1.0,
    output_path="result.tif",
)
```

### snap_endnodes

```
snap_endnodes(input, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Snaps nearby polyline start/end nodes to shared coordinates within the specified tolerance.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polyline vector layer. |
| `snap_tolerance` | float | no | Endpoint snapping tolerance in map units. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.snap_endnodes(
    input,
    snap_tolerance=1.0,
    output_path="result.tif",
)
```

### line_intersections

```
line_intersections(input1, input2, output_path=None, callback=None, snap_tolerance=None)
```

Finds intersection points between line or polygon boundaries in two input vector layers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Vector | yes | First input vector layer. |
| `input2` | Vector | yes | Second input vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional intersection snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.line_intersections(
    input1,
    input2,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### merge_line_segments

```
merge_line_segments(input, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Merges connected line segments whose endpoints match within the snap tolerance.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polyline vector layer. |
| `snap_tolerance` | float | no | Endpoint snapping tolerance. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.merge_line_segments(
    input,
    snap_tolerance=1.0,
    output_path="result.tif",
)
```

### polygonize

```
polygonize(input_layers, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Creates polygons from closed rings in one or more input line layers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_layers` | Vector | yes | List of input line vector layers. |
| `snap_tolerance` | float | no | Snapping tolerance used while polygonizing. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.polygonize(
    input_layers,
    snap_tolerance=1.0,
    output_path="result.tif",
)
```

### split_with_lines

```
split_with_lines(input, split_vector, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Splits line features in the input layer at intersections with a split line layer.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input line vector layer to split. |
| `split_vector` | Vector | yes | Line vector layer defining split locations. |
| `snap_tolerance` | float | no | Snapping tolerance used during splitting. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.split_with_lines(
    input,
    split_vector,
    snap_tolerance=1.0,
    output_path="result.tif",
)
```

### symmetrical_difference

```
symmetrical_difference(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Computes non-overlapping polygon regions from the input and overlay layers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.symmetrical_difference(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### voronoi_diagram

```
voronoi_diagram(input_points, output_path=None, callback=None)
```

Creates Voronoi (Thiessen) polygon cells from input point locations.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_points` | Vector | yes | Input point or multipoint vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.voronoi_diagram(
    input_points,
    output_path="result.tif",
)
```

### travelling_salesman_problem

```
travelling_salesman_problem(input, duration=60, output_path=None, callback=None)
```

Finds an approximate solution to the travelling salesman problem (TSP) for a set of points using 2-opt local search heuristics.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input point or multipoint vector layer. |
| `duration` | int | no | Maximum optimization duration in seconds (default: 60). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

Returns a polyline feature representing the optimal or near-optimal tour through the input points.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.network_analysis.travelling_salesman_problem(
    input,
    duration=1,
    output_path="result.tif",
)
```

### construct_vector_tin

```
construct_vector_tin(input_points, field_name="FID", max_triangle_edge_length=-1.0, output_path=None, callback=None)
```

Constructs a triangular irregular network (TIN) from point features using Delaunay triangulation.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_points` | Vector | yes | Input point or multipoint vector layer. |
| `field_name` | string | no | Numeric field name used as the z-value source when filtering triangle edge lengths (default: `"FID"`). |
| `max_triangle_edge_length` | float | no | Maximum allowable triangle edge length. Values <= 0 disable filtering. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.construct_vector_tin(
    input_points,
    field_name="value",
    max_triangle_edge_length=1.0,
    output_path="result.tif",
)
```

### vector_hex_binning

```
vector_hex_binning(vector_points, width, orientation="h", output_path=None, callback=None)
```

Bins point features into a generated hexagonal grid and writes per-cell point counts.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `vector_points` | Vector | yes | Input point vector layer. |
| `width` | float | yes | Hexagon width (distance between opposing sides). |
| `orientation` | string | no | Grid orientation (`"h"` for horizontal/pointy-top, `"v"` for vertical/flat-top). |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.sampling_gridding.vector_hex_binning(
    vector_points,
    width=1.0,
    orientation="value",
    output_path="result.tif",
)
```

### difference

```
difference(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Removes overlay polygon areas from input polygons.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.difference(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### eliminate_coincident_points

```
eliminate_coincident_points(input, tolerance_dist, output_path=None, callback=None)
```

Removes duplicate and near-duplicate points that fall within a specified distance tolerance.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input point vector layer. |
| `tolerance_dist` | float | yes | Distance threshold used to treat points as coincident. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.geometry_processing.eliminate_coincident_points(
    input,
    tolerance_dist=1.0,
    output_path="result.tif",
)
```

### erase

```
erase(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Erases overlay polygon areas from input polygons while preserving input attributes.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.erase(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### intersect

```
intersect(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Computes polygon intersections between input and overlay layers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.intersect(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### union

```
union(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Builds a unified polygon coverage from input and overlay layers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `overlay` | Vector | yes | Overlay polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `snap_tolerance` | float\|None | no | Optional overlay snapping tolerance. |

## GIS (Raster Polygon Masking)

These tools use polygon vectors to clip or erase cells from raster inputs.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.overlay_analysis.union(
    input,
    overlay,
    output_path="result.tif",
    snap_tolerance=1.0,
)
```

### Raster Polygon Masking Tool Index

- `clip_raster_to_polygon`
- `erase_polygon_from_raster`

### clip_raster_to_polygon

```
clip_raster_to_polygon(input, polygons, maintain_dimensions=False, output_path=None, callback=None)
```

Clips a raster to polygon coverage, setting cells outside polygons to NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `polygons` | Vector | yes | Input polygon vector layer. |
| `maintain_dimensions` | bool | no | If `True`, keep original raster dimensions; otherwise crop to polygon extent. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.clip_raster_to_polygon(
    input,
    polygons,
    output_path="result.tif",
)
```

### erase_polygon_from_raster

```
erase_polygon_from_raster(input, polygons, output_path=None, callback=None)
```

Sets raster cells inside polygons to NoData while preserving cells outside polygons.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `polygons` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.erase_polygon_from_raster(
    input,
    polygons,
    output_path="result.tif",
)
```

### average_overlay

```
average_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell average across a raster stack. NoData cells are ignored unless all inputs are NoData at that location.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.average_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### count_if

```
count_if(input_rasters, comparison_value, output_path=None, callback=None)
```

Counts how many rasters in the stack equal `comparison_value` at each cell. If all inputs are NoData at a cell, the output cell is NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `comparison_value` | float | yes | Numeric value to count within the stack. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.count_if(
    input_rasters,
    comparison_value=1.0,
    output_path="result.tif",
)
```

### highest_position

```
highest_position(input_rasters, output_path=None, callback=None)
```

Returns the zero-based input-stack index of the raster containing the highest value at each cell. If any input cell is NoData, the output cell is NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.highest_position(
    input_rasters,
    output_path="result.tif",
)
```

### lowest_position

```
lowest_position(input_rasters, output_path=None, callback=None)
```

Returns the zero-based input-stack index of the raster containing the lowest value at each cell. If any input cell is NoData, the output cell is NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.lowest_position(
    input_rasters,
    output_path="result.tif",
)
```

### max_overlay

```
max_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell maximum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.max_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### max_absolute_overlay

```
max_absolute_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell maximum absolute value across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.max_absolute_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### min_overlay

```
min_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell minimum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.min_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### min_absolute_overlay

```
min_absolute_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell minimum absolute value across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.min_absolute_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### multiply_overlay

```
multiply_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell product across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.multiply_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### percent_equal_to

```
percent_equal_to(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values equal the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `comparison` | Raster | yes | Comparison raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.percent_equal_to(
    input_rasters,
    comparison,
    output_path="result.tif",
)
```

### percent_greater_than

```
percent_greater_than(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values are greater than the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `comparison` | Raster | yes | Comparison raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.percent_greater_than(
    input_rasters,
    comparison,
    output_path="result.tif",
)
```

### percent_less_than

```
percent_less_than(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values are less than the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `comparison` | Raster | yes | Comparison raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.percent_less_than(
    input_rasters,
    comparison,
    output_path="result.tif",
)
```

### sum_overlay

```
sum_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell sum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.sum_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### pick_from_list

```
pick_from_list(input_rasters, pos_input, output_path=None, callback=None)
```

Selects per-cell values from an input raster stack using a zero-based position raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `pos_input` | Raster | yes | Raster containing zero-based indices into the raster stack. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.pick_from_list(
    input_rasters,
    pos_input,
    output_path="result.tif",
)
```

### weighted_overlay

```
weighted_overlay(factors, weights, cost=None, constraints=None, scale_max=1.0, output_path=None, callback=None)
```

Combines factor rasters using normalized weights, optional cost flags, and optional constraint rasters. Constraint cells with values less than or equal to zero force the output to zero.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `factors` | Raster | yes | Input factor raster stack as a Python list of rasters or raster paths. |
| `weights` | list[float] | yes | Numeric weights corresponding to each factor. |
| `cost` | list[bool]\|None | no | Optional list of booleans indicating whether each factor is a cost surface. |
| `constraints` | Raster | no | Optional list of raster constraints. |
| `scale_max` | float | no | Maximum scaled suitability value after per-factor normalization. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.weighted_overlay(
    factors,
    [weights_1, weights_2],
    [cost_1, cost_2],
    constraints,
    scale_max=1.0,
    output_path="result.tif",
)
```

### weighted_sum

```
weighted_sum(input_rasters, weights, output_path=None, callback=None)
```

Computes a weighted sum across a raster stack after normalizing weights so they sum to one.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `weights` | list[float] | yes | Numeric weights corresponding to each input raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.weighted_sum(
    input_rasters,
    [weights_1, weights_2],
    output_path="result.tif",
)
```

### standard_deviation_overlay

```
standard_deviation_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell standard deviation across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_rasters` | Raster | yes | Input raster stack as a Python list of rasters or raster paths. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Raster Value Updating)

These tools update raster values in-place by applying cell-wise value replacement logic between aligned rasters.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.standard_deviation_overlay(
    input_rasters,
    output_path="result.tif",
)
```

### Value Update Tool Index

- `update_nodata_cells`

### update_nodata_cells

```
update_nodata_cells(input1, input2, output_path=None, callback=None)
```

Assigns NoData cells in `input1` from corresponding valid cells in `input2`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input1` | Raster | yes | Primary raster to update. |
| `input2` | Raster | yes | Secondary raster supplying replacement values. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Distance And Cost Analysis)

These tools support Euclidean and friction/cost-based distance modelling workflows.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.overlay_math.update_nodata_cells(
    input1,
    input2,
    output_path="result.tif",
)
```

### Distance and Cost Tool Index

- `cost_allocation`
- `cost_distance`
- `cost_pathway`
- `euclidean_allocation`
- `euclidean_distance`

### cost_distance

```
cost_distance(source, cost, output_path=None, backlink_output_path=None, callback=None)
```

Computes accumulated cost distance from source cells over a cost/friction raster and outputs both the cost-accumulation raster and a backlink raster.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `source` | Raster | yes | Source raster with positive source cells. |
| `cost` | Raster | yes | Cost/friction raster. |
| `output_path` | string | no | Optional cost-accumulation output path. |
| `backlink_output_path` | string | no | Optional backlink output path. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

Returned as `tuple[Raster, Raster]` in this order:

- `result`: `Raster`
- `backlink`: `Raster`

**WbEnvironment usage**

```python
raster_1, raster_2 = wbe.raster.distance_cost.cost_distance(
    source,
    cost,
    output_path="result.tif",
    backlink_output="value",
)
```

### cost_allocation

```
cost_allocation(source, backlink, output_path=None, callback=None)
```

Assigns each cell to a source region using backlink connectivity from `cost_distance`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `source` | Raster | yes | Source raster with positive source cells. |
| `backlink` | Raster | yes | Backlink raster from `cost_distance`. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.distance_cost.cost_allocation(
    source,
    backlink,
    output_path="result.tif",
)
```

### cost_pathway

```
cost_pathway(destination, backlink, zero_background=False, output_path=None, callback=None)
```

Traces least-cost pathways from destination cells using backlink connectivity from `cost_distance`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `destination` | Raster | yes | Destination raster with positive destination cells. |
| `backlink` | Raster | yes | Backlink raster from `cost_distance`. |
| `zero_background` | bool | no | If `True`, set non-path cells to zero instead of NoData. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.distance_cost.cost_pathway(
    destination,
    backlink,
    output_path="result.tif",
)
```

### euclidean_distance

```
euclidean_distance(input, output_path=None, callback=None)
```

Computes Euclidean distance from each valid cell to the nearest non-zero target cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster with non-zero target cells. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.distance_cost.euclidean_distance(
    input,
    output_path="result.tif",
)
```

### euclidean_allocation

```
euclidean_allocation(input, output_path=None, callback=None)
```

Assigns each valid cell the value of the nearest non-zero target cell.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster with non-zero target cells. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Raster Polygon Metrics)

These tools estimate per-class polygon metrics from categorical rasters and write class totals back to each class cell.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.distance_cost.euclidean_allocation(
    input,
    output_path="result.tif",
)
```

### Polygon Metric Tool Index

- `polygon_area`
- `polygon_long_axis`
- `polygon_perimeter`
- `polygon_short_axis`
- `raster_area`
- `raster_perimeter`

### polygon_area

```
polygon_area(input, output_path=None, callback=None)
```

Calculates vector polygon area and appends an `AREA` field to the output.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.polygon_area(
    input,
    output_path="result.tif",
)
```

### polygon_perimeter

```
polygon_perimeter(input, output_path=None, callback=None)
```

Calculates vector polygon perimeter and appends a `PERIMETER` field to the output.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.polygon_perimeter(
    input,
    output_path="result.tif",
)
```

### polygon_short_axis

```
polygon_short_axis(input, output_path=None, callback=None)
```

Maps the short axis of each polygon's minimum bounding box to output line features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.polygon_short_axis(
    input,
    output_path="result.tif",
)
```

### polygon_long_axis

```
polygon_long_axis(input, output_path=None, callback=None)
```

Maps the long axis of each polygon's minimum bounding box to output line features.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.polygon_long_axis(
    input,
    output_path="result.tif",
)
```

### compactness_ratio

```
compactness_ratio(input, output_path=None, callback=None)
```

Computes compactness ratio for polygon features and appends `COMPACTNESS`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.compactness_ratio(
    input,
    output_path="result.tif",
)
```

### elongation_ratio

```
elongation_ratio(input, output_path=None, callback=None)
```

Computes polygon elongation ratio and appends `ELONGATION`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.elongation_ratio(
    input,
    output_path="result.tif",
)
```

### hole_proportion

```
hole_proportion(input, output_path=None, callback=None)
```

Computes polygon hole proportion and appends `HOLE_PROP`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.hole_proportion(
    input,
    output_path="result.tif",
)
```

### linearity_index

```
linearity_index(input, output_path=None, callback=None)
```

Computes linearity index and appends `LINEARITY`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input line or polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.linearity_index(
    input,
    output_path="result.tif",
)
```

### narrowness_index

```
narrowness_index(input, output_path=None, callback=None)
```

Computes raster narrowness index from each cell's local neighborhood.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.narrowness_index(
    input,
    output_path="result.tif",
)
```

### narrowness_index_vector

```
narrowness_index_vector(input, output_path=None, callback=None)
```

Computes narrowness index for polygon features and appends `NARROWNESS`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.narrowness_index_vector(
    input,
    output_path="result.tif",
)
```

### patch_orientation

```
patch_orientation(input, output_path=None, callback=None)
```

Computes patch orientation and appends `ORIENT`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.patch_orientation(
    input,
    output_path="result.tif",
)
```

### perimeter_area_ratio

```
perimeter_area_ratio(input, output_path=None, callback=None)
```

Computes perimeter-area ratio and appends `P_A_RATIO`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.perimeter_area_ratio(
    input,
    output_path="result.tif",
)
```

### related_circumscribing_circle

```
related_circumscribing_circle(input, output_path=None, callback=None)
```

Computes the related circumscribing circle metric and appends `RC_CIRCLE`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.related_circumscribing_circle(
    input,
    output_path="result.tif",
)
```

### shape_complexity_index_vector

```
shape_complexity_index_vector(input, output_path=None, callback=None)
```

Computes vector shape complexity index and appends `SCI`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.shape_complexity_index_vector(
    input,
    output_path="result.tif",
)
```

### deviation_from_regional_direction

```
deviation_from_regional_direction(input, elongation_threshold=0.75, output_path=None, callback=None)
```

Computes polygon directional deviation from the regional direction and appends `DEV_DIR`.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input polygon vector layer. |
| `elongation_threshold` | float | no | Threshold for including polygons in regional direction estimation. |
| `output_path` | string | no | Optional output vector path. If omitted, an auto-derived output path is used. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.vector.shape_metrics.deviation_from_regional_direction(
    input,
    elongation_threshold=1.0,
    output_path="result.tif",
)
```

### boundary_shape_complexity

```
boundary_shape_complexity(input, output_path=None, callback=None)
```

Computes raster patch boundary-shape complexity.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch-ID raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.boundary_shape_complexity(
    input,
    output_path="result.tif",
)
```

### edge_proportion

```
edge_proportion(input, output_path=None, callback=None)
```

Computes edge-cell proportion per raster patch.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch-ID raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.edge_proportion(
    input,
    output_path="result.tif",
)
```

### find_patch_edge_cells

```
find_patch_edge_cells(input, output_path=None, callback=None)
```

Identifies edge cells for each raster patch.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch-ID raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.find_patch_edge_cells(
    input,
    output_path="result.tif",
)
```

### radius_of_gyration

```
radius_of_gyration(input, output_path=None, callback=None)
```

Computes radius of gyration per raster patch.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch-ID raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.radius_of_gyration(
    input,
    output_path="result.tif",
)
```

### shape_complexity_index_raster

```
shape_complexity_index_raster(input, output_path=None, callback=None)
```

Computes raster patch shape complexity index.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input patch-ID raster. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.shape_complexity_index_raster(
    input,
    output_path="result.tif",
)
```

### raster_area

```
raster_area(input, units="map units", zero_background=False, output_path=None, callback=None)
```

Estimates per-class area from a categorical raster and assigns each class's total area to all cells of that class.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input categorical raster. |
| `units` | string | no | Area units (`"map units"` or `"grid cells"`). |
| `zero_background` | bool | no | If `True`, zero-valued cells are excluded. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.raster_area(
    input,
    units="value",
    output_path="result.tif",
)
```

### raster_perimeter

```
raster_perimeter(input, units="map units", zero_background=False, output_path=None, callback=None)
```

Estimates per-class perimeter from a categorical raster using an anti-aliasing lookup-table method and assigns each class's total perimeter to all cells of that class.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input categorical raster. |
| `units` | string | no | Perimeter units (`"map units"` or `"grid cells"`). |
| `zero_background` | bool | no | If `True`, zero-valued cells are excluded. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

## GIS (Raster Binary And Patch Tools)

These tools are used to derive binary proximity rasters and connected-component patch identifiers from categorical inputs.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.raster_perimeter(
    input,
    units="value",
    output_path="result.tif",
)
```

### Binary and Patch Tool Index

- `clump`

### clump

```
clump(input, diag=False, zero_background=False, output_path=None, callback=None)
```

Groups contiguous equal-valued cells into unique patch identifiers.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input categorical raster. |
| `diag` | bool | no | If `True`, include diagonal connectivity (8-neighbour); otherwise use 4-neighbour. |
| `zero_background` | bool | no | If `True`, keep zero-valued cells as background. |
| `output_path` | string | no | Optional output path. If omitted, returns an in-memory raster. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.raster.clump(
    input,
    output_path="result.tif",
)
```

