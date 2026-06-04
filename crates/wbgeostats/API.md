# wbgeostats API Reference

Complete API documentation for the production-grade geostatistics kriging library.

## Table of Contents
- [Python API](#python-api)
- [R API](#r-api)
- [Rust API](#rust-api)

---

## Python API

**Module**: `wbgeostats`  
**Build**: `maturin develop` in `crates/wbgeostats`

### Classes

#### `VariogramModel`

Represents a theoretical variogram model (nugget effect + spatial structure).

```python
class VariogramModel:
    def __init__(self, family: str, nugget: float, partial_sill: float, range: float)
```

**Parameters**:
- `family` (str): Model family. One of: `"spherical"`, `"exponential"`, `"gaussian"`
- `nugget` (float): Nugget effect (measurement error + microscale variation)
- `partial_sill` (float): Spatial variance component (semivariance at range)
- `range` (float): Distance at which spatial correlation becomes negligible

**Properties** (read-only):
- `family` (str): Model family name
- `nugget` (float): Nugget effect value
- `partial_sill` (float): Partial sill value
- `range` (float): Range parameter
- `total_sill` (float): Total sill = nugget + partial_sill
- `wrss` (float): Weighted residual sum of squares from fitting (if fitted)

**Example**:
```python
from wbgeostats import VariogramModel

# Create model with known parameters
vario = VariogramModel("spherical", nugget=0.1, partial_sill=1.5, range=50.0)
print(vario.total_sill)  # 1.6
print(vario)  # VariogramModel(family='spherical', nugget=0.1, psill=1.5, range=50.0)
```

---

#### `KrigingResult`

Result of a kriging prediction at a single location.

```python
class KrigingResult:
```

**Properties** (read-only):
- `prediction` (float): Predicted value
- `variance` (float): Kriging variance (uncertainty)
- `std_error` (float): Standard error = sqrt(variance)
- `ci_lower` (float): 95% confidence interval lower bound
- `ci_upper` (float): 95% confidence interval upper bound

**Example**:
```python
result = ok.predict(100.0, 200.0)
print(f"Predicted: {result.prediction:.2f} ± {result.std_error:.2f}")
print(f"95% CI: [{result.ci_lower:.2f}, {result.ci_upper:.2f}]")
```

---

#### `OrdinaryKriging`

Ordinary kriging model fitted to training data.

```python
class OrdinaryKriging:
    def __init__(self, coords: List[Tuple[float, float]], values: List[float], vario: VariogramModel)
```

**Parameters**:
- `coords`: List of (x, y) coordinate tuples
- `values`: List of observed values (same length as coords)
- `vario`: VariogramModel instance

**Raises**:
- `ValueError`: If coordinate/value arrays have different lengths or < 3 points

**Methods**:
- `predict(x: float, y: float) -> KrigingResult`: Single-point prediction
- `predict_batch(coords: List[Tuple[float, float]]) -> List[KrigingResult]`: Vectorized predictions

**Example**:
```python
from wbgeostats import OrdinaryKriging, VariogramModel

# Create kriging model
coords = [(0, 0), (10, 10), (20, 5), (15, 20)]
values = [1.2, 3.5, 2.8, 4.1]
vario = VariogramModel("exponential", nugget=0.2, partial_sill=2.0, range=15.0)

ok = OrdinaryKriging(coords, values, vario)

# Single prediction
result = ok.predict(12.5, 12.5)
print(result)  # KrigingResult(pred=3.2, var=0.5, ci=[2.6, 3.8])

# Grid predictions (parallelized)
grid_coords = [(x, y) for x in range(0, 21, 5) for y in range(0, 21, 5)]
results = ok.predict_batch(grid_coords)
print(f"Predicted {len(results)} grid points")
```

---

### Functions

#### `estimate_variogram()`

Estimate empirical variogram from point data.

```python
def estimate_variogram(
    x: List[float],
    y: List[float],
    values: List[float],
    lag_distance: Optional[float] = None,
    max_lag_count: Optional[int] = None
) -> dict
```

**Parameters**:
- `x`, `y`: Coordinate arrays
- `values`: Observed values
- `lag_distance`: Bin size for lag distances (auto-computed if None)
- `max_lag_count`: Maximum number of lag bins (default: 15)

**Returns**: Dictionary with keys:
- `"distances"` (List[float]): Lag bin distances
- `"semivariances"` (List[float]): Semivariance at each lag
- `"pair_counts"` (List[int]): Number of pairs in each bin
- `"max_lag"` (float): Maximum lag distance analyzed

**Example**:
```python
vario_dict = estimate_variogram(x_coords, y_coords, values, lag_distance=10.0)
print(f"Analyzed {len(vario_dict['distances'])} lag bins")
print(f"Max lag: {vario_dict['max_lag']}")
```

---

#### `fit_variogram()`

Fit theoretical variogram model to empirical data.

```python
def fit_variogram(
    distances: List[float],
    semivariances: List[float],
    pair_counts: List[int],
    model_family: str
) -> VariogramModel
```

**Parameters**:
- `distances`, `semivariances`, `pair_counts`: From `estimate_variogram()` output
- `model_family`: One of `"spherical"`, `"exponential"`, `"gaussian"`

**Returns**: Fitted VariogramModel

**Raises**:
- `ValueError`: If array lengths don't match or < 3 lags
- `RuntimeError`: If fitting optimization fails

**Example**:
```python
vario_dict = estimate_variogram(x, y, values)
fitted_vario = fit_variogram(
    vario_dict["distances"],
    vario_dict["semivariances"],
    vario_dict["pair_counts"],
    "spherical"
)
print(f"Fitted: nugget={fitted_vario.nugget}, range={fitted_vario.range}")
```

---

#### `cross_validate_kriging()`

Leave-One-Out Cross-Validation (LOOCV) for model assessment.

```python
def cross_validate_kriging(
    x: List[float],
    y: List[float],
    values: List[float],
    vario: VariogramModel
) -> dict
```

**Parameters**:
- `x`, `y`, `values`: Training data
- `vario`: VariogramModel to evaluate

**Returns**: Dictionary with CV metrics:
- `"mean_error"` (float): Mean prediction error (should be ≈ 0)
- `"rmse"` (float): Root mean squared error
- `"rmsse"` (float): Root mean squared standardized error (should be ≈ 1 for well-calibrated models)
- `"correlation"` (float): Pearson correlation between predictions and actuals
- `"sample_size"` (int): Number of validation points
- `"is_well_calibrated"` (bool): True if ME_std < 0.1 and 0.8 < RMSSE < 1.2

**Example**:
```python
metrics = cross_validate_kriging(x, y, values, fitted_vario)
print(f"RMSE: {metrics['rmse']:.3f}")
print(f"Well-calibrated: {metrics['is_well_calibrated']}")
```

---

## R API

**Package**: `wbgeostats` (via extendr)  
**Build**: Standard R package workflow

### Functions

All R functions return lists with named components.

#### `estimate_variogram()`

```r
estimate_variogram(
  x, y, values,
  lag_distance = NULL,
  max_lag_count = NULL
)
```

**Returns**: List with:
- `distances`: Lag distances vector
- `semivariances`: Semivariance vector
- `pair_counts`: Pair count vector
- `max_lag`: Maximum lag distance

**Example**:
```r
vario <- estimate_variogram(x, y, values, lag_distance = 10)
plot(vario$distances, vario$semivariances, 
     main = "Empirical Variogram")
```

---

#### `fit_variogram()`

```r
fit_variogram(
  distances, semivariances, pair_counts,
  model_family
)
```

**Parameters**:
- `model_family`: One of `"spherical"`, `"exponential"`, `"gaussian"`

**Returns**: List with:
- `family`: Model family string
- `nugget`: Nugget effect
- `partial_sill`: Partial sill
- `range`: Range parameter
- `total_sill`: Total sill
- `wrss`: Weighted residual sum of squares

**Example**:
```r
vario_empirical <- estimate_variogram(x, y, values)
vario_fit <- fit_variogram(
  vario_empirical$distances,
  vario_empirical$semivariances,
  vario_empirical$pair_counts,
  "spherical"
)
str(vario_fit)
```

---

#### `kriging_predict()`

Single-point kriging prediction.

```r
kriging_predict(
  train_x, train_y, train_values,
  pred_x, pred_y,
  family, nugget, psill, range
)
```

**Parameters**:
- `train_x`, `train_y`, `train_values`: Training data
- `pred_x`, `pred_y`: Prediction location
- `family`: Variogram model family
- `nugget`, `psill`, `range`: Variogram parameters

**Returns**: List with:
- `prediction`: Predicted value
- `variance`: Kriging variance
- `std_error`: Standard error
- `ci_lower`, `ci_upper`: 95% confidence interval bounds

**Example**:
```r
result <- kriging_predict(
  train_x, train_y, train_values,
  pred_x = 100, pred_y = 200,
  family = "exponential",
  nugget = 0.1, psill = 1.5, range = 50
)
```

---

#### `kriging_predict_grid()`

Vectorized grid prediction (parallelized via rayon).

```r
kriging_predict_grid(
  train_x, train_y, train_values,
  pred_x, pred_y,
  family, nugget, psill, range
)
```

**Parameters**: Same as `kriging_predict()`, but:
- `pred_x`, `pred_y`: Vectors of prediction locations

**Returns**: List of vectors:
- `prediction`: Predicted values vector
- `variance`: Kriging variance vector
- `std_error`: Standard error vector
- `ci_lower`, `ci_upper`: CI bound vectors

**Example**:
```r
# Create grid
grid <- expand.grid(
  x = seq(0, 100, by=10),
  y = seq(0, 100, by=10)
)

results <- kriging_predict_grid(
  train_x, train_y, train_values,
  pred_x = grid$x, pred_y = grid$y,
  family = "spherical",
  nugget = 0.1, psill = 1.5, range = 50
)

# Combine with grid
grid$pred <- results$prediction
grid$std_err <- results$std_error
```

---

#### `kriging_cross_validate()`

Leave-One-Out Cross-Validation.

```r
kriging_cross_validate(
  x, y, values,
  family, nugget, psill, range
)
```

**Returns**: List with CV metrics:
- `mean_error`: Mean prediction error
- `rmse`: Root mean squared error
- `rmsse`: Root mean squared standardized error
- `correlation`: Pearson correlation
- `sample_size`: Number of points
- `is_well_calibrated`: Logical

**Example**:
```r
cv_metrics <- kriging_cross_validate(
  x, y, values,
  family = "gaussian",
  nugget = 0.15, psill = 1.8, range = 45
)
print(cv_metrics$rmsse)
```

---

## Rust API

**Crate**: `wbgeostats`  
**Features**: 
- Default: Pure Rust library (no I/O)
- `python`: PyO3 bindings
- `r`: extendr bindings

### Core Types

#### `VariogramModel`

```rust
pub struct VariogramModel {
    pub family: VariogramModelFamily,
    pub nugget: f64,
    pub partial_sill: f64,
    pub range: f64,
    pub wrss: f64,
    pub condition_number: f64,
}
```

**Families** (enum `VariogramModelFamily`):
- `Spherical`
- `Exponential`
- `Gaussian`

#### `OrdinaryKriging`

```rust
pub struct OrdinaryKriging {
    pub training_coords: Vec<(f64, f64)>,
    pub training_values: Vec<f64>,
    pub variogram: VariogramModel,
    // ... internal matrices ...
}

impl OrdinaryKriging {
    pub fn new(
        coords: Vec<(f64, f64)>,
        values: Vec<f64>,
        variogram: VariogramModel,
    ) -> Result<Self> { ... }
    
    pub fn predict(&self, location: (f64, f64)) -> Result<KrigingResult> { ... }
    
    pub fn predict_batch(&self, locations: &[(f64, f64)]) -> Result<Vec<KrigingResult>> { ... }
}
```

**Features**:
- Cholesky decomposition with SVD fallback for ill-conditioned matrices
- Parallelized batch prediction via `rayon`
- Handles singular/near-singular covariance matrices gracefully

#### `KrigingResult`

```rust
pub struct KrigingResult {
    pub prediction: f64,
    pub variance: f64,
    pub std_error: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
}
```

---

### Tool Integration (wbtools_oss)

The `OrdinaryKrigingTool` in `wbtools_oss` provides:
- Template raster-based interpolation
- Parallel grid prediction (rayon)
- Raster output via wbraster crate
- Progress reporting

**Tool Parameters**:
- `training_points`: Input point shapefile
- `field`: Attribute field with values
- `template_raster`: Raster template for grid definition
- `variogram_json`: Variogram model as JSON
- `output`: Output raster path

**Example Rust usage**:
```rust
use wbgeostats::kriging::OrdinaryKriging;
use wbgeostats::variogram::VariogramModel;

let coords = vec![(0.0, 0.0), (10.0, 10.0), (20.0, 5.0)];
let values = vec![1.2, 3.5, 2.8];

let vario = VariogramModel {
    family: VariogramModelFamily::Spherical,
    nugget: 0.1,
    partial_sill: 1.5,
    range: 50.0,
    wrss: 0.0,
    condition_number: 1.0,
};

let ok = OrdinaryKriging::new(coords, values, vario)?;

// Single prediction
let result = ok.predict((12.5, 12.5))?;

// Batch predictions (parallelized)
let grid: Vec<_> = (0..21)
    .flat_map(|x| (0..21).map(move |y| (x as f64, y as f64)))
    .collect();
let results = ok.predict_batch(&grid)?;
```

---

## Error Handling

### Python

```python
from wbgeostats import OrdinaryKriging, VariogramModel

try:
    ok = OrdinaryKriging(coords, values, vario)
except ValueError as e:
    print(f"Invalid input: {e}")  # e.g., "At least 3 training points required"
except RuntimeError as e:
    print(f"Computation failed: {e}")  # e.g., singular matrix
```

### R

```r
tryCatch(
  {
    result <- kriging_predict(...)
  },
  error = function(e) {
    cat("Error:", conditionMessage(e), "\n")
  }
)
```

### Rust

```rust
match ok.predict((x, y)) {
    Ok(result) => println!("Prediction: {}", result.prediction),
    Err(e) => eprintln!("Prediction failed: {}", e),
}
```

---

## Performance Notes

- **Grid prediction**: Parallelized via rayon across all grid points
- **Batch prediction**: 100-1000 points: 10-100ms on typical hardware
- **LOOCV**: O(n²) complexity; 100 points ≈ 1-5 seconds
- **Variogram fitting**: Serial optimization; typically < 1 second

See [PERFORMANCE.md](./PERFORMANCE.md) for benchmarks.
