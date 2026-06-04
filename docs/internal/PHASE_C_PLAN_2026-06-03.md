# Phase C Implementation Plan - wbgeostats Enhancements

**Date**: 2026-06-03  
**Status**: Planning  
**Scope**: 4 major enhancements to kriging library

---

## Overview

Phase C extends the 0.1.0 kriging foundation with four strategic enhancements:
1. **Local kriging neighbourhoods** (C-A): Scale to 100k+ point datasets
2. **Simple kriging** (C-B): Known mean variant
3. **Robust variogram fitting** (C-C): Outlier-resistant parameter estimation
4. **Spatio-temporal kriging** (C-D): Space-time domain interpolation

**Target**: Release 0.2.0 (Q3 2026) with all four features.

---

## Phase C-A: Local Kriging Neighbourhoods

### Objective
Enable kriging on large point clouds (100k+ points) by using only the nearest k neighbors for prediction, reducing O(n³) covariance matrix inversion to O(k³).

### Architecture Changes

#### New struct: `LocalOrdinaryKriging`
```rust
pub struct LocalOrdinaryKriging {
    coords: Vec<(f64, f64)>,
    values: Vec<f64>,
    vario: VariogramModel,
    spatial_index: KdTree,  // kdtree crate (lightweight spatial index)
    k: usize,               // number of nearest neighbors
}

impl LocalOrdinaryKriging {
    pub fn new(coords: Vec<(f64, f64)>, values: Vec<f64>, vario: VariogramModel, k: usize) -> Result<Self> {
        // Build spatial index for fast NN queries
        let spatial_index = KdTree::build_from_coordinates(&coords)?;
        Ok(Self { coords, values, vario, spatial_index, k })
    }
    
    pub fn predict(&self, (x, y): (f64, f64)) -> Result<KrigingResult> {
        // 1. Query k nearest neighbors
        let neighbors = self.spatial_index.nearest_n((x, y), self.k)?;
        
        // 2. Extract neighbor coords/values
        let neighbor_coords: Vec<_> = neighbors.iter().map(|i| self.coords[*i]).collect();
        let neighbor_values: Vec<_> = neighbors.iter().map(|i| self.values[*i]).collect();
        
        // 3. Run standard kriging on neighborhood
        let local_ok = OrdinaryKriging::new(neighbor_coords, neighbor_values, self.vario.clone())?;
        local_ok.predict((x, y))
    }
    
    pub fn predict_batch(&self, coords: &[(f64, f64)]) -> Result<Vec<KrigingResult>> {
        // Parallelized via rayon (same as before)
        coords.par_iter()
            .map(|&c| self.predict(c))
            .collect()
    }
}
```

#### Dependencies
- **kdtree** crate (lightweight, ~5KB, no unsafe, BSD-2-Clause)
- Keep pure Rust, no C++ bindings

#### Key Design Decisions
1. **Optional feature**: Feature-gate as `features = ["local_kriging"]` (default off)
2. **Reuse core solver**: Use existing `OrdinaryKriging` for each neighborhood
3. **k parameter**: Default k=20, configurable (tradeoff: speed vs. accuracy)
4. **Spatial index**: Build once at instantiation, reuse for all predictions

### Performance Impact
- Training points: 100k → O(k) space vs. O(n²) before
- Prediction time: O(log n) NN query + O(k³) kriging solver
- Example: 100k points, k=20 → **50-100x speedup** vs. global kriging

### API Changes (Non-breaking)
- New struct `LocalOrdinaryKriging` (separate from `OrdinaryKriging`)
- No changes to existing classes

### Testing
- Benchmark: Compare predictions (should match within tolerance for k>50)
- Test: k=10, k=20, k=50 on Meuse dataset (verify results similar)
- Stress test: 100k synthetic points, measure speed

### Implementation Timeline
- Estimate: 4-6 hours
- Files modified:
  - `crates/wbgeostats/Cargo.toml` (add kdtree)
  - `crates/wbgeostats/src/kriging/mod.rs` (new local.rs)
  - `crates/wbgeostats/src/kriging/local.rs` (LocalOrdinaryKriging impl)
  - `crates/wbgeostats/src/lib.rs` (export LocalOrdinaryKriging)
  - Python bindings: `src/python.rs` (new PyLocalOrdinaryKriging class)
  - R bindings: `src/r.rs` (kriging_predict_local, kriging_predict_grid_local)

---

## Phase C-B: Simple Kriging

### Objective
Support kriging with *known* mean (vs. unknown mean in Ordinary Kriging). Simpler equations, sometimes useful for detrended data or prior knowledge.

### Architecture Changes

#### New struct: `SimpleKriging`
```rust
pub struct SimpleKriging {
    coords: Vec<(f64, f64)>,
    values: Vec<f64>,
    vario: VariogramModel,
    mean: f64,  // Known/fixed mean
}

impl SimpleKriging {
    pub fn new(coords: Vec<(f64, f64)>, values: Vec<f64>, vario: VariogramModel, mean: f64) -> Result<Self> {
        Ok(Self { coords, values, vario, mean })
    }
    
    pub fn predict(&self, (x, y): (f64, f64)) -> Result<KrigingResult> {
        // Solve: γ w = γ₀  (no mean constraint)
        // Prediction: Z* = μ + γᵀ w
        // Standard error: σ² = γ₀₀ - γᵀ Γ⁻¹ γ  (same)
        // Implementation is ~90% code reuse from OrdinaryKriging
    }
}
```

#### Key Differences vs. Ordinary Kriging
| Aspect | Ordinary Kriging | Simple Kriging |
|---|---|---|
| Mean | Unknown (estimated) | Known (fixed parameter) |
| Covariance matrix size | (n+1) × (n+1) | n × n |
| Equations | System includes mean constraint | No mean constraint |
| Use case | Standard (most common) | Detrended data, prior knowledge |

#### API Changes (Non-breaking)
- New struct `SimpleKriging` (separate from `OrdinaryKriging`)
- Function: `simple_kriging_predict()` in Python/R modules

#### Testing
- Test: Provide mean=data.mean() to SK; compare to OK (should be close)
- Test: Provide different mean; verify predictions shift appropriately
- Synthetic data: Generate with known trend, remove trend, fit SK

### Implementation Timeline
- Estimate: 2-3 hours (code reuse from OrdinaryKriging)
- Files modified:
  - `crates/wbgeostats/src/kriging/mod.rs` (new simple.rs)
  - `crates/wbgeostats/src/kriging/simple.rs` (SimpleKriging impl)
  - Python/R bindings (similar pattern to OK)

---

## Phase C-C: Robust Variogram Fitting

### Objective
Fit variogram models resistant to outliers using L¹ (absolute deviation) or Huber loss instead of L² (squared error).

### Architecture Changes

#### New fitting method: `VariogramFitterRobust`
```rust
pub enum RobustLoss {
    L1,              // Absolute deviation (most robust)
    Huber(f64),      // Huber loss with threshold (balanced)
}

pub struct VariogramFitterRobust {
    loss_fn: RobustLoss,
    max_iterations: usize,
}

impl VariogramFitterRobust {
    pub fn fit(
        lags: &[LagBin],
        family: VariogramModelFamily,
        loss: RobustLoss,
    ) -> Result<VariogramModel> {
        // Minimize: Σ ρ(residuals) where ρ is L1/Huber
        // Uses gradient descent (non-linear optimization)
    }
}
```

#### Loss Functions
- **L¹** (least absolute deviations):
  ```
  ρ(r) = |r|
  ```
  Most robust, ignores outliers completely.

- **Huber** (smooth L1):
  ```
  ρ(r) = { 0.5*r² if |r| ≤ δ, δ*|r| - 0.5*δ² otherwise }
  ```
  Balanced: L² for small residuals, L¹ for large.

#### Dependencies
- No new crates needed (implement simple optimization loop)
- Reuse existing parameter optimization framework

#### API Changes (Non-breaking)
- Function: `fit_variogram_robust()` in Python/R modules
- Parameter: `loss_fn = RobustLoss::Huber(threshold)`

#### Testing
- Synthetic data with outliers: Fit standard & robust, compare parameters
- Test: L¹ should ignore extreme outliers; Huber should be intermediate
- Real data (Meuse): Inject artificial outliers, verify robust > standard

### Implementation Timeline
- Estimate: 3-4 hours
- Files modified:
  - `crates/wbgeostats/src/variogram/mod.rs` (new robust.rs)
  - `crates/wbgeostats/src/variogram/robust.rs` (impl)
  - Python/R bindings (add `fit_variogram_robust()`)

---

## Phase C-D: Spatio-Temporal Kriging

### Objective
Extend kriging to space-time domain: data has (x, y, t) coordinates, variogram is separable σ²(h_space, h_time).

### Architecture Changes

#### New data type: `SpatioTemporalData`
```rust
pub struct SpatioTemporalData {
    coords_3d: Vec<(f64, f64, f64)>,  // (x, y, t)
    values: Vec<f64>,
}

pub enum VariogramProductModel {
    // Separable: γ(h,τ) = γ_space(h) * γ_time(τ)
    Separable {
        space_model: VariogramModel,
        time_model: VariogramModel,
    },
    // Metric (space-time distance): γ(h,τ) = γ(√(h² + (α*τ)²))
    Metric {
        model: VariogramModel,
        space_time_ratio: f64,  // α scaling factor
    },
}

pub struct SpatioTemporalKriging {
    data: SpatioTemporalData,
    vario: VariogramProductModel,
}

impl SpatioTemporalKriging {
    pub fn predict(&self, (x, y, t): (f64, f64, f64)) -> Result<KrigingResult> {
        // Compute covariance matrix using 3D distances
        // Solve kriging system in 3D
    }
}
```

#### Key Design
1. **Separable model** (easiest, most common):
   - γ(h,τ) = γ_space(h) × γ_time(τ)
   - Assumes space and time effects are independent
   
2. **Metric model** (simpler alternative):
   - Treat space-time distance as √(h² + α²τ²)
   - Single variogram model, 1 extra parameter (α)

#### API Changes (Non-breaking)
- New struct `SpatioTemporalKriging`
- New struct `VariogramProductModel`
- Functions: `spacetime_predict()`, `spacetime_predict_grid()` in Python/R

#### Testing
- Synthetic data: Generate with known space+time structure
- Test: Predict at (x, y, t_future), verify temporal extrapolation
- Real example: Temperature time series at multiple stations
- Validate: Compare to temporal-only + spatial-only baseline

### Implementation Timeline
- Estimate: 4-5 hours
- Files modified:
  - `crates/wbgeostats/src/variogram/mod.rs` (extend model types)
  - `crates/wbgeostats/src/kriging/mod.rs` (new spacetime.rs)
  - `crates/wbgeostats/src/kriging/spacetime.rs` (impl)
  - Python/R bindings (new functions)

---

## Phased Rollout Strategy

### Timeline
| Phase | Duration | Features | Target Release |
|---|---|---|---|
| C-A | 1 week | Local kriging | 0.2.0-alpha |
| C-B | 3-4 days | Simple kriging | 0.2.0-beta |
| C-C | 3-4 days | Robust fitting | 0.2.0-rc |
| C-D | 1 week | Spatio-temporal | 0.2.0 final |
| **Total** | **~4 weeks** | **4 features** | **Q3 2026** |

### Commit Strategy (Per User Preference)
- **C-A**: 3 commits (foundation, tests, Python/R bindings)
- **C-B**: 2 commits (implementation, bindings)
- **C-C**: 2 commits (implementation, bindings)
- **C-D**: 3 commits (implementation, tests, bindings)
- **Final**: 1 commit (documentation update + CHANGELOG)

### Breaking Changes
**None**. All new features are:
- New structs (don't modify existing `OrdinaryKriging`)
- New functions (additive to API)
- Feature-gated where appropriate (local kriging)

### Documentation Updates
- CHANGELOG.md: Add 0.2.0 feature section
- API.md: Document new classes + functions
- EXAMPLES.md: Add examples for each new feature
- PERFORMANCE.md: Benchmark local kriging vs. global

---

## Dependency Review

| Feature | Crate | Version | Size | License | Rationale |
|---|---|---|---|---|---|
| Local kriging | kdtree | 0.2 | ~5 KB | BSD-2 | Lightweight spatial index, pure Rust |
| All others | (existing) | — | — | — | No new dependencies |

**Total impact**: +1 lightweight crate for spatial indexing.

---

## Success Criteria

### C-A (Local Kriging)
- ✅ Predictions match global kriging within 2% for k>50
- ✅ 50-100x speedup on 100k point dataset
- ✅ All tests passing
- ✅ Python/R bindings functional

### C-B (Simple Kriging)
- ✅ SK with mean=data.mean() matches OK closely
- ✅ Known mean parameter works correctly
- ✅ Tests passing

### C-C (Robust Fitting)
- ✅ L¹ ignores outliers better than L²
- ✅ Huber is intermediate (as expected)
- ✅ Parameter estimates stable

### C-D (Spatio-Temporal)
- ✅ Temporal predictions extrapolate sensibly
- ✅ Space-time distance weighting works
- ✅ Separable model flexible

---

## Risk Mitigation

| Risk | Mitigation |
|---|---|
| KdTree performance | Benchmark on varying dataset sizes; choose k appropriately |
| Robust fitting convergence | Test convergence on synthetic data; set iteration limit |
| 3D kriging matrix singularity | Use SVD fallback (already in place) |
| Feature complexity | Implement in order; don't mix features in commits |

---

## Next Steps

1. **Review this plan** — Approve scope/timeline/dependencies
2. **Start Phase C-A** — Implement local kriging neighbourhoods
3. **Iterative testing** — Benchmark each feature before moving to next
4. **Final integration** — Ensure Python/R bindings work for all 4 features
5. **Release 0.2.0** — Publish to crates.io, PyPI, CRAN

---

## Questions for User

Before starting Phase C-A, please confirm:

1. ✅ **Scope approved?** All 4 features (local, simple, robust, spatio-temporal)?
2. ⏳ **Timeline acceptable?** ~4 weeks total, start with local kriging?
3. ✅ **Dependencies okay?** Add `kdtree` crate for spatial indexing?
4. ✅ **Release target?** Aim for 0.2.0 (Q3 2026)?
5. ⏳ **Priority order?** Start with C-A (local kriging) for max impact?

Ready to begin Phase C-A once you confirm! 🚀
