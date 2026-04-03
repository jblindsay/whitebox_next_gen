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

Returns `(hgm_class_raster, wetland_polygons_vector, confidence_raster, summary_json_path)`.

Example:

```python
hgm, polygons, confidence, summary = wbe.wetland_hydrogeomorphic_classification(
	dem=dem,
	wetland_mask=wetland_mask,
)
```

### urban_expansion_impact_assessment

```
urban_expansion_impact_assessment(baseline_urban, scenario_urban, streams, habitat_sensitivity=None, output_prefix=None, callback=None)
```

Returns `(impact_severity_raster, affected_streams_vector, habitat_loss_raster, summary_json_path)`.

Example:

```python
impact, streams_out, habitat_loss, summary = wbe.urban_expansion_impact_assessment(
	baseline_urban=urban_2020,
	scenario_urban=urban_2035,
	streams=streams,
	habitat_sensitivity=habitat_sensitivity,
)
```

### wind_turbine_siting

```
wind_turbine_siting(dem, settlements, settlements_epsg=None, visibility_radius_meters=5000, min_slope_degrees=5.0, max_slope_degrees=35.0, profile="balanced", output_prefix=None, callback=None)
```

Returns `(siting_score_raster, confidence_raster, summary_json_path)`.

Example:

```python
score, confidence, summary = wbe.wind_turbine_siting(
	dem=dem,
	settlements=settlements,
	profile="balanced",
)
```

### solar_site_suitability_analysis

```
solar_site_suitability_analysis(dem, candidate_threshold=0.7, max_candidate_sites=200, output_prefix=None, callback=None)
```

Returns `(suitability_score_raster, visual_impact_raster, candidate_sites_vector, summary_json_path)`.

Example:

```python
score, impact, sites, summary = wbe.solar_site_suitability_analysis(
	dem=dem,
	candidate_threshold=0.7,
)
```

### corridor_mapping_intelligence

```
corridor_mapping_intelligence(dem, start_features, end_features, constraints=None, cost_profile="slope_roughness", terminal_anchor_strategy="mixed", corridor_tolerance=0.15, output_prefix=None, callback=None)
```

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

Example:

```python
cost, acc_cost, route, suitability, summary = wbe.corridor_mapping_intelligence(
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

Returns `(susceptibility_raster, trigger_pressure_raster, confidence_raster, risk_zones_vector, summary_json_path)`.

Example:

```python
sus, trigger, confidence, zones, summary = wbe.landslide_susceptibility_assessment(
	dem=dem,
	rainfall_intensity=rainfall,
	profile="balanced",
)
```

### river_corridor_health_assessment

```
river_corridor_health_assessment(dem, streams, profile="balanced", output_prefix=None, callback=None)
```

Returns `(erosion_pressure_raster, corridor_confidence_raster, stream_health_score_raster, restoration_zones_vector, summary_json_path)`.

Example:

```python
erosion, confidence, health, restoration, summary = wbe.river_corridor_health_assessment(
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

Parameters:
- `input`: Input raster.
- `output_type`: One of `lowest`, `highest`, or `both`.
- `output_path`: Optional output vector path. If omitted, an auto-derived GeoJSON path is used.
- `callback`: Optional progress callback receiving JSON events.

### aggregate_raster

```
aggregate_raster(input, aggregation_factor=2, aggregation_type="mean", output_path=None, callback=None)
```

Reduces raster resolution by aggregating fixed-size source blocks using `mean`, `sum`, `maximum`, `minimum`, or `range`.

Parameters:
- `input`: Input raster.
- `aggregation_factor`: Integer block size in source cells.
- `aggregation_type`: Aggregation statistic to compute.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### create_plane

```
create_plane(base, gradient, aspect, constant, output_path=None, callback=None)
```

Creates a raster from a planar equation using a base raster for output geometry.

Parameters:
- `base`: Base raster providing output extent, resolution, and CRS.
- `gradient`: Plane slope gradient in degrees.
- `aspect`: Plane aspect in degrees.
- `constant`: Additive constant term.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### centroid_raster

```
centroid_raster(input, output_path=None, callback=None)
```

Calculates centroid cells for positive patch IDs in a raster and returns both output raster and a textual report.

Parameters:
- `input`: Input patch raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### buffer_raster

```
buffer_raster(input, buffer_size, grid_cell_units=False, output_path=None, callback=None)
```

Creates a binary buffer around non-zero, non-NoData raster cells.

Parameters:
- `input`: Input raster where non-zero cells are buffer targets.
- `buffer_size`: Buffer distance threshold.
- `grid_cell_units`: If `True`, interprets `buffer_size` in grid-cell units instead of map units.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### clump

```
clump(input, diag=False, zero_background=False, output_path=None, callback=None)
```

Groups contiguous equal-valued raster cells into unique patch IDs.

Parameters:
- `input`: Input categorical raster.
- `diag`: If `True`, uses 8-neighbour connectivity; otherwise 4-neighbour.
- `zero_background`: If `True`, preserves zero-valued cells as background.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### nibble

```
nibble(input, mask, use_nodata=False, nibble_nodata=True, output_path=None, callback=None)
```

Fills background regions in a raster by propagating values from nearest foreground cells,
constrained by a mask raster.

Parameters:
- `input`: Input raster to fill.
- `mask`: Binary mask raster (non-zero cells are preserved/eligible).
- `use_nodata`: If `True`, treats input NoData as a class value during nibbling.
- `nibble_nodata`: If `True`, restores NoData behavior for masked NoData regions.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### sieve

```
sieve(input, threshold=1.0, zero_background=False, output_path=None, callback=None)
```

Removes small raster patches below a cell-count threshold by replacing them with neighbouring
larger-patch values.

Parameters:
- `input`: Input categorical raster.
- `threshold`: Minimum patch size in grid cells to retain.
- `zero_background`: If `True`, preserves original zero-valued background as zero.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### heat_map

```
heat_map(points, bandwidth, field_name=None, cell_size=None, base_raster=None, kernel_function="quartic", output_path=None, callback=None)
```

Generates a kernel-density heat map raster from point occurrences.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric weight field; if omitted, each point contributes weight `1`.
- `bandwidth`: Kernel bandwidth in map units.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `kernel_function`: Kernel function type such as `quartic`, `gaussian`, `triangular`, or `uniform`.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### idw_interpolation

```
idw_interpolation(points, field_name="FID", use_z=False, weight=2.0, radius=0.0, min_points=0, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using inverse-distance weighting.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `weight`: IDW distance exponent.
- `radius`: Optional neighbourhood radius in map units.
- `min_points`: Minimum number of neighbours to use.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### layer_footprint_raster

```
layer_footprint_raster(input, output_path=None, callback=None)
```

Creates a rectangular polygon footprint from the full spatial extent of a raster.

Parameters:
- `input`: Input raster.
- `output_path`: Optional output vector path. If omitted, an auto-derived GeoJSON path is used.
- `callback`: Optional progress callback receiving JSON events.

### layer_footprint_vector

```
layer_footprint_vector(input, output_path=None, callback=None)
```

Creates a rectangular polygon footprint from the full bounding box of a vector layer.

Parameters:
- `input`: Input vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### hexagonal_grid_from_raster_base

```
hexagonal_grid_from_raster_base(base, width, orientation="h", output_path=None, callback=None)
```

Creates a hexagonal polygon grid using the extent of a base raster.

Parameters:
- `base`: Base raster controlling output extent.
- `width`: Hexagon width in map units.
- `orientation`: Hexagon orientation (`"h"`/horizontal or `"v"`/vertical).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### hexagonal_grid_from_vector_base

```
hexagonal_grid_from_vector_base(base, width, orientation="h", output_path=None, callback=None)
```

Creates a hexagonal polygon grid using the bounding extent of a base vector layer.

Parameters:
- `base`: Base vector layer controlling output extent.
- `width`: Hexagon width in map units.
- `orientation`: Hexagon orientation (`"h"`/horizontal or `"v"`/vertical).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### rectangular_grid_from_raster_base

```
rectangular_grid_from_raster_base(base, width, height, x_origin=0.0, y_origin=0.0, output_path=None, callback=None)
```

Creates a rectangular polygon grid using the extent of a base raster.

Parameters:
- `base`: Base raster controlling output extent.
- `width`: Grid cell width in map units.
- `height`: Grid cell height in map units.
- `x_origin`: Optional x-origin used to align the grid.
- `y_origin`: Optional y-origin used to align the grid.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### rectangular_grid_from_vector_base

```
rectangular_grid_from_vector_base(base, width, height, x_origin=0.0, y_origin=0.0, output_path=None, callback=None)
```

Creates a rectangular polygon grid using the bounding extent of a base vector layer.

Parameters:
- `base`: Base vector layer controlling output extent.
- `width`: Grid cell width in map units.
- `height`: Grid cell height in map units.
- `x_origin`: Optional x-origin used to align the grid.
- `y_origin`: Optional y-origin used to align the grid.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### map_features

```
map_features(input, min_feature_height, min_feature_size=1, output_path=None, callback=None)
```

Labels discrete terrain features in a raster using descending-priority region growth and small-feature merging.

Parameters:
- `input`: Input raster.
- `min_feature_height`: Minimum vertical separation required for separate features.
- `min_feature_size`: Minimum retained feature size in cells.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### natural_neighbour_interpolation

```
natural_neighbour_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, clip_to_hull=True, output_path=None, callback=None)
```

Interpolates a raster from point samples using Delaunay-neighbour weighted interpolation.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `clip_to_hull`: If `True`, limits interpolation to the points' convex hull.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### nearest_neighbour_interpolation

```
nearest_neighbour_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, max_dist=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using nearest-neighbour assignment.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `max_dist`: Optional maximum search distance in map units.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### modified_shepard_interpolation

```
modified_shepard_interpolation(points, field_name="FID", use_z=False, weight=2.0, radius=0.0, min_points=8, use_quadratic_basis=False, cell_size=None, base_raster=None, use_data_hull=False, output_path=None, callback=None)
```

Interpolates a raster from point samples using modified Shepard weighting.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `weight`: Shepard weight exponent.
- `radius`: Optional neighbourhood radius in map units.
- `min_points`: Minimum number of neighbours to use.
- `use_quadratic_basis`: Optional local basis flag (reserved for parity refinement).
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `use_data_hull`: If `True`, limits interpolation to the points' convex hull.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### radial_basis_function_interpolation

```
radial_basis_function_interpolation(points, field_name="FID", use_z=False, radius=0.0, min_points=16, cell_size=None, base_raster=None, func_type="thinplatespline", poly_order="none", weight=0.1, output_path=None, callback=None)
```

Interpolates a raster from point samples using local radial-basis similarity weighting.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `radius`: Optional neighbourhood radius in map units.
- `min_points`: Minimum number of neighbours to use.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `func_type`: Basis type (`thinplatespline`, `polyharmonic`, `gaussian`, `multiquadric`, `inversemultiquadric`).
- `poly_order`: Polynomial order hint (`none`, `constant`, `quadratic`).
- `weight`: Basis shape/exponent parameter.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### tin_interpolation

```
tin_interpolation(points, field_name="FID", use_z=False, cell_size=None, base_raster=None, max_triangle_edge_length=None, output_path=None, callback=None)
```

Interpolates a raster from point samples using Delaunay triangulation and planar interpolation within each triangle.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field; defaults to FID fallback.
- `use_z`: If `True`, uses point Z values instead of attributes.
- `cell_size`: Output cell size when `base_raster` is not provided.
- `base_raster`: Optional base raster controlling output geometry.
- `max_triangle_edge_length`: Optional maximum allowed triangle edge length in map units.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### raster_cell_assignment

```
raster_cell_assignment(input, what_to_assign="column", output_path=None, callback=None)
```

Creates a raster from a base raster by assigning each cell its row number, column number, x coordinate, or y coordinate.

Parameters:
- `input`: Input base raster.
- `what_to_assign`: One of `column`, `row`, `x`, or `y`.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### block_maximum

```
block_maximum(points, field_name=None, use_z=False, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Rasterizes point features by assigning the maximum observed value within each output cell.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field. If omitted or unavailable, the tool falls back to feature IDs.
- `use_z`: When `True`, use point Z values instead of attributes.
- `cell_size`: Output cell size when `base_raster` is not supplied.
- `base_raster`: Optional raster supplying output geometry.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### block_minimum

```
block_minimum(points, field_name=None, use_z=False, cell_size=None, base_raster=None, output_path=None, callback=None)
```

Rasterizes point features by assigning the minimum observed value within each output cell.

Parameters:
- `points`: Input points vector layer.
- `field_name`: Optional numeric attribute field. If omitted or unavailable, the tool falls back to feature IDs.
- `use_z`: When `True`, use point Z values instead of attributes.
- `cell_size`: Output cell size when `base_raster` is not supplied.
- `base_raster`: Optional raster supplying output geometry.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Bounding And Reclassification)

These tools create vector bounding geometries and reclassify labelled rasters.

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

Parameters:
- `input`: Input vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### minimum_convex_hull

```
minimum_convex_hull(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates convex hull polygons around vector features.

Parameters:
- `input`: Input vector layer.
- `individual_feature_hulls`: If `True`, output one hull per input feature; if `False`, output one hull for the full layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### minimum_bounding_box

```
minimum_bounding_box(input, min_criteria="area", individual_feature_hulls=True, output_path=None, callback=None)
```

Creates oriented minimum bounding box polygons around vector features.

Parameters:
- `input`: Input vector layer.
- `min_criteria`: Optimization target (`"area"`, `"perimeter"`, `"length"`, or `"width"`).
- `individual_feature_hulls`: If `True`, output one box per input feature; if `False`, output one box for the full layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### minimum_bounding_circle

```
minimum_bounding_circle(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates minimum enclosing circle polygons around vector features.

Parameters:
- `input`: Input vector layer.
- `individual_feature_hulls`: If `True`, output one circle per input feature; if `False`, output one circle for the full layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### minimum_bounding_envelope

```
minimum_bounding_envelope(input, individual_feature_hulls=True, output_path=None, callback=None)
```

Creates axis-aligned minimum bounding envelope polygons around vector features.

Parameters:
- `input`: Input vector layer.
- `individual_feature_hulls`: If `True`, output one envelope per input feature; if `False`, output one envelope for the full layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### reclass

```
reclass(input, reclass_values, assign_mode=False, output_path=None, callback=None)
```

Reclassifies raster values using either value ranges or exact assignment pairs.

Parameters:
- `input`: Input raster.
- `reclass_values`: Reclassification rows. Use `[new, from, to_less_than]` for range mode, or `[new, old]` when `assign_mode=True`.
- `assign_mode`: If `True`, interpret `reclass_values` rows as exact assignment pairs.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### reclass_equal_interval

```
reclass_equal_interval(input, interval_size, start_value=None, end_value=None, output_path=None, callback=None)
```

Reclassifies raster values into equal-width intervals.

Parameters:
- `input`: Input raster.
- `interval_size`: Interval width used for binning.
- `start_value`: Optional lower bound of the reclassification range.
- `end_value`: Optional upper bound of the reclassification range.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### filter_raster_features_by_area

```
filter_raster_features_by_area(input, threshold, zero_background=False, output_path=None, callback=None)
```

Removes integer-labelled raster features smaller than a cell-count threshold.

Parameters:
- `input`: Input raster containing integer-labelled features.
- `threshold`: Minimum feature size in cells to retain.
- `zero_background`: If `True`, removed features are assigned zero; otherwise they are assigned NoData.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Vector Overlay And Linework)

These tools perform vector overlay, line splitting/merging, and polygon generation from linework.

### Vector Overlay And Linework Tool Index

- `buffer_vector`
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

### buffer_vector

```
buffer_vector(input, distance, quadrant_segments=8, cap_style="round", join_style="round", mitre_limit=5.0, output_path=None, callback=None)
```

Creates polygon buffers around vector geometries with configurable cap and join styles.

Parameters:
- `input`: Input vector layer.
- `distance`: Buffer distance in map units.
- `quadrant_segments`: Arc resolution in segments per quadrant.
- `cap_style`: Line cap style (`"round"`, `"flat"`, or `"square"`).
- `join_style`: Corner join style (`"round"`, `"bevel"`, or `"mitre"`).
- `mitre_limit`: Mitre join limit when `join_style="mitre"`.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### extract_by_attribute

```
extract_by_attribute(input, statement, output_path=None, callback=None)
```

Extracts vector features whose attributes satisfy a boolean expression.

Parameters:
- `input`: Input vector layer.
- `statement`: Boolean expression evaluated against attribute field names.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### extract_raster_values_at_points

```
extract_raster_values_at_points(rasters, points, output_path=None, callback=None)
```

Samples one or more rasters at point locations and writes the values to new `VALUE1`, `VALUE2`, ... fields on the output point layer. Returns `(vector, report_text)`.

Parameters:
- `rasters`: List of input rasters to sample.
- `points`: Input points vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### centroid_vector

```
centroid_vector(input, output_path=None, callback=None)
```

Computes centroid points from vector features.

For point inputs, the output is one centroid point representing the mean location of all points.
For non-point inputs, the output contains one centroid point per input feature.

Parameters:
- `input`: Input vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### clip

```
clip(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Clips input polygons to overlay polygon boundaries.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

### dissolve

```
dissolve(input, dissolve_field="", snap_tolerance=EPSILON, output_path=None, callback=None)
```

Removes shared polygon boundaries globally or within dissolve-field groups.

Parameters:
- `input`: Input polygon vector layer.
- `dissolve_field`: Optional field name used to dissolve polygons within attribute groups.
- `snap_tolerance`: Snapping tolerance used by topology operations.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### extract_nodes

```
extract_nodes(input, output_path=None, callback=None)
```

Converts polyline or polygon vertices into output point features.

Parameters:
- `input`: Input polyline or polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### filter_vector_features_by_area

```
filter_vector_features_by_area(input, threshold, output_path=None, callback=None)
```

Removes polygon features smaller than the specified area threshold.

Parameters:
- `input`: Input polygon vector layer.
- `threshold`: Minimum polygon area to retain.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### extend_vector_lines

```
extend_vector_lines(input, distance, extend_direction="both", output_path=None, callback=None)
```

Extends line features by moving the start endpoint, end endpoint, or both along the local line direction.

Parameters:
- `input`: Input line vector layer.
- `distance`: Extension distance in map units.
- `extend_direction`: One of `"both"`, `"start"`, or `"end"`.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### smooth_vectors

```
smooth_vectors(input, filter_size=3, output_path=None, callback=None)
```

Smooths polyline and polygon geometries using an odd-sized moving-average window.

Parameters:
- `input`: Input polyline or polygon vector layer.
- `filter_size`: Smoothing window size (odd integer >= 3; even values are adjusted to the next odd value).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### split_vector_lines

```
split_vector_lines(input, segment_length, output_path=None, callback=None)
```

Divides polyline features into segments of a maximum specified length. Each output segment becomes
a separate feature. The output attributes include `FID`, `PARENT_ID` (the 1-based index of the
originating input feature), and all other input attributes.

Parameters:
- `input`: Input polyline vector layer.
- `segment_length`: Maximum segment length in map units (must be > 0).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### snap_endnodes

```
snap_endnodes(input, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Snaps nearby polyline start/end nodes to shared coordinates within the specified tolerance.

Parameters:
- `input`: Input polyline vector layer.
- `snap_tolerance`: Endpoint snapping tolerance in map units.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### line_intersections

```
line_intersections(input1, input2, output_path=None, callback=None, snap_tolerance=None)
```

Finds intersection points between line or polygon boundaries in two input vector layers.

Parameters:
- `input1`: First input vector layer.
- `input2`: Second input vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional intersection snapping tolerance.

### merge_line_segments

```
merge_line_segments(input, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Merges connected line segments whose endpoints match within the snap tolerance.

Parameters:
- `input`: Input polyline vector layer.
- `snap_tolerance`: Endpoint snapping tolerance.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### polygonize

```
polygonize(input_layers, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Creates polygons from closed rings in one or more input line layers.

Parameters:
- `input_layers`: List of input line vector layers.
- `snap_tolerance`: Snapping tolerance used while polygonizing.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### split_with_lines

```
split_with_lines(input, split_vector, snap_tolerance=EPSILON, output_path=None, callback=None)
```

Splits line features in the input layer at intersections with a split line layer.

Parameters:
- `input`: Input line vector layer to split.
- `split_vector`: Line vector layer defining split locations.
- `snap_tolerance`: Snapping tolerance used during splitting.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### symmetrical_difference

```
symmetrical_difference(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Computes non-overlapping polygon regions from the input and overlay layers.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

### voronoi_diagram

```
voronoi_diagram(input_points, output_path=None, callback=None)
```

Creates Voronoi (Thiessen) polygon cells from input point locations.

Parameters:
- `input_points`: Input point or multipoint vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### travelling_salesman_problem

```
travelling_salesman_problem(input, duration=60, output_path=None, callback=None)
```

Finds an approximate solution to the travelling salesman problem (TSP) for a set of points using 2-opt local search heuristics.

Parameters:
- `input`: Input point or multipoint vector layer.
- `duration`: Maximum optimization duration in seconds (default: 60).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

Returns a polyline feature representing the optimal or near-optimal tour through the input points.

### construct_vector_tin

```
construct_vector_tin(input_points, field_name="FID", max_triangle_edge_length=-1.0, output_path=None, callback=None)
```

Constructs a triangular irregular network (TIN) from point features using Delaunay triangulation.

Parameters:
- `input_points`: Input point or multipoint vector layer.
- `field_name`: Numeric field name used as the z-value source when filtering triangle edge lengths (default: `"FID"`).
- `max_triangle_edge_length`: Maximum allowable triangle edge length. Values <= 0 disable filtering.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### vector_hex_binning

```
vector_hex_binning(vector_points, width, orientation="h", output_path=None, callback=None)
```

Bins point features into a generated hexagonal grid and writes per-cell point counts.

Parameters:
- `vector_points`: Input point vector layer.
- `width`: Hexagon width (distance between opposing sides).
- `orientation`: Grid orientation (`"h"` for horizontal/pointy-top, `"v"` for vertical/flat-top).
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### difference

```
difference(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Removes overlay polygon areas from input polygons.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

### eliminate_coincident_points

```
eliminate_coincident_points(input, tolerance_dist, output_path=None, callback=None)
```

Removes duplicate and near-duplicate points that fall within a specified distance tolerance.

Parameters:
- `input`: Input point vector layer.
- `tolerance_dist`: Distance threshold used to treat points as coincident.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### erase

```
erase(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Erases overlay polygon areas from input polygons while preserving input attributes.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

### intersect

```
intersect(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Computes polygon intersections between input and overlay layers.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

### union

```
union(input, overlay, output_path=None, callback=None, snap_tolerance=None)
```

Builds a unified polygon coverage from input and overlay layers.

Parameters:
- `input`: Input polygon vector layer.
- `overlay`: Overlay polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.
- `snap_tolerance`: Optional overlay snapping tolerance.

## GIS (Raster Polygon Masking)

These tools use polygon vectors to clip or erase cells from raster inputs.

### Raster Polygon Masking Tool Index

- `clip_raster_to_polygon`
- `erase_polygon_from_raster`

### clip_raster_to_polygon

```
clip_raster_to_polygon(input, polygons, maintain_dimensions=False, output_path=None, callback=None)
```

Clips a raster to polygon coverage, setting cells outside polygons to NoData.

Parameters:
- `input`: Input raster.
- `polygons`: Input polygon vector layer.
- `maintain_dimensions`: If `True`, keep original raster dimensions; otherwise crop to polygon extent.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### erase_polygon_from_raster

```
erase_polygon_from_raster(input, polygons, output_path=None, callback=None)
```

Sets raster cells inside polygons to NoData while preserving cells outside polygons.

Parameters:
- `input`: Input raster.
- `polygons`: Input polygon vector layer.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### average_overlay

```
average_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell average across a raster stack. NoData cells are ignored unless all inputs are NoData at that location.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### count_if

```
count_if(input_rasters, comparison_value, output_path=None, callback=None)
```

Counts how many rasters in the stack equal `comparison_value` at each cell. If all inputs are NoData at a cell, the output cell is NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `comparison_value`: Numeric value to count within the stack.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### highest_position

```
highest_position(input_rasters, output_path=None, callback=None)
```

Returns the zero-based input-stack index of the raster containing the highest value at each cell. If any input cell is NoData, the output cell is NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### lowest_position

```
lowest_position(input_rasters, output_path=None, callback=None)
```

Returns the zero-based input-stack index of the raster containing the lowest value at each cell. If any input cell is NoData, the output cell is NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### max_overlay

```
max_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell maximum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### max_absolute_overlay

```
max_absolute_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell maximum absolute value across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### min_overlay

```
min_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell minimum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### min_absolute_overlay

```
min_absolute_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell minimum absolute value across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### multiply_overlay

```
multiply_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell product across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### percent_equal_to

```
percent_equal_to(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values equal the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `comparison`: Comparison raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### percent_greater_than

```
percent_greater_than(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values are greater than the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `comparison`: Comparison raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### percent_less_than

```
percent_less_than(input_rasters, comparison, output_path=None, callback=None)
```

Computes the fraction of rasters in the input stack whose values are less than the comparison raster at each cell. Any NoData in the comparison raster or input stack causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `comparison`: Comparison raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### sum_overlay

```
sum_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell sum across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### pick_from_list

```
pick_from_list(input_rasters, pos_input, output_path=None, callback=None)
```

Selects per-cell values from an input raster stack using a zero-based position raster.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `pos_input`: Raster containing zero-based indices into the raster stack.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### weighted_overlay

```
weighted_overlay(factors, weights, cost=None, constraints=None, scale_max=1.0, output_path=None, callback=None)
```

Combines factor rasters using normalized weights, optional cost flags, and optional constraint rasters. Constraint cells with values less than or equal to zero force the output to zero.

Parameters:
- `factors`: Input factor raster stack as a Python list of rasters or raster paths.
- `weights`: Numeric weights corresponding to each factor.
- `cost`: Optional list of booleans indicating whether each factor is a cost surface.
- `constraints`: Optional list of raster constraints.
- `scale_max`: Maximum scaled suitability value after per-factor normalization.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### weighted_sum

```
weighted_sum(input_rasters, weights, output_path=None, callback=None)
```

Computes a weighted sum across a raster stack after normalizing weights so they sum to one.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `weights`: Numeric weights corresponding to each input raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### standard_deviation_overlay

```
standard_deviation_overlay(input_rasters, output_path=None, callback=None)
```

Computes the per-cell standard deviation across a raster stack. Any NoData input cell causes the corresponding output cell to be NoData.

Parameters:
- `input_rasters`: Input raster stack as a Python list of rasters or raster paths.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Raster Value Updating)

These tools update raster values in-place by applying cell-wise value replacement logic between aligned rasters.

### Value Update Tool Index

- `update_nodata_cells`

### update_nodata_cells

```
update_nodata_cells(input1, input2, output_path=None, callback=None)
```

Assigns NoData cells in `input1` from corresponding valid cells in `input2`.

Parameters:
- `input1`: Primary raster to update.
- `input2`: Secondary raster supplying replacement values.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Distance And Cost Analysis)

These tools support Euclidean and friction/cost-based distance modelling workflows.

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

Parameters:
- `source`: Source raster with positive source cells.
- `cost`: Cost/friction raster.
- `output_path`: Optional cost-accumulation output path.
- `backlink_output_path`: Optional backlink output path.
- `callback`: Optional progress callback receiving JSON events.

### cost_allocation

```
cost_allocation(source, backlink, output_path=None, callback=None)
```

Assigns each cell to a source region using backlink connectivity from `cost_distance`.

Parameters:
- `source`: Source raster with positive source cells.
- `backlink`: Backlink raster from `cost_distance`.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### cost_pathway

```
cost_pathway(destination, backlink, zero_background=False, output_path=None, callback=None)
```

Traces least-cost pathways from destination cells using backlink connectivity from `cost_distance`.

Parameters:
- `destination`: Destination raster with positive destination cells.
- `backlink`: Backlink raster from `cost_distance`.
- `zero_background`: If `True`, set non-path cells to zero instead of NoData.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### euclidean_distance

```
euclidean_distance(input, output_path=None, callback=None)
```

Computes Euclidean distance from each valid cell to the nearest non-zero target cell.

Parameters:
- `input`: Input raster with non-zero target cells.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### euclidean_allocation

```
euclidean_allocation(input, output_path=None, callback=None)
```

Assigns each valid cell the value of the nearest non-zero target cell.

Parameters:
- `input`: Input raster with non-zero target cells.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Raster Polygon Metrics)

These tools estimate per-class polygon metrics from categorical rasters and write class totals back to each class cell.

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

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### polygon_perimeter

```
polygon_perimeter(input, output_path=None, callback=None)
```

Calculates vector polygon perimeter and appends a `PERIMETER` field to the output.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### polygon_short_axis

```
polygon_short_axis(input, output_path=None, callback=None)
```

Maps the short axis of each polygon's minimum bounding box to output line features.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### polygon_long_axis

```
polygon_long_axis(input, output_path=None, callback=None)
```

Maps the long axis of each polygon's minimum bounding box to output line features.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### compactness_ratio

```
compactness_ratio(input, output_path=None, callback=None)
```

Computes compactness ratio for polygon features and appends `COMPACTNESS`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### elongation_ratio

```
elongation_ratio(input, output_path=None, callback=None)
```

Computes polygon elongation ratio and appends `ELONGATION`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### hole_proportion

```
hole_proportion(input, output_path=None, callback=None)
```

Computes polygon hole proportion and appends `HOLE_PROP`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### linearity_index

```
linearity_index(input, output_path=None, callback=None)
```

Computes linearity index and appends `LINEARITY`.

Parameters:
- `input`: Input line or polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### narrowness_index

```
narrowness_index(input, output_path=None, callback=None)
```

Computes narrowness index and appends `NARROWNESS`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### patch_orientation

```
patch_orientation(input, output_path=None, callback=None)
```

Computes patch orientation and appends `ORIENT`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### perimeter_area_ratio

```
perimeter_area_ratio(input, output_path=None, callback=None)
```

Computes perimeter-area ratio and appends `P_A_RATIO`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### related_circumscribing_circle

```
related_circumscribing_circle(input, output_path=None, callback=None)
```

Computes the related circumscribing circle metric and appends `RC_CIRCLE`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### shape_complexity_index_vector

```
shape_complexity_index_vector(input, output_path=None, callback=None)
```

Computes vector shape complexity index and appends `SCI`.

Parameters:
- `input`: Input polygon vector layer.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### deviation_from_regional_direction

```
deviation_from_regional_direction(input, elongation_threshold=0.75, output_path=None, callback=None)
```

Computes polygon directional deviation from the regional direction and appends `DEV_DIR`.

Parameters:
- `input`: Input polygon vector layer.
- `elongation_threshold`: Threshold for including polygons in regional direction estimation.
- `output_path`: Optional output vector path. If omitted, an auto-derived output path is used.
- `callback`: Optional progress callback receiving JSON events.

### boundary_shape_complexity

```
boundary_shape_complexity(input, output_path=None, callback=None)
```

Computes raster patch boundary-shape complexity.

Parameters:
- `input`: Input patch-ID raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### edge_proportion

```
edge_proportion(input, output_path=None, callback=None)
```

Computes edge-cell proportion per raster patch.

Parameters:
- `input`: Input patch-ID raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### find_patch_edge_cells

```
find_patch_edge_cells(input, output_path=None, callback=None)
```

Identifies edge cells for each raster patch.

Parameters:
- `input`: Input patch-ID raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### radius_of_gyration

```
radius_of_gyration(input, output_path=None, callback=None)
```

Computes radius of gyration per raster patch.

Parameters:
- `input`: Input patch-ID raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### shape_complexity_index_raster

```
shape_complexity_index_raster(input, output_path=None, callback=None)
```

Computes raster patch shape complexity index.

Parameters:
- `input`: Input patch-ID raster.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### raster_area

```
raster_area(input, units="map units", zero_background=False, output_path=None, callback=None)
```

Estimates per-class area from a categorical raster and assigns each class's total area to all cells of that class.

Parameters:
- `input`: Input categorical raster.
- `units`: Area units (`"map units"` or `"grid cells"`).
- `zero_background`: If `True`, zero-valued cells are excluded.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

### raster_perimeter

```
raster_perimeter(input, units="map units", zero_background=False, output_path=None, callback=None)
```

Estimates per-class perimeter from a categorical raster using an anti-aliasing lookup-table method and assigns each class's total perimeter to all cells of that class.

Parameters:
- `input`: Input categorical raster.
- `units`: Perimeter units (`"map units"` or `"grid cells"`).
- `zero_background`: If `True`, zero-valued cells are excluded.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.

## GIS (Raster Binary And Patch Tools)

These tools are used to derive binary proximity rasters and connected-component patch identifiers from categorical inputs.

### Binary and Patch Tool Index

- `clump`

### clump

```
clump(input, diag=False, zero_background=False, output_path=None, callback=None)
```

Groups contiguous equal-valued cells into unique patch identifiers.

Parameters:
- `input`: Input categorical raster.
- `diag`: If `True`, include diagonal connectivity (8-neighbour); otherwise use 4-neighbour.
- `zero_background`: If `True`, keep zero-valued cells as background.
- `output_path`: Optional output path. If omitted, returns an in-memory raster.
- `callback`: Optional progress callback receiving JSON events.