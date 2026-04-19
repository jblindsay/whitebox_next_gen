//! MQ arithmetic encoder for JPEG 2000 (ITU-T T.800 Annex C).
//!
//! This is the encoding counterpart of `arithmetic_decoder.rs`.
//! It uses the same QE probability table and context state machine.

use alloc::vec::Vec;

/// MQ arithmetic encoder context (identical layout to decoder context).
///
/// Bits 0-6: state index (0-46) into the QE table.
/// Bit 7: MPS (Most Probable Symbol, 0 or 1).
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct ArithmeticEncoderContext(u8);

impl ArithmeticEncoderContext {
    #[inline(always)]
    pub(crate) fn index(self) -> u32 {
        (self.0 & 0x7F) as u32
    }

    #[inline(always)]
    pub(crate) fn mps(self) -> u32 {
        (self.0 >> 7) as u32
    }

    #[inline(always)]
    fn set_index(&mut self, index: u8) {
        self.0 = (self.0 & 0x80) | index;
    }

    #[inline(always)]
    fn xor_mps(&mut self, val: u32) {
        self.0 ^= ((val & 1) << 7) as u8;
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) fn reset(&mut self) {
        self.0 = 0;
    }

    #[inline(always)]
    pub(crate) fn reset_with_index(&mut self, index: u8) {
        self.0 = index;
    }
}

/// QE table entry for the MQ coder (Table C.2).
#[derive(Debug, Clone, Copy)]
struct QeData {
    qe: u32,
    nmps: u8,
    nlps: u8,
    switch: bool,
}

macro_rules! qe {
    ($($qe:expr, $nmps:expr, $nlps:expr, $switch:expr),+ $(,)?) => {
        [$(QeData { qe: $qe, nmps: $nmps, nlps: $nlps, switch: $switch }),+]
    }
}

#[rustfmt::skip]
static QE_TABLE: [QeData; 47] = qe!(
    0x5601, 1, 1, true,
    0x3401, 2, 6, false,
    0x1801, 3, 9, false,
    0x0AC1, 4, 12, false,
    0x0521, 5, 29, false,
    0x0221, 38, 33, false,
    0x5601, 7, 6, true,
    0x5401, 8, 14, false,
    0x4801, 9, 14, false,
    0x3801, 10, 14, false,
    0x3001, 11, 17, false,
    0x2401, 12, 18, false,
    0x1C01, 13, 20, false,
    0x1601, 29, 21, false,
    0x5601, 15, 14, true,
    0x5401, 16, 14, false,
    0x5101, 17, 15, false,
    0x4801, 18, 16, false,
    0x3801, 19, 17, false,
    0x3401, 20, 18, false,
    0x3001, 21, 19, false,
    0x2801, 22, 19, false,
    0x2401, 23, 20, false,
    0x2201, 24, 21, false,
    0x1C01, 25, 22, false,
    0x1801, 26, 23, false,
    0x1601, 27, 24, false,
    0x1401, 28, 25, false,
    0x1201, 29, 26, false,
    0x1101, 30, 27, false,
    0x0AC1, 31, 28, false,
    0x09C1, 32, 29, false,
    0x08A1, 33, 30, false,
    0x0521, 34, 31, false,
    0x0441, 35, 32, false,
    0x02A1, 36, 33, false,
    0x0221, 37, 34, false,
    0x0141, 38, 35, false,
    0x0111, 39, 36, false,
    0x0085, 40, 37, false,
    0x0049, 41, 38, false,
    0x0025, 42, 39, false,
    0x0015, 43, 40, false,
    0x0009, 44, 41, false,
    0x0005, 45, 42, false,
    0x0001, 45, 43, false,
    0x5601, 46, 46, false,
);

/// MQ arithmetic encoder (ITU-T T.800 Annex C).
///
/// Uses the direct buffer approach with proper 7-bit stuffing after 0xFF
/// to match the decoder's Annex G byte input convention.
pub(crate) struct ArithmeticEncoder {
    /// Output byte stream. Index 0 is a sentinel byte (0x00).
    data: Vec<u8>,
    /// A-register (interval size), 16-bit precision.
    a: u32,
    /// C-register (code value), 28-bit + carry at bit 27.
    c: u32,
    /// Bit shift counter (12 initially, then 7 after 0xFF, 8 otherwise).
    ct: u32,
}

impl ArithmeticEncoder {
    pub(crate) fn new() -> Self {
        Self {
            data: vec![0x00], // Sentinel byte at index 0
            a: 0x8000,
            c: 0,
            ct: 12,
        }
    }

    /// Encode a single symbol (0 or 1) with the given context (C.2.6 ENCODE).
    #[inline]
    pub(crate) fn encode(&mut self, bit: u32, context: &mut ArithmeticEncoderContext) {
        let qe_entry = &QE_TABLE[context.index() as usize];
        self.a -= qe_entry.qe;

        if bit == context.mps() {
            // MPS path (CODEMPS, C.2.6)
            if self.a & 0x8000 != 0 {
                // No renormalization needed: fast path
                self.c += qe_entry.qe;
                return;
            }
            if self.a < qe_entry.qe {
                // Conditional exchange: MPS coded in lower sub-interval
                // C stays (don't add Qe), A takes the larger Qe value
                self.a = qe_entry.qe;
            } else {
                // Normal: MPS coded in upper sub-interval
                self.c += qe_entry.qe;
            }
            context.set_index(qe_entry.nmps);
        } else {
            // LPS path (CODELPS, C.2.6)
            if self.a < qe_entry.qe {
                // Conditional exchange: LPS coded in upper sub-interval
                self.c += qe_entry.qe;
            } else {
                // Normal: LPS coded in lower sub-interval, A = Qe
                self.a = qe_entry.qe;
            }
            if qe_entry.switch {
                context.xor_mps(1);
            }
            context.set_index(qe_entry.nlps);
        }

        self.renormalize();
    }

    /// Renormalize the encoder (C.2.7 RENORME).
    fn renormalize(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
            if self.a & 0x8000 != 0 {
                break;
            }
        }
    }

    /// Output a byte with carry propagation and bit stuffing (C.2.8 BYTEOUT).
    ///
    /// After 0xFF, only 7 bits are extracted (bit stuffing) to prevent
    /// marker-like byte sequences in the output.
    fn byte_out(&mut self) {
        let last_byte = *self.data.last().unwrap();
        if last_byte == 0xFF {
            // 7-bit mode after 0xFF (bit stuffing)
            let b = (self.c >> 20) as u8;
            self.data.push(b);
            self.c &= 0xFFFFF;
            self.ct = 7;
        } else if self.c & 0x8000000 == 0 {
            // No carry: normal 8-bit output
            let b = (self.c >> 19) as u8;
            self.data.push(b);
            self.c &= 0x7FFFF;
            self.ct = 8;
        } else {
            // Carry occurred (bit 27 set): propagate into last byte
            let last = self.data.last_mut().unwrap();
            *last += 1;
            self.c &= 0x7FFFFFF; // Clear carry bit
            if *last == 0xFF {
                // Carry made last byte 0xFF: switch to 7-bit mode
                let b = (self.c >> 20) as u8;
                self.data.push(b);
                self.c &= 0xFFFFF;
                self.ct = 7;
            } else {
                let b = (self.c >> 19) as u8;
                self.data.push(b);
                self.c &= 0x7FFFF;
                self.ct = 8;
            }
        }
    }

    /// SETBITS procedure (C.2.9).
    fn set_bits(&mut self) {
        let temp = self.c + self.a;
        self.c |= 0xFFFF;
        if self.c >= temp {
            self.c -= 0x8000;
        }
    }

    /// Flush the encoder state (C.2.9 FLUSH).
    pub(crate) fn flush(&mut self) {
        self.set_bits();
        self.c <<= self.ct;
        self.byte_out();
        self.c <<= self.ct;
        self.byte_out();
        // Include one extra byte for proper termination
        if *self.data.last().unwrap() != 0xFF {
            self.data.push(0);
        }
    }

    /// Return the encoded data (excluding sentinel), consuming the encoder.
    pub(crate) fn finish(mut self) -> Vec<u8> {
        self.flush();
        // Remove sentinel byte at index 0
        self.data.drain(..1);
        self.data
    }

    /// Return the current length of encoded data so far (excluding sentinel).
    #[allow(dead_code)]
    pub(crate) fn encoded_len(&self) -> usize {
        self.data.len().saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::j2c::arithmetic_decoder::{ArithmeticDecoder, ArithmeticDecoderContext};

    #[test]
    fn test_encode_decode_round_trip() {
        let symbols: Vec<u32> = vec![0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1];
        let mut encoder = ArithmeticEncoder::new();
        let mut enc_ctx = ArithmeticEncoderContext::default();

        for &s in &symbols {
            encoder.encode(s, &mut enc_ctx);
        }
        let encoded = encoder.finish();

        // Decode and verify (new() already calls initialize())
        let mut decoder = ArithmeticDecoder::new(&encoded);
        let mut dec_ctx = ArithmeticDecoderContext::default();

        let mut decoded = Vec::new();
        for _ in 0..symbols.len() {
            decoded.push(decoder.decode(&mut dec_ctx));
        }

        assert_eq!(symbols, decoded);
    }

    #[test]
    fn test_encode_all_mps() {
        let mut encoder = ArithmeticEncoder::new();
        let mut ctx = ArithmeticEncoderContext::default();
        for _ in 0..100 {
            encoder.encode(0, &mut ctx);
        }
        let encoded = encoder.finish();

        let mut decoder = ArithmeticDecoder::new(&encoded);
        let mut dec_ctx = ArithmeticDecoderContext::default();
        for _ in 0..100 {
            assert_eq!(decoder.decode(&mut dec_ctx), 0);
        }
    }

    #[test]
    fn test_encode_all_lps() {
        let mut encoder = ArithmeticEncoder::new();
        let mut ctx = ArithmeticEncoderContext::default();
        for _ in 0..50 {
            encoder.encode(1, &mut ctx);
        }
        let encoded = encoder.finish();

        let mut decoder = ArithmeticDecoder::new(&encoded);
        let mut dec_ctx = ArithmeticDecoderContext::default();
        for _ in 0..50 {
            assert_eq!(decoder.decode(&mut dec_ctx), 1);
        }
    }

    #[test]
    fn test_multiple_contexts() {
        let symbols_a = [0u32, 1, 0, 0, 1, 1, 0, 1];
        let symbols_b = [1u32, 1, 0, 1, 0, 0, 1, 0];

        let mut encoder = ArithmeticEncoder::new();
        let mut ctx_a = ArithmeticEncoderContext::default();
        let mut ctx_b = ArithmeticEncoderContext::default();

        for i in 0..8 {
            encoder.encode(symbols_a[i], &mut ctx_a);
            encoder.encode(symbols_b[i], &mut ctx_b);
        }
        let encoded = encoder.finish();

        let mut decoder = ArithmeticDecoder::new(&encoded);
        let mut dec_ctx_a = ArithmeticDecoderContext::default();
        let mut dec_ctx_b = ArithmeticDecoderContext::default();

        for i in 0..8 {
            assert_eq!(decoder.decode(&mut dec_ctx_a), symbols_a[i]);
            assert_eq!(decoder.decode(&mut dec_ctx_b), symbols_b[i]);
        }
    }

    #[test]
    fn test_context_state_identical() {
        let mut enc_ctx = ArithmeticEncoderContext::default();
        let mut dec_ctx = ArithmeticDecoderContext::default();

        let bits = [0u32, 0, 1, 0, 1, 1, 0, 0];
        let mut encoder = ArithmeticEncoder::new();
        for &b in &bits {
            encoder.encode(b, &mut enc_ctx);
        }
        let encoded = encoder.finish();

        let mut decoder = ArithmeticDecoder::new(&encoded);
        for &b in &bits {
            let decoded = decoder.decode(&mut dec_ctx);
            assert_eq!(decoded, b);
        }

        // Both contexts should be in same state
        assert_eq!(enc_ctx.index(), dec_ctx.index());
        assert_eq!(enc_ctx.mps(), dec_ctx.mps());
    }
}
