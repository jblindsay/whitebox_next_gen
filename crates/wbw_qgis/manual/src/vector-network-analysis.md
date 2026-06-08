# Network Analysis — Tool Reference


---

## Build Network Topology

**Function name:** `build_network_topology`


*No help documentation available for this tool.*


---

## Closest Facility Network

**Function name:** `closest_facility_network`


Experimental

Finds the minimum-cost network route from each incident point to its nearest reachable facility point.

vector network closest-facility routing

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`incidents`Incident/demand point layer.Required`incidents.shp`
`facilities`Facility/supply point layer.Required`facilities.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from incident/facility points to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output closest-facility route line vector path.Required—

### Examples

*Routes each incident point to the nearest reachable facility by network cost.*
`wbe.closest_facility_network(facilities='facilities.shp', incidents='incidents.shp', input='network.shp', output='closest_facility_routes.shp')`


---

## Emergency Scenario Routing And Accessibility Simulator

**Function name:** `emergency_scenario_routing_and_accessibility_simulator`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Emergency Accessibility Scenario Planning**

#### Who It Is For

- Emergency management teams planning resilience under flood/fire/closure scenarios.
- Public safety routing analysts evaluating critical-facility reachability under disruptions.

#### Primary User

Emergency management, public safety operations, and municipal resilience planning teams.

#### What It Does

- Simulates emergency network accessibility under multiple disruption scenarios.
- Compares scenario accessibility against baseline service-area coverage.
- Supports scenario-specific blocked-edge simulation using network attribute mapping.
- Outputs baseline and worst-case service areas, scenario KPI table, and simulation report JSON.

#### How It Works

- Runs baseline merged multi-ring service areas from critical facilities using network_service_area.
- Reads scenario CSV (`scenario_id,max_cost_multiplier[,blocked_value]`).
- For each scenario:
- Scales max travel cost by `max_cost_multiplier`.
- Optionally maps `blocked_value` to scenario-blocked edges using `scenario_block_source_field`.
- **Scope boundary:** Emergency Accessibility Scenario Planning simulates disrupted-network conditions for critical facility coverage — it is the service-area tool for emergency resilience and response planning. Service Area Planning and Coverage Optimization (6.2) addresses public infrastructure coverage under normal operating conditions. Market Access and Site Planning (6.7) evaluates commercial expansion by drive-time access and competitive positioning. Choose this tool for resilience analysis, 6.2 for infrastructure coverage gaps, and 6.7 for commercial site decisions.
- Computes scenario service area polygons and demand-point coverage percent.
- Computes `delta_from_baseline_pct` per scenario and identifies best/worst scenario outcomes.

### Inputs

ParameterTypeRequiredDescription
`network`LineVector pathRequiredNetwork layer for routing
`critical_facilities`PointVector pathRequiredOrigin facilities (hospitals/fire/EMS/etc.)
`demand_points`PointVector pathOptionalDemand points for scenario coverage KPIs
`ring_costs`array[float]RequiredService area ring costs (e.g., [5,10,15])
`scenario_csv`pathRequiredCSV: scenario_id,max_cost_multiplier[,blocked_value]
`scenario_template`stringOptionalScenario authoring template: `custom` | `flood` | `wildfire` | `earthquake`; applies template guardrails
`scenario_block_source_field`stringOptionalNetwork attribute used to match scenario blocked_value
`baseline_service_areas`vector pathRequiredOutput baseline service areas
`worst_case_service_areas`vector pathRequiredOutput worst-scenario service areas
`scenario_summary_csv`pathRequiredOutput scenario KPI summary CSV
`simulation_report`pathRequiredOutput simulation summary JSON

### Outputs

OutputTypeContents
`baseline_service_areas`VectorBaseline merged service area polygons
`worst_case_service_areas`VectorWorst-performing scenario service area polygons
`scenario_summary_csv`CSVscenario_id, blocked_value, covered_pct, delta_from_baseline_pct and related KPIs
`simulation_report`JSONbaseline stats, scenario comparisons, best/worst scenario summary

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("emergency_scenario_routing_and_accessibility_simulator",
    network="city_network.gpkg",
    critical_facilities="critical_facilities.gpkg",
    demand_points="demand_points.gpkg",
    ring_costs=[5, 10, 15],
    scenario_csv="scenarios.csv",
    scenario_block_source_field="STATUS",
    baseline_service_areas="output/baseline_service_areas.gpkg",
    worst_case_service_areas="output/worst_service_areas.gpkg",
    scenario_summary_csv="output/scenario_summary.csv",
    simulation_report="output/simulation_report.json",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Fleet Routing And Dispatch Optimizer

**Function name:** `fleet_routing_and_dispatch_optimizer`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Fleet Routing and Dispatch Optimization**

#### Who It Is For

- Logistics dispatch teams planning daily vehicle assignments.
- Municipal waste and field-maintenance operations with constrained fleet capacity/time.
- Courier and distribution operations requiring transparent route exceptions.

#### Primary User

Logistics operations leaders, municipal service planners, and fleet-dispatch platform teams.

#### What It Does

- Optimizes vehicle dispatch plans across depots and stops for logistics and field operations.
- Builds feasible routes under capacity and shift-time constraints.
- Produces assignment, KPI, and exception outputs for operations review.
- Supports objective mode selection (`minimize_distance`, `minimize_time`, `minimize_cost`, `balanced`).

#### How It Works

- Parses depot and stop layers plus fleet specifications from `vehicles_csv`.
- Harmonizes depot/stop CRS to the network CRS when EPSG metadata is available and validates projected CRS for distance/time calculations.
- Filters infeasible stops where demand exceeds max vehicle capacity.
- Applies objective-aware greedy nearest-feasible stop construction for initial route assignment (`distance`, `time`, `cost`, `balanced`).
- Applies optional edge restrictions from CSV (`from_x,from_y,to_x,to_y,closed,penalty_factor`) as closures or impedance penalties.
- Runs local 2-opt sequence refinement for incremental route improvement.
- Computes per-route and fleet KPIs and emits exceptions with reason codes.

### Inputs

ParameterTypeRequiredDescription
`network`LineVector pathRequiredStreet/network layer used by routing workflow
`depots`PointVector pathRequiredDepot or start/end locations for vehicles
`stops`PointVector pathRequiredStops/tasks to assign to routes
`vehicles_csv`pathRequiredFleet specs (vehicle_id, capacity, available_time_minutes, cost_per_minute, cost_per_km, depot_id)
`objective`stringOptionalObjective mode: `minimize_distance`, `minimize_time`, `minimize_cost`, `balanced`
`restrictions`pathOptionalOptional restrictions CSV (`from_x,from_y,to_x,to_y[,closed][,penalty_factor]`) used for edge closures or impedance penalties
`routes_output`vector pathRequiredOutput route vector path
`assignment_csv_output`pathRequiredOutput stop-to-route assignment CSV
`route_kpis_csv_output`pathRequiredOutput per-route/fleet KPI CSV
`exceptions_csv_output`pathRequiredOutput infeasible stop diagnostics CSV

### Outputs

OutputTypeContents
`routes_output`VectorRoute geometries and route-level summaries by vehicle
`assignment_csv_output`CSVstop_id, route_id, vehicle_id, sequence_order, arrival/departure times
`route_kpis_csv_output`CSVPer-route metrics and fleet roll-up summary
`exceptions_csv_output`CSVstop_id with reason code (`demand_exceeds_max_vehicle_capacity`, `no_feasible_route`)

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("fleet_routing_and_dispatch_optimizer",
    network="city_network.gpkg",
    depots="depots.gpkg",
    stops="daily_stops.gpkg",
    vehicles_csv="fleet_specs.csv",
    objective="minimize_cost",
    routes_output="output/routes.gpkg",
    assignment_csv_output="output/assignments.csv",
    route_kpis_csv_output="output/route_kpis.csv",
    exceptions_csv_output="output/exceptions.csv",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Generate Network Nodes

**Function name:** `generate_network_nodes`


*No help documentation available for this tool.*


---

## K Shortest Paths Network

**Function name:** `k_shortest_paths_network`


Experimental

Finds the k shortest simple paths between start and end coordinates over a line network.

vector network k-shortest-paths

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`start_x`Start x coordinate.Required`0.0`
`start_y`Start y coordinate.Required`0.0`
`end_x`End x coordinate.Required`100.0`
`end_y`End y coordinate.Required`100.0`
`k`Number of shortest paths to return.Required`3`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from start/end coordinates to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output line vector path.Required—

### Examples

*Computes multiple alternative simple paths between two points on a line network.*
`wbe.k_shortest_paths_network(end_x=100.0, end_y=100.0, input='network.shp', k=3, output='k_shortest_paths.shp', start_x=0.0, start_y=0.0)`


---

## Location Allocation Network

**Function name:** `location_allocation_network`


Experimental

Selects k facilities and allocates demand points by network cost with greedy or exact solving, optional capacities, and required/forbidden candidate constraints.

vector network location-allocation allocation

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`demand_points`Demand point layer to allocate.Required`demand.shp`
`facilities`Candidate facility point layer.Required`facilities.shp`
`facility_count`Number of facilities to select (k).Required`2`
`solver_mode`Solver mode: auto, greedy, or exact (exact is intended for smaller problems).Optional—
`demand_weight_field`Optional numeric demand weight field in demand_points (default weight=1).Optional—
`facility_capacity_field`Optional numeric capacity field in facilities; capacity is consumed by demand_weight_field values.Optional—
`required_facility_field`Optional boolean facility field marking candidates that must be selected.Optional—
`forbidden_facility_field`Optional boolean facility field marking candidates that must not be selected.Optional—
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from demand/facility points to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output allocated route line vector path.Required—

### Examples

*Selects facilities and allocates demand points using network travel cost.*
`wbe.location_allocation_network(demand_points='demand.shp', facilities='facilities.shp', facility_count=2, input='network.shp', output='location_allocation_routes.shp')`


---

## Map Matching v1

**Function name:** `map_matching_v1`


Experimental

Snaps trajectory points onto a line network and reconstructs an inferred route with diagnostics.

vector network map-matching

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`trajectory_points`Input trajectory point layer.Required`trajectory_points.shp`
`timestamp_field`Trajectory field used for time ordering.Required`timestamp`
`search_radius`Optional candidate search radius around each trajectory point.Optional`25.0`
`candidate_k`Optional number of nearest candidates retained per point.Optional`5`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance for snapping trajectory points to network nodes.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`matched_points_output`Optional output vector path for per-point diagnostics.Optional—
`match_report`Optional JSON output path for summary diagnostics.Optional—
`output`Output line vector path for inferred route.Required—

### Examples

*Matches time-ordered trajectory points to a network and emits route and diagnostics outputs.*
`wbe.map_matching_v1(candidate_k=5, input='network.shp', matched_points_output='matched_points.shp', output='matched_route.shp', search_radius=25.0, timestamp_field='timestamp', trajectory_points='trajectory_points.shp')`


---

## Market Access And Site Intelligence Workflow

**Function name:** `market_access_and_site_intelligence_workflow`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Market Access and Site Planning**

#### Who It Is For

- Retail chains evaluating expansion locations based on demand accessibility and competitive saturation.
- Healthcare networks rating candidate clinic/hospital sites for market coverage and competitive positioning.
- Franchise developers assessing new territory opportunities via catchment analysis.

#### Primary User

Commercial real estate, retail operations, healthcare network planning, franchise development.

#### What It Does

- Evaluates candidate site locations for commercial expansion (retail, healthcare, franchise).
- Computes drive-time catchment areas for each candidate.
- Measures demand coverage and competitive overlap positioning.
- Ranks candidates by composite score: 50% demand coverage + 25% accessibility + 25% low competitive overlap.
- Outputs ranked candidates, competitive analysis, and executive summary with decision metrics.
- Adds opportunity bands and optional market action queue output for expansion triage.

#### How It Works

- For each candidate site:
- Compute demand coverage (% of demand points within max ring cost).
- Compute average distance to demand points (accessibility proxy).
- Compute competitive overlap vs existing/competitor sites (% within catchment radius).
- Existing-site baseline coverage is computed from the existing site layer, not from candidate seed sites.
- Accessibility score is normalized across the evaluated candidate set.
- Composite rank score = 0.50 × coverage + 0.25 × accessibility + 0.25 × (100 - overlap).
- **Scope boundary:** Market Access and Site Planning evaluates candidate commercial sites for expansion using drive-time access, demand coverage, and competitive positioning — it is the service-area tool for commercial decisions. Service Area Planning and Coverage Optimization (6.2) addresses public infrastructure coverage diagnostics. Emergency Accessibility Scenario Planning (6.6) simulates disrupted-network scenarios for resilience analysis. Choose this tool for commercial expansion, 6.2 for infrastructure planning, and 6.6 for emergency response planning.
- Rank candidates by composite score; classify each site as `expand_now`, `pilot`, `monitor`, or `saturated`; emit top candidates + decision gate (coverage > 70% AND overlap ParameterTypeRequiredDescription
`network`LineVector pathRequiredStreet/transit network for routing
`sites_existing`PointVector pathRequiredExisting own or benchmark competitive sites
`sites_candidates`PointVector pathRequiredCandidate expansion site locations
`demand_surface`PointVector pathRequiredDemand locations (customers, population centroids)
`competition_sites`PointVector pathOptionalCompetitor locations (separate from own/benchmark)
`ring_costs`array[float]RequiredDrive-time costs for catchments (e.g., [5, 10, 15])
`catchments_output`vector pathRequiredOutput drive-time catchment polygons
`overlap_analysis_output`vector pathRequiredOutput competitive overlap analysis layer
`candidate_rank_csv`pathRequiredOutput CSV: ranked candidates with KPIs
`executive_summary_json`pathRequiredOutput JSON: market metrics and decision gate
`market_action_queue_csv`pathOptionalOptional prioritized expansion action queue CSV

**Important input roles:** `sites_existing` is the incumbent baseline used for existing coverage and coverage-gain calculations. `competition_sites` is an optional separate competitor layer used for overlap pressure. If `competition_sites` is omitted, overlap falls back to `sites_existing`.

### Outputs

OutputTypeContents
`catchments_output`VectorDrive-time catchment polygons per candidate
`overlap_analysis_output`Vector/GeoJSONCandidate-level overlap visualization with coverage gain and opportunity band
`candidate_rank_csv`CSVrank, site_id, x, y, demand_coverage_pct, coverage_gain_pct, avg_distance_to_demand, competitive_overlap_pct, accessibility_score, composite_rank_score, opportunity_band
`executive_summary_json`JSONtotal_candidates, market_metrics, top_candidates, recommendation, decision_gate, decision_rationale
`market_action_queue_csv`CSVPrioritized expansion actions by candidate/opportunity band

**Map interpretation:** `catchments_output` contains one candidate-level polygon per candidate (not separate ring-band polygons). Catchments are convex-hull trade areas of covered demand plus candidate location; if a hull cannot be formed, a small fallback square polygon may be written. `overlap_analysis_output` is a point layer, and square symbols there are usually marker style.

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("market_access_and_site_intelligence_workflow",
    network="city_network.gpkg",
    sites_existing="existing_retail.gpkg",
    sites_candidates="candidate_expansion_sites.gpkg",
    demand_surface="customer_demand_points.gpkg",
    competition_sites="competitor_locations.gpkg",
    ring_costs=[5, 10, 15],
    catchments_output="output/candidate_catchments.gpkg",
    overlap_analysis_output="output/competitive_overlap.gpkg",
    candidate_rank_csv="output/candidate_ranking.csv",
    executive_summary_json="output/market_summary.json",
    market_action_queue_csv="output/market_action_queue.csv",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Multimodal OD Cost Matrix

**Function name:** `multimodal_od_cost_matrix`


Experimental

Computes batched multimodal OD costs and mode summaries between origin and destination point sets.

vector network multimodal od-matrix

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`mode_field`Line attribute field that identifies travel mode per segment.Required`MODE`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`default_mode_speed`Default mode speed in coordinate-units per time unit (default: 1).Optional`1.0`
`mode_speed_overrides`Optional comma-separated mode:speed overrides (for example: walk:1.4,drive:12,transit:8).Optional—
`allowed_modes`Optional comma-separated allow-list of modes to include in routing.Optional—
`transfer_penalty`Optional additive penalty applied each time the route changes mode.Optional`0.0`
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics when using direct temporal input.Optional—
`scenario_bundle_csv`Optional CSV listing named temporal scenarios for comparative multi-scenario OD output.Optional—
`output`Output CSV path.Required—

### Examples

*Creates a multimodal OD matrix with route cost and mode-sequence summaries.*
`wbe.multimodal_od_cost_matrix(default_mode_speed=1.0, destinations='destinations.shp', input='network.shp', mode_field='MODE', mode_speed_overrides='walk:1.4,transit:8', origins='origins.shp', output='multimodal_od_matrix.csv', transfer_penalty=0.0)`


---

## Multimodal Routes From OD

**Function name:** `multimodal_routes_from_od`


Experimental

Builds route geometries for multimodal origin-destination point pairs with per-route mode summaries.

vector network multimodal routes

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`mode_field`Line attribute field that identifies travel mode per segment.Required`MODE`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`default_mode_speed`Default mode speed in coordinate-units per time unit (default: 1).Optional`1.0`
`mode_speed_overrides`Optional comma-separated mode:speed overrides (for example: walk:1.4,drive:12,transit:8).Optional—
`allowed_modes`Optional comma-separated allow-list of modes to include in routing.Optional—
`transfer_penalty`Optional additive penalty applied each time the route changes mode.Optional`0.0`
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics when using direct temporal input.Optional—
`scenario_bundle_csv`Optional CSV listing named temporal scenarios for comparative multi-scenario route output.Optional—
`output`Output route line vector path.Required—

### Examples

*Creates route lines for each reachable multimodal origin-destination pair.*
`wbe.multimodal_routes_from_od(default_mode_speed=1.0, destinations='destinations.shp', input='network.shp', mode_field='MODE', mode_speed_overrides='walk:1.4,transit:8', origins='origins.shp', output='multimodal_routes_from_od.gpkg', transfer_penalty=0.0)`


---

## Multimodal Shortest Path

**Function name:** `multimodal_shortest_path`


Experimental

Finds a mode-aware shortest path over a line network with configurable transfer penalties.

vector network multimodal shortest-path

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`start_x`Start x coordinate.Required`0.0`
`start_y`Start y coordinate.Required`0.0`
`end_x`End x coordinate.Required`100.0`
`end_y`End y coordinate.Required`100.0`
`mode_field`Line attribute field that identifies travel mode per segment.Required`MODE`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from start/end coordinates to nearest network node.Optional—
`default_mode_speed`Default mode speed in coordinate-units per time unit (default: 1).Optional`1.0`
`mode_speed_overrides`Optional comma-separated mode:speed overrides (for example: walk:1.4,drive:12,transit:8).Optional—
`allowed_modes`Optional comma-separated allow-list of modes to include in routing.Optional—
`transfer_penalty`Optional additive penalty applied each time the route changes mode.Optional`0.0`
`output`Output line vector path.Required—

### Examples

*Routes between two coordinates using mode-aware costs and transfer penalties.*
`wbe.multimodal_shortest_path(default_mode_speed=1.0, end_x=100.0, end_y=100.0, input='network.shp', mode_field='MODE', mode_speed_overrides='walk:1.4,transit:8', output='multimodal_shortest_path.shp', start_x=0.0, start_y=0.0, transfer_penalty=0.0)`

*Demonstrates walk-drive routing with mode filtering and transfer penalty.*
`wbe.multimodal_shortest_path(allowed_modes='walk,drive', default_mode_speed=1.0, end_x=100.0, end_y=100.0, input='network.shp', mode_field='MODE', mode_speed_overrides='walk:1.4,drive:12', output='multimodal_walk_drive_path.shp', start_x=0.0, start_y=0.0, transfer_penalty=2.0)`

*Demonstrates walk-transit routing with mode filtering and transfer penalty.*
`wbe.multimodal_shortest_path(allowed_modes='walk,transit', default_mode_speed=1.0, end_x=100.0, end_y=100.0, input='network.shp', mode_field='MODE', mode_speed_overrides='walk:1.4,transit:8', output='multimodal_walk_transit_path.shp', start_x=0.0, start_y=0.0, transfer_penalty=1.0)`


---

## Network Accessibility Metrics

**Function name:** `network_accessibility_metrics`


Experimental

Computes accessibility indices for origin points based on reachability to destinations with optional impedance cutoffs and decay functions.

vector network accessibility

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional`0.0`
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`impedance_cutoff`Optional maximum distance threshold for counting reachable destinations (default: infinite).Optional—
`decay_function`Optional decay function: 'none' (default), 'linear', or 'exponential' for distance-weighted accessibility.Optional`none`
`decay_parameter`Optional decay parameter (lambda for exponential, rate for linear).Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`parallel_execution`If true (default), evaluate origins in parallel for faster accessibility computation.Optional—
`output`Output point vector path (origins with accessibility metrics).Required—

### Examples

*Computes accessibility index for origins to destinations within cutoff distance.*
`wbe.network_accessibility_metrics(decay_function='none', destinations='destinations.shp', input='network.shp', origins='origins.shp', output='origins_accessibility.shp', snap_tolerance=0.0)`


---

## Network Centrality Metrics

**Function name:** `network_centrality_metrics`


Experimental

Computes baseline degree, closeness, and betweenness centrality metrics for network nodes.

vector network centrality

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional`0.0`
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from graph construction (true/1/yes blocks).Optional—
`output`Output point vector path.Required—

### Examples

*Computes node-level centrality metrics for a line network.*
`wbe.network_centrality_metrics(input='network.shp', output='network_centrality.gpkg', snap_tolerance=0.0)`


---

## Network Connected Components

**Function name:** `network_connected_components`


Experimental

Assigns a connected-component ID to each line feature in a network.

vector network components

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional`0.0`
`output`Output line vector path.Required—

### Examples

*Labels disconnected subnetworks with unique component IDs.*
`wbe.network_connected_components(input='network.shp', output='network_components.shp', snap_tolerance=0.0)`


---

## Network Node Degree

**Function name:** `network_node_degree`


Experimental

Extracts network nodes from line features and computes node degree and node type.

vector network topology

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional`0.0`
`output`Output point vector path.Required—

### Examples

*Creates a node point layer with network degree attributes.*
`wbe.network_node_degree(input='network.shp', output='network_nodes.shp', snap_tolerance=0.0)`


---

## Network OD Cost Matrix

**Function name:** `network_od_cost_matrix`


Experimental

Compute origin-destination cost matrix between point pairs. Calculates travel distance or cost along network paths.

vector network od-matrix

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output CSV path.Required—

### Examples

*Creates an OD cost matrix from origins and destinations on a line network.*
`wbe.network_od_cost_matrix(destinations='destinations.shp', input='network.shp', origins='origins.shp', output='od_matrix.csv')`


---

## Network Readiness And Diagnostics Intelligence

**Function name:** `network_readiness_and_diagnostics_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Network Readiness and Diagnostics**

#### Problem It Solves

Is this network structurally and cost-wise ready for reliable routing and service optimization workflows?

#### Who It Is For

- Routing analysts, municipal transportation planning teams, and utility network operations.

#### Primary User

Municipal/public works GIS teams, utilities, and logistics operations requiring reproducible network QA gates.

#### What It Does

- Audits line-based transportation/utility networks for operational routing readiness.
- Detects dead-end concentration and cost-consistency anomalies before routing runs.
- Emits a machine-readable readiness score with pass/fail quality gate and diagnostics outputs.

#### How It Works

- Loads a line network and validates geometry is line-based (`LineString`/`MultiLineString`).
- Builds node degree counts from line endpoints to quantify dead-end prevalence.
- Computes per-segment length costs and assesses variance using z-score outlier detection.
- Computes weighted readiness score from connectivity and cost-consistency components.
- Indicative formula: `overall = 0.6 * connectivity_score + 0.4 * cost_consistency_score`.

#### Why It Wins

- Replaces manual topology spot-checks with a reproducible score + diagnostics package that is machine-checkable and reportable.

#### Typical Buying Trigger

Teams encounter unstable routing outcomes and need an auditable pre-routing network quality gate.

#### Typical Presets

- default: balanced scoring for day-to-day network readiness checks.
- pre-routing gate: run before route optimization or service-area generation.
- data onboarding QA: run when ingesting new road/utility centerline deliveries.

### Inputs

ParameterOptionalDescription
networknoInput line network layer (street, transit, or utility network).
qa_reportnoOutput CSV path for detailed QA findings and issue counts.
diagnostics_layernoOutput GeoJSON/GeoPackage path containing diagnostic geometries.
readiness_scorenoOutput JSON path with readiness score, component scores, and pass/fail gate.

### Outputs

ParameterTypeDescription
qa_reportCSVTabular QA findings including severity, check type, count, and descriptive diagnostics.
diagnostics_layerGeoJSON/GeoPackageSpatial diagnostics layer for issue visualization and spatial triage.
readiness_scoreJSONMachine-readable readiness contract with overall score, component scores, penalties, and pass/fail gate.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.network_readiness_and_diagnostics_intelligence(
    network="data/street_network.shp",
    qa_report="output/network_readiness_qa.csv",
    diagnostics_layer="output/network_readiness_diagnostics.geojson",
    readiness_score="output/network_readiness_score.json",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Network Routes From OD

**Function name:** `network_routes_from_od`


Experimental

Builds route geometries for origin-destination point pairs over a line network.

vector network routes

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output route line vector path.Required—

### Examples

*Creates route line features for OD point pairs on a network.*
`wbe.network_routes_from_od(destinations='destinations.shp', input='network.shp', origins='origins.shp', output='network_routes.shp')`


---

## Network Service Area

**Function name:** `network_service_area`


Experimental

Computes reachable network nodes from origin points within a maximum network cost.

vector network service-area

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`max_cost`Maximum reachable path cost.Required`1000.0`
`ring_costs`Optional comma-separated ring thresholds for multi-ring outputs (for example: 5,10,15).Optional—
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin points to nearest network node.Optional—
`output_mode`Output mode: 'nodes' (default), 'edges' for cost-trimmed reachable edge segments, or 'polygons' for per-origin isochrone-like polygons from reachable edge envelopes.Optional—
`polygon_merge_origins`If true and output_mode='polygons', dissolve overlapping origin polygons into merged coverage per ring instead of emitting one polygon per origin.Optional—
`mode_field`Optional line attribute field identifying travel mode per segment; enables mode-aware service-area costs.Optional—
`default_mode_speed`Default mode speed in coordinate-units per time unit when mode_field is provided (default: 1).Optional—
`mode_speed_overrides`Optional comma-separated mode:speed overrides (for example: walk:1.4,drive:12).Optional—
`allowed_modes`Optional comma-separated allow-list of modes to include when mode_field is provided.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output service-area vector path.Required—

### Examples

*Finds all nodes reachable from origins within max_cost.*
`wbe.network_service_area(input='network.shp', max_cost=1000.0, origins='origins.shp', output='service_area_nodes.shp')`


---

## Network Topology Audit

**Function name:** `network_topology_audit`


Experimental

Audits a line network for topology anomalies—disconnected components, dead ends, and degree anomalies—that cause routing failures.

vector network diagnostics topology

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`one_way_field`Optional line field marking one-way edges for directional analysis.Optional—
`blocked_field`Optional line field marking blocked edges to exclude from analysis.Optional—
`report`Optional JSON output path for the audit summary report.Optional—
`output`Output point vector path for per-node diagnostics.Required—

### Examples

*Writes per-node degree and component diagnostics and a summary JSON report for the input network.*
`wbe.network_topology_audit(input='network.shp', output='network_node_audit.shp', report='audit_report.json')`


---

## OD Sensitivity Analysis

**Function name:** `od_sensitivity_analysis`


Experimental

Computes OD shortest-path costs with impedance perturbations and outputs sensitivity statistics via Monte Carlo sampling.

vector network sensitivity

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`origins`Origin point layer.Required`origins.shp`
`destinations`Destination point layer.Required`destinations.shp`
`edge_cost_field`Required numeric line field used as an impedance multiplier for perturbation analysis.Required`cost`
`impedance_disturbance_range`Range for cost perturbation as 'min_factor,max_factor' (e.g., '0.8,1.2' for ±20% variation).Optional`0.8,1.2`
`monte_carlo_samples`Number of Monte Carlo samples for perturbation analysis (default 1, max 100).Optional`10`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from origin/destination points to nearest network node.Optional—
`one_way_field`Optional line field marking one-way digitized edges.Optional—
`blocked_field`Optional line field marking blocked/closed edges.Optional—
`parallel_execution`If true (default), evaluates origin searches in parallel for baseline and perturbed OD runs.Optional—
`output`Output CSV path with OD pairs and sensitivity statistics.Required—

### Examples

*Computes OD costs with Monte Carlo impedance perturbation sensitivity.*
`wbe.od_sensitivity_analysis(destinations='destinations.shp', edge_cost_field='cost', impedance_disturbance_range='0.8,1.2', input='network.shp', monte_carlo_samples=10, origins='origins.shp', output='od_sensitivity.csv')`


---

## Shortest Path Network

**Function name:** `shortest_path_network`


Experimental

Finds the shortest path between start and end coordinates over a line network.

vector network shortest-path

### Parameters

NameDescriptionRequiredDefault
`input`Input line network layer.Required`network.shp`
`start_x`Start x coordinate.Required`0.0`
`start_y`Start y coordinate.Required`0.0`
`end_x`End x coordinate.Required`100.0`
`end_y`End y coordinate.Required`100.0`
`snap_tolerance`Optional node snapping tolerance for graph construction.Optional—
`max_snap_distance`Optional max distance from start/end coordinates to nearest network node.Optional—
`edge_cost_field`Optional numeric line field used as an impedance multiplier for segment length.Optional—
`one_way_field`Optional line field marking one-way digitized edges (true/1/yes means from first to second vertex only).Optional—
`blocked_field`Optional line field marking blocked/closed edges to exclude from routing (true/1/yes blocks).Optional—
`barriers`Optional barrier point layer; nearest network nodes are blocked from traversal.Optional—
`barrier_snap_distance`Optional max distance from each barrier point to a network node for blocking.Optional—
`turn_penalty`Optional additive cost applied to non-straight turns at network nodes.Optional—
`u_turn_penalty`Optional additive cost applied to U-turn transitions.Optional—
`forbid_u_turns`If true, disallow U-turn transitions.Optional—
`forbid_left_turns`If true, disallow left-turn transitions.Optional—
`forbid_right_turns`If true, disallow right-turn transitions.Optional—
`turn_restrictions_csv`Optional CSV of turn transitions using columns prev_x,prev_y,node_x,node_y,next_x,next_y. Optional columns: forbidden (default true when no turn_cost column is provided) and turn_cost (or penalty/cost/extra_cost) for per-turn additive cost.Optional—
`temporal_cost_profile`Optional CSV defining time-dependent edge costs (columns: edge_id,dow,start_minute,end_minute,value).Optional—
`temporal_edge_id_field`Optional network field used to match temporal_cost_profile edge_id values (default EDGE_ID).Optional—
`departure_time`Optional RFC3339 departure time used for temporal profile lookup.Optional—
`temporal_mode`Optional temporal interpretation mode: multiplier or absolute.Optional—
`temporal_fallback`Optional fallback when temporal row is missing: static_cost or error.Optional—
`temporal_profile_report`Optional JSON output path for temporal profile diagnostics (coverage, unmatched edges, fallback usage).Optional—
`output`Output line vector path.Required—

### Examples

*Computes shortest path between two points on a line network.*
`wbe.shortest_path_network(end_x=100.0, end_y=100.0, input='network.shp', output='shortest_path.shp', start_x=0.0, start_y=0.0)`


---

## Split Lines At Intersections

**Function name:** `split_lines_at_intersections`


*No help documentation available for this tool.*


---

## Snap Points To Network

**Function name:** `snap_points_to_network`


*No help documentation available for this tool.*


---

## Service Area Planning And Coverage Optimization

**Function name:** `service_area_planning_and_coverage_optimization`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Service Area Planning and Coverage Optimization**

#### Problem It Solves

Which facilities and scenarios provide the strongest service coverage, and where do unmet demand gaps remain?

#### Who It Is For

- Accessibility planners, emergency coverage teams, and utility service design analysts.

#### Primary User

Municipal/public safety GIS teams, utilities, and logistics planners managing service-area targets.

#### What It Does

- Builds network-based multi-ring service-area polygons from facility points over a line network.
- Flags uncovered demand points outside baseline service coverage.
- Produces scenario summary and candidate ranking CSV outputs for open/close planning workflows.

#### How It Works

- Validates network/facility geometry types and parses ring costs.
- Runs network service-area generation using the OSS network engine with polygon outputs.
- Computes demand coverage and uncovered demand diagnostics against generated polygons.
- Optionally evaluates scenario CSV variants (`scenario_id,facility_id,is_open[,capacity]`) and exports comparative KPI rows.
- Indicative KPI formula: `coverage_pct = 100 * covered_demand / total_demand`.

#### Why It Wins

- Replaces disconnected manual GIS steps with a reproducible network-derived coverage workflow and explicit planning artifacts.

#### Typical Buying Trigger

Teams need auditable facility coverage plans with scenario comparison outputs for governance or budget review.

#### Typical Presets

- baseline-only: generate default service areas + uncovered demand.
- scenario-planning: include open/close scenario CSV for option analysis.
- candidate-screening: rank facilities by demand coverage proxy for expansion planning.

### Inputs

ParameterOptionalDescription
networknoInput line network layer (roads, trails, utility lines).
facilitiesnoInput facility point layer used as service origins.
demand_pointsyesOptional demand point layer used for covered/uncovered diagnostics and KPI generation.
ring_costsnoNumeric array of travel-cost ring thresholds (e.g., `[5, 10, 15]`).
scenariosyesOptional CSV with `scenario_id,facility_id,is_open[,capacity]` for open/close scenario runs.
service_areasnoOutput vector path for service-area polygons.
uncovered_demandnoOutput vector path for uncovered demand points.
scenario_summary_csvnoOutput CSV path for scenario KPIs.
ranked_candidates_csvnoOutput CSV path for candidate ranking metrics.

### Outputs

ParameterTypeDescription
service_areasGeoJSON/GeoPackage/ShapefileBaseline network-derived multi-ring service-area polygons.
uncovered_demandGeoJSON/GeoPackage/ShapefileDemand points outside baseline service-area coverage.
scenario_summary_csvCSVScenario-level KPI table (`scenario_id,total_demand_covered_pct,avg_accessibility,outlier_count`).
ranked_candidates_csvCSVCandidate ranking table (`candidate_id,coverage_gain_pct,avg_distance_improvement,rank`).

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.service_area_planning_and_coverage_optimization(
    network="data/street_network.shp",
    facilities="data/facilities.shp",
    demand_points="data/demand_points.shp",
    ring_costs=[5.0, 10.0, 15.0],
    scenarios="data/service_scenarios.csv",
    service_areas="output/service_areas.geojson",
    uncovered_demand="output/uncovered_demand.geojson",
    scenario_summary_csv="output/scenario_summary.csv",
    ranked_candidates_csv="output/ranked_candidates.csv",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Transfer Attributes

**Function name:** `transfer_attributes`


*No help documentation available for this tool.*


---

## Travelling Salesman Problem

**Function name:** `travelling_salesman_problem`


This tool finds approximate solutions to `travelling salesman problems`,  the goal of which is to identify the shortest route connecting a set of locations. The tool uses an algorithm that applies a `2-opt heuristic` and a  `3-opt` heuristic as a fall-back if the initial approach  takes too long. The user must specify the names of the input points vector (`input`) and output lines vector file (`output`), as well as the duration, in seconds, over which the algorithm is allowed to search for improved solutions (`duration`). The tool works in parallel to find more optimal solutions. 

### Python API

```python
def travelling_salesman_problem(self, input: Vector, duration: int = 60) -> Vector:
```


---

## Vehicle Routing CVRP

**Function name:** `vehicle_routing_cvrp`


Experimental

Builds capacity-constrained multi-depot delivery routes with heterogeneous fleet controls, objective modes, and optional local optimization.

vector network routing optimization

### Parameters

NameDescriptionRequiredDefault
`network`Input line network layer (validated for contract parity).Required`network.gpkg`
`depot_points`Depot point layer; each point can contribute one or more vehicles.Required`depots.gpkg`
`stop_points`Delivery stop point layer.Required`stops.gpkg`
`demand_field`Numeric demand field in stop_points (default: demand).Optional`demand`
`priority_field`Optional stop priority field using values like required/high/normal/low or numeric ranks.Optional`priority`
`allowed_vehicle_profiles_field`Optional stop field listing compatible vehicle profiles (comma/semicolon/pipe-delimited).Optional—
`allowed_route_classes_field`Optional alias of allowed_vehicle_profiles_field for route-class compatibility rules.Optional—
`depot_id_field`Optional depot ID field used in route/assignment outputs.Optional—
`vehicle_count_field`Optional depot field for number of vehicles spawned at each depot.Optional—
`vehicle_capacity_field`Optional depot field overriding vehicle_capacity per depot/vehicle template.Optional—
`vehicle_fixed_cost_field`Optional depot field overriding vehicle_fixed_cost per depot/vehicle template.Optional—
`travel_speed_field`Optional depot field overriding travel_speed per depot/vehicle template.Optional—
`max_route_distance_field`Optional depot field overriding max_route_distance per depot/vehicle template.Optional—
`max_route_time_field`Optional depot field overriding max_route_time per depot/vehicle template.Optional—
`vehicle_profile_field`Optional depot field defining vehicle profile/category token used for stop compatibility.Optional—
`vehicle_route_class_field`Optional alias of vehicle_profile_field for route-class compatibility rules.Optional—
`vehicle_capacity`Per-vehicle capacity (> 0).Required`100.0`
`vehicle_fixed_cost`Optional fixed cost charged per dispatched vehicle/route (default: 0).Optional`0.0`
`max_vehicles`Optional maximum number of vehicles/routes to construct.Optional—
`max_route_distance`Optional maximum travel distance per route, including return to depot.Optional—
`travel_speed`Travel speed in coordinate-units per time unit (default: 1).Optional—
`max_route_time`Optional maximum route duration in model time units, including return to depot.Optional—
`max_stops_per_vehicle`Optional maximum number of stops assigned to each vehicle route.Optional—
`objective_mode`Route-construction objective: minimize_distance, minimize_vehicles, or minimize_cost.Optional`minimize_distance`
`apply_local_optimization`When true, applies a deterministic 2-opt local improvement pass to each constructed route (default: true).Optional`True`
`apply_simulated_annealing`When true, applies a seeded simulated annealing refinement pass per route after greedy/local optimization (default: false).Optional`False`
`sa_iterations`Maximum simulated annealing iterations per route when apply_simulated_annealing=true (default: 1500).Optional`1500`
`sa_initial_temperature`Initial simulated annealing temperature (> 0, default: 1.0).Optional`1.0`
`sa_cooling_rate`Simulated annealing cooling multiplier in (0, 1); default 0.995.Optional`0.995`
`sa_seed`Optional deterministic random seed for simulated annealing (default: 42).Optional`42`
`output`Output route line vector path.Required—
`assignment_output`Optional stop assignment point output with visit order/load diagnostics.Optional—

### Examples

*Builds CVRP routes and writes route lines with deterministic local optimization and optional simulated annealing controls.*
`wbe.vehicle_routing_cvrp(apply_local_optimization=True, apply_simulated_annealing=False, demand_field='demand', depot_points='depots.gpkg', network='network.gpkg', objective_mode='minimize_distance', output='cvrp_routes.gpkg', priority_field='priority', sa_cooling_rate=0.995, sa_initial_temperature=1.0, sa_iterations=1500, sa_seed=42, stop_points='stops.gpkg', vehicle_capacity=100.0, vehicle_fixed_cost=0.0)`


---

## Vehicle Routing Pickup Delivery

**Function name:** `vehicle_routing_pickup_delivery`


Experimental

Builds paired pickup-delivery routes with precedence and capacity constraints using a deterministic nearest-neighbour baseline.

vector network routing optimization pickup-delivery

### Parameters

NameDescriptionRequiredDefault
`network`Input line network layer (validated for contract parity).Required`network.gpkg`
`depot_points`Depot point layer; first point is used as the active depot in this baseline implementation.Required`depots.gpkg`
`stop_points`Stop point layer containing paired pickup and delivery records.Required`stops.gpkg`
`request_id_field`Request identifier field in stop_points used to pair pickup and delivery records (default: request_id).Optional`request_id`
`stop_type_field`Stop type field in stop_points containing pickup/delivery labels (default: stop_type).Optional`stop_type`
`demand_field`Numeric demand field in stop_points; pickup demand is loaded and delivered demand is ignored (default: demand).Optional`demand`
`vehicle_capacity`Per-vehicle capacity (> 0).Required`100.0`
`max_vehicles`Optional maximum number of vehicles/routes to construct.Optional—
`output`Output route line vector path.Required—
`assignment_output`Optional stop assignment point output with request and precedence diagnostics.Optional—

### Examples

*Builds baseline pickup-delivery routes and writes route lines.*
`wbe.vehicle_routing_pickup_delivery(demand_field='demand', depot_points='depots.gpkg', network='network.gpkg', output='pickup_delivery_routes.gpkg', request_id_field='request_id', stop_points='stops.gpkg', stop_type_field='stop_type', vehicle_capacity=100.0)`


---

## Vehicle Routing VRPTW

**Function name:** `vehicle_routing_vrptw`


Experimental

Builds capacity-constrained multi-depot VRPTW routes with heterogeneous fleet settings, break windows, and objective-mode controls.

vector network routing optimization time-window

### Parameters

NameDescriptionRequiredDefault
`network`Input line network layer (validated for contract parity).Required`network.gpkg`
`depot_points`Depot point layer; each point can contribute one or more vehicles.Required`depots.gpkg`
`stop_points`Delivery stop point layer with demand and time-window fields.Required`stops.gpkg`
`demand_field`Numeric demand field in stop_points (default: demand).Optional`demand`
`priority_field`Optional stop priority field using values like required/high/normal/low or numeric ranks.Optional`priority`
`allowed_vehicle_profiles_field`Optional stop field listing compatible vehicle profiles (comma/semicolon/pipe-delimited).Optional—
`allowed_route_classes_field`Optional alias of allowed_vehicle_profiles_field for route-class compatibility rules.Optional—
`tw_start_field`Numeric time-window start field in stop_points (default: tw_start).Optional`tw_start`
`tw_end_field`Numeric time-window end field in stop_points (default: tw_end).Optional`tw_end`
`service_time_field`Numeric per-stop service time field in stop_points (default: service_time).Optional`service_time`
`depot_id_field`Optional depot ID field used in route/assignment outputs.Optional—
`vehicle_count_field`Optional depot field for number of vehicles spawned at each depot.Optional—
`vehicle_capacity_field`Optional depot field overriding vehicle_capacity per depot/vehicle template.Optional—
`vehicle_fixed_cost_field`Optional depot field overriding vehicle_fixed_cost per depot/vehicle template.Optional—
`travel_speed_field`Optional depot field overriding travel_speed per depot/vehicle template.Optional—
`max_route_distance_field`Optional depot field overriding max_route_distance per depot/vehicle template.Optional—
`max_route_time_field`Optional depot field overriding max_route_time per depot/vehicle template.Optional—
`vehicle_profile_field`Optional depot field defining vehicle profile/category token used for stop compatibility.Optional—
`vehicle_route_class_field`Optional alias of vehicle_profile_field for route-class compatibility rules.Optional—
`depot_close_time_field`Optional depot field overriding depot_close_time per depot/vehicle template.Optional—
`break_start_field`Optional depot field overriding break_start_time per depot/vehicle template.Optional—
`break_end_field`Optional depot field overriding break_end_time per depot/vehicle template.Optional—
`break_duration_field`Optional depot field overriding break_duration per depot/vehicle template.Optional—
`vehicle_capacity`Per-vehicle capacity (> 0).Required`100.0`
`vehicle_fixed_cost`Optional fixed cost charged per dispatched vehicle/route (default: 0).Optional`0.0`
`start_time`Route start time in model time units (default: 0).Optional`0.0`
`travel_speed`Travel speed in coordinate-units per time unit (default: 1).Optional`1.0`
`enforce_time_windows`When true, only stops with lateness Optional`False`
`allowed_lateness`Maximum lateness tolerated when enforce_time_windows=true (default: 0).Optional`0.0`
`depot_close_time`Optional hard close time by which each route must return to depot.Optional—
`break_start_time`Optional global break-window start time for all vehicles.Optional—
`break_end_time`Optional global break-window end time for all vehicles.Optional—
`break_duration`Optional global break duration applied once per route when break window is intersected.Optional—
`use_priority_scoring`When true, ranks feasible candidates by projected lateness/slack before travel distance; when false, uses nearest-neighbour baseline (default: true).Optional`True`
`max_vehicles`Optional maximum number of vehicles/routes to construct.Optional—
`max_route_distance`Optional maximum route travel distance, including return to depot.Optional—
`max_route_time`Optional maximum route duration in model time units, including return to depot.Optional—
`max_stops_per_vehicle`Optional maximum number of stops assigned to each vehicle route.Optional—
`objective_mode`Route-construction objective: minimize_lateness, minimize_distance, minimize_vehicles, or minimize_cost.Optional`minimize_lateness`
`output`Output route line vector path.Required—
`assignment_output`Optional stop assignment point output with time-window diagnostics.Optional—

### Examples

*Builds baseline VRPTW routes and reports time-window diagnostics.*
`wbe.vehicle_routing_vrptw(allowed_lateness=0.0, demand_field='demand', depot_points='depots.gpkg', enforce_time_windows=False, network='network.gpkg', objective_mode='minimize_lateness', output='vrptw_routes.gpkg', priority_field='priority', service_time_field='service_time', start_time=0.0, stop_points='stops.gpkg', travel_speed=1.0, tw_end_field='tw_end', tw_start_field='tw_start', use_priority_scoring=True, vehicle_capacity=100.0, vehicle_fixed_cost=0.0)`
