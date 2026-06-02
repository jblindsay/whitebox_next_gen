# Epoch-Aware Transform Rollout (Whitebox Next Gen)

Date: 2026-06-02
Status: CSRS matched-zone realization routing is operational across v2..v8 families under a mathematically-driven preferred-operation policy.

## Rollout Status Snapshot (2026-06-02)

Completed in this phase:
- Backend epoch-routing contract is wired through `wbprojection`, `wbraster`, `wbvector`, and `wblidar` reprojection options.
- CSRS preferred-operation routing now covers matched zones 7-24 for
  realization-to-realization corridors across v2..v8 families
  (excluding same-realization no-op pairs).
- `wbprojection` now exposes a programmatic CSRS support snapshot surface
  (`csrs_preferred_operation_support_snapshot`) so callers can query active vs
  same-realization no-op pairs and scoped zone bounds without reading internal docs.
- CSRS preferred-operation routing now uses a matrix-based helper in `wbprojection` (catalog-style scaffold) rather than a single hardcoded conditional, enabling phased realization-pair expansion.
- CSRS activation/pending policy is now data-driven (table-based) rather than
  hand-enumerated per zone pair.
- WbW-Python reprojection APIs (single + batch) now expose epoch-routing parameters:
  - `coordinate_epoch`
  - `source_reference_epoch`
  - `target_reference_epoch`
  - `operation_code`
  - `prefer_official_operation`
  - `epoch_policy`
- WbW-R manual now includes epoch-aware reprojection examples and argument guidance.
- WbW-QGIS reprojection wrappers now expose and forward epoch-routing parameters.
- Integration conformance tests now include preferred-vs-explicit and
  preferred-vs-baseline checks for active CSRS forward corridors, including
  newly activated `v6 -> v8` and `v7 -> v8` coverage slices.

Still pending:
- WbW-R typed facade/signature surfacing (beyond generic `run_tool` argument pass-through).
- End-to-end Python + R smoke examples with known dynamic-datum datasets.
- Broader corridor/catalog expansion beyond current CSRS prototype families and operation metadata.

## Checkpoint Resume Notes (2026-06-02)

If resuming this workflow later, start with:

1. Run `cargo check` at repo root.
2. Run targeted CSRS tests:
  - `cargo test -p wbprojection epsg_preferred_operation_csrs -- --nocapture`
  - `cargo test -p wbprojection transform_to_with_preferred_operation -- --nocapture`
  - `cargo test -p wbprojection transform_to_3d_with_preferred_operation -- --nocapture`
  - `cargo test -p wbprojection csrs_v6_to_v8 -- --nocapture`
  - `cargo test -p wbprojection csrs_v7_to_v8 -- --nocapture`
3. Review authoritative evidence docs:
  - `crates/wbprojection/docs/internal/AUTHORITATIVE_CHECKPOINT_SOURCES.md`
  - `crates/wbprojection/docs/internal/EPOCH_AWARE_CONFORMANCE_MATRIX.md`
4. For next CSRS expansion, prefer policy-table updates first, then add tests,
  then update Python/R/QGIS manuals in the same commit.

## Why This Document Exists

`wbprojection` now has additive epoch-aware and dynamic grid-shift capabilities, but platform adoption (WbW-Py/R, QGIS, raster/vector/lidar tools) requires a coordinated rollout plan. This document is the top-level implementation guide for that rollout.

## Current Core Status (`wbprojection`)

Implemented:
- Epoch context type (`TransformEpochContext`) and context-aware CRS transform APIs.
- Dynamic grid model support and dynamic datum transform variants.
- Explicit operation-code and preferred-operation transform entry points.
- Authoritative fixture ingestion tests for:
  - NRCan TRX workflow (public), and
  - Authenticated NRCan epoch propagation workflow (2010 -> 2020).

Known limitation:
- Direct external operation-10715 realization checkpoints (`NAD83(CSRS) v3 -> v8`) were not obtainable from the explored NRCan web UI.

## CSRS Realization Coverage: v3, v4, v5, v6, v7, v8

### What Is Implemented Today

Current preferred-operation mapping in `wbprojection` uses a broad
mathematically-driven corridor policy:
- Matched-zone UTM realization-to-realization transforms across v2..v8 use
  operation 10715 for zones 7-24 (same-realization pairs remain no-op/baseline).
- This routing uses an internal realization-pair matrix scaffold (catalog-style)
  covering v2-v8 families.

In other words, every non-identical matched-zone realization pair in v2..v8 is
active under preferred routing today.

This does not yet cover the broader active NAD83(CSRS) UTM family in the registry
(`2955–2962`, `3154–3160`, `3761`, `9709`, `9713`), which spans zones 7-24.

Projected-v5 note:
- The projected v5 UTM family (`225xx`) participates in the same matched-zone
  preferred-operation rule as other realization families.

### What Is Not Yet Implemented

- No broad operation-catalog ingestion exists yet for all CSRS realization pairings.

### What Adding/Activating Additional Corridors Requires

For each realization pair to be added, we need:
1. Validated source/target EPSG CRS pairs.
2. Preferred operation code(s) and method metadata.
3. Confirmed model assets (grid/velocity file or equivalent).
4. At least internal consistency tests; external authoritative checkpoints when available.

Acquisition guidance for pair-specific checkpoints (including date-only source limitations) is documented in:

- `crates/wbprojection/docs/internal/CSRS_PAIR_CHECKPOINT_ACQUISITION_PLAYBOOK.md`

Current anchor-epoch context (from EPSG metadata reviewed in this rollout):

- `NAD83(CSRS)v3`: anchor epoch 1997
- `NAD83(CSRS)v4`: anchor epoch 2002
- `NAD83(CSRS)v8`: anchor epoch 2010

These values help frame exploratory date-based checks, but they do not by themselves
replace explicit source/target realization evidence for pair activation.

## Should We Stay Use-Case-by-Use-Case?

Short answer: use a hybrid strategy.

- Infrastructure should be generalized once.
- Production activation should be corridor-by-corridor (use-case-by-use-case) based on authoritative data availability.

### Next Priority Order

For the next expansion phase, prioritize:

1. US epoch-aware corridors and realization catalogs that already have stable datum hooks in the engine.
2. European epoch-aware corridors and realization catalogs, starting with ETRS89-centered families.

This keeps the first follow-on scope aligned with the existing engine surfaces while keeping the rollout incremental and testable.

Why:
- Global dynamic-datum coverage is large and uneven by data availability.
- Authoritative checkpoints and file-distribution policies vary by country/agency.
- A corridor-based activation model prevents overclaiming support while enabling steady expansion.

## Broader Regional Rollout Strategy (Beyond Canada)

## Phase A: Generalized Infrastructure (one-time)

1. Operation catalog model
- Add a structured preferred-operation catalog (source CRS, target CRS, operation code, method, epoch assumptions, provenance).

2. Dataset capability registry
- Record which dynamic models/files are installed and usable at runtime.

3. Policy engine
- Route by policy:
  - preferred operation when catalog + assets available,
  - deterministic fallback otherwise,
  - explicit warning/error modes for missing epoch context.

4. Fixture pipeline
- Standard fixture schemas for external checkpoints by region/provider.

## Phase B: Regional Activation (repeatable)

For each region/provider:
1. Ingest operation metadata.
2. Attach required assets.
3. Add corridor tests (internal consistency).
4. Add authoritative checkpoints where available.
5. Mark support level explicitly in docs.

Suggested order:
1. Canada expansion (CSRS v4-v7 corridors where metadata/assets are available).
2. Other regions with mature dynamic datum infrastructure and accessible authoritative data.

## Platform Exposure Plan

## WbW-Python / WbW-R (first)

Expose optional parameters on reprojection workflows:
- `coordinate_epoch`
- `source_reference_epoch` (optional)
- `target_reference_epoch` (optional)
- `operation_code` (advanced/optional)
- `prefer_official_operation` (default true when catalog entry exists)

Behavior:
- If dynamic transform requires epoch and none is provided, return a clear error (or explicit opt-in fallback mode only).

### Concrete API Contract (Phase 1)

The first platform exposure should be a thin, consistent parameter contract across
WbW-Python and WbW-R that maps directly to `TransformEpochContext`.

#### Shared semantic fields

1. `coordinate_epoch`
- Decimal year (e.g. `2010.0`, `2024.5`).
- Represents the observation/coordinate epoch of input coordinates.

2. `source_reference_epoch`
- Optional decimal year.
- Normally omitted unless caller has explicit source datum reference epoch context.

3. `target_reference_epoch`
- Optional decimal year.
- Normally omitted unless caller has explicit target datum reference epoch context.

4. `operation_code`
- Optional integer.
- Advanced override to request a specific operation code where supported.

5. `prefer_official_operation`
- Boolean, default `true`.
- If true, try preferred-operation routing for recognized CRS pairs.

6. `epoch_policy`
- Enum/string.
- Initial values:
  - `strict` (default): error when a dynamic transform needs epoch context but none is provided.
  - `allow_static_fallback`: permit explicit fallback where implemented and documented.

#### WbW-Python signature shape (target)

Keep existing reprojection entry points and add optional keyword arguments:

```python
def reproject_raster(
    input,
    output,
    target_crs,
    *,
    coordinate_epoch: float | None = None,
    source_reference_epoch: float | None = None,
    target_reference_epoch: float | None = None,
    operation_code: int | None = None,
    prefer_official_operation: bool = True,
    epoch_policy: str = "strict",
    **kwargs,
):
    ...
```

Apply the same optional kwargs to:
- `reproject_vector(...)`
- `reproject_lidar(...)`

#### WbW-R signature shape (target)

Keep existing function names and add optional named arguments:

```r
reproject_raster <- function(
  input,
  output,
  target_crs,
  coordinate_epoch = NULL,
  source_reference_epoch = NULL,
  target_reference_epoch = NULL,
  operation_code = NULL,
  prefer_official_operation = TRUE,
  epoch_policy = "strict",
  ...
) {
  # ...
}
```

Apply the same optional arguments to:
- `reproject_vector(...)`
- `reproject_lidar(...)`

#### Backend tool parameter contract (Rust)

Define one shared internal struct and reuse it in raster/vector/lidar reprojection tools:

```rust
pub struct EpochTransformOptions {
    pub coordinate_epoch: Option<f64>,
    pub source_reference_epoch: Option<f64>,
    pub target_reference_epoch: Option<f64>,
    pub operation_code: Option<u32>,
    pub prefer_official_operation: bool,
    pub epoch_policy: EpochPolicy,
}

pub enum EpochPolicy {
    Strict,
    AllowStaticFallback,
}
```

Normalization rule:
- If any epoch field is provided, validate all provided values are finite decimal years.
- Build `TransformEpochContext` only when `coordinate_epoch` is present.
- If dynamic route requires context and `coordinate_epoch` is missing:
  - `Strict` -> return error.
  - `AllowStaticFallback` -> only fallback where supported, with explicit warning/trace.

#### Routing precedence

When transforming a coordinate with these options:

1. If `operation_code` is provided -> call explicit operation route.
2. Else if `prefer_official_operation` is true -> call preferred-operation route.
3. Else -> call standard CRS route.
4. In all cases, pass epoch context if constructed.

#### Error messages (user-facing requirements)

Messages should be explicit and actionable:
- Missing epoch for dynamic transform:
  - "This transform requires coordinate_epoch for dynamic datum/velocity-grid evaluation."
- Operation mismatch:
  - "Requested operation_code is not valid for source/target CRS pair."
- Missing assets:
  - "Required dynamic grid dataset is unavailable in this runtime."

#### Traceability requirement

All reprojection tools should emit (log/trace/metadata where available):
- source CRS,
- target CRS,
- selected routing mode (`explicit`, `preferred`, `standard`),
- operation code when used,
- epoch context values used.

This is required for reproducibility and auditability.

## QGIS Frontend (second)

UI additions (advanced section):
- Coordinate epoch field
- Optional source/target reference epoch fields
- Operation routing mode: Auto (preferred), Explicit code, Standard fallback

Important:
- Keep defaults simple; expose advanced controls only when relevant CRS pairs are detected.

## Raster / Vector / LiDAR Tool Impact

### Raster reprojection
- Usually one epoch per dataset is sufficient.
- Main change: carry epoch metadata into CRS transform calls.

### Vector reprojection
- Layer-level epoch first.
- Future extension: optional per-feature epoch attribute.

### LiDAR reprojection (highest value)
- Most sensitive to epoch-aware transforms.
- Leverage acquisition date/time metadata when present.
- Priority target for first full platform integration.

## Recommended Near-Term Execution (Next 3 Milestones)

1. Platform API design freeze
- Finalize shared epoch-aware parameter names across WbW-Py/R and backend tool interfaces.

2. Backend integration in reprojection toolchain
- Thread epoch context through raster/vector/lidar reprojection tool internals.

3. Frontend exposure
- Add WbW-Py/R surface first, then QGIS advanced controls.

Current milestone status:
- Milestone 1: in progress (Python complete; R typed-facade follow-up pending).
- Milestone 2: complete for raster/vector/lidar backend option threading.
- Milestone 3: in progress (QGIS controls pending).

## Definition of Done for Platform Rollout

Minimum acceptable completion:
1. Epoch-aware parameters are exposed in WbW-Py/R for reprojection workflows.
2. Raster/vector/lidar reprojection tools pass epoch context to `wbprojection` when provided.
3. LiDAR reprojection path has explicit validation coverage.
4. Documentation clearly states where support is authoritative vs internal-consistency only.
