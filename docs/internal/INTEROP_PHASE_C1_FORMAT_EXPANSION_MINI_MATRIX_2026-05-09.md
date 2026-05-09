# Interop Phase C.1 Format Expansion Mini-Matrix

Date: 2026-05-09
Owner: Whitebox Next Gen core team
Status: Draft ready to execute

## Goal

Add a small, high-value expansion set of raster and vector format cases that exercises failure classes not fully stressed by the current 15-case v1 matrix.

Design constraint:
- Keep this to six new cases in the first wave.
- Each case must contribute at least one distinct interoperability risk signal.

## Proposed Case Set (Wave 1)

### Raster (3)

| ID | Producer | Source Format | Why it is unique | Core assertions | Primary failure classes |
|---|---|---|---|---|---|
| R09 | GDAL | GeoPackage Raster (`.gpkg`) | Tiled SQLite container path and metadata indirection unlike GeoTIFF | read/write succeeds, rows/cols/bands stable, georeference preserved, sample cell tolerance | GEOTRANSFORM, PIXEL_VALUE, EXTERNAL_TOOL_DRIFT |
| R10 | GDAL | AAIGrid/ASCII Grid (`.asc`) | Text-header parsing, nodata token handling, precision loss risk | read/write succeeds, nodata preserved, extent/cellsize preserved, sample cell tolerance | NODATA, PIXEL_VALUE, GEOTRANSFORM |
| R11 | GDAL | JPEG2000 (lossy path) | Lossy codec behavior and georeferencing sidecar handling | read/write succeeds, dimensions preserved, georeference preserved, tolerant numeric comparison documented | PIXEL_VALUE, GEOTRANSFORM, EXTERNAL_TOOL_DRIFT |

### Vector (3)

| ID | Producer | Source Format | Why it is unique | Core assertions | Primary failure classes |
|---|---|---|---|---|---|
| V05 | GDAL | KML/KMZ | Schema coercion + geometry model simplification common in KML ecosystem | read/write succeeds, feature count stable, geometry family compatibility, CRS semantics (WGS84 expectations) | SCHEMA_MAPPING, GEOMETRY_MAPPING, CRS_EQUIVALENCE |
| V06 | GDAL | GPX | Point/route/track mapping with constrained schema and layer semantics | read/write succeeds, per-layer feature counts stable, geometry family compatibility, attribute survival expectations documented | GEOMETRY_MAPPING, SCHEMA_MAPPING, EXTERNAL_TOOL_DRIFT |
| V07 | GDAL | GML (Simple Features profile) | Verbose schema typing and CRS encoding variability | read/write succeeds, feature count stable, CRS semantic equivalence, field type compatibility mapping | SCHEMA_MAPPING, CRS_PARSE, CRS_EQUIVALENCE |

## Assertion Contract Additions

For this C.1 wave, keep existing v1 contract and add:

1. Explicit lossy policy for R11
- Declare tolerance metric and threshold in case notes.
- Record one-line justification for chosen tolerance.

2. Layer-aware checks for V06 (GPX)
- Validate counts by logical layer/type when produced as multiple layers.
- If producer collapses layers, mark as Passed with Exceptions only when behavior is documented.

3. Schema compatibility class mapping for V05/V07
- Compare by compatibility class instead of strict primitive type equality where format limits apply.

## Suggested Output Paths

- artifacts/interop/results/raster/R09/
- artifacts/interop/results/raster/R10/
- artifacts/interop/results/raster/R11/
- artifacts/interop/results/vector/V05/
- artifacts/interop/results/vector/V06/
- artifacts/interop/results/vector/V07/
- artifacts/interop/results/phase_c1_format_expansion_results.json

## Execution Order

1. R10 (ASCII Grid) first: fast signal on parser correctness.
2. V05 (KML/KMZ) and V06 (GPX): highest schema/geometry coercion risk.
3. R09 (GeoPackage Raster) and V07 (GML): container/schema robustness.
4. R11 (JPEG2000): finalize with documented lossy tolerance.

## Exit Criteria (C.1 Wave 1)

- All six cases implemented and reproducible.
- Every failed case classified with existing taxonomy.
- Any passed-with-exceptions case has explicit, format-grounded rationale.
- No untriaged failures.

## Latest Run Snapshot (2026-05-09)

Execution command:
- `python scripts/interop/phase_c1_format_expansion_test.py`

Result artifact:
- `artifacts/interop/results/phase_c1_format_expansion_results.json`

Outcome:
- Total: 6
- Passed: 6
- Failed: 0
- Not Started: 0

Case status summary:
- `R09` Passed: GeoPackage raster producer-compatibility issue resolved by accepting GDAL raster registration (`2d-gridded-coverage`) in tile-table discovery.
- `R10` Passed
- `R11` Passed
- `V05` Passed
- `V06` Passed
- `V07` Passed

## Notes for Follow-on Wave

Potential Wave 2 additions if Wave 1 is stable:
- Raster: ENVI binary header variants, MBTiles raster extraction path.
- Vector: MapInfo TAB, FileGDB read workflows, OSM PBF read-only compatibility case.
