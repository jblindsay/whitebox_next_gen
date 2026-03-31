# EPSG Parity Report

Comparison target: whitebox_workflows epsg_to_wkt.rs vs wbprojection::from_epsg support.

Last recomputed: 2026-03-15 (post step-2 + step-1 batch sequence).

## Current Coverage

- Total EPSG codes in epsg_to_wkt.rs: 4958
- Supported by wbprojection::from_epsg: 2484
- Missing in wbprojection::from_epsg: 2474
- Coverage: 50.1%

## Delta vs Previous Snapshot

Previous report snapshot: 2095 supported, 2863 missing.

- Newly covered codes vs previous snapshot: +389
- Remaining missing codes reduced by: 389

## Additional Context

- Total wbprojection built-in code count (EPSG + explicit ESRI codes): 2871
- Codes supported by wbprojection but absent from epsg_to_wkt.rs: 387

## Missing-Code Distribution (coarse buckets)

- 2000-range: 243
- 3000-range: 422
- 4000-range: 485
- 5000-range: 244
- 6000-range: 241
- 7000-range: 261
- 8000-range: 63
- 20000/30000-range: 768

## Largest Remaining Contiguous Missing Ranges

- 2000-2038
- 2391-2442
- 2867-2954
- 3036-3051
- 3068-3102
- 3113-3138
- 3161-3172
- 3175-3203
- 3350-3394
- 3415-3464
- 4001-4016
- 4044-4063
- 4120-4147
- 4190-4229
- 4231-4257
- 4270-4282
- 4291-4304
- 4306-4319

## Notes

- The whitebox_workflows and whitebox-tools epsg_to_wkt.rs files are identical at this snapshot.
- Prior Pulkovo-family additions materially improved overlap, but large geographic CRS blocks (especially 4000-range families) remain a major parity gap.
