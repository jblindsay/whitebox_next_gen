//! Inhomogeneous Poisson process modeling with intensity-weighted K function
//!
//! Accounts for non-uniform spatial intensity using kernel density estimation.
//! Allows testing whether departures from randomness reflect genuine clustering
//! or simply non-uniform underlying intensity.

use crate::density_estimation::KernelDensityEstimator;
use crate::GeostatError;
use serde::{Deserialize, Serialize};

/// Result from inhomogeneous K function analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InhomogeneousResult {
    /// Estimated intensity surface (intensity values at points)
    pub intensities: Vec<f64>,
    /// Inhomogeneous K values (intensity-corrected)
    pub k_values: Vec<f64>,
    /// Inhomogeneous L values
    pub l_values: Vec<f64>,
    /// Distances at which K/L were evaluated
    pub distances: Vec<f64>,
    /// KDE bandwidth used for intensity estimation
    pub bandwidth: f64,
    /// Number of points
    pub n_points: usize,
}

/// Inhomogeneous K-function analysis
pub struct InhomogeneousKProcess {
    /// Point coordinates
    points: Vec<(f64, f64)>,
    /// Estimated intensity values at each point
    intensities: Vec<f64>,
    /// Study bounds
    bounds: (f64, f64, f64, f64),
}

impl InhomogeneousKProcess {
    /// Create inhomogeneous K-process using KDE for intensity estimation
    pub fn new(
        points: Vec<(f64, f64)>,
        bandwidth: Option<f64>,
    ) -> Result<Self, GeostatError> {
        if points.len() < 5 {
            return Err(GeostatError::InsufficientData(
                "at least 5 points required for inhomogeneous analysis".to_string(),
            ));
        }

        // Create KDE for intensity estimation
        let bw = if let Some(bw) = bandwidth {
            bw
        } else {
            KernelDensityEstimator::scott_bandwidth(&points)
        };

        let kde = KernelDensityEstimator::new(points.clone(), bw)?;

        // Estimate intensity at each point
        let intensities: Vec<f64> = points.iter().map(|(x, y)| kde.estimate_at(*x, *y)).collect();

        let min_x = points.iter().map(|(x, _)| x).copied().fold(f64::INFINITY, f64::min);
        let max_x = points.iter().map(|(x, _)| x).copied().fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| y).copied().fold(f64::INFINITY, f64::min);
        let max_y = points.iter().map(|(_, y)| y).copied().fold(f64::NEG_INFINITY, f64::max);

        Ok(InhomogeneousKProcess {
            points,
            intensities,
            bounds: (min_x, min_y, max_x, max_y),
        })
    }

    /// Compute inhomogeneous K function
    /// 
    /// Uses weighted pairwise distances where weights are inversely proportional to intensity
    pub fn compute_k_inhom(&self, distances: &[f64]) -> Result<InhomogeneousResult, GeostatError> {
        if distances.is_empty() {
            return Err(GeostatError::InvalidParameters("no distances specified".to_string()));
        }

        let n = self.points.len() as f64;
        let area = (self.bounds.2 - self.bounds.0) * (self.bounds.3 - self.bounds.1);

        let k_values: Vec<f64> = distances
            .iter()
            .map(|&t| {
                let mut sum = 0.0;
                for i in 0..self.points.len() {
                    for j in 0..self.points.len() {
                        if i != j {
                            let dx = self.points[i].0 - self.points[j].0;
                            let dy = self.points[i].1 - self.points[j].1;
                            let dist = (dx * dx + dy * dy).sqrt();
                            
                            if dist <= t {
                                // Weight inversely proportional to intensity
                                let weight = 1.0 / (self.intensities[i] * self.intensities[j]).max(1e-10);
                                sum += weight;
                            }
                        }
                    }
                }
                // Normalize
                (area / (n * n)) * sum
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

        Ok(InhomogeneousResult {
            intensities: self.intensities.clone(),
            k_values,
            l_values,
            distances: distances.to_vec(),
            bandwidth: KernelDensityEstimator::scott_bandwidth(&self.points),
            n_points: self.points.len(),
        })
    }

    /// Get intensity at a specific point location
    pub fn intensity_at_point(&self, idx: usize) -> Result<f64, GeostatError> {
        if idx >= self.points.len() {
            return Err(GeostatError::InvalidParameters("point index out of range".to_string()));
        }
        Ok(self.intensities[idx])
    }

    /// Get all intensity values
    pub fn all_intensities(&self) -> &[f64] {
        &self.intensities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inhomogeneous_k_process_creation() {
        let points = vec![
            (0.0, 0.0),
            (0.1, 0.1),
            (0.2, 0.2),
            (0.5, 0.5),
            (0.9, 0.9),
        ];

        let result = InhomogeneousKProcess::new(points, None);
        assert!(result.is_ok());

        let process = result.unwrap();
        assert_eq!(process.all_intensities().len(), 5);

        // Intensities should be positive
        for &intensity in process.all_intensities() {
            assert!(intensity > 0.0);
        }
    }

    #[test]
    fn test_inhomogeneous_k_computation() {
        let points = vec![
            (0.0, 0.0),
            (0.1, 0.1),
            (0.2, 0.2),
            (0.5, 0.5),
            (0.9, 0.9),
        ];

        let process = InhomogeneousKProcess::new(points, None).unwrap();
        let distances = vec![0.2, 0.4, 0.6];

        let result = process.compute_k_inhom(&distances).unwrap();

        assert_eq!(result.distances.len(), 3);
        assert_eq!(result.k_values.len(), 3);
        assert_eq!(result.l_values.len(), 3);

        // K values should be non-decreasing
        for i in 0..2 {
            assert!(result.k_values[i] <= result.k_values[i + 1]);
        }
    }

    #[test]
    fn test_inhomogeneous_insufficient_points() {
        let points = vec![(0.0, 0.0), (1.0, 1.0)];
        let result = InhomogeneousKProcess::new(points, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_intensity_at_point() {
        let points = vec![
            (0.0, 0.0),
            (0.5, 0.5),
            (1.0, 1.0),
            (0.25, 0.25),
            (0.75, 0.75),
        ];

        let process = InhomogeneousKProcess::new(points, None).unwrap();

        let intensity_0 = process.intensity_at_point(0).unwrap();
        assert!(intensity_0 > 0.0);

        let result = process.intensity_at_point(10);
        assert!(result.is_err());
    }
}
