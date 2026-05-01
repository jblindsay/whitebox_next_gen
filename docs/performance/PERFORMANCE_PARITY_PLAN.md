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

---

## Lessons Learned

### L001 — Benchmark methodology: memory-backed vs. file-backed rasters (discovered 2026-04-30, find_noflow_cells)

**Problem**: NG `read_raster(file_mode='r')` returns a lazy, path-backed handle. Disk I/O is deferred to the first tool call that consumes the raster. Legacy `read_raster()` is always eager — the raster is fully loaded before any timing starts. When the NG benchmark timer wraps just the tool call (not the pre-load), it inadvertently includes the full GeoTIFF decode time. The apparent regression was ~1000% for find_noflow_cells; the true kernel regression was ~0%.

**Fix**: Always use `file_mode='m'` (memory-backed/eager load) in NG benchmark scripts so the raster is in-memory before the timer starts. This matches the effective state of legacy benchmarks.

```python
# WRONG — includes GeoTIFF decode inside the timed region
input1 = wbe.read_raster("/path/to/dem.tif")           # lazy
start = time.perf_counter()
output = wbe.hydrology.find_noflow_cells(dem=input1)   # I/O happens here

# CORRECT — decode outside the timed region
input1 = wbe.read_raster("/path/to/dem.tif", file_mode='m')  # eager
start = time.perf_counter()
output = wbe.hydrology.find_noflow_cells(dem=input1)         # kernel only
```

**Rule**: All benchmark scripts must use `file_mode='m'` unless the test is explicitly measuring end-to-end pipeline time (including I/O), not kernel performance.

---

### L002 — Output memory leak between benchmark iterations (discovered 2026-04-30, find_noflow_cells)

**Problem**: Each benchmark iteration creates a new output raster in the NG memory store. Without explicitly removing it, the store grows across iterations. Later iterations become slower due to increased memory pressure and internal store management overhead.

**Fix**: Call `wbe.remove_raster_from_memory(output)` at the end of every iteration.

```python
for i in range(5):
    start = time.perf_counter()
    output = wbe.hydrology.find_noflow_cells(dem=input1, interior_only=True)
    times.append(time.perf_counter() - start)
    wbe.remove_raster_from_memory(output)   # <-- mandatory
```

**Rule**: Every benchmark loop must free its output(s) before the next iteration.

---

### L003 — Thread spawn / allocator first-run overhead inflates run 1 (discovered 2026-04-30, find_noflow_cells)

**Problem**: Tools that use `std::thread::spawn` pay a one-time OS/allocator warmup cost on the first invocation within a process. Run 1 was consistently ~4.6s; runs 2–5 were ~2.2–2.5s. This made it look like there was a random cold-start regression when there was none.

**Fix**: Add one untimed warmup call before the measured loop to absorb the thread-spawn and allocator initialisation cost.

```python
# Warmup — absorbs thread-pool spin-up and allocator first-use cost
wbe.remove_raster_from_memory(
    wbe.hydrology.find_noflow_cells(dem=input1, interior_only=True)
)

times = []
for i in range(5):
    start = time.perf_counter()
    output = wbe.hydrology.find_noflow_cells(dem=input1, interior_only=True)
    times.append(time.perf_counter() - start)
    wbe.remove_raster_from_memory(output)
```

**Rule**: All benchmark scripts must include one untimed warmup iteration before the measured loop.

---

### L004 — Use median, not mean, for benchmark summaries (standing rule, reinforced 2026-04-30)

**Problem**: A single outlier run (e.g. run 1 with thread-spawn overhead, or a GC/OS hiccup) can inflate mean runtime significantly and give a misleading parity result.

**Rule**: Store `median` of 5 runs in the tracker. Always print/record the full run list so outliers are visible. Do not use mean as the primary parity metric.

---

### L005 — wbgeotiff single-band read path was unnecessarily slow (discovered 2026-04-30, find_noflow_cells)

**Problem**: `read_band_bytes` in wbgeotiff called `extract_band_bytes()` even for single-band rasters. That function iterates every pixel individually to extract the band's bytes from an interleaved buffer — O(N) work that is redundant when there is only one band (the decoded buffer already is the band).

**Fix**: Added a fast path in `read_band_bytes` that returns the decoded buffer directly when `samples_per_pixel == 1` and `planar_config == Chunky`, bypassing `extract_band_bytes` entirely.

**Impact**: Benefited every single-band raster read in the entire codebase (DEMs, single-band outputs, flow accumulation rasters, etc.).

---

### L006 — wbraster read_native_data used an intermediate aggregation buffer for single-band rasters (discovered 2026-04-30, find_noflow_cells)

**Problem**: `read_native_data` in `crates/wbraster/src/formats/geotiff.rs` always built a temporary aggregation `Vec` even when reading a single-band raster. For large DEMs this was a significant unnecessary allocation.

**Fix**: Added `bands == 1` fast paths for all 10 data types that return `RasterData` directly from `tiff.read_band_*()` without the intermediate buffer.

**Impact**: Same scope as L005 — applies to every single-band raster read.

---

### L007 — 1-tool-1-file structure improves debuggability (confirmed 2026-04-30, find_noflow_cells)

**Observation**: When all hydrology tools lived in a single large `mod.rs`, it was difficult to isolate performance issues to a specific tool, reason about individual kernels, and cleanly apply `Arc<BandView>` patterns. Extracting `find_noflow_cells` to its own file made the code trivial to audit and reason about, which directly enabled identifying the bottleneck and applying the fix without side-effect risk.

**Rule**: Each tool should live in its own file under its family subdirectory (e.g., `hydrology/find_noflow_cells.rs`). `mod.rs` should only contain `mod` declarations, `pub use` re-exports, and shared helpers (e.g., `parse_dem_and_output`, `load_raster`).

---

### L008 — Apparent regressions may be benchmark methodology failures, not kernel failures (meta-lesson, 2026-04-30)

**Observation**: find_noflow_cells appeared to be a ~1000% regression for weeks. Multiple rounds of structural optimisation (I16 native output, remove double alloc, BandView, etc.) shaved off real time but never reached parity, because they were attacking the kernel while the dominant cost was actually benchmark scaffolding (disk I/O inside the timer). The real kernel gap was ~0%.

**Rule**: Before investing engineering effort in kernel optimisation, verify the benchmark harness is measuring only what it should. Check:
1. Is input data pre-loaded before the timer starts? (L001)
2. Is the output freed between iterations? (L002)
3. Is there a warmup run? (L003)
4. Is `wbe.verbose` set to `False` to suppress I/O from progress printing? (set `wbe.verbose = False` at the top of every benchmark script)
5. Are both platforms running equivalent semantics / parameters? (see Parameter Policy above)
