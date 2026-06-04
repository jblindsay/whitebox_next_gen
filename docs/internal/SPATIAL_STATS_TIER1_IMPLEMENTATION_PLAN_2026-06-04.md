# Tier-1 Spatial Statistics Implementation Plan
**Date:** 2026-06-04  
**Status:** Planning Phase  
**Priority Features:** Permutation Testing, CoKriging, Directional Variography, Prediction Intervals

---

## Executive Summary

Whitebox has excellent foundational spatial statistics (kriging, SAR/SEM/GWR, Ripley's K, IDW). The 4 Tier-1 features below will unlock production-grade capabilities for uncertainty quantification, multivariate workflows, and robust inference.

| Feature | Current State | Impact | Est. Effort |
|---------|---------------|--------|-------------|
| **Permutation-Based Inference** | Asymptotic only | Enables small-sample inference, robustness | 2-3 weeks |
| **CoKriging** | Not implemented | Enables multivariate prediction | 3-4 weeks |
| **Directional Variography** | Omnidirectional only | Enables anisotropic modeling | 2-3 weeks |
| **Prediction Intervals** | Kriging variance only | Enables decision support, uncertainty quantification | 1-2 weeks |

---

## Feature 1: Permutation-Based Inference

### Current State
- All spatial autocorrelation tests (Moran's I, Getis-Ord, LISA) use asymptotic distributions
- Code comment in [crates/wbspatialstats/src/autocorrelation/mod.rs](crates/wbspatialstats/src/autocorrelation/mod.rs): `// TODO: permutation testing (future)`
- Risk: False positives/negatives with non-normal data or small samples

### What to Build

#### Backend Module: `crates/wbspatialstats/src/autocorrelation/permutation.rs` (NEW)
```
pub fn morans_i_permutation(
    values: &[f64],
    weights: &SpatialWeightsMatrix,
    n_simulations: usize,
    seed: Option<u64>,
) -> PermutationTestResult {
    // 1. Calculate observed Moran's I
    // 2. Generate n_simulations random permutations of values
    // 3. Calculate Moran's I for each permutation
    // 4. Compute empirical p-value: (count >= observed + 1) / (n_simulations + 1)
    // 5. Return: observed_statistic, p_value, distribution (for plotting)
}

pub fn getis_ord_gi_permutation(
    values: &[f64],
    weights: &SpatialWeightsMatrix,
    n_simulations: usize,
    seed: Option<u64>,
) -> PermutationTestResult { ... }

pub fn lisa_permutation(
    values: &[f64],
    weights: &SpatialWeightsMatrix,
    n_simulations: usize,
    fdr_correction: bool,
) -> LisaPermutationResult {
    // Local Moran's I permutation testing
    // Returns per-location p-values and corrected significance
}
```

#### Implementation Details
- **Algorithm:** Sampling without replacement
- **Parallelization:** rayon for permutation loop (each permutation independent)
- **Performance:** Cache weight matrix sparse structure, vectorize Moran's I calculation
- **Randomness:** Controlled via seed parameter for reproducibility
- **Dependencies:** Already have `rand` crate in wbspatialstats
- **Data Structure:** Return permutation distribution for diagnostic plotting

#### Tool Updates (wbtools_oss)
- Enhance `GlobalMoransITool`: Add `--permutation` and `--num_simulations` parameters
- Enhance `GetisOrdGiStarTool`: Add `--permutation` flag
- Enhance `LocalMoransILisaTool`: Add `--permutation` and `--fdr` flags
- Output: Include p-value column, permutation distribution CSV for diagnostics

#### Frontend Integration (wbw_python, wbw_r, wbw_qgis)
- New parameters propagate automatically through existing tool interface
- Python example:
  ```python
  result = env.vector.global_morans_i(
      input_file='points.shp',
      field='value',
      spatial_weights='queen',
      permutation=True,
      num_simulations=9999,
      output_file='morans_permutation.shp'
  )
  print(f"p-value (permutation): {result['p_value_permutation']}")
  ```

### Implementation Sequence
1. **Week 1:** Implement `permutation.rs` backend module with tests
2. **Week 2:** Integrate into tool methods, add diagnostic outputs
3. **Week 2.5:** Test with real datasets, performance validation

### Testing Strategy
- Unit tests: Known distributions (e.g., random normal field, regular grid)
- Validation: Compare p-values against R's `spdep::moran.mc()` on same data
- Performance: Benchmark 1000 simulations on 10k points with king weights

---

## Feature 2: CoKriging (Multivariate Kriging)

### Current State
- Only univariate kriging variants (Ordinary, Simple, Universal, Local, SpaceTime)
- No cross-variograms or multivariate covariance structures
- Cannot leverage correlated auxiliary variables

### What to Build

#### Backend: Cross-Variogram Module (`crates/wbspatialstats/src/variogram/cross_variogram.rs`) [NEW]
```
pub struct CrossVariogram {
    // For primary Z and auxiliary Y
    pub lags: Vec<f64>,
    pub semivariances_zy: Vec<f64>,     // Cross-semivariance
    pub counts: Vec<usize>,
    pub bin_size: f64,
}

impl CrossVariogram {
    pub fn compute(
        primary: &[(f64, f64, f64)],    // [(x, y, z_value)]
        auxiliary: &[(f64, f64, f64)],   // [(x, y, y_value)]
        max_distance: f64,
        bin_size: f64,
    ) -> Self { ... }
    
    pub fn fit_model(
        &self,
        model_type: &str,  // "spherical", "exponential", etc.
    ) -> CrossVariogramModel { ... }
}

pub struct CrossVariogramModel {
    pub nugget: f64,
    pub sill: f64,
    pub range: f64,
    pub model_type: String,
}
```

#### Backend: CoKriging Solver (`crates/wbspatialstats/src/kriging/cokriging.rs`) [NEW]
```
pub struct OrdinaryCoKriging {
    primary_variogram: VariogramModel,
    cross_variograms: [CrossVariogramModel; N_AUX],  // For each auxiliary var
    auxiliary_variograms: [VariogramModel; N_AUX],
    weights: SpatialWeightsGraph,
}

impl OrdinaryCoKriging {
    pub fn predict(
        &self,
        primary_sample_locations: &[(f64, f64, f64)],  // (x, y, z)
        auxiliary_sample_locations: &[Vec<(f64, f64, f64)>],  // Multiple auxiliaries
        target_location: (f64, f64),
        neighborhood_size: usize,
    ) -> KrigingPrediction {
        // 1. Select nearest neighbors (primary + all auxiliaries)
        // 2. Build cokriging system matrix using cross-variograms
        // 3. Solve with Cholesky decomposition
        // 4. Return prediction + variance
    }
    
    pub fn predict_grid(
        &self,
        grid: &RasterGrid,
        neighborhood_size: usize,
    ) -> RasterData { ... }
}
```

#### Tool: CoKriging Tool (`crates/wbtools_oss/src/tools/gis/spatial_stats_cokriging.rs`) [NEW]
```
pub struct OrdinaryCoKrigingTool;

impl Tool for OrdinaryCoKrigingTool {
    fn run(&self, args: &ToolArguments, ctx: &ToolContext) -> ToolRunResult {
        // Input:
        //   - primary_input: Points with primary variable (Z)
        //   - auxiliary_inputs: [Points with auxiliary vars (Y1, Y2, ...)]
        //   - primary_field: Name of Z column
        //   - auxiliary_fields: [Names of Y1, Y2, ... columns]
        //   - output_file: Path for kriged grid
        //
        // Process:
        //   1. Compute empirical variograms (primary, auxiliary)
        //   2. Compute cross-variograms (primary-auxiliary)
        //   3. Fit all variogram models
        //   4. Run CoKriging on grid
        //   5. Output: prediction + kriging_variance
    }
}
```

#### Frontend Integration
- New tools in taxonomy:
  - `vector.spatial_statistics.ordinary_cokriging`
  - `vector.spatial_statistics.local_cokriging`
- Python example:
  ```python
  result = env.vector.ordinary_cokriging(
      primary_input='temperature_points.shp',
      primary_field='temp_c',
      auxiliary_inputs=['elevation.shp'],
      auxiliary_fields=['dem_m'],
      cell_size=100,
      output_file='temp_cokried.tif'
  )
  ```

### Implementation Sequence
1. **Week 1:** Implement cross-variogram computation + model fitting
2. **Week 2:** Build cokriging solver (matrix setup, Cholesky solve)
3. **Week 3:** Tool wrapper, I/O handling, error checking
4. **Week 4:** Testing, validation against R's `gstat::krige(...formula=Z~Y1+Y2)`

### Testing Strategy
- Synthetic: Known correlation structure (e.g., Z = 2*Y + noise)
- Real: Temperature predicted from elevation (common use case)
- Validation: Compare kriged values + variances vs. R's gstat CoKriging

### Architecture Decision
- Store cross-variograms in `SpatialWeightsGraph` extension or separate struct?
- **Decision:** Separate `CrossVariogramMatrix` type to keep weight matrices clean

---

## Feature 3: Directional/Anisotropic Variography

### Current State
- Omnidirectional variograms only (all direction bins combined)
- Cannot model aligned features (faults, river channels, geological strata)
- Misses spatial anisotropy common in geology/geomorphology

### What to Build

#### Backend: Directional Variogram Module (`crates/wbspatialstats/src/variogram/directional.rs`) [NEW]
```
#[derive(Clone, Debug)]
pub struct DirectionalVariogramBin {
    pub direction_azimuth: f64,      // 0-180° (bidirectional)
    pub tolerance: f64,               // ±tolerance degrees
    pub lags: Vec<f64>,
    pub semivariances: Vec<f64>,
    pub counts: Vec<usize>,
    pub bin_size: f64,
}

pub struct AnisotropyModel {
    pub major_range: f64,             // Range along max continuity
    pub minor_range: f64,             // Range perpendicular
    pub major_azimuth: f64,           // Direction of max range (0-180°)
    pub ratio: f64,                   // minor/major
    pub angle_tolerance: f64,         // ±degrees for tolerance ellipse
}

pub fn compute_directional_variogram(
    sample_locations: &[(f64, f64, f64)],  // (x, y, value)
    direction_azimuth: f64,     // 0=East, 90=North
    tolerance: f64,              // e.g., ±22.5°
    max_distance: f64,
    bin_size: f64,
) -> DirectionalVariogramBin { ... }

pub fn fit_anisotropy(
    directional_vgrams: &[DirectionalVariogramBin],
    num_directions: usize,  // 4, 8, etc.
) -> AnisotropyModel { ... }
```

#### Enhanced Variogram Visualization Tool (wbtools_oss)
```
// New tool: DirectionalVariogramTool
// Output: 
//   - Rose diagram (azimuth vs. range)
//   - Variogram sill map (2D contour showing sill by direction)
//   - Anisotropy ellipse overlay
```

#### Tool Updates: Anisotropic Kriging

**OrdinaryKrigingTool Enhancement:**
- New parameters: `--directional` (boolean), `--major_azimuth`, `--anisotropy_ratio`
- If `--directional=true`: Use anisotropic distance metric in kriging system
- Implementation: Transform coordinates to anisotropic space before distance calc:
  ```
  distance_anisotropic = sqrt(
      (dx * cos(azimuth))^2 / major_range^2 +
      (dy * sin(azimuth))^2 / minor_range^2
  )
  ```

#### Frontend Integration
- New tool: `vector.spatial_statistics.directional_variogram`
- Enhanced kriging tools with anisotropy options
- Python example:
  ```python
  # Step 1: Compute directional variograms
  env.vector.directional_variogram(
      input_file='geological_samples.shp',
      field='ore_grade',
      directions=[0, 45, 90, 135],
      tolerance=22.5,
      output_prefix='vgram_dir'
  )
  
  # Step 2: Use in anisotropic kriging
  env.vector.ordinary_kriging(
      input_file='geological_samples.shp',
      field='ore_grade',
      cell_size=100,
      anisotropy=True,
      major_azimuth=45,  # Direction of max continuity
      major_range=500,
      minor_range=250,
      output_file='ore_cokried_anisotropic.tif'
  )
  ```

### Implementation Sequence
1. **Week 1:** Implement directional variogram computation + anisotropy fitting
2. **Week 2:** Enhance kriging solvers to use anisotropic distance
3. **Week 2.5:** Visualization tool (rose diagram, ellipse overlay)
4. **Week 3:** Testing, validation vs. GSLib `vario` + `kt3d`

### Testing Strategy
- Synthetic: Create aligned feature (e.g., linear drift) + verify directional variogram captures it
- Real: Geological strike/dip data (common in mining applications)
- Validation: Compare directional variograms vs. GSLib's `vario` output

### Architecture Decision
- Store anisotropy parameters in kriging predictor or separate struct?
- **Decision:** Add `anisotropy: AnisotropyModel` directly to kriging structs (no Optional wrapping). Clean, direct design; no backwards compat overhead.

---

## Feature 4: Prediction Intervals & Uncertainty Quantification

### Current State
- Kriging produces predictions + variances (kriging_variance)
- No confidence intervals, prediction intervals, or distribution inference
- Cannot answer: "What's the 90% confidence bound on my prediction?"

### Bootstrap Complexity Strategy
**Phase 1 (Weeks 1-3): Gaussian-only prediction intervals** (simplest, highest ROI)
- Assume kriging prediction ~ Normal(prediction, kriging_variance)
- Use normal quantiles for confidence/prediction intervals
- Fast, deterministic, no additional sampling required
- Production-ready for most use cases

**Phase 2+ (Weeks 4+): Add bootstrap/empirical methods** (when needed)
- Residual-based bootstrap intervals
- Quantile regression alternatives
- Posterior prediction intervals with measurement uncertainty

### What to Build

#### Backend: Prediction Intervals Module (`crates/wbspatialstats/src/kriging/prediction_intervals.rs`) [NEW]
```
pub struct PredictionInterval {
    pub lower: f64,          // e.g., 5th percentile
    pub point_estimate: f64, // mean/median
    pub upper: f64,          // e.g., 95th percentile
    pub confidence: f64,     // 0.90 for 90% CI
    pub method: String,      // "gaussian", "bootstrap", "quantile"
}

pub fn kriging_prediction_interval_gaussian(
    prediction: f64,
    kriging_variance: f64,
    confidence: f64,  // e.g., 0.95
) -> PredictionInterval {
    // Assume prediction ~ N(prediction, kriging_variance)
    // Use normal quantiles
    let z_critical = normal_quantile((1.0 + confidence) / 2.0);
    let margin_of_error = z_critical * kriging_variance.sqrt();
    
    PredictionInterval {
        lower: prediction - margin_of_error,
        point_estimate: prediction,
        upper: prediction + margin_of_error,
        confidence,
        method: "gaussian".to_string(),
    }
}

pub fn kriging_prediction_interval_bootstrap(
    neighbors: &[(f64, f64, f64)],  // Nearest neighbors (x, y, z)
    target: (f64, f64),
    kriging_model: &KrigingModel,
    confidence: f64,
    n_bootstrap: usize,
) -> PredictionInterval {
    // Bootstrap residuals to construct empirical CI
    // 1. Compute LOO residuals for neighbors
    // 2. Resample residuals n_bootstrap times
    // 3. Add to kriged prediction
    // 4. Compute empirical quantiles at (1-confidence)/2, (1+confidence)/2
}

pub fn posterior_prediction_interval(
    prediction: f64,
    kriging_variance: f64,
    residual_std: f64,  // From cross-validation
    confidence: f64,
) -> PredictionInterval {
    // Posterior distribution includes measurement uncertainty
    // var_total = kriging_variance + residual_std^2
    let z_critical = normal_quantile((1.0 + confidence) / 2.0);
    let margin = z_critical * var_total.sqrt();
    // ...
}
```

#### Tool: Enhanced Kriging with Prediction Intervals (`wbtools_oss`)

**OrdinaryKrigingTool Enhancement:**
```
// New parameters:
//   --output_intervals: boolean (default false)
//   --confidence_level: f64 (default 0.95 for 95% CI)
//   --interval_method: "gaussian" | "bootstrap" | "posterior"

// If output_intervals=true, produce 3 rasters:
//   1. kriged_predictions.tif
//   2. kriged_lower_bound.tif
//   3. kriged_upper_bound.tif
```

#### Frontend Integration
- Parameters propagate to Python/R/QGIS automatically
- Python example:
  ```python
  result = env.vector.ordinary_kriging(
      input_file='water_quality_samples.shp',
      field='ph_value',
      cell_size=50,
      output_file='ph_kriged.tif',
      output_intervals=True,
      confidence_level=0.90,
      interval_method='gaussian'
  )
  
  # Result includes:
  # - ph_kriged.tif (point predictions)
  # - ph_kriged_lower_bound.tif (5th percentile)
  # - ph_kriged_upper_bound.tif (95th percentile)
  ```

#### Diagnostic Outputs
- **Calibration Plot:** Cross-validation predictions vs. confidence intervals
  - X-axis: Observed value
  - Y-axis: Predicted + confidence band
  - Checks: How often does observed fall in predicted interval? (Should be ~90% for 90% CI)

- **Prediction Interval Width Map:** 
  - Visual of uncertainty across domain
  - Wider where data sparse, narrower where dense
  - Help identify high-uncertainty regions

### Implementation Sequence
1. **Week 1:** Implement Gaussian prediction intervals (simplest, highest ROI)
2. **Week 2:** Tool integration, raster output handling, diagnostic calibration check

*(Bootstrap/posterior methods deferred to Phase 2+ after Gaussian validation complete)*

### Testing Strategy
- Synthetic: Create prediction intervals, verify nominal coverage (e.g., 90% interval should contain observed value ~90% of time)
- Cross-validation: Compare interval width vs. LOO residuals
- Real data: Water quality, elevation, mineral grades

### Architecture Decision
- Return intervals as multi-band raster or 3 separate files?
- **Decision:** 3 separate files (cleaner for GDAL, simpler downstream processing)

---

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-3)
1. **Week 1:** Permutation testing backend + diagnostics
2. **Week 2:** Directional variography module
3. **Week 3:** Prediction intervals (Gaussian only)

**Milestone:** All backend modules compile, unit tests pass (70%+ coverage), validated against R benchmarks

### Phase 2: Integration & Validation (Weeks 4-6)
1. **Week 4:** Tool wrappers for Features 1, 3, 4
2. **Week 5:** Frontend bindings (Python/R/QGIS), documentation
3. **Week 6:** Cross-platform testing, performance benchmarking

**Milestone:** All 3 features accessible from Python/R CLI, validated against gstat/spdep

### Phase 3: CoKriging (Weeks 7-10) 
*(Scheduled AFTER Phase 1-2 complete)*
1. **Weeks 7-8:** Cross-variogram + cokriging solver backend
2. **Weeks 9-10:** Tool integration + cross-platform testing

**Milestone:** CoKriging functional for 1-3 auxiliary variables, performance validated

### Phase 4: Extended Methods (Week 11+)
- Bootstrap prediction intervals (enhance Feature 4)
- Advanced diagnostics, visualization
- Performance optimization where needed

---

## Performance as First-Order Priority

Implementation must maintain performance throughout all phases. Key principles:

### Backend Optimization Targets
- **Permutation Testing:** Parallelize permutation loop with rayon (each permutation independent)
  - Target: 1000 simulations on 10k points with queen weights < 5 minutes (preferred < 2 min)
  - Use sparse matrix operations where possible
  - Cache weight matrix structure; reuse across simulations

- **Directional Variography:** Compute directional bins efficiently
  - Vectorize angular binning; avoid per-pair overhead
  - Cache distance matrix for reuse across directions
  - Target: 8 directional bins on 5k points < 10 seconds

- **Prediction Intervals (Gaussian):** Negligible overhead vs. kriging
  - Single normal quantile lookup; O(1) per prediction
  - No additional iterations or loops

### Tool Integration Performance
- Kriging on 1000×1000 grid with neighbor search: < 30 seconds (maintain current baseline)
- Permutation inference as optional parameter: lazy execution (only if requested)
- Directional analysis: compute only requested azimuths (not all 16)

### Benchmarking & Profiling
- Benchmark each backend module before integration testing
- Profile tool wrappers on representative datasets (Meuse 155 points → Jura 359 points → NYC trees 650k)
- If Phase 1-2 performance degrades > 10%, investigate bottleneck before Phase 3 start
- Document performance characteristics in tool documentation for users

### No Backwards Compatibility Constraints
- Simplify architecture: no compatibility layer needed
- Design kriging structs directly for anisotropy support (not Optional wrapper)
- Cleaner code, faster compilation, better compiler optimization opportunities

---

## Dependency & Architecture Changes

### New Backend Modules (wbspatialstats)
| Module | Lines | Dependencies | Priority |
|--------|-------|-------------|----------|
| `autocorrelation/permutation.rs` | ~300 | `rand`, `rayon` | P1 |
| `variogram/cross_variogram.rs` | ~400 | (existing) | P2 |
| `variogram/directional.rs` | ~500 | (existing) | P1 |
| `kriging/cokriging.rs` | ~600 | `ndarray` | P2 |
| `kriging/prediction_intervals.rs` | ~250 | `statrs` (NEW) | P1 |

### New Crates to Consider
- **statrs** (for statistical distributions): Normal CDF, quantile functions
  - Lightweight, pure Rust, well-maintained
  - Already in Python dependencies, so acceptable addition

### Tool Registry Updates (tool_taxonomy.toml)
Add to `vector.spatial_statistics`:
```toml
# Permutation-Based Testing
permutation_morans_i = "Permutation test for spatial autocorrelation"
permutation_getis_ord = "Permutation test for hot/cold spots"

# Directional Variography
directional_variogram = "Compute directional semivariograms for anisotropy"

# Prediction Intervals  
kriging_with_intervals = "Kriging with confidence/prediction intervals"

# CoKriging (Phase 2)
ordinary_cokriging = "Multivariate kriging with auxiliary variables"
local_cokriging = "Local cokriging for localized multivariate prediction"
```

---

## Effort & Resource Estimate

| Feature | Backend | Tools | Testing | Total |
|---------|---------|-------|---------|-------|
| Permutation Testing | 1 week | 0.5 week | 0.5 week | **2 weeks** |
| Directional Variography | 1 week | 1 week | 0.5 week | **2.5 weeks** |
| Prediction Intervals (Gaussian) | 0.5 week | 0.5 week | 0.5 week | **1.5 weeks** |
| **Phase 1+2 Total** | **2.5 weeks** | **2 weeks** | **1.5 weeks** | **~6 weeks** |
| CoKriging (Phase 3) | 2 weeks | 1 week | 1 week | **4 weeks** |
| **All Features** | **4.5 weeks** | **3 weeks** | **2.5 weeks** | **~10 weeks** |

*(Includes code review, documentation, minor polish)*

---

## Success Criteria

### Tier 1 (Go/No-Go)
- [ ] All backend modules compile without warnings
- [ ] Unit test coverage ≥ 80%
- [ ] Tools accessible from Python/R/QGIS
- [ ] Documented examples for each feature

### Tier 2 (Production-Grade)
- [ ] Cross-validation with R benchmarks (gstat, spdep) ≥ 95% match
- [ ] Performance: Permutation testing 1000 sims on 10k points < 5 minutes
- [ ] Performance: Kriging on 1000×1000 grid < 30 seconds (maintain current baseline)
- [ ] Diagnostic plots (rose diagrams, calibration plots) functional
- [ ] Phase 1-2 integration complete on Python/R/QGIS platforms

### Tier 3 (Polish)
- [ ] User documentation with 5+ real-world examples
- [ ] Blog posts: "Multivariate Spatial Analysis with CoKriging", etc.
- [ ] Changelog entry with migration notes if applicable

---

## Validation Datasets (Publicly Available & Ready to Use)

### Tier 1: Benchmark Datasets (Start Here)
| Dataset | Purpose | Samples | Variables | Source |
|---------|---------|---------|-----------|--------|
| **Meuse** | Validate Ordinary/Universal Kriging | 155 | Zn, Cu, Pb, distance to river | R gstat; Pebesma & Wesseling 1998 |
| **Columbus Crime** | Validate Moran's I, LISA, SAR models | 49 areal | Crime rate, income, house value | R spdep; Anselin 1988 |
| **NC SIDS Deaths** | Validate spatial regression | 100 areal | Death counts, covariates, neighbors | R sf/spdep |
| **Redwood Seedlings** | Validate Ripley's K, envelope testing | 62 | Tree X/Y coordinates | R spatstat; Ripley 1977 |
| **Irish Precipitation** | Validate permutation testing | ~100 stations | Rainfall, elevation, coords | EPA; published benchmarks |

### Tier 2: Multi-Variable for Future Work
- **Jura Dataset:** 359 sites, 5 heavy metals (Zn, Cd, Pb, Cu, Cr) + covariates → Cross-variograms for Phase 3
- **USGS Colorado Streamflow:** 50+ gauges, discharge + elevation + drainage area → Multivariate validation
- **NYC Street Trees:** 650k+ trees, species + health → Large-scale Getis-Ord Gi* testing

### Quick Access
```r
# R: Direct import
data(meuse)        # gstat package
data(columbus)     # spdep package
data(nc)           # sf/spdep package
data(redwood)      # spatstat package
# Python: Use R integration or direct CSV download
```

### Validation Strategy
1. **Week 1-2:** Permutation testing → Compare p-values against R `spdep::moran.mc()`
2. **Week 2-3:** Directional variography on Jura → Compare rose diagram vs. GSLib/gstat
3. **Week 3-4:** Prediction intervals on Meuse → Verify 90% CI coverage on LOO cross-validation
4. **Phase 2:** Full cross-platform validation (Python/R/QGIS) with all 5 datasets

---

## Final Architecture Summary

### Decisions Made
- ✅ **No backwards compatibility:** Direct design for new features (cleaner code, better optimization)
- ✅ **Gaussian prediction intervals only in Phase 1:** Bootstrap deferred to Phase 2 (faster delivery of ROI)
- ✅ **CoKriging after Phase 2:** Allows Phase 1-2 validation before starting Phase 3 complexity
- ✅ **Performance first:** Rayon parallelization, sparse matrix caching, efficient binning in all implementations
- ✅ **Public validation datasets:** 5 Tier-1 + 3 Tier-2 ready for cross-checking against R/GSLib

### Phase 1 Expected Outcome
- 3 production-ready features (Permutation Testing, Directional Variography, Gaussian Prediction Intervals)
- Validated against R benchmarks (gstat, spdep, spatstat, GSLib)
- Performance baseline established; all tools meet performance targets
- Ready for immediate integration into wbtools_oss tool suite
- Ready for Phase 2 (tool integration + Python/R/QGIS bindings)

---

**Ready to begin Phase 1: Permutation Testing backend module.**
