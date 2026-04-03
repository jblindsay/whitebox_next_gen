# Issue Template: BA Sparse Schur + Covariance Milestone

Use this template for the next wbphotogrammetry BA milestone after the recent pre-Schur improvements.

## Title
BA: Introduce sparse Schur-complement solve and camera covariance diagnostics

## Problem Statement
Current BA quality has improved (structure-only point refresh, coupled second-order camera updates, selective intrinsics refinement policy, robust loop-closure correction rejection), but two architectural gaps remain:

1. Camera/structure updates are not solved with a full sparse Schur-complement normal-equation path.
2. Camera uncertainty is not exposed via covariance-style diagnostics.

These are now the primary blockers for scaling and uncertainty reporting in larger missions.

## Scope
1. Add block-sparse BA assembly for camera and structure parameters.
2. Add Schur elimination of structure blocks and reduced camera solve.
3. Preserve current LM damping and robust weighting behavior semantics.
4. Add covariance diagnostics for camera parameters (at minimum rotational and translational uncertainty proxies).
5. Keep existing fallback behavior for small/weak-geometry cases.

## Out of Scope
1. Full pose-graph SLAM backend rewrite.
2. Front-end feature matching redesign.
3. GPU acceleration.

## Implementation Checklist
- [x] Add block indexing and sparse normal-equation accumulation in alignment BA internals.
- [x] Implement Schur reduction and reduced camera system solve.
- [x] Back-substitute structure updates from solved camera increments.
- [x] Integrate LM damping and robust weights into reduced solve path.
- [x] Add stable fallback trigger to current non-Schur path for weak/degenerate systems.
- [x] Emit covariance diagnostics in alignment stats (with explicit field semantics).
- [x] Update README limitation wording and capability notes.

## Acceptance Tests
1. Correctness and stability
	- Existing wbphotogrammetry tests pass unchanged.
	- New synthetic large-network tests show stable convergence and finite outputs.
2. Performance and scaling
	- On at least one representative larger mission benchmark, Schur mode shows equal or better runtime and/or memory use versus current baseline.
3. Output quality
	- Final reprojection/error metrics are non-regressive versus baseline on current benchmark scenes.
4. Covariance reporting
	- Alignment outputs include camera covariance proxy fields.
	- Field meaning/units are documented and validated in tests.
5. Operational safety
	- Degenerate geometry triggers fallback without pipeline failure.

## Suggested Benchmark Matrix
1. Small network: dense overlap, low drift risk.
2. Medium network: moderate loop closure and baseline diversity.
3. Large network: long corridor or weakly connected topology.

Track per run:
1. BA iterations and final cost.
2. Runtime and peak memory.
3. Final alignment error metrics.
4. Covariance proxy ranges and outlier behavior.

## Current Benchmark Snapshot (2026-04-03)

Command:

`cargo test -p wbphotogrammetry schur_sparse_solver_benchmark_matrix_reports_runtime_and_parity -- --ignored --nocapture`

Synthetic reduced-system matrix (block-chain, damped solve) results:

1. Small (`blocks=6`, `dim=24`): sparse `257.04 us`, dense `9.96 us`, ratio `25.810`, error `0.000e0`
2. Medium (`blocks=16`, `dim=64`): sparse `124.12 us`, dense `83.54 us`, ratio `1.486`, error `0.000e0`
3. Large (`blocks=32`, `dim=128`): sparse `658.88 us`, dense `553.92 us`, ratio `1.189`, error `0.000e0`

Interpretation:

1. Numerical parity is exact to reported precision for all matrix sizes in this harness.
2. Sparse path currently shows fixed overhead at very small sizes.
3. Runtime gap narrows substantially at medium/large reduced systems.
4. Next step is to capture end-to-end mission-level runtime/memory on real datasets using `examples/profile_pipeline.rs`.

## Risks
1. Ill-conditioning in reduced camera system for weak geometry.
2. Sparse assembly memory growth if block storage is not tightly controlled.
3. Covariance misinterpretation without strict semantics documentation.

## Definition of Done
1. Schur BA path is enabled as a stable production path (with guarded fallback). (in progress: implemented and regression-tested on synthetic/targeted paths)
2. Camera covariance diagnostics are emitted and documented. (implemented)
3. Regression suite remains green and benchmark results are recorded. (in progress: targeted regressions and synthetic benchmark recorded; full mission benchmark matrix pending)
