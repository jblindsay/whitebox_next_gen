//! BufferOp orchestration scaffold for global curve/noding/graph buffering.
//!
//! This module provides a restart-safe entry point for the vector `buffer_vector`
//! rewrite. The design mirrors GEOS/JTS BufferOp staging, but keeps behavior
//! conservative while the full face-label pipeline is being completed.

use std::collections::VecDeque;

use crate::algorithms::segment::point_on_segment_eps;
use crate::constructive::{
    buffer_linestring, buffer_linestring_curve_set, buffer_polygon_curve_set, buffer_polygon_multi,
    BufferOptions, PolygonizeOptions,
};
use crate::geom::{Coord, Geometry, LineString, Polygon};
use crate::graph::TopologyGraph;
use crate::noding::{node_linestrings_with_options, NodingOptions, NodingStrategy};
use crate::overlay::polygon_unary_union;
use crate::spatial_index::SpatialIndex;

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
    /// Uses role-based delta classification (mirrors GEOS/JTS): each offset curve
    /// is tagged with its role (LeftOffset/RightOffset/EndCap), and depth labeling
    /// uses curve roles to determine inside/outside, not source polygon containment.
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

        // Stage 1: Generate offset curves with role metadata.
        let (curves, roles) = self.collect_raw_curves_with_roles(lines, distance);
        stats.raw_curves = curves.len();
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 2-6: Noding, graph build, depth labeling using curve roles.
        self.dissolve_curve_set_with_roles(curves, roles, true, stats)
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
                // First two curves are left and right offsets (for open lines).
                // Remaining are end caps (if any) or ring offsets (if closed).
                let role = if line.coords.len() < 2 || !is_line_closed(&line.coords) {
                    // Open line: first is left, second is right, rest are caps or geometry
                    if idx == 0 {
                        CurveRole::LeftOffset
                    } else if idx == 1 {
                        CurveRole::RightOffset
                    } else {
                        CurveRole::EndCap
                    }
                } else {
                    // Closed line: first is exterior, rest are interior/holes
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
        let graph = TopologyGraph::from_linestrings_with_options(&noded, self.options.noding);
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
        stats.labeled_inside_polygons = selected_face_count;

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
        stats.candidate_polygons = poly_result.polygons.len();

        if poly_result.polygons.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let dissolved = polygon_unary_union(&poly_result.polygons, self.options.epsilon);
        stats.dissolved_polygons = dissolved.len();

        BufferOpResult {
            polygons: dissolved,
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

        // Use reference-point method: place reference points based on curve roles,
        // then classify each face directly using point-in-polygon testing.
        let reference_points = self.compute_reference_points(curves, roles);
        
        // Classify faces: a face is inside if it contains at least one reference point
        let mut selected = Vec::<LineString>::new();
        for (face_id, (ring, _)) in face_rings_with_edges.iter().enumerate() {
            let mut is_inside = false;
            for ref_point_opt in &reference_points {
                if let Some(ref_point) = ref_point_opt {
                    if self.point_in_polygon_ring(*ref_point, &ring.coords, eps) {
                        is_inside = true;
                        break;
                    }
                }
            }
            if is_inside {
                selected.push(ring.clone());
            }
        }
        
        let selected_count = selected.len();
        (selected, selected_count)
    }

    fn compute_reference_points(
        &self,
        curves: &[LineString],
        roles: &[CurveRole],
    ) -> Vec<Option<Coord>> {
        let mut refs = Vec::with_capacity(curves.len());
        
        for (curve_idx, (curve, role)) in curves.iter().zip(roles.iter()).enumerate() {
            if curve.coords.len() < 2 {
                refs.push(None);
                continue;
            }
            
            // Generate reference points based on role.
            // Only meaningful roles get reference points:
            // - LeftOffset: point offset left of curve
            // - RightOffset: point offset right of curve
            // - ExteriorOffset: point at centroid
            // - HoleOffset: point at centroid
            // - EndCap: no reference point
            
            let ref_point = match role {
                CurveRole::LeftOffset | CurveRole::RightOffset => {
                    // Use the centroid of the curve, then offset perpendicular based on role.
                    let mut sum_x = 0.0;
                    let mut sum_y = 0.0;
                    for c in &curve.coords {
                        sum_x += c.x;
                        sum_y += c.y;
                    }
                    let centroid = Coord::xy(sum_x / curve.coords.len() as f64, sum_y / curve.coords.len() as f64);
                    
                    // Get a direction vector from the curve (use first two points).
                    let p0 = curve.coords[0];
                    let p1 = curve.coords[curve.coords.len() - 1];
                    let dx = p1.x - p0.x;
                    let dy = p1.y - p0.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    
                    if len < 1.0e-9 {
                        Some(centroid)
                    } else {
                        // Perpendicular direction.
                        let px = -dy / len;
                        let py = dx / len;
                        
                        // Use larger offset to ensure point is definitely inside.
                        let offset = 5.0;
                        let offset_mult = match role {
                            CurveRole::LeftOffset => 1.0,
                            CurveRole::RightOffset => -1.0,
                            _ => 0.0,
                        };
                        
                        Some(Coord::xy(
                            centroid.x + px * offset_mult * offset,
                            centroid.y + py * offset_mult * offset,
                        ))
                    }
                }
                CurveRole::ExteriorOffset | CurveRole::HoleOffset => {
                    // Use ring centroid for exterior/hole offsets.
                    let mut sum_x = 0.0;
                    let mut sum_y = 0.0;
                    for c in &curve.coords {
                        sum_x += c.x;
                        sum_y += c.y;
                    }
                    Some(Coord::xy(sum_x / curve.coords.len() as f64, sum_y / curve.coords.len() as f64))
                }
                CurveRole::EndCap => {
                    // End caps don't define inside/outside.
                    None
                }
            };
            
            refs.push(ref_point);
        }
        
        refs
    }

    fn compute_edge_deltas_from_reference_points(
        &self,
        graph: &TopologyGraph,
        face_rings_with_edges: &[(LineString, Vec<usize>)],
        reference_points: &[Option<Coord>],
        eps: f64,
        delta: &mut [i32],
    ) {
        // Use reference points to directly classify which faces are "inside" the buffer.
        // Don't use BFS; classify each face independently.
        let mut face_is_inside = vec![false; face_rings_with_edges.len()];
        
        for (curve_idx, ref_point_opt) in reference_points.iter().enumerate() {
            if let Some(ref_point) = ref_point_opt {
                // Test which faces contain this reference point.
                for (face_id, (ring, _)) in face_rings_with_edges.iter().enumerate() {
                    if self.point_in_polygon_ring(*ref_point, &ring.coords, eps) {
                        face_is_inside[face_id] = true;
                    }
                }
            }
        }
        
        // Set delta=1 for edges of inside faces, delta=-1 for outside faces.
        // The BFS in classify_faces_by_depth expects to propagate from exterior (depth=0).
        // Edges between inside and outside faces should have non-zero delta.
        for (face_id, inside) in face_is_inside.iter().enumerate() {
            if !*inside {
                continue;
            }
            
            // For each edge of this inside face, check its twin in the adjacent face.
            for &edge_id in &face_rings_with_edges[face_id].1 {
                let sym = edge_id ^ 1;
                if sym < graph.edges.len() {
                    // Mark this edge as belonging to inside: delta=1
                    // This tells BFS: "crossing this edge means entering inside buffer"
                    delta[edge_id] = 1;
                    delta[sym] = -1;
                }
            }
        }
    }

    fn point_in_polygon_ring(&self, p: Coord, ring: &[Coord], eps: f64) -> bool {
        if ring.len() < 4 {
            return false;
        }
        
        // Ray casting algorithm.
        let mut crossings = 0;
        let ray_p = Coord::xy(p.x, p.y + 1e6); // ray going upward
        
        for i in 0..(ring.len() - 1) {
            let a = ring[i];
            let b = ring[i + 1];
            
            if self.segments_intersect_ray(a, b, p, ray_p, eps) {
                crossings += 1;
            }
        }
        
        crossings % 2 == 1
    }

    fn segments_intersect_ray(
        &self,
        a: Coord,
        b: Coord,
        ray_start: Coord,
        ray_end: Coord,
        eps: f64,
    ) -> bool {
        // Check if segment ab intersects the ray from ray_start to ray_end.
        let (dx1, dy1) = (b.x - a.x, b.y - a.y);
        let (dx2, dy2) = (ray_end.x - ray_start.x, ray_end.y - ray_start.y);
        
        let denom = dx1 * dy2 - dy1 * dx2;
        if denom.abs() < eps {
            return false;
        }
        
        let (dx3, dy3) = (ray_start.x - a.x, ray_start.y - a.y);
        let t = (dx3 * dy2 - dy3 * dx2) / denom;
        let u = (dx3 * dy1 - dy3 * dx1) / denom;
        
        t >= eps && t <= (1.0 - eps) && u >= eps
    }

    fn collect_raw_curves(&self, lines: &[LineString], distance: f64) -> Vec<LineString> {
        let mut curves = Vec::<LineString>::new();
        for line in lines {
            curves.extend(buffer_linestring_curve_set(line, distance, self.options.buffer));
        }
        curves
    }

    fn collect_source_line_buffers(&self, lines: &[LineString], distance: f64) -> Vec<Polygon> {
        let mut polys = Vec::<Polygon>::new();
        for line in lines {
            let poly = buffer_linestring(line, distance, self.options.buffer);
            if poly.exterior.coords.len() >= 4 {
                polys.push(poly);
            }
        }
        polys
    }

    fn collect_source_polygon_buffers(&self, polygons: &[Polygon], distance: f64) -> Vec<Polygon> {
        let mut polys = Vec::<Polygon>::new();
        for poly in polygons {
            polys.extend(
                buffer_polygon_multi(poly, distance, self.options.buffer)
                    .into_iter()
                    .filter(|p| p.exterior.coords.len() >= 4),
            );
        }
        polys
    }

    fn dissolve_curve_set(
        &self,
        curves: Vec<LineString>,
        source_polygons: &[Polygon],
        allow_negative_depth_inside: bool,
        mut stats: BufferOpStats,
    ) -> BufferOpResult {
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let noded = node_linestrings_with_options(&curves, self.options.noding);
        stats.noded_curves = noded.len();
        if noded.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        // Stage 4: build a planar graph from globally noded curves and extract
        // bounded face rings for downstream labeling/ring assembly work.
        let epsilon = self.options.epsilon.max(self.options.noding.epsilon).max(1.0e-9);
        let graph = TopologyGraph::from_linestrings_with_options(&noded, self.options.noding);
        let face_rings_with_edges = graph.extract_bounded_face_rings_with_edges(epsilon);
        stats.face_rings = face_rings_with_edges.len();

        let (selected_face_rings, selected_face_count) =
            self.classify_inside_face_rings(
                &graph,
                &face_rings_with_edges,
                source_polygons,
                epsilon,
                allow_negative_depth_inside,
            );
        stats.labeled_inside_polygons = selected_face_count;

        // Prefer depth-labeled face rings when available.
        // Fallback order retains prior conservative behavior during staged rollout.
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
        stats.candidate_polygons = poly_result.polygons.len();

        if poly_result.polygons.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let dissolved = polygon_unary_union(&poly_result.polygons, self.options.epsilon);
        stats.dissolved_polygons = dissolved.len();

        BufferOpResult {
            polygons: dissolved,
            stats,
        }
    }

    fn classify_inside_face_rings(
        &self,
        graph: &TopologyGraph,
        face_rings_with_edges: &[(LineString, Vec<usize>)],
        source_polygons: &[Polygon],
        eps: f64,
        allow_negative_depth_inside: bool,
    ) -> (Vec<LineString>, usize) {
        if face_rings_with_edges.is_empty() || source_polygons.is_empty() {
            return (Vec::new(), 0);
        }

        let mut edge_delta = vec![0i32; graph.edges.len()];
        self.compute_edge_deltas(graph, source_polygons, eps, &mut edge_delta);
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

    fn compute_edge_deltas(
        &self,
        graph: &TopologyGraph,
        source_polygons: &[Polygon],
        eps: f64,
        delta: &mut [i32],
    ) {
        let source_geoms: Vec<Geometry> = source_polygons
            .iter()
            .cloned()
            .map(Geometry::Polygon)
            .collect();
        let source_index = SpatialIndex::from_geometries(&source_geoms);

        let mut edge_id = 0usize;
        while edge_id < graph.edges.len() {
            let edge = &graph.edges[edge_id];
            let a = graph.nodes[edge.from].coord;
            let b = graph.nodes[edge.to].coord;
            let midpoint = Coord::xy((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
            let dx = b.x - a.x;
            let dy = b.y - a.y;

            let mut d = 0i32;
            for poly_idx in source_index.query_point(midpoint) {
                if poly_idx >= source_polygons.len() {
                    continue;
                }
                let poly = &source_polygons[poly_idx];
                let ext_ccw = self.ring_signed_area(&poly.exterior.coords) >= 0.0;
                d += self.ring_segment_delta(&poly.exterior.coords, midpoint, dx, dy, ext_ccw, eps);
                for hole in &poly.holes {
                    let hole_ccw = self.ring_signed_area(&hole.coords) >= 0.0;
                    d += self.ring_segment_delta(&hole.coords, midpoint, dx, dy, hole_ccw, eps);
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

    fn ring_segment_delta(
        &self,
        ring: &[Coord],
        midpoint: Coord,
        dx: f64,
        dy: f64,
        ring_ccw: bool,
        eps: f64,
    ) -> i32 {
        let ring_sign: i32 = if ring_ccw { 1 } else { -1 };
        if ring.len() < 2 {
            return 0;
        }
        for i in 0..(ring.len() - 1) {
            let c = ring[i];
            let d = ring[i + 1];
            if !point_on_segment_eps(midpoint, c, d, eps) {
                continue;
            }
            let cdx = d.x - c.x;
            let cdy = d.y - c.y;
            let dot = dx * cdx + dy * cdy;
            let dir_sign: i32 = if dot >= 0.0 { 1 } else { -1 };
            return ring_sign * dir_sign;
        }
        0
    }

    fn ring_signed_area(&self, coords: &[Coord]) -> f64 {
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

        for face_id in 0..n_faces {
            let mut seed = i32::MIN;
            let mut fallback_seed = i32::MIN;
            for &edge_id in &face_rings[face_id].1 {
                let sym = edge_id ^ 1;
                if sym < n_edges && edge_to_face[sym] == usize::MAX {
                    let d = if edge_id < delta.len() { delta[edge_id] } else { 0 };
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
                face_depth[adj] = d - e_delta;
                queue.push_back(adj);
            }
        }

        for face_id in 0..n_faces {
            if face_depth[face_id] == i32::MIN {
                face_depth[face_id] = face_rings[face_id]
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

