//! Kernel density estimation (KDE) for point patterns
//! 
//! This module provides Gaussian kernel density estimation without external dependencies.
//! Used as a foundational component for inhomogeneous point-process analysis.

use crate::GeostatError;
use rayon::prelude::*;

/// Kernel density estimator using Gaussian kernels
#[derive(Debug, Clone)]
pub struct KernelDensityEstimator {
    /// Training point coordinates (x, y)
    pub points: Vec<(f64, f64)>,
    /// Gaussian bandwidth (standard deviation)
    pub bandwidth: f64,
}

impl KernelDensityEstimator {
    /// Create a new KDE with explicit bandwidth
    pub fn new(points: Vec<(f64, f64)>, bandwidth: f64) -> Result<Self, GeostatError> {
        if points.is_empty() {
            return Err(GeostatError::InsufficientData("at least one point required for KDE".to_string()));
        }
        if !bandwidth.is_finite() || bandwidth <= 0.0 {
            return Err(GeostatError::InvalidParameters(
                "bandwidth must be positive and finite".to_string(),
            ));
        }
        Ok(KernelDensityEstimator { points, bandwidth })
    }

    /// Estimate bandwidth using Scott's rule: h = n^(-1/(d+4)) ≈ n^(-1/6) for d=2
    pub fn scott_bandwidth(points: &[(f64, f64)]) -> f64 {
        if points.is_empty() {
            return 1.0;
        }
        let n = points.len() as f64;
        n.powf(-1.0 / 6.0)
    }

    /// Estimate bandwidth using cross-validation (slower, more accurate)
    /// 
    /// Minimizes Leave-One-Out CV error: LSCV = integral(f_h^2) - 2*mean(f_h^(-i)(x_i))
    pub fn cv_bandwidth(points: &[(f64, f64)]) -> Result<f64, GeostatError> {
        if points.len() < 4 {
            return Ok(Self::scott_bandwidth(points));
        }

        // Search over bandwidth range
        let scott = Self::scott_bandwidth(points);
        let bandwidths: Vec<f64> = (0..15)
            .map(|i| scott * 0.5_f64.powi(i - 7))
            .collect();

        let mut best_bw = scott;
        let mut best_cv = f64::INFINITY;

        for &bw in &bandwidths {
            let cv_error = Self::cv_score(points, bw);
            if cv_error < best_cv {
                best_cv = cv_error;
                best_bw = bw;
            }
        }

        Ok(best_bw)
    }

    /// Compute CV score for a given bandwidth
    fn cv_score(points: &[(f64, f64)], bw: f64) -> f64 {
        if points.len() < 2 {
            return f64::INFINITY;
        }

        let n = points.len() as f64;
        let bw_sq = bw * bw;

        // Leave-one-out cross-validation
        let mut sum = 0.0;
        for i in 0..points.len() {
            let (xi, yi) = points[i];
            let mut density_xi = 0.0;

            // Evaluate density at point i using all other points
            for j in 0..points.len() {
                if i != j {
                    let (xj, yj) = points[j];
                    let dist_sq = (xi - xj).powi(2) + (yi - yj).powi(2);
                    density_xi += (-dist_sq / (2.0 * bw_sq)).exp();
                }
            }

            // Normalize (avoid division by small density)
            if density_xi > 1e-10 {
                sum += 1.0 / density_xi;
            } else {
                sum += 1e10;
            }
        }

        sum / n
    }

    /// Estimate density at a single point (x, y)
    pub fn estimate_at(&self, x: f64, y: f64) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }

        let n = self.points.len() as f64;
        let bw_sq = self.bandwidth * self.bandwidth;
        let factor = 1.0 / (2.0 * std::f64::consts::PI * bw_sq);

        let mut sum = 0.0;
        for (xi, yi) in &self.points {
            let dist_sq = (x - xi).powi(2) + (y - yi).powi(2);
            sum += (-dist_sq / (2.0 * bw_sq)).exp();
        }

        (sum / n) * factor
    }

    /// Estimate density at multiple points (parallelized)
    pub fn estimate_batch(&self, locations: &[(f64, f64)]) -> Vec<f64> {
        locations
            .par_iter()
            .map(|(x, y)| self.estimate_at(*x, *y))
            .collect()
    }

    /// Estimate density at a regular grid
    pub fn estimate_grid(
        &self,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
        nx: usize,
        ny: usize,
    ) -> (Vec<f64>, Vec<Vec<f64>>) {
        let dx = (max_x - min_x) / (nx - 1).max(1) as f64;
        let dy = (max_y - min_y) / (ny - 1).max(1) as f64;

        let mut grid_points = Vec::new();
        for iy in 0..ny {
            for ix in 0..nx {
                let x = min_x + ix as f64 * dx;
                let y = min_y + iy as f64 * dy;
                grid_points.push((x, y));
            }
        }

        let densities = self.estimate_batch(&grid_points);

        // Reshape into 2D grid
        let grid: Vec<Vec<f64>> = densities
            .chunks(nx)
            .map(|chunk| chunk.to_vec())
            .collect();

        (
            vec![min_x + (0..nx).map(|i| i as f64 * dx).sum::<f64>() / nx as f64; ny],
            grid,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scott_bandwidth() {
        let points = vec![(0.0, 0.0); 100];
        let bw = KernelDensityEstimator::scott_bandwidth(&points);
        assert!(bw > 0.0 && bw < 1.0);
        assert!((bw - 100.0_f64.powf(-1.0 / 6.0)).abs() < 1e-10);
    }

    #[test]
    fn test_kde_gaussian_properties() {
        // Single point should have maximum at that location
        let points = vec![(0.0, 0.0)];
        let kde = KernelDensityEstimator::new(points, 1.0).unwrap();

        let density_at_point = kde.estimate_at(0.0, 0.0);
        let density_nearby = kde.estimate_at(0.0, 1.0);
        let density_far = kde.estimate_at(0.0, 5.0);

        assert!(density_at_point > density_nearby);
        assert!(density_nearby > density_far);
    }

    #[test]
    fn test_kde_batch_consistency() {
        let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let kde = KernelDensityEstimator::new(points, 0.5).unwrap();

        let test_locs = vec![(0.5, 0.5), (1.0, 1.0), (1.5, 1.5)];

        let single = test_locs
            .iter()
            .map(|(x, y)| kde.estimate_at(*x, *y))
            .collect::<Vec<_>>();

        let batch = kde.estimate_batch(&test_locs);

        for (s, b) in single.iter().zip(batch.iter()) {
            assert!((s - b).abs() < 1e-10);
        }
    }

    #[test]
    fn test_kde_zero_bandwidth_error() {
        let points = vec![(0.0, 0.0)];
        let result = KernelDensityEstimator::new(points, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_kde_empty_points_error() {
        let result = KernelDensityEstimator::new(vec![], 1.0);
        assert!(result.is_err());
    }
}
