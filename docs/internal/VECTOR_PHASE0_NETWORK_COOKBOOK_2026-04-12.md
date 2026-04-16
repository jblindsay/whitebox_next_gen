# Vector Phase 0 Network Cookbook

Date: 2026-04-12
Purpose: Deliverable for Phase 0 item 2 (network examples cookbook) and item 4 naming/UX consistency guidance.

## Canonical Naming And UX Conventions (Phase 0)

Use these canonical terms consistently in wrappers/docs/examples:
- input: network line layer path
- origins: origin point layer path
- destinations: destination point layer path
- edge_cost_field: numeric edge impedance field
- one_way_field: one-way direction field
- blocked_field: blocked-edge boolean/integer flag field
- barriers: optional barrier point layer path
- turn_restrictions_csv: CSV defining forbidden transitions
- output: output path (vector or CSV depending on tool)

Output-mode conventions:
- network_service_area output_mode: nodes, edges, polygons
- Network cost outputs should use the term cost consistently (not mixed with weight in user-facing docs)

## Recipe 1: Shortest Path With Turn Restrictions

Goal:
- Compute a path between two coordinates while enforcing turn restrictions.

Tool:
- shortest_path_network

Core args:
- input
- start_x, start_y, end_x, end_y
- turn_restrictions_csv
- optional: one_way_field, blocked_field, edge_cost_field

Expected output:
- line vector path with route geometry and route-level cost attributes.

Validation evidence in tests:
- shortest_path_network_respects_turn_restrictions_csv
- shortest_path_network_respects_one_way_field_direction
- shortest_path_network_respects_blocked_field_edges

## Recipe 2: Service Area With Cost Field

Goal:
- Delineate reachable network space from origin points within max_cost.

Tool:
- network_service_area

Core args:
- input
- origins
- max_cost
- output_mode (nodes, edges, polygons)
- optional: edge_cost_field, one_way_field, blocked_field, barriers, turn_restrictions_csv

Expected outputs:
- nodes mode: reachable nodes
- edges mode: traversable edges (partial trimming supported)
- polygons mode: origin-level hull polygons

Validation evidence in tests:
- network_service_area_returns_nodes_within_max_cost
- network_service_area_edges_output_trims_segments_by_remaining_cost
- network_service_area_polygons_output_emits_one_polygon_per_origin
- network_service_area_polygons_output_respects_turn_restrictions_csv

## Recipe 3: OD Cost Matrix And Route Extraction

Goal:
- Produce OD costs and optionally full OD route geometries.

Tools:
- network_od_cost_matrix
- network_routes_from_od

Core args:
- input
- origins
- destinations
- optional: edge_cost_field, one_way_field, blocked_field, barriers, turn_restrictions_csv

Expected outputs:
- network_od_cost_matrix: CSV path (cost matrix)
- network_routes_from_od: vector route geometries with route-level cost

Validation evidence in tests:
- network_od_cost_matrix_writes_expected_cost_row
- network_od_cost_matrix_marks_unreachable_with_one_way_field
- network_od_cost_matrix_marks_unreachable_with_blocked_field
- network_od_cost_matrix_respects_turn_restrictions_csv
- network_routes_from_od_outputs_route_geometry_and_cost
- network_routes_from_od_respects_turn_restrictions_csv

## Recipe 4: Baseline Network Diagnostics

Goal:
- Run foundational diagnostics before routing workflows.

Tools:
- network_connected_components
- network_node_degree

Suggested practice:
- Run connected-components first to identify fragmented subnetworks.
- Run node-degree analysis second to identify dead-ends/junction quality.
- Gate downstream routing workflow execution on diagnostics thresholds.

## Recipe 5: Topology Audit Diagnostics (Phase 1)

Goal:
- Produce actionable root-cause diagnostics for routing failures in a single pass.

Tool:
- network_topology_audit

Core args:
- input
- optional: snap_tolerance, one_way_field, blocked_field, report

Expected outputs:
- output vector (points): per-node audit fields
	- NODE_ID, DEG_OUT, DEG_IN, DEG_UNI, COMPONENT, NODE_TYPE
- optional report JSON:
	- component_count
	- largest_component_node_share
	- dead_end_node_count
	- isolated_node_count
	- source_node_count
	- sink_node_count
	- potential_routing_failures

Suggested practice:
1. Run network_topology_audit before OD/service-area workloads.
2. Fail fast if largest_component_node_share is low or isolated_node_count is non-zero.
3. Use NODE_TYPE + COMPONENT outputs to drive targeted cleanup and re-test loops.

Validation evidence in tests:
- network_topology_audit_reports_nodes_and_components
- network_topology_audit_detects_disconnected_components_and_warns

## Linear Referencing Companion Recipes

These are baseline-enabled and should be treated as first-class workflow inputs:
- locate_points_along_routes
- route_event_points_from_table
- route_event_lines_from_table
- route_event_points_from_layer
- route_event_lines_from_layer

Recommended sequencing:
1. Validate route network quality.
2. Locate/source events onto routes.
3. Run event QA (gaps/overlaps/measure consistency rules).
4. Feed governed events into downstream planning workflows.

## Phase 0 Completion Checklist Mapping

- Cookbook examples: completed in this document for shortest path, service area, and OD workflows.
- Naming/UX consistency: canonical field and argument naming documented here for wrappers and docs.
- Regression coverage references: mapped above to existing integration tests in registry_integration.rs.
