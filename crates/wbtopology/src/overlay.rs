//! Overlay face selection built on topology graph extraction.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "parallel")]
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
#[cfg(feature = "parallel")]
use rayon::join;

use crate::algorithms::segment::{point_on_segment_eps, segments_intersect_eps};
use crate::algorithms::distance::geometry_distance;
use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::geom::{Coord, Envelope, Geometry, LineString, LinearRing, Polygon};
use crate::graph::TopologyGraph;
use crate::noding::{NodingOptions, NodingStrategy};
use crate::precision::PrecisionModel;
use crate::spatial_index::SpatialIndex;

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

/// Strategy used for unary polygon dissolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryDissolveStrategy {
    /// Use graph/face assembly and source-classification.
    GraphDriven,
    /// Use spatially cascaded pairwise dissolve passes.
    CascadedHeuristic,
    /// Use legacy envelope-component pairwise merge heuristic.
    PairwiseHeuristic,
}

/// Options for unary polygon dissolve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnaryDissolveOptions {
    /// Predicate epsilon.
    pub epsilon: f64,
    /// Dissolve strategy.
    pub strategy: UnaryDissolveStrategy,
    /// Noding options used by graph-driven dissolve.
    pub noding: NodingOptions,
    /// Optional preferred precision model for pairwise/cascaded union attempts.
    pub preferred_union_precision: Option<PrecisionModel>,
}

impl Default for UnaryDissolveOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.0e-9,
            strategy: UnaryDissolveStrategy::GraphDriven,
            noding: NodingOptions {
                epsilon: 1.0e-9,
                strategy: NodingStrategy::SnapRounding,
                precision: None,
            },
            preferred_union_precision: None,
        }
    }
}

#[derive(Debug, Clone)]
struct ClassifiedFaces {
    rings: Vec<LineString>,
    in_a: Vec<bool>,
    in_b: Vec<bool>,
}

/// Select bounded arrangement faces for two polygons under `operation`.
///
/// Returns a face decomposition from the planar arrangement—a flat list of simple rings
/// (exterior rings without holes). This is a low-level primitive useful for diagnostic
/// purposes or advanced use cases that need raw face geometry.
///
/// **Important:** The returned polygons are FLAT (no holes). Adjacent faces have not
/// been merged, and containment relationships between rings have not been resolved into
/// shell/hole hierarchy.
///
/// **Recommendation:** For typical Boolean overlay operations, use [`polygon_overlay`]
/// instead, which merges adjacent faces and properly reconstructs holes via hole nesting.
///
/// This function is primarily useful for:
/// - Visualizing or debugging the underlying face decomposition
/// - Computing intermediate results for custom overlay algorithms
/// - Workflows that explicitly need flat face rings
///
/// To convert the flat output of this function to proper polygons with holes (if you
/// need it), collect the rings and call `assemble_polygons_from_rings` (internal helper);
/// however, `polygon_overlay` already does this and is the recommended path.
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
    // Clamp to the topology-scale minimum so that ultra-fine caller epsilons
    // (e.g., 1e-12 from Sibson interpolation) do not produce precision loss
    // from excessively tight node-merge thresholds in the face graph.
    let graph_eps = eps.max(1.0e-9);
    let graph = TopologyGraph::from_linestrings(&boundaries, graph_eps);

    // Use bounded face rings only (positive signed area).  Edges in CW-winding
    // rings (outer boundary / hole inner-boundary cycles) are NOT mapped, so
    // their `sym` edges report `edge_to_face[sym] == MAX` — that is the signal
    // used by the BFS to identify faces adjacent to the exterior.
    let rings_with_edges = graph.extract_bounded_face_rings_with_edges(eps);
    let n_edges = graph.edges.len();

    // Map edge id → face index.
    let mut edge_to_face = vec![usize::MAX; n_edges];
    for (face_id, (_, edge_ids)) in rings_with_edges.iter().enumerate() {
        for &eid in edge_ids {
            if eid < n_edges {
                edge_to_face[eid] = face_id;
            }
        }
    }

    // Compute per-source depth deltas and classify via BFS — one pass per polygon.
    let depth_a = classify_overlay_faces_depth(&graph, &rings_with_edges, &edge_to_face, a, n_edges, eps);
    let depth_b = classify_overlay_faces_depth(&graph, &rings_with_edges, &edge_to_face, b, n_edges, eps);

    let in_a: Vec<bool> = depth_a.iter().map(|&d| d > 0).collect();
    let in_b: Vec<bool> = depth_b.iter().map(|&d| d > 0).collect();
    let rings: Vec<LineString> = rings_with_edges.into_iter().map(|(ls, _)| ls).collect();

    ClassifiedFaces { rings, in_a, in_b }
}

/// Compute depth labels for all faces relative to a single source polygon using BFS.
/// Returns one i32 per face; depth > 0 means the face is inside the source polygon.
fn classify_overlay_faces_depth(
    graph: &TopologyGraph,
    face_rings: &[(LineString, Vec<usize>)],
    edge_to_face: &[usize],
    source: &Polygon,
    n_edges: usize,
    eps: f64,
) -> Vec<i32> {
    // Compute per-directed-edge depth deltas from source ring segments.
    let mut delta = vec![0i32; n_edges];
    let src_slice = std::slice::from_ref(source);
    compute_unary_edge_deltas(graph, src_slice, eps, &mut delta);

    // BFS from exterior-adjacent faces, same as unary dissolve.
    let mut face_depth = vec![i32::MIN; face_rings.len()];
    let mut queue = VecDeque::<usize>::new();

    for face_id in 0..face_rings.len() {
        let mut seed = i32::MIN;
        let mut fallback_seed = i32::MIN;
        for &eid in &face_rings[face_id].1 {
            let sym = eid ^ 1;
            if sym < n_edges && edge_to_face[sym] == usize::MAX {
                let d = if eid < delta.len() { delta[eid] } else { 0 };
                if d != 0 && seed == i32::MIN {
                    seed = d;
                    break;
                }
                if fallback_seed == i32::MIN {
                    fallback_seed = d;
                }
            }
        }
        let chosen = if seed != i32::MIN { seed } else { fallback_seed };
        if chosen != i32::MIN {
            face_depth[face_id] = chosen;
            queue.push_back(face_id);
        }
    }

    while let Some(current_id) = queue.pop_front() {
        let d = face_depth[current_id];
        let edge_ids = face_rings[current_id].1.clone();
        for eid in edge_ids {
            let sym = eid ^ 1;
            if sym >= n_edges {
                continue;
            }
            let adj = edge_to_face[sym];
            if adj == usize::MAX || face_depth[adj] != i32::MIN {
                continue;
            }
            let e_delta = if eid < delta.len() { delta[eid] } else { 0 };
            face_depth[adj] = d - e_delta;
            queue.push_back(adj);
        }
    }

    // Unseeded faces (fully interior, not touching exterior) keep depth 0.
    for d in face_depth.iter_mut() {
        if *d == i32::MIN {
            *d = 0;
        }
    }

    face_depth
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

/// Unary union of many polygons without source-membership tracking.
pub fn polygon_unary_union(polys: &[Polygon], epsilon: f64) -> Vec<Polygon> {
    polygon_unary_union_with_options(
        polys,
        UnaryDissolveOptions {
            epsilon,
            ..UnaryDissolveOptions::default()
        },
    )
}

/// Unary union of many polygons with explicit strategy options and no membership tracking.
pub fn polygon_unary_union_with_options(
    polys: &[Polygon],
    options: UnaryDissolveOptions,
) -> Vec<Polygon> {
    if polys.is_empty() {
        return Vec::new();
    }

    if polys.len() == 1 {
        return vec![polys[0].clone()];
    }

    let eps = normalized_eps(options.epsilon);

    match options.strategy {
        UnaryDissolveStrategy::GraphDriven => unary_union_graph(
            polys,
            eps,
            options.noding,
            options.preferred_union_precision,
        ),
        UnaryDissolveStrategy::CascadedHeuristic => unary_union_componentized(
            polys,
            eps,
            true,
            options.preferred_union_precision,
        ),
        UnaryDissolveStrategy::PairwiseHeuristic => unary_union_componentized(
            polys,
            eps,
            false,
            options.preferred_union_precision,
        ),
    }
}

/// Dissolve many polygons into non-overlapping groups.
///
/// The result includes both dissolved geometry and source membership so callers
/// can aggregate attributes at the application layer.
pub fn polygon_unary_dissolve(polys: &[Polygon], epsilon: f64) -> Vec<UnaryDissolveGroup> {
    polygon_unary_dissolve_with_options(
        polys,
        UnaryDissolveOptions {
            epsilon,
            ..UnaryDissolveOptions::default()
        },
    )
}

/// Dissolve many polygons into non-overlapping groups with explicit strategy options.
pub fn polygon_unary_dissolve_with_options(
    polys: &[Polygon],
    options: UnaryDissolveOptions,
) -> Vec<UnaryDissolveGroup> {
    if polys.is_empty() {
        return Vec::new();
    }

    if polys.len() == 1 {
        return vec![UnaryDissolveGroup {
            poly: polys[0].clone(),
            source_indices: vec![0],
        }];
    }

    let eps = normalized_eps(options.epsilon);

    match options.strategy {
        UnaryDissolveStrategy::GraphDriven => unary_dissolve_graph(
            polys,
            eps,
            options.noding,
            options.preferred_union_precision,
        ),
        UnaryDissolveStrategy::CascadedHeuristic => unary_dissolve_componentized(
            polys,
            eps,
            true,
            options.preferred_union_precision,
        ),
        UnaryDissolveStrategy::PairwiseHeuristic => unary_dissolve_componentized(
            polys,
            eps,
            false,
            options.preferred_union_precision,
        ),
    }
}

fn unary_dissolve_componentized(
    polys: &[Polygon],
    eps: f64,
    cascaded: bool,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    let components = envelope_connected_components(polys);

    #[cfg(feature = "parallel")]
    {
        if components.len() >= 4 {
            return components
                .par_iter()
                .map(|comp| {
                    if cascaded {
                        dissolve_component_cascaded(polys, comp, eps, preferred_precision)
                    } else {
                        dissolve_component(polys, comp, eps, preferred_precision)
                    }
                })
                .flatten()
                .collect();
        }
    }

    let mut out = Vec::<UnaryDissolveGroup>::new();
    for comp in components {
        if cascaded {
            out.extend(dissolve_component_cascaded(polys, &comp, eps, preferred_precision));
        } else {
            out.extend(dissolve_component(polys, &comp, eps, preferred_precision));
        }
    }
    out
}

fn unary_union_componentized(
    polys: &[Polygon],
    eps: f64,
    cascaded: bool,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    let components = envelope_connected_components(polys);

    #[cfg(feature = "parallel")]
    {
        if components.len() >= 4 {
            return components
                .par_iter()
                .map(|comp| {
                    if cascaded {
                        union_component_cascaded(polys, comp, eps, preferred_precision)
                    } else {
                        union_component(polys, comp, eps, preferred_precision)
                    }
                })
                .flatten()
                .collect();
        }
    }

    let mut out = Vec::<Polygon>::new();
    for comp in components {
        if cascaded {
            out.extend(union_component_cascaded(polys, &comp, eps, preferred_precision));
        } else {
            out.extend(union_component(polys, &comp, eps, preferred_precision));
        }
    }
    out
}

fn unary_dissolve_graph(
    polys: &[Polygon],
    eps: f64,
    noding: NodingOptions,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    let partitions = source_components_by_non_point_connectivity(polys, eps);
    if partitions.len() > 1 {
        let mut out = Vec::<UnaryDissolveGroup>::new();
        for part in partitions {
            let subset: Vec<Polygon> = part.iter().map(|&idx| polys[idx].clone()).collect();
            let mut groups = unary_dissolve_graph_component(&subset, eps, noding, preferred_precision);
            for g in &mut groups {
                for idx in &mut g.source_indices {
                    *idx = part[*idx];
                }
                g.source_indices.sort_unstable();
                g.source_indices.dedup();
            }
            out.extend(groups);
        }
        return out;
    }

    unary_dissolve_graph_component(polys, eps, noding, preferred_precision)
}

fn unary_union_graph(
    polys: &[Polygon],
    eps: f64,
    noding: NodingOptions,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    let partitions = source_components_by_non_point_connectivity(polys, eps);
    if partitions.len() > 1 {
        let mut out = Vec::<Polygon>::new();
        for part in partitions {
            let subset: Vec<Polygon> = part.iter().map(|&idx| polys[idx].clone()).collect();
            out.extend(unary_union_graph_component(
                &subset,
                eps,
                noding,
                preferred_precision,
            ));
        }
        return out;
    }

    unary_union_graph_component(polys, eps, noding, preferred_precision)
}

fn unary_dissolve_graph_component(
    polys: &[Polygon],
    eps: f64,
    noding: NodingOptions,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    let mut boundaries = Vec::<LineString>::new();
    for poly in polys {
        boundaries.extend(polygon_boundaries(poly));
    }

    let graph = TopologyGraph::from_linestrings_with_options(
        &boundaries,
        NodingOptions {
            epsilon: eps,
            ..noding
        },
    );

    // Extract bounded face rings together with the directed edge ids on each boundary.
    let face_rings = graph.extract_bounded_face_rings_with_edges(eps);
    if face_rings.is_empty() {
        return Vec::new();
    }

    // Assign a depth delta to every directed edge from source ring membership.
    // delta[e] = depth(left face of e) – depth(right face of e).
    // This is a purely topological computation — no point-in-polygon probing.
    let mut edge_delta = vec![0i32; graph.edges.len()];
    compute_unary_edge_deltas(&graph, polys, eps, &mut edge_delta);

    // BFS from the exterior region to classify each face as depth > 0 (include) or not.
    let included = classify_faces_by_depth(&graph, &face_rings, &edge_delta);

    let source_geoms: Vec<Geometry> = polys.iter().cloned().map(Geometry::Polygon).collect();
    let source_index = SpatialIndex::from_geometries(&source_geoms);

    // Assign source membership at face granularity, then propagate memberships
    // through cascaded dissolve merges instead of rescanning all dissolved outputs.
    let mut candidate_groups = Vec::<UnaryDissolveGroup>::new();
    for ((ring, _), &inc) in face_rings.iter().zip(included.iter()) {
        if !inc {
            continue;
        }

        let poly = Polygon::new(LinearRing::new(ring.coords.clone()), vec![]);
        let mut source_indices = Vec::<usize>::new();
        let poly_geom = Geometry::Polygon(poly.clone());
        let candidates = source_index.query_geometry(&poly_geom);
        for idx in candidates {
            if idx >= polys.len() {
                continue;
            }
            if polygons_overlap_fast(&poly, &polys[idx], eps) {
                source_indices.push(idx);
            }
        }
        source_indices.sort_unstable();
        source_indices.dedup();
        candidate_groups.push(UnaryDissolveGroup { poly, source_indices });
    }

    dissolve_pre_grouped_cascaded(candidate_groups, eps, preferred_precision)
}

fn unary_union_graph_component(
    polys: &[Polygon],
    eps: f64,
    noding: NodingOptions,
    _preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    let mut boundaries = Vec::<LineString>::new();
    for poly in polys {
        boundaries.extend(polygon_boundaries(poly));
    }

    let graph = TopologyGraph::from_linestrings_with_options(
        &boundaries,
        NodingOptions {
            epsilon: eps,
            ..noding
        },
    );

    let face_rings = graph.extract_bounded_face_rings_with_edges(eps);
    if face_rings.is_empty() {
        return Vec::new();
    }

    let mut edge_delta = vec![0i32; graph.edges.len()];
    compute_unary_edge_deltas(&graph, polys, eps, &mut edge_delta);
    let included = classify_faces_by_depth(&graph, &face_rings, &edge_delta);

    let candidate_rings: Vec<Vec<Coord>> = face_rings
        .iter()
        .zip(included.iter())
        .filter_map(|((ring, _), &inc)| if inc { Some(ring.coords.clone()) } else { None })
        .collect();

    if candidate_rings.is_empty() {
        return Vec::new();
    }

    // Graph-driven union already computes included bounded faces from a single
    // noded arrangement. Assemble shells/holes directly from these rings to
    // avoid an additional pairwise overlay-union stage.
    assemble_polygons_from_rings(candidate_rings, eps)
}

/// Compute depth deltas for every directed edge in `graph` based on which source
/// polygon ring segments contain each edge's midpoint.
///
/// `delta[e]` = depth(left face of e) − depth(right face of e).
/// For a CCW exterior ring segment whose direction matches the edge direction: delta = +1.
/// Twin edges always satisfy `delta[e ^ 1] = −delta[e]`.
fn compute_unary_edge_deltas(
    graph: &TopologyGraph,
    polys: &[Polygon],
    eps: f64,
    delta: &mut [i32],
) {
    // Build a spatial index on source polygons so we only check candidates
    // near each edge midpoint rather than all N polygons.
    let geoms: Vec<Geometry> = polys.iter().cloned().map(Geometry::Polygon).collect();
    let index = SpatialIndex::from_geometries(&geoms);

    // Edges are stored in pairs (e, sym(e)) = (even_id, even_id+1).
    let mut i = 0;
    while i < graph.edges.len() {
        let e = &graph.edges[i];
        let a = graph.nodes[e.from].coord;
        let b = graph.nodes[e.to].coord;
        let mx = (a.x + b.x) * 0.5;
        let my = (a.y + b.y) * 0.5;
        let m = Coord::xy(mx, my);
        let dx = b.x - a.x;
        let dy = b.y - a.y;

        // Query index for polygons whose envelopes contain the midpoint.
        let candidates = index.query_point(m);

        let mut d = 0i32;
        for poly_idx in candidates {
            if poly_idx >= polys.len() {
                continue;
            }
            let poly = &polys[poly_idx];
            let ext_ccw = unary_ring_signed_area(&poly.exterior.coords) >= 0.0;
            d += unary_ring_segment_delta(&poly.exterior.coords, m, dx, dy, ext_ccw, eps);
            for hole in &poly.holes {
                let hole_ccw = unary_ring_signed_area(&hole.coords) >= 0.0;
                d += unary_ring_segment_delta(&hole.coords, m, dx, dy, hole_ccw, eps);
            }
        }

        delta[i] = d;
        if i + 1 < delta.len() {
            delta[i + 1] = -d;
        }
        i += 2;
    }
}

/// Returns the delta contribution (+1, −1, or 0) from one ring to the directed
/// edge whose midpoint is `m` and whose direction vector is `(dx, dy)`.
///
/// `ring_ccw`: true if the ring winds counter-clockwise (positive signed area),
/// which is the convention for exterior rings.
fn unary_ring_segment_delta(
    ring: &[Coord],
    m: Coord,
    dx: f64,
    dy: f64,
    ring_ccw: bool,
    eps: f64,
) -> i32 {
    let ring_sign: i32 = if ring_ccw { 1 } else { -1 };
    let n = ring.len();
    if n < 2 {
        return 0;
    }
    for i in 0..(n - 1) {
        let c = ring[i];
        let d = ring[i + 1];
        if !point_on_segment_eps(m, c, d, eps) {
            continue;
        }
        let cd_x = d.x - c.x;
        let cd_y = d.y - c.y;
        // dot > 0 means same general direction as the ring segment.
        let dot = dx * cd_x + dy * cd_y;
        let dir_sign: i32 = if dot >= 0.0 { 1 } else { -1 };
        return ring_sign * dir_sign;
    }
    0
}

/// Shoelace signed area of a ring (positive = CCW, negative = CW).
fn unary_ring_signed_area(coords: &[Coord]) -> f64 {
    if coords.len() < 4 {
        return 0.0;
    }
    let mut s = 0.0f64;
    for i in 0..(coords.len() - 1) {
        let a = coords[i];
        let b = coords[i + 1];
        s += a.x * b.y - b.x * a.y;
    }
    0.5 * s
}

/// Classify bounded faces by depth using BFS from the unbounded exterior region.
///
/// Returns one bool per face (in the same order as `face_rings`): true when
/// `depth > 0`, meaning the face is inside at least one source polygon and
/// should be included in the union result.
///
/// This replaces point-in-polygon probing entirely: face membership is derived
/// from the edge depth deltas alone, which makes it immune to boundary-zone
/// misclassifications near short source polygon segments.
fn classify_faces_by_depth(
    graph: &TopologyGraph,
    face_rings: &[(LineString, Vec<usize>)],
    delta: &[i32],
) -> Vec<bool> {
    let n_faces = face_rings.len();
    let n_edges = graph.edges.len();

    // Map each directed edge id to the bounded face ring that contains it.
    // Edges not in this map belong to the unbounded exterior face (depth 0).
    let mut edge_to_face = vec![usize::MAX; n_edges];
    for (face_id, (_, edge_ids)) in face_rings.iter().enumerate() {
        for &eid in edge_ids {
            if eid < n_edges {
                edge_to_face[eid] = face_id;
            }
        }
    }

    // BFS depth propagation.
    // Seed: for each bounded face, find edges whose twin is in the exterior (unbounded
    // face, depth 0).  Crossing from the exterior through sym(eid) into the face gives:
    //   depth(face) = depth(exterior) − delta[sym(eid)]
    //               = 0 − (−delta[eid]) = delta[eid]
    let mut face_depth = vec![i32::MIN; n_faces];
    let mut queue = VecDeque::<usize>::new();

    for face_id in 0..n_faces {
        // Find the first exterior-adjacent edge that provides a non-zero seed,
        // falling back to any exterior-adjacent edge if none gives a non-zero delta.
        let mut seed = i32::MIN;
        let mut fallback_seed = i32::MIN;
        for &eid in &face_rings[face_id].1 {
            let sym = eid ^ 1;
            if sym < n_edges && edge_to_face[sym] == usize::MAX {
                let d = if eid < delta.len() { delta[eid] } else { 0 };
                if d != 0 && seed == i32::MIN {
                    seed = d;
                    break;
                }
                if fallback_seed == i32::MIN {
                    fallback_seed = d;
                }
            }
        }
        let chosen = if seed != i32::MIN { seed } else { fallback_seed };
        if chosen != i32::MIN {
            face_depth[face_id] = chosen;
            queue.push_back(face_id);
        }
    }

    // BFS: propagate from seeded faces.
    // Crossing edge `eid` (in current face's boundary) into the adjacent face gives:
    //   depth(adjacent) = depth(current) − delta[eid]
    while let Some(current_id) = queue.pop_front() {
        let d = face_depth[current_id];
        let edge_ids = face_rings[current_id].1.clone();
        for eid in edge_ids {
            let sym = eid ^ 1;
            if sym >= n_edges {
                continue;
            }
            let adj = edge_to_face[sym];
            if adj == usize::MAX || face_depth[adj] != i32::MIN {
                continue; // exterior or already visited
            }
            let e_delta = if eid < delta.len() { delta[eid] } else { 0 };
            face_depth[adj] = d - e_delta;
            queue.push_back(adj);
        }
    }

    // Any face not reached by BFS (isolated component not touching the exterior)
    // falls back to using its first edge's delta value, which may be conservative but
    // is rare in practice (requires complex overlapping topology). Log a diagnostic
    // if this occurs: it suggests possible topology graph issues or degenerate input.
    let mut unreached_count = 0usize;
    for face_id in 0..n_faces {
        if face_depth[face_id] == i32::MIN {
            unreached_count += 1;
            face_depth[face_id] = face_rings[face_id]
                .1
                .first()
                .and_then(|&eid| delta.get(eid).copied())
                .unwrap_or(0);
        }
    }

    if unreached_count > 0 {
        eprintln!(
            "WARNING: classify_faces_by_depth: {} unreached face(s) with isolated topology \
             (using delta fallback for robustness)",
            unreached_count
        );
    }

    face_depth.iter().map(|&d| d > 0).collect()
}

fn polygons_overlap_fast(a: &Polygon, b: &Polygon, eps: f64) -> bool {
    if let (Some(ea), Some(eb)) = (a.envelope(), b.envelope()) {
        if !ea.intersects(&eb) {
            return false;
        }
    }

    if polygon_boundaries_cross(a, b, eps) {
        return true;
    }

    if let Some(p) = a.exterior.coords.first().copied() {
        if matches!(classify_point_in_polygon_eps(p, b, eps), PointInRing::Inside | PointInRing::Boundary) {
            return true;
        }
    }

    if let Some(p) = b.exterior.coords.first().copied() {
        if matches!(classify_point_in_polygon_eps(p, a, eps), PointInRing::Inside | PointInRing::Boundary) {
            return true;
        }
    }

    false
}

fn source_components_by_non_point_connectivity(polys: &[Polygon], eps: f64) -> Vec<Vec<usize>> {
    if polys.is_empty() {
        return Vec::new();
    }

    let mut visited = vec![false; polys.len()];
    let mut comps = Vec::<Vec<usize>>::new();

    for start in 0..polys.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![start];
        let mut comp = Vec::<usize>::new();

        while let Some(i) = stack.pop() {
            comp.push(i);
            for j in 0..polys.len() {
                if visited[j] || i == j {
                    continue;
                }
                if sources_have_non_point_connection(&polys[i], &polys[j], eps) {
                    visited[j] = true;
                    stack.push(j);
                }
            }
        }

        comp.sort_unstable();
        comps.push(comp);
    }

    comps.sort();
    comps
}

fn sources_have_non_point_connection(a: &Polygon, b: &Polygon, eps: f64) -> bool {
    if let (Some(ea), Some(eb)) = (a.envelope(), b.envelope()) {
        if !ea.intersects(&eb) {
            let gap = geometry_distance(&Geometry::Polygon(a.clone()), &Geometry::Polygon(b.clone()));
            if !(gap.is_finite() && gap > 0.0 && gap <= eps) {
                return false;
            }
        }
    }

    if polygon_boundaries_have_nontrivial_contact(a, b, eps) {
        return true;
    }

    if let Some(p) = a.exterior.coords.first().copied() {
        if matches!(classify_point_in_polygon_eps(p, b, eps), PointInRing::Inside) {
            return true;
        }
    }

    if let Some(p) = b.exterior.coords.first().copied() {
        if matches!(classify_point_in_polygon_eps(p, a, eps), PointInRing::Inside) {
            return true;
        }
    }

    false
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

#[derive(Debug, Clone)]
struct DissolveWork {
    poly: Polygon,
    envelope: Option<Envelope>,
    area: f64,
    members: Vec<usize>,
}

#[derive(Debug, Clone)]
struct UnionWork {
    poly: Polygon,
    envelope: Option<Envelope>,
    area: f64,
}

fn init_component_work(polys: &[Polygon], component: &[usize]) -> Vec<DissolveWork> {
    component
        .iter()
        .copied()
        .map(|idx| DissolveWork {
            poly: polys[idx].clone(),
            envelope: polys[idx].envelope(),
            area: polygon_abs_area(&polys[idx]),
            members: vec![idx],
        })
        .collect()
}

fn init_union_component_work(polys: &[Polygon], component: &[usize]) -> Vec<UnionWork> {
    component
        .iter()
        .copied()
        .map(|idx| UnionWork {
            poly: polys[idx].clone(),
            envelope: polys[idx].envelope(),
            area: polygon_abs_area(&polys[idx]),
        })
        .collect()
}

fn finalize_component_work(groups: Vec<DissolveWork>) -> Vec<UnaryDissolveGroup> {
    groups
        .into_iter()
        .map(|mut g| {
            g.members.sort_unstable();
            g.members.dedup();
            UnaryDissolveGroup {
                poly: g.poly,
                source_indices: g.members,
            }
        })
        .collect()
}

fn finalize_union_work(groups: Vec<UnionWork>) -> Vec<Polygon> {
    groups.into_iter().map(|g| g.poly).collect()
}

fn dissolve_pre_grouped_cascaded(
    groups: Vec<UnaryDissolveGroup>,
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    if groups.is_empty() {
        return Vec::new();
    }

    let work: Vec<DissolveWork> = groups
        .into_iter()
        .map(|g| DissolveWork {
            area: polygon_abs_area(&g.poly),
            envelope: g.poly.envelope(),
            poly: g.poly,
            members: g.source_indices,
        })
        .collect();

    finalize_component_work(dissolve_work_cascaded(work, eps, 0, preferred_precision))
}

#[allow(dead_code)]
fn union_pre_grouped_cascaded(
    polys: Vec<Polygon>,
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    if polys.is_empty() {
        return Vec::new();
    }

    let work: Vec<UnionWork> = polys
        .into_iter()
        .map(|poly| UnionWork {
            area: polygon_abs_area(&poly),
            envelope: poly.envelope(),
            poly,
        })
        .collect();

    finalize_union_work(union_work_cascaded(work, eps, 0, preferred_precision))
}

fn dissolve_component_cascaded(
    polys: &[Polygon],
    component: &[usize],
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    if component.is_empty() {
        return Vec::new();
    }

    let groups = init_component_work(polys, component);
    let dissolved = dissolve_work_cascaded(groups, eps, 0, preferred_precision);
    finalize_component_work(dissolved)
}

fn union_component_cascaded(
    polys: &[Polygon],
    component: &[usize],
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    if component.is_empty() {
        return Vec::new();
    }

    let groups = init_union_component_work(polys, component);
    let dissolved = union_work_cascaded(groups, eps, 0, preferred_precision);
    finalize_union_work(dissolved)
}

fn dissolve_work_cascaded(
    groups: Vec<DissolveWork>,
    eps: f64,
    depth: usize,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<DissolveWork> {
    const LEAF_SIZE: usize = 16;

    if groups.len() <= 1 {
        return groups;
    }

    if groups.len() <= LEAF_SIZE {
        return dissolve_work_pairwise(groups, eps, preferred_precision);
    }

    let axis_x = depth % 2 == 0;
    let mut ordered = groups;
    ordered.sort_by(|a, b| dissolve_work_axis_value(a, axis_x)
        .partial_cmp(&dissolve_work_axis_value(b, axis_x))
        .unwrap_or(Ordering::Equal));

    let mid = ordered.len() / 2;
    let right = ordered.split_off(mid);
    let left = ordered;

    #[cfg(feature = "parallel")]
    if left.len() >= 64 && right.len() >= 64 {
        let (mut left_out, mut right_out) = join(
            || dissolve_work_cascaded(left, eps, depth + 1, preferred_precision),
            || dissolve_work_cascaded(right, eps, depth + 1, preferred_precision),
        );
        left_out.append(&mut right_out);
        return dissolve_work_pairwise(left_out, eps, preferred_precision);
    }

    let mut left_out = dissolve_work_cascaded(left, eps, depth + 1, preferred_precision);
    let mut right_out = dissolve_work_cascaded(right, eps, depth + 1, preferred_precision);
    left_out.append(&mut right_out);
    dissolve_work_pairwise(left_out, eps, preferred_precision)
}

fn dissolve_work_axis_value(work: &DissolveWork, axis_x: bool) -> f64 {
    if let Some(env) = work.envelope {
        if axis_x {
            (env.min_x + env.max_x) * 0.5
        } else {
            (env.min_y + env.max_y) * 0.5
        }
    } else {
        0.0
    }
}

fn union_work_axis_value(work: &UnionWork, axis_x: bool) -> f64 {
    if let Some(env) = work.envelope {
        if axis_x {
            (env.min_x + env.max_x) * 0.5
        } else {
            (env.min_y + env.max_y) * 0.5
        }
    } else {
        0.0
    }
}

fn dissolve_work_pairwise(
    mut groups: Vec<DissolveWork>,
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<DissolveWork> {
    if groups.len() < 2 {
        return groups;
    }

    loop {
        let mut merged_any = false;

        'scan: for i in 0..groups.len() {
            let env_i = groups[i].envelope;

            #[cfg(feature = "parallel")]
            {
                if groups.len() >= 64 {
                    let best = ((i + 1)..groups.len())
                        .into_par_iter()
                        .filter_map(|j: usize| {
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
                                preferred_precision,
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
                    preferred_precision,
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
}

fn union_work_cascaded(
    groups: Vec<UnionWork>,
    eps: f64,
    depth: usize,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnionWork> {
    const LEAF_SIZE: usize = 16;

    if groups.len() <= 1 {
        return groups;
    }

    if groups.len() <= LEAF_SIZE {
        return union_work_pairwise(groups, eps, preferred_precision);
    }

    let axis_x = depth % 2 == 0;
    let mut ordered = groups;
    ordered.sort_by(|a, b| union_work_axis_value(a, axis_x)
        .partial_cmp(&union_work_axis_value(b, axis_x))
        .unwrap_or(Ordering::Equal));

    let mid = ordered.len() / 2;
    let right = ordered.split_off(mid);
    let left = ordered;

    #[cfg(feature = "parallel")]
    if left.len() >= 64 && right.len() >= 64 {
        let (mut left_out, mut right_out) = join(
            || union_work_cascaded(left, eps, depth + 1, preferred_precision),
            || union_work_cascaded(right, eps, depth + 1, preferred_precision),
        );
        left_out.append(&mut right_out);
        return union_work_pairwise(left_out, eps, preferred_precision);
    }

    let mut left_out = union_work_cascaded(left, eps, depth + 1, preferred_precision);
    let mut right_out = union_work_cascaded(right, eps, depth + 1, preferred_precision);
    left_out.append(&mut right_out);
    union_work_pairwise(left_out, eps, preferred_precision)
}

fn union_work_pairwise(
    mut groups: Vec<UnionWork>,
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnionWork> {
    if groups.len() < 2 {
        return groups;
    }

    loop {
        let mut merged_any = false;

        'scan: for i in 0..groups.len() {
            let env_i = groups[i].envelope;

            #[cfg(feature = "parallel")]
            {
                if groups.len() >= 64 {
                    let best = ((i + 1)..groups.len())
                        .into_par_iter()
                        .filter_map(|j: usize| {
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
                                preferred_precision,
                            )
                            .map(|poly| (j, poly))
                        })
                        .reduce_with(|a, b| if a.0 < b.0 { a } else { b });

                    if let Some((j, merged_poly)) = best {
                        groups[i].poly = merged_poly;
                        groups[i].envelope = groups[i].poly.envelope();
                        groups[i].area = polygon_abs_area(&groups[i].poly);
                        groups.remove(j);
                        merged_any = true;
                        break 'scan;
                    }
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
                    preferred_precision,
                ) {
                    groups[i].poly = merged_poly;
                    groups[i].envelope = groups[i].poly.envelope();
                    groups[i].area = polygon_abs_area(&groups[i].poly);
                    groups.remove(j);
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
}

fn dissolve_component(
    polys: &[Polygon],
    component: &[usize],
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<UnaryDissolveGroup> {
    if component.is_empty() {
        return Vec::new();
    }

    let groups = init_component_work(polys, component);

    if groups.len() < 2 {
        return finalize_component_work(groups);
    }

    finalize_component_work(dissolve_work_pairwise(groups, eps, preferred_precision))
}

fn union_component(
    polys: &[Polygon],
    component: &[usize],
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Vec<Polygon> {
    if component.is_empty() {
        return Vec::new();
    }

    let groups = init_union_component_work(polys, component);

    if groups.len() < 2 {
        return finalize_union_work(groups);
    }

    finalize_union_work(union_work_pairwise(groups, eps, preferred_precision))
}

fn safe_dissolve_union(
    a: &Polygon,
    area_a: f64,
    b: &Polygon,
    area_b: f64,
    eps: f64,
    preferred_precision: Option<PrecisionModel>,
) -> Option<Polygon> {
    let quick_eps = eps.max(1.0e-9);
    if !polygon_boundaries_cross(a, b, quick_eps)
        && !shell_strictly_inside(a, b, quick_eps)
        && !shell_strictly_inside(b, a, quick_eps)
    {
        return None;
    }

    let min_expected = area_a.max(area_b);
    let area_tol = eps.max(1.0e-9) * 10.0;

    if let Some(precision) = preferred_precision {
        let union = polygon_union_with_precision(a, b, precision);
        if union.len() == 1 {
            let poly = union[0].clone();
            let tol = match precision {
                PrecisionModel::Fixed { scale } if scale > 0.0 && scale.is_finite() => {
                    area_tol.max(1.0 / scale)
                }
                _ => area_tol,
            };
            if union_candidate_is_valid_with_precision(&poly, a, b, eps, precision, tol, min_expected) {
                return Some(poly);
            }
        }
    }

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

fn ring_boundary_has_nontrivial_contact_eps(a: &[Coord], b: &[Coord], eps: f64) -> bool {
    if a.len() < 2 || b.len() < 2 {
        return false;
    }

    for i in 0..(a.len() - 1) {
        let a1 = a[i];
        let a2 = a[i + 1];
        for j in 0..(b.len() - 1) {
            let b1 = b[j];
            let b2 = b[j + 1];
            if !segments_intersect_eps(a1, a2, b1, b2, eps) {
                continue;
            }

            if collinear_segment_overlap_gt_eps(a1, a2, b1, b2, eps) {
                return true;
            }

            let shares_endpoint = nearly_eq(a1, b1, eps)
                || nearly_eq(a1, b2, eps)
                || nearly_eq(a2, b1, eps)
                || nearly_eq(a2, b2, eps);
            if !shares_endpoint {
                return true;
            }
        }
    }

    false
}

fn polygon_boundaries_have_nontrivial_contact(a: &Polygon, b: &Polygon, eps: f64) -> bool {
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
            if ring_boundary_has_nontrivial_contact_eps(ra, rb, eps) {
                return true;
            }
        }
    }

    false
}

fn collinear_segment_overlap_gt_eps(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> bool {
    let ax = a2.x - a1.x;
    let ay = a2.y - a1.y;
    let seg_len = (ax * ax + ay * ay).sqrt();
    if seg_len <= eps {
        return false;
    }

    let cross1 = ax * (b1.y - a1.y) - ay * (b1.x - a1.x);
    let cross2 = ax * (b2.y - a1.y) - ay * (b2.x - a1.x);
    if cross1.abs() > eps || cross2.abs() > eps {
        return false;
    }

    let ux = ax / seg_len;
    let uy = ay / seg_len;
    let a0 = 0.0f64;
    let a1p = seg_len;
    let b0 = (b1.x - a1.x) * ux + (b1.y - a1.y) * uy;
    let b1p = (b2.x - a1.x) * ux + (b2.y - a1.y) * uy;

    let amin = a0.min(a1p);
    let amax = a0.max(a1p);
    let bmin = b0.min(b1p);
    let bmax = b0.max(b1p);

    let overlap = amax.min(bmax) - amin.max(bmin);
    overlap > eps
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

/// Compute the envelope (bounding box) of a coordinate slice (ring).
fn linestring_envelope(coords: &[Coord]) -> Option<Envelope> {
    if coords.is_empty() {
        return None;
    }
    let mut min_x = coords[0].x;
    let mut max_x = coords[0].x;
    let mut min_y = coords[0].y;
    let mut max_y = coords[0].y;

    for &c in &coords[1..] {
        if c.x < min_x {
            min_x = c.x;
        }
        if c.x > max_x {
            max_x = c.x;
        }
        if c.y < min_y {
            min_y = c.y;
        }
        if c.y > max_y {
            max_y = c.y;
        }
    }

    Some(Envelope {
        min_x,
        max_x,
        min_y,
        max_y,
    })
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

            update_quantized_coord_map(&mut coord_map, qa, a);
            update_quantized_coord_map(&mut coord_map, qb, b);
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
            aa.total_cmp(&ab).then_with(|| na.cmp(nb))
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

    // Build spatial index over ring envelopes for fast candidate filtering.
    // For large ring sets (1000+ rings), the spatial index avoids O(n²) containment tests.
    let mut spatial_index = SpatialIndex::new();
    for ring in rings.iter() {
        spatial_index.insert(Geometry::LineString(LineString::new(ring.clone())));
    }

    let mut parent: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let mut best_parent: Option<usize> = None;
        let mut best_area = f64::INFINITY;

        // Use spatial index to find only rings whose envelope could potentially contain ring i.
        if let Some(env_i) = linestring_envelope(&rings[i]) {
            let candidates = spatial_index.query_envelope(env_i);
            for &j in &candidates {
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

    // Primary test: check if the candidate ring's centroid is inside the container.
    // This is more robust than vertex iteration for boundary-coincident rings.
    if let Some(centroid) = ring_centroid(candidate) {
        match classify_point_in_ring_eps(centroid, container, eps) {
            PointInRing::Inside => return true,
            PointInRing::Outside => return false,
            PointInRing::Boundary => {
                // Centroid is on the container boundary (rare but possible).
                // Fall through to secondary vertex check.
            }
        }
    }

    // Secondary test: iterate through candidate vertices, skipping boundary points.
    // If any interior point is clearly Inside or Outside, use that result.
    let mut saw_inside = false;
    let mut saw_outside = false;
    for p in &candidate[..candidate.len() - 1] {
        match classify_point_in_ring_eps(*p, container, eps) {
            PointInRing::Inside => saw_inside = true,
            PointInRing::Outside => saw_outside = true,
            PointInRing::Boundary => {
                // Skip boundary points; they don't help distinguish containment.
            }
        }

        if saw_outside {
            return false;
        }
    }

    saw_inside
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

fn update_quantized_coord_map(coord_map: &mut HashMap<QCoord, Coord>, key: QCoord, candidate: Coord) {
    match coord_map.get_mut(&key) {
        Some(existing) => {
            if coord_lex_lt(candidate, *existing) {
                *existing = candidate;
            }
        }
        None => {
            coord_map.insert(key, candidate);
        }
    }
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
