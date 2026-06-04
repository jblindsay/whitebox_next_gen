//! Ripley's K and L functions for point pattern analysis
//!
//! Computes cumulative distance-based statistics that characterize clustering/dispersion
//! in spatial point patterns.

use crate::GeostatError;
use serde::{Deserialize, Serialize};
use rayon::prelude::*;

/// Result from Ripley's K function computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KFunctionResult {
    /// Distance bins
    pub distances: Vec<f64>,
    /// K(t) values (observed)
    pub k_values: Vec<f64>,
    /// L(t) values (L(t) = sqrt(K(t)/π) - t, where L(t)=0 is CSR)
    pub l_values: Vec<f64>,
    /// Number of points in pattern
    pub n_points: usize,
    /// Study area
    pub area: f64,
    /// Intensity lambda = n / area
    pub intensity: f64,
}

/// Ripley's K function computation
#[derive(Debug)]
pub struct KFunction {
    /// Point coordinates (x, y)
    points: Vec<(f64, f64)>,
    /// Study area bounds
    bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
}

impl KFunction {
    /// Create a K function calculator
    pub fn new(points: Vec<(f64, f64)>) -> Result<Self, GeostatError> {
        if points.len() < 3 {
            return Err(GeostatError::InsufficientData(
                "at least 3 points required for K function".to_string(),
            ));
        }

        let min_x = points.iter().map(|(x, _)| x).copied().fold(f64::INFINITY, f64::min);
        let max_x = points.iter().map(|(x, _)| x).copied().fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| y).copied().fold(f64::INFINITY, f64::min);
        let max_y = points.iter().map(|(_, y)| y).copied().fold(f64::NEG_INFINITY, f64::max);

        Ok(KFunction {
            points,
            bounds: (min_x, min_y, max_x, max_y),
        })
    }

    /// Compute K(t) and L(t) for specified distances
    /// 
    /// # Arguments
    /// * `distances` - Distance values at which to compute K
    /// 
    /// # Returns
    /// K function result with K(t) and L(t) values
    pub fn compute(&self, distances: &[f64]) -> Result<KFunctionResult, GeostatError> {
        if distances.is_empty() {
            return Err(GeostatError::InvalidParameters("no distances specified".to_string()));
        }

        let n = self.points.len() as f64;
        let area = (self.bounds.2 - self.bounds.0) * (self.bounds.3 - self.bounds.1);
        let intensity = n / area;

        let k_values: Vec<f64> = distances
            .par_iter()
            .map(|&t| {
                let mut count = 0.0;
                for i in 0..self.points.len() {
                    for j in 0..self.points.len() {
                        if i != j {
                            let dx = self.points[i].0 - self.points[j].0;
                            let dy = self.points[i].1 - self.points[j].1;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist <= t {
                                count += 1.0;
                            }
                        }
                    }
                }
                // Normalize: K(t) = A/n^2 * sum of pairs with distance <= t
                (area / (n * n)) * count
            })
            .collect();

        let l_values: Vec<f64> = distances
            .iter()
            .zip(k_values.iter())
            .map(|(t, k)| {
                let k_norm = k / std::f64::consts::PI;
                k_norm.sqrt() - t
            })
            .collect();

        Ok(KFunctionResult {
            distances: distances.to_vec(),
            k_values,
            l_values,
            n_points: self.points.len(),
            area,
            intensity,
        })
    }

    /// Recommended maximum distance for K function analysis
    /// Typically 1/4 of the minimum study area dimension
    pub fn recommended_max_distance(&self) -> f64 {
        let width = self.bounds.2 - self.bounds.0;
        let height = self.bounds.3 - self.bounds.1;
        width.min(height) / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k_function_csr_homogeneous() {
        // Complete spatial randomness (CSR) should have L(t) ≈ 0
        // Create uniform random points in [0,1]^2
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let points: Vec<(f64, f64)> = (0..100)
            .map(|_| (rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0)))
            .collect();

        let kf = KFunction::new(points).unwrap();
        let distances = vec![0.05, 0.1, 0.15, 0.2];
        let result = kf.compute(&distances).unwrap();

        // L values should be relatively close to 0 for CSR
        for l in &result.l_values {
            assert!(l.abs() < 0.1); // Loose bound for random data
        }
    }

    #[test]
    fn test_k_function_clustered() {
        // Clustered pattern should have L(t) > 0
        let mut points = vec![];
        for cx in &[0.25, 0.75] {
            for cy in &[0.25, 0.75] {
                for _ in 0..10 {
                    points.push((cx + 0.02, cy + 0.02));
                }
            }
        }

        let kf = KFunction::new(points).unwrap();
        let distances = vec![0.05, 0.1, 0.15];
        let result = kf.compute(&distances).unwrap();

        // Small distances should show positive L (clustering)
        assert!(result.l_values[0] > 0.0);
    }

    #[test]
    fn test_k_function_minimum_points() {
        let result = KFunction::new(vec![(0.0, 0.0), (1.0, 1.0)]);
        assert!(result.is_err()); // < 3 points
    }

    #[test]
    fn test_k_function_computation() {
        let points = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ];

        let kf = KFunction::new(points).unwrap();
        let distances = vec![0.5, 1.0, 1.5];
        let result = kf.compute(&distances).unwrap();

        assert_eq!(result.n_points, 4);
        assert_eq!(result.distances.len(), 3);
        assert_eq!(result.k_values.len(), 3);
        assert_eq!(result.l_values.len(), 3);

        // K values should be non-decreasing
        for i in 0..2 {
            assert!(result.k_values[i] <= result.k_values[i + 1]);
        }
    }
}
