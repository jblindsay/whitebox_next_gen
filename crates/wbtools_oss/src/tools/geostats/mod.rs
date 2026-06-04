use std::collections::BTreeMap;
use serde_json::json;
use wbcore::{
    ToolArgs, ToolCategory, ToolContext,
    ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability, Tool, LicenseTier,
};
use wbvector;
use wbraster;
use wbgeostats::{
    variogram::{EmpiricalVariogramBuilder, VariogramModelFamily, VariogramFitter},
    kriging::OrdinaryKriging,
    cv::LeaveOneOutCV,
};
use serde_json::Value;

mod variogram_estimation;
pub use variogram_estimation::EstimateVariogramTool;

mod variogram_fitting;
pub use variogram_fitting::FitVariogramTool;

mod ordinary_kriging;
pub use ordinary_kriging::OrdinaryKrigingTool;

mod cross_validation;
pub use cross_validation::KrigingCrossValidationTool;

// Helper functions for vector loading
fn load_vector_arg(args: &ToolArgs, key: &str) -> Result<wbvector::Layer, ToolError> {
    let path = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;
    load_vector(path.trim(), key)
}

fn load_vector(path: &str, key: &str) -> Result<wbvector::Layer, ToolError> {
    // Handle in-memory vectors
    if wbvector::memory_store::vector_is_memory_path(path) {
        let id = wbvector::memory_store::vector_path_to_id(path).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' has malformed in-memory vector path",
                key
            ))
        })?;
        return wbvector::memory_store::get_vector_arc_by_id(id)
            .map(|layer| layer.as_ref().clone())
            .ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' references unknown in-memory vector id '{}': store entry is missing",
                    key, id
                ))
            });
    }
    
    // Load from file path
    wbvector::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading {} vector: {}", key, e)))
}

// Helper functions for argument parsing
fn parse_optional_f64_arg(args: &ToolArgs, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn parse_optional_i64_arg(args: &ToolArgs, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

fn parse_string_arg<'a>(args: &'a ToolArgs, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))
}
