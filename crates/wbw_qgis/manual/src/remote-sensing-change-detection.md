# Change Detection


---

## Change Vector Analysis

**Function name:** `change_vector_analysis`


Change Vector Analysis (CVA) is a change detection method that characterizes the magnitude and change direction in spectral space between two times. A change vector is the difference vector between two vectors in n-dimensional feature space defined for two observations of the same geographical location (i.e. corresponding pixels) during two dates. The CVA inputs include the set of raster images corresponding to the multispectral data for each date. Note that there must be the same number of image files (bands) for the two dates and they must be entered in the same order, i.e. if three bands, red, green, and blue are entered for date one, these same bands must be entered in the same order for date two. 

CVA outputs two image files. The first image contains the change vector length, i.e. magnitude, for each pixel in the multi-spectral dataset. The second image contains information about the direction of the change event in spectral feature space, which is related to the type of change event, e.g. deforestation will likely have a different change direction than say crop growth. The vector magnitude is a continuous numerical variable. The change vector direction is presented in the form of a code, referring to the multi-dimensional sector in which the change vector occurs. A text output will be produced to provide a key describing sector codes, relating the change vector to positive or negative shifts in n-dimensional feature space. 

It is common to apply a simple thresholding operation on the magnitude data to determine 'actual' change (i.e. change above some assumed level of error). The type of change (qualitatively) is then defined according to the corresponding sector code. Jensen (2015) provides a useful description of this approach to change detection. 

### Reference

 

Jensen, J. R. (2015). Introductory Digital Image Processing: A Remote Sensing Perspective. 

### See Also

 

`write_function_memory_insertion` 

### Python API

```python
def change_vector_analysis(self, date1_rasters: List[Raster], date2_rasters: List[Raster]) -> Tuple[Raster, Raster, str]:
```


---

## Image Difference Change Detection

**Function name:** `image_difference_change_detection`


*No help documentation available for this tool.*


---

## PCA Based Change Detection

**Function name:** `pca_based_change_detection`


*No help documentation available for this tool.*


---

## Post Classification Change

**Function name:** `post_classification_change`


*No help documentation available for this tool.*


---

## Remote Sensing Change Detection

**Function name:** `remote_sensing_change_detection`


PROProduction

Detect spectral change between baseline and change-date multispectral bundles with profile-based sensitivity.

workflow pro

### Workflow Narrative

**Remote Sensing Change Detection**

#### Problem It Solves

Where is meaningful vegetation change occurring, and which detections are reliable enough for reporting?

#### Who It Is For

- Environmental monitoring analysts and EO operations teams.

#### Primary User

Environmental consultancies, forestry/conservation agencies, and compliance programs.

#### What It Does

- Detects vegetation loss/gain between two dates using NDVI-style change logic.
- Uses bundle-native multiband inputs with explicit red/NIR band indices.
- Produces a signed change raster and a confidence raster for analyst triage.

#### How It Works

- Computes NDVI-like indices per date from selected red and NIR bands.
- Calculates signed per-pixel delta between change-date and baseline NDVI-like response.
- Applies profile-dependent thresholds and local consistency checks to derive confidence.
- Indicative formula: NDVI = (NIR - Red) / (NIR + Red), then change = NDVI_change - NDVI_baseline.

#### Why It Wins

- Bundle-native inputs reduce parameter mismatch errors and make cross-date processing more reproducible.

#### Typical Buying Trigger

A client or regulator requires repeatable, confidence-scored vegetation change evidence for periodic reporting.

#### Typical Presets

- aggressive: lower change threshold, more sensitive detection.
- balanced: default tradeoff for general monitoring.
- conservative: stricter thresholds, fewer false positives.

### Inputs

ParameterOptionalDescription
baseline_bundle, baseline_red_band_index, baseline_nir_band_indexnoBaseline multispectral bundle and red/NIR band selectors used to compute baseline vegetation response.
change_bundle, change_red_band_index, change_nir_band_indexnoChange-date multispectral bundle and red/NIR band selectors used for signed change estimation.
optional intermediate_ndviyesIntermediate-date NDVI raster used to strengthen temporal plausibility scoring.
profile: aggressive | balanced | conservativeyesProcessing profile controlling sensitivity, quality strictness, and runtime tradeoffs.
high_confidence_thresholdyesConfidence threshold used for summary metrics (default `0.85`).

### Outputs

ParameterTypeDescription
change_mapGeoTIFFPrimary change-intelligence raster showing direction and magnitude of detected change.
confidenceGeoTIFFConfidence layer quantifying reliability of modeled outputs.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

change_map, confidence, summary = wbe.remote_sensing_change_detection(
    baseline_bundle="data/baseline_bundle.tif",
    baseline_red_band_index=0,
    baseline_nir_band_index=1,
    change_bundle="data/change_bundle.tif",
    change_red_band_index=0,
    change_nir_band_index=1,
    profile="balanced",
    output="output/rs_change",
)

print(change_map)
print(confidence)
print(summary)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Time Series Change Intelligence

**Function name:** `time_series_change_intelligence`


PROProduction

Workflow-grade Pro analysis with audit-ready outputs.

workflow pro

### Workflow Narrative

**Time-Series Change Analysis**

#### Problem It Solves

Where and when are structural shifts emerging in the time series, and how confident are those signals?

#### Who It Is For

- Time-series monitoring teams and geospatial data science groups.

#### Primary User

Regional planning programs, policy/compliance analytics groups, and EO product teams.

#### What It Does

- Performs multi-date trend and breakpoint analysis from temporal stacks.
- Supports mode-dependent decomposition/segmentation behavior.
- Emits confidence-scored temporal change surfaces.

#### How It Works

- Traverses each pixel time series and estimates trend response over the observation window.
- Uses algorithm_mode-specific breakpoint logic to detect structural shifts.
- Converts fit strength, breakpoint support, and observation sufficiency into confidence output.
- Indicative formula: confidence ~= f(|trend|, breakpoint_support, n_observations), bounded to [0, 1].

#### Why It Wins

- Provides mode flexibility (fast/iterative/bfast) so users can balance throughput and analytical depth.

#### Typical Buying Trigger

A monitoring program moves from two-date snapshots to sustained time-series surveillance.

#### Typical Presets

- fast: throughput-oriented screening.
- iterative: stronger breakpoint refinement.
- bfast: decomposition-oriented analysis.

### Inputs

ParameterOptionalDescription
input_stack (required)noPrimary temporal raster stack used for trend and breakpoint analysis.
optional qa_stackyesOptional temporal QA stack used to weight or suppress low-quality observations.
algorithm_mode and thresholding controlsnoTime-series change algorithm mode and threshold controls.

### Outputs

ParameterTypeDescription
trend_changeGeoTIFFPrimary change-intelligence raster showing direction and magnitude of detected change.
breakpoint_countGeoTIFFPer-pixel count of detected temporal breakpoints.
breakpoint_dateGeoTIFFEstimated timing raster for dominant detected breakpoint events.
change_confidenceGeoTIFFConfidence surface for the time-series change detection result.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.time_series_change_intelligence(
    input_stack="data/time_stack.tif",
    qa_stack="data/time_stack_qa.tif",
    algorithm_mode="bfast",
    output_prefix="output/ts_change",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Write Function Memory Insertion

**Function name:** `write_function_memory_insertion`


Jensen (2015) describes write function memory (WFM) insertion as a simple yet effective method of visualizing land-cover change between two or three dates. WFM insertion may be used to qualitatively inspect change in any type of registered, multi-date imagery. The technique operates by creating a red-green-blue (RGB) colour composite image based on co-registered imagery from two or three dates. If two dates are input, the first date image will be put into the red channel, while the second date image will be put into both the green and blue channels. The result is an image where the areas of change are displayed as red (date 1 is brighter than date 2) and cyan (date 1 is darker than date 2), and areas of little change are represented in grey-tones. The larger the change in pixel brightness between dates, the more intense the resulting colour will be. 

If images from three dates are input, the resulting composite can contain many distinct colours. Again, more intense the colours are indicative of areas of greater land-cover change among the dates, while areas of little change are represented in grey-tones. Interpreting the direction of change is more difficult when three dates are used. Note that for multi-spectral imagery, only one band from each date can be used for creating a WFM insertion image. 

### Reference

 

Jensen, J. R. (2015). Introductory Digital Image Processing: A Remote Sensing Perspective. 

### See Also

 

`create_colour_composite`, `change_vector_analysis` 

### Python API

```python
def write_function_memory_insertion(self, image1: Raster, image2: Raster, image3: Raster) -> Raster:
```
