# QGIS 3/4 Manual Smoke Checklist

Date: 2026-05-24
Scope: `crates/wbw_qgis/plugin/whitebox_workflows_qgis`

## Test Matrix

- Host A: QGIS 3.28.x (Qt5)
- Host B: QGIS 4.x (Qt6)
- Plugin package: same build/artifact for both hosts

## Preconditions

1. Plugin installed and enabled in each host.
2. A valid project opens without host-level errors.
3. Whitebox backend reachable in at least one runtime mode (`auto`, `local`, or `qgis`).

## Smoke Steps (Run In Each Host)

1. Startup and load
- Open QGIS and verify plugin loads without immediate exception.
- Confirm provider appears under Processing.
- Confirm startup diagnostics do not indicate unsupported host major.

2. Settings dialog
- Open plugin settings from menu.
- Toggle key options (`include_pro`, `tier`, `runtime_mode`) and save.
- Reopen settings and verify persisted values.

3. Backend prompts
- Trigger backend diagnostics.
- Trigger backend install/update prompt path (or no-op path if already current).
- Verify message routing shows actionable info/warning and no host crash.

4. Panel interactions
- Open dock panel.
- Search for a known tool and run quick-open/selection path.
- Add/remove favorites and verify panel updates.
- Validate recipe actions: open file, reload, validate.

5. Processing execution surfaces
- Open-core tool path: run one open-tier algorithm end to end.
- Pro visibility path: verify locked vs available behavior for pro/entitlement-sensitive entries.
- Report path: run one flow that emits a report artifact and verify open/render behavior.

6. File and dialog behavior
- Validate modal dialogs open/close correctly.
- Validate local file open fallback behavior (host open or clipboard fallback message).

7. Cancellation/progress sanity
- Launch a longer-running processing call.
- Verify progress updates and cancellation path do not crash host.

## Result Template

Use this block for each host run.

Host: <QGIS version>
Date: <YYYY-MM-DD>
Tester: <name>

- Startup/load: PASS | FAIL | PARTIAL
- Settings open/save/load: PASS | FAIL | PARTIAL
- Backend prompts: PASS | FAIL | PARTIAL
- Panel interactions: PASS | FAIL | PARTIAL
- Open-core tool execution: PASS | FAIL | PARTIAL
- Pro visibility path: PASS | FAIL | PARTIAL
- Report artifact rendering: PASS | FAIL | PARTIAL
- File/dialog behavior: PASS | FAIL | PARTIAL
- Cancellation/progress: PASS | FAIL | PARTIAL

Known issues/notes:
- <issue or 'none'>

## Exit Criteria

- Both hosts complete all smoke steps with no host-specific crash.
- Any degradations are documented with severity and follow-up owner.
- Compatibility plan is updated with dated pass/fail summary.
