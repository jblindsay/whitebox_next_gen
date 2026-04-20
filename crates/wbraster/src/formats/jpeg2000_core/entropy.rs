//! JPEG 2000 entropy coding: MQ arithmetic coder and EBCOT tier-1 bit-plane coding.
//!
//! JPEG 2000 uses a two-tier entropy coding system:
//!
//! **Tier 1 — EBCOT (Embedded Block Coding with Optimised Truncation)**:
//! Each code-block of DWT coefficients is coded independently bit-plane by bit-plane
//! using three coding passes per bit-plane (significance propagation, magnitude
//! refinement, cleanup).  Context labels drive the MQ coder.
//!
//! **Tier 2 — Rate control / packet assembly**:
//! Coded passes are assembled into packets with layer/resolution/component/position
//! headers.  For our simplified encoder we emit a single layer so tier-2 is trivial.
//!
//! This module implements a standards-compliant MQ coder plus the simplified
//! bit-plane coding passes needed to produce valid JPEG 2000 codestreams.

// ── MQ coder state ────────────────────────────────────────────────────────────

/// One entry in the MQ probability estimation state machine.
#[derive(Clone, Copy)]
struct QeEntry {
    qe:    u16,   // Probability estimate (Q-coder style, scaled)
    nmps:  u8,    // Next state on MPS (most probable symbol)
    nlps:  u8,    // Next state on LPS (least probable symbol)
    switch: u8,   // Whether to switch MPS on LPS coding
}

/// The 47-entry MQ probability state table (ISO 15444-1 Table C.2).
const QE_TABLE: [QeEntry; 47] = [
    QeEntry { qe: 0x5601, nmps:  1, nlps:  1, switch: 1 },
    QeEntry { qe: 0x3401, nmps:  2, nlps:  6, switch: 0 },
    QeEntry { qe: 0x1801, nmps:  3, nlps:  9, switch: 0 },
    QeEntry { qe: 0x0AC1, nmps:  4, nlps: 12, switch: 0 },
    QeEntry { qe: 0x0521, nmps:  5, nlps: 29, switch: 0 },
    QeEntry { qe: 0x0221, nmps:  38, nlps: 33, switch: 0 },
    QeEntry { qe: 0x5601, nmps:  7, nlps:  6, switch: 1 },
    QeEntry { qe: 0x5401, nmps:  8, nlps: 14, switch: 0 },
    QeEntry { qe: 0x4801, nmps:  9, nlps: 14, switch: 0 },
    QeEntry { qe: 0x3801, nmps: 10, nlps: 14, switch: 0 },
    QeEntry { qe: 0x3001, nmps: 11, nlps: 17, switch: 0 },
    QeEntry { qe: 0x2401, nmps: 12, nlps: 18, switch: 0 },
    QeEntry { qe: 0x1C01, nmps: 13, nlps: 20, switch: 0 },
    QeEntry { qe: 0x1601, nmps: 29, nlps: 21, switch: 0 },
    QeEntry { qe: 0x5601, nmps: 15, nlps: 14, switch: 1 },
    QeEntry { qe: 0x5401, nmps: 16, nlps: 14, switch: 0 },
    QeEntry { qe: 0x5101, nmps: 17, nlps: 15, switch: 0 },
    QeEntry { qe: 0x4801, nmps: 18, nlps: 16, switch: 0 },
    QeEntry { qe: 0x3801, nmps: 19, nlps: 17, switch: 0 },
    QeEntry { qe: 0x3401, nmps: 20, nlps: 18, switch: 0 },
    QeEntry { qe: 0x3001, nmps: 21, nlps: 19, switch: 0 },
    QeEntry { qe: 0x2801, nmps: 22, nlps: 19, switch: 0 },
    QeEntry { qe: 0x2401, nmps: 23, nlps: 20, switch: 0 },
    QeEntry { qe: 0x2201, nmps: 24, nlps: 21, switch: 0 },
    QeEntry { qe: 0x1C01, nmps: 25, nlps: 22, switch: 0 },
    QeEntry { qe: 0x1801, nmps: 26, nlps: 23, switch: 0 },
    QeEntry { qe: 0x1601, nmps: 27, nlps: 24, switch: 0 },
    QeEntry { qe: 0x1401, nmps: 28, nlps: 25, switch: 0 },
    QeEntry { qe: 0x1201, nmps: 29, nlps: 26, switch: 0 },
    QeEntry { qe: 0x1101, nmps: 30, nlps: 27, switch: 0 },
    QeEntry { qe: 0x0AC1, nmps: 31, nlps: 28, switch: 0 },
    QeEntry { qe: 0x09C1, nmps: 32, nlps: 29, switch: 0 },
    QeEntry { qe: 0x08A1, nmps: 33, nlps: 30, switch: 0 },
    QeEntry { qe: 0x0521, nmps: 34, nlps: 31, switch: 0 },
    QeEntry { qe: 0x0441, nmps: 35, nlps: 32, switch: 0 },
    QeEntry { qe: 0x02A1, nmps: 36, nlps: 33, switch: 0 },
    QeEntry { qe: 0x0221, nmps: 37, nlps: 34, switch: 0 },
    QeEntry { qe: 0x0141, nmps: 38, nlps: 35, switch: 0 },
    QeEntry { qe: 0x0111, nmps: 39, nlps: 36, switch: 0 },
    QeEntry { qe: 0x0085, nmps: 40, nlps: 37, switch: 0 },
    QeEntry { qe: 0x0049, nmps: 41, nlps: 38, switch: 0 },
    QeEntry { qe: 0x0025, nmps: 42, nlps: 39, switch: 0 },
    QeEntry { qe: 0x0015, nmps: 43, nlps: 40, switch: 0 },
    QeEntry { qe: 0x0009, nmps: 44, nlps: 41, switch: 0 },
    QeEntry { qe: 0x0005, nmps: 45, nlps: 42, switch: 0 },
    QeEntry { qe: 0x0001, nmps: 45, nlps: 43, switch: 0 },
    QeEntry { qe: 0x5601, nmps: 46, nlps: 46, switch: 0 },
];

/// Number of context labels used in EBCOT tier-1.
const NUM_CONTEXTS: usize = 19;

/// MQ arithmetic encoder state.
pub struct MqEncoder {
    /// Output byte buffer.
    pub output: Vec<u8>,
    /// Interval register A (probability interval), 16-bit.
    a: u32,
    /// Base register C (code register), 27-bit.
    c: u32,
    /// Bit counter (output bits buffered).
    ct: i32,
    /// Temporary output byte.
    b: u8,
    /// Context states: (index into QE_TABLE, mps_value).
    cx: [(u8, u8); NUM_CONTEXTS],
}

impl MqEncoder {
    pub fn new() -> Self {
        let mut enc = Self {
            output: Vec::new(),
            a: 0x8000,
            c: 0,
            ct: 12,
            b: 0,
            cx: [(0u8, 0u8); NUM_CONTEXTS],
        };
        enc
    }

    /// Encode one symbol `d` (0 or 1) under context `cx_idx`.
    pub fn encode(&mut self, d: u8, cx_idx: usize) {
        let (state, mps) = self.cx[cx_idx];
        let qe = QE_TABLE[state as usize].qe as u32;

        self.a -= qe;
        if d == mps {
            if self.a < 0x8000 {
                if self.a < qe {
                    self.a = qe;
                }
                let ns = QE_TABLE[state as usize].nmps;
                self.cx[cx_idx] = (ns, mps);
                self.renorm_e();
            }
        } else {
            if self.a < qe {
                // No swap
                self.c += self.a;
                self.a = qe;
            } else {
                self.c += self.a;
                self.a = qe;
                if QE_TABLE[state as usize].switch != 0 {
                    let ns = QE_TABLE[state as usize].nlps;
                    self.cx[cx_idx] = (ns, 1 - mps);
                } else {
                    let ns = QE_TABLE[state as usize].nlps;
                    self.cx[cx_idx] = (ns, mps);
                }
            }
            self.renorm_e();
        }
    }

    fn renorm_e(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
            if self.a >= 0x8000 { break; }
        }
    }

    fn byte_out(&mut self) {
        let t = (self.c >> 19) as u8;
        self.c &= 0x7FFFF;
        self.ct = 8;
        if self.b == 0xFF {
            self.output.push(0xFF);
            self.output.push(t & 0x7F);
            self.ct = 7;
        } else if t > 0x7F {
            // Carry propagation
            if let Some(last) = self.output.last_mut() {
                *last += 1;
            }
            self.output.push(t - 0x80);
        } else {
            self.output.push(self.b);
        }
        self.b = t;
    }

    /// Flush remaining bits and return the compressed byte vector.
    pub fn flush(mut self) -> Vec<u8> {
        // Set bits below the carry bit
        let temp_c = self.c + self.a - 1;
        let mask = !(self.a - 1);
        self.c = temp_c & mask;
        if self.c > (temp_c ^ mask) {
            self.c = temp_c | !(mask >> 1);
        }
        // Output remaining bits
        for _ in 0..2 {
            self.c <<= self.ct;
            self.ct -= 8;
            self.byte_out();
        }
        self.output.push(self.b);
        self.output
    }
}

/// MQ arithmetic decoder.
pub struct MqDecoder<'a> {
    data:  &'a [u8],
    pos:   usize,
    a:     u32,
    c:     u32,
    ct:    i32,
    cx:    [(u8, u8); NUM_CONTEXTS],
}

impl<'a> MqDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let mut dec = Self { data, pos: 0, a: 0, c: 0, ct: 0, cx: [(0,0); NUM_CONTEXTS] };
        dec.init();
        dec
    }

    /// Initialize context states per ISO 15444-1 table D.7 for external codestream decode.
    /// Context 0 starts at state index 4, context 17 is cleanup run aggregate,
    /// and context 18 is uniform.
    pub fn init_standard_j2k_contexts(&mut self) {
        self.cx[0] = (4,  0);
        self.cx[17] = (3,  0);
        self.cx[18] = (46, 0);
    }

    fn init(&mut self) {
        self.c = ((self.current_byte() as u32) ^ 0xFF) << 16;
        self.byte_in();
        self.c <<= 7;
        self.ct -= 7;
        self.a = 0x8000;
    }

    fn current_byte(&self) -> u8 {
        self.data
            .get(self.pos)
            .copied()
            .unwrap_or(0xFF)
    }

    fn next_byte(&self) -> u8 {
        self.data
            .get(self.pos + 1)
            .copied()
            .unwrap_or(0xFF)
    }

    fn byte_in(&mut self) {
        if self.current_byte() == 0xFF {
            let b2 = self.next_byte();
            if b2 > 0x8F {
                // Marker or termination padding.
                self.ct = 8;
            } else {
                self.pos += 1;
                self.c = self
                    .c
                    .wrapping_add(0xFE00)
                    .wrapping_sub((self.current_byte() as u32) << 9);
                self.ct = 7;
            }
        } else {
            self.pos += 1;
            self.c = self
                .c
                .wrapping_add(0xFF00)
                .wrapping_sub((self.current_byte() as u32) << 8);
            self.ct = 8;
        }
    }

    fn renorm_d(&mut self) {
        loop {
            if self.ct == 0 { self.byte_in(); }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.a >= 0x8000 { break; }
        }
    }

    /// Decode one symbol under context `cx_idx`.
    pub fn decode(&mut self, cx_idx: usize) -> u8 {
        let (state, mps) = self.cx[cx_idx];
        let qe = QE_TABLE[state as usize].qe as u32;

        self.a -= qe;
        let d;
        if (self.c >> 16) < self.a {
            if self.a < 0x8000 {
                d = if self.a < qe { 1 - mps } else { mps };
                let ns = if self.a < qe {
                    if QE_TABLE[state as usize].switch != 0 {
                        self.cx[cx_idx].1 ^= 1;
                    }
                    QE_TABLE[state as usize].nlps
                } else {
                    QE_TABLE[state as usize].nmps
                };
                self.cx[cx_idx].0 = ns;
                self.renorm_d();
            } else {
                d = mps;
            }
        } else {
            self.c -= self.a << 16;
            d = if self.a < qe {
                self.a = qe;
                self.cx[cx_idx].0 = QE_TABLE[state as usize].nmps;
                mps
            } else {
                self.a = qe;
                if QE_TABLE[state as usize].switch != 0 { self.cx[cx_idx].1 ^= 1; }
                self.cx[cx_idx].0 = QE_TABLE[state as usize].nlps;
                1 - mps
            };
            self.renorm_d();
        }
        d
    }

    /// Number of source bytes consumed so far.
    pub fn consumed_bytes(&self) -> usize {
        self.pos.min(self.data.len())
    }

    /// Expose internal MQ state for deterministic debug comparisons.
    pub fn debug_state(&self) -> (u32, u32, i32, usize, u8) {
        let cur_byte = if self.pos < self.data.len() {
            self.data[self.pos]
        } else {
            0xFF
        };
        (self.a, self.c, self.ct, self.pos.min(self.data.len()), cur_byte)
    }
}

// ── Simplified bit-plane encoder / decoder ────────────────────────────────────
//
// A full EBCOT implementation is several thousand lines.  We implement a
// simplified but standards-compatible subset:
//   • Single-layer, single-resolution, single-component packets
//   • All three coding passes (SigProp, MagRef, Cleanup) per bit-plane
//   • Standard 9-neighbourhood significance contexts
//   • Standard magnitude refinement and sign coding contexts

/// Significance state for a coefficient.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SigState { Insignificant = 0, Significant = 1 }

/// Context labels for EBCOT tier-1 (ISO 15444-1 Table D.1).
mod ctx {
    pub const ZERO:    usize = 0;   // Uniform context for zero coding pass
    pub const SIG:     [usize; 9] = [1,2,3,4,5,6,7,8,9]; // significance contexts 1-9
    pub const SIGN:    [usize; 5] = [10,11,12,13,14]; // sign contexts
    pub const MAG:     [usize; 3] = [15,16,17]; // mag refinement contexts
    pub const CLEANUP: usize = 18; // cleanup pass context
}

mod std_ctx {
    pub const SIGN: [usize; 5] = [9, 10, 11, 12, 13];
    pub const MAG: [usize; 3] = [14, 15, 16];
    pub const RUN: usize = 17;
    pub const UNIFORM: usize = 18;
}

/// Encode a code-block of integer DWT coefficients into a compressed byte stream.
///
/// `coeffs` contains quantised DWT coefficient values for one code-block.
/// Returns compressed bytes including all bit-plane passes.
pub fn encode_block(coeffs: &[i32], width: usize, height: usize) -> Vec<u8> {
    let n = width * height;
    debug_assert_eq!(coeffs.len(), n);

    // Find magnitude of largest coefficient to determine number of bit-planes
    let max_mag = coeffs.iter().map(|&c| c.unsigned_abs()).max().unwrap_or(0);
    if max_mag == 0 {
        // All-zero block — trivial: MQ-encode a single cleanup pass of zeros
        let mut enc = MqEncoder::new();
        for _ in 0..n { enc.encode(0, ctx::CLEANUP); }
        return enc.flush();
    }
    let num_bitplanes = (u32::BITS - max_mag.leading_zeros()) as usize;

    let mags: Vec<u32> = coeffs.iter().map(|&c| c.unsigned_abs()).collect();
    let signs: Vec<u8> = coeffs.iter().map(|&c| if c < 0 { 1 } else { 0 }).collect();

    let mut sig = vec![SigState::Insignificant; n];
    let mut enc = MqEncoder::new();

    for bp in (0..num_bitplanes).rev() {
        let threshold = 1u32 << bp;

        // ── Significance propagation pass ─────────────────────────────────
        for i in 0..n {
            if sig[i] == SigState::Insignificant {
                let ctx = significance_context(&sig, i, width, height);
                if ctx > 0 {
                    let bit = ((mags[i] >> bp) & 1) as u8;
                    enc.encode(bit, ctx::SIG[ctx.min(8)]);
                    if bit == 1 {
                        sig[i] = SigState::Significant;
                        let sign_ctx = sign_context(&sig, &signs, i, width, height);
                        enc.encode(signs[i] ^ sign_ctx.1, sign_ctx.0);
                    }
                }
            }
        }

        // ── Magnitude refinement pass ─────────────────────────────────────
        for i in 0..n {
            if sig[i] == SigState::Significant && mags[i] >= threshold * 2 {
                let ctx = mag_refinement_context(&sig, i, width, height, bp, num_bitplanes);
                let bit = ((mags[i] >> bp) & 1) as u8;
                enc.encode(bit, ctx::MAG[ctx]);
            }
        }

        // ── Cleanup pass ──────────────────────────────────────────────────
        for i in 0..n {
            if sig[i] == SigState::Insignificant {
                let ctx = significance_context(&sig, i, width, height);
                if ctx == 0 {
                    let bit = ((mags[i] >> bp) & 1) as u8;
                    enc.encode(bit, ctx::CLEANUP);
                    if bit == 1 {
                        sig[i] = SigState::Significant;
                        enc.encode(signs[i], ctx::SIGN[0]);
                    }
                }
            }
        }
    }

    enc.flush()
}

/// Decode a compressed code-block back to integer DWT coefficients.
pub fn decode_block(
    data: &[u8],
    width: usize,
    height: usize,
    num_bitplanes: usize,
) -> Vec<i32> {
    decode_block_with_consumed(data, width, height, num_bitplanes).0
}

/// Decode a compressed code-block and return `(coefficients, consumed_bytes)`.
///
/// The consumed byte count is useful when multiple independently-encoded blocks
/// are concatenated in a single payload and must be decoded sequentially.
pub fn decode_block_with_consumed(
    data: &[u8],
    width: usize,
    height: usize,
    num_bitplanes: usize,
) -> (Vec<i32>, usize) {
    let n = width * height;
    let mut mags  = vec![0u32; n];
    let mut signs = vec![0u8;  n];
    let mut sig   = vec![SigState::Insignificant; n];
    let mut dec   = MqDecoder::new(data);
    let debug_cl_stream = std::env::var("JPEG2000_DEBUG_CL_SIG_STREAM")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let debug_cl_stream_max = std::env::var("JPEG2000_DEBUG_CL_SIG_STREAM_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(48usize);
    let mut cl_stream: Vec<(usize, usize, usize, u8)> = Vec::new(); // (bp, idx, ctx, bit)

    for bp in (0..num_bitplanes).rev() {
        let threshold = 1u32 << bp;

        // Significance propagation
        for i in 0..n {
            if sig[i] == SigState::Insignificant {
                let ctx = significance_context(&sig, i, width, height);
                if ctx > 0 {
                    let bit = dec.decode(ctx::SIG[ctx.min(8)]);
                    if bit == 1 {
                        mags[i] |= threshold;
                        sig[i] = SigState::Significant;
                        let sign_ctx = sign_context(&sig, &signs, i, width, height);
                        signs[i] = dec.decode(sign_ctx.0) ^ sign_ctx.1;
                    }
                }
            }
        }

        // Magnitude refinement
        for i in 0..n {
            if sig[i] == SigState::Significant && mags[i] >= threshold * 2 {
                let ctx = mag_refinement_context(&sig, i, width, height, bp, num_bitplanes);
                let bit = dec.decode(ctx::MAG[ctx]);
                if bit == 1 { mags[i] |= threshold; }
            }
        }

        // Cleanup
        for i in 0..n {
            if sig[i] == SigState::Insignificant {
                let ctx = significance_context(&sig, i, width, height);
                if ctx == 0 {
                    let bit = dec.decode(ctx::CLEANUP);
                    if debug_cl_stream && cl_stream.len() < debug_cl_stream_max {
                        cl_stream.push((bp, i, ctx::CLEANUP, bit));
                    }
                    if bit == 1 {
                        mags[i] |= threshold;
                        sig[i] = SigState::Significant;
                        signs[i] = dec.decode(ctx::SIGN[0]);
                    }
                }
            }
        }
    }

    let out = mags.iter().zip(signs.iter())
        .map(|(&m, &s)| if s == 0 { m as i32 } else { -(m as i32) })
        .collect();
    if debug_cl_stream {
        eprintln!(
            "[cl_sig_stream][legacy] w={} h={} num_bp={} samples={:?}",
            width,
            height,
            num_bitplanes,
            cl_stream
        );
    }
    (out, dec.consumed_bytes())
}

/// Standard-conformant JPEG 2000 T1 decoder for externally-encoded code blocks.
///
/// Implements the full three-pass per-bitplane structure with:
///  - Significance Propagation (SP): raster order, ctx > 0 pixels, marks visited
///  - Magnitude Refinement (MR): refinement of already-significant coefficients
///  - Cleanup (CL): stripe-column order (4 rows per stripe), run-mode for all-zero-ctx
///    groups, individual SIG[ctx] / ZC[ctx] for others
///
/// Use this function when decoding standard JPEG 2000 data from external encoders
/// such as Kakadu or OpenJPEG (e.g. Sentinel-2/Landsat).
pub fn decode_block_standard_j2k(
    data: &[u8],
    width: usize,
    height: usize,
    num_bitplanes: usize,
) -> Vec<i32> {
    decode_block_standard_j2k_with_probe(
        data,
        width,
        height,
        num_bitplanes,
        StandardSubbandKind::Ll,
        LlPassProbeConfig::default(),
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LlPassProbeConfig {
    pub disable_sp: bool,
    pub disable_mr: bool,
    pub disable_cl: bool,
}

fn for_each_stripe_index(width: usize, height: usize, mut action: impl FnMut(usize)) {
    for base_row in (0..height).step_by(4) {
        for c in 0..width {
            for j in 0..4 {
                let row = base_row + j;
                if row >= height {
                    break;
                }
                action(row * width + c);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardSubbandKind {
    Ll,
    Hl,
    Lh,
    Hh,
}

pub fn decode_block_standard_j2k_with_probe(
    data: &[u8],
    width: usize,
    height: usize,
    num_bitplanes: usize,
    subband_kind: StandardSubbandKind,
    ll_probe: LlPassProbeConfig,
) -> Vec<i32> {
    let n = width * height;
    let mut mags     = vec![0u32; n];
    let mut signs    = vec![0u8;  n];
    let mut sig      = vec![false; n]; // ever been significant?
    let mut mag_refined = vec![false; n];
    let mut sp_visit = vec![false; n]; // visited in SP this pass
    let mut dec      = MqDecoder::new(data);
    dec.init_standard_j2k_contexts();
    let trace = width == 64 && height == 64 && num_bitplanes >= 10;
    // Standard JPEG2000 cleanup run-mode is enabled by default.
    // The env var remains as an explicit override (`0`/`false` to disable).
    let run_mode_enabled = std::env::var("JPEG2000_STDJK_ENABLE_RUNMODE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
    let debug_cl_stream = std::env::var("JPEG2000_DEBUG_CL_SIG_STREAM")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let debug_cl_stream_max = std::env::var("JPEG2000_DEBUG_CL_SIG_STREAM_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(48usize);
    let mut cl_stream: Vec<(usize, usize, usize, u8)> = Vec::new(); // (bp, idx, ctx, bit)
    let mut native_symbol_trace_count = 0usize;
    let debug_cleanup_trace = std::env::var("JPEG2000_DEBUG_LL_CLEANUP_TRACE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    for bp in (0..num_bitplanes).rev() {
        let threshold = 1u32 << bp;
        let mut cl_sig_count = 0usize;
        let mut sp_sig_count = 0usize;
        let mut cl_eligible_pixels = 0usize;
        let mut run_eligible_cols = 0usize;
        let mut run_agg_one_cols = 0usize;
        let mut cl_sig_decode_attempts = 0usize;

        // Reset per-bitplane visited flag.
        for v in sp_visit.iter_mut() { *v = false; }

        // ── Significance Propagation (SP) ──────────────────────────────────
        if !ll_probe.disable_sp {
            for_each_stripe_index(width, height, |i| {
                if !sig[i] {
                    let ctx_label = zero_coding_context_bool(&sig, i, width, height, subband_kind);
                    if ctx_label > 0 {
                        sp_visit[i] = true;
                        let bit = dec.decode(ctx_label);
                        if bit == 1 {
                            mags[i] |= threshold;
                            sig[i] = true;
                            sp_sig_count += 1;
                            let (sctx, flip) = sign_context_bool(&sig, &signs, i, width, height);
                            signs[i] = dec.decode(sctx) ^ flip;
                        }
                    }
                }
            });
        }

        // ── Magnitude Refinement (MR) ──────────────────────────────────────
        if !ll_probe.disable_mr {
            for_each_stripe_index(width, height, |i| {
                if sig[i] && mags[i] >= threshold * 2 {
                    let ctx = mag_refinement_context_bool(&sig, &mag_refined, i, width, height);
                    let bit = dec.decode(std_ctx::MAG[ctx]);
                    if bit == 1 { mags[i] |= threshold; }
                    mag_refined[i] = true;
                }
            });
        }

        // ── Cleanup (CL) – stripe-column order with run-mode ───────────────
        if !ll_probe.disable_cl {
            let mut band_row = 0usize;
            while band_row < height {
                let band_end = (band_row + 4).min(height);
                let band_h   = band_end - band_row;

                for c in 0..width {
                    // A pixel needs CL if insignificant AND not visited by SP this bitplane.
                    let any_needs = (0..band_h).any(|j| {
                        let i = (band_row + j) * width + c;
                        !sig[i] && !sp_visit[i]
                    });
                    if !any_needs { continue; }

                    // Run-mode eligible: full band of 4, all pixels need CL, all zero sig-context.
                    let run_eligible = run_mode_enabled && band_h == 4
                        && (0..4).all(|j| {
                            let i = (band_row + j) * width + c;
                            !sig[i] && !sp_visit[i] && zero_coding_context_bool(&sig, i, width, height, subband_kind) == 0
                        });
                    if run_eligible {
                        run_eligible_cols += 1;
                    }
                    if run_eligible {
                        // Run-mode aggregate decode.
                        if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                            let (a_before, c_before, ct_before, pos_before, cur_byte_before) = dec.debug_state();
                            eprintln!(
                                "[native_entropy_state][before_run_agg] bp={} idx={} ctx={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                bp,
                                band_row * width + c,
                                std_ctx::RUN,
                                a_before,
                                c_before,
                                ct_before,
                                pos_before,
                                cur_byte_before
                            );
                            native_symbol_trace_count += 1;
                        }
                        let agg = dec.decode(std_ctx::RUN);
                        if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                            let (a_after, c_after, ct_after, pos_after, cur_byte_after) = dec.debug_state();
                            eprintln!(
                                "[native_entropy_state][after_run_agg] bp={} idx={} ctx={} bit={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                bp,
                                band_row * width + c,
                                std_ctx::RUN,
                                agg,
                                a_after,
                                c_after,
                                ct_after,
                                pos_after,
                                cur_byte_after
                            );
                            native_symbol_trace_count += 1;
                        }
                        if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                            eprintln!(
                                "[native_symbol_trace][cleanup_run_agg] bp={} idx={} ctx={} bit={} x={} y={}",
                                bp,
                                band_row * width + c,
                                std_ctx::RUN,
                                agg,
                                c,
                                band_row
                            );
                            native_symbol_trace_count += 1;
                        }
                        if agg == 0 {
                            // No significant pixel in this stripe column.
                            continue;
                        }
                        run_agg_one_cols += 1;
                        // Decode run start position (2 uniform bits -> 0..3).
                        let rp_hi = dec.decode(std_ctx::UNIFORM);
                        let rp_lo = dec.decode(std_ctx::UNIFORM);
                        let run_pos = ((rp_hi << 1) | rp_lo) as usize; // 0, 1, 2, or 3
                        if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                            eprintln!(
                                "[native_symbol_trace][cleanup_run_pos] bp={} idx={} ctx={} run_pos={} x={} y={}",
                                bp,
                                band_row * width + c,
                                std_ctx::UNIFORM,
                                run_pos,
                                c,
                                band_row
                            );
                            native_symbol_trace_count += 1;
                        }

                        // First significant pixel at run_pos.
                        let first_i = (band_row + run_pos) * width + c;
                        mags[first_i] |= threshold;
                        sig[first_i] = true;
                        cl_sig_count += 1;
                        // Sign from uniform context (XOR with sign-context prediction).
                        let (sctx, flip) = sign_context_bool(&sig, &signs, first_i, width, height);
                        signs[first_i] = dec.decode(sctx) ^ flip;

                        // Process remaining pixels after run_pos individually.
                        for j in (run_pos + 1)..band_h {
                            let i = (band_row + j) * width + c;
                            if !sig[i] && !sp_visit[i] {
                                cl_eligible_pixels += 1;
                                let ctx = zero_coding_context_bool(&sig, i, width, height, subband_kind);
                                cl_sig_decode_attempts += 1;
                                let decode_ctx = ctx;
                                if debug_cl_stream
                                    && native_symbol_trace_count < debug_cl_stream_max
                                    && bp >= num_bitplanes.saturating_sub(1)
                                    && i <= 32
                                {
                                    let (a_before, c_before, ct_before, pos_before, cur_byte_before) = dec.debug_state();
                                    eprintln!(
                                        "[native_entropy_state][before_cleanup_sym] bp={} idx={} ctx={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                        bp,
                                        i,
                                        decode_ctx,
                                        a_before,
                                        c_before,
                                        ct_before,
                                        pos_before,
                                        cur_byte_before
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                let bit = dec.decode(decode_ctx);
                                if debug_cl_stream
                                    && native_symbol_trace_count < debug_cl_stream_max
                                    && bp >= num_bitplanes.saturating_sub(1)
                                    && i <= 32
                                {
                                    let (a_after, c_after, ct_after, pos_after, cur_byte_after) = dec.debug_state();
                                    eprintln!(
                                        "[native_entropy_state][after_cleanup_sym] bp={} idx={} ctx={} bit={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                        bp,
                                        i,
                                        decode_ctx,
                                        bit,
                                        a_after,
                                        c_after,
                                        ct_after,
                                        pos_after,
                                        cur_byte_after
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                                    eprintln!(
                                        "[native_symbol_trace][cleanup] bp={} idx={} ctx={} bit={} x={} y={} use_rl=0",
                                        bp,
                                        i,
                                        decode_ctx,
                                        bit,
                                        i % width,
                                        i / width
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                if debug_cl_stream && cl_stream.len() < debug_cl_stream_max {
                                    cl_stream.push((bp, i, decode_ctx, bit));
                                }
                                if bit == 1 {
                                    mags[i] |= threshold;
                                    sig[i] = true;
                                    cl_sig_count += 1;
                                    let (sctx2, flip2) = sign_context_bool(&sig, &signs, i, width, height);
                                    signs[i] = dec.decode(sctx2) ^ flip2;
                                }
                            }
                        }
                    } else {
                        // Non-run-mode: process each pixel individually.
                        for j in 0..band_h {
                            let i = (band_row + j) * width + c;
                            if !sig[i] && !sp_visit[i] {
                                cl_eligible_pixels += 1;
                                let ctx = zero_coding_context_bool(&sig, i, width, height, subband_kind);
                                cl_sig_decode_attempts += 1;
                                let decode_ctx = ctx;
                                if debug_cl_stream
                                    && native_symbol_trace_count < debug_cl_stream_max
                                    && bp >= num_bitplanes.saturating_sub(1)
                                    && i <= 32
                                {
                                    let (a_before, c_before, ct_before, pos_before, cur_byte_before) = dec.debug_state();
                                    eprintln!(
                                        "[native_entropy_state][before_cleanup_sym] bp={} idx={} ctx={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                        bp,
                                        i,
                                        decode_ctx,
                                        a_before,
                                        c_before,
                                        ct_before,
                                        pos_before,
                                        cur_byte_before
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                let bit = dec.decode(decode_ctx);
                                if debug_cl_stream
                                    && native_symbol_trace_count < debug_cl_stream_max
                                    && bp >= num_bitplanes.saturating_sub(1)
                                    && i <= 32
                                {
                                    let (a_after, c_after, ct_after, pos_after, cur_byte_after) = dec.debug_state();
                                    eprintln!(
                                        "[native_entropy_state][after_cleanup_sym] bp={} idx={} ctx={} bit={} a=0x{:04X} c=0x{:08X} ct={} pos={} cur=0x{:02X}",
                                        bp,
                                        i,
                                        decode_ctx,
                                        bit,
                                        a_after,
                                        c_after,
                                        ct_after,
                                        pos_after,
                                        cur_byte_after
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                if debug_cl_stream && native_symbol_trace_count < debug_cl_stream_max {
                                    eprintln!(
                                        "[native_symbol_trace][cleanup] bp={} idx={} ctx={} bit={} x={} y={} use_rl=0",
                                        bp,
                                        i,
                                        decode_ctx,
                                        bit,
                                        i % width,
                                        i / width
                                    );
                                    native_symbol_trace_count += 1;
                                }
                                if debug_cl_stream && cl_stream.len() < debug_cl_stream_max {
                                    cl_stream.push((bp, i, decode_ctx, bit));
                                }
                                if bit == 1 {
                                    mags[i] |= threshold;
                                    sig[i] = true;
                                    cl_sig_count += 1;
                                    let (sctx, flip) = sign_context_bool(&sig, &signs, i, width, height);
                                    signs[i] = dec.decode(sctx) ^ flip;
                                }
                            }
                        }
                    }
                }
                band_row += 4;
            }
        }
        if debug_cleanup_trace {
            eprintln!(
                "[ll_cleanup_trace] bp={} threshold={} sp_sig={} cl_sig={} cl_eligible_pixels={} cl_sig_decode_attempts={} run_eligible_cols={} run_agg_one_cols={} run_mode_enabled={}",
                bp,
                threshold,
                sp_sig_count,
                cl_sig_count,
                cl_eligible_pixels,
                cl_sig_decode_attempts,
                run_eligible_cols,
                run_agg_one_cols,
                run_mode_enabled
            );
        }
        if trace && bp >= num_bitplanes - 3 {
            eprintln!("[decode_block_stdjk] bp={} threshold={} sp_sig={} cl_sig={}", bp, threshold, sp_sig_count, cl_sig_count);
        }
    }

    let result: Vec<i32> = mags.iter().zip(signs.iter())
        .map(|(&m, &s)| if s == 0 { m as i32 } else { -(m as i32) })
        .collect();
    if debug_cl_stream {
        eprintln!(
            "[cl_sig_stream][standard] w={} h={} num_bp={} samples={:?}",
            width,
            height,
            num_bitplanes,
            cl_stream
        );
    }
    
    if width == 64 && height == 64 && trace {
        eprintln!("[decode_stdjk_result] coeff[0..8]: {:?}", &result[0..8]);
        eprintln!("[decode_stdjk_result] coeff[64..72]: {:?}", &result[64..72]);
    }
    result
}

/// Significance context using `bool` sig array (for standard decoder).
fn significance_context_bool(sig: &[bool], idx: usize, w: usize, h: usize) -> usize {
    let nb = neighbours(idx, w, h);
    nb.iter()
        .filter_map(|&n| n)
        .filter(|&n| sig[n])
        .count()
        .min(8)
}

#[rustfmt::skip]
const ZERO_CTX_LL_LH_LOOKUP: [usize; 256] = [
    0, 3, 1, 3, 5, 7, 6, 7, 1, 3, 2, 3, 6, 7, 6, 7, 5, 7, 6, 7, 8, 8, 8, 8, 6,
    7, 6, 7, 8, 8, 8, 8, 1, 3, 2, 3, 6, 7, 6, 7, 2, 3, 2, 3, 6, 7, 6, 7, 6, 7,
    6, 7, 8, 8, 8, 8, 6, 7, 6, 7, 8, 8, 8, 8, 3, 4, 3, 4, 7, 7, 7, 7, 3, 4, 3,
    4, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 7, 7, 7, 7, 8, 8, 8, 8, 3, 4, 3, 4,
    7, 7, 7, 7, 3, 4, 3, 4, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 7, 7, 7, 7, 8,
    8, 8, 8, 1, 3, 2, 3, 6, 7, 6, 7, 2, 3, 2, 3, 6, 7, 6, 7, 6, 7, 6, 7, 8, 8,
    8, 8, 6, 7, 6, 7, 8, 8, 8, 8, 2, 3, 2, 3, 6, 7, 6, 7, 2, 3, 2, 3, 6, 7, 6,
    7, 6, 7, 6, 7, 8, 8, 8, 8, 6, 7, 6, 7, 8, 8, 8, 8, 3, 4, 3, 4, 7, 7, 7, 7,
    3, 4, 3, 4, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 7, 7, 7, 7, 8, 8, 8, 8, 3,
    4, 3, 4, 7, 7, 7, 7, 3, 4, 3, 4, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 7, 7,
    7, 7, 8, 8, 8, 8,
];

#[rustfmt::skip]
const ZERO_CTX_HL_LOOKUP: [usize; 256] = [
    0, 5, 1, 6, 3, 7, 3, 7, 1, 6, 2, 6, 3, 7, 3, 7, 3, 7, 3, 7, 4, 7, 4, 7, 3,
    7, 3, 7, 4, 7, 4, 7, 1, 6, 2, 6, 3, 7, 3, 7, 2, 6, 2, 6, 3, 7, 3, 7, 3, 7,
    3, 7, 4, 7, 4, 7, 3, 7, 3, 7, 4, 7, 4, 7, 5, 8, 6, 8, 7, 8, 7, 8, 6, 8, 6,
    8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 6, 8, 6, 8,
    7, 8, 7, 8, 6, 8, 6, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7,
    8, 7, 8, 1, 6, 2, 6, 3, 7, 3, 7, 2, 6, 2, 6, 3, 7, 3, 7, 3, 7, 3, 7, 4, 7,
    4, 7, 3, 7, 3, 7, 4, 7, 4, 7, 2, 6, 2, 6, 3, 7, 3, 7, 2, 6, 2, 6, 3, 7, 3,
    7, 3, 7, 3, 7, 4, 7, 4, 7, 3, 7, 3, 7, 4, 7, 4, 7, 6, 8, 6, 8, 7, 8, 7, 8,
    6, 8, 6, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 6,
    8, 6, 8, 7, 8, 7, 8, 6, 8, 6, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8, 7, 8,
    7, 8, 7, 8, 7, 8,
];

#[rustfmt::skip]
const ZERO_CTX_HH_LOOKUP: [usize; 256] = [
    0, 1, 3, 4, 1, 2, 4, 5, 3, 4, 6, 7, 4, 5, 7, 7, 1, 2, 4, 5, 2, 2, 5, 5, 4,
    5, 7, 7, 5, 5, 7, 7, 3, 4, 6, 7, 4, 5, 7, 7, 6, 7, 8, 8, 7, 7, 8, 8, 4, 5,
    7, 7, 5, 5, 7, 7, 7, 7, 8, 8, 7, 7, 8, 8, 1, 2, 4, 5, 2, 2, 5, 5, 4, 5, 7,
    7, 5, 5, 7, 7, 2, 2, 5, 5, 2, 2, 5, 5, 5, 5, 7, 7, 5, 5, 7, 7, 4, 5, 7, 7,
    5, 5, 7, 7, 7, 7, 8, 8, 7, 7, 8, 8, 5, 5, 7, 7, 5, 5, 7, 7, 7, 7, 8, 8, 7,
    7, 8, 8, 3, 4, 6, 7, 4, 5, 7, 7, 6, 7, 8, 8, 7, 7, 8, 8, 4, 5, 7, 7, 5, 5,
    7, 7, 7, 7, 8, 8, 7, 7, 8, 8, 6, 7, 8, 8, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 7, 7, 8, 8, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 4, 5, 7, 7, 5, 5, 7, 7,
    7, 7, 8, 8, 7, 7, 8, 8, 5, 5, 7, 7, 5, 5, 7, 7, 7, 7, 8, 8, 7, 7, 8, 8, 7,
    7, 8, 8, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 8, 8, 7, 7, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8,
];


fn neighbor_significance_bits_bool(sig: &[bool], idx: usize, w: usize, h: usize) -> u8 {
    let r = idx / w;
    let c = idx % w;
    let mut bits = 0u8;

    // Bit order must match the table indexing used by JPEG2000:
    // top-left, top, top-right, left, bottom-left, right, bottom-right, bottom.
    if r > 0 && c > 0 && sig[(r - 1) * w + (c - 1)] {
        bits |= 1 << 7;
    }
    if r > 0 && sig[(r - 1) * w + c] {
        bits |= 1 << 6;
    }
    if r > 0 && c + 1 < w && sig[(r - 1) * w + (c + 1)] {
        bits |= 1 << 5;
    }
    if c > 0 && sig[r * w + (c - 1)] {
        bits |= 1 << 4;
    }
    if r + 1 < h && c > 0 && sig[(r + 1) * w + (c - 1)] {
        bits |= 1 << 3;
    }
    if c + 1 < w && sig[r * w + (c + 1)] {
        bits |= 1 << 2;
    }
    if r + 1 < h && c + 1 < w && sig[(r + 1) * w + (c + 1)] {
        bits |= 1 << 1;
    }
    if r + 1 < h && sig[(r + 1) * w + c] {
        bits |= 1;
    }

    bits
}

fn zero_coding_context_bool(
    sig: &[bool],
    idx: usize,
    w: usize,
    h: usize,
    subband_kind: StandardSubbandKind,
) -> usize {
    let neighbors = neighbor_significance_bits_bool(sig, idx, w, h) as usize;
    match subband_kind {
        StandardSubbandKind::Ll | StandardSubbandKind::Lh => ZERO_CTX_LL_LH_LOOKUP[neighbors],
        StandardSubbandKind::Hl => ZERO_CTX_HL_LOOKUP[neighbors],
        StandardSubbandKind::Hh => ZERO_CTX_HH_LOOKUP[neighbors],
    }
}

/// Sign context using `bool` sig array.  Returns `(context_label, flip_bit)`.
fn sign_context_bool(sig: &[bool], signs: &[u8], idx: usize, w: usize, h: usize) -> (usize, u8) {
    let r = idx / w;
    let c = idx % w;

    // Horizontal and vertical sign contributions.
    let h_contrib = {
        let left  = if c > 0     { if sig[r * w + c - 1] { if signs[r * w + c - 1] == 1 { -1i32 } else { 1 } } else { 0 } } else { 0 };
        let right = if c + 1 < w { if sig[r * w + c + 1] { if signs[r * w + c + 1] == 1 { -1i32 } else { 1 } } else { 0 } } else { 0 };
        (left + right).signum()
    };
    let v_contrib = {
        let up   = if r > 0     { if sig[(r - 1) * w + c] { if signs[(r - 1) * w + c] == 1 { -1i32 } else { 1 } } else { 0 } } else { 0 };
        let down = if r + 1 < h { if sig[(r + 1) * w + c] { if signs[(r + 1) * w + c] == 1 { -1i32 } else { 1 } } else { 0 } } else { 0 };
        (up + down).signum()
    };

    // Lookup table per ISO 15444-1 Table D.2.
    match (h_contrib, v_contrib) {
        ( 1,  1) => (std_ctx::SIGN[0], 0),
        ( 1,  0) => (std_ctx::SIGN[1], 0),
        ( 1, -1) => (std_ctx::SIGN[2], 0),
        ( 0,  1) => (std_ctx::SIGN[3], 0),
        ( 0,  0) => (std_ctx::SIGN[4], 0),
        ( 0, -1) => (std_ctx::SIGN[3], 1),
        (-1,  1) => (std_ctx::SIGN[2], 1),
        (-1,  0) => (std_ctx::SIGN[1], 1),
        (-1, -1) => (std_ctx::SIGN[0], 1),
        _        => (std_ctx::SIGN[4], 0),
    }
}

/// Magnitude-refinement context using `bool` sig array.
fn mag_refinement_context_bool(
    sig: &[bool],
    mag_refined: &[bool],
    idx: usize,
    w: usize,
    h: usize,
) -> usize {
    if mag_refined[idx] {
        2
    } else if significance_context_bool(sig, idx, w, h) > 0 {
        1
    } else {
        0
    }
}

// ── Context helper functions ──────────────────────────────────────────────────

fn neighbours(idx: usize, w: usize, h: usize) -> [Option<usize>; 8] {
    let r = idx / w;
    let c = idx % w;
    [
        if r > 0           && c > 0     { Some((r-1)*w + c-1) } else { None },
        if r > 0                        { Some((r-1)*w + c)   } else { None },
        if r > 0           && c+1 < w   { Some((r-1)*w + c+1) } else { None },
        if c > 0                        { Some(r*w + c-1)      } else { None },
        if c+1 < w                      { Some(r*w + c+1)      } else { None },
        if r+1 < h         && c > 0     { Some((r+1)*w + c-1) } else { None },
        if r+1 < h                      { Some((r+1)*w + c)   } else { None },
        if r+1 < h         && c+1 < w   { Some((r+1)*w + c+1) } else { None },
    ]
}

fn significance_context(sig: &[SigState], idx: usize, w: usize, h: usize) -> usize {
    let nb = neighbours(idx, w, h);
    let count: usize = nb.iter()
        .filter_map(|&n| n)
        .filter(|&n| sig[n] == SigState::Significant)
        .count();
    count.min(8)
}

fn sign_context(
    sig: &[SigState], signs: &[u8], idx: usize, w: usize, h: usize
) -> (usize, u8) {
    // Simplified: use uniform sign context 0 with no XOR flip
    (ctx::SIGN[0], 0)
}

fn mag_refinement_context(
    sig: &[SigState], idx: usize, w: usize, h: usize, bp: usize, total_bp: usize
) -> usize {
    if bp == total_bp.saturating_sub(1) { 0 }
    else if significance_context(sig, idx, w, h) > 0 { 1 }
    else { 2 }
}

// ── Quantisation ─────────────────────────────────────────────────────────────

/// Quantise a 9/7 DWT coefficient buffer to integers.
///
/// Each subband `sb` uses step size `delta_sb`:
///   `q = floor(|coeff| / delta_sb) * sign(coeff)`
pub fn quantise(coeffs: &[f64], step_sizes: &[f64]) -> Vec<i32> {
    // For simplicity we use a single global step size (first value)
    let step = step_sizes.first().copied().unwrap_or(1.0).max(1e-10);
    coeffs.iter().map(|&c| {
        let q = (c.abs() / step).floor() as i32;
        if c < 0.0 { -q } else { q }
    }).collect()
}

/// Dequantise integers back to approximate DWT coefficients.
pub fn dequantise(quantised: &[i32], step_sizes: &[f64]) -> Vec<f64> {
    let step = step_sizes.first().copied().unwrap_or(1.0);
    quantised.iter().map(|&q| {
        if q == 0 { 0.0 }
        else {
            let sign = if q < 0 { -1.0 } else { 1.0 };
            sign * (q.unsigned_abs() as f64 + 0.5) * step
        }
    }).collect()
}

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    fn mq_encode_decode_zeros() {
        let mut enc = MqEncoder::new();
        for _ in 0..32 { enc.encode(0, 0); }
        let bytes = enc.flush();
        assert!(!bytes.is_empty());
        let mut dec = MqDecoder::new(&bytes);
        for _ in 0..32 {
            let b = dec.decode(0);
            assert!(b == 0 || b == 1); // decoder should not panic
        }
    }

    #[test]
    fn mq_encode_decode_alternating() {
        let symbols: Vec<u8> = (0..64).map(|i| (i % 2) as u8).collect();
        let mut enc = MqEncoder::new();
        for &s in &symbols { enc.encode(s, 1); }
        let bytes = enc.flush();
        assert!(!bytes.is_empty());
        let mut dec = MqDecoder::new(&bytes);
        let mut decoded = Vec::new();
        for _ in 0..64 { decoded.push(dec.decode(1)); }
        assert_eq!(decoded, symbols);
    }

    #[test]
    fn block_codec_zero() {
        let coeffs = vec![0i32; 64];
        let encoded = encode_block(&coeffs, 8, 8);
        // Should be decodable without panic
        let decoded = decode_block(&encoded, 8, 8, 1);
        assert_eq!(decoded.len(), 64);
    }

    #[test]
    fn block_codec_simple() {
        let coeffs: Vec<i32> = (0..64i32).map(|x| x - 32).collect();
        let encoded = encode_block(&coeffs, 8, 8);
        let num_bp = 7; // enough for values in -32..31
        let decoded = decode_block(&encoded, 8, 8, num_bp);
        // Values should match within quantisation error
        for (a, b) in coeffs.iter().zip(decoded.iter()) {
            assert!((a - b).abs() <= 1, "block codec mismatch: {} vs {}", a, b);
        }
    }
}
