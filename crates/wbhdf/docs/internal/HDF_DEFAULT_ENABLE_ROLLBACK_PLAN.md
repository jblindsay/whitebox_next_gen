# HDF Default-Enable Rollback Plan

## Purpose

This runbook defines how to quickly re-apply temporary stabilization guardrails if a
future default-enable decision for HDF integration in `wbraster` or `wblidar` causes
high-severity regressions.

It is intentionally operational: it prioritizes a fast, low-risk rollback path first,
then a clean follow-up diagnosis path.

## Trigger Conditions

Run this rollback when any of the following occurs after default-enable rollout:

- non-HDF raster read/write behavior regresses in production or release-candidate checks,
- HDF URI reads in `wbraster` produce unstable behavior not covered by current staged scope,
- `wblidar` HDF canopy reads show repeatable unsupported-layout/filter failures beyond known limits,
- failure triage cannot be completed within the release window.

## Rollback Modes

### Mode A: Fast Operational Guardrail (Preferred)

Use when immediate stabilization is required.

1. Temporarily disable HDF dataset-URI dispatch in `wbraster`:
   - target: `crates/wbraster/src/raster.rs`
   - behavior to gate: early path in `Raster::read(...)` that calls
     `crate::formats::is_hdf_dataset_uri(...)` and `crate::formats::read_hdf_dataset_uri(...)`.
2. Keep all non-HDF format detection/read/write paths unchanged.
3. Keep HDF code compiled, but not active in default `Raster::read` dispatch.

Expected result:
- Non-HDF behavior returns to pre-default-enable risk profile.
- HDF paths remain available for targeted development tests.

### Mode B: Hard Scope Clamp (If Mode A Is Insufficient)

Use when Mode A does not fully stabilize behavior.

1. In `crates/wbraster/src/formats/mod.rs`, make HDF5/NetCDF URI materialization return an
   explicit staged-scope `RasterError::Other(...)` without attempting decode.
2. Keep HDF4 path behavior as-is unless incident triage shows it must also be disabled.
3. Preserve explicit error text so users receive deterministic, actionable messaging.

Expected result:
- HDF5/NetCDF dataset-URI reads fail fast and deterministically.
- No ambiguous fallback behavior.

## Verification Checklist After Rollback

Run these checks immediately after applying Mode A or B:

1. `cargo check -p wbraster`
2. `cargo test -p wbraster raster::tests::get_set -- --nocapture`
3. `cargo test -p wbraster raster::tests::statistics -- --nocapture`
4. `cargo test -p wbraster --test integration roundtrip_esri_ascii -- --nocapture`
5. `cargo test -p wbraster --test integration roundtrip_geotiff -- --nocapture`

Optional HDF sanity checks (to confirm guardrail behavior):

1. `cargo test -p wbraster raster_read_hdf -- --nocapture`
2. `cargo test -p wbhdf multilevel_internal_fanout -- --nocapture`

## Incident Logging Requirements

For every rollback event, record:

- rollback mode used (`A` or `B`),
- commit hash that introduced rollback,
- failing command(s) and error signatures that triggered rollback,
- post-rollback verification command results,
- follow-up owner/task for re-enable criteria.

## Re-enable Criteria

Do not re-enable default integration until all are true:

- failing incident class has a deterministic regression test,
- staged-scope diagnostics remain explicit for unsupported layouts,
- non-HDF regression matrix is green,
- readiness decision is updated in
  `crates/wbhdf/docs/internal/HDF5_SCOPED_READER_ROADMAP.md`.
