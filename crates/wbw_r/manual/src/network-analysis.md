# Network Analysis

Whitebox Next Gen R brings the same deep network-analysis engine as the Python
API: topology auditing, point-to-point routing, service areas, closest
facility, OD cost matrices, location-allocation, accessibility metrics,
sensitivity analysis, multimodal transit modelling, map matching, and fleet
dispatch optimization. This chapter walks through those capabilities in order,
mirroring the prepare-then-query-then-scale-up workflow used in practice.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/network')
```

---

## Core Concepts You Should Know First

Before running tools, it helps to align on a few core terms used throughout
this chapter.

- **Network**: A graph made of edges (road or transit segments) and nodes
  (intersections, stops, junctions).
- **Cost / impedance**: The value minimized during routing. This can be
  distance, travel time, generalized cost, or another friction metric.
- **Origin / destination (OD)**: Origins are trip start points; destinations
  are trip end points.
- **OD matrix**: A table of costs from many origins to many destinations. This
  is the standard structure for accessibility, market access, and assignment
  analyses.
- **Shortest path**: The minimum-cost path between one origin and one
  destination.
- **K-shortest paths**: The best *k* distinct alternatives between the same OD
  pair, useful for resilience and choice modelling.
- **Service area (isochrone)**: The portion of the network reachable from an
  origin within a cost threshold (for example 10 minutes).
- **Closest facility**: For each incident or demand point, the least-cost
  route to the nearest candidate facility on the network.
- **Location-allocation**: Selecting facility sites that optimize a demand
  objective, such as minimizing total travel cost or maximizing coverage.
- **Connectivity**: Whether all required origins and destinations are in the
  same connected component. Disconnected components cause failed routes.
- **Node degree**: The number of edges touching a node. Degree supports basic
  network centrality interpretation and QA for odd junction structure.
- **Multimodal routing**: Pathfinding across multiple transport modes
  (walk/bus/rail) with transfer penalties and mode constraints.
- **Map matching**: Snapping GPS trajectories to the most plausible sequence of
  network edges.

If you keep these definitions in mind, each workflow step below becomes easier
to interpret and validate.

---

## Step 1 — Prepare and Audit the Network

Every routing workflow should begin with a topology check.

### Topology Audit

`network_topology_audit` scans a line network for dead ends, pseudo-nodes,
overshoots, and isolated islands, writing each flagged location as a point
feature and optionally producing a text report.

```r
wbw_run_tool('network_topology_audit', args = list(
  i             = 'roads.shp',
  output        = 'topology_errors.shp',
  snap_tolerance = 0.5,
  one_way_field  = 'ONEWAY',
  report         = 'topology_report.txt'
), session = s)
```

Review `topology_report.txt` to understand the error count and class
distribution before continuing.

### Connected Components

`network_connected_components` assigns a component identifier to every edge so
you can identify and resolve disconnected islands before running OD queries.

```r
wbw_run_tool('network_connected_components', args = list(
  i              = 'roads.shp',
  output         = 'roads_components.shp',
  snap_tolerance = 0.5
), session = s)
# Edges not in the dominant component are candidates for removal or bridging.
```

### Node Degree

`network_node_degree` writes the degree of every node as a point layer.
Degree-1 nodes are dead ends; unusually high-degree nodes may be duplicates.

```r
wbw_run_tool('network_node_degree', args = list(
  i              = 'roads.shp',
  output         = 'node_degree.shp',
  snap_tolerance = 0.5
), session = s)
```

---

## Step 2 — Shortest Path and Alternatives

### Single Shortest Path

`shortest_path_network` finds the minimum-cost path between two coordinates
using Dijkstra's algorithm. Supply `edge_cost_field` for travel-time routing;
omit it to route by Euclidean arc length.

```r
wbw_run_tool('shortest_path_network', args = list(
  i               = 'roads.shp',
  output          = 'route_shortest.shp',
  start_x         = 454230.0,
  start_y         = 4823150.0,
  end_x           = 458900.0,
  end_y           = 4819700.0,
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY'
), session = s)
```

Turn penalties model the real cost of left, right, and U-turns in dense urban
networks.

```r
wbw_run_tool('shortest_path_network', args = list(
  i               = 'roads.shp',
  output          = 'route_with_turns.shp',
  start_x         = 454230.0,
  start_y         = 4823150.0,
  end_x           = 458900.0,
  end_y           = 4819700.0,
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY',
  turn_penalty    = 0.5,
  u_turn_penalty  = 3.0,
  forbid_u_turns  = TRUE
), session = s)
```

### K-Shortest Alternative Paths

`k_shortest_paths_network` returns the *k* least-cost distinct paths for the
same endpoints — useful for resilience analysis and presenting alternatives to
planners.

```r
wbw_run_tool('k_shortest_paths_network', args = list(
  i               = 'roads.shp',
  output          = 'routes_k3.shp',
  start_x         = 454230.0,
  start_y         = 4823150.0,
  end_x           = 458900.0,
  end_y           = 4819700.0,
  k               = 3L,
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY'
), session = s)
# Each feature carries a PATH_RANK attribute (1 = shortest).
```

---

## Step 3 — Service Areas

`network_service_area` delineates every part of the network reachable within a
cost threshold from one or more origins. Typical uses include drive-time
catchments for emergency services, walking isochrones around transit stops, and
delivery zones.

```r
wbw_run_tool('network_service_area', args = list(
  i                    = 'roads.shp',
  origins              = 'fire_stations.shp',
  output               = 'fire_catchment_5min.shp',
  max_cost             = 5.0,
  snap_tolerance       = 20.0,
  output_mode          = 'polygon',
  polygon_merge_origins = TRUE,
  edge_cost_field      = 'MINUTES',
  one_way_field        = 'ONEWAY'
), session = s)
```

Use `output_mode = 'edges'` to retain actual road arcs inside the catchment
rather than fill a polygon — more appropriate when the network is sparse.

---

## Step 4 — Closest Facility

`closest_facility_network` routes each incident point to its nearest facility,
measuring cost along the network rather than in straight-line distance. This
is the core tool for emergency-response siting, healthcare access studies, and
school-catchment delineation.

```r
wbw_run_tool('closest_facility_network', args = list(
  i               = 'roads.shp',
  incidents       = 'accidents.shp',
  facilities      = 'hospitals.shp',
  output          = 'routes_to_hospital.shp',
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY'
), session = s)
# Output carries INCIDENT_FID, FACILITY_FID, and COST fields per route.
```

---

## Step 5 — OD Cost Matrix and Batch Route Export

### OD Cost Matrix

`network_od_cost_matrix` solves all pairwise paths and writes results to a CSV.
Each row contains an origin identifier, a destination identifier, and the
network cost between them.

```r
wbw_run_tool('network_od_cost_matrix', args = list(
  i               = 'roads.shp',
  origins         = 'schools.shp',
  destinations    = 'libraries.shp',
  output          = 'od_costs.csv',
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY'
), session = s)

od <- read.csv('od_costs.csv')
# Minimum travel time from each school to any library:
print(aggregate(COST ~ ORIGIN_FID, data = od, FUN = min))
```

### Materializing OD Routes as Geometry

`network_routes_from_od` creates the actual path lines between OD pairs.

```r
wbw_run_tool('network_routes_from_od', args = list(
  i               = 'roads.shp',
  origins         = 'schools.shp',
  destinations    = 'libraries.shp',
  output          = 'od_routes.shp',
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES',
  one_way_field   = 'ONEWAY'
), session = s)
```

---

## Step 6 — Location-Allocation

`location_allocation_network` solves the p-median problem: given candidate
facility locations and weighted demand points, which *p* facilities minimise
total travel cost? Directly supports clinic siting, school consolidation, and
warehouse network design.

```r
wbw_run_tool('location_allocation_network', args = list(
  i                    = 'roads.shp',
  demand_points        = 'demand_points.shp',
  facilities           = 'candidate_sites.shp',
  output               = 'selected_facilities.shp',
  facility_count       = 4L,
  solver_mode          = 'minimize_impedance',
  demand_weight_field  = 'POP',
  snap_tolerance       = 20.0,
  edge_cost_field      = 'MINUTES'
), session = s)
# SELECTED == 1 on the four chosen candidate sites.
# Demand points carry ASSIGNED_FID linking each to its nearest selected site.
```

Supported solver modes: `minimize_impedance` (p-median), `maximize_coverage`,
and `maximize_attendance`.

---

## Step 7 — Network Accessibility Metrics

`compute_network_accessibility()` is exposed as a typed facade wrapper. It
computes a gravity-model or cumulative-opportunity accessibility score per
origin — a standard indicator in transport equity analysis.

```r
residents    <- wbw_read_vector('resident_centroids.shp', session = s)
supermarkets <- wbw_read_vector('supermarkets.shp', session = s)

result <- s$compute_network_accessibility(
  input              = 'roads.shp',
  origins            = residents$file_path(),
  destinations       = supermarkets$file_path(),
  output             = 'food_accessibility.shp',
  edge_cost_field    = 'MINUTES',
  impedance_cutoff   = 30.0,
  decay_function     = 'negative_exponential',
  decay_parameter    = 0.1
)
# Each origin point carries an ACCESS_SCORE field.
```

---

## Step 8 — OD Sensitivity Analysis

`analyze_od_cost_sensitivity()` quantifies how stable OD costs are under
stochastic perturbation of edge weights — useful for stress-testing a routing
model against travel-time uncertainty or hypothetical road-closure scenarios.

```r
result <- s$analyze_od_cost_sensitivity(
  input                       = 'roads.shp',
  origins                     = 'schools.shp',
  destinations                = 'libraries.shp',
  output                      = 'od_sensitivity.shp',
  edge_cost_field             = 'MINUTES',
  impedance_disturbance_range = 0.2,  # ±20 % perturbation
  monte_carlo_samples         = 500L
)
```

---

## Step 9 — Multimodal Analysis

Whitebox Next Gen supports networks with a `MODE` field on each edge (e.g.
`walk`, `cycle`, `bus`, `rail`). Multimodal tools honour mode-specific speeds,
transfer penalties, and time-of-day profiles.

### Multimodal OD Scenarios

`analyze_multimodal_od_scenarios()` runs a batch of named scenarios from a CSV,
each with different mode allowances, speed overrides, or departure times.

```r
transit_net  <- wbw_read_vector('transit_network.shp', session = s)
bus_stops    <- wbw_read_vector('bus_stops.shp', session = s)
destinations <- wbw_read_vector('key_destinations.shp', session = s)

result <- s$analyze_multimodal_od_scenarios(
  input               = transit_net$file_path(),
  origins             = bus_stops$file_path(),
  destinations        = destinations$file_path(),
  output              = 'multimodal_od_scenarios.shp',
  mode_field          = 'MODE',
  allowed_modes       = 'walk,bus,rail',
  transfer_penalty    = 3.0,
  edge_cost_field     = 'MINUTES',
  scenario_bundle_csv = 'scenarios.csv',
  departure_time      = '08:00',
  temporal_mode       = 'scheduled'
)
```

### Exporting Multimodal Route Geometry

`export_multimodal_routes_for_od_pairs()` materializes the optimal multimodal
route for each OD pair with per-segment mode attributes.

```r
result <- s$export_multimodal_routes_for_od_pairs(
  input            = transit_net$file_path(),
  origins          = bus_stops$file_path(),
  destinations     = destinations$file_path(),
  output           = 'multimodal_routes.shp',
  mode_field       = 'MODE',
  allowed_modes    = 'walk,bus,rail',
  transfer_penalty = 3.0,
  edge_cost_field  = 'MINUTES'
)
```

---

## Step 10 — Map Matching

`map_matching_v1` snaps a raw GPS trajectory to the most plausible sequence of
network edges using a hidden Markov model with candidate expansion. It is the
first step in any floating-vehicle data or probe-data workflow.

```r
wbw_run_tool('map_matching_v1', args = list(
  i                     = 'roads.shp',
  trajectory_points     = 'gps_probe_points.shp',
  timestamp_field       = 'TIMESTAMP',
  output                = 'matched_route.shp',
  search_radius         = 30.0,
  candidate_k           = 5L,
  snap_tolerance        = 10.0,
  edge_cost_field       = 'MINUTES',
  matched_points_output = 'matched_points.shp',
  match_report          = 'match_report.txt'
), session = s)
```

---

## Step 11 — Fleet and Vehicle Routing *(Pro)*

`fleet_routing_and_dispatch_optimizer` solves CVRP and VRPTW problems: given
a fleet of vehicles and a set of service or delivery stops, it assigns and
sequences routes to minimise total cost subject to capacity and time-window
constraints.

```r
result <- s$run_tool(
  'fleet_routing_and_dispatch_optimizer',
  list(
    network               = 'roads.shp',
    depots                = 'depots.shp',
    stops                 = 'delivery_stops.shp',
    vehicles_csv          = 'fleet.csv',
    route_output          = 'fleet_routes.shp',
    route_kpis_csv_output = 'fleet_kpis.csv',
    edge_cost_field       = 'MINUTES',
    one_way_field         = 'ONEWAY',
    vrp_mode              = 'VRPTW'
  )
)

kpis <- read.csv('fleet_kpis.csv')
print(kpis)
```

> **Note:** This tool requires a session initialised with a valid Pro licence.

---

## Complete Workflow: Emergency Response Planning

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/emergency_planning')

# 1. Audit topology.
wbw_run_tool('network_topology_audit', args = list(
  i = 'roads.shp', output = 'topology_errors.shp',
  snap_tolerance = 0.5, report = 'topology_report.txt'
), session = s)

# 2. Five-minute drive catchments from existing stations.
wbw_run_tool('network_service_area', args = list(
  i                    = 'roads.shp',
  origins              = 'fire_stations.shp',
  output               = 'existing_catchment_5min.shp',
  max_cost             = 5.0,
  output_mode          = 'polygon',
  polygon_merge_origins = TRUE,
  edge_cost_field      = 'MINUTES',
  snap_tolerance       = 20.0
), session = s)

# 3. Route historical incidents to nearest existing station.
wbw_run_tool('closest_facility_network', args = list(
  i               = 'roads.shp',
  incidents       = 'historical_incidents.shp',
  facilities      = 'fire_stations.shp',
  output          = 'incident_routes.shp',
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES'
), session = s)

# 4. Find two new station sites that maximise coverage.
wbw_run_tool('location_allocation_network', args = list(
  i               = 'roads.shp',
  demand_points   = 'historical_incidents.shp',
  facilities      = 'candidate_stations.shp',
  output          = 'new_station_sites.shp',
  facility_count  = 2L,
  solver_mode     = 'maximize_coverage',
  snap_tolerance  = 20.0,
  edge_cost_field = 'MINUTES'
), session = s)
```

---

## Tips

- Always run `network_topology_audit` first — even one disconnected segment
  can cause a path query to return no result without an explicit error.
- Use `network_connected_components` to confirm all origins and destinations
  belong to the same component before running OD queries.
- Supply `edge_cost_field` pointing to a travel-time column for realistic
  routing; omit it only for pure geometric distance problems.
- For scheduled transit, pass `temporal_cost_profile` and `departure_time`
  to load time-of-day speeds.
- The `fleet_routing_and_dispatch_optimizer` Pro tool requires a session
  initialised with a valid Pro licence.
