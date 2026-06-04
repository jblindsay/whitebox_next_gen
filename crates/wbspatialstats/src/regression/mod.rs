// Spatial regression models (Phase C)
//
// Provides spatial lag, spatial error, and geographically weighted regression
// with production-grade estimation, diagnostics, and inference.

use crate::weights::SpatialWeightsGraph;
use nalgebra::{DMatrix, DVectorSlice};
use std::fmt;

pub mod spatial_lag;
pub mod spatial_error;
pub mod gwr;
pub mod diagnostics;

pub use spatial_lag::SpatialLagRegression;
pub use spatial_error::SpatialErrorRegression;
pub use gwr::GeographicallyWeightedRegression;

/// Common result type for regression operations
pub type RegressionResult<T> = Result<T, String>;

/// Effect decomposition for spatial lag models (Direct, Indirect, Total)
#[derive(Debug, Clone)]
pub struct EffectDecomposition {
    /// Direct effects: effect of own variable change
    pub direct_effects: Vec<f64>,
    /// Indirect effects: spillover effects through neighbors
    pub indirect_effects: Vec<f64>,
    /// Total effects: direct + indirect
    pub total_effects: Vec<f64>,
    /// Standard errors for direct effects
    pub direct_se: Vec<f64>,
    /// Standard errors for indirect effects
    pub indirect_se: Vec<f64>,
    /// Standard errors for total effects
    pub total_se: Vec<f64>,
    /// P-values for direct effects
    pub direct_pvalues: Vec<f64>,
    /// P-values for indirect effects
    pub indirect_pvalues: Vec<f64>,
    /// P-values for total effects
    pub total_pvalues: Vec<f64>,
}

impl EffectDecomposition {
    /// Create effect decomposition with safety checks
    pub fn new(
        direct: Vec<f64>,
        indirect: Vec<f64>,
        total: Vec<f64>,
        direct_se: Vec<f64>,
        indirect_se: Vec<f64>,
        total_se: Vec<f64>,
    ) -> RegressionResult<Self> {
        let n = direct.len();
        if indirect.len() != n || total.len() != n || direct_se.len() != n
            || indirect_se.len() != n || total_se.len() != n
        {
            return Err("Effect decomposition vectors must have equal length".to_string());
        }

        let direct_pvalues = direct
            .iter()
            .zip(direct_se.iter())
            .map(|(effect, se)| {
                if *se > 0.0 {
                    crate::weights::two_tailed_normal_p(effect / se)
                } else {
                    1.0
                }
            })
            .collect();

        let indirect_pvalues = indirect
            .iter()
            .zip(indirect_se.iter())
            .map(|(effect, se)| {
                if *se > 0.0 {
                    crate::weights::two_tailed_normal_p(effect / se)
                } else {
                    1.0
                }
            })
            .collect();

        let total_pvalues = total
            .iter()
            .zip(total_se.iter())
            .map(|(effect, se)| {
                if *se > 0.0 {
                    crate::weights::two_tailed_normal_p(effect / se)
                } else {
                    1.0
                }
            })
            .collect();

        Ok(EffectDecomposition {
            direct_effects: direct,
            indirect_effects: indirect,
            total_effects: total,
            direct_se,
            indirect_se,
            total_se,
            direct_pvalues,
            indirect_pvalues,
            total_pvalues,
        })
    }
}

/// Convergence diagnostics for iterative estimation methods
#[derive(Debug, Clone)]
pub struct ConvergenceDiagnostics {
    /// Whether iteration converged
    pub converged: bool,
    /// Number of iterations performed
    pub iterations: usize,
    /// Maximum iterations allowed
    pub max_iterations: usize,
    /// Final gradient norm (for convergence assessment)
    pub final_gradient_norm: f64,
    /// Convergence tolerance used
    pub tolerance: f64,
    /// Reason for stopping (e.g., "Converged", "Max iterations reached")
    pub stopping_reason: String,
}

/// Pre-flight diagnostic checks
#[derive(Debug, Clone)]
pub struct PreFlightDiagnostics {
    /// Design matrix condition number
    pub design_matrix_condition_number: f64,
    /// Design matrix rank
    pub design_matrix_rank: usize,
    /// Response variable variance
    pub response_variance: f64,
    /// Warnings from design matrix analysis
    pub design_warnings: Vec<String>,
    /// Warnings from response analysis
    pub response_warnings: Vec<String>,
    /// Warnings from weights analysis
    pub weights_warnings: Vec<String>,
    /// Can proceed with estimation
    pub can_proceed: bool,
}

impl fmt::Display for PreFlightDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PreFlightDiagnostics {{\n  condition_number: {:.2e},\n  rank: {},\n  response_variance: {:.6},\n  can_proceed: {},\n  warnings: {:?}\n}}",
            self.design_matrix_condition_number,
            self.design_matrix_rank,
            self.response_variance,
            self.can_proceed,
            [
                self.design_warnings.clone(),
                self.response_warnings.clone(),
                self.weights_warnings.clone()
            ]
            .concat()
        )
    }
}

/// Structured residual summary for interpretation
#[derive(Debug, Clone)]
pub struct ResidualSummary {
    /// Residual mean (should be near zero)
    pub mean: f64,
    /// Residual standard deviation
    pub std_dev: f64,
    /// Minimum residual
    pub min: f64,
    /// 25th percentile
    pub q25: f64,
    /// Median residual
    pub median: f64,
    /// 75th percentile
    pub q75: f64,
    /// Maximum residual
    pub max: f64,
    /// Spatial autocorrelation of residuals (Moran's I)
    pub morans_i: f64,
    /// P-value for Moran's I
    pub morans_i_pvalue: f64,
    /// Interpretation string
    pub interpretation: String,
}

impl fmt::Display for ResidualSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ResidualSummary {{\n  mean: {:.6},\n  std_dev: {:.6},\n  range: [{:.6}, {:.6}],\n  morans_i: {:.6} (p={:.6}),\n  interpretation: \"{}\"\n}}",
            self.mean, self.std_dev, self.min, self.max, self.morans_i, self.morans_i_pvalue, self.interpretation
        )
    }
}

/// Base regression result structure (shared by all three models)
#[derive(Debug, Clone)]
pub struct RegressionResultBase {
    /// Fitted model coefficients (intercept, then covariates)
    pub coefficients: Vec<f64>,
    /// Coefficient standard errors
    pub standard_errors: Vec<f64>,
    /// T-statistics for coefficients
    pub t_statistics: Vec<f64>,
    /// P-values for coefficients
    pub p_values: Vec<f64>,
    /// Fitted values
    pub fitted: Vec<f64>,
    /// Residuals (observed - fitted)
    pub residuals: Vec<f64>,
    /// Residual sum of squares
    pub rss: f64,
    /// Total sum of squares
    pub tss: f64,
    /// R-squared
    pub r_squared: f64,
    /// Adjusted R-squared
    pub r_squared_adj: f64,
    /// Log-likelihood
    pub log_likelihood: f64,
    /// AIC (Akaike Information Criterion)
    pub aic: f64,
    /// Number of observations
    pub n_obs: usize,
    /// Number of parameters
    pub n_params: usize,
    /// Pre-flight diagnostics
    pub preflight: PreFlightDiagnostics,
    /// Convergence diagnostics (if applicable)
    pub convergence: Option<ConvergenceDiagnostics>,
    /// Residual summary
    pub residual_summary: ResidualSummary,
}

/// Spatial lag regression (SAR) result
#[derive(Debug, Clone)]
pub struct SpatialLagResult {
    /// Base regression results
    pub base: RegressionResultBase,
    /// Spatial lag parameter (ρ/rho)
    pub rho: f64,
    /// Standard error of rho
    pub rho_se: f64,
    /// T-statistic for rho
    pub rho_t: f64,
    /// P-value for rho
    pub rho_pvalue: f64,
    /// Effect decomposition (Direct, Indirect, Total)
    pub effects: Option<EffectDecomposition>,
}

/// Spatial error regression (SEM) result
#[derive(Debug, Clone)]
pub struct SpatialErrorResult {
    /// Base regression results
    pub base: RegressionResultBase,
    /// Spatial error parameter (λ/lambda)
    pub lambda: f64,
    /// Standard error of lambda
    pub lambda_se: f64,
    /// T-statistic for lambda
    pub lambda_t: f64,
    /// P-value for lambda
    pub lambda_pvalue: f64,
    /// Estimation method used
    pub method: String, // "FGLS" or "MLE"
}

/// Geographically weighted regression (GWR) result
#[derive(Debug, Clone)]
pub struct GWRResult {
    /// Local coefficients for each location (rows: locations, cols: parameters)
    pub local_coefficients: DMatrix<f64>,
    /// Local standard errors for coefficients
    pub local_standard_errors: DMatrix<f64>,
    /// Local t-statistics
    pub local_t_statistics: DMatrix<f64>,
    /// Local p-values
    pub local_p_values: DMatrix<f64>,
    /// Global fitted values
    pub fitted: Vec<f64>,
    /// Residuals
    pub residuals: Vec<f64>,
    /// Global R-squared
    pub r_squared: f64,
    /// Global AIC
    pub aic: f64,
    /// Selected bandwidth
    pub bandwidth: f64,
    /// Bandwidth selection criterion (AICc value)
    pub aicc: f64,
    /// Kernel type used
    pub kernel: String, // "gaussian", "bisquare", etc.
    /// Number of observations
    pub n_obs: usize,
    /// Number of parameters
    pub n_params: usize,
    /// Coefficient stability metrics
    pub coefficient_stability: Vec<f64>,
    /// Pre-flight diagnostics
    pub preflight: PreFlightDiagnostics,
}
