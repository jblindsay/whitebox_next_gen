//! Topological predicates and validation helpers.

use crate::algorithms::point_in_ring::{
    classify_point_in_ring,
    classify_point_in_ring_eps,
    PointInRing,
};
use crate::algorithms::segment::{
    point_on_segment,
    point_on_segment_eps,
    segments_intersect,
    segments_intersect_eps,
};
use crate::geom::{Coord, Envelope, Geometry, LineString, Polygon};
use crate::precision::PrecisionModel;

#[derive(Debug, Clone)]
struct PreparedEdge {
    a: Coord,
    b: Coord,
    min_y: f64,
    max_y: f64,
}

#[derive(Debug, Clone)]
struct PreparedRing {
    edges: Vec<PreparedEdge>,
    min_y: f64,
    max_y: f64,
    inv_span_y: f64,
    bins: Vec<Vec<usize>>,
}

impl PreparedRing {
    fn from_coords(coords: &[Coord]) -> Self {
        let mut edges = Vec::with_capacity(coords.len().saturating_sub(1));
        if coords.len() >= 2 {
            for i in 0..(coords.len() - 1) {
                let a = coords[i];
                let b = coords[i + 1];
                edges.push(PreparedEdge {
                    a,
                    b,
                    min_y: a.y.min(b.y),
                    max_y: a.y.max(b.y),
                });
            }
        }

        if edges.is_empty() {
            return Self {
                edges,
                min_y: 0.0,
                max_y: 0.0,
                inv_span_y: 0.0,
                bins: vec![],
            };
        }

        let mut min_y = edges[0].min_y;
        let mut max_y = edges[0].max_y;
        for e in &edges[1..] {
            if e.min_y < min_y {
                min_y = e.min_y;
            }
            if e.max_y > max_y {
                max_y = e.max_y;
            }
        }

        const PREPARED_RING_BINS: usize = 64;
        let span = (max_y - min_y).max(1.0e-12);
        let inv_span_y = 1.0 / span;
        let mut bins = vec![Vec::<usize>::new(); PREPARED_RING_BINS];

        for (idx, e) in edges.iter().enumerate() {
            let fy0 = ((e.min_y - min_y) * inv_span_y).clamp(0.0, 1.0);
            let fy1 = ((e.max_y - min_y) * inv_span_y).clamp(0.0, 1.0);
            let b0 = ((fy0 * (PREPARED_RING_BINS as f64 - 1.0)).floor() as usize)
                .min(PREPARED_RING_BINS - 1);
            let b1 = ((fy1 * (PREPARED_RING_BINS as f64 - 1.0)).floor() as usize)
                .min(PREPARED_RING_BINS - 1);
            for b in b0.min(b1)..=b0.max(b1) {
                bins[b].push(idx);
            }
        }

        Self {
            edges,
            min_y,
            max_y,
            inv_span_y,
            bins,
        }
    }

    fn classify_point(&self, p: Coord) -> PointInRing {
        let mut inside = false;

        let use_bins = !self.bins.is_empty() && p.y >= self.min_y && p.y <= self.max_y;
        if use_bins {
            let fy = ((p.y - self.min_y) * self.inv_span_y).clamp(0.0, 1.0);
            let b = ((fy * (self.bins.len() as f64 - 1.0)).floor() as usize).min(self.bins.len() - 1);
            for &edge_idx in &self.bins[b] {
                let e = &self.edges[edge_idx];
                if point_on_segment(p, e.a, e.b) {
                    return PointInRing::Boundary;
                }

                if p.y <= e.min_y || p.y > e.max_y {
                    continue;
                }
                let x_int = (e.b.x - e.a.x) * (p.y - e.a.y) / (e.b.y - e.a.y) + e.a.x;
                if x_int > p.x {
                    inside = !inside;
                }
            }
        } else {
            for e in &self.edges {
                if point_on_segment(p, e.a, e.b) {
                    return PointInRing::Boundary;
                }

                if p.y <= e.min_y || p.y > e.max_y {
                    continue;
                }
                let x_int = (e.b.x - e.a.x) * (p.y - e.a.y) / (e.b.y - e.a.y) + e.a.x;
                if x_int > p.x {
                    inside = !inside;
                }
            }
        }

        if inside {
            PointInRing::Inside
        } else {
            PointInRing::Outside
        }
    }
}

/// Prepared polygon index for repeated point-in-polygon queries.
///
/// This caches envelope and ring edge metadata to reduce per-query work.
#[derive(Debug, Clone)]
pub struct PreparedPolygon {
    polygon: Polygon,
    envelope: Option<Envelope>,
    exterior: PreparedRing,
    holes: Vec<PreparedRing>,
}

impl PreparedPolygon {
    /// Build a prepared polygon from a polygon.
    pub fn new(polygon: Polygon) -> Self {
        let envelope = polygon.envelope();
        let exterior = PreparedRing::from_coords(&polygon.exterior.coords);
        let holes = polygon
            .holes
            .iter()
            .map(|h| PreparedRing::from_coords(&h.coords))
            .collect();
        Self {
            polygon,
            envelope,
            exterior,
            holes,
        }
    }

    /// Get the source polygon reference.
    #[inline]
    pub fn polygon(&self) -> &Polygon {
        &self.polygon
    }

    /// Fast boundary-inclusive containment check for points.
    pub fn contains_coord(&self, p: Coord) -> bool {
        if let Some(env) = self.envelope {
            if !env.contains_coord(p) {
                return false;
            }
        }

        match self.exterior.classify_point(p) {
            PointInRing::Outside => return false,
            PointInRing::Boundary => return true,
            PointInRing::Inside => {}
        }

        for hole in &self.holes {
            match hole.classify_point(p) {
                PointInRing::Inside => return false,
                PointInRing::Boundary => return true,
                PointInRing::Outside => {}
            }
        }

        true
    }

    /// Fast intersection check for points.
    #[inline]
    pub fn intersects_coord(&self, p: Coord) -> bool {
        self.contains_coord(p)
    }

    /// Fast boundary-inclusive containment query for a geometry.
    pub fn contains_geometry(&self, g: &Geometry) -> bool {
        if let (Some(a), Some(b)) = (self.envelope, geometry_envelope(g)) {
            if !a.intersects(&b) {
                return false;
            }
        }

        match g {
            Geometry::Point(p) => self.contains_coord(*p),
            _ => contains(&Geometry::Polygon(self.polygon.clone()), g),
        }
    }

    /// Fast intersection query for a geometry.
    pub fn intersects_geometry(&self, g: &Geometry) -> bool {
        if let (Some(a), Some(b)) = (self.envelope, geometry_envelope(g)) {
            if !a.intersects(&b) {
                return false;
            }
        }

        match g {
            Geometry::Point(p) => self.intersects_coord(*p),
            _ => intersects(&Geometry::Polygon(self.polygon.clone()), g),
        }
    }
}

fn geometry_envelope(g: &Geometry) -> Option<Envelope> {
    g.envelope()
}

/// True if geometry `a` intersects geometry `b`.
pub fn intersects(a: &Geometry, b: &Geometry) -> bool {
    match (a, b) {
        (Geometry::Point(pa), Geometry::Point(pb)) => pa.xy_eq(pb),
        (Geometry::Point(p), Geometry::LineString(ls)) | (Geometry::LineString(ls), Geometry::Point(p)) => {
            point_on_linestring(*p, ls)
        }
        (Geometry::Point(p), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::Point(p)) => {
            point_intersects_polygon(*p, poly)
        }
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => linestrings_intersect(a_ls, b_ls),
        (Geometry::LineString(ls), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            linestring_intersects_polygon(ls, poly)
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => polygons_intersect(a_poly, b_poly),
        _ => {
            let a_parts = a.components();
            let b_parts = b.components();
            a_parts
                .iter()
                .any(|ap| b_parts.iter().any(|bp| intersects(ap, bp)))
        }
    }
}

/// Precision-aware variant of [`intersects`].
pub fn intersects_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    intersects_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware variant of [`intersects`] without coordinate snapping.
pub fn intersects_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    match (a, b) {
        (Geometry::Point(pa), Geometry::Point(pb)) => eq_coord_eps(*pa, *pb, eps),
        (Geometry::Point(p), Geometry::LineString(ls)) | (Geometry::LineString(ls), Geometry::Point(p)) => {
            point_on_linestring_eps(*p, ls, eps)
        }
        (Geometry::Point(p), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::Point(p)) => {
            point_intersects_polygon_eps(*p, poly, eps)
        }
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            linestrings_intersect_eps(a_ls, b_ls, eps)
        }
        (Geometry::LineString(ls), Geometry::Polygon(poly))
        | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            linestring_intersects_polygon_eps(ls, poly, eps)
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => polygons_intersect_eps(a_poly, b_poly, eps),
        _ => {
            let a_parts = a.components();
            let b_parts = b.components();
            a_parts
                .iter()
                .any(|ap| b_parts.iter().any(|bp| intersects_with_epsilon(ap, bp, eps)))
        }
    }
}

/// True if geometry `container` contains geometry `item` (boundary-inclusive).
pub fn contains(container: &Geometry, item: &Geometry) -> bool {
    match (container, item) {
        (Geometry::Point(a), Geometry::Point(b)) => a.xy_eq(b),
        (Geometry::LineString(ls), Geometry::Point(p)) => point_on_linestring(*p, ls),
        (Geometry::Polygon(poly), Geometry::Point(p)) => point_in_polygon_inclusive(*p, poly),
        (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            ls.coords.iter().all(|p| point_in_polygon_inclusive(*p, poly))
                && !linestring_crosses_polygon_boundary(ls, poly)
        }
        (Geometry::Polygon(a), Geometry::Polygon(b)) => polygon_contains_polygon(a, b),
        // Multi-container: any component contains item.
        (Geometry::MultiPoint(_), _)
        | (Geometry::MultiLineString(_), _)
        | (Geometry::MultiPolygon(_), _)
        | (Geometry::GeometryCollection(_), _) => {
            container.components().iter().any(|cp| contains(cp, item))
        }
        // Multi-item (non-empty): all components must be contained.
        (_, Geometry::MultiPoint(_))
        | (_, Geometry::MultiLineString(_))
        | (_, Geometry::MultiPolygon(_))
        | (_, Geometry::GeometryCollection(_)) => {
            let parts = item.components();
            !parts.is_empty() && parts.iter().all(|ip| contains(container, ip))
        }
        _ => false,
    }
}

/// Precision-aware variant of [`contains`].
pub fn contains_with_precision(
    container: &Geometry,
    item: &Geometry,
    precision: PrecisionModel,
) -> bool {
    let s_container = precision.apply_geometry(container);
    let s_item = precision.apply_geometry(item);
    contains_with_epsilon(&s_container, &s_item, precision.epsilon())
}

/// Epsilon-aware variant of [`contains`] without coordinate snapping.
pub fn contains_with_epsilon(container: &Geometry, item: &Geometry, eps: f64) -> bool {
    match (container, item) {
        (Geometry::Point(a), Geometry::Point(b)) => eq_coord_eps(*a, *b, eps),
        (Geometry::LineString(ls), Geometry::Point(p)) => point_on_linestring_eps(*p, ls, eps),
        (Geometry::Polygon(poly), Geometry::Point(p)) => point_in_polygon_inclusive_eps(*p, poly, eps),
        (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            ls.coords
                .iter()
                .all(|p| point_in_polygon_inclusive_eps(*p, poly, eps))
                && !linestring_crosses_polygon_boundary_eps(ls, poly, eps)
        }
        (Geometry::Polygon(a), Geometry::Polygon(b)) => polygon_contains_polygon_eps(a, b, eps),
        (Geometry::MultiPoint(_), _)
        | (Geometry::MultiLineString(_), _)
        | (Geometry::MultiPolygon(_), _)
        | (Geometry::GeometryCollection(_), _) => {
            container
                .components()
                .iter()
                .any(|cp| contains_with_epsilon(cp, item, eps))
        }
        (_, Geometry::MultiPoint(_))
        | (_, Geometry::MultiLineString(_))
        | (_, Geometry::MultiPolygon(_))
        | (_, Geometry::GeometryCollection(_)) => {
            let parts = item.components();
            !parts.is_empty() && parts.iter().all(|ip| contains_with_epsilon(container, ip, eps))
        }
        _ => false,
    }
}

/// True if geometry `a` is within geometry `b`.
#[inline]
pub fn within(a: &Geometry, b: &Geometry) -> bool {
    contains(b, a)
}

/// Precision-aware variant of [`within`].
pub fn within_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    within_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware variant of [`within`] without coordinate snapping.
#[inline]
pub fn within_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    contains_with_epsilon(b, a, eps)
}

/// True if geometries touch at boundary points without interior overlap.
pub fn touches(a: &Geometry, b: &Geometry) -> bool {
    if !intersects(a, b) {
        return false;
    }

    match (a, b) {
        (Geometry::Point(_), Geometry::Point(_)) => false,
        (Geometry::Point(p), Geometry::LineString(ls)) | (Geometry::LineString(ls), Geometry::Point(p)) => {
            if ls.coords.len() < 2 {
                return false;
            }
            p.xy_eq(&ls.coords[0]) || p.xy_eq(&ls.coords[ls.coords.len() - 1])
        }
        (Geometry::Point(p), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::Point(p)) => {
            point_on_polygon_boundary(*p, poly)
        }
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            linestrings_intersect(a_ls, b_ls)
                && !linestrings_cross_proper(a_ls, b_ls)
                && !contains(
                    &Geometry::LineString(a_ls.clone()),
                    &Geometry::LineString(b_ls.clone()),
                )
                && !contains(
                    &Geometry::LineString(b_ls.clone()),
                    &Geometry::LineString(a_ls.clone()),
                )
        }
        (Geometry::LineString(ls), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            linestring_intersects_polygon(ls, poly)
                && !ls.coords.iter().any(|p| point_in_polygon_strict(*p, poly))
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => {
            ring_boundary_intersects(&a_poly.exterior.coords, &b_poly.exterior.coords)
                && !point_in_polygon_strict(a_poly.exterior.coords[0], b_poly)
                && !point_in_polygon_strict(b_poly.exterior.coords[0], a_poly)
        }
        _ => false,
    }
}

/// Precision-aware variant of [`touches`].
pub fn touches_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    touches_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware variant of [`touches`] without coordinate snapping.
pub fn touches_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    if !intersects_with_epsilon(a, b, eps) {
        return false;
    }

    match (a, b) {
        (Geometry::Point(_), Geometry::Point(_)) => false,
        (Geometry::Point(p), Geometry::LineString(ls))
        | (Geometry::LineString(ls), Geometry::Point(p)) => {
            if ls.coords.len() < 2 {
                return false;
            }
            eq_coord_eps(*p, ls.coords[0], eps)
                || eq_coord_eps(*p, ls.coords[ls.coords.len() - 1], eps)
        }
        (Geometry::Point(p), Geometry::Polygon(poly))
        | (Geometry::Polygon(poly), Geometry::Point(p)) => point_on_polygon_boundary_eps(*p, poly, eps),
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            linestrings_intersect_eps(a_ls, b_ls, eps)
                && !linestrings_cross_proper_eps(a_ls, b_ls, eps)
                && !contains_with_epsilon(
                    &Geometry::LineString(a_ls.clone()),
                    &Geometry::LineString(b_ls.clone()),
                    eps,
                )
                && !contains_with_epsilon(
                    &Geometry::LineString(b_ls.clone()),
                    &Geometry::LineString(a_ls.clone()),
                    eps,
                )
        }
        (Geometry::LineString(ls), Geometry::Polygon(poly))
        | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            linestring_intersects_polygon_eps(ls, poly, eps)
                && !ls
                    .coords
                    .iter()
                    .any(|p| point_in_polygon_strict_eps(*p, poly, eps))
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => {
            ring_boundary_intersects_eps(&a_poly.exterior.coords, &b_poly.exterior.coords, eps)
                && !point_in_polygon_strict_eps(a_poly.exterior.coords[0], b_poly, eps)
                && !point_in_polygon_strict_eps(b_poly.exterior.coords[0], a_poly, eps)
        }
        _ => false,
    }
}

/// True if geometries cross (dimension-specific crossing relation).
pub fn crosses(a: &Geometry, b: &Geometry) -> bool {
    match (a, b) {
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => linestrings_cross_proper(a_ls, b_ls),
        (Geometry::LineString(ls), Geometry::Polygon(poly)) | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            let has_inside = ls.coords.iter().any(|p| point_in_polygon_strict(*p, poly));
            let has_outside = ls.coords.iter().any(|p| !point_in_polygon_inclusive(*p, poly));
            has_inside && has_outside && linestring_crosses_polygon_boundary(ls, poly)
        }
        _ => false,
    }
}

/// Precision-aware variant of [`crosses`].
pub fn crosses_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    crosses_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware variant of [`crosses`] without coordinate snapping.
pub fn crosses_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    match (a, b) {
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            linestrings_cross_proper_eps(a_ls, b_ls, eps)
        }
        (Geometry::LineString(ls), Geometry::Polygon(poly))
        | (Geometry::Polygon(poly), Geometry::LineString(ls)) => {
            let has_inside = ls
                .coords
                .iter()
                .any(|p| point_in_polygon_strict_eps(*p, poly, eps));
            let has_outside = ls
                .coords
                .iter()
                .any(|p| !point_in_polygon_inclusive_eps(*p, poly, eps));
            has_inside && has_outside && linestring_crosses_polygon_boundary_eps(ls, poly, eps)
        }
        _ => false,
    }
}

/// True when `a` covers `b` (contains `b`, including boundary).
///
/// `covers` is like `contains` but returns true even when `b`'s points lie
/// only on `a`'s boundary (JTS semantics).  For primitive-primitive pairs the
/// existing `contains` implementation is already boundary-inclusive, so this
/// delegates directly to it.
pub fn covers(a: &Geometry, b: &Geometry) -> bool {
    contains(a, b)
}

/// Precision-aware variant of [`covers`].
pub fn covers_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    contains_with_precision(a, b, precision)
}

/// Epsilon-aware variant of [`covers`].
pub fn covers_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    contains_with_epsilon(a, b, eps)
}

/// True when `a` is covered by `b` (symmetric of [`covers`]).
#[inline]
pub fn covered_by(a: &Geometry, b: &Geometry) -> bool {
    covers(b, a)
}

/// Precision-aware variant of [`covered_by`].
pub fn covered_by_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    covers_with_precision(b, a, precision)
}

/// Epsilon-aware variant of [`covered_by`].
pub fn covered_by_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    covers_with_epsilon(b, a, eps)
}

/// True when `a` and `b` are disjoint (share no points).
#[inline]
pub fn disjoint(a: &Geometry, b: &Geometry) -> bool {
    !intersects(a, b)
}

/// Precision-aware variant of [`disjoint`].
pub fn disjoint_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    !intersects_with_precision(a, b, precision)
}

/// Epsilon-aware variant of [`disjoint`].
pub fn disjoint_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    !intersects_with_epsilon(a, b, eps)
}

/// True if geometries overlap (same dimension, intersecting interiors, neither contains the other).
pub fn overlaps(a: &Geometry, b: &Geometry) -> bool {
    if !intersects(a, b) {
        return false;
    }

    match (a, b) {
        (Geometry::Point(_), Geometry::Point(_)) => false,
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            let ga = Geometry::LineString(a_ls.clone());
            let gb = Geometry::LineString(b_ls.clone());
            !contains(&ga, &gb) && !contains(&gb, &ga) && !touches(&ga, &gb)
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => {
            let ga = Geometry::Polygon(a_poly.clone());
            let gb = Geometry::Polygon(b_poly.clone());
            !contains(&ga, &gb) && !contains(&gb, &ga) && !touches(&ga, &gb)
        }
        _ => false,
    }
}

/// Precision-aware variant of [`overlaps`].
pub fn overlaps_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> bool {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    overlaps_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware variant of [`overlaps`] without coordinate snapping.
pub fn overlaps_with_epsilon(a: &Geometry, b: &Geometry, eps: f64) -> bool {
    if !intersects_with_epsilon(a, b, eps) {
        return false;
    }

    match (a, b) {
        (Geometry::Point(_), Geometry::Point(_)) => false,
        (Geometry::LineString(a_ls), Geometry::LineString(b_ls)) => {
            let ga = Geometry::LineString(a_ls.clone());
            let gb = Geometry::LineString(b_ls.clone());
            !contains_with_epsilon(&ga, &gb, eps)
                && !contains_with_epsilon(&gb, &ga, eps)
                && !touches_with_epsilon(&ga, &gb, eps)
        }
        (Geometry::Polygon(a_poly), Geometry::Polygon(b_poly)) => {
            let ga = Geometry::Polygon(a_poly.clone());
            let gb = Geometry::Polygon(b_poly.clone());
            !contains_with_epsilon(&ga, &gb, eps)
                && !contains_with_epsilon(&gb, &ga, eps)
                && !touches_with_epsilon(&ga, &gb, eps)
        }
        _ => false,
    }
}

/// True when linestring has no self-intersections (except adjacent segment shared endpoints).
pub fn is_simple_linestring(ls: &LineString) -> bool {
    if ls.coords.len() < 2 {
        return false;
    }
    if ls.coords.len() == 2 {
        return !ls.coords[0].xy_eq(&ls.coords[1]);
    }

    let seg_count = ls.coords.len() - 1;
    for i in 0..seg_count {
        let a1 = ls.coords[i];
        let a2 = ls.coords[i + 1];

        for j in (i + 1)..seg_count {
            if j == i || j == i + 1 {
                continue;
            }

            // Allow first and last segment touching only for explicitly closed rings.
            if i == 0
                && j == seg_count - 1
                && ls.coords
                    .first()
                    .zip(ls.coords.last())
                    .map(|(a, b)| a.xy_eq(b))
                    .unwrap_or(false)
            {
                continue;
            }

            let b1 = ls.coords[j];
            let b2 = ls.coords[j + 1];
            if segments_intersect(a1, a2, b1, b2) {
                return false;
            }
        }
    }
    true
}

/// True when polygon is valid under core OGC-like checks.
///
/// Checks:
/// - exterior ring closed and at least 4 coordinates
/// - hole rings closed and at least 4 coordinates
/// - exterior and each hole are simple
/// - holes lie inside exterior and outside each other
/// - no ring boundary crossings between exterior and holes or hole-hole pairs
pub fn is_valid_polygon(poly: &Polygon) -> bool {
    if !is_ring_valid(&poly.exterior.coords) {
        return false;
    }
    if !is_ring_simple(&poly.exterior.coords) {
        return false;
    }

    for hole in &poly.holes {
        if !is_ring_valid(&hole.coords) || !is_ring_simple(&hole.coords) {
            return false;
        }

        let sample = hole.coords[0];
        if !point_in_ring_inclusive(sample, &poly.exterior.coords) {
            return false;
        }

        if ring_boundary_intersects(&poly.exterior.coords, &hole.coords) {
            return false;
        }
    }

    for i in 0..poly.holes.len() {
        for j in (i + 1)..poly.holes.len() {
            let a = &poly.holes[i].coords;
            let b = &poly.holes[j].coords;
            if ring_boundary_intersects(a, b) {
                return false;
            }
            if point_in_ring_inclusive(a[0], b) || point_in_ring_inclusive(b[0], a) {
                return false;
            }
        }
    }

    true
}

#[inline]
fn is_ring_valid(coords: &[Coord]) -> bool {
    coords.len() >= 4
        && coords
            .first()
            .zip(coords.last())
            .map(|(a, b)| a.xy_eq(b))
            .unwrap_or(false)
}

fn is_ring_simple(coords: &[Coord]) -> bool {
    let ls = LineString {
        coords: coords.to_vec(),
    };
    is_simple_linestring(&ls)
}

fn point_in_ring_inclusive(p: Coord, ring: &[Coord]) -> bool {
    matches!(
        classify_point_in_ring(p, ring),
        PointInRing::Inside | PointInRing::Boundary
    )
}

fn point_on_linestring(p: Coord, ls: &LineString) -> bool {
    if ls.coords.len() < 2 {
        return false;
    }
    for i in 0..(ls.coords.len() - 1) {
        if point_on_segment(p, ls.coords[i], ls.coords[i + 1]) {
            return true;
        }
    }
    false
}

fn point_intersects_polygon(p: Coord, poly: &Polygon) -> bool {
    point_in_polygon_inclusive(p, poly)
}

fn point_in_polygon_inclusive(p: Coord, poly: &Polygon) -> bool {
    match classify_point_in_ring(p, &poly.exterior.coords) {
        PointInRing::Outside => return false,
        PointInRing::Boundary => return true,
        PointInRing::Inside => {}
    }

    for hole in &poly.holes {
        match classify_point_in_ring(p, &hole.coords) {
            PointInRing::Inside => return false,
            PointInRing::Boundary => return true,
            PointInRing::Outside => {}
        }
    }

    true
}

fn point_in_polygon_inclusive_eps(p: Coord, poly: &Polygon, eps: f64) -> bool {
    match classify_point_in_ring_eps(p, &poly.exterior.coords, eps) {
        PointInRing::Outside => return false,
        PointInRing::Boundary => return true,
        PointInRing::Inside => {}
    }

    for hole in &poly.holes {
        match classify_point_in_ring_eps(p, &hole.coords, eps) {
            PointInRing::Inside => return false,
            PointInRing::Boundary => return true,
            PointInRing::Outside => {}
        }
    }

    true
}

fn point_in_polygon_strict(p: Coord, poly: &Polygon) -> bool {
    match classify_point_in_ring(p, &poly.exterior.coords) {
        PointInRing::Outside | PointInRing::Boundary => return false,
        PointInRing::Inside => {}
    }

    for hole in &poly.holes {
        match classify_point_in_ring(p, &hole.coords) {
            PointInRing::Inside | PointInRing::Boundary => return false,
            PointInRing::Outside => {}
        }
    }

    true
}

fn point_in_polygon_strict_eps(p: Coord, poly: &Polygon, eps: f64) -> bool {
    match classify_point_in_ring_eps(p, &poly.exterior.coords, eps) {
        PointInRing::Outside | PointInRing::Boundary => return false,
        PointInRing::Inside => {}
    }

    for hole in &poly.holes {
        match classify_point_in_ring_eps(p, &hole.coords, eps) {
            PointInRing::Inside | PointInRing::Boundary => return false,
            PointInRing::Outside => {}
        }
    }

    true
}

fn point_on_polygon_boundary(p: Coord, poly: &Polygon) -> bool {
    for i in 0..(poly.exterior.coords.len().saturating_sub(1)) {
        if point_on_segment(p, poly.exterior.coords[i], poly.exterior.coords[i + 1]) {
            return true;
        }
    }
    for h in &poly.holes {
        for i in 0..(h.coords.len().saturating_sub(1)) {
            if point_on_segment(p, h.coords[i], h.coords[i + 1]) {
                return true;
            }
        }
    }
    false
}

fn point_on_polygon_boundary_eps(p: Coord, poly: &Polygon, eps: f64) -> bool {
    for i in 0..(poly.exterior.coords.len().saturating_sub(1)) {
        if point_on_segment_eps(p, poly.exterior.coords[i], poly.exterior.coords[i + 1], eps) {
            return true;
        }
    }
    for h in &poly.holes {
        for i in 0..(h.coords.len().saturating_sub(1)) {
            if point_on_segment_eps(p, h.coords[i], h.coords[i + 1], eps) {
                return true;
            }
        }
    }
    false
}

fn linestrings_intersect(a: &LineString, b: &LineString) -> bool {
    if a.coords.len() < 2 || b.coords.len() < 2 {
        return false;
    }

    for i in 0..(a.coords.len() - 1) {
        let a1 = a.coords[i];
        let a2 = a.coords[i + 1];
        for j in 0..(b.coords.len() - 1) {
            let b1 = b.coords[j];
            let b2 = b.coords[j + 1];
            if segments_intersect(a1, a2, b1, b2) {
                return true;
            }
        }
    }
    false
}

fn linestrings_intersect_eps(a: &LineString, b: &LineString, eps: f64) -> bool {
    if a.coords.len() < 2 || b.coords.len() < 2 {
        return false;
    }

    for i in 0..(a.coords.len() - 1) {
        let a1 = a.coords[i];
        let a2 = a.coords[i + 1];
        for j in 0..(b.coords.len() - 1) {
            let b1 = b.coords[j];
            let b2 = b.coords[j + 1];
            if segments_intersect_eps(a1, a2, b1, b2, eps) {
                return true;
            }
        }
    }
    false
}

fn linestrings_cross_proper(a: &LineString, b: &LineString) -> bool {
    if a.coords.len() < 2 || b.coords.len() < 2 {
        return false;
    }

    for i in 0..(a.coords.len() - 1) {
        let a1 = a.coords[i];
        let a2 = a.coords[i + 1];
        for j in 0..(b.coords.len() - 1) {
            let b1 = b.coords[j];
            let b2 = b.coords[j + 1];
            if !segments_intersect(a1, a2, b1, b2) {
                continue;
            }
            if point_on_segment(a1, b1, b2)
                || point_on_segment(a2, b1, b2)
                || point_on_segment(b1, a1, a2)
                || point_on_segment(b2, a1, a2)
            {
                continue;
            }
            return true;
        }
    }
    false
}

fn linestrings_cross_proper_eps(a: &LineString, b: &LineString, eps: f64) -> bool {
    if a.coords.len() < 2 || b.coords.len() < 2 {
        return false;
    }

    for i in 0..(a.coords.len() - 1) {
        let a1 = a.coords[i];
        let a2 = a.coords[i + 1];
        for j in 0..(b.coords.len() - 1) {
            let b1 = b.coords[j];
            let b2 = b.coords[j + 1];
            if !segments_intersect_eps(a1, a2, b1, b2, eps) {
                continue;
            }
            if point_on_segment_eps(a1, b1, b2, eps)
                || point_on_segment_eps(a2, b1, b2, eps)
                || point_on_segment_eps(b1, a1, a2, eps)
                || point_on_segment_eps(b2, a1, a2, eps)
            {
                continue;
            }
            return true;
        }
    }
    false
}

fn linestring_intersects_polygon(ls: &LineString, poly: &Polygon) -> bool {
    if ls.coords.iter().any(|p| point_in_polygon_inclusive(*p, poly)) {
        return true;
    }
    ring_linestring_intersects(&poly.exterior.coords, ls)
        || poly
            .holes
            .iter()
            .any(|h| ring_linestring_intersects(&h.coords, ls))
}

fn linestring_intersects_polygon_eps(ls: &LineString, poly: &Polygon, eps: f64) -> bool {
    if ls
        .coords
        .iter()
        .any(|p| point_in_polygon_inclusive_eps(*p, poly, eps))
    {
        return true;
    }
    ring_linestring_intersects_eps(&poly.exterior.coords, ls, eps)
        || poly
            .holes
            .iter()
            .any(|h| ring_linestring_intersects_eps(&h.coords, ls, eps))
}

fn linestring_crosses_polygon_boundary(ls: &LineString, poly: &Polygon) -> bool {
    ring_linestring_intersects(&poly.exterior.coords, ls)
        || poly
            .holes
            .iter()
            .any(|h| ring_linestring_intersects(&h.coords, ls))
}

fn linestring_crosses_polygon_boundary_eps(ls: &LineString, poly: &Polygon, eps: f64) -> bool {
    ring_linestring_intersects_eps(&poly.exterior.coords, ls, eps)
        || poly
            .holes
            .iter()
            .any(|h| ring_linestring_intersects_eps(&h.coords, ls, eps))
}

fn ring_linestring_intersects(ring: &[Coord], ls: &LineString) -> bool {
    if ring.len() < 2 || ls.coords.len() < 2 {
        return false;
    }
    for i in 0..(ring.len() - 1) {
        let r1 = ring[i];
        let r2 = ring[i + 1];
        for j in 0..(ls.coords.len() - 1) {
            let l1 = ls.coords[j];
            let l2 = ls.coords[j + 1];
            if segments_intersect(r1, r2, l1, l2) {
                return true;
            }
        }
    }
    false
}

fn ring_linestring_intersects_eps(ring: &[Coord], ls: &LineString, eps: f64) -> bool {
    if ring.len() < 2 || ls.coords.len() < 2 {
        return false;
    }
    for i in 0..(ring.len() - 1) {
        let r1 = ring[i];
        let r2 = ring[i + 1];
        for j in 0..(ls.coords.len() - 1) {
            let l1 = ls.coords[j];
            let l2 = ls.coords[j + 1];
            if segments_intersect_eps(r1, r2, l1, l2, eps) {
                return true;
            }
        }
    }
    false
}

fn polygons_intersect(a: &Polygon, b: &Polygon) -> bool {
    if ring_boundary_intersects(&a.exterior.coords, &b.exterior.coords) {
        return true;
    }
    if point_in_polygon_inclusive(a.exterior.coords[0], b)
        || point_in_polygon_inclusive(b.exterior.coords[0], a)
    {
        return true;
    }
    false
}

fn polygons_intersect_eps(a: &Polygon, b: &Polygon, eps: f64) -> bool {
    if ring_boundary_intersects_eps(&a.exterior.coords, &b.exterior.coords, eps) {
        return true;
    }
    if point_in_polygon_inclusive_eps(a.exterior.coords[0], b, eps)
        || point_in_polygon_inclusive_eps(b.exterior.coords[0], a, eps)
    {
        return true;
    }
    false
}

fn polygon_contains_polygon(a: &Polygon, b: &Polygon) -> bool {
    if ring_boundary_intersects(&a.exterior.coords, &b.exterior.coords) {
        return false;
    }

    b.exterior
        .coords
        .iter()
        .all(|p| point_in_polygon_inclusive(*p, a))
}

fn polygon_contains_polygon_eps(a: &Polygon, b: &Polygon, eps: f64) -> bool {
    if ring_boundary_intersects_eps(&a.exterior.coords, &b.exterior.coords, eps) {
        return false;
    }

    b.exterior
        .coords
        .iter()
        .all(|p| point_in_polygon_inclusive_eps(*p, a, eps))
}

fn ring_boundary_intersects(a: &[Coord], b: &[Coord]) -> bool {
    if a.len() < 2 || b.len() < 2 {
        return false;
    }

    for i in 0..(a.len() - 1) {
        let a1 = a[i];
        let a2 = a[i + 1];
        for j in 0..(b.len() - 1) {
            let b1 = b[j];
            let b2 = b[j + 1];
            if segments_intersect(a1, a2, b1, b2) {
                return true;
            }
        }
    }
    false
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

#[inline]
fn point_on_linestring_eps(p: Coord, ls: &LineString, eps: f64) -> bool {
    if ls.coords.len() < 2 {
        return false;
    }
    for i in 0..(ls.coords.len() - 1) {
        if point_on_segment_eps(p, ls.coords[i], ls.coords[i + 1], eps) {
            return true;
        }
    }
    false
}

#[inline]
fn point_intersects_polygon_eps(p: Coord, poly: &Polygon, eps: f64) -> bool {
    point_in_polygon_inclusive_eps(p, poly, eps)
}

#[inline]
fn eq_coord_eps(a: Coord, b: Coord, eps: f64) -> bool {
    (a.x - b.x).abs() <= eps.abs() && (a.y - b.y).abs() <= eps.abs()
}
