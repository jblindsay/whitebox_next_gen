use std::collections::BTreeMap;
use std::path::Path;

use rayon::prelude::*;
use serde_json::{json, Value};
use wbcore::{
    LicenseTier, PercentCoalescer, Tool, ToolArgs, ToolCategory, ToolContext, ToolError,
    ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult,
    ToolStability,
};
use wbraster::{memory_store, Raster, RasterFormat};

struct UnaryRasterMathSpec {
    id: &'static str,
    display_name: &'static str,
    summary: &'static str,
    op: fn(f64) -> f64,
}

const UNARY_MATH_PAR_CHUNK: usize = 16_384;
const UNARY_MATH_PROGRESS_BATCH_CHUNKS: usize = 64;

fn parse_input(args: &ToolArgs) -> Result<&str, ToolError> {
    let input = args
        .get("input")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Validation("missing required string parameter 'input'".to_string()))?;
    Ok(input)
}

fn parse_optional_output(args: &ToolArgs) -> Result<Option<&str>, ToolError> {
    match args.get("output") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(ToolError::Validation(
            "parameter 'output' must be a string when provided".to_string(),
        )),
    }
}

fn load_input_raster(path: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Validation("malformed in-memory raster path".to_string()))?;
        return memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Validation(format!("unknown in-memory raster id '{id}'")));
    }

    Raster::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))
}

fn write_or_store_output(output: Raster, output_path: Option<&str>) -> Result<String, ToolError> {
    if let Some(output_path) = output_path {
        if let Some(parent) = Path::new(output_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
            }
        }

        let output_format = RasterFormat::for_output_path(output_path)
            .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;

        output
            .write(output_path, output_format)
            .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;
        Ok(output_path.to_string())
    } else {
        let id = memory_store::put_raster(output);
        Ok(memory_store::make_raster_memory_path(&id))
    }
}

fn metadata_for(spec: &UnaryRasterMathSpec) -> ToolMetadata {
    ToolMetadata {
        id: spec.id,
        display_name: spec.display_name,
        summary: spec.summary,
        category: ToolCategory::Raster,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamSpec {
                name: "input",
                description: "Input raster file path.",
                required: true,
            },
            ToolParamSpec {
                name: "output",
                description: "Optional output raster file path. If omitted, the result is stored in memory.",
                required: false,
            },
        ],
    }
}

fn manifest_for(spec: &UnaryRasterMathSpec) -> ToolManifest {
    let mut defaults = ToolArgs::new();
    defaults.insert("input".to_string(), json!("input.tif"));
    defaults.insert("output".to_string(), json!("output.tif"));

    let mut example_args = ToolArgs::new();
    example_args.insert("input".to_string(), json!("dem.tif"));
    example_args.insert("output".to_string(), json!(format!("{}_dem.tif", spec.id)));

    ToolManifest {
        id: spec.id.to_string(),
        display_name: spec.display_name.to_string(),
        summary: spec.summary.to_string(),
            category: ToolCategory::Raster,
        license_tier: LicenseTier::Open,
        params: vec![
            ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input raster file path.".to_string(),
                required: true,
            },
            ToolParamDescriptor {
                name: "output".to_string(),
                description: "Optional output raster file path. If omitted, the result is stored in memory.".to_string(),
                required: false,
            },
        ],
        defaults,
        examples: vec![ToolExample {
            name: "basic_run".to_string(),
            description: format!("Apply {} transform to each non-nodata cell.", spec.id),
            args: example_args,
        }],
        tags: vec!["raster".to_string(), "math".to_string(), spec.id.to_string()],
        stability: ToolStability::Experimental,
    }
}

fn run_unary_math(spec: &UnaryRasterMathSpec, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
    let input_path = parse_input(args)?;
    let output_path = parse_optional_output(args)?;

    ctx.progress
        .info(&format!("running {}", spec.id));

    let input = load_input_raster(input_path)?;
    let mut output = input.clone();

    let len = output.data.len();
    let in_values: Vec<f64> = (0..len).into_par_iter().map(|i| input.data.get_f64(i)).collect();
    let mut out_values = vec![input.nodata; len];
    let coalescer = PercentCoalescer::new(1, 99);
    let total_chunks = len.div_ceil(UNARY_MATH_PAR_CHUNK).max(1);
    let compute_progress = PercentCoalescer::new(1, 75);
    let mut completed_chunks = 0usize;

    while completed_chunks < total_chunks {
        let batch_chunks = (total_chunks - completed_chunks).min(UNARY_MATH_PROGRESS_BATCH_CHUNKS);
        let batch_start = completed_chunks * UNARY_MATH_PAR_CHUNK;
        let batch_end = (batch_start + batch_chunks * UNARY_MATH_PAR_CHUNK).min(len);

        out_values[batch_start..batch_end]
            .par_chunks_mut(UNARY_MATH_PAR_CHUNK)
            .zip(in_values[batch_start..batch_end].par_chunks(UNARY_MATH_PAR_CHUNK))
            .for_each(|(out_chunk, in_chunk)| {
                for i in 0..out_chunk.len() {
                    let z = in_chunk[i];
                    out_chunk[i] = if input.is_nodata(z) { input.nodata } else { (spec.op)(z) };
                }
            });

        completed_chunks += batch_chunks;
        compute_progress.emit_unit_fraction(ctx.progress, completed_chunks as f64 / total_chunks as f64);
    }
    compute_progress.finish(ctx.progress);

    for (i, value) in out_values.iter().enumerate() {
        output.data.set_f64(i, *value);
    }
    coalescer.emit_unit_fraction(ctx.progress, 0.9);

    let output_locator = write_or_store_output(output, output_path)?;
    ctx.progress.progress(1.0);

    let mut outputs = BTreeMap::new();
    outputs.insert("path".to_string(), json!(output_locator.clone()));
    outputs.insert(
        "output".to_string(),
        json!({"__wbw_type__": "raster", "path": output_locator, "active_band": 0}),
    );
    outputs.insert("cells_processed".to_string(), json!(len));
    Ok(ToolRunResult { outputs })
}

macro_rules! define_unary_tool {
    ($tool:ident, $id:literal, $display:literal, $summary:literal, $op:expr) => {
        pub struct $tool;

        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                let spec = UnaryRasterMathSpec {
                    id: $id,
                    display_name: $display,
                    summary: $summary,
                    op: $op,
                };
                metadata_for(&spec)
            }

            fn manifest(&self) -> ToolManifest {
                let spec = UnaryRasterMathSpec {
                    id: $id,
                    display_name: $display,
                    summary: $summary,
                    op: $op,
                };
                manifest_for(&spec)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = parse_input(args)?;
                let _ = parse_optional_output(args)?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                let spec = UnaryRasterMathSpec {
                    id: $id,
                    display_name: $display,
                    summary: $summary,
                    op: $op,
                };
                run_unary_math(&spec, args, ctx)
            }
        }
    };
}

define_unary_tool!(RasterAbsTool, "abs", "Abs", "Calculates the absolute value of each raster cell.", |z: f64| z.abs());
define_unary_tool!(RasterCeilTool, "ceil", "Ceil", "Rounds each raster cell upward to the nearest integer.", |z: f64| z.ceil());
define_unary_tool!(RasterFloorTool, "floor", "Floor", "Rounds each raster cell downward to the nearest integer.", |z: f64| z.floor());
define_unary_tool!(RasterRoundTool, "round", "Round", "Rounds each raster cell to the nearest integer.", |z: f64| z.round());
define_unary_tool!(RasterSqrtTool, "sqrt", "Sqrt", "Computes the square-root of each raster cell.", |z: f64| z.sqrt());
define_unary_tool!(RasterSquareTool, "square", "Square", "Squares each raster cell value.", |z: f64| z * z);
define_unary_tool!(RasterLnTool, "ln", "Ln", "Computes the natural logarithm of each raster cell.", |z: f64| z.ln());
define_unary_tool!(RasterLog10Tool, "log10", "Log10", "Computes the base-10 logarithm of each raster cell.", |z: f64| z.log10());
define_unary_tool!(RasterSinTool, "sin", "Sin", "Computes the sine of each raster cell value.", |z: f64| z.sin());
define_unary_tool!(RasterCosTool, "cos", "Cos", "Computes the cosine of each raster cell value.", |z: f64| z.cos());
define_unary_tool!(RasterTanTool, "tan", "Tan", "Computes the tangent of each raster cell value.", |z: f64| z.tan());
define_unary_tool!(RasterArcsinTool, "arcsin", "Arcsin", "Computes the inverse sine (arcsin) of each raster cell.", |z: f64| z.asin());
define_unary_tool!(RasterArccosTool, "arccos", "Arccos", "Computes the inverse cosine (arccos) of each raster cell.", |z: f64| z.acos());
define_unary_tool!(RasterArctanTool, "arctan", "Arctan", "Computes the inverse tangent (arctan) of each raster cell.", |z: f64| z.atan());
define_unary_tool!(RasterSinhTool, "sinh", "Sinh", "Computes the hyperbolic sine of each raster cell.", |z: f64| z.sinh());
define_unary_tool!(RasterCoshTool, "cosh", "Cosh", "Computes the hyperbolic cosine of each raster cell.", |z: f64| z.cosh());
define_unary_tool!(RasterTanhTool, "tanh", "Tanh", "Computes the hyperbolic tangent of each raster cell.", |z: f64| z.tanh());
define_unary_tool!(RasterArsinhTool, "arsinh", "Arsinh", "Computes the inverse hyperbolic sine of each raster cell.", |z: f64| z.asinh());
define_unary_tool!(RasterArcoshTool, "arcosh", "Arcosh", "Computes the inverse hyperbolic cosine of each raster cell.", |z: f64| z.acosh());
define_unary_tool!(RasterArtanhTool, "artanh", "Artanh", "Computes the inverse hyperbolic tangent of each raster cell.", |z: f64| z.atanh());
define_unary_tool!(RasterExpTool, "exp", "Exp", "Computes e raised to the power of each raster cell.", |z: f64| z.exp());
define_unary_tool!(RasterExp2Tool, "exp2", "Exp2", "Computes 2 raised to the power of each raster cell.", |z: f64| z.exp2());
define_unary_tool!(RasterLog2Tool, "log2", "Log2", "Computes the base-2 logarithm of each raster cell.", |z: f64| z.log2());
define_unary_tool!(RasterNegateTool, "negate", "Negate", "Negates each non-nodata raster cell value.", |z: f64| -z);
define_unary_tool!(RasterReciprocalTool, "reciprocal", "Reciprocal", "Computes the reciprocal (1/x) of each raster cell.", |z: f64| 1.0 / z);
define_unary_tool!(RasterTruncateTool, "truncate", "Truncate", "Truncates each raster cell value to its integer part.", |z: f64| z.trunc());
define_unary_tool!(RasterIncrementTool, "increment", "Increment", "Adds 1 to each non-nodata raster cell.", |z: f64| z + 1.0);
define_unary_tool!(RasterDecrementTool, "decrement", "Decrement", "Subtracts 1 from each non-nodata raster cell.", |z: f64| z - 1.0);
define_unary_tool!(RasterToDegTool, "to_degrees", "ToDegrees", "Converts each raster cell from radians to degrees.", |z: f64| z.to_degrees());
define_unary_tool!(RasterToRadTool, "to_radians", "ToRadians", "Converts each raster cell from degrees to radians.", |z: f64| z.to_radians());
define_unary_tool!(RasterBoolNotTool, "bool_not", "BoolNot", "Computes a logical NOT of each raster cell, outputting 1 for zero-valued cells and 0 otherwise.", |z: f64| if z == 0.0 { 1.0 } else { 0.0 });

// is_nodata: special kernel — outputs 1.0 where input is nodata, 0.0 where input is valid.
pub struct RasterIsNodataTool;

impl Tool for RasterIsNodataTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "is_nodata",
            display_name: "IsNodata",
            summary: "Outputs 1 for nodata cells and 0 for all valid cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster file path.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster file path. If omitted, the result is stored in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("output".to_string(), json!("output.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("output".to_string(), json!("dem_is_nodata.tif"));

        ToolManifest {
            id: "is_nodata".to_string(),
            display_name: "IsNodata".to_string(),
            summary: "Outputs 1 for nodata cells and 0 for all valid cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster file path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster file path. If omitted, the result is stored in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_run".to_string(),
                description: "Identify nodata cells.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "is_nodata".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_input(args)?;
        let _ = parse_optional_output(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_input(args)?;
        let output_path = parse_optional_output(args)?;
        ctx.progress.info("running is_nodata");

        let input = load_input_raster(input_path)?;
        let mut output = input.clone();
        let len = output.data.len();

        let in_values: Vec<f64> = (0..len).into_par_iter().map(|i| input.data.get_f64(i)).collect();
        let mut out_values = vec![0.0; len];
        let coalescer = PercentCoalescer::new(1, 99);
        let total_chunks = len.div_ceil(UNARY_MATH_PAR_CHUNK).max(1);
        let compute_progress = PercentCoalescer::new(1, 75);
        let mut completed_chunks = 0usize;

        while completed_chunks < total_chunks {
            let batch_chunks = (total_chunks - completed_chunks).min(UNARY_MATH_PROGRESS_BATCH_CHUNKS);
            let batch_start = completed_chunks * UNARY_MATH_PAR_CHUNK;
            let batch_end = (batch_start + batch_chunks * UNARY_MATH_PAR_CHUNK).min(len);

            out_values[batch_start..batch_end]
                .par_chunks_mut(UNARY_MATH_PAR_CHUNK)
                .zip(in_values[batch_start..batch_end].par_chunks(UNARY_MATH_PAR_CHUNK))
                .for_each(|(out_chunk, in_chunk)| {
                    for i in 0..out_chunk.len() {
                        let z = in_chunk[i];
                        out_chunk[i] = if input.is_nodata(z) { 1.0 } else { 0.0 };
                    }
                });

            completed_chunks += batch_chunks;
            compute_progress.emit_unit_fraction(ctx.progress, completed_chunks as f64 / total_chunks as f64);
        }
        compute_progress.finish(ctx.progress);

        for (i, value) in out_values.iter().enumerate() {
            output.data.set_f64(i, *value);
        }
        coalescer.emit_unit_fraction(ctx.progress, 0.9);

        let output_locator = write_or_store_output(output, output_path)?;
        ctx.progress.progress(1.0);

        let mut outputs = BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator.clone()));
        outputs.insert(
            "output".to_string(),
            json!({"__wbw_type__": "raster", "path": output_locator, "active_band": 0}),
        );
        outputs.insert("cells_processed".to_string(), json!(len));
        Ok(ToolRunResult { outputs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wbcore::{CapabilityProvider, ProgressSink};
    use wbraster::{DataType, RasterConfig};

    struct AllowAll;

    impl CapabilityProvider for AllowAll {
        fn has_tool_access(&self, _tool_id: &'static str, _tier: LicenseTier) -> bool {
            true
        }
    }

    struct RecordingProgress {
        percents: Mutex<Vec<f64>>,
    }

    impl RecordingProgress {
        fn new() -> Self {
            Self {
                percents: Mutex::new(Vec::new()),
            }
        }

        fn percents(&self) -> Vec<f64> {
            self.percents.lock().unwrap().clone()
        }
    }

    impl ProgressSink for RecordingProgress {
        fn progress(&self, pct: f64) {
            self.percents.lock().unwrap().push(pct);
        }
    }

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!(
                "wbtools_oss_unary_{}_{}_{}",
                prefix,
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_test_raster(path: &str, rows: usize, cols: usize) {
        let mut r = Raster::new(RasterConfig {
            cols,
            rows,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });

        for row in 0..rows {
            for col in 0..cols {
                let value = if (row + col) % 17 == 0 {
                    -1.0
                } else {
                    (row * cols + col) as f64
                };
                r.set(0, row as isize, col as isize, value).unwrap();
            }
        }

        r.write(path, RasterFormat::GeoTiff).unwrap();
    }

    #[test]
    fn abs_progress_is_monotonic_and_bounded() {
        let td = TempDirGuard::new("abs_progress");
        let input = td.path().join("input.tif");
        write_test_raster(input.to_str().unwrap(), 1024, 1024);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input));

        let caps = AllowAll;
        let progress = RecordingProgress::new();
        let ctx = ToolContext {
            progress: &progress,
            capabilities: &caps,
        };

        let tool = RasterAbsTool;
        let _ = tool.run(&args, &ctx).expect("abs should run");

        let percents = progress.percents();
        assert!(!percents.is_empty(), "expected at least one progress event");
        assert!(percents.len() <= 101, "progress events should be bounded to percent buckets");

        for window in percents.windows(2) {
            assert!(window[1] >= window[0], "progress should be monotonic non-decreasing");
        }

        let final_pct = *percents.last().unwrap();
        assert!((final_pct - 1.0).abs() < 1e-9, "final progress should be 100%");
    }

    #[test]
    fn is_nodata_progress_is_monotonic_and_bounded() {
        let td = TempDirGuard::new("is_nodata_progress");
        let input = td.path().join("input.tif");
        write_test_raster(input.to_str().unwrap(), 1024, 1024);

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input));

        let caps = AllowAll;
        let progress = RecordingProgress::new();
        let ctx = ToolContext {
            progress: &progress,
            capabilities: &caps,
        };

        let tool = RasterIsNodataTool;
        let _ = tool.run(&args, &ctx).expect("is_nodata should run");

        let percents = progress.percents();
        assert!(!percents.is_empty(), "expected at least one progress event");
        assert!(percents.len() <= 101, "progress events should be bounded to percent buckets");

        for window in percents.windows(2) {
            assert!(window[1] >= window[0], "progress should be monotonic non-decreasing");
        }

        let final_pct = *percents.last().unwrap();
        assert!((final_pct - 1.0).abs() < 1e-9, "final progress should be 100%");
    }
}
