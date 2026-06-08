# Radiometric Correction


---

## BRDF Surface Reflectance Consistency

**Function name:** `brdf_surface_reflectance_consistency`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Surface Reflectance Consistency Analysis**

#### Problem It Solves

Are directional and terrain illumination effects sufficiently normalized for reliable scene-to-scene reflectance comparison?

#### Who It Is For

- Optical remote sensing teams preparing cross-scene reflectance products for trend/change workflows.

#### Primary User

EO product teams, environmental mapping agencies, and analytics groups with multi-date reflectance pipelines.

#### What It Does

- Harmonizes reflectance across dates and sensors after terrain correction.
- Quantifies normalization magnitude and confidence for QA-aware downstream analysis.
- Packages reflectance consistency as a reproducible workflow-stage product.

#### How It Works

- Runs an OSS terrain-correction prep stage when the input scene is still topographically distorted.
- Derives normalization delta from pre/post correction residual magnitude.
- Computes consistency confidence using exponential damping of large correction residuals.
- Indicative formula: $C = \exp(-\Delta / s) \cdot Q$, where $\Delta$ is normalization delta and $Q$ is quality confidence.

#### Why It Wins

- Couples normalization output with explicit delta and confidence diagnostics, enabling QA-aware downstream acceptance rules.

#### Typical Buying Trigger

Teams observe inconsistent reflectance behavior across acquisition geometries and need a reproducible consistency gate.

#### Typical Presets

- fast: quicker normalization for large-area throughput.
- balanced: default quality/speed tradeoff.
- conservative: stronger correction confidence thresholding.

### Inputs

ParameterOptionalDescription
input_red, input_nir, input_demnoCore optical + terrain inputs used for topographic/reflectance normalization workflows.
solar_zenith_deg, solar_azimuth_degnoSolar geometry parameters used to model illumination and terrain incidence effects.
optional input_greenyesOptional green band used by workflows that include green-channel diagnostics.
profile: fast | balanced | conservativenoProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.

### Outputs

ParameterTypeDescription
brdf_normalized_reflectanceGeoTIFFBRDF-normalized reflectance raster with improved angular consistency.
normalization_deltaGeoTIFFDifference layer showing magnitude of BRDF normalization adjustments.
consistency_confidenceGeoTIFFConfidence surface indicating reflectance consistency after normalization.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

normalized, delta, confidence, summary = wbe.brdf_surface_reflectance_consistency(
    input_red="data/red.tif",
    input_nir="data/nir.tif",
    input_dem="data/dem.tif",
    input_green="data/green.tif",
    solar_zenith_deg=40.0,
    solar_azimuth_deg=165.0,
    profile="balanced",
    output_prefix="output/brdf_consistency",
)

print(normalized)
print(delta)
print(confidence)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## BRDF Normalization

**Function name:** `brdf_normalization`


*No help documentation available for this tool.*


---

## Correct Vignetting

**Function name:** `correct_vignetting`


This tool can be used to reduce vignetting within an image. Vignetting refers to the reduction of image brightness away from the image centre (i.e. the principal point). Vignetting is a radiometric distortion resulting from lens characteristics. The algorithm calculates the brightness value in the output image (BVout) as: 

BVout = BVin / [cos^n(arctan(d / f))] 

Where d is the photo-distance from the principal point in millimetres, f is the focal length of the camera, in millimeters, and n is a user-specified parameter. Pixel distances are converted to photo-distances (in millimetres) using the specified image width, i.e. distance between left and right edges (mm). For many cameras, 4.0 is an appropriate value of the n parameter. A second pass of the image is used to rescale the output image so that it possesses the same minimum and maximum values as the input image. 

If an RGB image is input, the analysis will be performed on the intensity component of the HSI transform. 

### Python API

```python
def correct_vignetting(self, image: Raster, principal_point: Vector, focal_length: float = 304.8, image_width: float = 228.6, n_param: float = 4.0) -> Raster:
```


---

## Dark Object Subtraction

**Function name:** `dark_object_subtraction`


*No help documentation available for this tool.*


---

## Dn To Toa Reflectance

**Function name:** `dn_to_toa_reflectance`


*No help documentation available for this tool.*


---

## Terrain Corrected Optical Analytics

**Function name:** `terrain_corrected_optical_analytics`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Terrain-Corrected Optical Prep**

#### Problem It Solves

Can we reduce terrain illumination bias so downstream indices and change products are defensible?

#### Who It Is For

- Remote sensing practitioners working in high-relief terrain.

#### Primary User

EO analytics teams, natural resource agencies, and monitoring service providers.

#### What It Does

- Applies terrain-aware optical correction (C-correction style) using DEM-derived geometry.
- Builds cloud/shadow masks and quality confidence diagnostics alongside corrected bands.
- Produces analysis-ready corrected optical outputs for downstream monitoring/classification.

#### How It Works

- Derives slope/aspect illumination terms from the DEM and solar geometry parameters.
- Applies per-band topographic normalization (C-correction style) to reduce relief-driven bias.
- Generates cloud-shadow and quality layers from correction residual behavior and masking rules.
- Indicative formula: L_corr ~= L_obs * (cos(theta_s) + C) / (cos(i) + C), where i is incidence angle from DEM slope/aspect.

#### Why It Wins

- Integrates correction and QA/mask outputs in one workflow rather than forcing separate ad hoc preprocessing scripts.

#### Typical Buying Trigger

Teams see unstable index/change outputs across steep terrain and need a standardized correction stage.

#### Typical Presets

- conservative: stricter correction and masking behavior.
- balanced: default profile.
- fast: quicker execution on large scenes.

### Inputs

ParameterOptionalDescription
input_red, input_nir, input_demnoCore optical + terrain inputs used for topographic/reflectance normalization workflows.
optional input_greenyesOptional green band used by workflows that include green-channel diagnostics.
solar_zenith_deg, solar_azimuth_degnoSolar geometry parameters used to model illumination and terrain incidence effects.
profile: conservative | balanced | fastnoProcessing profile controlling quality-vs-throughput behavior for correction workflow execution.

### Outputs

ParameterTypeDescription
corrected optical bandsGeoTIFFTerrain-corrected optical bands for downstream index and classification workflows.
cloud_shadow_maskGeoTIFFCloud and shadow mask used to suppress unreliable optical pixels.
topographic_correction_factorGeoTIFFPer-pixel topographic correction factor used in radiometric normalization.
quality_confidenceGeoTIFFConfidence surface indicating reliability of corrected optical values.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.terrain_corrected_optical_analytics(
    input_red="data/red.tif",
    input_nir="data/nir.tif",
    input_dem="data/dem.tif",
    solar_zenith_deg=40.0,
    solar_azimuth_deg=165.0,
    profile="balanced",
    output_prefix="output/tco",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.
