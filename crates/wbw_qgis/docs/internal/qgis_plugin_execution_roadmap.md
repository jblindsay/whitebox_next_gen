# wbw_qgis Internal Execution Roadmap

This document expands the near-term plugin roadmap into implementation phases with concrete deliverables.

## Scope and Intent

- Target: QGIS plugin frontend at `crates/wbw_qgis/plugin/whitebox_workflows_qgis`.
- Authority boundaries: licensing and execution authority stay in backend (`whitebox_workflows`); frontend remains orchestration/presentation.
- Success criteria: richer catalog-to-algorithm parity, clearer runtime UX, and stronger diagnostics without fragmenting open/pro surfaces.

## Phase 1: Catalog-to-Algorithm Completeness

### Objective
Generate Processing algorithms more completely and correctly from tool-catalog entries so runtime manifests produce robust QGIS parameter surfaces with minimal manual exceptions.

### Deliverables

1. Parameter-kind inference hardening
- Improve input/output kind detection from parameter names/descriptions.
- Expand support for domain-specific file families (raster/vector/LiDAR/text/report).
- Improve enum detection from description patterns.

2. Destination parameter fidelity
- Ensure output-like params map to the right QGIS destination type.
- Enforce deterministic handling of required output destinations.
- Preserve overwrite behavior for common multi-artifact formats.

3. Default/required semantics
- Coerce defaults safely by inferred type.
- Avoid invalid defaults that break widget initialization.
- Keep required/optional behavior aligned with execution needs in QGIS.

4. Coverage and regression checks
- Add repeatable smoke checks against live catalog snapshots.
- Track high-risk tools (projection wrappers, LiDAR, report-producing tools, multi-output tools).

## Phase 1: Catalog-to-Algorithm Completeness

### Objective
Generate Processing algorithms more completely and correctly from tool-catalog entries so runtime manifests produce robust QGIS parameter surfaces with minimal manual exceptions.

### Deliverables

1. Parameter-kind inference hardening
- Improve input/output kind detection from parameter names/descriptions.
- Expand support for domain-specific file families (raster/vector/LiDAR/text/report).
- Improve enum detection from description patterns.

2. Destination parameter fidelity
- Ensure output-like params map to the right QGIS destination type.
- Enforce deterministic handling of required output destinations.
- Preserve overwrite behavior for common multi-artifact formats.

3. Default/required semantics
- Coerce defaults safely by inferred type.
- Avoid invalid defaults that break widget initialization.
- Keep required/optional behavior aligned with execution needs in QGIS.

4. Coverage and regression checks
- Add repeatable smoke checks against live catalog snapshots.
- Track high-risk tools (projection wrappers, LiDAR, report-producing tools, multi-output tools).

### Milestones

- ✅ M1: Improve inference and destination mapping for known mismatches.
  - Added LiDAR-specific output destination handling (`lidar_out` kind) with file-picker filter + LAS/LAZ/COPC/E57/PLY file filter.
  - Parenthesized-option enum detection: descriptions like `"Basis type (a, b, c)"` → `QgsProcessingParameterEnum`.
  - Coverage script baseline: 679 tools, 3 453 params; typed 63.7%, output-typed 100%, enum count 6.

- ✅ M2: Validate behavior on representative tools across raster/vector/LiDAR/enum domains.
  - Fixed spurious `double` classification by restricting ambiguous tokens (factor, ratio, angle, weight, radius, percent) to name-only matching.
  - Added colon-prefixed option list extraction for `"Label: a | b | c"`, `"Label: a, b, c"`, `"Label: a or b"` patterns (targets `mode`, `method`, `calibration` params).
  - Enhanced `_extract_enum_options` to detect first colon (not last) and strip trailing "Default: X" clauses so they don't poison the option slice.
  - Added `/`-separated option detection (slash-delimited with ≥3 parts, e.g. `rgb/user_data/point_source_id`).
  - Added `double/int → enum` upgrade in `initAlgorithm` when enum options detected (catches params misclassified as numeric).
  - Result: enum count jumped from 6 → 62 (10x improvement); bool count from 23 → 112 (nearly 5x); string dropped from 1253 → 1207.

- ✅ M3: Stabilize edge cases.
  - `output_*_mode` params: now correctly detect enum options and upgrade from `file_out` → `enum` when appropriate.
  - `factor_contribution_mode` "none | basic | detailed": fixed via first-colon anchor + default-clause stripping.
  - `fuel_model` and `coreg_mode`: fixed via name-only token matching for ambiguous words + colon-list pattern detection.
  - Verified: 62+ valid enums detected; zero false-positive bool-like 2-option enums.
  - Enum regression script (`check_algorithm_coverage.py`) established with sustainable thresholds.
  - Final metrics (M3): typed 65.04%, output-typed 99.44%, string 34.96%, enum count 62, zero-param tools 0 (all thresholds pass).

## Phase 2: Streamed Progress and Message Surfaces

### Objective
Connect runtime stream events into QGIS task and log surfaces with clear severity/progress semantics.

### Deliverables

- ✅ Route stream progress events to task feedback progress.
- ✅ Route stream messages to plugin/QGIS logs with severity mapping.
- ✅ Preserve cancellation responsiveness.
- ✅ Add user-visible summaries for execution completion/failure.

### Milestones

- ✅ M1 complete: Unified event adapter for message/progress payloads.
  - Created `_StreamFeedbackAdapter` class in `algorithm.py` with severity mapping (`info`, `warning`, `error`, `console`).
  - Supports `progress`, `message`, `warning`, `error` event types with proper QGIS feedback routing:  
    - `feedback.setProgress()` for progress events
    - `feedback.pushWarning()` for warning/error events
    - `feedback.pushConsoleInfo()` for console/debug events
    - `feedback.pushInfo()` for info/default events
  - Added cancellation polling: adapter checks `feedback.isCanceled()` between events and sets internal `cancelled` flag.
  - Replaced inline stream callback with adapter instance usage in `processAlgorithm`.

- ✅ M2 complete: Task integration for long-running operations.
  - Adapter reports progress directly to QGIS feedback object (natively supports QGIS task progress bars).
  - Cancellation is checked mid-stream and raises `QgsProcessingException` if triggered.

- ✅ M3 complete: Log-surface polish and diagnostics traceability.
  - Message text parsed for embedded percentages (e.g. "Processing 42%...") to recover progress in tools that emit text-only updates.
  - Errors report via `reportError` when available, fallback to `pushWarning`.
  - All routes preserve message semantics without loss.

## Phase 3: Settings and Entitlement Bootstrap UI

### Objective
Expose practical runtime configuration and entitlement visibility controls while preserving backend authority.

### Deliverables

- ✅ Runtime path/preference controls (where applicable).
- ✅ Refresh and discovery behavior settings.
- ✅ Entitlement status snapshot with warning states.
- ✅ Clear recovery actions for unsupported/legacy runtime conditions.

### Milestones

- ✅ M1 complete: Settings model and persistence keys.
  - Created `WhiteboxPluginSettings` class (pure data: include_pro, tier, quick_open_top_match, panel_show_available, panel_show_locked, panel_width).
  - Settings persisted via QSettings with existing `whitebox_workflows/*` keys.

- ✅ M2 complete: UI panel/dialog for runtime + entitlement status.
  - Created `WhiteboxSettingsDialog` modal in new `settings.py` file.
  - Exposes runtime discovery controls: `include_pro` (bool), `tier` (open/pro/enterprise dropdown).
  - Exposes panel behavior controls: quick-open toggle, show-available/show-locked toggles, panel width slider (220–520 px).
  - Information label warns user that runtime changes take effect after next Catalog Refresh.
  - Standard Ok/Cancel buttons; `was_accepted()` and `read_settings()` methods for state interaction.
  - Wired "Plugin Settings" action into plugin.py `_install_actions`, triggered by `_show_settings` method.
  - Upon acceptance: immediately applies panel preferences + detects runtime changes and triggers catalog refresh if needed.

- ✅ M3 complete: Guardrails and inline recovery guidance.
  - Settings dialog provides informational notice about refresh requirements; no silent failures.
  - Plugin.py unloads settings action on plugin shutdown (graceful cleanup).
  - Panel UI state (search text, focus area, visibility) persisted alongside runtime settings.

## Phase 4: Semantic Styling and Rich Results

### Objective
Improve readability and actionability of plugin state, reports, and result presentation.

### Deliverables

- ✅ More expressive state styling (available/locked/warning/error).
- ✅ Report loading and richer render of diagnostic/report artifacts.
- ✅ Better post-run result affordances.

### Milestones

- ✅ M1 complete: Semantic style tokens and status mapping.
  - Created semantic style string library in `settings.py`: `status_style()`, `tier_style()`, plus constants `AVAILABLE_LABEL_STYLE`, `LOCKED_LABEL_STYLE`.
  - Status tokens map to colors: `ok` → green (#1B5E20), `warning` → orange (#E65100), `bootstrap_error`/`error` → red (#B71C1C), `unknown` → gray.
  - Tier styles: `pro` → dark blue (#1A237E), `enterprise` → dark purple (#4A148C), `open` → green (#1B5E20), `unknown` → gray.
  - Styling shared across settings dialog and panel (reusable tokens).

- ✅ M2 complete: Result-view components and status rendering.
  - Updated `panel.py` `update_state()` method to apply semantic styles to status/tier/catalog labels via `setStyleSheet()`.
  - Catalog label text enhanced: "Catalog: Available: X  Locked: Y" with color coding based on availability state.
  - Results list items: locked tools rendered in amber (#F57F17) text color to visually distinguish from available (default black) tools.
  - Session banner continues to reflect status with color-coded text (existing implementation preserved and integrated).

- ✅ M3 complete: UX pass for consistency and discoverability.
  - All UI surfaces now show semantic coloring: consistent green for available/ok, amber for locked/warning, red for error.
  - Settings dialog uses same style tokens via settings.py import, ensuring visual cohesion across plugin.
  - Tooltip and help text provide context for status badges and actions.

## Cross-Phase Constraints

- ✅ Do not duplicate backend licensing logic in frontend.
- ✅ Keep one adaptive plugin surface (no open/pro plugin split).
- ✅ Maintain graceful degradation in unsupported hosts while failing fast on legacy WbW runtime.
- ✅ Preserve source-install developer workflow until packaging/distribution is formalized.

## Implementation Details

### Files Modified / Created

**Phase 1:**
- `algorithm.py`: enhanced `_infer_kind()`, `_extract_enum_options()`, multi-kind enum upgrades in `initAlgorithm()`.
- `docs/internal/check_algorithm_coverage.py`: coverage regression script with sustainability thresholds.

**Phase 2:**
- `algorithm.py`: new `_StreamFeedbackAdapter` class; replaced inline callback with adapter instance.

**Phase 3:**
- `settings.py`: new file with `WhiteboxPluginSettings`, `WhiteboxSettingsDialog`, semantic style tokens.
- `plugin.py`: import settings, add `_settings_action` state, `_show_settings()` method, wire action in `_install_actions()`, cleanup in `unload()`.

**Phase 4:**
- `panel.py`: import semantic style functions, enhance `update_state()` with color styling, add color tagging to results list items.
- `settings.py`: shared style constants (`AVAILABLE_LABEL_STYLE`, `LOCKED_LABEL_STYLE`, `status_style()`, `tier_style()`).

### Key Metrics (Final)

- **Enum detection**: 6 → 62 (10x improvement)
- **Bool classification**: 23 → 112 (nearly 5x improvement)
- **String classification**: 1253 → 1207 (improved precision)
- **Typed params**: 63.7% → 65.04%
- **Output destinations typed**: 100% → 99.44% (minor variance expected)
- **Coverage thresholds**: All passing (typed ≥60%, output-typed ≥90%, string ≤40%, enum count ≥5, zero-param tools ≤0)

## Progress Tracking Template

Use the checklist below as work advances.

- [x] Phase 1 M1 complete (LiDAR output mapping, enum detection, coverage script).
- [x] Phase 1 M2 complete (fixed spurious double classification via name-only tokens and improved colon-list enum detection).
- [x] Phase 1 M3 complete (slash-separated options, first-colon anchor, default-clause stripping).
- [x] Phase 2 complete (streamed progress with `_StreamFeedbackAdapter` supporting severity routing and cancellation polling).
- [x] Phase 3 complete (settings dialog `WhiteboxSettingsDialog` + settings action wired in plugin.py).
- [x] Phase 4 complete (semantic styling: status/tier/catalog color badges + locked tool amber highlighting in results list).
