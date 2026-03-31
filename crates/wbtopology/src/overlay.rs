//! Overlay face selection built on topology graph extraction.

use std::collections::{HashMap, HashSet};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::algorithms::segment::segments_intersect_eps;
use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::geom::{Coord, Envelope, Geometry, LineString, LinearRing, Polygon};
use crate::graph::TopologyGraph;
use crate::precision::PrecisionModel;
use crate::spatial_index::SpatialIndex;

#[cfg(feature = "parallel")]
const PARALLEL_MIN_FACES: usize = 256;
const OVERLAY_ALL_TINY_VERTEX_THRESHOLD: usize = 24;
const OVERLAY_ALL_HOLERICH_VERTEX_THRESHOLD: usize = 64;
const OVERLAY_ALL_HOLERICH_HOLES_THRESHOLD: usize = 6;

/// Polygon overlay operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayOp {
    /// Keep faces inside both A and B.
    Intersection,
    /// Keep faces inside either A or B.
    Union,
    /// Keep faces inside A and outside B.
    DifferenceAB,
    /// Keep faces inside exactly one of A or B.
    SymmetricDifference,
}

/// All dissolved overlay outputs for a polygon pair.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayOutputs {
    /// Dissolved intersection output.
    pub intersection: Vec<Polygon>,
    /// Dissolved union output.
    pub union: Vec<Polygon>,
    /// Dissolved difference `A \ B` output.
    pub difference_ab: Vec<Polygon>,
    /// Dissolved symmetric difference output.
    pub sym_diff: Vec<Polygon>,
}

/// One dissolved polygon plus the contributing input polygon indices.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryDissolveGroup {
    /// Dissolved polygon geometry.
    pub poly: Polygon,
    /// Indices of source polygons from the input slice.
    pub source_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct ClassifiedFaces {
    rings: Vec<LineString>,
    in_a: Vec<bool>,
    in_b: Vec<bool>,
}

/// Select bounded arrangement faces for two polygons under `operation`.
///
/// This returns a face decomposition (not yet dissolved/merged).
pub fn polygon_overlay_faces(
    a: &Polygon,
    b: &Polygon,
    operation: OverlayOp,
    epsilon: f64,
) -> Vec<Polygon> {
    let eps = normalized_eps(epsilon);
    let classified = classify_overlay_faces(a, b, eps);
    select_classified_faces(&classified, operation)
}

fn classify_overlay_faces(a: &Polygon, b: &Polygon, eps: f64) -> ClassifiedFaces {
    let mut boundaries = polygon_boundaries(a);
    boundaries.extend(polygon_boundaries(b));
    let graph = TopologyGraph::from_linestrings(&boundaries, eps);
    let rings = graph.extract_face_rings(eps);

    #[cfg(feature = "parallel")]
    {
        if rings.len() >= PARALLEL_MIN_FACES {
            let flags: Vec<(bool, bool)> = rings
                .par_iter()
                .map(|ring| {
                    let probe = face_probe_point(ring, eps);
                    let in_a = face_in_polygon_with_probe(ring, probe, a, eps);
                    let in_b = face_in_polygon_with_probe(ring, probe, b, eps);
                    (in_a, in_b)
                })
                .collect();

            let mut in_a = Vec::with_capacity(flags.len());
            let mut in_b = Vec::with_capacity(flags.len());
            for (a_flag, b_flag) in flags {
                in_a.push(a_flag);
                in_b.push(b_flag);
            }

            return ClassifiedFaces { rings, in_a, in_b };
        }
    }

    let mut in_a = Vec::with_capacity(rings.len());
    let mut in_b = Vec::with_capacity(rings.len());
    for ring in &rings {
        let probe = face_probe_point(ring, eps);
        in_a.push(face_in_polygon_with_probe(ring, probe, a, eps));
        in_b.push(face_in_polygon_with_probe(ring, probe, b, eps));
    }

    ClassifiedFaces { rings, in_a, in_b }
}

fn select_classified_faces(classified: &ClassifiedFaces, operation: OverlayOp) -> Vec<Polygon> {
    let n = classified.rings.len();
    let mut out = Vec::<Polygon>::new();
    out.reserve(n / 2);

    for idx in 0..n {
        let a = classified.in_a[idx];
        let b = classified.in_b[idx];
        let keep = match operation {
            OverlayOp::Intersection => a && b,
            OverlayOp::Union => a || b,
            OverlayOp::DifferenceAB => a && !b,
            OverlayOp::SymmetricDifference => a ^ b,
        };
        if keep {
            out.push(Polygon::new(
                LinearRing::new(classified.rings[idx].coords.clone()),
                vec![],
            ));
        }
    }

    out
}

fn face_probe_point(face_ring: &LineString, eps: f64) -> Option<Coord> {
    if face_ring.coords.len() < 2 {
        return None;
    }

    let delta = (eps * 16.0).max(1.0e-9);
    for i in 0..(face_ring.coords.len() - 1) {
        let a = face_ring.coords[i];
        let b = face_ring.coords[i + 1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= eps {
            continue;
        }

        let mx = 0.5 * (a.x + b.x);
        let my = 0.5 * (a.y + b.y);
        let nx = -dy / len;
        let ny = dx / len;
        return Some(Coord::xy(mx + nx * delta, my + ny * delta));
    }

    None
}

fn face_in_polygon_with_probe(face_ring: &LineString, probe: Option<Coord>, poly: &Polygon, eps: f64) -> bool {
    if let Some(p) = probe {
        return matches!(classify_point_in_polygon_eps(p, poly, eps), PointInRing::Inside);
    }
    face_inside_polygon(face_ring, poly, eps)
}

/// Face-decomposed polygon intersection.
#[inline]
pub fn polygon_intersection_faces(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay_faces(a, b, OverlayOp::Intersection, epsilon)
}

/// Face-decomposed polygon union.
#[inline]
pub fn polygon_union_faces(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay_faces(a, b, OverlayOp::Union, epsilon)
}

/// Face-decomposed polygon difference `A \ B`.
#[inline]
pub fn polygon_difference_faces(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay_faces(a, b, OverlayOp::DifferenceAB, epsilon)
}

/// Face-decomposed polygon symmetric difference.
#[inline]
pub fn polygon_sym_diff_faces(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay_faces(a, b, OverlayOp::SymmetricDifference, epsilon)
}

/// Dissolved polygon overlay output for an operation.
///
/// This merges adjacent selected faces by canceling shared interior boundaries.
pub fn polygon_overlay(a: &Polygon, b: &Polygon, operation: OverlayOp, epsilon: f64) -> Vec<Polygon> {
    let eps = normalized_eps(epsilon);
    if let Some(result) = containment_overlay(a, b, operation, eps) {
        return normalize_polygons(result, eps);
    }
    let faces = polygon_overlay_faces(a, b, operation, eps);
    normalize_polygons(dissolve_faces(&faces, eps), eps)
}

/// Precision-aware dissolved polygon overlay output for an operation.
///
/// Inputs are snapped to the provided precision model before overlay processing.
pub fn polygon_overlay_with_precision(
    a: &Polygon,
    b: &Polygon,
    operation: OverlayOp,
    precision: PrecisionModel,
) -> Vec<Polygon> {
    let sa = precision.apply_polygon(a);
    let sb = precision.apply_polygon(b);
    polygon_overlay(&sa, &sb, operation, precision.epsilon())
}

/// Compute all dissolved polygon overlay outputs in one pass.
///
/// This reuses a single arrangement/face classification to derive all operations.
pub fn polygon_overlay_all(a: &Polygon, b: &Polygon, epsilon: f64) -> OverlayOutputs {
    let eps = normalized_eps(epsilon);

    if prefer_separate_overlay_all(a, b) {
        return OverlayOutputs {
            intersection: polygon_intersection(a, b, eps),
            union: polygon_union(a, b, eps),
            difference_ab: polygon_difference(a, b, eps),
            sym_diff: polygon_sym_diff(a, b, eps),
        };
    }

    // Reuse containment fast path logic through existing per-op API.
    if containment_overlay(a, b, OverlayOp::Intersection, eps).is_some()
        || containment_overlay(a, b, OverlayOp::Union, eps).is_some()
    {
        return OverlayOutputs {
            intersection: polygon_intersection(a, b, eps),
            union: polygon_union(a, b, eps),
            difference_ab: polygon_difference(a, b, eps),
            sym_diff: polygon_sym_diff(a, b, eps),
        };
    }

    // If boundaries do not cross, per-op paths are typically cheaper than
    // constructing and classifying a full shared arrangement.
    if !polygon_boundaries_cross(a, b, eps) {
        return OverlayOutputs {
            intersection: polygon_intersection(a, b, eps),
            union: polygon_union(a, b, eps),
            difference_ab: polygon_difference(a, b, eps),
            sym_diff: polygon_sym_diff(a, b, eps),
        };
    }

    let classified = classify_overlay_faces(a, b, eps);
    let inter_faces = select_classified_faces(&classified, OverlayOp::Intersection);
    let union_faces = select_classified_faces(&classified, OverlayOp::Union);
    let diff_faces = select_classified_faces(&classified, OverlayOp::DifferenceAB);
    let xor_faces = select_classified_faces(&classified, OverlayOp::SymmetricDifference);

    OverlayOutputs {
        intersection: normalize_polygons(dissolve_faces(&inter_faces, eps), eps),
        union: normalize_polygons(dissolve_faces(&union_faces, eps), eps),
        difference_ab: normalize_polygons(dissolve_faces(&diff_faces, eps), eps),
        sym_diff: normalize_polygons(dissolve_faces(&xor_faces, eps), eps),
    }
}

/// Precision-aware one-pass dissolved polygon overlay outputs.
///
/// Inputs are snapped to the provided precision model before overlay processing.
pub fn polygon_overlay_all_with_precision(
    a: &Polygon,
    b: &Polygon,
    precision: PrecisionModel,
) -> OverlayOutputs {
    let sa = precision.apply_polygon(a);
    let sb = precision.apply_polygon(b);
    polygon_overlay_all(&sa, &sb, precision.epsilon())
}

/// Dissolved polygon intersection.
#[inline]
pub fn polygon_intersection(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay(a, b, OverlayOp::Intersection, epsilon)
}

/// Precision-aware dissolved polygon intersection.
#[inline]
pub fn polygon_intersection_with_precision(
    a: &Polygon,
    b: &Polygon,
    precision: PrecisionModel,
) -> Vec<Polygon> {
    polygon_overlay_with_precision(a, b, OverlayOp::Intersection, precision)
}

/// Dissolved polygon union.
#[inline]
pub fn polygon_union(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay(a, b, OverlayOp::Union, epsilon)
}

/// Dissolve many polygons into non-overlapping groups.
///
/// The result includes both dissolved geometry and source membership so callers
/// can aggregate attributes at the application layer.
pub fn polygon_unary_dissolve(polys: &[Polygon], epsilon: f64) -> Vec<UnaryDissolveGroup> {
    if polys.is_empty() {
        return Vec::new();
    }

    if polys.len() == 1 {
        return vec![UnaryDissolveGroup {
            poly: polys[0].clone(),
            source_indices: vec![0],
        }];
    }

    let eps = normalized_eps(epsilon);
    let components = envelope_connected_components(polys);

    #[cfg(feature = "parallel")]
    {
        if components.len() >= 4 {
            return components
                .par_iter()
                .map(|comp| dissolve_component(polys, comp, eps))
                .flatten()
                .collect();
        }
    }

    let mut out = Vec::<UnaryDissolveGroup>::new();
    for comp in components {
        out.extend(dissolve_component(polys, &comp, eps));
    }
    out
}

fn envelope_connected_components(polys: &[Polygon]) -> Vec<Vec<usize>> {
    if polys.is_empty() {
        return Vec::new();
    }

    let geoms: Vec<Geometry> = polys.iter().cloned().map(Geometry::Polygon).collect();
    let index = SpatialIndex::from_geometries(&geoms);

    let mut visited = vec![false; polys.len()];
    let mut comps = Vec::<Vec<usize>>::new();

    for start in 0..polys.len() {
        if visited[start] {
            continue;
        }

        let mut stack = vec![start];
        visited[start] = true;
        let mut comp = Vec::<usize>::new();

        while let Some(i) = stack.pop() {
            comp.push(i);
            let neighbors = index.query_geometry(&geoms[i]);
            for n in neighbors {
                if n >= polys.len() || visited[n] {
                    continue;
                }
                visited[n] = true;
                stack.push(n);
            }
        }

        comps.push(comp);
    }

    comps
}

fn dissolve_component(polys: &[Polygon], component: &[usize], eps: f64) -> Vec<UnaryDissolveGroup> {
    #[derive(Debug, Clone)]
    struct Work {
        poly: Polygon,
        envelope: Option<Envelope>,
        area: f64,
        members: Vec<usize>,
    }

    if component.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<Work> = component
        .iter()
        .copied()
        .map(|idx| Work {
            poly: polys[idx].clone(),
            envelope: polys[idx].envelope(),
            area: polygon_abs_area(&polys[idx]),
            members: vec![idx],
        })
        .collect();

    if groups.len() < 2 {
        return groups
            .into_iter()
            .map(|g| UnaryDissolveGroup {
                poly: g.poly,
                source_indices: g.members,
            })
            .collect();
    }

    loop {
        let mut merged_any = false;

        'scan: for i in 0..groups.len() {
            let env_i = groups[i].envelope;

            #[cfg(feature = "parallel")]
            {
                // When one giant connected component dominates, component-level
                // parallelism is ineffective. In that case, parallelize partner
                // search for this seed polygon.
                if groups.len() >= 64 {
                    let best = ((i + 1)..groups.len())
                        .into_par_iter()
                        .filter_map(|j| {
                            if let (Some(a), Some(b)) = (env_i, groups[j].envelope) {
                                if !a.intersects(&b) {
                                    return None;
                                }
                            }

                            safe_dissolve_union(
                                &groups[i].poly,
                                groups[i].area,
                                &groups[j].poly,
                                groups[j].area,
                                eps,
                            )
                                .map(|poly| (j, poly))
                        })
                        .reduce_with(|a, b| if a.0 < b.0 { a } else { b });

                    if let Some((j, merged_poly)) = best {
                        groups[i].poly = merged_poly;
                        groups[i].envelope = groups[i].poly.envelope();
                        groups[i].area = polygon_abs_area(&groups[i].poly);
                        let mut other = groups.remove(j);
                        groups[i].members.append(&mut other.members);
                        merged_any = true;
                        break 'scan;
                    }
                    // No merge partner for this i found in parallel path.
                    continue;
                }
            }

            for j in (i + 1)..groups.len() {
                if let (Some(a), Some(b)) = (env_i, groups[j].envelope) {
                    if !a.intersects(&b) {
                        continue;
                    }
                }

                if let Some(merged_poly) = safe_dissolve_union(
                    &groups[i].poly,
                    groups[i].area,
                    &groups[j].poly,
                    groups[j].area,
                    eps,
                ) {
                    groups[i].poly = merged_poly;
                    groups[i].envelope = groups[i].poly.envelope();
                    groups[i].area = polygon_abs_area(&groups[i].poly);
                    let mut other = groups.remove(j);
                    groups[i].members.append(&mut other.members);
                    merged_any = true;
                    break 'scan;
                }
            }
        }

        if !merged_any {
            break;
        }
    }

    groups
        .into_iter()
        .map(|mut g| {
            g.members.sort_unstable();
            UnaryDissolveGroup {
                poly: g.poly,
                source_indices: g.members,
            }
        })
        .collect()
}

    fn safe_dissolve_union(a: &Polygon, area_a: f64, b: &Polygon, area_b: f64, eps: f64) -> Option<Polygon> {
        let quick_eps = eps.max(1.0e-9);
        if !polygon_boundaries_cross(a, b, quick_eps)
            && !shell_strictly_inside(a, b, quick_eps)
            && !shell_strictly_inside(b, a, quick_eps)
        {
            return None;
        }

    let min_expected = area_a.max(area_b);
    let area_tol = eps.max(1.0e-9) * 10.0;

    let union = polygon_union(a, b, eps);
    if union.len() == 1 {
        let poly = union[0].clone();
        if union_candidate_is_valid(&poly, a, b, eps, area_tol, min_expected) {
            return Some(poly);
        }
    }

    for scale in [10_000.0, 1_000.0, 100.0] {
        let precision = PrecisionModel::Fixed { scale };
        let union = polygon_union_with_precision(a, b, precision);
        if union.len() != 1 {
            continue;
        }

        let poly = union[0].clone();
        let tol = area_tol.max(1.0 / scale);
        if union_candidate_is_valid_with_precision(&poly, a, b, eps, precision, tol, min_expected) {
            return Some(poly);
        }
    }

    None
}

fn union_candidate_is_valid(
    candidate: &Polygon,
    a: &Polygon,
    b: &Polygon,
    eps: f64,
    area_tol: f64,
    min_expected: f64,
) -> bool {
    if polygon_abs_area(candidate) + area_tol < min_expected {
        return false;
    }

    let validate_eps = eps.max(1.0e-7);
    polygon_shell_is_covered(candidate, a, validate_eps)
        && polygon_shell_is_covered(candidate, b, validate_eps)
}

fn union_candidate_is_valid_with_precision(
    candidate: &Polygon,
    a: &Polygon,
    b: &Polygon,
    eps: f64,
    precision: PrecisionModel,
    area_tol: f64,
    min_expected: f64,
) -> bool {
    if polygon_abs_area(candidate) + area_tol < min_expected {
        return false;
    }

    let validate_eps = match precision {
        PrecisionModel::Fixed { scale } => eps.max(0.5 / scale),
        _ => eps.max(1.0e-7),
    };
    polygon_shell_is_covered(candidate, a, validate_eps)
        && polygon_shell_is_covered(candidate, b, validate_eps)
}

fn polygon_shell_is_covered(container: &Polygon, source: &Polygon, eps: f64) -> bool {
    let coords = &source.exterior.coords;
    if coords.len() < 2 {
        return false;
    }

    for i in 0..(coords.len() - 1) {
        let a = coords[i];
        let b = coords[i + 1];
        let mid = Coord::xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);

        if matches!(classify_point_in_polygon_eps(a, container, eps), PointInRing::Outside) {
            return false;
        }
        if matches!(classify_point_in_polygon_eps(mid, container, eps), PointInRing::Outside) {
            return false;
        }
    }

    true
}

/// Precision-aware dissolved polygon union.
#[inline]
pub fn polygon_union_with_precision(
    a: &Polygon,
    b: &Polygon,
    precision: PrecisionModel,
) -> Vec<Polygon> {
    polygon_overlay_with_precision(a, b, OverlayOp::Union, precision)
}

/// Dissolved polygon difference `A \ B`.
#[inline]
pub fn polygon_difference(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay(a, b, OverlayOp::DifferenceAB, epsilon)
}

/// Precision-aware dissolved polygon difference `A \ B`.
#[inline]
pub fn polygon_difference_with_precision(
    a: &Polygon,
    b: &Polygon,
    precision: PrecisionModel,
) -> Vec<Polygon> {
    polygon_overlay_with_precision(a, b, OverlayOp::DifferenceAB, precision)
}

/// Dissolved polygon symmetric difference.
#[inline]
pub fn polygon_sym_diff(a: &Polygon, b: &Polygon, epsilon: f64) -> Vec<Polygon> {
    polygon_overlay(a, b, OverlayOp::SymmetricDifference, epsilon)
}

/// Precision-aware dissolved polygon symmetric difference.
#[inline]
pub fn polygon_sym_diff_with_precision(
    a: &Polygon,
    b: &Polygon,
    precision: PrecisionModel,
) -> Vec<Polygon> {
    polygon_overlay_with_precision(a, b, OverlayOp::SymmetricDifference, precision)
}

fn containment_overlay(a: &Polygon, b: &Polygon, operation: OverlayOp, eps: f64) -> Option<Vec<Polygon>> {
    let a_contains_b = shell_strictly_inside(a, b, eps);
    let b_contains_a = shell_strictly_inside(b, a, eps);

    if !a_contains_b && !b_contains_a {
        return None;
    }

    let result = match operation {
        OverlayOp::Intersection => {
            if a_contains_b {
                vec![b.clone()]
            } else {
                vec![a.clone()]
            }
        }
        OverlayOp::Union => {
            if a_contains_b {
                vec![a.clone()]
            } else {
                vec![b.clone()]
            }
        }
        OverlayOp::DifferenceAB => {
            if a_contains_b {
                subtract_contained(a, b)
            } else {
                Vec::new()
            }
        }
        OverlayOp::SymmetricDifference => {
            if a_contains_b {
                subtract_contained(a, b)
            } else {
                subtract_contained(b, a)
            }
        }
    };

    Some(result)
}

fn shell_strictly_inside(container: &Polygon, candidate: &Polygon, eps: f64) -> bool {
    let mut container_rings: Vec<&[Coord]> = Vec::with_capacity(1 + container.holes.len());
    container_rings.push(&container.exterior.coords);
    for h in &container.holes {
        container_rings.push(&h.coords);
    }

    // Any boundary crossing/touching with candidate shell invalidates strict containment.
    for ring in &container_rings {
        if ring_boundary_intersects_eps(ring, &candidate.exterior.coords, eps) {
            return false;
        }
    }

    // Candidate shell vertices and segment midpoints must lie strictly inside container set.
    let c = &candidate.exterior.coords;
    if c.len() < 4 {
        return false;
    }

    for i in 0..(c.len() - 1) {
        let p = c[i];
        if !matches!(classify_point_in_polygon_eps(p, container, eps), PointInRing::Inside) {
            return false;
        }

        let q = c[i + 1];
        let m = Coord::xy((p.x + q.x) * 0.5, (p.y + q.y) * 0.5);
        if !matches!(classify_point_in_polygon_eps(m, container, eps), PointInRing::Inside) {
            return false;
        }
    }

    true
}

fn ring_boundary_intersects_eps(a: &[Coord], b: &[Coord], eps: f64) -> bool {
    if a.len() < 2 || b.len() < 2 {
        return false;
    }

    for i in 0..(a.len() - 1) {
        let a1 = a[i];
        let a2 = a[i + 1];
        for j in 0..(b.len() - 1) {
            let b1 = b[j];
            let b2 = b[j + 1];
            if segments_intersect_eps(a1, a2, b1, b2, eps) {
                return true;
            }
        }
    }
    false
}

fn polygon_boundaries_cross(a: &Polygon, b: &Polygon, eps: f64) -> bool {
    let mut a_rings: Vec<&[Coord]> = Vec::with_capacity(1 + a.holes.len());
    a_rings.push(&a.exterior.coords);
    for h in &a.holes {
        a_rings.push(&h.coords);
    }

    let mut b_rings: Vec<&[Coord]> = Vec::with_capacity(1 + b.holes.len());
    b_rings.push(&b.exterior.coords);
    for h in &b.holes {
        b_rings.push(&h.coords);
    }

    for ra in &a_rings {
        for rb in &b_rings {
            if ring_boundary_intersects_eps(ra, rb, eps) {
                return true;
            }
        }
    }

    false
}

fn prefer_separate_overlay_all(a: &Polygon, b: &Polygon) -> bool {
    let vertices = boundary_vertex_count(a) + boundary_vertex_count(b);
    if vertices <= OVERLAY_ALL_TINY_VERTEX_THRESHOLD {
        return true;
    }

    let holes = a.holes.len() + b.holes.len();
    holes >= OVERLAY_ALL_HOLERICH_HOLES_THRESHOLD
        && vertices <= OVERLAY_ALL_HOLERICH_VERTEX_THRESHOLD
}

fn boundary_vertex_count(poly: &Polygon) -> usize {
    let mut n = poly.exterior.coords.len();
    for h in &poly.holes {
        n += h.coords.len();
    }
    n
}

fn subtract_contained(container: &Polygon, contained: &Polygon) -> Vec<Polygon> {
    let mut out = Vec::<Polygon>::new();

    // Main shell: container shell minus contained shell.
    let mut holes = container.holes.clone();
    holes.push(contained.exterior.clone());
    out.push(Polygon::new(container.exterior.clone(), holes));

    // Holes inside the contained polygon represent additive islands.
    for h in &contained.holes {
        out.push(Polygon::new(h.clone(), vec![]));
    }

    out
}

fn normalized_eps(epsilon: f64) -> f64 {
    if epsilon.is_finite() {
        epsilon.abs().max(1.0e-12)
    } else {
        1.0e-12
    }
}

fn polygon_abs_area(poly: &Polygon) -> f64 {
    let mut area = ring_signed_area(&poly.exterior.coords).abs();
    for h in &poly.holes {
        area -= ring_signed_area(&h.coords).abs();
    }
    area.max(0.0)
}

fn polygon_boundaries(poly: &Polygon) -> Vec<LineString> {
    let mut out = Vec::with_capacity(1 + poly.holes.len());
    out.push(LineString::new(poly.exterior.coords.clone()));
    for hole in &poly.holes {
        out.push(LineString::new(hole.coords.clone()));
    }
    out
}

fn face_inside_polygon(face_ring: &LineString, poly: &Polygon, eps: f64) -> bool {
    if face_ring.coords.len() < 4 {
        return false;
    }

    let mut candidates = Vec::<Coord>::with_capacity(face_ring.coords.len() + 1);
    if let Some(c) = ring_centroid(&face_ring.coords) {
        candidates.push(c);
    }
    for i in 0..(face_ring.coords.len() - 1) {
        let a = face_ring.coords[i];
        let b = face_ring.coords[i + 1];
        candidates.push(Coord::xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5));
    }

    let mut saw_outside = false;
    let mut saw_inside = false;
    for p in candidates {
        match classify_point_in_polygon_eps(p, poly, eps) {
            PointInRing::Inside => saw_inside = true,
            PointInRing::Outside => saw_outside = true,
            PointInRing::Boundary => {}
        }
    }

    if saw_inside {
        true
    } else if saw_outside {
        false
    } else {
        // Degenerate fallback: if all probes are on boundary, keep as inside.
        true
    }
}

fn classify_point_in_polygon_eps(p: Coord, poly: &Polygon, eps: f64) -> PointInRing {
    match classify_point_in_ring_eps(p, &poly.exterior.coords, eps) {
        PointInRing::Outside => return PointInRing::Outside,
        PointInRing::Boundary => return PointInRing::Boundary,
        PointInRing::Inside => {}
    }

    for hole in &poly.holes {
        match classify_point_in_ring_eps(p, &hole.coords, eps) {
            PointInRing::Inside => return PointInRing::Outside,
            PointInRing::Boundary => return PointInRing::Boundary,
            PointInRing::Outside => {}
        }
    }

    PointInRing::Inside
}

fn ring_centroid(coords: &[Coord]) -> Option<Coord> {
    if coords.len() < 4 {
        return None;
    }

    let mut a2 = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;

    for i in 0..(coords.len() - 1) {
        let p = coords[i];
        let q = coords[i + 1];
        let cross = p.x * q.y - q.x * p.y;
        a2 += cross;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }

    if a2.abs() <= 1.0e-18 {
        return None;
    }

    let inv = 1.0 / (3.0 * a2);
    Some(Coord::xy(cx * inv, cy * inv))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct QCoord(i64, i64);

#[derive(Debug, Clone, Copy)]
struct SegState {
    count: usize,
}

fn dissolve_faces(faces: &[Polygon], eps: f64) -> Vec<Polygon> {
    let mut seg_counts = HashMap::<(QCoord, QCoord), SegState>::new();
    let mut coord_map = HashMap::<QCoord, Coord>::new();

    for poly in faces {
        let c = &poly.exterior.coords;
        if c.len() < 2 {
            continue;
        }
        for i in 0..(c.len() - 1) {
            let a = c[i];
            let b = c[i + 1];
            let qa = quantize_coord(a, eps);
            let qb = quantize_coord(b, eps);
            if qa == qb {
                continue;
            }

            coord_map.entry(qa).or_insert(a);
            coord_map.entry(qb).or_insert(b);
            let key = ordered_pair(qa, qb);
            seg_counts
                .entry(key)
                .and_modify(|s| s.count += 1)
                .or_insert(SegState { count: 1 });
        }
    }

    let mut adjacency = HashMap::<QCoord, Vec<QCoord>>::new();
    let mut boundary_edges = HashSet::<(QCoord, QCoord)>::new();

    for (key, state) in seg_counts {
        if state.count % 2 == 0 {
            continue;
        }
        let (a, b) = key;
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
        boundary_edges.insert(key);
    }

    for (node, neighbors) in &mut adjacency {
        neighbors.sort_by(|na, nb| {
            let aa = edge_angle_q(*node, *na, &coord_map);
            let ab = edge_angle_q(*node, *nb, &coord_map);
            aa.total_cmp(&ab)
        });
    }

    let mut rings = Vec::<Vec<Coord>>::new();

    while let Some(&(a, b)) = boundary_edges.iter().next() {
        let mut ring_keys = vec![a, b];
        boundary_edges.remove(&(a, b));

        let start = a;
        let mut prev = a;
        let mut curr = b;
        let mut closed = false;

        for _ in 0..(adjacency.len() * 4).max(16) {
            if curr == start {
                closed = true;
                break;
            }

            let Some(next) = choose_next_boundary_neighbor(
                curr,
                prev,
                &adjacency,
                &boundary_edges,
            ) else {
                break;
            };

            ring_keys.push(next);
            boundary_edges.remove(&ordered_pair(curr, next));
            prev = curr;
            curr = next;
        }

        if !closed || ring_keys.len() < 4 {
            continue;
        }

        let mut coords: Vec<Coord> = ring_keys
            .iter()
            .filter_map(|k| coord_map.get(k).copied())
            .collect();

        if coords.len() < 4 {
            continue;
        }

        if !nearly_eq(coords[0], *coords.last().unwrap_or(&coords[0]), eps) {
            coords.push(coords[0]);
        }

        if ring_signed_area(&coords).abs() <= eps * eps {
            continue;
        }

        rings.push(coords);
    }

    assemble_polygons_from_rings(rings, eps)
}

fn assemble_polygons_from_rings(rings: Vec<Vec<Coord>>, eps: f64) -> Vec<Polygon> {
    if rings.is_empty() {
        return Vec::new();
    }

    let n = rings.len();
    let areas: Vec<f64> = rings.iter().map(|r| ring_signed_area(r).abs()).collect();

    let mut parent: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let mut best_parent: Option<usize> = None;
        let mut best_area = f64::INFINITY;

        for j in 0..n {
            if i == j {
                continue;
            }
            if areas[j] <= areas[i] + eps * eps {
                continue;
            }
            if ring_contains_ring(&rings[j], &rings[i], eps) && areas[j] < best_area {
                best_parent = Some(j);
                best_area = areas[j];
            }
        }

        parent[i] = best_parent;
    }

    let mut depth = vec![0usize; n];
    for i in 0..n {
        let mut d = 0usize;
        let mut p = parent[i];
        while let Some(pi) = p {
            d += 1;
            p = parent[pi];
            if d > n {
                break;
            }
        }
        depth[i] = d;
    }

    let mut shell_indices = Vec::<usize>::new();
    for i in 0..n {
        if depth[i] % 2 == 0 {
            shell_indices.push(i);
        }
    }

    let mut holes_by_shell: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        if depth[i] % 2 == 0 {
            continue;
        }
        let mut p = parent[i];
        while let Some(pi) = p {
            if depth[pi] % 2 == 0 {
                holes_by_shell.entry(pi).or_default().push(i);
                break;
            }
            p = parent[pi];
        }
    }

    let mut polygons = Vec::<Polygon>::new();
    for shell_idx in shell_indices {
        let mut shell_coords = rings[shell_idx].clone();
        if ring_signed_area(&shell_coords) < 0.0 {
            shell_coords.reverse();
        }

        let exterior = LinearRing::new(shell_coords);
        let mut holes = Vec::<LinearRing>::new();

        if let Some(hole_idxs) = holes_by_shell.get(&shell_idx) {
            for hi in hole_idxs {
                let mut hole_coords = rings[*hi].clone();
                if ring_signed_area(&hole_coords) > 0.0 {
                    hole_coords.reverse();
                }
                holes.push(LinearRing::new(hole_coords));
            }
        }

        polygons.push(Polygon::new(exterior, holes));
    }

    polygons
}

fn ring_contains_ring(container: &[Coord], candidate: &[Coord], eps: f64) -> bool {
    if container.len() < 4 || candidate.len() < 4 {
        return false;
    }

    let mut saw_inside = false;
    let mut saw_outside = false;
    for p in &candidate[..candidate.len() - 1] {
        match classify_point_in_ring_eps(*p, container, eps) {
            PointInRing::Inside => saw_inside = true,
            PointInRing::Outside => saw_outside = true,
            PointInRing::Boundary => {}
        }

        if saw_outside {
            return false;
        }
    }

    if saw_inside {
        return true;
    }

    // Fallback for boundary-coincident vertices: classify candidate centroid.
    if let Some(c) = ring_centroid(candidate) {
        return matches!(classify_point_in_ring_eps(c, container, eps), PointInRing::Inside);
    }

    false
}

fn ordered_pair(a: QCoord, b: QCoord) -> (QCoord, QCoord) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn quantize_coord(c: Coord, eps: f64) -> QCoord {
    let qx = (c.x / eps).round() as i64;
    let qy = (c.y / eps).round() as i64;
    QCoord(qx, qy)
}

fn nearly_eq(a: Coord, b: Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps
}

fn ring_signed_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    0.5 * s
}

fn normalize_polygons(mut polys: Vec<Polygon>, eps: f64) -> Vec<Polygon> {
    for p in &mut polys {
        normalize_polygon(p, eps);
    }

    polys.sort_by(|a, b| polygon_sort_key(a).cmp(&polygon_sort_key(b)));
    polys
}

fn normalize_polygon(poly: &mut Polygon, eps: f64) {
    normalize_exterior_ring(&mut poly.exterior, eps);
    for h in &mut poly.holes {
        normalize_hole_ring(h, eps);
    }
    poly.holes
        .sort_by(|a, b| ring_sort_key(&a.coords).cmp(&ring_sort_key(&b.coords)));
}

fn normalize_exterior_ring(ring: &mut LinearRing, eps: f64) {
    if ring.coords.len() < 4 {
        return;
    }
    if ring_signed_area(&ring.coords) < -eps * eps {
        ring.coords.reverse();
    }
    canonicalize_ring_start(&mut ring.coords);
}

fn normalize_hole_ring(ring: &mut LinearRing, eps: f64) {
    if ring.coords.len() < 4 {
        return;
    }
    if ring_signed_area(&ring.coords) > eps * eps {
        ring.coords.reverse();
    }
    canonicalize_ring_start(&mut ring.coords);
}

fn canonicalize_ring_start(coords: &mut Vec<Coord>) {
    if coords.len() < 4 {
        return;
    }
    let n = coords.len() - 1;

    let mut min_idx = 0usize;
    for i in 1..n {
        if coord_lex_lt(coords[i], coords[min_idx]) {
            min_idx = i;
        }
    }

    if min_idx == 0 {
        return;
    }

    let mut out = Vec::with_capacity(coords.len());
    for k in 0..n {
        out.push(coords[(min_idx + k) % n]);
    }
    out.push(out[0]);
    *coords = out;
}

fn coord_lex_lt(a: Coord, b: Coord) -> bool {
    if a.x < b.x {
        true
    } else if a.x > b.x {
        false
    } else {
        a.y < b.y
    }
}

fn ring_sort_key(coords: &[Coord]) -> (u64, u64, usize) {
    let c0 = coords.first().copied().unwrap_or(Coord::xy(0.0, 0.0));
    (c0.x.to_bits(), c0.y.to_bits(), coords.len())
}

fn polygon_sort_key(poly: &Polygon) -> (u64, u64, usize, usize) {
    let c0 = poly
        .exterior
        .coords
        .first()
        .copied()
        .unwrap_or(Coord::xy(0.0, 0.0));
    (
        c0.x.to_bits(),
        c0.y.to_bits(),
        poly.exterior.coords.len(),
        poly.holes.len(),
    )
}

fn choose_next_boundary_neighbor(
    curr: QCoord,
    prev: QCoord,
    adjacency: &HashMap<QCoord, Vec<QCoord>>,
    boundary_edges: &HashSet<(QCoord, QCoord)>,
) -> Option<QCoord> {
    let neighbors = adjacency.get(&curr)?;
    if neighbors.is_empty() {
        return None;
    }

    if let Some(pos_back) = neighbors.iter().position(|n| *n == prev) {
        // Mirror the graph left-face rule: predecessor of back-edge in CCW order.
        for k in 1..=neighbors.len() {
            let idx = (pos_back + neighbors.len() - k) % neighbors.len();
            let cand = neighbors[idx];
            if boundary_edges.contains(&ordered_pair(curr, cand)) {
                return Some(cand);
            }
        }
    }

    neighbors
        .iter()
        .copied()
        .find(|n| boundary_edges.contains(&ordered_pair(curr, *n)))
}

fn edge_angle_q(from: QCoord, to: QCoord, coord_map: &HashMap<QCoord, Coord>) -> f64 {
    let Some(a) = coord_map.get(&from).copied() else {
        return 0.0;
    };
    let Some(b) = coord_map.get(&to).copied() else {
        return 0.0;
    };
    (b.y - a.y).atan2(b.x - a.x)
}
