//! Arithmetic range encoder primitives for LASzip-style streams.

use std::io::{self, Write};

use crate::laz::arithmetic_decoder::{AC_MAX_LENGTH, AC_MIN_LENGTH};
use crate::laz::arithmetic_model::{
    ArithmeticBitModel,
    ArithmeticSymbolModel,
    BIT_LENGTH_SHIFT,
    SYMBOL_LENGTH_SHIFT,
};

/// Streaming arithmetic encoder.
pub struct ArithmeticEncoder<W: Write> {
    output: W,
    base: u32,
    length: u32,
    emitted: Vec<u8>,
}

impl<W: Write> ArithmeticEncoder<W> {
    /// Create an encoder writing to `output`.
    pub fn new(output: W) -> Self {
        Self {
            output,
            base: 0,
            length: AC_MAX_LENGTH,
            emitted: Vec::new(),
        }
    }

    /// Encode one modeled bit.
    pub fn encode_bit(&mut self, model: &mut ArithmeticBitModel, bit_one: bool) -> io::Result<()> {
        let x = model.zero_probability() * (self.length >> BIT_LENGTH_SHIFT);
        if bit_one {
            self.add_to_base(x);
            self.length -= x;
        } else {
            self.length = x;
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }

        // Symmetric fix with the decoder: pass `bit_one` directly so the model
        // counts zeros and ones in the same direction as the C++ reference.
        model.observe_bit(bit_one);
        Ok(())
    }

    /// Encode one modeled symbol.
    pub fn encode_symbol(&mut self, model: &mut ArithmeticSymbolModel, symbol: u32) -> io::Result<()> {
        let full_length = self.length;
        self.length >>= SYMBOL_LENGTH_SHIFT;

        let lo = model.cdf_at(symbol) * self.length;
        if symbol == model.last_symbol() {
            self.add_to_base(lo);
            self.length = full_length - lo;
        } else {
            let hi = model.cdf_at(symbol + 1) * self.length;
            self.add_to_base(lo);
            self.length = hi - lo;
        }

        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }

        model.observe_symbol(symbol);
        Ok(())
    }

    /// Encode raw `bits` low bits from `value`.
    pub fn write_bits(&mut self, mut bits: u32, mut value: u32) -> io::Result<()> {
        debug_assert!(bits <= 32);
        debug_assert!(bits == 32 || value < (1u32 << bits));

        if bits > 19 {
            self.write_short((value & u32::from(u16::MAX)) as u16)?;
            value >>= 16;
            bits -= 16;
        }

        self.length >>= bits;
        self.add_to_base(value * self.length);

        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }
        Ok(())
    }

    fn write_short(&mut self, value: u16) -> io::Result<()> {
        self.length >>= 16;
        self.add_to_base(u32::from(value) * self.length);
        if self.length < AC_MIN_LENGTH {
            self.renorm()?;
        }
        Ok(())
    }

    /// Finalize encoding and flush bytes.
    pub fn done(mut self) -> io::Result<W> {
        let mut write_extra = true;

        if self.length > 2 * AC_MIN_LENGTH {
            self.add_to_base(AC_MIN_LENGTH);
            self.length = AC_MIN_LENGTH >> 1;
        } else {
            self.add_to_base(AC_MIN_LENGTH >> 1);
            self.length = AC_MIN_LENGTH >> 9;
            write_extra = false;
        }

        self.renorm()?;

        self.output.write_all(&self.emitted)?;
        self.output.write_all(&[0u8, 0u8])?;
        if write_extra {
            self.output.write_all(&[0u8])?;
        }
        Ok(self.output)
    }

    fn add_to_base(&mut self, delta: u32) {
        let prev = self.base;
        self.base = self.base.wrapping_add(delta);
        if prev > self.base {
            self.propagate_carry();
        }
    }

    fn renorm(&mut self) -> io::Result<()> {
        while self.length < AC_MIN_LENGTH {
            let out_byte = (self.base >> 24) as u8;
            self.emitted.push(out_byte);
            self.base <<= 8;
            self.length <<= 8;
        }
        Ok(())
    }

    fn propagate_carry(&mut self) {
        for b in self.emitted.iter_mut().rev() {
            let (v, overflow) = b.overflowing_add(1);
            *b = v;
            if !overflow {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::laz::arithmetic_decoder::ArithmeticDecoder;
    use crate::laz::arithmetic_model::{ArithmeticBitModel, ArithmeticSymbolModel};

    use super::ArithmeticEncoder;

    #[test]
    fn arithmetic_symbol_round_trip() -> std::io::Result<()> {
        let source: Vec<u32> = (0..500).map(|i| ((i * 13 + 7) % 23) as u32).collect();

        let mut writer = Cursor::new(Vec::<u8>::new());
        {
            let mut encoder = ArithmeticEncoder::new(&mut writer);
            let mut model = ArithmeticSymbolModel::new(23);
            for sym in &source {
                encoder.encode_symbol(&mut model, *sym)?;
            }
            let _ = encoder.done()?;
        }

        writer.set_position(0);
        let mut decoder = ArithmeticDecoder::new(writer);
        decoder.read_init_bytes()?;
        let mut model = ArithmeticSymbolModel::new(23);

        for expected in source {
            let got = decoder.decode_symbol(&mut model)?;
            assert_eq!(got, expected);
        }

        Ok(())
    }

    #[test]
    fn arithmetic_bit_round_trip() -> std::io::Result<()> {
        let bits: Vec<bool> = (0..512).map(|i| (i % 5) != 0).collect();

        let mut writer = Cursor::new(Vec::<u8>::new());
        {
            let mut encoder = ArithmeticEncoder::new(&mut writer);
            let mut model = ArithmeticBitModel::new();
            for b in &bits {
                encoder.encode_bit(&mut model, *b)?;
            }
            let _ = encoder.done()?;
        }

        writer.set_position(0);
        let mut decoder = ArithmeticDecoder::new(writer);
        decoder.read_init_bytes()?;
        let mut model = ArithmeticBitModel::new();

        for expected in bits {
            let got = decoder.decode_bit(&mut model)?;
            assert_eq!(got, expected);
        }

        Ok(())
    }
}
