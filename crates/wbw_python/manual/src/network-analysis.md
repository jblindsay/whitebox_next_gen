# Network Analysis

Whitebox Next Gen has deep capabilities across the full network-analysis
spectrum: topology auditing, point-to-point routing, service areas, closest
facility, OD cost matrices, location-allocation, accessibility metrics,
sensitivity analysis, multimodal transit modelling, map matching, and fleet
dispatch optimization. This chapter walks through those capabilities in the
order you would encounter them in a real project.

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

Every routing workflow should begin with a topology check. A single dangling
endpoint or disconnected island can silently invalidate a shortest-path result.

### Topology Audit

`network_topology_audit()` scans a line network for common errors — dead ends,
pseudo-nodes, overshoots, and isolated islands — and writes each flagged
location as a point feature. It also produces an optional text report.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/network'
wbe.verbose = True

roads = wbe.read_vector('roads.shp')

errors, report_path = wbe.network_topology_audit(
    roads,
    snap_tolerance=0.5,
    one_way_field='ONEWAY',
    report='topology_report.txt'
)
wbe.write_vector(errors, 'topology_errors.shp')
```

Review `topology_report.txt` before continuing. Understanding the error count
and class distribution will guide how much cleaning the network needs.

### Connected Components

An isolated cluster of road segments that cannot reach the main network will
cause any OD or closest-facility query to fail for origins or destinations on
that cluster. `network_connected_components()` labels every edge with its
component identifier.

```python
roads_comps = wbe.network_connected_components(roads, snap_tolerance=0.5)
wbe.write_vector(roads_comps, 'roads_components.shp')
# Edges not in the dominant component are candidates for removal or bridging.
```

### Node Degree

`network_node_degree()` writes the degree (number of connected edges) of every
node as a point layer. Degree-1 nodes are dead ends; unusually high-degree
nodes may indicate duplicate arcs.

```python
nodes = wbe.network_node_degree(roads, snap_tolerance=0.5)
wbe.write_vector(nodes, 'node_degree.shp')
```

---

## Step 2 — Shortest Path and Alternatives

### Single Shortest Path

`shortest_path_network()` finds the minimum-cost path between two coordinates
using Dijkstra's algorithm. Supply an `edge_cost_field` to use travel-time or
impedance; omit it to route by Euclidean arc length.

```python
path = wbe.shortest_path_network(
    roads,
    start_x=454230.0, start_y=4823150.0,
    end_x=458900.0,   end_y=4819700.0,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
wbe.write_vector(path, 'route_shortest.shp')
```

Turn penalties model the real-world cost of left, right, and U-turns — these
can substantially alter optimal routes in dense urban networks.

```python
path_turns = wbe.shortest_path_network(
    roads,
    start_x=454230.0, start_y=4823150.0,
    end_x=458900.0,   end_y=4819700.0,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY',
    turn_penalty=0.5,
    u_turn_penalty=3.0,
    forbid_u_turns=True
)
wbe.write_vector(path_turns, 'route_with_turns.shp')
```

### K-Shortest Alternative Paths

`k_shortest_paths_network()` returns the *k* least-cost distinct paths between
the same endpoints. Use this for resilience analysis, route-choice modelling,
or presenting alternatives to planners.

```python
alt_paths = wbe.k_shortest_paths_network(
    roads,
    start_x=454230.0, start_y=4823150.0,
    end_x=458900.0,   end_y=4819700.0,
    k=3,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
wbe.write_vector(alt_paths, 'routes_k3.shp')
# Each feature carries a PATH_RANK attribute (1 = shortest).
```

---

## Step 3 — Service Areas

`network_service_area()` delineates every part of the network reachable within
a cost threshold from one or more origins. Typical uses include drive-time
catchments for emergency services, walking isochrones for transit stops, and
delivery zones.

```python
fire_stations = wbe.read_vector('fire_stations.shp')

catchment = wbe.network_service_area(
    roads,
    origins=fire_stations,
    max_cost=5.0,               # 5 minutes
    snap_tolerance=20.0,
    output_mode='polygon',      # 'nodes', 'edges', or 'polygon'
    polygon_merge_origins=True, # dissolve overlapping catchments
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
wbe.write_vector(catchment, 'fire_catchment_5min.shp')
```

Use `output_mode='edges'` to retain the actual road arcs inside the catchment
rather than fill a polygon — more appropriate when the network is sparse or
when the arc-level result is needed for reporting.

---

## Step 4 — Closest Facility

`closest_facility_network()` routes each incident point to its nearest
facility, measuring cost along the network rather than in straight-line
distance. This is the core tool for emergency-response siting, healthcare
access studies, and school-catchment delineation.

```python
accidents   = wbe.read_vector('accidents.shp')
hospitals   = wbe.read_vector('hospitals.shp')

routes_to_hosp = wbe.closest_facility_network(
    roads,
    incidents=accidents,
    facilities=hospitals,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
wbe.write_vector(routes_to_hosp, 'routes_to_hospital.shp')
# Output carries INCIDENT_FID, FACILITY_FID, and COST fields per route.
```

---

## Step 5 — OD Cost Matrix and Batch Route Export

When you need costs between many origins and many destinations simultaneously,
an OD matrix is far more efficient than looping over `shortest_path_network()`.

### OD Cost Matrix

`network_od_cost_matrix()` solves all pairwise paths and writes the results to
a CSV. Each row contains an origin identifier, a destination identifier, and
the network cost between them.

```python
schools   = wbe.read_vector('schools.shp')
libraries = wbe.read_vector('libraries.shp')

cost_csv = wbe.network_od_cost_matrix(
    roads,
    origins=schools,
    destinations=libraries,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
print('OD matrix written to:', cost_csv)
```

The CSV is directly usable in pandas or any tabular analysis tool.

```python
import pandas as pd
df = pd.read_csv(cost_csv)
print(df.groupby('ORIGIN_FID')['COST'].min().describe())
```

### Materializing OD Routes as Geometry

To visualize or spatially analyse the actual path lines between OD pairs, use
`network_routes_from_od()`.

```python
od_routes = wbe.network_routes_from_od(
    roads,
    origins=schools,
    destinations=libraries,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES',
    one_way_field='ONEWAY'
)
wbe.write_vector(od_routes, 'od_routes_schools_to_libraries.shp')
```

---

## Step 6 — Location-Allocation

`location_allocation_network()` solves the classic p-median problem: given
candidate facility locations and weighted demand points, which *p* facilities
minimise total travel cost? Use this for clinic siting, school consolidation,
warehouse network design, and similar strategic planning problems.

```python
demand     = wbe.read_vector('demand_points.shp')  # population-weighted
candidates = wbe.read_vector('candidate_sites.shp')

sited = wbe.location_allocation_network(
    roads,
    demand_points=demand,
    facilities=candidates,
    facility_count=4,
    solver_mode='minimize_impedance',
    demand_weight_field='POP',
    snap_tolerance=20.0,
    edge_cost_field='MINUTES'
)
wbe.write_vector(sited, 'selected_facilities.shp')
# SELECTED == 1 on the four chosen candidate sites.
# Demand points carry ASSIGNED_FID linking each to its nearest selected site.
```

Solver modes include `minimize_impedance` (p-median), `maximize_coverage`, and
`maximize_attendance`. Required and forbidden facility flags let you fix certain
sites open or closed before the solver runs.

---

## Step 7 — Network Accessibility Metrics

`compute_network_accessibility()` measures how accessible a set of destinations
is from each origin, applying a decay function that down-weights distant
facilities. The result is a gravity-model or cumulative-opportunity
accessibility score per origin — a standard indicator in transport equity
analysis.

```python
residents    = wbe.read_vector('resident_centroids.shp')
supermarkets = wbe.read_vector('supermarkets.shp')

accessibility = wbe.compute_network_accessibility(
    roads,
    origins=residents,
    destinations=supermarkets,
    edge_cost_field='MINUTES',
    impedance_cutoff=30.0,
    decay_function='negative_exponential',
    decay_parameter=0.1
)
wbe.write_vector(accessibility, 'food_accessibility.shp')
# Each origin point carries an ACCESS_SCORE field.
```

---

## Step 8 — OD Sensitivity Analysis

`analyze_od_cost_sensitivity()` quantifies how stable OD costs are under
stochastic perturbation of edge weights. Use it to stress-test a routing model
against uncertainty in travel-time estimates, or to assess the impact of
hypothetical congestion or road-closure scenarios.

```python
sensitivity = wbe.analyze_od_cost_sensitivity(
    roads,
    origins=schools,
    destinations=libraries,
    edge_cost_field='MINUTES',
    impedance_disturbance_range=0.2,  # ±20 % perturbation
    monte_carlo_samples=500
)
wbe.write_vector(sensitivity, 'od_sensitivity.shp')
```

---

## Step 9 — Multimodal Analysis

Whitebox Next Gen supports networks that carry a `MODE` field on each edge
(e.g. `walk`, `cycle`, `bus`, `rail`). The multimodal tools honour
mode-specific speeds, transfer penalties, and time-of-day profiles.

### Multimodal OD Scenarios

`analyze_multimodal_od_scenarios()` runs a batch of named scenarios defined by
a CSV, each specifying different mode allowances, speed overrides, or departure
times. The output is a combined cost table across all scenarios for rapid
before/after or modal-mix comparisons.

```python
transit_net    = wbe.read_vector('transit_network.shp')
bus_stops      = wbe.read_vector('bus_stops.shp')
destinations   = wbe.read_vector('key_destinations.shp')

result = wbe.analyze_multimodal_od_scenarios(
    input=transit_net,
    origins=bus_stops,
    destinations=destinations,
    output='multimodal_od_scenarios.shp',
    mode_field='MODE',
    allowed_modes='walk,bus,rail',
    transfer_penalty=3.0,
    edge_cost_field='MINUTES',
    scenario_bundle_csv='scenarios.csv',
    departure_time='08:00',
    temporal_mode='scheduled'
)
```

### Exporting Multimodal Route Geometry

`export_multimodal_routes_for_od_pairs()` materializes the optimal multimodal
route for each OD pair as a line feature with per-segment mode attributes.

```python
mm_routes = wbe.export_multimodal_routes_for_od_pairs(
    input=transit_net,
    origins=bus_stops,
    destinations=destinations,
    output='multimodal_routes.shp',
    mode_field='MODE',
    allowed_modes='walk,bus,rail',
    transfer_penalty=3.0,
    edge_cost_field='MINUTES'
)
```

---

## Step 10 — Map Matching

`map_matching_v1()` snaps a raw GPS trajectory to the most plausible sequence
of network edges using a hidden Markov model with candidate expansion. It is
the first step in any floating-vehicle data or probe-data workflow.

```python
gps_points = wbe.read_vector('gps_probe_points.shp')

matched_path, match_report = wbe.map_matching_v1(
    roads,
    trajectory_points=gps_points,
    timestamp_field='TIMESTAMP',
    search_radius=30.0,
    candidate_k=5,
    snap_tolerance=10.0,
    edge_cost_field='MINUTES',
    matched_points_output='matched_points.shp',
    match_report='match_report.txt'
)
wbe.write_vector(matched_path, 'matched_route.shp')
```

The match report summarises per-point confidence scores and the percentage of
trajectory points that were successfully snapped to the network.

---

## Step 11 — Fleet and Vehicle Routing *(Pro)*

`fleet_routing_and_dispatch_optimizer` solves Capacitated Vehicle Routing
Problems (CVRP) and Vehicle Routing with Time Windows (VRPTW): given a fleet
of vehicles at one or more depots and a set of service or delivery stops, it
assigns stops to vehicles and sequences each route to minimise total travel
cost subject to capacity and time-window constraints.

```python
result = wbe.run_tool(
    'fleet_routing_and_dispatch_optimizer',
    {
        'network':               'roads.shp',
        'depots':                'depots.shp',
        'stops':                 'delivery_stops.shp',
        'vehicles_csv':          'fleet.csv',
        'route_output':          'fleet_routes.shp',
        'route_kpis_csv_output': 'fleet_kpis.csv',
        'edge_cost_field':       'MINUTES',
        'one_way_field':         'ONEWAY',
        'vrp_mode':              'VRPTW'
    }
)
print(result)
```

The KPI CSV reports per-route capacity utilization, total distance, time, and
stop count — ready for import into logistics dashboards.

> **Note:** This tool requires a `WbEnvironment` initialised with a valid Pro
> licence.

---

## Complete Workflow: Emergency Response Planning

The following example chains topology audit → service-area catchment →
closest-facility → location-allocation into a single emergency-planning
analysis.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/emergency_planning'

roads      = wbe.read_vector('roads.shp')
stations   = wbe.read_vector('fire_stations.shp')
candidates = wbe.read_vector('candidate_stations.shp')
incidents  = wbe.read_vector('historical_incidents.shp')

# 1. Audit topology before running any queries.
errors, _ = wbe.network_topology_audit(
    roads, snap_tolerance=0.5, report='topology_report.txt'
)
wbe.write_vector(errors, 'topology_errors.shp')

# 2. Map 5-minute drive catchments from existing stations.
catchment = wbe.network_service_area(
    roads,
    origins=stations,
    max_cost=5.0,
    output_mode='polygon',
    polygon_merge_origins=True,
    edge_cost_field='MINUTES',
    snap_tolerance=20.0
)
wbe.write_vector(catchment, 'existing_catchment_5min.shp')

# 3. Route each historical incident to its nearest station.
routes = wbe.closest_facility_network(
    roads,
    incidents=incidents,
    facilities=stations,
    snap_tolerance=20.0,
    edge_cost_field='MINUTES'
)
wbe.write_vector(routes, 'incident_routes.shp')

# 4. Find two additional station locations that maximise coverage.
sited = wbe.location_allocation_network(
    roads,
    demand_points=incidents,
    facilities=candidates,
    facility_count=2,
    solver_mode='maximize_coverage',
    snap_tolerance=20.0,
    edge_cost_field='MINUTES'
)
wbe.write_vector(sited, 'new_station_sites.shp')
```

---

## Tips

- Always run `network_topology_audit()` first — even one disconnected segment
  can cause a path query to return no result without an explicit error.
- Use `network_connected_components()` to confirm that all origins and
  destinations belong to the same component before running OD queries.
- Supply `edge_cost_field` pointing to a pre-computed travel-time field for
  realistic routing; omit it only for pure geometric distance problems.
- For time-sensitive routing, use `temporal_cost_profile` and
  `departure_time` to load scheduled speeds at the time of travel.
- For multimodal networks, store the mode identifier in a field called `MODE`
  and use `allowed_modes` to control which modes are permitted per query.
- The `fleet_routing_and_dispatch_optimizer` Pro tool requires a
  `WbEnvironment` initialised with a valid Pro licence.
