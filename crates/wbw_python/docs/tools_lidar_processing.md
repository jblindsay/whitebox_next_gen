# LiDAR Processing Tools

This document defines the LiDAR processing porting roadmap and CRS requirements for the backend migration. Tool-level API reference entries will be added here as each tool is ported.

For shared conventions (paths, callback payloads, optional output_path behavior), see [TOOLS.md](../TOOLS.md).

## Overview

LiDAR processing tools consume point-cloud datasets (LAS/LAZ/COPC/PLY/E57 via wblidar) and produce one or more of:
- LiDAR outputs (filtered/modified point clouds)
- Raster outputs (grids, interpolations, density, DSM/DTM-style derivatives)
- Vector outputs (contours, footprints, TIN-based products)
- Diagnostic products (metadata/statistics)

## Implementation Priorities

- Phase 1: LiDAR-to-raster interpolation and gridding tools.
- Phase 2: LiDAR-to-LiDAR processing tools.
- Phase 3: LiDAR-to-vector and specialized diagnostics/analysis tools.

Current migration status and canonical API guidance are tracked in [v2_migration_guide.md](v2_migration_guide.md) and [internal/wbw_py_wbw_r_parity_ledger.md](internal/wbw_py_wbw_r_parity_ledger.md).

## CRS Policy (Required)

These requirements are mandatory for all new LiDAR ports.

1. LiDAR-to-raster outputs must inherit CRS metadata from input LiDAR.
- If source has EPSG, set raster CRS EPSG.
- If source has WKT only, set raster CRS WKT.
- If both exist, preserve both.

2. Multi-input LiDAR tools must run in a single reference CRS.
- If all inputs already match, proceed directly.
- If inputs differ but are transformable, reproject to a common reference CRS before interpolation/aggregation.
- If CRS cannot be resolved, fail fast with a clear validation error.

3. LiDAR-to-vector outputs must carry LiDAR CRS.
- Output vector layer CRS must be assigned from source/reference LiDAR CRS.

4. Tests are required for CRS behavior.
- EPSG-only input
- WKT-only input
- Mismatched multi-input CRS
- Missing CRS metadata failure mode

## First Batch Targets

- lidar_nearest_neighbour_gridding
- lidar_idw_interpolation
- lidar_tin_gridding
- lidar_radial_basis_function_interpolation
- lidar_sibson_interpolation

Rationale:
- high user impact
- representative interpolation pathways
- establishes reusable CRS propagation/alignment patterns for all other LiDAR-to-raster tools

## Current Python API

The following convenience methods are available on `WbEnvironment`:

1. `lidar_nearest_neighbour_gridding(input=None, resolution=1.0, search_radius=2.5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
2. `lidar_idw_interpolation(input=None, resolution=1.0, weight=1.0, search_radius=2.5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, min_points=1, output_path=None, callback=None)`
3. `lidar_tin_gridding(input=None, resolution=1.0, max_triangle_edge_length=inf, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
4. `lidar_radial_basis_function_interpolation(input=None, resolution=1.0, num_points=15, search_radius=0.0, func_type="thinplatespline", poly_order="none", weight=0.1, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
5. `lidar_sibson_interpolation(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
6. `lidar_block_maximum(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
7. `lidar_block_minimum(input=None, resolution=1.0, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
8. `lidar_point_density(input=None, resolution=1.0, search_radius=2.5, returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
9. `lidar_digital_surface_model(input=None, resolution=1.0, search_radius=0.5, min_elev=None, max_elev=None, max_triangle_edge_length=inf, output_path=None, callback=None)`
10. `lidar_hillshade(input=None, resolution=1.0, search_radius=-1.0, azimuth=315.0, altitude=30.0, returns_included="all", excluded_classes=None, min_elev=None, max_elev=None, output_path=None, callback=None)`
11. `filter_lidar_classes(input=None, excluded_classes=None, output_path=None, callback=None)`
12. `lidar_shift(input=None, x_shift=0.0, y_shift=0.0, z_shift=0.0, output_path=None, callback=None)`
13. `remove_duplicates(input=None, include_z=False, output_path=None, callback=None)`
14. `filter_lidar_scan_angles(input=None, threshold=0, output_path=None, callback=None)`
15. `filter_lidar_noise(input=None, output_path=None, callback=None)`
16. `lidar_thin(input=None, resolution=1.0, method="first", save_filtered=False, output_path=None, filtered_output_path=None, callback=None)`
17. `lidar_elevation_slice(input=None, minz=-inf, maxz=inf, classify=False, in_class_value=2, out_class_value=1, output_path=None, callback=None)`
18. `lidar_join(inputs, output_path=None, callback=None)`
19. `lidar_thin_high_density(input=None, density=1.0, resolution=1.0, save_filtered=False, output_path=None, filtered_output_path=None, callback=None)`
20. `lidar_tile(input, tile_width=1000.0, tile_height=1000.0, origin_x=0.0, origin_y=0.0, min_points_in_tile=2, output_laz_format=True, output_directory=None, callback=None)`
21. `sort_lidar(sort_criteria, input=None, output_path=None, callback=None)`
22. `filter_lidar_by_percentile(input=None, percentile=0.0, block_size=1.0, output_path=None, callback=None)`
23. `split_lidar(split_criterion, input=None, interval=5.0, min_pts=5, output_directory=None, callback=None)`
24. `lidar_remove_outliers(input=None, search_radius=2.0, elev_diff=50.0, use_median=False, classify=False, output_path=None, callback=None)`
25. `normalize_lidar(input, dtm, no_negatives=False, output_path=None, callback=None)`
26. `height_above_ground(input, output_path=None, callback=None)`
27. `lidar_ground_point_filter(input=None, search_radius=2.0, min_neighbours=0, slope_threshold=45.0, height_threshold=1.0, classify=False, height_above_ground=False, output_path=None, callback=None)`
28. `filter_lidar(statement, input=None, output_path=None, callback=None)`
29. `modify_lidar(statement, input=None, output_path=None, callback=None)`
30. `filter_lidar_by_reference_surface(input, ref_surface, query="within", threshold=0.0, classify=False, true_class_value=2, false_class_value=1, preserve_classes=False, output_path=None, callback=None)`
31. `classify_lidar(input=None, search_radius=2.5, grd_threshold=0.1, oto_threshold=1.0, linearity_threshold=0.5, planarity_threshold=0.85, num_iter=30, facade_threshold=0.5, output_path=None, callback=None)`
32. `classify_buildings_in_lidar(in_lidar, building_footprints, output_path=None, callback=None)`
33. `ascii_to_las(input_ascii_files, pattern, epsg_code=4326, output_directory=None, callback=None)`
34. `las_to_ascii(input=None, output_path=None, callback=None)`
35. `select_tiles_by_polygon(input_directory, output_directory, polygons, callback=None)`
36. `lidar_info(input, output_path=None, show_point_density=True, show_vlrs=True, show_geokeys=True, callback=None)`
37. `lidar_histogram(input, output_path=None, parameter="elevation", clip_percent=1.0, callback=None)`
38. `lidar_point_stats(input=None, resolution=1.0, num_points=False, num_pulses=False, avg_points_per_pulse=False, z_range=False, intensity_range=False, predominant_class=False, output_directory=None, callback=None)`
39. `lidar_contour(input=None, contour_interval=10.0, base_contour=0.0, smooth=5, interpolation_parameter="elevation", returns_included="all", excluded_classes=None, min_elev=-inf, max_elev=inf, max_triangle_edge_length=inf, output_path=None, callback=None)`
40. `lidar_tile_footprint(input=None, output_hulls=False, output_path=None, callback=None)`
41. `las_to_shapefile(input=None, output_multipoint=False, output_path=None, callback=None)`
42. `lidar_construct_vector_tin(input=None, returns_included="all", excluded_classes=None, min_elev=-inf, max_elev=inf, max_triangle_edge_length=inf, output_path=None, callback=None)`
43. `lidar_hex_bin(input, width, orientation="h", output_path=None, callback=None)`
44. `lidar_point_return_analysis(input, create_output=False, output_path=None, report_path=None, callback=None)`
45. `flightline_overlap(input=None, resolution=1.0, output_path=None, callback=None)`
46. `recover_flightline_info(input, max_time_diff=5.0, pt_src_id=False, user_data=False, rgb=False, output_path=None, callback=None)`
47. `find_flightline_edge_points(input, output_path=None, callback=None)`
48. `lidar_tophat_transform(input, search_radius, output_path=None, callback=None)`
49. `normal_vectors(input, search_radius=-1.0, output_path=None, callback=None)`
50. `lidar_kappa(classification_lidar, reference_lidar, report_path, resolution=1.0, output_class_accuracy=False, output_path=None, callback=None)`
51. `lidar_eigenvalue_features(input=None, num_neighbours=None, search_radius=None, output_path=None, callback=None)`
52. `lidar_ransac_planes(input, search_radius=2.0, num_iterations=50, num_samples=10, inlier_threshold=0.15, acceptable_model_size=30, max_planar_slope=75.0, classify=False, only_last_returns=False, output_path=None, callback=None)`
53. `lidar_rooftop_analysis(lidar_inputs, building_footprints, search_radius=2.0, num_iterations=50, num_samples=10, inlier_threshold=0.15, acceptable_model_size=30, max_planar_slope=75.0, norm_diff_threshold=2.0, azimuth=180.0, altitude=30.0, output_path=None, callback=None)`
54. `lidar_qa_and_confidence(input, profile="balanced", block_size=1.0, max_building_size=150.0, slope_threshold=15.0, elev_threshold=0.15, high_confidence_threshold=0.8, output_prefix=None, output_path=None, callback=None)`
55. `lidar_terrain_product_suite(input, profile="balanced", block_size=1.0, max_building_size=150.0, slope_threshold=15.0, elev_threshold=0.15, z_factor=1.0, hillshade_azimuth=315.0, hillshade_altitude=45.0, high_confidence_threshold=0.8, output_prefix=None, output_path=None, callback=None)`
56. `utility_corridor_encroachment_intelligence(input, corridors, profile="balanced", resolution=2.0, risk_height_threshold=3.0, corridor_influence_distance=60.0, priority_zone_threshold=None, max_zone_features=5000, output_prefix=None, callback=None)`
57. `forestry_structure_and_biomass_intelligence(input, profile="balanced", resolution=2.0, stand_block_cells=12, biomass_cap=25.0, output_prefix=None, callback=None)`

Notes:
- `returns_included` supports `all`, `first`, and `last`.
- `interpolation_parameter` supports `elevation`, `intensity`, `class`, `return_number`, `number_of_returns`, `scan_angle`, `time`, `rgb`, and `user_data`.
- `excluded_classes` accepts integer classes (e.g., `[7, 18]` in Python).
- Backend tools now support legacy-style batch mode when `input` is omitted for all currently ported LiDAR raster tools.
- In batch mode, the backend scans the current working directory for supported LiDAR formats (LAS, LAZ, COPC, PLY, E57), processes tiles in parallel, and writes per-tile GeoTIFF outputs with tool-specific suffixes.
- For interpolation tools, batch mode also includes points from neighboring batch tiles when processing each target tile, which helps reduce interpolation edge effects at tile boundaries.
- Python convenience methods now accept `input=None` to trigger backend batch mode.
- In batch mode, the returned Python `Raster` is a placeholder pointing to the first written output tile; all batch outputs are written directly to disk and not retained as in-memory raster objects.
- For LiDAR-to-LiDAR tools in batch mode, the returned Python `Lidar` is similarly a placeholder path to the first written output tile, while all batch outputs are written directly to disk.

## Per-Tool Parameter Notes

### lidar_nearest_neighbour_gridding
- Use when preserving local point values is preferred over smoothing.
- `search_radius` controls nodata behavior away from points.
- Backend batch-mode suffix: `_nn.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_nearest_neighbour_gridding(
    input="value",
    resolution=1.0,
    search_radius=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_idw_interpolation
- `weight` controls distance decay. Higher values emphasize closer points.
- `search_radius <= 0` triggers k-nearest fallback; `min_points` controls k.
- Backend batch-mode suffix: `_idw.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_idw_interpolation(
    input="value",
    resolution=1.0,
    weight=1.0,
    search_radius=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    min_points=1,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `weight` | float | no | Numeric parameter for `weight`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `min_points` | int | no | Numeric parameter for `min_points`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_tin_gridding
- Uses Delaunay triangles with barycentric interpolation.
- `max_triangle_edge_length <= 0` means no edge-length limit.
- Backend batch-mode suffix: `_tin.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_tin_gridding(
    input="value",
    resolution=1.0,
    max_triangle_edge_length=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    triangulation_backend,
    triangulation_auto_threshold=1,
    triangulation_epsilon=1.0,
    triangulation_thin_cell_size=1.0,
    triangulation_thin_method,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `max_triangle_edge_length` | float | no | Numeric parameter for `max_triangle_edge_length`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `triangulation_backend` | Literal["auto", "delaunator", "wbtopology"] | no | Optional parameter `triangulation_backend`. |
| `triangulation_auto_threshold` | int | no | Numeric parameter for `triangulation_auto_threshold`. |
| `triangulation_epsilon` | float | no | Numeric parameter for `triangulation_epsilon`. |
| `triangulation_thin_cell_size` | float | no | Numeric parameter for `triangulation_thin_cell_size`. |
| `triangulation_thin_method` | Literal["nearest_center", "min_value", "max_value"] | no | Optional parameter `triangulation_thin_method`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_radial_basis_function_interpolation
- `num_points` sets the local neighbourhood size.
- `func_type` options: `thinplatespline`, `polyharmonic`, `gaussian`, `multiquadric`, `inversemultiquadric`.
- `poly_order` controls local polynomial trend correction: `none`, `constant`, or `quadratic`.
- Backend batch-mode suffix: `_rbf.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_radial_basis_function_interpolation(
    input="value",
    resolution=1.0,
    num_points=1,
    search_radius=1.0,
    func_type,
    poly_order,
    weight=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `num_points` | int | no | Numeric parameter for `num_points`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `func_type` | Literal["thinplatespline", "polyharmonic", "gaussian", "multiquadric", "inversemultiquadric"] | no | Optional parameter `func_type`. |
| `poly_order` | Literal["none", "constant", "quadratic"] | no | Optional parameter `poly_order`. |
| `weight` | float | no | Numeric parameter for `weight`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_sibson_interpolation
- True Sibson natural-neighbour interpolation intended for smooth terrain surfaces.
- No explicit weight/radius tuning parameter is required for common use.
- Backend batch-mode suffix: `_sibson.tif`.
- In backend batch mode, each target tile can include neighboring tile points for edge-effect reduction.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_sibson_interpolation(
    input="value",
    resolution=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_block_maximum
- Computes a cell-wise maximum from filtered LiDAR points for rapid surface approximation.
- Backend batch-mode suffix: `_block_max.tif`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_block_maximum(
    input="value",
    resolution=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_block_minimum
- Computes a cell-wise minimum from filtered LiDAR points for rapid lower-envelope approximation.
- Backend batch-mode suffix: `_block_min.tif`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_block_minimum(
    input="value",
    resolution=1.0,
    interpolation_parameter=1,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `interpolation_parameter` | Literal["elevation", "intensity", "class", "return_number", "number_of_returns", "scan_angle", "time", "rgb", "user_data"] | no | Numeric parameter for `interpolation_parameter`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_point_density
- Counts nearby points around each cell center within `search_radius` and reports density per area.
- Backend batch-mode suffix: `_density.tif`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_point_density(
    input="value",
    resolution=1.0,
    search_radius=1.0,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_digital_surface_model
- Uses local top-surface candidate filtering and TIN gridding for a DSM-style output.
- Supports `max_triangle_edge_length` masking for long-edge facet suppression.
- Backend batch-mode suffix: `_dsm.tif`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_digital_surface_model(
    input="value",
    resolution=1.0,
    search_radius=1.0,
    min_elev=1.0,
    max_elev=1.0,
    max_triangle_edge_length=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `max_triangle_edge_length` | float | no | Numeric parameter for `max_triangle_edge_length`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_hillshade
- Derives a hillshade raster from LiDAR-derived local surface values.
- Supports illumination controls via `azimuth` and `altitude`.
- Supports optional `search_radius` compatibility parameter for legacy call-shape parity.
- Backend batch-mode suffix: `_hillshade.tif`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_hillshade(
    input="value",
    resolution=1.0,
    search_radius=1.0,
    azimuth=1.0,
    altitude=1.0,
    returns_included,
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `azimuth` | float | no | Numeric parameter for `azimuth`. |
| `altitude` | float | no | Numeric parameter for `altitude`. |
| `returns_included` | Literal["all", "first", "last"] | no | Optional parameter `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_contour
- Builds contour lines from LiDAR points using TIN-based contouring.
- Supports interval/base controls, interpolation parameter selection, returns/class filters, and optional triangle-edge-length masking.
- In batch mode (`input=None`), processes all LiDAR files in the working directory and writes sibling contour shapefiles.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_contour(
    input="value",
    contour_interval=1.0,
    base_contour=1.0,
    smooth=1,
    interpolation_parameter="value",
    returns_included="value",
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    max_triangle_edge_length=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `contour_interval` | float | no | Numeric parameter for `contour_interval`. |
| `base_contour` | float | no | Numeric parameter for `base_contour`. |
| `smooth` | int | no | Numeric parameter for `smooth`. |
| `interpolation_parameter` | string | no | String parameter for `interpolation_parameter`. |
| `returns_included` | string | no | String parameter for `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `max_triangle_edge_length` | float | no | Numeric parameter for `max_triangle_edge_length`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_tile_footprint
- Generates polygon footprints per tile with summary attributes (`LAS_NM`, `NUM_PNTS`, `Z_MIN`, `Z_MAX`).
- Writes bounding boxes by default; set `output_hulls=True` to write convex hull footprints.
- In batch mode (`input=None`), writes a single combined footprint layer for all LiDAR tiles in the working directory.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_tile_footprint(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output_hulls` | bool | no | Boolean option for `output_hulls`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### las_to_shapefile
- Converts LiDAR points to vector output.
- By default writes one point feature per LiDAR point with key attributes (`Z`, `INTENSITY`, `CLASS`, return fields).
- If `output_multipoint=True`, writes a single multipoint feature instead.
- In batch mode (`input=None`), writes one shapefile per LiDAR tile in the working directory.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.las_to_shapefile(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output_multipoint` | bool | no | Boolean option for `output_multipoint`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_construct_vector_tin
- Builds a triangular mesh vector layer directly from LiDAR points using Delaunay triangulation.
- Supports return filtering, class filtering, elevation limits, and optional max triangle-edge filtering.
- In batch mode (`input=None`), writes one `_tin.shp` output per LiDAR tile in the working directory.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_construct_vector_tin(
    input="value",
    returns_included="value",
    [excluded_classes_1, excluded_classes_2],
    min_elev=1.0,
    max_elev=1.0,
    max_triangle_edge_length=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `returns_included` | string | no | String parameter for `returns_included`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `min_elev` | float | no | Numeric parameter for `min_elev`. |
| `max_elev` | float | no | Numeric parameter for `max_elev`. |
| `max_triangle_edge_length` | float | no | Numeric parameter for `max_triangle_edge_length`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_hex_bin
- Bins LiDAR points to a hexagonal polygon grid and summarizes per-cell point count and min/max z/intensity.
- `width` sets the distance between opposite hex sides.
- `orientation` supports `h` (pointy-up) and `v` (flat-up).

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_hex_bin(
    input="value",
    width=1.0,
    orientation="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `width` | float | yes | Numeric parameter for `width`. |
| `orientation` | string | no | String parameter for `orientation`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_point_return_analysis
- Produces a return-sequence quality-control report (`report_path`) covering missing returns, duplicates, and `r > n` anomalies.
- If `create_output=True`, also writes a classified QC LiDAR output (`output`) with class assignments for problematic return groups.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_point_return_analysis(
    input="value",
    output_path="result.tif",
    report_path="report.dat",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `create_output` | bool | no | Boolean option for `create_output`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `report_path` | string | no | Path value for `report`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### flightline_overlap
- Builds a raster whose cell values equal the number of distinct `point_source_id` values present in each cell.
- In batch mode (`input=None`), scans the working directory for LiDAR tiles and writes one `_flightline_overlap.tif` raster per tile.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.flightline_overlap(
    input="value",
    resolution=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### recover_flightline_info
- Sorts points by GPS time and starts a new inferred flightline when the time gap exceeds `max_time_diff` seconds.
- Writes inferred flightline identifiers into any combination of `point_source_id`, `user_data`, and RGB.
- If no output field flags are enabled, RGB output is enabled automatically.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.recover_flightline_info(
    input="value",
    max_time_diff=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `max_time_diff` | float | no | Numeric parameter for `max_time_diff`. |
| `pt_src_id` | bool | no | Boolean option for `pt_src_id`. |
| `user_data` | bool | no | Boolean option for `user_data`. |
| `rgb` | bool | no | Boolean option for `rgb`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### find_flightline_edge_points
- Filters the input LiDAR to only points carrying the LAS `edge_of_flight_line` flag.
- Returns a LiDAR output containing just the edge points.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.find_flightline_edge_points(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_tophat_transform
- Applies a white top-hat transform to point elevations by subtracting a locally opened surface from each point z value.
- `search_radius` controls the XY neighbourhood used for the erosion and dilation steps.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.lidar_tophat_transform(
    input="value",
    search_radius=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `search_radius` | float | yes | Numeric parameter for `search_radius`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### normal_vectors
- Estimates local plane normals from neighbouring points and writes a compatibility color encoding based on normal direction.
- If `search_radius <= 0`, the backend estimates a nominal point spacing and derives a default neighbourhood size automatically.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.normal_vectors(
    input="value",
    search_radius=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_kappa
- Compares classifications in two LiDAR datasets using nearest-point matching and writes an HTML agreement report.
- Also writes a raster of per-cell classification agreement percentages at `resolution`.
- `output_class_accuracy` is accepted for legacy call-shape parity; the class-agreement raster is produced regardless.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_kappa(
    classification_lidar="value",
    reference_lidar="value",
    report_path="report.dat",
    resolution=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `classification_lidar` | Lidar | yes | Input LiDAR dataset for `classification_lidar`. |
| `reference_lidar` | Lidar | yes | Input LiDAR dataset for `reference_lidar`. |
| `report_path` | string | yes | Path value for `report`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `output_class_accuracy` | bool | no | Boolean option for `output_class_accuracy`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_eigenvalue_features
- Computes local PCA-based neighbourhood features and writes a binary `.eigen` output plus a JSON schema sidecar.
- Supports single-file mode (`input=...`) and working-directory batch mode (`input=None`).
- `num_neighbours` must be at least `7` when specified; `search_radius` can be used alone or together with neighbour-count limiting.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_eigenvalue_features(
    input="value",
    num_neighbours=1,
    search_radius=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `num_neighbours` | int | no | Numeric parameter for `num_neighbours`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_ransac_planes
- Uses local RANSAC plane fitting to identify planar points.
- If `classify=False`, non-planar points are filtered out; if `classify=True`, all points are retained and tagged as planar vs non-planar by class value.
- `only_last_returns=True` restricts model fitting to late returns.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_ransac_planes(
    input="value",
    search_radius=1.0,
    num_iterations=1,
    num_samples=1,
    inlier_threshold=1.0,
    acceptable_model_size=1,
    max_planar_slope=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `num_iterations` | int | no | Numeric parameter for `num_iterations`. |
| `num_samples` | int | no | Numeric parameter for `num_samples`. |
| `inlier_threshold` | float | no | Numeric parameter for `inlier_threshold`. |
| `acceptable_model_size` | int | no | Numeric parameter for `acceptable_model_size`. |
| `max_planar_slope` | float | no | Numeric parameter for `max_planar_slope`. |
| `classify` | bool | no | Boolean option for `classify`. |
| `only_last_returns` | bool | no | Boolean option for `only_last_returns`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_rooftop_analysis
- Identifies rooftop facets inside `building_footprints` and writes polygon output with rooftop attributes such as `MAX_ELEV`, `HILLSHADE`, `SLOPE`, `ASPECT`, and `AREA`.
- `lidar_inputs` accepts one or more LiDAR tiles covering the buildings of interest.
- Current parity implementation outputs convex-hull roof facets per detected planar segment.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_rooftop_analysis(
    [lidar_inputs_1, lidar_inputs_2],
    building_footprints,
    search_radius=1.0,
    num_iterations=1,
    num_samples=1,
    inlier_threshold=1.0,
    acceptable_model_size=1,
    max_planar_slope=1.0,
    norm_diff_threshold=1.0,
    azimuth=1.0,
    altitude=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `lidar_inputs` | Lidar | yes | Input LiDAR dataset for `lidar_inputs`. |
| `building_footprints` | Vector | yes | Input vector layer for `building_footprints`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `num_iterations` | int | no | Numeric parameter for `num_iterations`. |
| `num_samples` | int | no | Numeric parameter for `num_samples`. |
| `inlier_threshold` | float | no | Numeric parameter for `inlier_threshold`. |
| `acceptable_model_size` | int | no | Numeric parameter for `acceptable_model_size`. |
| `max_planar_slope` | float | no | Numeric parameter for `max_planar_slope`. |
| `norm_diff_threshold` | float | no | Numeric parameter for `norm_diff_threshold`. |
| `azimuth` | float | no | Numeric parameter for `azimuth`. |
| `altitude` | float | no | Numeric parameter for `altitude`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_qa_and_confidence
- Runs a QA workflow that classifies/filters ground points and produces DTM, confidence, uncertainty, QA flags, and summary outputs.
- Returns a tuple of `(classified_lidar, dtm, confidence, uncertainty, qa_flags, summary_path)`.
- `profile` supports `strict`, `balanced`, and `permissive`.

**Outputs**

Returned as `tuple[Lidar, Raster, Raster, Raster, Raster, str]` in this order:

- `classified_lidar`: `Lidar`
- `dtm`: `Raster`
- `confidence`: `Raster`
- `uncertainty`: `Raster`
- `qa_flags`: `Raster`
- `summary`: `str`

**WbEnvironment usage**

```python
classified_lidar, dtm, confidence, uncertainty, qa_flags, summary = wbe.lidar.workflow_products.lidar_qa_and_confidence(
    input,
    profile="value",
    block_size=1.0,
    max_building_size=1.0,
    slope_threshold=1.0,
    elev_threshold=1.0,
    high_confidence_threshold=1.0,
    output_prefix="output/result",
    output_path="output_path.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `profile` | string | yes | String parameter for `profile`. |
| `block_size` | float | yes | Numeric parameter for `block_size`. |
| `max_building_size` | float | yes | Numeric parameter for `max_building_size`. |
| `slope_threshold` | float | yes | Numeric parameter for `slope_threshold`. |
| `elev_threshold` | float | yes | Numeric parameter for `elev_threshold`. |
| `high_confidence_threshold` | float | yes | Numeric parameter for `high_confidence_threshold`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `output_path` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_terrain_product_suite
- Runs an end-to-end terrain product workflow and outputs DTM, DSM, slope, hillshade, confidence, uncertainty, metadata summary, and optional classified lidar.
- Returns `(dtm, dsm, slope, hillshade, confidence, uncertainty, metadata_path, classified_lidar_optional)`.
- `profile` supports `strict`, `balanced`, and `permissive`.

**Outputs**

Returned as `tuple[Raster, Vector, Vector, Raster, str]` in this order:

- `risk`: `Raster`
- `zones`: `Vector`
- `table`: `Vector`
- `confidence`: `Raster`
- `summary`: `str`

**WbEnvironment usage**

```python
risk, zones, table, confidence, summary = wbe.lidar.workflow_products.lidar_terrain_product_suite(
    input,
    profile="value",
    block_size=1.0,
    max_building_size=1.0,
    slope_threshold=1.0,
    elev_threshold=1.0,
    z_factor=1.0,
    hillshade_azimuth=1.0,
    hillshade_altitude=1.0,
    high_confidence_threshold=1.0,
    output_prefix="output/result",
    output_path="output_path.tif",
    ) -> PyResult<(Raster, Raster, Raster, Raster, Raster, Raster, String, Option<Lidar>)> {
        let mut args = serde_json,
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `profile` | string | yes | String parameter for `profile`. |
| `block_size` | float | yes | Numeric parameter for `block_size`. |
| `max_building_size` | float | yes | Numeric parameter for `max_building_size`. |
| `slope_threshold` | float | yes | Numeric parameter for `slope_threshold`. |
| `elev_threshold` | float | yes | Numeric parameter for `elev_threshold`. |
| `z_factor` | float | yes | Numeric parameter for `z_factor`. |
| `hillshade_azimuth` | float | yes | Numeric parameter for `hillshade_azimuth`. |
| `hillshade_altitude` | float | yes | Numeric parameter for `hillshade_altitude`. |
| `high_confidence_threshold` | float | yes | Numeric parameter for `high_confidence_threshold`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `output_path` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |
| `) -> PyResult<(Raster, Raster, Raster, Raster, Raster, Raster, String, Option<Lidar>)> {
        let mut args = serde_json` | function | no | Input raster for `) -> PyResult<(Raster, Raster, Raster, Raster, Raster, Raster, String, Option<Lidar>)> {
        let mut args = serde_json`. |

### utility_corridor_encroachment_intelligence
- Detects LiDAR-derived vegetation encroachment risk near utility corridor centerlines.
- Returns `(encroachment_risk_raster, corridor_priority_zones_vector, asset_risk_table_vector, classification_confidence_raster, summary_path)`.
- `profile` supports `fast`, `balanced`, and `strict`; `priority_zone_threshold` and `max_zone_features` control zone selection density.
- `risk_height_threshold` controls the canopy height where risk increases, and `corridor_influence_distance` controls proximity decay.

**Outputs**

Returned as `tuple[Raster, Vector, Vector, Raster, str]` in this order:

- `risk`: `Raster`
- `zones`: `Vector`
- `table`: `Vector`
- `confidence`: `Raster`
- `summary`: `str`

**WbEnvironment usage**

```python
risk, zones, table, confidence, summary = wbe.terrain.workflow_products.utility_corridor_encroachment_intelligence(
    input,
    corridors,
    profile="value",
    resolution=1.0,
    risk_height_threshold=1.0,
    corridor_influence_distance=1.0,
    priority_zone_threshold=1.0,
    max_zone_features=1,
    output_prefix="output/result",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `corridors` | Vector | yes | Input vector layer for `corridors`. |
| `profile` | string | yes | String parameter for `profile`. |
| `resolution` | float | yes | Numeric parameter for `resolution`. |
| `risk_height_threshold` | float | yes | Numeric parameter for `risk_height_threshold`. |
| `corridor_influence_distance` | float | yes | Numeric parameter for `corridor_influence_distance`. |
| `priority_zone_threshold` | float | no | Numeric parameter for `priority_zone_threshold`. |
| `max_zone_features` | int | yes | Numeric parameter for `max_zone_features`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### forestry_structure_and_biomass_intelligence
- Produces canopy metrics, vertical structure classes, stand units, biomass proxy, and confidence outputs from LiDAR.
- Returns `(canopy_height_metrics_raster, vertical_structure_class_raster, stand_structure_units_vector, biomass_proxy_raster, confidence_raster, summary_path)`.
- `profile` supports `fast`, `balanced`, and `strict`; `stand_block_cells` controls stand-level aggregation size.
- `biomass_cap` sets an upper bound for biomass proxy scaling.

**Outputs**

Returned as `tuple[Raster, Raster, Vector, Raster, Raster, str]` in this order:

- `canopy`: `Raster`
- `class`: `Raster`
- `stand`: `Vector`
- `biomass`: `Raster`
- `confidence`: `Raster`
- `summary`: `str`

**WbEnvironment usage**

```python
canopy, class, stand, biomass, confidence, summary = wbe.terrain.workflow_products.forestry_structure_and_biomass_intelligence(
    input,
    profile="value",
    resolution=1.0,
    stand_block_cells=1,
    biomass_cap=1.0,
    terrain_adaptation="value",
    output_prefix="output/result",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `profile` | string | yes | String parameter for `profile`. |
| `resolution` | float | yes | Numeric parameter for `resolution`. |
| `stand_block_cells` | int | yes | Numeric parameter for `stand_block_cells`. |
| `biomass_cap` | float | yes | Numeric parameter for `biomass_cap`. |
| `terrain_adaptation` | string | yes | String parameter for `terrain_adaptation`. |
| `output_prefix` | string | no | Optional output prefix for multi-product outputs. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar_classes
- Removes points whose classification is listed in `excluded_classes`.
- Backend batch-mode suffix: `_filtered_cls` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar_classes(
    input="value",
    [excluded_classes_1, excluded_classes_2],
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `excluded_classes` | list[int] \|None | no | List input for `excluded_classes`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_shift
- Applies additive `x_shift`, `y_shift`, and `z_shift` offsets to each point.
- Backend batch-mode suffix: `_shifted` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.lidar_shift(
    input="value",
    x_shift=1.0,
    y_shift=1.0,
    z_shift=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `x_shift` | float | no | Numeric parameter for `x_shift`. |
| `y_shift` | float | no | Numeric parameter for `y_shift`. |
| `z_shift` | float | no | Numeric parameter for `z_shift`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### remove_duplicates
- Removes duplicate points by x/y coordinates, optionally including z when `include_z=True`.
- Backend batch-mode suffix: `_dedup` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.remove_duplicates(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `include_z` | bool | no | Boolean option for `include_z`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar_scan_angles
- Removes points with absolute scan angle greater than `threshold`.
- `threshold` uses LAS scan-angle integer units (`1 unit = 0.006°`).
- Backend batch-mode suffix: `_scan_filtered` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar_scan_angles(
    input="value",
    threshold=1,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `threshold` | int | no | Numeric parameter for `threshold`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar_noise
- Removes points classified as low noise (`class=7`) or high noise (`class=18`).
- Backend batch-mode suffix: `_denoised` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar_noise(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_thin
- Keeps at most one point per grid cell at `resolution`.
- `method` supports `first`, `last`, `lowest`, `highest`, and `nearest`.
- Backend batch-mode suffix: `_thinned` with LiDAR output extension.
- If `save_filtered=True`, filtered-out points are also written and exposed as `filtered_path` in backend outputs (wrapper return remains the kept-point `Lidar`).

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_thin(
    input="value",
    resolution=1.0,
    method="value",
    output_path="result.tif",
    filtered_output="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `method` | string | no | String parameter for `method`. |
| `save_filtered` | bool | no | Boolean option for `save_filtered`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `filtered_output` | string | no | String parameter for `filtered_output`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_elevation_slice
- In filter mode (`classify=False`), keeps only points with `minz <= z <= maxz`.
- In classify mode (`classify=True`), keeps all points and reassigns class values using `in_class_value` and `out_class_value`.
- Backend batch-mode suffix: `_elev_slice` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.lidar_elevation_slice(
    input="value",
    minz=1.0,
    maxz=1.0,
    in_class_value=1,
    out_class_value=1,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `minz` | float | no | Numeric parameter for `minz`. |
| `maxz` | float | no | Numeric parameter for `maxz`. |
| `classify` | bool | no | Boolean option for `classify`. |
| `in_class_value` | int | no | Numeric parameter for `in_class_value`. |
| `out_class_value` | int | no | Numeric parameter for `out_class_value`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_join
- Merges multiple input LiDAR files into a single output point cloud.
- Expects `inputs` to be a list of LiDAR objects/paths.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.lidar_join(
    [inputs_1, inputs_2],
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `inputs` | Lidar | yes | Input LiDAR dataset for `inputs`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_thin_high_density
- Performs density-aware thinning by x/y blocks and z bins.
- `density` sets target points-per-area threshold; areas above threshold are thinned.
- If `save_filtered=True`, filtered points are also written and returned via backend `filtered_path` metadata.
- Backend batch-mode suffix: `_thinned_hd` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.interpolation_gridding.lidar_thin_high_density(
    input="value",
    density=1.0,
    resolution=1.0,
    output_path="result.tif",
    filtered_output="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `density` | float | no | Numeric parameter for `density`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `save_filtered` | bool | no | Boolean option for `save_filtered`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `filtered_output` | string | no | String parameter for `filtered_output`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_tile
- Splits a single input LiDAR into row/column tile outputs using a regular grid.
- Writes multiple files to `output_directory` (or `<input_stem>/` by default) and returns a placeholder path to one written tile.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.lidar_tile(
    input="value",
    tile_width=1.0,
    tile_height=1.0,
    origin_x=1.0,
    origin_y=1.0,
    min_points_in_tile=1,
    output_directory="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `tile_width` | float | no | Numeric parameter for `tile_width`. |
| `tile_height` | float | no | Numeric parameter for `tile_height`. |
| `origin_x` | float | no | Numeric parameter for `origin_x`. |
| `origin_y` | float | no | Numeric parameter for `origin_y`. |
| `min_points_in_tile` | int | no | Numeric parameter for `min_points_in_tile`. |
| `output_laz_format` | bool | no | Boolean option for `output_laz_format`. |
| `output_directory` | string | no | String parameter for `output_directory`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### sort_lidar
- Sorts points by one or more criteria (e.g., `"x 100, y 100, z"`).
- Supports optional bin sizes per criterion for grouped sorting.
- Backend batch-mode suffix: `_sorted` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.sort_lidar(
    sort_criteria="value",
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `sort_criteria` | string | yes | String parameter for `sort_criteria`. |
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar_by_percentile
- Selects one representative point per grid block by elevation percentile.
- `percentile=0` selects local minima, `100` selects maxima, `50` approximates medians.
- Excludes withheld and noise-classified points from percentile candidate sets.
- Backend batch-mode suffix: `_percentile` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar_by_percentile(
    input="value",
    percentile=1.0,
    block_size=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `percentile` | float | no | Numeric parameter for `percentile`. |
| `block_size` | float | no | Numeric parameter for `block_size`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### split_lidar
- Splits a LiDAR file into multiple outputs using a grouping criterion (`num_pts`, `x`, `y`, `z`, `intensity`, `class`, `user_data`, `point_source_id`, `scan_angle`, `time`).
- `interval` controls bin width for numeric criteria and points-per-file when `split_criterion="num_pts"`.
- `min_pts` filters sparse split outputs.
- In both single and batch mode, tool returns a placeholder path to one written split file plus backend `output_count` metadata.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.split_lidar(
    split_criterion="value",
    input="value",
    interval=1.0,
    min_pts=1,
    output_directory="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `split_criterion` | string | yes | String parameter for `split_criterion`. |
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `interval` | float | no | Numeric parameter for `interval`. |
| `min_pts` | int | no | Numeric parameter for `min_pts`. |
| `output_directory` | string | no | String parameter for `output_directory`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_remove_outliers
- Computes local elevation residuals from neighborhood mean/median and either filters or reclassifies outliers.
- `classify=False`: removes outliers; `classify=True`: keeps all points and assigns low/high noise classes 7/18.
- Backend batch-mode suffix: `_outliers_removed` (filter mode) or `_outliers_classified` (classify mode).

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.lidar_remove_outliers(
    input="value",
    search_radius=1.0,
    elev_diff=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `elev_diff` | float | no | Numeric parameter for `elev_diff`. |
| `use_median` | bool | no | Boolean option for `use_median`. |
| `classify` | bool | no | Boolean option for `classify`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### normalize_lidar
- Uses an input DTM raster to convert LiDAR elevations to height above terrain (`z = z_lidar - z_dtm`).
- `dtm` may be provided as a path string or typed raster object.
- If `no_negatives=True`, negative normalized values are clamped to `0.0`.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.normalize_lidar(
    input="value",
    dtm,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `dtm` | Raster | yes | Input raster for `dtm`. |
| `no_negatives` | bool | no | Boolean option for `no_negatives`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### height_above_ground
- Converts each point elevation to height above the nearest ground-classified point (`class=2`).
- Ground-classified points are assigned `z=0.0` in output.
- Fails with a validation/runtime error if there are no ground-classified points.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.height_above_ground(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_ground_point_filter
- Applies a slope-and-height neighborhood test to identify off-terrain points.
- `classify=True` writes classification labels (`ground=2`, `off-terrain=1`); otherwise off-terrain points are removed.
- Supports optional `height_above_ground=True` output z normalization from local neighborhood minima.
- Backend batch-mode suffix: `_ground_filtered` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.remote_sensing.filters.lidar_ground_point_filter(
    input="value",
    search_radius=1.0,
    min_neighbours=1,
    slope_threshold=1.0,
    height_threshold=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `min_neighbours` | int | no | Numeric parameter for `min_neighbours`. |
| `slope_threshold` | float | no | Numeric parameter for `slope_threshold`. |
| `height_threshold` | float | no | Numeric parameter for `height_threshold`. |
| `classify` | bool | no | Boolean option for `classify`. |
| `height_above_ground` | bool | no | Boolean option for `height_above_ground`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar
- Filters points with a boolean `statement` evaluated against point attributes.
- Supports core variables: coordinates (`x,y,z`), returns (`ret,nret,is_late,...`), class flags, scan metrics, color/time, and file min/mid/max stats.
- Backend batch-mode suffix: `_filtered` with LiDAR output extension.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar(
    statement="value",
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `statement` | string | yes | String parameter for `statement`. |
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### modify_lidar
- Applies assignment expressions to mutate LiDAR point attributes (e.g., `z = z + 1`, `class = if(z > 10, 2, class)`, `rgb = (255,0,0)`).
- Supports semicolon-separated multi-expression statements evaluated per point.
- Supports no-input batch mode with `_modified` output suffix.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.modify_lidar(
    statement="value",
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `statement` | string | yes | String parameter for `statement`. |
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### filter_lidar_by_reference_surface
- Compares each point elevation against a raster reference surface using query modes: `within`, `<`, `<=`, `>`, `>=`.
- In filter mode (`classify=False`), outputs only points that satisfy the query.
- In classify mode, all points are retained and classes are written using `true_class_value` and `false_class_value` (or preserved when `preserve_classes=True`).

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.filter_lidar_by_reference_surface(
    input="value",
    ref_surface,
    query="value",
    threshold=1.0,
    true_class_value=1,
    false_class_value=1,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `ref_surface` | Raster | yes | Input raster for `ref_surface`. |
| `query` | string | no | String parameter for `query`. |
| `threshold` | float | no | Numeric parameter for `threshold`. |
| `classify` | bool | no | Boolean option for `classify`. |
| `true_class_value` | int | no | Numeric parameter for `true_class_value`. |
| `false_class_value` | int | no | Numeric parameter for `false_class_value`. |
| `preserve_classes` | bool | no | Boolean option for `preserve_classes`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### classify_lidar
- Performs basic neighborhood-based classification into ground (`2`), building (`6`), vegetation (`5`), and unclassified (`1`).
- Uses `search_radius`, `grd_threshold`, and `oto_threshold` as primary controls, with RANSAC-style local planarity/linearity estimation.
- Supports no-input batch mode; backend batch-mode suffix: `_classified` with LiDAR output extension.
- `linearity_threshold`, `planarity_threshold`, `num_iter`, and `facade_threshold` are actively used during segmentation and refinement stages.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.classify_lidar(
    input="value",
    search_radius=1.0,
    grd_threshold=1.0,
    oto_threshold=1.0,
    linearity_threshold=1.0,
    planarity_threshold=1.0,
    num_iter=1,
    facade_threshold=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `grd_threshold` | float | no | Numeric parameter for `grd_threshold`. |
| `oto_threshold` | float | no | Numeric parameter for `oto_threshold`. |
| `linearity_threshold` | float | no | Numeric parameter for `linearity_threshold`. |
| `planarity_threshold` | float | no | Numeric parameter for `planarity_threshold`. |
| `num_iter` | int | no | Numeric parameter for `num_iter`. |
| `facade_threshold` | float | no | Numeric parameter for `facade_threshold`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### classify_buildings_in_lidar
- Reclassifies points to building class (`6`) when they fall within polygon building footprints.
- Input polygons should represent building outlines in the same CRS as the LiDAR input.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.classify_buildings_in_lidar(
    in_lidar="value",
    building_footprints,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `in_lidar` | Lidar | yes | Input LiDAR dataset for `in_lidar`. |
| `building_footprints` | Vector | yes | Input vector layer for `building_footprints`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### ascii_to_las
- Converts one or more ASCII point files into LAS output files.
- `pattern` must include `x,y,z` and can include: `i,c,rn,nr,time,sa,r,g,b`.
- If `rn` is used, `nr` must also be used; RGB fields must be supplied as a full `r,g,b` set.
- `epsg_code` sets output LAS CRS metadata; outputs are written to `output_directory` when provided.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.ascii_to_las(
    [input_ascii_files_1, input_ascii_files_2],
    pattern="value",
    epsg_code=1,
    output_directory="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_ascii_files` | list[stringinginging] | yes | List input for `input_ascii_files`. |
| `pattern` | string | yes | String parameter for `pattern`. |
| `epsg_code` | int | no | Numeric parameter for `epsg_code`. |
| `output_directory` | string | no | String parameter for `output_directory`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### las_to_ascii
- Converts an input LiDAR file to CSV with columns based on available attributes (time/RGB columns are included when present).
- If `input=None`, runs in batch mode over LiDAR files in the current working directory and returns a placeholder path to one produced CSV.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.las_to_ascii(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### select_tiles_by_polygon
- Scans LiDAR tiles in `input_directory` and copies selected tiles to `output_directory` based on polygon overlap.
- Tile selection uses representative tile-bounding-box sample points (corners, center, and edge midpoints) against polygon geometry.
- Returns the output directory path and writes selected tile files directly to disk.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.select_tiles_by_polygon(
    input_directory="value",
    output_directory="value",
    polygons,
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_directory` | string | yes | String parameter for `input_directory`. |
| `output_directory` | string | yes | String parameter for `output_directory`. |
| `polygons` | Vector | yes | Input vector layer for `polygons`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_info
- Produces a LiDAR summary report with point counts, coordinate/intensity ranges, class counts, and return counts.
- Writes to `.txt` or `.html` report output and returns the report path.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_info(
    input="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `show_point_density` | bool | no | Boolean option for `show_point_density`. |
| `show_vlrs` | bool | no | Boolean option for `show_vlrs`. |
| `show_geokeys` | bool | no | Boolean option for `show_geokeys`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_histogram
- Builds a histogram report for one parameter (`elevation`, `intensity`, `scan angle`, `class`, or `time`).
- Supports lower/upper tail clipping using `clip_percent`.
- Writes an HTML report and returns its path.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_histogram(
    input="value",
    output_path="result.tif",
    parameter="value",
    clip_percent=1.0,
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `parameter` | string | no | String parameter for `parameter`. |
| `clip_percent` | float | no | Numeric parameter for `clip_percent`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_point_stats
- Generates one or more summary rasters (point count, pulse count, avg points/pulse, z range, intensity range, predominant class).
- If no output flags are set, all point-stat rasters are generated.
- Returns the output directory path containing generated GeoTIFF rasters.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.lidar_point_stats(
    input="value",
    resolution=1.0,
    output_directory="value",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `num_points` | bool | no | Boolean option for `num_points`. |
| `num_pulses` | bool | no | Boolean option for `num_pulses`. |
| `avg_points_per_pulse` | bool | no | Boolean option for `avg_points_per_pulse`. |
| `z_range` | bool | no | Boolean option for `z_range`. |
| `intensity_range` | bool | no | Boolean option for `intensity_range`. |
| `predominant_class` | bool | no | Boolean option for `predominant_class`. |
| `output_directory` | string | no | String parameter for `output_directory`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_classify_subset
- Reclassifies points in a base cloud when they spatially match points in a subset cloud.
- Uses 3D nearest-neighbour matching with configurable `tolerance` (map units).
- Assigns `subset_class_value` to matches and `nonsubset_class_value` to non-matches (`255` preserves original classes).

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.lidar_classify_subset(
    base_lidar="value",
    subset_lidar="value",
    subset_class_value=1,
    nonsubset_class_value=1,
    tolerance=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base_lidar` | Lidar | yes | Input LiDAR dataset for `base_lidar`. |
| `subset_lidar` | Lidar | yes | Input LiDAR dataset for `subset_lidar`. |
| `subset_class_value` | int | no | Numeric parameter for `subset_class_value`. |
| `nonsubset_class_value` | int | no | Numeric parameter for `nonsubset_class_value`. |
| `tolerance` | float | no | Numeric parameter for `tolerance`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### clip_lidar_to_polygon
- Retains only points that lie within input polygon geometry (`polygons` vector path/object).
- Supports polygon holes; points inside holes are excluded from output.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.clip_lidar_to_polygon(
    input="value",
    polygons,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `polygons` | Vector | yes | Input vector layer for `polygons`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### erase_polygon_from_lidar
- Removes points that lie within input polygon geometry (`polygons` vector path/object).
- Complement of `clip_lidar_to_polygon`; points outside polygons are retained.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.erase_polygon_from_lidar(
    input="value",
    polygons,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `polygons` | Vector | yes | Input vector layer for `polygons`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### classify_overlap_points
- Identifies overlap points in grid cells containing multiple point-source IDs and either classifies (`class=12`) or filters them.
- Supported criteria: `max scan angle`, `not min point source id`, `not min time`, `multiple point source IDs`.
- `filter=True` removes overlap points; otherwise flagged points are reclassified.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.classify_overlap_points(
    input="value",
    resolution=1.0,
    overlap_criterion="value",
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `resolution` | float | no | Numeric parameter for `resolution`. |
| `overlap_criterion` | string | no | String parameter for `overlap_criterion`. |
| `filter` | bool | no | Boolean option for `filter`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_segmentation
- Segments points into connected components using XY neighbourhood (`search_radius`) and vertical continuity (`max_z_diff`).
- Writes per-segment colours into point RGB values; largest segment receives dark green `(25,120,0)`.
- Compatibility parameters for legacy call-shape are accepted (`num_iterations`, `num_samples`, `inlier_threshold`, `acceptable_model_size`, `max_planar_slope`, `norm_diff_threshold`).
- Optional `ground=True` assigns class `2` to the largest segment and class `1` to other segmented points.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.lidar_segmentation(
    input="value",
    search_radius=1.0,
    num_iterations=1,
    num_samples=1,
    inlier_threshold=1.0,
    acceptable_model_size=1,
    max_planar_slope=1.0,
    norm_diff_threshold=1.0,
    max_z_diff=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `num_iterations` | int | no | Numeric parameter for `num_iterations`. |
| `num_samples` | int | no | Numeric parameter for `num_samples`. |
| `inlier_threshold` | float | no | Numeric parameter for `inlier_threshold`. |
| `acceptable_model_size` | int | no | Numeric parameter for `acceptable_model_size`. |
| `max_planar_slope` | float | no | Numeric parameter for `max_planar_slope`. |
| `norm_diff_threshold` | float | no | Numeric parameter for `norm_diff_threshold`. |
| `max_z_diff` | float | no | Numeric parameter for `max_z_diff`. |
| `classes` | bool | no | Boolean option for `classes`. |
| `ground` | bool | no | Boolean option for `ground`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### individual_tree_segmentation
- Segments individual tree points using a mean-shift mode-seeking workflow over LiDAR points.
- Algorithm inspiration and credit: this implementation is inspired by the self-adaptive mean-shift tree-segmentation approach described in the 2020 MDPI Remote Sensing article on individual-tree segmentation, and by the grid-accelerated approximation ideas in the MeanShift++ paper (arXiv, 2021).
- Defaults to vegetation-only segmentation (`only_use_veg=True`, `veg_classes="3,4,5"`) and height filtering (`min_height=2.0`).
- Uses adaptive per-seed horizontal bandwidth by default (`adaptive_bandwidth=True`) based on local neighbour density and angular sector crown-radius cues.
- Adaptive controls: `adaptive_neighbors` (default `24`) and `adaptive_sector_count` (default `8`), with fallback to bounded height-based scaling when adaptive mode is disabled.
- Supports optional MeanShift++-style grid approximation (`grid_acceleration=True`) that runs mode updates against aggregated grid cells for faster large-cloud processing.
- Grid approximation control: `grid_cell_size` (default `0.5` in XY map units).
- Optional post-grid exact refinement: `grid_refine_exact=True` with `grid_refine_iterations` (default `2`) to recover local precision after coarse grid updates.
- Optional tiled seed scheduling for very large inputs: `tile_size` (default `0.0`, disabled) and `tile_overlap` (default `0.0`, must be smaller than `tile_size`).
- Supports deterministic random segment colouring (`output_id_mode="rgb"`), optional id storage in `user_data` / `point_source_id`, and optional sidecar CSV output (`output_sidecar_csv=True`).
- Includes performance controls: `threads` and `simd`.

Benchmarking notes (developer reference):
- Benchmark target: `individual_tree_segmentation_bench` in [Rust/whitebox_next_gen/crates/wbtools_oss/benches/individual_tree_segmentation_bench.rs](Rust/whitebox_next_gen/crates/wbtools_oss/benches/individual_tree_segmentation_bench.rs)
- Run command:
	- `cargo bench -p wbtools_oss --bench individual_tree_segmentation_bench`
- Suggested result-record template:

| Date | Dataset | Points | Mode | Threads | SIMD | Wall time (ms) | Assigned veg points | Segment count |
|------|---------|--------|------|---------|------|----------------|---------------------|---------------|
| YYYY-MM-DD | name | n | exact / grid_accel | t | true/false | value | value | value |

Expanded local matrix (2026-03-24):

| Date | Dataset | Points | Mode | Threads | SIMD | Wall time (ms) | Assigned veg points | Segment count |
|------|---------|--------|------|---------|------|----------------|---------------------|---------------|
| 2026-03-24 | synthetic small_t32_p180 | 5952 | exact | auto | true | 28.28 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_accel | auto | true | 9.20 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine | auto | true | 14.29 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine_tiled | auto | true | 14.86 (median) | 5536 | 32 |
| 2026-03-24 | synthetic small_t32_p180 | 5952 | grid_refine_tiled_t1 | 1 | true | 61.15 (median) | 5536 | 32 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | exact | auto | true | 90.10 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_accel | auto | true | 28.40 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine | auto | true | 44.48 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled | auto | true | 48.20 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled_t1 | 1 | true | 191.91 (median) | 13632 | 64 |
| 2026-03-24 | synthetic medium_t64_p220 | 14464 | grid_refine_tiled_t4 | 4 | true | 72.74 (median) | 13632 | 64 |

Observed speed/quality summary:
- Quality parity on synthetic data: assigned-point and segment-count ratios are 1.0 for all measured variants versus exact.
- Fastest profile in this matrix: grid_accel (about 3.07x faster on small, 3.17x faster on medium versus exact).
- Grid refine improves precision pathway while retaining speedup (about 1.98x small, 2.03x medium versus exact).
- Tiling is useful for scalability controls, but on these moderate synthetic sizes it adds overhead unless required for memory/partitioning constraints.

**Parameter tuning guidance:** For recommended parameter profiles tailored to your use case (speed-critical, precision-optimized, balanced, or memory-constrained), see **Section 16 (Parameter tuning guide)** in [lidar_individual_tree_segmentation_design.md](lidar_individual_tree_segmentation_design.md#16-parameter-tuning-guide-by-use-case). This guide provides ready-to-use parameter sets plus hyperparameter tuning directions for common forest types.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.individual_tree_segmentation(
    input="value",
    veg_classes="value",
    4,
    5",
    min_height=1.0,
    max_height=1.0,
    bandwidth_min=1.0,
    bandwidth_max=1.0,
    adaptive_neighbors=1,
    adaptive_sector_count=1,
    grid_cell_size=1.0,
    grid_refine_iterations=1,
    tile_size=1.0,
    tile_overlap=1.0,
    vertical_bandwidth=1.0,
    max_iterations=1,
    convergence_tol=1.0,
    min_cluster_points=1,
    mode_merge_dist=1.0,
    threads=1,
    output_id_mode="value",
    seed=1,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `only_use_veg` | bool | no | Boolean option for `only_use_veg`. |
| `veg_classes` | string | no | String parameter for `veg_classes`. |
| `4` | Any | yes | Parameter `4`. |
| `5"` | Any | yes | Parameter `5"`. |
| `min_height` | float | no | Numeric parameter for `min_height`. |
| `max_height` | float | no | Numeric parameter for `max_height`. |
| `bandwidth_min` | float | no | Numeric parameter for `bandwidth_min`. |
| `bandwidth_max` | float | no | Numeric parameter for `bandwidth_max`. |
| `adaptive_bandwidth` | bool | no | Boolean option for `adaptive_bandwidth`. |
| `adaptive_neighbors` | int | no | Numeric parameter for `adaptive_neighbors`. |
| `adaptive_sector_count` | int | no | Numeric parameter for `adaptive_sector_count`. |
| `grid_acceleration` | bool | no | Boolean option for `grid_acceleration`. |
| `grid_cell_size` | float | no | Numeric parameter for `grid_cell_size`. |
| `grid_refine_exact` | bool | no | Boolean option for `grid_refine_exact`. |
| `grid_refine_iterations` | int | no | Numeric parameter for `grid_refine_iterations`. |
| `tile_size` | float | no | Numeric parameter for `tile_size`. |
| `tile_overlap` | float | no | Numeric parameter for `tile_overlap`. |
| `vertical_bandwidth` | float | no | Numeric parameter for `vertical_bandwidth`. |
| `max_iterations` | int | no | Numeric parameter for `max_iterations`. |
| `convergence_tol` | float | no | Numeric parameter for `convergence_tol`. |
| `min_cluster_points` | int | no | Numeric parameter for `min_cluster_points`. |
| `mode_merge_dist` | float | no | Numeric parameter for `mode_merge_dist`. |
| `threads` | int | no | Numeric parameter for `threads`. |
| `simd` | bool | no | Boolean option for `simd`. |
| `output_id_mode` | string | no | String parameter for `output_id_mode`. |
| `output_sidecar_csv` | bool | no | Boolean option for `output_sidecar_csv`. |
| `seed` | int | no | Numeric parameter for `seed`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### individual_tree_detection
- Identifies tree top points in a LiDAR cloud using local maxima detection.
- Detects points that are the highest within a local search neighbourhood in XY, with optional height range constraints.
- Search neighbourhood size can vary with height: `min_search_radius` applies at lower heights, `max_search_radius` at upper heights, with linear interpolation between.
- Requires vegetation-only filtering by default (`only_use_veg=True`, vegetation classes 3, 4, 5); can be disabled if input is unclassified.
- Outputs a vector shapefile of tree top points with attributes: `FID` (1-based point index) and `Z` (height value).
- Use case: rapid treetop detection for forest inventory, DBH estimation, or canopy analysis; complements individual_tree_segmentation for cases where only crown centroids are needed.
- Parameters:
  - `min_search_radius` (default 1.0): search neighbourhood radius at minimum height
  - `max_search_radius` (optional): neighbourhood size at upper heights; if not specified, uses min_search_radius (constant neighbourhood)
  - `min_height` (default 0.0): minimum height threshold (points below this are skipped)
  - `max_height` (optional): height value where max_search_radius applies (linear interpolation between min and max)
  - `only_use_veg` (default true): if true, process only vegetation classes 3, 4, 5; if false, use all non-withheld, non-noise points

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.individual_tree_detection(
    input="value",
    min_search_radius=1.0,
    min_height=1.0,
    max_search_radius=1.0,
    max_height=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `min_search_radius` | float | no | Numeric parameter for `min_search_radius`. |
| `min_height` | float | no | Numeric parameter for `min_height`. |
| `max_search_radius` | float | no | Numeric parameter for `max_search_radius`. |
| `max_height` | float | no | Numeric parameter for `max_height`. |
| `only_use_veg` | bool | no | Boolean option for `only_use_veg`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_segmentation_based_filter
- Performs neighbourhood-connected low-relief ground extraction using `search_radius` and `max_z_diff`.
- `classify_points=False` outputs only extracted ground-like points.
- `classify_points=True` retains all points and assigns class `2` (ground-like) or class `1` (other).

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.filtering_classification.lidar_segmentation_based_filter(
    input="value",
    search_radius=1.0,
    norm_diff_threshold=1.0,
    max_z_diff=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `norm_diff_threshold` | float | no | Numeric parameter for `norm_diff_threshold`. |
| `max_z_diff` | float | no | Numeric parameter for `max_z_diff`. |
| `classify_points` | bool | no | Boolean option for `classify_points`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### lidar_colourize
- Assigns point RGB values from an overlapping raster image sampled at each point XY location.
- `image` can be provided as a raster object/path; out-of-image or nodata samples are assigned black.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.io_management.lidar_colourize(
    input="value",
    image,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | yes | Input LiDAR dataset for `input`. |
| `image` | Raster | yes | Input raster for `image`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### colourize_based_on_class
- Applies class-based colours for LAS classes 0-18, with optional per-class overrides via `clr_str`.
- Supports optional intensity blending (`intensity_blending_amount` in percent).
- `use_unique_clrs_for_buildings=True` assigns unique colours to connected building clusters (`class=6`) using `search_radius`.
- Supports batch mode when `input=None`.

**Outputs**

- `return`: `Lidar`

**WbEnvironment usage**

```python
result = wbe.lidar.analysis_metrics.colourize_based_on_class(
    input="value",
    intensity_blending_amount=1.0,
    clr_str="value",
    search_radius=1.0,
    output_path="result.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `intensity_blending_amount` | float | no | Numeric parameter for `intensity_blending_amount`. |
| `clr_str` | string | no | String parameter for `clr_str`. |
| `use_unique_clrs_for_buildings` | bool | no | Boolean option for `use_unique_clrs_for_buildings`. |
| `search_radius` | float | no | Numeric parameter for `search_radius`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### colourize_based_on_point_returns
- Applies colours by return type: only, first, intermediate, and last returns.
- Supports optional intensity blending (`intensity_blending_amount` in percent).
- Supports batch mode when `input=None`.

## Callback Payload Examples

All LiDAR interpolation methods emit JSON strings through `callback` using message/progress events.

Example message event:

```json
{"type":"message","message":"reading input lidar"}
```

Example progress event:

```json
{"type":"progress","percent":0.63}
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Lidar | no | Input LiDAR dataset for `input`. |
| `intensity_blending_amount` | float | no | Numeric parameter for `intensity_blending_amount`. |
| `only_ret_colour` | string | no | String parameter for `only_ret_colour`. |
| `first_ret_colour` | string | no | String parameter for `first_ret_colour`. |
| `intermediate_ret_colour` | string | no | String parameter for `intermediate_ret_colour`. |
| `last_ret_colour` | string | no | String parameter for `last_ret_colour`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

`percent` is normalized in [0, 1] and reaches 1.0 on completion.

**Outputs**

- `return`: `Lidar`

Example:

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
lidar = wbe.read_lidar("tile.laz")

dem = wbe.lidar_idw_interpolation(
	lidar,
	resolution=1.0,
	weight=2.0,
	returns_included="last",
	excluded_classes=[7, 18],
)
```

## Notes

- This document is intentionally planning-oriented for now.
- As each tool is ported, add full parameter and examples sections in this file.

