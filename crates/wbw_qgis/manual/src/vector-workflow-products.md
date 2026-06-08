# Workflow Products


---

## Utility Corridor Encroachment And Access Planning

**Function name:** `utility_corridor_encroachment_and_access_planning`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Utility Corridor Access Planning**

#### Who It Is For

- Utility corridor maintenance planners and linear-infrastructure field operations teams.
- Vegetation/encroachment risk analysts coordinating access logistics.

#### Primary User

Transmission/distribution utilities and corridor maintenance operations.

#### What It Does

- Identifies encroachment hotspots that fall within a configurable corridor influence distance.
- Scores hotspot risk based on proximity to corridor centerlines.
- Assigns nearest access points for field-response feasibility.
- Produces ranked hotspot CSV plus planning summary JSON for operations teams.
- Adds dispatch-ready priority bands, SLA guidance, and optional response queue export.

#### How It Works

- Loads corridor line geometry, encroachment observations, and access points.
- Converts corridor geometry to line segments and computes minimum point-to-segment distance for each encroachment.
- Keeps in-range encroachments as hotspots using `corridor_influence_distance` threshold.
- Computes risk score as inverse-distance to corridor (closer = higher risk).
- Computes access score from nearest access-point distance and combines with risk into priority score.
- Classifies each hotspot into `critical`, `high`, `medium`, or `low` response bands and assigns response SLA hours.
- Emits hotspot vector with attributes: ENC_FID, DIST_CORR, RISK_SCORE, ACCESS_FID, ACCESS_DIST, PRIORITY, PRIOR_BAND, SLA_HOURS, ACCESS_DIFF.

### Inputs

ParameterTypeRequiredDescription
`corridors`LineVector pathRequiredCorridor centerline layer
`encroachments`Vector pathRequiredEncroachment observations (point/line/polygon; representative point sampled)
`access_points`PointVector pathRequiredField access points
`corridor_influence_distance`floatOptionalMax distance from corridor to retain as hotspot (default 30.0)
`high_risk_distance`floatOptionalDistance considered highest-risk zone (default 10.0)
`hotspots`vector pathRequiredOutput hotspot vector
`priority_csv`pathRequiredOutput ranked hotspot CSV
`planning_report`pathRequiredOutput planning summary JSON
`response_queue_csv`pathOptionalOptional dispatch-ready response queue with SLA guidance

### Outputs

OutputTypeContents
`hotspots`VectorHotspot points with risk/access/priority attributes plus corridor lineage fields (`CORR_FID`, `SEG_IDX`, `LINEAGE_ID`)
`priority_csv`CSVRanked hotspots: rank, enc_fid, corridor_fid, segment_idx, dist_to_corridor, risk_score, nearest_access_fid, access_dist, priority_score, priority_band, response_sla_hours, access_difficulty, lineage_id
`planning_report`JSONCounts, averages, thresholds, counts by priority band, and top hotspot summary
`response_queue_csv`CSVResponse queue with priority band, SLA target, access difficulty, recommended action, and lineage_id

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("utility_corridor_encroachment_and_access_planning",
    corridors="corridor_centerlines.gpkg",
    encroachments="encroachment_points.gpkg",
    access_points="field_access_points.gpkg",
    corridor_influence_distance=30.0,
    high_risk_distance=10.0,
    hotspots="output/corridor_hotspots.gpkg",
    priority_csv="output/corridor_priority.csv",
    planning_report="output/corridor_planning_report.json",
    response_queue_csv="output/corridor_response_queue.csv",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Parcel And Land Fabric Topology Compliance Workflow

**Function name:** `parcel_and_land_fabric_topology_compliance_workflow`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Parcel Fabric Topology Compliance**

#### Who It Is For

- Local government cadastral teams and parcel QA workflows.
- Land administration vendors with regulatory topology compliance requirements.

#### Primary User

Municipal cadastral programs and land administration platforms.

#### What It Does

- Audits parcel fabrics for topology compliance using rule-based checks.
- Flags parcel slivers below a configurable minimum area threshold.
- Produces violations vector, issues CSV, and a compliance summary JSON.
- Optionally runs topology auto-fix and outputs corrected parcel geometry.
- Supports jurisdiction templates (`generic`, `ontario_mpac`) for calibrated defaults.
- Optionally emits a remediation queue CSV with prioritized corrective actions.
- Emits sliver-threshold calibration diagnostics so parcel fabrics can be profiled before tightening production thresholds.

#### How It Works

- Runs `topology_validation_report` to produce per-feature topology issue CSV.
- Runs `topology_rule_validate` with parcel-focused rules (`polygon_must_not_overlap`, `polygon_must_not_have_gaps`).
- Performs additional sliver detection on polygon area (`area ParameterTypeRequiredDescription
`parcels`PolygonVector pathRequiredInput parcel polygon layer
`min_sliver_area`floatOptionalArea threshold for sliver detection (default 1.0)
`auto_fix`boolOptionalRun topology auto-fix and emit corrected output (default false)
`jurisdiction_template`stringOptionalRule-template preset (`generic` | `ontario_mpac`) used for calibrated defaults
`topology_violations`vector pathRequiredOutput topology violations layer
`issues_csv`pathRequiredOutput topology validation CSV
`compliance_report`pathRequiredOutput compliance summary JSON
`corrected_parcels`vector pathOptionalOptional corrected parcel output path when auto_fix=true
`remediation_queue_csv`pathOptionalOptional prioritized remediation action queue CSV
`html_report`pathOptionalOptional output HTML path for the compliance dashboard report

### Outputs

OutputTypeContents
`topology_violations`VectorRule violations generated by topology rule validation
`issues_csv`CSVPer-feature topology issue report from topology validation
`compliance_report`JSONSummary counts by rule, sliver diagnostics, sliver calibration profile, autofix summary, pass/fail
`corrected_parcels`VectorAuto-fix output when enabled and path provided
`remediation_queue_csv`CSVPriority-ranked remediation actions by issue type/rule
`html_report`HTMLOptional compliance dashboard report with visual summary

### Python Example

`env = WbEnvironment(license_tier="pro")

result = env.run_tool("parcel_and_land_fabric_topology_compliance_workflow",
    parcels="parcel_fabric.gpkg",
    min_sliver_area=1.0,
    jurisdiction_template="ontario_mpac",
    auto_fix=True,
    topology_violations="output/parcel_violations.gpkg",
    issues_csv="output/parcel_issues.csv",
    compliance_report="output/parcel_compliance.json",
    corrected_parcels="output/parcel_corrected.gpkg",
    remediation_queue_csv="output/parcel_remediation_queue.csv",
    html_report="output/parcel_compliance_report.html",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
