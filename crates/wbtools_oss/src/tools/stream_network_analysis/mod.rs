/// Stream network analysis tools
///
/// This module contains tools for analyzing stream networks, including:
/// - Strahler, Horton, Hack, and other stream ordering systems
/// - Stream magnitude calculations
/// - Stream link identification and analysis
/// - Flow accumulation-based stream extraction
/// - Valley extraction
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterFormat};
use wbvector::{Coord, FieldDef, FieldType, FieldValue, Geometry, GeometryType, Layer, VectorFormat};

use crate::memory_store;
use super::flow_algorithms::{D8FlowAccumTool, D8PointerTool};
mod pro_stream_tools;
pub use pro_stream_tools::{PruneVectorStreamsTool, RiverCenterlinesTool, RidgeAndValleyVectorsTool};

// ──────────────────────────────────────────────────────────────────────────────
// Tool Structs
// ──────────────────────────────────────────────────────────────────────────────

pub struct StrahlerStreamOrderTool;
pub struct HortonStreamOrderTool;
pub struct BurnStreamsTool;
pub struct HortonRatiosTool;
pub struct HackStreamOrderTool;
pub struct ShreveStreamMagnitudeTool;
pub struct TopologicalStreamOrderTool;
pub struct StreamLinkIdentifierTool;
pub struct StreamLinkClassTool;
pub struct StreamLinkLengthTool;
pub struct StreamLinkSlopeTool;
pub struct StreamSlopeContinuousTool;
pub struct DistanceToOutletTool;
pub struct LengthOfUpstreamChannelsTool;
pub struct FindMainStemTool;
pub struct FarthestChannelHeadTool;
pub struct TributaryIdentifierTool;
pub struct RemoveShortStreamsTool;
pub struct ExtractStreamsTool;
pub struct ExtractValleysTool;
pub struct RasterStreamsToVectorTool;
pub struct RasterizeStreamsTool;
pub struct LongProfileTool;
pub struct LongProfileFromPointsTool;
pub struct RepairStreamVectorTopologyTool;
pub struct VectorStreamNetworkAnalysisTool;

// ──────────────────────────────────────────────────────────────────────────────
// Core D8 Utilities
// ──────────────────────────────────────────────────────────────────────────────

struct D8Core;

impl D8Core {
    /// D8 direction offsets (column and row)
    const D_X: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
    const D_Y: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

    /// Whitebox-style inflowing cell pointer values
    const WB_INFLOWING_VALS: [f64; 8] = [16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0, 8.0];
    /// ESRI-style inflowing cell pointer values
    const ESRI_INFLOWING_VALS: [f64; 8] = [8.0, 16.0, 32.0, 64.0, 128.0, 1.0, 2.0, 4.0];

    /// Build pntr_matches lookup table for converting pointer values to direction indices
    fn build_pntr_matches(esri_style: bool) -> [usize; 129] {
        let mut pntr_matches = [999usize; 129];
        if !esri_style {
            pntr_matches[1] = 0;
            pntr_matches[2] = 1;
            pntr_matches[4] = 2;
            pntr_matches[8] = 3;
            pntr_matches[16] = 4;
            pntr_matches[32] = 5;
            pntr_matches[64] = 6;
            pntr_matches[128] = 7;
        } else {
            pntr_matches[1] = 1;
            pntr_matches[2] = 2;
            pntr_matches[4] = 3;
            pntr_matches[8] = 4;
            pntr_matches[16] = 5;
            pntr_matches[32] = 6;
            pntr_matches[64] = 7;
            pntr_matches[128] = 0;
        }
        pntr_matches
    }

    /// Get inflowing cell pointer values based on pointer style
    fn inflowing_vals(esri_style: bool) -> [f64; 8] {
        if esri_style {
            Self::ESRI_INFLOWING_VALS
        } else {
            Self::WB_INFLOWING_VALS
        }
    }

    /// Load a raster, handling memory store paths
    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!("unknown in-memory raster id '{}'", id))
            });
        }
        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    /// Store or write output raster
    fn write_or_store_output(
        output: Raster,
        output_path: Option<std::path::PathBuf>,
    ) -> Result<String, ToolError> {
        if let Some(output_path) = output_path {
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::Execution(format!("failed creating output directory: {e}"))
                    })?;
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

    /// Build tool result from output path
    fn build_result(output_locator: String) -> ToolRunResult {
        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    /// Count inflowing stream cells for headwater detection
    fn count_inflowing_cells(
        streams: &Raster,
        pntr: &Raster,
        row: isize,
        col: isize,
        inflowing_vals: &[f64; 8],
    ) -> i8 {
        let mut count = 0i8;
        for i in 0..8 {
            let y = row + Self::D_Y[i];
            let x = col + Self::D_X[i];
            if streams.get(0, y, x) > 0.0 && pntr.get(0, y, x) == inflowing_vals[i] {
                count += 1;
            }
        }
        count
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Strahler Stream Order Tool
// ──────────────────────────────────────────────────────────────────────────────

impl Tool for StrahlerStreamOrderTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "strahler_stream_order",
            display_name: "Strahler Stream Order",
            summary: "Assigns Strahler stream order to stream cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "d8_pntr",
                    description: "D8 flow pointer raster",
                    required: true,
                },
                ToolParamSpec {
                    name: "streams_raster",
                    description: "Stream raster (positive values = stream cells)",
                    required: true,
                },
                ToolParamSpec {
                    name: "esri_pntr",
                    description: "Use ESRI-style pointer values (default: Whitebox)",
                    required: false,
                },
                ToolParamSpec {
                    name: "zero_background",
                    description: "Assign zero to background instead of NoData",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Output raster path",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("d8_pntr".to_string(), json!("d8_pointer.tif"));
        defaults.insert("streams_raster".to_string(), json!("streams.tif"));
        defaults.insert("esri_pntr".to_string(), json!(false));
        defaults.insert("zero_background".to_string(), json!(false));

        ToolManifest {
            id: "strahler_stream_order".to_string(),
            display_name: "Strahler Stream Order".to_string(),
            summary: "Assigns Strahler stream order to stream cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "strahler_example".to_string(),
                description: "Compute Strahler stream order".to_string(),
                args: ToolArgs::new(),
            }],
            tags: vec![
                "stream_network".to_string(),
                "stream_order".to_string(),
                "hydrology".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "d8_pntr")?;
        parse_raster_path_arg(args, "streams_raster")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr")?;
        let streams_path = parse_raster_path_arg(args, "streams_raster")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);

        let pntr = D8Core::load_raster(&d8_pntr_path)?;
        let streams = D8Core::load_raster(&streams_path)?;

        if streams.rows != pntr.rows || streams.cols != pntr.cols {
            return Err(ToolError::Validation(
                "Input rasters must have the same dimensions".to_string(),
            ));
        }

        let rows = pntr.rows;
        let cols = pntr.cols;
        let nodata = streams.nodata;
        let pntr_nodata = pntr.nodata;
        let background_val = if zero_background { 0.0 } else { nodata };

        let mut output = streams.clone();
        output.data_type = DataType::I32;

        // Initialize: count inflowing cells for each stream cell
        let mut num_inflowing = vec![vec![-1i8; cols]; rows];
        let inflowing_vals = D8Core::inflowing_vals(esri_style);
        let mut stack = Vec::new();

        for row in 0..rows {
            for col in 0..cols {
                if streams.get(0, row as isize, col as isize) > 0.0 {
                    let count = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing_vals);
                    num_inflowing[row][col] = count;
                    if count == 0 {
                        stack.push((row, col));
                        output.set_unchecked(0, row as isize, col as isize, 1.0);
                    }
                } else {
                    let p = pntr.get(0, row as isize, col as isize);
                    if p != pntr_nodata {
                        output.set_unchecked(0, row as isize, col as isize, background_val);
                    } else {
                        output.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
        }

        // Build pntr_matches lookup
        let pntr_matches = D8Core::build_pntr_matches(esri_style);

        // Process stack (downstream traversal)
        while !stack.is_empty() {
            let (row, col) = stack.pop().unwrap();
            let order_val = output.get(0, row as isize, col as isize);

            // Find downstream cell
            let dir_val = pntr.get(0, row as isize, col as isize) as usize;
            if dir_val > 0 && dir_val <= 128 && pntr_matches[dir_val] != 999 {
                let dir_idx = pntr_matches[dir_val];
                let row_n = (row as isize + D8Core::D_Y[dir_idx]) as usize;
                let col_n = (col as isize + D8Core::D_X[dir_idx]) as usize;

                if row_n < rows && col_n < cols && streams.get(0, row_n as isize, col_n as isize) > 0.0 {
                    let order_val_n = output.get(0, row_n as isize, col_n as isize);
                    if order_val == order_val_n {
                        output.set_unchecked(0, row_n as isize, col_n as isize, order_val + 1.0);
                    } else if order_val > order_val_n {
                        output.set_unchecked(0, row_n as isize, col_n as isize, order_val);
                    }

                    num_inflowing[row_n][col_n] -= 1;
                    if num_inflowing[row_n][col_n] == 0 {
                        stack.push((row_n, col_n));
                    }
                }
            }

            ctx.progress.progress(((row * cols + col) as f64) / ((rows * cols) as f64));
        }

        Ok(D8Core::build_result(D8Core::write_or_store_output(output, output_path)?))
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Stream Tool Implementations
// ──────────────────────────────────────────────────────────────────────────────

// Remaining tools are routed through the shared stream-tool dispatcher so each
// tool id executes a concrete algorithmic path rather than a placeholder error.

fn detect_vector_format(path: &str) -> Result<VectorFormat, ToolError> {
    match VectorFormat::detect(path) {
        Ok(fmt) => Ok(fmt),
        Err(_) => {
            if Path::new(path).extension().is_none() {
                Ok(VectorFormat::Shapefile)
            } else {
                Err(ToolError::Validation(format!(
                    "could not determine vector output format from path '{}'",
                    path
                )))
            }
        }
    }
}

fn load_vector(path: &str) -> Result<Layer, ToolError> {
    wbvector::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading vector input: {}", e)))
}

fn write_vector(layer: &Layer, path: &str) -> Result<String, ToolError> {
    let fmt = detect_vector_format(path)?;
    wbvector::write(layer, path, fmt)
        .map_err(|e| ToolError::Execution(format!("failed writing vector output: {}", e)))?;
    Ok(path.to_string())
}

fn ensure_parent_dir(path: &str) -> Result<(), ToolError> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating output directory: {}", e)))?;
        }
    }
    Ok(())
}

fn point_to_row_col(raster: &Raster, x: f64, y: f64) -> Option<(isize, isize)> {
    if x < raster.x_min || x >= raster.x_max() || y < raster.y_min || y >= raster.y_max() {
        return None;
    }
    let col = ((x - raster.x_min) / raster.cell_size_x).floor() as isize;
    let row = ((y - raster.y_min) / raster.cell_size_y).floor() as isize;
    if row < 0 || col < 0 || row >= raster.rows as isize || col >= raster.cols as isize {
        None
    } else {
        Some((row, col))
    }
}

fn rasterize_segment(raster: &mut Raster, start: &Coord, end: &Coord, value: f64) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let sx = raster.cell_size_x.abs().max(f64::EPSILON);
    let sy = raster.cell_size_y.abs().max(f64::EPSILON);
    let steps = ((dx.abs() / sx).max(dy.abs() / sy).ceil() as usize)
        .saturating_mul(2)
        .max(1);
    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = start.x + t * dx;
        let y = start.y + t * dy;
        if let Some((row, col)) = point_to_row_col(raster, x, y) {
            raster.set_unchecked(0, row, col, value);
        }
    }
}

fn output_html_path(args: &ToolArgs) -> Result<String, ToolError> {
    args.get("output")
        .or_else(|| args.get("output_html_file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Validation("missing required parameter 'output' for HTML output".to_string()))
}

fn render_profile_html(title: &str, profiles: &[(Vec<f64>, Vec<f64>)]) -> String {
    let width = 900.0;
    let height = 560.0;
    let left = 70.0;
    let right = 20.0;
    let top = 30.0;
    let bottom = 55.0;
    let plot_w = width - left - right;
    let plot_h = height - top - bottom;

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (xs, ys) in profiles {
        for &x in xs {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
        for &y in ys {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        min_x = 0.0;
        max_x = 1.0;
        min_y = 0.0;
        max_y = 1.0;
    }
    if (max_x - min_x).abs() < f64::EPSILON {
        max_x = min_x + 1.0;
    }
    if (max_y - min_y).abs() < f64::EPSILON {
        max_y = min_y + 1.0;
    }

    let profile_color = |line_idx: usize, point_count: usize| -> String {
        // Generate a deterministic pseudo-random, high-saturation color per profile.
        let mut state = ((line_idx as u64 + 1) * 0x9E37_79B9_7F4A_7C15)
            ^ ((point_count as u64 + 1) * 0xBF58_476D_1CE4_E5B9);
        let mut next_unit = || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let r = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
            (r as f64) / (u64::MAX as f64)
        };

        let hue = next_unit() * 360.0;
        let sat = 0.75 + next_unit() * 0.22;
        let val = 0.82 + next_unit() * 0.16;

        let c = val * sat;
        let h_prime = hue / 60.0;
        let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
        let (r1, g1, b1) = if h_prime < 1.0 {
            (c, x, 0.0)
        } else if h_prime < 2.0 {
            (x, c, 0.0)
        } else if h_prime < 3.0 {
            (0.0, c, x)
        } else if h_prime < 4.0 {
            (0.0, x, c)
        } else if h_prime < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        let m = val - c;
        let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        format!("#{r:02X}{g:02X}{b:02X}")
    };
    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg viewBox=\"0 0 {width} {height}\" width=\"100%\" height=\"{height}\" xmlns=\"http://www.w3.org/2000/svg\">"
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#fffdf8\"/>");
    svg.push_str(&format!(
        "<line x1=\"{left}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.2\"/>",
        height - bottom,
        width - right,
        height - bottom
    ));
    svg.push_str(&format!(
        "<line x1=\"{left}\" y1=\"{top}\" x2=\"{left}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.2\"/>",
        height - bottom
    ));

    for tick in 0..=5 {
        let frac = tick as f64 / 5.0;
        let x = left + frac * plot_w;
        let y = top + frac * plot_h;
        let xv = min_x + frac * (max_x - min_x);
        let yv = max_y - frac * (max_y - min_y);
        svg.push_str(&format!(
            "<line x1=\"{x}\" y1=\"{top}\" x2=\"{x}\" y2=\"{}\" stroke=\"#e6dfd2\" stroke-width=\"1\"/>",
            height - bottom
        ));
        svg.push_str(&format!(
            "<line x1=\"{left}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" stroke=\"#e6dfd2\" stroke-width=\"1\"/>",
            width - right
        ));
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"{}\" font-size=\"12\" text-anchor=\"middle\" fill=\"#444\">{:.2}</text>",
            height - bottom + 20.0,
            xv
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"12\" text-anchor=\"end\" fill=\"#444\">{:.2}</text>",
            left - 8.0,
            y + 4.0,
            yv
        ));
    }

    for (idx, (xs, ys)) in profiles.iter().enumerate() {
        if xs.len() < 2 || xs.len() != ys.len() {
            continue;
        }
        let color = profile_color(idx, xs.len());
        let mut pts = String::new();
        for (&xv, &yv) in xs.iter().zip(ys.iter()) {
            let x = left + (xv - min_x) / (max_x - min_x) * plot_w;
            let y = top + (max_y - yv) / (max_y - min_y) * plot_h;
            pts.push_str(&format!("{:.2},{:.2} ", x, y));
        }
        svg.push_str(&format!(
            "<polyline fill=\"none\" stroke=\"{}\" stroke-width=\"2\" points=\"{}\"/>",
            color,
            pts.trim_end()
        ));
    }

    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"14\" text-anchor=\"middle\" fill=\"#222\">Distance from Mouth</text>",
        left + plot_w / 2.0,
        height - 12.0
    ));
    svg.push_str(&format!(
        "<text x=\"18\" y=\"{}\" font-size=\"14\" text-anchor=\"middle\" fill=\"#222\" transform=\"rotate(-90 18,{})\">Elevation</text>",
        top + plot_h / 2.0,
        top + plot_h / 2.0
    ));
    svg.push_str("</svg>");

    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title><style>body{{font-family:Georgia,serif;background:#f6f1e8;color:#1f1f1f;margin:0;padding:24px}}main{{max-width:980px;margin:0 auto}}h1{{font-weight:600;letter-spacing:.02em}}.card{{background:#fff;border:1px solid #e6dfd2;border-radius:12px;padding:16px;box-shadow:0 8px 24px rgba(0,0,0,.06)}}</style></head><body><main><h1>{}</h1><div class=\"card\">{}</div></main></body></html>",
        title, title, svg
    )
}

fn stream_heads(streams: &Raster, pntr: &Raster, esri_style: bool) -> Vec<(usize, usize)> {
    let inflowing = D8Core::inflowing_vals(esri_style);
    let mut heads = Vec::new();
    for row in 0..streams.rows {
        for col in 0..streams.cols {
            if streams.get(0, row as isize, col as isize) > 0.0
                && D8Core::count_inflowing_cells(streams, pntr, row as isize, col as isize, &inflowing) == 0
            {
                heads.push((row, col));
            }
        }
    }
    heads
}

fn sample_profile_from_start(
    start: (usize, usize),
    pntr: &Raster,
    dem: &Raster,
    stream_mask: Option<&Raster>,
    esri_style: bool,
) -> Result<(Vec<f64>, Vec<f64>), ToolError> {
    let pntr_matches = D8Core::build_pntr_matches(esri_style);
    let lengths = grid_lengths(pntr);
    let mut cells = vec![start];
    let mut dists = vec![0.0f64];
    let mut total = 0.0f64;
    let mut y = start.0;
    let mut x = start.1;
    while let Some((yn, xn, idx)) = downstream_cell(pntr, y, x, &pntr_matches) {
        if let Some(mask) = stream_mask {
            if mask.get(0, yn as isize, xn as isize) <= 0.0 {
                break;
            }
        }
        total += lengths[idx];
        cells.push((yn, xn));
        dists.push(total);
        y = yn;
        x = xn;
    }
    if cells.len() < 2 {
        return Err(ToolError::Execution("profile path contains fewer than two cells".to_string()));
    }
    let mut xs = Vec::with_capacity(cells.len());
    let mut ys = Vec::with_capacity(cells.len());
    for ((row, col), dist) in cells.into_iter().zip(dists.into_iter()) {
        xs.push((total - dist).max(0.0));
        ys.push(dem.get(0, row as isize, col as isize));
    }
    if let Some(last) = xs.last_mut() {
        if *last == 0.0 {
            *last = 1.0e-7;
        }
    }
    Ok((xs, ys))
}

fn line_geometries(layer: &Layer) -> Vec<Vec<Coord>> {
    let mut lines = Vec::new();
    for feat in &layer.features {
        if let Some(geom) = &feat.geometry {
            match geom {
                Geometry::LineString(coords) if coords.len() >= 2 => lines.push(coords.clone()),
                Geometry::MultiLineString(parts) => {
                    for part in parts {
                        if part.len() >= 2 {
                            lines.push(part.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    lines
}

fn coord_distance(a: &Coord, b: &Coord) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn endpoint_key(c: &Coord, tol: f64) -> (i64, i64) {
    let scale = tol.max(1.0e-9);
    ((c.x / scale).round() as i64, (c.y / scale).round() as i64)
}

fn collect_link_key_nodes(lines: &[Vec<Coord>], snap_dist: f64, precision_sq: f64) -> Vec<Vec<(i64, i64)>> {
    let mut endpoints = Vec::<(Coord, usize)>::new();
    for (i, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            continue;
        }
        endpoints.push((line[0].clone(), i));
        endpoints.push((line.last().unwrap().clone(), i));
    }

    let mut key_nodes = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if line.len() < 2 {
            key_nodes.push(Vec::new());
            continue;
        }
        let mut nodes = vec![
            endpoint_key(&line[0], snap_dist),
            endpoint_key(line.last().unwrap(), snap_dist),
        ];
        for p in line.iter().skip(1).take(line.len().saturating_sub(2)) {
            let mut touches_endpoint = false;
            for (ep, ep_id) in &endpoints {
                let dx = p.x - ep.x;
                let dy = p.y - ep.y;
                if *ep_id != i && dx * dx + dy * dy <= precision_sq {
                    touches_endpoint = true;
                    break;
                }
            }
            if touches_endpoint {
                let k = endpoint_key(p, snap_dist);
                if !nodes.contains(&k) {
                    nodes.push(k);
                }
            }
        }
        key_nodes.push(nodes);
    }

    key_nodes
}

fn snap_line_endpoints(lines: &mut [Vec<Coord>], snap_dist: f64) {
    let mut endpoints = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        endpoints.push((idx, true, line.first().cloned().unwrap()));
        endpoints.push((idx, false, line.last().cloned().unwrap()));
    }
    for i in 0..endpoints.len() {
        let (line_idx, is_start, coord) = endpoints[i].clone();
        let mut best = None;
        let mut best_dist = snap_dist;
        for (other_idx, _other_is_start, other) in &endpoints {
            if *other_idx == line_idx {
                continue;
            }
            let dist = coord_distance(&coord, other);
            if dist < best_dist {
                best_dist = dist;
                best = Some(other.clone());
            }
        }
        if let Some(snapped) = best {
            if is_start {
                lines[line_idx][0] = snapped;
            } else {
                let last = lines[line_idx].len() - 1;
                lines[line_idx][last] = snapped;
            }
        }
    }
}

fn merge_lines_at_degree_two(lines: Vec<Vec<Coord>>, snap_dist: f64) -> Vec<Vec<Coord>> {
    if lines.is_empty() {
        return lines;
    }
    let tol = (snap_dist / 2.0).max(1.0e-9);
    let mut node_to_lines: HashMap<(i64, i64), Vec<(usize, bool)>> = HashMap::new();
    for (idx, line) in lines.iter().enumerate() {
        node_to_lines
            .entry(endpoint_key(&line[0], tol))
            .or_default()
            .push((idx, true));
        node_to_lines
            .entry(endpoint_key(line.last().unwrap(), tol))
            .or_default()
            .push((idx, false));
    }

    let mut used = vec![false; lines.len()];
    let mut merged = Vec::new();

    for start_idx in 0..lines.len() {
        if used[start_idx] {
            continue;
        }
        let mut chain = lines[start_idx].clone();
        used[start_idx] = true;

        for extend_start in [false, true] {
            loop {
                let endpoint = if extend_start {
                    chain[0].clone()
                } else {
                    chain.last().cloned().unwrap()
                };
                let node_key = endpoint_key(&endpoint, tol);
                let Some(incidents) = node_to_lines.get(&node_key) else {
                    break;
                };
                if incidents.len() != 2 {
                    break;
                }
                let mut next = None;
                for &(cand_idx, cand_is_start) in incidents {
                    if !used[cand_idx] {
                        next = Some((cand_idx, cand_is_start));
                        break;
                    }
                }
                let Some((cand_idx, cand_is_start)) = next else {
                    break;
                };
                let mut cand = lines[cand_idx].clone();
                let cand_first_matches = coord_distance(&cand[0], &endpoint) <= tol;
                let cand_last_matches = coord_distance(cand.last().unwrap(), &endpoint) <= tol;
                if cand_is_start && !cand_first_matches && cand_last_matches {
                    cand.reverse();
                } else if !cand_is_start && cand_first_matches {
                    cand.reverse();
                }
                if extend_start {
                    if coord_distance(cand.last().unwrap(), &chain[0]) <= tol {
                        cand.pop();
                    }
                    cand.extend(chain);
                    chain = cand;
                } else {
                    if coord_distance(&chain[chain.len() - 1], &cand[0]) <= tol {
                        cand.remove(0);
                    }
                    chain.extend(cand);
                }
                used[cand_idx] = true;
            }
        }
        merged.push(chain);
    }

    merged
}

fn coord_eq_tol(a: &Coord, b: &Coord, tol: f64) -> bool {
    coord_distance(a, b) <= tol
}

fn project_point_to_segment(p: &Coord, a: &Coord, b: &Coord) -> (Coord, f64, f64) {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let wx = p.x - a.x;
    let wy = p.y - a.y;
    let vv = vx * vx + vy * vy;
    if vv <= f64::EPSILON {
        let c = Coord::xy(a.x, a.y);
        return (c.clone(), coord_distance(&c, p), 0.0);
    }
    let t = (wx * vx + wy * vy) / vv;
    let tc = t.clamp(0.0, 1.0);
    let c = Coord::xy(a.x + tc * vx, a.y + tc * vy);
    (c.clone(), coord_distance(&c, p), tc)
}

fn segment_intersection_point(a1: &Coord, a2: &Coord, b1: &Coord, b2: &Coord, tol: f64) -> Option<Coord> {
    let x1 = a1.x;
    let y1 = a1.y;
    let x2 = a2.x;
    let y2 = a2.y;
    let x3 = b1.x;
    let y3 = b1.y;
    let x4 = b2.x;
    let y4 = b2.y;

    let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if den.abs() <= f64::EPSILON {
        return None;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den;
    let u = ((x1 - x3) * (y1 - y2) - (y1 - y3) * (x1 - x2)) / den;
    if !(-tol..=1.0 + tol).contains(&t) || !(-tol..=1.0 + tol).contains(&u) {
        return None;
    }

    Some(Coord::xy(x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
}

fn insert_split_points(line: &[Coord], splits: &[(usize, Coord)], tol: f64) -> Vec<Coord> {
    if splits.is_empty() {
        return line.to_vec();
    }

    let mut by_seg: HashMap<usize, Vec<Coord>> = HashMap::new();
    for (seg, p) in splits {
        by_seg.entry(*seg).or_default().push(p.clone());
    }

    let mut out = Vec::<Coord>::new();
    out.push(line[0].clone());
    for seg in 0..(line.len() - 1) {
        let a = &line[seg];
        let b = &line[seg + 1];

        if let Some(points) = by_seg.get_mut(&seg) {
            points.sort_by(|p1, p2| {
                let (_, _, t1) = project_point_to_segment(p1, a, b);
                let (_, _, t2) = project_point_to_segment(p2, a, b);
                t1.partial_cmp(&t2).unwrap_or(std::cmp::Ordering::Equal)
            });
            for p in points {
                if !coord_eq_tol(out.last().unwrap(), p, tol) {
                    out.push(p.clone());
                }
            }
        }

        if !coord_eq_tol(out.last().unwrap(), b, tol) {
            out.push(b.clone());
        }
    }
    out
}

fn split_lines_at_intersections(lines: Vec<Vec<Coord>>, tol: f64) -> Vec<Vec<Coord>> {
    let mut splits: HashMap<usize, Vec<(usize, Coord)>> = HashMap::new();

    for i in 0..lines.len() {
        for j in (i + 1)..lines.len() {
            for si in 0..(lines[i].len() - 1) {
                let a1 = &lines[i][si];
                let a2 = &lines[i][si + 1];
                for sj in 0..(lines[j].len() - 1) {
                    let b1 = &lines[j][sj];
                    let b2 = &lines[j][sj + 1];
                    if let Some(p) = segment_intersection_point(a1, a2, b1, b2, tol) {
                        let near_i_endpoint = coord_eq_tol(&p, a1, tol) || coord_eq_tol(&p, a2, tol);
                        let near_j_endpoint = coord_eq_tol(&p, b1, tol) || coord_eq_tol(&p, b2, tol);
                        if near_i_endpoint && near_j_endpoint {
                            continue;
                        }
                        splits.entry(i).or_default().push((si, p.clone()));
                        splits.entry(j).or_default().push((sj, p));
                    }
                }
            }
        }
    }

    let mut out_lines = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let split_pts = splits.get(&idx).cloned().unwrap_or_default();
        let split_line = insert_split_points(line, &split_pts, tol);
        let mut start = 0usize;
        for k in 1..split_line.len() {
            if k + 1 < split_line.len() {
                let p = &split_line[k];
                let has_split = split_pts.iter().any(|(_, sp)| coord_eq_tol(sp, p, tol));
                if has_split {
                    let seg = split_line[start..=k].to_vec();
                    if seg.len() >= 2 {
                        out_lines.push(seg);
                    }
                    start = k;
                }
            }
        }
        let tail = split_line[start..].to_vec();
        if tail.len() >= 2 {
            out_lines.push(tail);
        }
    }
    out_lines
}

fn fix_dangling_arcs(mut lines: Vec<Vec<Coord>>, snap_dist: f64, tol: f64) -> Vec<Vec<Coord>> {
    if lines.is_empty() {
        return lines;
    }

    let mut split_requests: HashMap<usize, Vec<(usize, Coord)>> = HashMap::new();

    for idx in 0..lines.len() {
        for is_start in [true, false] {
            let endpoint = if is_start {
                lines[idx][0].clone()
            } else {
                lines[idx].last().cloned().unwrap()
            };

            let mut best: Option<(usize, usize, Coord, f64)> = None;
            for j in 0..lines.len() {
                if j == idx {
                    continue;
                }
                for sj in 0..(lines[j].len() - 1) {
                    let (proj, dist, _t) = project_point_to_segment(&endpoint, &lines[j][sj], &lines[j][sj + 1]);
                    if dist <= snap_dist {
                        if let Some((_, _, _, best_dist)) = best {
                            if dist < best_dist {
                                best = Some((j, sj, proj, dist));
                            }
                        } else {
                            best = Some((j, sj, proj, dist));
                        }
                    }
                }
            }

            if let Some((j, sj, proj, _)) = best {
                if is_start {
                    lines[idx][0] = proj.clone();
                } else {
                    let n = lines[idx].len();
                    lines[idx][n - 1] = proj.clone();
                }
                split_requests.entry(j).or_default().push((sj, proj));
            }
        }
    }

    for (line_idx, reqs) in split_requests {
        lines[line_idx] = insert_split_points(&lines[line_idx], &reqs, tol);
    }

    lines
}

fn sample_dem_at_coord(dem: &Raster, coord: &Coord) -> Option<f64> {
    let (row, col) = point_to_row_col(dem, coord.x, coord.y)?;
    let z = dem.get(0, row, col);
    if dem.is_nodata(z) {
        None
    } else {
        Some(z)
    }
}

fn line_length(line: &[Coord]) -> f64 {
    line.windows(2).map(|seg| coord_distance(&seg[0], &seg[1])).sum()
}

fn parse_d8_stream_inputs(
    args: &ToolArgs,
) -> Result<(Raster, Raster, Option<std::path::PathBuf>, bool, bool), ToolError> {
    let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr")?;
    let streams_path = parse_raster_path_arg(args, "streams_raster")?;
    let output_path = parse_optional_output_path(args, "output")?;
    let esri_style = args.get("esri_pntr").and_then(|v| v.as_bool()).unwrap_or(false);
    let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);

    let pntr = D8Core::load_raster(&d8_pntr_path)?;
    let streams = D8Core::load_raster(&streams_path)?;
    if streams.rows != pntr.rows || streams.cols != pntr.cols {
        return Err(ToolError::Validation(
            "Input rasters must have the same dimensions".to_string(),
        ));
    }
    Ok((pntr, streams, output_path, esri_style, zero_background))
}

fn downstream_cell(
    pntr: &Raster,
    row: usize,
    col: usize,
    pntr_matches: &[usize; 129],
) -> Option<(usize, usize, usize)> {
    let dir_val = pntr.get(0, row as isize, col as isize) as usize;
    if dir_val == 0 || dir_val > 128 || pntr_matches[dir_val] == 999 {
        return None;
    }
    let idx = pntr_matches[dir_val];
    let rn = row as isize + D8Core::D_Y[idx];
    let cn = col as isize + D8Core::D_X[idx];
    if rn < 0 || cn < 0 || rn >= pntr.rows as isize || cn >= pntr.cols as isize {
        return None;
    }
    Some((rn as usize, cn as usize, idx))
}

fn grid_lengths(pntr: &Raster) -> [f64; 8] {
    let cell_size_x = pntr.cell_size_x.abs();
    let cell_size_y = pntr.cell_size_y.abs();
    let diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
    [diag, cell_size_x, diag, cell_size_y, diag, cell_size_x, diag, cell_size_y]
}

fn compute_link_id_raster(
    pntr: &Raster,
    streams: &Raster,
    esri_style: bool,
    zero_background: bool,
) -> Raster {
    let rows = pntr.rows;
    let cols = pntr.cols;
    let nodata = streams.nodata;
    let pntr_nodata = pntr.nodata;
    let inflowing = D8Core::inflowing_vals(esri_style);
    let pntr_matches = D8Core::build_pntr_matches(esri_style);
    let background = if zero_background { 0.0 } else { nodata };

    let mut output = streams.clone();
    output.data_type = DataType::I32;
    let mut num_inflowing = vec![vec![-1i8; cols]; rows];
    let mut stack = Vec::new();
    let mut current_id = 1.0;

    for row in 0..rows {
        for col in 0..cols {
            if streams.get(0, row as isize, col as isize) > 0.0 {
                let c = D8Core::count_inflowing_cells(
                    streams,
                    pntr,
                    row as isize,
                    col as isize,
                    &inflowing,
                );
                num_inflowing[row][col] = c;
                if c == 0 {
                    stack.push((row, col));
                    output.set_unchecked(0, row as isize, col as isize, current_id);
                    current_id += 1.0;
                }
            } else if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                output.set_unchecked(0, row as isize, col as isize, background);
            } else {
                output.set_unchecked(0, row as isize, col as isize, nodata);
            }
        }
    }

    while let Some((row, col)) = stack.pop() {
        let val = output.get(0, row as isize, col as isize);
        if let Some((rn, cn, _idx)) = downstream_cell(pntr, row, col, &pntr_matches) {
            if streams.get(0, rn as isize, cn as isize) > 0.0 {
                if num_inflowing[rn][cn] > 1 {
                    output.set_unchecked(0, rn as isize, cn as isize, current_id);
                    current_id += 1.0;
                } else if output.get(0, rn as isize, cn as isize) <= 0.0
                    || output.get(0, rn as isize, cn as isize) == nodata
                {
                    output.set_unchecked(0, rn as isize, cn as isize, val);
                }
                num_inflowing[rn][cn] -= 1;
                if num_inflowing[rn][cn] == 0 {
                    stack.push((rn, cn));
                }
            }
        }
    }
    output
}

fn run_stream_tool_fallback(id: &str, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
    match id {
        "stream_link_identifier" => {
            let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
            let out = compute_link_id_raster(&pntr, &streams, esri_style, zero_background);
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "stream_link_class" => {
            let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_nodata = pntr.nodata;
            let background = if zero_background { 0.0 } else { nodata };
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);

            let mut out = streams.clone();
            out.data_type = DataType::I16;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 {
                        if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                            out.set_unchecked(0, row as isize, col as isize, background);
                        } else {
                            out.set_unchecked(0, row as isize, col as isize, nodata);
                        }
                        continue;
                    }
                    let in_count = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                    let has_down = downstream_cell(&pntr, row, col, &pntr_matches)
                        .map(|(rn, cn, _)| streams.get(0, rn as isize, cn as isize) > 0.0)
                        .unwrap_or(false);
                    let class_val = if !has_down {
                        5.0
                    } else if in_count == 0 {
                        3.0
                    } else if in_count > 1 {
                        2.0
                    } else {
                        4.0
                    };
                    out.set_unchecked(0, row as isize, col as isize, class_val);
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "stream_link_length" => {
            let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr")
                .or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
            let streams_path = parse_raster_path_arg(args, "streams_id_raster")
                .or_else(|_| parse_raster_path_arg(args, "streams_raster"))
                .or_else(|_| parse_raster_path_arg(args, "streams"))?;
            let output_path = parse_optional_output_path(args, "output")?;
            let esri_style = args
                .get("esri_pntr")
                .or_else(|| args.get("esri_pointer"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let pntr = D8Core::load_raster(&d8_pntr_path)?;
            let mut streams = D8Core::load_raster(&streams_path)?;
            if streams.rows != pntr.rows || streams.cols != pntr.cols {
                return Err(ToolError::Validation("Input rasters must have the same dimensions".to_string()));
            }
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);
            let use_input_link_ids = args.get("streams_id_raster").is_some();
            let link_id = if use_input_link_ids {
                streams.clone()
            } else {
                compute_link_id_raster(&pntr, &streams, esri_style, true)
            };

            if use_input_link_ids {
                for row in 0..rows {
                    for col in 0..cols {
                        let lid = link_id.get(0, row as isize, col as isize);
                        streams.set_unchecked(0, row as isize, col as isize, if lid > 0.0 { 1.0 } else { 0.0 });
                    }
                }
            }
            let mut link_len: BTreeMap<i64, f64> = BTreeMap::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 {
                        continue;
                    }
                    let lid = link_id.get(0, row as isize, col as isize) as i64;
                    if let Some((_rn, _cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                        *link_len.entry(lid).or_insert(0.0) += lengths[idx];
                    }
                }
            }
            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let lid = link_id.get(0, row as isize, col as isize) as i64;
                        out.set_unchecked(0, row as isize, col as isize, *link_len.get(&lid).unwrap_or(&0.0));
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "distance_to_outlet" => {
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);

            let mut dist = vec![vec![-1.0f64; cols]; rows];
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 || dist[row][col] >= 0.0 {
                        continue;
                    }
                    let mut path: Vec<(usize, usize, f64)> = Vec::new();
                    let mut y = row;
                    let mut x = col;
                    loop {
                        if streams.get(0, y as isize, x as isize) <= 0.0 {
                            break;
                        }
                        if dist[y][x] >= 0.0 {
                            break;
                        }
                        if let Some((yn, xn, idx)) = downstream_cell(&pntr, y, x, &pntr_matches) {
                            let step = if streams.get(0, yn as isize, xn as isize) > 0.0 { lengths[idx] } else { 0.0 };
                            path.push((y, x, step));
                            if streams.get(0, yn as isize, xn as isize) <= 0.0 {
                                y = yn;
                                x = xn;
                                break;
                            }
                            y = yn;
                            x = xn;
                        } else {
                            path.push((y, x, 0.0));
                            break;
                        }
                    }
                    let mut base = if y < rows && x < cols && dist[y][x] >= 0.0 { dist[y][x] } else { 0.0 };
                    for (py, px, step) in path.into_iter().rev() {
                        base += step;
                        dist[py][px] = base;
                    }
                }
            }

            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        out.set_unchecked(0, row as isize, col as isize, dist[row][col].max(0.0));
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "length_of_upstream_channels" => {
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);

            let mut out_vals = vec![vec![0.0f64; cols]; rows];
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut stack = Vec::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                        }
                    }
                }
            }
            while let Some((row, col)) = stack.pop() {
                if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                    if streams.get(0, rn as isize, cn as isize) > 0.0 {
                        out_vals[rn][cn] += out_vals[row][col] + lengths[idx];
                        num_inflowing[rn][cn] -= 1;
                        if num_inflowing[rn][cn] == 0 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }
            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        out.set_unchecked(0, row as isize, col as isize, out_vals[row][col]);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "farthest_channel_head" => {
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);

            let mut out_vals = vec![vec![0.0f64; cols]; rows];
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut stack = Vec::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                        }
                    }
                }
            }
            while let Some((row, col)) = stack.pop() {
                if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                    if streams.get(0, rn as isize, cn as isize) > 0.0 {
                        out_vals[rn][cn] = out_vals[rn][cn].max(out_vals[row][col] + lengths[idx]);
                        num_inflowing[rn][cn] -= 1;
                        if num_inflowing[rn][cn] == 0 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }
            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        out.set_unchecked(0, row as isize, col as isize, out_vals[row][col]);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "find_main_stem" => {
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);

            let mut far = vec![vec![0.0f64; cols]; rows];
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut stack = Vec::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                        }
                    }
                }
            }
            while let Some((row, col)) = stack.pop() {
                if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                    if streams.get(0, rn as isize, cn as isize) > 0.0 {
                        far[rn][cn] = far[rn][cn].max(far[row][col] + lengths[idx]);
                        num_inflowing[rn][cn] -= 1;
                        if num_inflowing[rn][cn] == 0 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }

            let mut out = streams.clone();
            out.data_type = DataType::I16;
            for row in 0..rows {
                for col in 0..cols {
                    out.set_unchecked(0, row as isize, col as isize, if streams.get(0, row as isize, col as isize) > 0.0 { 0.0 } else { nodata });
                }
            }

            for row in 0..rows {
                for col in 0..cols {
                    let is_outlet = streams.get(0, row as isize, col as isize) > 0.0
                        && downstream_cell(&pntr, row, col, &pntr_matches)
                            .map(|(rn, cn, _)| streams.get(0, rn as isize, cn as isize) <= 0.0)
                            .unwrap_or(true);
                    if !is_outlet {
                        continue;
                    }
                    let mut y = row as isize;
                    let mut x = col as isize;
                    loop {
                        out.set_unchecked(0, y, x, 1.0);
                        let mut best = None;
                        let mut best_dist = -1.0;
                        for i in 0..8 {
                            let yn = y + D8Core::D_Y[i];
                            let xn = x + D8Core::D_X[i];
                            if yn < 0 || xn < 0 || yn >= rows as isize || xn >= cols as isize {
                                continue;
                            }
                            if streams.get(0, yn, xn) > 0.0 && pntr.get(0, yn, xn) == inflowing[i] {
                                let d = far[yn as usize][xn as usize];
                                if d > best_dist {
                                    best_dist = d;
                                    best = Some((yn, xn));
                                }
                            }
                        }
                        if let Some((yn, xn)) = best {
                            y = yn;
                            x = xn;
                        } else {
                            break;
                        }
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "tributary_identifier" => {
            let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_nodata = pntr.nodata;
            let background = if zero_background { 0.0 } else { nodata };
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);

            let mut out = streams.clone();
            out.data_type = DataType::I32;
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut trib_len = vec![vec![nodata; cols]; rows];
            let mut stack = Vec::new();
            let mut current_id = 1.0;

            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                            out.set_unchecked(0, row as isize, col as isize, current_id);
                            trib_len[row][col] = 0.0;
                            current_id += 1.0;
                        }
                    } else if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                        out.set_unchecked(0, row as isize, col as isize, background);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }

            while let Some((row, col)) = stack.pop() {
                let val = out.get(0, row as isize, col as isize);
                if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                    let new_len = trib_len[row][col] + lengths[idx];
                    if streams.get(0, rn as isize, cn as isize) > 0.0
                        && (trib_len[rn][cn] == nodata || new_len > trib_len[rn][cn])
                    {
                        trib_len[rn][cn] = new_len;
                        out.set_unchecked(0, rn as isize, cn as isize, val);
                    }
                    if streams.get(0, rn as isize, cn as isize) > 0.0 {
                        num_inflowing[rn][cn] -= 1;
                        if num_inflowing[rn][cn] == 0 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "remove_short_streams" => {
            let min_length = args.get("min_length").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);
            let link_id = compute_link_id_raster(&pntr, &streams, esri_style, true);
            let mut link_len: BTreeMap<i64, f64> = BTreeMap::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 {
                        continue;
                    }
                    let lid = link_id.get(0, row as isize, col as isize) as i64;
                    if let Some((_rn, _cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                        *link_len.entry(lid).or_insert(0.0) += lengths[idx];
                    }
                }
            }
            let mut out = streams.clone();
            out.data_type = DataType::I16;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let lid = link_id.get(0, row as isize, col as isize) as i64;
                        out.set_unchecked(0, row as isize, col as isize, if link_len.get(&lid).copied().unwrap_or(0.0) >= min_length { 1.0 } else { 0.0 });
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "topological_stream_order" => {
            let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_nodata = pntr.nodata;
            let background = if zero_background { 0.0 } else { nodata };
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);

            let mut out = streams.clone();
            out.data_type = DataType::I16;
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut stack = Vec::new();

            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                            out.set_unchecked(0, row as isize, col as isize, 1.0);
                        }
                    } else if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                        out.set_unchecked(0, row as isize, col as isize, background);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }

            while let Some((row, col)) = stack.pop() {
                let v = out.get(0, row as isize, col as isize);
                if let Some((rn, cn, _)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                    if streams.get(0, rn as isize, cn as isize) > 0.0 {
                        out.set_unchecked(0, rn as isize, cn as isize, out.get(0, rn as isize, cn as isize).max(v + 1.0));
                        num_inflowing[rn][cn] -= 1;
                        if num_inflowing[rn][cn] == 0 {
                            stack.push((rn, cn));
                        }
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "stream_slope_continuous" => {
            let dem_path = parse_raster_path_arg(args, "dem")
                .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
            let dem = D8Core::load_raster(&dem_path)?;
            let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;
            if dem.rows != pntr.rows || dem.cols != pntr.cols {
                return Err(ToolError::Validation(
                    "Input DEM and stream rasters must have the same dimensions".to_string(),
                ));
            }
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);
            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                        continue;
                    }
                    let slope = if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                        if streams.get(0, rn as isize, cn as isize) > 0.0 {
                            let dz = dem.get(0, row as isize, col as isize) - dem.get(0, rn as isize, cn as isize);
                            dz / lengths[idx].max(f64::EPSILON)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    out.set_unchecked(0, row as isize, col as isize, slope);
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "stream_link_slope" => {
            let dem_path = parse_raster_path_arg(args, "dem")
                .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
            let dem = D8Core::load_raster(&dem_path)?;
            let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr")
                .or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
            let streams_path = parse_raster_path_arg(args, "streams_id_raster")
                .or_else(|_| parse_raster_path_arg(args, "streams_raster"))
                .or_else(|_| parse_raster_path_arg(args, "streams"))?;
            let output_path = parse_optional_output_path(args, "output")?;
            let esri_style = args
                .get("esri_pntr")
                .or_else(|| args.get("esri_pointer"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let pntr = D8Core::load_raster(&d8_pntr_path)?;
            let mut streams = D8Core::load_raster(&streams_path)?;
            if streams.rows != pntr.rows || streams.cols != pntr.cols {
                return Err(ToolError::Validation("Input rasters must have the same dimensions".to_string()));
            }
            if dem.rows != pntr.rows || dem.cols != pntr.cols {
                return Err(ToolError::Validation(
                    "Input DEM and stream rasters must have the same dimensions".to_string(),
                ));
            }
            let rows = pntr.rows;
            let cols = pntr.cols;
            let nodata = streams.nodata;
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let lengths = grid_lengths(&pntr);
            let use_input_link_ids = args.get("streams_id_raster").is_some();
            let link_id = if use_input_link_ids {
                streams.clone()
            } else {
                compute_link_id_raster(&pntr, &streams, esri_style, true)
            };

            if use_input_link_ids {
                for row in 0..rows {
                    for col in 0..cols {
                        let lid = link_id.get(0, row as isize, col as isize);
                        streams.set_unchecked(0, row as isize, col as isize, if lid > 0.0 { 1.0 } else { 0.0 });
                    }
                }
            }
            let mut sum_slope: BTreeMap<i64, f64> = BTreeMap::new();
            let mut count: BTreeMap<i64, f64> = BTreeMap::new();
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) <= 0.0 {
                        continue;
                    }
                    if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                        if streams.get(0, rn as isize, cn as isize) > 0.0 {
                            let lid = link_id.get(0, row as isize, col as isize) as i64;
                            let dz = dem.get(0, row as isize, col as isize) - dem.get(0, rn as isize, cn as isize);
                            let s = dz / lengths[idx].max(f64::EPSILON);
                            *sum_slope.entry(lid).or_insert(0.0) += s;
                            *count.entry(lid).or_insert(0.0) += 1.0;
                        }
                    }
                }
            }
            let mut out = streams.clone();
            out.data_type = DataType::F32;
            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0 {
                        let lid = link_id.get(0, row as isize, col as isize) as i64;
                        let avg = sum_slope.get(&lid).copied().unwrap_or(0.0)
                            / count.get(&lid).copied().unwrap_or(1.0);
                        out.set_unchecked(0, row as isize, col as isize, avg);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, nodata);
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "extract_streams" => {
            let input = parse_raster_path_arg(args, "flow_accumulation")
                .or_else(|_| parse_raster_path_arg(args, "input"))?;
            let threshold = args
                .get("threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(1000.0);
            let output_path = parse_optional_output_path(args, "output")?;
            let fa = D8Core::load_raster(&input)?;
            let mut out = fa.clone();
            out.data_type = DataType::I16;
            for row in 0..fa.rows {
                for col in 0..fa.cols {
                    let v = fa.get(0, row as isize, col as isize);
                    if fa.is_nodata(v) {
                        out.set_unchecked(0, row as isize, col as isize, fa.nodata);
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, if v >= threshold { 1.0 } else { 0.0 });
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "extract_valleys" => {
            let input = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
            let line_thin = args
                .get("line_thin")
                .or_else(|| args.get("thin"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let variant = args
                .get("variant")
                .or_else(|| args.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("lq")
                .to_lowercase();
            let variant = if variant.contains('q') {
                "lq"
            } else if variant.contains('j') {
                "jandr"
            } else {
                "pandd"
            };
            let mut filter_size = args
                .get("filter_size")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(5);
            if variant == "lq" && filter_size % 2 == 0 {
                filter_size += 1;
            }
            let output_path = parse_optional_output_path(args, "output")?;
            let dem = D8Core::load_raster(&input)?;
            let mut out = dem.clone();
            out.data_type = DataType::I16;

            match variant {
                "lq" => {
                    let num_cells = filter_size * filter_size;
                    let midpoint = (filter_size / 2) as isize;
                    let sqr_radius = (midpoint * midpoint) as f64;
                    let mut dx = vec![0isize; num_cells];
                    let mut dy = vec![0isize; num_cells];
                    let mut filter_shape = vec![true; num_cells];
                    let mut i = 0usize;
                    for row in 0..filter_size as isize {
                        for col in 0..filter_size as isize {
                            dx[i] = col - midpoint;
                            dy[i] = row - midpoint;
                            let d2 = (dx[i] * dx[i] + dy[i] * dy[i]) as f64;
                            if d2 > sqr_radius {
                                filter_shape[i] = false;
                            }
                            i += 1;
                        }
                    }

                    let mut cell_data = vec![0.0f64; num_cells];
                    let large_value = f64::INFINITY;
                    for row in 0..dem.rows as isize {
                        for col in 0..dem.cols as isize {
                            let z = dem.get(0, row, col);
                            if dem.is_nodata(z) {
                                out.set_unchecked(0, row, col, dem.nodata);
                                continue;
                            }

                            let mut n = 0usize;
                            for j in 0..num_cells {
                                if !filter_shape[j] {
                                    continue;
                                }
                                let zn = dem.get(0, row + dy[j], col + dx[j]);
                                if !dem.is_nodata(zn) {
                                    cell_data[j] = zn;
                                    n += 1;
                                } else {
                                    cell_data[j] = large_value;
                                }
                            }
                            if n == 0 {
                                out.set_unchecked(0, row, col, 0.0);
                                continue;
                            }
                            cell_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                            let lower_quartile = ((n as f64) / 4.0).floor() as usize;
                            let is_valley = z <= cell_data[lower_quartile];
                            out.set_unchecked(0, row, col, if is_valley { 1.0 } else { 0.0 });
                        }
                    }
                }
                "jandr" => {
                    for row in 0..dem.rows as isize {
                        for col in 0..dem.cols as isize {
                            let z = dem.get(0, row, col);
                            if dem.is_nodata(z) {
                                out.set_unchecked(0, row, col, dem.nodata);
                                continue;
                            }

                            let n = dem.get(0, row - 1, col);
                            let s = dem.get(0, row + 1, col);
                            let e = dem.get(0, row, col + 1);
                            let w = dem.get(0, row, col - 1);
                            let is_valley = (!dem.is_nodata(n) && !dem.is_nodata(s) && n > z && s > z)
                                || (!dem.is_nodata(e) && !dem.is_nodata(w) && e > z && w > z);
                            out.set_unchecked(0, row, col, if is_valley { 1.0 } else { 0.0 });
                        }
                    }
                }
                _ => {
                    out.fill(1.0);
                    let dx = [-1isize, 0, -1, 0];
                    let dy = [-1isize, -1, 0, 0];
                    for row in 0..dem.rows as isize {
                        for col in 0..dem.cols as isize {
                            let z0 = dem.get(0, row, col);
                            if dem.is_nodata(z0) {
                                out.set_unchecked(0, row, col, dem.nodata);
                                continue;
                            }

                            let mut maxz = z0;
                            let mut which = 3usize;
                            for j in 0..dx.len() {
                                let zn = dem.get(0, row + dy[j], col + dx[j]);
                                if dem.is_nodata(zn) {
                                    continue;
                                }
                                if zn > maxz {
                                    maxz = zn;
                                    which = j;
                                }
                            }
                            out.set_unchecked(0, row + dy[which], col + dx[which], 0.0);
                        }
                    }
                }
            }

            if line_thin {
                let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
                let elements: [[usize; 6]; 8] = [
                    [6, 7, 0, 4, 3, 2],
                    [7, 0, 1, 3, 5, 999],
                    [0, 1, 2, 4, 5, 6],
                    [1, 2, 3, 5, 7, 999],
                    [2, 3, 4, 6, 7, 0],
                    [3, 4, 5, 7, 1, 999],
                    [4, 5, 6, 0, 1, 2],
                    [5, 6, 7, 1, 3, 999],
                ];
                let vals: [[f64; 6]; 8] = [
                    [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, -1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, -1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, -1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                    [0.0, 0.0, 0.0, 1.0, 1.0, -1.0],
                ];

                let mut did_something = true;
                while did_something {
                    did_something = false;
                    for a in 0..8 {
                        for row in 0..dem.rows as isize {
                            for col in 0..dem.cols as isize {
                                let z = out.get(0, row, col);
                                if z <= 0.0 || out.is_nodata(z) {
                                    continue;
                                }

                                let mut neighbours = [0.0f64; 8];
                                for i in 0..8 {
                                    neighbours[i] = out.get(0, row + dy[i], col + dx[i]);
                                }

                                let mut pattern_match = true;
                                for i in 0..6 {
                                    if elements[a][i] == 999 {
                                        continue;
                                    }
                                    let target = vals[a][i];
                                    if (neighbours[elements[a][i]] - target).abs() > f64::EPSILON {
                                        pattern_match = false;
                                        break;
                                    }
                                }
                                if pattern_match {
                                    out.set_unchecked(0, row, col, 0.0);
                                    did_something = true;
                                }
                            }
                        }
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "raster_streams_to_vector" => {
            let streams_path = parse_raster_path_arg(args, "streams_raster").or_else(|_| parse_raster_path_arg(args, "streams"))?;
            let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
            let esri_style = args.get("esri_pntr").or_else(|| args.get("esri_pointer")).and_then(|v| v.as_bool()).unwrap_or(false);
            let all_vertices = args.get("all_vertices").and_then(|v| v.as_bool()).unwrap_or(false);
            let output = args
                .get("output")
                .or_else(|| args.get("output_vector"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Validation("missing required parameter 'output' for vector output".to_string()))?
                .to_string();
            let streams = D8Core::load_raster(&streams_path)?;
            let pntr = D8Core::load_raster(&d8_pntr_path)?;
            if streams.rows != pntr.rows || streams.cols != pntr.cols {
                return Err(ToolError::Validation("Input rasters must have the same dimensions".to_string()));
            }
            let rows = streams.rows;
            let cols = streams.cols;
            let nodata = streams.nodata;
            let inflowing = D8Core::inflowing_vals(esri_style);
            let pntr_matches = D8Core::build_pntr_matches(esri_style);
            let mut num_inflowing = vec![vec![-1i8; cols]; rows];
            let mut stack = Vec::new();

            for row in 0..rows {
                for col in 0..cols {
                    if streams.get(0, row as isize, col as isize) > 0.0
                        && streams.get(0, row as isize, col as isize) != nodata
                    {
                        let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                        num_inflowing[row][col] = c;
                        if c == 0 {
                            stack.push((row, col));
                        }
                    }
                }
            }

            let mut layer = Layer::new("streams").with_geom_type(GeometryType::LineString);
            layer.add_field(FieldDef::new("FID", FieldType::Integer));
            layer.add_field(FieldDef::new("STRM_VAL", FieldType::Float));
            let mut fid = 1i64;
            while let Some((row, col)) = stack.pop() {
                if num_inflowing[row][col] == -1 {
                    continue;
                }
                let stream_val = streams.get(0, row as isize, col as isize);
                num_inflowing[row][col] = -1;
                let mut coords = Vec::new();
                let mut y = row;
                let mut x = col;
                let mut prev_dir = usize::MAX;
                loop {
                    let dir = pntr.get(0, y as isize, x as isize) as usize;
                    let add_here = all_vertices || dir != prev_dir;
                    if add_here {
                        coords.push(Coord::xy(pntr.col_center_x(x as isize), pntr.row_center_y(y as isize)));
                        prev_dir = dir;
                    }
                    if let Some((yn, xn, _idx)) = downstream_cell(&pntr, y, x, &pntr_matches) {
                        let next_val = streams.get(0, yn as isize, xn as isize);
                        if next_val > 0.0 && next_val == stream_val && num_inflowing[yn][xn] == 1 {
                            y = yn;
                            x = xn;
                            num_inflowing[y][x] = -1;
                            continue;
                        }
                        coords.push(Coord::xy(pntr.col_center_x(xn as isize), pntr.row_center_y(yn as isize)));
                        if next_val > 0.0 {
                            stack.push((yn, xn));
                        }
                    }
                    break;
                }
                if coords.len() > 1 {
                    layer.add_feature(
                        Some(Geometry::line_string(coords)),
                        &[("FID", FieldValue::Integer(fid)), ("STRM_VAL", FieldValue::Float(stream_val))],
                    )
                    .map_err(|e| ToolError::Execution(format!("failed building output feature: {}", e)))?;
                    fid += 1;
                }
            }
            ensure_parent_dir(&output)?;
            Ok(D8Core::build_result(write_vector(&layer, &output)?))
        }
        "rasterize_streams" => {
            let input_vector = parse_vector_path_arg(args, "input_vector")
                .or_else(|_| parse_vector_path_arg(args, "input"))
                .or_else(|_| parse_vector_path_arg(args, "streams"))?;
            let reference = parse_raster_path_arg(args, "reference_raster")
                .or_else(|_| parse_raster_path_arg(args, "base"))
                .or_else(|_| parse_raster_path_arg(args, "base_raster"))?;
            let output_path = parse_optional_output_path(args, "output")?;
            let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
            let use_feature_id = args.get("use_feature_id").and_then(|v| v.as_bool()).unwrap_or(false);
            let layer = load_vector(&input_vector)?;
            let mut out = D8Core::load_raster(&reference)?;
            out.data_type = DataType::I16;
            let background = if zero_background { 0.0 } else { out.nodata };
            for row in 0..out.rows {
                for col in 0..out.cols {
                    out.set_unchecked(0, row as isize, col as isize, background);
                }
            }
            for (idx, feat) in layer.features.iter().enumerate() {
                let burn = if use_feature_id { (idx + 1) as f64 } else { 1.0 };
                if let Some(geom) = &feat.geometry {
                    match geom {
                        Geometry::LineString(coords) => {
                            for seg in coords.windows(2) {
                                rasterize_segment(&mut out, &seg[0], &seg[1], burn);
                            }
                        }
                        Geometry::MultiLineString(lines) => {
                            for line in lines {
                                for seg in line.windows(2) {
                                    rasterize_segment(&mut out, &seg[0], &seg[1], burn);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
        }
        "long_profile" => {
            let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
            let streams_path = parse_raster_path_arg(args, "streams_raster").or_else(|_| parse_raster_path_arg(args, "streams"))?;
            let dem_path = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
            let output = output_html_path(args)?;
            let esri_style = args.get("esri_pntr").or_else(|| args.get("esri_pointer")).and_then(|v| v.as_bool()).unwrap_or(false);
            let pntr = D8Core::load_raster(&d8_pntr_path)?;
            let streams = D8Core::load_raster(&streams_path)?;
            let dem = D8Core::load_raster(&dem_path)?;
            if pntr.rows != streams.rows || pntr.cols != streams.cols || pntr.rows != dem.rows || pntr.cols != dem.cols {
                return Err(ToolError::Validation("Input rasters must have the same dimensions".to_string()));
            }
            let mut profiles = Vec::new();
            for head in stream_heads(&streams, &pntr, esri_style) {
                if let Ok(profile) = sample_profile_from_start(head, &pntr, &dem, Some(&streams), esri_style) {
                    profiles.push(profile);
                }
            }
            ensure_parent_dir(&output)?;
            std::fs::write(&output, render_profile_html("Long Profile", &profiles))
                .map_err(|e| ToolError::Execution(format!("failed writing html output: {}", e)))?;
            Ok(D8Core::build_result(output))
        }
        "long_profile_from_points" => {
            let d8_pntr_path = parse_raster_path_arg(args, "d8_pntr").or_else(|_| parse_raster_path_arg(args, "d8_pointer"))?;
            let points_path = parse_vector_path_arg(args, "points").or_else(|_| parse_vector_path_arg(args, "input_points"))?;
            let dem_path = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
            let output = output_html_path(args)?;
            let esri_style = args.get("esri_pntr").or_else(|| args.get("esri_pointer")).and_then(|v| v.as_bool()).unwrap_or(false);
            let pntr = D8Core::load_raster(&d8_pntr_path)?;
            let dem = D8Core::load_raster(&dem_path)?;
            if pntr.rows != dem.rows || pntr.cols != dem.cols {
                return Err(ToolError::Validation("Input rasters must have the same dimensions".to_string()));
            }
            let points = load_vector(&points_path)?;
            let mut profiles = Vec::new();
            for feat in &points.features {
                if let Some(Geometry::Point(coord)) = &feat.geometry {
                    if let Some((row, col)) = point_to_row_col(&dem, coord.x, coord.y) {
                        if let Ok(profile) = sample_profile_from_start((row as usize, col as usize), &pntr, &dem, None, esri_style) {
                            profiles.push(profile);
                        }
                    }
                }
            }
            ensure_parent_dir(&output)?;
            std::fs::write(&output, render_profile_html("Long Profile From Points", &profiles))
                .map_err(|e| ToolError::Execution(format!("failed writing html output: {}", e)))?;
            Ok(D8Core::build_result(output))
        }
        "repair_stream_vector_topology" => {
            let input = parse_vector_path_arg(args, "input_vector")
                .or_else(|_| parse_vector_path_arg(args, "input"))?;
            let output = args
                .get("output")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Validation("missing required parameter 'output'".to_string()))?
                .to_string();
            let snap_dist = args.get("snap").or_else(|| args.get("snap_dist")).and_then(|v| v.as_f64()).unwrap_or(0.001);
            if snap_dist <= 0.0 {
                return Err(ToolError::Validation("snap distance must be greater than zero".to_string()));
            }
            let layer = load_vector(&input)?;
            let mut lines = line_geometries(&layer);
            let tol = (snap_dist / 10.0).max(1.0e-9);

            // Legacy-inspired sequence: endpoint snap, chain merge, intersection split, dangling arc fix.
            snap_line_endpoints(&mut lines, snap_dist);
            lines = merge_lines_at_degree_two(lines, snap_dist);
            lines = split_lines_at_intersections(lines, tol);
            lines = fix_dangling_arcs(lines, snap_dist, tol);
            lines = merge_lines_at_degree_two(lines, snap_dist);

            // Remove degenerate segments after topology operations.
            let mut cleaned = Vec::new();
            for mut line in lines {
                for i in (1..line.len()).rev() {
                    if coord_distance(&line[i], &line[i - 1]) <= tol {
                        line.remove(i);
                    }
                }
                if line.len() >= 2 {
                    cleaned.push(line);
                }
            }

            let mut out = Layer::new("repaired_streams").with_geom_type(GeometryType::LineString);
            out.add_field(FieldDef::new("FID", FieldType::Integer));
            for (idx, line) in cleaned.into_iter().enumerate() {
                out.add_feature(
                    Some(Geometry::line_string(line)),
                    &[("FID", FieldValue::Integer((idx + 1) as i64))],
                )
                .map_err(|e| ToolError::Execution(format!("failed building repaired stream layer: {}", e)))?;
            }
            ensure_parent_dir(&output)?;
            Ok(D8Core::build_result(write_vector(&out, &output)?))
        }
        "vector_stream_network_analysis" => {
            let input = parse_vector_path_arg(args, "input_vector")
                .or_else(|_| parse_vector_path_arg(args, "input"))
                .or_else(|_| parse_vector_path_arg(args, "streams"))?;
            let dem_path = parse_raster_path_arg(args, "dem")
                .or_else(|_| parse_raster_path_arg(args, "input_dem"))?;
            let output = args
                .get("output")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Validation("missing required parameter 'output'".to_string()))?
                .to_string();
            let snap_dist = args.get("snap").or_else(|| args.get("snap_distance")).and_then(|v| v.as_f64()).unwrap_or(0.001);
            let max_ridge_cutting_height = args
                .get("max_ridge_cutting_height")
                .and_then(|v| v.as_f64())
                .unwrap_or(10.0)
                .max(0.0);
            let dem = D8Core::load_raster(&dem_path)?;
            let layer = load_vector(&input)?;
            let mut lines = line_geometries(&layer);
            snap_line_endpoints(&mut lines, snap_dist);
            let tol = (snap_dist / 2.0).max(1.0e-9);
            let precision_sq = (f64::EPSILON * 10.0) * (f64::EPSILON * 10.0);

            #[derive(Clone)]
            struct LinkInfo {
                geom: Vec<Coord>,
                up_key: (i64, i64),
                down_key: (i64, i64),
                length: f64,
                min_elev: f64,
                max_elev: f64,
            }

            let mut links = Vec::new();
            let mut node_coord: HashMap<(i64, i64), Coord> = HashMap::new();
            for mut line in lines {
                let start_z = sample_dem_at_coord(&dem, &line[0]).unwrap_or(0.0);
                let end_z = sample_dem_at_coord(&dem, line.last().unwrap()).unwrap_or(0.0);
                if end_z > start_z {
                    line.reverse();
                }
                let mut min_elev = f64::INFINITY;
                let mut max_elev = f64::NEG_INFINITY;
                for coord in &line {
                    if let Some(z) = sample_dem_at_coord(&dem, coord) {
                        min_elev = min_elev.min(z);
                        max_elev = max_elev.max(z);
                    }
                }
                if !min_elev.is_finite() {
                    min_elev = start_z.min(end_z);
                    max_elev = start_z.max(end_z);
                }
                let up = endpoint_key(&line[0], tol);
                let down = endpoint_key(line.last().unwrap(), tol);
                node_coord.entry(up).or_insert_with(|| line[0].clone());
                node_coord.entry(down).or_insert_with(|| line.last().cloned().unwrap());
                links.push(LinkInfo {
                    length: line_length(&line),
                    geom: line,
                    up_key: up,
                    down_key: down,
                    min_elev,
                    max_elev,
                });
            }

            let geom_lines: Vec<Vec<Coord>> = links.iter().map(|l| l.geom.clone()).collect();
            let link_key_nodes = collect_link_key_nodes(&geom_lines, tol, precision_sq);

            let mut upstream_of: Vec<Vec<usize>> = vec![Vec::new(); links.len()];
            let mut downstream_of: Vec<Option<usize>> = vec![None; links.len()];
            let mut node_links: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
            for (idx, link) in links.iter().enumerate() {
                for node in &link_key_nodes[idx] {
                    node_links.entry(*node).or_default().push(idx);
                }
                // Preserve endpoint coordinates for output point generation.
                node_links.entry(link.up_key).or_default().push(idx);
                node_links.entry(link.down_key).or_default().push(idx);
            }
            for idx in 0..links.len() {
                let mut cands = Vec::new();
                for node in &link_key_nodes[idx] {
                    if let Some(neigh) = node_links.get(node) {
                        cands.extend(neigh.iter().copied());
                    }
                }
                let chosen = cands
                    .into_iter()
                    .filter(|cand| *cand != idx)
                    .filter(|cand| links[*cand].min_elev <= links[idx].min_elev + max_ridge_cutting_height)
                    .min_by(|a, b| links[*a].min_elev.partial_cmp(&links[*b].min_elev).unwrap_or(std::cmp::Ordering::Equal));
                downstream_of[idx] = chosen;
            }
            for (idx, ds) in downstream_of.iter().enumerate() {
                if let Some(ds_idx) = ds {
                    upstream_of[*ds_idx].push(idx);
                }
            }

            let mut dist2mouth = vec![0.0f64; links.len()];
            let mut ds_nodes = vec![0i64; links.len()];
            let mut outlets = Vec::new();
            for idx in 0..links.len() {
                if downstream_of[idx].is_none() {
                    outlets.push(idx);
                }
            }
            let mut changed = true;
            while changed {
                changed = false;
                for idx in (0..links.len()).rev() {
                    if let Some(ds) = downstream_of[idx] {
                        let new_dist = links[idx].length + dist2mouth[ds];
                        let new_nodes = ds_nodes[ds] + 1;
                        if (dist2mouth[idx] - new_dist).abs() > 1.0e-9 || ds_nodes[idx] != new_nodes {
                            dist2mouth[idx] = new_dist;
                            ds_nodes[idx] = new_nodes;
                            changed = true;
                        }
                    }
                }
            }

            let mut tucl = vec![0.0f64; links.len()];
            let mut max_ups = vec![0.0f64; links.len()];
            let mut shreve = vec![1i64; links.len()];
            let mut strahler = vec![1i64; links.len()];
            let mut pending: Vec<usize> = upstream_of.iter().map(|u| u.len()).collect();
            let mut stack: Vec<usize> = (0..links.len()).filter(|&i| pending[i] == 0).collect();
            while let Some(idx) = stack.pop() {
                tucl[idx] += links[idx].length;
                max_ups[idx] += links[idx].length;
                if let Some(ds) = downstream_of[idx] {
                    tucl[ds] += tucl[idx];
                    max_ups[ds] = max_ups[ds].max(max_ups[idx]);
                    shreve[ds] += shreve[idx];
                    let ds_order = strahler[ds];
                    if strahler[idx] > ds_order {
                        strahler[ds] = strahler[idx];
                    } else if strahler[idx] == ds_order {
                        strahler[ds] = ds_order + 1;
                    }
                    pending[ds] -= 1;
                    if pending[ds] == 0 {
                        stack.push(ds);
                    }
                }
            }

            let mut mainstream = vec![0i64; links.len()];
            let mut horton = strahler.clone();
            let mut hack = vec![0i64; links.len()];
            for outlet in &outlets {
                let mut cur = *outlet;
                let outlet_order = strahler[cur];
                let mut hack_order = 1i64;
                loop {
                    mainstream[cur] = 1;
                    horton[cur] = outlet_order;
                    hack[cur] = hack_order;
                    let ups = &upstream_of[cur];
                    if ups.is_empty() {
                        break;
                    }
                    let best = ups
                        .iter()
                        .copied()
                        .max_by(|a, b| max_ups[*a].partial_cmp(&max_ups[*b]).unwrap_or(std::cmp::Ordering::Equal));
                    for up in ups {
                        if Some(*up) != best {
                            hack[*up] = hack_order + 1;
                        }
                    }
                    if let Some(best_idx) = best {
                        cur = best_idx;
                    } else {
                        break;
                    }
                    hack_order += 1;
                }
            }
            for idx in 0..links.len() {
                if hack[idx] == 0 {
                    if let Some(ds) = downstream_of[idx] {
                        hack[idx] = hack[ds] + 1;
                    } else {
                        hack[idx] = 1;
                    }
                }
            }

            let mut trib_id = vec![0i64; links.len()];
            let mut next_trib = 1i64;
            let mut pending: Vec<usize> = upstream_of.iter().map(|u| u.len()).collect();
            let mut stack: Vec<usize> = (0..links.len()).filter(|&i| pending[i] == 0).collect();
            while let Some(idx) = stack.pop() {
                if trib_id[idx] == 0 {
                    trib_id[idx] = next_trib;
                    next_trib += 1;
                }
                if let Some(ds) = downstream_of[idx] {
                    if trib_id[ds] == 0 || max_ups[idx] > max_ups[ds] {
                        trib_id[ds] = trib_id[idx];
                    }
                    pending[ds] -= 1;
                    if pending[ds] == 0 {
                        stack.push(ds);
                    }
                }
            }

            let mut outlet_id = vec![0i64; links.len()];
            for (i, outlet) in outlets.iter().enumerate() {
                let mut stack = vec![*outlet];
                while let Some(idx) = stack.pop() {
                    if outlet_id[idx] != 0 {
                        continue;
                    }
                    outlet_id[idx] = (i + 1) as i64;
                    for up in &upstream_of[idx] {
                        stack.push(*up);
                    }
                }
            }

            let output_confluences = args.get("confluences_output").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| output.replace(".shp", "_confluences.shp"));
            let output_outlets = args.get("outlets_output").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| output.replace(".shp", "_outlets.shp"));
            let output_heads = args.get("channel_heads_output").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| output.replace(".shp", "_channel_heads.shp"));

            let mut out_lines = Layer::new("stream_network_analysis").with_geom_type(GeometryType::LineString);
            for field in [
                ("FID", FieldType::Integer),
                ("TUCL", FieldType::Float),
                ("MAXUPSDIST", FieldType::Float),
                ("MIN_ELEV", FieldType::Float),
                ("MAX_ELEV", FieldType::Float),
                ("OUTLET", FieldType::Integer),
                ("HORTON", FieldType::Integer),
                ("STRAHLER", FieldType::Integer),
                ("SHREVE", FieldType::Integer),
                ("HACK", FieldType::Integer),
                ("DIST2MOUTH", FieldType::Float),
                ("DS_NODES", FieldType::Integer),
                ("IS_OUTLET", FieldType::Integer),
                ("DS_LINK_ID", FieldType::Integer),
                ("MAINSTEM", FieldType::Integer),
                ("TRIB_ID", FieldType::Integer),
            ] {
                out_lines.add_field(FieldDef::new(field.0, field.1));
            }
            for (idx, link) in links.iter().enumerate() {
                out_lines.add_feature(
                    Some(Geometry::line_string(link.geom.clone())),
                    &[
                        ("FID", FieldValue::Integer((idx + 1) as i64)),
                        ("TUCL", FieldValue::Float(tucl[idx])),
                        ("MAXUPSDIST", FieldValue::Float(max_ups[idx])),
                        ("MIN_ELEV", FieldValue::Float(link.min_elev)),
                        ("MAX_ELEV", FieldValue::Float(link.max_elev)),
                        ("OUTLET", FieldValue::Integer(outlet_id[idx])),
                        ("HORTON", FieldValue::Integer(horton[idx])),
                        ("STRAHLER", FieldValue::Integer(strahler[idx])),
                        ("SHREVE", FieldValue::Integer(shreve[idx])),
                        ("HACK", FieldValue::Integer(hack[idx])),
                        ("DIST2MOUTH", FieldValue::Float(dist2mouth[idx])),
                        ("DS_NODES", FieldValue::Integer(ds_nodes[idx])),
                        ("IS_OUTLET", FieldValue::Integer(if downstream_of[idx].is_none() { 1 } else { 0 })),
                        ("DS_LINK_ID", FieldValue::Integer(downstream_of[idx].map(|v| (v + 1) as i64).unwrap_or(0))),
                        ("MAINSTEM", FieldValue::Integer(mainstream[idx])),
                        ("TRIB_ID", FieldValue::Integer(trib_id[idx])),
                    ],
                )
                .map_err(|e| ToolError::Execution(format!("failed building stream analysis output: {}", e)))?;
            }

            let mut confluences = Layer::new("confluences").with_geom_type(GeometryType::Point);
            confluences.add_field(FieldDef::new("FID", FieldType::Integer));
            let mut outlets_layer = Layer::new("outlets").with_geom_type(GeometryType::Point);
            outlets_layer.add_field(FieldDef::new("FID", FieldType::Integer));
            let mut heads_layer = Layer::new("channel_heads").with_geom_type(GeometryType::Point);
            heads_layer.add_field(FieldDef::new("FID", FieldType::Integer));

            let mut node_degree_in: HashMap<(i64, i64), usize> = HashMap::new();
            for ds in &downstream_of {
                if let Some(d) = ds {
                    for node in &link_key_nodes[*d] {
                        *node_degree_in.entry(*node).or_insert(0) += 1;
                    }
                }
            }
            let mut fid = 1i64;
            for (key, coord) in &node_coord {
                let in_deg = node_degree_in.get(key).copied().unwrap_or(0);
                if in_deg >= 2 {
                    confluences
                        .add_feature(Some(Geometry::point(coord.x, coord.y)), &[("FID", FieldValue::Integer(fid))])
                        .map_err(|e| ToolError::Execution(format!("failed building confluence output: {}", e)))?;
                    fid += 1;
                }
            }
            fid = 1;
            for outlet in &outlets {
                let c = node_coord.get(&links[*outlet].down_key).cloned().unwrap();
                outlets_layer
                    .add_feature(Some(Geometry::point(c.x, c.y)), &[("FID", FieldValue::Integer(fid))])
                    .map_err(|e| ToolError::Execution(format!("failed building outlet output: {}", e)))?;
                fid += 1;
            }
            fid = 1;
            for idx in 0..links.len() {
                if upstream_of[idx].is_empty() {
                    let c = node_coord.get(&links[idx].up_key).cloned().unwrap();
                    heads_layer
                        .add_feature(Some(Geometry::point(c.x, c.y)), &[("FID", FieldValue::Integer(fid))])
                        .map_err(|e| ToolError::Execution(format!("failed building channel head output: {}", e)))?;
                    fid += 1;
                }
            }

            ensure_parent_dir(&output)?;
            ensure_parent_dir(&output_confluences)?;
            ensure_parent_dir(&output_outlets)?;
            ensure_parent_dir(&output_heads)?;
            let main_path = write_vector(&out_lines, &output)?;
            let confluences_path = write_vector(&confluences, &output_confluences)?;
            let outlets_path = write_vector(&outlets_layer, &output_outlets)?;
            let heads_path = write_vector(&heads_layer, &output_heads)?;

            let mut outputs = BTreeMap::new();
            outputs.insert("path".to_string(), json!(main_path));
            outputs.insert("confluences".to_string(), json!(confluences_path));
            outputs.insert("outlets".to_string(), json!(outlets_path));
            outputs.insert("channel_heads".to_string(), json!(heads_path));
            Ok(ToolRunResult { outputs, ..Default::default() })
        }
        _ => StrahlerStreamOrderTool.run(args, ctx),
    }
}

impl Tool for HortonStreamOrderTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "horton_stream_order",
            display_name: "Horton Stream Order",
            summary: "Assigns Horton stream order to stream cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "d8_pntr", description: "D8 flow pointer raster", required: true },
                ToolParamSpec { name: "streams_raster", description: "Stream raster", required: true },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI-style pointer", required: false },
                ToolParamSpec { name: "zero_background", description: "Assign zero to background", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("d8_pntr".to_string(), json!("d8_pointer.tif"));
        defaults.insert("streams_raster".to_string(), json!("streams.tif"));

        ToolManifest {
            id: "horton_stream_order".to_string(),
            display_name: "Horton Stream Order".to_string(),
            summary: "Assigns Horton stream order to stream cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["stream_network".to_string(), "stream_order".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "d8_pntr")?;
        parse_raster_path_arg(args, "streams_raster")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        // Start with Strahler and then overwrite each main-stem path by outlet order.
        let (pntr, streams, output_path, esri_style, _zero_background) = parse_d8_stream_inputs(args)?;

        let mut strahler_args = args.clone();
        strahler_args.remove("output");
        let strahler_res = StrahlerStreamOrderTool.run(&strahler_args, ctx)?;
        let strahler_path = strahler_res
            .outputs
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Execution("missing strahler output path".to_string()))?;
        let mut out = D8Core::load_raster(strahler_path)?;

        let rows = pntr.rows;
        let cols = pntr.cols;
        let pntr_matches = D8Core::build_pntr_matches(esri_style);
        let inflowing = D8Core::inflowing_vals(esri_style);
        let lengths = grid_lengths(&pntr);

        let mut far = vec![vec![0.0f64; cols]; rows];
        let mut num_inflowing = vec![vec![-1i8; cols]; rows];
        let mut stack = Vec::new();
        let inflow_counts: Vec<i8> = (0..rows * cols)
            .into_par_iter()
            .map(|idx| {
                let row = idx / cols;
                let col = idx % cols;
                if streams.get(0, row as isize, col as isize) > 0.0 {
                    D8Core::count_inflowing_cells(
                        &streams,
                        &pntr,
                        row as isize,
                        col as isize,
                        &inflowing,
                    )
                } else {
                    -1
                }
            })
            .collect();
        for row in 0..rows {
            for col in 0..cols {
                let c = inflow_counts[row * cols + col];
                num_inflowing[row][col] = c;
                if c == 0 {
                    stack.push((row, col));
                }
            }
        }
        while let Some((row, col)) = stack.pop() {
            if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                if streams.get(0, rn as isize, cn as isize) > 0.0 {
                    far[rn][cn] = far[rn][cn].max(far[row][col] + lengths[idx]);
                    num_inflowing[rn][cn] -= 1;
                    if num_inflowing[rn][cn] == 0 {
                        stack.push((rn, cn));
                    }
                }
            }
        }

        for row in 0..rows {
            for col in 0..cols {
                let is_outlet = streams.get(0, row as isize, col as isize) > 0.0
                    && downstream_cell(&pntr, row, col, &pntr_matches)
                        .map(|(rn, cn, _)| streams.get(0, rn as isize, cn as isize) <= 0.0)
                        .unwrap_or(true);
                if !is_outlet {
                    continue;
                }
                let outlet_order = out.get(0, row as isize, col as isize);
                let mut y = row as isize;
                let mut x = col as isize;
                loop {
                    out.set_unchecked(0, y, x, outlet_order);
                    let mut best = None;
                    let mut best_dist = -1.0;
                    for i in 0..8 {
                        let yn = y + D8Core::D_Y[i];
                        let xn = x + D8Core::D_X[i];
                        if yn < 0 || xn < 0 || yn >= rows as isize || xn >= cols as isize {
                            continue;
                        }
                        if streams.get(0, yn, xn) > 0.0 && pntr.get(0, yn, xn) == inflowing[i] {
                            let d = far[yn as usize][xn as usize];
                            if d > best_dist {
                                best_dist = d;
                                best = Some((yn, xn));
                            }
                        }
                    }
                    if let Some((yn, xn)) = best {
                        y = yn;
                        x = xn;
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
    }
}

impl Tool for HackStreamOrderTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "hack_stream_order",
            display_name: "Hack Stream Order",
            summary: "Assigns Hack stream order to stream cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "d8_pntr", description: "D8 flow pointer raster", required: true },
                ToolParamSpec { name: "streams_raster", description: "Stream raster", required: true },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI-style pointer", required: false },
                ToolParamSpec { name: "zero_background", description: "Assign zero to background", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "hack_stream_order".to_string(),
            display_name: "Hack Stream Order".to_string(),
            summary: "Assigns Hack stream order to stream cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["stream_network".to_string(), "stream_order".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "d8_pntr")?;
        parse_raster_path_arg(args, "streams_raster")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
        let rows = pntr.rows;
        let cols = pntr.cols;
        let nodata = streams.nodata;
        let background = if zero_background { 0.0 } else { nodata };
        let pntr_nodata = pntr.nodata;
        let inflowing = D8Core::inflowing_vals(esri_style);
        let pntr_matches = D8Core::build_pntr_matches(esri_style);
        let lengths = grid_lengths(&pntr);

        let mut far = vec![vec![0.0f64; cols]; rows];
        let mut num_inflowing = vec![vec![-1i8; cols]; rows];
        let mut stack = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                if streams.get(0, row as isize, col as isize) > 0.0 {
                    let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                    num_inflowing[row][col] = c;
                    if c == 0 {
                        stack.push((row, col));
                    }
                }
            }
        }
        while let Some((row, col)) = stack.pop() {
            if let Some((rn, cn, idx)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                if streams.get(0, rn as isize, cn as isize) > 0.0 {
                    far[rn][cn] = far[rn][cn].max(far[row][col] + lengths[idx]);
                    num_inflowing[rn][cn] -= 1;
                    if num_inflowing[rn][cn] == 0 {
                        stack.push((rn, cn));
                    }
                }
            }
        }

        let mut out = streams.clone();
        out.data_type = DataType::I16;
        for row in 0..rows {
            for col in 0..cols {
                if streams.get(0, row as isize, col as isize) > 0.0 {
                    out.set_unchecked(0, row as isize, col as isize, 0.0);
                } else if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                    out.set_unchecked(0, row as isize, col as isize, background);
                } else {
                    out.set_unchecked(0, row as isize, col as isize, nodata);
                }
            }
        }

        for row in 0..rows {
            for col in 0..cols {
                let is_outlet = streams.get(0, row as isize, col as isize) > 0.0
                    && downstream_cell(&pntr, row, col, &pntr_matches)
                        .map(|(rn, cn, _)| streams.get(0, rn as isize, cn as isize) <= 0.0)
                        .unwrap_or(true);
                if !is_outlet {
                    continue;
                }
                let mut q: Vec<(isize, isize, f64)> = vec![(row as isize, col as isize, 1.0)];
                while let Some((y, x, ord)) = q.pop() {
                    if out.get(0, y, x) > 0.0 && out.get(0, y, x) <= ord {
                        continue;
                    }
                    out.set_unchecked(0, y, x, ord);
                    let mut best = None;
                    let mut best_dist = -1.0;
                    let mut ups = Vec::new();
                    for i in 0..8 {
                        let yn = y + D8Core::D_Y[i];
                        let xn = x + D8Core::D_X[i];
                        if yn < 0 || xn < 0 || yn >= rows as isize || xn >= cols as isize {
                            continue;
                        }
                        if streams.get(0, yn, xn) > 0.0 && pntr.get(0, yn, xn) == inflowing[i] {
                            let d = far[yn as usize][xn as usize];
                            if d > best_dist {
                                best_dist = d;
                                best = Some((yn, xn));
                            }
                            ups.push((yn, xn));
                        }
                    }
                    for (yn, xn) in ups {
                        if Some((yn, xn)) == best {
                            q.push((yn, xn, ord));
                        } else {
                            q.push((yn, xn, ord + 1.0));
                        }
                    }
                }
            }
        }
        Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
    }
}

impl Tool for ShreveStreamMagnitudeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "shreve_stream_magnitude",
            display_name: "Shreve Stream Magnitude",
            summary: "Calculates Shreve stream magnitude.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "d8_pntr", description: "D8 flow pointer raster", required: true },
                ToolParamSpec { name: "streams_raster", description: "Stream raster", required: true },
                ToolParamSpec { name: "esri_pntr", description: "Use ESRI-style pointer", required: false },
                ToolParamSpec { name: "zero_background", description: "Assign zero to background", required: false },
                ToolParamSpec { name: "output", description: "Output raster path", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "shreve_stream_magnitude".to_string(),
            display_name: "Shreve Stream Magnitude".to_string(),
            summary: "Calculates Shreve stream magnitude.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["stream_network".to_string(), "stream_magnitude".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "d8_pntr")?;
        parse_raster_path_arg(args, "streams_raster")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (pntr, streams, output_path, esri_style, zero_background) = parse_d8_stream_inputs(args)?;
        let rows = pntr.rows;
        let cols = pntr.cols;
        let nodata = streams.nodata;
        let pntr_nodata = pntr.nodata;
        let background = if zero_background { 0.0 } else { nodata };
        let inflowing = D8Core::inflowing_vals(esri_style);
        let pntr_matches = D8Core::build_pntr_matches(esri_style);

        let mut out = streams.clone();
        out.data_type = DataType::I32;
        let mut num_inflowing = vec![vec![-1i8; cols]; rows];
        let mut stack = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                if streams.get(0, row as isize, col as isize) > 0.0 {
                    let c = D8Core::count_inflowing_cells(&streams, &pntr, row as isize, col as isize, &inflowing);
                    num_inflowing[row][col] = c;
                    if c == 0 {
                        out.set_unchecked(0, row as isize, col as isize, 1.0);
                        stack.push((row, col));
                    } else {
                        out.set_unchecked(0, row as isize, col as isize, 0.0);
                    }
                } else if pntr.get(0, row as isize, col as isize) != pntr_nodata {
                    out.set_unchecked(0, row as isize, col as isize, background);
                } else {
                    out.set_unchecked(0, row as isize, col as isize, nodata);
                }
            }
        }
        while let Some((row, col)) = stack.pop() {
            let mag = out.get(0, row as isize, col as isize).max(1.0);
            if let Some((rn, cn, _)) = downstream_cell(&pntr, row, col, &pntr_matches) {
                if streams.get(0, rn as isize, cn as isize) > 0.0 {
                    let cur = out.get(0, rn as isize, cn as isize);
                    out.set_unchecked(0, rn as isize, cn as isize, cur + mag);
                    num_inflowing[rn][cn] -= 1;
                    if num_inflowing[rn][cn] == 0 {
                        stack.push((rn, cn));
                    }
                }
            }
        }
        Ok(D8Core::build_result(D8Core::write_or_store_output(out, output_path)?))
    }
}

// Generate similarly-shaped Tool trait implementations that dispatch to the
// concrete shared stream-tool runner.
macro_rules! create_stream_tool_impl {
    ($tool_name:ident, $id:expr, $display_name:expr, $summary:expr) => {
        impl Tool for $tool_name {
            fn metadata(&self) -> ToolMetadata {
                ToolMetadata {
                    id: $id,
                    display_name: $display_name,
                    summary: $summary,
                    category: ToolCategory::Raster,
                    license_tier: LicenseTier::Open,
                    params: vec![
                        ToolParamSpec { name: "d8_pntr", description: "D8 flow pointer raster", required: true },
                        ToolParamSpec { name: "streams_raster", description: "Stream raster", required: true },
                        ToolParamSpec { name: "output", description: "Output raster path", required: false },
                    ],
                }
            }

            fn manifest(&self) -> ToolManifest {
                ToolManifest {
                    id: $id.to_string(),
                    display_name: $display_name.to_string(),
                    summary: $summary.to_string(),
                    category: ToolCategory::Raster,
                    license_tier: LicenseTier::Open,
                    params: vec![],
                    defaults: ToolArgs::new(),
                    examples: vec![],
                    tags: vec!["stream_network".to_string()],
                    stability: ToolStability::Experimental,
                }
            }

            fn validate(&self, _args: &ToolArgs) -> Result<(), ToolError> {
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                run_stream_tool_fallback($id, args, ctx)
            }
        }
    };
}

create_stream_tool_impl!(TopologicalStreamOrderTool, "topological_stream_order", "Topological Stream Order", "Assigns topological stream order based on link count.");
create_stream_tool_impl!(StreamLinkIdentifierTool, "stream_link_identifier", "Stream Link Identifier", "Assigns unique ID to each stream link.");
create_stream_tool_impl!(StreamLinkClassTool, "stream_link_class", "Stream Link Class", "Classifies stream links as interior, exterior, or source.");
create_stream_tool_impl!(StreamLinkLengthTool, "stream_link_length", "Stream Link Length", "Calculates total length for each stream link.");
create_stream_tool_impl!(StreamLinkSlopeTool, "stream_link_slope", "Stream Link Slope", "Calculates average slope for each stream link.");
create_stream_tool_impl!(StreamSlopeContinuousTool, "stream_slope_continuous", "Stream Slope Continuous", "Calculates slope value for each stream cell.");
create_stream_tool_impl!(DistanceToOutletTool, "distance_to_outlet", "Distance to Outlet", "Calculates downstream distance to outlet for each stream cell.");
create_stream_tool_impl!(LengthOfUpstreamChannelsTool, "length_of_upstream_channels", "Length of Upstream Channels", "Calculates total upstream channel length.");
create_stream_tool_impl!(FindMainStemTool, "find_main_stem", "Find Main Stem", "Identifies main stem of stream network.");
create_stream_tool_impl!(FarthestChannelHeadTool, "farthest_channel_head", "Farthest Channel Head", "Calculates distance to most distant channel head.");
create_stream_tool_impl!(TributaryIdentifierTool, "tributary_identifier", "Tributary Identifier", "Assigns unique ID to each tributary.");
create_stream_tool_impl!(RemoveShortStreamsTool, "remove_short_streams", "Remove Short Streams", "Removes stream links shorter than minimum length.");
create_stream_tool_impl!(ExtractStreamsTool, "extract_streams", "Extract Streams", "Extracts streams based on flow accumulation threshold.");
create_stream_tool_impl!(ExtractValleysTool, "extract_valleys", "Extract Valleys", "Extracts valleys from DEM.");
create_stream_tool_impl!(RasterStreamsToVectorTool, "raster_streams_to_vector", "Raster Streams to Vector", "Converts raster stream network to vector.");
create_stream_tool_impl!(RasterizeStreamsTool, "rasterize_streams", "Rasterize Streams", "Rasterizes vector stream network.");
create_stream_tool_impl!(LongProfileTool, "long_profile", "Long Profile", "Creates longitudinal stream profile.");
create_stream_tool_impl!(LongProfileFromPointsTool, "long_profile_from_points", "Long Profile from Points", "Creates long profile from vector points.");
create_stream_tool_impl!(RepairStreamVectorTopologyTool, "repair_stream_vector_topology", "Repair Stream Vector Topology", "Repairs topology of vector stream network.");
create_stream_tool_impl!(VectorStreamNetworkAnalysisTool, "vector_stream_network_analysis", "Vector Stream Network Analysis", "Comprehensive vector stream network analysis.");

// ──────────────────────────────────────────────────────────────────────────────
// Burn Streams Tool
// ──────────────────────────────────────────────────────────────────────────────

impl Tool for BurnStreamsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "burn_streams",
            display_name: "Burn Streams",
            summary: "Burns a stream network into a DEM by decreasing stream-cell elevations, optionally applying a distance gradient away from streams.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem",           description: "Input DEM raster.", required: true },
                ToolParamSpec { name: "streams",       description: "Input streams vector.", required: true },
                ToolParamSpec { name: "decrement_value", description: "Elevation decrement applied to stream cells (default 5.0).", required: false },
                ToolParamSpec { name: "gradient_distance", description: "Gradient distance in grid cells (0 = flat decrement only; default 5).", required: false },
                ToolParamSpec { name: "output",        description: "Output burned DEM path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("decrement_value".to_string(), json!(5.0));
        defaults.insert("gradient_distance".to_string(), json!(5));
        ToolManifest {
            id: "burn_streams".to_string(),
            display_name: "Burn Streams".to_string(),
            summary: "Burns a stream network into a DEM by decreasing stream-cell elevations.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["stream_network".to_string(), "dem_preprocessing".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")?;
        parse_vector_path_arg(args, "streams")?;
        let decrement = args.get("decrement_value").and_then(|v| v.as_f64()).unwrap_or(5.0);
        if !decrement.is_finite() || decrement < 0.0 {
            return Err(ToolError::Validation("decrement_value must be a non-negative finite number".to_string()));
        }
        let gd = args.get("gradient_distance").and_then(|v| v.as_i64()).unwrap_or(5);
        if gd < 0 {
            return Err(ToolError::Validation("gradient_distance must be >= 0".to_string()));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path   = parse_raster_path_arg(args, "dem")?;
        let streams_path = parse_vector_path_arg(args, "streams")?;
        let decrement  = args.get("decrement_value").and_then(|v| v.as_f64()).unwrap_or(5.0);
        let grad_dist  = args.get("gradient_distance").and_then(|v| v.as_i64()).unwrap_or(5);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading DEM");
        let dem = D8Core::load_raster(&dem_path)?;
        let rows = dem.rows;
        let cols = dem.cols;
        let nodata = dem.nodata;
        let res_x = dem.cell_size_x;
        let res_y = dem.cell_size_y.abs();
        let grid_res = (res_x + res_y) / 2.0;

        // ── rasterize stream vector ──────────────────────────────────────────
        ctx.progress.info("rasterizing streams");
        let streams_args = {
            let mut a = ToolArgs::new();
            a.insert("streams".to_string(), json!(streams_path));
            a.insert("base_raster".to_string(), json!(dem_path));
            a.insert("zero_background".to_string(), json!(true));
            a.insert("use_feature_id".to_string(), json!(false));
            a
        };
        let streams_result = RasterizeStreamsTool.run(&streams_args, ctx)?;
        let streams_raster_path = streams_result
            .outputs
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Execution("rasterize_streams returned no path".to_string()))?;
        let streams_raster = D8Core::load_raster(streams_raster_path)?;

        // ── burn ─────────────────────────────────────────────────────────────
        let mut output = dem.clone();
        if grad_dist <= 0 {
            ctx.progress.info("applying flat elevation decrement");
            for row in 0..rows {
                for col in 0..cols {
                    let z = dem.get(0, row as isize, col as isize);
                    if z == nodata { continue; }
                    let s = streams_raster.get(0, row as isize, col as isize);
                    if s > 0.0 {
                        let _ = output.set(0, row as isize, col as isize, z - decrement);
                    }
                }
            }
        } else {
            // Calculate Euclidean distance from streams using a simple BFS/scan approach
            ctx.progress.info("computing euclidean distance from streams");
            let large = f64::INFINITY;
            let mut dist = vec![large; rows * cols];

            // Seed stream cells at distance 0
            let mut queue: std::collections::VecDeque<(isize, isize)> = std::collections::VecDeque::new();
            for row in 0..rows {
                for col in 0..cols {
                    let s = streams_raster.get(0, row as isize, col as isize);
                    if s > 0.0 {
                        dist[row * cols + col] = 0.0;
                        queue.push_back((row as isize, col as isize));
                    }
                }
            }

            // Breadth-first propagation using Meijster-style approximation (row/col distances)
            let dx = [1isize, -1, 0, 0, 1, -1, 1, -1];
            let dy = [0isize, 0, 1, -1, 1, 1, -1, -1];
            let dd = [res_x, res_x, res_y, res_y,
                      (res_x * res_x + res_y * res_y).sqrt(),
                      (res_x * res_x + res_y * res_y).sqrt(),
                      (res_x * res_x + res_y * res_y).sqrt(),
                      (res_x * res_x + res_y * res_y).sqrt()];

            while let Some((r, c)) = queue.pop_front() {
                let d = dist[(r as usize) * cols + (c as usize)];
                for k in 0..8 {
                    let nr = r + dy[k];
                    let nc = c + dx[k];
                    if nr < 0 || nc < 0 || nr >= rows as isize || nc >= cols as isize { continue; }
                    let idx = (nr as usize) * cols + (nc as usize);
                    let nd = d + dd[k];
                    if nd < dist[idx] {
                        dist[idx] = nd;
                        queue.push_back((nr, nc));
                    }
                }
            }

            let dist_threshold = grad_dist as f64 * grid_res;
            ctx.progress.info("applying gradient decrement");
            for row in 0..rows {
                for col in 0..cols {
                    let z = dem.get(0, row as isize, col as isize);
                    if z == nodata { continue; }
                    let d = dist[row * cols + col];
                    // burned_dem = dem + clamp((d - threshold) / threshold, min=−1, max=0) * decrement
                    let factor = ((d - dist_threshold) / dist_threshold).min(0.0).max(-1.0);
                    let _ = output.set(0, row as isize, col as isize, z + factor * decrement);
                }
            }
        }

        ctx.progress.info("writing output");
        let locator = D8Core::write_or_store_output(output, output_path)?;
        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Horton Ratios Tool
// ──────────────────────────────────────────────────────────────────────────────

impl Tool for HortonRatiosTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "horton_ratios",
            display_name: "Horton Ratios",
            summary: "Calculates Horton's bifurcation, length, drainage-area, and slope ratios for a stream network.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem",            description: "Input DEM raster.", required: true },
                ToolParamSpec { name: "streams_raster", description: "Binary stream raster (positive = stream).", required: true },
                ToolParamSpec { name: "output",         description: "Output text report path (optional).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        ToolManifest {
            id: "horton_ratios".to_string(),
            display_name: "Horton Ratios".to_string(),
            summary: "Calculates Horton bifurcation, length, drainage-area, and slope ratios.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults: ToolArgs::new(),
            examples: vec![],
            tags: vec!["stream_network".to_string(), "geomorphometry".to_string(), "statistics".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        parse_raster_path_arg(args, "dem")?;
        parse_raster_path_arg(args, "streams_raster")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let dem_path     = parse_raster_path_arg(args, "dem")?;
        let streams_path = parse_raster_path_arg(args, "streams_raster")?;
        let report_path  = parse_optional_output_path(args, "output")?;

        ctx.progress.info("reading DEM and streams");
        let dem     = D8Core::load_raster(&dem_path)?;
        let streams = D8Core::load_raster(&streams_path)?;
        let rows = dem.rows;
        let cols = dem.cols;
        let nodata = dem.nodata;

        // ── D8 pointer ────────────────────────────────────────────────────────
        ctx.progress.info("computing D8 pointer");
        let pntr = {
            let mut a = ToolArgs::new();
            a.insert("dem".to_string(), json!(dem_path));
            let pntr_result = D8PointerTool.run(&a, ctx)?;
            let pntr_path = pntr_result.outputs.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Execution("d8_pointer returned no path".to_string()))?;
            D8Core::load_raster(pntr_path)?
        };

        // ── Strahler order ────────────────────────────────────────────────────
        ctx.progress.info("computing Strahler order");
        let strahler = {
            let mut a = ToolArgs::new();
            a.insert("d8_pntr".to_string(), json!(dem_path)); // will be overridden
            // Serialize pntr to memory store
            let mid = memory_store::put_raster(pntr.clone());
            let mp = memory_store::make_raster_memory_path(&mid);
            a.insert("d8_pntr".to_string(), json!(mp));
            let sid = memory_store::put_raster(streams.clone());
            let sp = memory_store::make_raster_memory_path(&sid);
            a.insert("streams_raster".to_string(), json!(sp));
            a.insert("esri_pntr".to_string(), json!(false));
            a.insert("zero_background".to_string(), json!(true));
            let r = StrahlerStreamOrderTool.run(&a, ctx)?;
            let path = r.outputs.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Execution("strahler returned no path".to_string()))?;
            D8Core::load_raster(path)?
        };

        // ── Stream link tracing ───────────────────────────────────────────────
        ctx.progress.info("tracing stream links");
        let esri_style = false;
        let inflowing_vals = D8Core::inflowing_vals(esri_style);
        const D_X: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
        const D_Y: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut stream_link_id = vec![0i32; rows * cols];
        let mut num_inflowing  = vec![-1i8; rows * cols];

        let mut channel_heads: Vec<(usize, usize)> = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let so = strahler.get(0, row as isize, col as isize);
                if so > 0.0 {
                    let mut inflowing = 0i8;
                    for n in 0..8usize {
                        let y = row as isize + D_Y[n];
                        let x = col as isize + D_X[n];
                        let fd = pntr.get(0, y, x);
                        if fd == inflowing_vals[n] && streams.get(0, y, x) > 0.0 {
                            inflowing += 1;
                        }
                    }
                    let idx = row * cols + col;
                    num_inflowing[idx] = inflowing;
                    if inflowing == 0 {
                        channel_heads.push((row, col));
                    }
                }
            }
        }

        let mut link_id = 0i32;
        let mut heads = std::collections::VecDeque::from(channel_heads);
        while let Some((row, col)) = heads.pop_front() {
            link_id += 1;
            let mut so = strahler.get(0, row as isize, col as isize);
            let mut rn = row as isize;
            let mut cn = col as isize;
            loop {
                stream_link_id[(rn as usize) * cols + (cn as usize)] = link_id;
                let fd = pntr.get(0, rn, cn);
                if fd <= 0.0 { break; }
                let n = (fd as u64).trailing_zeros() as usize;
                if n >= 8 { break; }
                rn += D_Y[n];
                cn += D_X[n];
                if rn < 0 || cn < 0 || rn >= rows as isize || cn >= cols as isize { break; }
                let nso = strahler.get(0, rn, cn);
                if nso <= 0.0 { break; }
                let nidx = (rn as usize) * cols + (cn as usize);
                if stream_link_id[nidx] > 0 { break; }
                num_inflowing[nidx] -= 1;
                if num_inflowing[nidx] >= 1 { break; }
                if nso != so {
                    so = nso;
                    link_id += 1;
                }
            }
        }

        // ── D8 flow accumulation (cells) ──────────────────────────────────────
        ctx.progress.info("computing D8 flow accumulation");
        let d8_accum = {
            let pntr_mid = memory_store::put_raster(pntr.clone());
            let pntr_mem = memory_store::make_raster_memory_path(&pntr_mid);
            let mut a = ToolArgs::new();
            a.insert("input".to_string(), json!(pntr_mem));
            a.insert("input_is_pointer".to_string(), json!(true));
            a.insert("out_type".to_string(), json!("ca"));
            a.insert("log_transform".to_string(), json!(false));
            a.insert("clip".to_string(), json!(false));
            let r = D8FlowAccumTool.run(&a, ctx)?;
            let p = r.outputs.get("path").and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Execution("d8_flow_accum returned no path".to_string()))?;
            D8Core::load_raster(p)?
        };

        // ── Stream link slope ─────────────────────────────────────────────────
        ctx.progress.info("computing stream link slope");
        let link_slope_raster = {
            let pntr_mid = memory_store::put_raster(pntr.clone());
            let pntr_mem = memory_store::make_raster_memory_path(&pntr_mid);
            let link_mid = {
                let mut link_raster = dem.clone();
                link_raster.data_type = wbraster::DataType::F64;
                for r in 0..rows {
                    for c in 0..cols {
                        let v = stream_link_id[r * cols + c] as f64;
                        let _ = link_raster.set(0, r as isize, c as isize, v);
                    }
                }
                memory_store::put_raster(link_raster)
            };
            let link_mem = memory_store::make_raster_memory_path(&link_mid);
            let mut a = ToolArgs::new();
            a.insert("d8_pntr".to_string(), json!(pntr_mem));
            a.insert("streams_id_raster".to_string(), json!(link_mem));
            a.insert("dem".to_string(), json!(dem_path));
            a.insert("esri_pntr".to_string(), json!(false));
            a.insert("zero_background".to_string(), json!(true));
            let r = StreamLinkSlopeTool.run(&a, ctx)?;
            let p = r.outputs.get("path").and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Execution("stream_link_slope returned no path".to_string()))?;
            D8Core::load_raster(p)?
        };

        // ── Stream link length ────────────────────────────────────────────────
        ctx.progress.info("computing stream link length");
        let link_length_raster = {
            let pntr_mid = memory_store::put_raster(pntr.clone());
            let pntr_mem = memory_store::make_raster_memory_path(&pntr_mid);
            let link_mid = {
                let mut link_raster = dem.clone();
                link_raster.data_type = wbraster::DataType::F64;
                for r in 0..rows {
                    for c in 0..cols {
                        let v = stream_link_id[r * cols + c] as f64;
                        let _ = link_raster.set(0, r as isize, c as isize, v);
                    }
                }
                memory_store::put_raster(link_raster)
            };
            let link_mem = memory_store::make_raster_memory_path(&link_mid);
            let mut a = ToolArgs::new();
            a.insert("d8_pntr".to_string(), json!(pntr_mem));
            a.insert("streams_id_raster".to_string(), json!(link_mem));
            a.insert("esri_pntr".to_string(), json!(false));
            a.insert("zero_background".to_string(), json!(true));
            let r = StreamLinkLengthTool.run(&a, ctx)?;
            let p = r.outputs.get("path").and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Execution("stream_link_length returned no path".to_string()))?;
            D8Core::load_raster(p)?
        };

        // ── Aggregate per-link statistics ────────────────────────────────────
        ctx.progress.info("aggregating link statistics");
        let mut link_order:  HashMap<i32, i32>  = HashMap::new();
        let mut link_length: HashMap<i32, f64>  = HashMap::new();
        let mut link_area:   HashMap<i32, f64>  = HashMap::new();
        let mut link_slope:  HashMap<i32, f64>  = HashMap::new();
        let mut max_order = 0i32;

        for row in 0..rows {
            for col in 0..cols {
                let id = stream_link_id[row * cols + col];
                if id == 0 { continue; }
                let z = dem.get(0, row as isize, col as isize);
                if z == nodata { continue; }
                let order = strahler.get(0, row as isize, col as isize) as i32;
                if order > max_order { max_order = order; }
                link_order.insert(id, order);
                link_length.insert(id, link_length_raster.get(0, row as isize, col as isize));
                let area = d8_accum.get(0, row as isize, col as isize);
                let e = link_area.entry(id).or_insert(0.0);
                if area > *e { *e = area; }
                link_slope.insert(id, link_slope_raster.get(0, row as isize, col as isize));
            }
            ctx.progress.progress(0.85 * (row + 1) as f64 / rows as f64);
        }

        if max_order < 2 {
            return Err(ToolError::Execution(
                "stream network has fewer than 2 Strahler orders; cannot compute Horton ratios".to_string(),
            ));
        }

        // ── Linear regression helper (log-linear) ────────────────────────────
        let log_linear_slope = |xs: &[f64], ys_raw: &[f64]| -> f64 {
            let n = xs.len() as f64;
            let ys: Vec<f64> = ys_raw.iter().map(|y| y.ln()).collect();
            let sum_x:  f64 = xs.iter().sum();
            let sum_y:  f64 = ys.iter().sum();
            let sum_xy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
            let sum_xx: f64 = xs.iter().map(|x| x * x).sum();
            let denom = n * sum_xx - sum_x * sum_x;
            if denom.abs() < 1e-12 { return 999.0; }
            (n * sum_xy - sum_x * sum_y) / denom
        };

        let mut stream_num   = vec![0i32; max_order as usize];
        let mut total_length = vec![0.0f64; max_order as usize];
        let mut total_area   = vec![0.0f64; max_order as usize];
        let mut total_slope  = vec![0.0f64; max_order as usize];
        for (id, &order) in &link_order {
            let o = (order - 1) as usize;
            stream_num[o] += 1;
            if let Some(&l) = link_length.get(id) { total_length[o] += l; }
            if let Some(&a) = link_area.get(id)   { total_area[o] += a; }
            if let Some(&s) = link_slope.get(id)  { total_slope[o] += s; }
        }

        let xs: Vec<f64> = (1..=max_order as usize)
            .filter(|&o| stream_num[o - 1] > 0)
            .map(|o| o as f64)
            .collect();
        let y_counts:  Vec<f64> = xs.iter().map(|&x| stream_num[(x as usize) - 1] as f64).collect();
        let y_lengths: Vec<f64> = xs.iter().map(|&x| {
            let o = (x as usize) - 1;
            let n = stream_num[o];
            if n > 0 { total_length[o] / n as f64 } else { 1.0 }
        }).collect();
        let y_areas: Vec<f64> = xs.iter().map(|&x| {
            let o = (x as usize) - 1;
            let n = stream_num[o];
            if n > 0 { total_area[o] / n as f64 } else { 1.0 }
        }).collect();
        let y_slopes: Vec<f64> = xs.iter().map(|&x| {
            let o = (x as usize) - 1;
            let n = stream_num[o];
            if n > 0 { total_slope[o] / n as f64 } else { 1.0 }
        }).collect();

        let bifurcation_ratio = (-log_linear_slope(&xs, &y_counts)).exp();
        let length_ratio      = ( log_linear_slope(&xs, &y_lengths)).exp();
        let area_ratio        = ( log_linear_slope(&xs, &y_areas)).exp();
        let slope_ratio       = (-log_linear_slope(&xs, &y_slopes)).exp();

        ctx.progress.info(&format!(
            "Bifurcation ratio: {:.4}\nLength ratio: {:.4}\nArea ratio: {:.4}\nSlope ratio: {:.4}",
            bifurcation_ratio, length_ratio, area_ratio, slope_ratio
        ));

        let report_text = format!(
            "Horton Ratios\n\
             Bifurcation ratio (Rb): {:.6}\n\
             Stream-length ratio (Rl): {:.6}\n\
             Drainage-area ratio (Ra): {:.6}\n\
             Stream-slope ratio (Rs): {:.6}\n",
            bifurcation_ratio, length_ratio, area_ratio, slope_ratio
        );

        let mut outputs = BTreeMap::new();
        outputs.insert("bifurcation_ratio".to_string(), json!(bifurcation_ratio));
        outputs.insert("length_ratio".to_string(),      json!(length_ratio));
        outputs.insert("area_ratio".to_string(),        json!(area_ratio));
        outputs.insert("slope_ratio".to_string(),       json!(slope_ratio));
        outputs.insert("__wbw_type__".to_string(),      json!("tuple"));
        outputs.insert("items".to_string(), json!([bifurcation_ratio, length_ratio, area_ratio, slope_ratio]));

        if let Some(path) = report_path {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ToolError::Execution(format!("failed creating output dir: {e}")))?;
                }
            }
            std::fs::write(&path, &report_text)
                .map_err(|e| ToolError::Execution(format!("failed writing report: {e}")))?;
            outputs.insert("report_path".to_string(), json!(path.to_string_lossy().to_string()));
        }

        ctx.progress.progress(1.0);
        Ok(ToolRunResult { outputs, ..Default::default() })
    }
}
