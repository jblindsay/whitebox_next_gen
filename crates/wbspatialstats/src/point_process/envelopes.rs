//! Critical band envelope testing for point patterns
//!
//! Uses Monte Carlo simulation to generate confidence envelopes for hypothesis testing.
//! Tests whether observed K/L functions are consistent with complete spatial randomness (CSR).

use crate::GeostatError;
use serde::{Deserialize, Serialize};
use rand::Rng;
use rayon::prelude::*;

/// Envelope test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeResult {
    /// Distance values
    pub distances: Vec<f64>,
    /// Observed K values
    pub observed_k: Vec<f64>,
    /// Observed L values
    pub observed_l: Vec<f64>,
    /// Lower envelope (minimum from simulations)
    pub k_lower: Vec<f64>,
    /// Upper envelope (maximum from simulations)
    pub k_upper: Vec<f64>,
    /// L lower envelope
    pub l_lower: Vec<f64>,
    /// L upper envelope
    pub l_upper: Vec<f64>,
    /// Number of Monte Carlo simulations
    pub n_simulations: usize,
    /// Alpha level for critical band (e.g., 0.05)
    pub alpha: f64,
    /// Significance at each distance: true if observed is outside envelope
    pub is_significant: Vec<bool>,
}

/// Critical band envelope generator
pub struct CriticalBandEnvelope;

impl CriticalBandEnvelope {
    /// Generate critical band envelope for observed K function
    /// 
    /// # Arguments
    /// * `observed_k` - Observed K values at distance points
    /// * `distances` - Distance bins
    /// * `study_bounds` - Study area bounds (min_x, min_y, max_x, max_y)
    /// * `n_simulations` - Number of Monte Carlo simulations
    /// * `alpha` - Significance level (e.g., 0.05)
    /// 
    /// # Returns
    /// Envelope result with lower/upper bounds and significance flags
    pub fn generate(
        observed_k: &[f64],
        observed_l: &[f64],
        distances: &[f64],
        study_bounds: (f64, f64, f64, f64),
        n_simulations: usize,
        alpha: f64,
        intensity: f64,
    ) -> Result<EnvelopeResult, GeostatError> {
        if n_simulations < 10 {
            return Err(GeostatError::InvalidParameters(
                "need at least 10 simulations for envelope".to_string(),
            ));
        }

        let n_points = ((intensity * (study_bounds.2 - study_bounds.0) * (study_bounds.3 - study_bounds.1)) as usize).max(3);
        
        // Run Monte Carlo simulations in parallel
        let sim_results: Vec<Vec<f64>> = (0..n_simulations)
            .into_par_iter()
            .map(|_| {
                let csr_points = Self::generate_csr_points(n_points, study_bounds);
                Self::compute_k_for_points(&csr_points, study_bounds, distances)
            })
            .collect();

        // Extract envelopes as min/max across simulations
        let mut k_lower = vec![f64::INFINITY; distances.len()];
        let mut k_upper = vec![f64::NEG_INFINITY; distances.len()];

        for sim in &sim_results {
            for (i, &k) in sim.iter().enumerate() {
                k_lower[i] = k_lower[i].min(k);
                k_upper[i] = k_upper[i].max(k);
            }
        }

        // Convert to L values
        let l_lower: Vec<f64> = k_lower
            .iter()
            .map(|k| (k / std::f64::consts::PI).sqrt())
            .collect();

        let l_upper: Vec<f64> = k_upper
            .iter()
            .map(|k| (k / std::f64::consts::PI).sqrt())
            .collect();

        // Compute significance: observed is outside envelope
        let is_significant: Vec<bool> = observed_k
            .iter()
            .zip(0..distances.len())
            .map(|(obs_k, i)| obs_k < &k_lower[i] || obs_k > &k_upper[i])
            .collect();

        Ok(EnvelopeResult {
            distances: distances.to_vec(),
            observed_k: observed_k.to_vec(),
            observed_l: observed_l.to_vec(),
            k_lower,
            k_upper,
            l_lower,
            l_upper,
            n_simulations,
            alpha,
            is_significant,
        })
    }

    /// Generate complete spatial random (CSR) point pattern
    fn generate_csr_points(n: usize, bounds: (f64, f64, f64, f64)) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();
        (0..n)
            .map(|_| {
                (
                    rng.gen_range(bounds.0..=bounds.2),
                    rng.gen_range(bounds.1..=bounds.3),
                )
            })
            .collect()
    }

    /// Compute K function for arbitrary points
    fn compute_k_for_points(
        points: &[(f64, f64)],
        bounds: (f64, f64, f64, f64),
        distances: &[f64],
    ) -> Vec<f64> {
        let n = points.len() as f64;
        let area = (bounds.2 - bounds.0) * (bounds.3 - bounds.1);

        distances
            .iter()
            .map(|&t| {
                let mut count = 0.0;
                for i in 0..points.len() {
                    for j in 0..points.len() {
                        if i != j {
                            let dx = points[i].0 - points[j].0;
                            let dy = points[i].1 - points[j].1;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist <= t {
                                count += 1.0;
                            }
                        }
                    }
                }
                (area / (n * n)) * count
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_generation() {
        let observed_k = vec![0.05, 0.1, 0.15, 0.2];
        let observed_l = vec![-0.01, 0.02, 0.03, 0.04];
        let distances = vec![0.05, 0.1, 0.15, 0.2];
        let bounds = (0.0, 0.0, 1.0, 1.0);
        let intensity = 100.0;

        let result =
            CriticalBandEnvelope::generate(&observed_k, &observed_l, &distances, bounds, 50, 0.05, intensity)
                .unwrap();

        assert_eq!(result.distances.len(), 4);
        assert_eq!(result.k_lower.len(), 4);
        assert_eq!(result.k_upper.len(), 4);
        assert_eq!(result.is_significant.len(), 4);

        // Envelopes should be valid (lower < upper)
        for i in 0..4 {
            assert!(result.k_lower[i] <= result.k_upper[i]);
            assert!(result.l_lower[i] <= result.l_upper[i]);
        }
    }

    #[test]
    fn test_csr_generation() {
        let bounds = (0.0, 0.0, 1.0, 1.0);
        let points = CriticalBandEnvelope::generate_csr_points(50, bounds);

        assert_eq!(points.len(), 50);
        for (x, y) in points {
            assert!(x >= bounds.0 && x <= bounds.2);
            assert!(y >= bounds.1 && y <= bounds.3);
        }
    }

    #[test]
    fn test_envelope_invalid_simulations() {
        let result = CriticalBandEnvelope::generate(
            &vec![0.05],
            &vec![-0.01],
            &vec![0.05],
            (0.0, 0.0, 1.0, 1.0),
            5, // < 10, should error
            0.05,
            100.0,
        );

        assert!(result.is_err());
    }
}
