# Non-Tool Session And Facade API

This chapter documents non-tool APIs for `whiteboxworkflows` in R. Tool wrappers
are documented in [Tool API Reference (All Tools)](./api-tools-reference.md).

## Session Constructors

- `wbw_session(...)`
- `wbw_build_session(...)`

License-aware startup modes are supported by constructor arguments for:

- open runtime
- floating license
- signed entitlement JSON/file

## Tool Discovery And Execution (Generic)

- `wbw_tool_ids(session = ...)`
- `wbw_has_tool(tool_id, session = ...)`
- `wbw_search_tools(query, session = ...)`
- `wbw_describe_tool(tool_id, session = ...)`
- `wbw_run_tool(tool_id, args = list(), session = ...)`
- `wbw_run_tool_with_progress(tool_id, args = list(), session = ...)`

## Typed Data I/O

- `wbw_read_raster(path, session = ...)`
- `wbw_read_vector(path, session = ...)`
- `wbw_read_lidar(path, session = ...)`
- `wbw_read_bundle(path, session = ...)`

## Writers And Output Controls

- `wbw_write_raster(raster, output_path, ..., session = ...)`
- `wbw_write_rasters(rasters, output_paths, ..., session = ...)`
- `wbw_write_vector(vector, output_path, ..., session = ...)`
- `wbw_write_lidar(lidar, output_path, ..., session = ...)`

## Session-Level Utility Methods

`wbw_build_session(...)` provides session-bound helper methods, including:

- read/write helpers (`session$read_*`, `session$write_*`)
- memory management (`session$clear_memory`, object-store clear/remove/count)
- projection helpers (`session$projection_*`)
- topology helpers (`session$topology_*`)

## Projection Helpers

- `wbw_projection_to_ogc_wkt(...)`
- `wbw_projection_identify_epsg(...)`
- `wbw_projection_reproject_points(...)`
- `wbw_projection_reproject_point(...)`
- `wbw_projection_from_proj_string(...)`
- `wbw_projection_area_of_use(...)`

## Topology Helpers

- `wbw_topology_intersects_wkt(...)`
- `wbw_topology_contains_wkt(...)`
- `wbw_topology_within_wkt(...)`
- `wbw_topology_touches_wkt(...)`
- `wbw_topology_disjoint_wkt(...)`
- `wbw_topology_crosses_wkt(...)`
- `wbw_topology_overlaps_wkt(...)`
- `wbw_topology_covers_wkt(...)`
- `wbw_topology_covered_by_wkt(...)`
- `wbw_topology_relate_wkt(...)`
- `wbw_topology_distance_wkt(...)`
- `wbw_topology_vector_feature_relation(...)`
- `wbw_topology_is_valid_polygon_wkt(...)`
- `wbw_topology_make_valid_polygon_wkt(...)`
- `wbw_topology_buffer_wkt(...)`

## Generated Session Tool Methods

In addition to generic `wbw_run_tool(...)`, generated session methods provide
direct tool wrappers (e.g., `session$slope(...)`, `session$d8_flow_accum(...)`,
etc.). These wrappers are generated and updated from runtime catalog metadata.
