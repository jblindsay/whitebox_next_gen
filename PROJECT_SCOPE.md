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
- Priority 1: Continue targeted Python stub typing cleanup to reduce unresolved `*args/**kwargs -> Any` placeholders in `crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi` while preserving conservative safety rules.
- Priority 2: Keep signature-rollout automation centralized in `scripts/rollout_stub_literals.py` (Literal rollout + placeholder fill modes) and prefer unambiguous-only replacement paths.
- Priority 3: Maintain taxonomy/runtime/frontend alignment checks after stub updates so Python/R/QGIS discovery/catalog behavior remains coherent.

## Out Of Scope (For Now)
- 

## Safety And Release Constraints
- Sensitive/private repositories and licensing-related code should never be pushed publicly.
- Prefer small checkpoint commits for risky changes.

## Dependency Governance Tiers

### Tier 1: Core Backend Crates (Strict)
- Scope: wbgeotiff, wbprojection, wbraster, wbvector, wblidar, wbtopology, wbcore, wblicense_core.
- Goal: Keep the core geospatial backend as pure-Rust as practical and dependency-light.
- Rule: No new `-sys` or `links` dependencies without explicit approval and documented rationale.
- Rule: Prefer pure-Rust codec/compression stacks when feature parity is acceptable.
- Rule: Avoid adding broad "default feature" dependency bundles unless required by user-facing behavior.
- Rule: Any Tier 1 dependency increase should include a short impact note (what was added, why needed, and alternatives considered).

### Tier 2: Interop And Product-Surface Crates (Constrained)
- Scope: wbtools_oss, wbtools_pro, wbw_python, wbw_r, wbw_qgis.
- Goal: Preserve full platform interoperability while containing heavy format/runtime dependencies at the edges.
- Rule: Heavier dependencies are allowed when they unlock required platform functionality (for example GeoParquet, Python/R interop, QGIS integration).
- Rule: Keep heavy dependencies from leaking inward into Tier 1 when a boundary API can isolate them.
- Rule: Prefer optional feature wiring in Tier 2 when it does not reduce required shipping functionality.
- Rule: Frontend/interoperability crates should treat Tier 1 purity constraints as upstream non-negotiables.

## Pre-Work Alignment Checklist
Before coding, restate:
1. Mission and immediate goal
2. Non-negotiables that apply
3. Files/systems in scope
4. Validation plan

## Session Notes (Optional)
- Scope refresh completed on 2026-04-23 after targeted stub-typing follow-up.
- Latest checkpoint commit: `d4836d77da4587e4d27a161fe4eeefd8ded74201`.
- `whitebox_workflows.pyi` placeholder count reduced from 88 to 63 using runtime-signature-derived fills.
- `scripts/rollout_stub_literals.py` now includes:
	- `--fill-any-from-existing`
	- `--fill-any-from-rust-signatures`
	- `--fill-any-from-runtime-signatures`
- Remaining placeholders are primarily ambiguous/missing-source cases and should be handled with manual high-confidence passes or additional source mapping.
