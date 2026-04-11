# Topology Utilities

WbW-R currently emphasizes topology through tool workflows (category-driven) rather
than a dedicated R-native `topology` utility namespace equivalent to Python.

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

## Geometry Validation via Interop

For strict in-memory geometry predicates/repairs in R scripts, an `sf` interop path is often practical.

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

## Guidance

- Use `wbw_search_tools(...)` + `wbw_describe_tool(...)` to select backend topology tools.
- Use `sf` for script-level predicate checks and ad hoc geometry fixes when needed.
- Re-open outputs with `wbw_read_vector(...)` and verify schema/CRS after topology operations.