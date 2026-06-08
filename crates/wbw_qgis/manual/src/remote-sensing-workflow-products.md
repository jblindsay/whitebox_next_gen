# Workflow Products


---

## Multi Sensor Fusion Monitoring

**Function name:** `multi_sensor_fusion_monitoring`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Multi-Sensor Fusion Monitoring**

#### Problem It Solves

Where do optical and SAR signals jointly support actionable change alerts, and where is confidence high enough to triage immediately?

#### Who It Is For

- Multi-source monitoring teams combining optical and SAR evidence.

#### Primary User

National/regional EO monitoring programs, environmental observatories, and risk intelligence teams.

#### What It Does

- Fuses optical change, SAR stability cues, and terrain context into one disturbance-monitoring product.
- Produces sensor-agreement and fused-probability outputs for confidence-first screening.
- Emits high-confidence change zones suitable for direct review and reporting.

#### How It Works

- Runs `remote_sensing_change_detection` and `sar_analysis_readiness` as upstream stages.
- Computes a per-pixel agreement score from optical confidence plus SAR consistency/coherence cues.
- Combines normalized change strength and agreement into fused change probability.
- Indicative formula: $P_{fused} = w_c \cdot |\Delta_{optical}|_{norm} + w_a \cdot A_{sensor}$.

#### Why It Wins

- Prevents one-sensor overconfidence by explicitly encoding cross-sensor agreement into final change probability.

#### Typical Buying Trigger

Operations teams need lower-false-alarm change monitoring in cloudy/seasonally variable regions where single-sensor methods are unstable.

#### Typical Presets

- fast: lower-overhead configuration for broader candidate capture.
- balanced: default setting for operational triage.
- conservative: stricter thresholding for high-specificity alerting.

### Inputs

ParameterOptionalDescription
baseline_bundle, baseline_red_band_index, baseline_nir_band_indexnoBaseline multispectral bundle and red/NIR band selectors used to compute baseline vegetation response.
change_bundle, change_red_band_index, change_nir_band_indexnoChange-date multispectral bundle and red/NIR band selectors used for signed change estimation.
input_sar, input_demnoSAR scene and terrain model used for radiometric terrain correction and readiness metrics.
optional pair_saryesOptional second SAR scene used when pair/coherence diagnostics are enabled.
optional thermal_bundle, thermal_band_indexyesOptional thermal raster and 0-based band index used for three-modality diagnostics.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.
harmonization_modeyesCross-sensor bias harmonization mode: off, robust, or conservative.
high_confidence_threshold, max_zone_featuresnoThreshold and feature-cap controls for extracting high-confidence change zones.
vector_output_formatyesOutput vector format for zones: gpkg, geojson, or shp.

### Outputs

ParameterTypeDescription
fused_change_probabilityGeoTIFFCross-sensor fused probability of meaningful environmental change.
sensor_agreementGeoTIFFAgreement surface indicating where sensors support the same change interpretation.
terrain_contextGeoTIFFDerived terrain context layer used by fused change interpretation.
uncertainty_inflationGeoTIFFPer-pixel uncertainty inflation diagnostic from cross-modality fusion.
high_confidence_change_zonesGeoPackageVector zones representing high-confidence change hotspots.
thermal_input_contractJSONThermal coverage and weighting contract generated when the workflow runs.
modality_contribution_diagnosticsJSONRelative modality contribution diagnostics for optical/SAR/thermal sources.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

fused, agreement, terrain, uncertainty, zones, thermal_contract, modality_diagnostics, summary = wbe.multi_sensor_fusion_monitoring(
    baseline_bundle="data/baseline_bundle.tif",
    baseline_red_band_index=0,
    baseline_nir_band_index=1,
    change_bundle="data/change_bundle.tif",
    change_red_band_index=0,
    change_nir_band_index=1,
    input_sar="data/sar_a.tif",
    input_dem="data/dem.tif",
    pair_sar="data/sar_b.tif",
    thermal_bundle="data/thermal.tif",
    thermal_band_index=0,
    profile="balanced",
    harmonization_mode="robust",
    vector_output_format="gpkg",
    high_confidence_threshold=0.8,
    max_zone_features=25000,
    output_prefix="output/ms_fusion",
)

print(fused)
print(agreement)
print(terrain)
print(uncertainty)
print(zones)
print(thermal_contract)
print(modality_diagnostics)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Guided Uav Image Intake Workflow

**Function name:** `guided_uav_image_intake_workflow`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**UAV Image Intake QA**

#### Problem It Solves

Is this image set good enough to proceed into expensive downstream processing, and what should be fixed first if not?

#### Who It Is For

- UAV mission operators, photogrammetry technicians, and geospatial production teams.

#### Primary User

Drone services teams, geomatics operations managers, and data-engineering teams responsible for production intake quality.

#### What It Does

- Scans UAV imagery and builds a metadata/coverage inventory.
- Computes intake QA metrics (GPS/EXIF completeness, overlap estimate, image-count sufficiency).
- Extracts per-image blur scores and flags blurry images with warnings.
- Parses RTK fix status and gimbal/flight orientation priors from DJI XMP metadata where available.
- Returns workflow status (`pass`, `review`, `fail`) with warnings suitable for operator triage.

#### How It Works

- Recursively discovers supported image files (`jpg`, `jpeg`, `tif`, `tiff`, `png`).
- Parses EXIF fields for timestamp and GPS coordinates; parses DJI XMP sidecar fields for RTK fix status and gimbal yaw/pitch/roll and flight yaw.
- Estimates overlap from GPS nearest-neighbor spacing against a nominal footprint heuristic.
- Computes a per-image blur score from a Laplacian variance kernel (configurable as `off`, `fast`, or `full` mode) with wide-SIMD acceleration.
- Metadata extraction and blur scoring run in parallel across all images using Rayon thread pools.
- Applies profile thresholds to classify readiness and emit actionable warnings.
- Indicative heuristic: overlap_proxy ~= 1 - (nn_spacing_m / nominal_footprint_m), clipped to [0, 1].

#### Why It Wins

- Converts ad hoc manual preflight checks into a single reproducible QA workflow with machine-readable outputs, including blur quality, RTK status, and orientation priors alongside the standard GPS and overlap diagnostics.

#### Typical Buying Trigger

Frequent failed/rework-heavy photogrammetry or registration runs caused by avoidable intake quality issues.

#### Typical Presets

- fast: permissive intake thresholds with fast-mode blur scoring for rapid field feedback.
- balanced: default operations profile with fast-mode blur scoring.
- strict: tighter quality gate with full-mode blur scoring before production processing.

### Inputs

ParameterOptionalDescription
images_dirnoInput directory containing UAV images to screen.
profile: fast | balanced | strictyes[pro] QA profile controlling overlap, metadata readiness, and blur thresholds; drives pass/review/fail classification. Defaults to `balanced`.
recursiveyesIf true, scans subdirectories under images_dir. Defaults to `true`.
output_prefixyesPrefix used to name all output artifacts.
blur_mode: off | fast | fullyesBlur scoring mode. `off` skips blur scoring; `fast` downsamples before scoring; `full` scores at original resolution. Defaults to `fast`.

### Outputs

ParameterTypeDescription
image_inventoryCSVPer-image inventory with metadata flags, GPS coordinates, capture fields, blur score, and gimbal/RTK orientation columns (`*_image_inventory.csv`).
qa_reportJSONStructured QA checks and warning details for intake triage, including blur and orientation hint sections (`*_intake_qa_report.json`).
summaryJSONWorkflow summary contract with status and aggregate metrics including blur coverage and RTK coverage fractions (`*_intake_summary.json`).
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.
image_centersGeoJSONImage center points from EXIF GPS with per-feature blur score and orientation properties (`*_image_centers.geojson`).
flight_path_linesGeoJSONFlight path line geometry built from ordered image centers with GPS (`*_flight_path_lines.geojson`).

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.guided_uav_image_intake_workflow(
    images_dir="data/uav_mission/images",
    profile="balanced",
    recursive=True,
    output_prefix="output/uav_intake",
    blur_mode="fast",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Registration Oriented Feature Workflow

**Function name:** `registration_oriented_feature_workflow`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Image Registration Diagnostics**

#### Problem It Solves

Which pairs are registration-ready, and do we have enough correspondence quality to proceed confidently?

#### Who It Is For

- Geospatial registration specialists, EO preprocessing teams, and image-fusion analysts.

#### Primary User

Teams building repeatable image registration pipelines (RGB-RGB, thermal-RGB, and cross-date alignment).

#### What It Does

- Runs image-pair or image-set registration diagnostics using a RootSIFT feature engine.
- Produces tie-point outputs and pair-level quality metrics with full fallback attempt traces.
- Handles cross-modal pairs (e.g., thermal LWIR to RGB visible) via automatic histogram equalization preprocessing.
- Optionally emits annotated side-by-side pair visualizations with tie-point overlays.
- Emits workflow-level readiness status for registration-first pipelines.

#### How It Works

- Builds pair candidates (single pair mode or ranked set mode).
- Extracts RootSIFT descriptors from a Gaussian scale-space pyramid; descriptor distances are computed with wide-SIMD acceleration.
- Applies cross-verified nearest-neighbor matching with ratio test filtering.
- On match shortfall, escalates through a six-strategy fallback chain: `baseline` → `high_feature_baseline` → `relaxed_ratio` → `high_feature_relaxed` → `preprocess_eq_right` → `preprocess_eq_both`; the final two strategies apply histogram equalization to recover keypoints from low-contrast inputs such as LWIR thermal imagery.
- Records the full strategy attempt trace per pair in diagnostics for auditable QA.
- Reports per-pair keypoint counts, match counts, confidence summary, inlier-style proxy metrics, strategy used, and fallback policy.
- Indicative rule: accept match if d1 / d2 ParameterOptionalDescription
mode: set | pairnoExecution mode for set-wide pair planning or explicit pair diagnostics.
images_dirno (set mode)Input image directory used when mode is set.
left_image, right_imageno (pair mode)Explicit pair inputs used when mode is pair.
max_pairsyesMaximum number of candidate pairs evaluated in set mode.
max_features_per_imageyesUpper bound on extracted keypoints per image.
ratio_testyesDescriptor ratio-test threshold controlling match strictness.
min_matchesyes[pro] Minimum accepted match count per pair; drives the QA gate, fallback routing, and pass/review/fail classification.
output_prefixyesPrefix used to name workflow outputs.
emit_pair_match_vizyesIf true, writes annotated side-by-side pair images with tie-point lines. Defaults to `false`.
max_pair_visualizationsyesMaximum number of pair visualizations to write when emit_pair_match_viz is true. Defaults to `8`.
max_lines_per_pairyesMaximum number of tie-point lines drawn per visualization. Defaults to `150`.
viz_scaleyesDownscale factor applied to visualization images, in [0.05, 1.0]. Defaults to `0.5`.

### Outputs

ParameterTypeDescription
pair_diagnosticsJSONPair-level diagnostics including candidate score, keypoint counts, match quality, confidence proxies, strategy used, fallback flag, and full strategy attempt trace (`*_pair_diagnostics.json`).
match_summaryJSONWorkflow summary contract with aggregate match metrics, status, and fallback policy record (`*_match_summary.json`).
html_reportHTMLHuman-readable customer-facing report generated from the workflow summary contract for stakeholder review and QA traceability.
tie_pointsCSVTie-point table (`pair_id,left_x,left_y,right_x,right_y,confidence`) for downstream registration workflows (`*_tie_points.csv`).
pair_match_vizdirectoryAnnotated side-by-side JPEG images per pair with tie-point lines overlaid, written when emit_pair_match_viz is true.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.registration_oriented_feature_workflow(
    mode="set",
    images_dir="data/uav_mission/images",
    max_pairs=24,
    max_features_per_image=500,
    ratio_test=0.80,
    min_matches=24,
    output_prefix="output/registration_workflow",
    emit_pair_match_viz=True,
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
