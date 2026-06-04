# Point Pattern Analysis

## Overview

Point pattern analysis examines spatial arrangement of events (disease cases, crimes, tree locations, earthquakes) to detect clustering, dispersion, or randomness. Unlike attribute-based spatial statistics (autocorrelation, regression), point analysis focuses on **whether locations themselves form patterns**.

**Key Questions:**
- Are events randomly distributed?
- Are they clustered (aggregated)?
- Are they dispersed (regular/even)?
- Does clustering vary at different spatial scales?

---

## Basic Point Pattern Tests

### Nearest Neighbour Index (NNI)

**Purpose:** Simple clustering vs. dispersion test based on average nearest neighbor distance.

**Concept:**
1. For each point, measure distance to nearest neighbor
2. Compute mean nearest neighbor distance (d̄)
3. Compare to expected distance under complete spatial randomness (CSR)

**Statistic:**
R = d̄_observed / d̄_expected

**Interpretation:**
- **R ≈ 1:** Random distribution (CSR)
- **R < 1:** Clustering (nearby points closer than random)
- **R > 1:** Dispersion (nearby points farther than random)
- **R → 0:** Perfect clustering (all points at single location)
- **R → 2.15:** Maximum dispersion (perfect grid)

**Advantages:**
- Simple, computationally efficient
- Single summary statistic
- Well-understood null distribution

**Limitations:**
- Sensitive to boundary effects (edge bias)
- Misses scale-dependent patterns (clustering at some scales, random at others)
- Assumes complete spatial randomness under null (often violated with inhomogeneous intensity)

**When to use:** Quick screening for obvious clustering vs. dispersion; preliminary analysis before detailed multi-scale methods.

---

### Quadrat Count Test

**Purpose:** Grid-based test dividing region into cells (quadrats), counting points per cell.

**Concept:**
1. Divide study area into regular grid cells
2. Count points per cell
3. Compute chi-squared test: Are cell counts uniform or variable?

**Parameters:**
- `quadrat_size`: Cell side length (balance between detail and sample size)
  - Small cells: Fine-grained pattern detail but sparse counts
  - Large cells: Robust counts but coarse pattern

**Output:**
- Expected count per cell (uniform distribution)
- Observed counts by cell
- Chi-squared statistic, p-value, Variance-to-Mean ratio

**Interpretation:**
- **Variance:Mean = 1:** Random (Poisson)
- **Variance:Mean > 1:** Clustered (overdispersed)
- **Variance:Mean < 1:** Dispersed (underdispersed)

**Advantages:**
- Flexible; can use irregular quadrat shapes/sizes
- Maps spatial structure directly (show counts per quadrat)

**Limitations:**
- Coarse resolution; misses fine-scale patterns
- Quadrat size choice affects results (sensitivity analysis recommended)

**When to use:** When study area is naturally divided (administrative boundaries, management zones) or when looking for broad-scale patterns.

---

## Multi-Scale Point Process Analysis

### Ripley's K Function

**Purpose:** Scale-dependent clustering analysis revealing clustering/dispersion at different distances.

**Concept:**
1. For each distance d, count average number of points within distance d of a typical point
2. Normalize by point density to get K(d)
3. Compare to expected K(d) under CSR

**Statistic:**
K(d) = λ⁻¹ * E[number of points within distance d of typical point]

Where λ = overall point density.

**Interpretation:**
- **K(d) > π*d²:** More points at distance d than random (clustering)
- **K(d) = π*d²:** Random at distance d
- **K(d) < π*d²:** Fewer points at distance d than random (dispersion)

**L-Function (Stabilized Ripley's K):**
L(d) = √[K(d)/π] - d

- L(d) > 0: Clustering
- L(d) = 0: Random
- L(d) < 0: Dispersion

**Advantages:**
- Reveals multiple scales of clustering
- Sensitive across full range of distances
- Can identify clustering range (where L peaks)

**Output Options:**
- **Single point pattern:** L(d) for single dataset
- **Confidence envelope (Monte Carlo):** Generate random patterns, compare observed to envelope
  - If observed L(d) exceeds upper envelope: Significant clustering
  - If observed L(d) below lower envelope: Significant dispersion

**When to use:** Detailed exploratory analysis identifying clustering scales; appropriate for ecological, epidemiological point patterns.

---

### Envelope Testing

**Purpose:** Formal significance testing via Monte Carlo simulation envelope.

**Method:**
1. Generate m simulated point patterns under null model (usually CSR)
2. Compute test statistic (e.g., K function) for each simulation
3. Compute upper/lower envelopes across simulations
4. Compare observed statistic to envelope
5. Calculate significance: proportion of simulations exceeding observed

**Outputs:**
- Significance envelope plot (shaded region)
- Observed curve
- p-value (proportion of simulations exceeding observed)

**Envelopes:**
- **Complete Spatial Randomness (CSR):** Null = uniform random points
- **Inhomogeneous Poisson:** Null = variable intensity by region (more realistic when intensity varies)
- **Fitted point process:** Null = fitted parametric model (most specific if model is known)

**Interpretation:**
- Observed curve entirely within envelope → Not significantly different from null
- Observed curve outside envelope → Significant departure (clustering or dispersion)
- Early departure (small d): Local clustering
- Late departure (large d): Regional clustering

**Computational Cost:** High (m simulations × full pattern analysis each); practical for m=99-999 simulations.

**When to use:** Formal hypothesis testing; publication-quality results; when visual inspection is insufficient.

---

## Advanced Point Process Tools

### Inhomogeneous Intensity Analysis

**Purpose:** Kernel density estimation (KDE) mapping point intensity (concentration per unit area).

**Concept:**
1. Place Gaussian/Epanechnikov kernel at each point
2. Evaluate kernel at grid cells
3. Sum kernels across all points
4. Normalize by kernel mass and count

**Bandwidth (Kernel Width):**
- Small bandwidth: Fine-grained detail, local clustering visible
- Large bandwidth: Smooth regional patterns, noise suppressed

**Bandwidth Selection:**
- **Rule of thumb:** Silverman's rule (quick default)
- **Data-driven:** Cross-validation or plug-in estimator
- **Manual:** Fixed for dataset comparability

**Output:**
- Intensity raster (density, units: points per unit area)
- Optionally: 95% confidence region (high-intensity areas)

**Use Cases:**
- **Disease mapping:** Identify high-incidence regions
- **Crime analysis:** Hotspot visualization
- **Ecology:** Species distribution estimation
- **Astronomy:** Galaxy cluster detection

---

### Point Process Residuals

**Purpose:** Diagnostic comparison of observed pattern to fitted Poisson process model.

**Concept:**
1. Fit intensity model (constant, linear trend, or custom)
2. Compute residuals: Observed minus expected points
3. Assess if residuals show spatial structure
4. If residuals clustered: Model incomplete (omitted factors)

**Residual Types:**
- **Raw residuals:** Observed counts - predicted counts per quadrat
- **Standardized residuals:** (Observed - Predicted) / √Predicted
- **Pearson residuals:** Standardized by variance (for non-Poisson models)

**Diagnostics:**
- **Spatial autocorrelation test (Moran's I):** Should be non-significant if model is adequate
- **Histogram:** Should be approximately normal
- **Q-Q plot:** Should follow 1:1 line

**Interpretation:**
- **Spatial clusters in residuals:** Model missing spatial structure (e.g., omitted covariates, true clustering)
- **High standard deviation:** Model underfitting (too simple)
- **Systematic patterns:** Non-Poisson process or structural changes

**When to use:** Model validation; identifying omitted variables or spatial processes.

---

## Point Pattern Analysis Workflow

```
1. Load point data (e.g., disease cases by location)
   ↓
2. NEAREST NEIGHBOUR INDEX (NNI)
   → R < 1 (clustering) or R > 1 (dispersion) or R ≈ 1 (random)?
   ↓
3. IF CLUSTERED or INTEREST IN SCALES:
   ├─ RIPLEY'S K ANALYSIS
   │  → Where do clusters occur? (peaks in L(d))
   │  → What scale? (distance d at peak)
   │
   └─ ENVELOPE TESTING (Monte Carlo)
      → Are patterns significant? (compare to CSR envelope)
   ↓
4. QUADRAT COUNT TEST (if natural grid exists)
   → Map cluster locations across management zones
   ↓
5. INHOMOGENEOUS INTENSITY (KDE)
   → Create intensity surface for visualization
   → Identify exact hotspots
   ↓
6. IF MODELING:
   ├─ FIT PARAMETRIC MODEL (constant/linear intensity)
   └─ POINT PROCESS RESIDUALS
      → Check for unmodeled spatial structure
   ↓
7. ACTION
   → Deploy resources to hotspots (HH clusters, high intensity)
   → Investigate coldspots (LL clusters, low intensity)
   → Monitor boundary transitions (intensity gradients)
```

---

## Practical Considerations

### Boundary Effects
Point patterns near study area boundaries are incompletely observed (neighbors outside area not counted). All point tests have boundary bias; mitigation:
- **Exclude edge:** Buffer inward, exclude near-boundary points
- **Toroidal correction:** Wrap around study area (unrealistic but avoids bias)
- **Inhomogeneous K:** Adjust for unobserved neighbors (requires intensity model)

### Intensity Variation
If point density varies spatially (inhomogeneous), standard CSR tests give false positives:
- **Solution 1:** Detrend (remove spatial trend, analyze residuals)
- **Solution 2:** Fit intensity model including spatial covariates
- **Solution 3:** Conditional test (CSR at given intensity level)

### Multiple Testing
Running tests at many distance values or quadrat sizes inflates false positive rates. Report key findings with appropriate multiple-comparison corrections.

---

## References

- Ripley, B. D. (1976). "The Second-Order Analysis of Stationary Point Processes." *Journal of Applied Probability*, 13(2), 255-266.
- Wiegand, T., & Moloney, K. A. (2014). "Handbook of Spatial Point-Pattern Analysis in Ecology." *Ecology Letters*, 17(11), 1411-1425.
- Diggle, P. J. (2003). *Statistical Analysis of Spatial and Spatio-Temporal Point Patterns* (3rd ed.). Chapman and Hall.
- Clark, P. J., & Evans, F. C. (1954). "Distance to Nearest Neighbor as a Measure of Spatial Relationships in Populations." *Ecology*, 35(4), 445-453.
