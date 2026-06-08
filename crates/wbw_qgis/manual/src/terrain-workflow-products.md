# Workflow Products


---

## Topo Render

**Function name:** `topo_render`


PROExperimental

Creates a pseudo-3D topographic rendering using palette tinting, hillshade, shadows, and attenuation.

geomorphometry terrain rendering topographic

### Parameters

NameDescriptionRequiredDefault
`dem`Input DEM raster path.Required`dem.tif`
`palette`Palette name (soft, atlas, high_relief, turbo, viridis, dem, grey, white).Optional`soft`
`reverse_palette`Reverse palette order.Optional`False`
`azimuth`Light-source azimuth in degrees [0, 360].Optional`315.0`
`altitude`Light-source altitude in degrees [0, 90].Optional`30.0`
`clipping_polygon`Optional polygon vector path; only DEM cells inside polygon(s) are rendered.Optional—
`background_hgt_offset`Vertical offset from minimum DEM elevation to background plane.Optional`10.0`
`background_clr`Background RGBA colour as array [r,g,b,a].Optional`[255, 255, 255, 255]`
`attenuation_parameter`Distance attenuation exponent (>= 0).Optional`0.3`
`ambient_light`Ambient light amount in [0, 1].Optional`0.2`
`z_factor`Vertical exaggeration multiplier.Optional`1.0`
`max_dist`Maximum shadow search distance in map units.Optional—
`output`Output raster path.Optional`topo_render.tif`

### Examples

*Generate a pseudo-3D topographic render from a DEM.*
`wbe.topo_render(altitude=30.0, azimuth=315.0, dem='dem.tif', output='topo_render.tif', palette='soft')`


---

## Wetland Hydrogeomorphic Classification

**Function name:** `wetland_hydrogeomorphic_classification`


PROProduction

Classify wetlands into hydrogeomorphic classes using DEM context and wetland masks.

workflow pro

### Workflow Narrative

**Wetland Hydrogeomorphic Classification**

#### Problem It Solves

Which mapped wetland regions belong to key HGM classes, and where is class confidence high enough for permitting decisions?

#### Who It Is For

- Wetland scientists, permitting consultants, and mitigation planners.

#### Primary User

Environmental consulting firms, permitting agencies, and mitigation banking organizations.

#### What It Does

- Classifies wetland mask cells into HGM classes using terrain context.
- Builds confidence surfaces for wetland classification reliability.
- Polygonizes connected wetland regions with region-level attributes.

#### How It Works

- Computes local terrain signatures in wetland-mask cells from DEM gradients and relief context.
- Assigns HGM class codes using rule-based thresholds over those signatures.
- Aggregates connected components into polygons and summarizes dominant class and mean confidence.
- Indicative formula: class = rule(slope, relief, wetness_proxy), confidence from distance-to-threshold stability.

#### Why It Wins

- Produces connected-region polygons with explicit confidence and class attributes, not just cell-level labels.

#### Typical Buying Trigger

Permitting or mitigation workflows require polygon deliverables with defensible classification metadata.

#### Typical Presets

- default for full region polygon extraction.
- lower max_polygon_features for very large AOIs.

### Inputs

ParameterOptionalDescription
dem, wetland_masknoDEM and wetland mask defining candidate wetland extent and hydrogeomorphic context.
max_polygon_featuresnoMaximum number of polygon features to emit for mapped wetland units.

### Outputs

ParameterTypeDescription
hgm_classGeoTIFFCategorical hydrogeomorphic wetland class raster.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
wetland_polygonsGeoPackageVectorized wetland class polygons for mapping and reporting.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

hgm, conf, polys, summary = wbe.wetland_hydrogeomorphic_classification(
    dem="data/dem.tif",
    wetland_mask="data/wetlands.tif",
    max_polygon_features=10000,
    output_prefix="output/wetland_hgm",
)

print(hgm)
print(conf)
print(polys)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Urban Expansion Impact Assessment

**Function name:** `urban_expansion_impact_assessment`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Urban Expansion Impact Assessment**

#### Problem It Solves

What ecological and stream-network impacts should we expect under this growth scenario, and where are priorities for mitigation?

#### Who It Is For

- Urban planners, environmental assessors, and watershed impact analysts.

#### Primary User

Municipal planning departments, environmental consultancies, and stormwater authorities.

#### What It Does

- Quantifies expansion-driven impact severity from baseline/scenario urban surfaces.
- Derives habitat-loss raster products from change footprint logic.
- Scores stream features against impact raster exposure.

#### How It Works

- Builds urban change footprint by differencing scenario and baseline urban rasters.
- Translates new/expanded footprint intensity into impact and habitat-loss scores.
- Samples stream geometries against impact surfaces to compute attributed reach-level metrics.
- Indicative formula: change = max(0, urban_scenario - urban_baseline); impact ~= change * habitat_weight.

#### Why It Wins

- Links scenario change, habitat loss, and attributed stream impact in one report-ready package.

#### Typical Buying Trigger

Planning approvals require quantified environmental impact evidence under multiple development scenarios.

#### Typical Presets

- baseline/scenario-only impact analysis.
- add habitat_sensitivity for weighted ecological impact scoring.

### Inputs

ParameterOptionalDescription
baseline_urban, scenario_urban, streamsnoBaseline and scenario urban rasters plus stream network used for impact assessment.
optional habitat_sensitivityyesOptional habitat sensitivity layer used to weight ecological impact severity.

### Outputs

ParameterTypeDescription
impact_severityGeoTIFFImpact severity raster for baseline-versus-scenario urban change.
habitat_lossGeoTIFFRaster estimate of habitat loss intensity under scenario comparison.
affected_streamsGeoPackageVector stream segments flagged as impacted under scenario conditions.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

impact, habitat, streams, summary = wbe.urban_expansion_impact_assessment(
    baseline_urban="data/urban_2020.tif",
    scenario_urban="data/urban_2035.tif",
    streams="data/streams.gpkg",
    habitat_sensitivity="data/habitat_sensitivity.tif",
    output_prefix="output/urban_impact",
)

print(impact)
print(habitat)
print(streams)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Wind Turbine Siting

**Function name:** `wind_turbine_siting`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Wind Turbine Siting Analysis**

#### Problem It Solves

Which candidate areas are most promising for wind siting at early-stage screening, and how confident are those rankings?

#### Who It Is For

- Renewable energy siting teams and feasibility analysts.

#### Primary User

Wind developers, utility planning groups, and engineering consultancies.

#### What It Does

- Creates suitability and confidence surfaces for wind siting screening.
- Combines slope constraints, terrain exposure, and settlement-visibility signals.
- Supports profile-based scoring behavior for speed vs quality tradeoffs.

#### How It Works

- Derives slope and terrain-exposure factors from the DEM.
- Applies distance/visibility penalties around settlement inputs.
- Combines normalized factors with profile-dependent weights into siting score and confidence.
- Indicative formula: score ~= w_s*terrain_suitability + w_e*exposure - w_v*visibility_penalty.

#### Why It Wins

- Combines terrain and visibility constraints with confidence scoring to support transparent shortlist decisions.

#### Typical Buying Trigger

A development team needs to narrow a broad search region before expensive met mast or field campaigns.

#### Typical Presets

- fast for broad regional pre-screening.
- balanced for standard feasibility support.
- quality for higher-confidence candidate ranking.

### Inputs

ParameterOptionalDescription
dem, settlementsnoTerrain model and settlement features used for slope/visibility siting constraints.
visibility_radius_metersnoMaximum visibility analysis radius used during visual-impact screening.
min_slope_degrees, max_slope_degreesnoSlope suitability bounds used to constrain turbine placement candidates.
profile: fast | balanced | qualitynoSiting profile controlling screening speed versus quality/strictness of constraints.

### Outputs

ParameterTypeDescription
siting_scoreGeoTIFFCore siting suitability score raster produced by the model.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

When `sweep_spec` is supplied, the workflow also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map`. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`), while `stability_map` uses classes `3=high`, `2=medium`, `1=low`.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

score, conf, summary = wbe.wind_turbine_siting(
    dem="data/dem.tif",
    settlements="data/settlements.gpkg",
    profile="balanced",
    output_prefix="output/wind_siting",
)

print(score)
print(conf)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Solar Site Suitability Analysis

**Function name:** `solar_site_suitability_analysis`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Solar Site Suitability Analysis**

#### Problem It Solves

Where are top solar candidates based on terrain suitability, and which shortlisted sites are strong enough for deeper engineering review?

#### Who It Is For

- Solar development teams and regional screening analysts.

#### Primary User

Solar developers, renewable consultants, and utility planning units.

#### What It Does

- Computes solar siting suitability and visual-impact proxies from terrain.
- Selects and ranks candidate point sites.
- Emits attributed candidate vectors for downstream review.

#### How It Works

- Computes terrain suitability response from slope/aspect and neighborhood context.
- Estimates visual-impact proxy from terrain prominence and local exposure contrasts.
- Applies thresholding and ranking to emit top candidate points with attributes.
- Indicative formula: suitability ~= f(slope, aspect, relief); candidates = arg top-k(suitability) above threshold.

#### Why It Wins

- Emits ranked candidate vectors with visual-impact attributes, enabling immediate GIS review and filtering.

#### Typical Buying Trigger

Early project screening demands a short list of high-probability solar candidates across a large area.

#### Typical Presets

- higher candidate_threshold for only top candidates.
- lower threshold plus larger max_candidate_sites for exploratory workflows.

### Inputs

ParameterOptionalDescription
demnoDigital elevation model used as the terrain reference surface.
candidate_thresholdnoMinimum suitability threshold required for candidate-site extraction.
max_candidate_sitesnoUpper limit on the number of candidate sites emitted in vector output.

### Outputs

ParameterTypeDescription
suitability_scoreGeoTIFFCore suitability score raster produced by the model.
visual_impactGeoTIFFVisual-impact raster used to screen candidate sites and stakeholder constraints.
candidate_sitesGeoPackageVector candidate-site features passing selection thresholds.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

When `sweep_spec` is supplied, the workflow also emits `run_matrix_summary`, `sensitivity_report`, `sensitivity_report_html`, and `stability_map`. The sensitivity report includes `metrics.primary_metric`, `metrics.primary_relative_span`, and `metrics.stability_class` (`high`, `medium`, `low`), while `stability_map` uses classes `3=high`, `2=medium`, `1=low`.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

suit, vis, sites, summary = wbe.solar_site_suitability_analysis(
    dem="data/dem.tif",
    candidate_threshold=0.7,
    max_candidate_sites=200,
    output_prefix="output/solar_siting",
)

print(suit)
print(vis)
print(sites)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Corridor Mapping Intelligence

**Function name:** `corridor_mapping_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Corridor Mapping and Route Planning**

#### Problem It Solves

What is the terrain-optimal route for this linear infrastructure, and what alternative corridor band exists within an acceptable cost margin?

#### Who It Is For

- Infrastructure planners, environmental consultants, and natural resource agencies assessing route options for roads, pipelines, or utility lines.

#### Primary User

Forestry companies, energy utilities, and environmental regulatory consultancies evaluating new linear infrastructure corridors.

#### What It Does

- Builds a terrain impedance cost surface from DEM-derived slope and roughness.
- Finds the least-cost path between two user-specified endpoints using Dijkstra accumulation.
- Delineates a corridor suitability band of near-optimal alternative routes.
- Supports polygon exclusion zones (steep terrain, protected areas, existing infrastructure).

#### How It Works

- Computes per-cell cost from slope (and optionally local roughness) normalised to [0, 1].
- Runs Dijkstra from both start and end to accumulate cost surfaces.
- Least-cost path traced back from end → start through predecessor pointers.
- Corridor band: cells where `acc_from_start + acc_from_end ≤ optimal_cost × (1 + tolerance)`.
- Indicative formula: cost ~= w_slope × slope_norm + w_rough × roughness_norm (profile-controlled weights).

#### Why It Wins

- Compared with OSS least-cost building blocks (`cost_distance` + `cost_pathway` + `cost_allocation`), this tool is an end-to-end siting workflow: it derives the terrain cost surface from the DEM, computes the optimal route, delineates near-optimal corridor alternatives, and packages decision-ready outputs in one run.
- Why it wins vs OSS least-cost tools:
- Requires fewer analyst preparation steps (no separate friction-raster engineering pipeline required).
- Returns vector route geometry with engineering-friendly attributes, not just a path raster.
- Produces a corridor suitability band for option-space review, not only a single least-cost trace.
- Supports polygon exclusion constraints directly in the routing workflow.
- Emits a machine-readable summary contract for reproducible QA/reporting integration.

#### Typical Buying Trigger

A feasibility study needs a first-pass route alignment and suitability band for stakeholder review before field surveys.  Cost profiles: - `slope_only` — slope-only impedance; fastest, suitable for quick screening. - `slope_roughness` — balanced slope + roughness blend (default); recommended for road and pipeline siting. - `conservative` — equal slope/roughness weighting; preferred for sensitive terrain or pipelines.

### Inputs

ParameterOptionalDescription
demnoInput DEM raster path.
start_featuresnoStart feature vector path (point or polygon).
end_featuresnoEnd feature vector path (point or polygon).
constraintsyesOptional exclusion zone vector path (polygons to avoid).
cost_profileyesCost weighting profile: `slope_only` | `slope_roughness` | `conservative`. Default: `slope_roughness`.
terminal_anchor_strategyyesPolygon terminal anchor mode: `mixed` | `centroid_only` | `boundary_only`. Default: `mixed`.
corridor_toleranceyesFraction above optimal cost included in corridor suitability band. Default: `0.15`.
output_prefixyesOutput prefix for generated artifacts.

### Outputs

ParameterTypeDescription
cost_surfaceGeoTIFFNormalized terrain impedance surface [0-1].
accumulated_costGeoTIFFDijkstra accumulated cost from selected start anchor.
optimal_routeGeoPackageLeast-cost route LineString with route metrics.
corridor_suitabilityGeoTIFFBinary suitability band (`1` = within tolerance).
summaryJSONSummary contract with metrics, selected anchors, and harmonization metadata.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

cost, acc_cost, route, suitability, summary = wbe.corridor_mapping_intelligence(
    dem="data/dem.tif",
    start_features="data/start_access_points.gpkg",
    end_features="data/forest_block_targets.gpkg",
    cost_profile="slope_roughness",
    terminal_anchor_strategy="mixed",
    corridor_tolerance=0.15,
    output_prefix="output/access_road",
)

print(route)       # path to optimal_route.gpkg
print(suitability) # path to corridor suitability raster
print(summary)     # path to summary JSON`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Landslide Susceptibility Assessment

**Function name:** `landslide_susceptibility_assessment`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Landslide Susceptibility Assessment**

#### Problem It Solves

Which areas present elevated slope-failure risk today, and where is trigger pressure most concerning?

#### Who It Is For

- Hazard analysts, corridor planners, and geotechnical screening teams.

#### Primary User

Geological surveys, transportation agencies, and hazard consulting teams.

#### What It Does

- Estimates terrain-driven landslide susceptibility from slope/curvature context.
- Integrates optional rainfall pressure to strengthen trigger interpretation.
- Produces susceptibility, trigger-pressure, and confidence rasters with contract summary diagnostics.

#### How It Works

- Computes local slope and roughness/curvature proxies from DEM neighborhood derivatives.
- Blends terrain susceptibility with optional rainfall intensity to form trigger pressure.
- Applies profile-weighted scaling and thresholding to output susceptibility, confidence, and risk zones.
- Indicative formula: susceptibility ~= w1*slope_term + w2*roughness_term; trigger ~= susceptibility * (1 + rainfall_term).

#### Why It Wins

- Provides reproducible hazard screening outputs with structured summary metrics for downstream reporting.

#### Typical Buying Trigger

A public or infrastructure program requires defensible first-pass landslide risk zoning.

#### Typical Presets

- fast for rapid regional screening.
- balanced for standard hazard screening.
- conservative for stricter slope/curvature sensitivity.

### Inputs

ParameterOptionalDescription
demnoDigital elevation model used as the terrain reference surface.
optional rainfall_intensityyesOptional rainfall forcing raster used to refine landslide trigger pressure estimation.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.
susceptibility_thresholdnoThreshold used to convert continuous susceptibility into risk-zone candidates.

### Outputs

ParameterTypeDescription
susceptibilityGeoTIFFLandslide susceptibility raster used to identify unstable terrain.
trigger_pressureGeoTIFFTrigger-pressure raster indicating forcing required to initiate failures.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
risk_zonesGeoPackagePriority vector polygons highlighting intervention zones.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

sus, trig, conf, zones, summary = wbe.landslide_susceptibility_assessment(
    dem="data/dem.tif",
    rainfall_intensity="data/rainfall_intensity.tif",
    profile="balanced",
    susceptibility_threshold=0.65,
    max_zone_features=5000,
    output_prefix="output/landslide",
)

print(sus)
print(trig)
print(conf)
print(zones)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## River Corridor Health Assessment

**Function name:** `river_corridor_health_assessment`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**River Corridor Health Assessment**

#### Problem It Solves

Which river reaches are stable versus at-risk, and where should restoration action be prioritized?

#### Who It Is For

- Watershed restoration teams, river health analysts, and permit support consultants.

#### Primary User

Watershed councils, conservation authorities, and environmental consulting groups.

#### What It Does

- Derives erosion-pressure context from terrain.
- Scores stream reaches by sampled corridor pressure.
- Emits restoration-priority line outputs for intervention planning.

#### How It Works

- Calculates per-pixel erosion pressure from slope and local roughness terms.
- Samples stream vertices over the erosion raster and derives reach metrics (including high quantiles).
- Converts reach scores into health classes and restoration action categories.
- Indicative formula: erosion ~= a*slope_norm + b*roughness_norm; health ~= 1 - mean(erosion_samples_along_reach).

#### Why It Wins

- Combines raster pressure context and attributed reach-level restoration outputs in one workflow.

#### Typical Buying Trigger

A watershed plan needs rapid reach triage with GIS-ready outputs for restoration budgeting.

#### Typical Presets

- fast for rapid watershed screening.
- balanced for default corridor scoring.
- conservative for roughness-sensitive erosion scoring.

### Inputs

ParameterOptionalDescription
demnoDigital elevation model used as the terrain reference surface.
streamsnoVector stream network used for corridor health and restoration targeting.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.

### Outputs

ParameterTypeDescription
erosion_pressureGeoTIFFErosion pressure raster for river corridor condition assessment.
corridor_confidenceGeoTIFFConfidence surface for river corridor health interpretation.
stream_health_scoreGeoPackageStream-reach health score output summarizing corridor condition.
restoration_zonesGeoPackagePriority vector polygons highlighting restoration intervention zones.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

erosion, confidence, health, restoration, summary = wbe.river_corridor_health_assessment(
    dem="data/dem.tif",
    streams="data/streams.gpkg",
    profile="balanced",
    output_prefix="output/river_health",
)

print(erosion)
print(confidence)
print(health)
print(restoration)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Baseline Matching And Diagnostics Assessment

**Function name:** `baseline_matching_and_diagnostics_assessment`


*No help documentation available for this tool.*


---

## Carbon Sequestration Verification Audit

**Function name:** `carbon_sequestration_verification_audit`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Carbon Verification Audit**

#### Problem It Solves

Where did vegetation-linked carbon proxy increase or decrease, and where is confidence high enough for audit triage?

#### Who It Is For

- Carbon program analysts, forest monitoring teams, and environmental verification workflows.

#### Primary User

Carbon project developers, ESG/MRV teams, and land-management compliance programs.

#### What It Does

- Quantifies baseline-to-current vegetation change using NDVI deltas.
- Derives a carbon-proxy change surface with confidence scoring.
- Optionally blends LiDAR biomass proxy inputs to strengthen stand-level interpretation.
- Produces verification zone polygons and an audit-ready contract output for MRV workflows.

#### How It Works

- Extracts baseline and current red/NIR bands from multiband bundles.
- Computes per-date NDVI and signed delta: NDVI_current - NDVI_baseline.
- Converts NDVI delta to a carbon-proxy index and computes confidence from vegetation signal and change magnitude.
- Optionally blends in biomass proxy values to improve relative stand-level carbon interpretation.
- Aggregates pixels into verification blocks and assigns zone-level change class labels (`gain`, `loss`, `unchanged`).
- Indicative formula: NDVI = (NIR - Red) / (NIR + Red), carbon_proxy ~= 10 * (NDVI_current - NDVI_baseline).

#### Why It Wins

- Combines change mapping, confidence scoring, zone-level vector outputs, and an audit contract in one reproducible run.

#### Typical Buying Trigger

Teams need standardized remote-sensing evidence packages ahead of formal carbon verification reporting.

#### Typical Presets

- conservative for stricter gain/loss interpretation thresholds.
- balanced for default operational monitoring.
- aggressive for early-signal monitoring workflows.

### Inputs

ParameterOptionalDescription
baseline_bundle, current_bundlenoBaseline and current multiband rasters used for NDVI change analysis.
baseline_red_band_index, baseline_nir_band_indexyesBaseline red/NIR band indices. Defaults: 0, 1.
current_red_band_index, current_nir_band_indexyesCurrent red/NIR band indices. Defaults: 0, 1.
biomass_proxyyesOptional LiDAR-derived biomass proxy raster for blended carbon-proxy interpretation.
profile: conservative | balanced | aggressiveyesSensitivity profile controlling gain/loss thresholds and confidence blending behavior.
zone_block_cellsyesPixel block size used for verification zone polygon aggregation. Defaults to `16`.
mrv_templateyesMRV profile: verra_vcs_vm0010, american_carbon_registry, gold_standard, or none.
methodology_referenceyesOptional methodology lineage note for audit metadata (for example Verra references).
output_prefixyesPrefix used to name output artifacts.

### Outputs

ParameterTypeDescription
ndvi_baselineGeoTIFFBaseline NDVI surface (`*_ndvi_baseline.tif`).
ndvi_currentGeoTIFFCurrent NDVI surface (`*_ndvi_current.tif`).
ndvi_deltaGeoTIFFSigned NDVI change surface (`*_ndvi_delta.tif`).
carbon_proxyGeoTIFFCarbon-proxy change index (`*_carbon_proxy.tif`).
change_confidenceGeoTIFFConfidence raster for change interpretation (`*_change_confidence.tif`).
verification_zonesGeoPackageBlock-aggregated verification polygons with change class and summary attributes (`*_verification_zones.gpkg`).
audit_contractJSONMachine-readable audit summary contract (`*_audit_contract.json`).
compliance_evidence_packetJSONSubmission-oriented compliance evidence packet (`*_compliance_evidence_packet.json`).
regulator_ready_tableCSVFlat regulator-ready summary table (`*_regulator_ready_table.csv`).
html_reportHTMLHuman-readable report generated from the contract for stakeholder review.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.carbon_sequestration_verification_audit(
    baseline_bundle="data/baseline_multiband.tif",
    current_bundle="data/current_multiband.tif",
    baseline_red_band_index=0,
    baseline_nir_band_index=1,
    current_red_band_index=0,
    current_nir_band_index=1,
    biomass_proxy="data/biomass_proxy.tif",
    profile="balanced",
    zone_block_cells=16,
    mrv_template="verra_vcs_vm0010",
    methodology_reference="Verra VM0010 v1.3",
    output_prefix="output/carbon_audit",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Wildfire Fuel Loading And Risk Matrix

**Function name:** `wildfire_fuel_loading_and_risk_matrix`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Wildfire Fuel Risk Analysis**

#### Problem It Solves

Where are the highest-priority fuel and spread-risk areas, and what dominant fuel conditions drive those zones?

#### Who It Is For

- Wildfire planning teams, utility risk programs, and fuels management analysts.

#### Primary User

Utility wildfire mitigation teams, land-management agencies, and hazard/risk operations groups.

#### What It Does

- Classifies sparse, surface, ladder, and canopy fuel classes from optical and optional structure inputs.
- Computes moisture index and ladder-fuel continuity diagnostics.
- Builds a terrain-amplified wildfire risk matrix with zone-level risk tiers.
- Emits risk-zone vector outputs and summary contracts for operations planning.

#### How It Works

- Extracts required optical bands and computes NDMI (if SWIR available) or NDWI proxy fallback.
- Uses NDVI, moisture thresholds, and optional biomass proxy to assign fuel class per cell.
- Computes ladder continuity and combines fuel risk with optional slope/aspect spread amplifiers.
- Produces a clipped risk score in [0,1] and polygonized risk zones with dominant fuel and risk tier.
- Indicative formula: risk ~= (base_fuel_risk + dryness_boost) * slope_amp * aspect_amp.

#### Why It Wins

- Packages moisture, structure, terrain amplification, and zone-level risk outputs in a single reproducible workflow.

#### Typical Buying Trigger

Seasonal mitigation planning and risk-tier prioritization require consistent spatial evidence outputs.

#### Typical Presets

- conservative for lower sensitivity and tighter risk escalation.
- balanced for default planning workflows.
- aggressive for early-warning and preventative treatment prioritization.

### Inputs

ParameterOptionalDescription
optical_bundlenoMultispectral raster containing red/NIR and optionally SWIR bands.
red_band_index, nir_band_indexyesRed and NIR band indices. Defaults: 0, 1.
swir_band_indexyesOptional SWIR index; enables NDMI moisture estimation when available.
biomass_proxyyesOptional LiDAR biomass/height proxy used to refine fuel class and ladder continuity signals.
slope, aspectyesOptional terrain slope/aspect rasters used for spread amplification terms.
profile: conservative | balanced | aggressiveyesSensitivity profile controlling class/risk thresholds.
zone_block_cellsyesPixel block size used for risk-zone polygon aggregation. Defaults to `20`.
output_prefixyesPrefix used to name output artifacts.

### Outputs

ParameterTypeDescription
moisture_indexGeoTIFFNDMI/NDWI moisture response raster (`*_moisture_index.tif`).
fuel_load_classGeoTIFFFuel class code raster (sparse/surface/ladder/canopy) (`*_fuel_load_class.tif`).
ladder_fuel_continuityGeoTIFFLadder continuity index surface (`*_ladder_fuel_continuity.tif`).
risk_matrixGeoTIFFTerrain-amplified wildfire risk score raster (`*_risk_matrix.tif`).
risk_zonesGeoPackageAggregated risk polygons with tier and dominant fuel attributes (`*_risk_zones.gpkg`).
summaryJSONMachine-readable summary contract (`*_summary.json`).
html_reportHTMLHuman-readable report generated from the summary contract for review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.wildfire_fuel_loading_and_risk_matrix(
    optical_bundle="data/optical_multiband.tif",
    red_band_index=0,
    nir_band_index=1,
    swir_band_index=2,
    biomass_proxy="data/biomass_proxy.tif",
    slope="data/slope.tif",
    aspect="data/aspect.tif",
    profile="balanced",
    zone_block_cells=20,
    output_prefix="output/wildfire_risk",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Mine Site Reclamation Compliance Tracker

**Function name:** `mine_site_reclamation_compliance_tracker`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Mine Reclamation Compliance Tracker**

#### Problem It Solves

Are reclamation milestones being achieved spatially, and where are intervention priorities for compliance closure?

#### Who It Is For

- Mine closure teams, compliance analysts, and environmental reporting operations.

#### Primary User

Mine operators, reclamation contractors, and regulatory compliance monitoring groups.

#### What It Does

- Tracks vegetation recovery between baseline and current imagery.
- Computes per-cell reclamation progress against target NDVI milestone.
- Optionally evaluates slope stability milestone compliance.
- Produces compliance-zone outputs and a contract-ready compliance summary.

#### How It Works

- Extracts baseline/current red and NIR bands and computes per-date NDVI.
- Computes recovery delta and normalized progress toward target NDVI threshold.
- Optionally evaluates slope raster against maximum acceptable slope angle.
- Assigns milestone statuses and an overall compliance grade summary.
- Aggregates per-cell progress into compliance zones with pass/conditional/fail class attributes.
- Indicative formula: progress ~= (NDVI_current - NDVI_baseline) / (NDVI_target - NDVI_baseline), clipped to [0,1].

#### Why It Wins

- Integrates vegetation recovery, optional slope milestone evaluation, zone outputs, and compliance-grade contract reporting in one run.

#### Typical Buying Trigger

Regulatory milestone reporting cycles require reproducible evidence and zone-level compliance diagnostics.

#### Typical Presets

- spectral-only for vegetation recovery milestone tracking.
- full compliance mode with slope stability for stricter regulatory submissions.

### Inputs

ParameterOptionalDescription
baseline_bundle, current_bundlenoBaseline and current multiband rasters used for NDVI recovery analysis.
baseline_red_band_index, baseline_nir_band_indexyesBaseline red/NIR band indices. Defaults: 0, 1.
current_red_band_index, current_nir_band_indexyesCurrent red/NIR band indices. Defaults: 0, 1.
slopeyesOptional slope raster (degrees) for stability milestone evaluation.
reclamation_target_ndviyesTarget NDVI threshold used for vegetation milestone scoring. Defaults to `0.35`.
slope_stability_max_degyesMaximum acceptable slope for stability compliance. Defaults to `30.0`.
jurisdictionyesCompliance template: us_federal_mtbs, us_california_mining, us_pennsylvania_coal, aus_western_australia, canada_bc_mines, south_africa_dmre, none.
site_nameyesOptional site name or permit identifier in contract outputs.
has_hydrology_evidence, has_soil_ph_evidence, has_perennial_vegetation_evidenceyesBoolean evidence flags used by submission-readiness diagnostics.
report_interval_monthsyesReported cadence in months (1-120) compared against template expectations.
zone_block_cellsyesPixel block size used for compliance-zone aggregation. Defaults to `20`.
output_prefixyesPrefix used to name output artifacts.

### Outputs

ParameterTypeDescription
ndvi_baselineGeoTIFFBaseline NDVI raster (`*_ndvi_baseline.tif`).
ndvi_currentGeoTIFFCurrent NDVI raster (`*_ndvi_current.tif`).
vegetation_recoveryGeoTIFFNDVI recovery delta raster (`*_vegetation_recovery.tif`).
reclamation_progressGeoTIFFNormalized progress-to-target raster (`*_reclamation_progress.tif`).
compliance_zonesGeoPackageZone-level compliance polygons with progress/recovery attributes (`*_compliance_zones.gpkg`).
compliance_contractJSONMachine-readable compliance summary contract (`*_compliance_contract.json`).
validation_diagnosticsJSONEvidence completeness and warning diagnostics (`*_validation_diagnostics.json`).
html_reportHTMLHuman-readable report generated from compliance contract outputs.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.mine_site_reclamation_compliance_tracker(
    baseline_bundle="data/mine_baseline.tif",
    current_bundle="data/mine_current.tif",
    baseline_red_band_index=0,
    baseline_nir_band_index=1,
    current_red_band_index=0,
    current_nir_band_index=1,
    slope="data/slope.tif",
    reclamation_target_ndvi=0.35,
    slope_stability_max_deg=30.0,
    jurisdiction="canada_bc_mines",
    site_name="Mine Site Alpha",
    has_hydrology_evidence=True,
    has_soil_ph_evidence=True,
    has_perennial_vegetation_evidence=True,
    report_interval_months=12,
    zone_block_cells=20,
    output_prefix="output/reclamation",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Terrain Constraint And Conflict Analysis

**Function name:** `terrain_constraint_and_conflict_analysis`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Terrain Constraint and Conflict Analysis**

#### Problem It Solves

Where are terrain constraints likely to create permitting, access, or design conflicts before field mobilization?

#### Who It Is For

- Infrastructure siting teams and engineering predesign analysts.

#### Primary User

Renewable developers, utilities, and civil engineering consultancies.

#### What It Does

- Builds a harmonized terrain conflict score from slope and optional risk overlays.
- Produces conflict classes for early-stage siting and routing screening.

#### How It Works

- Uses DEM-derived slope as the primary terrain constraint signal.
- Harmonizes optional wetness, flood-risk, and landcover-penalty rasters to the DEM grid.
- Blends constraints into a normalized conflict score and class map.
- QA acceptance guidance:
- `status=pass` indicates the conflict fraction remained below review threshold.
- `diagnostics.acceptance_thresholds.high_conflict_fraction_review` is the baseline threshold for escalation.
- Elevated `summary.high_conflict_fraction` should trigger engineering review before downstream routing/siting commitments.
- MVP hardening assets:
- Benchmark scaffold: `tests/fixtures/terrain_siting_benchmark/`
- Promotion guide: `docs/internal/development/TERRAIN_PRECISION_AG_BENCHMARK_PROMOTION_GUIDE_2026_04_14.md`

### Inputs

ParameterOptionalDescription
demnoReference DEM raster for terrain conflict analysis.
wetnessyesOptional wetness raster normalized to [0,1].
flood_riskyesOptional flood-risk raster normalized to [0,1].
landcover_penaltyyesOptional landcover penalty raster normalized to [0,1].
slope_limit_degyesSlope threshold where terrain conflict accelerates.

### Outputs

ParameterTypeDescription
conflict_scoreGeoTIFFContinuous terrain conflict score [0,1].
conflict_classGeoTIFFDiscrete conflict class raster for planning triage.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

conflict, classes, summary = wbe.terrain_constraint_and_conflict_analysis(
    dem="data/dem.tif",
    wetness="data/wetness.tif",
    flood_risk="data/flood_risk.tif",
    slope_limit_deg=15.0,
    output_prefix="output/terrain_conflict",
)

print(conflict)
print(classes)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Terrain Constructability And Cost Analysis

**Function name:** `terrain_constructability_and_cost_analysis`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Terrain Constructability and Cost Analysis**

#### Problem It Solves

Which areas are practically constructible at lower relative cost before detailed design?

#### Who It Is For

- Engineering predesign teams and capital planning groups.

#### Primary User

Infrastructure developers, engineering firms, and utility planning teams.

#### What It Does

- Converts terrain and optional risk/cost context into constructability and cost-class surfaces.
- Supports quick predesign ranking of lower-cost, lower-risk development zones.

#### How It Works

- Computes slope from DEM and harmonizes optional conflict, wetness, and access-cost rasters.
- Blends penalties into a constructability score and relative cost class output.
- QA acceptance guidance:
- `status=pass` indicates high-cost fraction is within baseline tolerance.
- `diagnostics.acceptance_thresholds.high_cost_fraction_review` defines review escalation threshold.
- Use `summary.high_cost_fraction` with local access constraints before capital-stage budgeting decisions.
- MVP hardening assets:
- Benchmark scaffold: `tests/fixtures/terrain_siting_benchmark/`
- Promotion guide: `docs/internal/development/TERRAIN_PRECISION_AG_BENCHMARK_PROMOTION_GUIDE_2026_04_14.md`

### Inputs

ParameterOptionalDescription
demnoReference DEM raster for constructability scoring.
existing_conflictyesOptional prior terrain conflict raster normalized to [0,1].
wetnessyesOptional wetness raster normalized to [0,1].
access_costyesOptional access-friction raster normalized to [0,1].

### Outputs

ParameterTypeDescription
constructability_scoreGeoTIFFConstructability score raster [0,1] (higher is better).
cost_classGeoTIFFRelative cost class raster (1 low cost to 5 high cost).
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

constructability, cost_class, summary = wbe.terrain_constructability_and_cost_analysis(
    dem="data/dem.tif",
    existing_conflict="output/terrain_conflict_conflict_score.tif",
    access_cost="data/access_friction.tif",
    output_prefix="output/terrain_constructability",
)

print(constructability)
print(cost_class)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Utility Corridor Encroachment Intelligence

**Function name:** `utility_corridor_encroachment_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Utility Corridor Encroachment Detection**

#### Problem It Solves

Which corridor segments present the highest near-term encroachment risk, and which assets should be prioritized first?

#### Who It Is For

- Utility vegetation-management teams and corridor maintenance planners.

#### Primary User

Transmission and distribution operators, utility contractors, and corridor asset-management groups.

#### What It Does

- Builds corridor-adjacent encroachment risk surfaces from LiDAR-derived canopy structure.
- Produces priority zones and per-asset risk summaries for maintenance planning.

#### How It Works

- Classifies vegetation and ground structure, then computes height-above-ground and local point-density context.
- Builds a corridor-relative risk surface that emphasizes canopy proximity, density, and confidence-weighted structure signals.
- Applies profile-based sensitivity settings and emits thresholded, ranked priority zones for maintenance triage.
- Indicative formula: risk ~= f(proximity_to_corridor, canopy_height, local_density, classification_confidence).

#### Why It Wins

- Produces both spatial priority zones and asset-level risk tables, enabling field-ready scheduling rather than map-only screening.

#### Typical Buying Trigger

Seasonal vegetation cycles or reliability programs require objective, repeatable encroachment prioritization across large networks.

#### Typical Presets

- fast for broad network screening.
- balanced for default operational planning.
- strict for conservative risk escalation and tighter action thresholds.
- Operational controls:
- profile: fast | balanced | strict.
- priority_zone_threshold and max_zone_features: bound priority-zone density for operational triage.

### Inputs

ParameterOptionalDescription
input (LAS/LAZ)noInput LiDAR point cloud used to derive QA, terrain, structure, or encroachment products.
optional corridors and asset_featuresyesOptional corridor and asset vectors used to focus encroachment risk analysis.
profile: fast | balanced | strictnoOperational profile controlling sensitivity and QA strictness for risk workflows.
priority_zone_thresholdnoRisk threshold used to classify high-priority encroachment zones.
max_zone_featuresnoUpper cap on number of output zone features to control product size.

### Outputs

ParameterTypeDescription
encroachment_riskGeoTIFFEncroachment risk surface used for corridor maintenance prioritization.
corridor_priority_zonesGeoPackageVector priority zones for field action planning.
asset_risk_tableGeoPackageVector table/layer with per-asset encroachment risk summaries.
classification_confidenceGeoTIFFConfidence surface for LiDAR-derived classification and risk outputs.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

risk, zones, assets, conf, summary = wbe.utility_corridor_encroachment_intelligence(
    input="data/corridor_points.laz",
    corridors="data/corridors.gpkg",
    asset_features="data/assets.gpkg",
    profile="balanced",
    priority_zone_threshold=0.75,
    max_zone_features=5000,
    output_prefix="output/utility_corridor",
)

print(risk)
print(zones)
print(assets)
print(conf)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Forestry Structure And Biomass Intelligence

**Function name:** `forestry_structure_and_biomass_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Forestry Structure and Biomass Analysis**

#### Problem It Solves

Where are high-structure and high-biomass stands concentrated, and how confident are those estimates for inventory decisions?

#### Who It Is For

- Forest inventory analysts, carbon accounting teams, and silviculture planning groups.

#### Primary User

Forestry agencies, carbon-market project developers, and natural resource consultancies.

#### What It Does

- Generates canopy structure, vertical class, and biomass-proxy products from LiDAR.
- Produces stand-level structure units and confidence diagnostics for inventory workflows.
- Requires Pro runtime visibility (`include_pro=True`, `tier='pro'` or higher).

#### How It Works

- Grids LiDAR to derive canopy-height, density, and ground surfaces, then computes CHM-style structure metrics.
- Classifies canopy structure from adaptive height thresholds and density support into vertical strata classes.
- Scales a terrain-adaptive biomass proxy with configurable cap control.
- Aggregates stand-level structure units and confidence indicators for inventory and monitoring.
- Indicative formula: biomass_proxy ~= g(canopy_height, density_support, structure_class, terrain_relief), clamped by biomass_cap.

#### Why It Wins

- Combines structure classes, biomass proxy, and stand-unit outputs in one reproducible workflow suitable for operational reporting.

#### Typical Buying Trigger

Inventory refresh, carbon-baseline updates, or stand treatment planning needs consistent structure and biomass surfaces.

#### Typical Presets

- fast for regional reconnaissance.
- balanced for default inventory support.
- strict for conservative structure/banding and confidence interpretation.
- Operational controls:
- profile: fast | balanced | strict.
- terrain_adaptation: off | moderate | strong.
- biomass_cap: bounds biomass-proxy scaling for predictable downstream comparisons.

### Inputs

ParameterOptionalDescription
input (LAS/LAZ or Lidar)noInput LiDAR source used to derive forest structure and biomass products.
profile: fast | balanced | strictnoOperational profile controlling sensitivity and QA strictness.
resolutionyesOutput raster resolution, default 2.0.
stand_block_cellsyesStand aggregation block size in cells, default 12.
biomass_capyesUpper bound applied to biomass proxy estimates, default 25.0.
terrain_adaptation: off | moderate | strongyesBiomass terrain-adaptation mode, default moderate.

### Outputs

ParameterTypeDescription
canopy_height_metricsGeoTIFFCanopy height metrics raster for forestry structure interpretation.
vertical_structure_classGeoTIFFCategorical vertical-structure class raster.
stand_structure_unitsGeoPackageVector stand structure units for reporting and management.
biomass_proxyGeoTIFFBiomass proxy raster derived from structural LiDAR metrics.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

lidar = wbw.Lidar("data/forest_points.laz")

height, vclass, stands, biomass, conf, summary = wbe.forestry_structure_and_biomass_intelligence(
    input=lidar,
    profile="balanced",
    resolution=2.0,
    stand_block_cells=12,
    biomass_cap=30.0,
    terrain_adaptation="moderate",
    output_prefix="output/forestry_structure",
)

print(height)
print(vclass)
print(stands)
print(biomass)
print(conf)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
