//! Point-process analysis: Ripley's K/L functions, envelope testing, and diagnostics
//! 
//! This module provides tools for analyzing spatial point patterns including:
//! - K and L functions for pattern characterization
//! - Critical-band envelope testing via Monte Carlo simulation
//! - Inhomogeneous process modeling with intensity correction
//! - Residual diagnostics for goodness-of-fit
//! 
//! References:
//! - Ripley, B.D. (1976). "The second-order analysis of stationary point processes"
//! - Baddeley, A., Rubak, E., Turner, R. (2015). "Spatial Point Patterns: Methodology and Applications"
//! - Møller, J., Waagepetersen, R.P. (2003). "Statistical Inference and Simulation for Spatial Point Processes"

pub mod ripley;
pub mod envelopes;
pub mod inhomogeneous;
pub mod diagnostics;

pub use ripley::{KFunction, KFunctionResult};
pub use envelopes::{EnvelopeResult, CriticalBandEnvelope};
pub use inhomogeneous::{InhomogeneousKProcess, InhomogeneousResult};
pub use diagnostics::{PointProcessResiduals, ResidualType};

use crate::GeostatError;
use serde::{Deserialize, Serialize};

/// Distance range for K/L computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceRange {
    /// Minimum distance
    pub min: f64,
    /// Maximum distance
    pub max: f64,
    /// Number of distance bins
    pub n_bins: usize,
}

impl DistanceRange {
    /// Create a distance range with specified parameters
    pub fn new(min: f64, max: f64, n_bins: usize) -> Result<Self, GeostatError> {
        if !min.is_finite() || !max.is_finite() || min < 0.0 || max <= min {
            return Err(GeostatError::InvalidParameters(
                "distance range must have 0 <= min < max (both finite)".to_string(),
            ));
        }
        if n_bins < 2 {
            return Err(GeostatError::InvalidParameters(
                "must have at least 2 distance bins".to_string(),
            ));
        }
        Ok(DistanceRange { min, max, n_bins })
    }

    /// Get bin edges (n_bins + 1 values)
    pub fn bin_edges(&self) -> Vec<f64> {
        (0..=self.n_bins)
            .map(|i| self.min + i as f64 * (self.max - self.min) / self.n_bins as f64)
            .collect()
    }

    /// Get bin centers
    pub fn bin_centers(&self) -> Vec<f64> {
        let edges = self.bin_edges();
        edges.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect()
    }

    /// Get bin width
    pub fn bin_width(&self) -> f64 {
        (self.max - self.min) / self.n_bins as f64
    }
}

/// Study area definition for edge correction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StudyAreaType {
    /// Bounding rectangle (min_x, min_y, max_x, max_y)
    Rectangle,
    /// Convex hull (automatic computation)
    ConvexHull,
    /// Circular region (center_x, center_y, radius)
    Circle,
}

/// Overall point pattern summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSummary {
    /// Number of points
    pub n_points: usize,
    /// Study area
    pub area: f64,
    /// Point intensity (points per unit area)
    pub intensity: f64,
    /// Mean nearest-neighbor distance
    pub mean_nnd: f64,
    /// Minimum x coordinate
    pub min_x: f64,
    /// Maximum x coordinate
    pub max_x: f64,
    /// Minimum y coordinate
    pub min_y: f64,
    /// Maximum y coordinate
    pub max_y: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_range_creation() {
        let dr = DistanceRange::new(0.0, 100.0, 10).unwrap();
        assert_eq!(dr.n_bins, 10);
        assert_eq!(dr.bin_edges().len(), 11);
    }

    #[test]
    fn test_distance_range_bin_centers() {
        let dr = DistanceRange::new(0.0, 100.0, 10).unwrap();
        let centers = dr.bin_centers();
        assert_eq!(centers.len(), 10);
        assert!((centers[0] - 5.0).abs() < 1e-10);
        assert!((centers[9] - 95.0).abs() < 1e-10);
    }

    #[test]
    fn test_distance_range_invalid() {
        assert!(DistanceRange::new(100.0, 50.0, 10).is_err()); // min > max
        assert!(DistanceRange::new(0.0, 100.0, 1).is_err()); // not enough bins
        assert!(DistanceRange::new(0.0, -1.0, 10).is_err()); // max is negative
    }
}
