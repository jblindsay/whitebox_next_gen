//! Local Kriging: Ordinary Kriging with k-nearest neighbors for scalability

use crate::{GeostatError, GeostatResult};
use crate::variogram::VariogramModel;
use super::OrdinaryKriging;
use super::KrigingResult;
use rayon::prelude::*;

/// Local Ordinary Kriging: uses k-nearest neighbors for large datasets
///
/// Reduces computational complexity from O(n³) to O(k³) by predicting using
/// only the k nearest training points instead of all n points.
///
/// # Example
/// ```ignore
/// let local_ok = LocalOrdinaryKriging::new(
///     coords,
///     values,
///     variogram,
///     20,  // k = 20 nearest neighbors
/// )?;
///
/// let result = local_ok.predict((5.0, 5.0))?;
/// ```
pub struct LocalOrdinaryKriging {
    /// Training point coordinates
    training_coords: Vec<(f64, f64)>,
    /// Training point values
    training_values: Vec<f64>,
    /// Fitted variogram model
    variogram: VariogramModel,
    /// Number of nearest neighbors to use
    k: usize,
}

impl LocalOrdinaryKriging {
    /// Create new local kriging engine
    ///
    /// # Arguments
    /// * `training_coords` - Training point coordinates (x, y)
    /// * `training_values` - Training point values
    /// * `variogram` - Fitted variogram model
    /// * `k` - Number of nearest neighbors (default: 20)
    ///
    /// # Errors
    /// - If training data has fewer than 3 points
    /// - If coordinates and values have different lengths
    /// - If k is larger than number of training points
    pub fn new(
        training_coords: Vec<(f64, f64)>,
        training_values: Vec<f64>,
        variogram: VariogramModel,
        k: usize,
    ) -> GeostatResult<Self> {
        if training_coords.len() != training_values.len() {
            return Err(GeostatError::InvalidParameters(
                "coordinates and values must have same length".to_string(),
            ));
        }

        let n = training_coords.len();

        if n < 3 {
            return Err(GeostatError::InsufficientData(
                "at least 3 training points required".to_string(),
            ));
        }

        if k > n {
            return Err(GeostatError::InvalidParameters(
                format!("k ({}) cannot exceed number of training points ({})", k, n),
            ));
        }

        if k < 3 {
            return Err(GeostatError::InvalidParameters(
                "k must be at least 3 for kriging".to_string(),
            ));
        }

        Ok(LocalOrdinaryKriging {
            training_coords,
            training_values,
            variogram,
            k,
        })
    }

    /// Predict at a single target location using k-nearest neighbors
    ///
    /// # Arguments
    /// * `target` - Target location (x, y)
    ///
    /// # Returns
    /// Kriging result with prediction and uncertainty
    pub fn predict(&self, target: (f64, f64)) -> GeostatResult<KrigingResult> {
        // Find k nearest neighbors using brute-force search
        let neighbor_indices = self.find_nearest_neighbors(target);

        // Extract neighbor coordinates and values
        let neighbor_coords: Vec<(f64, f64)> = neighbor_indices
            .iter()
            .map(|&i| self.training_coords[i])
            .collect();
        let neighbor_values: Vec<f64> = neighbor_indices
            .iter()
            .map(|&i| self.training_values[i])
            .collect();

        // Create local kriging engine with neighbors
        let local_ok = OrdinaryKriging::new(neighbor_coords, neighbor_values, self.variogram.clone())?;

        // Predict using local kriging
        local_ok.predict(target)
    }

    /// Batch predict at multiple locations (parallel with rayon)
    ///
    /// # Arguments
    /// * `targets` - Target locations
    ///
    /// # Returns
    /// Vector of kriging results (same order as targets)
    pub fn predict_batch(&self, targets: &[(f64, f64)]) -> GeostatResult<Vec<KrigingResult>> {
        targets
            .par_iter()
            .map(|&t| self.predict(t))
            .collect()
    }

    /// Get the k value used for this local kriging engine
    pub fn k(&self) -> usize {
        self.k
    }

    /// Get the number of training points
    pub fn n_training(&self) -> usize {
        self.training_coords.len()
    }

    /// Get reference to variogram model
    pub fn variogram(&self) -> &VariogramModel {
        &self.variogram
    }

    /// Find k nearest neighbors using brute-force search
    /// 
    /// Returns indices of k nearest neighbors sorted by distance
    fn find_nearest_neighbors(&self, target: (f64, f64)) -> Vec<usize> {
        let mut distances: Vec<(usize, f64)> = self
            .training_coords
            .iter()
            .enumerate()
            .map(|(i, &coord)| {
                let dist = Self::distance(coord, target);
                (i, dist)
            })
            .collect();

        // Sort by distance
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Return indices of k nearest neighbors
        distances
            .iter()
            .take(self.k)
            .map(|(i, _)| *i)
            .collect()
    }

    /// Euclidean distance between two 2D points
    fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::{VariogramModel, VariogramModelFamily};

    fn create_test_data() -> (Vec<(f64, f64)>, Vec<f64>) {
        let coords = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (0.0, 10.0),
            (10.0, 10.0),
            (5.0, 5.0),
            (15.0, 5.0),
            (5.0, 15.0),
            (20.0, 20.0),
            (25.0, 25.0),
            (30.0, 30.0),
        ];
        let values = vec![1.0, 2.5, 1.5, 3.0, 2.2, 3.5, 2.8, 4.0, 4.5, 5.0];
        (coords, values)
    }

    #[test]
    fn test_local_kriging_creation() {
        let (coords, values) = create_test_data();
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let result = LocalOrdinaryKriging::new(coords, values, vario, 5);
        assert!(result.is_ok());
        let local_ok = result.unwrap();
        assert_eq!(local_ok.k(), 5);
        assert_eq!(local_ok.n_training(), 10);
    }

    #[test]
    fn test_local_kriging_insufficient_points() {
        let coords = vec![(0.0, 0.0), (10.0, 10.0)];
        let values = vec![1.0, 2.0];
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let result = LocalOrdinaryKriging::new(coords, values, vario, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_local_kriging_k_too_large() {
        let (coords, values) = create_test_data();
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let result = LocalOrdinaryKriging::new(coords, values, vario, 20);
        assert!(result.is_err()); // k > n
    }

    #[test]
    fn test_local_kriging_prediction() {
        let (coords, values) = create_test_data();
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let local_ok = LocalOrdinaryKriging::new(coords, values, vario, 5)
            .expect("failed to create local kriging");
        let result = local_ok.predict((5.0, 5.0));
        assert!(result.is_ok());

        let pred = result.unwrap();
        assert!(pred.prediction.is_finite());
        assert!(pred.variance >= 0.0);
        assert!(pred.std_error >= 0.0);
    }

    #[test]
    fn test_local_kriging_batch_prediction() {
        let (coords, values) = create_test_data();
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let local_ok = LocalOrdinaryKriging::new(coords, values, vario, 5)
            .expect("failed to create local kriging");

        let targets = vec![(5.0, 5.0), (15.0, 15.0), (25.0, 25.0)];
        let results = local_ok.predict_batch(&targets);
        assert!(results.is_ok());

        let predictions = results.unwrap();
        assert_eq!(predictions.len(), 3);
        for pred in &predictions {
            assert!(pred.prediction.is_finite());
            assert!(pred.variance >= 0.0);
        }
    }

    #[test]
    fn test_local_vs_global_kriging_similarity() {
        // Test that local kriging predictions are similar to global kriging
        // when using k close to n
        let (coords, values) = create_test_data();
        let vario = VariogramModel {
            family: VariogramModelFamily::Spherical,
            nugget: 0.1,
            partial_sill: 2.0,
            range: 10.0,
            wrss: 0.0,
            condition_number: 1.0,
        };

        // Global kriging
        let global_ok = OrdinaryKriging::new(coords.clone(), values.clone(), vario.clone())
            .expect("failed to create global kriging");
        let global_pred = global_ok.predict((5.0, 5.0)).expect("global prediction failed");

        // Local kriging with k = n
        let local_ok = LocalOrdinaryKriging::new(coords, values, vario, 10)
            .expect("failed to create local kriging");
        let local_pred = local_ok.predict((5.0, 5.0)).expect("local prediction failed");

        // Predictions should be identical (same training points used)
        assert!((global_pred.prediction - local_pred.prediction).abs() < 1e-6);
        assert!((global_pred.variance - local_pred.variance).abs() < 1e-6);
    }
}
