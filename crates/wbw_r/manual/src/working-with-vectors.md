# Working with Vectors

This chapter covers schema inspection, feature iteration, attribute reads/writes,
and persistence workflows.

Reliable vector workflows depend on stable schema and attribute contracts as much
as geometry itself. The patterns in this chapter emphasize inspecting structure
early, applying deterministic edits, and validating outputs after persistence so
downstream analysis remains predictable.

## See Also: Online Sources

If your workflow starts by downloading vectors from web providers (starting with
OSM Overpass), use the dedicated chapter:

- [Online Data Downloads](./online-data-downloads.md)

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

## Memory-Backed Vectors for Pipeline Efficiency

For workflows that chain multiple vector operations, memory-backed vectors eliminate
disk I/O between steps. This is valuable for complex pipelines where intermediate
results are passed between spatial operations.

Load a vector into memory with `file_mode = "m"`:

```r
library(whiteboxworkflows)

# Read directly into memory
roads <- wbw_read_vector('roads.gpkg', file_mode = "m")
rivers <- wbw_read_vector('rivers.gpkg', file_mode = "m")

print(roads$path)  # prints: memory://vector/...
```

Memory-backed vectors are compatible with all downstream operations:

```r
library(whiteboxworkflows)

v <- wbw_read_vector('polygons.gpkg', file_mode = "m")

# Inspect schema and metadata
schema <- v$schema()
meta <- v$metadata()

# Pass to spatial tools
centroid_path <- wbw_centroid_vector(input = v$path, output = 'centroids')

# Export to disk when ready
result <- wbw_read_vector(centroid_path)
result$write('centroids_final.gpkg')
```

### Vector Memory Lifecycle

Memory-backed vectors persist until explicitly removed or cleared. For long-running
vector pipelines, manage memory explicitly:

```r
library(whiteboxworkflows)

# Check current memory
cat('Vectors in memory:', wbw_vector_memory_count(), '\n')

# Read vectors
v1 <- wbw_read_vector('large1.gpkg', file_mode = "m")
v2 <- wbw_read_vector('large2.gpkg', file_mode = "m")

cat('After reads:', wbw_vector_memory_count(), '\n')

# Remove when done
wbw_remove_vector_from_memory(v1)
cat('After remove:', wbw_vector_memory_count(), '\n')

# Or clear all
wbw_clear_vector_memory()
cat('After clear:', wbw_vector_memory_count(), '\n')
```

### Implicit Memory Output from Tools

All vector-output tools store their result in memory automatically when the
`output` argument is omitted (`NULL`). You do not need to pass `file_mode = "m"`
or choose a temporary path — simply leave `output` out and the returned
`wbw_vector` object is already memory-backed:

```r
library(whiteboxworkflows)

wbe <- wbw_make_session()
roads <- wbw_read_vector('roads.gpkg')

# No output path — result is stored in memory automatically
centroids <- wbe$centroid_vector(input = roads$path)
cat(centroids$path, '\n')  # prints: memory://vector/...

# Chain operations without any intermediate files
clipped <- wbe$clip(input = centroids$path, clip = 'boundary.gpkg')
cat(clipped$path, '\n')  # also memory://vector/...

# Persist the final result only
wbw_write_vector(clipped, 'result.gpkg')
```

This applies to all tool categories — GIS, hydrology, geomorphometry, and stream
network tools all follow the same rule. Providing an explicit `output` path
writes to disk as before.

Best practices:
- Use `file_mode = "m"` for intermediate spatial analysis results.
- Export memory-backed vectors to disk with `write()` when persisting final outputs.
- Call `remove_vector_from_memory()` after a vector is no longer needed.
- Use `clear_vector_memory()` between independent analysis phases.
- Use `wbw_clear_memory()` when resetting all in-process raster/vector/lidar stores together.

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

For complete write-option keys and allowed values, see
[Output Controls](output-controls.md#vector-write-option-reference).

```r
library(whiteboxworkflows)

s <- wbw_session()
roads <- wbw_read_vector('roads.gpkg')

wbw_centroid_vector(input = roads$file_path(), output = 'roads_centroids.gpkg')

centroids <- wbw_read_vector('roads_centroids.gpkg')
print(centroids$metadata())
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
