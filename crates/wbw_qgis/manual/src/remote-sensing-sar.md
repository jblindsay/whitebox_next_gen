# SAR Processing


---

## Cloude Pottier Decomposition

**Function name:** `cloude_pottier_decomposition`


*No help documentation available for this tool.*


---

## Enhanced Lee Filter

**Function name:** `enhanced_lee_filter`


*No help documentation available for this tool.*


---

## Freeman Durden Decomposition

**Function name:** `freeman_durden_decomposition`


*No help documentation available for this tool.*


---

## Frost Filter

**Function name:** `frost_filter`


Experimental

Speckle reduction for SAR intensity imagery using Frost adaptive filtering.

remote_sensing raster filter frost_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Local window radius in pixels (default 2).Optional`2`
`damping_factor`Frost damping factor controlling exponential decay (default 2.0).Optional`2.0`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies frost_filter to an input raster.*
`wbe.frost_filter(input='image.tif', output='frost_filter.tif')`


---

## Gamma Map Filter

**Function name:** `gamma_map_filter`


Experimental

Gamma-MAP speckle filter for SAR imagery with ENL-aware noise modeling.

remote_sensing raster filter gamma_map_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Local window radius in pixels (default 2).Optional`2`
`enl`Equivalent number of looks (default 1.0).Optional`1.0`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies gamma_map_filter to an input raster.*
`wbe.gamma_map_filter(input='image.tif', output='gamma_map_filter.tif')`


---

## H Alpha Wisart Classification

**Function name:** `h_alpha_wisart_classification`


*No help documentation available for this tool.*


---

## Kuan Filter

**Function name:** `kuan_filter`


Experimental

Kuan adaptive speckle filter for SAR intensity data.

remote_sensing raster filter kuan_filter legacy-port

### Parameters

NameDescriptionRequiredDefault
`input`Input raster path or typed raster object.Required`input.tif`
`radius`Local window radius in pixels (default 2).Optional`2`
`enl`Equivalent number of looks (default 1.0).Optional`1.0`
`output`Optional output path. If omitted, output remains in memory.Optional—

### Examples

*Applies kuan_filter to an input raster.*
`wbe.kuan_filter(input='image.tif', output='kuan_filter.tif')`


---

## Refined Lee Filter

**Function name:** `refined_lee_filter`


*No help documentation available for this tool.*


---

## SAR Analysis Readiness

**Function name:** `sar_analysis_readiness`


PROProduction

Evaluate SAR scene readiness for downstream analysis, including optional terrain and pairwise coherence-proxy checks.

workflow pro

### Workflow Narrative

**SAR Readiness QA**

#### Problem It Solves

Are SAR scenes normalized enough for robust multi-scene comparison and downstream analytics?

#### Who It Is For

- SAR specialists and all-weather monitoring teams.

#### Primary User

Disaster/hazard monitoring units, remote sensing service firms, and infrastructure monitoring groups.

#### What It Does

- Produces analysis-ready SAR derivatives from SAR + DEM.
- Applies calibration, speckle filtering, and RTC-support factor generation.
- Accepts either direct SAR rasters or supported SAR bundles and records bundle metadata provenance.
- Optionally emits a pair-based coherence proxy and can auto-coregister the pair first when explicit opt-in alignment is requested.

#### How It Works

- Converts input SAR intensity to calibrated backscatter-compatible values.
- Applies configurable speckle-window filtering for noise suppression.
- Computes terrain correction factors from DEM geometry, resolves bundle metadata when present, and optionally evaluates a pair coherence proxy.
- Indicative formula: gamma0 ~= sigma0 / cos(local_incidence), then local-window filtering and optional pair coherence estimation.

#### Why It Wins

- Couples SAR preprocessing with terrain support, bundle-native provenance, and explicit geometry guardrails, while still allowing an audited auto-coregistration handoff when pair alignment is not pre-established.
- Boundary note:
- `sar_analysis_readiness` is not the dedicated interferogram+coherence production workflow.
- Dedicated companion tool (single combined workflow): `sar_interferogram_coherence`.
- Design spec and scope details: `docs/sar_interferogram_coherence_spec.md`.

#### Typical Buying Trigger

An organization needs dependable all-weather monitoring where optical imagery is frequently unavailable.

#### Typical Presets

- default with single scene for preprocessing.
- pair mode with either direct raster pairs or bundle-resolved pair inputs for coherence-proxy output.

### Inputs

ParameterOptionalDescription
input_sar or input_sar_bundle, input_demnoPrimary SAR source plus terrain model used for radiometric terrain correction and readiness metrics.
input_measurement_keyyesOptional bundle measurement selector when the input SAR bundle contains multiple assets.
optional pair_sar or pair_sar_bundle and look-angle controlsyesOptional secondary SAR source and geometry controls used for pair-based coherence-proxy diagnostics.
pair_measurement_keyyesOptional bundle measurement selector when the pair SAR bundle contains multiple assets.
auto_coregister_pair, coreg_max_offset_px, coreg_decimation, coreg_min_overlap_fractionyesOptional opt-in handoff to translation-mode pair alignment before coherence-proxy estimation when CRS/grid do not already match.
speckle_window, z_factornoNoise-filter window and vertical scaling controls used in SAR preprocessing.

### Outputs

ParameterTypeDescription
sar_backscatter_calibratedGeoTIFFCalibrated SAR backscatter raster suitable for quantitative comparison.
speckle_filteredGeoTIFFSpeckle-reduced SAR raster for improved interpretability and downstream analysis.
rtc_factorGeoTIFFRadiometric terrain correction factor raster used to normalize SAR signal.
coherence_proxyoptional GeoTIFFOptional pair-based amplitude-domain coherence proxy raster when a compatible SAR pair is provided.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, and key metrics.
html_reportHTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.sar_analysis_readiness(
    input_sar_bundle="data/S1_reference.SAFE",
    input_measurement_key="vv",
    input_dem="data/dem.tif",
    pair_sar_bundle="data/S1_pair.SAFE",
    pair_measurement_key="vv",
    auto_coregister_pair=True,
    coreg_max_offset_px=24,
    speckle_window=5,
    output_prefix="output/sar_ready",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## SAR Coregistration

**Function name:** `sar_coregistration`


PROProduction

Coregister moving SAR imagery to a reference grid (translation/affine/local-offset-grid modes).

workflow pro

### Workflow Narrative

**SAR Coregistration**

#### Problem It Solves

Can we put a moving SAR scene on the reference scene grid with enough geometric confidence for pairwise analytics?

#### Who It Is For

- SAR analysts, infrastructure monitoring teams, and registration-first EO pipelines.

#### Primary User

InSAR/infrastructure monitoring teams, geospatial intelligence groups, and SAR QA operations.

#### What It Does

- Aligns a moving SAR raster or resolved SAR-bundle measurement onto a reference SAR grid.
- Supports `translation`, `affine`, and `local_offset_grid` modes (`affine` and `local_offset_grid` remain experimental).
- Emits aligned output plus transform/summary diagnostics, including compatibility and acceptance gates for machine-checkable downstream use.

#### How It Works

- Harmonizes the moving SAR raster to the reference CRS/grid when needed and evaluates strict scene compatibility where metadata allows.
- Applies amplitude-domain matching with bounded search and subpixel refinement, then executes the selected transform model.
- Emits per-run QA including burst diagnostics (Sentinel-1), continuity checks, and Phase-A acceptance gates.
- Indicative formula: $\rho(\Delta x, \Delta y) = \mathrm{corr}(\log(1 + I_{ref}), \log(1 + I_{mov}(x-\Delta x, y-\Delta y)))$.

#### Why It Wins

- Accepts both direct rasters and supported SAR bundles while turning pair alignment into a reproducible audited workflow artifact instead of an opaque manual preprocessing step.

#### Typical Buying Trigger

A team needs a defensible alignment stage before coherence, pair differencing, or cross-sensor fusion.

#### Typical Presets

- translation: default global shift path.
- affine: global affine refinement (experimental).
- local_offset_grid: local residual warp model (experimental).

### Inputs

ParameterOptionalDescription
reference_sar or reference_sar_bundlenoReference SAR source provided either as a raster path or a supported SAR bundle.
moving_sar or moving_sar_bundlenoMoving SAR source provided either as a raster path or a supported SAR bundle.
reference_measurement_key, moving_measurement_keyyesOptional bundle measurement selectors when either SAR bundle contains multiple measurement assets.
coreg_modeyesCoregistration mode: `translation`, `affine`, or `local_offset_grid` (experimental for affine/local).
max_offset_px, decimation, min_overlap_fractionyesSearch-radius and sampled-overlap controls for correlation-based shift estimation.
input_dem, dem_z_factoryesOptional DEM-assisted initialization controls used for geometry-informed matching support.
phase_a_* thresholdsyesOptional acceptance/continuity threshold overrides for deterministic quality gating.
resample_method, output_prefixyesOutput resampling mode and artifact prefix.

### Outputs

ParameterTypeDescription
moving_alignedGeoTIFFMoving SAR raster resampled onto the reference SAR grid.
offset_xGeoTIFFConstant x-offset surface in map units for the estimated translation.
offset_yGeoTIFFConstant y-offset surface in map units for the estimated translation.
transformJSONMachine-readable transform and QA summary for the estimated alignment.
summaryJSONMachine-readable workflow report containing parameters, QA diagnostics, and artifact paths.
html_reportHTMLHuman-readable report generated from the summary contract for review and traceability.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.sar_coregistration(
    reference_sar_bundle="data/S1_reference.SAFE",
    reference_measurement_key="vv",
    moving_sar_bundle="data/S1_pair.SAFE",
    moving_measurement_key="vv",
    max_offset_px=24,
    decimation=4,
    resample_method="bilinear",
    output_prefix="output/sar_coreg",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## SAR Interferogram Coherence

**Function name:** `sar_interferogram_coherence`


PROProduction

Estimate interferometric coherence between SAR acquisitions (direct, complex, or bundle modes).

workflow pro

### Workflow Narrative

**SAR Interferogram and Coherence**

#### Problem It Solves

Can this SAR pair be converted into defensible interferogram/coherence products with auditable provenance and QA diagnostics?

#### Who It Is For

- InSAR practitioners and SAR operations teams requiring repeatable pair-level coherence/interferogram products.

#### Primary User

Infrastructure deformation monitoring teams, hazard intelligence groups, and SAR analytics operations.

#### What It Does

- Produces interferogram, coherence, and valid-mask outputs from compatible SAR pairs in one dedicated workflow.
- Accepts either direct SAR rasters, supported SAR bundles, or complex split real/imag SAR inputs.
- Emits standardized machine-readable summary and optional HTML report artifacts for auditable downstream use.

#### How It Works

- Resolves reference and moving SAR inputs from either direct raster mode, bundle mode, or complex component mode.
- Optionally performs internal `sar_coregistration` handoff when grids are mismatched, unless the caller explicitly asserts a prealigned pair.
- Computes either complex-domain interferometric phase and coherence magnitude (complex mode) or amplitude-domain proxy interferogram/coherence (scalar mode).
- Uses summed-area coherence kernels and parallel row-chunk evaluation to reduce runtime on larger scenes; optional fast mode can further decimate coherence sampling.
- Indicative formulas: $\phi = \mathrm{atan2}(\Im(z), \Re(z))$ for interferometric phase where $z = s_{ref}\,\overline{s_{mov}}$, and local-window coherence proxy from normalized cross-correlation magnitude.

#### Why It Wins

- Unifies SAR pair ingest, optional alignment handoff, product generation, acceptance gating, and QA/report artifacts in one reproducible contract workflow.
- Operational note:
- When `auto_coregister_pair=true`, this tool invokes the existing `sar_coregistration` engine internally if the pair grids do not already match.
- If the pair is known to already be aligned, prefer `assume_prealigned_pair=true` and leave `auto_coregister_pair=false` to avoid unnecessary alignment work.
- Failed acceptance on the coreg residual gate indicates the pair/alignment should not be treated as registration-quality enough for trusted downstream interpretation.

#### Typical Buying Trigger

Teams need a dedicated production stage for interferogram/coherence outputs instead of ad hoc post-processing scripts.

#### Typical Presets

- scalar raster pair: direct amplitude-domain processing with optional auto-coregistration.
- bundle pair: bundle-native measurement resolution with identical output contract.
- complex split mode: complex-domain phase/coherence computation from explicit real/imag rasters.
- fast large-scene mode: reduced coherence workload via `performance_profile="fast"`, optional `coherence_decimation`, and selective artifact writing.

### Inputs

ParameterOptionalDescription
reference_sar or reference_sar_bundleno (unless complex mode)Reference SAR source provided either as a direct raster path or supported SAR bundle root.
moving_sar or moving_sar_bundleno (unless complex mode)Moving SAR source provided either as a direct raster path or supported SAR bundle root.
reference_sar_real, reference_sar_imag, moving_sar_real, moving_sar_imagno (complex mode)Complex input mode components for explicit complex-domain interferogram/coherence processing.
reference_measurement_key, moving_measurement_keyyesOptional bundle measurement selectors when SAR bundles include multiple measurement assets.
auto_coregister_pair, assume_prealigned_pair, coreg_modeyesPair-alignment controls: either invoke internal `sar_coregistration` when needed or explicitly assert the pair is already aligned.
coreg_max_offset_px, coreg_decimation, coreg_min_overlap_fractionyesOptional scalar-mode coreg handoff tuning controls for search radius, sampling stride, and minimum overlap.
coherence_window, performance_profile, coherence_decimationyesCoherence kernel controls. Fast mode can reduce effective window size and decimate coherence sampling on very large scenes.
input_demyesOptional DEM used for terrain-context masking and geometry-support pathways.
write_interferogram, write_coherence, write_valid_mask, write_html_reportyesOptional artifact suppression controls used to reduce heavy-run output cost.
output_layout, output_compression, output_tile_sizeyesGeoTIFF write-profile controls for standard vs COG-style output and compression behavior.
output_prefixyesOutput artifact prefix.

### Outputs

ParameterTypeDescription
interferogramoptional GeoTIFFInterferogram raster (complex phase in complex mode, amplitude-domain proxy in scalar mode) when writing is enabled.
coherenceoptional GeoTIFFCoherence magnitude raster from local-window pair statistics when writing is enabled.
valid_maskoptional GeoTIFFBinary/flag mask indicating valid pair-support pixels used during computation when writing is enabled.
summaryJSONMachine-readable summary report containing run metadata, QA diagnostics, timings, and artifact paths.
html_reportoptional HTMLHuman-readable customer-facing report generated from the summary contract for stakeholder review and QA traceability when writing is enabled.

### Python Example

`import whitebox_workflows as wbw

wbe = wbw.WbEnvironment(include_pro=True, tier="pro")

result = wbe.sar_interferogram_coherence(
    reference_sar="output/sar_coreg_reference.tif",
    moving_sar="output/sar_coreg_moving_aligned.tif",
    assume_prealigned_pair=True,
    coherence_window=7,
    performance_profile="balanced",
    output_prefix="output/sar_ifg_coh",
)

print(result)`

### License Notice

Use of this function requires a license for Whitebox Workflows Professional (WbW-Pro). Please visit `www.whiteboxgeo.com` to purchase a license.


---

## Wisart Iterative Clustering

**Function name:** `wisart_iterative_clustering`


*No help documentation available for this tool.*


---

## Yamaguchi 4component Decomposition

**Function name:** `yamaguchi_4component_decomposition`


*No help documentation available for this tool.*
