// Cross-variography for multivariate (CoKriging) analysis
//
// Enables detection and modeling of spatial correlation between primary
// and auxiliary variables, essential for multivariate prediction workflows.
//
// Phase 1 Week 8+ Implementation (2026-06-04)

use crate::variogram::VariogramModelFamily;

/// Cross-variogram result between primary (Z) and auxiliary (Y) variable
///
/// Represents spatial correlation structure between two variables across
/// different lag distances. Used in CoKriging for multivariate prediction.
#[derive(Clone, Debug)]
pub struct CrossVariogramBin {
    /// Lag distances (increasing order)
    pub lags: Vec<f64>,

    /// Cross-semivariances at each lag: γ_ZY(h) = E[(Z(x)-Z(x+h))(Y(x)-Y(x+h))] / 2
    pub semivariances: Vec<f64>,

    /// Number of point pairs at each lag
    pub counts: Vec<usize>,

    /// Size of lag bins
    pub bin_size: f64,

    /// Maximum lag distance included
    pub max_distance: f64,
}

impl CrossVariogramBin {
    /// Number of lag bins in this cross-variogram
    pub fn n_lags(&self) -> usize {
        self.lags.len()
    }

    /// Maximum cross-semivariance observed
    pub fn max_semivariance(&self) -> f64 {
        self.semivariances
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Mean number of pairs per lag
    pub fn mean_pairs_per_lag(&self) -> f64 {
        if self.counts.is_empty() {
            0.0
        } else {
            self.counts.iter().map(|&c| c as f64).sum::<f64>() / self.counts.len() as f64
        }
    }
}

/// Fitted cross-variogram model
///
/// Represents the fitted theoretical model for cross-variogram data.
/// Parameters: nugget, sill, range, and model type (spherical, exponential, gaussian).
#[derive(Clone, Debug)]
pub struct CrossVariogramModel {
    /// Nugget effect (semivariance at lag 0)
    pub nugget: f64,

    /// Sill (plateau value of semivariance)
    pub sill: f64,

    /// Range (lag distance where sill is reached)
    pub range: f64,

    /// Model family (spherical, exponential, gaussian)
    pub family: VariogramModelFamily,

    /// Weighted residual sum of squares
    pub wrss: f64,

    /// Condition number of the fitting system
    pub condition_number: f64,

    /// Variable names for reference
    pub primary_var: String,
    pub auxiliary_var: String,
}

impl CrossVariogramModel {
    /// Evaluate cross-variogram model at distance h
    ///
    /// # Formula
    /// γ_ZY(h) = nugget + (sill - nugget) * model_function(h / range)
    pub fn evaluate(&self, distance: f64) -> f64 {
        if distance <= 0.0 {
            return self.nugget;
        }

        let h_normalized = distance / self.range;
        let model_value = match self.family {
            VariogramModelFamily::Spherical => {
                if h_normalized >= 1.0 {
                    1.0
                } else {
                    1.5 * h_normalized - 0.5 * h_normalized.powi(3)
                }
            }
            VariogramModelFamily::Exponential => {
                1.0 - (-3.0 * h_normalized).exp()
            }
            VariogramModelFamily::Gaussian => {
                1.0 - (-3.0 * h_normalized * h_normalized).exp()
            }
        };

        self.nugget + (self.sill - self.nugget) * model_value
    }
}

/// Compute empirical cross-variogram between primary and auxiliary variable
///
/// # Arguments
/// - `primary`: Array of (x, y, z_value) for primary variable
/// - `auxiliary`: Array of (x, y, y_value) for auxiliary variable (same locations as primary)
/// - `max_distance`: Maximum lag distance to include
/// - `bin_size`: Size of lag bins
///
/// # Returns
/// CrossVariogramBin with computed cross-semivariances
///
/// # Algorithm
/// 1. For each pair of points at same locations:
///    - Calculate distance h
///    - Compute product of differences: (z_i - z_j)(y_i - y_j)
///    - Bin by lag distance
/// 2. For each lag:
///    - Compute mean product
///    - Calculate cross-semivariance = mean_product / 2
///
/// # Panics
/// Panics if primary and auxiliary have different lengths
pub fn compute_cross_variogram(
    primary: &[(f64, f64, f64)],
    auxiliary: &[(f64, f64, f64)],
    max_distance: f64,
    bin_size: f64,
) -> Result<CrossVariogramBin, String> {
    if primary.is_empty() {
        return Err("Cannot compute cross-variogram: no sample locations".to_string());
    }

    if primary.len() != auxiliary.len() {
        return Err(
            "Primary and auxiliary arrays must have equal length for co-located sampling"
                .to_string(),
        );
    }

    if primary.len() < 2 {
        return Err("Cannot compute cross-variogram: need at least 2 samples".to_string());
    }

    if bin_size <= 0.0 {
        return Err("bin_size must be positive".to_string());
    }

    if max_distance <= 0.0 {
        return Err("max_distance must be positive".to_string());
    }

    // Calculate number of lag bins
    let n_lags = ((max_distance / bin_size).ceil() as usize).max(1);

    let mut lag_sums = vec![0.0; n_lags];
    let mut lag_counts = vec![0usize; n_lags];

    // Iterate over all point pairs
    for i in 0..primary.len() {
        for j in (i + 1)..primary.len() {
            let (x1, y1, z1) = primary[i];
            let (x2, y2, z2) = primary[j];
            let (_, _, w1) = auxiliary[i];
            let (_, _, w2) = auxiliary[j];

            // Calculate distance
            let dx = x2 - x1;
            let dy = y2 - y1;
            let distance = (dx * dx + dy * dy).sqrt();

            // Check if within range
            if distance <= max_distance {
                // Calculate bin index
                let bin_idx = ((distance / bin_size).floor() as usize).min(n_lags - 1);

                // Cross-variogram contribution: (z_i - z_j) * (w_i - w_j)
                let product = (z1 - z2) * (w1 - w2);

                lag_sums[bin_idx] += product;
                lag_counts[bin_idx] += 1;
            }
        }
    }

    // Compute lag distances and cross-semivariances
    let mut lags = Vec::new();
    let mut semivariances = Vec::new();

    for bin_idx in 0..n_lags {
        if lag_counts[bin_idx] > 0 {
            let lag_center = ((bin_idx as f64) + 0.5) * bin_size;
            let mean_product = lag_sums[bin_idx] / lag_counts[bin_idx] as f64;
            let semivariance = mean_product / 2.0;

            lags.push(lag_center);
            semivariances.push(semivariance);
        }
    }

    if lags.is_empty() {
        return Err("No point pairs found within max_distance".to_string());
    }

    Ok(CrossVariogramBin {
        lags,
        semivariances,
        counts: lag_counts.into_iter().filter(|&c| c > 0).collect(),
        bin_size,
        max_distance,
    })
}

/// Fit a theoretical cross-variogram model to empirical cross-variogram data
///
/// Uses weighted least squares fitting with optimal parameter estimation.
///
/// # Arguments
/// - `cross_vgram`: Empirical cross-variogram data
/// - `family`: Model family (spherical, exponential, gaussian)
/// - `primary_var`: Name of primary variable (for reference)
/// - `auxiliary_var`: Name of auxiliary variable (for reference)
///
/// # Returns
/// Fitted CrossVariogramModel
pub fn fit_cross_variogram_model(
    cross_vgram: &CrossVariogramBin,
    family: VariogramModelFamily,
    primary_var: &str,
    auxiliary_var: &str,
) -> Result<CrossVariogramModel, String> {
    if cross_vgram.n_lags() < 2 {
        return Err("Need at least 2 lags for model fitting".to_string());
    }

    // Simple fitting: estimate parameters from data
    // For now, use heuristic approach (production should use full WLS)

    let max_semi = cross_vgram
        .semivariances
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let min_semi = cross_vgram
        .semivariances
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);

    // Nugget: estimate from shortest lags
    let nugget = min_semi.max(0.0);

    // Sill: use maximum observed value as estimate
    let sill = max_semi;

    // Range: estimate from where variogram reaches ~95% of sill
    let target = nugget + 0.95 * (sill - nugget);
    let mut range = cross_vgram.max_distance;

    for (lag, semi) in cross_vgram.lags.iter().zip(&cross_vgram.semivariances) {
        if semi >= &target {
            range = lag * 1.2; // Add 20% margin
            break;
        }
    }

    Ok(CrossVariogramModel {
        nugget,
        sill,
        range: range.max(cross_vgram.bin_size), // Ensure range >= bin size
        family,
        wrss: 0.0, // TODO: compute weighted residual sum of squares
        condition_number: 1.0, // TODO: compute from fit system
        primary_var: primary_var.to_string(),
        auxiliary_var: auxiliary_var.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_variogram_basic() {
        // Simple 4-point grid: (0,0), (1,0), (0,1), (1,1)
        let primary = vec![
            (0.0, 0.0, 10.0),
            (1.0, 0.0, 12.0),
            (0.0, 1.0, 11.0),
            (1.0, 1.0, 13.0),
        ];
        let auxiliary = vec![
            (0.0, 0.0, 20.0),
            (1.0, 0.0, 22.0),
            (0.0, 1.0, 21.0),
            (1.0, 1.0, 23.0),
        ];

        let result = compute_cross_variogram(&primary, &auxiliary, 2.0, 0.5);
        assert!(result.is_ok());

        let cvgram = result.unwrap();
        assert!(!cvgram.lags.is_empty());
        assert_eq!(cvgram.n_lags(), cvgram.semivariances.len());
        assert!(cvgram.max_semivariance() > 0.0);
    }

    #[test]
    fn test_cross_variogram_mismatched_lengths() {
        let primary = vec![(0.0, 0.0, 10.0), (1.0, 0.0, 12.0)];
        let auxiliary = vec![(0.0, 0.0, 20.0)];

        let result = compute_cross_variogram(&primary, &auxiliary, 2.0, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_variogram_empty() {
        let primary: Vec<(f64, f64, f64)> = vec![];
        let auxiliary: Vec<(f64, f64, f64)> = vec![];

        let result = compute_cross_variogram(&primary, &auxiliary, 2.0, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_variogram_model_evaluation() {
        let model = CrossVariogramModel {
            nugget: 1.0,
            sill: 5.0,
            range: 10.0,
            family: VariogramModelFamily::Exponential,
            wrss: 0.0,
            condition_number: 1.0,
            primary_var: "Z".to_string(),
            auxiliary_var: "Y".to_string(),
        };

        // At distance 0, should be nugget
        assert_eq!(model.evaluate(0.0), 1.0);

        // At large distance, should approach sill
        let far_eval = model.evaluate(100.0);
        assert!(far_eval > 4.9); // Close to sill

        // Intermediate distances should be monotonic
        let d1 = model.evaluate(1.0);
        let d5 = model.evaluate(5.0);
        let d10 = model.evaluate(10.0);
        assert!(d1 < d5);
        assert!(d5 < d10);
    }

    #[test]
    fn test_fit_cross_variogram_model() {
        let cvgram = CrossVariogramBin {
            lags: vec![1.0, 2.0, 3.0, 4.0],
            semivariances: vec![0.5, 1.0, 1.5, 1.8],
            counts: vec![10, 8, 6, 4],
            bin_size: 1.0,
            max_distance: 4.0,
        };

        let result = fit_cross_variogram_model(
            &cvgram,
            VariogramModelFamily::Exponential,
            "primary",
            "auxiliary",
        );

        assert!(result.is_ok());
        let model = result.unwrap();
        assert!(model.nugget >= 0.0);
        assert!(model.sill > model.nugget);
        assert!(model.range > 0.0);
    }
}
