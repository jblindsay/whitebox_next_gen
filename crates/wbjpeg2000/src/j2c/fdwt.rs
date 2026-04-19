//! Forward Discrete Wavelet Transform for JPEG 2000 encoding.
//!
//! Counterpart of the inverse DWT in `idwt.rs`.
//! Supports both 5-3 reversible (lossless) and 9-7 irreversible (lossy) transforms.
//!
//! The forward DWT decomposes spatial-domain samples into wavelet coefficients
//! organized in subbands (LL, HL, LH, HH) at each decomposition level.

use alloc::vec;
use alloc::vec::Vec;

/// 9-7 filter lifting coefficients (Table F.4 in ITU-T T.800).
const ALPHA: f32 = -1.586_134_3;
const BETA: f32 = -0.052_980_117;
const GAMMA: f32 = 0.882_911_1;
const DELTA: f32 = 0.443_506_87;
const KAPPA: f32 = 1.230_174_1;
const INV_KAPPA: f32 = 1.0 / 1.230_174_1;

/// Result of the forward DWT: wavelet coefficients organized by subbands.
#[derive(Debug)]
pub(crate) struct DwtDecomposition {
    /// LL subband coefficients (from the lowest decomposition level).
    pub(crate) ll: Vec<f32>,
    pub(crate) ll_width: u32,
    pub(crate) ll_height: u32,
    /// Each level contains (HL, LH, HH) subbands.
    pub(crate) levels: Vec<DwtLevel>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct DwtLevel {
    pub(crate) hl: Vec<f32>,
    pub(crate) lh: Vec<f32>,
    pub(crate) hh: Vec<f32>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Dimensions of the low-pass subband at this level.
    pub(crate) low_width: u32,
    pub(crate) low_height: u32,
    /// Dimensions of the high-pass subband at this level.
    pub(crate) high_width: u32,
    pub(crate) high_height: u32,
}

/// Perform multi-level forward DWT on the given image samples.
///
/// `samples` are in row-major order, `width × height`.
/// `num_levels` is the number of decomposition levels (typically 5).
/// `reversible` selects 5-3 (true) or 9-7 (false) filter.
pub(crate) fn forward_dwt(
    samples: &[f32],
    width: u32,
    height: u32,
    num_levels: u8,
    reversible: bool,
) -> DwtDecomposition {
    let w = width as usize;
    let h = height as usize;

    // Working buffer: we transform in-place level by level
    let mut buffer = samples.to_vec();
    let mut current_width = w;
    let mut current_height = h;

    let mut levels = Vec::with_capacity(num_levels as usize);

    for _ in 0..num_levels {
        if current_width < 2 && current_height < 2 {
            break;
        }

        // Apply 1D horizontal transform to each row
        if current_width >= 2 {
            let mut row_buf = vec![0.0f32; current_width];
            for y in 0..current_height {
                let row_start = y * w;
                row_buf[..current_width]
                    .copy_from_slice(&buffer[row_start..row_start + current_width]);

                if reversible {
                    forward_lift_53(&mut row_buf[..current_width]);
                } else {
                    forward_lift_97(&mut row_buf[..current_width]);
                }

                // De-interleave: evens (low) then odds (high)
                let num_low = current_width.div_ceil(2);
                for i in 0..num_low {
                    buffer[row_start + i] = row_buf[i * 2];
                }
                for i in 0..(current_width / 2) {
                    buffer[row_start + num_low + i] = row_buf[i * 2 + 1];
                }
            }
        }

        // Apply 1D vertical transform to each column
        if current_height >= 2 {
            let mut col_buf = vec![0.0f32; current_height];
            for x in 0..current_width {
                for y in 0..current_height {
                    col_buf[y] = buffer[y * w + x];
                }

                if reversible {
                    forward_lift_53(&mut col_buf[..current_height]);
                } else {
                    forward_lift_97(&mut col_buf[..current_height]);
                }

                // De-interleave: evens (low) then odds (high)
                let num_low = current_height.div_ceil(2);
                for i in 0..num_low {
                    buffer[i * w + x] = col_buf[i * 2];
                }
                for i in 0..(current_height / 2) {
                    buffer[(num_low + i) * w + x] = col_buf[i * 2 + 1];
                }
            }
        }

        let low_w = current_width.div_ceil(2);
        let low_h = current_height.div_ceil(2);
        let high_w = current_width / 2;
        let high_h = current_height / 2;

        // Extract subbands: HL (top-right), LH (bottom-left), HH (bottom-right)
        let mut hl = vec![0.0f32; high_w * low_h];
        let mut lh = vec![0.0f32; low_w * high_h];
        let mut hh = vec![0.0f32; high_w * high_h];

        for y in 0..low_h {
            for x in 0..high_w {
                hl[y * high_w + x] = buffer[y * w + low_w + x];
            }
        }
        for y in 0..high_h {
            for x in 0..low_w {
                lh[y * low_w + x] = buffer[(low_h + y) * w + x];
            }
        }
        for y in 0..high_h {
            for x in 0..high_w {
                hh[y * high_w + x] = buffer[(low_h + y) * w + low_w + x];
            }
        }

        levels.push(DwtLevel {
            hl,
            lh,
            hh,
            width: current_width as u32,
            height: current_height as u32,
            low_width: low_w as u32,
            low_height: low_h as u32,
            high_width: high_w as u32,
            high_height: high_h as u32,
        });

        current_width = low_w;
        current_height = low_h;
    }

    // Extract final LL subband
    let mut ll = vec![0.0f32; current_width * current_height];
    for y in 0..current_height {
        for x in 0..current_width {
            ll[y * current_width + x] = buffer[y * w + x];
        }
    }

    // Levels are stored from highest resolution to lowest, but we want
    // them in the same order the decoder expects (lowest to highest).
    levels.reverse();

    DwtDecomposition {
        ll,
        ll_width: current_width as u32,
        ll_height: current_height as u32,
        levels,
    }
}

/// Forward 5-3 reversible lifting (integer arithmetic).
///
/// Equations F-2 and F-3 from ITU-T T.800:
///   d(n) = x(2n+1) - floor((x(2n) + x(2n+2)) / 2)
///   s(n) = x(2n)   + floor((d(n-1) + d(n)) / 4 + 0.5)
///
/// Applied in-place: even indices are low-pass, odd indices are high-pass.
fn forward_lift_53(data: &mut [f32]) {
    let n = data.len();
    if n < 2 {
        return;
    }

    // Step 1: Predict (high-pass) — update odd samples
    // d(i) = x(2i+1) - floor((x(2i) + x(2i+2)) / 2)
    let last_even = if n % 2 == 0 { n - 2 } else { n - 1 };
    for i in (1..n).step_by(2) {
        let left = data[i - 1];
        let right = if i + 1 < n {
            data[i + 1]
        } else {
            data[last_even]
        };
        data[i] -= ((left + right) * 0.5).floor();
    }

    // Step 2: Update (low-pass) — update even samples
    // s(i) = x(2i) + floor((d(i-1) + d(i)) / 4 + 0.5)
    for i in (0..n).step_by(2) {
        let left = if i > 0 { data[i - 1] } else { data[1] };
        let right = if i + 1 < n { data[i + 1] } else { left };
        data[i] += ((left + right) * 0.25 + 0.5).floor();
    }
}

/// Forward 9-7 irreversible lifting (floating-point).
///
/// The forward transform applies the lifting steps in the order that is
/// the reverse of the inverse DWT in idwt.rs.
///
/// Forward lifting steps:
///   1. s(n) += α * (d(n-1) + d(n))     (predict high from low neighbors)
///   2. d(n) += β * (s(n) + s(n+1))     (update low from high neighbors)
///   3. s(n) += γ * (d(n-1) + d(n))     (second predict)
///   4. d(n) += δ * (s(n) + s(n+1))     (second update)
///   5. s(n) *= κ                         (scale low-pass)
///   6. d(n) *= 1/κ                       (scale high-pass)
fn forward_lift_97(data: &mut [f32]) {
    let n = data.len();
    if n < 2 {
        return;
    }

    // Step 1: α lifting on even (low-pass) samples
    for i in (0..n).step_by(2) {
        let left = if i > 0 { data[i - 1] } else { data[1] };
        let right = if i + 1 < n { data[i + 1] } else { left };
        data[i] += ALPHA * (left + right);
    }

    // Step 2: β lifting on odd (high-pass) samples
    for i in (1..n).step_by(2) {
        let left = data[i - 1];
        let right = if i + 1 < n { data[i + 1] } else { data[i - 1] };
        data[i] += BETA * (left + right);
    }

    // Step 3: γ lifting on even samples
    for i in (0..n).step_by(2) {
        let left = if i > 0 { data[i - 1] } else { data[1] };
        let right = if i + 1 < n { data[i + 1] } else { left };
        data[i] += GAMMA * (left + right);
    }

    // Step 4: δ lifting on odd samples
    for i in (1..n).step_by(2) {
        let left = data[i - 1];
        let right = if i + 1 < n { data[i + 1] } else { data[i - 1] };
        data[i] += DELTA * (left + right);
    }

    // Step 5 & 6: Scale
    for i in (0..n).step_by(2) {
        data[i] *= KAPPA;
    }
    for i in (1..n).step_by(2) {
        data[i] *= INV_KAPPA;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq_slice(a: &[f32], b: &[f32], eps: f32) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < eps)
    }

    #[test]
    fn test_forward_53_basic() {
        // Simple 4-element signal
        let mut data = vec![10.0, 20.0, 30.0, 40.0];
        forward_lift_53(&mut data);

        // After forward transform, reconstruct with inverse and check
        inverse_lift_53(&mut data);
        assert!(approx_eq_slice(&data, &[10.0, 20.0, 30.0, 40.0], 0.001));
    }

    #[test]
    fn test_forward_97_round_trip() {
        let original = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let mut data = original.clone();
        forward_lift_97(&mut data);
        inverse_lift_97(&mut data);
        assert!(approx_eq_slice(&data, &original, 0.01));
    }

    #[test]
    fn test_forward_dwt_53_single_level() {
        // 4×4 image
        let samples: Vec<f32> = (0..16).map(|x| x as f32).collect();
        let decomp = forward_dwt(&samples, 4, 4, 1, true);
        assert_eq!(decomp.ll_width, 2);
        assert_eq!(decomp.ll_height, 2);
        assert_eq!(decomp.levels.len(), 1);
    }

    #[test]
    fn test_forward_dwt_97_multi_level() {
        let samples: Vec<f32> = (0..64).map(|x| x as f32).collect();
        let decomp = forward_dwt(&samples, 8, 8, 3, false);
        assert_eq!(decomp.levels.len(), 3);
        // After 3 levels of 8×8: 4×4 → 2×2 → 1×1
        assert_eq!(decomp.ll_width, 1);
        assert_eq!(decomp.ll_height, 1);
    }

    #[test]
    fn test_odd_dimensions() {
        let samples: Vec<f32> = (0..15).map(|x| x as f32).collect();
        let decomp = forward_dwt(&samples, 5, 3, 1, true);
        assert_eq!(decomp.ll_width, 3);
        assert_eq!(decomp.ll_height, 2);
        assert_eq!(decomp.levels[0].high_width, 2);
        assert_eq!(decomp.levels[0].high_height, 1);
    }

    // Inverse lifting functions for round-trip testing
    fn inverse_lift_53(data: &mut [f32]) {
        let n = data.len();
        if n < 2 {
            return;
        }
        // Undo update
        for i in (0..n).step_by(2) {
            let left = if i > 0 { data[i - 1] } else { data[1] };
            let right = if i + 1 < n { data[i + 1] } else { left };
            data[i] -= ((left + right) * 0.25 + 0.5).floor();
        }
        // Undo predict
        let last_even = if n % 2 == 0 { n - 2 } else { n - 1 };
        for i in (1..n).step_by(2) {
            let left = data[i - 1];
            let right = if i + 1 < n {
                data[i + 1]
            } else {
                data[last_even]
            };
            data[i] += ((left + right) * 0.5).floor();
        }
    }

    fn inverse_lift_97(data: &mut [f32]) {
        let n = data.len();
        if n < 2 {
            return;
        }
        // Undo scale
        for i in (0..n).step_by(2) {
            data[i] *= 1.0 / KAPPA;
        }
        for i in (1..n).step_by(2) {
            data[i] *= KAPPA;
        }
        // Undo δ, γ, β, α in reverse order
        for i in (1..n).step_by(2) {
            let left = data[i - 1];
            let right = if i + 1 < n { data[i + 1] } else { data[i - 1] };
            data[i] -= DELTA * (left + right);
        }
        for i in (0..n).step_by(2) {
            let left = if i > 0 { data[i - 1] } else { data[1] };
            let right = if i + 1 < n { data[i + 1] } else { left };
            data[i] -= GAMMA * (left + right);
        }
        for i in (1..n).step_by(2) {
            let left = data[i - 1];
            let right = if i + 1 < n { data[i + 1] } else { data[i - 1] };
            data[i] -= BETA * (left + right);
        }
        for i in (0..n).step_by(2) {
            let left = if i > 0 { data[i - 1] } else { data[1] };
            let right = if i + 1 < n { data[i + 1] } else { left };
            data[i] -= ALPHA * (left + right);
        }
    }
}
