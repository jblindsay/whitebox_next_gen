//! Convex and concave hull utilities.
//!
//! The convex hull implementation uses Andrew's monotone chain and returns the
//! tightest enclosing geometry as a `Point`, `LineString`, or `Polygon`.
//!
//! The concave hull implementation is a pragmatic alpha-shape-style wrapper
//! over Delaunay triangulation. Triangles whose longest edge exceeds the user
//! threshold are discarded; the boundary of the remaining triangle union is then
//! reconstructed into polygon shells/holes.

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
    /// Delaunay triangle filtering + polygonization (current default).
    Delaunay,
    /// Convex-hull edge refinement inspired by concaveman-style workflows.
    FastRefine,
}

/// Configuration options for concave hull generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConcaveHullOptions {
    /// Concave hull backend algorithm.
    pub engine: ConcaveHullEngine,
    /// Maximum allowed edge length in kept Delaunay triangles.
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
            engine: ConcaveHullEngine::Delaunay,
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
    let mut packed_edges = Vec::<u128>::with_capacity(tri.triangles.len() * 3);
    let mut kept_triangles = 0usize;

    for t in &tri.triangles {
        let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
        let keep = edges.iter().all(|&(a, b)| dist2(tri.points[a], tri.points[b]) <= max_len2);
        if !keep {
            continue;
        }
        kept_triangles += 1;
        for &(a, b) in &edges {
            packed_edges.push(pack_edge(a, b));
        }
    }

    if kept_triangles == 0 {
        return convex_hull(&tri.points, eps);
    }

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
