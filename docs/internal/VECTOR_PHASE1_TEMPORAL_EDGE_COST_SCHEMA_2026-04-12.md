# Vector Phase 1 Temporal Edge-Cost Schema

Date: 2026-04-12
Phase: 1 (High-ROI Network Extensions)
Status: Initial spec

## Goal

Add optional time-dependent edge costs to existing network tools without breaking current static-cost behavior.

Initial target tools:
- shortest_path_network
- k_shortest_paths_network
- network_service_area
- network_routes_from_od
- network_od_cost_matrix

## Backward Compatibility

- If no temporal profile is provided, behavior is unchanged.
- Existing edge_cost_field semantics remain valid.
- Temporal support is additive.

## Proposed New Arguments

1. temporal_cost_profile
- Type: string path to CSV
- Optional
- Meaning: profile table that defines cost multipliers or absolute costs by edge/time bucket

2. departure_time
- Type: RFC3339 datetime string (for example, 2026-04-12T08:30:00Z)
- Optional
- Meaning: query start time used to choose effective edge costs

3. temporal_mode
- Type: enum string
- Allowed values:
  - multiplier (default): base static cost multiplied by temporal factor
  - absolute: temporal table provides full effective edge cost
- Optional

4. temporal_fallback
- Type: enum string
- Allowed values:
  - static_cost (default)
  - error
- Optional

## Temporal CSV Schema (v1)

Required columns:
- edge_id: unique edge identifier matching a network field
- dow: integer day of week (1=Mon ... 7=Sun)
- start_minute: minute of day [0..1439]
- end_minute: minute of day [1..1440], end > start
- value: multiplier or absolute cost (depends on temporal_mode)

Optional columns:
- profile_id: named profile selector (future multi-scenario support)
- quality_flag: optional QA indicator for diagnostics

Constraints:
- No overlapping windows per (edge_id, dow, profile_id).
- value must be finite and > 0.
- Full-day coverage is not required in v1; uncovered times use temporal_fallback behavior.

## Tool Field Mapping

Network layer requirements for v1 temporal support:
- edge temporal key field (recommended name: EDGE_ID)

Proposed new optional argument:
- temporal_edge_id_field: string, defaults to EDGE_ID if present

Validation rules:
- If temporal_cost_profile is provided, temporal_edge_id_field must resolve to an existing field.
- Every profile row edge_id must match at least one network edge or produce warning/error based on strictness mode.

## Runtime Evaluation (v1)

At traversal-time for each edge:
1. Determine effective query timestamp for edge entry.
2. Resolve day-of-week and minute-of-day.
3. Lookup profile row by (edge_id, dow, minute window).
4. Apply temporal_mode:
  - multiplier: effective_cost = static_cost * value
  - absolute: effective_cost = value
5. If lookup misses:
  - static_cost fallback or error according to temporal_fallback.

Notes:
- Phase 1 implementation may use departure-time-fixed costs as an initial simplification.
- Full time-propagation while traversing edges can be introduced as a subsequent enhancement.

## Diagnostics Outputs (Phase 1)

When temporal profile is used, include a diagnostics table (CSV/JSON):
- unmatched_profile_edge_ids
- network_edges_without_temporal_rows
- overlapping_time_windows
- invalid_rows_count
- fallback_usage_count

## Acceptance Criteria

1. Existing static regression tests remain unchanged and pass.
2. New temporal tests verify:
- different departure times select different paths/costs
- fallback behavior for uncovered windows
- invalid temporal profiles fail with actionable errors
3. Documentation includes one runnable example for each affected network tool.
