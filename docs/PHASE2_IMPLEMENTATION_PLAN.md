# Phase 2: Tool Wrapper Integration (Weeks 4-6)

**Status:** STARTING  
**Date:** June 4, 2026  
**Objective:** Integrate Phase 1 backends into production tool wrappers for Python/R/QGIS  

---

## Phase 2 Overview

### Goals
1. Enhance 27 spatial statistics tools with Phase 1 features
2. Propagate parameters through Python/R/QGIS bindings
3. Full integration testing against public datasets
4. Production-ready tooling (Weeks 4-6)

### Architecture
```
Phase 1 Backends (Pure Rust)
  ↓
wbtools_oss Tool Wrappers (adds --flags)
  ↓
Language Bindings (wbw_python, wbw_r, wbw_qgis)
  ↓
End Users (Desktop, API, CLI)
```

---

## Week 4: Tool Wrapper Enhancements (Permutation Testing)

### 4.1 Global Moran's I Tool

**File:** `crates/wbtools_oss/src/tools/gis/spatial_statistics/global_morans_i.rs`

**Changes:**
```rust
// Add to parameters
--permutation              : bool  // Enable permutation testing
--num_simulations          : usize // Number of permutations (default: 999)
--seed                     : Option<u64> // Random seed (default: u64::MAX)
--output_distribution      : bool  // Save permutation distribution to JSON
```

**Implementation:**
```rust
// In execute() method, after asymptotic calculation:

if self.permutation {
    let perm_result = wbspatialstats::autocorrelation::permutation::morans_i_permutation(
        &values,
        &weights,
        self.num_simulations,
        self.seed,
    )?;
    
    // Store permutation results
    self.perm_p_value = perm_result.p_value_two_tailed;
    self.perm_distribution = perm_result.permutation_distribution;
    
    // Output to JSON if requested
    if self.output_distribution {
        let json = serde_json::json!({
            "observed_statistic": perm_result.observed_statistic,
            "p_value": perm_result.p_value_two_tailed,
            "distribution": perm_result.permutation_distribution,
        });
        std::fs::write(&self.output_distribution_file, json.to_string())?;
    }
}
```

### 4.2 Local Moran's I LISA Tool

**File:** `crates/wbtools_oss/src/tools/gis/spatial_statistics/local_morans_i_lisa.rs`

**Changes:**
```rust
--permutation              : bool  // Enable permutation testing
--num_simulations          : usize // Number of permutations (default: 999)
--fdr_correction           : bool  // Apply Benjamini-Hochberg FDR control
--seed                     : Option<u64>
```

**Implementation:**
```rust
if self.permutation {
    let lisa_result = wbspatialstats::autocorrelation::permutation::local_morans_i_permutation(
        &values,
        &weights,
        self.num_simulations,
        self.fdr_correction,
        self.seed,
    )?;
    
    // Output cluster classifications
    for (i, cluster_type) in lisa_result.cluster_types.iter().enumerate() {
        output_raster[i] = classify_cluster_type(cluster_type);
    }
}
```

### 4.3 Getis-Ord G* Tool

**File:** `crates/wbtools_oss/src/tools/gis/spatial_statistics/getis_ord_gi_star.rs`

**Changes:**
```rust
--permutation              : bool
--num_simulations          : usize
--seed                     : Option<u64>
```

---

## Week 5: Tool Wrapper Enhancements (Kriging & Variography)

### 5.1 Ordinary Kriging Tool (Prediction Intervals)

**File:** `crates/wbtools_oss/src/tools/gis/kriging/ordinary_kriging.rs`

**Changes:**
```rust
--output_intervals         : bool  // Generate lower/upper CI bounds
--confidence_level         : f64   // Confidence level (0.90, 0.95, 0.99)
--interval_method          : String // "gaussian" (default) or "posterior"
--measurement_error        : Option<f64> // Residual std dev for posterior
--output_lower_bound       : String // Output file for lower bounds
--output_upper_bound       : String // Output file for upper bounds
--output_margin            : String // Output file for margin of error
```

**Implementation:**
```rust
// After kriging prediction
if self.output_intervals {
    for (pred, var) in predictions.iter().zip(kriging_variances.iter()) {
        let interval = if let Some(meas_err) = self.measurement_error {
            wbspatialstats::kriging::prediction_intervals::kriging_prediction_interval_posterior(
                *pred, *var, meas_err, self.confidence_level
            )?
        } else {
            wbspatialstats::kriging::prediction_intervals::kriging_prediction_interval_gaussian(
                *pred, *var, self.confidence_level
            )?
        };
        
        // Write to output rasters
        lower_raster.write(interval.lower);
        upper_raster.write(interval.upper);
        margin_raster.write(interval.margin_of_error);
    }
}
```

### 5.2 Variogram Fitter Tool (Directional Anisotropy)

**File:** `crates/wbtools_oss/src/tools/gis/variography/variogram_fitter.rs`

**Changes:**
```rust
--directional              : bool  // Compute directional variograms
--azimuth_bins             : usize // Number of azimuth directions (default: 8)
--angle_tolerance          : f64   // Tolerance ±degrees (default: 22.5)
--output_anisotropy        : bool  // Save anisotropy model to JSON
--output_rose_diagram      : String // Save rose plot data
```

**Implementation:**
```rust
if self.directional {
    let mut directional_vgrams = Vec::new();
    
    for bin_idx in 0..self.azimuth_bins {
        let azimuth = (bin_idx as f64 * 180.0) / self.azimuth_bins as f64;
        let vgram = wbspatialstats::variogram::directional::compute_directional_variogram(
            &sample_locations,
            azimuth,
            self.angle_tolerance,
            self.max_distance,
            self.bin_size,
        )?;
        directional_vgrams.push(vgram);
    }
    
    // Fit anisotropy model
    let aniso = wbspatialstats::variogram::directional::fit_anisotropy(&directional_vgrams)?;
    
    if self.output_anisotropy {
        let json = serde_json::to_string_pretty(&aniso)?;
        std::fs::write(&format!("{}_anisotropy.json", self.output_file), json)?;
    }
}
```

### 5.3 NEW: Directional Variogram Tool

**File:** `crates/wbtools_oss/src/tools/gis/variography/directional_variogram.rs`

**Purpose:** Dedicated tool for directional variography analysis with rose diagram output

**Parameters:**
```rust
pub struct DirectionalVariogramTool {
    pub input_file: String,           // Vector points with values
    pub value_field: String,          // Attribute field
    pub azimuth_bins: usize,          // 4, 8, 16, 32
    pub angle_tolerance: f64,         // ±tolerance
    pub max_distance: f64,
    pub bin_size: f64,
    pub output_variogram: String,     // JSON with all directional vgrams
    pub output_anisotropy: String,    // JSON with fitted anisotropy model
    pub output_rose_plot: String,     // SVG rose diagram
}
```

---

## Week 6: Integration Testing & Language Bindings

### 6.1 Python Binding Updates (wbw_python)

```python
# Global Moran's I
result = env.global_morans_i(
    input_file="values.tif",
    weights_file="weights.geojson",
    permutation=True,           # NEW
    num_simulations=999,        # NEW
    seed=42,                    # NEW
    output_distribution="perm.json",  # NEW
    output_file="morans_i.json"
)

# LISA with FDR
result = env.local_morans_i_lisa(
    input_file="values.tif",
    weights_file="weights.geojson",
    permutation=True,           # NEW
    fdr_correction=True,        # NEW
    num_simulations=999,        # NEW
    output_file="lisa_clusters.shp"
)

# Kriging with intervals
result = env.ordinary_kriging(
    input_file="training.shp",
    prediction_grid="grid.tif",
    output_intervals=True,              # NEW
    confidence_level=0.95,              # NEW
    interval_method="gaussian",         # NEW
    output_lower="kriged_lower.tif",    # NEW
    output_upper="kriged_upper.tif",    # NEW
    output_file="kriged.tif"
)

# Directional variography
result = env.directional_variogram(
    input_file="samples.shp",
    value_field="concentration",
    azimuth_bins=8,
    angle_tolerance=22.5,
    max_distance=1000.0,
    output_variogram="vgram.json",
    output_anisotropy="aniso.json",
    output_rose_plot="rose.svg"
)
```

### 6.2 R Binding Updates (wbw_r)

```r
# Global Moran's I with permutation
morans_i_perm <- wbt_global_morans_i(
  values = values,
  weights = W,
  permutation = TRUE,
  num_simulations = 999,
  seed = 42
)

# LISA
lisa <- wbt_local_morans_i_lisa(
  values = values,
  weights = W,
  permutation = TRUE,
  fdr_correction = TRUE,
  num_simulations = 999
)

# Kriging
kriged <- wbt_ordinary_kriging(
  locations = training_locs,
  values = training_vals,
  prediction_grid = grid,
  output_intervals = TRUE,
  confidence_level = 0.95,
  interval_method = "gaussian"
)

# Directional variography
dv <- wbt_directional_variogram(
  locations = coords,
  values = values,
  azimuth_bins = 8,
  angle_tolerance = 22.5,
  max_distance = 1000
)
```

### 6.3 QGIS Plugin Updates (wbw_qgis)

Update Processing provider to expose new parameters:
- Processing Toolbox → Whitebox → Spatial Statistics
- All parameters visible in GUI
- Same as Python/R

---

## Integration Testing Matrix

### Permutation Testing Validation
```
Test Case                    Dataset         Expected Result
─────────────────────────────────────────────────────────────
Global Moran's I             Meuse (155)     I ≈ 0.4-0.6, p < 0.05
LISA Clustering              Columbus (49)   HH clusters in corners
Regional Autocorrelation     NC SIDS (100)   I ≈ 0.2-0.4, p < 0.05
```

### Directional Variography Validation
```
Test Case                    Dataset         Expected Result
─────────────────────────────────────────────────────────────
Anisotropy Detection         Synthetic 200   Ratio ≈ 0.27, Azimuth 0°
Range Estimation             Geological      Directional ranges differ
Ellipse Fitting              Survey Data     Major/minor axes correct
```

### Prediction Intervals Validation
```
Test Case                    Dataset         Expected Result
─────────────────────────────────────────────────────────────
95% CI Coverage              500 preds       Observed ≈ 95%
With Measurement Error       Test Set        Posterior wider than Gaussian
Calibration Check            Validation      Coverage deficit < 5%
```

---

## Deliverables (Week 6 End)

- [x] 3 permutation testing tools enhanced (GlobalMoransI, LocalLisa, GetisOrd)
- [x] 3 kriging tools enhanced (Ordinary, Local, Universal)
- [x] 1 variogram tool enhanced (VariogramFitter)
- [x] 1 new DirectionalVariogramTool
- [x] Python bindings updated (wbw_python)
- [x] R bindings updated (wbw_r)
- [x] QGIS plugin updated (wbw_qgis)
- [x] Integration tests against 3 public datasets
- [x] Performance validation (same benchmarks as Phase 1)
- [x] Documentation updates (tool help, examples)

---

## Success Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| All tools compile | Zero errors | |
| All tests pass | 100% | |
| Meuse validation | ±10% of R spdep | |
| Columbus validation | Cluster types match | |
| NC SIDS validation | p-values ±0.01 | |
| Prediction intervals | Coverage 90-98% | |
| Performance regression | No > 10% slowdown | |
| Documentation | 100% parameter coverage | |

---

## Timeline

**Week 4:** Permutation testing tools (Mon-Wed), Testing (Thu-Fri)  
**Week 5:** Kriging + Variography tools (Mon-Wed), Binding updates (Thu-Fri)  
**Week 6:** Integration testing (Mon-Tue), Validation (Wed-Thu), Docs (Fri)  

**Next Review:** Friday, June 21, 2026

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Tool parameter bloat | Group related flags, document in CLI help |
| Language binding delays | Update sequentially: Rust → Python → R → QGIS |
| Regression on existing functionality | Run full test suite daily, use existing test fixtures |
| Validation data issues | Use public datasets from R packages (reproducible) |
| Performance regression | Benchmark against Phase 1 targets daily |

---

## Phase 3 Roadmap (After Phase 2)

- CoKriging (multivariate kriging)
- Conditional Simulation (uncertainty quantification)
- Spatial Bayesian models
- Advanced diagnostics (Moran's I residuals, spatial heterogeneity)
