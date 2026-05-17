# Linear Referencing

Linear referencing is the practice of locating observations, measurements, or
events along a route using a distance-based measure rather than absolute
coordinates. A road asset database might record a pothole at 2.4 km from the
start of route R-12; a utility corridor might flag a pressure anomaly at 847 m
along a pipeline. Whitebox Next Gen provides the tools to build measured routes,
locate observations against them, place events from tables or spatial layers,
and — with a Pro licence — audit the consistency and governance of large linear
asset datasets.

---

## Core Concepts

A linear-referencing workflow has three parts:

1. **Routes** — line features that define the measurement axis. Each route has
   a unique identifier and carries M-values (measures) representing cumulative
   distance from its start point.
2. **Measures** — the distance value used to locate a position along a route.
3. **Events** — point or line observations located by (route\_id, measure) or
   (route\_id, from\_measure, to\_measure) pairs.

Common Whitebox Next Gen applications include:

- road-pavement condition assessment by segment
- pipeline corridor integrity monitoring
- trail difficulty and visibility reporting
- environmental sampling along survey transects
- GPS probe data quality control and kilometrage reporting

---

## Route Calibration and M-Value Management

Measures are only useful when anchored to real-world control points. If your
raw routes lack calibration, or have been edited, use the calibration tools
to establish stable, field-verified measures.

### Initial Calibration from Control Points

`route_calibrate()` establishes measure values on routes using control points
with known measures. For example, if you have kilometre posts at known
distances along a highway, calibration ensures your event locations align with
field reality.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/linear_referencing'

routes = wbe.read_vector('highway_centerlines.shp')
km_posts = wbe.read_vector('km_post_locations.shp')  # with ROUTE_ID and KNOWN_MEASURE fields

calibrated = wbe.vector.linear_referencing.route_calibrate(
    routes=routes,
    control_points=km_posts,
    control_measure_field='KNOWN_MEASURE',
    route_id_field='ROUTE_ID',
    snap_tolerance=10.0  # max control-point offset from route (meters)
)
wbe.write_vector(calibrated, 'highway_calibrated.shp')
# Output includes FROM_MEASURE and TO_MEASURE fields.
```

### Recalibration After Edits

If you edit routes (split, merge, or regeometrize), use `route_recalibrate()`
to scale measures proportionally and maintain event alignment.

```python
edited_routes = wbe.read_vector('highway_edited.shp')  # after geometric changes

recalibrated = wbe.vector.linear_referencing.route_recalibrate(
    original_routes=calibrated,     # reference with valid measures
    edited_routes=edited_routes,
    route_id_field='ROUTE_ID'
)
wbe.write_vector(recalibrated, 'highway_recalibrated.shp')
```

### Validating Event Snapping

Before placing events on routes, diagnose snapping issues with `snap_events_to_routes()`.
This reports snap distance, offset, and any unmatched events.

```python
obs_points = wbe.read_vector('field_observations.shp')

diag = wbe.vector.linear_referencing.snap_events_to_routes(
    routes=calibrated,
    events=obs_points,
    max_offset_distance=15.0
)
wbe.write_vector(diag, 'observations_snap_diagnostics.shp')
# Output includes ROUTE_ID, MEASURE, and OFFSET fields; unmatched features are excluded.
```

---

## Step 1 — Understand Your Route Geometry

Routes must be single-part polylines with a consistent digitizing direction.
Before dropping events, confirm:

- Each route has a unique identifier stored in a field (e.g. `ROUTE_ID`).
- No route self-intersects.
- Routes that form a corridor are merged into one feature per route identifier.

Use `snap_endnodes` and `merge_line_segments` to clean ragged street-centreline
inputs before treating them as routes.

---

## Step 2 — Locate Points Along Routes

`locate_points_along_routes()` takes an existing point layer and finds the
nearest position on each matching route, writing back the M-value (measure)
for every point. This is the first tool to reach for when your field team has
collected GPS observation points and you need to convert them to route-distance
offsets for further analysis.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/linear_referencing'

routes      = wbe.read_vector('roads_measured.shp')
obs_points  = wbe.read_vector('field_observations.shp')

located = wbe.vector.linear_referencing.locate_points_along_routes(
    routes=routes,
    points=obs_points,
    max_offset_distance=15.0   # max perpendicular snap distance in map units
)
wbe.write_vector(located, 'observations_located.shp')
# Output adds ROUTE_ID, MEASURE, and OFFSET fields to every input point.
```

The `MEASURE` field in the output is the along-route distance from the route
start. `OFFSET` is the perpendicular distance from the point to the route.
Points beyond `max_offset_distance` are not matched and are excluded from the
output.

---

## Step 3 — Place Events from a Table

### Point Events

`route_event_points_from_table()` reads a CSV (or other tabular file) where
each row specifies a route identifier and a measure value, and places a point
feature at that position along the matching route. This is the standard import
path for lab results, inspection records, or maintenance logs stored in
external databases.

```python
# events.csv columns: ROUTE_ID, MEASURE, SEVERITY, NOTES
pts = wbe.vector.linear_referencing.route_event_points_from_table(
    routes=routes,
    events='pavement_defects.csv',
    event_route_field='ROUTE_ID',
    measure_field='MEASURE'
)
wbe.write_vector(pts, 'pavement_defects_located.shp')
```

### Line (Interval) Events

`route_event_lines_from_table()` works the same way but reads `FROM_MEASURE`
and `TO_MEASURE` columns to produce line segments — useful for pavement
condition ratings, speed zones, or any attribute that applies to a stretch of
route rather than a single point.

```python
# condition.csv columns: ROUTE_ID, FROM_MEASURE, TO_MEASURE, IRI, CONDITION
segs = wbe.vector.linear_referencing.route_event_lines_from_table(
    routes=routes,
    events='pavement_condition.csv',
    event_route_field='ROUTE_ID',
    from_measure_field='FROM_MEASURE',
    to_measure_field='TO_MEASURE'
)
wbe.write_vector(segs, 'pavement_condition_segments.shp')
```

---

## Step 4 — Place Events from a Spatial Layer

When your event data is already a vector layer rather than a plain table, use
the `_from_layer` variants. These carry across all attributes of the source
feature and can optionally write the feature's original FID and XY coordinates
into the output.

### Point Events from a Layer

```python
inspections = wbe.read_vector('manhole_inspections.shp')

pts_from_layer = wbe.vector.linear_referencing.route_event_points_from_layer(
    routes=routes,
    events=inspections,
    event_route_field='ROUTE_ID',
    measure_field='MEASURE',
    write_event_fid=True,   # retain original feature ID in output
    write_event_xy=True     # also store original XY in output
)
wbe.write_vector(pts_from_layer, 'manholes_on_routes.shp')
```

### Line Events from a Layer

```python
speed_zones = wbe.read_vector('speed_zone_events.shp')

segs_from_layer = wbe.vector.linear_referencing.route_event_lines_from_layer(
    routes=routes,
    events=speed_zones,
    event_route_field='ROUTE_ID',
    from_measure_field='FROM_M',
    to_measure_field='TO_M',
    write_event_fid=True
)
wbe.write_vector(segs_from_layer, 'speed_zones_on_routes.shp')
```

---

## Step 5 — Linear Asset Governance *(Pro)*

`route_event_governance_for_linear_assets` audits a complete linear asset
dataset for measure gaps, overlaps, duplicate events, orphaned route
references, and out-of-range measures. It produces a flagged event output and
a structured report — essential for maintaining the integrity of a production
road or utility asset database.

```python
result = wbe.run_tool(
    'route_event_governance_for_linear_assets',
    {
        'routes':             'roads_measured.shp',
        'events':             'pavement_condition.shp',
        'route_id_field':     'ROUTE_ID',
        'from_measure_field': 'FROM_MEASURE',
        'to_measure_field':   'TO_MEASURE',
        'flagged_output':     'event_flags.shp',
        'report':             'governance_report.csv'
    }
)
print(result)
```

The flagged output marks each event with an error code and description.
The report CSV summarises error counts by class and route, ready for
integration into a maintenance management system.

> **Note:** This tool requires a `WbEnvironment` initialised with a valid Pro
> licence.

---

## Complete Workflow: Road Pavement Assessment

The following example takes a road network, locates inspection points collected
in the field, overlays a condition rating table, and exports both a point layer
and a segment layer ready for pavement management reporting.

```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()
wbe.working_directory = '/data/pavement_assessment'

routes         = wbe.read_vector('roads_measured.shp')
field_gps      = wbe.read_vector('field_inspection_gps.shp')

# Step 1: Snap GPS observation points onto routes and extract M-values.
located = wbe.vector.linear_referencing.locate_points_along_routes(
    routes=routes,
    points=field_gps,
    max_offset_distance=10.0
)
wbe.write_vector(located, 'gps_on_routes.shp')

# Step 2: Place point defect records from the inspection database.
defects = wbe.vector.linear_referencing.route_event_points_from_table(
    routes=routes,
    events='defect_records.csv',
    event_route_field='ROUTE_ID',
    measure_field='MEASURE'
)
wbe.write_vector(defects, 'defects_located.shp')

# Step 3: Place condition rating intervals from the same database.
condition = wbe.vector.linear_referencing.route_event_lines_from_table(
    routes=routes,
    events='condition_ratings.csv',
    event_route_field='ROUTE_ID',
    from_measure_field='FROM_M',
    to_measure_field='TO_M'
)
wbe.write_vector(condition, 'condition_segments.shp')

# Step 4 (Pro): Audit the condition layer for gaps and overlaps.
result = wbe.run_tool(
    'route_event_governance_for_linear_assets',
    {
        'routes':             'roads_measured.shp',
        'events':             'condition_segments.shp',
        'route_id_field':     'ROUTE_ID',
        'from_measure_field': 'FROM_M',
        'to_measure_field':   'TO_M',
        'flagged_output':     'condition_flags.shp',
        'report':             'governance_report.csv'
    }
)
print('Governance report:', result)
```

---

## Tips

- Routes must have a consistent digitizing direction. If your source network
  was assembled from multiple editors, run `snap_endnodes` and check that all
  segments in a single route are digitized in the same direction before
  computing M-values.
- `locate_points_along_routes()` excludes points that exceed
  `max_offset_distance`. Inspect unmatched points to identify GPS outliers or
  route coverage gaps.
- Use `route_event_points_from_table()` and `route_event_lines_from_table()`
  for bulk imports from asset management databases where the event locations
  are already stored as route+measure pairs.
- Use the `_from_layer` variants when you have existing vector event layers
  that already carry a route identifier and measure fields.
- The `route_event_governance_for_linear_assets` Pro tool scales to production
  road-asset databases with millions of events and produces actionable error
  reports ready for integration into maintenance workflows.
