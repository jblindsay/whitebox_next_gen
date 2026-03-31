//! Integer corrector codec used by LASzip point field compressors.

use std::io::{Read, Write};

use crate::laz::arithmetic_decoder::ArithmeticDecoder;
use crate::laz::arithmetic_encoder::ArithmeticEncoder;
use crate::laz::arithmetic_model::{ArithmeticBitModel, ArithmeticSymbolModel};

/// Integer predictor/corrector decoder.
#[derive(Clone)]
pub struct IntegerDecompressor {
    k: u32,
    contexts: u32,
    bits_high: u32,
    corr_bits: u32,
    corr_range: u32,
    corr_min: i32,
    models_k: Vec<ArithmeticSymbolModel>,
    model_corrector_0: ArithmeticBitModel,
    models_corrector: Vec<ArithmeticSymbolModel>,
}

impl IntegerDecompressor {
    /// Create a new integer decompressor.
    pub fn new(bits: u32, contexts: u32, bits_high: u32, mut range: u32) -> Self {
        let (corr_bits, corr_range, corr_min) = if range != 0 {
            let mut cb = 0;
            let corr_range = range;
            while range != 0 {
                range >>= 1;
                cb += 1;
            }
            if corr_range == (1u32 << (cb - 1)) {
                cb -= 1;
            }
            (cb, corr_range, -((corr_range as i32) / 2))
        } else if bits > 0 && bits < 32 {
            let corr_range = 1u32 << bits;
            (bits, corr_range, -((corr_range as i32) / 2))
        } else {
            (32, 0, i32::MIN)
        };

        let mut out = Self {
            k: 0,
            contexts,
            bits_high,
            corr_bits,
            corr_range,
            corr_min,
            models_k: Vec::new(),
            model_corrector_0: ArithmeticBitModel::new(),
            models_corrector: Vec::new(),
        };
        out.init();
        out
    }

    /// Last decoded `k` interval index.
    pub fn k(&self) -> u32 {
        self.k
    }

    fn init(&mut self) {
        if self.models_k.is_empty() {
            for _ in 0..self.contexts {
                self.models_k.push(ArithmeticSymbolModel::new(self.corr_bits + 1));
            }
            for i in 1..=self.corr_bits {
                let symbols = if i <= self.bits_high {
                    1 << i
                } else {
                    1 << self.bits_high
                };
                self.models_corrector.push(ArithmeticSymbolModel::new(symbols));
            }
        }
    }

    /// Decode an integer from predictor `pred` and arithmetic stream.
    pub fn decompress<T: Read>(
        &mut self,
        dec: &mut ArithmeticDecoder<T>,
        pred: i32,
        context: u32,
    ) -> std::io::Result<i32> {
        self.k = dec.decode_symbol(&mut self.models_k[context as usize])?;

        let corr = if self.k != 0 {
            if self.k < 32 {
                let mut c = if self.k <= self.bits_high {
                    dec.decode_symbol(&mut self.models_corrector[(self.k - 1) as usize])? as i32
                } else {
                    let k1 = self.k - self.bits_high;
                    let hi = dec.decode_symbol(&mut self.models_corrector[(self.k - 1) as usize])?
                        as i32;
                    let lo = dec.read_bits(k1)? as i32;
                    (hi << k1) | lo
                };

                if c >= (1u32 << (self.k - 1)) as i32 {
                    c += 1;
                } else {
                    c -= ((1u32 << self.k) - 1) as i32;
                }
                c
            } else {
                self.corr_min
            }
        } else if dec.decode_bit(&mut self.model_corrector_0)? {
            1
        } else {
            0
        };

        let mut real = pred.wrapping_add(corr);
        if self.corr_range != 0 {
            if real < 0 {
                real += self.corr_range as i32;
            } else if real >= self.corr_range as i32 {
                real -= self.corr_range as i32;
            }
        }
        Ok(real)
    }
}

/// Integer predictor/corrector encoder.
pub struct IntegerCompressor {
    k: u32,
    contexts: u32,
    bits_high: u32,
    corr_bits: u32,
    corr_range: u32,
    corr_min: i32,
    corr_max: i32,
    models_k: Vec<ArithmeticSymbolModel>,
    model_corrector_0: ArithmeticBitModel,
    models_corrector: Vec<ArithmeticSymbolModel>,
}

impl IntegerCompressor {
    /// Create a new integer compressor.
    pub fn new(bits: u32, contexts: u32, bits_high: u32, mut range: u32) -> Self {
        let (corr_bits, corr_range, corr_min, corr_max) = if range != 0 {
            let mut cb = 0;
            let corr_range = range;
            while range != 0 {
                range >>= 1;
                cb += 1;
            }
            if corr_range == (1u32 << (cb - 1)) {
                cb -= 1;
            }
            let corr_min = -((corr_range as i32) / 2);
            (cb, corr_range, corr_min, corr_min + (corr_range - 1) as i32)
        } else if bits > 0 && bits < 32 {
            let corr_range = 1u32 << bits;
            let corr_min = -((corr_range as i32) / 2);
            (bits, corr_range, corr_min, corr_min + (corr_range - 1) as i32)
        } else {
            (32, 0, i32::MIN, i32::MAX)
        };

        let mut out = Self {
            k: 0,
            contexts,
            bits_high,
            corr_bits,
            corr_range,
            corr_min,
            corr_max,
            models_k: Vec::new(),
            model_corrector_0: ArithmeticBitModel::new(),
            models_corrector: Vec::new(),
        };
        out.init();
        out
    }

    /// Last encoded `k` interval index.
    pub fn k(&self) -> u32 {
        self.k
    }

    fn init(&mut self) {
        if self.models_k.is_empty() {
            for _ in 0..self.contexts {
                self.models_k.push(ArithmeticSymbolModel::new(self.corr_bits + 1));
            }
            for i in 1..=self.corr_bits {
                let symbols = if i <= self.bits_high {
                    1 << i
                } else {
                    1 << self.bits_high
                };
                self.models_corrector.push(ArithmeticSymbolModel::new(symbols));
            }
        }
    }

    /// Encode integer `real` around predictor `pred`.
    pub fn compress<T: Write>(
        &mut self,
        enc: &mut ArithmeticEncoder<T>,
        pred: i32,
        real: i32,
        context: u32,
    ) -> std::io::Result<()> {
        let mut corr = real.wrapping_sub(pred);
        if corr < self.corr_min {
            corr += self.corr_range as i32;
        } else if corr > self.corr_max {
            corr -= self.corr_range as i32;
        }

        let mut c1 = if corr <= 0 { corr.wrapping_neg() } else { corr - 1 } as u32;
        self.k = 0;
        while c1 != 0 {
            c1 >>= 1;
            self.k += 1;
        }

        enc.encode_symbol(&mut self.models_k[context as usize], self.k)?;

        if self.k != 0 {
            if self.k < 32 {
                let mut c = corr;
                if c >= 0 {
                    c -= 1;
                } else {
                    c += ((1u32 << self.k) - 1) as i32;
                }

                if self.k <= self.bits_high {
                    enc.encode_symbol(&mut self.models_corrector[(self.k - 1) as usize], c as u32)?;
                } else {
                    let k1 = self.k - self.bits_high;
                    let lo = (c & ((1u32 << k1) - 1) as i32) as u32;
                    c >>= k1 as i32;
                    enc.encode_symbol(&mut self.models_corrector[(self.k - 1) as usize], c as u32)?;
                    enc.write_bits(k1, lo)?;
                }
            }
        } else {
            enc.encode_bit(&mut self.model_corrector_0, corr != 0)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::laz::arithmetic_decoder::ArithmeticDecoder;
    use crate::laz::arithmetic_encoder::ArithmeticEncoder;

    use super::{IntegerCompressor, IntegerDecompressor};

    #[test]
    fn integer_codec_round_trip() -> std::io::Result<()> {
        let values: Vec<i32> = (0..2048).map(|i| ((i * 97) % 65536) as i32).collect();

        let mut writer = Cursor::new(Vec::<u8>::new());
        {
            let mut encoder = ArithmeticEncoder::new(&mut writer);
            let mut codec = IntegerCompressor::new(16, 4, 8, 0);
            let mut pred = 0i32;
            for (i, v) in values.iter().enumerate() {
                codec.compress(&mut encoder, pred, *v, (i as u32) & 0x03)?;
                pred = *v;
            }
            let _ = encoder.done()?;
        }

        writer.set_position(0);
        let mut decoder = ArithmeticDecoder::new(writer);
        decoder.read_init_bytes()?;

        let mut codec = IntegerDecompressor::new(16, 4, 8, 0);
        let mut pred = 0i32;
        for (i, expected) in values.iter().enumerate() {
            let got = codec.decompress(&mut decoder, pred, (i as u32) & 0x03)?;
            assert_eq!(got, *expected);
            pred = got;
        }
        Ok(())
    }
}
