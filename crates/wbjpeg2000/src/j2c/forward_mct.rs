//! Forward multi-component transforms for JPEG 2000 encoding.
//!
//! Counterpart of the inverse transforms in `mct.rs`.
//! - Forward RCT (Reversible Color Transform) for lossless mode
//! - Forward ICT (Irreversible Color Transform) for lossy mode

/// Apply the forward Reversible Color Transform (RCT) in-place.
///
/// Converts (R, G, B) to (Y, Cb, Cr) using integer arithmetic:
///   Y  = floor((R + 2G + B) / 4)
///   Cb = B - G
///   Cr = R - G
///
/// This is the inverse of the decoder's inverse RCT (G.2).
pub(crate) fn forward_rct(components: &mut [Vec<f32>]) {
    debug_assert!(components.len() >= 3);
    let (r_components, rest) = components.split_at_mut(1);
    let (g_components, b_components) = rest.split_at_mut(1);
    let r_components = &mut r_components[0];
    let g_components = &mut g_components[0];
    let b_components = &mut b_components[0];

    for ((r, g), b) in r_components
        .iter_mut()
        .zip(g_components.iter_mut())
        .zip(b_components.iter_mut())
    {
        let r0 = *r;
        let g0 = *g;
        let b0 = *b;

        let y = ((r0 + 2.0 * g0 + b0) * 0.25).floor();
        let cb = b0 - g0;
        let cr = r0 - g0;

        *r = y;
        *g = cb;
        *b = cr;
    }
}

/// Apply the forward Irreversible Color Transform (ICT) in-place.
///
/// Converts (R, G, B) to (Y, Cb, Cr) using the standard matrix:
///   Y  =  0.299   * R + 0.587   * G + 0.114   * B
///   Cb = -0.16875 * R - 0.33126 * G + 0.5     * B
///   Cr =  0.5     * R - 0.41869 * G - 0.08131 * B
///
/// This is the inverse of the decoder's inverse ICT (G.3).
pub(crate) fn forward_ict(components: &mut [Vec<f32>]) {
    debug_assert!(components.len() >= 3);
    let (r_components, rest) = components.split_at_mut(1);
    let (g_components, b_components) = rest.split_at_mut(1);
    let r_components = &mut r_components[0];
    let g_components = &mut g_components[0];
    let b_components = &mut b_components[0];

    for ((r, g), b) in r_components
        .iter_mut()
        .zip(g_components.iter_mut())
        .zip(b_components.iter_mut())
    {
        let r0 = *r;
        let g0 = *g;
        let b0 = *b;

        let y = 0.299 * r0 + 0.587 * g0 + 0.114 * b0;
        let cb = -0.16875 * r0 - 0.33126 * g0 + 0.5 * b0;
        let cr = 0.5 * r0 - 0.41869 * g0 - 0.08131 * b0;

        *r = y;
        *g = cb;
        *b = cr;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_forward_rct_basic() {
        // R=100, G=150, B=200
        let mut comps = vec![vec![100.0], vec![150.0], vec![200.0]];
        forward_rct(&mut comps);

        // Y = floor((100 + 300 + 200) / 4) = floor(150) = 150
        assert_eq!(comps[0][0], 150.0);
        // Cb = 200 - 150 = 50
        assert_eq!(comps[1][0], 50.0);
        // Cr = 100 - 150 = -50
        assert_eq!(comps[2][0], -50.0);
    }

    #[test]
    fn test_rct_round_trip() {
        let r = 128.0f32;
        let g = 64.0f32;
        let b = 200.0f32;
        let mut comps = vec![vec![r], vec![g], vec![b]];
        forward_rct(&mut comps);

        // Inverse RCT (from decoder mct.rs):
        // i1 = y0 - floor((y2 + y1) * 0.25)
        // i0 = y2 + i1
        // i2 = y1 + i1
        let y0 = comps[0][0];
        let y1 = comps[1][0];
        let y2 = comps[2][0];

        let i1 = y0 - ((y2 + y1) * 0.25).floor();
        let i0 = y2 + i1;
        let i2 = y1 + i1;

        assert_eq!(i0, r);
        assert_eq!(i1, g);
        assert_eq!(i2, b);
    }

    #[test]
    fn test_forward_ict_gray() {
        // Pure gray: R=G=B=128 → Cb≈0, Cr≈0
        let mut comps = vec![vec![128.0], vec![128.0], vec![128.0]];
        forward_ict(&mut comps);
        assert!(approx_eq(comps[0][0], 128.0, 0.01));
        assert!(approx_eq(comps[1][0], 0.0, 0.01));
        assert!(approx_eq(comps[2][0], 0.0, 0.01));
    }

    #[test]
    fn test_ict_round_trip() {
        let r = 200.0f32;
        let g = 100.0f32;
        let b = 50.0f32;
        let mut comps = vec![vec![r], vec![g], vec![b]];
        forward_ict(&mut comps);

        // Inverse ICT (from decoder mct.rs):
        // i0 = y0 + 1.402 * y2
        // i1 = y0 - 0.34413 * y1 - 0.71414 * y2
        // i2 = y0 + 1.772 * y1
        let y0 = comps[0][0];
        let y1 = comps[1][0];
        let y2 = comps[2][0];

        let i0 = y0 + 1.402 * y2;
        let i1 = y0 - 0.34413 * y1 - 0.71414 * y2;
        let i2 = y0 + 1.772 * y1;

        assert!(approx_eq(i0, r, 0.1));
        assert!(approx_eq(i1, g, 0.1));
        assert!(approx_eq(i2, b, 0.1));
    }
}
