# Spatial Stats and Curriculum Gaps: Improvements Roadmap

**Date:** May 21, 2026
**Author:** Platform Capability Review
**Scope:** Whitebox Next Gen open tier, with emphasis on teaching readiness for introductory and advanced spatial-analysis courses.

---

## Executive Summary

Whitebox open tier is strong for most introductory GIS and many advanced applied spatial-analysis workflows (terrain, hydrology, remote sensing preprocessing, vector overlay, and increasingly deep network analysis).

The largest curriculum-relevant gap is spatial statistics depth. Priority additions should include:

- Global and local autocorrelation (Moran's I, Local Moran / LISA)
- Hot-spot analysis (Getis-Ord Gi/Gi*)
- Kriging family (ordinary/universal plus diagnostics)
- Spatial regression families and geographically weighted regression (GWR)
- Point-process models and related diagnostics

A secondary, but important, gap is packaging these methods into teaching-oriented workflows (examples, diagnostics, assumptions, and interpretation aids), not just standalone computational kernels.

---

## 1. Corrective Note on Advanced Network Analysis

A fresh open-tier source audit indicates the network engine is materially stronger than a "basic-routing-only" characterization.

Evidence in open-tier code/manifests includes network and multimodal tool families such as:

- `shortest_path_network`
- `k_shortest_paths_network`
- `network_service_area`
- `network_od_cost_matrix`
- `closest_facility_network`
- `location_allocation_network`
- `multimodal_shortest_path`
- `multimodal_od_cost_matrix`
- `multimodal_routes_from_od`

Advanced impedance controls are also represented in manifests/args, including:

- one-way direction fields
- node-entry costs
- turn and U-turn penalties
- temporal cost profiles (`temporal_cost_profile` + `departure_time`)
- scenario bundle patterns for temporal/multimodal runs

### Practical implication

The core advanced network engine appears present in open tier. The remaining gaps are more likely to be:

- GUI parity/ergonomics across all advanced parameters
- teaching examples and benchmarked validation cases
- explicit interoperability guidance versus QGIS native network tools

---

## 2. High-Priority Gap: Spatial Statistics

### Why this matters for teaching

Upper-level GIS and GIScience courses commonly require a spatial statistics block. Without native support, instructors must split workflows across external ecosystems, reducing coherence for students.

### Priority capability groups

1. **Autocorrelation and clustering**
- Global Moran's I
- Local Moran's I (LISA) with significance outputs
- Getis-Ord Gi/Gi* hot/cold spots
- Join-count style metrics for categorical lattices

2. **Geostatistics (kriging)**
- Variogram/semivariogram estimation and model fitting
- Ordinary kriging
- Universal kriging
- Cross-validation diagnostics (ME, RMSE, standardized errors)

3. **Spatial regression and local models**
- Spatial lag and spatial error models
- Basic spatial Durbin variants (at least in matrix form)
- Geographically weighted regression (GWR) and multiscale extensions over time

4. **Point-pattern and point-process analysis**
- Kernel density diagnostics beyond simple heatmap outputs
- Ripley's K/L, nearest-neighbor index, quadrat tests
- Inhomogeneous Poisson process baselines and residual checks (phased)

---

## 3. Additional Teaching-Critical Gaps (Non-Cartographic)

1. **Inference-first outputs**
- Confidence intervals, p-values, multiple-testing handling, and assumption checks should be first-class outputs, not optional afterthoughts.

2. **Reproducible teaching diagnostics**
- Every statistics tool should output machine-readable diagnostics and concise interpretation strings suitable for lab assignments.

3. **Reference datasets and expected answers**
- Curated teaching datasets and known-answer tests are needed for classroom reliability.

4. **Cross-language parity**
- Ensure Python, R, and QGIS manuals expose equivalent workflows for new stats tools.

5. **Performance guidance and data-size envelopes**
- Instructors need practical guidance on what dataset sizes are appropriate for classroom machines.

---

## 4. Proposed Implementation Phases

### Phase A (Foundational spatial stats)

- Global Moran's I
- Local Moran's I (LISA)
- Getis-Ord Gi/Gi*
- Nearest-neighbor index and quadrat tests
- Shared significance and diagnostics schema

**Deliverables:** core tools, QA tests, Python/R/QGIS examples, teaching lab scripts.

### Phase B (Geostatistics and interpolation inference)

- Variogram modeling
- Ordinary kriging
- Universal kriging
- Cross-validation and uncertainty surfaces

**Deliverables:** reproducible kriging pipeline, benchmark reports, guidance on model assumptions.

### Phase C (Spatial regression and local modeling)

- Spatial lag/error regression
- Intro GWR
- Diagnostic suite (residual spatial autocorrelation, local instability flags)

**Deliverables:** teaching-oriented regression examples and interpretation templates.

### Phase D (Point-process expansion)

- Ripley's K/L and envelope testing
- Inhomogeneous process baselines
- Residual diagnostics and hotspot-vs-process comparison aids

**Deliverables:** advanced GIScience module support.

---

## 5. Phase A Execution Order (Risk- and Performance-First)

This is the recommended implementation order to maximize early value, minimize algorithmic risk, and avoid expensive rework.

### Step 0: Shared weights and diagnostics core (required first)

- Build one shared neighborhood/weights module used by all Phase A tools.
- Include contiguity (queen/rook), k-nearest, and distance-band in one place.
- Ship diagnostics from day one: island counts, degree min/mean/max, connected components, row-standardization flag.

### Step 1: Global Moran's I (first production stat tool)

- Lowest implementation risk with high curriculum value.
- Validates shared weights, inference schema, and deterministic diagnostics contract.
- Adds immediate utility for spatial autocorrelation teaching labs.

### Step 2: Point-pattern pair (NNI and quadrat)

- Implement nearest-neighbour index and quadrat count test next.
- Keeps algorithmic complexity moderate while expanding beyond lattice autocorrelation.
- Surfaces study-area and edge-policy decisions early, before local-cluster tools.

### Step 3: Local Moran's I (LISA)

- Reuses weights core but introduces per-feature inference and multiple-testing complexity.
- Add class outputs and adjusted p-values only after Steps 0-2 are stable.

### Step 4: Getis-Ord Gi/Gi*

- Implement after LISA because many per-feature output and correction pathways are shared.
- Keep hot/cold classification and adjusted significance fully aligned with LISA conventions.

### Step 5: Binding and manual parity hardening

- Wire Python/R/QGIS wrappers only after core tool outputs are stable.
- Freeze output key names and diagnostics schema before broad documentation rollout.

### Why this order

- Early steps create reusable infrastructure and lower-risk wins.
- Later steps consume that infrastructure rather than duplicating neighborhood logic.
- Prevents jumping into local-statistics complexity before deterministic core behaviors are proven.

---

## 6. Performance Policy (First-Order Requirement)

Performance is a design constraint, not a post-hoc optimization task.

### Mandatory implementation rules

1. Every new Phase A tool must declare expected time complexity and memory footprint before implementation.
2. Shared neighbor graph/weights construction should be reused across tools in a run when possible.
3. Avoid repeated spatial-index rebuilds inside per-feature loops.
4. Use numerically stable accumulation paths that are also cache-friendly.
5. Parallelize only after deterministic single-thread correctness is established, then preserve reproducibility.

### Mandatory validation rules

1. Add a lightweight performance sanity check per tool (small, medium, large synthetic workloads).
2. Track runtime envelopes in docs for classroom-scale datasets.
3. Treat major regressions as release blockers for spatial-statistics milestones.

### Practical target envelope (Phase A)

- A default classroom/lab machine should run 10k-50k features interactively.
- 100k-250k features should remain practical for batch workflows with clear runtime guidance.

---

## 7. Curriculum Readiness Matrix (Open Tier)

### Introductory GIS (current)
- **Status:** Strong
- **Notes:** Core raster/vector/network workflows are sufficient for most intro curricula.

### Advanced applied spatial analysis (current)
- **Status:** Strong to very strong
- **Notes:** Network, terrain, hydrology, and remote-sensing workflows are broadly capable.

### Advanced GIScience/spatial statistics (current)
- **Status:** Moderate
- **Primary blocker:** Native spatial statistics and inference depth.

### Target after Phases A-C
- **Status goal:** Strong
- **Condition:** Complete autocorrelation/hotspot/kriging/spatial regression baseline with diagnostics and parity docs.

---

## 8. Immediate Next Actions

1. Publish an explicit open-tier capability note clarifying advanced network support.
2. Implement the shared weights/diagnostics core as the first engineering milestone.
3. Implement `global_morans_i` as the first production Phase A tool.
4. Add lightweight performance sanity checks and runtime envelopes for each new tool.
5. Track parity progress in Python, R, and QGIS manuals as each tool lands.

---

## 9. Relationship to Existing Roadmaps

- Complements [docs/internal/VECTOR_platform_improvements_2026-05-20.md](docs/internal/VECTOR_platform_improvements_2026-05-20.md) by focusing specifically on spatial-statistics and curriculum readiness gaps.
- Should be treated as the primary planning document for statistics and inference-focused platform expansion.
- Also see [/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/docs/internal/SPATIAL_STATS_PHASE_A_DESIGN_SPEC_2026-05-21.md](/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/docs/internal/SPATIAL_STATS_PHASE_A_DESIGN_SPEC_2026-05-21.md).