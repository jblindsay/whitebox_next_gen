//! Constructive geometry utilities.
//!
//! This module provides pragmatic building blocks for Phase 2 parity work:
//! - polygon repair (`make_valid_polygon`)
//! - polygonization from closed linestrings (`polygonize_closed_linestrings`)
//! - point buffering (`buffer_point`)

use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::algorithms::distance::geometry_distance;
use crate::algorithms::segment::segments_intersect_eps;
use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};
use crate::graph::TopologyGraph;
use crate::noding::{node_linestrings_with_options, NodingOptions, NodingStrategy};
use crate::overlay::{polygon_unary_union, polygon_union, polygon_union_with_precision};
use crate::precision::{PrecisionModel, TopologyPrecisionOptions};
use crate::topology::is_valid_polygon;

/// Buffer end-cap style for linear geometries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferCapStyle {
    /// Rounded caps.
    Round,
    /// Flat caps at line endpoints.
    Flat,
    /// Square caps extending half-width beyond endpoints.
    Square,
}

/// Buffer join style for connected segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferJoinStyle {
    /// Rounded joins.
    Round,
    /// Mitre joins.
    Mitre,
    /// Bevel joins.
    Bevel,
}

/// Buffer generation options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferOptions {
    /// Number of segments per quarter circle.
    pub quadrant_segments: usize,
    /// Line end-cap style.
    pub cap_style: BufferCapStyle,
    /// Segment join style.
    pub join_style: BufferJoinStyle,
    /// Maximum ratio of mitre length to buffer distance.
    ///
    /// Used only when `join_style` is `Mitre`. If the computed mitre point is
    /// farther than `mitre_limit * distance` from the source vertex, the join
    /// falls back to a bevel.
    pub mitre_limit: f64,
}

impl Default for BufferOptions {
    fn default() -> Self {
        Self {
            quadrant_segments: 8,
            cap_style: BufferCapStyle::Round,
            join_style: BufferJoinStyle::Round,
            mitre_limit: 5.0,
        }
    }
}

/// Buffer construction pipeline selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPipelineStrategy {
    /// Existing constructive buffer implementation.
    Legacy,
    /// Graph-oriented buffer builder pipeline.
    GraphBuilder,
}

/// Which side of a linestring to offset toward.
///
/// "Left" and "Right" are defined relative to the direction of travel along the
/// input linestring (i.e. the direction from the first coordinate to the last).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetSide {
    /// Left side when facing the direction of travel (positive normal direction).
    Left,
    /// Right side when facing the direction of travel (negative normal direction).
    Right,
}

/// Options for one-sided offset curve generation via [`offset_linestring`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetCurveOptions {
    /// Number of segments per quarter circle (used for `Round` joins).
    pub quadrant_segments: usize,
    /// Join style at intermediate vertices.
    pub join_style: BufferJoinStyle,
    /// Maximum mitre ratio (used when `join_style == Mitre`).
    ///
    /// If the computed mitre point exceeds `mitre_limit * distance` from the
    /// source vertex, the join falls back to bevel.
    pub mitre_limit: f64,
}

impl Default for OffsetCurveOptions {
    fn default() -> Self {
        Self {
            quadrant_segments: 8,
            join_style: BufferJoinStyle::Round,
            mitre_limit: 5.0,
        }
    }
}

/// Builder for buffering operations with explicit robustness controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferBuilder {
    /// Geometric buffer style options.
    pub options: BufferOptions,
    /// Optional precision model for curve-set noding.
    pub precision: Option<PrecisionModel>,
    /// Noding options used by graph-oriented pipeline.
    pub noding: NodingOptions,
    /// Pipeline strategy.
    pub pipeline: BufferPipelineStrategy,
}

impl BufferBuilder {
    /// Build a new buffer builder from base buffer options.
    pub fn new(options: BufferOptions) -> Self {
        Self {
            options,
            precision: None,
            noding: NodingOptions {
                epsilon: 1.0e-9,
                strategy: NodingStrategy::SnapRounding,
                precision: None,
            },
            pipeline: BufferPipelineStrategy::GraphBuilder,
        }
    }

    /// Set precision model for topology pre-processing.
    pub fn with_precision(mut self, precision: PrecisionModel) -> Self {
        self.precision = Some(precision);
        self.noding.precision = Some(precision);
        self
    }

    /// Set noding options for graph-oriented buffering.
    pub fn with_noding(mut self, noding: NodingOptions) -> Self {
        self.noding = noding;
        self
    }

    /// Set buffer pipeline strategy.
    pub fn with_pipeline(mut self, pipeline: BufferPipelineStrategy) -> Self {
        self.pipeline = pipeline;
        self
    }

    /// Build a polygon buffer using the selected pipeline.
    pub fn build_polygon(self, poly: &Polygon, distance: f64) -> Polygon {
        match self.pipeline {
            BufferPipelineStrategy::Legacy => buffer_polygon_legacy_impl(poly, distance, self.options),
            BufferPipelineStrategy::GraphBuilder => self.build_polygon_graph(poly, distance),
        }
    }

    fn build_polygon_graph(self, poly: &Polygon, distance: f64) -> Polygon {
        // Negative/zero buffer semantics remain on the legacy path.
        // The legacy negative buffer is well-optimized: it handles self-intersecting
        // offset rings via make_valid_polygon (splitting into components), has robust
        // collapse detection, and produces correct multi-component results. The buffer_polygon_multi
        // function returns all erosion components; buffer_polygon returns only the largest.
        // Graph pipeline conversion for negative buffer is deferred as a future optimization.
        if distance <= 0.0 {
            return buffer_polygon_legacy_impl(poly, distance, self.options);
        }

        // Stage 1: Build curve set from source boundaries.
        let curves = build_polygon_buffer_curve_set(poly, distance, self.options);
        if curves.is_empty() {
            return buffer_polygon_legacy_impl(poly, distance, self.options);
        }

        // Stage 2: Node generated curves under explicit precision/noding settings.
        let noded_curves = node_linestrings_with_options(&curves, self.noding);
        if noded_curves.is_empty() {
            return buffer_polygon_legacy_impl(poly, distance, self.options);
        }

        // Stage 3: Build planar graph and extract bounded faces.
        let graph = TopologyGraph::from_linestrings_with_options(&noded_curves, self.noding);
        let face_rings = graph.extract_bounded_face_rings(self.noding.epsilon.max(1.0e-9));
        if face_rings.is_empty() {
            return buffer_polygon_legacy_impl(poly, distance, self.options);
        }

        // Stage 4: Assemble labeled faces into polygons.
        let poly_result = polygonize_linework(
            &face_rings,
            PolygonizeOptions {
                epsilon: self.noding.epsilon,
                noding: self.noding,
            },
        );
        if poly_result.polygons.is_empty() {
            return buffer_polygon_legacy_impl(poly, distance, self.options);
        }

        // Stage 5: Depth-style face filtering against source polygon.
        let selected = select_buffer_polygons_by_depth(
            &poly_result.polygons,
            poly,
            distance,
            self.noding.epsilon.max(1.0e-9),
        );

        // Skip component merging for buffer operations. The depth-sorting already
        // prioritizes the correct outer buffer by inside_count and area. Merging
        // via polygon_union can corrupt results in boundary/collapsed-hole cases.
        // Instead, just take the first depth-sorted candidate with reasonable area.
        let best_candidate = selected
            .into_iter()
            .find(|p| {
                let p_area = polygon_abs_area(p);
                if p_area < 1.0e-3 {
                    return false;
                }
                // For positive buffers, the result should be substantially larger than source.
                // Intermediate stage faces will be only slightly larger.
                // Approximation: buffer distance d expands perimeter, so rough estimate is
                // that area grows by ~perimeter * d. For a square 10x10 with d=2.5,
                // growth is ~40*2.5 = 100, so buffered area ~200 vs source area 100.
                // Accept if at least 1.5x source exterior area (loose threshold).
                if distance > 0.0 {
                    let src_ext_area = ring_abs_area(&poly.exterior.coords);
                    if p_area < src_ext_area * 1.2 {
                        return false;
                    }
                }
                true
            });
        
        if let Some(out) = best_candidate {
            // Sanity check: for positive buffers the output polygon must be at
            // least as large as the source's exterior ring.  If the graph
            // pipeline selected a wrong component (e.g., a tiny face in a
            // collapsed-hole region), reject it and fall back to legacy.
            if distance > 0.0 {
                let src_ext_area = ring_abs_area(&poly.exterior.coords);
                let out_area = polygon_abs_area(&out);
                if out_area < src_ext_area * 0.9 {
                    return buffer_polygon_legacy_impl(poly, distance, self.options);
                }
            }
            if !is_valid_polygon(&out) {
                return buffer_polygon_legacy_impl(poly, distance, self.options);
            }
            // If the source polygon has holes, attach contracted hole rings to the
            // graph-pipeline outer shell.  The graph pipeline's segment-buffer
            // approach does not distinguish hole rings from exterior rings when
            // building the curve set, so it cannot reliably reconstruct contracted
            // holes.  The legacy inward-offset logic is exact and cheap for this
            // step.
            if !poly.holes.is_empty() && distance > 0.0 {
                buffer_polygon_attach_holes(out, poly, distance, self.options)
            } else {
                out
            }
        } else {
            buffer_polygon_legacy_impl(poly, distance, self.options)
        }
    }
}

/// Geometry fixing strategy selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryFixMode {
    /// Prioritize repairing ring/polygon structure first.
    StructureFirst,
    /// Prioritize noded linework polygonization first.
    LineworkFirst,
}

/// Options controlling make-valid behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryFixOptions {
    /// Predicate epsilon.
    pub epsilon: f64,
    /// Strategy mode.
    pub mode: GeometryFixMode,
    /// Preserve collapsed remnants when possible.
    pub keep_collapsed: bool,
}

impl Default for GeometryFixOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.0e-9,
            mode: GeometryFixMode::StructureFirst,
            keep_collapsed: false,
        }
    }
}

/// Options controlling full linework polygonization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolygonizeOptions {
    /// Predicate epsilon.
    pub epsilon: f64,
    /// Noding options used before graph polygon assembly.
    pub noding: NodingOptions,
}

impl Default for PolygonizeOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.0e-9,
            noding: NodingOptions::default(),
        }
    }
}

/// Result bundle for linework polygonization.
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonizeResult {
    /// Output polygons.
    pub polygons: Vec<Polygon>,
    /// Dangle segments (dangling ends).
    pub dangles: Vec<LineString>,
    /// Cut edges not used in bounded faces.
    pub cut_edges: Vec<LineString>,
    /// Rings rejected as invalid.
    pub invalid_rings: Vec<LineString>,
}

/// Build an approximate circular buffer polygon around a point.
///
/// Negative and zero distances produce an empty polygon.
pub fn buffer_point(center: Coord, distance: f64, options: BufferOptions) -> Polygon {
    if !distance.is_finite() || distance <= 0.0 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let mut coords = Vec::<Coord>::with_capacity(segs + 1);

    for i in 0..segs {
        let t = (i as f64) * std::f64::consts::TAU / (segs as f64);
        coords.push(Coord::xy(
            center.x + distance * t.cos(),
            center.y + distance * t.sin(),
        ));
    }

    Polygon::new(LinearRing::new(coords), vec![])
}

/// Build a buffer polygon around a linestring.
///
/// Constructs left/right offset curves with configurable joins and end caps.
/// This is a practical offset-curve implementation intended to approximate JTS
/// line buffering semantics more closely than a convex-hull approximation.
pub fn buffer_linestring(ls: &LineString, distance: f64, options: BufferOptions) -> Polygon {
    if !distance.is_finite() || distance <= 0.0 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }
    if ls.coords.is_empty() {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }
    if ls.coords.len() == 1 {
        return buffer_point(ls.coords[0], distance, options);
    }

    let mut coords = sanitize_path(&ls.coords);
    if coords.len() < 2 {
        return buffer_point(coords[0], distance, options);
    }

    // Closed centreline loops buffer as corridor rings with no end caps.
    // Treating them as open lines fills the loop interior and loses the hole.
    //
    // Real-world data rarely stores an exact first==last vertex; use a
    // distance-proportional tolerance so near-closed loops are detected even
    // when endpoints differ by floating-point noise or road-network snapping.
    // Any gap smaller than 5% of the buffer distance will look closed visually.
    let close_thresh2 = (distance * 0.05).powi(2).max(1.0e-24);
    let closed_dist2 = if coords.len() >= 4 {
        coord_dist2(coords[0], coords[coords.len() - 1])
    } else {
        f64::INFINITY
    };
    if coords.len() >= 4 && closed_dist2 <= close_thresh2 {
        // Snap near-closed endpoints together so ring offsetting does not
        // retain a tiny closure segment that can invalidate the inner hole.
        if closed_dist2 > 1.0e-24 {
            let last = coords.len() - 1;
            coords[last] = coords[0];
        }
        let segs = (options.quadrant_segments.max(2) * 4).max(8);
        let shell_coords = build_offset_ring(
            &coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
            true,
        );
        if shell_coords.len() < 4 {
            return Polygon::new(LinearRing::new(vec![]), vec![]);
        }

        let shell = LinearRing::new(shell_coords);
        let mut holes = Vec::<LinearRing>::new();

        let hole_coords = build_offset_ring(
            &coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
            false,
        );
        if hole_coords.len() >= 4 {
            // ISO 19125: holes must have opposite winding order from exterior.
            // build_offset_ring traverses input in same order for both outer/inner,
            // so we must reverse the hole to get opposite winding.
            let mut reversed_hole_coords = hole_coords.clone();
            reversed_hole_coords.reverse();
            // Close the reversed ring.
            if reversed_hole_coords.first() != reversed_hole_coords.last() {
                reversed_hole_coords.push(reversed_hole_coords[0]);
            } else {
                let first = reversed_hole_coords[0];
                let last_idx = reversed_hole_coords.len() - 1;
                reversed_hole_coords[last_idx] = first;
            }
            let hole = LinearRing::new(reversed_hole_coords);
            let eps = 1.0e-9;
            if is_ring_simple_eps(&hole.coords, eps)
                && ring_abs_area(&hole.coords) > eps * eps
                && point_in_ring_inclusive_eps(hole.coords[0], &shell.coords, eps)
                && !ring_boundary_intersects_eps(&shell.coords, &hole.coords, eps)
            {
                holes.push(hole);
            } else {
                // GEOS/JTS resolves closed-line buffer holes from noded offset
                // linework rather than dropping them when a direct inner-offset
                // ring fails local validation. Mirror that behavior narrowly here.
                let fallback = polygonize_linework(
                    &[
                        LineString::new(shell.coords.clone()),
                        LineString::new(hole.coords.clone()),
                    ],
                    PolygonizeOptions {
                        epsilon: eps,
                        ..Default::default()
                    },
                );
                if let Some(poly) = fallback.polygons.into_iter().max_by(|a, b| {
                    ring_abs_area(&a.exterior.coords)
                        .total_cmp(&ring_abs_area(&b.exterior.coords))
                }) {
                    return repair_buffer_polygon(poly, eps);
                }
            }
        }

        return repair_buffer_polygon(Polygon::new(shell, holes), 1.0e-9);
    }

    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let left = build_offset_side(
        &coords,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
    );

    coords.reverse();
    let right = build_offset_side(
        &coords,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
    );
    coords.reverse();

    if left.len() < 2 || right.len() < 2 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let mut ring = Vec::<Coord>::new();
    ring.extend(left.iter().copied());

    // End cap (from left end to right start)
    append_cap(
        &mut ring,
        coords[coords.len() - 1],
        unit_dir(coords[coords.len() - 2], coords[coords.len() - 1]),
        distance,
        options.cap_style,
        segs,
        true,
    );

    ring.extend(right.iter().copied());

    // Start cap (from right end to left start)
    append_cap(
        &mut ring,
        coords[0],
        unit_dir(coords[0], coords[1]),
        distance,
        options.cap_style,
        segs,
        false,
    );

    // Deduplicate adjacent repeats and build closed ring.
    let mut cleaned = Vec::<Coord>::with_capacity(ring.len());
    for p in ring {
        if cleaned
            .last()
            .map(|q| coord_dist2(*q, p) <= 1.0e-24)
            .unwrap_or(false)
        {
            continue;
        }
        cleaned.push(p);
    }

    if cleaned.len() < 3 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let poly = Polygon::new(LinearRing::new(cleaned), vec![]);
    buffer_linestring_graph_repair(poly, 1.0e-9)
}

/// Build raw closed buffer boundary curves for a linestring.
///
/// This exposes the GEOS/JTS-style "curve set" stage for callers that want
/// to node and polygonize many buffered lines together in one global pass,
/// instead of buffering each feature into a polygon and dissolving afterward.
pub fn buffer_linestring_curve_set(
    ls: &LineString,
    distance: f64,
    options: BufferOptions,
) -> Vec<LineString> {
    if !distance.is_finite() || distance <= 0.0 {
        return Vec::new();
    }
    if ls.coords.is_empty() {
        return Vec::new();
    }
    if ls.coords.len() == 1 {
        let poly = buffer_point(ls.coords[0], distance, options);
        return polygon_boundaries_as_lines(&poly);
    }

    let mut coords = sanitize_path(&ls.coords);
    if coords.len() < 2 {
        return vec![LineString::new(vec![])];
    }

    let close_thresh2 = (distance * 0.05).powi(2).max(1.0e-24);
    let closed_dist2 = if coords.len() >= 4 {
        coord_dist2(coords[0], coords[coords.len() - 1])
    } else {
        f64::INFINITY
    };
    if coords.len() >= 4 && closed_dist2 <= close_thresh2 {
        if closed_dist2 > 1.0e-24 {
            let last = coords.len() - 1;
            coords[last] = coords[0];
        }
        let segs = (options.quadrant_segments.max(2) * 4).max(8);
        let mut out = Vec::<LineString>::new();

        let shell_coords = build_offset_ring(
            &coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
            true,
        );
        if shell_coords.len() >= 4 {
            out.push(LineString::new(shell_coords));
        }

        let hole_coords = build_offset_ring(
            &coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
            false,
        );
        if hole_coords.len() >= 4 {
            out.push(LineString::new(hole_coords));
        }
        return out;
    }

    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let left = build_offset_side(
        &coords,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
    );

    coords.reverse();
    let right = build_offset_side(
        &coords,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
    );
    coords.reverse();

    if left.len() < 2 || right.len() < 2 {
        return Vec::new();
    }

    let mut ring = Vec::<Coord>::new();
    ring.extend(left.iter().copied());

    append_cap(
        &mut ring,
        coords[coords.len() - 1],
        unit_dir(coords[coords.len() - 2], coords[coords.len() - 1]),
        distance,
        options.cap_style,
        segs,
        true,
    );

    ring.extend(right.iter().copied());

    append_cap(
        &mut ring,
        coords[0],
        unit_dir(coords[0], coords[1]),
        distance,
        options.cap_style,
        segs,
        false,
    );

    let mut cleaned = Vec::<Coord>::with_capacity(ring.len());
    for p in ring {
        if cleaned
            .last()
            .map(|q| coord_dist2(*q, p) <= 1.0e-24)
            .unwrap_or(false)
        {
            continue;
        }
        cleaned.push(p);
    }

    if cleaned.len() < 3 {
        return Vec::new();
    }

    vec![LineString::new(cleaned)]
}

/// Generate a one-sided offset curve for a linestring.
///
/// Unlike [`buffer_linestring`], which produces a closed polygon corridor,
/// this function returns an open `LineString` representing only one side of
/// the offset. It is analogous to JTS/GEOS `OffsetCurve` and is suitable for:
///
/// - Road edge and lane boundary extraction
/// - Hydration buffer centreline offsets
/// - Planning setback lines
/// - Any workflow that requires the raw offset geometry, not a filled polygon
///
/// # Parameters
///
/// - `ls`: input linestring
/// - `distance`: magnitude of the offset; must be positive and finite
/// - `side`: [`OffsetSide::Left`] (left when facing direction of travel) or
///   [`OffsetSide::Right`] (right when facing direction of travel)
/// - `options`: join style and resolution controls via [`OffsetCurveOptions`]
///
/// # Returns
///
/// An open `LineString`. The number of output vertices equals the number of
/// input vertices for `Mitre`/`Bevel` joins; `Round` joins insert additional
/// arc vertices at convex turns. Returns an empty `LineString` when:
/// - `distance` is zero, negative, or non-finite
/// - the input has fewer than two distinct coordinates
///
/// # Notes
///
/// - Single-point inputs return an empty `LineString`.
/// - Self-intersecting input curves are not split; the offset curve may also
///   self-intersect. Post-process with [`make_valid_geometry`] if needed.
/// - Use [`buffer_linestring`] when you need a closed polygon (corridor buffer).
///
/// # Example
///
/// ```
/// use wbtopology::{offset_linestring, OffsetSide, OffsetCurveOptions, LineString, Coord};
///
/// let road = LineString::new(vec![
///     Coord::xy(0.0, 0.0),
///     Coord::xy(100.0, 0.0),
///     Coord::xy(200.0, 50.0),
/// ]);
/// // 5-metre left kerb line
/// let left_edge = offset_linestring(&road, 5.0, OffsetSide::Left, OffsetCurveOptions::default());
/// assert!(left_edge.coords.len() >= 3);
/// ```
pub fn offset_linestring(
    ls: &LineString,
    distance: f64,
    side: OffsetSide,
    options: OffsetCurveOptions,
) -> LineString {
    if !distance.is_finite() || distance <= 0.0 {
        return LineString::new(vec![]);
    }
    if ls.coords.len() < 2 {
        return LineString::new(vec![]);
    }

    let mut coords = sanitize_path(&ls.coords);
    if coords.len() < 2 {
        return LineString::new(vec![]);
    }

    let segs = (options.quadrant_segments.max(2) * 4).max(8);

    // Left side: positive normal direction (standard build_offset_side direction).
    // Right side: reverse the path so the right side becomes the left, compute,
    // then reverse the result to restore the original travel direction.
    let offset_coords = match side {
        OffsetSide::Left => build_offset_side(
            &coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
        ),
        OffsetSide::Right => {
            coords.reverse();
            let mut result = build_offset_side(
                &coords,
                distance,
                options.join_style,
                segs,
                options.mitre_limit,
            );
            // Restore original direction so the output linestring runs from the
            // input's start to its end.
            result.reverse();
            result
        }
    };

    if offset_coords.len() < 2 {
        return LineString::new(vec![]);
    }

    // Deduplicate adjacent near-identical points (produced by collinear segments).
    let mut cleaned = Vec::<Coord>::with_capacity(offset_coords.len());
    for p in offset_coords {
        if cleaned
            .last()
            .map(|q| coord_dist2(*q, p) <= 1.0e-24)
            .unwrap_or(false)
        {
            continue;
        }
        cleaned.push(p);
    }

    LineString::new(cleaned)
}

/// Buffer a polygon by the given distance.
///
/// Positive distances expand the shell and shrink holes. Zero distance returns
/// a repaired copy of the input polygon. Negative distances erode the shell and
/// expand holes; when erosion collapses the polygon or would require a
/// multipolygon result, the largest surviving component or an empty polygon is
/// returned.
pub fn buffer_polygon(poly: &Polygon, distance: f64, options: BufferOptions) -> Polygon {
    BufferBuilder::new(options)
        .with_pipeline(BufferPipelineStrategy::GraphBuilder)
        .build_polygon(poly, distance)
}

/// Repair a raw linestring-buffer ring using the graph pipeline when the ring is
/// self-intersecting.  For simple rings this is a no-op pass-through.  For rings
/// with self-intersections (acute-angle joins, very short segments, complex paths)
/// this nodes the ring, extracts bounded face rings, assembles valid polygons, and
/// returns the largest result — equivalent to what the polygon BufferBuilder pipeline
/// does via `polygonize_closed_linestrings`.
fn buffer_linestring_graph_repair(poly: Polygon, eps: f64) -> Polygon {
    if is_ring_simple_eps(&poly.exterior.coords, eps) {
        return poly;
    }

    let ring_ls = LineString::new(poly.exterior.coords.clone());
    let noded = node_linestrings_with_options(
        &[ring_ls],
        NodingOptions {
            epsilon: eps,
            strategy: NodingStrategy::SnapRounding,
            precision: None,
        },
    );
    if noded.is_empty() {
        return repair_buffer_polygon(poly, eps);
    }

    let face_rings = TopologyGraph::from_linestrings(&noded, eps).extract_bounded_face_rings(eps);
    if face_rings.is_empty() {
        return repair_buffer_polygon(poly, eps);
    }

    let polys = polygonize_closed_linestrings(&face_rings, eps);
    if polys.is_empty() {
        return repair_buffer_polygon(poly, eps);
    }

    // A self-crossing raw corridor can polygonize into multiple bounded faces.
    // Keeping only the single largest face drops valid pieces near sharp bends,
    // which shows up as inner wedge cutouts and the matching lost outer round
    // lobe. Dissolve all bounded faces first, then select the largest dissolved
    // component as the repaired corridor.
    let dissolved = polygon_unary_union(&polys, eps);
    dissolved
        .into_iter()
        .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
        .unwrap_or_else(|| repair_buffer_polygon(poly, eps))
}

/// Attach contracted hole rings to a graph-pipeline outer shell for a positive buffer.
///
/// The graph pipeline builds its curve set from all source rings (exterior + holes)
/// symmetrically, so it does not inherently contract holes.  This helper applies the
/// same inward-offset hole logic as `buffer_polygon_legacy_impl` and returns the shell
/// polygon with any surviving contracted hole rings attached.
fn buffer_polygon_attach_holes(
    shell_poly: Polygon,
    source: &Polygon,
    distance: f64,
    options: BufferOptions,
) -> Polygon {
    let shell = shell_poly.exterior.clone();
    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let eps = 1.0e-9;

    let mut holes = Vec::<LinearRing>::new();
    for h in &source.holes {
        if let Some(env) = h.envelope() {
            let w = env.max_x - env.min_x;
            let hgt = env.max_y - env.min_y;
            if w <= 2.0 * distance || hgt <= 2.0 * distance {
                continue;
            }
        }

        let hr = build_offset_ring(
            &h.coords,
            distance,
            BufferJoinStyle::Mitre,
            segs,
            options.mitre_limit,
            false,
        );
        if hr.len() < 4 {
            continue;
        }

        let hole = LinearRing::new(hr);
        if !is_ring_simple_eps(&hole.coords, eps) {
            continue;
        }
        if ring_abs_area(&hole.coords) <= eps * eps {
            continue;
        }

        let sample = hole.coords[0];
        if !point_in_ring_inclusive_eps(sample, &shell.coords, eps) {
            continue;
        }
        if ring_boundary_intersects_eps(&shell.coords, &hole.coords, eps) {
            continue;
        }
        if holes.iter().any(|kh| {
            ring_boundary_intersects_eps(&kh.coords, &hole.coords, eps)
                || point_in_ring_inclusive_eps(kh.coords[0], &hole.coords, eps)
                || point_in_ring_inclusive_eps(hole.coords[0], &kh.coords, eps)
        }) {
            continue;
        }
        holes.push(hole);
    }

    repair_buffer_polygon(Polygon::new(shell, holes), eps)
}

fn buffer_polygon_legacy_impl(poly: &Polygon, distance: f64, options: BufferOptions) -> Polygon {
    if !distance.is_finite() {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }
    if poly.exterior.coords.len() < 4 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    if distance.abs() <= 1.0e-12 {
        return repair_buffer_polygon(poly.clone(), 1.0e-9);
    }

    if distance > 0.0 {
        return buffer_polygon_positive(poly, distance, options);
    }

    buffer_polygon_negative(poly, -distance, options)
}

fn buffer_polygon_positive(poly: &Polygon, distance: f64, options: BufferOptions) -> Polygon {
    // Robust round-join path: union buffered segments with the source polygon.
    // This avoids offset-ring corner pathologies that can create notch artifacts
    // on complex real-world building footprints.
    if options.join_style == BufferJoinStyle::Round {
        let parts = buffer_polygon_positive_round(poly, distance, options);
        if let Some(selected) = select_round_positive_component(parts, poly, 1.0e-9) {
            let sanitized = sanitize_round_positive_component(selected, poly, 1.0e-9);
            let mut result = enforce_valid_round_positive_output(sanitized, poly, 1.0e-9);
            // Strip any holes whose bbox has collapsed under the inward offset.
            // A hole whose output bbox dimensions are ≤ 2×distance is either a
            // collapsed-source residue or a zero-area artifact; either way it
            // should not appear in the output.  This mirrors the explicit
            // collapse check used by the non-round (Mitre/Flat) path.
            result.holes.retain(|h| {
                if let Some(env) = h.envelope() {
                    env.max_x - env.min_x > 2.0 * distance
                        && env.max_y - env.min_y > 2.0 * distance
                } else {
                    false
                }
            });
            return result;
        }
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let ring = &poly.exterior.coords;
    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let out = build_offset_ring(
        ring,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
        true,
    );
    if out.len() < 4 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    // Repair shell if offsetting produced self-intersection artifacts.
    let shell = {
        let repaired = repair_buffer_polygon(Polygon::new(LinearRing::new(out), vec![]), 1.0e-9);
        repaired.exterior
    };

    // Positive buffer shrinks holes inward; collapsed/invalid holes are dropped.
    let mut holes = Vec::<LinearRing>::new();
    let eps = 1.0e-9;

    for h in &poly.holes {
        // Conservative collapse check: if a hole's bbox span is already less
        // than twice the offset distance in any axis, the inward offset is
        // treated as collapsed.
        if let Some(env) = h.envelope() {
            let w = env.max_x - env.min_x;
            let hgt = env.max_y - env.min_y;
            if w <= 2.0 * distance || hgt <= 2.0 * distance {
                continue;
            }
        }

        // Inward hole offsets are stabilized using mitre joins to avoid
        // self-crossing artifacts from round inward joins.
        let hr = build_offset_ring(
            &h.coords,
            distance,
            BufferJoinStyle::Mitre,
            segs,
            options.mitre_limit,
            false,
        );
        if hr.len() < 4 {
            continue;
        }

        let hole = LinearRing::new(hr);
        if !is_ring_simple_eps(&hole.coords, eps) {
            continue;
        }
        if ring_abs_area(&hole.coords) <= eps * eps {
            continue;
        }

        // Hole must remain inside shell and not cross shell boundary.
        let sample = hole.coords[0];
        if !point_in_ring_inclusive_eps(sample, &shell.coords, eps) {
            continue;
        }
        if ring_boundary_intersects_eps(&shell.coords, &hole.coords, eps) {
            continue;
        }

        // Hole must not overlap/cross existing kept holes.
        if holes.iter().any(|kh| {
            ring_boundary_intersects_eps(&kh.coords, &hole.coords, eps)
                || point_in_ring_inclusive_eps(kh.coords[0], &hole.coords, eps)
                || point_in_ring_inclusive_eps(hole.coords[0], &kh.coords, eps)
        }) {
            continue;
        }

        holes.push(hole);
    }

    repair_buffer_polygon(Polygon::new(shell, holes), eps)
}

fn ring_open_coords(ring: &LinearRing) -> Vec<Coord> {
    let mut open = sanitize_path(&ring.coords);
    if open.first() == open.last() && open.len() > 1 {
        open.pop();
    }
    open
}

fn add_union_piece(parts: &mut Vec<Polygon>, piece: Polygon, eps: f64) {
    let candidates = make_valid_polygon(&piece, eps);
    let mut queue = if candidates.is_empty() {
        vec![piece]
    } else {
        candidates
    };

    for mut current in queue.drain(..) {
        if current.exterior.coords.len() < 4 || ring_abs_area(&current.exterior.coords) <= eps * eps {
            continue;
        }

        let mut i = 0usize;
        while i < parts.len() {
            let part_area = polygon_abs_area(&parts[i]);
            let cur_area = polygon_abs_area(&current);
            let min_expected = part_area.max(cur_area);
            let area_tol = eps.max(1.0e-9) * 10.0;

            let mut accepted_merge: Option<Polygon> = None;

            let merged = polygon_union(&parts[i], &current, eps);
            if merged.len() == 1 {
                let cand = merged[0].clone();
                if polygon_abs_area(&cand) + area_tol >= min_expected {
                    accepted_merge = Some(cand);
                }
            }

            // Rare robustness fallback: the epsilon overlay can occasionally
            // misclassify containment and return a smaller polygon. Retry the
            // same union on progressively coarser fixed grids.
            if accepted_merge.is_none() {
                for scale in [10_000.0, 1_000.0, 100.0] {
                    let merged_prec = polygon_union_with_precision(
                        &parts[i],
                        &current,
                        PrecisionModel::Fixed { scale },
                    );
                    if merged_prec.len() != 1 {
                        continue;
                    }
                    let cand = merged_prec[0].clone();
                    let tol = area_tol.max(1.0 / scale);
                    if polygon_abs_area(&cand) + tol >= min_expected {
                        accepted_merge = Some(cand);
                        break;
                    }
                }
            }

            if let Some(merged_poly) = accepted_merge {
                current = merged_poly;
                parts.swap_remove(i);
            } else {
                i += 1;
            }
        }

        parts.push(current);
    }
}

fn polygon_abs_area(poly: &Polygon) -> f64 {
    let mut area = ring_abs_area(&poly.exterior.coords);
    for h in &poly.holes {
        area -= ring_abs_area(&h.coords);
    }
    area.max(0.0)
}

fn polygon_contains_point_inclusive(poly: &Polygon, p: Coord, eps: f64) -> bool {
    if !point_in_ring_inclusive_eps(p, &poly.exterior.coords, eps) {
        return false;
    }
    !poly
        .holes
        .iter()
        .any(|h| point_in_ring_inclusive_eps(p, &h.coords, eps))
}

fn select_round_positive_component(
    parts: Vec<Polygon>,
    source: &Polygon,
    eps: f64,
) -> Option<Polygon> {
    if parts.is_empty() {
        return None;
    }

    // Pick a stable interior-ish sample from source exterior vertices.
    let mut sample = source
        .exterior
        .coords
        .first()
        .copied()
        .unwrap_or(Coord::xy(0.0, 0.0));
    for &v in &source.exterior.coords {
        if !source
            .holes
            .iter()
            .any(|h| point_in_ring_inclusive_eps(v, &h.coords, eps))
        {
            sample = v;
            break;
        }
    }

    let mut containing = parts
        .iter()
        .filter(|p| polygon_contains_point_inclusive(p, sample, eps))
        .cloned()
        .collect::<Vec<_>>();

    if !containing.is_empty() {
        return containing
            .drain(..)
            .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)));
    }

    parts
        .into_iter()
        .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
}

fn sanitize_round_positive_component(poly: Polygon, source: &Polygon, eps: f64) -> Polygon {
    let sample = source
        .exterior
        .coords
        .first()
        .copied()
        .unwrap_or(Coord::xy(0.0, 0.0));

    for tol in [eps, eps * 10.0, eps * 100.0, eps * 1_000.0] {
        let candidates = make_valid_polygon(&poly, tol);
        if candidates.is_empty() {
            continue;
        }

        let mut containing = candidates
            .iter()
            .filter(|p| polygon_contains_point_inclusive(p, sample, tol))
            .cloned()
            .collect::<Vec<_>>();

        if !containing.is_empty() {
            if let Some(best) = containing
                .drain(..)
                .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
            {
                return best;
            }
        }

        if let Some(best_any) = candidates
            .into_iter()
            .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
        {
            return best_any;
        }
    }

    // Final fallback for stubborn floating-point self-intersections:
    // snap to progressively coarser grids, then re-run make_valid.
    for scale in [10_000.0, 1_000.0, 100.0, 10.0] {
        let snapped = PrecisionModel::Fixed { scale }.apply_polygon(&poly);
        let candidates = make_valid_polygon(&snapped, eps.max(0.5 / scale));
        if candidates.is_empty() {
            continue;
        }

        let mut containing = candidates
            .iter()
            .filter(|p| polygon_contains_point_inclusive(p, sample, eps.max(0.5 / scale)))
            .cloned()
            .collect::<Vec<_>>();

        if let Some(best) = containing
            .drain(..)
            .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
        {
            return best;
        }

        if let Some(best_any) = candidates
            .into_iter()
            .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
        {
            return best_any;
        }
    }

    poly
}

fn choose_best_candidate_for_source(
    candidates: Vec<Polygon>,
    sample: Coord,
    eps: f64,
) -> Option<Polygon> {
    if candidates.is_empty() {
        return None;
    }

    let mut containing = candidates
        .iter()
        .filter(|p| polygon_contains_point_inclusive(p, sample, eps))
        .cloned()
        .collect::<Vec<_>>();

    if !containing.is_empty() {
        return containing
            .drain(..)
            .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)));
    }

    candidates
        .into_iter()
        .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
}

fn enforce_valid_round_positive_output(poly: Polygon, source: &Polygon, eps: f64) -> Polygon {
    if is_valid_polygon(&poly) {
        return poly;
    }

    let sample = source
        .exterior
        .coords
        .first()
        .copied()
        .unwrap_or(Coord::xy(0.0, 0.0));

    for tol in [eps, eps * 10.0, eps * 100.0, eps * 1_000.0, eps * 10_000.0] {
        let candidates = make_valid_polygon(&poly, tol);
        if let Some(best) = choose_best_candidate_for_source(candidates, sample, tol) {
            if is_valid_polygon(&best) {
                return best;
            }
        }
    }

    for scale in [10_000.0, 1_000.0, 100.0, 10.0] {
        let snapped = PrecisionModel::Fixed { scale }.apply_polygon(&poly);
        let tol = eps.max(0.5 / scale);
        let candidates = make_valid_polygon(&snapped, tol);
        if let Some(best) = choose_best_candidate_for_source(candidates, sample, tol) {
            if is_valid_polygon(&best) {
                return best;
            }
        }
    }

    poly
}

fn buffer_polygon_positive_round(poly: &Polygon, distance: f64, options: BufferOptions) -> Vec<Polygon> {
    let eps = 1.0e-9;
    let seg_options = BufferOptions {
        quadrant_segments: options.quadrant_segments.max(2),
        cap_style: BufferCapStyle::Round,
        join_style: BufferJoinStyle::Round,
        mitre_limit: options.mitre_limit,
    };

    let mut parts = Vec::<Polygon>::new();

    // Seed with the original polygon so interiors are preserved while outward
    // expansion is accumulated by segment buffers.
    add_union_piece(&mut parts, poly.clone(), eps);

    for ring in std::iter::once(&poly.exterior).chain(poly.holes.iter()) {
        let open = ring_open_coords(ring);
        let n = open.len();
        if n < 2 {
            continue;
        }

        for i in 0..n {
            let a = open[i];
            let b = open[(i + 1) % n];
            if coord_dist2(a, b) <= eps * eps {
                continue;
            }

            let ls = LineString::new(vec![a, b]);
            let seg_buf = buffer_linestring(&ls, distance, seg_options);
            if seg_buf.exterior.coords.len() >= 4 {
                add_union_piece(&mut parts, seg_buf, eps);
            }
        }
    }

    parts
}

fn buffer_polygon_negative(poly: &Polygon, distance: f64, options: BufferOptions) -> Polygon {
    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let eps = 1.0e-9;

    let out = build_offset_ring(
        &poly.exterior.coords,
        distance,
        options.join_style,
        segs,
        options.mitre_limit,
        false,
    );
    if out.len() < 4 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let shell = {
        let repaired = repair_buffer_polygon(Polygon::new(LinearRing::new(out), vec![]), eps);
        repaired.exterior
    };
    if shell.coords.len() < 4 || ring_abs_area(&shell.coords) <= eps * eps {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    let mut holes = Vec::<LinearRing>::new();
    for h in &poly.holes {
        let hr = build_offset_ring(
            &h.coords,
            distance,
            options.join_style,
            segs,
            options.mitre_limit,
            true,
        );
        if hr.len() < 4 {
            continue;
        }

        let hole = LinearRing::new(hr);
        if !is_ring_simple_eps(&hole.coords, eps) {
            continue;
        }
        if ring_abs_area(&hole.coords) <= eps * eps {
            continue;
        }

        if !point_in_ring_inclusive_eps(hole.coords[0], &shell.coords, eps)
            || ring_boundary_intersects_eps(&shell.coords, &hole.coords, eps)
        {
            return Polygon::new(LinearRing::new(vec![]), vec![]);
        }

        if holes.iter().any(|kh| {
            ring_boundary_intersects_eps(&kh.coords, &hole.coords, eps)
                || point_in_ring_inclusive_eps(kh.coords[0], &hole.coords, eps)
                || point_in_ring_inclusive_eps(hole.coords[0], &kh.coords, eps)
        }) {
            return Polygon::new(LinearRing::new(vec![]), vec![]);
        }

        holes.push(hole);
    }

    repair_buffer_polygon(Polygon::new(shell, holes), eps)
}

/// Buffer a polygon by a signed distance, returning all resulting components.
///
/// This is the recommended function for erosion (negative distance) operations
/// where the shell may split into multiple disconnected pieces. It is the only way
/// to recover all components when erosion is wide enough to separate parts of the
/// polygon. [`buffer_polygon`] returns only the largest component for comparison.
///
/// Implementation notes:
/// - Positive `distance`: delegates to `buffer_polygon_positive` and wraps
///   the result in a single-element `Vec`.
/// - Zero `distance`: returns a repaired copy as a single-element `Vec`.
/// - Negative `distance`: erodes the shell using Mitre offset joins (which correctly
///   self-intersect only when components disconnect), then calls `make_valid_polygon`
///   to resolve self-intersections into separate sub-shells. Expands holes outward and
///   attaches them to the appropriate sub-shells. Returns every disconnected component.
pub fn buffer_polygon_multi(poly: &Polygon, distance: f64, options: BufferOptions) -> Vec<Polygon> {
    if !distance.is_finite() {
        return vec![];
    }
    if poly.exterior.coords.len() < 4 {
        return vec![];
    }

    if distance.abs() <= 1.0e-12 {
        let repaired = repair_buffer_polygon(poly.clone(), 1.0e-9);
        return if repaired.exterior.coords.len() >= 4 {
            vec![repaired]
        } else {
            vec![]
        };
    }

    if distance > 0.0 {
        // For positive distance, always delegate to buffer_polygon_positive so
        // that select_round_positive_component (which picks the main expanded
        // component by checking that it contains a source vertex) is applied
        // regardless of join style.  Returning all union components directly
        // caused tiny artifact fragments from failed merge steps to leak into
        // the output as separate undersized polygons.
        let result = buffer_polygon_positive(poly, distance, options);
        return if result.exterior.coords.len() >= 4 {
            vec![result]
        } else {
            vec![]
        };
    }

    // --- Negative distance: erode shell, expand holes. ---
    let abs_dist = -distance;
    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let eps = 1.0e-9;

    // When computing the eroded shell for multi-component detection we use Mitre
    // joins instead of Round.  Round joins produce arc-based self-intersections
    // at every convex corner of the original ring, flooding `make_valid_polygon`
    // with spurious tiny fragments.  With Mitre joins the offset ring only
    // self-intersects when the erosion genuinely creates disconnected components,
    // which is exactly what we want to detect and surface.
    let out = build_offset_ring(
        &poly.exterior.coords,
        abs_dist,
        BufferJoinStyle::Mitre,
        segs,
        options.mitre_limit,
        false,
    );
    if out.len() < 4 {
        return vec![];
    }

    // Collapse check: every vertex of the eroded shell must sit strictly inside
    // the original polygon.  With Mitre joins, an over-eroded polygon produces
    // a ring whose corner vertices land on or outside the original boundary.
    // Rejecting those early avoids degenerate output from make_valid_polygon.
    let orig_shell_ref = &poly.exterior.coords;
    let eroded_open = if out.first() == out.last() && out.len() > 1 {
        &out[..out.len() - 1]
    } else {
        &out[..]
    };
    if eroded_open.iter().any(|&p| {
        !matches!(classify_point_in_ring_eps(p, orig_shell_ref, eps), PointInRing::Inside)
    }) {
        return vec![];
    }

    // Feed the (possibly self-intersecting) eroded ring through make_valid_polygon
    // so that self-intersections are resolved into distinct sub-shells.
    let eroded_poly = Polygon::new(LinearRing::new(out), vec![]);
    let sub_shells = make_valid_polygon(&eroded_poly, eps);
    if sub_shells.is_empty() {
        return vec![];
    }

    // Compute all expanded holes (holes grow outward during erosion).
    let mut expanded_holes = Vec::<LinearRing>::new();
    for h in &poly.holes {
        let hr = build_offset_ring(
            &h.coords,
            abs_dist,
            BufferJoinStyle::Mitre,
            segs,
            options.mitre_limit,
            true,
        );
        if hr.len() < 4 {
            continue;
        }
        let hole = LinearRing::new(hr);
        if !is_ring_simple_eps(&hole.coords, eps) {
            continue;
        }
        if ring_abs_area(&hole.coords) <= eps * eps {
            continue;
        }
        expanded_holes.push(hole);
    }

    let orig_shell = &poly.exterior.coords;
    let mut result = Vec::<Polygon>::new();

    for sub_poly in sub_shells {
        let shell = &sub_poly.exterior;
        if shell.coords.len() < 4 {
            continue;
        }
        if ring_abs_area(&shell.coords) <= eps * eps {
            continue;
        }

        // Sub-shell must lie inside the original polygon's shell.
        let sample = shell.coords[0];
        if !point_in_ring_inclusive_eps(sample, orig_shell, eps) {
            continue;
        }

        // Assign holes that fall fully inside this sub-shell.
        let mut holes = Vec::<LinearRing>::new();
        for h in &expanded_holes {
            let h_sample = h.coords[0];
            if !point_in_ring_inclusive_eps(h_sample, &shell.coords, eps) {
                continue;
            }
            if ring_boundary_intersects_eps(&shell.coords, &h.coords, eps) {
                continue;
            }
            if holes.iter().any(|kh| {
                ring_boundary_intersects_eps(&kh.coords, &h.coords, eps)
                    || point_in_ring_inclusive_eps(kh.coords[0], &h.coords, eps)
                    || point_in_ring_inclusive_eps(h.coords[0], &kh.coords, eps)
            }) {
                continue;
            }
            holes.push(h.clone());
        }

        let out_poly = repair_buffer_polygon(Polygon::new(shell.clone(), holes), eps);
        if out_poly.exterior.coords.len() >= 4 {
            result.push(out_poly);
        }
    }

    result
}

/// Attempt to repair a polygon under epsilon-based validity checks.
///
/// Current strategy:
/// - normalize and close rings
/// - drop degenerate/non-simple rings
/// - retain only holes that are fully inside exterior and mutually non-overlapping
///
/// Returns zero polygons when the exterior cannot be repaired.
pub fn make_valid_polygon(poly: &Polygon, epsilon: f64) -> Vec<Polygon> {
    let eps = normalized_eps(epsilon);

    let Some(exterior_coords) = sanitize_ring(&poly.exterior.coords, eps) else {
        return vec![];
    };
    if !is_ring_simple_eps(&exterior_coords, eps) {
        let parts = split_all_self_intersections(&exterior_coords, eps, 0);
        return parts
            .into_iter()
            .map(|shell| Polygon::new(LinearRing::new(shell), vec![]))
            .collect();
    }

    let mut kept_holes = Vec::<LinearRing>::new();

    'hole_loop: for hole in &poly.holes {
        let Some(hole_coords) = sanitize_ring(&hole.coords, eps) else {
            continue;
        };
        if !is_ring_simple_eps(&hole_coords, eps) {
            continue;
        }

        let sample = hole_coords[0];
        if !point_in_ring_inclusive_eps(sample, &exterior_coords, eps) {
            continue;
        }
        if ring_boundary_intersects_eps(&exterior_coords, &hole_coords, eps) {
            continue;
        }

        for existing in &kept_holes {
            if ring_boundary_intersects_eps(&existing.coords, &hole_coords, eps) {
                continue 'hole_loop;
            }
            if point_in_ring_inclusive_eps(existing.coords[0], &hole_coords, eps)
                || point_in_ring_inclusive_eps(hole_coords[0], &existing.coords, eps)
            {
                continue 'hole_loop;
            }
        }

        kept_holes.push(LinearRing::new(hole_coords));
    }

    vec![Polygon::new(LinearRing::new(exterior_coords), kept_holes)]
}

/// Attempt to repair any geometry using configurable fixing strategies.
pub fn make_valid_geometry(geom: &Geometry, options: GeometryFixOptions) -> Geometry {
    let eps = options.epsilon.abs().max(1.0e-12);
    match geom {
        Geometry::Polygon(poly) => {
            if options.mode == GeometryFixMode::LineworkFirst {
                let lines = polygon_boundaries_as_lines(poly);
                let res = polygonize_linework(
                    &lines,
                    PolygonizeOptions {
                        epsilon: eps,
                        noding: NodingOptions {
                            epsilon: eps,
                            strategy: NodingStrategy::SnapRounding,
                            precision: Some(PrecisionModel::Fixed {
                                scale: 1.0 / eps.max(1.0e-9),
                            }),
                        },
                    },
                );
                if res.polygons.is_empty() {
                    let repaired = make_valid_polygon(poly, eps);
                    return geometry_from_valid_parts(repaired, options.keep_collapsed);
                }
                return geometry_from_valid_parts(res.polygons, options.keep_collapsed);
            }

            let reduced = options
                .keep_collapsed
                .then_some(poly.clone())
                .and_then(|p| {
                    PrecisionModel::Floating
                        .apply_polygon_topology(&p, TopologyPrecisionOptions::default())
                })
                .unwrap_or_else(|| poly.clone());
            let repaired = make_valid_polygon(&reduced, eps);
            geometry_from_valid_parts(repaired, options.keep_collapsed)
        }
        Geometry::MultiPolygon(polys) => {
            let mut out = Vec::<Polygon>::new();
            for poly in polys {
                let repaired = make_valid_polygon(poly, eps);
                out.extend(repaired);
            }
            geometry_from_valid_parts(out, options.keep_collapsed)
        }
        Geometry::LineString(ls) => {
            if ls.coords.len() < 2 && !options.keep_collapsed {
                return Geometry::LineString(LineString::new(vec![]));
            }
            Geometry::LineString(ls.clone())
        }
        Geometry::MultiLineString(lines) => {
            let filtered: Vec<LineString> = lines
                .iter()
                .filter(|ls| options.keep_collapsed || ls.coords.len() >= 2)
                .cloned()
                .collect();
            Geometry::MultiLineString(filtered)
        }
        Geometry::GeometryCollection(geoms) => Geometry::GeometryCollection(
            geoms
                .iter()
                .map(|g| make_valid_geometry(g, options))
                .collect(),
        ),
        _ => geom.clone(),
    }
}

fn geometry_from_valid_parts(parts: Vec<Polygon>, keep_collapsed: bool) -> Geometry {
    match parts.len() {
        0 => {
            if keep_collapsed {
                Geometry::MultiPolygon(Vec::new())
            } else {
                Geometry::Polygon(Polygon::new(LinearRing::new(vec![]), vec![]))
            }
        }
        1 => Geometry::Polygon(parts[0].clone()),
        _ => Geometry::MultiPolygon(parts),
    }
}

/// Polygonize a set of closed/simple linestring rings.
///
/// Rings that are not closed/simple are ignored. Rings found inside larger rings
/// become holes of their nearest containing shell.
pub fn polygonize_closed_linestrings(lines: &[LineString], epsilon: f64) -> Vec<Polygon> {
    let eps = normalized_eps(epsilon);

    let mut rings = Vec::<Vec<Coord>>::new();
    for ls in lines {
        let Some(ring) = sanitize_ring(&ls.coords, eps) else {
            continue;
        };
        if !is_ring_simple_eps(&ring, eps) {
            continue;
        }
        rings.push(ring);
    }

    rings.sort_by(|a, b| ring_abs_area(b).total_cmp(&ring_abs_area(a)));

    struct Shell {
        shell: Vec<Coord>,
        holes: Vec<Vec<Coord>>,
    }

    let mut shells = Vec::<Shell>::new();

    'ring_loop: for ring in rings {
        let sample = ring[0];

        let mut container_idx: Option<usize> = None;
        let mut best_area = f64::INFINITY;

        for (i, sh) in shells.iter().enumerate() {
            if ring_boundary_intersects_eps(&sh.shell, &ring, eps) {
                continue;
            }
            if !point_in_ring_inclusive_eps(sample, &sh.shell, eps) {
                continue;
            }
            let area = ring_abs_area(&sh.shell);
            if area < best_area {
                best_area = area;
                container_idx = Some(i);
            }
        }

        if let Some(i) = container_idx {
            for h in &shells[i].holes {
                if ring_boundary_intersects_eps(h, &ring, eps)
                    || point_in_ring_inclusive_eps(h[0], &ring, eps)
                    || point_in_ring_inclusive_eps(ring[0], h, eps)
                {
                    continue 'ring_loop;
                }
            }
            shells[i].holes.push(ring);
        } else {
            shells.push(Shell {
                shell: ring,
                holes: vec![],
            });
        }
    }

    shells
        .into_iter()
        .map(|s| {
            Polygon::new(
                LinearRing::new(s.shell),
                s.holes.into_iter().map(LinearRing::new).collect(),
            )
        })
        .collect()
}

/// Polygonize arbitrary linework after noding and graph assembly.
pub fn polygonize_linework(lines: &[LineString], options: PolygonizeOptions) -> PolygonizeResult {
    let eps = options.epsilon.abs().max(1.0e-12);
    let noding = NodingOptions {
        epsilon: eps,
        ..options.noding
    };
    let noded = node_linestrings_with_options(lines, noding);
    let graph = TopologyGraph::from_linestrings_with_options(&noded, noding);
    let all_rings = graph.extract_face_rings(eps);

    let mut rings = Vec::<LineString>::new();
    let mut invalid_rings = Vec::<LineString>::new();
    let area_min = eps * eps;
    for ring in all_rings {
        if ring.coords.len() < 4 {
            invalid_rings.push(ring);
            continue;
        }

        if !is_ring_simple_eps(&ring.coords, eps) || ring_abs_area(&ring.coords) <= area_min {
            invalid_rings.push(ring);
            continue;
        }

        let mut open = ring.coords.clone();
        if open.first() == open.last() && open.len() > 1 {
            open.pop();
        }
        if ring_signed_area_closed(&open) > area_min {
            rings.push(ring);
        }
    }

    let polygons = polygonize_closed_linestrings(&rings, eps);

    let mut used_edges = std::collections::HashSet::<(i64, i64, i64, i64)>::new();
    for ring in &rings {
        for seg in ring.coords.windows(2) {
            let a = seg[0];
            let b = seg[1];
            used_edges.insert(edge_key_quantized(a, b, eps));
        }
    }

    let mut dangles = Vec::<LineString>::new();
    let mut cut_edges = Vec::<LineString>::new();
    for edge in &graph.edges {
        if edge.id > edge.sym {
            continue;
        }
        let a = graph.nodes[edge.from].coord;
        let b = graph.nodes[edge.to].coord;
        let key = edge_key_quantized(a, b, eps);
        if used_edges.contains(&key) {
            continue;
        }

        let ls = LineString::new(vec![a, b]);
        let da = graph.nodes[edge.from].outgoing.len();
        let db = graph.nodes[edge.to].outgoing.len();
        if da <= 1 || db <= 1 {
            dangles.push(ls);
        } else {
            cut_edges.push(ls);
        }
    }

    PolygonizeResult {
        polygons,
        dangles,
        cut_edges,
        invalid_rings,
    }
}

fn edge_key_quantized(a: Coord, b: Coord, eps: f64) -> (i64, i64, i64, i64) {
    let scale = 1.0 / eps.max(1.0e-9);
    let ax = (a.x * scale).round() as i64;
    let ay = (a.y * scale).round() as i64;
    let bx = (b.x * scale).round() as i64;
    let by = (b.y * scale).round() as i64;
    if (ax, ay) <= (bx, by) {
        (ax, ay, bx, by)
    } else {
        (bx, by, ax, ay)
    }
}

fn polygon_boundaries_as_lines(poly: &Polygon) -> Vec<LineString> {
    let mut out = Vec::<LineString>::new();
    out.push(LineString::new(poly.exterior.coords.clone()));
    for hole in &poly.holes {
        out.push(LineString::new(hole.coords.clone()));
    }
    out
}

/// Build the set of raw offset rings to feed into the graph buffer pipeline.
///
/// **Gap J improvement:** Instead of emitting one buffered polygon per ring *segment*
/// (O(N_segments) curves with redundant end-caps), we walk each source ring
/// continuously with `build_offset_ring`, producing a single closed offset curve per
/// ring.  For a polygon with N exterior vertices and M holes this reduces input
/// curve count from O(N + Σ hole_segments) down to O(1 + M), dramatically shrinking
/// the noding and graph-construction work for large polygons.
///
/// The exterior ring is expanded outward (`outward = true`); hole rings are shrunk
/// inward (`outward = false`), consistent with positive-buffer semantics.
fn build_polygon_buffer_curve_set(poly: &Polygon, distance: f64, options: BufferOptions) -> Vec<LineString> {
    let segs = (options.quadrant_segments.max(2) * 4).max(8);
    let d = distance.abs();
    let mut curves = Vec::<LineString>::new();

    // Exterior ring — expand outward.
    let ext_coords = build_offset_ring(
        &poly.exterior.coords,
        d,
        options.join_style,
        segs,
        options.mitre_limit,
        true, // outward
    );
    if ext_coords.len() >= 4 {
        curves.push(LineString::new(ext_coords));
    }

    // Hole rings — the positive buffer shrinks holes, so offset inward.
    for hole in &poly.holes {
        let hole_coords = build_offset_ring(
            &hole.coords,
            d,
            options.join_style,
            segs,
            options.mitre_limit,
            false, // inward (positive buffer shrinks holes)
        );
        if hole_coords.len() >= 4 {
            curves.push(LineString::new(hole_coords));
        }
    }

    // If no continuous curves were produced (e.g., degenerate rings), fall back
    // to the original per-segment approach so the caller can still use its own
    // legacy fallback path.
    if curves.is_empty() {
        for ring in std::iter::once(&poly.exterior).chain(poly.holes.iter()) {
            if ring.coords.len() < 2 {
                continue;
            }
            for seg in ring.coords.windows(2) {
                let ls = LineString::new(vec![seg[0], seg[1]]);
                let buffered = buffer_linestring(&ls, d, options);
                curves.push(LineString::new(buffered.exterior.coords.clone()));
                for h in buffered.holes {
                    curves.push(LineString::new(h.coords));
                }
            }
        }
    }

    curves
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

fn select_buffer_polygons_by_depth(
    candidates: &[Polygon],
    source: &Polygon,
    distance: f64,
    eps: f64,
) -> Vec<Polygon> {
    let source_geom = Geometry::Polygon(source.clone());
    let mut selected = Vec::<(FaceDepthLabel, Polygon)>::new();

    // Filter candidates: exclude degenerate/collapsed faces with near-zero area.
    // These are remnants from hole collapse or noding artifacts and should not compete
    // with the main buffer result.
    let min_face_area = 1.0e-6;

    for poly in candidates {
        let area = ring_abs_area(&poly.exterior.coords);
        if area < min_face_area {
            continue;
        }

        let samples = buffer_face_sample_points(poly);
        if samples.is_empty() {
            continue;
        }

        let mut inside_count = 0usize;
        let mut boundary_count = 0usize;
        let mut min_distance = f64::INFINITY;

        for p in &samples {
            match classify_point_in_polygon_eps(*p, source, eps) {
                PointInRing::Inside => inside_count += 1,
                PointInRing::Boundary => {
                    boundary_count += 1;
                    inside_count += 1;
                }
                PointInRing::Outside => {}
            }

            let d = geometry_distance(&Geometry::Point(*p), &source_geom);
            if d.is_finite() {
                min_distance = min_distance.min(d);
            }
        }

        let near_source = min_distance.is_finite() && min_distance <= distance.abs() + eps;
        if inside_count > 0 || near_source {
            selected.push((
                FaceDepthLabel {
                    inside_count,
                    boundary_count,
                    sample_count: samples.len(),
                    near_source,
                    min_source_distance: min_distance,
                },
                poly.clone(),
            ));
        }
    }

    if selected.is_empty() {
        return candidates.to_vec();
    }

    selected.sort_by(|a, b| {
        a.0
            .cmp(&b.0)
            .then_with(|| {
                ring_abs_area(&b.1.exterior.coords).total_cmp(&ring_abs_area(&a.1.exterior.coords))
            })
            .then_with(|| buffer_poly_sort_key(&a.1).cmp(&buffer_poly_sort_key(&b.1)))
    });
    selected.into_iter().map(|(_, poly)| poly).collect()
}

fn buffer_poly_sort_key(poly: &Polygon) -> (u64, u64, usize, usize) {
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

#[derive(Debug, Clone, Copy)]
struct FaceDepthLabel {
    inside_count: usize,
    boundary_count: usize,
    sample_count: usize,
    near_source: bool,
    min_source_distance: f64,
}

impl PartialEq for FaceDepthLabel {
    fn eq(&self, other: &Self) -> bool {
        self.inside_count == other.inside_count
            && self.boundary_count == other.boundary_count
            && self.sample_count == other.sample_count
            && self.near_source == other.near_source
            && self.min_source_distance == other.min_source_distance
    }
}

impl Eq for FaceDepthLabel {}

impl PartialOrd for FaceDepthLabel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FaceDepthLabel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .inside_count
            .cmp(&self.inside_count)
            .then_with(|| other.boundary_count.cmp(&self.boundary_count))
            .then_with(|| other.sample_count.cmp(&self.sample_count))
            .then_with(|| other.near_source.cmp(&self.near_source))
            .then_with(|| {
                self.min_source_distance
                    .partial_cmp(&other.min_source_distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

fn buffer_face_sample_points(poly: &Polygon) -> Vec<Coord> {
    let mut out = Vec::<Coord>::new();

    if let Some(probe) = buffer_face_probe_point(poly) {
        out.push(probe);
    }

    // Add a few ring vertices to approximate face depth classification.
    let n = poly.exterior.coords.len();
    if n > 1 {
        out.push(poly.exterior.coords[0]);
        out.push(poly.exterior.coords[n / 2]);
        out.push(poly.exterior.coords[(3 * n) / 4]);
    }

    // Add centroid-like average of exterior coordinates.
    if !poly.exterior.coords.is_empty() {
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut c = 0usize;
        for p in &poly.exterior.coords {
            sx += p.x;
            sy += p.y;
            c += 1;
        }
        if c > 0 {
            out.push(Coord::xy(sx / c as f64, sy / c as f64));
        }
    }

    // Deduplicate with small tolerance.
    let mut dedup = Vec::<Coord>::new();
    for p in out {
        if dedup
            .iter()
            .any(|q| (q.x - p.x).abs() <= 1.0e-12 && (q.y - p.y).abs() <= 1.0e-12)
        {
            continue;
        }
        dedup.push(p);
    }
    dedup
}

fn buffer_face_probe_point(poly: &Polygon) -> Option<Coord> {
    if poly.exterior.coords.len() < 2 {
        return poly.exterior.coords.first().copied();
    }

    for seg in poly.exterior.coords.windows(2) {
        let a = seg[0];
        let b = seg[1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 1.0e-12 {
            continue;
        }
        let mx = 0.5 * (a.x + b.x);
        let my = 0.5 * (a.y + b.y);
        let nx = -dy / len;
        let ny = dx / len;
        return Some(Coord::xy(mx + nx * 1.0e-6, my + ny * 1.0e-6));
    }

    poly.exterior.coords.first().copied()
}

/// Precision-aware variant of [`buffer_point`].
pub fn buffer_point_with_precision(
    center: Coord,
    distance: f64,
    options: BufferOptions,
    precision: PrecisionModel,
) -> Polygon {
    let c = precision.apply_coord(center);
    let poly = buffer_point(c, distance, options);
    match precision.apply_geometry(&Geometry::Polygon(poly.clone())) {
        Geometry::Polygon(p) => p,
        _ => poly,
    }
}

/// Precision-aware variant of [`buffer_linestring`].
pub fn buffer_linestring_with_precision(
    ls: &LineString,
    distance: f64,
    options: BufferOptions,
    precision: PrecisionModel,
) -> Polygon {
    let snapped = precision.apply_linestring(ls);
    let poly = buffer_linestring(&snapped, distance, options);
    match precision.apply_geometry(&Geometry::Polygon(poly.clone())) {
        Geometry::Polygon(p) => p,
        _ => poly,
    }
}

/// Precision-aware variant of [`buffer_polygon`].
pub fn buffer_polygon_with_precision(
    poly: &Polygon,
    distance: f64,
    options: BufferOptions,
    precision: PrecisionModel,
) -> Polygon {
    let snapped = precision.apply_polygon(poly);
    let out = buffer_polygon(&snapped, distance, options);
    match precision.apply_geometry(&Geometry::Polygon(out.clone())) {
        Geometry::Polygon(p) => p,
        _ => out,
    }
}

fn sanitize_ring(coords: &[Coord], eps: f64) -> Option<Vec<Coord>> {
    if coords.is_empty() {
        return None;
    }

    let mut out = Vec::<Coord>::with_capacity(coords.len() + 1);

    for &c in coords {
        if let Some(&last) = out.last() {
            if coord_dist2(last, c) <= eps * eps {
                continue;
            }
        }
        out.push(c);
    }

    if out.len() < 4 {
        return None;
    }

    if out.first() != out.last() {
        out.push(out[0]);
    } else {
        let first = out[0];
        let last_idx = out.len() - 1;
        out[last_idx] = first;
    }

    if out.len() < 4 {
        return None;
    }

    Some(out)
}

fn is_ring_simple_eps(coords: &[Coord], eps: f64) -> bool {
    if coords.len() < 4 {
        return false;
    }

    let seg_count = coords.len() - 1;
    for i in 0..seg_count {
        let a1 = coords[i];
        let a2 = coords[i + 1];

        for j in (i + 1)..seg_count {
            if j == i || j == i + 1 {
                continue;
            }
            if i == 0 && j == seg_count - 1 {
                continue;
            }

            let b1 = coords[j];
            let b2 = coords[j + 1];
            if segments_intersect_eps(a1, a2, b1, b2, eps) {
                return false;
            }
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

fn point_in_ring_inclusive_eps(p: Coord, ring: &[Coord], eps: f64) -> bool {
    matches!(
        classify_point_in_ring_eps(p, ring, eps),
        PointInRing::Inside | PointInRing::Boundary
    )
}

fn ring_abs_area(coords: &[Coord]) -> f64 {
    let mut s = 0.0;
    if coords.len() < 2 {
        return 0.0;
    }
    for i in 0..(coords.len() - 1) {
        s += coords[i].x * coords[i + 1].y - coords[i + 1].x * coords[i].y;
    }
    (0.5 * s).abs()
}

fn coord_dist2(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn sanitize_path(coords: &[Coord]) -> Vec<Coord> {
    let mut out = Vec::<Coord>::with_capacity(coords.len());
    for &c in coords {
        if out
            .last()
            .map(|q| coord_dist2(*q, c) <= 1.0e-24)
            .unwrap_or(false)
        {
            continue;
        }
        out.push(c);
    }
    out
}

fn build_offset_side(
    path: &[Coord],
    distance: f64,
    join_style: BufferJoinStyle,
    segs: usize,
    mitre_limit: f64,
) -> Vec<Coord> {
    if path.len() < 2 {
        return vec![];
    }

    let mut dirs = Vec::<(f64, f64)>::with_capacity(path.len() - 1);
    let mut norms = Vec::<(f64, f64)>::with_capacity(path.len() - 1);
    for i in 0..(path.len() - 1) {
        let (ux, uy) = unit_dir(path[i], path[i + 1]);
        dirs.push((ux, uy));
        norms.push((-uy, ux));
    }

    let mut out = Vec::<Coord>::new();
    let (n0x, n0y) = norms[0];
    out.push(Coord::xy(path[0].x + n0x * distance, path[0].y + n0y * distance));

    for i in 1..(path.len() - 1) {
        let v = path[i];
        let (dpx, dpy) = dirs[i - 1];
        let (dcx, dcy) = dirs[i];
        let (npx, npy) = norms[i - 1];
        let (ncx, ncy) = norms[i];

        let p_prev = Coord::xy(v.x + npx * distance, v.y + npy * distance);
        let p_next = Coord::xy(v.x + ncx * distance, v.y + ncy * distance);

        let turn = dpx * dcy - dpy * dcx;
        // Near-collinear consecutive segments are numerically unstable for
        // line-line intersection and can create tiny spikes/notches.
        if turn.abs() <= 1.0e-9 {
            out.push(Coord::xy(
                0.5 * (p_prev.x + p_next.x),
                0.5 * (p_prev.y + p_next.y),
            ));
            continue;
        }
        let outside = turn > 0.0;

        if outside {
            match join_style {
                BufferJoinStyle::Round => {
                    let bis_x = npx + ncx;
                    let bis_y = npy + ncy;
                    let test = if bis_x.abs() + bis_y.abs() <= 1.0e-15 {
                        Coord::xy(v.x + npx * distance, v.y + npy * distance)
                    } else {
                        let len = (bis_x * bis_x + bis_y * bis_y).sqrt();
                        Coord::xy(
                            v.x + (bis_x / len) * distance,
                            v.y + (bis_y / len) * distance,
                        )
                    };
                    let ccw = ccw_arc_contains(v, p_prev, p_next, test);
                    append_arc(
                        &mut out,
                        v,
                        p_prev,
                        p_next,
                        segs / 2,
                        ccw,
                        true,
                    );
                }
                BufferJoinStyle::Bevel => {
                    out.push(p_prev);
                    out.push(p_next);
                }
                BufferJoinStyle::Mitre => {
                    if let Some(m) = line_line_intersection(
                        p_prev,
                        Coord::xy(p_prev.x + dpx, p_prev.y + dpy),
                        p_next,
                        Coord::xy(p_next.x + dcx, p_next.y + dcy),
                        1.0e-12,
                    ) {
                        let max_len = distance * mitre_limit.max(1.0);
                        let max_len2 = max_len * max_len;
                        if coord_dist2(m, v) <= max_len2 {
                            out.push(m);
                        } else {
                            out.push(p_prev);
                            out.push(p_next);
                        }
                    } else {
                        out.push(p_prev);
                        out.push(p_next);
                    }
                }
            }
        } else if let Some(m) = line_line_intersection(
            p_prev,
            Coord::xy(p_prev.x + dpx, p_prev.y + dpy),
            p_next,
            Coord::xy(p_next.x + dcx, p_next.y + dcy),
            1.0e-12,
        ) {
            // Cap very deep inset joins (acute reflex turns) to avoid small,
            // visually distracting notch artifacts in practical datasets.
            let max_len = distance * mitre_limit.max(1.0);
            let max_len2 = max_len * max_len;
            if coord_dist2(m, v) <= max_len2 {
                out.push(m);
            } else {
                out.push(p_next);
            }
        } else {
            out.push(p_next);
        }
    }

    let (nex, ney) = norms[norms.len() - 1];
    out.push(Coord::xy(
        path[path.len() - 1].x + nex * distance,
        path[path.len() - 1].y + ney * distance,
    ));

    out
}

fn build_offset_ring(
    ring: &[Coord],
    distance: f64,
    join_style: BufferJoinStyle,
    segs: usize,
    mitre_limit: f64,
    outward: bool,
) -> Vec<Coord> {
    if ring.len() < 4 {
        return vec![];
    }

    let mut open = sanitize_path(ring);
    if open.first() == open.last() && open.len() >= 2 {
        open.pop();
    }
    if open.len() < 3 {
        return vec![];
    }

    let n = open.len();
    let mut dirs = Vec::<(f64, f64)>::with_capacity(n);
    let mut norms = Vec::<(f64, f64)>::with_capacity(n);
    for i in 0..n {
        let a = open[i];
        let b = open[(i + 1) % n];
        let (ux, uy) = unit_dir(a, b);
        dirs.push((ux, uy));
        norms.push((-uy, ux)); // left normal
    }

    // Positive signed area means CCW exterior; outward is right side.
    let signed = ring_signed_area_closed(&open);
    let mut side_sign = if signed > 0.0 { -1.0 } else { 1.0 };
    if !outward {
        side_sign = -side_sign;
    }

    let mut out = Vec::<Coord>::new();

    for i in 0..n {
        let v = open[i];
        let ip = if i == 0 { n - 1 } else { i - 1 };
        let inext = i;

        let (dpx, dpy) = dirs[ip];
        let (dcx, dcy) = dirs[inext];
        let (nplx, nply) = norms[ip];
        let (nclx, ncly) = norms[inext];

        let np = (nplx * side_sign, nply * side_sign);
        let nc = (nclx * side_sign, ncly * side_sign);

        let p_prev = Coord::xy(v.x + np.0 * distance, v.y + np.1 * distance);
        let p_next = Coord::xy(v.x + nc.0 * distance, v.y + nc.1 * distance);

        let turn = dpx * dcy - dpy * dcx;
        // Near-collinear consecutive segments are numerically unstable for
        // line-line intersection and can create tiny spikes/notches.
        if turn.abs() <= 1.0e-9 {
            out.push(Coord::xy(
                0.5 * (p_prev.x + p_next.x),
                0.5 * (p_prev.y + p_next.y),
            ));
            continue;
        }
        // `outside` is true when the offset lines diverge (convex exterior corner
        // from the perspective of the offset side), meaning a gap must be filled
        // with an arc, bevel or mitre join.  Because the offset direction is
        // encoded in `side_sign`, the sign of `turn * side_sign` must be inverted
        // relative to a fixed-side algorithm: the gap opens when the two normals
        // point *away* from each other, which happens when `turn * side_sign < 0`.
        let outside = turn * side_sign < 0.0;

        if outside {
            match join_style {
                BufferJoinStyle::Round => {
                    let include_start = out.is_empty();
                    let bis_x = np.0 + nc.0;
                    let bis_y = np.1 + nc.1;
                    let test = if bis_x.abs() + bis_y.abs() <= 1.0e-15 {
                        Coord::xy(v.x + np.0 * distance, v.y + np.1 * distance)
                    } else {
                        let len = (bis_x * bis_x + bis_y * bis_y).sqrt();
                        Coord::xy(
                            v.x + (bis_x / len) * distance,
                            v.y + (bis_y / len) * distance,
                        )
                    };
                    let ccw = ccw_arc_contains(v, p_prev, p_next, test);
                    append_arc(
                        &mut out,
                        v,
                        p_prev,
                        p_next,
                        segs / 2,
                        ccw,
                        include_start,
                    );
                }
                BufferJoinStyle::Bevel => {
                    out.push(p_prev);
                    out.push(p_next);
                }
                BufferJoinStyle::Mitre => {
                    if let Some(m) = line_line_intersection(
                        p_prev,
                        Coord::xy(p_prev.x + dpx, p_prev.y + dpy),
                        p_next,
                        Coord::xy(p_next.x + dcx, p_next.y + dcy),
                        1.0e-12,
                    ) {
                        let max_len = distance * mitre_limit.max(1.0);
                        let max_len2 = max_len * max_len;
                        if coord_dist2(m, v) <= max_len2 {
                            out.push(m);
                        } else {
                            out.push(p_prev);
                            out.push(p_next);
                        }
                    } else {
                        out.push(p_prev);
                        out.push(p_next);
                    }
                }
            }
        } else if let Some(m) = line_line_intersection(
            p_prev,
            Coord::xy(p_prev.x + dpx, p_prev.y + dpy),
            p_next,
            Coord::xy(p_next.x + dcx, p_next.y + dcy),
            1.0e-12,
        ) {
            // Cap very deep inset joins (acute reflex turns) to avoid small,
            // visually distracting notch artifacts in practical datasets.
            let max_len = distance * mitre_limit.max(1.0);
            let max_len2 = max_len * max_len;
            if coord_dist2(m, v) <= max_len2 {
                out.push(m);
            } else {
                out.push(p_next);
            }
        } else {
            out.push(p_next);
        }
    }

    if out.len() < 3 {
        return vec![];
    }

    // Deduplicate adjacent repeats, then close.
    let mut cleaned = Vec::<Coord>::with_capacity(out.len() + 1);
    for p in out {
        if cleaned
            .last()
            .map(|q| coord_dist2(*q, p) <= 1.0e-24)
            .unwrap_or(false)
        {
            continue;
        }
        cleaned.push(p);
    }

    if cleaned.len() < 3 {
        return vec![];
    }

    if cleaned.first() != cleaned.last() {
        cleaned.push(cleaned[0]);
    }

    cleaned
}

fn ring_signed_area_closed(open: &[Coord]) -> f64 {
    if open.len() < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..open.len() {
        let j = (i + 1) % open.len();
        s += open[i].x * open[j].y - open[j].x * open[i].y;
    }
    0.5 * s
}

fn repair_buffer_polygon(poly: Polygon, eps: f64) -> Polygon {
    let original = poly.clone();
    if poly.exterior.coords.len() < 4 {
        return Polygon::new(LinearRing::new(vec![]), vec![]);
    }

    if is_ring_simple_eps(&poly.exterior.coords, eps) {
        return poly;
    }

    let repaired = make_valid_polygon(&poly, eps);
    if repaired.is_empty() {
        return original;
    }

    repaired
        .into_iter()
        .max_by(|a, b| ring_abs_area(&a.exterior.coords).total_cmp(&ring_abs_area(&b.exterior.coords)))
        .unwrap_or(original)
}

fn append_cap(
    ring: &mut Vec<Coord>,
    endpoint: Coord,
    dir: (f64, f64),
    distance: f64,
    cap_style: BufferCapStyle,
    segs: usize,
    at_end: bool,
) {
    let (ux, uy) = dir;
    let (nx, ny) = (-uy, ux);

    let left_pt = Coord::xy(endpoint.x + nx * distance, endpoint.y + ny * distance);
    let right_pt = Coord::xy(endpoint.x - nx * distance, endpoint.y - ny * distance);

    match cap_style {
        BufferCapStyle::Flat => {
            if at_end {
                // Bridge from left_end to right_end across the tip.
                ring.push(right_pt);
            }
            // at_end=false: the ring's implicit closure from right_start back to
            // left_start (ring[0]) already forms the flat cap — nothing to push.
        }
        BufferCapStyle::Square => {
            let ext = if at_end {
                Coord::xy(ux * distance, uy * distance)
            } else {
                Coord::xy(-ux * distance, -uy * distance)
            };
            if at_end {
                // End cap: left_end → left_ext → right_ext → right_end
                ring.push(Coord::xy(left_pt.x + ext.x, left_pt.y + ext.y));
                ring.push(Coord::xy(right_pt.x + ext.x, right_pt.y + ext.y));
                ring.push(right_pt);
            } else {
                // Start cap: right_start → right_ext → left_ext → (close to left_start)
                ring.push(Coord::xy(right_pt.x + ext.x, right_pt.y + ext.y));
                ring.push(Coord::xy(left_pt.x + ext.x, left_pt.y + ext.y));
                // implicit ring closure lands on left_pt = ring[0]
            }
        }
        BufferCapStyle::Round => {
            // End cap: arc from left_end → right_end (wraps around the tip).
            // Start cap: arc from right_start → left_start (wraps behind the start).
            // The ring traversal always ends at the "from" point and the arc
            // delivers us to the "to" point; swapping start/end for at_end=false
            // corrects the direction so the cap wraps the correct side.
            let (arc_start, arc_end) = if at_end {
                (left_pt, right_pt)
            } else {
                (right_pt, left_pt)
            };
            let test = if at_end {
                Coord::xy(endpoint.x + ux * distance, endpoint.y + uy * distance)
            } else {
                Coord::xy(endpoint.x - ux * distance, endpoint.y - uy * distance)
            };

            let ccw = ccw_arc_contains(endpoint, arc_start, arc_end, test);
            append_arc(ring, endpoint, arc_start, arc_end, segs / 2, ccw, false);
        }
    }
}

fn append_arc(
    out: &mut Vec<Coord>,
    center: Coord,
    start: Coord,
    end: Coord,
    steps: usize,
    ccw: bool,
    include_start: bool,
) {
    let a0 = (start.y - center.y).atan2(start.x - center.x);
    let a1 = (end.y - center.y).atan2(end.x - center.x);
    let mut delta = if ccw { a1 - a0 } else { a0 - a1 };
    while delta < 0.0 {
        delta += std::f64::consts::TAU;
    }

    let r = ((start.x - center.x).powi(2) + (start.y - center.y).powi(2)).sqrt();
    let n = steps.max(2);
    let start_k = if include_start { 0 } else { 1 };
    for k in start_k..=n {
        let t = k as f64 / n as f64;
        let a = if ccw { a0 + delta * t } else { a0 - delta * t };
        out.push(Coord::xy(center.x + r * a.cos(), center.y + r * a.sin()));
    }
}

fn ccw_arc_contains(center: Coord, start: Coord, end: Coord, test: Coord) -> bool {
    let a0 = (start.y - center.y).atan2(start.x - center.x);
    let a1 = (end.y - center.y).atan2(end.x - center.x);
    let at = (test.y - center.y).atan2(test.x - center.x);
    let d01 = normalize_angle(a1 - a0);
    let d0t = normalize_angle(at - a0);
    d0t <= d01
}

fn normalize_angle(mut a: f64) -> f64 {
    while a < 0.0 {
        a += std::f64::consts::TAU;
    }
    while a >= std::f64::consts::TAU {
        a -= std::f64::consts::TAU;
    }
    a
}

fn line_line_intersection(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> Option<Coord> {
    let r_x = a2.x - a1.x;
    let r_y = a2.y - a1.y;
    let s_x = b2.x - b1.x;
    let s_y = b2.y - b1.y;
    let denom = r_x * s_y - r_y * s_x;
    if denom.abs() <= eps {
        return None;
    }
    let q_p_x = b1.x - a1.x;
    let q_p_y = b1.y - a1.y;
    let t = (q_p_x * s_y - q_p_y * s_x) / denom;
    Some(Coord::xy(a1.x + t * r_x, a1.y + t * r_y))
}

fn unit_dir(a: Coord, b: Coord) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        (1.0, 0.0)
    } else {
        (dx / len, dy / len)
    }
}

fn segment_intersection_point(a1: Coord, a2: Coord, b1: Coord, b2: Coord, eps: f64) -> Option<Coord> {
    let r_x = a2.x - a1.x;
    let r_y = a2.y - a1.y;
    let s_x = b2.x - b1.x;
    let s_y = b2.y - b1.y;
    let denom = r_x * s_y - r_y * s_x;
    if denom.abs() <= eps {
        return None;
    }

    let q_p_x = b1.x - a1.x;
    let q_p_y = b1.y - a1.y;
    let t = (q_p_x * s_y - q_p_y * s_x) / denom;
    let u = (q_p_x * r_y - q_p_y * r_x) / denom;

    if t < -eps || t > 1.0 + eps || u < -eps || u > 1.0 + eps {
        return None;
    }

    Some(Coord::interpolate_segment(a1, a2, t))
}

/// Split a closed ring at its first detected self-intersection and return the
/// two sub-rings.  Returns `None` if no intersection is found or if splitting
/// would produce degenerate rings.
fn split_single_self_intersection(ring: &[Coord], eps: f64) -> Option<Vec<Vec<Coord>>> {
    // A minimal closed bow-tie has 5 coordinates (4 unique + closure).
    if ring.len() < 5 {
        return None;
    }

    let seg_count = ring.len() - 1;
    for i in 0..seg_count {
        let a1 = ring[i];
        let a2 = ring[i + 1];

        for j in (i + 2)..seg_count {
            if i == 0 && j == seg_count - 1 {
                continue;
            }

            let b1 = ring[j];
            let b2 = ring[j + 1];
            if !segments_intersect_eps(a1, a2, b1, b2, eps) {
                continue;
            }

            let x = segment_intersection_point(a1, a2, b1, b2, eps)?;

            let mut p1 = Vec::<Coord>::new();
            p1.push(x);
            p1.extend_from_slice(&ring[i + 1..=j]);
            p1.push(x);

            let mut p2 = Vec::<Coord>::new();
            p2.push(x);
            p2.extend_from_slice(&ring[j + 1..seg_count]);
            p2.extend_from_slice(&ring[0..=i]);
            p2.push(x);

            let s1 = sanitize_ring(&p1, eps)?;
            let s2 = sanitize_ring(&p2, eps)?;
            if s1.len() < 4 || s2.len() < 4 {
                return None;
            }

            return Some(vec![s1, s2]);
        }
    }

    None
}

/// Recursively split a closed ring at every self-intersection, yielding one or
/// more simple rings.  Rings that are still non-simple after splitting (e.g.
/// triple crossings) are discarded rather than returned corrupted.
fn split_all_self_intersections(ring: &[Coord], eps: f64, depth: usize) -> Vec<Vec<Coord>> {
    const MAX_DEPTH: usize = 16;
    if depth > MAX_DEPTH || ring.len() < 5 {
        return if ring.len() >= 4 && is_ring_simple_eps(ring, eps) {
            vec![ring.to_vec()]
        } else {
            vec![]
        };
    }

    if is_ring_simple_eps(ring, eps) {
        return vec![ring.to_vec()];
    }

    match split_single_self_intersection(ring, eps) {
        None => vec![], // can't split → discard
        Some(parts) => parts
            .into_iter()
            .flat_map(|r| split_all_self_intersections(&r, eps, depth + 1))
            .collect(),
    }
}

fn normalized_eps(epsilon: f64) -> f64 {
    if epsilon.is_finite() {
        epsilon.abs().max(1.0e-12)
    } else {
        1.0e-12
    }
}
