# Vector Phase 1 Service-Area Ring Enhancements

Date: 2026-04-12
Phase: 1 (High-ROI Network Extensions)
Status: Initial spec

## Goal

Enhance service-area outputs for planning workflows with explicit multi-ring support and clearer polygonization controls.

## Current Baseline

Existing tool:
- network_service_area

Current output modes:
- nodes
- edges
- polygons

## Proposed Additions

1. Multi-ring thresholds
- New argument: ring_costs
- Type: comma-separated list or JSON array of positive costs
- Behavior: generate ring classification per reachable feature.

2. Ring labeling fields
- ring_index: integer ring ordinal (1..N)
- ring_min_cost: lower bound
- ring_max_cost: upper bound
- origin_id: source origin identifier

3. Polygonization controls
- polygon_method: convex_hull (v1 default), alpha_shape (future)
- min_ring_points: minimum points to emit polygon ring
- merge_origins: bool (default false)

4. Frontier diagnostics
- frontier_edges_count
- partial_edges_count
- ring_coverage_summary table

## Backward Compatibility

- If ring_costs is not supplied, behavior matches current max_cost pathway.
- Existing output_mode values stay unchanged.

## Acceptance Criteria

1. Ring correctness:
- features assigned to exactly one ring interval per origin.

2. Output determinism:
- stable ring assignment across repeated runs.

3. Existing tests remain passing:
- current service area tests must pass unchanged.

4. New tests:
- multi-ring nodes output
- multi-ring edges output
- multi-ring polygons output
- ring behavior with barriers and turn restrictions
