# QGIS Plugin Changelog

This changelog tracks user-visible changes to the Whitebox Workflows QGIS plugin.

## Unreleased

### Fixed
- Fixed backend update failures on Windows with `PermissionError: [WinError 5]` when users click "Update Backend" to upgrade the whitebox-workflows package. On Windows, pip cannot replace the `.pyd` file while it is loaded in QGIS's Python process. The plugin now unloads the module from `sys.modules` and invalidates importlib caches **before** pip runs, releasing the file lock and allowing pip to safely replace the binary. This resolves the issue where updates fail and tools disappear until QGIS is restarted. ([#23](https://github.com/jblindsay/whitebox_next_gen/issues/23))

## 2.1.3 - 2026-06-14

### Fixed
- Fixed classifier tools showing a file-picker text field for the class label field parameter instead of a dropdown selector. Affected tools include `knn_classification`, `logistic_regression`, `random_forest_classification`, `fuzzy_knn_classification`, `nnd_classification`, `min_dist_classification`, and `parallelepiped_classification`. The parameter schema for `class_field` (and similar attribute field parameters) was incorrectly typed as a LiDAR file input, causing the QGIS plugin to render a file picker. Additionally, the parameter type-inference pre-pass was using only heuristic inference rather than explicit schema kinds, causing `training_data` vectors to be misclassified. Both issues are now resolved: schema kinds take precedence over heuristics, and attribute field parameters are correctly promoted to `QgsProcessingParameterField` dropdowns. Users can now select from available fields in the training layer without manual text entry.
- Fixed `obia_pipeline_basic` and `classify_objects_random_forest`, `classify_objects_svm`, and `classify_objects_ensemble_pro` OBIA tools incorrectly treating `training` and `class_field` parameters as LiDAR inputs. These tools now correctly render `training` as a CSV/Table file picker and `class_field` as a field name text input.
- Added curated parameter descriptions for `random_forest_classification_fit` and `random_forest_regression_fit` tools in the QGIS parameter metadata, providing users with clear guidance on band ordering, scaling options, and model usage.

### Enhanced
- **Baseline Matching tool** (`baseline_matching_analysis_pro`) is now a full 6-covariate Mahalanobis-distance model with SEMDB Table A.3 compliance:
  - **New covariate: slope** — computed from elevation via Horn's 8-neighbour algorithm (degrees).
  - **New covariate: mean pre-period NDVI** — extracted from multi-band NDVI stack (active matching feature, previously used only for diagnostics).
  - **Improved road distance** — now uses segment-based rasterization instead of vertex-only distance, correctly measuring distance to road interiors.
  - **Tenure field-based matching** — `tenure_field` parameter accepts a vector field name for exact-match constraints (replaces feature-identity matching).
  - **SEMDB-spec absolute-unit callipers** — replaced standard-deviation multipliers with absolute units (e.g., `calliper_elevation_m=200`, `calliper_slope_deg=10`, `calliper_soc_pct=10`, `calliper_ndvi_pct=10`, `calliper_road_distance_km=1`), matching published SEMDB guidelines. Callipers remain optional; when omitted, all donors eligible across full range.
  - **Covariate table updated** — documentation now reflects all six covariates with default callipers and interpretation guidance for practitioners.

## 2.1.2 - 2026-06-14

### Fixed
- Fixed Pro tools remaining [Locked] after a successful license activation. The activation flow now immediately queries the runtime for the effective tier and promotes the plugin's Tier setting to `pro` before performing the catalog refresh, so Pro tools unlock without any manual settings change. The same post-query logic applies to deactivation and license transfer, correctly resetting to `open` tier in those cases.
- Fixed `quantiles` tool failing catastrophically on rasters with extreme positive skewness. The fixed-bin histogram approach produced bin widths so coarse that quantile boundaries fell within a single bin, causing all valid pixels to be assigned the highest class. The backend tool now uses an adaptive-bin histogram that scales with valid cell count, ensuring correct equal-count quantile classes on any data distribution. The fix is transparent to QGIS users—the tool will produce correct results on highly skewed data without any parameter changes.

- Fixed `lidar_join` and `lidar_rooftop_analysis` only allowing a single LiDAR file to be selected in the QGIS tool dialog. The backend correctly marks the `inputs` parameter with `cardinality: multiple`, but the plugin's parameter type-inference ignored cardinality for LiDAR datasets. A new `lidar_files_in` parameter kind is now recognised; QGIS renders it as a multi-selection file list using `QgsProcessingParameterMultipleLayers` with `TypeFile`, allowing users to select any number of `.las`/`.laz` files in Finder/Explorer.
- Fixed `merge_vectors` and `polygonize` only allowing a single vector layer to be selected. A new `vector_layers_in` parameter kind is now recognised for vector inputs with `cardinality: multiple`; QGIS renders it as a multi-layer selector using `QgsProcessingParameterMultipleLayers` with `TypeVectorAnyGeometry`.
- Fixed `ascii_to_las` only allowing a single input file to be selected. The `inputs` parameter has `cardinality: multiple` on the `File` dataset kind and now maps to a new `files_in` kind, rendered as a multi-selection file list.
- Enhanced help documentation for Individual Tree Segmentation tool with expanded algorithm description, key features overview (adaptive bandwidth, vegetation filtering, flexible output encoding, grid acceleration, tiling support), real-world use cases, and performance tuning guidance for speed vs. accuracy trade-offs.
- Fixed QGIS plugin warning "Duplicate point_process_residuals for provider whitebox_workflows". The `PointProcessResidualsComparisonTool` in the backend had a mismatched manifest ID (`point_process_residuals` instead of `point_process_residuals_comparison`), causing it to collide with the original `PointProcessResidualsTool` during dynamic catalog registration. Manifest ID now correctly matches the tool's unique identifier.

## 2.1.0 - 2026-06-09

### Major Architecture Change
- **Eliminated static metadata bundling**: The plugin no longer ships with bundled static help files, tool taxonomy files, or parameter definitions. All tool discovery, help text, and parameter metadata is now queried dynamically from the backend at runtime via the catalog and schema APIs. This simplifies plugin maintenance, enables real-time tool updates without plugin redeployment, and ensures UI and backend metadata stay perfectly in sync.

### Fixed
- Fixed LiDAR tool outputs not appearing in the QGIS Layers panel after the algorithm completes. `QgsProcessingParameterFileDestination` (used for `.las`/`.laz` outputs) does not trigger auto-loading in QGIS. The plugin now explicitly calls `context.addLayerToLoadOnCompletion()` after each LiDAR tool run so the output point cloud is loaded into the Layers panel automatically — both for user-specified paths and `TEMPORARY_OUTPUT` — matching the behaviour users expect from raster and vector tools.
- Fixed `output_id_mode` parameter in `individual_tree_segmentation` not appearing as a dropdown in the QGIS tool dialog. The backend describes enum options using curly-brace format `{a|b|c}` without a preceding colon. The enum-option extractor now recognises this pattern, including options that contain `+` for combination modes (e.g., `rgb+user_data`, `rgb+point_source_id`). Users now see a dropdown with all five modes instead of a plain text box with no guidance.
- Removed `*.zlidar` from the LiDAR output file-format filter. The wblidar backend does not support the zlidar format; its presence in the save-dialog filter was misleading.
- Fixed `individual_tree_segmentation` tool GUI failure where the `output_id_mode` enum parameter was incorrectly being treated as a file destination, causing QGIS to pass a temp file path instead of the mode string (`rgb`, `user_data`, `point_source_id`, or combinations). Updated the plugin parameter type-inference logic to exclude semantic parameter names ending in `_mode`, `_type`, `_encoding`, `_format`, `_units`, `_method`, `_style`, `_scheme`, `_class`, and `_kind` from the `output_` file-destination rule.
- Fixed backend installation failure on Windows when QGIS is installed via the OSGeo4W installer. The OSGeo4W bundled Python does not include `pip` by default. The plugin now detects when pip is missing and displays a **persistent, user-friendly dialog** with step-by-step remediation instructions specific to OSGeo4W, rather than a generic error message. The remediation guidance includes commands for bootstrapping pip via `ensurepip` or the direct pip bootstrap script, eliminating the need for users to search documentation when they encounter this issue.
- Fixed "No Python interpreter is available" error when using Check Backend Updates or Runtime Diagnostics on macOS (and some Linux builds). In these environments `sys.executable` inside QGIS is the QGIS binary, not a Python executable. The interpreter discovery logic now also searches for Python inside the macOS QGIS `.app` bundle (alongside the binary and inside `Contents/Frameworks/Python.framework/`). As a further fallback, when `whitebox_workflows` is importable in-process the in-process path is used for version queries (which never spawns a subprocess), preventing the error entirely in the common case.
- Fixed Runtime Diagnostics panel rendering blank / appearing to hang on macOS. The panel was using `QMessageBox.information` which renders a blank white window on some macOS Qt builds when the message text is large. Replaced with a proper scrollable `QDialog` containing a `QPlainTextEdit` with a monospace font, matching the style of other plugin dialogs.
- Fixed "License transfer failed: No external Python interpreter was found for license operations" error when activating, deactivating, or transferring a Pro license in the standard QGIS-mode installation. License operations previously required an external subprocess Python, which does not exist in the standard pip-installed QGIS environment. License functions (`activate_license`, `deactivate_license`, `transfer_license`) are now called in-process via the already-loaded `whitebox_workflows` module when no external interpreter is configured.
- Added comprehensive flake8 validation pass to ensure compliance with QGIS plugin repository validator rules (W293, W503, W504, F841, F401, E303, E305). Fixed trailing whitespace on blank lines and removed unused variable assignments.
- Fixed `deploy_wbw_to_qgis.sh` development script emitting "Target directory already exists" warnings on repeated installs. The script now pre-removes the existing `whitebox_workflows/` package directory and `.dist-info` folder before installing, so pip always sees a clean target. Removed `--force-reinstall` flag (redundant after the pre-clean).

## 2.0.15 - 2026-06-09

- Fixed LiDAR tool outputs not appearing in the QGIS Layers panel after the algorithm completes. `QgsProcessingParameterFileDestination` (used for `.las`/`.laz` outputs) does not trigger auto-loading in QGIS. The plugin now explicitly calls `context.addLayerToLoadOnCompletion()` after each LiDAR tool run so the output point cloud is loaded into the Layers panel automatically — both for user-specified paths and `TEMPORARY_OUTPUT` — matching the behaviour users expect from raster and vector tools.
- Fixed `output_id_mode` parameter in `individual_tree_segmentation` not appearing as a dropdown in the QGIS tool dialog. The backend describes enum options using curly-brace format `{a|b|c}` without a preceding colon. The enum-option extractor now recognises this pattern, including options that contain `+` for combination modes (e.g., `rgb+user_data`, `rgb+point_source_id`). Users now see a dropdown with all five modes instead of a plain text box with no guidance.
- Removed `*.zlidar` from the LiDAR output file-format filter. The wblidar backend does not support the zlidar format; its presence in the save-dialog filter was misleading.
- Fixed `individual_tree_segmentation` tool GUI failure where the `output_id_mode` enum parameter was incorrectly being treated as a file destination, causing QGIS to pass a temp file path instead of the mode string (`rgb`, `user_data`, `point_source_id`, or combinations). Updated the plugin parameter type-inference logic to exclude semantic parameter names ending in `_mode`, `_type`, `_encoding`, `_format`, `_units`, `_method`, `_style`, `_scheme`, `_class`, and `_kind` from the `output_` file-destination rule.
- Fixed backend installation failure on Windows when QGIS is installed via the OSGeo4W installer. The OSGeo4W bundled Python does not include `pip` by default. The plugin now detects when pip is missing and displays a **persistent, user-friendly dialog** with step-by-step remediation instructions specific to OSGeo4W, rather than a generic error message. The remediation guidance includes commands for bootstrapping pip via `ensurepip` or the direct pip bootstrap script, eliminating the need for users to search documentation when they encounter this issue.
- Fixed "No Python interpreter is available" error when using Check Backend Updates or Runtime Diagnostics on macOS (and some Linux builds). In these environments `sys.executable` inside QGIS is the QGIS binary, not a Python executable. The interpreter discovery logic now also searches for Python inside the macOS QGIS `.app` bundle (alongside the binary and inside `Contents/Frameworks/Python.framework/`). As a further fallback, when `whitebox_workflows` is importable in-process the in-process path is used for version queries (which never spawns a subprocess), preventing the error entirely in the common case.
- Fixed Runtime Diagnostics panel rendering blank / appearing to hang on macOS. The panel was using `QMessageBox.information` which renders a blank white window on some macOS Qt builds when the message text is large. Replaced with a proper scrollable `QDialog` containing a `QPlainTextEdit` with a monospace font, matching the style of other plugin dialogs.
- Fixed "License transfer failed: No external Python interpreter was found for license operations" error when activating, deactivating, or transferring a Pro license in the standard QGIS-mode installation. License operations previously required an external subprocess Python, which does not exist in the standard pip-installed QGIS environment. License functions (`activate_license`, `deactivate_license`, `transfer_license`) are now called in-process via the already-loaded `whitebox_workflows` module when no external interpreter is configured.
- Added comprehensive flake8 validation pass to ensure compliance with QGIS plugin repository validator rules (W293, W503, W504, F841, F401, E303, E305). Fixed trailing whitespace on blank lines and removed unused variable assignments.
- Fixed `deploy_wbw_to_qgis.sh` development script emitting "Target directory already exists" warnings on repeated installs. The script now pre-removes the existing `whitebox_workflows/` package directory and `.dist-info` folder before installing, so pip always sees a clean target. Removed `--force-reinstall` flag (redundant after the pre-clean).

## 2.0.14 - 2026-06-07

- Removed debug print statements from `bootstrap.py` `load_whitebox_workflows()` and `discovery.py` `clear_runtime_cache()` used during development.
- Fixed a settings dialog crash on startup caused by dangling QCheckBox widget reference to a removed plugin settings field (`skip_auto_update_checks_in_local_mode`).
- Improved backend installation experience by silently suppressing red error popups after users close the install instructions dialog, preventing distracting error notifications when the backend is not yet installed.
- Enhanced the backend installation instructions dialog with a clear ⚠️ warning title ("Action Required — Install Whitebox Workflows Backend"), prominent orange header, and explicit text ("none of the tools will be available"). Made the copy button the default action so users can press Enter to copy the install command instead of accidentally closing the dialog.
- Applied code style fixes for QGIS plugin repository validator compliance: removed unused imports (`is_backend_not_installed_error`, `get_loaded_backend_info`); fixed unused variable assignments; added missing blank lines before nested functions; broke long lines to fit 120-character limit; stripped trailing whitespace from blank lines.
- Confirmed "Check backend updates" button works correctly with QGIS-only backend installation via pip-based setup.

## 2.0.13 - 2026-05-30

- Fixed QGIS parameter inference for stream-network tools so `streams` and `flow_accumulation` inputs are recognized as vector/raster selectors instead of plain string fields.
- Updated parameter-kind resolution to consume canonical backend `schema` metadata first, with `io_role`/`data_kind` and heuristic inference retained only as compatibility fallbacks.
- Fixed enum dropdown option population to prefer backend schema-enumerated `options` values/labels before legacy parsing fallbacks.
- Improved parameter-panel consistency by consuming backend-provided parameter ordering from manifest metadata for tool forms.

## 2.0.12 - 2026-05-28

- Fixed a `conditional_evaluation` parameter forwarding regression where optional branch values could be dropped from the QGIS payload, causing the FALSE branch to fall back to NoData in affected workflows.
- Added tool-specific branch-value handling for `true`/`false` (and alias forms) so raster, constant, and expression inputs are preserved reliably at execution time.
- Rebuilt and validated the packaged plugin ZIP for version 2.0.12.

## 2.0.11 - 2026-05-28

- Fixed a parameter-widget regression where some runtime-schema `string`/`file` inputs could override stronger inferred types, causing raster/vector/file selectors to render as plain text boxes for affected tools (including `raster_streams_to_vector`).
- Updated kind arbitration to preserve stronger inferred input/output layer and destination types when runtime metadata is generic, restoring expected QGIS processing widget behavior.
- Rebuilt and validated the packaged plugin ZIP for version 2.0.11.

## 2.0.10 - 2026-05-28

- Bumped plugin metadata version to 2.0.10 for local validation builds so test installs are distinguishable from repository-distributed 2.0.9 builds in QGIS Plugin Manager.
- Fixed a startup/upgrade regression introduced in 2.0.9 where bulk help-cache generation could block the QGIS UI while requesting per-tool runtime metadata, causing QGIS to appear hung or not responding during plugin load.
- Added a follow-up flake8 compatibility pass for QGIS plugin-repository validator rules (`W503`, `W504`, `F841`, `F401`, `W293`, `E303`, `E305`) after 2.0.7 packaging feedback.
- Fixed additional multiline boolean formatting issues in `algorithm.py`, `bootstrap.py`, `discovery.py`, and `plugin.py` to eliminate `W504` findings from the validator environment.
- Fixed remaining help-system lint findings by removing unused locals in `help.py`, removing an unused import and whitespace-only blank lines in `help_provider.py`, and normalizing spacing in `discovery.py`.
- Rebuilt and revalidated the packaged plugin ZIP to confirm the same validator rule classes are clean in both source and packaged files.

## 2.0.9 - 2026-05-27

- Fixed output destination type inference for tools with generic destination labels so raster/vector/LiDAR outputs are exposed to QGIS as layer destinations and auto-added to the layer tree.
- Updated parameter kind mapping to prefer canonical runtime metadata fields (`io_role`, `data_kind`) before heuristic fallback, reducing incorrect destination/widget typing for tools with generic parameter labels.
- Updated help-generation and tool-description rendering to prefer per-tool runtime metadata (`get_tool_metadata_json`) and fall back to catalog manifests when metadata is unavailable (for example injected wrapper/pseudo-tool entries).
- Propagated runtime context (`include_pro`, `tier`) through discovery/provider help generation to keep metadata-enriched descriptions aligned with active license/session visibility.

## 2.0.8 - 2026-05-27

- Restored missing output destination parameters for raster/vector/LiDAR tools when runtime metadata omits the destination but the static help signature still returns a loadable layer type.
- Suppressed transient external Python console windows where the host platform supports hidden subprocess launches.
- Ran flake8 cleanup against the QGIS validator rule classes (`W503`, `W504`, `F841`, `F401`, `W293`, `E303`, `E305`) across plugin source files and the packaged plugin ZIP payload.
- Fixed multiline boolean formatting (`W503`/`W504`) in `algorithm.py`, `bootstrap.py`, `discovery.py`, `plugin.py`, and `help_provider.py`; fixed spacing (`E303`/`E305`) in `discovery.py`; fixed assigned-but-unused local (`F841`) in `help.py`; and fixed unused import/trailing-whitespace (`F401`/`W293`) in `help_provider.py`.
- Rebuilt the plugin package through preflight and validated that the same lint classes are clean in both source and `whitebox_workflows_for_qgis-2.0.8.zip`.

## 2.0.7 - 2026-05-27

- Improved temporary-output handling for generic output destinations so `TEMPORARY_OUTPUT` and `.file` placeholders are materialized to concrete writable paths, including tools that expose outputs as generic file parameters.
- Resolved plugin source lint issues (W503, F841, W292, W293, E203) reported by the QGIS plugin repository validator.

## 2.0.6 - 2026-05-27

- Fixed raster temporary-output handling so QGIS placeholder destinations (e.g. `TEMPORARY_OUTPUT`) are materialized to a real temporary GeoTIFF before backend execution, resolving `unknown raster format: .file` errors.
- Fixed vector temporary-output handling so QGIS temporary vector destinations are similarly materialized to a temporary GeoPackage before backend execution.

## 2.0.5 - 2026-05-26

- Improved runtime initialization defaults and capability-based startup behavior for first-run reliability across environments.
- Applied staged source cleanup updates from plugin validation and lint passes.
- Refreshed packaged plugin metadata and rebuilt the distribution zip for version 2.0.5.

## 2.0.4 - 2026-05-26

- Fixed plugin startup order so backend availability/install checks run before provider registration, preventing early runtime bootstrap failures on fresh QGIS installs.
- Resolved a Windows first-run issue where provider initialization could fail with `ModuleNotFoundError: No module named 'whitebox_workflows'` before users were prompted to install the backend.
- Added regression coverage to ensure backend checks execute before processing provider registration.
- Refreshed packaged plugin metadata and rebuilt the distribution zip for version 2.0.4.

## 2.0.3 - 2026-05-26

- Fixed Windows external-runtime bootstrap selection so the plugin avoids invalid QGIS launcher interpreter paths and prefers the real embedded Python runtime when probing whitebox_workflows.
- Added bootstrap regression tests covering Windows interpreter discovery and current-runtime fallback behavior.
- Updated packaged plugin metadata and refreshed the release zip for version 2.0.3.
- Cleaned packaged plugin files to remove cache artifacts, macOS hidden files, and release-lint issues in staged Python sources.
- Updated bundled help-page links and manual targets in the packaged plugin help content.

## Format

- Add new entries to `Unreleased` while work is in progress.
- When shipping a plugin release, move those entries into a new version section with the release date.
- Prefer short, user-visible bullets over internal implementation detail unless the detail explains a compatibility or packaging issue.