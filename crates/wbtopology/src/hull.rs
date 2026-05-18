//! Convex and concave hull utilities.
//!
//! The convex hull implementation uses Andrew's monotone chain and returns the
//! tightest enclosing geometry as a `Point`, `LineString`, or `Polygon`.
//!
//! The concave hull implementation provides three backends selectable via
//! [`ConcaveHullEngine`]:
//!
//! * [`ConcaveHullEngine::Concaveman`] (**default**) — a Rust port of the
//!   concaveman algorithm (Mapbox / Park & Oh, 2012).  Starting from the convex
//!   hull, boundary edges are iteratively refined by pulling in nearby interior
//!   points, guided by a dimensionless `concavity` parameter and an optional
//!   minimum-edge-length threshold.  Always produces a single connected polygon.
//!
//! * [`ConcaveHullEngine::Delaunay`] — an alpha-shape-style approach based on
//!   Delaunay triangulation with connectivity-aware boundary-inward triangle
//!   filtering.  Supports disjoint multipolygon output when `allow_disjoint`
//!   is `true`.
//!
//! * [`ConcaveHullEngine::FastRefine`] — a lightweight convex-hull edge
//!   refinement algorithm suited to small or simple point sets.

use std::collections::HashSet;

use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::algorithms::segment::segments_intersect_eps;
use crate::constructive::polygonize_closed_linestrings;
use crate::geom::{Coord, Envelope, Geometry, LineString, LinearRing, Polygon};
use crate::precision::PrecisionModel;
use crate::spatial_index::SpatialIndex;
use crate::triangulation::delaunay_triangulation;

/// Concave hull backend algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcaveHullEngine {
    /// Concaveman algorithm (Mapbox / Park & Oh, 2012) — the default.
    ///
    /// Starts from the convex hull and iteratively pulls boundary edges toward
    /// nearby interior points.  Controlled by [`ConcaveHullOptions::concavity`]
    /// and [`ConcaveHullOptions::max_edge_length`].  Always produces a single
    /// connected polygon.
    Concaveman,
    /// Delaunay triangle filtering + polygonization.
    ///
    /// Alpha-shape-style approach with connectivity-aware boundary-inward
    /// triangle filtering.  Supports disjoint multipolygon output.
    Delaunay,
    /// Lightweight convex-hull edge refinement.
    FastRefine,
}

/// Configuration options for concave hull generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConcaveHullOptions {
    /// Concave hull backend algorithm.
    pub engine: ConcaveHullEngine,
    /// Dimensionless concavity factor used by [`ConcaveHullEngine::Concaveman`].
    ///
    /// Controls how aggressively boundary edges are allowed to become concave
    /// relative to their length.  `1.0` yields a highly detailed hull;
    /// `f64::INFINITY` produces the convex hull.  The default `2.0` matches
    /// the original Mapbox JavaScript implementation.
    pub concavity: f64,
    /// Edge-length threshold used by the selected engine.
    ///
    /// * **Concaveman**: edges shorter than this length are not refined further
    ///   (`lengthThreshold` in the original JS API).
    /// * **Delaunay**: maximum edge length allowed in kept triangles.
    ///
    /// Default: `f64::INFINITY` (unlimited refinement / largest threshold).
    pub max_edge_length: f64,
    /// Optional relative threshold expressed as a fraction of the input bbox diagonal.
    ///
    /// When set to `Some(r)`, the effective edge threshold becomes
    /// `r * bbox_diagonal(input_points)`. This provides a scale-free concavity
    /// control that is often easier to tune than an absolute distance.
    ///
    /// If both `relative_edge_length_ratio` and `max_edge_length` are set, the
    /// relative threshold takes precedence.
    pub relative_edge_length_ratio: Option<f64>,
    /// Epsilon used for point deduplication and geometric tolerances.
    pub epsilon: f64,
    /// Optional precision snapping applied before hull construction.
    pub precision: Option<PrecisionModel>,
    /// Whether disconnected components are allowed in the result.
    ///
    /// When `false`, the largest surviving polygonal component is returned.
    pub allow_disjoint: bool,
    /// Minimum polygon area to keep in the output.
    ///
    /// This is useful for dropping tiny sliver artifacts from aggressive
    /// concave hull thresholds.
    pub min_area: f64,
}

impl Default for ConcaveHullOptions {
    fn default() -> Self {
        Self {
            engine: ConcaveHullEngine::Concaveman,
            concavity: 2.0,
            max_edge_length: f64::INFINITY,
            relative_edge_length_ratio: None,
            epsilon: 1.0e-12,
            precision: None,
            allow_disjoint: true,
            min_area: 0.0,
        }
    }
}

/// Compute the convex hull of a point set.
///
/// Returns:
/// - empty `GeometryCollection` when `coords` is empty
/// - `Point` for a single unique coordinate
/// - `LineString` for two unique coordinates or collinear inputs
/// - `Polygon` otherwise
pub fn convex_hull(coords: &[Coord], epsilon: f64) -> Geometry {
    let eps = normalized_eps(epsilon);
    let pts = unique_sorted_points(coords, eps);
    if pts.is_empty() {
        return Geometry::GeometryCollection(vec![]);
    }
    if pts.len() == 1 {
        return Geometry::Point(pts[0]);
    }
    if pts.len() == 2 {
        return Geometry::LineString(LineString::new(vec![pts[0], pts[1]]));
    }

    let mut lower = Vec::<Coord>::new();
    for &p in &pts {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= eps
        {
            lower.pop();
        }
        lower.push(p);
    }

    let mut upper = Vec::<Coord>::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= eps
        {
            upper.pop();
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);

    if lower.len() <= 1 {
        return Geometry::Point(lower[0]);
    }
    if lower.len() == 2 {
        return Geometry::LineString(LineString::new(lower));
    }

    Geometry::Polygon(Polygon::new(LinearRing::new(lower), vec![]))
}

/// Compute the convex hull of all coordinates contained in a geometry.
pub fn convex_hull_geometry(geometry: &Geometry, epsilon: f64) -> Geometry {
    let coords = collect_geometry_coords(geometry);
    convex_hull(&coords, epsilon)
}

/// Compute the convex hull of a point set after snapping under `precision`.
pub fn convex_hull_with_precision(coords: &[Coord], precision: PrecisionModel) -> Geometry {
    let mut input: Vec<Coord> = coords
        .iter()
        .copied()
        .filter(|c| c.x.is_finite() && c.y.is_finite())
        .collect();
    precision.apply_coords_in_place(&mut input);
    convex_hull(&input, precision.epsilon())
}

/// Compute the convex hull of all coordinates contained in a geometry after snapping under `precision`.
pub fn convex_hull_geometry_with_precision(
    geometry: &Geometry,
    precision: PrecisionModel,
) -> Geometry {
    let snapped = precision.apply_geometry(geometry);
    convex_hull_geometry(&snapped, precision.epsilon())
}

/// Compute a pragmatic concave hull of a point set.
///
/// `max_edge_length` controls the amount of concavity: smaller values preserve
/// only tighter local triangles, larger values approach the convex hull.
///
/// Returns:
/// - empty `GeometryCollection` when `coords` is empty
/// - `Point` / `LineString` for degenerate small inputs
/// - `Polygon` or `MultiPolygon` for areal outputs
pub fn concave_hull(coords: &[Coord], max_edge_length: f64, epsilon: f64) -> Geometry {
    concave_hull_with_options(
        coords,
        ConcaveHullOptions {
            max_edge_length,
            epsilon,
            ..Default::default()
        },
    )
}

/// Compute a pragmatic concave hull using advanced options.
pub fn concave_hull_with_options(coords: &[Coord], options: ConcaveHullOptions) -> Geometry {
    let eps = options
        .precision
        .map(|pm| normalized_eps(options.epsilon).max(pm.epsilon()))
        .unwrap_or_else(|| normalized_eps(options.epsilon));

    let mut input: Vec<Coord> = coords
        .iter()
        .copied()
        .filter(|c| c.x.is_finite() && c.y.is_finite())
        .collect();
    if let Some(pm) = options.precision {
        pm.apply_coords_in_place(&mut input);
    }

    if input.is_empty() {
        return Geometry::GeometryCollection(vec![]);
    }

    let pts = unique_sorted_points(&input, eps);
    if pts.is_empty() {
        return Geometry::GeometryCollection(vec![]);
    }
    if pts.len() < 3 {
        return convex_hull(&pts, eps);
    }

    match options.engine {
        ConcaveHullEngine::Concaveman => concave_hull_concaveman_from_points(&pts, eps, options),
        ConcaveHullEngine::Delaunay => concave_hull_delaunay_from_points(&pts, eps, options),
        ConcaveHullEngine::FastRefine => concave_hull_fast_refine_from_points(&pts, eps, options),
    }
}

fn concave_hull_delaunay_from_points(
    pts: &[Coord],
    eps: f64,
    options: ConcaveHullOptions,
) -> Geometry {

    let tri = delaunay_triangulation(&pts, eps);

    let effective_max_edge_length = effective_max_edge_length(&tri.points, options);
    if !effective_max_edge_length.is_finite() || effective_max_edge_length <= 0.0 {
        return convex_hull(&tri.points, eps);
    }

    if tri.triangles.is_empty() {
        return convex_hull(&tri.points, eps);
    }

    let max_len2 = (effective_max_edge_length + eps).powi(2);
    
    // Iterative boundary-inward filtering with connectivity preservation.
    // Start with all triangles, then iteratively remove boundary triangles
    // that violate the max_edge_length criterion, without orphaning nodes.
    let mut remaining = vec![true; tri.triangles.len()];
    let mut changed = true;
    
    while changed {
        changed = false;
        
        // Build edge occurrence map for current remaining triangles.
        let mut edge_count: std::collections::HashMap<u128, usize> = std::collections::HashMap::new();
        for (tri_idx, t) in tri.triangles.iter().enumerate() {
            if !remaining[tri_idx] {
                continue;
            }
            let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            for &(a, b) in &edges {
                *edge_count.entry(pack_edge(a, b)).or_insert(0) += 1;
            }
        }
        
        // Find boundary triangles (those with at least one boundary edge).
        let mut boundary_triangles = Vec::<usize>::new();
        for (tri_idx, t) in tri.triangles.iter().enumerate() {
            if !remaining[tri_idx] {
                continue;
            }
            let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            let has_boundary_edge = edges.iter().any(|&(a, b)| {
                edge_count.get(&pack_edge(a, b)).copied().unwrap_or(0) == 1
            });
            if has_boundary_edge {
                boundary_triangles.push(tri_idx);
            }
        }
        
        // For each boundary triangle, check if it violates max_edge_length
        // and can be removed without orphaning nodes.
        for tri_idx in boundary_triangles {
            let t = &tri.triangles[tri_idx];
            let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            let violates = edges.iter().any(|&(a, b)| {
                dist2(tri.points[a], tri.points[b]) > max_len2
            });
            
            if !violates {
                continue;
            }
            
            // Check if removing this triangle would orphan any node.
            let would_orphan = t.iter().any(|&node_id| {
                let node_count = tri.triangles
                    .iter()
                    .enumerate()
                    .filter(|(idx, tri_t)| {
                        remaining[*idx] && *idx != tri_idx && tri_t.iter().any(|&n| n == node_id)
                    })
                    .count();
                node_count == 0
            });
            
            if !would_orphan {
                remaining[tri_idx] = false;
                changed = true;
            }
        }
    }
    
    // Extract edges from remaining triangles.
    let mut packed_edges = Vec::<u128>::new();
    for (tri_idx, t) in tri.triangles.iter().enumerate() {
        if !remaining[tri_idx] {
            continue;
        }
        let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
        for &(a, b) in &edges {
            packed_edges.push(pack_edge(a, b));
        }
    }
    
    if packed_edges.is_empty() {
        return convex_hull(&tri.points, eps);
    }
    
    // Identify boundary edges (appearing exactly once in remaining triangles).
    packed_edges.sort_unstable();
    let mut boundary_edges = Vec::<(usize, usize)>::new();
    let mut i = 0usize;
    while i < packed_edges.len() {
        let edge = packed_edges[i];
        let mut count = 1usize;
        i += 1;
        while i < packed_edges.len() && packed_edges[i] == edge {
            count += 1;
            i += 1;
        }
        if count == 1 {
            boundary_edges.push(unpack_edge(edge));
        }
    }
    if boundary_edges.is_empty() {
        return convex_hull(&tri.points, eps);
    }

    let mut adjacency = vec![Vec::<usize>::new(); tri.points.len()];
    for &(a, b) in &boundary_edges {
        adjacency[a].push(b);
        adjacency[b].push(a);
    }

    let mut unused: HashSet<u128> = boundary_edges.iter().map(|&(a, b)| pack_edge(a, b)).collect();
    let mut rings = Vec::<LineString>::new();

    for &(a, b) in &boundary_edges {
        let edge = pack_edge(a, b);
        if !unused.contains(&edge) {
            continue;
        }
        if let Some(ring) = walk_boundary_ring(a, b, &adjacency, &mut unused, &tri.points) {
            if ring.coords.len() >= 4 {
                rings.push(ring);
            }
        }
    }

    if rings.is_empty() {
        return convex_hull(&tri.points, eps);
    }

    let polys = polygonize_closed_linestrings(&rings, eps);
    postprocess_concave_output(geometry_from_polygons(polys), options)
}

fn concave_hull_fast_refine_from_points(
    pts: &[Coord],
    eps: f64,
    options: ConcaveHullOptions,
) -> Geometry {
    if pts.len() < 3 {
        return convex_hull(pts, eps);
    }

    let mut ring = convex_hull_indices_sorted(pts, eps);
    if ring.len() < 3 {
        return convex_hull(pts, eps);
    }

    let stop_length = effective_max_edge_length(pts, options);
    if !stop_length.is_finite() || stop_length <= 0.0 {
        return convex_hull(pts, eps);
    }

    let point_geoms: Vec<Geometry> = pts.iter().copied().map(Geometry::Point).collect();
    let point_index = SpatialIndex::from_geometries(&point_geoms);
    let mut on_ring = vec![false; pts.len()];
    for &id in &ring {
        on_ring[id] = true;
    }

    let max_inserts = pts.len().saturating_mul(2);
    let mut inserts = 0usize;

    loop {
        let mut changed = false;
        let mut i = 0usize;

        while i < ring.len() {
            let next = (i + 1) % ring.len();
            let a_idx = ring[i];
            let b_idx = ring[next];
            let a = pts[a_idx];
            let b = pts[b_idx];
            let seg_len = dist2(a, b).sqrt();
            if seg_len <= stop_length + eps {
                i += 1;
                continue;
            }

            let expand = seg_len * 0.5 + eps;
            let env = Envelope::new(
                a.x.min(b.x) - expand,
                a.y.min(b.y) - expand,
                a.x.max(b.x) + expand,
                a.y.max(b.y) + expand,
            );

            let candidate_ids = point_index.query_envelope(env);
            let mut best: Option<(usize, f64)> = None;

            for id in candidate_ids {
                if id >= pts.len() || on_ring[id] || id == a_idx || id == b_idx {
                    continue;
                }
                let p = pts[id];
                let t = segment_param(a, b, p);
                if !(eps..=(1.0 - eps)).contains(&t) {
                    continue;
                }

                let perp = point_segment_distance(a, b, p);
                if perp <= eps {
                    continue;
                }

                let new_max = dist2(a, p).sqrt().max(dist2(p, b).sqrt());
                if new_max + eps >= seg_len {
                    continue;
                }

                if !candidate_is_inside_ring(id, &ring, pts, eps) {
                    continue;
                }

                if !edge_insertion_is_valid(a_idx, b_idx, id, &ring, pts, eps) {
                    continue;
                }

                let score = perp;
                if best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((id, score));
                }
            }

            if let Some((chosen, _)) = best {
                ring.insert(i + 1, chosen);
                on_ring[chosen] = true;
                inserts += 1;
                changed = true;
                if inserts >= max_inserts {
                    break;
                }
                continue;
            }

            i += 1;
        }

        if !changed || inserts >= max_inserts {
            break;
        }
    }

    let mut coords = Vec::with_capacity(ring.len() + 1);
    for &idx in &ring {
        coords.push(pts[idx]);
    }
    if !coords.is_empty() {
        coords.push(coords[0]);
    }

    let poly = Polygon::new(LinearRing::new(coords), vec![]);
    postprocess_concave_output(Geometry::Polygon(poly), options)
}

// ---------------------------------------------------------------------------
// Concaveman engine (Mapbox / Park & Oh, 2012)
// ---------------------------------------------------------------------------

/// Internal linked-list node used by the concaveman refinement loop.
struct ConcaveNode {
    /// Index of this point in the `pts` slice.
    point_idx: usize,
    /// Position of the previous node in the `nodes` Vec.
    prev: usize,
    /// Position of the next node in the `nodes` Vec.
    next: usize,
}

/// Concaveman algorithm: convex-hull → iterative boundary edge refinement.
///
/// Reference: Park & Oh (2012), "A New Concave Hull Algorithm and Concaveness
/// Measure for n-dimensional Datasets".
/// Implementation adapted from Mapbox's JavaScript concaveman library.
fn concave_hull_concaveman_from_points(
    pts: &[Coord],
    eps: f64,
    options: ConcaveHullOptions,
) -> Geometry {
    use std::collections::VecDeque;

    // ---- 1. Start with the convex hull ----
    let hull_indices = convex_hull_indices_sorted(pts, eps);
    if hull_indices.len() < 3 {
        return convex_hull(pts, eps);
    }

    // ---- 2. Parameters ----
    // max_edge_length is the minimum edge length below which refinement stops.
    let length_threshold = effective_max_edge_length(pts, options);
    let sq_len_threshold = if length_threshold.is_finite() && length_threshold > 0.0 {
        length_threshold * length_threshold
    } else {
        0.0
    };

    // concavity: dimensionless ratio; higher = less concave.
    let concavity = options.concavity.max(1.0e-10);
    let sq_concavity = concavity * concavity;

    // ---- 3. Build doubly-linked list from convex hull ----
    let n_hull = hull_indices.len();
    let mut nodes: Vec<ConcaveNode> = hull_indices
        .iter()
        .enumerate()
        .map(|(pos, &pid)| ConcaveNode {
            point_idx: pid,
            prev: (pos + n_hull - 1) % n_hull,
            next: (pos + 1) % n_hull,
        })
        .collect();

    // Track which points are already on the hull ring.
    let mut on_hull = vec![false; pts.len()];
    for &idx in &hull_indices {
        on_hull[idx] = true;
    }

    // ---- 4. Index all points in an R-tree for fast nearest-to-segment queries ----
    let point_geoms: Vec<Geometry> = pts.iter().copied().map(Geometry::Point).collect();
    let point_index = SpatialIndex::from_geometries(&point_geoms);

    // ---- 5. Process edge queue ----
    let mut queue: VecDeque<usize> = (0..n_hull).collect();
    // Safety cap: at most 10 insertions per input point.
    let max_inserts = pts.len().saturating_mul(10);
    let mut n_inserts = 0usize;

    while let Some(node_pos) = queue.pop_front() {
        if n_inserts >= max_inserts {
            break;
        }

        let a_pos = node_pos;
        let b_pos = nodes[a_pos].next;
        let a_idx = nodes[a_pos].point_idx;
        let b_idx = nodes[b_pos].point_idx;
        let a = pts[a_idx];
        let b = pts[b_idx];

        let sq_len = dist2(a, b);

        // Skip edges already short enough to leave unrefined.
        if sq_len <= sq_len_threshold {
            continue;
        }

        // Maximum squared distance from a candidate to edge (a,b) for it to
        // be accepted.  Mirrors: maxSqLen = sqLen / sqConcavity in the JS.
        let max_sq_len = sq_len / sq_concavity;
        let search_radius = max_sq_len.sqrt() + eps;

        let env = Envelope::new(
            a.x.min(b.x) - search_radius,
            a.y.min(b.y) - search_radius,
            a.x.max(b.x) + search_radius,
            a.y.max(b.y) + search_radius,
        );

        // Collect and sort candidates by distance to edge (ascending).
        let mut candidates: Vec<usize> = point_index
            .query_envelope(env)
            .into_iter()
            .filter(|&idx| !on_hull[idx])
            .collect();

        if candidates.is_empty() {
            continue;
        }

        candidates.sort_unstable_by(|&ci, &cj| {
            cm_sq_seg_dist(pts[ci], a, b).total_cmp(&cm_sq_seg_dist(pts[cj], a, b))
        });

        // Adjacent-edge points for the adjacency constraint.
        let prev_pt = pts[nodes[nodes[a_pos].prev].point_idx];
        let next_next_pt = pts[nodes[nodes[b_pos].next].point_idx];

        // Find the first valid candidate in distance order.
        let mut chosen: Option<usize> = None;
        for &c_idx in &candidates {
            let c = pts[c_idx];
            let d_edge = cm_sq_seg_dist(c, a, b);

            // Candidates are sorted; once we exceed max distance, stop.
            if d_edge > max_sq_len {
                break;
            }

            // Adjacency constraint: c must be closer to edge (a,b) than to
            // either neighbouring edge, to avoid a sawtooth boundary.
            if d_edge >= cm_sq_seg_dist(c, prev_pt, a) {
                continue;
            }
            if d_edge >= cm_sq_seg_dist(c, b, next_next_pt) {
                continue;
            }

            // Self-intersection guards for the two proposed new edges.
            if !cm_no_intersections(a_idx, c_idx, &nodes, pts, eps) {
                continue;
            }
            if !cm_no_intersections(b_idx, c_idx, &nodes, pts, eps) {
                continue;
            }

            // Final endpoint-distance check (mirrors the JS main-loop guard).
            if dist2(c, a).min(dist2(c, b)) > max_sq_len {
                continue;
            }

            chosen = Some(c_idx);
            break;
        }

        if let Some(c_idx) = chosen {
            // Insert c between a_pos and b_pos in the linked list.
            let new_pos = nodes.len();
            nodes.push(ConcaveNode {
                point_idx: c_idx,
                prev: a_pos,
                next: b_pos,
            });
            nodes[a_pos].next = new_pos;
            nodes[b_pos].prev = new_pos;

            on_hull[c_idx] = true;
            n_inserts += 1;

            // Enqueue both new edges for further potential refinement.
            queue.push_back(a_pos);   // edge a → c
            queue.push_back(new_pos); // edge c → b
        }
    }

    // ---- 6. Walk linked list and build output polygon ----
    let mut coords = Vec::new();
    let mut cur = 0usize;
    loop {
        coords.push(pts[nodes[cur].point_idx]);
        cur = nodes[cur].next;
        if cur == 0 {
            break;
        }
    }
    if coords.len() < 3 {
        return convex_hull(pts, eps);
    }
    coords.push(coords[0]); // close ring

    let poly = Polygon::new(LinearRing::new(coords), vec![]);
    postprocess_concave_output(Geometry::Polygon(poly), options)
}

/// Squared distance from point `p` to segment (`a`, `b`).
///
/// Ported from the concaveman JS `sqSegDist` helper.
fn cm_sq_seg_dist(p: Coord, a: Coord, b: Coord) -> f64 {
    let mut x = a.x;
    let mut y = a.y;
    let dx = b.x - x;
    let dy = b.y - y;
    let denom = dx * dx + dy * dy;
    if denom > 0.0 {
        let t = ((p.x - x) * dx + (p.y - y) * dy) / denom;
        if t >= 1.0 {
            x = b.x;
            y = b.y;
        } else if t > 0.0 {
            x += dx * t;
            y += dy * t;
        }
    }
    let qx = p.x - x;
    let qy = p.y - y;
    qx * qx + qy * qy
}

/// Returns `true` if the proposed segment (`from`, `to`) does not intersect
/// any existing edge of the concaveman hull linked list.
///
/// Edges that share a vertex with the proposed segment are excluded (they
/// cannot produce a crossing, only a shared endpoint).
fn cm_no_intersections(
    from_idx: usize,
    to_idx: usize,
    nodes: &[ConcaveNode],
    pts: &[Coord],
    eps: f64,
) -> bool {
    let from = pts[from_idx];
    let to = pts[to_idx];
    let mut cur = 0usize;
    loop {
        let next = nodes[cur].next;
        let u_idx = nodes[cur].point_idx;
        let v_idx = nodes[next].point_idx;
        // Only test edges that share no vertex with the proposed segment.
        if u_idx != from_idx && u_idx != to_idx && v_idx != from_idx && v_idx != to_idx {
            if segments_intersect_eps(from, to, pts[u_idx], pts[v_idx], eps) {
                return false;
            }
        }
        cur = next;
        if cur == 0 {
            break;
        }
    }
    true
}

/// Compute a pragmatic concave hull of all coordinates contained in a geometry.
pub fn concave_hull_geometry(geometry: &Geometry, max_edge_length: f64, epsilon: f64) -> Geometry {
    concave_hull_geometry_with_options(
        geometry,
        ConcaveHullOptions {
            max_edge_length,
            epsilon,
            ..Default::default()
        },
    )
}

/// Compute a pragmatic concave hull of all coordinates in `geometry` using advanced options.
pub fn concave_hull_geometry_with_options(
    geometry: &Geometry,
    options: ConcaveHullOptions,
) -> Geometry {
    let coords = collect_geometry_coords(geometry);
    concave_hull_with_options(&coords, options)
}

/// Compute a pragmatic concave hull of a point set after snapping under `precision`.
pub fn concave_hull_with_precision(
    coords: &[Coord],
    max_edge_length: f64,
    precision: PrecisionModel,
) -> Geometry {
    concave_hull_with_options(
        coords,
        ConcaveHullOptions {
            max_edge_length,
            epsilon: precision.epsilon(),
            precision: Some(precision),
            ..Default::default()
        },
    )
}

/// Compute a pragmatic concave hull of all coordinates in `geometry` after snapping under `precision`.
pub fn concave_hull_geometry_with_precision(
    geometry: &Geometry,
    max_edge_length: f64,
    precision: PrecisionModel,
) -> Geometry {
    concave_hull_geometry_with_options(
        geometry,
        ConcaveHullOptions {
            max_edge_length,
            epsilon: precision.epsilon(),
            precision: Some(precision),
            ..Default::default()
        },
    )
}

fn postprocess_concave_output(geometry: Geometry, options: ConcaveHullOptions) -> Geometry {
    let min_area = options.min_area.max(0.0);
    let mut polys = match geometry {
        Geometry::Polygon(poly) => vec![poly],
        Geometry::MultiPolygon(polys) => polys,
        other => return other,
    };

    if min_area > 0.0 {
        polys.retain(|poly| polygon_area(poly) >= min_area);
    }

    if polys.is_empty() {
        return Geometry::GeometryCollection(vec![]);
    }

    if !options.allow_disjoint && polys.len() > 1 {
        let best = polys
            .into_iter()
            .max_by(|a, b| polygon_area(a).total_cmp(&polygon_area(b)))
            .unwrap();
        return Geometry::Polygon(best);
    }

    geometry_from_polygons(polys)
}

fn walk_boundary_ring(
    start: usize,
    next: usize,
    adjacency: &[Vec<usize>],
    unused: &mut HashSet<u128>,
    points: &[Coord],
) -> Option<LineString> {
    let mut ring = vec![points[start], points[next]];
    let mut prev = start;
    let mut current = next;
    unused.remove(&pack_edge(start, next));

    loop {
        let neighbors = adjacency.get(current)?;
        if neighbors.len() < 2 {
            return None;
        }
        let candidate = if neighbors[0] == prev {
            neighbors[1]
        } else {
            neighbors[0]
        };

        if candidate == start {
            ring.push(points[start]);
            return Some(LineString::new(ring));
        }

        let edge = pack_edge(current, candidate);
        if !unused.contains(&edge) {
            return None;
        }
        unused.remove(&edge);
        ring.push(points[candidate]);
        prev = current;
        current = candidate;
    }
}

fn geometry_from_polygons(polys: Vec<Polygon>) -> Geometry {
    match polys.len() {
        0 => Geometry::GeometryCollection(vec![]),
        1 => Geometry::Polygon(polys.into_iter().next().unwrap()),
        _ => Geometry::MultiPolygon(polys),
    }
}

fn convex_hull_indices_sorted(points: &[Coord], eps: f64) -> Vec<usize> {
    if points.len() <= 1 {
        return (0..points.len()).collect();
    }

    let mut lower = Vec::<usize>::new();
    for i in 0..points.len() {
        while lower.len() >= 2 {
            let a = points[lower[lower.len() - 2]];
            let b = points[lower[lower.len() - 1]];
            let c = points[i];
            if cross(a, b, c) <= eps {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(i);
    }

    let mut upper = Vec::<usize>::new();
    for i in (0..points.len()).rev() {
        while upper.len() >= 2 {
            let a = points[upper[upper.len() - 2]];
            let b = points[upper[upper.len() - 1]];
            let c = points[i];
            if cross(a, b, c) <= eps {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(i);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn segment_param(a: Coord, b: Coord, p: Coord) -> f64 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let denom = vx * vx + vy * vy;
    if denom <= 0.0 {
        0.0
    } else {
        ((p.x - a.x) * vx + (p.y - a.y) * vy) / denom
    }
}

fn point_segment_distance(a: Coord, b: Coord, p: Coord) -> f64 {
    let t = segment_param(a, b, p).clamp(0.0, 1.0);
    let proj = Coord::xy(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    dist2(p, proj).sqrt()
}

fn candidate_is_inside_ring(candidate: usize, ring: &[usize], points: &[Coord], eps: f64) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut coords = Vec::with_capacity(ring.len() + 1);
    for &idx in ring {
        coords.push(points[idx]);
    }
    coords.push(points[ring[0]]);
    matches!(
        classify_point_in_ring_eps(points[candidate], &coords, eps),
        PointInRing::Inside | PointInRing::Boundary
    )
}

fn edge_insertion_is_valid(
    a_idx: usize,
    b_idx: usize,
    p_idx: usize,
    ring: &[usize],
    points: &[Coord],
    eps: f64,
) -> bool {
    let a = points[a_idx];
    let b = points[b_idx];
    let p = points[p_idx];

    for i in 0..ring.len() {
        let u_idx = ring[i];
        let v_idx = ring[(i + 1) % ring.len()];
        if u_idx == a_idx || u_idx == b_idx || v_idx == a_idx || v_idx == b_idx {
            continue;
        }
        if u_idx == p_idx || v_idx == p_idx {
            continue;
        }

        let u = points[u_idx];
        let v = points[v_idx];
        if segments_intersect_eps(a, p, u, v, eps) {
            return false;
        }
        if segments_intersect_eps(p, b, u, v, eps) {
            return false;
        }
    }
    true
}

fn effective_max_edge_length(points: &[Coord], options: ConcaveHullOptions) -> f64 {
    if let Some(ratio) = options.relative_edge_length_ratio {
        if ratio.is_finite() && ratio > 0.0 {
            let (min_x, min_y, max_x, max_y) = points.iter().fold(
                (points[0].x, points[0].y, points[0].x, points[0].y),
                |(min_x, min_y, max_x, max_y), p| {
                    (
                        min_x.min(p.x),
                        min_y.min(p.y),
                        max_x.max(p.x),
                        max_y.max(p.y),
                    )
                },
            );
            let dx = max_x - min_x;
            let dy = max_y - min_y;
            let diag = (dx * dx + dy * dy).sqrt();
            return ratio * diag;
        }
    }
    options.max_edge_length
}

fn polygon_area(poly: &Polygon) -> f64 {
    let mut area = ring_area(&poly.exterior.coords);
    for hole in &poly.holes {
        area -= ring_area(&hole.coords);
    }
    area.max(0.0)
}

fn ring_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    (0.5 * s).abs()
}

fn collect_geometry_coords(geometry: &Geometry) -> Vec<Coord> {
    fn push_ring_coords(out: &mut Vec<Coord>, ring: &LinearRing) {
        if ring.coords.is_empty() {
            return;
        }
        let end = ring.coords.len().saturating_sub(1);
        out.extend_from_slice(&ring.coords[..end]);
    }

    let mut out = Vec::<Coord>::new();
    match geometry {
        Geometry::Point(c) => out.push(*c),
        Geometry::LineString(ls) => out.extend_from_slice(&ls.coords),
        Geometry::Polygon(poly) => {
            push_ring_coords(&mut out, &poly.exterior);
            for hole in &poly.holes {
                push_ring_coords(&mut out, hole);
            }
        }
        Geometry::MultiPoint(pts) => out.extend_from_slice(pts),
        Geometry::MultiLineString(lines) => {
            for ls in lines {
                out.extend_from_slice(&ls.coords);
            }
        }
        Geometry::MultiPolygon(polys) => {
            for poly in polys {
                push_ring_coords(&mut out, &poly.exterior);
                for hole in &poly.holes {
                    push_ring_coords(&mut out, hole);
                }
            }
        }
        Geometry::GeometryCollection(geoms) => {
            for g in geoms {
                out.extend(collect_geometry_coords(g));
            }
        }
    }
    out
}

fn unique_sorted_points(coords: &[Coord], epsilon: f64) -> Vec<Coord> {
    let eps = normalized_eps(epsilon);
    let mut pts: Vec<Coord> = coords
        .iter()
        .copied()
        .filter(|c| c.x.is_finite() && c.y.is_finite())
        .collect();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
    pts.dedup_by(|a, b| (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps);
    pts
}

fn normalized_eps(epsilon: f64) -> f64 {
    if epsilon.is_finite() {
        epsilon.abs().max(1.0e-12)
    } else {
        1.0e-12
    }
}

fn cross(o: Coord, a: Coord, b: Coord) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn dist2(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn norm_edge(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn pack_edge(a: usize, b: usize) -> u128 {
    let (lo, hi) = norm_edge(a, b);
    ((lo as u128) << 64) | (hi as u128)
}

fn unpack_edge(edge: u128) -> (usize, usize) {
    ((edge >> 64) as usize, edge as usize)
}
