# vector buffer_vector: Postmortem and Future Roadmap

**Date:** 2026-05-07  
**Status:** Suspended — tool hidden from public APIs pending architectural rewrite  
**Author:** Development session notes

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
correctly multiple times during audits, but kept returning to incremental patches of the existing
union pipeline rather than committing to the necessary architectural break.

---

## The Path Forward

The only fix that will work is a from-scratch `BufferOp` implementation in `wbtopology` modelled
directly on GEOS's architecture. This is a non-trivial but well-understood piece of computational
geometry.

### High-Level Design

```
BufferOp::run(features, distance, options) -> Vec<TopoPolygon>
  1. Curve generation (parallel, per feature)
     - For each feature geometry, generate raw offset linework (open curves, no polygon closing)
     - Cap and join styles applied here
     - Output: Vec<TopoLineString> (all features combined)

  2. Global noding
     - Run MCIndexNoder (or equivalent snap-rounding noder) over ALL curves at once
     - Output: Vec<TopoLineString> (noded, fully split at intersections)

  3. Planar graph construction
     - Build directed half-edge graph from noded linework
     - Output: PlanarGraph

  4. Subgraph labeling
     - Traverse graph, label each face by buffer depth (inside/outside)
     - For dissolve: faces with depth >= 1 are "inside"
     - For non-dissolve: faces that belong to exactly one feature's buffer are "inside"
     - Output: face labels on PlanarGraph

  5. Ring extraction
     - Walk outer boundary rings of "inside" faces
     - Collect hole rings
     - Output: Vec<TopoPolygon>
```

### Key Prerequisite: MCIndexNoder in wbtopology

wbtopology already has a snap-rounding noder. Whether it is efficient enough at GEOS MCIndexNoder
scale (millions of segments) needs to be validated. MCIndexNoder uses an STR-tree to find candidate
crossing pairs before doing exact intersection tests — this is the O(N log N) noding that makes
GEOS fast.

### Key Prerequisite: Face Labeling / BufferSubgraph

This is the most novel piece relative to current wbtopology capability. GEOS uses `EdgeRing` traversal
with `depthDelta` tagging to propagate inside/outside labels across all faces. This logic needs to
be implemented cleanly in wbtopology's graph module before `BufferOp` can be completed.

### Scope Estimate

This is a 3–5 day focused implementation task assuming:
- The existing noder is reusable with minor extensions
- The planar graph (`wbtopology::graph`) is extended with face-label traversal
- The `BufferOp` orchestration layer is built on top

It should not be started piecemeal. It should be designed in one sitting and implemented front-to-back
before testing, because partial states of the pipeline are not testable in isolation.

### Recommended Starting Point

Read the following GEOS source files before writing a single line of Rust:
- `src/operation/buffer/BufferOp.cpp` — top-level orchestration
- `src/operation/buffer/BufferBuilder.cpp` — curve generation → noding → graph build
- `src/operation/buffer/BufferSubgraph.cpp` — face labeling
- `src/operation/buffer/OffsetCurveBuilder.cpp` — the actual geometry generation

These four files contain the complete GEOS buffer algorithm. Everything else (cap styles, join styles,
arc approximation) is detail that can be adapted from our existing `constructive.rs` implementation.

---

## Current Status of the Tool

The `buffer_vector` tool remains registered in the wbtools_oss registry and all existing code is
preserved. It has been **hidden from the public-facing APIs**:

- Removed from `wbw_python/tool_taxonomy.toml` (nested accessor `wbe.vector.geometry_processing.buffer_vector` no longer exists)
- Removed explicit Python method from `wb_environment.rs`
- Commented out in both R generated wrapper files
- Removed from QGIS plugin recipes and tool taxonomy

The underlying implementation can be restored and tested internally at any time. When the `BufferOp`
rewrite is complete, the existing non-dissolve path may be reused as a fallback, or replaced entirely.

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
