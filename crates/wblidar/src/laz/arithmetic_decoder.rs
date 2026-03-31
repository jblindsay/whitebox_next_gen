//! Arithmetic range decoder primitives for LASzip-style streams.

use std::io::{self, Read};

use crate::laz::arithmetic_model::{
    ArithmeticBitModel,
    ArithmeticSymbolModel,
    BIT_LENGTH_SHIFT,
    SYMBOL_LENGTH_SHIFT,
};

/// Upper bound for decoder interval length.
pub const AC_MAX_LENGTH: u32 = 0xFFFF_FFFF;
/// Renormalization threshold for decoder interval length.
pub const AC_MIN_LENGTH: u32 = 0x0100_0000;

/// Streaming arithmetic decoder.
pub struct ArithmeticDecoder<R: Read> {
    input: R,
    value: u32,
    length: u32,
}

impl<R: Read> ArithmeticDecoder<R> {
    /// Create a decoder; call `read_init_bytes` before decoding symbols.
    pub fn new(input: R) -> Self {
        Self {
            input,
            value: 0,
            length: AC_MAX_LENGTH,
        }
    }

    /// Initialize decoder state from the stream preamble.
    pub fn read_init_bytes(&mut self) -> io::Result<()> {
        let mut init = [0u8; 4];
        self.input.read_exact(&mut init)?;
        self.value = u32::from(init[0]) << 24
            | u32::from(init[1]) << 16
            | u32::from(init[2]) << 8
            | u32::from(init[3]);
        Ok(())
    }

    /// Decode one modeled bit.
    pub fn decode_bit(&mut self, model: &mut ArithmeticBitModel) -> io::Result<bool> {
        let x = model.zero_probability() * (self.length >> BIT_LENGTH_SHIFT);
        let one = self.value >= x;

        if one {
            self.value -= x;
            self.length -= x;
        } else {
            self.length = x;
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }

        // Pass `one` directly: observe_bit(false) counts a zero, observe_bit(true) counts a one.
        // The earlier `!one` was inverted relative to the C++ reference (which increments
        // bit_0_count when the decoded bit is 0).
        model.observe_bit(one);
        Ok(one)
    }

    /// Decode one modeled symbol.
    pub fn decode_symbol(&mut self, model: &mut ArithmeticSymbolModel) -> io::Result<u32> {
        let mut y = self.length;
        self.length >>= SYMBOL_LENGTH_SHIFT;
        if self.length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "arithmetic decode interval collapsed",
            ));
        }

        // Match LASzip's no-table decoder path exactly: binary search by comparing
        // value against z = length * cdf[k], updating [x, y) bounds as integers.
        let mut x = 0u32;
        let mut symbol = 0u32;
        let mut n = model.symbols();
        let mut k = n >> 1;
        loop {
            let z = self.length * model.cdf_at(k);
            if z > self.value {
                n = k;
                y = z;
            } else {
                symbol = k;
                x = z;
            }
            k = (symbol + n) >> 1;
            if k == symbol {
                break;
            }
        }

        self.value -= x;
        self.length = y - x;

        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }

        model.observe_symbol(symbol);
        Ok(symbol)
    }

    /// Decode an unmodeled bit.
    pub fn read_bit(&mut self) -> io::Result<u32> {
        self.length >>= 1;
        let symbol = self.value / self.length;
        self.value -= self.length * symbol;
        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }
        Ok(symbol)
    }

    /// Decode `bits` raw bits as an unsigned integer.
    pub fn read_bits(&mut self, mut bits: u32) -> io::Result<u32> {
        debug_assert!(bits > 0 && bits <= 32);
        if bits > 19 {
            let low = u32::from(self.read_short()?);
            bits -= 16;
            let high = self.read_bits(bits)? << 16;
            Ok(high | low)
        } else {
            self.length >>= bits;
            let symbol = self.value / self.length;
            self.value -= self.length * symbol;
            if self.length < AC_MIN_LENGTH {
                self.renorm()?;
            }
            Ok(symbol)
        }
    }

    /// Decode a raw 32-bit unsigned integer.
    pub fn read_int(&mut self) -> io::Result<u32> {
        self.read_bits(32)
    }

    /// Decode a raw 64-bit unsigned integer.
    pub fn read_int64(&mut self) -> io::Result<u64> {
        let lo = u64::from(self.read_int()?);
        let hi = u64::from(self.read_int()?);
        Ok((hi << 32) | lo)
    }

    fn read_short(&mut self) -> io::Result<u16> {
        self.length >>= 16;
        let symbol = self.value / self.length;
        self.value -= self.length * symbol;
        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }
        Ok(symbol as u16)
    }

    fn renorm(&mut self) -> io::Result<()> {
        while self.length < AC_MIN_LENGTH {
            let mut b = [0u8; 1];
            self.input.read_exact(&mut b)?;
            self.value = (self.value << 8) | u32::from(b[0]);
            self.length <<= 8;
        }
        Ok(())
    }

    /// Borrow the underlying stream.
    pub fn get_ref(&self) -> &R {
        &self.input
    }

    /// Mutably borrow the underlying stream.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.input
    }

    /// Consume decoder and return inner stream.
    pub fn into_inner(self) -> R {
        self.input
    }
}
