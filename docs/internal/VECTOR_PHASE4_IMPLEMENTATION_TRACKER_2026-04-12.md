# Vector Phase 4 Implementation Tracker

Date: 2026-04-12 (Created 2026-04-12)
Phase: 4 (Advanced Solvers, Batch Analytics, and Temporal Robustness)
Status: In Progress (Stream A underway; VRPTW refinement next)

## Scope Anchors

Phase 4 planned outcomes:
1. Advanced routing solvers beyond greedy baselines.
2. Batch and matrix workflows for multimodal and OD analytics.
3. Temporal and robustness-aware routing extensions.
4. Performance scaling improvements for large graph workflows.

## Phase 4 Candidate Deliverables

- Metaheuristic optimization upgrades for vehicle routing.
- Batch multimodal path computation and OD matrix outputs.
- Temporal edge-cost models and time-aware sensitivity analysis.
- Performance improvements for snapping, graph traversal, and repeated OD computation.

## Work Breakdown

### Stream A: Advanced Vehicle Routing Solvers
- [x] Add local-improvement stage for CVRP route optimization.
- [ ] Evaluate Tabu Search or Simulated Annealing for route refinement.
- [x] Improve VRPTW insertion heuristics and infeasibility recovery.
- [x] Add comparative benchmarks against Phase 3 greedy baselines.

### Stream B: Batch Multimodal and Matrix Analytics
- [ ] Add multi-origin / multi-destination multimodal shortest-path runs.
- [ ] Support OD matrix outputs for multimodal accessibility workflows.
- [ ] Add batch result formats suitable for downstream analytics.
- [ ] Add regression tests for repeated and batched path queries.

### Stream C: Temporal Robustness and Scenario Modelling
- [ ] Extend multimodal and OD tools with time-dependent edge profiles.
- [ ] Add scenario bundles for peak / off-peak / disruption analysis.
- [ ] Support comparative outputs across scenarios in a single run.
- [ ] Add benchmark fixtures for temporal routing and uncertainty workflows.

### Stream D: Performance and Scalability Hardening
- [ ] Add spatial indexing for snapping origins/destinations to networks.
- [ ] Reduce repeated shortest-path cost where multi-query reuse is possible.
- [ ] Evaluate parallel execution for OD sensitivity and accessibility batches.
- [ ] Publish large-network benchmark reports and runtime targets.

### Stream E: Wrapper UX and Cookbook Expansion
- [ ] Add optional high-level convenience methods in Python where justified.
- [ ] Expand R and Python cookbooks with Phase 4 scenarios.
- [ ] Regenerate any wrappers or stubs needed for new public APIs.
- [ ] Finalize Phase 4 regression gate commands and release checklist.

## Suggested Execution Order

1. Advanced CVRP / VRPTW solver refinement.
2. Batch multimodal and OD matrix workflows.
3. Temporal and scenario-aware routing.
4. Performance scaling and parallelization.
5. Wrapper UX and cookbook closeout.

## Exit Criteria

- Vehicle routing tools materially improve over Phase 3 greedy baselines on benchmark scenarios.
- Multimodal and OD analytics support batch and matrix workflows.
- Temporal scenario analysis is available for routing and sensitivity workflows.
- Large-network performance targets are benchmarked and documented.
- Runtime surfaces and cookbook coverage remain aligned with new APIs.

## Initial Focus Recommendation

Recommended first implementation target:
- Stream A: Add a local-improvement optimization pass for `vehicle_routing_cvrp`.

Rationale:
- Lowest-risk Phase 4 upgrade.
- Directly improves an existing production tool.
- Easy to benchmark against current deterministic baseline.
- Creates a reusable pattern for later VRPTW and pickup/delivery improvements.

Next implementation target:
- Stream A: Improve `vehicle_routing_vrptw` stop selection with a feasible-candidate scoring heuristic that prioritizes lateness pressure before raw nearest-neighbour distance.

Reasoning:
- Current VRPTW baseline still uses nearest-neighbour candidate choice once feasibility is checked.
- A better scoring rule should improve route quality without requiring a full solver rewrite.
- This is the most direct follow-on from the CVRP local-improvement work.

## Progress Log

- 2026-04-12: Phase 4 tracker created after Phase 3 completion.
- 2026-04-12: Initial streams drafted from Phase 3 closeout findings and known limitations.
- 2026-04-12: First target selected: CVRP local-improvement optimization pass.
- 2026-04-12: **STREAM A CVRP LOCAL OPTIMIZATION PASS IMPLEMENTED**
	- Extended `vehicle_routing_cvrp` with `apply_local_optimization` (default: `true`).
	- Added deterministic 2-opt local search on each constructed route after greedy assignment.
	- Preserved Phase 3 baseline behavior by allowing `apply_local_optimization = false` for regression comparison.
	- Added runtime outputs: `apply_local_optimization`, `optimized_route_count`, and `total_distance`.
	- Added regression coverage: `vehicle_routing_cvrp_local_optimization_reduces_route_distance`.
	- Validation commands:
		- `cargo test -p wbtools_oss --test registry_integration vehicle_routing_cvrp_local_optimization_reduces_route_distance -- --nocapture` (PASS)
		- `cargo check -p wbtools_oss` (PASS)
	- Added comparative benchmark coverage: `vehicle_routing_cvrp_benchmark_local_optimization_outperforms_phase3_greedy_baseline`.
	- Benchmarked optimized two-route total distance against the Phase 3 greedy baseline using `apply_local_optimization = false` as the baseline control.
- 2026-04-12: Reviewed `vehicle_routing_vrptw` Phase 3 baseline to select the next Stream A refinement target.
	- Confirmed current candidate selection is still nearest-neighbour after time-window feasibility filtering.
	- Selected next implementation target: feasible-candidate scoring heuristic for VRPTW.
- 2026-04-13: **STREAM A VRPTW PRIORITY SCORING HEURISTIC IMPLEMENTED**
	- Extended `vehicle_routing_vrptw` with `use_priority_scoring` (default: `true`).
	- Added deterministic feasible-candidate ranking by projected lateness, then projected slack, then travel distance, with nearest-neighbour baseline still available via `use_priority_scoring = false`.
	- Added runtime output echo: `use_priority_scoring`.
	- Added comparative benchmark coverage: `vehicle_routing_vrptw_benchmark_priority_scoring_reduces_total_lateness_vs_phase3_baseline`.
	- Benchmarked total lateness reduction versus the Phase 3 baseline control path.
