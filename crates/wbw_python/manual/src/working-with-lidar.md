# Working with Lidar

This chapter documents lidar workflows in WbW-Py, including file-backed processing,
metadata checks, and output controls.

Lidar datasets are typically large enough that memory strategy becomes a primary
design concern. The core pattern in this chapter is to combine file-backed
objects, vectorized column operations, and chunked processing so scripts scale
from small validation tiles to production-scale point clouds without rewriting
your workflow model.

## Baseline Workflow

This baseline confirms read, inspect, transform, and write in a single lidar
path before introducing chunking complexity.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
las = wbe.read_lidar('survey.las')
meta = las.metadata()

print(meta.file_path)
print('epsg:', meta.crs_epsg)

normals = wbe.lidar.calculate_point_normals(las)
wbe.write_lidar(normals, 'survey_normals.copc.laz')
```

## Iterating Through Lidar Points

Stable WbW-Py lidar objects are file-backed and tool-oriented, with explicit
columnar point access through NumPy. Direct point-by-point Python iterators are
still not the primary API path.

Recommended point-level workflow:
1. Use `Lidar.to_numpy()` with selected point fields.
2. Perform vectorized filtering/classification edits in NumPy.
3. Write updates with `Lidar.from_numpy(...)`.

The example below reclassifies points using a simple mask to illustrate the
pattern without domain-specific classification logic.

```python
import whitebox_workflows as wb
import numpy as np

wbe = wb.WbEnvironment()
las = wbe.read_lidar('survey.las')

arr = las.to_numpy(cols=['x', 'y', 'z', 'classification'])
ground_mask = arr[:, 3] == 2
arr[ground_mask, 3] = 6

edited = wb.Lidar.from_numpy(
    arr,
    base=las,
    cols=['x', 'y', 'z', 'classification'],
    output_path='survey_reclassified.laz',
)

print('points:', edited.point_count)
```

## Output Controls

Use these options when you need to tune compression and structure for storage,
cloud access, or downstream compatibility.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
las = wbe.read_lidar('survey.las')

wbe.write_lidar(las, 'survey_out.laz', options={
    'laz': {
        'chunk_size': 25000,
        'compression_level': 7,
    },
})

wbe.write_lidar(las, 'survey_out.copc.laz', options={
    'copc': {
        'max_points_per_node': 75000,
        'max_depth': 8,
        'node_point_ordering': 'hilbert',
    },
})
```

## Chunked Numpy Streaming

For very large point clouds, use chunked column streaming to avoid holding a
full point matrix in memory.

Recommended chunked workflow:
1. Read chunks with `Lidar.to_numpy_chunks(...)`.
2. Apply vectorized edits per chunk.
3. Write edited chunks with `Lidar.from_numpy_chunks(...)`.

Use this pattern whenever full-matrix reads risk exhausting memory.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
lidar = wbe.read_lidar('survey.las')

cols = ['x', 'y', 'z', 'classification']
chunks = lidar.to_numpy_chunks(chunk_size=200_000, cols=cols)

for chunk in chunks:
    # Reclassify non-ground points above a simple elevation threshold.
    mask = chunk[:, 2] > 250.0
    chunk[mask, 3] = 6

edited = wb.Lidar.from_numpy_chunks(
    chunks,
    base=lidar,
    cols=cols,
    output_path='survey_chunked_reclassified.laz',
)

print('points:', edited.point_count)
```

For callback-driven processing, pass `callback=` to `to_numpy_chunks(...)` to
process each chunk as it is decoded. In callback mode, no list is returned.

Notes:
- LAS/LAZ chunk outputs use the shared core streaming rewrite path.
- Other output formats currently use the existing non-streaming write path.

## Best Practices

- Validate CRS before and after transformations.
- Prefer COPC output for large datasets and cloud streaming scenarios.
- Keep raw source lidar immutable; write derived products to new files.

## Lidar Object Method Reference

Common simple properties such as `file_path`, `file_name`, and `point_count`
are omitted here so the tables stay focused on callable `Lidar` methods and the
most important array-processing workflows.

### Array and Chunk Workflows

| Method | Description |
|---|---|
| `to_numpy` | Export selected point fields as a 2D NumPy array. |
| `to_numpy_chunks` | Stream selected point fields as chunked NumPy arrays for large clouds. |
| `from_numpy` | Build a new lidar file by applying a full 2D NumPy array back onto a base cloud. |
| `from_numpy_chunks` | Build a new lidar file from an iterable of chunked NumPy arrays. |

### File, Metadata, and Copying

| Method | Description |
|---|---|
| `metadata` | Return `LidarMetadata` for path, file state, and CRS information. |
| `absolute_path` | Resolve the cloud to an absolute file path string. |
| `parent_directory` | Return the containing directory path. |
| `exists` | Check whether the backing lidar file exists. |
| `get_short_filename`, `get_file_extension` | Return convenience filename information. |
| `get_file_size_in_bytes`, `get_last_modified_unix_seconds` | Inspect filesystem metadata for reporting or audit logs. |
| `deep_copy` | Write a copied lidar dataset to a new path. |
| `copy_to_path` | Copy the lidar dataset to an explicit destination. |
| `write_to_path` | Persist the lidar dataset with optional format-specific write options. |

### CRS and Coordinate Management

| Method | Description |
|---|---|
| `crs_wkt`, `crs_epsg` | Inspect CRS metadata as WKT text or EPSG code. |
| `set_crs_wkt`, `set_crs_epsg` | Assign CRS metadata without moving point coordinates. |
| `clear_crs` | Remove CRS metadata when it is wrong or unknown. |
| `reproject` | Reproject the point cloud with explicit 3D-transform and failure-policy controls. |
