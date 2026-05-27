# Session and Discovery

This chapter covers session lifecycle and tool discovery patterns.

Session and discovery APIs establish runtime certainty before execution. By
creating a session explicitly and checking tool visibility up front, you avoid
late failures in long pipelines and make script behavior easier to reason about
across different environments and deployment targets.

## Create Session

This is the standard session bootstrap for scripts. Keep it explicit so runtime
state is obvious to future readers.

```r
library(whiteboxworkflows)

s <- wbw_session()
print(s)
```

## Visibility Checks

Use hard visibility checks before long jobs so missing-tool failures happen
immediately.

```r
library(whiteboxworkflows)

s <- wbw_session()
ids <- wbw_tool_ids(session = s)
cat('Visible tools:', length(ids), '\n')

if (!wbw_has_tool('slope', session = s)) {
	stop('slope is not available in this runtime session')
}
```

## Search and Describe

Use this when you know the task objective but need to confirm exact tool IDs and
parameter contracts.

```r
library(whiteboxworkflows)

matches <- wbw_search_tools('lidar')
print(matches[1:min(5, length(matches))])

desc <- wbw_describe_tool('slope')
print(desc)
```

## Schema-Aware Tool Metadata JSON

For canonical parameter I/O typing, use `get_tool_info_json` (or
`get_tool_metadata_json`) rather than inferring type from parameter names or
descriptions. These JSON payloads include `io_role` and `data_kind` fields for
each parameter.

```r
library(whiteboxworkflows)
library(jsonlite)

info <- fromJSON(get_tool_info_json('slope'))
params <- info$params
has_role <- 'io_role' %in% names(params)
has_kind <- 'data_kind' %in% names(params)

for (i in seq_len(nrow(params))) {
	role <- if (has_role) params$io_role[[i]] else 'unknown'
	kind <- if (has_kind) params$data_kind[[i]] else 'unknown'
	cat(params$name[[i]], ' role=', role, ' kind=', kind, '\n', sep = '')
}
```

| Field | Meaning |
|---|---|
| `io_role` | Parameter role: `input`, `output`, or non-I/O `argument`. |
| `data_kind` | Canonical family such as `raster`, `vector`, `lidar`, `table`, `json`, `text`, `file`, `bool`, `number`, or `string`. |

Use these fields for integration logic such as destination/output typing,
layer handling, and parameter validation.

## Category-Based Discovery

Category discovery is useful for UI generation, guided workflows, and policy
checks in larger automation systems.

```r
library(whiteboxworkflows)

summary <- wbw_category_summary()
print(summary)

cats <- wbw_get_all_categories()
print(cats)

raster_tools <- wbw_tools_in_category('Raster')
print(head(raster_tools, 20))

proj_tools <- wbw_tools_in_category('projection_georeferencing')
print(head(proj_tools, 20))
```

## Session API Reference

WbW-R is function-first, but the `wbw_session()` object still exposes a useful
set of callable methods for execution, projection utilities, topology checks,
and typed I/O helpers.

### Session Construction and Execution

| Method | Description |
|---|---|
| `run_tool` | Execute a tool with a named argument list. |
| `run_tool_with_progress` | Execute a tool and return structured progress/result output. |
| `list_tools` | Return visible tool IDs for the session/license context. |
| `get_tool_metadata_json` | Return canonical metadata JSON for one tool ID, including `io_role`/`data_kind`. |
| `get_tool_info_json` | Alias of `get_tool_metadata_json` for cross-binding API parity. |

### Typed I/O Helpers

| Method | Description |
|---|---|
| `read_vector` | Read vector data with optional read options. |
| `write_raster` | Write one raster with optional format options. |
| `write_rasters` | Write multiple rasters in one call. |
| `write_vector` | Write one vector with optional format options. |

### Projection Utility Methods

| Method | Description |
|---|---|
| `projection_to_ogc_wkt` | Convert EPSG code to OGC WKT text. |
| `projection_identify_epsg` | Identify EPSG from CRS text where possible. |
| `projection_reproject_points` | Reproject point collections between EPSG systems. |
| `projection_reproject_point` | Reproject a single point between EPSG systems. |

### Topology Utility Methods

| Method | Description |
|---|---|
| `topology_intersects_wkt`, `topology_contains_wkt`, `topology_within_wkt` | Core spatial predicate checks on WKT geometries. |
| `topology_touches_wkt`, `topology_disjoint_wkt`, `topology_crosses_wkt`, `topology_overlaps_wkt` | Additional topological predicates for rule-based QA. |
| `topology_covers_wkt`, `topology_covered_by_wkt` | Boundary-aware containment checks. |
| `topology_relate_wkt` | Return DE-9IM relation text for exact topology logic. |
| `topology_distance_wkt` | Return shortest distance between WKT geometries. |
| `topology_vector_feature_relation` | Evaluate topology relation between indexed features in two vector objects. |
| `topology_is_valid_polygon_wkt`, `topology_make_valid_polygon_wkt` | Validate and repair polygon WKT geometries. |
| `topology_buffer_wkt` | Build buffered geometry from WKT and distance. |

## Recommended Discovery Pattern

1. Build session explicitly.
2. Check required tools with `wbw_has_tool(...)`.
3. Use `wbw_describe_tool(...)` for parameter verification.
4. Use `get_tool_info_json(...)` when you need canonical I/O typing for integration logic.
5. Use category functions to drive guided UX or script validation.
