//! Scalar HTJ2K cleanup-only block encoding.

use alloc::vec;
use alloc::vec::Vec;

use super::bitplane_encode::EncodedCodeBlock;
use super::ht_encode_tables::{
    HtUvlcTableEntry, HT_UVLC_ENCODE_TABLE, HT_VLC_ENCODE_TABLE0, HT_VLC_ENCODE_TABLE1,
};

const MEL_EXP: [usize; 13] = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];
const MAX_HT_BITPLANES: u8 = 30;
const MEL_SIZE: usize = 192;
const VLC_SIZE: usize = 3072 - MEL_SIZE;
const MS_SIZE: usize = (16384usize * 16).div_ceil(15);

struct MelEncoder {
    buffer: Vec<u8>,
    pos: usize,
    remaining_bits: u8,
    tmp: u8,
    run: usize,
    k: usize,
    threshold: usize,
}

impl MelEncoder {
    fn new() -> Self {
        Self {
            buffer: vec![0; MEL_SIZE],
            pos: 0,
            remaining_bits: 8,
            tmp: 0,
            run: 0,
            k: 0,
            threshold: 1,
        }
    }

    fn emit_bit(&mut self, bit: bool) -> Result<(), &'static str> {
        self.tmp = (self.tmp << 1) | u8::from(bit);
        self.remaining_bits -= 1;

        if self.remaining_bits == 0 {
            if self.pos >= self.buffer.len() {
                return Err("HTJ2K MEL encoder buffer is full");
            }

            self.buffer[self.pos] = self.tmp;
            self.pos += 1;
            self.remaining_bits = if self.tmp == 0xFF { 7 } else { 8 };
            self.tmp = 0;
        }

        Ok(())
    }

    fn encode(&mut self, bit: bool) -> Result<(), &'static str> {
        if !bit {
            self.run += 1;
            if self.run >= self.threshold {
                self.emit_bit(true)?;
                self.run = 0;
                self.k = (self.k + 1).min(MEL_EXP.len() - 1);
                self.threshold = 1 << MEL_EXP[self.k];
            }
        } else {
            self.emit_bit(false)?;
            let mut t = MEL_EXP[self.k];
            while t > 0 {
                t -= 1;
                self.emit_bit(((self.run >> t) & 1) != 0)?;
            }
            self.run = 0;
            self.k = self.k.saturating_sub(1);
            self.threshold = 1 << MEL_EXP[self.k];
        }

        Ok(())
    }
}

struct VlcEncoder {
    buffer: Vec<u8>,
    pos: usize,
    used_bits: u8,
    tmp: u8,
    last_greater_than_8f: bool,
}

impl VlcEncoder {
    fn new() -> Self {
        let mut buffer = vec![0; VLC_SIZE];
        let last = buffer.len() - 1;
        buffer[last] = 0xFF;

        Self {
            buffer,
            pos: 1,
            used_bits: 4,
            tmp: 0x0F,
            last_greater_than_8f: true,
        }
    }

    fn encode(&mut self, mut codeword: u32, mut codeword_len: u8) -> Result<(), &'static str> {
        while codeword_len > 0 {
            if self.pos >= self.buffer.len() {
                return Err("HTJ2K VLC encoder buffer is full");
            }

            let mut available_bits = 8 - u8::from(self.last_greater_than_8f) - self.used_bits;
            let take = available_bits.min(codeword_len);
            let mask = if take == 32 {
                u32::MAX
            } else {
                (1u32 << take) - 1
            };
            self.tmp |= ((codeword & mask) as u8) << self.used_bits;
            self.used_bits += take;
            available_bits -= take;
            codeword_len -= take;
            codeword >>= take;

            if available_bits == 0 {
                if self.last_greater_than_8f && self.tmp != 0x7F {
                    self.last_greater_than_8f = false;
                    continue;
                }

                let write_index = self.buffer.len() - 1 - self.pos;
                self.buffer[write_index] = self.tmp;
                self.pos += 1;
                self.last_greater_than_8f = self.tmp > 0x8F;
                self.tmp = 0;
                self.used_bits = 0;
            }
        }

        Ok(())
    }
}

struct MagSgnEncoder {
    buffer: Vec<u8>,
    pos: usize,
    max_bits: u8,
    used_bits: u8,
    tmp: u32,
}

impl MagSgnEncoder {
    fn new() -> Self {
        Self {
            buffer: vec![0; MS_SIZE],
            pos: 0,
            max_bits: 8,
            used_bits: 0,
            tmp: 0,
        }
    }

    fn encode(&mut self, mut codeword: u32, mut codeword_len: u32) -> Result<(), &'static str> {
        while codeword_len > 0 {
            if self.pos >= self.buffer.len() {
                return Err("HTJ2K magnitude/sign encoder buffer is full");
            }

            let take = u32::from(self.max_bits - self.used_bits).min(codeword_len);
            let mask = if take == 32 {
                u32::MAX
            } else {
                (1u32 << take) - 1
            };
            self.tmp |= (codeword & mask) << self.used_bits;
            self.used_bits += take as u8;
            codeword >>= take;
            codeword_len -= take;

            if self.used_bits >= self.max_bits {
                self.buffer[self.pos] = self.tmp as u8;
                self.pos += 1;
                self.max_bits = if self.tmp == 0xFF { 7 } else { 8 };
                self.tmp = 0;
                self.used_bits = 0;
            }
        }

        Ok(())
    }

    fn terminate(&mut self) -> Result<(), &'static str> {
        if self.used_bits > 0 {
            let unused = self.max_bits - self.used_bits;
            self.tmp |= (0xFF & ((1u32 << unused) - 1)) << self.used_bits;
            self.used_bits += unused;

            if self.tmp != 0xFF {
                if self.pos >= self.buffer.len() {
                    return Err("HTJ2K magnitude/sign encoder buffer is full");
                }

                self.buffer[self.pos] = self.tmp as u8;
                self.pos += 1;
            }
        } else if self.max_bits == 7 {
            self.pos = self.pos.saturating_sub(1);
        }

        Ok(())
    }
}

pub(crate) fn encode_code_block(
    coefficients: &[i32],
    width: u32,
    height: u32,
    total_bitplanes: u8,
) -> Result<EncodedCodeBlock, &'static str> {
    if total_bitplanes == 0 || total_bitplanes > MAX_HT_BITPLANES {
        return Err("HTJ2K scalar encoder currently supports 1..=30 bitplanes");
    }

    let max_magnitude = coefficients
        .iter()
        .map(|coefficient| coefficient.unsigned_abs())
        .max()
        .unwrap_or(0);

    if max_magnitude == 0 {
        return Ok(EncodedCodeBlock {
            data: Vec::new(),
            num_coding_passes: 0,
            num_zero_bitplanes: total_bitplanes,
        });
    }

    let block_bitplanes = (u32::BITS - max_magnitude.leading_zeros()) as u8;
    if block_bitplanes > total_bitplanes {
        return Err("HTJ2K block magnitude exceeds configured bitplane count");
    }

    let missing_msbs = total_bitplanes.saturating_sub(1);
    let aligned = convert_to_aligned_sign_magnitude(coefficients, total_bitplanes);
    let data = encode_cleanup_segment(&aligned, missing_msbs, width as usize, height as usize)?;

    Ok(EncodedCodeBlock {
        data,
        num_coding_passes: 1,
        num_zero_bitplanes: missing_msbs,
    })
}

fn convert_to_aligned_sign_magnitude(coefficients: &[i32], k_max: u8) -> Vec<u32> {
    let shift = u32::from(31_u8.saturating_sub(k_max));

    coefficients
        .iter()
        .map(|&coefficient| {
            if coefficient == 0 {
                0
            } else {
                let sign = if coefficient < 0 { 0x8000_0000 } else { 0 };
                let magnitude = coefficient.unsigned_abs() << shift;
                sign | magnitude
            }
        })
        .collect()
}

fn encode_cleanup_segment(
    coefficients: &[u32],
    missing_msbs: u8,
    width: usize,
    height: usize,
) -> Result<Vec<u8>, &'static str> {
    let mut mel = MelEncoder::new();
    let mut vlc = VlcEncoder::new();
    let mut ms = MagSgnEncoder::new();

    let p = 30_u32.saturating_sub(u32::from(missing_msbs));
    let stride = width;

    let mut e_val = [0u8; 513];
    let mut cx_val = [0u8; 513];

    let mut e_qmax = [0i32; 2];
    let mut e_q = [0i32; 8];
    let mut rho = [0i32; 2];
    let mut c_q0 = 0usize;
    let mut s = [0u32; 8];
    let mut sp = 0usize;
    let mut x = 0usize;

    while x < width {
        encode_first_quad_pair(
            coefficients,
            stride,
            height,
            p,
            &mut sp,
            x,
            &mut e_val,
            &mut cx_val,
            &mut c_q0,
            &mut rho,
            &mut e_q,
            &mut e_qmax,
            &mut s,
            &mut mel,
            &mut vlc,
            &mut ms,
        )?;
        x += 4;
    }

    let e_val_sentinel = width.div_ceil(2) + 1;
    e_val[e_val_sentinel] = 0;

    let mut y = 2usize;
    while y < height {
        let mut lep = 0usize;
        let mut max_e = i32::from(e_val[lep].max(e_val[lep + 1])) - 1;
        e_val[lep] = 0;

        let mut lcxp = 0usize;
        c_q0 = usize::from(cx_val[lcxp]) + (usize::from(cx_val[lcxp + 1]) << 2);
        cx_val[lcxp] = 0;

        sp = y * stride;
        x = 0;
        while x < width {
            encode_non_initial_quad_pair(
                coefficients,
                stride,
                width,
                height,
                y,
                p,
                &mut sp,
                x,
                &mut e_val,
                &mut cx_val,
                &mut lep,
                &mut lcxp,
                &mut max_e,
                &mut c_q0,
                &mut rho,
                &mut e_q,
                &mut e_qmax,
                &mut s,
                &mut mel,
                &mut vlc,
                &mut ms,
            )?;
            x += 4;
        }

        y += 2;
    }

    terminate_mel_vlc(&mut mel, &mut vlc)?;
    ms.terminate()?;

    let total_len = ms.pos + mel.pos + vlc.pos;
    if total_len < 2 {
        return Err("HTJ2K cleanup segment is too short");
    }

    let mut data = Vec::with_capacity(total_len);
    data.extend_from_slice(&ms.buffer[..ms.pos]);
    data.extend_from_slice(&mel.buffer[..mel.pos]);
    let vlc_start = vlc.buffer.len() - vlc.pos;
    data.extend_from_slice(&vlc.buffer[vlc_start..]);

    let locator_bytes = mel.pos + vlc.pos;
    let last = data.len() - 1;
    let prev = data.len() - 2;
    data[last] = (locator_bytes >> 4) as u8;
    data[prev] = (data[prev] & 0xF0) | ((locator_bytes as u8) & 0x0F);

    Ok(data)
}

fn process_sample(
    slot: usize,
    value: u32,
    p: u32,
    rho_acc: &mut i32,
    e_q: &mut [i32; 8],
    e_qmax: &mut i32,
    s: &mut [u32; 8],
) {
    let mut val = value.wrapping_add(value);
    val >>= p;
    val &= !1u32;
    if val != 0 {
        *rho_acc |= 1 << (slot & 0x3);
        val -= 1;
        e_q[slot] = (u32::BITS - val.leading_zeros()) as i32;
        *e_qmax = (*e_qmax).max(e_q[slot]);
        val -= 1;
        s[slot] = val + (value >> 31);
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_first_quad_pair(
    coefficients: &[u32],
    stride: usize,
    height: usize,
    p: u32,
    sp: &mut usize,
    x: usize,
    e_val: &mut [u8; 513],
    cx_val: &mut [u8; 513],
    c_q0: &mut usize,
    rho: &mut [i32; 2],
    e_q: &mut [i32; 8],
    e_qmax: &mut [i32; 2],
    s: &mut [u32; 8],
    mel: &mut MelEncoder,
    vlc: &mut VlcEncoder,
    ms: &mut MagSgnEncoder,
) -> Result<(), &'static str> {
    let lep = x / 2;
    let lcxp = x / 2;

    process_sample(0, coefficients[*sp], p, &mut rho[0], e_q, &mut e_qmax[0], s);
    process_sample(
        1,
        if height > 1 {
            coefficients[*sp + stride]
        } else {
            0
        },
        p,
        &mut rho[0],
        e_q,
        &mut e_qmax[0],
        s,
    );
    *sp += 1;

    if x + 1 < stride {
        process_sample(2, coefficients[*sp], p, &mut rho[0], e_q, &mut e_qmax[0], s);
        process_sample(
            3,
            if height > 1 {
                coefficients[*sp + stride]
            } else {
                0
            },
            p,
            &mut rho[0],
            e_q,
            &mut e_qmax[0],
            s,
        );
        *sp += 1;
    }

    let u_q0 = encode_quad_initial_row(
        0, *c_q0, rho[0], e_qmax[0], e_q, s, lep, lcxp, e_val, cx_val, mel, vlc, ms,
    )?;

    if x + 2 < stride {
        process_sample(4, coefficients[*sp], p, &mut rho[1], e_q, &mut e_qmax[1], s);
        process_sample(
            5,
            if height > 1 {
                coefficients[*sp + stride]
            } else {
                0
            },
            p,
            &mut rho[1],
            e_q,
            &mut e_qmax[1],
            s,
        );
        *sp += 1;

        if x + 3 < stride {
            process_sample(6, coefficients[*sp], p, &mut rho[1], e_q, &mut e_qmax[1], s);
            process_sample(
                7,
                if height > 1 {
                    coefficients[*sp + stride]
                } else {
                    0
                },
                p,
                &mut rho[1],
                e_q,
                &mut e_qmax[1],
                s,
            );
            *sp += 1;
        }

        let c_q1 = ((rho[0] >> 1) | (rho[0] & 1)) as usize;
        let u_q1 = encode_quad_initial_row(
            4,
            c_q1,
            rho[1],
            e_qmax[1],
            e_q,
            s,
            lep + 1,
            lcxp + 1,
            e_val,
            cx_val,
            mel,
            vlc,
            ms,
        )?;

        if u_q0 > 0 && u_q1 > 0 {
            mel.encode(u_q0.min(u_q1) > 2)?;
        }
        encode_uvlc(u_q0, u_q1, &mut *vlc)?;
        *c_q0 = ((rho[1] >> 1) | (rho[1] & 1)) as usize;
    } else {
        encode_uvlc(u_q0, 0, &mut *vlc)?;
        *c_q0 = 0;
    }

    *rho = [0; 2];
    *e_q = [0; 8];
    *e_qmax = [0; 2];
    *s = [0; 8];

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn encode_non_initial_quad_pair(
    coefficients: &[u32],
    stride: usize,
    width: usize,
    height: usize,
    y: usize,
    p: u32,
    sp: &mut usize,
    x: usize,
    e_val: &mut [u8; 513],
    cx_val: &mut [u8; 513],
    lep: &mut usize,
    lcxp: &mut usize,
    max_e: &mut i32,
    c_q0: &mut usize,
    rho: &mut [i32; 2],
    e_q: &mut [i32; 8],
    e_qmax: &mut [i32; 2],
    s: &mut [u32; 8],
    mel: &mut MelEncoder,
    vlc: &mut VlcEncoder,
    ms: &mut MagSgnEncoder,
) -> Result<(), &'static str> {
    process_sample(0, coefficients[*sp], p, &mut rho[0], e_q, &mut e_qmax[0], s);
    process_sample(
        1,
        if y + 1 < height {
            coefficients[*sp + stride]
        } else {
            0
        },
        p,
        &mut rho[0],
        e_q,
        &mut e_qmax[0],
        s,
    );
    *sp += 1;

    if x + 1 < width {
        process_sample(2, coefficients[*sp], p, &mut rho[0], e_q, &mut e_qmax[0], s);
        process_sample(
            3,
            if y + 1 < height {
                coefficients[*sp + stride]
            } else {
                0
            },
            p,
            &mut rho[0],
            e_q,
            &mut e_qmax[0],
            s,
        );
        *sp += 1;
    }

    let prev_max = *max_e;
    let u_q0 = encode_quad_non_initial_row(
        0, *c_q0, rho[0], e_qmax[0], prev_max, e_q, s, *lep, *lcxp, e_val, cx_val, mel, vlc, ms,
    )?;

    e_val[*lep] = e_val[*lep].max(e_q[1] as u8);
    *lep += 1;
    *max_e = i32::from(e_val[*lep].max(e_val[*lep + 1])) - 1;
    e_val[*lep] = e_q[3] as u8;
    cx_val[*lcxp] |= ((rho[0] & 2) >> 1) as u8;
    *lcxp += 1;
    let c_q1 = usize::from(cx_val[*lcxp]) + (usize::from(cx_val[*lcxp + 1]) << 2);
    cx_val[*lcxp] = ((rho[0] & 8) >> 3) as u8;

    let mut u_q1 = 0;
    if x + 2 < width {
        process_sample(4, coefficients[*sp], p, &mut rho[1], e_q, &mut e_qmax[1], s);
        process_sample(
            5,
            if y + 1 < height {
                coefficients[*sp + stride]
            } else {
                0
            },
            p,
            &mut rho[1],
            e_q,
            &mut e_qmax[1],
            s,
        );
        *sp += 1;

        if x + 3 < width {
            process_sample(6, coefficients[*sp], p, &mut rho[1], e_q, &mut e_qmax[1], s);
            process_sample(
                7,
                if y + 1 < height {
                    coefficients[*sp + stride]
                } else {
                    0
                },
                p,
                &mut rho[1],
                e_q,
                &mut e_qmax[1],
                s,
            );
            *sp += 1;
        }

        let mut c_q1_local = c_q1;
        c_q1_local |= ((rho[0] & 4) >> 1) as usize;
        c_q1_local |= ((rho[0] & 8) >> 2) as usize;

        u_q1 = encode_quad_non_initial_row(
            4, c_q1_local, rho[1], e_qmax[1], *max_e, e_q, s, *lep, *lcxp, e_val, cx_val, mel, vlc,
            ms,
        )?;

        e_val[*lep] = e_val[*lep].max(e_q[5] as u8);
        *lep += 1;
        *max_e = i32::from(e_val[*lep].max(e_val[*lep + 1])) - 1;
        e_val[*lep] = e_q[7] as u8;
        cx_val[*lcxp] |= ((rho[1] & 2) >> 1) as u8;
        *lcxp += 1;
        *c_q0 = usize::from(cx_val[*lcxp]) + (usize::from(cx_val[*lcxp + 1]) << 2);
        cx_val[*lcxp] = ((rho[1] & 8) >> 3) as u8;

        *c_q0 |= ((rho[1] & 4) >> 1) as usize;
        *c_q0 |= ((rho[1] & 8) >> 2) as usize;
    } else {
        *c_q0 = 0;
    }

    encode_uvlc_non_initial(u_q0, u_q1, &mut *vlc)?;

    *rho = [0; 2];
    *e_q = [0; 8];
    *e_qmax = [0; 2];
    *s = [0; 8];

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn encode_quad_initial_row(
    offset: usize,
    c_q: usize,
    rho: i32,
    e_qmax: i32,
    e_q: &[i32; 8],
    s: &[u32; 8],
    lep: usize,
    lcxp: usize,
    e_val: &mut [u8; 513],
    cx_val: &mut [u8; 513],
    mel: &mut MelEncoder,
    vlc: &mut VlcEncoder,
    ms: &mut MagSgnEncoder,
) -> Result<i32, &'static str> {
    let u_q = e_qmax.max(1) - 1;
    let mut eps = 0u16;

    if u_q > 0 {
        eps |= u16::from((e_q[offset] == e_qmax) as u8);
        eps |= u16::from((e_q[offset + 1] == e_qmax) as u8) << 1;
        eps |= u16::from((e_q[offset + 2] == e_qmax) as u8) << 2;
        eps |= u16::from((e_q[offset + 3] == e_qmax) as u8) << 3;
    }

    e_val[lep] = e_val[lep].max(e_q[offset + 1] as u8);
    e_val[lep + 1] = e_q[offset + 3] as u8;
    cx_val[lcxp] |= ((rho & 2) >> 1) as u8;
    cx_val[lcxp + 1] = ((rho & 8) >> 3) as u8;

    let tuple = HT_VLC_ENCODE_TABLE0[(c_q << 8) | ((rho as usize) << 4) | eps as usize];
    vlc.encode(u32::from(tuple >> 8), ((tuple >> 4) & 0x7) as u8)?;

    if c_q == 0 {
        mel.encode(rho != 0)?;
    }

    encode_mag_signs(rho, e_qmax.max(1), tuple, s, offset, ms)?;
    Ok(u_q)
}

#[allow(clippy::too_many_arguments)]
fn encode_quad_non_initial_row(
    offset: usize,
    c_q: usize,
    rho: i32,
    e_qmax: i32,
    max_e: i32,
    e_q: &[i32; 8],
    s: &[u32; 8],
    _lep: usize,
    _lcxp: usize,
    _e_val: &mut [u8; 513],
    _cx_val: &mut [u8; 513],
    mel: &mut MelEncoder,
    vlc: &mut VlcEncoder,
    ms: &mut MagSgnEncoder,
) -> Result<i32, &'static str> {
    let kappa = if (rho & (rho - 1)) != 0 {
        max_e.max(1)
    } else {
        1
    };
    let u_q = e_qmax.max(kappa) - kappa;
    let mut eps = 0u16;

    if u_q > 0 {
        eps |= u16::from((e_q[offset] == e_qmax) as u8);
        eps |= u16::from((e_q[offset + 1] == e_qmax) as u8) << 1;
        eps |= u16::from((e_q[offset + 2] == e_qmax) as u8) << 2;
        eps |= u16::from((e_q[offset + 3] == e_qmax) as u8) << 3;
    }

    let tuple = HT_VLC_ENCODE_TABLE1[(c_q << 8) | ((rho as usize) << 4) | eps as usize];
    vlc.encode(u32::from(tuple >> 8), ((tuple >> 4) & 0x7) as u8)?;

    if c_q == 0 {
        mel.encode(rho != 0)?;
    }

    encode_mag_signs(rho, e_qmax.max(kappa), tuple, s, offset, ms)?;
    Ok(u_q)
}

fn encode_mag_signs(
    rho: i32,
    u_q: i32,
    tuple: u16,
    s: &[u32; 8],
    offset: usize,
    ms: &mut MagSgnEncoder,
) -> Result<(), &'static str> {
    let e_k = tuple & 0xF;

    let mut encode = |bit: i32, shift: u32, sample_offset: usize| -> Result<(), &'static str> {
        let sample_mask = 1 << bit;
        if (rho & sample_mask) == 0 {
            return Ok(());
        }

        let reduction = ((u32::from(e_k) >> shift) & 1) as i32;
        let magnitude_bits = (u_q - reduction) as u32;
        let payload = if magnitude_bits == 0 {
            0
        } else {
            s[offset + sample_offset] & ((1u32 << magnitude_bits) - 1)
        };
        ms.encode(payload, magnitude_bits)
    };

    encode(0, 0, 0)?;
    encode(1, 1, 1)?;
    encode(2, 2, 2)?;
    encode(3, 3, 3)?;
    Ok(())
}

fn encode_uvlc(u_q0: i32, u_q1: i32, vlc: &mut VlcEncoder) -> Result<(), &'static str> {
    if u_q0 > 2 && u_q1 > 2 {
        let first = HT_UVLC_ENCODE_TABLE[(u_q0 - 2) as usize];
        let second = HT_UVLC_ENCODE_TABLE[(u_q1 - 2) as usize];
        encode_uvlc_pair(vlc, first, second)
    } else if u_q0 > 2 && u_q1 > 0 {
        let first = HT_UVLC_ENCODE_TABLE[u_q0 as usize];
        vlc.encode(u32::from(first.pre), first.pre_len)?;
        vlc.encode((u_q1 - 1) as u32, 1)?;
        vlc.encode(u32::from(first.suf), first.suf_len)
    } else {
        let first = HT_UVLC_ENCODE_TABLE[u_q0.max(0) as usize];
        let second = HT_UVLC_ENCODE_TABLE[u_q1.max(0) as usize];
        encode_uvlc_pair(vlc, first, second)
    }
}

fn encode_uvlc_non_initial(u_q0: i32, u_q1: i32, vlc: &mut VlcEncoder) -> Result<(), &'static str> {
    let first = HT_UVLC_ENCODE_TABLE[u_q0.max(0) as usize];
    let second = HT_UVLC_ENCODE_TABLE[u_q1.max(0) as usize];
    encode_uvlc_pair(vlc, first, second)
}

fn encode_uvlc_pair(
    vlc: &mut VlcEncoder,
    first: HtUvlcTableEntry,
    second: HtUvlcTableEntry,
) -> Result<(), &'static str> {
    vlc.encode(u32::from(first.pre), first.pre_len)?;
    vlc.encode(u32::from(second.pre), second.pre_len)?;
    vlc.encode(u32::from(first.suf), first.suf_len)?;
    vlc.encode(u32::from(second.suf), second.suf_len)
}

fn terminate_mel_vlc(mel: &mut MelEncoder, vlc: &mut VlcEncoder) -> Result<(), &'static str> {
    if mel.run > 0 {
        mel.emit_bit(true)?;
    }

    mel.tmp = (u16::from(mel.tmp) << mel.remaining_bits) as u8;
    let mel_mask = ((0xFFu16 << mel.remaining_bits) & 0xFF) as u8;
    let vlc_mask = if vlc.used_bits == 0 {
        0
    } else {
        ((1u16 << vlc.used_bits) - 1) as u8
    };

    if (mel_mask | vlc_mask) == 0 {
        return Ok(());
    }

    let fused = mel.tmp | vlc.tmp;
    let fused_ok =
        (((fused ^ mel.tmp) & mel_mask) | ((fused ^ vlc.tmp) & vlc_mask)) == 0 && fused != 0xFF;

    if fused_ok && vlc.pos > 1 {
        if mel.pos >= mel.buffer.len() {
            return Err("HTJ2K MEL encoder buffer is full");
        }

        mel.buffer[mel.pos] = fused;
        mel.pos += 1;
    } else {
        if mel.pos >= mel.buffer.len() {
            return Err("HTJ2K MEL encoder buffer is full");
        }
        if vlc.pos >= vlc.buffer.len() {
            return Err("HTJ2K VLC encoder buffer is full");
        }

        mel.buffer[mel.pos] = mel.tmp;
        mel.pos += 1;
        let write_index = vlc.buffer.len() - 1 - vlc.pos;
        vlc.buffer[write_index] = vlc.tmp;
        vlc.pos += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_aligned_sign_magnitude() {
        let aligned = convert_to_aligned_sign_magnitude(&[0, 1, -2, 3], 2);
        assert_eq!(aligned, vec![0, 0x2000_0000, 0xC000_0000, 0x6000_0000]);
    }

    #[test]
    fn test_encode_cleanup_only_nonzero_block() {
        let encoded = encode_code_block(&[1], 1, 1, 5).expect("encode HT block");
        assert_eq!(encoded.num_coding_passes, 1);
        assert_eq!(encoded.num_zero_bitplanes, 4);
        assert!(encoded.data.len() >= 2);
    }
}
