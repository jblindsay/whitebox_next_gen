//! Constructive geometry utilities.
//!
//! This module provides pragmatic building blocks for Phase 2 parity work:
//! - polygon repair (`make_valid_polygon`)
//! - polygonization from closed linestrings (`polygonize_closed_linestrings`)
//! - point buffering (`buffer_point`)

use crate::algorithms::point_in_ring::{classify_point_in_ring_eps, PointInRing};
use crate::algorithms::segment::segments_intersect_eps;
use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};
use crate::overlay::{polygon_union, polygon_union_with_precision};
use crate::precision::PrecisionModel;
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
    repair_buffer_polygon(poly, 1.0e-9)
}

/// Buffer a polygon by the given distance.
///
/// Positive distances expand the shell and shrink holes. Zero distance returns
/// a repaired copy of the input polygon. Negative distances erode the shell and
/// expand holes; when erosion collapses the polygon or would require a
/// multipolygon result, the largest surviving component or an empty polygon is
/// returned.
pub fn buffer_polygon(poly: &Polygon, distance: f64, options: BufferOptions) -> Polygon {
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
            return enforce_valid_round_positive_output(sanitized, poly, 1.0e-9);
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
/// This is the multipolygon variant of [`buffer_polygon`]. While
/// [`buffer_polygon`] returns only the largest surviving component for negative
/// distances that split the shell into disconnected pieces, this function
/// returns every valid component.
///
/// - Positive `distance`: delegates to `buffer_polygon_positive` and wraps
///   the result in a single-element `Vec`.
/// - Zero `distance`: returns a repaired copy as a single-element `Vec`.
/// - Negative `distance`: erodes the shell, expands holes, and recovers each
///   surviving disconnected component. Returns an empty `Vec` when the polygon
///   fully collapses.
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
            ring.push(right_pt);
        }
        BufferCapStyle::Square => {
            let ext = if at_end {
                Coord::xy(ux * distance, uy * distance)
            } else {
                Coord::xy(-ux * distance, -uy * distance)
            };
            ring.push(Coord::xy(left_pt.x + ext.x, left_pt.y + ext.y));
            ring.push(Coord::xy(right_pt.x + ext.x, right_pt.y + ext.y));
            ring.push(right_pt);
        }
        BufferCapStyle::Round => {
            let start = left_pt;
            let end = right_pt;
            let test = if at_end {
                Coord::xy(endpoint.x + ux * distance, endpoint.y + uy * distance)
            } else {
                Coord::xy(endpoint.x - ux * distance, endpoint.y - uy * distance)
            };

            let ccw = ccw_arc_contains(endpoint, start, end, test);
            append_arc(ring, endpoint, start, end, segs / 2, ccw, false);
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
