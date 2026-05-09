# Interop v1 Matrix and Projection Conformance Plan

Date: 2026-05-08
Owner: Whitebox Next Gen core team
Status: Draft v1 (ready to schedule)

## Purpose

Define a practical, high-value validation program that confirms cross-ecosystem interoperability for:
- Raster crate: wbraster
- Vector crate: wbvector
- Lidar crate: wblidar

This plan starts with a small projection conformance suite, then executes a 15-case interop roundtrip matrix against GDAL, QGIS, and PDAL outputs.

## Why This Order

1. Run projection conformance first to reduce false failures in interop tests.
2. Then run cross-ecosystem roundtrip matrix to validate real-world producer compatibility.

If projection math or CRS equivalence is not stable first, roundtrip failures are hard to triage.

## Scope Boundaries

In scope:
- Read/write interoperability for formats already supported in backend crates.
- CRS equivalence checks (semantic), not strict WKT string identity.
- Numeric and schema validations with explicit tolerances.

Out of scope (v1):
- Exhaustive format permutations.
- Massive performance benchmarks.
- Vendor-proprietary codecs not already supported natively.

Planned follow-on track (v1.5):
- Topology stress corpus mirroring GEOS/JTS pathological inputs for vector robustness.

## Phase A: Projection Conformance Mini-Suite

### A1. CRS Families (v1)

- Geographic: EPSG:4326, EPSG:4269
- Web/global projected: EPSG:3857, EPSG:3395
- UTM north/south: EPSG:32610, EPSG:32633, EPSG:32733
- Polar stereographic: EPSG:3413, EPSG:3031
- Equal-area examples: EPSG:5070, EPSG:3035

### A2. Geographic Regions and Point Selection

Use 8-15 points per CRS family with emphasis on:
- North America
- Europe
- Polar regions
- Southern Hemisphere
- Zone edges for UTM

Include at least:
- One central point
- One near projection bounds (but valid)
- One near zone boundary (UTM)

### A3. Assertions and Tolerances

Forward transform assertions (lon/lat -> x/y):
- Typical projected CRS: absolute error <= 0.10 m
- Preferred target for stable families: <= 0.01 m

Inverse transform assertions (x/y -> lon/lat):
- Absolute error <= 1e-7 degrees
- Preferred target: <= 1e-8 degrees for common CRS

Roundtrip stability:
- lon/lat -> x/y -> lon/lat must remain within inverse tolerance.

### A4. Reference Output Source

- Generate reference outputs with pinned PROJ version.
- Store reference files as test fixtures:
  - artifacts/interop/projection/reference/*.csv
  - artifacts/interop/projection/reference/*.json

### A5. Phase A Exit Criteria

- All selected CRS families pass tolerances.
- Any exceptions documented with root cause and explicit temporary tolerance override.
- No unknown CRS mismatch failures carried into Phase B.

## Phase B: Cross-Ecosystem Roundtrip v1 Matrix (15 Cases)

### B0. Validation Contract

Must match exactly:
- Raster rows/cols/bands
- Vector feature count
- Lidar point count
- Geometry type family
- Presence of nodata/CRS metadata where supported

Must match semantically:
- CRS equivalent (EPSG/proj semantics), not raw WKT string equality

Allowed differences (explicit):
- JPEG pixel differences due to lossy compression
- Shapefile schema truncation/coercion within format limits
- Internal block/tile/compression layout differences

### B1. Raster Cases (8)

| ID | Producer | Source format | Pipeline | Core assertions |
|---|---|---|---|---|
| R01 | GDAL | GeoTIFF int16 + nodata + EPSG | GDAL create A -> wbraster read -> wbraster write B -> GDAL inspect B | dims, nodata, CRS semantic match, sampled cell equality |
| R02 | GDAL | GeoTIFF float32 + scale/offset | same | dims, numeric tolerance, scale/offset handling, CRS |
| R03 | GDAL | COG tiled/compressed | same | dims, geotransform, CRS, pixel sample tolerance |
| R04 | GDAL | DTED level 1 | same | dims, EPSG:4326 semantics, nodata handling |
| R05 | GDAL | ERDAS HFA (.img) with RLC sample | same | read correctness, CRS recovery, sampled values |
| R06 | GDAL | Esri Float Grid (.flt/.hdr) | same | dims, nodata, geotransform consistency |
| R07 | GDAL | PNG + world + prj | same | world georeference, CRS, pixel values |
| R08 | QGIS | GeoTIFF export | QGIS export A -> wbraster read/write -> GDAL inspect B | geotransform/CRS parity across producer variance |

### B2. Vector Cases (4)

| ID | Producer | Source format | Pipeline | Core assertions |
|---|---|---|---|---|
| V01 | QGIS | GeoPackage mixed fields + nulls + multipart polygons | QGIS create A -> wbvector read/write B -> ogrinfo compare | feature count, geometry family, schema + null behavior, CRS |
| V02 | GDAL | Shapefile with long field names and constrained types | GDAL create A -> wbvector read/write B -> ogrinfo compare | expected truncation/coercion policy, count, CRS |
| V03 | GDAL | GeoJSON | GDAL create A -> wbvector read/write B -> ogrinfo compare | count, geometry envelopes, CRS semantics |
| V04 | GDAL | FlatGeobuf | GDAL create A -> wbvector read/write B -> ogrinfo compare | count, schema, CRS, envelope |

### B3. Lidar Cases (3)

| ID | Producer | Source format | Pipeline | Core assertions |
|---|---|---|---|---|
| L01 | PDAL | LAS 1.4 Point14 | PDAL create A -> wblidar read/write B -> pdal info compare | point count, bbox, dimensions (classification/intensity/returns), CRS VLR |
| L02 | PDAL | LAZ compressed | same | count, bbox, key dimensions, decode/encode stability |
| L03 | PDAL | COPC | same | hierarchy readability, count, bbox, key dimensions |

## Test Harness Structure (Suggested)

- artifacts/interop/
  - projection/
    - inputs/
    - reference/
  - raster/
    - producer_inputs/
    - roundtrip_outputs/
    - expected/
  - vector/
    - producer_inputs/
    - roundtrip_outputs/
    - expected/
  - lidar/
    - producer_inputs/
    - roundtrip_outputs/
    - expected/

- scripts/interop/
  - generate_projection_reference.sh
  - run_projection_conformance.sh
  - run_roundtrip_raster.sh
  - run_roundtrip_vector.sh
  - run_roundtrip_lidar.sh
  - summarize_results.py

## Result Normalization Rules

Raster:
- Compare selected sample windows and summary stats (min/max/mean/std).
- For lossy formats, use tolerance-based comparisons.

Vector:
- Normalize field order before comparison.
- Compare schema types by compatibility class when formats are limited.
- Compare geometry envelopes and counts before deeper topology checks.

Lidar:
- Compare required dimensions only for v1.
- Compare CRS metadata presence and semantic identity where available.

## Proposed Tolerance Defaults (v1)

Raster cell value tolerance:
- Lossless formats: exact where datatype permits
- Float data: abs diff <= 1e-6 to 1e-5 (dataset dependent)
- JPEG: PSNR or abs diff threshold documented per case

CRS equivalence:
- Prefer EPSG identity
- If EPSG absent, use semantic proj/WKT equivalence check

Vector geometry numeric tolerance:
- Coordinate abs diff <= 1e-9 degrees for geographic
- Coordinate abs diff <= 1e-4 m for projected datasets unless tighter validated

## Failure Taxonomy

Classify each failure as one of:
- CRS_PARSE
- CRS_EQUIVALENCE
- GEOTRANSFORM
- NODATA
- PIXEL_VALUE
- SCHEMA_MAPPING
- GEOMETRY_MAPPING
- POINT_DIMENSION_MAPPING
- EXTERNAL_TOOL_DRIFT

This makes triage and trend tracking much faster.

## Implementation Plan (2-4 Weeks)

Week 1:
- Build Phase A projection suite and freeze reference artifacts.
- Finalize tolerance baselines.

Week 2:
- Implement raster cases R01-R08 and reporting.

Week 3:
- Implement vector and lidar cases V01-V04, L01-L03.
- Add failure taxonomy and summary report.

Week 4 (buffer):
- Stabilize flaky cases and document known acceptable deviations.

Week 5-6 (recommended follow-on):
- Build and run topology stress corpus against wbvector.
- Classify and triage failures by topology category.

## CI and Execution Strategy

Given current constraints, prefer local/manual orchestration first:
- Run scripts locally in a repeatable environment.
- Archive outputs under artifacts/interop/results/<date>/.
- Add lightweight CI later only if maintenance cost remains low.

## v1 Exit Criteria

- Projection conformance passes for all listed CRS families within approved tolerances.
- Interop matrix has all 15 cases implemented and reproducible.
- Known acceptable deviations are explicitly documented.
- No unclassified failures remain.

Note: topology stress corpus is not a blocker for v1 exit; it is a recommended v1.5 quality gate.

## Phase C (Recommended v1.5): Topology Stress Corpus

### C1. Why Add It

Yes, this should be added to the plan. It targets a different risk class than format interop:
- Interop proves read/write compatibility with external producers.
- Topology corpus proves geometric robustness under pathological inputs.

These failures tend to be high-severity and hard to discover with normal datasets.

### C2. Scope and Data Sources

Construct a corpus inspired by GEOS/JTS pathological classes, including:
- Self-intersections (bow-tie polygons)
- Nearly-coincident edges and sliver polygons
- Ring orientation anomalies
- Duplicate/near-duplicate vertices
- Tiny gaps and overlaps in polygon fabrics
- Touching boundaries at single points
- Multipart edge cases (empty parts, mixed validity)

For each class, include:
- Minimal synthetic fixture
- One medium-complexity real-world-like fixture

### C3. Core Operations to Stress

Run wbvector operations that are topology-sensitive:
- buffer
- dissolve / union
- intersection / clip
- simplify (topology-preserving expectations where applicable)
- polygonize and validity-oriented workflows

### C4. Assertions

Per case, assert:
- Operation completes (no panic/crash)
- Deterministic result count and geometry type family
- Geometry validity status as expected
- Envelope/area sanity bounds
- No catastrophic coordinate blow-up

Add failure classes for this phase:
- TOPO_INVALID_OUTPUT
- TOPO_OPERATION_FAILURE
- TOPO_NUMERIC_INSTABILITY

### C5. Exit Criteria for Phase C

- Corpus includes all listed pathology classes.
- No untriaged topology failures.
- Known limitations are documented with reproducible fixtures.

## Future Expansion (Post-v1)

Phase C.1 execution draft for immediate raster/vector format expansion:
- docs/internal/INTEROP_PHASE_C1_FORMAT_EXPANSION_MINI_MATRIX_2026-05-09.md

- Add more producers and versions per format.
- Expand vector coverage (KML/KMZ/GPX, OSM PBF read workflows).
- Add lidar stress cases (larger COPC, optional dimensions).
- Add region-specific CRS packs and datum-shift edge cases.
- Expand topology stress corpus breadth and operation depth.
