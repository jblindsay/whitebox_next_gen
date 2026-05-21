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

## 5. Curriculum Readiness Matrix (Open Tier)

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

## 6. Immediate Next Actions

1. Publish an explicit open-tier capability note clarifying advanced network support.
2. Start Phase A spatial-statistics design spec (inputs, outputs, diagnostics, significance policy).
3. Define shared statistical output schema used across Python/R/QGIS bindings.
4. Create classroom benchmark dataset pack and known-answer validation suite.
5. Track parity progress in Python, R, and QGIS manuals as each tool lands.

---

## 7. Relationship to Existing Roadmaps

- Complements [docs/internal/VECTOR_platform_improvements_2026-05-20.md](docs/internal/VECTOR_platform_improvements_2026-05-20.md) by focusing specifically on spatial-statistics and curriculum readiness gaps.
- Should be treated as the primary planning document for statistics and inference-focused platform expansion.
