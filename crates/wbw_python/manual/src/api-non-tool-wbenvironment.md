# Non-Tool WbEnvironment API

This chapter documents the non-tool surface on `WbEnvironment` and related
utility namespaces. Tool wrappers are documented in
[Tool API Reference (All Tools)](./api-tools-reference.md).

## Runtime And Licensing Constructors

- `WbEnvironment()`
- `WbEnvironment.from_floating_license_id(...)`
- `WbEnvironment.from_signed_entitlement_file(...)`
- `WbEnvironment.from_signed_entitlement_json(...)`

## Runtime Introspection And Discovery

- `version()`
- `license_type()`
- `license_info()`
- `list_tools()`
- `list_tools_json()`
- `list_tool_catalog_json()`
- `list_tools_detailed(include_locked=False)`
- `describe_tool(tool_id, include_locked=False)`
- `get_tool_metadata_json(tool_id)`
- `get_tool_info_json(tool_id)`
- `search_tools(query, include_locked=False)`
- `categories()`
- `category(name)`
- `domain_namespaces()`
- `domain(name)`

## Data Readers

- `read_raster(file_name, file_mode='r')`
- `read_rasters(file_names, parallel=True, file_mode='r')`
- `read_vector(file_name, options=None, file_mode='r')`
- `read_vectors(file_names, parallel=True, options=None, file_mode='r')`
- `read_lidar(file_name, file_mode='r')`
- `read_lidars(file_names, parallel=True, file_mode='r')`

### Sensor Bundle Readers

- `read_bundle(bundle_root)`
- `read_landsat(bundle_root)`
- `read_sentinel2(safe_root)`
- `read_sentinel1(safe_root)`
- `read_planetscope(bundle_root)`
- `read_iceye(bundle_root)`
- `read_dimap(bundle_root)`
- `read_maxar_worldview(bundle_root)`
- `read_radarsat2(bundle_root)`
- `read_rcm(bundle_root)`

## Reprojection Helpers

- `reproject_raster(...)`
- `reproject_vector(...)`
- `reproject_lidar(...)`
- `reproject_rasters(...)`
- `reproject_vectors(...)`
- `reproject_lidars(...)`
- `georeference_raster_from_control_points(...)`

## Writers

- `write_raster(raster, output_path, options=None, remove_after_write=False)`
- `write_rasters(rasters, output_paths, options=None, remove_after_write=False)`
- `write_vector(vector, output_path, options=None)`
- `write_lidar(lidar, output_path, options=None)`
- `write_text(text, file_name)`

## Memory Management

- `remove_raster_from_memory(raster)`
- `clear_raster_memory()`
- `raster_memory_count()`
- `raster_memory_bytes()`
- `remove_vector_from_memory(vector)`
- `clear_vector_memory()`
- `vector_memory_count()`
- `remove_lidar_from_memory(lidar)`
- `clear_lidar_memory()`
- `lidar_memory_count()`
- `clear_memory()`

## Utility Namespaces (Non-Tool)

### Projection Utility Namespace

Access via `wbe.projection`:

- `to_ogc_wkt(epsg)`
- `identify_epsg(crs_text)`
- `reproject_points(points, src_epsg, dst_epsg)`
- `reproject_point(x, y, src_epsg, dst_epsg)`

### Topology Utility Namespace

Access via `wbe.topology`:

- `intersects_wkt(a_wkt, b_wkt)`
- `contains_wkt(a_wkt, b_wkt)`
- `within_wkt(a_wkt, b_wkt)`
- `touches_wkt(a_wkt, b_wkt)`
- `disjoint_wkt(a_wkt, b_wkt)`
- `crosses_wkt(a_wkt, b_wkt)`
- `overlaps_wkt(a_wkt, b_wkt)`
- `covers_wkt(a_wkt, b_wkt)`
- `covered_by_wkt(a_wkt, b_wkt)`
- `relate_wkt(a_wkt, b_wkt)`
- `distance_wkt(a_wkt, b_wkt)`
- `vector_feature_relation(a_vector, a_feature_index, b_vector, b_feature_index)`
- `is_valid_polygon_wkt(wkt)`
- `make_valid_polygon_wkt(wkt, epsilon=1e-9)`
- `buffer_wkt(wkt, distance)`

## Note On Tool Namespaces

Tool namespaces such as `wbe.raster`, `wbe.hydrology`, `wbe.vector`,
`wbe.remote_sensing`, and nested taxonomy-driven categories are documented in:

- [Tool API Reference (All Tools)](./api-tools-reference.md)
