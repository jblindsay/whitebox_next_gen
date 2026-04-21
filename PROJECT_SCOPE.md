# Project Scope And Operating Constraints

Use this file as the canonical big-picture reference for Whitebox Next Gen work.
Update this file whenever priorities, architecture, or non-negotiables change.

## Mission
- Keep Python, R, and QGIS frontends aligned with the same canonical tool taxonomy.
- Prevent category churn and disappearing tools across frontends.
- Keep Pro-enabled behavior available for integration and platform testing.

## Non-Negotiables
- Always build wbw_python with Pro enabled when doing platform integration/testing.
- Use tool_taxonomy.toml as the single source of truth for category/subcategory membership.
- Keep RuntimeSession catalog, taxonomy TOML, and frontend exports in sync.

## Canonical Sources
- Taxonomy source: crates/wbw_python/tool_taxonomy.toml
- Python runtime + catalog: crates/wbw_python/src/lib.rs
- Python env/category helpers: crates/wbw_python/src/wb_environment.rs
- QGIS plugin source: crates/wbw_qgis/plugin/whitebox_workflows_qgis/
- R package taxonomy export: crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/

## Current Design Decisions
- QGIS grouping should follow resolved taxonomy JSON, not ad hoc heuristics.
- true_colour_composite and false_colour_composite must be visible in runtime catalog and taxonomy.
- Taxonomy sync should regenerate both QGIS and R resolved JSON exports.

## Active Priorities
- Priority 1:
- Priority 2:
- Priority 3:

## Out Of Scope (For Now)
- 

## Safety And Release Constraints
- Sensitive/private repositories and licensing-related code should never be pushed publicly.
- Prefer small checkpoint commits for risky changes.

## Pre-Work Alignment Checklist
Before coding, restate:
1. Mission and immediate goal
2. Non-negotiables that apply
3. Files/systems in scope
4. Validation plan

## Session Notes (Optional)
- 
