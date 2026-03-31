//! Precision model utilities.
//!
//! Inspired by JTS precision concepts:
//! - floating precision (no snapping)
//! - fixed precision (grid snapping with configurable scale)

use crate::geom::{Coord, Geometry, LineString, LinearRing, Polygon};

/// Precision model controlling coordinate snapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrecisionModel {
    /// No snapping; coordinates are used as-is.
    Floating,
    /// Fixed grid precision where values are rounded to multiples of `1/scale`.
    Fixed {
        /// Number of grid units per coordinate unit.
        scale: f64,
    },
}

impl PrecisionModel {
    /// Apply model to a scalar value.
    #[inline]
    pub fn apply_scalar(self, v: f64) -> f64 {
        match self {
            Self::Floating => v,
            Self::Fixed { scale } => {
                if scale <= 0.0 || !scale.is_finite() {
                    v
                } else {
                    (v * scale).round() / scale
                }
            }
        }
    }

    /// Apply model to a coordinate.
    #[inline]
    pub fn apply_coord(self, c: Coord) -> Coord {
        Coord {
            x: self.apply_scalar(c.x),
            y: self.apply_scalar(c.y),
            z: c.z,
        }
    }

    /// Apply model to all coordinates in-place.
    pub fn apply_coords_in_place(self, coords: &mut [Coord]) {
        for c in coords {
            *c = self.apply_coord(*c);
        }
    }

    /// Apply model to a linestring.
    pub fn apply_linestring(self, ls: &LineString) -> LineString {
        let mut coords = ls.coords.clone();
        self.apply_coords_in_place(&mut coords);
        LineString::new(coords)
    }

    /// Apply model to a ring.
    pub fn apply_ring(self, ring: &LinearRing) -> LinearRing {
        let mut coords = ring.coords.clone();
        self.apply_coords_in_place(&mut coords);
        LinearRing::new(coords)
    }

    /// Apply model to a polygon.
    pub fn apply_polygon(self, poly: &Polygon) -> Polygon {
        let exterior = self.apply_ring(&poly.exterior);
        let holes = poly.holes.iter().map(|h| self.apply_ring(h)).collect();
        Polygon::new(exterior, holes)
    }

    /// Apply model to a geometry.
    pub fn apply_geometry(self, geom: &Geometry) -> Geometry {
        match geom {
            Geometry::Point(c) => Geometry::Point(self.apply_coord(*c)),
            Geometry::LineString(ls) => Geometry::LineString(self.apply_linestring(ls)),
            Geometry::Polygon(poly) => Geometry::Polygon(self.apply_polygon(poly)),
            Geometry::MultiPoint(pts) => {
                Geometry::MultiPoint(pts.iter().map(|&c| self.apply_coord(c)).collect())
            }
            Geometry::MultiLineString(lss) => {
                Geometry::MultiLineString(lss.iter().map(|ls| self.apply_linestring(ls)).collect())
            }
            Geometry::MultiPolygon(polys) => {
                Geometry::MultiPolygon(polys.iter().map(|p| self.apply_polygon(p)).collect())
            }
            Geometry::GeometryCollection(geoms) => {
                Geometry::GeometryCollection(geoms.iter().map(|g| self.apply_geometry(g)).collect())
            }
        }
    }

    /// Epsilon implied by this model.
    #[inline]
    pub fn epsilon(self) -> f64 {
        match self {
            Self::Floating => 1.0e-12,
            Self::Fixed { scale } => {
                if scale <= 0.0 || !scale.is_finite() {
                    1.0e-12
                } else {
                    0.5 / scale
                }
            }
        }
    }

    /// Compare two scalars under this precision model.
    #[inline]
    pub fn eq(self, a: f64, b: f64) -> bool {
        (self.apply_scalar(a) - self.apply_scalar(b)).abs() <= self.epsilon()
    }

    /// Compare two coordinates under this precision model.
    #[inline]
    pub fn eq_coord(self, a: Coord, b: Coord) -> bool {
        self.eq(a.x, b.x) && self.eq(a.y, b.y)
    }
}
