# R API Parity Plan (wbw_r)

This document tracks staged parity between `wbw_python` and `wbw_r`.

## Scope

Parity is defined in three layers:

1. Coverage parity: every visible tool manifest is callable from R.
2. Runtime parity: licensing/tier behavior matches Python paths.
3. Ergonomic parity: idiomatic data/object wrappers and convenience helpers.

## Milestones

### M1: Generated coverage parity (in progress)

Goal: make all currently visible tools callable from R with minimal manual work.

- [x] Add runtime/API support for tool-wrapper stub generation.
- [x] Add `generate_r_wrapper_module_with_options(include_pro, tier)`.
- [x] Add tests validating generated wrapper module shape.
- [x] Add script/workflow to materialize generated wrappers into package files (`examples/generate_r_wrappers.rs`).
- [x] Add parity gate test that fails when generated wrapper count/names drift from visible manifests.
- [x] Add sourceable thin facade over generated wrappers for stable user entry points.
- [x] Add package scaffold with DESCRIPTION/NAMESPACE/R sources consuming generated wrappers.
- [x] Add native R binding/export layer via extendr in the Rust crate.
- [x] Add staged development sync workflow that builds and copies a loadable native library into the package scaffold.
- [ ] Add final package-native install/load workflow (no manual/staged dylib sync step).

### M2: Runtime/licensing parity (mostly complete)

Goal: align runtime bootstrapping and licensing behavior with Python.

- [x] Tier-only runtime options.
- [x] Signed entitlement runtime construction.
- [x] Provider bootstrap with fail-open/fail-closed.
- [x] Floating license activation path.
- [x] Legacy-style floating helper (`whitebox_tools(...)`).
- [ ] End-to-end R-facing integration tests against provider fixtures.

### M3: Ergonomic parity (pending)

Goal: provide object/data APIs comparable to Python (`WbEnvironment` style).

- [ ] Define idiomatic R environment/session object shape.
- [ ] Add structured argument conversion APIs (avoid JSON strings in user-facing layer).
- [ ] Add raster/vector/lidar object wrappers in the R package layer.
- [ ] Add progress/callback ergonomics.

### M4: Docs and examples parity (pending)

Goal: docs and examples stay in-step with callable/API surface.

- [ ] Add generated-tool usage docs for R package users.
- [ ] Add parity smoke examples for representative tool categories.
- [ ] Add migration notes for legacy R users.

## Short-term execution order

1. Land generated wrappers into the R package layer (M1).
2. Add parity CI checks (M1/M2).
3. Replace the staged dev sync path with a final package-native install/load workflow.
4. Expand docs/examples in lockstep with each milestone (M4).
