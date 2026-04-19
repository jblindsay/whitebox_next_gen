# JPEG2000 Bridge Retirement Plan

Status: Draft (execution-ready)
Owner: wbraster core team
Last updated: 2026-04-19

## Objective
Retire the feature-gated vendored bridge decode path in wbraster and run JPEG2000 decode on native jpeg2000_core by default, with no regression for production GIS scenes.

## Scope
- In scope:
  - Native decode parity for multicomponent JP2.
  - Native support for POC progression changes and PPM/PPT packet-header workflows.
  - Differential/corpus validation against current bridge behavior.
  - Feature-default flip and bridge-path removal in wbraster.
- Out of scope:
  - Rewriting wbjpeg2000 crate internals.
  - Non-JPEG2000 format work.

## Timeline Estimates
- Best case: 6-9 engineering weeks
- Conservative: 8-12 engineering weeks
- Main uncertainty: POC plus PPM/PPT edge-case coverage

## Roles
- Owner: TBD
- Reviewer: TBD
- QA/validation: TBD
- Release lead: TBD

## Readiness Gates
- Gate A: Native multicomponent decode passes corpus parity checks.
- Gate B: No native NotImplemented for targeted POC and PPM/PPT fixture classes.
- Gate C: Default build runs native decode path with acceptable regression metrics.
- Gate D: Bridge path removed from wbraster runtime path.

## Phase Plan

### Phase A - Multicomponent Native Decode (2-4 weeks)
Targets:
- crates/wbraster/src/formats/jpeg2000_core/reader.rs
- crates/wbraster/src/formats/jpeg2000.rs

Tasks:
- [ ] Implement native multicomponent decode path (remove current fail-fast guard for supported codestreams).
- [ ] Validate component ordering, signedness, and bit-depth handling.
- [ ] Add/expand multiband fixtures and expected outputs.
- [ ] Add parity assertions against bridge-backed baseline corpus.

Exit criteria:
- [ ] Native multicomponent fixtures pass.
- [ ] Differential mismatch rate is within agreed epsilon/threshold.
- [ ] No regressions in existing JPEG2000 tests.

#### Phase A kickoff workboard (function-level)

Sprint target: remove multicomponent fail-fast for supported codestreams while preserving existing single-band behavior.

1. A1 - Decode dispatch and capability gate cleanup (0.5-1 day)
  - [x] Update `decode_component` to route supported multicomponent flows into native packet path.
  - [ ] Keep explicit fail-fast for still-unsupported marker workflows during Phase A.
  - [ ] Location: `reader.rs` function `decode_component`.
  - Done when:
  - [ ] Existing fail-fast tests still pass for unsupported workflows.
  - [ ] Supported multicomponent fixture no longer trips generic multicomponent NotImplemented.

2. A2 - Packet payload extraction for multicomponent contexts (1-2 days)
  - [x] Verify component-aware packet traversal context state in `collect_tile_packet_payload_for_progression`.
  - [ ] Ensure payload windows are not incorrectly shared/collapsed across components.
  - [ ] Locations: `extract_tile_data`, `build_packet_traversal_plan`, `collect_tile_packet_payload`, `collect_tile_packet_payload_for_progression`.
  - Done when:
  - [ ] Per-component sample counts are correct for test fixtures.
  - [ ] No packet-body overrun/underrun errors on representative multiband fixtures.

3. A3 - Component decode reconstruction path (1-2 days)
  - [x] Implement/complete per-component reconstruction in native path used by multicomponent decode.
  - [ ] Full component-loop for `decode_component_proper` / `decode_component_v2` (multi-layer external codestreams).
  - Note: `decode_component_proper` and `decode_component_v2` now return `NotImplemented` for nc>1 (preventing silent wrong-data decode) with clear bridge-fallback advisory.
  - [ ] Validate inter-component ordering matches output contract expected by adapter layer.
  - [ ] Locations: `decode_component_v2`, `decode_component_proper`.
  - Done when:
  - [ ] Decoded component buffers have expected lengths and deterministic ordering.
  - [ ] RGB-like fixture sanity checks pass (channel identity checks).

4. A4 - Bit-depth and signedness alignment checks (0.5-1 day)
  - [ ] Verify signed/unsigned handling parity with existing adapter expectations.
  - [ ] Add targeted fixtures/assertions for mixed precision edge cases used in remote-sensing products.
  - [ ] Locations: `decode_component*` path plus adapter mapping in `jpeg2000.rs`.
  - Done when:
  - [ ] No systematic value bias/offset in multicomponent outputs.
  - [ ] Typed output mapping remains stable for U8/U16/I16 paths.

5. A5 - Test fixture expansion and expected-output baselines (1-2 days)
  - [x] Add at least 3 multicomponent fixtures: small RGB JP2, Sentinel-style multiband sample, tiled multicomponent sample.
  - [x] Add deterministic expected checks (sample probes + metadata checks).
  - Progress:
  - [x] Small RGB fixture added: `crates/wbraster/tests/fixtures/rgb_8x8_lossless.jp2`.
  - [x] Validation tests added for fixture metadata + per-band deterministic sample checks.
  - [x] Sentinel-style fixture added: `crates/wbraster/tests/fixtures/sentinel_style_16x16_4band_lossless.jp2`.
  - [x] Tiled multicomponent fixture added: `crates/wbraster/tests/fixtures/tiled_rgb_64x64_block32_lossless.jp2`.
  - [x] Location: `reader.rs` tests and `wbraster` JPEG2000 validation tests.
  - Done when:
  - [ ] New fixtures run in CI and are stable across reruns.

6. A6 - Differential parity harness updates (0.5-1 day)
  - [ ] Extend differential corpus gating to report multicomponent-native parity progress clearly.
  - [ ] Capture counts for native_error, metadata_mismatch, sample_value_mismatch specific to multicomponent files.
  - [ ] Location: `jpeg2000.rs` differential test module.
  - Done when:
  - [ ] Summary output can be used as weekly progress KPI.

7. A7 - Adapter-level behavior check under bridge-enabled builds (0.5 day)
  - [ ] Confirm native path outputs are consumable by adapter fallback/selection logic without regressions.
  - [ ] Location: `jpeg2000.rs` read adapter path.
  - Done when:
  - [ ] Existing read-path tests and Sentinel smoke checks remain green.

8. A8 - Phase A go/no-go report (0.5 day)
  - [ ] Record which multicomponent classes are now supported natively and which remain explicitly out-of-scope for Phase A.
  - [ ] Update this plan with checkbox status and residual blockers.
  - Done when:
  - [ ] Gate A decision is evidence-backed (tests + differential metrics).

Phase A estimate subtotal: 5.5-10 days (roughly 1.5-3 weeks focused work; 2-4 weeks calendar including review/iteration).

### Phase B - POC Progression Support (1.5-3 weeks)
Targets:
- crates/wbraster/src/formats/jpeg2000_core/reader.rs

Tasks:
- [ ] Implement POC-aware packet traversal transitions.
- [ ] Add POC-positive fixtures (main header and tile-part forms).
- [ ] Validate traversal continuity across tile-parts.

Exit criteria:
- [ ] POC fixtures decode without NotImplemented errors.
- [ ] Traversal context remains stable across progression changes.

### Phase C - PPM/PPT External Header Support (1.5-3 weeks)
Targets:
- crates/wbraster/src/formats/jpeg2000_core/reader.rs

Tasks:
- [ ] Implement PPM/PPT packet-header sourcing in native walker.
- [ ] Integrate with existing bounded packet preflight/body-span checks.
- [ ] Add malformed-marker negative tests and positive fixtures.

Exit criteria:
- [ ] PPM/PPT fixtures decode without NotImplemented errors.
- [ ] Corrupt marker cases fail-fast with deterministic errors.

### Phase D - Cutover and Bridge Retirement (0.5-1.5 weeks)
Targets:
- crates/wbraster/Cargo.toml
- crates/wbraster/src/formats/jpeg2000.rs
- crates/wbraster/README.md
- crates/wbraster/CHANGELOG.md

Tasks:
- [ ] Flip default features to native-first (bridge disabled by default).
- [ ] Remove bridge-first runtime path from JPEG2000 adapter.
- [ ] Run full corpus + regression + performance sanity checks.
- [ ] Remove bridge dependency from wbraster runtime path.

Exit criteria:
- [ ] Full JPEG2000 suite is green on native-only default path.
- [ ] Sentinel clipping and representative production scenes pass.
- [ ] No bridge dependency required for default wbraster decode builds.

## Weekly Delivery Cadence
- Week 1:
  - [ ] Multicomponent design + prototype merged behind test coverage.
- Week 2:
  - [ ] Multicomponent parity pass and corpus report delivered.
- Week 3:
  - [ ] POC support first pass + fixture pack merged.
- Week 4:
  - [ ] POC hardening complete; PPM/PPT implementation started.
- Week 5:
  - [ ] PPM/PPT positive fixtures passing.
- Week 6:
  - [ ] Corpus-wide differential run and go/no-go cutover report.
- Week 7:
  - [ ] Default-feature flip on staging branch; integration smoke tests.
- Week 8:
  - [ ] Bridge retirement PR finalized after stabilization.

## Metrics to Track
- [ ] native_error count (differential corpus)
- [ ] sample_value_mismatch count
- [ ] metadata_mismatch count
- [ ] decode throughput on representative scenes
- [ ] peak memory on representative scenes

## Risks and Mitigations
- Risk: Rare codestream edge cases appear late.
  - Mitigation: Keep corpus expansion continuous and include external fixtures early.
- Risk: PPM/PPT complexity delays cutover.
  - Mitigation: Isolate parser/walker interfaces and add focused fixture-driven tests.
- Risk: Performance regressions after bridge removal.
  - Mitigation: Add per-phase benchmark checks before Gate C.

## Decision Log
- 2026-04-19: Plan created; bridge removal blocked by native multicomponent + POC + PPM/PPT parity gaps.
- 2026-04-19: Phase A kickoff implementation landed for the in-house single-layer multicomponent stream class:
  added consumed-byte tracking in tier-1 decode and sequential per-component extraction in native single-layer decode path, with a new writer/reader multiband roundtrip smoke test.
- 2026-04-19: Added component-selective packet payload extraction in the native packet walker
  (`extract_tile_data_for_component` + `collect_tile_packet_payload_for_progression` target-component routing),
  with tests that verify per-component LRCP packet-body filtering and component bounds checks.
- 2026-04-19: Added explicit `nc > 1` guards to `decode_component_proper` and `decode_component_v2`
  returning `NotImplemented` with a bridge-fallback advisory, replacing prior silent wrong-data decode
  risk for multicomponent codestreams routed to those paths. Full JPEG2000 suite: 60 passed, 0 failed.
