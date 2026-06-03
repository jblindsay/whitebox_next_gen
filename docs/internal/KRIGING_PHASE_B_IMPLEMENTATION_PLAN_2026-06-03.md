# Kriging Phase B Implementation Plan
**Date:** 2026-06-03  
**Status:** Design Phase  
**Quality Target:** Production Grade  
**Estimated Effort:** 3–5 weeks (MVP); 6–8 weeks (full Phase B with anisotropy & drift)

---

## Executive Summary

Phase B introduces geostatistics via **Ordinary Kriging (OK)** with robust variogram estimation and numerical stability guarantees. This is a production-grade implementation intended for real-world interpolation workflows, competitive with commercial GIS systems.

**Key Deliverables (MVP):**
1. Empirical semivariogram calculator with lag binning & cloud computation
2. Variogram model fitter (Spherical, Exponential, Gaussian families)
3. Ordinary Kriging predictor with regularized Cholesky solve
4. Cross-validation metrics (RMSE, MAE, standardized residuals)
5. Prediction surface (value + variance raster outputs)
6. Python/R/QGIS wrappers with consistent parameter exposure

**Phase B+ (Scope Deferral):**
- Universal Kriging (trend removal)
- Anisotropic variograms
- Co-kriging
- Block kriging
- Indicator kriging

---

## Core Architecture

### 1. Variogram Pipeline

**1.1 Empirical Semivariogram**
- Input: point cloud with attribute values
- Process:
  - Lag binning (uniform or adaptive; configurable lag distance)
  - Pairwise distance/value-difference accumulation
  - Bin aggregation (mean, median, robust estimators)
  - Cloud output (optional diagnostic artifact)
- Output: lag distance bins, semivariance values, point counts per bin
- Robustness:
  - Handle colocated points (same location, different values) → store separately
  - Outlier detection via IQR/robust scaling (optional flag)
  - Sparse lag handling: suppress bins with < 10 pairs (configurable threshold)

**1.2 Variogram Model Fitting**
- Supported model families (MVP):
  - **Spherical**: $\gamma(h) = c_0 + c \left[ 1.5 \frac{h}{a} - 0.5 \left( \frac{h}{a} \right)^3 \right]$ for $h \leq a$; $c_0 + c$ otherwise
  - **Exponential**: $\gamma(h) = c_0 + c \left[ 1 - \exp\left( -\frac{3h}{a} \right) \right]$
  - **Gaussian**: $\gamma(h) = c_0 + c \left[ 1 - \exp\left( -\frac{3h^2}{a^2} \right) \right]$
- Parameters:
  - $c_0$ (nugget): micro-scale variance
  - $c$ (sill): total variance contribution
  - $a$ (range): distance of spatial correlation
- Fitting strategy:
  - Weighted least-squares (weights = bin point counts)
  - Robust initial guess from empirical data (max lag, max variance)
  - Bounded optimization (range > 0, sill ≥ nugget)
  - Regularization: penalize extreme range/sill ratios
- Output: fitted model object (family, parameters, residual error)

### 2. Kriging Solver

**2.1 Ordinary Kriging System**
- For $n$ observation points and 1 prediction location:
  - Build $n \times n$ covariance matrix $\mathbf{K}$ using fitted variogram
  - Add Lagrange multiplier row/column for mean constraint
  - Solve $(n+1) \times (n+1)$ system for weights
  - Compute prediction and prediction variance
- Numerical stability:
  - **Cholesky decomposition** with regularization: $\mathbf{K} \leftarrow \mathbf{K} + \epsilon \mathbb{I}$ (nugget effect)
  - **Condition number check**: warn/error if $\kappa(\mathbf{K}) > 10^{10}$
  - **Fallback to pseudo-inverse**: if Cholesky fails
  - **Colocated observations handling**: downweight or skip near-exact duplicates

**2.2 Batch Prediction**
- Input: grid cells (raster domain) or point set
- Parallelization:
  - Rayon par_iter over prediction locations
  - Each thread solves independent Kriging system
  - Accumulate predictions & variances
- Output: prediction raster, variance raster

### 3. Cross-Validation Pipeline

- Leave-one-out (LOO) or k-fold CV
- Per-fold metrics:
  - **Bias**: mean(predicted − observed)
  - **RMSE**: sqrt(mean((predicted − observed)²))
  - **MAE**: mean(|predicted − observed|)
  - **Standardized residuals**: (predicted − observed) / variance^0.5
    - Should be ~ N(0, 1) under correct model
  - **Spatial correlation of residuals**: flag systematic bias by region
- Diagnostic plot suggestions:
  - Predicted vs. observed scatter
  - Residual spatial map
  - Q–Q plot of standardized residuals

---

## Task Breakdown (MVP)

### Task 1: Variogram Core (Week 1)
**Owner:** Backend (Rust)  
**Deliverable:** `wbgeostat` crate with variogram estimation & fitting

- [ ] **1.1** Empirical semivariogram calculator
  - Lag binning logic (uniform, adaptive distance)
  - Pairwise accumulation with filtering
  - Output: binned lag/semivariance vectors
  - Tests: synthetic isotropic field, known analytical variogram

- [ ] **1.2** Variogram model fitter
  - Spherical, Exponential, Gaussian families
  - Weighted least-squares optimizer
  - Robust initial parameter guess
  - Tests: fit to synthetic variograms, verify sill/range/nugget

- [ ] **1.3** Colocated point handling
  - Detect duplicates (e.g., same location, different Z)
  - Store separately; downweight in fitting
  - Tests: colocated obs. → correct semivariance exclusion

**Acceptance Criteria:**
- Empirical variogram on 200-point synthetic grid matches R geostat::variogram within 5%
- Fitted model sill/range/nugget estimates converge correctly

---

### Task 2: Kriging Solver (Week 2)
**Owner:** Backend (Rust)  
**Deliverable:** `wbgeostat` kriging prediction engine

- [ ] **2.1** Ordinary Kriging OK matrix builder
  - Covariance matrix from fitted variogram
  - Lagrange constraint for mean
  - Condition number diagnostics
  - Tests: known point set → correct covariance matrix

- [ ] **2.2** Regularized solver (Cholesky with fallback)
  - Cholesky decomposition with nugget regularization
  - Condition number check
  - Pseudo-inverse fallback
  - Tests: ill-conditioned problems → graceful degradation

- [ ] **2.3** Batch prediction with rayon
  - Parallel prediction loop
  - Accumulate predictions + variances
  - Handle edge cases (prediction at observation locations)
  - Tests: 1000-cell grid → correct output shape & variance ≥ 0

**Acceptance Criteria:**
- Single-point prediction on known data set matches R geostat::kriging within 2%
- Variance decreases at observation locations
- Batch prediction speedup ≥ 4× on 8-core system

---

### Task 3: Cross-Validation & Diagnostics (Week 2.5)
**Owner:** Backend (Rust)  
**Deliverable:** CV metrics and diagnostic helpers

- [ ] **3.1** LOO cross-validation harness
  - Iterative leave-one-out
  - Accumulate residuals & predictions
  - Tests: synthetic data → known CV error

- [ ] **3.2** Metric computation
  - Bias, RMSE, MAE, standardized residuals
  - Early termination if standardized residuals show systematic skew
  - Tests: normal residuals → Q–Q flatness

- [ ] **3.3** Optional diagnostic outputs
  - Residual raster (if prediction domain is raster)
  - CV summary JSON or CSV report
  - Tests: output format validation

**Acceptance Criteria:**
- CV metrics reproducible across runs
- Residuals ~ N(0,1) on synthetic Gaussian field

---

### Task 4: Python Wrapper & Integration (Week 3)
**Owner:** Python bindings (PyO3)  
**Deliverable:** `whitebox_workflows` Python API

- [ ] **4.1** Tool definitions
  - `variogram_estimation`: point vector + attribute → variogram diagram JSON
  - `fit_variogram_model`: variogram + model family → fitted model params
  - `ordinary_kriging`: raster/grid + point vector + attribute + model → prediction + variance rasters
  - `kriging_cross_validation`: point vector + model → CV metrics + diagnostic report

- [ ] **4.2** Parameter exposure
  - Lag distance, lag tolerance, max lag count
  - Model family (spherical|exponential|gaussian)
  - Regularization epsilon, condition number threshold
  - CV type (LOO|k-fold), number of folds
  - Standardized residual tolerance for flagging

- [ ] **4.3** Output handling
  - Prediction + variance rasters (write to disk or memory)
  - CV report as JSON or CSV
  - Model parameters as JSON metadata

**Acceptance Criteria:**
- Python calls kriging tool successfully
- Output rasters load correctly in GDAL/rasterio
- All parameters controllable from Python

---

### Task 5: R Wrapper & Integration (Week 3.5)
**Owner:** R bindings (extendr)  
**Deliverable:** `whiteboxworkflows` R API

- [ ] **5.1** Tool wrappers
  - `wbw_variogram_estimation(...)`
  - `wbw_fit_variogram_model(...)`
  - `wbw_ordinary_kriging(...)`
  - `wbw_kriging_cross_validation(...)`

- [ ] **5.2** Session integration
  - Methods on session object (`s$ordinary_kriging(...)`)
  - Consistency with Python parameter names
  - Data frame / spatial object interop

- [ ] **5.3** Documentation & examples
  - Roxygen docstrings with usage examples
  - Tutorial Rmd showing workflow (points → kriging → diagnostics)

**Acceptance Criteria:**
- R calls kriging tools via session
- Output rasters readable in R (raster/sf packages)
- Example script runs end-to-end

---

### Task 6: QGIS Plugin Integration (Week 4)
**Owner:** QGIS plugin  
**Deliverable:** Processing tools + algorithm descriptions

- [ ] **6.1** Processing provider registration
  - Register 4 kriging tools in plugin
  - Parameter definitions for QGIS UI
  - Output layer registration

- [ ] **6.2** Tool descriptions & help
  - `descriptions/geostatistics_kriging.json`
  - Parameter tooltips (lag distance units, model family advice)
  - Output raster layer naming conventions

- [ ] **6.3** UX improvements
  - Variogram plot preview (optional: matplotlib on success)
  - Model fit diagnostics in QGIS message bar
  - Progress feedback during kriging

**Acceptance Criteria:**
- Tools visible in QGIS Processing toolbox
- Parameter UI matches Python/R naming
- Execution produces correct outputs in QGIS project

---

### Task 7: Validation & Benchmarking (Week 4.5)
**Owner:** QA / Backend  
**Deliverable:** Validation suite + performance baseline

- [ ] **7.1** Parity tests (R geostat package)
  - Meuse dataset: fit variogram, OK prediction → compare to gstat
  - Tolerance: ≤ 2% prediction difference
  - Tolerance: ≤ 5% variance difference
  - Tests: documented in `tests/kriging_parity.rs`

- [ ] **7.2** Numerical stability tests
  - Ill-conditioned covariance matrices
  - Colocated / near-duplicate observations
  - Extreme range/sill ratios
  - Tests: no panics, graceful error messages

- [ ] **7.3** Performance baseline
  - 1000-point cloud → 100×100 grid kriging time (target: < 5 sec)
  - Parallelization scaling (1/2/4/8 threads)
  - Memory usage (target: < 500 MB for typical workflows)
  - Benchmark harness in `benches/kriging_bench.rs`

**Acceptance Criteria:**
- Parity tests pass within tolerance
- No panics on edge cases
- Performance ≥ 8× speedup on 8-core system

---

### Task 8: Documentation & Release (Week 5)
**Owner:** Docs  
**Deliverable:** User manuals + API reference

- [ ] **8.1** Python manual chapter: Kriging workflows
  - Example: soil property interpolation
  - Variogram interpretation guide
  - Cross-validation workflow

- [ ] **8.2** R manual chapter
  - Parallel example: same workflow in R
  - Session integration pattern

- [ ] **8.3** API reference updates
  - Tool call paths (Python/R)
  - Parameter descriptions
  - Output raster metadata

- [ ] **8.4** Changelog entry
  - Phase B kriging MVP release notes
  - Known limitations (no anisotropy yet)
  - Breaking changes: none expected

**Acceptance Criteria:**
- Users can follow example scripts end-to-end
- API reference is complete and accurate

---

## API Contract (MVP)

### Python
```python
import whitebox_workflows as wbw

wbe = wbw.WbEnvironment()

# Fit variogram
variogram_data = wbe.geostatistics.variogram_estimation(
    input_vector="soil_samples.shp",
    field="ph_value",
    lag_distance=100.0,
    lag_tolerance=50.0,
    max_lag_count=20
)
# Output: JSON with lag distances, semivariances, point counts

# Fit model
model = wbe.geostatistics.fit_variogram_model(
    variogram_data=variogram_data,
    model_family="spherical",  # or "exponential", "gaussian"
    nugget_initial=0.1,
    output_params="model_params.json"
)

# Ordinary Kriging prediction
prediction = wbe.geostatistics.ordinary_kriging(
    input_vector="soil_samples.shp",
    field="ph_value",
    model_params="model_params.json",
    output_prediction="kriged_ph.tif",
    output_variance="kriged_variance.tif",
    base_raster="dem.tif",  # or specify resolution
    cell_size=100.0
)

# Cross-validation
cv_result = wbe.geostatistics.kriging_cross_validation(
    input_vector="soil_samples.shp",
    field="ph_value",
    model_params="model_params.json",
    output_report="cv_diagnostics.json"
)
print(cv_result["rmse"])  # e.g., 0.35
```

### R
```r
library(whiteboxworkflows)

s <- wbw_session()

# Fit variogram
vario <- wbw_variogram_estimation(
  input_vector = "soil_samples.shp",
  field = "ph_value",
  lag_distance = 100.0,
  output = "variogram.json",
  session = s
)

# Fit model
model <- wbw_fit_variogram_model(
  variogram_data = "variogram.json",
  model_family = "spherical",
  output = "model_params.json",
  session = s
)

# Ordinary Kriging
wbw_ordinary_kriging(
  input_vector = "soil_samples.shp",
  field = "ph_value",
  model_params = "model_params.json",
  output_prediction = "kriged_ph.tif",
  output_variance = "kriged_variance.tif",
  base_raster = "dem.tif",
  cell_size = 100.0,
  session = s
)

# Cross-validation
cv <- wbw_kriging_cross_validation(
  input_vector = "soil_samples.shp",
  field = "ph_value",
  model_params = "model_params.json",
  output_report = "cv_report.json",
  session = s
)
cat("RMSE:", cv$rmse, "\n")
```

### QGIS
- Tools appear in Processing toolbox under **Geostatistics → Kriging**
- Parameter UI mirrors Python/R names
- Outputs registered as layers in active project

---

## Numerical Robustness Requirements

1. **Covariance Matrix Conditioning**
   - Flag if $\kappa(\mathbf{K}) > 10^{10}$
   - Add nugget regularization automatically
   - Document in output metadata

2. **Singular System Detection**
   - If Cholesky fails after regularization, fall back to SVD-based solve
   - Log warning with condition number
   - Proceed with reduced precision warning

3. **Outlier Handling**
   - Optional: reject semivariance pairs > 3σ from median
   - Document in variogram report
   - Benchmark impact on prediction

4. **Edge Effects**
   - Kriging at prediction location near observation: variance → 0
   - Kriging far from all observations: variance → sill
   - Test both extremes

---

## Validation Strategy

**Phase 1: Parity with R geostat**
- Meuse dataset (278 observations)
- Fit spherical variogram
- Predict on 100×100 grid
- Compare predictions & variances within 2%
- Reference: `library(gstat)` output

**Phase 2: Synthetic Benchmarks**
- Generate known Gaussian field (specified covariance)
- Subsample to 50–500 points
- Fit variogram, cross-validate
- Verify CV residuals ~ N(0,1)

**Phase 3: Stability Tests**
- Colocated observations (same point, different Z)
- Near-duplicate observations (< 1 m apart)
- Extreme range/sill ratios
- Verify no panics, sensible error messages

---

## Minimum Shippable Product (MSP)

**Target:** End of Week 5

**Includes:**
1. Variogram estimation & model fitting (3 families)
2. Ordinary Kriging predictor (isotropic only)
3. Cross-validation metrics
4. Python/R/QGIS wrappers
5. Parity validation against gstat
6. User manual chapters + examples
7. API reference

**Does NOT include:**
- Anisotropic variograms
- Universal/Simple Kriging
- Co-kriging
- Block kriging
- Advanced model selection (AIC/BIC)

**Post-MSP Phase B+ (Weeks 6–8):**
- Anisotropic variogram fitting
- Trend removal (Universal Kriging)
- Indicator kriging
- Co-kriging framework
- Advanced diagnostics

---

## Integration Points

### Backend Crate (`wbgeostat`)
- New crate in `crates/wbgeostat/`
- Dependencies: `nalgebra` (linear algebra), `ndarray`, `rayon` (parallelization)
- Stable public API: `Variogram`, `VariogramModel`, `OrdinaryKriging`, `CrossValidation`
- Tests: unit tests + integration tests against parity suite

### Python Binding
- PyO3 wrappers in `crates/wbw_python/src/`
- Tool registration in `crates/wbtools_oss/src/tools/geostatistics/mod.rs`
- Output handling: raster writes via `wbraster`

### R Binding
- extendr wrappers in `crates/wbw_r/src/lib.rs`
- Session methods added to facade layer
- Roxygen documentation in `crates/wbw_r/r-package/whiteboxworkflows/R/`

### QGIS Plugin
- Tool descriptions in `crates/wbw_qgis/plugin/whitebox_workflows_qgis/descriptions/geostatistics_kriging.json`
- Processing provider registration in plugin initialization

---

## Timeline Estimate

| Week | Milestone | Owner | Status |
|------|-----------|-------|--------|
| 1 | Variogram core (estimation + fitting) | Backend | Not started |
| 2 | Kriging solver + regularization | Backend | Not started |
| 2.5 | CV + diagnostics | Backend | Not started |
| 3 | Python wrappers & integration | Python binding | Not started |
| 3.5 | R wrappers & session methods | R binding | Not started |
| 4 | QGIS plugin tools & UI | QGIS plugin | Not started |
| 4.5 | Validation & benchmarking | QA / Backend | Not started |
| 5 | Documentation + release | Docs | Not started |

**Total: ~5 weeks for MVP (production grade)**

---

## Success Criteria

1. ✓ Parity tests pass (Meuse dataset vs. gstat within 2%)
2. ✓ All numerical stability tests pass without panics
3. ✓ Performance benchmark: 1000-point cloud → 100×100 grid < 5 sec
4. ✓ Python/R/QGIS examples run end-to-end
5. ✓ User documentation complete with workflows
6. ✓ API reference accurate
7. ✓ No critical bugs in internal testing

---

## Open Questions

1. **Nugget effect handling:** Auto-regularize vs. user-specified?
   - *Proposal:* Auto-detect, but allow user override
2. **Anisotropy MVP inclusion?**
   - *Proposal:* Defer to Phase B+ (scope creep risk)
3. **Indicator kriging for categorical data?**
   - *Proposal:* Defer to Phase B+
4. **Benchmark against ArcGIS/QGIS built-ins?**
   - *Proposal:* Compare accuracy only (not speed; API design differs)

---

## References & External Resources

- Matheron, G. (1963). "Principles of geostatistics." Economic Geology 58(8): 1246–1266.
- R `gstat` package: https://github.com/r-spatial/gstat
- SGeMS (Stanford Geostatistical Modeling Software): http://sgems.sourceforge.net/
- Production kriging systems: ArcGIS Spatial Analyst, QGIS (basic support via `rasterInterp`)
