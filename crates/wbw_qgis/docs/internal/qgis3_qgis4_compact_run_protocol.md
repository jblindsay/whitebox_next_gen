# QGIS 3/4 Compact Run Protocol

Date: 2026-05-24
Purpose: Fast, repeatable click-path run for dual-host smoke validation.
Scope: `crates/wbw_qgis/plugin/whitebox_workflows_qgis`

## Host Sessions

1. Run once in QGIS 3.28.x.
2. Run once in QGIS 4.x.
3. Use the same plugin build/artifact in both.

## Protocol A: Startup and Provider Visibility

1. Launch QGIS.
2. Click Plugins > Manage and Install Plugins....
3. Confirm Whitebox Workflows is Enabled.
4. Click Processing > Toolbox.
5. In toolbox search, type Whitebox Workflows.
6. Confirm provider group appears without startup crash.

Pass condition: provider visible and host remains stable.

## Protocol B: Settings Persistence

1. Click Plugins > Whitebox Workflows > Settings.
2. In Runtime Discovery:
- Set include_pro to checked.
- Set tier to open.
- Set runtime_mode to auto.
3. Click OK.
4. Reopen Plugins > Whitebox Workflows > Settings.
5. Verify values persisted exactly.
6. Click Cancel.

Pass condition: dialog opens, saves, and reloads values correctly.

## Protocol C: Backend Diagnostics and Prompt Path

1. Click Plugins > Whitebox Workflows > Diagnostics.
2. Confirm diagnostics info is displayed (dialog or host message bar fallback).
3. Click Plugins > Whitebox Workflows > Check for Backend Updates.
4. If prompt appears, choose Skip or Defer (no install required for smoke).
5. Confirm no crash and feedback message appears.

Pass condition: prompts/feedback render and host remains stable.

## Protocol D: Panel and Recipe Interactions

1. Click Plugins > Whitebox Workflows > Open Panel.
2. In search box, type slope.
3. Select top result and open quick action path.
4. Add selected result to favorites.
5. Remove same favorite.
6. Open recipe menu in panel.
7. Click Open Recipe File.
8. Click Reload Recipes.
9. Click Validate Recipes.

Pass condition: interactions complete with no UI exceptions.

## Protocol E: Processing Surface Checks

1. Open-core path:
- Processing > Toolbox > Whitebox Workflows > run one open-tier tool.
2. Pro visibility path:
- In panel, verify pro/locked entries show expected state for current entitlement/tier.
3. Report path:
- Run one tool/flow that emits a report artifact and open/render it.

Pass condition: all three paths execute or clearly report expected lock state without crash.

## Protocol F: Progress and Cancellation

1. Start a longer-running Whitebox tool.
2. Observe progress updates in host UI.
3. Trigger cancellation (if available for that tool path).
4. Confirm host/plugin remains stable after cancellation.

Pass condition: no hang/crash; UI returns to usable state.

## Fast Result Record (Paste into Plan)

Host: <QGIS 3.28.x or QGIS 4.x>
Date: <YYYY-MM-DD>

- Startup/provider: PASS | FAIL | PARTIAL
- Settings persistence: PASS | FAIL | PARTIAL
- Backend prompts: PASS | FAIL | PARTIAL
- Panel/recipes: PASS | FAIL | PARTIAL
- Open-core run: PASS | FAIL | PARTIAL
- Pro visibility path: PASS | FAIL | PARTIAL
- Report path: PASS | FAIL | PARTIAL
- Progress/cancel: PASS | FAIL | PARTIAL

Notes:
- <issue or none>
