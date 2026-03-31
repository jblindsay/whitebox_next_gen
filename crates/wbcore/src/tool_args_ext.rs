//! Shared helpers for parsing common [`ToolArgs`] parameters.
//!
//! Placing these here avoids duplicate parsing logic across `wbtools_oss` and
//! `wbtools_pro` without introducing any cross-crate dependency between the two
//! tool crates.

use std::path::PathBuf;

use serde_json::Value;

use crate::{ToolArgs, ToolError};

/// Resolve a raster-path `Value` to a plain path string.
///
/// Accepts either a plain path string, or a typed raster object
/// `{ "__wbw_type__": "raster", "path": "…" }`.  `param` is used only in
/// error messages.
pub fn parse_raster_path_value(value: &Value, param: &str) -> Result<String, ToolError> {
    if let Some(path) = value.as_str() {
        return Ok(path.to_string());
    }

    if let Some(obj) = value.as_object() {
        if obj.get("__wbw_type__").and_then(Value::as_str) == Some("raster") {
            let path = obj
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ToolError::Validation(format!(
                        "typed raster '{}' requires string field 'path'",
                        param
                    ))
                })?;
            return Ok(path.to_string());
        }
    }

    Err(ToolError::Validation(format!(
        "parameter '{}' must be a raster path string or typed raster object",
        param
    )))
}

/// Resolve a vector-path `Value` to a plain path string.
///
/// Accepts either a plain path string, or a typed vector object
/// `{ "__wbw_type__": "vector", "path": "…" }`.
pub fn parse_vector_path_value(value: &Value, param: &str) -> Result<String, ToolError> {
    if let Some(path) = value.as_str() {
        return Ok(path.to_string());
    }

    if let Some(obj) = value.as_object() {
        if obj.get("__wbw_type__").and_then(Value::as_str) == Some("vector") {
            let path = obj
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ToolError::Validation(format!(
                        "typed vector '{}' requires string field 'path'",
                        param
                    ))
                })?;
            return Ok(path.to_string());
        }
    }

    Err(ToolError::Validation(format!(
        "parameter '{}' must be a vector path string or typed vector object",
        param
    )))
}

/// Look up a required raster-path parameter from `args` and resolve it to a
/// plain path string.
///
/// Equivalent to extracting `args[param]` and then calling
/// [`parse_raster_path_value`].
pub fn parse_raster_path_arg(args: &ToolArgs, param: &str) -> Result<String, ToolError> {
    let val = args.get(param).ok_or_else(|| {
        ToolError::Validation(format!("missing required parameter '{}'", param))
    })?;
    parse_raster_path_value(val, param)
}

/// Look up a required vector-path parameter from `args` and resolve it to a
/// plain path string.
pub fn parse_vector_path_arg(args: &ToolArgs, param: &str) -> Result<String, ToolError> {
    let val = args.get(param).ok_or_else(|| {
        ToolError::Validation(format!("missing required parameter '{}'", param))
    })?;
    parse_vector_path_value(val, param)
}

/// Parse an optional output-path parameter from `args`.
///
/// Returns `None` when the parameter is absent.  Returns an error when it is
/// present but not a string.
pub fn parse_optional_output_path(
    args: &ToolArgs,
    param: &str,
) -> Result<Option<PathBuf>, ToolError> {
    if let Some(value) = args.get(param) {
        let path = value.as_str().ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' must be a string when provided",
                param
            ))
        })?;
        return Ok(Some(PathBuf::from(path)));
    }
    Ok(None)
}
