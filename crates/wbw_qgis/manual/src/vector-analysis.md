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

### Step 5b — Field Calculator Assistant (Expression + Preview Workflow)

Use this when you need guided expression authoring for derived attributes
(for example TYPE-to-SPEED conversion before network impedance analysis).

Open from the **Whitebox panel** (recommended path):

**Whitebox panel → tool search → `field_calculator`**

The assistant provides:

- expression editor with SQL-style presets/snippets
- geometry token insertion (`$area`, `$length`, `$perimeter`, centroid tokens)
- category and keyword snippet filtering
- preview table driven by backend `preview_rows` payload
- one-click handoff to the standard processing dialog with prefilled parameters

Supported expression features include:

- `CASE WHEN ... THEN ... ELSE ... END`
- simple `CASE field WHEN value THEN ... END`
- optional `UPDATE ... SET ... WHERE ...` wrapper syntax
- SQL operators (`=`, `<>`, `AND`, `OR`, `NOT`) and null predicates
- `CAST(... AS integer|float|text|boolean)`

Example expression:

```sql
UPDATE roads SET SPEED = CASE
  WHEN TYPE == 'motorway' THEN 100
  WHEN TYPE == 'primary' THEN 80
  WHEN TYPE == 'collector' THEN 60
  ELSE 40
END
```

Notes:

- Launching `field_calculator` from the Whitebox panel opens the assistant.
- Launching from the generic Processing Toolbox can open the standard dialog
  directly, depending on host/API path.

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

## TopoJSON Conversion Chain (QGIS Interop)

Use this workflow when you need to exchange shared-boundary vector data with
web clients while keeping a GeoPackage working copy for analysis.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `zones.gpkg` | Polygon vector | Authoritative analysis dataset |

### Step 1 — Run a Whitebox vector operation and emit TopoJSON

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Add Geometry Attributes`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `zones.gpkg` |
| Units | `Metres` |
| Output | `zones_metrics.topojson` |

This confirms the plugin accepts `.topojson` output targets in a normal vector
processing chain.

### Step 2 — Re-open TopoJSON and convert back to GeoPackage

1. Add `zones_metrics.topojson` to the QGIS project.
2. Right-click the layer, then choose **Export → Save Features As...**.
3. Set format to **GeoPackage** and save as `zones_metrics_roundtrip.gpkg`.

### Step 3 — Validate roundtrip integrity

Check these before publishing or reusing the roundtrip layer:

- Feature count matches source layer.
- Core attributes (e.g., ID fields and geometry metrics) are preserved.
- CRS is correctly populated on the roundtrip GeoPackage.

### Recommended use pattern

- Keep `.gpkg` as the editable analysis master.
- Generate `.topojson` as interchange or web-delivery artifacts.
- Re-import to `.gpkg` for heavier downstream spatial analysis.

## TopoJSON Boundary-Preserving Generalization Chain

Use this chain when you need smaller delivery payloads while preserving shared
boundary consistency during simplification.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `admin_units.gpkg` | Polygon vector | Shared boundaries between adjacent polygons |

### Step 1 — Simplify and emit TopoJSON

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Simplify Features`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `admin_units.gpkg` |
| Algorithm | `Douglas-Peucker` |
| Tolerance | `25.0` (adjust to target scale) |
| Output | `admin_units_simplified.topojson` |

### Step 2 — Inspect topology consistency in QGIS

1. Add `admin_units_simplified.topojson` to the map.
2. Inspect shared boundaries at high zoom for slivers/gaps.
3. Validate feature count versus source before publication.

### Step 3 — Export analysis copy

Export to `admin_units_simplified.gpkg` for downstream joins/overlay work.

## TopoJSON Transport + Enrichment Return Chain

Use this chain when TopoJSON is used only for transport and you need to return
to an analysis-grade format for attribute enrichment.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `transport_in.topojson` | Topology-preserving vector | Interchange input received from external system |

### Step 1 — Convert transport input to GeoPackage

1. Add `transport_in.topojson` to the project.
2. Export as `transport_stage.gpkg`.

### Step 2 — Apply enrichment tools

Run Whitebox vector tools against `transport_stage.gpkg`:

- `Add Geometry Attributes` for geometry metrics.
- `Spatial Join` for contextual attribute enrichment.
- `Near` for proximity attributes.

### Step 3 — Emit deliverables

Write two outputs:

- `transport_enriched.gpkg` for analytic persistence.
- `transport_enriched.topojson` for interchange/web handoff.

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
