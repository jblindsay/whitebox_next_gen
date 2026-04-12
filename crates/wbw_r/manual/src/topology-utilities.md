# Topology Utilities

WbW-R currently emphasizes topology through tool workflows (category-driven) rather
than a dedicated R-native `topology` utility namespace equivalent to Python.

At the same time, session-level topology helpers provide a fast path for
geometry QA checks and validation gates before heavier vector operations.

## Topology Workflow Pattern

A robust topology-first script pattern is:

1. Validate CRS compatibility first.
2. Run fast predicate checks to detect obvious incompatibilities.
3. Repair invalid polygon geometries if needed.
4. Re-run critical predicates after repair.
5. Continue into heavier vector processing only after checks pass.

This keeps failures early, localized, and easier to diagnose.

## Session-Level Topology Predicate Checks

Use these methods when you need script-level yes/no checks on WKT geometries.

```r
library(whiteboxworkflows)

s <- wbw_session()

poly <- 'POLYGON((0 0,10 0,10 10,0 10,0 0))'
pt <- 'POINT(5 5)'

print(s$topology_contains_wkt(poly, pt))
print(s$topology_intersects_wkt(poly, pt))
print(s$topology_within_wkt(pt, poly))
print(s$topology_touches_wkt(poly, pt))
print(s$topology_disjoint_wkt(poly, pt))
```

## Topology via Tool Workflows

Use session execution and topology/geometry tool IDs through category discovery.

```r
library(whiteboxworkflows)

s <- wbw_session()

# Discover likely topology-related tools
hits <- wbw_search_tools('topology')
print(hits)

# Inspect a specific candidate tool
if (length(hits) > 0) {
  first_hit <- hits[[1]]
  tool_id <- if (is.list(first_hit) && !is.null(first_hit$tool_id)) first_hit$tool_id else as.character(first_hit)
  print(wbw_describe_tool(tool_id))
}
```

Use this pattern when topology checks are part of larger, file-based vector
pipelines that should remain in Whitebox tooling end to end.

## Vector Topology Pattern

```r
library(whiteboxworkflows)

s <- wbw_session()

# Example pattern: run a topology-oriented vector tool once selected.
# Replace 'tool_id_here' and args with the concrete tool from discovery.
# wbw_run_tool(
#   'tool_id_here',
#   args = list(input = 'input.gpkg', output = 'output.gpkg'),
#   session = s
# )
```

Use this template after selecting a concrete tool ID through discovery.

## Geometry Validation and Repair

Use session helpers for pure WKT checks and repairs.

```r
library(whiteboxworkflows)

s <- wbw_session()

invalid <- 'POLYGON((0 0,4 4,4 0,0 4,0 0))'
print(s$topology_is_valid_polygon_wkt(invalid))

fixed <- s$topology_make_valid_polygon_wkt(invalid)
print(fixed)

buf <- s$topology_buffer_wkt('LINESTRING(0 0, 10 0)', 1.5)
print(buf)
```

For broader in-memory feature editing, an `sf` interop path is still practical:

```r
library(whiteboxworkflows)
library(sf)

v <- wbw_read_vector('polygons.gpkg')
g <- v$to_sf()

valid_flags <- st_is_valid(g)
print(table(valid_flags))

g_fixed <- st_make_valid(g)
st_write(g_fixed, 'polygons_valid.gpkg', delete_dsn = TRUE, quiet = TRUE)
```

## Relationship and Distance

Use relation/distance methods when binary predicates are not expressive enough.

```r
library(whiteboxworkflows)

s <- wbw_session()

a <- 'LINESTRING(0 0, 10 0)'
b <- 'LINESTRING(0 1, 10 1)'

print(s$topology_distance_wkt(a, b))
print(s$topology_relate_wkt(a, b))
```

## Topology Method Reference

| Session Method | Description |
|---|---|
| `topology_intersects_wkt` | Return `TRUE` when two geometries share any space. |
| `topology_contains_wkt` | Return `TRUE` when geometry A strictly contains geometry B. |
| `topology_within_wkt` | Return `TRUE` when geometry A is strictly within geometry B. |
| `topology_touches_wkt` | Return `TRUE` when geometries meet at boundaries only. |
| `topology_disjoint_wkt` | Return `TRUE` when geometries do not intersect. |
| `topology_crosses_wkt` | Return `TRUE` when geometries cross with dimensional reduction behavior. |
| `topology_overlaps_wkt` | Return `TRUE` when same-dimension geometries partially overlap. |
| `topology_covers_wkt` | Boundary-aware containment test (`A` covers `B`). |
| `topology_covered_by_wkt` | Boundary-aware containment test (`A` covered by `B`). |
| `topology_relate_wkt` | Return DE-9IM relationship text for exact topology-rule evaluation. |
| `topology_distance_wkt` | Return shortest distance between geometries. |
| `topology_vector_feature_relation` | Evaluate relation between indexed features in vector objects. |
| `topology_is_valid_polygon_wkt` | Check polygon validity before topology-sensitive workflows. |
| `topology_make_valid_polygon_wkt` | Repair an invalid polygon WKT representation. |
| `topology_buffer_wkt` | Create a buffered geometry from WKT and distance. |

## Guidance

- Use `wbw_search_tools(...)` + `wbw_describe_tool(...)` to select backend topology tools.
- Use session topology helpers for fast script-level predicates and repairs.
- Use `sf` for broader in-memory feature editing when needed.
- Re-open outputs with `wbw_read_vector(...)` and verify schema/CRS after topology operations.