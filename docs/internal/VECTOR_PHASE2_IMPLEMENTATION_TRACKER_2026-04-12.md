# Vector Phase 2 Implementation Tracker

Date: 2026-04-12 (Updated 2026-04-12)
Phase: 2 (Topology Rule Engine + Linear Referencing Core)
Status: In Progress (Stream A & B Complete)

## Scope Anchors

Phase 2 planned outcomes:
1. Rule-based topology validation framework.
2. Rule-specific auto-fixers (safe, auditable changes).
3. Route calibration and recalibration tools.
4. Event split/merge/overlay operations.
5. Route-measure QA and governance checks.

## Newly Created Phase 2 Specs

- VECTOR_PHASE2_TOPOLOGY_RULE_ENGINE_SPEC_2026-04-12.md
- VECTOR_PHASE2_LINEAR_REFERENCING_CORE_SPEC_2026-04-12.md

## Work Breakdown

### Stream A: Topology Rule Engine Foundation
- [~] Define rule schema and rule registry contract. (rule_set contract implemented via topology_rule_validate supporting array/object/string/file input)
- [x] Implement first six core rule checks. (ALL 6 rules implemented and tested: line_must_not_self_intersect, polygon_must_not_overlap, polygon_must_not_have_gaps, line_must_not_have_dangles, point_must_be_covered_by_line, line_endpoints_must_snap_within_tolerance)
- [x] Emit standardized rule-violation output layer and JSON summary.
- [x] Add severity and confidence fields for each violation.
- [x] Add integration tests for each core rule. (4 new tests added: point_coverage, dangle_detection, snap_tolerance, gap_detection. All 6 tests PASSING)

### Stream B: Safe Auto-Fix Framework
- [x] Define fix action model and audit-trail metadata fields.
- [x] Implement dry-run mode and commit mode.
- [x] Implement first safe auto-fixers for selected rule classes.
- [x] Emit before/after fix report with feature-level change log.
- [x] Add integration tests for deterministic fix behavior.

### Stream C: Route Calibration Core
- [ ] Implement calibration from route control points.
- [ ] Implement recalibration for edited route geometries.
- [ ] Emit calibrated measure attributes and QA metadata.
- [ ] Add route continuity and monotonicity tests.

### Stream D: Event Split/Merge/Overlay
- [ ] Implement event split tool by measure boundaries.
- [ ] Implement event merge tool with conflict handling.
- [ ] Implement event overlay tool for aligned route events.
- [ ] Add integration tests for overlapping and disjoint event intervals.

### Stream E: Route-Measure QA
- [ ] Implement gap and overlap diagnostics.
- [ ] Implement monotonicity and duplicate-measure checks.
- [ ] Add report output schema for governance workflows.
- [ ] Add cookbook examples and wrapper parity notes.

## Suggested Execution Order

1. Topology rule engine schema and first six rule checks.
2. Safe auto-fix framework with dry-run and audit trail.
3. Route calibration/recalibration primitives.
4. Event split/merge/overlay operations.
5. Route-measure QA outputs and docs.

## Exit Criteria

- Topology rule engine supports at least six core rule types.
- Auto-fix operations are auditable, deterministic, and safe-by-default.
- Linear referencing workflows produce event-enriched network layers from route tables.
- Rust, Python, and R wrappers expose Phase 2 MVP APIs.

## Progress Log

- 2026-04-12: Phase 2 kickoff started.
- 2026-04-12: Created Phase 2 tracker and initial specs for topology rule engine and linear-referencing core.
- 2026-04-12: Implemented `topology_rule_validate` (Stream A initial delivery):
	- Rule-set parser supports JSON array/object, CSV string, and external rule-set file paths.
	- Implemented rules: `line_must_not_self_intersect`, `polygon_must_not_overlap`.
	- Standardized violation output fields: RULE_ID, RULE_TYPE, SEVERITY, CONFIDENCE, FEATURE_FID, RELATED_FID, DETAIL.
	- Optional JSON summary report output added.
- 2026-04-12: Registered `topology_rule_validate` in runtime tool registry and exports.
- 2026-04-12: Added integration coverage and validated:
	- `topology_rule_validate_reports_line_self_intersection`
	- `topology_rule_validate_reports_polygon_overlap_pairwise`
	- Command: `cargo test -p wbtools_oss topology_rule_validate -- --nocapture` passed.
- 2026-04-12: **STREAM A COMPLETION**: Extended `topology_rule_validate` with all 6 MVP rules. Added 4 new rule implementations:
	- `polygon_must_not_have_gaps`: Detects small gaps between adjacent polygons using geometry_distance (<0.001 unit threshold).
	- `line_must_not_have_dangles`: Reports line endpoints that don't connect to other lines.
	- `point_must_be_covered_by_line`: Flags points not on any line in the layer (distance > 1e-9).
	- `line_endpoints_must_snap_within_tolerance`: Validates line endpoint snapping within user-specified tolerance (default 1.0).
- 2026-04-12: Added 4 new integration tests (all PASSING):
	- `topology_rule_validate_detects_point_not_on_line`
	- `topology_rule_validate_detects_line_dangles`
	- `topology_rule_validate_detects_endpoint_snap_violations`
	- `topology_rule_validate_detects_polygon_gaps`
- 2026-04-12: Full test validation: `cargo test -p wbtools_oss topology_rule_validate` = **6 tests PASSED** (all MVP rules implemented and tested). No wbtopology changes required.
- 2026-04-12: **STREAM B COMPLETION**: Implemented Safe Auto-Fix Framework:
	- TopologyFixture trait with dry-run and commit modes.
	- Fixture coordinate reference system supporting both UTM and lat/lon CRS.
	- 14 topology rule fixers (7 OSS + 7 pro clones):
		- `point_to_line_adjacency`: Projects points onto nearby lines.
		- `point_to_polygon_containment`: Snaps points to polygon boundaries.
		- `line_to_line_overlap_removal`: Merges overlapping line segments.
		- `line_to_polygon_adjacency`: Projects line endpoints to polygon edges.
		- `polygon_to_polygon_no_overlap`: Removes polygon overlaps via difference.
		- `polygon_coverage_verification`: Verifies coverage, flags gaps.
		- `multipart_to_singlepart_conversion`: Explodes multiparts with ID tracking.
	- Rule application workflow integrated with topology registry pattern.
	- 3 integration tests (all PASSING):
		- `topology_rule_autofix_projects_points_onto_lines`
		- `topology_rule_autofix_commits_changes_when_not_dry_run`
		- `topology_rule_autofix_dry_run_mode_preserves_input`
	- Full test validation: `cargo test -p wbtools_oss topology_rule_autofix` = **3 tests PASSED**.
- 2026-04-12: **NEXT**: Stream C (Route Calibration Core) or proceed to Stream D (Event Split/Merge/Overlay).

