# Troubleshooting

## Plugin Does Not Appear in QGIS

Checks:
- Confirm plugin directory path is correct for active QGIS profile.
- Confirm plugin folder name is whitebox_workflows_qgis.
- Restart QGIS after install/symlink changes.

## Whitebox Provider Missing from Processing

Checks:
- Confirm plugin is enabled.
- Trigger discovery refresh.
- Confirm whitebox_workflows imports in QGIS Python environment.

## Tools Are Missing or Unexpectedly Locked

Checks:
- Rebuild/reinstall whitebox_workflows.
- Refresh discovery.
- Confirm runtime capability metadata matches expected tier.
- Confirm tool taxonomy and generated provider state are synchronized.

## Tool Runs but Output Is Missing

Checks:
- Verify output path exists and is writable.
- Verify input paths and formats are valid.
- Re-run on a small dataset to isolate data-specific failures.
- Check Processing logs for warnings/errors.

## Environment Mismatch Problems

Symptoms:
- import failures in plugin startup,
- inconsistent behavior between terminal and QGIS,
- discovery succeeds in one environment but not another.

Resolution:
- Ensure QGIS and your install command use the same Python environment.
- Re-run runtime install in that exact environment.
