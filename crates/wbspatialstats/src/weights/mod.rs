// Spatial weights infrastructure for neighborhood definitions in spatial statistics
//
// Provides shared weight construction and neighborhood analysis for:
// - Phase A: Autocorrelation (Moran's I, LISA, Getis-Ord, NNI, Quadrat)
// - Phase C: Spatial Regression (Spatial Lag, Spatial Error, GWR)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialWeightsMode {
    /// Queen contiguity (edge or corner touching)
    Queen,
    /// Rook contiguity (edge touching only)
    Rook,
    /// k-nearest neighbors
    KNearest,
    /// Fixed distance band threshold
    DistanceBand,
}

impl SpatialWeightsMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Queen => "queen",
            Self::Rook => "rook",
            Self::KNearest => "k_nearest",
            Self::DistanceBand => "distance_band",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "queen" => Some(Self::Queen),
            "rook" => Some(Self::Rook),
            "k_nearest" => Some(Self::KNearest),
            "distance_band" => Some(Self::DistanceBand),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IslandPolicy {
    /// Drop islands with a warning message
    DropWithWarning,
    /// Keep islands with zero-weight rows
    KeepZeroWeight,
    /// Raise an error if islands exist
    Error,
}

impl IslandPolicy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DropWithWarning => "drop_with_warning",
            Self::KeepZeroWeight => "keep_zero_weight",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "drop_with_warning" => Some(Self::DropWithWarning),
            "keep_zero_weight" => Some(Self::KeepZeroWeight),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Diagnostic statistics for a spatial weights matrix
#[derive(Clone, Debug)]
pub struct SpatialWeightsDiagnostics {
    /// Number of features in the weights matrix
    pub n_features: usize,
    /// Number of islands (features with zero neighbors)
    pub n_islands: usize,
    /// Minimum neighbor count across features
    pub neighbor_count_min: usize,
    /// Mean neighbor count (arithmetic mean)
    pub neighbor_count_mean: f64,
    /// Maximum neighbor count across features
    pub neighbor_count_max: usize,
    /// Number of connected components in the spatial graph
    pub connected_component_count: usize,
    /// Whether row standardization was applied
    pub row_standardized: bool,
    /// Number of features dropped due to island policy
    pub dropped_feature_count: usize,
}

/// Spatial weights graph with adjacency neighbors and diagnostics
#[derive(Clone, Debug)]
pub struct SpatialWeightsGraph {
    /// Adjacency list: neighbors[i] = [(j, weight_ij), ...]
    pub neighbors: Vec<Vec<(usize, f64)>>,
    /// Diagnostic statistics about the weight matrix
    pub diagnostics: SpatialWeightsDiagnostics,
    /// Warning messages (e.g., islands dropped, approximations used)
    pub warnings: Vec<String>,
}

impl SpatialWeightsGraph {
    /// Get the number of features
    pub fn n_features(&self) -> usize {
        self.neighbors.len()
    }

    /// Get the number of islands (isolated features)
    pub fn n_islands(&self) -> usize {
        self.diagnostics.n_islands
    }

    /// Check if row standardization was applied
    pub fn is_row_standardized(&self) -> bool {
        self.diagnostics.row_standardized
    }

    /// Get all warning messages
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// Statistical utility: Normal CDF approximation
pub fn normal_cdf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let poly = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 1.0 - pdf * poly;
    if x >= 0.0 { cdf } else { 1.0 - cdf }
}

/// Two-tailed normal p-value for a z-score
pub fn two_tailed_normal_p(z: f64) -> f64 {
    (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0)
}

/// Count connected components in an undirected spatial graph
pub fn connected_components(neighbors: &[Vec<(usize, f64)>]) -> usize {
    let n = neighbors.len();
    let mut undirected = vec![Vec::<usize>::new(); n];
    for (i, row) in neighbors.iter().enumerate() {
        for (j, _) in row {
            undirected[i].push(*j);
            undirected[*j].push(i);
        }
    }

    let mut visited = vec![false; n];
    let mut components = 0usize;
    for start in 0..n {
        if visited[start] {
            continue;
        }
        components += 1;
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(node) = stack.pop() {
            for next in &undirected[node] {
                if !visited[*next] {
                    visited[*next] = true;
                    stack.push(*next);
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_weights_mode_parsing() {
        assert_eq!(SpatialWeightsMode::from_str("queen"), Some(SpatialWeightsMode::Queen));
        assert_eq!(SpatialWeightsMode::from_str("ROOK"), Some(SpatialWeightsMode::Rook));
        assert_eq!(SpatialWeightsMode::from_str("k_nearest"), Some(SpatialWeightsMode::KNearest));
        assert_eq!(SpatialWeightsMode::from_str("distance_band"), Some(SpatialWeightsMode::DistanceBand));
        assert_eq!(SpatialWeightsMode::from_str("invalid"), None);
    }

    #[test]
    fn test_island_policy_parsing() {
        assert_eq!(IslandPolicy::from_str("drop_with_warning"), Some(IslandPolicy::DropWithWarning));
        assert_eq!(IslandPolicy::from_str("KEEP_ZERO_WEIGHT"), Some(IslandPolicy::KeepZeroWeight));
        assert_eq!(IslandPolicy::from_str("error"), Some(IslandPolicy::Error));
        assert_eq!(IslandPolicy::from_str("invalid"), None);
    }

    #[test]
    fn test_normal_cdf() {
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.001);
        assert!(normal_cdf(2.0) > 0.95);
        assert!(normal_cdf(-2.0) < 0.05);
    }

    #[test]
    fn test_two_tailed_p() {
        let p = two_tailed_normal_p(2.0);
        assert!(p > 0.0 && p < 0.05);
    }

    #[test]
    fn test_connected_components_simple() {
        // Single component: 0-1-2
        let neighbors = vec![
            vec![(1, 1.0)],
            vec![(0, 1.0), (2, 1.0)],
            vec![(1, 1.0)],
        ];
        assert_eq!(connected_components(&neighbors), 1);
    }

    #[test]
    fn test_connected_components_multiple() {
        // Two components: (0-1) and (2)
        let neighbors = vec![
            vec![(1, 1.0)],
            vec![(0, 1.0)],
            vec![],
        ];
        assert_eq!(connected_components(&neighbors), 2);
    }
}
