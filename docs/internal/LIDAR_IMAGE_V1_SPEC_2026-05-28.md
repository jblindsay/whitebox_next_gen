# LiDAR Image Tool V1 Specification
Date: 2026-05-28
Status: Draft
Owner: whitebox_next_gen

Related documents:
- docs/internal/LIDAR_POINT_NEIGHBOURHOOD_METRICS_PLAN_2026-05-28.md

## 1. Purpose
Create a fast, easy, and reliable tool for generating quick raster images from point clouds, with emphasis on:
- intensity image creation
- RGB image creation when color channels are present
- optional intensity normalization modes for more comparable outputs

This is a sibling convenience tool, not a replacement for advanced LiDAR interpolation tools.

## 2. Tool Identity
Proposed tool name: lidar_image
Category: lidar / io_management (or interpolation_gridding if preferred taxonomy fit)
License tier: open

## 3. High-Level Behavior
Inputs:
- one LiDAR file or a semicolon-delimited list of LiDAR files

Outputs:
- single raster output for intensity mode
- three-band raster output for RGB mode
- optional auxiliary outputs (count raster, spread/uncertainty raster) in later versions

Design goal:
- minimal parameter surface for quick image generation
- deterministic and performant defaults

## 4. V1 Modes
Mode enum:
- intensity
- rgb

V1.1 candidate mode:
- return_count (diagnostic density image)

## 5. Core Parameters (V1)
Proposed signature:
- input: string
  - LiDAR input path or list separated by semicolons
- output: string
  - output raster path
- mode: string = intensity
  - allowed: intensity, rgb
- resolution: float
  - output cell size in XY units
- method: string = mean
  - allowed: nearest, mean, max
- nodata: float = -32768.0
- background: string = nodata
  - allowed: nodata, zero

Extent controls:
- extent_mode: string = auto
  - allowed: auto, explicit
- min_x: float optional
- min_y: float optional
- max_x: float optional
- max_y: float optional

Point filtering controls:
- min_z: float optional
- max_z: float optional
- classes: string optional
  - comma-list of ASPRS classes
- returns: string = all
  - allowed: all, first, last

RGB controls:
- rgb_scaling: string = auto
  - allowed: auto, none, stretch_percentile
- rgb_p_low: float = 2.0
- rgb_p_high: float = 98.0

## 6. Intensity Normalization Parameters
Normalization is explicit and optional.

- intensity_normalization: string = none
  - allowed: none, range, range_angle, strip_robust

- sensor_altitude: float optional
  - used only if normalization requires it and metadata is missing

- strip_field: string optional
  - field used to separate strips/flightlines for strip_robust mode

- local_window_cells: int = 0
  - 0 disables local residual normalization
  - >0 enables local baseline correction after global normalization

- intensity_output: string = raw_or_normalized
  - allowed: raw, normalized, raw_or_normalized

Practical behavior:
- mode none: raw intensity aggregation
- mode range: apply range compensation prior to gridding
- mode range_angle: range compensation plus incidence-angle adjustment where estimable
- mode strip_robust: per-strip robust normalization then aggregate

## 7. Aggregation Semantics
For each raster cell:
- nearest: nearest point by XY distance
- mean: arithmetic mean of selected per-point channel
- max: maximum selected per-point channel

Tie handling:
- nearest ties resolved by stable point index order

Multi-tile overlap policy:
- all points from all inputs contribute to same grid
- aggregation method handles overlaps naturally

## 8. Metadata and Provenance
Embed raster metadata tags:
- lidar_image.mode
- lidar_image.method
- lidar_image.resolution
- lidar_image.normalization
- lidar_image.strip_field
- lidar_image.input_count
- lidar_image.returns_filter
- lidar_image.class_filter

For intensity normalized outputs:
- include normalization constants and assumptions
- include warning if required terms were approximated

## 9. Error Handling and Validation
Hard errors:
- output path missing
- resolution <= 0
- mode invalid
- method invalid
- rgb mode selected but no RGB values available in any input

Soft warnings:
- requested normalization not fully supported from available metadata
- empty class/return filter result
- very sparse coverage relative to resolution

## 10. Performance Design (V1)
- One streaming pass through points whenever possible
- Grid-first accumulators with chunked writes
- Thread-local accumulators merged per tile/block to avoid lock contention
- Bounded memory by tiled processing for very large extents

Implementation notes:
- Reuse existing LAS/LAZ reading pathway
- Reuse existing gridding kernels where practical
- Keep normalization functions branch-light and vectorization-friendly

## 11. Compatibility and Frontend UX
Python/R:
- expose concise wrapper with defaults matching CLI
- preserve descriptive parameter names

QGIS plugin:
- output should register as raster_out
- mode and normalization as enum widgets
- resolution as double
- return/class filters as strings with help text examples

Suggested QGIS labels:
- Mode
- Cell size
- Aggregation method
- Intensity normalization
- Return filter
- Class filter

## 12. Test Plan (V1)
Correctness tests:
- tiny synthetic cloud with known per-cell expected values
- nearest/mean/max behavior consistency
- RGB band ordering and scaling behavior

Normalization tests:
- no-op equivalence when normalization=none
- strip_robust produces reduced strip boundary artifacts in benchmark sample
- stable outputs under repeated runs

Performance tests:
- runtime and memory on small/medium/large point clouds
- thread scaling sanity checks

## 13. Rollout Plan
Phase 1:
- intensity mode with none and range normalization
- methods nearest/mean/max

Phase 2:
- RGB mode
- strip_robust normalization

Phase 3:
- range_angle normalization refinements
- optional diagnostic outputs (count/coverage)

## 14. Open Decisions
- Taxonomy placement: io_management vs interpolation_gridding
- Default output type and compression profile
- Whether to include return_count mode in v1 or v1.1
- Extent alignment strategy for reproducibility across runs

## 15. Recommendation
Proceed with lidar_image as a sibling tool now.

Reason:
- high user value for quick-look products
- practical implementation path by reusing existing kernels
- natural place to expose explicit intensity normalization options that current quick workflows lack
