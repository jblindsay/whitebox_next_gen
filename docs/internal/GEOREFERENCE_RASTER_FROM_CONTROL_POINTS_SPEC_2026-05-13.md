# Georeference Raster From Control Points: Implementation Spec

Date: 2026-05-13
Status: Implemented (v1)
Owner: Raster/Projection stack
Tool ID: georeference_raster_from_control_points
License tier: open

## 1. Decision Summary

Implement georeference_raster_from_control_points as a canonical runtime backend tool.

Frontend layers (Python, R, QGIS) may provide convenience helpers for input shaping,
but they must delegate to the same runtime tool ID so all environments share identical
fit, warp, diagnostics, and failure behavior.

Implementation note (2026-05-13):
- Runtime backend tool implemented and registered.
- Transform support implemented: affine, projective, polynomial (orders 1/2/3), thin_plate_spline.
- Frontend normalization paths wired in Python, R, and QGIS fallback bridge.

## 2. Scope and Goals

This tool georeferences an input raster from GCPs, estimates a selected transform,
warps raster values into an output geospatial grid, and emits diagnostics.

Primary goals:
- Numerical correctness and explicit failure modes.
- Performance as a first-order criterion.
- Stable cross-frontend behavior.
- Clear quality reporting (RMSE + per-GCP residuals).

Non-goals:
- CRS assignment-only metadata writes without geometric warp.
- GUI workflows in backend logic.

## 3. Frontend Contract

### 3.1 Python

Method name remains georeference_raster_from_control_points.

GCP argument remains:
- list[tuple[float, float, float, float]]

Tuple semantics (fixed order):
1. pixel_x
2. pixel_y
3. map_x
4. map_y

resample parameter remains supported and default is bilinear.

### 3.2 R

Expose wrapper that accepts a data.frame/matrix/list and normalizes into canonical GCP
records with the same semantics:
- pixel_x, pixel_y, map_x, map_y

Default resampling: bilinear.

### 3.3 QGIS plugin

Processing wrapper may accept CSV path and/or table UI input, but must normalize and
submit canonical payload to the runtime tool.

## 4. Runtime Parameter Schema

Required parameters:
- input_raster: string
- gcps: array of objects OR array of 4-tuples
- output_raster: string
- epsg or target_crs: integer or string

Optional parameters:
- transform_type: string (default: polynomial)
  - allowed: affine, projective, polynomial, thin_plate_spline
- transform_order: integer (default: 1)
  - used only when transform_type = polynomial
  - allowed: 1, 2, 3
- resample: string (default: bilinear)
  - accepted values must map to wbraster resamplers
- nodata: float|null (default: null)
- output_x_res: float|null
- output_y_res: float|null
- output_bounds: [float, float, float, float]|null
- allow_auto_downgrade: bool (default: false)
- report: string|null (default: derived beside output raster)
- report_format: string (default: both)
  - allowed: json, csv, both
- max_threads: int|null (default: runtime-managed)

Canonical GCP object form:
- pixel_x: float
- pixel_y: float
- map_x: float
- map_y: float
- optional fields for future extension only: id, z, weight

## 5. Transform and GCP Rules

Minimum GCP counts:
- affine: 3
- projective: 4
- polynomial order 1: 3
- polynomial order 2: 6
- polynomial order 3: 10
- thin_plate_spline: 10 (enforced for v1)

If requested transform is invalid for supplied GCP count:
- allow_auto_downgrade = false: fail with explicit required count.
- allow_auto_downgrade = true: downgrade stepwise to the highest valid model and emit warning.

Downgrade ladder:
- polynomial3 -> polynomial2 -> polynomial1 -> affine
- thin_plate_spline -> polynomial2 -> polynomial1 -> affine
- projective -> affine

No silent downgrade is permitted.

## 6. Validation Rules

Input validation:
- Reject NaN/Inf in any coordinate.
- Reject duplicated GCP pixel coordinates within epsilon.
- Reject degenerate control geometries (for example, near-collinear for models requiring full 2D support).
- Reject non-finite or invalid bounds/resolutions.

Quality warnings (non-fatal):
- Edge coverage weak (GCP convex hull too interior).
- Highly clustered GCPs.
- High fit condition number.
- High outlier residuals.

## 7. Performance Requirements

Performance is a first-order requirement.

Mandatory implementation constraints:
- Parallelize destination-grid warp/resample by blocks/tiles using rayon.
- Avoid duplicate full-image loops.
- Fuse operations where possible:
  - coordinate mapping
  - nodata masking
  - resample accumulation
  - output write staging
- Precompute transform coefficients exactly once per run.
- Pre-allocate tile buffers and reuse them.
- Keep allocation churn low inside pixel loops.
- Use cache-friendly iteration order matching raster memory layout.
- Avoid per-pixel dynamic dispatch in hot paths.

Recommended implementation structure:
1. Validate + normalize GCPs.
2. Fit transform + diagnostics in one pass over GCPs where possible.
3. Build immutable transform context.
4. Process output raster by tile in parallel:
   - map destination pixel -> source coordinates
   - resample source value
   - apply nodata logic
   - write tile buffer
5. Reduce tile diagnostics to global counters/statistics.

Threading behavior:
- Default: runtime thread pool.
- Optional max_threads parameter may cap worker usage.
- Deterministic output values are required regardless of thread count.

## 8. Diagnostics and Reporting

Report content (required):
- tool_id, timestamp, input/output paths
- transform_type requested and applied
- transform_order requested and applied
- gcp_count
- global RMSE
- per-GCP residuals:
  - dx, dy, radial_error
- warnings
- downgrade details (if any)
- output raster summary (crs, bounds, resolution, dimensions)

RMSE definition:
RMSE = sqrt((1/N) * sum((dx_i^2 + dy_i^2)))

Output artifacts:
- JSON report (machine-friendly)
- CSV residual table (per-GCP)

## 9. Error Codes

Tool-specific error code namespace: GEOREF_CP_

Required errors:
- GEOREF_CP_INVALID_INPUT_RASTER
- GEOREF_CP_INVALID_OUTPUT_PATH
- GEOREF_CP_MISSING_GCPS
- GEOREF_CP_INVALID_GCP_RECORD
- GEOREF_CP_DUPLICATE_GCP
- GEOREF_CP_DEGENERATE_GCP_LAYOUT
- GEOREF_CP_INSUFFICIENT_GCPS
- GEOREF_CP_UNSUPPORTED_TRANSFORM
- GEOREF_CP_UNSUPPORTED_RESAMPLER
- GEOREF_CP_FIT_FAILED
- GEOREF_CP_INVERSE_MAPPING_FAILED
- GEOREF_CP_WARP_FAILED
- GEOREF_CP_WRITE_FAILED
- GEOREF_CP_REPORT_WRITE_FAILED

Warnings:
- GEOREF_CP_WARN_AUTO_DOWNGRADE_APPLIED
- GEOREF_CP_WARN_WEAK_EDGE_COVERAGE
- GEOREF_CP_WARN_CLUSTERED_GCPS
- GEOREF_CP_WARN_POOR_CONDITIONING
- GEOREF_CP_WARN_HIGH_RESIDUAL_OUTLIERS

## 10. Return Payload

Return object fields:
- success: bool
- output_raster: string
- transform_applied: string
- effective_order: int|null
- gcp_count_used: int
- rmse: float
- downgraded: bool
- warnings: string[]
- report_json: string|null
- report_csv: string|null

## 11. Integration Status

Backend:
- Runtime registration entry added for georeference_raster_from_control_points.
- Tool implemented in open crate path.
- Diagnostics payload (RMSE, residuals, warnings, downgrade metadata) wired.

Python:
- Method name/signature family preserved.
- Tuple-list and CSV-path control point forms normalized correctly.
- Default resample remains bilinear.

R:
- Wrapper normalization added from data.frame/matrix/list to canonical GCP CSV payload.
- Default resample remains bilinear.

QGIS:
- Projection-wrapper fallback normalizes and calls canonical runtime tool.
- Output/report arguments forwarded to runtime georeference call.

## 12. Acceptance Criteria

Functional:
- Tool appears in runtime list_tools.
- Works with polynomial order 1/2/3 and projective/affine/tps (per constraints above).
- Produces georeferenced output and report files.
- Errors and warnings are emitted with listed codes.

Performance:
- Warp path is parallelized and tile-based.
- No duplicate full-raster loops for mapping + resampling + nodata handling.
- Memory allocations inside hot loops are minimized.

Cross-frontend:
- Python, R, and QGIS calls produce equivalent outputs and diagnostics for the same inputs.

## 13. Implementation Sequence (Completed)

Phase A (core) - completed:
- affine + projective + polynomial 1/2/3
- bilinear default resampling
- report generation
- strict validation and error codes

Phase B (advanced) - completed:
- thin_plate_spline fit/warp path
- additional numerical quality diagnostics

Phase C (frontend ergonomics) - completed:
- helper utilities for CSV/vector GCP ingestion where needed
- no backend behavior forks
