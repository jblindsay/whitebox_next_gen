//! DE-9IM style relation matrix support.
//!
//! This module provides a compact relation matrix and a `relate` function.
//! The current implementation focuses on Point/LineString/Polygon and is
//! intended as an extensible production scaffold.

use crate::geom::Geometry;
use crate::precision::PrecisionModel;
use crate::topology::{
    contains, contains_with_epsilon, crosses, crosses_with_epsilon, intersects,
    intersects_with_epsilon, overlaps, overlaps_with_epsilon, touches,
    touches_with_epsilon, within, within_with_epsilon,
};

/// Topological location in DE-9IM matrix axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// Interior location.
    Interior = 0,
    /// Boundary location.
    Boundary = 1,
    /// Exterior location.
    Exterior = 2,
}

/// 3x3 DE-9IM-like relation matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelateMatrix {
    cells: [[char; 3]; 3],
}

impl RelateMatrix {
    /// Build an empty matrix initialized to `F`.
    pub fn empty() -> Self {
        Self {
            cells: [['F'; 3]; 3],
        }
    }

    /// Set one matrix cell.
    pub fn set(&mut self, a: Location, b: Location, v: char) {
        self.cells[a as usize][b as usize] = v;
    }

    /// Read one matrix cell.
    pub fn get(&self, a: Location, b: Location) -> char {
        self.cells[a as usize][b as usize]
    }

    /// Export the 9-character row-major matrix string.
    pub fn as_str9(&self) -> String {
        let mut s = String::with_capacity(9);
        for r in 0..3 {
            for c in 0..3 {
                s.push(self.cells[r][c]);
            }
        }
        s
    }

    /// Check matrix against a DE-9IM pattern string.
    ///
    /// Pattern rules:
    /// - `F` matches only `F`
    /// - `T` matches any non-`F` value (`0`, `1`, `2`)
    /// - `0`, `1`, `2` match exact dimensions
    /// - `*` matches any value
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern.len() != 9 {
            return false;
        }

        for (actual, expected) in self.as_str9().chars().zip(pattern.chars()) {
            let ok = match expected {
                '*' => true,
                'T' => actual != 'F',
                'F' | '0' | '1' | '2' => actual == expected,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// True when interiors are disjoint.
    #[inline]
    pub fn is_disjoint(&self) -> bool {
        self.matches("FF*FF****")
    }

    /// True when there is any intersection.
    #[inline]
    pub fn is_intersects(&self) -> bool {
        !self.is_disjoint()
    }

    /// True when interiors and boundaries intersect only on boundaries.
    #[inline]
    pub fn is_touches(&self) -> bool {
        self.matches("FT*******") || self.matches("F**T*****") || self.matches("F***T****")
    }

    /// True when `A` is within `B`.
    #[inline]
    pub fn is_within(&self) -> bool {
        self.matches("T*F**F***")
    }

    /// True when `A` contains `B`.
    #[inline]
    pub fn is_contains(&self) -> bool {
        self.matches("T*****FF*")
    }
}

/// Compute a DE-9IM-like relation matrix between two geometries.
///
/// Notes:
/// - `II` (interior/interior) is dimension-aware for Point/LineString/Polygon.
/// - Boundary/exterior cells are populated conservatively and are intended as
///   a stable API scaffold for future full DE-9IM expansion.
pub fn relate(a: &Geometry, b: &Geometry) -> RelateMatrix {
    relate_impl(a, b, None)
}

/// Precision-aware relation matrix between two geometries.
pub fn relate_with_precision(a: &Geometry, b: &Geometry, precision: PrecisionModel) -> RelateMatrix {
    let sa = precision.apply_geometry(a);
    let sb = precision.apply_geometry(b);
    relate_with_epsilon(&sa, &sb, precision.epsilon())
}

/// Epsilon-aware relation matrix between two geometries.
pub fn relate_with_epsilon(a: &Geometry, b: &Geometry, epsilon: f64) -> RelateMatrix {
    relate_impl(a, b, Some(epsilon))
}

fn relate_impl(a: &Geometry, b: &Geometry, epsilon: Option<f64>) -> RelateMatrix {
    let mut m = RelateMatrix::empty();

    // Exterior intersections are generally non-empty for finite geometries.
    m.set(Location::Exterior, Location::Exterior, '2');

    // Boundary to exterior intersections exist for non-point geometries.
    m.set(Location::Boundary, Location::Exterior, boundary_dim_char(a));
    m.set(Location::Exterior, Location::Boundary, boundary_dim_char(b));

    let intersects_v = match epsilon {
        Some(eps) => intersects_with_epsilon(a, b, eps),
        None => intersects(a, b),
    };
    let touches_v = match epsilon {
        Some(eps) => touches_with_epsilon(a, b, eps),
        None => touches(a, b),
    };
    let crosses_v = match epsilon {
        Some(eps) => crosses_with_epsilon(a, b, eps),
        None => crosses(a, b),
    };
    let overlaps_v = match epsilon {
        Some(eps) => overlaps_with_epsilon(a, b, eps),
        None => overlaps(a, b),
    };
    let within_ab = match epsilon {
        Some(eps) => within_with_epsilon(a, b, eps),
        None => within(a, b),
    };
    let within_ba = match epsilon {
        Some(eps) => within_with_epsilon(b, a, eps),
        None => within(b, a),
    };
    let contains_ab = match epsilon {
        Some(eps) => contains_with_epsilon(a, b, eps),
        None => contains(a, b),
    };
    let contains_ba = match epsilon {
        Some(eps) => contains_with_epsilon(b, a, eps),
        None => contains(b, a),
    };

    if !intersects_v {
        // Disjoint: interiors do not intersect.
        m.set(Location::Interior, Location::Interior, 'F');
        m.set(Location::Interior, Location::Exterior, dim_char(a));
        m.set(Location::Exterior, Location::Interior, dim_char(b));
        return m;
    }

    m.set(
        Location::Interior,
        Location::Exterior,
        if within_ab { 'F' } else { dim_char(a) },
    );
    m.set(
        Location::Exterior,
        Location::Interior,
        if within_ba { 'F' } else { dim_char(b) },
    );

    if touches_v {
        m.set(Location::Interior, Location::Interior, 'F');
    } else if crosses_v {
        // Crosses interior-interior dimension is pair-dependent.
        m.set(Location::Interior, Location::Interior, crosses_ii_dim_char(a, b));
    } else {
        m.set(Location::Interior, Location::Interior, ii_dim_char(a, b));
    }

    if touches_v {
        m.set(Location::Boundary, Location::Boundary, '0');
    }

    if crosses_v {
        m.set(Location::Boundary, Location::Boundary, '0');
    }

    if overlaps_v {
        let d = dim_char(a).min(dim_char(b));
        m.set(Location::Interior, Location::Interior, d);
    }

    if contains_ab {
        m.set(Location::Exterior, Location::Interior, 'F');
    }

    if contains_ba {
        m.set(Location::Interior, Location::Exterior, 'F');
    }

    if intersects_v {
        apply_pair_specific_contact_cells(
            &mut m,
            a,
            b,
            touches_v,
            crosses_v,
            overlaps_v,
            within_ab,
            within_ba,
            contains_ab,
            contains_ba,
        );
    }

    m
}

fn dim_char(g: &Geometry) -> char {
    match g {
        Geometry::Point(_) => '0',
        Geometry::LineString(_) => '1',
        Geometry::Polygon(_) => '2',
        Geometry::MultiPolygon(_) => '2',
        Geometry::MultiLineString(_) => '1',
        _ => '0',
    }
}

fn boundary_dim_char(g: &Geometry) -> char {
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => 'F',
        Geometry::LineString(_) | Geometry::MultiLineString(_) => '0',
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => '1',
        Geometry::GeometryCollection(_) => 'F',
    }
}

fn ii_dim_char(a: &Geometry, b: &Geometry) -> char {
    let da = dim_char(a);
    let db = dim_char(b);
    da.min(db)
}

fn crosses_ii_dim_char(a: &Geometry, b: &Geometry) -> char {
    match (a, b) {
        (Geometry::LineString(_), Geometry::LineString(_)) => '0',
        (Geometry::LineString(_), Geometry::Polygon(_)) | (Geometry::Polygon(_), Geometry::LineString(_)) => '1',
        _ => ii_dim_char(a, b),
    }
}

fn apply_pair_specific_contact_cells(
    m: &mut RelateMatrix,
    a: &Geometry,
    b: &Geometry,
    touches_v: bool,
    crosses_v: bool,
    overlaps_v: bool,
    within_ab: bool,
    within_ba: bool,
    contains_ab: bool,
    contains_ba: bool,
) {
    match (a, b) {
        (Geometry::Point(_), Geometry::LineString(_)) | (Geometry::Point(_), Geometry::Polygon(_)) => {
            if touches_v {
                m.set(Location::Interior, Location::Boundary, '0');
            }
        }
        (Geometry::LineString(_), Geometry::Point(_)) | (Geometry::Polygon(_), Geometry::Point(_)) => {
            if touches_v {
                m.set(Location::Boundary, Location::Interior, '0');
            }
        }
        (Geometry::LineString(_), Geometry::LineString(_)) => {
            if touches_v || crosses_v || overlaps_v {
                m.set(Location::Interior, Location::Boundary, '0');
                m.set(Location::Boundary, Location::Interior, '0');
            }
        }
        (Geometry::LineString(_), Geometry::Polygon(_)) => {
            if touches_v || crosses_v {
                m.set(Location::Interior, Location::Boundary, '0');
            }
            if touches_v || crosses_v || within_ab {
                m.set(Location::Boundary, Location::Interior, '0');
            }
        }
        (Geometry::Polygon(_), Geometry::LineString(_)) => {
            if touches_v || crosses_v {
                m.set(Location::Boundary, Location::Interior, '0');
            }
            if touches_v || crosses_v || within_ba {
                m.set(Location::Interior, Location::Boundary, '0');
            }
        }
        (Geometry::Polygon(_), Geometry::Polygon(_)) => {
            if touches_v || overlaps_v || contains_ab || contains_ba {
                m.set(Location::Interior, Location::Boundary, '1');
                m.set(Location::Boundary, Location::Interior, '1');
            }
            if touches_v {
                m.set(Location::Boundary, Location::Boundary, '1');
            }
        }
        _ => {}
    }
}
