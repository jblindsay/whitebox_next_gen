# Vector Phase 2 Spec: Topology Rule Engine

Date: 2026-04-12
Phase: 2
Status: Draft (Kickoff)

## Objective

Provide a configurable, auditable topology rule engine that validates vector datasets against explicit rule sets and emits structured, machine-readable violation outputs.

## MVP Tool Surface

Planned tool IDs:
- topology_rule_validate
- topology_rule_autofix

MVP inputs (topology_rule_validate):
- input: vector layer path
- rule_set: JSON file path or inline JSON
- output: vector diagnostics output path
- report: JSON summary output path (optional)
- strict: boolean (optional; fail on schema/rule errors)

MVP outputs:
- Diagnostics vector layer with one feature per violation.
- JSON summary report with counts by rule, severity, and geometry type.

## Core Rule Types (MVP Six)

1. polygon_must_not_overlap
2. polygon_must_not_have_gaps
3. line_must_not_self_intersect
4. line_must_not_have_dangles
5. point_must_be_covered_by_line
6. line_endpoints_must_snap_within_tolerance

## Rule Schema (Draft)

Each rule entry:
- rule_id: stable unique ID string
- rule_type: one of supported rule types
- enabled: boolean
- severity: info | warning | error
- tolerance: optional numeric tolerance
- params: optional object (rule-specific)

Rule-set object:
- version: schema version string
- target_geometry_type: optional enum
- rules: array of rule entries

## Diagnostics Output Schema

Vector fields:
- RULE_ID (text)
- RULE_TYPE (text)
- SEVERITY (text)
- CONFIDENCE (float 0-1)
- FEATURE_FID (integer)
- RELATED_FID (integer nullable)
- DETAIL (text)

JSON summary:
- total_violations
- violations_by_rule
- violations_by_severity
- violations_by_geometry_type
- processing_time_ms

## Auto-Fix Safety Model (MVP)

topology_rule_autofix behaviors:
- dry_run default true
- apply_changes optional true
- always emit change report
- never delete features in MVP
- fixes must be deterministic and reversible from audit metadata

Change report fields:
- change_id
- rule_id
- action_type
- target_fid
- pre_state_hash
- post_state_hash

## Validation And Test Strategy

Integration tests:
- One fixture per rule type with expected violation count.
- Mixed-rule fixture validating grouped reporting.
- Strict-mode failure when rule schema is invalid.
- Deterministic output ordering for stable CI snapshots.

Performance target (MVP):
- Complete six-rule validation on medium benchmark layer within agreed Phase 2 budget.

## Out Of Scope (MVP)

- Full persistent topology graph editing sessions.
- Cross-layer enterprise geodatabase rule orchestration.
- Aggressive geometry reconstruction fixers with high semantic risk.
