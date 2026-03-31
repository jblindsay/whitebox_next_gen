//! Affine transforms for geometry types.

use crate::geom::{Coord, Geometry, LinearRing, LineString, Polygon};

// ── Coordinate transforms ─────────────────────────────────────────────────────

#[inline]
fn translate_coord(c: Coord, dx: f64, dy: f64) -> Coord {
    Coord {
        x: c.x + dx,
        y: c.y + dy,
        z: c.z,
    }
}

#[inline]
fn scale_coord(c: Coord, sx: f64, sy: f64, ox: f64, oy: f64) -> Coord {
    Coord {
        x: ox + (c.x - ox) * sx,
        y: oy + (c.y - oy) * sy,
        z: c.z,
    }
}

#[inline]
fn rotate_coord(c: Coord, sin_a: f64, cos_a: f64, ox: f64, oy: f64) -> Coord {
    let dx = c.x - ox;
    let dy = c.y - oy;
    Coord {
        x: ox + dx * cos_a - dy * sin_a,
        y: oy + dx * sin_a + dy * cos_a,
        z: c.z,
    }
}

// ── LineString / Polygon helpers ─────────────────────────────────────────────

fn transform_ls<F: Fn(Coord) -> Coord>(ls: &LineString, f: &F) -> LineString {
    LineString {
        coords: ls.coords.iter().copied().map(f).collect(),
    }
}

fn transform_ring<F: Fn(Coord) -> Coord>(ring: &LinearRing, f: &F) -> LinearRing {
    LinearRing {
        coords: ring.coords.iter().copied().map(f).collect(),
    }
}

fn transform_poly<F: Fn(Coord) -> Coord>(poly: &Polygon, f: &F) -> Polygon {
    Polygon {
        exterior: transform_ring(&poly.exterior, f),
        holes: poly.holes.iter().map(|h| transform_ring(h, f)).collect(),
    }
}

// ── Recursive geometry transform ──────────────────────────────────────────────

fn transform_geom<F: Fn(Coord) -> Coord>(g: &Geometry, f: &F) -> Geometry {
    match g {
        Geometry::Point(c) => Geometry::Point(f(*c)),
        Geometry::LineString(ls) => Geometry::LineString(transform_ls(ls, f)),
        Geometry::Polygon(poly) => Geometry::Polygon(transform_poly(poly, f)),
        Geometry::MultiPoint(pts) => Geometry::MultiPoint(pts.iter().copied().map(f).collect()),
        Geometry::MultiLineString(lines) => {
            Geometry::MultiLineString(lines.iter().map(|ls| transform_ls(ls, f)).collect())
        }
        Geometry::MultiPolygon(polys) => {
            Geometry::MultiPolygon(polys.iter().map(|p| transform_poly(p, f)).collect())
        }
        Geometry::GeometryCollection(parts) => {
            Geometry::GeometryCollection(parts.iter().map(|p| transform_geom(p, f)).collect())
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Translate all coordinates by `(dx, dy)`.
pub fn translate(g: &Geometry, dx: f64, dy: f64) -> Geometry {
    transform_geom(g, &|c| translate_coord(c, dx, dy))
}

/// Scale all coordinates by `(sx, sy)` around `origin`.
///
/// Pass `origin = None` to scale around the coordinate origin `(0, 0)`.
pub fn scale(g: &Geometry, sx: f64, sy: f64, origin: Option<Coord>) -> Geometry {
    let (ox, oy) = origin.map(|c| (c.x, c.y)).unwrap_or((0.0, 0.0));
    transform_geom(g, &|c| scale_coord(c, sx, sy, ox, oy))
}

/// Rotate all coordinates by `angle_rad` (radians, counter-clockwise) around
/// `origin`.
///
/// Pass `origin = None` to rotate around the coordinate origin `(0, 0)`.
pub fn rotate(g: &Geometry, angle_rad: f64, origin: Option<Coord>) -> Geometry {
    let (ox, oy) = origin.map(|c| (c.x, c.y)).unwrap_or((0.0, 0.0));
    let sin_a = angle_rad.sin();
    let cos_a = angle_rad.cos();
    transform_geom(g, &|c| rotate_coord(c, sin_a, cos_a, ox, oy))
}
