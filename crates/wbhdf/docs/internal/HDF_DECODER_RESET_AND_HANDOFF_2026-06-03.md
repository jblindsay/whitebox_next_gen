# HDF Decoder Reset And Handoff (2026-06-03)

## Why This Document Exists

This effort entered an expensive empirical loop (repeated probing and long-running tests) that consumed substantial time and Copilot credits without delivering an acceptable product-level outcome.

This document is a hard reset point.

Goals:
1. Preserve exact stop-state context.
2. Prevent repeating the same process failure.
3. Define strict guardrails for any future restart.

## Current Stop-State (Repository)

Working tree currently contains these modified files:
1. crates/wbhdf/CHANGELOG.md
2. crates/wbhdf/docs/SUPPORTED_HDF_PRODUCT_LAYOUTS.md
3. crates/wbraster/CHANGELOG.md
4. crates/wbraster/src/formats/mod.rs
5. crates/wbraster/tests/integration.rs

Observed test state at stop:
1. Targeted VNP21 smoke tests passed.
2. VNP21 full-scene parity test passed in a long run.
3. Despite passing checks, this outcome is not accepted as sufficient progress toward deterministic, broadly reliable HDF decoding goals.

## Problem Statement (Process Failure)

The process failed because implementation and validation order drifted:
1. Too much behavior was validated through expensive empirical loops.
2. Too little deterministic architecture was completed before testing.
3. Long tests were run too frequently.
4. Product-specific probing pressure increased brittleness risk and uncertainty.

User-level impact:
1. High CPU usage.
2. Rapid Copilot credit burn.
3. Low confidence in trajectory.

## Non-Negotiable Process Guardrails For Future Work

Any future HDF work must obey all of the following:

1. No long-running test command without explicit user approval each time.
2. Default validation is cheap only: compile check plus narrow unit/targeted checks.
3. No broad exploratory sweeps by default.
4. No repeated parity-gate runs during implementation loops.
5. Maximum one expensive parity run at end-of-milestone.
6. If a cycle does not reduce architecture risk, stop and reassess before further commands.

## Scope Reality Check (Do Not Overclaim)

Do not claim complete support for a full family unless explicitly demonstrated.

At this handoff point:
1. There is bounded validated behavior for selected paths and fixtures.
2. There is not a complete claim for all VIIRS products.
3. There is not a complete claim for all MODIS products.
4. There is not a complete claim for all GEDI products.
5. There is not a complete claim for all ICESat-2 products.

## Required Architectural Direction (Deterministic-First)

Future implementation must prioritize container-level deterministic decode:
1. Resolve payload location from HDF metadata traversal.
2. Keep product adapters for semantic mapping only (dataset names, scaling, fill conventions).
3. Avoid dependence on product-specific physical byte anchors as steady-state architecture.
4. Preserve explicit unsupported-layout diagnostics instead of silent fallback behavior.

## Restart Plan (If Work Resumes Later)

Use this exact sequence:

Phase 0: Re-baseline
1. Review this document and current git diff.
2. Confirm scope for one narrow target path only.
3. Confirm what is explicitly out-of-scope for the milestone.

Phase 1: Implement
1. Implement one deterministic metadata-driven decode improvement.
2. Do not add broad fixture sweeps.

Phase 2: Validate cheap
1. Run compile check.
2. Run smallest relevant targeted check.

Phase 3: Gate once
1. Run exactly one expensive parity command at milestone end (with explicit approval).
2. Record pass/fail and stop.

## Milestone Acceptance Criteria Template

A milestone is accepted only if all are true:
1. Architecture risk is reduced (not merely hidden by new probes).
2. Unsupported cases fail fast with clear diagnostics.
3. One approved expensive gate passes.
4. Claims are bounded and honest.

## What To Avoid Next Time

Do not repeat:
1. Probe-first workflows that discover behavior by trial and error.
2. Re-running the same long tests during active coding loops.
3. Expanding target surface mid-milestone.
4. Treating test count as progress.

## Practical Next Action (When Ready)

Choose exactly one:
1. Stabilize deterministic HDF5 chunk-index traversal for one concrete layout variant.
2. Harden diagnostics for unsupported layout/filter/datatype combinations.
3. Refactor product-specific physical anchor logic behind a removable compatibility layer.

Any future session should start by picking one of the three actions above and nothing else.
