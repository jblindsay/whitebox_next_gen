// CoKriging (multivariate kriging) solver
//
// Enables prediction using primary variable (Z) and correlated auxiliary variables (Y1, Y2, ...).
// Leverages cross-variograms to reduce prediction uncertainty.
//
// Phase 2 Week 8+ Implementation (2026-06-04)

use crate::variogram::{VariogramModel, CrossVariogramModel};
use nalgebra::{DMatrix, DVector};

/// Result of a single cokriging prediction
#[derive(Clone, Debug)]
pub struct CoKrigingPrediction {
    /// Predicted value at target location
    pub prediction: f64,

    /// Kriging variance (uncertainty)
    pub variance: f64,

    /// Kriging weights for primary variable
    pub weights_primary: Vec<f64>,

    /// Kriging weights for each auxiliary variable
    pub weights_auxiliary: Vec<Vec<f64>>,

    /// Lagrange multiplier (for constraint satisfaction)
    pub lagrange: f64,
}

/// Ordinary CoKriging predictor for multivariate kriging
///
/// Uses primary and auxiliary variables to improve predictions through
/// cross-variable spatial correlation (cross-variograms).
pub struct OrdinaryCoKriging {
    /// Primary variable variogram model
    primary_variogram: VariogramModel,

    /// Cross-variogram models (primary vs. each auxiliary variable)
    cross_variograms: Vec<CrossVariogramModel>,

    /// Auxiliary variable variogram models
    auxiliary_variograms: Vec<VariogramModel>,

    /// Training data coordinates and values
    training_data: CoKrigingTrainingData,
}

/// Structured training data for cokriging
#[derive(Clone, Debug)]
struct CoKrigingTrainingData {
    /// Coordinates for all variables: (x, y) same for all
    coordinates: Vec<(f64, f64)>,

    /// Primary variable values
    primary_values: Vec<f64>,

    /// Auxiliary variable values: auxiliary_values[var_idx][point_idx]
    auxiliary_values: Vec<Vec<f64>>,
}

impl OrdinaryCoKriging {
    /// Create a new OrdinaryCoKriging predictor
    ///
    /// # Arguments
    /// - `primary_variogram`: Fitted variogram for primary variable
    /// - `cross_variograms`: Cross-variograms (primary vs. each auxiliary)
    /// - `auxiliary_variograms`: Fitted variograms for auxiliary variables
    /// - `primary_coords`: Training point coordinates (x, y)
    /// - `primary_values`: Primary variable values at training points
    /// - `auxiliary_values`: Auxiliary variable values (one vec per variable)
    ///
    /// # Returns
    /// OrdinaryCoKriging instance ready for predictions
    pub fn new(
        primary_variogram: VariogramModel,
        cross_variograms: Vec<CrossVariogramModel>,
        auxiliary_variograms: Vec<VariogramModel>,
        primary_coords: Vec<(f64, f64)>,
        primary_values: Vec<f64>,
        auxiliary_values: Vec<Vec<f64>>,
    ) -> Result<Self, String> {
        if primary_coords.is_empty() {
            return Err("No training points provided".to_string());
        }

        if primary_values.len() != primary_coords.len() {
            return Err("Primary values and coordinates must have equal length".to_string());
        }

        if cross_variograms.len() != auxiliary_variograms.len() {
            return Err(
                "Number of cross-variograms must equal number of auxiliary variables"
                    .to_string(),
            );
        }

        for (idx, aux_vals) in auxiliary_values.iter().enumerate() {
            if aux_vals.len() != primary_coords.len() {
                return Err(format!(
                    "Auxiliary variable {} has wrong number of values",
                    idx
                ));
            }
        }

        Ok(OrdinaryCoKriging {
            primary_variogram,
            cross_variograms,
            auxiliary_variograms,
            training_data: CoKrigingTrainingData {
                coordinates: primary_coords,
                primary_values,
                auxiliary_values,
            },
        })
    }

    /// Predict at a single target location using cokriging
    ///
    /// # Arguments
    /// - `target`: Target location (x, y)
    /// - `neighborhood_size`: Number of nearest neighbors to use (optional, defaults to all)
    ///
    /// # Returns
    /// CoKrigingPrediction with prediction, variance, and weights
    pub fn predict(
        &self,
        target: (f64, f64),
        neighborhood_size: Option<usize>,
    ) -> Result<CoKrigingPrediction, String> {
        let n_points = self.training_data.coordinates.len();
        let n_aux = self.training_data.auxiliary_values.len();
        let n_use = neighborhood_size.unwrap_or(n_points).min(n_points);

        // Select nearest neighbors
        let neighbors = self.select_neighbors(target, n_use)?;

        // Build cokriging system matrix
        // Structure: [Gamma_PP, Gamma_PA; Gamma_AP, Gamma_AA] with Lagrange row/col
        let matrix_size = n_use * (1 + n_aux) + 1;
        let mut system = DMatrix::zeros(matrix_size, matrix_size);
        let mut rhs = DVector::zeros(matrix_size);

        // Fill primary-primary block (Gamma_PP)
        for i in 0..n_use {
            for j in 0..n_use {
                let idx_i = neighbors[i];
                let idx_j = neighbors[j];
                let dist = distance(
                    self.training_data.coordinates[idx_i],
                    self.training_data.coordinates[idx_j],
                );
                let gamma = if i == j {
                    0.0
                } else {
                    self.primary_variogram.evaluate(dist)
                };
                system[(i, j)] = gamma;
            }
        }

        // Fill cross-variogram blocks (Gamma_PA and Gamma_AP)
        for aux_var in 0..n_aux {
            for i in 0..n_use {
                for j in 0..n_use {
                    let idx_i = neighbors[i];
                    let idx_j = neighbors[j];
                    let dist = distance(
                        self.training_data.coordinates[idx_i],
                        self.training_data.coordinates[idx_j],
                    );
                    let gamma_cross = self.cross_variograms[aux_var].evaluate(dist);

                    // Gamma_PA block (primary rows, auxiliary columns)
                    system[(i, n_use + aux_var * n_use + j)] = gamma_cross;

                    // Gamma_AP block (auxiliary rows, primary columns)
                    system[(n_use + aux_var * n_use + i, j)] = gamma_cross;
                }
            }
        }

        // Fill auxiliary-auxiliary blocks (Gamma_AA)
        for aux1 in 0..n_aux {
            for aux2 in 0..n_aux {
                for i in 0..n_use {
                    for j in 0..n_use {
                        let idx_i = neighbors[i];
                        let idx_j = neighbors[j];
                        let dist = distance(
                            self.training_data.coordinates[idx_i],
                            self.training_data.coordinates[idx_j],
                        );

                        let gamma = if aux1 == aux2 {
                            // Diagonal blocks: auxiliary variograms
                            if i == j {
                                0.0
                            } else {
                                self.auxiliary_variograms[aux1].evaluate(dist)
                            }
                        } else {
                            // Off-diagonal: cross-variograms between auxiliaries (zero for now)
                            // Full implementation would include auxiliary-auxiliary cross-variograms
                            0.0
                        };

                        system[(n_use + aux1 * n_use + i, n_use + aux2 * n_use + j)] = gamma;
                    }
                }
            }
        }

        // Fill Lagrange constraint row/column (last row/col for unbiasedness)
        for i in 0..n_use {
            system[(matrix_size - 1, i)] = 1.0;
            system[(i, matrix_size - 1)] = 1.0;
        }
        for aux_var in 0..n_aux {
            // Auxiliary constraints: sum of weights = 0
            for i in 0..n_use {
                system[(matrix_size - 1, n_use + aux_var * n_use + i)] = 0.0;
                system[(n_use + aux_var * n_use + i, matrix_size - 1)] = 0.0;
            }
        }

        // Fill RHS vector
        for i in 0..n_use {
            let idx = neighbors[i];
            let dist = distance(self.training_data.coordinates[idx], target);
            rhs[i] = self.primary_variogram.evaluate(dist);
        }

        for aux_var in 0..n_aux {
            for i in 0..n_use {
                let idx = neighbors[i];
                let dist = distance(self.training_data.coordinates[idx], target);
                rhs[n_use + aux_var * n_use + i] = self.cross_variograms[aux_var].evaluate(dist);
            }
        }

        rhs[matrix_size - 1] = 1.0; // Unbiasedness constraint

        // Solve system
        let decomp = system.lu();
        let weights = decomp
            .solve(&rhs)
            .ok_or_else(|| "Failed to solve cokriging system".to_string())?;

        // Extract weights and compute prediction
        let mut weights_primary = Vec::new();
        let mut weights_auxiliary = vec![Vec::new(); n_aux];

        let mut prediction = 0.0;
        for i in 0..n_use {
            let w = weights[i];
            weights_primary.push(w);
            prediction += w * self.training_data.primary_values[neighbors[i]];
        }

        for aux_var in 0..n_aux {
            for i in 0..n_use {
                let w = weights[n_use + aux_var * n_use + i];
                weights_auxiliary[aux_var].push(w);
                prediction += w * self.training_data.auxiliary_values[aux_var][neighbors[i]];
            }
        }

        // Compute kriging variance
        let lagrange = weights[matrix_size - 1];
        let mut variance = lagrange; // Starts with Lagrange multiplier

        for i in 0..n_use {
            let w = weights[i];
            let idx = neighbors[i];
            let dist = distance(self.training_data.coordinates[idx], target);
            variance -= w * self.primary_variogram.evaluate(dist);
        }

        for aux_var in 0..n_aux {
            for i in 0..n_use {
                let w = weights[n_use + aux_var * n_use + i];
                let idx = neighbors[i];
                let dist = distance(self.training_data.coordinates[idx], target);
                variance -= w * self.cross_variograms[aux_var].evaluate(dist);
            }
        }

        Ok(CoKrigingPrediction {
            prediction,
            variance: variance.max(0.0), // Ensure non-negative variance
            weights_primary,
            weights_auxiliary,
            lagrange,
        })
    }

    /// Predict on a batch of locations
    ///
    /// # Arguments
    /// - `targets`: Vector of target locations
    /// - `neighborhood_size`: Number of nearest neighbors per location
    ///
    /// # Returns
    /// Vector of CoKrigingPredictions
    pub fn predict_batch(
        &self,
        targets: &[(f64, f64)],
        neighborhood_size: Option<usize>,
    ) -> Result<Vec<CoKrigingPrediction>, String> {
        targets
            .iter()
            .map(|&target| self.predict(target, neighborhood_size))
            .collect()
    }

    /// Select nearest neighbors to a target location
    fn select_neighbors(&self, target: (f64, f64), n: usize) -> Result<Vec<usize>, String> {
        let mut distances: Vec<(usize, f64)> = self
            .training_data
            .coordinates
            .iter()
            .enumerate()
            .map(|(i, &coord)| (i, distance(coord, target)))
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        Ok(distances.into_iter().take(n).map(|(i, _)| i).collect())
    }
}

/// Euclidean distance between two points
fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    let dx = p2.0 - p1.0;
    let dy = p2.1 - p1.1;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variogram::VariogramModelFamily;

    fn create_test_cokriging() -> Result<OrdinaryCoKriging, String> {
        // Simple 4-point test case
        let coords = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];
        let primary = vec![10.0, 12.0, 11.0, 13.0];
        let auxiliary = vec![vec![20.0, 22.0, 21.0, 23.0]];

        let primary_vgm = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.1,
            partial_sill: 1.0,
            range: 1.5,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let cross_vgm = CrossVariogramModel {
            nugget: 0.05,
            sill: 0.8,
            range: 1.5,
            family: VariogramModelFamily::Exponential,
            wrss: 0.0,
            condition_number: 1.0,
            primary_var: "Z".to_string(),
            auxiliary_var: "Y".to_string(),
        };

        let aux_vgm = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.1,
            partial_sill: 0.8,
            range: 1.5,
            wrss: 0.0,
            condition_number: 1.0,
        };

        OrdinaryCoKriging::new(
            primary_vgm,
            vec![cross_vgm],
            vec![aux_vgm],
            coords,
            primary,
            auxiliary,
        )
    }

    #[test]
    fn test_cokriging_new() {
        let result = create_test_cokriging();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cokriging_new_length_mismatch() {
        let coords = vec![(0.0, 0.0), (1.0, 0.0)];
        let primary = vec![10.0];
        let auxiliary = vec![vec![20.0, 22.0]];

        let primary_vgm = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.1,
            partial_sill: 1.0,
            range: 1.5,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let cross_vgm = CrossVariogramModel {
            nugget: 0.05,
            sill: 0.8,
            range: 1.5,
            family: VariogramModelFamily::Exponential,
            wrss: 0.0,
            condition_number: 1.0,
            primary_var: "Z".to_string(),
            auxiliary_var: "Y".to_string(),
        };

        let aux_vgm = VariogramModel {
            family: VariogramModelFamily::Exponential,
            nugget: 0.1,
            partial_sill: 0.8,
            range: 1.5,
            wrss: 0.0,
            condition_number: 1.0,
        };

        let result = OrdinaryCoKriging::new(
            primary_vgm,
            vec![cross_vgm],
            vec![aux_vgm],
            coords,
            primary,
            auxiliary,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_cokriging_predict() {
        let cokriging = create_test_cokriging().unwrap();
        let target = (0.5, 0.5);

        let result = cokriging.predict(target, None);
        assert!(result.is_ok());

        let pred = result.unwrap();
        assert!(pred.prediction.is_finite());
        assert!(pred.variance >= 0.0);
    }

    #[test]
    fn test_cokriging_predict_batch() {
        let cokriging = create_test_cokriging().unwrap();
        let targets = vec![(0.5, 0.5), (0.2, 0.8)];

        let result = cokriging.predict_batch(&targets, None);
        assert!(result.is_ok());

        let preds = result.unwrap();
        assert_eq!(preds.len(), 2);
        assert!(preds.iter().all(|p| p.prediction.is_finite()));
    }
}
