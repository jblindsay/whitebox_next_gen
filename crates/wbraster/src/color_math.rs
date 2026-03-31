//! HSI colour-space conversion helpers for packed-RGB and normalised-RGB raster values.
//!
//! These functions are shared across image-filter tools in both `wbtools_oss` and
//! `wbtools_pro` and live here in `wbraster` so that neither tool crate needs to
//! depend on the other.

use std::f64::consts::PI;

/// Extract the intensity (I) channel from a packed-ARGB `f64` pixel value.
///
/// Bits 0–7 = R, bits 8–15 = G, bits 16–23 = B.  Returns a normalised intensity
/// in `[0.0, 1.0]`.
pub fn value2i(value: f64) -> f64 {
    let r = (value as u32 & 0xFF) as f64 / 255.0;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255.0;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255.0;
    (r + g + b) / 3.0
}

/// Convert a packed-ARGB `f64` pixel value to the HSI colour space.
///
/// Returns `(H, S, I)` where H is in radians `[0, 2π)`, S ∈ `[0, 1]`, and I ∈ `[0, 1]`.
pub fn value2hsi(value: f64) -> (f64, f64, f64) {
    let r = (value as u32 & 0xFF) as f64 / 255.0;
    let g = ((value as u32 >> 8) & 0xFF) as f64 / 255.0;
    let b = ((value as u32 >> 16) & 0xFF) as f64 / 255.0;
    rgb_to_hsi_norm(r, g, b)
}

/// Pack an HSI triplet back into a packed-ARGB `f64` pixel value.
///
/// The alpha channel is always set to 255.
pub fn hsi2value(h: f64, s: f64, i: f64) -> f64 {
    let (r, g, b) = hsi_to_rgb_norm(h, s, i);
    let r = (r * 255.0).round().clamp(0.0, 255.0) as u32;
    let g = (g * 255.0).round().clamp(0.0, 255.0) as u32;
    let b = (b * 255.0).round().clamp(0.0, 255.0) as u32;
    ((255 << 24) | (b << 16) | (g << 8) | r) as f64
}

/// Convert normalised RGB ∈ `[0, 1]³` to the HSI colour space.
///
/// Returns `(H, S, I)`.  H is in radians `[0, 2π)`.
pub fn rgb_to_hsi_norm(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let sum = r + g + b;
    if sum <= f64::EPSILON {
        return (0.0, 0.0, 0.0);
    }

    let i = sum / 3.0;
    let rn = r / sum;
    let gn = g / sum;
    let bn = b / sum;

    let mut h = if rn != gn || rn != bn {
        ((0.5 * ((rn - gn) + (rn - bn)))
            / ((rn - gn) * (rn - gn) + (rn - bn) * (gn - bn)).sqrt())
        .acos()
    } else {
        0.0
    };
    if b > g {
        h = 2.0 * PI - h;
    }

    let s = 1.0 - 3.0 * rn.min(gn).min(bn);
    (h, s, i)
}

/// Convert an HSI triplet back to normalised RGB ∈ `[0, 1]³`.
pub fn hsi_to_rgb_norm(h: f64, s: f64, i: f64) -> (f64, f64, f64) {
    let x = i * (1.0 - s);

    let (r, g, b) = if h < 2.0 * PI / 3.0 {
        let y = i * (1.0 + (s * h.cos()) / ((PI / 3.0 - h).cos()));
        let z = 3.0 * i - (x + y);
        (y, z, x)
    } else if h < 4.0 * PI / 3.0 {
        let h = h - 2.0 * PI / 3.0;
        let y = i * (1.0 + (s * h.cos()) / ((PI / 3.0 - h).cos()));
        let z = 3.0 * i - (x + y);
        (x, y, z)
    } else {
        let h = h - 4.0 * PI / 3.0;
        let y = i * (1.0 + (s * h.cos()) / ((PI / 3.0 - h).cos()));
        let z = 3.0 * i - (x + y);
        (z, x, y)
    };

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_packed_rgb() {
        // Encode a known colour, decompose to HSI, reconstruct, check values match.
        let r: u32 = 120;
        let g: u32 = 80;
        let b: u32 = 200;
        let packed = ((255 << 24) | (b << 16) | (g << 8) | r) as f64;

        let (h, s, _i) = value2hsi(packed);
        let i_new = value2i(packed);
        let packed_out = hsi2value(h, s, i_new);

        let r_out = packed_out as u32 & 0xFF;
        let g_out = (packed_out as u32 >> 8) & 0xFF;
        let b_out = (packed_out as u32 >> 16) & 0xFF;

        assert!((r as i32 - r_out as i32).unsigned_abs() <= 2, "R channel off by >{}", 2);
        assert!((g as i32 - g_out as i32).unsigned_abs() <= 2, "G channel off by >{}", 2);
        assert!((b as i32 - b_out as i32).unsigned_abs() <= 2, "B channel off by >{}", 2);
    }

    #[test]
    fn grey_pixel_has_zero_saturation() {
        let v = 128u32;
        let packed = ((255 << 24) | (v << 16) | (v << 8) | v) as f64;
        let (_h, s, _i) = value2hsi(packed);
        assert!(s < 1e-9, "grey pixel should have S ≈ 0, got {s}");
    }
}
