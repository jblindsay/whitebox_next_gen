//! LiDAR processing tools for wbtools_oss.

mod improved_ground_point_filter;
pub use improved_ground_point_filter::ImprovedGroundPointFilterTool;

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};

use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use nalgebra::{linalg::SymmetricEigen, Matrix3, Vector3};
use evalexpr::{build_operator_tree, Context, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value as EvalValue};
use rand::{RngExt, SeedableRng};
use rand::seq::IndexedRandom;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use serde_json::{json, Value};
use wide::f64x4;
use wbcore::{
    parse_optional_output_path,
    parse_raster_path_value,
    parse_vector_path_arg,
    LicenseTier,
    Tool,
    ToolArgs,
    ToolCategory,
    ToolContext,
    ToolError,
    ToolMetadata,
    ToolParamSpec,
    ToolRunResult,
};
use wblidar::{Crs as LidarCrs, PointCloud, PointRecord, Rgb16};
use wbraster::{CrsInfo, DataType, Raster, RasterConfig, RasterFormat};
use wbtopology::{delaunay_triangulation, Coord as TopoCoord, PreparedSibsonInterpolator};

use crate::memory_store;

pub struct LidarNearestNeighbourGriddingTool;
pub struct LidarIdwInterpolationTool;
pub struct LidarTinGriddingTool;
pub struct LidarRadialBasisFunctionInterpolationTool;
pub struct LidarSibsonInterpolationTool;
pub struct LidarBlockMaximumTool;
pub struct LidarBlockMinimumTool;
pub struct LidarPointDensityTool;
pub struct LidarDigitalSurfaceModelTool;
pub struct LidarHillshadeTool;
pub struct FilterLidarClassesTool;
pub struct LidarShiftTool;
pub struct RemoveDuplicatesTool;
pub struct FilterLidarScanAnglesTool;
pub struct FilterLidarNoiseTool;
pub struct LidarThinTool;
pub struct LidarElevationSliceTool;
pub struct LidarJoinTool;
pub struct LidarThinHighDensityTool;
pub struct LidarTileTool;
pub struct SortLidarTool;
pub struct FilterLidarByPercentileTool;
pub struct SplitLidarTool;
pub struct LidarRemoveOutliersTool;
pub struct NormalizeLidarTool;
pub struct HeightAboveGroundTool;
pub struct LidarGroundPointFilterTool;
pub struct FilterLidarTool;
pub struct ModifyLidarTool;
pub struct FilterLidarByReferenceSurfaceTool;
pub struct ClassifyLidarTool;
pub struct LidarClassifySubsetTool;
pub struct ClipLidarToPolygonTool;
pub struct ErasePolygonFromLidarTool;
pub struct ClassifyOverlapPointsTool;
pub struct LidarSegmentationTool;
pub struct IndividualTreeSegmentationTool;
pub struct IndividualTreeDetectionTool;
pub struct LidarSegmentationBasedFilterTool;
pub struct LidarColourizeTool;
pub struct ColourizeBasedOnClassTool;
pub struct ColourizeBasedOnPointReturnsTool;
pub struct ClassifyBuildingsInLidarTool;
pub struct AsciiToLasTool;
pub struct LasToAsciiTool;
pub struct SelectTilesByPolygonTool;
pub struct LidarContourTool;
pub struct LidarTileFootprintTool;
pub struct LasToShapefileTool;
pub struct LidarConstructVectorTinTool;
pub struct LidarHexBinTool;
pub struct LidarPointReturnAnalysisTool;
pub struct LidarInfoTool;
pub struct LidarHistogramTool;
pub struct LidarPointStatsTool;
pub struct FlightlineOverlapTool;
pub struct RecoverFlightlineInfoTool;
pub struct FindFlightlineEdgePointsTool;
pub struct LidarTophatTransformTool;
pub struct NormalVectorsTool;
pub struct LidarKappaTool;
pub struct LidarEigenvalueFeaturesTool;
pub struct LidarRansacPlanesTool;
pub struct LidarRooftopAnalysisTool;

#[derive(Clone, Copy)]
enum ReturnsMode {
    All,
    First,
    Last,
}

/// Check if a file has a valid LiDAR extension (LAS, LAZ, COPC, PLY, E57).
fn is_valid_lidar_extension(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        let name_lower = name.to_string_lossy().to_ascii_lowercase();
        return name_lower.ends_with(".copc.las")
            || name_lower.ends_with(".laz")
            || name_lower.ends_with(".las")
            || name_lower.ends_with(".ply")
            || name_lower.ends_with(".e57");
    }
    false
}

/// Scan the current working directory for valid LiDAR files.
/// Returns sorted list of paths with valid LiDAR extensions.
fn find_lidar_files() -> Result<Vec<PathBuf>, ToolError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ToolError::Execution(format!("failed getting current directory: {e}")))?;
    
    let mut files = Vec::new();
    
    // Scan directory (non-recursive for now)
    for entry in fs::read_dir(&cwd)
        .map_err(|e| ToolError::Execution(format!("failed reading directory: {e}")))? 
    {
        let entry = entry
            .map_err(|e| ToolError::Execution(format!("failed reading directory entry: {e}")))?;
        let path = entry.path();
        
        if path.is_file() && is_valid_lidar_extension(&path) {
            files.push(path);
        }
    }
    
    // Sort for deterministic processing
    files.sort();
    
    if files.is_empty() {
        return Err(ToolError::Execution(
            "no LiDAR files (*.las, *.laz, *.copc.las, *.ply, *.e57) found in current directory".to_string(),
        ));
    }
    
    Ok(files)
}

/// Generate batch output filename from input LiDAR file.
/// For example: input.las -> input_dem.tif (for DSM tool), input_density.tif (for density tool)
fn generate_batch_output_path(input_path: &Path, tool_suffix: &str) -> PathBuf {
    let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
    // Remove .copc suffix if present (so input.copc.las becomes input, not input.copc)
    let stem_clean = if stem.ends_with(".copc") {
        stem.trim_end_matches(".copc").to_string()
    } else {
        stem.to_string()
    };
    
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let filename = format!("{}_{}.tif", stem_clean, tool_suffix);
    parent.join(filename)
}

/// Generate batch output filename for LiDAR outputs.
/// Preserves the input LiDAR extension where practical and defaults to .las.
fn generate_batch_lidar_output_path(input_path: &Path, tool_suffix: &str) -> PathBuf {
    let stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
    // Remove .copc suffix if present (so input.copc.las becomes input, not input.copc)
    let stem_clean = if stem.ends_with(".copc") {
        stem.trim_end_matches(".copc").to_string()
    } else {
        stem.to_string()
    };

    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = input_path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    let ext = if file_name.ends_with(".copc.las") {
        "las".to_string()
    } else {
        input_path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.is_empty())
            .unwrap_or("las")
            .to_ascii_lowercase()
    };

    let filename = format!("{}_{}.{}", stem_clean, tool_suffix, ext);
    parent.join(filename)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RbfPolyOrder {
    None,
    Constant,
    Quadratic,
}

/// Convert LiDAR CRS metadata into raster CRS metadata.
///
/// Phase-1 LiDAR-to-raster tools use this helper so all raster outputs inherit
/// CRS metadata from source point clouds.
#[allow(dead_code)]
pub(crate) fn lidar_crs_to_raster_crs(crs: Option<&LidarCrs>) -> CrsInfo {
    if let Some(crs) = crs {
        if let Some(epsg) = crs.epsg {
            return CrsInfo::from_epsg(epsg);
        }
        if let Some(wkt) = crs.wkt.as_deref() {
            let wkt_trimmed = wkt.trim();
            if !wkt_trimmed.is_empty() {
                return CrsInfo::from_wkt(wkt_trimmed.to_string());
            }
        }
    }
    CrsInfo::default()
}

/// Validate that a LiDAR CRS is present for workflows that require CRS-aware outputs.
pub(crate) fn require_lidar_crs(crs: Option<&LidarCrs>, tool_name: &str) -> Result<(), ToolError> {
    let has_epsg = crs.and_then(|c| c.epsg).is_some();
    let has_wkt = crs
        .and_then(|c| c.wkt.as_deref())
        .map(|w| !w.trim().is_empty())
        .unwrap_or(false);
    if has_epsg || has_wkt {
        Ok(())
    } else {
        Err(ToolError::Validation(format!(
            "{} requires input LiDAR CRS metadata to assign output CRS",
            tool_name
        )))
    }
}

fn parse_lidar_path_value(value: &Value, param: &str) -> Result<String, ToolError> {
    if let Some(path) = value.as_str() {
        return Ok(path.to_string());
    }
    if let Some(obj) = value.as_object() {
        if obj.get("__wbw_type__").and_then(Value::as_str) == Some("lidar") {
            return obj
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| {
                    ToolError::Validation(format!(
                        "typed lidar '{}' requires string field 'path'",
                        param
                    ))
                });
        }
    }
    Err(ToolError::Validation(format!(
        "parameter '{}' must be a lidar path string or typed lidar object",
        param
    )))
}

fn parse_lidar_path_arg_optional(args: &ToolArgs) -> Result<Option<String>, ToolError> {
    if let Some(value) = args.get("input") {
        return Ok(Some(parse_lidar_path_value(value, "input")?));
    }
    if let Some(value) = args.get("input_lidar") {
        return Ok(Some(parse_lidar_path_value(value, "input_lidar")?));
    }
    Ok(None)
}

fn parse_required_lidar_path_alias(args: &ToolArgs, names: &[&str], label: &str) -> Result<String, ToolError> {
    for name in names {
        if let Some(value) = args.get(*name) {
            return parse_lidar_path_value(value, name);
        }
    }
    Err(ToolError::Validation(format!("{label} is required")))
}

fn parse_required_vector_path_alias(args: &ToolArgs, names: &[&str], label: &str) -> Result<String, ToolError> {
    for name in names {
        if args.get(*name).is_some() {
            return parse_vector_path_arg(args, name);
        }
    }
    Err(ToolError::Validation(format!("{label} is required")))
}

#[derive(Clone)]
struct PreparedPolygon {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    exterior: Vec<(f64, f64)>,
    interiors: Vec<Vec<(f64, f64)>>,
}

fn ring_to_xy(ring: &wbvector::Ring) -> Vec<(f64, f64)> {
    ring.0.iter().map(|c| (c.x, c.y)).collect()
}

fn push_polygon(exterior: &wbvector::Ring, interiors: &[wbvector::Ring], out: &mut Vec<PreparedPolygon>) {
    let ext = ring_to_xy(exterior);
    if ext.len() < 3 {
        return;
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in &ext {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let holes = interiors.iter().map(ring_to_xy).collect();
    out.push(PreparedPolygon {
        min_x,
        max_x,
        min_y,
        max_y,
        exterior: ext,
        interiors: holes,
    });
}

fn collect_polygons_from_geometry(geom: &wbvector::Geometry, out: &mut Vec<PreparedPolygon>) {
    match geom {
        wbvector::Geometry::Polygon { exterior, interiors } => push_polygon(exterior, interiors, out),
        wbvector::Geometry::MultiPolygon(polys) => {
            for (exterior, interiors) in polys {
                push_polygon(exterior, interiors, out);
            }
        }
        wbvector::Geometry::GeometryCollection(geoms) => {
            for g in geoms {
                collect_polygons_from_geometry(g, out);
            }
        }
        _ => {}
    }
}

fn read_prepared_polygons(path: &str) -> Result<Vec<PreparedPolygon>, ToolError> {
    let layer = wbvector::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading polygon vector '{}': {e}", path)))?;
    let mut polys = Vec::new();
    for feature in &layer.features {
        if let Some(geom) = feature.geometry.as_ref() {
            collect_polygons_from_geometry(geom, &mut polys);
        }
    }
    if polys.is_empty() {
        return Err(ToolError::Validation(
            "polygons input must contain at least one polygon geometry".to_string(),
        ));
    }
    Ok(polys)
}

fn point_in_prepared_polygon(x: f64, y: f64, poly: &PreparedPolygon) -> bool {
    if x < poly.min_x || x > poly.max_x || y < poly.min_y || y > poly.max_y {
        return false;
    }
    if !point_in_polygon_2d((x, y), &poly.exterior) {
        return false;
    }
    for hole in &poly.interiors {
        if point_in_polygon_2d((x, y), hole) {
            return false;
        }
    }
    true
}

fn point_in_any_prepared_polygon(x: f64, y: f64, polys: &[PreparedPolygon]) -> bool {
    polys.iter().any(|poly| point_in_prepared_polygon(x, y, poly))
}

fn parse_batch_neighbor_inputs(args: &ToolArgs) -> Result<Vec<String>, ToolError> {
    let Some(value) = args.get("batch_neighbor_inputs") else {
        return Ok(Vec::new());
    };
    let Some(arr) = value.as_array() else {
        return Err(ToolError::Validation(
            "batch_neighbor_inputs must be an array of path strings".to_string(),
        ));
    };
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            return Err(ToolError::Validation(
                "batch_neighbor_inputs entries must be path strings".to_string(),
            ));
        };
        out.push(s.to_string());
    }
    Ok(out)
}

fn parse_f64_alias(args: &ToolArgs, names: &[&str], default: f64) -> f64 {
    for name in names {
        if let Some(value) = args.get(*name).and_then(Value::as_f64) {
            return value;
        }
    }
    default
}

fn parse_bool_alias(args: &ToolArgs, names: &[&str], default: bool) -> bool {
    for name in names {
        if let Some(value) = args.get(*name).and_then(Value::as_bool) {
            return value;
        }
    }
    default
}

fn color8_to_rgb16(r: u8, g: u8, b: u8) -> Rgb16 {
    Rgb16 {
        red: u16::from(r) * 257,
        green: u16::from(g) * 257,
        blue: u16::from(b) * 257,
    }
}

fn parse_rgb_spec(spec: &str) -> Result<Rgb16, ToolError> {
    let s = spec.trim();
    if s.is_empty() {
        return Err(ToolError::Validation("colour specification cannot be empty".to_string()));
    }

    if let Some(hex) = s.strip_prefix('#').or_else(|| s.strip_prefix("0x")).or_else(|| s.strip_prefix("0X")) {
        if hex.len() != 6 {
            return Err(ToolError::Validation(format!("invalid hex colour '{}': expected 6 hex digits", s)));
        }
        let v = u32::from_str_radix(hex, 16)
            .map_err(|_| ToolError::Validation(format!("invalid hex colour '{}': expected 6 hex digits", s)))?;
        let r = ((v >> 16) & 0xFF) as u8;
        let g = ((v >> 8) & 0xFF) as u8;
        let b = (v & 0xFF) as u8;
        return Ok(color8_to_rgb16(r, g, b));
    }

    let cleaned = s.replace('(', "").replace(')', "").replace(' ', "");
    let parts: Vec<&str> = cleaned.split(',').filter(|p| !p.is_empty()).collect();
    if parts.len() != 3 {
        return Err(ToolError::Validation(format!("invalid rgb colour '{}': expected (r,g,b)", s)));
    }
    let r = parts[0]
        .parse::<u16>()
        .map_err(|_| ToolError::Validation(format!("invalid red channel in '{}'", s)))?
        .min(255) as u8;
    let g = parts[1]
        .parse::<u16>()
        .map_err(|_| ToolError::Validation(format!("invalid green channel in '{}'", s)))?
        .min(255) as u8;
    let b = parts[2]
        .parse::<u16>()
        .map_err(|_| ToolError::Validation(format!("invalid blue channel in '{}'", s)))?
        .min(255) as u8;
    Ok(color8_to_rgb16(r, g, b))
}

fn blend_rgb_with_intensity(base: Rgb16, intensity: u16, blend: f64) -> Rgb16 {
    let blend = blend.clamp(0.0, 1.0);
    let i = (f64::from(intensity) / 65535.0) * 255.0;
    let br = f64::from(base.red) / 257.0;
    let bg = f64::from(base.green) / 257.0;
    let bb = f64::from(base.blue) / 257.0;
    let r = ((1.0 - blend) * br + blend * i).round().clamp(0.0, 255.0) as u8;
    let g = ((1.0 - blend) * bg + blend * i).round().clamp(0.0, 255.0) as u8;
    let b = ((1.0 - blend) * bb + blend * i).round().clamp(0.0, 255.0) as u8;
    color8_to_rgb16(r, g, b)
}

fn default_class_palette() -> [Rgb16; 19] {
    [
        color8_to_rgb16(200, 200, 200),
        color8_to_rgb16(255, 255, 255),
        color8_to_rgb16(230, 214, 170),
        color8_to_rgb16(91, 255, 48),
        color8_to_rgb16(41, 199, 0),
        color8_to_rgb16(25, 120, 0),
        color8_to_rgb16(255, 0, 0),
        color8_to_rgb16(255, 213, 0),
        color8_to_rgb16(255, 255, 255),
        color8_to_rgb16(0, 153, 255),
        color8_to_rgb16(150, 150, 150),
        color8_to_rgb16(150, 150, 150),
        color8_to_rgb16(255, 255, 255),
        color8_to_rgb16(255, 255, 255),
        color8_to_rgb16(0, 0, 255),
        color8_to_rgb16(255, 255, 0),
        color8_to_rgb16(50, 50, 50),
        color8_to_rgb16(100, 100, 100),
        color8_to_rgb16(255, 213, 0),
    ]
}

fn parse_returns_mode(args: &ToolArgs) -> ReturnsMode {
    let text = args
        .get("returns")
        .or_else(|| args.get("returns_included"))
        .and_then(Value::as_str)
        .unwrap_or("all")
        .to_lowercase();
    if text.contains("first") {
        ReturnsMode::First
    } else if text.contains("last") {
        ReturnsMode::Last
    } else {
        ReturnsMode::All
    }
}

fn parse_excluded_classes(args: &ToolArgs) -> Result<[bool; 256], ToolError> {
    let mut include = [true; 256];
    let raw = if let Some(v) = args.get("excluded_classes") {
        Some(v)
    } else {
        args.get("exclude_cls")
    };
    let Some(raw) = raw else {
        return Ok(include);
    };

    if let Some(arr) = raw.as_array() {
        for v in arr {
            let Some(class_u64) = v.as_u64() else {
                return Err(ToolError::Validation(
                    "excluded_classes must contain integer values".to_string(),
                ));
            };
            if class_u64 < 256 {
                include[class_u64 as usize] = false;
            }
        }
        return Ok(include);
    }

    if let Some(text) = raw.as_str() {
        for token in text.split(',') {
            let t = token.trim();
            if t.is_empty() {
                continue;
            }
            let class_u64: u64 = t.parse().map_err(|_| {
                ToolError::Validation(format!(
                    "failed parsing excluded class '{}' as integer",
                    t
                ))
            })?;
            if class_u64 < 256 {
                include[class_u64 as usize] = false;
            }
        }
        return Ok(include);
    }

    Err(ToolError::Validation(
        "excluded_classes/exclude_cls must be an array of integers or a comma-delimited string"
            .to_string(),
    ))
}

fn parse_include_classes_arg(
    args: &ToolArgs,
    names: &[&str],
    default_classes: &[u8],
) -> Result<[bool; 256], ToolError> {
    let mut include = [false; 256];
    for c in default_classes {
        include[*c as usize] = true;
    }

    for name in names {
        let Some(raw) = args.get(*name) else {
            continue;
        };
        include = [false; 256];

        if let Some(arr) = raw.as_array() {
            for v in arr {
                let Some(class_u64) = v.as_u64() else {
                    return Err(ToolError::Validation(format!(
                        "{} must contain integer class values",
                        name
                    )));
                };
                if class_u64 < 256 {
                    include[class_u64 as usize] = true;
                }
            }
            return Ok(include);
        }

        if let Some(text) = raw.as_str() {
            for token in text.split(',') {
                let t = token.trim();
                if t.is_empty() {
                    continue;
                }
                let class_u64: u64 = t.parse().map_err(|_| {
                    ToolError::Validation(format!("failed parsing class value '{}' as integer", t))
                })?;
                if class_u64 < 256 {
                    include[class_u64 as usize] = true;
                }
            }
            return Ok(include);
        }

        return Err(ToolError::Validation(format!(
            "{} must be an array of class integers or comma-delimited class string",
            name
        )));
    }

    Ok(include)
}

fn parse_usize_alias(args: &ToolArgs, names: &[&str], default: usize) -> usize {
    for name in names {
        if let Some(value) = args.get(*name).and_then(Value::as_u64) {
            return value as usize;
        }
    }
    default
}

fn parse_string_alias<'a>(args: &'a ToolArgs, names: &[&str], default: &'a str) -> &'a str {
    for name in names {
        if let Some(value) = args.get(*name).and_then(Value::as_str) {
            return value;
        }
    }
    default
}

fn segment_color_from_id(segment_id: usize, seed: u64) -> Rgb16 {
    let mut x = (segment_id as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(seed ^ 0xA5A5_A5A5_A5A5_A5A5);
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    let r = ((x & 0xFF) as u8).max(32);
    let g = (((x >> 8) & 0xFF) as u8).max(32);
    let b = (((x >> 16) & 0xFF) as u8).max(32);
    color8_to_rgb16(r, g, b)
}

fn derive_sidecar_csv_path(output_lidar_path: &Path) -> PathBuf {
    let parent = output_lidar_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = output_lidar_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("segmented");
    parent.join(format!("{}_segments.csv", stem))
}

fn shift_mode_for_seed(
    seed_idx: usize,
    xs: &[f64],
    ys: &[f64],
    zs: &[f64],
    tree: &KdTree<f64, usize, [f64; 2]>,
    h_xy: f64,
    h_z: f64,
    max_iterations: usize,
    tol: f64,
    simd: bool,
) -> [f64; 3] {
    let mut mx = xs[seed_idx];
    let mut my = ys[seed_idx];
    let mut mz = zs[seed_idx];
    let hxy2 = (h_xy * h_xy).max(f64::EPSILON);
    let hz2 = (h_z * h_z).max(f64::EPSILON);
    let radius_sq = hxy2;

    for _ in 0..max_iterations {
        let neighbors = tree
            .within(&[mx, my], radius_sq, &squared_euclidean)
            .unwrap_or_default();
        if neighbors.is_empty() {
            break;
        }

        let mut sw = 0.0;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;

        if simd {
            let mut i = 0usize;
            while i + 4 <= neighbors.len() {
                let i0 = *neighbors[i].1;
                let i1 = *neighbors[i + 1].1;
                let i2 = *neighbors[i + 2].1;
                let i3 = *neighbors[i + 3].1;

                let xv = f64x4::new([xs[i0], xs[i1], xs[i2], xs[i3]]);
                let yv = f64x4::new([ys[i0], ys[i1], ys[i2], ys[i3]]);
                let zv = f64x4::new([zs[i0], zs[i1], zs[i2], zs[i3]]);

                let dx = xv - f64x4::splat(mx);
                let dy = yv - f64x4::splat(my);
                let dz = zv - f64x4::splat(mz);
                let dxy2 = dx * dx + dy * dy;
                let dz2 = dz * dz;

                let dxy2_arr = dxy2.as_array();
                let dz2_arr = dz2.as_array();
                let x_arr = xv.as_array();
                let y_arr = yv.as_array();
                let z_arr = zv.as_array();
                for lane in 0..4 {
                    let wxy = (-0.5 * dxy2_arr[lane] / hxy2).exp();
                    let wz = (-0.5 * dz2_arr[lane] / hz2).exp();
                    let w = wxy * wz;
                    sw += w;
                    sx += w * x_arr[lane];
                    sy += w * y_arr[lane];
                    sz += w * z_arr[lane];
                }
                i += 4;
            }

            for (_, idx_ref) in neighbors.iter().skip((neighbors.len() / 4) * 4) {
                let idx = **idx_ref;
                let dx = xs[idx] - mx;
                let dy = ys[idx] - my;
                let dz = zs[idx] - mz;
                let wxy = (-0.5 * (dx * dx + dy * dy) / hxy2).exp();
                let wz = (-0.5 * (dz * dz) / hz2).exp();
                let w = wxy * wz;
                sw += w;
                sx += w * xs[idx];
                sy += w * ys[idx];
                sz += w * zs[idx];
            }
        } else {
            for (_, idx_ref) in &neighbors {
                let idx = **idx_ref;
                let dx = xs[idx] - mx;
                let dy = ys[idx] - my;
                let dz = zs[idx] - mz;
                let wxy = (-0.5 * (dx * dx + dy * dy) / hxy2).exp();
                let wz = (-0.5 * (dz * dz) / hz2).exp();
                let w = wxy * wz;
                sw += w;
                sx += w * xs[idx];
                sy += w * ys[idx];
                sz += w * zs[idx];
            }
        }

        if sw <= f64::EPSILON {
            break;
        }

        let nx = sx / sw;
        let ny = sy / sw;
        let nz = sz / sw;
        let shift = ((nx - mx) * (nx - mx) + (ny - my) * (ny - my) + (nz - mz) * (nz - mz)).sqrt();
        mx = nx;
        my = ny;
        mz = nz;
        if shift < tol {
            break;
        }
    }

    [mx, my, mz]
}

fn refine_mode_exact_from_start(
    start_mode: [f64; 3],
    xs: &[f64],
    ys: &[f64],
    zs: &[f64],
    tree: &KdTree<f64, usize, [f64; 2]>,
    h_xy: f64,
    h_z: f64,
    max_iterations: usize,
    tol: f64,
    simd: bool,
) -> [f64; 3] {
    let mut mx = start_mode[0];
    let mut my = start_mode[1];
    let mut mz = start_mode[2];
    let hxy2 = (h_xy * h_xy).max(f64::EPSILON);
    let hz2 = (h_z * h_z).max(f64::EPSILON);
    let radius_sq = hxy2;

    for _ in 0..max_iterations {
        let neighbors = tree
            .within(&[mx, my], radius_sq, &squared_euclidean)
            .unwrap_or_default();
        if neighbors.is_empty() {
            break;
        }

        let mut sw = 0.0;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;

        if simd {
            let mut i = 0usize;
            while i + 4 <= neighbors.len() {
                let i0 = *neighbors[i].1;
                let i1 = *neighbors[i + 1].1;
                let i2 = *neighbors[i + 2].1;
                let i3 = *neighbors[i + 3].1;

                let xv = f64x4::new([xs[i0], xs[i1], xs[i2], xs[i3]]);
                let yv = f64x4::new([ys[i0], ys[i1], ys[i2], ys[i3]]);
                let zv = f64x4::new([zs[i0], zs[i1], zs[i2], zs[i3]]);

                let dx = xv - f64x4::splat(mx);
                let dy = yv - f64x4::splat(my);
                let dz = zv - f64x4::splat(mz);
                let dxy2 = dx * dx + dy * dy;
                let dz2 = dz * dz;

                let dxy2_arr = dxy2.as_array();
                let dz2_arr = dz2.as_array();
                let x_arr = xv.as_array();
                let y_arr = yv.as_array();
                let z_arr = zv.as_array();
                for lane in 0..4 {
                    let wxy = (-0.5 * dxy2_arr[lane] / hxy2).exp();
                    let wz = (-0.5 * dz2_arr[lane] / hz2).exp();
                    let w = wxy * wz;
                    sw += w;
                    sx += w * x_arr[lane];
                    sy += w * y_arr[lane];
                    sz += w * z_arr[lane];
                }
                i += 4;
            }

            for (_, idx_ref) in neighbors.iter().skip((neighbors.len() / 4) * 4) {
                let idx = **idx_ref;
                let dx = xs[idx] - mx;
                let dy = ys[idx] - my;
                let dz = zs[idx] - mz;
                let wxy = (-0.5 * (dx * dx + dy * dy) / hxy2).exp();
                let wz = (-0.5 * (dz * dz) / hz2).exp();
                let w = wxy * wz;
                sw += w;
                sx += w * xs[idx];
                sy += w * ys[idx];
                sz += w * zs[idx];
            }
        } else {
            for (_, idx_ref) in &neighbors {
                let idx = **idx_ref;
                let dx = xs[idx] - mx;
                let dy = ys[idx] - my;
                let dz = zs[idx] - mz;
                let wxy = (-0.5 * (dx * dx + dy * dy) / hxy2).exp();
                let wz = (-0.5 * (dz * dz) / hz2).exp();
                let w = wxy * wz;
                sw += w;
                sx += w * xs[idx];
                sy += w * ys[idx];
                sz += w * zs[idx];
            }
        }

        if sw <= f64::EPSILON {
            break;
        }

        let nx = sx / sw;
        let ny = sy / sw;
        let nz = sz / sw;
        let shift = ((nx - mx) * (nx - mx) + (ny - my) * (ny - my) + (nz - mz) * (nz - mz)).sqrt();
        mx = nx;
        my = ny;
        mz = nz;
        if shift < tol {
            break;
        }
    }

    [mx, my, mz]
}

fn estimate_adaptive_bandwidth_for_seed(
    seed_idx: usize,
    xs: &[f64],
    ys: &[f64],
    zs: &[f64],
    tree: &KdTree<f64, usize, [f64; 2]>,
    bandwidth_min: f64,
    bandwidth_max: f64,
    vertical_bandwidth: f64,
    adaptive_neighbors: usize,
    adaptive_sector_count: usize,
) -> f64 {
    let sx = xs[seed_idx];
    let sy = ys[seed_idx];
    let sz = zs[seed_idx];
    let radius_sq = bandwidth_max * bandwidth_max;
    let neighbors = tree
        .within(&[sx, sy], radius_sq, &squared_euclidean)
        .unwrap_or_default();
    if neighbors.len() <= 2 {
        return bandwidth_min;
    }

    let mut radial_distances = Vec::with_capacity(neighbors.len().saturating_sub(1));
    let sector_count = adaptive_sector_count.max(1);
    let mut sector_nearest = vec![f64::INFINITY; sector_count];
    let dz_limit = (vertical_bandwidth * 2.0).max(0.5);

    for (_, idx_ref) in neighbors {
        let idx = *idx_ref;
        if idx == seed_idx {
            continue;
        }
        let dx = xs[idx] - sx;
        let dy = ys[idx] - sy;
        let dist = (dx * dx + dy * dy).sqrt();
        if !dist.is_finite() || dist <= f64::EPSILON {
            continue;
        }
        radial_distances.push(dist);

        if (zs[idx] - sz).abs() <= dz_limit {
            let mut sector = (((dy.atan2(dx) + PI) / (2.0 * PI)) * (sector_count as f64)).floor() as usize;
            if sector >= sector_count {
                sector = sector_count - 1;
            }
            if dist < sector_nearest[sector] {
                sector_nearest[sector] = dist;
            }
        }
    }

    if radial_distances.is_empty() {
        return bandwidth_min;
    }
    radial_distances.sort_by(|a, b| a.total_cmp(b));
    let knn_index = adaptive_neighbors
        .saturating_sub(1)
        .min(radial_distances.len() - 1);
    let density_scale = (radial_distances[knn_index] * 1.75).clamp(bandwidth_min, bandwidth_max);

    let mut sector_samples: Vec<f64> = sector_nearest
        .into_iter()
        .filter(|d| d.is_finite())
        .collect();
    if sector_samples.len() >= (sector_count / 4).max(3) {
        sector_samples.sort_by(|a, b| a.total_cmp(b));
        let pct60_idx = (((sector_samples.len() - 1) as f64) * 0.6).round() as usize;
        let sector_scale = (sector_samples[pct60_idx] * 1.2).clamp(bandwidth_min, bandwidth_max);
        (density_scale * 0.55 + sector_scale * 0.45).clamp(bandwidth_min, bandwidth_max)
    } else {
        density_scale
    }
}

#[derive(Clone, Copy)]
struct MeanShiftGridCell {
    cx: f64,
    cy: f64,
    cz: f64,
    count: f64,
}

fn grid_cell_index(x: f64, y: f64, cell_size: f64) -> (i32, i32) {
    let inv = 1.0 / cell_size;
    ((x * inv).floor() as i32, (y * inv).floor() as i32)
}

fn build_meanshift_grid(
    xs: &[f64],
    ys: &[f64],
    zs: &[f64],
    cell_size: f64,
) -> HashMap<(i32, i32), MeanShiftGridCell> {
    let mut accum: HashMap<(i32, i32), (f64, f64, f64, f64)> = HashMap::new();
    for i in 0..xs.len() {
        let key = grid_cell_index(xs[i], ys[i], cell_size);
        let entry = accum.entry(key).or_insert((0.0, 0.0, 0.0, 0.0));
        entry.0 += xs[i];
        entry.1 += ys[i];
        entry.2 += zs[i];
        entry.3 += 1.0;
    }

    let mut grid = HashMap::with_capacity(accum.len());
    for (key, (sx, sy, sz, c)) in accum {
        let inv_c = if c > 0.0 { 1.0 / c } else { 0.0 };
        grid.insert(
            key,
            MeanShiftGridCell {
                cx: sx * inv_c,
                cy: sy * inv_c,
                cz: sz * inv_c,
                count: c,
            },
        );
    }
    grid
}

fn shift_mode_for_seed_grid(
    seed_idx: usize,
    xs: &[f64],
    ys: &[f64],
    zs: &[f64],
    grid: &HashMap<(i32, i32), MeanShiftGridCell>,
    grid_cell_size: f64,
    h_xy: f64,
    h_z: f64,
    max_iterations: usize,
    tol: f64,
) -> [f64; 3] {
    let mut mx = xs[seed_idx];
    let mut my = ys[seed_idx];
    let mut mz = zs[seed_idx];
    let hxy2 = (h_xy * h_xy).max(f64::EPSILON);
    let hz2 = (h_z * h_z).max(f64::EPSILON);
    let cell_radius = (h_xy / grid_cell_size).ceil().max(1.0) as i32;

    for _ in 0..max_iterations {
        let (cx, cy) = grid_cell_index(mx, my, grid_cell_size);
        let mut sw = 0.0;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;

        for yy in (cy - cell_radius)..=(cy + cell_radius) {
            for xx in (cx - cell_radius)..=(cx + cell_radius) {
                let Some(cell) = grid.get(&(xx, yy)) else {
                    continue;
                };
                let dx = cell.cx - mx;
                let dy = cell.cy - my;
                let dz = cell.cz - mz;
                let dxy2 = dx * dx + dy * dy;
                if dxy2 > hxy2 {
                    continue;
                }
                let wxy = (-0.5 * dxy2 / hxy2).exp();
                let wz = (-0.5 * (dz * dz) / hz2).exp();
                let w = wxy * wz * cell.count;
                sw += w;
                sx += w * cell.cx;
                sy += w * cell.cy;
                sz += w * cell.cz;
            }
        }

        if sw <= f64::EPSILON {
            break;
        }

        let nx = sx / sw;
        let ny = sy / sw;
        let nz = sz / sw;
        let shift = ((nx - mx) * (nx - mx) + (ny - my) * (ny - my) + (nz - mz) * (nz - mz)).sqrt();
        mx = nx;
        my = ny;
        mz = nz;
        if shift < tol {
            break;
        }
    }

    [mx, my, mz]
}

fn parse_optional_lidar_output_path(args: &ToolArgs) -> Result<Option<PathBuf>, ToolError> {
    let Some(value) = args.get("output") else {
        return Ok(None);
    };
    let Some(text) = value.as_str() else {
        return Err(ToolError::Validation(
            "output must be a path string".to_string(),
        ));
    };
    let path = PathBuf::from(text);
    if path.as_os_str().is_empty() {
        return Err(ToolError::Validation(
            "output must not be empty".to_string(),
        ));
    }
    Ok(Some(path))
}

fn parse_required_raster_path_alias(args: &ToolArgs, names: &[&str], label: &str) -> Result<String, ToolError> {
    for name in names {
        if let Some(value) = args.get(*name) {
            return parse_raster_path_value(value, name);
        }
    }
    Err(ToolError::Validation(format!("{label} is required")))
}

fn parse_lidar_inputs_arg(args: &ToolArgs) -> Result<Vec<String>, ToolError> {
    let Some(value) = args.get("inputs") else {
        return Err(ToolError::Validation("inputs is required".to_string()));
    };
    let Some(arr) = value.as_array() else {
        return Err(ToolError::Validation("inputs must be an array".to_string()));
    };
    if arr.is_empty() {
        return Err(ToolError::Validation("inputs must not be empty".to_string()));
    }
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let path = parse_lidar_path_value(item, &format!("inputs[{i}]"))?;
        out.push(path);
    }
    Ok(out)
}

fn parse_ascii_inputs_arg(args: &ToolArgs) -> Result<Vec<String>, ToolError> {
    let value = args
        .get("inputs")
        .or_else(|| args.get("input_ascii_files"))
        .ok_or_else(|| ToolError::Validation("inputs is required".to_string()))?;
    let arr = value
        .as_array()
        .ok_or_else(|| ToolError::Validation("inputs must be an array of path strings".to_string()))?;
    if arr.is_empty() {
        return Err(ToolError::Validation("inputs must not be empty".to_string()));
    }
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let Some(path) = item.as_str() else {
            return Err(ToolError::Validation(format!("inputs[{i}] must be a path string")));
        };
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(ToolError::Validation(format!("inputs[{i}] must not be empty")));
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

#[derive(Clone, Copy)]
struct AsciiPatternSpec {
    x_idx: usize,
    y_idx: usize,
    z_idx: usize,
    i_idx: Option<usize>,
    c_idx: Option<usize>,
    rn_idx: Option<usize>,
    nr_idx: Option<usize>,
    time_idx: Option<usize>,
    sa_idx: Option<usize>,
    r_idx: Option<usize>,
    g_idx: Option<usize>,
    b_idx: Option<usize>,
    field_count: usize,
}

fn parse_ascii_pattern(pattern: &str) -> Result<AsciiPatternSpec, ToolError> {
    let parts: Vec<String> = pattern
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(ToolError::Validation(
            "pattern must not be empty (e.g. 'x,y,z,i,c,rn,nr,sa')".to_string(),
        ));
    }

    let mut spec = AsciiPatternSpec {
        x_idx: usize::MAX,
        y_idx: usize::MAX,
        z_idx: usize::MAX,
        i_idx: None,
        c_idx: None,
        rn_idx: None,
        nr_idx: None,
        time_idx: None,
        sa_idx: None,
        r_idx: None,
        g_idx: None,
        b_idx: None,
        field_count: parts.len(),
    };

    for (idx, token) in parts.iter().enumerate() {
        match token.as_str() {
            "x" => spec.x_idx = idx,
            "y" => spec.y_idx = idx,
            "z" => spec.z_idx = idx,
            "i" => spec.i_idx = Some(idx),
            "c" => spec.c_idx = Some(idx),
            "rn" => spec.rn_idx = Some(idx),
            "nr" => spec.nr_idx = Some(idx),
            "time" => spec.time_idx = Some(idx),
            "sa" => spec.sa_idx = Some(idx),
            "r" => spec.r_idx = Some(idx),
            "g" => spec.g_idx = Some(idx),
            "b" => spec.b_idx = Some(idx),
            other => {
                return Err(ToolError::Validation(format!(
                    "unrecognized pattern token '{}'; expected x,y,z,i,c,rn,nr,time,sa,r,g,b",
                    other
                )));
            }
        }
    }

    if spec.x_idx == usize::MAX || spec.y_idx == usize::MAX || spec.z_idx == usize::MAX {
        return Err(ToolError::Validation(
            "pattern must contain x, y, and z fields".to_string(),
        ));
    }
    if spec.rn_idx.is_some() ^ spec.nr_idx.is_some() {
        return Err(ToolError::Validation(
            "pattern fields rn and nr must either both be present or both omitted".to_string(),
        ));
    }
    let rgb_count = usize::from(spec.r_idx.is_some())
        + usize::from(spec.g_idx.is_some())
        + usize::from(spec.b_idx.is_some());
    if rgb_count != 0 && rgb_count != 3 {
        return Err(ToolError::Validation(
            "if any of r/g/b are provided, all r, g, and b must be provided".to_string(),
        ));
    }

    Ok(spec)
}

fn split_ascii_line(line: &str) -> Vec<&str> {
    if line.contains(',') {
        line.split(',').map(str::trim).filter(|s| !s.is_empty()).collect()
    } else {
        line.split_whitespace().collect()
    }
}

fn parse_field<T: std::str::FromStr>(
    fields: &[&str],
    idx: usize,
    field_name: &str,
    line_num: usize,
    input_path: &Path,
) -> Result<T, ToolError> {
    let raw = fields
        .get(idx)
        .ok_or_else(|| {
            ToolError::Execution(format!(
                "failed parsing '{}': line {} missing field '{}'",
                input_path.to_string_lossy(),
                line_num,
                field_name
            ))
        })?
        .trim();
    raw.parse::<T>().map_err(|_| {
        ToolError::Execution(format!(
            "failed parsing '{}': line {} invalid '{}' value '{}'",
            input_path.to_string_lossy(),
            line_num,
            field_name,
            raw
        ))
    })
}

fn derived_las_output_from_ascii(input: &Path, output_dir: Option<&Path>) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ascii_points");
    let parent = output_dir
        .map(Path::to_path_buf)
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    parent.join(format!("{stem}.las"))
}

fn derived_ascii_output_from_lidar(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("lidar_points")
        .trim_end_matches(".copc");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.csv"))
}

fn write_point_cloud_as_csv(cloud: &PointCloud, output_path: &Path) -> Result<(), ToolError> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
        }
    }

    let has_time = cloud.points.iter().any(|p| p.gps_time.is_some());
    let has_rgb = cloud.points.iter().any(|p| p.color.is_some());

    let file = File::create(output_path).map_err(|e| {
        ToolError::Execution(format!(
            "failed creating output csv '{}': {e}",
            output_path.to_string_lossy()
        ))
    })?;
    let mut writer = BufWriter::new(file);

    if !has_rgb && !has_time {
        writeln!(writer, "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE")
            .map_err(|e| ToolError::Execution(format!("failed writing csv header: {e}")))?;
    } else if !has_rgb && has_time {
        writeln!(writer, "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE,TIME")
            .map_err(|e| ToolError::Execution(format!("failed writing csv header: {e}")))?;
    } else if has_rgb && !has_time {
        writeln!(writer, "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE,RED,GREEN,BLUE")
            .map_err(|e| ToolError::Execution(format!("failed writing csv header: {e}")))?;
    } else {
        writeln!(
            writer,
            "X,Y,Z,INTENSITY,CLASS,RETURN,NUM_RETURN,SCAN_ANGLE,TIME,RED,GREEN,BLUE"
        )
        .map_err(|e| ToolError::Execution(format!("failed writing csv header: {e}")))?;
    }

    for p in &cloud.points {
        let time = p.gps_time.map(|t| t.0);
        let clr = p.color;
        match (has_time, has_rgb, time, clr) {
            (false, false, _, _) => {
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{}",
                    p.x,
                    p.y,
                    p.z,
                    p.intensity,
                    p.classification,
                    p.return_number,
                    p.number_of_returns,
                    p.scan_angle
                )
                .map_err(|e| ToolError::Execution(format!("failed writing csv row: {e}")))?;
            }
            (true, false, t, _) => {
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{}",
                    p.x,
                    p.y,
                    p.z,
                    p.intensity,
                    p.classification,
                    p.return_number,
                    p.number_of_returns,
                    p.scan_angle,
                    t.unwrap_or(0.0)
                )
                .map_err(|e| ToolError::Execution(format!("failed writing csv row: {e}")))?;
            }
            (false, true, _, c) => {
                let c = c.unwrap_or(Rgb16 {
                    red: 0,
                    green: 0,
                    blue: 0,
                });
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{},{},{}",
                    p.x,
                    p.y,
                    p.z,
                    p.intensity,
                    p.classification,
                    p.return_number,
                    p.number_of_returns,
                    p.scan_angle,
                    c.red,
                    c.green,
                    c.blue
                )
                .map_err(|e| ToolError::Execution(format!("failed writing csv row: {e}")))?;
            }
            (true, true, t, c) => {
                let c = c.unwrap_or(Rgb16 {
                    red: 0,
                    green: 0,
                    blue: 0,
                });
                writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{},{},{},{},{}",
                    p.x,
                    p.y,
                    p.z,
                    p.intensity,
                    p.classification,
                    p.return_number,
                    p.number_of_returns,
                    p.scan_angle,
                    t.unwrap_or(0.0),
                    c.red,
                    c.green,
                    c.blue
                )
                .map_err(|e| ToolError::Execution(format!("failed writing csv row: {e}")))?;
            }
        }
    }

    writer
        .flush()
        .map_err(|e| ToolError::Execution(format!("failed flushing csv writer: {e}")))
}

fn ensure_html_or_txt(path: &Path) -> Result<(), ToolError> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "html" || ext == "htm" || ext == "txt" || ext.is_empty() {
        Ok(())
    } else {
        Err(ToolError::Validation(
            "output path must use .html, .htm, or .txt extension".to_string(),
        ))
    }
}

fn parse_histogram_parameter(text: &str) -> &'static str {
    let t = text.to_ascii_lowercase();
    if t.contains("intensity") {
        "intensity"
    } else if t.contains("scan") {
        "scan_angle"
    } else if t.contains("class") {
        "class"
    } else if t.contains("time") {
        "time"
    } else {
        "elevation"
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f64 * qq).round() as usize;
    sorted[idx]
}

fn default_output_sibling_path(input: &Path, suffix: &str, ext: &str) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .trim_end_matches(".copc");
    parent.join(format!("{stem}_{suffix}.{ext}"))
}

#[derive(Clone, Copy, Debug, Default)]
struct Plane {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
}

impl Plane {
    fn zero() -> Self { Self::default() }

    fn from_points(points: &[Vector3<f64>]) -> Self {
        let Some((normal, centroid)) = plane_normal_and_centroid(points) else {
            return Self::zero();
        };
        Self {
            a: normal.x,
            b: normal.y,
            c: normal.z,
            d: -(normal.x * centroid.x + normal.y * centroid.y + normal.z * centroid.z),
        }
    }

    fn residual(&self, point: &Vector3<f64>) -> f64 {
        let denom = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        if denom <= 0.0 {
            return f64::INFINITY;
        }
        (self.a * point.x + self.b * point.y + self.c * point.z + self.d).abs() / denom
    }

    fn slope(&self) -> f64 {
        let denom = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        if denom <= 0.0 {
            return 90.0;
        }
        (self.c.abs() / denom).acos().to_degrees()
    }

}

#[derive(Clone, Copy, Debug)]
struct NeighborhoodPca {
    lambda1: f32,
    lambda2: f32,
    lambda3: f32,
    normal: Vector3<f64>,
    slope: f32,
    residual: f32,
}

fn point_to_vec3(p: &PointRecord) -> Vector3<f64> {
    Vector3::new(p.x, p.y, p.z)
}

fn estimate_nominal_spacing(cloud: &PointCloud) -> f64 {
    if cloud.points.len() < 2 {
        return 1.0;
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &cloud.points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let area = ((max_x - min_x).abs() * (max_y - min_y).abs()).max(1.0e-9);
    (area / cloud.points.len() as f64).sqrt().max(0.5)
}

fn plane_normal_and_centroid(points: &[Vector3<f64>]) -> Option<(Vector3<f64>, Vector3<f64>)> {
    if points.len() < 3 {
        return None;
    }
    let centroid = points.iter().fold(Vector3::new(0.0, 0.0, 0.0), |acc, p| acc + *p)
        / points.len() as f64;

    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut xz = 0.0;
    let mut yy = 0.0;
    let mut yz = 0.0;
    let mut zz = 0.0;
    for p in points {
        let r = *p - centroid;
        xx += r.x * r.x;
        xy += r.x * r.y;
        xz += r.x * r.z;
        yy += r.y * r.y;
        yz += r.y * r.z;
        zz += r.z * r.z;
    }
    let cov = Matrix3::new(xx, xy, xz, xy, yy, yz, xz, yz, zz) / points.len() as f64;
    let se = SymmetricEigen::new(cov);
    let mut vals = vec![
        (se.eigenvalues[0], se.eigenvectors.column(0).into_owned()),
        (se.eigenvalues[1], se.eigenvectors.column(1).into_owned()),
        (se.eigenvalues[2], se.eigenvectors.column(2).into_owned()),
    ];
    vals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let normal = vals[0].1.normalize();
    Some((normal, centroid))
}

fn neighborhood_pca(points: &[Vector3<f64>], center: Vector3<f64>) -> Option<NeighborhoodPca> {
    if points.len() < 8 {
        return None;
    }
    let centroid = points.iter().fold(Vector3::new(0.0, 0.0, 0.0), |acc, p| acc + *p)
        / points.len() as f64;
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut xz = 0.0;
    let mut yy = 0.0;
    let mut yz = 0.0;
    let mut zz = 0.0;
    for p in points {
        let r = *p - centroid;
        xx += r.x * r.x;
        xy += r.x * r.y;
        xz += r.x * r.z;
        yy += r.y * r.y;
        yz += r.y * r.z;
        zz += r.z * r.z;
    }
    let cov = Matrix3::new(xx, xy, xz, xy, yy, yz, xz, yz, zz) / points.len() as f64;
    let se = SymmetricEigen::new(cov);
    let mut vals = vec![
        (se.eigenvalues[0], se.eigenvectors.column(0).into_owned()),
        (se.eigenvalues[1], se.eigenvectors.column(1).into_owned()),
        (se.eigenvalues[2], se.eigenvectors.column(2).into_owned()),
    ];
    vals.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    let lambda1 = vals[0].0.max(0.0) as f32;
    let lambda2 = vals[1].0.max(0.0) as f32;
    let lambda3 = vals[2].0.max(0.0) as f32;
    let normal = vals[2].1.normalize();
    let slope = (normal.z.abs().clamp(0.0, 1.0)).acos().to_degrees() as f32;
    let residual = ((center - centroid).dot(&normal)).abs() as f32;
    Some(NeighborhoodPca {
        lambda1,
        lambda2,
        lambda3,
        normal,
        slope,
        residual,
    })
}

fn rgb_from_unit_normal(normal: Vector3<f64>) -> Rgb16 {
    let encode = |v: f64| -> u16 {
        (((v.clamp(-1.0, 1.0) + 1.0) * 0.5 * 255.0).round() as u16) * 257
    };
    Rgb16 {
        red: encode(normal.x),
        green: encode(normal.y),
        blue: encode(normal.z),
    }
}

fn polygon_area(ring: &[wbvector::Coord]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..ring.len() {
        let p1 = &ring[i];
        let p2 = &ring[(i + 1) % ring.len()];
        area += p1.x * p2.y - p2.x * p1.y;
    }
    0.5 * area.abs()
}

fn lidar_bbox_sample_points(cloud: &PointCloud) -> Option<[(f64, f64); 9]> {
    if cloud.points.is_empty() {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &cloud.points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let mid_x = 0.5 * (min_x + max_x);
    let mid_y = 0.5 * (min_y + max_y);
    Some([
        (min_x, min_y),
        (min_x, max_y),
        (max_x, min_y),
        (max_x, max_y),
        (mid_x, mid_y),
        (mid_x, min_y),
        (mid_x, max_y),
        (min_x, mid_y),
        (max_x, mid_y),
    ])
}

fn point_is_withheld(p: &PointRecord) -> bool {
    (p.flags & 0b0000_0100) != 0
}

fn point_is_noise(p: &PointRecord) -> bool {
    p.classification == 7 || p.classification == 18
}

fn sample_dtm_elevation(dtm: &Raster, x: f64, y: f64) -> Option<f64> {
    let (col, row) = dtm.world_to_pixel(x, y)?;
    let z = dtm.get(0, row, col);
    if !dtm.is_nodata(z) {
        return Some(z);
    }
    const OFFSETS: [(isize, isize); 8] = [
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
    ];
    for (dc, dr) in OFFSETS {
        let v = dtm.get(0, row + dr, col + dc);
        if !dtm.is_nodata(v) {
            return Some(v);
        }
    }
    None
}

fn is_late_return(p: &PointRecord) -> bool {
    if p.return_number == 0 || p.number_of_returns == 0 {
        return true;
    }
    p.return_number == p.number_of_returns
}

fn build_filter_context(
    p: &PointRecord,
    point_index: usize,
    n_points: usize,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_z: f64,
    max_z: f64,
) -> Result<HashMapContext, ToolError> {
    let mut ctx = HashMapContext::<DefaultNumericTypes>::new();
    let is_noise = point_is_noise(p);
    let is_withheld = point_is_withheld(p);
    let is_only = p.return_number == 1 && p.number_of_returns == 1;
    let is_multiple = p.number_of_returns > 1;
    let is_early = p.return_number == 1;
    let is_intermediate = p.return_number > 1 && p.return_number < p.number_of_returns;
    let is_late = is_late_return(p);
    let is_first = p.return_number == 1 && p.number_of_returns > 1;
    let is_last = p.return_number == p.number_of_returns && p.number_of_returns > 1;
    let scanner_channel = (p.flags & 0b0000_0011) as i64;
    let is_synthetic = (p.flags & 0b0000_0100) != 0;
    let is_keypoint = (p.flags & 0b0000_1000) != 0;
    let is_overlap = (p.flags & 0b0001_0000) != 0;
    let (red, green, blue) = if let Some(c) = p.color {
        (c.red as f64, c.green as f64, c.blue as f64)
    } else {
        (0.0, 0.0, 0.0)
    };
    let time = p.gps_time.map(|t| t.0).unwrap_or(0.0);
    let n_pts = n_points as f64;

    let vars: [(&str, EvalValue); 41] = [
        ("x", EvalValue::from_float(p.x)),
        ("y", EvalValue::from_float(p.y)),
        ("z", EvalValue::from_float(p.z)),
        ("intensity", EvalValue::from_int(i64::from(p.intensity))),
        ("ret", EvalValue::from_int(i64::from(p.return_number))),
        ("nret", EvalValue::from_int(i64::from(p.number_of_returns))),
        ("is_only", EvalValue::from(is_only)),
        ("is_multiple", EvalValue::from(is_multiple)),
        ("is_early", EvalValue::from(is_early)),
        ("is_intermediate", EvalValue::from(is_intermediate)),
        ("is_late", EvalValue::from(is_late)),
        ("is_first", EvalValue::from(is_first)),
        ("is_last", EvalValue::from(is_last)),
        ("class", EvalValue::from_int(i64::from(p.classification))),
        ("is_noise", EvalValue::from(is_noise)),
        ("is_synthetic", EvalValue::from(is_synthetic)),
        ("is_keypoint", EvalValue::from(is_keypoint)),
        ("is_withheld", EvalValue::from(is_withheld)),
        ("is_overlap", EvalValue::from(is_overlap)),
        ("scan_angle", EvalValue::from_float(f64::from(p.scan_angle))),
        ("scan_direction", EvalValue::from(p.scan_direction_flag)),
        ("is_flightline_edge", EvalValue::from(p.edge_of_flight_line)),
        ("user_data", EvalValue::from_int(i64::from(p.user_data))),
        ("point_source_id", EvalValue::from_int(i64::from(p.point_source_id))),
        ("scanner_channel", EvalValue::from_int(scanner_channel)),
        ("time", EvalValue::from_float(time)),
        ("red", EvalValue::from_int(red as i64)),
        ("green", EvalValue::from_int(green as i64)),
        ("blue", EvalValue::from_int(blue as i64)),
        ("nir", EvalValue::from_int(i64::from(p.nir.unwrap_or(0)))),
        ("pt_num", EvalValue::from_int(point_index as i64)),
        ("n_pts", EvalValue::from_int(n_pts as i64)),
        ("min_x", EvalValue::from_float(min_x)),
        ("mid_x", EvalValue::from_float((min_x + max_x) / 2.0)),
        ("max_x", EvalValue::from_float(max_x)),
        ("min_y", EvalValue::from_float(min_y)),
        ("mid_y", EvalValue::from_float((min_y + max_y) / 2.0)),
        ("max_y", EvalValue::from_float(max_y)),
        ("min_z", EvalValue::from_float(min_z)),
        ("mid_z", EvalValue::from_float((min_z + max_z) / 2.0)),
        ("max_z", EvalValue::from_float(max_z)),
    ];

    for (name, value) in vars {
        let _ = ctx.set_value(name.to_string(), value);
    }
    Ok(ctx)
}

fn parse_modify_statements(text: &str) -> Result<Vec<String>, ToolError> {
    if text.contains("print(") {
        return Err(ToolError::Validation(
            "statement must not contain print() expressions".to_string(),
        ));
    }
    let cleaned = text.replace('\n', ";");
    let parts: Vec<String> = cleaned
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if parts.is_empty() {
        return Err(ToolError::Validation("statement must be non-empty".to_string()));
    }
    Ok(parts)
}

fn parse_assignment_lhs(statement: &str) -> Option<String> {
    let s = statement.trim_start();
    let mut lhs = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            lhs.push(ch);
        } else {
            break;
        }
    }
    if lhs.is_empty() {
        return None;
    }
    let tail = s[lhs.len()..].trim_start();
    let is_assign = tail.starts_with('=')
        || tail.starts_with("+=")
        || tail.starts_with("-=")
        || tail.starts_with("*=")
        || tail.starts_with("/=")
        || tail.starts_with("%=");
    if is_assign { Some(lhs) } else { None }
}

fn is_supported_modify_target(name: &str) -> bool {
    matches!(
        name,
        "x"
            | "y"
            | "z"
            | "xy"
            | "xyz"
            | "intensity"
            | "ret"
            | "nret"
            | "class"
            | "user_data"
            | "point_source_id"
            | "scan_angle"
            | "time"
            | "rgb"
            | "red"
            | "green"
            | "blue"
            | "nir"
            | "is_keypoint"
            | "is_withheld"
            | "is_overlap"
    )
}

fn value_as_f64(ctx: &HashMapContext, key: &str) -> Option<f64> {
    ctx.get_value(key).and_then(|v| v.as_number().ok())
}

fn value_as_i64(ctx: &HashMapContext, key: &str) -> Option<i64> {
    value_as_f64(ctx, key).map(|v| v.round() as i64)
}

fn value_as_bool(ctx: &HashMapContext, key: &str) -> Option<bool> {
    ctx.get_value(key).and_then(|v| v.as_boolean().ok())
}

fn value_as_triplet_f64(ctx: &HashMapContext, key: &str) -> Option<(f64, f64, f64)> {
    let tuple = ctx.get_value(key)?.as_tuple().ok()?;
    if tuple.len() != 3 {
        return None;
    }
    let a = tuple[0].as_number().ok()?;
    let b = tuple[1].as_number().ok()?;
    let c = tuple[2].as_number().ok()?;
    Some((a, b, c))
}

fn value_as_pair_f64(ctx: &HashMapContext, key: &str) -> Option<(f64, f64)> {
    let tuple = ctx.get_value(key)?.as_tuple().ok()?;
    if tuple.len() != 2 {
        return None;
    }
    let a = tuple[0].as_number().ok()?;
    let b = tuple[1].as_number().ok()?;
    Some((a, b))
}

#[derive(Clone, Copy, Debug)]
enum RefSurfaceQueryType {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Within,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverlapCriterion {
    MaxScanAngle,
    NotMinPointSourceId,
    NotMinTime,
    MultiplePointSourceIds,
}

fn parse_overlap_criterion(text: &str) -> OverlapCriterion {
    let t = text.to_ascii_lowercase();
    if t.contains("scan") {
        OverlapCriterion::MaxScanAngle
    } else if t.contains("min point source id") || t.contains("min pt_src_id") {
        OverlapCriterion::NotMinPointSourceId
    } else if t.contains("time") {
        OverlapCriterion::NotMinTime
    } else {
        OverlapCriterion::MultiplePointSourceIds
    }
}

fn parse_ref_surface_query_type(text: &str) -> RefSurfaceQueryType {
    match text.trim().to_lowercase().as_str() {
        "below" | "less" | "less_than" | "less than" | "<" => RefSurfaceQueryType::Less,
        "<=" => RefSurfaceQueryType::LessEqual,
        "above" | "greater" | "greater_than" | "greater than" | ">" => RefSurfaceQueryType::Greater,
        ">=" => RefSurfaceQueryType::GreaterEqual,
        _ => RefSurfaceQueryType::Within,
    }
}

#[derive(Clone, Copy, Debug)]
enum SortCriterion {
    X,
    Y,
    Z,
    Intensity,
    Class,
    UserData,
    PointSourceId,
    ScanAngle,
    ScannerChannel,
    ReturnNumber,
    Time,
}

fn parse_sort_criteria(text: &str) -> Result<Vec<(SortCriterion, Option<f64>)>, ToolError> {
    let raw = text.trim();
    if raw.is_empty() {
        return Err(ToolError::Validation("sort_criteria must not be empty".to_string()));
    }
    let tokens: Vec<&str> = raw
        .split(&[' ', ',', '|', ';', '='][..])
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(ToolError::Validation("sort_criteria must not be empty".to_string()));
    }

    let mut out: Vec<(SortCriterion, Option<f64>)> = Vec::new();
    for token in tokens {
        if let Ok(bin) = token.parse::<f64>() {
            if let Some(last) = out.last_mut() {
                last.1 = Some(bin);
            } else {
                return Err(ToolError::Validation("sort_criteria cannot start with a numeric bin size".to_string()));
            }
            continue;
        }

        let key = token.to_lowercase();
        let criterion = match key.as_str() {
            "x" => SortCriterion::X,
            "y" => SortCriterion::Y,
            "z" => SortCriterion::Z,
            "intensity" => SortCriterion::Intensity,
            "class" | "classification" => SortCriterion::Class,
            "user_data" => SortCriterion::UserData,
            "point_source_id" => SortCriterion::PointSourceId,
            "scan_angle" => SortCriterion::ScanAngle,
            "scanner_channel" => SortCriterion::ScannerChannel,
            "ret_num" | "ret" | "return_number" => SortCriterion::ReturnNumber,
            "time" => SortCriterion::Time,
            _ => {
                return Err(ToolError::Validation(format!(
                    "unrecognized sort criterion '{}'",
                    token
                )));
            }
        };
        out.push((criterion, None));
    }

    if out.is_empty() {
        return Err(ToolError::Validation("sort_criteria must include at least one criterion".to_string()));
    }
    Ok(out)
}

fn sort_value(p: &PointRecord, criterion: SortCriterion) -> f64 {
    match criterion {
        SortCriterion::X => p.x,
        SortCriterion::Y => p.y,
        SortCriterion::Z => p.z,
        SortCriterion::Intensity => p.intensity as f64,
        SortCriterion::Class => p.classification as f64,
        SortCriterion::UserData => p.user_data as f64,
        SortCriterion::PointSourceId => p.point_source_id as f64,
        SortCriterion::ScanAngle => p.scan_angle as f64,
        SortCriterion::ScannerChannel => ((p.flags >> 4) & 0b0000_0011) as f64,
        SortCriterion::ReturnNumber => p.return_number as f64,
        SortCriterion::Time => p.gps_time.map(|t| t.0).unwrap_or(0.0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitCriterion {
    NumPts,
    X,
    Y,
    Z,
    Intensity,
    Class,
    UserData,
    PointSourceId,
    ScanAngle,
    Time,
}

fn parse_split_criterion(text: &str) -> Result<SplitCriterion, ToolError> {
    let key = text.trim().to_lowercase();
    if key.is_empty() {
        return Err(ToolError::Validation("split_criterion must not be empty".to_string()));
    }
    if key.contains("num") {
        Ok(SplitCriterion::NumPts)
    } else if key.contains("x") {
        Ok(SplitCriterion::X)
    } else if key.contains("y") {
        Ok(SplitCriterion::Y)
    } else if key.contains("z") {
        Ok(SplitCriterion::Z)
    } else if key.contains("int") {
        Ok(SplitCriterion::Intensity)
    } else if key.contains("cl") {
        Ok(SplitCriterion::Class)
    } else if key.contains("us") {
        Ok(SplitCriterion::UserData)
    } else if key.contains("id") {
        Ok(SplitCriterion::PointSourceId)
    } else if key.contains("sc") {
        Ok(SplitCriterion::ScanAngle)
    } else if key.contains("ti") {
        Ok(SplitCriterion::Time)
    } else {
        Err(ToolError::Validation(format!("unrecognized split_criterion '{}'", text)))
    }
}

fn split_value(point: &PointRecord, criterion: SplitCriterion) -> f64 {
    match criterion {
        SplitCriterion::X => point.x,
        SplitCriterion::Y => point.y,
        SplitCriterion::Z => point.z,
        SplitCriterion::Intensity => point.intensity as f64,
        SplitCriterion::Class => point.classification as f64,
        SplitCriterion::UserData => point.user_data as f64,
        SplitCriterion::PointSourceId => point.point_source_id as f64,
        SplitCriterion::ScanAngle => point.scan_angle as f64,
        SplitCriterion::Time => point.gps_time.map(|t| t.0).unwrap_or(0.0),
        SplitCriterion::NumPts => 0.0,
    }
}

fn split_output_base(path: &Path) -> Result<(PathBuf, String, String), ToolError> {
    let parent = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ToolError::Validation("input filename is invalid UTF-8".to_string()))?
        .to_string();
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".copc.las") {
        let stem = filename[..filename.len() - 9].to_string();
        return Ok((parent, stem, "copc.las".to_string()));
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ToolError::Validation("input file stem is invalid UTF-8".to_string()))?
        .to_string();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("las")
        .to_string();
    Ok((parent, stem, ext))
}

fn split_output_path(base_dir: &Path, stem: &str, suffix: &str, ext: &str) -> PathBuf {
    base_dir.join(format!("{}_{}.{}", stem, suffix, ext))
}

fn unique_lidar_temp_output_path(tool_suffix: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("wbtools_oss_{}_{}.las", tool_suffix, ts))
}

fn is_withheld(point: &PointRecord) -> bool {
    (point.flags & (1 << 2)) != 0
}

fn return_filter_match(point: &PointRecord, mode: ReturnsMode) -> bool {
    match mode {
        ReturnsMode::All => true,
        ReturnsMode::First => point.return_number <= 1,
        ReturnsMode::Last => {
            let n = point.number_of_returns;
            if n == 0 {
                point.return_number <= 1
            } else {
                point.return_number == n
            }
        }
    }
}

fn select_point_value(point: &PointRecord, parameter: &str) -> Option<f64> {
    match parameter {
        "elevation" | "elev" | "z" => Some(point.z),
        "intensity" => Some(point.intensity as f64),
        "class" | "classification" => Some(point.classification as f64),
        "return_number" | "return number" => Some(point.return_number as f64),
        "number_of_returns" | "num_returns" | "number of returns" => Some(point.number_of_returns as f64),
        "scan_angle" | "scan angle" => Some(point.scan_angle as f64),
        "time" | "gps_time" | "gps time" => Some(point.gps_time.map(|t| t.0).unwrap_or(0.0)),
        "rgb" => point.color.map(|clr| {
            let r = (clr.red >> 8) as u32;
            let g = (clr.green >> 8) as u32;
            let b = (clr.blue >> 8) as u32;
            ((255u32 << 24) | (b << 16) | (g << 8) | r) as f64
        }),
        "user_data" | "user data" => Some(point.user_data as f64),
        _ => None,
    }
}

fn supports_interpolation_parameter(parameter: &str) -> bool {
    matches!(
        parameter,
        "elevation"
            | "elev"
            | "z"
            | "intensity"
            | "class"
            | "classification"
            | "return_number"
            | "return number"
            | "number_of_returns"
            | "num_returns"
            | "number of returns"
            | "scan_angle"
            | "scan angle"
            | "time"
            | "gps_time"
            | "gps time"
            | "rgb"
            | "user_data"
            | "user data"
    )
}

fn collect_lidar_samples(
    points: &[PointRecord],
    parameter: &str,
    returns_mode: ReturnsMode,
    include_classes: &[bool; 256],
    min_z: f64,
    max_z: f64,
) -> Result<Vec<(f64, f64, f64)>, ToolError> {
    let mut samples = Vec::with_capacity(points.len());
    let mut rgb_value_missing_count = 0usize;
    for p in points {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            continue;
        }
        if p.z < min_z || p.z > max_z {
            continue;
        }
        if is_withheld(p) {
            continue;
        }
        if !return_filter_match(p, returns_mode) {
            continue;
        }
        if !include_classes[p.classification as usize] {
            continue;
        }
        if let Some(value) = select_point_value(p, parameter) {
            if value.is_finite() {
                samples.push((p.x, p.y, value));
            }
        } else if parameter == "rgb" {
            rgb_value_missing_count += 1;
        }
    }
    if samples.is_empty() {
        if parameter == "rgb" && rgb_value_missing_count > 0 {
            return Err(ToolError::Validation(
                "interpolation_parameter 'rgb' requires RGB colour values in the input lidar"
                    .to_string(),
            ));
        }
        return Err(ToolError::Validation(
            "input lidar contains no valid points after filtering".to_string(),
        ));
    }
    Ok(samples)
}

impl RbfPolyOrder {
    fn parse(value: Option<&str>) -> Self {
        let text = value.unwrap_or("none").trim().to_ascii_lowercase();
        if text.contains("quad") {
            Self::Quadratic
        } else if text.contains("const") {
            Self::Constant
        } else {
            Self::None
        }
    }

    fn basis_size(self) -> usize {
        match self {
            Self::None => 0,
            Self::Constant => 1,
            Self::Quadratic => 6,
        }
    }

    fn fill_basis(self, x: f64, y: f64, basis: &mut [f64]) {
        match self {
            Self::None => {}
            Self::Constant => {
                basis[0] = 1.0;
            }
            Self::Quadratic => {
                basis[0] = 1.0;
                basis[1] = x;
                basis[2] = y;
                basis[3] = x * x;
                basis[4] = x * y;
                basis[5] = y * y;
            }
        }
    }
}

fn solve_linear_system(mut a: Vec<f64>, mut b: Vec<f64>, n: usize) -> Option<Vec<f64>> {
    for col in 0..n {
        let mut pivot = col;
        let mut max_abs = a[col * n + col].abs();
        for row in (col + 1)..n {
            let v = a[row * n + col].abs();
            if v > max_abs {
                max_abs = v;
                pivot = row;
            }
        }
        if max_abs <= 1.0e-15 {
            return None;
        }

        if pivot != col {
            for k in 0..n {
                a.swap(col * n + k, pivot * n + k);
            }
            b.swap(col, pivot);
        }

        let diag = a[col * n + col];
        for row in (col + 1)..n {
            let factor = a[row * n + col] / diag;
            if factor == 0.0 {
                continue;
            }
            a[row * n + col] = 0.0;
            for k in (col + 1)..n {
                a[row * n + k] -= factor * a[col * n + k];
            }
            b[row] -= factor * b[col];
        }
    }

    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut rhs = b[row];
        for k in (row + 1)..n {
            rhs -= a[row * n + k] * x[k];
        }
        let denom = a[row * n + row];
        if denom.abs() <= 1.0e-15 {
            return None;
        }
        x[row] = rhs / denom;
    }
    Some(x)
}

fn weighted_poly_predict(
    x: f64,
    y: f64,
    neighbors: &[(f64, f64, f64, f64)],
    order: RbfPolyOrder,
) -> Option<f64> {
    if order == RbfPolyOrder::None {
        return None;
    }

    let n = order.basis_size();
    if neighbors.len() < n || n == 0 {
        return None;
    }

    let mut ata = vec![0.0; n * n];
    let mut atz = vec![0.0; n];
    let mut basis = vec![0.0; n];

    for &(sx, sy, z, w) in neighbors {
        if !w.is_finite() || w <= 0.0 {
            continue;
        }
        order.fill_basis(sx, sy, &mut basis);
        for r in 0..n {
            let wr = w * basis[r];
            atz[r] += wr * z;
            for c in r..n {
                ata[r * n + c] += wr * basis[c];
            }
        }
    }

    for r in 1..n {
        for c in 0..r {
            ata[r * n + c] = ata[c * n + r];
        }
    }

    let coeffs = solve_linear_system(ata, atz, n)?;
    order.fill_basis(x, y, &mut basis);
    let mut out = 0.0;
    for i in 0..n {
        out += coeffs[i] * basis[i];
    }
    if out.is_finite() {
        Some(out)
    } else {
        None
    }
}

fn build_lidar_output(
    samples: &[(f64, f64, f64)],
    cell_size: f64,
    crs: CrsInfo,
    data_type: DataType,
) -> Result<Raster, ToolError> {
    if !cell_size.is_finite() || cell_size <= 0.0 {
        return Err(ToolError::Validation(
            "resolution/cell_size must be a positive finite value".to_string(),
        ));
    }

    let min_x = samples.iter().map(|(x, _, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = samples
        .iter()
        .map(|(x, _, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = samples.iter().map(|(_, y, _)| *y).fold(f64::INFINITY, f64::min);
    let max_y = samples
        .iter()
        .map(|(_, y, _)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    let cols = (((max_x - min_x) / cell_size).ceil() as usize).max(1);
    let rows = (((max_y - min_y) / cell_size).ceil() as usize).max(1);

    Ok(Raster::new(RasterConfig {
        cols,
        rows,
        bands: 1,
        x_min: min_x,
        y_min: max_y - rows as f64 * cell_size,
        cell_size,
        cell_size_y: Some(cell_size),
        nodata: -32768.0,
        data_type,
        crs,
        metadata: Vec::new(),
    }))
}

fn store_or_write_output(
    output: Raster,
    output_path: Option<std::path::PathBuf>,
) -> Result<String, ToolError> {
    if let Some(output_path) = output_path {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
            }
        }
        let output_path_str = output_path.to_string_lossy().to_string();
        let output_format = RasterFormat::for_output_path(&output_path_str)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;
        output
            .write(&output_path_str, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
        Ok(output_path_str)
    } else {
        let id = memory_store::put_raster(output);
        Ok(memory_store::make_raster_memory_path(&id))
    }
}

fn store_or_write_lidar_output(
    cloud: &PointCloud,
    output_path: Option<PathBuf>,
    tool_suffix: &str,
) -> Result<String, ToolError> {
    let output_path = output_path.unwrap_or_else(|| unique_lidar_temp_output_path(tool_suffix));
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
        }
    }
    cloud
        .write(&output_path)
        .map_err(|e| ToolError::Execution(format!("failed writing output lidar: {e}")))?;
    Ok(output_path.to_string_lossy().to_string())
}

fn build_lidar_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("__wbw_type__".to_string(), json!("lidar"));
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult { outputs }
}

fn build_lidar_result_with_filtered(path: String, filtered_path: Option<String>) -> ToolRunResult {
    let mut result = build_lidar_result(path);
    if let Some(p) = filtered_path {
        result.outputs.insert("filtered_path".to_string(), json!(p));
    }
    result
}

fn build_raster_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("__wbw_type__".to_string(), json!("raster"));
    outputs.insert("path".to_string(), json!(path));
    outputs.insert("active_band".to_string(), json!(0));
    ToolRunResult { outputs }
}

fn build_string_output_result(key: &str, value: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert(key.to_string(), json!(value));
    ToolRunResult { outputs }
}

fn detect_vector_output_format(path: &str) -> Result<wbvector::VectorFormat, ToolError> {
    match wbvector::VectorFormat::detect(path) {
        Ok(fmt) => Ok(fmt),
        Err(_) => {
            if Path::new(path).extension().is_none() {
                Ok(wbvector::VectorFormat::Shapefile)
            } else {
                Err(ToolError::Validation(format!(
                    "could not determine vector output format from path '{}'",
                    path
                )))
            }
        }
    }
}

fn write_vector_output(layer: &wbvector::Layer, path: &str) -> Result<String, ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
        }
    }

    let format = detect_vector_output_format(path)?;
    wbvector::write(layer, path, format)
        .map_err(|e| ToolError::Execution(format!("failed writing output vector: {}", e)))?;
    Ok(path.to_string())
}

fn build_vector_result(path: String) -> ToolRunResult {
    let mut outputs = BTreeMap::new();
    outputs.insert("path".to_string(), json!(path));
    ToolRunResult { outputs }
}

fn build_batch_placeholder_vector_result(mut paths: Vec<String>) -> Result<ToolRunResult, ToolError> {
    if paths.is_empty() {
        return Err(ToolError::Execution(
            "batch mode produced no output vector files".to_string(),
        ));
    }
    paths.sort();
    Ok(build_vector_result(paths[0].clone()))
}

fn lidar_crs_to_vector_crs(crs: Option<&LidarCrs>) -> Option<wbvector::Crs> {
    let epsg = crs.and_then(|c| c.epsg);
    let wkt = crs.and_then(|c| c.wkt.clone());
    if epsg.is_some() || wkt.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false) {
        Some(wbvector::Crs { epsg, wkt })
    } else {
        None
    }
}

fn monotonic_chain_convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
    });

    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower: Vec<(f64, f64)> = Vec::new();
    for p in &pts {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper: Vec<(f64, f64)> = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn close_ring(mut ring: Vec<wbvector::Coord>) -> Vec<wbvector::Coord> {
    if ring.is_empty() {
        return ring;
    }
    let first = ring[0].clone();
    let last = ring[ring.len() - 1].clone();
    if (first.x - last.x).abs() > 1.0e-12 || (first.y - last.y).abs() > 1.0e-12 {
        ring.push(first);
    }
    ring
}

fn interpolate_edge_contour(
    p1: (f64, f64),
    z1: f64,
    p2: (f64, f64),
    z2: f64,
    level: f64,
) -> Option<(f64, f64)> {
    let d1 = z1 - level;
    let d2 = z2 - level;
    if d1.abs() <= 1.0e-12 && d2.abs() <= 1.0e-12 {
        return None;
    }
    if d1 * d2 > 0.0 {
        return None;
    }
    let dz = z2 - z1;
    if dz.abs() <= 1.0e-12 {
        return None;
    }
    let t = ((level - z1) / dz).clamp(0.0, 1.0);
    Some((p1.0 + t * (p2.0 - p1.0), p1.1 + t * (p2.1 - p1.1)))
}

fn build_batch_placeholder_raster_result(mut paths: Vec<String>) -> Result<ToolRunResult, ToolError> {
    if paths.is_empty() {
        return Err(ToolError::Execution(
            "batch mode produced no output rasters".to_string(),
        ));
    }
    paths.sort();
    Ok(build_raster_result(paths[0].clone()))
}

fn build_batch_placeholder_lidar_result(mut paths: Vec<String>) -> Result<ToolRunResult, ToolError> {
    if paths.is_empty() {
        return Err(ToolError::Execution(
            "batch mode produced no output lidar files".to_string(),
        ));
    }
    paths.sort();
    Ok(build_lidar_result(paths[0].clone()))
}

fn extract_raster_path_from_result(result: ToolRunResult, tool_id: &str) -> Result<String, ToolError> {
    let out_type = result
        .outputs
        .get("__wbw_type__")
        .and_then(Value::as_str)
        .unwrap_or("");
    if out_type != "raster" {
        return Err(ToolError::Execution(format!(
            "{} expected raster output but got '{}'",
            tool_id, out_type
        )));
    }
    result
        .outputs
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ToolError::Execution(format!("{} missing output path", tool_id)))
}

fn run_block_extrema_tile(
    input_path: &Path,
    output_path: Option<&Path>,
    resolution: f64,
    parameter: &str,
    returns_mode: ReturnsMode,
    include_classes: &[bool; 256],
    min_z: f64,
    max_z: f64,
    use_max: bool,
) -> Result<String, ToolError> {
    let cloud = PointCloud::read(input_path).map_err(|e| {
        ToolError::Execution(format!(
            "failed reading input lidar '{}': {e}",
            input_path.to_string_lossy()
        ))
    })?;

    let samples = collect_lidar_samples(&cloud.points, parameter, returns_mode, include_classes, min_z, max_z)?;
    let mut output = build_lidar_output(
        &samples,
        resolution,
        lidar_crs_to_raster_crs(cloud.crs.as_ref()),
        DataType::F64,
    )?;

    let mut out_values = vec![output.nodata; output.data.len()];
    let y_max = output.y_max();
    for (x, y, z) in &samples {
        if let Some(idx) = raster_cell_index(
            *x,
            *y,
            output.x_min,
            y_max,
            output.cell_size_x,
            output.cell_size_y,
            output.rows,
            output.cols,
        ) {
            if out_values[idx] == output.nodata
                || (use_max && *z > out_values[idx])
                || (!use_max && *z < out_values[idx])
            {
                out_values[idx] = *z;
            }
        }
    }

    for (idx, value) in out_values.iter().enumerate() {
        output.data.set_f64(idx, *value);
    }

    store_or_write_output(output, output_path.map(Path::to_path_buf))
}

fn run_point_density_tile(
    input_path: &Path,
    output_path: Option<&Path>,
    resolution: f64,
    radius: f64,
    returns_mode: ReturnsMode,
    include_classes: &[bool; 256],
    min_z: f64,
    max_z: f64,
) -> Result<String, ToolError> {
    let cloud = PointCloud::read(input_path).map_err(|e| {
        ToolError::Execution(format!(
            "failed reading input lidar '{}': {e}",
            input_path.to_string_lossy()
        ))
    })?;

    let samples = collect_lidar_samples(&cloud.points, "elevation", returns_mode, include_classes, min_z, max_z)?;
    let mut output = build_lidar_output(
        &samples,
        resolution,
        lidar_crs_to_raster_crs(cloud.crs.as_ref()),
        DataType::F64,
    )?;

    let mut tree = KdTree::new(2);
    for (x, y, _) in &samples {
        tree.add([*x, *y], 1u8)
            .map_err(|e| ToolError::Execution(format!("failed building point-density index: {e}")))?;
    }

    let area = std::f64::consts::PI * radius * radius;
    let rows = output.rows;
    let cols = output.cols;
    let nodata = output.nodata;
    let x_min = output.x_min;
    let y_max = output.y_max();
    let cell_x = output.cell_size_x;
    let cell_y = output.cell_size_y;
    let mut out_values = vec![nodata; output.data.len()];

    for row in 0..rows {
        for col in 0..cols {
            let x = x_min + (col as f64 + 0.5) * cell_x;
            let y = y_max - (row as f64 + 0.5) * cell_y;
            let ret = tree
                .within(&[x, y], radius * radius, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("point-density search failed: {e}")))?;
            out_values[row * cols + col] = ret.len() as f64 / area;
        }
    }

    for (idx, value) in out_values.iter().enumerate() {
        output.data.set_f64(idx, *value);
    }

    store_or_write_output(output, output_path.map(Path::to_path_buf))
}

fn run_dsm_tile(
    input_path: &Path,
    output_path: Option<&Path>,
    resolution: f64,
    radius: f64,
    min_z: f64,
    max_z: f64,
    max_triangle_edge_length: f64,
) -> Result<String, ToolError> {
    let cloud = PointCloud::read(input_path).map_err(|e| {
        ToolError::Execution(format!(
            "failed reading input lidar '{}': {e}",
            input_path.to_string_lossy()
        ))
    })?;

    let mut include_classes = [true; 256];
    include_classes[7] = false;
    include_classes[18] = false;
    let mut samples = collect_lidar_samples(&cloud.points, "elevation", ReturnsMode::All, &include_classes, min_z, max_z)?;
    if samples.len() < 3 {
        return Err(ToolError::Validation("input lidar must contain at least three points for triangulation".to_string()));
    }

    // Keep local top-surface candidates before TIN interpolation.
    let mut tree = KdTree::new(2);
    for (idx, (x, y, _)) in samples.iter().enumerate() {
        tree.add([*x, *y], idx)
            .map_err(|e| ToolError::Execution(format!("failed building dsm index: {e}")))?;
    }
    let mut top_samples = Vec::with_capacity(samples.len());
    for (i, (x, y, z)) in samples.iter().enumerate() {
        let neighbors = tree
            .within(&[*x, *y], radius * radius, &squared_euclidean)
            .map_err(|e| ToolError::Execution(format!("dsm neighborhood search failed: {e}")))?;
        let mut dominated = false;
        for (_, n_idx) in neighbors {
            if *n_idx != i && samples[*n_idx].2 > *z {
                dominated = true;
                break;
            }
        }
        if !dominated {
            top_samples.push((*x, *y, *z));
        }
    }
    if top_samples.len() >= 3 {
        samples = top_samples;
    }

    let mut output = build_lidar_output(
        &samples,
        resolution,
        lidar_crs_to_raster_crs(cloud.crs.as_ref()),
        DataType::F64,
    )?;
    let topo_points: Vec<TopoCoord> = samples.iter().map(|(x, y, _)| TopoCoord::xy(*x, *y)).collect();
    let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
    if triangulation.triangles.is_empty() {
        return Err(ToolError::Execution("failed to build triangulation from input lidar points".to_string()));
    }

    let mut value_lookup = HashMap::with_capacity(samples.len());
    for (x, y, value) in &samples {
        value_lookup.entry(point_key_bits(*x, *y)).or_insert(*value);
    }

    let rows = output.rows;
    let cols = output.cols;
    let nodata = output.nodata;
    let x_min = output.x_min;
    let y_max = output.y_max();
    let cell_x = output.cell_size_x;
    let cell_y = output.cell_size_y;
    let max_edge_sq = if max_triangle_edge_length.is_infinite() {
        f64::INFINITY
    } else {
        max_triangle_edge_length * max_triangle_edge_length
    };

    let mut out_values = vec![nodata; output.data.len()];
    for tri in &triangulation.triangles {
        let p1 = triangulation.points[tri[0]];
        let p2 = triangulation.points[tri[1]];
        let p3 = triangulation.points[tri[2]];

        let z1 = *value_lookup.get(&point_key_bits(p1.x, p1.y)).ok_or_else(|| ToolError::Execution("triangulation lookup failed for vertex 1".to_string()))?;
        let z2 = *value_lookup.get(&point_key_bits(p2.x, p2.y)).ok_or_else(|| ToolError::Execution("triangulation lookup failed for vertex 2".to_string()))?;
        let z3 = *value_lookup.get(&point_key_bits(p3.x, p3.y)).ok_or_else(|| ToolError::Execution("triangulation lookup failed for vertex 3".to_string()))?;

        if max_triangle_edge_length_2d_sq((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)) > max_edge_sq {
            continue;
        }

        let min_x = p1.x.min(p2.x.min(p3.x));
        let max_x = p1.x.max(p2.x.max(p3.x));
        let min_y = p1.y.min(p2.y.min(p3.y));
        let max_y = p1.y.max(p2.y.max(p3.y));
        let col_start = (((min_x - x_min) / cell_x).floor() as isize).clamp(0, cols as isize - 1) as usize;
        let col_end = (((max_x - x_min) / cell_x).ceil() as isize).clamp(0, cols as isize - 1) as usize;
        let row_start = (((y_max - max_y) / cell_y).floor() as isize).clamp(0, rows as isize - 1) as usize;
        let row_end = (((y_max - min_y) / cell_y).ceil() as isize).clamp(0, rows as isize - 1) as usize;

        for row in row_start..=row_end {
            for col in col_start..=col_end {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;
                if let Some((w1, w2, w3)) = point_in_triangle_with_barycentric(x, y, (p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y), 1.0e-10) {
                    out_values[row * cols + col] = w1 * z1 + w2 * z2 + w3 * z3;
                }
            }
        }
    }

    for (idx, value) in out_values.iter().enumerate() {
        output.data.set_f64(idx, *value);
    }

    store_or_write_output(output, output_path.map(Path::to_path_buf))
}

fn point_in_triangle_with_barycentric(
    x: f64,
    y: f64,
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    epsilon: f64,
) -> Option<(f64, f64, f64)> {
    let denom = (p2.1 - p3.1) * (p1.0 - p3.0) + (p3.0 - p2.0) * (p1.1 - p3.1);
    if denom.abs() <= epsilon {
        return None;
    }

    let w1 = ((p2.1 - p3.1) * (x - p3.0) + (p3.0 - p2.0) * (y - p3.1)) / denom;
    let w2 = ((p3.1 - p1.1) * (x - p3.0) + (p1.0 - p3.0) * (y - p3.1)) / denom;
    let w3 = 1.0 - w1 - w2;

    if w1 >= -epsilon && w2 >= -epsilon && w3 >= -epsilon {
        Some((w1, w2, w3))
    } else {
        None
    }
}

fn max_triangle_edge_length_2d_sq(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
    let d12 = (p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2);
    let d13 = (p1.0 - p3.0).powi(2) + (p1.1 - p3.1).powi(2);
    let d23 = (p2.0 - p3.0).powi(2) + (p2.1 - p3.1).powi(2);
    d12.max(d13).max(d23)
}

fn point_key_bits(x: f64, y: f64) -> (u64, u64) {
    (x.to_bits(), y.to_bits())
}

fn cross2d(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn convex_hull_2d(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() <= 3 {
        return points.to_vec();
    }

    let mut pts = points.to_vec();
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
    });
    pts.dedup_by(|a, b| (a.0 - b.0).abs() <= 1.0e-12 && (a.1 - b.1).abs() <= 1.0e-12);

    if pts.len() <= 3 {
        return pts;
    }

    let mut lower: Vec<(f64, f64)> = Vec::new();
    for p in &pts {
        while lower.len() >= 2
            && cross2d(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }

    let mut upper: Vec<(f64, f64)> = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2
            && cross2d(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn point_on_segment_2d(point: (f64, f64), a: (f64, f64), b: (f64, f64), epsilon: f64) -> bool {
    let cross = cross2d(a, b, point).abs();
    if cross > epsilon {
        return false;
    }
    let min_x = a.0.min(b.0) - epsilon;
    let max_x = a.0.max(b.0) + epsilon;
    let min_y = a.1.min(b.1) - epsilon;
    let max_y = a.1.max(b.1) + epsilon;
    point.0 >= min_x && point.0 <= max_x && point.1 >= min_y && point.1 <= max_y
}

fn point_in_polygon_2d(point: (f64, f64), polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let (px, py) = point;
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        if point_on_segment_2d(point, (xi, yi), (xj, yj), 1.0e-10) {
            return true;
        }
        let intersects = ((yi > py) != (yj > py))
            && (px < (xj - xi) * (py - yi) / ((yj - yi).abs().max(1.0e-15)) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn convex_hull_scanline_span(y: f64, hull: &[(f64, f64)], epsilon: f64) -> Option<(f64, f64)> {
    if hull.len() < 3 {
        return None;
    }

    let mut xs = Vec::with_capacity(hull.len());
    let mut j = hull.len() - 1;
    for i in 0..hull.len() {
        let (x1, y1) = hull[j];
        let (x2, y2) = hull[i];
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);
        if y < min_y - epsilon || y > max_y + epsilon {
            j = i;
            continue;
        }

        let dy = y2 - y1;
        if dy.abs() <= epsilon {
            xs.push(x1.min(x2));
            xs.push(x1.max(x2));
        } else {
            let t = ((y - y1) / dy).clamp(0.0, 1.0);
            xs.push(x1 + t * (x2 - x1));
        }

        j = i;
    }

    if xs.is_empty() {
        return None;
    }

    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let min_x = *xs.first().unwrap_or(&f64::NAN);
    let max_x = *xs.last().unwrap_or(&f64::NAN);
    if min_x.is_finite() && max_x.is_finite() && min_x <= max_x {
        Some((min_x, max_x))
    } else {
        None
    }
}

fn raster_cell_index(
    x: f64,
    y: f64,
    x_min: f64,
    y_max: f64,
    cell_x: f64,
    cell_y: f64,
    rows: usize,
    cols: usize,
) -> Option<usize> {
    let col = ((x - x_min) / cell_x).floor() as isize;
    let row = ((y_max - y) / cell_y).floor() as isize;
    if row < 0 || col < 0 || row >= rows as isize || col >= cols as isize {
        None
    } else {
        Some(row as usize * cols + col as usize)
    }
}

fn parse_azimuth_altitude(args: &ToolArgs) -> (f64, f64) {
    let az = parse_f64_alias(args, &["azimuth"], 315.0).to_radians();
    let alt = parse_f64_alias(args, &["altitude"], 30.0).to_radians();
    (az, alt)
}

fn hillshade_from_raster(
    values: &[f64],
    rows: usize,
    cols: usize,
    cell_x: f64,
    cell_y: f64,
    nodata: f64,
    azimuth_rad: f64,
    altitude_rad: f64,
) -> Vec<f64> {
    let mut out = vec![nodata; rows * cols];
    if rows < 3 || cols < 3 {
        return out;
    }

    let sin_alt = altitude_rad.sin();
    let cos_alt = altitude_rad.cos();

    for row in 1..(rows - 1) {
        for col in 1..(cols - 1) {
            let idx = row * cols + col;
            let z = values[idx];
            if z == nodata {
                continue;
            }

            let z1 = values[(row - 1) * cols + (col - 1)];
            let z2 = values[(row - 1) * cols + col];
            let z3 = values[(row - 1) * cols + (col + 1)];
            let z4 = values[row * cols + (col - 1)];
            let z5 = values[row * cols + (col + 1)];
            let z6 = values[(row + 1) * cols + (col - 1)];
            let z7 = values[(row + 1) * cols + col];
            let z8 = values[(row + 1) * cols + (col + 1)];

            if [z1, z2, z3, z4, z5, z6, z7, z8].iter().any(|v| *v == nodata) {
                continue;
            }

            let dzdx = ((z3 + 2.0 * z5 + z8) - (z1 + 2.0 * z4 + z6)) / (8.0 * cell_x);
            let dzdy = ((z6 + 2.0 * z7 + z8) - (z1 + 2.0 * z2 + z3)) / (8.0 * cell_y);
            let slope = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            let aspect = dzdy.atan2(-dzdx);

            let hs = 255.0
                * (sin_alt * slope.sin() + cos_alt * slope.cos() * (azimuth_rad - aspect).cos())
                    .max(0.0);
            out[idx] = hs;
        }
    }

    for idx in 0..out.len() {
        if out[idx] == nodata && values[idx] != nodata {
            out[idx] = 0.0;
        }
    }

    out
}


#[derive(Clone, Copy)]
enum RbfBasisType {
    ThinPlateSpline,
    PolyHarmonic,
    Gaussian,
    MultiQuadric,
    InverseMultiQuadric,
}

impl RbfBasisType {
    fn parse(value: Option<&str>) -> Self {
        let text = value.unwrap_or("thinplatespline").trim().to_ascii_lowercase();
        if text.contains("thin") {
            Self::ThinPlateSpline
        } else if text.contains("polyharmonic") {
            Self::PolyHarmonic
        } else if text.contains("gaussian") {
            Self::Gaussian
        } else if text.contains("multiquadric") {
            Self::MultiQuadric
        } else {
            Self::InverseMultiQuadric
        }
    }
}

fn rbf_similarity_weight(dist: f64, basis: RbfBasisType, weight: f64) -> f64 {
    let eps = weight.abs().max(1.0e-12);
    let r = dist.max(1.0e-12);
    match basis {
        RbfBasisType::ThinPlateSpline => {
            let v = r * r * r.ln().abs();
            1.0 / (v + 1.0e-12)
        }
        RbfBasisType::PolyHarmonic => 1.0 / (r.powf(weight.abs().max(1.0)) + 1.0e-12),
        RbfBasisType::Gaussian => (-(eps * r).powi(2)).exp(),
        RbfBasisType::MultiQuadric => 1.0 / ((1.0 + (eps * r).powi(2)).sqrt() + 1.0e-12),
        RbfBasisType::InverseMultiQuadric => 1.0 / (1.0 + (eps * r).powi(2)).sqrt(),
    }
}

impl Tool for LidarNearestNeighbourGriddingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_nearest_neighbour_gridding",
            display_name: "LiDAR Nearest-Neighbour Gridding",
            summary: "Interpolates a raster from LiDAR points using nearest-neighbour assignment.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution",
                    description: "Output cell size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_radius",
                    description: "Maximum nearest-neighbour distance; cells beyond this become NoData.",
                    required: false,
                },
                ToolParamSpec {
                    name: "interpolation_parameter",
                    description: "Point attribute to interpolate (elevation, intensity, class, return_number, number_of_returns, scan_angle, time, rgb, user_data).",
                    required: false,
                },
                ToolParamSpec {
                    name: "returns_included",
                    description: "Return filtering mode: all, first, or last.",
                    required: false,
                },
                ToolParamSpec {
                    name: "excluded_classes",
                    description: "Classes to exclude (array or comma-delimited string).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_elev",
                    description: "Minimum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_elev",
                    description: "Maximum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/cell_size must be a positive finite value".to_string(),
            ));
        }
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ToolError::Validation(
                "search_radius/radius must be a positive finite value".to_string(),
            ));
        }
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation(
                "min_elev/minz must be <= max_elev/maxz".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        if input_path.is_none() {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_paths: Vec<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let mut tile_args = args.clone();
                    let input_str = input.to_string_lossy().to_string();
                    tile_args.insert("input".to_string(), json!(input_str.clone()));
                    let neighbors: Vec<String> = all_paths
                        .iter()
                        .filter(|p| *p != &input_str)
                        .cloned()
                        .collect();
                    tile_args.insert("batch_neighbor_inputs".to_string(), json!(neighbors));
                    tile_args.remove("input_lidar");
                    let out = generate_batch_output_path(&input, "nn");
                    tile_args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                    let result = self.run(&tile_args, ctx)?;
                    extract_raster_path_from_result(result, "lidar_nearest_neighbour_gridding")
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            return build_batch_placeholder_raster_result(outputs);
        }
        let input_path = input_path.expect("checked above");
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let neighbor_paths = parse_batch_neighbor_inputs(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        require_lidar_crs(cloud.crs.as_ref(), "lidar_nearest_neighbour_gridding")?;

        let target_samples = collect_lidar_samples(
            &cloud.points,
            &parameter,
            returns_mode,
            &include_classes,
            min_z,
            max_z,
        )?;
        let mut samples = target_samples.clone();
        for neighbor_path in neighbor_paths {
            let n_cloud = PointCloud::read(&neighbor_path)
                .map_err(|e| ToolError::Execution(format!("failed reading batch-neighbor lidar '{}': {e}", neighbor_path)))?;
            let mut n_samples = collect_lidar_samples(
                &n_cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            samples.append(&mut n_samples);
        }
        let mut output = build_lidar_output(
            &target_samples,
            resolution,
            lidar_crs_to_raster_crs(cloud.crs.as_ref()),
            DataType::F64,
        )?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building nearest-neighbour index: {e}")))?;
        }

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        let mut out_values = vec![nodata; output.data.len()];
        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;
                let nearest = tree
                    .nearest(&[x, y], 1, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("nearest-neighbour search failed: {e}")))?;
                if let Some((dist2, value)) = nearest.first() {
                    if dist2.sqrt() <= radius {
                        out_values[row * cols + col] = **value;
                    }
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = store_or_write_output(output, output_path)?;
        ctx.progress.progress(1.0);
        Ok(build_raster_result(locator))
    }
}

impl Tool for LidarIdwInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_idw_interpolation",
            display_name: "LiDAR IDW Interpolation",
            summary: "Interpolates a raster from LiDAR points using inverse-distance weighting.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution",
                    description: "Output cell size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "weight",
                    description: "IDW exponent (power).",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_radius",
                    description: "Neighbourhood radius; <=0 switches to k-nearest mode.",
                    required: false,
                },
                ToolParamSpec {
                    name: "interpolation_parameter",
                    description: "Point attribute to interpolate (elevation, intensity, class, return_number, number_of_returns, scan_angle, time, rgb, user_data).",
                    required: false,
                },
                ToolParamSpec {
                    name: "returns_included",
                    description: "Return filtering mode: all, first, or last.",
                    required: false,
                },
                ToolParamSpec {
                    name: "excluded_classes",
                    description: "Classes to exclude (array or comma-delimited string).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_elev",
                    description: "Minimum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_elev",
                    description: "Maximum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_points",
                    description: "Minimum number of points for k-nearest fallback.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/cell_size must be a positive finite value".to_string(),
            ));
        }
        let weight = parse_f64_alias(args, &["weight", "idw_weight"], 1.0);
        if !weight.is_finite() || weight < 0.0 {
            return Err(ToolError::Validation(
                "weight/idw_weight must be a finite value >= 0".to_string(),
            ));
        }
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        if !radius.is_finite() {
            return Err(ToolError::Validation(
                "search_radius/radius must be finite".to_string(),
            ));
        }
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation(
                "min_elev/minz must be <= max_elev/maxz".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        if input_path.is_none() {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_paths: Vec<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let mut tile_args = args.clone();
                    let input_str = input.to_string_lossy().to_string();
                    tile_args.insert("input".to_string(), json!(input_str.clone()));
                    let neighbors: Vec<String> = all_paths
                        .iter()
                        .filter(|p| *p != &input_str)
                        .cloned()
                        .collect();
                    tile_args.insert("batch_neighbor_inputs".to_string(), json!(neighbors));
                    tile_args.remove("input_lidar");
                    let out = generate_batch_output_path(&input, "idw");
                    tile_args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                    let result = self.run(&tile_args, ctx)?;
                    extract_raster_path_from_result(result, "lidar_idw_interpolation")
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            return build_batch_placeholder_raster_result(outputs);
        }
        let input_path = input_path.expect("checked above");
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let weight = parse_f64_alias(args, &["weight", "idw_weight"], 1.0);
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let neighbor_paths = parse_batch_neighbor_inputs(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let min_points = args.get("min_points").and_then(Value::as_u64).unwrap_or(1) as usize;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        require_lidar_crs(cloud.crs.as_ref(), "lidar_idw_interpolation")?;

        let target_samples = collect_lidar_samples(
            &cloud.points,
            &parameter,
            returns_mode,
            &include_classes,
            min_z,
            max_z,
        )?;
        let mut samples = target_samples.clone();
        for neighbor_path in neighbor_paths {
            let n_cloud = PointCloud::read(&neighbor_path)
                .map_err(|e| ToolError::Execution(format!("failed reading batch-neighbor lidar '{}': {e}", neighbor_path)))?;
            let mut n_samples = collect_lidar_samples(
                &n_cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            samples.append(&mut n_samples);
        }
        let mut output = build_lidar_output(
            &target_samples,
            resolution,
            lidar_crs_to_raster_crs(cloud.crs.as_ref()),
            DataType::F64,
        )?;

        let mut tree = KdTree::new(2);
        for (x, y, value) in &samples {
            tree.add([*x, *y], *value)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;

        let mut out_values = vec![nodata; output.data.len()];
        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;

                let mut neighbours = if radius > 0.0 {
                    tree.within(&[x, y], radius * radius, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("idw radius search failed: {e}")))?
                } else {
                    Vec::new()
                };

                if radius <= 0.0 || neighbours.len() < min_points.max(1) {
                    let k = min_points.max(1).min(samples.len());
                    neighbours = tree
                        .nearest(&[x, y], k, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("idw nearest-neighbour search failed: {e}")))?;
                }

                if neighbours.is_empty() {
                    continue;
                }

                let mut weighted_sum = 0.0;
                let mut sum_w = 0.0;
                let mut assigned = false;
                for (dist2, value) in neighbours {
                    if dist2 <= f64::EPSILON {
                        out_values[row * cols + col] = *value;
                        assigned = true;
                        break;
                    }
                    let dist = dist2.sqrt();
                    let w = if weight == 0.0 { 1.0 } else { 1.0 / dist.powf(weight) };
                    weighted_sum += *value * w;
                    sum_w += w;
                }

                if !assigned && sum_w > 0.0 {
                    out_values[row * cols + col] = weighted_sum / sum_w;
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = store_or_write_output(output, output_path)?;
        ctx.progress.progress(1.0);
        Ok(build_raster_result(locator))
    }
}

impl Tool for LidarTinGriddingTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_tin_gridding",
            display_name: "LiDAR TIN Gridding",
            summary: "Interpolates a raster from LiDAR points using Delaunay triangulation.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution",
                    description: "Output cell size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_triangle_edge_length",
                    description: "Optional maximum triangle edge length to suppress long-edge facets.",
                    required: false,
                },
                ToolParamSpec {
                    name: "interpolation_parameter",
                    description: "Point attribute to interpolate (elevation, intensity, class, return_number, number_of_returns, scan_angle, time, rgb, user_data).",
                    required: false,
                },
                ToolParamSpec {
                    name: "returns_included",
                    description: "Return filtering mode: all, first, or last.",
                    required: false,
                },
                ToolParamSpec {
                    name: "excluded_classes",
                    description: "Classes to exclude (array or comma-delimited string).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_elev",
                    description: "Minimum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_elev",
                    description: "Maximum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/cell_size must be a positive finite value".to_string(),
            ));
        }
        let max_edge = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
        if !max_edge.is_finite() && !max_edge.is_infinite() {
            return Err(ToolError::Validation(
                "max_triangle_edge_length must be finite or infinity".to_string(),
            ));
        }
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation(
                "min_elev/minz must be <= max_elev/maxz".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        if input_path.is_none() {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_paths: Vec<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let mut tile_args = args.clone();
                    let input_str = input.to_string_lossy().to_string();
                    tile_args.insert("input".to_string(), json!(input_str.clone()));
                    let neighbors: Vec<String> = all_paths
                        .iter()
                        .filter(|p| *p != &input_str)
                        .cloned()
                        .collect();
                    tile_args.insert("batch_neighbor_inputs".to_string(), json!(neighbors));
                    tile_args.remove("input_lidar");
                    let out = generate_batch_output_path(&input, "tin");
                    tile_args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                    let result = self.run(&tile_args, ctx)?;
                    extract_raster_path_from_result(result, "lidar_tin_gridding")
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            return build_batch_placeholder_raster_result(outputs);
        }
        let input_path = input_path.expect("checked above");
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let max_triangle_edge_length_raw =
            parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
        let max_triangle_edge_length = if max_triangle_edge_length_raw <= 0.0 {
            f64::INFINITY
        } else {
            max_triangle_edge_length_raw
        };
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let neighbor_paths = parse_batch_neighbor_inputs(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        require_lidar_crs(cloud.crs.as_ref(), "lidar_tin_gridding")?;

        let target_samples = collect_lidar_samples(
            &cloud.points,
            &parameter,
            returns_mode,
            &include_classes,
            min_z,
            max_z,
        )?;
        let mut samples = target_samples.clone();
        for neighbor_path in neighbor_paths {
            let n_cloud = PointCloud::read(&neighbor_path)
                .map_err(|e| ToolError::Execution(format!("failed reading batch-neighbor lidar '{}': {e}", neighbor_path)))?;
            let mut n_samples = collect_lidar_samples(
                &n_cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            samples.append(&mut n_samples);
        }
        if samples.len() < 3 || target_samples.is_empty() {
            return Err(ToolError::Validation(
                "input lidar must contain at least three points for triangulation".to_string(),
            ));
        }

        let mut output = build_lidar_output(
            &target_samples,
            resolution,
            lidar_crs_to_raster_crs(cloud.crs.as_ref()),
            DataType::F64,
        )?;

        let topo_points: Vec<TopoCoord> = samples
            .iter()
            .map(|(x, y, _)| TopoCoord::xy(*x, *y))
            .collect();
        let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
        if triangulation.triangles.is_empty() {
            return Err(ToolError::Execution(
                "failed to build triangulation from input lidar points".to_string(),
            ));
        }

        let mut value_lookup = HashMap::with_capacity(samples.len());
        for (x, y, value) in &samples {
            value_lookup.entry(point_key_bits(*x, *y)).or_insert(*value);
        }

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let max_edge_sq = if max_triangle_edge_length.is_infinite() {
            f64::INFINITY
        } else {
            max_triangle_edge_length * max_triangle_edge_length
        };

        let mut out_values = vec![nodata; output.data.len()];
        for (tri_idx, tri) in triangulation.triangles.iter().enumerate() {
            let p1 = triangulation.points[tri[0]];
            let p2 = triangulation.points[tri[1]];
            let p3 = triangulation.points[tri[2]];

            let z1 = *value_lookup.get(&point_key_bits(p1.x, p1.y)).ok_or_else(|| {
                ToolError::Execution("triangulation lookup failed for vertex 1".to_string())
            })?;
            let z2 = *value_lookup.get(&point_key_bits(p2.x, p2.y)).ok_or_else(|| {
                ToolError::Execution("triangulation lookup failed for vertex 2".to_string())
            })?;
            let z3 = *value_lookup.get(&point_key_bits(p3.x, p3.y)).ok_or_else(|| {
                ToolError::Execution("triangulation lookup failed for vertex 3".to_string())
            })?;

            if max_triangle_edge_length_2d_sq((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y)) > max_edge_sq {
                continue;
            }

            let min_x = p1.x.min(p2.x.min(p3.x));
            let max_x = p1.x.max(p2.x.max(p3.x));
            let min_y = p1.y.min(p2.y.min(p3.y));
            let max_y = p1.y.max(p2.y.max(p3.y));

            let col_start = (((min_x - x_min) / cell_x).floor() as isize).clamp(0, cols as isize - 1) as usize;
            let col_end = (((max_x - x_min) / cell_x).ceil() as isize).clamp(0, cols as isize - 1) as usize;
            let row_start = (((y_max - max_y) / cell_y).floor() as isize).clamp(0, rows as isize - 1) as usize;
            let row_end = (((y_max - min_y) / cell_y).ceil() as isize).clamp(0, rows as isize - 1) as usize;

            for row in row_start..=row_end {
                for col in col_start..=col_end {
                    let x = x_min + (col as f64 + 0.5) * cell_x;
                    let y = y_max - (row as f64 + 0.5) * cell_y;
                    if let Some((w1, w2, w3)) = point_in_triangle_with_barycentric(
                        x,
                        y,
                        (p1.x, p1.y),
                        (p2.x, p2.y),
                        (p3.x, p3.y),
                        1.0e-10,
                    ) {
                        out_values[row * cols + col] = w1 * z1 + w2 * z2 + w3 * z3;
                    }
                }
            }

            ctx.progress
                .progress((tri_idx + 1) as f64 / triangulation.triangles.len().max(1) as f64 * 0.99);
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = store_or_write_output(output, output_path)?;
        ctx.progress.progress(1.0);
        Ok(build_raster_result(locator))
    }
}

impl Tool for LidarRadialBasisFunctionInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_radial_basis_function_interpolation",
            display_name: "LiDAR Radial Basis Function Interpolation",
            summary: "Interpolates a raster from LiDAR points using local radial-basis similarity weighting.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution",
                    description: "Output cell size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "num_points",
                    description: "Minimum number of neighbours used for local interpolation.",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_radius",
                    description: "Neighbourhood radius; <=0 uses only k-nearest mode.",
                    required: false,
                },
                ToolParamSpec {
                    name: "func_type",
                    description: "RBF basis type (thinplatespline, polyharmonic, gaussian, multiquadric, inversemultiquadric).",
                    required: false,
                },
                ToolParamSpec {
                    name: "poly_order",
                    description: "Polynomial trend order used for local correction (none, constant, quadratic).",
                    required: false,
                },
                ToolParamSpec {
                    name: "weight",
                    description: "Basis shape parameter/exponent depending on func_type.",
                    required: false,
                },
                ToolParamSpec {
                    name: "interpolation_parameter",
                    description: "Point attribute to interpolate (elevation, intensity, class, return_number, number_of_returns, scan_angle, time, rgb, user_data).",
                    required: false,
                },
                ToolParamSpec {
                    name: "returns_included",
                    description: "Return filtering mode: all, first, or last.",
                    required: false,
                },
                ToolParamSpec {
                    name: "excluded_classes",
                    description: "Classes to exclude (array or comma-delimited string).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_elev",
                    description: "Minimum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_elev",
                    description: "Maximum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/cell_size must be a positive finite value".to_string(),
            ));
        }
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 0.0);
        if !radius.is_finite() {
            return Err(ToolError::Validation(
                "search_radius/radius must be finite".to_string(),
            ));
        }
        let num_points = args.get("num_points").and_then(Value::as_u64).unwrap_or(15) as usize;
        if num_points == 0 {
            return Err(ToolError::Validation(
                "num_points must be greater than zero".to_string(),
            ));
        }
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = RbfBasisType::parse(args.get("func_type").and_then(Value::as_str));
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation(
                "min_elev/minz must be <= max_elev/maxz".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        if input_path.is_none() {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_paths: Vec<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let mut tile_args = args.clone();
                    let input_str = input.to_string_lossy().to_string();
                    tile_args.insert("input".to_string(), json!(input_str.clone()));
                    let neighbors: Vec<String> = all_paths
                        .iter()
                        .filter(|p| *p != &input_str)
                        .cloned()
                        .collect();
                    tile_args.insert("batch_neighbor_inputs".to_string(), json!(neighbors));
                    tile_args.remove("input_lidar");
                    let out = generate_batch_output_path(&input, "rbf");
                    tile_args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                    let result = self.run(&tile_args, ctx)?;
                    extract_raster_path_from_result(result, "lidar_radial_basis_function_interpolation")
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            return build_batch_placeholder_raster_result(outputs);
        }
        let input_path = input_path.expect("checked above");
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let num_points = args.get("num_points").and_then(Value::as_u64).unwrap_or(15) as usize;
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 0.0);
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let neighbor_paths = parse_batch_neighbor_inputs(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let basis = RbfBasisType::parse(args.get("func_type").and_then(Value::as_str));
        let poly_order = RbfPolyOrder::parse(
            args.get("poly_order")
                .and_then(Value::as_str),
        );
        let shape_weight = args.get("weight").and_then(Value::as_f64).unwrap_or(0.1);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        require_lidar_crs(cloud.crs.as_ref(), "lidar_radial_basis_function_interpolation")?;

        let target_samples = collect_lidar_samples(
            &cloud.points,
            &parameter,
            returns_mode,
            &include_classes,
            min_z,
            max_z,
        )?;
        let mut samples = target_samples.clone();
        for neighbor_path in neighbor_paths {
            let n_cloud = PointCloud::read(&neighbor_path)
                .map_err(|e| ToolError::Execution(format!("failed reading batch-neighbor lidar '{}': {e}", neighbor_path)))?;
            let mut n_samples = collect_lidar_samples(
                &n_cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            samples.append(&mut n_samples);
        }
        let mut output = build_lidar_output(
            &target_samples,
            resolution,
            lidar_crs_to_raster_crs(cloud.crs.as_ref()),
            DataType::F64,
        )?;

        let mut tree = KdTree::new(2);
        for (idx, (x, y, _)) in samples.iter().enumerate() {
            tree.add([*x, *y], idx)
                .map_err(|e| ToolError::Execution(format!("failed building interpolation index: {e}")))?;
        }

        let hull = convex_hull_2d(&samples.iter().map(|(x, y, _)| (*x, *y)).collect::<Vec<_>>());
        let radius_sq = radius * radius;
        let k = num_points.max(3).min(samples.len());

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let mut out_values = vec![nodata; output.data.len()];

        for row in 0..rows {
            for col in 0..cols {
                let x = x_min + (col as f64 + 0.5) * cell_x;
                let y = y_max - (row as f64 + 0.5) * cell_y;
                if !point_in_polygon_2d((x, y), &hull) {
                    continue;
                }

                let mut neighbours = if radius > 0.0 {
                    tree.within(&[x, y], radius_sq, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("rbf radius search failed: {e}")))?
                } else {
                    Vec::new()
                };
                if neighbours.len() < k {
                    neighbours = tree
                        .nearest(&[x, y], k, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("rbf nearest-neighbour search failed: {e}")))?;
                }

                if neighbours.is_empty() {
                    continue;
                }

                let mut weighted_sum = 0.0;
                let mut sum_w = 0.0;
                let mut assigned = false;
                let mut poly_neighbors = Vec::with_capacity(neighbours.len());
                for (dist2, sample_idx) in neighbours {
                    let sample = samples[*sample_idx];
                    if dist2 <= f64::EPSILON {
                        out_values[row * cols + col] = sample.2;
                        assigned = true;
                        break;
                    }
                    let dist = dist2.sqrt();
                    let w = rbf_similarity_weight(dist, basis, shape_weight);
                    if w.is_finite() && w > 0.0 {
                        weighted_sum += sample.2 * w;
                        sum_w += w;
                        poly_neighbors.push((sample.0, sample.1, sample.2, w));
                    }
                }

                if !assigned && sum_w > 0.0 {
                    if poly_order == RbfPolyOrder::None {
                        out_values[row * cols + col] = weighted_sum / sum_w;
                    } else if let Some(zn) = weighted_poly_predict(x, y, &poly_neighbors, poly_order) {
                        out_values[row * cols + col] = zn;
                    } else {
                        out_values[row * cols + col] = weighted_sum / sum_w;
                    }
                }
            }
            ctx.progress.progress((row + 1) as f64 / rows.max(1) as f64 * 0.99);
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = store_or_write_output(output, output_path)?;
        ctx.progress.progress(1.0);
        Ok(build_raster_result(locator))
    }
}

impl Tool for LidarSibsonInterpolationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_sibson_interpolation",
            display_name: "LiDAR Sibson Interpolation",
            summary: "Interpolates a raster from LiDAR points using true Sibson natural-neighbour interpolation.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.",
                    required: false,
                },
                ToolParamSpec {
                    name: "resolution",
                    description: "Output cell size.",
                    required: false,
                },
                ToolParamSpec {
                    name: "interpolation_parameter",
                    description: "Point attribute to interpolate (elevation, intensity, class, return_number, number_of_returns, scan_angle, time, rgb, user_data).",
                    required: false,
                },
                ToolParamSpec {
                    name: "returns_included",
                    description: "Return filtering mode: all, first, or last.",
                    required: false,
                },
                ToolParamSpec {
                    name: "excluded_classes",
                    description: "Classes to exclude (array or comma-delimited string).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_elev",
                    description: "Minimum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_elev",
                    description: "Maximum elevation threshold used for point inclusion filtering.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/cell_size must be a positive finite value".to_string(),
            ));
        }
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation(
                "min_elev/minz must be <= max_elev/maxz".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        if input_path.is_none() {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_paths: Vec<String> = files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let mut tile_args = args.clone();
                    let input_str = input.to_string_lossy().to_string();
                    tile_args.insert("input".to_string(), json!(input_str.clone()));
                    let neighbors: Vec<String> = all_paths
                        .iter()
                        .filter(|p| *p != &input_str)
                        .cloned()
                        .collect();
                    tile_args.insert("batch_neighbor_inputs".to_string(), json!(neighbors));
                    tile_args.remove("input_lidar");
                    let out = generate_batch_output_path(&input, "sibson");
                    tile_args.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                    let result = self.run(&tile_args, ctx)?;
                    extract_raster_path_from_result(result, "lidar_sibson_interpolation")
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            return build_batch_placeholder_raster_result(outputs);
        }
        let input_path = input_path.expect("checked above");
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let parameter = args
            .get("interpolation_parameter")
            .or_else(|| args.get("parameter"))
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let neighbor_paths = parse_batch_neighbor_inputs(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        require_lidar_crs(cloud.crs.as_ref(), "lidar_sibson_interpolation")?;

        let target_samples = collect_lidar_samples(
            &cloud.points,
            &parameter,
            returns_mode,
            &include_classes,
            min_z,
            max_z,
        )?;
        let mut samples = target_samples.clone();
        for neighbor_path in neighbor_paths {
            let n_cloud = PointCloud::read(&neighbor_path)
                .map_err(|e| ToolError::Execution(format!("failed reading batch-neighbor lidar '{}': {e}", neighbor_path)))?;
            let mut n_samples = collect_lidar_samples(
                &n_cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            samples.append(&mut n_samples);
        }
        if samples.len() < 3 || target_samples.is_empty() {
            return Err(ToolError::Validation(
                "input lidar must contain at least three points for interpolation".to_string(),
            ));
        }

        let mut output = build_lidar_output(
            &target_samples,
            resolution,
            lidar_crs_to_raster_crs(cloud.crs.as_ref()),
            DataType::F64,
        )?;

        let topo_points: Vec<TopoCoord> = samples
            .iter()
            .map(|(x, y, _)| TopoCoord::xy(*x, *y))
            .collect();
        let sibson = PreparedSibsonInterpolator::new(&topo_points, 1.0e-12);
        if sibson.triangles.is_empty() {
            return Err(ToolError::Execution(
                "failed to build triangulation from input lidar points".to_string(),
            ));
        }

        let mut value_lookup = HashMap::with_capacity(samples.len());
        for (x, y, value) in &samples {
            value_lookup.entry(point_key_bits(*x, *y)).or_insert(*value);
        }
        let point_values: Vec<f64> = sibson
            .points
            .iter()
            .map(|p| {
                value_lookup
                    .get(&point_key_bits(p.x, p.y))
                    .copied()
                    .ok_or_else(|| ToolError::Execution("point value lookup failed".to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let hull = convex_hull_2d(
            &sibson
                .points
                .iter()
                .map(|p| (p.x, p.y))
                .collect::<Vec<_>>(),
        );
        let hull_min_x = hull.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let hull_max_x = hull
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let hull_min_y = hull.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let hull_max_y = hull
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);

        let rows = output.rows;
        let cols = output.cols;
        let nodata = output.nodata;
        let x_min = output.x_min;
        let y_max = output.y_max();
        let cell_x = output.cell_size_x;
        let cell_y = output.cell_size_y;
        let row_results: Vec<(usize, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|row| -> Result<(usize, Vec<f64>), ToolError> {
                let mut row_values = vec![nodata; cols];
                let y = y_max - (row as f64 + 0.5) * cell_y;
                let mut scratch = sibson.new_scratch();

                if y < hull_min_y || y > hull_max_y {
                    return Ok((row, row_values));
                }

                let Some((scan_min_x, scan_max_x)) = convex_hull_scanline_span(y, &hull, 1.0e-12) else {
                    return Ok((row, row_values));
                };
                let start_col_f = ((scan_min_x - x_min) / cell_x - 0.5).ceil();
                let end_col_f = ((scan_max_x - x_min) / cell_x - 0.5).floor();
                if !start_col_f.is_finite() || !end_col_f.is_finite() {
                    return Ok((row, row_values));
                }
                let start_col = start_col_f.max(0.0) as usize;
                let end_col = end_col_f.min(cols as f64 - 1.0).max(-1.0) as isize;
                if end_col < start_col as isize {
                    return Ok((row, row_values));
                }

                for col in start_col..=(end_col as usize) {
                    let cell = &mut row_values[col];
                    let x = x_min + (col as f64 + 0.5) * cell_x;
                    if x < hull_min_x || x > hull_max_x {
                        continue;
                    }

                    if let Some(value) = sibson.interpolate_with_scratch(
                        TopoCoord::xy(x, y),
                        &point_values,
                        &mut scratch,
                    ) {
                        *cell = value;
                    }
                }

                Ok((row, row_values))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut out_values = vec![nodata; output.data.len()];
        let mut completed = 0usize;
        for (row, row_values) in row_results {
            let start = row * cols;
            out_values[start..start + cols].copy_from_slice(&row_values);
            completed += 1;
            ctx.progress.progress(completed as f64 / rows.max(1) as f64 * 0.99);
        }

        for (idx, value) in out_values.iter().enumerate() {
            output.data.set_f64(idx, *value);
        }

        let locator = store_or_write_output(output, output_path)?;
        ctx.progress.progress(1.0);
        Ok(build_raster_result(locator))
    }
}

impl Tool for LidarBlockMaximumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_block_maximum",
            display_name: "LiDAR Block Maximum",
            summary: "Creates a raster by assigning each cell the maximum value of included LiDAR points.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output cell size.", required: false },
                ToolParamSpec { name: "interpolation_parameter", description: "Point attribute (elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data).", required: false },
                ToolParamSpec { name: "returns_included", description: "Return filtering mode: all, first, or last.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Classes to exclude (array or comma-delimited string).", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let parameter = args.get("interpolation_parameter").or_else(|| args.get("parameter")).and_then(Value::as_str).unwrap_or("elevation").to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation("min_elev/minz must be <= max_elev/maxz".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let parameter = args.get("interpolation_parameter").or_else(|| args.get("parameter")).and_then(Value::as_str).unwrap_or("elevation").to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_block_extrema_tile(
                Path::new(&input_path),
                output_path.as_deref(),
                resolution,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
                true,
            )?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let total = files.len().max(1);
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out_path = generate_batch_output_path(&input, "block_max");
                    run_block_extrema_tile(
                        &input,
                        Some(out_path.as_path()),
                        resolution,
                        &parameter,
                        returns_mode,
                        &include_classes,
                        min_z,
                        max_z,
                        true,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress((outputs.len() as f64 / total as f64).min(1.0));
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for LidarBlockMinimumTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_block_minimum",
            display_name: "LiDAR Block Minimum",
            summary: "Creates a raster by assigning each cell the minimum value of included LiDAR points.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output cell size.", required: false },
                ToolParamSpec { name: "interpolation_parameter", description: "Point attribute (elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data).", required: false },
                ToolParamSpec { name: "returns_included", description: "Return filtering mode: all, first, or last.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Classes to exclude (array or comma-delimited string).", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let parameter = args.get("interpolation_parameter").or_else(|| args.get("parameter")).and_then(Value::as_str).unwrap_or("elevation").to_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'; expected elevation/intensity/class/return_number/number_of_returns/scan_angle/time/rgb/user_data",
                parameter
            )));
        }
        let _ = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        if min_z > max_z {
            return Err(ToolError::Validation("min_elev/minz must be <= max_elev/maxz".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let parameter = args.get("interpolation_parameter").or_else(|| args.get("parameter")).and_then(Value::as_str).unwrap_or("elevation").to_lowercase();
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_block_extrema_tile(
                Path::new(&input_path),
                output_path.as_deref(),
                resolution,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
                false,
            )?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let total = files.len().max(1);
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out_path = generate_batch_output_path(&input, "block_min");
                    run_block_extrema_tile(
                        &input,
                        Some(out_path.as_path()),
                        resolution,
                        &parameter,
                        returns_mode,
                        &include_classes,
                        min_z,
                        max_z,
                        false,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress((outputs.len() as f64 / total as f64).min(1.0));
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for LidarPointDensityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_point_density",
            display_name: "LiDAR Point Density",
            summary: "Computes point density from LiDAR samples within a moving-radius neighbourhood.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output cell size.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for point counting.", required: false },
                ToolParamSpec { name: "returns_included", description: "Return filtering mode: all, first, or last.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Classes to exclude (array or comma-delimited string).", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let _ = parse_excluded_classes(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_point_density_tile(
                Path::new(&input_path),
                output_path.as_deref(),
                resolution,
                radius,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out_path = generate_batch_output_path(&input, "density");
                    run_point_density_tile(
                        &input,
                        Some(out_path.as_path()),
                        resolution,
                        radius,
                        returns_mode,
                        &include_classes,
                        min_z,
                        max_z,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for LidarDigitalSurfaceModelTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_digital_surface_model",
            display_name: "LiDAR Digital Surface Model",
            summary: "Builds a DSM from top-surface LiDAR points and TIN interpolation.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output cell size.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for selecting top-surface candidates.", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_triangle_edge_length", description: "Optional maximum triangle edge length to suppress long-edge facets.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 0.5);
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 0.5);
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let max_triangle_edge_length_raw = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
        let max_triangle_edge_length = if max_triangle_edge_length_raw <= 0.0 { f64::INFINITY } else { max_triangle_edge_length_raw };
        let output_path = parse_optional_output_path(args, "output")?;

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_dsm_tile(
                Path::new(&input_path),
                output_path.as_deref(),
                resolution,
                radius,
                min_z,
                max_z,
                max_triangle_edge_length,
            )?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out_path = generate_batch_output_path(&input, "dsm");
                    run_dsm_tile(
                        &input,
                        Some(out_path.as_path()),
                        resolution,
                        radius,
                        min_z,
                        max_z,
                        max_triangle_edge_length,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for LidarHillshadeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_hillshade",
            display_name: "LiDAR Hillshade",
            summary: "Creates a hillshade raster from LiDAR elevations using local block maxima as surface input.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output cell size.", required: false },
                ToolParamSpec { name: "search_radius", description: "Compatibility parameter reserved for legacy hillshade neighborhood control.", required: false },
                ToolParamSpec { name: "azimuth", description: "Illumination azimuth in degrees.", required: false },
                ToolParamSpec { name: "altitude", description: "Illumination altitude in degrees.", required: false },
                ToolParamSpec { name: "returns_included", description: "Return filtering mode: all, first, or last.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Classes to exclude (array or comma-delimited string).", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum elevation threshold used for point inclusion filtering.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], -1.0);
        if !search_radius.is_finite() {
            return Err(ToolError::Validation("search_radius/radius must be finite".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let _search_radius = parse_f64_alias(args, &["search_radius", "radius"], -1.0);
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let output_path = parse_optional_output_path(args, "output")?;
        let (azimuth, altitude) = parse_azimuth_altitude(args);

        let run_single = |input_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(input_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path.to_string_lossy())))?;
            require_lidar_crs(cloud.crs.as_ref(), "lidar_hillshade")?;

            let samples = collect_lidar_samples(&cloud.points, "elevation", returns_mode, &include_classes, min_z, max_z)?;
            let mut output = build_lidar_output(&samples, resolution, lidar_crs_to_raster_crs(cloud.crs.as_ref()), DataType::F64)?;

            // Build local surface from block maxima then hillshade it.
            let mut surface = vec![output.nodata; output.data.len()];
            let y_max = output.y_max();
            for (x, y, z) in &samples {
                if let Some(idx) = raster_cell_index(*x, *y, output.x_min, y_max, output.cell_size_x, output.cell_size_y, output.rows, output.cols) {
                    if surface[idx] == output.nodata || *z > surface[idx] {
                        surface[idx] = *z;
                    }
                }
            }

            let shaded = hillshade_from_raster(
                &surface,
                output.rows,
                output.cols,
                output.cell_size_x,
                output.cell_size_y,
                output.nodata,
                azimuth,
                altitude,
            );
            for (idx, value) in shaded.iter().enumerate() {
                output.data.set_f64(idx, *value);
            }

            store_or_write_output(output, out_path)
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_output_path(&input, "hillshade");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for FilterLidarClassesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar_classes",
            display_name: "Filter LiDAR Classes",
            summary: "Removes points that match excluded classification values.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Classes to exclude (array or comma-delimited string).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let _ = parse_excluded_classes(args)?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let include_classes = parse_excluded_classes(args)?;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let points: Vec<PointRecord> = cloud
                .points
                .iter()
                .filter(|p| include_classes[p.classification as usize])
                .cloned()
                .collect();
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar_classes")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "filtered_cls");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for LidarShiftTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_shift",
            display_name: "LiDAR Shift",
            summary: "Shifts LiDAR point coordinates by x/y/z offsets.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "x_shift", description: "Shift to add to x coordinates.", required: false },
                ToolParamSpec { name: "y_shift", description: "Shift to add to y coordinates.", required: false },
                ToolParamSpec { name: "z_shift", description: "Shift to add to z coordinates.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let x_shift = parse_f64_alias(args, &["x_shift"], 0.0);
        let y_shift = parse_f64_alias(args, &["y_shift"], 0.0);
        let z_shift = parse_f64_alias(args, &["z_shift"], 0.0);
        if !(x_shift.is_finite() && y_shift.is_finite() && z_shift.is_finite()) {
            return Err(ToolError::Validation("x_shift/y_shift/z_shift must be finite values".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let x_shift = parse_f64_alias(args, &["x_shift"], 0.0);
        let y_shift = parse_f64_alias(args, &["y_shift"], 0.0);
        let z_shift = parse_f64_alias(args, &["z_shift"], 0.0);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let points: Vec<PointRecord> = cloud
                .points
                .iter()
                .map(|p| {
                    let mut q = p.clone();
                    q.x += x_shift;
                    q.y += y_shift;
                    q.z += z_shift;
                    q
                })
                .collect();
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "lidar_shift")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "shifted");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for RemoveDuplicatesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "remove_duplicates",
            display_name: "Remove Duplicates",
            summary: "Removes duplicate LiDAR points using x/y and optionally z coordinates.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "include_z", description: "If true, duplicate detection includes z; otherwise uses x/y only.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let _ = parse_bool_alias(args, &["include_z"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let include_z = parse_bool_alias(args, &["include_z"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let mut seen_xy: HashSet<(u64, u64)> = HashSet::with_capacity(cloud.points.len());
            let mut seen_xyz: HashSet<(u64, u64, u64)> = HashSet::with_capacity(cloud.points.len());
            let mut points = Vec::with_capacity(cloud.points.len());

            for p in &cloud.points {
                if include_z {
                    let key = (p.x.to_bits(), p.y.to_bits(), p.z.to_bits());
                    if seen_xyz.insert(key) {
                        points.push(p.clone());
                    }
                } else {
                    let key = (p.x.to_bits(), p.y.to_bits());
                    if seen_xy.insert(key) {
                        points.push(p.clone());
                    }
                }
            }

            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "remove_duplicates")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "dedup");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for FilterLidarScanAnglesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar_scan_angles",
            display_name: "Filter LiDAR Scan Angles",
            summary: "Removes LiDAR points whose absolute scan angle exceeds a threshold.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "threshold", description: "Maximum absolute scan angle (integer LAS units; 1 unit = 0.006°). Required.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let threshold = parse_f64_alias(args, &["threshold"], f64::NAN);
        if threshold.is_nan() {
            return Err(ToolError::Validation("threshold is required".to_string()));
        }
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(ToolError::Validation("threshold must be a non-negative finite value".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let threshold = parse_f64_alias(args, &["threshold"], 0.0) as i16;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let points: Vec<PointRecord> = cloud
                .points
                .iter()
                .filter(|p| p.scan_angle.abs() <= threshold)
                .cloned()
                .collect();
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar_scan_angles")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "scan_filtered");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for FilterLidarNoiseTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar_noise",
            display_name: "Filter LiDAR Noise",
            summary: "Removes low (class 7) and high (class 18) noise-classified points from a LiDAR file.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let points: Vec<PointRecord> = cloud
                .points
                .iter()
                .filter(|p| p.classification != 7 && p.classification != 18)
                .cloned()
                .collect();
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar_noise")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "denoised");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for LidarThinTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_thin",
            display_name: "LiDAR Thin",
            summary: "Thins a LiDAR point cloud by retaining at most one point per grid cell.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Grid cell size for thinning (default 1.0).", required: false },
                ToolParamSpec { name: "method", description: "Point selection method per cell: first, last, lowest, highest, nearest (default 'first').", required: false },
                ToolParamSpec { name: "save_filtered", description: "If true, also writes filtered-out points and returns their locator in outputs.filtered_path.", required: false },
                ToolParamSpec { name: "filtered_output", description: "Optional output path for filtered-out points (single-input mode).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution must be a positive finite value".to_string()));
        }
        if let Some(method) = args.get("method").and_then(Value::as_str) {
            match method.to_lowercase().as_str() {
                "first" | "last" | "lowest" | "highest" | "nearest" => {}
                other => return Err(ToolError::Validation(
                    format!("method '{}' is not recognised; use first, last, lowest, highest, or nearest", other)
                )),
            }
        }
        let _ = parse_bool_alias(args, &["save_filtered"], false);
        let _ = parse_optional_output_path(args, "filtered_output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution"], 1.0);
        let save_filtered = parse_bool_alias(args, &["save_filtered"], false);
        let method = args
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("first")
            .to_lowercase();
        let output_path = parse_optional_lidar_output_path(args)?;
        let filtered_output_path = parse_optional_output_path(args, "filtered_output")?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>, filtered_out_path: Option<PathBuf>| -> Result<(String, Option<String>), ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let n = cloud.points.len();
            if n == 0 {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                let kept_path = store_or_write_lidar_output(&out_cloud, out_path, "lidar_thin")?;
                let filtered_path = if save_filtered {
                    let filtered_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                    Some(store_or_write_lidar_output(&filtered_cloud, filtered_out_path, "lidar_thin_filtered")?)
                } else {
                    None
                };
                return Ok((kept_path, filtered_path));
            }

            // Compute bounding box
            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

            // Map each point to a grid cell key (col, row)
            let cell_key = |px: f64, py: f64| -> (i64, i64) {
                let col = ((px - min_x) / resolution).floor() as i64;
                let row = ((max_y - py) / resolution).floor() as i64;
                (col, row)
            };

            // Track which point index wins each cell; n_points sentinel = no winner yet
            let mut cell_winner: HashMap<(i64, i64), usize> = HashMap::new();

            match method.as_str() {
                "first" => {
                    for (i, p) in cloud.points.iter().enumerate() {
                        cell_winner.entry(cell_key(p.x, p.y)).or_insert(i);
                    }
                }
                "last" => {
                    for (i, p) in cloud.points.iter().enumerate() {
                        cell_winner.insert(cell_key(p.x, p.y), i);
                    }
                }
                "lowest" => {
                    for (i, p) in cloud.points.iter().enumerate() {
                        let key = cell_key(p.x, p.y);
                        let winner = cell_winner.entry(key).or_insert(i);
                        if p.z < cloud.points[*winner].z {
                            *winner = i;
                        }
                    }
                }
                "highest" => {
                    for (i, p) in cloud.points.iter().enumerate() {
                        let key = cell_key(p.x, p.y);
                        let winner = cell_winner.entry(key).or_insert(i);
                        if p.z > cloud.points[*winner].z {
                            *winner = i;
                        }
                    }
                }
                "nearest" => {
                    // Track minimum squared distance from each point to its cell centre
                    let mut cell_min_dist: HashMap<(i64, i64), f64> = HashMap::new();
                    for (i, p) in cloud.points.iter().enumerate() {
                        let key = cell_key(p.x, p.y);
                        let (col, row) = key;
                        let centre_x = min_x + (col as f64 + 0.5) * resolution;
                        let centre_y = max_y - (row as f64 + 0.5) * resolution;
                        let dist2 = (p.x - centre_x).powi(2) + (p.y - centre_y).powi(2);
                        let winner = cell_winner.entry(key).or_insert(i);
                        let min_dist = cell_min_dist.entry(key).or_insert(f64::INFINITY);
                        if dist2 < *min_dist {
                            *winner = i;
                            *min_dist = dist2;
                        }
                    }
                }
                _ => {
                    return Err(ToolError::Validation(format!(
                        "method '{}' is not recognised; use first, last, lowest, highest, or nearest",
                        method
                    )));
                }
            }

            // Build output using only winning indices
            let mut keep = vec![false; n];
            for idx in cell_winner.values() {
                keep[*idx] = true;
            }
            let mut kept_points = Vec::with_capacity(n);
            let mut filtered_points = if save_filtered { Vec::with_capacity(n) } else { Vec::new() };
            for (i, p) in cloud.points.iter().enumerate() {
                if keep[i] {
                    kept_points.push(*p);
                } else if save_filtered {
                    filtered_points.push(*p);
                }
            }

            let out_cloud = PointCloud { points: kept_points, crs: cloud.crs.clone() };
            let kept_path = store_or_write_lidar_output(&out_cloud, out_path, "lidar_thin")?;
            let filtered_path = if save_filtered {
                let filtered_cloud = PointCloud { points: filtered_points, crs: cloud.crs.clone() };
                Some(store_or_write_lidar_output(&filtered_cloud, filtered_out_path, "lidar_thin_filtered")?)
            } else {
                None
            };
            Ok((kept_path, filtered_path))
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let (locator, filtered_locator) = run_single(Path::new(&input_path), output_path, filtered_output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result_with_filtered(locator, filtered_locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "thinned");
                    let filtered_out = if save_filtered {
                        Some(generate_batch_lidar_output_path(&input, "thinned_filtered"))
                    } else {
                        None
                    };
                    run_single(&input, Some(out), filtered_out)
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            let mut kept_paths: Vec<String> = Vec::with_capacity(outputs.len());
            let mut filtered_paths: Vec<String> = Vec::new();
            for (kept, filtered) in outputs {
                kept_paths.push(kept);
                if let Some(fp) = filtered {
                    filtered_paths.push(fp);
                }
            }
            if kept_paths.is_empty() {
                return Err(ToolError::Execution("batch mode produced no output lidar files".to_string()));
            }
            kept_paths.sort();
            if !filtered_paths.is_empty() {
                filtered_paths.sort();
                Ok(build_lidar_result_with_filtered(
                    kept_paths[0].clone(),
                    Some(filtered_paths[0].clone()),
                ))
            } else {
                Ok(build_lidar_result(kept_paths[0].clone()))
            }
        }
    }
}

impl Tool for LidarElevationSliceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_elevation_slice",
            display_name: "LiDAR Elevation Slice",
            summary: "Extracts or reclassifies LiDAR points within a specified elevation range.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "minz", description: "Lower bound of the elevation slice (default -infinity).", required: false },
                ToolParamSpec { name: "maxz", description: "Upper bound of the elevation slice (default +infinity).", required: false },
                ToolParamSpec { name: "classify", description: "If true, reclassify points instead of filtering them (default false).", required: false },
                ToolParamSpec { name: "in_class_value", description: "Classification assigned to points inside the slice when classify=true (default 2).", required: false },
                ToolParamSpec { name: "out_class_value", description: "Classification assigned to points outside the slice when classify=true (default 1).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let minz = parse_f64_alias(args, &["minz", "min_elev"], f64::NEG_INFINITY);
        let maxz = parse_f64_alias(args, &["maxz", "max_elev"], f64::INFINITY);
        if minz > maxz {
            return Err(ToolError::Validation("minz must be <= maxz".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let minz = parse_f64_alias(args, &["minz", "min_elev"], f64::NEG_INFINITY);
        let maxz = parse_f64_alias(args, &["maxz", "max_elev"], f64::INFINITY);
        let classify = parse_bool_alias(args, &["classify"], false);
        let in_class_value = parse_f64_alias(args, &["in_class_value"], 2.0) as u8;
        let out_class_value = parse_f64_alias(args, &["out_class_value"], 1.0) as u8;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let points: Vec<PointRecord> = if !classify {
                // Filter mode: keep only points inside slice
                cloud
                    .points
                    .iter()
                    .filter(|p| p.z >= minz && p.z <= maxz)
                    .cloned()
                    .collect()
            } else {
                // Classify mode: reassign classification, keep all points
                cloud
                    .points
                    .iter()
                    .map(|p| {
                        let mut q = *p;
                        q.classification = if p.z >= minz && p.z <= maxz {
                            in_class_value
                        } else {
                            out_class_value
                        };
                        q
                    })
                    .collect()
            };
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "lidar_elevation_slice")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "elev_slice");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for LidarJoinTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_join",
            display_name: "LiDAR Join",
            summary: "Merges multiple LiDAR files into a single output point cloud.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of input LiDAR paths or typed LiDAR objects.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_inputs_arg(args)?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let inputs = parse_lidar_inputs_arg(args)?;
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("reading input lidar files");
        let mut out_points: Vec<PointRecord> = Vec::new();
        let mut out_crs: Option<LidarCrs> = None;
        for input in &inputs {
            let cloud = PointCloud::read(input)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input)))?;
            if out_crs.is_none() {
                out_crs = cloud.crs.clone();
            }
            out_points.extend(cloud.points.iter().copied());
        }

        let out_cloud = PointCloud {
            points: out_points,
            crs: out_crs,
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_join")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarThinHighDensityTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_thin_high_density",
            display_name: "LiDAR Thin High Density",
            summary: "Thins points in locally high-density areas while preserving lower-density regions.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "density", description: "Target point density threshold in points per square unit.", required: true },
                ToolParamSpec { name: "resolution", description: "Grid resolution used for x/y and z binning (default 1.0).", required: false },
                ToolParamSpec { name: "save_filtered", description: "If true, writes filtered-out points and returns outputs.filtered_path.", required: false },
                ToolParamSpec { name: "filtered_output", description: "Optional output path for filtered-out points in single-input mode.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let density = parse_f64_alias(args, &["density"], f64::NAN);
        if density.is_nan() || !density.is_finite() || density <= 0.0 {
            return Err(ToolError::Validation("density is required and must be a positive finite value".to_string()));
        }
        let resolution = parse_f64_alias(args, &["resolution"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution must be a positive finite value".to_string()));
        }
        let _ = parse_bool_alias(args, &["save_filtered"], false);
        let _ = parse_optional_output_path(args, "filtered_output")?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let density = parse_f64_alias(args, &["density"], 1.0);
        let resolution = parse_f64_alias(args, &["resolution"], 1.0);
        let save_filtered = parse_bool_alias(args, &["save_filtered"], false);
        let output_path = parse_optional_lidar_output_path(args)?;
        let filtered_output_path = parse_optional_output_path(args, "filtered_output")?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>, filtered_out_path: Option<PathBuf>| -> Result<(String, Option<String>), ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let n_points = cloud.points.len();
            if n_points == 0 {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                let kept_path = store_or_write_lidar_output(&out_cloud, out_path, "lidar_thin_high_density")?;
                let filtered_path = if save_filtered {
                    let filtered_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                    Some(store_or_write_lidar_output(&filtered_cloud, filtered_out_path, "lidar_thin_high_density_filtered")?)
                } else {
                    None
                };
                return Ok((kept_path, filtered_path));
            }

            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let half_res = resolution / 2.0;
            let cols = (((max_x - min_x) / resolution).ceil() as i64).max(1);
            let rows = (((max_y - min_y) / resolution).ceil() as i64).max(1);
            let ew_range = (cols as f64 * resolution).max(resolution);
            let ns_range = (rows as f64 * resolution).max(resolution);

            let mut bins: HashMap<(i64, i64), Vec<(f64, usize)>> = HashMap::new();
            for (idx, p) in cloud.points.iter().enumerate() {
                let col = (((cols - 1) as f64 * (p.x - min_x - half_res) / ew_range).round()) as i64;
                let row = (((rows - 1) as f64 * (max_y - half_res - p.y) / ns_range).round()) as i64;
                bins.entry((row, col)).or_default().push((p.z, idx));
            }

            let threshold = resolution * resolution * density;
            let mut filtered = vec![false; n_points];
            for vals in bins.values() {
                if vals.len() as f64 <= threshold {
                    continue;
                }

                let minz = vals.iter().map(|v| v.0).fold(f64::INFINITY, f64::min);
                let maxz = vals.iter().map(|v| v.0).fold(f64::NEG_INFINITY, f64::max);
                let mut num_bins = ((maxz - minz) / resolution).ceil() as usize;
                if ((maxz - minz) % resolution).abs() < f64::EPSILON {
                    num_bins += 1;
                }
                if num_bins == 0 {
                    num_bins = 1;
                }

                let mut histo = vec![0.0_f64; num_bins];
                for (z, _) in vals {
                    let mut b = ((z - minz) / resolution).floor() as usize;
                    if b >= num_bins {
                        b = num_bins - 1;
                    }
                    histo[b] += 1.0;
                }

                let mut skip_factor = vec![1usize; num_bins];
                for i in 0..num_bins {
                    if histo[i] > threshold {
                        skip_factor[i] = (histo[i] / threshold).floor() as usize;
                        if skip_factor[i] == 0 {
                            skip_factor[i] = 1;
                        }
                    }
                }

                let mut skipped = vec![0usize; num_bins];
                for (z, idx) in vals {
                    let mut b = ((z - minz) / resolution).floor() as usize;
                    if b >= num_bins {
                        b = num_bins - 1;
                    }
                    if histo[b] > threshold {
                        skipped[b] += 1;
                        if skipped[b] <= skip_factor[b] {
                            filtered[*idx] = true;
                        } else {
                            skipped[b] = 0;
                        }
                    }
                }
            }

            let mut kept_points = Vec::with_capacity(n_points);
            let mut filtered_points = if save_filtered { Vec::with_capacity(n_points) } else { Vec::new() };
            for (i, p) in cloud.points.iter().enumerate() {
                if filtered[i] {
                    if save_filtered {
                        filtered_points.push(*p);
                    }
                } else {
                    kept_points.push(*p);
                }
            }

            let kept_cloud = PointCloud {
                points: kept_points,
                crs: cloud.crs.clone(),
            };
            let kept_path = store_or_write_lidar_output(&kept_cloud, out_path, "lidar_thin_high_density")?;
            let filtered_path = if save_filtered {
                let filtered_cloud = PointCloud {
                    points: filtered_points,
                    crs: cloud.crs.clone(),
                };
                Some(store_or_write_lidar_output(
                    &filtered_cloud,
                    filtered_out_path,
                    "lidar_thin_high_density_filtered",
                )?)
            } else {
                None
            };
            Ok((kept_path, filtered_path))
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let (locator, filtered_locator) = run_single(Path::new(&input_path), output_path, filtered_output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result_with_filtered(locator, filtered_locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "thinned_hd");
                    let filtered_out = if save_filtered {
                        Some(generate_batch_lidar_output_path(&input, "thinned_hd_filtered"))
                    } else {
                        None
                    };
                    run_single(&input, Some(out), filtered_out)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut kept_paths = Vec::with_capacity(outputs.len());
            let mut filtered_paths = Vec::new();
            for (k, f) in outputs {
                kept_paths.push(k);
                if let Some(fp) = f {
                    filtered_paths.push(fp);
                }
            }
            kept_paths.sort();
            filtered_paths.sort();
            ctx.progress.progress(1.0);
            if kept_paths.is_empty() {
                return Err(ToolError::Execution("batch mode produced no output lidar files".to_string()));
            }
            if !filtered_paths.is_empty() {
                Ok(build_lidar_result_with_filtered(
                    kept_paths[0].clone(),
                    Some(filtered_paths[0].clone()),
                ))
            } else {
                Ok(build_lidar_result(kept_paths[0].clone()))
            }
        }
    }
}

impl Tool for LidarTileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_tile",
            display_name: "LiDAR Tile",
            summary: "Splits an input LiDAR file into a regular tile grid and writes one output per populated tile.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "tile_width", description: "Tile width in x units (default 1000).", required: false },
                ToolParamSpec { name: "tile_height", description: "Tile height in y units (default 1000).", required: false },
                ToolParamSpec { name: "origin_x", description: "Grid origin x coordinate (default 0).", required: false },
                ToolParamSpec { name: "origin_y", description: "Grid origin y coordinate (default 0).", required: false },
                ToolParamSpec { name: "min_points_in_tile", description: "Minimum points required for writing a tile (default 2).", required: false },
                ToolParamSpec { name: "output_laz_format", description: "If true, writes .laz outputs; otherwise .las (default true).", required: false },
                ToolParamSpec { name: "output_directory", description: "Optional directory to write tile outputs; defaults to <input_dir>/<input_stem>/.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = parse_lidar_path_arg_optional(args)?;
        if input.is_none() {
            return Err(ToolError::Validation("input is required".to_string()));
        }
        let tile_width = parse_f64_alias(args, &["tile_width", "width"], 1000.0);
        let tile_height = parse_f64_alias(args, &["tile_height", "height"], 1000.0);
        if !tile_width.is_finite() || !tile_height.is_finite() || tile_width <= 0.0 || tile_height <= 0.0 {
            return Err(ToolError::Validation("tile_width/tile_height must be positive finite values".to_string()));
        }
        let _ = parse_f64_alias(args, &["origin_x"], 0.0);
        let _ = parse_f64_alias(args, &["origin_y"], 0.0);
        let _ = parse_bool_alias(args, &["output_laz_format"], true);
        let _ = parse_optional_output_path(args, "output_directory")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let tile_width = parse_f64_alias(args, &["tile_width", "width"], 1000.0);
        let tile_height = parse_f64_alias(args, &["tile_height", "height"], 1000.0);
        let origin_x = parse_f64_alias(args, &["origin_x"], 0.0);
        let origin_y = parse_f64_alias(args, &["origin_y"], 0.0);
        let mut min_points = parse_f64_alias(args, &["min_points_in_tile", "min_points"], 2.0) as usize;
        let output_laz = parse_bool_alias(args, &["output_laz_format"], true);
        let output_dir_override = parse_optional_output_path(args, "output_directory")?;
        if min_points < 2 {
            min_points = 2;
        }

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        if cloud.points.is_empty() {
            return Err(ToolError::Execution("input lidar contains no points".to_string()));
        }

        let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        let start_x_grid = ((min_x - origin_x) / tile_width).floor();
        let end_x_grid = ((max_x - origin_x) / tile_width).ceil();
        let start_y_grid = ((min_y - origin_y) / tile_height).floor();
        let end_y_grid = ((max_y - origin_y) / tile_height).ceil();
        let cols = (end_x_grid - start_x_grid).abs() as usize;
        let rows = (end_y_grid - start_y_grid).abs() as usize;
        let num_tiles = rows.saturating_mul(cols);
        if num_tiles == 0 {
            return Err(ToolError::Execution("no output tiles would be created with current parameters".to_string()));
        }
        if num_tiles > 32767 {
            return Err(ToolError::Validation("too many output tiles; increase tile width/height".to_string()));
        }

        let mut tile_index = vec![0usize; cloud.points.len()];
        let mut tile_counts = vec![0usize; num_tiles];
        for (i, p) in cloud.points.iter().enumerate() {
            let col_raw = (((p.x - origin_x) / tile_width) - start_x_grid).floor() as isize;
            let row_raw = (((p.y - origin_y) / tile_height) - start_y_grid).floor() as isize;
            let col = col_raw.clamp(0, cols as isize - 1) as usize;
            let row = row_raw.clamp(0, rows as isize - 1) as usize;
            let tid = row * cols + col;
            tile_index[i] = tid;
            tile_counts[tid] += 1;
        }

        let mut write_tile = vec![false; num_tiles];
        for tid in 0..num_tiles {
            if tile_counts[tid] > min_points {
                write_tile[tid] = true;
            }
        }

        let mut min_row = usize::MAX;
        let mut min_col = usize::MAX;
        for (tid, out) in write_tile.iter().enumerate() {
            if *out {
                let row = tid / cols;
                let col = tid % cols;
                if row < min_row {
                    min_row = row;
                }
                if col < min_col {
                    min_col = col;
                }
            }
        }
        if min_row == usize::MAX {
            return Err(ToolError::Execution("no tiles met min_points_in_tile threshold".to_string()));
        }

        let input_path_obj = Path::new(&input_path);
        let input_name = input_path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ToolError::Validation("input filename stem could not be determined".to_string()))?
            .to_string();
        let base_output_dir = if let Some(dir) = output_dir_override {
            dir
        } else {
            let parent = input_path_obj
                .parent()
                .ok_or_else(|| ToolError::Validation("input path parent directory could not be determined".to_string()))?;
            parent.join(&input_name)
        };
        fs::create_dir_all(&base_output_dir)
            .map_err(|e| ToolError::Execution(format!("failed creating output directory '{}': {e}", base_output_dir.to_string_lossy())))?;

        let mut buckets: Vec<Vec<PointRecord>> = vec![Vec::new(); num_tiles];
        for (i, p) in cloud.points.iter().enumerate() {
            let tid = tile_index[i];
            if write_tile[tid] {
                buckets[tid].push(*p);
            }
        }

        let ext = if output_laz { "laz" } else { "las" };
        let mut written_paths: Vec<String> = Vec::new();
        for tid in 0..num_tiles {
            if !write_tile[tid] {
                continue;
            }
            let row = tid / cols;
            let col = tid % cols;
            let out_name = format!(
                "{}_row{}_col{}.{}",
                input_name,
                row - min_row + 1,
                col - min_col + 1,
                ext
            );
            let out_path = base_output_dir.join(out_name);
            let out_cloud = PointCloud {
                points: buckets[tid].clone(),
                crs: cloud.crs.clone(),
            };
            out_cloud
                .write(&out_path)
                .map_err(|e| ToolError::Execution(format!("failed writing tiled lidar '{}': {e}", out_path.to_string_lossy())))?;
            written_paths.push(out_path.to_string_lossy().to_string());
        }

        if written_paths.is_empty() {
            return Err(ToolError::Execution("no output tiles were written".to_string()));
        }
        written_paths.sort();
        ctx.progress.progress(1.0);
        let mut result = build_lidar_result(written_paths[0].clone());
        result.outputs.insert("tile_count".to_string(), json!(written_paths.len()));
        Ok(result)
    }
}

impl Tool for SortLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "sort_lidar",
            display_name: "Sort LiDAR",
            summary: "Sorts points by one or more LiDAR properties, with optional bin sizes per criterion.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "sort_criteria", description: "Sort criteria expression, e.g. 'x 100, y 100, z 10, scan_angle'.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let criteria = args
            .get("sort_criteria")
            .or_else(|| args.get("criteria"))
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("sort_criteria is required".to_string()))?;
        let _ = parse_sort_criteria(criteria)?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let criteria_text = args
            .get("sort_criteria")
            .or_else(|| args.get("criteria"))
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("sort_criteria is required".to_string()))?;
        let criteria = parse_sort_criteria(criteria_text)?;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            let mut order: Vec<usize> = (0..cloud.points.len()).collect();
            order.par_sort_unstable_by(|a, b| {
                let pa = &cloud.points[*a];
                let pb = &cloud.points[*b];
                let mut cmp = Ordering::Equal;
                for (criterion, bin) in &criteria {
                    let va = sort_value(pa, *criterion);
                    let vb = sort_value(pb, *criterion);
                    cmp = if let Some(step) = bin {
                        if *step != 0.0 {
                            let ba = (va / *step).floor();
                            let bb = (vb / *step).floor();
                            ba.partial_cmp(&bb).unwrap_or(Ordering::Equal)
                        } else {
                            va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
                        }
                    } else {
                        va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
                    };
                    if cmp != Ordering::Equal {
                        break;
                    }
                }
                cmp
            });

            let points: Vec<PointRecord> = order.into_iter().map(|idx| cloud.points[idx]).collect();
            let out_cloud = PointCloud {
                points,
                crs: cloud.crs.clone(),
            };
            store_or_write_lidar_output(&out_cloud, out_path, "sort_lidar")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "sorted");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for FilterLidarByPercentileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar_by_percentile",
            display_name: "Filter LiDAR By Percentile",
            summary: "Selects one representative point per grid block based on elevation percentile.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "percentile", description: "Percentile in [0, 100] (0=lowest, 100=highest).", required: false },
                ToolParamSpec { name: "block_size", description: "Grid block size for local percentile selection (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let percentile = parse_f64_alias(args, &["percentile"], 0.0);
        if !percentile.is_finite() || !(0.0..=100.0).contains(&percentile) {
            return Err(ToolError::Validation("percentile must be a finite value in [0, 100]".to_string()));
        }
        let block_size = parse_f64_alias(args, &["block_size", "resolution"], 1.0);
        if !block_size.is_finite() || block_size <= 0.0 {
            return Err(ToolError::Validation("block_size/resolution must be a positive finite value".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let percentile = parse_f64_alias(args, &["percentile"], 0.0);
        let block_size = parse_f64_alias(args, &["block_size", "resolution"], 1.0);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar_by_percentile");
            }

            let west = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let north = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let rows = (((north - min_y) / block_size).ceil() as usize).max(1);
            let cols = (((max_x - west) / block_size).ceil() as usize).max(1);
            let south = north - rows as f64 * block_size;
            let east = west + cols as f64 * block_size;
            let ns_range = (north - south).max(block_size);
            let ew_range = (east - west).max(block_size);

            let mut cell_ids: Vec<Vec<usize>> = vec![Vec::new(); rows * cols];
            for (i, p) in cloud.points.iter().enumerate() {
                if point_is_withheld(p) || point_is_noise(p) {
                    continue;
                }
                let col = (((cols - 1) as f64 * ((p.x - west) / ew_range).clamp(0.0, 1.0)).floor()) as usize;
                let row = (((rows - 1) as f64 * ((north - p.y) / ns_range).clamp(0.0, 1.0)).floor()) as usize;
                cell_ids[row * cols + col].push(i);
            }

            let mut selected_ids = Vec::new();
            for ids in &cell_ids {
                if ids.is_empty() {
                    continue;
                }
                let mut sorted: Vec<(f64, usize)> = ids.iter().map(|id| (cloud.points[*id].z, *id)).collect();
                sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
                let idx = ((percentile / 100.0) * (sorted.len() - 1) as f64).round() as usize;
                selected_ids.push(sorted[idx].1);
            }
            selected_ids.sort_unstable();

            let points: Vec<PointRecord> = selected_ids.into_iter().map(|i| cloud.points[i]).collect();
            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar_by_percentile")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "percentile");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for SplitLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "split_lidar",
            display_name: "Split LiDAR",
            summary: "Splits LiDAR points into multiple output files based on a grouping criterion.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "split_criterion", description: "Grouping criterion: num_pts, x, y, z, intensity, class, user_data, point_source_id, scan_angle, or time.", required: true },
                ToolParamSpec { name: "interval", description: "Bin size for numeric criteria; for num_pts this is points-per-output-file.", required: false },
                ToolParamSpec { name: "min_pts", description: "Minimum points needed before writing a split output file (default 5).", required: false },
                ToolParamSpec { name: "output_directory", description: "Optional directory for split outputs.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let criterion_text = args
            .get("split_criterion")
            .or_else(|| args.get("criterion"))
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("split_criterion is required".to_string()))?;
        let criterion = parse_split_criterion(criterion_text)?;
        let interval = parse_f64_alias(args, &["interval"], 5.0);
        if !interval.is_finite() || interval <= 0.0 {
            return Err(ToolError::Validation("interval must be a positive finite value".to_string()));
        }
        if criterion == SplitCriterion::NumPts && interval.floor() < 100.0 {
            return Err(ToolError::Validation("for split_criterion=num_pts, interval must be at least 100".to_string()));
        }
        let _ = parse_f64_alias(args, &["min_pts"], 5.0);
        let _ = parse_optional_output_path(args, "output_directory")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let criterion_text = args
            .get("split_criterion")
            .or_else(|| args.get("criterion"))
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("split_criterion is required".to_string()))?;
        let criterion = parse_split_criterion(criterion_text)?;
        let interval = parse_f64_alias(args, &["interval"], 5.0);
        let min_pts = parse_f64_alias(args, &["min_pts"], 5.0).max(0.0) as usize;
        let output_dir_override = parse_optional_output_path(args, "output_directory")?;

        let run_single = |in_path: &Path| -> Result<Vec<String>, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                return Ok(Vec::new());
            }

            let (input_parent, stem, ext) = split_output_base(in_path)?;
            let out_dir = output_dir_override
                .clone()
                .unwrap_or(input_parent);
            fs::create_dir_all(&out_dir)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory '{}': {e}", out_dir.to_string_lossy())))?;

            let mut outputs: Vec<String> = Vec::new();

            if criterion == SplitCriterion::NumPts {
                let chunk = interval.floor() as usize;
                let mut split_idx = 1usize;
                for chunk_points in cloud.points.chunks(chunk) {
                    if chunk_points.len() <= min_pts {
                        split_idx += 1;
                        continue;
                    }
                    let suffix = format!("split{}", split_idx);
                    let out_path = split_output_path(&out_dir, &stem, &suffix, &ext);
                    let out_cloud = PointCloud {
                        points: chunk_points.to_vec(),
                        crs: cloud.crs.clone(),
                    };
                    out_cloud.write(&out_path).map_err(|e| {
                        ToolError::Execution(format!("failed writing split lidar '{}': {e}", out_path.to_string_lossy()))
                    })?;
                    outputs.push(out_path.to_string_lossy().to_string());
                    split_idx += 1;
                }
                return Ok(outputs);
            }

            let mut groups: BTreeMap<String, Vec<PointRecord>> = BTreeMap::new();
            for p in &cloud.points {
                let key = match criterion {
                    SplitCriterion::Class => format!("class{}", p.classification),
                    SplitCriterion::PointSourceId => format!("point_source_id{}", p.point_source_id),
                    _ => {
                        let v = split_value(p, criterion);
                        let bin = (v / interval).floor();
                        let label = (bin * interval).to_string();
                        match criterion {
                            SplitCriterion::X => format!("x{}", label),
                            SplitCriterion::Y => format!("y{}", label),
                            SplitCriterion::Z => format!("z{}", label),
                            SplitCriterion::Intensity => format!("intensity{}", label),
                            SplitCriterion::UserData => format!("user_data{}", label),
                            SplitCriterion::ScanAngle => format!("scan_angle{}", label),
                            SplitCriterion::Time => format!("time{}", label),
                            SplitCriterion::NumPts => "split".to_string(),
                            SplitCriterion::Class | SplitCriterion::PointSourceId => unreachable!(),
                        }
                    }
                };
                groups.entry(key).or_default().push(*p);
            }

            for (suffix, points) in groups {
                if points.len() <= min_pts {
                    continue;
                }
                let out_path = split_output_path(&out_dir, &stem, &suffix, &ext);
                let out_cloud = PointCloud {
                    points,
                    crs: cloud.crs.clone(),
                };
                out_cloud.write(&out_path).map_err(|e| {
                    ToolError::Execution(format!("failed writing split lidar '{}': {e}", out_path.to_string_lossy()))
                })?;
                outputs.push(out_path.to_string_lossy().to_string());
            }
            outputs.sort();
            Ok(outputs)
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let outputs = run_single(Path::new(&input_path))?;
            if outputs.is_empty() {
                return Err(ToolError::Execution("split_lidar produced no output files".to_string()));
            }
            let mut result = build_lidar_result(outputs[0].clone());
            result.outputs.insert("output_count".to_string(), json!(outputs.len()));
            ctx.progress.progress(1.0);
            Ok(result)
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_outputs = files
                .into_par_iter()
                .map(|f| run_single(&f))
                .collect::<Result<Vec<_>, _>>()?;
            let mut flat: Vec<String> = all_outputs.into_iter().flatten().collect();
            if flat.is_empty() {
                return Err(ToolError::Execution("split_lidar batch mode produced no output files".to_string()));
            }
            flat.sort();
            let mut result = build_lidar_result(flat[0].clone());
            result.outputs.insert("output_count".to_string(), json!(flat.len()));
            ctx.progress.progress(1.0);
            Ok(result)
        }
    }
}

impl Tool for LidarRemoveOutliersTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_remove_outliers",
            display_name: "LiDAR Remove Outliers",
            summary: "Filters or classifies outlier points based on local elevation residuals.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighborhood radius for local residual calculation (default 2.0).", required: false },
                ToolParamSpec { name: "elev_diff", description: "Absolute elevation residual threshold for outlier detection (default 50.0).", required: false },
                ToolParamSpec { name: "use_median", description: "Use median instead of mean neighborhood elevation.", required: false },
                ToolParamSpec { name: "classify", description: "If true, classify outliers as class 7/18 instead of removing them.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let elev_diff = parse_f64_alias(args, &["elev_diff", "threshold"], 50.0);
        if !elev_diff.is_finite() || elev_diff < 0.0 {
            return Err(ToolError::Validation("elev_diff/threshold must be a non-negative finite value".to_string()));
        }
        let _ = parse_bool_alias(args, &["use_median"], false);
        let _ = parse_bool_alias(args, &["classify"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        let elev_diff = parse_f64_alias(args, &["elev_diff", "threshold"], 50.0);
        let use_median = parse_bool_alias(args, &["use_median"], false);
        let classify = parse_bool_alias(args, &["classify"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "lidar_remove_outliers");
            }

            let mut tree: KdTree<f64, usize, [f64; 2]> = KdTree::new(2);
            for (i, p) in cloud.points.iter().enumerate() {
                if !point_is_noise(p) && !is_withheld(p) {
                    let _ = tree.add([p.x, p.y], i);
                }
            }

            let radius_sq = search_radius * search_radius;
            let mut residuals = vec![0.0_f64; cloud.points.len()];
            for (i, p) in cloud.points.iter().enumerate() {
                let neigh = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                let mut z_vals = Vec::with_capacity(neigh.len());
                for (_, idx_ref) in neigh {
                    let idx = *idx_ref;
                    if idx != i {
                        z_vals.push(cloud.points[idx].z);
                    }
                }
                let baseline = if z_vals.is_empty() {
                    p.z
                } else if use_median {
                    z_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                    let n = z_vals.len();
                    if n % 2 == 1 {
                        z_vals[n / 2]
                    } else {
                        (z_vals[n / 2 - 1] + z_vals[n / 2]) / 2.0
                    }
                } else {
                    z_vals.iter().sum::<f64>() / z_vals.len() as f64
                };
                residuals[i] = p.z - baseline;
            }

            let points: Vec<PointRecord> = if !classify {
                cloud
                    .points
                    .iter()
                    .enumerate()
                    .filter(|(i, p)| residuals[*i].abs() < elev_diff && !point_is_noise(p))
                    .map(|(_, p)| *p)
                    .collect()
            } else {
                cloud
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut q = *p;
                        if residuals[i] < -elev_diff {
                            q.classification = 7;
                        } else if residuals[i] > elev_diff {
                            q.classification = 18;
                        }
                        q
                    })
                    .collect()
            };

            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            let suffix = if classify {
                "lidar_remove_outliers_classified"
            } else {
                "lidar_remove_outliers"
            };
            store_or_write_lidar_output(&out_cloud, out_path, suffix)
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let suffix = if classify { "outliers_classified" } else { "outliers_removed" };
                    let out = generate_batch_lidar_output_path(&input, suffix);
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for NormalizeLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "normalize_lidar",
            display_name: "Normalize LiDAR",
            summary: "Normalizes LiDAR z-values using a raster DTM so elevations become height above ground.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "dtm", description: "Input DTM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "no_negatives", description: "Clamp negative normalized heights to zero.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let _ = parse_required_raster_path_alias(args, &["dtm", "input_dtm"], "dtm/input_dtm")?;
        let _ = parse_bool_alias(args, &["no_negatives"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let dtm_path = parse_required_raster_path_alias(args, &["dtm", "input_dtm"], "dtm/input_dtm")?;
        let no_negatives = parse_bool_alias(args, &["no_negatives"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        ctx.progress.info("reading DTM raster");
        let dtm = Raster::read(Path::new(&dtm_path))
            .map_err(|e| ToolError::Execution(format!("failed reading dtm raster '{}': {e}", dtm_path)))?;

        let mut points = cloud.points.clone();
        for p in &mut points {
            if point_is_withheld(p) || point_is_noise(p) {
                continue;
            }
            if let Some(ground_z) = sample_dtm_elevation(&dtm, p.x, p.y) {
                let mut z = p.z - ground_z;
                if no_negatives && z < 0.0 {
                    z = 0.0;
                }
                p.z = z;
            } else {
                p.z = 0.0;
            }
        }

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "normalize_lidar")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for HeightAboveGroundTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "height_above_ground",
            display_name: "Height Above Ground",
            summary: "Converts LiDAR elevations to heights above the nearest ground-classified point.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;

        let mut tree: KdTree<f64, f64, [f64; 2]> = KdTree::new(2);
        for p in &cloud.points {
            if !point_is_withheld(p) && p.classification == 2 {
                let _ = tree.add([p.x, p.y], p.z);
            }
        }
        if tree.size() == 0 {
            return Err(ToolError::Execution(
                "none of the input points are ground-classified (class 2)".to_string(),
            ));
        }

        let mut points = cloud.points.clone();
        for p in &mut points {
            if p.classification == 2 {
                p.z = 0.0;
                continue;
            }
            let nearest = tree
                .nearest(&[p.x, p.y], 1, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("failed nearest-ground lookup: {e}")))?;
            if let Some((_, ground_z_ref)) = nearest.first() {
                p.z -= *ground_z_ref;
            } else {
                p.z = 0.0;
            }
        }

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "height_above_ground")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarGroundPointFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_ground_point_filter",
            display_name: "LiDAR Ground-Point Filter",
            summary: "Slope-based filtering/classification of off-terrain points in LiDAR data.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighborhood search radius (default 2.0).", required: false },
                ToolParamSpec { name: "min_neighbours", description: "Minimum neighbors target (default 0).", required: false },
                ToolParamSpec { name: "slope_threshold", description: "Slope threshold in degrees (default 45.0, max 88).", required: false },
                ToolParamSpec { name: "height_threshold", description: "Minimum vertical separation to flag off-terrain (default 1.0).", required: false },
                ToolParamSpec { name: "classify", description: "If true classify points (ground=2, off-terrain=1); else filter off-terrain points.", required: false },
                ToolParamSpec { name: "height_above_ground", description: "If true, output z values as local height above nearest lower neighbor.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path for single-input mode.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let min_neighbours = parse_f64_alias(args, &["min_neighbours"], 0.0);
        if !min_neighbours.is_finite() || min_neighbours < 0.0 {
            return Err(ToolError::Validation("min_neighbours must be a non-negative value".to_string()));
        }
        let slope_threshold = parse_f64_alias(args, &["slope_threshold"], 45.0);
        if !slope_threshold.is_finite() || slope_threshold < 0.0 {
            return Err(ToolError::Validation("slope_threshold must be a non-negative finite value".to_string()));
        }
        let height_threshold = parse_f64_alias(args, &["height_threshold"], 1.0);
        if !height_threshold.is_finite() || height_threshold < 0.0 {
            return Err(ToolError::Validation("height_threshold must be a non-negative finite value".to_string()));
        }
        let _ = parse_bool_alias(args, &["classify"], false);
        let _ = parse_bool_alias(args, &["height_above_ground"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        let min_neighbours = parse_f64_alias(args, &["min_neighbours"], 0.0).max(0.0) as usize;
        let slope_degrees = parse_f64_alias(args, &["slope_threshold"], 45.0).clamp(0.0, 88.0);
        let slope_threshold = slope_degrees.to_radians().tan();
        let height_threshold = parse_f64_alias(args, &["height_threshold"], 1.0);
        let classify = parse_bool_alias(args, &["classify"], false);
        let height_above_ground = parse_bool_alias(args, &["height_above_ground"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "lidar_ground_point_filter");
            }

            let mut tree: KdTree<f64, usize, [f64; 2]> = KdTree::new(2);
            for (idx, p) in cloud.points.iter().enumerate() {
                if !point_is_noise(p) && !point_is_withheld(p) && is_late_return(p) {
                    let _ = tree.add([p.x, p.y], idx);
                }
            }

            let radius_sq = search_radius * search_radius;
            let mut is_off_terrain = vec![false; cloud.points.len()];
            let mut hag = vec![0.0_f64; cloud.points.len()];

            for (i, p) in cloud.points.iter().enumerate() {
                if point_is_withheld(p) || point_is_noise(p) {
                    continue;
                }
                let mut neigh = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                if neigh.len() < min_neighbours + 1 && min_neighbours > 0 {
                    neigh = tree
                        .nearest(&[p.x, p.y], min_neighbours + 1, &squared_euclidean)
                        .unwrap_or_default();
                }

                let mut local_min = p.z;
                let mut off_terrain = false;
                for (dist_sq, idx_ref) in neigh {
                    let n_idx = *idx_ref;
                    if n_idx == i {
                        continue;
                    }
                    let n = cloud.points[n_idx];
                    if point_is_withheld(&n) || point_is_noise(&n) {
                        continue;
                    }
                    if n.z < local_min {
                        local_min = n.z;
                    }
                    if n.z + height_threshold < p.z {
                        let dist = dist_sq.sqrt();
                        if dist > 0.0 {
                            let slope = (p.z - n.z) / dist;
                            if slope > slope_threshold {
                                off_terrain = true;
                                break;
                            }
                        }
                    }
                }
                is_off_terrain[i] = off_terrain;
                hag[i] = (p.z - local_min).max(0.0);
            }

            let points: Vec<PointRecord> = if classify {
                cloud
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut q = *p;
                        q.classification = if is_off_terrain[i] { 1 } else { 2 };
                        if height_above_ground {
                            q.z = hag[i];
                        }
                        q
                    })
                    .collect()
            } else {
                cloud
                    .points
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| {
                        if is_off_terrain[i] {
                            None
                        } else {
                            let mut q = *p;
                            if height_above_ground {
                                q.z = hag[i];
                            }
                            Some(q)
                        }
                    })
                    .collect()
            };

            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            let suffix = if classify { "lidar_ground_point_filter_classified" } else { "lidar_ground_point_filter" };
            store_or_write_lidar_output(&out_cloud, out_path, suffix)
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "ground_filtered");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for FilterLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar",
            display_name: "Filter LiDAR",
            summary: "Filters LiDAR points using a boolean expression over point attributes.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "statement", description: "Boolean expression, e.g. '!is_noise && class == 2'.", required: true },
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path for single-input mode.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let statement = args
            .get("statement")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| ToolError::Validation("statement is required".to_string()))?;
        if statement.is_empty() {
            return Err(ToolError::Validation("statement must be non-empty".to_string()));
        }
        let _ = build_operator_tree::<DefaultNumericTypes>(statement)
            .map_err(|e| ToolError::Validation(format!("invalid statement expression: {e}")))?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let statement = args
            .get("statement")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| ToolError::Validation("statement is required".to_string()))?
            .to_string();
        let tree = build_operator_tree::<DefaultNumericTypes>(&statement)
            .map_err(|e| ToolError::Validation(format!("invalid statement expression: {e}")))?;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar");
            }

            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let min_z = cloud.points.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
            let max_z = cloud.points.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);

            let mut points = Vec::with_capacity(cloud.points.len());
            for (i, p) in cloud.points.iter().enumerate() {
                let ctx = build_filter_context(
                    p,
                    i,
                    cloud.points.len(),
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_z,
                    max_z,
                )?;
                let keep = tree
                    .eval_boolean_with_context(&ctx)
                    .map_err(|e| ToolError::Execution(format!("statement evaluation failed for point {}: {e}", i + 1)))?;
                if keep {
                    points.push(*p);
                }
            }

            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "filter_lidar")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "filtered");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for ModifyLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "modify_lidar",
            display_name: "Modify LiDAR",
            summary: "Applies assignment expressions to modify LiDAR point attributes.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "statement", description: "One or more assignment expressions separated by semicolons, e.g. 'z = z + 1; class = if(z > 5, 2, class)'.", required: true },
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path for single-input mode.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let statement = args
            .get("statement")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("statement is required".to_string()))?;
        let statements = parse_modify_statements(statement)?;

        let mut found_assignment = false;
        for s in &statements {
            if let Some(lhs) = parse_assignment_lhs(s) {
                if is_supported_modify_target(&lhs) {
                    found_assignment = true;
                }
            }
            let _ = build_operator_tree::<DefaultNumericTypes>(s)
                .map_err(|e| ToolError::Validation(format!("invalid statement expression '{}': {e}", s)))?;
        }
        if !found_assignment {
            return Err(ToolError::Validation(
                "statement must contain at least one assignment to a recognized modifiable variable"
                    .to_string(),
            ));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let statement_raw = args
            .get("statement")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("statement is required".to_string()))?;
        let statements = parse_modify_statements(statement_raw)?;
        let trees = statements
            .iter()
            .map(|s| build_operator_tree::<DefaultNumericTypes>(s).map_err(|e| ToolError::Validation(format!("invalid statement expression '{}': {e}", s))))
            .collect::<Result<Vec<_>, _>>()?;
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!(
                    "failed reading input lidar '{}': {e}",
                    in_path.to_string_lossy()
                ))
            })?;

            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "modify_lidar");
            }

            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let min_z = cloud.points.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
            let max_z = cloud.points.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);

            let mut out_points = Vec::with_capacity(cloud.points.len());
            for (i, p) in cloud.points.iter().enumerate() {
                let mut point = *p;
                let mut eval_ctx = build_filter_context(
                    p,
                    i,
                    cloud.points.len(),
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_z,
                    max_z,
                )?;

                // Add tuple aliases for statement compatibility.
                let _ = eval_ctx.set_value(
                    "xy".to_string(),
                    EvalValue::Tuple(vec![EvalValue::from_float(point.x), EvalValue::from_float(point.y)]),
                );
                let _ = eval_ctx.set_value(
                    "xyz".to_string(),
                    EvalValue::Tuple(vec![
                        EvalValue::from_float(point.x),
                        EvalValue::from_float(point.y),
                        EvalValue::from_float(point.z),
                    ]),
                );
                let (r, g, b) = point
                    .color
                    .map(|c| (c.red as i64, c.green as i64, c.blue as i64))
                    .unwrap_or((0, 0, 0));
                let _ = eval_ctx.set_value(
                    "rgb".to_string(),
                    EvalValue::Tuple(vec![
                        EvalValue::from_int(r),
                        EvalValue::from_int(g),
                        EvalValue::from_int(b),
                    ]),
                );

                for tree in &trees {
                    let _ = tree.eval_with_context_mut(&mut eval_ctx).map_err(|e| {
                        ToolError::Execution(format!(
                            "statement evaluation failed for point {}: {e}",
                            i + 1
                        ))
                    })?;
                }

                if let Some((x, y)) = value_as_pair_f64(&eval_ctx, "xy") {
                    point.x = x;
                    point.y = y;
                }
                if let Some((x, y, z)) = value_as_triplet_f64(&eval_ctx, "xyz") {
                    point.x = x;
                    point.y = y;
                    point.z = z;
                }
                if let Some(v) = value_as_f64(&eval_ctx, "x") {
                    point.x = v;
                }
                if let Some(v) = value_as_f64(&eval_ctx, "y") {
                    point.y = v;
                }
                if let Some(v) = value_as_f64(&eval_ctx, "z") {
                    point.z = v;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "intensity") {
                    point.intensity = v.clamp(0, i64::from(u16::MAX)) as u16;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "ret") {
                    point.return_number = v.clamp(0, 15) as u8;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "nret") {
                    point.number_of_returns = v.clamp(0, 15) as u8;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "class") {
                    point.classification = v.clamp(0, 255) as u8;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "user_data") {
                    point.user_data = v.clamp(0, 255) as u8;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "point_source_id") {
                    point.point_source_id = v.clamp(0, i64::from(u16::MAX)) as u16;
                }
                if let Some(v) = value_as_i64(&eval_ctx, "scan_angle") {
                    point.scan_angle = v.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
                }
                if let Some(v) = value_as_f64(&eval_ctx, "time") {
                    point.gps_time = Some(wblidar::GpsTime(v));
                }

                let mut rgb_out = value_as_triplet_f64(&eval_ctx, "rgb").map(|(rr, gg, bb)| {
                    (
                        rr.round().clamp(0.0, f64::from(u16::MAX)) as u16,
                        gg.round().clamp(0.0, f64::from(u16::MAX)) as u16,
                        bb.round().clamp(0.0, f64::from(u16::MAX)) as u16,
                    )
                });
                if rgb_out.is_none() {
                    let r = value_as_i64(&eval_ctx, "red").map(|v| v.clamp(0, i64::from(u16::MAX)) as u16);
                    let g = value_as_i64(&eval_ctx, "green").map(|v| v.clamp(0, i64::from(u16::MAX)) as u16);
                    let b = value_as_i64(&eval_ctx, "blue").map(|v| v.clamp(0, i64::from(u16::MAX)) as u16);
                    if let (Some(rr), Some(gg), Some(bb)) = (r, g, b) {
                        rgb_out = Some((rr, gg, bb));
                    }
                }
                if let Some((rr, gg, bb)) = rgb_out {
                    point.color = Some(Rgb16 {
                        red: rr,
                        green: gg,
                        blue: bb,
                    });
                }

                if let Some(v) = value_as_i64(&eval_ctx, "nir") {
                    point.nir = Some(v.clamp(0, i64::from(u16::MAX)) as u16);
                }

                if let Some(k) = value_as_bool(&eval_ctx, "is_keypoint") {
                    if k {
                        point.flags |= 0b0000_1000;
                    } else {
                        point.flags &= !0b0000_1000;
                    }
                }
                if let Some(w) = value_as_bool(&eval_ctx, "is_withheld") {
                    if w {
                        point.flags |= 0b0000_0100;
                    } else {
                        point.flags &= !0b0000_0100;
                    }
                }
                if let Some(o) = value_as_bool(&eval_ctx, "is_overlap") {
                    if o {
                        point.flags |= 0b0001_0000;
                    } else {
                        point.flags &= !0b0001_0000;
                    }
                }

                out_points.push(point);
            }

            let out_cloud = PointCloud {
                points: out_points,
                crs: cloud.crs.clone(),
            };
            store_or_write_lidar_output(&out_cloud, out_path, "modify_lidar")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "modified");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for FilterLidarByReferenceSurfaceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "filter_lidar_by_reference_surface",
            display_name: "Filter LiDAR By Reference Surface",
            summary: "Extracts or classifies points based on z relation to a reference raster surface.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "ref_surface", description: "Reference raster path or typed raster object.", required: true },
                ToolParamSpec { name: "query", description: "Query type: within, <, <=, >, >= (default within).", required: false },
                ToolParamSpec { name: "threshold", description: "Absolute z-distance threshold used by query=within.", required: false },
                ToolParamSpec { name: "classify", description: "If true classify points; otherwise filter and keep only matches.", required: false },
                ToolParamSpec { name: "true_class_value", description: "Class value assigned to matching points in classify mode.", required: false },
                ToolParamSpec { name: "false_class_value", description: "Class value assigned to non-matching points in classify mode.", required: false },
                ToolParamSpec { name: "preserve_classes", description: "If true preserve non-matching classes in classify mode.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let _ = parse_required_raster_path_alias(args, &["ref_surface", "surface", "input_surface"], "ref_surface/surface/input_surface")?;
        let threshold = parse_f64_alias(args, &["threshold"], 0.0);
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(ToolError::Validation("threshold must be a non-negative finite value".to_string()));
        }
        let _ = parse_bool_alias(args, &["classify"], false);
        let _ = parse_bool_alias(args, &["preserve_classes"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?
            .ok_or_else(|| ToolError::Validation("input is required".to_string()))?;
        let ref_surface_path = parse_required_raster_path_alias(
            args,
            &["ref_surface", "surface", "input_surface"],
            "ref_surface/surface/input_surface",
        )?;
        let query = args.get("query").and_then(Value::as_str).unwrap_or("within");
        let query_type = parse_ref_surface_query_type(query);
        let threshold = parse_f64_alias(args, &["threshold"], 0.0);
        let classify = parse_bool_alias(args, &["classify"], false);
        let true_class_value = parse_f64_alias(args, &["true_class_value"], 2.0).round().clamp(0.0, 255.0) as u8;
        let false_class_value = parse_f64_alias(args, &["false_class_value"], 1.0).round().clamp(0.0, 255.0) as u8;
        let preserve_classes = parse_bool_alias(args, &["preserve_classes"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        ctx.progress.info("reading reference surface");
        let surface = Raster::read(Path::new(&ref_surface_path))
            .map_err(|e| ToolError::Execution(format!("failed reading reference surface '{}': {e}", ref_surface_path)))?;

        let mut matches = vec![false; cloud.points.len()];
        for (i, p) in cloud.points.iter().enumerate() {
            if point_is_withheld(p) || point_is_noise(p) {
                continue;
            }
            let Some(z_ref) = sample_dtm_elevation(&surface, p.x, p.y) else {
                continue;
            };
            matches[i] = match query_type {
                RefSurfaceQueryType::Within => (p.z - z_ref).abs() < threshold,
                RefSurfaceQueryType::Less => p.z < z_ref,
                RefSurfaceQueryType::LessEqual => p.z <= z_ref,
                RefSurfaceQueryType::Greater => p.z > z_ref,
                RefSurfaceQueryType::GreaterEqual => p.z >= z_ref,
            };
        }

        let points: Vec<PointRecord> = if !classify {
            cloud
                .points
                .iter()
                .enumerate()
                .filter(|(i, _)| matches[*i])
                .map(|(_, p)| *p)
                .collect()
        } else {
            cloud
                .points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let mut q = *p;
                    if point_is_noise(p) {
                        return q;
                    }
                    q.classification = if matches[i] {
                        true_class_value
                    } else if preserve_classes {
                        q.classification
                    } else {
                        false_class_value
                    };
                    q
                })
                .collect()
        };

        let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "filter_lidar_by_reference_surface")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for ClassifyLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "classify_lidar",
            display_name: "Classify LiDAR",
            summary: "Performs LiDAR classification into ground, building, and vegetation using neighborhood geometry and segmentation.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighborhood radius (default 2.5).", required: false },
                ToolParamSpec { name: "grd_threshold", description: "Height above local minimum considered ground (default 0.1).", required: false },
                ToolParamSpec { name: "oto_threshold", description: "Height above local minimum considered off-terrain object (default 1.0).", required: false },
                ToolParamSpec { name: "linearity_threshold", description: "Linearity threshold used in final unclassified screening (default 0.5).", required: false },
                ToolParamSpec { name: "planarity_threshold", description: "Planarity threshold used in segmentation and refinement (default 0.85).", required: false },
                ToolParamSpec { name: "num_iter", description: "Number of local RANSAC iterations for planarity/linearity estimates (default 30).", required: false },
                ToolParamSpec { name: "facade_threshold", description: "Neighbor distance used when labeling building facades (default 0.5).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let grd_threshold = parse_f64_alias(args, &["grd_threshold"], 0.1);
        let oto_threshold = parse_f64_alias(args, &["oto_threshold"], 1.0);
        if !grd_threshold.is_finite() || !oto_threshold.is_finite() || grd_threshold < 0.0 || oto_threshold < 0.0 {
            return Err(ToolError::Validation("grd_threshold and oto_threshold must be non-negative finite values".to_string()));
        }
        if grd_threshold > oto_threshold {
            return Err(ToolError::Validation("grd_threshold must be <= oto_threshold".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.5);
        let grd_threshold = parse_f64_alias(args, &["grd_threshold"], 0.1);
        let oto_threshold = parse_f64_alias(args, &["oto_threshold"], 1.0);
        let linearity_threshold = parse_f64_alias(args, &["linearity_threshold"], 0.5).clamp(0.0, 1.0);
        let planarity_threshold = parse_f64_alias(args, &["planarity_threshold"], 0.85).clamp(0.0, 1.0);
        let num_iter = parse_f64_alias(args, &["num_iter"], 30.0).max(1.0) as usize;
        let facade_threshold = parse_f64_alias(args, &["facade_threshold"], 0.5).max(0.0);
        let output_path = parse_optional_lidar_output_path(args)?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;
            if cloud.points.is_empty() {
                let out_cloud = PointCloud { points: vec![], crs: cloud.crs.clone() };
                return store_or_write_lidar_output(&out_cloud, out_path, "classify_lidar");
            }

            let mut tree: KdTree<f64, usize, [f64; 2]> = KdTree::new(2);
            for (i, p) in cloud.points.iter().enumerate() {
                if !point_is_withheld(p) && !point_is_noise(p) {
                    let _ = tree.add([p.x, p.y], i);
                }
            }
            let radius_sq = search_radius * search_radius;

            let n_points = cloud.points.len();
            let mut active_late = vec![false; n_points];
            for (i, p) in cloud.points.iter().enumerate() {
                active_late[i] = !point_is_withheld(p) && !point_is_noise(p) && is_late_return(p);
            }

            // Stage 1: local geometry (RANSAC-like planarity + linearity around each late-return point).
            let mut planar = vec![0.0_f64; n_points];
            let mut linear = vec![0.0_f64; n_points];
            let mut rng = rand::rng();
            for i in 0..n_points {
                if !active_late[i] {
                    continue;
                }
                let p1 = cloud.points[i];
                let found = tree.within(&[p1.x, p1.y], radius_sq, &squared_euclidean).unwrap_or_default();
                let mut neigh: Vec<usize> = Vec::with_capacity(found.len());
                for (_, idx_ref) in found {
                    let idx = *idx_ref;
                    if idx == i {
                        continue;
                    }
                    if !active_late[idx] {
                        continue;
                    }
                    let p2 = cloud.points[idx];
                    if (p1.z - p2.z).abs() <= search_radius {
                        neigh.push(idx);
                    }
                }
                if neigh.len() <= 5 {
                    continue;
                }

                let n = neigh.len();
                let mut max_planar_pts = 0usize;
                let mut max_linear_pts = 0usize;
                for _ in 0..num_iter {
                    let n1 = rng.random_range(0..n);
                    let mut n2 = rng.random_range(0..n);
                    while n2 == n1 {
                        n2 = rng.random_range(0..n);
                    }
                    let p2 = cloud.points[neigh[n1]];
                    let p3 = cloud.points[neigh[n2]];

                    let a = (p2.y - p1.y) * (p3.z - p1.z) - (p3.y - p1.y) * (p2.z - p1.z);
                    let b = (p2.z - p1.z) * (p3.x - p1.x) - (p3.z - p1.z) * (p2.x - p1.x);
                    let c = (p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y);
                    let d = -(a * p1.x + b * p1.y + c * p1.z);
                    let norm1 = (a * a + b * b + c * c).sqrt().max(1e-12);

                    let p = p2.y - p1.y;
                    let q = p1.x - p2.x;
                    let r = -(p * p1.x + q * p1.y);
                    let norm2 = (p * p + q * q).sqrt().max(1e-12);

                    let mut planar_pts = 0usize;
                    let mut linear_pts = 0usize;
                    for (j, idx) in neigh.iter().enumerate() {
                        if j != n1 && j != n2 {
                            let pt = cloud.points[*idx];
                            let residual_plane = (a * pt.x + b * pt.y + c * pt.z + d).abs() / norm1;
                            if residual_plane < grd_threshold {
                                planar_pts += 1;
                            }
                        }
                        if j != n1 {
                            let pt = cloud.points[*idx];
                            let residual_line = (p * pt.x + q * pt.y + r).abs() / norm2;
                            if residual_line < grd_threshold {
                                linear_pts += 1;
                            }
                        }
                    }
                    max_planar_pts = max_planar_pts.max(planar_pts);
                    max_linear_pts = max_linear_pts.max(linear_pts);
                }

                planar[i] = (max_planar_pts + 2) as f64 / n as f64;
                linear[i] = (max_linear_pts + 1) as f64 / n as f64;
            }

            // Stage 2: white top-hat-like residuals from late-return neighbors (erosion + dilation).
            let mut neighborhood_min = vec![f64::MAX; n_points];
            for i in 0..n_points {
                if !active_late[i] {
                    continue;
                }
                let p = cloud.points[i];
                let found = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                let mut min_z = f64::MAX;
                for (_, idx_ref) in found {
                    let idx = *idx_ref;
                    if active_late[idx] {
                        min_z = min_z.min(cloud.points[idx].z);
                    }
                }
                if min_z.is_finite() {
                    neighborhood_min[i] = min_z;
                }
            }

            let mut residuals = vec![f64::MIN; n_points];
            for i in 0..n_points {
                if !active_late[i] {
                    continue;
                }
                let p = cloud.points[i];
                let found = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                let mut max_z = f64::NEG_INFINITY;
                for (_, idx_ref) in found {
                    let idx = *idx_ref;
                    if active_late[idx] {
                        let zn = neighborhood_min[idx];
                        if zn.is_finite() {
                            max_z = max_z.max(zn);
                        }
                    }
                }
                if max_z.is_finite() {
                    residuals[i] = p.z - max_z;
                } else {
                    residuals[i] = 0.0;
                }
            }

            // Stage 3: cluster planar low-residual surfaces, selecting largest-area as primary ground cluster.
            let mut cluster = vec![0usize; n_points];
            let mut cluster_num = 1usize; // 1 stays the vegetation/default cluster.
            let mut grd_cluster = 0usize;
            let mut largest_cluster_area = 0.0_f64;

            for i in 0..n_points {
                if cluster[i] != 0 {
                    continue;
                }
                if active_late[i] && residuals[i].abs() <= grd_threshold && planar[i] >= planarity_threshold {
                    cluster_num += 1;
                    cluster[i] = cluster_num;
                    let mut min_x = f64::INFINITY;
                    let mut max_x = f64::NEG_INFINITY;
                    let mut min_y = f64::INFINITY;
                    let mut max_y = f64::NEG_INFINITY;
                    let mut stack = vec![i];

                    while let Some(point_num) = stack.pop() {
                        let p = cloud.points[point_num];
                        min_x = min_x.min(p.x);
                        max_x = max_x.max(p.x);
                        min_y = min_y.min(p.y);
                        max_y = max_y.max(p.y);

                        let found = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                        for (_, idx_ref) in found {
                            let idx = *idx_ref;
                            let z_n = cloud.points[idx].z;
                            if active_late[idx]
                                && residuals[idx].abs() <= grd_threshold
                                && cluster[idx] == 0
                                && planar[idx] >= planarity_threshold
                                && (z_n - p.z).abs() <= oto_threshold
                            {
                                cluster[idx] = cluster_num;
                                stack.push(idx);
                            } else if residuals[idx].abs() > grd_threshold && cluster[idx] < 2 {
                                cluster[idx] = 1;
                            }
                        }
                    }

                    let area = (max_x - min_x).max(0.0) * (max_y - min_y).max(0.0);
                    if area > largest_cluster_area {
                        largest_cluster_area = area;
                        grd_cluster = cluster_num;
                    }
                } else {
                    cluster[i] = 1;
                }
            }

            if grd_cluster == 0 {
                // Fallback to largest non-veg cluster, else keep no ground cluster.
                let mut counts: HashMap<usize, usize> = HashMap::new();
                for c in &cluster {
                    if *c > 1 {
                        *counts.entry(*c).or_insert(0) += 1;
                    }
                }
                if let Some((best_cluster, _)) = counts.into_iter().max_by_key(|(_, n)| *n) {
                    grd_cluster = best_cluster;
                }
            }

            // Stage 4: include roof-edge vegetation points into nearby building clusters.
            for point_num in 0..n_points {
                let cluster_val = cluster[point_num];
                if cluster_val > 1 && cluster_val != grd_cluster {
                    let p = cloud.points[point_num];
                    let found = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                    for (_, idx_ref) in found {
                        let idx = *idx_ref;
                        if cluster[idx] == 1 {
                            let z_n = cloud.points[idx].z;
                            if (p.z - z_n).abs() < grd_threshold {
                                cluster[idx] = cluster_val;
                            }
                        }
                    }
                }
            }

            // Build ground-only tree for height-above-ground refinements.
            let mut ground_tree: KdTree<f64, usize, [f64; 2]> = KdTree::new(2);
            if grd_cluster > 1 {
                for (idx, p) in cloud.points.iter().enumerate() {
                    if !point_is_withheld(p) && cluster[idx] == grd_cluster {
                        let _ = ground_tree.add([p.x, p.y], idx);
                    }
                }
            }

            // Stage 5: small above-ground planar clusters near ground get absorbed into ground.
            if grd_cluster > 1 && ground_tree.size() > 0 {
                for point_num in 0..n_points {
                    let cluster_val = cluster[point_num];
                    if cluster_val > 1 && cluster_val != grd_cluster {
                        let p = cloud.points[point_num];
                        let neigh = ground_tree
                            .nearest(&[p.x, p.y], 10, &squared_euclidean)
                            .unwrap_or_default();
                        if neigh.is_empty() {
                            continue;
                        }
                        let mut avg = 0.0;
                        for (_, idx_ref) in &neigh {
                            avg += cloud.points[**idx_ref].z;
                        }
                        avg /= neigh.len() as f64;
                        if p.z - avg < oto_threshold {
                            cluster[point_num] = grd_cluster;
                        }
                    }
                }
            }

            // Stage 6: building-facade capture.
            if facade_threshold > 0.0 {
                let facade_sq = facade_threshold * facade_threshold;
                let mut changes: Vec<(usize, usize)> = Vec::new();
                for point_num in 0..n_points {
                    if !point_is_withheld(&cloud.points[point_num])
                        && cluster[point_num] > 1
                        && cluster[point_num] != grd_cluster
                    {
                        let p = cloud.points[point_num];
                        let found = tree.within(&[p.x, p.y], facade_sq, &squared_euclidean).unwrap_or_default();
                        for (_, idx_ref) in found {
                            let idx = *idx_ref;
                            let p2 = cloud.points[idx];
                            if cluster[idx] == 1 && p.z > p2.z {
                                changes.push((idx, cluster[point_num]));
                            }
                        }
                    }
                }
                for (idx, c) in changes {
                    cluster[idx] = c;
                }
            }

            // Stage 7: compute residuals for non-late points from nearest ground and reassign low-height rooftop clutter.
            if grd_cluster > 1 && ground_tree.size() > 0 {
                for point_num in 0..n_points {
                    if !active_late[point_num] && !point_is_withheld(&cloud.points[point_num]) {
                        let p = cloud.points[point_num];
                        let neigh = ground_tree
                            .nearest(&[p.x, p.y], 10, &squared_euclidean)
                            .unwrap_or_default();
                        if neigh.is_empty() {
                            continue;
                        }
                        let mut avg = 0.0;
                        for (_, idx_ref) in &neigh {
                            avg += cloud.points[**idx_ref].z;
                        }
                        avg /= neigh.len() as f64;
                        residuals[point_num] = p.z - avg;
                    }
                }

                for point_num in 0..n_points {
                    if point_is_withheld(&cloud.points[point_num]) || cluster[point_num] != 1 {
                        continue;
                    }
                    if residuals[point_num] < 5.0 {
                        let p = cloud.points[point_num];
                        let found = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                        let mut building_cluster = 0usize;
                        let mut reassign = true;
                        for (_, idx_ref) in found {
                            let idx = *idx_ref;
                            if cluster[idx] == grd_cluster {
                                reassign = false;
                                break;
                            }
                            if cluster[idx] > 1 && cluster[idx] != grd_cluster {
                                building_cluster = cluster[idx];
                            }
                        }
                        if reassign && building_cluster > 1 {
                            cluster[point_num] = building_cluster;
                        }
                    }
                }
            }

            // Stage 8: low/linear/planar vegetation becomes unclassified.
            cluster_num += 1;
            let unclassed_cluster = cluster_num;
            for point_num in 0..n_points {
                if cluster[point_num] == 1
                    && (residuals[point_num] < 2.0
                        || planar[point_num] >= planarity_threshold
                        || linear[point_num] >= linearity_threshold)
                {
                    cluster[point_num] = unclassed_cluster;
                }
            }

            let mut points = cloud.points.clone();
            for i in 0..n_points {
                if point_is_withheld(&points[i]) {
                    continue;
                }
                points[i].classification = if grd_cluster > 1 && cluster[i] == grd_cluster {
                    2
                } else if grd_cluster <= 1 && residuals[i].abs() <= grd_threshold {
                    2
                } else if cluster[i] == unclassed_cluster {
                    1
                } else if cluster[i] == 1 {
                    5
                } else if cluster[i] > 1 {
                    6
                } else {
                    points[i].classification
                };
            }

            let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
            store_or_write_lidar_output(&out_cloud, out_path, "classify_lidar")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "classified");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for LidarClassifySubsetTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_classify_subset",
            display_name: "LiDAR Classify Subset",
            summary: "Classifies points in a base LiDAR cloud that spatially match points in a subset cloud.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "subset", description: "Subset LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "subset_class_value", description: "Classification value assigned to matched subset points (0-18).", required: true },
                ToolParamSpec { name: "nonsubset_class_value", description: "Optional class for non-matching points (0-18). Use 255 to preserve existing classes.", required: false },
                ToolParamSpec { name: "tolerance", description: "3D nearest-neighbour matching tolerance in map units.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["base", "base_lidar", "input"], "base")?;
        let _ = parse_required_lidar_path_alias(args, &["subset", "subset_lidar"], "subset")?;
        let subset_class = parse_f64_alias(args, &["subset_class_value", "subset_class"], f64::NAN);
        if !subset_class.is_finite() {
            return Err(ToolError::Validation("subset_class_value is required".to_string()));
        }
        let subset_class = subset_class.round() as i64;
        if !(0..=18).contains(&subset_class) {
            return Err(ToolError::Validation("subset_class_value must be in [0, 18]".to_string()));
        }
        let nonsubset_class = parse_f64_alias(args, &["nonsubset_class_value", "nonsubset_class"], 255.0).round() as i64;
        if nonsubset_class != 255 && !(0..=18).contains(&nonsubset_class) {
            return Err(ToolError::Validation("nonsubset_class_value must be in [0, 18] or 255".to_string()));
        }
        let tolerance = parse_f64_alias(args, &["tolerance"], 0.001);
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(ToolError::Validation("tolerance must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_required_lidar_path_alias(args, &["base", "base_lidar", "input"], "base")?;
        let subset_path = parse_required_lidar_path_alias(args, &["subset", "subset_lidar"], "subset")?;
        let subset_class = parse_f64_alias(args, &["subset_class_value", "subset_class"], 2.0).round() as u8;
        let nonsubset_class = parse_f64_alias(args, &["nonsubset_class_value", "nonsubset_class"], 255.0).round() as i64;
        let tolerance = parse_f64_alias(args, &["tolerance"], 0.001);
        let tolerance_sq = tolerance * tolerance;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading base and subset lidar");
        let base = PointCloud::read(Path::new(&base_path))
            .map_err(|e| ToolError::Execution(format!("failed reading base lidar '{}': {e}", base_path)))?;
        let subset = PointCloud::read(Path::new(&subset_path))
            .map_err(|e| ToolError::Execution(format!("failed reading subset lidar '{}': {e}", subset_path)))?;

        let mut tree = KdTree::new(3);
        for (i, p) in subset.points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing subset points: {e}")))?;
        }

        let mut out_points = Vec::with_capacity(base.points.len());
        for mut p in base.points.iter().copied() {
            let is_subset = if subset.points.is_empty() {
                false
            } else {
                tree
                    .nearest(&[p.x, p.y, p.z], 1, &squared_euclidean)
                    .map(|hits| !hits.is_empty() && hits[0].0 <= tolerance_sq)
                    .unwrap_or(false)
            };
            if is_subset {
                p.classification = subset_class;
            } else if nonsubset_class != 255 {
                p.classification = nonsubset_class as u8;
            }
            out_points.push(p);
        }

        let out_cloud = PointCloud {
            points: out_points,
            crs: base.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_classify_subset")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for ClipLidarToPolygonTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "clip_lidar_to_polygon",
            display_name: "Clip LiDAR To Polygon",
            summary: "Retains only LiDAR points that fall within polygon geometry.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "polygons", description: "Input polygon vector path or typed vector object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar"], "input")?;
        let _ = parse_vector_path_arg(args, "polygons")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar"], "input")?;
        let poly_path = parse_vector_path_arg(args, "polygons")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar and polygons");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let polys = read_prepared_polygons(&poly_path)?;

        let points: Vec<PointRecord> = cloud
            .points
            .iter()
            .copied()
            .filter(|p| point_in_any_prepared_polygon(p.x, p.y, &polys))
            .collect();

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "clip_lidar_to_polygon")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for ErasePolygonFromLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "erase_polygon_from_lidar",
            display_name: "Erase Polygon From LiDAR",
            summary: "Removes LiDAR points that fall within polygon geometry.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "polygons", description: "Input polygon vector path or typed vector object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar"], "input")?;
        let _ = parse_vector_path_arg(args, "polygons")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar"], "input")?;
        let poly_path = parse_vector_path_arg(args, "polygons")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar and polygons");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let polys = read_prepared_polygons(&poly_path)?;

        let points: Vec<PointRecord> = cloud
            .points
            .iter()
            .copied()
            .filter(|p| !point_in_any_prepared_polygon(p.x, p.y, &polys))
            .collect();

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "erase_polygon_from_lidar")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for ClassifyOverlapPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "classify_overlap_points",
            display_name: "Classify Overlap Points",
            summary: "Flags or filters LiDAR points in grid cells containing multiple point source IDs.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "resolution", description: "Grid cell size used for overlap analysis.", required: false },
                ToolParamSpec { name: "overlap_criterion", description: "max scan angle, not min point source id, not min time, or multiple point source IDs.", required: false },
                ToolParamSpec { name: "filter", description: "If true, remove overlap points; otherwise classify overlap points as class 12.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let resolution = parse_f64_alias(args, &["resolution", "grid_res"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let resolution = parse_f64_alias(args, &["resolution", "grid_res"], 1.0);
        let overlap_criterion = parse_overlap_criterion(
            args.get("overlap_criterion")
                .or_else(|| args.get("criterion"))
                .and_then(Value::as_str)
                .unwrap_or("max scan angle"),
        );
        let filter = parse_bool_alias(args, &["filter"], false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "classify_overlap_points")?;
            return Ok(build_lidar_result(locator));
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        for p in &cloud.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
        }

        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (idx, p) in cloud.points.iter().enumerate() {
            if point_is_withheld(p) {
                continue;
            }
            let col = ((p.x - min_x) / resolution).floor() as i64;
            let row = ((p.y - min_y) / resolution).floor() as i64;
            cells.entry((row, col)).or_default().push(idx);
        }

        let mut overlapping = vec![false; cloud.points.len()];
        for ids in cells.values() {
            if ids.len() < 2 {
                continue;
            }
            let mut psids = HashSet::new();
            for id in ids {
                psids.insert(cloud.points[*id].point_source_id);
            }
            if psids.len() <= 1 {
                continue;
            }

            match overlap_criterion {
                OverlapCriterion::MultiplePointSourceIds => {
                    for id in ids {
                        overlapping[*id] = true;
                    }
                }
                OverlapCriterion::NotMinPointSourceId => {
                    let min_psid = ids.iter().map(|i| cloud.points[*i].point_source_id).min().unwrap_or(0);
                    for id in ids {
                        if cloud.points[*id].point_source_id != min_psid {
                            overlapping[*id] = true;
                        }
                    }
                }
                OverlapCriterion::NotMinTime => {
                    let mut min_time = f64::INFINITY;
                    let mut min_time_psid = cloud.points[ids[0]].point_source_id;
                    for id in ids {
                        let t = cloud.points[*id].gps_time.map(|v| v.0).unwrap_or(f64::INFINITY);
                        if t < min_time {
                            min_time = t;
                            min_time_psid = cloud.points[*id].point_source_id;
                        }
                    }
                    for id in ids {
                        if cloud.points[*id].point_source_id != min_time_psid {
                            overlapping[*id] = true;
                        }
                    }
                }
                OverlapCriterion::MaxScanAngle => {
                    let mut max_abs_angle = -1i32;
                    let mut psid = cloud.points[ids[0]].point_source_id;
                    for id in ids {
                        let a = i32::from(cloud.points[*id].scan_angle).abs();
                        if a > max_abs_angle {
                            max_abs_angle = a;
                            psid = cloud.points[*id].point_source_id;
                        }
                    }
                    for id in ids {
                        if cloud.points[*id].point_source_id == psid {
                            overlapping[*id] = true;
                        }
                    }
                }
            }
        }

        let points = if filter {
            cloud
                .points
                .iter()
                .enumerate()
                .filter_map(|(i, p)| if overlapping[i] { None } else { Some(*p) })
                .collect()
        } else {
            let mut pts = cloud.points.clone();
            for (i, p) in pts.iter_mut().enumerate() {
                if overlapping[i] {
                    p.classification = 12;
                }
            }
            pts
        };

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "classify_overlap_points")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarSegmentationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_segmentation",
            display_name: "LiDAR Segmentation",
            summary: "Segments a LiDAR cloud into connected components and assigns segment colours.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for region growing.", required: false },
                ToolParamSpec { name: "num_iterations", description: "Compatibility parameter for legacy RANSAC iterations.", required: false },
                ToolParamSpec { name: "num_samples", description: "Compatibility parameter for legacy model sampling.", required: false },
                ToolParamSpec { name: "inlier_threshold", description: "Compatibility parameter for legacy planar inlier threshold.", required: false },
                ToolParamSpec { name: "acceptable_model_size", description: "Compatibility parameter for legacy planar model size.", required: false },
                ToolParamSpec { name: "max_planar_slope", description: "Compatibility parameter for legacy planar slope filtering.", required: false },
                ToolParamSpec { name: "norm_diff_threshold", description: "Compatibility parameter for normal-angle thresholding.", required: false },
                ToolParamSpec { name: "max_z_diff", description: "Maximum Z difference allowed while growing a segment.", required: false },
                ToolParamSpec { name: "classes", description: "If true, do not cross class boundaries while growing segments.", required: false },
                ToolParamSpec { name: "ground", description: "If true, assigns class=2 to the largest segment and class=1 to other segmented points.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius must be a positive finite value".to_string()));
        }
        let max_z_diff = parse_f64_alias(args, &["max_z_diff", "maxzdiff"], 1.0);
        if !max_z_diff.is_finite() || max_z_diff < 0.0 {
            return Err(ToolError::Validation("max_z_diff must be a finite non-negative value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        let _norm_diff_threshold = parse_f64_alias(args, &["norm_diff_threshold", "norm_diff"], 2.0);
        let max_z_diff = parse_f64_alias(args, &["max_z_diff", "maxzdiff"], 1.0);
        let class_boundaries = parse_bool_alias(args, &["classes"], false);
        let assign_ground_classes = parse_bool_alias(args, &["ground"], false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;

        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "lidar_segmentation")?;
            return Ok(build_lidar_result(locator));
        }

        let mut tree = KdTree::new(2);
        let mut active_indices = Vec::new();
        for (i, p) in cloud.points.iter().enumerate() {
            if point_is_withheld(p) || point_is_noise(p) {
                continue;
            }
            tree.add([p.x, p.y], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
            active_indices.push(i);
        }

        let mut segment_id = vec![0usize; cloud.points.len()];
        let mut current_segment = 0usize;
        let radius_sq = search_radius * search_radius;

        for seed in active_indices {
            if segment_id[seed] != 0 {
                continue;
            }
            current_segment += 1;
            segment_id[seed] = current_segment;
            let mut stack = vec![seed];

            while let Some(idx) = stack.pop() {
                let p = cloud.points[idx];
                let neighbours = tree
                    .within(&[p.x, p.y], radius_sq, &squared_euclidean)
                    .unwrap_or_default();
                for (_, nref) in neighbours {
                    let nidx = *nref;
                    if segment_id[nidx] != 0 {
                        continue;
                    }
                    let pn = cloud.points[nidx];
                    if class_boundaries && pn.classification != p.classification {
                        continue;
                    }
                    if (pn.z - p.z).abs() <= max_z_diff {
                        segment_id[nidx] = current_segment;
                        stack.push(nidx);
                    }
                }
            }
        }

        let mut counts: HashMap<usize, usize> = HashMap::new();
        for sid in &segment_id {
            if *sid > 0 {
                *counts.entry(*sid).or_insert(0) += 1;
            }
        }
        let largest_segment = counts
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(sid, _)| *sid)
            .unwrap_or(0);

        let mut points = cloud.points.clone();
        for (i, p) in points.iter_mut().enumerate() {
            let sid = segment_id[i];
            if sid == 0 {
                continue;
            }

            let (r8, g8, b8) = if sid == largest_segment {
                (25u8, 120u8, 0u8)
            } else {
                let r = ((sid.wrapping_mul(73)) % 256) as u8;
                let g = ((sid.wrapping_mul(151)) % 256) as u8;
                let b = ((sid.wrapping_mul(199)) % 256) as u8;
                (r, g, b)
            };
            p.color = Some(wblidar::Rgb16 {
                red: u16::from(r8) * 257,
                green: u16::from(g8) * 257,
                blue: u16::from(b8) * 257,
            });

            if assign_ground_classes {
                p.classification = if sid == largest_segment { 2 } else { 1 };
            }
        }

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_segmentation")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for IndividualTreeSegmentationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "individual_tree_segmentation",
            display_name: "Individual Tree Segmentation",
            summary: "Segments vegetation LiDAR points into individual tree clusters using a mean-shift mode-seeking workflow.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "only_use_veg", description: "If true, process only vegetation classes (default true).", required: false },
                ToolParamSpec { name: "veg_classes", description: "Vegetation classes as comma-delimited text or integer array (default '3,4,5').", required: false },
                ToolParamSpec { name: "min_height", description: "Minimum point height for segmentation (default 2.0).", required: false },
                ToolParamSpec { name: "max_height", description: "Optional maximum point height.", required: false },
                ToolParamSpec { name: "bandwidth_min", description: "Minimum horizontal bandwidth (default 1.0).", required: false },
                ToolParamSpec { name: "bandwidth_max", description: "Maximum horizontal bandwidth (default 6.0).", required: false },
                ToolParamSpec { name: "adaptive_bandwidth", description: "Estimate per-seed horizontal bandwidth from local crown geometry (default true).", required: false },
                ToolParamSpec { name: "adaptive_neighbors", description: "Neighbour count used for adaptive local density scale (default 24).", required: false },
                ToolParamSpec { name: "adaptive_sector_count", description: "Number of angular sectors for local crown-radius estimation (default 8).", required: false },
                ToolParamSpec { name: "grid_acceleration", description: "Use MeanShift++-style grid approximation for faster mode updates (default false).", required: false },
                ToolParamSpec { name: "grid_cell_size", description: "Grid cell size for accelerated mode updates (default 0.5).", required: false },
                ToolParamSpec { name: "grid_refine_exact", description: "Run short exact-neighbour refinement after grid acceleration (default false).", required: false },
                ToolParamSpec { name: "grid_refine_iterations", description: "Exact refinement iteration cap after grid mode updates (default 2).", required: false },
                ToolParamSpec { name: "tile_size", description: "Optional tile size for seed scheduling; <=0 disables tiling (default 0.0).", required: false },
                ToolParamSpec { name: "tile_overlap", description: "Tile overlap width for tiled seed scheduling (default 0.0).", required: false },
                ToolParamSpec { name: "vertical_bandwidth", description: "Vertical kernel bandwidth (default 5.0).", required: false },
                ToolParamSpec { name: "max_iterations", description: "Maximum mean-shift iterations per seed (default 30).", required: false },
                ToolParamSpec { name: "convergence_tol", description: "Convergence tolerance for shift magnitude (default 0.05).", required: false },
                ToolParamSpec { name: "min_cluster_points", description: "Minimum points per retained tree cluster (default 50).", required: false },
                ToolParamSpec { name: "mode_merge_dist", description: "Distance threshold for merging converged modes (default 0.8).", required: false },
                ToolParamSpec { name: "threads", description: "Thread count override (0 uses default Rayon pool).", required: false },
                ToolParamSpec { name: "simd", description: "Enable SIMD-assisted arithmetic in weighting loops (default true).", required: false },
                ToolParamSpec { name: "output_id_mode", description: "Output segment id encoding: rgb/user_data/point_source_id or combinations like rgb+user_data.", required: false },
                ToolParamSpec { name: "output_sidecar_csv", description: "If true, write point_index,segment_id CSV beside lidar output.", required: false },
                ToolParamSpec { name: "seed", description: "Deterministic seed for colour mapping (default 1).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let _ = parse_include_classes_arg(args, &["veg_classes"], &[3, 4, 5])?;

        let min_height = parse_f64_alias(args, &["min_height"], 2.0);
        if !min_height.is_finite() {
            return Err(ToolError::Validation("min_height must be finite".to_string()));
        }
        let max_height = args.get("max_height").and_then(Value::as_f64);
        if let Some(v) = max_height {
            if !v.is_finite() || v <= min_height {
                return Err(ToolError::Validation(
                    "max_height must be finite and greater than min_height".to_string(),
                ));
            }
        }

        let h_min = parse_f64_alias(args, &["bandwidth_min"], 1.0);
        let h_max = parse_f64_alias(args, &["bandwidth_max"], 6.0);
        if !h_min.is_finite() || !h_max.is_finite() || h_min <= 0.0 || h_max < h_min {
            return Err(ToolError::Validation(
                "bandwidth_min and bandwidth_max must be finite with 0 < bandwidth_min <= bandwidth_max"
                    .to_string(),
            ));
        }

        let adaptive_neighbors = parse_usize_alias(args, &["adaptive_neighbors"], 24);
        if adaptive_neighbors == 0 {
            return Err(ToolError::Validation(
                "adaptive_neighbors must be greater than 0".to_string(),
            ));
        }
        let adaptive_sector_count = parse_usize_alias(args, &["adaptive_sector_count"], 8);
        if adaptive_sector_count == 0 || adaptive_sector_count > 64 {
            return Err(ToolError::Validation(
                "adaptive_sector_count must be in the range [1, 64]".to_string(),
            ));
        }

        let grid_cell_size = parse_f64_alias(args, &["grid_cell_size"], 0.5);
        if !grid_cell_size.is_finite() || grid_cell_size <= 0.0 {
            return Err(ToolError::Validation(
                "grid_cell_size must be a positive finite value".to_string(),
            ));
        }
        let grid_refine_iterations = parse_usize_alias(args, &["grid_refine_iterations"], 2);
        if grid_refine_iterations == 0 {
            return Err(ToolError::Validation(
                "grid_refine_iterations must be greater than 0".to_string(),
            ));
        }
        let tile_size = parse_f64_alias(args, &["tile_size"], 0.0);
        if !tile_size.is_finite() {
            return Err(ToolError::Validation("tile_size must be finite".to_string()));
        }
        let tile_overlap = parse_f64_alias(args, &["tile_overlap"], 0.0);
        if !tile_overlap.is_finite() || tile_overlap < 0.0 {
            return Err(ToolError::Validation(
                "tile_overlap must be a finite non-negative value".to_string(),
            ));
        }
        if tile_size > 0.0 && tile_overlap >= tile_size {
            return Err(ToolError::Validation(
                "tile_overlap must be less than tile_size when tiling is enabled".to_string(),
            ));
        }

        let h_z = parse_f64_alias(args, &["vertical_bandwidth"], 5.0);
        if !h_z.is_finite() || h_z <= 0.0 {
            return Err(ToolError::Validation(
                "vertical_bandwidth must be a positive finite value".to_string(),
            ));
        }

        let max_iterations = parse_usize_alias(args, &["max_iterations"], 30);
        if max_iterations == 0 {
            return Err(ToolError::Validation("max_iterations must be greater than 0".to_string()));
        }

        let tol = parse_f64_alias(args, &["convergence_tol"], 0.05);
        if !tol.is_finite() || tol <= 0.0 {
            return Err(ToolError::Validation(
                "convergence_tol must be a positive finite value".to_string(),
            ));
        }

        let merge_dist = parse_f64_alias(args, &["mode_merge_dist"], 0.8);
        if !merge_dist.is_finite() || merge_dist < 0.0 {
            return Err(ToolError::Validation(
                "mode_merge_dist must be a non-negative finite value".to_string(),
            ));
        }

        let _ = parse_optional_output_path(args, "output")?;
        let sidecar = parse_bool_alias(args, &["output_sidecar_csv"], false);
        if sidecar && args.get("output").is_none() {
            return Err(ToolError::Validation(
                "output_sidecar_csv=true requires an explicit 'output' path".to_string(),
            ));
        }

        let mode = parse_string_alias(args, &["output_id_mode"], "rgb").to_ascii_lowercase();
        let allowed = ["rgb", "user_data", "point_source_id", "rgb+user_data", "rgb+point_source_id"];
        if !allowed.contains(&mode.as_str()) {
            return Err(ToolError::Validation(format!(
                "unsupported output_id_mode '{}'; expected one of rgb/user_data/point_source_id/rgb+user_data/rgb+point_source_id",
                mode
            )));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let only_use_veg = parse_bool_alias(args, &["only_use_veg"], true);
        let veg_classes = parse_include_classes_arg(args, &["veg_classes"], &[3, 4, 5])?;
        let min_height = parse_f64_alias(args, &["min_height"], 2.0);
        let max_height = args.get("max_height").and_then(Value::as_f64);
        let bandwidth_min = parse_f64_alias(args, &["bandwidth_min"], 1.0);
        let bandwidth_max = parse_f64_alias(args, &["bandwidth_max"], 6.0);
        let adaptive_bandwidth = parse_bool_alias(args, &["adaptive_bandwidth"], true);
        let adaptive_neighbors = parse_usize_alias(args, &["adaptive_neighbors"], 24);
        let adaptive_sector_count = parse_usize_alias(args, &["adaptive_sector_count"], 8);
        let grid_acceleration = parse_bool_alias(args, &["grid_acceleration"], false);
        let grid_cell_size = parse_f64_alias(args, &["grid_cell_size"], 0.5);
        let grid_refine_exact = parse_bool_alias(args, &["grid_refine_exact"], false);
        let grid_refine_iterations = parse_usize_alias(args, &["grid_refine_iterations"], 2);
        let tile_size = parse_f64_alias(args, &["tile_size"], 0.0);
        let tile_overlap = parse_f64_alias(args, &["tile_overlap"], 0.0);
        let vertical_bandwidth = parse_f64_alias(args, &["vertical_bandwidth"], 5.0);
        let max_iterations = parse_usize_alias(args, &["max_iterations"], 30);
        let convergence_tol = parse_f64_alias(args, &["convergence_tol"], 0.05);
        let min_cluster_points = parse_usize_alias(args, &["min_cluster_points"], 50);
        let mode_merge_dist = parse_f64_alias(args, &["mode_merge_dist"], 0.8);
        let threads = parse_usize_alias(args, &["threads"], 0);
        let simd = parse_bool_alias(args, &["simd"], true);
        let output_id_mode = parse_string_alias(args, &["output_id_mode"], "rgb").to_ascii_lowercase();
        let output_sidecar_csv = parse_bool_alias(args, &["output_sidecar_csv"], false);
        let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);

        let use_rgb = output_id_mode.contains("rgb");
        let use_user_data = output_id_mode.contains("user_data");
        let use_point_source_id = output_id_mode.contains("point_source_id");

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path)).map_err(|e| {
            ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
        })?;
        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "individual_tree_segmentation")?;
            return Ok(build_lidar_result(locator));
        }

        ctx.progress.info("filtering candidate vegetation points");
        let mut eligible_indices: Vec<usize> = Vec::new();
        for (idx, p) in cloud.points.iter().enumerate() {
            if point_is_withheld(p) || point_is_noise(p) {
                continue;
            }
            if only_use_veg && !veg_classes[p.classification as usize] {
                continue;
            }
            if p.z < min_height {
                continue;
            }
            if let Some(max_h) = max_height {
                if p.z > max_h {
                    continue;
                }
            }
            eligible_indices.push(idx);
        }

        if eligible_indices.is_empty() {
            return Err(ToolError::Validation(
                "no eligible points remained after filtering (check class and height filters)".to_string(),
            ));
        }

        let mut xs = Vec::with_capacity(eligible_indices.len());
        let mut ys = Vec::with_capacity(eligible_indices.len());
        let mut zs = Vec::with_capacity(eligible_indices.len());
        for idx in &eligible_indices {
            let p = cloud.points[*idx];
            xs.push(p.x);
            ys.push(p.y);
            zs.push(p.z);
        }

        ctx.progress.info("building spatial index");
        let mut tree: KdTree<f64, usize, [f64; 2]> = KdTree::new(2);
        for i in 0..eligible_indices.len() {
            tree.add([xs[i], ys[i]], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing candidate points: {e}")))?;
        }

        let grid = if grid_acceleration {
            Some(build_meanshift_grid(&xs, &ys, &zs, grid_cell_size))
        } else {
            None
        };

        let bandwidth_range = (bandwidth_max - bandwidth_min).max(0.0);
        let height_range = max_height
            .map(|mh| (mh - min_height).max(f64::EPSILON))
            .unwrap_or(1.0);

        ctx.progress.info("running mean-shift mode seeking");
        let local_indices: Vec<usize> = (0..eligible_indices.len()).collect();
        let hxy_values: Vec<f64> = local_indices
            .par_iter()
            .map(|seed_idx| {
                let z = zs[*seed_idx];
                let fallback_hxy = if let Some(mh) = max_height {
                    if z <= min_height {
                        bandwidth_min
                    } else if z >= mh {
                        bandwidth_max
                    } else {
                        bandwidth_min + ((z - min_height) / height_range) * bandwidth_range
                    }
                } else {
                    bandwidth_min
                }
                .clamp(bandwidth_min, bandwidth_max);

                if adaptive_bandwidth {
                    let adaptive_hxy = estimate_adaptive_bandwidth_for_seed(
                        *seed_idx,
                        &xs,
                        &ys,
                        &zs,
                        &tree,
                        bandwidth_min,
                        bandwidth_max,
                        vertical_bandwidth,
                        adaptive_neighbors,
                        adaptive_sector_count,
                    );
                    (adaptive_hxy * 0.85 + fallback_hxy * 0.15).clamp(bandwidth_min, bandwidth_max)
                } else {
                    fallback_hxy
                }
            })
            .collect();

        let compute_seed_mode = |seed_idx: usize| {
            let hxy = hxy_values[seed_idx];
            if let Some(g) = &grid {
                let coarse = shift_mode_for_seed_grid(
                    seed_idx,
                    &xs,
                    &ys,
                    &zs,
                    g,
                    grid_cell_size,
                    hxy,
                    vertical_bandwidth,
                    max_iterations,
                    convergence_tol,
                );
                if grid_refine_exact {
                    refine_mode_exact_from_start(
                        coarse,
                        &xs,
                        &ys,
                        &zs,
                        &tree,
                        hxy,
                        vertical_bandwidth,
                        grid_refine_iterations,
                        convergence_tol,
                        simd,
                    )
                } else {
                    coarse
                }
            } else {
                shift_mode_for_seed(
                    seed_idx,
                    &xs,
                    &ys,
                    &zs,
                    &tree,
                    hxy,
                    vertical_bandwidth,
                    max_iterations,
                    convergence_tol,
                    simd,
                )
            }
        };

        let compute_modes = || {
            if tile_size > 0.0 {
                let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
                let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
                let tile_stride = (tile_size - tile_overlap).max(tile_size * 0.25);

                let mut tile_bins: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
                for seed_idx in 0..local_indices.len() {
                    let tx = ((xs[seed_idx] - min_x) / tile_stride).floor() as i32;
                    let ty = ((ys[seed_idx] - min_y) / tile_stride).floor() as i32;
                    tile_bins.entry((tx, ty)).or_default().push(seed_idx);
                }

                let tile_groups: Vec<Vec<usize>> = tile_bins.into_values().collect();
                let per_tile_modes: Vec<Vec<(usize, [f64; 3])>> = tile_groups
                    .par_iter()
                    .map(|group| {
                        group
                            .iter()
                            .map(|seed_idx| (*seed_idx, compute_seed_mode(*seed_idx)))
                            .collect::<Vec<(usize, [f64; 3])>>()
                    })
                    .collect();

                let mut modes = vec![[0.0; 3]; local_indices.len()];
                for group in per_tile_modes {
                    for (seed_idx, mode) in group {
                        modes[seed_idx] = mode;
                    }
                }
                modes
            } else {
                local_indices
                    .par_iter()
                    .map(|seed_idx| compute_seed_mode(*seed_idx))
                    .collect::<Vec<[f64; 3]>>()
            }
        };

        let modes = if threads > 0 {
            let pool = ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|e| ToolError::Execution(format!("failed building thread pool: {e}")))?;
            pool.install(compute_modes)
        } else {
            compute_modes()
        };

        ctx.progress.info("merging nearby modes and assigning clusters");
        let merge_dist2 = mode_merge_dist * mode_merge_dist;
        let mut merged: Vec<[f64; 3]> = Vec::new();
        let mut assigned_segment_local: Vec<usize> = vec![0; eligible_indices.len()];
        for (i, mode) in modes.iter().enumerate() {
            let mut sid = 0usize;
            for (j, m) in merged.iter().enumerate() {
                let dx = mode[0] - m[0];
                let dy = mode[1] - m[1];
                if dx * dx + dy * dy <= merge_dist2 {
                    sid = j + 1;
                    break;
                }
            }
            if sid == 0 {
                merged.push(*mode);
                sid = merged.len();
            }
            assigned_segment_local[i] = sid;
        }

        let mut counts = vec![0usize; merged.len() + 1];
        for sid in &assigned_segment_local {
            if *sid > 0 {
                counts[*sid] += 1;
            }
        }

        for sid in &mut assigned_segment_local {
            if *sid > 0 && counts[*sid] < min_cluster_points {
                *sid = 0;
            }
        }

        let retained: Vec<usize> = (1..counts.len())
            .filter(|sid| counts[*sid] >= min_cluster_points)
            .collect();

        if !retained.is_empty() {
            let max_assign_dist2 = bandwidth_max * bandwidth_max;
            for (i, sid) in assigned_segment_local.iter_mut().enumerate() {
                if *sid != 0 {
                    continue;
                }
                let px = xs[i];
                let py = ys[i];
                let mut best = 0usize;
                let mut best_d2 = f64::INFINITY;
                for rsid in &retained {
                    let c = merged[*rsid - 1];
                    let dx = px - c[0];
                    let dy = py - c[1];
                    let d2 = dx * dx + dy * dy;
                    if d2 < best_d2 {
                        best_d2 = d2;
                        best = *rsid;
                    }
                }
                if best > 0 && best_d2 <= max_assign_dist2 {
                    *sid = best;
                }
            }
        }

        let mut remap: HashMap<usize, usize> = HashMap::new();
        let mut next_id = 1usize;
        for sid in &assigned_segment_local {
            if *sid == 0 {
                continue;
            }
            remap.entry(*sid).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
        for sid in &mut assigned_segment_local {
            if *sid > 0 {
                *sid = *remap.get(sid).unwrap_or(&0);
            }
        }

        let mut out_points = cloud.points.clone();
        for (local_idx, orig_idx) in eligible_indices.iter().enumerate() {
            let sid = assigned_segment_local[local_idx];
            if sid == 0 {
                continue;
            }
            let p = &mut out_points[*orig_idx];
            if use_rgb {
                p.color = Some(segment_color_from_id(sid, seed));
            }
            if use_user_data {
                p.user_data = sid.min(255) as u8;
            }
            if use_point_source_id {
                p.point_source_id = sid.min(u16::MAX as usize) as u16;
            }
        }

        let out_cloud = PointCloud {
            points: out_points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path.clone(), "individual_tree_segmentation")?;

        let mut result = build_lidar_result(locator.clone());
        if output_sidecar_csv {
            let lidar_path = PathBuf::from(&locator);
            let csv_path = derive_sidecar_csv_path(&lidar_path);
            let mut writer = BufWriter::new(File::create(&csv_path).map_err(|e| {
                ToolError::Execution(format!("failed creating sidecar csv '{}': {e}", csv_path.to_string_lossy()))
            })?);
            writer
                .write_all(b"point_index,segment_id\n")
                .map_err(|e| ToolError::Execution(format!("failed writing sidecar csv header: {e}")))?;
            for (local_idx, orig_idx) in eligible_indices.iter().enumerate() {
                let sid = assigned_segment_local[local_idx];
                writer
                    .write_all(format!("{},{}\n", orig_idx, sid).as_bytes())
                    .map_err(|e| ToolError::Execution(format!("failed writing sidecar csv row: {e}")))?;
            }
            writer
                .flush()
                .map_err(|e| ToolError::Execution(format!("failed finalizing sidecar csv: {e}")))?;
            result.outputs.insert(
                "sidecar_csv_path".to_string(),
                json!(csv_path.to_string_lossy().to_string()),
            );
        }

        ctx.progress.progress(1.0);
        Ok(result)
    }
}

impl Tool for IndividualTreeDetectionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "individual_tree_detection",
            display_name: "Individual Tree Detection",
            summary: "Identifies tree top points in a LiDAR cloud using local maxima detection.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "min_search_radius", description: "Minimum search radius in map units (default 1.0).", required: false },
                ToolParamSpec { name: "min_height", description: "Minimum height to consider points (default 0.0).", required: false },
                ToolParamSpec { name: "max_search_radius", description: "Maximum search radius; if not set uses min_search_radius.", required: false },
                ToolParamSpec { name: "max_height", description: "Maximum height; if not set uses min_height.", required: false },
                ToolParamSpec { name: "only_use_veg", description: "If true, process only vegetation classes (default true).", required: false },
                ToolParamSpec { name: "output", description: "Output vector shapefile path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        
        let min_search_radius = parse_f64_alias(args, &["min_search_radius"], 1.0);
        if !min_search_radius.is_finite() || min_search_radius <= 0.0 {
            return Err(ToolError::Validation("min_search_radius must be finite and positive".to_string()));
        }
        
        let max_search_radius = parse_f64_alias(args, &["max_search_radius"], min_search_radius);
        if !max_search_radius.is_finite() || max_search_radius <= 0.0 {
            return Err(ToolError::Validation("max_search_radius must be finite and positive".to_string()));
        }
        if max_search_radius < min_search_radius {
            return Err(ToolError::Validation("max_search_radius must be >= min_search_radius".to_string()));
        }

        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_output_path(args, "output")?;

        let min_search_radius = parse_f64_alias(args, &["min_search_radius"], 1.0);
        let min_height = parse_f64_alias(args, &["min_height"], 0.0);
        let max_search_radius = parse_f64_alias(args, &["max_search_radius"], min_search_radius);
        let max_height = parse_f64_alias(args, &["max_height"], min_height);
        let only_use_veg = parse_bool_alias(args, &["only_use_veg"], true);

        let radius_range = max_search_radius - min_search_radius;
        let height_range = max_height - min_height;

        // Read input LiDAR
        ctx.progress.info("Reading input LiDAR");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("Failed to read input LiDAR: {}", e)))?;

        let n_points = cloud.points.len();
        if n_points == 0 {
            return Err(ToolError::Execution("Input LiDAR has no points".to_string()));
        }

        ctx.progress.progress(0.2);

        // Filter eligible points and build KdTree
        ctx.progress.info("Building spatial index and filtering points");
        let mut eligible_pts: Vec<(usize, f64, f64, f64)> = Vec::new();

        for (i, point) in cloud.points.iter().enumerate() {
            // Skip withheld and noise points
            if point_is_withheld(point) || point_is_noise(point) {
                continue;
            }

            // Check vegetation filter
            if only_use_veg && !matches!(point.classification, 3 | 4 | 5) {
                continue;
            }

            if point.z >= min_height {
                eligible_pts.push((i, point.x, point.y, point.z));
            }
        }

        if eligible_pts.is_empty() {
            return Err(ToolError::Execution(
                "No eligible points found. Try setting only_use_veg=false.".to_string()
            ));
        }

        ctx.progress.progress(0.4);

        // Create output layer
        ctx.progress.info("Identifying tree tops");
        let mut layer = wbvector::Layer::new("treetops").with_geom_type(wbvector::GeometryType::Point);
        layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());

        layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("Z", wbvector::FieldType::Float));

        // Find tree tops using brute force neighbor search
        for &(point_idx, x, y, z) in &eligible_pts {
            // Calculate search radius based on height
            let radius = if z > max_height {
                max_search_radius
            } else if height_range > 0.0 {
                min_search_radius + (z - min_height) / height_range * radius_range
            } else {
                min_search_radius
            };

            // Find neighbors within radius and check if this point is the highest
            let mut is_highest = true;
            for &(neighbor_idx, nx, ny, nz) in &eligible_pts {
                if neighbor_idx != point_idx {
                    let dist_sq = (nx - x).powi(2) + (ny - y).powi(2);
                    if dist_sq <= radius.powi(2) && nz > z {
                        is_highest = false;
                        break;
                    }
                }
            }

            if is_highest {
                let point = &cloud.points[point_idx];
                layer
                    .add_feature(
                        Some(wbvector::Geometry::point(point.x, point.y)),
                        &[
                            ("FID", wbvector::FieldValue::Integer((point_idx + 1) as i64)),
                            ("Z", wbvector::FieldValue::Float(z)),
                        ],
                    )
                    .map_err(|e| ToolError::Execution(format!("Failed to add feature: {}", e)))?;
            }

            ctx.progress.progress(0.4 + 0.4 * (point_idx as f64 / eligible_pts.len() as f64));
        }

        ctx.progress.progress(0.8);

        // Write output
        ctx.progress.info("Writing output shapefile");
        let out = output_path
            .map(PathBuf::from)
            .unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "treetops", "shp"));
        let out_path = write_vector_output(&layer, out.to_string_lossy().as_ref())?;

        ctx.progress.progress(1.0);
        Ok(build_vector_result(out_path))
    }
}

impl Tool for LidarSegmentationBasedFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_segmentation_based_filter",
            display_name: "LiDAR Segmentation Based Filter",
            summary: "Ground-point filtering based on neighbourhood-connected low-relief segments.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for connected-component growth.", required: false },
                ToolParamSpec { name: "norm_diff_threshold", description: "Compatibility parameter for legacy normal-angle checks.", required: false },
                ToolParamSpec { name: "max_z_diff", description: "Maximum elevation difference for connected growth.", required: false },
                ToolParamSpec { name: "classify_points", description: "If true, classify points instead of filtering.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 5.0);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius must be a positive finite value".to_string()));
        }
        let max_z_diff = parse_f64_alias(args, &["max_z_diff", "maxzdiff"], 1.0);
        if !max_z_diff.is_finite() || max_z_diff < 0.0 {
            return Err(ToolError::Validation("max_z_diff must be a finite non-negative value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 5.0);
        let _norm_diff_threshold = parse_f64_alias(args, &["norm_diff_threshold", "norm_diff"], 2.0);
        let max_z_diff = parse_f64_alias(args, &["max_z_diff", "maxzdiff"], 1.0);
        let classify_points = parse_bool_alias(args, &["classify_points", "classify"], false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;

        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "lidar_segmentation_based_filter")?;
            return Ok(build_lidar_result(locator));
        }

        let mut tree = KdTree::new(2);
        let mut active = Vec::new();
        let mut min_z = f64::INFINITY;
        for (i, p) in cloud.points.iter().enumerate() {
            if point_is_withheld(p) || point_is_noise(p) {
                continue;
            }
            min_z = min_z.min(p.z);
            tree.add([p.x, p.y], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
            active.push(i);
        }

        let mut is_ground = vec![false; cloud.points.len()];
        let mut stack = Vec::new();
        for idx in active {
            if cloud.points[idx].z <= min_z + max_z_diff {
                is_ground[idx] = true;
                stack.push(idx);
            }
        }

        let radius_sq = search_radius * search_radius;
        while let Some(idx) = stack.pop() {
            let p = cloud.points[idx];
            let neighbours = tree
                .within(&[p.x, p.y], radius_sq, &squared_euclidean)
                .unwrap_or_default();
            for (_, nref) in neighbours {
                let nidx = *nref;
                if is_ground[nidx] {
                    continue;
                }
                let pn = cloud.points[nidx];
                if (pn.z - p.z).abs() <= max_z_diff {
                    is_ground[nidx] = true;
                    stack.push(nidx);
                }
            }
        }

        let points = if classify_points {
            let mut pts = cloud.points.clone();
            for (i, p) in pts.iter_mut().enumerate() {
                if point_is_withheld(p) {
                    continue;
                }
                p.classification = if is_ground[i] { 2 } else { 1 };
            }
            pts
        } else {
            cloud
                .points
                .iter()
                .enumerate()
                .filter_map(|(i, p)| if is_ground[i] { Some(*p) } else { None })
                .collect()
        };

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_segmentation_based_filter")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarColourizeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_colourize",
            display_name: "LiDAR Colourize",
            summary: "Assigns LiDAR point RGB values from an overlapping raster image.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "image", description: "Input image raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let _ = parse_required_raster_path_alias(args, &["image", "input_image", "in_image"], "image")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let image_path = parse_required_raster_path_alias(args, &["image", "input_image", "in_image"], "image")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar and image raster");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let image = Raster::read(Path::new(&image_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input image '{}': {e}", image_path)))?;

        let mut points = cloud.points.clone();
        for p in &mut points {
            let rgb = if let Some((col, row)) = image.world_to_pixel(p.x, p.y) {
                if let Some(v) = image.get_opt(0, row, col) {
                    let value = v as u32;
                    let r = (value & 0xFF) as u8;
                    let g = ((value >> 8) & 0xFF) as u8;
                    let b = ((value >> 16) & 0xFF) as u8;
                    color8_to_rgb16(r, g, b)
                } else {
                    color8_to_rgb16(0, 0, 0)
                }
            } else {
                color8_to_rgb16(0, 0, 0)
            };
            p.color = Some(rgb);
        }

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_colourize")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for ColourizeBasedOnClassTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "colourize_based_on_class",
            display_name: "Colourize Based On Class",
            summary: "Sets LiDAR point RGB values based on point classifications.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "intensity_blending_amount", description: "Percent blend [0,100] between class colour and intensity.", required: false },
                ToolParamSpec { name: "clr_str", description: "Optional class-colour overrides, e.g. '2:(184,167,108);5:#9ab86c'.", required: false },
                ToolParamSpec { name: "use_unique_clrs_for_buildings", description: "If true, assigns unique colours to connected building clusters.", required: false },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius used for building-cluster colouring.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let blend = parse_f64_alias(args, &["intensity_blending_amount", "intensity_blending"], 50.0);
        if !blend.is_finite() || !(0.0..=100.0).contains(&blend) {
            return Err(ToolError::Validation("intensity_blending_amount must be in [0, 100]".to_string()));
        }
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        if !search_radius.is_finite() || search_radius <= 0.0 {
            return Err(ToolError::Validation("search_radius must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let blend = parse_f64_alias(args, &["intensity_blending_amount", "intensity_blending"], 50.0) / 100.0;
        let clr_str = args.get("clr_str").and_then(Value::as_str).unwrap_or("").to_string();
        let unique_buildings = parse_bool_alias(args, &["use_unique_clrs_for_buildings", "unique_building_colours"], false);
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;

            let mut palette = default_class_palette();
            if !clr_str.trim().is_empty() {
                for pair in clr_str.split(';') {
                    let pair = pair.trim();
                    if pair.is_empty() {
                        continue;
                    }
                    let Some((cls_txt, colour_txt)) = pair.split_once(':') else {
                        return Err(ToolError::Validation(format!("invalid clr_str token '{}': expected class:colour", pair)));
                    };
                    let cls = cls_txt
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| ToolError::Validation(format!("invalid class '{}' in clr_str", cls_txt.trim())))?;
                    if cls >= palette.len() {
                        continue;
                    }
                    palette[cls] = parse_rgb_spec(colour_txt.trim())?;
                }
            }

            let mut building_cluster_colour: HashMap<usize, Rgb16> = HashMap::new();
            let mut building_cluster_id = vec![0usize; cloud.points.len()];
            if unique_buildings {
                let mut tree = KdTree::new(2);
                let mut building_indices = Vec::new();
                for (i, p) in cloud.points.iter().enumerate() {
                    if p.classification == 6 {
                        tree.add([p.x, p.y], i).map_err(|e| {
                            ToolError::Execution(format!("failed indexing building points for colour clustering: {e}"))
                        })?;
                        building_indices.push(i);
                    }
                }
                let radius_sq = search_radius * search_radius;
                let mut next_cluster = 0usize;
                for seed in building_indices {
                    if building_cluster_id[seed] != 0 {
                        continue;
                    }
                    next_cluster += 1;
                    building_cluster_id[seed] = next_cluster;
                    let mut stack = vec![seed];
                    while let Some(idx) = stack.pop() {
                        let p = cloud.points[idx];
                        let neighbours = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                        for (_, nref) in neighbours {
                            let nidx = *nref;
                            if building_cluster_id[nidx] == 0 {
                                building_cluster_id[nidx] = next_cluster;
                                stack.push(nidx);
                            }
                        }
                    }
                }
                for cid in 1..=next_cluster {
                    let r = ((cid.wrapping_mul(67)) % 256) as u8;
                    let g = ((cid.wrapping_mul(149)) % 256) as u8;
                    let b = ((cid.wrapping_mul(211)) % 256) as u8;
                    building_cluster_colour.insert(cid, color8_to_rgb16(r, g, b));
                }
            }

            let mut points = cloud.points.clone();
            for (i, p) in points.iter_mut().enumerate() {
                let mut base = palette[(p.classification as usize).min(18)];
                if unique_buildings && p.classification == 6 {
                    if let Some(c) = building_cluster_colour.get(&building_cluster_id[i]) {
                        base = *c;
                    }
                }
                p.color = Some(blend_rgb_with_intensity(base, p.intensity, blend));
            }

            let out_cloud = PointCloud {
                points,
                crs: cloud.crs.clone(),
            };
            store_or_write_lidar_output(&out_cloud, out_path, "colourize_based_on_class")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "classified");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for ColourizeBasedOnPointReturnsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "colourize_based_on_point_returns",
            display_name: "Colourize Based On Point Returns",
            summary: "Sets LiDAR point RGB values based on return-type categories.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "intensity_blending_amount", description: "Percent blend [0,100] between return colour and intensity.", required: false },
                ToolParamSpec { name: "only_ret_colour", description: "Colour for only-return points.", required: false },
                ToolParamSpec { name: "first_ret_colour", description: "Colour for first-return points in multi-return pulses.", required: false },
                ToolParamSpec { name: "intermediate_ret_colour", description: "Colour for intermediate-return points.", required: false },
                ToolParamSpec { name: "last_ret_colour", description: "Colour for last-return points in multi-return pulses.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let blend = parse_f64_alias(args, &["intensity_blending_amount", "intensity_blending"], 50.0);
        if !blend.is_finite() || !(0.0..=100.0).contains(&blend) {
            return Err(ToolError::Validation("intensity_blending_amount must be in [0, 100]".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let blend = parse_f64_alias(args, &["intensity_blending_amount", "intensity_blending"], 50.0) / 100.0;
        let only_colour = parse_rgb_spec(
            args.get("only_ret_colour")
                .and_then(Value::as_str)
                .unwrap_or("(230,214,170)"),
        )?;
        let first_colour = parse_rgb_spec(
            args.get("first_ret_colour")
                .and_then(Value::as_str)
                .unwrap_or("(0,140,0)"),
        )?;
        let intermediate_colour = parse_rgb_spec(
            args.get("intermediate_ret_colour")
                .and_then(Value::as_str)
                .unwrap_or("(255,0,255)"),
        )?;
        let last_colour = parse_rgb_spec(
            args.get("last_ret_colour")
                .and_then(Value::as_str)
                .unwrap_or("(0,0,255)"),
        )?;
        let output_path = parse_optional_output_path(args, "output")?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path)
                .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy())))?;

            let mut points = cloud.points.clone();
            for p in &mut points {
                let nret = p.number_of_returns;
                let ret = p.return_number;
                let base = if nret <= 1 || ret == 0 {
                    only_colour
                } else if ret == 1 {
                    first_colour
                } else if ret < nret {
                    intermediate_colour
                } else {
                    last_colour
                };
                p.color = Some(blend_rgb_with_intensity(base, p.intensity, blend));
            }

            let out_cloud = PointCloud {
                points,
                crs: cloud.crs.clone(),
            };
            store_or_write_lidar_output(&out_cloud, out_path, "colourize_based_on_point_returns")
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar");
            let locator = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_lidar_result(locator))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_lidar_output_path(&input, "classified");
                    run_single(&input, Some(out))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for ClassifyBuildingsInLidarTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "classify_buildings_in_lidar",
            display_name: "Classify Buildings In LiDAR",
            summary: "Assigns classification 6 to LiDAR points falling inside building footprint polygons.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "buildings", description: "Input building-footprint polygon vector path or typed vector object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let _ = parse_required_vector_path_alias(args, &["buildings", "building_footprints", "polygons"], "buildings")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let buildings_path = parse_required_vector_path_alias(args, &["buildings", "building_footprints", "polygons"], "buildings")?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading input lidar and building polygons");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let polys = read_prepared_polygons(&buildings_path)?;

        let mut points = cloud.points.clone();
        for p in &mut points {
            if point_in_any_prepared_polygon(p.x, p.y, &polys) {
                p.classification = 6;
            }
        }

        let out_cloud = PointCloud {
            points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "classify_buildings_in_lidar")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for AsciiToLasTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ascii_to_las",
            display_name: "ASCII To LAS",
            summary: "Converts one or more ASCII LiDAR point files to LAS.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Array of input ASCII file paths.", required: true },
                ToolParamSpec { name: "pattern", description: "Field pattern, e.g. 'x,y,z,i,c,rn,nr,sa'.", required: true },
                ToolParamSpec { name: "epsg_code", description: "EPSG code for output LAS CRS metadata.", required: false },
                ToolParamSpec { name: "output_directory", description: "Optional output directory for generated LAS files.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_ascii_inputs_arg(args)?;
        let pattern = args
            .get("pattern")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("pattern is required".to_string()))?;
        let _ = parse_ascii_pattern(pattern)?;
        let epsg = parse_f64_alias(args, &["epsg_code", "epsg"], 4326.0).round() as i64;
        if !(1..=998_999).contains(&epsg) {
            return Err(ToolError::Validation("epsg_code must be in [1, 998999]".to_string()));
        }
        let _ = parse_optional_output_path(args, "output_directory")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_ascii_inputs_arg(args)?;
        let pattern = args
            .get("pattern")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("pattern is required".to_string()))?;
        let pattern = parse_ascii_pattern(pattern)?;
        let epsg = parse_f64_alias(args, &["epsg_code", "epsg"], 4326.0).round() as u32;
        let output_dir = parse_optional_output_path(args, "output_directory")?;
        if let Some(ref dir) = output_dir {
            fs::create_dir_all(dir)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory '{}': {e}", dir.to_string_lossy())))?;
        }

        let mut outputs = Vec::with_capacity(input_paths.len());
        for (file_idx, input_path_text) in input_paths.iter().enumerate() {
            ctx.progress.info(&format!(
                "parsing ascii input {} of {}",
                file_idx + 1,
                input_paths.len()
            ));
            let input_path = Path::new(input_path_text);
            let file = File::open(input_path).map_err(|e| {
                ToolError::Execution(format!(
                    "failed opening input ascii '{}': {e}",
                    input_path.to_string_lossy()
                ))
            })?;
            let reader = BufReader::new(file);

            let mut points = Vec::new();
            for (line_idx, line_result) in reader.lines().enumerate() {
                let line_num = line_idx + 1;
                let line = line_result.map_err(|e| {
                    ToolError::Execution(format!(
                        "failed reading '{}': line {}: {e}",
                        input_path.to_string_lossy(),
                        line_num
                    ))
                })?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let fields = split_ascii_line(trimmed);
                if fields.len() < pattern.field_count {
                    continue;
                }
                // Ignore a likely header row if the x field is not numeric.
                if fields[pattern.x_idx].parse::<f64>().is_err() {
                    continue;
                }

                let mut p = PointRecord {
                    return_number: 1,
                    number_of_returns: 1,
                    ..PointRecord::default()
                };
                p.x = parse_field::<f64>(&fields, pattern.x_idx, "x", line_num, input_path)?;
                p.y = parse_field::<f64>(&fields, pattern.y_idx, "y", line_num, input_path)?;
                p.z = parse_field::<f64>(&fields, pattern.z_idx, "z", line_num, input_path)?;

                if let Some(i_idx) = pattern.i_idx {
                    p.intensity = parse_field::<u16>(&fields, i_idx, "i", line_num, input_path)?;
                }
                if let Some(c_idx) = pattern.c_idx {
                    p.classification = parse_field::<u8>(&fields, c_idx, "c", line_num, input_path)?;
                }
                if let Some(rn_idx) = pattern.rn_idx {
                    p.return_number = parse_field::<u8>(&fields, rn_idx, "rn", line_num, input_path)?;
                }
                if let Some(nr_idx) = pattern.nr_idx {
                    p.number_of_returns = parse_field::<u8>(&fields, nr_idx, "nr", line_num, input_path)?;
                }
                if let Some(time_idx) = pattern.time_idx {
                    let t = parse_field::<f64>(&fields, time_idx, "time", line_num, input_path)?;
                    p.gps_time = Some(wblidar::GpsTime(t));
                }
                if let Some(sa_idx) = pattern.sa_idx {
                    p.scan_angle = parse_field::<i16>(&fields, sa_idx, "sa", line_num, input_path)?;
                }
                if let (Some(r_idx), Some(g_idx), Some(b_idx)) = (pattern.r_idx, pattern.g_idx, pattern.b_idx) {
                    p.color = Some(Rgb16 {
                        red: parse_field::<u16>(&fields, r_idx, "r", line_num, input_path)?,
                        green: parse_field::<u16>(&fields, g_idx, "g", line_num, input_path)?,
                        blue: parse_field::<u16>(&fields, b_idx, "b", line_num, input_path)?,
                    });
                }
                points.push(p);
            }

            let out_path = derived_las_output_from_ascii(input_path, output_dir.as_deref());
            if let Some(parent) = out_path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("failed creating output directory: {e}"))
                    })?;
                }
            }
            let cloud = PointCloud {
                points,
                crs: Some(LidarCrs::from_epsg(epsg)),
            };
            cloud.write(&out_path).map_err(|e| {
                ToolError::Execution(format!(
                    "failed writing output lidar '{}': {e}",
                    out_path.to_string_lossy()
                ))
            })?;
            outputs.push(out_path.to_string_lossy().to_string());
            ctx.progress.progress((file_idx + 1) as f64 / input_paths.len() as f64);
        }

        if outputs.len() == 1 {
            Ok(build_lidar_result(outputs[0].clone()))
        } else {
            build_batch_placeholder_lidar_result(outputs)
        }
    }
}

impl Tool for LasToAsciiTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "las_to_ascii",
            display_name: "LAS To ASCII",
            summary: "Converts LiDAR points to CSV ASCII text.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output CSV path (single-input mode only).", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let input = parse_lidar_path_arg_optional(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if input.is_none() && args.get("output").is_some() {
            return Err(ToolError::Validation(
                "output is only supported when an explicit input is provided".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let run_single = |in_path: &Path, out_path: Option<PathBuf>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!(
                    "failed reading input lidar '{}': {e}",
                    in_path.to_string_lossy()
                ))
            })?;
            let out = out_path.unwrap_or_else(|| derived_ascii_output_from_lidar(in_path));
            write_point_cloud_as_csv(&cloud, &out)?;
            Ok(out.to_string_lossy().to_string())
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("reading input lidar and writing csv");
            let out = run_single(Path::new(&input_path), output_path)?;
            ctx.progress.progress(1.0);
            Ok(build_string_output_result("output", out))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| run_single(&input, None))
                .collect::<Result<Vec<_>, _>>()?;
            if outputs.is_empty() {
                return Err(ToolError::Execution(
                    "batch mode produced no ASCII outputs".to_string(),
                ));
            }
            let mut sorted = outputs;
            sorted.sort();
            ctx.progress.progress(1.0);
            Ok(build_string_output_result("output", sorted[0].clone()))
        }
    }
}

impl Tool for SelectTilesByPolygonTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "select_tiles_by_polygon",
            display_name: "Select Tiles By Polygon",
            summary: "Copies LiDAR tiles from an input directory to an output directory when tile sample points overlap polygon geometries.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input_directory", description: "Input directory containing LiDAR tiles.", required: true },
                ToolParamSpec { name: "output_directory", description: "Output directory for selected LiDAR tiles.", required: true },
                ToolParamSpec { name: "polygons", description: "Input polygon vector path or typed vector object.", required: true },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let in_dir = args
            .get("input_directory")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("input_directory is required".to_string()))?;
        let out_dir = args
            .get("output_directory")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::Validation("output_directory is required".to_string()))?;
        if in_dir.trim().is_empty() {
            return Err(ToolError::Validation("input_directory must not be empty".to_string()));
        }
        if out_dir.trim().is_empty() {
            return Err(ToolError::Validation("output_directory must not be empty".to_string()));
        }
        let in_path = Path::new(in_dir);
        if !in_path.is_dir() {
            return Err(ToolError::Validation(format!(
                "input_directory '{}' does not exist or is not a directory",
                in_dir
            )));
        }
        let _ = parse_vector_path_arg(args, "polygons")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_directory = PathBuf::from(
            args.get("input_directory")
                .and_then(Value::as_str)
                .ok_or_else(|| ToolError::Validation("input_directory is required".to_string()))?
                .trim(),
        );
        let output_directory = PathBuf::from(
            args.get("output_directory")
                .and_then(Value::as_str)
                .ok_or_else(|| ToolError::Validation("output_directory is required".to_string()))?
                .trim(),
        );
        let poly_path = parse_vector_path_arg(args, "polygons")?;

        ctx.progress.info("reading polygon geometries");
        let polys = read_prepared_polygons(&poly_path)?;

        let mut tile_paths = Vec::new();
        for entry in fs::read_dir(&input_directory).map_err(|e| {
            ToolError::Execution(format!(
                "failed reading input directory '{}': {e}",
                input_directory.to_string_lossy()
            ))
        })? {
            let entry = entry.map_err(|e| ToolError::Execution(format!("failed reading directory entry: {e}")))?;
            let path = entry.path();
            if path.is_file() && is_valid_lidar_extension(&path) {
                tile_paths.push(path);
            }
        }
        tile_paths.sort();
        if tile_paths.is_empty() {
            return Err(ToolError::Execution(format!(
                "no LiDAR tiles found in input_directory '{}'",
                input_directory.to_string_lossy()
            )));
        }

        fs::create_dir_all(&output_directory).map_err(|e| {
            ToolError::Execution(format!(
                "failed creating output directory '{}': {e}",
                output_directory.to_string_lossy()
            ))
        })?;

        let copied = tile_paths
            .par_iter()
            .map(|tile| -> Result<Option<PathBuf>, ToolError> {
                let cloud = PointCloud::read(tile).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed reading lidar tile '{}': {e}",
                        tile.to_string_lossy()
                    ))
                })?;
                let Some(samples) = lidar_bbox_sample_points(&cloud) else {
                    return Ok(None);
                };
                let intersects = samples
                    .iter()
                    .any(|(x, y)| point_in_any_prepared_polygon(*x, *y, &polys));
                if !intersects {
                    return Ok(None);
                }
                let out_name = tile.file_name().ok_or_else(|| {
                    ToolError::Execution(format!(
                        "failed getting file name for tile '{}'",
                        tile.to_string_lossy()
                    ))
                })?;
                let out_path = output_directory.join(out_name);
                fs::copy(tile, &out_path).map_err(|e| {
                    ToolError::Execution(format!(
                        "failed copying '{}' to '{}': {e}",
                        tile.to_string_lossy(),
                        out_path.to_string_lossy()
                    ))
                })?;
                Ok(Some(out_path))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "output_directory".to_string(),
            json!(output_directory.to_string_lossy().to_string()),
        );
        outputs.insert("copied_files".to_string(), json!(copied.len()));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for LidarInfoTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_info",
            display_name: "LiDAR Info",
            summary: "Generates a textual or HTML summary report for a LiDAR file.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output report path (.html/.txt).", required: false },
                ToolParamSpec { name: "show_point_density", description: "If true includes approximate bbox point density metrics.", required: false },
                ToolParamSpec { name: "show_vlrs", description: "Compatibility flag; reserved for future detailed metadata output.", required: false },
                ToolParamSpec { name: "show_geokeys", description: "Compatibility flag; reserved for future detailed CRS-key output.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            ensure_html_or_txt(&path)?;
        }
        let _ = parse_bool_alias(args, &["show_point_density"], true);
        let _ = parse_bool_alias(args, &["show_vlrs"], true);
        let _ = parse_bool_alias(args, &["show_geokeys"], true);
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let show_density = parse_bool_alias(args, &["show_point_density"], true);

        ctx.progress.info("reading lidar and building info report");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let n = cloud.points.len();
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut max_z = f64::NEG_INFINITY;
        let mut min_i = u16::MAX;
        let mut max_i = u16::MIN;
        let mut class_counts: BTreeMap<u8, usize> = BTreeMap::new();
        let mut ret_counts = [0usize; 5];
        for p in &cloud.points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
            min_i = min_i.min(p.intensity);
            max_i = max_i.max(p.intensity);
            *class_counts.entry(p.classification).or_insert(0) += 1;
            let r = p.return_number.max(1).min(5) as usize;
            ret_counts[r - 1] += 1;
        }

        let mut report = String::new();
        report.push_str("LiDAR File Summary\n\n");
        report.push_str(&format!("input: {}\n", input_path));
        report.push_str(&format!("points: {}\n", n));
        if n > 0 {
            report.push_str(&format!(
                "x range: [{:.6}, {:.6}]\ny range: [{:.6}, {:.6}]\nz range: [{:.6}, {:.6}]\n",
                min_x, max_x, min_y, max_y, min_z, max_z
            ));
            report.push_str(&format!("intensity range: [{}, {}]\n", min_i, max_i));
            if show_density {
                let area = ((max_x - min_x).abs() * (max_y - min_y).abs()).max(1.0e-12);
                let density = n as f64 / area;
                report.push_str(&format!("bbox point density: {:.6} pts/unit^2\n", density));
            }
        }
        report.push_str("\nreturn counts (1..5):\n");
        for (i, c) in ret_counts.iter().enumerate() {
            report.push_str(&format!("  {}: {}\n", i + 1, c));
        }
        report.push_str("\nclass counts:\n");
        for (cls, c) in class_counts {
            report.push_str(&format!("  {}: {}\n", cls, c));
        }

        let out = output_path.unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "info", "txt"));
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
            }
        }
        let ext = out
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("txt")
            .to_ascii_lowercase();
        if ext == "html" || ext == "htm" {
            let mut html = String::new();
            html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>LiDAR Info</title></head><body><pre>");
            html.push_str(&report.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"));
            html.push_str("</pre></body></html>");
            fs::write(&out, html)
                .map_err(|e| ToolError::Execution(format!("failed writing report '{}': {e}", out.to_string_lossy())))?;
        } else {
            fs::write(&out, report)
                .map_err(|e| ToolError::Execution(format!("failed writing report '{}': {e}", out.to_string_lossy())))?;
        }

        ctx.progress.progress(1.0);
        Ok(build_string_output_result(
            "report_path",
            out.to_string_lossy().to_string(),
        ))
    }
}

impl Tool for LidarHistogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_histogram",
            display_name: "LiDAR Histogram",
            summary: "Builds a simple histogram report for a selected LiDAR attribute.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output HTML path for histogram report.", required: false },
                ToolParamSpec { name: "parameter", description: "One of elevation, intensity, scan angle, class, or time.", required: false },
                ToolParamSpec { name: "clip_percent", description: "Percentile clip amount in [0,50] for lower/upper tails.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            ensure_html_or_txt(&path)?;
        }
        let clip = parse_f64_alias(args, &["clip_percent", "clip"], 1.0);
        if !clip.is_finite() || !(0.0..=50.0).contains(&clip) {
            return Err(ToolError::Validation("clip_percent must be in [0, 50]".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let parameter = parse_histogram_parameter(
            args.get("parameter")
                .and_then(Value::as_str)
                .unwrap_or("elevation"),
        );
        let clip_percent = parse_f64_alias(args, &["clip_percent", "clip"], 1.0);

        ctx.progress.info("reading lidar and computing histogram bins");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        if cloud.points.is_empty() {
            return Err(ToolError::Execution("input LiDAR has no points".to_string()));
        }

        let mut values: Vec<f64> = cloud
            .points
            .iter()
            .map(|p| match parameter {
                "intensity" => f64::from(p.intensity),
                "scan_angle" => f64::from(p.scan_angle),
                "class" => f64::from(p.classification),
                "time" => p.gps_time.map(|t| t.0).unwrap_or(0.0),
                _ => p.z,
            })
            .collect();

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let clip = clip_percent / 100.0;
        let lo = quantile(&values, clip);
        let hi = quantile(&values, 1.0 - clip).max(lo + 1.0e-12);

        let bins = if parameter == "class" { 256usize } else { (values.len() as f64).log2().ceil() as usize + 1 };
        let bins = bins.max(8).min(512);
        let width = (hi - lo) / bins as f64;
        let mut freq = vec![0usize; bins];
        for v in values {
            if v < lo || v > hi {
                continue;
            }
            let mut b = ((v - lo) / width).floor() as isize;
            if b < 0 {
                b = 0;
            }
            if b as usize >= bins {
                b = bins as isize - 1;
            }
            freq[b as usize] += 1;
        }

        let mut html = String::new();
        html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>LiDAR Histogram</title></head><body>");
        html.push_str(&format!(
            "<h1>LiDAR Histogram</h1><p><strong>Input:</strong> {}<br><strong>Parameter:</strong> {}<br><strong>Clip:</strong> {}%</p>",
            input_path, parameter, clip_percent
        ));
        html.push_str("<table border=\"1\" cellspacing=\"0\" cellpadding=\"4\"><tr><th>Bin</th><th>Start</th><th>End</th><th>Count</th></tr>");
        for (i, c) in freq.iter().enumerate() {
            let a = lo + i as f64 * width;
            let b = a + width;
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.6}</td><td>{:.6}</td><td>{}</td></tr>",
                i, a, b, c
            ));
        }
        html.push_str("</table></body></html>");

        let out = output_path.unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "histogram", "html"));
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
            }
        }
        fs::write(&out, html)
            .map_err(|e| ToolError::Execution(format!("failed writing histogram '{}': {e}", out.to_string_lossy())))?;

        ctx.progress.progress(1.0);
        Ok(build_string_output_result(
            "report_path",
            out.to_string_lossy().to_string(),
        ))
    }
}

impl Tool for LidarPointStatsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_point_stats",
            display_name: "LiDAR Point Stats",
            summary: "Creates one or more raster grids summarizing LiDAR point distributions.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Output grid resolution.", required: false },
                ToolParamSpec { name: "num_points", description: "Create point-count raster.", required: false },
                ToolParamSpec { name: "num_pulses", description: "Create early-return pulse-count raster.", required: false },
                ToolParamSpec { name: "avg_points_per_pulse", description: "Create average points-per-pulse raster.", required: false },
                ToolParamSpec { name: "z_range", description: "Create elevation-range raster.", required: false },
                ToolParamSpec { name: "intensity_range", description: "Create intensity-range raster.", required: false },
                ToolParamSpec { name: "predominant_class", description: "Create predominant-class raster.", required: false },
                ToolParamSpec { name: "output_directory", description: "Optional output directory for generated rasters.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let res = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !res.is_finite() || res <= 0.0 {
            return Err(ToolError::Validation("resolution must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output_directory")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        let mut num_points_flag = parse_bool_alias(args, &["num_points"], false);
        let mut num_pulses_flag = parse_bool_alias(args, &["num_pulses"], false);
        let mut avg_pp_flag = parse_bool_alias(args, &["avg_points_per_pulse"], false);
        let mut z_range_flag = parse_bool_alias(args, &["z_range"], false);
        let mut i_range_flag = parse_bool_alias(args, &["intensity_range"], false);
        let mut pred_class_flag = parse_bool_alias(args, &["predominant_class"], false);
        let output_dir = parse_optional_output_path(args, "output_directory")?;

        if !(num_points_flag || num_pulses_flag || avg_pp_flag || z_range_flag || i_range_flag || pred_class_flag) {
            num_points_flag = true;
            num_pulses_flag = true;
            avg_pp_flag = true;
            z_range_flag = true;
            i_range_flag = true;
            pred_class_flag = true;
        }

        let run_single = |in_path: &Path, out_dir_override: Option<&Path>| -> Result<Vec<String>, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy()))
            })?;
            if cloud.points.is_empty() {
                return Ok(Vec::new());
            }

            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

            let cols = (((max_x - min_x) / resolution).ceil() as usize).max(1);
            let rows = (((max_y - min_y) / resolution).ceil() as usize).max(1);

            let cfg = RasterConfig {
                cols,
                rows,
                bands: 1,
                x_min: min_x,
                y_min: min_y,
                cell_size: resolution,
                cell_size_y: None,
                nodata: -9999.0,
                data_type: DataType::F64,
                crs: lidar_crs_to_raster_crs(cloud.crs.as_ref()),
                metadata: Vec::new(),
            };

            let mut count = vec![0usize; rows * cols];
            let mut pulses = vec![0usize; rows * cols];
            let mut minz = vec![f64::INFINITY; rows * cols];
            let mut maxz = vec![f64::NEG_INFINITY; rows * cols];
            let mut mini = vec![u16::MAX; rows * cols];
            let mut maxi = vec![u16::MIN; rows * cols];
            let mut class_counts: Vec<HashMap<u8, usize>> = vec![HashMap::new(); rows * cols];

            let idx = |row: usize, col: usize| -> usize { row * cols + col };
            for p in &cloud.points {
                let col = ((p.x - min_x) / resolution).floor() as isize;
                let row = ((max_y - p.y) / resolution).floor() as isize;
                if row < 0 || col < 0 || row >= rows as isize || col >= cols as isize {
                    continue;
                }
                let i = idx(row as usize, col as usize);
                count[i] += 1;
                if p.return_number <= 1 {
                    pulses[i] += 1;
                }
                minz[i] = minz[i].min(p.z);
                maxz[i] = maxz[i].max(p.z);
                mini[i] = mini[i].min(p.intensity);
                maxi[i] = maxi[i].max(p.intensity);
                *class_counts[i].entry(p.classification).or_insert(0) += 1;
            }

            let out_dir = out_dir_override
                .map(Path::to_path_buf)
                .or_else(|| in_path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."));
            fs::create_dir_all(&out_dir)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory '{}': {e}", out_dir.to_string_lossy())))?;
            let stem = in_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("lidar")
                .trim_end_matches(".copc");

            let mut outputs = Vec::new();

            let mut write_stat = |suffix: &str, compute: &dyn Fn(usize) -> Option<f64>| -> Result<(), ToolError> {
                let mut raster = Raster::new(cfg.clone());
                for row in 0..rows {
                    for col in 0..cols {
                        let i = idx(row, col);
                        let v = compute(i).unwrap_or(cfg.nodata);
                        raster
                            .set(0, row as isize, col as isize, v)
                            .map_err(|e| ToolError::Execution(format!("failed writing raster cell: {e}")))?;
                    }
                }
                let out_path = out_dir.join(format!("{stem}_{suffix}.tif"));
                raster.write(&out_path, RasterFormat::GeoTiff).map_err(|e| {
                    ToolError::Execution(format!("failed writing raster '{}': {e}", out_path.to_string_lossy()))
                })?;
                outputs.push(out_path.to_string_lossy().to_string());
                Ok(())
            };

            if num_points_flag {
                write_stat("num_pnts", &|i| Some(count[i] as f64))?;
            }
            if num_pulses_flag {
                write_stat("num_pulses", &|i| Some(pulses[i] as f64))?;
            }
            if avg_pp_flag {
                write_stat("avg_points_per_pulse", &|i| {
                    if pulses[i] > 0 {
                        Some(count[i] as f64 / pulses[i] as f64)
                    } else {
                        None
                    }
                })?;
            }
            if z_range_flag {
                write_stat("z_range", &|i| {
                    if count[i] > 0 {
                        Some(maxz[i] - minz[i])
                    } else {
                        None
                    }
                })?;
            }
            if i_range_flag {
                write_stat("intensity_range", &|i| {
                    if count[i] > 0 {
                        Some(f64::from(maxi[i] - mini[i]))
                    } else {
                        None
                    }
                })?;
            }
            if pred_class_flag {
                write_stat("predominant_class", &|i| {
                    class_counts[i]
                        .iter()
                        .max_by_key(|(_, c)| **c)
                        .map(|(cls, _)| f64::from(*cls))
                })?;
            }

            Ok(outputs)
        };

        let output_dir_ref = output_dir.as_deref();
        if let Some(input_path) = input_path {
            ctx.progress.info("computing lidar point stats rasters");
            let outputs = run_single(Path::new(&input_path), output_dir_ref)?;
            if outputs.is_empty() {
                return Err(ToolError::Execution("no point-stats rasters were generated".to_string()));
            }
            let mut sorted = outputs;
            sorted.sort();
            ctx.progress.progress(1.0);
            Ok(build_string_output_result("output_directory", output_dir_ref
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| Path::new(&input_path)
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_string_lossy()
                    .to_string())))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let all_outputs = files
                .into_par_iter()
                .map(|input| run_single(&input, output_dir_ref))
                .collect::<Result<Vec<_>, _>>()?;
            let generated: usize = all_outputs.iter().map(Vec::len).sum();
            if generated == 0 {
                return Err(ToolError::Execution("batch mode produced no point-stats rasters".to_string()));
            }
            ctx.progress.progress(1.0);
            Ok(build_string_output_result(
                "output_directory",
                output_dir_ref
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).to_string_lossy().to_string()),
            ))
        }
    }
}

impl Tool for LidarContourTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_contour",
            display_name: "LiDAR Contour",
            summary: "Creates contour vector lines from a LiDAR point cloud using TIN contouring.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output vector path (.shp/.geojson). In batch mode each input writes a sibling .shp.", required: false },
                ToolParamSpec { name: "interval", description: "Contour interval (must be > 0).", required: false },
                ToolParamSpec { name: "base_contour", description: "Base contour offset value.", required: false },
                ToolParamSpec { name: "smooth", description: "Compatibility smoothing parameter (accepted for call-shape parity).", required: false },
                ToolParamSpec { name: "interpolation_parameter", description: "One of elevation, intensity, scan_angle, time, or user_data.", required: false },
                ToolParamSpec { name: "returns", description: "Returns filter: all, first, or last.", required: false },
                ToolParamSpec { name: "excluded_classes", description: "Optional class values to exclude.", required: false },
                ToolParamSpec { name: "min_elev", description: "Minimum allowed point elevation.", required: false },
                ToolParamSpec { name: "max_elev", description: "Maximum allowed point elevation.", required: false },
                ToolParamSpec { name: "max_triangle_edge_length", description: "Optional maximum triangle edge length; longer triangles are skipped.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
        }
        let interval = parse_f64_alias(args, &["interval", "contour_interval"], 10.0);
        if !interval.is_finite() || interval <= 0.0 {
            return Err(ToolError::Validation("interval must be a positive finite value".to_string()));
        }
        let _ = parse_f64_alias(args, &["base_contour"], 0.0);
        let smooth = parse_f64_alias(args, &["smooth"], 5.0) as i64;
        if smooth < 0 {
            return Err(ToolError::Validation("smooth must be >= 0".to_string()));
        }
        let _ = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
        let _ = parse_excluded_classes(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let interval = parse_f64_alias(args, &["interval", "contour_interval"], 10.0);
        let base = parse_f64_alias(args, &["base_contour"], 0.0);
        let parameter = args
            .get("interpolation_parameter")
            .and_then(Value::as_str)
            .unwrap_or("elevation")
            .trim()
            .to_ascii_lowercase();
        if !supports_interpolation_parameter(&parameter) {
            return Err(ToolError::Validation(format!(
                "unsupported interpolation_parameter '{}'",
                parameter
            )));
        }
        let returns_mode = parse_returns_mode(args);
        let include_classes = parse_excluded_classes(args)?;
        let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
        let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
        let max_edge = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
        let max_edge_sq = if max_edge.is_finite() { max_edge * max_edge } else { f64::INFINITY };

        let run_single = |in_path: &Path, out_path: Option<&Path>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy()))
            })?;
            let samples = collect_lidar_samples(
                &cloud.points,
                &parameter,
                returns_mode,
                &include_classes,
                min_z,
                max_z,
            )?;
            if samples.len() < 3 {
                return Err(ToolError::Validation(
                    "input lidar must contain at least three points after filtering".to_string(),
                ));
            }

            let topo_points: Vec<TopoCoord> = samples
                .iter()
                .map(|(x, y, _)| TopoCoord::xy(*x, *y))
                .collect();
            let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
            if triangulation.triangles.is_empty() {
                return Err(ToolError::Execution(
                    "failed to build triangulation from input lidar points".to_string(),
                ));
            }

            let mut layer = wbvector::Layer::new("lidar_contours").with_geom_type(wbvector::GeometryType::LineString);
            layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());
            layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
            layer.add_field(wbvector::FieldDef::new("HEIGHT", wbvector::FieldType::Float));

            let mut fid = 1i64;
            for tri in &triangulation.triangles {
                let (x1, y1, z1) = samples[tri[0]];
                let (x2, y2, z2) = samples[tri[1]];
                let (x3, y3, z3) = samples[tri[2]];
                if max_triangle_edge_length_2d_sq((x1, y1), (x2, y2), (x3, y3)) > max_edge_sq {
                    continue;
                }

                let min_val = z1.min(z2.min(z3));
                let max_val = z1.max(z2.max(z3));
                let lower = ((min_val - base) / interval).ceil() as i64;
                let upper = ((max_val - base) / interval).floor() as i64;
                if lower > upper {
                    continue;
                }

                for level_idx in lower..=upper {
                    let level = base + level_idx as f64 * interval;
                    let mut intersections: Vec<(f64, f64)> = Vec::with_capacity(3);
                    if let Some(p) = interpolate_edge_contour((x1, y1), z1, (x2, y2), z2, level) {
                        intersections.push(p);
                    }
                    if let Some(p) = interpolate_edge_contour((x2, y2), z2, (x3, y3), z3, level) {
                        intersections.push(p);
                    }
                    if let Some(p) = interpolate_edge_contour((x3, y3), z3, (x1, y1), z1, level) {
                        intersections.push(p);
                    }
                    if intersections.len() < 2 {
                        continue;
                    }

                    let a = intersections[0];
                    let b = intersections[1];
                    layer
                        .add_feature(
                            Some(wbvector::Geometry::line_string(vec![
                                wbvector::Coord::xy(a.0, a.1),
                                wbvector::Coord::xy(b.0, b.1),
                            ])),
                            &[
                                ("FID", wbvector::FieldValue::Integer(fid)),
                                ("HEIGHT", wbvector::FieldValue::Float(level)),
                            ],
                        )
                        .map_err(|e| ToolError::Execution(format!("failed creating contour feature: {}", e)))?;
                    fid += 1;
                }
            }

            if layer.features.is_empty() {
                return Err(ToolError::Execution(
                    "no contour lines were generated from input lidar".to_string(),
                ));
            }

            let out = out_path
                .map(Path::to_path_buf)
                .unwrap_or_else(|| default_output_sibling_path(in_path, "contours", "shp"));
            write_vector_output(&layer, out.to_string_lossy().as_ref())
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("building contour vectors from lidar input");
            let out = run_single(Path::new(&input_path), output_path.as_deref())?;
            ctx.progress.progress(1.0);
            Ok(build_vector_result(out))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|p| run_single(&p, None))
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_vector_result(outputs)
        }
    }
}

impl Tool for LidarTileFootprintTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_tile_footprint",
            display_name: "LiDAR Tile Footprint",
            summary: "Creates polygon footprints (bounding boxes or convex hulls) for LiDAR tiles.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs over all LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output vector path for footprints.", required: false },
                ToolParamSpec { name: "output_hulls", description: "If true writes convex-hull footprints; otherwise writes axis-aligned bounding boxes.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
        }
        let _ = parse_bool_alias(args, &["output_hulls", "hull"], false);
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_hulls = parse_bool_alias(args, &["output_hulls", "hull"], false);

        let mut files = if let Some(input) = input_path.as_ref() {
            vec![PathBuf::from(input)]
        } else {
            find_lidar_files()?
        };
        files.sort();

        let mut layer = wbvector::Layer::new("lidar_tile_footprints").with_geom_type(wbvector::GeometryType::Polygon);
        layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("LAS_NM", wbvector::FieldType::Text));
        layer.add_field(wbvector::FieldDef::new("NUM_PNTS", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("Z_MIN", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("Z_MAX", wbvector::FieldType::Float));

        let mut fid = 1i64;
        for file in &files {
            let cloud = PointCloud::read(file).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", file.to_string_lossy()))
            })?;
            if layer.crs.is_none() {
                layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());
            }
            if cloud.points.is_empty() {
                continue;
            }

            let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
            let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
            let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
            let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
            let min_z = cloud.points.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
            let max_z = cloud.points.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max);

            let ring = if output_hulls {
                let xy: Vec<(f64, f64)> = cloud.points.iter().map(|p| (p.x, p.y)).collect();
                let hull = monotonic_chain_convex_hull(&xy);
                if hull.len() >= 3 {
                    close_ring(hull.into_iter().map(|(x, y)| wbvector::Coord::xy(x, y)).collect())
                } else {
                    vec![
                        wbvector::Coord::xy(min_x, max_y),
                        wbvector::Coord::xy(max_x, max_y),
                        wbvector::Coord::xy(max_x, min_y),
                        wbvector::Coord::xy(min_x, min_y),
                        wbvector::Coord::xy(min_x, max_y),
                    ]
                }
            } else {
                vec![
                    wbvector::Coord::xy(min_x, max_y),
                    wbvector::Coord::xy(max_x, max_y),
                    wbvector::Coord::xy(max_x, min_y),
                    wbvector::Coord::xy(min_x, min_y),
                    wbvector::Coord::xy(min_x, max_y),
                ]
            };

            let name = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("lidar")
                .to_string();

            layer
                .add_feature(
                    Some(wbvector::Geometry::polygon(ring, vec![])),
                    &[
                        ("FID", wbvector::FieldValue::Integer(fid)),
                        ("LAS_NM", wbvector::FieldValue::Text(name)),
                        ("NUM_PNTS", wbvector::FieldValue::Integer(cloud.points.len() as i64)),
                        ("Z_MIN", wbvector::FieldValue::Float(min_z)),
                        ("Z_MAX", wbvector::FieldValue::Float(max_z)),
                    ],
                )
                .map_err(|e| ToolError::Execution(format!("failed creating footprint feature: {}", e)))?;
            fid += 1;
        }

        if layer.features.is_empty() {
            return Err(ToolError::Execution(
                "no footprints were generated from input lidar files".to_string(),
            ));
        }

        let out = output_path.unwrap_or_else(|| {
            if let Some(input) = &input_path {
                default_output_sibling_path(Path::new(input), "footprint", "shp")
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("lidar_tile_footprint.shp")
            }
        });

        ctx.progress.info("writing lidar footprint vector output");
        let out_path = write_vector_output(&layer, out.to_string_lossy().as_ref())?;
        ctx.progress.progress(1.0);
        Ok(build_vector_result(out_path))
    }
}

impl Tool for LidarConstructVectorTinTool {
            fn metadata(&self) -> ToolMetadata {
                ToolMetadata {
                    id: "lidar_construct_vector_tin",
                    display_name: "LiDAR Construct Vector TIN",
                    summary: "Creates a vector TIN (triangular mesh) from LiDAR points.",
                    category: ToolCategory::Lidar,
                    license_tier: LicenseTier::Open,
                    params: vec![
                        ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                        ToolParamSpec { name: "output", description: "Optional output vector path; in batch mode each input writes a sibling _tin.shp.", required: false },
                        ToolParamSpec { name: "returns", description: "Returns filter: all, first, or last.", required: false },
                        ToolParamSpec { name: "excluded_classes", description: "Optional class values to exclude.", required: false },
                        ToolParamSpec { name: "min_elev", description: "Minimum allowed point elevation.", required: false },
                        ToolParamSpec { name: "max_elev", description: "Maximum allowed point elevation.", required: false },
                        ToolParamSpec { name: "max_triangle_edge_length", description: "Optional maximum allowed triangle edge length.", required: false },
                    ],
                }
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = parse_lidar_path_arg_optional(args)?;
                if let Some(path) = parse_optional_output_path(args, "output")? {
                    let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
                }
                let _ = parse_excluded_classes(args)?;
                let max_edge = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
                if !max_edge.is_finite() && !max_edge.is_infinite() {
                    return Err(ToolError::Validation(
                        "max_triangle_edge_length must be finite or infinity".to_string(),
                    ));
                }
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                let input_path = parse_lidar_path_arg_optional(args)?;
                let output_path = parse_optional_output_path(args, "output")?;
                let returns_mode = parse_returns_mode(args);
                let include_classes = parse_excluded_classes(args)?;
                let min_z = parse_f64_alias(args, &["min_elev", "minz"], f64::NEG_INFINITY);
                let max_z = parse_f64_alias(args, &["max_elev", "maxz"], f64::INFINITY);
                let max_edge = parse_f64_alias(args, &["max_triangle_edge_length"], f64::INFINITY);
                let max_edge_sq = if max_edge.is_infinite() {
                    f64::INFINITY
                } else {
                    max_edge * max_edge
                };

                let run_single = |in_path: &Path, out_path: Option<&Path>| -> Result<String, ToolError> {
                    let cloud = PointCloud::read(in_path).map_err(|e| {
                        ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy()))
                    })?;
                    let samples = collect_lidar_samples(
                        &cloud.points,
                        "elevation",
                        returns_mode,
                        &include_classes,
                        min_z,
                        max_z,
                    )?;
                    if samples.len() < 3 {
                        return Err(ToolError::Validation(
                            "input lidar must contain at least three points after filtering".to_string(),
                        ));
                    }

                    let topo_points: Vec<TopoCoord> = samples
                        .iter()
                        .map(|(x, y, _)| TopoCoord::xy(*x, *y))
                        .collect();
                    let triangulation = delaunay_triangulation(&topo_points, 1.0e-12);
                    if triangulation.triangles.is_empty() {
                        return Err(ToolError::Execution(
                            "failed to build triangulation from input lidar points".to_string(),
                        ));
                    }

                    let mut layer = wbvector::Layer::new("lidar_tin").with_geom_type(wbvector::GeometryType::Polygon);
                    layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());
                    layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
                    layer.add_field(wbvector::FieldDef::new("AVG_Z", wbvector::FieldType::Float));

                    let mut fid = 1i64;
                    for tri in &triangulation.triangles {
                        let a = samples[tri[0]];
                        let b = samples[tri[1]];
                        let c = samples[tri[2]];
                        if max_triangle_edge_length_2d_sq((a.0, a.1), (b.0, b.1), (c.0, c.1)) > max_edge_sq {
                            continue;
                        }
                        let ring = vec![
                            wbvector::Coord::xy(a.0, a.1),
                            wbvector::Coord::xy(b.0, b.1),
                            wbvector::Coord::xy(c.0, c.1),
                            wbvector::Coord::xy(a.0, a.1),
                        ];
                        layer
                            .add_feature(
                                Some(wbvector::Geometry::polygon(ring, vec![])),
                                &[
                                    ("FID", wbvector::FieldValue::Integer(fid)),
                                    (
                                        "AVG_Z",
                                        wbvector::FieldValue::Float((a.2 + b.2 + c.2) / 3.0),
                                    ),
                                ],
                            )
                            .map_err(|e| ToolError::Execution(format!("failed creating TIN feature: {}", e)))?;
                        fid += 1;
                    }

                    if layer.features.is_empty() {
                        return Err(ToolError::Execution(
                            "no TIN triangles were generated from input lidar".to_string(),
                        ));
                    }

                    let out = out_path
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| default_output_sibling_path(in_path, "tin", "shp"));
                    write_vector_output(&layer, out.to_string_lossy().as_ref())
                };

                if let Some(input_path) = input_path {
                    ctx.progress.info("constructing vector TIN from lidar input");
                    let out = run_single(Path::new(&input_path), output_path.as_deref())?;
                    ctx.progress.progress(1.0);
                    Ok(build_vector_result(out))
                } else {
                    ctx.progress.info("batch mode: scanning working directory for lidar files");
                    let files = find_lidar_files()?;
                    let outputs = files
                        .into_par_iter()
                        .map(|p| run_single(&p, None))
                        .collect::<Result<Vec<_>, _>>()?;
                    ctx.progress.progress(1.0);
                    build_batch_placeholder_vector_result(outputs)
                }
            }
        }

    impl Tool for LidarHexBinTool {
            fn metadata(&self) -> ToolMetadata {
                ToolMetadata {
                    id: "lidar_hex_bin",
                    display_name: "LiDAR Hex Bin",
                    summary: "Bins LiDAR points into a hexagonal grid and outputs per-cell summary attributes.",
                    category: ToolCategory::Lidar,
                    license_tier: LicenseTier::Open,
                    params: vec![
                        ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                        ToolParamSpec { name: "width", description: "Hexagon width (distance between opposite sides).", required: true },
                        ToolParamSpec { name: "orientation", description: "Grid orientation: h (pointy-up) or v (flat-up).", required: false },
                        ToolParamSpec { name: "output", description: "Optional output vector path.", required: false },
                    ],
                }
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
                let width = parse_f64_alias(args, &["width"], f64::NAN);
                if !width.is_finite() || width <= 0.0 {
                    return Err(ToolError::Validation("width must be a positive finite value".to_string()));
                }
                if let Some(path) = parse_optional_output_path(args, "output")? {
                    let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
                }
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
                let output_path = parse_optional_output_path(args, "output")?;
                let width = parse_f64_alias(args, &["width"], 0.0);
                let orientation = args
                    .get("orientation")
                    .and_then(Value::as_str)
                    .unwrap_or("h")
                    .trim()
                    .to_ascii_lowercase();
                let is_vertical = orientation.starts_with('v');

                ctx.progress.info("building hexagonal bins from lidar points");
                let cloud = PointCloud::read(Path::new(&input_path)).map_err(|e| {
                    ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
                })?;
                if cloud.points.is_empty() {
                    return Err(ToolError::Execution("input lidar has no points".to_string()));
                }

                let min_x = cloud.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                let max_x = cloud.points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                let min_y = cloud.points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                let max_y = cloud.points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

                let sixty = std::f64::consts::PI / 6.0;
                let half_width = 0.5 * width;
                let size = half_width / sixty.cos();
                let height = size * 2.0;
                let step_a = width;
                let step_b = 0.75 * height;

                let mut centers: Vec<(f64, f64)> = Vec::new();
                if !is_vertical {
                    let center_x_0 = min_x + half_width;
                    let center_y_0 = max_y - 0.25 * height;
                    let rows = (((max_y - min_y) / step_b).ceil() as usize).max(1);
                    for row in 0..rows {
                        let cy = center_y_0 - row as f64 * step_b;
                        let cols = (((max_x - min_x + half_width * ((row % 2) as f64)) / step_a).ceil() as usize).max(1);
                        for col in 0..cols {
                            let cx = (center_x_0 - half_width * ((row % 2) as f64)) + col as f64 * step_a;
                            centers.push((cx, cy));
                        }
                    }
                } else {
                    let center_x_0 = min_x + 0.5 * size;
                    let center_y_0 = max_y - half_width;
                    let cols = (((max_x - min_x) / step_b).ceil() as usize).max(1);
                    for col in 0..cols {
                        let rows = (((max_y - min_y + ((col % 2) as f64) * half_width) / step_a).ceil() as usize).max(1);
                        for row in 0..rows {
                            let cx = center_x_0 + col as f64 * step_b;
                            let cy = center_y_0 - row as f64 * step_a + ((col % 2) as f64) * half_width;
                            centers.push((cx, cy));
                        }
                    }
                }

                if centers.is_empty() {
                    return Err(ToolError::Execution("failed generating hexagonal grid centers".to_string()));
                }

                let mut tree = KdTree::new(2);
                for (i, c) in centers.iter().enumerate() {
                    tree.add([c.0, c.1], i)
                        .map_err(|e| ToolError::Execution(format!("failed building hex index: {e}")))?;
                }

                let mut count = vec![0usize; centers.len()];
                let mut min_z = vec![f64::INFINITY; centers.len()];
                let mut max_z = vec![f64::NEG_INFINITY; centers.len()];
                let mut min_i = vec![u16::MAX; centers.len()];
                let mut max_i = vec![u16::MIN; centers.len()];

                for p in &cloud.points {
                    if let Some((_, idx_ref)) = tree
                        .nearest(&[p.x, p.y], 1, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("failed searching hex index: {e}")))?
                        .into_iter()
                        .next()
                    {
                        let idx = *idx_ref;
                        count[idx] += 1;
                        min_z[idx] = min_z[idx].min(p.z);
                        max_z[idx] = max_z[idx].max(p.z);
                        min_i[idx] = min_i[idx].min(p.intensity);
                        max_i[idx] = max_i[idx].max(p.intensity);
                    }
                }

                let mut layer = wbvector::Layer::new("lidar_hex_bin").with_geom_type(wbvector::GeometryType::Polygon);
                layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());
                layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("COUNT", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("MIN_Z", wbvector::FieldType::Float));
                layer.add_field(wbvector::FieldDef::new("MAX_Z", wbvector::FieldType::Float));
                layer.add_field(wbvector::FieldDef::new("MIN_I", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("MAX_I", wbvector::FieldType::Integer));

                let mut fid = 1i64;
                for (idx, (cx, cy)) in centers.iter().enumerate() {
                    if count[idx] == 0 {
                        continue;
                    }
                    let mut ring: Vec<wbvector::Coord> = Vec::with_capacity(7);
                    for i in (0..=6).rev() {
                        let theta = if !is_vertical {
                            2.0 * sixty * (i as f64 + 0.5)
                        } else {
                            2.0 * sixty * (i as f64 + 0.5) - sixty
                        };
                        ring.push(wbvector::Coord::xy(cx + size * theta.cos(), cy + size * theta.sin()));
                    }
                    layer
                        .add_feature(
                            Some(wbvector::Geometry::polygon(ring, vec![])),
                            &[
                                ("FID", wbvector::FieldValue::Integer(fid)),
                                ("COUNT", wbvector::FieldValue::Integer(count[idx] as i64)),
                                ("MIN_Z", wbvector::FieldValue::Float(min_z[idx])),
                                ("MAX_Z", wbvector::FieldValue::Float(max_z[idx])),
                                ("MIN_I", wbvector::FieldValue::Integer(i64::from(min_i[idx]))),
                                ("MAX_I", wbvector::FieldValue::Integer(i64::from(max_i[idx]))),
                            ],
                        )
                        .map_err(|e| ToolError::Execution(format!("failed creating hex-bin feature: {}", e)))?;
                    fid += 1;
                }

                if layer.features.is_empty() {
                    return Err(ToolError::Execution("hex binning produced no populated cells".to_string()));
                }

                let out = output_path
                    .unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "hex_bin", "shp"));
                let out_path = write_vector_output(&layer, out.to_string_lossy().as_ref())?;
                ctx.progress.progress(1.0);
                Ok(build_vector_result(out_path))
            }
        }

    impl Tool for LidarPointReturnAnalysisTool {
            fn metadata(&self) -> ToolMetadata {
                ToolMetadata {
                    id: "lidar_point_return_analysis",
                    display_name: "LiDAR Point Return Analysis",
                    summary: "Runs return-sequence QC analysis and writes a text report; optionally writes a classified QC LiDAR output.",
                    category: ToolCategory::Lidar,
                    license_tier: LicenseTier::Open,
                    params: vec![
                        ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                        ToolParamSpec { name: "create_output", description: "If true writes a classified LiDAR QC output.", required: false },
                        ToolParamSpec { name: "output", description: "Optional LiDAR output path used when create_output=true.", required: false },
                        ToolParamSpec { name: "report", description: "Optional text report output path.", required: false },
                    ],
                }
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
                let _ = parse_bool_alias(args, &["create_output"], false);
                let _ = parse_optional_output_path(args, "output")?;
                let _ = parse_optional_output_path(args, "report")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
                let create_output = parse_bool_alias(args, &["create_output"], false);
                let output_path = parse_optional_output_path(args, "output")?;
                let report_path = parse_optional_output_path(args, "report")?;

                ctx.progress.info("analyzing lidar return sequence quality");
                let cloud = PointCloud::read(Path::new(&input_path)).map_err(|e| {
                    ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
                })?;
                if cloud.points.is_empty() {
                    return Err(ToolError::Execution("input lidar has no points".to_string()));
                }

                let mut grouped: HashMap<(u64, u8, u8), Vec<(usize, u8)>> = HashMap::new();
                let mut r_greater_n = 0usize;
                for (i, p) in cloud.points.iter().enumerate() {
                    if p.return_number > p.number_of_returns {
                        r_greater_n += 1;
                    }
                    let time_bits = p.gps_time.map(|t| t.0.to_bits()).unwrap_or((i as f64).to_bits());
                    let channel = (p.flags & 0b0000_0011) as u8;
                    grouped
                        .entry((time_bits, channel, p.number_of_returns))
                        .or_default()
                        .push((i, p.return_number));
                }

                let mut missing_points = 0usize;
                let mut duplicate_points = 0usize;
                let mut missing_by_rn: BTreeMap<(u8, u8), usize> = BTreeMap::new();
                let mut duplicate_by_rn: BTreeMap<(u8, u8), usize> = BTreeMap::new();
                let mut is_missing = vec![false; cloud.points.len()];
                let mut is_duplicate = vec![false; cloud.points.len()];

                for ((_, _, nret), members) in grouped {
                    if nret == 0 {
                        continue;
                    }
                    let mut present = vec![0usize; nret as usize + 1];
                    for (_, r) in &members {
                        if *r > 0 && (*r as usize) <= nret as usize {
                            present[*r as usize] += 1;
                        }
                    }

                    let mut group_missing = false;
                    for r in 1..=nret {
                        if present[r as usize] == 0 {
                            missing_points += 1;
                            *missing_by_rn.entry((r, nret)).or_insert(0) += 1;
                            group_missing = true;
                        }
                    }
                    if group_missing {
                        for (idx, _) in &members {
                            is_missing[*idx] = true;
                        }
                    }

                    for r in 1..=nret {
                        let c = present[r as usize];
                        if c > 1 {
                            duplicate_points += c - 1;
                            *duplicate_by_rn.entry((r, nret)).or_insert(0) += c - 1;
                            for (idx, rr) in &members {
                                if *rr == r {
                                    is_duplicate[*idx] = true;
                                }
                            }
                        }
                    }
                }

                let total = cloud.points.len().max(1);
                let mut report = String::new();
                report.push_str("LiDAR Point Return Analysis\n\n");
                report.push_str(&format!("input: {}\n", input_path));
                report.push_str(&format!(
                    "Missing Returns: {} ({:.3} percent)\n",
                    missing_points,
                    100.0 * missing_points as f64 / total as f64
                ));
                if !missing_by_rn.is_empty() {
                    report.push_str("\n| r | n | Missing Pts |\n|---|---|-------------|\n");
                    for ((r, n), c) in &missing_by_rn {
                        report.push_str(&format!("| {} | {} | {} |\n", r, n, c));
                    }
                }
                report.push_str(&format!(
                    "\nDuplicate Returns: {} ({:.3} percent)\n",
                    duplicate_points,
                    100.0 * duplicate_points as f64 / total as f64
                ));
                if !duplicate_by_rn.is_empty() {
                    report.push_str("\n| r | n | Duplicates |\n|---|---|------------|\n");
                    for ((r, n), c) in &duplicate_by_rn {
                        report.push_str(&format!("| {} | {} | {} |\n", r, n, c));
                    }
                }
                report.push_str(&format!(
                    "\nReturn Greater Than Num. Returns: {} ({:.3} percent)\n",
                    r_greater_n,
                    100.0 * r_greater_n as f64 / total as f64
                ));

                let report_out = report_path
                    .unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "point_return_report", "txt"));
                if let Some(parent) = report_out.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)
                            .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
                    }
                }
                fs::write(&report_out, report)
                    .map_err(|e| ToolError::Execution(format!("failed writing report '{}': {e}", report_out.to_string_lossy())))?;

                let mut outputs = BTreeMap::new();
                outputs.insert(
                    "report_path".to_string(),
                    json!(report_out.to_string_lossy().to_string()),
                );

                if create_output {
                    let mut out_cloud = cloud.clone();
                    for (i, p) in out_cloud.points.iter_mut().enumerate() {
                        p.classification = match (is_missing[i], is_duplicate[i]) {
                            (true, true) => 15,
                            (true, false) => 13,
                            (false, true) => 14,
                            (false, false) => 1,
                        };
                    }
                    let out = output_path
                        .unwrap_or_else(|| default_output_sibling_path(Path::new(&input_path), "return_qc", "las"));
                    if let Some(parent) = out.parent() {
                        if !parent.as_os_str().is_empty() {
                            fs::create_dir_all(parent)
                                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
                        }
                    }
                    out_cloud.write(&out).map_err(|e| {
                        ToolError::Execution(format!("failed writing output lidar '{}': {e}", out.to_string_lossy()))
                    })?;
                    outputs.insert("output".to_string(), json!(out.to_string_lossy().to_string()));
                }

                ctx.progress.progress(1.0);
                Ok(ToolRunResult { outputs })
            }
        }

impl Tool for LasToShapefileTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "las_to_shapefile",
            display_name: "LAS to Shapefile",
            summary: "Converts LAS/LAZ point clouds into vector point shapefiles.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "output", description: "Optional output vector path for single-input mode.", required: false },
                ToolParamSpec { name: "output_multipoint", description: "If true outputs a multipoint geometry with one feature; otherwise outputs one point feature per point.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
        }
        let _ = parse_bool_alias(args, &["output_multipoint"], false);
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_multipoint = parse_bool_alias(args, &["output_multipoint"], false);

        let run_single = |in_path: &Path, out_path: Option<&Path>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy()))
            })?;

            let mut layer = if output_multipoint {
                wbvector::Layer::new("las_to_multipoint").with_geom_type(wbvector::GeometryType::MultiPoint)
            } else {
                wbvector::Layer::new("las_to_points").with_geom_type(wbvector::GeometryType::Point)
            };
            layer.crs = lidar_crs_to_vector_crs(cloud.crs.as_ref());
            layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));

            if output_multipoint {
                let coords: Vec<wbvector::Coord> = cloud
                    .points
                    .iter()
                    .map(|p| wbvector::Coord::xyz(p.x, p.y, p.z))
                    .collect();
                if coords.is_empty() {
                    return Err(ToolError::Execution("input lidar has no points".to_string()));
                }
                layer
                    .add_feature(
                        Some(wbvector::Geometry::MultiPoint(coords)),
                        &[("FID", wbvector::FieldValue::Integer(1))],
                    )
                    .map_err(|e| ToolError::Execution(format!("failed creating multipoint feature: {}", e)))?;
            } else {
                layer.add_field(wbvector::FieldDef::new("Z", wbvector::FieldType::Float));
                layer.add_field(wbvector::FieldDef::new("INTENSITY", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("CLASS", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("RTN_NUM", wbvector::FieldType::Integer));
                layer.add_field(wbvector::FieldDef::new("NUM_RTNS", wbvector::FieldType::Integer));

                for (i, p) in cloud.points.iter().enumerate() {
                    layer
                        .add_feature(
                            Some(wbvector::Geometry::point(p.x, p.y)),
                            &[
                                ("FID", wbvector::FieldValue::Integer((i + 1) as i64)),
                                ("Z", wbvector::FieldValue::Float(p.z)),
                                (
                                    "INTENSITY",
                                    wbvector::FieldValue::Integer(i64::from(p.intensity)),
                                ),
                                (
                                    "CLASS",
                                    wbvector::FieldValue::Integer(i64::from(p.classification)),
                                ),
                                (
                                    "RTN_NUM",
                                    wbvector::FieldValue::Integer(i64::from(p.return_number)),
                                ),
                                (
                                    "NUM_RTNS",
                                    wbvector::FieldValue::Integer(i64::from(p.number_of_returns)),
                                ),
                            ],
                        )
                        .map_err(|e| ToolError::Execution(format!("failed creating point feature: {}", e)))?;
                }
            }

            let out = out_path
                .map(Path::to_path_buf)
                .unwrap_or_else(|| default_output_sibling_path(in_path, "points", "shp"));
            write_vector_output(&layer, out.to_string_lossy().as_ref())
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("converting lidar points to vector output");
            let out = run_single(Path::new(&input_path), output_path.as_deref())?;
            ctx.progress.progress(1.0);
            Ok(build_vector_result(out))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| run_single(&input, None))
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_vector_result(outputs)
        }
    }
}

impl Tool for FlightlineOverlapTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "flightline_overlap",
            display_name: "Flightline Overlap",
            summary: "Counts distinct point-source IDs per raster cell to identify overlapping flightlines.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "resolution", description: "Grid resolution used to count distinct flightlines per cell.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "grid_res", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation(
                "resolution/grid_res/cell_size must be a positive finite value".to_string(),
            ));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let resolution = parse_f64_alias(args, &["resolution", "grid_res", "cell_size"], 1.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let run_single = |in_path: &Path, out_path: Option<&Path>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!(
                    "failed reading input lidar '{}': {e}",
                    in_path.to_string_lossy()
                ))
            })?;

            let active_points: Vec<&PointRecord> = cloud
                .points
                .iter()
                .filter(|p| !point_is_withheld(p))
                .collect();

            let raster = if active_points.is_empty() {
                Raster::new(RasterConfig {
                    cols: 1,
                    rows: 1,
                    bands: 1,
                    x_min: 0.0,
                    y_min: 0.0,
                    cell_size: resolution,
                    cell_size_y: Some(resolution),
                    nodata: -32768.0,
                    data_type: DataType::F32,
                    crs: lidar_crs_to_raster_crs(cloud.crs.as_ref()),
                    metadata: vec![("tool".to_string(), "flightline_overlap".to_string())],
                })
            } else {
                let min_x = active_points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                let max_x = active_points
                    .iter()
                    .map(|p| p.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                let min_y = active_points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                let max_y = active_points
                    .iter()
                    .map(|p| p.y)
                    .fold(f64::NEG_INFINITY, f64::max);

                let cols = (((max_x - min_x) / resolution).ceil() as usize).max(1);
                let rows = (((max_y - min_y) / resolution).ceil() as usize).max(1);
                let y_min = max_y - rows as f64 * resolution;

                let mut cells: HashMap<(usize, usize), HashSet<u16>> = HashMap::new();
                for p in &active_points {
                    let col = (((p.x - min_x) / resolution).floor() as isize)
                        .clamp(0, cols.saturating_sub(1) as isize) as usize;
                    let row = (((max_y - p.y) / resolution).floor() as isize)
                        .clamp(0, rows.saturating_sub(1) as isize) as usize;
                    cells.entry((row, col)).or_default().insert(p.point_source_id);
                }

                let mut raster = Raster::new(RasterConfig {
                    cols,
                    rows,
                    bands: 1,
                    x_min: min_x,
                    y_min,
                    cell_size: resolution,
                    cell_size_y: Some(resolution),
                    nodata: -32768.0,
                    data_type: DataType::F32,
                    crs: lidar_crs_to_raster_crs(cloud.crs.as_ref()),
                    metadata: vec![("tool".to_string(), "flightline_overlap".to_string())],
                });
                for ((row, col), ids) in cells {
                    raster
                        .set(0, row as isize, col as isize, ids.len() as f64)
                        .map_err(|e| ToolError::Execution(format!("failed populating output raster: {e}")))?;
                }
                raster
            };

            let out = out_path
                .map(Path::to_path_buf)
                .unwrap_or_else(|| default_output_sibling_path(in_path, "flightline_overlap", "tif"));
            store_or_write_output(raster, Some(out))
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("counting flightline overlap by raster cell");
            let out = run_single(Path::new(&input_path), output_path.as_deref())?;
            ctx.progress.progress(1.0);
            Ok(build_raster_result(out))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| {
                    let out = generate_batch_output_path(&input, "flightline_overlap");
                    run_single(&input, Some(out.as_path()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            build_batch_placeholder_raster_result(outputs)
        }
    }
}

impl Tool for RecoverFlightlineInfoTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "recover_flightline_info",
            display_name: "Recover Flightline Info",
            summary: "Infers flightlines from GPS-time gaps and writes identifiers to point source ID, user data, and/or RGB.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "max_time_diff", description: "Maximum within-flightline GPS time gap before starting a new flightline.", required: false },
                ToolParamSpec { name: "pt_src_id", description: "If true, write inferred flightline IDs to point_source_id.", required: false },
                ToolParamSpec { name: "user_data", description: "If true, write inferred flightline IDs to user_data (modulo 256).", required: false },
                ToolParamSpec { name: "rgb", description: "If true, assign a random colour per inferred flightline.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let max_time_diff = parse_f64_alias(args, &["max_time_diff"], 5.0);
        if !max_time_diff.is_finite() || max_time_diff <= 0.0 {
            return Err(ToolError::Validation(
                "max_time_diff must be a positive finite value".to_string(),
            ));
        }
        let _ = parse_bool_alias(args, &["pt_src_id"], false);
        let _ = parse_bool_alias(args, &["user_data"], false);
        let _ = parse_bool_alias(args, &["rgb"], false);
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let max_time_diff = parse_f64_alias(args, &["max_time_diff"], 5.0);
        let pt_src_id = parse_bool_alias(args, &["pt_src_id"], false);
        let user_data = parse_bool_alias(args, &["user_data"], false);
        let mut rgb = parse_bool_alias(args, &["rgb"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("recovering flightline identifiers from gps time");
        let cloud = PointCloud::read(Path::new(&input_path)).map_err(|e| {
            ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
        })?;
        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "recover_flightline_info")?;
            ctx.progress.progress(1.0);
            return Ok(build_lidar_result(locator));
        }
        if cloud.points.iter().any(|p| p.gps_time.is_none()) {
            return Err(ToolError::Validation(
                "recover_flightline_info requires GPS time data for all points".to_string(),
            ));
        }
        if !pt_src_id && !user_data && !rgb {
            rgb = true;
        }

        let mut order: Vec<usize> = (0..cloud.points.len()).collect();
        order.sort_unstable_by(|a, b| {
            let pa = cloud.points[*a];
            let pb = cloud.points[*b];
            pa.gps_time
                .map(|t| t.0)
                .unwrap_or(0.0)
                .partial_cmp(&pb.gps_time.map(|t| t.0).unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
                .then_with(|| (pa.flags & 0b0000_0011).cmp(&(pb.flags & 0b0000_0011)))
                .then_with(|| pa.number_of_returns.cmp(&pb.number_of_returns))
                .then_with(|| pa.return_number.cmp(&pb.return_number))
                .then_with(|| a.cmp(b))
        });

        let mut rng = rand::rngs::StdRng::seed_from_u64(41);
        let mut next_colour = || Rgb16 {
            red: rng.random_range(0..=u16::MAX),
            green: rng.random_range(0..=u16::MAX),
            blue: rng.random_range(0..=u16::MAX),
        };

        let mut out_points = Vec::with_capacity(cloud.points.len());
        let mut prev_time = cloud.points[order[0]].gps_time.map(|t| t.0).unwrap_or(0.0);
        let mut flightline_id: u32 = 0;
        let mut colour = next_colour();

        for (rank, idx) in order.into_iter().enumerate() {
            let mut point = cloud.points[idx];
            let gps_time = point.gps_time.map(|t| t.0).unwrap_or(0.0);
            if rank > 0 && gps_time - prev_time > max_time_diff {
                flightline_id = flightline_id.saturating_add(1);
                if rgb {
                    colour = next_colour();
                }
            }
            prev_time = gps_time;

            if pt_src_id {
                point.point_source_id = flightline_id.min(u32::from(u16::MAX)) as u16;
            }
            if user_data {
                point.user_data = (flightline_id % 256) as u8;
            }
            if rgb {
                point.color = Some(colour);
            }
            out_points.push(point);
        }

        let out_cloud = PointCloud {
            points: out_points,
            crs: cloud.crs.clone(),
        };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "recover_flightline_info")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for FindFlightlineEdgePointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "find_flightline_edge_points",
            display_name: "Find Flightline Edge Points",
            summary: "Extracts only points flagged as edge-of-flightline.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("extracting flightline edge points");
        let cloud = PointCloud::read(Path::new(&input_path)).map_err(|e| {
            ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
        })?;
        let out_cloud = PointCloud {
            points: cloud
                .points
                .iter()
                .copied()
                .filter(|p| p.edge_of_flight_line)
                .collect(),
            crs: cloud.crs.clone(),
        };

        let locator = store_or_write_lidar_output(&out_cloud, output_path, "find_flightline_edge_points")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarTophatTransformTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_tophat_transform",
            display_name: "LiDAR Tophat Transform",
            summary: "Applies a white top-hat transform to LiDAR elevations to approximate height above local ground.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius used for erosion and dilation.", required: true },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let radius = parse_f64_alias(args, &["search_radius", "radius"], f64::NAN);
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 1.0);
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("performing lidar tophat transform");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;

        let mut tree = KdTree::new(2);
        for (i, p) in cloud.points.iter().enumerate() {
            if point_is_noise(p) {
                continue;
            }
            tree.add([p.x, p.y], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
        }
        let radius_sq = search_radius * search_radius;

        let mut min_surface = vec![0.0; cloud.points.len()];
        for (i, p) in cloud.points.iter().enumerate() {
            let neighbours = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
            let local_min = neighbours
                .iter()
                .map(|(_, idx)| cloud.points[**idx].z)
                .fold(f64::INFINITY, f64::min);
            min_surface[i] = if local_min.is_finite() { local_min } else { p.z };
        }

        let mut points = cloud.points.clone();
        for (i, p) in cloud.points.iter().enumerate() {
            let neighbours = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
            let opened = neighbours
                .iter()
                .map(|(_, idx)| min_surface[**idx])
                .fold(f64::NEG_INFINITY, f64::max);
            points[i].z = p.z - if opened.is_finite() { opened } else { p.z };
        }

        let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_tophat_transform")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for NormalVectorsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "normal_vectors",
            display_name: "Normal Vectors",
            summary: "Estimates local point-cloud normals and stores them in point normals and RGB values.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for local plane fitting. Values <= 0 use an estimated nominal spacing.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let radius = parse_f64_alias(args, &["search_radius", "radius"], -1.0);
        if !radius.is_finite() {
            return Err(ToolError::Validation("search_radius/radius must be finite".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("calculating lidar normal vectors");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        let mut search_radius = parse_f64_alias(args, &["search_radius", "radius"], -1.0);
        if search_radius <= 0.0 {
            search_radius = estimate_nominal_spacing(&cloud) * 3.0;
        }

        let mut tree = KdTree::new(3);
        for (i, p) in cloud.points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
        }
        let radius_sq = search_radius * search_radius;

        let mut points = cloud.points.clone();
        for (i, p) in cloud.points.iter().enumerate() {
            let neighbours = tree.within(&[p.x, p.y, p.z], radius_sq, &squared_euclidean).unwrap_or_default();
            let sample: Vec<Vector3<f64>> = neighbours
                .iter()
                .map(|(_, idx)| point_to_vec3(&cloud.points[**idx]))
                .collect();
            if let Some((normal, _)) = plane_normal_and_centroid(&sample) {
                points[i].normal_x = Some(normal.x as f32);
                points[i].normal_y = Some(normal.y as f32);
                points[i].normal_z = Some(normal.z as f32);
                points[i].color = Some(rgb_from_unit_normal(normal));
            }
        }

        let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "normal_vectors")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarKappaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_kappa",
            display_name: "LiDAR Kappa",
            summary: "Computes a kappa agreement report between two classified LiDAR clouds and writes a class-agreement raster.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Classification LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "input2", description: "Reference LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "report", description: "Output HTML report path.", required: true },
                ToolParamSpec { name: "resolution", description: "Grid resolution for spatial class-agreement output.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
                ToolParamSpec { name: "output_class_accuracy", description: "Compatibility flag retained for legacy parity; the agreement raster is always produced.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input1", "input_lidar1", "classification"], "input1")?;
        let _ = parse_required_lidar_path_alias(args, &["input2", "input_lidar2", "reference"], "input2")?;
        let report = parse_optional_output_path(args, "report")?
            .ok_or_else(|| ToolError::Validation("report is required".to_string()))?;
        ensure_html_or_txt(&report)?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);
        if !resolution.is_finite() || resolution <= 0.0 {
            return Err(ToolError::Validation("resolution/cell_size must be a positive finite value".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_required_lidar_path_alias(args, &["input1", "input_lidar1", "classification"], "input1")?;
        let input2_path = parse_required_lidar_path_alias(args, &["input2", "input_lidar2", "reference"], "input2")?;
        let report_path = parse_optional_output_path(args, "report")?.unwrap();
        let output_path = parse_optional_output_path(args, "output")?;
        let resolution = parse_f64_alias(args, &["resolution", "cell_size"], 1.0);

        ctx.progress.info("computing lidar kappa agreement");
        let classification = PointCloud::read(Path::new(&input1_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input1 '{}': {e}", input1_path)))?;
        let reference = PointCloud::read(Path::new(&input2_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input2 '{}': {e}", input2_path)))?;

        let mut tree = KdTree::new(3);
        for (i, p) in reference.points.iter().enumerate() {
            tree.add([p.x, p.y, p.z], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing reference lidar: {e}")))?;
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for p in &classification.points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        let cols = (((max_x - min_x) / resolution).ceil() as usize).max(1);
        let rows = (((max_y - min_y) / resolution).ceil() as usize).max(1);
        let y_min = max_y - rows as f64 * resolution;
        let mut correct = vec![0usize; rows * cols];
        let mut total = vec![0usize; rows * cols];
        let mut error_matrix = vec![vec![0usize; 256]; 256];
        let mut active_class = [false; 256];

        for p in &classification.points {
            let nearest = tree.nearest(&[p.x, p.y, p.z], 1, &squared_euclidean)
                .map_err(|e| ToolError::Execution(format!("failed querying reference lidar: {e}")))?;
            if let Some((_, idx)) = nearest.first() {
                let ref_pt = reference.points[**idx];
                let c1 = p.classification as usize;
                let c2 = ref_pt.classification as usize;
                error_matrix[c1][c2] += 1;
                active_class[c1] = true;
                active_class[c2] = true;
                let col = (((p.x - min_x) / resolution).floor() as isize).clamp(0, cols.saturating_sub(1) as isize) as usize;
                let row = (((max_y - p.y) / resolution).floor() as isize).clamp(0, rows.saturating_sub(1) as isize) as usize;
                let index = row * cols + col;
                total[index] += 1;
                if c1 == c2 {
                    correct[index] += 1;
                }
            }
        }

        let mut raster = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: min_x,
            y_min,
            cell_size: resolution,
            cell_size_y: Some(resolution),
            nodata: -32768.0,
            data_type: DataType::F32,
            crs: lidar_crs_to_raster_crs(classification.crs.as_ref()),
            metadata: vec![("tool".to_string(), "lidar_kappa".to_string())],
        });
        for row in 0..rows {
            for col in 0..cols {
                let index = row * cols + col;
                if total[index] > 0 {
                    raster.set(0, row as isize, col as isize, 100.0 * correct[index] as f64 / total[index] as f64)
                        .map_err(|e| ToolError::Execution(format!("failed populating kappa raster: {e}")))?;
                }
            }
        }

        let mut agreements = 0usize;
        let mut total_points = 0usize;
        for a in 0..256 {
            agreements += error_matrix[a][a];
            total_points += error_matrix[a].iter().sum::<usize>();
        }
        let mut expected = 0.0;
        for a in 0..256 {
            let row_total: usize = error_matrix[a].iter().sum();
            let col_total: usize = (0..256).map(|b| error_matrix[b][a]).sum();
            expected += (row_total as f64 * col_total as f64) / (total_points.max(1) as f64);
        }
        let overall_accuracy = agreements as f64 / total_points.max(1) as f64;
        let kappa = if (total_points as f64 - expected).abs() <= 1.0e-12 {
            0.0
        } else {
            (agreements as f64 - expected) / (total_points as f64 - expected)
        };

        let mut html = String::new();
        html.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>LiDAR Kappa</title></head><body>");
        html.push_str("<h1>LiDAR Kappa Index of Agreement</h1>");
        html.push_str(&format!("<p><strong>Classification Data:</strong> {}</p>", input1_path));
        html.push_str(&format!("<p><strong>Reference Data:</strong> {}</p>", input2_path));
        html.push_str(&format!("<p><strong>Overall Accuracy:</strong> {:.4}</p>", overall_accuracy));
        html.push_str(&format!("<p><strong>Kappa:</strong> {:.4}</p>", kappa));
        html.push_str("<table border=\"1\" cellspacing=\"0\" cellpadding=\"4\"><tr><th>Class</th><th>User's Accuracy</th><th>Producer's Accuracy</th></tr>");
        for class_id in 0..256usize {
            if !active_class[class_id] {
                continue;
            }
            let row_total: usize = error_matrix[class_id].iter().sum();
            let col_total: usize = (0..256).map(|b| error_matrix[b][class_id]).sum();
            let users = if row_total > 0 { error_matrix[class_id][class_id] as f64 / row_total as f64 } else { 0.0 };
            let producers = if col_total > 0 { error_matrix[class_id][class_id] as f64 / col_total as f64 } else { 0.0 };
            html.push_str(&format!(
                "<tr><td>{}</td><td>{:.4}</td><td>{:.4}</td></tr>",
                class_id, users, producers
            ));
        }
        html.push_str("</table></body></html>");
        if let Some(parent) = report_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating report directory: {e}")))?;
            }
        }
        fs::write(&report_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing report '{}': {e}", report_path.to_string_lossy())))?;

        let locator = store_or_write_output(
            raster,
            Some(output_path.unwrap_or_else(|| default_output_sibling_path(Path::new(&input1_path), "kappa", "tif"))),
        )?;
        let mut result = build_raster_result(locator);
        result.outputs.insert("report_path".to_string(), json!(report_path.to_string_lossy().to_string()));
        ctx.progress.progress(1.0);
        Ok(result)
    }
}

impl Tool for LidarEigenvalueFeaturesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_eigenvalue_features",
            display_name: "LiDAR Eigenvalue Features",
            summary: "Computes local PCA-based LiDAR neighbourhood features and writes a .eigen binary with JSON sidecar.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Optional input LiDAR path or typed LiDAR object. If omitted, runs in batch mode over LiDAR files in current directory.", required: false },
                ToolParamSpec { name: "num_neighbours", description: "Optional target neighbourhood size (excluding the point itself).", required: false },
                ToolParamSpec { name: "search_radius", description: "Optional maximum search radius for neighbourhood collection.", required: false },
                ToolParamSpec { name: "output", description: "Optional output .eigen path in single-input mode.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_lidar_path_arg_optional(args)?;
        let k = parse_f64_alias(args, &["num_neighbours", "num_neighbors"], -1.0);
        if k.is_finite() && k > 0.0 && k < 7.0 {
            return Err(ToolError::Validation("num_neighbours must be at least 7 when specified".to_string()));
        }
        let r = parse_f64_alias(args, &["search_radius", "radius"], f64::NAN);
        if r.is_finite() && r <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be positive when specified".to_string()));
        }
        if args.get("input").is_some() || args.get("input_lidar").is_some() {
            let _ = parse_optional_output_path(args, "output")?;
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_lidar_path_arg_optional(args)?;
        let k = parse_f64_alias(args, &["num_neighbours", "num_neighbors"], -1.0);
        let k = if k.is_finite() && k > 0.0 { Some(k as usize + 1) } else { None };
        let radius = parse_f64_alias(args, &["search_radius", "radius"], f64::NAN);
        let radius = if radius.is_finite() { Some(radius) } else { None };
        let output_path = parse_optional_output_path(args, "output")?;

        let run_single = |in_path: &Path, out_path: Option<&Path>| -> Result<String, ToolError> {
            let cloud = PointCloud::read(in_path).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", in_path.to_string_lossy()))
            })?;
            let mut tree = KdTree::new(3);
            for (i, p) in cloud.points.iter().enumerate() {
                tree.add([p.x, p.y, p.z], i)
                    .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
            }

            let out = out_path
                .map(Path::to_path_buf)
                .unwrap_or_else(|| in_path.with_extension("eigen"));
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)
                        .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
                }
            }
            let mut json_file = File::create(format!("{}.json", out.to_string_lossy()))
                .map_err(|e| ToolError::Execution(format!("failed creating eigen sidecar: {e}")))?;
            let schema = format!(
                "{{\n\"byte_order\": \"little\",\n\"field_descriptions\": [\n{{\"name\": \"point_num\", \"data_type\": \"u64\"}},\n{{\"name\": \"lambda1\", \"data_type\": \"f32\"}},\n{{\"name\": \"lambda2\", \"data_type\": \"f32\"}},\n{{\"name\": \"lambda3\", \"data_type\": \"f32\"}},\n{{\"name\": \"linearity\", \"data_type\": \"f32\"}},\n{{\"name\": \"planarity\", \"data_type\": \"f32\"}},\n{{\"name\": \"sphericity\", \"data_type\": \"f32\"}},\n{{\"name\": \"omnivariance\", \"data_type\": \"f32\"}},\n{{\"name\": \"eigentropy\", \"data_type\": \"f32\"}},\n{{\"name\": \"slope\", \"data_type\": \"f32\"}},\n{{\"name\": \"residual\", \"data_type\": \"f32\"}}\n],\n\"num_records\": {}\n}}",
                cloud.points.len()
            );
            json_file
                .write_all(schema.as_bytes())
                .map_err(|e| ToolError::Execution(format!("failed writing eigen sidecar: {e}")))?;

            let mut writer = BufWriter::new(File::create(&out)
                .map_err(|e| ToolError::Execution(format!("failed creating eigen output: {e}")))?);
            for (point_num, p) in cloud.points.iter().enumerate() {
                let sample_idx = if let Some(k) = k {
                    tree.nearest(&[p.x, p.y, p.z], k, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("failed querying neighbours: {e}")))?
                        .into_iter()
                        .filter_map(|(dist, idx)| {
                            if let Some(r) = radius {
                                if dist > r * r {
                                    return None;
                                }
                            }
                            Some(*idx)
                        })
                        .collect::<Vec<_>>()
                } else {
                    let r = radius.unwrap_or_else(|| estimate_nominal_spacing(&cloud) * 3.0);
                    tree.within(&[p.x, p.y, p.z], r * r, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("failed querying neighbours: {e}")))?
                        .into_iter()
                        .map(|(_, idx)| *idx)
                        .collect::<Vec<_>>()
                };
                let sample: Vec<Vector3<f64>> = sample_idx.iter().map(|idx| point_to_vec3(&cloud.points[*idx])).collect();
                writer.write_all(&(point_num as u64).to_le_bytes())
                    .map_err(|e| ToolError::Execution(format!("failed writing eigen record: {e}")))?;
                if let Some(features) = neighborhood_pca(&sample, point_to_vec3(p)) {
                    let sum = (features.lambda1 + features.lambda2 + features.lambda3).max(f32::EPSILON);
                    let linearity = (features.lambda1 - features.lambda2) / features.lambda1.max(f32::EPSILON);
                    let planarity = (features.lambda2 - features.lambda3) / features.lambda1.max(f32::EPSILON);
                    let sphericity = features.lambda3 / features.lambda1.max(f32::EPSILON);
                    let omnivariance = (features.lambda1 * features.lambda2 * features.lambda3).max(0.0).cbrt();
                    let e1 = features.lambda1 / sum;
                    let e2 = features.lambda2 / sum;
                    let e3 = features.lambda3 / sum;
                    let eigentropy = -([e1, e2, e3]
                        .into_iter()
                        .filter(|v| *v > 0.0)
                        .map(|v| v * v.ln())
                        .sum::<f32>());
                    for value in [
                        features.lambda1,
                        features.lambda2,
                        features.lambda3,
                        linearity,
                        planarity,
                        sphericity,
                        omnivariance,
                        eigentropy,
                        features.slope,
                        features.residual,
                    ] {
                        writer.write_all(&value.to_le_bytes())
                            .map_err(|e| ToolError::Execution(format!("failed writing eigen record: {e}")))?;
                    }
                } else {
                    for _ in 0..10 {
                        writer.write_all(&0f32.to_le_bytes())
                            .map_err(|e| ToolError::Execution(format!("failed writing eigen record: {e}")))?;
                    }
                }
            }
            writer.flush().map_err(|e| ToolError::Execution(format!("failed flushing eigen output: {e}")))?;
            Ok(out.to_string_lossy().to_string())
        };

        if let Some(input_path) = input_path {
            ctx.progress.info("computing lidar eigenvalue features");
            let out = run_single(Path::new(&input_path), output_path.as_deref())?;
            ctx.progress.progress(1.0);
            Ok(build_string_output_result("output", out))
        } else {
            ctx.progress.info("batch mode: scanning working directory for lidar files");
            let files = find_lidar_files()?;
            let outputs = files
                .into_par_iter()
                .map(|input| run_single(&input, Some(input.with_extension("eigen").as_path())))
                .collect::<Result<Vec<_>, _>>()?;
            ctx.progress.progress(1.0);
            let mut sorted = outputs;
            sorted.sort();
            Ok(build_string_output_result("output", sorted[0].clone()))
        }
    }
}

impl Tool for LidarRansacPlanesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_ransac_planes",
            display_name: "LiDAR RANSAC Planes",
            summary: "Identifies locally planar LiDAR points using neighbourhood RANSAC plane fitting.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path or typed LiDAR object.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for local plane fitting.", required: false },
                ToolParamSpec { name: "num_iterations", description: "Number of RANSAC iterations per point.", required: false },
                ToolParamSpec { name: "num_samples", description: "Number of sampled neighbour points per RANSAC iteration.", required: false },
                ToolParamSpec { name: "inlier_threshold", description: "Maximum point-to-plane residual for inliers.", required: false },
                ToolParamSpec { name: "acceptable_model_size", description: "Minimum number of inlier points required for a planar model.", required: false },
                ToolParamSpec { name: "max_planar_slope", description: "Maximum accepted plane slope in degrees.", required: false },
                ToolParamSpec { name: "classify", description: "If true classify planar vs non-planar points instead of filtering.", required: false },
                ToolParamSpec { name: "only_last_returns", description: "If true, only use late returns in model fitting.", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ToolError::Validation("search_radius/radius must be a positive finite value".to_string()));
        }
        let _ = parse_optional_lidar_output_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_required_lidar_path_alias(args, &["input", "input_lidar", "in_lidar"], "input")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0);
        let num_iterations = parse_f64_alias(args, &["num_iterations", "num_iter"], 50.0).max(1.0) as usize;
        let num_samples = parse_f64_alias(args, &["num_samples"], 10.0).max(3.0) as usize;
        let inlier_threshold = parse_f64_alias(args, &["inlier_threshold", "threshold"], 0.15).max(0.0);
        let acceptable_model_size = parse_f64_alias(args, &["acceptable_model_size", "model_size"], 30.0).max(5.0) as usize;
        let max_planar_slope = parse_f64_alias(args, &["max_planar_slope", "max_slope"], 75.0).clamp(0.0, 90.0);
        let classify = parse_bool_alias(args, &["classify"], false);
        let only_last_returns = parse_bool_alias(args, &["only_last_returns"], false);
        let output_path = parse_optional_lidar_output_path(args)?;

        ctx.progress.info("identifying planar lidar points with ransac");
        let cloud = PointCloud::read(Path::new(&input_path))
            .map_err(|e| ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path)))?;
        if cloud.points.is_empty() {
            let locator = store_or_write_lidar_output(&cloud, output_path, "lidar_ransac_planes")?;
            return Ok(build_lidar_result(locator));
        }

        let mut tree = KdTree::new(3);
        let mut active_indices = Vec::new();
        for (i, p) in cloud.points.iter().enumerate() {
            if point_is_noise(p) || point_is_withheld(p) || (only_last_returns && !is_late_return(p)) {
                continue;
            }
            tree.add([p.x, p.y, p.z], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing lidar points: {e}")))?;
            active_indices.push(i);
        }
        let radius_sq = search_radius * search_radius;
        let mut rng = rand::rng();
        let mut is_planar = vec![false; cloud.points.len()];

        for idx in active_indices {
            let center = point_to_vec3(&cloud.points[idx]);
            let neighbours = tree.within(&[center.x, center.y, center.z], radius_sq, &squared_euclidean).unwrap_or_default();
            if neighbours.len() < num_samples.max(acceptable_model_size) {
                continue;
            }
            let neighbour_points: Vec<(usize, Vector3<f64>)> = neighbours
                .iter()
                .map(|(_, nidx)| (**nidx, point_to_vec3(&cloud.points[**nidx])))
                .collect();
            let sample_indices: Vec<usize> = (0..neighbour_points.len()).collect();
            let mut best_plane = Plane::zero();
            let mut best_inliers = Vec::new();
            let mut best_rmse = f64::INFINITY;
            for _ in 0..num_iterations {
                let picks: Vec<usize> = sample_indices
                    .sample(&mut rng, num_samples.min(sample_indices.len()))
                    .copied()
                    .collect();
                let sample: Vec<Vector3<f64>> = picks.iter().map(|p| neighbour_points[*p].1).collect();
                let plane = Plane::from_points(&sample);
                if plane.slope() > max_planar_slope || plane.residual(&center) > inlier_threshold {
                    continue;
                }
                let inliers: Vec<usize> = neighbour_points
                    .iter()
                    .filter_map(|(pid, pt)| if plane.residual(pt) <= inlier_threshold { Some(*pid) } else { None })
                    .collect();
                if inliers.len() < acceptable_model_size {
                    continue;
                }
                let refined_points: Vec<Vector3<f64>> = inliers.iter().map(|pid| point_to_vec3(&cloud.points[*pid])).collect();
                let refined = Plane::from_points(&refined_points);
                let rmse = refined_points.iter().map(|pt| refined.residual(pt)).sum::<f64>() / refined_points.len() as f64;
                if rmse < best_rmse {
                    best_rmse = rmse;
                    best_plane = refined;
                    best_inliers = inliers;
                }
            }
            if best_rmse.is_finite() {
                for pid in best_inliers {
                    is_planar[pid] = true;
                }
                if best_plane.residual(&center) <= inlier_threshold {
                    is_planar[idx] = true;
                }
            }
        }

        let points = if classify {
            cloud.points.iter().enumerate().map(|(i, p)| {
                let mut q = *p;
                q.classification = if is_planar[i] { 0 } else { 1 };
                q
            }).collect::<Vec<_>>()
        } else {
            cloud.points.iter().enumerate().filter_map(|(i, p)| if is_planar[i] { Some(*p) } else { None }).collect::<Vec<_>>()
        };

        let out_cloud = PointCloud { points, crs: cloud.crs.clone() };
        let locator = store_or_write_lidar_output(&out_cloud, output_path, "lidar_ransac_planes")?;
        ctx.progress.progress(1.0);
        Ok(build_lidar_result(locator))
    }
}

impl Tool for LidarRooftopAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "lidar_rooftop_analysis",
            display_name: "LiDAR Rooftop Analysis",
            summary: "Identifies planar rooftop segments within building footprints and outputs segment polygons with roof attributes.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Input LiDAR paths or typed LiDAR objects.", required: true },
                ToolParamSpec { name: "building_footprints", description: "Building-footprint polygon layer.", required: true },
                ToolParamSpec { name: "search_radius", description: "Neighbourhood radius for local roof analysis.", required: false },
                ToolParamSpec { name: "inlier_threshold", description: "Maximum residual for local planar support.", required: false },
                ToolParamSpec { name: "acceptable_model_size", description: "Minimum segment size.", required: false },
                ToolParamSpec { name: "max_planar_slope", description: "Maximum slope for rooftop facets in degrees.", required: false },
                ToolParamSpec { name: "norm_diff_threshold", description: "Maximum angular difference between neighbouring normals in degrees.", required: false },
                ToolParamSpec { name: "azimuth", description: "Illumination azimuth for hillshade-style facet lighting.", required: false },
                ToolParamSpec { name: "altitude", description: "Illumination altitude for hillshade-style facet lighting.", required: false },
                ToolParamSpec { name: "output", description: "Optional output vector path.", required: false },
            ],
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_lidar_inputs_arg(args)?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("inputs is required".to_string()));
        }
        let _ = parse_required_vector_path_alias(args, &["building_footprints", "buildings"], "building_footprints")?;
        if let Some(path) = parse_optional_output_path(args, "output")? {
            let _ = detect_vector_output_format(path.to_string_lossy().as_ref())?;
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_lidar_inputs_arg(args)?;
        let buildings_path = parse_required_vector_path_alias(args, &["building_footprints", "buildings"], "building_footprints")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let search_radius = parse_f64_alias(args, &["search_radius", "radius"], 2.0).max(0.1);
        let inlier_threshold = parse_f64_alias(args, &["inlier_threshold", "threshold"], 0.15).max(0.0);
        let acceptable_model_size = parse_f64_alias(args, &["acceptable_model_size", "model_size"], 30.0).max(5.0) as usize;
        let max_planar_slope = parse_f64_alias(args, &["max_planar_slope", "max_slope"], 75.0).clamp(0.0, 90.0);
        let norm_diff_threshold = parse_f64_alias(args, &["norm_diff_threshold", "norm_diff"], 2.0).clamp(0.0, 90.0).to_radians();
        let azimuth = parse_f64_alias(args, &["azimuth"], 180.0).to_radians();
        let altitude = parse_f64_alias(args, &["altitude"], 30.0).to_radians();

        ctx.progress.info("analyzing rooftop facets within building footprints");
        let buildings = wbvector::read(&buildings_path)
            .map_err(|e| ToolError::Execution(format!("failed reading building footprints '{}': {e}", buildings_path)))?;

        let mut feature_polys: Vec<Vec<PreparedPolygon>> = Vec::new();
        for feature in &buildings.features {
            let mut polys = Vec::new();
            if let Some(geometry) = feature.geometry.as_ref() {
                collect_polygons_from_geometry(geometry, &mut polys);
            }
            feature_polys.push(polys);
        }

        let mut selected_points: Vec<PointRecord> = Vec::new();
        let mut building_ids: Vec<usize> = Vec::new();
        let mut crs = None;
        for input_path in &input_paths {
            let cloud = PointCloud::read(Path::new(input_path)).map_err(|e| {
                ToolError::Execution(format!("failed reading input lidar '{}': {e}", input_path))
            })?;
            if crs.is_none() {
                crs = cloud.crs.clone();
            }
            for p in &cloud.points {
                if point_is_withheld(p) || point_is_noise(p) || !is_late_return(p) {
                    continue;
                }
                if let Some((building_id, _)) = feature_polys.iter().enumerate().find(|(_, polys)| point_in_any_prepared_polygon(p.x, p.y, polys)) {
                    selected_points.push(*p);
                    building_ids.push(building_id);
                }
            }
        }

        let mut layer = wbvector::Layer::new("lidar_rooftop_analysis").with_geom_type(wbvector::GeometryType::Polygon);
        layer.crs = lidar_crs_to_vector_crs(crs.as_ref());
        layer.add_field(wbvector::FieldDef::new("FID", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("BUILDING", wbvector::FieldType::Integer));
        layer.add_field(wbvector::FieldDef::new("MAX_ELEV", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("HILLSHADE", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("SLOPE", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("ASPECT", wbvector::FieldType::Float));
        layer.add_field(wbvector::FieldDef::new("AREA", wbvector::FieldType::Float));

        if selected_points.is_empty() {
            let out = output_path.unwrap_or_else(|| default_output_sibling_path(Path::new(&input_paths[0]), "rooftops", "shp"));
            let out_path = write_vector_output(&layer, out.to_string_lossy().as_ref())?;
            ctx.progress.progress(1.0);
            return Ok(build_vector_result(out_path));
        }

        let mut tree = KdTree::new(2);
        for (i, p) in selected_points.iter().enumerate() {
            tree.add([p.x, p.y], i)
                .map_err(|e| ToolError::Execution(format!("failed indexing rooftop points: {e}")))?;
        }
        let radius_sq = search_radius * search_radius;
        let mut local_pca: Vec<Option<NeighborhoodPca>> = vec![None; selected_points.len()];
        let mut planar = vec![false; selected_points.len()];
        for (i, p) in selected_points.iter().enumerate() {
            let neighbours = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
            let sample: Vec<Vector3<f64>> = neighbours
                .iter()
                .filter_map(|(_, idx)| {
                    if building_ids[**idx] != building_ids[i] {
                        None
                    } else {
                        Some(point_to_vec3(&selected_points[**idx]))
                    }
                })
                .collect();
            local_pca[i] = neighborhood_pca(&sample, point_to_vec3(p));
            if let Some(features) = local_pca[i] {
                planar[i] = f64::from(features.residual) <= inlier_threshold && f64::from(features.slope) <= max_planar_slope;
            }
        }

        let mut segment_id = vec![0usize; selected_points.len()];
        let mut current_segment = 0usize;
        for i in 0..selected_points.len() {
            if !planar[i] || segment_id[i] != 0 {
                continue;
            }
            current_segment += 1;
            segment_id[i] = current_segment;
            let mut stack = vec![i];
            while let Some(idx) = stack.pop() {
                let p = selected_points[idx];
                let Some(pca0) = local_pca[idx] else { continue; };
                let neighbours = tree.within(&[p.x, p.y], radius_sq, &squared_euclidean).unwrap_or_default();
                for (_, nref) in neighbours {
                    let nidx = *nref;
                    if segment_id[nidx] != 0 || !planar[nidx] || building_ids[nidx] != building_ids[idx] {
                        continue;
                    }
                    let Some(pca1) = local_pca[nidx] else { continue; };
                    let angle = pca0.normal.dot(&pca1.normal).clamp(-1.0, 1.0).abs().acos();
                    if angle <= norm_diff_threshold && (selected_points[nidx].z - selected_points[idx].z).abs() <= 1.0 {
                        segment_id[nidx] = current_segment;
                        stack.push(nidx);
                    }
                }
            }
        }

        let mut segment_points: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, sid) in segment_id.iter().enumerate() {
            if *sid > 0 {
                segment_points.entry(*sid).or_default().push(i);
            }
        }

        let mut fid = 1i64;
        for point_ids in segment_points.values() {
            if point_ids.len() < acceptable_model_size {
                continue;
            }
            let building_id = building_ids[point_ids[0]];
            let data: Vec<Vector3<f64>> = point_ids.iter().map(|idx| point_to_vec3(&selected_points[*idx])).collect();
            let plane = Plane::from_points(&data);
            if plane.slope() > max_planar_slope {
                continue;
            }
            let coords: Vec<(f64, f64)> = point_ids
                .iter()
                .map(|idx| (selected_points[*idx].x, selected_points[*idx].y))
                .collect();
            let hull = close_ring(
                convex_hull_2d(&coords)
                    .into_iter()
                    .map(|(x, y)| wbvector::Coord::xy(x, y))
                    .collect(),
            );
            if hull.len() < 4 {
                continue;
            }
            let max_elev = point_ids.iter().map(|idx| selected_points[*idx].z).fold(f64::NEG_INFINITY, f64::max);
            let slope = plane.slope();
            let fx = if plane.c.abs() > 1.0e-12 { -plane.a / plane.c } else { 0.0 };
            let fy = if plane.c.abs() > 1.0e-12 { -plane.b / plane.c } else { 0.0 };
            let aspect = (180.0 - fy.atan2(fx).to_degrees() + 90.0).rem_euclid(360.0);
            let slope_rad = slope.to_radians();
            let aspect_rad = aspect.to_radians();
            let hillshade = 255.0 * (altitude.sin() * slope_rad.cos() + altitude.cos() * slope_rad.sin() * (azimuth - aspect_rad).cos()).max(0.0);
            let area = polygon_area(&hull);
            layer.add_feature(
                Some(wbvector::Geometry::polygon(hull, vec![])),
                &[
                    ("FID", wbvector::FieldValue::Integer(fid)),
                    ("BUILDING", wbvector::FieldValue::Integer(building_id as i64 + 1)),
                    ("MAX_ELEV", wbvector::FieldValue::Float(max_elev)),
                    ("HILLSHADE", wbvector::FieldValue::Float(hillshade)),
                    ("SLOPE", wbvector::FieldValue::Float(slope)),
                    ("ASPECT", wbvector::FieldValue::Float(aspect)),
                    ("AREA", wbvector::FieldValue::Float(area)),
                ],
            ).map_err(|e| ToolError::Execution(format!("failed creating rooftop feature: {e}")))?;
            fid += 1;
        }

        let out = output_path.unwrap_or_else(|| default_output_sibling_path(Path::new(&input_paths[0]), "rooftops", "shp"));
        let out_path = write_vector_output(&layer, out.to_string_lossy().as_ref())?;
        ctx.progress.progress(1.0);
        Ok(build_vector_result(out_path))
    }
}

#[cfg(test)]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lidar_crs_prefers_epsg_when_available() {
        let crs = LidarCrs {
            epsg: Some(4326),
            wkt: Some("GEOGCS[\"WGS 84\"]".to_string()),
        };
        let out = lidar_crs_to_raster_crs(Some(&crs));
        assert_eq!(out.epsg, Some(4326));
    }

    #[test]
    fn lidar_crs_uses_wkt_when_epsg_missing() {
        let crs = LidarCrs {
            epsg: None,
            wkt: Some("GEOGCS[\"WGS 84\"]".to_string()),
        };
        let out = lidar_crs_to_raster_crs(Some(&crs));
        assert_eq!(out.epsg, None);
        assert!(out.wkt.is_some());
    }

    #[test]
    fn require_lidar_crs_rejects_missing_metadata() {
        let err = require_lidar_crs(None, "lidar_idw_interpolation").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("requires input LiDAR CRS metadata"));
    }
}
