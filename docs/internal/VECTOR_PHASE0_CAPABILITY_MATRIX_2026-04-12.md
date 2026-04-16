# Vector Phase 0 Capability Matrix

Date: 2026-04-12
Purpose: Deliverable for Phase 0 item 1 (tool -> category -> maturity -> wrapper parity).
Scope: Network analysis baseline, linear referencing baseline, and foundational topology QA.

## Maturity Legend

- Implemented: Production-usable baseline exists in backend + wrappers.
- Partial: Baseline exists but advanced enterprise depth is pending in Phases 1-4.
- Planned: Not implemented yet; covered by roadmap.

## Matrix

| Tool / Capability | Category | Maturity | Backend (wbtools_oss) | Python Runtime | Python Stubs | R Wrapper | Regression Evidence |
|---|---|---|---|---|---|---|---|
| shortest_path_network | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs shortest-path tests |
| k_shortest_paths_network | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs k-shortest tests |
| network_service_area | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs service-area tests |
| network_routes_from_od | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs OD-routes tests |
| network_od_cost_matrix | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs OD-matrix tests |
| network_connected_components | Vector / Network Diagnostics | Implemented | Yes | Yes | Yes | Yes | registry registration + tool integration coverage |
| network_node_degree | Vector / Network Diagnostics | Implemented | Yes | Yes | Yes | Yes | registry registration + tool integration coverage |
| travelling_salesman_problem | Vector / Network | Implemented | Yes | Yes | Yes | Yes | registry registration + existing integration coverage |
| locate_points_along_routes | Vector / Linear Referencing | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs locate-points tests |
| route_event_points_from_table | Vector / Linear Referencing | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs route-event table tests |
| route_event_lines_from_table | Vector / Linear Referencing | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs route-event table tests |
| route_event_points_from_layer | Vector / Linear Referencing | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs route-event layer tests |
| route_event_lines_from_layer | Vector / Linear Referencing | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs route-event layer tests |
| topology_validation_report | Conversion / Topology QA | Implemented | Yes | Yes | Yes | Yes | registry_integration.rs topology report tests |
| Time-dependent routing | Network | Planned (Phase 1) | No | No | No | No | VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md |
| Multimodal routing | Network | Planned (Phase 3) | No | No | No | No | VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md |
| VRP family (CVRP/VRPTW/P&D) | Network Optimization | Planned (Phase 3) | No | No | No | No | VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md |
| Rule-based topology engine | Topology QA | Planned (Phase 2) | No | No | No | No | VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md |
| Route calibration/recalibration | Linear Referencing | Planned (Phase 2) | No | No | No | No | VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md |

## Evidence Pointers (Current Repo)

Backend registration and inventory:
- crates/wbtools_oss/tests/registry_integration.rs

Python runtime methods:
- crates/wbw_python/src/wb_environment.rs

Python typed stubs:
- crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi

R generated wrappers:
- crates/wbw_r/r-package/whiteboxworkflows/R/zz_generated_wrappers.R

Roadmap source:
- docs/internal/VECTOR_GIS_GAP_PHASED_PLAN_2026-04-12.md

## Phase 0 Status Note

For the baseline vector-network and linear-referencing stack, wrapper parity is in place across Rust/Python/R for the implemented capabilities listed above. Remaining high-value gaps are advanced features intentionally deferred to Phases 1-4.
