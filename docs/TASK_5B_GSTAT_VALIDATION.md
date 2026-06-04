# Task 5B: Gstat Parity Validation

## Objective

Validate wbgeostats kriging implementation against the reference R gstat library across:
- Variogram model fitting (spherical, exponential, gaussian)
- Kriging predictions and uncertainty estimates
- Cross-validation metrics
- Performance benchmarking

---

## Strategy

### Phase 1: Variogram Fitting Validation (Current)

**Reference Dataset**: Meuse river floodplain (sp R package)
- 155 training points with heavy metal measurements (zinc)
- 3,103 prediction grid cells
- Classic geostatistics benchmark

**Validation Approach**:
1. Load Meuse dataset from R sp package
2. Fit variogram with gstat for each model family
3. Extract fitted parameters (nugget, partial sill, range)
4. Compare against wbgeostats fits (once Python bindings available)

**Expected Tolerances**:
- Nugget, partial_sill, range: ±5% or ±small_absolute value
- Wrap Range Estimation Squared (WRSS): ±10%

### Phase 2: Prediction Validation (Requires Task 5C)

Once Python bindings available:
1. Run OrdinaryKriging on Meuse dataset
2. Predict at all grid points
3. Compare predictions vs gstat output
4. Calculate metrics:
   - RMSE (root mean square error)
   - MAE (mean absolute error)
   - Correlation coefficient
   - CI coverage (95% prediction intervals)

### Phase 3: Synthetic Data Validation (Now Available)

**Test Cases** (with known ground truth):
1. Spherical model (nugget=100, psill=2000, range=500)
2. Exponential model (nugget=200, psill=1500, range=800)
3. Gaussian model (nugget=150, psill=2500, range=1200)

**Validation**:
- Generate synthetic data with known spatial structure
- Fit variogram with wbgeostats
- Compare fitted parameters vs ground truth
- Cross-validate via LOOCV

### Phase 4: Performance Benchmarking (Later)

Compare computation time for:
- Variogram estimation (R gstat vs wbgeostats)
- Model fitting (optimize parameters)
- Kriging predictions (parallel vs serial)
- Overall pipeline throughput

---

## Implementation Status

### Available Now

✅ **Synthetic Data Validator** (`validate_kriging_synthetic.py`)
- Generates synthetic point data with known spatial structure
- Creates test cases for spherical, exponential, gaussian models
- Outputs validation baseline

✅ **Gstat Interface** (`validate_kriging_vs_gstat.py`)
- Loads Meuse dataset from R sp package
- Fits variogram with R gstat
- Extracts model parameters
- Placeholder for prediction comparison

### Blocked on Task 5C

⏳ **Python Bindings** (PyO3 wrapper)
- Required to call wbgeostats kriging from Python scripts
- Needed for prediction comparison and benchmarking

---

## Running Validation Scripts

### Synthetic Data (No R Required)

```bash
python scripts/validate_kriging_synthetic.py
```

Output: `kriging_synthetic_validation.json` with test case definitions and data statistics.

### Gstat Comparison (Requires R + rpy2)

```bash
# Install dependencies
pip install rpy2

# In R
install.packages(c("sp", "gstat"))

# Run validation
python scripts/validate_kriging_vs_gstat.py
```

Output: `kriging_validation_results.json` with gstat variogram fits.

---

## Expected Results

### Variogram Fitting Accuracy

For Meuse dataset (gstat reference):

| Model | Nugget | Psill | Range |
|-------|--------|-------|-------|
| Spherical | ~15000 | ~35000 | ~900 |
| Exponential | ~13000 | ~38000 | ~700 |
| Gaussian | ~12000 | ~40000 | ~600 |

Target: wbgeostats fits within ±5% of gstat values.

### Cross-Validation Metrics

For well-calibrated kriging:
- Mean Error (ME) ≈ 0 (unbiased)
- RMSSE ∈ [0.8, 1.2] (properly scaled uncertainty)
- Correlation ≥ 0.95 (high predictive power)

---

## Dependency Installation

### Required for Gstat Comparison

```bash
# Install rpy2 (R interface for Python)
pip install rpy2

# Install R packages (in R console)
install.packages("sp")
install.packages("gstat")
```

### Optional for Full Workflow

```bash
# For detailed analysis
pip install pandas matplotlib scipy

# For performance profiling
pip install line_profiler memory_profiler
```

---

## Files

- `scripts/validate_kriging_synthetic.py` — Synthetic data test suite
- `scripts/validate_kriging_vs_gstat.py` — Meuse dataset + gstat comparison
- `kriging_synthetic_validation.json` — Synthetic validation output
- `kriging_validation_results.json` — Gstat comparison results

---

## Next Steps

1. **Immediate** (Task 5B Progress):
   - Run synthetic validation script
   - Document baseline results
   - Install rpy2 + R packages for gstat comparison

2. **After Task 5C** (Python Bindings):
   - Integrate wbgeostats predictions into validation scripts
   - Run full comparison vs gstat
   - Calculate accuracy metrics

3. **After Task 5D** (R Bindings):
   - Create R script for direct gstat integration
   - Benchmark performance vs gstat
   - Generate publication-ready comparison plots

4. **Task 5E** (Documentation):
   - Include validation results in API documentation
   - Note any known differences vs gstat
   - Provide tuning guidance for specific use cases

---

## References

- **Meuse Dataset**: Burrough et al. (1989), sp R package
- **Gstat Reference**: Pebesma & Wesseling (1998), https://gstat.org/
- **Kriging Theory**: Chilès & Delfiner (2012), "Geostatistics: Modeling Spatial Uncertainty"
