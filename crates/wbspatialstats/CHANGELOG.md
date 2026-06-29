# Changelog - wbspatialstats

All notable changes to the `wbspatialstats` project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added explicit field parameter schemas to spatial autocorrelation and kriging tools for QGIS field dropdown widget support:
  - **Spatial Autocorrelation**: `morans_i`, `local_morans_i`, `getis_ord_gi_star`, `nearest_neighbour_index` — field parameter with parent reference to input layer.
  - **Kriging Interpolation**: `ordinary_kriging`, `local_kriging`, `simple_kriging`, `universal_kriging`, `space_time_kriging` — field parameter (z_field) with parent reference to points layer.
  - **CoKriging**: `ordinary_cokriging` — primary_field and covariate_field parameters with parent references to points layer.
- Field schemas enable downstream front-ends (QGIS, R, Python) to automatically render field parameters as dropdown selectors with parent layer resolution.

### Fixed
- **Critical**: Fixed catastrophic memory issue in `EmpiricalVariogramBuilder::compute_lag_histogram()` that caused out-of-memory crashes on large point datasets (e.g., 13M+ points).
  - Changed algorithm from storing all O(n²) pairwise differences in memory to binning on-the-fly into lag maps.
  - Memory footprint reduced from O(n²) pairs (1.35 TB for 13M points) to O(num_lags) bins (~50-100 MB).
  - Maintains mathematically identical statistical output.
  - Resolves issue where Estimate Variogram tool would exhaust system RAM on realistic geospatial datasets.
  - Consistent with existing memory-efficient patterns in directional and cross-variogram implementations.

## [0.1.0] - 2026-06-04

Initial release of `wbspatialstats` as a unified spatial statistics library for Whitebox Geospatial.

### Added

#### Kriging & Interpolation
- **Ordinary Kriging**: Standard kriging interpolation with optional anisotropy support via directional distance metrics
- **Local Kriging**: Neighborhood-based kriging for large datasets with locality radius control
- **Simple Kriging**: Kriging with known (fixed) mean for specialized applications
- **Universal Kriging**: Trend-fitting kriging with polynomial detrending (degrees 0-2)
- **Space-Time Kriging**: 3D spatiotemporal kriging for dynamic phenomena with temporal decay
- **Ordinary CoKriging**: Multivariate kriging leveraging auxiliary variables via cross-variograms
- **Bootstrap Prediction Intervals**: Uncertainty quantification via bootstrap resampling (95% CI)

#### Variography
- **Empirical Variogram**: Direct semivariogram estimation from point samples with lag binning
- **Directional Variography**: Anisotropy detection via directional binning (0-180° azimuths with tolerance, distance-lag analysis)
- **Anisotropy Modeling**: Estimation of anisotropy ratio/direction and automatic distance transformation
- **Variogram Model Fitting**: Robust WLSQ fitting for exponential, Gaussian, linear, power, and spherical models
- **Cross-Variogram Computation**: Empirical cross-variograms for multivariate kriging (CoKriging)
- **Rose Diagram Visualization**: SVG output showing directional spatial dependence patterns

#### Spatial Autocorrelation
- **Global Moran's I**: Asymptotic inference with permutation-based significance testing
- **Local Moran's I (LISA)**: Local cluster/outlier detection with significance maps
- **Getis-Ord Gi\***: Hot/cold spot analysis with significance bounds
- **Ripley's K**: Point pattern intensity analysis with envelope testing (analytic and Monte Carlo)
- **Envelope Testing**: Analytic and bootstrap-based envelopes for point process validation
- **Quadrat Count Tests**: Regular lattice-based spatial aggregation with chi-square testing
- **Nearest Neighbour Index**: Clark-Evans clustering measurement with significance
- **Inhomogeneous Point Analysis**: Intensity estimation for non-stationary patterns

#### Spatial Regression
- **Spatial Lag Model (SAR)**: Spatial autoregressive modeling with maximum likelihood estimation and diagnostics
- **Spatial Error Model (SEM)**: SEM with simultaneous lag and error components
- **Geographically Weighted Regression (GWR)**: Local regression with Gaussian/tricube/bisquare kernels and AICc bandwidth optimization
- **Spatial Weights Construction**: Manual edge lists and distance-based weight matrices with customizable kernels

#### Cross-Validation & Diagnostics
- **Leave-One-Out Cross-Validation (LOOCV)**: Robust model evaluation with prediction residuals
- **k-Fold Cross-Validation**: Stratified folds with random seed control
- **Residual Analysis**: Bias, RMSE, MAE, mean error, correlation metrics
- **Spatial Autocorrelation in Residuals**: Moran's I test for model adequacy detection

#### Statistical Testing & Multiple Comparison Correction
- **Asymptotic Hypothesis Testing**: Z-scores and p-values under normality assumption
- **Permutation-Based Inference**: Rank-based p-values with configurable realization counts (default 9999)
- **Multiple Testing Corrections**: Bonferroni, Benjamini–Hochberg (FDR-BH) adjustments
- **Confidence Intervals**: Asymptotic (normal approx) and bootstrap-based (percentile) CI

### Architecture

- **Pure Rust Implementation**: No external C/C++ dependencies; full memory safety
- **Parallelization**: Rayon data parallelism for large datasets, permutation loops, batch kriging
- **Robust Numerics**: LU decomposition with partial pivoting; SVD fallback for singular systems; condition number awareness
- **Serialization**: Full serde support for variogram models, regression results, and intermediate computations
- **Performance**: Optimized for 1M+ point datasets via spatial indexing, chunked parallelism, and rayon work-stealing

### Known Limitations

- **Conditional Simulation**: Stochastic simulation not yet implemented (v0.2 candidate)
- **Indicator Kriging**: Probability kriging deferred to future release
- **Bayesian Methods**: CAR models and MCMC inference not included
- **Network Distances**: Euclidean distance only; network/graph-based analysis requires pre-computation
- **Spline Interpolation**: Radial basis functions and thin-plate splines not implemented

### Dependencies

- **nalgebra 0.34+**: Dense linear algebra with matrix decomposition
- **ndarray 0.15+**: N-dimensional array operations for spatial grids
- **rayon 1.7+**: Data parallelism and thread pool management
- **serde 1.0+**: Serialization for model persistence
- **thiserror 1.0+**: Ergonomic error handling
- **rand 0.8+**: Random number generation for permutation tests
- **log 0.4+**: Structured logging

### Removed

- Python bindings moved to wbw_python crate (maturin-based)
- R bindings moved to wbw_r crate (extendr-based)
- Frontend integration code removed; pure backend library focus

### License

AGPL-3.0-or-later
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
