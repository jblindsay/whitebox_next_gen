# Workflow Products


---

## LiDAR QA And Confidence

**Function name:** `lidar_qa_and_confidence`


PROProduction

Assess LiDAR point-cloud quality and compute confidence metrics for terrain extraction readiness.

workflow pro

### Workflow Narrative

**LiDAR QA and Confidence**

#### Problem It Solves

Is this LiDAR deliverable trustworthy enough for production terrain modeling, and where are the risk zones?

#### Who It Is For

- LiDAR production QA teams and data acceptance reviewers.

#### Primary User

Survey/mapping firms, government mapping programs, and enterprise geospatial platforms.

#### What It Does

- Runs LiDAR QA workflow with ground-surface diagnostics.
- Produces confidence, uncertainty, and QA flags for acceptance screening.
- Supports qa_mode-driven strictness behavior (strict, balanced, permissive, auto).
- Supports fast_mode for exploratory runs that skip non-critical diagnostics.

#### How It Works

- Classifies and normalizes point-cloud structure to estimate ground-surface consistency.
- Builds rasterized QA metrics (confidence, uncertainty, flags) from neighborhood evidence.
- Applies mode-specific acceptance thresholds to summarize pass-risk patterns.
- Optionally runs checkpoint vertical validation and auto-mode recommendations in summary outputs.
- Indicative formula: confidence ~= 1 - normalized(local_residual + return_structure_penalty).

#### Why It Wins

- Converts QA from a binary pass/fail decision into spatial confidence and uncertainty diagnostics.

#### Typical Buying Trigger

Data acceptance teams need objective QA evidence before approving vendor LiDAR for production use.

#### Typical Presets

- strict for conservative acceptance checks.
- balanced for normal production QA.
- permissive for exploratory preprocessing.
- auto for recommendation-guided QA mode selection.
- fast_mode=true for rapid exploratory runs when full diagnostics are not required.

### Inputs

ParameterOptionalDescription
input (LAS/LAZ)noInput LiDAR point cloud used to derive QA, terrain, structure, or encroachment products.
qa_mode and QA threshold controlsnoQA strictness mode and threshold controls for LiDAR acceptance diagnostics.
fast_modeyesOptional acceleration mode that skips hotspot extraction, stratified metrics, and checkpoint validation for faster exploratory runs.

### Outputs

ParameterTypeDescription
classified_lidaroptional LAS/LAZOptional classified LiDAR point cloud output from QA/terrain workflows.
dtmGeoTIFFDigital terrain model raster generated from workflow processing.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
uncertaintyGeoTIFFUncertainty diagnostics layer highlighting low-certainty areas.
qa_flagsGeoTIFFQA flag raster identifying cells that failed quality checks.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.lidar_qa_and_confidence(
    input="data/points.laz",
    qa_mode="balanced",
    fast_mode=False,
    output_prefix="output/lidar_qa",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## LiDAR Terrain Product Suite

**Function name:** `lidar_terrain_product_suite`


PROProduction

Generate terrain-focused raster products from LiDAR with consistent QA and surface controls.

workflow pro

### Workflow Narrative

**LiDAR Terrain Product Suite**

#### Problem It Solves

How can we convert raw LiDAR into a consistent, production-ready terrain package quickly and reproducibly?

#### Who It Is For

- Terrain product operations teams and applied engineering GIS units.

#### Primary User

Mapping programs, engineering/environmental consultancies, and topographic data providers.

#### What It Does

- Produces a full terrain derivative package from raw LiDAR in one run.
- Includes core terrain products and propagated QA metrics.
- Emits metadata contract for reproducibility and downstream automation.

#### How It Works

- Performs LiDAR preprocessing/classification and interpolates terrain surfaces.
- Derives DTM/DSM/slope/hillshade products from the processed ground and surface model.
- Propagates QA diagnostics into confidence/uncertainty and records run metadata contract.
- Indicative formula: slope = atan(sqrt((dz/dx)^2 + (dz/dy)^2)); hillshade from slope/aspect and illumination azimuth/zenith.

#### Why It Wins

- Unifies classification, terrain derivatives, and QA metadata into one operational pipeline.

#### Typical Buying Trigger

Production teams must deliver standardized terrain products at scale with minimal manual orchestration.

#### Typical Presets

- balanced for default production.
- strict for high-stakes engineering QA contexts.

### Inputs

ParameterOptionalDescription
input (LAS/LAZ)noInput LiDAR point cloud used to derive QA, terrain, structure, or encroachment products.
profile, block size, slope/elevation thresholdsnoTerrain-suite processing profile and block/threshold controls for derivative generation.
hillshade and QA controlsnoHillshade illumination and QA export controls for terrain package outputs.

### Outputs

ParameterTypeDescription
dtmGeoTIFFDigital terrain model raster generated from workflow processing.
dsmGeoTIFFDigital surface model raster generated from workflow processing.
slopeGeoTIFFSlope derivative raster generated from terrain processing.
hillshadeGeoTIFFHillshade visualization raster generated from terrain derivatives.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
uncertaintyGeoTIFFUncertainty diagnostics layer highlighting low-certainty areas.
metadataJSONMetadata contract describing generated products and provenance.
html_reportHTMLHuman-readable customer-facing report generated from the metadata/summary contract for stakeholder review and QA traceability.
classified_lidaroptional LAS/LAZOptional classified LiDAR point cloud output from QA/terrain workflows.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.lidar_terrain_product_suite(
    input="data/points.laz",
    profile="balanced",
    output_prefix="output/terrain_suite",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## LiDAR Change And Disturbance Analysis

**Function name:** `lidar_change_and_disturbance_analysis`


PROProduction

Compare baseline and monitoring LiDAR epochs to quantify elevation change and disturbance intensity.

workflow pro

### Workflow Narrative

**LiDAR Change and Disturbance Analysis**

#### Problem It Solves

Where has significant terrain or canopy disturbance occurred between LiDAR acquisition epochs?

#### Who It Is For

- LiDAR monitoring programs, forestry operations, and infrastructure change-monitoring teams.

#### Primary User

Asset monitoring groups, agencies running repeat LiDAR acquisitions, and environmental compliance teams.

#### What It Does

- Performs epoch-to-epoch LiDAR change analysis using tile-native processing.
- Produces per-tile delta rasters plus a disturbance manifest and run summary.

#### How It Works

- Accepts baseline and monitoring tile sets (arrays or directories), sorted and paired per tile.
- Grids each tile to surface rasters and harmonizes monitor tile CRS/grid to baseline per pair.
- Computes elevation delta and thresholded disturbed area metrics per tile.
- QA acceptance guidance:
- `status=pass` indicates no seam-risk or low-support warnings were triggered.
- `diagnostics.acceptance_thresholds.min_valid_cells_per_tile` defaults to 500 and flags sparse tile support.
- `diagnostics.acceptance_thresholds.seam_risk_warn_cv` defaults to 0.75 and flags elevated inter-tile inconsistency.
- Review `diagnostics.tile_diagnostics` before publishing disturbance totals for operational reporting.
- MVP hardening assets:
- Municipal ingestion guide: `docs/internal/development/LIDAR_CHANGE_SIDEWALK_MUNICIPAL_SCHEMA_INGESTION_GUIDE_2026_04_14.md`
- Benchmark fixture scaffold: `tests/fixtures/lidar_change_city_benchmark/`

### Inputs

ParameterOptionalDescription
baseline_tilesnoBaseline LiDAR tiles (array or directory path).
monitor_tilesnoMonitoring-epoch LiDAR tiles (array or directory path).
resolutionyesOutput tile-surface resolution in map units.
min_change_myesAbsolute change threshold for disturbance accounting.

### Outputs

ParameterTypeDescription
tile_directorydirectoryDirectory containing per-tile delta rasters.
tile_manifestJSONPer-tile output and disturbance metrics manifest.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

summary, manifest, tile_dir = wbe.lidar_change_and_disturbance_analysis(
    baseline_tiles="data/lidar_2023_tiles/",
    monitor_tiles="data/lidar_2025_tiles/",
    resolution=2.0,
    min_change_m=1.0,
    output_prefix="output/lidar_change",
)

print(summary)
print(manifest)
print(tile_dir)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Sidewalk Vegetation Accessibility Monitoring

**Function name:** `sidewalk_vegetation_accessibility_monitoring`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Sidewalk Vegetation Accessibility Monitoring**

#### Problem It Solves

Which sidewalk segments are most obstructed by vegetation, and where is LiDAR coverage missing?

#### Who It Is For

- Municipal accessibility teams, urban forestry programs, and public-works corridor management.

#### Primary User

Municipal accessibility offices and operations teams responsible for pedestrian corridor clearance.

#### What It Does

- Scores vegetation encroachment along sidewalks using tile-native LiDAR processing.
- Aggregates all tile evidence into one city-level output layer.
- Supports centerline segmentation for line inputs and feature-level fallback for polygon inputs.

#### How It Works

- Reads LiDAR tiles from an array or directory and processes them one tile at a time.
- Builds per-tile DSM/DTM and computes height-above-ground obstruction surfaces.
- Reprojects sidewalks to tile CRS when needed, samples obstruction neighborhoods, and aggregates results.
- For line/multiline sidewalks, optionally segments centerlines into fixed-length analysis units.
- QA acceptance guidance:
- `status=pass` indicates coverage and obstruction diagnostics met baseline QA thresholds.
- `diagnostics.acceptance_thresholds.minimum_coverage_fraction` defaults to 0.75; values below this trigger review.
- `summary.no_lidar_coverage_features` are unresolved units and should be targeted for additional capture or fallback policy.
- Prioritize operational responses using `STATUS`, `MAX_OBSTR`, and tile-level diagnostics together.
- MVP hardening assets:
- Municipal ingestion guide: `docs/internal/development/LIDAR_CHANGE_SIDEWALK_MUNICIPAL_SCHEMA_INGESTION_GUIDE_2026_04_14.md`
- Benchmark fixture scaffold: `tests/fixtures/sidewalk_accessibility_city_benchmark/`

### Inputs

ParameterOptionalDescription
lidar_tilesnoLiDAR tiles as array or directory path (LAS/LAZ/ZLidar).
sidewalksnoSidewalk layer (line, multiline, polygon, or multipolygon).
sidewalks_epsgyesEPSG override when sidewalk CRS metadata is missing.
resolutionyesIntermediate tile raster resolution in map units.
segment_length_myesSegment length for line inputs; ignored for polygon inputs.
clearance_height_myesHeight threshold used to label obstruction status.
buffer_distance_myesNeighborhood sampling radius around sidewalk geometry.

### Outputs

ParameterTypeDescription
sidewalk_accessibilityGeoPackageCity-level aggregated sidewalk/segment accessibility layer.
summaryJSONMachine-readable summary report containing coverage, obstruction counts, and QA diagnostics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

access, summary = wbe.sidewalk_vegetation_accessibility_monitoring(
    lidar_tiles="data/lidar_1km_tiles/",
    sidewalks="data/sidewalk_centerlines.gpkg",
    segment_length_m=10.0,
    clearance_height_m=2.5,
    output_prefix="output/sidewalk_access",
)

print(access)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
