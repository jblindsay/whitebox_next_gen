# vector buffer_vector: Postmortem and Future Roadmap

**Date:** 2026-05-07  
**Status:** Suspended — re-hidden from public frontends on 2026-05-11 pending a proven architectural rewrite  
**Author:** Development session notes

**Latest Audit Delta (2026-05-11):**
- Confirmed and fixed one systemic graph issue: coincident undirected segment multiplicity was being collapsed during graph build.
- Graph multiplicity GEOS/JTS parity probe now passes as an active (non-ignored) test.
- Remaining parity divergences are narrowed to downstream stages (overlay overlap area loss and BufferOp duplicate-coincident-line dissolve mismatch).

---

## What Was Built

A working implementation of vector buffering (`buffer_vector`) was completed for the Whitebox Next Gen
platform over approximately one work week (early May 2026). The tool correctly generates:

- Round, flat, and square cap styles for line features
- Round, bevel, and mitre join styles
- Per-quadrant arc resolution control
- Configurable mitre limit
- Optional dissolve of overlapping buffers into a unified output polygon

Non-dissolve buffering (each feature gets its own output polygon) is functionally correct and runs in
acceptable time for moderate datasets.

The implementation lives in:
- `crates/wbtools_oss/src/tools/gis/mod.rs` — `BufferVectorTool`, `BufferCapChoice`, `BufferJoinChoice`,
  and all helper geometry-collection functions
- `crates/wbtopology/src/constructive.rs` — offset curve and polygon buffer primitives
- `crates/wbtopology/src/overlay.rs` — unary union / dissolve backend

---

## What Failed

**Dissolve buffering is unacceptably slow.**

Benchmark comparison on a real-world municipal streets dataset (StreetCentrelines.shp):

| Tool | Non-dissolve | Dissolve |
|------|-------------|---------|
| QGIS / GEOS | ~1.48 s | ~1.80 s |
| wbtopology (best achieved) | ~1.55 s | killed after >10 min |

The dissolve cost in GEOS is **0.32 seconds** on this dataset. Every approach we tried failed to close
this gap in any meaningful way.

---

## Approaches Tried and Why They Failed

### 1. Cascaded pairwise polygon union
Buffer every feature into a polygon, then union all polygons together using a cascade merge strategy
(merge smallest pairs first, progressively building larger merged polygons).

**Why it failed:** On a large dense network (thousands of features), the number of pairwise intersection
tests is enormous. Even with an STR-tree spatial index to prune non-overlapping candidates, the work
on touching/overlapping features dominates. The cascaded merge loop still spends huge amounts of time
on the largest connected components.

### 2. Graph-driven union with connected-component decomposition
Decompose the full polygon set into connected components first (using a spatial graph), then dissolve
each component independently using a face-ring assembly approach.

**Why it failed:** Face assembly from a set of already-built polygons still requires noding all those
polygons' rings against each other — which is effectively the same expensive topology work. We were
just reorganizing expensive work, not eliminating it.

### 3. Line fast path: collect offset curves → polygonize → union
For line input only, skip individual per-feature polygon building. Instead collect raw offset curves
from all features in parallel, then polygonize all curves at once, then union the resulting polygons.

**Why it failed:** The final union step over thousands of polygonized faces is still the bottleneck.
Polygonization itself introduces additional overhead (global noding of all curves). And the union
step is not eliminated — we just moved it later in the pipeline.

### 4. Various parallelisation improvements
Rayon parallel iterators, partitioned chunk merges, etc.

**Why it helped but wasn't enough:** Non-dissolve benefited significantly. But for dissolve, the
sequential topology-build at the end is the bottleneck, and it cannot be parallelised with the current
architecture because it operates over shared global geometry state.

---

## The Root Cause (Identified Too Late)

The entire approach is wrong at the architectural level.

Our pipeline is:
```
for each feature:
    offset curves → close into polygon
union all polygons together → dissolved result
```

GEOS's `BufferOp` pipeline is:
```
for each feature:
    generate raw offset curves (do NOT close into polygon)
merge all raw curves into one global line collection
node all curves globally in one pass (MCIndexNoder)
build planar graph from noded linework (PlanarGraph)
label faces by buffer depth (BufferSubgraph)
extract outer-boundary faces → dissolved result
```

**There is no union step in GEOS.** The dissolve is essentially free because it is a byproduct of
the face-labeling pass over the already-built planar graph. The graph is built once from all curves
simultaneously, so every feature's contribution is incorporated during global noding rather than as
a separate merge operation later.

The cost difference is not about algorithm constants or data structures. It is about the number of
fundamental operations:

- Our approach: O(N²) pairwise intersection tests (even with spatial pruning) to dissolve N polygons
- GEOS approach: O(N·k) noding work where k is average segment count per feature, done once globally

For a dense road network with thousands of features and many overlapping buffers, this is a difference
of many orders of magnitude.

---

## Why This Was Not Identified Sooner

In hindsight, the signal was always there: GEOS's dissolve cost is 0.32 s more than non-dissolve on
this dataset. That near-zero marginal cost of dissolving is only possible if dissolve is not an
independent step — it must be inherent to the topology build. We interpreted the GEOS source code
correctly multiple times during audits, and **a GEOS-like face-walk implementation was attempted in
2026-05 modeled directly on GEOS's BufferOp architecture**. This attempt also failed with the same
signature problems: performance degradation (15–90s vs expected <2s) and inaccurate output.

This indicates the problem is not architectural misunderstanding. Instead, it points to a deeper
issue in graph construction, face labeling, or ring assembly that affects even a correctly-modeled
approach. The fact that four independent implementations (cascaded union, connected-component
decomposition, line fast path, and GEOS-like face-walk) all produced identical failure signatures
suggests either:
1. A systematic bug in wbtopology's graph or noding primitives
2. A subtle incompleteness in the algorithm design that survives correct architectural modeling
3. An implementation detail that silently corrupts output in a way that is hard to debug without
   side-by-side GEOS reference traces

### 2026-05-11 Audit Update (GEOS/JTS-Parity Deep Dive)

Follow-up parity probes were added and run across `graph`, `noding`, `buffer_op`, and `overlay`.
One previously suspected systemic issue is now confirmed:

- **Confirmed and fixed:** graph build was collapsing coincident undirected segment multiplicity.
   This violated GEOS/JTS-style depth accounting assumptions. The deduplication logic was removed,
   and a parity test for coincident multiplicity now passes as an active (non-ignored) test.

Subsequent fixes in the same audit cycle addressed the initially failing parity probes:

- **Resolved:** overlay unary union parity fixtures (touching/disjoint/overlap/containment/partial-overlap)
   now pass and have been promoted from ignored to active regression tests.
- **Resolved:** BufferOp duplicate coincident source-line area parity now passes and has been promoted
   from ignored to active regression tests.

Interpretation: this is no longer just a "known divergence" state. Multiple concrete graph/overlay/buffer
defects were identified and fixed under GEOS/JTS-oriented probes. The vector pipeline is materially more
trustworthy than at suspension time, but performance parity and broader real-world benchmarking still need
separate validation before any frontend re-exposure decision.

---

## The Path Forward: Two Remaining Options

The GEOS-like rewrite approach was attempted and failed to produce correct output. Two paths remain:

### Option A: Deep Debugging of Graph/Noding Primitives

The GEOS-like attempt failed at the face-walk stage (15–90s, inaccurate output). Before abandoning
the vector approach entirely, investigate whether wbtopology's graph or noder has subtle bugs or
incompleteness that cascade into corrupted output.

**Required work:**
- Add extensive test coverage comparing wbtopology's noding output against GEOS's noding on identical input
- Trace face-walk logic with actual geometric data, not just algorithm flow
- Validate that half-edge graph construction is correct under all ring topology scenarios
- Compare BufferSubgraph face-labeling logic against GEOS reference traces

**Risks:**
- May consume 5–10 days of careful debugging without guarantee of success
- If bugs are found and fixed, may still not achieve <2s performance due to architectural differences
- Even with resolved parity probes, latent edge cases may remain in complex real-world topology workloads
   not yet covered by differential fixture corpora

**Worth pursuing only if:**
- You have time for careful debugging and reference comparison against GEOS
- You are committed to understanding why even a correctly-modeled approach failed
- You want to preserve the "pure vector, exact geometry" path for future use

---

## Alternative: Raster-Guided Vector Buffering

Discovered 2026-05-11 during architectural reflection: a hybrid raster/vector approach may be viable
and offers certain advantages over a pure GEOS-like vector rewrite.

### Concept

1. Rasterize input geometries at high DPI within bounding box
2. Compute distance transform via separable convolution
3. Threshold at buffer distance → binary raster
4. Run connectivity analysis to identify holes
5. Extract boundary using marching squares → vector rings
6. Return as MultiPolygon with holes

### Why This Works

| Aspect | Pure Vector (current) | GEOS-like Rewrite | Raster-Guided Hybrid |
|--------|----------------------|-------------------|----------------------|
| Non-dissolve buffers | ✅ Works | ✅ Works | ✅ Works |
| Dissolve/union | ❌ Fails (O(N²)) | ✅ Works (O(N·k)) | ✅ Works (O(pixels)) |
| Hole detection | ❌ Complex | ✅ Via face labels | ✅ Trivial (connectivity) |
| Exactness | ✅ Exact | ✅ Exact | ⚠️ Bounded discretization |
| Performance | ❌ Slow | ✅ Fast | ✅ Fast |

### Why Raster-Guided Is Now the Stronger Path

Given that even a correctly-modeled GEOS-like implementation failed with performance and correctness
issues, the raster-guided approach has distinct advantages:

1. **Avoids the tricky part**: Graph construction, face-labeling, and ring assembly were the failure
   point across all vector attempts. The raster approach sidesteps these entirely.

2. **Independently testable pieces**:
   - Distance transform (decades-old image-processing standard)
   - Connectivity analysis (textbook graph algorithm)
   - Marching squares (well-understood boundary extraction)
   
   Each component can be validated against reference implementations without depending on the others.

3. **Clearer error diagnosis**: If output is wrong, you can inspect the distance raster, connectivity
   labels, and extracted boundaries independently. Graph corruption in a half-edge implementation is
   much harder to debug.

4. **Precision is explicit**: DPI-to-error trade-off is measurable and configurable. No hidden precision
   thresholds or algorithm-dependent robustness factors. Users can reason about DPI choices the same
   way they reason about coordinate precision in any GIS workflow.

### Recommendation

**Raster-guided approach is now the priority path** for future buffer implementation because:
- Four independent vector approaches all failed with identical signatures
- The GEOS-like architecture, while correct in design, also produced corrupted output (15–90s, inaccurate)
- The raster approach achieves the same goals (dissolve, hole detection, exact geometry) with simpler,
  independently-testable components
- No hidden precision thresholds: DPI parameter is explicit, measurable, and under user control

The raster approach is not a compromise—it's the clearer engineering solution given what we've learned.

That said, the 2026-05-11 parity audit reduced uncertainty: one systemic graph bug was found and
corrected. If vector buffering is revisited, continue from the current GEOS/JTS differential harnesses
rather than restarting from first principles.


---

## Current Status of the Tool

The `buffer_vector` tool remains registered in the wbtools_oss registry and all existing code is
preserved. It has been **removed again from the public-facing frontends** after the 2026-05-11
reopen attempt failed to produce trustworthy output:

- Removed from `wbw_python/tool_taxonomy.toml` (nested accessor `wbe.vector.geometry_processing.buffer_vector` no longer exists)
- Removed explicit Python method from `wb_environment.rs`
- Removed from Python `.pyi` stubs and frontend docs/examples
- Removed from both R generated wrapper files and R manual examples
- Removed from QGIS plugin recipes/help and tool taxonomy

The underlying implementation remains available only for internal investigation. It should not be
re-exposed until there is a proven, reference-validated implementation with correct output on real
datasets.

### Reopen Reminder

If vector buffering is revisited in the future, treat it as a ground-up algorithm problem rather than
an incremental patch task. Do not re-open the frontend surface until there is:

- a reference comparison against GEOS/JTS on real datasets,
- output validation beyond visual spot checks, and
- a reasoned explanation for why the architecture is expected to be correct before public exposure.

---

## Lessons Learned

1. **Audit the algorithm first, not the constants.** When a competitor runs a step in 0.32 s and yours
   takes 10+ minutes, the difference is not about STR-trees or rayon parallelism. It is about whether
   you are doing the same work at all.

2. **GEOS's near-zero dissolve cost is a diagnostic, not a curiosity.** A 0.32 s marginal cost for
   dissolving thousands of features in a dense road network can only mean dissolve is a face-labeling
   pass over an already-built structure, not a separate union operation. That observation should have
   driven a full architectural audit on day one.

3. **Incremental patches to the wrong architecture do not converge.** Every optimization we applied
   reduced constant factors but could not change the asymptotic behaviour, because the O(N²) pairwise
   union loop was the ceiling on performance regardless of how efficiently individual pieces ran.

4. **The existing correctness work is not wasted.** Offset curve generation, cap/join styles, arc
   approximation, and the non-dissolve per-feature path are all correct and will be reused directly
   in the `BufferOp` design.
5. **Architectural correctness is not sufficient for implementation success.** Understanding GEOS's
   algorithm correctly does not guarantee a correct Rust implementation. The identical failure signatures
   across different approaches (cascaded union, connected-component decomposition, line fast path, and
   GEOS-like face-walk) suggest the bugs may be in subtle areas (ordering of operations, coordinate
   precision interactions, or edge cases in noding) that survive high-level architectural reviews.

6. **Empirical data trumps theoretical analysis.** The GEOS dissolve cost of 0.32s (near-zero marginal)
   was correctly interpreted as "dissolve is not a separate step." But this observation alone did not
   lead to a working implementation. The actual problem revealed by four failures is deeper than
   architectural choice.

---

## Sprint Restart Pointer (2026-05-12)

For the next wbtopology hardening session, use this file as the canonical restart handoff:

- `crates/wbtopology/docs/WBTOPOLOGY_SPRINT_RESTART_STARTING_POINT_2026-05-12.md`
