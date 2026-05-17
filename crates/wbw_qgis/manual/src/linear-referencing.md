# Linear Referencing

Linear referencing (LRS) is a data model where features are located along
routes by a measured distance from a known origin — rather than by absolute
X/Y coordinates. It is the foundation of road/rail inventory, pipeline
inspection data, accident records, and pavement condition databases.

WbW-QGIS provides tools for building measure fields on route networks,
locating point and line events along routes, and exporting event geometries
for spatial analysis.

---

## Key Concepts

- **Route**: A polyline feature with a unique, stable route identifier
  (`ROUTE_ID`) and a monotonically increasing measure value (M-value) along
  its length.
- **M-value**: The accumulated distance (or time, or post number) from the
  route origin to each vertex. Stored as the M coordinate in an MZ geometry.
- **Event**: A point or interval on a route located by one measure (point
  event) or two measures (line event: from-measure, to-measure).
- **Event table**: A tabular record set with `ROUTE_ID`, `MEASURE` (point) or
  `FROM_M`/`TO_M` (line), and any associated attributes.
- **Dynamic segmentation**: The process of converting event tables to geometry
  by interpolating measure positions along routes.
- **Calibration**: Adjusting M-values to match real-world control points —
  for example, aligning stationing to kilometre posts.

---

## End-to-End Workflow: Locating Inspection Events Along a Road Network

This workflow builds measure fields on a road network, then locates a set of
field inspection points as events on their respective routes.

### Inputs

| Layer | Format | Notes |
|-------|--------|-------|
| `roads.shp` | Polyline vector | Road centrelines, unique `ROUTE_ID` field |
| `inspections.csv` | CSV table | Columns: `ROUTE_ID`, `CHAINAGE_M`, `CONDITION` |

---

### Step 1 — Add Cumulative Distance (Measure) Field

WbW computes the cumulative distance from each route's start vertex to every
subsequent vertex and writes it as the M coordinate.

**Processing Toolbox → Whitebox Workflows → Vector Analysis →
`Add Geometry Attributes`**

| Parameter | Recommended value |
|-----------|------------------|
| Input vector | `roads.shp` |
| Units | `Metres` |
| Output | `roads_geom.shp` |

This step appends `LENGTH` to each segment. For building full route M-values
use the QGIS **`Set M Value`** tool (from Geometry group) after merging
segments by `ROUTE_ID`.

**Alternative — set M values from a field:**

**Processing Toolbox → Vector Geometry → `Set M Value`** (QGIS native)

| Parameter | Recommended value |
|-----------|------------------|
| Input layer | `roads.shp` (merged per route) |
| M value | Expression: `$length` (QGIS expression — cumulative along merged route) |
| Output | `routes_m.shp` |

---

## Route Calibration and Recalibration

Measures are only useful when anchored to real-world control points such as kilometre posts or survey stations. If your routes lack calibration or have been edited, use these tools to establish stable, field-verified measures.

### Calibrate Routes from Control Points

**Processing Toolbox → Whitebox Workflows → Linear Referencing →
`Route Calibrate`**

| Parameter | Value |
|-----------|-------|
| Input routes | roads.shp (with ROUTE_ID field) |
| Control points | km_posts.shp (with ROUTE_ID and KNOWN_MEASURE fields) |
| Control measure field | KNOWN_MEASURE |
| Snap tolerance | 10.0 (meters) |
| Output | routes_calibrated.shp |

Output adds `FROM_MEASURE` and `TO_MEASURE` fields containing the calibrated values.

### Recalibrate After Route Edits

If you split, merge, or redraw routes, use recalibration to scale measures proportionally:

**Processing Toolbox → Whitebox Workflows → Linear Referencing →
`Route Recalibrate`**

| Parameter | Value |
|-----------|-------|
| Original routes | routes_calibrated.shp (reference with valid measures) |
| Edited routes | routes_edited.shp (after geometric changes) |
| Output | routes_recalibrated.shp |

---

### Step 2 — Validate Route Identifiers

Route IDs must be unique per route and stable across updates. Check for
duplicates.

**QGIS → Open Attribute Table → Field Calculator** or via the Python Console:

```python
from qgis.core import QgsVectorLayer

layer = QgsVectorLayer('/data/routes_m.shp', 'routes', 'ogr')
ids = [f['ROUTE_ID'] for f in layer.getFeatures()]
duplicates = [x for x in ids if ids.count(x) > 1]
if duplicates:
    print(f"Duplicate route IDs found: {set(duplicates)}")
else:
    print("All route IDs are unique.")
```

---

### Step 3 — Locate Point Events (Dynamic Segmentation)

**Processing Toolbox → Whitebox Workflows → Linear Referencing →
`Locate Point Events`**

| Parameter | Recommended value |
|-----------|------------------|
| Input routes | `routes_m.shp` |
| Event table | `inspections.csv` |
| Route ID field (routes) | `ROUTE_ID` |
| Route ID field (events) | `ROUTE_ID` |
| Measure field | `CHAINAGE_M` |
| Output | `inspection_points.shp` |

Each CSV row becomes a point geometry placed at the corresponding measure
position on its route. Rows with unmatched route IDs or out-of-range measures
are written to an error table.

---

### Step 4 — Locate Line Events (Optional)

If the inspection table records intervals (e.g. pavement condition rated over
100 m segments):

**Processing Toolbox → Whitebox Workflows → Linear Referencing →
`Locate Line Events`**

| Parameter | Recommended value |
|-----------|------------------|
| Input routes | `routes_m.shp` |
| Event table | `pavement.csv` |
| Route ID field (routes) | `ROUTE_ID` |
| Route ID field (events) | `ROUTE_ID` |
| From-measure field | `FROM_M` |
| To-measure field | `TO_M` |
| Output | `pavement_segments.shp` |

---

### Step 5 — Inspect and Validate Event Geometry

Load `inspection_points.shp` in QGIS. Pan to several known inspection records
and confirm point positions against the road centreline.

Use **QGIS Identify tool** to click a point and verify that `CHAINAGE_M`
matches the M-value of the nearest route vertex within acceptable tolerance
(typically ± half the route vertex spacing).

---

## Python Console Equivalent

```python
import processing

# Step 3: locate point events
processing.run('whitebox_workflows:locate_point_events', {
    'routes': '/data/routes_m.shp',
    'events': '/data/inspections.csv',
    'route_id_field': 'ROUTE_ID',
    'event_route_id_field': 'ROUTE_ID',
    'measure_field': 'CHAINAGE_M',
    'output': '/data/inspection_points.shp',
})

# Step 4: locate line events
processing.run('whitebox_workflows:locate_line_events', {
    'routes': '/data/routes_m.shp',
    'events': '/data/pavement.csv',
    'route_id_field': 'ROUTE_ID',
    'event_route_id_field': 'ROUTE_ID',
    'from_measure_field': 'FROM_M',
    'to_measure_field': 'TO_M',
    'output': '/data/pavement_segments.shp',
})

print("Linear referencing complete.")
```

---

## Advanced: Calibrate Routes Against Control Points

If field-collected kilometre posts differ from computed cumulative distance,
calibrate M-values by interpolating between control points.

**Processing Toolbox → Whitebox Workflows → Linear Referencing →
`Calibrate Route`**

| Parameter | Recommended value |
|-----------|------------------|
| Input routes | `routes_m.shp` |
| Calibration points | `km_posts.shp` (with `ROUTE_ID` and `KNOWN_M` fields) |
| Route ID field | `ROUTE_ID` |
| Measure field | `KNOWN_M` |
| Search tolerance (m) | `50` |
| Output | `routes_calibrated.shp` |

After calibration, re-run `Locate Point Events` on `routes_calibrated.shp` to
position events against field-verified measures.

---

## Common Pitfalls

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Events do not locate (0 features output) | Route ID field names do not match | Check both `ROUTE_ID` parameter values match the actual field names |
| Events placed far from expected position | M-values in event table use different units | Confirm both routes and events use the same unit (metres vs km) |
| Out-of-range events produce no error output | Measure > route end measure | Check that `FROM_M`/`TO_M` do not exceed the route total length |
| Calibration shifts all events uniformly | Only one control point per route | Add at least two control points per route for interpolation |
| Duplicate route IDs cause incorrect event assignment | Merged route has repeated IDs | Dissolve route features by `ROUTE_ID` before building M-values |

---

## Validation Checklist

- [ ] Route IDs are unique per route with no duplicates.
- [ ] M-values are monotonically increasing along each route (no reversals).
- [ ] Event table measures fall within the range [0, route total length].
- [ ] Located point events visually snap to the correct road centreline.
- [ ] Line event segment lengths match `TO_M - FROM_M` within 0.1 m.
- [ ] Error table from `Locate Events` contains zero unmatched records.
