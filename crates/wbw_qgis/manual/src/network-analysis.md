# Network Analysis

Network analysis in WbW-QGIS spans both transportation and hydrologic
networks. This chapter is aligned with the Python and R manuals and now covers
three common tracks:

- Transportation routing and service areas
- OD and nearest-facility analysis
- Stream-network hierarchy and connectivity

### Capability Note (Open Tier)

Whitebox open tier provides advanced network tools directly in the QGIS plugin,
including shortest path, k-shortest alternatives, service areas, OD matrices,
closest facility, location-allocation, and multimodal OD/routes. Advanced
impedance controls include one-way directionality, turn/u-turn penalties,
optional node-entry costs, and optional temporal cost profiles.

---

## Core Concepts You Should Know First

- Network: A graph of edges (line segments) and nodes (junctions/endpoints).
- Cost or impedance: Value minimized by routing (distance, minutes, or other
  weighted friction).
- OD pair: Origin and destination used in path queries.
- Service area: All network locations reachable under a cost budget.
- Closest facility: Nearest destination by network cost, not straight-line
  distance.
- Connectivity: Whether all required features are in connected components.
- Directed network: Edge direction matters (one-way roads, downstream streams).

---

## Typical Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| roads.shp | Polyline vector | Cleaned road centerlines |
| facilities.shp | Point vector | Hospitals, depots, schools, etc. |
| demand_points.shp | Point vector | Incidents, customers, or population centroids |
| streams.tif | Raster | Binary stream raster for hydrologic hierarchy |
| d8_pointer.tif | Raster | D8 flow-direction raster |

---

## Workflow A: Transportation Network Preparation

### Step 1 - Topology QA and Geometry Cleanup

Use standard QGIS cleanup first:

- Check validity
- Snap Geometries to Layer
- Fix Geometries

Then enrich network attributes with Whitebox tools:

Processing Toolbox -> Whitebox Workflows -> Vector Analysis ->
Add Geometry Attributes

This provides segment length fields needed for distance-based routing.

If travel-time routing is required, compute a time field such as:

- TIME_MIN = LENGTH_M / SPEED_M_PER_MIN

using Field Calculator.

---

### Step 2 - Build Cost-Aware Road Layer

Recommended fields:

- LENGTH_M (meters)
- SPEED_KMH (if available)
- TIME_MIN (derived)
- ONEWAY (optional directional control)

Use this prepared layer as the routing network for Whitebox network-analysis
tools in the Processing Toolbox.

---

### Step 2.5 - Build Network Topology and Snap Points (Optional)

If your network lacks proper node structure or you need to snap facility/demand points to the network:

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Build Network Topology`**

| Parameter | Value |
|-----------|-------|
| Input vector | roads_prepared.shp |
| Snap tolerance | 0.5 |
| Output | roads_noded.shp |
| Output nodes | network_nodes.shp |

Then snap your facilities and demand points:

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Snap Points to Network`**

| Parameter | Value |
|-----------|-------|
| Network layer | roads_noded.shp |
| Points layer | fire_stations.shp |
| Snap distance | 50.0 (meters) |
| Output | fire_stations_snapped.shp |

Output includes `SNAP_DIST` (offset to network) for diagnostics.

---

## Workflow B: Routing, Service Areas, and Closest Facility

### Intersection Delay / Node Cost Modeling

For Whitebox network tools that support advanced impedance (for example
service-area and closest-facility workflows), you can include node-entry costs
to model intersection delay:

- `node_cost_points`: point layer of intersection/gate delay observations.
- `node_cost_field`: numeric field in `node_cost_points` with non-negative
  delay/cost values.
- `node_cost_snap_distance`: optional max assignment distance from each
  node-cost point to a network node.

Practical pattern:

1. Build/snap a clean network first.
2. Prepare an intersection delay points layer (signals, crossings, gates).
3. Run network tools with both edge impedance and node-cost parameters.

When node-cost parameters are omitted, routing uses edge impedance only.

### Step 3 - Shortest Path and K-Shortest Alternatives

Processing Toolbox -> Whitebox Workflows -> Network Analysis:

- `Shortest Path Network`
- `K Shortest Paths Network`

Recommended parameters:

| Parameter | Example |
|-----------|---------|
| Input network | roads_prepared.shp |
| Start / End | route endpoints or point features |
| Edge cost field | TIME_MIN |
| One-way field | ONEWAY |
| Turn penalty | 0.3 to 0.8 (minutes) |
| U-turn penalty | 2.0 to 4.0 (minutes) |

Use k-shortest outputs when teaching resilience, alternate routing, or
choice-model concepts.

---

### Step 4 - Service Area (Isochrone)

Processing Toolbox -> Whitebox Workflows -> Network Analysis ->
`Network Service Area`

Recommended parameters:

| Parameter | Example |
|-----------|---------|
| Network layer | roads_prepared.shp |
| Origins | facilities.shp |
| Max cost | 5.0 (minutes) or 3000 (meters) |
| Output mode | polygon or edges |
| Polygon merge origins | true/false |
| Edge cost field | TIME_MIN |
| One-way field | ONEWAY |

Advanced options (recommended for realistic urban travel times):

- `node_cost_points`, `node_cost_field`, `node_cost_snap_distance`
- `turn_penalty`, `u_turn_penalty`, `forbid_u_turns`
- `temporal_cost_profile`, `departure_time`, `temporal_mode`

---

### Step 5 - Closest Facility and OD Matrices

Processing Toolbox -> Whitebox Workflows -> Network Analysis:

- `Closest Facility Network`
- `Network OD Cost Matrix`
- `Network Routes From OD`

Use these for assignment, accessibility summaries, and route materialization.
OD matrix output is ideal for downstream tabular analysis (Python/R/pandas).

---

## Workflow C: Location-Allocation and Accessibility

Processing Toolbox -> Whitebox Workflows -> Network Analysis:

- `Location Allocation Network`
- `Compute Network Accessibility`

Recommended location-allocation pattern:

1. Prepare candidate sites and weighted demand points.
2. Select solver mode (`minimize_impedance`, `maximize_coverage`, or
  `maximize_attendance`).
3. Compare static-cost and peak-period temporal-profile runs.

Recommended accessibility pattern:

1. Provide origins and destination opportunities.
2. Set impedance cutoff and decay function.
3. Map/compare resulting accessibility scores across scenarios.

## Workflow D: Multimodal Network Analysis

Processing Toolbox -> Whitebox Workflows -> Network Analysis:

- `Multimodal Shortest Path`
- `Multimodal OD Cost Matrix`
- `Multimodal Routes From OD`

Requirements:

- mode field on network edges (e.g., walk/bus/rail)
- allowed-modes and transfer-penalty configuration
- optional temporal profile for schedule/peak scenarios

---

## Workflow E: Hydrologic Stream Networks

Hydrologic network tools remain an important part of network analysis and are
included here as a dedicated sub-workflow rather than the entire chapter.

### Step 6 - Stream Hierarchy

Processing Toolbox -> Whitebox Workflows -> Spatial Hydrology:

- Strahler Stream Order
- Shreve Stream Magnitude
- Hack Stream Order

These tools characterize stream position and downstream accumulation.

### Step 7 - Stream Vectorization

Processing Toolbox -> Whitebox Workflows -> Spatial Hydrology ->
Raster Streams to Vector

Convert ordered stream rasters to vector lines for cartography and further
network operations.

---

## QGIS Python Console Equivalent

```python
import processing

# Add geometry attributes for road cost preparation
processing.run('whitebox_workflows:add_geometry_attributes', {
    'input': '/data/roads.shp',
    'output': '/data/roads_prepared.shp',
})

# Whitebox network service area
processing.run('whitebox_workflows:network_service_area', {
  'input': '/data/roads_prepared.shp',
  'origins': '/data/facilities.shp',
  'max_cost': 5.0,
  'output_mode': 'polygon',
  'edge_cost_field': 'TIME_MIN',
  'one_way_field': 'ONEWAY',
  'turn_penalty': 0.4,
  'u_turn_penalty': 2.5,
  'output': '/data/service_area_5min.shp',
})

# Whitebox OD matrix
processing.run('whitebox_workflows:network_od_cost_matrix', {
  'input': '/data/roads_prepared.shp',
  'origins': '/data/origins.shp',
  'destinations': '/data/destinations.shp',
  'edge_cost_field': 'TIME_MIN',
  'one_way_field': 'ONEWAY',
  'output': '/data/od_costs.csv',
})

# Stream order
processing.run('whitebox_workflows:strahler_stream_order', {
    'd8_pntr': '/data/d8_pointer.tif',
    'streams': '/data/streams.tif',
    'output': '/data/strahler.tif',
})
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| No route found between known-connected points | Topology gaps or unsnapped endpoints | Run snapping and revalidate connectivity |
| Service area too small or too large | Cost units inconsistent | Keep all costs in either meters or minutes |
| One-way streets ignored | Direction field not configured | Verify direction settings in network algorithm |
| Batch routing is slow | Unnecessary repeated reprojection or heavy geometry | Preprocess to common CRS and simplify where appropriate |
| Stream order appears uniform | Bad stream threshold or mismatched d8/stream rasters | Rebuild streams and ensure matching extent/grid |

---

## Validation Checklist

- [ ] Routing network passes geometry validity and snapping checks.
- [ ] Cost field units are consistent across all analyses.
- [ ] Directionality assumptions are documented (directed vs undirected).
- [ ] Service-area outputs were spot-checked against known travel behavior.
- [ ] Stream-order outputs were checked at confluences.
- [ ] Workflow parameters were saved in model or processing history.
