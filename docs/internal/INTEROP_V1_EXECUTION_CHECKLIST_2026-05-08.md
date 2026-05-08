# Interop v1 Execution Checklist

Date: 2026-05-08
Related plan: docs/internal/INTEROP_V1_MATRIX_AND_PROJECTION_CONFORMANCE_PLAN_2026-05-08.md
Status legend:
- Not Started
- In Progress
- Blocked
- Passed
- Failed
- Passed with Exceptions

Scope note:
- Phase C is recommended v1.5 follow-on and does not block v1 sign-off.

## Usage Notes

- One row per checklist item.
- Keep failure taxonomy tags from the plan: CRS_PARSE, CRS_EQUIVALENCE, GEOTRANSFORM, NODATA, PIXEL_VALUE, SCHEMA_MAPPING, GEOMETRY_MAPPING, POINT_DIMENSION_MAPPING, EXTERNAL_TOOL_DRIFT.
- For Phase C, use topology tags: TOPO_INVALID_OUTPUT, TOPO_OPERATION_FAILURE, TOPO_NUMERIC_INSTABILITY.
- For Failed or Passed with Exceptions, always add a short reason in Notes.

## Run Metadata

| Field | Value |
|---|---|
| Run ID | v1-2026-05-08 |
| Operator | user |
| Date | 2026-05-08 |
| Environment summary | Phase A complete (33/33 pass); QGIS CLI available; Python bindings installing for Phase B roundtrip execution |
| Producer tool versions (GDAL/QGIS/PDAL/PROJ) | GDAL 3.12.3; PDAL 2.10.1; PROJ 9.8.0; QGIS 4.0.0 Norrköping |
| Branch | checkpoint/phase1-phase2-pre-streamc-2026-04-12 |
| Commit SHA | f184f28 (compilation fixes) |

## Phase A: Projection Conformance Mini-Suite

### A0. Phase Gate Checklist

| ID | Task | Status | Owner | Last Updated | Evidence Path | Notes |
|---|---|---|---|---|---|---|
| A00 | Reference outputs generated with pinned PROJ version | Passed | | 2026-05-08 | artifacts/interop/projection/reference/proj_toolchain.txt | Reference CSV/JSON generated via scripts/interop/generate_projection_reference.sh |
| A01 | CRS family coverage complete (4326, 4269, 3857, 3395, 32610, 32633, 32733, 3413, 3031, 5070, 3035) | Passed | | 2026-05-08 | artifacts/interop/projection/inputs/phase_a_points.csv | 33-point set spans all planned CRS families |
| A02 | Point set includes central, edge-valid, and UTM zone-boundary cases | Passed | | 2026-05-08 | artifacts/interop/projection/inputs/phase_a_points.csv | central, regional, southern, polar, and zone_edge categories present |
| A03 | Forward tolerance checks implemented and run | Passed | | 2026-05-08 | artifacts/interop/results/projection/phase_a_results.csv | Forward checks executed under 0.10 m tolerance for projected CRS |
| A04 | Inverse tolerance checks implemented and run | Passed | | 2026-05-08 | artifacts/interop/results/projection/phase_a_results.csv | Inverse checks executed under 1e-7 degree tolerance |
| A05 | Roundtrip stability checks implemented and run | Passed | | 2026-05-08 | artifacts/interop/results/projection/phase_a_results.csv | Forward then inverse checks pass across all cases |
| A06 | Exceptions documented (if any) with temporary tolerance rationale | Passed | | 2026-05-08 | artifacts/interop/results/projection/summary_phase_a.json | No exceptions required in initial run |
| A07 | Phase A exit criteria satisfied | Passed | | 2026-05-08 | artifacts/interop/results/projection/summary_phase_a.json | Initial run status PASS (33/33) |

### A1. Per-CRS Tracking

| CRS ID | Family | Forward tol | Inverse tol | Status | Evidence Path | Notes |
|---|---|---|---|---|---|---|
| EPSG:4326 | Geographic | n/a | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:4269 | Geographic | n/a | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:3857 | Web Mercator | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:3395 | World Mercator | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:32610 | UTM North | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:32633 | UTM North | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:32733 | UTM South | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:3413 | Polar stereographic | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:3031 | Polar stereographic | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:5070 | Equal area | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |
| EPSG:3035 | Equal area | <= 0.10 m | <= 1e-7 deg | Passed | artifacts/interop/results/projection/phase_a_results.csv | Case set passed |

## Phase B: Interop Matrix v1 (15 Cases)

### B0. Raster Cases (R01-R08)

| ID | Case | Producer | Source Format | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|---|---|
| R01 | int16 + nodata + EPSG roundtrip | GDAL | GeoTIFF | Not Started | | artifacts/interop/results/raster/R01/ | |
| R02 | float32 + scale/offset roundtrip | GDAL | GeoTIFF | Not Started | | artifacts/interop/results/raster/R02/ | |
| R03 | tiled/compressed roundtrip | GDAL | COG | Not Started | | artifacts/interop/results/raster/R03/ | |
| R04 | elevation roundtrip | GDAL | DTED L1 | Not Started | | artifacts/interop/results/raster/R04/ | |
| R05 | RLC sample roundtrip | GDAL | HFA (.img) | Not Started | | artifacts/interop/results/raster/R05/ | |
| R06 | sidecar header roundtrip | GDAL | Esri Float Grid | Not Started | | artifacts/interop/results/raster/R06/ | |
| R07 | world file + prj roundtrip | GDAL | PNG + World File | Not Started | | artifacts/interop/results/raster/R07/ | |
| R08 | producer variance check | QGIS | GeoTIFF export | Not Started | | artifacts/interop/results/raster/R08/ | |

### B1. Vector Cases (V01-V04)

| ID | Case | Producer | Source Format | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|---|---|
| V01 | mixed fields/nulls/multipart | QGIS | GeoPackage | Not Started | | artifacts/interop/results/vector/V01/ | |
| V02 | schema constraints behavior | GDAL | Shapefile | Not Started | | artifacts/interop/results/vector/V02/ | |
| V03 | basic interchange roundtrip | GDAL | GeoJSON | Not Started | | artifacts/interop/results/vector/V03/ | |
| V04 | binary interchange roundtrip | GDAL | FlatGeobuf | Not Started | | artifacts/interop/results/vector/V04/ | |

### B2. Lidar Cases (L01-L03)

| ID | Case | Producer | Source Format | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|---|---|
| L01 | point14 baseline roundtrip | PDAL | LAS 1.4 | Not Started | | artifacts/interop/results/lidar/L01/ | |
| L02 | compressed roundtrip | PDAL | LAZ | Not Started | | artifacts/interop/results/lidar/L02/ | |
| L03 | hierarchy-aware roundtrip | PDAL | COPC | Not Started | | artifacts/interop/results/lidar/L03/ | |

## Phase C (Recommended v1.5): Topology Stress Corpus

### C0. Phase Gate Checklist

| ID | Task | Status | Owner | Last Updated | Evidence Path | Notes |
|---|---|---|---|---|---|---|
| C00 | Pathology class list finalized | Not Started | | | artifacts/interop/topology/corpus/ | |
| C01 | Minimal synthetic fixtures created for each pathology class | Not Started | | | artifacts/interop/topology/corpus/synthetic/ | |
| C02 | Medium-complexity fixtures curated | Not Started | | | artifacts/interop/topology/corpus/complex/ | |
| C03 | Topology-sensitive operation suite implemented | Not Started | | | artifacts/interop/results/topology/ | |
| C04 | Failure taxonomy and triage workflow applied | Not Started | | | artifacts/interop/results/topology/ | |
| C05 | Phase C exit criteria satisfied | Not Started | | | artifacts/interop/results/topology/summary_phase_c.* | |

### C1. Pathology Coverage Tracking

| ID | Pathology Class | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|
| TC01 | Self-intersection (bow-tie) | Not Started | | artifacts/interop/results/topology/TC01/ | |
| TC02 | Nearly-coincident edges / slivers | Not Started | | artifacts/interop/results/topology/TC02/ | |
| TC03 | Ring orientation anomalies | Not Started | | artifacts/interop/results/topology/TC03/ | |
| TC04 | Duplicate or near-duplicate vertices | Not Started | | artifacts/interop/results/topology/TC04/ | |
| TC05 | Tiny gaps and overlaps | Not Started | | artifacts/interop/results/topology/TC05/ | |
| TC06 | Point-touch boundaries | Not Started | | artifacts/interop/results/topology/TC06/ | |
| TC07 | Multipart edge cases | Not Started | | artifacts/interop/results/topology/TC07/ | |

## Per-Case Assertion Checklist (Copy into each case folder or ticket)

| Assertion | Result | Notes |
|---|---|---|
| Read succeeds | | |
| Write succeeds | | |
| Dimensions/count match | | |
| CRS semantic equivalence | | |
| Nodata semantics preserved (if applicable) | | |
| Core values within tolerance | | |
| Format-specific expected lossy behavior documented | | |

## Summary Dashboard

| Category | Total | Passed | Failed | Blocked | Passed with Exceptions | Not Started |
|---|---:|---:|---:|---:|---:|---:|
| Phase A tasks | 8 | 8 | 0 | 0 | 0 | 0 |
| Raster cases | 8 | 0 | 0 | 1 | 0 | 7 |
| Vector cases | 4 | 0 | 0 | 1 | 0 | 3 |
| Lidar cases | 3 | 0 | 0 | 0 | 0 | 3 |
| Phase C gate tasks | 6 | 0 | 0 | 0 | 0 | 6 |
| Phase C pathology cases | 7 | 0 | 0 | 0 | 0 | 7 |
| Total | 36 | 8 | 0 | 2 | 0 | 26 |

## Exit Sign-Off

| Gate | Status | Sign-Off By | Date | Notes |
|---|---|---|---|---|
| Phase A complete | Passed | | 2026-05-08 | Initial projection conformance run complete |
| All 15 Phase B cases executed | Not Started | | | |
| Known deviations documented | Not Started | | | |
| v1 interop sign-off | Not Started | | | |
| Phase C topology sign-off (v1.5) | Not Started | | | |
