# QGIS Plugin Changelog

This changelog tracks user-visible changes to the Whitebox Workflows QGIS plugin.

## Unreleased

- No unreleased entries yet.

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