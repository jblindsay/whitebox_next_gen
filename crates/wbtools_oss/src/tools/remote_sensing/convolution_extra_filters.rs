use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::color_math::{hsi2value, value2hsi, value2i};
use wbraster::{Raster, RasterFormat};

use super::color_support;
use crate::memory_store;

pub struct ScharrFilterTool;
pub struct RobertsCrossFilterTool;
pub struct LineDetectionFilterTool;
pub struct EmbossFilterTool;
pub struct UserDefinedWeightsFilterTool;

#[derive(Clone, Copy)]
enum ExtraOp {
    Scharr,
    Roberts,
    Line,
    Emboss,
    UserDefined,
}

impl ExtraOp {
    fn id(self) -> &'static str {
        match self {
            Self::Scharr => "scharr_filter",
            Self::Roberts => "roberts_cross_filter",
            Self::Line => "line_detection_filter",
            Self::Emboss => "emboss_filter",
            Self::UserDefined => "user_defined_weights_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Scharr => "Scharr Filter",
            Self::Roberts => "Roberts Cross Filter",
            Self::Line => "Line Detection Filter",
            Self::Emboss => "Emboss Filter",
            Self::UserDefined => "User Defined Weights Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Scharr => r#"The Scharr filter is an edge detection operator that improves upon the Sobel filter by using optimized kernel coefficients specifically designed to reduce directional bias and provide superior rotation invariance. The Scharr operator employs 3×3 convolution kernels with integer coefficients (3, 10, 3) that are empirically optimized for 0°, 45°, 90°, and 135° edge directions, delivering more accurate gradient estimation than traditional Sobel filters especially for circular features and rotated edges. The filter computes both horizontal and gradient magnitude simultaneously, enabling robust edge localization. Key advantages include superior accuracy for directional gradients, reduced rotational bias compared to Sobel, and efficient 3×3 kernel computation requiring minimal memory overhead. Output includes both magnitude and optional directional components. The Scharr filter excels in feature extraction, boundary detection, and quality assurance workflows requiring high directional accuracy. Use cases include extracting building footprints from aerial imagery, detecting road networks, delineating water boundaries with minimal distortion, and identifying geological lineaments in satellite data. The filter performs exceptionally well in urban mapping, agricultural boundary detection, and autonomous navigation applications. Output interpretation requires understanding that magnitude values represent edge strength—higher values indicate sharper transitions between distinct spectral classes. Directional components reveal predominant edge orientation (horizontal, diagonal, or vertical), useful for lineament analysis. For multi-band rasters, apply separately to each band or use a computed index. Background values appear dark in output; strong edges appear bright. RMSE comparison with reference edges validates filter performance. Scale the output to 0-255 for standard visualization or preserve floating-point for quantitative analysis."#,
            Self::Roberts => r#"The Roberts Cross filter is one of the earliest edge detection operators, employing a simple yet effective 2×2 diagonal cross-kernel pattern to compute image gradients with minimal computational overhead. This operator uses two orthogonal 2×2 matrices rotated 45° from horizontal-vertical alignment, creating cross-shaped convolution masks that detect edges emphasizing corners and diagonal transitions. The filter applies separate kernels for X and Y gradients, computing magnitude through the sum of absolute values or Euclidean norm. Key features include exceptional computational efficiency due to small 2×2 kernel size, minimal memory requirements, and fast processing on large raster datasets. The Roberts Cross is particularly effective for detecting fine-scale features and sharp transitions in high-resolution imagery. Primary use cases include rapid edge detection in time-critical applications, real-time video stream processing, preliminary boundary detection before advanced algorithms, and resource-constrained environments. Applications span aerial survey preprocessing, satellite imagery quality assessment, feature extraction for machine learning pipelines, and mobile GIS implementations. Output interpretation shows edge locations as high-magnitude pixels where spectral changes occur across the 2×2 neighborhood. Background regions typically display near-zero values; edges appear as bright linear features indicating boundaries between distinct land cover classes. The output is inherently sparse, containing edges only where gradients exceed computational precision thresholds. For multi-spectral images, apply independently to each band or compute a normalized difference index first. Edge thinning post-processing often follows Roberts application to refine output for vector conversion workflows."#,
            Self::Line => r#"The line detection filter is a specialized edge detector designed to enhance linear features—roads, rivers, powerlines, and geological lineaments—across four cardinal directions (horizontal, vertical, and both diagonals) by applying directional convolution kernels optimized for line connectivity and continuity. This filter employs 3×3 kernels that emphasize linear structures aligned with each cardinal direction while suppressing perpendicular noise and random feature variations. The directional approach separates detections by orientation, enabling downstream applications to distinguish horizontal infrastructure (pipelines, field boundaries) from vertical structures (tree rows, utility corridors). Key features include multi-directional decomposition generating separate magnitude and direction bands, adaptive sensitivity tuning for line width and contrast variations, and exceptional performance on subtle linear features embedded in complex terrain. Use cases span infrastructure mapping (roads, railways, powerlines), natural feature extraction (streams, ridges, faults), and land-use boundary delineation. Applications include vector conversion preprocessing for cadastral digitization, transportation network extraction from orthophotography, geological structure mapping from satellite imagery, and pattern recognition in remote sensing analysis. Output interpretation requires understanding that each directional component reveals line strength in that specific orientation—examine all four cardinal outputs to identify predominant feature directions. Higher magnitude values indicate stronger line continuity; low values suggest noise or breaks. The output is naturally sparse, highlighting only pixels representing linear features; background areas remain near-zero. Multi-directional output enables post-classification where roads (typically horizontal/vertical in built areas) are distinguished from natural lineaments (arbitrary orientation). Combine directional components to generate unified line map or analyze each direction separately for oriented structure mapping."#,
            Self::Emboss => r#"The emboss filter enhances edge features through directional shading, creating a distinctive three-dimensional relief appearance where edges appear as raised or depressed surfaces depending on directional lighting simulation. This filter applies a 3×3 kernel that combines edge detection with unidirectional illumination, typically simulating light from the upper-left quadrant, creating dramatic visual contrast at boundaries while preserving smooth regions. The emboss transformation subtracts weighted neighbor values in specific directions, producing output where highlights and shadows accentuate topographic and feature discontinuities. Key features include intuitive directional control allowing simulated light direction adjustment (eight-directional variants), natural incorporation of 3D perception enhancing visual interpretation, and robust performance across varied lighting conditions in source imagery. The emboss filter serves quality assurance, interpretative visualization, and feature boundary emphasis applications. Use cases include visual enhancement for geological interpretation where mineral boundaries become visible as relief patterns, aerial photograph interpretation improving subtle boundary visibility, cartographic production where embossed digital elevation models generate compelling relief maps, and archaeological feature detection where buried structures appear as subtle topographic variations. Output interpretation treats high values as illuminated surfaces and low values as shadows, creating perceptual depth that the human eye naturally interprets as relief. Embossed output typically requires scaling to 0-255 visualization range; raw output often contains negative values representing shadow areas. The effect is purely visual—emboss does not compute true illumination or create elevation derivatives. For multi-spectral imagery, apply to principal components or selected bands based on analytical objectives. Post-processing often includes contrast enhancement or tone mapping optimizing visual impact."#,
            Self::UserDefined => r#"The user-defined weights filter provides maximum flexibility for custom convolution analysis by accepting arbitrary weighted coefficients for neighborhood pixels, enabling implementation of specialized operators, domain-specific kernels, and research-grade algorithms without requiring tool modifications or specialized software. Users specify kernel dimensions (typically 3×3 or 5×5), assign floating-point weights to each position, and optionally designate edge-handling methods (reflection, wrapping, constant-fill). This filter applies the custom kernel across the entire raster through standard convolution mathematics: each output pixel equals the sum of weighted neighbors, enabling both traditional image processing filters and custom analytical kernels. Key features include complete customization for research applications, support for both enhancement and analysis operations, compatibility with normalized and unnormalized kernels, and preservation of floating-point precision throughout computation. Use cases span advanced spatial filtering for specialized spectral indices, implementation of experimental operators for algorithm validation, custom texture analysis kernels, weighted neighborhood aggregations for multi-criteria analysis, and standardized kernel application across diverse datasets. Applications include academic research prototyping, industry algorithm evaluation, regional customization of processing pipelines, and performance comparison studies. Output interpretation depends entirely on user-defined coefficients; different kernels produce fundamentally different results. Normalized kernels (sum of weights equals 1.0) preserve value ranges useful for smoothing; unnormalized kernels (typically summing to zero) emphasize differences useful for edge/derivative detection. Users must validate kernel properties—coefficient sign, magnitude, and sum—before production application. Documentation of kernel specifications is essential for reproducible workflows. Output value ranges depend on kernel design; documentation should specify expected output characteristics and scaling requirements."#,
        }
    }

    fn tags(self) -> Vec<String> {
        vec![
            "remote_sensing".to_string(),
            "raster".to_string(),
            "filter".to_string(),
            "convolution".to_string(),
            self.id().to_string(),
            "legacy-port".to_string(),
        ]
    }
}

impl ScharrFilterTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_clip(args: &ToolArgs, key: &str, max_pct: f64) -> f64 {
        args.get(key)
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, max_pct)
    }

    fn parse_line_variant(args: &ToolArgs) -> String {
        let raw = args
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("v")
            .to_lowercase();
        if raw.contains("135") {
            "135".to_string()
        } else if raw.contains("45") {
            "45".to_string()
        } else if raw.contains('h') {
            "h".to_string()
        } else {
            "v".to_string()
        }
    }

    fn parse_emboss_direction(args: &ToolArgs) -> String {
        let d = args
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("n")
            .to_lowercase();
        match d.as_str() {
            "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw" => d,
            _ => "n".to_string(),
        }
    }

    fn parse_abs_values(args: &ToolArgs) -> bool {
        args.get("abs_values")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn parse_weights(args: &ToolArgs) -> Result<Vec<Vec<f64>>, ToolError> {
        let w = args.get("weights").ok_or_else(|| {
            ToolError::Validation("missing required parameter 'weights'".to_string())
        })?;
        let rows = w.as_array().ok_or_else(|| {
            ToolError::Validation("parameter 'weights' must be a 2D array".to_string())
        })?;
        if rows.is_empty() {
            return Err(ToolError::Validation("parameter 'weights' cannot be empty".to_string()));
        }
        let mut out = Vec::<Vec<f64>>::with_capacity(rows.len());
        let mut width = None::<usize>;
        for row in rows {
            let arr = row.as_array().ok_or_else(|| {
                ToolError::Validation("parameter 'weights' must be a 2D array".to_string())
            })?;
            if arr.is_empty() {
                return Err(ToolError::Validation("parameter 'weights' rows cannot be empty".to_string()));
            }
            if let Some(wd) = width {
                if arr.len() != wd {
                    return Err(ToolError::Validation(
                        "parameter 'weights' rows must have equal length".to_string(),
                    ));
                }
            } else {
                width = Some(arr.len());
            }
            let mut vals = Vec::<f64>::with_capacity(arr.len());
            for v in arr {
                vals.push(v.as_f64().ok_or_else(|| {
                    ToolError::Validation("parameter 'weights' entries must be numbers".to_string())
                })?);
            }
            out.push(vals);
        }
        Ok(out)
    }

    fn parse_kernel_center(args: &ToolArgs) -> String {
        args.get("kernel_center")
            .and_then(|v| v.as_str())
            .unwrap_or("center")
            .to_lowercase()
    }

    fn parse_normalize_weights(args: &ToolArgs) -> bool {
        args.get("normalize_weights")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn load_raster(path: &str) -> Result<Arc<Raster>, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_arc_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map(Arc::new)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
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

    fn clip_tails_in_place(output: &mut Raster, percent: f64) {
        if percent <= 0.0 {
            return;
        }
        let mut vals = Vec::<f64>::new();
        for b in 0..output.bands as isize {
            for r in 0..output.rows as isize {
                for c in 0..output.cols as isize {
                    let z = output.get(b, r, c);
                    if !output.is_nodata(z) {
                        vals.push(z);
                    }
                }
            }
        }
        if vals.is_empty() {
            return;
        }
        vals.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let n = vals.len();
        let k = ((percent / 100.0) * n as f64).floor() as usize;
        if k >= n {
            return;
        }
        let low = vals[k];
        let high = vals[n - 1 - k];
        for b in 0..output.bands as isize {
            for r in 0..output.rows as isize {
                for c in 0..output.cols as isize {
                    let z = output.get(b, r, c);
                    if !output.is_nodata(z) {
                        output.set(b, r, c, z.clamp(low, high)).ok();
                    }
                }
            }
        }
    }

    fn run_scalar_kernel(
        input: &Raster,
        output: &mut Raster,
        dx: &[isize],
        dy: &[isize],
        w: &[f64],
        abs_values: bool,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let rgb_mode = color_support::detect_rgb_mode(input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out_row = vec![nodata; cols];
                    for c in 0..cols {
                        let z0 = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z0) {
                            continue;
                        }
                        let center = if packed_rgb { value2i(z0) } else { z0 };

                        let mut sum = 0.0;
                        for i in 0..dx.len() {
                            let zn = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                            let zz = if input.is_nodata(zn) {
                                center
                            } else if packed_rgb {
                                value2i(zn)
                            } else {
                                zn
                            };
                            sum += zz * w[i];
                        }

                        if packed_rgb {
                            let out_v = if abs_values { sum.abs() } else { sum };
                            let (h, s, _) = value2hsi(z0);
                            out_row[c] = hsi2value(h, s, out_v);
                        } else {
                            out_row[c] = if abs_values { sum.abs() } else { sum };
                        }
                    }
                    out_row
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
        }

        Ok(())
    }

    fn run_gradient_kernel(
        input: &Raster,
        output: &mut Raster,
        dx: &[isize],
        dy: &[isize],
        wx: &[f64],
        wy: &[f64],
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let rgb_mode = color_support::detect_rgb_mode(input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out_row = vec![nodata; cols];
                    for c in 0..cols {
                        let z0 = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z0) {
                            continue;
                        }
                        let center = if packed_rgb { value2i(z0) } else { z0 };
                        let mut sx = 0.0;
                        let mut sy = 0.0;
                        for i in 0..dx.len() {
                            let zn = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                            let zz = if input.is_nodata(zn) {
                                center
                            } else if packed_rgb {
                                value2i(zn)
                            } else {
                                zn
                            };
                            sx += zz * wx[i];
                            sy += zz * wy[i];
                        }
                        let v = (sx * sx + sy * sy).sqrt();
                        if packed_rgb {
                            let (h, s, _) = value2hsi(z0);
                            out_row[c] = hsi2value(h, s, v);
                        } else {
                            out_row[c] = v;
                        }
                    }
                    out_row
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output
                    .set_row_slice(band, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
        }
        Ok(())
    }

    fn run_with_op(op: ExtraOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        ctx.progress.info(&format!("running {}", op.id()));
        let input = Self::load_raster(&input_path)?;
        let mut output = input.as_ref().clone();

        match op {
            ExtraOp::Scharr => {
                let dx = vec![1, 1, 1, 0, -1, -1, -1, 0];
                let dy = vec![-1, 0, 1, 1, 1, 0, -1, -1];
                let wx = vec![-3.0, -10.0, -3.0, 0.0, 3.0, 10.0, 3.0, 0.0];
                let wy = vec![3.0, 0.0, -3.0, -10.0, -3.0, 0.0, 3.0, 10.0];
                Self::run_gradient_kernel(&input, &mut output, &dx, &dy, &wx, &wy)?;
                Self::clip_tails_in_place(&mut output, Self::parse_clip(args, "clip_tails", 40.0));
            }
            ExtraOp::Roberts => {
                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let rgb_mode = color_support::detect_rgb_mode(&input, false, true);
                let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;
                for band_idx in 0..bands {
                    let band = band_idx as isize;
                    let row_data: Vec<Vec<f64>> = (0..rows)
                        .into_par_iter()
                        .map(|r| {
                            let mut out_row = vec![nodata; cols];
                            for c in 0..cols {
                                let z1_raw = input.get(band, r as isize, c as isize);
                                if input.is_nodata(z1_raw) {
                                    continue;
                                }
                                let z1 = if packed_rgb { value2i(z1_raw) } else { z1_raw };
                                let z2_raw = input.get(band, r as isize, c as isize + 1);
                                let z3_raw = input.get(band, r as isize + 1, c as isize);
                                let z4_raw = input.get(band, r as isize + 1, c as isize + 1);
                                let z2 = if input.is_nodata(z2_raw) { z1 } else if packed_rgb { value2i(z2_raw) } else { z2_raw };
                                let z3 = if input.is_nodata(z3_raw) { z1 } else if packed_rgb { value2i(z3_raw) } else { z3_raw };
                                let z4 = if input.is_nodata(z4_raw) { z1 } else if packed_rgb { value2i(z4_raw) } else { z4_raw };
                                let v = (z1 - z4).abs() + (z2 - z3).abs();
                                if packed_rgb {
                                    let (h, s, _) = value2hsi(z1_raw);
                                    out_row[c] = hsi2value(h, s, v);
                                } else {
                                    out_row[c] = v;
                                }
                            }
                            out_row
                        })
                        .collect();
                    for (r, row) in row_data.iter().enumerate() {
                        output
                            .set_row_slice(band, r as isize, row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                    }
                }
                Self::clip_tails_in_place(&mut output, Self::parse_clip(args, "clip_amount", 50.0));
            }
            ExtraOp::Line => {
                let variant = Self::parse_line_variant(args);
                let weights = if variant == "h" {
                    vec![-1.0, -1.0, -1.0, 2.0, 2.0, 2.0, -1.0, -1.0, -1.0]
                } else if variant == "135" {
                    vec![2.0, -1.0, -1.0, -1.0, 2.0, -1.0, -1.0, -1.0, 2.0]
                } else if variant == "45" {
                    vec![-1.0, -1.0, 2.0, -1.0, 2.0, -1.0, 2.0, -1.0, -1.0]
                } else {
                    vec![-1.0, 2.0, -1.0, -1.0, 2.0, -1.0, -1.0, 2.0, -1.0]
                };
                let dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                let dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                Self::run_scalar_kernel(&input, &mut output, &dx, &dy, &weights, Self::parse_abs_values(args))?;
                Self::clip_tails_in_place(&mut output, Self::parse_clip(args, "clip_tails", 40.0));
            }
            ExtraOp::Emboss => {
                let direction = Self::parse_emboss_direction(args);
                let weights = match direction.as_str() {
                    "n" => vec![0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                    "s" => vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
                    "e" => vec![0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0],
                    "w" => vec![0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                    "ne" => vec![0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                    "nw" => vec![-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
                    "se" => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
                    _ => vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0],
                };
                let dx = vec![-1, 0, 1, -1, 0, 1, -1, 0, 1];
                let dy = vec![-1, -1, -1, 0, 0, 0, 1, 1, 1];
                Self::run_scalar_kernel(&input, &mut output, &dx, &dy, &weights, false)?;
                Self::clip_tails_in_place(&mut output, Self::parse_clip(args, "clip_amount", 40.0));
            }
            ExtraOp::UserDefined => {
                let weights2d = Self::parse_weights(args)?;
                let kernel_rows = weights2d.len();
                let kernel_cols = weights2d[0].len();
                let kernel_center = Self::parse_kernel_center(args);
                let normalize = Self::parse_normalize_weights(args);

                let (cx, cy) = match kernel_center.as_str() {
                    "upper-left" => (0isize, 0isize),
                    "upper-right" => ((kernel_cols - 1) as isize, 0isize),
                    "lower-left" => (0isize, (kernel_rows - 1) as isize),
                    "lower-right" => ((kernel_cols - 1) as isize, (kernel_rows - 1) as isize),
                    _ => {
                        if kernel_rows % 2 == 0 || kernel_cols % 2 == 0 {
                            return Err(ToolError::Validation(
                                "center kernel_center requires odd kernel dimensions".to_string(),
                            ));
                        }
                        ((kernel_cols / 2) as isize, (kernel_rows / 2) as isize)
                    }
                };

                let mut dx = Vec::<isize>::with_capacity(kernel_rows * kernel_cols);
                let mut dy = Vec::<isize>::with_capacity(kernel_rows * kernel_cols);
                let mut w = Vec::<f64>::with_capacity(kernel_rows * kernel_cols);
                for (r, row) in weights2d.iter().enumerate() {
                    for (c, ww) in row.iter().enumerate() {
                        dx.push(c as isize - cx);
                        dy.push(r as isize - cy);
                        w.push(*ww);
                    }
                }

                let rows = input.rows;
                let cols = input.cols;
                let bands = input.bands;
                let nodata = input.nodata;
                let rgb_mode = color_support::detect_rgb_mode(&input, false, true);
                let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;

                for band_idx in 0..bands {
                    let band = band_idx as isize;
                    let row_data: Vec<Vec<f64>> = (0..rows)
                        .into_par_iter()
                        .map(|r| {
                            let mut out_row = vec![nodata; cols];
                            for c in 0..cols {
                                let z0_raw = input.get(band, r as isize, c as isize);
                                if input.is_nodata(z0_raw) {
                                    continue;
                                }
                                let z0 = if packed_rgb { value2i(z0_raw) } else { z0_raw };
                                let mut sw = 0.0;
                                let mut sum = 0.0;
                                for i in 0..dx.len() {
                                    let zn_raw = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                                    if input.is_nodata(zn_raw) {
                                        continue;
                                    }
                                    let zn = if packed_rgb { value2i(zn_raw) } else { zn_raw };
                                    sw += w[i];
                                    sum += w[i] * zn;
                                }
                                let v = if normalize {
                                    if sw.abs() > f64::EPSILON {
                                        sum / sw
                                    } else {
                                        z0
                                    }
                                } else {
                                    sum
                                };

                                if packed_rgb {
                                    let (h, s, _) = value2hsi(z0_raw);
                                    out_row[c] = hsi2value(h, s, v);
                                } else {
                                    out_row[c] = v;
                                }
                            }
                            out_row
                        })
                        .collect();

                    for (r, row) in row_data.iter().enumerate() {
                        output
                            .set_row_slice(band, r as isize, row)
                            .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
                    }
                }
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }

    fn metadata_for(op: ExtraOp) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            ExtraOp::Scharr => params.push(ToolParamSpec {
                name: "clip_tails",
                description: "Optional symmetric tail clipping percent (0-40).",
                required: false,
            }),
            ExtraOp::Roberts => params.push(ToolParamSpec {
                name: "clip_amount",
                description: "Optional symmetric tail clipping percent (0-50).",
                required: false,
            }),
            ExtraOp::Line => {
                params.push(ToolParamSpec {
                    name: "variant",
                    description: "Line direction variant: v, h, 45, or 135.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "abs_values",
                    description: "If true, output absolute response values.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "clip_tails",
                    description: "Optional symmetric tail clipping percent (0-40).",
                    required: false,
                });
            }
            ExtraOp::Emboss => {
                params.push(ToolParamSpec {
                    name: "direction",
                    description: "Emboss direction: n,s,e,w,ne,nw,se,sw.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "clip_amount",
                    description: "Optional symmetric tail clipping percent (0-40).",
                    required: false,
                });
            }
            ExtraOp::UserDefined => {
                params.push(ToolParamSpec {
                    name: "weights",
                    description: "2D kernel weights as nested arrays.",
                    required: true,
                });
                params.push(ToolParamSpec {
                    name: "kernel_center",
                    description: "Kernel center mode: center, upper-left, upper-right, lower-left, lower-right.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "normalize_weights",
                    description: "If true, normalize by local sum of active weights.",
                    required: false,
                });
            }
        }

        params.push(ToolParamSpec {
            name: "output",
            description: "Optional output path. If omitted, output remains in memory.",
            required: false,
        });

        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
        }
    }

    fn manifest_for(op: ExtraOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        let mut ex = ToolArgs::new();
        ex.insert("input".to_string(), json!("image.tif"));
        ex.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        match op {
            ExtraOp::Scharr => {
                defaults.insert("clip_tails".to_string(), json!(0.0));
            }
            ExtraOp::Roberts => {
                defaults.insert("clip_amount".to_string(), json!(0.0));
            }
            ExtraOp::Line => {
                defaults.insert("variant".to_string(), json!("v"));
                defaults.insert("abs_values".to_string(), json!(false));
                defaults.insert("clip_tails".to_string(), json!(0.0));
                ex.insert("variant".to_string(), json!("v"));
            }
            ExtraOp::Emboss => {
                defaults.insert("direction".to_string(), json!("n"));
                defaults.insert("clip_amount".to_string(), json!(0.0));
                ex.insert("direction".to_string(), json!("n"));
            }
            ExtraOp::UserDefined => {
                defaults.insert(
                    "weights".to_string(),
                    json!([[0.0, -1.0, 0.0], [-1.0, 5.0, -1.0], [0.0, -1.0, 0.0]]),
                );
                defaults.insert("kernel_center".to_string(), json!("center"));
                defaults.insert("normalize_weights".to_string(), json!(false));
                ex.insert(
                    "weights".to_string(),
                    json!([[0.0, -1.0, 0.0], [-1.0, 5.0, -1.0], [0.0, -1.0, 0.0]]),
                );
            }
        }

        let params = Self::metadata_for(op)
            .params
            .into_iter()
            .map(|p| ToolParamDescriptor {
                name: p.name.to_string(),
                description: p.description.to_string(),
                required: p.required,
            })
            .collect();

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params,
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Applies {} to an input raster.", op.id()),
                args: ex,
            }],
            tags: op.tags(),
            stability: ToolStability::Experimental,
        }
    }
}

macro_rules! define_extra_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                ScharrFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                ScharrFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = ScharrFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                ScharrFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_extra_tool!(ScharrFilterTool, ExtraOp::Scharr);
define_extra_tool!(RobertsCrossFilterTool, ExtraOp::Roberts);
define_extra_tool!(LineDetectionFilterTool, ExtraOp::Line);
define_extra_tool!(EmbossFilterTool, ExtraOp::Emboss);
define_extra_tool!(UserDefinedWeightsFilterTool, ExtraOp::UserDefined);

#[cfg(test)]
mod tests {
    use super::*;
    use wbcore::{AllowAllCapabilities, ProgressSink, ToolContext};
    use wbraster::RasterConfig;

    struct NoopProgress;
    impl ProgressSink for NoopProgress {}

    fn make_ctx() -> ToolContext<'static> {
        static PROGRESS: NoopProgress = NoopProgress;
        static CAPS: AllowAllCapabilities = AllowAllCapabilities;
        ToolContext {
            progress: &PROGRESS,
            capabilities: &CAPS,
        }
    }

    fn make_constant_raster(rows: usize, cols: usize, value: f64) -> Raster {
        let cfg = RasterConfig {
            rows,
            cols,
            bands: 1,
            nodata: -9999.0,
            ..Default::default()
        };
        let mut r = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                r.set(0, row, col, value).unwrap();
            }
        }
        r
    }

    fn run_with_memory(tool: &dyn Tool, args: &mut ToolArgs, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let input_path = memory_store::make_raster_memory_path(&id);
        args.insert("input".to_string(), json!(input_path));
        let result = tool.run(args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap().to_string();
        let out_id = memory_store::raster_path_to_id(&out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn scharr_and_roberts_constant_raster_are_zero() {
        let scharr = run_with_memory(&ScharrFilterTool, &mut ToolArgs::new(), make_constant_raster(20, 20, 1.0));
        let roberts = run_with_memory(&RobertsCrossFilterTool, &mut ToolArgs::new(), make_constant_raster(20, 20, 1.0));
        assert!(scharr.get(0, 10, 10).abs() < 1e-9);
        assert!(roberts.get(0, 10, 10).abs() < 1e-9);
    }

    #[test]
    fn user_defined_identity_keeps_constant_value() {
        let mut args = ToolArgs::new();
        args.insert(
            "weights".to_string(),
            json!([
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0]
            ]),
        );
        let out = run_with_memory(&UserDefinedWeightsFilterTool, &mut args, make_constant_raster(20, 20, 9.0));
        assert!((out.get(0, 10, 10) - 9.0).abs() < 1e-9);
    }
}
