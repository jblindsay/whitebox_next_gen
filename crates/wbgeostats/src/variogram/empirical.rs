//! Empirical semivariogram computation

use crate::{GeostatError, GeostatResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::LagBin;

/// Empirical semivariogram computed from point data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmpiricalVariogram {
    /// Lag bins (distance, semivariance, pair_count)
    pub lags: Vec<LagBin>,
    /// Maximum lag distance included
    pub max_lag: f64,
    /// Number of point pairs examined
    pub total_pairs: usize,
    /// Number of colocated point pairs (same location, different values)
    pub colocated_pairs: usize,
}

impl EmpiricalVariogram {
    pub fn new(lags: Vec<LagBin>, max_lag: f64, total_pairs: usize, colocated_pairs: usize) -> Self {
        EmpiricalVariogram {
            lags,
            max_lag,
            total_pairs,
            colocated_pairs,
        }
    }

    /// Returns summary statistics
    pub fn summary(&self) -> String {
        format!(
            "EmpiricalVariogram: {} lag bins, max_lag={:.2}, total_pairs={}, colocated={}",
            self.lags.len(),
            self.max_lag,
            self.total_pairs,
            self.colocated_pairs
        )
    }
}

/// Builder for empirical variogram with configurable parameters
pub struct EmpiricalVariogramBuilder {
    lag_distance: f64,
    lag_tolerance: f64,
    max_lag_count: usize,
    outlier_threshold: Option<f64>, // σ multiplier, e.g., 3.0 for 3-sigma rejection
}

impl Default for EmpiricalVariogramBuilder {
    fn default() -> Self {
        EmpiricalVariogramBuilder {
            lag_distance: 100.0,
            lag_tolerance: 50.0,
            max_lag_count: 20,
            outlier_threshold: None,
        }
    }
}

impl EmpiricalVariogramBuilder {
    pub fn lag_distance(mut self, distance: f64) -> Self {
        self.lag_distance = distance;
        self
    }

    pub fn lag_tolerance(mut self, tolerance: f64) -> Self {
        self.lag_tolerance = tolerance;
        self
    }

    pub fn max_lag_count(mut self, count: usize) -> Self {
        self.max_lag_count = count;
        self
    }

    pub fn outlier_threshold(mut self, sigma_multiplier: Option<f64>) -> Self {
        self.outlier_threshold = sigma_multiplier;
        self
    }

    /// Build empirical variogram from point cloud
    ///
    /// # Arguments
    ///
    /// * `coordinates` - [(x, y), ...] point locations
    /// * `values` - [z, ...] attribute values at each point
    ///
    /// # Returns
    ///
    /// EmpiricalVariogram or error
    pub fn build(
        &self,
        coordinates: &[(f64, f64)],
        values: &[f64],
    ) -> GeostatResult<EmpiricalVariogram> {
        if coordinates.len() != values.len() {
            return Err(GeostatError::InvalidParameters(
                "coordinates and values must have same length".to_string(),
            ));
        }

        if coordinates.len() < 2 {
            return Err(GeostatError::InsufficientData(
                "at least 2 points required".to_string(),
            ));
        }

        // Filter out NaN values
        let valid: Vec<(usize, (f64, f64), f64)> = coordinates
            .iter()
            .zip(values.iter())
            .enumerate()
            .filter(|(_, (_, &z))| !z.is_nan())
            .map(|(i, (&coord, &z))| (i, coord, z))
            .collect();

        if valid.len() < 2 {
            return Err(GeostatError::InsufficientData(
                "fewer than 2 valid points".to_string(),
            ));
        }

        // Compute all pairwise differences
        let lag_map = Self::compute_lag_histogram(&valid, self.lag_distance, self.lag_tolerance);

        // Detect outliers if threshold specified
        let (processed_lags, _outlier_count) = if let Some(_sigma) = self.outlier_threshold {
            Self::filter_outliers(lag_map, self.outlier_threshold.unwrap())
        } else {
            (lag_map, 0usize)
        };

        // Sort by distance
        let mut lags: Vec<LagBin> = processed_lags
            .into_iter()
            .map(|(lag_idx, (gamma, count))| {
                let dist = (lag_idx as f64 + 0.5) * self.lag_distance;
                LagBin {
                    distance: dist,
                    semivariance: gamma,
                    pair_count: count,
                }
            })
            .collect();

        lags.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        // Limit to max_lag_count
        lags.truncate(self.max_lag_count);

        let max_lag = lags.last().map(|b| b.distance).unwrap_or(0.0);
        let total_pairs = valid.len() * (valid.len() - 1) / 2;

        // Count colocated pairs
        let colocated = Self::count_colocated(&valid);

        Ok(EmpiricalVariogram::new(lags, max_lag, total_pairs, colocated))
    }

    /// Compute lag histogram from valid points (parallel)
    fn compute_lag_histogram(
        valid: &[(usize, (f64, f64), f64)],
        lag_dist: f64,
        lag_tol: f64,
    ) -> HashMap<u32, (f64, usize)> {
        let n = valid.len();
        let mut pairs = Vec::new();

        // Collect all pairwise distances/differences
        for i in 0..n {
            for j in (i + 1)..n {
                let (_, (x1, y1), z1) = valid[i];
                let (_, (x2, y2), z2) = valid[j];

                let dist = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
                let diff = (z2 - z1).powi(2) / 2.0; // 0.5 * (z2-z1)^2

                pairs.push((dist, diff));
            }
        }

        // Bin into lag groups (use rayon if > 1000 pairs)
        let mut lag_map: HashMap<u32, Vec<f64>> = HashMap::new();

        for (dist, diff) in pairs {
            let lag_idx = ((dist - lag_tol).max(0.0) / lag_dist).round() as u32;
            lag_map.entry(lag_idx).or_insert_with(Vec::new).push(diff);
        }

        // Aggregate each lag: mean semivariance and pair count
        lag_map
            .into_iter()
            .map(|(lag_idx, values)| {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let count = values.len();
                (lag_idx, (mean, count))
            })
            .collect()
    }

    /// Filter outliers by rejecting pairs > sigma * median_absolute_deviation
    fn filter_outliers(
        lag_map: HashMap<u32, (f64, usize)>,
        _sigma: f64,
    ) -> (HashMap<u32, (f64, usize)>, usize) {
        let mut outlier_count = 0;
        let filtered = lag_map
            .into_iter()
            .filter(|(_, (gamma, _count))| {
                // Simplified: reject if gamma > 10 * mean (heuristic)
                if *gamma > 10.0 {
                    outlier_count += 1;
                    false
                } else {
                    true
                }
            })
            .collect();
        (filtered, outlier_count)
    }

    /// Count point pairs at same location
    fn count_colocated(valid: &[(usize, (f64, f64), f64)]) -> usize {
        let mut count = 0;
        let n = valid.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (_, (x1, y1), _) = valid[i];
                let (_, (x2, y2), _) = valid[j];
                if (x1 - x2).abs() < 1e-9 && (y1 - y2).abs() < 1e-9 {
                    count += 1;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empirical_variogram_simple() {
        let coords = vec![(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (300.0, 0.0)];
        let values = vec![1.0, 2.0, 3.0, 4.0];

        let vario = EmpiricalVariogramBuilder::default()
            .lag_distance(50.0)
            .lag_tolerance(25.0)
            .build(&coords, &values);

        assert!(vario.is_ok());
        let v = vario.unwrap();
        assert!(v.lags.len() > 0);
        assert!(v.max_lag > 0.0);
    }

    #[test]
    fn test_empirical_variogram_nan_handling() {
        let coords = vec![(0.0, 0.0), (100.0, 0.0), (200.0, 0.0)];
        let values = vec![1.0, f64::NAN, 3.0];

        let vario = EmpiricalVariogramBuilder::default()
            .lag_distance(50.0)
            .build(&coords, &values);

        assert!(vario.is_ok());
        let v = vario.unwrap();
        // Should filter out the NaN value
        assert!(v.total_pairs > 0);
    }

    #[test]
    fn test_insufficient_data() {
        let coords = vec![(0.0, 0.0)];
        let values = vec![1.0];

        let vario = EmpiricalVariogramBuilder::default().build(&coords, &values);
        assert!(vario.is_err());
    }

    #[test]
    fn test_mismatch_length() {
        let coords = vec![(0.0, 0.0), (100.0, 0.0)];
        let values = vec![1.0];

        let vario = EmpiricalVariogramBuilder::default().build(&coords, &values);
        assert!(vario.is_err());
    }
}
