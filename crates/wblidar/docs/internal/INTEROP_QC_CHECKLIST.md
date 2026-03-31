# Interoperability QC Checklist (All PDRFs)

This checklist validates a generated bundle of LAZ and COPC artifacts across LAS PDRF 0-10.

## 1. Generate the Bundle

From the workspace root:

```bash
cargo run -p wblidar --release --example generate_all_pdrf_artifact_bundle -- artifacts/ponui_row6_col4.laz
```

Default output directory:

`artifacts/ponui_row6_col4_all_pdrf_bundle`

Expected key output:

- `bundle_manifest.csv`
- 11 LAZ files (`*_pdrf0.laz` ... `*_pdrf10.laz`)
- 11 COPC files (`*_pdrf0.copc.laz` ... `*_pdrf10.copc.laz`)

## 2. Quick Structural Checks (PDAL)

Run for each file (LAZ and COPC):

```bash
pdal info <file>
pdal info --metadata <file>
pdal info --stats <file>
```

Confirm:

- point count is non-zero and stable across LAZ/COPC pairs
- LAZ header PDRF matches `laz_header_pdrf` in `bundle_manifest.csv`
- COPC header PDRF matches `copc_header_pdrf` in `bundle_manifest.csv`
- expected dimensions are present:
  - GPS time for PDRF 1/3/4/5/6/7/8/9/10
  - RGB for PDRF 2/3/5/7/8/10
  - NIR for PDRF 8
  - waveform metadata for PDRF 4/5/9/10

## 3. Visual Checks (QGIS)

> **Note:** QGIS/PDAL does not support waveform point formats.
> Files with `qgis_safe = false` in `bundle_manifest.csv` (requested PDRF 4, 5, 9, 10) will be
> rejected with `readers.las: Unsupported LAS input point format`. This is expected behavior.
> Load only files where `qgis_safe = true` (PDRF 0, 1, 2, 3, 6, 7, 8).

1. Load LAZ and COPC files where `qgis_safe = true` in the manifest.
2. In point cloud rendering, test:
   - `Single color`
   - `Classification`
   - `RGB`
3. If points appear missing under `RGB`, switch to `Single color` first.

Confirm:

- points render in all QGIS-safe files
- Z range and spatial extent are sensible and consistent
- RGB renders only where RGB channels are available/non-zero

## 4. Visual Checks (CloudCompare)

1. Open a LAZ/COPC pair for at least three requested PDRFs.
2. Verify point count, bounding box, and available scalar fields.
3. Compare visual extents and color behavior with QGIS observations.

## 5. Cross-Tool Consistency

For a sample set (for example requested PDRFs 0, 5, 8, 10):

- compare point count across PDAL, QGIS, and CloudCompare
- compare bounding boxes/min-max XYZ
- compare visible attribute channels (time/color/nir)

## 6. Pass/Fail Criteria

Pass when all are true:

- no parser/read errors in PDAL/QGIS/CloudCompare
- all files have non-zero points and correct extents
- LAZ and COPC header PDRFs match manifest declarations
- requested-to-actual promotions are explicitly captured in `bundle_manifest.csv`

Flag for investigation when any occurs:

- zero visible points in non-empty files under non-RGB renderer
- unexpected COPC header PDRF relative to manifest
- mismatched point counts or spatial extents across tools