# JPEG2000 Bridge Retirement Plan

Status: Phase A Complete, Ready for Phase B
Owner: wbraster core team
Last updated: 2026-04-20

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
- [x] Implement native multicomponent decode path (remove current fail-fast guard for supported codestreams).
- [ ] Validate component ordering, signedness, and bit-depth handling. (partial: bit-depth/signedness checks landed; ordering/value parity still open)
- [x] Add/expand multiband fixtures and expected outputs.
- [x] Add parity assertions against bridge-backed baseline corpus.

Exit criteria:
- [x] Native multicomponent fixtures pass.
- [x] Differential mismatch rate is within agreed epsilon/threshold.
- [x] No regressions in existing JPEG2000 tests.

#### Phase A kickoff workboard (function-level)

Sprint target: remove multicomponent fail-fast for supported codestreams while preserving existing single-band behavior.

1. A1 - Decode dispatch and capability gate cleanup (0.5-1 day)
  - [x] Update `decode_component` to route supported multicomponent flows into native packet path.
  - [x] Keep explicit fail-fast for still-unsupported marker workflows during Phase A.
  - [x] Location: `reader.rs` function `decode_component`.
  - Done when:
  - [x] Existing fail-fast tests still pass for unsupported workflows.
  - [x] Supported multicomponent fixture no longer trips generic multicomponent NotImplemented.

2. A2 - Packet payload extraction for multicomponent contexts (1-2 days)
  - [x] Verify component-aware packet traversal context state in `collect_tile_packet_payload_for_progression`.
  - [x] Ensure payload windows are not incorrectly shared/collapsed across components.
  - [x] Locations: `extract_tile_data`, `build_packet_traversal_plan`, `collect_tile_packet_payload`, `collect_tile_packet_payload_for_progression`.
  - Done when:
  - [x] Per-component sample counts are correct for test fixtures.
  - [x] No packet-body overrun/underrun errors on representative multiband fixtures.

3. A3 - Component decode reconstruction path (1-2 days)
  - [x] Implement/complete per-component reconstruction in native path used by multicomponent decode.
  - [x] Full component-loop for `decode_component_proper` / `decode_component_v2` (multi-layer external codestreams).
  - [x] Validate inter-component ordering matches output contract expected by adapter layer.
  - [x] Locations: `decode_component_v2`, `decode_component_proper`.
  - Done when:
  - [x] Decoded component buffers have expected lengths and deterministic ordering.
  - [x] RGB-like fixture sanity checks pass (channel identity checks).

4. A4 - Bit-depth and signedness alignment checks (0.5-1 day)
  - [x] Verify signed/unsigned handling parity with existing adapter expectations.
  - [x] Thread per-component bit-depth/signedness through all native decode paths (`decode_component_single_layer`, `decode_component_proper`, `decode_component_v2`) for dequant/level-shift math.
  - [x] Add targeted fixtures/assertions for bit-depth/signedness stability in current multiband fixtures.
  - [x] Add remote-sensing-style mixed-range edge assertions (low/high reflectance + binary mask + mid-range) in multiband U16 coverage.
  - [x] Locations: `decode_component*` path plus adapter mapping in `jpeg2000.rs`.
  - Done when:
  - [x] No systematic value bias/offset in multicomponent outputs.
  - [x] Typed output mapping remains stable for U8/U16/I16 paths.

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
  - [x] New fixtures run in CI and are stable across reruns.

6. A6 - Differential parity harness updates (0.5-1 day)
  - [x] Extend differential corpus gating to report multicomponent-native parity progress clearly.
  - [x] Capture counts for native_error, bridge_error, metadata_mismatch, sample_count_mismatch, and sample_value_mismatch specific to multicomponent files.
  - [x] Add multicomponent-specific threshold env gates for KPI enforcement (`JPEG2000_DIFF_MAX_MULTICOMPONENT_*`).
  - [x] Enrich mismatch diagnostics with localization fields (`band`, `row`, `col`, `pixel`, `abs_err`).
  - [x] Location: `jpeg2000.rs` differential test module.
  - Done when:
  - [x] Summary output can be used as weekly progress KPI.

7. A7 - Adapter-level behavior check under bridge-enabled builds (0.5 day)
  - [x] Confirm native path outputs are consumable by adapter fallback/selection logic without regressions.
  - [x] Location: `jpeg2000.rs` read adapter path.
  - Done when:
  - [x] Existing read-path tests and Sentinel smoke checks remain green.

8. A8 - Phase A go/no-go report (0.5 day)
  - [x] Record which multicomponent classes are now supported natively and which remain explicitly out-of-scope for Phase A.
  - [x] Update this plan with checkbox status and residual blockers.
  - Done when:
  - [x] Gate A decision is evidence-backed (tests + differential metrics).

A8 Gate A report (2026-04-20 UPDATED):
- Decision: **Go** for bridge retirement at end of Phase A.
- Evidence:
  - A5 fixture validation tests are green through both native and bridge paths for RGB, Sentinel-style multiband, and tiled multicomponent fixtures.
  - A6 differential harness confirms zero multicomponent sample-value mismatches: `multicomponent_sample_value_mismatch=0`, `multicomponent_native_error=0`, `multicomponent_metadata_mismatch=0`, `multicomponent_sample_count_mismatch=0` across all 3 standard-profile fixtures.
  - A7 adapter read-path tests confirm guarded behavior: supported multiband native decode succeeds with full parity against bridge baseline, and multicomponent samples match bridge output exactly.
  - Root cause identified and fixed: MQ context 0 was uninitialized in native entropy decoder (missing context state index 4 per ISO 15444-1 Table D.7).
- Native multicomponent status:
  - **Complete for Phase A scope**: packet traversal, component handling, metadata alignment, and tier-1 entropy decoding all verified for standard-profile lossless multicomponent JP2.
  - Parity-complete: multicomponent sample-value correctness fully restored against bridge baseline.
- Explicitly out-of-scope for Phase A completion:
  - Full POC traversal support (Phase B).
  - Full PPM/PPT packet-header sourcing support (Phase C).
- Gate A criteria achieved:
  - ✅ All multicomponent sample-value mismatches eliminated (drove `multicomponent_sample_value_mismatch` to 0 for agreed fixture set).
  - ✅ No regressions in existing JPEG2000 tests.
  - ✅ Differential corpus validates full parity for all three standard-profile multicomponent fixtures.

Phase A estimate subtotal: 5.5-10 days (roughly 1.5-3 weeks focused work; 2-4 weeks calendar including review/iteration).

### Phase A remaining estimate (as of 2026-04-19)

Current blocker profile:
- Differential corpus still reports multicomponent sample-value mismatches on local fixture trio (`multicomponent_sample_value_mismatch=3`), while native/bridge/metadata/sample-count error classes are currently 0 for this set.
- Latest LL cleanup tracing reduced uncertainty around one branch: on the
  checked fixture, run-mode path selection is not currently active
  (`run_eligible_cols=0`), narrowing likely root-cause scope to cleanup
  significance/sign semantics and/or CL context-state preparation.

Remaining work packages:
- A4 mixed-precision edge coverage:
  - Add remote-sensing-oriented mixed precision fixtures/assertions and stabilize expectations.
  - Estimate: 0.5-1.0 engineering day.
- Native parity root-cause and fix pass (critical path):
  - Complete deeper packet-header / tier-1 decode correctness work for multicomponent value parity.
  - Target: reduce `multicomponent_sample_value_mismatch` from current baseline to agreed threshold (ideally 0 for the agreed fixture set).
  - Estimate: 3-6.5 engineering days (still highest uncertainty item; CL stream divergence now directly evidenced).
- Differential reruns + KPI gating tune:
  - Re-baseline reports and tighten `JPEG2000_DIFF_MAX_MULTICOMPONENT_*` thresholds based on improved parity.
  - Estimate: 0.5-1.0 engineering day.
- Final Phase A closure checks:
  - Confirm checklist/exit criteria status, produce final A8 update, and verify no regressions in JPEG2000 suites.
  - Estimate: 0.5-1.0 engineering day.

Estimated Phase A remaining total:
- Best case: 4-5.5 engineering days.
- Most likely: 4.5-8 engineering days.
- Conservative: 7-11 engineering days (if deeper entropy/packet interpretation issues require iterative fixes).

### Phase A Hard Timebox Protocol (final spike)

Purpose:
- Prevent indefinite codec-deep work by running one bounded final spike with
  measurable checkpoints and explicit stop conditions.

Timebox:
- Maximum effort: 2-3 engineering days (no extension without explicit
  go-decision).

Execution windows:
- Window 1 (Day 0 to Day 1): CL semantics correction attempt
  - Focus only on LL cleanup significance/sign semantics and context-state
    handling inferred from CL stream divergence.
  - Keep all changes behind diagnostic toggles where possible until KPI
    movement is demonstrated.
  - Status:
    - Attempt 1 completed and reverted: routing LL cleanup zero-context samples
      through uniform context produced no class improvement and regressed
      first-mismatch magnitude on 3/3 matrix fixtures.
    - Attempt 2 completed and reverted: routing all LL non-run-mode cleanup
      significance samples through cleanup context produced no class
      improvement and regressed to near cleanup-disabled mismatch magnitudes on
      3/3 matrix fixtures.
- Window 2 (Day 1 to Day 2): validation and hardening
  - Re-run parity matrix and differential corpus.
  - Keep only net-positive changes; revert non-winning experiments.
- Window 3 (Day 2 to Day 3): final decision checkpoint
  - Produce go/no-go decision for continued native parity effort in Phase A.

Success criteria (must hit at least one):
- Reduce multicomponent mismatch class count on agreed local fixture trio, or
- Keep mismatch class count unchanged but achieve clear first-mismatch
  magnitude improvement across at least 2 of 3 fixtures with no regressions,
  and provide a coherent root-cause fix narrative.

Stop criteria (any one triggers immediate pivot):
- No KPI movement after two distinct CL-lane correction attempts.
- Any attempted fix only shifts mismatch shape/magnitude without stable net
  gain on matrix reruns.
- New regressions in baseline matrix profiles that cannot be eliminated in the
  same window.

If stop criteria are met (fallback posture):
- Freeze bridge-backed decode as default for problematic JP2 classes.
- Keep native path enabled for verified-safe classes only.
- Move full parity completion to a scheduled follow-up milestone instead of
  blocking broader roadmap delivery.

Decision outputs required at end of timebox:
- Go: continue Phase A parity work with narrowed CL-lane plan and updated
  remaining estimate.
- No-Go: mark Phase A parity as partial completion, adopt fallback posture,
  and proceed with non-blocked roadmap work.

Timebox decision checkpoint (2026-04-20 UPDATED):
- Decision: **Go** - Phase A parity achieved; all exit criteria met.
- Evidence:
  - Root cause found and fixed: MQ context 0 initialization missing in native entropy decoder.
  - Single-line fix in `crates/wbraster/src/formats/jpeg2000_core/entropy.rs` MqDecoder::init_standard_j2k_contexts().
  - Added line: `self.cx[0] = (4, 0);` to initialize zero-coding context per ISO 15444-1 Table D.7.
  - All three standard-profile fixtures now pass: `multicomponent_sample_value_mismatch=0`.
  - Differential summary: `fixtures_total=3 ok=3 native_error=0 bridge_error=0 metadata_mismatch=0 sample_count_mismatch=0 sample_value_mismatch=0`.
  - No regressions in baseline matrix profiles.
- Action items for transition to Phase B:
  - Clean up diagnostic hooks (ref_ll_decode_head in wbjpeg2000/decode.rs) if still present.
  - Update Phase B readiness and begin POC/PPM/PPT work.
  - Document root-cause discovery narrative in Phase A closure report.

### Phase B - POC Progression Support (2-4 weeks)
**Status: In progress. Narrow main-header POC subset implemented and verified.**

Targets:
- crates/wbraster/src/formats/jpeg2000_core/reader.rs (POC marker packet traversal)
- crates/wbraster/src/formats/jpeg2000.rs (adapter handling)

Tasks:
- [~] Implement POC packet progression order transitions. (safe subset complete: single full-range main-header POC acts as a global progression override)
- [ ] Add POC-style fixtures with progression order changes.
- [ ] Add parity assertions for POC-enabled variants.
- [x] Verify no regressions in Phase A multicomponent fixtures.

Exit criteria:
- [ ] POC-enabled fixtures pass native decode.
- [ ] Differential corpus shows no POC-related mismatches.
- [x] No regressions in Phase A fixtures.

#### Phase B Progress (2026-04-20)

B1 Status: **COMPLETE** (2026-04-20)
- Created Poc struct with PocChange entries in codestream.rs
- Implemented Poc::parse() with proper variable-width encoding support
- Added main_header_poc: Option<Poc> field to GeoJp2 reader
- Updated main header parsing to parse POC marker data with component count awareness
- Phase A fixtures validation: all 3 still passing (ok=3)
- Differential summary: `native_unsupported_poc=0` (test fixtures don't exercise POC boundaries)

B2 Status: **PARTIAL** (safe subset implemented, 2026-04-20)
- Implemented safe main-header POC subset in packet traversal planning:
  - Supports exactly one full-range main-header POC entry as a global progression override.
  - Rejects multi-segment and partial-range main-header POC with explicit NotImplemented errors.
  - Keeps tile-part POC unsupported (explicit NotImplemented) pending full per-part progression window support.
- Added unit tests for supported and rejected shapes:
  - Accept single full-range main-header POC.
  - Reject partial-range main-header POC.
  - Reject tile-part POC.
- Regression validation: Phase A differential corpus remains green (`ok=3`, `native_unsupported_poc=0`).
- Verification completed on code path:
  - `cargo test -p wbraster resolve_progression_ -- --nocapture` passes.
  - `JPEG2000_DIFF_FIXTURES="tests/fixtures/rgb_8x8_lossless.jp2,tests/fixtures/sentinel_style_16x16_4band_lossless.jp2,tests/fixtures/tiled_rgb_64x64_block32_lossless.jp2" cargo test -p wbraster formats::jpeg2000::differential_tests::jpeg2000_native_vs_bridge_differential_corpus -- --nocapture` remains green.
- Remaining B2 work:
  1. Full multi-segment POC boundary transitions during packet walking.
  2. Tile-part POC transition support.
  3. POC-positive fixture corpus to validate real boundary-switch behavior.
- Impact: Phase B now has a functioning low-risk baseline; Phase C can still proceed in parallel.

- Best case: 2-3 engineering weeks.
- Most likely: 2.5-4 engineering weeks.
- Conservative: 3-5 engineering weeks.

### Phase C - PPM/PPT Packet Header Sourcing (1.5-3 weeks)
**Status: Pending Phase B completion.**

Targets:
- crates/wbraster/src/formats/jpeg2000_core/reader.rs (packet header marker parsing)
- crates/wbraster/src/formats/jpeg2000.rs

Tasks:
- [ ] Implement PPM/PPT marker parsing and packet header reconstruction.
- [ ] Add PPM/PPT-style fixtures.
- [ ] Add parity assertions for PPM/PPT-enabled variants.
- [ ] Verify no regressions in Phase A & B fixtures.

Exit criteria:
- [ ] PPM/PPT-enabled fixtures pass native decode.
- [ ] Differential corpus shows no PPM/PPT-related mismatches.
- [ ] No regressions in Phase A & B fixtures.

Estimate:
- Best case: 1.5-2.5 engineering weeks.
- Most likely: 2-3 engineering weeks.
- Conservative: 2.5-4 engineering weeks.

### Phase D - Cutover and Bridge Retirement (0.5-1.5 weeks)
**Status: Pending Phase C completion.**

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

Estimate:
- Best case: 0.5-1 engineering week.
- Most likely: 0.5-1.5 engineering weeks.
- Conservative: 1-2 engineering weeks.

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
- 2026-04-19: Added one-command parity matrix workflow (`dev/run_jpeg2000_parity_matrix.sh`) to run
  baseline + targeted decoder profiles each iteration. Latest matrix reinforced LL-dominant divergence:
  LL-only legacy fallback matches all-subband legacy signature, while HF-only legacy fallback remains
  near standard baseline; cleanup run-mode enabled remains strongly regressive.
- 2026-04-19: Added LL pass-level probe controls (`JPEG2000_DIFF_LL_DISABLE_SP`,
  `JPEG2000_DIFF_LL_DISABLE_MR`, `JPEG2000_DIFF_LL_DISABLE_CL`) and extended matrix runs.
  Result: LL cleanup disable caused strongest regression (~32768 class), LL MR disable produced
  mid-level regression (~16384 class), and LL SP disable smaller/mixed regression. This prioritizes
  LL cleanup semantics as next critical debugging lane.
- 2026-04-19: Added LL code-block A/B debug instrumentation
  (`JPEG2000_DEBUG_LL_BLOCK_AB`) in proper-path first LL block decode. Initial runs confirmed
  cleanup-pass indispensability (`no_cl` decoding yields zero nonzero coefficients) but still showed
  substantial standard-vs-legacy LL coefficient pattern divergence; a follow-on table-driven sign-context
  trial did not improve parity KPIs and was reverted.
- 2026-04-19: Added LL cleanup eligibility/run trace counters
  (`JPEG2000_DEBUG_LL_CLEANUP_TRACE`). Initial rgb fixture run showed zero
  run-eligible columns while run-mode is disabled, with cleanup activity
  concentrated at the top decoded bitplane before SP/MR dominate subsequent
  bitplanes. This narrows immediate Phase A focus to CL significance/sign
  semantics and context-state setup rather than run-mode branch gating.
- 2026-04-19: Added side-by-side cleanup bitstream tracing
  (`JPEG2000_DEBUG_CL_SIG_STREAM`) for standard and legacy decoders and
  captured LL first-block samples. Standard path consumes mixed SIG contexts
  (`ctx 0..3`) while legacy cleanup consumes context 18 stream, confirming
  divergence at CL symbol/context stream level before reconstruction.
- 2026-04-20: Added native run-aggregate/run-position cleanup trace events and
  captured a three-artifact Step 4 checkpoint. Result: even after correcting
  standard context numbering and subband-aware zero-coding lookup, the first
  native cleanup run-aggregate decision remains wrong (`ctx17 bit=0` vs
  reference `ctx17 bit=1`), so the active root-cause lane has moved earlier
  than cleanup branch selection itself.
- 2026-04-20: Added entropy-state snapshots on both native and reference
  run-aggregate decode events. First canonical event shows register mismatch
  before `ctx17` decode, confirming the next correction lane is decoder
  initialization/byte-in semantics.
- 2026-04-20: Aligned native MQ initialization/BYTEIN semantics to reference.
  This fixed the first run-aggregate event match and materially reduced mismatch
  magnitude class, but did not yet complete parity.
- 2026-04-20: Landed the next retained native parity fixes in Phase A follow-up:
  corrected MQ LPS exchange semantics, corrected standard neighbor-significance
  bit packing, corrected MR context-state tracking, and re-enabled standard
  cleanup run-mode by default. These changes brought the canonical
  `rgb_8x8_lossless.jp2` LL coefficient stream into direct agreement with the
  `wbjpeg2000` reference decoder.
- 2026-04-20: Added inverse multicomponent transform handling to the native
  assembled multiband read path with correct sign-shift ordering. This closed
  the canonical RGB parity gap completely: the differential harness now reports
  `ok=1` and `sample_value_mismatch=0` for
  `rgb_8x8_lossless.jp2`.
- 2026-04-20: Re-ran the parity matrix after the retained Tier-1 and inverse
  MCT fixes. Baseline standard/native default posture improved to `ok=2/3` on
  the agreed fixture trio; the only remaining baseline mismatch is now
  `tiled_rgb_64x64_block32_lossless.jp2` with a narrowed first-sample error of
  `native=79` vs `bridge=100`.
- 2026-04-19: Adopted a hard 2-3 day final Phase A spike protocol with
  explicit success and stop gates. If stop gates trigger, fallback posture is
  bridge-default for problematic JP2 classes plus native-only for verified-safe
  classes.
- 2026-04-20 (PHASE A COMPLETION): Identified and fixed root cause of final tiled fixture parity mismatch.
  Root cause: MQ context 0 (zero-coding context) was uninitialized in native entropy decoder.
  Fix: Added `self.cx[0] = (4, 0);` to MqDecoder::init_standard_j2k_contexts() in entropy.rs,
  initializing context 0 to state index 4 per ISO 15444-1 Table D.7 (matches reference wbjpeg2000 behavior).
  Single-line fix with immediate validation: differential corpus now reports `fixtures_total=3 ok=3`
  across all three standard-profile multicomponent fixtures (rgb_8x8, sentinel_style_16x16_4band, tiled_rgb_64x64_block32).
  All error classes at zero: `native_error=0 bridge_error=0 metadata_mismatch=0 sample_count_mismatch=0 sample_value_mismatch=0`.
  **Gate A Decision: GO** — Phase A multicomponent native parity is COMPLETE. Ready for Phase B entry. 
