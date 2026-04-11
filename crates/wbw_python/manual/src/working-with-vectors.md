# Working with Vectors

This chapter covers schema inspection, feature iteration, attribute reads/writes,
and persistence workflows.

## Read and Inspect

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

schema = roads.schema()
print(schema)
print('features:', roads.feature_count())
```

## Iterate Through Features

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
