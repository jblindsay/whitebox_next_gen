// Prediction intervals and uncertainty quantification for kriging
//
// Enables confidence/prediction intervals on kriging predictions,
// essential for decision-making and risk assessment.
//
// Phase 1 Week 3 Implementation (2026-06-04)

/// Prediction interval result
#[derive(Clone, Debug)]
pub struct PredictionInterval {
    /// Lower bound of the interval
    pub lower: f64,

    /// Point estimate (mean/median)
    pub point_estimate: f64,

    /// Upper bound of the interval
    pub upper: f64,

    /// Confidence level (e.g., 0.90 for 90% CI)
    pub confidence: f64,

    /// Method used ("gaussian", "bootstrap", "posterior", etc.)
    pub method: String,

    /// Half-width of interval (for convenience)
    pub margin_of_error: f64,
}

impl PredictionInterval {
    /// Width of the interval (upper - lower)
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }

    /// Check if a value falls within the interval
    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower && value <= self.upper
    }

    /// Normalized standard error (margin_of_error relative to point estimate)
    pub fn normalized_margin(&self) -> f64 {
        if self.point_estimate.abs() > 1e-10 {
            self.margin_of_error / self.point_estimate.abs()
        } else {
            f64::INFINITY
        }
    }
}

/// Compute z-score (critical value) for normal distribution using lookup table
///
/// # Arguments
/// - `confidence`: Confidence level (e.g., 0.95 for 95% CI)
///
/// # Returns
/// Standard normal quantile (e.g., 1.96 for 95% CI)
fn standard_normal_quantile(confidence: f64) -> f64 {
    // Pre-computed quantiles for common confidence levels
    // These are: P(Z <= z) = (1 + confidence) / 2
    match (confidence * 1000.0).round() as i32 {
        800 => 1.2816, // 80%
        850 => 1.4395, // 85%
        900 => 1.6449, // 90%
        950 => 1.9600, // 95%
        990 => 2.5758, // 99%
        _ => {
            // For other values, use approximation
            // Based on error function approximation
            let p = (1.0 + confidence) / 2.0;
            // Rational approximation of inverse normal CDF
            let t = (-2.0 * (1.0 - p).ln()).sqrt();
            let num = 2.515517 + 0.802853 * t + 0.010328 * t * t;
            let denom = 1.0 + 1.432788 * t + 0.189269 * t * t + 0.001308 * t * t * t;
            if p > 0.5 {
                num / denom
            } else {
                -num / denom
            }
        }
    }
}

/// Compute Gaussian prediction interval
///
/// Assumes the kriging prediction follows a normal distribution with the
/// kriging variance.
///
/// # Arguments
/// - `prediction`: Kriged value (mean)
/// - `kriging_variance`: Kriging variance from solver
/// - `confidence`: Confidence level in [0.5, 1.0) (e.g., 0.95 for 95% CI)
///
/// # Returns
/// PredictionInterval with lower/upper bounds
///
/// # Formula
/// Given prediction ~ N(μ, σ²_kriging):
/// - z = standard_normal_quantile(confidence)
/// - margin = z * σ_kriging
/// - CI = [μ - margin, μ + margin]
pub fn kriging_prediction_interval_gaussian(
    prediction: f64,
    kriging_variance: f64,
    confidence: f64,
) -> Result<PredictionInterval, String> {
    if !confidence.is_finite() || confidence <= 0.5 || confidence >= 1.0 {
        return Err("Confidence level must be in (0.5, 1.0)".to_string());
    }

    if !kriging_variance.is_finite() || kriging_variance < 0.0 {
        return Err("Kriging variance must be non-negative and finite".to_string());
    }

    if !prediction.is_finite() {
        return Err("Prediction must be finite".to_string());
    }

    let z_critical = standard_normal_quantile(confidence);
    let std_error = kriging_variance.sqrt();
    let margin = z_critical * std_error;

    Ok(PredictionInterval {
        lower: prediction - margin,
        point_estimate: prediction,
        upper: prediction + margin,
        confidence,
        method: "gaussian".to_string(),
        margin_of_error: margin,
    })
}

/// Compute posterior prediction interval
///
/// Incorporates measurement uncertainty (e.g., from CV residuals)
/// into the interval calculation.
///
/// # Arguments
/// - `prediction`: Kriged value
/// - `kriging_variance`: Kriging variance from solver
/// - `residual_std`: Standard deviation of CV residuals
/// - `confidence`: Confidence level
///
/// # Returns
/// PredictionInterval accounting for both kriging and measurement uncertainty
///
/// # Formula
/// Total variance = kriging_variance + residual_std²
/// Then apply same quantile-based calculation as Gaussian interval
pub fn kriging_prediction_interval_posterior(
    prediction: f64,
    kriging_variance: f64,
    residual_std: f64,
    confidence: f64,
) -> Result<PredictionInterval, String> {
    if residual_std < 0.0 {
        return Err("Residual standard deviation must be non-negative".to_string());
    }

    // Total variance includes both kriging and measurement uncertainty
    let total_variance = kriging_variance + residual_std * residual_std;

    // Use Gaussian interval with total variance
    let mut interval = kriging_prediction_interval_gaussian(prediction, total_variance, confidence)?;
    interval.method = "posterior".to_string();

    Ok(interval)
}

/// Calibration diagnostics for prediction intervals
///
/// Assesses whether prediction intervals have correct coverage
/// (e.g., do 95% of test points fall within 95% CI?)
#[derive(Clone, Debug)]
pub struct IntervalCalibration {
    /// Observed coverage: fraction of test points within intervals
    pub observed_coverage: f64,

    /// Expected coverage (target)
    pub expected_coverage: f64,

    /// Coverage deficit (expected - observed)
    pub coverage_deficit: f64,

    /// Mean interval width
    pub mean_interval_width: f64,

    /// Intervals calibrated? (coverage within ±0.05 of expected)
    pub is_calibrated: bool,
}

/// Compute interval calibration from test data
///
/// # Arguments
/// - `predictions`: Kriged predictions
/// - `intervals`: Corresponding prediction intervals
/// - `observations`: Held-out test observations
///
/// # Returns
/// Calibration diagnostics
pub fn assess_interval_calibration(
    predictions: &[f64],
    intervals: &[PredictionInterval],
    observations: &[f64],
) -> Result<IntervalCalibration, String> {
    if predictions.len() != intervals.len() || predictions.len() != observations.len() {
        return Err("Predictions, intervals, and observations must have equal length".to_string());
    }

    if predictions.is_empty() {
        return Err("No data for calibration assessment".to_string());
    }

    // Count how many observations fall within their intervals
    let mut covered = 0usize;
    let mut total_width = 0.0;

    for (i, obs) in observations.iter().enumerate() {
        if intervals[i].contains(*obs) {
            covered += 1;
        }
        total_width += intervals[i].width();
    }

    let observed_coverage = covered as f64 / predictions.len() as f64;
    let expected_coverage = intervals[0].confidence; // Assume all intervals have same confidence
    let coverage_deficit = (expected_coverage - observed_coverage).abs();
    let mean_interval_width = total_width / predictions.len() as f64;

    // Consider calibrated if coverage within ±0.05 of expected
    let is_calibrated = coverage_deficit <= 0.05;

    Ok(IntervalCalibration {
        observed_coverage,
        expected_coverage,
        coverage_deficit,
        mean_interval_width,
        is_calibrated,
    })
}


/// Compute bootstrap prediction interval using residual resampling
///
/// Constructs empirical confidence intervals by resampling residuals from
/// cross-validation. This method is model-free and works with any kriging model.
///
/// # Algorithm
/// 1. Take residuals from cross-validation: obs - pred for each training point
/// 2. Center residuals (subtract mean to preserve symmetry)
/// 3. Resample residuals with replacement (bootstrap samples)
/// 4. Add bootstrap samples to kriging prediction
/// 5. Compute empirical quantiles at (1-conf)/2 and (1+conf)/2
///
/// # Arguments
/// - `prediction`: Kriged prediction at target location
/// - `residuals`: CV residuals from training set (observed - predicted)
/// - `confidence`: Confidence level in [0.5, 1.0) (e.g., 0.95 for 95% CI)
/// - `n_bootstrap`: Number of bootstrap resamples (typically 999-9999)
/// - `seed`: Optional seed for reproducibility (None = random)
///
/// # Returns
/// PredictionInterval with empirical bounds
///
/// # Notes
/// - Assumes residuals are IID (independent, identically distributed)
/// - Works best with ≥30 residuals for reliable quantiles
/// - More computationally expensive than Gaussian method
/// - More robust to non-normal residual distributions
pub fn kriging_prediction_interval_bootstrap(
    prediction: f64,
    residuals: &[f64],
    confidence: f64,
    n_bootstrap: usize,
    seed: Option<u64>,
) -> Result<PredictionInterval, String> {
    if !confidence.is_finite() || confidence <= 0.5 || confidence >= 1.0 {
        return Err("Confidence level must be in (0.5, 1.0)".to_string());
    }

    if !prediction.is_finite() {
        return Err("Prediction must be finite".to_string());
    }

    if residuals.is_empty() {
        return Err("Residuals cannot be empty for bootstrap intervals".to_string());
    }

    if n_bootstrap < 100 {
        return Err("n_bootstrap should be ≥100 for reliable bootstrap intervals".to_string());
    }

    // Check all residuals are finite
    if residuals.iter().any(|r| !r.is_finite()) {
        return Err("All residuals must be finite".to_string());
    }

    // Center residuals (subtract mean) to preserve symmetry
    let mean_residual = residuals.iter().sum::<f64>() / residuals.len() as f64;
    let centered_residuals: Vec<f64> = residuals.iter().map(|r| r - mean_residual).collect();

    // Initialize random number generator
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let seed_val = seed.unwrap_or_else(|| {
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        hasher.finish()
    });

    // Simple LCG pseudo-random generator for consistency across platforms
    let mut rng_state = seed_val as u64;

    let lcg_next = |state: &mut u64| -> f64 {
        const A: u64 = 6364136223846793005;
        const C: u64 = 1442695040888963407;
        *state = A.wrapping_mul(*state).wrapping_add(C);
        (*state >> 11) as f64 / (1u64 << 53) as f64
    };

    // Generate bootstrap samples of predictions
    let mut bootstrap_predictions = Vec::with_capacity(n_bootstrap);

    for _ in 0..n_bootstrap {
        // Resample one residual uniformly at random
        let idx = ((lcg_next(&mut rng_state) * centered_residuals.len() as f64).floor() as usize)
            .min(centered_residuals.len() - 1);
        let bootstrap_residual = centered_residuals[idx];
        bootstrap_predictions.push(prediction + bootstrap_residual);
    }

    // Sort for quantile computation
    bootstrap_predictions.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Compute empirical quantiles
    let lower_idx = (((1.0 - confidence) / 2.0) * n_bootstrap as f64).floor() as usize;
    let upper_idx = (((1.0 + confidence) / 2.0) * n_bootstrap as f64).ceil() as usize;

    let lower_idx = lower_idx.min(bootstrap_predictions.len() - 1);
    let upper_idx = upper_idx.min(bootstrap_predictions.len() - 1);

    let lower = bootstrap_predictions[lower_idx];
    let upper = bootstrap_predictions[upper_idx];
    let margin_of_error = (upper - lower) / 2.0;

    Ok(PredictionInterval {
        lower,
        point_estimate: prediction,
        upper,
        confidence,
        method: "bootstrap".to_string(),
        margin_of_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_interval_95_ci() {
        let result = kriging_prediction_interval_gaussian(100.0, 4.0, 0.95);
        assert!(result.is_ok());

        let interval = result.unwrap();
        assert_eq!(interval.point_estimate, 100.0);
        assert!(interval.lower < 100.0);
        assert!(interval.upper > 100.0);
        // For 95% CI with σ=2, margin ≈ 1.96*2 ≈ 3.92
        assert!((interval.margin_of_error - 3.92).abs() < 0.1);
        assert_eq!(interval.confidence, 0.95);
        assert_eq!(interval.method, "gaussian");
    }

    #[test]
    fn test_gaussian_interval_90_ci() {
        let result = kriging_prediction_interval_gaussian(50.0, 1.0, 0.90);
        assert!(result.is_ok());

        let interval = result.unwrap();
        assert_eq!(interval.point_estimate, 50.0);
        // 90% CI should be narrower than 95%
        assert!(interval.width() < 5.0); // Rough estimate
    }

    #[test]
    fn test_gaussian_interval_zero_variance() {
        let result = kriging_prediction_interval_gaussian(100.0, 0.0, 0.95);
        assert!(result.is_ok());

        let interval = result.unwrap();
        // Zero variance → zero margin
        assert_eq!(interval.margin_of_error, 0.0);
        assert_eq!(interval.lower, 100.0);
        assert_eq!(interval.upper, 100.0);
    }

    #[test]
    fn test_gaussian_interval_invalid_confidence() {
        // Confidence too low
        let result = kriging_prediction_interval_gaussian(100.0, 4.0, 0.4);
        assert!(result.is_err());

        // Confidence too high
        let result = kriging_prediction_interval_gaussian(100.0, 4.0, 1.0);
        assert!(result.is_err());

        // Confidence exactly 0.5
        let result = kriging_prediction_interval_gaussian(100.0, 4.0, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_gaussian_interval_invalid_variance() {
        let result = kriging_prediction_interval_gaussian(100.0, -1.0, 0.95);
        assert!(result.is_err());
    }

    #[test]
    fn test_posterior_interval_with_measurement_uncertainty() {
        // Kriging variance = 4, residual std = 1
        let result = kriging_prediction_interval_posterior(100.0, 4.0, 1.0, 0.95);
        assert!(result.is_ok());

        let interval = result.unwrap();
        assert_eq!(interval.point_estimate, 100.0);
        assert_eq!(interval.method, "posterior");

        // Total variance = 4 + 1² = 5, so σ ≈ 2.24
        // Margin ≈ 1.96 * 2.24 ≈ 4.39
        assert!((interval.margin_of_error - 4.39).abs() < 0.2);
    }

    #[test]
    fn test_interval_contains() {
        let interval = PredictionInterval {
            lower: 90.0,
            point_estimate: 100.0,
            upper: 110.0,
            confidence: 0.95,
            method: "gaussian".to_string(),
            margin_of_error: 10.0,
        };

        assert!(interval.contains(95.0));
        assert!(interval.contains(100.0));
        assert!(interval.contains(105.0));
        assert!(!interval.contains(85.0));
        assert!(!interval.contains(115.0));
    }

    #[test]
    fn test_interval_width() {
        let interval = PredictionInterval {
            lower: 90.0,
            point_estimate: 100.0,
            upper: 110.0,
            confidence: 0.95,
            method: "gaussian".to_string(),
            margin_of_error: 10.0,
        };

        assert_eq!(interval.width(), 20.0);
    }

    #[test]
    fn test_calibration_perfect() {
        let predictions = vec![100.0, 150.0, 200.0, 250.0];
        let intervals = vec![
            PredictionInterval {
                lower: 90.0,
                point_estimate: 100.0,
                upper: 110.0,
                confidence: 0.90,
                method: "gaussian".to_string(),
                margin_of_error: 10.0,
            },
            PredictionInterval {
                lower: 140.0,
                point_estimate: 150.0,
                upper: 160.0,
                confidence: 0.90,
                method: "gaussian".to_string(),
                margin_of_error: 10.0,
            },
            PredictionInterval {
                lower: 190.0,
                point_estimate: 200.0,
                upper: 210.0,
                confidence: 0.90,
                method: "gaussian".to_string(),
                margin_of_error: 10.0,
            },
            PredictionInterval {
                lower: 240.0,
                point_estimate: 250.0,
                upper: 260.0,
                confidence: 0.90,
                method: "gaussian".to_string(),
                margin_of_error: 10.0,
            },
        ];
        let observations = vec![100.0, 150.0, 200.0, 250.0]; // All within intervals

        let result = assess_interval_calibration(&predictions, &intervals, &observations);
        assert!(result.is_ok());

        let calib = result.unwrap();
        assert_eq!(calib.observed_coverage, 1.0); // Perfect coverage
        assert_eq!(calib.expected_coverage, 0.90);
        // Coverage deficit = |0.90 - 1.0| = 0.10, which is > 0.05, so not calibrated
        // But that's okay - the test validates the mechanics work correctly
        assert!((calib.coverage_deficit - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_calibration_poor() {
        let predictions = vec![100.0, 150.0, 200.0];
        let intervals = vec![
            PredictionInterval {
                lower: 99.0,
                point_estimate: 100.0,
                upper: 101.0,
                confidence: 0.95,
                method: "gaussian".to_string(),
                margin_of_error: 1.0,
            },
            PredictionInterval {
                lower: 149.0,
                point_estimate: 150.0,
                upper: 151.0,
                confidence: 0.95,
                method: "gaussian".to_string(),
                margin_of_error: 1.0,
            },
            PredictionInterval {
                lower: 199.0,
                point_estimate: 200.0,
                upper: 201.0,
                confidence: 0.95,
                method: "gaussian".to_string(),
                margin_of_error: 1.0,
            },
        ];
        let observations = vec![110.0, 160.0, 210.0]; // None within intervals (test residuals too large)

        let result = assess_interval_calibration(&predictions, &intervals, &observations);
        assert!(result.is_ok());

        let calib = result.unwrap();
        assert_eq!(calib.observed_coverage, 0.0);
        assert!(!calib.is_calibrated); // Not calibrated
    }

    #[test]
    fn test_calibration_length_mismatch() {
        let predictions = vec![100.0, 150.0];
        let intervals = vec![
            PredictionInterval {
                lower: 95.0,
                point_estimate: 100.0,
                upper: 105.0,
                confidence: 0.95,
                method: "gaussian".to_string(),
                margin_of_error: 5.0,
            },
        ];
        let observations = vec![100.0, 150.0];

        let result = assess_interval_calibration(&predictions, &intervals, &observations);
        assert!(result.is_err());
    }

    #[test]
    fn test_bootstrap_interval_basic() {
        let prediction = 100.0;
        let residuals = vec![-5.0, -3.0, -1.0, 0.0, 1.0, 3.0, 5.0, 2.0, -2.0, 4.0];
        let result = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.90, 999, Some(42));

        assert!(result.is_ok());
        let interval = result.unwrap();
        assert_eq!(interval.point_estimate, 100.0);
        assert!(interval.lower < 100.0);
        assert!(interval.upper > 100.0);
        assert_eq!(interval.confidence, 0.90);
        assert_eq!(interval.method, "bootstrap");
        // For residuals ~[-5, 5], 90% CI should be roughly [95, 105]
        assert!(interval.lower >= 94.0 && interval.lower <= 96.0);
        assert!(interval.upper >= 104.0 && interval.upper <= 106.0);
    }

    #[test]
    fn test_bootstrap_interval_95_ci() {
        let prediction = 50.0;
        let residuals = vec![-10.0, -5.0, 0.0, 5.0, 10.0];
        let result = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.95, 999, Some(123));

        assert!(result.is_ok());
        let interval = result.unwrap();
        assert_eq!(interval.confidence, 0.95);
        // 95% CI should be wider than 90% CI
        assert!(interval.width() > 15.0);
    }

    #[test]
    fn test_bootstrap_interval_reproducibility() {
        let prediction = 100.0;
        let residuals = vec![-3.0, -1.0, 0.0, 1.0, 3.0];

        let result1 = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.90, 999, Some(42));
        let result2 = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.90, 999, Some(42));

        assert!(result1.is_ok() && result2.is_ok());
        let int1 = result1.unwrap();
        let int2 = result2.unwrap();
        // Same seed should give identical results
        assert_eq!(int1.lower, int2.lower);
        assert_eq!(int1.upper, int2.upper);
    }

    #[test]
    fn test_bootstrap_interval_invalid_confidence() {
        let prediction = 100.0;
        let residuals = vec![-5.0, 0.0, 5.0];

        let result = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.4, 999, Some(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_bootstrap_interval_empty_residuals() {
        let prediction = 100.0;
        let residuals: Vec<f64> = vec![];

        let result = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.90, 999, Some(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_bootstrap_interval_insufficient_bootstrap_samples() {
        let prediction = 100.0;
        let residuals = vec![-5.0, 0.0, 5.0];

        let result = kriging_prediction_interval_bootstrap(prediction, &residuals, 0.90, 50, Some(42));
        assert!(result.is_err());
    }
}
