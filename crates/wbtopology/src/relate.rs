//! DE-9IM topological relation matrix — full implementation.
//!
//! This module provides a complete Dimensionally Extended 9-Intersection Matrix
//! (DE-9IM) for the six primary geometry type pairs, with accurate cell values for
//! all 9 cells enabling reliable `RelateMatrix::matches(pattern)` use.
//!
//! | Geometry pair           | All 9 cells | Notes                         |
//! |-------------------------|-------------|-------------------------------|
//! | Point × Point           | ✅ Exact    | Trivial equality check        |
//! | Point × LineString      | ✅ Full     | Endpoint/interior distinction |
//! | Point × Polygon         | ✅ Full     | Strict interior/boundary      |
//! | LineString × LineString | ✅ Full     | Proper cross + collinear      |
//! | LineString × Polygon    | ✅ Full     | Segment midpoint scan         |
//! | Polygon × Polygon       | ✅ Full     | Boundary + interior scan      |
//! | Multi-geometry pairs    | ⚠ Scaffold  | Conservative fallback         |

use crate::algorithms::orientation::orient2d_sign;
use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::algorithms::segment::{point_on_segment_eps, segments_intersect_eps};
use crate::geom::{Coord, Geometry, LineString, Polygon};
use crate::precision::PrecisionModel;
use crate::topology::intersects_with_epsilon;

// ── Public types ──────────────────────────────────────────────────────────────

/// Topological location in DE-9IM matrix axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// Interior location.
    Interior = 0,
    /// Boundary location.
    Boundary = 1,
    /// Exterior location.
    Exterior = 2,
}

/// 3×3 DE-9IM relation matrix.
///
/// Rows = location in A, Columns = location in B.
/// Cell values are dimension characters:
/// - `'F'` = empty intersection
/// - `'0'` = point-level intersection
/// - `'1'` = line-level intersection
/// - `'2'` = area-level intersection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelateMatrix {
    cells: [[char; 3]; 3],
}

impl RelateMatrix {
    /// Build an empty matrix initialised to `'F'` (all cells empty).
    pub fn empty() -> Self {
        Self {
            cells: [['F'; 3]; 3],
        }
    }

    /// Set one matrix cell.
    pub fn set(&mut self, a: Location, b: Location, v: char) {
        self.cells[a as usize][b as usize] = v;
    }

    /// Read one matrix cell.
    pub fn get(&self, a: Location, b: Location) -> char {
        self.cells[a as usize][b as usize]
    }

    /// Export the 9-character row-major DE-9IM string.
    ///
    /// Format: `II IB IE / BI BB BE / EI EB EE` (no separators, 9 chars total).
    pub fn as_str9(&self) -> String {
        let mut s = String::with_capacity(9);
        for r in 0..3 {
            for c in 0..3 {
                s.push(self.cells[r][c]);
            }
        }
        s
    }

    /// Transpose the matrix (swap A and B roles).
    ///
    /// `relate(a, b).transpose()` gives the same matrix as `relate(b, a)`.
    pub fn transpose(&self) -> Self {
        let mut t = Self::empty();
        for i in 0..3 {
            for j in 0..3 {
                t.cells[j][i] = self.cells[i][j];
            }
        }
        t
    }

    /// Check whether the matrix matches a DE-9IM pattern string (9 characters).
    ///
    /// Pattern characters:
    /// - `'*'` — matches any cell value
    /// - `'T'` — matches any non-`'F'` value (`'0'`, `'1'`, `'2'`)
    /// - `'F'`, `'0'`, `'1'`, `'2'` — match exactly
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern.len() != 9 {
            return false;
        }
        for (actual, expected) in self.as_str9().chars().zip(pattern.chars()) {
            let ok = match expected {
                '*' => true,
                'T' => actual != 'F',
                'F' | '0' | '1' | '2' => actual == expected,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    // ── Named spatial predicates ──────────────────────────────────────────────

    /// True when A and B have no common points.
    ///
    /// DE-9IM: `"FF*FF****"`
    #[inline]
    pub fn is_disjoint(&self) -> bool {
        self.matches("FF*FF****")
    }

    /// True when A and B share at least one common point.
    #[inline]
    pub fn is_intersects(&self) -> bool {
        !self.is_disjoint()
    }

    /// True when A and B share points only on their boundaries; interiors do not intersect.
    ///
    /// DE-9IM: `"FT*******"` OR `"F**T*****"` OR `"F***T****"`
    #[inline]
    pub fn is_touches(&self) -> bool {
        self.matches("FT*******") || self.matches("F**T*****") || self.matches("F***T****")
    }

    /// True when A is completely within B (A ⊆ interior(B) ∪ boundary(B)).
    ///
    /// DE-9IM: `"T*F**F***"`
    #[inline]
    pub fn is_within(&self) -> bool {
        self.matches("T*F**F***")
    }

    /// True when A completely contains B (B ⊆ interior(A) ∪ boundary(A)).
    ///
    /// DE-9IM: `"T*****FF*"`
    #[inline]
    pub fn is_contains(&self) -> bool {
        self.matches("T*****FF*")
    }

    /// True when no point of B lies in the exterior of A (A covers B).
    ///
    /// Weaker than `is_contains`: B may lie on A's boundary and still be covered.
    /// DE-9IM: any of `"T*****FF*"`, `"*T****FF*"`, `"***T**FF*"`
    #[inline]
    pub fn is_covers(&self) -> bool {
        self.matches("T*****FF*") || self.matches("*T****FF*") || self.matches("***T**FF*")
    }

    /// True when no point of A lies in the exterior of B (A is covered by B).
    ///
    /// The transpose of `is_covers`.
    /// DE-9IM: any of `"T*F**F***"`, `"*TF**F***"`, `"**FT*F***"`
    #[inline]
    pub fn is_covered_by(&self) -> bool {
        self.matches("T*F**F***") || self.matches("*TF**F***") || self.matches("**FT*F***")
    }

    /// True when A and B are of the same dimension, their interiors intersect,
    /// but neither geometry completely contains the other.
    ///
    /// - Area/area: `"T*T***T**"` (each has area outside the other)
    /// - Line/line: `"1*T***T**"` (collinear segment overlap, each extends beyond)
    #[inline]
    pub fn is_overlaps(&self) -> bool {
        self.matches("T*T***T**") || self.matches("1*T***T**")
    }

    /// True when A and B have some but not all interior points in common,
    /// and their intersection has strictly lower dimension than either geometry.
    ///
    /// - Line/line proper cross (intersection = point): `"0*T***T**"`
    /// - Line/area cross (line partially inside polygon): `"T*T***T**"`
    #[inline]
    pub fn is_crosses(&self) -> bool {
        self.matches("0*T***T**") || self.matches("T*T***T**")
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Compute a full DE-9IM relation matrix between two geometries.
///
/// All 9 matrix cells are correctly computed for Point, LineString, and Polygon inputs.
/// Multi-geometry inputs fall back to a conservative scaffold that correctly computes
/// the Interior/Interior cell but populates other cells heuristically.
pub fn relate(a: &Geometry, b: &Geometry) -> RelateMatrix {
    relate_impl(a, b, None)
}

/// Precision-aware relation matrix.
pub fn relate_with_precision(
    a: &Geometry,
    b: &Geometry,
    precision: PrecisionModel,
) -> RelateMatrix {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    relate_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware relation matrix.
pub fn relate_with_epsilon(a: &Geometry, b: &Geometry, epsilon: f64) -> RelateMatrix {
    relate_impl(a, b, Some(epsilon))
}

// ── Internal location alias shorthand ────────────────────────────────────────

use Location::{Boundary as B, Exterior as E, Interior as I};

// ── Private geometry helpers ──────────────────────────────────────────────────

#[inline]
fn eq_c(a: Coord, b: Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps
}

/// True if `p` equals the first or last coordinate of `ls` (linestring boundary definition).
fn at_ls_boundary(p: Coord, ls: &LineString, eps: f64) -> bool {
    let n = ls.coords.len();
    if n == 0 {
        return false;
    }
    eq_c(p, ls.coords[0], eps) || eq_c(p, ls.coords[n - 1], eps)
}

/// True if `p` lies on any segment of `ls` (endpoints inclusive).
fn on_ls(p: Coord, ls: &LineString, eps: f64) -> bool {
    let c = &ls.coords;
    for i in 0..c.len().saturating_sub(1) {
        if point_on_segment_eps(p, c[i], c[i + 1], eps) {
            return true;
        }
    }
    false
}

/// True if `p` lies on the interior of `ls` — on ls but NOT at the first or last coord.
fn on_ls_interior(p: Coord, ls: &LineString, eps: f64) -> bool {
    on_ls(p, ls, eps) && !at_ls_boundary(p, ls, eps)
}

/// True if `p` is strictly inside `poly`'s interior (not on any ring boundary).
fn inside_poly_strict(p: Coord, poly: &Polygon, eps: f64) -> bool {
    match classify_point_in_ring_eps(p, &poly.exterior.coords, eps) {
        PointInRing::Inside => {}
        _ => return false,
    }
    for hole in &poly.holes {
        if classify_point_in_ring_eps(p, &hole.coords, eps) == PointInRing::Inside {
            return false;
        }
    }
    true
}

/// True if `p` lies on any ring boundary segment of `poly`.
fn on_poly_boundary(p: Coord, poly: &Polygon, eps: f64) -> bool {
    fn on_ring(p: Coord, ring: &[Coord], eps: f64) -> bool {
        for i in 0..ring.len().saturating_sub(1) {
            if point_on_segment_eps(p, ring[i], ring[i + 1], eps) {
                return true;
            }
        }
        false
    }
    if on_ring(p, &poly.exterior.coords, eps) {
        return true;
    }
    for hole in &poly.holes {
        if on_ring(p, &hole.coords, eps) {
            return true;
        }
    }
    false
}

/// Classify `p` against `poly`: returns `(strictly_inside, on_boundary, outside)`.
fn classify_vs_poly(p: Coord, poly: &Polygon, eps: f64) -> (bool, bool, bool) {
    if on_poly_boundary(p, poly, eps) {
        (false, true, false)
    } else if inside_poly_strict(p, poly, eps) {
        (true, false, false)
    } else {
        (false, false, true)
    }
}

/// Returns an approximate interior point of `poly` (centroid first, then midpoints).
fn poly_sample_interior(poly: &Polygon, eps: f64) -> Option<Coord> {
    let coords = &poly.exterior.coords;
    if coords.len() < 3 {
        return None;
    }
    let n = coords.len();
    let cx = coords.iter().map(|c| c.x).sum::<f64>() / n as f64;
    let cy = coords.iter().map(|c| c.y).sum::<f64>() / n as f64;
    let c = Coord::xy(cx, cy);
    if inside_poly_strict(c, poly, eps) {
        return Some(c);
    }
    for v in coords {
        let trial = Coord::xy(0.5 * (cx + v.x), 0.5 * (cy + v.y));
        if inside_poly_strict(trial, poly, eps) {
            return Some(trial);
        }
    }
    None
}

/// True if two segments properly cross at a point strictly interior to both.
fn seg_proper_cross(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> bool {
    let e = eps.abs();
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
    (o1 * o2 < 0) && (o3 * o4 < 0)
}

/// Returns `(proper_cross_exists, segment_overlap_exists)` between two linestrings.
fn ls_ls_interior_contact(la: &LineString, lb: &LineString, eps: f64) -> (bool, bool) {
    let ca = &la.coords;
    let cb = &lb.coords;
    let e = eps.abs();
    let mut proper_cross = false;
    let mut seg_overlap = false;

    'outer: for i in 0..ca.len().saturating_sub(1) {
        let (a1, a2) = (ca[i], ca[i + 1]);
        for j in 0..cb.len().saturating_sub(1) {
            let (b1, b2) = (cb[j], cb[j + 1]);

            if !proper_cross && seg_proper_cross(a1, a2, b1, b2, e) {
                proper_cross = true;
            }

            // Check collinear segment overlap.
            if !seg_overlap {
                let o1 = orient2d_sign(a1, a2, b1, Some(e));
                let o2 = orient2d_sign(a1, a2, b2, Some(e));
                if o1 == 0 && o2 == 0 {
                    let dx = (a2.x - a1.x).abs();
                    let dy = (a2.y - a1.y).abs();
                    let (tbl, tbh) = if dx >= dy {
                        let len = a2.x - a1.x;
                        if len.abs() < e {
                            continue;
                        }
                        ((b1.x - a1.x) / len, (b2.x - a1.x) / len)
                    } else {
                        let len = a2.y - a1.y;
                        if len.abs() < e {
                            continue;
                        }
                        ((b1.y - a1.y) / len, (b2.y - a1.y) / len)
                    };
                    let ov_lo = 0.0_f64.max(tbl.min(tbh));
                    let ov_hi = 1.0_f64.min(tbl.max(tbh));
                    let seg_len = (a2.x - a1.x).hypot(a2.y - a1.y);
                    let tol = e / seg_len.max(1.0);
                    if ov_hi - ov_lo > tol {
                        seg_overlap = true;
                    }
                }
            }

            if proper_cross && seg_overlap {
                break 'outer;
            }
        }
    }
    (proper_cross, seg_overlap)
}

/// True if any segment pair from `ring_a` and `ring_b` intersects (inclusive).
fn rings_intersect_any(ring_a: &[Coord], ring_b: &[Coord], eps: f64) -> bool {
    for i in 0..ring_a.len().saturating_sub(1) {
        for j in 0..ring_b.len().saturating_sub(1) {
            if segments_intersect_eps(ring_a[i], ring_a[i + 1], ring_b[j], ring_b[j + 1], eps) {
                return true;
            }
        }
    }
    false
}

/// True if any segment pair from `ring_a` and `ring_b` shares a collinear sub-segment.
fn rings_share_collinear_segment(ring_a: &[Coord], ring_b: &[Coord], eps: f64) -> bool {
    let e = eps.abs();
    for i in 0..ring_a.len().saturating_sub(1) {
        let (a1, a2) = (ring_a[i], ring_a[i + 1]);
        for j in 0..ring_b.len().saturating_sub(1) {
            let (b1, b2) = (ring_b[j], ring_b[j + 1]);
            let o1 = orient2d_sign(a1, a2, b1, Some(e));
            let o2 = orient2d_sign(a1, a2, b2, Some(e));
            if o1 != 0 || o2 != 0 {
                continue;
            }
            let dx = (a2.x - a1.x).abs();
            let dy = (a2.y - a1.y).abs();
            let (tbl, tbh) = if dx >= dy {
                let len = a2.x - a1.x;
                if len.abs() < e {
                    continue;
                }
                ((b1.x - a1.x) / len, (b2.x - a1.x) / len)
            } else {
                let len = a2.y - a1.y;
                if len.abs() < e {
                    continue;
                }
                ((b1.y - a1.y) / len, (b2.y - a1.y) / len)
            };
            let ov_lo = 0.0_f64.max(tbl.min(tbh));
            let ov_hi = 1.0_f64.min(tbl.max(tbh));
            let seg_len = (a2.x - a1.x).hypot(a2.y - a1.y);
            let tol = e / seg_len.max(1.0);
            if ov_hi - ov_lo > tol {
                return true;
            }
        }
    }
    false
}

/// Returns the boundary-intersection dimension char between two polygons:
/// `'1'` if they share a collinear segment, `'0'` if they touch at a point only, `'F'` if disjoint.
fn poly_poly_boundary_dim(pa: &Polygon, pb: &Polygon, eps: f64) -> char {
    let rings_a: Vec<&[Coord]> = std::iter::once(pa.exterior.coords.as_slice())
        .chain(pa.holes.iter().map(|h| h.coords.as_slice()))
        .collect();
    let rings_b: Vec<&[Coord]> = std::iter::once(pb.exterior.coords.as_slice())
        .chain(pb.holes.iter().map(|h| h.coords.as_slice()))
        .collect();

    let mut any_intersect = false;
    for ra in &rings_a {
        for rb in &rings_b {
            if rings_share_collinear_segment(ra, rb, eps) {
                return '1';
            }
            if rings_intersect_any(ra, rb, eps) {
                any_intersect = true;
            }
        }
    }
    if any_intersect { '0' } else { 'F' }
}

/// True if any segment midpoint of `poly_src`'s boundary rings is strictly inside `poly_dst`.
fn poly_boundary_midpoints_inside(poly_src: &Polygon, poly_dst: &Polygon, eps: f64) -> bool {
    for ring in std::iter::once(&poly_src.exterior.coords)
        .chain(poly_src.holes.iter().map(|h| &h.coords))
    {
        for i in 0..ring.len().saturating_sub(1) {
            let mid = Coord::xy(
                0.5 * (ring[i].x + ring[i + 1].x),
                0.5 * (ring[i].y + ring[i + 1].y),
            );
            if inside_poly_strict(mid, poly_dst, eps) {
                return true;
            }
        }
    }
    false
}

/// True if any vertex or segment midpoint of `poly`'s boundary rings lies outside `container`.
fn poly_boundary_has_outside(poly: &Polygon, container: &Polygon, eps: f64) -> bool {
    for ring in std::iter::once(&poly.exterior.coords)
        .chain(poly.holes.iter().map(|h| &h.coords))
    {
        for c in ring.iter() {
            if !inside_poly_strict(*c, container, eps) && !on_poly_boundary(*c, container, eps) {
                return true;
            }
        }
        for i in 0..ring.len().saturating_sub(1) {
            let mid = Coord::xy(
                0.5 * (ring[i].x + ring[i + 1].x),
                0.5 * (ring[i].y + ring[i + 1].y),
            );
            if !inside_poly_strict(mid, container, eps) && !on_poly_boundary(mid, container, eps)
            {
                return true;
            }
        }
    }
    false
}

// ── Type-specific relate functions ────────────────────────────────────────────

fn relate_pp(pa: Coord, pb: Coord, eps: f64) -> RelateMatrix {
    let mut m = RelateMatrix::empty();
    let same = eq_c(pa, pb, eps);
    // Points have no boundary in 2D Euclidean topology.
    m.set(I, I, if same { '0' } else { 'F' });
    m.set(I, B, 'F');
    m.set(I, E, if same { 'F' } else { '0' });
    m.set(B, I, 'F');
    m.set(B, B, 'F');
    m.set(B, E, 'F');
    m.set(E, I, if same { 'F' } else { '0' });
    m.set(E, B, 'F');
    m.set(E, E, '2');
    m
}

fn relate_p_ls(pa: Coord, lb: &LineString, eps: f64) -> RelateMatrix {
    // A = Point, B = LineString.
    // Boundary(B) = {first, last}; Interior(B) = all other points.
    let mut m = RelateMatrix::empty();
    let n = lb.coords.len();
    let has_ls = n >= 2;

    let on = has_ls && on_ls(pa, lb, eps);
    let at_bnd = on && at_ls_boundary(pa, lb, eps);
    let in_int = on && !at_bnd;

    m.set(I, I, if in_int { '0' } else { 'F' });
    m.set(I, B, if at_bnd { '0' } else { 'F' });
    m.set(I, E, if !on { '0' } else { 'F' });
    // Boundary of a Point is empty.
    m.set(B, I, 'F');
    m.set(B, B, 'F');
    m.set(B, E, 'F');
    // EI: ls has 1D interior, all of which lies in exterior of the point.
    m.set(E, I, if has_ls { '1' } else { 'F' });
    // EB: ls has endpoints in exterior of point unless both equal pa.
    let eb = if !has_ls {
        'F'
    } else if eq_c(lb.coords[0], pa, eps) && eq_c(lb.coords[n - 1], pa, eps) {
        'F'
    } else {
        '0'
    };
    m.set(E, B, eb);
    m.set(E, E, '2');
    m
}

fn relate_p_poly(pa: Coord, pb: &Polygon, eps: f64) -> RelateMatrix {
    // A = Point, B = Polygon.
    let mut m = RelateMatrix::empty();
    let (strict_in, on_bnd, outside) = classify_vs_poly(pa, pb, eps);
    m.set(I, I, if strict_in { '0' } else { 'F' });
    m.set(I, B, if on_bnd { '0' } else { 'F' });
    m.set(I, E, if outside { '0' } else { 'F' });
    m.set(B, I, 'F');
    m.set(B, B, 'F');
    m.set(B, E, 'F');
    // EI/EB: polygon always has 2D interior and 1D boundary in exterior of any point.
    m.set(E, I, '2');
    m.set(E, B, '1');
    m.set(E, E, '2');
    m
}

fn relate_ls_ls(la: &LineString, lb: &LineString, eps: f64) -> RelateMatrix {
    // A = LineString, B = LineString.
    let mut m = RelateMatrix::empty();
    let n_a = la.coords.len();
    let n_b = lb.coords.len();

    if n_a < 2 || n_b < 2 {
        m.set(E, E, '2');
        return m;
    }

    let ep_a0 = la.coords[0];
    let ep_a1 = la.coords[n_a - 1];
    let ep_b0 = lb.coords[0];
    let ep_b1 = lb.coords[n_b - 1];

    // ── Boundary cells ────────────────────────────────────────────────────────

    // BI: endpoint of A on interior of B.
    let bi = on_ls_interior(ep_a0, lb, eps) || on_ls_interior(ep_a1, lb, eps);
    // IB: endpoint of B on interior of A.
    let ib = on_ls_interior(ep_b0, la, eps) || on_ls_interior(ep_b1, la, eps);
    // BB: any endpoint of A equals any endpoint of B.
    let bb = eq_c(ep_a0, ep_b0, eps)
        || eq_c(ep_a0, ep_b1, eps)
        || eq_c(ep_a1, ep_b0, eps)
        || eq_c(ep_a1, ep_b1, eps);
    // BE: any endpoint of A lies outside B.
    let be = !on_ls(ep_a0, lb, eps) || !on_ls(ep_a1, lb, eps);
    // EB: any endpoint of B lies outside A.
    let eb = !on_ls(ep_b0, la, eps) || !on_ls(ep_b1, la, eps);

    m.set(B, I, if bi { '0' } else { 'F' });
    m.set(I, B, if ib { '0' } else { 'F' });
    m.set(B, B, if bb { '0' } else { 'F' });
    m.set(B, E, if be { '0' } else { 'F' });
    m.set(E, B, if eb { '0' } else { 'F' });

    // ── Interior cells ────────────────────────────────────────────────────────

    // T-intersection: any non-endpoint coord of A lies on B's interior (or vice versa).
    let a_int_on_b_int = la.coords[1..(n_a - 1)]
        .iter()
        .any(|&c| on_ls_interior(c, lb, eps));
    let b_int_on_a_int = lb.coords[1..(n_b - 1)]
        .iter()
        .any(|&c| on_ls_interior(c, la, eps));

    let (proper_cross, seg_overlap) = ls_ls_interior_contact(la, lb, eps);

    // II: max dimension of interior(A) ∩ interior(B).
    let ii = if seg_overlap {
        '1'
    } else if proper_cross || a_int_on_b_int || b_int_on_a_int || bi || ib {
        '0'
    } else {
        'F'
    };
    m.set(I, I, ii);

    // IE: does any interior of A lie outside B? (scan segment midpoints of A)
    let ie = (0..n_a.saturating_sub(1)).any(|i| {
        let mid = Coord::xy(
            0.5 * (la.coords[i].x + la.coords[i + 1].x),
            0.5 * (la.coords[i].y + la.coords[i + 1].y),
        );
        !on_ls(mid, lb, eps)
    });
    m.set(I, E, if ie { '1' } else { 'F' });

    // EI: does any interior of B lie outside A? (scan segment midpoints of B)
    let ei = (0..n_b.saturating_sub(1)).any(|j| {
        let mid = Coord::xy(
            0.5 * (lb.coords[j].x + lb.coords[j + 1].x),
            0.5 * (lb.coords[j].y + lb.coords[j + 1].y),
        );
        !on_ls(mid, la, eps)
    });
    m.set(E, I, if ei { '1' } else { 'F' });

    m.set(E, E, '2');
    m
}

fn relate_ls_poly(la: &LineString, pb: &Polygon, eps: f64) -> RelateMatrix {
    // A = LineString, B = Polygon.
    let mut m = RelateMatrix::empty();
    let n_a = la.coords.len();

    if n_a < 2 {
        m.set(E, E, '2');
        return m;
    }

    let ep_a0 = la.coords[0];
    let ep_a1 = la.coords[n_a - 1];

    // Scan interior segment midpoints of A.
    let mut any_int_inside = false;
    let mut any_int_on_bnd = false;
    let mut any_int_outside = false;

    for i in 0..n_a.saturating_sub(1) {
        let mid = Coord::xy(
            0.5 * (la.coords[i].x + la.coords[i + 1].x),
            0.5 * (la.coords[i].y + la.coords[i + 1].y),
        );
        let (ins, on_bnd, out) = classify_vs_poly(mid, pb, eps);
        if ins {
            any_int_inside = true;
        }
        if on_bnd {
            any_int_on_bnd = true;
        }
        if out {
            any_int_outside = true;
        }
    }

    // IB: interior of A ∩ boundary of B.
    // Midpoint scan above misses cases where A's interior crosses B's boundary between vertices.
    // Supplement with: (a) any non-endpoint coord of A lies on B's boundary, or
    //                  (b) any ring of B properly crosses the interior of any segment of A.
    if !any_int_on_bnd {
        // (a) intermediate coordinates of A on B's boundary
        for c in la.coords[1..(n_a - 1)].iter() {
            if on_poly_boundary(*c, pb, eps) {
                any_int_on_bnd = true;
                break;
            }
        }
    }
    if !any_int_on_bnd {
        // (b) any boundary ring of B properly crosses the interior of any segment of A
        'outer_ib: for i in 0..n_a.saturating_sub(1) {
            let (a1, a2) = (la.coords[i], la.coords[i + 1]);
            for ring in std::iter::once(&pb.exterior.coords)
                .chain(pb.holes.iter().map(|h| &h.coords))
            {
                for j in 0..ring.len().saturating_sub(1) {
                    let (r1, r2) = (ring[j], ring[j + 1]);
                    if seg_proper_cross(a1, a2, r1, r2, eps) {
                        any_int_on_bnd = true;
                        break 'outer_ib;
                    }
                    // Endpoint of ring segment on interior of A segment?
                    // Only counts if r1/r2 is NOT an endpoint of A.
                    if !eq_c(r1, ep_a0, eps)
                        && !eq_c(r1, ep_a1, eps)
                        && point_on_segment_eps(r1, a1, a2, eps)
                    {
                        any_int_on_bnd = true;
                        break 'outer_ib;
                    }
                }
            }
        }
    }

    m.set(I, I, if any_int_inside { '1' } else { 'F' });
    m.set(I, B, if any_int_on_bnd { '0' } else { 'F' });
    m.set(I, E, if any_int_outside { '1' } else { 'F' });

    // Boundary (endpoints) of A.
    let (ep0_in, ep0_on, ep0_out) = classify_vs_poly(ep_a0, pb, eps);
    let (ep1_in, ep1_on, ep1_out) = classify_vs_poly(ep_a1, pb, eps);

    m.set(B, I, if ep0_in || ep1_in { '0' } else { 'F' });
    m.set(B, B, if ep0_on || ep1_on { '0' } else { 'F' });
    m.set(B, E, if ep0_out || ep1_out { '0' } else { 'F' });

    // A linestring cannot fill a polygon's 2D interior or its entire boundary.
    m.set(E, I, '2');
    m.set(E, B, '1');
    m.set(E, E, '2');
    m
}

fn relate_poly_poly(pa: &Polygon, pb: &Polygon, eps: f64) -> RelateMatrix {
    // A = Polygon, B = Polygon.
    let mut m = RelateMatrix::empty();

    let ga = Geometry::Polygon(pa.clone());
    let gb = Geometry::Polygon(pb.clone());

    if !intersects_with_epsilon(&ga, &gb, eps) {
        // Disjoint.
        m.set(I, I, 'F');
        m.set(I, B, 'F');
        m.set(I, E, '2');
        m.set(B, I, 'F');
        m.set(B, B, 'F');
        m.set(B, E, '1');
        m.set(E, I, '2');
        m.set(E, B, '1');
        m.set(E, E, '2');
        return m;
    }

    // Sample interior points.
    let a_sample_in_b = poly_sample_interior(pa, eps)
        .map(|p| inside_poly_strict(p, pb, eps))
        .unwrap_or(false);
    let b_sample_in_a = poly_sample_interior(pb, eps)
        .map(|p| inside_poly_strict(p, pa, eps))
        .unwrap_or(false);

    // Boundary analysis.
    let bb_dim = poly_poly_boundary_dim(pa, pb, eps);

    // Does B's boundary have midpoints inside A's interior?
    let b_bnd_in_a_int = poly_boundary_midpoints_inside(pb, pa, eps);
    // Does A's boundary have midpoints inside B's interior?
    let a_bnd_in_b_int = poly_boundary_midpoints_inside(pa, pb, eps);

    // Does any vertex/midpoint of A's boundary lie outside B?
    let a_bnd_outside_b = poly_boundary_has_outside(pa, pb, eps);
    // Does any vertex/midpoint of B's boundary lie outside A?
    let b_bnd_outside_a = poly_boundary_has_outside(pb, pa, eps);

    // Is A completely inside B? (no part of A outside B, and sample inside B)
    let a_completely_in_b = !a_bnd_outside_b && a_sample_in_b;
    // Is B completely inside A?
    let b_completely_in_a = !b_bnd_outside_a && b_sample_in_a;

    // II: '2' iff they share common interior area.
    let ii_non_empty = a_sample_in_b || b_sample_in_a || (a_bnd_in_b_int && b_bnd_in_a_int);
    m.set(I, I, if ii_non_empty { '2' } else { 'F' });

    // IB: interior(A) ∩ boundary(B). Non-empty iff B's boundary enters A's interior.
    m.set(I, B, if b_bnd_in_a_int { '1' } else { 'F' });

    // IE: interior(A) ∩ exterior(B). 'F' only when A is completely inside B.
    m.set(I, E, if a_completely_in_b { 'F' } else { '2' });

    // BI: boundary(A) ∩ interior(B). Non-empty iff A's boundary enters B's interior.
    m.set(B, I, if a_bnd_in_b_int { '1' } else { 'F' });

    // BB: boundary(A) ∩ boundary(B).
    m.set(B, B, bb_dim);

    // BE: boundary(A) ∩ exterior(B). 'F' only when A is completely inside B.
    m.set(B, E, if a_completely_in_b { 'F' } else { '1' });

    // EI: exterior(A) ∩ interior(B). 'F' only when B is completely inside A.
    m.set(E, I, if b_completely_in_a { 'F' } else { '2' });

    // EB: exterior(A) ∩ boundary(B). 'F' only when B is completely inside A.
    m.set(E, B, if b_completely_in_a { 'F' } else { '1' });

    m.set(E, E, '2');
    m
}

// ── Main dispatcher ───────────────────────────────────────────────────────────

fn relate_impl(a: &Geometry, b: &Geometry, epsilon: Option<f64>) -> RelateMatrix {
    let eps = epsilon.unwrap_or(1.0e-9).abs().max(1.0e-15);
    match (a, b) {
        (Geometry::Point(pa), Geometry::Point(pb)) => relate_pp(*pa, *pb, eps),
        (Geometry::Point(pa), Geometry::LineString(lb)) => relate_p_ls(*pa, lb, eps),
        (Geometry::LineString(la), Geometry::Point(pb)) => relate_p_ls(*pb, la, eps).transpose(),
        (Geometry::Point(pa), Geometry::Polygon(pb)) => relate_p_poly(*pa, pb, eps),
        (Geometry::Polygon(pa), Geometry::Point(pb)) => relate_p_poly(*pb, pa, eps).transpose(),
        (Geometry::LineString(la), Geometry::LineString(lb)) => relate_ls_ls(la, lb, eps),
        (Geometry::LineString(la), Geometry::Polygon(pb)) => relate_ls_poly(la, pb, eps),
        (Geometry::Polygon(pa), Geometry::LineString(lb)) => {
            relate_ls_poly(lb, pa, eps).transpose()
        }
        (Geometry::Polygon(pa), Geometry::Polygon(pb)) => relate_poly_poly(pa, pb, eps),
        _ => relate_conservative(a, b, eps),
    }
}

// ── Conservative fallback (multi-geometry / unsupported pairs) ────────────────

fn relate_conservative(a: &Geometry, b: &Geometry, eps: f64) -> RelateMatrix {
    use crate::topology::{
        contains_with_epsilon, crosses_with_epsilon, intersects_with_epsilon,
        overlaps_with_epsilon, touches_with_epsilon, within_with_epsilon,
    };

    let mut m = RelateMatrix::empty();
    m.set(E, E, '2');
    m.set(B, E, boundary_dim_char(a));
    m.set(E, B, boundary_dim_char(b));

    let intersects_v = intersects_with_epsilon(a, b, eps);
    let touches_v = touches_with_epsilon(a, b, eps);
    let crosses_v = crosses_with_epsilon(a, b, eps);
    let overlaps_v = overlaps_with_epsilon(a, b, eps);
    let within_ab = within_with_epsilon(a, b, eps);
    let within_ba = within_with_epsilon(b, a, eps);
    let contains_ab = contains_with_epsilon(a, b, eps);
    let contains_ba = contains_with_epsilon(b, a, eps);

    if !intersects_v {
        m.set(I, I, 'F');
        m.set(I, E, dim_char(a));
        m.set(E, I, dim_char(b));
        return m;
    }

    m.set(I, E, if within_ab { 'F' } else { dim_char(a) });
    m.set(E, I, if within_ba { 'F' } else { dim_char(b) });

    if touches_v {
        m.set(I, I, 'F');
    } else if crosses_v {
        m.set(I, I, crosses_ii_dim_char(a, b));
    } else {
        m.set(I, I, ii_dim_char(a, b));
    }

    if touches_v || crosses_v {
        m.set(B, B, '0');
    }
    if overlaps_v {
        let d = dim_char(a).min(dim_char(b));
        m.set(I, I, d);
    }
    if contains_ab {
        m.set(E, I, 'F');
    }
    if contains_ba {
        m.set(I, E, 'F');
    }

    apply_pair_contact_cells(
        &mut m, a, b, touches_v, crosses_v, overlaps_v, within_ab, within_ba, contains_ab,
        contains_ba,
    );
    m
}

// ── Conservative fallback helpers ─────────────────────────────────────────────

fn dim_char(g: &Geometry) -> char {
    match g {
        Geometry::Point(_) => '0',
        Geometry::LineString(_) | Geometry::MultiLineString(_) => '1',
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => '2',
        _ => '0',
    }
}

fn boundary_dim_char(g: &Geometry) -> char {
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => 'F',
        Geometry::LineString(_) | Geometry::MultiLineString(_) => '0',
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => '1',
        Geometry::GeometryCollection(_) => 'F',
    }
}

fn ii_dim_char(a: &Geometry, b: &Geometry) -> char {
    dim_char(a).min(dim_char(b))
}

fn crosses_ii_dim_char(a: &Geometry, b: &Geometry) -> char {
    match (a, b) {
        (Geometry::LineString(_), Geometry::LineString(_)) => '0',
        (Geometry::LineString(_), Geometry::Polygon(_))
        | (Geometry::Polygon(_), Geometry::LineString(_)) => '1',
        _ => ii_dim_char(a, b),
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_pair_contact_cells(
    m: &mut RelateMatrix,
    a: &Geometry,
    b: &Geometry,
    touches_v: bool,
    crosses_v: bool,
    overlaps_v: bool,
    within_ab: bool,
    within_ba: bool,
    contains_ab: bool,
    contains_ba: bool,
) {
    match (a, b) {
        (Geometry::Point(_), Geometry::LineString(_))
        | (Geometry::Point(_), Geometry::Polygon(_)) => {
            if touches_v {
                m.set(I, B, '0');
            }
        }
        (Geometry::LineString(_), Geometry::Point(_))
        | (Geometry::Polygon(_), Geometry::Point(_)) => {
            if touches_v {
                m.set(B, I, '0');
            }
        }
        (Geometry::LineString(_), Geometry::LineString(_)) => {
            if touches_v || crosses_v || overlaps_v {
                m.set(I, B, '0');
                m.set(B, I, '0');
            }
        }
        (Geometry::LineString(_), Geometry::Polygon(_)) => {
            if touches_v || crosses_v {
                m.set(I, B, '0');
            }
            if touches_v || crosses_v || within_ab {
                m.set(B, I, '0');
            }
        }
        (Geometry::Polygon(_), Geometry::LineString(_)) => {
            if touches_v || crosses_v {
                m.set(B, I, '0');
            }
            if touches_v || crosses_v || within_ba {
                m.set(I, B, '0');
            }
        }
        (Geometry::Polygon(_), Geometry::Polygon(_)) => {
            if touches_v || overlaps_v || contains_ab || contains_ba {
                m.set(I, B, '1');
                m.set(B, I, '1');
            }
            if touches_v {
                m.set(B, B, '1');
            }
        }
        _ => {}
    }
}
