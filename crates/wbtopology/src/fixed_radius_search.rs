//! Fixed-radius point search utilities.
//!
//! This module provides a lightweight 2D fixed-radius search index backed by
//! a hash-grid. It is optimized for high-volume local neighbourhood lookups,
//! such as LiDAR gridding/interpolation workflows.

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

/// Distance metric used when returning search distances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Return Euclidean distance values.
    Euclidean,
    /// Return squared Euclidean distance values.
    SquaredEuclidean,
}

#[derive(Clone, Copy)]
struct FixedRadiusSearchEntry2D<T: Copy> {
    x: f64,
    y: f64,
    value: T,
}

/// A 2D hash-based fixed-radius search structure.
///
/// The search checks a fixed 5x5 neighbourhood of bins around the query
/// location, then filters candidates by exact radius.
#[derive(Clone)]
pub struct FixedRadiusSearch2D<T: Copy> {
    inv_r: f64,
    r_sqr: f64,
    hm: HashMap<[i32; 2], Vec<FixedRadiusSearchEntry2D<T>>>,
    size: usize,
    is_distance_squared: bool,
    dx: [i32; 25],
    dy: [i32; 25],
}

impl<T: Copy> FixedRadiusSearch2D<T> {
    /// Create a new fixed-radius search index.
    ///
    /// `radius` must be a finite value > 0.
    pub fn new(radius: f64, metric: DistanceMetric) -> Self {
        assert!(radius.is_finite() && radius > 0.0, "radius must be finite and > 0");
        let sqr_dist = matches!(metric, DistanceMetric::SquaredEuclidean);
        Self {
            inv_r: 1.0 / (radius * 0.5),
            r_sqr: radius * radius,
            hm: HashMap::new(),
            size: 0,
            is_distance_squared: sqr_dist,
            dx: [
                -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0,
                1, 2,
            ],
            dy: [
                -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2,
                2, 2,
            ],
        }
    }

    /// Insert a point/value record.
    #[inline]
    pub fn insert(&mut self, x: f64, y: f64, value: T) {
        let key = [(x * self.inv_r).floor() as i32, (y * self.inv_r).floor() as i32];
        let val = match self.hm.entry(key) {
            Vacant(entry) => entry.insert(vec![]),
            Occupied(entry) => entry.into_mut(),
        };
        val.push(FixedRadiusSearchEntry2D { x, y, value });
        self.size += 1;
    }

    /// Search for all values within the configured radius from `(x, y)`.
    ///
    /// Returned tuples are `(value, distance)` where `distance` is either
    /// Euclidean or squared-Euclidean depending on index metric.
    pub fn search(&self, x: f64, y: f64) -> Vec<(T, f64)> {
        let i = (x * self.inv_r).floor() as i32;
        let j = (y * self.inv_r).floor() as i32;

        let mut num_points = 0usize;
        for k in 0..25 {
            if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
                num_points += vals.len();
            }
        }

        let mut ret = Vec::with_capacity(num_points);
        for k in 0..25 {
            if let Some(vals) = self.hm.get(&[i + self.dx[k], j + self.dy[k]]) {
                for val in vals {
                    let dist2 = (x - val.x) * (x - val.x) + (y - val.y) * (y - val.y);
                    if dist2 <= self.r_sqr {
                        if self.is_distance_squared {
                            ret.push((val.value, dist2));
                        } else {
                            ret.push((val.value, dist2.sqrt()));
                        }
                    }
                }
            }
        }

        ret
    }

    /// Number of inserted entries.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
}
