//! Segment relation predicates.

use crate::algorithms::orientation::{orient2d, orient2d_sign, orient2d_tol};
use crate::geom::Coord;

#[inline]
fn in_range(v: f64, a: f64, b: f64) -> bool {
    let min = a.min(b);
    let max = a.max(b);
    v >= min - 1.0e-12 && v <= max + 1.0e-12
}

#[inline]
fn in_range_eps(v: f64, a: f64, b: f64, eps: f64) -> bool {
    let min = a.min(b);
    let max = a.max(b);
    let e = eps.abs();
    v >= min - e && v <= max + e
}

/// Check whether point p lies on closed segment ab.
#[inline]
pub fn point_on_segment(p: Coord, a: Coord, b: Coord) -> bool {
    let cross = orient2d(a, b, p).abs();
    cross <= orient2d_tol(a, b, p, None) && in_range(p.x, a.x, b.x) && in_range(p.y, a.y, b.y)
}

/// Check whether point p lies on closed segment ab under caller-provided epsilon.
#[inline]
pub fn point_on_segment_eps(p: Coord, a: Coord, b: Coord, eps: f64) -> bool {
    let e = eps.abs();
    let cross = orient2d(a, b, p).abs();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    let tol_area = (e * len.max(1.0)).max(orient2d_tol(a, b, p, Some(e)));
    cross <= tol_area && in_range_eps(p.x, a.x, b.x, e) && in_range_eps(p.y, a.y, b.y, e)
}

/// Check whether closed segments a1-a2 and b1-b2 intersect.
#[inline]
pub fn segments_intersect(a1: Coord, a2: Coord, b1: Coord, b2: Coord) -> bool {
    // Fast envelope rejection.
    if a1.x.max(a2.x) < b1.x.min(b2.x)
        || b1.x.max(b2.x) < a1.x.min(a2.x)
        || a1.y.max(a2.y) < b1.y.min(b2.y)
        || b1.y.max(b2.y) < a1.y.min(a2.y)
    {
        return false;
    }

    let o1 = orient2d_sign(a1, a2, b1, None);
    let o2 = orient2d_sign(a1, a2, b2, None);
    let o3 = orient2d_sign(b1, b2, a1, None);
    let o4 = orient2d_sign(b1, b2, a2, None);

    if o1 == 0 && point_on_segment(b1, a1, a2) {
        return true;
    }
    if o2 == 0 && point_on_segment(b2, a1, a2) {
        return true;
    }
    if o3 == 0 && point_on_segment(a1, b1, b2) {
        return true;
    }
    if o4 == 0 && point_on_segment(a2, b1, b2) {
        return true;
    }

    (o1 * o2 < 0) && (o3 * o4 < 0)
}

/// Check whether closed segments a1-a2 and b1-b2 intersect under caller-provided epsilon.
#[inline]
pub fn segments_intersect_eps(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> bool {
    let e = eps.abs();
    // Fast envelope rejection.
    if a1.x.max(a2.x) < b1.x.min(b2.x) - e
        || b1.x.max(b2.x) < a1.x.min(a2.x) - e
        || a1.y.max(a2.y) < b1.y.min(b2.y) - e
        || b1.y.max(b2.y) < a1.y.min(a2.y) - e
    {
        return false;
    }

    let o1 = orient2d_sign(a1, a2, b1, Some(e));
    let o2 = orient2d_sign(a1, a2, b2, Some(e));
    let o3 = orient2d_sign(b1, b2, a1, Some(e));
    let o4 = orient2d_sign(b1, b2, a2, Some(e));

    if o1 == 0 && point_on_segment_eps(b1, a1, a2, e) {
        return true;
    }
    if o2 == 0 && point_on_segment_eps(b2, a1, a2, e) {
        return true;
    }
    if o3 == 0 && point_on_segment_eps(a1, b1, b2, e) {
        return true;
    }
    if o4 == 0 && point_on_segment_eps(a2, b1, b2, e) {
        return true;
    }

    (o1 * o2 < 0) && (o3 * o4 < 0)
}
