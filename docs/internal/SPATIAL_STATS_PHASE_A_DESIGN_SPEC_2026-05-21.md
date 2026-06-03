# Spatial Statistics Phase A Design Spec

**Date:** May 21, 2026
**Author:** Platform Capability Design
**Scope:** Open-tier Phase A implementation for core spatial statistics used in advanced GIS coursework.

---

## 1. Phase A Objectives

Deliver a robust baseline spatial-statistics suite for teaching and applied analysis:

1. Global Moran's I
2. Local Moran's I (LISA)
3. Getis-Ord Gi / Gi*
4. Nearest-neighbour index (NNI)
5. Quadrat count test

Primary goals:

- reproducible computation
- explicit inference outputs (p-values, z-scores, significance class)
- consistent API surface across Python, R, and QGIS
- machine-readable diagnostics for classroom and QA workflows

---

## 2. Shared Design Principles

1. **Inference is first-class output**
- Every tool returns statistic, expected value, variance basis, z-score, p-value, and significance class.

2. **Deterministic by default**
- Closed-form inference when available.
- If permutation inference is selected, require explicit seed and report permutation count.

3. **Weight-matrix transparency**
- Emit summary diagnostics for neighborhood construction and row standardization.

4. **Fail loudly on invalid assumptions**
- No silent fallback for null/constant fields, zero-neighbour features, or invalid CRS assumptions.

5. **Cross-binding parity**
- Same argument names and output key names in core runtime; wrappers map naming style only.

---

## 3. Proposed Tool IDs and Contracts

### 3.1 `global_morans_i`

**Inputs**
- `input`: vector layer (point/polygon)
- `field`: numeric attribute
- `weights_mode`: `queen|rook|k_nearest|distance_band`
- `k`: optional (required for `k_nearest`)
- `distance`: optional (required for `distance_band`)
- `row_standardize`: bool (default `true`)
- `inference`: `asymptotic|permutation` (default `asymptotic`)
- `permutations`: int (default 999 when permutation)
- `seed`: optional int
- `output_report`: optional JSON path

**Outputs**
- `statistic_i`
- `expected_i`
- `variance_i`
- `z_score`
- `p_value_two_sided`
- `n_features_used`
- `n_features_dropped`
- `weights_summary` (neighbors min/mean/max, islands)
- optional report path

### 3.2 `local_morans_i_lisa`

**Inputs**
- all neighborhood/inference controls above
- `output`: vector output path
- `alpha`: significance threshold (default 0.05)
- `multiple_testing`: `none|fdr_bh|bonferroni` (default `fdr_bh`)

**Outputs**
- output vector with per-feature fields:
  - `LISA_I`
  - `LISA_Z`
  - `LISA_P`
  - `LISA_P_ADJ`
  - `LISA_SIG`
  - `LISA_CLASS` (`HH|LL|HL|LH|NS`)
- `summary` object in run outputs with class counts and island counts

### 3.3 `getis_ord_gi_star`

**Inputs**
- same neighborhood controls
- `variant`: `gi|gi_star` (default `gi_star`)
- `alpha`, `multiple_testing`
- `output`: vector output path

**Outputs**
- output vector fields:
  - `GI_Z`
  - `GI_P`
  - `GI_P_ADJ`
  - `GI_SIG`
  - `GI_CLASS` (`hot|cold|ns`)
- summary counts by class/significance

### 3.4 `nearest_neighbour_index`

**Inputs**
- `input`: point layer
- `distance_metric`: `euclidean` (Phase A)
- `study_area_mode`: `hull|envelope|polygon_layer`
- `study_area_polygon`: optional when `polygon_layer`
- `edge_correction`: `none|guard` (default `none`)

**Outputs**
- `observed_mean_distance`
- `expected_mean_distance_csr`
- `nni_ratio`
- `z_score`
- `p_value_two_sided`
- `n_points`
- `study_area`

### 3.5 `quadrat_count_test`

**Inputs**
- `input`: point layer
- `grid_mode`: `rows_cols|cell_size`
- `rows`, `cols` or `cell_size`
- `study_area_mode`: `hull|envelope|polygon_layer`
- `study_area_polygon`: optional when `polygon_layer`
- `output_grid`: optional vector output path

**Outputs**
- `chi_square`
- `df`
- `p_value`
- `variance_to_mean_ratio`
- `n_quadrats`
- `n_points`
- optional quadrat grid with count and expected fields

---

## 4. Shared Statistical Output Schema

All Phase A tools should emit a harmonized JSON-like output block:

- `tool_id`
- `inference_method`
- `statistic`
- `z_score` (if applicable)
- `p_value`
- `alpha`
- `significance_class`
- `n_observations`
- `dropped_observations`
- `weights_diagnostics`
- `assumption_flags`
- `warnings`
- `runtime_metadata` (seed, permutations, timing)

This schema is used by wrappers and QGIS plugin preview/report panes.

---

## 5. Report-Style Output Contract

For tools that do not primarily emit spatial layer outputs, output formatting should follow a deterministic, automation-friendly contract.

1. **Primary output: structured JSON payload (default)**
- Every run writes or returns the shared statistical output schema in machine-readable form.
- This is the source of truth for wrappers, reproducibility, and regression checks.

2. **Secondary output: concise CLI summary**
- Print a compact human-readable summary to stdout/stderr.
- Summary must be derived from the same JSON payload fields.

3. **Optional artifact: formatted HTML report**
- HTML is supported as an optional teaching/presentation artifact.
- HTML is not the canonical output and should not be required for automation.

4. **Optional artifact: CSV tables**
- CSV is optional for summary/per-feature tables used in grading or spreadsheets.

### 5.1 Recommended output arguments

- `output_json`: preferred structured result path (required or strongly recommended default)
- `output_html`: optional formatted report path
- `output_csv`: optional tabular export path

### 5.2 Design rationale

- JSON-first preserves deterministic API contracts and testability.
- Optional HTML improves teaching usability without coupling compute logic to presentation formatting.
- Optional CSV supports workflow interoperability without replacing the structured schema.

---

## 6. Neighborhood / Weights Design

### 6.1 Phase A weight builders

- Contiguity (queen/rook) for polygon support
- k-nearest for point and centroid-derived polygon workflows
- distance-band

### 6.2 Diagnostics requirements

Always emit:

- features with zero neighbours
- neighbour-count histogram summary
- whether row standardization applied
- connected-component count in neighbor graph

### 6.3 Island policy

Configurable policy (default `drop_with_warning`):

- `drop_with_warning`
- `keep_zero_weight` (where mathematically valid)
- `error`

---

## 7. Numerical and Performance Requirements

1. Use stable accumulation for sum-of-products terms.
2. Parallelize neighbourhood evaluation where safe.
3. Support classroom-scale datasets efficiently (10k to 250k features).
4. Document expected scaling and memory envelopes.

---

## 8. Wrapper and UI Parity Plan

### Python
- Add typed methods under a nested stats namespace.
- Provide examples for points and polygons with explicit weights config.

### R
- Add `wbw_<tool>` wrappers matching core args.
- Return structured lists with outputs and diagnostics payload.

### QGIS
- Expose all parameters in Processing dialogs.
- Provide static help pages with interpretation guidance.
- Add optional report-open action for diagnostics JSON.

---

## 9. QA and Validation Plan

1. **Known-answer tests**
- small synthetic datasets with analytically verifiable outputs
- textbook benchmark datasets for Moran/LISA/Gi*

2. **Cross-tool validation**
- compare against established references (R/spdep workflows) with tolerance bounds

3. **Permutation determinism tests**
- fixed seed reproducibility across runs

4. **Edge-case tests**
- constant field
- high island count
- tiny sample size
- mixed/null-heavy inputs

---

## 10. Teaching Deliverables (Required)

1. Intro lab: global Moran's I and interpretation.
2. Intermediate lab: LISA + Gi* with multiple-testing discussion.
3. Point-pattern lab: NNI + quadrat count interpretation.
4. Instructor notes: assumptions, common misuse, and troubleshooting.

---

## 11. Out-of-Scope for Phase A

- Kriging (moved to Phase B)
- Spatial regression/GWR (moved to Phase C)
- Point-process model fitting (moved to Phase D)

---

## 12. Immediate Engineering Next Steps

1. Create `spatial_stats` module in open-tier tools with shared weight builders.
2. Implement `global_morans_i` first and finalize shared schema from real output.
3. Implement `local_morans_i_lisa` and `getis_ord_gi_star` using same core weights stack.
4. Implement point-pattern tools (`nearest_neighbour_index`, `quadrat_count_test`).
5. Wire wrappers/manual examples in Python/R/QGIS in lockstep.

---

## 13. Status Snapshot (June 3, 2026)

### Checked Off

1. Core Phase A tools implemented and registered in open-tier runtime:
- `global_morans_i`
- `local_morans_i_lisa`
- `getis_ord_gi_star`
- `nearest_neighbour_index`
- `quadrat_count_test`

2. Core runtime tests in place for:
- registry presence/wiring
- output-field expectations for LISA and Gi/Gi*
- branded optional HTML report generation
- real-world smoke runs (attribute-rich points + polygon sample when available)

3. Explicit permutation-mode guardrails are implemented and regression-tested:
- all three autocorrelation tools currently reject `inference=permutation` with clear validation errors

4. Core runtime output-schema harmonization is implemented across all five Phase A tools:
- each tool now emits both `report` and `summary` keys
- shared contract keys are present (`statistic`, `p_value`, observation counts, diagnostics/warnings, runtime metadata)

### Remaining For Phase A Closeout

1. Wrapper/UI parity completion:
- verify argument/output exposure and help parity for Python, R, and QGIS surfaces
- ensure optional report outputs and diagnostics are consistently surfaced

2. Additional known-answer numeric QA:
- extend beyond sign-level assertions to fixed-value tolerance checks against benchmark references

3. End-user documentation parity:
- add/update concise user-facing notes that permutation inference is intentionally deferred in Phase A and currently asymptotic-only
