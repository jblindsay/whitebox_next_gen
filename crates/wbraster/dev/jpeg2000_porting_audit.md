# JPEG2000 Native Porting Audit

Purpose: Use the in-repo `wbjpeg2000` crate as a correctness reference to improve `wbraster/src/formats/jpeg2000_core` and eventually remove the vendored bridge from the `wbraster` read path.

## Current runtime strategy

- Default read path: `wbjpeg2000` decode + native georeferencing extraction.
- Native-only mode: disable `jpeg2000-vendored-bridge` feature.
- Safety guard: if vendored decode fails for multiband images, fail loudly instead of silently falling back to potentially corrupt legacy decode.

## Known native-risk areas in jpeg2000_core

1. Packet/component payload handling in `reader.rs`:
- Existing `extract_component_data` uses equal chunk splitting for multi-component tile data.
- This is not valid for general JP2 packet layouts and can induce repeating artifacts.

2. Tier-1 entropy assumptions in `entropy.rs`:
- Implementation is intentionally simplified and may diverge from full EBCOT semantics.
- Marker-stuffing and context pass details are likely incomplete for diverse external scenes.

3. Tile-part/progression assumptions in `reader.rs`:
- Current extraction logic largely assumes simple contiguous payload patterns.
- External scenes may use progression/tile-part patterns requiring stricter packet parsing.

## Mapping: wbjpeg2000 -> native modules

- `crates/wbjpeg2000/src/j2c/codestream.rs` -> `src/formats/jpeg2000_core/codestream.rs`
- `crates/wbjpeg2000/src/j2c/decode.rs` -> `src/formats/jpeg2000_core/reader.rs` + `entropy.rs`
- `crates/wbjpeg2000/src/j2c/bitplane.rs` -> `src/formats/jpeg2000_core/entropy.rs`
- `crates/wbjpeg2000/src/j2c/arithmetic_decoder.rs` -> `src/formats/jpeg2000_core/entropy.rs`
- `crates/wbjpeg2000/src/j2c/progression.rs` -> `src/formats/jpeg2000_core/reader.rs`
- `crates/wbjpeg2000/src/j2c/tile.rs` -> `src/formats/jpeg2000_core/reader.rs`
- `crates/wbjpeg2000/src/j2c/idwt.rs` -> `src/formats/jpeg2000_core/wavelet.rs`

## Phase 1 port targets (high impact, low API disruption)

1. Replace packet parsing / component payload extraction path in native reader.
Status: in progress.
- Completed: native reader now parses tile-parts with `SOT`/`Psot` bounded `SOD` extraction and concatenates multiple tile-parts for the same tile index.
- Completed: native reader now materializes ordered tile-part metadata (`isot`, `tpsot`, `tnsot`, payload bounds) and builds a packet traversal plan scaffold for per-tile decode flow.
- Completed: native packet payload collection now uses an LRCP-only traversal stage with explicit fail-fast for non-LRCP progressions until packet-header parsing is ported.
- Completed: native tile-part header scan now parses pre-`SOD` marker segments and fail-fast rejects unsupported `POC` and packed/explicit packet-header marker workflows (`PLM`, `PLT`, `PPM`, `PPT`).
- Completed: native LRCP traversal now includes bounded first packet-header preflight in tile-part payloads, with explicit checks for optional `SOP`/`EPH`, empty payload after `SOD`, and invalid/truncated first-header conditions.
- Completed: native tile-part header parsing now consumes `COD` segments and applies tile-part progression/layer overrides to packet traversal planning.
- Completed: native LRCP first-packet preflight now includes bounded non-zero packet bit parsing with classic coding-pass codeword shape checks to catch truncated/malformed header bitstreams earlier.
- Completed: native LRCP preflight now probes classic `Lblock` unary increments and segment-length bitfield widths, rejecting unterminated unary and excessive/truncated length-bit requests as malformed packet headers.
- Completed: native LRCP preflight now validates first inclusion signaling bit availability and rejects non-zero packet headers that leave no packet body bytes within tile-part payload bounds.
- Completed: native LRCP preflight now performs provisional classic segment-length value parsing and rejects packet headers whose declared segment length exceeds remaining tile-part body bytes.
- Completed: native LRCP preflight ordering now evaluates first inclusion signaling before coding-pass/length probes, and applies empty-body/declared-span guards conditionally when first inclusion indicates contribution.
- Completed: native LRCP preflight now performs bounded multi-contribution preview (inclusion-gated coding-pass/length probes) and enforces cumulative declared-length vs remaining body-byte consistency checks.
- Completed: native LRCP extraction now uses preflight-derived body span metadata for previewed included contributions (body-start + cumulative declared length), reducing dependence on raw full-payload concatenation in those paths.
- Completed: native LRCP extraction now walks a bounded number of packet boundaries within a tile-part and applies preview-derived body slicing across multiple packet starts.
- Completed: native LRCP packet-preview iteration budget is now progression-aware (layer/component/resolution scaled with caps) instead of fixed-count previewing.
- Completed: native LRCP preview loop now uses explicit packet-context traversal scaffolding (`layer/resolution/component`) and includes LRCP context in preflight error diagnostics.
- Completed: native LRCP packet preflight now carries per-context `Lblock` state across packet walking and applies state-evolved `Lblock` values during classic segment-length parsing.
- Completed: native LRCP packet-context state now persists inclusion/history counters (`packets_seen`, `contributions_seen`, `ever_included`) and surfaces these in LRCP-context diagnostics.
- Completed: native LRCP packet-context state now tracks inclusion-transition history (`first_included_packet_index`, `last_included_packet_index`, `packets_since_last_inclusion`, `zero_length_packets`) and updates it in packet-header preflight.
- Completed: LRCP tile-part packet-body assembly now stages preview slices per tile-part and reverts to conservative full tile-part payload when post-preview non-zero packets have no previewed contribution (ambiguous body-span case).
- Note: current LRCP packet walker is single-pass over `(layer,resolution,component)` contexts, so same-context inclusion-history fallback hooks are in place but cannot yet be fully exploited until packet-context revisit traversal is implemented.
- Completed: LRCP packet walker now supports bounded context-round revisits, enabling same-context packet-state continuity across multiple packet positions in one tile-part.
- Completed: preview budget logic now allows revisit rounds with explicit hard bounds (`x2` revisit factor, clamp to 128).
- Completed: ambiguous non-zero/no-preview packet handling is now context-aware under revisit traversal: same-context prior inclusion forces full tile-part fallback, while new-context ambiguity preserves collected preview spans and halts preview walking.
- Completed: packet preflight now detects bounded contribution-preview cap exhaustion (no observed inclusion termination) and routes extraction to conservative full tile-part fallback for that ambiguous packet-body span case.
- Completed: native constrained RLCP traversal support added (RLCP packet-context cursor ordering, bounded revisit rounds, RLCP-context diagnostics).
- Completed: tile-part COD progression overrides to RLCP are now consumed by native packet traversal rather than failing as unsupported progression mode.
- Completed: packet-body accounting now appends unresolved tail bytes when preview walking halts at ambiguous new-context non-zero packets after collected preview contributions (LRCP and constrained RLCP paths).
- Completed: LRCP and constrained RLCP walkers are now unified under one progression-parameterized traversal core, reducing divergence risk while preserving existing packet preflight/fallback semantics.
- Completed: native packet walker now supports all progression orders (`LRCP`, `RLCP`, `RPCL`, `PCRL`, `CPRL`) within the current constrained model.
- Completed: constrained `RPCL`/`PCRL`/`CPRL` cursor-order traversal is covered by deterministic tests and tile-part COD override fixtures.
- Completed: packet context/state continuity now persists across tile-parts for a tile (instead of resetting per tile-part).
- Completed: writer invariants hardened with decomposition-level bounds tied to image dimensions and color-space/component compatibility checks.
- Completed: writer defaults now auto-cap decomposition levels to image-supported limits.
- Completed: COD parser minimum length updated to accept standards-conformant 10-byte COD payloads (legacy padding no longer required).
- Completed: active writer confidence tests added for single-band encode/decode shape smoke, multiband metadata roundtrip, and invariant rejection paths.
- Completed: writer lossy-mode confidence smoke test added (encode/parse metadata behavior).
- Completed: writer geometadata roundtrip test added (geo-transform, EPSG, NODATA).
- Completed: lossy writer parameter validation now rejects non-finite/non-positive `quality_db` values.
- Completed: differential corpus harness added for native-vs-bridge decode comparison with mismatch classification and optional enforcement mode.
- Differential harness control env vars:
  - `JPEG2000_DIFF_FIXTURES`
  - `JPEG2000_DIFF_FIXTURE_FILE`
  - `JPEG2000_DIFF_EPS`
  - `JPEG2000_DIFF_ENFORCE`
- Differential harness enhancements:
  - optional JSON report output via `JPEG2000_DIFF_REPORT`
  - threshold-based pass/fail gates via `JPEG2000_DIFF_MAX_*` and `JPEG2000_DIFF_MIN_OK`
  - multicomponent-specific threshold gates via:
    - `JPEG2000_DIFF_MAX_MULTICOMPONENT_NATIVE_ERROR`
    - `JPEG2000_DIFF_MAX_MULTICOMPONENT_BRIDGE_ERROR`
    - `JPEG2000_DIFF_MAX_MULTICOMPONENT_METADATA_MISMATCH`
    - `JPEG2000_DIFF_MAX_MULTICOMPONENT_SAMPLE_COUNT_MISMATCH`
    - `JPEG2000_DIFF_MAX_MULTICOMPONENT_SAMPLE_VALUE_MISMATCH`
  - sample mismatch details now include localization fields:
    - `band`, `row`, `col`, and within-band `pixel` index
    - absolute error value `abs_err`
- Differential harness now tracks `native_unsupported_packet_header_markers` as
  an explicit subtype of `native_error` (for `PLM`/`PLT`/`PPM`/`PPT` workflows).
- Baseline corpus run (3 Sentinel-2 `R10m` band fixtures) report:
  - **Before Chunk E:** `ok=0`, `native_error=3`, `native_unsupported_packet_header_markers=3`
  - **After Chunk E (PLT skip fix):** `ok=0`, `native_error=0`, `metadata_mismatch=2`, `sample_value_mismatch=1`
  - **After Chunk F (pixel_type + POC detection):** `ok=0`, `native_error=0`, `metadata_mismatch=0`, `sample_count_mismatch=0`, `sample_value_mismatch=3`
  - Remaining issues (Chunk G):
    - All 3 bands show sample_value_mismatch. Root cause: `decode_block` uses a
      single-block MQ arithmetic decode approximation. Real JPEG2000 code-blocks
      use per-code-block segment lengths from packet header Lblock fields, not a
      single `bits + num_decomps` heuristic. This produces incorrect coefficients
      for all real-world files that use standard code-block subdivision.
  - Report artifact: `crates/wbraster/dev/jpeg2000_diff_report_sentinel2_chunk_f.json`
- Follow-up controlled experiments against local multicomponent fixture trio
  (`rgb_8x8_lossless.jp2`, `sentinel_style_16x16_4band_lossless.jp2`,
  `tiled_rgb_64x64_block32_lossless.jp2`) showed no mismatch-count reduction:
  - `decode_component_proper`: `decode_block_standard_j2k` -> `decode_block`
    changed sample values but kept `multicomponent_sample_value_mismatch=3`.
  - `decode_component_proper`: lossless bitplane count tweak
    (`raw-missing-1` -> `raw-missing`) had no measurable effect.
  - Internal-reader inverse RCT post-pass for lossless 3-band RGB had no
    measurable effect on first-mismatch metrics.
  - Lossless subband raw-bitplane derivation tweak (`proper`/`v2`) using
    `comp_bits + subband_gain` instead of QCD exponent showed no measurable
    mismatch-count improvement and was reverted.
  - Added targeted decode diagnostics for `decode_component_proper`/`v2`
    (`cblk_style`, progression order, `scod`) to improve triage on external
    codestream behavior.
  - Added debug-only entropy A/B probe (`JPEG2000_DEBUG_ENTROPY_AB`) in
    `decode_component_proper` for first code-block comparison:
    - `decode_block_standard_j2k` output: all-zero coefficients on the failing
      fixture sample.
    - Legacy `decode_block` output: non-zero coefficient field from the same
      payload bytes and `num_bp`.
    - Example (`rgb_8x8_lossless`, band 0, LL cb0):
      - `std_nonzero=0`
      - `legacy_nonzero=55`
  - Interpretation: segment-body bytes and bitplane count are plausibly present,
    and the highest-leverage next fix is in `decode_block_standard_j2k`
    (context/pass logic) rather than packet body extraction or level-shift.
  - Interim stabilization landed: cleanup run-mode in `decode_block_standard_j2k`
    is now disabled by default, with opt-in override
    `JPEG2000_STDJK_ENABLE_RUNMODE=1` for experimentation.
    - On local multicomponent fixture trio, mismatch count remained `3` but
      first-mismatch absolute error dropped substantially (about 4x):
      - `rgb_8x8_lossless`: `32668 -> 8092`
      - `sentinel_style_16x16_4band_lossless`: `31831 -> 7193`
      - `tiled_rgb_64x64_block32_lossless`: `32684 -> 8092`
  - Follow-up experiments after this stabilization:
    - SP/MR stripe-column traversal experiment in `decode_block_standard_j2k`
      produced mixed results and was reverted:
      - `rgb_8x8_lossless`: first-mismatch abs err improved to `99`
      - `sentinel_style_16x16_4band_lossless` / `tiled_rgb_64x64_block32_lossless`:
        first-mismatch abs err regressed to `15384` / `16284`
    - Pass-limit decoding integration (using packet-declared coding-pass totals)
      produced no measurable KPI change on local fixture trio and was reverted.
    - Debug forcing `missing_bp=0` in proper/v2 paths caused catastrophic
      over-range mismatch (`native=-65536` at first mismatch on rgb fixture)
      and was reverted.
    - Differential-helper inverse RCT experiment (`JPEG2000_DIFF_APPLY_INTERNAL_RCT`)
      worsened 3-band fixture mismatch magnitudes (`24476`) and was reverted.
    - Porting LL/LH zero-coding context lookup (from `wbjpeg2000`) into
      `decode_block_standard_j2k` significance context mapping produced no
      meaningful KPI change on the local fixture trio (still
      `multicomponent_sample_value_mismatch=3` and first-mismatch abs error
      remained around `~8k` / `~7.2k`) and was reverted.
    - Added temporary env-gated probe (`JPEG2000_DIFF_FORCE_LEGACY_T1=1`) to
      force proper-path block decode through legacy tier-1 for A/B parity
      checks. Result was mixed/regressive: `rgb_8x8` improved first mismatch
      (`8193 -> 4097`), `sentinel_style_16x16_4band` unchanged (`8192`), and
      `tiled_rgb_64x64_block32` worsened (`8193 -> 16384`), with mismatch class
      still `multicomponent_sample_value_mismatch=3`.
    - Added targeted subband probes:
      - `JPEG2000_DIFF_FORCE_LEGACY_T1_LL=1` (LL-only legacy tier-1) matched
        the all-subband legacy signature exactly on the fixture trio
        (`4097` / `8192` / `16384`, mismatch class still `=3`).
      - `JPEG2000_DIFF_FORCE_LEGACY_T1_HF=1` (HF-only legacy tier-1) stayed
        near standard baseline (`8192` / `8193` / `8192`) and did not reduce
        mismatch class (`multicomponent_sample_value_mismatch=3`).
      - This isolates current dominant divergence to LL tier-1 decode behavior,
        with HF changes only nudging low-order parity.
    - Added parity matrix runner script:
      - `dev/run_jpeg2000_parity_matrix.sh`
      - Runs baseline + key decoder profiles in one invocation and prints
        compact KPI summaries and first mismatch rows.
      - Latest matrix confirms:
        - baseline standard: (`8192` / `8193` / `8192`)
        - standard run-mode on: severe regression (`~32768` class values)
        - legacy all and legacy LL-only: identical (`4097` / `8192` / `16384`)
        - legacy HF-only: baseline-like (`8192` / `8193` / `8192`)
      - Throughput impact: captures 5 profile comparisons per iteration with a
        single command, reducing per-cycle manual command overhead.
    - Expanded matrix with LL pass-level isolation probes:
      - `JPEG2000_DIFF_LL_DISABLE_SP=1`: (`8193` / `16384` / `8193`)
      - `JPEG2000_DIFF_LL_DISABLE_MR=1`: (`16384` / `16384` / `16384`)
      - `JPEG2000_DIFF_LL_DISABLE_CL=1`: (`32768` / `32768` / `32768`)
      - All three retain mismatch class `multicomponent_sample_value_mismatch=3`,
        but magnitude escalation ranks cleanup > MR > SP.
      - Interpretation: LL cleanup semantics remain highest-priority root-cause
        lane (including interaction with run-mode gating and CL eligibility),
        with LL MR behavior as secondary lane.
    - Added targeted LL code-block A/B diagnostics in proper path
      (`JPEG2000_DEBUG_LL_BLOCK_AB=1`) for first LL block per component.
      - Example (`rgb_8x8_lossless`, comp 0, LL cb0):
        - `nnz std=60`, `no_sp=61`, `no_mr=60`, `no_cl=0`, `legacy=55`
        - `first_nz std=(-24576)`, `no_sp=(-24575)`, `no_mr=(-16384)`,
          `no_cl=None`, `legacy=(-28671)`
      - Confirms cleanup pass is structurally essential (no-CL -> all zeros)
        and that standard-vs-legacy divergence is not just sparse/non-sparse;
        reconstructed LL coefficient magnitude patterns still differ.
    - Table-driven sign-context experiment in standard decoder bool path was
      tested via full parity matrix and reverted after no KPI class improvement
      plus regression in tiled fixture first-mismatch magnitude.
    - Added LL cleanup trace counters in `decode_block_standard_j2k`
      (`JPEG2000_DEBUG_LL_CLEANUP_TRACE=1`) and ran targeted differential on
      `rgb_8x8_lossless`:
      - Component 0 LL path (bp14): `cl_eligible_pixels=64`,
        `cl_sig_decode_attempts=64`, `cl_sig=33`, `run_eligible_cols=0`.
      - Lower bitplanes then show `cl_eligible_pixels=0`, with significance
        activity shifting almost entirely to SP/MR.
      - Equivalent pattern observed for components 1 and 2 at their top
        bitplanes (`cl_sig=35` and `cl_sig=34`, respectively), again with
        `run_eligible_cols=0` while run-mode remains disabled.
      - Interpretation: current LL divergence is unlikely to be run-mode branch
        selection itself on this fixture and is more likely in cleanup
        significance/sign decode semantics or upstream context state feeding CL.
    - Added cleanup bitstream stream tracing for both standard and legacy
      decoders (`JPEG2000_DEBUG_CL_SIG_STREAM=1`) and captured side-by-side
      LL block samples via `JPEG2000_DEBUG_LL_BLOCK_AB=1` on
      `rgb_8x8_lossless`:
      - Standard CL decode attempts include mixed significance contexts
        (`ctx=0..3`) in stripe order with early samples such as
        `(bp14, idx0, ctx0, bit1)`, `(bp14, idx8, ctx1, bit1)`,
        `(bp14, idx16, ctx1, bit0)`, `(bp14, idx24, ctx0, bit1)`.
      - Legacy cleanup stream is restricted to cleanup context label 18
        (`(bp14, idx0, ctx18, bit1)`, `(bp14, idx2, ctx18, bit1)`, ...).
      - This confirms that standard and legacy CL decode decisions diverge at
        the symbol/context stream level before reconstruction, tightening
        root-cause scope to CL context/significance semantics mismatch rather
        than only downstream inverse-transform or level-shift handling.
    - Window 1 attempt 1 under hard timebox:
      - Tried LL-only correction routing zero-context cleanup samples through
        uniform context instead of `SIG[0]`.
      - Result: no mismatch-class improvement and clear regressions in
        first-mismatch magnitude (`rgb_8x8: 8192 -> 8193`,
        `sentinel_style_16x16_4band: 8193 -> 16384`,
        `tiled_rgb_64x64_block32: 8192 -> 8193`).
      - Outcome: reverted. Counts as first distinct non-winning CL-lane
        correction attempt under the stop protocol.
    - Window 1 attempt 2 under hard timebox:
      - Tried LL-only correction routing all non-run-mode cleanup
        significance samples through cleanup context 18 instead of
        `SIG[ctx]`.
      - Result: no mismatch-class improvement and strong regression to the
        cleanup-disabled error scale (`rgb_8x8: 8192 -> 32768`,
        `sentinel_style_16x16_4band: 8193 -> 32753`,
        `tiled_rgb_64x64_block32: 8192 -> 32768`).
      - Outcome: reverted. This is the second distinct non-winning CL-lane
        correction attempt, so the hard-stop criterion is now met and the
        spike should pivot to fallback posture rather than more CL speculation.
  - Net: no additional runtime fix beyond default run-mode disable passed
    acceptance; unresolved blocker remains in standard tier-1 decode semantics.
  - Single-fixture (`rgb_8x8_lossless.jp2`) debug run shows:
    - `cblk_style=0x00`, `progression=Lrcp`, `scod=0x01` (baseline coding style).
    - Non-empty code-block payload bytes are collected (e.g., 69 bytes for band 0).
    - Decoded coefficient grid remains all zeros before inverse DWT for those
      payload bytes, then level-shift produces flat ~32768 outputs.
  - This narrows the blocker: primary failure is now strongly indicated in
    tier-1 code-block entropy/packet-body interpretation (or segment assembly),
    not in post-IDWT color/level-shift handling.
  - Conclusion: remaining blocker is likely deeper in tier-1 entropy / packet
    interpretation rather than simple post-processing or off-by-one bitplane math.
  - Recommended next investigation posture after the stop decision:
    - Do not spend more Phase A time on cleanup-context remapping variants
      without new evidence.
    - Re-enter through code-block segment accounting and packet/body assembly
      validation against `wbjpeg2000`, since repeated CL-only fixes changed
      magnitude signatures but not mismatch class.
    - Build a narrower reference-trace harness for the first failing LL block
      so byte consumption, context labels, and symbol decisions can be compared
      before full-block reconstruction diverges.
    - Treat future runtime fixes as acceptable only if they survive the full
      parity matrix with no regression in existing baseline profiles.
- Completed: deterministic unit tests added for `Psot` boundary parsing and multi tile-part payload concatenation.
- Remaining: packet header parsing and progression traversal port from `wbjpeg2000` into native core.

2. Align bitstream stuffing and arithmetic decode boundary behavior.
3. Add fixture-backed regression tests that fail on lattice/checkerboard signatures.

## Regression test ideas

- Sentinel JP2 real fixture decode test:
  - Assert value range is plausible (not mostly {0, max, fixed periodic values}).
  - Assert row samples do not show period-4 repeating pattern.
- Multiband JP2 smoke:
  - Assert decode succeeds with expected sample count and no per-band aliasing.

## Exit criteria for bridge removal

- Native-only build passes all new JP2 regression tests.
- Native decode reproduces expected stats and row signatures on known problematic scenes.
- `jpeg2000-vendored-bridge` path can be disabled by default, then removed in follow-up.
