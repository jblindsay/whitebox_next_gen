# Vector GIS Capability Gap Plan (Phased)

Date: 2026-04-12
Scope: Whitebox vector GIS analysis roadmap, with emphasis on remaining gaps after recent network/vector expansion work.

## Why This Plan Exists

Recent work closed many previously missing vector operations and significantly improved Python/R surface parity. This document reassesses what is still missing, prioritizes by user impact and implementation risk, and separates:

1. Already implemented baseline capabilities.
2. Remaining advanced capabilities and workflow gaps.

## Clarification: Network Analysis Items 1 and 2

The earlier gap list overstated these two as missing. Current status:

- Item 1 (turn-aware shortest path): Baseline implemented.
  - `shortest_path_network`
  - `k_shortest_paths_network`
  - Supports turn penalties, U-turn penalties, and turn restrictions CSV.
- Item 2 (service area / isochrone capability): Baseline implemented.
  - `network_service_area`
  - Supports impedance/cost fields and turn-restriction-aware options.

What still remains for these two:

- Time-dependent network costs (time-of-day/day-of-week edge impedance).
- Transit/multimodal impedance models.
- Richer service-area outputs and generalized cartographic isochrone polygonization options.

### Turn Restrictions CSV Examples (Current Behavior)

The network tools that accept `turn_restrictions_csv` now support both hard restrictions and per-turn additive costs.

Required columns:

- `prev_x,prev_y,node_x,node_y,next_x,next_y`

Optional columns:

- `forbidden` (`true/false`; defaults to `true` when no turn-cost column is present)
- one of `turn_cost`, `penalty`, `cost`, or `extra_cost` (non-negative additive turn cost)

Example A: classic hard restrictions only

```csv
prev_x,prev_y,node_x,node_y,next_x,next_y
0,0,1,0,1,1
5,2,5,3,6,3
```

Example B: mixed restrictions and weighted turns

```csv
prev_x,prev_y,node_x,node_y,next_x,next_y,forbidden,turn_cost
0,0,1,0,1,1,true,
2,0,3,0,3,1,false,8.5
4,1,5,1,5,2,false,2.0
```

Interpretation of Example B:

- first row is a hard forbidden transition
- second and third rows are allowed turns with added costs (8.5 and 2.0)

Python invocation example (`network_od_cost_matrix`):

```python
from whitebox_workflows import WbEnvironment

wbe = WbEnvironment()
wbe.network_od_cost_matrix(
      input="network.gpkg",
      origins="origins.gpkg",
      destinations="destinations.gpkg",
      edge_cost_field="IMP",
      turn_restrictions_csv="turns.csv",
      output="od_costs.csv",
)
```

R invocation example (`network_od_cost_matrix`):

```r
library(whiteboxworkflows)

wbe <- wbe_new()
wbe_network_od_cost_matrix(
   wbe,
   input = "network.gpkg",
   origins = "origins.gpkg",
   destinations = "destinations.gpkg",
   edge_cost_field = "IMP",
   turn_restrictions_csv = "turns.csv",
   output = "od_costs.csv"
)
```

### Service-Area Polygon Workflow Examples (Per-Origin vs Merged Coverage)

`network_service_area` polygon output now supports both:

- per-origin polygons (default)
- merged coverage polygons per ring (`polygon_merge_origins = true`)

Python per-origin polygons:

```python
from whitebox_workflows import WbEnvironment

wbe = WbEnvironment()
wbe.network_service_area(
   input="network.gpkg",
   origins="origins.gpkg",
   max_cost=15.0,
   output_mode="polygons",
   output="service_area_per_origin.gpkg",
)
```

Python merged coverage polygons (by ring):

```python
from whitebox_workflows import WbEnvironment

wbe = WbEnvironment()
wbe.network_service_area(
   input="network.gpkg",
   origins="origins.gpkg",
   max_cost=15.0,
   ring_costs="5,10,15",
   output_mode="polygons",
   polygon_merge_origins=True,
   output="service_area_merged_by_ring.gpkg",
)
```

R per-origin polygons:

```r
library(whiteboxworkflows)

wbe <- wbe_new()
wbe_network_service_area(
   wbe,
   input = "network.gpkg",
   origins = "origins.gpkg",
   max_cost = 15.0,
   output_mode = "polygons",
   output = "service_area_per_origin.gpkg"
)
```

### Vehicle Routing Practical Constraints Added (2026-04-13)

To reduce the practical operations gap in network analysis workflows, the open-tier vehicle routing tools now expose additional per-route controls:

- `vehicle_routing_cvrp`
   - `priority_field`: supports required/high/normal/low stop prioritization under constrained routing
   - `vehicle_fixed_cost`: fixed per-route cost included in route/objective cost reporting
   - `max_route_distance`: caps travel distance per route (including return to depot)
   - `travel_speed`: enables time scaling for operational route duration checks
   - `max_route_time`: caps route duration (distance / speed, including return to depot)
   - `max_stops_per_vehicle`: caps stop count per vehicle route
- `vehicle_routing_vrptw`
   - `priority_field`: supports required/high/normal/low stop prioritization under constrained routing
   - `vehicle_fixed_cost`: fixed per-route cost included in route/objective cost reporting
   - `max_route_distance`: caps total route travel distance (including return to depot)
   - `max_route_time`: caps total route duration (including return to depot)
   - `depot_close_time`: enforces a hard return deadline to depot
   - `max_stops_per_vehicle`: caps stop count per vehicle route

These are optional and preserve prior behavior when omitted.

Validation and behavior checks were added in integration tests:

- `vehicle_routing_cvrp_respects_max_route_distance`
- `vehicle_routing_cvrp_respects_max_route_time`
- `vehicle_routing_cvrp_total_cost_includes_vehicle_fixed_cost`
- `vehicle_routing_cvrp_priority_field_prefers_required_stops`
- `vehicle_routing_vrptw_respects_max_route_time`
- `vehicle_routing_vrptw_respects_max_route_distance`
- `vehicle_routing_vrptw_respects_depot_close_time`
- `vehicle_routing_vrptw_total_cost_includes_vehicle_fixed_cost`
- `vehicle_routing_vrptw_priority_field_prefers_required_stops`

R merged coverage polygons (by ring):

```r
library(whiteboxworkflows)

wbe <- wbe_new()
wbe_network_service_area(
   wbe,
   input = "network.gpkg",
   origins = "origins.gpkg",
   max_cost = 15.0,
   ring_costs = "5,10,15",
   output_mode = "polygons",
   polygon_merge_origins = TRUE,
   output = "service_area_merged_by_ring.gpkg"
)
```

## Deeper Audit Corrections (2026-04-12)

A second pass against runtime and wrapper surfaces found additional capabilities that were previously listed as gaps.

Reclassified as implemented baseline:

- Linear-referencing event generation (dynamic-segmentation style):
   - `route_event_points_from_table`
   - `route_event_lines_from_table`
   - `route_event_points_from_layer`
   - `route_event_lines_from_layer`
- Route point location with tolerance/offset control:
   - `locate_points_along_routes` (`max_offset_distance`)
- Basic network diagnostics:
   - `network_connected_components`
   - `network_node_degree`
- Specialized stream-network topology/QA tools:
   - `repair_stream_vector_topology`
   - `vector_stream_network_analysis`

Reclassified as partially covered (not full enterprise depth):

- Dynamic segmentation is available for common event materialization workflows, but full route-calibration and measure-governance tooling is still missing.
- Network diagnostics exists at baseline level, but deeper integrity/impedance audit workflows remain a gap.
- Topology auto-fix exists for specific hydrography workflows (`repair_stream_vector_topology`) but not as a generalized, rule-engine-driven auto-fix framework.

## Current Baseline (Implemented)

### Network / Routing

- `shortest_path_network`
- `k_shortest_paths_network`
- `network_service_area`
- `network_routes_from_od`
- `network_od_cost_matrix`
- `network_connected_components`
- `network_node_degree`
- `travelling_salesman_problem`

### Topology / QA (Foundational)

- `topology_validation_report` (report-driven topology QA baseline)

### Core Vector Geoprocessing (already broad)

- Overlay, selection, spatial join, proximity, and many geometry/table operations are present.

## Remaining Gaps by Category (Top Priority)

## A) Network Analysis (Advanced)

1. Time-dependent routing (edge cost profiles by time period).
2. Multimodal routing (walk/drive/transit).
3. VRP family:
   - Capacitated VRP.
   - VRP with time windows.
   - Pickup and delivery routing.
4. Advanced map matching for GPS trajectories.
5. Advanced network pre-processing diagnostics beyond current baseline (`network_connected_components`, `network_node_degree`), including impedance integrity and unreachable-path root-cause diagnostics.

## B) Topology and Data Quality

1. Rule-based topology validator with structured rule sets (must-not-overlap, must-not-have-gaps, etc.).
2. Generalized auto-fix workflows per rule class (beyond current specialized stream-topology repair tools).
3. Persistent topology context/layer model for iterative editing QA cycles.
4. Network conflation and change reconciliation tooling.

## C) Linear Referencing

1. Route calibration and recalibration from control points.
2. Event overlay/split/merge tools.
3. Full offset event geometry generation from route centerlines (beyond point-location offset tolerance).
4. Route-measure QA (gaps, overlaps, monotonicity errors).

## D) Overlay/ETL at Scale and Robustness

1. High-throughput chunked/streamed overlay for very large vector layers.
2. Sliver detection and elimination framework.
3. Expanded invalid geometry diagnostics and repair modes.
4. Parcel-fabric style topology maintenance patterns.

## E) Attribute/Network Analytics Depth

1. Rich expression engine enhancements (window/group operators in vector context).
2. Native network centrality and accessibility indices.
3. Expanded OD diagnostics and uncertainty/quality metrics.

## Phased Roadmap

## Phase 0: Baseline Consolidation (2-4 weeks)

Goal: lock in what is already implemented and avoid capability confusion.

Deliverables:

1. Capability matrix document: tool -> category -> maturity -> wrapper parity (Rust/Python/R).
2. Network examples cookbook for:
   - shortest path with turn restrictions
   - service area with cost field
   - OD matrix and route extraction
3. Regression tests for network options:
   - one-way handling
   - blocked edges
   - turn restrictions CSV
4. Naming/UX consistency pass in wrappers and docs.

Exit criteria:

- All network baseline tools have reproducible examples and validation tests.
- No ambiguity on implemented vs planned NA capabilities.

## Phase 1: High-ROI Network Extensions (4-8 weeks)

Goal: extend current network foundation without introducing full new solver families.

Deliverables:

1. Time-dependent edge cost support (optional temporal cost profile input).
2. Service-area output enhancements:
   - stronger polygonization controls
   - multiple ring outputs
3. Network diagnostics toolkit:
   - disconnected subnetworks
   - dead-end/degree anomalies
   - unreachable OD pairs diagnostics
4. Map-matching v1 (snap trajectory points to candidate paths).

Exit criteria:

- Temporal routing example suite passes.
- Service-area outputs validated on benchmark datasets.
- Diagnostics identify common data integrity failures.

Phase 1 kickoff artifacts created (2026-04-12):

1. VECTOR_PHASE1_TEMPORAL_EDGE_COST_SCHEMA_2026-04-12.md
2. VECTOR_PHASE1_MAP_MATCHING_V1_SPEC_2026-04-12.md
3. VECTOR_PHASE1_SERVICE_AREA_RING_ENHANCEMENTS_2026-04-12.md
4. VECTOR_PHASE1_IMPLEMENTATION_TRACKER_2026-04-12.md

## Phase 2: Topology Rule Engine + Linear Referencing Core (8-14 weeks)

Goal: add core enterprise-style vector QA capabilities.

Deliverables:

1. Rule-based topology validation framework.
2. Rule-specific auto-fixers (safe, auditable changes).
3. Route calibration/recalibration tools.
4. Event splitting/merging/overlay operations.
5. Route-measure QA and governance checks.

Exit criteria:

- Topology rule engine supports at least 6 core rule types.
- Linear referencing workflows can produce event-enriched network layers from route tables.

Phase 2 kickoff artifacts created (2026-04-12):

1. VECTOR_PHASE2_TOPOLOGY_RULE_ENGINE_SPEC_2026-04-12.md
2. VECTOR_PHASE2_LINEAR_REFERENCING_CORE_SPEC_2026-04-12.md
3. VECTOR_PHASE2_IMPLEMENTATION_TRACKER_2026-04-12.md

## Phase 3: Advanced Optimization and Multimodal (12-20 weeks)

Goal: close major parity gaps with advanced GIS network stacks.

Deliverables:

1. VRP solver set:
   - CVRP
   - VRPTW
   - Pickup/Delivery
2. Multimodal routing model.
3. Network centrality and accessibility metrics.
4. OD uncertainty and sensitivity analysis options.

Exit criteria:

- End-to-end benchmark scenarios for logistics, municipal routing, and accessibility analytics.
- Production-ready API in Rust/Python/R surfaces.

Phase 3 kickoff artifacts created (2026-04-12):

1. VECTOR_PHASE3_IMPLEMENTATION_TRACKER_2026-04-12.md
2. VECTOR_PHASE3_ADVANCED_OPTIMIZATION_SPEC_2026-04-12.md
3. VECTOR_PHASE3_MULTIMODAL_AND_ACCESSIBILITY_SPEC_2026-04-12.md

## Phase 4: Scale and Reliability Hardening (ongoing)

Goal: ensure performance and reliability on large operational datasets.

Deliverables:

1. Chunked/streamed overlay architecture.
2. Sliver-aware overlay cleanup pipeline.
3. Geometry repair diagnostics and confidence reporting.
4. Stress/perf harness for large vector datasets.

Exit criteria:

- Performance targets met on large benchmark suite.
- Deterministic and documented error handling for invalid/edge-case geometries.

## Prioritized Top 10 Backlog Items (Cross-Phase)

1. Time-dependent routing.
2. Service-area polygon/ring improvements.
3. Network diagnostics toolkit.
4. Map matching v1.
5. Topology rule engine.
6. Topology auto-fix operations.
7. Route-measure QA tooling.
8. Route calibration.
9. VRPTW.
10. Chunked high-volume overlay.

## Suggested Immediate Next Actions (This Sprint)

1. Finalize Phase 2 topology rule schema and implement first two rule evaluators.
2. Implement route calibration MVP with control-point input contract.
3. Add integration fixtures for topology-rule violations and route-measure QA.
4. Define wrapper parity checklist (Rust/Python/R) for all planned Phase 2 tools.

## Notes on Rating Implications

Given current implementation, Whitebox should be described as:

- Strong in baseline network analysis and scripted vector workflows.
- Not yet complete for advanced network optimization, full topology-rule ecosystems, and linear-referencing depth.

This distinction should be reflected in future capability summaries to avoid understating progress already achieved.
