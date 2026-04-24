# Vector Analysis

Vector analysis in WbW-QGIS covers geometry validation, overlay operations,
attribute enrichment, spatial selection, proximity analysis, and spatial
joining. Whitebox supplements the native QGIS vector toolbox with
high-performance tools built on the wbtopology spatial index.

This chapter walks through a complete parcel-attribute enrichment workflow —
a common task in land management and environmental assessment.

---

## Key Concepts

- **Geometry validity**: Self-intersecting rings, duplicate vertices, and
  unclosed polygons cause silent failures in overlay tools. Always validate
  and repair geometry before any overlay operation.
- **Spatial join**: Assigns attributes from one layer to features in another
  based on spatial relationship (intersects, contains, nearest). Supports
  aggregation modes (first, last, sum, mean, count, min, max).
- **Near analysis**: Finds the nearest feature (or features within a distance)
  from a source layer to a target layer. Returns distance and optional target
  attributes.
- **Clip / Intersection / Difference**: Standard polygon overlay operations.
  Clip retains the geometry of input A bounded by B. Intersection produces
  the geometric overlap. Difference removes the overlap.
- **Add geometry attributes**: Computes and appends area, perimeter, length,
  centroid coordinates, or bounding box dimensions as new attribute fields.
- **Select by location**: Spatial predicate query (intersects, within,
  contains, etc.) that produces a feature selection or a new filtered layer.

---

## End-to-End Workflow: Parcel Attribute Enrichment

This workflow assigns catchment statistics and proximity-to-road measurements
to a parcel layer.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `parcels.shp` | Polygon vector | Land parcel boundaries |
| `catchments.shp` | Polygon vector | Watershed polygons with area and slope stats |
| `roads.shp` | Polyline vector | Road network |

---

### Step 1 — Validate and Repair Geometry

**Processing Toolbox → Vector Geometry → `Fix Geometries`** (QGIS native)

| Parameter | Recommended value |
|-----------|------------------|
| Input layer | `parcels.shp` |
| Output | `parcels_valid.shp` |

Run **`Check Validity`** on both `catchments.shp` and `roads.shp` and fix any
errors before proceeding.

---

### Step 2 — Add Geometry Attributes to Parcels

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Add Geometry Attributes`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `parcels_valid.shp` |
| Units | `Metres` |
| Output | `parcels_geom.shp` |

This appends `AREA`, `PERIMETER`, and centroid `X`/`Y` fields to each parcel.

---

### Step 3 — Spatial Join: Assign Catchment Attributes to Parcels

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Spatial Join`**

| Parameter | Recommended value |
|-----------|------------------|
| Target layer | `parcels_geom.shp` |
| Join layer | `catchments.shp` |
| Spatial relationship | `Intersects` |
| Join strategy | `First` (largest overlap catchment) |
| Fields to join | `catch_id`, `mean_slope`, `area_km2` |
| Output | `parcels_joined.shp` |

Each parcel now carries the attributes of the catchment it intersects most.

---

### Step 4 — Near: Distance from Each Parcel to Nearest Road

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Near`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector (source) | `parcels_joined.shp` |
| Near vector (target) | `roads.shp` |
| Max search distance (m) | `5000` (0 = search all) |
| Output | `parcels_near.shp` |

Appended fields: `NEAR_DIST` (metres to nearest road segment),
`NEAR_FID` (FID of nearest road feature).

---

### Step 5 — Select High-Priority Parcels

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Select By Attribute`** or use QGIS **Select by Expression**:

```
"AREA" > 10000 AND "NEAR_DIST" < 500 AND "mean_slope" < 10
```

Export the selection as `priority_parcels.shp` using
**Layer → Export → Save Selected Features As**.

---

### Step 6 — Clip Parcels to Study Area (Optional)

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Line Polygon Clip`** (for lines) or QGIS **`Clip`** for polygon-on-polygon.

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `priority_parcels.shp` |
| Clip polygon | `study_area.shp` |
| Output | `priority_parcels_clipped.shp` |

---

## Python Console Equivalent

```python
import processing

# Step 1: fix geometry
processing.run('native:fixgeometries', {
    'INPUT': '/data/parcels.shp',
    'OUTPUT': '/data/parcels_valid.shp',
})

# Step 2: add geometry attributes
processing.run('whitebox_workflows:add_geometry_attributes', {
    'input': '/data/parcels_valid.shp',
    'units': 'Metres',
    'output': '/data/parcels_geom.shp',
})

# Step 3: spatial join
processing.run('whitebox_workflows:spatial_join', {
    'input': '/data/parcels_geom.shp',
    'join': '/data/catchments.shp',
    'spatial_relation': 'Intersects',
    'join_method': 'First',
    'output': '/data/parcels_joined.shp',
})

# Step 4: near
processing.run('whitebox_workflows:near', {
    'input': '/data/parcels_joined.shp',
    'near': '/data/roads.shp',
    'max_dist': 5000.0,
    'output': '/data/parcels_near.shp',
})

print("Parcel enrichment complete.")
```

---

## Advanced: Simplify Features for Cartographic Output

Large polygon datasets with many vertices slow down rendering and tile export.
Simplify geometries while preserving topology.

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Simplify Features`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `priority_parcels_clipped.shp` |
| Algorithm | `Douglas-Peucker` |
| Tolerance (m) | `5.0` (adjust to display scale) |
| Output | `parcels_simplified.shp` |

```python
processing.run('whitebox_workflows:simplify_features', {
    'input': '/data/priority_parcels_clipped.shp',
    'algorithm': 'DouglasPeucker',
    'tolerance': 5.0,
    'output': '/data/parcels_simplified.shp',
})
```

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Spatial join returns no matches | CRS mismatch between target and join layers | Reproject both to the same CRS before joining |
| Near returns –1 for all distances | Search distance too small for data extent | Increase `max_dist` or set to 0 for unlimited search |
| Add geometry attributes returns wrong area | Layer CRS is geographic (degrees) | Reproject to a projected CRS (metres) first |
| Simplify removes valid narrow features | Tolerance too large | Use a smaller tolerance (< 1 m for cadastral data) |
| Select by location selects too many features | Predicate too inclusive (intersects vs. within) | Switch to `Within` or `Contains` for strict containment |

---

## Validation Checklist

- [ ] All input layers pass geometry validity check.
- [ ] All vector layers share the same projected CRS.
- [ ] Spatial join result preserves original feature count (check attribute table row count).
- [ ] NEAR_DIST values are plausible (inspect histogram).
- [ ] Simplified geometry does not self-intersect at the chosen tolerance.
- [ ] Attribute field names in output do not exceed shapefile 10-character limit.
