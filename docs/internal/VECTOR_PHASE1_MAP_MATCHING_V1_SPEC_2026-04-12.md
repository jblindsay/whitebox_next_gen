# Vector Phase 1 Map Matching v1 Spec

Date: 2026-04-12
Phase: 1 (High-ROI Network Extensions)
Status: Initial spec

## Goal

Provide an MVP map-matching tool that snaps GPS trajectory points onto a network and returns matched route geometry with per-point diagnostics.

## Proposed Tool

Tool id:
- map_matching_v1

Category:
- Vector / Network Analysis

## Inputs

Required:
- network: line vector
- trajectory_points: point vector (ordered by timestamp field)
- timestamp_field: string

Optional:
- snap_tolerance: float
- max_snap_distance: float
- edge_cost_field: string
- one_way_field: string
- blocked_field: string
- barriers: point vector
- barrier_snap_distance: float
- turn_restrictions_csv: string path
- candidate_k: integer (default 5)
- search_radius: float
- heading_field: string
- speed_field: string

## Outputs

1. matched_route
- line vector representing inferred network path(s)
- attributes: trajectory_id, total_cost, matched_points, unmatched_points

2. matched_points
- point vector with map-match diagnostics per input point
- attributes:
  - input_point_id
  - matched_edge_id
  - projected_measure
  - offset_distance
  - match_confidence
  - status (matched/unmatched/ambiguous)

3. match_report
- JSON summary:
  - match_rate
  - mean_offset
  - median_offset
  - ambiguous_count
  - disconnected_segments_count

## Algorithm (v1)

MVP approach:
1. Candidate generation:
- For each input point, find candidate edges within search radius.
- Keep top K by offset distance.

2. Transition scoring:
- For consecutive timestamps, estimate plausible transition cost over network.
- Penalize transitions that violate one-way/blocked/turn restrictions.

3. Sequence inference:
- Use dynamic programming (Viterbi-style) over candidate states to pick highest-likelihood path.

4. Geometry reconstruction:
- Build network path between chosen candidates.
- Emit matched route and projected point positions.

## v1 Constraints

- Primary focus: single trajectory per run.
- Assumes trajectory points are pre-sorted by timestamp.
- No lane-level matching in v1.
- No multipath confidence envelope in v1.

## Validation And Error Handling

Hard errors:
- Missing timestamp field
- Non-point trajectory geometry
- Invalid network geometry type

Soft diagnostics:
- points with no candidate edges
- transitions requiring disconnected subnetworks
- high-offset matches above threshold

## Acceptance Criteria

1. Synthetic trajectory tests:
- clean on-network trajectory matches expected route
- sparse points still reconstruct path where feasible
- one-way restriction violations avoided

2. Robustness tests:
- partial unmatched segments still produce output with diagnostics
- confidence decreases with increasing offset/noise

3. Interop:
- output compatible with existing route-event and network diagnostics workflows.
