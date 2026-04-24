# Network Analysis

Network analysis models movement and connectivity through a graph of
connected line features. It underpins routing, service-area delineation,
travel-time catchment mapping, and infrastructure connectivity assessment.
WbW-QGIS provides network-preparation tools, hydrologic network ordering
tools, and connectivity metrics that complement the native QGIS network
analysis framework.

This chapter demonstrates a stream network ordering and connectivity
workflow, and a road network service area example.

---

## Key Concepts

- **Network topology**: Connectivity rules for a line network. Edges (lines)
  connect at nodes (endpoints). A topologically clean network has no dangling
  ends, no undershoots, and no overshoots at intended junctions.
- **Directed network**: Edges have a direction (from-node to to-node).
  Hydrologic networks are inherently directed (downstream). Road networks may
  be directed (one-way streets) or undirected.
- **Cost attribute**: A numeric field on each edge representing traversal cost
  — distance, travel time, impedance, or a composite metric.
- **Shortest path**: Minimum-cost route between two nodes. Dijkstra's
  algorithm is the standard solver for non-negative edge weights.
- **Service area / catchment**: Set of all network locations reachable within
  a specified cost budget from an origin node.
- **Strahler / Shreve order**: Integer stream ordering systems that summarise
  network hierarchy from headwaters (order 1) to main channel. Strahler
  increments only at equal-order confluences; Shreve is additive.

---

## End-to-End Workflow: Stream Network Ordering and Connectivity

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `dem_conditioned.tif` | GeoTIFF raster | Hydrologically conditioned DEM |
| `streams.tif` | GeoTIFF raster | Binary stream raster from flow accumulation threshold |
| `d8_pointer.tif` | GeoTIFF raster | D8 flow direction raster |

---

### Step 1 — Strahler Stream Order

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Strahler Stream Order`**

| Parameter | Recommended value |
|-----------|------------------|
| D8 pointer | `d8_pointer.tif` |
| Streams raster | `streams.tif` |
| Output | `strahler.tif` |

Visualise using a sequential colour ramp from light (order 1) to dark (highest
order). Inspect order transitions at confluences to confirm correct topology.

---

### Step 2 — Shreve Stream Magnitude

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Shreve Stream Magnitude`**

| Parameter | Recommended value |
|-----------|------------------|
| D8 pointer | `d8_pointer.tif` |
| Streams raster | `streams.tif` |
| Output | `shreve.tif` |

Shreve magnitude equals the count of first-order tributaries draining to each
stream reach. High values indicate major channels.

---

### Step 3 — Vector Stream Network with Order Attributes

Convert the Strahler raster to a vector network for cartographic display and
downstream routing analysis.

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Raster Streams to Vector`**

| Parameter | Recommended value |
|-----------|------------------|
| Streams raster | `strahler.tif` |
| D8 pointer | `d8_pointer.tif` |
| Output | `streams_vector.shp` |

The output polyline feature class carries the Strahler order value as a
field, enabling symbol classification by order.

---

### Step 4 — Hack Stream Order (Optional)

Hack ordering numbers stream reaches sequentially from outlet to headwaters —
useful for long-profile analysis and channel numbering in reports.

**Processing Toolbox → Whitebox Workflows → Spatial Hydrology →
`Hack Stream Order`**

| Parameter | Recommended value |
|-----------|------------------|
| D8 pointer | `d8_pointer.tif` |
| Streams raster | `streams.tif` |
| Output | `hack.tif` |

---

## Python Console Equivalent

```python
import processing

# Strahler order
processing.run('whitebox_workflows:strahler_stream_order', {
    'd8_pntr': '/data/d8_pointer.tif',
    'streams': '/data/streams.tif',
    'output': '/data/strahler.tif',
})

# Shreve magnitude
processing.run('whitebox_workflows:shreve_stream_magnitude', {
    'd8_pntr': '/data/d8_pointer.tif',
    'streams': '/data/streams.tif',
    'output': '/data/shreve.tif',
})

# Vector stream network
processing.run('whitebox_workflows:raster_streams_to_vector', {
    'streams': '/data/strahler.tif',
    'd8_pntr': '/data/d8_pointer.tif',
    'output': '/data/streams_vector.shp',
})

print("Stream network ordering complete.")
```

---

## Road Network: Service Area in QGIS

For road network routing (shortest path, service areas, turn restrictions),
use the **QGIS Network Analysis** framework, with WbW used to prepare and
enrich the road layer first.

### Step 1 — Prepare Road Network

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Add Geometry Attributes`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `roads.shp` |
| Units | `Metres` |
| Output | `roads_length.shp` |

This appends `LENGTH` (metres) to each road segment, which becomes the cost
attribute for routing.

### Step 2 — Service Area from Origin

**Processing Toolbox → Network Analysis → `Service Area (From Layer)`**
(QGIS native)

| Parameter | Recommended value |
|-----------|------------------|
| Vector layer | `roads_length.shp` |
| Path type | `Shortest` |
| Start points | `facility_points.shp` |
| Travel cost | `300` (metres — or seconds if using speed-adjusted cost) |
| Direction field | *(leave blank for undirected)* |
| Output convex hull | ☐ |
| Output | `service_area.shp` |

```python
processing.run('native:serviceareafromlayer', {
    'INPUT': '/data/roads_length.shp',
    'STRATEGY': 0,  # 0 = Shortest
    'START_POINTS': '/data/facility_points.shp',
    'TRAVEL_COST': 300,
    'OUTPUT_LINES': '/data/service_area_lines.shp',
    'OUTPUT': 'TEMPORARY_OUTPUT',
})
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Strahler order is 1 everywhere | D8 pointer and streams raster have different extents | Clip both to the same catchment extent |
| Vector stream network has gaps | Raster-to-vector conversion missed isolated stream cells | Ensure streams raster is topologically connected (no isolated pixels) |
| Road service area does not extend across a known bridge | Network has a topological gap at the bridge | Run `Snap Geometries to Layer` to close undershoots before analysis |
| Shortest path reports no route found | Origin or destination is not snapped to the network | Snap start/end points to nearest line vertex before routing |
| Stream order jumps unexpectedly | Confluence cell is misassigned in D8 pointer | Verify conditioned DEM and recheck flow direction around problem area |

---

## Validation Checklist

- [ ] Strahler order transitions occur only at confirmed tributary confluences.
- [ ] Vector stream network is a connected graph with no isolated segments.
- [ ] Road network geometry passes validity check before routing.
- [ ] Cost field is non-negative and in consistent units across all edges.
- [ ] Service area boundary is closed (no dangling open polygons).
- [ ] Stream order rasters match expected hierarchy from 1:50 000 reference map.
