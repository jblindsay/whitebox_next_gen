# Supported Utilities

WbW-Py exposes non-tool utility namespaces for projection and topology tasks.

## Utility Namespaces

- `wbe.projection`: CRS/projection helpers.
- `wbe.topology`: WKT-based topology and geometry relations.

These are utility APIs, distinct from tool-category namespaces like
`wbe.hydrology`, `wbe.terrain`, and `wbe.vector`.

## Projection Utility Methods

Common methods:
- `to_ogc_wkt(epsg)`
- `identify_epsg(crs_text)`
- `reproject_points(points, src_epsg, dst_epsg)`
- `reproject_point(x, y, src_epsg, dst_epsg)`

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

wkt_4326 = wbe.projection.to_ogc_wkt(4326)
print(wbe.projection.identify_epsg(wkt_4326))

pt = wbe.projection.reproject_point(-79.3832, 43.6532, src_epsg=4326, dst_epsg=32618)
print(pt)
```

## Topology Utility Methods

Common methods include:
- `intersects_wkt(...)`, `contains_wkt(...)`, `within_wkt(...)`
- `touches_wkt(...)`, `crosses_wkt(...)`, `overlaps_wkt(...)`
- `covers_wkt(...)`, `covered_by_wkt(...)`, `disjoint_wkt(...)`
- `relate_wkt(...)`, `distance_wkt(...)`
- `is_valid_polygon_wkt(...)`, `make_valid_polygon_wkt(...)`, `buffer_wkt(...)`

See [Topology Utilities](./topology-utilities.md) for deeper examples.

## When to Use Utilities vs Tools

- Use utility namespaces for lightweight in-memory checks and CRS helpers.
- Use tool categories for heavy geoprocessing and file/object transformations.