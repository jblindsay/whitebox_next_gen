# Whitebox Next Gen Performance Parity Plan

## Objective
Stabilize release risk by tracking performance parity against legacy tools using real datasets and explicit pass/fail bands.

## Parity Bands
- Green: delta <= 10% (acceptable parity)
- Yellow: delta > 10% and <= 25% (needs follow-up)
- Red: delta > 25% (regression)

Delta formula:
- delta_pct = ((next_gen_runtime_s - legacy_runtime_s) / legacy_runtime_s) * 100

Repeated-run baseline rule:
- When multiple timings are collected for the same tool+dataset+parameter set, store the median in `legacy_runtime_s` / `next_gen_runtime_s` and preserve the full run list in notes.

## Hard-Stop Rule (Prevent Benchmark Loops)
For each NG run, terminate if elapsed time exceeds:
- min(10 * legacy_runtime_s, legacy_runtime_s + 600s)

On termination, mark:
- FAIL_OVERRUN

This keeps long-running failures from consuming hours.

## Batch Strategy
Use batches, not one-off random testing:
- Family batches: lidar, hydrology, terrain, vector, raster, image, stream network
- Motif batches: fixed-radius neighbor search, nearest-neighbor search, raster neighborhood scans, polygon candidate pruning, triangulation kernels

Prioritize batches by:
- user-visible impact
- dataset size/real-world frequency
- known regression severity

## Workflow Split
- User role: run legacy commands on real datasets and populate legacy_runtime_s + dataset_id.
- Agent role: generate/run NG commands in bounded batches using timeout-enforced runner.

## Parameter Policy
Performance parity must be measured under equivalent semantics, not blindly under each platform's current defaults.

Use these benchmark modes:
- `LEGACY_DEFAULT`: run the legacy tool with its documented/default parameters.
- `SEMANTIC_MATCH`: run NG with parameters adjusted to match the legacy tool's effective behaviour as closely as possible.
- `NG_DEFAULT`: optional secondary run using current NG defaults when they differ from legacy.

Rules:
- Release parity decisions should be based on `SEMANTIC_MATCH`.
- If legacy and NG defaults differ, do not compare `LEGACY_DEFAULT` versus `NG_DEFAULT` as the primary parity result.
- When defaults differ materially, record both runs if useful, but treat `NG_DEFAULT` as an API/default-behaviour check rather than a kernel-performance check.
- Record any parameter translation in benchmark notes, especially for renamed parameters, changed default radii, changed interpolation settings, changed return filters, and changed CRS/projection handling.

Parameter precedence:
1. Match legacy defaults when possible.
2. If exact matching is impossible, choose the closest semantic equivalent and document the gap.
3. If the tool has gained new mandatory semantics in NG, document the minimal parameter set needed to approximate legacy behaviour.

Default drift categories to flag in notes:
- changed default search radius / neighbourhood size
- changed interpolation parameter or statistic
- changed class / return filtering defaults
- changed nodata, edge handling, or fill behaviour
- changed CRS or reprojection behaviour
- changed output resolution or base-raster alignment rules

## Default Failure Response
When a tool is Red or FAIL_OVERRUN:
1. Stop repeated reruns.
2. Inspect legacy implementation structure and index strategy.
3. Port kernel structure faithfully (data structure + loop shape + chunking).
4. Re-test once after a meaningful patch set.

## Artifacts
- Tracker: docs/performance/tool_parity_tracker.csv
- NG manifest: docs/performance/ng_benchmark_manifest.csv
- NG results: docs/performance/ng_benchmark_results.csv
- Batch runner: scripts/performance/run_ng_benchmarks.py
