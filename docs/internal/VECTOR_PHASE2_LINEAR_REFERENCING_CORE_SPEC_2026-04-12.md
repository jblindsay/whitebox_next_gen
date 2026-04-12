# Vector Phase 2 Spec: Linear Referencing Core

Date: 2026-04-12
Phase: 2
Status: Draft (Kickoff)

## Objective

Add enterprise-grade linear-referencing primitives beyond current event materialization so route measures remain calibrated, governable, and QA-auditable across edits.

## MVP Tool Surface

Planned tool IDs:
- route_calibrate
- route_recalibrate
- route_event_split
- route_event_merge
- route_event_overlay
- route_measure_qa

## MVP Capabilities

1. Route calibration from control points
- Build or correct route measures from known station/control locations.

2. Route recalibration after geometry edits
- Preserve measure continuity after route segment updates.

3. Event split/merge/overlay
- Split events by boundaries.
- Merge adjacent compatible events.
- Overlay event tables/layers onto calibrated routes.

4. Route-measure QA
- Detect gaps, overlaps, non-monotonic intervals, and duplicate measures.

## Canonical Data Contracts (Draft)

Route layer requirements:
- route_id field
- geometry as line/multiline
- optional from_measure and to_measure fields

Event layer/table requirements:
- route_id field
- from_measure
- to_measure
- payload attributes

Control point requirements (for calibration):
- route_id
- measure
- point geometry

## QA Report Schema (Draft)

JSON report sections:
- route_count
- event_count
- gap_count
- overlap_count
- non_monotonic_count
- duplicate_measure_count
- route_level_details array

Diagnostics output fields:
- ROUTE_ID
- ISSUE_TYPE
- SEVERITY
- FROM_MEAS
- TO_MEAS
- DETAIL

## Wrapper/API Expectations

Rust/Python/R parity requirements:
- All MVP tools exposed consistently.
- Shared argument naming for route_id/from_measure/to_measure.
- Consistent output contract descriptions in docs and stubs.

## Validation And Test Strategy

Integration tests:
- Calibration baseline fixture with known control points.
- Recalibration fixture after synthetic geometry edits.
- Event split/merge round-trip invariants.
- Overlay correctness on overlapping and disjoint intervals.
- QA report accuracy on intentionally malformed measure sequences.

Performance target (MVP):
- Route/event QA completes within Phase 2 target budget on medium benchmark datasets.

## Out Of Scope (MVP)

- Full LRS governance UI/state model.
- Cross-system route version reconciliation.
- Real-time streaming calibration updates.
