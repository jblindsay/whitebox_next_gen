//! Orientation and numeric helpers.

use crate::geom::Coord;

/// Default epsilon used in geometric predicate comparisons.
pub const EPS: f64 = 1.0e-12;

// Shewchuk-style floating-point roundoff bound constant for orient2d.
const ORIENT2D_ROUNDOFF_A: f64 = (3.0 + 16.0 * f64::EPSILON) * f64::EPSILON;
const SPLITTER: f64 = 134_217_729.0; // 2^27 + 1

/// Signed area * 2 of triangle (a, b, c).
#[inline]
pub fn orient2d(a: Coord, b: Coord, c: Coord) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Floating-point roundoff bound for [`orient2d`] on triangle (a, b, c).
#[inline]
pub fn orient2d_roundoff_bound(a: Coord, b: Coord, c: Coord) -> f64 {
    let acx = c.x - a.x;
    let bcx = b.x - c.x;
    let acy = c.y - a.y;
    let bcy = b.y - c.y;
    let detsum = (acx * bcy).abs() + (acy * bcx).abs();
    ORIENT2D_ROUNDOFF_A * detsum.max(1.0)
}

#[inline]
fn line_scale(a: Coord, b: Coord) -> f64 {
    let dx = (b.x - a.x).abs();
    let dy = (b.y - a.y).abs();
    (dx + dy).max(1.0)
}

/// Tolerance used for orientation sign classification.
#[inline]
pub fn orient2d_tol(a: Coord, b: Coord, c: Coord, eps: Option<f64>) -> f64 {
    let roundoff = orient2d_roundoff_bound(a, b, c);
    if let Some(e) = eps {
        roundoff.max(e.abs() * line_scale(a, b))
    } else {
        roundoff.max(EPS)
    }
}

#[inline]
fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let a_hi = c - (c - a);
    let a_lo = a - a_hi;
    (a_hi, a_lo)
}

#[inline]
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (a_hi, a_lo) = split(a);
    let (b_hi, b_lo) = split(b);
    let err = ((a_hi * b_hi - x) + a_hi * b_lo + a_lo * b_hi) + a_lo * b_lo;
    (x, err)
}

#[inline]
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    (s, err)
}

#[inline]
fn two_diff(a: f64, b: f64) -> (f64, f64) {
    two_sum(a, -b)
}

#[inline]
fn orient2d_dd(a: Coord, b: Coord, c: Coord) -> (f64, f64) {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let acx = c.x - a.x;
    let acy = c.y - a.y;

    let (p_hi, p_lo) = two_product(abx, acy);
    let (q_hi, q_lo) = two_product(aby, acx);

    let (d_hi, d_err) = two_diff(p_hi, q_hi);
    let lo = d_err + (p_lo - q_lo);
    let (hi, lo2) = two_sum(d_hi, lo);
    (hi, lo2)
}

/// Robust sign of [`orient2d`] using an uncertainty-triggered high-precision fallback.
///
/// Returns `1` for left turn, `-1` for right turn, and `0` for collinear
/// under the effective tolerance.
#[inline]
pub fn orient2d_sign(a: Coord, b: Coord, c: Coord, eps: Option<f64>) -> i8 {
    let fast = orient2d(a, b, c);
    let tol = orient2d_tol(a, b, c, eps);
    if fast > tol {
        return 1;
    }
    if fast < -tol {
        return -1;
    }

    // Fallback for numerically hard cases near zero determinant.
    let (hi, lo) = orient2d_dd(a, b, c);
    let dd = hi + lo;
    if dd > tol {
        1
    } else if dd < -tol {
        -1
    } else {
        0
    }
}

/// True when value is near zero under EPS tolerance.
#[inline]
pub fn near_zero(v: f64) -> bool {
    v.abs() <= EPS
}

/// True when value is near zero under caller-provided epsilon.
#[inline]
pub fn near_zero_eps(v: f64, eps: f64) -> bool {
    v.abs() <= eps.abs()
}
