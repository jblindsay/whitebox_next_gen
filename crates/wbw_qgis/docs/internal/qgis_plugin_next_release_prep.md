# QGIS Plugin Next Release Prep

Date: 2026-05-24
Scope: Whitebox Workflows for QGIS plugin upload prep

## Goal

Have a one-command preflight and packaging step ready for the next plugin release.

## One-Command Preflight + Package

From the repo root:

```bash
bash crates/wbw_qgis/plugin/scripts/preflight_qgis_plugin_release.sh
```

Optional explicit version override:

```bash
bash crates/wbw_qgis/plugin/scripts/preflight_qgis_plugin_release.sh --version 2.0.2
```

## What The Script Does

1. Verifies required plugin files are present.
2. Optionally runs `pycodestyle` if installed.
3. Optionally runs `bandit` if installed.
4. Builds upload archive with required root folder name:
   - `whitebox_workflows_for_qgis/`
5. Excludes forbidden files from archive:
   - `__pycache__/`
   - `.DS_Store`
   - `._*`
6. Verifies final archive structure and forbidden-file absence.

## Suggested Changelog Line For Next Release

- Maintenance: cleaned plugin style warnings reported by QGIS scan (whitespace/newline/line-break formatting) and kept security scanner compatibility fixes in place.

## Upload Checklist

1. Confirm `metadata.txt` version is set for release.
2. Run the preflight script.
3. Upload generated zip from:
   - `crates/wbw_qgis/plugin/whitebox_workflows_for_qgis-<version>.zip`
4. Verify QGIS scan summary and warning count.
