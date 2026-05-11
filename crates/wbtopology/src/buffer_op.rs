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
    /// Current behavior keeps the staged architecture (curve generation -> global noding
    /// -> polygonization -> dissolve) while face-depth labeling is being integrated.
    ///
    /// This makes the execution flow explicit and testable without changing public
    /// tool semantics in one large rewrite.
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

        let curves = self.collect_raw_curves(lines, distance);
        stats.raw_curves = curves.len();
        if curves.is_empty() {
            return BufferOpResult {
                polygons: Vec::new(),
                stats,
            };
        }

        let source_polygons = self.collect_source_line_buffers(lines, distance);

        self.dissolve_curve_set(curves, &source_polygons, stats)
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

        let mut curves = Vec::<LineString>::new();
        for poly in polygons {
            curves.extend(buffer_polygon_curve_set(poly, distance, self.options.buffer));
        }
        stats.raw_curves = curves.len();

        let source_polygons = self.collect_source_polygon_buffers(polygons, distance);

        self.dissolve_curve_set(curves, &source_polygons, stats)
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
            self.classify_inside_face_rings(&graph, &face_rings_with_edges, source_polygons, epsilon);
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
    ) -> (Vec<LineString>, usize) {
        if face_rings_with_edges.is_empty() || source_polygons.is_empty() {
            return (Vec::new(), 0);
        }

        let mut edge_delta = vec![0i32; graph.edges.len()];
        self.compute_edge_deltas(graph, source_polygons, eps, &mut edge_delta);
        let included = self.classify_faces_by_depth(graph, face_rings_with_edges, &edge_delta);

        let mut selected = Vec::<LineString>::new();
        for ((ring, _), keep) in face_rings_with_edges.iter().zip(included.into_iter()) {
            if keep {
                selected.push(ring.clone());
            }
        }
        let selected_count = selected.len();
        (selected, selected_count)
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

        face_depth.into_iter().map(|d| d > 0).collect()
    }
}

impl Default for BufferOp {
    fn default() -> Self {
        Self {
            options: BufferOpOptions::default(),
        }
    }
}
