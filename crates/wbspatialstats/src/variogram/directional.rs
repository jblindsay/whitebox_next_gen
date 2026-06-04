// Directional variography for anisotropic spatial analysis
//
// Enables detection and modeling of spatially-directional dependence,
// essential for geological, geomorphological, and linear feature analysis.
//
// Phase 1 Week 2 Implementation (2026-06-04)

use std::f64::consts::PI;

/// Result of directional variogram computation for a single azimuth
#[derive(Clone, Debug)]
pub struct DirectionalVariogramBin {
    /// Azimuth direction in degrees (0-180°, bidirectional)
    pub direction_azimuth: f64,

    /// Tolerance around azimuth in degrees (e.g., ±22.5°)
    pub tolerance: f64,

    /// Lag distances (increasing order)
    pub lags: Vec<f64>,

    /// Semivariance at each lag
    pub semivariances: Vec<f64>,

    /// Number of point pairs at each lag
    pub counts: Vec<usize>,

    /// Size of lag bins
    pub bin_size: f64,

    /// Sill (plateau value, for reference)
    pub sill: Option<f64>,

    /// Nugget effect (at lag 0)
    pub nugget: Option<f64>,
}

impl DirectionalVariogramBin {
    /// Returns the direction tolerance band as (min_azimuth, max_azimuth)
    pub fn azimuth_range(&self) -> (f64, f64) {
        let min = (self.direction_azimuth - self.tolerance) % 180.0;
        let max = (self.direction_azimuth + self.tolerance) % 180.0;
        (min, max)
    }

    /// Number of lag bins in this directional variogram
    pub fn n_lags(&self) -> usize {
        self.lags.len()
    }

    /// Maximum semivariance observed
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

/// Anisotropy model describing directional variation in spatial continuity
#[derive(Clone, Debug)]
pub struct AnisotropyModel {
    /// Major range (maximum spatial continuity direction)
    pub major_range: f64,

    /// Minor range (perpendicular to major direction)
    pub minor_range: f64,

    /// Azimuth of maximum continuity (0-180°)
    pub major_azimuth: f64,

    /// Anisotropy ratio (minor_range / major_range, 0 < ratio ≤ 1)
    pub ratio: f64,

    /// Angle tolerance for direction binning
    pub angle_tolerance: f64,

    /// Method used to fit anisotropy ("max_min", "robust", etc.)
    pub method: String,
}

impl AnisotropyModel {
    /// Check if anisotropy is significant (ratio well below 1.0)
    pub fn is_anisotropic(&self, threshold: f64) -> bool {
        self.ratio < threshold
    }

    /// Anisotropic distance metric for kriging
    ///
    /// Transforms Euclidean distance to account for directional variation.
    /// Using the standard anisotropy transformation:
    /// 1. Rotate coordinates by major_azimuth
    /// 2. Scale coordinates: y' *= 1/ratio (along minor range, to stretch it)
    /// 3. Compute Euclidean distance in transformed space
    pub fn anisotropic_distance(&self, dx: f64, dy: f64) -> f64 {
        let az_rad = self.major_azimuth * PI / 180.0;
        let cos_az = az_rad.cos();
        let sin_az = az_rad.sin();

        // Rotate coordinates to align with major axis
        let dx_rot = dx * cos_az + dy * sin_az;
        let dy_rot = -dx * sin_az + dy * cos_az;

        // Scale perpendicular direction: divide by ratio to stretch minor direction
        let dx_scaled = dx_rot;
        let dy_scaled = dy_rot / self.ratio;

        (dx_scaled * dx_scaled + dy_scaled * dy_scaled).sqrt()
    }
}

/// Compute directional variogram for a specific azimuth
///
/// # Arguments
/// - `sample_locations`: Array of (x, y, value) tuples
/// - `direction_azimuth`: Target azimuth in degrees (0-180°)
/// - `tolerance`: Tolerance around azimuth (e.g., ±22.5°)
/// - `max_distance`: Maximum lag distance to include
/// - `bin_size`: Size of lag bins
///
/// # Algorithm
/// 1. For each point pair:
///    - Calculate distance and azimuth
///    - Check if azimuth falls within tolerance band
///    - Bin by lag distance
/// 2. For each lag:
///    - Compute mean squared difference
///    - Calculate semivariance = mean / 2
pub fn compute_directional_variogram(
    sample_locations: &[(f64, f64, f64)],
    direction_azimuth: f64,
    tolerance: f64,
    max_distance: f64,
    bin_size: f64,
) -> Result<DirectionalVariogramBin, String> {
    if sample_locations.is_empty() {
        return Err("Cannot compute variogram: no sample locations".to_string());
    }

    if sample_locations.len() < 2 {
        return Err("Cannot compute variogram: need at least 2 samples".to_string());
    }

    if bin_size <= 0.0 {
        return Err("bin_size must be positive".to_string());
    }

    if max_distance <= 0.0 {
        return Err("max_distance must be positive".to_string());
    }

    if tolerance < 0.0 || tolerance > 90.0 {
        return Err("tolerance must be in [0, 90]".to_string());
    }

    // Normalize azimuth to [0, 180)
    let az_norm = direction_azimuth % 180.0;

    // Calculate number of lag bins
    let n_lags = ((max_distance / bin_size).ceil() as usize).max(1);

    let mut lag_sums = vec![0.0; n_lags];
    let mut lag_counts = vec![0usize; n_lags];

    // Iterate over all point pairs
    for i in 0..sample_locations.len() {
        for j in (i + 1)..sample_locations.len() {
            let (x1, y1, z1) = sample_locations[i];
            let (x2, y2, z2) = sample_locations[j];

            let dx = x2 - x1;
            let dy = y2 - y1;

            let distance = (dx * dx + dy * dy).sqrt();

            // Skip if distance exceeds max
            if distance > max_distance {
                continue;
            }

            // Calculate azimuth (0-180° convention, bidirectional)
            let azimuth_rad = dy.atan2(dx);
            let mut azimuth = (azimuth_rad * 180.0 / PI) % 180.0;
            if azimuth < 0.0 {
                azimuth += 180.0;
            }

            // Check if azimuth is within tolerance band
            let az_diff = (azimuth - az_norm).abs();
            let az_diff_normalized = if az_diff > 90.0 {
                180.0 - az_diff
            } else {
                az_diff
            };

            if az_diff_normalized > tolerance {
                continue;
            }

            // Bin by lag distance
            let lag_idx = ((distance / bin_size).floor() as usize).min(n_lags - 1);

            // Add squared difference
            let dz = z2 - z1;
            lag_sums[lag_idx] += dz * dz;
            lag_counts[lag_idx] += 1;
        }
    }

    // Calculate semivariances and lags
    let mut lags = vec![];
    let mut semivariances = vec![];
    let mut counts = vec![];

    for lag_idx in 0..n_lags {
        if lag_counts[lag_idx] > 0 {
            let semivar = lag_sums[lag_idx] / (2.0 * lag_counts[lag_idx] as f64);
            let lag_distance = (lag_idx as f64 + 0.5) * bin_size; // Center of lag bin

            lags.push(lag_distance);
            semivariances.push(semivar);
            counts.push(lag_counts[lag_idx]);
        }
    }

    if lags.is_empty() {
        return Err("No valid point pairs found in azimuth tolerance band".to_string());
    }

    // Estimate sill (mean of top 25% semivariances) and nugget (first lag value)
    let mut sorted_semivars = semivariances.clone();
    sorted_semivars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let top_25_idx = (sorted_semivars.len() * 3 / 4).max(0);
    let sill = sorted_semivars[top_25_idx..].iter().sum::<f64>() / sorted_semivars[top_25_idx..].len().max(1) as f64;

    let nugget = Some(semivariances[0]);

    Ok(DirectionalVariogramBin {
        direction_azimuth: az_norm,
        tolerance,
        lags,
        semivariances,
        counts,
        bin_size,
        sill: Some(sill),
        nugget,
    })
}

/// Fit anisotropy model from directional variograms
///
/// # Arguments
/// - `directional_vgrams`: Array of directional variograms at different azimuths
///
/// # Algorithm
/// 1. For each directional variogram, identify the maximum semivariance lag (practical range)
/// 2. Use the lag with the highest semivariance as proxy for range
/// 3. Find direction with maximum range (major_azimuth)
/// 4. Calculate anisotropy ratio from major and minor ranges
pub fn fit_anisotropy(
    directional_vgrams: &[DirectionalVariogramBin],
) -> Result<AnisotropyModel, String> {
    if directional_vgrams.is_empty() {
        return Err("Cannot fit anisotropy: no directional variograms provided".to_string());
    }

    if directional_vgrams.len() < 2 {
        return Err("Need at least 2 directional variograms to fit anisotropy".to_string());
    }

    // Estimate range for each direction (using lag at maximum semivariance)
    let mut ranges = vec![];
    for vgram in directional_vgrams {
        // Find lag with maximum semivariance as practical range estimate
        let max_idx = vgram
            .semivariances
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        if max_idx < vgram.lags.len() {
            ranges.push((vgram.direction_azimuth, vgram.lags[max_idx]));
        }
    }

    if ranges.is_empty() {
        return Err("Could not estimate ranges from directional variograms".to_string());
    }

    // Sort by range to find major and minor
    ranges.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let major_azimuth = ranges[0].0;
    let major_range = ranges[0].1;
    let minor_range = ranges.last().map(|(_, r)| r).copied().unwrap_or(major_range * 0.7);

    let ratio = (minor_range / major_range).max(0.01).min(1.0);

    let angle_tolerance = directional_vgrams[0].tolerance; // Use first vgram's tolerance

    Ok(AnisotropyModel {
        major_range,
        minor_range,
        major_azimuth,
        ratio,
        angle_tolerance,
        method: "max_min".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directional_variogram_east_west() {
        // Create data with strong E-W trend
        let samples = vec![
            (0.0, 0.0, 1.0),
            (1.0, 0.0, 2.0),
            (2.0, 0.0, 3.0),
            (3.0, 0.0, 4.0),
            // Perpendicular points (should have less correlation)
            (0.0, 1.0, 1.5),
            (0.0, 2.0, 2.5),
        ];

        let result = compute_directional_variogram(&samples, 0.0, 22.5, 5.0, 1.0);
        assert!(result.is_ok());

        let vgram = result.unwrap();
        assert!(!vgram.lags.is_empty());
        assert_eq!(vgram.direction_azimuth, 0.0);
        assert_eq!(vgram.tolerance, 22.5);
    }

    #[test]
    fn test_directional_variogram_north_south() {
        // Create data with strong N-S trend
        let samples = vec![
            (0.0, 0.0, 1.0),
            (0.0, 1.0, 2.0),
            (0.0, 2.0, 3.0),
            (0.0, 3.0, 4.0),
        ];

        let result = compute_directional_variogram(&samples, 90.0, 22.5, 5.0, 1.0);
        assert!(result.is_ok());

        let vgram = result.unwrap();
        assert!(!vgram.lags.is_empty());
        // Azimuth should be normalized
        assert_eq!(vgram.direction_azimuth, 90.0);
    }

    #[test]
    fn test_directional_variogram_too_few_samples() {
        let samples = vec![(0.0, 0.0, 1.0)];
        let result = compute_directional_variogram(&samples, 0.0, 22.5, 5.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_directional_variogram_empty() {
        let samples: Vec<(f64, f64, f64)> = vec![];
        let result = compute_directional_variogram(&samples, 0.0, 22.5, 5.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_anisotropy_model_distance() {
        let model = AnisotropyModel {
            major_range: 100.0,
            minor_range: 50.0,
            major_azimuth: 0.0,
            ratio: 0.5,
            angle_tolerance: 22.5,
            method: "max_min".to_string(),
        };

        // Test that distances vary by direction
        let d_east = model.anisotropic_distance(1.0, 0.0); // East (along major axis at 0°)
        let d_north = model.anisotropic_distance(0.0, 1.0); // North (perpendicular)
        let d_45 = model.anisotropic_distance(1.0, 1.0); // Northeast

        // All should be positive finite distances
        assert!(d_east.is_finite() && d_east > 0.0);
        assert!(d_north.is_finite() && d_north > 0.0);
        assert!(d_45.is_finite() && d_45 > 0.0);

        // North should be larger than East (because minor range < major range)
        assert!(d_north > d_east, "Perpendicular distance should be greater due to anisotropy ratio");
    }

    #[test]
    fn test_fit_anisotropy_basic() {
        // Create two directional variograms with different max semivariances
        // This represents different ranges in different directions
        let vgram_0 = DirectionalVariogramBin {
            direction_azimuth: 0.0,
            tolerance: 22.5,
            lags: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            semivariances: vec![1.0, 2.0, 4.0, 5.5, 5.0], // Peak at lag 4
            counts: vec![10, 8, 6, 4, 2],
            bin_size: 1.0,
            sill: Some(5.5),
            nugget: Some(0.5),
        };

        let vgram_90 = DirectionalVariogramBin {
            direction_azimuth: 90.0,
            tolerance: 22.5,
            lags: vec![1.0, 2.0, 3.0],
            semivariances: vec![1.0, 1.5, 2.0], // Peak at lag 3
            counts: vec![10, 8, 6],
            bin_size: 1.0,
            sill: Some(2.0),
            nugget: Some(0.2),
        };

        let result = fit_anisotropy(&[vgram_0, vgram_90]);
        assert!(result.is_ok());

        let model = result.unwrap();
        // 0° direction peaks at lag 4 (larger range than 90° direction which peaks at lag 3)
        assert!(model.major_range > model.minor_range);
        assert!(model.ratio > 0.0 && model.ratio <= 1.0);
        assert_eq!(model.major_azimuth, 0.0);
    }

    #[test]
    fn test_anisotropy_isotropic_check() {
        let isotropic = AnisotropyModel {
            major_range: 100.0,
            minor_range: 95.0,
            major_azimuth: 0.0,
            ratio: 0.95,
            angle_tolerance: 22.5,
            method: "max_min".to_string(),
        };

        assert!(!isotropic.is_anisotropic(0.8)); // Ratio 0.95 > threshold 0.8
        assert!(isotropic.is_anisotropic(0.96)); // Ratio 0.95 < threshold 0.96

        let anisotropic = AnisotropyModel {
            major_range: 100.0,
            minor_range: 40.0,
            major_azimuth: 0.0,
            ratio: 0.4,
            angle_tolerance: 22.5,
            method: "max_min".to_string(),
        };

        assert!(anisotropic.is_anisotropic(0.5)); // Strong anisotropy
    }
}
