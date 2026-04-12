# Topology Utilities

This chapter covers topology helper methods available under `wbe.topology`.

Topology utilities are best used as fast, script-level geometry checks and
diagnostics. They answer questions like:

- Do these two geometries intersect, touch, or overlap?
- Is this polygon valid before a downstream operation?
- What is the exact DE-9IM relationship between two geometries?

These methods are lightweight and ideal for validation gates. For large-scale
overlay workflows and heavy production geometry processing, prefer dedicated
vector tools in `wbe.vector.*`.

## Topology Workflow Pattern

A robust pattern for topology-aware scripts is:

1. Validate CRS compatibility first.
2. Run quick topology predicates to detect obvious incompatibilities.
3. Repair invalid geometries if required.
4. Re-check relationships after repairs.
5. Continue into heavier vector analysis only after topology checks pass.

This keeps failures early and localized, which is usually easier to debug than
finding topology issues deep into a long processing chain.

## WKT Predicate Checks

Use predicate checks for fast boolean decisions in filters and QA gates.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

poly = 'POLYGON((0 0,10 0,10 10,0 10,0 0))'
pt = 'POINT(5 5)'

print('contains:', wbe.topology.contains_wkt(poly, pt))
print('intersects:', wbe.topology.intersects_wkt(poly, pt))
print('within:', wbe.topology.within_wkt(pt, poly))
print('touches:', wbe.topology.touches_wkt(poly, pt))
print('disjoint:', wbe.topology.disjoint_wkt(poly, pt))
```

## Relationship and Distance

Use `relate_wkt` when a simple predicate is not enough and you need the full
topological relationship signature.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

a = 'LINESTRING(0 0, 10 0)'
b = 'LINESTRING(0 1, 10 1)'

print('distance:', wbe.topology.distance_wkt(a, b))
print('relate:', wbe.topology.relate_wkt(a, b))
```

The DE-9IM string returned by `relate_wkt` is useful for rule-based quality
checks in advanced pipelines, especially when business logic depends on exact
boundary/interior behavior.

## Geometry Validation and Repair

Validation and repair are especially useful before overlays, dissolves, and
buffer-heavy workflows where invalid rings can cause downstream failures.

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

When repairing, prefer writing repaired geometries to new outputs and keeping
source data immutable for auditability.

## Feature-to-Feature Relation

For vector feature comparisons, use `vector_feature_relation(...)`.

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
roads = wbe.read_vector('roads.gpkg')

rel = wbe.topology.vector_feature_relation(roads, 0, roads, 1)
print(rel)
```

For batch validation across many feature pairs, call this in a controlled loop
and log results with feature IDs for traceable QA reports.

## Topology Namespace Method Reference

The table below summarizes the core `wbe.topology` methods.

| Method | Description |
|---|---|
| `intersects_wkt` | Return `True` when two geometries share any portion of space. |
| `contains_wkt` | Return `True` when geometry A strictly contains geometry B. |
| `within_wkt` | Return `True` when geometry A is strictly within geometry B. |
| `touches_wkt` | Return `True` when geometries meet at boundaries but interiors do not overlap. |
| `disjoint_wkt` | Return `True` when geometries have no spatial intersection. |
| `crosses_wkt` | Return `True` when geometries cross with dimensional reduction behavior. |
| `overlaps_wkt` | Return `True` when same-dimension geometries partially overlap. |
| `covers_wkt` | Return `True` when geometry A covers geometry B including boundary cases. |
| `covered_by_wkt` | Return `True` when geometry A is covered by geometry B including boundary cases. |
| `relate_wkt` | Return DE-9IM relationship text for exact topology-rule evaluation. |
| `distance_wkt` | Return shortest distance between two geometries. |
| `is_valid_polygon_wkt` | Check polygon validity before topology-sensitive workflows. |
| `make_valid_polygon_wkt` | Repair an invalid polygon WKT representation. |
| `buffer_wkt` | Build a buffered geometry from WKT and distance. |
| `vector_feature_relation` | Evaluate topology relation between indexed features in vector datasets. |

## Practical Guidance

- Use topology utilities for fast geometry checks in validation pipelines.
- For complex overlays and production transforms, prefer vector tool workflows.
- Keep CRS explicit before topology checks; mismatched CRS can produce plausible but wrong relations.
- Use `relate_wkt` when QA rules depend on exact boundary/interior semantics.