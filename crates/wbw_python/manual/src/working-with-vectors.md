# Working with Vectors

This chapter covers schema inspection, feature iteration, attribute reads/writes,
and persistence workflows.

Vector processing is often more about data contracts than geometry mechanics.
Schemas, field types, and attribute consistency determine whether downstream
analysis remains trustworthy. The patterns below emphasize validating structure
first, then applying deterministic edits, then persisting to stable interchange
formats for downstream tools.

## Read and Inspect

This step establishes the schema contract your downstream edits depend on.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

schema = roads.schema()
print(schema)
print('features:', roads.feature_count())
```

## Iterate Through Features

Use feature iteration for inspections, QA checks, or bespoke attribute rules.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
v = wbe.read_vector('roads.gpkg')

n = v.feature_count()
for i in range(n):
    attrs = v.attributes(i)
    # attrs is dict-like; process values
    print(i, attrs)
```

## Read and Update Attribute Table

This example demonstrates single-field updates, grouped updates, and schema
extension in one controlled sequence.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
v = wbe.read_vector('roads.gpkg')

# Read one field value
name0 = v.attribute(0, 'name')
print('name[0]=', name0)

# Update one field
v.update_attribute(0, 'name', 'Main Street')

# Update multiple fields
v.update_attributes(1, {'speed': 50, 'class': 'collector'})

# Add a new field
v.add_field('reviewed', field_type='bool', default_value=False)
```

## Persist Vector Outputs

This pattern shows both default extension behavior and explicit format control
for reproducibility.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')
buffered = wbe.vector.buffer_vector(roads, distance=15.0)

# Extensionless output defaults to GeoPackage
wbe.write_vector(buffered, 'roads_buffer')

# Explicit output format
wbe.write_vector(buffered, 'roads_buffer.parquet', options={
    'strict_format_options': True,
    'geoparquet': {'compression': 'zstd'},
})
```

## Practical Notes

- Use `schema()` first to validate field names and types.
- Prefer `update_attributes()` for grouped edits to a feature.
- Re-read and validate after major writes, especially when switching formats.

## Vector Object Method Reference

Common simple properties such as `file_path` and `file_name` are omitted here so
the tables stay focused on callable `Vector` methods.

### Schema and Attribute Access

| Method | Description |
|---|---|
| `schema` | Return the vector schema, including field structure and geometry information. |
| `feature_count` | Report how many features are present. |
| `attribute_fields`, `attribute_field_names` | Inspect available attribute fields by full definition or by field name list. |
| `attribute` | Read a single field value from one feature. |
| `attributes` | Read all attribute values for one feature as a grouped record. |
| `add_field` | Add a new attribute field to the dataset schema. |
| `update_attribute` | Update one field in one feature. |
| `update_attributes` | Update multiple fields in one feature at once. |

### File, Metadata, and Copying

| Method | Description |
|---|---|
| `metadata` | Return `VectorMetadata` describing file state, CRS, and feature count. |
| `absolute_path` | Resolve the vector to an absolute file path string. |
| `parent_directory` | Return the containing directory path. |
| `exists` | Check whether the backing dataset exists on disk. |
| `get_short_filename`, `get_file_extension` | Return convenience filename information. |
| `get_file_size_in_bytes`, `get_last_modified_unix_seconds` | Inspect filesystem metadata for reporting or audit logs. |
| `deep_copy` | Write a copied vector dataset to a derived or explicit output path. |

### CRS and Geometry-Safe Persistence

| Method | Description |
|---|---|
| `crs_wkt`, `crs_epsg` | Inspect CRS metadata as WKT text or EPSG code. |
| `set_crs_wkt`, `set_crs_epsg` | Assign CRS metadata without moving feature coordinates. |
| `clear_crs` | Remove CRS metadata so it can be assigned again explicitly. |
| `reproject` | Reproject the vector dataset with explicit failure, topology, and antimeridian policies. |
