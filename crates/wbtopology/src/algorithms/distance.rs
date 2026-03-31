//! Distance computations between geometries.

use crate::algorithms::point_in_ring::{classify_point_in_ring, PointInRing};
use crate::algorithms::segment::segments_intersect;
use crate::geom::{Coord, Geometry, LineString, Polygon};

// ── Coordinate helpers ───────────────────────────────────────────────────────

/// Euclidean distance between two coordinates.
#[inline]
pub fn coord_dist(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

// ── Segment helpers ───────────────────────────────────────────────────────────

/// Nearest point on segment a→b to p, and its squared distance to p.
pub fn nearest_on_segment(p: Coord, a: Coord, b: Coord) -> (Coord, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0 {
        let d2 = (p.x - a.x).powi(2) + (p.y - a.y).powi(2);
        return (a, d2);
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let nx = a.x + t * dx;
    let ny = a.y + t * dy;
    let d2 = (p.x - nx).powi(2) + (p.y - ny).powi(2);
    (Coord::interpolate_segment(a, b, t), d2)
}

// ── LineString helpers ────────────────────────────────────────────────────────

/// Nearest point on a linestring to p, and the Euclidean distance.
pub fn nearest_on_linestring(p: Coord, ls: &LineString) -> (Coord, f64) {
    let mut best_pt = p;
    let mut best_d2 = f64::INFINITY;
    for i in 0..(ls.coords.len().saturating_sub(1)) {
        let (np, d2) = nearest_on_segment(p, ls.coords[i], ls.coords[i + 1]);
        if d2 < best_d2 {
            best_d2 = d2;
            best_pt = np;
        }
    }
    (best_pt, best_d2.sqrt())
}

// ── Polygon helpers ───────────────────────────────────────────────────────────

/// Nearest point on a polygon's exterior ring to p, and the Euclidean distance.
pub fn nearest_on_polygon_boundary(p: Coord, poly: &Polygon) -> (Coord, f64) {
    let mut best_pt = p;
    let mut best_d2 = f64::INFINITY;
    let ring = &poly.exterior.coords;
    for i in 0..(ring.len().saturating_sub(1)) {
        let (np, d2) = nearest_on_segment(p, ring[i], ring[i + 1]);
        if d2 < best_d2 {
            best_d2 = d2;
            best_pt = np;
        }
    }
    (best_pt, best_d2.sqrt())
}

/// Distance from a point to a polygon.
///
/// Returns 0 if the point is inside or on the boundary.
pub fn point_to_polygon_dist(p: Coord, poly: &Polygon) -> f64 {
    match classify_point_in_ring(p, &poly.exterior.coords) {
        PointInRing::Inside | PointInRing::Boundary => return 0.0,
        PointInRing::Outside => {}
    }
    nearest_on_polygon_boundary(p, poly).1
}

// ── Primitive nearest-point pairs ─────────────────────────────────────────────

/// Nearest point pair and distance for two geometry primitives.
///
/// Both `a` and `b` must be plain `Point`/`LineString`/`Polygon` variants.
fn nearest_prim(a: &Geometry, b: &Geometry) -> (Coord, Coord, f64) {
    let zero = Coord::xy(0.0, 0.0);
    match (a, b) {
        // ── Point / Point ────────────────────────────────────────────────────
        (Geometry::Point(pa), Geometry::Point(pb)) => {
            let d = coord_dist(*pa, *pb);
            (*pa, *pb, d)
        }
        // ── Point / LineString ───────────────────────────────────────────────
        (Geometry::Point(p), Geometry::LineString(ls)) => {
            let (q, d) = nearest_on_linestring(*p, ls);
            (*p, q, d)
        }
        (Geometry::LineString(ls), Geometry::Point(p)) => {
            let (q, d) = nearest_on_linestring(*p, ls);
            (q, *p, d)
        }
        // ── Point / Polygon ──────────────────────────────────────────────────
        (Geometry::Point(p), Geometry::Polygon(poly)) => {
            let d = point_to_polygon_dist(*p, poly);
            let q = if d == 0.0 { *p } else { nearest_on_polygon_boundary(*p, poly).0 };
            (*p, q, d)
        }
        (Geometry::Polygon(poly), Geometry::Point(p)) => {
            let d = point_to_polygon_dist(*p, poly);
            let q = if d == 0.0 { *p } else { nearest_on_polygon_boundary(*p, poly).0 };
            (q, *p, d)
        }
        // ── LineString / LineString ──────────────────────────────────────────
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            let mut best_pa = zero;
            let mut best_pb = zero;
            let mut best_d2 = f64::INFINITY;
            for i in 0..(a_ls.coords.len().saturating_sub(1)) {
                for j in 0..(b_ls.coords.len().saturating_sub(1)) {
                    let a1 = a_ls.coords[i];
                    let a2 = a_ls.coords[i + 1];
                    let b1 = b_ls.coords[j];
                    let b2 = b_ls.coords[j + 1];
                    if segments_intersect(a1, a2, b1, b2) {
                        return (a1, b1, 0.0);
                    }
                    let candidates = [
                        nearest_on_segment(a1, b1, b2),
                        nearest_on_segment(a2, b1, b2),
                        nearest_on_segment(b1, a1, a2),
                        nearest_on_segment(b2, a1, a2),
                    ];
                    for (np, d2) in candidates {
                        if d2 < best_d2 {
                            best_d2 = d2;
                            // Determine which side np belongs to
                            let da2 = (a1.x - np.x).powi(2) + (a1.y - np.y).powi(2);
                            if da2 < d2 + 1.0e-20 {
                                best_pa = np;
                                let (nb, _) = nearest_on_segment(np, b1, b2);
                                best_pb = nb;
                            } else {
                                best_pb = np;
                                let (na, _) = nearest_on_segment(np, a1, a2);
                                best_pa = na;
                            }
                        }
                    }
                }
            }
            (best_pa, best_pb, best_d2.sqrt())
        }
        // ── LineString / Polygon ─────────────────────────────────────────────
        (Geometry::LineString(ls), Geometry::Polygon(poly))
        | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            let ls_is_a = matches!(a, Geometry::LineString(_));
            // If any LS vertex is inside the polygon → distance 0
            if let Some(&first) = ls.coords.first() {
                if matches!(
                    classify_point_in_ring(first, &poly.exterior.coords),
                    PointInRing::Inside | PointInRing::Boundary
                ) {
                    return if ls_is_a {
                        (first, first, 0.0)
                    } else {
                        (first, first, 0.0)
                    };
                }
            }
            // If any LS segment crosses poly boundary → distance 0
            let ring = &poly.exterior.coords;
            for i in 0..(ls.coords.len().saturating_sub(1)) {
                let a1 = ls.coords[i];
                let a2 = ls.coords[i + 1];
                for j in 0..(ring.len().saturating_sub(1)) {
                    if segments_intersect(a1, a2, ring[j], ring[j + 1]) {
                        return (a1, ring[j], 0.0);
                    }
                }
            }
            // Minimum boundary distance
            let mut best_pa = zero;
            let mut best_pb = zero;
            let mut best_d2 = f64::INFINITY;
            for i in 0..(ls.coords.len().saturating_sub(1)) {
                let la = ls.coords[i];
                let lb = ls.coords[i + 1];
                for j in 0..(ring.len().saturating_sub(1)) {
                    let ra = ring[j];
                    let rb = ring[j + 1];
                    for (p_from_ls, p, sa, sb) in [
                        (true, la, ra, rb),
                        (true, lb, ra, rb),
                        (false, ra, la, lb),
                        (false, rb, la, lb),
                    ] {
                        let (np, d2) = nearest_on_segment(p, sa, sb);
                        if d2 < best_d2 {
                            best_d2 = d2;
                            if p_from_ls {
                                best_pa = p;
                                best_pb = np;
                            } else {
                                best_pb = p;
                                best_pa = np;
                            }
                        }
                    }
                }
            }
            if ls_is_a {
                (best_pa, best_pb, best_d2.sqrt())
            } else {
                (best_pb, best_pa, best_d2.sqrt())
            }
        }
        // ── Polygon / Polygon ────────────────────────────────────────────────
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => {
            // If a vertex of one poly is inside the other → overlap/contain → 0
            if let Some(&pt) = a_poly.exterior.coords.first() {
                if matches!(
                    classify_point_in_ring(pt, &b_poly.exterior.coords),
                    PointInRing::Inside | PointInRing::Boundary
                ) {
                    return (pt, pt, 0.0);
                }
            }
            if let Some(&pt) = b_poly.exterior.coords.first() {
                if matches!(
                    classify_point_in_ring(pt, &a_poly.exterior.coords),
                    PointInRing::Inside | PointInRing::Boundary
                ) {
                    return (pt, pt, 0.0);
                }
            }
            let ring_a = &a_poly.exterior.coords;
            let ring_b = &b_poly.exterior.coords;
            let mut best_pa = zero;
            let mut best_pb = zero;
            let mut best_d2 = f64::INFINITY;
            for i in 0..(ring_a.len().saturating_sub(1)) {
                let a1 = ring_a[i];
                let a2 = ring_a[i + 1];
                for j in 0..(ring_b.len().saturating_sub(1)) {
                    let b1 = ring_b[j];
                    let b2 = ring_b[j + 1];
                    if segments_intersect(a1, a2, b1, b2) {
                        return (a1, b1, 0.0);
                    }
                    for (p, sa, sb, is_a_side) in [
                        (a1, b1, b2, true),
                        (a2, b1, b2, true),
                        (b1, a1, a2, false),
                        (b2, a1, a2, false),
                    ] {
                        let (np, d2) = nearest_on_segment(p, sa, sb);
                        if d2 < best_d2 {
                            best_d2 = d2;
                            if is_a_side {
                                best_pa = p;
                                best_pb = np;
                            } else {
                                best_pb = p;
                                best_pa = np;
                            }
                        }
                    }
                }
            }
            (best_pa, best_pb, best_d2.sqrt())
        }
        _ => (zero, zero, f64::INFINITY),
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Nearest point pair (one on `a`, one on `b`) for any geometry pair.
pub fn nearest_points(a: &Geometry, b: &Geometry) -> (Coord, Coord) {
    let a_parts = a.components();
    let b_parts = b.components();
    let mut best_d = f64::INFINITY;
    let mut best_pa = Coord::xy(0.0, 0.0);
    let mut best_pb = Coord::xy(0.0, 0.0);
    for ap in &a_parts {
        for bp in &b_parts {
            let (pa, pb, d) = nearest_prim(ap, bp);
            if d < best_d {
                best_d = d;
                best_pa = pa;
                best_pb = pb;
            }
        }
    }
    (best_pa, best_pb)
}

/// Minimum Euclidean distance between two geometries.
///
/// Returns 0 when the geometries intersect or touch.
pub fn geometry_distance(a: &Geometry, b: &Geometry) -> f64 {
    let a_parts = a.components();
    let b_parts = b.components();
    let mut best = f64::INFINITY;
    for ap in &a_parts {
        for bp in &b_parts {
            let (_, _, d) = nearest_prim(ap, bp);
            if d < best {
                best = d;
                if best == 0.0 {
                    return 0.0;
                }
            }
        }
    }
    best
}

/// True when the distance between `a` and `b` is at most `max_distance`.
#[inline]
pub fn is_within_distance(a: &Geometry, b: &Geometry, max_distance: f64) -> bool {
    geometry_distance(a, b) <= max_distance
}
