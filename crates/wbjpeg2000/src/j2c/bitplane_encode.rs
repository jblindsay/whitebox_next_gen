//! EBCOT Tier-1 encoder for JPEG 2000 (ITU-T T.800 Annex D).
//!
//! Encodes quantized wavelet coefficients into code-block bitstreams using:
//! - MQ arithmetic coding
//! - Context-dependent coding with the same 19 contexts as the decoder
//! - Three passes per bitplane: significance propagation, magnitude refinement, cleanup
//! - Column-stripe scanning order (4-row stripes)

use alloc::vec;
use alloc::vec::Vec;

use super::arithmetic_encoder::{ArithmeticEncoder, ArithmeticEncoderContext};
use super::build::SubBandType;

/// Coefficient state flags.
const SIGNIFICANT: u8 = 1 << 7;
const MAGNITUDE_REFINED: u8 = 1 << 6;
const CODED_IN_CURRENT_PASS: u8 = 1 << 5;

/// Result of encoding a single code-block.
#[derive(Debug)]
pub(crate) struct EncodedCodeBlock {
    /// The compressed bitstream data.
    pub(crate) data: Vec<u8>,
    /// Number of coding passes actually generated.
    pub(crate) num_coding_passes: u8,
    /// Number of leading zero bitplanes (missing MSBs).
    pub(crate) num_zero_bitplanes: u8,
}

/// Context labels for zero coding (Table D.1).
/// Index into 256-entry lookup tables by neighbor significance pattern.
#[rustfmt::skip]
const ZERO_CTX_LL_LH: [u8; 256] = [
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
const ZERO_CTX_HL: [u8; 256] = [
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
const ZERO_CTX_HH: [u8; 256] = [
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

/// Sign coding context lookup (Table D.2), matching the decoder's convention.
///
/// The index is built by combining significance and sign of the 4 cardinal
/// neighbors into a merged byte:
///   1. significances = neighbor_byte & 0b01010101 (keep T(6), L(4), R(2), B(0))
///   2. signs = (top_sign << 6) | (left_sign << 4) | (right_sign << 2) | bottom_sign
///   3. negative_sigs = significances & signs
///   4. positive_sigs = significances & !signs
///   5. merged = (negative_sigs << 1) | positive_sigs
///
/// Each entry is (context_label, xor_bit). (0,0) represents impossible combinations.
#[rustfmt::skip]
const SIGN_CONTEXT_LOOKUP: [(u8, u8); 256] = [
    (9,0), (10,0), (10,1), (0,0), (12,0), (13,0), (11,0), (0,0), (12,1), (11,1),
    (13,1), (0,0), (0,0), (0,0), (0,0), (0,0), (12,0), (13,0), (11,0), (0,0),
    (12,0), (13,0), (11,0), (0,0), (9,0), (10,0), (10,1), (0,0), (0,0), (0,0),
    (0,0), (0,0), (12,1), (11,1), (13,1), (0,0), (9,0), (10,0), (10,1), (0,0),
    (12,1), (11,1), (13,1), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (10,0), (10,0), (9,0), (0,0), (13,0), (13,0), (12,0),
    (0,0), (11,1), (11,1), (12,1), (0,0), (0,0), (0,0), (0,0), (0,0), (13,0),
    (13,0), (12,0), (0,0), (13,0), (13,0), (12,0), (0,0), (10,0), (10,0), (9,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (11,1), (11,1), (12,1), (0,0), (10,0),
    (10,0), (9,0), (0,0), (11,1), (11,1), (12,1), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (10,1), (9,0), (10,1), (0,0),
    (11,0), (12,0), (11,0), (0,0), (13,1), (12,1), (13,1), (0,0), (0,0), (0,0),
    (0,0), (0,0), (11,0), (12,0), (11,0), (0,0), (11,0), (12,0), (11,0), (0,0),
    (10,1), (9,0), (10,1), (0,0), (0,0), (0,0), (0,0), (0,0), (13,1), (12,1),
    (13,1), (0,0), (10,1), (9,0), (10,1), (0,0), (13,1), (12,1), (13,1), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
    (0,0), (0,0), (0,0), (0,0), (0,0), (0,0), (0,0),
];

/// Encode a single code-block's quantized coefficients.
///
/// `coefficients` are quantized i32 values in row-major order.
/// `width`, `height` are the code-block dimensions.
/// `sub_band_type` determines which zero-coding context table to use.
/// `total_bitplanes` is the JPEG 2000 `Mb` value for this subband/code-block.
pub(crate) fn encode_code_block(
    coefficients: &[i32],
    width: u32,
    height: u32,
    sub_band_type: SubBandType,
    total_bitplanes: u8,
) -> EncodedCodeBlock {
    let w = width as usize;
    let h = height as usize;

    // Determine maximum magnitude and number of bitplanes
    let max_magnitude = coefficients
        .iter()
        .map(|c| c.unsigned_abs())
        .max()
        .unwrap_or(0);

    if max_magnitude == 0 {
        return EncodedCodeBlock {
            data: Vec::new(),
            num_coding_passes: 0,
            num_zero_bitplanes: total_bitplanes,
        };
    }

    let num_bitplanes = 32 - max_magnitude.leading_zeros();
    debug_assert!(num_bitplanes as u8 <= total_bitplanes);
    let num_zero_bitplanes = total_bitplanes.saturating_sub(num_bitplanes as u8);

    // Build coefficient magnitude and sign arrays
    let pw = w + 2; // Padded width for neighbor access
    let ph = h + 2;
    let mut magnitudes = vec![0u32; pw * ph];
    let mut signs = vec![false; pw * ph];
    let mut states = vec![0u8; pw * ph];
    let mut neighbors = vec![0u8; pw * ph]; // Packed neighbor significances

    for y in 0..h {
        for x in 0..w {
            let idx = (y + 1) * pw + (x + 1);
            let coeff = coefficients[y * w + x];
            magnitudes[idx] = coeff.unsigned_abs();
            signs[idx] = coeff < 0;
        }
    }

    let mut encoder = ArithmeticEncoder::new();
    let mut contexts = [ArithmeticEncoderContext::default(); 19];
    // Initialize contexts per spec
    contexts[0].reset_with_index(4);
    contexts[17].reset_with_index(3); // RLC context
    contexts[18].reset_with_index(46); // UNIFORM context

    let mut num_coding_passes = 0u8;

    // Process bitplanes from MSB to LSB
    for bp in (0..num_bitplanes).rev() {
        let bit_mask = 1u32 << bp;
        let is_first_bitplane = bp == num_bitplanes - 1;

        if is_first_bitplane {
            // First bitplane: cleanup pass only
            cleanup_pass(
                &magnitudes,
                &signs,
                &mut states,
                &mut neighbors,
                &mut encoder,
                &mut contexts,
                w,
                h,
                pw,
                bit_mask,
                sub_band_type,
            );
            num_coding_passes += 1;
        } else {
            // Subsequent bitplanes: SPP, MRP, Cleanup
            significance_propagation_pass(
                &magnitudes,
                &signs,
                &mut states,
                &mut neighbors,
                &mut encoder,
                &mut contexts,
                w,
                h,
                pw,
                bit_mask,
                sub_band_type,
            );
            num_coding_passes += 1;

            magnitude_refinement_pass(
                &magnitudes,
                &mut states,
                &mut neighbors,
                &mut encoder,
                &mut contexts,
                w,
                h,
                pw,
                bit_mask,
            );
            num_coding_passes += 1;

            cleanup_pass(
                &magnitudes,
                &signs,
                &mut states,
                &mut neighbors,
                &mut encoder,
                &mut contexts,
                w,
                h,
                pw,
                bit_mask,
                sub_band_type,
            );
            num_coding_passes += 1;
        }

        // Clear coded-in-current-pass flags
        for s in states.iter_mut() {
            *s &= !CODED_IN_CURRENT_PASS;
        }
    }

    let data = encoder.finish();

    EncodedCodeBlock {
        data,
        num_coding_passes,
        num_zero_bitplanes,
    }
}

/// Significance Propagation Pass (D.3.1)
fn significance_propagation_pass(
    magnitudes: &[u32],
    signs: &[bool],
    states: &mut [u8],
    neighbors: &mut [u8],
    encoder: &mut ArithmeticEncoder,
    contexts: &mut [ArithmeticEncoderContext; 19],
    w: usize,
    h: usize,
    pw: usize,
    bit_mask: u32,
    sub_band_type: SubBandType,
) {
    for y_base in (0..h).step_by(4) {
        for x in 0..w {
            let y_end = (y_base + 4).min(h);
            for y in y_base..y_end {
                let idx = (y + 1) * pw + (x + 1);
                let is_significant = states[idx] & SIGNIFICANT != 0;
                let has_sig_neighbors = neighbors[idx] != 0;

                if !is_significant && has_sig_neighbors {
                    let ctx_label = zero_coding_ctx(neighbors[idx], sub_band_type);
                    let bit = (magnitudes[idx] & bit_mask != 0) as u32;
                    encoder.encode(bit, &mut contexts[ctx_label as usize]);
                    states[idx] |= CODED_IN_CURRENT_PASS;

                    if bit == 1 {
                        encode_sign(
                            idx, signs[idx], neighbors, signs, states, encoder, contexts, pw,
                        );
                        set_significant(idx, states, neighbors, pw);
                    }
                }
            }
        }
    }
}

/// Magnitude Refinement Pass (D.3.3)
fn magnitude_refinement_pass(
    magnitudes: &[u32],
    states: &mut [u8],
    neighbors: &mut [u8],
    encoder: &mut ArithmeticEncoder,
    contexts: &mut [ArithmeticEncoderContext; 19],
    w: usize,
    h: usize,
    pw: usize,
    bit_mask: u32,
) {
    for y_base in (0..h).step_by(4) {
        for x in 0..w {
            let y_end = (y_base + 4).min(h);
            for y in y_base..y_end {
                let idx = (y + 1) * pw + (x + 1);
                let is_significant = states[idx] & SIGNIFICANT != 0;
                let coded_this_pass = states[idx] & CODED_IN_CURRENT_PASS != 0;

                if is_significant && !coded_this_pass {
                    let ctx_label = magnitude_refinement_ctx(states[idx], neighbors[idx]);
                    let bit = (magnitudes[idx] & bit_mask != 0) as u32;
                    encoder.encode(bit, &mut contexts[ctx_label as usize]);
                    states[idx] |= MAGNITUDE_REFINED;
                }
            }
        }
    }
}

/// Cleanup Pass (D.3.4)
fn cleanup_pass(
    magnitudes: &[u32],
    signs: &[bool],
    states: &mut [u8],
    neighbors: &mut [u8],
    encoder: &mut ArithmeticEncoder,
    contexts: &mut [ArithmeticEncoderContext; 19],
    w: usize,
    h: usize,
    pw: usize,
    bit_mask: u32,
    sub_band_type: SubBandType,
) {
    for y_base in (0..h).step_by(4) {
        for x in 0..w {
            let y_end = (y_base + 4).min(h);
            let stripe_height = y_end - y_base;

            // Try run-length coding for full 4-row stripes
            if stripe_height == 4 {
                let mut all_zero_uncoded = true;
                for y in y_base..y_end {
                    let idx = (y + 1) * pw + (x + 1);
                    if states[idx] & (SIGNIFICANT | CODED_IN_CURRENT_PASS) != 0
                        || neighbors[idx] != 0
                    {
                        all_zero_uncoded = false;
                        break;
                    }
                }

                if all_zero_uncoded {
                    // Check if any coefficient in this stripe becomes significant
                    let mut first_sig = None;
                    for (j, y) in (y_base..y_end).enumerate() {
                        let idx = (y + 1) * pw + (x + 1);
                        if magnitudes[idx] & bit_mask != 0 {
                            first_sig = Some(j);
                            break;
                        }
                    }

                    if let Some(pos) = first_sig {
                        // Not all zero: encode RLC=1, then position
                        encoder.encode(1, &mut contexts[17]); // RLC context
                        encoder.encode((pos >> 1) as u32 & 1, &mut contexts[18]); // UNIFORM
                        encoder.encode(pos as u32 & 1, &mut contexts[18]); // UNIFORM

                        // Encode sign for the first significant
                        let y = y_base + pos;
                        let idx = (y + 1) * pw + (x + 1);
                        encode_sign(
                            idx, signs[idx], neighbors, signs, states, encoder, contexts, pw,
                        );
                        set_significant(idx, states, neighbors, pw);

                        // Continue cleanup for remaining samples in stripe
                        for y in (y_base + pos + 1)..y_end {
                            let idx = (y + 1) * pw + (x + 1);
                            if states[idx] & (SIGNIFICANT | CODED_IN_CURRENT_PASS) == 0 {
                                let ctx_label = zero_coding_ctx(neighbors[idx], sub_band_type);
                                let bit = (magnitudes[idx] & bit_mask != 0) as u32;
                                encoder.encode(bit, &mut contexts[ctx_label as usize]);
                                if bit == 1 {
                                    encode_sign(
                                        idx, signs[idx], neighbors, signs, states, encoder,
                                        contexts, pw,
                                    );
                                    set_significant(idx, states, neighbors, pw);
                                }
                            }
                        }
                        continue;
                    } else {
                        // All zero: encode RLC=0
                        encoder.encode(0, &mut contexts[17]);
                        continue;
                    }
                }
            }

            // Non-RLC: process each sample individually
            for y in y_base..y_end {
                let idx = (y + 1) * pw + (x + 1);
                if states[idx] & (SIGNIFICANT | CODED_IN_CURRENT_PASS) == 0 {
                    let ctx_label = zero_coding_ctx(neighbors[idx], sub_band_type);
                    let bit = (magnitudes[idx] & bit_mask != 0) as u32;
                    encoder.encode(bit, &mut contexts[ctx_label as usize]);
                    if bit == 1 {
                        encode_sign(
                            idx, signs[idx], neighbors, signs, states, encoder, contexts, pw,
                        );
                        set_significant(idx, states, neighbors, pw);
                    }
                }
            }
        }
    }
}

/// Encode the sign of a newly significant coefficient.
///
/// The sign context is computed exactly as the decoder does it:
/// combine significance and sign of the 4 cardinal neighbors into a
/// merged byte and look up SIGN_CONTEXT_LOOKUP.
fn encode_sign(
    idx: usize,
    is_negative: bool,
    neighbors: &[u8],
    coeff_signs: &[bool],
    states: &[u8],
    encoder: &mut ArithmeticEncoder,
    contexts: &mut [ArithmeticEncoderContext; 19],
    pw: usize,
) {
    // Get cardinal-neighbor significances: T(6), L(4), R(2), B(0)
    let significances = neighbors[idx] & 0b0101_0101;

    // Get sign of each cardinal neighbor (0=positive, 1=negative).
    // Only meaningful for significant neighbors; insignificant neighbors get 0.
    let top_sign = if states[idx - pw] & SIGNIFICANT != 0 {
        coeff_signs[idx - pw] as u8
    } else {
        0
    };
    let left_sign = if states[idx - 1] & SIGNIFICANT != 0 {
        coeff_signs[idx - 1] as u8
    } else {
        0
    };
    let right_sign = if states[idx + 1] & SIGNIFICANT != 0 {
        coeff_signs[idx + 1] as u8
    } else {
        0
    };
    let bottom_sign = if states[idx + pw] & SIGNIFICANT != 0 {
        coeff_signs[idx + pw] as u8
    } else {
        0
    };

    // Build sign bits at the same positions as significances
    let sign_bits = (top_sign << 6) | (left_sign << 4) | (right_sign << 2) | bottom_sign;

    // Split into negative-significant and positive-significant
    let negative_sigs = significances & sign_bits;
    let positive_sigs = significances & !sign_bits;
    // Merge: negative at (pos+1), positive at (pos) → 2-bit per neighbor
    let merged = (negative_sigs << 1) | positive_sigs;

    let (ctx_label, xor_bit) = SIGN_CONTEXT_LOOKUP[merged as usize];
    let sign_bit = is_negative as u32;
    encoder.encode(sign_bit ^ xor_bit as u32, &mut contexts[ctx_label as usize]);
}

/// Get the zero-coding context label for a coefficient.
#[inline]
fn zero_coding_ctx(neighbor_sig: u8, sub_band_type: SubBandType) -> u8 {
    match sub_band_type {
        SubBandType::LowLow | SubBandType::LowHigh => ZERO_CTX_LL_LH[neighbor_sig as usize],
        SubBandType::HighLow => ZERO_CTX_HL[neighbor_sig as usize],
        SubBandType::HighHigh => ZERO_CTX_HH[neighbor_sig as usize],
    }
}

/// Get the magnitude refinement context label (Table D.4).
///
/// Matches the decoder: if already magnitude-refined → 16,
/// else if at least one neighbor is significant → 15, else 14.
#[inline]
fn magnitude_refinement_ctx(state: u8, neighbor_sig: u8) -> u8 {
    if state & MAGNITUDE_REFINED != 0 {
        16
    } else {
        14 + neighbor_sig.min(1)
    }
}

/// Mark a coefficient as significant and update neighbor significance maps.
fn set_significant(idx: usize, states: &mut [u8], neighbors: &mut [u8], pw: usize) {
    states[idx] |= SIGNIFICANT;

    // Update 8 neighbors
    // Neighbor bit layout: TL(7) T(6) TR(5) L(4) BL(3) R(2) BR(1) B(0)
    let top = idx - pw;
    let bottom = idx + pw;

    neighbors[top - 1] |= 1 << 1; // bottom-right of top-left
    neighbors[top] |= 1; // bottom of top
    neighbors[top + 1] |= 1 << 3; // bottom-left of top-right
    neighbors[idx - 1] |= 1 << 2; // right of left
    neighbors[idx + 1] |= 1 << 4; // left of right
    neighbors[bottom - 1] |= 1 << 5; // top-right of bottom-left
    neighbors[bottom] |= 1 << 6; // top of bottom
    neighbors[bottom + 1] |= 1 << 7; // top-left of bottom-right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_all_zeros() {
        let coeffs = vec![0i32; 16];
        let result = encode_code_block(&coeffs, 4, 4, SubBandType::LowLow, 8);
        assert_eq!(result.num_coding_passes, 0);
        assert!(result.data.is_empty());
        assert_eq!(result.num_zero_bitplanes, 8);
    }

    #[test]
    fn test_encode_single_nonzero() {
        let mut coeffs = vec![0i32; 16];
        coeffs[0] = 128;
        let result = encode_code_block(&coeffs, 4, 4, SubBandType::LowLow, 8);
        assert!(result.num_coding_passes > 0);
        assert!(!result.data.is_empty());
        assert_eq!(result.num_zero_bitplanes, 0);
    }

    #[test]
    fn test_encode_various_magnitudes() {
        let coeffs: Vec<i32> = (0..64)
            .map(|x| if x % 3 == 0 { x * 10 } else { -x })
            .collect();
        let result = encode_code_block(&coeffs, 8, 8, SubBandType::HighHigh, 12);
        assert!(result.num_coding_passes > 0);
        assert!(!result.data.is_empty());
    }

    #[test]
    fn test_zero_bitplanes_count() {
        // Max value is 7 (3 bits), so with Mb=8 we have 8 - 3 = 5 zero bitplanes.
        let coeffs = vec![7i32, -3, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let result = encode_code_block(&coeffs, 4, 4, SubBandType::LowLow, 8);
        assert_eq!(result.num_zero_bitplanes, 5);
    }
}
