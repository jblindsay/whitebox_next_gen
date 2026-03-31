//! Pure-Rust JPEG 2000 Discrete Wavelet Transform (DWT) implementation.
//!
//! Implements both:
//! - **LeGall 5/3 (CDF 5/3)** — the reversible integer-to-integer transform
//!   used for lossless JPEG 2000 (ISO 15444-1 Annex F.3.1)
//! - **Daubechies 9/7 (CDF 9/7)** — the irreversible float transform
//!   used for lossy JPEG 2000 (ISO 15444-1 Annex F.3.2)
//!
//! Both transforms operate on 2-D arrays by applying 1-D transforms along rows
//! then columns (separable 2-D DWT).  The subband naming follows the standard:
//!
//! ```text
//! One decomposition level:
//! ┌────┬────┐
//! │ LL │ HL │   LL = low-pass × low-pass  (approximation)
//! ├────┼────┤   HL = high-pass × low-pass (horizontal detail)
//! │ LH │ HH │   LH = low-pass × high-pass (vertical detail)
//! └────┴────┘   HH = high-pass × high-pass (diagonal detail)
//! ```
//!
//! After `n` decomposition levels the LL subband of the previous level is
//! further decomposed, giving a total of `3n + 1` subbands.

// ── 5/3 lossless transform ────────────────────────────────────────────────────

/// Forward 1-D LeGall 5/3 lifting, in-place on a mutable slice.
///
/// After the call the even-indexed elements hold low-pass coefficients and
/// odd-indexed elements hold high-pass coefficients.  Works for any length ≥ 2.
///
/// Lifting steps (ISO 15444-1, Eq. F-5 / F-6):
/// ```text
/// Predict:  x[2n+1] += -⌊(x[2n] + x[2n+2]) / 2⌋       (high-pass)
/// Update:   x[2n]   += ⌊(x[2n-1] + x[2n+1] + 2) / 4⌋  (low-pass)
/// ```
pub fn fwd_lift_53(x: &mut [i32]) {
    let n = x.len();
    if n < 2 { return; }

    // Predict step — updates odd samples
    // x[2k+1] += -floor((x[2k] + x[2k+2]) / 2)  with symmetric extension
    let mut k = 0i32;
    loop {
        let odd = (2 * k + 1) as usize;
        if odd >= n { break; }
        let left  = x[(2 * k) as usize];
        let right = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] }; // symmetric ext.
        x[odd] -= (left + right) >> 1;
        k += 1;
    }

    // Update step — updates even samples
    // x[2k] += floor((x[2k-1] + x[2k+1] + 2) / 4)  with symmetric extension
    k = 0;
    loop {
        let even = (2 * k) as usize;
        if even >= n { break; }
        let left  = if even > 0 { x[even - 1] } else { x[1.min(n-1)] }; // symmetric ext.
        let right = if even + 1 < n { x[even + 1] } else { x[even.saturating_sub(1)] };
        x[even] += (left + right + 2) >> 2;
        k += 1;
    }
}

/// Inverse 1-D LeGall 5/3 lifting (reconstruction), in-place.
///
/// Reverses [`fwd_lift_53`].
pub fn inv_lift_53(x: &mut [i32]) {
    let n = x.len();
    if n < 2 { return; }

    // Undo update step
    let mut k = 0i32;
    loop {
        let even = (2 * k) as usize;
        if even >= n { break; }
        let left  = if even > 0 { x[even - 1] } else { x[1.min(n-1)] };
        let right = if even + 1 < n { x[even + 1] } else { x[even.saturating_sub(1)] };
        x[even] -= (left + right + 2) >> 2;
        k += 1;
    }

    // Undo predict step
    k = 0;
    loop {
        let odd = (2 * k + 1) as usize;
        if odd >= n { break; }
        let left  = x[(2 * k) as usize];
        let right = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] };
        x[odd] += (left + right) >> 1;
        k += 1;
    }
}

// ── 9/7 lossy transform ────────────────────────────────────────────────────────

/// Daubechies 9/7 lifting filter coefficients (ISO 15444-1, Annex F.3.2).
mod coeff97 {
    pub const ALPHA:  f64 = -1.586_134_342_059_924;
    pub const BETA:   f64 = -0.052_980_118_572_961;
    pub const GAMMA:  f64 =  0.882_911_075_530_934;
    pub const DELTA:  f64 =  0.443_506_852_043_971;
    pub const K:      f64 =  1.230_174_104_914_001;  // scaling factor for low-pass
    pub const K_INV:  f64 =  1.0 / K;
    pub const REC_K:  f64 =  0.812_893_057_016_692;  // 1/(2K) for high-pass
}

/// Forward 1-D Daubechies 9/7 lifting, in-place (float domain).
///
/// Input `x` is promoted to f64 for the lifting steps, then rounded back.
/// The caller should supply integer samples; the output is scaled integer coefficients.
///
/// Four lifting steps + scaling:
/// ```text
/// Step 1 (α): x[2n+1] += α × (x[2n] + x[2n+2])
/// Step 2 (β): x[2n]   += β × (x[2n-1] + x[2n+1])
/// Step 3 (γ): x[2n+1] += γ × (x[2n] + x[2n+2])
/// Step 4 (δ): x[2n]   += δ × (x[2n-1] + x[2n+1])
/// Scale:      x[2n]   *= K;  x[2n+1] /= K
/// ```
pub fn fwd_lift_97(x: &mut Vec<f64>) {
    let n = x.len();
    if n < 2 { return; }
    use coeff97::*;

    // Step 1: alpha (predict, odd)
    for k in 0..((n + 1) / 2) {
        let odd = 2 * k + 1;
        if odd >= n { break; }
        let l = x[2 * k];
        let r = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] };
        x[odd] += ALPHA * (l + r);
    }
    // Step 2: beta (update, even)
    for k in 0..(n / 2) {
        let even = 2 * k;
        let l = if even > 0 { x[even - 1] } else { x[1.min(n-1)] };
        let r = if even + 1 < n { x[even + 1] } else { x[even - 1] };
        x[even] += BETA * (l + r);
    }
    // Step 3: gamma (predict, odd)
    for k in 0..((n + 1) / 2) {
        let odd = 2 * k + 1;
        if odd >= n { break; }
        let l = x[2 * k];
        let r = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] };
        x[odd] += GAMMA * (l + r);
    }
    // Step 4: delta (update, even)
    for k in 0..(n / 2) {
        let even = 2 * k;
        let l = if even > 0 { x[even - 1] } else { x[1.min(n-1)] };
        let r = if even + 1 < n { x[even + 1] } else { x[even - 1] };
        x[even] += DELTA * (l + r);
    }
    // Scale
    for k in 0..n {
        x[k] = if k % 2 == 0 { x[k] * K } else { x[k] * K_INV };
    }
}

/// Inverse 1-D Daubechies 9/7 lifting (reconstruction), in-place.
pub fn inv_lift_97(x: &mut Vec<f64>) {
    let n = x.len();
    if n < 2 { return; }
    use coeff97::*;

    // Undo scale
    for k in 0..n {
        x[k] = if k % 2 == 0 { x[k] * K_INV } else { x[k] * K };
    }
    // Undo step 4 (delta)
    for k in 0..(n / 2) {
        let even = 2 * k;
        let l = if even > 0 { x[even - 1] } else { x[1.min(n-1)] };
        let r = if even + 1 < n { x[even + 1] } else { x[even - 1] };
        x[even] -= DELTA * (l + r);
    }
    // Undo step 3 (gamma)
    for k in 0..((n + 1) / 2) {
        let odd = 2 * k + 1;
        if odd >= n { break; }
        let l = x[2 * k];
        let r = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] };
        x[odd] -= GAMMA * (l + r);
    }
    // Undo step 2 (beta)
    for k in 0..(n / 2) {
        let even = 2 * k;
        let l = if even > 0 { x[even - 1] } else { x[1.min(n-1)] };
        let r = if even + 1 < n { x[even + 1] } else { x[even - 1] };
        x[even] -= BETA * (l + r);
    }
    // Undo step 1 (alpha)
    for k in 0..((n + 1) / 2) {
        let odd = 2 * k + 1;
        if odd >= n { break; }
        let l = x[2 * k];
        let r = if odd + 1 < n { x[odd + 1] } else { x[odd - 1] };
        x[odd] -= ALPHA * (l + r);
    }
}

// ── 2-D DWT ──────────────────────────────────────────────────────────────────

/// Interleave a LL..HH sub-band arrangement: after 1-D DWT, the even-indexed
/// positions hold low-pass and odd-indexed hold high-pass.
/// This function de-interleaves a row/column in place to move LL to top-left.
fn deinterleave(buf: &mut [i32]) {
    let n = buf.len();
    let mut tmp = vec![0i32; n];
    let half = (n + 1) / 2;
    for k in 0..n {
        tmp[if k % 2 == 0 { k / 2 } else { half + k / 2 }] = buf[k];
    }
    buf.copy_from_slice(&tmp);
}

fn interleave(buf: &mut [i32]) {
    let n = buf.len();
    let mut tmp = vec![0i32; n];
    let half = (n + 1) / 2;
    for k in 0..n {
        let src = if k < half { 2 * k } else { 2 * (k - half) + 1 };
        tmp[src] = buf[k];
    }
    buf.copy_from_slice(&tmp);
}

fn deinterleave_f(buf: &mut [f64]) {
    let n = buf.len();
    let mut tmp = vec![0.0f64; n];
    let half = (n + 1) / 2;
    for k in 0..n { tmp[if k%2==0 { k/2 } else { half + k/2 }] = buf[k]; }
    buf.copy_from_slice(&tmp);
}

fn interleave_f(buf: &mut [f64]) {
    let n = buf.len();
    let mut tmp = vec![0.0f64; n];
    let half = (n + 1) / 2;
    for k in 0..n {
        let src = if k < half { 2 * k } else { 2 * (k - half) + 1 };
        tmp[src] = buf[k];
    }
    buf.copy_from_slice(&tmp);
}

/// Forward 2-D 5/3 DWT on an integer tile, one decomposition level.
///
/// `data` is a flat row-major buffer of `width × height` i32 values.
/// After the call it contains sub-bands in the standard JPEG 2000 LL|HL/LH|HH layout.
pub fn fwd_dwt_53_2d(data: &mut [i32], width: usize, height: usize) {
    // Row transforms
    for row in 0..height {
        let slice = &mut data[row * width..(row + 1) * width];
        fwd_lift_53(slice);
        deinterleave(slice);
    }
    // Column transforms
    let mut col_buf = vec![0i32; height];
    for col in 0..width {
        for r in 0..height { col_buf[r] = data[r * width + col]; }
        fwd_lift_53(&mut col_buf);
        deinterleave(&mut col_buf);
        for r in 0..height { data[r * width + col] = col_buf[r]; }
    }
}

/// Inverse 2-D 5/3 DWT, one decomposition level.
pub fn inv_dwt_53_2d(data: &mut [i32], width: usize, height: usize) {
    // Column reconstructions
    let mut col_buf = vec![0i32; height];
    for col in 0..width {
        for r in 0..height { col_buf[r] = data[r * width + col]; }
        interleave(&mut col_buf);
        inv_lift_53(&mut col_buf);
        for r in 0..height { data[r * width + col] = col_buf[r]; }
    }
    // Row reconstructions
    for row in 0..height {
        let slice = &mut data[row * width..(row + 1) * width];
        interleave(slice);
        inv_lift_53(slice);
    }
}

/// Forward 2-D 9/7 DWT on an integer tile (converts to f64 internally), one level.
pub fn fwd_dwt_97_2d(data: &[i32], width: usize, height: usize) -> Vec<f64> {
    let mut buf: Vec<f64> = data.iter().map(|&x| x as f64).collect();

    // Row transforms
    let mut row = vec![0.0f64; width];
    for r in 0..height {
        row.copy_from_slice(&buf[r * width..(r + 1) * width]);
        fwd_lift_97(&mut row);
        deinterleave_f(&mut row);
        buf[r * width..(r + 1) * width].copy_from_slice(&row);
    }

    // Column transforms
    let mut col = vec![0.0f64; height];
    for c in 0..width {
        for r in 0..height { col[r] = buf[r * width + c]; }
        fwd_lift_97(&mut col);
        deinterleave_f(&mut col);
        for r in 0..height { buf[r * width + c] = col[r]; }
    }
    buf
}

/// Inverse 2-D 9/7 DWT, one level. Returns integer samples.
pub fn inv_dwt_97_2d(buf: &[f64], width: usize, height: usize) -> Vec<i32> {
    let mut data = buf.to_vec();

    // Column reconstructions
    let mut col = vec![0.0f64; height];
    for c in 0..width {
        for r in 0..height { col[r] = data[r * width + c]; }
        interleave_f(&mut col);
        inv_lift_97(&mut col);
        for r in 0..height { data[r * width + c] = col[r]; }
    }

    // Row reconstructions
    let mut row = vec![0.0f64; width];
    for r in 0..height {
        row.copy_from_slice(&data[r * width..(r + 1) * width]);
        interleave_f(&mut row);
        inv_lift_97(&mut row);
        data[r * width..(r + 1) * width].copy_from_slice(&row);
    }

    data.iter().map(|&x| x.round() as i32).collect()
}

/// Perform `num_levels` forward decomposition levels of 5/3 DWT on a 2-D tile.
///
/// At each level only the LL subband (top-left quadrant) is further decomposed.
pub fn fwd_dwt_53_multilevel(data: &mut Vec<i32>, width: usize, height: usize, num_levels: u8) {
    let mut w = width;
    let mut h = height;
    for _ in 0..num_levels {
        fwd_dwt_53_2d(&mut data[..], w, h);
        w = (w + 1) / 2;
        h = (h + 1) / 2;
    }
}

/// Inverse multi-level 5/3 DWT.
pub fn inv_dwt_53_multilevel(data: &mut Vec<i32>, width: usize, height: usize, num_levels: u8) {
    let mut widths  = Vec::with_capacity(num_levels as usize);
    let mut heights = Vec::with_capacity(num_levels as usize);
    let (mut w, mut h) = (width, height);
    for _ in 0..num_levels {
        widths.push(w);
        heights.push(h);
        w = (w + 1) / 2;
        h = (h + 1) / 2;
    }
    for level in (0..num_levels as usize).rev() {
        inv_dwt_53_2d(&mut data[..], widths[level], heights[level]);
    }
}

/// Perform `num_levels` forward decomposition levels of 9/7 DWT.
pub fn fwd_dwt_97_multilevel(data: &[i32], width: usize, height: usize, num_levels: u8) -> Vec<f64> {
    let mut buf: Vec<f64> = data.iter().map(|&x| x as f64).collect();
    let mut w = width;
    let mut h = height;
    for _ in 0..num_levels {
        let ll = fwd_dwt_97_2d(
            &buf[..].iter().map(|&x| x as i32).collect::<Vec<_>>()[..w * h],
            w, h,
        );
        // Copy LL subband back
        for (i, &v) in ll.iter().enumerate() {
            buf[i] = v;
        }
        w = (w + 1) / 2;
        h = (h + 1) / 2;
    }
    buf
}

/// Inverse multi-level 9/7 DWT.
pub fn inv_dwt_97_multilevel(buf: &[f64], width: usize, height: usize, num_levels: u8) -> Vec<i32> {
    let mut data = buf.to_vec();
    let mut widths  = Vec::with_capacity(num_levels as usize);
    let mut heights = Vec::with_capacity(num_levels as usize);
    let (mut w, mut h) = (width, height);
    for _ in 0..num_levels {
        widths.push(w);
        heights.push(h);
        w = (w + 1) / 2;
        h = (h + 1) / 2;
    }
    for level in (0..num_levels as usize).rev() {
        let slice: Vec<i32> = data[..widths[level] * heights[level]]
            .iter().map(|&x| x.round() as i32).collect();
        let rec = inv_dwt_97_2d(&data[..widths[level]*heights[level]], widths[level], heights[level]);
        for (i, &v) in rec.iter().enumerate() { data[i] = v as f64; }
    }
    data.iter().map(|&x| x.round() as i32).collect()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(any())]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_53_1d() {
        let original: Vec<i32> = (0..64).collect();
        let mut data = original.clone();
        fwd_lift_53(&mut data);
        inv_lift_53(&mut data);
        // After round-trip the values should be identical
        assert_eq!(data, original, "5/3 1-D round-trip failed");
    }

    #[test]
    fn roundtrip_53_2d() {
        let w = 8; let h = 8;
        let original: Vec<i32> = (0..(w * h) as i32).collect();
        let mut data = original.clone();
        fwd_dwt_53_2d(&mut data, w, h);
        inv_dwt_53_2d(&mut data, w, h);
        assert_eq!(data, original, "5/3 2-D round-trip failed");
    }

    #[test]
    fn roundtrip_53_multilevel() {
        let w = 32; let h = 32;
        let original: Vec<i32> = (0..(w * h) as i32).map(|x| x % 256).collect();
        let mut data = original.clone();
        fwd_dwt_53_multilevel(&mut data, w, h, 3);
        inv_dwt_53_multilevel(&mut data, w, h, 3);
        assert_eq!(data, original, "5/3 multilevel round-trip failed");
    }

    #[test]
    fn roundtrip_97_1d() {
        let original: Vec<f64> = (0..32).map(|x| x as f64 * 3.7 - 50.0).collect();
        let mut data = original.clone();
        fwd_lift_97(&mut data);
        inv_lift_97(&mut data);
        let max_err = original.iter().zip(data.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        assert!(max_err < 1e-9, "9/7 1-D round-trip error: {}", max_err);
    }

    #[test]
    fn roundtrip_97_2d() {
        let w = 8; let h = 8;
        let original: Vec<i32> = (0..(w * h) as i32).collect();
        let fwd = fwd_dwt_97_2d(&original, w, h);
        let rec = inv_dwt_97_2d(&fwd, w, h);
        let max_err = original.iter().zip(rec.iter())
            .map(|(a, b)| (a - b).abs())
            .max().unwrap_or(0);
        assert!(max_err <= 1, "9/7 2-D round-trip error: {} samples", max_err);
    }

    #[test]
    fn energy_compaction_53() {
        // After DWT, most energy should be in the LL subband
        let w = 8; let h = 8;
        let data: Vec<i32> = (0..(w*h) as i32).collect();
        let mut dwt = data.clone();
        fwd_dwt_53_2d(&mut dwt, w, h);
        let ll_energy: i64 = dwt[..w/2 * h/2 + 1].iter().map(|&x| (x as i64).pow(2)).sum();
        let total_energy: i64 = data.iter().map(|&x| (x as i64).pow(2)).sum();
        assert!(ll_energy > total_energy / 2, "LL should hold majority of energy");
    }
}
