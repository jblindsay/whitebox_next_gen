use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

pub struct MeanFilterTool;
pub struct TotalFilterTool;
pub struct StandardDeviationFilterTool;
pub struct MinimumFilterTool;
pub struct MaximumFilterTool;
pub struct RangeFilterTool;

#[derive(Clone, Copy)]
enum WindowOp {
    Mean,
    Total,
    StdDev,
    Min,
    Max,
    Range,
}

impl WindowOp {
    fn id(self) -> &'static str {
        match self {
            Self::Mean => "mean_filter",
            Self::Total => "total_filter",
            Self::StdDev => "standard_deviation_filter",
            Self::Min => "minimum_filter",
            Self::Max => "maximum_filter",
            Self::Range => "range_filter",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Mean => "Mean Filter",
            Self::Total => "Total Filter",
            Self::StdDev => "Standard Deviation Filter",
            Self::Min => "Minimum Filter",
            Self::Max => "Maximum Filter",
            Self::Range => "Range Filter",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Mean => "Computes a moving-window mean for each raster cell.",
            Self::Total => "Computes a moving-window total for each raster cell.",
            Self::StdDev => "Computes a moving-window standard deviation for each raster cell.",
            Self::Min => "Computes a moving-window minimum for each raster cell.",
            Self::Max => "Computes a moving-window maximum for each raster cell.",
            Self::Range => "Computes a moving-window range (max-min) for each raster cell.",
        }
    }

    fn processing_message(self) -> &'static str {
        match self {
            Self::Mean => "applying moving-window mean",
            Self::Total => "applying moving-window total",
            Self::StdDev => "applying moving-window standard deviation",
            Self::Min => "applying moving-window minimum",
            Self::Max => "applying moving-window maximum",
            Self::Range => "applying moving-window range",
        }
    }

    fn tags(self) -> Vec<String> {
        vec![
            "remote_sensing".to_string(),
            "raster".to_string(),
            "filter".to_string(),
            "moving_window".to_string(),
            self.id().to_string(),
            "legacy-port".to_string(),
        ]
    }
}

impl MeanFilterTool {
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

    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
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

    fn metadata_for(op: WindowOp) -> ToolMetadata {
        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "filter_size_x",
                    description: "Window width in pixels (odd integer, default 11). Alias: filterx.",
                    required: false,
                },
                ToolParamSpec {
                    name: "filter_size_y",
                    description: "Window height in pixels (odd integer, default = filter_size_x). Alias: filtery.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, output remains in memory.",
                    required: false,
                },
            ],
        }
    }

    fn manifest_for(op: WindowOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("image.tif"));
        example_args.insert("filter_size_x".to_string(), json!(11));
        example_args.insert("filter_size_y".to_string(), json!(11));
        example_args.insert("output".to_string(), json!(format!("{}.tif", op.id())));

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "filter_size_x".to_string(),
                    description: "Window width in pixels (odd integer, default 11). Alias: filterx.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "filter_size_y".to_string(),
                    description: "Window height in pixels (odd integer, default = filter_size_x). Alias: filtery.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.id()),
                description: format!("Applies {} with an 11x11 neighborhood.", op.id()),
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

    fn run_with_integral_op(
        input: &Raster,
        output: &mut Raster,
        filter_x: usize,
        filter_y: usize,
        op: WindowOp,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;

        for band_idx in 0..bands {
            let band = band_idx as isize;

            let stride = cols + 1;
            let mut integral_sum = vec![0.0f64; (rows + 1) * (cols + 1)];
            let mut integral_sum_sq = vec![0.0f64; (rows + 1) * (cols + 1)];
            let mut integral_count = vec![0u32; (rows + 1) * (cols + 1)];

            for r in 0..rows {
                let mut row_sum = 0.0f64;
                let mut row_sum_sq = 0.0f64;
                let mut row_count = 0u32;
                let ir = (r + 1) * stride;
                let ir_prev = r * stride;
                for c in 0..cols {
                    let z = input.get(band, r as isize, c as isize);
                    if !input.is_nodata(z) {
                        row_sum += z;
                        row_sum_sq += z * z;
                        row_count += 1;
                    }
                    let idx = ir + (c + 1);
                    integral_sum[idx] = integral_sum[ir_prev + (c + 1)] + row_sum;
                    integral_sum_sq[idx] = integral_sum_sq[ir_prev + (c + 1)] + row_sum_sq;
                    integral_count[idx] = integral_count[ir_prev + (c + 1)] + row_count;
                }
            }

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z_center = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z_center) {
                            continue;
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
                            row_out[c] = 0.0;
                            continue;
                        }

                        let sum = integral_sum[d] + integral_sum[a] - integral_sum[b] - integral_sum[cidx];

                        row_out[c] = match op {
                            WindowOp::Total => sum,
                            WindowOp::Mean => sum / n,
                            WindowOp::StdDev => {
                                let sum_sq = integral_sum_sq[d] + integral_sum_sq[a] - integral_sum_sq[b] - integral_sum_sq[cidx];
                                let variance = (sum_sq - (sum * sum) / n) / n;
                                if variance > 0.0 { variance.sqrt() } else { 0.0 }
                            }
                            _ => nodata,
                        };
                    }
                    row_out
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

    fn run_with_extrema_op(
        input: &Raster,
        output: &mut Raster,
        filter_x: usize,
        filter_y: usize,
        op: WindowOp,
    ) -> Result<(), ToolError> {
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let half_x = (filter_x / 2) as isize;
        let half_y = (filter_y / 2) as isize;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z_center = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z_center) {
                            continue;
                        }

                        let mut min_val = f64::INFINITY;
                        let mut max_val = f64::NEG_INFINITY;
                        let mut has_data = false;

                        for ny in (r as isize - half_y)..=(r as isize + half_y) {
                            for nx in (c as isize - half_x)..=(c as isize + half_x) {
                                let zn = input.get(band, ny, nx);
                                if input.is_nodata(zn) {
                                    continue;
                                }
                                has_data = true;
                                if zn < min_val {
                                    min_val = zn;
                                }
                                if zn > max_val {
                                    max_val = zn;
                                }
                            }
                        }

                        row_out[c] = if !has_data {
                            0.0
                        } else {
                            match op {
                                WindowOp::Min => min_val,
                                WindowOp::Max => max_val,
                                WindowOp::Range => max_val - min_val,
                                _ => nodata,
                            }
                        };
                    }
                    row_out
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

    fn run_with_op(op: WindowOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_x, filter_y) = Self::parse_window_sizes(args);

        ctx.progress.info(&format!("running {}", op.id()));
        ctx.progress.info("reading input raster");
        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();

        ctx.progress.info(op.processing_message());

        match op {
            WindowOp::Mean | WindowOp::Total | WindowOp::StdDev => {
                Self::run_with_integral_op(&input, &mut output, filter_x, filter_y, op)?;
            }
            WindowOp::Min | WindowOp::Max | WindowOp::Range => {
                Self::run_with_extrema_op(&input, &mut output, filter_x, filter_y, op)?;
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;

        ctx.progress.progress(1.0);
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

macro_rules! define_window_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                MeanFilterTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                MeanFilterTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = MeanFilterTool::parse_input(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                let _ = MeanFilterTool::parse_window_sizes(args);
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                MeanFilterTool::run_with_op($op, args, ctx)
            }
        }
    };
}

define_window_tool!(MeanFilterTool, WindowOp::Mean);
define_window_tool!(TotalFilterTool, WindowOp::Total);
define_window_tool!(StandardDeviationFilterTool, WindowOp::StdDev);
define_window_tool!(MinimumFilterTool, WindowOp::Min);
define_window_tool!(MaximumFilterTool, WindowOp::Max);
define_window_tool!(RangeFilterTool, WindowOp::Range);

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
    fn mean_filter_constant_raster_is_unchanged() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(5));
        args.insert("filter_size_y".to_string(), json!(5));
        let out = run_with_memory(&MeanFilterTool, &mut args, make_constant_raster(25, 25, 10.0));
        for row in 0..25isize {
            for col in 0..25isize {
                assert!((out.get(0, row, col) - 10.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn min_max_and_range_constant_raster_expected_values() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(7));
        args.insert("filter_size_y".to_string(), json!(7));

        let min_out = run_with_memory(&MinimumFilterTool, &mut args.clone(), make_constant_raster(21, 21, 3.0));
        let max_out = run_with_memory(&MaximumFilterTool, &mut args.clone(), make_constant_raster(21, 21, 3.0));
        let rng_out = run_with_memory(&RangeFilterTool, &mut args, make_constant_raster(21, 21, 3.0));

        for row in 0..21isize {
            for col in 0..21isize {
                assert!((min_out.get(0, row, col) - 3.0).abs() < 1e-9);
                assert!((max_out.get(0, row, col) - 3.0).abs() < 1e-9);
                assert!(rng_out.get(0, row, col).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn stddev_filter_constant_raster_is_zero() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(9));
        args.insert("filter_size_y".to_string(), json!(9));
        let out = run_with_memory(&StandardDeviationFilterTool, &mut args, make_constant_raster(30, 30, 42.0));
        for row in 0..30isize {
            for col in 0..30isize {
                assert!(out.get(0, row, col).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn total_filter_matches_neighborhood_sum_interior() {
        let mut args = ToolArgs::new();
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let out = run_with_memory(&TotalFilterTool, &mut args, make_constant_raster(10, 10, 2.0));
        assert!((out.get(0, 5, 5) - 18.0).abs() < 1e-9);
        assert!((out.get(0, 0, 0) - 8.0).abs() < 1e-9);
    }
}
