# Epoch-Aware Datum Transform Execution Checklist

Date: 2026-06-01
Scope: `wbprojection` implementation checklist derived from `EPOCH_AWARE_DATUM_TRANSFORM_PLAN.md`
Style: commit-oriented, additive-first, backward-compatible

## Guardrails (Apply To Every Step)

- Keep existing `transform_to*` APIs behavior unchanged.
- Add new epoch-aware APIs as opt-in first.
- Prefer small checkpoint commits on `main` at logical milestones.
- Keep static transform performance path intact.
- Add tests with each feature step, not only at the end.

## Milestone 0: Baseline And Safety Net

### Commit 1: Add epoch context type (no behavior change)

Files (expected):
- `src/transform.rs` or new `src/transform_context.rs`
- `src/lib.rs` re-export
- tests (small compile/construct tests)

Tasks:
- Add `TransformEpochContext` struct.
- Add constructors/helpers (minimal).
- Add docs clarifying decimal year semantics.

Acceptance:
- No existing API changes required by callers.
- Full existing test suite remains green.

---

### Commit 2: Add context-aware CRS API shells (pass-through)

Files (expected):
- `src/crs.rs`
- `src/lib.rs`
- tests for pass-through equivalence

Tasks:
- Add `transform_to_with_context(...)`.
- Add `transform_to_3d_with_context(...)`.
- Route to existing static logic internally (no epoch math yet).

Acceptance:
- New methods compile and run.
- Existing and new APIs return identical results in static mode.

## Milestone 1: Dynamic Grid Core

### Commit 3: Add dynamic grid sample and grid structs

Files (expected):
- `src/grid_shift.rs`
- tests for interpolation/evaluation

Tasks:
- Add dynamic sample fields: base shift + rate shift.
- Add `reference_epoch_decimal_year` on dynamic grid.
- Add epoch-evaluated sample method:
  - `sample_shift_degrees_at_epoch(lon, lat, t)`

Acceptance:
- Synthetic tests validate:
  - zero delta-time returns base shift,
  - non-zero delta-time applies correct linear rate.

---

### Commit 4: Add dynamic grid registry

Files (expected):
- `src/grid_shift.rs`
- `src/lib.rs`
- tests

Tasks:
- Add register/get/has/unregister for dynamic grids.
- Keep static registry untouched.

Acceptance:
- Registry tests pass for both static and dynamic registries.

## Milestone 2: Loader Extensions

### Commit 5: Add loader support for dynamic grid inputs

Files (expected):
- `src/grid_formats.rs`
- tests with fixture/synthetic binary snippets

Tasks:
- Add parser path for velocity-enabled grid payloads or companion inputs.
- Persist reference epoch metadata.
- Validate required fields and robust errors.

Acceptance:
- Parser tests pass with valid and invalid input coverage.
- Loaded objects can be sampled at epoch.

---

### Commit 6: Add dynamic hierarchy registration and lookup

Files (expected):
- `src/grid_formats.rs`
- tests

Tasks:
- Add hierarchy registration for dynamic datasets.
- Add runtime coordinate-based subgrid selection for dynamic hierarchy.

Acceptance:
- Selected subgrid matches expected hierarchy rule for test points.

## Milestone 3: Datum Integration

### Commit 7: Extend DatumTransform enum with dynamic variants

Files (expected):
- `src/datum.rs`
- tests

Tasks:
- Add `DynamicGridShift` and `DynamicNtv2Hierarchy` variants.
- Keep existing variants unchanged.

Acceptance:
- Existing enum usage compiles.
- Existing static tests remain green.

---

### Commit 8: Add context-aware datum transform functions

Files (expected):
- `src/datum.rs`
- `src/crs.rs`
- tests

Tasks:
- Add context-aware geodetic transform methods.
- Apply dynamic grid evaluation when dynamic variant + context is provided.
- Define strict behavior when context is missing for dynamic transform.

Acceptance:
- Dynamic tests pass with epoch context.
- Clear error behavior verified for missing required context.

## Milestone 4: CRS Pipeline Wiring

### Commit 9: Propagate context through CRS transform pipeline

Files (expected):
- `src/crs.rs`
- tests/integration tests

Tasks:
- Route context-aware CRS methods into context-aware datum path.
- Ensure `transform_to*` existing methods use static default context.

Acceptance:
- No regressions in existing transform outputs.
- Context-aware outputs differ only where expected.

## Milestone 5: Operation Selection Layer

### Commit 10: Introduce coordinate operation definition model

Files (expected):
- new `src/operations.rs` (or equivalent)
- `src/lib.rs`
- tests

Tasks:
- Add operation metadata struct (`operation_code`, source CRS, target CRS, method).
- Add lookup/registration model.

Acceptance:
- Operation lookup tests pass for synthetic definitions.

---

### Commit 11: Add explicit operation-code transform API

Files (expected):
- `src/crs.rs`
- `src/epsg.rs` (if lookup integration needed)
- tests/integration tests

Tasks:
- Add transform methods that accept operation code (+ optional context).
- Execute selected operation path deterministically.
- Add explicit fallback behavior.

Acceptance:
- Operation-routed transforms match expected branch and outputs.

## Milestone 6: Conformance And Hardening

### Commit 12: Add authoritative conformance tests

Files (expected):
- `src/tests/integration_tests.rs`
- `src/tests/epsg_tests.rs`
- test fixture metadata docs

Tasks:
- Add reference-point conformance tests for priority corridors.
- Add tolerance matrix by corridor.

Acceptance:
- Conformance tests pass at approved tolerances.

---

### Commit 13: Documentation and migration notes

Files (expected):
- `README.md`
- internal docs under `docs/internal/`
- `CHANGELOG.md`

Tasks:
- Document new APIs and examples.
- Document static vs dynamic behavior and guidance.
- Update changelog entries.

Acceptance:
- Docs are complete enough for first external adopters.

## Suggested Initial Delivery Scope (Fastest Useful Prototype)

Use this subset first:
1. Commits 1-2 (context scaffolding).
2. Commits 3-4 (dynamic core + registry).
3. Commit 7-9 (datum + CRS integration).
4. Minimal conformance tests for one CSRS corridor.

Expected prototype window:
- About 2-4 weeks.

## Effort And Impact Snapshot

- Internal code impact: medium-to-large.
- External API disruption risk: low (if additive path retained).
- Highest complexity: loader semantics + operation selection correctness.
- Largest regression risk area: `datum.rs` and `crs.rs` transform plumbing.

## Ready-For-Implementation Definition

Begin coding when all are true:
- One target corridor and reference checkpoints chosen.
- Dynamic input format source finalized.
- Tolerance targets agreed.
- Strict missing-context behavior agreed for dynamic transforms.
