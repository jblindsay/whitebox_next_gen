//! Delaunay triangulation utilities.
//!
//! The implementation uses an incremental Bowyer-Watson strategy with cached
//! circumcircles and in-place cavity updates.

use std::collections::{HashMap, HashSet};

use crate::algorithms::orientation::orient2d_sign;
use crate::error::{Result, TopologyError};
use crate::geom::Coord;
use crate::precision::PrecisionModel;

const TRI_INDEX_GRID_SIZE: usize = 64;
const TRI_INDEX_MAX_CELLS_PER_TRI: usize = 128;

/// Configuration options for Delaunay triangulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangulationOptions {
    /// Epsilon used for deduplication and robust predicate tolerance.
    pub epsilon: f64,
    /// Optional precision snapping applied before triangulation.
    pub precision: Option<PrecisionModel>,
}

impl Default for TriangulationOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.0e-12,
            precision: None,
        }
    }
}

/// Delaunay triangulation result over a unique point set.
#[derive(Debug, Clone, PartialEq)]
pub struct DelaunayTriangulation {
    /// Unique points used by triangulation, after epsilon-based deduplication.
    pub points: Vec<Coord>,
    /// Triangle vertex indices into [`Self::points`].
    pub triangles: Vec<[usize; 3]>,
}

#[derive(Debug, Clone)]
struct Triangle {
    v: [usize; 3],
    center: Coord,
    radius2: f64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

struct TriangleCandidateIndex {
    min_x: f64,
    min_y: f64,
    inv_dx: f64,
    inv_dy: f64,
    nx: usize,
    ny: usize,
    bins: Vec<Vec<usize>>,
    global: Vec<usize>,
}

impl TriangleCandidateIndex {
    fn new(points: &[Coord]) -> Self {
        let mut min_x = points[0].x;
        let mut min_y = points[0].y;
        let mut max_x = points[0].x;
        let mut max_y = points[0].y;

        for &p in &points[1..] {
            if p.x < min_x {
                min_x = p.x;
            }
            if p.x > max_x {
                max_x = p.x;
            }
            if p.y < min_y {
                min_y = p.y;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }

        let dx = (max_x - min_x).max(1.0e-9);
        let dy = (max_y - min_y).max(1.0e-9);
        let nx = TRI_INDEX_GRID_SIZE;
        let ny = TRI_INDEX_GRID_SIZE;

        Self {
            min_x,
            min_y,
            inv_dx: 1.0 / dx,
            inv_dy: 1.0 / dy,
            nx,
            ny,
            bins: vec![Vec::new(); nx * ny],
            global: Vec::new(),
        }
    }

    fn clear(&mut self) {
        for b in &mut self.bins {
            b.clear();
        }
        self.global.clear();
    }

    fn rebuild(&mut self, triangles: &[Triangle], alive: &[bool]) {
        self.clear();
        for (tri_id, tri) in triangles.iter().enumerate() {
            if alive[tri_id] {
                self.insert_triangle(tri_id, tri);
            }
        }
    }

    fn insert_triangle(&mut self, tri_id: usize, tri: &Triangle) {
        let Some((ix0, ix1, iy0, iy1)) =
            self.bbox_range(tri.min_x, tri.max_x, tri.min_y, tri.max_y)
        else {
            self.global.push(tri_id);
            return;
        };

        let nx = ix1 - ix0 + 1;
        let ny = iy1 - iy0 + 1;
        if nx * ny > TRI_INDEX_MAX_CELLS_PER_TRI {
            self.global.push(tri_id);
            return;
        }

        for iy in iy0..=iy1 {
            let row = iy * self.nx;
            for ix in ix0..=ix1 {
                self.bins[row + ix].push(tri_id);
            }
        }
    }

    fn collect_candidates(&self, p: Coord, seen: &mut [u32], stamp: u32, out: &mut Vec<usize>) {
        out.clear();

        for &tri_id in &self.global {
            if seen[tri_id] != stamp {
                seen[tri_id] = stamp;
                out.push(tri_id);
            }
        }

        let Some((ix, iy)) = self.point_cell(p) else {
            return;
        };
        let cell = iy * self.nx + ix;
        for &tri_id in &self.bins[cell] {
            if seen[tri_id] != stamp {
                seen[tri_id] = stamp;
                out.push(tri_id);
            }
        }
    }

    fn bbox_range(
        &self,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    ) -> Option<(usize, usize, usize, usize)> {
        if max_x < self.min_x || max_y < self.min_y {
            return None;
        }

        let fx0 = ((min_x - self.min_x) * self.inv_dx).clamp(0.0, 1.0);
        let fx1 = ((max_x - self.min_x) * self.inv_dx).clamp(0.0, 1.0);
        let fy0 = ((min_y - self.min_y) * self.inv_dy).clamp(0.0, 1.0);
        let fy1 = ((max_y - self.min_y) * self.inv_dy).clamp(0.0, 1.0);

        let ix0 = ((fx0 * (self.nx as f64 - 1.0)).floor() as usize).min(self.nx - 1);
        let ix1 = ((fx1 * (self.nx as f64 - 1.0)).floor() as usize).min(self.nx - 1);
        let iy0 = ((fy0 * (self.ny as f64 - 1.0)).floor() as usize).min(self.ny - 1);
        let iy1 = ((fy1 * (self.ny as f64 - 1.0)).floor() as usize).min(self.ny - 1);
        Some((ix0.min(ix1), ix0.max(ix1), iy0.min(iy1), iy0.max(iy1)))
    }

    fn point_cell(&self, p: Coord) -> Option<(usize, usize)> {
        let fx = ((p.x - self.min_x) * self.inv_dx).clamp(0.0, 1.0);
        let fy = ((p.y - self.min_y) * self.inv_dy).clamp(0.0, 1.0);
        if !fx.is_finite() || !fy.is_finite() {
            return None;
        }
        let ix = ((fx * (self.nx as f64 - 1.0)).floor() as usize).min(self.nx - 1);
        let iy = ((fy * (self.ny as f64 - 1.0)).floor() as usize).min(self.ny - 1);
        Some((ix, iy))
    }
}

/// Build a 2D Delaunay triangulation from input points.
///
/// Duplicate/near-duplicate points are collapsed using `epsilon` to improve
/// robustness and avoid zero-area triangles.
pub fn delaunay_triangulation(points: &[Coord], epsilon: f64) -> DelaunayTriangulation {
    let eps = normalized_eps(epsilon);
    let mut unique = dedup_points(points, eps);
    spatially_order_points(&mut unique);

    if unique.len() < 3 {
        return DelaunayTriangulation {
            points: unique,
            triangles: vec![],
        };
    }

    let mut all_points = unique.clone();
    let super_start = all_points.len();
    let [s0, s1, s2] = build_super_triangle(&unique);
    all_points.push(s0);
    all_points.push(s1);
    all_points.push(s2);

    let mut triangles = Vec::<Triangle>::with_capacity(unique.len() * 2 + 8);
    let mut alive = Vec::<bool>::with_capacity(unique.len() * 2 + 8);
    if let Some(t) = make_triangle(super_start, super_start + 1, super_start + 2, &all_points, eps)
    {
        triangles.push(t);
        alive.push(true);
    }

    let mut tri_index = TriangleCandidateIndex::new(&unique);
    if !triangles.is_empty() {
        tri_index.insert_triangle(0, &triangles[0]);
    }

    let mut bad = Vec::<usize>::new();
    let mut cavity_edges = Vec::<u128>::new();
    let mut candidates = Vec::<usize>::new();
    let mut seen_stamp = vec![0u32; triangles.len().max(1)];
    let mut stamp = 1u32;
    let mut live_count = triangles.len();
    let mut dead_since_rebuild = 0usize;

    for p_idx in 0..unique.len() {
        let p = all_points[p_idx];
        bad.clear();

        if stamp == u32::MAX {
            seen_stamp.fill(0);
            stamp = 1;
        }

        tri_index.collect_candidates(p, &mut seen_stamp, stamp, &mut candidates);
        stamp = stamp.wrapping_add(1);

        for &tri_idx in &candidates {
            if tri_idx >= triangles.len() || !alive[tri_idx] {
                continue;
            }
            if point_in_circumcircle(p, &triangles[tri_idx], eps) {
                bad.push(tri_idx);
            }
        }

        if bad.is_empty() {
            continue;
        }

        cavity_edges.clear();
        cavity_edges.reserve(bad.len() * 3);
        for &tri_idx in &bad {
            let tri = &triangles[tri_idx];
            for (a, b) in tri_edges(tri.v) {
                cavity_edges.push(pack_edge(a, b));
            }
        }
        cavity_edges.sort_unstable();

        bad.sort_unstable();
        for &tri_idx in &bad {
            if alive[tri_idx] {
                alive[tri_idx] = false;
                dead_since_rebuild += 1;
                live_count = live_count.saturating_sub(1);
            }
        }

        let mut i = 0usize;
        while i < cavity_edges.len() {
            let edge = cavity_edges[i];
            let mut j = i + 1;
            while j < cavity_edges.len() && cavity_edges[j] == edge {
                j += 1;
            }
            if j == i + 1 {
                let (a, b) = unpack_edge(edge);
                if let Some(new_tri) = make_triangle(a, b, p_idx, &all_points, eps) {
                    let tri_id = triangles.len();
                    triangles.push(new_tri);
                    alive.push(true);
                    seen_stamp.push(0);
                    tri_index.insert_triangle(tri_id, &triangles[tri_id]);
                    live_count += 1;
                }
            }
            i = j;
        }

        if dead_since_rebuild > live_count && dead_since_rebuild > 1024 {
            tri_index.rebuild(&triangles, &alive);
            dead_since_rebuild = 0;
        }
    }

    let triangles = triangles
        .into_iter()
        .enumerate()
        .filter_map(|(idx, t)| {
            if !alive[idx] {
                return None;
            }
            if t.v[0] >= super_start || t.v[1] >= super_start || t.v[2] >= super_start {
                None
            } else {
                Some(t.v)
            }
        })
        .collect();

    DelaunayTriangulation {
        points: unique,
        triangles,
    }
}

/// Build a Delaunay triangulation after snapping points to a precision model.
pub fn delaunay_triangulation_with_precision(
    points: &[Coord],
    precision: PrecisionModel,
) -> DelaunayTriangulation {
    let mut snapped = points.to_vec();
    precision.apply_coords_in_place(&mut snapped);
    delaunay_triangulation(&snapped, precision.epsilon())
}

/// Build a Delaunay triangulation using advanced options.
pub fn delaunay_triangulation_with_options(
    points: &[Coord],
    options: TriangulationOptions,
) -> DelaunayTriangulation {
    if let Some(pm) = options.precision {
        let mut snapped = points.to_vec();
        pm.apply_coords_in_place(&mut snapped);
        return delaunay_triangulation(&snapped, options.epsilon.max(pm.epsilon()));
    }
    delaunay_triangulation(points, options.epsilon)
}

/// Build a Delaunay triangulation and enforce that constraint edges exist.
///
/// Constraint edges are provided as coordinate pairs and validated against the
/// undirected edge set of the resulting triangulation under `epsilon`.
pub fn delaunay_triangulation_with_constraints(
    points: &[Coord],
    constraints: &[(Coord, Coord)],
    epsilon: f64,
) -> Result<DelaunayTriangulation> {
    let tri = delaunay_triangulation(points, epsilon);
    let missing = find_missing_constraints(&tri, constraints, normalized_eps(epsilon));
    if missing.is_empty() {
        Ok(tri)
    } else {
        Err(TopologyError::InvalidGeometry(format!(
            "{} constraint edge(s) are not present in triangulation",
            missing.len()
        )))
    }
}

/// Options-based constrained triangulation variant with enforcement checks.
pub fn delaunay_triangulation_with_options_checked(
    points: &[Coord],
    options: TriangulationOptions,
    constraints: &[(Coord, Coord)],
) -> Result<DelaunayTriangulation> {
    let tri = delaunay_triangulation_with_options(points, options);
    let eps = if let Some(pm) = options.precision {
        options.epsilon.max(pm.epsilon())
    } else {
        options.epsilon
    };
    let missing = find_missing_constraints(&tri, constraints, normalized_eps(eps));
    if missing.is_empty() {
        Ok(tri)
    } else {
        Err(TopologyError::InvalidGeometry(format!(
            "{} constraint edge(s) are not present in triangulation",
            missing.len()
        )))
    }
}

#[inline]
fn tri_edges(v: [usize; 3]) -> [(usize, usize); 3] {
    [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])]
}

#[inline]
fn pack_edge(a: usize, b: usize) -> u128 {
    let (lo, hi) = if a < b {
        (a as u128, b as u128)
    } else {
        (b as u128, a as u128)
    };
    (hi << 64) | lo
}

#[inline]
fn unpack_edge(key: u128) -> (usize, usize) {
    let lo = (key & 0xFFFF_FFFF_FFFF_FFFF) as usize;
    let hi = (key >> 64) as usize;
    (lo, hi)
}

#[inline]
fn pack_cell_key(x: i64, y: i64) -> u128 {
    ((x as u64 as u128) << 64) | (y as u64 as u128)
}

fn edge_key_from_coords(a: Coord, b: Coord, eps: f64) -> ((i64, i64), (i64, i64)) {
    let inv = 1.0 / eps;
    let qa = ((a.x * inv).round() as i64, (a.y * inv).round() as i64);
    let qb = ((b.x * inv).round() as i64, (b.y * inv).round() as i64);
    if qa <= qb {
        (qa, qb)
    } else {
        (qb, qa)
    }
}

fn find_missing_constraints(
    tri: &DelaunayTriangulation,
    constraints: &[(Coord, Coord)],
    eps: f64,
) -> Vec<(Coord, Coord)> {
    let mut edge_set = HashSet::<((i64, i64), (i64, i64))>::new();
    for t in &tri.triangles {
        let a = tri.points[t[0]];
        let b = tri.points[t[1]];
        let c = tri.points[t[2]];
        edge_set.insert(edge_key_from_coords(a, b, eps));
        edge_set.insert(edge_key_from_coords(b, c, eps));
        edge_set.insert(edge_key_from_coords(c, a, eps));
    }

    let mut missing = Vec::<(Coord, Coord)>::new();
    for &(a, b) in constraints {
        if !edge_set.contains(&edge_key_from_coords(a, b, eps)) {
            missing.push((a, b));
        }
    }
    missing
}

#[inline]
fn normalized_eps(epsilon: f64) -> f64 {
    if epsilon.is_finite() {
        epsilon.abs().max(1.0e-12)
    } else {
        1.0e-12
    }
}

fn dedup_points(points: &[Coord], eps: f64) -> Vec<Coord> {
    if points.is_empty() {
        return vec![];
    }

    let inv = 1.0 / eps;
    let mut buckets = HashMap::<u128, Vec<Coord>>::with_capacity(points.len());
    let mut out = Vec::<Coord>::with_capacity(points.len());

    for &p in points {
        let qx = (p.x * inv).round() as i64;
        let qy = (p.y * inv).round() as i64;

        let mut duplicate = false;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let key = pack_cell_key(qx + dx, qy + dy);
                if let Some(candidates) = buckets.get(&key) {
                    for &c in candidates {
                        if (p.x - c.x).abs() <= eps && (p.y - c.y).abs() <= eps {
                            duplicate = true;
                            break;
                        }
                    }
                }
                if duplicate {
                    break;
                }
            }
            if duplicate {
                break;
            }
        }

        if duplicate {
            continue;
        }

        out.push(p);
        buckets.entry(pack_cell_key(qx, qy)).or_default().push(p);
    }

    out.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    out
}

fn spatially_order_points(points: &mut [Coord]) {
    if points.len() < 128 {
        return;
    }

    let first = points[0];
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for &p in &points[1..] {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let sx = 1.0 / (max_x - min_x).max(1.0e-12);
    let sy = 1.0 / (max_y - min_y).max(1.0e-12);

    points.sort_by(|a, b| {
        let ka = morton_code(*a, min_x, min_y, sx, sy);
        let kb = morton_code(*b, min_x, min_y, sx, sy);
        ka.cmp(&kb)
            .then(a.x.total_cmp(&b.x))
            .then(a.y.total_cmp(&b.y))
    });
}

#[inline]
fn morton_code(p: Coord, min_x: f64, min_y: f64, sx: f64, sy: f64) -> u64 {
    let xi = (((p.x - min_x) * sx).clamp(0.0, 1.0) * 65_535.0).round() as u32;
    let yi = (((p.y - min_y) * sy).clamp(0.0, 1.0) * 65_535.0).round() as u32;
    interleave16(xi) | (interleave16(yi) << 1)
}

#[inline]
fn interleave16(mut v: u32) -> u64 {
    v &= 0x0000_FFFF;
    v = (v | (v << 8)) & 0x00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333;
    v = (v | (v << 1)) & 0x5555_5555;
    v as u64
}

fn build_super_triangle(points: &[Coord]) -> [Coord; 3] {
    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = points[0].x;
    let mut max_y = points[0].y;

    for &p in &points[1..] {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let d = dx.max(dy).max(1.0);
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;

    [
        Coord::xy(cx - 20.0 * d, cy - d),
        Coord::xy(cx, cy + 20.0 * d),
        Coord::xy(cx + 20.0 * d, cy - d),
    ]
}

fn make_triangle(a: usize, b: usize, c: usize, points: &[Coord], eps: f64) -> Option<Triangle> {
    let pa = points[a];
    let pb = points[b];
    let pc = points[c];

    let sign = orient2d_sign(pa, pb, pc, Some(eps));
    if sign == 0 {
        return None;
    }

    let v = if sign > 0 { [a, b, c] } else { [a, c, b] };
    let (center, radius2) = circumcircle(points[v[0]], points[v[1]], points[v[2]], eps)?;
    let radius = radius2.sqrt();

    Some(Triangle {
        v,
        center,
        radius2,
        min_x: center.x - radius,
        max_x: center.x + radius,
        min_y: center.y - radius,
        max_y: center.y + radius,
    })
}

fn circumcircle(a: Coord, b: Coord, c: Coord, eps: f64) -> Option<(Coord, f64)> {
    let ax = a.x;
    let ay = a.y;
    let bx = b.x;
    let by = b.y;
    let cx = c.x;
    let cy = c.y;

    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() <= eps {
        return None;
    }

    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;

    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    let center = Coord::xy(ux, uy);
    let dx = ux - ax;
    let dy = uy - ay;
    let radius2 = dx * dx + dy * dy;

    Some((center, radius2))
}

#[inline]
fn point_in_circumcircle(p: Coord, tri: &Triangle, eps: f64) -> bool {
    let dx = p.x - tri.center.x;
    let dy = p.y - tri.center.y;
    let dist2 = dx * dx + dy * dy;
    dist2 <= tri.radius2 + eps * tri.radius2.max(1.0)
}
