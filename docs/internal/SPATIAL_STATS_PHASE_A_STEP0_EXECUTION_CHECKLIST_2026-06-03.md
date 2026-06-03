# Spatial Stats Phase A - Step 0 Execution Checklist

Date: 2026-06-03
Scope: Build shared neighborhood/weights and diagnostics core before any Phase A stat tool implementation.

## Purpose

Step 0 exists to prevent duplicated logic, inconsistent assumptions, and late performance surprises across:
1. Global Moran's I
2. Local Moran's I (LISA)
3. Getis-Ord Gi/Gi*
4. Nearest-neighbour index
5. Quadrat count test

## Hard Constraints

1. Performance is first-order: no post-hoc optimization mindset.
2. Deterministic behavior by default.
3. No long-running benchmark or parity commands without explicit approval.
4. Shared core APIs must be stable before wrapper rollout.

## Deliverables

Step 0 is complete only when all deliverables below are done.

### D1. Shared Neighborhood Builder API

Implement one core module that supports:
1. Contiguity: queen and rook.
2. K-nearest neighbors.
3. Distance-band neighbors.

Required input contract:
1. Feature ids/index map.
2. Geometry source abstraction (point, polygon centroid, polygon topology).
3. Neighbor mode + params.
4. CRS distance assumptions flag.

Required output contract:
1. Neighbor list per feature.
2. Optional edge weights.
3. Stable ordering for deterministic downstream use.

### D2. Weight Standardization Layer

Implement row-standardization and raw-weight modes in one place.

Required output metadata:
1. Standardization mode.
2. Zero-row handling policy.
3. Effective row sums summary.

### D3. Diagnostics Object (Shared Across All Stats Tools)

Emit a common diagnostics payload from the core:
1. n_features
2. n_islands
3. neighbor_count_min
4. neighbor_count_mean
5. neighbor_count_max
6. connected_component_count
7. row_standardized
8. dropped_feature_count and reason counts

Diagnostics must be serializable to JSON without schema changes by individual tools.

### D4. Island and Invalid-Case Policy

Implement explicit policies (no silent fallback):
1. drop_with_warning
2. keep_zero_weight (when mathematically valid)
3. error

Policy decision must be present in runtime metadata.

### D5. Performance Baseline Hooks

Add lightweight timing/memory hooks for core operations:
1. index_build_ms
2. neighbor_build_ms
3. standardize_ms
4. total_core_ms

No heavy benchmark harness required in Step 0.

## Acceptance Tests (Cheap Only)

### T1. Determinism

1. Same input + params returns identical neighbor ordering and diagnostics over repeated runs.

### T2. Correctness Smoke Cases

1. Small polygon lattice: queen/rook neighbor counts match expected values.
2. Point set: k-nearest has fixed cardinality except explicit ties policy.
3. Distance-band: isolated points are detected correctly.

### T3. Policy Behavior

1. Island policy modes produce expected drop/keep/error outcomes.

### T4. Performance Sanity

Run lightweight synthetic checks only:
1. small: 1k features
2. medium: 10k features
3. large: 50k features

Pass criteria:
1. No pathological blow-up.
2. Runtime summaries emitted.

## Out of Scope for Step 0

1. Moran's I numeric kernel.
2. LISA per-feature significance classes.
3. Gi/Gi* computation.
4. NNI/quadrat statistical inference.
5. Wrapper/UI output formatting.

## Exit Criteria (Go/No-Go)

Proceed to Step 1 (Global Moran's I) only if:
1. D1-D5 are complete.
2. T1-T4 pass.
3. Diagnostics schema is frozen for Phase A.
4. No known O(n^2) hotspot remains in default path for 10k-50k workflows.

## Immediate Next Task After Step 0

Implement Global Moran's I using this core without adding any new neighborhood logic in the tool layer.
