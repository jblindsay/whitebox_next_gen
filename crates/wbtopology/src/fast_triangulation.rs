//! Fast Delaunay triangulation utilities.
//!
//! This module implements a fast sweep-hull style triangulator adapted from
//! the Delaunator family of algorithms.
//!
//! Attribution:
//! - Mapbox Delaunator (JavaScript): https://github.com/mapbox/delaunator
//! - mourner/delaunator-rs (Rust port): https://github.com/mourner/delaunator-rs
//!
//! The implementation here is an in-crate adaptation that returns
//! [`crate::triangulation::DelaunayTriangulation`].

use std::cmp::Ordering;

use crate::geom::Coord;
use crate::triangulation::DelaunayTriangulation;

const EMPTY: usize = usize::MAX;

#[derive(Clone, Copy)]
struct Point2 {
    x: f64,
    y: f64,
}

impl Point2 {
    #[inline]
    fn dist2(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    #[inline]
    fn orient(self, q: Self, r: Self, eps: f64) -> bool {
        orient2d(self, q, r) >= -eps
    }

    #[inline]
    fn circumradius2(self, b: Self, c: Self) -> f64 {
        let d = 2.0 * (self.x * (b.y - c.y) + b.x * (c.y - self.y) + c.x * (self.y - b.y));
        if d.abs() <= 1.0e-30 {
            return f64::INFINITY;
        }
        let a2 = self.x * self.x + self.y * self.y;
        let b2 = b.x * b.x + b.y * b.y;
        let c2 = c.x * c.x + c.y * c.y;
        let ux = (a2 * (b.y - c.y) + b2 * (c.y - self.y) + c2 * (self.y - b.y)) / d;
        let uy = (a2 * (c.x - b.x) + b2 * (self.x - c.x) + c2 * (b.x - self.x)) / d;
        let dx = ux - self.x;
        let dy = uy - self.y;
        dx * dx + dy * dy
    }

    #[inline]
    fn circumcenter(self, b: Self, c: Self) -> Self {
        let d = 2.0 * (self.x * (b.y - c.y) + b.x * (c.y - self.y) + c.x * (self.y - b.y));
        if d.abs() <= 1.0e-30 {
            return Self { x: f64::NAN, y: f64::NAN };
        }
        let a2 = self.x * self.x + self.y * self.y;
        let b2 = b.x * b.x + b.y * b.y;
        let c2 = c.x * c.x + c.y * c.y;
        let ux = (a2 * (b.y - c.y) + b2 * (c.y - self.y) + c2 * (self.y - b.y)) / d;
        let uy = (a2 * (c.x - b.x) + b2 * (self.x - c.x) + c2 * (b.x - self.x)) / d;
        Self { x: ux, y: uy }
    }

    #[inline]
    fn in_circle(self, b: Self, c: Self, p: Self) -> bool {
        let ax = self.x - p.x;
        let ay = self.y - p.y;
        let bx = b.x - p.x;
        let by = b.y - p.y;
        let cx = c.x - p.x;
        let cy = c.y - p.y;

        let det = (ax * ax + ay * ay) * (bx * cy - by * cx)
            - (bx * bx + by * by) * (ax * cy - ay * cx)
            + (cx * cx + cy * cy) * (ax * by - ay * bx);
        det > 0.0
    }
}

#[inline]
fn orient2d(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

struct Hull {
    prev: Vec<usize>,
    next: Vec<usize>,
    tri: Vec<usize>,
    hash: Vec<usize>,
    start: usize,
    center: Point2,
}

impl Hull {
    fn new(n: usize, center: Point2, i0: usize, i1: usize, i2: usize, points: &[Point2]) -> Self {
        let hash_len = (n as f64).sqrt().max(4.0) as usize;
        let mut hull = Self {
            prev: vec![0; n],
            next: vec![0; n],
            tri: vec![0; n],
            hash: vec![EMPTY; hash_len],
            start: i0,
            center,
        };

        hull.next[i0] = i1;
        hull.prev[i2] = i1;
        hull.next[i1] = i2;
        hull.prev[i0] = i2;
        hull.next[i2] = i0;
        hull.prev[i1] = i0;

        hull.tri[i0] = 0;
        hull.tri[i1] = 1;
        hull.tri[i2] = 2;

        hull.hash_edge(points[i0], i0);
        hull.hash_edge(points[i1], i1);
        hull.hash_edge(points[i2], i2);

        hull
    }

    #[inline]
    fn hash_key(&self, p: Point2) -> usize {
        let dx = p.x - self.center.x;
        let dy = p.y - self.center.y;
        let q = dx / (dx.abs() + dy.abs()).max(1.0e-30);
        let a = if dy > 0.0 { (3.0 - q) / 4.0 } else { (1.0 + q) / 4.0 };
        ((self.hash.len() as f64 * a).floor() as usize) % self.hash.len()
    }

    #[inline]
    fn hash_edge(&mut self, p: Point2, i: usize) {
        let key = self.hash_key(p);
        self.hash[key] = i;
    }

    fn find_visible_edge(&self, p: Point2, points: &[Point2], eps: f64) -> (usize, bool) {
        let mut start = EMPTY;
        let key = self.hash_key(p);
        for j in 0..self.hash.len() {
            start = self.hash[(key + j) % self.hash.len()];
            if start != EMPTY && self.next[start] != EMPTY {
                break;
            }
        }
        if start == EMPTY {
            return (EMPTY, false);
        }
        start = self.prev[start];
        let mut e = start;
        while !p.orient(points[e], points[self.next[e]], eps) {
            e = self.next[e];
            if e == start {
                return (EMPTY, false);
            }
        }
        (e, e == start)
    }
}

struct FastTriangulation {
    triangles: Vec<usize>,
    halfedges: Vec<usize>,
    hull: Vec<usize>,
}

impl FastTriangulation {
    fn new(n: usize) -> Self {
        let max_triangles = 2 * n.saturating_sub(5);
        Self {
            triangles: Vec::with_capacity(max_triangles * 3),
            halfedges: Vec::with_capacity(max_triangles * 3),
            hull: Vec::new(),
        }
    }

    #[inline]
    fn next_halfedge(edge: usize) -> usize {
        if edge % 3 == 2 { edge - 2 } else { edge + 1 }
    }

    #[inline]
    fn prev_halfedge(edge: usize) -> usize {
        if edge % 3 == 0 { edge + 2 } else { edge - 1 }
    }

    fn add_triangle(&mut self, i0: usize, i1: usize, i2: usize, a: usize, b: usize, c: usize) -> usize {
        let t = self.triangles.len();
        self.triangles.push(i0);
        self.triangles.push(i1);
        self.triangles.push(i2);
        self.halfedges.push(a);
        self.halfedges.push(b);
        self.halfedges.push(c);
        if a != EMPTY { self.halfedges[a] = t; }
        if b != EMPTY { self.halfedges[b] = t + 1; }
        if c != EMPTY { self.halfedges[c] = t + 2; }
        t
    }

    fn legalize(&mut self, a: usize, points: &[Point2], hull: &mut Hull) -> usize {
        let b = self.halfedges[a];
        let ar = Self::prev_halfedge(a);
        if b == EMPTY {
            return ar;
        }
        let al = Self::next_halfedge(a);
        let bl = Self::prev_halfedge(b);

        let p0 = self.triangles[ar];
        let pr = self.triangles[a];
        let pl = self.triangles[al];
        let p1 = self.triangles[bl];

        if points[p0].in_circle(points[pr], points[pl], points[p1]) {
            self.triangles[a] = p1;
            self.triangles[b] = p0;

            let hbl = self.halfedges[bl];
            let har = self.halfedges[ar];

            if hbl == EMPTY {
                let mut e = hull.start;
                loop {
                    if hull.tri[e] == bl {
                        hull.tri[e] = a;
                        break;
                    }
                    e = hull.next[e];
                    if e == hull.start || e == EMPTY {
                        break;
                    }
                }
            }

            self.halfedges[a] = hbl;
            self.halfedges[b] = har;
            self.halfedges[ar] = bl;

            if hbl != EMPTY { self.halfedges[hbl] = a; }
            if har != EMPTY { self.halfedges[har] = b; }
            if bl != EMPTY { self.halfedges[bl] = ar; }

            let br = Self::next_halfedge(b);
            self.legalize(a, points, hull);
            return self.legalize(br, points, hull);
        }
        ar
    }
}

fn calc_bbox_center(points: &[Point2]) -> Point2 {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Point2 { x: (min_x + max_x) * 0.5, y: (min_y + max_y) * 0.5 }
}

fn find_closest_point(points: &[Point2], p0: Point2) -> Option<usize> {
    let mut min_dist = f64::INFINITY;
    let mut idx = 0usize;
    for (i, p) in points.iter().enumerate() {
        let d = p0.dist2(*p);
        if d > 0.0 && d < min_dist {
            min_dist = d;
            idx = i;
        }
    }
    if min_dist.is_finite() { Some(idx) } else { None }
}

fn find_seed_triangle(points: &[Point2]) -> Option<(usize, usize, usize)> {
    let center = calc_bbox_center(points);
    let i0 = find_closest_point(points, center)?;
    let p0 = points[i0];
    let i1 = find_closest_point(points, p0)?;
    let p1 = points[i1];

    let mut min_radius = f64::INFINITY;
    let mut i2 = 0usize;
    for (i, p) in points.iter().enumerate() {
        if i == i0 || i == i1 {
            continue;
        }
        let r = p0.circumradius2(p1, *p);
        if r < min_radius {
            min_radius = r;
            i2 = i;
        }
    }
    if !min_radius.is_finite() {
        return None;
    }
    Some(if orient2d(p0, p1, points[i2]) > 0.0 {
        (i0, i2, i1)
    } else {
        (i0, i1, i2)
    })
}

fn triangulate_fast_indices(points: &[Point2], epsilon: f64) -> Option<FastTriangulation> {
    let n = points.len();
    let (i0, i1, i2) = find_seed_triangle(points)?;
    let center = points[i0].circumcenter(points[i1], points[i2]);

    let mut tri = FastTriangulation::new(n);
    tri.add_triangle(i0, i1, i2, EMPTY, EMPTY, EMPTY);

    let mut dists: Vec<(usize, f64)> = points
        .iter()
        .enumerate()
        .map(|(i, p)| (i, center.dist2(*p)))
        .collect();
    dists.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

    let mut hull = Hull::new(n, center, i0, i1, i2, points);

    for (k, &(i, _)) in dists.iter().enumerate() {
        let p = points[i];
        if k > 0 {
            let prev = points[dists[k - 1].0];
            if (p.x - prev.x).abs() <= epsilon && (p.y - prev.y).abs() <= epsilon {
                continue;
            }
        }
        if i == i0 || i == i1 || i == i2 {
            continue;
        }

        let (mut e, walk_back) = hull.find_visible_edge(p, points, epsilon);
        if e == EMPTY {
            continue;
        }

        let t = tri.add_triangle(e, i, hull.next[e], EMPTY, EMPTY, hull.tri[e]);
        hull.tri[i] = tri.legalize(t + 2, points, &mut hull);
        hull.tri[e] = t;

        let mut n_edge = hull.next[e];
        loop {
            let q = hull.next[n_edge];
            if !p.orient(points[n_edge], points[q], epsilon) {
                break;
            }
            let t2 = tri.add_triangle(n_edge, i, q, hull.tri[i], EMPTY, hull.tri[n_edge]);
            hull.tri[i] = tri.legalize(t2 + 2, points, &mut hull);
            hull.next[n_edge] = EMPTY;
            n_edge = q;
        }

        if walk_back {
            loop {
                let q = hull.prev[e];
                if !p.orient(points[q], points[e], epsilon) {
                    break;
                }
                let t2 = tri.add_triangle(q, i, e, EMPTY, hull.tri[e], hull.tri[q]);
                tri.legalize(t2 + 2, points, &mut hull);
                hull.tri[q] = t2;
                hull.next[e] = EMPTY;
                e = q;
            }
        }

        hull.prev[i] = e;
        hull.next[i] = n_edge;
        hull.prev[n_edge] = i;
        hull.next[e] = i;
        hull.start = e;

        hull.hash_edge(p, i);
        hull.hash_edge(points[e], e);
    }

    let mut e = hull.start;
    loop {
        tri.hull.push(e);
        e = hull.next[e];
        if e == hull.start {
            break;
        }
    }

    tri.triangles.shrink_to_fit();
    tri.halfedges.shrink_to_fit();
    Some(tri)
}

/// Build a fast 2D Delaunay triangulation using a sweep-hull strategy.
///
/// This routine is intended for high-throughput workflows (for example large
/// LiDAR TIN gridding). It keeps the `Coord.z` value from the input points and
/// returns triangle indices into the returned point array.
pub fn delaunay_triangulation_fast(points: &[Coord], epsilon: f64) -> DelaunayTriangulation {
    let eps = if epsilon.is_finite() { epsilon.abs().max(1.0e-12) } else { 1.0e-12 };
    let filtered: Vec<Coord> = points
        .iter()
        .copied()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    if filtered.len() < 3 {
        return DelaunayTriangulation { points: filtered, triangles: vec![] };
    }

    let points2d: Vec<Point2> = filtered.iter().map(|p| Point2 { x: p.x, y: p.y }).collect();
    let Some(work) = triangulate_fast_indices(&points2d, eps) else {
        return DelaunayTriangulation { points: filtered, triangles: vec![] };
    };

    let mut triangles = Vec::with_capacity(work.triangles.len() / 3);
    for tri in work.triangles.chunks_exact(3) {
        triangles.push([tri[0], tri[1], tri[2]]);
    }

    DelaunayTriangulation {
        points: filtered,
        triangles,
    }
}