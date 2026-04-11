# WbW-Py Phase 1 Execution Checklist

This is a compact implementation checklist derived from the broader usability and capability plan.

## 1) API Idiom Consolidation

- [x] Publish canonical API style guide for preferred naming and patterns.
- [x] Document preferred metadata method usage (`metadata()` over legacy aliases).
- [x] Build compatibility alias inventory (preferred / compatibility / deprecate-later).
- [x] Remove highest-confusion redundant aliases while pre-release.
- [x] Add explicit list of intentionally retained temporary aliases.

## 2) Discovery and IntelliSense

- [x] Add explicit "preferred API conventions" section to README.
- [x] Improve `.pyi` inline guidance for key preferred methods and namespaces.
- [x] Add intent-driven quick links in README for common tasks.
- [x] Expand top-level namespace docstrings in stubs for tool categories and utility namespaces.

## 3) Interoperability Tightening

- [x] Write metadata-preservation matrix for NumPy/rasterio/GeoPandas/Shapely/xarray/pyproj bridges.
- [x] Add round-trip smoke tests for priority interop pathways.
- [x] Document copy-vs-view behavior and expected memory implications per conversion.
- [ ] Normalize argument naming across conversion helpers where feasible.

## 4) Happy-Path Standardization

- [x] Define 5 canonical workflows (raster, vector, lidar, reprojection, interop-first).
- [x] Provide one preferred end-to-end example per workflow.
- [x] Add "recommended vs advanced" notes beside option-dense examples.
- [ ] Add a checklist gate for new docs/examples: "does this follow the canonical path?"

## 5) WbW-R Parallelization Review

- [x] Add a WbW-R parity decision note to each substantial WbW-Py API change.
- [x] Maintain a parity ledger (`parallel now`, `parallel later`, `Python-only`).
- [x] Prioritize parallelization for naming/discovery/workflow convention changes.

## Near-Term Milestones

## Milestone A (current)

- [x] Create compact execution checklist.
- [x] Start Phase 1 documentation cleanup.
- [x] Add changelog entry for Phase 1 kickoff updates.

## Milestone B

- [x] Complete canonical style guide and compatibility inventory.
- [x] Add intent-based discovery sections.
- [x] Add first round of interop behavior documentation.
- [x] Complete initial WbW-R parity ledger for Phase 1/2 changes.

## Milestone C

- [x] Ship golden-path workflow examples.
- [x] Finalize pre-release alias removals for redundant paths.
- [ ] Reassess usability score after Phase 1/2 changes.