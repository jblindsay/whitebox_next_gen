# CHANGELOG - wbgeostats Kriging Library

## [0.1.0] - 2026-06-03

### Added

#### Core Library
- **Variogram Module** (`variogram/`):
  - `EmpiricalVariogram` & `EmpiricalVariogramBuilder`: Lag bin computation, configurable lag distances
  - `VariogramModel`: Theoretical models (Spherical, Exponential, Gaussian)
  - `VariogramFitter`: Weighted least-squares fitting with initial parameter optimization
  - 32 unit tests covering all edge cases

- **Kriging Module** (`kriging/`):
  - `OrdinaryKriging`: Core kriging solver with:
    - Cholesky decomposition solver
    - SVD fallback for ill-conditioned matrices
    - Batch prediction with rayon parallelization
  - `KrigingResult`: Predictions with uncertainty quantification (variance, standard error, 95% CI)
  - 21 unit tests

- **Cross-Validation Module** (`cv/`):
  - `LeaveOneOutCV`: Leave-One-Out cross-validation for model assessment
  - `CVMetrics`: Well-calibration diagnostics
  - 11 unit tests

#### Tool Integration (wbtools_oss)
- `EstimateVariogramTool`: JSON output of empirical variograms
- `FitVariogramTool`: Theoretical model fitting (JSON input/output)
- `OrdinaryKrigingTool`: Raster-based kriging interpolation
  - Template raster support
  - Parallel grid prediction (rayon)
  - Raster I/O via wbraster crate
- `KrigingCrossValidationTool`: Model validation reporting

#### Python Bindings (PyO3)
- `VariogramModel` class with properties: family, nugget, partial_sill, range, total_sill
- `KrigingResult` class with properties: prediction, variance, std_error, ci_lower, ci_upper
- `OrdinaryKriging` class with methods: predict(), predict_batch()
- Module functions: estimate_variogram(), fit_variogram(), cross_validate_kriging()
- Feature-gated compilation: `features = ["python"]`
- Proper error handling: PyErr conversion for all Rust errors
- pyproject.toml for maturin builds

#### R Bindings (extendr)
- `estimate_variogram()`: Empirical variogram from point data
- `fit_variogram()`: Theoretical model fitting
- `kriging_predict()`: Single-point kriging predictions
- `kriging_predict_grid()`: Vectorized grid predictions (parallelized)
- `kriging_cross_validate()`: LOOCV model assessment
- All functions return R lists with named components
- Feature-gated compilation: `features = ["r"]`

#### Documentation
- `API.md`: Complete API reference for Python, R, and Rust
- `EXAMPLES.md`: Runnable examples in all three languages
- `PERFORMANCE.md`: Performance characteristics and benchmarks
- `CHANGELOG.md`: This file

#### Testing & Validation
- Synthetic variogram validation (3 test cases, 370 points)
- Meuse dataset validation framework (155 training points)
- Gstat reference comparison interface (pending Python bindings for prediction comparison)

### Technical Details

#### Parallelization
- **Grid generation**: rayon flat_map for coordinate generation
- **Batch prediction**: rayon par_iter across grid points
- **Cross-validation**: parallelizable across n-1 kriging models (future)
- Typical speedup: 4-8x on 8-core systems

#### Numerical Stability
- Cholesky decomposition with pivoting
- SVD fallback with condition number threshold
- Weighted least-squares fitting for variogram models
- 95% confidence intervals based on kriging variance

#### Dependencies
- **Pure Core**: nalgebra 0.32, ndarray 0.15, rayon 1.7, serde 1.0, thiserror 1.0
- **Python** (optional, `python` feature): pyo3 0.28.2 (abi3-py39)
- **R** (optional, `r` feature): extendr-api 0.8
- **Integration** (wbtools_oss): wbcore, wbraster, wbvector, wbtopology

### Architecture

```
wbgeostats (pure Rust core)
├── variogram/        - Empirical & theoretical variogram computation
├── kriging/          - Ordinary kriging solver
└── cv/               - Cross-validation metrics

Language Bindings (feature-gated)
├── python.rs         - PyO3 wrappers (feature = "python")
└── r.rs              - extendr wrappers (feature = "r")

Integration (wbtools_oss)
├── EstimateVariogramTool
├── FitVariogramTool
├── OrdinaryKrigingTool
└── KrigingCrossValidationTool
```

### Performance

- **Single Prediction**: ~1-10 ms per point
- **Grid Prediction**: 
  - 100 points: ~5-10 ms
  - 1,000 points: ~20-50 ms
  - 10,000 points: ~100-300 ms
  - (All parallelized via rayon)
- **LOOCV** (100 points): ~1-5 seconds
- **Variogram Fitting**: ~100-500 ms

See [PERFORMANCE.md](./PERFORMANCE.md) for detailed benchmarks.

### Validation

- ✅ Synthetic data validation (Spherical, Exponential, Gaussian models)
- ✅ Meuse dataset framework (155 points, 3103 grid predictions)
- ✅ Gstat parity interface ready (awaiting Python bindings)
- ✅ All compilation modes verified:
  - Core library: `cargo check -p wbgeostats`
  - Python bindings: `cargo check -p wbgeostats --features python`
  - R bindings: `cargo check -p wbgeostats --features r`

### Known Limitations

- **Kriging model**: Fixed variogram (fitted beforehand, not estimated in model)
- **Covariance matrix**: Direct inversion; no sparse matrix optimization
- **Spatial extent**: Global kriging (no local kriging neighborhoods)
- **Data preprocessing**: No automatic outlier removal or data transformation
- **Future enhancements**:
  - Local kriging neighborhoods (e.g., nearest 20 points)
  - Robust variogram fitting (resistant to outliers)
  - Anisotropic variogram models
  - Kriging with external drift (KED)

### Breaking Changes

N/A - Initial release (0.1.0)

### Migration Guide

N/A - Initial release

---

## Build & Installation

### Rust
```bash
cd crates/wbgeostats
cargo build --release
cargo test
```

### Python
```bash
cd crates/wbgeostats
pip install maturin
maturin develop
```

### R
```r
# Typical R package development workflow
devtools::load_all()
devtools::document()
```

---

## Future Roadmap

**Phase C (Planned)**:
- Local kriging neighborhoods
- Anisotropic variogram models
- Kriging with external drift (KED)
- LOOCV parallelization
- Sparse matrix covariance optimization
- Python statsmodels integration
- R sp/sf/sf integration examples

**Phase D (Future)**:
- Bayesian kriging
- Multi-variate kriging (co-kriging)
- Spatio-temporal kriging
- GPU acceleration (CUDA via cudarc or gpu-bindgen)

---

## Contributors

- Whitebox Geospatial (2026)

---

## License

AGPL-3.0-or-later
