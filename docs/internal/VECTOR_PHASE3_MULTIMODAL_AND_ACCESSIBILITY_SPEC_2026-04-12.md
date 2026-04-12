# Vector Phase 3 Multimodal and Accessibility Spec

Date: 2026-04-12
Phase: 3
Area: Multimodal routing, centrality, accessibility, OD uncertainty

## Objectives

1. Support mode-aware routing with transfer penalties.
2. Provide network centrality and accessibility analytics for planning use cases.
3. Add uncertainty and sensitivity controls for OD analysis outputs.

## MVP Tool Set

1. `multimodal_shortest_path`
2. `multimodal_od_cost_matrix`
3. `network_centrality_metrics`
4. `network_accessibility_index`
5. `od_sensitivity_analysis`

## Multimodal Graph Contract

Input layers:
- `network` (base graph edges)
- `transfer_nodes` (optional explicit transfer points)

Required attributes:
- `mode` (walk/drive/transit/bike/etc.)
- `cost` and optional `time`

Controls:
- `mode_sequence` or `allowed_modes`
- `transfer_penalty`
- `mode_switch_penalty_table` (optional)

## Centrality Metrics (MVP)

Minimum metrics:
- degree centrality
- closeness centrality
- betweenness centrality (baseline implementation)

Output fields:
- `degree_c`, `closeness_c`, `betweenness_c`, `component_id`

## Accessibility Index (MVP)

Inputs:
- `origins`, `destinations`
- impedance and cutoff/decay options

Outputs:
- origin-side accessibility score
- optional destination pressure/load indicators

## OD Sensitivity Analysis (MVP)

Parameters:
- perturbation ranges for impedance fields
- optional Monte Carlo sample count (bounded)

Outputs:
- baseline OD metrics
- percentile/variance summary
- sensitivity attribution by parameter

## Validation Plan

1. Integration fixtures for walk-drive and walk-transit routing.
2. Reproducibility checks for deterministic mode constraints.
3. Sanity checks for centrality metrics on known graph motifs.
4. Accessibility and OD-sensitivity regression snapshots.

## Wrapper and Docs Expectations

- Rust: full manifest + registry + integration tests.
- Python: dynamic runtime exposure + cookbook examples.
- R: regenerated wrappers and package scaffold parity.

## Out of Scope (Phase 3 MVP)

- Transit schedule/headway simulation depth.
- Full agent-based demand assignment.
- Real-time multimodal congestion feedback loops.
