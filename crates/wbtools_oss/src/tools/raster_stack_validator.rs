// Raster stack validation utilities for multi-raster tooling.
// Provides CRS consistency checking, spatial overlap validation, and optional auto-reprojection.

use wbraster::{Raster, CrsInfo};

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
        
        // Check spatial extent overlap
        if !config.allow_no_overlap {
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
