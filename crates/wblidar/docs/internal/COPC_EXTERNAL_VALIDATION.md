# COPC External Validation Workflow

This document defines the repeatable external interoperability checks used to
close Milestone 3 for COPC writer conformance.

## 1) Generate fixtures

Run:

```bash
cargo run -p wblidar --example copc_validation_fixture_pack -- /tmp/wblidar_copc_validation
```

Expected outputs:
- /tmp/wblidar_copc_validation/fixture_pdrf6.copc.laz
- /tmp/wblidar_copc_validation/fixture_pdrf7.copc.laz
- /tmp/wblidar_copc_validation/fixture_pdrf8.copc.laz

## 2) Validate with validate.copc.io

For each file above:
- Open https://validate.copc.io
- Upload the file
- Record pass/fail and any warnings/errors

## 3) Validate with at least one additional external consumer

Use at least one of:
- PDAL (`pdal info`)
- CloudCompare
- QGIS
- any other independent COPC-capable reader

### PDAL example

```bash
pdal info /tmp/wblidar_copc_validation/fixture_pdrf6.copc.laz
pdal info /tmp/wblidar_copc_validation/fixture_pdrf7.copc.laz
pdal info /tmp/wblidar_copc_validation/fixture_pdrf8.copc.laz
```

Pass criteria:
- Command succeeds for each file
- Reported point count is 256
- No fatal parse/decompression errors

## 4) Results table

Fill this table and commit alongside Milestone 3 completion notes.

| File | validate.copc.io | External consumer | Notes |
|---|---|---|---|
| fixture_pdrf6.copc.laz | pending | pending | |
| fixture_pdrf7.copc.laz | pending | pending | |
| fixture_pdrf8.copc.laz | pending | pending | |

## 5) Local environment status

At the time this workflow was added, no external CLI tools were detected in the
active environment (`pdal`, `cloudcompare`, `lasinfo` not found), so step 3 is
manual or CI-runner dependent.
