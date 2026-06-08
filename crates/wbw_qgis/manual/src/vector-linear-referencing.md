# Linear Referencing — Tool Reference


---

## Snap Events To Routes

**Function name:** `snap_events_to_routes`


*No help documentation available for this tool.*


---

## Route Event Governance For Linear Assets

**Function name:** `route_event_governance_for_linear_assets`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Route Event Governance**

#### Who It Is For

- Departments of Transportation managing pavement/paving event datasets.
- Pipeline operators maintaining linear segment datasets (coating, pressure class, material type).
- Rail and powerline operators tracking inspection or maintenance event inventories.
- GIS production teams responsible for LRS data quality before system integration.

#### Primary User

DOTs, pipeline operators, rail/powerline operators, and telecom managers.

#### What It Does

- Validates route event datasets (line segments with from/to measures) against production governance rules.
- Detects overlapping events (events sharing measure ranges on the same route).
- Detects measure gaps (discontinuities between sequential events).
- Detects descending intervals (to_measure ParameterTypeRequiredDescription
`events`Vector pathRequiredEvent layer (GeoPackage, GeoJSON, Shapefile) with route ID and from/to measure fields
`route_id_field`stringRequiredField name containing route identifiers
`from_measure_field`stringRequiredField name for interval start measure
`to_measure_field`stringRequiredField name for interval end measure
`gap_tolerance`floatOptionalGaps smaller than this value are not flagged (default 0.0)
`overlap_tolerance`floatOptionalOverlaps smaller than this value are not flagged (default 0.0)
`auto_fix`boolOptionalEnable auto-correction of descending intervals and trimming of overlaps (default false)
`domain_rules_json`pathOptionalJSON file defining per-field validation rules with `allowed_values`, `regex`, `min`, and `max` checks
`governed_events`vector pathRequiredOutput path for QA-passed events with GOVERNANCE_STATUS and CORRECTIONS attributes
`issues_csv`pathRequiredOutput CSV path for per-event issue log
`corrected_events`vector pathOptionalOutput path for auto-corrected events (only written when auto_fix=true)
`governance_report`pathRequiredOutput JSON path for governance summary report
`remediation_queue_csv`pathOptionalOptional prioritized remediation queue with recommended corrective action

### Outputs

OutputTypeContents
`governed_events`VectorQA-passed events; attributes include GOVERNANCE_STATUS ("PASSED"/"CORRECTED") and CORRECTIONS (correction type or "none")
`issues_csv`CSVPer-violation log: event_id, route_id, rule_violated, severity, description, measure_start, measure_end
`corrected_events`VectorOnly written when auto_fix=true; lists corrected events with CORR_TYPE, ORIG_FROM, ORIG_TO, CORR_FROM, CORR_TO columns
`governance_report`JSONSummary: total_events, passed_events, failed_events, pass_rate_percent, rules_violated, severity_distribution, correctable_count, domain_rules_applied
`remediation_queue_csv`CSVPrioritized queue with rule category, severity, and recommended corrective action

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("route_event_governance_for_linear_assets",
    events="pavement_events.gpkg",
    route_id_field="ROUTE_ID",
    from_measure_field="FROM_MEAS",
    to_measure_field="TO_MEAS",
    gap_tolerance=0.5,
    overlap_tolerance=0.1,
    auto_fix=True,
    domain_rules_json="output/route_event_rules.json",
    governed_events="output/governed_events.gpkg",
    issues_csv="output/issues.csv",
    corrected_events="output/corrected_events.gpkg",
    governance_report="output/governance_report.json",
    remediation_queue_csv="output/remediation_queue.csv",
)

import json
report = json.loads(open(result["governance_report"]).read())
print(f"Pass rate: {report[&#x27;pass_rate_percent&#x27;]:.1f}%  "
      f"({report[&#x27;passed_events&#x27;]}/{report[&#x27;total_events&#x27;]} events)")`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Locate Points Along Routes

**Function name:** `locate_points_along_routes`


Experimental

Locates point features along route lines and writes route-measure attributes.

vector linear-referencing routes points

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.shp`
`points`Input point layer to locate along routes.Required`events.shp`
`max_offset_distance`Optional maximum point-to-route offset distance.Optional—
`output`Output point vector path.Required—

### Examples

*Adds route-measure attributes to points by locating them on the nearest route.*
`wbe.locate_points_along_routes(max_offset_distance=25.0, output='located_points.shp', points='events.shp', routes='routes.shp')`


---

## Points Along Lines

**Function name:** `points_along_lines`


Experimental

Creates regularly spaced point features along input line geometries.

vector points lines

### Parameters

NameDescriptionRequiredDefault
`input`Input line layer.Required`lines.shp`
`spacing`Spacing distance between points.Required`50.0`
`include_end`Include line endpoints (default true).Optional`True`
`output`Output point vector path.Required—

### Examples

*Creates points at fixed spacing along each line.*
`wbe.points_along_lines(include_end=True, input='lines.shp', output='points_along_lines.shp', spacing=50.0)`


---

## Route Calibrate

**Function name:** `route_calibrate`


Experimental

Calibrates route start/end measures from control points with known measures.

vector linear-referencing calibration

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.gpkg`
`control_points`Input control-point layer.Required`control_points.gpkg`
`control_measure_field`Control-point field containing known measure values.Required`measure`
`route_id_field`Optional route identifier field in routes. Defaults to feature FID.Optional—
`control_route_id_field`Optional route identifier field in control points. Defaults to feature FID.Optional—
`from_measure_field`Output field for route start measure (default 'from_measure').Optional—
`to_measure_field`Output field for route end measure (default 'to_measure').Optional—
`snap_tolerance`Maximum control-point offset distance from route geometry (default 1.0).Optional`1.0`
`output`Output calibrated route layer.Required—

### Examples

*Calibrates route start/end measures using route control points.*
`wbe.route_calibrate(control_measure_field='measure', control_points='control_points.gpkg', control_route_id_field='route_id', output='routes_calibrated.gpkg', route_id_field='route_id', routes='routes.gpkg', snap_tolerance=1.0)`


---

## Route Event Lines From Layer

**Function name:** `route_event_lines_from_layer`


Experimental

Creates routed line events from an event vector layer using from/to measures.

vector linear-referencing events

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.gpkg`
`events`Input event vector layer.Required`line_events.gpkg`
`event_route_field`Event-layer field containing route identifiers.Required`route_id`
`from_measure_field`Event-layer field containing start measures.Required`from_m`
`to_measure_field`Event-layer field containing end measures.Required`to_m`
`route_id_field`Optional route-layer field containing route identifiers. Defaults to feature FID.Optional—
`write_event_fid`Write EVENT_FID to preserve source event feature IDs (default true).Optional`True`
`write_event_xy`Write source event geometry X/Y attributes (default false).Optional`False`
`output`Output line vector path.Required—

### Examples

*Creates line events on routes from from/to measures in an event vector layer.*
`wbe.route_event_lines_from_layer(event_route_field='route_id', events='line_events.gpkg', from_measure_field='from_m', output='route_event_lines_layer.gpkg', route_id_field='RID', routes='routes.gpkg', to_measure_field='to_m', write_event_fid=True, write_event_xy=False)`


---

## Route Event Lines From Table

**Function name:** `route_event_lines_from_table`


Experimental

Creates routed line events from a CSV event table and a route layer using from/to measures.

vector linear-referencing events csv

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.gpkg`
`events`Input CSV event table path.Required`line_events.csv`
`event_route_field`CSV field containing route identifiers.Required`route_id`
`from_measure_field`CSV field containing start measures.Required`from_m`
`to_measure_field`CSV field containing end measures.Required`to_m`
`route_id_field`Optional route-layer field containing route identifiers. Defaults to feature FID.Optional—
`output`Output line vector path.Required—

### Examples

*Creates line events on routes from from/to measures stored in a CSV table.*
`wbe.route_event_lines_from_table(event_route_field='route_id', events='line_events.csv', from_measure_field='from_m', output='route_event_lines.gpkg', route_id_field='RID', routes='routes.gpkg', to_measure_field='to_m')`


---

## Route Event Merge

**Function name:** `route_event_merge`


Experimental

Merges adjacent compatible route events.

vector linear-referencing events merge

### Parameters

NameDescriptionRequiredDefault
`events`Input event layer containing route intervals.Required`events.gpkg`
`event_route_field`Event-layer route identifier field.Required`route_id`
`from_measure_field`Event-layer interval start field.Required`from_m`
`to_measure_field`Event-layer interval end field.Required`to_m`
`group_fields`Optional comma-delimited fields used for merge compatibility. Defaults to all non-measure fields.Optional—
`gap_tolerance`Maximum gap allowed for adjacency merge (default 0.0).Optional`0.0`
`conflict_mode`Overlap handling mode: error|skip (default error).Optional`error`
`output`Output merged-event layer.Required—

### Examples

*Merges compatible adjacent events on each route.*
`wbe.route_event_merge(conflict_mode='error', event_route_field='route_id', events='events.gpkg', from_measure_field='from_m', gap_tolerance=0.0, group_fields='route_id,road_class,speed', output='events_merged.gpkg', to_measure_field='to_m')`


---

## Route Event Overlay

**Function name:** `route_event_overlay`


Experimental

Overlays two route event layers by interval overlap.

vector linear-referencing events overlay

### Parameters

NameDescriptionRequiredDefault
`primary_events`Primary event layer.Required`primary_events.gpkg`
`overlay_events`Overlay event layer.Required`overlay_events.gpkg`
`primary_route_field`Primary layer route identifier field.Required`route_id`
`primary_from_measure_field`Primary layer interval start field.Required`from_m`
`primary_to_measure_field`Primary layer interval end field.Required`to_m`
`overlay_route_field`Overlay layer route identifier field.Required`route_id`
`overlay_from_measure_field`Overlay layer interval start field.Required`from_m`
`overlay_to_measure_field`Overlay layer interval end field.Required`to_m`
`min_overlap_length`Minimum overlap length to keep (default 0.0).Optional`0.0`
`output`Output overlay layer.Required—

### Examples

*Computes overlapping route-event intervals between two event layers.*
`wbe.route_event_overlay(min_overlap_length=0.0, output='events_overlay.gpkg', overlay_events='overlay_events.gpkg', overlay_from_measure_field='from_m', overlay_route_field='route_id', overlay_to_measure_field='to_m', primary_events='primary_events.gpkg', primary_from_measure_field='from_m', primary_route_field='route_id', primary_to_measure_field='to_m')`


---

## Route Event Points From Layer

**Function name:** `route_event_points_from_layer`


Experimental

Creates routed point events from an event vector layer and a route layer.

vector linear-referencing events

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.gpkg`
`events`Input event vector layer.Required`point_events.gpkg`
`event_route_field`Event-layer field containing route identifiers.Required`route_id`
`measure_field`Event-layer field containing point-event measures.Required`measure`
`route_id_field`Optional route-layer field containing route identifiers. Defaults to feature FID.Optional—
`write_event_fid`Write EVENT_FID to preserve source event feature IDs (default true).Optional`True`
`write_event_xy`Write source event geometry X/Y attributes (default false).Optional`False`
`output`Output point vector path.Required—

### Examples

*Creates point events on routes from measure values in an event vector layer.*
`wbe.route_event_points_from_layer(event_route_field='route_id', events='point_events.gpkg', measure_field='measure', output='route_event_points_layer.gpkg', route_id_field='RID', routes='routes.gpkg', write_event_fid=True, write_event_xy=False)`


---

## Route Event Points From Table

**Function name:** `route_event_points_from_table`


Experimental

Creates routed point events from a CSV event table and a route layer.

vector linear-referencing events csv

### Parameters

NameDescriptionRequiredDefault
`routes`Input route line layer.Required`routes.gpkg`
`events`Input CSV event table path.Required`point_events.csv`
`event_route_field`CSV field containing route identifiers.Required`route_id`
`measure_field`CSV field containing point-event measures.Required`measure`
`route_id_field`Optional route-layer field containing route identifiers. Defaults to feature FID.Optional—
`output`Output point vector path.Required—

### Examples

*Creates point events on routes from measure values stored in a CSV table.*
`wbe.route_event_points_from_table(event_route_field='route_id', events='point_events.csv', measure_field='measure', output='route_event_points.gpkg', route_id_field='RID', routes='routes.gpkg')`


---

## Route Event Split

**Function name:** `route_event_split`


Experimental

Splits route events by per-route boundary measures.

vector linear-referencing events split

### Parameters

NameDescriptionRequiredDefault
`events`Input event layer containing route intervals.Required`events.gpkg`
`boundaries`Input boundary layer containing route measure breakpoints.Required`event_boundaries.gpkg`
`event_route_field`Event-layer route identifier field.Required`route_id`
`from_measure_field`Event-layer interval start field.Required`from_m`
`to_measure_field`Event-layer interval end field.Required`to_m`
`boundary_route_field`Boundary-layer route identifier field.Required`route_id`
`boundary_measure_field`Boundary-layer measure field.Required`measure`
`min_segment_length`Optional minimum split segment length to keep (default 0.0).Optional`0.0`
`output`Output split-event layer.Required—

### Examples

*Splits route event intervals at supplied route measure boundaries.*
`wbe.route_event_split(boundaries='event_boundaries.gpkg', boundary_measure_field='measure', boundary_route_field='route_id', event_route_field='route_id', events='events.gpkg', from_measure_field='from_m', min_segment_length=0.0, output='events_split.gpkg', to_measure_field='to_m')`


---

## Route Measure QA

**Function name:** `route_measure_qa`


Experimental

Diagnoses route-event measure gaps, overlaps, non-monotonic sequences, and duplicate measures.

vector linear-referencing qa diagnostics

### Parameters

NameDescriptionRequiredDefault
`events`Input event layer containing route intervals.Required`events.gpkg`
`route_field`Route identifier field.Required`route_id`
`from_measure_field`Interval start field.Required`from_m`
`to_measure_field`Interval end field.Required`to_m`
`gap_tolerance`Gap tolerance (default 0.0).Optional`0.0`
`overlap_tolerance`Overlap tolerance (default 0.0).Optional`0.0`
`output`Output QA diagnostics layer.Required—

### Examples

*Generates route-measure diagnostics for interval event data.*
`wbe.route_measure_qa(events='events.gpkg', from_measure_field='from_m', gap_tolerance=0.0, output='route_measure_qa.gpkg', overlap_tolerance=0.0, route_field='route_id', to_measure_field='to_m')`


---

## Route Recalibrate

**Function name:** `route_recalibrate`


Experimental

Recalibrates edited route measures from a reference route layer while preserving route measure continuity.

vector linear-referencing recalibration

### Parameters

NameDescriptionRequiredDefault
`original_routes`Reference routes containing prior calibrated measures.Required`routes_original.gpkg`
`edited_routes`Edited routes to recalibrate.Required`routes_edited.gpkg`
`route_id_field`Optional shared route identifier field. Defaults to feature FID.Optional—
`from_measure_field`Measure-start field name (default 'from_measure').Optional—
`to_measure_field`Measure-end field name (default 'to_measure').Optional—
`output`Output recalibrated route layer.Required—

### Examples

*Scales edited route measures from a previously calibrated route layer.*
`wbe.route_recalibrate(edited_routes='routes_edited.gpkg', original_routes='routes_original.gpkg', output='routes_recalibrated.gpkg', route_id_field='route_id')`
