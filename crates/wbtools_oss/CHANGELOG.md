# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while in pre-1.0 development.

## [Unreleased]

## [0.1.3] - 2026-06-30

### Added
- Added explicit field parameter schemas across 40+ GIS tools for QGIS field dropdown widget support:
  - **Interpolation Tools**: `idw_interpolation`, `modified_shepard_interpolation`, `natural_neighbour_interpolation` — field_name parameter with parent reference to points layer.
  - **Spatial Statistics**: `morans_i`, `local_morans_i`, `bivariable_correlation` — field parameter with parent reference to input layer.
  - **Vector Analysis**: `buffer_vector`, `explode_features`, `near`, `select_by_location`, `spatial_join` — field parameters (dissolve_field, search_field, etc.) with parent references.
  - **Linear Referencing**: `route_calibrate`, `locate_along_route`, `locate_point_on_route` — route_id_field and measure_field parameters with parent references to route/event layers.
  - **Network Analysis**: `network_routes_from_od`, `network_accessibility_metrics` — node_cost_field parameter with parent reference to node_cost_points layer.
  - **Classification**: `training_sample_filter`, `knn_classification` — class_field parameter with parent reference to training_data layer.
  - **Field Operations**: `add_field`, `delete_field`, `rename_field` — field parameters with appropriate parent vector layer references.
- Field schemas enable QGIS front-end to render field parameters as dropdown selectors (instead of text input) with automatic parent layer resolution.

### Fixed
- Fixed `longest_flowpath` tool metadata schema incorrectly specifying output as raster instead of vector. The tool was grouped with flowpath-length tools (which produce rasters) in the schema registry, causing QGIS plugin to render it as a raster output parameter. Extracted into separate schema entry with correct specification: `basins` as input raster, `output` as `output_vector_any()`. QGIS will now correctly display output parameter as vector layer sink once published binary is updated.
- Fixed `polygons_to_lines` tool producing open polylines with missing closing segments. Ring internal representation intentionally omits the closing duplicate vertex for efficiency. The tool was cloning ring coordinates directly into output line strings, losing the closing segment. Added `close_ring()` step that appends the first coordinate to each ring if not already closed, with guard against double-closing rings from formats that include closing vertex on read. Applies to both `Polygon` and `MultiPolygon` inputs (resolves issue #19).

## [0.1.2] - 2026-06-14

### Added
- Added pilot explicit parameter schemas for `extract_streams` and `vector_stream_network_analysis` via `stream_tool_param_schemas(...)`.
- Added explicit `Tool` metadata/manifests for the two pilot stream tools so emitted metadata includes canonical parameter names/descriptions and defaults.

### Changed
- Re-exported stream tool schema mapping helper from `tools` module for binding/front-end metadata integration.

### Fixed
- Fixed `quantiles` tool catastrophic failure on rasters with extreme positive skewness. The fixed-bin histogram approach (10,000 bins over full [min, max] range) produced bin widths so coarse that quantile boundaries fell within a single bin, causing all valid pixels to be assigned the highest class. Replaced with an adaptive-bin histogram that scales bin count proportionally to valid cell count (capped at 32 MB), ensuring quantile boundaries map to distinct bins regardless of distribution shape. The tool now correctly computes equal-count quantile classes on highly skewed data while maintaining O(n) time complexity and cache-friendly memory usage.