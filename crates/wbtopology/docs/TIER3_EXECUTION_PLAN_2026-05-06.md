# Tier 3 Gaps Execution Plan (G–J)
## Building wbtopology into a Topology Powerhouse

**Date:** May 6, 2026  
**Scope:** Complete remaining 4 gaps to achieve production-grade topology parity with GEOS/JTS  
**Investment Level:** High (major features + refactoring)  
**Expected Timeline:** 3–4 intensive sessions  

---

## Strategic Execution Order

### Phase 1: Quick Wins (Unlocks Workflows)
**Gap I: Offset-Curve API** — 1.5 sessions  
- Extract one-sided offset logic from existing buffer pipeline
- Minimal new algorithm; mostly extraction + API wrapping
- Unblocks road edge extraction and centreline workflows
- Medium risk, medium effort, high value

### Phase 2: Foundation (Relates All Operations)
**Gap G: Full DE-9IM Implementation** — 2–3 sessions (may split into sub-phases)  
- Replace conservative scaffold with full arrangement traversal
- Affects spatial relations for complex geometries (touches, covers, crosses, overlaps, etc.)
- Highest impact on correctness for advanced workflows
- Very high risk/complexity; recommend Phase 2a (audit + optimize known predicates) before full Phase 2b (full traversal)

### Phase 3: Performance Optimization (Polish)
**Gap J: Continuous-Ring Buffer Curves** — 1.5–2 sessions  
- Optimize segment-by-segment noding to single-pass ring walk
- Expected 2–5x speedup for large polygons; affects every buffer operation
- High refactoring risk; only after G/I locked down
- High complexity; requires extensive validation

### Phase 4: Robustness (Optional, Deferred)
**Gap H: Robust Predicates** — Deferred until failure cases arise  
- Pervasive f64 → exact arithmetic retrofit
- High risk of performance regression
- Most workflows unaffected; only needed for near-degenerate inputs
- Recommendation: Skip unless clear failures emerge during later testing

---

## Gap I: Offset-Curve API (SESSION 1, NEXT UP)

### Overview
Extract one-sided offset curve generation from buffer logic into standalone `offset_linestring` function.

### Current State
- Buffer pipeline in `constructive.rs` builds symmetric closed polygons
- Internal offset-curve generation happens in `build_polygon_buffer_curve_set`
- No public API for one-sided offset
- Road edge extraction requires workarounds

### Target API
```rust
pub enum CapStyle {
    Butt,    // No end cap
    Square,  // Extended perpendicular
    Round,   // Rounded arc
}

/// Generate a one-sided offset curve from a linestring
/// Returns the offset as an open linestring (no closing point)
pub fn offset_linestring(
    linestring: &LineString,
    distance: f64,
    join_style: JoinStyle,
    quadrant_segments: usize,
    cap_style: CapStyle,
) -> Result<LineString, TopologyError>
```

### Implementation Steps
1. **Audit existing offset logic** in `build_polygon_buffer_curve_set` (lines ~360–450)
2. **Extract offset-curve builder** into new function `offset_single_linestring_to_curve`
3. **Add end-cap logic**: Butt (straight), Square (extend 90°), Round (arc)
4. **Create public wrapper** `offset_linestring` with error handling
5. **Unit tests**: Open linestring with all cap styles, intersecting offsets, reversed input
6. **Documentation**: Use cases (road edges, hydration buffers), limitations (self-intersecting curves)

### Estimated Effort
- Code extraction: 1–2 hours
- End-cap implementation: 1–2 hours
- Testing + documentation: 1 hour
- **Total: 3–5 hours (0.5 session)**

### Risk Level
🟡 Medium (code extraction + new public API)

### Success Criteria
- `cargo test -p wbtopology` passes all tests
- New `offset_linestring` compiles with no warnings
- Example: buffer a road linestring, extract left edge with `offset_linestring(..., distance=width/2, cap=Butt)`
- Docstring includes use-case examples

---

## Gap G: Full DE-9IM Implementation (SESSIONS 2–3)

### Overview
Replace incomplete spatial relations scaffold with full Dimensionally Extended 9-Intersection Matrix (DE-9IM) traversal.

### Current State
- `relate.rs` builds conservative matrix: fills boundary/exterior conservatively
- Named predicates (`touches`, `covers`, `within`, `overlaps`, `crosses`) derived from incomplete matrix
- Mostly works for simple/non-degenerate cases
- Fails for complex overlapping geometries, edge-on-face cases, near-tangency

### Problem Examples
- Polygon A partially overlaps Polygon B; edge of B touches but doesn't cross A's boundary → `crosses` may misclassify
- Point on boundary of polygon; `touches` should be true but might be classified as `boundary` instead
- Complex polygon with self-touching boundaries; relate matrix incomplete

### Target: Full DE-9IM Matrix
```
         I(A)  B(A)  E(A)
I(B)   [ ?     ?     ?  ]
B(B)   [ ?     ?     ?  ]
E(B)   [ ?     ?     ?  ]
```
Each cell = max dimension of intersection:
- 0 = point, 1 = line, 2 = area, F = empty

### Implementation Plan: Two Phases

#### Phase 2a: Audit & Quick Wins (1 session)
1. Audit current predicates (`intersects`, `disjoint`, `touches`, `covers`, `within`, `overlaps`, `crosses`)
2. Test each against known failure cases
3. Document which predicates are robust vs. conservative
4. Optimize stable predicates (e.g., `intersects` is usually correct)
5. Add warnings to docstrings for unreliable predicates
6. **Output:** Clear matrix of what works/doesn't work; decision point for full Phase 2b

#### Phase 2b: Full Arrangement Traversal (1–2 sessions)
1. Build full boundary/interior/exterior classification for both input geometries
2. Walk topology graph to compute all 9 cells of DE-9IM matrix
3. Implement named predicates from complete matrix (automatic correctness)
4. Comprehensive test suite vs. GEOS reference implementation
5. **Output:** Production-ready spatial relations

### Estimated Effort
- Phase 2a (audit): 3–4 hours
- Phase 2b (full implementation): 6–8 hours
- **Total: 10–12 hours (1.5–2 sessions)**

### Risk Level
🔴 Very High (foundational algorithm; pervasive impact)

### Success Criteria (Phase 2b)
- All 9 DE-9IM cells correctly computed for arbitrary geometry pairs
- All named predicates match GEOS output on reference test suite
- `cargo test -p wbtopology` passes all tests
- Docstring includes DE-9IM matrix definition and examples
- Performance regression <10% on typical operations

---

## Gap J: Continuous-Ring Buffer Curves (SESSION 3)

### Overview
Optimize buffer curve generation from segment-by-segment to single-pass ring traversal.

### Current State
- `build_polygon_buffer_curve_set` buffers each segment independently → ~N raw curves for N segments
- Each segment produces separate offset curve segment
- Joins handled by noding; can produce redundant intersections
- Works correctly but inefficient for large rings

### Target Optimization
- Walk ring once, building single continuous offset curve with embedded joins
- Handles join styles (Mitre, Bevel, Round) during traversal
- Produces 1–2 offset curves instead of N
- Expected 2–5x reduction in noded edges

### Implementation Steps
1. **Profile existing approach**: Measure noding time for 1000-segment polygon
2. **Design continuous walker**: Ring traversal with join detection/generation
3. **Implement offset walk**: Build curve incrementally with joins at each vertex
4. **Handle edge cases**: Nearly-collinear segments, extreme join angles, self-intersecting output
5. **Validate correctness**: Topology must match old approach
6. **Benchmark**: 2–5x fewer noded edges expected
7. **Test suite**: Large polygons, all join styles, corner cases

### Estimated Effort
- Profiling + design: 2–3 hours
- Implementation: 4–5 hours
- Validation + testing: 2–3 hours
- **Total: 8–11 hours (1.5–2 sessions)**

### Risk Level
🔴 Very High (core algorithm refactor; extensive validation needed)

### Success Criteria
- Continuous curve approach produces topologically identical output to old approach
- Benchmark shows 2–5x speedup on large polygon buffer
- `cargo test -p wbtopology` passes all tests
- No regressions in existing buffer tests
- Handles all join styles correctly

---

## Gap H: Robust Predicates (DEFERRED)

### Why Defer
- Most workflows unaffected by f64 precision
- Only impacts near-degenerate inputs (nearly-collinear, near-parallel segments)
- Pervasive change with high risk of performance regression
- Recommendation: Wait for concrete failure cases from real workflows
- If inputs come from CAD files (higher precision issues), revisit then

### Contingency Plan
If tests reveal precision failures:
1. Integrate `robust` crate for orientation predicates
2. Retrofit `segment_intersection_point` in `noding.rs`
3. Benchmark to measure overhead
4. Expand to other critical paths if acceptable

---

## Commit Strategy

Each gap will be committed in **logical sub-commits**:

### Gap I: Offset-Curve API
```
1. Extract offset logic to internal helper function
2. Add CapStyle enum and end-cap logic
3. Create public offset_linestring wrapper
4. Add unit tests and documentation
```

### Gap G: Full DE-9IM
```
[Phase 2a]
1. Audit current predicates; add test matrix
2. Document robust vs. conservative classifications
3. Add warnings to unreliable predicates

[Phase 2b, if proceeding]
1. Implement full boundary/interior/exterior classification
2. Build arrangement traversal for DE-9IM matrix
3. Reimplement named predicates from matrix
4. Comprehensive test suite
```

### Gap J: Continuous-Ring Buffer
```
1. Profile existing segment-by-segment approach
2. Design and implement continuous walker
3. Validate topological equivalence
4. Benchmark and optimize
```

---

## Decision Point: Recommended Next Action

**Recommendation:** Proceed with **Gap I first** (Offset-Curve API)

### Rationale
✅ Unlocks concrete workflows immediately (road edge extraction)  
✅ Medium risk, proven extraction pattern  
✅ ~0.5 session investment for quick momentum  
✅ Success builds confidence for larger Gap G/J refactors  
✅ No blocking dependencies; can start immediately  

### Then Proceed with Gap G (Phase 2a audit first), then Gap J

---

## Notes

- All gaps assume wbtopology codebase is in clean state post-commit
- `cargo check -p wbtopology` should pass before each gap starts
- Each gap's success criteria includes full test pass
- If any gap reveals architectural issues, pause and reassess
- User is welcome to interleave gaps based on real-world blockers
