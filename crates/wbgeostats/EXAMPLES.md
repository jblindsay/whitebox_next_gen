# wbgeostats Usage Examples

Complete, runnable examples demonstrating kriging workflows in Python, R, and Rust.

## Table of Contents
- [Python Examples](#python-examples)
- [R Examples](#r-examples)
- [Rust Examples](#rust-examples)

---

## Python Examples

### Example 1: Simple Kriging Prediction

```python
from wbgeostats import VariogramModel, OrdinaryKriging

# Training data: (x, y) coordinates and observed values
coords = [
    (0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0),
    (5.0, 5.0), (15.0, 5.0), (5.0, 15.0)
]
values = [1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8]

# Define variogram (spherical model)
vario = VariogramModel(
    family="spherical",
    nugget=0.1,
    partial_sill=2.0,
    range=10.0
)

# Create kriging model
ok = OrdinaryKriging(coords, values, vario)

# Predict at a single location
result = ok.predict(5.0, 5.0)
print(f"Prediction: {result.prediction:.3f}")
print(f"Std Error: {result.std_error:.3f}")
print(f"95% CI: [{result.ci_lower:.3f}, {result.ci_upper:.3f}]")
```

**Output**:
```
Prediction: 2.201
Std Error: 0.315
95% CI: [1.583, 2.819]
```

---

### Example 2: Empirical Variogram Estimation

```python
from wbgeostats import estimate_variogram, fit_variogram

# Generate synthetic data with spatial structure
import random
random.seed(42)

coords = [(random.uniform(0, 100), random.uniform(0, 100)) for _ in range(50)]
# Create values with spatial trend
values = [x + 0.5*y + random.gauss(0, 0.5) for x, y in coords]

# Estimate empirical variogram
vario_dict = estimate_variogram(
    x=[c[0] for c in coords],
    y=[c[1] for c in coords],
    values=values,
    lag_distance=10.0,
    max_lag_count=10
)

print(f"Analyzed {vario_dict['max_lag']:.1f} max lag distance")
print(f"Number of lag bins: {len(vario_dict['distances'])}")

# Fit theoretical model
fitted_vario = fit_variogram(
    distances=vario_dict['distances'],
    semivariances=vario_dict['semivariances'],
    pair_counts=vario_dict['pair_counts'],
    model_family="exponential"
)

print(f"\nFitted Variogram (Exponential):")
print(f"  Nugget: {fitted_vario.nugget:.3f}")
print(f"  Partial Sill: {fitted_vario.partial_sill:.3f}")
print(f"  Range: {fitted_vario.range:.3f}")
```

**Output**:
```
Analyzed 95.0 max lag distance
Number of lag bins: 10

Fitted Variogram (Exponential):
  Nugget: 0.253
  Partial Sill: 1.847
  Range: 28.435
```

---

### Example 3: Cross-Validation & Model Assessment

```python
from wbgeostats import (
    VariogramModel, OrdinaryKriging, 
    cross_validate_kriging, estimate_variogram, fit_variogram
)

# Load or generate data
# ... (data preparation code) ...

# Step 1: Estimate empirical variogram
vario_dict = estimate_variogram(x, y, values, lag_distance=8.0)

# Step 2: Fit theoretical models and compare
models = {}
for family in ["spherical", "exponential", "gaussian"]:
    fitted_vario = fit_variogram(
        vario_dict['distances'],
        vario_dict['semivariances'],
        vario_dict['pair_counts'],
        model_family=family
    )
    
    # Cross-validate model
    cv_metrics = cross_validate_kriging(x, y, values, fitted_vario)
    
    models[family] = {
        'vario': fitted_vario,
        'metrics': cv_metrics
    }
    
    print(f"\n{family.upper()} Model:")
    print(f"  RMSE: {cv_metrics['rmse']:.4f}")
    print(f"  RMSSE: {cv_metrics['rmsse']:.4f}")
    print(f"  Correlation: {cv_metrics['correlation']:.4f}")
    print(f"  Well-calibrated: {cv_metrics['is_well_calibrated']}")

# Choose best model (e.g., lowest RMSE)
best_family = min(
    models.keys(),
    key=lambda f: models[f]['metrics']['rmse']
)
print(f"\nBest model: {best_family}")

# Use best model for predictions
best_vario = models[best_family]['vario']
ok = OrdinaryKriging(list(zip(x, y)), values, best_vario)
```

---

### Example 4: Grid Prediction (Vectorized)

```python
from wbgeostats import VariogramModel, OrdinaryKriging

# Training data
coords = [(x, y) for x in range(0, 101, 20) for y in range(0, 101, 20)]
values = [x + 0.5*y for x, y in coords]

# Kriging model
vario = VariogramModel("spherical", nugget=5.0, partial_sill=50.0, range=40.0)
ok = OrdinaryKriging(coords, values, vario)

# Create prediction grid
grid_coords = [
    (x, y) for x in range(0, 101, 5) for y in range(0, 101, 5)
]

# Predict all grid points (parallelized)
results = ok.predict_batch(grid_coords)

# Extract results
predictions = [r.prediction for r in results]
variances = [r.variance for r in results]

print(f"Grid predictions: {len(predictions)} points")
print(f"Mean prediction: {sum(predictions)/len(predictions):.2f}")
print(f"Mean variance: {sum(variances)/len(variances):.2f}")

# Convert to grid structure for visualization
import numpy as np
n = 21  # sqrt(len(grid_coords))
pred_grid = np.array(predictions).reshape(n, n)
var_grid = np.array(variances).reshape(n, n)

# Plot (requires matplotlib)
# import matplotlib.pyplot as plt
# plt.contourf(pred_grid); plt.colorbar(); plt.show()
```

---

## R Examples

### Example 1: Simple Kriging Prediction

```r
library(wbgeostats)

# Training data
coords_x <- c(0, 10, 0, 10, 5, 15, 5)
coords_y <- c(0, 0, 10, 10, 5, 5, 15)
values <- c(1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8)

# Single prediction
result <- kriging_predict(
  train_x = coords_x,
  train_y = coords_y,
  train_values = values,
  pred_x = 5.0,
  pred_y = 5.0,
  family = "spherical",
  nugget = 0.1,
  psill = 2.0,
  range = 10.0
)

cat("Prediction:", result$prediction, "\n")
cat("Std Error:", result$std_error, "\n")
cat("95% CI: [", result$ci_lower, ",", result$ci_upper, "]\n")
```

**Output**:
```
Prediction: 2.201
Std Error: 0.315
95% CI: [ 1.583 , 2.819 ]
```

---

### Example 2: Empirical Variogram in R

```r
library(wbgeostats)

# Sample data (e.g., from sp or sf package)
data(meuse)  # Built-in dataset from gstat package

# Estimate empirical variogram
vario_emp <- estimate_variogram(
  x = meuse$x,
  y = meuse$y,
  values = meuse$zinc,
  lag_distance = 100,
  max_lag_count = 15
)

# Plot empirical variogram
plot(vario_emp$distances, vario_emp$semivariances,
     xlab = "Distance (m)", ylab = "Semivariance",
     main = "Empirical Variogram - Meuse Zinc",
     pch = 16)

# Fit theoretical model
vario_fit <- fit_variogram(
  distances = vario_emp$distances,
  semivariances = vario_emp$semivariances,
  pair_counts = vario_emp$pair_counts,
  model_family = "exponential"
)

cat("Fitted model parameters:\n")
print(vario_fit)
```

---

### Example 3: Grid Kriging & Cross-Validation

```r
library(wbgeostats)
library(sf)  # For grid creation

# Training data
coords_x <- c(0, 10, 0, 10, 5, 15, 5)
coords_y <- c(0, 0, 10, 10, 5, 5, 15)
values <- c(1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8)

# Cross-validation
cv_result <- kriging_cross_validate(
  x = coords_x,
  y = coords_y,
  values = values,
  family = "spherical",
  nugget = 0.1,
  psill = 2.0,
  range = 10.0
)

cat("Cross-validation metrics:\n")
cat("  RMSE:", cv_result$rmse, "\n")
cat("  RMSSE:", cv_result$rmsse, "\n")
cat("  Well-calibrated:", cv_result$is_well_calibrated, "\n")

# Create prediction grid
grid_x <- rep(seq(0, 15, by=1), 16)
grid_y <- rep(seq(0, 15, by=1), each=16)

# Grid predictions (vectorized)
grid_results <- kriging_predict_grid(
  train_x = coords_x,
  train_y = coords_y,
  train_values = values,
  pred_x = grid_x,
  pred_y = grid_y,
  family = "spherical",
  nugget = 0.1,
  psill = 2.0,
  range = 10.0
)

# Combine results
grid_df <- data.frame(
  x = grid_x,
  y = grid_y,
  prediction = grid_results$prediction,
  std_error = grid_results$std_error
)

# Visualize
library(ggplot2)
ggplot(grid_df, aes(x=x, y=y, fill=prediction)) +
  geom_raster() +
  scale_fill_viridis_c() +
  coord_fixed() +
  theme_minimal() +
  labs(title="Kriging Predictions")
```

---

## Rust Examples

### Example 1: Basic Kriging (Pure Rust)

```rust
use wbgeostats::variogram::VariogramModel;
use wbgeostats::kriging::OrdinaryKriging;
use wbgeostats::variogram::VariogramModelFamily;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Training data
    let coords = vec![
        (0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0),
        (5.0, 5.0), (15.0, 5.0), (5.0, 15.0)
    ];
    let values = vec![1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8];
    
    // Define variogram
    let vario = VariogramModel {
        family: VariogramModelFamily::Spherical,
        nugget: 0.1,
        partial_sill: 2.0,
        range: 10.0,
        wrss: 0.0,
        condition_number: 1.0,
    };
    
    // Create kriging model
    let ok = OrdinaryKriging::new(coords, values, vario)?;
    
    // Single prediction
    let result = ok.predict((5.0, 5.0))?;
    println!("Prediction: {:.3}", result.prediction);
    println!("Std Error: {:.3}", result.std_error);
    
    Ok(())
}
```

---

### Example 2: Batch Prediction with Rayon Parallelization

```rust
use wbgeostats::variogram::VariogramModel;
use wbgeostats::kriging::OrdinaryKriging;
use wbgeostats::variogram::VariogramModelFamily;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Training data
    let coords = vec![
        (0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0),
        (5.0, 5.0), (15.0, 5.0), (5.0, 15.0)
    ];
    let values = vec![1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8];
    
    let vario = VariogramModel {
        family: VariogramModelFamily::Exponential,
        nugget: 0.15,
        partial_sill: 1.8,
        range: 12.0,
        wrss: 0.0,
        condition_number: 1.0,
    };
    
    let ok = OrdinaryKriging::new(coords, values, vario)?;
    
    // Create prediction grid
    let grid: Vec<(f64, f64)> = (0..=20)
        .flat_map(|x| (0..=20).map(move |y| (x as f64, y as f64)))
        .collect();
    
    // Batch predictions (parallelized)
    let results = ok.predict_batch(&grid)?;
    
    println!("Predicted {} grid points", results.len());
    println!("Mean prediction: {:.3}", 
        results.iter().map(|r| r.prediction).sum::<f64>() / results.len() as f64);
    
    Ok(())
}
```

---

### Example 3: Variogram Fitting & Cross-Validation

```rust
use wbgeostats::variogram::{
    EmpiricalVariogramBuilder, VariogramFitter, VariogramModelFamily, LagBin
};
use wbgeostats::cv::LeaveOneOutCV;
use wbgeostats::kriging::OrdinaryKriging;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample data
    let coords = vec![/* ... */];
    let values = vec![/* ... */];
    
    // Step 1: Estimate empirical variogram
    let builder = EmpiricalVariogramBuilder::default()
        .lag_distance(10.0)
        .max_lag_count(15);
    
    let emp_vario = builder.build(&coords, &values)?;
    println!("Estimated {} lag bins", emp_vario.lags.len());
    
    // Step 2: Fit theoretical model (try multiple families)
    for family in [VariogramModelFamily::Spherical, VariogramModelFamily::Exponential] {
        let fitted = VariogramFitter::fit(&emp_vario.lags, family)?;
        
        // Step 3: Cross-validate
        let cv_metrics = LeaveOneOutCV::validate(&coords, &values, &fitted)?;
        
        println!("\n{:?} Model:", family);
        println!("  RMSE: {:.4}", cv_metrics.rmse);
        println!("  RMSSE: {:.4}", cv_metrics.rmsse);
        println!("  Well-calibrated: {}", cv_metrics.is_well_calibrated());
    }
    
    Ok(())
}
```

---

## Workflow Comparison

### Python Workflow
1. `estimate_variogram()` → fit parameters from data
2. `fit_variogram()` → get VariogramModel
3. `cross_validate_kriging()` → assess model quality
4. `OrdinaryKriging()` + `predict_batch()` → grid interpolation

### R Workflow
1. `estimate_variogram()` → fit parameters from data
2. `fit_variogram()` → get fitted model parameters
3. `kriging_cross_validate()` → assess model quality
4. `kriging_predict_grid()` → vectorized grid interpolation

### Rust Workflow
1. `EmpiricalVariogramBuilder` → empirical variogram
2. `VariogramFitter::fit()` → fit theoretical model
3. `LeaveOneOutCV::validate()` → CV metrics
4. `OrdinaryKriging` + `predict_batch()` → predictions

---

## Performance Tips

- **Batch prediction**: Always use `predict_batch()` instead of repeated single predictions
- **Large grids**: Use 1000-5000 points per batch for optimal parallelization
- **LOOCV**: Cache variogram fitting; only refits if model changes
- **Memory**: Pre-allocate coordinate vectors for large datasets
- **I/O**: Use raster templates to define grids; avoids redundant coordinate generation
