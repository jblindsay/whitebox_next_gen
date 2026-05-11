# NG vs Legacy Tool Audit (2026-05-10)

## Purpose

This document is the working log for a deep, code-based audit of Next Gen (NG) tools against their corresponding legacy tools.

## Scope and Prioritization

- `exists_next_gen = TRUE`

Important working rule:
- Every tool must be re-verified directly in code before recording any finding.

## Relationship To Existing Performance Docs
This document complements, but does not replace:

### 1. Semantic Equivalence
- Are the NG and legacy tools doing the same job?
- If they differ, is the NG tool intentionally more correct?
- Does NG miss legacy parallelization, indexing, buffering, or loop-fusion patterns?

### 3. Design Improvements
- Are there major opportunities to improve the NG tool beyond pure parity?
- Do not clutter entries with small cosmetic or micro-optimization ideas.

## Required Inputs For Each Audit Entry

Each entry should identify:

- tool name,
- tracker status and priority hint,
- legacy implementation path,
- NG implementation path,
- audit date,
- reviewer notes.

## Verdict Vocabulary

Use the following controlled terms when possible.

### Semantic Verdict

- `Equivalent`
- `EquivalentWithMinorDifferences`
- `EquivalentWithNGCorrection`
- `PartiallyDivergentAcceptable`
- `DivergentNeedsRedesign`
- `Undetermined`

### Performance Verdict

- `LegacyLikelyFaster`
- `NGLikelyFaster`
- `LikelyNearParity`
- `MixedOrDataDependent`
- `Undetermined`

### Parallelization Parity

- `FullMatch`
- `PartialMatch`
- `MissingInNG`
- `NGImproved`
- `NotApplicable`

### Loop-Shape Verdict

- `LoopFusionParity`
- `LoopStructureChanged`
- `NGUsesExtraPasses`
- `NGFusesMoreWork`
- `NotApplicable`

### Design Opportunity Severity

- `None`
- `Minor`
- `Moderate`
- `Major`

## Audit Procedure

For each tool:

1. Locate the tracker row and record the current priority hint.
2. Read the legacy tool file. Treat legacy as the baseline for intended behavior unless there is a clear correctness defect.
3. Read the NG implementation and identify the controlling code path.
4. Compare semantics first, before performance opinions.
5. Compare loop shape, number of passes, data structures, indexing strategy, and parallelization.
6. Record only major design opportunities.
7. Conclude with an action recommendation.

## Entry Template

Copy this block for each audited tool.

```md
## <tool_name>

- Audit date:
- Tracker status:
- Priority hint:
- Legacy path:
- NG path:

### Semantic Assessment
- Verdict:
- Summary:
- Important differences:
- Correctness note:

### Performance Assessment
- Verdict:
- Parallelization parity:
- Loop-shape verdict:
- Likely faster implementation:
- Evidence from code:
- Tuning opportunities:

### Design Improvements
- Severity:
- Opportunity:

### Recommended Action
- `NoAction`
- `PerformanceTuning`
- `SemanticRedesign`
- `SemanticRedesignAndPerformanceTuning`
- `ConfirmWithBenchmarkLater`

### Notes
-
```

## Cross-Tool Themes To Watch For

The audit should explicitly look for these recurring NG underperformance patterns:

1. NG uses multiple full passes where legacy fuses the same work into one pass.
2. NG recomputes derived values in later loops instead of carrying them forward.
3. NG builds intermediate vectors or maps that legacy avoids.
4. NG performs sequential aggregation where legacy uses thread-level accumulation.
5. NG samples or indexes data repeatedly instead of caching local state per pass.
6. NG separates filtering, metric calculation, and output assembly into distinct loops where legacy combines them safely.

Also watch for the inverse case:

1. NG may intentionally split loops to improve correctness, determinism, or maintainability.
2. When that happens, document whether the extra pass is justified or whether the same correctness could be preserved with fewer passes.

## Entry Index

Initial entries recorded below. Continue appending audited tools in priority order using the template above.

## abs

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/abs.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute absolute value for each valid raster cell and preserve nodata cells.
- Important differences: NG implements `abs` via shared unary-math framework; legacy uses a dedicated tool implementation.
- Correctness note: Cell transform semantics are identical.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are embarrassingly parallel unary transforms; NG avoids channel-based row transfer overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Straightforward, high-confidence parity.

## accumulation_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/accumulation_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute accumulation curvature using third-order bivariate polynomial fitting and optional log-transform output.
- Important differences: NG routes through a shared curvature-op framework; legacy uses a standalone implementation with similar numerical kernels.
- Correctness note: Projected/geographic handling and optional log-transform behavior appear aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both apply window-based polynomial derivative estimation in parallel row workflows with similar computational load.
- Tuning opportunities: Benchmark large geographic rasters where branch-specific kernels may diverge in cache behavior.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted regression fixtures for projected vs geographic CRS transitions.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence for curvature semantics.

## adaptive_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/adaptive_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply threshold-based adaptive mean filtering, replacing center cells when local deviation exceeds user threshold.
- Important differences: NG uses the shared phase-3 filter framework and modern argument parsing; legacy is a dedicated tool path.
- Correctness note: Replacement criterion and neighborhood mean workflow are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform moving-window mean/deviation checks with parallel row processing.
- Tuning opportunities: Evaluate integral-image acceleration for larger filter sizes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Clarify threshold interpretation in docs with one numeric example.

### Recommended Action
- `Accept`

### Notes
- Practical behavior appears tightly aligned.

## add

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/add.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform cellwise raster addition for aligned rasters with nodata preservation.
- Important differences: NG validates raster compatibility through shared binary-op plumbing before execution.
- Correctness note: Addition semantics and nodata behavior match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both use parallel row/cell math; NG avoids manual channel coordination overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## add_point_coordinates_to_table

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/add_point_coordinates_to_table.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform the same core operation: copy point features and append XCOORD/YCOORD attribute fields from point geometry. NG adds stricter validation (errors on null or non-point geometries encountered mid-stream) and supports generic vector I/O, while legacy assumes shapefile point records and directly indexes the first point.
- Important differences: NG writes `XCOORD`/`YCOORD` as Float fields with wider precision (`width=18`, `precision=8`) versus legacy (`width=12`, `precision=4`). NG also does explicit schema-aware attribute copying, while legacy reuses DBF records directly.
- Correctness note: NG’s stricter geometry handling is likely a correctness/robustness improvement rather than a regression.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG now uses a single streaming feature loop that validates point geometry, copies attributes, appends coordinate fields, and writes output immediately, matching the legacy loop shape and removing the intermediate materialization pass.
- Tuning opportunities: Low priority; any remaining delta is likely from generic vector abstraction overhead rather than pass structure.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep current streaming path and only revisit if benchmarks show vector backend overhead dominates on very large layers.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Single-pass legacy-aligned streaming refactor has been applied in NG; re-benchmark to confirm parity expectations.

## aggregate_raster

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/aggregate_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both aggregate source rasters by integer factor with `mean`, `sum`, `min`, `max`, and `range` methods.
- Important differences: NG uses chunked parallel aggregation and centralized progress coalescing; legacy uses manual threaded row dispatch.
- Correctness note: Block-window aggregation semantics and method outputs are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG applies rayon chunked iterators and avoids thread-channel collection overhead.
- Tuning opportunities: Benchmark high aggregation factors for memory bandwidth saturation.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional report field showing effective valid-cell count per output block.

### Recommended Action
- `Accept`

### Notes
- Strong parity with modernized execution path.

## anova

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/anova.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Statistical core behavior remains aligned (one-way ANOVA over class raster groups), and NG now supports optional legacy-style HTML report output while retaining JSON report output.
- Important differences: Legacy is file-first and opens reports in the desktop environment; NG adds optional HTML output via `output`/`output_html_file` aliases and still returns JSON reports by default.
- Correctness note: No material computation mismatch was identified; remaining differences are report-delivery style and UX defaults.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG computes per-class stats and global accumulators in one parallel fold/reduce over raster cells, then performs compact post-aggregation. Legacy does an initial threaded stats pass, then a second full raster pass for class aggregates, plus HTML generation/writing overhead. NG removes one major data pass and avoids report I/O inside the core tool path.
- Tuning opportunities: Main tuning is already done; the remaining work is output-mode compatibility, not kernel speed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional polish only: tighten table/CSS parity and any residual wording differences in interpretation text.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This tool demonstrates why semantic/API parity should be tracked separately from kernel performance parity.

## arccos

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/arccos.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both compute inverse cosine for raster cells.
- Important differences: Legacy explicitly maps out-of-domain values (outside [-1, 1]) to nodata-like handling; NG follows direct floating-point `acos` behavior, which can produce NaN.
- Correctness note: For valid inputs in [-1, 1], outputs align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary transforms; NG uses shared framework with lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-compatible domain guard mode to map invalid values deterministically.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Domain-handling policy is the main behavioral difference.

## arcosh

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/arcosh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both compute inverse hyperbolic cosine per raster cell.
- Important differences: Legacy checks the valid domain (`z >= 1`) and handles invalid values explicitly; NG applies raw `acosh`, which may propagate NaN for invalid inputs.
- Correctness note: For valid-domain cells, results are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are lightweight parallel unary math kernels; NG has reduced thread orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Offer optional domain-guard behavior for strict legacy compatibility.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Main divergence is invalid-input handling.

## arcsin

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/arcsin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both compute inverse sine for each valid raster cell.
- Important differences: Legacy explicitly guards the domain [-1, 1]; NG applies direct `asin` with NaN propagation on invalid inputs.
- Correctness note: Valid-domain outputs align closely.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both use parallel unary passes; NG framework reduces messaging overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy domain-validation mode.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Behavior differs mainly on out-of-domain cells.

## arctan

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/arctan.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute inverse tangent per valid raster cell with nodata preservation.
- Important differences: NG uses shared unary-math tool generation; legacy uses dedicated implementation.
- Correctness note: Numerical transformation semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary kernels; NG has lower thread orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## arsinh

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/arsinh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute inverse hyperbolic sine per valid raster cell with nodata preserved.
- Important differences: NG routes through shared unary-math framework.
- Correctness note: Core transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary operations; NG avoids explicit channel-based row collection.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## artanh

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/artanh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute inverse hyperbolic tangent per valid raster cell and preserve nodata.
- Important differences: NG uses shared unary-math macro infrastructure; legacy uses dedicated implementation.
- Correctness note: Core transformation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary kernels; NG avoids explicit channel fan-in overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## ascii_to_las

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/ascii_to_las.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools parse ASCII point records by user-specified field pattern and emit LAS outputs with inferred point format (time/color aware). NG largely preserves legacy semantics and adds quality-of-life features (optional output directory, typed argument parsing, stronger per-field validation).
- Important differences: Legacy requires an EPSG code argument and errors if projection metadata is unspecified; NG defaults EPSG to 4326 when omitted. NG also appears more defensive for malformed lines/fields and skips short or header-like lines similarly to legacy.
- Correctness note: Defaulting EPSG may be convenient but can silently encode wrong CRS metadata if users omit the argument; this is a potential semantic risk versus legacy’s stricter requirement.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie; legacy may be slightly faster on raw parsing.
- Evidence from code: Both implementations process each file line-by-line in a single main parse loop and write points sequentially. NG introduces richer validation and generic parsing helpers (`parse_field`/typed pattern checks), which may add modest overhead but not a structural performance shift. Neither implementation currently parallelizes per-file conversion across input file lists.
- Tuning opportunities: Optional file-level parallel conversion in batch mode (one file per worker) could yield high value for multi-file jobs without altering per-file semantics.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Require explicit CRS unless user intentionally opts into a default; add parallel batch conversion mode for input file lists.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Keep an eye on CRS-default behavior because it affects correctness more than runtime.

## aspect

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/aspect.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/basic_terrain_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute terrain aspect (degrees clockwise from north) using polynomial surface derivatives with projected/geographic CRS handling.
- Important differences: NG factors derivative kernels into shared helpers; legacy keeps logic local in the tool implementation.
- Correctness note: Aspect equations and CRS-dependent derivative paths align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both run row-parallel neighborhood derivative calculations with similar arithmetic intensity.
- Tuning opportunities: Benchmark 5x5 mode over large rasters for cache locality opportunities.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for projected-to-geographic boundary behavior.

### Recommended Action
- `Accept`

### Notes
- Strong parity for standard aspect workflows.

## assess_route

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/assess_route.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations split route polylines into fixed-length segments and compute the same segment metrics from a projected DEM: average slope, min/max elevation, relief, sinuosity, changes in slope direction, and openness-based visibility, with parent attributes copied through to output segments.
- Important differences: NG uses the new vector/raster abstractions and generic field assignment, while legacy operates directly on shapefile internals. NG uses null for missing visibility values, which is consistent with modern schema handling and functionally similar to legacy’s placeholder field behavior.
- Correctness note: No major semantic mismatch was identified in the core route metric workflow.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie; possibly slight legacy edge in raw throughput.
- Evidence from code: Both versions are largely single-threaded for segmentation and metric extraction and perform per-segment DEM sampling/visibility calculations in nested loops. NG introduces more schema/value-setting overhead through generic feature APIs, while legacy writes shapefile records and DBF attributes more directly. Kernel structure is otherwise similar.
- Tuning opportunities: A high-value opportunity is segment-level parallelism (parallel metric computation over independent segments with sequential deterministic write), especially for large route networks.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Introduce optional parallel segment-metric precompute with stable output ordering to accelerate large route datasets without changing semantics.

### Recommended Action
- `PerformanceTuning`

### Notes
- This is a good candidate for parallel-prep + sequential-apply pattern already used successfully in other NG tools.

## atan2

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/atan2.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute four-quadrant inverse tangent from paired raster inputs (`atan2(y, x)`) per valid cell.
- Important differences: NG dispatches through shared binary-op enum framework; legacy uses dedicated tool logic.
- Correctness note: Mathematical transform and nodata propagation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary raster passes; NG benefits from unified binary-op execution path.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## attribute_correlation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/attribute_correlation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Pearson correlation computation remains aligned, and NG now supports optional legacy-style HTML matrix report output while keeping JSON report output.
- Important differences: Legacy is report-file-first and desktop-open oriented; NG keeps API-first JSON defaults and optional HTML report generation.
- Correctness note: The statistical core is aligned; residual differences are in output defaults and presentation details.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG for larger tables.
- Evidence from code: NG uses parallel pairwise correlation evaluation across numeric fields and now computes each pair correlation via a single streaming accumulator over preloaded column vectors, removing per-pair temporary `xs`/`ys` materialization and associated extra reduction passes.
- Tuning opportunities: Remaining optimization opportunities are secondary (memory layout/cache locality of column vectors and HTML/report generation overhead).

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional HTML style/layout touch-ups and matrix formatting refinements for closer visual parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Another clear case where kernel modernization succeeded but user-facing parity drifted.

## attribute_histogram

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/attribute_histogram.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Core histogram computation is aligned and NG now supports optional HTML histogram report output using legacy-style rendering while preserving JSON report output.
- Important differences: Legacy report generation is primary behavior; NG remains API-first with optional HTML report output.
- Correctness note: Histogram math is aligned; remaining deltas are output defaults and minor rendering details.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Slight NG edge overall due no HTML rendering/writing.
- Evidence from code: Legacy performs separate min/max and binning loops over records, then renders and writes HTML/SVG output. NG now mirrors that two-pass loop shape directly over features (without intermediate full-value materialization) while still avoiding report-file generation overhead in non-HTML mode.
- Tuning opportunities: Limited; remaining differences are mostly output mode and rendering costs rather than kernel pass structure.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional graph-style polish (axes/labels/visual defaults) for tighter visual parity with legacy expectations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Pass-structure gap has been removed; remaining differences are primarily output compatibility and report UX.

## attribute_scattergram

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/attribute_scattergram.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Scattergram computation remains aligned and NG now supports optional HTML scattergram output (including trendline rendering when requested), while retaining JSON summary output.
- Important differences: Legacy is HTML-report-first; NG remains JSON-first with optional HTML report generation.
- Correctness note: Analytical core is aligned; remaining differences are report defaults and visual/output UX details.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG now computes paired sums, covariance terms, and min/max bounds in a single parallel reduction over paired values (after the required paired-value collection for plotting), reducing redundant follow-on scans while preserving optional trendline/report outputs.
- Tuning opportunities: Remaining optimization headroom is mainly tied to optional HTML rendering and retained paired-value storage needed for SVG plot generation.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional visual parity tuning for chart styling and annotation behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This extends the same recurring pattern as correlation/histogram: compute parity is good, report/output parity diverges.

## average_flowpath_slope

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/average_flowpath_slope.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools implement the same D8-based upslope traversal workflow: derive flow directions, count inflowing neighbours, process cells in topological order, accumulate total path length and divide elevation, and compute average flowpath slope in degrees per cell.
- Important differences: Legacy emits an explicit warning when interior pits are detected; NG does not currently expose the same warning behavior. Core computed metric logic appears aligned.
- Correctness note: No major numerical semantic drift is evident; warning/reporting parity is the main difference.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG now parallelizes the inflowing-neighbour preprocessing pass with row-level Rayon execution, reducing the earlier front-end gap. Legacy still parallelizes both direction setup and inflow counting explicitly, while both implementations retain a mostly serial topological stack traversal core.
- Tuning opportunities: Add legacy-style interior pit warnings/diagnostics and benchmark very large DEMs to confirm whether additional preprocessing fusion is warranted.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Restore legacy-style pit warning messaging and add optional diagnostics for unresolved interior depressions.

### Recommended Action
- `PerformanceTuning`

### Notes
- This is a clear example where parity should focus on missed parallel preprocessing, not algorithm redesign.

## average_horizon_distance

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/average_horizon_distance.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute mean distance-to-horizon from multi-azimuth horizon scans with observer-height support.
- Important differences: NG uses modernized azimuth-loop orchestration and cached offsets in shared sky-visibility tools.
- Correctness note: Horizon-distance accumulation intent and output meaning are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by directional horizon scanning; NG and legacy differ mainly in worker orchestration strategy.
- Tuning opportunities: Benchmark azimuth density and optimize per-azimuth cache reuse.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for per-azimuth horizon-distance variance.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core horizon-distance semantics appear well aligned.

## average_normal_vector_angular_deviation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/average_normal_vector_angular_deviation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools follow the same conceptual approach: smooth DEM with Gaussian-like filtering, compute normals for original and smoothed surfaces, derive angular difference, then average angular deviation over the local neighborhood window.
- Important differences: NG exposes an explicit `z_factor` parameter (legacy computes scaling behavior internally) and uses shared helper kernels (`gaussian_blur_values`, `compute_normals_from_values`, integral-based neighborhood means). These are implementation differences more than behavioral drift.
- Correctness note: No major semantic incompatibility is evident from the code path comparison.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGUsesExtraPasses`
- Likely faster implementation: Data-dependent, likely close.
- Evidence from code: Legacy includes threaded smoothing and substantial multi-stage processing; NG now parallelizes DEM extraction, angular-difference construction, and row-based neighborhood averaging but still materializes several full-size intermediate arrays (`base`, `smoothed`, normals, `diff`, integrals). Both are pass-heavy kernels by design, so net advantage remains data and memory-bandwidth dependent.
- Tuning opportunities: Biggest gain opportunity is reducing intermediate allocations and fusing post-normal-difference stages where possible.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Document/validate `z_factor` behavior relative to legacy default scaling to ensure users can reproduce legacy outputs exactly when needed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This tool appears structurally close enough that benchmark confirmation is more valuable than immediate redesign.

## average_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/average_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell mean across input raster stack while ignoring nodata unless all inputs are nodata.
- Important differences: NG runs through consolidated overlay-op framework shared with other overlay statistics.
- Correctness note: Averaging and nodata rules are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel stack-reduction passes; NG avoids legacy channel-based row collection overhead.
- Tuning opportunities: Evaluate memory-bandwidth limits for large raster stacks.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## average_upslope_flowpath_length

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/average_upslope_flowpath_length.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations compute D8 flow directions, derive inflowing-neighbour counts, traverse cells in topological order, accumulate upslope path lengths and flowpath counts, and output per-cell average upslope flowpath length.
- Important differences: Legacy includes explicit interior-pit warnings; NG currently does not surface an equivalent warning. Core accumulation logic and output meaning appear aligned.
- Correctness note: No major semantic mismatch in the core numerical method was identified.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG now parallelizes the inflow-neighbour preprocessing pass using row-parallel Rayon workflow, reducing the prior preprocessing deficit relative to legacy. Both versions still spend most runtime in dependency-ordered stack propagation that is largely serial.
- Tuning opportunities: Benchmark large conditioned DEMs and consider additional preprocessing fusion only if remaining gaps are measurable.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Reintroduce pit-detection warnings/diagnostics so users are warned when DEM conditioning likely affects result quality.

### Recommended Action
- `PerformanceTuning`

### Notes
- Similar tuning strategy as `average_flowpath_slope`: parallelize front-end passes, preserve deterministic stack traversal.

## balance_contrast_enhancement

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/balance_contrast_enhancement.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations apply Liu-style balance contrast enhancement on packed RGB imagery by computing per-channel distribution statistics and applying per-channel parabolic remapping to target the requested band mean.
- Important differences: NG explicitly validates packed RGB mode up-front and runs through shared non-filter remote-sensing dispatch; legacy is single-tool-specific but algorithmically matches.
- Correctness note: NG’s explicit packed-RGB validation is a robustness improvement and does not appear to alter intended output semantics.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon fold/reduce for channel statistics and parallel pixel remapping; legacy uses thread channels and two threaded loops with more coordination overhead. Both retain a two-phase structure (stats then remap), but NG’s reductions and contiguous output mapping are likely more cache- and threading-efficient.
- Tuning opportunities: Minor: avoid final sequential per-pixel `set` loop by adding row/band bulk write path for packed RGB output when available.

### Design Improvements
- Severity: `None`
- Opportunity: No major behavioral redesign opportunity identified beyond optional micro-optimizations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This appears to be one of the cleaner NG ports where both parity and performance posture are strong.

## basins

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/basins.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both delineate D8 basins from flow-direction pointers, assigning basin IDs to outlet-connected cells.
- Important differences: NG integrates basin logic into shared hydrology module flow-graph routines; legacy is a standalone tool path.
- Correctness note: Basin delineation objective and pointer-style handling are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by flow-pointer traversal and basin label propagation; orchestration differs more than core complexity.
- Tuning opportunities: Benchmark very large flats/pit-heavy terrains for label-propagation hotspot behavior.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional summary output for number of delineated basins and unresolved interior pits.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Hydrologic semantics appear consistent.

## bilateral_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/bilateral_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/bilateral_filter.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both implement edge-preserving bilateral filtering with spatial and intensity Gaussian weighting.
- Important differences: NG precomputes spatial kernel terms once and reuses them more explicitly; legacy computes equivalent logic in a more monolithic path.
- Correctness note: Bilateral weighting formulation and output intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel neighborhood filters; NG reduces repeated kernel setup overhead.
- Tuning opportunities: Benchmark large sigma settings where window size expansion dominates runtime.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional approximation mode for very large kernels.

### Recommended Action
- `Accept`

### Notes
- Strong parity with practical NG optimization.

## block_maximum

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/block_maximum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools rasterize point samples by assigning per-cell maxima with optional attribute or Z-based values and optional base-raster geometry. Output intent is aligned.
- Important differences: Legacy is shapefile-point specific and mostly single-point-per-record oriented. NG routes through generic vector sampling helpers and is more schema/geometry agnostic.
- Correctness note: NG’s generalized sampling path is likely more robust across vector sources, but should be checked for exact edge behavior versus legacy for unusual geometry encodings.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Mixed; NG likely faster on larger point datasets.
- Evidence from code: NG performs a parallel per-thread cell-binning fold/reduce over samples and writes one reduced value per touched cell, replacing the previous per-sample sequential update loop. The redundant explicit full-raster nodata initialization pass has now been removed.
- Tuning opportunities: If needed, benchmark sparse-vs-dense point distributions to confirm where hash-map reduction overhead dominates.

### Design Improvements
- Severity: `Minor`
- Opportunity: Benchmark-focused follow-up only; no additional structural changes are currently justified.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Shared parallel cell-binning optimization is in place and redundant output initialization has been removed; verify gains on sparse vs dense point distributions.

## block_minimum

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/block_minimum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools rasterize point samples by assigning per-cell minima with optional attribute or Z inputs and optional base-raster geometry.
- Important differences: Legacy is shapefile-point oriented with explicit attribute/FID handling; NG uses generic vector sampling helpers and a shared extrema core used by both block min/max tools.
- Correctness note: Output intent is aligned, but boundary-cell and mixed-geometry edge behavior should still be parity-checked because NG’s generalized sampling path differs from legacy assumptions.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Mixed; NG likely faster on larger point datasets.
- Evidence from code: NG uses the shared parallel cell-binning fold/reduce path and writes one reduced minimum per populated output cell, removing the previous per-sample sequential update bottleneck. The redundant explicit full-raster nodata initialization pass has now been removed.
- Tuning opportunities: Benchmark sparse-vs-dense point distributions to determine when map-reduction overhead dominates.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep the shared min/max primitive; revisit only if benchmarks show remaining reduction overhead worth tackling.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Shared parallel cell-binning optimization and initialization-pass trimming are now applied; re-benchmark against legacy across dataset scales.

## bool_and

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/bool_and.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform boolean AND on two input rasters treating non-zero as true and zero as false.
- Important differences: NG dispatches through unified binary-op framework.
- Correctness note: Boolean truth-table and nodata propagation behavior match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary cellwise kernels; NG has lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## bool_not

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/bool_not.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform boolean NOT on one raster, mapping zero to one and non-zero to zero for valid cells.
- Important differences: NG uses macro-generated unary op path.
- Correctness note: Truth-table behavior is aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary boolean transforms; NG avoids explicit row-channel fan-in.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## bool_or

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/bool_or.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform boolean OR on two input rasters treating non-zero as true and zero as false.
- Important differences: NG uses consolidated binary-op enum dispatch.
- Correctness note: Truth-table semantics and nodata behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary kernels; NG reduces thread orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## bool_xor

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/bool_xor.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform boolean XOR between two rasters, treating non-zero as true and zero as false.
- Important differences: NG uses a shared binary-op dispatch framework for XOR.
- Correctness note: Truth-table and nodata propagation behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary per-cell kernels; NG avoids manual channel-based row handoff.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## boundary_shape_complexity

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/boundary_shape_complexity.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations compute boundary complexity from a line-thinned patch skeleton and estimate complexity using exterior-link dominance relative to skeleton size.
- Important differences: NG rewrites data structures to flat vectors and helper functions; legacy uses `Array2D` and thread-channel setup in early stages. Core thinning/link-tracing and index formula remain aligned.
- Correctness note: No high-impact semantic mismatch was identified in the main metric logic.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both versions keep the expensive iterative thinning and skeleton traversal mostly serial. NG parallelizes initialization/output mapping with Rayon; legacy parallelizes initial row conversion with thread workers. The dominant middle stages remain similar complexity.
- Tuning opportunities: If this becomes a hotspot, focus on algorithmic thinning acceleration rather than small loop-level threading tweaks.

### Design Improvements
- Severity: `None`
- Opportunity: No major redesign needed based on code audit alone.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This appears to be a relatively faithful NG port compared with several neighboring geomorphometry tools.

## breach_depressions_least_cost

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/breach_depressions_least_cost.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both breach depressions using least-cost path search under distance and cost constraints.
- Important differences: NG integrates the implementation in shared hydrology infrastructure with modern argument and output handling.
- Correctness note: Breaching objective, key parameters, and output DEM semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are dominated by constrained path-search over depressions; NG benefits from tighter dataflow and reduced orchestration overhead.
- Tuning opportunities: Profile high-depression-density terrains for queue/update hotspot optimization.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostic output summarizing number and length of breached paths.

### Recommended Action
- `Accept`

### Notes
- Benchmarked parity outcome is strong.

## breach_single_cell_pits

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/breach_single_cell_pits.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both remove single-cell pits by breaching toward the best local outlet direction.
- Important differences: NG uses shared hydrology module plumbing.
- Correctness note: Pit-removal behavior and output semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both perform local neighborhood checks and updates; NG reduces overhead in row orchestration.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## breakline_mapping

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/breakline_mapping.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG now follows the legacy breakline workflow much more closely, including legacy-style curvedness derivatives (projected 5x5 and geographic local-distance forms), edge-cell suppression, anchor-aware smoothing, geographic length handling, and Z-enabled line vertices.
- Important differences: NG still uses a modernized component-labelling/vectorization structure rather than the exact legacy line-trace/anchor-injection sequence, so some junction-order and feature segmentation outcomes may still differ in edge cases.
- Correctness note: The largest semantic gaps have been reduced; remaining differences are now targeted parity follow-up items rather than a full redesign blocker.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGUsesExtraPasses`
- Likely faster implementation: Near tie (data dependent).
- Evidence from code: NG now matches legacy in the heaviest curvedness kernel semantics and keeps that stage parallelized, but still performs serial-heavy post-thinning component ordering and uses hash-set-based tracing structures.
- Tuning opportunities: Reduce hash-set churn in component ordering and parallelize line-ordering preparation with deterministic write ordering.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Complete final parity by tightening branch/junction tracing behavior to the legacy anchor-seeding sequence, then apply throughput tuning.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Fidelity-first refactor has started and closed the highest-impact curvedness/geographic/edge-behavior gaps; benchmark + edge-case tracing regression tests are the next gate.

## buffer_raster

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/buffer_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate raster buffers around target-valued cells using distance-based expansion logic.
- Important differences: NG is integrated into the consolidated GIS tools module and uses modernized buffering dataflow.
- Correctness note: Buffer-distance semantics and output raster behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Both are distance/buffer expansion dominated; NG uses improved parallel workload distribution.
- Tuning opportunities: Optimize memory access for very large contiguous buffer regions.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output of raw distance raster used for thresholding.

### Recommended Action
- `Accept`

### Notes
- Verified tested parity with large runtime improvement.

## burn_streams

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools (standalone burn_streams.rs not present in this source tree)`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both burn stream vectors into a DEM using decrement/gradient control to enforce channels.
- Important differences: NG implementation is in stream-network-analysis module with consolidated rasterization helpers.
- Correctness note: Channel-burning objective and main parameter semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Both are rasterization plus DEM update workflows; NG tested benchmarks indicate substantially lower runtime.
- Tuning opportunities: Benchmark dense stream networks and optimize segment rasterization ordering.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for number of burned cells and mean burn depth.

### Recommended Action
- `Accept`

### Notes
- Legacy path note reflects source-layout gap in current workflows tree.

## burn_streams_at_roads

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/burn_streams_at_roads.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG now provides a legacy-fidelity default path (`behavior_mode="legacy"`) that mirrors legacy crossing detection and two-pass burn traversal, while retaining the previous overlap/BFS implementation as `behavior_mode="fast"`.
- Important differences: Legacy mode in NG ports the legacy-style row/column segment-intersection scans, diagonal/intermediate crossing checks, adjacent-intersection pruning, and two-pass stream traversal (`1 -> 3` mark then lowering). The additional `fast` mode remains available and intentionally uses overlap-seed + local BFS flattening.
- Correctness note: Default behavior is now aligned to legacy intent; remaining differences are primarily implementation-detail/rounding sensitivity in coordinate-to-cell mapping and the presence of an optional non-legacy fast mode.

### Performance Assessment
- Verdict: `MixedOrDataDependent`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Data dependent by mode (`legacy` mode near parity with legacy; `fast` mode can be faster when overlap seeding is sufficient).
- Evidence from code: NG `legacy` mode now follows the same major algorithm shape as legacy (row/column geometric intersection scans plus two-pass stream lowering), while `fast` mode keeps the prior overlap+BFS structure with lower geometric overhead but extra per-seed queue/hash churn.
- Tuning opportunities: Benchmark both modes across sparse vs dense crossing datasets, then optimize whichever mode becomes the recommended default for production workloads.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add focused regression fixtures for near-miss/intermediate crossings and adjacent-intersection pruning to lock in legacy-mode parity over future refactors.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Parity risk has been substantially reduced by making legacy-fidelity behavior the default and retaining `fast` as an explicit opt-in mode.

## canny_edge_detection

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/canny_edge_detection.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations follow the same canonical Canny pipeline: Gaussian smoothing, Sobel gradients, non-maximum suppression, double-thresholding, and hysteresis.
- Important differences: For `add_back` behavior on packed RGB inputs, legacy restores modified intensity back into RGB color space, while NG currently returns intensity-domain values (not fully color-restored RGB semantics).
- Correctness note: Edge-map mode is close to equivalent; `add_back` RGB mode appears semantically weaker in NG and should be treated as a parity gap.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both NG and legacy follow the same canonical 5-stage Canny pipeline (Gaussian smoothing → Sobel gradient → non-maximum suppression → double threshold → hysteresis); NG does not have structurally extra passes relative to legacy. NG now parallelizes the first three stages using row-parallel Rayon passes and writes Gaussian results directly into a flat buffer plus uses row-slice writes at hysteresis output, reducing intermediate materialization overhead.
- Tuning opportunities: Resolve packed-RGB `add_back` semantic parity and benchmark memory traffic across the remaining staged buffers.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Fix RGB `add_back` semantics to match legacy intent, then optimize early-stage pipeline parallelism.

### Recommended Action
- `SemanticRedesignAndPerformanceTuning`

### Notes
- This tool has clear, actionable parity and throughput work items without requiring a full algorithm rewrite.

## ceil

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/ceil.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply ceiling transform to each valid raster cell.
- Important differences: NG routes through shared unary-math macro infrastructure.
- Correctness note: Cell transform and nodata behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary raster kernels; NG has lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## centroid_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/centroid_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations accumulate row/column coordinates per positive patch ID and place each patch ID at its centroid cell in an output raster.
- Important differences: NG uses hash-map accumulation rather than dense ID-range vectors and returns the same style of text report via tool outputs.
- Correctness note: Core centroid computation semantics match legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie, data dependent.
- Evidence from code: NG parallelizes global accumulation with Rayon fold/reduce; legacy uses simple sequential dense-vector accumulation. NG no longer performs a full output initialization pass, reducing redundant raster-wide writes.
- Tuning opportunities: If patch IDs are dense integer ranges, consider an optional dense-array accumulation fast path to reduce hash-map overhead further.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- NG now emits centroid report rows in deterministic ascending patch-ID order.

## centroid_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/centroid_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both versions output a single centroid for point layers and per-feature centroids for non-point layers, preserving source attributes for feature outputs.
- Important differences: NG uses unified geometry coordinate collectors and parallel feature processing for non-point geometry.
- Correctness note: No major semantic mismatch was identified.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature centroid computation for non-point inputs and uses parallel reduction for point-layer global centroid totals; legacy runs fully sequential loops.
- Tuning opportunities: Minor: avoid transient coordinate buffers for very large geometries with streaming coordinate sums.

### Design Improvements
- Severity: `None`
- Opportunity: No major redesign required from this code audit.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- A good example of an NG modernization that appears both faithful and faster.

## change_vector_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/change_vector_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations compute CVA magnitude (Euclidean norm of per-band deltas) and direction code (bit-encoded sign pattern) across equal-length date stacks.
- Important differences: Legacy emits a textual direction-key legend as part of the output contract; NG currently outputs only magnitude and direction rasters.
- Correctness note: Numerical raster core appears aligned; missing key output is a contract-level parity gap.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-cell CVA evaluation over all bands in one pass. Legacy iterates by band and then by rows/cols in serial nested loops.
- Tuning opportunities: Minor: reduce final sequential `set` writes via row-slice writes when available.

### Design Improvements
- Severity: `Minor`
- Opportunity: Restore/optionally emit the legacy direction-key text report to close output-contract parity.

### Recommended Action
- `PerformanceTuning`

### Notes
- This is mostly a contract-completeness issue rather than algorithmic mismatch.

## circular_variance_of_aspect

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/circular_variance_of_aspect.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute local circular variance of aspect with smoothing plus neighbourhood resultant-length aggregation, and NG now applies CRS-aware derivative scaling for geographic datasets.
- Important differences: Legacy retains a more specialized smoothing pipeline and staged integral-image implementation details that are not fully mirrored in NG.
- Correctness note: The major CRS/gradient-semantic mismatch has been addressed; remaining differences are primarily in smoothing implementation and performance profile.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGUsesExtraPasses`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy parallelizes heavy aspect-analysis phases and uses integral-image driven neighbourhood evaluation with optimized smoothing branches; NG now also parallelizes DEM extraction, aspect-vector derivation, and final integral-window aggregation. NG remains staged and pass-heavy, but the prior serial preparation bottlenecks have been reduced.
- Tuning opportunities: Rework NG around a closer integral-image and parallel stage layout for larger filters, and reduce intermediate row materialization where feasible.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Align NG smoothing and CRS-sensitive derivative semantics with legacy before doing micro-optimizations.

### Recommended Action
- `AcceptWithFollowUpOptimization`

### Notes
- Prior high-risk CRS derivative mismatch has been addressed; remaining gaps are implementation-level smoothing differences.

## classify_buildings_in_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/classify_buildings_in_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools classify LiDAR points as building class (6) when points fall within building footprint polygons, including hole handling.
- Important differences: Legacy uses explicit threaded point classification over polygon candidates; NG uses pre-prepared polygon structures but a simpler sequential membership loop.
- Correctness note: No major semantic mismatch was identified in the classification rule itself.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG now parallelizes per-point building-membership classification with Rayon, removing the prior single-thread loop. Legacy still benefits from mature threaded point tests, and both remain exposed to polygon-candidate lookup costs.
- Tuning opportunities: Add polygon spatial indexing (bbox grid/tree) to reduce O(points × polygons) candidate scans.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add a spatial index over prepared polygon extents and parallel point classification to recover expected LiDAR throughput.

### Recommended Action
- `PerformanceTuning`

### Notes
- This appears semantically stable but is a clear performance catch-up candidate.

## classify_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/classify_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations target the same high-level LiDAR classification outcome (ground, buildings, vegetation, unclassified) using local geometric cues, residual heights, and segmentation-style cluster logic.
- Important differences: NG mirrors the multi-stage legacy flow but with a modernized/simplified implementation layout and helper abstractions. Some tie-breaking and neighborhood details are not identical line-for-line.
- Correctness note: Behavior appears close but not guaranteed bit-for-bit equivalent; this is best treated as near-parity rather than strict semantic identity.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie, data dependent.
- Evidence from code: NG now parallelizes dominant per-point phases (local geometry estimation and residual/pre-morphology preparation) within single-tile `run_single`, and recent cleanup removed a redundant stage-1 materialization/copy pass (`Vec<(planar,linearity)> -> two vectors`) while parallelizing final class-write output assembly. Several later cluster-propagation/refinement stages remain serial and preserve a slight potential legacy edge on complex scenes.
- Tuning opportunities: Parallelize selected refinement stages with deterministic merge rules and add representative urban/forest benchmark fixtures.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Keep the current maintainable NG architecture, but recover legacy-grade throughput by parallelizing the dominant per-point stages.

### Recommended Action
- `PerformanceTuning`

### Notes
- This looks like a case where maintainability improved but single-tile throughput regressed.

## classify_overlap_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/classify_overlap_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations identify overlap cells with multiple point source IDs and apply the same criterion families (`max scan angle`, `not min point source id`, `not min time`, `multiple point source IDs`) for classifying/removing overlap points.
- Important differences: NG uses a cell-hash aggregation model; legacy uses kd-tree searches over grid-cell centres. The output behavior is aligned conceptually.
- Correctness note: No major semantic mismatch was identified from code review.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG builds grid-cell groupings with parallel folds/reduces and then classifies points per cell. Legacy performs repeated kd-tree neighborhood scans for each cell, which is generally heavier at scale.
- Tuning opportunities: Minor: add optional streaming mode for extremely large clouds to reduce peak memory in cell hash aggregation.

### Design Improvements
- Severity: `None`
- Opportunity: No major redesign required based on current audit.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This appears to be a strong NG port with both semantic and performance posture improved.

## clean_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/clean_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations remove null/invalid geometries and preserve valid features with attributes.
- Important differences: NG routes through generic `clean_geometry` logic over broader geometry support, while legacy contains explicit shape-type-specific checks (notably polygon-part handling and ring closure behavior).
- Correctness note: NG is likely more general, but exact edge-case outcomes for certain multipart polygon pathologies may differ from legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature cleanup and then writes valid outputs; legacy runs sequential record-by-record loops.
- Tuning opportunities: Minor: if needed, avoid full prepared row materialization by streaming cleaned features with bounded buffering.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted regression fixtures for degenerate multipart polygon cases to lock in intended behavior versus legacy.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This tool likely benefits from NG’s generic geometry pipeline without obvious downside.

## clip

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/clip.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both clip vector features against polygon boundaries with geometry intersection output.
- Important differences: NG uses consolidated GIS overlay framework and modern spatial-index orchestration.
- Correctness note: Core clip semantics are aligned.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker benchmark metadata indicates NG runtime regression for this tested fixture despite architectural modernization.
- Tuning opportunities: Reduce intersection overhead by improving candidate filtering and geometry allocation reuse.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add targeted performance optimization pass for clip-specific geometry intersection hot paths.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics are sound; performance remains the follow-up item.

## clip_lidar_to_polygon

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/clip_lidar_to_polygon.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools retain LiDAR points inside polygon exteriors while respecting polygon holes.
- Important differences: NG uses prepared polygon structures and Rayon point filtering; legacy uses thread workers with per-point polygon/part tests.
- Correctness note: Core inclusion/exclusion semantics match legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both paths parallelize per-point clipping checks and use polygon bbox short-circuiting before full point-in-polygon tests.
- Tuning opportunities: Add polygon spatial indexing (e.g., uniform grid or R-tree) for scenes with very large polygon counts.

### Design Improvements
- Severity: `Minor`
- Opportunity: Share a common polygon-index utility with other LiDAR polygon tools to improve maintainability and consistency.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This appears to be a faithful and mature port.

## clip_raster_to_polygon

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/clip_raster_to_polygon.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations clip raster cells to polygon extents, preserve hole semantics, and support a maintain/full-dimensions vs cropped extent mode.
- Important differences: NG uses aligned vector loading and generalized polygon collection helpers, while legacy performs direct shapefile loops and manual bbox/scan iteration.
- Correctness note: No major semantic mismatch was identified.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations iterate polygon bbox windows and test pixel centers against polygon membership. Neither has substantial intra-polygon parallelism in the core clipping loops.
- Tuning opportunities: Parallelize per-polygon or per-row chunks for large polygon masks and high-resolution rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider optional scanline/rasterized mask acceleration for very large clip polygons.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Behavior and structure look stable; most gains now are likely in optional acceleration paths.

## closing

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/closing.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations perform morphological closing as dilation followed by erosion using rectangular structuring windows.
- Important differences: NG reuses generic morphology helpers (`morph_dilate`/`morph_erode`) across multiple tools rather than a single dedicated implementation.
- Correctness note: No semantic mismatch in operation order or output meaning was identified.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-row morphology stages with Rayon and shared helper paths; legacy uses thread channels and more manual per-thread orchestration.
- Tuning opportunities: Minor: add row-slice bulk writes to reduce per-pixel set overhead in final write-back.

### Design Improvements
- Severity: `None`
- Opportunity: No major redesign needed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This appears to be a strong shared-kernel modernization.

## clump

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/clump.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools assign unique IDs to contiguous equal-valued raster patches with selectable 4/8-neighbour connectivity and optional zero background preservation.
- Important differences: NG supports multi-band iteration via shared raster core while preserving legacy behavior for typical single-band categorical use.
- Correctness note: Core flood-fill clumping behavior aligns with legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations rely on essentially serial stack-based connected-component expansion. NG adds coalesced progress and generic data access but no major algorithmic change.
- Tuning opportunities: For very large rasters, explore tiled/union-find parallel connected-component labeling.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional deterministic ID ordering guarantees across bands/tiles if reproducibility across execution strategies becomes important.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Stable parity candidate with limited immediate tuning upside.

## colourize_based_on_class

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/colourize_based_on_class.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools colorize LiDAR points by LAS class with optional class-color overrides and intensity blending, and both support optional unique building colors.
- Important differences: NG uses deterministic hash-style cluster colors for unique-building mode, whereas legacy uses random-like color assignment/segmentation behavior. Visual output differs but intent remains aligned.
- Correctness note: Functional semantics are aligned; visualization identity (exact colors) is intentionally/structurally different.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations support batch parallelism across tiles; per-tile building-cluster segmentation is mostly serial in both.
- Tuning opportunities: Parallelize unique-building clustering stage for dense urban tiles.

### Design Improvements
- Severity: `Minor`
- Opportunity: Provide an optional seed-controlled random color mode to more closely mimic legacy visuals where desired.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This is primarily a visualization parity nuance, not an algorithmic correctness issue.

## colourize_based_on_point_returns

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/colourize_based_on_point_returns.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations colorize only/first/intermediate/last return categories with user-selectable colors and optional intensity blending.
- Important differences: NG normalizes argument alias handling and batching through shared LiDAR tool infrastructure.
- Correctness note: No significant semantic deviation detected.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes across batch files with Rayon and simplifies per-point color assignment flow; legacy relies on manual thread/channel orchestration.
- Tuning opportunities: Minor: introduce per-point parallel map for very large single-tile runs.

### Design Improvements
- Severity: `None`
- Opportunity: No major redesign needed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Another high-confidence parity port with straightforward behavior.

## compactness_ratio

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/compactness_ratio.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools now compute compactness as `area / perimeter` and write the legacy-compatible `COMPACT` field.
- Important differences: NG still uses a parallel feature pass while legacy uses sequential record iteration.
- Correctness note: The compactness metric definition and output field contract are now aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature metric computation; legacy uses sequential record iteration.
- Tuning opportunities: Performance is secondary until metric-definition parity is restored.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures that assert exact `COMPACT` parity on multipart polygons with holes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantic contract gap has been closed; remaining work is benchmark confirmation.

## conditional_evaluation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/conditional_evaluation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations evaluate per-cell conditional expressions and emit true/false branch values from constants or rasters.
- Important differences: NG additionally allows expression-based `true`/`false` branches and defaults missing branches to `nodata`, extending legacy behavior.
- Correctness note: Core if-then-else semantics are preserved; NG behavior is a strict superset in branch-expression capability.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Legacy parallelizes row processing across threads; NG currently evaluates expressions in a single nested row/column loop with per-cell context updates.
- Tuning opportunities: Parallelize by row blocks and pre-bind immutable expression context fragments.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep NG expression extensibility but recover legacy-like throughput with parallel evaluation partitions.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Functional parity is acceptable; this is mainly a throughput risk.

## conservative_smoothing_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/conservative_smoothing_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply conservative smoothing by clamping center cells within neighborhood min/max bounds.
- Important differences: NG hosts the implementation in shared rank-filter tooling.
- Correctness note: Filter behavior and parameter semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform neighborhood min/max scans with row parallelization; tested runtime delta is small.
- Tuning opportunities: Minor neighborhood-access caching improvements.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with near-neutral runtime gap.

## construct_vector_tin

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/construct_vector_tin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools build polygon TINs from Delaunay triangulation and now support `use_z` geometry-Z elevation mode with legacy-style `CENTROID_Z` and `HILLSHADE` outputs.
- Important differences: NG keeps a parallelized triangle preparation path and modern vector abstractions, while legacy is more monolithic/sequential.
- Correctness note: The previously reported output-contract gap has been addressed.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use Delaunay triangulation core; NG parallelizes feature extraction and triangle filtering, while legacy adds per-triangle hillshade/centroid computations.
- Tuning opportunities: Performance is secondary until semantic parity is restored.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for mixed point/multipoint inputs and edge-length filtering with `use_z` enabled.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Output contract and elevation-mode parity are now substantially aligned.

## contours_from_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/contours_from_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/contour_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations contour point-elevation TINs using interval/base controls, optional Z sourcing, edge-length filtering, and smoothing.
- Important differences: NG uses a modernized segment chaining pipeline and parallel triangle processing; legacy uses kd-tree-driven stitching/cleanup steps.
- Correctness note: No material semantic mismatch was identified in contour generation intent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes triangulation-segment extraction over triangles and uses streamlined chaining; legacy is more serial and branch-heavy in assembly loops.
- Tuning opportunities: Validate memory scaling for dense triangulations with very small contour intervals.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional debug/strict stitching mode for pathological geometric edge cases.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong modernization candidate with maintained behavior.

## contours_from_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/contours_from_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/contour_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations generate contour polylines from raster surfaces with interval/base, smoothing, and deflection-tolerance simplification controls.
- Important differences: NG uses explicit marching-squares segment generation plus chaining; legacy uses a more procedural edge-tracking construction.
- Correctness note: Parameter semantics and output intent align with legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes cell-level segment extraction and keeps post-processing focused; legacy involves several serial stitching passes and kd-tree lookups.
- Tuning opportunities: Consider parallelizing feature emission for extremely large contour sets.

### Design Improvements
- Severity: `None`
- Opportunity: No redesign required.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity and architecture refresh.

## convergence_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/convergence_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/basic_terrain_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute convergence/divergence index from neighbour-aspect alignment and preserve projected-vs-geographic handling behavior.
- Important differences: Both implementations use the same two-stage workflow (full aspect pass followed by convergence evaluation from neighbour aspects), with differences mainly in threading/runtime plumbing.
- Correctness note: Core metric definition and output meaning align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie, with possible legacy edge on memory traffic.
- Evidence from code: Legacy and NG both materialize an intermediate aspect raster and then compute convergence in a second pass; neither fuses these stages. Differences are primarily in row-worker/channel orchestration (legacy) versus Rayon chunk execution and row-slice writes (NG).
- Tuning opportunities: Benchmark only; no clear low-risk legacy-aligned pass-fusion opportunity exists because both codepaths already share the same stage structure.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for pit/edge behaviour consistency across projected and geographic modes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Classification corrected: this is a loop-shape parity case rather than NG extra-pass divergence.

## convert_nodata_to_zero

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/convert_nodata_to_zero.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools replace `nodata` cells with zero while leaving valid raster values unchanged.
- Important differences: NG implementation is integrated with typed-output infrastructure and optional output path semantics.
- Correctness note: Core transformation behavior matches legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses `par_fill_with` over raster cells; legacy uses manual thread/channel row batching.
- Tuning opportunities: Minimal; this is already a simple memory-bandwidth-limited operation.

### Design Improvements
- Severity: `None`
- Opportunity: None significant.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity case.

## corner_detection

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/corner_detection.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations perform hit-and-miss style corner pattern detection on binarized foreground/background neighborhoods.
- Important differences: NG explicitly normalizes input to binary first via shared helper and then applies the same four pattern templates.
- Correctness note: Pattern logic and output intent match legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon over flattened cell indices with contiguous output buffers; legacy uses per-row thread channels.
- Tuning opportunities: Minor write-back batching could further reduce per-cell set overhead.

### Design Improvements
- Severity: `None`
- Opportunity: None substantial.

### Recommended Action
- `AcceptAsIs`

### Notes
- Clean parity port with shared morphology infrastructure.

## correct_vignetting

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/correct_vignetting.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools apply cosine-model vignetting correction using principal point, focal length, image width, and exponent `n`, with output rescaling to input brightness range.
- Important differences: NG uses shared vector-point parsing and packed-RGB HSI conversions via centralized color helpers.
- Correctness note: Core radiometric correction math aligns with legacy behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes major passes (unscaled computation, extrema reductions, final write preparation) with Rayon; legacy relies on manual thread/channel loops.
- Tuning opportunities: Consider fusing min/max scan with first pass to reduce memory traffic on very large images.

### Design Improvements
- Severity: `Minor`
- Opportunity: Validate parity on edge RGB types and nodata-heavy scenes with regression tests.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity with modernized internals.

## cos

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/cos.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations apply cosine to each non-nodata raster cell and preserve nodata cells.
- Important differences: NG routes through shared unary-math kernel infrastructure used by the broader trig/math family.
- Correctness note: Transformation semantics are unchanged.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses a common unary kernel path (`apply_unary_math_from`) with contiguous data operations, avoiding explicit channel synchronization.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None needed.

### Recommended Action
- `AcceptAsIs`

### Notes
- Straightforward parity case.

## cosh

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/cosh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute hyperbolic cosine on non-nodata raster cells with nodata passthrough.
- Important differences: NG again uses shared unary-math framework rather than per-tool custom loops.
- Correctness note: Semantics match legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Shared unary kernel and reduced orchestration overhead compared with legacy thread/channel implementation.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None needed.

### Recommended Action
- `AcceptAsIs`

### Notes
- Another high-confidence math-tool parity port.

## cost_allocation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/cost_allocation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations propagate source IDs through backlink flow directions to allocate each reachable cell to a source region.
- Important differences: NG adds explicit loop-step safeguards and broader raster-grid validation via shared GIS core.
- Correctness note: Allocation semantics and pointer interpretation remain consistent with legacy D8 backlink conventions.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Core path-follow and backfill loops are largely serial in both implementations; NG parallelizes initialization but not full propagation stage.
- Tuning opportunities: Explore path-compression style memoization and parallel chunked propagation for very large grids.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add an optional fast mode using union/path compression techniques where backlink graphs are acyclic and well-formed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good functional parity with room for deeper acceleration later.

## cost_distance

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/cost_distance.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute accumulated-cost and backlink rasters from source and cost grids using D8 neighbourhood propagation with priority-queue expansion.
- Important differences: NG wraps outputs in typed dual-result infrastructure (`cost_accum` and `backlink`) and strengthens grid-compatibility validation.
- Correctness note: Core cost and backlink semantics align with legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on mostly serial heap-driven expansion; NG parallelizes setup and output handling but frontier relaxation remains single-queue dominated.
- Tuning opportunities: Consider bucketed/radix queues or tile-frontier decomposition for large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional algorithm mode for faster integer-cost surfaces.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity with moderate headroom for advanced queue optimizations.

## cost_pathway

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/cost_pathway.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations trace least-cost pathways from destination cells through backlink pointers and increment pathway counts along traversed routes.
- Important differences: NG adds explicit maximum-step safeguards and shared validation/helpers.
- Correctness note: Path-tracing behavior and `zero_background` semantics are consistent with legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are primarily serial pointer-follow traversals with similar complexity and branching structure.
- Tuning opportunities: Path compression/cache of previously traced segments to reduce repeated traversals.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional compressed backtrace cache for dense destination sets.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Stable parity candidate.

## count_if

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/count_if.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both count per-cell condition matches across input rasters against a comparison rule/value.
- Important differences: NG is integrated into raster-stats framework with shared argument and output plumbing.
- Correctness note: Counting semantics and nodata handling are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel stack scans; NG tested benchmarks indicate substantial runtime reduction.
- Tuning opportunities: Minimize repeated condition parsing overhead for large raster lists.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output of per-input contribution counts for diagnostics.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## create_colour_composite

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/create_colour_composite.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compose packed RGBA outputs from red/green/blue rasters, optional opacity raster, optional BCE enhancement, and optional zero-as-nodata treatment.
- Important differences: NG uses packed RGB metadata/nodata conventions in shared raster core that differ slightly from legacy output nodata handling.
- Correctness note: Core compositing semantics align, with minor output-metadata contract differences.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon-based per-cell compositing and shared optimized BCE path; legacy uses manual thread/channel orchestration plus serial enhancement passes.
- Tuning opportunities: Validate BCE-pass scaling on very large scenes with optional tiled enhancement.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document nodata/alpha contract explicitly to avoid downstream interpretation mismatches.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Good functional parity with small contract nuance.

## create_plane

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/create_plane.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithNGCorrection`
- Summary: Both tools generate planar rasters from gradient/aspect/constant; NG intentionally evaluates at cell centers while legacy used edge-interpolated coordinates.
- Important differences: NG center-based sampling changes numeric values relative to legacy edge-based coordinates.
- Correctness note: Team decision is to retain NG center sampling as the preferred coordinate convention.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-row plane evaluation with Rayon and sequential row writes; legacy uses manual threaded row channels.
- Tuning opportunities: Semantics should be aligned before further performance tuning.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document coordinate-sampling convention clearly and add an opt-in legacy compatibility mode only if needed later.

### Recommended Action
- `NoAction`

### Notes
- Keep NG implementation as-is; legacy coordinate difference is accepted as intentional correction.

## crispness_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/crispness_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Metric computation is aligned, and NG now supports optional HTML report output while retaining JSON report output.
- Important differences: Legacy is report-file-first; NG remains JSON-first with optional HTML report generation.
- Correctness note: Core formula behavior is aligned; remaining differences are output defaults and report UX.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon reductions for count/sum/variance terms and avoids HTML file I/O path in-core.
- Tuning opportunities: Performance is secondary to restoring legacy-equivalent output modes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional report wording/layout refinements only.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core formula parity is good; interface parity is not.

## cross_tabulation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/cross_tabulation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Core contingency-table computation is aligned, and NG now supports optional legacy-style HTML table report output while keeping JSON report output.
- Important differences: Legacy is HTML-report-first and desktop-open oriented; NG remains JSON-first with optional HTML report generation.
- Correctness note: Core computation is aligned; remaining differences are output defaults and UX behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel fold/reduce counting over raster cells and avoids HTML generation/I/O in the core run path.
- Tuning opportunities: Performance tuning is secondary until output-contract parity is restored.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional table formatting tweaks for tighter visual parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Numeric logic is good; report-mode parity is the blocker.

## csv_points_to_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/csv_points_to_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools import CSV point records to vector points using configurable X/Y field indices, inferred attributes, and optional CRS/EPSG assignment.
- Important differences: NG writes through unified vector backends and typed field parsing helpers.
- Correctness note: Core import behavior and coordinate extraction semantics match legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by CSV parsing and row-wise feature emission with little room for parallel acceleration.
- Tuning opportunities: Optional chunked parsing for very large CSVs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit delimiter override option for non-standard CSV variants.

### Recommended Action
- `AcceptAsIs`

### Notes
- Stable conversion parity case.

## cumulative_distribution

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/cumulative_distribution.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations convert non-nodata raster values to cumulative probabilities via histogram/CDF mapping.
- Important differences: NG uses parallel reductions and robust edge handling for near-constant ranges.
- Correctness note: Output meaning is aligned with legacy CDF semantics.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes min/max scan, histogram construction, and output mapping; legacy performs serial two-pass loops.
- Tuning opportunities: Evaluate adaptive bin count selection for speed/precision trade-offs.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity and modernization.

## curvedness

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/curvedness.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute curvedness as RMS combination of principal curvatures over local neighborhoods.
- Important differences: NG integrates curvedness into shared Pro-curvature dispatch with consolidated helpers.
- Correctness note: Curvature metric intent and output semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both rely on neighborhood derivative kernels; NG tested benchmarks show moderate speedup.
- Tuning opportunities: Add micro-optimizations for repeated derivative coefficient reuse.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for high-curvature synthetic surfaces.

### Recommended Action
- `Accept`

### Notes
- Tested parity is strong and stable.

## d8_flow_accum

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/d8_flow_accum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute D8 flow accumulation from either a pointer raster or a DEM-derived pointer field.
- Important differences: NG adds explicit `input_is_pointer` flexibility and routes through the shared flow-algorithm module.
- Correctness note: Accumulation semantics and output meaning are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both rely on flow-topology propagation; NG has less orchestration overhead and cleaner buffering.
- Tuning opportunities: Benchmark pointer-vs-DEM input paths separately.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document pointer-input mode explicitly in user-facing docs.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## d8_mass_flux

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/d8_mass_flux.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools route mass downslope using D8 flow routing from DEM-derived directions with loading, efficiency, and absorption controls.
- Important differences: NG uses local D8 direction/inflow helpers and modernized raster utilities, while preserving efficiency scaling behavior (percent vs proportion).
- Correctness note: Mass-routing equation and accumulation behavior align with legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG parallelizes inflow counting setup but core stack-based propagation remains largely serial in both.
- Tuning opportunities: Parallel frontier processing for independent sub-basins.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional strict legacy warning/output text parity around interior pits.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong algorithmic parity with incremental infrastructure improvements.

## d8_pointer

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/d8_pointer.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both derive D8 flow direction pointers from DEMs using the same cardinal/intercardinal encoding scheme.
- Important differences: NG consolidates pointer logic with reusable decode helpers and module-level flow utilities.
- Correctness note: Flow encoding and nodata behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel raster passes; NG avoids some manual thread orchestration in favor of shared hydrology helpers.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## dbscan

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/dbscan.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform density-based clustering on raster feature stacks with scaling options and mark noise as nodata.
- Important differences: Legacy relies on linfa DBSCAN workflow; NG uses an in-house KD-tree DBSCAN implementation.
- Correctness note: Clustering intent is aligned, but implementation and potential cluster-label ordering/details may differ.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG parallelizes preprocessing/scaling and feature extraction but executes cluster expansion in a primarily serial region-growing loop, similar to legacy bottlenecks.
- Tuning opportunities: Parallel neighbourhood expansion or batched region-growth strategies for large valid-pixel sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional deterministic label-order mode and explicit compatibility notes for label numbering.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Behavior is close enough for audit progression, with some reproducibility nuances.

## decrement

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/decrement.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both subtract a scalar value from each valid raster cell.
- Important differences: NG exposes the decrement amount as a configurable `value` parameter, while legacy is fixed at 1.0.
- Correctness note: Default behavior matches legacy when `value=1.0`.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary scalar transforms; NG uses a shared unary-math kernel with less boilerplate.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the configurable decrement value as a compatibility extension.

### Recommended Action
- `Accept`

### Notes
- Good parity with added flexibility.

## dem_void_filling

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/dem_void_filling.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/dem_void_filling.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools implement the same delta-surface fusion workflow (resample fill DEM, estimate edge offsets, interpolate offsets into void interiors, and synthesize filled elevations).
- Important differences: Legacy resamples fill values using neighbour-wise weighting that can still return values when some neighbours are nodata, while NG uses stricter bilinear sampling and drops cells when any of the four source neighbours are nodata. NG also enforces a minimum `weight_value` floor.
- Correctness note: Core algorithm intent matches, but outputs can diverge near fill-data gaps and edge conditions.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations parallelize the dominant raster passes; NG uses Rayon row-parallel collections, while legacy uses manual thread/channel fan-out.
- Tuning opportunities: Reduce temporary whole-raster buffers during offset interpolation/application.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add a legacy-compatible fill-resampling mode for nodata-edge behavior parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Algorithm family is preserved; edge-case sampling semantics should be explicitly documented.

## depth_in_sink

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/depth_in_sink.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute sink depth as `(filled_dem - original_dem)` with optional zero-valued background for non-sink cells.
- Important differences: NG delegates depression filling to shared `fill_depressions_core` helpers; legacy performs an explicit in-tool priority-region grow/fill routine.
- Correctness note: Output meaning and sink-depth criteria align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy parallelizes pit discovery; NG relies on shared fill helpers and serial difference pass. Dominant work remains depression filling in both.
- Tuning opportunities: Parallelize final depth differencing pass in NG for very large rasters.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity case with different internal organization.

## depth_to_water

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/depth_to_water.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools target cartographic DTW from stream/lake source features and now both seed full lake polygon interiors (with hole handling) plus stream line sources.
- Important differences: Legacy still includes richer geographic-distance handling and nuanced slope-cost treatment, while NG retains a streamlined isotropic slope-cost accumulation path.
- Correctness note: The prior lake-interior source mismatch is resolved; remaining divergence is primarily in geographic-distance/cost modeling details.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses streamlined source rasterization helpers and a compact heap-based accumulation loop; legacy performs more complex rasterization logic and geographic-distance branching.
- Tuning opportunities: Prioritize semantic parity restoration before further performance tuning.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add legacy-compatible geographic distance handling option for geographic-coordinate DEMs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Major water-source semantics are aligned; remaining gap is geographic-distance cost behavior.

## deviation_from_mean_elevation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/deviation_from_mean_elevation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute local standardized elevation residuals (local z-scores) from neighbourhood mean and standard deviation using integral-image style window summaries.
- Important differences: Legacy bins elevations to fixed precision before integral computation; NG computes directly from full-precision values.
- Correctness note: Output intent is aligned; minor numeric drift from precision strategy differences is expected.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon for row-parallel window computation and avoids legacy binning/quantization preprocessing overhead.
- Tuning opportunities: Fuse some intermediate row-vector materialization in NG to reduce memory traffic.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optionally expose a legacy-quantized mode for tighter reproducibility when needed.

### Recommended Action
- `AcceptAsIs`

### Notes
- Core DEV semantics are preserved.

## deviation_from_regional_direction

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/deviation_from_regional_direction.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute each polygon's directional deviation from a weighted regional mean orientation using RMA-derived orientation and elongation-based weights.
- Important differences: NG uses shared vector geometry helpers and parallel fold/reduce for regional-angle accumulation.
- Correctness note: The axial-direction treatment and final `DEV_DIR` derivation are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes both regional-orientation accumulation and per-feature deviation computation; legacy uses serial per-record passes.
- Tuning opportunities: Add optional deterministic reduction ordering if exact floating reproducibility is required.

### Design Improvements
- Severity: `Minor`
- Opportunity: Emit optional diagnostics reporting count of polygons used/excluded by elongation threshold.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity with clear scalability improvements in NG.

## diff_of_gaussians_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/diff_of_gaussians_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform Difference-of-Gaussians band-pass filtering with two configurable Gaussian scales.
- Important differences: NG integrates the filter into shared phase-3 infrastructure.
- Correctness note: Filtering semantics and output intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are convolution-heavy filters; NG uses shared rayon-based infrastructure with less repeated setup.
- Tuning opportunities: Benchmark sigma combinations with wider kernel footprints.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## difference

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/difference.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute set difference for polygon geometry, removing overlapping area from the first input.
- Important differences: NG relies on wbvector topology primitives rather than a manual KdTree/spatial-index implementation.
- Correctness note: Geometry difference semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are geometry-boolean operations dominated by topology resolution and intersection handling.
- Tuning opportunities: Benchmark complex multipart intersections and cache geometry envelopes more aggressively.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong semantic parity.

## difference_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/difference_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute difference curvature as profile curvature minus tangential curvature over DEM neighborhoods.
- Important differences: NG folds the tool into shared pro-curvature enum dispatch.
- Correctness note: Curvature definition and output semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both use neighborhood curvature estimation with similar computational load.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## difference_from_mean_elevation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/difference_from_mean_elevation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute local elevation residuals as each cell minus neighbourhood mean.
- Important differences: Legacy bins elevations to fixed precision before integral-image computation; NG computes directly from full-precision values.
- Correctness note: Output intent is aligned, with only minor numeric drift expected from quantization differences.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon row-parallel windows without legacy binning overhead while preserving integral-sum efficiency.
- Tuning opportunities: Reduce temporary row-buffer allocation in NG writeback path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-quantized mode for strict reproducibility workflows.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity case.

## direct_decorrelation_stretch

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/direct_decorrelation_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools apply direct decorrelation stretch by reducing achromatic content then clipping/stretching channel intensity tails.
- Important differences: NG uses a unified packed-RGB non-filter pipeline with explicit validation and parallel histogram reduction.
- Correctness note: Core DDS transform behavior matches legacy intent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes first-pass transform and histogram aggregation with Rayon reductions; legacy uses thread/channel staging and serial merge loops.
- Tuning opportunities: Fuse final packed-RGB writeback with output allocation to reduce copy overhead.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Modernized implementation with preserved behavior.

## directional_relief

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/directional_relief.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools ray-trace directional neighbourhoods and report mean directional elevation offset from each source cell.
- Important differences: NG normalizes azimuth directly over [0,360), uses explicit bilinear sampling helpers, and differs slightly in edge/step handling versus legacy's axis-intersection ray traversal.
- Correctness note: Metric intent is consistent, but small boundary and interpolation-path differences can cause local value drift.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are heavily ray-tracing dominated and parallelized per-row; NG has cleaner helper-based sampling but still performs long per-cell directional walks.
- Tuning opportunities: Add adaptive step skipping for long homogeneous rays to reduce traversal cost.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional strict legacy stepping mode for reproducibility-sensitive workflows.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Acceptable parity for now with known minor numeric variance risk.

## dissolve

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/dissolve.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both dissolve polygons by shared attribute or global dissolve, merging interior boundaries.
- Important differences: NG exposes multiple strategy modes for dissolve processing rather than a single fixed path.
- Correctness note: Primary dissolve semantics remain aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are topology-heavy geometry operations; NG strategy selection changes structure more than raw complexity.
- Tuning opportunities: Benchmark large multipart dissolve sets under each NG strategy.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the tradeoffs among dissolve strategies more explicitly.

### Recommended Action
- `Accept`

### Notes
- Good semantic parity with more flexible NG execution.

## distance_to_outlet

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/distance_to_outlet.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute downstream stream-network distance to outlet and now honour the same background output contract.
- Important differences: NG uses a path-caching traversal structure while legacy uses outlet-seeded stack propagation.
- Correctness note: NG now respects `zero_background`, assigning either 0 or nodata to non-stream cells as in legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by path-following downstream traversal on stream cells; NG path caching and vectorized buffers keep complexity similar.
- Tuning opportunities: Secondary until semantic output parity is fixed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a regression test that validates `zero_background = true/false` output toggling on non-stream cells.

### Recommended Action
- `Accept`

### Notes
- Output-contract parity is restored.

## diversity_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/diversity_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both count unique values in a moving window to produce a diversity raster.
- Important differences: NG is bundled into shared rank-filter infrastructure.
- Correctness note: Window uniqueness semantics and nodata behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are neighborhood scans; NG benefits from shared kernel infrastructure and improved parallelism.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## divide

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/divide.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both divide one raster by another cell-by-cell and preserve nodata semantics.
- Important differences: NG includes cleaner unified binary-op dispatch and explicit scalar/binary variants.
- Correctness note: Default division semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary raster kernels; NG uses shared dispatch and lower overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider documenting zero-division handling more explicitly if not already covered elsewhere.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## downslope_distance_to_stream

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/downslope_distance_to_stream.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute distance to streams along downslope flowpaths and support D8 and D-Infinity routing modes.
- Important differences: NG uses shared D8/DInf helpers and queue-based propagation; legacy uses explicit thread/channel blocks.
- Correctness note: Core distance semantics and stream-source behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by routing and propagation passes; legacy has explicit threaded phases while NG has compact helper-based traversal.
- Tuning opportunities: Benchmark D-Infinity mode separately because queue ordering can shift cache behavior.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for interior pits in NG to match legacy warning behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity candidate.

## downslope_flowpath_length

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/downslope_flowpath_length.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools traverse D8 pointers to compute downstream flowpath length with optional watershed-bounded routing and optional per-cell weights.
- Important differences: NG validates optional raster dimensions through shared helpers and uses flattened vector buffers; legacy uses Array2D-backed traversals.
- Correctness note: Pointer decoding, watershed constraints, and weighted length accumulation are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Legacy.
- Evidence from code: Legacy and NG both perform serial path walks, but NG now parallelizes pointer/auxiliary preprocessing and final normalization passes. Core downstream traversal remains serial and path-dependent in both implementations.
- Tuning opportunities: Add path-compression caching and optional independent-watershed tiling.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose explicit output nodata/background mode controls consistent with related stream-distance tools.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity with modest optimization headroom.

## downslope_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/downslope_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute Hjerdt-style downslope index using D8-based path traversal to a specified vertical drop and support tangent/degrees/radians/distance output types.
- Important differences: NG uses shared raster helpers and explicit step guards against runaway loops.
- Correctness note: Metric intent and output-type semantics match legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both versions are flowpath-walk dominated with per-cell iterative tracing; NG parallelizes row computations while legacy parallelizes flow-direction setup plus row computation.
- Tuning opportunities: Reuse cached downstream chains for repeated traversals in low-relief terrain.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep optional legacy-compatible warning text for interior pits to ease script diffs.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## edge_contamination

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/edge_contamination.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools map edge-contaminated cells for D8/MFD/DInf flow assumptions with aligned default z-scaling semantics.
- Important differences: NG now mirrors legacy-style automatic geographic `z_factor` derivation when `z_factor < 0`, while retaining compact internal traversal structure.
- Correctness note: Geographic-coordinate default scaling parity is restored.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now parallelizes the expensive flow-receiver determination for D8/DInf/MFD modes via row-parallel precomputation, while retaining serial deterministic frontier propagation. Legacy and NG remain similar in propagation complexity.
- Tuning opportunities: Explore frontier-batch parallel propagation for very large connected edge regions.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a regression test case for geographic DEM input with `z_factor = -1` to guard parity.

### Recommended Action
- `Accept`

### Notes
- Main semantic blocker resolved.

## edge_density

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/edge_density.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute local density of breaks-in-slope from neighbour normal-vector angular differences and moving-window aggregation.
- Important differences: Legacy auto-adjusts `z_factor` for geographic DEMs, while NG applies user-provided `z_factor` directly; this can shift magnitudes in geographic coordinate systems.
- Correctness note: Core edge-density concept matches, but geographic default behavior differs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes normal computation and edge-mask generation with Rayon and uses compact integral accumulation; legacy performs more threaded channel handoffs.
- Tuning opportunities: Add streaming integral updates to reduce temporary buffer pressure on very large rasters.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional auto-geographic `z_factor` mode for legacy behavior matching.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics are close enough for progression, with one notable default-parameter difference.

## edge_preserving_mean_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/edge_preserving_mean_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply a thresholded smoothing mean that preserves edges by limiting replacement when local deviation exceeds a threshold.
- Important differences: NG operates on the scalar intensity path in shared phase-3 filter infrastructure rather than legacy RGB/HSI-specific plumbing.
- Correctness note: Filtering intent and threshold semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are neighborhood filters; NG avoids RGB/HSI decomposition overhead for single-band workflows and uses shared parallel kernels.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the scalar-vs-RGB behavior difference more clearly for users migrating from legacy.

### Recommended Action
- `Accept`

### Notes
- Strong parity with improved single-band performance.

## edge_proportion

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/edge_proportion.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute per-patch edge-cell proportion and map the value back onto patch cells with legacy-style tabular output.
- Important differences: NG emits tabular text under a structured output key (`table`) rather than legacy tuple-position semantics.
- Correctness note: Raster semantics and tabular content are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon fold/reduce for patch counts and edge counts and parallel output remapping; legacy relies on thread-channel aggregation with extra post-loop text formatting.
- Tuning opportunities: Performance is secondary until report-output parity is decided.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document `table` output-key naming for migration from tuple-style legacy consumers.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Prior report-output gap was an audit artifact; parity appears intact.

## elev_relative_to_min_max

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/elev_relative_to_min_max.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools express elevation as percentage of raster-wide min-max relief.
- Important differences: NG computes min/max with parallel reduce and includes explicit constant-range handling; legacy relies on raster metadata min/max from the input object.
- Correctness note: Core normalization behavior is aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes min/max scan and row transforms with Rayon; legacy uses thread-channel row processing with separate metadata prep.
- Tuning opportunities: Minor memory-write coalescing only.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## elev_relative_to_watershed_min_max

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/elev_relative_to_watershed_min_max.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/hydrologic_index_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute each cell's relative elevation percentage within its watershed zone min-max range.
- Important differences: NG uses hash-map based watershed range accumulation, while legacy uses range-indexed arrays based on watershed min/max IDs.
- Correctness note: Output intent is aligned, and NG appears more robust for sparse/non-contiguous watershed IDs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both require at least two major passes (range discovery + remap); NG uses generic helpers but hash-map operations can offset parallel gains on some datasets.
- Tuning opportunities: Optional integer-zone fast path in NG for dense contiguous watershed IDs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit nodata/constant-range messaging parity with legacy behavior.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity with potentially better robustness in NG.

## elevation_above_stream

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/elevation_above_stream.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute HAND-style elevation above streams along D8-derived downslope flow paths.
- Important differences: NG uses shared D8 helper routines and sentinel handling in flat vectors; legacy uses explicit Array2D + stack propagation.
- Correctness note: Stream-seeded propagation and elevation-difference semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by flow-direction computation plus stack-based upstream propagation; structural differences are mostly implementation plumbing.
- Tuning opportunities: Reuse cached D8 directions when chaining related hydrology tools.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional legacy-style interior pit warning text parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Behavior appears aligned enough for progression.

## elevation_above_stream_euclidean

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/elevation_above_stream_euclidean.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute elevation above nearest stream using straight-line (Euclidean) stream proximity.
- Important differences: Legacy uses two-pass distance-transform style allocation grids; NG uses Dijkstra-like expansion from stream cells while carrying source elevations.
- Correctness note: Resulting metric is aligned with legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both approaches are efficient nearest-source propagations with similar asymptotic behavior; runtime differences will be data-size dependent.
- Tuning opportunities: Optional bucketed queue optimization for uniform-cost neighborhoods.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong semantic match despite different distance-propagation mechanics.

## elevation_percentile

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/elevation_percentile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute elevation percentiles from a moving-window histogram with the same rounding/binning semantics.
- Important differences: NG normalizes filter aliases into shared filter-size parameters and uses consolidated terrain-analysis helpers.
- Correctness note: Percentile computation and output meaning are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel row-wise neighborhood reductions; NG avoids manual thread orchestration.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## eliminate_coincident_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/eliminate_coincident_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations remove near-duplicate points using a spatial proximity search and preserve the first encountered point attributes.
- Important differences: NG permits `tolerance_dist=0` and handles exact duplicates via KD-tree query, while legacy currently rejects non-positive tolerances despite its own docstring indicating `0.0` should be valid.
- Correctness note: NG aligns with documented tool intent better than legacy input validation.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are single-pass spatial-index workflows; NG uses KD-tree radius queries while legacy uses fixed-radius search.
- Tuning opportunities: Optional batched inserts/queries if very large point layers become a hotspot.

### Design Improvements
- Severity: `Minor`
- Opportunity: Decide and document canonical zero-tolerance behavior across legacy and NG.

### Recommended Action
- `AcceptAsIs`

### Notes
- Legacy source contains contradictory geometry/tolerance checks; benchmark parity should be interpreted with that caveat.

## elongation_ratio

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/elongation_ratio.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG now matches legacy elongation semantics: `1 - short_axis/long_axis` based on an oriented minimum bounding box.
- Important differences: NG uses shared minimum-bounding-box helpers and a parallel feature pass; legacy is sequential and tool-local.
- Correctness note: The previous metric-definition mismatch has been resolved.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses simple parallel per-feature bounding-box calculations; legacy computes minimum bounding boxes per polygon with heavier geometry operations.
- Tuning opportunities: Performance tuning should wait until semantic formula parity is restored.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for rotated and multipart polygons to lock parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core metric-definition parity has been restored.

## embankment_mapping

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/embankment_mapping.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG reproduces the same high-level embankment seed/reposition, region growth, and optional embankment-removal workflow.
- Important differences: NG uses refactored raster/vector cores and output-map style (`path`, optional `output_dem`) while legacy returns `(mask_raster, optional_dem)` directly in API form.
- Correctness note: Parameter set and decision rules are broadly aligned, but interpolation edge details should be confirmed with targeted regression scenes.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by queue-based region growing and neighborhood interpolation over raster cells; NG refactors storage and indexing but retains similar algorithmic cost.
- Tuning opportunities: Consider optional parallelization for embankment-removal interpolation when masks are large.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for mask boundaries and removed-DEM interpolation stability.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Substantial semantic parity appears present; confirm numerically on known embankment test sets.

## emboss_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/emboss_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply an 8-direction emboss convolution with matching kernel weights and direction modes.
- Important differences: NG sits inside shared convolution infrastructure.
- Correctness note: Kernel semantics and direction selection align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are separable convolution-style filters; NG uses consolidated filter infrastructure with lower setup overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## equal_to

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/equal_to.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compare two rasters cell-by-cell and emit 1 when equal, 0 when not equal, with nodata propagation.
- Important differences: NG uses shared binary math dispatch.
- Correctness note: Equality semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary raster comparisons; NG reduces boilerplate and channel overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## erase

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/erase.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both erase input vector features using polygon erase geometries and preserve non-erased portions.
- Important differences: NG uses wbtopology spatial indexing and parallel processing; legacy uses KdTree-based spatial acceleration.
- Correctness note: Erase semantics are aligned for valid polygon erase layers.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker parity band is red, indicating NG is slower on the tested fixture despite architectural modernization.
- Tuning opportunities: Reduce geometry intersection overhead and candidate generation cost in NG.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Improve NG erase candidate pruning and geometry allocation reuse before considering broader refactors.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics are fine; performance is the main gap.

## erase_polygon_from_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/erase_polygon_from_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools now implement erase semantics by keeping points outside polygon geometry.
- Important differences: NG uses prepared polygon predicates with parallel filtering; legacy uses bbox screening and explicit part-wise point-in-polygon checks.
- Correctness note: Predicate direction now matches legacy erase behavior.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are point-in-polygon dominated; NG parallel filtering and prepared geometry should offset setup cost for larger clouds.
- Tuning opportunities: Add large-cloud benchmark to quantify crossover point where prepared geometry amortizes best.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add clip/erase complementary regression tests to protect against future predicate inversions.

### Recommended Action
- `Accept`

### Notes
- Functional parity blocker resolved.

## erase_polygon_from_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/erase_polygon_from_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools set raster cells inside polygon areas to NoData and preserve values in polygon holes.
- Important differences: NG uses shared polygon collection/containment helpers with row/column bbox restriction; legacy uses explicit per-part loops.
- Correctness note: Hole handling and inside-polygon erase behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations iterate polygon-local raster bounding boxes and perform point-in-polygon tests; complexity drivers are similar.
- Tuning opportunities: Optional scanline fill acceleration for very large polygon masks.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity for expected erase semantics.

## euclidean_allocation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/euclidean_allocation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations assign each non-target cell to the value of its nearest non-zero target cell using the same two-scan Euclidean transform family.
- Important differences: NG routes through shared `euclidean_transform` core and writes F64 output; legacy writes F32 output with equivalent value semantics.
- Correctness note: Allocation behavior and nodata handling are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Core distance/allocation transform remains sequential scan-based in both; NG mainly differs in surrounding I/O/data abstractions.
- Tuning opportunities: Optional band-parallel execution for multi-band inputs.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## euclidean_distance

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/euclidean_distance.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute Euclidean distance to nearest non-zero target cell and preserve nodata in nodata input cells.
- Important differences: NG reuses shared transform internals and emits F64 values; legacy emits F32.
- Correctness note: Distance metric and target definition are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG (slightly).
- Evidence from code: NG parallelizes output square-root/post-scaling phase over cells; core transform cost remains similar.
- Tuning opportunities: Evaluate cache-aware tiling for very large rasters.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good semantic and numerical parity expected.

## evaluate_training_sites

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/evaluate_training_sites.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools evaluate class separability across multi-band training polygons and produce per-class, per-band distribution summaries in HTML.
- Important differences: NG now includes legacy-style box-and-whisker visualizations; remaining differences are primarily in broader report depth (e.g., extended narrative/covariance-style context).
- Correctness note: Core training-site sampling and class-by-band distribution analysis are aligned; the remaining gap is report richness rather than algorithmic mismatch.

### Performance Assessment
- Verdict: `Undetermined`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Not meaningful until semantic parity target is settled.
- Evidence from code: NG uses modern raster/vector helper paths and structured HTML assembly, but it computes a narrower metric set.
- Tuning opportunities: Defer optimization until report/statistics parity scope is finalized.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-style visualization/report sections (e.g., box plots and covariance-oriented summaries) behind a richer-report mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Box-and-whisker visual parity is restored; keep tracking residual report-depth differences only.

## exp

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/exp.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the natural exponential function per valid raster cell.
- Important differences: NG uses shared unary-math infrastructure.
- Correctness note: Mathematical transform and nodata handling align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary transforms; NG has less execution overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## exp2

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/exp2.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute base-2 exponential per valid raster cell.
- Important differences: NG uses shared unary-math infrastructure.
- Correctness note: Numeric transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary transforms with low arithmetic complexity; NG has lower overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## export_table_to_csv

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/export_table_to_csv.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools export vector attribute tables to CSV with optional header row.
- Important differences: NG applies consistent CSV escaping/quoting through field-value conversion helpers; legacy uses manual formatting with type-specific formatting logic.
- Correctness note: Core export semantics match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG builds row strings in parallel (`par_iter`) before sequential write; legacy builds each row in a single sequential loop.
- Tuning opportunities: Stream parallel row chunks directly to buffered writer if memory pressure appears for very large tables.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- NG also improves robustness of CSV quoting behavior.

## exposure_towards_wind_flux

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/exposure_towards_wind_flux.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute wind-flux exposure from slope, aspect, and upwind horizon angle using the same conceptual formulation and azimuth-driven ray tracing.
- Important differences: NG uses refactored raster kernels and explicit offset list processing; legacy uses thread-channel stages with similar math.
- Correctness note: Formula and major parameter semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform heavy per-cell horizon scanning and multi-stage parallel row processing; cost profile is dominated by ray-offset evaluation.
- Tuning opportunities: Share/cached azimuth offset tables across runs with same grid geometry.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit regression set for geographic CRS z-factor handling edge cases.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong algorithmic parity with implementation modernization.

## extend_vector_lines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/extend_vector_lines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools extend polyline endpoints by a specified distance at start, end, or both ends.
- Important differences: NG additionally supports multiline geometry containers while preserving the same extension direction semantics.
- Correctness note: Endpoint extension logic and defaults are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes feature processing with Rayon; legacy iterates records sequentially.
- Tuning opportunities: Minimal; performance already dominated by lightweight per-feature trigonometry.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity with modest scalability improvement in NG.

## extract_by_attribute

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/extract_by_attribute.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools evaluate a boolean expression against attributes and output matching features.
- Important differences: NG validates expressions up front and parallelizes feature evaluation; legacy performs expression evaluation in a sequential loop.
- Correctness note: Expression model and null/pi alias handling are preserved.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG executes per-feature expression evaluation in parallel then compacts selected features; legacy processes serially.
- Tuning opportunities: Consider short-circuit schema-to-context mapping caches for very wide attribute tables.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with better scaling characteristics in NG.

## extract_nodes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/extract_nodes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations convert polyline/polygon vertices into output point features with `FID` and `PARENT_ID` attributes.
- Important differences: Legacy hard-errors when input base geometry type is not polyline/polygon; NG skips unsupported feature geometries rather than failing the whole run.
- Correctness note: For valid line/polygon datasets, node extraction output semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes feature-to-node expansion and then assigns final IDs; legacy processes records sequentially.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Decide whether NG should enforce legacy-style hard validation for non line/polygon content.

### Recommended Action
- `AcceptAsIs`

### Notes
- Divergence is mostly in validation strictness, not core extraction behavior.

## extract_raster_values_at_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/extract_raster_values_at_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools sample one or more rasters at point locations, append `VALUE*` fields, and emit a textual point-value report.
- Important differences: NG emits the report via structured outputs map (`report`) rather than tuple return shape, while preserving content intent.
- Correctness note: Sampling behavior and field-append semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature sampling across points; legacy loops sequentially through records and rasters.
- Tuning opportunities: Optional spatial locality batching by raster block for very large point sets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity for common point-sampling workflows.

## extract_streams

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/extract_streams.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both extract stream cells from a flow-accumulation raster using a threshold and emit a binary stream raster.
- Important differences: NG is wired through the stream-network-analysis module infrastructure.
- Correctness note: Threshold and output semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are row-wise raster threshold passes; NG uses consolidated tool infrastructure with lower overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## extract_valleys

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/extract_valleys.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools support LQ, Johnston-and-Rosenfeld, and Peucker-and-Douglas valley extraction variants with optional line thinning.
- Important differences: NG is routed through a consolidated stream-analysis dispatcher and helper cores; legacy is a dedicated monolithic tool body.
- Correctness note: Variant selection rules, filter-size oddification in LQ mode, and line-thinning behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations include heavy neighborhood scans and iterative thinning; parallelism benefits vary by selected variant and DEM size.
- Tuning opportunities: Reintroduce/expand parallel row scheduling for LQ and JandR variants in NG where deterministic behavior is preserved.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures that compare outputs across all three variants and line-thin on/off combinations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good semantic parity confidence across variant modes.

## farthest_channel_head

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/farthest_channel_head.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools output upstream distance-to-farthest-head values for stream cells.
- Important differences: NG uses an inflow-count + downstream max-propagation structure and now honours `zero_background` for non-stream cells.
- Correctness note: Output contract now aligns for stream distances and background assignment.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by stream-network dependency traversal and downstream propagation passes.
- Tuning opportunities: Benchmark very large dendritic networks for stack/queue ordering cache effects.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add dedicated regression comparing distance values and `zero_background` behavior.

### Recommended Action
- `Accept`

### Notes
- Prior audit diagnosis was stale; parity blocker is resolved.

## fast_almost_gaussian_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/fast_almost_gaussian_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both implement a fast Gaussian approximation filter with comparable sigma-driven smoothing behavior.
- Important differences: NG is embedded in shared phase-3 filter infrastructure.
- Correctness note: Approximation goal and output intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both use separable fast smoothing; NG benefits from shared infrastructure and rayon scheduling.
- Tuning opportunities: Benchmark very large sigma settings for cache behavior.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document approximation quality tradeoffs versus exact Gaussian blur.

### Recommended Action
- `Accept`

### Notes
- Good tested parity.

## fd8_flow_accum

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fd8_flow_accum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute FD8 flow accumulation with fractional divergence and convergence controls.
- Important differences: NG factors the logic into reusable flow-algorithm helpers and supports a shared module interface.
- Correctness note: FD8 accumulation behavior is aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are propagation-heavy accumulation kernels; NG has cleaner core reuse and less wrapper overhead.
- Tuning opportunities: Validate behavior on large flat areas with many converging contributors.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document convergence-threshold behavior clearly in the NG tool help.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## fd8_pointer

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fd8_pointer.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute FD8 flow-direction pointers from a DEM.
- Important differences: NG exposes the core logic through reusable flow-algorithm helpers.
- Correctness note: Direction encoding and nodata handling are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are raster direction passes; NG has lower orchestration overhead and cleaner helper reuse.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## feature_preserving_smoothing

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/feature_preserving_smoothing.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations use normal-vector smoothing and iterative DEM updates to reduce roughness while preserving edges.
- Important differences: NG introduces SIMD-accelerated interior-window processing and ping-pong buffers while preserving core parameter semantics (`filter_size`, `normal_diff_threshold`, `iterations`, `max_elevation_diff`, `z_factor`).
- Correctness note: Core algorithmic intent and output characteristics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG combines Rayon parallel row processing with SIMD (`f32x8`) and reduced per-iteration allocation overhead; legacy uses thread-channel stages and full-array style passes.
- Tuning opportunities: Validate SIMD fallback boundaries on very narrow rasters for edge-case consistency.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with clear implementation modernization.

## fetch_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/fetch_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute directional fetch distance to first upwind obstacle using DEM ray tracing and height-increment thresholding.
- Important differences: NG uses consolidated bilinear sampling and a unified step-based ray traversal; legacy uses split horizontal/vertical intersection loops.
- Correctness note: Positive obstacle distance and negative edge-truncated distance conventions are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by per-cell ray casting and parallel row execution; asymptotic cost profile is similar.
- Tuning opportunities: Add optional max-distance cap parameter parity if future profiling shows long-ray overhead on large DEMs.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence for directional exposure/fetch workflows.

## fill_burn

- Audit date: 2026-05-10
- Tracker status: `Green`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fill_burn.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools hydro-enforce DEMs by burning streams, filling depressions, and re-adjusting stream cells.
- Important differences: Legacy rasterizes streams with an explicit thinning phase before burn/fill; NG relies on shared vector-to-raster masking utilities and applies the same core lower-fill-adjust pattern.
- Correctness note: End-state hydro-enforcement behavior is aligned for standard line-stream inputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by stream rasterization plus depression-fill passes; NG simplifies preprocessing while legacy includes additional thinning work.
- Tuning opportunities: Benchmark stream-mask density sensitivity and optional thinning parity mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style stream thinning toggle in NG for strict preprocessing parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics appear robustly aligned.

## fill_depressions

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fill_depressions.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both fill depressions using the same priority-queue pit visitation logic and support the same flat-resolution options.
- Important differences: NG factors the core into shared hydrology infrastructure and adds cleaner output-path plumbing.
- Correctness note: Depression-fill semantics and capping behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by stateful queue propagation; NG parallelizes surrounding row work but not the core queue.
- Tuning opportunities: Benchmark deep-depression and flat-resolution heavy cases separately.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the shared-core factoring and any optional output-path behavior.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## fill_depressions_planchon_and_darboux

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fill_depressions_planchon_and_darboux.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implement the same four-pass sweep depression-fill algorithm with matching convergence behavior.
- Important differences: NG adds an early-termination changed-flag optimization and stricter nodata handling.
- Correctness note: End-state filled DEM semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both remain sweep-based and sequential in the core algorithm.
- Tuning opportunities: Track convergence-flag short-circuit efficacy on large flat terrains.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the early-termination optimization in NG help text.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## fill_depressions_wang_and_liu

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fill_depressions_wang_and_liu.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implement the same priority-queue edge-propagation fill algorithm for depressions.
- Important differences: NG uses a background sentinel for unvisited cells and slightly stricter minimum-elevation enforcement.
- Correctness note: Core filled-surface semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are queue-based and sequential in the core path.
- Tuning opportunities: Minimal, aside from queue implementation tuning.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document sentinel handling for unvisited cells more explicitly.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## fill_missing_data

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/fill_missing_data.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools fill interior NoData gaps by IDW interpolation from valid gap-edge cells with optional exclusion of edge-connected NoData regions.
- Important differences: NG uses explicit alias support (`no_edges`) and integrated raster helpers; legacy uses fixed-radius search structure with thread workers.
- Correctness note: Gap-fill behavior, weighting exponent logic, and edge-exclusion semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform edge detection followed by neighborhood-weighted interpolation with parallel row updates.
- Tuning opportunities: Consider spatial indexing/candidate caching for very large contiguous NoData regions.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity for typical DEM hole-filling tasks.

## fill_pits

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/fill_pits.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both remove single-cell pits by scanning the 8-neighbourhood and raising the center to the spill elevation.
- Important differences: NG inlines all 8 checks with early exit and uses rayon-style row chunking around the core logic.
- Correctness note: Pit-removal semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple local-neighborhood repairs; NG reduces thread orchestration overhead and inlines bounds checks.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity; the tracker marks this red-band only on runtime, not semantics.

## filter_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both implementations support statement-based LiDAR point filtering and optional batch directory processing.
- Important differences: Legacy supports a very broad expression surface (specialized geometry helper functions and optional eigen-feature variables); NG currently exposes a narrower generic expression context.
- Correctness note: Core boolean point-retention workflow is preserved, but advanced statement compatibility is not full parity.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon point filtering and concise context construction without legacy eigen-file parsing overhead.
- Tuning opportunities: Add optional cached derived-feature channels when parity expansion adds advanced variables.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document supported statement variables/functions explicitly and stage additional legacy-compatible expression helpers.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Functional core is good; advanced statement parity remains a roadmap item.

## filter_lidar_by_percentile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar_by_percentile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools select one representative point per grid block according to elevation percentile.
- Important differences: Legacy explicitly excludes withheld/noise points by fixed predicates; NG follows shared point-utility inclusion behavior with the same percentile/block-size mechanics.
- Correctness note: Core percentile-selection semantics are aligned for typical use.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-cell representative selection and streamlines in-memory point-cloud handling.
- Tuning opportunities: Consider memory pooling for large block-cell vectors in very dense clouds.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document exact inclusion/exclusion defaults (noise/withheld) for strict cross-version reproducibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity with licensing-scope divergence.

## filter_lidar_by_reference_surface

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar_by_reference_surface.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools filter or classify points based on relation to a raster reference surface and support query modes (`within`, `<`, `<=`, `>`, `>=`).
- Important differences: Legacy uses direct neighbour fallback around nodata cells; NG uses shared raster sampling helpers with equivalent query semantics.
- Correctness note: Query behavior is aligned for common workflows.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG performs compact parallel match evaluation and direct classify/filter materialization.
- Tuning opportunities: Optional cached raster tile sampling for very large single-tile point clouds.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit regression cases for nodata-neighbour fallback equivalence.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong functional parity with licensing-scope divergence.

## filter_lidar_classes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar_classes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools remove points belonging to specified excluded class values.
- Important differences: NG supports batch mode through shared lidar-path utilities and parses class exclusions from either array or CSV-style text.
- Correctness note: Class-based inclusion/exclusion semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel filtering over point vectors; legacy iterates sequentially.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## filter_lidar_noise

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar_noise.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools remove low/high noise points (class 7 and 18).
- Important differences: NG integrates single-file and batch paths through shared helpers; legacy is single-input API form.
- Correctness note: Noise-class filtering semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel point filtering for single-input mode.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Straightforward parity.

## filter_lidar_scan_angles

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/filter_lidar_scan_angles.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools keep only points whose absolute scan angle is less than or equal to the specified threshold.
- Important differences: NG validates threshold as non-negative finite and supports shared batch plumbing.
- Correctness note: Core threshold filtering behavior is aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG performs parallel point filtering while legacy loops serially.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## filter_raster_features_by_area

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/filter_raster_features_by_area.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools remove integer-labeled raster features whose area in cells is below a threshold and assign either `0` or `NoData` as replacement background.
- Important differences: NG counts labels using rounded integer keys in a hash map while legacy uses min/max-indexed histogram bins.
- Correctness note: For intended integer-labeled rasters, behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations parallelize feature-size counting; NG uses hash-map fold/reduce while legacy uses direct indexed histogram and parallel row writes.
- Tuning opportunities: If very large label ranges are common and integer labels are dense, consider an indexed counting path in NG to reduce hash overhead.

### Design Improvements
- Severity: `Minor`
- Opportunity: Clarify in docs that inputs are expected to be integer-labeled rasters; otherwise rounded key behavior may surprise users.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity for the tool's canonical use case.

## filter_vector_features_by_area

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/filter_vector_features_by_area.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools filter polygon features by area threshold and preserve original attributes for retained features.
- Important differences: Legacy detects likely geographic coordinates and internally converts coordinates to UTM for area calculations; NG computes planar area directly from geometry coordinates.
- Correctness note: Outputs are aligned in projected coordinate systems; divergence is most likely for geographic-coordinate datasets.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG performs per-feature area filtering in parallel and avoids per-feature reprojection work.
- Tuning opportunities: Optional geodesic-area mode could retain speed while improving geographic CRS fidelity.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add an explicit geographic CRS handling option in NG (or warning) to mirror legacy intent and avoid silent area-unit pitfalls.

### Recommended Action
- `FollowupPatch`

### Notes
- Strong projected-CRS parity; geographic-CRS behavior needs explicit policy.

## find_flightline_edge_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/find_flightline_edge_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools extract only points with the edge-of-flightline flag set.
- Important differences: Legacy emits a warning when no points are flagged; NG currently returns an empty cloud without a dedicated warning message.
- Correctness note: Core extracted-point semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG performs parallel filtering over in-memory point records.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional empty-result warning parity for user guidance.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity on algorithmic behavior.

## find_lowest_or_highest_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/find_lowest_or_highest_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools find the minimum and/or maximum non-NoData raster cell and emit point features with value attributes.
- Important differences: NG includes explicit `X` and `Y` attribute fields in addition to `FID` and `Value`; legacy stores `FID` and `Value` only.
- Correctness note: Extreme-cell detection semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses a direct parallel fold/reduce over flattened raster cells.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## find_main_stem

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/find_main_stem.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools trace upstream along the longest branch from each outlet with aligned output semantics.
- Important differences: NG computes upstream distance support via a parallel inflow-count helper and then traces stems sequentially.
- Correctness note: NG now preserves original stream values on selected main-stem cells and honours `zero_background` background assignment.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute farthest-head distances through dependency-driven traversal; NG runs inflow counting in a parallel helper and then traces stems sequentially.
- Tuning opportunities: Primary need is semantic alignment first; performance tuning is secondary.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression tests for main-stem value retention and `zero_background` toggling.

### Recommended Action
- `Accept`

### Notes
- Output-contract parity is restored.

## find_noflow_cells

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/find_noflow_cells.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/find_noflow_cells.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both identify cells that have no lower neighbor in the D8 neighborhood.
- Important differences: NG is implemented as a standalone Tool trait object rather than the older wrapped environment pattern.
- Correctness note: No-flow detection semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are simple neighbor scans with similar computational load.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## find_parallel_flow

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/find_parallel_flow.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools identify stream cells that have nearby stream neighbours with parallel D8 pointer values.
- Important differences: NG accepts broader input aliases (`d8_pointer`, `input`) and has the same optional stream-mask behavior as legacy.
- Correctness note: Parallel-flow detection logic is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy, slightly.
- Evidence from code: Legacy partitions rows across threads with channel-based assembly; NG currently uses a serial nested loop for the core scan.
- Tuning opportunities: Parallelize NG row evaluation to match legacy throughput on large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add parallel row processing in NG for large-grid parity.

### Recommended Action
- `FollowupPatch`

### Notes
- Semantics are good; this is mainly a throughput tuning opportunity.

## find_patch_edge_cells

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/find_patch_edge_cells.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools mark patch-edge cells with the patch ID and set non-edge patch cells to zero.
- Important differences: NG explicitly supports multi-band traversal through generic raster helpers; legacy is effectively single-band focused.
- Correctness note: Core edge-cell classification semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-band cell evaluation with direct vector writes; legacy parallelizes by row with channel merge overhead.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## find_ridges

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/find_ridges.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools identify ridge cells where either N/S or E/W neighbours are lower, with optional iterative line thinning.
- Important differences: NG routes through shared raster plumbing and helper-based thinning loops; legacy implements the same pattern directly.
- Correctness note: Ridge detection and thinning intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use parallel row evaluation for initial ridge detection and iterative thinning passes with similar stencil logic.
- Tuning opportunities: If needed, optimize thinning pass termination checks to reduce full-grid rescans.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## fix_dangling_arcs

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/fix_dangling_arcs.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools repair undershot/overshot dangling line endpoints by snapping/intersecting with nearby segments within a user threshold.
- Important differences: Legacy explicitly uses an R-tree nearest-segment scan; NG implements similar endpoint candidate selection and intersection logic through in-memory segment collections and modern vector layer IO.
- Correctness note: Core network-cleaning behavior aligns, but edge-case equivalence (complex multipart intersections) should be spot-checked.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both split processing into segment preparation and endpoint repair; NG parallelizes feature materialization while repair loop is largely sequential per part.
- Tuning opportunities: Add spatial index acceleration in NG candidate search path for very large line networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted regression fixtures for multipart and near-collinear overshoot scenarios.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics appear aligned with moderate implementation refactor risk.

## flatten_lakes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/flatten_lakes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools flatten each lake polygon interior to its minimum perimeter elevation, excluding interior holes/islands.
- Important differences: NG enforces DEM/vector CRS alignment with auto-reprojection when possible; legacy relies on direct coordinate usage.
- Correctness note: Core lake-flattening semantics are aligned and NG adds safer CRS handling.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform perimeter scan followed by polygon interior rewrite using bounding-box constrained loops.
- Tuning opportunities: Optional spatial tiling and parallel polygon interior updates could help with many large lakes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document CRS auto-alignment behavior explicitly in NG user docs.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity with practical robustness improvement in NG.

## flightline_overlap

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/flightline_overlap.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools rasterize LiDAR space at user resolution and count distinct point-source IDs per cell as a flightline overlap measure.
- Important differences: NG uses direct point-to-cell assignment with per-cell ID sets; legacy uses a kd-tree neighbourhood query per cell.
- Correctness note: Output intent and measurement semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG avoids per-cell kd-tree radius queries and uses direct assignment + set aggregation.
- Tuning opportunities: Optional memory-compacting structures for extremely dense tiles.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence with clearer NG execution path.

## flip_image

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/flip_image.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools flip rasters vertically, horizontally, or both based on a direction argument.
- Important differences: NG names the default mode as `vertical` (with shorthand aliases) while preserving legacy-compatible behavior.
- Correctness note: Reflection behavior is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute output values in parallel over raster cells and then write rows/cells.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Straightforward parity.

## flood_order

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/flood_order.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform a priority-flood style traversal from edges inward and assign sequential flood-order ranks.
- Important differences: NG implements this in vector-backed buffers; legacy uses Array2D/raster objects in-place.
- Correctness note: Flood-order logic and nodata treatment are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use queue + min-heap traversal with similar complexity and memory profile.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## floor

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/floor.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply the floor function to each valid raster cell.
- Important differences: NG routes through the shared unary-math dispatcher.
- Correctness note: Numerical transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary cell transforms; NG has lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## flow_accum_full_workflow

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/flow_accum_full_workflow.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG now uses an integrated legacy-style workflow in one tool kernel: edge-seeded priority-flood conditioning, aspect-aligned non-divergent flow-direction refinement, and topological D8 accumulation generation.
- Important differences: NG retains API-level `clip` as a compatibility flag but does not apply legacy display-stat clipping in output metadata; core raster values follow the integrated workflow logic.
- Correctness note: The previous major semantic gap from chained-tool composition is resolved; remaining differences are primarily output-display/metadata behavior.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie, workload-dependent.
- Evidence from code: Both implementations now follow an integrated in-memory processing shape with priority-flood + pointer refinement + accumulation stages, avoiding earlier NG multi-tool orchestration overhead.
- Tuning opportunities: Benchmark large flat-heavy DEMs and optimize inner flood/refinement loops if any hotspot appears.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures focused on flat/depression-rich scenes to guard aspect-aligned pointer-refinement parity over future refactors.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- The high-priority semantic gap has been closed by moving NG to an integrated legacy-style workflow path.

## flow_length_diff

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/flow_length_diff.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute downslope flowpath length from D8 pointers and map local maximum absolute neighbour differences (4-neighbour) with optional log transform.
- Important differences: NG uses explicit cached direction decoding and iterative path memoization vectors; legacy stores memoized lengths in Array2D.
- Correctness note: Core metric definition and output semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on memoized downstream traversal and a subsequent local-difference pass.
- Tuning opportunities: Consider parallelizing final neighbour-difference pass in NG on very large rasters.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## fuzzy_knn_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/fuzzy_knn_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform fuzzy k-NN supervised classification over multi-raster feature stacks and output class + membership-confidence rasters.
- Important differences: NG uses shared raster-stack alignment and modern vector sampling helpers, while preserving key parameters (`scaling`, `k`, `m`, class field semantics).
- Correctness note: Core fuzzy-membership classification behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations parallelize per-cell prediction and use kd-tree nearest-neighbour queries.
- Tuning opportunities: Optional batched neighbour query optimization in NG for very large scenes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit regression tests for polygon-training edge sampling parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong functional parity with implementation modernization.

## gamma_correction

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/gamma_correction.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/advanced_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools apply gamma exponent correction to grayscale and packed RGB rasters with gamma clamped to [0, 4].
- Important differences: NG routes through shared advanced-filter framework and packed-RGB helpers; legacy applies equivalent intensity-domain handling inline.
- Correctness note: Transform behavior is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute gamma-adjusted values in parallel and write per-row/per-band outputs.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Straightforward parity.

## gaussian_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/gaussian_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform Gaussian contrast stretching with matching histogram-shaping intent.
- Important differences: NG consolidates the logic into a shared remote-sensing non-filter framework.
- Correctness note: Contrast-stretch semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are histogram-heavy but parallelizable; NG has better batching and less wrapper overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Clarify any RGB/IHS output consolidation differences in docs.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## gaussian_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/gaussian_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute Gaussian curvature using third-order bivariate polynomial fitting in projected CRS and 3x3 polynomial fitting in geographic CRS.
- Important differences: NG centralizes derivative and CRS handling in `TerrainCore`-style helpers.
- Correctness note: Curvature formulation and log-transform semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are row-parallel neighborhood curvature computations; NG has cleaner helper reuse and reduced wrapper overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## gaussian_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/gaussian_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/gaussian_filter.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply Gaussian smoothing with matching sigma-to-kernel-size conversion.
- Important differences: NG exposes `treat_as_rgb` and `assume_three_band_rgb` controls and is integrated into modern remote-sensing tooling.
- Correctness note: Core smoothing semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are convolution-heavy and parallelizable; NG uses rayon-backed infrastructure with less manual setup.
- Tuning opportunities: Benchmark RGB and scalar modes independently.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the RGB mode flags clearly for migration users.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## generalize_classified_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/generalize_classified_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools clump contiguous class patches and generalize undersized features using methods `longest`, `largest`, or `nearest`.
- Important differences: NG expresses the same logic with explicit component vectors and method dispatch helpers.
- Correctness note: Method semantics and output intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by connected-component traversal and iterative merge loops; NG parallelizes initialization/write phases.
- Tuning opportunities: Potential union-find style merge acceleration for large numbers of tiny patches.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## generalize_with_similarity

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/generalize_with_similarity.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generalize small class features by merging to neighbouring features with minimum standardized similarity-center distance.
- Important differences: NG uses explicit z-score arrays and component vectors; legacy uses raster/Array2D scans.
- Correctness note: Similarity-driven merge behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both require clumping + iterative neighbourhood merge passes; NG parallelizes several preparatory stages.
- Tuning opportunities: Parallel candidate-neighbour scoring during iterative merge phases.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## generating_function

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/generating_function.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute generating function using 5x5 third-order derivative terms with optional log transform and geographic-distance approximation.
- Important differences: NG integrates generating function into a shared Pro-curvature core while preserving dedicated formula evaluation.
- Correctness note: Metric definition and transform semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses multi-worker row processing with derivative-mask aware execution and coalesced output writes.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with improved implementation structure.

## geomorphons

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/geomorphons.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both classify cells into geomorphon landform classes using horizon-angle comparisons over the 8 principal directions.
- Important differences: NG adds an experimental residuals flag for global-inclination correction and modernizes output plumbing.
- Correctness note: Landform code semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are expensive but parallel per-cell horizon classifiers; NG benefits from rayon scheduling and helper consolidation.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the residuals flag as experimental if it remains user-facing.

### Recommended Action
- `Accept`

### Notes
- Strong parity with an optional NG extension.

## greater_than

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/greater_than.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compare two rasters cell-by-cell and emit 1 where the first input is greater than the second.
- Important differences: NG is implemented through the shared binary-math dispatch framework.
- Correctness note: Comparison semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary raster comparisons; NG has lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## greater_than_or_equal_to

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/greater_than.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compare two rasters cell-by-cell and emit 1 where the first input is greater than or equal to the second.
- Important differences: NG exposes the operator as a dedicated tool instead of a legacy flag on the same implementation path.
- Correctness note: Comparison semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary comparisons; NG benefits from consolidated operator dispatch.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the operator split from the legacy combined comparison tool.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## hack_stream_order

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/hack_stream_order.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both assign stream order based on upstream distance at bifurcations, increasing from outlets toward headwaters.
- Important differences: NG adds a `zero_background` parameter and integrates into shared stream-network-analysis tooling.
- Correctness note: Stream-order semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both depend on traversal of stream graphs with similar asymptotic work.
- Tuning opportunities: Benchmark large dendritic networks and validate any graph traversal caching in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the `zero_background` option clearly for migration users.

### Recommended Action
- `Accept`

### Notes
- Good parity.

## heat_map

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/heat_map.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate kernel-density heat maps from point data using bandwidth, optional weighting field, optional base raster control, and selectable kernel functions.
- Important differences: NG uses shared vector/raster loading and output helpers, while preserving the same KDE behavior and kernel-family semantics.
- Correctness note: Core density-estimation behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use kd-tree neighbour searches per output cell; NG emphasizes simpler sequential cell traversal with optimized helper infrastructure, while legacy uses explicit multi-thread row dispatch.
- Tuning opportunities: Parallelize NG per-row kernel accumulation for very large output rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression checks for kernel-function numerical parity at the bandwidth edge.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence semantic parity.

## height_above_ground

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/height_above_ground.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools convert point elevations to height above nearest ground-classified (class 2) point and fail if no ground points exist.
- Important differences: NG uses shared point-cloud structures and rayon point mapping; legacy uses explicit worker threads with channel aggregation.
- Correctness note: Ground-normalization semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Both build a 2D kd-tree from ground points, but NG applies parallel per-point normalization with lower coordination overhead than legacy mutex/channel progress-driven threading.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with cleaner execution model.

## hexagonal_grid_from_raster_base

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/hexagonal_grid_from_raster_base.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate horizontal/vertical hexagonal polygon grids covering a raster extent with ROW/COLUMN/FID attributes and width-based geometry.
- Important differences: NG factors grid generation into reusable helpers and writes via layer abstractions.
- Correctness note: Grid topology/orientation behavior matches legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Slight edge to NG.
- Evidence from code: Both build similar cell loops, while NG parallelizes feature construction after cell coordinate generation.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## hexagonal_grid_from_vector_base

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/hexagonal_grid_from_vector_base.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate horizontal/vertical hexagonal polygon grids from vector-layer extent using identical geometric construction logic.
- Important differences: NG uses bbox extraction from modern layer abstractions and shared grid helper functions.
- Correctness note: Grid coverage/orientation behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Slight edge to NG.
- Evidence from code: Core geometry loop complexity is similar, with NG parallel feature materialization and streamlined output writing.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## high_pass_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/high_pass_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both emphasize local high-frequency variation by subtracting a neighborhood mean from the center cell.
- Important differences: NG places the filter in consolidated convolution infrastructure.
- Correctness note: High-pass response semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are neighborhood convolution-style filters; NG uses shared rayon-based execution paths.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## high_pass_median_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/high_pass_median_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute a center-minus-median high-pass response over a moving window.
- Important differences: NG is implemented in shared rank-filter infrastructure.
- Correctness note: Median-based high-pass semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are neighborhood rank filters; NG benefits from framework reuse but still performs similar window work.
- Tuning opportunities: Minor optimization of median selection on large windows.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document any sig-digit rounding behavior if retained in NG.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## highest_position

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/highest_position.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools return the zero-based stack index of the maximum raster value per cell and propagate nodata if any stack member is nodata at that cell.
- Important differences: NG implements highest-position through shared GIS overlay infrastructure rather than a dedicated standalone file.
- Correctness note: Indexing contract (0 for first raster, 1 for second, etc.) aligns with legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both scan raster stacks per cell and track best index; both are parallelized row/cell workflows with similar computational shape.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Missing-port status is resolved.

## hillshade

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/hillshade.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/basic_terrain_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute hillshade from slope/aspect and solar illumination parameters with the same core shading model.
- Important differences: NG uses shared terrain helpers and cleaner CRS-aware z-factor handling.
- Correctness note: Hillshade output semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are row-parallel terrain shading operations; NG avoids some legacy wrapper overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## hillslopes

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/hillslopes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools delineate hillslope regions draining to stream links using the same D8 stream-link seeding, downstream watershed labeling, stream-cell zeroing, and clump-grouping workflow.
- Important differences: NG implements the algorithm in shared hydrology module infrastructure and supports both `d8_pntr` and `d8_pointer` argument aliases.
- Correctness note: Core hillslope delineation semantics are aligned with legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations perform equivalent multi-pass raster traversal (stream-link assignment, watershed labeling, and clump pass), with similar computational shape and memory access patterns.
- Tuning opportunities: Minor cache-locality tuning in the clump flood-fill path for very large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression tests that compare hillslope IDs for representative D8 pointer fixtures under both Whitebox and Esri pointer encodings.

### Recommended Action
- `Accept`

### Notes
- Previous divergence label was stale audit drift; NG implementation exists in the hydrology module and follows legacy algorithm structure.

## histogram_equalization

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/histogram_equalization.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both remap raster values via cumulative histogram equalization to spread tones more uniformly.
- Important differences: NG lives in the shared remote-sensing module and uses modern output handling.
- Correctness note: Histogram equalization semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are histogram-based remapping workflows; NG uses consolidated parallel infrastructure.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## histogram_matching

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `LOW_INFERRED_FROM_histogram_matching_two_images|BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/histogram_matching.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools match input-image histogram/CDF to a supplied reference histogram or CDF and remap image values by cumulative probability.
- Important differences: NG normalizes reference histogram pairs through a dedicated helper and uses binary-search interpolation mapping, while retaining the same input/output intent.
- Correctness note: CDF-matching behavior aligns.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes histogram construction and remap passes and precomputes bin-to-reference mapping, reducing repeated per-pixel CDF searches present in legacy.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with likely speedup on larger rasters.

## histogram_matching_two_images

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/histogram_matching_two_images.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both match the histogram of an input image to a reference image using CDF-based remapping.
- Important differences: NG sits inside the shared remote-sensing module.
- Correctness note: Histogram matching semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are histogram construction plus remap pipelines; NG benefits from shared infrastructure and lower wrapper overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## hole_proportion

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/hole_proportion.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute polygon hole area divided by hull area and append a `HOLE_PROP` attribute.
- Important differences: NG supports polygon and multipolygon geometry through modern vector abstractions and writes to explicit output path.
- Correctness note: Area-ratio semantics align with legacy intent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature ratio calculation and then performs deterministic write-back; legacy is serial over records/parts.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## horizon_angle

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/horizon_angle.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute ray-traced horizon angle (maximum upwind slope/angle) along a specified azimuth with optional max-distance control.
- Important differences: NG implementation is hosted in shared sky-visibility tooling with consolidated offset generation and output handling.
- Correctness note: Core horizon-angle contract and parameter intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both run threaded ray-tracing style horizon scans over per-cell offset lists and evaluate max slope/angle along rays.
- Tuning opportunities: Benchmark large max-dist settings where offset list size dominates runtime.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for azimuth edge cases and geographic-coordinate scaling behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Missing-port status is resolved.

## horizon_area

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/horizon_area.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute horizon area by tracing directional horizon points and applying polygon-area accumulation (shoelace-style) across azimuth sweeps.
- Important differences: NG implementation is integrated into shared sky-visibility logic and exposes modernized parameter/manifest plumbing.
- Correctness note: Core horizon-polygon-area contract is present in NG.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both iterate across azimuth slices, ray-trace horizon distances, and accumulate per-cell polygon area contributions with threaded per-row work.
- Tuning opportunities: Profile az_fraction extremes and high max-dist settings for pass-count hotspots.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add golden-raster parity fixtures for observer-height offsets and azimuth fraction settings.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Missing-port status is resolved.

## horizontal_excess_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/horizontal_excess_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute horizontal excess curvature from polynomial terrain derivatives with optional log-transform support.
- Important differences: NG implements the operation through a shared pro-curvature kernel family rather than a dedicated standalone tool implementation.
- Correctness note: Core horizontal-excess formulation (`unsphericity - difference_curvature`) and log-transform behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both run neighbourhood derivative fitting and per-cell curvature evaluation in parallel row workflows with comparable arithmetic intensity.
- Tuning opportunities: Benchmark large geographic DEMs where distance handling branches may shift cache/compute balance.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for projected vs geographic CRS parity under log-transform mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Missing-port status is resolved; this is now a parity-validation item.

## horton_stream_order

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/horton_stream_order.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools assign Horton stream order from D8 pointer + stream rasters, including ESRI-pointer compatibility and optional zero background handling.
- Important differences: NG implementation is integrated into stream-network-analysis shared infrastructure and mirrors legacy logic with compact array-based traversal.
- Correctness note: Core ordering semantics align, including Strahler accumulation followed by Horton main-stem propagation.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG parallelizes stream inflow counting and uses compact downstream arrays; legacy uses dense grid structures and sequential traversal with similar asymptotic work.
- Tuning opportunities: Benchmark very large stream networks to confirm memory-layout effects.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for ESRI-pointer mode and complex confluence networks.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Missing-port status is resolved; this is now a parity-validation/benchmark follow-up item.

## hydrologic_connectivity

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/hydrologic_connectivity.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both implementations target DUL and UDSA outputs using exponent-driven MFD-style accumulation, WI-based connectivity routing, and stream-threshold convergence behavior.
- Important differences: Legacy includes richer geographic-distance handling and warning/report UX, while NG uses a streamlined shared hydrology path with modern output contracts.
- Correctness note: The prior exponent/dispersion semantic gap is closed; remaining differences are secondary implementation and reporting details.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute multi-stage accumulation/routing passes with similar asymptotic work; NG uses compact vectorized structures while legacy uses thread/channel orchestration and additional reporting branches.
- Tuning opportunities: Benchmark large geographic DEMs where CRS-aware distance handling and slope-gradient branches are most sensitive.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures spanning projected vs geographic DEMs and convergence-threshold edge cases.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Prior high-risk exponent compatibility gap has been addressed.

## hypsometric_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/hypsometric_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate hypsometric-analysis HTML reports from one-or-more DEM inputs with optional watershed partitioning.
- Important differences: NG uses a modern HTML renderer and grouped series generation but preserves the same analytical objective and output form.
- Correctness note: Functional behavior aligns for standard use cases.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by raster scanning/binning and report generation; no major complexity shift.
- Tuning opportunities: Optional parallel raster bin accumulation in NG for large multi-DEM runs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit regression snapshots for watershed-partition HTML output.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence.

## hypsometrically_tinted_hillshade

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/hypsometrically_tinted_hillshade.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate hypsometrically tinted hillshade RGB renderings from DEM input with palette, hillshade blending, brightness, atmospheric effects, and 360-illumination options.
- Important differences: NG uses a shared sky-visibility/rendering core and modern palette alias handling; legacy uses thread-channel orchestration and historical palette plumbing.
- Correctness note: Core rendering contract now exists in NG and closely matches legacy parameter intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both run multi-stage hillshade + hypsometric blending pipelines with palette/atmospheric post-processing; implementation structures differ but computational shape is comparable.
- Tuning opportunities: Benchmark full_360_mode and atmospheric-heavy runs to assess pass-level hotspots.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add side-by-side golden-image regression fixtures for palette and atmospheric-effect parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Missing-port status is resolved; remaining work is parity hardening and benchmark confirmation.

## idw_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `LOW_INFERRED_FROM_LIDAR_IDW_INTERPOLATION`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/idw_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools interpolate point values to raster using inverse-distance weighting with radius and minimum-neighbour fallback logic.
- Important differences: Legacy uses `FixedRadiusSearch2D`; NG uses kd-tree radius/nearest queries with equivalent weighting semantics.
- Correctness note: IDW behavior and key parameters (`weight`, `radius`, `min_points`, `use_z`) align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most time in per-cell neighbour queries and weighted reduction; data-structure choice differs but complexity remains similar.
- Tuning opportunities: Parallelize NG per-row interpolation loop for larger rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add side-by-side parity tests for radius-boundary behavior and `min_points` fallback.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High semantic parity confidence.

## ihs_to_rgb

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/ihs_to_rgb.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools convert intensity-hue-saturation rasters back to red/green/blue channel rasters using the same HSI-to-RGB transform family.
- Important differences: NG uses shared non-filter remote-sensing framework and standardized output handling.
- Correctness note: Range assumptions and conversion intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both process full raster grids with row-oriented computation and straightforward channel writes.
- Tuning opportunities: None required.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## image_autocorrelation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/image_autocorrelation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute global Moran's I for one or more rasters with rook/queen-bishop style contiguity options and significance statistics.
- Important differences: NG returns structured report output rather than HTML-file workflow, while preserving core statistical outputs.
- Correctness note: Moran's I and associated variance/z/p calculations are aligned in scope.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both scan rasters with neighbourhood loops and compute higher moments; NG uses rayon reductions over rows/cells.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High parity confidence.

## image_correlation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/image_correlation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute pairwise Pearson correlation among two-or-more co-registered rasters.
- Important differences: NG emits matrix/counts in structured report form instead of legacy HTML output.
- Correctness note: Pairwise correlation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel reductions for means and pairwise deviation terms; legacy parallelizes row loops but with more repeated per-pair scan setup.
- Tuning opportunities: None required.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity with likely compute gains on large inputs.

## image_correlation_neighbourhood_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/image_correlation_neighbourhood_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform moving-window local correlation analysis between two rasters and output correlation + significance rasters for Pearson/Spearman/Kendall options.
- Important differences: NG centralizes statistic dispatch and output writing in shared raster-stats infrastructure.
- Correctness note: Local-correlation intent and outputs align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute per-cell window scans and statistic-specific computations; NG runs fully parallel cell evaluation with shared helper kernels.
- Tuning opportunities: Optional adaptive-window shortcuts for sparse nodata-heavy scenes.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## image_regression

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/image_regression.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools run bivariate linear regression on paired raster values and produce residual rasters with model diagnostics.
- Important differences: NG exposes core regression output in structured report form and omits legacy HTML/scattergram presentation workflow.
- Correctness note: Regression core (slope/intercept/r, ANOVA-style stats, residual generation) aligns.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel reductions for sufficient statistics and residual accumulation; legacy relies on multi-thread channels and sequential merge steps.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider optional scattergram/report renderer plug-in if legacy-style presentation parity is required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Analytical parity is strong despite report-format modernization.

## image_segmentation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/image_segmentation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform seeded region-growing segmentation over standardized multi-band feature stacks with threshold/steps controls and minimum-area cleanup.
- Important differences: NG adds stack auto-reprojection safeguards and modern queue-based merge handling; legacy constrains `min_area` range more tightly.
- Correctness note: Core segmentation and small-region merge behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes z-score preprocessing, seed classification, and multiple merge stages with fewer thread-channel boundaries.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit parity tests for `min_area` edge-case behavior against legacy limits.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong functional parity with improved robustness features.

## image_slider

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/image_slider.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate interactive HTML image sliders from two rasters with per-image palette and reverse-palette controls.
- Important differences: NG keeps a modernized single-file HTML/CSS/JS template and emits structured outputs while applying legacy-style palette controls for non-RGB rasters.
- Correctness note: Missing visualization-control parity gap is resolved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rasterize two full images and write PNG/HTML assets; workload shape is similar.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression snapshots for a few representative palettes and reverse-palette combinations.

### Recommended Action
- `Accept`

### Notes
- Visualization-control parity blocker resolved.

## image_stack_profile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/image_stack_profile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools sample ordered raster stacks at point locations and generate profile/signature outputs with optional HTML reporting.
- Important differences: NG returns profile arrays directly in structured outputs and can optionally emit HTML.
- Correctness note: Sampling and profile intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform per-point per-band sampling; NG uses parallel profile extraction.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## impoundment_size_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/impoundment_size_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG now follows the same three-stage ISI structure as legacy: profile-based crest estimation, priority-flood downstream threshold propagation, and downstream flow-path accumulation constrained by crest/cutoff elevations.
- Important differences: NG still uses a direct in-memory vector-of-elevations propagation strategy and may diverge from legacy in edge-case tie handling and large-raster memory/runtime behavior.
- Correctness note: Core model semantics are now aligned; residual differences are implementation-level rather than algorithm-class mismatch.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Undetermined; likely data-dependent.
- Evidence from code: Both now execute multi-stage crest + priority-flood + accumulation logic; NG implementation currently prioritizes semantic alignment over memory efficiency.
- Tuning opportunities: Reduce transient upslope-elevation memory pressure (streaming/compaction) before any speed claims.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Optimize accumulation data structures for large rasters and confirm edge-case parity via focused regression tests.

### Recommended Action
- `AcceptWithFollowUpOptimization`

### Notes
- Prior high-risk semantic mismatch has been addressed with a legacy-aligned algorithm structure.

## improved_ground_point_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `not_in_legacy_whitebox_workflows`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/improved_ground_point_filter.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: This is an NG-only multi-stage ground-filtering pipeline rather than a direct legacy port.
- Important differences: NG composes percentile filtering, TIN gridding, pit filling, off-terrain-object removal, and reference-surface filtering into one open-tier workflow; there is no legacy equivalent in the audited source tree.
- Correctness note: No direct legacy parity target exists.

### Performance Assessment
- Verdict: `Undetermined`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `NotApplicable`
- Likely faster implementation: Not meaningful to compare.
- Evidence from code: The tool is NG-only and built from subtool composition.
- Tuning opportunities: Benchmark the internal stages separately if this becomes a migration target.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the composed pipeline clearly so users understand it is a new NG workflow, not a legacy port.

### Recommended Action
- `AcceptAsNewCapability`

### Notes
- NG-only capability; no legacy counterpart in the audited source tree.

## increment

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/increment.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both add a scalar value to each valid raster cell, with default increment 1.0.
- Important differences: NG exposes scalar addition through shared unary-math infrastructure and configurable value parsing.
- Correctness note: Increment semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary transforms; NG has lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## individual_tree_detection

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/individual_tree_detection.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools detect local canopy maxima using variable-radius neighbourhood searches based on point height.
- Important differences: NG defaults `only_use_veg=true` (legacy defaults false) and currently requires explicit single input (legacy also supports directory batch mode).
- Correctness note: Core tree-top detection behavior aligns, but defaults/operation mode differ.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Legacy uses kd-tree neighbourhood queries; NG currently performs parallel brute-force neighbour checks over eligible points.
- Tuning opportunities: Replace NG brute-force neighbourhood scan with kd-tree radius query path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Restore batch-mode option and align default `only_use_veg` behavior with legacy.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Functional parity is close; performance path can be improved.

## insert_dams

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/insert_dams.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools insert profile-based dam embankments at user-specified point locations over a DEM using maximum dam length.
- Important differences: NG uses aligned-vector utilities and modern output plumbing, while retaining orientation/profile search logic.
- Correctness note: Dam insertion intent and thresholding behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform localized profile scans around each dam point and update DEM elevations accordingly.
- Tuning opportunities: Optional parallelization across independent dam points in NG for large point sets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## integral_image_transform

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/integral_image_transform.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute summed-area (integral image) transforms and now treat NoData inputs as zeros while writing cumulative values at all cells.
- Important differences: NG remains implemented in shared remote-sensing infrastructure and returns typed outputs rather than legacy-style direct console workflow.
- Correctness note: The core NoData semantic mismatch has been resolved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations perform full raster band scans for cumulative sums; NG adds a parallel remap/write phase for final output.
- Tuning opportunities: Decide parity target first (legacy-style cumulative-at-NoData vs NG-style preserved NoData) before optimization.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted NoData-heavy regression fixtures to lock cumulative behavior for masked inputs.

### Recommended Action
- `Accept`

### Notes
- Main semantic gap has been closed.

## intersect

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/intersect.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform boolean overlay intersection between vector layers and merge attributes from overlapping features.
- Important differences: NG uses topology-native polygon intersection with tolerance handling, while legacy uses KdTree-based matching.
- Correctness note: Intersection semantics are aligned for valid polygon inputs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses topology primitives and parallel alignment work; legacy performs more manual spatial matching.
- Tuning opportunities: Benchmark dense polygon overlays and inspect epsilon sensitivity.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the tolerance parameter mapping from legacy `snap_tolerance` to NG epsilon terminology.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## inverse_pca

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/inverse_pca.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools reconstruct original-image space from PCA component rasters using stored eigenvectors.
- Important differences: Legacy reads eigenvectors from PCA HTML comments and requires >=3 inputs; NG reads JSON report content and accepts >=2 inputs.
- Correctness note: Core inverse transform (weighted component recombination per output image) aligns.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses chunked weighted-sum paths and parallel writes to contiguous output slices; legacy iterates nested row/col loops per output image.
- Tuning opportunities: None urgent.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optionally support legacy HTML-report parsing shim for migration convenience.

### Recommended Action
- `AcceptAsIs`

### Notes
- Modernized I/O contract improves interoperability with NG PCA outputs.

## is_nodata

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/is_nodata.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools emit binary rasters where NoData cells map to 1 and valid cells map to 0.
- Important differences: Legacy explicitly constructs an I16 output, while NG preserves normal NG raster output plumbing and typed output descriptors.
- Correctness note: Cell-level classification semantics match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Legacy uses worker threads plus channel row transfers; NG uses parallel full-buffer fill without per-row channel overhead.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity item.

## isobasins

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/isobasins.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools partition DEMs into approximately equal-area basins using D8 flow routing, target-area thresholding, and downstream labeling.
- Important differences: Legacy optionally emits basin-connection CSV output and interior-pit warnings; NG currently omits these reporting extras.
- Correctness note: Core isobasin delineation workflow is preserved.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Legacy parallelizes flow-direction construction and uses thread-channel staging; NG currently implements the main phases with sequential loops.
- Tuning opportunities: Add rayon parallelization for flow-direction and inflow-count passes in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Reintroduce optional basin-connections export for full feature parity.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Functional parity is solid; remaining gaps are reporting and throughput.

## jenson_snap_pour_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/jenson_snap_pour_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools snap each pour point to the nearest stream raster cell within a search radius and preserve input attributes.
- Important differences: NG adds explicit out-of-extent handling and modern vector output abstraction.
- Correctness note: Stream-cell proximity search and snapping intent align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform per-point local neighbourhood scans over the same window geometry and nearest-distance criterion.
- Tuning opportunities: Optional point-level parallelization in NG for very large pour-point sets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## join_tables

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/join_tables.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools join attributes from a foreign table/layer onto a primary layer using key-field lookup with unmatched rows receiving nulls.
- Important differences: NG adds optional output-path behavior (including in-memory default in current implementation) and generalized vector I/O wrappers.
- Correctness note: One-to-one and many-to-one join semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both build a foreign-key hash map and then iterate primary features for append/write.
- Tuning opportunities: Optional duplicate-key diagnostics in NG (without changing last-write-wins behavior).

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity item.

## k_means_clustering

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/k_means_clustering.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools run iterative k-means clustering over multi-raster feature stacks with class-count, convergence threshold, initialization mode, and minimum-class-size controls.
- Important differences: Legacy requires an HTML output path; NG makes HTML optional and emits structured outputs while preserving optional report generation.
- Correctness note: Core clustering workflow and stopping criteria are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations use parallel row/class assignment patterns and iterative centroid updates.
- Tuning opportunities: Benchmark large stacks to confirm centroid-update and class-reinitialization hot spots in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit parity test fixtures for initialization-mode and minimum-class-size edge cases.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity confidence with modernized output contract.

## k_nearest_mean_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/k_nearest_mean_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform k-nearest mean smoothing in a moving window, selecting neighbourhood values closest in intensity to the center cell before averaging.
- Important differences: NG runs through shared phase-3 filter infrastructure and uses insertion-sorted nearest-distance buffers instead of full-neighbour sort.
- Correctness note: Core k-nearest selection and averaging semantics align, including NoData handling and RGB intensity workflow.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-band row chunks with Rayon and uses bounded in-place nearest-value insertion, avoiding per-cell full-window sort overhead used by legacy.
- Tuning opportunities: Benchmark larger windows for cache behavior and potential SIMD acceleration.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Missing-port status is resolved.

## kappa_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/kappa_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute confusion-matrix and kappa-style agreement metrics on paired categorical rasters, and NG now supports optional legacy-style HTML report output.
- Important differences: NG remains API-first (JSON `report` plus optional `report_html`) while legacy is file/open oriented by default.
- Correctness note: Metric math and reportability are aligned; remaining differences are output default/UX behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel fold/reduce matrix accumulation and avoids HTML rendering/file output in the kernel path.
- Tuning opportunities: Primary need is semantic/report parity restoration, not speed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional HTML wording/layout polish for closer legacy presentation parity.

### Recommended Action
- `Accept`

### Notes
- API/output parity gap is now materially closed.

## knn_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/knn_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform supervised kNN classification from multi-raster predictors and vector training data, with scaling and optional clipping behavior.
- Important differences: Legacy includes model-test split and variable-importance reporting workflow; NG focuses on classification output and does not mirror the same reporting contract.
- Correctness note: Core nearest-neighbour classification behavior is aligned, but model-evaluation/reporting behavior diverges.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on kd-tree neighbour search and row-wise classification loops; NG parallelizes row prediction over the raster grid.
- Tuning opportunities: Add benchmark coverage for high-dimensional predictor stacks and large training sets.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Reintroduce optional model-evaluation outputs (test split metrics/importance) for closer legacy workflow parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Classification kernel parity is good; reporting workflow parity is partial.

## knn_regression

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/knn_regression.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools implement supervised kNN regression over multi-raster predictors with optional scaling and distance weighting.
- Important differences: Legacy includes explicit stochastic train/test split reporting and variable-importance workflow; NG currently focuses on prediction raster generation and omits equivalent model-report outputs.
- Correctness note: Core neighbour-averaging prediction semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on kd-tree nearest-neighbour queries; NG parallelizes raster prediction rows while legacy also spends time in testing/diagnostic phases.
- Tuning opportunities: Benchmark on high-dimensional stacks to validate kd-tree query scaling and clipping of NoData-heavy cells.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-style model diagnostics (test split metrics and variable importance) without changing default prediction path.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Kernel parity is strong; reporting parity remains partial.

## ks_normality_test

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/ks_normality_test.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools perform a one-sample K-S style normality assessment on raster values, including optional sampling.
- Important differences: NG now supports optional HTML report output (`output`/`output_html_file`) while retaining structured metrics output by default.
- Correctness note: Statistical intent and reportability are aligned; remaining differences are output default/UX behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel histogram/stat accumulation and avoids report rendering/file composition in core path.
- Tuning opportunities: Prioritize report-parity decision before further micro-optimizations.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional visual-polish pass for HTML layout/text to more closely mirror legacy presentation.

### Recommended Action
- `Accept`

### Notes
- Report-contract gap is now closed; core behavior remains aligned.

## laplacian_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/laplacian_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply Laplacian edge-detection kernels with the same family of 3x3 and 5x5 variants.
- Important differences: NG consolidates the variants into a shared convolution dispatch enum.
- Correctness note: Kernel semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel convolution filters; NG reduces duplication and wrapper overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## laplacian_of_gaussians_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/laplacian_of_gaussians_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply Laplacian-of-Gaussian edge enhancement with matching sigma-driven smoothing and edge-reflection behavior.
- Important differences: NG places the tool inside shared phase-3 filter infrastructure.
- Correctness note: Core filter semantics align.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker benchmark data indicates NG is slower on the tested fixture despite using rayon-based infrastructure.
- Tuning opportunities: Reduce NG parallel overhead and kernel setup cost for this filter.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Investigate whether rayon scheduling or shared filter setup is the dominant regression before further refactoring.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Semantic parity is good, but this is a performance regression item.

## las_to_ascii

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/las_to_ascii.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools convert LAS/LAZ point clouds to CSV-style ASCII output with core point attributes and optional extended fields.
- Important differences: NG modernizes argument handling/output routing and batch behavior, while preserving the same core conversion intent.
- Correctness note: Field-level conversion behaviour is functionally aligned for common LAS attributes.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are fundamentally streaming point-to-text writers with dominant I/O cost; batch mode is parallelized in both codebases.
- Tuning opportunities: Optional buffered chunk writes and configurable precision formatting for very large files.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence conversion parity.

## las_to_shapefile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/las_to_shapefile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools convert LiDAR points to vector outputs with support for multipoint versus per-point records.
- Important differences: NG routes through generalized vector backends and output-format detection, while legacy is shapefile-centric.
- Correctness note: Core point/multipoint conversion semantics and attribute intent are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute point-wise feature creation loops with similar dominant geometry/attribute serialization costs.
- Tuning opportunities: For huge clouds, expose chunked writer/backpressure controls in NG output pipeline.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document and test feature-count limits/behaviour for extremely large point clouds across output formats.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity with improved format flexibility.

## layer_footprint_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/layer_footprint_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate a rectangular polygon footprint from raster extent bounds and assign a simple `FID` attribute.
- Important differences: NG uses modern vector abstractions/output path requirements; legacy returns in-memory shapefile object directly.
- Correctness note: Extent-to-polygon construction semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform constant-size geometry construction with negligible computational cost.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Simple parity-strong utility tool.

## layer_footprint_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/layer_footprint_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools create a rectangular polygon footprint from input vector extent bounds and assign a simple `FID` attribute.
- Important differences: NG uses generalized vector layer abstractions and explicit output path handling; legacy returns an in-memory shapefile object.
- Correctness note: Bounding-box footprint semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform constant-size extent-to-polygon construction with negligible compute cost.
- Tuning opportunities: None needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Straightforward parity match.

## lee_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/lee_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply the Lee speckle filter using neighborhood averaging with sigma and M parameters.
- Important differences: NG is implemented in the shared phase-3 filter framework.
- Correctness note: Core denoising semantics align.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker benchmarks show NG slower on the tested fixture, likely due to shared infrastructure overhead.
- Tuning opportunities: Improve NG kernel specialization and reduce shared filter setup costs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Profile the shared phase-3 path for repeated allocation or synchronization overhead.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Good semantic parity with a measured runtime regression.

## length_of_upstream_channels

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/length_of_upstream_channels.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute total upstream channel length for each stream cell using D8 routing and stream-cell masks.
- Important differences: NG routes through shared stream-network core utilities and standardized output plumbing.
- Correctness note: Upstream-length accumulation semantics align, including ESRI/Whitebox pointer handling.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use dependency-driven stack traversal; NG parallelizes inflow-count preprocessing and then performs similar topological propagation.
- Tuning opportunities: Minor cache-locality tuning of accumulation buffers could help very large rasters.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Good parity confidence.

## less_than

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/less_than.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compare two rasters cell-by-cell and emit 1 where the first is less than the second.
- Important differences: Legacy optionally folds equality into the same tool via a flag, whereas NG splits the operators into dedicated tools.
- Correctness note: Comparison semantics are aligned for the strict less-than operator.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary comparisons; NG uses the shared raster math backend with lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the operator split from the legacy `equal_to` flag.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## less_than_or_equal_to

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `not_in_legacy_whitebox_workflows`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG provides a dedicated less-than-or-equal-to raster comparison tool; the legacy line was historically exposed via a flag on less-than rather than a separate audited tool surface.
- Important differences: This is a packaging/API split rather than a numerical mismatch.
- Correctness note: The operator semantics themselves are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Dedicated binary math dispatch avoids the legacy combined-path branching.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep documentation explicit that this is a separated NG operator rather than a legacy standalone file port.

### Recommended Action
- `Accept`

### Notes
- Implementation is clean even though the legacy packaging differs.

## lidar_block_maximum

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_block_maximum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools rasterize LiDAR by block and assign per-cell maxima from included points.
- Important differences: NG supports broader interpolation parameters and explicit return/class/elevation filters through unified LiDAR gridding options, extending beyond legacy’s narrower interface.
- Correctness note: Default maximum-elevation workflow aligns; NG adds optional capability rather than conflicting base behavior.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use parallel/batch ingestion patterns and aggregate points into raster cells with similar asymptotic cost.
- Tuning opportunities: Benchmark high-density tiles for hashmap/cell-assignment hot spots in NG generalized path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit legacy-equivalence mode docs for default parameter combinations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics are compatible with expanded NG parameter surface.

## lidar_block_minimum

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_block_minimum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools rasterize LiDAR by block and assign per-cell minima from included points.
- Important differences: As with block maximum, NG adds generalized parameterization (interpolation attributes and filtering controls) while preserving the minimum-surface default workflow.
- Correctness note: Core minimum-block behavior matches legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by point-to-cell assignment and extrema updates, with comparable parallel batch structures.
- Tuning opportunities: Validate memory behaviour for very large batch directories.

### Design Improvements
- Severity: `Minor`
- Opportunity: Clarify default filter parity in docs/examples to avoid accidental behavioural drift from optional NG knobs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity for baseline usage.

## lidar_classify_subset

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_classify_subset.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools classify base-cloud points by matching them to a subset cloud using nearest-neighbour spatial matching and class reassignment rules.
- Important differences: Legacy derives default tolerance from point scale factors; NG exposes an explicit tolerance parameter (default fixed) and modern output routing.
- Correctness note: Class reassignment semantics are aligned; tolerance-default behavior differs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use kd-tree nearest-neighbour lookup; NG parallelizes point-class reassignment over base points.
- Tuning opportunities: Consider adaptive default tolerance tied to LAS scale to match legacy robustness.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-tolerance mode or auto-derive default tolerance from input scale metadata.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Parity is high with a small but important tolerance-default difference.

## lidar_colourize

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_colourize.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools assign per-point RGB values from an overlapping raster image by mapping each LiDAR point to the source image pixel.
- Important differences: Legacy includes explicit LAS point-format coercion/write-path handling while NG writes through shared point-cloud I/O abstractions.
- Correctness note: Core colour-assignment semantics are aligned, including fallback to black for out-of-coverage/nodata lookups.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Legacy parallelizes point sampling with multi-threaded work partitioning; NG currently uses a single sequential pass over points.
- Tuning opportunities: Add parallel point-colour sampling in NG for large clouds.

### Design Improvements
- Severity: `Minor`
- Opportunity: Preserve NG's cleaner output abstraction while adding optional chunked/parallel raster sampling.

### Recommended Action
- `PerformanceTuning`

### Notes
- Semantics are close enough for practical parity; main gap is throughput risk on very large files.

## lidar_construct_vector_tin

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_construct_vector_tin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools create LiDAR-derived Delaunay TIN polygons with return/class/elevation filtering controls.
- Important differences: Legacy outputs `CENTROID_Z` and per-triangle `HILLSHADE` attributes and applies max-edge filtering using 3D edge length; NG outputs `AVG_Z` only and applies max-edge filtering in XY (2D) space.
- Correctness note: Triangulation intent matches, but output schema and edge-rejection behavior are not fully equivalent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by triangulation and per-triangle vector emission; batching is parallelized in both implementations.
- Tuning opportunities: Reassess triangle-attribute generation cost if NG adopts legacy-compatible hillshade attributes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-compatible output fields (`CENTROID_Z`, `HILLSHADE`) and optional 3D edge-length filtering mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Behavior is acceptable for many workflows but attribute-level parity is incomplete.

## lidar_contour

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_contour.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools contour LiDAR via triangulation and support interval/base contour, returns/class/elevation filtering, and triangle edge-length controls.
- Important differences: Legacy supports actual contour-line smoothing and tile-overlap neighbour ingestion in batch mode; NG accepts `smooth` only for call-shape parity and currently omits smoothing and tile-overlap behavior.
- Correctness note: Core contour generation is present, but two meaningful legacy behaviors are missing in NG.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG emits per-triangle contour segments directly without legacy smoothing/line-chaining phases and without neighbour-tile overlap expansion.
- Tuning opportunities: If smoothing is reintroduced, prefer optional post-processing that can be toggled for performance-sensitive runs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Implement functional `smooth` support and optional tile-overlap neighbour sampling to restore behavioral parity for production contouring.

### Recommended Action
- `SemanticRedesignAndPerformanceTuning`

### Notes
- Current NG implementation is useful but omits two legacy capabilities that affect map quality and tile-edge continuity.

## lidar_digital_surface_model

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_digital_surface_model.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools build DSM rasters from LiDAR using top-surface candidate filtering followed by TIN rasterization.
- Important differences: Legacy uses a slope-aware dominance test and batch neighbour-tile overlap loading; NG uses a simpler local-maximum dominance filter and batch processing without cross-tile overlap ingestion.
- Correctness note: DSM concept is preserved, but canopy/roof edge behavior and tile seam handling may differ.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both run neighbourhood queries plus triangulation/raster fill; legacy parallelizes batch tile handling, while NG uses streamlined in-tile logic.
- Tuning opportunities: Add optional neighbour-tile overlap mode in NG for better seam consistency in tiled production runs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add legacy-compatible overlap support (and optionally a slope-threshold mode) to improve equivalence on dense urban/forest mosaics.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-level workflow parity is good, but edge-case surface behavior needs targeted validation.

## lidar_eigenvalue_features

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_eigenvalue_features.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute neighbourhood PCA/eigenvalue feature stacks and write `.eigen` binary output with JSON sidecar metadata.
- Important differences: Legacy explicitly excludes withheld/noise points from tree construction and processing path; NG currently indexes all points and relies mainly on neighbourhood-size guards and downstream math safeguards.
- Correctness note: Feature definitions align, but inclusion-filter differences may affect outputs in noisy clouds.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use kd-tree neighbour searches and parallel per-point feature extraction; runtime should be dominated by neighbourhood query cost and eigendecomposition throughput.
- Tuning opportunities: Consider optional prefiltering of withheld/noise points before index construction to reduce neighbour-query overhead.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit legacy-compatible inclusion filtering toggle and document sidecar schema compatibility expectations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity overall; small differences concentrate in point inclusion policy.

## lidar_elevation_slice

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_elevation_slice.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools support elevation-slice filtering and optional in/out class reassignment using lower/upper z bounds.
- Important differences: NG adds optional batch mode and alias parameter names (`min_elev`/`max_elev`) while preserving legacy default class values and classify/filter behavior.
- Correctness note: Core inside/outside slice logic matches legacy intent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel point transforms for single-input mode and parallel file fan-out in batch mode; legacy is single-threaded for this operation.
- Tuning opportunities: Validate memory pressure for very large single clouds in NG classify mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document batch-mode output naming conventions relative to legacy single-file behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence parity for standard workflows.

## lidar_ground_point_filter

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_ground_point_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools now perform slope-based ground/off-terrain discrimination with optional classify/filter outputs and active terrain-normalization (`slope_norm`) support.
- Important differences: Remaining differences are implementation-level (parallel morphology/slope execution, neighbour tie handling, and height-above-ground edge cases for sparse neighborhoods), so exact point-level labels may still differ in fringe cases.
- Correctness note: The prior high-risk `slope_norm` semantic gap is now closed; remaining differences are moderate and localized.

### Performance Assessment
- Verdict: `MixedUnknown`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `ShapeSimilar`
- Likely faster implementation: unknown without measurement.
- Evidence from code: Both implementations now execute multi-pass morphology and slope filtering; NG uses rayon-style parallel stages while legacy uses thread/channel orchestration.
- Tuning opportunities: Benchmark steep terrain and sparse-neighbourhood tiles to validate classification/runtime sensitivity.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Tighten tie-handling parity for sparse neighborhoods and add focused rugged-terrain regression fixtures.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Redesign-class divergence is resolved after restoring `slope_norm` and late-return-oriented morphology flow.

## lidar_hex_bin

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_hex_bin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools hex-bin LiDAR points and emit per-cell count and elevation/intensity extrema.
- Important differences: Legacy writes full grid coverage with `ROW`/`COLUMN` fields (including empty cells), while NG emits only populated hexes and omits row/column attributes.
- Correctness note: Statistical summaries per populated hex are aligned; complete grid topology parity is not.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes point-to-hex assignments and skips empty-cell feature writes; legacy performs sequential bin updates and writes all generated hexes.
- Tuning opportunities: Add optional full-grid mode in NG for workflows that require legacy row/column lattice completeness.

### Design Improvements
- Severity: `Minor`
- Opportunity: Provide compatibility flag for legacy full-grid output and row/column attributes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity is strong for density-style mapping, with topology-output differences.

## lidar_hillshade

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_hillshade.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools now produce LiDAR output with per-point grayscale hillshade encoded in RGB fields from local plane-normal estimation.
- Important differences: Remaining differences are implementation-level (normal-fitting helper behavior in low-neighbourhood cases, azimuth/aspect numerical tie handling, and deterministic floating-point differences), so exact grayscale values may vary slightly.
- Correctness note: The prior output-type mismatch (raster vs LiDAR) is closed; workflow semantics are now aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `ShapeSimilar`
- Likely faster implementation: Near tie.
- Evidence from code: Both now spend most runtime in neighbourhood searches and per-point local normal/hillshade evaluation.
- Tuning opportunities: Benchmark dense clouds with large search radii and profile kd-tree neighbourhood query costs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add small regression fixtures for azimuth edge angles and low-neighbourhood behavior to tighten numeric parity.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Largest mismatch is resolved after restoring LiDAR output contract and active local-normal hillshade behavior.

## lidar_histogram

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_histogram.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute LiDAR attribute histograms and emit HTML reports with parameter/clip controls.
- Important differences: Legacy renders SVG-style histogram graphics with legacy tail-clipping behavior (including special handling for time), while NG currently emits tabular HTML bins and simplified clipping/binning policy.
- Correctness note: Distribution summarization is aligned, but reporting UX and clipping details differ.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes value extraction and uses streamlined report generation; legacy performs multiple sequential passes and richer rendering scaffolding.
- Tuning opportunities: If legacy-style chart rendering is required, keep a lightweight mode to preserve current NG speed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional charted SVG output mode to improve legacy report compatibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Numerically useful parity is present; output presentation parity is incomplete.

## lidar_idw_interpolation

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_idw_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform inverse-distance interpolation from LiDAR points with the same filtering and batch-processing behavior.
- Important differences: NG adds a `min_points` fallback path and stronger alias normalization.
- Correctness note: Interpolation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes tile-level batch operations with Rayon and has cleaner dataflow than legacy thread orchestration.
- Tuning opportunities: Minor: refine spatial search batching for dense point clouds.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the `min_points` fallback and alias mapping in the NG help text.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## lidar_info

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_info.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate LiDAR metadata summaries, and NG now supports functional optional VLR/geokey reporting paths in addition to core bbox/statistics output.
- Important differences: Legacy still includes additional convex-hull/density diagnostics and browser-oriented HTML detail that NG does not fully replicate.
- Correctness note: Core summary metrics and LAS metadata diagnostic toggles (`show_vlrs`, `show_geokeys`) now align for practical inspection workflows.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes summary reductions and avoids heavy HTML table serialization of legacy VLR/geokey sections.
- Tuning opportunities: If legacy-depth metadata reporting is added, keep lightweight mode as default for fast inspection.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional convex-hull density metrics and richer HTML formatting for full report-depth parity.

### Recommended Action
- `Accept`

### Notes
- Remaining differences are report-depth enhancements rather than core metadata correctness issues.

## lidar_join

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_join.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools merge multiple LiDAR inputs into one output cloud.
- Important differences: Legacy enforces same LAS point format across all inputs and errors if mismatched; NG merges through normalized point records and does not enforce this strict legacy precondition.
- Correctness note: Merge intent matches, but mixed-format guard behavior differs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by input read and append work; NG uses parallel fold/reduce for aggregate point collection.
- Tuning opportunities: Validate peak memory use for very large multi-tile merges in NG's collect/reduce path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional strict-format validation mode to mirror legacy safety checks.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity is good for common same-schema merge workflows.

## lidar_kappa

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_kappa.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute nearest-point classification agreement, Kappa statistics, HTML reporting, and a spatial class-agreement raster.
- Important differences: Legacy supports optional class-specific accuracy raster output mode (`output_class_accuracy`), while NG currently retains the flag for compatibility but always emits overall agreement raster.
- Correctness note: Core contingency/Kappa math is aligned; output-mode flexibility differs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on kd-tree nearest-neighbour matching over classification points and similar matrix accumulation work.
- Tuning opportunities: Benchmark memory locality of NG parallel comparison collection on very dense clouds.

### Design Improvements
- Severity: `Minor`
- Opportunity: Implement functional `output_class_accuracy` mode parity and document default raster semantics clearly.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Statistical parity is strong; behavior divergence is mostly output-mode control.

## lidar_nearest_neighbour_gridding

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_nearest_neighbour_gridding.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform nearest-neighbour LiDAR raster gridding with parameter/returns/class/elevation filtering and radius-constrained NoData behavior.
- Important differences: Legacy batch mode explicitly ingests overlapping neighbour tiles to reduce edge effects; NG batch mode uses neighbour-aware argument plumbing but implementation path is refactored and not strictly equivalent to legacy overlap semantics in all cases.
- Correctness note: Core NN gridding behavior is aligned, with potential seam-handling differences in tiled workflows.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both center on kd-tree/fixed-radius nearest lookup and raster cell assignment with parallel batch fan-out.
- Tuning opportunities: Re-benchmark multi-tile mosaics where overlap handling dominates I/O and index build cost.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit docs/tests that confirm NG batch neighbour inclusion matches legacy edge-effect expectations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High parity for single-tile runs; batch seam behavior deserves focused verification.

## lidar_point_density

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_point_density.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute moving-radius point-density rasters with returns/class/elevation filters and batch processing support.
- Important differences: Legacy uses explicit overlap-expanded tile ingestion in batch mode; NG batch implementation is streamlined but preserves the same density formulation and filter controls.
- Correctness note: Density computation intent is aligned and differences are largely in batch orchestration details.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations are dominated by neighbourhood query counts per output cell and similar batch parallelization patterns.
- Tuning opportunities: Validate very large-radius runs for memory/query scaling in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document any remaining batch overlap nuances relative to legacy tile-edge handling.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good practical parity for core density workflows.

## lidar_point_return_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_point_return_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools analyze return-sequence QC issues (missing returns, duplicate returns, and `r > n`) and can optionally write a classified QC LiDAR output.
- Important differences: Legacy emits console-rich narrative guidance; NG emits a concise structured report with equivalent classification coding (13/14/15/1).
- Correctness note: Core defect-detection logic and output class semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel pre-processing and grouped aggregation with streamlined report generation; legacy performs heavier serial sorting/scan flows with verbose output.
- Tuning opportunities: Validate large-file peak memory behavior for grouped key maps in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional richer explanatory report mode to mirror legacy guidance text when needed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence semantic parity for QC workflows.

## lidar_point_stats

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_point_stats.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate multi-raster LiDAR summary products (point count, pulses, avg points/pulse, z range, intensity range, predominant class).
- Important differences: Legacy errors when no output flags are set; NG defaults to generating all outputs in that case and returns directory-oriented output metadata.
- Correctness note: Raster statistic definitions are aligned, with small default-behavior differences.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes point assignment and consolidates stat output writing in a shared path; legacy includes more explicit per-output branching and sequential tile write handling.
- Tuning opportunities: Stress-test memory footprint when all stat layers are requested on very fine grids.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider optional strict legacy mode that requires at least one explicit stat flag.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity is strong across core outputs.

## lidar_radial_basis_function_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_radial_basis_function_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform LiDAR surface interpolation using local radial-basis influence with configurable basis/weighting and neighbourhood controls.
- Important differences: Legacy computes local RBF models more explicitly with k-nearest neighborhood solves; NG uses a hybrid similarity-weighted local predictor with optional polynomial correction and neighbour-tile wiring in batch mode.
- Correctness note: Interpolation intent is aligned, but exact numeric surfaces are expected to differ due to model formulation differences.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by repeated neighbour searches and per-cell interpolation math with parallel row/tile execution.
- Tuning opportunities: Benchmark large search neighborhoods and compare numerical stability/throughput across basis functions.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document expected numeric differences from legacy RBF solve strategy and add tolerance-based parity tests.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Functionally comparable, but not bitwise-equivalent surfaces.

## lidar_ransac_planes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_ransac_planes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools identify planar LiDAR points via neighbourhood RANSAC plane fitting and support filter-vs-classify output behavior.
- Important differences: Both retain major parameter controls, but stochastic sampling details and neighbourhood assembly strategy differ, so exact planar membership can vary at boundaries.
- Correctness note: Core algorithmic intent and class/filter semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both tools are heavily dominated by repeated neighbourhood searches and iterative model fits with multi-threaded execution.
- Tuning opportunities: Compare convergence/runtime sensitivity to `num_iterations` and `num_samples` under dense canopy scenes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add reproducibility controls (seed handling docs/options) for tighter cross-version comparison.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity for intended planar segmentation behavior.

## lidar_remove_outliers

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_remove_outliers.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute local elevation residuals (mean/median option) and either filter or classify outliers using thresholded residual magnitude.
- Important differences: Legacy and NG both map negative/positive outliers to classes 7/18 in classify mode, with minor differences in neighbourhood bookkeeping and default suffix naming.
- Correctness note: Outlier decision semantics are strongly aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel residual computation and streamlined classify/filter emit paths; legacy uses thread-channel fanout and explicit record reconstruction.
- Tuning opportunities: Validate median-mode scaling for high-density neighbourhoods.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit note in docs about residual baseline excluding the query point for both mean and median modes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High practical parity for outlier filtering/classification workflows.

## lidar_rooftop_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_rooftop_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools detect rooftop planar facets within building footprints and output roof polygons with slope/aspect/hillshade-style attributes.
- Important differences: Legacy uses explicit local RANSAC plane fitting; NG uses local PCA planarity features plus region-growing.
- Correctness note: End-to-end rooftop-facet extraction intent and output attribute family are aligned, though boundary membership can differ from the model-choice change.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both versions are dominated by neighbourhood search + local surface analysis and parallel per-point processing; NG avoids repeated random-sampling loops but adds polygon preparation and hull generation paths.
- Tuning opportunities: Benchmark dense downtown scenes where building-footprint overlap checks dominate.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional compatibility mode exposing legacy RANSAC controls (`num_iterations`, `num_samples`) with active effect rather than metadata-only compatibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity is strong for rooftop delineation workflows.

## lidar_segmentation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_segmentation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG now uses legacy-like local RANSAC planar modelling and region growing constrained by planar/non-planar state, normal-angle threshold, optional class boundaries, and `max_z_diff`.
- Important differences: Remaining differences are implementation-level (sampling seed strategy, deterministic coloring details, and internal tie-breaking during model assignment), so exact segment IDs/colors may still differ from legacy even when semantic grouping is similar.
- Correctness note: Topology divergence risk is reduced substantially relative to prior NG behavior, especially for planar roof/facet transitions.

### Performance Assessment
- Verdict: `MixedUnknown`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `ShapeSimilar`
- Likely faster implementation: unknown without measurement.
- Evidence from code: Both implementations now perform neighbourhood model fitting and constrained region growth; runtime should depend mainly on neighbourhood density, iteration count, and sampling parameters.
- Tuning opportunities: Add regression/perf cases for dense rooftops and heterogeneous vegetation to validate quality and runtime sensitivity.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Tighten residual/tie-break parity and document any intentional non-determinism differences in segment colouring/IDs.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Semantic gap is no longer redesign-class after the RANSAC + normal-threshold parity patch.

## lidar_segmentation_based_filter

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_segmentation_based_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools now produce ground filtering/classification outputs from top-hat residual preprocessing, 3D residual-space neighbourhoods, and normal-angle constrained region growth.
- Important differences: NG still uses a streamlined single-path implementation (shared kd-tree and PCA normal estimation) and may differ from legacy in seed initialization/tie handling and neighbourhood composition edge cases.
- Correctness note: The prior high-risk compatibility gap (`norm_diff_threshold` inactive and no top-hat residual stage) is addressed; remaining differences are implementation-detail rather than algorithm-family mismatch.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG now performs top-hat residual + normal-angle gating stages but remains a streamlined in-memory pipeline with fewer orchestration passes than legacy.
- Tuning opportunities: Validate slope/rough-terrain false-positive rates and compare against legacy normals-driven behavior.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add targeted rugged-terrain regression fixtures and tune seed/normal-neighbour defaults for closer edge-case parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Prior compatibility-only parameter gap has been resolved with active normal-angle gating.

## lidar_shift

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_shift.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools apply user-specified X/Y/Z offsets to all LiDAR points and preserve the remaining point-record attributes.
- Important differences: Legacy shifts integer-scaled LAS record coordinates directly; NG shifts normalized point coordinates in `PointRecord` representation before write-back.
- Correctness note: Physical coordinate outcomes are equivalent for standard workflows.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG performs a straightforward parallel map over points; legacy uses serial per-record match/rebuild logic across LAS point variants.
- Tuning opportunities: Confirm memory throughput on very large clouds where clone+map dominates.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit no-op warning when all shifts are zero (legacy docs imply at least one non-zero).

### Recommended Action
- `Accept`

### Notes
- Strong parity and low risk.

## lidar_sibson_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_sibson_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools perform Sibson/natural-neighbour LiDAR interpolation with comparable interpolation-parameter and return/class filters, including batch tile handling.
- Important differences: Legacy includes older error/panic handling and parameter narrative; NG is integrated with modern memory-output and batch-neighbour orchestration.
- Correctness note: Core interpolation intent and filtering controls are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by point loading/filtering, triangulation/natural-neighbour computations, and row/tile parallel work; NG adds modern orchestration but similar computational hotspots.
- Tuning opportunities: Benchmark large tile mosaics to compare neighbour-tile overhead and triangulation scaling.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit documentation note confirming expected output compatibility tolerances vs legacy Pro outputs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Parity appears high despite licensing-tier shift.

## lidar_thin

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_thin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools thin LiDAR points by assigning one representative point per grid cell.
- Important differences: NG adds a `nearest` selection mode and uses sparse map-based cell bookkeeping; legacy uses array-backed grid indexing with legacy method set.
- Correctness note: Core thinning semantics are aligned and produce one retained representative per occupied cell.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel output filtering and sparse map cell assignment; legacy follows mostly serial array-grid traversal.
- Tuning opportunities: Optimize nearest-mode distance calculations with tighter per-cell arithmetic paths.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document nearest-mode behavior and equivalence expectations versus legacy thinning methods.

### Recommended Action
- `Accept`

### Notes
- High practical parity with a useful NG extension.

## lidar_thin_high_density

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_thin_high_density.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools identify locally over-dense bins, compute skip factors, and thin points using cyclic retention logic.
- Important differences: NG modernizes bin aggregation and filtering paths with parallel iterators; algorithm intent remains the same.
- Correctness note: High-density thinning decisions are equivalent under matching parameters.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG performs parallel fold/reduce style aggregation and parallel filtered output construction; legacy uses older multi-step flow.
- Tuning opportunities: Reduce histogram allocation churn in very large tiles.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add benchmark notes for density-threshold sensitivity across canopy-heavy datasets.

### Recommended Action
- `Accept`

### Notes
- Strong semantic and behavioral parity.

## lidar_tile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_tile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools split LiDAR input into regular spatial tiles and emit per-tile outputs subject to point-count constraints.
- Important differences: NG includes improved orchestration for output handling and batch paths; core tiling math is aligned.
- Correctness note: Tile indexing and assignment semantics are equivalent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations share the same partitioning structure; NG can parallelize portions of output production while legacy is mostly serial.
- Tuning opportunities: Improve write-path locality by grouping points by tile before output flush.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional deterministic tile write ordering controls for reproducible pipelines.

### Recommended Action
- `Accept`

### Notes
- Clear parity for expected tiling workflows.

## lidar_tile_footprint

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_tile_footprint.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools derive tile footprint polygons (bounds/optional hull) with output attributes for LiDAR tile inventorying.
- Important differences: NG uses modern vector abstractions and parallel bounds pipelines; legacy uses older thread/mutex patterns.
- Correctness note: Footprint geometry and attribute intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel folds for bounds and streamlined vector emit; legacy performs equivalent logic with heavier threading primitives.
- Tuning opportunities: Add optional fast-path when convex hull is disabled and only AABB footprints are needed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose explicit hull simplification tolerance for very dense footprints.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity and cleaner NG execution model.

## lidar_tin_gridding

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_tin_gridding.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools interpolate LiDAR-derived rasters from TIN triangulation with similar point filters and interpolation-parameter handling.
- Important differences: NG adds backend flexibility and modernization around batch/memory orchestration.
- Correctness note: Interpolation semantics are equivalent under matched parameterization.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG keeps the same triangulation-to-raster workflow but adds improved parallel handling and optional preprocessing options.
- Tuning opportunities: Auto-select triangulation backend based on point-count heuristics.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit compatibility notes for backend-dependent numeric variation tolerance.

### Recommended Action
- `Accept`

### Notes
- Strong parity with useful implementation flexibility.

## lidar_tophat_transform

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/lidar_tophat_transform.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools perform morphological opening on neighborhood surfaces and output top-hat residuals as a height-above-local-ground proxy.
- Important differences: NG uses modern KD-tree + rayon composition while legacy uses fixed-radius structures with channel-based threading.
- Correctness note: Erosion-dilation residual semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG consolidates neighborhood computations with parallel iterator workflows; legacy splits into staged threaded passes with higher coordination overhead.
- Tuning opportunities: Benchmark sparse-point regimes where index query overhead dominates.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider separate control of erosion and dilation support radii for advanced morphology use cases.

### Recommended Action
- `Accept`

### Notes
- Robust parity for top-hat transform behavior.

## line_detection_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/line_detection_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply directional line-detection kernels with optional absolute-value and tail-clipping controls.
- Important differences: NG uses the consolidated convolution extra-filter infrastructure.
- Correctness note: Line-response semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel convolution filters; NG uses streamlined dispatch and lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## line_intersections

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/line_intersections.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute line-segment intersections between line layers and emit intersection point features.
- Important differences: NG includes stronger duplicate-intersection suppression logic; legacy is more permissive under numeric precision noise.
- Correctness note: Intersection detection intent and outputs are equivalent in normal use.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel line-pair evaluation with streamlined dedup handling; legacy uses manual synchronization-heavy threading.
- Tuning opportunities: Add optional spatial pre-index filtering to reduce candidate segment pair checks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose dedup tolerance knob for edge-case cartographic precision control.

### Recommended Action
- `Accept`

### Notes
- Mature parity with improved robustness to near-duplicate outputs.

## line_thinning

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/line_thinning.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools execute the same iterative binary raster skeletonization/thinning pattern logic.
- Important differences: NG keeps a cleaner serial kernel implementation; legacy includes legacy threading scaffolding that does not materially change algorithm behavior.
- Correctness note: Pattern tables and thinning behavior are effectively equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG avoids thread/channel overhead for an iteration-dependent algorithm; legacy incurs extra synchronization without clear throughput gain.
- Tuning opportunities: Introduce lookup-table micro-optimizations in inner neighbourhood pattern checks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document serial-by-design rationale for predictable skeletonization behavior.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity for binary line skeleton outputs.

## linearity_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/linearity_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute polygon linearity using the legacy regression-based r-squared metric derived from exterior-ring vertex coordinates.
- Important differences: NG enforces polygon input validation and applies the same metric in a parallel per-feature execution path.
- Correctness note: The prior metric-definition drift has been removed.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both now execute the same regression accumulation kernel; NG parallelizes feature processing while legacy iterates serially.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures covering multipart polygons and degenerate single-vertex/collinear rings.

### Recommended Action
- `Accept`

### Notes
- Semantics now align with legacy linearity definition.

## lines_to_polygons

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/lines_to_polygons.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools convert line features into polygon features by ring closing and exterior/hole orientation handling.
- Important differences: NG improves validation/error handling and parallelizes feature conversion; core geometry conversion behavior is aligned.
- Correctness note: Ring closure and orientation semantics match legacy behavior for standard valid inputs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG performs parallel feature conversion with integrated orientation handling; legacy follows serial multi-step conversion.
- Tuning opportunities: Pre-size per-feature ring buffers based on part metadata to reduce reallocations.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional topology validation report for self-intersections and ring anomalies.

### Recommended Action
- `Accept`

### Notes
- Strong parity with better operational resilience in NG.

## list_unique_values

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/list_unique_values.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute per-category frequency counts for a vector attribute field.
- Important differences: NG returns a structured JSON report map while legacy returns tuple-style records; null and category ordering behavior differ slightly.
- Correctness note: Category frequency semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses parallel per-feature frequency aggregation and reduction; legacy iterates records serially.
- Tuning opportunities: Preserve optional deterministic category ordering for large category dictionaries without extra sorting cost.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add output format toggle to mirror legacy tuple/CSV-style reporting when strict parity consumers depend on it.

### Recommended Action
- `Accept`

### Notes
- Strong functional parity for vector unique-value listing.

## list_unique_values_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/list_unique_values_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG now supports strict legacy-compatible category-frequency output and retains capped unique listing as an explicit alternate mode.
- Important differences: NG adds `strict_parity` (default true) and returns a CSV table output (`table_csv`) plus JSON report metadata; capped unique listing remains available when `strict_parity=false`.
- Correctness note: Full category-frequency parity behavior is restored by default.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG strict-parity mode now performs full-grid category-frequency accumulation using Rayon fold/reduce map aggregation, matching or exceeding legacy threaded aggregation structure.
- Tuning opportunities: Minor: benchmark high-cardinality categorical rasters for hash-map merge overhead and consider specialized integer-key accumulators.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add parallel map/reduce frequency accumulation to recover legacy-style threading in strict mode.

### Recommended Action
- `Accept`

### Notes
- Default behavior now matches legacy expectations for full category-frequency listing.

## ln

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/ln.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the natural logarithm per valid raster cell and preserve nodata.
- Important differences: NG uses shared unary-math dispatcher infrastructure.
- Correctness note: Numerical transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary cell transforms; NG has lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## local_hypsometric_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/local_hypsometric_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute minimum local hypsometric integral across nonlinearly sampled neighborhood scales and output magnitude plus key scale.
- Important differences: NG uses integral-image helpers plus parallel row kernels; legacy uses deque-based sliding-window accumulation with thread channels.
- Correctness note: HI definition and min-over-scales behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG mixes integral-image sums and parallel per-row evaluation; legacy repeatedly recomputes moving-window extrema/means with heavier per-scale row state management.
- Tuning opportunities: Replace remaining per-cell min/max neighborhood scans with monotonic-window extrema structures for larger scales.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document tolerance expectations for tiny numeric differences caused by different accumulation strategies.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence semantic parity with improved execution structure.

## log10

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/log10.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute base-10 logarithm per valid raster cell and preserve nodata.
- Important differences: NG uses shared unary-math dispatcher infrastructure.
- Correctness note: Numerical transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary cell transforms; NG has lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## log2

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/log2.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute base-2 logarithm per valid raster cell and preserve nodata.
- Important differences: NG uses shared unary-math dispatcher infrastructure.
- Correctness note: Numerical transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are simple unary cell transforms; NG has lower dispatch overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## logistic_regression

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/logistic_regression.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools train logistic models from raster predictors and vector class labels and produce classified raster outputs.
- Important differences: Legacy includes stochastic train/test split diagnostics and richer model-report workflow; NG focuses on deterministic fit-and-predict output with streamlined parameters.
- Correctness note: Core supervised classification intent is preserved, but analytics/reporting semantics differ.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most time in training-sample extraction and model fitting; NG includes aligned-stack helpers and direct prediction raster write path.
- Tuning opportunities: Add optional sampled holdout metrics mode to compare accuracy/runtime against legacy split workflow.

### Design Improvements
- Severity: `Major`
- Opportunity: Restore optional test/validation reporting mode for legacy-equivalent model diagnostics while retaining current streamlined path.

### Recommended Action
- `NeedsAlgorithmDecision`

### Notes
- Core classification parity is good; evaluation/reporting parity is not complete.

## long_profile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/long_profile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate HTML longitudinal profiles derived from D8 traversal of stream heads and DEM elevations.
- Important differences: NG routes through stream-network fallback helpers and shared profile rendering utilities; legacy uses a dedicated in-tool HTML writer and traversal flow.
- Correctness note: Profile generation intent and required inputs remain aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by stream traversal and profile sampling; NG uses shared helper pipelines while legacy manually threads head detection.
- Tuning opportunities: Parallelize head-profile sampling in NG for very dense stream networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output style controls to mirror legacy chart/label presentation.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Functional parity appears strong despite refactored architecture.

## long_profile_from_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/long_profile_from_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools create HTML long profiles from vector start points following D8 flow paths over a DEM.
- Important differences: NG delegates to shared sampling/rendering helpers and modern output path handling; legacy performs dedicated point-loop traversal and HTML construction.
- Correctness note: Core profile extraction from input points is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both primarily cost flowpath traversal and chart output; NG has lighter shared helper composition, legacy has explicit point traversal loops.
- Tuning opportunities: Batch and parallel profile generation for many input points.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional per-point profile metadata fields in output report for easier downstream comparison.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity appears high for point-seeded profile workflows.

## longest_flowpath

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/longest_flowpath.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools delineate one longest flowpath per basin with associated elevation/length/slope attributes.
- Important differences: NG uses local D8 direction/inflow graph arrays and modern vector emit helpers; legacy uses Array2D structures with similar traversal semantics.
- Correctness note: Basin-wise longest-path objective and output attribute meaning are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG uses tighter contiguous vector buffers for validity/flow/inflow passes; legacy performs multiple Array2D/channel-thread stages.
- Tuning opportunities: Benchmark large basin mosaics and optimize endpoint candidate filtering.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit interior-pit warning parity with legacy messaging when unresolved pits are encountered.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity for hydrologic longest-flowpath delineation.

## low_points_on_headwater_divides

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/low_points_on_headwater_divides.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools identify low pass points between neighbouring headwater basins derived from DEM flow structure and stream heads.
- Important differences: NG uses modern in-memory vector/raster buffers and helper routines; legacy uses older array/channel staging and value sorting flow.
- Correctness note: End-to-end pass-point detection workflow remains aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute multi-stage pointer/headwater/label traversal; NG uses contiguous vectors and coalesced progress pathways, legacy uses explicit multi-pass loops with more synchronization.
- Tuning opportunities: Add basin-edge candidate pruning to reduce neighbour-pair comparisons in large terrains.

### Design Improvements
- Severity: `Minor`
- Opportunity: Provide optional diagnostic outputs for intermediate headwater labels to aid validation.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core geomorphometric pass-detection parity looks strong.

## lowest_position

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/lowest_position.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both return the zero-based stack index of the lowest-valued raster in a stack for each cell.
- Important differences: NG exposes the behavior via shared GIS overlay/core code rather than a standalone legacy implementation.
- Correctness note: Lowest-position semantics and NoData behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG uses Rayon fold/reduce for multi-raster traversal and has lower overhead in output assembly.
- Tuning opportunities: Minor dense-stack optimization only.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## majority_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/majority_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply a moving-window majority/mode filter over categorical rasters.
- Important differences: NG uses an incremental quantized histogram while the window slides.
- Correctness note: Core majority selection semantics align with legacy.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's sliding histogram avoids repeated full-window recounts and is about 21% faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with a meaningful throughput improvement.

## map_features

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/map_features.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools perform descending-priority region-growth feature labelling with merge/override rules and minimum-size reconciliation.
- Important differences: NG keeps the legacy logic but modernizes data structures and coalesced progress flow; legacy includes more direct imperative state mutations.
- Correctness note: Feature segmentation and merge semantics are closely aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are heap-driven and inherently sequential in core growth, while NG parallelizes preprocessing and some post passes.
- Tuning opportunities: Explore tile-based frontier decomposition for partial parallelism in large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional debug layer outputs for replacement/override lineage.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity for complex feature-labelling behavior.

## map_off_terrain_objects

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/map_off_terrain_objects.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools map connected off-terrain objects using slope-constrained region growing with optional minimum feature-size suppression.
- Important differences: NG modernizes memory/output handling and uses consolidated progress mechanisms; core slope-connectivity logic matches legacy.
- Correctness note: Segment mapping and min-size reassignment behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by DFS/BFS-like region growth over raster neighbours with comparable per-cell checks.
- Tuning opportunities: Introduce optional union-find prepass for very large contiguous terrains to reduce stack churn.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional per-segment stats output (size, max height span) for QA workflows.

### Recommended Action
- `Accept`

### Notes
- Strong parity and low risk.

## max

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/max.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell maximums over raster-raster or raster-constant inputs.
- Important differences: NG routes the operator through a unified raster-math stats pipeline.
- Correctness note: The max operator semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are straightforward binary raster comparisons with similar work per cell.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## max_absolute_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/max_absolute_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the cell-wise value with the maximum absolute magnitude across raster stacks while preserving NoData rules.
- Important differences: NG uses the shared overlay reducer path in the GIS module.
- Correctness note: Overlay semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's shared overlay reducer is about 66% faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with a large performance win.

## max_anisotropy_dev

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_anisotropy_dev.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute maximum anisotropy in elevation deviation across scale windows and output magnitude plus dominant scale.
- Important differences: NG reimplements panel statistics with integral-image helper routines and parallel row kernels; legacy uses explicit pane-by-pane threaded channel logic.
- Correctness note: Core anisotropy formulation and max-over-scales behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG performs scale loops with rayon row-parallel computation and shared rectangle-sum helpers; legacy uses heavier per-thread channel fanout and repeated pane bookkeeping.
- Tuning opportunities: Optimize diagonal-panel aggregation to reduce repeated panel-stat calls for very large scale ranges.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional strict-legacy pane computation switch for validation experiments.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence parity with modernized NG internals.

## max_anisotropy_dev_signature

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_anisotropy_dev_signature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute anisotropy signatures at user points over scale and write an HTML line-graph report.
- Important differences: NG uses shared signature HTML writer and modern vector/raster parsing helpers; legacy builds report HTML inline with older rendering scaffolding.
- Correctness note: Signature-series computation and reporting intent are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both loop over scales and sites with similar panel-statistics cost; NG keeps computation paths compact but remains dominated by repeated per-site neighborhood statistics.
- Tuning opportunities: Parallelize per-site signature evaluation when many points are supplied.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional CSV sidecar export of per-scale signature values for downstream analysis.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity appears strong for signature workflows.

## max_branch_length

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_branch_length.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools estimate maximum branch length by tracing paired neighbouring D8 flowpaths until confluence/termination, with optional log transform.
- Important differences: NG computes D8 and paired tracing with contiguous vector buffers; legacy uses Array2D/state grids with similar trace logic.
- Correctness note: Branch-length objective and output semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both remain dominated by serial pairwise flowpath tracing; NG parallelizes direction derivation while legacy threads flow-direction preprocessing.
- Tuning opportunities: Explore cache-friendly path-marker reuse strategies to reduce repeated trace writes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit diagnostic counters for unresolved pits and early trace terminations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core divide-highlighting behavior remains consistent.

## max_difference_from_mean

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_difference_from_mean.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute maximum absolute difference-from-local-mean over a scale range and return magnitude plus winning scale.
- Important differences: NG uses consolidated integral-image helpers and parallel row kernels; legacy uses thread/channel loops with direct integral access.
- Correctness note: Scale sweep and selection semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG runs row computations via rayon and simplified update loops; legacy incurs channel synchronization overhead each scale.
- Tuning opportunities: Fuse magnitude/scale update loops further to reduce memory traffic.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional signed-vs-absolute selection mode documentation to clarify interpretation.

### Recommended Action
- `Accept`

### Notes
- Strong parity with low algorithmic risk.

## max_downslope_elev_change

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_downslope_elev_change.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools return the greatest elevation drop from each cell to any lower neighbour.
- Important differences: NG uses parallel row-map execution with shared offset arrays; legacy uses threaded channel workers.
- Correctness note: Max-drop neighborhood semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both perform identical 8-neighbour comparisons; NG avoids per-row channel messaging and writes rows directly from parallel vectors.
- Tuning opportunities: SIMD-friendly neighborhood loop unrolling could further reduce per-cell cost.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output of corresponding downslope direction index for QA/debug use.

### Recommended Action
- `Accept`

### Notes
- Clear parity and straightforward behavior.

## max_elevation_deviation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_elevation_deviation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both implementations now follow the same DEVmax method and thresholding behavior: build integral images, evaluate standardized elevation deviation across scales, and retain the maximum absolute response plus winning scale, with `min_vertical` suppression support.
- Important differences: NG keeps the same multiscale kernel in shared terrain-window infrastructure and validates `min_vertical` as a finite non-negative value.
- Correctness note: The previously missing `min_vertical` behavior has been restored in NG.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Roughly similar, with a slight edge to legacy on memory traffic.
- Evidence from code: Both implementations use integral-image preprocessing and parallelize per-scale per-row DEV calculation. NG now removes explicit output-initialization passes and uses nodata-initialized working buffers for max-response/scale selection before final row writes, reducing redundant raster I/O while preserving deterministic merge semantics.
- Tuning opportunities: Remaining opportunity is reducing per-scale `row_data` buffering (legacy-style streamed merge) if benchmarks show memory traffic still dominates.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures covering low-relief terrain to verify `min_vertical` suppression parity across representative scale ranges.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- The dropped `min_vertical` parameter gap is now closed.

## max_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/max_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the cell-wise maximum across stacked rasters with NoData propagation.
- Important differences: NG uses the same shared GIS overlay path as other stack reducers.
- Correctness note: Max-overlay semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's overlay reducer is about 65% faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with a large performance win.

## max_upslope_elev_change

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/max_upslope_elev_change.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute the maximum elevation gain from each cell to higher neighbouring cells.
- Important differences: NG modernizes execution with rayon row kernels; legacy uses thread/channel staging.
- Correctness note: Neighbourhood rise selection semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both apply the same 8-neighbour slope/rise logic; NG minimizes synchronization overhead and keeps compact write paths.
- Tuning opportunities: Add optional fused down/up change computation mode to reuse neighbour reads.

### Design Improvements
- Severity: `Minor`
- Opportunity: Include optional paired output with downslope change for combined relief diagnostics.

### Recommended Action
- `Accept`

### Notes
- Strong parity and low risk.

## max_upslope_flowpath_length

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/max_upslope_flowpath_length.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools propagate maximum upstream D8 flowpath length through each DEM cell.
- Important differences: NG uses compact vector-based topological propagation; legacy uses Array2D plus threaded inflow counting and explicit pit warnings.
- Correctness note: Flowpath-length propagation objective is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implement the same inflow-count stack propagation; NG is leaner in memory access, while legacy parallelizes preprocessing stages.
- Tuning opportunities: Parallelize inflow count construction in NG for large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style interior pit warning text for behavior transparency.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Hydrologic semantics match with mostly architectural differences.

## max_upslope_value

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/max_upslope_value.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools propagate the maximum upstream value raster along D8 flow paths defined from a DEM.
- Important differences: NG uses vector-based flow/inflow arrays and concise propagation loops; legacy uses Array2D structures and threaded prepasses.
- Correctness note: Max-upstream-value propagation behavior is equivalent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both use identical topological stack traversal from source cells; NG reduces overhead with contiguous vectors while legacy parallelizes some setup phases.
- Tuning opportunities: Add optional SIMD-friendly batch updates for large contiguous flow networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Emit optional ancillary raster indicating source-cell location of max value.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity for upstream maximum propagation.

## maximal_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/maximal_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute maximal curvature from the standard curvature tensor family and preserve the same overall output meaning.
- Important differences: NG uses the shared pro-curvature pipeline and fixed an out-of-memory regression by streaming rows instead of holding larger intermediates.
- Correctness note: Core curvature semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG is about 24% faster on the tested fixture after the streaming-row fix.
- Tuning opportunities: Minimal now that the OOM issue is resolved.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep the streaming row-worker layout; it appears to improve both stability and throughput.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity after the NG memory fix.

## maximum_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/maximum_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform moving-window maximum filtering over raster neighborhoods.
- Important differences: NG uses the shared window-stats filter kernel.
- Correctness note: Maximum-filter semantics align.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker results indicate NG is about 4% slower on the tested fixture despite shared-kernel modernization.
- Tuning opportunities: Reduce shared-kernel overhead and verify whether the regression is stable across window sizes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Profile the shared window-stats path for avoidable allocations or branch overhead.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Semantics are fine; this is a small runtime regression item.

## mdinf_flow_accum

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/mdinf_flow_accum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools implement MD-infinity triangular facet flow accumulation with exponent control, convergence threshold, and output type variants.
- Important differences: NG routes through shared flow-algorithm cores and common output post-processing helpers; legacy is a dedicated in-tool implementation.
- Correctness note: Parameter semantics and accumulation intent are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy explicitly parallelizes inflow-neighbour preprocessing; NG centralizes logic in core routines and keeps similar downstream dependency-driven propagation structure.
- Tuning opportunities: Reconfirm inflow-count preprocessing parallelism in NG core for large DEMs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics/warnings for interior pits consistent with legacy messaging.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good algorithmic parity with implementation refactoring differences.

## mean_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_PLAN_CURVATURE|BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/mean_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute mean curvature from the same curvature tensor family and preserve the expected signed curvature meaning.
- Important differences: NG uses the shared curvature kernel and the streaming-row fix also used in adjacent curvature tools.
- Correctness note: Core curvature semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG is about 5% faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity with a modest NG speedup.

## mean_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/mean_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute moving-window arithmetic mean smoothing over raster neighborhoods.
- Important differences: NG reuses the shared window-stats kernel.
- Correctness note: Mean-filter semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG is about 19% faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with a healthy speedup.

## median_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/median_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform moving-window median smoothing over raster neighborhoods.
- Important differences: NG uses the shared rank-filter kernel.
- Correctness note: Median-filter semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG is about 0.8% slower on the tested fixture, which is within near-parity range.
- Tuning opportunities: Minor histogram/update-path tuning only if later benchmarks show a stable gap.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This is effectively parity for audit purposes.

## medoid

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/medoid.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools output medoid point(s) based on nearest observed coordinate to median-center location.
- Important differences: NG uses modern geometry traversal and parallel per-feature medoid extraction for non-point layers; legacy uses mostly serial loops.
- Correctness note: Point-layer global medoid and per-feature non-point medoid behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes non-point feature medoid derivation and avoids repeated shapefile record access overhead; legacy is serial for all cases.
- Tuning opportunities: Improve point-layer global medoid scalability with spatial sampling for massive point sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional alternative medoid definitions for robustness comparisons.

### Recommended Action
- `Accept`

### Notes
- Strong parity with modest NG throughput upside.

## merge_line_segments

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/merge_line_segments.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools merge connected line segments at non-branching endpoints with snap-tolerance control.
- Important differences: NG uses quantized endpoint hashing and forward/backward extension logic; legacy uses KD-tree endpoint matching and staged loop handling.
- Correctness note: Endpoint-degree-based merge semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are graph-traversal dominated around endpoint adjacency; NG uses hash adjacency and dedup helpers, legacy uses KD-tree queries per segment.
- Tuning opportunities: Benchmark high-density endpoint clusters and consider adaptive spatial indexing strategy.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional topology-validation summary (branch points skipped, loops merged, segment counts).

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good practical parity with different endpoint-indexing internals.

## merge_table_with_csv

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/merge_table_with_csv.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools merge CSV attributes into a primary vector table by key matching, with optional single-field import.
- Important differences: NG uses a typed CSV parser/inference path and explicit field-definition construction; legacy infers delimiter and field types on the fly during line scanning.
- Correctness note: Join behavior and null-fill semantics for unmatched keys are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by CSV parse + hash-map join + feature rewrite; NG is structurally cleaner but still single-threaded for core join loops, similar to legacy.
- Tuning opportunities: Stream CSV parsing with pre-sized maps for very large join tables.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add duplicate-key collision diagnostics and an optional conflict policy (`first`/`last`/`error`).

### Recommended Action
- `Accept`

### Notes
- Solid parity with low semantic risk.

## merge_vectors

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/merge_vectors.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools merge multiple vectors of the same geometry type into one output with `FID`, `PARENT`, `PARENT_FID`, and common shared fields.
- Important differences: NG uses schema-level field intersection over typed layer objects; legacy uses attribute intersection over shapefile tables.
- Correctness note: Output field contract and per-feature parent attribution match legacy intent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations iterate layers/features serially and perform per-feature attribute assembly; neither introduces substantial extra passes.
- Tuning opportunities: Batch attribute writes for large multi-layer merges.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional strict schema mode requiring identical field order/type across inputs.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## min

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/min.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell minimums over raster-raster or raster-constant inputs.
- Important differences: NG uses a unified raster-math stats pipeline.
- Correctness note: Min operator semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: This is a straightforward per-cell operator with similar work per cell in both code paths.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence parity.

## min_absolute_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/min_absolute_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both return the per-cell minimum by absolute value across raster stacks.
- Important differences: NG uses the shared overlay reducer path.
- Correctness note: Overlay semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's shared reducer is materially faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## min_dist_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/min_dist_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools perform supervised minimum-distance classification from polygon training data and optional z-score distance thresholding.
- Important differences: NG currently exposes this as open-tier tooling and uses consolidated raster-stack/vector extraction helpers; legacy implementation is Pro-gated and uses older scanline sampling scaffolding.
- Correctness note: Core class-mean distance assignment and threshold logic are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG classifies rows with rayon parallelism after training-stat preparation, while legacy spends more time in nested scanline intersections and thread-channel coordination.
- Tuning opportunities: Parallelize training-pixel extraction for very large training polygon sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional sampling controls for training extraction density to improve speed/robustness trade-offs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Functional parity is strong; licensing-tier change is a product decision, not an algorithm defect.

## min_downslope_elev_change

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/min_downslope_elev_change.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute the minimum non-negative downslope elevation drop among 8 neighbours.
- Important differences: NG modernizes execution with rayon row kernels and direct row-slice writes; legacy uses thread/channel row transfer.
- Correctness note: Neighbour scan, slope condition, and output assignment semantics match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Identical 8-neighbour work per cell, but NG avoids channel overhead and uses compact parallel row maps.
- Tuning opportunities: Minor SIMD/vectorization opportunity in neighbour loop.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optionally return neighbour direction of selected minimum drop for diagnostics.

### Recommended Action
- `Accept`

### Notes
- Very strong parity.

## min_max_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/min_max_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform min-max contrast stretching with clipping of the input range to a target output range.
- Important differences: NG routes the remap through the shared non-filter remote-sensing pipeline.
- Correctness note: Contrast-stretch semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes the band remap and tested faster than legacy.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## min_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/min_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the per-cell minimum across stacked rasters with NoData propagation.
- Important differences: NG uses the shared GIS overlay reducer.
- Correctness note: Min-overlay semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's shared reducer is much faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## minimal_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/minimal_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute minimal curvature from the same curvature-tensor family.
- Important differences: NG uses the shared pro-curvature kernel and the same streaming-row structure as related curvature tools.
- Correctness note: Core curvature semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG is faster overall in the tested fixture and avoids the legacy memory-heavy path.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep the streamed row-worker structure; it appears to help both memory usage and speed.

### Recommended Action
- `Accept`

### Notes
- Strong parity with modernized implementation.

## minimal_dispersion_flow_algorithm

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/minimal_dispersion_flow_algorithm.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute a minimal-dispersion flow solution that returns directions and accumulation-style outputs.
- Important differences: NG consolidates the algorithm in the shared flow-algorithms module with modernized plumbing.
- Correctness note: Core flow-routing semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG is about one-third faster on the tested fixture.
- Tuning opportunities: Main cost remains the routing core, so future gains likely require deeper algorithmic changes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a small diagnostic summary of chosen flow directions if helpful for debugging.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with a substantial speedup.

## minimum_bounding_box

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/minimum_bounding_box.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute oriented minimum bounding boxes for individual features or whole-layer aggregate with configurable minimization criterion.
- Important differences: NG uses unified geometry-coordinate collectors and epsilon-aware helpers; legacy directly builds per-record point vectors.
- Correctness note: Criteria handling (`area/perimeter/length/width`) and per-feature/global behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG parallelizes per-feature hull/box generation in individual mode; legacy is serial but lightweight. End-to-end balance depends on feature count and geometry complexity.
- Tuning opportunities: Reduce coordinate collection overhead for very large multipart geometries.

### Design Improvements
- Severity: `Minor`
- Opportunity: Emit optional box orientation/length/width attributes directly in output.

### Recommended Action
- `Accept`

### Notes
- Practical behavior matches well.

## minimum_bounding_circle

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/minimum_bounding_circle.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute minimum enclosing circles and emit polygon approximations (default 128 vertices) per feature or globally.
- Important differences: NG uses common helper routines for ring creation and small-circle solving over generic vector geometry types.
- Correctness note: Circle construction and output semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most time in smallest-enclosing-circle computation and ring construction; NG parallelizes individual-feature mode while legacy is serial.
- Tuning opportunities: Adaptive ring vertex count based on radius/target tolerance.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional attributes for center/radius in output schema.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## minimum_bounding_envelope

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/minimum_bounding_envelope.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate axis-aligned bounding envelope polygons per feature or for the entire layer.
- Important differences: NG computes bounds through generic geometry utilities and supports broader vector backends; legacy uses shapefile record/header extents.
- Correctness note: Envelope geometry semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature envelope generation in individual mode; legacy processes records serially.
- Tuning opportunities: Minimal; operation is already lightweight.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optionally emit envelope width/height/area attributes.

### Recommended Action
- `Accept`

### Notes
- Straightforward parity.

## minimum_convex_hull

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/minimum_convex_hull.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute convex hull polygons for each feature or for all features combined.
- Important differences: NG uses generalized coordinate extraction and epsilon-aware hull helpers; legacy applies convex hull directly on shapefile point collections.
- Correctness note: Hull creation semantics and output modes are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes individual-feature hull construction; legacy runs serial loops across records.
- Tuning opportunities: Consider optional simplification pre-pass for extremely dense geometries.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional retention of source feature ID in aggregate mode via mapping table.

### Recommended Action
- `Accept`

### Notes
- Strong parity with favorable NG throughput characteristics.

## minimum_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/minimum_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform moving-window minimum filtering over raster neighborhoods.
- Important differences: NG uses the shared sliding-window cache path.
- Correctness note: Minimum-filter semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: NG is effectively at parity with legacy on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Stable parity.

## modified_k_means_clustering

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/modified_k_means_clustering.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools run modified k-means with large initial cluster counts and iterative centroid merging by distance threshold.
- Important differences: NG reuses a shared k-means core and report writer; legacy uses dedicated iterative loops with explicit printed progress and merger updates.
- Correctness note: Start-cluster, merge distance, iteration, and convergence-threshold semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy explicitly parallelizes assignment via thread channels; NG delegates to shared clustering core that is efficient but structured differently. Both remain dominated by repeated distance computations.
- Tuning opportunities: Validate and tune shared k-means core for large `start_clusters` values to ensure merge-step scalability.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional deterministic seed parameter for reproducible clustering runs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Behavior appears aligned with implementation modernization.

## modified_shepard_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/modified_shepard_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools now implement modified-Shepard interpolation with local quadratic basis correction support and distance-weighted neighbour blending.
- Important differences: NG keeps a modernized single-pass interpolation structure and conservative neighbour floor handling (`max(8)`), while legacy uses thread-channel orchestration and slightly different neighbour fallback defaults.
- Correctness note: The prior semantic gap (ignored `use_quadratic_basis`) has been closed; remaining differences are implementation-shape and tuning details.

### Performance Assessment
- Verdict: `MixedOrDataDependent`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: data-dependent.
- Evidence from code: Both implementations now incur basis-derivation plus interpolation neighbour queries; NG's structure is leaner but legacy parallelization strategy may win on some datasets/hardware.
- Tuning opportunities: Evaluate precomputed neighbour lists and optional rayon-based basis precompute when point counts are very high.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add focused parity tests that exercise both `use_quadratic_basis = true` and `false` paths against a fixed fixture.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- NG now includes explicit quadratic basis behavior compatible with legacy modified-Shepard intent.

## modify_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/modify_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools apply expression statements to mutate LiDAR point attributes, coordinates, timing, class flags, and colour channels.
- Important differences: NG uses unified expression-context helpers and supports batch/single modes through shared LiDAR I/O; legacy has broader Pro-era messaging and some header-scale/offset handling nuances.
- Correctness note: Core per-point expression-modification semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize point processing with expression-tree evaluation per point; both are CPU-bound on expression evaluation and attribute writes.
- Tuning opportunities: Cache expression target lookups to reduce per-point context churn.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit compatibility matrix in docs for all supported assignment targets.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong practical parity for expression-driven LiDAR edits.

## modify_nodata_value

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/modify_nodata_value.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools rewrite existing nodata pixels to a user-specified value and update output nodata metadata accordingly.
- Important differences: NG uses parallel fill helper over raster buffer; legacy uses thread/channel row workflows.
- Correctness note: Output value and metadata semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG performs a direct parallel buffer transform; legacy incurs additional row messaging and row writeback.
- Tuning opportunities: Minimal; current NG path is already efficient.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional warning when new nodata is outside safe range for integer output types.

### Recommended Action
- `Accept`

### Notes
- Clear parity with low risk.

## modulo

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/modulo.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute raster remainder cell-by-cell, preserving nodata and zero-division safety.
- Important differences: NG implements the operator via the shared binary math kernel.
- Correctness note: Modulo semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's binary math kernel is dramatically faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## mosaic

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/mosaic.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools mosaic multiple rasters with selectable resampling (`nn`, `bilinear`, `cc`) and overlapping-tile precedence behavior.
- Important differences: NG wraps logic in shared raster-stack and reprojection helpers; legacy manually handles tile indexing and overlap traversal.
- Correctness note: Mosaic intent and user-facing method semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy uses explicit per-row threaded sampling with tile search structures; NG delegates to shared mosaic kernels with similar computational profile.
- Tuning opportunities: Benchmark tile-overlap density cases and optimize candidate tile retrieval ordering.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit deterministic tie-break documentation for overlapping valid cells.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- No high-risk semantic gaps observed.

## mosaic_with_feathering

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/mosaic_with_feathering.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools blend two overlapping rasters using edge-distance feather weights with configurable resampling method and exponent.
- Important differences: NG centralizes implementation in reusable mosaic-feather kernel; legacy computes distance rasters and blends directly in-tool.
- Correctness note: Feather weighting behavior is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform distance-precompute and per-cell blending with comparable complexity; both use parallel row processing.
- Tuning opportunities: Accelerate distance-raster creation with shared integral/scanline primitives.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional blend-mask output for QA of overlap influence.

### Recommended Action
- `Accept`

### Notes
- Strong parity in overlap blending semantics.

## multidirectional_hillshade

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L3`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multidirectional_hillshade.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/basic_terrain_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute multi-azimuth hillshade with projected and geographic branches.
- Important differences: NG centralizes the workflow in shared terrain helpers and preserves the same overall output family.
- Correctness note: Hillshade intent matches legacy.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker results indicate this is the one clear slowdown in this wave.
- Tuning opportunities: Reduce repeated sun-angle passes and inspect shared-helper overhead.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Profile the multi-azimuth helper path; this looks like the best candidate for a targeted speedup.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Semantics are fine; runtime needs attention.

## multipart_to_singlepart

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/multipart_to_singlepart.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools split multipart geometries into singlepart outputs, with polygon `exclude_holes` behavior.
- Important differences: NG uses generalized geometry expansion and deterministic FID regeneration; legacy uses shapefile-part iteration and hole-to-hull assignment loops.
- Correctness note: Main splitting semantics are aligned across point/line/polygon modes.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature decomposition before serial output writes; legacy is predominantly serial.
- Tuning opportunities: Avoid post-pass progress emission by using part-count based coalescing.

### Design Improvements
- Severity: `Minor`
- Opportunity: Emit optional mapping table from source FID to produced child FIDs.

### Recommended Action
- `Accept`

### Notes
- Good parity with modernized geometry handling.

## multiply

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/multiply.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform per-cell multiplication between rasters or a raster and a scalar.
- Important differences: NG uses the shared binary raster math kernel.
- Correctness note: Multiplication semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG's binary raster op is extremely fast on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## multiply_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/multiply_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute the per-cell product across stacked rasters with NoData propagation.
- Important differences: NG uses the shared GIS overlay reducer path.
- Correctness note: Multiply-overlay semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG is much faster on the tested fixture.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## multiscale_curvatures

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_curvatures.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/multiscale_curvatures.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools now share the same multiscale intent and closely aligned scale-space defaults. NG now defaults to the same minimum scale as legacy and applies a two-path smoothing strategy (direct Gaussian for small scales and a fast almost-Gaussian approximation for larger scales), reducing the largest semantic gaps that previously prevented equivalency claims.
- Important differences: NG still uses a modernized helper implementation rather than a line-by-line port of the legacy smoothing kernel and threading layout, so residual differences may remain in edge and NoData handling. Geographic derivative handling and curvature formulations remain aligned with legacy equations.
- Correctness note: This is now best classified as an acceptable partial divergence rather than a redesign blocker; a focused numeric parity test pass would be the next step before upgrading to full equivalence.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now parallelizes both Gaussian smoothing sweeps and both fast-box passes (horizontal and vertical), in addition to existing parallel curvature evaluation, and it already collapsed standardization/best-response selection overhead. This removes the previous serial vertical fast-box hotspot and shifts the remaining differences to implementation details rather than obvious pass-level bottlenecks.
- Tuning opportunities: Main remaining work is representative terrain benchmarking and any targeted edge/NoData micro-optimizations identified from those runs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Remaining work is mainly verification and edge-case alignment (NoData and border behavior) rather than foundational redesign.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- The high-confidence low-risk performance refactors are in place; benchmark retest should decide whether any additional smoothing specialization is worthwhile.

## multiscale_elevated_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_elevated_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implementations follow the same conceptual workflow: build a nonlinear scale list, smooth the DEM at each scale using the Gaussian-scale-space approximation, compute DIFF-Gaussian, standardize by the scale raster standard deviation, and keep the maximum positive standardized response plus its key scale. The NG helper consolidates the implementation but retains the same elevated-index behavior.
- Important differences: The legacy docs advertise `step_nonlinearity=1.0`, but the legacy code actually defaults to `1.1`; NG also defaults to `1.1`, so NG is aligned with legacy code rather than legacy prose. NG returns through the standard wbtools output path conventions, but the underlying raster semantics remain aligned.
- Correctness note: No major semantic defect is apparent in NG for this tool. If anything, NG is cleaner in that it factors the shared elevated/low-lying logic into one helper without changing the actual selection rule.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: Legacy performs multiple explicit raster-wide passes per scale: smoothing, diff raster creation, raster-wide variance accumulation, scaled raster creation, and final best-response comparison. NG still performs the same high-level stages, but it fuses diff creation and sum-of-squares accumulation into one pass, avoids materializing separate `Raster` objects for both diff and scaled products, and updates the winning response directly from the in-memory vectors. The helper is also substantially less allocation-heavy than legacy’s repeated intermediate raster construction.
- Tuning opportunities: The remaining improvement would be to parallelize more of the post-smoothing diff/stat selection path, but the existing NG structure is already a notable cleanup over legacy.

### Design Improvements
- Severity: `Moderate`
- Opportunity: The main value-add here is documentation clarity, especially around the actual default `step_nonlinearity` and the fact that the NG/legacy implementation is scale-mosaic oriented rather than a simple local filter.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- This tool is a useful example where a deep audit may conclude that NG should be kept, not made more legacy-like, because the NG structure is leaner without obvious semantic loss.

## multiscale_elevation_percentile

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_elevation_percentile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute multiscale local elevation percentile/rank surfaces over neighborhood windows.
- Important differences: Legacy relies more heavily on floor-based binning while NG uses optimized windowing internals.
- Correctness note: Core percentile surface semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show NG faster (about 9.7%) via optimized neighborhood traversal.
- Tuning opportunities: Minor tuning for very large window sizes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document any subtle percentile tie-handling behavior if users compare exact values across versions.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## multiscale_low_lying_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `PERF_RETEST_REQUIRED`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_low_lying_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: This is the sign-inverted companion to multiscale elevated index in both codebases. Both implementations compute the standardized DIFF-Gaussian response across the sampled scales, retain only negative responses, sign-reverse them, and write the strongest low-lying response and key scale.
- Important differences: As with MsEI, the main discrepancy is documentation drift rather than code drift: legacy prose suggests a `step_nonlinearity` default of `1.0`, while the actual code defaults to `1.1`, which NG matches. The algorithmic selection rule itself appears aligned.
- Correctness note: No substantive semantic issue is apparent in the NG port for this tool.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: The same structural observation as MsEI applies here. Legacy materializes smoothed, diff, and scaled rasters in separate stages and uses more full-raster passes. NG keeps the source data in flat vectors, fuses diff creation with variance accumulation, and updates the winning low-lying response directly without constructing the same chain of intermediate raster objects.
- Tuning opportunities: As with MsEI, further speedups likely come from tightening the smoothing path or parallelizing more of the post-smoothing reduction, not from restoring the legacy multi-pass structure.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Keep the shared NG helper, but tighten documentation so the public defaults and explanation of key-scale behavior match the actual implementation.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- MsEI and MsLLI should probably be audited and benchmarked as a pair going forward because they share the same NG kernel and differ mainly in the selection criterion.

## multiscale_roughness

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_roughness.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools follow the same roughness-family workflow: smooth DEM by scale, compare normals of unsmoothed vs smoothed surfaces, and keep maximum response with key scale.
- Important differences: NG currently uses modernized helper structure and now matches legacy loop bounds (`min_scale..max_scale`) for sampled scales; residual variation risk is mainly implementation-shape details and edge/NoData handling.
- Correctness note: This is no longer a redesign blocker; remaining work is numeric regression validation across representative terrains.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG still follows a different smoothing/integral workflow than legacy, but now removes several avoidable intermediate materializations in its per-scale loop (flat-buffer smoothing, roughness-angle, and averaging stages replacing nested row-buffer collections). Legacy retains specialized large-kernel smoothing branches, so hotspot balance can still vary by terrain.
- Tuning opportunities: Benchmark before any further structural change; remaining candidates are deeper smoothing-kernel alignment and additional integral-reuse opportunities.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add focused numeric fixtures to confirm parity stability at large scales and nodata-heavy edges.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Important for downstream users expecting historical roughness signatures.

## multiscale_roughness_signature

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_roughness_signature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute site-based roughness signatures from the same roughness kernel family, and NG now follows legacy scale-iteration bounds used by the raster companion tool.
- Important differences: HTML rendering stack is modernized in NG and may differ cosmetically from legacy output pages; residual numeric differences are expected to be minor and tied to edge handling.
- Correctness note: Signature parity is now best treated as acceptable partial divergence pending numeric regression fixtures.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by repeated per-scale smoothing and site extraction; NG now trims a low-risk extra materialization stage by using flat per-scale buffers (instead of nested row-buffer collections) before integral-based site extraction. Legacy still relies on broader threaded loops and older output plumbing.
- Tuning opportunities: Reuse cached scale intermediates across site extraction and benchmark whether additional smoothing reuse is worthwhile.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add fixture-based signature comparisons at fixed sites and scales to lock down numeric parity.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Signature parity should follow core roughness parity decision.

## multiscale_std_dev_normals

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_std_dev_normals.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG now aligns with legacy on the high-impact semantic points: `min_scale` default is 4 and smoothing uses the same Gaussian/fast-almost-Gaussian scale-space helper pattern already used in other multiscale terrain tools.
- Important differences: NG still uses modernized orchestration and buffer handling, so small numeric differences can remain around border and nodata transitions.
- Correctness note: This closes the largest parity gaps; remaining work is targeted numeric confirmation rather than redesign.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG still rebuilds smoothed rasters and component integrals per scale, but now removes an extra row-wise intermediate materialization in the per-scale standard-deviation response stage by using a flat parallel buffer. Legacy keeps heavier but optimized smoothing branches and threaded loops, so results remain terrain dependent.
- Tuning opportunities: Benchmark current state first, then evaluate shared smoothing acceleration paths and possible integral reuse.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add fixed-fixture comparisons to validate spherical std-dev values and key-scale selection at representative sites.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- High-impact tool for roughness-scale interpretation.

## multiscale_std_dev_normals_signature

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_std_dev_normals_signature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG signature generation now inherits the same aligned std-dev kernel defaults and smoothing strategy as the raster variant (`min_scale` 4 and legacy-like scale-space smoothing path).
- Important differences: Report rendering remains modernized and may differ visually from legacy HTML; residual numeric differences may persist in edge and nodata neighborhoods.
- Correctness note: This is now a targeted verification item rather than a redesign-level divergence.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both iterate scales and compute smoothed intermediates, but NG now removes a low-risk extra pass by computing unit-normal component integrals directly from the smoothed flat buffer (without writing/reading an intermediate smoothed raster each scale). Legacy still uses its own threaded smoothing/integral shape, so overall hotspot balance remains terrain dependent.
- Tuning opportunities: Benchmark current state; remaining opportunities are shared per-scale component caching and deeper smoothing-kernel reuse.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add signature regression fixtures for fixed points across scales to verify value-shape parity.

### Recommended Action
- `TargetedParityFollowup`

### Notes
- Treat as coupled with parent tool redesign.

## multiscale_topographic_position_image

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/multiscale_topographic_position_image.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compose local/meso/broad DEVmax rasters into an RGB packed output using the same logistic lightness mapping and optional hillshade modulation.
- Important differences: NG wraps this in the shared raster I/O and packed-RGB metadata path; legacy uses explicit thread/channel row assembly.
- Correctness note: Channel mapping (broad->R, meso->G, local->B), nodata handling, and hillshade scaling semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy parallelizes row computation across worker threads; NG currently runs a serial row loop but keeps a lightweight per-cell kernel and contiguous row writes. Both are linear memory-bandwidth dominated.
- Tuning opportunities: Add optional rayon row parallelism in NG for very large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional per-channel gain controls for visualization tuning.

### Recommended Action
- `Accept`

### Notes
- Strong parity for this visualization utility.

## narrowness_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/narrowness_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `LikelyNearParity`
- Summary: NG `narrowness_index` now matches legacy raster semantics by computing per-patch area divided by $\pi \cdot MD^2$ where $MD$ is the maximum distance-to-edge, and maps the per-patch value back to all patch cells.
- Important differences: NG keeps the former vector perimeter-over-sqrt(area) metric under a new id (`narrowness_index_vector`) to avoid functionality loss.
- Correctness note: Core legacy identity mismatch has been resolved; remaining differences are implementation-level details (shared raster wrapper, progress reporting, and bounds-safe neighbor checks).

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both legacy and NG execute a four-stage raster workflow (edge initialization, forward/backward distance propagation, per-patch max-distance aggregation, and patch remap). NG currently executes serial row loops similar to many other patch-shape tools in this module.
- Tuning opportunities: Add optional row-level rayon parallelism in initialization/remap phases for very large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider documenting `narrowness_index_vector` in the same shape-metrics section where users may have previously discovered the vector metric under `narrowness_index`.

### Recommended Action
- `Accept`

### Notes
- Parity blocker addressed by raster tool identity restoration.

## natural_neighbour_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/natural_neighbour_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools now implement Sibson-style natural-neighbour interpolation with Voronoi area-stealing weights derived from Delaunay neighbourhoods.
- Important differences: NG uses a prepared reusable Sibson interpolator path with cached triangulation/Voronoi structures, while legacy repeatedly reconstructs local neighbourhood triangulations during raster traversal.
- Correctness note: NG now follows true natural-neighbour weighting intent rather than an inverse-distance approximation.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both paths now perform true Sibson weighting. NG precomputes and reuses Delaunay/Voronoi/cavity structures, which should reduce per-cell reconstruction overhead relative to legacy's repeated local triangulation flow.
- Tuning opportunities: Add row-parallel execution with thread-local Sibson scratch buffers and benchmark against legacy on dense grids.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add an optional explicit `mode` (`true_sibson` / `fast_approx`) only if a speed-first approximation mode is still desired for very large runs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- The major semantic gap is closed; remaining work is performance benchmarking and optional speed-mode ergonomics.

## nearest_neighbour_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/nearest_neighbour_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools rasterize point values by assigning each cell the nearest sample value, with optional `max_dist` clipping and base-raster/cell-size grid control.
- Important differences: Legacy uses fixed-radius search infrastructure with nearest fallback behavior baked into that structure; NG uses KD-tree nearest queries directly.
- Correctness note: Core nearest assignment and distance-threshold semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform one nearest-neighbour query per output cell over point index structures; both remain dominated by raster-cell query count.
- Tuning opportunities: Row-parallelism in NG would improve very large grids.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional tie-break policy documentation for equal-distance cases.

### Recommended Action
- `Accept`

### Notes
- High-confidence functional parity.

## negate

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/negate.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools negate each non-nodata cell value while preserving nodata.
- Important differences: NG implements negate via a shared unary-math tool macro; legacy uses a dedicated function with explicit row workers.
- Correctness note: Numeric transform semantics are identical (including `0 -> 0`).

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are embarrassingly parallel unary raster transforms; NG's shared vectorized/par buffer path avoids thread-channel row transfer overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required beyond standard unary-math consistency tests.

### Recommended Action
- `Accept`

### Notes
- Simple and solid parity.

## new_raster_from_base_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/new_raster_from_base_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools create a new raster using base raster geometry/CRS with selectable output data type and optional fill value.
- Important differences: Legacy normalizes output nodata to `-32768` regardless of base nodata; NG retains base nodata unless user-provided fill is specified.
- Correctness note: Geometry and data-type behavior match; nodata default convention differs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are metadata cloning + optional full-buffer fill operations.
- Tuning opportunities: Clarify and unify nodata fill initialization behavior.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Decide whether NG should adopt legacy fixed nodata convention for strict parity.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Main mismatch is default nodata policy.

## new_raster_from_base_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/new_raster_from_base_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools create a raster from vector extent + cell size with optional fill and data-type selection.
- Important differences: NG uses modern vector-layer bbox and CRS helpers; legacy pulls extent from shapefile header.
- Correctness note: Rows/cols extent derivation, nodata default (`-32768`), and datatype options are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Operation cost is dominated by raster allocation/fill only in both implementations.
- Tuning opportunities: None material.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional guardrail warning for very large inferred grid sizes.

### Recommended Action
- `Accept`

### Notes
- Strong parity and low risk.

## nnd_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/nnd_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform nearest-normalized-distance class assignment from multi-band predictors with class-wise distance normalization and outlier rejection controls.
- Important differences: Legacy also includes train/test split diagnostics, accuracy/kappa, variable-importance reporting, and model-only run modes that NG does not yet replicate.
- Correctness note: Classification core is present and aligned; remaining divergence is primarily evaluation/report contract scope.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by per-cell feature extraction + per-class neighbour distance queries; NG is cleaner but omits some legacy evaluation overhead.
- Tuning opportunities: Add optional evaluation mode and compare end-to-end throughput separately from pure classification.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional evaluation/report mode (accuracy/kappa/importance/test split) while keeping the current lightweight classification path.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Treat as contract-depth follow-up rather than algorithm redesign.

## normal_vectors

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/normal_vectors.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools estimate local LiDAR point normals via neighbourhood plane fitting and encode the normal into RGB-like output.
- Important differences: NG additionally stores explicit normal components (`normal_x/y/z`) in point records; legacy primarily writes encoded RGB and sets LAS point format accordingly.
- Correctness note: Core normal-estimation intent and colour encoding behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both build a spatial index then run per-point neighbourhood plane fitting in parallel; both are neighbour-query and local linear-algebra dominated.
- Tuning opportunities: Cache neighbourhoods for dense clouds with repeated spatial overlap.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document the additional NG normal component fields as an intentional enhancement.

### Recommended Action
- `Accept`

### Notes
- Good parity with useful NG metadata extension.

## normalize_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/normalize_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools normalize LiDAR elevation by subtracting DTM ground elevation with optional negative-height clamping.
- Important differences: Legacy explicitly checks neighbouring DTM cells when the direct sample is nodata; NG uses a helper sampling path and assigns zero when no sample is available.
- Correctness note: Main normalization behavior is aligned, with potential edge-case differences at DTM nodata boundaries.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG performs a single parallel map over points with direct point-record rewrites; legacy computes residual arrays then does a second point-record reconstruction pass over LAS format variants.
- Tuning opportunities: Add optional interpolation mode control (nearest/bilinear) for reproducible boundary behavior.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Expose nodata-boundary fallback policy to match legacy behavior when strict reproducibility is needed.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Very close parity with some boundary-condition nuance.

## normalized_difference_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/normalized_difference_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute normalized difference index from two bands as `(b1 - b2) / (b1 + b2)` with nodata-aware handling.
- Important differences: Legacy exposes optional `correction_value` and `clip_percent` post-processing; NG currently exposes only band selection (`band1`, `band2`) and no correction/clip parameters.
- Correctness note: Core NDI computation is aligned, but optional legacy controls are missing in NG.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute per-cell NDI in parallel row-oriented passes and are bandwidth dominated.
- Tuning opportunities: If legacy clipping support is added to NG, fuse clipping into output write pass to avoid extra scan.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional `correction_value` and `clip_percent` aliases for strict legacy API parity.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Numerically similar outputs for default settings.

## not_equal_to

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/not_equal_to.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell inequality comparisons across raster/raster and raster/scalar paths.
- Important differences: NG routes the operator through shared binary math dispatch.
- Correctness note: Truth-table and nodata behavior align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show a very large NG speedup from consolidated binary-op execution.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity with substantial throughput improvement.

## num_downslope_neighbours

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/num_downslope_neighbours.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools count the number of 8-neighbour cells with lower elevation for each DEM cell.
- Important differences: NG is implemented through a shared terrain analysis core and supports multi-band processing consistently.
- Correctness note: Downslope count semantics (0-8 range, nodata propagation) are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize row computation; NG avoids thread-channel row transfer overhead and writes rows from collected parallel results.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional output of upslope/downslope pair in one pass could reduce duplicate reads across companion tools.

### Recommended Action
- `Accept`

### Notes
- Strong and straightforward parity.

## num_inflowing_neighbours

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/num_inflowing_neighbours.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools derive D8 flow directions from a DEM and count inflowing neighbours per cell.
- Important differences: Legacy emits an interior-pit warning side effect; NG focuses on raster output and omits that warning message.
- Correctness note: Core inflowing-neighbour count behavior is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy runs multi-threaded two-stage processing (direction then count); NG uses compact local direction computation and direct count rasterization with fewer orchestration layers.
- Tuning opportunities: Consider optional warning/diagnostics mode in NG to preserve legacy pit-detection side channel.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional interior-pit warning parity mode.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Raster outputs should largely match for conditioned DEMs.

## olympic_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/olympic_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform Olympic filtering by trimming extreme values in neighborhood samples before averaging.
- Important differences: NG is integrated into shared phase-3 filter infrastructure.
- Correctness note: Core filter behavior aligns.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show NG faster (about 16.7%) after refactoring to shared filter internals.
- Tuning opportunities: Minor allocation reductions in neighborhood handling.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep documenting RGB/HSI mode handling where relevant.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## opening

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/opening.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both implement grayscale morphological opening as erosion followed by dilation with odd-sized rectangular windows.
- Important differences: NG uses reusable `morph_erode`/`morph_dilate` kernels; legacy performs explicit two-loop execution in a dedicated tool.
- Correctness note: Window normalization and nodata-aware behavior are consistent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both use deque-based sliding-window style row kernels and parallel row execution for each morphology phase.
- Tuning opportunities: Consider in-place band chunking to reduce temporary allocations for very large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose explicit structuring-element shape option (future enhancement, not legacy parity requirement).

### Recommended Action
- `Accept`

### Notes
- Good parity.

## openness

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/openness.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/openness.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute positive/negative openness via directional horizon-angle sampling around each DEM cell.
- Important differences: NG uses Rayon-backed horizon scans and open-tier packaging in NG infrastructure.
- Correctness note: Output openness semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show a notable NG speedup (about 26.9%).
- Tuning opportunities: Minimal for current architecture.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with meaningful speed gain.

## otsu_thresholding

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/otsu_thresholding.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute Otsu thresholding from histogram class-variance maximization and apply binary threshold output.
- Important differences: NG parallelizes supporting min/max/histogram passes.
- Correctness note: Threshold-selection semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show NG faster (about 15.3%) from parallel preprocessing and output mapping.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## paired_sample_t_test

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/paired_sample_t_test.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute paired-difference statistics from two rasters, and NG now supports optional HTML reporting alongside structured JSON output.
- Important differences: NG remains API-first and currently uses a normal-approximation p-value path; legacy is file/report-first with fuller narrative/diagnostic presentation.
- Correctness note: Core paired-statistic workflow and reportability are aligned; remaining differences are p-value method and report-detail polish.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG computes reduced summary statistics directly and avoids heavy report rendering/graph generation and multi-stage formatting present in legacy.
- Tuning opportunities: If richer parity reporting is added, keep stats computation decoupled from rendering path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional follow-up: add t-distribution p-value mode and tighten HTML narrative/diagnostic details to match legacy wording.

### Recommended Action
- `Accept`

### Notes
- Report-contract gap is closed; residual differences are method/detail-level.

## panchromatic_sharpening

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/panchromatic_sharpening.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both support Brovey and IHS pan-sharpening from either composite RGB or separate bands plus a panchromatic raster.
- Important differences: NG adds explicit `output_mode` control (`packed` vs `bands`) and uses shared RGB handling helpers.
- Correctness note: Core fusion method semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform per-pixel fusion transforms over pan-resolution grids and are arithmetic-heavy but straightforwardly parallelizable.
- Tuning opportunities: Benchmark method-specific hotspots (IHS conversions) for large scenes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document default scaling behavior differences if any scene-dependent normalization is adjusted in helper paths.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong functional parity.

## parallelepiped_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/parallelepiped_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform supervised polygon-based parallelepiped classification from multi-band rasters using per-class min/max bounds and first-match assignment.
- Important differences: NG uses aligned-raster stack helpers and explicit class-hypervolume ordering logic in a modernized structure.
- Correctness note: Class-bound inclusion logic and output labelling are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform training-pixel extraction plus per-cell class inclusion checks; NG parallelizes row classification with compact vectorized memory paths.
- Tuning opportunities: Optional early class-pruning heuristics for high-band-count cases.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional unclassified mask output for QA parity workflows.

### Recommended Action
- `Accept`

### Notes
- Good semantic parity.

## patch_orientation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/patch_orientation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute polygon orientation from reduced-major-axis regression and append `ORIENT` in degrees from north.
- Important differences: NG uses generalized geometry helpers and parallel feature evaluation.
- Correctness note: Orientation metric and output field intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy iterates records serially; NG computes orientations via parallel feature maps then deterministic attribute appends.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional confidence/stability metric for near-degenerate polygons.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## pennock_landform_classification

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/pennock_landform_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both classify Pennock landforms from curvature/slope-derived thresholds.
- Important differences: NG has additional row-slice preload and tuple return plumbing in the modernized API surface.
- Correctness note: Classification intent aligns.

### Performance Assessment
- Verdict: `LegacyLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Legacy.
- Evidence from code: Tracker data marks this as a red item; NG is slower on the tested fixture.
- Tuning opportunities: Profile row preload and result assembly overhead in NG path.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Keep the current semantics but target post-derivative processing overhead in NG.

### Recommended Action
- `PerformanceTuningOnly`

### Notes
- Semantic parity appears sound; this is primarily a runtime regression.

## percent_elev_range

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/percent_elev_range.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute local topographic position as percent of neighbourhood elevation range using odd-sized moving windows.
- Important differences: NG uses shared filter-size parsers and row-parallel kernels; legacy uses explicit deque window buffers.
- Correctness note: Formula and nodata/range-zero handling are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute sliding neighbourhood min/max retrieval with row-wise parallelism and similar complexity.
- Tuning opportunities: Integral/monotonic-queue hybrid for very large windows.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## percent_equal_to

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/percent_equal_to.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell percentage of input stack members equal to a target value.
- Important differences: NG uses unified overlay enum dispatch.
- Correctness note: Percentage semantics and nodata handling align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show large NG speedups from shared reducer structure.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with major throughput gain.

## percent_greater_than

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/percent_greater_than.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell percentage of input stack members greater than a target value.
- Important differences: NG uses shared overlay comparison dispatch.
- Correctness note: Percentage-greater-than semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show large NG speedups on tested fixtures.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with large speedup.

## percent_less_than

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/percent_less_than.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell percentage of input stack members less than a target value.
- Important differences: NG uses shared overlay enum execution with early-exit opportunities.
- Correctness note: Percentage-less-than semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show large NG speedups on tested fixtures.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with large speedup.

## percentage_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/percentage_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform percentage-tail contrast stretching with clipping based on lower/upper percentile thresholds.
- Important differences: NG uses Arc-backed memory reuse and reduced allocation strategy.
- Correctness note: Core stretch semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker runs show strong NG speedups (about 42.4%).
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep documenting percentile clipping behavior for migration clarity.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity with substantial throughput improvement.

## percentile_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/percentile_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/rank_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply a moving-window percentile (rank) filter over raster neighborhoods.
- Important differences: NG routes through the shared rank-filter kernel family.
- Correctness note: Percentile-selection semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel row-window rank computations; NG benefits from lower orchestration overhead.
- Tuning opportunities: Minor optimization only for very large windows.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## perimeter_area_ratio

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/perimeter_area_ratio.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute polygon perimeter-to-area ratio and append `P_A_RATIO` to output attributes.
- Important differences: NG supports generalized geometry variants through shared perimeter/area helpers and parallel per-feature computation.
- Correctness note: Metric semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy processes features serially; NG computes values with rayon then appends deterministically.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional reciprocal compactness field output for convenience.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## phi_coefficient

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/phi_coefficient.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute binary-agreement metrics from two rasters and include phi; NG now supports optional legacy-style HTML report output while retaining structured JSON output.
- Important differences: Legacy remains Pro-gated and browser-launch oriented; NG is open-tier and API-first with optional `output` / `output_html_file` report paths.
- Correctness note: Core contingency-table math and reportability are aligned; remaining differences are product-tier and UX defaults.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both implementations parallelize raster scans and aggregate counts; legacy pays extra HTML serialization/launch overhead that NG avoids.
- Tuning opportunities: If HTML parity is restored, keep JSON as a low-overhead mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional polish only: tune HTML text/wording and formatting to match legacy phrasing more closely.

### Recommended Action
- `Accept`

### Notes
- Arithmetic parity is good; core report-contract gap is now closed.

## pick_from_list

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/pick_from_list.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithNGCorrection`
- Summary: Both perform per-cell raster-stack selection from a zero-based position raster.
- Important differences: NG adds optional automatic reprojection alignment and explicit out-of-range index protection (mapped to nodata), while legacy directly indexes the stack and can panic on invalid position values.
- Correctness note: NG behavior is a robustness correction with better failure handling for malformed position rasters.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform one main pass over output cells with parallel chunk/row processing.
- Tuning opportunities: Benchmark overhead of optional reprojection path on large mixed-CRS stacks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document legacy-incompatible safety behavior for invalid position values.

### Recommended Action
- `Accept`

### Notes
- Strong parity with safer NG behavior.

## piecewise_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/piecewise_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply piecewise linear brightness mapping for grayscale and packed RGB imagery, with intensity-domain transformation for RGB.
- Important differences: NG parser is stricter/cleaner, clamps output proportions explicitly, and supports single or multiple interior breakpoints; legacy requires at least two user-provided breakpoints and has slightly different statement parsing tolerances.
- Correctness note: Core transfer-function behavior is aligned for well-formed inputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform parallel min/max discovery plus parallel per-cell mapping; both run RGB HSI conversion paths when needed.
- Tuning opportunities: Consider LUT-based acceleration for very large grayscale rasters with dense breakpoint lists.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit compatibility note for breakpoint-count and parsing differences.

### Recommended Action
- `Accept`

### Notes
- High functional parity.

## plan_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `BENCHMARK_ANCHOR`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/plan_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute plan/contour curvature from DEM derivatives with projected/geographic handling.
- Important differences: NG consolidates curvature variants behind shared curvature dispatch.
- Correctness note: Core plan-curvature semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker marks this green; both are parallel neighborhood derivative kernels, with NG benefiting from shared optimized infrastructure.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep projected/geographic regression fixtures synchronized across curvature family tools.

### Recommended Action
- `Accept`

### Notes
- Good benchmark-anchor parity.

## polygon_area

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/polygon_area.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute polygon area (respecting interior holes) and append an `AREA` attribute.
- Important differences: NG uses generalized geometry helpers and supports broader vector geometry container forms under a common API.
- Correctness note: Area metric semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy iterates records serially; NG computes areas in parallel then appends attributes deterministically.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## polygon_long_axis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/polygon_long_axis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Legacy and NG now both output the long axis of each polygon's minimum bounding box.
- Important differences: NG uses shared helper dispatch and parallel feature processing; legacy uses per-record sequential logic.
- Correctness note: Axis-selection inversion is fixed and parity is restored.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute MBB-derived axes; NG parallelizes feature processing but currently computes the wrong axis target for this tool.
- Tuning opportunities: Fix semantics first; performance is secondary.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a regression test that verifies long-axis output length is always >= short-axis output length for representative polygons.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity bug resolved.

## polygon_perimeter

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/polygon_perimeter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute polygon perimeter (including interior ring boundaries) and append `PERIMETER`.
- Important differences: NG uses shared geometry utilities and supports generalized polygon containers.
- Correctness note: Metric semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy loops serially per feature; NG computes perimeters in parallel and appends deterministically.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong parity.

## polygon_short_axis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/polygon_short_axis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Legacy and NG now both output the short axis of each polygon's minimum bounding box.
- Important differences: NG routes through shared axis helpers and parallel feature processing; legacy uses direct per-record computation.
- Correctness note: Axis-selection inversion is fixed and parity is restored.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are MBB-based and cheap relative to I/O; NG parallelizes features but the axis target is incorrect.
- Tuning opportunities: Fix semantics first.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a regression test that verifies short-axis output length is always <= long-axis output length for representative polygons.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity bug resolved.

## polygonize

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/polygonize.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both polygonize intersecting line networks by splitting/traversing linework connectivity to form enclosed faces.
- Important differences: NG now routes through shared topology graph polygonization and supports multi-layer input aggregation; legacy uses its original in-tool traversal/report flow.
- Correctness note: Core intersecting-line polygonization semantics are now aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations now perform graph-style polygonization work over linework; NG benefits from shared topology routines while legacy has mature specialized traversal.
- Tuning opportunities: Benchmark dense, highly intersecting networks where node/edge explosion dominates runtime.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a regression fixture that polygonizes open/intersecting linework and verifies hole assignment parity against legacy outputs.

### Recommended Action
- `Accept`

### Notes
- Core capability gap closed; keep an eye on dense-network performance.

## polygons_to_lines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/polygons_to_lines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both convert polygon boundaries to line output while preserving attributes.
- Important differences: Legacy converts shape type directly to polyline records; NG emits multiline geometries explicitly (including interior rings) through generalized vector geometry types.
- Correctness note: Boundary conversion behavior is aligned for polygon inputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are linear in feature/ring count; NG parallelizes row preparation then writes output sequentially.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document output geometry-type differences (`PolyLine` vs `MultiLineString`) for strict downstream compatibility users.

### Recommended Action
- `Accept`

### Notes
- Practical parity is strong.

## prewitt_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/prewitt_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply Prewitt edge-detection kernels and output edge-intensity responses.
- Important differences: NG uses unified convolution dispatch shared with other edge filters.
- Correctness note: Kernel behavior is aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel convolution passes; NG benefits from streamlined shared convolution infrastructure.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## principal_component_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/principal_component_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform multi-raster PCA and emit component rasters, and NG now aligns explained-variance/loading calculations with legacy signed-eigenvalue conventions.
- Important differences: Legacy is HTML-report-first with scree/formatting UX, while NG remains JSON-report-first.
- Correctness note: The major statistical semantics mismatch has been resolved; remaining differences are report delivery/UX.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by full-stack covariance accumulation and eigendecomposition; NG parallelizes accumulation and output component synthesis broadly.
- Tuning opportunities: Re-check numerical stability and reporting semantics before low-level optimization.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional future enhancement: add legacy-style HTML PCA report mode (including scree plot table formatting).

### Recommended Action
- `Accept`

### Notes
- Statistical/report-core parity is now substantially aligned.

## print_geotiff_tags

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/print_geotiff_tags.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both expose GeoTIFF tag inspection, but legacy writes directly to stdout while NG returns a text report payload.
- Important differences: Legacy panics on non-TIFF-family inputs; NG returns warning reports for invalid/non-TIFF inputs and avoids hard failure. Legacy behavior is console-oriented, NG behavior is structured-output oriented.
- Correctness note: NG is more robust operationally, but output contract is not strictly equivalent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `NotApplicable`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by single-file metadata decode and string formatting.
- Tuning opportunities: None material.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optionally add explicit legacy-style console-print mode for parity-sensitive scripts.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Main gap is interface contract, not tag parsing intent.

## profile

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/profile.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both sample one or more profile lines over a raster surface and output an HTML elevation-vs-distance plot.
- Important differences: NG uses generalized vector geometry handling and a custom SVG rendering path; legacy uses its legacy `LineGraph` report stack and auto-opens the report in verbose mode.
- Correctness note: Core sampling/plot intent is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both primarily perform sequential polyline stepping and raster sampling per segment.
- Tuning opportunities: Add optional chunked/parallel profile extraction for very large multipart line sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional tabular output of sampled profile points in addition to HTML.

### Recommended Action
- `Accept`

### Notes
- Good practical parity.

## profile_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_PLAN_CURVATURE|BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/profile_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute profile curvature from local terrain derivatives.
- Important differences: NG is implemented in shared curvature dispatch with common helper paths.
- Correctness note: Profile-curvature output semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker marks this green and performance is consistent with nearby curvature-tool outcomes.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Maintain parity tests alongside plan/mean/gaussian curvature variants.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## prune_vector_streams

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/prune_vector_streams.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/pro_stream_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both prune stream vectors using Shreve-magnitude style topology traversal with DEM-guided downstream logic.
- Important differences: Legacy is Pro-licensed and includes a denser topological repair flow around intermediate-node handling and ridge-cut checks; NG reimplements the workflow in open tier with merged centerline preprocessing and compatibility parameters (`max_ridge_cutting_height`) but with refactored graph/queue logic.
- Correctness note: Core objective and decision axes align, but traversal internals are not identical and should be treated as implementation-divergent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both do substantial topology construction and priority traversal; NG shifts some preprocessing to modern vector helpers, legacy leans heavily on kd-tree endpoint logic.
- Tuning opportunities: Benchmark large dendritic networks; hotspot likely in line merge/snap plus downstream linkage resolution.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add targeted parity tests for known overshoot/undershoot topological error cases to verify behavioural equivalence.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics appear close but need dataset-backed confirmation.

## qin_flow_accumulation

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/qin_flow_accumulation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute Qin-style flow accumulation with adaptive slope-power dispersion behavior.
- Important differences: NG integrates the workflow in shared flow-algorithm infrastructure.
- Correctness note: Core accumulation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker is green and NG benefits from modernized dataflow in shared flow-algorithm code.
- Tuning opportunities: Additional profiling around convergence threshold paths.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for adaptive exponent/convergence behavior on edge-case terrains.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## quantiles

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/quantiles.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both convert raster values to quantile classes based on histogram/CDF partitioning.
- Important differences: NG uses shared raster-stats infrastructure.
- Correctness note: Quantile assignment semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both follow histogram accumulation plus remap; NG uses parallel reduction infrastructure.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## quinn_flow_accumulation

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/quinn_flow_accumulation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute Quinn-style multiple-flow-direction accumulation with adaptive dispersion behavior.
- Important differences: NG implements in shared flow-algorithms module alongside related accumulation methods.
- Correctness note: Core Quinn accumulation semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker is green and NG benefits from consolidated flow helper internals.
- Tuning opportunities: Profile steep-slope and flat-area branching balance.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep algorithm-specific regression fixtures across accumulation variants.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## radial_basis_function_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/radial_basis_function_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: NG now exposes a user-selectable `approximate_mode` parameter: fast similarity weighting when `true`, and local RBF system solving when `false` (legacy-style behavior).
- Important differences: The default mode remains the NG approximate path (`approximate_mode=true`) for speed; strict parity-style behavior is available explicitly with `approximate_mode=false`.
- Correctness note: The prior hard contract mismatch has been reduced to a mode-selection default and should be treated as a configurable behavior choice.

### Performance Assessment
- Verdict: `MixedOrDataDependent`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG in approximate mode; near tie vs legacy in local-solve mode depending on neighbourhood size and raster dimensions.
- Evidence from code: `approximate_mode=true` avoids per-cell local system solves and uses direct weighted accumulation; `approximate_mode=false` performs local RBF solves and is expected to be computationally heavier but closer to legacy math.
- Tuning opportunities: Add parallel row batching and neighbourhood-size guards for `approximate_mode=false` to reduce solve overhead on large grids.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep both modes, and ensure wrappers/docs surface mode intent clearly so users can select speed-first vs parity-first behavior intentionally.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Recommended default by use case:
- Set `approximate_mode=true` for fast exploratory interpolation and larger jobs.
- Set `approximate_mode=false` for legacy-aligned mathematical behavior and parity-sensitive workflows.

## radius_of_gyration

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/radius_of_gyration.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithNGCorrection`
- Summary: Both compute per-patch radius of gyration and map values back to patch cells with a tabular summary.
- Important differences: NG uses explicit two-stage accumulation (`sum` then squared-distance accumulation) with deterministic mapping; legacy row-local accumulation appears susceptible to under-accumulation behavior in its second loop.
- Correctness note: NG appears to preserve intended metric semantics while avoiding fragile accumulator behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes expensive cellwise passes and keeps compact bin-wise arrays; legacy uses threaded channels but with less efficient aggregation patterns.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit regression tests on synthetic patches with known RoG values.

### Recommended Action
- `Accept`

### Notes
- Strong parity with likely robustness improvement.

## raise_walls

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/raise_walls.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both raise DEM cells along wall vectors, apply diagonal-thickening safeguards, and optionally breach selected crossings by restoring DEM values.
- Important differences: NG includes alignment safeguards and generalized line/polygon boundary rasterization paths; legacy enforces polyline-only wall inputs.
- Correctness note: Core wall-raising and breach semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are rasterization plus full-grid update passes with similar complexity.
- Tuning opportunities: Introduce optional sparse write masks for very large DEMs with short wall networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document polygon-boundary handling explicitly for compatibility expectations.

### Recommended Action
- `Accept`

### Notes
- Good parity.

## random_field

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_field.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate random raster fields with configurable distribution parameters.
- Important differences: NG also exposes an FFT-based autocorrelated variant while preserving basic random-field behavior.
- Correctness note: Core random-field generation semantics align for base mode.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker is green; NG uses modern random-generation and raster write paths with lower coordination overhead.
- Tuning opportunities: Benchmark autocorrelated mode separately from simple RNG mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document distinctions between base random field and autocorrelated variant.

### Recommended Action
- `Accept`

### Notes
- Tested parity is strong for baseline behavior.

## random_forest_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both perform supervised RF classification from predictor rasters and vector training data with the same core fit-and-predict objective.
- Important differences: Legacy includes richer in-tool diagnostics/report controls (test-proportion metrics and importance reporting), while NG currently emphasizes the core classification path.
- Correctness note: Core classification behavior is present; remaining differences are primarily model-evaluation/report contract depth.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most time in sample extraction and row-wise prediction; NG uses modern stack alignment helpers and batched row prediction.
- Tuning opportunities: Add optional lightweight diagnostics without duplicating full expensive report generation.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional diagnostics mode for test-split metrics and variable-importance reporting without changing the fast default path.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Treat as a contract-depth follow-up rather than a core algorithm redesign.

## random_forest_classification_fit

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_classification_fit.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG fit now trains and serializes a concrete random-forest classifier model at fit-time, matching the legacy fit/predict contract shape.
- Important differences: NG writes a versioned v2 model bundle (including scaling metadata/scalers) and keeps backward compatibility by allowing legacy v1 training-payload models to be consumed in predict.
- Correctness note: The prior deferred-training contract mismatch is resolved for new fit outputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now performs full fit-time training plus model serialization, similar to legacy contract expectations.
- Tuning opportunities: Benchmark model serialization overhead and consider optional compression for very large forests.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional model metadata diagnostics (feature count, tree count, training sample count) in a lightweight report field.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Contract-level parity blocker has been closed; remaining work is benchmark/compatibility verification.

## random_forest_classification_predict

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_classification_predict.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG predict now deserializes and applies a pre-trained classifier model directly, matching legacy predict semantics.
- Important differences: NG accepts the new v2 serialized model bundle and also supports legacy v1 payload-based models via fallback reconstruction for backward compatibility.
- Correctness note: The primary contract divergence (predict-time refit) is resolved for v2 model bytes.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now avoids refit for v2 model bytes and performs direct deserialize-and-predict, consistent with legacy workflow shape.
- Tuning opportunities: Benchmark v2 direct-deserialize path separately from legacy-v1 fallback mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Consider deprecating legacy-v1 fallback in a future major version once migration window closes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-impact contract mismatch addressed; keep compatibility and throughput checks on the follow-up list.

## random_forest_regression

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_regression.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both perform RF regression from predictor rasters and point targets with aligned core surface-prediction intent.
- Important differences: Legacy provides richer in-tool diagnostics and agreement reporting, while NG currently prioritizes streamlined fit/predict output generation.
- Correctness note: Core regression behavior is present; remaining divergence is mainly report/diagnostic contract scope.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by sample extraction plus row-wise model prediction; NG uses aligned-stack helpers and batched prediction loops.
- Tuning opportunities: Add optional diagnostics mode that can be disabled for throughput-sensitive runs.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-style diagnostics/concordance reporting as a non-default mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Keep this tracked as diagnostics parity work, not a core kernel blocker.

## random_forest_regression_fit

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_regression_fit.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG fit now trains and serializes a concrete random-forest regressor model at fit-time, aligning with legacy fit/predict contract intent.
- Important differences: NG emits a versioned v2 model bundle with scaling metadata/scalers and preserves legacy-v1 payload compatibility through predict fallback.
- Correctness note: The deferred-training payload contract gap is closed for new fit outputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now performs actual fit-time model training and model serialization, consistent with legacy fit-stage semantics.
- Tuning opportunities: Measure model bundle size/write cost for larger forests and tune optional compression if needed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional fit diagnostics output (sample counts/field summary) without changing serialized-model contract.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Contract mismatch has been resolved with the same v2-model approach used for classification.

## random_forest_regression_predict

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_forest_regression_predict.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: NG predict now deserializes and applies trained regressor model bytes directly, matching legacy predict-stage model application semantics.
- Important differences: NG supports v2 model bundles and retains v1 payload fallback reconstruction for backward compatibility.
- Correctness note: Predict-time retraining is removed for v2 models, closing the main parity blocker.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: NG now uses direct deserialize-and-predict for v2 models; extra retraining path is restricted to legacy-v1 compatibility fallback only.
- Tuning opportunities: Benchmark v2 path and quantify any remaining overhead from bundle decode compared with legacy.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add an explicit warning/info flag when legacy-v1 fallback reconstruction is used.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-impact contract divergence addressed; remaining work is compatibility/performance confirmation.

## random_sample

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/random_sample.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithNGCorrection`
- Summary: Both create a random, unique sample of valid raster cells and assign sequential sample IDs over a zero background.
- Important differences: NG validates against the true valid-cell count up front; legacy only checks total grid size and may early-stop with a warning if nodata density is high.
- Correctness note: NG behavior is stricter and more deterministic for invalid sampling requests.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Legacy uses repeated random probing with retry limit; NG builds valid-index pool once (parallel) then shuffles and takes first `n`.
- Tuning opportunities: For very large rasters, consider streaming reservoir sampling to reduce index-memory footprint.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document NG prevalidation behavior versus legacy warning-and-break behavior.

### Recommended Action
- `Accept`

### Notes
- Strong parity with a robustness improvement.

## range_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/range_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute local neighborhood range as `(max - min)` over the moving window.
- Important differences: NG routes through shared window-stats filter dispatch.
- Correctness note: Range-filter semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker is green and NG benefits from consolidated sliding-window implementation.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## raster_area

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/raster_area.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both compute per-class area totals and map class totals back to class cells with optional zero-background exclusion.
- Important differences: Legacy adjusts map-unit area for geographic rasters using geodesic cell-size estimation; NG currently uses constant `cell_size_x * cell_size_y` map-unit area.
- Correctness note: Projected-CRS behavior aligns; geographic-CRS area semantics diverge.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize class-area accumulation then perform full-grid remap/output pass.
- Tuning opportunities: Add optional geodesic-area path in NG for geographic CRS parity.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Introduce CRS-aware map-unit area handling to match legacy geographic behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity for projected data; geographic parity needs follow-up.

## raster_calculator

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/raster_calculator.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both evaluate user expressions over aligned raster stacks with support for row/column/extent context variables.
- Important differences: Legacy uses `fasteval` expression syntax and dynamic integer/float output type inference; NG uses `evalexpr`-style parsing with normalized conditionals and emits F32 output.
- Correctness note: Core cellwise expression workflow is aligned, but expression-language and output-typing contracts are not identical.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize row/cell evaluation and write full output grids in a second pass.
- Tuning opportunities: Add optional output-type inference mode for stricter legacy compatibility.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Provide compatibility mode for legacy expression quirks and integer-preserving output typing.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High utility parity, moderate contract drift.

## raster_cell_assignment

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/raster_cell_assignment.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both generate a raster assigning either row, column, x, or y value per cell from a base raster geometry.
- Important differences: NG validates assignment keyword explicitly and routes through shared raster-output helpers.
- Correctness note: Assignment semantics are aligned across all four assignment modes.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both compute per-cell assignments in parallelized row/cell loops and then write row-major output.
- Tuning opportunities: Minor memory-write batching only.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## raster_histogram

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/raster_histogram.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both build raster-value histograms and NG now supports optional legacy-style HTML report output along with adaptive default bin selection.
- Important differences: NG remains API-first (JSON `report` + optional `report_html`) while legacy is HTML/open-first by default.
- Correctness note: Core frequency counting and reportability are aligned; remaining differences are output default/UX behavior.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG avoids HTML rendering and browser-launch overhead, using parallel fold/reduce bin accumulation only.
- Tuning opportunities: Add optional legacy default-bin policy and categorical behavior mode.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional follow-up for categorical-bin behavior detection parity if strict categorical workflows require exact legacy bin semantics.

### Recommended Action
- `Accept`

### Notes
- Contract parity gap is materially closed.

## raster_perimeter

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/raster_perimeter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both use the same 8-neighbour LUT anti-aliasing approach to estimate class perimeter and map class totals back to cells.
- Important differences: Legacy includes geographic CRS distance scaling (vincenty/haversine) for map-unit output; NG currently applies a fixed average cell-size scale.
- Correctness note: Projected-CRS parity is strong; geographic map-unit perimeter semantics diverge.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are parallel class-bin accumulation passes using LUT pattern indexing plus full-grid remap/output.
- Tuning opportunities: Add optional geodesic scale computation path for geographic parity mode.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Restore geographic map-unit perimeter handling parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Excellent algorithmic parity except CRS-specific scaling behavior.

## raster_streams_to_vector

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/raster_streams_to_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both trace stream raster cells with D8 pointers into vector stream polylines.
- Important differences: NG uses a shared stream-tool implementation macro and unified registry plumbing.
- Correctness note: Stream-tracing semantics and output intent align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker is green; NG uses streamlined stream-network path construction in shared module infrastructure.
- Tuning opportunities: Benchmark very dense stream grids with many short segments.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted regression fixtures around confluence tracing and pointer edge cases.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## raster_summary_stats

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/raster_summary_stats.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute core raster distribution summaries over valid cells (count/min/max/mean/spread/sum family).
- Important differences: Legacy returns human-formatted text including nodata count/range/variance; NG returns compact JSON report and currently omits explicit nodata-count and variance/range fields.
- Correctness note: Core numeric aggregates align; report payload content differs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize accumulation; NG avoids extra formatting/report string overhead and keeps a compact reduction path.
- Tuning opportunities: Optional extended-report mode can preserve speed by toggling extra fields only when requested.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style expanded report fields (nodata count, range, variance).

### Recommended Action
- `Accept`

### Notes
- Good mathematical parity with lighter NG output contract.

## raster_to_vector_lines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/raster_to_vector_lines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both trace non-zero/non-nodata raster line cells into polyline features using endpoint-seeded traversal plus a second closed-loop pass.
- Important differences: NG uses generic vector output abstractions and in-memory output fallback, while preserving the same tracing queue logic and `VALUE` attribute behavior.
- Correctness note: Core tracing/branch-queue semantics match legacy implementation closely.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are serial graph-tracing passes over active cells with similar queue and neighbour-degree heuristics.
- Tuning opportunities: Spatial chunk partitioning for very large sparse rasters could improve throughput if needed.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- One of the strongest strict-algorithm parity matches in this wave.

## raster_to_vector_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/raster_to_vector_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both convert non-zero, non-nodata raster cells to point features at cell centers with `FID` and `VALUE` attributes.
- Important differences: NG uses parallel row collection and modern vector output plumbing; legacy emits directly through shapefile APIs.
- Correctness note: Inclusion criteria and attribute semantics match.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy performs serial row/column scans; NG parallelizes per-row candidate extraction before sequential write-out.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity item.

## raster_to_vector_polygons

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/raster_to_vector_polygons.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both clump non-zero categorical raster regions and trace polygon boundaries with ring orientation handling and `VALUE` attributes.
- Important differences: NG uses modern geometry/ring normalization and in-memory/output-path abstractions; legacy uses direct shapefile geometry assembly with similar tracing heuristics.
- Correctness note: Core polygonization behavior is aligned for typical classified rasters.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both follow comparable phases (clump, edge extraction, ring tracing, feature build) with similar complexity.
- Tuning opportunities: Optional parallelization for ring tracing on very large clump counts.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for complex hole assignment edge cases.

### Recommended Action
- `Accept`

### Notes
- Strong parity with modernized output handling.

## rasterize_streams

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/rasterize_streams.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both rasterize stream polylines to a base raster with optional `use_feature_id` and configurable background behavior.
- Important differences: Legacy reports stream collisions/adjacencies and exposes additional console diagnostics; NG implements streamlined rasterization through shared segment routines without those diagnostics.
- Correctness note: Main rasterized-stream output semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are primarily segment-rasterization loops over stream geometries; legacy adds extra post-raster diagnostic passes.
- Tuning opportunities: Optional collision/adjacency metrics mode in NG when users need legacy diagnostics.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style diagnostics output (collisions, adjacencies, stream-cell counts).

### Recommended Action
- `Accept`

### Notes
- Functional rasterization parity is good.

## reciprocal

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/reciprocal.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-cell reciprocal (`1/x`) with nodata propagation.
- Important differences: NG is implemented through shared unary-math dispatch.
- Correctness note: Transform semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker benchmarks show a major NG speedup with equivalent unary-kernel behavior.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## reclass

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/reclass.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both support range-based reclassification and assign-mode pair mapping for exact value remaps.
- Important differences: Both retain the 10,000-scaling hash strategy for assign-mode floating value keys; NG formalizes argument parsing/validation and output plumbing.
- Correctness note: Rule-application semantics match legacy behavior.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize raster traversal and apply either rule-scan or hashmap lookups per cell.
- Tuning opportunities: Pre-sort interval rules and binary search for large rule tables.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence semantic parity.

## reclass_equal_interval

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/reclass_equal_interval.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both reclassify values into equal-width bins over an optional start/end range while preserving values outside the range.
- Important differences: NG uses explicit `min_max_valid` fallback and shared output helpers; legacy uses raster cached min/max update routines.
- Correctness note: Interval binning behavior matches.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform one parallel per-cell mapping pass with simple interval math.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Straightforward parity case.

## recover_flightline_info

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/recover_flightline_info.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithNGCorrection`
- Summary: Both infer flightlines from sorted GPS-time gaps and optionally write IDs to point source ID, user data, and RGB.
- Important differences: NG remains open-tier and uses modulo 256 for user-data packing; legacy is Pro-gated and uses `%255` in user-data assignment.
- Correctness note: NG matches documented byte-range intent more closely while preserving overall workflow semantics.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by sort-by-time plus single pass ID/color assignment.
- Tuning opportunities: Optional stable-sort mode for strict deterministic tie handling across platforms.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document user-data wrap behavior explicitly for compatibility expectations.

### Recommended Action
- `Accept`

### Notes
- Good parity with a subtle correction.

## rectangular_grid_from_raster_base

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/rectangular_grid_from_raster_base.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both generate clipped rectangular polygon grids over raster extents using width/height and optional origin alignment.
- Important differences: NG validates finite origin values and writes through unified vector layer abstractions.
- Correctness note: Geometry construction and row/column attribute semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both enumerate grid cells over extent ranges and emit one polygon per cell; NG parallelizes feature materialization.
- Tuning opportunities: Chunked writes for extremely large grids.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## rectangular_grid_from_vector_base

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/rectangular_grid_from_vector_base.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both create rectangular grids over vector-layer extents with configurable width/height and optional origin alignment.
- Important differences: NG adds stronger validation and uses unified vector I/O APIs.
- Correctness note: Output grid geometry and attribute semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both follow extent-to-cell iteration with comparable complexity; NG parallelizes feature construction.
- Tuning opportunities: Optional memory-aware batching for massive grids.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## reinitialize_attribute_table

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/reinitialize_attribute_table.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both rebuild vector attributes so only regenerated sequential `FID` remains while preserving geometries.
- Important differences: Legacy mutates the in-memory input object directly; NG uses explicit input/output path semantics with optional overwrite when output is omitted.
- Correctness note: Table reinitialization semantics match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are linear per-feature copy/reset workflows; NG parallelizes geometry cloning before output append.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document overwrite behavior explicitly when output is omitted.

### Recommended Action
- `Accept`

### Notes
- Clean parity with improved workflow ergonomics in NG.

## related_circumscribing_circle

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/related_circumscribing_circle.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute related circumscribing circle as `1 - polygon_area / circumscribing_circle_area` and append `RC_CIRCLE`.
- Important differences: NG supports polygon and multipolygon geometry through unified geometry handling and parallel value computation.
- Correctness note: Formula and hole-area handling semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy iterates records serially; NG computes per-feature metrics in parallel and appends deterministically.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## relative_aspect

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/relative_aspect.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute angular difference between local aspect and a specified reference azimuth.
- Important differences: NG uses shared terrain-analysis helpers and streamlined argument plumbing.
- Correctness note: Relative-aspect semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker marks this green and code paths are structurally similar with lower NG orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## relative_stream_power_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/relative_stream_power_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/hydrologic_index_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute RSP as `sca^p * tan(slope_degrees)`.
- Important differences: NG explicitly skips non-positive `sca` values; legacy only filters nodata before computing.
- Correctness note: Behavior is aligned for typical positive-SCA hydrologic inputs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize row/band traversal; NG avoids thread-channel row exchange overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document NG non-positive-SCA handling as an explicit compatibility guard.

### Recommended Action
- `Accept`

### Notes
- Strong parity with a small input-domain guard difference.

## relative_topographic_position

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/relative_topographic_position.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute RTP piecewise relative to neighborhood min/mean/max over odd-sized filters.
- Important differences: NG uses integral images for neighborhood mean/count while still scanning windows for min/max.
- Correctness note: Output range/interpretation semantics align with legacy RTP.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are parallel row-wise neighborhood analyses; NG trades some repeated summations for integral-image lookup.
- Tuning opportunities: Sliding-window min/max acceleration could further improve NG.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## remove_duplicates

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/remove_duplicates.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both remove duplicate LiDAR points by x/y (and optional z) and now share all-members duplicate removal semantics.
- Important differences: NG uses direct coordinate-key counting while legacy uses fixed-radius neighbor search then equality checks.
- Correctness note: Duplicate-group retention/removal behavior is now aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Legacy builds fixed-radius index plus per-point neighborhood scans/channels; NG uses parallel hash-key dedup reduction.
- Tuning opportunities: Resolve parity semantics first, then benchmark.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional future switch for representative-retention mode if users want a non-legacy dedup variant.

### Recommended Action
- `Accept`

### Notes
- Highest semantic-risk behavior gap in this wave has been closed.

## remove_off_terrain_objects

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/remove_off_terrain_objects.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply white top-hat preprocessing and slope-constrained backfilling to suppress off-terrain objects.
- Important differences: NG reimplements core phases with modern buffers and helper routines; legacy includes more granular progress/reporting stages.
- Correctness note: Core OTO-removal workflow is preserved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize expensive neighbourhood phases and then perform region-growing style restoration.
- Tuning opportunities: Benchmark very large windows and optimize min/max window kernels if needed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted raster fixture tests for steep-edge threshold boundary behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good semantic alignment.

## remove_polygon_holes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/remove_polygon_holes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both remove interior polygon rings while preserving outer shells and feature attributes.
- Important differences: NG supports polygon/multipolygon through topology-safe geometry normalization helpers.
- Correctness note: Hole-removal behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Slight edge to NG.
- Evidence from code: Legacy is serial per record; NG parallelizes per-feature geometry stripping before deterministic output writes.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## remove_raster_polygon_holes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/remove_raster_polygon_holes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both identify enclosed background clumps (0/nodata), preserve edge-connected background, and fill interior holes subject to threshold/connectivity options.
- Important differences: Legacy is Pro-gated; NG is open-tier in current implementation.
- Correctness note: Core hole-removal semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both use flood-fill/clumping passes and nearest-foreground style fill assignment logic.
- Tuning opportunities: Parallelize clump labeling for very large rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Clarify license-tier change in migration notes to avoid user surprise.

### Recommended Action
- `Accept`

### Notes
- Functional parity is strong.

## remove_short_streams

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/remove_short_streams.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both remove short stream branches based on stream-network traversal and length thresholds.
- Important differences: NG is integrated into shared stream-network analysis infrastructure.
- Correctness note: Branch-pruning behavior aligns.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker benchmarks show NG faster with comparable traversal logic.
- Tuning opportunities: Additional profiling on dense branching networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep edge-case regression tests for branch-threshold and confluence handling.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## remove_spurs

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/remove_spurs.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both binarize foreground, then iteratively prune spur pixels via 8 directional neighbourhood patterns.
- Important differences: NG runs inside shared non-filter raster framework with explicit error-return writes.
- Correctness note: Pattern set and alternating scan strategy match legacy.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are intentionally sequential iterative morphology operations.
- Tuning opportunities: Optional tiling/bitmask acceleration could help large rasters.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## repair_stream_vector_topology

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/repair_stream_vector_topology.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both repair stream topology with endpoint snapping, confluence-consistent splitting, and dangling-arc correction under a snap tolerance.
- Important differences: NG uses a compact pipeline of helper stages (snap, merge degree-2, split intersections, fix dangling arcs) versus legacy’s detailed polyline/segment bookkeeping.
- Correctness note: Core repair intent and output type are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform multiple topology passes over line networks with spatial-index support.
- Tuning opportunities: Add stress tests on dense confluence networks for split/merge throughput.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures covering overshoot/undershoot edge cases from legacy examples.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Parity appears good, but topology edge-case tests are important.

## resample

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/resample.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both resample one-or-more rasters to base/cell-size output grids with `nn`/`bilinear`/`cc` modes and now use legacy-first stack precedence for overlaps.
- Important differences: NG remains integrated in shared remote-sensing infrastructure and API-first output plumbing.
- Correctness note: The material overlap-composition mismatch has been resolved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize output-cell sampling; major difference is stack precedence policy, not computational complexity.
- Tuning opportunities: Resolve precedence compatibility mode first, then benchmark interpolation kernels.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional explicit `stack_precedence` parameter can still be added later for user-controlled compositing modes.

### Recommended Action
- `Accept`

### Notes
- High-risk stack precedence mismatch has been closed.

## rescale_value_range

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/rescale_value_range.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both linearly rescale raster values into user-specified output ranges with optional clipping behavior.
- Important differences: NG executes via shared raster-stats tool infrastructure.
- Correctness note: Rescaling semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker benchmarks show a substantial NG speedup for the same transformation.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## rgb_to_ihs

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/rgb_to_ihs.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both convert RGB inputs (either separate bands or packed composite) into intensity, hue, and saturation outputs.
- Important differences: NG separates composite and per-band paths through dedicated helper functions and stores named outputs via tool result plumbing.
- Correctness note: IHS transform equations and input-mode semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize per-cell conversions; NG avoids thread-channel row transfers and writes from contiguous output vectors.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## rho8_flow_accum

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/rho8_flow_accum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute stochastic Rho8 flow accumulation from DEM/pointer-driven routing.
- Important differences: NG integrates the implementation with shared flow-algorithm helpers.
- Correctness note: Accumulation semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: Tracker shows clear NG speedup after flow-direction parallelization and output-stage fusion.
- Tuning opportunities: Minor tuning around randomization and pointer update hot paths.

### Design Improvements
- Severity: `Minor`
- Opportunity: Preserve regression fixtures for stochastic reproducibility settings.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## rho8_pointer

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/rho8_pointer.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/flow_algorithms/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate Rho8 stochastic flow-direction pointers with expected encoding options.
- Important differences: NG shares helpers with other flow-direction tools.
- Correctness note: Pointer semantics align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Legacy on current benchmark.
- Evidence from code: Tracker marks this yellow; NG remains slightly slower but close to parity after recent optimizations.
- Tuning opportunities: Continue reducing pointer output-stage overhead in NG.

### Design Improvements
- Severity: `Minor`
- Opportunity: Keep this as a benchmark follow-up anchor for flow-direction micro-optimizations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Semantics are strong; remaining gap is small performance delta.

## ring_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/ring_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute ring curvature from polynomial derivative-based curvature components.
- Important differences: NG uses shared pro-curvature dispatch and streaming-row memory handling.
- Correctness note: Ring-curvature semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker shows NG faster following the streaming-row curvature optimization path.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## river_centerlines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/river_centerlines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/pro_stream_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both derive river centerlines from a binary water raster using distance-transform skeletonization, cleanup, tracing, and endpoint connection.
- Important differences: NG factors operations into compact helper stages (`distance transform`, `thinning`, `segment merge`) while legacy uses a longer monolithic workflow.
- Correctness note: Core centerline extraction intent and controls (`min_length`, `search_radius`) are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by full-raster distance/skeleton passes and vector tracing; NG emphasizes simpler dataflow and reduced intermediate structures.
- Tuning opportunities: Benchmark very large river masks with many disconnected reaches.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for near-touching channel connections at `search_radius` boundary values.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence.

## roberts_cross_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/roberts_cross_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply Roberts Cross edge-detection kernels to raster inputs.
- Important differences: NG is hosted in shared convolution-extra filter infrastructure.
- Correctness note: Edge-response semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker benchmarks show meaningful NG speedup with equivalent convolution behavior.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## root_mean_square_error

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/root_mean_square_error.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute RMSE, mean vertical error, 95% confidence limit, and LE90 between rasters, and NG now applies bilinear sampling for mismatched grids.
- Important differences: NG uses bilinear sampling with partial-kernel nodata handling through `sample_world` infrastructure.
- Correctness note: The major interpolation-method mismatch is resolved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize cell comparisons and residual sorting; the primary gap is interpolation method, not throughput structure.
- Tuning opportunities: Restore bilinear compatibility mode first, then benchmark.

### Design Improvements
- Severity: `Minor`
- Opportunity: Optional future enhancement: expose explicit interpolation-method parameter for advanced control.

### Recommended Action
- `Accept`

### Notes
- Highest-risk interpolation mismatch in this wave has been closed.

## rotor

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/rotor.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute rotor curvature as a rotational terrain derivative measure from polynomial surface terms.
- Important differences: NG uses shared pro-curvature infrastructure and memory-streaming row workers.
- Correctness note: Rotor semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker shows green with NG faster after curvature pipeline updates.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## round

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/round.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both round each valid raster cell to the nearest integer value.
- Important differences: NG uses shared unary-math tool generation.
- Correctness note: Rounding semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Tracker benchmarks show very large NG speedup for this unary transform.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## ruggedness_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/ruggedness_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute ruggedness/TRI as RMS deviation between each cell and valid 8-neighbour elevations.
- Important differences: NG wraps execution in shared terrain-analysis scaffold; computational kernel mirrors legacy.
- Correctness note: TRI semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize row computations; NG writes row slices without channel receive loops.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## scharr_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/scharr_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools apply a Scharr edge-detection convolution to raster imagery and preserve nodata handling around invalid neighbors.
- Important differences: NG routes Scharr through a shared extra-convolution filter framework, while legacy uses a standalone tool implementation.
- Correctness note: Core filter-kernel behavior is aligned for standard Scharr use.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both implementations perform neighborhood convolution over full rasters with parallel row-style execution.
- Tuning opportunities: Benchmark very large kernels/images to confirm cache behavior parity.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a short doc note distinguishing Scharr vs Sobel gradient response characteristics.

### Recommended Action
- `Accept`

### Notes
- High-confidence semantic match.

## sediment_transport_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/sediment_transport_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/hydrologic_index_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute STI/LS using `(n+1) * (sca/22.13)^n * (sin(slope)/0.0896)^m` with user exponents.
- Important differences: NG explicitly skips non-positive `sca` and non-positive `sin` terms; legacy computes when values are non-nodata.
- Correctness note: Semantics align for standard positive-SCA and positive-slope terrain inputs.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both use parallel row traversal; NG uses simpler write-back pattern with reduced synchronization.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document non-positive-term handling for strict reproducibility expectations.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity for normal hydrologic domains.

## select_tiles_by_polygon

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/select_tiles_by_polygon.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both select/copy LiDAR tiles to an output directory based on polygon overlap using sampled tile footprint points.
- Important differences: NG uses prepared polygon geometry helpers and parallel file scanning/copying through shared LiDAR I/O utilities.
- Correctness note: Selection intent and sample-point overlap strategy align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy uses mutex-coordinated worker loops; NG uses rayon parallel iteration over tiles with reduced coordination overhead.
- Tuning opportunities: Add I/O throttling option for very large directory copies on spinning disks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose a `sample_mode` parameter if stricter bbox/polygon intersection modes are needed later.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## set_nodata_value

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/set_nodata_value.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both set raster nodata to `back_value` and convert existing nodata pixels to that value.
- Important differences: NG centralizes output dtype promotion logic in helper functions while preserving unsigned-to-signed promotion behavior for negative background values.
- Correctness note: Cell and metadata semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both parallelize raster remapping; NG uses parallel full-buffer fill without channel row collection.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## shadow_animation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/shadow_animation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate daily sun-shadow animations from DEM/DSM input and write HTML plus GIF outputs using date/time/location solar modeling.
- Important differences: NG validates/parses inputs in shared helpers and uses modern rendering/writer utilities, while legacy includes a more monolithic implementation path.
- Correctness note: Main animation semantics and user-facing controls are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by repeated horizon/shadow calculations and frame rendering over time steps.
- Tuning opportunities: Benchmark long frame sequences and optimize frame encoding throughput if needed.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression checks for exact frame count and timestamp labeling.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good functional parity confidence.

## shadow_image

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/shadow_image.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute terrain shadow rasters for specified date/time/location with optional palette-based hypsometric tinting.
- Important differences: NG centralizes date/time/location parsing and palette validation through shared sky-visibility utilities.
- Correctness note: Core shadow-image generation behavior aligns.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both combine horizon-angle style directional scans with raster rendering passes.
- Tuning opportunities: Benchmark large DEM cases and optimize directional-scan cache locality.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add fixture tests for AM/PM parsing edge-cases (e.g., 12:00AM, 12:00PM).

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity confidence.

## shape_complexity_index_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/shape_complexity_index_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute raster patch shape complexity from horizontal/vertical boundary transition counts normalized by patch span extents.
- Important differences: NG uses parallel fold/reduce accumulators for patch statistics and robust bin indexing helpers.
- Correctness note: Index definition and output semantics align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy performs multiple threaded passes with channel merges; NG computes transitions/statistics in parallel reductions and vectorized output assignment.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

- NG appears intentionally safer than legacy.

## shape_complexity_index_vector

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/shape_complexity_index_vector.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute per-polygon shape complexity via convex-hull-based form-factor metric (area-to-convex-hull ratio).
- Important differences: NG uses unified geometry helpers and parallelizes per-feature hull/complexity computation; legacy iterates records serially.
- Correctness note: Shape complexity metric and output attribute semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy processes features serially; NG parallelizes hull generation and complexity computation then appends deterministically.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong parity with favourable NG parallelization.

## shape_index

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/shape_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute terrain shape index from local surface derivatives/curvature terms.
- Important differences: NG uses shared curvature-tool scaffolding and centralized argument plumbing; legacy is a dedicated one-tool path.
- Correctness note: Shape-index intent and output meaning are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both process raster neighborhoods with comparable derivative workloads and parallelizable row loops.
- Tuning opportunities: Profile geographic-CRS runs where derivative branch logic may influence throughput.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add a focused regression fixture for flat/near-flat terrain edge cases to lock denominator handling.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Priority hint reflects historical Gaussian-adjacent performance context rather than a confirmed current regression.

## shreve_stream_magnitude

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/shreve_stream_magnitude.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute Shreve stream magnitude by accumulating upstream tributary contributions over a stream network.
- Important differences: NG integrates the tool into consolidated stream-network infrastructure and shared I/O utilities.
- Correctness note: Upstream accumulation intent and resulting magnitude field semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Mixed or near tie depending on network size/topology.
- Evidence from code: Both rely on topological flow-routing style traversals where dependency order constrains full parallelism.
- Tuning opportunities: Evaluate chunked frontier processing for large sparse basins.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add consistency checks for disconnected stream segments and pour-point boundary behavior.

### Recommended Action
- `Accept`

### Notes
- Sequential dependency structure naturally limits full parallel speedups.

## sigmoidal_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/sigmoidal_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply a sigmoidal contrast transform controlled by cutoff/gain style parameters.
- Important differences: NG uses consolidated non-filter remote-sensing tool plumbing and modernized parameter handling.
- Correctness note: Contrast mapping behavior and nodata treatment are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are per-cell transforms; NG benefits from shared optimized raster iteration patterns and reduced orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Straightforward parity case.

## sin

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/sin.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute sine for each valid raster cell and preserve nodata cells.
- Important differences: NG uses shared unary-math macro tooling.
- Correctness note: Core mathematical transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary transforms; NG avoids legacy-style channel coordination overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## singlepart_to_multipart

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/singlepart_to_multipart.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both group singlepart features into multipart geometries via optional field matching or global aggregation.
- Important differences: NG uses modern multipart-geometry construction and explicit duplicate-removal semantics; legacy uses record-level grouping and shapefile part coalescing.
- Correctness note: Main grouping intent and output type alignment are preserved.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform multi-pass grouping (collection + merge) with similar algorithmic complexity.
- Tuning opportunities: Parallelize per-group geometry construction in NG for large feature counts.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document duplicate-geometry behaviour under full aggregation mode.

### Recommended Action
- `Accept`

### Notes
- Practical parity is solid.

## sinh

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/sinh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute hyperbolic sine per valid raster cell with nodata preservation.
- Important differences: NG routes through the shared unary-math framework.
- Correctness note: Transform behavior matches for valid data.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are lightweight unary kernels; NG centralizes execution with lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## sink

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/sink.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both identify pit/depression cells by comparing original DEM to filled (depression-removed) surface, outputting depth raster.
- Important differences: NG delegates to shared fill-depressions core; legacy uses dedicated in-tool implementation.
- Correctness note: Depth calculation and pit-identification intent match.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by depression-fill operations (same underlying algorithm); secondary depth raster is parallel comparison.
- Tuning opportunities: Minimal beyond fill-depressions core tuning.

### Design Improvements
- Severity: `Minor`
- Opportunity: None required for parity.

### Recommended Action
- `Accept`

### Notes
- Strong hydrologic parity.

## sky_view_factor

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/sky_view_factor.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute sky view factor via multi-azimuth horizon visibility raytracing with optional weighting by zenith angle.
- Important differences: NG uses parallel rayon multi-azimuth loops and modern raytracing helpers; legacy uses threaded azimuth channels and explicit visibility accumulation.
- Correctness note: Core SVF metric semantics and raytracing intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG runs azimuths via rayon parallel work; legacy uses thread-channel coordination for azimuth workers.
- Tuning opportunities: Benchmark zenith-weighting accumulation on large DEMs; consider ray-tracing caching optimizations.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output statistics (e.g., per-azimuth visibility variance) for analysis workflows.

### Recommended Action
- `Accept`

### Notes
- Good parity with NG parallelization advantage.

## skyline_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `Pro-tier_legacy`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/skyline_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute horizon elevation angles for user-supplied observation points using azimuth-based raytracing and emit vector polygon + HTML outputs.
- Important differences: NG wraps horizon-angle computation in shared sky-visibility tools and uses modern vector output paths; legacy generates standalone polygon+report artifacts.
- Correctness note: Horizon-angle calculation and polygon vertex generation intents align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by per-point multi-azimuth visibility scanning; NG is serial per point, legacy is threaded.
- Tuning opportunities: Parallelize over observation-point batches for many input points.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional azimuth histogram output raster for visual analysis.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity with refactored output handling.

## slope

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/slope.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/basic_terrain_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute raster slope from local elevation gradients with projected/geographic handling branches.
- Important differences: NG implements slope within shared terrain-analysis infrastructure and centralized raster helpers.
- Correctness note: Output intent (slope magnitude from DEM neighborhood derivatives) is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both use neighborhood derivative kernels with similar data dependencies and parallel row workflows.
- Tuning opportunities: Benchmark high-resolution geographic DEMs where trigonometric corrections dominate compute time.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures that compare projected vs geographic branch consistency for known analytic surfaces.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Priority hint suggests low-complexity batch benchmarking candidate.

## slope_vs_aspect_plot

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `Pro-tier_legacy`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/slope_vs_aspect_plot.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both derive slope and aspect rasters, bin them into a 2D histogram, and emit an HTML radial/polar-plot visualization.
- Important differences: NG uses shared slope/aspect cores and HTML report writer; legacy includes dedicated aspect-binning histogram logic and legacy graph rendering.
- Correctness note: Binning behavior and plot intent are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize slope/aspect computation and binning; NG avoids custom graph infrastructure overhead.
- Tuning opportunities: Benchmark bin-count sensitivity and optional resolution parameter tuning.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional per-bin count statistics output alongside plot.

### Recommended Action
- `Accept`

### Notes
- Strong practical parity.

## slope_vs_elev_plot

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/slope_vs_elev_plot.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute elevation-binned mean slopes from single or multiple DEMs and generate HTML scatter/line-plot reports.
- Important differences: NG uses unified multi-DEM processing pipeline and modern HTML report framework; legacy uses per-DEM manual processing loop and older graph APIs.
- Correctness note: Core plot-generation logic and binning behavior align.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both parallelize slope computation and histogram accumulation over elevation bins.
- Tuning opportunities: Benchmark multi-DEM stacking memory overhead; consider streaming aggregation mode for very large stacks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional output of per-bin cell counts and variance estimates.

### Recommended Action
- `Accept`

### Notes
- Strong parity for multi-DEM analysis.

## smooth_vectors

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/smooth_vectors.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply moving-average coordinate smoothing to polyline/polygon geometries with configurable filter size and `preserve_endpoints` option.
- Important differences: NG supports multipart geometries and uses modern vertex manipulation helpers; legacy processes records serially and uses shapefile part APIs.
- Correctness note: Smoothing filter semantics and endpoint preservation behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Legacy processes geometry serially per record; NG parallelizes per-feature smoothing before deterministic output writes.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional Catmull-Rom spline mode alongside moving-average.

### Recommended Action
- `Accept`

### Notes
- Strong parity with improved parallelization.

## smooth_vegetation_residual

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: `Pro-tier_legacy`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/smooth_vegetation_residual.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute DEV (normalized elevation deviation) at multiple scales and suppress roughness by inverse-distance weighting from locally-smoothed seeds.
- Important differences: NG uses shared integral-image DEV computation and parallel per-scale IDW interpolation; legacy uses explicit multi-scale smoothing passes and per-cell IDW lookups.
- Correctness note: Core vegetation-suppression intent and smoothing behavior are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes DEV computation across scales and uses batched rayon IDW; legacy uses threaded channels for scale workers and more explicit per-cell computations.
- Tuning opportunities: Benchmark seed-density thresholds and optional neighbourhood-size adaptation for varying roughness levels.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional seed-distance weighting decay parameter for workflow tuning.

### Recommended Action
- `Accept`

### Notes
- Good parity with NG throughput advantage.

## snap_endnodes

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/snap_endnodes.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both snap polyline endpoints to nearby vertices via KD-tree within configurable tolerance, respecting user-provided snap-to-point sets optionally.
- Important differences: NG uses modern KD-tree wrappers and explicit endpoint-pair matching logic; legacy uses shapefile part endpoints and direct coordinate comparisons.
- Correctness note: Core snapping intent and node-merge semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG build KD-tree once and parallelizes per-feature endpoint snapping; legacy uses per-record endpoint search and has heavier update logic.
- Tuning opportunities: Benchmark high-density endpoint clusters and consider coalesced updates for many-to-one snaps.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional snap-distance summary output for QA validation.

### Recommended Action
- `Accept`

### Notes
- Good parity with modernized spatial-index approach.

## snap_pour_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/snap_pour_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both relocate input pour points to nearby high-flow cells using a user-defined search radius.
- Important differences: NG places the logic in consolidated hydrology infrastructure and includes adjacent compatibility wrappers (`jenson_snap_pour_points`) in the same module.
- Correctness note: The snapping objective and expected output geometry behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Mixed or near tie depending on point count and search radius.
- Evidence from code: Both combine neighborhood raster scans with point-wise relocation, where per-point locality dominates runtime.
- Tuning opportunities: Add optional localized max-flow index/cache for dense point sets.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add explicit tie-break policy documentation when multiple candidate cells share identical flow values.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Remains `not_tested` in tracker; benchmark confirmation is still pending.

## sobel_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/sobel_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both apply Sobel edge-detection convolution behavior with standard nodata-aware neighborhood handling.
- Important differences: NG uses a shared convolution-filter framework with enum-dispatched kernels.
- Correctness note: Sobel operator semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform regular stencil convolution over raster rows with parallel execution patterns.
- Tuning opportunities: Evaluate SIMD vectorization opportunities in the inner kernel loops.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Stable parity profile.

## sort_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/sort_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both sort LiDAR point records according to selected sort criteria and emit reordered point clouds.
- Important differences: NG uses consolidated LAS/LAZ I/O helpers and unified lidar tool plumbing within a shared module.
- Correctness note: Sort intent and output ordering objectives are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Mixed or near tie depending on dataset size and I/O backend.
- Evidence from code: Dominant cost is comparison sort plus point-cloud read/write; algorithmic complexity is similar between implementations.
- Tuning opportunities: Explore parallel sort thresholds for very large in-memory point sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose/clarify stable-vs-unstable sort policy in user-facing docs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Remains `not_tested` in tracker; practical runtime depends strongly on file size and storage speed.

## spherical_std_dev_of_normals

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/spherical_std_dev_of_normals.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute local spherical standard deviation of surface normals from DEM neighbourhoods.
- Important differences: NG is implemented in shared terrain-analysis infrastructure with consolidated argument and raster helpers.
- Correctness note: Metric intent and output interpretation are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform neighbourhood-derivative style raster passes with parallel row-oriented execution.
- Tuning opportunities: Profile large kernels for cache behavior and consider sliding-window derivative reuse.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for flat-terrain and high-relief edge cases.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tracker remains `not_tested`, so benchmark confirmation is still pending.

## split_colour_composite

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/split_colour_composite.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools split a packed colour composite raster into red, green, and blue outputs.
- Important differences: NG routes through shared non-filter remote-sensing utilities and standardized output handling.
- Correctness note: Channel extraction semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are linear per-cell channel-unpacking transforms and bandwidth-dominated.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Straightforward parity item.

## split_lidar

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/lidar_processing/split_lidar.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/lidar_processing/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools split LiDAR datasets into multiple outputs based on selected criteria/modes.
- Important differences: NG uses consolidated LiDAR I/O and batch-path helpers with modernized error propagation.
- Correctness note: Split intent and output partitioning behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by point-cloud read, partition, and write costs rather than heavy numeric kernels.
- Tuning opportunities: Add buffered writer tuning for many-output split workloads.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document deterministic output-file naming and ordering guarantees.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Runtime sensitivity is strongly tied to storage throughput.

## split_vector_lines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/split_vector_lines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools split polyline features into individual segment records with attribute carry-through.
- Important differences: NG uses generalized geometry handling and standardized vector write helpers.
- Correctness note: Segmentization behavior and output intent are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `NGImproved`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: NG parallelizes per-feature segment extraction before deterministic output assembly; legacy is primarily serial.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## split_with_lines

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/split_with_lines.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools split polygon features using line inputs and output subdivided polygons.
- Important differences: NG uses unified geometry and intersection helpers with explicit manifest-driven parameters.
- Correctness note: Core split objective and resulting feature semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by geometric intersection/splitting operations and topology repair overhead.
- Tuning opportunities: Benchmark dense cut-line datasets and optimize spatial prefiltering.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for features skipped due to non-intersection.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity appears strong but is dataset-shape sensitive.

## sqrt

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/sqrt.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both compute square root per valid raster cell.
- Important differences: Legacy guards negative inputs and maps invalid-domain values consistently, while NG applies direct floating-point `sqrt` behavior that can propagate NaN.
- Correctness note: For non-negative inputs, outputs align.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary transforms; NG uses shared framework with lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-compatible domain-guard mode for negative inputs.

### Recommended Action
- `DocumentedDeviation`

### Notes
- Domain handling is the key behavioral difference.

## square

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/square.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both square each valid raster cell and preserve nodata.
- Important differences: NG routes through the shared unary-math tool macro path.
- Correctness note: Numeric transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are embarrassingly parallel unary kernels; NG avoids row-channel coordination overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## standard_deviation_contrast_stretch

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/standard_deviation_contrast_stretch.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform contrast stretching around the mean using standard-deviation clipping controls.
- Important differences: NG is integrated into consolidated non-filter remote-sensing tooling.
- Correctness note: Stretch mapping behavior and nodata handling are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform global stats plus per-cell remapping in parallel row workflows.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Stable parity profile.

## standard_deviation_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/standard_deviation_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute moving-window standard deviation for raster cells.
- Important differences: NG uses a shared window-statistics filter framework with enum-dispatched kernels.
- Correctness note: Windowed variability metric semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both apply neighbourhood-stat kernels over full rasters with parallel row execution.
- Tuning opportunities: Sliding-window variance reuse can further reduce large-kernel cost.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add targeted tests for boundary/window-edge treatment parity.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good functional alignment.

## standard_deviation_of_slope

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/standard_deviation_of_slope.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute local standard deviation of slope over user-defined neighbourhoods.
- Important differences: NG uses shared terrain-window infrastructure and integral/stat helper pathways.
- Correctness note: Metric intent and output semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both require slope derivation plus neighbourhood variability analysis with parallel row kernels.
- Tuning opportunities: Reuse intermediate slope buffers across related terrain-window tools.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures over synthetic planes and rugged terrain for scale sensitivity checks.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Remains `not_tested` in tracker; benchmark confirmation is pending.

## standard_deviation_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/standard_deviation_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute local standard deviation from an input stack/collection context and write variability output rasters.
- Important differences: NG runs through consolidated GIS overlay infrastructure and shared parameter parsing.
- Correctness note: Overlay variability intent and output interpretation are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform neighbourhood/statistical accumulation with parallel row-style raster execution.
- Tuning opportunities: Profile large multi-input overlays for temporary-buffer reuse opportunities.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit docs clarifying behaviour for nodata-heavy overlap regions.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tracker marks this tool as `tested`; no immediate parity risk signals.

## stochastic_depression_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/stochastic_depression_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform stochastic perturbation/depression analysis workflows over DEMs for uncertainty-aware hydrologic interpretation.
- Important differences: NG uses modernized hydrology module plumbing and shared raster utilities.
- Correctness note: Core stochastic-depression intent appears aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by repeated DEM perturbation/fill-style passes and aggregate statistics.
- Tuning opportunities: Reuse random/perturbation buffers between iterations to reduce allocation churn.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add deterministic-seed reporting in outputs for exact reproducibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Still `not_tested` in tracker, so benchmark confirmation remains pending.

## strahler_order_basins

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/strahler_order_basins.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both delineate basin regions keyed to Strahler stream-order structure.
- Important differences: NG implementation is integrated with shared hydrology routing helpers.
- Correctness note: Basin-order assignment semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on D8/topological traversal stages with similar dependency constraints.
- Tuning opportunities: Parallelize independent headwater-group propagation where safe.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Good parity confidence.

## strahler_stream_order

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/strahler_stream_order.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute Strahler stream order from stream raster/topology inputs.
- Important differences: NG runs inside shared stream-network tool scaffolding.
- Correctness note: Order-computation semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are topological propagation workflows where dependency ordering limits full parallelism.
- Tuning opportunities: Focus on inflow-count preprocessing and memory locality.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Stable parity profile.

## stream_link_class

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/stream_link_class.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both classify stream links (e.g., source/interior/exterior categories) from network topology.
- Important differences: NG uses macro-generated shared stream-link infrastructure.
- Correctness note: Class assignment intent is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both derive link IDs then apply per-link classification passes with similar traversal complexity.
- Tuning opportunities: Cache per-link metadata across sibling stream-link tools.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional summary table of class counts in tool output metadata.

### Recommended Action
- `Accept`

### Notes
- Parity confidence is high.

## stream_link_identifier

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/stream_link_identifier.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both assign unique identifiers to each stream link segment.
- Important differences: NG routes through shared stream-link implementation macros and helpers.
- Correctness note: ID assignment semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on topology traversal and link segmentation with similar complexity.
- Tuning opportunities: Reuse computed link maps between identifier/class/length/slope companion tools.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## stream_link_length

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/stream_link_length.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute per-link stream length values and map outputs to stream cells.
- Important differences: NG uses shared link-analysis infrastructure and streamlined output handling.
- Correctness note: Length metric semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both accumulate along identified links with similar traversal and distance-calculation costs.
- Tuning opportunities: Cache distance-step arrays and link IDs when run in batch with related tools.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Clean parity case.

## stream_link_slope

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/stream_link_slope.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute average slope statistics per stream link.
- Important differences: NG is integrated with shared stream-link pipeline and helper methods.
- Correctness note: Link-slope intent and output meaning are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both require link delineation followed by per-link elevation-distance aggregation.
- Tuning opportunities: Batch reuse of elevation/link intermediates across stream-link family tools.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics for links skipped due to insufficient elevation variation.

### Recommended Action
- `Accept`

### Notes
- Good parity confidence.

## stream_slope_continuous

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/stream_slope_continuous.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute continuous stream-cell slope values along the network.
- Important differences: NG uses unified stream-network helper pipeline with modernized output handling.
- Correctness note: Continuous slope semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are traversal-heavy stream analyses with similar dependency constraints.
- Tuning opportunities: Improve cache locality in downstream-neighbour lookup loops.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document slope sign and unit conventions explicitly for interoperability.

### Recommended Action
- `Accept`

### Notes
- Strong practical parity.

## subbasins

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/subbasins.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both delineate subbasin labels from D8 flow direction and stream network structure.
- Important differences: NG uses consolidated hydrology module infrastructure and shared raster helpers.
- Correctness note: Subbasin delineation behavior is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on topological flow-routing style propagation with similar dependency-limited stages.
- Tuning opportunities: Parallelize independent outlet-seeded label propagation where correctness permits.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity for this hydrology core tool.

## subtract

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/subtract.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_add.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both perform cellwise raster subtraction with nodata-aware handling.
- Important differences: NG routes subtraction through a shared binary raster math framework.
- Correctness note: Arithmetic and nodata semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel binary per-cell transforms; NG avoids legacy channel orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## sum_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/sum_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute summed overlap statistics/values across input rasters.
- Important differences: NG uses consolidated overlay mode dispatch (`sum`) in a shared GIS overlay engine.
- Correctness note: Summation intent and output semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute parallel row-wise accumulation over aligned inputs with similar memory-access patterns.
- Tuning opportunities: Reuse input-window caches for large stack counts.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit docs for nodata handling when only a subset of layers are valid.

### Recommended Action
- `Accept`

### Notes
- Stable parity profile.

## surface_area_ratio

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/surface_area_ratio.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute local surface-area ratio metrics from DEM neighbourhood geometry.
- Important differences: NG uses shared terrain-analysis helper routines and modern argument plumbing.
- Correctness note: Metric meaning and output interpretation are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform neighbourhood derivative/area computations in parallel row workflows.
- Tuning opportunities: Optimize large-kernel edge handling and trig reuse.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for steep synthetic surfaces to lock numeric expectations.

### Recommended Action
- `Accept`

### Notes
- Good tested-parity confidence.

## svm_classification

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/svm_classification.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform supervised SVM-based raster classification from training data.
- Important differences: NG integrates with shared raster-stack/training extraction helpers and modern report/output handling.
- Correctness note: Core class-assignment intent appears aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most runtime in training-sample extraction and prediction passes; orchestration differs more than core complexity.
- Tuning opportunities: Benchmark high-band-count inputs and optimize training extraction parallelism.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics parity for confusion/accuracy reporting fields.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Still `not_tested` in tracker.

## svm_regression

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/svm_regression.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform SVM regression over raster predictors and emit continuous-value output rasters.
- Important differences: NG uses centralized model/training plumbing with modernized output contracts.
- Correctness note: Core regression intent is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Similar workload split across sample extraction, model fitting, and per-cell inference.
- Tuning opportunities: Benchmark large training sets and cache model artifacts for repeated inference workflows.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style regression diagnostics output mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Still `not_tested` in tracker.

## symmetrical_difference

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/symmetrical_difference.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute polygon symmetric difference between input layers.
- Important differences: NG uses generalized geometry/topology utilities with shared overlay plumbing.
- Correctness note: Set-operation semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by geometry intersection/splitting/topology assembly stages.
- Tuning opportunities: Improve candidate filtering with stronger envelope prechecks.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity.

## tan

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/tan.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute tangent per valid raster cell with nodata preserved.
- Important differences: NG is implemented via shared unary-math generation.
- Correctness note: Transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are parallel unary kernels; NG avoids thread-channel overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## tangential_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_PLAN_CURVATURE`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/tangential_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both compute tangential curvature from local surface derivatives.
- Important differences: NG dispatches through shared curvature-kernel infrastructure alongside companion curvature tools.
- Correctness note: Curvature meaning and units are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform neighbourhood derivative math in parallel row workflows with similar arithmetic intensity.
- Tuning opportunities: Benchmark geographic-coordinate branch behavior on large DEMs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add cross-check fixtures against plan/profile curvature companion outputs.

### Recommended Action
- `Accept`

### Notes
- Priority hint is low and inferred; no strong regression signal.

## tanh

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/tanh.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute hyperbolic tangent per valid raster cell with nodata preservation.
- Important differences: NG routes through the shared unary-math framework.
- Correctness note: Transform semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are lightweight unary transforms; NG has lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## thicken_raster_line

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/thicken_raster_line.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both iteratively thicken raster line features to close diagonal or narrow connectivity gaps.
- Important differences: NG is integrated into shared non-filter tooling and modernized raster write paths.
- Correctness note: Intended line-thickening behavior appears aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are iterative morphology-style passes where step dependency limits parallelism.
- Tuning opportunities: Use compact bitmask neighbourhood encoding to speed inner pattern checks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional iteration-count cap reporting for QA reproducibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Remains `not_tested` in tracker.

## time_in_daylight

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/time_in_daylight.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both estimate daylight-duration style visibility metrics from terrain and sun-position parameterization.
- Important differences: NG routes through consolidated sky-visibility infrastructure with shared validation/plumbing.
- Correctness note: Core daylight-time intent and output interpretation are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by horizon/visibility sampling over many azimuth/elevation evaluations.
- Tuning opportunities: Profile high angular-resolution runs and cache trig intermediates aggressively.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit docs for latitude/date parameter edge cases at high latitudes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tracker remains `not_tested`; benchmark confirmation pending.

## tin_interpolation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/tin_interpolation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform TIN-based interpolation from vector points to raster output.
- Important differences: NG integrates interpolation in shared GIS tooling with modern argument and output handling.
- Correctness note: Core interpolation semantics appear aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are triangulation plus raster sampling workloads with similar complexity drivers.
- Tuning opportunities: Benchmark large point clouds and optimize triangulation build reuse.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add tolerance-based regression fixtures for edge-of-hull behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Still `not_tested` in tracker.

## to_degrees

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/to_degrees.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both convert raster values from radians to degrees per valid cell.
- Important differences: NG uses shared unary-math macro generation.
- Correctness note: Conversion semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are lightweight unary transforms; NG avoids channel-based row transfer overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## to_radians

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/to_radians.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both convert raster values from degrees to radians per valid cell.
- Important differences: NG routes through shared unary-math infrastructure.
- Correctness note: Conversion semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are unary per-cell transforms; NG has lower orchestration overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## tophat_transform

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/tophat_transform.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both apply morphological top-hat transform variants over raster neighbourhood windows.
- Important differences: NG centralizes morphology in shared non-filter tooling and modern argument plumbing.
- Correctness note: Core top-hat behavior and output intent are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on erosion/dilation-style neighbourhood passes with comparable computational patterns.
- Tuning opportunities: Benchmark large kernel sizes and reduce temporary raster allocation churn.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document white-vs-black top-hat variant semantics with concise examples.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tracker remains `not_tested`.

## topo_render

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/topo_render.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate topographic render outputs from DEM-derived illumination/visibility style inputs.
- Important differences: NG implementation is integrated with shared sky-visibility and rendering helper paths.
- Correctness note: Visualization intent and principal controls are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are raster-wide rendering pipelines with similar per-cell shading arithmetic.
- Tuning opportunities: Cache expensive angular terms across output passes.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit notes on expected output differences under geographic CRS.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Remains `not_tested` in tracker.

## topographic_hachures

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/topographic_hachures.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/contour_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate topographic hachure line features and associated metrics from terrain surfaces.
- Important differences: NG uses shared contour/line construction helpers and modern vector schema output APIs.
- Correctness note: Hachure-generation intent and output class are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are geometry-generation heavy and depend on contour-like tracing plus attribute computation.
- Tuning opportunities: Improve line simplification and batching for dense relief terrains.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add fixtures validating hachure orientation consistency on synthetic slopes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Still `not_tested` in tracker.

## topographic_position_animation

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/topographic_position_animation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_window_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both generate animated topographic-position visualization outputs across scales.
- Important differences: NG uses modernized HTML/render helper scaffolding and standardized output plumbing.
- Correctness note: Animation intent and scale-sequence behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by repeated multiscale raster computations and frame/report assembly.
- Tuning opportunities: Cache per-scale intermediates across frames to reduce recomputation.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional frame-count/quality presets for predictable runtime.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tracker status remains `not_tested`.

## topological_breach_burn

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/topological_breach_burn.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both perform topological breach-and-burn style hydrologic enforcement for DEM conditioning.
- Important differences: NG uses consolidated hydrology helper routines and modernized path/output orchestration.
- Correctness note: Core conditioning intent and major controls are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are multi-stage hydrologic transforms combining traversal and raster update passes.
- Tuning opportunities: Benchmark large basins and optimize repeated neighbour traversals.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit diagnostics for conditioning decisions (breach vs burn counts).

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Tool remains `not_tested` in tracker.

## topological_stream_order

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/topological_stream_order.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both compute topological stream order based on stream-link relationships.
- Important differences: NG routes the tool through shared stream-network framework macros/helpers.
- Correctness note: Ordering semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are topology-traversal driven and constrained by dependency ordering.
- Tuning opportunities: Reuse link-index intermediates across stream-order companion tools.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong tested parity confidence.

## total_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_PLAN_CURVATURE|BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/total_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute total curvature from local second-derivative terrain terms and write raster output in map units.
- Important differences: NG routes through the shared curvature framework and common derivative helpers; legacy is a dedicated per-tool implementation.
- Correctness note: Core total-curvature formulation appears aligned for matching DEM/cell-size inputs.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both use row-parallel neighbourhood derivative evaluation; NG reduces duplication through shared kernels.
- Tuning opportunities: Re-test large DEMs with varied edge conditions to confirm cache behavior in shared-curvature path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit doc note on curvature unit conventions and edge-cell handling parity with legacy.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High confidence in semantic parity with implementation modernization.

## total_filter

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/total_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/window_stats_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute the neighbourhood total (sum) filter over raster windows.
- Important differences: NG provides the filter via a unified window-stats kernel; legacy keeps a dedicated implementation.
- Correctness note: Window-sum behavior and nodata-aware accumulation semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG shares optimized moving-window machinery across filters and includes focused tests for total-filter interior equivalence.
- Tuning opportunities: Validate very large kernels for memory reuse and temporary-buffer churn.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## trace_downslope_flowpaths

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/trace_downslope_flowpaths.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools trace downslope flowpaths from seed locations using D8 routing over a DEM/pointer representation.
- Important differences: NG integrates tracing with shared hydrology utility routines and standardized output plumbing.
- Correctness note: Path-following intent is aligned, with minor implementation differences in validation/error messaging.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by pointer-follow traversal and output marking; little high-leverage parallel work exists in core tracing.
- Tuning opportunities: Optimize repeated pointer dereference patterns for dense seed sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics on terminated paths (edge exit, pit, nodata) for QA parity workflows.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core tracing behavior appears consistent.

## travelling_salesman_problem

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/travelling_salesman_problem.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools solve point-visit route ordering with TSP-style heuristics and output route geometry/ordering metadata.
- Important differences: NG wraps solver behavior in modern vector abstractions and includes a basic benchmark hook in metadata.
- Correctness note: Route-construction intent is aligned, though heuristic tie behavior may vary slightly.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are primarily heuristic-search and distance-matrix dominated; parallelism impact is limited at common problem sizes.
- Tuning opportunities: Add optional nearest-neighbour pre-pruning for larger point sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document deterministic/reproducibility expectations when multiple equivalent next hops exist.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity appears good for standard route-design usage.

## trend_surface

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/trend_surface.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools fit polynomial trend surfaces to raster data and output fitted trend rasters with residual-analysis context.
- Important differences: NG uses consolidated raster-stats regression pathways and modern report/output handling.
- Correctness note: Core polynomial fit intent is aligned, with minor differences in reporting format and metadata phrasing.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by sample accumulation and linear-system solve stages rather than per-cell arithmetic.
- Tuning opportunities: Benchmark high-order polynomials for conditioning and solve-time stability.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add explicit diagnostics for ill-conditioned fits and rank-deficient designs.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence parity for common trend orders.

## trend_surface_vector_points

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/trend_surface_vector_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools fit polynomial trend surfaces from vector point samples and produce modeled surface outputs.
- Important differences: NG shares regression infrastructure with raster stats tools and unified I/O handling.
- Correctness note: Vector-point polynomial fitting intent and output semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both spend most runtime in design-matrix construction and solve routines, with limited parallel upside.
- Tuning opportunities: Add optional sampling/downweight controls for very large point sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Include optional coefficient table sidecar export for downstream reproducibility checks.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Functional parity appears strong.

## tributary_identifier

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/tributary_identifier.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools assign unique identifiers to tributary segments based on stream-network topology.
- Important differences: NG implements within the shared stream-network module and standardized output conventions.
- Correctness note: Tributary labelling semantics and expected ID propagation behavior are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on topological traversal and link bookkeeping where algorithmic complexity is comparable.
- Tuning opportunities: Evaluate large braided networks for queue/cache locality in NG path.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## truncate

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/truncate.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_unary_math.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools truncate each valid raster cell value toward zero.
- Important differences: NG uses macro-based unary math implementation shared across many transforms.
- Correctness note: Numeric truncation semantics are equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both are straightforward parallel unary transforms; NG avoids legacy row-channel overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Clear parity.

## turning_bands_simulation

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/turning_bands_simulation.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools generate stochastic continuous fields using turning-bands simulation principles.
- Important differences: NG routes simulation/report handling through shared raster-stats infrastructure and modern metadata output.
- Correctness note: Simulation family and parameter intent are aligned; stochastic realizations are expected to differ run-to-run.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are compute-heavy stochastic pipelines; runtime is driven by number of bands, raster size, and random-field accumulation passes.
- Tuning opportunities: Expose deterministic seed controls and benchmark scaling with band count.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document reproducibility expectations and seed handling explicitly.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence for method intent.

## two_sample_ks_test

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/two_sample_ks_test.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools perform two-sample Kolmogorov-Smirnov style distribution comparison for raster samples.
- Important differences: Legacy emphasizes rich HTML narrative/report output, while NG returns a streamlined structured report in shared stats output format.
- Correctness note: Core test intent appears aligned, but output contract and auxiliary diagnostics are not fully equivalent.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG emphasizes direct summary-stat path and avoids heavier legacy reporting/rendering overhead.
- Tuning opportunities: If richer report parity is needed, keep computational core decoupled from rendering to preserve fast-path throughput.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-style report mode with fuller diagnostic context while preserving current concise output.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Statistical-core parity looks good; reporting parity is the main gap.

## union

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/union.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute geometric union overlays between polygon layers and output merged topology with carried attributes.
- Important differences: NG uses shared topology/overlay helpers and robust unary-union options internally; legacy uses older in-tool overlay flow.
- Correctness note: Core polygon union intent is aligned, with minor differences expected in edge-case sliver handling and attribute conflict resolution.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are geometry-overlay dominated; NG benefits from modern helper kernels and cleaner adjacency handling.
- Tuning opportunities: Benchmark dense multi-part overlays and tune snapping/tolerance pathways for pathological boundaries.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document deterministic field-resolution policy when both inputs contain same-named attributes.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence parity for standard overlay workflows.

## unnest_basins

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/unnest_basins.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools derive nested basin structure by topological traversal of flow-direction and basin relationships.
- Important differences: NG is integrated into shared hydrology module conventions and standardized output routing.
- Correctness note: Basin nesting objective and resulting hierarchical IDs appear aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by graph-style traversal and label propagation over basin rasters.
- Tuning opportunities: Improve memory locality in parent-child queue processing for very large basin sets.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional hierarchy summary output (depth, child count) for QA and downstream analytics.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Parity appears strong for expected nested-basin delineation.

## unsharp_masking

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/unsharp_masking.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/phase3_filters.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools apply unsharp masking by subtracting/combining blurred detail components to enhance high-frequency contrast.
- Important differences: NG routes via shared filter kernels in phase3 remote-sensing tools; legacy is standalone.
- Correctness note: Sharpening behavior and key parameter semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both perform blur plus per-cell recombination and are largely memory-bandwidth bound.
- Tuning opportunities: Fuse blur/detail recombination where possible to reduce temporary buffer traffic.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong practical parity.

## unsphericity

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/unsphericity.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute unsphericity curvature from the same principal-curvature formulation (half-difference magnitude of principal curvatures) using local derivative kernels.
- Important differences: NG computes through the shared pro-curvature engine and argument plumbing; legacy uses a dedicated tool file and older worker orchestration.
- Correctness note: Core metric definition is aligned; remaining differences are implementation-shape and minor floating-order effects.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both execute as direct per-cell curvature kernels over local derivatives rather than multiscale repeated smoothing. NG now uses row-parallel rayon collection in the shared pro-curvature path and avoids channel handoff overhead.
- Tuning opportunities: Minor profiling only; no obvious extra-pass cleanup remains in this tool path.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add focused fixtures for border and nodata neighborhoods to lock in numeric tolerance expectations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Prior NGUsesExtraPasses label was stale for this tool path.

## update_nodata_cells

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/update_nodata_cells.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools fill/update nodata cells in a target raster from a source raster where valid donor values exist.
- Important differences: NG uses modern argument and output plumbing with shared raster utility helpers.
- Correctness note: Donor-replacement semantics for nodata cells are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Operation is a straightforward per-cell conditional replacement and NG avoids legacy-style channel overhead.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Clear parity and low risk.

## upslope_depression_storage

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/upslope_depression_storage.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools estimate upslope depression storage contribution over D8 flow structure.
- Important differences: NG is integrated with shared hydrology traversal and output conventions.
- Correctness note: Core storage-accumulation intent appears aligned, with minor messaging/diagnostic differences.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dependency-driven propagation workflows dominated by raster traversal.
- Tuning opportunities: Profile inflow-count and propagation queue locality on large terrains.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostic outputs for unresolved pits/depressions during propagation.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence for hydrologic storage mapping.

## user_defined_weights_filter

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/user_defined_weights_filter.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/convolution_extra_filters.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools apply user-supplied convolution kernels/weights to raster imagery.
- Important differences: NG centralizes convolution handling in shared filter modules and modern argument parsing.
- Correctness note: Core weighted-filter semantics are aligned when kernels are matched.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both run windowed convolution passes and are arithmetic/memory balanced with similar complexity.
- Tuning opportunities: Optimize large-kernel cache reuse and optional separable-kernel detection.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add kernel validation diagnostics (normalization/sum checks) for user-supplied weight files.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity is strong.

## vector_hex_binning

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/vector_hex_binning.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools aggregate vector points into hexagonal bins and output per-bin summary attributes.
- Important differences: NG uses modern vector backend abstractions and consolidated geometry/stat bookkeeping.
- Correctness note: Core hex-binning intent and statistics generation are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: NG.
- Evidence from code: NG emphasizes streamlined bin assignment and modern data structures for aggregation.
- Tuning opportunities: Evaluate memory footprint for dense, very large extents with high bin counts.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional legacy-style row/column index attributes if downstream users depend on lattice indexing.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong operational parity for common density mapping tasks.

## vector_lines_to_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/vector_lines_to_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools rasterize vector lines (and compatible geometries) onto a target raster grid using selected assignment semantics.
- Important differences: NG validates geometry type and uses shared rasterization helpers in the data-tools module.
- Correctness note: Line burn-in behavior and grid alignment intent are equivalent.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are scan-conversion dominated and bounded by geometry traversal plus cell write operations.
- Tuning opportunities: Improve segment clipping efficiency against tile windows for very large vector datasets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## vector_points_to_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/vector_points_to_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools rasterize point/multipoint vector inputs into grid cells using selectable assignment/statistics behavior.
- Important differences: NG uses modern geometry handling and stricter upfront input validation messaging.
- Correctness note: Point-to-cell assignment semantics are aligned for matching parameters.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by point iteration and cell accumulator updates with similar asymptotic complexity.
- Tuning opportunities: Add cache-friendly accumulator layouts for very high point densities.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional collision-report output to summarize multi-point-per-cell resolution behavior.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity for standard vector-to-raster conversion workflows.

## vector_polygons_to_raster

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/vector_polygons_to_raster.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/data_tools/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools rasterize polygon features onto a target grid using attribute or constant-value assignment modes.
- Important differences: NG uses shared data-tools rasterization helpers with tighter input validation and modern output plumbing.
- Correctness note: Polygon burn-in semantics and grid alignment behavior are aligned for matched parameters.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by polygon-edge scan conversion and interior fill assignment.
- Tuning opportunities: Improve scanline clipping and ring-bounds prefiltering for very complex multipart polygons.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional diagnostics on overwritten cells for overlapping polygons.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High-confidence parity for common polygon rasterization workflows.

## vector_stream_network_analysis

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/stream_network_analysis/vector_stream_network_analysis.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/stream_network_analysis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute vector stream-network attributes and topology-based summaries over polyline river networks.
- Important differences: NG integrates shared stream-network helper infrastructure and standardized output metadata.
- Correctness note: Core network-analysis intent appears aligned, with small differences likely in reporting granularity.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are graph/topology dominated with modest opportunities for parallel preprocessing.
- Tuning opportunities: Profile edge-adjacency construction on large braided networks.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional topology diagnostics export (node degree distribution, disconnected components).

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity confidence for mainstream stream-network analytics.

## vertical_excess_curvature

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: `LOW_INFERRED_FROM_GAUSSIAN|BATCH_SIMPLE_CANDIDATE_L2`
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/vertical_excess_curvature.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/pro_curvature_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute vertical-excess curvature from the shared principal-curvature/normal-curvature derivative family, with matching high-level metric intent.
- Important differences: NG runs through the shared pro-curvature engine and modern runtime plumbing; legacy uses a dedicated tool implementation.
- Correctness note: Core metric formulation is aligned; small numeric differences may come from implementation details rather than algorithm class.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both follow direct per-cell curvature evaluation over local derivatives in a single main kernel pass (plus output write), not a multiscale repeated-smoothing loop. NG now uses the same row-parallel shared kernel improvements as adjacent pro-curvature tools.
- Tuning opportunities: Minor profiling only; no clear low-risk pass-fusion target remains here.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add regression fixtures for extreme-slope and nodata-edge cells to stabilize tolerance expectations.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Prior NGUsesExtraPasses label was stale for this tool path.

## viewshed

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/viewshed.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute terrain visibility from one or more observer locations over a DEM.
- Important differences: NG uses shared terrain-analysis infrastructure and modern manifest/argument plumbing.
- Correctness note: Line-of-sight visibility intent is aligned; edge-case handling can differ with implementation details.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are sightline-heavy and cost scales with observer count and raster extent.
- Tuning opportunities: Add observer batching and angular-sector caching for multi-observer workloads.

### Design Improvements
- Severity: `Minor`
- Opportunity: Provide optional diagnostic rasters for visibility counts versus binary visibility.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Core viewshed parity appears strong.

## visibility_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/visibility_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/sky_visibility_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute visibility index style terrain openness/visibility measures across viewpoints.
- Important differences: NG implements through shared sky-visibility core components and modernized parameter handling.
- Correctness note: Visibility-index metric intent is aligned, with potential small differences from implementation refinements.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both remain computationally heavy due to repeated directional/line-of-sight evaluations.
- Tuning opportunities: Benchmark azimuth-sampling density and caching strategies on very large DEMs.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose explicit reproducibility controls for sampling orientation/step choices.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Good parity confidence with expected numeric tolerance differences.

## voronoi_diagram

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/voronoi_diagram.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools generate Voronoi polygons from point inputs and output planar partition cells.
- Important differences: NG uses shared geometry helper libraries and modern vector abstractions.
- Correctness note: Voronoi partitioning intent and output geometry family are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `NotApplicable`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by Voronoi computation and polygon assembly complexity.
- Tuning opportunities: Improve robustness/tolerance handling for near-collinear point sets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## watershed

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/watershed.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools delineate watershed basins from pour points using D8 flow routing over DEM-derived pointers.
- Important differences: NG integrates shared hydrology parsing and output conventions with modernized plumbing.
- Correctness note: Basin delineation behavior is aligned for matching inputs and pointer conventions.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both rely on pointer-following flood/label propagation where memory access patterns dominate runtime.
- Tuning opportunities: Optimize seed queue layout for very large pour-point sets.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- High-confidence hydrology parity.

## watershed_from_raster_pour_points

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/hydrology/watershed_from_raster_pour_points.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/hydrology/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools delineate watersheds using raster pour-point inputs and D8 flow routing.
- Important differences: NG uses shared hydrology module pathways and standardized output handling.
- Correctness note: Raster pour-point delineation semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both primarily perform pointer traversal and label propagation from pour-point seeds.
- Tuning opportunities: Improve cache locality in large raster seed expansion operations.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `AcceptAsIs`

### Notes
- Strong parity confidence.

## weighted_overlay

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/weighted_overlay.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute weighted overlay combinations of factor rasters into a composite suitability/output surface.
- Important differences: NG uses modernized parser and metadata/report pathways.
- Correctness note: Weighted combination intent and ranking semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are per-cell weighted arithmetic passes over aligned raster stacks.
- Tuning opportunities: Validate numeric stability/rounding policies under many-factor overlays.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expose optional normalization diagnostics for factor weights.

### Recommended Action
- `Accept`

### Notes
- Practical parity is strong.

## weighted_sum

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/gis/weighted_sum.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/gis/mod.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools compute weighted linear combination of multiple rasters into a single summed output surface.
- Important differences: NG routes through shared GIS raster-combination plumbing and modern output metadata handling.
- Correctness note: Core weighted-sum arithmetic semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are straightforward row-parallel stack arithmetic operations.
- Tuning opportunities: Add fast-path for contiguous f32 stacks where precision requirements allow.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- High-confidence parity.

## wetness_index

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/geomorphometry/wetness_index.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/geomorphometry/terrain_analysis_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute topographic wetness index style outputs from slope and upslope contributing area terms.
- Important differences: NG is integrated in shared terrain-analysis core with modern output/validation handling.
- Correctness note: Core wetness-index formulation intent is aligned for matched input conventions.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are raster-wide per-cell transforms with straightforward parallel row execution.
- Tuning opportunities: Add guardrails for near-zero slope handling to avoid unstable tails while keeping parity behavior explicit.

### Design Improvements
- Severity: `Minor`
- Opportunity: Document exact slope-unit expectations and numerical safeguards for low-relief cells.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Strong parity confidence for standard hydrologic terrain workflows.

## wilcoxon_signed_rank_test

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/wilcoxon_signed_rank_test.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `PartiallyDivergentAcceptable`
- Summary: Both tools compute Wilcoxon signed-rank style paired raster comparison statistics.
- Important differences: Legacy emphasizes richer HTML narrative/report output; NG uses streamlined structured report output in shared stats format.
- Correctness note: Core non-parametric paired-test intent is aligned, with divergence mostly in reporting contract.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `NGFusesMoreWork`
- Likely faster implementation: NG.
- Evidence from code: NG focuses on direct statistics computation and avoids heavier report rendering overhead.
- Tuning opportunities: If richer report parity is required, keep compute and rendering paths separated to preserve fast mode.

### Design Improvements
- Severity: `Moderate`
- Opportunity: Add optional legacy-style full HTML report mode with equivalent diagnostic detail.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Statistical core parity is good; output/report parity remains partial.

## write_function_memory_insertion

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/image_processing/write_function_memory_insertion.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools perform write-function memory insertion style multitemporal image composition emphasizing change signal channels.
- Important differences: NG uses shared non-filter remote-sensing infrastructure and includes direct unit tests for key multi-date behavior.
- Correctness note: Core channel-composition intent is aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: Near tie.
- Evidence from code: Both are raster-wide per-pixel transforms with similar arithmetic complexity.
- Tuning opportunities: Validate large-scene memory bandwidth utilization and opportunity for SIMD channel packing.

### Design Improvements
- Severity: `Minor`
- Opportunity: Expand documentation with canonical two-date and three-date usage examples to reduce parameter misuse.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- Practical parity appears strong.

## z_scores

- Audit date: 2026-05-10
- Tracker status: `tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/z_scores.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `Equivalent`
- Summary: Both tools standardize raster values to z-scores using global mean and standard deviation.
- Important differences: NG routes through shared raster-stats infrastructure with modern output metadata conventions.
- Correctness note: Standardization semantics are aligned.

### Performance Assessment
- Verdict: `NGLikelyFaster`
- Parallelization parity: `FullMatch`
- Loop-shape verdict: `LoopFusionParity`
- Likely faster implementation: NG.
- Evidence from code: Both perform aggregate-stat pass plus normalization pass; NG uses cleaner parallel buffer pathways.
- Tuning opportunities: Minimal.

### Design Improvements
- Severity: `None`
- Opportunity: None required.

### Recommended Action
- `Accept`

### Notes
- Strong parity confidence.

## zonal_statistics

- Audit date: 2026-05-10
- Tracker status: `not_tested`
- Priority hint: none
- Legacy path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/math/zonal_statistics.rs`
- NG path: `/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen/crates/wbtools_oss/src/tools/raster/raster_stats.rs`

### Semantic Assessment
- Verdict: `EquivalentWithMinorDifferences`
- Summary: Both tools compute zone-based summary statistics by aggregating values raster cells by categorical zone IDs.
- Important differences: NG emits streamlined structured outputs and uses consolidated raster-stats kernels.
- Correctness note: Core zonal aggregation semantics are aligned.

### Performance Assessment
- Verdict: `LikelyNearParity`
- Parallelization parity: `PartialMatch`
- Loop-shape verdict: `LoopStructureChanged`
- Likely faster implementation: Near tie.
- Evidence from code: Both are dominated by group-by aggregation and per-zone summary reduction.
- Tuning opportunities: Improve hashmap aggregation locality for very high zone-count rasters.

### Design Improvements
- Severity: `Minor`
- Opportunity: Add optional deterministic zone ordering and legacy-style table formatting mode.

### Recommended Action
- `ConfirmWithBenchmarkLater`

### Notes
- High practical parity for standard zonal analysis workflows.
