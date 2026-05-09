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
| Environment summary | Phase A complete (33/33 pass); Phase B now 15/15 pass with internal indexed FlatGeobuf read/write path; no external `flatgeobuf` crate dependency in wbvector; R05 resolved via GDAL HFA→GeoTIFF fallback; Phase C parser hardening stream remains active for cleanup/hardening |
| Producer tool versions (GDAL/QGIS/PDAL/PROJ) | GDAL 3.12.3; PDAL 2.10.1; PROJ 9.8.0; QGIS 4.0.0 Norrköping |
| Branch | checkpoint/phase1-phase2-pre-streamc-2026-04-12 |
| Commit SHA | b157dff (Phase C telemetry checkpoint) |

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
| R01 | int16 + nodata + EPSG roundtrip | GDAL | GeoTIFF | Passed | | artifacts/interop/results/raster/R01/ | wbraster roundtrip verified |
| R02 | float32 + scale/offset roundtrip | GDAL | GeoTIFF | Passed | | artifacts/interop/results/raster/R02/ | scale/offset source produced and roundtrip preserved cell/band counts |
| R03 | tiled/compressed roundtrip | GDAL | COG | Passed | | artifacts/interop/results/raster/R03/ | COG producer artifact roundtripped successfully |
| R04 | elevation roundtrip | GDAL | DTED L1 | Passed | | artifacts/interop/results/raster/R04/ | DTED read/write path stabilized in current Phase B v1.5 run |
| R05 | RLC sample roundtrip | GDAL | HFA (.img) | Passed | | artifacts/interop/results/raster/R05/ | native read variance mitigated by contained GDAL HFA→GeoTIFF fallback |
| R06 | sidecar header roundtrip | GDAL | Esri Float Grid | Passed | | artifacts/interop/results/raster/R06/ | reader updated for GDAL EHdr (`ULXMAP/ULYMAP/XDIM/YDIM`) |
| R07 | world file + prj roundtrip | GDAL | PNG + World File | Passed | | artifacts/interop/results/raster/R07/ | sidecar copy and `.wld` validation now pass |
| R08 | producer variance check | QGIS | GeoTIFF export | Passed | | artifacts/interop/results/raster/R08/ | qgis_process gdal:translate producer roundtrip succeeded |

### B1. Vector Cases (V01-V04)

| ID | Case | Producer | Source Format | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|---|---|
| V01 | mixed fields/nulls/multipart | QGIS | GeoPackage | Passed | | artifacts/interop/results/vector/V01/ | qgis_process native:savefeatures producer roundtrip succeeded |
| V02 | schema constraints behavior | GDAL | Shapefile | Passed | | artifacts/interop/results/vector/V02/ | homogeneous point schema roundtrip succeeded |
| V03 | basic interchange roundtrip | GDAL | GeoJSON | Passed | | artifacts/interop/results/vector/V03/ | GeoJSON roundtrip succeeded |
| V04 | binary interchange roundtrip | GDAL | FlatGeobuf | Passed | | artifacts/interop/results/vector/V04/ | internal indexed parser/writer path restored deterministic native handling under lean dependency policy |

### B2. Lidar Cases (L01-L03)

| ID | Case | Producer | Source Format | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|---|---|
| L01 | point14 baseline roundtrip | PDAL | LAS 1.4 | Passed | | artifacts/interop/results/lidar/L01/ | r-interop-enabled wbw_python build produced successful read/write roundtrip |
| L02 | compressed roundtrip | PDAL | LAZ | Passed | | artifacts/interop/results/lidar/L02/ | r-interop-enabled wbw_python build produced successful read/write roundtrip |
| L03 | hierarchy-aware roundtrip | PDAL | COPC | Passed | | artifacts/interop/results/lidar/L03/ | r-interop-enabled wbw_python build produced successful read/write roundtrip |

## Phase C (Recommended v1.5): Topology Stress Corpus

### C-Format. Active Parser Hardening Stream (In Progress)

| ID | Task | Status | Owner | Last Updated | Evidence Path | Notes |
|---|---|---|---|---|---|---|
| CF01 | Indexed FlatGeobuf native validation gate | Passed | | 2026-05-09 | crates/wbvector/src/flatgeobuf/mod.rs | deterministic indexed native parse restored without adding external flatgeobuf crate |
| CF02 | Indexed FlatGeobuf compatibility fallback instrumentation | Passed | | 2026-05-09 | crates/wbvector/src/flatgeobuf/mod.rs | telemetry environment flag/logging remains available for indexed parse decisions |
| CF03 | Remove indexed FlatGeobuf fallback entirely | Passed | | 2026-05-09 | crates/wbvector/src/flatgeobuf/mod.rs | indexed external fallback removed; dependency-lean native-only path enforced |

### C0. Phase Gate Checklist

| ID | Task | Status | Owner | Last Updated | Evidence Path | Notes |
|---|---|---|---|---|---|---|
| C00 | Pathology class list finalized | Passed | | 2026-05-09 | artifacts/interop/topology/corpus/synthetic/manifest.json | TC01-TC07 pathology classes captured in synthetic corpus manifest |
| C01 | Minimal synthetic fixtures created for each pathology class | Passed | | 2026-05-09 | artifacts/interop/topology/corpus/synthetic/ | generated TC01-TC07 minimal GeoJSON fixtures |
| C02 | Medium-complexity fixtures curated | Passed | | 2026-05-09 | artifacts/interop/topology/corpus/complex/ | generated TC01-TC07 medium-complexity GeoJSON fixtures |
| C03 | Topology-sensitive operation suite implemented | Passed | | 2026-05-09 | scripts/interop/phase_c_topology_test.py | synthetic runner executes buffer/simplify/union/intersection + wbw roundtrip and writes per-case artifacts |
| C04 | Failure taxonomy and triage workflow applied | Passed | | 2026-05-09 | artifacts/interop/results/phase_c_topology_results.json | failure taxonomy classes and per-case triage fields emitted; untriaged_failures=0 |
| C05 | Phase C exit criteria satisfied | Passed | | 2026-05-09 | artifacts/interop/results/topology/summary_phase_c.json | all TC synthetic+complex cases pass with no untriaged failures; known limitations documented |

### C1. Pathology Coverage Tracking

| ID | Pathology Class | Status | Failure Class | Evidence Path | Notes |
|---|---|---|---|---|---|
| TC01 | Self-intersection (bow-tie) | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC02 | Nearly-coincident edges / slivers | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC03 | Ring orientation anomalies | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC04 | Duplicate or near-duplicate vertices | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC05 | Tiny gaps and overlaps | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC06 | Point-touch boundaries | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |
| TC07 | Multipart edge cases | Passed | | artifacts/interop/results/phase_c_topology_results.json | synthetic and complex runs passed |

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

| Category | Total | Passed | In Progress | Failed | Blocked | Passed with Exceptions | Not Started |
|---|---:|---:|---:|---:|---:|---:|---:|
| Phase A tasks | 8 | 8 | 0 | 0 | 0 | 0 | 0 |
| Raster cases | 8 | 8 | 0 | 0 | 0 | 0 | 0 |
| Vector cases | 4 | 4 | 0 | 0 | 0 | 0 | 0 |
| Lidar cases | 3 | 3 | 0 | 0 | 0 | 0 | 0 |
| Phase C gate tasks | 6 | 6 | 0 | 0 | 0 | 0 | 0 |
| Phase C pathology cases | 7 | 7 | 0 | 0 | 0 | 0 | 0 |
| Total | 36 | 36 | 0 | 0 | 0 | 0 | 0 |

## Exit Sign-Off

| Gate | Status | Sign-Off By | Date | Notes |
|---|---|---|---|---|
| Phase A complete | Passed | | 2026-05-08 | Initial projection conformance run complete |
| All 15 Phase B cases executed | Passed | | 2026-05-08 | v1.5 runner executed full matrix; see artifacts/interop/results/phase_b_matrix_results.json |
| All interop test cases passing | Passed | | 2026-05-09 | Current run is 15/15 with V04 restored using internal indexed FlatGeobuf logic |
| Phase C parser hardening stream started | In Progress | | 2026-05-09 | native-only indexed path preserved and CF01 complete; additional cleanup/hardening still pending |
| v1 interop sign-off | Not Started | | | |
| Phase C topology sign-off (v1.5) | Passed | | 2026-05-09 | summary artifact generated at artifacts/interop/results/topology/summary_phase_c.json with 14/14 pass |
