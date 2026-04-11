# Working with Lidar

This chapter documents lidar workflows in WbW-Py, including file-backed processing,
metadata checks, and output controls.

## Baseline Workflow

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

Current stable WbW-Py lidar objects are file-backed and tool-oriented. Direct
point-by-point iterators are not the primary API path.

Recommended point-level workflow:
1. Use WbW-Py for lidar processing and reprojection.
2. For explicit point iteration, use a point-cloud reader library on the output file.
3. Return to WbW-Py for downstream geoprocessing.

```python
import whitebox_workflows as wb
# Optional external bridge for explicit point iteration.
import laspy

wbe = wb.WbEnvironment()
las_obj = wbe.read_lidar('survey.las')
filtered = wbe.lidar.classify_overlap_points(las_obj)

out_path = 'survey_filtered.laz'
wbe.write_lidar(filtered, out_path)

las = laspy.read(out_path)
for x, y, z in zip(las.x[:1000], las.y[:1000], las.z[:1000]):
    # point-level logic
    pass
```

## Output Controls

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

## Best Practices

- Validate CRS before and after transformations.
- Prefer COPC output for large datasets and cloud streaming scenarios.
- Keep raw source lidar immutable; write derived products to new files.
