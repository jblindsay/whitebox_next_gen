// Raster stack validation utilities for multi-raster tooling.
// Provides CRS consistency checking, spatial overlap validation, and optional auto-reprojection.

use wbraster::{CrsInfo, Raster, ResampleMethod};

/// Configuration options for raster stack validation and reprojection.
#[derive(Debug, Clone)]
pub struct RasterStackConfig {
    /// If true, automatically reproject inputs to match the first raster's CRS.
    /// Uses nearest-neighbour for categorical rasters, bilinear for continuous.
    pub auto_reproject: bool,
    
    /// Resampling method override (e.g., "nearest", "bilinear", "cubic").
    /// If None, will be auto-detected based on raster interpretation.
    pub resampling_method: Option<String>,
    
    /// If true, allow rasters with non-overlapping extents (not recommended for most overlay ops).
    pub allow_no_overlap: bool,
}

impl Default for RasterStackConfig {
    fn default() -> Self {
        RasterStackConfig {
            auto_reproject: true,
            resampling_method: None,
            allow_no_overlap: false,
        }
    }
}

/// Validation result for a raster stack.
#[derive(Debug, Clone)]
pub struct RasterStackValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub crs_mismatch: bool,
    pub extent_mismatch: bool,
}

impl RasterStackValidation {
    pub fn new() -> Self {
        RasterStackValidation {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            crs_mismatch: false,
            extent_mismatch: false,
        }
    }
}

/// Validates a stack of rasters for compatibility.
///
/// Checks:
/// * All rasters have same dimensions (rows, cols)
/// * All rasters have matching CRS (with optional auto-reproject)
/// * Spatial extents overlap (unless allow_no_overlap is true)
///
/// Returns a validation result. If `config.auto_reproject` is true, rasters are
/// considered compatible even with CRS differences (reprojection will be applied).
pub fn validate_raster_stack(
    rasters: &[Raster],
    config: &RasterStackConfig,
) -> RasterStackValidation {
    let mut result = RasterStackValidation::new();
    
    if rasters.is_empty() {
        result.errors.push("Raster stack is empty".to_string());
        result.is_valid = false;
        return result;
    }
    
    let first = &rasters[0];
    let first_rows = first.rows;
    let first_cols = first.cols;
    let first_crs = &first.crs;
    let first_extent = first.extent();
    
    for (idx, raster) in rasters.iter().enumerate().skip(1) {
        // Check dimensions match
        if raster.rows != first_rows || raster.cols != first_cols {
            result.errors.push(
                format!(
                    "Raster {} dimension mismatch: {} rows × {} cols (expected {} × {})",
                    idx, raster.rows, raster.cols, first_rows, first_cols
                )
            );
            result.is_valid = false;
        }
        
        // Check CRS compatibility
        if !crs_compatible(first_crs, &raster.crs) {
            result.crs_mismatch = true;
            if config.auto_reproject {
                result.warnings.push(
                    format!(
                        "Raster {} will be reprojected from CRS {} to {}",
                        idx,
                        format_crs(&raster.crs),
                        format_crs(first_crs)
                    )
                );
            } else {
                result.errors.push(
                    format!(
                        "Raster {} CRS mismatch: {} (expected {})",
                        idx,
                        format_crs(&raster.crs),
                        format_crs(first_crs)
                    )
                );
                result.is_valid = false;
            }
        }
        
        // Check spatial extent overlap only when rasters are in the same CRS.
        if !config.allow_no_overlap && crs_compatible(first_crs, &raster.crs) {
            let extent = raster.extent();
            if !extents_overlap(&first_extent, &extent) {
                result.extent_mismatch = true;
                result.errors.push(
                    format!(
                        "Raster {} has non-overlapping spatial extent",
                        idx
                    )
                );
                result.is_valid = false;
            }
        }
    }
    
    result
}

/// Validates raster stack dimensions and CRS only (faster than full validation).
pub fn validate_raster_stack_fast(
    rasters: &[Raster],
    config: &RasterStackConfig,
) -> RasterStackValidation {
    let mut result = RasterStackValidation::new();
    
    if rasters.is_empty() {
        result.errors.push("Raster stack is empty".to_string());
        result.is_valid = false;
        return result;
    }
    
    let first = &rasters[0];
    let first_rows = first.rows;
    let first_cols = first.cols;
    let first_crs = &first.crs;
    
    for (idx, raster) in rasters.iter().enumerate().skip(1) {
        // Check dimensions match
        if raster.rows != first_rows || raster.cols != first_cols {
            result.errors.push(
                format!(
                    "Raster {} dimension mismatch: {} × {} (expected {} × {})",
                    idx, raster.rows, raster.cols, first_rows, first_cols
                )
            );
            result.is_valid = false;
        }
        
        // Check CRS compatibility
        if !crs_compatible(first_crs, &raster.crs) {
            result.crs_mismatch = true;
            if config.auto_reproject {
                result.warnings.push(
                    format!(
                        "Raster {} will be reprojected from {} to {}",
                        idx,
                        format_crs(&raster.crs),
                        format_crs(first_crs)
                    )
                );
            } else {
                result.errors.push(
                    format!(
                        "Raster {} CRS mismatch: {} (expected {})",
                        idx,
                        format_crs(&raster.crs),
                        format_crs(first_crs)
                    )
                );
                result.is_valid = false;
            }
        }
    }
    
    result
}

/// Parse a user-facing resampling method string into a wbraster resampling enum.
pub fn parse_resample_method(method: &str) -> Option<ResampleMethod> {
    match method.trim().to_ascii_lowercase().as_str() {
        "nearest" | "nearest_neighbor" | "nearest-neighbour" | "nearest-neighbor" => {
            Some(ResampleMethod::Nearest)
        }
        "bilinear" => Some(ResampleMethod::Bilinear),
        "cubic" | "bicubic" => Some(ResampleMethod::Cubic),
        "lanczos" => Some(ResampleMethod::Lanczos),
        "average" | "mean" => Some(ResampleMethod::Average),
        "min" | "minimum" => Some(ResampleMethod::Min),
        "max" | "maximum" => Some(ResampleMethod::Max),
        "mode" | "modal" => Some(ResampleMethod::Mode),
        "median" => Some(ResampleMethod::Median),
        "stddev" | "standard_deviation" | "standard-deviation" => {
            Some(ResampleMethod::StdDev)
        }
        _ => None,
    }
}

/// Align and validate a raster stack against the first raster.
///
/// Behavior:
/// * hard-fails on non-overlapping extents (unless allow_no_overlap is true)
/// * auto-reprojects CRS-mismatched rasters when auto_reproject is true
/// * for auto-reprojection, uses configured resampling method if provided,
///   otherwise nearest for categorical rasters and bilinear for continuous
pub fn align_and_validate_raster_stack(
    rasters: &mut [Raster],
    config: &RasterStackConfig,
) -> Result<Vec<String>, String> {
    if rasters.is_empty() {
        return Err("Raster stack is empty".to_string());
    }

    let reference = rasters[0].clone();
    let mut warnings = Vec::<String>::new();

    for (idx, raster) in rasters.iter_mut().enumerate().skip(1) {
        if !crs_compatible(&reference.crs, &raster.crs) {
            if !config.auto_reproject {
                return Err(format!(
                    "Raster {} CRS mismatch: {} (expected {})",
                    idx,
                    format_crs(&raster.crs),
                    format_crs(&reference.crs)
                ));
            }

            if reference.crs.epsg.is_none() {
                return Err(
                    "Auto-reprojection requires EPSG on the reference raster (inputs[0])"
                        .to_string(),
                );
            }

            let method_name = config
                .resampling_method
                .as_deref()
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| infer_resampling_method(raster));
            let method = parse_resample_method(method_name).ok_or_else(|| {
                format!(
                    "Unsupported auto_reproject_method '{}'. Supported: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev",
                    method_name
                )
            })?;

            *raster = raster
                .reproject_to_match_grid(&reference, method)
                .map_err(|e| {
                    format!(
                        "Failed to auto-reproject raster {} from {} to {}: {}",
                        idx,
                        format_crs(&raster.crs),
                        format_crs(&reference.crs),
                        e
                    )
                })?;

            warnings.push(format!(
                "Auto-reprojected raster {} to {} using {}",
                idx,
                format_crs(&reference.crs),
                method_name
            ));
        }

        if raster.rows != reference.rows || raster.cols != reference.cols || raster.bands != reference.bands {
            return Err(format!(
                "Raster {} dimension mismatch: {}x{}x{} (expected {}x{}x{})",
                idx,
                raster.rows,
                raster.cols,
                raster.bands,
                reference.rows,
                reference.cols,
                reference.bands
            ));
        }

        if !config.allow_no_overlap {
            let extent = raster.extent();
            let reference_extent = reference.extent();
            if !extents_overlap(&reference_extent, &extent) {
                return Err(format!(
                    "Raster {} has non-overlapping spatial extent",
                    idx
                ));
            }
        }
    }

    Ok(warnings)
}

/// Checks if two CRS objects are compatible (same EPSG or equal definitions).
fn crs_compatible(crs1: &CrsInfo, crs2: &CrsInfo) -> bool {
    // Check EPSG match first (fastest path)
    if let (Some(epsg1), Some(epsg2)) = (crs1.epsg, crs2.epsg) {
        return epsg1 == epsg2;
    }
    
    // Fall back to WKT comparison
    if let (Some(wkt1), Some(wkt2)) = (crs1.wkt.as_deref(), crs2.wkt.as_deref()) {
        return wkt1.trim() == wkt2.trim();
    }
    
    // If both are empty/default, they're compatible
    crs1.epsg.is_none() && crs1.wkt.is_none() && crs2.epsg.is_none() && crs2.wkt.is_none()
}

/// Formats a CrsInfo for display purposes.
fn format_crs(crs: &CrsInfo) -> String {
    if let Some(epsg) = crs.epsg {
        return format!("EPSG:{}", epsg);
    }
    if let Some(wkt) = &crs.wkt {
        let wkt_short = if wkt.len() > 50 {
            format!("{}...", &wkt[..47])
        } else {
            wkt.clone()
        };
        return wkt_short;
    }
    "Default CRS".to_string()
}

/// Checks if two extents overlap in space.
/// Extents are ordered as: min_x, min_y, max_x, max_y (from the Extent struct)
#[inline]
fn extents_overlap(ext1: &wbraster::Extent, ext2: &wbraster::Extent) -> bool {
    // Check if rectangles are disjoint (then return false), else they overlap
    ext1.x_min <= ext2.x_max && ext2.x_min <= ext1.x_max &&  // x-axis overlap
    ext1.y_min <= ext2.y_max && ext2.y_min <= ext1.y_max     // y-axis overlap
}

/// Determines the appropriate resampling method for a raster.
///
/// Returns "nearest" for categorical/palette data, "bilinear" for continuous.
pub fn infer_resampling_method(raster: &Raster) -> &'static str {
    // Check metadata for photometric/color interpretation hints
    for (key, value) in &raster.metadata {
        let key_lower = key.to_lowercase();
        let value_lower = value.to_lowercase();
        
        // Look for palette/categorical indicators
        if key_lower.contains("color_interpretation") || key_lower.contains("photometric") {
            if value_lower.contains("palette") || value_lower.contains("index") || 
               value_lower.contains("gray") || value_lower.contains("palette_index") {
                return "nearest";
            }
        }
        
        // Look for class/category indicators
        if key_lower.contains("interpretation") {
            if value_lower.contains("class") || value_lower.contains("category") || 
               value_lower.contains("palette") {
                return "nearest";
            }
        }
    }
    
    // Default to bilinear for continuous/unknown data
    "bilinear"
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extents_overlap() {
        // Simple overlap
        let ext1 = wbraster::Extent { x_min: 0.0, y_min: 0.0, x_max: 10.0, y_max: 10.0 };
        let ext2 = wbraster::Extent { x_min: 5.0, y_min: 5.0, x_max: 15.0, y_max: 15.0 };
        assert!(extents_overlap(&ext1, &ext2));
        
        // No overlap - disjoint
        let ext3 = wbraster::Extent { x_min: 11.0, y_min: 11.0, x_max: 20.0, y_max: 20.0 };
        assert!(!extents_overlap(&ext1, &ext3));
        
        // Complete overlap
        let ext4 = wbraster::Extent { x_min: 2.0, y_min: 2.0, x_max: 8.0, y_max: 8.0 };
        assert!(extents_overlap(&ext1, &ext4));
        
        // Edge touching (should be considered overlapping)
        let ext5 = wbraster::Extent { x_min: 10.0, y_min: 0.0, x_max: 20.0, y_max: 10.0 };
        assert!(extents_overlap(&ext1, &ext5));
    }
    
    #[test]
    fn test_crs_compatible() {
        // Same EPSG
        let crs1 = CrsInfo { epsg: Some(4326), wkt: None };
        let crs2 = CrsInfo { epsg: Some(4326), wkt: None };
        assert!(crs_compatible(&crs1, &crs2));
        
        // Different EPSG
        let crs3 = CrsInfo { epsg: Some(3857), wkt: None };
        assert!(!crs_compatible(&crs1, &crs3));
        
        // Both default (empty)
        let crs4 = CrsInfo { epsg: None, wkt: None };
        let crs5 = CrsInfo { epsg: None, wkt: None };
        assert!(crs_compatible(&crs4, &crs5));
    }
}
