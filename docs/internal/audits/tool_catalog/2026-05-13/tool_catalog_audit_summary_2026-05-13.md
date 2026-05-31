# Tool Catalog Deep Dive (2026-05-13)

## Scope
This audit compares:
- Runtime backend catalog from Python API: WbEnvironment.list_tools()
- Frontend taxonomy catalogs:
  - crates/wbw_python/tool_taxonomy.resolved.json
  - crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/tool_taxonomy.resolved.json
  - crates/wbw_qgis/plugin/whitebox_workflows_qgis/tool_taxonomy.resolved.json

## Headline Counts
- Runtime backend tools: 683
- Frontend tools (Python/R/QGIS each): 742
- Frontend tools missing in runtime: 60
- Runtime tools missing in all frontends: 1

## Interpretation
- 52 of the frontend-missing tools are Pro tier and not expected in this current session because license_info() reports no active local license state.
- 8 of the frontend-missing tools are Open tier and need attention.

## Open-Tier Frontend IDs Missing In Runtime (8)
- assign_projection_lidar
- assign_projection_raster
- assign_projection_vector
- false_colour_composite
- georeference_raster_from_control_points
- reproject_lidar
- reproject_raster
- true_colour_composite

## Classification Of The Open-Tier IDs
- Likely true backend gap:
  - georeference_raster_from_control_points
- Likely taxonomy leakage of utility/helper methods rather than runtime tools:
  - assign_projection_lidar
  - assign_projection_raster
  - assign_projection_vector
  - reproject_raster
  - reproject_lidar
  - true_colour_composite
  - false_colour_composite

Evidence in wb_environment wrappers/methods:
- reproject_raster and reproject_lidar delegate to object methods (not runtime tool IDs).
- true_colour_composite and false_colour_composite are bundle helper methods that call run_bundle_colour_composite.
- georeference_raster_from_control_points explicitly calls runtime tool ID georeference_raster_from_control_points.

## Runtime Tools Missing In All Frontends (1)
- buffer_vector

Notes:
- greater_than_or_equal_to and less_than_or_equal_to are now exposed in all three frontend catalogs.
- buffer_vector remains intentionally absent from the frontend catalogs.

## Files Produced By Audit
- tool_catalog_audit_2026-05-13.json
- tool_catalog_audit_tiered_2026-05-13.json
- wb_environment_mapping_audit_2026-05-13.json
