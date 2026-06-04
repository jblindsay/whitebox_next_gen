# wbspatialstats

Pure-Rust spatial statistics and geostatistics library for kriging, variography, spatial autocorrelation, spatial regression, and point-process analysis, designed to serve as the statistical engine for [Whitebox](https://www.whiteboxgeo.com).

## Table of Contents

- [Mission](#mission)
- [The Whitebox Project](#the-whitebox-project)
- [Is wbspatialstats Only for Whitebox?](#is-wbspatialstats-only-for-whitebox)
- [What wbspatialstats Is Not](#what-wbspatialstats-is-not)
- [Design Goals](#design-goals)
- [Current Capabilities](#current-capabilities)
  - [Kriging](#kriging)
  - [Variography](#variography)
  - [Spatial Autocorrelation](#spatial-autocorrelation)
  - [Spatial Regression](#spatial-regression)
  - [Point Process Analysis](#point-process-analysis)
  - [Cross-Validation & Diagnostics](#cross-validation--diagnostics)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Examples](#examples)
- [Known Limitations](#known-limitations)
- [Architecture & Dependencies](#architecture--dependencies)
- [Development](#development)
- [License](#license)

## Mission

- Provide robust spatial statistics and geostatistics capabilities for Whitebox applications and workflows.
- Implement kriging variants, variogram estimation/fitting, spatial autocorrelation measures, spatial regression, and point-process analysis in pure Rust.
- Support production-grade spatial inference with proper diagnostics, cross-validation, and uncertainty quantification.
- Prioritize numerical stability and performance while maintaining minimal external dependencies.

## The Whitebox Project

[Whitebox](https://www.whiteboxgeo.com) is a suite of open-source geospatial data analysis software with roots at the [University of Guelph](https://geg.uoguelph.ca), Canada, where [Dr. John Lindsay](https://jblindsay.github.io/ghrg/index.html) began the project in 2009. Over more than fifteen years it has grown into a widely used platform for geomorphometry, spatial hydrology, LiDAR processing, and remote sensing research. In 2021 Dr. Lindsay and Anthony Francioni founded [Whitebox Geospatial Inc.](https://www.whiteboxgeo.com) to ensure the project's long-term, sustainable development. **Whitebox Next Gen** is the current major iteration of that work, and this crate is part of that larger effort.

Whitebox Next Gen is a ground-up redesign that improves on its predecessor in nearly every dimension:

- **CRS & reprojection** — Full read/write of coordinate reference system metadata across raster, vector, and LiDAR data, with multiple resampling methods for raster reprojection.
- **Raster I/O** — More robust GeoTIFF handling (including Cloud-Optimized GeoTIFFs), plus newly supported formats such as GeoPackage Raster and JPEG2000.
- **Vector I/O** — Expanded from Esri Shapefile-only to 11 formats, including GeoPackage, FlatGeobuf, GeoParquet, and other modern interchange formats.
- **Vector topology** — A new, dedicated topology engine (`wbtopology`) enabling robust overlay, buffering, and related operations.
- **LiDAR I/O** — Full support for LAS 1.0–1.5, LAZ, COPC, E57, and PLY via `wblidar`, a high-performance, modern LiDAR I/O engine.
- **Spatial Statistics** — A comprehensive spatial statistics engine (`wbspatialstats`) with kriging, variography, autocorrelation, regression, and point-process tools.
- **Frontends** — Whitebox Workflows for Python (WbW-Python), Whitebox Workflows for R (WbW-R), and a QGIS 4-compliant plugin are in active development.

## Is wbspatialstats Only for Whitebox?

No. `wbspatialstats` is developed primarily to support Whitebox, but it is not restricted to Whitebox projects.

- **Whitebox-first**: API and roadmap decisions prioritize Whitebox geospatial analysis needs.
- **General-purpose**: the crate is usable as a standalone spatial statistics library in other Rust geospatial applications.
- **Production-ready**: proper error handling, numerical stability checks, and comprehensive test coverage make it suitable for production workflows.

## What wbspatialstats Is Not

`wbspatialstats` is a spatial statistics computation engine. It is **not** a full statistics framework.

- Not a format I/O library (file reading/writing belongs in `wbraster` and `wbvector`).
- Not a rendering or visualization engine.
- Not a coordinate reference system engine (CRS and projection belong in `wbprojection`).
- Not a full statistical modeling framework (Bayesian inference, advanced model selection, simulation methods are beyond current scope).
- Not a spatial database (point storage and management is the caller's responsibility).

## Design Goals

- Pure Rust only with no unsafe code in core algorithms.
- Keep dependencies minimal; only use `nalgebra` for linear algebra and `thiserror` for error handling.
- Prioritize numerical stability with appropriate scaling, regularization, and stability checks.
- Support both synchronous batch-mode computation and single-prediction workflows.
- Interoperability with `wbraster` and `wbvector` via coordinate/value tuples and standard Rust collections.

## Current Capabilities

### Kriging

Deterministic spatial interpolation with uncertainty quantification (kriging variance):

- **Ordinary Kriging**: Local prediction with mean estimation from data
- **Local Kriging**: Neighborhood-based kriging for large datasets
- **Simple Kriging**: Prediction with known, fixed mean
- **Universal Kriging**: Linear trend-surface integration (drift polynomial: constant, linear, quadratic)
- **Space-Time Kriging**: Spatio-temporal prediction with optional temporal autocorrelation weighting
- **Ordinary CoKriging**: Multivariate kriging leveraging auxiliary variables via cross-variograms for improved predictions

Features:
- Kriging variance estimation for uncertainty quantification
- Cross-covariance matrix computation for error analysis
- Neighborhood-size control for scalability
- Support for Gaussian and exponential variogram models
- Anisotropic kriging support via variogram anisotropy parameters
- CoKriging block-structured system matrix for multivariate prediction

### Variography

Empirical and theoretical variogram computation for spatial dependence characterization:

- **Empirical Variogram**: Lag-based semi-variance estimation with configurable bin sizes
- **Directional Variogram**: Azimuthal/anisotropic analysis (quantify directional dependence)
- **Cross-Variogram**: Spatial dependence between primary and auxiliary variables (required for CoKriging)
- **Robust Variogram Fitting**: Multiple robust loss functions (Cressie, Dowd, Genton)
- **Variogram Model Families**:
  - Exponential (isotropic and anisotropic)
  - Gaussian (isotropic and anisotropic)
  - Linear
  - Power
  - Spherical

Features:
- Cloud-based semi-variance diagnostics (pairwise analysis for outlier detection)
- Nested variogram support for complex spatial structures
- Automatic nugget effect estimation
- Cross-variogram fitting for multivariate kriging workflows

### Spatial Autocorrelation

Measures of spatial structure and clustering for exploratory analysis:

- **Global Moran's I**: Global spatial autocorrelation statistic with asymptotic significance testing
- **Local Moran's I (LISA)**: Local indicators of spatial association with significance maps
- **Getis-Ord Gi***: Local clustering/hotspot identification with asymptotic inference
- **Spatial Weights Construction**:
  - Inverse-distance weights (customizable decay exponent)
  - Queen/Rook contiguity (vector polygon neighbors)
  - K-nearest neighbor weights
  - Distance-band (fixed radius) weights

Features:
- Multiple testing correction (Bonferroni, FDR-Benjamini-Hochberg)
- Island policy configuration (how to handle isolated features)
- Standardized/row-normalized weight matrices
- Comprehensive diagnostics (weight matrix sparsity, neighbor counts)

### Spatial Regression

Spatial inference combining regression with spatial structure:

- **Spatial Lag (SAR)**: Include neighboring dependent variable values as predictor
- **Spatial Error (SEM)**: Model spatial autocorrelation in residuals
- **Geographically Weighted Regression (GWR)**: Local regression with distance-decay kernel weighting
  - Kernel functions: Gaussian, Biweight, Tricube
  - Adaptive (nearest-k) and fixed-radius bandwidth options
  - Local coefficient estimation with uncertainty quantification

Features:
- Ordinary least squares (OLS) baseline for comparison
- Lagged variable/residual spatial dependence
- Local goodness-of-fit diagnostics per regression window
- Bandwidth optimization via cross-validation

### Point Process Analysis

Statistical tools for point pattern analysis and hypothesis testing:

- **Ripley's K Function**: Second-order intensity analysis (clustering vs. dispersion)
- **Ripley's L Transform**: Centered version of K for easier interpretation
- **Envelope Testing**: Monte Carlo confidence envelopes for point patterns
  - Configurable null model (homogeneous Poisson or random shifting)
  - Adjustable simulation count for trade-off between precision and speed
- **Inhomogeneous Intensity**: Kernel density estimation for spatially varying rate processes
- **Quadrat Count Test**: Goodness-of-fit test for spatial randomness (Poisson vs. observed counts)

Features:
- Distance-based (isotropic) and directional K variants
- Bootstrap resampling for residual uncertainty
- Support for marked point patterns (optional attribute weighting)

### Cross-Validation & Diagnostics

Model validation and diagnostic tools for spatial model assessment:

- **Leave-One-Out Cross-Validation (LOOCV)**: Remove each observation, predict it, and assess error
- **Spatial k-Fold Cross-Validation**: Spatially stratified train/test splitting
- **Prediction Interval Calculation**: Confidence bounds from kriging variance
- **Residual Analysis**: Residuals, standardized residuals, and regression diagnostics
- **Prediction Intervals**: Gaussian-based confidence and prediction intervals
- **Bootstrap Confidence Intervals**: Resampling-based uncertainty quantification for kriging

Features:
- Root-mean-square error (RMSE) and mean-absolute-error (MAE) computation
- Quantile-quantile (Q-Q) plots for normality checking
- Spatial residual autocorrelation diagnostics

## Installation

Crates.io dependency:

```toml
[dependencies]
wbspatialstats = "0.1"
```

Local workspace/path dependency:

```toml
[dependencies]
wbspatialstats = { path = "../wbspatialstats" }
```

## Quick Start

### Ordinary Kriging Example

```rust
use wbspatialstats::kriging::OrdinaryKriging;
use wbspatialstats::variogram::{VariogramModel, VariogramModelFamily};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample data: (x, y) coordinates with z-values
    let primary_coords = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (1.0, 1.0),
    ];
    let primary_values = vec![10.0, 12.0, 11.0, 13.0];

    // Create variogram model
    let vgm = VariogramModel::exponential(
        1.0,    // range parameter
        1.0,    // sill (total variance)
        0.1,    // nugget
        "z",    // variable name
    );

    // Initialize kriging predictor
    let kriging = OrdinaryKriging::new(
        &vgm,
        &primary_coords,
        &primary_values,
        16,  // neighborhood size
    )?;

    // Predict at a new location
    let prediction = kriging.predict((0.5, 0.5))?;
    println!("Predicted value: {}", prediction.value);
    println!("Kriging variance: {}", prediction.variance);
    println!("Kriging std error: {}", prediction.variance.sqrt());

    Ok(())
}
```

### Empirical Variogram Example

```rust
use wbspatialstats::variogram::EmpiricalVariogramBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample data
    let coords = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (1.0, 1.0),
    ];
    let values = vec![10.0, 12.0, 11.0, 13.0];

    // Build empirical variogram
    let builder = EmpiricalVariogramBuilder::new(&coords, &values);
    let vgm = builder
        .with_max_distance(2.0)
        .with_bin_size(0.5)
        .build()?;

    println!("Empirical variogram: {} lags", vgm.lags.len());
    for lag in &vgm.lags {
        println!("  Distance: {:.2}, Semi-variance: {:.2}", lag.distance, lag.semi_variance);
    }

    Ok(())
}
```

### Spatial Autocorrelation Example

```rust
use wbspatialstats::autocorrelation::GlobalMoransI;
use wbspatialstats::weights::SpatialWeightsGraph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample data
    let values = vec![10.0, 12.0, 11.0, 13.0];
    
    // Create spatial weights (inverse distance)
    let weights = SpatialWeightsGraph::inverse_distance(
        &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
        1.0,  // exponent
    )?;

    // Compute Moran's I
    let morans_i = GlobalMoransI::new(&weights, &values)?;
    println!("Moran's I: {:.4}", morans_i.statistic);
    println!("Expected I (under null): {:.6}", morans_i.expected);
    println!("Variance: {:.8}", morans_i.variance);
    println!("p-value (one-tailed): {:.4}", morans_i.p_value);

    Ok(())
}
```

## Examples

See [examples/](examples/) directory for additional worked examples:

- `kriging_workflow.rs` — End-to-end kriging pipeline with variogram fitting
- `spatial_autocorrelation.rs` — Computing and interpreting Moran's I and LISA
- `gwr_local_regression.rs` — Geographically weighted regression with local coefficients
- `point_process.rs` — Ripley's K analysis and envelope testing

## Known Limitations

1. **Kriging**:
   - Ordinary kriging assumes stationarity; non-stationary processes may need trend removal first.
   - Large datasets (>5000 points) may require local kriging or neighborhood selection for performance.
   - Anisotropy support is via variogram model parameters; geometric anisotropy tensors are not yet supported.
   - CoKriging supports arbitrary number of auxiliary variables but limited to pairwise cross-variogram relationships.

2. **Variography**:
   - Nested (multi-structure) variograms are partially supported; some complex nesting patterns may require post-hoc model fitting.
   - Cross-variograms are supported but currently limited to primary-auxiliary variable pairs.

3. **Spatial Regression**:
   - Bayesian spatial models (MCMC, Gibbs sampling) are not yet implemented.
   - Spatial quantile regression is not yet implemented.
   - Robust M-estimation variants are not yet integrated into regression solvers.

4. **Point Process**:
   - Marked point patterns are supported but multitype/multivariate patterns have limited statistical support.
   - 3D/4D point patterns are not yet implemented.

5. **Numerical**:
   - Matrix condition number warnings are logged but not enforced; ill-conditioned systems may silently produce unreliable results.
   - No preconditioning support for very large matrices.

## Architecture & Dependencies

### Dependencies

- `nalgebra` — Linear algebra (LU decomposition, matrices, vectors) for kriging solvers and regression
- `thiserror` — Error type derivation and Display formatting
- Standard library only for core types and algorithms

### Module Structure

- `variogram/` — Empirical variogram, model families, fitting algorithms, cloud diagnostics
- `kriging/` — OrdinaryKriging, LocalOrdinaryKriging, SimpleKriging, UniversalKriging, SpaceTimeKriging, and result types
- `cv/` — Leave-one-out and spatial k-fold cross-validation
- `weights/` — Spatial weights matrices with multiple construction modes (inverse distance, queen, k-nearest)
- `autocorrelation/` — Global/local Moran's I, Getis-Ord Gi*, significance testing, multiple testing correction
- `regression/` — Spatial lag, spatial error, and geographically weighted regression solvers
- `density_estimation/` — Kernel density estimation for inhomogeneous intensity functions
- `point_process/` — Ripley's K, envelope testing, quadrat count tests

### Error Handling

`wbspatialstats::GeostatError` is the primary error type, covering:
- Invalid variograms (singular matrices, degenerate models)
- Kriging solve failures (numerical instability, rank deficiency)
- Invalid parameters (negative ranges, inappropriate coordinates)
- Numerical instability warnings

## Development

### Testing

Run the test suite:

```bash
cargo test -p wbspatialstats
```

Run with backtrace on failures:

```bash
RUST_BACKTRACE=1 cargo test -p wbspatialstats -- --nocapture
```

### Building Documentation

```bash
cargo doc -p wbspatialstats --open
```

### Benchmarking

Benchmarks (if present) can be run with:

```bash
cargo bench -p wbspatialstats
```

## License

`wbspatialstats` is licensed under the MIT license. See [LICENSE](../../LICENSE) for details.
