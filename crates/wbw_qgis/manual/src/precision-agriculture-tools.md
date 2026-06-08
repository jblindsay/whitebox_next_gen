# General Tools


---

## Field Trafficability And Operation Planning

**Function name:** `field_trafficability_and_operation_planning`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Field Trafficability and Operation Planning**

#### Problem It Solves

Where can equipment operate safely now, and which areas should be delayed or rerouted?

#### Who It Is For

- Machinery planning teams, farm managers, and field operations coordinators.

#### Primary User

Farm operations teams and precision agriculture service providers.

#### What It Does

- Scores field trafficability and operation timing risk from terrain and saturation context.
- Produces operation classes for go/hold/reroute-style field execution decisions.

#### How It Works

- Derives slope from DEM and harmonizes moisture/rainfall context to the same grid.
- Blends terrain, soil saturation, and rainfall-risk penalties into trafficability scores.
- Converts continuous scores into operation classes for practical planning use.
- QA acceptance guidance:
- `status=pass` indicates low-trafficability burden stayed within baseline tolerance.
- `diagnostics.acceptance_thresholds.low_trafficability_fraction_review` defines review escalation threshold.
- High `summary.low_trafficability_fraction` should trigger cautious equipment scheduling and field confirmation.
- MVP hardening assets:
- Benchmark scaffold: `tests/fixtures/precision_ag_ops_benchmark/`
- Promotion guide: `docs/internal/development/TERRAIN_PRECISION_AG_BENCHMARK_PROMOTION_GUIDE_2026_04_14.md`

### Inputs

ParameterOptionalDescription
demnoDEM raster for terrain slope context.
soil_moisturenoSoil saturation/moisture raster normalized to [0,1].
rainfall_forecastyesOptional rainfall-risk raster normalized to [0,1].

### Outputs

ParameterTypeDescription
trafficability_scoreGeoTIFFContinuous trafficability score [0,1] (higher is better).
operation_classGeoTIFFDiscrete operation class raster (1 favorable to 4 poor).
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

traffic, op_class, summary = wbe.field_trafficability_and_operation_planning(
    dem="data/dem.tif",
    soil_moisture="data/soil_saturation.tif",
    rainfall_forecast="data/rain_risk.tif",
    output_prefix="output/field_trafficability",
)

print(traffic)
print(op_class)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## In Season Crop Stress Intervention Planning

**Function name:** `in_season_crop_stress_intervention_planning`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**In-Season Crop Stress Intervention Planning**

#### Problem It Solves

Where should interventions be prioritized this week to limit stress-driven yield loss?

#### Who It Is For

- Agronomy advisors and farm operations teams managing in-season response.

#### Primary User

Precision agronomy service providers and large production farms.

#### What It Does

- Prioritizes intervention zones from in-season crop stress indicators.
- Combines vigor decline with optional thermal and moisture stress context.

#### How It Works

- Uses NDVI/vigor deficit as the primary stress signal.
- Harmonizes optional canopy-temperature and soil-moisture rasters to the NDVI grid.
- Produces intervention-priority and intervention-class surfaces.
- QA acceptance guidance:
- `status=pass` indicates intervention burden is below review threshold.
- `diagnostics.acceptance_thresholds.high_priority_fraction_review` defines escalation threshold for broad intervention risk.
- High `summary.high_priority_fraction` should trigger stress-source verification before large-scale treatment rollout.
- MVP hardening assets:
- Benchmark scaffold: `tests/fixtures/precision_ag_ops_benchmark/`
- Promotion guide: `docs/internal/development/TERRAIN_PRECISION_AG_BENCHMARK_PROMOTION_GUIDE_2026_04_14.md`

### Inputs

ParameterOptionalDescription
ndvinoNDVI/vigor raster normalized to [0,1].
canopy_temperatureyesOptional thermal-stress raster normalized to [0,1].
soil_moistureyesOptional moisture-deficit raster normalized to [0,1].

### Outputs

ParameterTypeDescription
intervention_priorityGeoTIFFContinuous intervention-priority score [0,1].
intervention_classGeoTIFFDiscrete intervention class raster (1 low to 4 urgent).
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

priority, classes, summary = wbe.in_season_crop_stress_intervention_planning(
    ndvi="data/ndvi_current.tif",
    canopy_temperature="data/canopy_temp_stress.tif",
    soil_moisture="data/moisture_deficit.tif",
    output_prefix="output/in_season_stress",
)

print(priority)
print(classes)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Precision Irrigation Optimization

**Function name:** `precision_irrigation_optimization`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Precision Irrigation Optimization**

#### Problem It Solves

Where should irrigation depth be increased or reduced to achieve target moisture with less water waste?

#### Who It Is For

- Precision agriculture teams, irrigation planners, and agronomy analytics groups.

#### Primary User

Large growers, precision ag service providers, and irrigation technology partners.

#### What It Does

- Generates variable-rate irrigation prescription depth from terrain and moisture context.
- Estimates moisture stress risk to prioritize intervention zones.
- Emits summary diagnostics suitable for irrigation planning dashboards.

#### How It Works

- Estimates local moisture deficit from target_moisture and measured or terrain-inferred moisture.
- Adjusts prescription depth by slope-derived terrain factor and max_irrigation_mm limits.
- Converts prescribed depth to VRI zones and computes moisture-stress risk index.
- Indicative formula: prescribed_mm = max(0, target - moisture) * max_irrigation_mm * terrain_factor.

#### Why It Wins

- Produces direct raster-ready prescription depths and risk diagnostics from reproducible inputs.

#### Typical Buying Trigger

A grower or consultant needs to shift from uniform irrigation to variable-rate execution.

#### Typical Presets

- fast for quick field-scale recommendations.
- balanced for default VRI planning.
- conservative for stronger terrain-risk penalties.

### Inputs

ParameterOptionalDescription
demnoDigital elevation model used as the terrain reference surface.
optional soil_moistureyesOptional soil moisture raster used to refine irrigation need and stress scoring.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.
target_moisturenoTarget moisture level used to compute irrigation prescription amounts.
max_irrigation_mmnoMaximum irrigation depth allowed in generated irrigation recommendations.

### Outputs

ParameterTypeDescription
irrigation_prescriptionGeoTIFFIrrigation prescription raster indicating recommended application depth.
moisture_stress_riskGeoTIFFMoisture stress risk raster supporting irrigation prioritization.
vri_zonesGeoTIFFVariable-rate irrigation zone raster.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

When `sweep_spec` is supplied, the workflow also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map`. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`), while `stability_map` uses classes `3=high`, `2=medium`, `1=low`.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

prescription, stress, zones, summary = wbe.precision_irrigation_optimization(
    dem="data/dem.tif",
    soil_moisture="data/soil_moisture.tif",
    profile="balanced",
    target_moisture=0.6,
    max_irrigation_mm=18.0,
    output_prefix="output/irrigation",
)

print(prescription)
print(stress)
print(zones)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Precision Ag Yield Zone Intelligence

**Function name:** `precision_ag_yield_zone_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Precision Ag Yield Zone Analysis**

#### Problem It Solves

Which management zones should receive differentiated input strategy based on stable productivity patterns?

#### Who It Is For

- Precision agronomy teams and farm analytics providers.

#### Primary User

Enterprise farms, agronomy consultancies, and digital ag platforms.

#### What It Does

- Builds yield-stability surfaces from yield productivity context.
- Integrates optional terrain context to improve zone differentiation.
- Produces management-zone rasters for variable-rate strategy design.

#### How It Works

- Normalizes yield response and computes local stability/variability signatures.
- Optionally blends terrain_context influence with profile-dependent weighting.
- Partitions pixels into zone_count management classes and emits zone confidence and polygons.
- Indicative formula: zone_score ~= w_y*yield_stability + w_t*terrain_term; zone = discretize(zone_score, zone_count).

#### Why It Wins

- Converts yield and terrain context into explicit, contract-backed management zone outputs.

#### Typical Buying Trigger

A farm program is formalizing zone-based management for seed, fertility, and water decisions.

#### Typical Presets

- fast for high-throughput zone generation.
- balanced for default zone planning.
- conservative for stronger terrain-context influence.

### Inputs

ParameterOptionalDescription
yield_surfacenoYield raster used to estimate productivity stability and zone segmentation.
optional terrain_contextyesOptional terrain context raster used to strengthen management-zone differentiation.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.
zone_count (2..8)noRequested number of management zones to generate.

### Outputs

ParameterTypeDescription
yield_stabilityGeoTIFFYield stability raster used to delineate management zones.
management_zonesGeoTIFFManagement-zone classification raster.
management_zones_vectorGeoPackageVectorized management zones for operations and reporting.
zone_confidenceGeoTIFFConfidence surface associated with generated management zones.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

When `sweep_spec` is supplied, the workflow also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map`. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`), while `stability_map` uses classes `3=high`, `2=medium`, `1=low`.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

stability, zones, zone_polys, confidence, summary = wbe.precision_ag_yield_zone_intelligence(
    yield_surface="data/yield_surface.tif",
    terrain_context="data/terrain_context.tif",
    profile="balanced",
    zone_count=4,
    max_zone_features=5000,
    output_prefix="output/yield_zone",
)

print(stability)
print(zones)
print(zone_polys)
print(confidence)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Soil Landscape Classification

**Function name:** `soil_landscape_classification`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Soil Landscape Classification**

#### Problem It Solves

Which terrain-defined landform units dominate the area, and how can they guide management zoning or sampling strategy?

#### Who It Is For

- Geomorphometry analysts, soil-landscape researchers, and precision-ag planning teams.

#### Primary User

Agronomy/land capability groups, environmental consulting teams, and land resource agencies.

#### What It Does

- Classifies landform units using multiscale curvature and slope signatures.
- Produces raster class maps and optional polygon outputs.
- Emits summary distributions for each landform class.

#### How It Works

- Computes slope, profile curvature, and plan curvature at fine and coarse scales.
- Applies rule-based landform assignment thresholds to each pixel.
- Optionally polygonizes class regions and aggregates class-area summary statistics.
- Indicative formula: landform_class = rule(slope, k_profile, k_plan, multiscale_signature).

#### Why It Wins

- Couples interpretable geomorphometric classes with optional polygon outputs for direct planning integration.

#### Typical Buying Trigger

A land management or agronomy program needs terrain-driven zones for sampling design or variable-rate planning.

#### Typical Presets

- default thresholds for general terrain partitioning.
- tune fine/coarse scales for local vs regional landform emphasis.

### Inputs

ParameterOptionalDescription
input DEMnoInput DEM used for terrain-derivative and landform classification workflows.
flat/profile/plan thresholdsnoCurvature and surface-form thresholds used to separate landform classes.
fine_scale, coarse_scalenoMultiscale analysis windows used to capture local and broad terrain structure.
optional landform_polygons_outputyesOptional switch/path to emit vectorized landform polygons.

### Outputs

ParameterTypeDescription
landform_unitsGeoTIFFCategorical landform unit raster from terrain analysis.
multiscale_signatureGeoTIFFMultiscale terrain-signature raster for landscape interpretation.
landform_polygonsoptional vectorOptional vectorized landform polygons for cartographic/reporting use.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

landform, signature, polygons, summary = wbe.soil_landscape_classification(
    input="data/dem.tif",
    fine_scale=2.0,
    coarse_scale=8.0,
    output_prefix="output/soil_landscape",
    landform_polygons_output="output/landforms.gpkg",
)

print(landform)
print(signature)
print(polygons)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Yield Data Conditioning And QA

**Function name:** `yield_data_conditioning_and_qa`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Yield Data Conditioning and QA**

#### Problem It Solves

How do we turn raw, noisy harvest monitor points into defensible, analysis-ready yield products for management decisions?

#### Who It Is For

- Precision-ag service providers, agronomy analytics teams, and farm data engineering groups.

#### Primary User

Enterprise farms, precision agriculture consultancies, and digital agronomy platforms.

#### What It Does

- Runs an end-to-end conditioning pipeline for noisy combine-harvester yield points.
- Orchestrates edge QA, pass reconstruction, optional multi-header reconciliation, filtering, normalization, and swath map generation.
- Produces contract-ready summary outputs for auditability and downstream zoning workflows.

#### How It Works

- Flags/removes likely edge points using local neighborhood support.
- Optionally filters telemetry anomalies using speed and heading consistency thresholds.
- Auto-resolves common monitor-export field aliases when requested names are missing.
- Reconstructs pass structure from point geometry and heading continuity, then smooths yield using pass-aware neighborhood statistics.
- Optionally applies distance-based lag correction along reconstructed passes to compensate harvest monitor delay.
- Optionally converts raw yield to a target moisture basis using moisture_field_name and target_moisture_pct.
- Optionally applies robust MAD-based clipping on filtered yield values before normalization.
- Propagates a QA confidence score to final point outputs, then builds swath polygons for map-ready products.
- Indicative formula: filtered_yield ~= weighted_neighborhood(yield) with outlier replacement when z = |y - mean_adjacent_pass| / sd_adjacent_pass exceeds threshold.

#### Why It Wins

- Provides a reproducible and transparent preprocessing chain with explicit QA outputs rather than opaque one-click cleaning.

#### Typical Buying Trigger

A team needs to standardize inconsistent yield data cleaning before zone generation, variable-rate planning, or year-over-year analytics.

#### Typical Presets

- fast: lighter cleanup with larger tolerances for rapid turnaround.
- balanced: default production profile.
- strict: stronger edge/outlier controls for high-confidence analytics.

### Inputs

ParameterOptionalDescription
input (yield point vector)noRaw yield telemetry points used as input to the conditioning and QA pipeline.
yield_field_name, moisture_field_name, target_moisture_pct, header_field_namenoField-name mappings for yield/moisture/header attributes used by cleaning stages.
use_field_aliasesnoWhether known attribute aliases should be resolved automatically.
speed_field_name, heading_field_namenoTelemetry field mappings used for speed and heading quality checks.
min_speed_kmh, max_speed_kmh, max_heading_change_degnoOperating-speed and heading-change limits used for telemetry QC filtering.
profile: fast | balanced | strictnoOperational profile controlling sensitivity and QA strictness for risk workflows.
swath_width, edge_radius, reconcile_radius, normalization_radiusnoGeometric parameters controlling swath-edge handling and neighborhood reconciliation.
lag_correction_mode: none | distancenoLag correction mode controlling whether and how harvest lag compensation is applied.
lag_distance_mnoDistance offset used when distance-based lag correction is enabled.
filtering_mode: standard | robustnoOutlier-filtering method selection for yield cleaning.
robust_mad_threshold, z_score_threshold, min_yield, max_yieldnoStatistical thresholds and hard limits used for yield outlier rejection.
optional mean_tonnageyesOptional mean tonnage override used during normalization/reconciliation steps.

### Outputs

ParameterTypeDescription
qa_flagsGeoPackageQA flag layer identifying records that failed yield telemetry checks.
telemetry_qc_pointsoptional GeoPackageOptional point layer of telemetry QC diagnostics by record.
clean_pointsGeoPackageCleaned yield points after telemetry and statistical filtering.
clean_mapGeoPackageCartography-ready cleaned yield layer for rapid map production.
confidence_pointsGeoPackagePoint-level confidence diagnostics for cleaned yield records.
pass_linesGeoPackageHarvest pass line features inferred from telemetry trajectories.
pass_pointsGeoPackageHarvest pass points used in overlap and reconciliation analyses.
lag_corrected_pointsoptional GeoPackageOptional points after lag-correction adjustment.
moisture_adjusted_pointsoptional GeoPackageOptional points after moisture normalization adjustments.
filtered_pointsGeoPackagePoints retained after standard filtering criteria are applied.
robust_filtered_pointsoptional GeoPackageOptional points retained by robust filtering mode diagnostics.
normalized_pointsGeoPackageYield points after normalization to common analytical basis.
reconciled_pointsoptional GeoPackageOptional reconciled points after overlap and pass harmonization.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.yield_data_conditioning_and_qa(
    input="data/yield_points.gpkg",
    yield_field_name="YIELD",
    header_field_name="HEADER",
    profile="balanced",
    swath_width=6.096,
    output_prefix="output/yield_pipeline",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
