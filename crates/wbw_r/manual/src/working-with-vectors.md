# Working with Vectors

This chapter covers schema inspection, feature iteration, attribute reads/writes,
and persistence workflows.

Reliable vector workflows depend on stable schema and attribute contracts as much
as geometry itself. The patterns in this chapter emphasize inspecting structure
early, applying deterministic edits, and validating outputs after persistence so
downstream analysis remains predictable.

## Read and Inspect

Begin with schema and metadata inspection so edits are grounded in the actual
field model.

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
schema <- v$schema()
print(schema)
print(v$metadata())
```

## Iterate Through Features

Use this for quality checks, custom filters, and record-level diagnostics.

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')
count <- v$metadata()$feature_count

for (i in seq_len(count)) {
  attrs <- v$attributes(i)
  # attrs is a named list
  print(attrs)
}
```

## Read and Update Attribute Table

This example combines common edit actions: single-value updates, grouped updates,
and field creation.

```r
library(whiteboxworkflows)

v <- wbw_read_vector('roads.gpkg')

name1 <- v$attribute(1, 'name')
print(name1)

v$update_attribute(1, 'name', 'Main Street')
v$update_attributes(2, list(speed = 50, class = 'collector'))
v$add_field('reviewed', field_type = 'integer', default_value = 0)
```

## Persist Vector Outputs

This pattern demonstrates tool-driven persistence and post-write verification.

```r
library(whiteboxworkflows)

s <- wbw_session()
roads <- wbw_read_vector('roads.gpkg')

wbw_run_tool(
  'buffer_vector',
  args = list(input = roads$file_path(), output = 'roads_buffer.gpkg', distance = 15.0),
  session = s
)

buffered <- wbw_read_vector('roads_buffer.gpkg')
print(buffered$metadata())
```

## Practical Notes

- Call `schema()` first to confirm field names and expected types.
- Use `update_attributes()` for grouped feature edits.
- Re-read output files to validate schema and values after writes.

## Vector Object Method Reference

### Metadata and Structure

| Method | Description |
|---|---|
| `metadata` | Return vector metadata (geometry type, feature count, CRS, fields). |
| `schema` | Return field names and types as a data frame. |
| `path` | Return backing vector path. |
| `to_terra`, `to_sf` | Convert to `terra`/`sf` objects for ecosystem workflows. |

### Attribute Access and Edits

| Method | Description |
|---|---|
| `attributes` | Return all attribute values for one feature index. |
| `attribute` | Return one field value for one feature index. |
| `update_attributes` | Update multiple fields for one feature index. |
| `update_attribute` | Update one field for one feature index. |
| `add_field` | Add a new field with declared type and default value. |

### Persistence

| Method | Description |
|---|---|
| `deep_copy` | Copy vector to a new path with optional write options. |
| `write` | Write vector to a new output path. |
