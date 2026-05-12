# wbtopology Sprint Restart Starting Point

Date: 2026-05-12
Status: ready for next sprint
Owner: wbtopology hardening stream

## Purpose

This document is the canonical restart point for the next wbtopology correctness sprint.
It summarizes what is stable now, what is unresolved, and the exact first tasks to run
when work resumes.

## Current Checkpoint Assessment

This is a clean pause point operationally, but not strategic completion.

What is stable now:
- Full wbtopology test suite is green after recent hardening updates.
- Noding multiplicity regressions are active.
- Near-coincident and shallow-angle noding corpus tests are active.
- Provenance-tagged reference parity fixture import path is active (offline GEOS/JTS traces only; no runtime dependency).
- Strict 4-hole symmetric-difference diagnostics are active and non-blocking with side-by-side signatures.
- Graph precision differential diagnostics are active.

What remains unresolved:
- Strict 4-hole symmetric-difference API-vs-direct divergence remains significant in known frontier cases.
- Deeper downstream trace alignment is still needed for face labeling and ring assembly behavior.
- Performance parity and real-world dissolve throughput are still unproven in this hardening line.

## Primary Unresolved Risk Focus (from postmortem)

1. Systematic bug in graph or noding primitives.
2. Subtle algorithm incompleteness that survives architecture-level correctness.
3. Silent implementation corruption that requires side-by-side GEOS reference traces to localize.

## Recommended Next Sprint Objective

Primary objective:
- Convert strict 4-hole symmetric-difference diagnostics from non-blocking reporting into passing parity guards.

Secondary objective:
- Expand downstream traceability so the failing stage is explicitly localized (noding, graph depth labels, or ring assembly).

Out of scope for this next sprint:
- Performance benchmarking and optimization as a main target.
- Frontend exposure decisions.

## First-Session Task List

1. Baseline validation run:
- cargo test -p wbtopology --test noding_tests
- cargo test -p wbtopology --test overlay_fixture_corpus_tests -- --nocapture
- cargo test -p wbtopology

2. Reproduce strict 4-hole symmetric-difference divergence from diagnostics output.

3. Add stage-localized assertions around the symmetric-difference path so each stage can be compared between:
- API route (polygon_sym_diff)
- direct route (polygon_overlay with SymmetricDifference)

4. Promote one stable parity condition at a time from diagnostics to strict assertions.

5. Keep diagnostics non-blocking for unstable conditions until reproducibly converged.

## Fixture and Test Assets To Start From

Noding corpora:
- tests/fixtures/noding_shallow_angle_cases.txt
- tests/fixtures/noding_reference_parity_cases.txt

Noding test entry points:
- tests/noding_tests.rs

Overlay frontier and strict 4-hole diagnostics:
- tests/fixtures/overlay_invariants.txt
- tests/overlay_fixture_corpus_tests.rs

Graph precision differential diagnostics:
- tests/graph_tests.rs

## Definition Of Done For Next Sprint

Minimum completion criteria:
- At least one strict 4-hole symmetric-difference case promoted from diagnostic-only to strict parity pass.
- No regressions in full wbtopology suite.
- New behavior documented in CHANGELOG with explicit note on what became strict and what remains diagnostic.

Stretch criteria:
- All current strict 4-hole symmetric-difference divergences resolved or reduced to documented, bounded residuals.

## Safety and Process Notes

- Maintain small checkpoint commits at logical milestones.
- Keep GEOS/JTS reference usage fixture-based and offline unless explicitly introducing a separate optional comparison harness.
- Treat performance measurement as a separate track from correctness convergence.

## Resume Command Block

Run from repository root:

- cargo test -p wbtopology --test noding_tests
- cargo test -p wbtopology --test graph_tests -- --nocapture
- cargo test -p wbtopology --test overlay_fixture_corpus_tests -- --nocapture
- cargo test -p wbtopology
