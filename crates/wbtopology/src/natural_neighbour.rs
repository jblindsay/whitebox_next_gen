//! True natural-neighbour (Sibson) interpolation utilities.

use std::collections::HashMap;

use crate::algorithms::measurements::polygon_area;
use crate::geom::{Coord, Envelope, LinearRing, Polygon};
use crate::overlay::polygon_intersection;
use crate::triangulation::delaunay_triangulation;

const INDEX_GRID_SIZE: usize = 64;
const INDEX_MAX_CELLS_PER_ITEM: usize = 128;

/// Prepared true Sibson interpolator over a point set.
///
/// The interpolator stores Delaunay/Voronoi structures and auxiliary indices
/// for fast point-location and cavity-walk based natural-neighbour retrieval.
#[derive(Debug, Clone)]
pub struct PreparedSibsonInterpolator {
    /// Delaunay sites used for interpolation after deduplication/reordering.
    pub points: Vec<Coord>,
    /// Delaunay triangles over [`Self::points`].
    pub triangles: Vec<[usize; 3]>,
    /// Bounded Voronoi cells aligned with [`Self::points`].
    pub cells: Vec<Polygon>,
    /// Clipping envelope used to bound Voronoi cells.
    pub clip: Envelope,
    epsilon: f64,
    circumcircles: Vec<(Coord, f64)>,
    triangle_neighbors: Vec<[Option<usize>; 3]>,
    triangle_locate_index: BBoxCandidateIndex,
    circumcircle_index: BBoxCandidateIndex,
}

/// Reusable working buffers for high-throughput Sibson queries.
#[derive(Debug, Clone)]
pub struct SibsonScratch {
    candidates: Vec<usize>,
    queue: Vec<usize>,
    cavity: Vec<usize>,
    tri_seen: Vec<u32>,
    tri_stamp: u32,
    site_seen: Vec<u32>,
    site_stamp: u32,
    neighbors: Vec<usize>,
    cell_a: Vec<Coord>,
    cell_b: Vec<Coord>,
}

impl SibsonScratch {
    fn new(triangle_count: usize, point_count: usize) -> Self {
        Self {
            candidates: Vec::new(),
            queue: Vec::new(),
            cavity: Vec::new(),
            tri_seen: vec![0; triangle_count],
            tri_stamp: 1,
            site_seen: vec![0; point_count],
            site_stamp: 1,
            neighbors: Vec::new(),
            cell_a: Vec::new(),
            cell_b: Vec::new(),
        }
    }

    fn reset_tri_seen(&mut self, triangle_count: usize) {
        if self.tri_seen.len() != triangle_count {
            self.tri_seen.resize(triangle_count, 0);
            self.tri_stamp = 1;
        } else if self.tri_stamp == u32::MAX {
            self.tri_seen.fill(0);
            self.tri_stamp = 1;
        } else {
            self.tri_stamp = self.tri_stamp.wrapping_add(1);
        }
    }

    fn reset_site_seen(&mut self, point_count: usize) {
        if self.site_seen.len() != point_count {
            self.site_seen.resize(point_count, 0);
            self.site_stamp = 1;
        } else if self.site_stamp == u32::MAX {
            self.site_seen.fill(0);
            self.site_stamp = 1;
        } else {
            self.site_stamp = self.site_stamp.wrapping_add(1);
        }
    }
}

impl PreparedSibsonInterpolator {
    /// Build a prepared true Sibson interpolator from input sites.
    pub fn new(points: &[Coord], epsilon: f64) -> Self {
        let eps = normalized_eps(epsilon);
        let tri = delaunay_triangulation(points, eps);
        let clip = if tri.points.is_empty() {
            Envelope::new(0.0, 0.0, 0.0, 0.0)
        } else {
            auto_clip_envelope(&tri.points)
        };

        let site_neighbors = build_site_neighbors(tri.points.len(), &tri.triangles);
        let cells = if tri.points.is_empty() {
            Vec::new()
        } else if tri.points.len() == 1 {
            vec![rect_polygon(clip)]
        } else {
            tri.points
                .iter()
                .enumerate()
                .map(|(i, &p)| build_cell_for_site(i, p, &tri.points, &site_neighbors, clip, eps))
                .collect()
        };

        let circumcircles: Vec<(Coord, f64)> = tri
            .triangles
            .iter()
            .map(|tri_idx| {
                circumcircle(
                    tri.points[tri_idx[0]],
                    tri.points[tri_idx[1]],
                    tri.points[tri_idx[2]],
                    eps,
                )
                .unwrap_or((Coord::xy(f64::NAN, f64::NAN), f64::NAN))
            })
            .collect();

        let tri_bounds = build_triangle_bboxes(&tri.points, &tri.triangles);
        let circle_bounds = build_circumcircle_bboxes(&circumcircles);
        let triangle_locate_index = BBoxCandidateIndex::new(&tri_bounds);
        let circumcircle_index = BBoxCandidateIndex::new(&circle_bounds);
        let triangle_neighbors = build_triangle_neighbors(&tri.triangles);

        Self {
            points: tri.points,
            triangles: tri.triangles,
            cells,
            clip,
            epsilon: eps,
            circumcircles,
            triangle_neighbors,
            triangle_locate_index,
            circumcircle_index,
        }
    }

    /// Interpolate `values` at `query` using true Sibson weights.
    pub fn interpolate(&self, query: Coord, values: &[f64]) -> Option<f64> {
        let mut scratch = self.new_scratch();
        self.interpolate_with_scratch(query, values, &mut scratch)
    }

    /// Interpolate `values` at `query` using a reusable scratch buffer.
    pub fn interpolate_with_scratch(
        &self,
        query: Coord,
        values: &[f64],
        scratch: &mut SibsonScratch,
    ) -> Option<f64> {
        if values.len() != self.points.len() {
            return None;
        }

        if self.points.is_empty() {
            return None;
        }

        let tol2 = self.epsilon * self.epsilon;
        for (idx, p) in self.points.iter().enumerate() {
            let dx = query.x - p.x;
            let dy = query.y - p.y;
            if dx * dx + dy * dy <= tol2 {
                return Some(values[idx]);
            }
        }

        self.natural_neighbor_indices_with_scratch(query, scratch);
        if scratch.neighbors.is_empty() {
            return None;
        }

        let neighbors = std::mem::take(&mut scratch.neighbors);

        if !self.query_cell_coords(query, &neighbors, scratch) {
            scratch.neighbors = neighbors;
            return None;
        }
        let query_cell = Polygon::new(LinearRing::new(scratch.cell_a.clone()), vec![]);
        let query_area = polygon_area(&query_cell);
        if query_area <= self.epsilon {
            scratch.neighbors = neighbors;
            return None;
        }

        let mut weighted_sum = 0.0;
        let mut area_sum = 0.0;
        for &idx in &neighbors {
            let overlap = polygon_intersection(&query_cell, &self.cells[idx], self.epsilon);
            let area = overlap.iter().map(polygon_area).sum::<f64>();
            if area > self.epsilon {
                weighted_sum += values[idx] * area;
                area_sum += area;
            }
        }

        scratch.neighbors = neighbors;

        if area_sum > 0.0 {
            Some(weighted_sum / area_sum)
        } else {
            None
        }
    }

    /// Compute true Sibson weights for `query`.
    pub fn weights(&self, query: Coord) -> Vec<(usize, f64)> {
        let mut scratch = self.new_scratch();
        self.weights_with_scratch(query, &mut scratch)
    }

    /// Compute true Sibson weights for `query` using a reusable scratch buffer.
    pub fn weights_with_scratch(&self, query: Coord, scratch: &mut SibsonScratch) -> Vec<(usize, f64)> {
        if self.points.is_empty() {
            return Vec::new();
        }

        let tol2 = self.epsilon * self.epsilon;
        for (idx, p) in self.points.iter().enumerate() {
            let dx = query.x - p.x;
            let dy = query.y - p.y;
            if dx * dx + dy * dy <= tol2 {
                return vec![(idx, 1.0)];
            }
        }

        self.natural_neighbor_indices_with_scratch(query, scratch);
        if scratch.neighbors.is_empty() {
            return Vec::new();
        }

        let neighbors = std::mem::take(&mut scratch.neighbors);

        if !self.query_cell_coords(query, &neighbors, scratch) {
            scratch.neighbors = neighbors;
            return Vec::new();
        }
        let query_cell = Polygon::new(LinearRing::new(scratch.cell_a.clone()), vec![]);
        let query_area = polygon_area(&query_cell);
        if query_area <= self.epsilon {
            scratch.neighbors = neighbors;
            return Vec::new();
        }

        let mut weights = Vec::with_capacity(neighbors.len());
        let mut sum = 0.0;
        for &idx in &neighbors {
            let overlap = polygon_intersection(&query_cell, &self.cells[idx], self.epsilon);
            let area = overlap.iter().map(polygon_area).sum::<f64>();
            if area > self.epsilon {
                let w = area / query_area;
                weights.push((idx, w));
                sum += w;
            }
        }

        if sum > 0.0 {
            for (_, w) in &mut weights {
                *w /= sum;
            }
        }

        scratch.neighbors = neighbors;

        weights
    }

    /// Locate a triangle containing `query` using indexed candidate lookup.
    ///
    /// Returns triangle index into [`Self::triangles`] when found.
    pub fn locate_triangle(&self, query: Coord) -> Option<usize> {
        let mut candidates = Vec::new();
        self.triangle_locate_index
            .collect_candidates_for_point(query, &mut candidates);
        for tri_idx in candidates {
            let tri = self.triangles[tri_idx];
            let p1 = self.points[tri[0]];
            let p2 = self.points[tri[1]];
            let p3 = self.points[tri[2]];
            if point_in_triangle(query, p1, p2, p3, self.epsilon) {
                return Some(tri_idx);
            }
        }
        None
    }

    /// Return triangle indices in the Bowyer-Watson cavity for `query`.
    ///
    /// The cavity is traversed locally by graph-walking adjacent triangles
    /// starting from a located seed triangle (or an indexed circumcircle seed).
    pub fn cavity_triangles(&self, query: Coord) -> Vec<usize> {
        let mut scratch = self.new_scratch();
        self.cavity_triangles_with_scratch(query, &mut scratch);
        scratch.cavity
    }

    /// Create a reusable scratch object sized for this interpolator.
    pub fn new_scratch(&self) -> SibsonScratch {
        SibsonScratch::new(self.triangles.len(), self.points.len())
    }

    fn cavity_triangles_with_scratch(
        &self,
        query: Coord,
        scratch: &mut SibsonScratch,
    ) {
        if self.triangles.is_empty() {
            scratch.cavity.clear();
            return;
        }

        scratch.queue.clear();
        if let Some(seed) = self.locate_triangle_with_scratch(query, scratch) {
            scratch.queue.push(seed);
        } else {
            // Fallback seed for points close to/just outside the convex hull.
            if let Some(seed) = self
                .circumcircle_index
                .candidates_for_point(query)
                .into_iter()
                .find(|&tri_idx| self.point_in_circumcircle(query, tri_idx))
            {
                scratch.queue.push(seed);
            }
        }

        if scratch.queue.is_empty() {
            // Last-resort: use indexed circumcircle candidates directly.
            scratch.cavity.clear();
            self.circumcircle_index
                .collect_candidates_for_point(query, &mut scratch.cavity);
            scratch
                .cavity
                .retain(|&tri_idx| self.point_in_circumcircle(query, tri_idx));
            return;
        }

        scratch.reset_tri_seen(self.triangles.len());
        scratch.cavity.clear();

        while let Some(tri_idx) = scratch.queue.pop() {
            if tri_idx >= self.triangles.len() || scratch.tri_seen[tri_idx] == scratch.tri_stamp {
                continue;
            }
            scratch.tri_seen[tri_idx] = scratch.tri_stamp;

            if !self.point_in_circumcircle(query, tri_idx) {
                continue;
            }

            scratch.cavity.push(tri_idx);
            for maybe_nb in self.triangle_neighbors[tri_idx] {
                if let Some(nb) = maybe_nb {
                    if scratch.tri_seen[nb] != scratch.tri_stamp {
                        scratch.queue.push(nb);
                    }
                }
            }
        }

    }

    fn natural_neighbor_indices_with_scratch(
        &self,
        query: Coord,
        scratch: &mut SibsonScratch,
    ) {
        self.cavity_triangles_with_scratch(query, scratch);
        scratch.reset_site_seen(self.points.len());
        scratch.neighbors.clear();

        let cavity = std::mem::take(&mut scratch.cavity);

        for &tri_idx in &cavity {
            for &vid in &self.triangles[tri_idx] {
                if vid < self.points.len() && scratch.site_seen[vid] != scratch.site_stamp {
                    scratch.site_seen[vid] = scratch.site_stamp;
                    scratch.neighbors.push(vid);
                }
            }
        }

        scratch.cavity = cavity;
    }

    fn point_in_circumcircle(&self, query: Coord, tri_idx: usize) -> bool {
        let (center, radius2) = self.circumcircles[tri_idx];
        if !center.x.is_finite() || !center.y.is_finite() || !radius2.is_finite() {
            return false;
        }
        let dx = query.x - center.x;
        let dy = query.y - center.y;
        let dist2 = dx * dx + dy * dy;
        dist2 <= radius2 + self.epsilon * radius2.max(1.0)
    }

    fn query_cell_coords(&self, query: Coord, neighbors: &[usize], scratch: &mut SibsonScratch) -> bool {
        scratch.cell_a.clear();
        scratch.cell_a.extend(rect_coords(self.clip));
        scratch.cell_b.clear();
        scratch
            .cell_b
            .reserve(scratch.cell_a.len() + neighbors.len() + 8);

        for &idx in neighbors {
            let site = self.points[idx];
            let nx = site.x - query.x;
            let ny = site.y - query.y;
            if nx == 0.0 && ny == 0.0 {
                continue;
            }
            let c = 0.5
                * ((site.x * site.x + site.y * site.y)
                    - (query.x * query.x + query.y * query.y));
            clip_polygon_halfplane_into(&scratch.cell_a, &mut scratch.cell_b, nx, ny, c, self.epsilon);
            std::mem::swap(&mut scratch.cell_a, &mut scratch.cell_b);
            scratch.cell_b.clear();
            if scratch.cell_a.len() < 3 {
                return false;
            }
        }

        true
    }

    fn locate_triangle_with_scratch(&self, query: Coord, scratch: &mut SibsonScratch) -> Option<usize> {
        self.triangle_locate_index
            .collect_candidates_for_point(query, &mut scratch.candidates);
        for &tri_idx in &scratch.candidates {
            let tri = self.triangles[tri_idx];
            let p1 = self.points[tri[0]];
            let p2 = self.points[tri[1]];
            let p3 = self.points[tri[2]];
            if point_in_triangle(query, p1, p2, p3, self.epsilon) {
                return Some(tri_idx);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
struct BBoxCandidateIndex {
    min_x: f64,
    min_y: f64,
    inv_dx: f64,
    inv_dy: f64,
    nx: usize,
    ny: usize,
    bins: Vec<Vec<usize>>,
    global: Vec<usize>,
}

impl BBoxCandidateIndex {
    fn new(bounds: &[Envelope]) -> Self {
        if bounds.is_empty() {
            return Self {
                min_x: 0.0,
                min_y: 0.0,
                inv_dx: 1.0,
                inv_dy: 1.0,
                nx: 1,
                ny: 1,
                bins: vec![Vec::new()],
                global: Vec::new(),
            };
        }

        let mut min_x = bounds[0].min_x;
        let mut min_y = bounds[0].min_y;
        let mut max_x = bounds[0].max_x;
        let mut max_y = bounds[0].max_y;
        for b in &bounds[1..] {
            min_x = min_x.min(b.min_x);
            min_y = min_y.min(b.min_y);
            max_x = max_x.max(b.max_x);
            max_y = max_y.max(b.max_y);
        }

        let dx = (max_x - min_x).max(1.0e-9);
        let dy = (max_y - min_y).max(1.0e-9);
        let nx = INDEX_GRID_SIZE;
        let ny = INDEX_GRID_SIZE;
        let mut out = Self {
            min_x,
            min_y,
            inv_dx: 1.0 / dx,
            inv_dy: 1.0 / dy,
            nx,
            ny,
            bins: vec![Vec::new(); nx * ny],
            global: Vec::new(),
        };

        for (id, b) in bounds.iter().enumerate() {
            out.insert_bbox(id, *b);
        }

        out
    }

    fn candidates_for_point(&self, p: Coord) -> Vec<usize> {
        let mut out = Vec::new();
        self.collect_candidates_for_point(p, &mut out);
        out
    }

    fn collect_candidates_for_point(&self, p: Coord, out: &mut Vec<usize>) {
        out.clear();
        out.extend(self.global.iter().copied());

        if let Some((ix, iy)) = self.point_cell(p) {
            let cell = iy * self.nx + ix;
            out.extend(self.bins[cell].iter().copied());
        }
    }

    fn insert_bbox(&mut self, id: usize, b: Envelope) {
        let Some((ix0, ix1, iy0, iy1)) = self.bbox_range(b) else {
            self.global.push(id);
            return;
        };

        let nx = ix1 - ix0 + 1;
        let ny = iy1 - iy0 + 1;
        if nx * ny > INDEX_MAX_CELLS_PER_ITEM {
            self.global.push(id);
            return;
        }

        for iy in iy0..=iy1 {
            let row = iy * self.nx;
            for ix in ix0..=ix1 {
                self.bins[row + ix].push(id);
            }
        }
    }

    fn bbox_range(&self, b: Envelope) -> Option<(usize, usize, usize, usize)> {
        if b.max_x < self.min_x || b.max_y < self.min_y {
            return None;
        }

        let fx0 = ((b.min_x - self.min_x) * self.inv_dx).clamp(0.0, 1.0);
        let fx1 = ((b.max_x - self.min_x) * self.inv_dx).clamp(0.0, 1.0);
        let fy0 = ((b.min_y - self.min_y) * self.inv_dy).clamp(0.0, 1.0);
        let fy1 = ((b.max_y - self.min_y) * self.inv_dy).clamp(0.0, 1.0);

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

fn normalized_eps(epsilon: f64) -> f64 {
    epsilon.abs().max(1.0e-12)
}

fn auto_clip_envelope(points: &[Coord]) -> Envelope {
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
    let pad = dx.max(dy).max(1.0) * 2.0;
    Envelope::new(min_x - pad, min_y - pad, max_x + pad, max_y + pad)
}

fn rect_coords(env: Envelope) -> Vec<Coord> {
    vec![
        Coord::xy(env.min_x, env.min_y),
        Coord::xy(env.max_x, env.min_y),
        Coord::xy(env.max_x, env.max_y),
        Coord::xy(env.min_x, env.max_y),
    ]
}

fn rect_polygon(env: Envelope) -> Polygon {
    Polygon::new(LinearRing::new(rect_coords(env)), vec![])
}

fn build_site_neighbors(site_count: usize, triangles: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::<usize>::new(); site_count];
    for &t in triangles {
        let a = t[0];
        let b = t[1];
        let c = t[2];

        if a < site_count && b < site_count {
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
        if b < site_count && c < site_count {
            neighbors[b].push(c);
            neighbors[c].push(b);
        }
        if c < site_count && a < site_count {
            neighbors[c].push(a);
            neighbors[a].push(c);
        }
    }

    let mut seen = vec![0u32; site_count];
    let mut stamp = 1u32;
    for nbrs in &mut neighbors {
        if stamp == u32::MAX {
            seen.fill(0);
            stamp = 1;
        }

        let mut write = 0usize;
        for read in 0..nbrs.len() {
            let id = nbrs[read];
            if id >= site_count {
                continue;
            }
            if seen[id] == stamp {
                continue;
            }
            seen[id] = stamp;
            nbrs[write] = id;
            write += 1;
        }
        nbrs.truncate(write);
        stamp = stamp.wrapping_add(1);
    }

    neighbors
}

fn build_triangle_neighbors(triangles: &[[usize; 3]]) -> Vec<[Option<usize>; 3]> {
    let mut neighbors = vec![[None; 3]; triangles.len()];
    let mut edge_map: HashMap<(usize, usize), (usize, usize)> = HashMap::new();

    for (tri_idx, tri) in triangles.iter().enumerate() {
        let edges = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
        for (edge_pos, &(a, b)) in edges.iter().enumerate() {
            let key = if a <= b { (a, b) } else { (b, a) };
            if let Some((other_tri, other_edge_pos)) = edge_map.remove(&key) {
                neighbors[tri_idx][edge_pos] = Some(other_tri);
                neighbors[other_tri][other_edge_pos] = Some(tri_idx);
            } else {
                edge_map.insert(key, (tri_idx, edge_pos));
            }
        }
    }

    neighbors
}

fn build_triangle_bboxes(points: &[Coord], triangles: &[[usize; 3]]) -> Vec<Envelope> {
    triangles
        .iter()
        .map(|t| {
            let p0 = points[t[0]];
            let p1 = points[t[1]];
            let p2 = points[t[2]];
            let min_x = p0.x.min(p1.x.min(p2.x));
            let max_x = p0.x.max(p1.x.max(p2.x));
            let min_y = p0.y.min(p1.y.min(p2.y));
            let max_y = p0.y.max(p1.y.max(p2.y));
            Envelope::new(min_x, min_y, max_x, max_y)
        })
        .collect()
}

fn build_circumcircle_bboxes(circles: &[(Coord, f64)]) -> Vec<Envelope> {
    circles
        .iter()
        .map(|(c, r2)| {
            if !c.x.is_finite() || !c.y.is_finite() || !r2.is_finite() {
                Envelope::new(0.0, 0.0, 0.0, 0.0)
            } else {
                let r = r2.sqrt();
                Envelope::new(c.x - r, c.y - r, c.x + r, c.y + r)
            }
        })
        .collect()
}

fn point_in_triangle(p: Coord, a: Coord, b: Coord, c: Coord, eps: f64) -> bool {
    let v0x = c.x - a.x;
    let v0y = c.y - a.y;
    let v1x = b.x - a.x;
    let v1y = b.y - a.y;
    let v2x = p.x - a.x;
    let v2y = p.y - a.y;

    let dot00 = v0x * v0x + v0y * v0y;
    let dot01 = v0x * v1x + v0y * v1y;
    let dot02 = v0x * v2x + v0y * v2y;
    let dot11 = v1x * v1x + v1y * v1y;
    let dot12 = v1x * v2x + v1y * v2y;

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() <= eps {
        return false;
    }
    let inv_denom = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    u >= -eps && v >= -eps && (u + v) <= 1.0 + eps
}

fn build_cell_for_site(
    i: usize,
    p: Coord,
    sites: &[Coord],
    neighbors: &[Vec<usize>],
    clip: Envelope,
    epsilon: f64,
) -> Polygon {
    let mut cell = rect_coords(clip);
    let mut scratch = Vec::<Coord>::with_capacity(cell.len() + 8);
    for &j in &neighbors[i] {
        let q = sites[j];
        let nx = q.x - p.x;
        let ny = q.y - p.y;
        if nx == 0.0 && ny == 0.0 {
            continue;
        }
        let c = 0.5 * ((q.x * q.x + q.y * q.y) - (p.x * p.x + p.y * p.y));
        clip_polygon_halfplane_into(&cell, &mut scratch, nx, ny, c, epsilon);
        std::mem::swap(&mut cell, &mut scratch);
        scratch.clear();
        if cell.len() < 3 {
            break;
        }
    }

    if cell.len() < 3 {
        Polygon::new(LinearRing::new(vec![]), vec![])
    } else {
        Polygon::new(LinearRing::new(cell), vec![])
    }
}

fn clip_polygon_halfplane_into(
    poly: &[Coord],
    out: &mut Vec<Coord>,
    nx: f64,
    ny: f64,
    c: f64,
    eps: f64,
) {
    if poly.len() < 3 {
        out.clear();
        return;
    }

    out.clear();
    out.reserve(poly.len() + 4);
    let n = poly.len();

    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let da = nx * a.x + ny * a.y - c;
        let db = nx * b.x + ny * b.y - c;
        let a_in = da <= eps;
        let b_in = db <= eps;

        if a_in && b_in {
            out.push(b);
        } else if a_in && !b_in {
            if let Some(x) = segment_line_intersection(a, b, nx, ny, c, eps) {
                out.push(x);
            }
        } else if !a_in && b_in {
            if let Some(x) = segment_line_intersection(a, b, nx, ny, c, eps) {
                out.push(x);
            }
            out.push(b);
        }
    }

    dedup_consecutive(out, eps);
}

fn segment_line_intersection(a: Coord, b: Coord, nx: f64, ny: f64, c: f64, eps: f64) -> Option<Coord> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let denom = nx * dx + ny * dy;
    if denom.abs() <= eps.abs().max(1.0e-12) {
        return None;
    }

    let t = (c - (nx * a.x + ny * a.y)) / denom;
    let tc = t.clamp(0.0, 1.0);
    Some(Coord::interpolate_segment(a, b, tc))
}

fn dedup_consecutive(coords: &mut Vec<Coord>, eps: f64) {
    if coords.is_empty() {
        return;
    }

    let tol = eps.abs().max(1.0e-12);
    let mut write = 1usize;
    for read in 1..coords.len() {
        let p = coords[read];
        let q = coords[write - 1];
        if (p.x - q.x).abs() <= tol && (p.y - q.y).abs() <= tol {
            continue;
        }
        coords[write] = p;
        write += 1;
    }
    coords.truncate(write);

    if coords.len() >= 2 {
        let first = coords[0];
        let last = coords[coords.len() - 1];
        if (first.x - last.x).abs() <= tol && (first.y - last.y).abs() <= tol {
            coords.pop();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibson_exactly_reproduces_site_value() {
        let points = vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0), Coord::xy(0.0, 1.0)];
        let interp = PreparedSibsonInterpolator::new(&points, 1.0e-12);
        let values: Vec<f64> = interp
            .points
            .iter()
            .map(|p| {
                if (p.x - 0.0).abs() < 1.0e-12 && (p.y - 0.0).abs() < 1.0e-12 {
                    5.0
                } else if (p.x - 1.0).abs() < 1.0e-12 && (p.y - 0.0).abs() < 1.0e-12 {
                    7.0
                } else {
                    11.0
                }
            })
            .collect();
        let out = interp.interpolate(Coord::xy(1.0, 0.0), &values).unwrap();
        assert!((out - 7.0).abs() < 1.0e-12);
    }

    #[test]
    fn sibson_reproduces_linear_field_inside_triangle() {
        let points = vec![Coord::xy(0.0, 0.0), Coord::xy(1.0, 0.0), Coord::xy(0.0, 1.0)];
        let interp = PreparedSibsonInterpolator::new(&points, 1.0e-12);
        let values: Vec<f64> = interp.points.iter().map(|p| p.x + p.y).collect();
        let query = Coord::xy(1.0 / 3.0, 1.0 / 3.0);
        let out = interp.interpolate(query, &values).unwrap();
        assert!((out - (query.x + query.y)).abs() < 1.0e-8, "out={out}");
    }

    #[test]
    fn sibson_weights_sum_to_one() {
        let points = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ];
        let interp = PreparedSibsonInterpolator::new(&points, 1.0e-12);
        let weights = interp.weights(Coord::xy(0.5, 0.5));
        let sum: f64 = weights.iter().map(|(_, w)| *w).sum();
        assert!((sum - 1.0).abs() < 1.0e-8, "weights={weights:?}");
    }

    #[test]
    fn locate_triangle_finds_interior_query() {
        let points = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
        ];
        let interp = PreparedSibsonInterpolator::new(&points, 1.0e-12);
        assert!(interp.locate_triangle(Coord::xy(0.4, 0.4)).is_some());
    }

    #[test]
    fn sibson_reproduces_linear_field_on_scattered_sites() {
        let points = vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.0),
            Coord::xy(1.0, 1.0),
            Coord::xy(0.0, 1.0),
            Coord::xy(0.5, 0.1),
            Coord::xy(0.8, 0.4),
            Coord::xy(0.6, 0.8),
            Coord::xy(0.2, 0.7),
            Coord::xy(0.3, 0.3),
        ];
        let interp = PreparedSibsonInterpolator::new(&points, 1.0e-12);

        let plane = |p: Coord| 2.5 * p.x - 1.75 * p.y + 4.0;
        let values: Vec<f64> = interp.points.iter().map(|&p| plane(p)).collect();

        let queries = vec![
            Coord::xy(0.2, 0.2),
            Coord::xy(0.4, 0.3),
            Coord::xy(0.7, 0.2),
            Coord::xy(0.7, 0.7),
            Coord::xy(0.3, 0.8),
            Coord::xy(0.55, 0.45),
        ];

        for q in queries {
            let out = interp.interpolate(q, &values).expect("interpolation should succeed");
            let expected = plane(q);
            assert!(
                (out - expected).abs() < 1.0e-6,
                "query=({:.3},{:.3}) out={} expected={}",
                q.x,
                q.y,
                out,
                expected
            );
        }
    }
}
