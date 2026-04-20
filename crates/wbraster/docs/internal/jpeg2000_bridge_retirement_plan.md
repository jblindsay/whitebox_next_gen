# JPEG2000 Bridge Retirement Plan

Status: Draft (execution-ready)
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
- [ ] Native multicomponent fixtures pass.
- [ ] Differential mismatch rate is within agreed epsilon/threshold.
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
  - [ ] Ensure payload windows are not incorrectly shared/collapsed across components. (partially evidenced; residual value mismatch indicates further validation needed)
  - [x] Locations: `extract_tile_data`, `build_packet_traversal_plan`, `collect_tile_packet_payload`, `collect_tile_packet_payload_for_progression`.
  - Done when:
  - [x] Per-component sample counts are correct for test fixtures.
  - [x] No packet-body overrun/underrun errors on representative multiband fixtures.

3. A3 - Component decode reconstruction path (1-2 days)
  - [x] Implement/complete per-component reconstruction in native path used by multicomponent decode.
  - [x] Full component-loop for `decode_component_proper` / `decode_component_v2` (multi-layer external codestreams).
  - [ ] Validate inter-component ordering matches output contract expected by adapter layer.
  - [x] Locations: `decode_component_v2`, `decode_component_proper`.
  - Done when:
  - [x] Decoded component buffers have expected lengths and deterministic ordering.
  - [ ] RGB-like fixture sanity checks pass (channel identity checks).

4. A4 - Bit-depth and signedness alignment checks (0.5-1 day)
  - [x] Verify signed/unsigned handling parity with existing adapter expectations.
  - [x] Thread per-component bit-depth/signedness through all native decode paths (`decode_component_single_layer`, `decode_component_proper`, `decode_component_v2`) for dequant/level-shift math.
  - [x] Add targeted fixtures/assertions for bit-depth/signedness stability in current multiband fixtures.
  - [x] Add remote-sensing-style mixed-range edge assertions (low/high reflectance + binary mask + mid-range) in multiband U16 coverage.
  - [x] Locations: `decode_component*` path plus adapter mapping in `jpeg2000.rs`.
  - Done when:
  - [ ] No systematic value bias/offset in multicomponent outputs.
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
  - [ ] New fixtures run in CI and are stable across reruns.

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

A8 Gate A report (2026-04-19):
- Decision: **No-Go** for bridge retirement at end of Phase A.
- Evidence:
  - A5 fixture validation tests are green through adapter path (bridge-enabled default) for RGB, Sentinel-style multiband, and tiled multicomponent fixtures.
  - A6 differential harness reports multicomponent-specific KPIs and currently shows multicomponent sample-value mismatches on all local multicomponent fixtures (`multicomponent_sample_value_mismatch=3`, `multicomponent_native_error=0`, `multicomponent_metadata_mismatch=0`).
  - A7 adapter read-path tests confirm guarded behavior: supported multiband bridge decode succeeds, and bridge-fail multiband inputs are blocked from unsafe native fallback.
- Native multicomponent status:
  - In-scope and partially complete: packet traversal / component handling / metadata alignment infrastructure.
  - Not yet parity-complete: multicomponent sample-value correctness against bridge baseline.
- Explicitly out-of-scope for Phase A completion:
  - Full POC traversal support (Phase B).
  - Full PPM/PPT packet-header sourcing support (Phase C).
- Residual blocker to Gate A "Go":
  - Eliminate multicomponent sample-value mismatches in differential corpus (target: drive `multicomponent_sample_value_mismatch` to 0 for agreed fixture set).

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

Timebox decision checkpoint (2026-04-20):
- Decision: No-Go for further immediate CL-lane parity iteration inside Phase A.
- Evidence:
  - Attempt 1 (zero-context uniform routing) produced no mismatch-class
    improvement and regressed first-mismatch magnitude on all 3 matrix
    fixtures.
  - Attempt 2 (all cleanup samples via cleanup context 18) again produced no
    mismatch-class improvement and regressed to near cleanup-disabled error
    scale on all 3 matrix fixtures.
  - Hard-stop criterion hit: no KPI movement after two distinct CL-lane
    correction attempts.
- Required posture:
  - Keep bridge-backed decode as default for problematic multicomponent JP2
    classes.
  - Treat native multicomponent parity as partial completion pending a later,
    explicitly scheduled codec-deep follow-up milestone.

### Follow-up Milestone - Native Parity Recovery (scheduled later, not in Phase A)

Goal:
- Resume multicomponent native parity only under an explicitly scheduled
  codec-deep milestone with fresh budget, rather than continuing ad hoc CL
  experiments inside Phase A.

Entry criteria:
- Bridge-backed decode remains the production default for problematic
  multicomponent JP2 classes.
- The follow-up work is scoped as a dedicated milestone with its own estimate
  and stop/go checkpoint.
- No more CL-only correction attempts are started without new evidence that
  changes the root-cause model.

Priority work lanes:
- Lane 1: code-block segment and pass accounting audit
  - Reconcile packet-declared coding-pass totals, segment assembly, and
    code-block body slicing against `wbjpeg2000` reference behavior.
  - Confirm that the bytes consumed by native LL block decode match the
    intended packet/header interpretation before adjusting entropy semantics.
- Lane 2: side-by-side tier-1 reference trace harness
  - Add a narrow fixture-backed trace path for first failing LL code blocks,
    capturing significance decisions, sign decisions, MQ context labels, and
    byte-consumption checkpoints for both implementations.
  - Required outcome: make divergence observable at a smaller unit than the
    current whole-block matrix summary.
- Lane 3: targeted port of proven reference semantics
  - Prefer porting a verified `wbjpeg2000` tier-1 behavior slice into
    `jpeg2000_core` over additional context-remap speculation.
  - Focus first on the smallest behavior slice that can explain the current LL
    divergence signature.
- Lane 4: acceptance gates and regression fixtures
  - Keep the existing parity matrix as a fast screen.
  - Add fixture-backed acceptance criteria for any future parity fix:
    mismatch-class reduction or stable first-mismatch improvement on at least
    2 of 3 agreed fixtures with no regressions.

Initial estimate for the follow-up milestone:
- Best case: 3-5 engineering days.
- Most likely: 5-8 engineering days.
- Conservative: 8-12 engineering days.

Definition of done for re-entry:
- A new fix narrative is grounded in segment-accounting or reference-trace
  evidence, not only in cleanup-context experimentation.
- The first retained runtime change survives full matrix reruns without
  regression in the existing baseline profiles.

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
- 2026-04-19: Adopted a hard 2-3 day final Phase A spike protocol with
  explicit success and stop gates. If stop gates trigger, fallback posture is
  bridge-default for problematic JP2 classes plus native-only for verified-safe
  classes.
