# Service-Area-Only Commit Boundary (2026-04-13)

This note defines a clean boundary for the service-area workflow improvements relative to broader network-analysis additions.

## Keep In Service-Area Commit

- crates/wbtools_oss/src/tools/gis/mod.rs
  - NetworkServiceAreaTool metadata and manifest params:
    - polygon_merge_origins
    - mode_field
    - default_mode_speed
    - mode_speed_overrides
    - allowed_modes
  - NetworkServiceAreaTool validation for mode params and defaults.
  - NetworkServiceAreaTool run-path parsing for new params.
  - Mode-aware service-area graph build usage via build_line_network_graph_mode_aware.
  - Polygon output merge-by-ring/per-origin branching and ORIGIN_CT field behavior.
- crates/wbtools_oss/tests/registry_integration.rs
  - network_service_area_mode_allowlist_filters_edges
  - network_service_area_mode_speed_overrides_change_reachability
  - network_service_area_polygons_can_merge_overlapping_origins
  - network_service_area_polygons_can_merge_origins_by_ring
- crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi
  - network_service_area signature updates in both category and network subcategory surfaces.
- crates/wbw_r/generated/wbw_tools_generated.R
  - generated wrapper signature refresh for network_service_area args.
- crates/wbw_r/r-package/whiteboxworkflows/R/zz_generated_wrappers.R
  - generated wrapper signature refresh for network_service_area args.
- docs/internal/VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md
  - Service-area polygon workflow examples section.
- crates/wbw_python/README.md
  - public user examples for per-origin vs merged polygons and mode-aware service-area.
- crates/wbw_r/r-package/whiteboxworkflows/README.md
  - public user examples for per-origin vs merged polygons and mode-aware service-area.

## Exclude From Service-Area Commit (Broader NA)

- crates/wbtools_oss/src/lib.rs
  - tool registration for closest_facility_network and location_allocation_network.
- crates/wbtools_oss/src/tools/mod.rs
  - exports for ClosestFacilityNetworkTool and LocationAllocationNetworkTool.
- crates/wbtools_oss/src/tools/gis/mod.rs
  - ClosestFacilityNetworkTool implementation.
  - LocationAllocationNetworkTool implementation and solver helpers.
  - generalized turn_restrictions_csv turn_cost support used by multiple tools.
  - turn-override ingestion into shortest_path_network, network_od_cost_matrix, network_routes_from_od, map_matching_v1.
- crates/wbtools_oss/tests/registry_integration.rs
  - closest facility tests.
  - location-allocation tests.
  - OD matrix turn_cost override test.
  - registry assertions for closest_facility_network and location_allocation_network.
- crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi
  - closest_facility_network and location_allocation_network signatures.
- crates/wbw_r/generated/wbw_tools_generated.R
  - generated wrappers for closest_facility_network and location_allocation_network.
- crates/wbw_r/r-package/whiteboxworkflows/R/zz_generated_wrappers.R
  - generated wrappers for closest_facility_network and location_allocation_network.
- crates/wbw_r/r-package/whiteboxworkflows/inst/libs/libwbw_r.dylib
  - regenerated native binary artifact.

## Practical Commit Plan

1. Service-area workflow commit:
   - isolate only the "Keep" sections above.
2. Broader network-analysis commit:
   - closest facility + location allocation + turn cost overrides + generated wrapper/native updates.
