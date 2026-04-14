# WbW QGIS 4 Plugin Architecture and Implementation Plan

Date: 2026-04-14
Status: Internal architecture baseline
Priority: High
Scope: QGIS 4-first plugin architecture for Whitebox Workflows Open Core and Whitebox Workflows Pro

---

## 1. Strategic Role

The QGIS plugin is expected to become the most important user-facing surface for Whitebox Workflows.

Why it matters:

1. It is the most visible desktop distribution channel for the Whitebox ecosystem.
2. It is likely to become the primary discovery surface for many OSS users.
3. It is expected to be a major commercial conversion surface for WbW-Pro.
4. It provides the most direct path for bringing Whitebox capabilities into mainstream GIS workflows.

This plugin should therefore be treated as a premier product surface, not a thin utility wrapper.

---

## 2. Core Decisions Locked By This Plan

1. The plugin will be designed primarily for QGIS 4.
2. QGIS 3 support, if provided at all, is transitional and must not distort the QGIS 4 architecture.
3. The plugin will be built as a Python frontend layered on top of WbW-Py (`wbw_python`).
4. The Processing provider will be the primary tool-execution surface.
5. A smaller custom product UI will complement the Processing provider for setup, licensing, discovery, upgrade messaging, and result/report browsing.
6. Tool discovery must be dynamic and metadata-driven.
7. Users should be able to refresh tool discovery after updating `whitebox-workflows` without requiring a plugin release.
8. The plugin should support both WbW-Open and WbW-Pro from a single runtime distribution model, with licensing enforced at runtime.
9. Local execution is the launch model.
10. Light semantic styling should be designed in from the start.

---

## 3. Product Positioning

The plugin should be understood as both:

1. A native QGIS Processing integration for the full Whitebox ecosystem.
2. A product shell for Whitebox-specific runtime setup, licensing, discovery, and workflow experience.

The Processing provider should carry the majority of tool access because:

1. Whitebox has too many tools for a custom dialog-only UX to scale cleanly.
2. QGIS users already understand the Processing Toolbox, model builder, history, and batch execution patterns.
3. Native Processing integration lowers training burden and improves adoption.

The custom plugin UI should focus on the parts Processing does not handle well:

1. Runtime/environment diagnostics.
2. Install and upgrade flow for WbW-Py.
3. License state and entitlement messaging.
4. Curated workflow discovery and featured bundles.
5. Reports, HTML outputs, and artifact browsing.
6. Upgrade prompts and locked-tool explanation.

---

## 4. High-Level Architecture

### 4.1 Layering model

The QGIS plugin should be a thin orchestration/presentation layer over WbW-Py.

Recommended stack:

1. Rust backend crates (`wbtools_oss`, `wbtools_pro`, and supporting crates).
2. WbW-Py as the canonical Python runtime and discovery surface.
3. QGIS plugin layer that consumes WbW-Py metadata and execution APIs.

The plugin should not bind directly to `wbtools_oss` or `wbtools_pro`.

Reasons:

1. WbW-Py should remain the single canonical Python runtime contract.
2. Tool discovery, execution, capability state, and error behavior should not be reimplemented separately in the plugin.
3. It reduces maintenance and avoids a second Python integration surface.
4. It allows the QGIS plugin to evolve largely as a UI/integration project.

### 4.2 Plugin components

The plugin should be structured into the following subsystems:

1. Bootstrap and compatibility layer.
2. Runtime/environment manager.
3. Metadata and capability discovery layer.
4. Processing provider.
5. Product shell UI (dock/panel/dialogs).
6. Background task runner.
7. Result loading and styling layer.
8. Licensing/session UX.

---

## 5. Monorepo Placement

The plugin should live in the monorepo alongside the Python and R frontends.

Recommended placement:

1. Add a new frontend package for WbW-QGIS in the monorepo.
2. Keep the QGIS plugin source close to `wbw_python` because it depends on that surface conceptually.
3. Treat it as a first-class frontend, not as an external add-on repository.

This preserves:

1. Shared release planning.
2. Coordinated frontend evolution.
3. Easier contract synchronization between WbW-Py and the plugin.

---

## 6. QGIS Version Strategy

### 6.1 Primary target

QGIS 4 should be the architectural target.

Reasons:

1. It is the long-term platform direction.
2. User migration appears to be happening quickly.
3. Attempting to optimize the design around both QGIS 3 and QGIS 4 from the start risks weakening the product.

### 6.2 Transitional support

If QGIS 3 support is provided, it should be explicitly transitional.

Requirements:

1. QGIS 3 compatibility logic must live in a narrow adapter layer.
2. Core plugin logic must not be written to QGIS 3 constraints.
3. Any QGIS 3-specific compromises must be temporary and documented.

---

## 7. Dynamic Tool Discovery

Dynamic discovery is a core requirement.

Implications:

1. The plugin must not ship a fixed tool catalog.
2. Tool registration in QGIS should be built from WbW-Py metadata at runtime.
3. Users should be able to refresh the discovered catalog after upgrading `whitebox-workflows` via pip.

This enables:

1. Tool catalog updates without requiring a new plugin release.
2. Faster delivery of new tools.
3. Cleaner separation between runtime/tool evolution and plugin-shell evolution.

### 7.1 Required user-facing actions

The plugin should provide:

1. Initial tool discovery on first successful runtime setup.
2. Manual “Refresh Tool Catalog” action.
3. A clear message recommending refresh after `pip install whitebox-workflows -U`.
4. Optional automatic version mismatch detection between plugin expectations and installed WbW-Py metadata schema.

---

## 8. WbW-Py Contract Requirements

The current `list_tools` style approach is not sufficient long-term.

The plugin should rely on JSON-based metadata and capability contracts.

### 8.1 Required metadata APIs

WbW-Py should expose runtime-friendly JSON methods such as:

1. `list_tool_metadata_json()`
2. `get_runtime_capabilities_json()`
3. `get_tool_capability_json(tool_id)`
4. `get_runtime_environment_json()`

### 8.2 Required metadata fields per tool

Each tool metadata record should include at minimum:

1. Tool ID.
2. Display name.
3. Category and subcategory.
4. Short description.
5. Parameter schema.
6. Input/output type hints.
7. Tier (`open` or `pro`).
8. Availability state (`available`, `locked`, `unsupported`, or similar).
9. Licensing/capability hints.
10. Output/report artifact hints.
11. Optional styling hints for QGIS rendering.

### 8.3 Capability state

Runtime capability JSON should include:

1. Installed WbW-Py version.
2. Runtime tier and entitlement status.
3. Pro availability state.
4. License mode (open, node-locked, floating, offline entitlement, etc.).
5. Environment health diagnostics.

---

## 9. Licensing and Tier Visibility

### 9.1 Runtime distribution model

The most practical model is a single WbW-Py runtime distribution that contains all tools, with runtime licensing determining what can execute.

This is acceptable if:

1. Enforcement happens in the runtime.
2. The plugin clearly communicates tier and locked state.
3. The user experience is respectful and filterable.

### 9.2 Visibility policy

Recommended visibility model:

1. Show all tools by default.
2. Clearly indicate Pro tools.
3. Clearly indicate when a Pro tool is locked under the current entitlement.
4. Provide filters such as:
   - available only
   - all tools
   - Pro tools only
   - hide locked tools

This supports both:

1. Marketing/discovery of Pro capabilities.
2. Reduction of annoyance for OSS-only users.

### 9.3 Locked tool behavior

Locked tools should:

1. Be visibly marked before execution.
2. Fail cleanly with informative messaging if run.
3. Explain why they are unavailable.
4. Offer upgrade or licensing guidance in the custom product UI.

---

## 10. Licensing Modes

### 10.1 Launch recommendation

Launch should assume local execution and a primarily local licensing posture.

Recommended launch order:

1. Open tier.
2. Local Pro entitlement / node-locked usage.
3. Floating-license support later if needed.

### 10.2 Floating licenses

Floating licenses may be valuable later, but they should not dominate the first launch architecture.

If supported later, the plugin must handle:

1. Session acquisition.
2. Lease renewal and expiry behavior.
3. Clear reconnect/failure messaging.
4. Explicit offline behavior.

At launch, local-machine entitlement is the simpler and safer assumption.

---

## 11. First-Run Experience and Environment Setup

The plugin should guide users through runtime setup on first run.

Recommended flow:

1. Plugin loads.
2. It checks for a compatible installed `whitebox-workflows` package in the QGIS Python environment.
3. If missing or incompatible, it offers guided installation or upgrade.
4. After successful setup, it queries metadata and builds the available tool catalog dynamically.

### 11.1 Required diagnostics UI

The plugin should include a runtime diagnostics or “environment doctor” screen that shows:

1. QGIS Python executable.
2. Installed WbW-Py version.
3. Plugin compatibility state.
4. License/capability status.
5. Last installation or runtime error.

This is essential for supportability.

---

## 12. Execution Model

### 12.1 Launch model

Execution should be local-only at launch.

This means:

1. The plugin executes tools on the user’s local machine.
2. No remote job infrastructure is assumed initially.
3. The plugin uses QGIS task/background infrastructure for non-blocking execution.

### 12.2 Future possibility

Remote or managed execution can be considered later for enterprise use cases, but it should not shape Phase 1 design.

### 12.3 Progress, message, and task event contract

The plugin must be able to consume runtime progress and message updates emitted by WbW-Py.

This should be treated as a first-class requirement.

Recommended event categories:

1. Progress events (percentage complete and optional stage information).
2. Informational status/message events.
3. Warning events.
4. Error events.
5. Final result/artifact events.

Recommended plugin behavior:

1. Map progress events to QGIS task progress updates.
2. Map short informational messages to task status and optional message bar notifications.
3. Stream richer message text to a Whitebox panel/log view.
4. Surface warnings without necessarily failing the run.
5. Use final result events to drive output loading, report display, and completion summaries.

Additional launch requirements:

1. Support graceful task cancellation initiated from QGIS.
2. Ensure event delivery is structured and machine-readable rather than plain unstructured log text.
3. Keep the event model stable enough that Processing and product-shell UI can share the same execution plumbing.

---

## 13. Output Loading and Styling

### 13.1 Launch baseline

The plugin should load raster and vector outputs into QGIS automatically where possible.

### 13.2 Styling strategy

The plugin should support light semantic styling from the start.

This means:

1. Use QGIS default rendering when no metadata hints exist.
2. Apply basic semantic styling when output metadata indicates meaningful value roles.

Examples:

1. Risk or suitability raster -> graduated color ramp.
2. Classified raster -> categorized palette.
3. Zone polygons -> categorized fill by class field.
4. Corridor/network outputs -> graduated line styling by score or cost field.

### 13.3 Required runtime hints

WbW-Py metadata should eventually support optional render hints such as:

1. `default_qgis_render_hint`
2. `value_semantics`
3. `recommended_color_ramp`
4. `geometry_render_role`

The goal is not full hand-crafted styling for every tool. The goal is better-than-default loading for major workflow outputs.

---

## 14. Tool Model: Registry Tools vs Object Methods

Not every WbW-Py capability should be represented as a one-to-one registry tool.

The plugin should support three categories:

1. Registry-backed tools.
2. Frontend-defined operations implemented through WbW-Py object methods.
3. Layer-context actions.

### 14.1 Registry-backed tools

These map directly to normal Whitebox tools and are ideal for the Processing provider.

### 14.2 Method-backed frontend operations

Some WbW-Py functionality exists primarily as operations on data objects, such as:

1. Raster reprojection.
2. Vector reprojection.
3. LiDAR reprojection.
4. Raster object methods such as `Con`.

These should still be exposed in the QGIS plugin, but they may be implemented as:

1. Processing algorithms backed by WbW-Py object-method logic.
2. Contextual actions on raster/vector/lidar layers.

They do not all need to become first-class backend registry tools immediately.

### 14.3 Layer-context actions

The plugin should eventually provide context-aware actions for selected layer types, especially for common object-method workflows.

---

## 15. Recommended UX Model

The plugin should be intentionally split into two user experiences:

### 15.1 Processing provider

Primary responsibilities:

1. Broad tool access.
2. Batch execution.
3. Model-builder compatibility.
4. Searchable native QGIS integration.

### 15.2 Whitebox product shell

Primary responsibilities:

1. Environment setup and diagnostics.
2. Licensing and entitlement state.
3. Featured workflows and bundle discovery.
4. Locked-tool explanations and upgrade prompts.
5. Output/report management.

This hybrid model best exposes the full Whitebox ecosystem without forcing all behavior into a custom panel.

---

## 16. Current Assessment of WbW-Py Readiness

WbW-Py is ready enough to begin plugin architecture and Phase 0/1 work.

It is not yet fully complete for final plugin delivery.

This is acceptable and expected.

Why now is the right time:

1. The plugin requirements should help shape the next evolution of WbW-Py.
2. Metadata, capability discovery, and styling contracts should be designed intentionally rather than retrofitted later.
3. Waiting for “API stability” risks stabilizing the wrong surface.

Conclusion:

1. Start planning and foundational development now.
2. Evolve WbW-Py and the plugin together.

---

## 17. Phased Implementation Plan

### Phase 0: Architecture and Contracts

Deliverables:

1. JSON metadata schema for tools.
2. JSON capability/licensing schema.
3. Runtime/environment diagnostics contract.
4. Plugin bootstrap/install flow specification.
5. QGIS 4 compatibility baseline.
6. Registry-tool vs method-tool model definition.

### Phase 1: Minimal Processing Provider

Deliverables:

1. Plugin bootstrap.
2. Dynamic tool discovery from WbW-Py metadata.
3. Processing provider registration.
4. Basic execution through QGIS task infrastructure.
5. Automatic output loading with default styling.
6. Locked Pro tool labeling and clean failure messaging.

### Phase 2: Product Shell and Setup UX

Deliverables:

1. Whitebox dock/panel.
2. Runtime diagnostics screen.
3. Install/upgrade flow for WbW-Py.
4. License state UI.
5. Refresh tool catalog action.
6. Report/artifact viewer integration.

### Phase 3: Semantic Styling and Context Actions

Deliverables:

1. Metadata-driven light semantic styling.
2. Contextual layer actions.
3. Method-backed plugin operations such as reprojection and raster conditionals.
4. Better post-run UX and artifact loading.

### Phase 4: Advanced Commercial / Enterprise Features

Deliverables:

1. Floating-license support if required.
2. Expanded entitlement UX.
3. Enhanced admin and audit reporting surfaces.
4. Optional enterprise deployment refinements.

---

## 18. Tool Help Documentation Strategy

### Background

The legacy `whitebox_workflows` codebase followed a **one-tool-one-file** Rust design. Each file contained a single `#[pymethods]` block with a rich `///` doc comment directly above the tool function. These Rust doc comments became Python docstrings accessible via `inspect.getdoc()`, and were the source material for the ~515 static HTML files in the old `wbw_qgis/help/` directory.

The new `whitebox_next_gen` codebase uses **consolidated, category-level files** (e.g. `basic_terrain_tools.rs`, `curvature_tools.rs`). This trades compilation speed for file count but produces very large files — `wb_environment.rs` alone is ~30,000 lines. The dispatch methods in that file currently carry no `///` doc comments, so `inspect.getdoc()` returns `None` for essentially every tool and the plugin `help.py` falls back to sparse stub pages (one-liner summary + parameter table).

### Design Tension

Adding full per-tool `///` documentation blocks to the consolidated files is the architecturally clean solution, but it would expand already large files dramatically and create a maintenance burden. The one-tool-one-file approach solves that at the cost of compile time, which is a real constraint for a large Rust project.

There is no perfect answer. The strategy below manages this tension pragmatically.

### Hybrid Help Documentation Strategy (Adopted)

**Tier 1 — Legacy tools (immediate):** Bundle the existing 515 HTML files from `whitebox_workflows/wbw_qgis/help/` as a static asset alongside the plugin (`whitebox_workflows_qgis/help_static/`). Update `help.py` to prefer a bundled file by `tool_id` lookup and fall back to the dynamically-generated stub only when no bundled file exists. This gives full, high-quality help for all legacy-era tools today without any new writing work.

**Tier 2 — New tools (ongoing):** For tools added to `wbtools_oss` or `wbtools_pro` after the legacy migration, write a standalone `help/<tool_id>.html` in the plugin source tree at authoring time. No requirement to backfill `///` docstrings into the large Rust source files.

**Tier 3 — Future structural consideration:** If the consolidated-file approach is ever revisited (e.g. per-category feature modules with separate help modules), a proper `///` docstring layer becomes viable again. This is not planned but the door is open.

### Implementation Notes for `help.py`

- `get_help_url(tool_id)` should check `help_static/` bundled files first, then the user cache, then fall back to generating from the manifest.
- `generate_help_files()` should skip tools that already have a bundled file unless `force=True`.
- The bundled `help_static/` directory should be committed to the plugin source tree (not gitignored) so it ships with the plugin on installation.
- When a WbW-Py upgrade adds new tools not covered by bundled HTML, the generated stub is acceptable as a temporary placeholder until a handcrafted file is added.

---

## 19. Immediate WbW-Py Work Items Triggered By This Plan

To support the plugin, WbW-Py should prioritize:

1. JSON tool metadata export.
2. JSON runtime capability export.
3. Machine-readable parameter schemas.
4. Tool tier and availability state exposure.
5. Output/render hint metadata.
6. Stable execution/result envelope for the plugin.
7. Environment health/diagnostics hooks.

---

## 20. Non-Goals For Initial Delivery

The first plugin release should not attempt to solve everything.

Not initial goals:

1. Full remote execution infrastructure.
2. Perfect handcrafted styling for every output.
3. Fully symmetric QGIS 3 and 4 feature parity.
4. Turning every WbW-Py object method into a first-class backend tool.

---

## 21. Summary

This plan treats the QGIS plugin as a top-tier product surface for the Whitebox ecosystem.

The key design commitments are:

1. QGIS 4 first.
2. Processing provider as the backbone.
3. WbW-Py as the canonical runtime contract.
4. Dynamic metadata-driven discovery.
5. Single runtime distribution with runtime licensing enforcement.
6. Local execution at launch.
7. Light semantic styling from the start.

If executed well, this architecture should provide:

1. Broad exposure of the Whitebox tool ecosystem inside QGIS.
2. A clean upgrade path as WbW-Py evolves.
3. A credible OSS-to-Pro conversion surface.
4. A plugin design worthy of the strategic importance of this channel.