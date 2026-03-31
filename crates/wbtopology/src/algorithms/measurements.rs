//! Area, length, and centroid measurements for any geometry type.

use crate::geom::{Coord, Geometry, LineString, Polygon};

// ── Ring helpers ──────────────────────────────────────────────────────────────

/// Signed area of a closed ring using the shoelace formula.
///
/// Positive for counter-clockwise orientation (right-hand rule), negative for
/// clockwise.  The ring does not need to repeat its first vertex at the end.
pub fn ring_signed_area(coords: &[Coord]) -> f64 {
    let n = coords.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += coords[i].x * coords[j].y;
        sum -= coords[j].x * coords[i].y;
    }
    sum * 0.5
}

/// Absolute (unsigned) area of a closed ring.
#[inline]
pub fn ring_area(coords: &[Coord]) -> f64 {
    ring_signed_area(coords).abs()
}

// ── Polygon helpers ───────────────────────────────────────────────────────────

/// Area of a polygon (exterior minus holes).
pub fn polygon_area(poly: &Polygon) -> f64 {
    let mut area = ring_area(&poly.exterior.coords);
    for hole in &poly.holes {
        area -= ring_area(&hole.coords);
    }
    area.max(0.0)
}

/// Centroid of a closed ring using the standard weighted-centroid formula.
///
/// Returns `None` if the ring is degenerate (area ≈ 0).
pub fn ring_centroid(coords: &[Coord]) -> Option<Coord> {
    let n = coords.len();
    if n < 3 {
        return None;
    }
    let area2 = ring_signed_area(coords) * 2.0;
    if area2.abs() < f64::EPSILON {
        return None;
    }
    let mut cx = 0.0f64;
    let mut cy = 0.0f64;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = coords[i].x * coords[j].y - coords[j].x * coords[i].y;
        cx += (coords[i].x + coords[j].x) * cross;
        cy += (coords[i].y + coords[j].y) * cross;
    }
    Some(Coord::xy(cx / (3.0 * area2), cy / (3.0 * area2)))
}

/// Centroid of a polygon (exterior ring only; holes shift the result slightly
/// but are kept simple here — sufficient for a polygon without large holes).
pub fn polygon_centroid(poly: &Polygon) -> Option<Coord> {
    ring_centroid(&poly.exterior.coords)
}

// ── LineString helpers ────────────────────────────────────────────────────────

/// Length of a linestring.
pub fn linestring_length(ls: &LineString) -> f64 {
    let mut len = 0.0;
    for i in 0..(ls.coords.len().saturating_sub(1)) {
        let dx = ls.coords[i + 1].x - ls.coords[i].x;
        let dy = ls.coords[i + 1].y - ls.coords[i].y;
        len += (dx * dx + dy * dy).sqrt();
    }
    len
}

// ── Public Geometry API ───────────────────────────────────────────────────────

/// Area of a geometry.
///
/// Only `Polygon` and `MultiPolygon` have non-zero area.  All other
/// variants (including `GeometryCollection`) return 0.
pub fn geometry_area(g: &Geometry) -> f64 {
    match g {
        Geometry::Polygon(poly) => polygon_area(poly),
        Geometry::MultiPolygon(polys) => polys.iter().map(polygon_area).sum(),
        Geometry::GeometryCollection(parts) => parts.iter().map(geometry_area).sum(),
        _ => 0.0,
    }
}

/// Length of a geometry.
///
/// `LineString` and `MultiLineString` have non-zero length.
/// `Polygon` perimeter is *not* returned (use the boundary explicitly).
/// `GeometryCollection` sums lengths of all members.
pub fn geometry_length(g: &Geometry) -> f64 {
    match g {
        Geometry::LineString(ls) => linestring_length(ls),
        Geometry::MultiLineString(lines) => lines.iter().map(linestring_length).sum(),
        Geometry::GeometryCollection(parts) => parts.iter().map(geometry_length).sum(),
        _ => 0.0,
    }
}

/// Centroid of a geometry.
///
/// For multi-geometries the centroid is the area-weighted (or count-weighted
/// for 0-dimensional types) average of component centroids.
///
/// Returns `None` for empty or degenerate geometries.
pub fn geometry_centroid(g: &Geometry) -> Option<Coord> {
    match g {
        Geometry::Point(p) => Some(*p),
        Geometry::LineString(ls) => {
            // Midpoint of a linestring (parameterized at L/2)
            let total = linestring_length(ls);
            if total == 0.0 {
                return ls.coords.first().copied();
            }
            let half = total / 2.0;
            let mut accum = 0.0;
            for i in 0..(ls.coords.len().saturating_sub(1)) {
                let dx = ls.coords[i + 1].x - ls.coords[i].x;
                let dy = ls.coords[i + 1].y - ls.coords[i].y;
                let seg = (dx * dx + dy * dy).sqrt();
                if accum + seg >= half {
                    let t = (half - accum) / seg;
                    return Some(Coord::interpolate_segment(ls.coords[i], ls.coords[i + 1], t));
                }
                accum += seg;
            }
            ls.coords.last().copied()
        }
        Geometry::Polygon(poly) => polygon_centroid(poly),
        Geometry::MultiPoint(pts) => {
            if pts.is_empty() {
                return None;
            }
            let n = pts.len() as f64;
            let sx: f64 = pts.iter().map(|c| c.x).sum();
            let sy: f64 = pts.iter().map(|c| c.y).sum();
            Some(Coord::xy(sx / n, sy / n))
        }
        Geometry::MultiLineString(lines) => {
            let mut wx = 0.0f64;
            let mut wy = 0.0f64;
            let mut total_w = 0.0f64;
            for ls in lines {
                let w = linestring_length(ls);
                if let Some(c) = geometry_centroid(&Geometry::LineString(ls.clone())) {
                    wx += c.x * w;
                    wy += c.y * w;
                    total_w += w;
                }
            }
            if total_w == 0.0 {
                None
            } else {
                Some(Coord::xy(wx / total_w, wy / total_w))
            }
        }
        Geometry::MultiPolygon(polys) => {
            let mut wx = 0.0f64;
            let mut wy = 0.0f64;
            let mut total_w = 0.0f64;
            for poly in polys {
                let w = polygon_area(poly);
                if let Some(c) = polygon_centroid(poly) {
                    wx += c.x * w;
                    wy += c.y * w;
                    total_w += w;
                }
            }
            if total_w == 0.0 {
                None
            } else {
                Some(Coord::xy(wx / total_w, wy / total_w))
            }
        }
        Geometry::GeometryCollection(parts) => {
            // Weight by area first, then length, then count
            let mut wx = 0.0f64;
            let mut wy = 0.0f64;
            let mut total_w = 0.0f64;
            for p in parts {
                let w = {
                    let a = geometry_area(p);
                    if a > 0.0 {
                        a
                    } else {
                        let l = geometry_length(p);
                        if l > 0.0 { l } else { 1.0 }
                    }
                };
                if let Some(c) = geometry_centroid(p) {
                    wx += c.x * w;
                    wy += c.y * w;
                    total_w += w;
                }
            }
            if total_w == 0.0 {
                None
            } else {
                Some(Coord::xy(wx / total_w, wy / total_w))
            }
        }
    }
}
