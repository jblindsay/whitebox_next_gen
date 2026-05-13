# Network Analysis

Network analysis in WbW-QGIS spans both transportation and hydrologic
networks. This chapter is aligned with the Python and R manuals and now covers
three common tracks:

- Transportation routing and service areas
- OD and nearest-facility analysis
- Stream-network hierarchy and connectivity

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

Use this prepared layer as the routing network for native QGIS algorithms.

---

## Workflow B: Routing, Service Areas, and Closest Facility

### Step 3 - Shortest Path

Processing Toolbox -> Network Analysis:

- Shortest Path (Point to Point)
- Shortest Path (Layer to Point)
- Shortest Path (Point to Layer)

Use the prepared cost field (distance or time) consistently.

---

### Step 4 - Service Area (Isochrone)

Processing Toolbox -> Network Analysis -> Service Area (From Layer)

Recommended parameters:

| Parameter | Example |
|-----------|---------|
| Network layer | roads_prepared.shp |
| Strategy | Shortest |
| Start points | facilities.shp |
| Travel cost | 5.0 (minutes) or 3000 (meters) |

Export output lines and optional polygons for reporting.

---

### Step 5 - Closest Facility Pattern

Use Service Area and shortest-path tools together:

- Build candidate facilities
- Route demand points to nearest reachable facilities
- Summarize cost by facility catchment

For large batches, run model-builder or Python Console loops.

---

## Workflow C: OD-Style Batch Analysis in QGIS

QGIS does not provide a single OD matrix tool equivalent to the Python/R
chapters, so the standard QGIS pattern is:

- Iterate origins and destinations in batch
- Run shortest path for each pair
- Aggregate travel cost in an output table

Use this when you need accessibility summaries or assignment baselines directly
inside QGIS projects.

---

## Workflow D: Hydrologic Stream Networks

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

# Service area from facilities
processing.run('native:serviceareafromlayer', {
    'INPUT': '/data/roads_prepared.shp',
    'STRATEGY': 0,
    'START_POINTS': '/data/facilities.shp',
    'TRAVEL_COST': 5.0,
    'OUTPUT_LINES': '/data/service_area_lines.shp',
    'OUTPUT': 'TEMPORARY_OUTPUT',
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
