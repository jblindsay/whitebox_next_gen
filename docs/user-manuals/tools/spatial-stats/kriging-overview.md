# Kriging Interpolation Methods

## Overview

Kriging is a family of geostatistical interpolation techniques that produce both predictions and uncertainty estimates at unsampled locations. All kriging methods leverage spatial correlation structure captured by variograms to assign weights to neighboring observations. The primary advantage of kriging over simpler interpolation (IDW, splines) is the explicit modeling of spatial correlation and the provision of prediction variance, enabling uncertainty quantification.

## Kriging Variants in Whitebox

### Ordinary Kriging

**When to use:** Your primary choice when the mean is unknown and varies naturally. Ordinary kriging is the most commonly used variant.

**Prerequisites:**
- Minimum 3 training points (recommended: 10-50 for typical analysis)
- Numeric field with continuous values
- Reasonable spatial extent (kriging assumes stationarity within the domain)

**Parameters:**
- `points`: Input vector layer with measurement locations
- `value_field`: Attribute field containing values to interpolate
- `cell_size`: Output grid resolution (or use `base_raster` to inherit geometry)
- `base_raster`: Optional template defining output extent/resolution

**Output:**
- Interpolated raster with predicted values
- (Optional) Kriging variance raster showing prediction uncertainty

**Interpretation:** Kriging variance is independent of actual values—it depends only on sample configuration and variogram model. High variance indicates sparse/clustered training points; low variance indicates dense/well-distributed training points. Use variance maps for survey design: allocate additional samples to high-variance areas.

**Computational Cost:** O(n³) for fitting, O(n) per prediction. Practical for datasets up to ~5,000 points. For larger datasets, use Local Ordinary Kriging.

---

### Local Ordinary Kriging

**When to use:** Large datasets (>5,000 points) or when spatial correlation structure varies spatially (non-stationary phenomena).

**Advantages over Ordinary Kriging:**
- Computationally feasible for large datasets
- Automatically handles spatial non-stationarity
- Produces locally optimized weights

**Key Parameters:**
- `k_neighbors`: Number of nearest neighbors per prediction (typical: 8-20)
  - Smaller k: Faster computation, more local influence
  - Larger k: Smoother results, more global influence, slower computation
- Other parameters: Same as Ordinary Kriging

**Practical Guidance:**
- Start with k=10 for exploratory analysis; adjust based on pattern smoothness
- Use k=6-8 for very detailed local patterns
- Use k=15-20 for regional trends
- Vary k to perform sensitivity analysis

---

### Simple Kriging

**When to use:** Rare. Only when the mean is known a priori with high confidence.

**Prerequisites:**
- Known constant mean (from theory, regional calibration, or long-term average)
- Same data requirements as Ordinary Kriging otherwise

**Advantages:**
- Lower kriging variance than Ordinary Kriging (variance reduction = confidence in known mean)
- Fewer equations to solve (slightly faster)

**Typical Applications:**
- Anomaly detection from known baseline (e.g., soil contamination above background)
- When regional mean from external data is highly reliable

**Caution:** Specifying incorrect mean systematically biases predictions. Use only when confident.

---

### Universal Kriging

**When to use:** Data exhibiting systematic spatial trend (elevation gradients, temperature change with latitude, resource grades with geological structure).

**How It Works:**
1. Fit polynomial trend (linear or quadratic) to capture large-scale structure
2. Compute residuals (data minus trend)
3. Apply ordinary kriging to residuals
4. Combine trend + kriged residuals for final prediction

**Parameters:**
- `trend_order`: 
  - 1 = Linear trend (sloping plane)
  - 2 = Quadratic trend (curved surface)

**When to use each:**
- Linear (order=1): Smooth, monotonic change across domain (e.g., temperature gradient)
- Quadratic (order=2): Curved patterns, peaks/valleys (e.g., elevation with hills)

**Diagnostic:** If ordinary kriging residuals (after detrending) show spatial autocorrelation patterns, you need Universal Kriging.

**Output Interpretation:** Predictions combine global trend + local spatial variation. Kriging variance still reflects prediction uncertainty but now on residuals.

---

### Space-Time Kriging

**When to use:** Spatio-temporal point series with temporal dimension (e.g., monitoring networks, environmental sensors, time-stamped samples).

**Key Concept:** Jointly models spatial and temporal correlation. Points from nearby times AND nearby locations are weighted more heavily.

**Prerequisites:**
- Time field in attribute table (numeric: days since epoch, fractional years, etc.)
- Sufficient temporal density (multiple time slices recommended)

**Parameters:**
- `value_field`: Measured values
- `time_field`: Temporal coordinates

**Applications:**
- Gap-filling in sensor networks (missing measurements in space/time)
- Environmental trend analysis with uncertainty
- Disease surveillance with changing spatial patterns
- Climate variable interpolation over time

**Computational Complexity:** Higher than spatial kriging alone; O(n⁴) in space-time. Practical for datasets with moderate points × time steps.

---

## Kriging Workflow Best Practices

### Step 1: Data Preparation
- Check for outliers (kriging is sensitive to extreme values)
- Verify adequate spatial coverage (clustered data limits interpolation range)
- Inspect attribute field for missing values

### Step 2: Variogram Estimation
- Use **Estimate Variogram** tool to examine spatial correlation structure
- Plot variogram: Look for nugget (small-scale variance), sill (asymptotic variance), range (correlation distance)

### Step 3: Variogram Fitting
- Use **Fit Variogram Model** to select Spherical, Exponential, or Gaussian model
- Compare visual fit; model choice affects predictions for sparse regions

### Step 4: Cross-Validation
- Run **Kriging Cross-Validation** (LOOCV) to validate model
- Examine diagnostics:
  - Mean Error: Should be near 0 (unbiased)
  - RMSE: Lower is better
  - Standardized errors: Should be approximately normal with mean 0, std dev 1

### Step 5: Interpolation
- Choose kriging variant (ordinary, local, simple, universal, space-time)
- Select output grid resolution and extent
- Run kriging and examine predictions + variance

### Step 6: Interpretation
- Map predictions for spatial patterns
- Map variance to identify high-uncertainty regions
- Validate with independent test data if available

---

## References & Further Reading

- Matheron, G. (1963). "Principles of Geostatistics." *Economic Geology*, 58(8), 1246-1266.
- Cressie, N. (1993). *Statistics for Spatial Data* (Revised Edition). Wiley.
- Journel, A. G., & Huijbregts, C. J. (1978). *Mining Geostatistics*. Academic Press.
- Wackernagel, H. (2003). *Multivariate Geostatistics* (3rd ed.). Springer-Verlag.

---

## Quick Reference: Kriging Variant Selection

| Scenario | Variant | Reason |
|----------|---------|--------|
| Unknown, constant mean | **Ordinary** | Most general, most common |
| Large dataset (>5k points) | **Local Ordinary** | Computational feasibility |
| Known constant mean | **Simple** | Reduces variance if confident |
| Spatial trend present | **Universal** | Detrends before kriging |
| Time-series data | **Space-Time** | Leverages temporal correlation |
| Non-stationary region | **Local Ordinary** | Adapts to local variogram |
