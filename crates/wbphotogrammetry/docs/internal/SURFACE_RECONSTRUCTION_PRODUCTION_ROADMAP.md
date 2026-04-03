# wbphotogrammetry Surface Reconstruction: Production-Grade Roadmap

Date: 2026-04-03
Owner: wbphotogrammetry (internal)
Status: Draft execution plan

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

Implement Milestone 1 task 1: adaptive sampling-density controller based on local texture and support confidence.
