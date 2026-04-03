# wbphotogrammetry Surface Reconstruction: Production-Grade Roadmap

Date: 2026-04-03
Owner: wbphotogrammetry (internal)
Status: Active execution (Milestone 1 complete in code; Milestone 2 in progress)

## Goal

Move dense surface reconstruction from advanced prototype to production-grade quality, robustness, and scalability for operational missions.

## Current Position (Summary)

Implemented baseline now includes:

1. Multi-view depth-map seeding
2. Epipolar-constrained matching with left-right consistency checks
3. Hybrid ZNCC + Census matching cost
4. Coarse-to-fine (2-level) matching guidance
5. Geometric consistency checks and geometry-aware confidence weighting
6. Occlusion-aware bin voting
7. Neighborhood propagation and iterative sparse-hole fill
8. Edge-aware attenuation, weak-isolated suppression, local agreement calibration, and robust near-front cluster fusion

Remaining gaps to production:

1. No full depth-map optimization stage with strong regularization
2. No benchmark-gated quality acceptance against reference datasets
3. No guaranteed large-mission memory/runtime envelope
4. Limited operational fallback policy and deterministic reproducibility controls

## Progress Update (2026-04-03)

Milestone 1 implementation status:

1. Task 1 completed: adaptive texture-driven MVS sampling density
2. Task 2 completed: local edge-aware depth-bin regularization
3. Task 3 completed: confidence-calibrated residual refinement pass
4. Task 4 completed: per-reference completeness metrics + low-confidence mask emission
5. Dense tests remain green after all Task 1-4 changes

Milestone 1 implementation commits:

1. `4d8fcb2` (Task 1)
2. `f4d43d3` (Task 2)
3. `29d2c38` (Task 3)
4. `a8cf6ba` (Task 4)

Milestone 2 groundwork completed:

1. Added size-aware dataset matrix runner: `examples/run_dataset_matrix.py`
2. Added large-dataset safeguards: `--max-images-per-dataset`, `--max-dataset-gb`, `--max-total-gb`, `--dry-run`
3. Extended dataset report comparator with new dense metrics:
   - `dsm.mvs_mean_reference_completeness_pct`
   - `dsm.low_confidence_cells_pct`

First real dataset-matrix run (balanced, rootsift, 0.1 m, sparse-pcg):

1. Command root: `/Users/johnlindsay/Documents/programming/Rust/drone_sfm_real_flight/datasets`
2. Selection controls: `max-images-per-dataset=120`, `max-dataset-gb=8`, `max-total-gb=20`
3. Datasets executed: 6
4. Successful runs: 6
5. QA status counts: Fail=5, Review=1, Pass=0
6. Mean total runtime: 334.52 s
7. Summary JSON: `target/wbphotogrammetry_dataset_matrix_run1/dataset_matrix_summary.json`

Key observations from run1:

1. Pipeline reliability is stable (0 hard failures) across all discovered datasets under size controls.
2. Current QA thresholds remain strict for this matrix (no Pass outcomes yet).
3. `shitan_tw` is an outlier in both runtime and low-confidence area:
   - total runtime: 1329.03 s
   - low_confidence_cells_pct: 33.65%
4. Mean MVS reference completeness over run1 is generally low-to-moderate (about 9.5% to 20.1%).

## Milestone 1 (6-8 weeks): Dense Depth-Map Quality Core

Objective:

Establish a stronger depth-map estimation and local regularization core while preserving current architecture.

Scope:

1. Increase depth-map sampling density adaptively by texture and support confidence
2. Add local edge-aware regularization over reference-view depth bins
3. Add confidence-calibrated depth residual refinement pass (sub-bin consistency tuning)
4. Add per-reference depth completeness metric and low-confidence mask emission
5. Maintain current MVS fallback chain behavior

Out of scope:

1. Full SGM/SGM++ cost-volume implementation
2. Full PatchMatch stereo rewrite

Acceptance criteria:

1. Dense test suite remains green
2. At least 3 representative mission datasets show improved DSM completeness versus current baseline
3. No regression greater than 5% in median local-relief stability metric on baseline datasets
4. New confidence masks are emitted and documented

## Milestone 2 (6-8 weeks): Validation and Reliability Gates

Objective:

Turn quality into measurable, enforced release gates.

Scope:

1. Add benchmark dataset matrix (urban, vegetation, mixed terrain, low-texture scenes)
2. Add quality report comparator pipeline with standardized outputs
3. Add CI regression checks for:
   - completeness
   - vertical error proxy statistics
   - seam artifact proxy from downstream mosaic
4. Add fallback policy table for weak geometry / poor overlap / low texture
5. Add deterministic mode toggle for repeatability testing

Current status (2026-04-03):

1. Scope item 1 started and partially delivered (matrix runner + first real run)
2. Scope item 2 started and partially delivered (comparator extended for new dense metrics)
3. Scope items 3-5 not started

Out of scope:

1. New sensor families
2. Full absolute GCP bundle adjustment integration

Acceptance criteria:

1. Every release candidate runs quality matrix and passes configured thresholds
2. Deterministic mode variance remains within pre-agreed epsilon bands
3. Fallback routing demonstrates graceful completion (no hard failure) on stress fixtures

## Milestone 3 (6-8 weeks): Scalability and Operational Hardening

Objective:

Enable predictable operation on larger mission sizes with bounded resources.

Scope:

1. Add tiled/streamed dense processing mode
2. Add memory-budget aware chunk scheduling
3. Add performance telemetry outputs for stage timing and peak memory approximation
4. Add watchdog thresholds and user-facing diagnostics for overload conditions
5. Add long-run soak tests on larger datasets

Out of scope:

1. GPU backend
2. Distributed processing

Acceptance criteria:

1. Meets target runtime/memory budget on agreed large-mission benchmark set
2. No OOM or fatal instability in soak tests
3. Quality remains within threshold relative to non-tiled baseline

## Suggested Metrics (Track Per Run)

1. DSM completeness ratio
2. Confidence-weighted valid-cell ratio
3. Vertical error proxy (RMSE-like and robust quantiles)
4. Local relief consistency (mean and P95 deltas)
5. Outlier artifact count proxy (isolated spikes/depressions)
6. Dense stage runtime and peak memory estimate

## Risk Register

1. Over-regularization can blur true elevation discontinuities
2. Quality gates may be brittle without dataset diversity
3. Tiling can introduce seam artifacts if border exchange is weak
4. Deterministic mode can reduce throughput and parallel scaling

## Definition of Production-Ready (Surface Reconstruction)

1. Quality: benchmark matrix thresholds pass consistently
2. Reliability: fallback behavior covers common failure modes
3. Reproducibility: deterministic mode produces stable outputs within tolerance
4. Scalability: large-mission runtime/memory envelopes are met
5. Operability: diagnostics and telemetry are actionable for support

## Next Execution Step

Implement Milestone 2 scope item 3: CI-ready regression gate checks for completeness, vertical error proxy statistics, and seam artifact proxy based on the new matrix summary/report outputs.
