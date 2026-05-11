# buffer_vector Reimplementation Playbook (Crash-Resilient)

**Date:** 2026-05-11
**Owner:** current implementation session
**Status:** active

## Why This File Exists

VS Code instability can interrupt long geometry work. This playbook is the single restart point for:

- architecture decisions,
- implementation order,
- checkpoint commands, and
- frontend reinstatement steps.

If the session crashes, resume from this file first.

## Target Architecture (Non-Negotiable)

Replace dissolve buffering path:

```
per-feature polygon buffers -> global polygon union
```

with GEOS-style topology pipeline:

```
per-feature raw offset curves
-> global noding (all curves together)
-> planar graph build
-> face/subgraph labeling (inside/outside)
-> ring extraction (shells + holes)
```

Key principle: dissolve is not a separate union stage. Dissolve is a byproduct of face labeling on one global graph.

## Scope Boundaries

- In scope:
  - `wbtopology` BufferOp-style dissolve pipeline
  - `wbtools_oss` integration for `buffer_vector` dissolve path
  - Re-enable tool visibility in Python/R/QGIS frontends once stable
- Out of scope for first pass:
  - aggressive micro-optimizations before correctness parity
  - adding new user-facing parameters

## Implementation Order

1. **Scaffold BufferOp module in `wbtopology`**
   - Add orchestration struct/function for end-to-end run.
   - Keep API internal until parity checks pass.

2. **Curve generation stage**
   - Reuse existing cap/join/arc logic.
   - Emit raw offset linework for all features.
   - Do not close per-feature polygons in dissolve path.

3. **Global noding stage**
   - Node all generated curves in one run.
   - Validate split/intersection completeness on dense line cases.

4. **Planar graph stage**
   - Build directed graph/half-edge representation from noded lines.
   - Track edge direction and depth delta attributes needed for face traversal.

5. **Face labeling stage**
   - Implement BufferSubgraph-style inside/outside propagation.
   - Dissolve criterion: faces with depth >= 1 are inside.

6. **Ring extraction stage**
   - Extract shell/hole rings from labeled faces.
   - Assemble final polygons with valid topology.

7. **Integration into `buffer_vector` tool**
   - Route `dissolve=true` to new pipeline.
   - Keep old non-dissolve path available.

8. **Frontend reinstatement (after backend confidence)**
   - Re-add `buffer_vector` to curated taxonomy.
   - Regenerate synced taxonomy artifacts for Python/R/QGIS.

## Checkpoint Rhythm (Mandatory)

Use small checkpoints after each major stage above.

At each checkpoint:

1. Run `cargo check -p wbtopology`
2. Run `cargo check -p wbtools_oss`
3. Commit with stage-specific message, e.g.:
   - `buffer_vector: scaffold BufferOp orchestration`
   - `buffer_vector: add global curve noding stage`

Do not batch multiple stages into one large commit.

## Crash Recovery Protocol

After a crash:

1. Open this file.
2. Run:
   - `git status --short`
   - `git --no-pager log --oneline -n 12`
3. Resume at first unchecked item in "Progress Ledger" below.
4. Re-run both checks before new edits:
   - `cargo check -p wbtopology`
   - `cargo check -p wbtools_oss`

## Progress Ledger

- [x] Stage 1: BufferOp scaffold in `wbtopology`
- [x] Stage 2: raw curve generation wired for dissolve path
- [x] Stage 3: global noding over all curves
- [x] Stage 4: planar graph build from noded segments
- [x] Stage 5: face/subgraph depth labeling
- [x] Stage 6: ring extraction to polygons
- [x] Stage 7: `buffer_vector` dissolve integration in `wbtools_oss`
- [x] Stage 8: frontend reinstatement and taxonomy sync

Stage 7 note: pure line-input, pure polygon-input, and mixed line/polygon/point dissolve flows are now routed through `BufferOp` staging with final merged dissolve.

Stage 8 note: `buffer_vector` re-added to `wbw_python/tool_taxonomy.toml`, taxonomy sync applied, and regenerated Python/R/QGIS resolved taxonomy artifacts now include `buffer_vector`; `cargo check -p wbw_python` passes after regeneration.

Stage 5 note: BufferOp now uses graph edge depth-delta computation plus BFS propagation from exterior-adjacent faces to label inside faces (depth > 0).

Stage 6 note: depth-selected bounded face rings are now passed to polygonization/assembly prior to final dissolve; BufferOp regression coverage added in `crates/wbtopology/tests/buffer_op_pipeline_tests.rs` for line/polygon dissolve paths and stage-stat invariants.

## Frontend Reinstatement Steps

1. Re-add `buffer_vector` in:
   - `crates/wbw_python/tool_taxonomy.toml` under `vector.geometry_processing`.
2. Run:
   - `python scripts/sync_tool_taxonomy.py --apply`
3. Verify regenerated/synced artifacts include `buffer_vector`:
   - `crates/wbw_python/src/wb_environment.rs`
   - `crates/wbw_python/tool_taxonomy.resolved.json`
   - `crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/tool_taxonomy.resolved.json`
   - `crates/wbw_qgis/plugin/whitebox_workflows_qgis/tool_taxonomy.resolved.json`
4. Smoke-check discovery/API visibility in Python, R, and QGIS plugin listings.

## Primary Reference

Architectural rationale and postmortem baseline:

- `crates/wbtopology/docs/BUFFER_VECTOR_POSTMORTEM_AND_ROADMAP_2026-05-07.md`