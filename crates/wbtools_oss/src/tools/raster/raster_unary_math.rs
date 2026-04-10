use std::collections::BTreeMap;
use std::path::Path;

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
use rayon::prelude::*;
use serde_json::{json, Value};
use wbcore::{
    LicenseTier, Tool, ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest,
    ToolMetadata, ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

struct UnaryRasterMathSpec {
    id: &'static str,
    display_name: &'static str,
    summary: &'static str,
    op: fn(f64) -> f64,
}

const UNARY_MATH_PAR_CHUNK: usize = 16_384;

impl UnaryRasterMathSpec {
    fn simd_supported(&self) -> bool {
        matches!(
            self.id,
            "abs"
                | "square"
                | "negate"
                | "increment"
                | "decrement"
                | "reciprocal"
                | "to_degrees"
                | "to_radians"
        )
    }
}

fn compute_unary_chunk_scalar(spec: &UnaryRasterMathSpec, nodata: f64, input: &[f64], output: &mut [f64]) {
    for i in 0..output.len() {
        let z = input[i];
        output[i] = if z == nodata { nodata } else { (spec.op)(z) };
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn compute_unary_chunk_avx(
    spec: &UnaryRasterMathSpec,
    nodata: f64,
    input: &[f64],
    output: &mut [f64],
) {
    let len = output.len();
    let mut i = 0usize;
    let nodata_v = _mm256_set1_pd(nodata);
    let one = _mm256_set1_pd(1.0);
    let deg = _mm256_set1_pd(180.0 / std::f64::consts::PI);
    let rad = _mm256_set1_pd(std::f64::consts::PI / 180.0);
    let sign_mask = _mm256_set1_pd(-0.0);
    let zero = _mm256_set1_pd(0.0);

    while i + 4 <= len {
        let a = _mm256_loadu_pd(input.as_ptr().add(i));
        let nodata_mask = _mm256_cmp_pd(a, nodata_v, _CMP_EQ_OQ);
        let result = match spec.id {
            "abs" => _mm256_andnot_pd(sign_mask, a),
            "square" => _mm256_mul_pd(a, a),
            "negate" => _mm256_sub_pd(zero, a),
            "increment" => _mm256_add_pd(a, one),
            "decrement" => _mm256_sub_pd(a, one),
            "reciprocal" => _mm256_div_pd(one, a),
            "to_degrees" => _mm256_mul_pd(a, deg),
            "to_radians" => _mm256_mul_pd(a, rad),
            _ => unreachable!(),
        };
        let blended = _mm256_blendv_pd(result, nodata_v, nodata_mask);
        _mm256_storeu_pd(output.as_mut_ptr().add(i), blended);
        i += 4;
    }

    if i < len {
        compute_unary_chunk_scalar(spec, nodata, &input[i..], &mut output[i..]);
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn compute_unary_chunk_neon(
    spec: &UnaryRasterMathSpec,
    nodata: f64,
    input: &[f64],
    output: &mut [f64],
) {
    let len = output.len();
    let mut i = 0usize;
    let nodata_v = vdupq_n_f64(nodata);
    let one = vdupq_n_f64(1.0);
    let deg = vdupq_n_f64(180.0 / std::f64::consts::PI);
    let rad = vdupq_n_f64(std::f64::consts::PI / 180.0);
    let zero = vdupq_n_f64(0.0);

    while i + 2 <= len {
        let a = vld1q_f64(input.as_ptr().add(i));
        let nodata_mask = vceqq_f64(a, nodata_v);
        let result = match spec.id {
            "abs" => vabsq_f64(a),
            "square" => vmulq_f64(a, a),
            "negate" => vsubq_f64(zero, a),
            "increment" => vaddq_f64(a, one),
            "decrement" => vsubq_f64(a, one),
            "reciprocal" => vdivq_f64(one, a),
            "to_degrees" => vmulq_f64(a, deg),
            "to_radians" => vmulq_f64(a, rad),
            _ => unreachable!(),
        };
        let blended = vbslq_f64(nodata_mask, nodata_v, result);
        vst1q_f64(output.as_mut_ptr().add(i), blended);
        i += 2;
    }

    if i < len {
        compute_unary_chunk_scalar(spec, nodata, &input[i..], &mut output[i..]);
    }
}

fn compute_unary_chunk(spec: &UnaryRasterMathSpec, nodata: f64, input: &[f64], output: &mut [f64]) {
    if !spec.simd_supported() || nodata.is_nan() {
        compute_unary_chunk_scalar(spec, nodata, input, output);
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx") {
            unsafe {
                compute_unary_chunk_avx(spec, nodata, input, output);
            }
            return;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            compute_unary_chunk_neon(spec, nodata, input, output);
        }
        return;
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    compute_unary_chunk_scalar(spec, nodata, input, output);
}

fn parse_input_output(args: &ToolArgs) -> Result<(&str, &str), ToolError> {
    let input = args
        .get("input")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Validation("missing required string parameter 'input'".to_string()))?;
    let output = args
        .get("output")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Validation("missing required string parameter 'output'".to_string()))?;
    Ok((input, output))
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
                description: "Output raster file path.",
                required: true,
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
                description: "Output raster file path.".to_string(),
                required: true,
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
    let (input_path, output_path) = parse_input_output(args)?;

    ctx.progress
        .info(&format!("running {}", spec.id));

    let input = Raster::read(input_path)
        .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
    let mut output = input.clone();

    let len = output.data.len();
    let in_values: Vec<f64> = (0..len).into_par_iter().map(|i| input.data.get_f64(i)).collect();
    let mut out_values = vec![input.nodata; len];
    out_values
        .par_chunks_mut(UNARY_MATH_PAR_CHUNK)
        .zip(in_values.par_chunks(UNARY_MATH_PAR_CHUNK))
        .for_each(|(out_chunk, in_chunk)| {
            compute_unary_chunk(spec, input.nodata, in_chunk, out_chunk);
        });
    ctx.progress.progress(0.75);

    for (i, value) in out_values.iter().enumerate() {
        output.data.set_f64(i, *value);
    }

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

    let mut outputs = BTreeMap::new();
    outputs.insert("output".to_string(), json!(output_path));
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
                let _ = parse_input_output(args)?;
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
                ToolParamSpec { name: "output", description: "Output raster file path.", required: true },
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
                    description: "Output raster file path.".to_string(),
                    required: true,
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
        let _ = parse_input_output(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input_path, output_path) = parse_input_output(args)?;
        ctx.progress.info("running is_nodata");

        let input = Raster::read(input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {e}")))?;
        let mut output = input.clone();
        let len = output.data.len();

        let in_values: Vec<f64> = (0..len).into_par_iter().map(|i| input.data.get_f64(i)).collect();
        let out_values: Vec<f64> = in_values
            .par_iter()
            .map(|&z| if input.is_nodata(z) { 1.0 } else { 0.0 })
            .collect();

        for (i, value) in out_values.iter().enumerate() {
            output.data.set_f64(i, *value);
        }

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

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), json!(output_path));
        outputs.insert("cells_processed".to_string(), json!(len));
        Ok(ToolRunResult { outputs })
    }
}
