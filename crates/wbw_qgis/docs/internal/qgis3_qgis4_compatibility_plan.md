# QGIS 3 and QGIS 4 Compatibility Plan

Date: 2026-05-24
Status: In progress (Phase 1 complete, Phase 2 complete, Phase 3 complete, Phase 4 release gate complete)
Scope: `crates/wbw_qgis/plugin/whitebox_workflows_qgis`

## Goal

Support both QGIS 3 (Qt5) and QGIS 4 (Qt6) from one plugin codebase, with compatibility logic isolated so QGIS 3 support can be removed later with minimal surgery.

## Principles

1. One plugin package and one codebase.
2. Compatibility logic centralized in a host adapter layer.
3. Business logic remains version-neutral.
4. Feature checks first, version checks only when unavoidable.
5. Every QGIS 3 shim is labeled for later removal.

## Target Architecture

- Primary compatibility module: `host_api.py`
- Optional split if needed:
  - `host_api_qgis3.py`
  - `host_api_qgis4.py`
  - facade in `host_api.py`
- Plugin/runtime modules (`plugin.py`, `settings.py`, `algorithm.py`, `panel.py`) call adapter functions only for host-specific behavior.

## Compatibility Surface Inventory

Checklist:

- [x] Metadata/version gates
- [x] Plugin startup/version checks
- [x] QAction and menu wiring behavior
- [x] Dialog execution semantics (`exec`/`exec_` style differences)
- [x] Qt enum/value differences
- [ ] Signal/slot edge differences
- [x] Message bar and log APIs
- [ ] File dialog behavior and filters
- [ ] Processing parameter and destination API differences
- [ ] Icon/theme resource handling
- [ ] Settings persistence behavior
- [ ] Thread/task cancellation and progress feedback paths

## Phase 1: Reopen Version Window

Objective: Remove hard QGIS4-only gating while preserving safe behavior.

Tasks:

- [x] Update plugin metadata to include QGIS 3 support window.
- [x] Replace hard major-version reject logic in `plugin.py` with adapter-driven capability checks.
- [x] Add startup diagnostics message when host capabilities are partial.

Acceptance:

- [x] Plugin loads in QGIS 3 and QGIS 4.
- [x] No immediate startup exception in either host.

## Phase 2: Adapter Hardening

Objective: Centralize all host differences in adapter utilities.

Tasks:

- [x] Add explicit host capability map in `host_api.py`.
- [x] Add wrappers for dialog execution, message severity routing, and action wiring.
- [x] Add wrappers for enums/constants likely to differ.
- [x] Add fallback logic for optional APIs missing in QGIS 3.
- [x] Mark all QGIS 3-only code paths with `QGIS3_COMPAT`.

Acceptance:

- [x] No direct Qt5/Qt6 conditional logic remains in feature modules.
- [x] Adapter unit/smoke checks pass for both host versions.

## Phase 3: UI and Workflow Compatibility Pass

Objective: Verify major user flows in both hosts.

Tasks:

- [x] Validate settings dialog open/save/load.
- [x] Validate backend bootstrap/install/update prompts.
- [x] Validate plugin panel interactions and search behavior.
- [x] Validate at least one processing run from each major surface:
  - open-core tool execution
  - pro/entitlement-aware tool visibility path
  - report artifact rendering path

Acceptance:

- [x] Core user flows complete in QGIS 3 and QGIS 4 without host-specific crashes.
- [x] Any degraded behavior is listed in known limitations.

## Phase 4: Test Matrix and Release Guardrails

Objective: Establish repeatable verification before release.

Tasks:

- [x] Add a compatibility smoke checklist for local/manual QA.
- [x] Add a minimal scripted validation where possible.
- [x] Add release checklist gate: both hosts tested before tag.

Acceptance:

- [x] Release checklist includes explicit QGIS 3 and QGIS 4 pass records.

## QGIS 3 Sunset Design (Future)

Objective: Ensure future removal is surgical.

Rules now:

- [x] Keep all QGIS 3 branches inside adapter modules only.
- [x] Tag each temporary branch with `QGIS3_COMPAT`.
- [ ] Do not split settings keys, data contracts, or backend behavior by host version.

Future removal checklist:

- [ ] Remove QGIS 3 metadata support window.
- [ ] Delete `QGIS3_COMPAT` branches in adapter modules.
- [ ] Remove QGIS 3 smoke tests.
- [ ] Run QGIS 4-only regression pass.

## Risks and Mitigations

1. Risk: Scattered version checks create maintenance overhead.
- Mitigation: enforce adapter-only host branching in code review.

2. Risk: Hidden Qt enum/method differences cause runtime errors.
- Mitigation: wrap known hotspots and use capability checks.

3. Risk: QGIS 3 support delays QGIS 4 delivery.
- Mitigation: define explicit non-goals and avoid feature divergence.

## Immediate Next Tasks

- [x] Implement Phase 1 gates in `metadata.txt` and `plugin.py`.
- [x] Add adapter capability map and wrappers in `host_api.py`.
- [x] Run first dual-host startup smoke test and log outcomes.
- [x] Execute Phase 3 manual smoke checklist in real QGIS 3.28 and QGIS 4.x hosts.
- [x] Prepare compact click-path run protocol for fast dual-host execution.
- [x] Execute and log Host B (QGIS 4.x) runtime results.

## Progress Notes

- 2026-05-24: Phase 1 completed.
  - `metadata.txt`: minimum host version opened to QGIS 3.28.
  - `plugin.py`: startup host gating moved to adapter capability map with partial-host warning.
- 2026-05-24: Phase 2 (partial) completed.
  - `host_api.py`: added host capability map and wrappers for context menu execution, dialog execution, and local file open.
  - `plugin.py`: removed direct feature-module `exec/exec_` branching for menus and settings/update dialogs in favor of adapter wrappers.
  - `host_api.py`: added cross-version Qt event/constant resolvers for Qt5/Qt6 namespace differences.
  - `panel.py`: switched event/constant compatibility lookups to adapter utilities.
  - `host_api.py`: added `push_host_message()` to centralize message bar severity routing.
  - `host_api.py`: added `show_info_dialog()` to centralize informational modal dialog routing.
  - `plugin.py`: routed startup and runtime notifications through `push_host_message()`.
  - `plugin.py`: replaced direct `QMessageBox.information(...)` usage with adapter dialog wrapper calls.
  - `field_calculator_dialog.py`: switched assistant modal execution to adapter `run_dialog()` wrapper.
- 2026-05-24: Static validation pass completed (non-host runtime).
  - Static audit: feature modules no longer contain direct Qt5/Qt6 conditional branches; compatibility branching remains in adapter/shim areas.
  - Import smoke (in `.venv-wbw`): `host_api`, `settings`, `field_calculator_dialog`, `panel`, and `plugin` modules import successfully.
  - Remaining blocker: true dual-host runtime smoke still requires manual execution inside real QGIS 3.28 and QGIS 4.x sessions.
- 2026-05-24: Phase 4 QA artifact added.
  - Added manual checklist: `qgis3_qgis4_manual_smoke_checklist.md` covering startup, settings, backend prompts, panel paths, processing runs, dialogs/files, and cancellation/progress.
- 2026-05-24: Phase 4 scripted smoke baseline added.
  - Added script: `check_qgis_compat_smoke.py`.
  - Script validates importability of compatibility-critical modules and adapter callable surface.
  - Current non-host run result in `.venv-wbw`: PASS (18 checks).
- 2026-05-24: Dual-host execution protocol added.
  - Added compact click-path runbook: `qgis3_qgis4_compact_run_protocol.md`.
  - Protocol is optimized for quick operator execution and direct pass/fail logging into this plan.
- 2026-05-24: QGIS 3 host install/startup sanity confirmed by operator.
  - Plugin installed into QGIS 3.44.10-Solothurn profile and reported as working in basic use.
  - This is a positive startup sanity signal for Host A (QGIS 3), but full Phase 3 flow-by-flow smoke evidence is still pending.
- 2026-05-24: Host A full runtime smoke recorded (operator run).
  - Host: QGIS 3.44.10-Solothurn
  - Startup/provider: PASS
  - Settings persistence: PASS
  - Backend prompts: PASS
  - Panel/recipes: PASS
  - Open-core run: PASS
  - Pro visibility path: PASS
  - Report path: PASS
  - Progress/cancel: PASS
  - Outcome: Host A runtime smoke passed end-to-end; remaining runtime gate is Host B (QGIS 4.x).
- 2026-05-24: Host B startup blocker identified and fixed.
  - Symptom in QGIS 4 Plugin Manager: "Plugin designed for QGIS 3.28 - 3.99".
  - Root cause: metadata did not explicitly declare a QGIS 4-compatible maximum version.
  - Fix: set `qgisMaximumVersion=4.99` in `metadata.txt`.
  - Next: relaunch/reload QGIS 4 plugin list and continue Host B smoke run.
- 2026-05-24: Host B full runtime smoke recorded (operator run).
  - Host: QGIS 4.x
  - Startup/provider: PASS
  - Settings persistence: PASS
  - Backend prompts: PASS
  - Panel/recipes: PASS
  - Open-core run: PASS
  - Pro visibility path: PASS
  - Report path: PASS
  - Progress/cancel: PASS
  - Outcome: Host B runtime smoke passed end-to-end.
- 2026-05-24: Dual-host compatibility gate completed.
  - Host A (QGIS 3.44.10-Solothurn) and Host B (QGIS 4.x) both passed full compact run protocol.
  - No host-specific crashes were reported in validated flows.
