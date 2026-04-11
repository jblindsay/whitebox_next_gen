# Topology Utilities

This chapter covers topology helper methods available under `wbe.topology`.

## WKT Predicate Checks

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

poly = 'POLYGON((0 0,10 0,10 10,0 10,0 0))'
pt = 'POINT(5 5)'

print('contains:', wbe.topology.contains_wkt(poly, pt))
print('intersects:', wbe.topology.intersects_wkt(poly, pt))
print('within:', wbe.topology.within_wkt(pt, poly))
```

## Relationship and Distance

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

a = 'LINESTRING(0 0, 10 0)'
b = 'LINESTRING(0 1, 10 1)'

print('distance:', wbe.topology.distance_wkt(a, b))
print('relate:', wbe.topology.relate_wkt(a, b))
```

## Geometry Validation and Repair

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

invalid = 'POLYGON((0 0,4 4,4 0,0 4,0 0))'
print('is_valid:', wbe.topology.is_valid_polygon_wkt(invalid))

fixed = wbe.topology.make_valid_polygon_wkt(invalid)
print('fixed:', fixed)

buf = wbe.topology.buffer_wkt('LINESTRING(0 0, 10 0)', 1.5)
print('buffer:', buf)
```

## Feature-to-Feature Relation

For vector feature comparisons, use `vector_feature_relation(...)`.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

rel = wbe.topology.vector_feature_relation(roads, 0, roads, 1)
print(rel)
```

## Practical Guidance

- Use topology utilities for fast geometry checks in validation pipelines.
- For complex overlays and production transforms, prefer vector tool workflows.