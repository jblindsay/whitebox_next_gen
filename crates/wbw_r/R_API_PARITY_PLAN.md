# R API Parity Execution Plan (wbw_r)

This document is the implementation checklist for bringing `wbw_r` close to `wbw_python` parity while preserving R idioms.

## 1) Current Status Snapshot

Parity is tracked in four layers.

| Layer | Definition | Current status | Estimate |
|---|---|---|---|
| Coverage parity | Visible tools are callable from R | Largely in place via generated wrappers + facade | 90-95% |
| Runtime parity | Licensing/tier behavior aligns with Python | Mostly in place (open, entitlement, floating paths) | 80-90% |
| Ergonomic parity | Typed data-object workflows (raster/vector/lidar/sensor bundles) | Raster/vector/lidar wrappers now exist; raster convenience accessors (`file_path`, `band_count`, `active_band`, `crs_epsg`, `crs_wkt`) and write/copy methods added to raster and vector objects; sensor bundle key-list/read helpers plus preview and true/false-colour methods with SAR-family default channel tuning; broader operator parity remains | 85-92% |
| Docs/examples parity | README/example structure and practical workflows | Improved with object quickstarts for raster/vector/lidar/sensor bundles including preview/composite helper documentation, but advanced multi-family examples are still thinner than Python | 78-86% |

## 2) Definition of Done for "Practical Parity"

R parity is considered practically complete when:

1. Typical Python README workflows have direct R equivalents (not necessarily identical naming).
2. Raster, vector, lidar, and sensor-bundle workflows are represented with typed wrappers in R package user API.
3. Licensing behavior and failure modes are integration-tested in R against provider fixtures.
4. README and examples can show object-centric workflows rather than generic tool JSON calls.

## 3) Execution Phases

## Phase A: Baseline and Governance (1 week)

Goal: lock scope and prevent drift while parity work proceeds.

### Tasks

- [x] Add parity status table in `crates/wbw_r/README.md` linking to this plan.
- [x] Add API mapping table (Python -> R target) for high-level surfaces.
- [x] Add local wrapper coverage count/name parity gate command.
- [x] Add local package install + R smoke test command.

### File targets

- `crates/wbw_r/R_API_PARITY_PLAN.md`
- `crates/wbw_r/README.md`
- `crates/wbw_r/r-package/whiteboxworkflows/tests/testthat/test-runtime.R`
- Local workflow documentation and helper scripts under `crates/wbw_r/` (no hosted CI requirement)

### Acceptance criteria

- Mapping table merged and reviewed.
- Local parity gate commands are documented and can be run before merge/release to catch wrapper and package regressions.

## Phase B: Typed Object Foundation (2-3 weeks)

Goal: add minimal typed wrappers so users can work with data objects, not just generic `run_tool`.

### Tasks

- [x] Introduce R `wbw_raster` wrapper type and constructor/read path.
- [x] Introduce R `wbw_vector` wrapper type and constructor/read path.
- [x] Introduce R `wbw_lidar` wrapper type and constructor/read path.
- [x] Add `print.*` methods and basic metadata access for each wrapper.
- [x] Ensure wrappers are session-aware and compose with existing tool execution helpers.

### File targets

- `crates/wbw_r/r-package/whiteboxworkflows/R/facade.R`
- `crates/wbw_r/r-package/whiteboxworkflows/R/native.R`
- `crates/wbw_r/src/lib.rs`
- `crates/wbw_r/r-package/whiteboxworkflows/NAMESPACE`
- `crates/wbw_r/r-package/whiteboxworkflows/tests/testthat/test-runtime.R`

### Acceptance criteria

- `read_*` style object entry points exist for raster/vector/lidar.
- Each wrapper supports metadata access and human-readable print method.
- Smoke tests cover object creation + metadata path for all three data types.

## Phase C: Reader/Writer and Workflow Parity (2-3 weeks)

Goal: make core day-to-day R workflows match Python capability.

### Tasks

- [x] Add object-centric read/write helpers that wrap existing runtime behavior.
- [x] Add representative raster/vector/lidar examples in package `inst/examples`.
- [x] Ensure progress helper works naturally with object-centric tool calls.
- [x] Add robust argument conversion at facade layer to reduce user JSON friction.

### File targets

- `crates/wbw_r/r-package/whiteboxworkflows/R/facade.R`
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/`
- `crates/wbw_r/r-package/whiteboxworkflows/tests/testthat/test-runtime.R`

### Acceptance criteria

- README-quality examples for raster/vector/lidar can run with typed API.
- Package tests validate at least one object-centric workflow per type.

## Phase D: Sensor Bundle Parity (2-3 weeks)

Goal: bring R closer to Python sensor bundle ergonomics.

### Tasks

- [x] Define R sensor bundle wrapper (`wbw_sensor_bundle`) with family metadata.
- [x] Add bundle read/open helper(s).
- [x] Add band-key discovery and band read helper methods.
- [x] Add at least one bundle composite workflow example.
- [x] Add raster convenience accessors: `file_path()`, `band_count()`, `active_band()`, `crs_epsg()`, `crs_wkt()`.
- [x] Add `deep_copy()`/`write()` to `wbw_raster` and `wbw_vector`.

### File targets

- `crates/wbw_r/r-package/whiteboxworkflows/R/facade.R`
- `crates/wbw_r/src/lib.rs`
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/`
- `crates/wbw_r/r-package/whiteboxworkflows/tests/testthat/test-runtime.R`

### Acceptance criteria

- Sensor bundle overview example exists in R analogous to Python intent.
- Bundle metadata and band access are covered by tests.

## Phase E: Runtime/Licensing Integration Hardening (1-2 weeks)

Goal: close the remaining runtime parity risk by testing real flows.

### Tasks

- [x] Add R-facing integration tests for entitlement startup (constructor guards + invalid entitlement failure path covered in testthat).
- [x] Add R-facing integration tests for floating activation startup (floating startup failure paths covered in testthat).
- [x] Add tests for fail-open vs fail-closed policy behavior (skip-guarded for non-pro builds; activate automatically in pro builds via WBW_LICENSE_POLICY env var).
- [x] Add failure-path assertions for missing provider URL/unreachable provider and key mismatch.
- [ ] Add fixture-backed happy-path licensing startup tests (entitlement and floating) for environments with provider fixtures.

### File targets

- `crates/wbw_r/src/lib.rs`
- `crates/wbw_r/r-package/whiteboxworkflows/tests/testthat/`
- provider fixture/test harness files used in current workspace

### Acceptance criteria

- Integration tests pass in CI environment with fixtures.
- Known licensing regressions are covered by test cases.

## Phase F: Docs and Example Parity Completion (1 week)

Goal: align R docs format and runnable examples with Python-level clarity.

### Tasks

- [x] Finalize README section ordering to mirror Python flow where appropriate.
- [x] Add explicit parity status table and "not yet implemented" markers where needed (raster operators, vector attribute table, lidar point array).
- [x] Add migration notes from legacy generated-wrapper usage to typed object API.
- [x] Ensure examples listed in README exist and execute (guarded placeholder-path examples with file.exists checks).

### File targets

- `crates/wbw_r/README.md`
- `crates/wbw_r/r-package/whiteboxworkflows/README.md`
- `crates/wbw_r/r-package/whiteboxworkflows/inst/examples/`

### Acceptance criteria

- R README and package README have consistent narrative and runnable examples.
- No README examples reference missing APIs.

## 4) Python-to-R Mapping Matrix (Initial)

This matrix defines target parity intent, not necessarily identical names.

| Python concept | Current R state | Target R state |
|---|---|---|
| `WbEnvironment()` | `wbw_session()` available | Keep session pattern; document as primary equivalent |
| `list_tools()` | `wbw_list_tools()`, `wbw_tool_ids()` | Keep; add richer category/discovery helpers |
| `search_tools()`, `describe_tool()` | partial via low-level tool listing metadata | add user-facing helpers in facade |
| `read_raster()` object workflow | typed `wbw_read_raster(...)` wrapper with `file_path()`, `band_count()`, `active_band()`, `crs_epsg()`, `crs_wkt()`, `deep_copy()`, `write()`, `to_array()`, `to_stars()` | add raster arithmetic/math operator parity |
| `read_vector()` object workflow | typed `wbw_read_vector(...)` wrapper with `deep_copy()`, `write()`, `to_terra()`, `to_sf()` | expand attribute helpers; richer write-format control |
| `read_lidar()` object workflow | typed `wbw_read_lidar(...)` wrapper present with metadata and file-backed copy/write | expand to fuller lidar object parity as native object support grows |
| sensor bundle workflow | `wbw_read_bundle(...)` wrapper now includes key-list/read helpers plus `read_preview_raster()`, `write_true_colour()`, and `write_false_colour()`, with SAR measurement-first defaults | expand to broader multi-family validation and richer examples |
| NumPy roundtrip helpers | R analogue via `terra`/`stars` bridges | keep and expand with typed wrapper integration |
| progress callbacks | structured progress return helper exists with `on_progress` callback ergonomics | complete |
| licensing startup variants | open/entitlement/floating supported | complete with integration hardening + fixture-backed skip-guarded tests |

## 5) Milestone Progress Tracker

- [x] M1 coverage scaffolding mostly complete (generated wrappers + facade + package scaffold).
- [x] M2 runtime licensing core mostly complete.
- [x] M3 ergonomic object parity delivered for core raster/vector/lidar/sensor-bundle workflows and raster math convenience methods.
- [x] M3 ergonomic object parity final polish: S3 arithmetic operators (`+`, `-`, `*`, `/`) for `wbw_raster` implemented; `on_progress` callback added to `wbw_run_tool_with_progress`.
- [x] M4 docs/examples parity completion: READMEs updated with not-yet-implemented markers, example scripts guarded, Tests section updated with current 140+ baseline.

## 6) Immediate Next 10 Tasks (Execution Queue)

1. [x] Add raster arithmetic/math operator convenience methods (`r$add(other)`, `r$subtract()`, `r$multiply()`, `r$divide()`) matching Python `Raster.add()` etc. Extended with unary conveniences (`abs`, `ceil`, `floor`, `round`, `square`, `sqrt`, `log10`, `log2`, `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `exp`, `exp2`).
2. [x] Add API mapping table (Python -> R target) into `crates/wbw_r/README.md` or linked docs.
3. [x] Extend SAR composite defaults to cover additional key aliases from provider-specific naming. Implemented via `wbw_bundle_enhance_candidates()` function that probes available bundle keys and intelligently expands channel candidate lists.
4. [x] Add optional fixture-discovery helper docs for WBW_TEST_DATA_ROOT and expected local layout.
5. [x] Add local package install + R smoke commands.
6. [x] Add local wrapper coverage count/name parity command.
7. [x] Add R-facing integration tests for entitlement startup (guard/failure paths).
8. [x] Add R-facing integration tests for floating activation startup (failure paths).
9. [x] Add failure-path assertions for licensing and provider errors.
10. [x] Add tests for fail-open vs fail-closed policy behavior (skip-guarded in non-pro builds, active in pro builds).

## 7) Risks and Mitigations

- Risk: generated-wrapper path and typed facade drift apart.
  - Mitigation: keep generated wrappers low-level and facade as stable contract with tests.
- Risk: README over-promises before object APIs exist.
  - Mitigation: require example-exists + example-runs checks before README additions.
- Risk: licensing regressions across startup modes.
  - Mitigation: provider fixture integration tests + explicit denial-path tests.

## 8) Reporting Cadence

Use this cadence while parity execution is active:

- Weekly: update completion percentage per phase.
- Per merged PR: update "Milestone Progress Tracker" and remove completed items from "Immediate Next 10 Tasks".
- On major API additions: add example + test in same PR.
