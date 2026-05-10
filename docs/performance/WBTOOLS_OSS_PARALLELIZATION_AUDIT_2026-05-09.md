# wbtools_oss Parallelization Audit (2026-05-09)

Goal: identify tools that are:
1. `not_tested` in parity tracker,
2. present in both legacy and NG,
3. parallelized in legacy,
4. not parallelized in NG.

Data sources:
- `docs/performance/tool_parity_tracker.csv`
- Legacy implementations: `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools`
- NG implementations: `crates/wbtools_oss/src/tools`

Method:
- Filter parity rows: `exists_legacy=TRUE`, `exists_next_gen=TRUE`, `test_status=not_tested`.
- Detect legacy parallelization using code patterns: `thread::spawn`, `num_cpus::get`, `mpsc::channel`, Rayon tokens.
- Detect NG parallelization by scanning each `impl Tool for ...` block tied to a tool `id: "..."`.
- Output machine candidate list to:
  - `docs/performance/audit_parallelization_gap_candidates_blockscan_v2.json`

## High-Confidence Confirmed Gaps

These were manually verified in code:

1. `convert_nodata_to_zero`
- Parity row: line 78 in tracker.
- Legacy parallel evidence:
  - `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/convert_nodata_to_zero.rs`
  - uses `num_cpus::get`, `mpsc::channel`, `thread::spawn`.
- NG non-parallel evidence:
  - `crates/wbtools_oss/src/tools/data_tools/mod.rs` (tool block starts near line 2920)
  - simple sequential loop over raster cells in `run`.

2. `modify_nodata_value`
- Parity row: line 383 in tracker.
- Legacy parallel evidence:
  - `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/modify_nodata_value.rs`
  - uses `num_cpus::get`, `mpsc::channel`, `thread::spawn`.
- NG non-parallel evidence:
  - `crates/wbtools_oss/src/tools/data_tools/mod.rs` (tool block starts near line 3007)
  - sequential loop over raster cells in `run`.

3. `set_nodata_value`
- Parity row: line 559 in tracker.
- Legacy parallel evidence:
  - `/Users/johnlindsay/Documents/programming/Rust/whitebox_workflows/src/tools/data_tools/set_nodata_value.rs`
  - uses `num_cpus::get`, `mpsc::channel`, `thread::spawn`.
- NG non-parallel evidence:
  - `crates/wbtools_oss/src/tools/data_tools/mod.rs` (tool block starts near line 3776)
  - sequential loop over raster cells in `run`.

## Optimization Progress

Batch 1 completed (2026-05-09):
- `convert_nodata_to_zero`: migrated hot cell loop to `Raster::par_fill_with`.
- `modify_nodata_value`: migrated hot cell loop to `Raster::par_fill_with`.
- `set_nodata_value`: migrated hot cell loop to `Raster::par_fill_with`.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batch 2 completed (2026-05-09):
- `anova`: migrated full-cell class/stat accumulation to a Rayon fold/reduce pass.
- `phi_coefficient`: migrated binary contingency accumulation loop to a Rayon fold/reduce pass.
- `root_mean_square_error`: migrated difference extraction loops (same-grid and reprojection-sampled paths) to Rayon parallel collection/reduction.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 3 completed (2026-05-09):
- `cross_tabulation`: migrated contingency-count accumulation to a Rayon fold/reduce pass.
- `kappa_index`: migrated confusion-matrix count accumulation to a Rayon fold/reduce pass.
- `image_regression`: migrated paired summary-stat and ANOVA residual/total sum-of-squares accumulation loops to Rayon fold/reduce passes.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 4 completed (2026-05-09):
- `image_correlation`: migrated per-raster mean/valid-count scans and pairwise covariance/deviation accumulation loops to Rayon reductions.
- Shared helpers `collect_valid_values` and `collect_paired_differences`: migrated to Rayon parallel filter-map collection, accelerating downstream tools (including `two_sample_ks_test`, `paired_sample_t_test`, and `wilcoxon_signed_rank_test`).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 5 completed (2026-05-09):
- `image_autocorrelation`: migrated valid-value mean/count scan and neighborhood-stat accumulation (`total_deviation`, `weights_sum`, `numerator`, `s2`, `k`) to Rayon reductions across rows/cells.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 6 completed (2026-05-09):
- `image_correlation_neighbourhood_analysis`: migrated local moving-window correlation and significance computation to a parallel all-cells pass (Rayon), with deterministic post-pass output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 7 completed (2026-05-09):
- `image_regression`: migrated residual raster generation to a parallel per-cell computation pass (Rayon) with deterministic final writes.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 8 completed (2026-05-09):
- `dbscan`: migrated per-band scaling-stat computation and valid-pixel feature extraction preprocessing to Rayon parallel passes (row-wise deterministic merge retained).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 9 completed (2026-05-09):
- `zonal_statistics`: migrated zone accumulation (value vectors + diversity sets), per-zone statistic computation, and output-cell value derivation to Rayon parallel passes.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 10 completed (2026-05-09):
- `raster_summary_stats`: migrated full-raster summary accumulation (`count`, `min`, `max`, `sum`, `sum2`) to Rayon fold/reduce.
- `raster_histogram`: switched valid-value collection to shared parallel helper and parallelized bin-count accumulation.
- `ks_normality_test`: switched valid-value collection to shared parallel helper and parallelized histogram-bin accumulation used by the K-S statistic.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 11 completed (2026-05-09):
- `mosaic` core helper (`run_mosaic`): migrated per-band per-cell sample selection to a parallel all-cells pass with deterministic sequential write-back.
- `resample` core helper (`run_resample`): migrated per-band per-cell sample selection to a parallel all-cells pass with deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 12 completed (2026-05-09):
- `canny_edge_detection`: migrated double-threshold classification (stage 4) to parallel vector fill and migrated final output-value derivation (stage 5, including hysteresis neighbour checks) to a parallel all-cells pass with deterministic write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 13 completed (2026-05-09):
- `k_means_clustering` core helper (`run_kmeans`): migrated iterative label-assignment and centroid-accumulation loop to Rayon reductions, and migrated final assignment/counting pass to Rayon reductions; deterministic sequential label write-back retained.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 14 completed (2026-05-09):
- `correct_vignetting` core helper (`run_correct_vignetting`): migrated unscaled intensity computation, min/max stat passes, and final per-cell output-value derivation to Rayon parallel passes with deterministic sequential raster writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 15 completed (2026-05-09):
- `panchromatic_sharpening` core helper (`run_panchromatic_sharpening`): migrated full per-cell pan/MS fusion computation to a Rayon parallel all-cells pass with deterministic sequential output writes for packed and multi-band modes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 16 completed (2026-05-09):
- `change_vector_analysis` core helper (`run_change_vector_analysis`): migrated full per-cell magnitude/direction computation to a Rayon parallel all-cells pass with deterministic sequential write-back to output rasters.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 17 completed (2026-05-09):
- `image_stack_profile` core helper (`run_image_stack_profile`): migrated profile extraction to a Rayon parallel per-point pass, computing each point's across-stack values independently.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 18 completed (2026-05-09):
- `write_function_memory_insertion` core helper (`run_write_function_memory_insertion`): migrated full per-cell RGB normalization and packed-value derivation to a Rayon parallel all-cells pass with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 19 completed (2026-05-09):
- Shared helper `band_min_max`: migrated full-raster valid-value min/max scan to a Rayon fold/reduce pass.
- `split_colour_composite` core helper (`run_split_colour_composite`): migrated per-cell packed-RGB unpacking to a Rayon parallel all-cells pass with deterministic sequential write-back to output bands.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 20 completed (2026-05-09):
- `rgb_to_ihs` conversion helpers: migrated `run_rgb_to_ihs_from_composite` and `run_rgb_to_ihs_from_bands` to Rayon parallel all-cells passes with deterministic sequential write-back to output bands.
- `ihs_to_rgb` conversion helper (`run_ihs_to_rgb`): migrated per-cell conversion to a Rayon parallel all-cells pass with deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 21 completed (2026-05-09):
- `create_colour_composite` core helper (`run_create_colour_composite`): migrated per-band min/max scans to Rayon fold/reduce passes and migrated per-cell packed-RGBA derivation to a Rayon parallel all-cells pass with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 22 completed (2026-05-09):
- `balance_contrast_enhancement` core helper (`run_balance_contrast_enhancement`): migrated packed-RGB statistics accumulation to a Rayon fold/reduce pass and migrated per-cell enhanced packed-value derivation to a Rayon parallel all-cells pass with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 23 completed (2026-05-09):
- `direct_decorrelation_stretch` core helper (`run_direct_decorrelation_stretch`): migrated stage-1 per-cell achromatic reduction plus histogram/sample accumulation to parallel fold/reduce, and migrated stage-2 per-cell stretch derivation to a Rayon parallel all-cells pass with deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 24 completed (2026-05-09):
- `flip_image` core helper (`run_flip`): migrated per-band per-cell source-coordinate remap to Rayon parallel all-cells passes with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 25 completed (2026-05-09):
- `generalize_classified_raster` core helper (`run_generalize_classified_raster`): migrated independent full-raster value-derivation passes (initial component-class output and nearest-method reassignment candidates) to Rayon parallel all-cells passes, with deterministic sequential output writes retained.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 26 completed (2026-05-09):
- `integral_image` core helper (`run_integral`): retained sequential integral-prefix construction per band for dependency correctness, and migrated independent per-cell output-value derivation/write-prep to a Rayon parallel all-cells pass with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 27 completed (2026-05-09):
- Shared helper `to_binary_raster`: migrated per-band per-cell binarization value derivation to Rayon parallel all-cells passes with deterministic sequential output writes, accelerating downstream binary morphology/thinning routines that depend on this conversion step.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 28 completed (2026-05-09):
- `otsu_thresholding` core helper (`run_otsu_thresholding`): migrated final per-row thresholded output derivation to a Rayon parallel row pass, retaining deterministic sequential row writes via ordered `set_row_slice`.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 29 completed (2026-05-09):
- `tophat_transform` core helper (`run_tophat_transform`): migrated per-band per-cell top-hat value derivation to Rayon parallel all-cells passes with deterministic sequential output writes, while preserving prior invalid-cell behavior.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 30 completed (2026-05-09):
- Shared helper `collect_valid_values`: migrated valid-value extraction to a Rayon parallel row pass with reduction-based flattening, accelerating downstream contrast-stretch and histogram-matching style helpers that consume per-band valid-value vectors.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 31 completed (2026-05-09):
- `corner_detection` core helper (`run_corner_detection`): migrated per-band per-cell neighborhood-pattern classification to Rayon parallel all-cells passes with deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 32 completed (2026-05-09):
- Shared helper `raster_to_rgba_image`: migrated per-pixel RGBA derivation to a Rayon parallel all-cells pass with deterministic sequential `put_pixel` writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 33 completed (2026-05-09):
- `histogram_equalization` core helper (`run_histogram_equalization`): migrated per-band valid-range counting and histogram accumulation to Rayon fold/reduce passes.
- `histogram_matching` core helper (`run_histogram_matching`): migrated per-band valid-range counting and histogram accumulation to Rayon fold/reduce passes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 34 completed (2026-05-09):
- `histogram_matching_two_images` core helper (`run_histogram_matching_two_images`): migrated reference-band valid-range counting and reference histogram accumulation to Rayon fold/reduce passes before CDF pair construction.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 35 completed (2026-05-09):
- `sigmoidal_contrast_stretch` core helper (`run_sigmoidal_contrast_stretch`): migrated valid-value min/max scan to a Rayon fold/reduce pass.
- `standard_deviation_contrast_stretch` core helper (`run_standard_deviation_contrast_stretch`): migrated valid-value mean and variance accumulations to Rayon parallel reductions.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 36 completed (2026-05-09):
- Shared helper `run_inplace_binary_op`: migrated both constant and raster second-operand data-path loops to Rayon parallel all-cells output derivation with deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 37 completed (2026-05-09):
- `cumulative_distribution` core helper (`run` in `CumulativeDistributionTool`): migrated valid-range/count scan, histogram accumulation, and final output value derivation to Rayon parallel passes with deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 38 completed (2026-05-09):
- `random_sample` core helper (`run` in `RandomSampleTool`): migrated valid-cell index extraction to a Rayon parallel filter-map pass and migrated F32 output zero-initialization to a Rayon parallel fill pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 39 completed (2026-05-09):
- `dbscan` core helper (`run` in `DbscanTool`): migrated per-band standardization statistics accumulation (`n`, `sum`, `sum_sq`) in the `band_scale` stage to Rayon fold/reduce passes.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 40 completed (2026-05-09):
- `random_field` core helper (`run` in `RandomFieldTool`): migrated the Gaussian spectral gain filtering pass from sequential nested row/column loops to a flat Rayon `par_iter_mut().enumerate()` cell pass while preserving the exact frequency-domain gain equation.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 41 completed (2026-05-09):
- `quantiles` core helper (`run` in `QuantilesTool`): migrated the valid-cell min/max/count scan and fixed-bin histogram accumulation to Rayon fold/reduce reductions, preserving existing binning and quantile-class assignment semantics.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 42 completed (2026-05-09):
- `quantiles` core helper (`run` in `QuantilesTool`): migrated per-cell quantile class derivation to a Rayon pass that computes output values in parallel, followed by deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 43 completed (2026-05-09):
- `max` core helper (`run` in `MaxTool`): migrated raster-raster and raster-constant per-cell value derivation to Rayon passes with preserved nodata rules and deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 44 completed (2026-05-09):
- `min` core helper (`run` in `MinTool`): migrated raster-raster and raster-constant per-cell value derivation to Rayon passes with preserved nodata rules and deterministic sequential write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 45 completed (2026-05-09):
- `list_unique_values` core helper (`run` in `ListUniqueValuesTool`): migrated feature-frequency aggregation to Rayon fold/reduce over feature rows, then converted to ordered output mapping for stable report serialization.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 46 completed (2026-05-09):
- `root_mean_square_error` core helper (`run` in `RootMeanSquareErrorTool`): migrated residual `sum`/`sq_sum` reductions and absolute-residual vector construction to Rayon parallel passes while preserving downstream metric formulas and report fields.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 47 completed (2026-05-09):
- shared polynomial regression helper (`fit_polynomial_surface`): migrated design-matrix row assembly to Rayon parallel chunk fills (`par_chunks_mut`) for independent sample rows.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 48 completed (2026-05-09):
- shared polynomial regression helper (`fit_polynomial_surface`): migrated residual and response moment accumulation (`ss_resid`, `z_sum`, `z_ss`) to Rayon parallel reduction while preserving existing $R^2$ computation.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 49 completed (2026-05-09):
- statistical test helpers: migrated paired-sample t-test moments (`sum`, `sq_sum`) and Wilcoxon signed non-zero pair extraction to Rayon parallel passes, preserving all downstream test formulas and report fields.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 50 completed (2026-05-09):
- `turning_bands_simulation` core helper (`run` in `TurningBandsSimulationTool`): migrated per-iteration 1D band convolution output generation (`y[j]`) from a sequential index loop to a Rayon parallel index-map pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 51 completed (2026-05-09):
- `turning_bands_simulation` core helper (`run` in `TurningBandsSimulationTool`): migrated per-iteration `y` series moment accumulation (`sum`, `sq_sum`) and z-score normalization pass to Rayon parallel reductions/updates.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 52 completed (2026-05-09):
- shared sampling helper (`sample_with_replacement`): migrated random draw generation to a Rayon parallel map over sample count for replacement sampling workloads used by multiple raster statistical tests.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 53 completed (2026-05-09):
- shared correlation helper (`pearson_from_pairs`): migrated sum/mean and covariance/variance accumulations to Rayon parallel zip-reductions, preserving the Pearson coefficient formula and guard conditions.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 54 completed (2026-05-09):
- shared rank-correlation helper (`kendall_tau_b_from_pairs`): migrated outer pairwise comparison accumulation to Rayon parallel reduction with thread-local concordant/discordant/tie counters.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 55 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated per-band statistics extraction (`mean`, `valid_count`) to a Rayon parallel pass over input rasters.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 56 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated correlation/covariance row scaling pass to a Rayon parallel row-wise transform over matrix rows.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 57 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated factor-loading matrix construction to a Rayon parallel row-wise pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 58 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated component-raster write-back from per-cell `set_f64` loops to typed F32 buffer fills using Rayon parallel assignment (with existing fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 59 completed (2026-05-09):
- `inverse_pca` core helper (`run` in `InversePcaTool`): migrated reconstructed image write-back from per-cell `set_f64` loops to typed F32 buffer fills using Rayon parallel assignment (with existing fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 60 completed (2026-05-09):
- `kappa_index` core helper (`run` in `KappaIndexTool`): migrated confusion matrix construction and row/column total derivations to Rayon parallel row/column passes while preserving class ordering and matrix semantics.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 61 completed (2026-05-09):
- `kappa_index` core helper (`run` in `KappaIndexTool`): migrated producer/user accuracy vector construction and diagonal accumulation to Rayon parallel index passes with unchanged metric formulas.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 62 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated sorted eigenvector remapping (`component_order` to eigenvector rows) to a Rayon parallel outer-index pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 63 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated explained-variance remapping to sorted component order using a Rayon parallel index pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 64 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated eigenvalue remapping to sorted component order using a Rayon parallel index pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 65 completed (2026-05-09):
- `trend_surface` core helper (`run` in `TrendSurfaceTool`): migrated fitted raster write-back from per-cell `set_f64` loops to typed F32 buffer fills using Rayon parallel assignment (with fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 66 completed (2026-05-09):
- `trend_surface_vector_points` core helper (`run` in `TrendSurfaceVectorPointsTool`): migrated fitted raster write-back from per-cell `set_f64` loops to typed F32 buffer fills using Rayon parallel assignment (with fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 67 completed (2026-05-09):
- statistical distribution helpers: migrated two-sample KS sample sorting (`two_sample_ks_statistic`) from sequential `sort_by` to Rayon `par_sort_by` for both sample vectors.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 68 completed (2026-05-09):
- rank-based helpers/tests: migrated rank preparation sort (`ranked_values`) and Wilcoxon signed-absolute sort (`WilcoxonSignedRankTestTool`) from sequential `sort_by` to Rayon `par_sort_by`.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 69 completed (2026-05-09):
- summary/statistics tools: migrated RMSE absolute-residual sorting (`RootMeanSquareErrorTool`) and zonal median sorting (`ZonalStatisticsTool`) from sequential `sort_by` to Rayon `par_sort_by`.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 70 completed (2026-05-09):
- `zonal_statistics` core helper (`run` in `ZonalStatisticsTool`): migrated zone-ID report sorting from sequential `sort_unstable` to Rayon `par_sort_unstable`.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 71 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated component-order ranking (descending explained variance) from iterative selection to a Rayon parallel index sort pass.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 72 completed (2026-05-09):
- `zonal_statistics` core helper (`run` in `ZonalStatisticsTool`): migrated output raster write-back from per-cell `set_f64` loop to typed F32 buffer fill using Rayon parallel assignment (with fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 73 completed (2026-05-09):
- `principal_component_analysis` core helper (`run` in `PrincipalComponentAnalysisTool`): migrated explained-variance setup reductions (`total_ev` and `explained` vector construction) to Rayon parallel iterators.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 74 completed (2026-05-09):
- `turning_bands_simulation` core helper (`run` in `TurningBandsSimulationTool`): migrated per-iteration random normal seed generation for the 1D band vector (`t[0..diagonal_size)`) from a sequential loop to a Rayon parallel fill with thread-local RNG state.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 75 completed (2026-05-09):
- neighbourhood correlation analysis helper (`run` in `ImageCorrelationNeighbourhoodAnalysisTool`): migrated dual-raster output write-back (`out_corr`, `out_sig`) from sequential per-cell `set_f64` loops to typed F32 parallel buffer assignment (fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 76 completed (2026-05-09):
- `attribute_scattergram` core helper (`run` in `AttributeScattergramTool`): migrated mean/covariance accumulations and x/y min-max reductions to Rayon parallel zip/reduce passes while preserving correlation/trendline formulas and report fields.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 77 completed (2026-05-09):
- `attribute_correlation` core helper (`run` in `AttributeCorrelationTool`): migrated pairwise field-correlation computation across independent field-index pairs to Rayon parallel passes, retaining deterministic sequential matrix assignment.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 78 completed (2026-05-09):
- `image_regression` core helper (`run` in `ImageRegressionTool`): migrated residual raster write-back from sequential per-cell `set_f64` loop to typed F32 parallel buffer assignment (fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 79 completed (2026-05-09):
- `attribute_correlation` core helper (`run` in `AttributeCorrelationTool`): replaced manual per-pair Pearson accumulation loops with the shared `pearson_from_pairs` helper (already parallelized), reducing duplicate sequential work inside each pair computation.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 80 completed (2026-05-09):
- `raster_calculator` core helper (`run` in `RasterCalculatorTool`): migrated row/column expression evaluation from a single shared sequential context loop to Rayon parallel per-row evaluation with row-local contexts, preserving existing nodata handling and deterministic sequential output write-back.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 81 completed (2026-05-09):
- `dbscan` core helper (`run` in `DbscanTool`): migrated output raster initialization/label write path to typed I16 buffer operations, including Rayon parallel nodata fill and direct cluster label assignment (fallback path preserved).

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 82 completed (2026-05-10):
- `network_od_cost_matrix` core helper (`run` in `NetworkOdCostMatrixTool`): migrated origin-wise OD row generation from a sequential loop with mutable cache/string appends to Rayon parallel per-origin row-block generation, with deterministic row-order restoration before final CSV write.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 83 completed (2026-05-10):
- `network_centrality_metrics` core helper (`run` in `NetworkCentralityMetricsTool`): migrated per-source shortest-path accumulation loop (`s in 0..n`) to a Rayon fold/reduce pattern with thread-local closeness/betweenness accumulators and deterministic reduction.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 84 completed (2026-05-10):
- `route_event_points_from_table` core helper (`run` in `RouteEventPointsFromTableTool`): migrated CSV row-wise route lookup/measure validation/point derivation and attribute assembly to a Rayon parallel row pass, followed by deterministic sequential feature materialization.
- `route_event_lines_from_table` core helper (`run` in `RouteEventLinesFromTableTool`): migrated CSV row-wise route lookup/measure-range validation/line slicing and attribute assembly to a Rayon parallel row pass, followed by deterministic sequential feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 85 completed (2026-05-10):
- `route_event_points_from_layer` core helper (`run` in `RouteEventPointsFromLayerTool`): migrated per-event route lookup/measure validation/point derivation and attribute assembly to a Rayon parallel event pass, followed by deterministic sequential feature materialization.
- `route_event_lines_from_layer` core helper (`run` in `RouteEventLinesFromLayerTool`): migrated per-event route lookup/measure-range validation/line slicing and attribute assembly to a Rayon parallel event pass, followed by deterministic sequential feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 86 completed (2026-05-10):
- `network_connected_components` core helper (`run` in `NetworkConnectedComponentsTool`): migrated per-line component lookup over extracted linework to a Rayon parallel pass, while preserving deterministic sequential first-assignment behavior when mapping component IDs back to source features.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 87 completed (2026-05-10):
- `network_node_degree` core helper (`run` in `NetworkNodeDegreeTool`): migrated per-node degree/type derivation from a sequential adjacency scan to a Rayon parallel adjacency pass, with deterministic sequential feature output preserved.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 88 completed (2026-05-10):
- `od_sensitivity_analysis` core helper (`run` in `OdSensitivityAnalysisTool`): migrated origin and destination point-to-network snapping passes to Rayon parallel scans when `parallel_execution=true`, preserving existing snap-distance filtering and downstream Monte Carlo behavior.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 89 completed (2026-05-10):
- `location_allocation_network` core helper (`run` in `LocationAllocationNetworkTool`): migrated demand-wise cost-matrix construction to Rayon parallel passes for both turn-aware and non-turn-aware routing paths, and migrated assignment-to-route geometry derivation to a Rayon parallel assignment pass with deterministic sequential output materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 90 completed (2026-05-10):
- `network_service_area` core helper (`run` in `NetworkServiceAreaTool`): migrated per-origin shortest-path cost-surface generation in polygon mode to a Rayon parallel pass (`dijkstra_all_costs` per origin), preserving deterministic downstream ring/geometry materialization and output write order.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 91 completed (2026-05-10):
- `network_service_area` core helper (`run` in `NetworkServiceAreaTool`): migrated edge-mode frontier segment derivation from a sequential adjacency scan to a Rayon parallel per-node pass that computes geometry/attribute payloads in parallel, with deterministic sequential feature materialization retained.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 92 completed (2026-05-10):
- `network_service_area` core helper (`run` in `NetworkServiceAreaTool`): migrated polygon-mode ring frontier/envelope accumulation loop from sequential adjacency scanning to a Rayon parallel per-node pass with deterministic ordered merge, preserving existing hull/ring construction behavior.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 93 completed (2026-05-10):
- `network_service_area` core helper (`run` in `NetworkServiceAreaTool`): migrated node-mode reachable-node feature payload derivation to a Rayon parallel pass over graph nodes, retaining deterministic sequential feature materialization and fid assignment.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 94 completed (2026-05-10):
- `network_routes_from_od` core helper (`run` in `NetworkRoutesFromOdTool`): migrated origin and destination point-to-network snapping from sequential loops to Rayon parallel scans, with deterministic post-sort of snapped node pairs before downstream route construction.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 95 completed (2026-05-10):
- `network_routes_from_od` core helper (`run` in `NetworkRoutesFromOdTool`): migrated final route payload preparation (including deterministic OD ordering and per-route payload staging) to Rayon parallel passes before sequential feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 96 completed (2026-05-10):
- `closest_facility_network` core helper (`run` in `ClosestFacilityNetworkTool`): migrated incident/facility point-to-network snapping loops to Rayon parallel scans with deterministic post-sort, and migrated final route payload staging to a Rayon parallel pass before sequential feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 97 completed (2026-05-10):
- `location_allocation_network` core helper (`run` in `LocationAllocationNetworkTool`): migrated final route payload staging (including demand/facility ordering and per-route payload serialization) to Rayon parallel pass with deterministic pre-sort before sequential feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 98 completed (2026-05-10):
- `raster_calculator` core helper (`run` in `RasterCalculatorTool`): migrated output materialization from sequential nested row/column loop to Rayon parallel flatten of row results into flat output vector.

Implementation file:
- `crates/wbtools_oss/src/tools/raster/raster_stats.rs`

Batch 99 completed (2026-05-10):
- `k_means_clustering` core helper (`run_kmeans`): migrated output class value collection from sequential loop over valid indices to Rayon parallel map/collect pass.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 100 completed (2026-05-10):
- `resample` core helper (`run_resample`): migrated output value filtering and collection from sequential loop to Rayon parallel filter-map/collect pass before sequential band-wise cell writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 101 completed (2026-05-10):
- `mosaic` core helper (`run_mosaic`): migrated output value filtering and collection from sequential loop to Rayon parallel filter-map/collect pass before sequential band-wise cell writes.

Implementation file:
- `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Batch 102 completed (2026-05-10):
- `polygon_short_axis` and `polygon_long_axis` core helpers (`run` in respective tools): migrated final feature collection from sequential conditional push to Rayon parallel-generated feature collection with deterministic FID-based sorting before output.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 103–106 completed (2026-05-10):
- `hexagonal_grid_from_raster_base`, `hexagonal_grid_from_vector_base`, `rectangular_grid_from_raster_base`, `rectangular_grid_from_vector_base`: migrated sequential feature construction loops from `into_iter().enumerate()` to Rayon `into_par_iter().enumerate()` with deterministic FID-based feature generation (fid = index + 1).

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 107–108 completed (2026-05-10):
- `medoid`: parallelized non-point-layer feature processing with `par_iter().map()` collecting Option<Feature>, deterministic sequential FID assignment for output-only features.
- `points_along_lines`: parallelized line-to-points generation with `par_iter().map()` collecting feature vectors per input, sequential deterministic FID assignment and attribute materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 109–110 completed (2026-05-10):
- `extract_nodes`: parallelized coordinate extraction from features with `par_iter().map()` collecting feature vectors per input, sequential FID assignment.
- `select_by_attributes_query`: parallelized expression evaluation with `par_iter().map()` collecting Option<Feature>, sequential FID assignment for selected features.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 111–113 completed (2026-05-10):
- `filter_by_area`: parallelized polygon area filtering with `par_iter().map()` collecting Option<Feature> based on area threshold, sequential FID assignment.
- `centroid_vector`: parallelized centroid calculation (point-layer path via `fold().reduce()` parallel aggregation; non-point-layer path via `par_iter().map()` collecting Option<Feature> with computed centroids).
- `voronoi_diagram`: parallelized point extraction with `par_iter().flat_map()` collecting coordinate-attribute pairs from point/multipoint geometries, then sequential voronoi diagram generation.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 114–115 completed (2026-05-10):
- `add_geometry_attributes`: parallelized per-feature geometry metric derivation (`area`, `length`, `perimeter`, centroid X/Y) with `par_iter().map()` collecting per-feature attribute value vectors, followed by deterministic sequential attribute append.
- `field_calculator`: parallelized expression-context setup and evaluation with `par_iter().map()` collecting `Result<FieldValue, ToolError>` per feature, followed by deterministic sequential target-field assignment/update.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 116–117 completed (2026-05-10):
- `dissolve`: parallelized per-feature polygon extraction with `par_iter().map()` collecting `(group_value, polygons)` tuples, followed by deterministic sequential grouping and dissolve output materialization.
- shared helper `collect_overlay_polygon_pieces`: parallelized feature-to-polygon-piece extraction with `par_iter().map()` and deterministic flattening; accelerates overlay workflows consuming this helper.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 118–119 completed (2026-05-10):
- `simplify_features`: parallelized per-feature geometry simplify transform with `par_iter().map()` collecting transformed geometries (`Result<Option<Geometry>, ToolError>`), followed by deterministic sequential geometry assignment.
- `densify_features`: parallelized per-feature densification transform with `par_iter().map()` collecting transformed geometries, followed by deterministic sequential geometry assignment.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 120–121 completed (2026-05-10):
- `add_point_coordinates_to_table`: parallelized per-feature point validation and attribute/coordinate extraction with `par_iter().map()` collecting prepared rows, followed by deterministic sequential output feature materialization.
- `clean_vector`: parallelized per-feature geometry cleaning and attribute extraction with `par_iter().map()` collecting prepared rows, followed by deterministic sequential output feature materialization.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batches 122–123 completed (2026-05-10):
- `fix_dangling_arcs`: parallelized per-feature output row preparation in both passthrough and snapped-output materialization paths (`par_iter().map()` collecting geometry/attribute rows), followed by deterministic sequential output writes.
- `topology_validation_report`: parallelized per-feature topology issue extraction with `par_iter().map()` collecting `(fid, geom_type, issues)` rows, followed by deterministic sequential CSV assembly.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batches 124–126 completed (2026-05-10):
- `lines_to_polygons`: parallelized per-feature polygon conversion and attribute extraction with `par_iter().map()` prepared rows, followed by deterministic sequential output feature writes.
- `reinitialize_attribute_table`: parallelized per-feature geometry cloning with `par_iter().map()` and retained deterministic sequential FID assignment/write order.
- `remove_polygon_holes`: parallelized per-feature topology-based hole stripping and attribute extraction with `par_iter().map()` prepared rows, followed by deterministic sequential output writes.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batches 127–129 completed (2026-05-10):
- `polygons_to_lines`: parallelized per-feature boundary extraction and attribute preparation with `par_iter().map()` prepared rows, followed by deterministic sequential output writes.
- `export_table_to_csv`: parallelized per-feature CSV row formatting with `par_iter().map()`, followed by deterministic sequential file writes.
- topology-rule helper builders (`build_indexed_polygon_features`, `build_indexed_line_features`, `collect_line_endpoint_records`): parallelized per-feature topology conversion/endpoint extraction with `par_iter().map()` and deterministic flattening.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batches 130–132 completed (2026-05-10):
- `topology_rule_validate` (`line_must_not_self_intersect` branch): parallelized per-feature self-intersection issue extraction with `par_iter().flat_map()` and deterministic merge into violation output.
- `topology_rule_validate` (`point_must_be_covered_by_line` branch): parallelized per-feature point coverage checks with `par_iter().map()` collecting optional violations, followed by deterministic merge.
- `topology_rule_autofix` (`point_must_be_covered_by_line` prep): parallelized line geometry collection with `par_iter().filter_map()` prior to sequential fix application.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batches 133–135 completed (2026-05-10):
- shared GIS helper `collect_feature_topo_geometries`: parallelized per-feature geometry-to-topology conversion with `par_iter().map()` and deterministic ordered collection.
- shared GIS helper `collect_layer_polygons`: parallelized per-feature polygon extraction with `par_iter().map()` into per-feature vectors, followed by deterministic flattening.
- shared GIS point helpers (`collect_point_samples`, `collect_point_weights`, `collect_point_coords_from_layer`): parallelized per-feature coordinate extraction/attribute derivation with `par_iter().map()` and deterministic flattening, preserving existing validation and output ordering semantics.

Implementation file:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batches 136–137 completed (2026-05-10):
- `topology_rule_autofix` (`line_endpoints_must_snap_within_tolerance` branch): migrated in-place mutable endpoint snapping loop to a parallel preparation pass over immutable features with deterministic sequential geometry application and ordered change-log assignment.
- `topology_rule_autofix` (`point_must_be_covered_by_line` branch): migrated point projection loop to a parallel preparation pass over immutable features with deterministic sequential geometry application and ordered change-log assignment.

Implementation file:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batch 138 completed (2026-05-10):
- `multipart_to_singlepart`: parallelized per-feature multipart decomposition using `par_iter().flat_map()` and retained deterministic sequential FID assignment/output ordering.
- `extract_raster_values_at_points`: parallelized per-feature point sampling with `par_iter().map()` and deterministic sequential attribute write-back.
- `deviation_from_regional_direction`: parallelized regional-angle accumulation with Rayon `fold/reduce` and parallel per-feature deviation computation, followed by deterministic sequential attribute writes.
- `split_with_lines`: parallelized per-line split-piece generation with `par_iter().map()` and deterministic sequential output feature assembly/FID assignment.
- `related_circumscribing_circle`: parallelized per-feature RC_CIRCLE metric computation with deterministic sequential attribute write-back.
- `hole_proportion`: parallelized per-feature HOLE_PROP computation with deterministic sequential attribute write-back.
- `patch_orientation`: parallelized per-feature orientation computation with deterministic sequential attribute write-back.
- `perimeter_area_ratio`: parallelized per-feature perimeter/area computation with deterministic sequential attribute write-back.
- `route_calibrate`: parallelized per-route calibration result derivation with deterministic sequential attribute/status application.
- `route_recalibrate`: parallelized per-route recalibration derivation with deterministic sequential attribute/status application.
- `random_points_in_polygon`: parallelized polygon extraction/preparation stages (`extract_polygons_from_geometry`, envelope/prepared polygon pairing) while preserving deterministic feature-order flattening and existing point-generation behavior.

Implementation files:
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 139 in progress (2026-05-10):
- `construct_vector_tin`: parallelized per-feature point extraction/value pairing and per-triangle ring preparation with deterministic sequential feature write/FID assignment.
- `raster_to_vector_points`: parallelized per-row raster point record extraction with deterministic sequential row-order feature writes and FID assignment.
- `line_intersections`: parallelized per-input-line intersection generation with `par_iter()` and deterministic sequential output feature assembly/FID assignment.
- `polygonize`: parallelized per-layer closed-ring extraction/dedup preparation with `par_iter().filter_map()` and deterministic sequential layer aggregation.
- `concave_hull`: parallelized per-feature coordinate extraction/collection with `par_iter().map()` and deterministic sequential flattening before hull construction.
- `route_event_overlay`: parallelized per-route overlap row generation over sorted route keys with `par_iter().map()`, retaining deterministic sequential output append/FID assignment.
- `route_event_merge`: parallelized per-route event merge processing over sorted route keys with `par_iter().map()`, preserving deterministic sequential final feature append/FID assignment and existing conflict-mode behavior.
- `delete_field`: parallelized per-feature attribute projection into retained schema columns with deterministic sequential assignment back to output features.
- `add_field`: parallelized per-feature attribute expansion with default-value append using parallel preparation and deterministic sequential assignment.
- `network_connected_components`: parallelized final component-attribute preparation with `par_iter().map()` and deterministic sequential feature attribute append.
- `route_measure_qa`: parallelized per-route QA issue generation over sorted route keys with `par_iter().map()`, retaining deterministic sequential output feature append/FID assignment and aggregate counter/report semantics.
- `route_event_split`: parallelized per-feature event split generation over immutable input with `par_iter().map()`, retaining deterministic sequential output append/FID assignment and existing boundary/min-segment semantics.
- `travelling_salesman_problem`: parallelized per-feature coordinate extraction and local-bounds preprocessing with `par_iter().map()`, retaining deterministic sequential point-order flattening and unchanged optimization semantics.
- `raster_area`: parallelized per-cell class-area accumulation with Rayon `fold/reduce`, plus parallel output-value derivation with deterministic sequential raster writes.
- `raster_perimeter`: parallelized per-cell class-perimeter accumulation (lookup-table pattern) with Rayon `fold/reduce`, plus parallel output-value derivation with deterministic sequential raster writes.
- `edge_proportion`: parallelized per-cell patch-count and edge-count accumulation with Rayon `fold/reduce`, preserving deterministic sequential output materialization.
- `boundary_shape_complexity`: parallelized initial per-cell skeleton-state initialization pass with `into_par_iter().map()`, preserving existing sequential thinning and branch-analysis semantics.
- `map_features`: parallelized initial per-cell nodata/priority preprocessing with Rayon `fold/reduce`, preserving deterministic row-major heap insertion and downstream labeling semantics.

Implementation files:
- `crates/wbtools_oss/src/tools/gis/mod.rs`
- `crates/wbtools_oss/src/tools/data_tools/mod.rs`

Batch 140 completed (2026-05-10):
- `find_lowest_or_highest_points`: parallelized per-cell min/max scan with Rayon `fold/reduce`, preserving deterministic sequential feature write/FID assignment.
- `filter_raster_features_by_area`: parallelized per-cell class-count accumulation with Rayon `fold/reduce`, preserving deterministic sequential output materialization.
- `euclidean_distance`: parallelized final per-cell output-value derivation with `into_par_iter().map()`, preserving deterministic sequential raster writes.
- `shape_complexity_index_raster`: parallelized combined per-cell transition-counting and bounds-aggregation pass with Rayon `fold/reduce`, reducing two sequential scans into one parallel pass.
- `raster_cell_assignment`: parallelized per-cell row/column/x/y value assignment with `into_par_iter().map()`, preserving deterministic sequential raster writes.

Batch 141 completed (2026-05-10):
- `centroid_raster`: parallelized per-cell row/column accumulation with Rayon `fold/reduce`, preserving deterministic sequential centroid mapping and report generation.
- `reclass_equal_interval`: parallelized per-cell interval-based reclassification with `into_par_iter().map()`, preserving deterministic sequential raster writes.
- `reclass`: parallelized per-cell reclassification in both assign-mode (HashMap lookup) and range-mode (rule matching) with `into_par_iter().map()` and Arc-wrapped rules, preserving deterministic sequential raster writes.

Implementation files:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Batch 142 in progress (2026-05-10):
- `cost_allocation`: parallelized per-cell source initialization phase with `into_par_iter().map()` computing backlink status and source value assignment in parallel, followed by deterministic sequential backtracking propagation; determinism preserved via sorted sequential write-back of initialized values.

Implementation files:
- `crates/wbtools_oss/src/tools/gis/mod.rs`

Note: Several medium-risk audit candidates already parallelized in prior work (raster_area, raster_perimeter, pick_from_list, line_intersections).

## Automated Screening Set (Needs Manual Confirmation)

Block-scan surfaced **90 candidates** where legacy appears parallelized and NG tool blocks do not contain explicit parallel tokens.

Important caveat:
- This list includes false positives where NG may parallelize in helper functions outside the specific `impl Tool` block.
- Use this set as triage input, not final truth.

Top examples from the screening set:
- `canny_edge_detection` -> `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`
- `k_means_clustering` -> `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`
- `mosaic` -> `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`
- `raster_calculator` -> `crates/wbtools_oss/src/tools/raster/raster_stats.rs`
- `zonal_statistics` -> `crates/wbtools_oss/src/tools/raster/raster_stats.rs`
- `resample` -> `crates/wbtools_oss/src/tools/remote_sensing/non_filter_tools.rs`

Full machine output:
- `docs/performance/audit_parallelization_gap_candidates_blockscan_v2.json`

## Recommended Batch Order

1. High-confidence quick wins (do first):
- `convert_nodata_to_zero`
- `modify_nodata_value`
- `set_nodata_value`

2. Next triage tranche:
- `raster_stats.rs` tools from the candidate JSON.
- `non_filter_tools.rs` tools from the candidate JSON.

3. Last tranche:
- hydrology/lidar candidates where helper-level parallelization is more likely and requires deeper path tracing.
