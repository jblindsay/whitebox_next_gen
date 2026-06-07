# QGIS Plugin Changelog

This changelog tracks user-visible changes to the Whitebox Workflows QGIS plugin.

## Unreleased

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