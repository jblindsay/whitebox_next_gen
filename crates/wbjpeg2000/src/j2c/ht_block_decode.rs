//! Scalar HTJ2K block decoding.

use alloc::vec::Vec;

use super::build::CodeBlock;
use super::decode::DecompositionStorage;
use super::ht_tables::{UVLC_TABLE0, UVLC_TABLE1, VLC_TABLE0, VLC_TABLE1};
use crate::error::{bail, DecodingError, Result};

#[derive(Default)]
pub(crate) struct HtBlockDecodeContext {
    coefficients: Vec<u32>,
    width: u32,
    height: u32,
}

impl HtBlockDecodeContext {
    fn reset(&mut self, code_block: &CodeBlock) {
        self.width = code_block.rect.width();
        self.height = code_block.rect.height();
        self.coefficients.clear();
        self.coefficients
            .resize((self.width * self.height) as usize, 0);
    }

    pub(crate) fn coefficient_rows(&self) -> impl Iterator<Item = &[u32]> {
        self.coefficients.chunks_exact(self.width as usize)
    }
}

pub(crate) fn coefficient_to_i32(value: u32, k_max: u8) -> i32 {
    let shift = 31_u32.saturating_sub(k_max as u32);
    let magnitude = ((value & 0x7FFF_FFFF) >> shift) as i32;

    if (value & 0x8000_0000) != 0 {
        -magnitude
    } else {
        magnitude
    }
}

pub(crate) fn decode(
    code_block: &CodeBlock,
    total_bitplanes: u8,
    stripe_causal: bool,
    ctx: &mut HtBlockDecodeContext,
    storage: &DecompositionStorage<'_>,
    strict: bool,
) -> Result<()> {
    ctx.reset(code_block);

    if total_bitplanes == 0 {
        return Ok(());
    }

    if total_bitplanes > 31 {
        bail!(DecodingError::TooManyBitplanes);
    }

    let actual_bitplanes = if strict {
        total_bitplanes
            .checked_sub(code_block.missing_bit_planes)
            .ok_or(DecodingError::InvalidBitplaneCount)?
    } else {
        total_bitplanes.saturating_sub(code_block.missing_bit_planes)
    };

    let max_coding_passes = if actual_bitplanes == 0 {
        0
    } else {
        1 + 3 * (actual_bitplanes - 1)
    };

    if code_block.number_of_coding_passes > max_coding_passes && strict {
        bail!(DecodingError::TooManyCodingPasses);
    }

    if code_block.number_of_coding_passes == 0 || actual_bitplanes == 0 {
        return Ok(());
    }

    let combined = collect_code_block_data(code_block, storage)?;

    decode_impl(
        &combined.data,
        &mut ctx.coefficients,
        code_block.missing_bit_planes as u32,
        code_block.number_of_coding_passes as u32,
        combined.cleanup_length,
        combined.refinement_length,
        code_block.rect.width(),
        code_block.rect.height(),
        code_block.rect.width(),
        stripe_causal,
    )
    .ok_or(DecodingError::CodeBlockDecodeFailure.into())
}

struct CombinedCodeBlockData {
    data: Vec<u8>,
    cleanup_length: u32,
    refinement_length: u32,
}

fn collect_code_block_data(
    code_block: &CodeBlock,
    storage: &DecompositionStorage<'_>,
) -> Result<CombinedCodeBlockData> {
    let mut data = Vec::new();
    let mut cleanup_length = 0;
    let mut refinement_length = 0;
    let mut saw_cleanup = false;
    let mut saw_refinement = false;

    for layer in &storage.layers[code_block.layers.start..code_block.layers.end] {
        let Some(range) = layer.segments.clone() else {
            continue;
        };

        for segment in &storage.segments[range] {
            match segment.idx {
                0 if !saw_cleanup => {
                    cleanup_length = segment.data_length;
                    data.extend_from_slice(segment.data);
                    saw_cleanup = true;
                }
                1 if !saw_refinement => {
                    refinement_length = segment.data_length;
                    data.extend_from_slice(segment.data);
                    saw_refinement = true;
                }
                _ => bail!(DecodingError::UnsupportedFeature(
                    "unexpected HTJ2K segment layout"
                )),
            }
        }
    }

    if !saw_cleanup {
        bail!(DecodingError::CodeBlockDecodeFailure);
    }

    Ok(CombinedCodeBlockData {
        data,
        cleanup_length,
        refinement_length,
    })
}

struct MelDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    remaining: usize,
    unstuff: bool,
    current_byte: u8,
    bits_left: u8,
    k: usize,
    num_runs: usize,
    runs: u64,
}

impl<'a> MelDecoder<'a> {
    fn new(data: &'a [u8], lcup: usize, scup: usize) -> Self {
        Self {
            data,
            pos: lcup - scup,
            remaining: scup - 1,
            unstuff: false,
            current_byte: 0,
            bits_left: 0,
            k: 0,
            num_runs: 0,
            runs: 0,
        }
    }

    fn read_bit(&mut self) -> Option<u32> {
        if self.bits_left == 0 {
            let mut byte = if self.remaining > 0 {
                let byte = self.data.get(self.pos).copied()?;
                self.pos += 1;
                self.remaining -= 1;
                byte
            } else {
                0xFF
            };

            if self.remaining == 0 {
                byte |= 0x0F;
            }

            self.current_byte = byte;
            self.bits_left = 8 - u8::from(self.unstuff);
            self.unstuff = byte == 0xFF;
        }

        self.bits_left -= 1;
        Some(((self.current_byte >> self.bits_left) & 1) as u32)
    }

    fn read_bits(&mut self, count: usize) -> Option<u32> {
        let mut value = 0;

        for _ in 0..count {
            value = (value << 1) | self.read_bit()?;
        }

        Some(value)
    }

    fn decode_more_runs(&mut self) -> Option<()> {
        const MEL_EXP: [usize; 13] = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];

        while self.num_runs < 8 {
            let eval = MEL_EXP[self.k];
            let first = self.read_bit()?;
            let run = if first == 1 {
                self.k = (self.k + 1).min(12);
                ((1usize << eval) - 1) << 1
            } else {
                self.k = self.k.saturating_sub(1);
                (self.read_bits(eval)? as usize) << 1 | 1
            };

            self.runs |= (run as u64) << (self.num_runs * 7);
            self.num_runs += 1;

            if eval == 5 && first == 0 && self.num_runs >= 8 {
                break;
            }
        }

        Some(())
    }

    fn get_run(&mut self) -> Option<i32> {
        if self.num_runs == 0 {
            self.decode_more_runs()?;
        }

        let run = (self.runs & 0x7F) as i32;
        self.runs >>= 7;
        self.num_runs -= 1;
        Some(run)
    }
}

struct ForwardBitReader<'a, const PAD: u8> {
    data: &'a [u8],
    pos: usize,
    tmp: u64,
    bits: u32,
    unstuff: bool,
}

impl<'a, const PAD: u8> ForwardBitReader<'a, PAD> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            tmp: 0,
            bits: 0,
            unstuff: false,
        }
    }

    fn fill(&mut self) {
        while self.bits <= 32 {
            let byte = if self.pos < self.data.len() {
                let byte = self.data[self.pos];
                self.pos += 1;
                byte
            } else {
                PAD
            };

            self.tmp |= (byte as u64) << self.bits;
            self.bits += 8 - u32::from(self.unstuff);
            self.unstuff = byte == 0xFF;
        }
    }

    fn fetch(&mut self) -> u32 {
        if self.bits < 32 {
            self.fill();
        }

        self.tmp as u32
    }

    fn advance(&mut self, count: u32) {
        debug_assert!(count <= self.bits);
        self.tmp >>= count;
        self.bits -= count;
    }
}

struct ReverseBitReader<'a> {
    data: &'a [u8],
    pos: isize,
    remaining: usize,
    tmp: u64,
    bits: u32,
    unstuff: bool,
}

impl<'a> ReverseBitReader<'a> {
    fn new_vlc(data: &'a [u8], lcup: usize, scup: usize) -> Self {
        let d = data[lcup - 2];
        let tmp = u64::from(d >> 4);
        let bits = 4 - u32::from((tmp & 0x7) == 0x7);

        Self {
            data,
            pos: lcup as isize - 3,
            remaining: scup - 2,
            tmp,
            bits,
            unstuff: (d | 0x0F) > 0x8F,
        }
    }

    fn new_mrp(data: &'a [u8], lcup: usize, len2: usize) -> Self {
        Self {
            data,
            pos: (lcup + len2) as isize - 1,
            remaining: len2,
            tmp: 0,
            bits: 0,
            unstuff: true,
        }
    }

    fn fill(&mut self) {
        while self.bits <= 32 {
            let byte = if self.remaining > 0 {
                let byte = self.data[self.pos as usize];
                self.pos -= 1;
                self.remaining -= 1;
                byte
            } else {
                0
            };

            let d_bits = 8 - u32::from(self.unstuff && (byte & 0x7F) == 0x7F);
            self.tmp |= (byte as u64) << self.bits;
            self.bits += d_bits;
            self.unstuff = byte > 0x8F;
        }
    }

    fn fetch(&mut self) -> u32 {
        if self.bits < 32 {
            self.fill();
        }

        self.tmp as u32
    }

    fn advance(&mut self, count: u32) -> u32 {
        debug_assert!(count <= self.bits);
        self.tmp >>= count;
        self.bits -= count;
        self.tmp as u32
    }
}

fn read_u32_pair(values: &[u16], index: usize) -> u32 {
    u32::from(values[index]) | (u32::from(values[index + 1]) << 16)
}

fn sample_mask(bit: u32) -> u32 {
    1 << (4 + bit)
}

fn decode_mag_sgn_sample_with_vn(
    magsgn: &mut ForwardBitReader<0xFF>,
    inf: u32,
    bit: u32,
    uq: u32,
    p: u32,
) -> (u32, u32) {
    if (inf & sample_mask(bit)) == 0 {
        return (0, 0);
    }

    let ms_val = magsgn.fetch();
    let m_n = uq - ((inf >> (12 + bit)) & 1);
    magsgn.advance(m_n);

    let mut value = ms_val << 31;
    let mask = if m_n == 0 { 0 } else { (1_u32 << m_n) - 1 };
    let mut v_n = ms_val & mask;
    v_n |= ((inf >> (8 + bit)) & 1) << m_n;
    v_n |= 1;
    value |= (v_n + 2) << (p - 1);
    (value, v_n)
}

fn decode_impl(
    coded_data: &[u8],
    decoded_data: &mut [u32],
    missing_msbs: u32,
    mut num_passes: u32,
    lengths1: u32,
    lengths2: u32,
    width: u32,
    height: u32,
    stride: u32,
    stripe_causal: bool,
) -> Option<()> {
    if num_passes > 1 && lengths2 == 0 {
        num_passes = 1;
    }

    if num_passes > 3 || missing_msbs > 30 {
        return None;
    }

    if missing_msbs == 30 {
        return None;
    }

    if missing_msbs == 29 && num_passes > 1 {
        num_passes = 1;
    }

    let p = 30 - missing_msbs;
    let lcup = lengths1 as usize;
    let len2 = lengths2 as usize;

    if lcup < 2 || coded_data.len() < lcup.saturating_add(len2) {
        return None;
    }

    let scup = ((coded_data[lcup - 1] as usize) << 4) + usize::from(coded_data[lcup - 2] & 0x0F);
    if !(2..=lcup).contains(&scup) || scup > 4079 {
        return None;
    }

    let quad_rows = height.div_ceil(2) as usize;
    let sstr = ((width + 2 + 7) & !7) as usize;
    let mut scratch = vec![0u16; sstr * (quad_rows + 1)];
    let mmsbp2 = missing_msbs + 2;

    {
        let mut mel = MelDecoder::new(coded_data, lcup, scup);
        let mut vlc = ReverseBitReader::new_vlc(coded_data, lcup, scup);
        let mut run = mel.get_run()?;
        let mut c_q = 0u32;
        let mut row_offset = 0usize;
        let mut x = 0u32;

        while x < width {
            let mut vlc_val = vlc.fetch();
            let mut t0 = u32::from(VLC_TABLE0[(c_q + (vlc_val & 0x7F)) as usize]);
            if c_q == 0 {
                run -= 2;
                t0 = if run == -1 { t0 } else { 0 };
                if run < 0 {
                    run = mel.get_run()?;
                }
            }
            scratch[row_offset] = t0 as u16;
            x += 2;
            c_q = ((t0 & 0x10) << 3) | ((t0 & 0xE0) << 2);
            vlc_val = vlc.advance(t0 & 0x7);

            let mut t1 = u32::from(VLC_TABLE0[(c_q + (vlc_val & 0x7F)) as usize]);
            if c_q == 0 && x < width {
                run -= 2;
                t1 = if run == -1 { t1 } else { 0 };
                if run < 0 {
                    run = mel.get_run()?;
                }
            }
            if x >= width {
                t1 = 0;
            }
            scratch[row_offset + 2] = t1 as u16;
            x += 2;
            c_q = ((t1 & 0x10) << 3) | ((t1 & 0xE0) << 2);
            vlc_val = vlc.advance(t1 & 0x7);

            let mut uvlc_mode = ((t0 & 0x8) << 3) | ((t1 & 0x8) << 4);
            if uvlc_mode == 0xC0 {
                run -= 2;
                if run == -1 {
                    uvlc_mode += 0x40;
                }
                if run < 0 {
                    run = mel.get_run()?;
                }
            }

            let mut uvlc_entry = u32::from(UVLC_TABLE0[(uvlc_mode + (vlc_val & 0x3F)) as usize]);
            vlc_val = vlc.advance(uvlc_entry & 0x7);
            uvlc_entry >>= 3;
            let mut len = uvlc_entry & 0xF;
            let tmp = vlc_val & ((1_u32 << len) - 1);
            vlc_val = vlc.advance(len);
            uvlc_entry >>= 4;
            len = uvlc_entry & 0x7;
            uvlc_entry >>= 3;
            scratch[row_offset + 1] = (1 + (uvlc_entry & 0x7) + (tmp & !(0xFF_u32 << len))) as u16;
            scratch[row_offset + 3] = (1 + (uvlc_entry >> 3) + (tmp >> len)) as u16;

            row_offset += 4;
        }
        scratch[row_offset] = 0;
        scratch[row_offset + 1] = 0;

        for y in (2..height).step_by(2) {
            let row_base = (y >> 1) as usize * sstr;
            let prev_base = row_base - sstr;
            let mut x = 0u32;
            let mut c_q = 0u32;
            let mut row_offset = row_base;

            while x < width {
                c_q |= (u32::from(scratch[prev_base + (row_offset - row_base)]) & 0xA0) << 2;
                c_q |= (u32::from(scratch[prev_base + (row_offset - row_base) + 2]) & 0x20) << 4;

                let mut vlc_val = vlc.fetch();
                let mut t0 = u32::from(VLC_TABLE1[(c_q + (vlc_val & 0x7F)) as usize]);
                if c_q == 0 {
                    run -= 2;
                    t0 = if run == -1 { t0 } else { 0 };
                    if run < 0 {
                        run = mel.get_run()?;
                    }
                }
                scratch[row_offset] = t0 as u16;
                x += 2;

                c_q = ((t0 & 0x40) << 2) | ((t0 & 0x80) << 1);
                c_q |= u32::from(scratch[prev_base + (row_offset - row_base)]) & 0x80;
                c_q |= (u32::from(scratch[prev_base + (row_offset - row_base) + 2]) & 0xA0) << 2;
                c_q |= (u32::from(scratch[prev_base + (row_offset - row_base) + 4]) & 0x20) << 4;
                vlc_val = vlc.advance(t0 & 0x7);

                let mut t1 = u32::from(VLC_TABLE1[(c_q + (vlc_val & 0x7F)) as usize]);
                if c_q == 0 && x < width {
                    run -= 2;
                    t1 = if run == -1 { t1 } else { 0 };
                    if run < 0 {
                        run = mel.get_run()?;
                    }
                }
                if x >= width {
                    t1 = 0;
                }
                scratch[row_offset + 2] = t1 as u16;
                x += 2;

                c_q = ((t1 & 0x40) << 2) | ((t1 & 0x80) << 1);
                c_q |= u32::from(scratch[prev_base + (row_offset - row_base) + 2]) & 0x80;
                vlc_val = vlc.advance(t1 & 0x7);

                let uvlc_mode = ((t0 & 0x8) << 3) | ((t1 & 0x8) << 4);
                let mut uvlc_entry =
                    u32::from(UVLC_TABLE1[(uvlc_mode + (vlc_val & 0x3F)) as usize]);
                vlc_val = vlc.advance(uvlc_entry & 0x7);
                uvlc_entry >>= 3;
                let mut len = uvlc_entry & 0xF;
                let tmp = vlc_val & ((1_u32 << len) - 1);
                vlc_val = vlc.advance(len);
                uvlc_entry >>= 4;
                len = uvlc_entry & 0x7;
                uvlc_entry >>= 3;
                scratch[row_offset + 1] = ((uvlc_entry & 0x7) + (tmp & !(0xFF_u32 << len))) as u16;
                scratch[row_offset + 3] = ((uvlc_entry >> 3) + (tmp >> len)) as u16;

                row_offset += 4;
            }

            scratch[row_offset] = 0;
            scratch[row_offset + 1] = 0;
        }
    }

    {
        let mut magsgn = ForwardBitReader::<0xFF>::new(&coded_data[..lcup - scup]);
        let v_n_width = width.div_ceil(2) as usize + 2;
        let mut v_n_scratch = vec![0u32; v_n_width];
        let mut prev_v_n = 0u32;
        let mut x = 0u32;
        let mut sp = 0usize;
        let mut vp = 0usize;
        let mut dp = 0usize;
        let second_row_present = height > 1;

        while x < width {
            let inf = u32::from(scratch[sp]);
            let uq = u32::from(scratch[sp + 1]);
            if uq > mmsbp2 {
                return None;
            }

            let (val0, _) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 0, uq, p);
            decoded_data[dp] = val0;

            let (val1, v_n1) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 1, uq, p);
            if second_row_present {
                decoded_data[dp + stride as usize] = val1;
            }
            v_n_scratch[vp] = prev_v_n | v_n1;
            prev_v_n = 0;
            dp += 1;
            x += 1;

            if x >= width {
                vp += 1;
                break;
            }

            let (val2, _) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 2, uq, p);
            decoded_data[dp] = val2;

            let (val3, v_n3) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 3, uq, p);
            if second_row_present {
                decoded_data[dp + stride as usize] = val3;
            }
            prev_v_n = v_n3;
            dp += 1;
            x += 1;
            sp += 2;
            vp += 1;
        }
        v_n_scratch[vp] = prev_v_n;

        for y in (2..height).step_by(2) {
            let row_base = (y >> 1) as usize * sstr;
            let mut sp = row_base;
            let mut vp = 0usize;
            let mut dp = (y * stride) as usize;
            let mut prev_v_n = 0u32;
            let mut x = 0u32;
            let second_row_present = y + 1 < height;

            while x < width {
                let inf = u32::from(scratch[sp]);
                let u_q = u32::from(scratch[sp + 1]);
                let mut gamma = inf & 0xF0;
                gamma &= gamma.wrapping_sub(0x10);
                let mut emax = v_n_scratch[vp] | v_n_scratch[vp + 1];
                emax = 31 - (emax | 2).leading_zeros();
                let kappa = if gamma != 0 { emax } else { 1 };
                let uq = u_q + kappa;
                if uq > mmsbp2 {
                    return None;
                }

                let (val0, _) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 0, uq, p);
                decoded_data[dp] = val0;

                let (val1, v_n1) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 1, uq, p);
                if second_row_present {
                    decoded_data[dp + stride as usize] = val1;
                }
                v_n_scratch[vp] = prev_v_n | v_n1;
                prev_v_n = 0;
                dp += 1;
                x += 1;

                if x >= width {
                    vp += 1;
                    break;
                }

                let (val2, _) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 2, uq, p);
                decoded_data[dp] = val2;

                let (val3, v_n3) = decode_mag_sgn_sample_with_vn(&mut magsgn, inf, 3, uq, p);
                if second_row_present {
                    decoded_data[dp + stride as usize] = val3;
                }
                prev_v_n = v_n3;
                dp += 1;
                x += 1;
                sp += 2;
                vp += 1;
            }

            v_n_scratch[vp] = prev_v_n;
        }
    }

    if num_passes > 1 {
        let sigma_rows = height.div_ceil(4) as usize + 1;
        let mstr = ((width.div_ceil(4) + 2 + 7) & !7) as usize;
        let mut sigma = vec![0u16; sigma_rows * mstr];

        {
            let mut y = 0u32;
            while y < height {
                let sp_base = (y >> 1) as usize * sstr;
                let dp_base = (y >> 2) as usize * mstr;
                let mut x = 0u32;
                let mut sp = sp_base;
                let mut dp = dp_base;

                while x < width {
                    let mut t0 = ((u32::from(scratch[sp]) & 0x30) >> 4)
                        | ((u32::from(scratch[sp]) & 0xC0) >> 2);
                    t0 |= ((u32::from(scratch[sp + 2]) & 0x30) << 4)
                        | ((u32::from(scratch[sp + 2]) & 0xC0) << 6);
                    let mut t1 = ((u32::from(scratch[sp + sstr]) & 0x30) >> 2)
                        | (u32::from(scratch[sp + sstr]) & 0xC0);
                    t1 |= ((u32::from(scratch[sp + sstr + 2]) & 0x30) << 6)
                        | ((u32::from(scratch[sp + sstr + 2]) & 0xC0) << 8);
                    sigma[dp] = (t0 | t1) as u16;

                    x += 4;
                    sp += 4;
                    dp += 1;
                }

                sigma[dp] = 0;
                y += 4;
            }

            let dp_base = (height.div_ceil(4) as usize) * mstr;
            for x in 0..=width.div_ceil(4) as usize {
                sigma[dp_base + x] = 0;
            }
        }

        {
            let mut prev_row_sig = vec![0u16; width.div_ceil(4) as usize + 8];
            let mut sigprop = ForwardBitReader::<0>::new(&coded_data[lcup..lcup + len2]);

            for y in (0..height).step_by(4) {
                let mut pattern = 0xFFFFu32;
                if height - y < 4 {
                    pattern = 0x7777;
                    if height - y < 3 {
                        pattern = 0x3333;
                        if height - y < 2 {
                            pattern = 0x1111;
                        }
                    }
                }

                let mut prev = 0u32;
                let cur_row = (y >> 2) as usize * mstr;
                let next_row = cur_row + mstr;
                let dpp = (y * stride) as usize;

                for x in (0..width).step_by(4) {
                    let mut col_pattern = pattern;
                    let mut s = x as i32 + 4 - width as i32;
                    s = s.max(0);
                    col_pattern >>= (s * 4) as u32;

                    let idx = (x >> 2) as usize;
                    let ps =
                        u32::from(prev_row_sig[idx]) | (u32::from(prev_row_sig[idx + 1]) << 16);
                    let ns = read_u32_pair(&sigma, next_row + idx);
                    let mut u = (ps & 0x8888_8888) >> 3;
                    if !stripe_causal {
                        u |= (ns & 0x1111_1111) << 3;
                    }

                    let cs = read_u32_pair(&sigma, cur_row + idx);
                    let mut mbr = cs;
                    mbr |= (cs & 0x7777_7777) << 1;
                    mbr |= (cs & 0xEEEE_EEEE) >> 1;
                    mbr |= u;
                    let t = mbr;
                    mbr |= t << 4;
                    mbr |= t >> 4;
                    mbr |= prev >> 12;
                    mbr &= col_pattern;
                    mbr &= !cs;

                    let mut new_sig = mbr;
                    if new_sig != 0 {
                        let mut cwd = sigprop.fetch();
                        let mut cnt = 0u32;
                        let mut col_mask = 0xFu32;
                        let inv_sig = !cs & col_pattern;

                        for i in (0..16).step_by(4) {
                            if (col_mask & new_sig) == 0 {
                                col_mask <<= 4;
                                continue;
                            }

                            let mut sample_mask = 0x1111u32 & col_mask;
                            if (new_sig & sample_mask) != 0 {
                                new_sig &= !sample_mask;
                                if (cwd & 1) != 0 {
                                    let t = 0x33u32 << i;
                                    new_sig |= t & inv_sig;
                                }
                                cwd >>= 1;
                                cnt += 1;
                            }

                            sample_mask <<= 1;
                            if (new_sig & sample_mask) != 0 {
                                new_sig &= !sample_mask;
                                if (cwd & 1) != 0 {
                                    let t = 0x76u32 << i;
                                    new_sig |= t & inv_sig;
                                }
                                cwd >>= 1;
                                cnt += 1;
                            }

                            sample_mask <<= 1;
                            if (new_sig & sample_mask) != 0 {
                                new_sig &= !sample_mask;
                                if (cwd & 1) != 0 {
                                    let t = 0xECu32 << i;
                                    new_sig |= t & inv_sig;
                                }
                                cwd >>= 1;
                                cnt += 1;
                            }

                            sample_mask <<= 1;
                            if (new_sig & sample_mask) != 0 {
                                new_sig &= !sample_mask;
                                if (cwd & 1) != 0 {
                                    let t = 0xC8u32 << i;
                                    new_sig |= t & inv_sig;
                                }
                                cwd >>= 1;
                                cnt += 1;
                            }

                            col_mask <<= 4;
                        }

                        if new_sig != 0 {
                            let mut dp = dpp + x as usize;
                            let value = 3u32 << (p - 2);
                            let mut col_mask = 0xFu32;

                            for _ in 0..4 {
                                if (col_mask & new_sig) == 0 {
                                    col_mask <<= 4;
                                    dp += 1;
                                    continue;
                                }

                                let mut sample_mask = 0x1111u32 & col_mask;
                                if (new_sig & sample_mask) != 0 {
                                    decoded_data[dp] = (cwd << 31) | value;
                                    cwd >>= 1;
                                    cnt += 1;
                                }

                                sample_mask <<= 1;
                                if (new_sig & sample_mask) != 0 {
                                    decoded_data[dp + stride as usize] = (cwd << 31) | value;
                                    cwd >>= 1;
                                    cnt += 1;
                                }

                                sample_mask <<= 1;
                                if (new_sig & sample_mask) != 0 {
                                    decoded_data[dp + 2 * stride as usize] = (cwd << 31) | value;
                                    cwd >>= 1;
                                    cnt += 1;
                                }

                                sample_mask <<= 1;
                                if (new_sig & sample_mask) != 0 {
                                    decoded_data[dp + 3 * stride as usize] = (cwd << 31) | value;
                                    cwd >>= 1;
                                    cnt += 1;
                                }

                                col_mask <<= 4;
                                dp += 1;
                            }
                        }

                        sigprop.advance(cnt);
                    }

                    let combined_sig = new_sig | cs;
                    prev_row_sig[idx] = combined_sig as u16;
                    if idx + 1 < prev_row_sig.len() {
                        prev_row_sig[idx + 1] = (combined_sig >> 16) as u16;
                    }

                    let t = combined_sig;
                    let mut next_prev = combined_sig;
                    next_prev |= (t & 0x7777) << 1;
                    next_prev |= (t & 0xEEEE) >> 1;
                    prev = (next_prev | u) & 0xF000;
                }
            }
        }

        if num_passes > 2 {
            let mut magref = ReverseBitReader::new_mrp(coded_data, lcup, len2);
            let half = 1u32 << (p - 2);
            let mstr = ((width.div_ceil(4) + 2 + 7) & !7) as usize;
            let sigma_rows = height.div_ceil(4) as usize + 1;
            let mut sigma = vec![0u16; sigma_rows * mstr];

            let mut y = 0u32;
            while y < height {
                let sp_base = (y >> 1) as usize * sstr;
                let dp_base = (y >> 2) as usize * mstr;
                let mut x = 0u32;
                let mut sp = sp_base;
                let mut dp = dp_base;
                while x < width {
                    let mut t0 = ((u32::from(scratch[sp]) & 0x30) >> 4)
                        | ((u32::from(scratch[sp]) & 0xC0) >> 2);
                    t0 |= ((u32::from(scratch[sp + 2]) & 0x30) << 4)
                        | ((u32::from(scratch[sp + 2]) & 0xC0) << 6);
                    let mut t1 = ((u32::from(scratch[sp + sstr]) & 0x30) >> 2)
                        | (u32::from(scratch[sp + sstr]) & 0xC0);
                    t1 |= ((u32::from(scratch[sp + sstr + 2]) & 0x30) << 6)
                        | ((u32::from(scratch[sp + sstr + 2]) & 0xC0) << 8);
                    sigma[dp] = (t0 | t1) as u16;
                    x += 4;
                    sp += 4;
                    dp += 1;
                }
                sigma[dp] = 0;
                y += 4;
            }

            let dp_base = (height.div_ceil(4) as usize) * mstr;
            for x in 0..=width.div_ceil(4) as usize {
                sigma[dp_base + x] = 0;
            }

            for y in (0..height).step_by(4) {
                let mut cur_sig_idx = (y >> 2) as usize * mstr;
                let dpp = (y * stride) as usize;

                for i in (0..width).step_by(8) {
                    let cwd = magref.fetch();
                    let sig = read_u32_pair(&sigma, cur_sig_idx);
                    cur_sig_idx += 2;
                    let mut col_mask = 0xFu32;
                    let mut cwd_mut = cwd;

                    if sig != 0 {
                        for j in 0..8 {
                            if (sig & col_mask) != 0 {
                                let mut dp = dpp + i as usize + j;
                                let mut sample_mask = 0x1111_1111u32 & col_mask;

                                for _ in 0..4 {
                                    if (sig & sample_mask) != 0 {
                                        let mut sym = cwd_mut & 1;
                                        sym = (1 - sym) << (p - 1);
                                        sym |= half;
                                        decoded_data[dp] ^= sym;
                                        cwd_mut >>= 1;
                                    }
                                    sample_mask <<= 1;
                                    dp += stride as usize;
                                }
                            }
                            col_mask <<= 4;
                        }
                    }

                    magref.advance(sig.count_ones());
                }
            }
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::j2c::ht_block_encode::encode_code_block;

    #[test]
    fn test_coefficient_to_i32_shifted_alignment() {
        let aligned = 3u32 << (31 - 5);
        assert_eq!(coefficient_to_i32(aligned, 5), 3);
        assert_eq!(coefficient_to_i32(0x8000_0000 | aligned, 5), -3);
    }

    #[test]
    fn test_direct_ht_block_roundtrip_varied_4x4() {
        let original: Vec<i32> = (0..16).map(|i| (i * 3) - 20).collect();
        let total_bitplanes = 6u8;
        let encoded = encode_code_block(&original, 4, 4, total_bitplanes).expect("encode HT block");
        assert_eq!(encoded.num_coding_passes, 1);

        let mut decoded = vec![0u32; original.len()];
        let decoded_ok = decode_impl(
            &encoded.data,
            &mut decoded,
            u32::from(encoded.num_zero_bitplanes),
            u32::from(encoded.num_coding_passes),
            encoded.data.len() as u32,
            0,
            4,
            4,
            4,
            false,
        );
        assert!(decoded_ok.is_some(), "encoded={:02x?}", encoded.data);

        let decoded_i32: Vec<i32> = decoded
            .into_iter()
            .map(|value| coefficient_to_i32(value, total_bitplanes))
            .collect();
        assert_eq!(decoded_i32, original, "encoded={:02x?}", encoded.data);
    }

    #[test]
    fn test_direct_ht_block_roundtrip_positive_varied_4x4() {
        let original: Vec<i32> = (0..16).map(|i| i * 3).collect();
        let total_bitplanes = 6u8;
        let encoded = encode_code_block(&original, 4, 4, total_bitplanes).expect("encode HT block");
        assert_eq!(encoded.num_coding_passes, 1);

        let mut decoded = vec![0u32; original.len()];
        let decoded_ok = decode_impl(
            &encoded.data,
            &mut decoded,
            u32::from(encoded.num_zero_bitplanes),
            u32::from(encoded.num_coding_passes),
            encoded.data.len() as u32,
            0,
            4,
            4,
            4,
            false,
        );
        assert!(decoded_ok.is_some(), "encoded={:02x?}", encoded.data);

        let decoded_i32: Vec<i32> = decoded
            .into_iter()
            .map(|value| coefficient_to_i32(value, total_bitplanes))
            .collect();
        assert_eq!(decoded_i32, original, "encoded={:02x?}", encoded.data);
    }
}
