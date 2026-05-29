# LiDAR Point Neighbourhood Metrics Plan (Performance-First)
Date: 2026-05-28
Status: Draft for implementation planning

Related documents:
- docs/internal/LIDAR_IMAGE_V1_SPEC_2026-05-28.md

## 1. Executive Decision
Recommended design: build this as a sibling tool to lidar_eigenvalue_features, not a redesign/replacement.

Why:
- Preserves current behavior and user expectations for lidar_eigenvalue_features.
- Avoids breaking changes in Python/R/QGIS wrappers and existing workflows.
- Lets us ship new metrics incrementally with lower risk.
- Keeps implementation modular so we can share internal kernels without coupling user-facing APIs.

Proposed tool name (working): lidar_neighbourhood_point_metrics
Alternative: lidar_point_metrics

Related recommendation:
- Add a second sibling convenience tool, lidar_image, for quick rasterization of intensity and RGB products from point clouds.

## 2. Relationship to Existing Tool
Current tool role:
- lidar_eigenvalue_features computes a focused PCA/eigenvalue-derived metric family and writes sidecar outputs consumed by downstream LiDAR tools.

New tool role:
- Adds broader local geometric and radiometric metrics (for classification, QA, and analytics) from configurable neighbourhoods.

Design principle:
- Shared internal engine, separate public tools.
- Do not absorb eigenvalue tool into a monolithic replacement in v1.

Future option:
- If adoption shows overlap and maintenance burden, create an internal common core now and consider a unified facade later, while keeping old entrypoints as compatibility wrappers.

## 3. Primary Use Cases
- Point-cloud classification feature engineering.
- Ground/non-ground discrimination support.
- Building/vegetation separation support.
- Noise/outlier and confidence diagnostics.
- Pre-segmentation feature enrichment.

Note:
- These metrics are primarily used for classification and segmentation, but also valuable for QC, filtering, and change detection.

## 4. Proposed V1 Metric Families
Start with metrics that are high signal, computationally practical, and interpretable.

### 4.1 Elevation/Height Structure
- height_above_local_min
- height_above_local_p05 (robust low baseline)
- local_z_range
- local_z_iqr
- local_z_stddev

### 4.2 Neighbourhood Shape and Surface Structure
- plane_fit_residual_mean
- plane_fit_residual_median
- verticality (from local normal)
- local_slope_deg
- roughness_mad_z (robust roughness)

### 4.3 Density and Sampling
- neighbour_count
- point_density_2d (cylindrical neighbourhood)
- point_density_3d (spherical neighbourhood)

### 4.4 Return/Pulse Structure (if available in source format)
- first_return_fraction
- last_return_fraction
- multi_return_fraction
- return_number_entropy

### 4.5 Intensity (optional in v1, guarded by availability)
- local_intensity_mean
- local_intensity_stddev
- local_intensity_cv
- normalized_intensity_residual

Important caveat:
- Raw LiDAR intensity is typically not radiometrically normalized and is influenced by range, angle, scan geometry, and instrument settings. Interpret absolute intensity with caution.

## 5. Neighbourhood Models
Support both models from the outset because they capture different physical structure.

- Spherical neighbourhood
  - Parameters: search_radius and/or max_neighbours
  - Best for isotropic local geometry

- Cylindrical neighbourhood
  - Parameters: radius_xy, optionally z_min/z_max window
  - Best for height-above-neighbour and urban/vegetation discrimination

V1 recommendation:
- Provide mode enum: sphere | cylinder
- Common controls:
  - max_neighbours (optional cap)
  - min_neighbours (validity threshold)

## 6. Outputs and Format
Recommended output strategy:
- Sidecar binary and schema JSON, analogous to eigenvalue features pattern.
- Keep output co-located with source point cloud by default.

Reason:
- Avoid rewriting full LAS/LAZ for feature extraction-only workflows.
- Preserve compatibility with existing downstream tools that can consume sidecars.

Suggested files:
- input.metrics (binary blocks)
- input.metrics.json (field schema, units, metadata)

Schema metadata additions (for intensity transparency):
- intensity_normalization_mode (none | range | range_angle | strip_robust)
- normalization_parameters (per-run constants and assumptions)
- validity_flags (per-metric availability and confidence)

## 7. Performance-First Architecture
Performance should be a first-class design goal, not a later optimization pass.

### 7.1 Single Neighbour Search Pass per Point
- Build neighbour list once per point.
- Compute all selected metrics from the same neighbourhood payload.
- Avoid per-metric neighbour query duplication.

### 7.2 Shared Accumulators
- Compute shared primitives once:
  - running mean/variance for z
  - robust order statistics buffer for quantiles
  - covariance terms for local plane fitting
  - return histogram counters
- Derive multiple metrics from these primitives.

### 7.3 Cache-Friendly Memory Layout
- Prefer structure-of-arrays style for hot numeric fields (x, y, z, intensity, return fields).
- Keep per-thread scratch buffers reused across chunks to reduce allocations.

### 7.4 Parallelism Strategy
- Chunk points into contiguous blocks.
- Use rayon parallel iterators with bounded chunk size.
- Write results into pre-allocated output buffers by index to avoid locks.

### 7.5 I/O Strategy
- Stream read point records and populate compact working arrays.
- Write sidecar sequentially in large buffered blocks.
- Avoid random writes and frequent flushes.

### 7.6 Numerical Robustness
- Explicit handling for small neighbourhood counts.
- Emit NaN or sentinel by policy (configurable) when min_neighbours is not met.
- Document exact behavior in schema metadata.

### 7.7 Intensity-Robust Design
- Compute raw intensity metrics and normalized/relative variants side-by-side.
- Prefer robust local relative features (z-score, percentile rank, robust residual) over absolute intensity.
- Keep normalization optional and explicit; never silently alter raw intensity.
- Gate return-aware metrics by data availability and store feature availability flags.

## 8. API and Parameters (Draft)
Draft signature concept:
- input: lidar path
- output: optional sidecar path stem
- neighbourhood_mode: sphere | cylinder
- search_radius: float (sphere/cylinder)
- search_radius_xy: float (cylinder override)
- max_neighbours: int optional
- min_neighbours: int default >= 7
- metrics: comma-list or preset name
- include_intensity_metrics: bool
- include_return_metrics: bool
- intensity_mode: raw | relative_local | normalized
- intensity_normalization: none | range | range_angle | strip_robust
- strip_field: optional source field for per-flightline normalization

Preset packs:
- basic_classification
- ground_filtering
- urban_structures
- vegetation_structure

## 9. Implementation Plan (Phased)

Phase A: Core scaffolding
- Add tool shell, arg parsing, sidecar schema writer.
- Implement neighbourhood retrieval abstraction (sphere/cylinder).

Phase B: High-value geometry metrics
- Height structure and roughness/plane residual family.
- Density metrics.

Phase C: Return and intensity metrics
- Add conditional return/intensity families with capability checks.
- Add optional normalization module and explicit metadata emission.

Phase D: Integration and docs
- Python/R/QGIS wrappers and help text.
- QGIS output typing and parameter typing tests.
- Example workflows for classification and QA.

Phase E: lidar_image sibling tool
- Add quick image generation tool for intensity and RGB raster products.
- Reuse gridding kernels from existing LiDAR interpolators with simplified UX and presets.
- Support optional intensity normalization modes shared with neighbourhood metrics tool.

## 10. lidar_image Tool Concept (Feasibility and Practicality)
Decision:
- This is worthwhile, feasible, and practical as a sibling convenience tool.

Why it is worthwhile:
- Common user workflow: "I just need a quick intensity image" should not require selecting from advanced interpolation tools.
- Improves discoverability and reduces friction for QA/preview tasks.
- Creates a consistent entrypoint for intensity normalization options absent in current quick workflows.

Why it is feasible:
- Existing interpolators already provide core gridding behavior.
- Tool can be implemented as a thin orchestration layer over existing kernels with a narrower parameter surface.
- Shared normalization code can be reused by both lidar_neighbourhood_point_metrics and lidar_image.

Practical v1 scope:
- Modes: intensity, rgb, and optional return-count image.
- Gridding presets: nearest, mean, max (keep small curated set first).
- Resolution controls: cell_size and optional bounds from input extent.
- Optional normalization for intensity: none, range, range_angle, strip_robust.
- Output: raster image(s) with metadata describing normalization mode.

Non-goals for v1:
- Full radiometric calibration workflow requiring sensor-specific calibration archives.
- Exhaustive interpolation method parity with all advanced LiDAR interpolator tools.

Performance strategy for lidar_image:
- Stream points once, aggregate into raster tiles/chunks.
- Use thread-local tile accumulators merged by block to reduce contention.
- Keep memory bounded with chunked processing on very large clouds.

## 11. Validation Plan (Performance + Correctness)

## 10. Validation Plan (Performance + Correctness)
Correctness checks:
- Deterministic synthetic clouds with known expected metrics.
- Edge cases: sparse neighborhoods, boundaries, duplicate points.

Performance checks:
- Runtime and memory profile on representative LAS sizes.
- Scaling with thread count.
- Compare all-metrics single-pass versus per-metric repeated pass (sanity benchmark).
- Compare lidar_image runtime against current interpolator-based equivalent workflows.
- Validate normalization stability across strips/flightlines on multi-strip projects.

## 12. Recommendation Summary
Best design for now:
- Keep lidar_eigenvalue_features as-is.
- Introduce a sibling lidar_neighbourhood_point_metrics tool.
- Introduce a sibling lidar_image convenience tool for quick intensity/RGB products.
- Share internal computational kernels between tools where possible.
- Optimize for one-pass neighbourhood reuse and buffered sidecar output from day one.

Intensity guidance in summary:
- Treat raw intensity as acquisition-dependent.
- Default to raw plus robust relative metrics.
- Make normalization explicit, optional, and fully documented in outputs.

This approach maximizes stability and delivery speed while setting up a clean path for long-term convergence if desired.
