# Phase 1 API Documentation for Frontend Teams

## Overview

This document provides API reference for Phase 1 spatial statistics modules. Frontend integration points are through the tool wrapper registry (`wbtools_oss`). All modules are in the `wbspatialstats` Rust crate with public Rust API and language bindings (Python, R, QGIS).

---

## Module 1: Permutation Testing Backend

### Purpose
Distribution-free inference for spatial autocorrelation tests via empirical null hypothesis distributions. Addresses small-sample scenarios where asymptotic assumptions may not hold.

### Rust API

```rust
use wbspatialstats::autocorrelation::permutation::*;
use wbspatialstats::{SpatialWeightsGraph, SpatialWeightsDiagnostics};

// Global permutation test
pub fn morans_i_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    seed: Option<u64>,
) -> Result<PermutationTestResult, String>

// Returns
pub struct PermutationTestResult {
    pub observed_statistic: f64,      // Observed Moran's I
    pub expected_value: f64,          // E[I] under null
    pub variance: f64,                // Var[I] permutation-based
    pub z_score: f64,                 // (I - E[I]) / sqrt(Var[I])
    pub p_value_one_tailed: f64,      // P(I >= observed) / (n_sims + 1)
    pub p_value_two_tailed: f64,      // P(|I| >= |observed|) / (n_sims + 1)
    pub permutation_distribution: Vec<f64>, // All n_simulations values
    pub n_simulations: usize,
}
```

**Local Indicators (LISA):**
```rust
pub fn local_morans_i_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    fdr_correction: bool,
    seed: Option<u64>,
) -> Result<LocalPermutationTestResult, String>

pub struct LocalPermutationTestResult {
    pub local_statistics: Vec<f64>,     // Per-location I_i
    pub p_values: Vec<f64>,             // Per-location p-values (after FDR if enabled)
    pub cluster_types: Vec<String>,     // HH, LL, HL, LH, insignificant
    pub fdr_adjusted: bool,             // Whether Benjamini-Hochberg applied
    pub fdr_threshold: f64,             // Rejection threshold if FDR enabled
}
```

**Getis-Ord G*:**
```rust
pub fn getis_ord_gi_star_permutation(
    values: &[f64],
    weights: &SpatialWeightsGraph,
    n_simulations: usize,
    seed: Option<u64>,
) -> Result<PermutationTestResult, String>
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `values` | `&[f64]` | Yes | Spatial variable (e.g., pollution, crime, yield) |
| `weights` | `&SpatialWeightsGraph` | Yes | Spatial weights (Queen, Rook, K-nearest, etc.) |
| `n_simulations` | `usize` | Yes | Number of random permutations (999-9999 typical) |
| `seed` | `Option<u64>` | No | Random seed for reproducibility (default: u64::MAX) |
| `fdr_correction` | `bool` | No | Apply Benjamini-Hochberg FDR control (LISA only) |

### Interpretation

**Global Moran's I:**
- `p_value_two_tailed < 0.05`: Significant spatial autocorrelation
- `observed_statistic > expected_value`: Positive autocorrelation (clustering)
- `observed_statistic < expected_value`: Negative autocorrelation (dispersion)

**LISA (Local Indicators):**
- `cluster_types = "HH"`: High-value clusters (e.g., hot spots)
- `cluster_types = "LL"`: Low-value clusters (e.g., cold spots)
- `cluster_types = "HL"` / `"LH"`: Outliers
- `cluster_types = "insignificant"`: No local spatial structure

**FDR Correction:**
When enabled, adjusts p-values to control false discovery rate at α=0.05. Reduces false positives in multiple testing.

### Example Usage (Rust)

```rust
// Create spatial weights (e.g., from shapefile or grid)
let weights = /* SpatialWeightsGraph from file/grid */;
let values = vec![1.5, 2.3, 1.8, 3.2, ...]; // n values

// Run permutation test (1000 permutations)
let result = morans_i_permutation(&values, &weights, 1000, Some(42))?;

if result.p_value_two_tailed < 0.05 {
    println!("Significant clustering detected");
    println!("I = {:.4}, p = {:.4}", 
        result.observed_statistic, 
        result.p_value_two_tailed);
}

// LISA with FDR correction
let lisa_result = local_morans_i_permutation(&values, &weights, 999, true, Some(42))?;

for (i, cluster_type) in lisa_result.cluster_types.iter().enumerate() {
    if cluster_type != "insignificant" {
        println!("Location {}: {} (p={})", i, cluster_type, lisa_result.p_values[i]);
    }
}
```

### Performance

- **Complexity**: O(k·n²) where k = n_simulations, n = sample size
- **Typical benchmarks** (release mode):
  - 155 pts, 1000 sims: 0.00s
  - 500 pts, 1000 sims: 0.01-0.05s
  - 1000 pts, 1000 sims: 0.10-0.20s
- **Rayon parallelized** across permutations

---

## Module 2: Directional Variography Backend

### Purpose
Models spatially-directional dependence (anisotropy) for geological surveys, terrain analysis, and linear feature mapping.

### Rust API

```rust
use wbspatialstats::variogram::directional::*;

pub fn compute_directional_variogram(
    sample_locations: &[(f64, f64, f64)],  // (x, y, value)
    direction_azimuth: f64,                // 0-180° (0=E, 90=N)
    tolerance: f64,                        // ±degrees (e.g., 22.5)
    max_distance: f64,                     // Maximum lag distance
    bin_size: f64,                         // Lag spacing
) -> Result<DirectionalVariogramBin, String>

pub struct DirectionalVariogramBin {
    pub direction_azimuth: f64,
    pub tolerance: f64,
    pub lags: Vec<f64>,                    // Lag distances
    pub semivariances: Vec<f64>,           // γ(h) values
    pub counts: Vec<usize>,                // Pair counts per lag
    pub bin_size: f64,
    pub sill: f64,                         // Estimated sill
    pub nugget: f64,                       // Estimated nugget
}
```

**Anisotropy Fitting:**
```rust
pub fn fit_anisotropy(
    directional_vgrams: &[DirectionalVariogramBin]
) -> Result<AnisotropyModel, String>

pub struct AnisotropyModel {
    pub major_range: f64,                  // Range in major axis (long correlation)
    pub minor_range: f64,                  // Range in minor axis (short correlation)
    pub major_azimuth: f64,                // Azimuth of major axis (0-180°)
    pub ratio: f64,                        // minor / major (0-1)
    pub angle_tolerance: f64,
    pub method: String,
}

impl AnisotropyModel {
    pub fn anisotropic_distance(&self, dx: f64, dy: f64) -> f64 {
        // Transform (dx, dy) to anisotropic metric
        // Used in kriging equations
    }
}
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `sample_locations` | `&[(f64, f64, f64)]` | Points with (x, y, value) |
| `direction_azimuth` | `f64` | Direction: 0°=East, 90°=North (0-180°) |
| `tolerance` | `f64` | Angle band ±tolerance (e.g., 22.5 → 0-45°) |
| `max_distance` | `f64` | Maximum lag (omit pairs beyond) |
| `bin_size` | `f64` | Lag spacing (e.g., 10m) |

### Example Usage (Rust)

```rust
// Sample locations from survey
let samples = vec![
    (0.0, 0.0, 100.5),
    (10.0, 5.0, 102.3),
    // ... more samples
];

// Compute directional variograms (8 cardinal + intercardinal directions)
let mut directional_vgrams = Vec::new();

for dir in [0.0, 22.5, 45.0, 67.5, 90.0, 112.5, 135.0, 157.5] {
    let vgram = compute_directional_variogram(
        &samples,
        dir,
        22.5,      // ±22.5° tolerance
        100.0,     // max distance
        10.0,      // 10-unit lags
    )?;
    
    println!("Direction {:.1}°: range = {:.1}, nugget = {:.2}",
        dir, vgram.sill, vgram.nugget);
    
    directional_vgrams.push(vgram);
}

// Fit anisotropy model
let aniso = fit_anisotropy(&directional_vgrams)?;

println!("Anisotropy Detected:");
println!("  Major axis: {:.1}° (range = {:.1})", aniso.major_azimuth, aniso.major_range);
println!("  Minor axis: {:.1}° (range = {:.1})", 
    (aniso.major_azimuth + 90.0) % 180.0, aniso.minor_range);
println!("  Ratio: {:.3}", aniso.ratio);

// Use in kriging with anisotropy correction
let dist_aniso = aniso.anisotropic_distance(dx, dy);
```

### Performance

- **Complexity**: O(n² · d) where n = sample count, d = directions
- **Typical benchmarks** (8 directions, release):
  - 100 pts: 0.000s
  - 500 pts: 0.016s
  - 1000 pts: 0.044s
  - 5000 pts: 0.883s

---

## Module 3: Gaussian Prediction Intervals Backend

### Purpose
Uncertainty quantification for kriging predictions via confidence/prediction intervals.

### Rust API

```rust
use wbspatialstats::kriging::prediction_intervals::*;

pub fn kriging_prediction_interval_gaussian(
    prediction: f64,
    kriging_variance: f64,
    confidence: f64,  // 0.5 < confidence < 1.0
) -> Result<PredictionInterval, String>

pub struct PredictionInterval {
    pub lower: f64,
    pub point_estimate: f64,
    pub upper: f64,
    pub confidence: f64,
    pub method: String,
    pub margin_of_error: f64,
}
```

**With Measurement Uncertainty:**
```rust
pub fn kriging_prediction_interval_posterior(
    prediction: f64,
    kriging_variance: f64,
    residual_std: f64,        // Standard deviation of measurement error
    confidence: f64,
) -> Result<PredictionInterval, String>
// Total variance = kriging_variance + residual_std²
```

**Calibration Assessment:**
```rust
pub fn assess_interval_calibration(
    predictions: &[f64],
    intervals: &[PredictionInterval],
    observations: &[f64],
) -> Result<IntervalCalibration, String>

pub struct IntervalCalibration {
    pub observed_coverage: f64,     // Fraction of obs within intervals
    pub expected_coverage: f64,     // Target (e.g., 0.95)
    pub coverage_deficit: f64,      // |observed - expected|
    pub mean_interval_width: f64,
    pub is_calibrated: bool,        // deficit ≤ 0.05?
}

impl PredictionInterval {
    pub fn width(&self) -> f64;
    pub fn contains(&self, value: f64) -> bool;
    pub fn normalized_margin(&self) -> f64;  // margin / point_estimate
}
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `prediction` | `f64` | Kriged point estimate (μ) |
| `kriging_variance` | `f64` | Kriging variance (σ²_kriging) |
| `residual_std` | `f64` | Measurement error std dev (σ_residual) |
| `confidence` | `f64` | Confidence level (e.g., 0.95 for 95% CI) |

### Formula

**Gaussian Interval:**
```
z_crit = standard_normal_quantile(confidence)
margin = z_crit × √(kriging_variance)
CI = [prediction - margin, prediction + margin]
```

**Posterior (with measurement error):**
```
total_variance = kriging_variance + residual_std²
margin = z_crit × √(total_variance)
```

### Example Usage (Rust)

```rust
// After ordinary kriging prediction
let kriged_value = 45.3;
let kriging_var = 2.5;

// 95% confidence interval
let interval = kriging_prediction_interval_gaussian(
    kriged_value,
    kriging_var,
    0.95,
)?;

println!("95% Prediction Interval: [{:.2}, {:.2}]",
    interval.lower, interval.upper);

// With measurement uncertainty (e.g., sensor noise)
let measurement_std = 0.5;
let interval_posterior = kriging_prediction_interval_posterior(
    kriged_value,
    kriging_var,
    measurement_std,
    0.95,
)?;

println!("With measurement error: [{:.2}, {:.2}]",
    interval_posterior.lower, interval_posterior.upper);

// Validate interval calibration on test set
let result = assess_interval_calibration(&predictions, &intervals, &observations)?;

if result.is_calibrated {
    println!("✓ Intervals well-calibrated (coverage = {:.1}%)",
        result.observed_coverage * 100.0);
} else {
    println!("⚠️  Intervals {} calibrated (coverage = {:.1}%, target = {:.1}%)",
        if result.coverage_deficit > 0.0 { "over" } else { "under" },
        result.observed_coverage * 100.0,
        result.expected_coverage * 100.0);
}
```

### Performance

- **Complexity**: O(1) per prediction (negligible)
- **Typical benchmarks** (release):
  - 100 predictions: 114 ns each
  - 10000 predictions: 33.6 ns each
  - 100000 predictions: 29.5 ns each
  - Calibration (1000 samples): 0.000s

---

## Frontend Integration Points

### Wbtools_oss Tool Wrappers

Tools that will integrate Phase 1 modules in Phase 2:

```
Permutation Testing:
  - GlobalMoransITool → add --permutation, --num_simulations flags
  - LocalMoransILisaTool → add --permutation, --fdr_correction flags
  - GetisOrdGiStarTool → add --permutation flag

Directional Variography:
  - NEW: DirectionalVariogramTool
  - VariogramFitterTool → add --anisotropy, --azimuth_bins flags
  - OrdinaryKrigingTool → add --use_anisotropy flag

Prediction Intervals:
  - OrdinaryKrigingTool → add --output_intervals, --confidence_level flags
  - LocalKrigingTool → add --output_intervals flag
  - UniversalKrigingTool → add --output_intervals flag
```

### Python Binding (wbw_python)

```python
import wbw_python

env = wbw_python.WhiteboxEnvironment()

# Permutation testing
result = env.global_morans_i(
    input_file="values.tif",
    weights_file="weights.geojson",
    permutation=True,
    num_simulations=999,
    seed=42,
    output_file="morans_i_result.json"
)

# LISA with FDR
result = env.local_morans_i_lisa(
    input_file="values.tif",
    weights_file="weights.geojson",
    permutation=True,
    fdr_correction=True,
    output_file="lisa_clusters.shp"
)

# Directional variography
result = env.directional_variogram(
    input_file="samples.shp",
    value_field="concentration",
    azimuth_bins=8,
    tolerance=22.5,
    max_distance=1000.0,
    bin_size=50.0,
    output_file="directional_vgram.json"
)

# Kriging with prediction intervals
result = env.ordinary_kriging(
    input_file="training.shp",
    prediction_grid="grid.tif",
    output_intervals=True,
    confidence_level=0.95,
    interval_method="gaussian",
    output_file="kriged_with_intervals.tif"
)
```

### R Binding (wbw_r)

```r
library(whitebox)

# Permutation test
morans_i_perm <- wbt_global_morans_i(
  values = values,
  weights = W,
  permutation = TRUE,
  num_simulations = 999,
  seed = 42
)

# LISA with FDR
lisa <- wbt_local_morans_i_lisa(
  values = values,
  weights = W,
  permutation = TRUE,
  fdr_correction = TRUE
)

# Directional variography
dv <- wbt_directional_variogram(
  locations = coords,
  values = values,
  azimuth_bins = 8,
  tolerance = 22.5,
  max_distance = 1000
)

# Kriging with intervals
kriged <- wbt_ordinary_kriging(
  locations = training_locs,
  values = training_vals,
  prediction_grid = grid,
  output_intervals = TRUE,
  confidence_level = 0.95
)
```

### QGIS Plugin (wbw_qgis)

Accessible via:
- Processing Toolbox → Whitebox → Spatial Statistics
- Same parameters as R/Python bindings
- Output: GeoJSON/GeoTIFF with intervals as attributes

---

## Testing & Validation

See `docs/phase1_cross_validation.py` for expected results on public datasets:
- **Meuse**: Expected I = 0.4-0.6, p < 0.05
- **Columbus**: HH clusters in corners, LL in middle
- **NC SIDS**: Moderate autocorrelation (I ≈ 0.2-0.4)
- **Directional**: Anisotropy ratio detectable at 4:1 domain ratio

---

## References

1. **Permutation Testing**: 
   - Anselin & Bera (1998) "Spatial dependence in linear regression models"
   - Sokal (1979) "Testing for correlation between traits of relatives"

2. **Directional Variography**:
   - Journel & Huijbregts (1978) "Mining Geostatistics"
   - Goovaerts (1997) "Geostatistics for Natural Resources Evaluation"

3. **Prediction Intervals**:
   - Wackernagel (1995) "Multivariate Geostatistics"
   - Cressie (1993) "Statistics for Spatial Data"

---

## Support & Issues

For bugs, feature requests, or clarifications:
- Backend: File issue in whitebox_next_gen repo
- Bindings: File issue in wbw_python/wbw_r/wbw_qgis repos
- Documentation: Update docs/phase1_*.md
