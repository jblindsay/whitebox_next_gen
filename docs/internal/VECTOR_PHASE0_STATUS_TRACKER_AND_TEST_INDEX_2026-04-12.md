# Vector Phase 0 Status Tracker And Test Index

Date: 2026-04-12
Scope: Phase 0 (baseline consolidation) tracking for vector network, linear referencing, and topology QA.

## Phase 0 Deliverable Status

1. Capability matrix document
- Status: Completed
- Artifact: VECTOR_PHASE0_CAPABILITY_MATRIX_2026-04-12.md

2. Network examples cookbook
- Status: Completed
- Artifact: VECTOR_PHASE0_NETWORK_COOKBOOK_2026-04-12.md

3. Regression tests for network options
- Status: Completed (baseline coverage exists; representative tests executed and passing on 2026-04-12)
- Focus options: one-way handling, blocked edges, turn restrictions CSV

4. Naming and UX consistency pass
- Status: Completed (canonical argument naming and output conventions documented)
- Artifact: VECTOR_PHASE0_NETWORK_COOKBOOK_2026-04-12.md

## Representative Validation Executed On 2026-04-12

All tests below passed:
- shortest_path_network_respects_one_way_field_direction
- shortest_path_network_respects_blocked_field_edges
- shortest_path_network_respects_turn_restrictions_csv
- network_service_area_polygons_output_respects_turn_restrictions_csv
- network_od_cost_matrix_respects_turn_restrictions_csv
- network_routes_from_od_respects_turn_restrictions_csv
- k_shortest_paths_network_respects_turn_restrictions_csv
- network_connected_components_labels_disconnected_subnetworks
- network_node_degree_identifies_junction_and_dead_ends
- route_event_points_from_table_creates_measured_point_events
- topology_validation_report_flags_invalid_line_and_polygon_features

## Full Test Index (Current Baseline)

Source file:
- crates/wbtools_oss/tests/registry_integration.rs

### A) Shortest Path Network
- shortest_path_network_finds_connected_route (line 13226)
- shortest_path_network_uses_edge_cost_field_multiplier (line 13289)
- shortest_path_network_respects_one_way_field_direction (line 13376)
- shortest_path_network_respects_blocked_field_edges (line 13426)
- shortest_path_network_respects_barrier_points (line 13513)
- shortest_path_network_applies_turn_penalty_to_cost (line 13573)
- shortest_path_network_can_forbid_left_turns (line 13629)
- shortest_path_network_respects_turn_restrictions_csv (line 13699)
- shortest_path_network_barrier_snap_distance_can_ignore_distant_barriers (line 15416)

### B) Network Service Area
- network_service_area_returns_nodes_within_max_cost (line 13854)
- network_service_area_respects_barrier_points (line 13923)
- network_service_area_edges_output_trims_segments_by_remaining_cost (line 13990)
- network_service_area_polygons_output_emits_origin_hulls (line 14073)
- network_service_area_polygons_output_tracks_partial_edge_frontier (line 14142)
- network_service_area_polygons_output_emits_one_polygon_per_origin (line 14210)
- network_service_area_polygons_output_respects_barrier_points (line 14296)
- network_service_area_polygons_output_respects_turn_restrictions_csv (line 14370)
- network_service_area_polygons_output_handles_duplicate_origins (line 14458)
- network_service_area_polygons_output_with_tiny_snap_tolerance (line 14522)
- network_service_area_polygons_output_single_reachable_node_has_degenerate_hull_bbox (line 14585)
- network_service_area_rejects_unknown_output_mode (line 14666)

### C) Network OD Cost Matrix
- network_od_cost_matrix_writes_expected_cost_row (line 14720)
- network_od_cost_matrix_uses_edge_cost_field_multiplier (line 14794)
- network_od_cost_matrix_marks_unreachable_with_one_way_field (line 14866)
- network_od_cost_matrix_marks_unreachable_with_blocked_field (line 14938)
- network_od_cost_matrix_marks_unreachable_with_barriers (line 15016)
- network_od_cost_matrix_applies_turn_penalty_to_cost (line 15098)
- network_od_cost_matrix_can_forbid_left_turns (line 15169)
- network_od_cost_matrix_respects_turn_restrictions_csv (line 15254)

### D) Network Routes From OD
- network_routes_from_od_outputs_route_geometry_and_cost (line 15483)
- network_routes_from_od_respects_turn_restrictions_csv (line 15566)
- network_routes_from_od_respects_barrier_points (line 15665)

### E) K-Shortest Paths Network
- k_shortest_paths_network_returns_multiple_ranked_routes (line 15743)
- k_shortest_paths_network_respects_turn_restrictions_csv (line 15838)
- k_shortest_paths_network_respects_barrier_points (line 15920)

### F) Network Diagnostics
- network_node_degree_identifies_junction_and_dead_ends (line 13780)
- network_connected_components_labels_disconnected_subnetworks (line 15350)

### G) Linear Referencing And Event Materialization
- locate_points_along_routes_writes_measure_and_offset_attributes (line 330)
- route_event_points_from_table_creates_measured_point_events (line 421)
- route_event_lines_from_table_segments_routes_by_from_to_measures (line 487)
- route_event_points_from_layer_creates_measured_point_events (line 568)
- route_event_lines_from_layer_segments_routes_by_from_to_measures (line 669)
- route_event_points_from_table_rejects_duplicate_route_ids (line 782)
- route_event_lines_from_layer_rejects_equal_from_to_measures (line 834)
- route_event_points_from_layer_can_disable_event_traceability_fields (line 897)

### H) Topology QA
- topology_validation_report_flags_invalid_line_and_polygon_features (line 252)

### I) Related Baseline Utility Coverage
- travelling_salesman_problem_run_end_to_end (line 7324)
- isobasins_produces_valid_basin_ids (line 4786)

## Phase 0 Exit Check

Current state supports a Phase 0 exit recommendation for the vector baseline consolidation scope:
- Artifacts exist for capability transparency and cookbook guidance.
- Core network constraint behavior has explicit integration tests.
- Representative cross-domain regression checks passed on 2026-04-12.
