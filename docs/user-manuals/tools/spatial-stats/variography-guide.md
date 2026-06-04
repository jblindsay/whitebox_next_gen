# Variography & Variogram Fitting

## What is a Variogram?

A variogram quantifies spatial correlation in geostatistical data. Specifically, the *semivariogram* γ(h) measures average squared difference between values at locations separated by distance h:

γ(h) = 0.5 * mean[(z(u) - z(u+h))²]

**Interpretation:**
- Small h: Small γ(h) (nearby values are similar)
- Large h: Large γ(h) (distant values differ more)
- At some distance (the *range*): γ(h) plateaus at the *sill* (maximum variance)

The variogram is the **foundation for all kriging**. It captures spatial correlation structure that kriging uses for weighted interpolation.

---

## Variography Tools Workflow

### 1. Estimate Variogram (Empirical)

**Purpose:** Compute empirical semivariogram from point observations.

**Parameters:**
- `input`: Point layer with measured values
- `field`: Attribute field with values to analyze
- `lag_distance`: Distance bin size (e.g., 100 units)
- `lag_tolerance`: Tolerance for binning (e.g., ±50 units)
- `max_lag_count`: Maximum number of lags to compute (e.g., 20)

**Process:**
1. For each lag bin [h - tolerance, h + tolerance]:
   - Find all point pairs with separation distance in that bin
   - Compute squared difference for each pair
   - Average squared differences to get γ(h)
2. Output: Empirical lag data with distances and semivariance values

**Interpretation Tips:**
- **Nugget**: Small-lag variance (measurement error + micro-scale variation)
- **Sill**: Asymptotic variance (maximum spatial correlation distance)
- **Range**: Distance where semivariance reaches ~95% of sill (correlation distance)

**Visualize the Output:**
Plot distance (x-axis) vs. semivariance (y-axis). Look for:
- Smooth increase from nugget → sill
- Clear range (where curve flattens)
- Directional patterns (anisotropy)

---

### 2. Fit Variogram Model

**Purpose:** Smooth empirical variogram with theoretical model (Spherical, Exponential, Gaussian).

**Why?** Empirical variograms are noisy. Theoretical models:
- Provide smooth, continuous functions for kriging
- Define consistent behavior between lag distances
- Ensure positive-definite kriging matrix

**Models Provided:**

| Model | Formula | Use Case |
|-------|---------|----------|
| **Spherical** | γ(h) = C₀ + C₁[1.5(h/a) - 0.5(h/a)³] for h ≤ a | Abrupt spatial transitions, clear range |
| **Exponential** | γ(h) = C₀ + C₁[1 - exp(-h/a)] | Gradual decrease, practical range = 3a |
| **Gaussian** | γ(h) = C₀ + C₁[1 - exp(-(h/a)²)] | Very smooth, parabolic near origin |

**Parameters:**
- Nugget (C₀): Small-scale variance
- Sill (C₀ + C₁): Asymptotic variance
- Range (a): Correlation distance

**Model Selection:**
1. Visual inspection: Which model best fits empirical curve?
2. Cross-validation: Fit each model, compare LOOCV diagnostics
3. Information criteria: Use AICc, BIC for formal comparison

**Output:** Fitted model parameters used by kriging tools.

---

### 3. Directional Variogram Analysis

**Purpose:** Detect and characterize spatial anisotropy—directional variation in correlation structure.

**When anisotropy occurs:**
- Geological layering (bedding planes, faulting)
- Wind/flow patterns (groundwater, atmospheric)
- Linear infrastructure (roads, rivers)

**Parameters:**
- `directions`: List of azimuths (0-180°) to analyze
  - Example: 0°, 45°, 90°, 135° (cardinal + diagonal)
- `tolerance`: Direction tolerance in degrees (e.g., ±22.5°)

**Process:**
1. For each direction, compute separate variogram
2. Compare ranges/sills across directions
3. Detect if one direction has shorter range (restricted correlation)

**Output:**
- Directional variogram values for each azimuth
- Optional rose diagram showing correlation intensity by direction

**Interpretation:**
- If all directions similar: Isotropic (no anisotropy)
- If directions differ: Anisotropic
  - One direction shorter range = major axis of correlation
  - Perpendicular direction longer range = minor axis

**Use with Kriging:** 
Anisotropic variograms can be incorporated into kriging for realistic interpolation when directionality is present. Otherwise kriging biases predictions perpendicular to main correlation direction.

---

### 4. Kriging Cross-Validation

**Purpose:** Validate kriging model by comparing predictions to actual holdout values.

**Method:** Leave-One-Out Cross-Validation (LOOCV)
1. Remove each data point sequentially
2. Predict its value using remaining points
3. Compare prediction to actual value
4. Compute diagnostic statistics

**Key Statistics:**

| Statistic | Interpretation | Target |
|-----------|---|---|
| **ME (Mean Error)** | Bias in predictions | Near 0 |
| **MAE (Mean Absolute Error)** | Average prediction accuracy | Lower is better |
| **RMSE (Root Mean Squared Error)** | Emphasizes large errors | Lower is better |
| **Standardized Residuals** | Normalized prediction errors | Should be Normal(0,1) |

**Validation Interpretation:**
- **ME ≈ 0**: Predictions are unbiased
- **RMSE small**: Predictions are accurate
- **Standardized residuals ~ Normal**: Kriging variance estimates are realistic
  - If residuals have high std dev > 1: Kriging underestimated variance
  - If residuals have low std dev < 1: Kriging overestimated variance

**When Validation Fails:**
- Poor ME → Wrong mean assumption (use Universal Kriging to detrend)
- Poor RMSE → Poor variogram model (refine fitting)
- Bad standardized residuals → Variance estimates unreliable (revisit model)

---

## Variography Workflow Example

```
1. Load point data (soil contamination measurements)
   ↓
2. ESTIMATE VARIOGRAM (lag_distance=100, max_lag=20)
   → Examine empirical variogram plot
   → Identify nugget, sill, range visually
   ↓
3. FIT VARIOGRAM MODEL (try Spherical first)
   → Overlay fitted model on empirical points
   → Examine fit quality
   ↓
4. DIRECTIONAL VARIOGRAM (azimuths: 0°, 45°, 90°, 135°)
   → Check for anisotropy
   → If detected, note major/minor axes
   ↓
5. KRIGING CROSS-VALIDATION
   → Examine ME, MAE, RMSE, standardized errors
   → If unsatisfactory, revisit model fitting
   ↓
6. ORDINARY KRIGING with validated model
   → Generate interpolated raster
   → Map predictions + variance
```

---

## Variogram Troubleshooting

| Problem | Likely Cause | Solution |
|---------|---|---|
| Very small range | Strong local clustering | Use Local Kriging; check for outliers |
| Erratic variogram | Insufficient data/poor spatial coverage | Increase data points; check for outliers |
| Poor LOOCV fit | Wrong variogram model | Try different models (Spherical, Exponential, Gaussian) |
| High kriging variance everywhere | Small sill relative to nugget | Measurement error is dominant; more/better data needed |
| Anisotropic pattern unclear | Weak anisotropy or wrong directions | Rotate azimuth angles; increase tolerance |

---

## References

- Chilès, J.-P., & Delfiner, P. (2012). *Geostatistics: Modeling Spatial Uncertainty* (2nd ed.). Wiley.
- Isaaks, E. H., & Srivastava, R. M. (1989). *An Introduction to Applied Geostatistics*. Oxford University Press.
- Wackernagel, H. (2003). *Multivariate Geostatistics* (3rd ed.). Springer-Verlag.
