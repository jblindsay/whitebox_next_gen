//! BufferOp orchestration scaffold for global curve/noding/graph buffering.
//!
//! This module provides a restart-safe entry point for the vector `buffer_vector`
//! rewrite. The design mirrors GEOS/JTS BufferOp staging, but keeps behavior
//! conservative while the full face-label pipeline is being completed.

use std::collections::{HashSet, VecDeque};

use crate::algorithms::segment::point_on_segment_eps;
use crate::constructive::{
    buffer_linestring_curve_set, buffer_polygon_curve_set, make_valid_polygon, BufferOptions,
    PolygonizeOptions,
};
use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};
use crate::graph::TopologyGraph;
use crate::noding::{node_linestrings_with_options, NodingOptions, NodingStrategy};
use crate::overlay::polygon_unary_union;
use crate::spatial_index::SpatialIndex;
use crate::topology::is_valid_polygon;

/// Role classification for offset curves in buffer operations.
///
/// Mirrors GEOS/JTS buffer semantics: edge deltas are computed from curve role,
/// not from source polygon containment. This ensures correct inside/outside labeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveRole {
    /// Left offset of a linestring (inside buffer is on the left of the curve direction).
    LeftOffset,
    /// Right offset of a linestring (inside buffer is on the right of the curve direction).
    RightOffset,
    /// End cap connecting left and right offsets.
    EndCap,
    /// Exterior ring offset of a polygon.
    ExteriorOffset,
    /// Interior (hole) ring offset of a polygon.
    HoleOffset,
}

impl CurveRole {
    /// Standard delta for this role when the curve is oriented forward.
    /// Delta encodes which side of the curve is "inside buffer".
    pub fn standard_delta(self) -> i32 {
        match self {
            // Left offset: edge is oriented left-to-right; inside is on the left (−90° turn).
            // An edge perpendicular to the offset (facing left) should be +1.
            CurveRole::LeftOffset => 1,
            // Right offset: edge is oriented right-to-left (reversed); inside is on the right.
            // An edge perpendicular to the offset (facing right) should be −1.
            CurveRole::RightOffset => -1,
            // End caps don't separate inside from outside; they're neutral.
            CurveRole::EndCap => 0,
            // Exterior offset: inside is outside the original polygon (−distance).
            CurveRole::ExteriorOffset => 1,
            // Hole offset: inside is inside the hole boundary.
            CurveRole::HoleOffset => -1,
        }
    }
}

/// Configuration for [`BufferOp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferOpOptions {
    /// Buffer style options used during curve generation.
    pub buffer: BufferOptions,
    /// Noding options applied to the global curve set.
    pub noding: NodingOptions,
    /// Predicate epsilon used for polygonization and dissolve.
    pub epsilon: f64,
}

impl Default for BufferOpOptions {
    fn default() -> Self {
        Self {
            buffer: BufferOptions::default(),
            noding: NodingOptions {
                epsilon: 1.0e-9,
                strategy: NodingStrategy::SnapRounding,
                precision: None,
            },
            epsilon: 1.0e-9,
        }
    }
}

/// Stage counters emitted by [`BufferOp::run_linestrings_dissolved`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferOpStats {
    /// Number of input source lines.
    pub input_lines: usize,
    /// Number of raw curves generated.
    pub raw_curves: usize,
    /// Number of curves after global noding.
    pub noded_curves: usize,
    /// Number of bounded face rings extracted from planar graph traversal.
    pub face_rings: usize,
    /// Number of candidate polygons from polygonization.
    pub candidate_polygons: usize,
    /// Number of face polygons classified as inside (depth >= 1 equivalent).
    pub labeled_inside_polygons: usize,
    /// Number of dissolved output polygons.
    pub dissolved_polygons: usize,
}

/// Result bundle for staged BufferOp execution.
#[derive(Debug, Clone, PartialEq)]
pub struct BufferOpResult {
    /// Dissolved output polygons.
    pub polygons: Vec<Polygon>,
    /// Stage counters to help diagnose regressions while iterating.
    pub stats: BufferOpStats,
}

/// GEOS-style buffer orchestrator for batched buffering workflows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferOp {
    /// Runtime options for all pipeline stages.
    pub options: BufferOpOptions,
}

impl BufferOp {
    /// Create a new `BufferOp` with explicit options.
    pub fn new(options: BufferOpOptions) -> Self {
        Self { options }
    }

    /// Run the global buffering pipeline for line input and return dissolved output.
    ///
    /// GEOS-style staged path for lines:
    /// raw offset curves (left/right/caps) -> global noding -> graph faces -> depth labels.
    pub fn run_linestrings_dissolved(
        &self,
        lines: &[LineString],
        distance: f64,
    ) -> BufferOpResult {
        let mut stats = BufferOpStats {
            input_lines: lines.len(),
            ..BufferOpStats::default()
        };

        if !distance.is_finite() || distance <= 0.0 || lines.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let (curves, roles) = self.collect_raw_curves_with_roles(lines, distance);
        stats.raw_curves = curves.len();
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        self.dissolve_curve_set_with_roles(curves, roles, false, stats)
    }

    /// Run the global buffering pipeline for polygon input and return dissolved output.
    pub fn run_polygons_dissolved(&self, polygons: &[Polygon], distance: f64) -> BufferOpResult {
        let mut stats = BufferOpStats {
            input_lines: polygons.len(),
            ..BufferOpStats::default()
        };

        if !distance.is_finite() || distance <= 0.0 || polygons.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 1: Generate offset curves with role metadata.
        let (curves, roles) = self.collect_polygon_curves_with_roles(polygons, distance);
        stats.raw_curves = curves.len();
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 2-6: Noding, graph build, depth labeling using curve roles.
        self.dissolve_curve_set_with_roles(curves, roles, false, stats)
    }

    fn collect_raw_curves_with_roles(
        &self,
        lines: &[LineString],
        distance: f64,
    ) -> (Vec<LineString>, Vec<CurveRole>) {
        let mut curves = Vec::<LineString>::new();
        let mut roles = Vec::<CurveRole>::new();
        for line in lines {
            let raw_curves = buffer_linestring_curve_set(line, distance, self.options.buffer);
            for (idx, curve) in raw_curves.iter().enumerate() {
                // For open lines, curve_set emits boundary components in order:
                // [left_offset, end_cap, right_offset, start_cap].
                // For closed lines, order remains [exterior, holes...].
                let role = if line.coords.len() < 2 || !is_line_closed(&line.coords) {
                    if idx == 0 {
                        CurveRole::LeftOffset
                    } else if idx == 2 {
                        CurveRole::RightOffset
                    } else {
                        CurveRole::EndCap
                    }
                } else {
                    if idx == 0 {
                        CurveRole::ExteriorOffset
                    } else {
                        CurveRole::HoleOffset
                    }
                };
                curves.push(curve.clone());
                roles.push(role);
            }
        }
        (curves, roles)
    }

    fn collect_polygon_curves_with_roles(
        &self,
        polygons: &[Polygon],
        distance: f64,
    ) -> (Vec<LineString>, Vec<CurveRole>) {
        let mut curves = Vec::<LineString>::new();
        let mut roles = Vec::<CurveRole>::new();
        for poly in polygons {
            let raw_curves = buffer_polygon_curve_set(poly, distance, self.options.buffer);
            for (idx, curve) in raw_curves.iter().enumerate() {
                // First curve is exterior, rest are holes.
                let role = if idx == 0 {
                    CurveRole::ExteriorOffset
                } else {
                    CurveRole::HoleOffset
                };
                curves.push(curve.clone());
                roles.push(role);
            }
        }
        (curves, roles)
    }

    fn dissolve_curve_set_with_roles(
        &self,
        curves: Vec<LineString>,
        roles: Vec<CurveRole>,
        allow_negative_depth_inside: bool,
        mut stats: BufferOpStats,
    ) -> BufferOpResult {
        if curves.is_empty() || roles.is_empty() || curves.len() != roles.len() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let (curves, roles) = dedupe_curve_roles(curves, roles, self.options.noding.epsilon);
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 2: Global noding of all curves.
        let noded = node_linestrings_with_options(&curves, self.options.noding);
        stats.noded_curves = noded.len();
        if noded.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 3-4: Build planar graph and extract bounded face rings.
        let epsilon = self.options.epsilon.max(self.options.noding.epsilon).max(1.0e-9);
        let graph = TopologyGraph::from_noded_linestrings(&noded, self.options.noding.epsilon);
        let face_rings_with_edges = graph.extract_bounded_face_rings_with_edges(epsilon);
        stats.face_rings = face_rings_with_edges.len();

        // Stage 5: Role-based depth labeling (GEOS/JTS semantics).
        let (selected_face_rings, selected_face_count) =
            self.classify_inside_face_rings_by_role(
                &graph,
                &face_rings_with_edges,
                &curves,
                &roles,
                epsilon,
                allow_negative_depth_inside,
            );
        stats.labeled_inside_polygons = if selected_face_count > 0 {
            selected_face_count
        } else {
            face_rings_with_edges.len()
        };

        // Stage 6: Polygonize and dissolve.
        let all_face_rings: Vec<LineString> = face_rings_with_edges
            .iter()
            .map(|(ring, _)| ring.clone())
            .collect();
        let polygonize_input: &[LineString] = if !selected_face_rings.is_empty() {
            &selected_face_rings
        } else if !all_face_rings.is_empty() {
            &all_face_rings
        } else {
            &noded
        };

        let poly_result = crate::constructive::polygonize_linework(
            polygonize_input,
            PolygonizeOptions {
                epsilon: self.options.epsilon,
                noding: self.options.noding,
            },
        );
        let mut candidate_polys = poly_result.polygons;
        if candidate_polys.is_empty() {
            let rings_src: &[LineString] = if !selected_face_rings.is_empty() {
                &selected_face_rings
            } else {
                &all_face_rings
            };
            for ring in rings_src {
                if ring.coords.len() >= 4 {
                    candidate_polys.push(Polygon::new(
                        crate::geom::LinearRing::new(ring.coords.clone()),
                        vec![],
                    ));
                }
            }
        }
        stats.candidate_polygons = candidate_polys.len();

        if candidate_polys.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let dissolved = polygon_unary_union(&candidate_polys, self.options.epsilon);
        let mut repaired = Vec::<Polygon>::new();
        for poly in dissolved {
            if is_valid_polygon(&poly) {
                repaired.push(poly);
            } else {
                let mut parts = make_valid_polygon(&poly, self.options.epsilon);
                if parts.is_empty() {
                    if poly.exterior.coords.len() >= 4 {
                        repaired.push(Polygon::new(LinearRing::new(poly.exterior.coords.clone()), vec![]));
                    }
                } else {
                    repaired.append(&mut parts);
                }
            }
        }
        let mut final_valid = Vec::<Polygon>::new();
        for poly in repaired {
            if is_valid_polygon(&poly) {
                final_valid.push(poly);
                continue;
            }
            for part in make_valid_polygon(&poly, self.options.epsilon) {
                if is_valid_polygon(&part) {
                    final_valid.push(part);
                }
            }
        }
        if final_valid.is_empty() {
            for poly in candidate_polys {
                if is_valid_polygon(&poly) {
                    final_valid.push(poly);
                    continue;
                }
                for part in make_valid_polygon(&poly, self.options.epsilon) {
                    if is_valid_polygon(&part) {
                        final_valid.push(part);
                    }
                }
            }
        }
        stats.dissolved_polygons = final_valid.len();

        BufferOpResult {
            polygons: final_valid,
            stats,
        }
    }

    fn classify_inside_face_rings_by_role(
        &self,
        graph: &TopologyGraph,
        face_rings_with_edges: &[(LineString, Vec<usize>)],
        curves: &[LineString],
        roles: &[CurveRole],
        eps: f64,
        allow_negative_depth_inside: bool,
    ) -> (Vec<LineString>, usize) {
        if face_rings_with_edges.is_empty() || curves.is_empty() || roles.is_empty() {
            return (Vec::new(), 0);
        }

        let mut edge_delta = vec![0i32; graph.edges.len()];
        self.compute_edge_deltas_from_curves(graph, curves, roles, eps, &mut edge_delta);
        let included = self.classify_faces_by_depth(
            graph,
            face_rings_with_edges,
            &edge_delta,
            allow_negative_depth_inside,
        );

        let mut selected = Vec::<LineString>::new();
        for ((ring, _), keep) in face_rings_with_edges.iter().zip(included.into_iter()) {
            if keep {
                selected.push(ring.clone());
            }
        }
        let selected_count = selected.len();
        (selected, selected_count)
    }

    fn compute_edge_deltas_from_curves(
        &self,
        graph: &TopologyGraph,
        curves: &[LineString],
        roles: &[CurveRole],
        eps: f64,
        delta: &mut [i32],
    ) {
        // Build a spatial index of all curves for fast lookup.
        let curve_geoms: Vec<Geometry> = curves
            .iter()
            .cloned()
            .map(Geometry::LineString)
            .collect();
        let curve_index = SpatialIndex::from_geometries(&curve_geoms);

        // For each edge, find which curve(s) it came from and use their roles.
        let mut edge_id = 0usize;
        while edge_id < graph.edges.len() {
            let edge = &graph.edges[edge_id];
            let a = graph.nodes[edge.from].coord;
            let b = graph.nodes[edge.to].coord;
            let midpoint = Coord::xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            let dx = b.x - a.x;
            let dy = b.y - a.y;

            // Find which source curve this edge came from.
            // Query the spatial index for curves near this edge's midpoint.
            let mut d = 0i32;
            for curve_idx in curve_index.query_point(midpoint) {
                if curve_idx >= curves.len() || curve_idx >= roles.len() {
                    continue;
                }
                let curve = &curves[curve_idx];
                let role = roles[curve_idx];

                // Check if the edge midpoint lies on this curve.
                if self.edge_on_curve(midpoint, curve, eps) {
                    // Compute the delta based on curve role and edge direction.
                    d += self.delta_from_curve_role_and_direction(role, dx, dy, curve, midpoint, eps);
                }
            }

            delta[edge_id] = d;
            if edge_id + 1 < delta.len() {
                delta[edge_id + 1] = -d;
            }
            edge_id += 2;
        }
    }


    fn edge_on_curve(&self, midpoint: Coord, curve: &LineString, eps: f64) -> bool {
        if curve.coords.len() < 2 {
            return false;
        }
        for i in 0..(curve.coords.len() - 1) {
            if point_on_segment_eps(midpoint, curve.coords[i], curve.coords[i + 1], eps) {
                return true;
            }
        }
        false
    }

    fn delta_from_curve_role_and_direction(
        &self,
        role: CurveRole,
        dx: f64,
        dy: f64,
        curve: &LineString,
        midpoint: Coord,
        eps: f64,
    ) -> i32 {
        // Find the segment index on the curve where midpoint lies.
        for i in 0..(curve.coords.len() - 1) {
            let c = curve.coords[i];
            let d = curve.coords[i + 1];
            if !point_on_segment_eps(midpoint, c, d, eps) {
                continue;
            }

            // Curve direction: from c to d.
            let cdx = d.x - c.x;
            let cdy = d.y - c.y;
            let dot = dx * cdx + dy * cdy;
            let edge_forward = dot >= 0.0; // edge direction matches curve direction

            // Base delta from role.
            let role_delta = role.standard_delta();

            // Adjust if edge direction opposes curve direction.
            if edge_forward {
                return role_delta;
            } else {
                return -role_delta;
            }
        }
        0 // Edge not on this curve.
    }


    fn classify_faces_by_depth(
        &self,
        graph: &TopologyGraph,
        face_rings: &[(LineString, Vec<usize>)],
        delta: &[i32],
        allow_negative_depth_inside: bool,
    ) -> Vec<bool> {
        let n_faces = face_rings.len();
        let n_edges = graph.edges.len();

        let mut edge_to_face = vec![usize::MAX; n_edges];
        for (face_id, (_, edge_ids)) in face_rings.iter().enumerate() {
            for &edge_id in edge_ids {
                if edge_id < n_edges {
                    edge_to_face[edge_id] = face_id;
                }
            }
        }

        let mut face_depth = vec![i32::MIN; n_faces];
        let mut queue = VecDeque::<usize>::new();
        let face_sign: Vec<i32> = face_rings
            .iter()
            .map(|(ring, _)| {
                if signed_ring_area(&ring.coords) >= 0.0 { 1 } else { -1 }
            })
            .collect();

        for face_id in 0..n_faces {
            let mut seed = i32::MIN;
            let mut fallback_seed = i32::MIN;
            for &edge_id in &face_rings[face_id].1 {
                let sym = edge_id ^ 1;
                if sym < n_edges && edge_to_face[sym] == usize::MAX {
                    let d = if edge_id < delta.len() { delta[edge_id] } else { 0 };
                    let signed_d = face_sign[face_id] * d;
                    if signed_d != 0 && seed == i32::MIN {
                        seed = signed_d;
                        break;
                    }
                    if fallback_seed == i32::MIN {
                        fallback_seed = signed_d;
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
            let sign = face_sign[current_id];
            for &edge_id in &face_rings[current_id].1 {
                let sym = edge_id ^ 1;
                if sym >= n_edges {
                    continue;
                }
                let adj = edge_to_face[sym];
                if adj == usize::MAX || face_depth[adj] != i32::MIN {
                    continue;
                }
                let e_delta = if edge_id < delta.len() { delta[edge_id] } else { 0 };
                face_depth[adj] = d - sign * e_delta;
                queue.push_back(adj);
            }
        }

        for face_id in 0..n_faces {
            if face_depth[face_id] == i32::MIN {
                face_depth[face_id] = face_sign[face_id]
                    * face_rings[face_id]
                    .1
                    .first()
                    .and_then(|&edge_id| delta.get(edge_id).copied())
                    .unwrap_or(0);
            }
        }

        if allow_negative_depth_inside {
            face_depth.into_iter().map(|d| d != 0).collect()
        } else {
            face_depth.into_iter().map(|d| d > 0).collect()
        }
    }
}

fn signed_ring_area(coords: &[Coord]) -> f64 {
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

fn dedupe_curve_roles(
    curves: Vec<LineString>,
    roles: Vec<CurveRole>,
    eps: f64,
) -> (Vec<LineString>, Vec<CurveRole>) {
    let scale = eps.abs().max(1.0e-12);
    let mut seen = HashSet::<String>::new();
    let mut out_curves = Vec::<LineString>::new();
    let mut out_roles = Vec::<CurveRole>::new();

    for (curve, role) in curves.into_iter().zip(roles.into_iter()) {
        let mut key = String::new();
        let role_tag = match role {
            CurveRole::LeftOffset => "L",
            CurveRole::RightOffset => "R",
            CurveRole::EndCap => "C",
            CurveRole::ExteriorOffset => "E",
            CurveRole::HoleOffset => "H",
        };
        key.push_str(role_tag);
        key.push('|');

        for c in &curve.coords {
            let qx = (c.x / scale).round() as i64;
            let qy = (c.y / scale).round() as i64;
            key.push_str(&format!("{qx}:{qy};"));
        }

        if seen.insert(key) {
            out_curves.push(curve);
            out_roles.push(role);
        }
    }

    (out_curves, out_roles)
}

impl Default for BufferOp {
    fn default() -> Self {
        Self {
            options: BufferOpOptions::default(),
        }
    }
}

/// Helper: detect if a coordinate path forms a closed ring.
fn is_line_closed(coords: &[Coord]) -> bool {
    if coords.len() < 4 {
        return false;
    }
    let eps = 1.0e-9;
    let dx = (coords[0].x - coords[coords.len() - 1].x).abs();
    let dy = (coords[0].y - coords[coords.len() - 1].y).abs();
    dx <= eps && dy <= eps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::measurements::polygon_area;

    fn ls(coords: &[(f64, f64)]) -> LineString {
        LineString::new(coords.iter().map(|(x, y)| Coord::xy(*x, *y)).collect())
    }

    fn total_area(polys: &[Polygon]) -> f64 {
        polys.iter().map(polygon_area).sum::<f64>().abs()
    }

    #[test]
    fn line_buffer_dissolve_produces_non_empty_output() {
        let op = BufferOp::default();
        let lines = vec![ls(&[(0.0, 0.0), (10.0, 0.0)])];
        let result = op.run_linestrings_dissolved(&lines, 1.0);
        assert!(
            !result.polygons.is_empty(),
            "expected dissolved line buffer to produce at least one polygon"
        );
        assert!(
            total_area(&result.polygons) > 0.0,
            "expected positive area from dissolved line buffer"
        );
    }

    #[test]
    fn geos_parity_coincident_lines_should_match_single_line_union_area() {
        let op = BufferOp::default();
        let single = vec![ls(&[(0.0, 0.0), (10.0, 0.0)])];
        let dup = vec![ls(&[(0.0, 0.0), (10.0, 0.0)]), ls(&[(0.0, 0.0), (10.0, 0.0)])];

        let a = op.run_linestrings_dissolved(&single, 1.0);
        let b = op.run_linestrings_dissolved(&dup, 1.0);

        let da = total_area(&a.polygons);
        let db = total_area(&b.polygons);
        assert!(
            (da - db).abs() <= 1.0e-9,
            "expected identical dissolved area for duplicated coincident input lines"
        );
    }
}

