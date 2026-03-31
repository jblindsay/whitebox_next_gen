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

**WbEnvironment usage**

```python
points_with_xy = env.add_point_coordinates_to_table(points, output_path="points_with_xy.geojson")
```

### `clean_vector`

Removes null and invalid geometries from a vector layer. For line and polygon data, undersized parts are removed and fully invalid features are dropped.

**WbEnvironment usage**

```python
cleaned = env.clean_vector(input_vector, output_path="cleaned.geojson")
```

### `convert_nodata_to_zero`

Replaces nodata cells in a raster with `0` while leaving all valid cells unchanged.

**WbEnvironment usage**

```python
result = env.convert_nodata_to_zero(input_raster, output_path="zeroed.tif")
```

### `csv_points_to_vector`

Imports point features from a CSV text table using selected X and Y field indices.

**WbEnvironment usage**

```python
points = env.csv_points_to_vector(
	input_file="samples.csv",
	x_field_num=2,
	y_field_num=3,
	epsg=4326,
	output_path="samples_points.geojson",
)
```

### `export_table_to_csv`

Exports a vector layer's attribute table to CSV format.

**WbEnvironment usage**

```python
csv_path = env.export_table_to_csv(parcels, output_csv_file="parcels_table.csv", headers=True)
```

### `fix_dangling_arcs`

Fixes undershot and overshot dangling arcs in a line network by snapping line endpoints that fall within a distance threshold.

**WbEnvironment usage**

```python
fixed_lines = env.fix_dangling_arcs(lines, snap_dist=5.0, output_path="lines_fixed.geojson")
```

### `new_raster_from_base_raster`

Creates a new raster with the same rows, columns, extent, cell size, and CRS as a base raster.

**WbEnvironment usage**

```python
blank = env.new_raster_from_base_raster(base_raster, out_val=0.0, data_type="float", output_path="blank.tif")
```

Parameters:

- `base`: Base raster.
- `out_val`: Optional fill value. Defaults to the base raster nodata value.
- `data_type`: One of `float`, `double`, or `integer`.
- `output_path`: Optional output path.

### `new_raster_from_base_vector`

Creates a new raster using the extent and CRS from a base vector layer and a required `cell_size`.

**WbEnvironment usage**

```python
blank = env.new_raster_from_base_vector(
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

### `polygons_to_lines`

Converts polygon or multipolygon features into boundary linework.

**WbEnvironment usage**

```python
lines = env.polygons_to_lines(polygons, output_path="boundaries.geojson")
```

### `lines_to_polygons`

Converts polyline features into polygon features. The first part of a multipart line becomes the exterior ring and later parts become holes.

**WbEnvironment usage**

```python
polygons = env.lines_to_polygons(lines, output_path="polygons.geojson")
```

### `join_tables`

Joins attributes from one vector table onto another by matching key fields.

**WbEnvironment usage**

```python
joined = env.join_tables(
	primary_vector=countries,
	primary_key_field="COUNTRY",
	foreign_vector=stats,
	foreign_key_field="COUNTRY",
	import_field="POPULATION",
	output_path="countries_joined.gpkg",
)
```

### `merge_table_with_csv`

Merges attributes from a CSV table into a vector attribute table using key fields.

**WbEnvironment usage**

```python
merged = env.merge_table_with_csv(
	primary_vector=countries,
	primary_key_field="COUNTRY",
	foreign_csv_filename="country_stats.csv",
	foreign_key_field="COUNTRY",
	import_field="GDP",
	output_path="countries_merged.gpkg",
)
```

### `modify_nodata_value`

Changes the raster nodata value and rewrites any existing nodata cells to the new value.

**WbEnvironment usage**

```python
updated = env.modify_nodata_value(input_raster, new_value=-9999.0, output_path="nodata_modified.tif")
```

### `print_geotiff_tags`

Builds a text report describing TIFF/GeoTIFF tags and key metadata. If the input is not a TIFF-family raster, the tool returns a warning message instead of a hard failure.

**WbEnvironment usage**

```python
report = env.print_geotiff_tags(input_raster)
```

### `raster_to_vector_points`

Converts each non-zero, non-nodata cell in a single-band raster into a point feature located at the cell centre, with `FID` and `VALUE` attributes.

**WbEnvironment usage**

```python
points = env.raster_to_vector_points(classified_raster, output_path="classified_points.geojson")
```

### `raster_to_vector_lines`

Converts non-zero, non-nodata cells in a single-band raster to polyline features. Output attributes include `FID` and the raster `VALUE` represented by each traced line.

**WbEnvironment usage**

```python
lines = env.raster_to_vector_lines(line_raster, output_path="line_features.geojson")
```

### `raster_to_vector_polygons`

Converts contiguous non-zero, non-nodata raster regions into polygon vector features. Output attributes include `FID` and source raster `VALUE` for each polygonized region.

**WbEnvironment usage**

```python
polys = env.raster_to_vector_polygons(classified_raster, output_path="regions.geojson")
```

### `reinitialize_attribute_table`

Creates a copy of a vector layer whose attribute table contains only a regenerated `FID` field.

**WbEnvironment usage**

```python
fid_only = env.reinitialize_attribute_table(input_vector, output_path="fid_only.geojson")
```

### `remove_polygon_holes`

Removes all interior rings from polygon and multipolygon features while preserving attributes.

**WbEnvironment usage**

```python
solid_polygons = env.remove_polygon_holes(polygons, output_path="polygons_no_holes.geojson")
```

### `remove_raster_polygon_holes`

Removes enclosed background holes from raster polygons, where background is defined as `0` or nodata. Holes connected to raster edges are preserved. Optionally limits removals to holes below a `threshold_size` and can use 8-neighbour connectedness with `use_diagonals=True`.

**WbEnvironment usage**

```python
filled = env.remove_raster_polygon_holes(
	input=classified,
	threshold_size=500,
	use_diagonals=True,
	output_path="classified_no_holes.tif",
)
```

### `set_nodata_value`

Sets a new nodata background value for a raster and maps existing nodata cells to that value. If a negative value is used with an unsigned input, the output raster type is promoted to a compatible signed integer type.

**WbEnvironment usage**

```python
updated = env.set_nodata_value(input_raster, back_value=-9999.0, output_path="nodata_set.tif")
```

### `merge_vectors`

Combines two or more input vectors of the same geometry type into a single output vector. The output attribute table contains `FID`, `PARENT` (source layer filename stem), `PARENT_FID`, and any attribute fields that are common to all input layers (same name and field type).

**WbEnvironment usage**

```python
merged = env.merge_vectors([roads_a, roads_b, roads_c], output_path="roads_merged.geojson")
```

### `multipart_to_singlepart`

Splits multi-part features (MultiPoint, MultiLineString, MultiPolygon) into individual single-part features. Each sub-geometry becomes a new output feature inheriting the source feature's attributes. For polygon inputs, setting `exclude_holes=True` keeps interior rings attached to their enclosing exterior ring rather than splitting them into independent features.

**WbEnvironment usage**

```python
# Split all parts (holes become independent polygons)
single = env.multipart_to_singlepart(parcels, output_path="parcels_single.geojson")

# Keep holes attached to their enclosing polygon
single = env.multipart_to_singlepart(parcels, exclude_holes=True, output_path="parcels_single.geojson")
```

### `singlepart_to_multipart`

Merges single-part features into multi-part features. When `field` is provided, features sharing the same value for that field are grouped into one multi-part geometry. When `field` is omitted, all features in the layer are merged into a single geometry. Output geometry type is promoted to the corresponding multi-part type (Point → MultiPoint; LineString → MultiLineString; Polygon → MultiPolygon).

**WbEnvironment usage**

```python
# Group parcels belonging to the same owner
multi = env.singlepart_to_multipart(parcels, field="OWNER_ID", output_path="parcels_multi.geojson")

# Merge all features into one geometry
multi = env.singlepart_to_multipart(parcels, output_path="all_merged.geojson")
```

### `vector_points_to_raster`

Rasterizes point or multipoint vectors to a grid using a selected assignment operation (`last`, `first`, `min`, `max`, `sum`, `num`, `mean`). Grid definition can come from a `base_raster` or from `cell_size` plus vector extent.

**WbEnvironment usage**

```python
# Use a base raster grid and compute per-cell mean
out = env.vector_points_to_raster(
	input=points,
	field_name="VALUE",
	assign_op="mean",
	base_raster=base,
	output_path="points_mean.tif",
)

# Build output grid from point extent and cell size
out2 = env.vector_points_to_raster(
	input=points,
	field_name="VALUE",
	assign_op="max",
	cell_size=5.0,
	zero_background=True,
	output_path="points_max.tif",
)
```

### `vector_lines_to_raster`

Rasterizes line or polygon-boundary geometries to a raster grid. Burn values come from an optional numeric `field_name` or from feature IDs when no field is supplied. Grid geometry can be inherited from a `base_raster` or created from input extent and `cell_size`.

**WbEnvironment usage**

```python
# Burn road class values onto an existing base raster grid
roads_r = env.vector_lines_to_raster(
	input=roads,
	field_name="CLASS_ID",
	base_raster=base,
	output_path="roads_burned.tif",
)

# Build output grid from vector extent
lines_r = env.vector_lines_to_raster(
	input=lines,
	cell_size=10.0,
	zero_background=True,
	output_path="lines_extent_grid.tif",
)
```

### `vector_polygons_to_raster`

Rasterizes polygon features using an optional numeric attribute field; otherwise burns feature IDs.

**WbEnvironment usage**

```python
poly_raster = env.vector_polygons_to_raster(
	input=landcover_polys,
	field_name="CLASS_ID",
	zero_background=True,
	base_raster=base,
	output_path="landcover.tif",
)
```