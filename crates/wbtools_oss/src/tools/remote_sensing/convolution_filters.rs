use std::cmp::Ordering;
use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::color_math::value2i;
use wbraster::{Raster, RasterFormat};

use super::color_support;
use crate::memory_store;

pub struct HighPassFilterTool;
pub struct LaplacianFilterTool;
pub struct SobelFilterTool;
pub struct PrewittFilterTool;

#[derive(Clone, Copy)]
enum ConvOp {
    HighPass,
    Laplacian,
    Sobel,
    Prewitt,
}

impl ConvOp {
    fn id(self) -> &'static str {
        match self {
            Self::HighPass => "high_pass_filter",
            Self::Laplacian => "laplacian_filter",
            Self::Sobel => "sobel_filter",
            Self::Prewitt => "prewitt_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::HighPass => "High-Pass Filter",
            Self::Laplacian => "Laplacian Filter",
            Self::Sobel => "Sobel Filter",
            Self::Prewitt => "Prewitt Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::HighPass => "Performs high-pass filtering using neighborhood mean subtraction.",
            Self::Laplacian => "Performs Laplacian edge/sharpen filtering.",
            Self::Sobel => "Performs Sobel edge detection.",
            Self::Prewitt => "Performs Prewitt edge detection.",
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

impl HighPassFilterTool {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_window_sizes(args: &ToolArgs) -> (usize, usize) {
        let mut filter_x = args
            .get("filter_size_x")
            .and_then(|v| v.as_u64())
            .or_else(|| args.get("filterx").and_then(|v| v.as_u64()))
            .unwrap_or(11) as usize;
        let mut filter_y = args
            .get("filter_size_y")
            .and_then(|v| v.as_u64())
            .or_else(|| args.get("filtery").and_then(|v| v.as_u64()))
            .unwrap_or(filter_x as u64) as usize;

        if filter_x < 3 {
            filter_x = 3;
        }
        if filter_y < 3 {
            filter_y = 3;
        }
        if filter_x % 2 == 0 {
            filter_x += 1;
        }
        if filter_y % 2 == 0 {
            filter_y += 1;
        }
        (filter_x, filter_y)
    }

    fn parse_clip_percent(args: &ToolArgs, key: &str) -> f64 {
        args.get(key)
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            .clamp(0.0, 40.0)
    }

    fn parse_laplacian_variant(args: &ToolArgs) -> String {
        args.get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("3x3(1)")
            .to_string()
    }

    fn parse_sobel_variant(args: &ToolArgs) -> String {
        let raw = args
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("3x3")
            .to_lowercase();
        if raw.contains('5') {
            "5x5".to_string()
        } else {
            "3x3".to_string()
        }
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}': store entry is missing",
                    id
                ))
            });
        }

        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
    }

    fn metadata_for(op: ConvOp) -> ToolMetadata {
        let mut params = vec![ToolParamSpec {
            name: "input",
            description: "Input raster path or typed raster object.",
            required: true,
        }];

        match op {
            ConvOp::HighPass => {
                params.push(ToolParamSpec {
                    name: "filter_size_x",
                    description: "Window width in pixels (odd integer, default 11). Alias: filterx.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "filter_size_y",
                    description: "Window height in pixels (odd integer, default = filter_size_x). Alias: filtery.",
                    required: false,
                });
            }
            ConvOp::Laplacian => {
                params.push(ToolParamSpec {
                    name: "variant",
                    description: "Kernel variant: 3x3(1), 3x3(2), 3x3(3), 3x3(4), 5x5(1), 5x5(2).",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "clip_amount",
                    description: "Optional symmetric tail clipping percent (0-40).",
                    required: false,
                });
            }
            ConvOp::Sobel => {
                params.push(ToolParamSpec {
                    name: "variant",
                    description: "Kernel size variant: 3x3 (default) or 5x5.",
                    required: false,
                });
                params.push(ToolParamSpec {
                    name: "clip_tails",
                    description: "Optional symmetric tail clipping percent (0-40).",
                    required: false,
                });
            }
            ConvOp::Prewitt => {
                params.push(ToolParamSpec {
                    name: "clip_tails",
                    description: "Optional symmetric tail clipping percent (0-40).",
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

    fn manifest_for(op: ConvOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        match op {
            ConvOp::HighPass => {
                defaults.insert("filter_size_x".to_string(), json!(11));
                defaults.insert("filter_size_y".to_string(), json!(11));
            }
            ConvOp::Laplacian => {
                defaults.insert("variant".to_string(), json!("3x3(1)"));
                defaults.insert("clip_amount".to_string(), json!(0.0));
            }
            ConvOp::Sobel => {
                defaults.insert("variant".to_string(), json!("3x3"));
                defaults.insert("clip_tails".to_string(), json!(0.0));
            }
            ConvOp::Prewitt => {
                defaults.insert("clip_tails".to_string(), json!(0.0));
            }
        }

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));
        if matches!(op, ConvOp::HighPass) {
            example_args.insert("filter_size_x".to_string(), json!(11));
            example_args.insert("filter_size_y".to_string(), json!(11));
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
                args: example_args,
            }],
            tags: op.tags(),
            stability: ToolStability::Experimental,
        }
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
                    if output.is_nodata(z) {
                        continue;
                    }
                    output.set(b, r, c, z.clamp(low, high)).ok();
                }
            }
        }
    }

    fn high_pass(input: &Raster, output: &mut Raster, filter_x: usize, filter_y: usize) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;
        let rgb_mode = color_support::detect_rgb_mode(input, false, true);
        let packed_rgb = matches!(rgb_mode, color_support::RgbMode::Packed) && bands == 1;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let stride = cols + 1;
            let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
            let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

            for r in 0..rows {
                let mut row_sum = 0.0f64;
                let mut row_count = 0u32;
                let ir = (r + 1) * stride;
                let ir_prev = r * stride;
                for c in 0..cols {
                    let mut z = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z) {
                        if packed_rgb {
                            z = value2i(z);
                        }
                        row_sum += z;
                        row_count += 1;
                    }
                    let idx = ir + (c + 1);
                    integral_sum[idx] = integral_sum[ir_prev + (c + 1)] + row_sum;
                    integral_count[idx] = integral_count[ir_prev + (c + 1)] + row_count;
                }
            }

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out_row = vec![nodata; cols];
                    for c in 0..cols {
                        let mut z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        if packed_rgb {
                            z = value2i(z);
                        }

                        let y1 = (r as isize - half_y).max(0) as usize;
                        let y2 = (r as isize + half_y).min((rows - 1) as isize) as usize;
                        let x1 = (c as isize - half_x).max(0) as usize;
                        let x2 = (c as isize + half_x).min((cols - 1) as isize) as usize;

                        let a = y1 * stride + x1;
                        let b = y1 * stride + (x2 + 1);
                        let cidx = (y2 + 1) * stride + x1;
                        let d = (y2 + 1) * stride + (x2 + 1);

                        let n = (integral_count[d] + integral_count[a] - integral_count[b] - integral_count[cidx]) as f64;
                        if n <= 0.0 {
                            out_row[c] = 0.0;
                            continue;
                        }
                        let sum = integral_sum[d] + integral_sum[a] - integral_sum[b] - integral_sum[cidx];
                        out_row[c] = z - (sum / n);
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

    fn laplacian_kernel(variant: &str) -> (Vec<isize>, Vec<isize>, Vec<f64>) {
        if variant.contains("3x3(1)") {
            (
                vec![-1, 0, 1, -1, 0, 1, -1, 0, 1],
                vec![-1, -1, -1, 0, 0, 0, 1, 1, 1],
                vec![0.0, -1.0, 0.0, -1.0, 4.0, -1.0, 0.0, -1.0, 0.0],
            )
        } else if variant.contains("3x3(2)") {
            (
                vec![-1, 0, 1, -1, 0, 1, -1, 0, 1],
                vec![-1, -1, -1, 0, 0, 0, 1, 1, 1],
                vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
            )
        } else if variant.contains("3x3(3)") {
            (
                vec![-1, 0, 1, -1, 0, 1, -1, 0, 1],
                vec![-1, -1, -1, 0, 0, 0, 1, 1, 1],
                vec![-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0],
            )
        } else if variant.contains("3x3(4)") {
            (
                vec![-1, 0, 1, -1, 0, 1, -1, 0, 1],
                vec![-1, -1, -1, 0, 0, 0, 1, 1, 1],
                vec![1.0, -2.0, 1.0, -2.0, 4.0, -2.0, 1.0, -2.0, 1.0],
            )
        } else if variant.contains("5x5(1)") {
            (
                vec![
                    -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1,
                    0, 1, 2,
                ],
                vec![
                    -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2,
                    2, 2, 2,
                ],
                vec![
                    0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 17.0,
                    -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0,
                ],
            )
        } else {
            (
                vec![
                    -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1,
                    0, 1, 2,
                ],
                vec![
                    -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2,
                    2, 2, 2,
                ],
                vec![
                    0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, -2.0, -1.0, 0.0, -1.0, -2.0, 16.0,
                    -2.0, -1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0,
                ],
            )
        }
    }

    fn apply_single_kernel(input: &Raster, output: &mut Raster, dx: &[isize], dy: &[isize], w: &[f64]) -> Result<(), ToolError> {
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
                            let z = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                            let zz = if input.is_nodata(z) {
                                center
                            } else if packed_rgb {
                                value2i(z)
                            } else {
                                z
                            };
                            sum += zz * w[i];
                        }
                        out_row[c] = sum;
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

    fn apply_dual_gradient_kernel(
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
                            let z = input.get(band, r as isize + dy[i], c as isize + dx[i]);
                            let zz = if input.is_nodata(z) {
                                center
                            } else if packed_rgb {
                                value2i(z)
                            } else {
                                z
                            };
                            sx += zz * wx[i];
                            sy += zz * wy[i];
                        }
                        out_row[c] = (sx * sx + sy * sy).sqrt();
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

    fn run_with_op(op: ConvOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(&format!("running {}", op.id()));
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();

        match op {
            ConvOp::HighPass => {
                let (fx, fy) = Self::parse_window_sizes(args);
                Self::high_pass(&input, &mut output, fx, fy)?;
            }
            ConvOp::Laplacian => {
                let variant = Self::parse_laplacian_variant(args);
                let (dx, dy, w) = Self::laplacian_kernel(&variant);
                Self::apply_single_kernel(&input, &mut output, &dx, &dy, &w)?;
                let clip = Self::parse_clip_percent(args, "clip_amount");
                Self::clip_tails_in_place(&mut output, clip);
            }
            ConvOp::Sobel => {
                let variant = Self::parse_sobel_variant(args);
                if variant == "5x5" {
                    let dx = vec![
                        -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2, -1, 0, 1, 2, -2,
                        -1, 0, 1, 2,
                    ];
                    let dy = vec![
                        -2, -2, -2, -2, -2, -1, -1, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2,
                        2, 2, 2, 2,
                    ];
                    let wx = vec![
                        2.0, 1.0, 0.0, -1.0, -2.0, 3.0, 2.0, 0.0, -2.0, -3.0, 4.0, 3.0, 0.0,
                        -3.0, -4.0, 3.0, 2.0, 0.0, -2.0, -3.0, 2.0, 1.0, 0.0, -1.0, -2.0,
                    ];
                    let wy = vec![
                        2.0, 3.0, 4.0, 3.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0,
                        0.0, -1.0, -2.0, -3.0, -2.0, -1.0, -2.0, -3.0, -4.0, -3.0, -2.0,
                    ];
                    Self::apply_dual_gradient_kernel(&input, &mut output, &dx, &dy, &wx, &wy)?;
                } else {
                    let dx = vec![1, 1, 1, 0, -1, -1, -1, 0];
                    let dy = vec![-1, 0, 1, 1, 1, 0, -1, -1];
                    let wx = vec![1.0, 2.0, 1.0, 0.0, -1.0, -2.0, -1.0, 0.0];
                    let wy = vec![1.0, 0.0, -1.0, -2.0, -1.0, 0.0, 1.0, 2.0];
                    Self::apply_dual_gradient_kernel(&input, &mut output, &dx, &dy, &wx, &wy)?;
                }
                let clip = Self::parse_clip_percent(args, "clip_tails");
                Self::clip_tails_in_place(&mut output, clip);
            }
            ConvOp::Prewitt => {
                let dx = vec![1, 1, 1, 0, -1, -1, -1, 0];
                let dy = vec![-1, 0, 1, 1, 1, 0, -1, -1];
                let wx = vec![1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0, 0.0];
                let wy = vec![1.0, 0.0, -1.0, -1.0, -1.0, 0.0, 1.0, 1.0];
                Self::apply_dual_gradient_kernel(&input, &mut output, &dx, &dy, &wx, &wy)?;
                let clip = Self::parse_clip_percent(args, "clip_tails");
                Self::clip_tails_in_place(&mut output, clip);
            }
        }

        ctx.progress.progress(1.0);
        let output_locator = Self::write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

macro_rules! define_conv_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                HighPassFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                HighPassFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = HighPassFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                HighPassFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_conv_tool!(HighPassFilterTool, ConvOp::HighPass);
define_conv_tool!(LaplacianFilterTool, ConvOp::Laplacian);
define_conv_tool!(SobelFilterTool, ConvOp::Sobel);
define_conv_tool!(PrewittFilterTool, ConvOp::Prewitt);

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
    fn highpass_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(7));
        args.insert("filter_size_y".to_string(), json!(7));
        let out = run_with_memory(&HighPassFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn laplacian_3x3_1_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("variant".to_string(), json!("3x3(1)"));
        let out = run_with_memory(&LaplacianFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        assert!(out.get(0, 12, 12).abs() < 1e-9);
    }

    #[test]
    fn sobel_and_prewitt_constant_raster_are_zero() {
        let mut sobel_args = ToolArgs::new();
        sobel_args.insert("variant".to_string(), json!("3x3"));
        let sobel_out = run_with_memory(&SobelFilterTool, &mut sobel_args, make_constant_raster(25, 25, 10.0));
        assert!(sobel_out.get(0, 12, 12).abs() < 1e-9);

        let mut prewitt_args = ToolArgs::new();
        let prewitt_out = run_with_memory(&PrewittFilterTool, &mut prewitt_args, make_constant_raster(25, 25, 10.0));
        assert!(prewitt_out.get(0, 12, 12).abs() < 1e-9);
    }
}
