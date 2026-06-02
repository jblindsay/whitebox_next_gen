# Epoch-Aware Datum Transform Implementation Plan (wbprojection)

Date: 2026-06-01
Status: Draft implementation plan for CSRS/NATRF-style dynamic transformations
Owner: wbprojection

Companion execution checklist:
- `docs/internal/EPOCH_AWARE_IMPLEMENTATION_CHECKLIST.md`

## 1. Objective

Add epoch-aware, velocity-capable datum transformation support to `wbprojection` while preserving full backward compatibility for existing static transformation APIs.

Target capability includes:
- Coordinate-epoch-aware transforms for dynamic datums.
- Velocity grid support (base shift + rate model).
- Operation-aware routing for workflows similar to EPSG operation chains (e.g., CSRS realization transitions).
- Controlled rollout with strict regression coverage for existing users.

Non-goals for v1:
- Full global dynamic-datum coverage.
- Replacing existing static CRS lookups or legacy transform behavior.

## 2. Current State (Summary)

`wbprojection` already has strong foundations:
- CRS pipeline: source geodetic -> WGS84 -> target geodetic.
- Datum strategies include Helmert/Molodensky/GridShift/NTv2 hierarchy.
- NTv2 hierarchy loading and subgrid selection are implemented.
- Broad EPSG CRS support including NAD83(CSRS) realization code families.

Known limitation for modern dynamic workflows:
- No epoch parameter in transform APIs.
- Grid sampling is static (no velocity/time terms).
- No operation-code-driven transform selection layer.

## 3. Design Principles

1. Additive API strategy (non-breaking first).
2. Keep current methods as default static behavior.
3. Make epoch-aware behavior explicit and opt-in.
4. Separate concerns:
- CRS/Datum math
n- Grid model and loaders
- Operation selection metadata
5. Enforce deterministic behavior and traceability in tests.

## 4. Proposed Architecture Changes

### 4.1 Temporal Transform Context

Introduce a context object carried through transform operations.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformEpochContext {
    pub coordinate_epoch_decimal_year: f64,
    pub source_reference_epoch_decimal_year: Option<f64>,
    pub target_reference_epoch_decimal_year: Option<f64>,
}
```

Add epoch-aware entry points (additive):

```rust
impl Crs {
    pub fn transform_to_with_context(
        &self,
        x: f64,
        y: f64,
        target: &Crs,
        ctx: TransformEpochContext,
    ) -> Result<(f64, f64)>;

    pub fn transform_to_3d_with_context(
        &self,
        x: f64,
        y: f64,
        z: f64,
        target: &Crs,
        ctx: TransformEpochContext,
    ) -> Result<(f64, f64, f64)>;
}
```

Backward compatibility:
- Existing `transform_to*` methods remain unchanged.
- Internally, existing methods call context-aware path with `None`/static defaults.

### 4.2 Velocity-Capable Grid Model

Extend grid sample semantics beyond static dlon/dlat.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicGridShiftSample {
    pub dlon0_arcsec: f64,
    pub dlat0_arcsec: f64,
    pub dlon_rate_arcsec_per_year: f64,
    pub dlat_rate_arcsec_per_year: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicGridShiftGrid {
    pub name: String,
    pub reference_epoch_decimal_year: f64,
    // existing regular grid geometry fields
    pub samples: Vec<DynamicGridShiftSample>,
}
```

At epoch `t`, evaluate:

Delta(t) = Delta0 + (t - t0) * DeltaRate

Applied per interpolated component.

### 4.3 Datum Transform Extensions

Add dynamic transform variants while preserving static ones.

```rust
pub enum DatumTransform {
    None,
    Helmert3(HelmertParams),
    Helmert7(HelmertParams),
    Molodensky(MolodenskyParams),
    GridShift { grid_name: &'static str },
    Ntv2Hierarchy { dataset_name: &'static str },

    // new variants
    DynamicGridShift { grid_name: &'static str },
    DynamicNtv2Hierarchy { dataset_name: &'static str },
}
```

Add context-aware datum geodetic APIs:

```rust
pub fn to_wgs84_geodetic_with_context(..., ctx: Option<TransformEpochContext>) -> Result<_>;
pub fn from_wgs84_geodetic_with_context(..., ctx: Option<TransformEpochContext>) -> Result<_>;
```

### 4.4 Operation Selection Layer

Introduce optional operation routing by CRS pair and operation code.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateOperationDef {
    pub operation_code: u32,
    pub source_crs_code: u32,
    pub target_crs_code: u32,
    pub method: OperationMethod,
    pub preferred: bool,
}
```

Expose opt-in API:

```rust
pub fn transform_to_with_operation(
    &self,
    x: f64,
    y: f64,
    target: &Crs,
    operation_code: u32,
    ctx: Option<TransformEpochContext>,
) -> Result<(f64, f64)>;
```

## 5. Phase Plan

### Phase 0: Scaffolding and Compatibility Guardrails (1 week)

Deliverables:
- Add `TransformEpochContext` type.
- Add new context-aware APIs with static passthrough implementation.
- Add compile-time and test safeguards to ensure no behavior change in existing methods.

Exit criteria:
- Existing test suite passes unchanged.
- No API breakages for current callers.

### Phase 1: Dynamic Grid Core (1-2 weeks)

Deliverables:
- Add dynamic grid structs and interpolation/evaluation methods.
- Add runtime registry for dynamic grids (parallel to static grid registry).
- Add unit tests for interpolation + epoch evaluation math.

Exit criteria:
- Deterministic dynamic sampling tests pass for synthetic grids.

### Phase 2: Loader Extensions (1-2 weeks)

Deliverables:
- Extend loader layer for velocity-enabled input(s) and metadata.
- Register dynamic hierarchy datasets and selected subgrid tracing.
- Validate parser robustness and metadata completeness.

Exit criteria:
- Loader round-trip tests for representative dynamic datasets pass.

### Phase 3: Datum/CRS Pipeline Integration (1-2 weeks)

Deliverables:
- Wire context-aware dynamic variants into datum geodetic transforms.
- Integrate context propagation through CRS transform pipeline.
- Preserve static path performance and behavior.

Exit criteria:
- Mixed static/dynamic integration tests pass.
- Existing static regression tests remain green.

### Phase 4: Operation Selection (1-2 weeks)

Deliverables:
- Add operation definition model and lookup.
- Implement explicit operation-code-based transform execution.
- Fallback strategy when operation unavailable.

Exit criteria:
- Operation-routing integration tests pass for known CRS pairs.

### Phase 5: Conformance and Hardening (2-3 weeks)

Deliverables:
- Build conformance set versus authoritative reference outputs.
- Add precision tolerance matrices by region/workflow.
- Add documentation and user migration notes.

Exit criteria:
- Approved accuracy targets reached.
- Public API docs completed.

## 6. Blast Radius / Impact Assessment

### High-impact modules
- `src/datum.rs`
- `src/grid_shift.rs`
- `src/grid_formats.rs`

Reason:
- Core datum math and grid models are the center of change.

### Medium-impact modules
- `src/crs.rs`
- `src/epsg.rs`

Reason:
- CRS API plumbing and operation/variant lookup expansion.

### Low-to-medium impact modules
- `src/lib.rs`
- docs/tests/examples

Reason:
- Re-exports, docs, and additive usage examples.

### User-facing impact
- Low if additive APIs are used.
- Medium if default behavior is altered (not recommended in v1).

## 7. Effort Estimate

Prototype scope (single corridor, e.g., CSRS v3 -> v8 dynamic path):
- 2-4 weeks.

Production-ready core (robust loaders, operation routing, documentation, tests):
- 6-10 weeks.

High-confidence release (broader conformance coverage + hardening):
- 10-14 weeks.

## 8. Risks and Mitigations

1. Metadata/operation semantics mismatch.
- Mitigation: operation registry with explicit codes; no hidden heuristic switching.

2. Backward-compatibility regressions.
- Mitigation: additive APIs + strict legacy regression suite.

3. Data quality/coverage differences across reference products.
- Mitigation: curated conformance datasets and published tolerance envelopes.

4. Performance regressions in large batch transforms.
- Mitigation: preserve static fast path and benchmark only epoch-aware path deltas.

## 9. Test Strategy

1. Unit tests:
- Dynamic interpolation and epoch evaluation.
- Edge cases around epoch boundaries.

2. Integration tests:
- CRS-to-CRS dynamic transforms with known expected outputs.
- Explicit operation-code routing tests.

3. Regression tests:
- Ensure static workflows remain unchanged numerically.

4. Conformance tests:
- Compare against authoritative checkpoints for representative corridors.

## 10. Recommended Execution Order

1. Phase 0 and Phase 1 in one branchless checkpoint sequence on main.
2. Phase 2 parser work once dynamic core is stable.
3. Phase 3 pipeline integration.
4. Phase 4 operation routing.
5. Phase 5 conformance and release hardening.

## 11. Decision Summary

- This is a medium-to-large internal change.
- It is highly feasible with current architecture.
- It can be delivered with low external disruption by keeping changes additive and opt-in.
- A practical first milestone is a narrow corridor prototype with external validation support.
