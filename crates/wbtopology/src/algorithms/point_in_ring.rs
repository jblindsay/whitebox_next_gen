//! Point-in-ring tests.

use crate::algorithms::segment::{point_on_segment, point_on_segment_eps};
use crate::geom::Coord;

/// Result for point-in-ring classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointInRing {
    /// Outside ring.
    Outside,
    /// Strictly inside ring.
    Inside,
    /// On ring boundary.
    Boundary,
}

/// Ray-casting point-in-ring classification.
pub fn classify_point_in_ring(p: Coord, ring: &[Coord]) -> PointInRing {
    if ring.len() < 4 {
        return PointInRing::Outside;
    }

    let mut inside = false;
    for i in 0..(ring.len() - 1) {
        let a = ring[i];
        let b = ring[i + 1];

        if point_on_segment(p, a, b) {
            return PointInRing::Boundary;
        }

        let y_cross = (a.y > p.y) != (b.y > p.y);
        if y_cross {
            let x_int = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
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

/// Ray-casting point-in-ring classification with caller-provided epsilon.
pub fn classify_point_in_ring_eps(p: Coord, ring: &[Coord], eps: f64) -> PointInRing {
    if ring.len() < 4 {
        return PointInRing::Outside;
    }

    let mut inside = false;
    for i in 0..(ring.len() - 1) {
        let a = ring[i];
        let b = ring[i + 1];

        if point_on_segment_eps(p, a, b, eps) {
            return PointInRing::Boundary;
        }

        let y_cross = (a.y > p.y + eps) != (b.y > p.y + eps);
        if y_cross {
            let x_int = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if x_int > p.x - eps {
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
