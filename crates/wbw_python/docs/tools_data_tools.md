# Data Tools

This document covers the first batch of data and format conversion tools ported into the new backend.

## Tool Index

- `add_point_coordinates_to_table`
- `clean_vector`
- `convert_nodata_to_zero`
- `csv_points_to_vector`
- `export_table_to_csv`
- `fix_dangling_arcs`
- `join_tables`
- `lines_to_polygons`
- `merge_table_with_csv`
- `merge_vectors`
- `modify_nodata_value`
- `multipart_to_singlepart`
- `new_raster_from_base_raster`
- `new_raster_from_base_vector`
- `polygons_to_lines`
- `print_geotiff_tags`
- `raster_to_vector_lines`
- `raster_to_vector_polygons`
- `raster_to_vector_points`
- `reinitialize_attribute_table`
- `remove_polygon_holes`
- `remove_raster_polygon_holes`
- `set_nodata_value`
- `singlepart_to_multipart`
- `vector_lines_to_raster`
- `vector_polygons_to_raster`
- `vector_points_to_raster`

### `add_point_coordinates_to_table`

Copies a point layer and appends `XCOORD` and `YCOORD` fields using each feature's point coordinates.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
points_with_xy = wbe.conversion.vector_table_io.add_point_coordinates_to_table(points, output_path="points_with_xy.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `clean_vector`

Removes null and invalid geometries from a vector layer. For line and polygon data, undersized parts are removed and fully invalid features are dropped.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
cleaned = wbe.conversion.vector_table_io.clean_vector(input_vector, output_path="cleaned.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `convert_nodata_to_zero`

Replaces nodata cells in a raster with `0` while leaving all valid cells unchanged.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
result = wbe.conversion.raster_vector_conversion.convert_nodata_to_zero(input_raster, output_path="zeroed.tif")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `csv_points_to_vector`

Imports point features from a CSV text table using selected X and Y field indices.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
points = wbe.conversion.vector_table_io.csv_points_to_vector(
	input_file="samples.csv",
	x_field_num=2,
	y_field_num=3,
	epsg=4326,
	output_path="samples_points.geojson",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input_file` | string | yes | String parameter for `input_file`. |
| `x_field_num` | int | no | Numeric parameter for `x_field_num`. |
| `y_field_num` | int | no | Numeric parameter for `y_field_num`. |
| `epsg` | int \|None | no | Numeric parameter for `epsg`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `export_table_to_csv`

Exports a vector layer's attribute table to CSV format.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
csv_path = wbe.conversion.vector_table_io.export_table_to_csv(parcels, output_csv_file="parcels_table.csv", headers=True)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output_csv_file` | string \|None | no | String parameter for `output_csv_file`. |
| `headers` | bool | no | Boolean option for `headers`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `fix_dangling_arcs`

Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints that fall within a distance threshold.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
fixed_lines = wbe.conversion.geometry_topology.fix_dangling_arcs(lines, snap_dist=5.0, output_path="lines_fixed.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `snap_dist` | float | yes | Numeric parameter for `snap_dist`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `new_raster_from_base_raster`

Creates a new raster with the same rows, columns, extent, cell size, and CRS as a base raster.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
blank = wbe.conversion.raster_vector_conversion.new_raster_from_base_raster(base_raster, out_val=0.0, data_type="float", output_path="blank.tif")
```

Parameters:

- `base`: Base raster.
- `out_val`: Optional fill value. Defaults to the base raster nodata value.
- `data_type`: One of `float`, `double`, or `integer`.
- `output_path`: Optional output path.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Raster | yes | Input raster for `base`. |
| `out_val` | float \|None | no | Numeric parameter for `out_val`. |
| `data_type` | string | no | String parameter for `data_type`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `new_raster_from_base_vector`

Creates a new raster using the extent and CRS from a base vector layer and a required `cell_size`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
blank = wbe.conversion.raster_vector_conversion.new_raster_from_base_vector(
	base=study_area,
	cell_size=10.0,
	out_val=0.0,
	data_type="float",
	output_path="blank_from_vector.tif",
)
```

Parameters:

- `base`: Base vector defining extent.
- `cell_size`: Output cell size (> 0).
- `out_val`: Optional fill value. Defaults to nodata (`-32768`).
- `data_type`: One of `float`, `double`, or `integer`.
- `output_path`: Optional output path.

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `base` | Vector | yes | Input vector layer for `base`. |
| `cell_size` | float | yes | Numeric parameter for `cell_size`. |
| `out_val` | float \|None | no | Numeric parameter for `out_val`. |
| `data_type` | Literal["integer", "float", "double"] | no | Numeric parameter for `data_type`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `polygons_to_lines`

Converts polygon or multipolygon features into boundary linework.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
lines = wbe.conversion.geometry_topology.polygons_to_lines(polygons, output_path="boundaries.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `lines_to_polygons`

Converts polyline features into polygon features. The first part of a multipart line becomes the exterior ring and later parts become holes.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
polygons = wbe.conversion.geometry_topology.lines_to_polygons(lines, output_path="polygons.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `join_tables`

Joins attributes from one vector table onto another by matching key fields.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
joined = wbe.conversion.vector_table_io.join_tables(
	primary_vector=countries,
	primary_key_field="COUNTRY",
	foreign_vector=stats,
	foreign_key_field="COUNTRY",
	import_field="POPULATION",
	output_path="countries_joined.gpkg",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `primary_vector` | Vector | yes | Input vector layer for `primary_vector`. |
| `primary_key_field` | string | yes | String parameter for `primary_key_field`. |
| `foreign_vector` | Vector | yes | Input vector layer for `foreign_vector`. |
| `foreign_key_field` | string | yes | String parameter for `foreign_key_field`. |
| `import_field` | string \|None | no | String parameter for `import_field`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `merge_table_with_csv`

Merges attributes from a CSV table into a vector attribute table using key fields.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
merged = wbe.conversion.vector_table_io.merge_table_with_csv(
	primary_vector=countries,
	primary_key_field="COUNTRY",
	foreign_csv_filename="country_stats.csv",
	foreign_key_field="COUNTRY",
	import_field="GDP",
	output_path="countries_merged.gpkg",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `primary_vector` | Vector | yes | Input vector layer for `primary_vector`. |
| `primary_key_field` | string | yes | String parameter for `primary_key_field`. |
| `foreign_csv_filename` | string | yes | String parameter for `foreign_csv_filename`. |
| `foreign_key_field` | string | yes | String parameter for `foreign_key_field`. |
| `import_field` | string \|None | no | String parameter for `import_field`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `modify_nodata_value`

Changes the raster nodata value and rewrites any existing nodata cells to the new value.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
updated = wbe.conversion.raster_vector_conversion.modify_nodata_value(input_raster, new_value=-9999.0, output_path="nodata_modified.tif")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `new_value` | float | no | Numeric parameter for `new_value`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `print_geotiff_tags`

Builds a text report describing TIFF/GeoTIFF tags and key metadata. If the input is not a TIFF-family raster, the tool returns a warning message instead of a hard failure.

**Outputs**

- `return`: `str`

**WbEnvironment usage**

```python
report = wbe.raster.print_geotiff_tags(input_raster)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `raster_to_vector_points`

Converts each non-zero, non-nodata cell in a single-band raster into a point feature located at the cell centre, with `FID` and `VALUE` attributes.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
points = wbe.conversion.raster_vector_conversion.raster_to_vector_points(classified_raster, output_path="classified_points.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `raster_to_vector_lines`

Converts non-zero, non-nodata cells in a single-band raster to polyline features. Output attributes include `FID` and the raster `VALUE` represented by each traced line.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
lines = wbe.conversion.raster_vector_conversion.raster_to_vector_lines(line_raster, output_path="line_features.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `raster_to_vector_polygons`

Converts contiguous non-zero, non-nodata raster regions into polygon vector features. Output attributes include `FID` and source raster `VALUE` for each polygonized region.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
polys = wbe.conversion.raster_vector_conversion.raster_to_vector_polygons(classified_raster, output_path="regions.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `reinitialize_attribute_table`

Creates a copy of a vector layer whose attribute table contains only a regenerated `FID` field.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
fid_only = wbe.conversion.vector_table_io.reinitialize_attribute_table(input_vector, output_path="fid_only.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `remove_polygon_holes`

Removes all interior rings from polygon and multipolygon features while preserving attributes.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
solid_polygons = wbe.conversion.geometry_topology.remove_polygon_holes(polygons, output_path="polygons_no_holes.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `remove_raster_polygon_holes`

Removes enclosed background holes from raster polygons, where background is defined as `0` or nodata. Holes connected to raster edges are preserved. Optionally limits removals to holes below a `threshold_size` and can use 8-neighbour connectedness with `use_diagonals=True`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
filled = wbe.conversion.raster_vector_conversion.remove_raster_polygon_holes(
	input=classified,
	threshold_size=500,
	use_diagonals=True,
	output_path="classified_no_holes.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `threshold_size` | int \|None | no | Numeric parameter for `threshold_size`. |
| `use_diagonals` | bool | no | Boolean option for `use_diagonals`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `set_nodata_value`

Sets a new nodata background value for a raster and maps existing nodata cells to that value. If a negative value is used with an unsigned input, the output raster type is promoted to a compatible signed integer type.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
updated = wbe.conversion.raster_vector_conversion.set_nodata_value(input_raster, back_value=-9999.0, output_path="nodata_set.tif")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Raster | yes | Input raster for `input`. |
| `back_value` | float | no | Numeric parameter for `back_value`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `merge_vectors`

Combines two or more input vectors of the same geometry type into a single output vector. The output attribute table contains `FID`, `PARENT` (source layer filename stem), `PARENT_FID`, and any attribute fields that are common to all input layers (same name and field type).

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
merged = wbe.conversion.vector_table_io.merge_vectors([roads_a, roads_b, roads_c], output_path="roads_merged.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `inputs` | Vector | yes | Input vector layer for `inputs`. |
| `output_path` | string | yes | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `multipart_to_singlepart`

Splits multi-part features (MultiPoint, MultiLineString, MultiPolygon) into individual single-part features. Each sub-geometry becomes a new output feature inheriting the source feature's attributes. For polygon inputs, setting `exclude_holes=True` keeps interior rings attached to their enclosing exterior ring rather than splitting them into independent features.

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
# Split all parts (holes become independent polygons)
single = wbe.conversion.geometry_topology.multipart_to_singlepart(parcels, output_path="parcels_single.geojson")

# Keep holes attached to their enclosing polygon
single = wbe.conversion.geometry_topology.multipart_to_singlepart(parcels, exclude_holes=True, output_path="parcels_single.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `exclude_holes` | bool | no | Boolean option for `exclude_holes`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `singlepart_to_multipart`

Merges single-part features into multi-part features. When `field` is provided, features sharing the same value for that field are grouped into one multi-part geometry. When `field` is omitted, all features in the layer are merged into a single geometry. Output geometry type is promoted to the corresponding multi-part type (Point → MultiPoint; LineString → MultiLineString; Polygon → MultiPolygon).

**Outputs**

- `return`: `Vector`

**WbEnvironment usage**

```python
# Group parcels belonging to the same owner
multi = wbe.conversion.geometry_topology.singlepart_to_multipart(parcels, field="OWNER_ID", output_path="parcels_multi.geojson")

# Merge all features into one geometry
multi = wbe.conversion.geometry_topology.singlepart_to_multipart(parcels, output_path="all_merged.geojson")
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `field` | string \|None | no | String parameter for `field`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `vector_points_to_raster`

Rasterizes point or multipoint vectors to a grid using a selected assignment operation (`last`, `first`, `min`, `max`, `sum`, `num`, `mean`). Grid definition can come from a `base_raster` or from `cell_size` plus vector extent.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
# Use a base raster grid and compute per-cell mean
out = wbe.conversion.raster_vector_conversion.vector_points_to_raster(
	input=points,
	field_name="VALUE",
	assign_op="mean",
	base_raster=base,
	output_path="points_mean.tif",
)

# Build output grid from point extent and cell size
out2 = wbe.conversion.raster_vector_conversion.vector_points_to_raster(
	input=points,
	field_name="VALUE",
	assign_op="max",
	cell_size=5.0,
	zero_background=True,
	output_path="points_max.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `field_name` | string \|None | no | String parameter for `field_name`. |
| `assign_op` | string | no | String parameter for `assign_op`. |
| `zero_background` | bool | no | Boolean option for `zero_background`. |
| `cell_size` | float | no | Numeric parameter for `cell_size`. |
| `base_raster` | Raster | no | Input raster for `base_raster`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `vector_lines_to_raster`

Rasterizes line or polygon-boundary geometries to a raster grid. Burn values come from an optional numeric `field_name` or from feature IDs when no field is supplied. Grid geometry can be inherited from a `base_raster` or created from input extent and `cell_size`.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
# Burn road class values onto an existing base raster grid
roads_r = wbe.conversion.raster_vector_conversion.vector_lines_to_raster(
	input=roads,
	field_name="CLASS_ID",
	base_raster=base,
	output_path="roads_burned.tif",
)

# Build output grid from vector extent
lines_r = wbe.conversion.raster_vector_conversion.vector_lines_to_raster(
	input=lines,
	cell_size=10.0,
	zero_background=True,
	output_path="lines_extent_grid.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `field_name` | string \|None | no | String parameter for `field_name`. |
| `zero_background` | bool | no | Boolean option for `zero_background`. |
| `cell_size` | float | no | Numeric parameter for `cell_size`. |
| `base_raster` | Raster | no | Input raster for `base_raster`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

### `vector_polygons_to_raster`

Rasterizes polygon features using an optional numeric attribute field; otherwise burns feature IDs.

**Outputs**

- `return`: `Raster`

**WbEnvironment usage**

```python
poly_raster = wbe.conversion.raster_vector_conversion.vector_polygons_to_raster(
	input=landcover_polys,
	field_name="CLASS_ID",
	zero_background=True,
	base_raster=base,
	output_path="landcover.tif",
)
```

**Parameters**

| Name | Type | Required | Description |
|---|---|---|---|
| `input` | Vector | yes | Input vector layer for `input`. |
| `field_name` | string \|None | no | String parameter for `field_name`. |
| `zero_background` | bool | no | Boolean option for `zero_background`. |
| `cell_size` | float | no | Numeric parameter for `cell_size`. |
| `base_raster` | Raster | no | Input raster for `base_raster`. |
| `output` | string | no | Optional output path. If omitted, the result is returned in memory when supported. |
| `callback` | function | no | Optional progress callback receiving JSON events. |

