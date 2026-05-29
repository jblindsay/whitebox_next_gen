# LiDAR Gap Roadmap (Now Next Later)
Date: 2026-05-28
Status: Draft
Scope: whitebox_next_gen LiDAR portfolio prioritization after legacy parity

## 1. Goal
Prioritize high-impact LiDAR additions that improve user outcomes beyond parity, with emphasis on:
- usability and workflow speed
- feature robustness for classification and analytics
- radiometric consistency for intensity-driven products
- performance from day one

## 2. Current Baseline (Context)
- LiDAR portfolio has broad legacy parity and strong coverage across interpolation, filtering, classification, and diagnostics.
- The main opportunity is no longer basic parity, but workflow ergonomics and robust modern feature/radiometric tooling.

Reference context:
- docs/internal/TOOL_INVENTORY_SUMMARY.md
- docs/internal/LIDAR_POINT_NEIGHBOURHOOD_METRICS_PLAN_2026-05-28.md
- docs/internal/LIDAR_IMAGE_V1_SPEC_2026-05-28.md

## 3. Prioritization Method
Ranking dimensions:
- user impact: how many workflows improve and by how much
- implementation risk: algorithmic and integration risk
- delivery speed: realistic time to useful v1
- strategic leverage: how many future tools can reuse the core

Priority labels:
- P0: do now
- P1: do next
- P2: do later

Effort buckets:
- S: small (days)
- M: medium (1-3 weeks)
- L: large (multi-sprint)

## 4. Roadmap

## 4.1 P0 (Now)

### A. lidar_image quick product tool
- Priority: P0
- Effort: M
- Impact: very high
- Why now:
  - common user request for fast intensity and RGB rasters
  - major UX win with limited algorithmic risk
  - can reuse existing interpolation kernels
- Dependencies:
  - none blocking
  - optional shared intensity normalization module
- Deliverable:
  - intensity and RGB modes with nearest/mean/max
  - explicit normalization options and output provenance metadata
- Spec anchor:
  - docs/internal/LIDAR_IMAGE_V1_SPEC_2026-05-28.md

### B. Generalized neighbourhood point metrics sibling tool
- Priority: P0
- Effort: L
- Impact: very high
- Why now:
  - fills a core feature-engineering gap for classification and QA
  - creates reusable local-neighbourhood computation core
- Dependencies:
  - efficient neighbour search and shared accumulator architecture
- Deliverable:
  - sphere and cylinder neighbourhoods
  - elevation, shape, density, return, and intensity feature families
  - sidecar metrics output with schema and normalization provenance
- Spec anchor:
  - docs/internal/LIDAR_POINT_NEIGHBOURHOOD_METRICS_PLAN_2026-05-28.md

### C. Intensity normalization module (shared)
- Priority: P0
- Effort: M
- Impact: high
- Why now:
  - intensity comparability is a pain point across multiple workflows
  - shared module reduces duplication and inconsistency
- Dependencies:
  - metadata handling for range/angle/strip assumptions
- Deliverable:
  - normalization modes: none, range, range_angle, strip_robust
  - metadata and warnings for approximation assumptions
- Reuse targets:
  - lidar_image
  - neighbourhood metrics tool

## 4.2 P1 (Next)

### D. Flightline diagnostics to correction bridge
- Priority: P1
- Effort: L
- Impact: high
- Why next:
  - current tools emphasize diagnosis; users also need correction workflows
- Dependencies:
  - robust strip grouping and transformation model selection
- Deliverable:
  - strip bias correction workflow (intensity and/or geometric adjustment)
  - before/after QA metrics and reports

### E. Explicit canopy structure metric tools
- Priority: P1
- Effort: M-L
- Impact: high (forestry workflows)
- Why next:
  - existing tree and workflow tools are strong, but low-level transparent canopy metrics are limited as reusable base tools
- Dependencies:
  - neighbourhood and optional voxel support
- Deliverable candidates:
  - canopy height model helper pipeline
  - vertical profile metrics per grid cell
  - foliage/structure index products with clear definitions

### F. Dormant legacy workflow decision: lidar_dem_full_workflow
- Priority: P1
- Effort: S-M
- Impact: medium
- Why next:
  - either port, supersede, or formally retire with migration guidance
- Dependencies:
  - decision on preferred modern replacement pipeline
- Deliverable:
  - explicit status resolution and user-facing migration note

## 4.3 P2 (Later)

### G. Voxel and 3D occupancy analytics suite
- Priority: P2
- Effort: L
- Impact: medium-high
- Why later:
  - high value for advanced forestry/urban analysis, but larger architectural lift
- Deliverable:
  - voxel density, occupancy, and profile derivatives
  - optional octree acceleration paths

### H. Advanced radiometric calibration workflow
- Priority: P2
- Effort: L
- Impact: medium-high (specialist users)
- Why later:
  - requires careful handling of sensor/mission-specific assumptions and metadata variability
- Deliverable:
  - richer calibration modes beyond robust normalization
  - calibration QA report outputs

## 5. Suggested Execution Order (Practical)
1. Implement shared intensity normalization module.
2. Ship lidar_image v1 on top of shared module and existing gridding kernels.
3. Build neighbourhood metrics v1 with one-pass neighbour reuse and sidecar schema.
4. Add flightline correction bridge with QA outputs.
5. Decide and resolve dormant lidar_dem_full_workflow status.

## 6. Performance Guardrails
Apply to all new LiDAR roadmap items:
- one-pass or bounded-pass design where feasible
- thread-local accumulators and lock-minimized merges
- chunked processing to cap memory growth on large clouds
- explicit metadata/provenance for all transformed outputs
- benchmark fixtures established before broad rollout

## 7. Release Strategy
- Release in vertical slices with clear user value, not giant batch drops.
- For each slice:
  - CLI/core implementation
  - Python and R wrappers
  - QGIS parameter typing and output typing validation
  - concise docs with realistic example workflows

## 8. Immediate Next Actions
- A1. Approve P0 scope and parameter surfaces for lidar_image and neighbourhood metrics.
- A2. Define shared normalization API and metadata schema contract.
- A3. Create implementation issue checklist from this roadmap with owners and milestones.

## 9. Summary
The obvious gap is not breadth of LiDAR tools but ease, robustness, and modern feature/radiometric workflows.
The highest-return path is:
- quick image products
- shared intensity normalization
- generalized neighbourhood feature extraction
followed by correction-oriented and canopy-structure extensions.
