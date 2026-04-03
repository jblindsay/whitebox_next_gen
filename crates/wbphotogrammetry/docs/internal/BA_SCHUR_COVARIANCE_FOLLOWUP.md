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

## Mission-Level Synthetic Benchmark Matrix (2026-04-03)

Command:

`bash crates/wbphotogrammetry/examples/run_profile_benchmark.sh --out-dir target/wbphotogrammetry_profiles_schur --repeats 2 --profile balanced --frames 30,60,120 --resolution 0.12`

Generated reports:

1. `target/wbphotogrammetry_profiles_schur/profile_balanced_30f_2r.json`
2. `target/wbphotogrammetry_profiles_schur/profile_balanced_60f_2r.json`
3. `target/wbphotogrammetry_profiles_schur/profile_balanced_120f_2r.json`

Summary by frame count (mean stage runtime):

1. Small (`30` frames): feature `6.438 s`, alignment `0.590 s`, dense `0.023 s`, mosaic `0.100 s`
2. Medium (`60` frames): feature `24.926 s`, alignment `2.383 s`, dense `0.128 s`, mosaic `0.390 s`
3. Large (`120` frames): feature `107.915 s`, alignment `9.720 s`, dense `1.032 s`, mosaic `2.060 s`

Observed scaling notes:

1. Alignment stage scales near-linearly across this synthetic matrix.
2. Feature stage remains dominant in total runtime at all tested sizes.
3. Mission-level benchmark matrix is now recorded for repeatable synthetic workloads.
4. Strict Schur-vs-baseline runtime acceptance still needs an explicit runtime switch to force the old dense-only reduced solve path during the same benchmark harness.

## Solver Mode A/B Benchmark (2026-04-03)

Command set:

1. `bash crates/wbphotogrammetry/examples/run_profile_benchmark.sh --out-dir target/wbphotogrammetry_profiles_ab/sparse_pcg --repeats 2 --profile balanced --frames 30,60,120 --resolution 0.12 --reduced-solver-mode sparse-pcg`
2. `bash crates/wbphotogrammetry/examples/run_profile_benchmark.sh --out-dir target/wbphotogrammetry_profiles_ab/dense_lu --repeats 2 --profile balanced --frames 30,60,120 --resolution 0.12 --reduced-solver-mode dense-lu`

Alignment stage mean runtime comparison:

1. Small (30 frames): sparse `0.597 s`, dense `0.603 s` (sparse ~`1.0%` faster)
2. Medium (60 frames): sparse `2.447 s`, dense `2.367 s` (sparse ~`3.4%` slower)
3. Large (120 frames): sparse `9.702 s`, dense `9.629 s` (sparse ~`0.8%` slower)

Notes:

1. This provides the requested runtime-toggle A/B evidence on identical workloads.
2. Differences are small and mixed across matrix sizes; no consistent runtime win is demonstrated yet.
3. Additional repeats and real mission datasets are still needed for a stable acceptance conclusion.

## Real Mission A/B Benchmark: Toledo (2026-04-03)

Dataset:

1. Images: `/Users/johnlindsay/Documents/programming/Rust/drone_sfm_real_flight/datasets/toledo/images`
2. Frame count: `87`
3. Profile: `balanced`
4. Feature method: `rootsift`
5. DEM resolution: `0.1 m`

Command set:

1. `cargo run -p wbphotogrammetry --example run_dataset_pipeline -- --images-dir /Users/johnlindsay/Documents/programming/Rust/drone_sfm_real_flight/datasets/toledo/images --out-dir target/wbphotogrammetry_toledo_ab/sparse_pcg --profile balanced --camera-model auto --resolution 0.1 --reduced-solver-mode sparse-pcg`
2. `cargo run -p wbphotogrammetry --example run_dataset_pipeline -- --images-dir /Users/johnlindsay/Documents/programming/Rust/drone_sfm_real_flight/datasets/toledo/images --out-dir target/wbphotogrammetry_toledo_ab/dense_lu --profile balanced --camera-model auto --resolution 0.1 --reduced-solver-mode dense-lu`

Timing comparison:

1. Ingest: sparse `22.915 s`, dense `22.656 s`
2. Feature: sparse `51.738 s`, dense `50.748 s`
3. Alignment: sparse `1.260 s`, dense `1.238 s`
4. Dense: sparse `65.975 s`, dense `66.532 s`
5. Mosaic: sparse `113.596 s`, dense `116.140 s`

Alignment quality parity:

1. RMSE: sparse `2.8 px`, dense `2.8 px`
2. Residual p95: sparse `10.2749 px`, dense `10.2749 px`
3. BA final cost: sparse `6.7922`, dense `6.7922`
4. Covariance supported cameras: sparse `86`, dense `86`
5. Translation sigma p95: sparse `0.4303 m`, dense `0.4303 m`
6. Rotation sigma p95: sparse `0.4515 deg`, dense `0.4515 deg`
7. QA status: sparse `Fail`, dense `Fail`

Peak memory comparison (`/usr/bin/time -l`):

1. Sparse maximum resident set size: `4,249,878,528 bytes` (~`3.96 GiB`)
2. Dense maximum resident set size: `4,205,936,640 bytes` (~`3.92 GiB`)
3. Sparse peak RSS delta vs dense: ~`41.9 MiB` higher

Interpretation:

1. Reduced solver mode does not materially change end-to-end real-mission outputs on Toledo.
2. Alignment stage timing differs by only ~`22 ms` between sparse and dense modes on this mission.
3. Peak memory is effectively similar at mission scale, with sparse mode slightly higher on this run.
4. Full-pipeline runtime remains dominated by feature extraction, dense reconstruction, and mosaicking.
5. Real-dataset quality parity is now demonstrated for the current Schur reduced solve path.

## Risks
1. Ill-conditioning in reduced camera system for weak geometry.
2. Sparse assembly memory growth if block storage is not tightly controlled.
3. Covariance misinterpretation without strict semantics documentation.

## Definition of Done
1. Schur BA path is enabled as a stable production path (with guarded fallback). (in progress: implemented and regression-tested on synthetic/targeted paths)
2. Camera covariance diagnostics are emitted and documented. (implemented)
3. Regression suite remains green and benchmark results are recorded. (substantially complete: targeted regressions, synthetic mission benchmark matrix, solver-mode A/B timing, Toledo real-dataset parity, and Toledo peak-memory comparison recorded; runtime/memory superiority not demonstrated)
