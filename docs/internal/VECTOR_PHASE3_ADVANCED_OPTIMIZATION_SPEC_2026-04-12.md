# Vector Phase 3 Advanced Optimization Spec

Date: 2026-04-12
Phase: 3
Area: Optimization solvers (CVRP, VRPTW, Pickup/Delivery)

## Objectives

1. Introduce production-usable routing optimization primitives beyond shortest path.
2. Provide deterministic and auditable optimization behavior for repeated runs.
3. Return rich diagnostics that explain feasibility and solution quality.

## MVP Tool Set

1. `vehicle_routing_cvrp`
2. `vehicle_routing_vrptw`
3. `vehicle_routing_pickup_delivery`

## Shared Input Contracts

- Required baseline inputs:
  - `network` (line network layer)
  - `depot_points` (one or more depots)
  - `stop_points` (demand or pickup/dropoff stops)
- Optional controls:
  - `cost_field`, `speed_field`, `turn_restrictions_csv`
  - `seed`, `max_iterations`, `time_limit_seconds`
  - `solver_mode` (`strict`, `relaxed`)

## CVRP Contract (MVP)

Inputs:
- `vehicle_capacity_field` or scalar `vehicle_capacity`
- `demand_field`

Outputs:
- `routes_output`: line features with route metrics
- `unserved_output` (optional): unserved stops with reasons
- `summary_json`: global objective and feasibility metrics

Diagnostics fields:
- `route_id`, `vehicle_id`, `load_total`, `distance_total`, `cost_total`, `stop_count`, `status`

## VRPTW Contract (MVP)

Additional inputs:
- `tw_start_field`, `tw_end_field`, `service_time_field`

Additional diagnostics:
- `arrival_time`, `service_start`, `lateness`, `window_violation_flag`

## Pickup/Delivery Contract (MVP)

Additional inputs:
- `pair_id_field`
- `pickup_dropoff_type_field`

Additional diagnostics:
- `precedence_ok`, `pair_status`, `orphan_pair_flag`

## Algorithm/Runtime Notes

- Determinism: seed-controlled randomized steps must be reproducible.
- Failure policy: infeasible scenarios must return structured diagnostics, not silent fallback paths.
- Performance target (MVP): medium-size municipal/logistics datasets should complete in bounded runtime with explicit quality metadata.

## Validation Plan

1. Unit tests for contract validation and deterministic seeding.
2. Integration tests for feasible and infeasible CVRP/VRPTW/PD scenarios.
3. Golden-result fixtures for route-level metrics.
4. Wrapper parity tests across Rust/Python/R tool discovery.

## Out of Scope (Phase 3 MVP)

- Full exact solver parity with enterprise OR suites.
- Stochastic demand forecasting integration.
- Real-time re-optimization streaming.
