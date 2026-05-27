# QGIS Plugin Changelog

This changelog tracks user-visible changes to the Whitebox Workflows QGIS plugin.

## Unreleased

- Fixed output destination type inference for tools with generic destination labels so raster/vector/LiDAR outputs are exposed to QGIS as layer destinations and auto-added to the layer tree.
- Updated parameter kind mapping to prefer canonical runtime metadata fields (`io_role`, `data_kind`) before heuristic fallback, reducing incorrect destination/widget typing for tools with generic parameter labels.

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