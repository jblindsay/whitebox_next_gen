# Linear Referencing

Linear referencing in WbW-R is the practice of locating events, measurements,
or observations along a route using a distance-based measure rather than
absolute coordinates. A road pavement database records a pothole at 2.4 km
along route R-12; a pipeline corridor flags an anomaly at 847 m along a trunk
line. Whitebox Next Gen provides the tools to locate observations onto measured
routes, place events from tabular or spatial sources, and — with a Pro licence
— audit the consistency of large linear asset datasets.

---

## Session Setup

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/linear_referencing')
```

---

## Core Concepts

A linear-referencing workflow has three parts:

1. **Routes** — line features defining the measurement axis. Each route has a
   unique identifier and M-values (cumulative distance from its start).
2. **Measures** — the distance value used to locate a position along a route.
3. **Events** — point or line observations located by (route\_id, measure) or
   (route\_id, from\_measure, to\_measure) pairs.

Common applications include road-pavement condition assessment, pipeline
integrity monitoring, trail difficulty reporting, environmental transect
sampling, and GPS probe data quality control.

---

## Step 1 — Understand Your Route Geometry

Routes must be single-part polylines with a consistent digitizing direction.
Before dropping events, confirm:

- Each route has a unique identifier stored in a field (e.g. `ROUTE_ID`).
- No route self-intersects.
- Routes forming a corridor are merged into one feature per identifier.

Use `snap_endnodes` and `merge_line_segments` via `wbw_run_tool` to clean
ragged street-centreline inputs before treating them as routes.

---

## Step 2 — Locate Points Along Routes

`locate_points_along_routes` takes an existing point layer and finds the
nearest position on each matching route, writing back the M-value (measure)
for every point. Use this when field teams have collected GPS observation points
and you need to convert them to route-distance offsets.

```r
wbw_run_tool('locate_points_along_routes', args = list(
  routes               = 'roads_measured.shp',
  points               = 'field_observations.shp',
  output               = 'observations_located.shp',
  max_offset_distance  = 15.0
), session = s)
# Output adds ROUTE_ID, MEASURE, and OFFSET fields to every input point.
```

The `MEASURE` field is the along-route distance from the route start.
`OFFSET` is the perpendicular snap distance. Points beyond
`max_offset_distance` are excluded from the output.

---

## Step 3 — Place Events from a Table

### Point Events

`route_event_points_from_table` reads a CSV where each row specifies a route
identifier and a measure value, and places a point feature at that position.
This is the standard import path for lab results, inspection records, or
maintenance logs stored in external databases.

```r
# pavement_defects.csv columns: ROUTE_ID, MEASURE, SEVERITY, NOTES
wbw_run_tool('route_event_points_from_table', args = list(
  routes             = 'roads_measured.shp',
  events             = 'pavement_defects.csv',
  event_route_field  = 'ROUTE_ID',
  measure_field      = 'MEASURE',
  output             = 'pavement_defects_located.shp'
), session = s)
```

### Line (Interval) Events

`route_event_lines_from_table` reads `FROM_MEASURE` and `TO_MEASURE` columns
to produce line segments — useful for pavement condition ratings, speed zones,
or any attribute that applies to a stretch of route rather than a single point.

```r
# pavement_condition.csv columns: ROUTE_ID, FROM_MEASURE, TO_MEASURE, IRI, CONDITION
wbw_run_tool('route_event_lines_from_table', args = list(
  routes             = 'roads_measured.shp',
  events             = 'pavement_condition.csv',
  event_route_field  = 'ROUTE_ID',
  from_measure_field = 'FROM_MEASURE',
  to_measure_field   = 'TO_MEASURE',
  output             = 'pavement_condition_segments.shp'
), session = s)
```

---

## Step 4 — Place Events from a Spatial Layer

When your event data is already a vector layer rather than a plain table, use
the `_from_layer` variants. These carry across all attributes of the source
feature and can optionally write the original FID and XY into the output.

### Point Events from a Layer

```r
wbw_run_tool('route_event_points_from_layer', args = list(
  routes             = 'roads_measured.shp',
  events             = 'manhole_inspections.shp',
  event_route_field  = 'ROUTE_ID',
  measure_field      = 'MEASURE',
  output             = 'manholes_on_routes.shp',
  write_event_fid    = TRUE,
  write_event_xy     = TRUE
), session = s)
```

### Line Events from a Layer

```r
wbw_run_tool('route_event_lines_from_layer', args = list(
  routes             = 'roads_measured.shp',
  events             = 'speed_zone_events.shp',
  event_route_field  = 'ROUTE_ID',
  from_measure_field = 'FROM_M',
  to_measure_field   = 'TO_M',
  output             = 'speed_zones_on_routes.shp',
  write_event_fid    = TRUE
), session = s)
```

---

## Step 5 — Linear Asset Governance *(Pro)*

`route_event_governance_for_linear_assets` audits a complete linear asset
dataset for measure gaps, overlaps, duplicate events, orphaned route
references, and out-of-range measures. It produces a flagged event output and
a structured report — essential for maintaining the integrity of a production
road or utility asset database.

```r
result <- s$run_tool(
  'route_event_governance_for_linear_assets',
  list(
    routes             = 'roads_measured.shp',
    events             = 'pavement_condition.shp',
    route_id_field     = 'ROUTE_ID',
    from_measure_field = 'FROM_MEASURE',
    to_measure_field   = 'TO_MEASURE',
    flagged_output     = 'event_flags.shp',
    report             = 'governance_report.csv'
  )
)

flags <- read.csv('governance_report.csv')
print(table(flags$ERROR_CLASS))
```

> **Note:** This tool requires a session initialised with a valid Pro licence.

---

## Complete Workflow: Road Pavement Assessment

```r
library(whiteboxworkflows)

s <- wbw_session()
setwd('/data/pavement_assessment')

# Step 1: Snap GPS observation points onto routes and extract M-values.
wbw_run_tool('locate_points_along_routes', args = list(
  routes              = 'roads_measured.shp',
  points              = 'field_inspection_gps.shp',
  output              = 'gps_on_routes.shp',
  max_offset_distance = 10.0
), session = s)

# Step 2: Place point defect records from the inspection database.
wbw_run_tool('route_event_points_from_table', args = list(
  routes            = 'roads_measured.shp',
  events            = 'defect_records.csv',
  event_route_field = 'ROUTE_ID',
  measure_field     = 'MEASURE',
  output            = 'defects_located.shp'
), session = s)

# Step 3: Place condition rating intervals.
wbw_run_tool('route_event_lines_from_table', args = list(
  routes             = 'roads_measured.shp',
  events             = 'condition_ratings.csv',
  event_route_field  = 'ROUTE_ID',
  from_measure_field = 'FROM_M',
  to_measure_field   = 'TO_M',
  output             = 'condition_segments.shp'
), session = s)

# Step 4 (Pro): Audit the condition layer for gaps and overlaps.
result <- s$run_tool(
  'route_event_governance_for_linear_assets',
  list(
    routes             = 'roads_measured.shp',
    events             = 'condition_segments.shp',
    route_id_field     = 'ROUTE_ID',
    from_measure_field = 'FROM_M',
    to_measure_field   = 'TO_M',
    flagged_output     = 'condition_flags.shp',
    report             = 'governance_report.csv'
  )
)
cat('Governance report:', 'governance_report.csv', '\n')
```

---

## Tips

- Routes must have a consistent digitizing direction. Run `snap_endnodes` and
  confirm that all segments in a route are digitized in the same direction
  before locating events.
- `locate_points_along_routes` excludes points beyond `max_offset_distance`.
  Inspect unmatched points to identify GPS outliers or route coverage gaps.
- Use `route_event_points_from_table` and `route_event_lines_from_table` for
  bulk imports from asset management databases where locations are already
  stored as route+measure pairs.
- Use the `_from_layer` variants when existing vector event layers already
  carry route identifier and measure fields.
- The `route_event_governance_for_linear_assets` Pro tool scales to production
  databases with millions of events and produces actionable error reports for
  integration into maintenance management systems.
