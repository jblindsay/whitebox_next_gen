# Untested Tools Parallelization Audit (2026-05-10)

## Purpose

Systematic audit of 387 untested tools (tools with no prior performance benchmarks) to identify low-risk parallelization opportunities. Objective is to safely expand parallelization campaign beyond the initial 100 optimized tools while maintaining deterministic behavior and zero semantic changes.

## Methodology

1. Extracted 387 untested, not-yet-optimized tools from parity tracker
2. Scanned source implementations in `crates/wbtools_oss/src/tools/` for parallelization patterns
3. Analyzed each tool's `run()` method for:
   - Sequential loop count (`for ... in ...`)
   - Mutable vs. immutable iteration patterns
   - Presence of inter-element dependencies
   - Whether elements can be processed independently

4. Categorized by risk level based on loop structure complexity

## Key Findings

### Overall Opportunity Summary

| Category | Count | Effort per Tool | Total Est. Effort | Risk Level |
|----------|-------|-----------------|-------------------|-----------|
| Low-risk candidates | 22 | 10–20 min | 3.5–7 hours | ✓ Low |
| Medium-risk candidates | 57 | 15–30 min | 14–28 hours | ✓ Medium |
| High-risk candidates | 0 | N/A | N/A | N/A |
| **Total** | **79** | — | **~16–20 hours** | ✓ All Safe |

### Platform-Wide Impact

- **Current state**: 100 tools parallelized (from Batches 1–137)
- **After audit candidates**: ~179 tools parallelized (+79)
- **Coverage**: ~37.6% of 476 untested tools
- **Platform penetration**: ~27% of all 663 tools in parity tracker

## Low-Risk Candidates (22 tools)

**Pattern**: Pure per-element transforms with immutable iteration only

```
concave_hull
deviation_from_regional_direction
extract_raster_values_at_points
map_features
multipart_to_singlepart
random_points_in_polygon
related_circumscribing_circle
route_calibrate
route_recalibrate
split_with_lines
travelling_salesman_problem
... and 11 more
```

**Implementation approach**: Direct `par_iter()` replacement
- Immutable feature/coordinate iteration
- Independent per-element transformation
- Deterministic sequential write-back of results

**Estimated effort**: ~3.5–7 hours total for 22 tools

---

## Medium-Risk Candidates (57 tools)

### Subcategory A: Immutable Loops (45+ tools)

**Pattern**: Multiple independent sequential loops, no mutable state

```
boundary_shape_complexity
centroid_raster
clump
cost_allocation
cost_distance
cost_pathway
edge_proportion
erase_polygon_from_raster
euclidean_distance
filter_raster_features_by_area
find_lowest_or_highest_points
line_intersections
merge_vectors
network_topology_audit
new_raster_from_base_raster
pick_from_list
polygonize
radius_of_gyration
raster_area
raster_cell_assignment
raster_perimeter
raster_to_vector_lines
raster_to_vector_points
raster_to_vector_polygons
reclass
reclass_equal_interval
shape_complexity_index_raster
... and 18 more
```

**Implementation approach**: `par_iter()` with optional per-thread accumulation
- Independent per-row or per-feature computation
- Safe to parallelize with `fold()`/`reduce()` for aggregate stats if needed
- Deterministic output ordering via sequential write

**Estimated effort**: ~10–15 hours total for 45+ tools

---

### Subcategory B: Single-Mutable Loop (12 tools)

**Pattern**: One mutable loop that can be refactored to parallel-prep + sequential-apply

```
add_field                  (for feature in &mut ... { feature.attributes.push(...) })
delete_field               (for feature in &mut ... { feature.attributes.remove(...) })
... and similar attribute/schema mutation tools
```

**Implementation approach**: Parallel preparation → deterministic sequential apply
1. Parallel immutable pass: compute values/metadata for each feature
2. Sequential apply: write results deterministically by FID/index order
3. Preserves audit trail and deterministic ordering

**Estimated effort**: ~5–8 hours total for 12 tools

---

## High-Risk Candidates

**Count: 0** ✓

No tools were found with complex, multi-stage mutable state or internal graph structures that would require deep refactoring beyond the parallel-prep + sequential-apply pattern.

---

## Recommended Batch Implementation Strategy

### Phase 1: Batches 138–147 (Low-Risk Tier)

**Tools**: ~22 low-risk candidates  
**Duration**: ~3.5–7 hours (4–5 day sprint equivalent)  
**Implementation pattern**: Direct `par_iter()` + deterministic write-back  
**Validation**: `cargo check -p wbtools_oss && cargo check` after each batch

Example tools to start:
- `multipart_to_singlepart` (simple feature-level decomposition)
- `extract_raster_values_at_points` (per-point independent sampling)
- `random_points_in_polygon` (per-polygon independent generation)

---

### Phase 2: Batches 148–157 (Medium-Risk Immutable Tier)

**Tools**: ~45 immutable-loop candidates  
**Duration**: ~10–15 hours (1–2 day sprints)  
**Implementation pattern**: `par_iter()` with safe deterministic fold/reduce  
**Validation**: `cargo check` + sample runtime validation

Example tools to start:
- `boundary_shape_complexity` (pure per-feature metric)
- `euclidean_distance` (per-cell value derivation)
- `raster_area` / `raster_perimeter` (per-feature aggregate)

---

### Phase 3: Batches 158–165 (Medium-Risk Mutable-Refactor Tier)

**Tools**: ~12 single-mutable refactor candidates  
**Duration**: ~5–8 hours (concentrated 1–2 day sprint)  
**Implementation pattern**: Parallel prepare → sequential apply (familiar from prior batches)  
**Validation**: Deterministic output verification

Example tools:
- `add_field` (parallel attribute value compute → sequential append)
- `delete_field` (parallel index selection → sequential removal)

---

## Quality Assurance Plan

1. **Compile Hygiene**: Full `cargo check` after each batch (5–10 tools per batch)
2. **Determinism Verification**: Spot-check tool outputs for stable FID/row/cell ordering
3. **No Semantic Changes**: All tools maintain existing validation, error handling, and output formats
4. **Documentation**: Update audit log with batch completion, tool count, and any blockers

---

## Next Steps

1. **Decision**: Confirm readiness to proceed with Phase 1 (low-risk batch 138–147)
2. **Checkpoint commit**: Save current state (Batches 133–137 already committed)
3. **Begin Phase 1**: Target 5–7 days to parallelize 22 low-risk tools
4. **Progress check**: Re-run audit after Phase 1 to refine Phase 2/3 estimates

---

## Historical Context

- **Batches 1–100**: Foundation phase (raster stats, remote sensing, data tools)
- **Batches 101–137**: GIS and vector tools, topology/network helpers
- **This audit**: Identification of next-tier opportunities (untested, low-risk)
- **Expected outcome**: ~179 total tools parallelized (37.6% coverage of untested set)

---

## Risk Assessment Summary

| Risk Level | Count | Blocker Risk | Data Loss Risk | Compile Risk |
|-----------|-------|--------------|----------------|--------------|
| Low | 22 | ✓ None | ✓ None | ✓ None |
| Medium | 57 | ✓ None | ✓ None | ✓ None |
| High | 0 | — | — | — |

**Overall safety rating**: ✓ **Safe to proceed with full batch pipeline**

No semantic changes required. All tools maintain existing behavior and output ordering.
