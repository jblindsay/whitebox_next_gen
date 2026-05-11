use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use evalexpr::{build_operator_tree, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value as EvalValue};
use nalgebra::{DMatrix, DVector};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use rayon::prelude::*;
use rand::seq::SliceRandom;
use rand::RngExt;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, NodataPolicy, Raster, RasterConfig, RasterFormat, ResampleMethod};

use crate::memory_store;
use crate::rendering::html::get_css;
use crate::rendering::{Histogram, Scattergram};
use crate::tools::raster_stack_validator::{
    align_and_validate_raster_stack, parse_resample_method, RasterStackConfig,
};

const PCA_SIMD_CHUNK: usize = 2048;

fn weighted_sum_chunked(
    inputs: &[Raster],
    weights: &[f64],
    nodata: f64,
    n: usize,
) -> Vec<f64> {
    let mut output = vec![nodata; n];
    output
        .par_chunks_mut(PCA_SIMD_CHUNK)
        .enumerate()
        .for_each(|(chunk_idx, out_chunk)| {
            let start = chunk_idx * PCA_SIMD_CHUNK;
            for (i, dst) in out_chunk.iter_mut().enumerate() {
                let idx = start + i;
                let base = inputs[0].data.get_f64(idx);
                if base == nodata {
                    *dst = nodata;
                    continue;
                }
                let mut sum = 0.0;
                for (k, &w) in weights.iter().enumerate() {
                    if w != 0.0 {
                        sum += inputs[k].data.get_f64(idx) * w;
                    }
                }
                *dst = sum;
            }
        });
    output
}

fn load_raster(path: &str, param_name: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' has malformed in-memory raster path",
                param_name
            ))
        })?;
        return memory_store::get_raster_by_id(id).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' references unknown in-memory raster id '{}': store entry is missing",
                param_name, id
            ))
        });
    }

    Raster::read(path).map_err(|e| {
        ToolError::Execution(format!("failed reading {} raster: {}", param_name, e))
    })
}

fn load_raster_arc(path: &str, param_name: &str) -> Result<Arc<Raster>, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' has malformed in-memory raster path",
                param_name
            ))
        })?;
        return memory_store::get_raster_arc_by_id(id).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' references unknown in-memory raster id '{}': store entry is missing",
                param_name, id
            ))
        });
    }

    Raster::read(path)
        .map(Arc::new)
        .map_err(|e| {
            ToolError::Execution(format!("failed reading {} raster: {}", param_name, e))
        })
}

fn load_vector(path: &str, param_name: &str) -> Result<wbvector::Layer, ToolError> {
    if wbvector::memory_store::vector_is_memory_path(path) {
        let id = wbvector::memory_store::vector_path_to_id(path).ok_or_else(|| {
            ToolError::Validation(format!(
                "parameter '{}' has malformed in-memory vector path",
                param_name
            ))
        })?;
        return wbvector::memory_store::get_vector_arc_by_id(id)
            .map(|layer| layer.as_ref().clone())
            .ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' references unknown in-memory vector id '{}': store entry is missing",
                    param_name, id
                ))
            });
    }

    wbvector::read(path)
        .map_err(|e| ToolError::Execution(format!("failed reading {} vector: {}", param_name, e)))
}

fn write_or_store_output(output: Raster, output_path: Option<std::path::PathBuf>) -> Result<String, ToolError> {
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

fn parse_optional_html_report_path(args: &ToolArgs) -> Result<Option<String>, ToolError> {
    let path_opt = args
        .get("output")
        .or_else(|| args.get("output_html_file"))
        .or_else(|| args.get("output_html"));

    match path_opt {
        None => Ok(None),
        Some(value) => {
            let path = value
                .as_str()
                .ok_or_else(|| {
                    ToolError::Validation(
                        "HTML output path must be a string for 'output', 'output_html_file', or 'output_html'".to_string(),
                    )
                })?
                .trim();

            if path.is_empty() {
                return Err(ToolError::Validation("HTML output path cannot be empty".to_string()));
            }

            if path.to_lowercase().ends_with(".html") {
                Ok(Some(path.to_string()))
            } else {
                Ok(Some(format!("{path}.html")))
            }
        }
    }
}

fn write_html_report(path: &str, html: &str) -> Result<String, ToolError> {
    let output_path = std::path::Path::new(path);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Execution(format!("failed creating HTML output directory: {e}")))?;
        }
    }

    std::fs::write(output_path, html)
        .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {e}")))?;

    Ok(output_path.to_string_lossy().to_string())
}

fn html_document(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">\n<head>\n<meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\">\n<title>{title}</title>{}\n</head>\n<body>\n{}\n</body>",
        get_css(),
        body
    )
}

fn anova_p_string(p: f64) -> String {
    if p == 0.0 {
        "< .0001".to_string()
    } else if p > 0.01 {
        format!("{p:.4}")
    } else {
        format!("{p:.4e}")
    }
}

pub struct RasterSummaryStatsTool;
pub struct RasterHistogramTool;
pub struct ListUniqueValuesRasterTool;
pub struct ZScoresTool;
pub struct RescaleValueRangeTool;
pub struct MaxTool;
pub struct MinTool;
pub struct QuantilesTool;
pub struct ListUniqueValuesTool;
pub struct RootMeanSquareErrorTool;
pub struct RandomFieldTool;
pub struct RandomSampleTool;
pub struct CumulativeDistributionTool;
pub struct CrispnessIndexTool;
pub struct KsNormalityTestTool;
pub struct InPlaceAddTool;
pub struct InPlaceSubtractTool;
pub struct InPlaceMultiplyTool;
pub struct InPlaceDivideTool;
pub struct AttributeHistogramTool;
pub struct AttributeScattergramTool;
pub struct AttributeCorrelationTool;
pub struct CrossTabulationTool;
pub struct KappaIndexTool;
pub struct PairedSampleTTestTool;
pub struct TwoSampleKsTestTool;
pub struct WilcoxonSignedRankTestTool;
pub struct ConditionalEvaluationTool;
pub struct AnovaTool;
pub struct PhiCoefficientTool;
pub struct ImageCorrelationTool;
pub struct ImageAutocorrelationTool;
pub struct ImageCorrelationNeighbourhoodAnalysisTool;
pub struct ImageRegressionTool;
pub struct DbscanTool;
pub struct ZonalStatisticsTool;
pub struct FftRandomFieldTool;
pub struct TurningBandsSimulationTool;
pub struct TrendSurfaceTool;
pub struct TrendSurfaceVectorPointsTool;
pub struct RasterCalculatorTool;
pub struct PrincipalComponentAnalysisTool;
pub struct InversePcaTool;

enum RasterOrConstant {
    Raster(String),
    Constant(f64),
}

enum ConditionalValueSource {
    Constant(f64),
    Raster(Raster),
    Expr(evalexpr::Node),
}

fn parse_raster_or_constant_arg(args: &ToolArgs, key: &str) -> Result<RasterOrConstant, ToolError> {
    let v = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;

    if let Some(n) = v.as_f64() {
        return Ok(RasterOrConstant::Constant(n));
    }

    if let Some(s) = v.as_str() {
        if let Ok(n) = s.parse::<f64>() {
            Ok(RasterOrConstant::Constant(n))
        } else {
            Ok(RasterOrConstant::Raster(s.to_string()))
        }
    } else {
        Err(ToolError::Validation(format!(
            "parameter '{}' must be a raster path string or numeric constant",
            key
        )))
    }
}

fn parse_raster_input_list(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
    let v = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;

    if let Some(arr) = v.as_array() {
        let mut out = Vec::<String>::new();
        for item in arr {
            let s = item.as_str().ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' items must be raster path strings",
                    key
                ))
            })?;
            let t = s.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{}' must contain at least one raster path",
                key
            )));
        }
        return Ok(out);
    }

    let s = v.as_str().ok_or_else(|| {
        ToolError::Validation(format!(
            "parameter '{}' must be a raster-path list (array or delimited string)",
            key
        ))
    })?;

    let parts: Vec<String> = s
        .split([';', ','])
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();

    if parts.is_empty() {
        return Err(ToolError::Validation(format!(
            "parameter '{}' must contain at least one raster path",
            key
        )));
    }
    Ok(parts)
}

fn normalize_conditional_expression(s: &str) -> String {
    s.replace("NoData", "nodata")
        .replace("Nodata", "nodata")
        .replace("NODATA", "nodata")
        .replace("NULL", "nodata")
        .replace("Null", "nodata")
        .replace("null", "nodata")
        .replace("COLS", "columns")
        .replace("Cols", "columns")
        .replace("cols", "columns")
        .replace("Columns", "columns")
        .replace("COL", "column")
        .replace("Col", "column")
        .replace("col", "column")
        .replace("ROWS", "rows")
        .replace("Rows", "rows")
        .replace("ROW", "row")
        .replace("Row", "row")
        .replace(" or ", " || ")
        .replace(" OR ", " || ")
        .replace(" and ", " && ")
        .replace(" AND ", " && ")
        .replace("pi()", "pi")
        .replace("e()", "e")
}

fn eval_value_to_bool(v: EvalValue) -> Result<bool, ToolError> {
    match v {
        EvalValue::Boolean(b) => Ok(b),
        EvalValue::Int(i) => Ok(i != 0),
        EvalValue::Float(f) => Ok(f != 0.0),
        _ => Err(ToolError::Execution(
            "conditional expression must evaluate to boolean or numeric".to_string(),
        )),
    }
}

fn eval_value_to_f64(v: EvalValue) -> Result<f64, ToolError> {
    match v {
        EvalValue::Int(i) => Ok(i as f64),
        EvalValue::Float(f) => Ok(f),
        EvalValue::Boolean(b) => Ok(if b { 1.0 } else { 0.0 }),
        _ => Err(ToolError::Execution(
            "true/false expression must evaluate to numeric or boolean".to_string(),
        )),
    }
}

fn parse_conditional_value_source(
    args: &ToolArgs,
    key: &str,
    input: &Raster,
) -> Result<ConditionalValueSource, ToolError> {
    let fallback_nodata = || Ok(ConditionalValueSource::Constant(input.nodata));

    let Some(raw) = args.get(key) else {
        return fallback_nodata();
    };

    if let Some(v) = raw.as_f64() {
        return Ok(ConditionalValueSource::Constant(v));
    }

    let Some(s0) = raw.as_str() else {
        return Err(ToolError::Validation(format!(
            "parameter '{}' must be a numeric constant, raster path, or expression string",
            key
        )));
    };

    let s = s0.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("nodata") || s.eq_ignore_ascii_case("null") {
        return fallback_nodata();
    }

    if let Ok(v) = s.parse::<f64>() {
        return Ok(ConditionalValueSource::Constant(v));
    }

    if memory_store::raster_is_memory_path(s) || std::path::Path::new(s).exists() {
        let r = load_raster(s, key)?;
        if r.rows != input.rows || r.cols != input.cols || r.bands != input.bands {
            return Err(ToolError::Validation(format!(
                "parameter '{}' raster must match input rows, columns, and bands",
                key
            )));
        }
        return Ok(ConditionalValueSource::Raster(r));
    }

    let expr = normalize_conditional_expression(s);
    let tree = build_operator_tree::<DefaultNumericTypes>(&expr)
        .map_err(|e| ToolError::Validation(format!("invalid '{}' expression: {e}", key)))?;
    Ok(ConditionalValueSource::Expr(tree))
}

fn resolve_conditional_value(
    source: &ConditionalValueSource,
    idx: usize,
    context: &HashMapContext,
) -> Result<f64, ToolError> {
    match source {
        ConditionalValueSource::Constant(v) => Ok(*v),
        ConditionalValueSource::Raster(r) => Ok(r.data.get_f64(idx)),
        ConditionalValueSource::Expr(expr) => {
            let v = expr
                .eval_with_context(context)
                .map_err(|e| ToolError::Execution(format!("expression evaluation failed: {e}")))?;
            eval_value_to_f64(v)
        }
    }
}

fn typed_raster_output(locator: String) -> serde_json::Value {
    json!({"__wbw_type__": "raster", "path": locator, "active_band": 0})
}

fn parse_raster_list_arg(args: &ToolArgs, key: &str) -> Result<Vec<String>, ToolError> {
    let value = args
        .get(key)
        .ok_or_else(|| ToolError::Validation(format!("parameter '{}' is required", key)))?;

    if let Some(s) = value.as_str() {
        let out: Vec<String> = s
            .split(|c| c == ',' || c == ';')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect();
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{}' did not contain any raster paths",
                key
            )));
        }
        return Ok(out);
    }

    if let Some(arr) = value.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, v) in arr.iter().enumerate() {
            let s = v.as_str().ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' array element {} must be a string path",
                    key, i
                ))
            })?;
            let s = s.trim();
            if s.is_empty() {
                return Err(ToolError::Validation(format!(
                    "parameter '{}' array element {} is empty",
                    key, i
                )));
            }
            out.push(s.to_string());
        }
        if out.is_empty() {
            return Err(ToolError::Validation(format!(
                "parameter '{}' did not contain any raster paths",
                key
            )));
        }
        return Ok(out);
    }

    Err(ToolError::Validation(format!(
        "parameter '{}' must be a string list (comma/semicolon-delimited) or an array of strings",
        key
    )))
}

fn sample_standard_normal<R: RngExt + ?Sized>(rng: &mut R) -> f64 {
    let u1: f64 = rng.random::<f64>().clamp(f64::MIN_POSITIVE, 1.0);
    let u2: f64 = rng.random::<f64>();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn fft2_in_place(data: &mut [Complex<f64>], rows: usize, cols: usize, inverse: bool) {
    let mut planner = FftPlanner::<f64>::new();
    let row_fft = if inverse {
        planner.plan_fft_inverse(cols)
    } else {
        planner.plan_fft_forward(cols)
    };

    for row in data.chunks_exact_mut(cols) {
        row_fft.process(row);
    }

    let col_fft = if inverse {
        planner.plan_fft_inverse(rows)
    } else {
        planner.plan_fft_forward(rows)
    };

    let mut col_buffer = vec![Complex::new(0.0, 0.0); rows];
    for col in 0..cols {
        for row in 0..rows {
            col_buffer[row] = data[row * cols + col];
        }
        col_fft.process(&mut col_buffer);
        for row in 0..rows {
            data[row * cols + col] = col_buffer[row];
        }
    }
}

fn write_inplace_raster(raster: Raster, input1_path: &str) -> Result<String, ToolError> {
    if memory_store::raster_is_memory_path(input1_path) {
        let id = memory_store::put_raster(raster);
        return Ok(memory_store::make_raster_memory_path(&id));
    }

    let output_format = RasterFormat::for_output_path(input1_path)
        .map_err(|e| ToolError::Validation(format!("unsupported input1 path: {e}")))?;
    raster
        .write(input1_path, output_format)
        .map_err(|e| ToolError::Execution(format!("failed writing in-place raster: {e}")))?;
    Ok(input1_path.to_string())
}

fn run_inplace_binary_op<F>(args: &ToolArgs, tool_id: &str, op: F) -> Result<ToolRunResult, ToolError>
where
    F: Fn(f64, f64, f64, bool) -> Option<f64> + Sync,
{
    let input1_path = parse_raster_path_arg(args, "input1")?;
    let input2 = parse_raster_or_constant_arg(args, "input2")?;
    let mut in1 = (*load_raster_arc(&input1_path, "input1")?).clone();

    match input2 {
        RasterOrConstant::Constant(c) => {
            if tool_id == "inplace_divide" && c == 0.0 {
                return Err(ToolError::Validation("illegal division by zero".to_string()));
            }
            let out_values: Vec<f64> = (0..in1.data.len())
                .into_par_iter()
                .map(|i| {
                    let a = in1.data.get_f64(i);
                    if in1.is_nodata(a) {
                        a
                    } else {
                        op(a, c, in1.nodata, false).unwrap_or(in1.nodata)
                    }
                })
                .collect();
            for (i, out) in out_values.into_iter().enumerate() {
                in1.data.set_f64(i, out);
            }
        }
        RasterOrConstant::Raster(path2) => {
            let in2 = load_raster(&path2, "input2")?;
            if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
                return Err(ToolError::Validation(
                    "input files must have the same rows, columns, and bands".to_string(),
                ));
            }
            let out_values: Vec<f64> = (0..in1.data.len())
                .into_par_iter()
                .map(|i| {
                    let a = in1.data.get_f64(i);
                    let b = in2.data.get_f64(i);
                    if in1.is_nodata(a) {
                        a
                    } else if in2.is_nodata(b) {
                        in1.nodata
                    } else {
                        op(a, b, in1.nodata, true).unwrap_or(in1.nodata)
                    }
                })
                .collect();
            for (i, out) in out_values.into_iter().enumerate() {
                in1.data.set_f64(i, out);
            }
        }
    }

    let locator = write_inplace_raster(in1, &input1_path)?;
    let mut outputs = BTreeMap::new();
    outputs.insert("output".to_string(), typed_raster_output(locator));
    Ok(ToolRunResult { outputs })
}

fn normal_cdf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let poly = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 1.0 - pdf * poly;
    if x >= 0.0 { cdf } else { 1.0 - cdf }
}

fn two_tailed_normal_p(z: f64) -> f64 {
    (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0)
}

fn calculate_ks_p_value(alam: f64) -> f64 {
    let mut fac = 2.0f64;
    let mut sum = 0.0f64;
    let mut termbf = 0.0f64;
    let eps1 = 0.001f64;
    let eps2 = 1.0e-8f64;
    let a2 = -2.0 * alam * alam;
    for j in 1..=100 {
        let term = fac * (a2 * (j * j) as f64).exp();
        sum += term;
        if term.abs() <= eps1 * termbf || term.abs() <= eps2 * sum.abs() {
            return sum.clamp(0.0, 1.0);
        }
        fac = -fac;
        termbf = term.abs();
    }
    1.0
}

fn anova_f_call(x: f64) -> f64 {
    if x >= 0.0 {
        x + 0.0000005
    } else {
        x - 0.0000005
    }
}

fn anova_lj_spin(q: f64, i: f64, j: f64, b: f64) -> f64 {
    let mut zz = 1.0;
    let mut z = zz;
    let mut k = i;
    while k <= j {
        zz = zz * q * k / (k - b);
        z += zz;
        k += 2.0;
    }
    z
}

fn anova_f_spin(f: f64, df1: usize, df2: usize) -> f64 {
    let pj2 = std::f64::consts::PI / 2.0;
    let x = df2 as f64 / (df1 as f64 * f + df2 as f64);
    if (df1 as f64 % 2.0) == 0.0 {
        return anova_lj_spin(
            1.0 - x,
            df2 as f64,
            df1 as f64 + df2 as f64 - 4.0,
            df2 as f64 - 2.0,
        ) * x.powf(df2 as f64 / 2.0);
    }
    if (df2 as f64 % 2.0) == 0.0 {
        return 1.0
            - anova_lj_spin(
                x,
                df1 as f64,
                df1 as f64 + df2 as f64 - 4.0,
                df1 as f64 - 2.0,
            ) * (1.0 - x).powf(df1 as f64 / 2.0);
    }

    let tan = ((df1 as f64 * f / df2 as f64).sqrt()).atan();
    let mut a = tan / pj2;
    let sat = tan.sin();
    let cot = tan.cos();
    if df2 as f64 > 1.0 {
        a += sat * cot * anova_lj_spin(cot * cot, 2.0, df2 as f64 - 3.0, -1.0) / pj2;
    }
    if df1 == 1 {
        return 1.0 - a;
    }

    let mut c =
        4.0 * anova_lj_spin(sat * sat, df2 as f64 + 1.0, df1 as f64 + df2 as f64 - 4.0, df2 as f64 - 2.0)
            * sat
            * cot.powf(df2 as f64)
            / std::f64::consts::PI;
    if df2 == 1 {
        return 1.0 - a + c / 2.0;
    }
    let mut k = 2.0;
    while k <= (df2 as f64 - 1.0) / 2.0 {
        c = c * k / (k - 0.5);
        k += 1.0;
    }
    1.0 - a + c
}

fn collect_valid_values(r: &Raster) -> Vec<f64> {
    (0..r.data.len())
        .into_par_iter()
        .filter_map(|i| {
            let z = r.data.get_f64(i);
            if r.is_nodata(z) {
                None
            } else {
                Some(z)
            }
        })
        .collect()
}

fn sample_with_replacement(values: &[f64], count: usize) -> Vec<f64> {
    (0..count)
        .into_par_iter()
        .map(|_| {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..values.len());
            values[idx]
        })
        .collect()
}

fn collect_paired_differences(in1: &Raster, in2: &Raster) -> Vec<f64> {
    (0..in1.data.len())
        .into_par_iter()
        .filter_map(|i| {
            let a = in1.data.get_f64(i);
            let b = in2.data.get_f64(i);
            if in1.is_nodata(a) || in2.is_nodata(b) {
                None
            } else {
                Some(b - a)
            }
        })
        .collect()
}

fn two_sample_ks_statistic(data1: &[f64], data2: &[f64]) -> (f64, f64) {
    let mut v1 = data1.to_vec();
    let mut v2 = data2.to_vec();
    v1.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v2.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n1 = v1.len();
    let n2 = v2.len();
    let en1 = n1 as f64;
    let en2 = n2 as f64;

    let mut j1 = 0usize;
    let mut j2 = 0usize;
    let mut fn1 = 0.0f64;
    let mut fn2 = 0.0f64;
    let mut dmax = 0.0f64;

    while j1 < n1 && j2 < n2 {
        let d1 = v1[j1];
        let d2 = v2[j2];
        if d1 <= d2 {
            j1 += 1;
            fn1 = j1 as f64 / en1;
        }
        if d2 <= d1 {
            j2 += 1;
            fn2 = j2 as f64 / en2;
        }
        dmax = dmax.max((fn2 - fn1).abs());
    }

    let en = (en1 * en2 / (en1 + en2)).sqrt();
    let p = calculate_ks_p_value(en * dmax);
    (dmax, p)
}

fn ranked_values(values: &[f64]) -> (Vec<f64>, usize) {
    if values.is_empty() {
        return (Vec::new(), 0);
    }

    let mut indexed: Vec<(usize, f64)> = values.iter().copied().enumerate().collect();
    indexed.par_sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0f64; values.len()];
    let mut i = 0usize;
    let mut ties = 0usize;
    while i < indexed.len() {
        let mut j = i;
        while j + 1 < indexed.len() && indexed[j + 1].1 == indexed[i].1 {
            j += 1;
        }

        let rank_start = i as f64 + 1.0;
        let rank_end = j as f64 + 1.0;
        let avg_rank = (rank_start + rank_end) / 2.0;
        for k in i..=j {
            ranks[indexed[k].0] = avg_rank;
        }

        if j > i {
            ties += j - i;
        }
        i = j + 1;
    }

    (ranks, ties)
}

fn pearson_from_pairs(x: &[f64], y: &[f64]) -> Option<(f64, usize)> {
    if x.len() != y.len() || x.len() < 3 {
        return None;
    }

    let n = x.len();
    let (sum_x, sum_y) = x
        .par_iter()
        .zip(y.par_iter())
        .map(|(a, b)| (*a, *b))
        .reduce(|| (0.0f64, 0.0f64), |u, v| (u.0 + v.0, u.1 + v.1));
    let mean_x = sum_x / n as f64;
    let mean_y = sum_y / n as f64;
    let (dev_x, dev_y, dev_xy) = x
        .par_iter()
        .zip(y.par_iter())
        .map(|(xa, ya)| {
            let dx = *xa - mean_x;
            let dy = *ya - mean_y;
            (dx * dx, dy * dy, dx * dy)
        })
        .reduce(
            || (0.0f64, 0.0f64, 0.0f64),
            |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
        );

    if dev_x <= 0.0 || dev_y <= 0.0 {
        return None;
    }
    Some((dev_xy / (dev_x * dev_y).sqrt(), n))
}

fn spearman_from_pairs(x: &[f64], y: &[f64]) -> Option<(f64, usize, usize)> {
    if x.len() != y.len() || x.len() < 3 {
        return None;
    }
    let (rx, tx) = ranked_values(x);
    let (ry, ty) = ranked_values(y);
    let (rho, n) = pearson_from_pairs(&rx, &ry)?;
    Some((rho, n, tx + ty))
}

fn pearson_from_column_pair(columns: &[Vec<f64>], a: usize, b: usize) -> f64 {
    let mut n = 0.0f64;
    let mut mean_x = 0.0f64;
    let mut mean_y = 0.0f64;
    let mut m2_x = 0.0f64;
    let mut m2_y = 0.0f64;
    let mut c_xy = 0.0f64;

    for i in 0..columns[a].len() {
        let x = columns[a][i];
        let y = columns[b][i];
        if !x.is_finite() || !y.is_finite() {
            continue;
        }

        let next_n = n + 1.0;
        let dx = x - mean_x;
        mean_x += dx / next_n;
        let dy = y - mean_y;
        mean_y += dy / next_n;

        m2_x += dx * (x - mean_x);
        m2_y += dy * (y - mean_y);
        c_xy += dx * (y - mean_y);
        n = next_n;
    }

    if n < 2.0 || m2_x <= 0.0 || m2_y <= 0.0 {
        f64::NAN
    } else {
        c_xy / (m2_x * m2_y).sqrt()
    }
}

fn kendall_tau_b_from_pairs(x: &[f64], y: &[f64]) -> Option<(f64, usize)> {
    if x.len() != y.len() || x.len() < 3 {
        return None;
    }

    let n = x.len();
    let (concordant, discordant, ties_x, ties_y) = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut local_concordant = 0usize;
            let mut local_discordant = 0usize;
            let mut local_ties_x = 0usize;
            let mut local_ties_y = 0usize;
            for j in (i + 1)..n {
                let dx = x[i] - x[j];
                let dy = y[i] - y[j];
                if dx == 0.0 && dy == 0.0 {
                    continue;
                }
                if dx == 0.0 {
                    local_ties_x += 1;
                    continue;
                }
                if dy == 0.0 {
                    local_ties_y += 1;
                    continue;
                }
                if dx.signum() == dy.signum() {
                    local_concordant += 1;
                } else {
                    local_discordant += 1;
                }
            }
            (local_concordant, local_discordant, local_ties_x, local_ties_y)
        })
        .reduce(
            || (0usize, 0usize, 0usize, 0usize),
            |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3),
        );

    let numer = concordant as f64 - discordant as f64;
    let n0 = (n * (n - 1) / 2) as f64;
    let denom = ((n0 - ties_x as f64) * (n0 - ties_y as f64)).sqrt();
    if denom <= 0.0 {
        return None;
    }

    Some((numer / denom, n))
}

impl Tool for RasterSummaryStatsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_summary_stats",
            display_name: "Raster Summary Stats",
            summary: "Computes basic summary statistics for valid raster cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));

        ToolManifest {
            id: "raster_summary_stats".to_string(),
            display_name: "Raster Summary Stats".to_string(),
            summary: "Computes basic summary statistics for valid raster cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input raster path.".to_string(),
                required: true,
            }],
            defaults,
            examples: vec![ToolExample {
                name: "basic_raster_summary_stats".to_string(),
                description: "Compute summary statistics for a raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let input = load_raster(&input_path, "input")?;

        let (count, min_val, max_val, sum, sum2) = (0..input.data.len())
            .into_par_iter()
            .fold(
                || (0usize, f64::INFINITY, f64::NEG_INFINITY, 0.0f64, 0.0f64),
                |mut acc, i| {
                    let z = input.data.get_f64(i);
                    if input.is_nodata(z) {
                        return acc;
                    }
                    acc.0 += 1;
                    if z < acc.1 {
                        acc.1 = z;
                    }
                    if z > acc.2 {
                        acc.2 = z;
                    }
                    acc.3 += z;
                    acc.4 += z * z;
                    acc
                },
            )
            .reduce(
                || (0usize, f64::INFINITY, f64::NEG_INFINITY, 0.0f64, 0.0f64),
                |a, b| {
                    (
                        a.0 + b.0,
                        a.1.min(b.1),
                        a.2.max(b.2),
                        a.3 + b.3,
                        a.4 + b.4,
                    )
                },
            );

        if count == 0 {
            return Err(ToolError::Validation(
                "input raster contains no valid cells".to_string(),
            ));
        }

        let mean = sum / count as f64;
        let variance = (sum2 / count as f64 - mean * mean).max(0.0);
        let stdev = variance.sqrt();

        let report = json!({
            "count": count,
            "min": min_val,
            "max": max_val,
            "mean": mean,
            "stdev": stdev,
            "sum": sum,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RasterHistogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_histogram",
            display_name: "Raster Histogram",
            summary: "Builds a fixed-bin histogram for valid raster cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional HTML report output path (alias: output_html_file).",
                    required: false,
                },
                ToolParamSpec {
                    name: "bins",
                    description: "Number of histogram bins (default log2(rows*cols)+1).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("bins".to_string(), json!(0));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("image.tif"));
        example.insert("output".to_string(), json!("raster_histogram.html"));

        ToolManifest {
            id: "raster_histogram".to_string(),
            display_name: "Raster Histogram".to_string(),
            summary: "Builds a fixed-bin histogram for valid raster cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional HTML report output path (alias: output_html_file).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "bins".to_string(),
                    description: "Number of histogram bins (default log2(rows*cols)+1).".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_raster_histogram".to_string(),
                description: "Compute a histogram of raster values.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let input = load_raster(&input_path, "input")?;

        let default_bins = ((input.rows * input.cols) as f64).log2().ceil() as usize + 1;
        let bins = args
            .get("bins")
            .and_then(|v| v.as_u64())
            .map(|v| if v == 0 { default_bins } else { v as usize })
            .unwrap_or(default_bins)
            .max(2);

        let values = collect_valid_values(&input);

        if values.is_empty() {
            return Err(ToolError::Validation(
                "input raster contains no valid cells".to_string(),
            ));
        }

        let min_val = values
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_val = values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let range = (max_val - min_val).max(1e-12);
        let counts = values
            .par_iter()
            .fold(
                || vec![0usize; bins],
                |mut local, z| {
                    let idx = (((*z - min_val) / range) * bins as f64).floor() as isize;
                    let idx = idx.clamp(0, bins as isize - 1) as usize;
                    local[idx] += 1;
                    local
                },
            )
            .reduce(
                || vec![0usize; bins],
                |mut a, b| {
                    for i in 0..bins {
                        a[i] += b[i];
                    }
                    a
                },
            );

        let report = json!({
            "input": input_path,
            "min": min_val,
            "max": max_val,
            "bins": bins,
            "bin_width": range / bins as f64,
            "counts": counts,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let histo = Histogram {
                parent_id: "histo".to_string(),
                width: 700.0,
                height: 500.0,
                freq_data: counts.clone(),
                min_bin_val: min_val,
                bin_width: range / bins as f64,
                x_axis_label: "Image Value (X)".to_string(),
                cumulative: false,
            };

            let body = format!(
                "<h1>Histogram Analysis</h1><p><strong>Image</strong>: {}</p><div id='histo' align=\"center\">{}</div>",
                input_path,
                histo.get_svg()
            );
            let html = html_document("Histogram Analysis", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ListUniqueValuesRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "list_unique_values_raster",
            display_name: "List Unique Values (Raster)",
            summary: "Lists unique valid raster categories and their frequencies.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "strict_parity",
                    description: "When true, return complete category-frequency output (default true).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_values",
                    description: "Maximum unique values to include when strict_parity is false (default 10000).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("strict_parity".to_string(), json!(true));
        defaults.insert("max_values".to_string(), json!(10000));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("classified.tif"));
        example.insert("strict_parity".to_string(), json!(true));

        ToolManifest {
            id: "list_unique_values_raster".to_string(),
            display_name: "List Unique Values (Raster)".to_string(),
            summary: "Lists unique valid raster categories and their frequencies.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "strict_parity".to_string(),
                    description: "When true, return complete category-frequency output (default true).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "max_values".to_string(),
                    description: "Maximum unique values to include when strict_parity is false (default 10000).".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_list_unique_values_raster".to_string(),
                description: "List unique values in a categorical raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let strict_parity = args
            .get("strict_parity")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let max_values = args
            .get("max_values")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10000)
            .max(1);
        let input = load_raster(&input_path, "input")?;

        let mut outputs = BTreeMap::new();

        if strict_parity {
            let freqs_hash = (0..input.data.len())
                .into_par_iter()
                .fold(
                    HashMap::<i64, usize>::new,
                    |mut local, i| {
                        let z = input.data.get_f64(i);
                        if !input.is_nodata(z) {
                            let category = z as i64;
                            *local.entry(category).or_insert(0) += 1;
                        }
                        local
                    },
                )
                .reduce(
                    HashMap::<i64, usize>::new,
                    |mut acc, local| {
                        for (k, v) in local {
                            *acc.entry(k).or_insert(0) += v;
                        }
                        acc
                    },
                );

            let mut freqs = BTreeMap::<i64, usize>::new();
            for (k, v) in freqs_hash {
                freqs.insert(k, v);
            }

            let frequencies: Vec<(i64, usize)> = freqs.iter().map(|(k, v)| (*k, *v)).collect();
            let mut table_csv = String::from("Category,Frequency\n");
            for (category, count) in &frequencies {
                table_csv.push_str(&format!("{},{}\n", category, count));
            }

            let report = json!({
                "mode": "strict_parity",
                "count": frequencies.len(),
                "frequencies": frequencies,
                "truncated": false,
            })
            .to_string();

            outputs.insert("report".to_string(), json!(report));
            outputs.insert("table_csv".to_string(), json!(table_csv));
            return Ok(ToolRunResult { outputs });
        }

        let mut set = BTreeSet::<i64>::new();
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            set.insert(z as i64);
            if set.len() >= max_values {
                break;
            }
        }

        let values: Vec<i64> = set.into_iter().collect();
        let report = json!({
            "mode": "capped_values",
            "count": values.len(),
            "values": values,
            "truncated": values.len() >= max_values,
        })
        .to_string();

        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ZScoresTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "z_scores",
            display_name: "Z Scores",
            summary: "Standardizes raster values to z-scores using global mean and standard deviation.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("output".to_string(), json!("dem_z_scores.tif"));

        ToolManifest {
            id: "z_scores".to_string(),
            display_name: "Z Scores".to_string(),
            summary: "Standardizes raster values to z-scores using global mean and standard deviation.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_z_scores".to_string(),
                description: "Compute z-scores for a raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = load_raster(&input_path, "input")?;

        let (count, sum, sum2) = (0..input.data.len())
            .into_par_iter()
            .fold(
                || (0usize, 0.0f64, 0.0f64),
                |(mut local_count, mut local_sum, mut local_sum2), i| {
                    let z = input.data.get_f64(i);
                    if !input.is_nodata(z) {
                        local_count += 1;
                        local_sum += z;
                        local_sum2 += z * z;
                    }
                    (local_count, local_sum, local_sum2)
                },
            )
            .reduce(
                || (0usize, 0.0f64, 0.0f64),
                |(count_a, sum_a, sum2_a), (count_b, sum_b, sum2_b)| {
                    (count_a + count_b, sum_a + sum_b, sum2_a + sum2_b)
                },
            );

        if count == 0 {
            return Err(ToolError::Validation(
                "input raster contains no valid cells".to_string(),
            ));
        }
        let mean = sum / count as f64;
        let stdev = (sum2 / count as f64 - mean * mean).max(0.0).sqrt().max(1e-12);

        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F32,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        let nodata = input.nodata;
        let output_values: Vec<f64> = (0..input.data.len())
            .into_par_iter()
            .map(|i| {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    nodata
                } else {
                    (z - mean) / stdev
                }
            })
            .collect();

        for (i, z) in output_values.into_iter().enumerate() {
            output.data.set_f64(i, z);
        }

        let output_locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RescaleValueRangeTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "rescale_value_range",
            display_name: "Rescale Value Range",
            summary: "Linearly rescales raster values into a target range.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "out_min",
                    description: "Minimum output value.",
                    required: true,
                },
                ToolParamSpec {
                    name: "out_max",
                    description: "Maximum output value.",
                    required: true,
                },
                ToolParamSpec {
                    name: "clip_min",
                    description: "Optional input minimum for clipping before rescale.",
                    required: false,
                },
                ToolParamSpec {
                    name: "clip_max",
                    description: "Optional input maximum for clipping before rescale.",
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

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("out_min".to_string(), json!(0.0));
        defaults.insert("out_max".to_string(), json!(1.0));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("image.tif"));
        example.insert("out_min".to_string(), json!(0.0));
        example.insert("out_max".to_string(), json!(255.0));
        example.insert("output".to_string(), json!("image_rescaled.tif"));

        ToolManifest {
            id: "rescale_value_range".to_string(),
            display_name: "Rescale Value Range".to_string(),
            summary: "Linearly rescales raster values into a target range.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "out_min".to_string(),
                    description: "Minimum output value.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "out_max".to_string(),
                    description: "Maximum output value.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "clip_min".to_string(),
                    description: "Optional input minimum for clipping before rescale.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "clip_max".to_string(),
                    description: "Optional input maximum for clipping before rescale.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_rescale_value_range".to_string(),
                description: "Rescale raster values to 0-255.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = args
            .get("out_min")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'out_min' is required".to_string()))?;
        let _ = args
            .get("out_max")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'out_max' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let out_min = args
            .get("out_min")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'out_min' is required".to_string()))?;
        let out_max = args
            .get("out_max")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("parameter 'out_max' is required".to_string()))?;
        let clip_min = args.get("clip_min").and_then(|v| v.as_f64());
        let clip_max = args.get("clip_max").and_then(|v| v.as_f64());
        let output_path = parse_optional_output_path(args, "output")?;

        let input = load_raster(&input_path, "input")?;

        let (min_val, max_val) = if clip_min.is_none() && clip_max.is_none() {
            let stats = input.statistics();
            (stats.min, stats.max)
        } else {
            let clip_floor = clip_min.unwrap_or(f64::NEG_INFINITY);
            let clip_ceiling = clip_max.unwrap_or(f64::INFINITY);
            (0..input.data.len())
                .into_par_iter()
                .fold(
                    || (f64::INFINITY, f64::NEG_INFINITY),
                    |(mut local_min, mut local_max), i| {
                        let z = input.data.get_f64(i);
                        if !input.is_nodata(z) {
                            let zz = z.max(clip_floor).min(clip_ceiling);
                            if zz < local_min {
                                local_min = zz;
                            }
                            if zz > local_max {
                                local_max = zz;
                            }
                        }
                        (local_min, local_max)
                    },
                )
                .reduce(
                    || (f64::INFINITY, f64::NEG_INFINITY),
                    |(min_a, max_a), (min_b, max_b)| (min_a.min(min_b), max_a.max(max_b)),
                )
        };

        if !min_val.is_finite() || !max_val.is_finite() {
            return Err(ToolError::Validation(
                "input raster contains no valid cells".to_string(),
            ));
        }

        let denom = (max_val - min_val).max(1e-12);
        let scale = (out_max - out_min) / denom;

        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F32,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        let nodata = input.nodata;
        let clip_floor = clip_min.unwrap_or(f64::NEG_INFINITY);
        let clip_ceiling = clip_max.unwrap_or(f64::INFINITY);
        let output_values: Vec<f64> = if clip_min.is_none() && clip_max.is_none() {
            (0..input.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = input.data.get_f64(i);
                    if input.is_nodata(z) {
                        nodata
                    } else {
                        out_min + (z - min_val) * scale
                    }
                })
                .collect()
        } else {
            (0..input.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = input.data.get_f64(i);
                    if input.is_nodata(z) {
                        nodata
                    } else {
                        let zz = z.max(clip_floor).min(clip_ceiling);
                        out_min + (zz - min_val) * scale
                    }
                })
                .collect()
        };

        for (i, z) in output_values.into_iter().enumerate() {
            output.data.set_f64(i, z);
        }

        let output_locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomFieldTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_field",
            display_name: "Random Field",
            summary: "Creates a raster containing standard normal random values.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster path used for grid geometry.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("input.tif"));

        let mut example = ToolArgs::new();
        example.insert("base".to_string(), json!("input.tif"));
        example.insert("output".to_string(), json!("random_field.tif"));

        ToolManifest {
            id: "random_field".to_string(),
            display_name: "Random Field".to_string(),
            summary: "Creates a raster containing standard normal random values.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster path used for grid geometry.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_random_field".to_string(),
                description: "Create a standard normal random raster using another raster as the grid template.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "random".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "base")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_raster_path_arg(args, "base")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let base = load_raster(&base_path, "base")?;

        let mut output = Raster::new(RasterConfig {
            rows: base.rows,
            cols: base.cols,
            bands: base.bands,
            x_min: base.x_min,
            y_min: base.y_min,
            cell_size: base.cell_size_x,
            cell_size_y: Some(base.cell_size_y),
            nodata: base.nodata,
            data_type: DataType::F32,
            crs: base.crs.clone(),
            metadata: base.metadata.clone(),
        });

        // Generate parallel random F32 values directly into the typed buffer
        // This avoids the per-cell dynamic dispatch overhead of set_f64
        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            use rayon::prelude::*;
            data_slice.par_iter_mut().for_each(|cell| {
                let mut rng = rand::rng();
                *cell = sample_standard_normal(&mut rng) as f32;
            });
        } else {
            // Fallback: shouldn't happen since we just created F32 output above
            let mut rng = rand::rng();
            for i in 0..output.data.len() {
                output.data.set_f64(i, sample_standard_normal(&mut rng));
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for FftRandomFieldTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "fft_random_field",
            display_name: "FFT Random Field",
            summary: "Creates a spatially-autocorrelated random field using FFT spectral synthesis.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base_raster", description: "Base raster path used for grid geometry.", required: true },
                ToolParamSpec { name: "range", description: "Approximate correlation range in map units. Default: 1.0.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("range".to_string(), json!(1.0));

        let mut example = ToolArgs::new();
        example.insert("base_raster".to_string(), json!("dem.tif"));
        example.insert("range".to_string(), json!(250.0));
        example.insert("output".to_string(), json!("fft_random_field.tif"));

        ToolManifest {
            id: "fft_random_field".to_string(),
            display_name: "FFT Random Field".to_string(),
            summary: "Creates a spatially-autocorrelated random field using FFT spectral synthesis.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base_raster".to_string(), description: "Base raster path used for grid geometry.".to_string(), required: true },
                ToolParamDescriptor { name: "range".to_string(), description: "Approximate correlation range in map units. Default: 1.0.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_fft_random_field".to_string(),
                description: "Create an autocorrelated random raster using FFT spectral filtering.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "simulation".to_string(), "fft".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "base_raster")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_raster_path_arg(args, "base_raster")?;
        let range = args.get("range").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let base = load_raster(&base_path, "base_raster")?;
        let rows = base.rows;
        let cols = base.cols;
        let n = rows * cols;

        if rows == 0 || cols == 0 {
            return Err(ToolError::Validation("base_raster must have non-zero rows and columns".to_string()));
        }

        let mut spectral = vec![Complex::new(0.0, 0.0); n];
        spectral.par_iter_mut().for_each(|z| {
            let mut rng = rand::rng();
            z.re = sample_standard_normal(&mut rng);
        });

        fft2_in_place(&mut spectral, rows, cols, false);

        if range > 0.0 {
            let cell_size = base.cell_size_x.abs().max(f64::MIN_POSITIVE);
            let sigma_cells = (range / cell_size).max(f64::MIN_POSITIVE);
            let two_pi = 2.0 * std::f64::consts::PI;

            spectral
                .par_iter_mut()
                .enumerate()
                .for_each(|(idx, z)| {
                    let row = idx / cols;
                    let col = idx % cols;
                    let fy = if row <= rows / 2 {
                        row as f64 / rows as f64
                    } else {
                        (rows - row) as f64 / rows as f64
                    };
                    let fx = if col <= cols / 2 {
                        col as f64 / cols as f64
                    } else {
                        (cols - col) as f64 / cols as f64
                    };
                    let wy = two_pi * fy;
                    let wx = two_pi * fx;
                    let k2 = wx * wx + wy * wy;
                    let gain = (-0.5 * sigma_cells * sigma_cells * k2).exp();
                    *z *= gain;
                });
        }

        fft2_in_place(&mut spectral, rows, cols, true);

        let inv_n = 1.0 / n as f64;
        let mut field = vec![0.0f64; n];
        field
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, v)| *v = spectral[i].re * inv_n);

        let mean = field.par_iter().copied().sum::<f64>() / n as f64;
        let variance = field
            .par_iter()
            .map(|&v| {
                let d = v - mean;
                d * d
            })
            .sum::<f64>()
            / n as f64;
        let stdev = variance.max(0.0).sqrt();

        if stdev > 1.0e-15 {
            field.par_iter_mut().for_each(|v| *v = (*v - mean) / stdev);
        } else {
            field.par_iter_mut().for_each(|v| *v = 0.0);
        }

        let mut output = Raster::new(RasterConfig {
            rows,
            cols,
            bands: 1,
            x_min: base.x_min,
            y_min: base.y_min,
            cell_size: base.cell_size_x,
            cell_size_y: Some(base.cell_size_y),
            nodata: base.nodata,
            data_type: DataType::F32,
            crs: base.crs.clone(),
            metadata: base.metadata.clone(),
            ..Default::default()
        });

        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            data_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cell)| *cell = field[i] as f32);
        } else {
            for i in 0..n {
                output.data.set_f64(i, field[i]);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RandomSampleTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "random_sample",
            display_name: "Random Sample",
            summary: "Creates a raster containing randomly located sample cells with unique IDs.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "base", description: "Base raster path used for grid geometry and valid-cell mask.", required: true },
                ToolParamSpec { name: "num_samples", description: "Number of sample cells to generate.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("base".to_string(), json!("input.tif"));
        defaults.insert("num_samples".to_string(), json!(1000));

        let mut example = ToolArgs::new();
        example.insert("base".to_string(), json!("input.tif"));
        example.insert("num_samples".to_string(), json!(1000));
        example.insert("output".to_string(), json!("random_sample.tif"));

        ToolManifest {
            id: "random_sample".to_string(),
            display_name: "Random Sample".to_string(),
            summary: "Creates a raster containing randomly located sample cells with unique IDs.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster path used for grid geometry and valid-cell mask.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Number of sample cells to generate.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_random_sample".to_string(),
                description: "Create a random sample raster using valid cells from a base raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "random".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "base")?;
        let _ = args
            .get("num_samples")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::Validation("parameter 'num_samples' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let base_path = parse_raster_path_arg(args, "base")?;
        let num_samples = args
            .get("num_samples")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .ok_or_else(|| ToolError::Validation("parameter 'num_samples' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;
        let base = load_raster(&base_path, "base")?;

        let mut valid_indices: Vec<usize> = (0..base.data.len())
            .into_par_iter()
            .filter_map(|i| {
                let z = base.data.get_f64(i);
                if !base.is_nodata(z) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        if num_samples > valid_indices.len() {
            return Err(ToolError::Validation(format!(
                "num_samples ({}) exceeds number of valid raster cells ({})",
                num_samples,
                valid_indices.len()
            )));
        }

        let mut output = Raster::new(RasterConfig {
            rows: base.rows,
            cols: base.cols,
            bands: base.bands,
            x_min: base.x_min,
            y_min: base.y_min,
            cell_size: base.cell_size_x,
            cell_size_y: Some(base.cell_size_y),
            nodata: base.nodata,
            data_type: DataType::F32,
            crs: base.crs.clone(),
            metadata: base.metadata.clone(),
        });
        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            data_slice.par_iter_mut().for_each(|cell| *cell = 0.0);
        } else {
            for i in 0..output.data.len() {
                output.data.set_f64(i, 0.0);
            }
        }

        let mut rng = rand::rng();
        valid_indices.shuffle(&mut rng);
        for (sample_id, idx) in valid_indices.into_iter().take(num_samples).enumerate() {
            output.data.set_f64(idx, (sample_id + 1) as f64);
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for CumulativeDistributionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cumulative_distribution",
            display_name: "Cumulative Distribution",
            summary: "Converts raster values to cumulative distribution probabilities.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("output".to_string(), json!("dem_cdf.tif"));

        ToolManifest {
            id: "cumulative_distribution".to_string(),
            display_name: "Cumulative Distribution".to_string(),
            summary: "Converts raster values to cumulative distribution probabilities.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_cumulative_distribution".to_string(),
                description: "Transform a raster into cumulative probabilities.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let input = load_raster(&input_path, "input")?;

        let (min_val, max_val, num_cells) = (0..input.data.len())
            .into_par_iter()
            .fold(
                || (f64::INFINITY, f64::NEG_INFINITY, 0usize),
                |(mut local_min, mut local_max, mut local_count), i| {
                    let z = input.data.get_f64(i);
                    if !input.is_nodata(z) {
                        local_min = local_min.min(z);
                        local_max = local_max.max(z);
                        local_count += 1;
                    }
                    (local_min, local_max, local_count)
                },
            )
            .reduce(
                || (f64::INFINITY, f64::NEG_INFINITY, 0usize),
                |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
            );

        if num_cells == 0 {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        let mut output = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::F32,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        if (max_val - min_val).abs() < 1.0e-12 {
            let out_values: Vec<f64> = (0..input.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = input.data.get_f64(i);
                    if input.is_nodata(z) { input.nodata } else { 1.0 }
                })
                .collect();
            for (i, out) in out_values.into_iter().enumerate() {
                output.data.set_f64(i, out);
            }
        } else {
            let num_bins = 50_000usize;
            let bin_size = (max_val - min_val) / num_bins as f64;
            let histogram = (0..input.data.len())
                .into_par_iter()
                .fold(
                    || vec![0usize; num_bins],
                    |mut local_hist, i| {
                        let z = input.data.get_f64(i);
                        if !input.is_nodata(z) {
                            let idx = (((z - min_val) / bin_size) as isize)
                                .clamp(0, num_bins as isize - 1)
                                as usize;
                            local_hist[idx] += 1;
                        }
                        local_hist
                    },
                )
                .reduce(
                    || vec![0usize; num_bins],
                    |mut acc, local| {
                        for (dst, src) in acc.iter_mut().zip(local) {
                            *dst += src;
                        }
                        acc
                    },
                );

            let mut cdf = vec![0.0; num_bins];
            let mut running = 0.0;
            for (i, count) in histogram.iter().enumerate() {
                running += *count as f64;
                cdf[i] = running / num_cells as f64;
            }

            let out_values: Vec<f64> = (0..input.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = input.data.get_f64(i);
                    if input.is_nodata(z) {
                        input.nodata
                    } else {
                        let idx = (((z - min_val) / bin_size) as isize)
                            .clamp(0, num_bins as isize - 1)
                            as usize;
                        cdf[idx]
                    }
                })
                .collect();
            for (i, out) in out_values.into_iter().enumerate() {
                output.data.set_f64(i, out);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for CrispnessIndexTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "crispness_index",
            display_name: "Crispness Index",
            summary: "Calculates the crispness index for a membership probability raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("membership.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("membership.tif"));
        example.insert("output".to_string(), json!("crispness_report.html"));

        ToolManifest {
            id: "crispness_index".to_string(),
            display_name: "Crispness Index".to_string(),
            summary: "Calculates the crispness index for a membership probability raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input raster path.".to_string(),
                required: true,
            }, ToolParamDescriptor {
                name: "output".to_string(),
                description: "Optional HTML report output path (alias: output_html_file).".to_string(),
                required: false,
            }],
            defaults,
            examples: vec![ToolExample {
                name: "basic_crispness_index".to_string(),
                description: "Compute the crispness index for a membership probability raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let input = load_raster(&input_path, "input")?;

        let (count, sum, warning) = (0..input.data.len())
            .into_par_iter()
            .map(|i| {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    (0usize, 0.0f64, false)
                } else {
                    (1usize, z, !(0.0..=1.0).contains(&z))
                }
            })
            .reduce(
                || (0usize, 0.0f64, false),
                |(c1, s1, w1), (c2, s2, w2)| (c1 + c2, s1 + s2, w1 || w2),
            );

        if count == 0 {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        let mean = sum / count as f64;
        let ss_mp = (0..input.data.len())
            .into_par_iter()
            .map(|i| {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    0.0
                } else {
                    (z - mean) * (z - mean)
                }
            })
            .sum::<f64>();

        let ss_b = sum * (1.0 - mean) * (1.0 - mean) + (count as f64 - sum) * mean * mean;
        let crispness = if ss_b.abs() < 1.0e-12 { 0.0 } else { ss_mp / ss_b };

        let report = json!({
            "input": input_path,
            "count": count,
            "mean": mean,
            "ss_mp": ss_mp,
            "ss_b": ss_b,
            "crispness_index": crispness,
            "warning_values_outside_probability_range": warning,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let warning_html = if warning {
                "<p><strong>WARNING</strong>: This tool is intended to be applied to membership probability (MP) rasters, with probability values ranging from 0-1. The input image contains values outside this range. <em>Therefore, it is unlikely that the results are meaningful</em>.</p>"
            } else {
                ""
            };
            let body = format!(
                "<h1>Crispness Index Report</h1>\n<p><strong>Input file</strong>: {}</p>\n{}\n<br><table align=\"center\">\n<tr><td><em>SS<sub>mp</sub></em></td><td class=\"numberCell\">{:.4}</td></tr>\n<tr><td><em>SS<sub>B</sub></em></td><td class=\"numberCell\">{:.4}</td></tr>\n<tr><td><em>C</em></td><td class=\"numberCell\">{:.4}</td></tr>\n</table>",
                input_path,
                warning_html,
                ss_mp,
                ss_b,
                crispness
            );
            let html = html_document("Crispness Index", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for KsNormalityTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "ks_normality_test",
            display_name: "K-S Normality Test",
            summary: "Evaluates whether raster values are drawn from a normal distribution.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "num_samples", description: "Optional random sample size. Omit to use all valid cells.", required: false },
                ToolParamSpec { name: "output", description: "Optional HTML report output path (alias: output_html_file).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("input.tif"));
        example.insert("num_samples".to_string(), json!(1000));

        ToolManifest {
            id: "ks_normality_test".to_string(),
            display_name: "K-S Normality Test".to_string(),
            summary: "Evaluates whether raster values are drawn from a normal distribution.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Optional random sample size. Omit to use all valid cells.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_ks_normality_test".to_string(),
                description: "Run a Kolmogorov-Smirnov normality test on raster values.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let requested_samples = args.get("num_samples").and_then(|v| v.as_u64()).map(|v| v as usize);
        let input = load_raster(&input_path, "input")?;

        let valid_values = collect_valid_values(&input);

        if valid_values.is_empty() {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        let values = if let Some(num_samples) = requested_samples {
            if num_samples == 0 {
                return Err(ToolError::Validation("num_samples must be greater than zero when provided".to_string()));
            }
            let mut rng = rand::rng();
            let mut sampled = Vec::with_capacity(num_samples);
            for _ in 0..num_samples {
                let idx = rng.random_range(0..valid_values.len());
                sampled.push(valid_values[idx]);
            }
            sampled
        } else {
            valid_values
        };

        let n = values.len() as f64;
        let min_value: f64 = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max_value: f64 = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = values.iter().sum();
        let mean = sum / n;
        let total_deviation: f64 = values.iter().map(|v| (v - mean) * (v - mean)).sum();
        let std_dev = if values.len() > 1 {
            (total_deviation / (n - 1.0)).sqrt()
        } else {
            0.0
        };

        let mut dmax: f64 = 0.0;
        let mut p_value: f64 = 1.0;

        if std_dev > 0.0 && max_value > min_value {
            let num_bins = 10_000usize;
            let bin_size = (max_value - min_value) / num_bins as f64;
            let histogram = values
                .par_iter()
                .fold(
                    || vec![0usize; num_bins],
                    |mut local, z| {
                        let idx = (((*z - min_value) / bin_size).floor() as isize)
                            .clamp(0, num_bins as isize - 1) as usize;
                        local[idx] += 1;
                        local
                    },
                )
                .reduce(
                    || vec![0usize; num_bins],
                    |mut a, b| {
                        for i in 0..num_bins {
                            a[i] += b[i];
                        }
                        a
                    },
                );

            let mut cdf = vec![0.0; num_bins];
            let mut running = 0.0;
            for (i, count) in histogram.iter().enumerate() {
                running += *count as f64;
                cdf[i] = running / n;
            }

            let sd_root_2pi = std_dev * (2.0 * std::f64::consts::PI).sqrt();
            let two_sd_sqr = 2.0 * std_dev * std_dev;
            let mut normal_cdf = vec![0.0; num_bins];
            for (i, item) in normal_cdf.iter_mut().enumerate() {
                let z = min_value + i as f64 * bin_size;
                *item = (1.0 / sd_root_2pi) * ((-(z - mean) * (z - mean)) / two_sd_sqr).exp();
            }
            for i in 1..num_bins {
                normal_cdf[i] += normal_cdf[i - 1];
            }
            let total = normal_cdf[num_bins - 1].max(1.0e-12);
            for item in &mut normal_cdf {
                *item /= total;
            }

            for i in 0..num_bins {
                dmax = dmax.max((cdf[i] - normal_cdf[i]).abs());
            }

            let s = n * dmax * dmax;
            p_value = 2.0 * (-(2.000_071 + 0.331 / n.sqrt() + 1.409 / n) * s).exp();
            p_value = p_value.clamp(0.0, 1.0);
        }

        let report = json!({
            "input": input_path,
            "num_samples": values.len(),
            "sampled": requested_samples.is_some(),
            "mean": mean,
            "std_dev": std_dev,
            "dmax": dmax,
            "p_value": p_value,
            "reject_normality_at_0_05": p_value < 0.05,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let fd_num_bins = ((values.len() as f64).log2().ceil() as usize + 1).max(2);
            let value_range = (max_value - min_value).max(1.0e-9);
            let fd_bin_width = value_range / fd_num_bins as f64;
            let mut freq_data = vec![0usize; fd_num_bins];
            for z in &values {
                let idx = (((*z - min_value) / fd_bin_width).floor() as isize)
                    .clamp(0, fd_num_bins as isize - 1) as usize;
                freq_data[idx] += 1;
            }

            let p_value_str = if p_value > 0.001 {
                format!("{p_value:.4}")
            } else {
                "&lt;0.001".to_string()
            };

            let result_str = if p_value < 0.05 {
                "The test <strong>rejects</strong> the null hypothesis that the values come from a normal distribution."
            } else {
                "The test <strong>fails to reject</strong> the null hypothesis that the values come from a normal distribution."
            };

            let histo = Histogram {
                parent_id: "histo".to_string(),
                width: 700.0,
                height: 500.0,
                freq_data,
                min_bin_val: min_value,
                bin_width: fd_bin_width,
                x_axis_label: "Value".to_string(),
                cumulative: true,
            };

            let body = format!(
                "<h1>Kolmogorov-Smirnov (K-S) Test for Normality Report</h1>\
                 <p><strong>Input image</strong>: {}<br>\
                 <strong>Sample size (N)</strong>: {:.0}<br>\
                 <strong>Test Statistic (D<sub>max</sub>)</strong>: {:.4}<br>\
                 <strong>Significance (p-value)</strong>: {}<br>\
                 <strong>Result</strong>: {}\
                 </p>\
                 <p><strong>Caveat</strong>: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, and statistical significance says nothing about the practical significance of a difference.</p>\
                 <div id='histo' align=\"center\">{}</div>",
                input_path,
                n,
                dmax,
                p_value_str,
                result_str,
                histo.get_svg()
            );

            let html = html_document("K-S Test for Normality", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for InPlaceAddTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inplace_add",
            display_name: "InPlace Add",
            summary: "Performs an in-place addition operation (input1 += input2).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input raster to modify.", required: true },
                ToolParamSpec { name: "input2", description: "Input raster path or numeric constant.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!(10.5));
        ToolManifest {
            id: "inplace_add".to_string(),
            display_name: "InPlace Add".to_string(),
            summary: "Performs an in-place addition operation (input1 += input2).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster to modify.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster path or numeric constant.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_inplace_add".to_string(), description: "Modify input1 by adding input2.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "math".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_inplace_binary_op(args, "inplace_add", |a, b, _nodata, _is_raster_rhs| Some(a + b))
    }
}

impl Tool for InPlaceSubtractTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inplace_subtract",
            display_name: "InPlace Subtract",
            summary: "Performs an in-place subtraction operation (input1 -= input2).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input raster to modify.", required: true },
                ToolParamSpec { name: "input2", description: "Input raster path or numeric constant.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!(10.5));
        ToolManifest {
            id: "inplace_subtract".to_string(),
            display_name: "InPlace Subtract".to_string(),
            summary: "Performs an in-place subtraction operation (input1 -= input2).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster to modify.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster path or numeric constant.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_inplace_subtract".to_string(), description: "Modify input1 by subtracting input2.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "math".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_inplace_binary_op(args, "inplace_subtract", |a, b, _nodata, _is_raster_rhs| Some(a - b))
    }
}

impl Tool for InPlaceMultiplyTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inplace_multiply",
            display_name: "InPlace Multiply",
            summary: "Performs an in-place multiplication operation (input1 *= input2).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input raster to modify.", required: true },
                ToolParamSpec { name: "input2", description: "Input raster path or numeric constant.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!(10.5));
        ToolManifest {
            id: "inplace_multiply".to_string(),
            display_name: "InPlace Multiply".to_string(),
            summary: "Performs an in-place multiplication operation (input1 *= input2).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster to modify.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster path or numeric constant.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_inplace_multiply".to_string(), description: "Modify input1 by multiplying with input2.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "math".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_inplace_binary_op(args, "inplace_multiply", |a, b, _nodata, _is_raster_rhs| Some(a * b))
    }
}

impl Tool for InPlaceDivideTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inplace_divide",
            display_name: "InPlace Divide",
            summary: "Performs an in-place division operation (input1 /= input2).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input raster to modify.", required: true },
                ToolParamSpec { name: "input2", description: "Input raster path or non-zero numeric constant.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!(10.5));
        ToolManifest {
            id: "inplace_divide".to_string(),
            display_name: "InPlace Divide".to_string(),
            summary: "Performs an in-place division operation (input1 /= input2).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster to modify.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster path or non-zero numeric constant.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_inplace_divide".to_string(), description: "Modify input1 by dividing by input2.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "math".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        run_inplace_binary_op(args, "inplace_divide", |a, b, _nodata, _is_raster_rhs| {
            if b == 0.0 {
                None
            } else {
                Some(a / b)
            }
        })
    }
}

impl Tool for AttributeHistogramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "attribute_histogram",
            display_name: "Attribute Histogram",
            summary: "Creates a histogram for numeric field values in a vector attribute table.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "field", description: "Numeric attribute field name.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.shp"));
        defaults.insert("field".to_string(), json!("HEIGHT"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("lakes.shp"));
        example.insert("field".to_string(), json!("HEIGHT"));
        example.insert("output".to_string(), json!("attribute_histogram.html"));
        ToolManifest {
            id: "attribute_histogram".to_string(),
            display_name: "Attribute Histogram".to_string(),
            summary: "Creates a histogram for numeric field values in a vector attribute table.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field name.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_attribute_histogram".to_string(),
                description: "Generate histogram counts for a numeric vector field.".to_string(),
                args: example,
            }],
            tags: vec!["vector".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;

        let layer = load_vector(&input_path, "input")?;
        let field_idx = layer
            .schema
            .field_index(field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", field)))?;
        let field_type = layer.schema.fields()[field_idx].field_type;
        if !matches!(field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float) {
            return Err(ToolError::Validation(format!("field '{}' must be numeric", field)));
        }

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut valid_count = 0usize;
        for feat in &layer.features {
            if let Some(v) = feat.attributes.get(field_idx).and_then(|v| v.as_f64()) {
                min = min.min(v);
                max = max.max(v);
                valid_count += 1;
            }
        }
        if valid_count == 0 {
            return Err(ToolError::Validation("field contains no numeric values".to_string()));
        }

        let num_bins = (valid_count as f64).log2().ceil().max(1.0) as usize + 1;
        let width = (max - min + 1.0e-5) / num_bins as f64;
        let mut counts = vec![0usize; num_bins];
        for feat in &layer.features {
            if let Some(v) = feat.attributes.get(field_idx).and_then(|v| v.as_f64()) {
                let idx = (((v - min) / width).floor() as isize).clamp(0, num_bins as isize - 1) as usize;
                counts[idx] += 1;
            }
        }

        let report = json!({
            "input": input_path,
            "field": field,
            "min": min,
            "max": max,
            "num_bins": num_bins,
            "bin_width": width,
            "counts": counts,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let histo = Histogram {
                parent_id: "histo".to_string(),
                width: 700.0,
                height: 500.0,
                freq_data: counts,
                min_bin_val: min,
                bin_width: width,
                x_axis_label: field.to_string(),
                cumulative: false,
            };
            let body = format!(
                "<h1>Histogram Analysis</h1>\n<p><strong>Input</strong>: {}</p>\n<p><strong>Field Name</strong>: {}</p>\n<div id='histo' align=\"center\">{}</div>",
                input_path,
                field,
                histo.get_svg()
            );
            let html = html_document("Histogram Analysis", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for AttributeScattergramTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "attribute_scattergram",
            display_name: "Attribute Scattergram",
            summary: "Computes scatterplot summary statistics between two numeric vector fields.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "fieldx", description: "Numeric x-axis field name.", required: true },
                ToolParamSpec { name: "fieldy", description: "Numeric y-axis field name.", required: true },
                ToolParamSpec { name: "trendline", description: "Include trendline summary (default false).", required: false },
                ToolParamSpec { name: "output", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.shp"));
        defaults.insert("fieldx".to_string(), json!("x"));
        defaults.insert("fieldy".to_string(), json!("y"));
        defaults.insert("trendline".to_string(), json!(false));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("lakes.shp"));
        example.insert("fieldx".to_string(), json!("HEIGHT"));
        example.insert("fieldy".to_string(), json!("AREA"));
        example.insert("trendline".to_string(), json!(true));
        example.insert("output".to_string(), json!("attribute_scattergram.html"));
        ToolManifest {
            id: "attribute_scattergram".to_string(),
            display_name: "Attribute Scattergram".to_string(),
            summary: "Computes scatterplot summary statistics between two numeric vector fields.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "fieldx".to_string(), description: "Numeric x-axis field name.".to_string(), required: true },
                ToolParamDescriptor { name: "fieldy".to_string(), description: "Numeric y-axis field name.".to_string(), required: true },
                ToolParamDescriptor { name: "trendline".to_string(), description: "Include trendline summary (default false).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_attribute_scattergram".to_string(),
                description: "Compute scatter summary for two vector attributes.".to_string(),
                args: example,
            }],
            tags: vec!["vector".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = args.get("fieldx").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldx' is required".to_string()))?;
        let _ = args.get("fieldy").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldy' is required".to_string()))?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let fieldx = args.get("fieldx").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldx' is required".to_string()))?;
        let fieldy = args.get("fieldy").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldy' is required".to_string()))?;
        let trendline = args.get("trendline").and_then(|v| v.as_bool()).unwrap_or(false);

        let layer = load_vector(&input_path, "input")?;
        let ix = layer
            .schema
            .field_index(fieldx)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", fieldx)))?;
        let iy = layer
            .schema
            .field_index(fieldy)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", fieldy)))?;

        if !matches!(layer.schema.fields()[ix].field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float) {
            return Err(ToolError::Validation(format!("field '{}' must be numeric", fieldx)));
        }
        if !matches!(layer.schema.fields()[iy].field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float) {
            return Err(ToolError::Validation(format!("field '{}' must be numeric", fieldy)));
        }

        let mut xs = Vec::<f64>::new();
        let mut ys = Vec::<f64>::new();
        for feat in &layer.features {
            let x = feat.attributes.get(ix).and_then(|v| v.as_f64());
            let y = feat.attributes.get(iy).and_then(|v| v.as_f64());
            if let (Some(xv), Some(yv)) = (x, y) {
                xs.push(xv);
                ys.push(yv);
            }
        }
        if xs.is_empty() {
            return Err(ToolError::Validation("no valid paired numeric values found".to_string()));
        }

        let (n, sum_x, sum_y, sum_x2, sum_y2, sum_xy, x_min, x_max, y_min, y_max) = xs
            .par_iter()
            .zip(ys.par_iter())
            .map(|(x, y)| {
                (1usize, *x, *y, *x * *x, *y * *y, *x * *y, *x, *x, *y, *y)
            })
            .reduce(
                || {
                    (
                        0usize,
                        0.0f64,
                        0.0f64,
                        0.0f64,
                        0.0f64,
                        0.0f64,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                    )
                },
                |a, b| {
                    (
                        a.0 + b.0,
                        a.1 + b.1,
                        a.2 + b.2,
                        a.3 + b.3,
                        a.4 + b.4,
                        a.5 + b.5,
                        a.6.min(b.6),
                        a.7.max(b.7),
                        a.8.min(b.8),
                        a.9.max(b.9),
                    )
                },
            );
        let n_f64 = n as f64;
        let mean_x = sum_x / n_f64;
        let mean_y = sum_y / n_f64;
        let sxx = sum_x2 - (sum_x * sum_x) / n_f64;
        let syy = sum_y2 - (sum_y * sum_y) / n_f64;
        let sxy = sum_xy - (sum_x * sum_y) / n_f64;
        let correlation = if sxx > 0.0 && syy > 0.0 {
            sxy / (sxx * syy).sqrt()
        } else {
            0.0
        };

        let (slope, intercept) = if trendline && sxx > 0.0 {
            let m = sxy / sxx;
            (Some(m), Some(mean_y - m * mean_x))
        } else {
            (None, None)
        };

        let report = json!({
            "input": input_path,
            "fieldx": fieldx,
            "fieldy": fieldy,
            "count": xs.len(),
            "correlation": correlation,
            "trendline": trendline,
            "slope": slope,
            "intercept": intercept,
            "x_min": x_min,
            "x_max": x_max,
            "y_min": y_min,
            "y_max": y_max,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let graph = Scattergram {
                parent_id: "graph".to_string(),
                width: 700.0,
                height: 500.0,
                data_x: vec![xs],
                data_y: vec![ys],
                series_labels: vec![format!("Series {} - {}", fieldx, fieldy)],
                x_axis_label: fieldx.to_string(),
                y_axis_label: fieldy.to_string(),
                draw_trendline: trendline,
                draw_gridlines: true,
                draw_legend: false,
                draw_grey_background: false,
            };

            let body = format!(
                "<h1>Scatergram Analysis</h1>\n<p><strong>Input</strong>: {}</p>\n<p><strong>X Field Name</strong>: {}</p>\n<p><strong>Y Field Name</strong>: {}</p>\n<div id='graph' align=\"center\">{}</div>",
                input_path,
                fieldx,
                fieldy,
                graph.get_svg()
            );
            let html = html_document("Scattergram Analysis", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for AttributeCorrelationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "attribute_correlation",
            display_name: "Attribute Correlation",
            summary: "Performs Pearson correlation analysis on numeric vector attribute fields.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamSpec {
                name: "input",
                description: "Input vector path.",
                required: true,
            }, ToolParamSpec {
                name: "output",
                description: "Optional HTML report output path.",
                required: false,
            }],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.shp"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("data.shp"));
        example.insert("output".to_string(), json!("attribute_correlation.html"));
        ToolManifest {
            id: "attribute_correlation".to_string(),
            display_name: "Attribute Correlation".to_string(),
            summary: "Performs Pearson correlation analysis on numeric vector attribute fields.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamDescriptor {
                name: "input".to_string(),
                description: "Input vector path.".to_string(),
                required: true,
            }, ToolParamDescriptor {
                name: "output".to_string(),
                description: "Optional HTML report output path (alias: output_html_file).".to_string(),
                required: false,
            }],
            defaults,
            examples: vec![ToolExample {
                name: "basic_attribute_correlation".to_string(),
                description: "Compute correlation matrix for numeric vector fields.".to_string(),
                args: example,
            }],
            tags: vec!["vector".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let html_output_path = parse_optional_html_report_path(args)?;
        let layer = load_vector(&input_path, "input")?;

        let mut numeric_indices = Vec::<usize>::new();
        let mut field_names = Vec::<String>::new();
        for (i, fd) in layer.schema.fields().iter().enumerate() {
            if matches!(fd.field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float) {
                numeric_indices.push(i);
                field_names.push(fd.name.clone());
            }
        }
        if numeric_indices.len() < 2 {
            return Err(ToolError::Validation("input vector must contain at least two numeric fields".to_string()));
        }

        let mut columns = vec![Vec::<f64>::new(); numeric_indices.len()];
        for feat in &layer.features {
            for (j, idx) in numeric_indices.iter().enumerate() {
                columns[j].push(feat.attributes.get(*idx).and_then(|v| v.as_f64()).unwrap_or(f64::NAN));
            }
        }

        let k = numeric_indices.len();
        let mut matrix = vec![vec![1.0f64; k]; k];
        let pair_corrs: Vec<(usize, usize, f64)> = (0..k)
            .into_par_iter()
            .map(|a| {
                let mut local = Vec::with_capacity(a);
                for b in 0..a {
                    let corr = pearson_from_column_pair(&columns, a, b);

                    local.push((a, b, corr));
                }
                local
            })
            .reduce(Vec::new, |mut a, mut b| {
                a.append(&mut b);
                a
            });

        for (a, b, corr) in pair_corrs {
            matrix[a][b] = corr;
            matrix[b][a] = corr;
        }

        let report = json!({
            "input": input_path,
            "fields": field_names,
            "matrix": matrix,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let mut attributes = String::new();
            for (i, name) in field_names.iter().enumerate() {
                attributes.push_str(&format!("<strong>Field {}</strong>: {}<br>", i + 1, name));
            }

            let mut table = String::from("<table align=\"center\"><caption>Pearson correlation matrix</caption><tr><th></th>");
            for i in 0..field_names.len() {
                table.push_str(&format!("<th>Field {}</th>", i + 1));
            }
            table.push_str("</tr>");

            for (row_idx, row) in matrix.iter().enumerate() {
                table.push_str(&format!("<tr><td><strong>Field {}</strong></td>", row_idx + 1));
                for value in row {
                    if value.is_finite() {
                        table.push_str(&format!("<td>{:.4}</td>", value));
                    } else {
                        table.push_str("<td></td>");
                    }
                }
                table.push_str("</tr>");
            }
            table.push_str("</table>");

            let body = format!(
                "<h1>Attributes Correlation Report</h1>\n<p><strong>Attributes</strong>:<br>{}</p>\n<br>{}",
                attributes,
                table
            );
            let html = html_document("Attribute Correlation", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for CrossTabulationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "cross_tabulation",
            display_name: "Cross Tabulation",
            summary: "Performs cross-tabulation on two categorical rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input raster 1 path.", required: true },
                ToolParamSpec { name: "input2", description: "Input raster 2 path.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("file1.tif"));
        defaults.insert("input2".to_string(), json!("file2.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("class_2000.tif"));
        example.insert("input2".to_string(), json!("class_2020.tif"));
        example.insert("output".to_string(), json!("cross_tabulation.html"));
        ToolManifest {
            id: "cross_tabulation".to_string(),
            display_name: "Cross Tabulation".to_string(),
            summary: "Performs cross-tabulation on two categorical rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster 1 path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster 2 path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_cross_tabulation".to_string(),
                description: "Generate contingency counts between two classified rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let html_output_path = parse_optional_html_report_path(args)?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let counts = (0..in1.data.len())
            .into_par_iter()
            .fold(
                || HashMap::<(i64, i64), usize>::new(),
                |mut acc, i| {
                    let z1 = in1.data.get_f64(i);
                    let z2 = in2.data.get_f64(i);
                    if in1.is_nodata(z1) || in2.is_nodata(z2) {
                        return acc;
                    }
                    let c1 = z1.round() as i64;
                    let c2 = z2.round() as i64;
                    *acc.entry((c2, c1)).or_insert(0) += 1;
                    acc
                },
            )
            .reduce(
                || HashMap::<(i64, i64), usize>::new(),
                |mut a, b| {
                    for (k, v) in b {
                        *a.entry(k).or_insert(0) += v;
                    }
                    a
                },
            );

        let mut row_classes = BTreeSet::<i64>::new();
        let mut col_classes = BTreeSet::<i64>::new();
        for &(r, c) in counts.keys() {
            row_classes.insert(r);
            col_classes.insert(c);
        }

        let counts: BTreeMap<(i64, i64), usize> = counts.into_iter().collect();

        let rows: Vec<i64> = row_classes.into_iter().collect();
        let cols: Vec<i64> = col_classes.into_iter().collect();
        let mut table = vec![vec![0usize; cols.len()]; rows.len()];
        for (ri, rv) in rows.iter().enumerate() {
            for (ci, cv) in cols.iter().enumerate() {
                table[ri][ci] = *counts.get(&(*rv, *cv)).unwrap_or(&0);
            }
        }

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "columns_classes": cols,
            "rows_classes": rows,
            "table": table,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let mut table_html = String::from("<div><table align=\"center\"><caption>Cross Tabulation Results</caption><tr><td></td>");
            for class in &cols {
                table_html.push_str(&format!("<td class=\"header\">{}</td>", class));
            }
            table_html.push_str("</tr>");

            for (ri, row_class) in rows.iter().enumerate() {
                table_html.push_str(&format!("<tr><td class=\"header\">{}</td>", row_class));
                for value in &table[ri] {
                    table_html.push_str(&format!("<td class=\"numberCell\">{}</td>", value));
                }
                table_html.push_str("</tr>");
            }
            table_html.push_str("</table></div>");

            let body = format!(
                "<h1>Cross Tabulation Report</h1>\n<p><strong>Image 1</strong> (columns): {}</p>\n<p><strong>Image 2</strong> (rows): {}</p>\n{}",
                input1_path,
                input2_path,
                table_html
            );
            let html = html_document("Cross Tabulation", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for AnovaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "anova",
            display_name: "ANOVA",
            summary: "Performs one-way ANOVA on raster values grouped by class raster categories.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Measurement raster path.", required: true },
                ToolParamSpec { name: "features", description: "Class/category raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.tif"));
        defaults.insert("features".to_string(), json!("classes.tif"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("data.tif"));
        example.insert("features".to_string(), json!("classes.tif"));
        example.insert("output".to_string(), json!("anova.html"));

        ToolManifest {
            id: "anova".to_string(),
            display_name: "ANOVA".to_string(),
            summary: "Performs one-way ANOVA on raster values grouped by class raster categories.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Measurement raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "features".to_string(), description: "Class/category raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_anova".to_string(),
                description: "Compare class means of a raster using one-way ANOVA.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_raster_path_arg(args, "features")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let feature_path = parse_raster_path_arg(args, "features")?;
        let html_output_path = parse_optional_html_report_path(args)?;

        let input = load_raster(&input_path, "input")?;
        let features = load_raster(&feature_path, "features")?;
        if input.rows != features.rows || input.cols != features.cols || input.bands != features.bands {
            return Err(ToolError::Validation(
                "input and features rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        let (class_stats_raw, overall_n, overall_sum, overall_sum_sqr) = (0..input.data.len())
            .into_par_iter()
            .fold(
                || (HashMap::<i64, (usize, f64, f64)>::new(), 0usize, 0.0f64, 0.0f64),
                |mut acc, i| {
                    let z = input.data.get_f64(i);
                    let cls = features.data.get_f64(i);
                    if input.is_nodata(z) || features.is_nodata(cls) {
                        return acc;
                    }

                    let class_id = cls.round() as i64;
                    let entry = acc.0.entry(class_id).or_insert((0usize, 0.0, 0.0));
                    entry.0 += 1;
                    entry.1 += z;
                    entry.2 += z * z;

                    acc.1 += 1;
                    acc.2 += z;
                    acc.3 += z * z;
                    acc
                },
            )
            .reduce(
                || (HashMap::<i64, (usize, f64, f64)>::new(), 0usize, 0.0f64, 0.0f64),
                |mut a, b| {
                    for (class_id, (n, sum, sum_sqr)) in b.0 {
                        let entry = a.0.entry(class_id).or_insert((0usize, 0.0, 0.0));
                        entry.0 += n;
                        entry.1 += sum;
                        entry.2 += sum_sqr;
                    }
                    a.1 += b.1;
                    a.2 += b.2;
                    a.3 += b.3;
                    a
                },
            );

        let class_stats: BTreeMap<i64, (usize, f64, f64)> =
            class_stats_raw.into_iter().collect();

        if overall_n < 2 {
            return Err(ToolError::Validation("insufficient valid cells for ANOVA".to_string()));
        }
        if class_stats.len() < 2 {
            return Err(ToolError::Validation("ANOVA requires at least two populated classes".to_string()));
        }

        let overall_mean = overall_sum / overall_n as f64;
        let overall_variance = (overall_sum_sqr - (overall_sum * overall_sum) / overall_n as f64)
            / (overall_n as f64 - 1.0);
        let ss_t = overall_sum_sqr - overall_n as f64 * overall_mean * overall_mean;

        let mut ss_b = 0.0f64;
        let mut ss_w = overall_sum_sqr;
        let mut groups_json = Vec::new();
        let mut group_rows = Vec::new();
        for (class_id, (n, sum, sum_sqr)) in &class_stats {
            let mean = *sum / *n as f64;
            let variance = if *n > 1 {
                (*sum_sqr - (*sum * *sum) / *n as f64) / (*n as f64 - 1.0)
            } else {
                0.0
            };

            ss_b += *n as f64 * (mean - overall_mean) * (mean - overall_mean);
            ss_w -= (*sum * *sum) / *n as f64;

            groups_json.push(json!({
                "class": class_id,
                "n": n,
                "mean": mean,
                "std_dev": variance.max(0.0).sqrt(),
            }));
            group_rows.push((*class_id, *n, mean, variance.max(0.0).sqrt()));
        }

        let num_classes = class_stats.len();
        let df_b = num_classes - 1;
        let df_w = overall_n - num_classes;
        if df_w == 0 {
            return Err(ToolError::Validation("ANOVA requires within-group degrees of freedom > 0".to_string()));
        }
        let df_t = overall_n - 1;
        let ms_b = ss_b / df_b as f64;
        let ms_w = ss_w / df_w as f64;
        let f_stat = ms_b / ms_w;
        let p_value = anova_f_call(anova_f_spin(f_stat, df_b, df_w));

        let report = json!({
            "input": input_path,
            "features": feature_path,
            "groups": groups_json,
            "overall": {
                "n": overall_n,
                "mean": overall_mean,
                "std_dev": overall_variance.max(0.0).sqrt(),
            },
            "anova": {
                "ss_between": ss_b,
                "ss_within": ss_w,
                "ss_total": ss_t,
                "df_between": df_b,
                "df_within": df_w,
                "df_total": df_t,
                "ms_between": ms_b,
                "ms_within": ms_w,
                "f_stat": f_stat,
                "p_value": p_value,
                "reject_equal_means_at_0_05": p_value < 0.05,
            }
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let mut group_summary = String::from(
                "<br><table align=\"center\"><caption>Group Summaries</caption><tr><th class=\"headerCell\">Group</th><th class=\"headerCell\">N</th><th class=\"headerCell\">Mean</th><th class=\"headerCell\">St. Dev.</th></tr>",
            );
            for (class_id, n, mean, std_dev) in &group_rows {
                group_summary.push_str(&format!(
                    "<tr><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.4}</td><td class=\"numberCell\">{:.4}</td></tr>",
                    class_id,
                    n,
                    mean,
                    std_dev
                ));
            }
            group_summary.push_str(&format!(
                "<tr><td class=\"numberCell\">Overall</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.4}</td><td class=\"numberCell\">{:.4}</td></tr></table>",
                overall_n,
                overall_mean,
                overall_variance.max(0.0).sqrt()
            ));

            let p_str = anova_p_string(p_value);
            let anova_table = format!(
                "<br><br><table align=\"center\"><caption>ANOVA Table</caption><tr><th class=\"headerCell\">Source of<br>Variation</th><th class=\"headerCell\">Sum of<br>Squares</th><th class=\"headerCell\">df</th><th class=\"headerCell\">Mean Square<br>Variance</th><th class=\"headerCell\">F</th><th class=\"headerCell\">p</th></tr><tr><td class=\"numberCell\">Between groups</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\">{}</td></tr><tr><td class=\"numberCell\">Within groups</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\"></td><td class=\"numberCell\"></td></tr><tr><td class=\"numberCell\">Total variation</td><td class=\"numberCell\">{:.3}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\"></td><td class=\"numberCell\"></td><td class=\"numberCell\"></td></tr></table>",
                ss_b,
                df_b,
                ms_b,
                f_stat,
                p_str,
                ss_w,
                df_w,
                ms_w,
                ss_t,
                df_t,
            );

            let interpretation = if p_value < 0.05 {
                format!(
                    "<br><br><h3>Interpretation:</h3><p>The null hypothesis states that the means of the measurement variable are the same for the different categories of data; the alternative hypothesis states that they are not all the same. The analysis showed that the category means were significantly heterogeneous (one-way anova, F<sub>&alpha;=0.05, df1={}, df2={}</sub>={:.3}, p{}), i.e. using an &alpha; of 0.05 the null hypothesis should be <strong>rejected</strong>.</p><p>Caveat: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant and statistical significance says nothing about the practical significance of a difference.</p>",
                    df_b,
                    df_w,
                    f_stat,
                    if p_value > 0.0001 { format!("={:.5}", p_value) } else { "< 0.0001".to_string() }
                )
            } else {
                format!(
                    "<br><br><h3>Interpretation:</h3><p>The null hypothesis states that the means of the measurement variable are the same for the different categories of data; the alternative hypothesis states that they are not all the same. The analysis showed that the category means were not significantly different (one-way anova, F<sub>&alpha;=0.05, df1={}, df2={}</sub>={:.3}, p={:.3}), i.e. using an &alpha; of 0.05 the null hypothesis should be <strong>accepted</strong>.</p><p>Caveat: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant and statistical significance says nothing about the practical significance of a difference.</p>",
                    df_b,
                    df_w,
                    f_stat,
                    p_value
                )
            };

            let assumptions = "<h3>Assumptions:</h3><p>The ANOVA test has important assumptions that must be satisfied in order for the associated p-value to be valid:</p><ol><li>The samples are independent.</li><li>Each sample is from a normally distributed population.</li><li>The population standard deviations of the groups are all equal. This property is known as homoscedasticity.</li></ol>";

            let body = format!(
                "<h1>One-way ANOVA test</h1><p><strong>Measurement variable:</strong> {}</p><p><strong>Nominal variable:</strong> {}</p>{}{}{}{}",
                input_path,
                feature_path,
                group_summary,
                anova_table,
                interpretation,
                assumptions
            );
            let html = html_document("ANOVA", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for PhiCoefficientTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "phi_coefficient",
            display_name: "Phi Coefficient",
            summary: "Performs binary classification agreement assessment using the phi coefficient.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First binary raster path.", required: true },
                ToolParamSpec { name: "input2", description: "Second binary raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path (alias: output_html_file).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("class_a.tif"));
        defaults.insert("input2".to_string(), json!("class_b.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("classification.tif"));
        example.insert("input2".to_string(), json!("reference.tif"));

        ToolManifest {
            id: "phi_coefficient".to_string(),
            display_name: "Phi Coefficient".to_string(),
            summary: "Performs binary classification agreement assessment using the phi coefficient.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First binary raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second binary raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_phi_coefficient".to_string(),
                description: "Compute binary agreement metrics and phi coefficient for two rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let html_output_path = parse_optional_html_report_path(args)?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        // Binary contingency table entries:
        // a=true positive, b=false positive, c=false negative, d=true negative.
        let (a, b, c, d) = (0..in1.data.len())
            .into_par_iter()
            .fold(
                || (0usize, 0usize, 0usize, 0usize),
                |mut acc, i| {
                    let z1 = in1.data.get_f64(i);
                    let z2 = in2.data.get_f64(i);
                    if in1.is_nodata(z1) || in2.is_nodata(z2) {
                        return acc;
                    }

                    let p = z1 != 0.0;
                    let q = z2 != 0.0;
                    match (p, q) {
                        (true, true) => acc.0 += 1,
                        (true, false) => acc.1 += 1,
                        (false, true) => acc.2 += 1,
                        (false, false) => acc.3 += 1,
                    }
                    acc
                },
            )
            .reduce(
                || (0usize, 0usize, 0usize, 0usize),
                |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3),
            );

        let n = a + b + c + d;
        if n == 0 {
            return Err(ToolError::Validation("no overlapping valid cells were found".to_string()));
        }

        let num = (a * d) as f64 - (b * c) as f64;
        let den = ((a + b) as f64 * (a + c) as f64 * (b + d) as f64 * (c + d) as f64).sqrt();
        let phi = if den > 0.0 { num / den } else { 0.0 };

        let overall_accuracy = (a + d) as f64 / n as f64;
        let precision = if (a + b) > 0 { a as f64 / (a + b) as f64 } else { f64::NAN };
        let recall = if (a + c) > 0 { a as f64 / (a + c) as f64 } else { f64::NAN };

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "contingency": {
                "a_true_positive": a,
                "b_false_positive": b,
                "c_false_negative": c,
                "d_true_negative": d,
                "n": n,
            },
            "phi_coefficient": phi,
            "overall_accuracy": overall_accuracy,
            "precision": precision,
            "recall": recall,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let n11 = a;
            let n10 = b;
            let n01 = c;
            let n00 = d;
            let n1dot = n11 + n10;
            let n0dot = n01 + n00;
            let ndot1 = n11 + n01;
            let ndot0 = n10 + n00;

            let body = format!(
                "<h1>Phi Coefficient Analysis</h1>\
                 <p><strong>Inputs</strong>:<br>Image 'x': {}<br>Image 'y': {}</p>\
                 <p>The input images have been interpreted as binary (Boolean) rasters, containing only 0's and 1's. All non-zero non-nodata values are considered to be valued 1. NoData values in either of the two input rasters are ignored during the analysis.</p>\
                 <br><h2>Confusion Matrix</h2>\
                 <table align=\"center\">\
                 <tr><th></th><th>y = 1</th><th>y = 0</th><th>Total</th></tr>\
                 <tr><th>x = 1</th><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>\
                 <tr><th>x = 0</th><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>\
                 <tr><th>Total</th><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td><td class=\"numberCell\">{}</td></tr>\
                 </table>\
                 <br><p><strong>phi</strong> = {:.4}</p>\
                 <p><strong>overall accuracy</strong> = {:.4}</p>\
                 <p><strong>precision</strong> = {:.4}</p>\
                 <p><strong>recall</strong> = {:.4}</p>\
                 <p>Note: The phi coefficient is a measure of association between two binary variables and is similar to the Pearson correlation coefficient in interpretation. It varies from -1.0 to 1.0.</p>",
                input1_path,
                input2_path,
                n11,
                n10,
                n1dot,
                n01,
                n00,
                n0dot,
                ndot1,
                ndot0,
                n,
                phi,
                overall_accuracy,
                precision,
                recall,
            );

            let html = html_document("Phi Coefficient Analysis", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageCorrelationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_correlation",
            display_name: "Image Correlation",
            summary: "Computes Pearson correlation matrix for two or more raster images.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamSpec {
                name: "inputs",
                description: "Input raster path list.",
                required: true,
            }],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["file1.tif", "file2.tif"]));
        let mut example = ToolArgs::new();
        example.insert(
            "inputs".to_string(),
            json!(["band1.tif", "band2.tif", "band3.tif"]),
        );

        ToolManifest {
            id: "image_correlation".to_string(),
            display_name: "Image Correlation".to_string(),
            summary: "Computes Pearson correlation matrix for two or more raster images.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![ToolParamDescriptor {
                name: "inputs".to_string(),
                description: "Input raster path list.".to_string(),
                required: true,
            }],
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_correlation".to_string(),
                description: "Compute pairwise image correlations for a raster set.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_input_list(args, "inputs")?;
        if inputs.len() < 2 {
            return Err(ToolError::Validation(
                "image_correlation requires at least two input rasters".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_input_list(args, "inputs")?;
        if input_paths.len() < 2 {
            return Err(ToolError::Validation(
                "image_correlation requires at least two input rasters".to_string(),
            ));
        }

        let mut rasters = Vec::<Arc<Raster>>::new();
        for p in &input_paths {
            rasters.push(load_raster_arc(p, "inputs")?);
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let bands = rasters[0].bands;
        for r in rasters.iter().skip(1) {
            if r.rows != rows || r.cols != cols || r.bands != bands {
                return Err(ToolError::Validation(
                    "all input images must have the same rows, columns, and bands".to_string(),
                ));
            }
        }

        let nfiles = rasters.len();
        let mut means = vec![0.0f64; nfiles];
        let mut valid_counts = vec![0usize; nfiles];
        for (k, r) in rasters.iter().enumerate() {
            let (s, n) = (0..r.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = r.data.get_f64(i);
                    if r.is_nodata(z) {
                        (0.0f64, 0usize)
                    } else {
                        (z, 1usize)
                    }
                })
                .reduce(
                    || (0.0f64, 0usize),
                    |a, b| (a.0 + b.0, a.1 + b.1),
                );
            if n == 0 {
                return Err(ToolError::Validation(format!(
                    "input raster '{}' contains no valid cells",
                    input_paths[k]
                )));
            }
            means[k] = s / n as f64;
            valid_counts[k] = n;
        }

        let mut matrix = vec![vec![f64::NAN; nfiles]; nfiles];
        let mut paired_n = vec![vec![0usize; nfiles]; nfiles];
        for a in 0..nfiles {
            matrix[a][a] = 1.0;
            paired_n[a][a] = valid_counts[a];
            for b in 0..a {
                let (dev_a, dev_b, dev_ab, n) = (0..rasters[a].data.len())
                    .into_par_iter()
                    .fold(
                        || (0.0f64, 0.0f64, 0.0f64, 0usize),
                        |mut acc, i| {
                            let z1 = rasters[a].data.get_f64(i);
                            let z2 = rasters[b].data.get_f64(i);
                            if rasters[a].is_nodata(z1) || rasters[b].is_nodata(z2) {
                                return acc;
                            }
                            let d1 = z1 - means[a];
                            let d2 = z2 - means[b];
                            acc.0 += d1 * d1;
                            acc.1 += d2 * d2;
                            acc.2 += d1 * d2;
                            acc.3 += 1;
                            acc
                        },
                    )
                    .reduce(
                        || (0.0f64, 0.0f64, 0.0f64, 0usize),
                        |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3),
                    );

                let corr = if n > 1 && dev_a > 0.0 && dev_b > 0.0 {
                    dev_ab / (dev_a * dev_b).sqrt()
                } else {
                    f64::NAN
                };
                matrix[a][b] = corr;
                matrix[b][a] = corr;
                paired_n[a][b] = n;
                paired_n[b][a] = n;
            }
        }

        let report = json!({
            "inputs": input_paths,
            "means": means,
            "valid_counts": valid_counts,
            "paired_counts": paired_n,
            "correlation_matrix": matrix,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageAutocorrelationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_autocorrelation",
            display_name: "Image Autocorrelation",
            summary: "Computes Moran's I for one or more raster images.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Input raster path list.",
                    required: true,
                },
                ToolParamSpec {
                    name: "contiguity",
                    description: "Neighbourhood rule: rook, king/queen, or bishop.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("inputs".to_string(), json!(["file1.tif", "file2.tif"]));
        defaults.insert("contiguity".to_string(), json!("rook"));
        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["file1.tif", "file2.tif"]));
        example.insert("contiguity".to_string(), json!("bishop"));

        ToolManifest {
            id: "image_autocorrelation".to_string(),
            display_name: "Image Autocorrelation".to_string(),
            summary: "Computes Moran's I for one or more raster images.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "inputs".to_string(),
                    description: "Input raster path list.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "contiguity".to_string(),
                    description: "Neighbourhood rule: rook, king/queen, or bishop.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_autocorrelation".to_string(),
                description: "Compute Moran's I for multiple rasters under a contiguity rule.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_input_list(args, "inputs")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_input_list(args, "inputs")?;
        let contiguity = args
            .get("contiguity")
            .and_then(|v| v.as_str())
            .unwrap_or("rook")
            .to_ascii_lowercase();

        let (dx, dy): (Vec<isize>, Vec<isize>) = if contiguity.contains("bishop") {
            (vec![1, 1, -1, -1], vec![-1, 1, 1, -1])
        } else if contiguity.contains("queen") || contiguity.contains("king") {
            (
                vec![1, 1, 1, 0, -1, -1, -1, 0],
                vec![-1, 0, 1, 1, 1, 0, -1, -1],
            )
        } else {
            (vec![1, 0, -1, 0], vec![0, 1, 0, -1])
        };

        let mut rasters = Vec::<Arc<Raster>>::new();
        for p in &input_paths {
            rasters.push(load_raster_arc(p, "inputs")?);
        }

        if rasters.is_empty() {
            return Err(ToolError::Validation("no input rasters provided".to_string()));
        }

        let rows = rasters[0].rows;
        let cols = rasters[0].cols;
        let bands = rasters[0].bands;
        for r in rasters.iter().skip(1) {
            if r.rows != rows || r.cols != cols || r.bands != bands {
                return Err(ToolError::Validation(
                    "all input images must have the same rows, columns, and bands".to_string(),
                ));
            }
        }

        let mut per_image = Vec::<serde_json::Value>::new();
        for (idx, r) in rasters.iter().enumerate() {
            let (sum, n_count) = (0..r.data.len())
                .into_par_iter()
                .map(|i| {
                    let z = r.data.get_f64(i);
                    if r.is_nodata(z) {
                        (0.0f64, 0usize)
                    } else {
                        (z, 1usize)
                    }
                })
                .reduce(
                    || (0.0f64, 0usize),
                    |a, b| (a.0 + b.0, a.1 + b.1),
                );
            let n = n_count as f64;

            if n <= 3.0 {
                per_image.push(json!({
                    "input": input_paths[idx],
                    "valid_count": n,
                    "error": "insufficient valid cells for autocorrelation",
                }));
                continue;
            }

            let mean = sum / n;
            let (total_deviation, w, numerator, s2, k) = (0..rows)
                .into_par_iter()
                .map(|row| {
                    let row_i = row as isize;
                    let mut total_deviation_local = 0.0f64;
                    let mut w_local = 0.0f64;
                    let mut numerator_local = 0.0f64;
                    let mut s2_local = 0.0f64;
                    let mut k_local = 0.0f64;

                    for col in 0..cols as isize {
                        let z = r.get_raw(0, row_i, col).unwrap_or(r.nodata);
                        if r.is_nodata(z) {
                            continue;
                        }

                        let dz = z - mean;
                        total_deviation_local += dz * dz;
                        k_local += dz * dz * dz * dz;

                        let mut wij = 0.0;
                        for nidx in 0..dx.len() {
                            let x = col + dx[nidx];
                            let y = row_i + dy[nidx];
                            if x < 0 || x >= cols as isize || y < 0 || y >= rows as isize {
                                continue;
                            }
                            let zn = r.get_raw(0, y, x).unwrap_or(r.nodata);
                            if r.is_nodata(zn) {
                                continue;
                            }
                            w_local += 1.0;
                            numerator_local += dz * (zn - mean);
                            wij += 1.0;
                        }
                        s2_local += wij * wij;
                    }

                    (
                        total_deviation_local,
                        w_local,
                        numerator_local,
                        s2_local,
                        k_local,
                    )
                })
                .reduce(
                    || (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64),
                    |a, b| {
                        (
                            a.0 + b.0,
                            a.1 + b.1,
                            a.2 + b.2,
                            a.3 + b.3,
                            a.4 + b.4,
                        )
                    },
                );

            if w <= 0.0 || total_deviation <= 0.0 {
                per_image.push(json!({
                    "input": input_paths[idx],
                    "valid_count": n,
                    "error": "insufficient neighborhood support for autocorrelation",
                }));
                continue;
            }

            let s1 = 4.0 * w;
            let s2 = 4.0 * s2;
            let std_dev = (total_deviation / (n - 1.0)).sqrt();
            let morans_i = n * numerator / (total_deviation * w);
            let expected_i = -1.0 / (n - 1.0);

            let var_normality =
                (n * n * s1 - n * s2 + 3.0 * w * w) / ((w * w) * (n * n - 1.0));
            let z_n = if var_normality > 0.0 {
                (morans_i - expected_i) / var_normality.sqrt()
            } else {
                0.0
            };
            let p_n = two_tailed_normal_p(z_n);

            let k = if std_dev > 0.0 {
                k / (n * std_dev * std_dev * std_dev * std_dev)
            } else {
                0.0
            };

            let var_randomization = (n
                * ((n * n - 3.0 * n + 3.0) * s1 - n * s2 + 3.0 * w * w)
                - k * (n * n - n) * s1
                - 2.0 * n * s1
                + 6.0 * w * w)
                / ((n - 1.0) * (n - 2.0) * (n - 3.0) * w * w);

            let z_r = if var_randomization > 0.0 {
                (morans_i - expected_i) / var_randomization.sqrt()
            } else {
                0.0
            };
            let p_r = two_tailed_normal_p(z_r);

            per_image.push(json!({
                "input": input_paths[idx],
                "valid_count": n,
                "mean": mean,
                "std_dev": std_dev,
                "morans_i": morans_i,
                "expected_i": expected_i,
                "weights_sum": w,
                "variance_normality": var_normality,
                "variance_randomization": var_randomization,
                "z_normality": z_n,
                "z_randomization": z_r,
                "p_value_normality": p_n,
                "p_value_randomization": p_r,
            }));
        }

        let report = json!({
            "contiguity": contiguity,
            "results": per_image,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageCorrelationNeighbourhoodAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_correlation_neighbourhood_analysis",
            display_name: "Image Correlation Neighbourhood Analysis",
            summary: "Performs moving-window correlation analysis between two rasters and returns correlation and p-value rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "First input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Second input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "filter_size",
                    description: "Moving window size in cells (minimum 3, default 11).",
                    required: false,
                },
                ToolParamSpec {
                    name: "correlation_stat",
                    description: "Correlation metric: pearson, spearman, or kendall (default pearson).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output1",
                    description: "Optional output path for correlation raster.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output2",
                    description: "Optional output path for significance (p-value) raster.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("image1.tif"));
        defaults.insert("input2".to_string(), json!("image2.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("correlation_stat".to_string(), json!("pearson"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("band1.tif"));
        example.insert("input2".to_string(), json!("band2.tif"));
        example.insert("filter_size".to_string(), json!(11));
        example.insert("correlation_stat".to_string(), json!("spearman"));
        example.insert("output1".to_string(), json!("local_corr.tif"));
        example.insert("output2".to_string(), json!("local_p.tif"));

        ToolManifest {
            id: "image_correlation_neighbourhood_analysis".to_string(),
            display_name: "Image Correlation Neighbourhood Analysis".to_string(),
            summary: "Performs moving-window correlation analysis between two rasters and returns correlation and p-value rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input1".to_string(),
                    description: "First input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "input2".to_string(),
                    description: "Second input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "filter_size".to_string(),
                    description: "Moving window size in cells (minimum 3, default 11).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "correlation_stat".to_string(),
                    description: "Correlation metric: pearson, spearman, or kendall (default pearson).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output1".to_string(),
                    description: "Optional output path for correlation raster.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output2".to_string(),
                    description: "Optional output path for significance (p-value) raster.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_correlation_neighbourhood_analysis".to_string(),
                description: "Compute local correlation and significance rasters between two images.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_output_path(args, "output1")?;
        let _ = parse_optional_output_path(args, "output2")?;

        let filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);
        if filter_size < 3 {
            return Err(ToolError::Validation(
                "filter_size must be at least 3".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let output1_path = parse_optional_output_path(args, "output1")?;
        let output2_path = parse_optional_output_path(args, "output2")?;

        let mut filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);
        filter_size = filter_size.max(3);

        let correlation_stat = args
            .get("correlation_stat")
            .and_then(|v| v.as_str())
            .unwrap_or("pearson")
            .to_ascii_lowercase();

        let stat = if correlation_stat.contains("ken") {
            "kendall"
        } else if correlation_stat.contains("spear") {
            "spearman"
        } else {
            "pearson"
        };

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        let rows = in1.rows;
        let cols = in1.cols;
        let bands = in1.bands;
        let rows_i = rows as isize;
        let cols_i = cols as isize;

        let mut out_corr = Raster::new(RasterConfig {
            rows,
            cols,
            bands,
            x_min: in1.x_min,
            y_min: in1.y_min,
            cell_size: in1.cell_size_x,
            cell_size_y: Some(in1.cell_size_y),
            nodata: in1.nodata,
            data_type: DataType::F32,
            crs: in1.crs.clone(),
            metadata: in1.metadata.clone(),
        });
        let mut out_sig = Raster::new(RasterConfig {
            rows,
            cols,
            bands,
            x_min: in1.x_min,
            y_min: in1.y_min,
            cell_size: in1.cell_size_x,
            cell_size_y: Some(in1.cell_size_y),
            nodata: in1.nodata,
            data_type: DataType::F32,
            crs: in1.crs.clone(),
            metadata: in1.metadata.clone(),
        });

        let half = (filter_size as isize) / 2;
        let mut offsets = Vec::<(isize, isize)>::with_capacity(filter_size * filter_size);
        for r in 0..filter_size as isize {
            for c in 0..filter_size as isize {
                offsets.push((r - half, c - half));
            }
        }

        let total_cells = bands * rows * cols;
        let (corr_values, sig_values): (Vec<f64>, Vec<f64>) = (0..total_cells)
            .into_par_iter()
            .map(|idx| {
                let band_idx = idx / (rows * cols);
                let band = band_idx as isize;
                let rem = idx % (rows * cols);
                let row = (rem / cols) as isize;
                let col = (rem % cols) as isize;

                let z1 = in1.get_raw(band, row, col).unwrap_or(in1.nodata);
                let z2 = in2.get_raw(band, row, col).unwrap_or(in2.nodata);
                if in1.is_nodata(z1) || in2.is_nodata(z2) {
                    return (in1.nodata, in1.nodata);
                }

                let mut a = Vec::<f64>::with_capacity(offsets.len());
                let mut b = Vec::<f64>::with_capacity(offsets.len());
                for (dr, dc) in &offsets {
                    let rr = row + *dr;
                    let cc = col + *dc;
                    if rr < 0 || rr >= rows_i || cc < 0 || cc >= cols_i {
                        continue;
                    }
                    let v1 = in1.get_raw(band, rr, cc).unwrap_or(in1.nodata);
                    let v2 = in2.get_raw(band, rr, cc).unwrap_or(in2.nodata);
                    if in1.is_nodata(v1) || in2.is_nodata(v2) {
                        continue;
                    }
                    a.push(v1);
                    b.push(v2);
                }

                if a.len() < 3 {
                    return (in1.nodata, in1.nodata);
                }

                if stat == "kendall" {
                    if let Some((tau, n)) = kendall_tau_b_from_pairs(&a, &b) {
                        let nn = n as f64;
                        let z = if nn > 2.0 {
                            3.0 * tau * (nn * (nn - 1.0) / (2.0 * (2.0 * nn + 5.0))).sqrt()
                        } else {
                            0.0
                        };
                        (tau, two_tailed_normal_p(z))
                    } else {
                        (in1.nodata, in1.nodata)
                    }
                } else if stat == "spearman" {
                    if let Some((rho, n, ties)) = spearman_from_pairs(&a, &b) {
                        let df = n as f64 - 2.0;
                        let t = if df > 0.0 && (1.0 - rho * rho) > 0.0 {
                            rho * (df / (1.0 - rho * rho)).sqrt()
                        } else {
                            0.0
                        };
                        let p = two_tailed_normal_p(t);
                        if ties > 0 {
                            (rho, p.max(0.0))
                        } else {
                            (rho, p)
                        }
                    } else {
                        (in1.nodata, in1.nodata)
                    }
                } else if let Some((r, n)) = pearson_from_pairs(&a, &b) {
                    let df = n as f64 - 2.0;
                    let t = if df > 0.0 && (1.0 - r * r) > 0.0 {
                        r * (df / (1.0 - r * r)).sqrt()
                    } else {
                        0.0
                    };
                    (r, two_tailed_normal_p(t))
                } else {
                    (in1.nodata, in1.nodata)
                }
            })
            .unzip();

        if let (Some(corr_slice), Some(sig_slice)) = (out_corr.data.as_f32_slice_mut(), out_sig.data.as_f32_slice_mut()) {
            corr_slice
                .par_iter_mut()
                .zip(sig_slice.par_iter_mut())
                .enumerate()
                .for_each(|(i, (corr_cell, sig_cell))| {
                    *corr_cell = corr_values[i] as f32;
                    *sig_cell = sig_values[i] as f32;
                });
        } else {
            for i in 0..total_cells {
                out_corr.data.set_f64(i, corr_values[i]);
                out_sig.data.set_f64(i, sig_values[i]);
            }
        }

        let output1_locator = write_or_store_output(out_corr, output1_path)?;
        let output2_locator = write_or_store_output(out_sig, output2_path)?;

        let mut outputs = BTreeMap::new();
        outputs.insert("output1".to_string(), typed_raster_output(output1_locator));
        outputs.insert("output2".to_string(), typed_raster_output(output2_locator));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ImageRegressionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "image_regression",
            display_name: "Image Regression",
            summary: "Performs bivariate linear regression between two rasters and outputs a residual raster and report.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "Independent-variable raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Dependent-variable raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "standardize_residuals",
                    description: "Whether to standardize residuals by model standard error.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path for residual raster.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("independent.tif"));
        defaults.insert("input2".to_string(), json!("dependent.tif"));
        defaults.insert("standardize_residuals".to_string(), json!(false));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("elevation.tif"));
        example.insert("input2".to_string(), json!("soil_moisture.tif"));
        example.insert("standardize_residuals".to_string(), json!(true));
        example.insert("output".to_string(), json!("image_regression_residuals.tif"));

        ToolManifest {
            id: "image_regression".to_string(),
            display_name: "Image Regression".to_string(),
            summary: "Performs bivariate linear regression between two rasters and outputs a residual raster and report.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input1".to_string(),
                    description: "Independent-variable raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "input2".to_string(),
                    description: "Dependent-variable raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "standardize_residuals".to_string(),
                    description: "Whether to standardize residuals by model standard error.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path for residual raster.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_image_regression".to_string(),
                description: "Run bivariate regression and create a residual raster.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let standardize_residuals = args
            .get("standardize_residuals")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        let (n, sum_x, sum_y, sum_xy, sum_xx, sum_yy) = (0..in1.data.len())
            .into_par_iter()
            .fold(
                || (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64),
                |mut acc, i| {
                    let x = in1.data.get_f64(i);
                    let y = in2.data.get_f64(i);
                    if in1.is_nodata(x) || in2.is_nodata(y) {
                        return acc;
                    }
                    acc.0 += 1.0;
                    acc.1 += x;
                    acc.2 += y;
                    acc.3 += x * y;
                    acc.4 += x * x;
                    acc.5 += y * y;
                    acc
                },
            )
            .reduce(
                || (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64),
                |a, b| {
                    (
                        a.0 + b.0,
                        a.1 + b.1,
                        a.2 + b.2,
                        a.3 + b.3,
                        a.4 + b.4,
                        a.5 + b.5,
                    )
                },
            );

        if n <= 2.0 {
            return Err(ToolError::Validation(
                "insufficient paired valid cells for regression".to_string(),
            ));
        }

        let denom_x = n * sum_xx - sum_x * sum_x;
        if denom_x.abs() <= 1.0e-12 {
            return Err(ToolError::Validation(
                "independent variable has near-zero variance".to_string(),
            ));
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom_x;
        let intercept = (sum_y - slope * sum_x) / n;

        let denom_r = ((n * sum_xx - sum_x * sum_x) * (n * sum_yy - sum_y * sum_y)).sqrt();
        let r = if denom_r > 0.0 {
            (n * sum_xy - sum_x * sum_y) / denom_r
        } else {
            0.0
        };
        let r_sqr = r * r;
        let y_mean = sum_y / n;

        let (ss_error, ss_total) = (0..in1.data.len())
            .into_par_iter()
            .fold(
                || (0.0f64, 0.0f64),
                |mut acc, i| {
                    let x = in1.data.get_f64(i);
                    let y = in2.data.get_f64(i);
                    if in1.is_nodata(x) || in2.is_nodata(y) {
                        return acc;
                    }
                    let yhat = slope * x + intercept;
                    acc.0 += (y - yhat) * (y - yhat);
                    acc.1 += (y - y_mean) * (y - y_mean);
                    acc
                },
            )
            .reduce(
                || (0.0f64, 0.0f64),
                |a, b| (a.0 + b.0, a.1 + b.1),
            );

        let df_reg = 1.0f64;
        let df_error = n - 2.0;
        let ss_reg = (ss_total - ss_error).max(0.0);
        let ms_reg = ss_reg / df_reg;
        let ms_error = if df_error > 0.0 {
            ss_error / df_error
        } else {
            0.0
        };
        let f_stat = if ms_error > 0.0 { ms_reg / ms_error } else { 0.0 };
        let f_pvalue = if df_error >= 1.0 {
            anova_f_spin(f_stat.max(0.0), 1, df_error as usize).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let se_of_estimate = ms_error.sqrt();

        let x_mean = sum_x / n;
        let msse = (sum_yy - (sum_xy * sum_xy) / sum_xx).max(0.0) / (n - 2.0);
        let intercept_se = (msse * ((1.0 / n) + (x_mean * x_mean) / sum_xx)).sqrt();
        let slope_se = (msse / sum_xx).sqrt();
        let intercept_t = if intercept_se > 0.0 { intercept / intercept_se } else { 0.0 };
        let slope_t = if slope_se > 0.0 { slope / slope_se } else { 0.0 };
        let intercept_pvalue = two_tailed_normal_p(intercept_t);
        let slope_pvalue = two_tailed_normal_p(slope_t);

        let mut residuals = Raster::new(RasterConfig {
            rows: in1.rows,
            cols: in1.cols,
            bands: in1.bands,
            x_min: in1.x_min,
            y_min: in1.y_min,
            cell_size: in1.cell_size_x,
            cell_size_y: Some(in1.cell_size_y),
            nodata: in1.nodata,
            data_type: DataType::F32,
            crs: in1.crs.clone(),
            metadata: in1.metadata.clone(),
        });

        let residual_values: Vec<f64> = (0..residuals.data.len())
            .into_par_iter()
            .map(|i| {
                let x = in1.data.get_f64(i);
                let y = in2.data.get_f64(i);
                if in1.is_nodata(x) || in2.is_nodata(y) {
                    return in1.nodata;
                }
                let yhat = slope * x + intercept;
                let mut res = y - yhat;
                if standardize_residuals && se_of_estimate > 0.0 {
                    res /= se_of_estimate;
                }
                res
            })
            .collect();

        if let Some(data_slice) = residuals.data.as_f32_slice_mut() {
            data_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cell)| *cell = residual_values[i] as f32);
        } else {
            for (i, v) in residual_values.iter().enumerate() {
                residuals.data.set_f64(i, *v);
            }
        }

        let out_loc = write_or_store_output(residuals, output_path)?;
        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "paired_count": n,
            "model": {
                "r": r,
                "r_squared": r_sqr,
                "slope": slope,
                "intercept": intercept,
                "std_error_of_estimate": se_of_estimate,
                "equation": format!("Y = {:.12} * X + {:.12}", slope, intercept),
            },
            "anova": {
                "ss_regression": ss_reg,
                "ss_error": ss_error,
                "ss_total": ss_total,
                "df_regression": df_reg,
                "df_error": df_error,
                "ms_regression": ms_reg,
                "ms_error": ms_error,
                "f": f_stat,
                "p": f_pvalue,
            },
            "coefficients": {
                "constant": {
                    "b": intercept,
                    "std_error": intercept_se,
                    "t": intercept_t,
                    "p": intercept_pvalue,
                },
                "slope": {
                    "b": slope,
                    "std_error": slope_se,
                    "t": slope_t,
                    "p": slope_pvalue,
                }
            },
            "standardize_residuals": standardize_residuals,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(out_loc));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for DbscanTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "dbscan",
            display_name: "DBSCAN Clustering",
            summary: "Performs unsupervised DBSCAN density-based clustering on a stack of input rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "inputs",
                    description: "Comma/semicolon-delimited list or JSON array of input raster paths (feature bands).",
                    required: true,
                },
                ToolParamSpec {
                    name: "scaling_method",
                    description: "Feature scaling: 'none' (default), 'normalize' (0-1 range), or 'standardize' (z-scores).",
                    required: false,
                },
                ToolParamSpec {
                    name: "search_distance",
                    description: "Epsilon: neighbourhood search radius in feature space (default 1.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_points",
                    description: "Minimum number of neighbours within epsilon for a core point (default 5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster path for cluster-ID labels.",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("scaling_method".to_string(), json!("none"));
        defaults.insert("search_distance".to_string(), json!(1.0));
        defaults.insert("min_points".to_string(), json!(5));

        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["band1.tif", "band2.tif", "band3.tif"]));
        example.insert("scaling_method".to_string(), json!("normalize"));
        example.insert("search_distance".to_string(), json!(0.1));
        example.insert("min_points".to_string(), json!(10));
        example.insert("output".to_string(), json!("dbscan_clusters.tif"));

        ToolManifest {
            id: "dbscan".to_string(),
            display_name: "DBSCAN Clustering".to_string(),
            summary: "Performs unsupervised DBSCAN density-based clustering on a stack of input rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "inputs".to_string(),
                    description: "Comma/semicolon-delimited list or JSON array of input raster paths (feature bands).".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "scaling_method".to_string(),
                    description: "Feature scaling: 'none' (default), 'normalize' (0-1 range), or 'standardize' (z-scores).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "search_distance".to_string(),
                    description: "Epsilon: neighbourhood search radius in feature space (default 1.0).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "min_points".to_string(),
                    description: "Minimum number of neighbours within epsilon for a core point (default 5).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster path for cluster-ID labels.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_dbscan".to_string(),
                description: "Cluster a three-band raster stack using DBSCAN with normalisation.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "clustering".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let inputs = parse_raster_list_arg(args, "inputs")?;
        if inputs.is_empty() {
            return Err(ToolError::Validation("parameter 'inputs' must contain at least one raster path".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let scaling_method = args
            .get("scaling_method")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_lowercase();
        let normalize = scaling_method.contains("nor");
        let standardize = !normalize && scaling_method.contains("stan");
        let eps = args
            .get("search_distance")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .max(0.0);
        let min_points = args
            .get("min_points")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5)
            .max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        // Load all input rasters
        let rasters: Vec<Raster> = input_paths
            .iter()
            .enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;

        let num_features = rasters.len();
        let rows = rasters[0].rows;
        let cols = rasters[0].cols;

        for (i, r) in rasters.iter().enumerate() {
            if r.rows != rows || r.cols != cols {
                return Err(ToolError::Validation(format!(
                    "raster at inputs[{}] has different dimensions ({} x {}) compared to inputs[0] ({} x {})",
                    i, r.rows, r.cols, rows, cols
                )));
            }
        }

        // Compute per-band scaling offsets and multipliers
        let band_scale: Vec<(f64, f64)> = rasters
            .par_iter()
            .map(|r| {
                if normalize {
                    let stats = r.statistics();
                    let rng = stats.max - stats.min;
                    (stats.min, if rng.abs() < 1.0e-15 { 1.0 } else { rng })
                } else if standardize {
                    let (n, sum, sum_sq) = (0..r.data.len())
                        .into_par_iter()
                        .fold(
                            || (0.0f64, 0.0f64, 0.0f64),
                            |(mut local_n, mut local_sum, mut local_sum_sq), i| {
                                let z = r.data.get_f64(i);
                                if !r.is_nodata(z) {
                                    local_n += 1.0;
                                    local_sum += z;
                                    local_sum_sq += z * z;
                                }
                                (local_n, local_sum, local_sum_sq)
                            },
                        )
                        .reduce(
                            || (0.0f64, 0.0f64, 0.0f64),
                            |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
                        );
                    if n < 2.0 {
                        (0.0, 1.0)
                    } else {
                        let mean = sum / n;
                        let var = (sum_sq / n - mean * mean).max(0.0);
                        let stdev = var.sqrt();
                        (mean, if stdev < 1.0e-15 { 1.0 } else { stdev })
                    }
                } else {
                    (0.0, 1.0)
                }
            })
            .collect();

        // Collect valid pixels as feature vectors
        let n_total = rows * cols;
        let row_features: Vec<Vec<(usize, Vec<f64>)>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let row_i = row as isize;
                let mut local = Vec::<(usize, Vec<f64>)>::new();
                for col in 0..cols as isize {
                    let flat_idx = row * cols + col as usize;
                    let mut feat = vec![0.0f64; num_features];
                    let mut is_nodata = false;
                    for (b, r) in rasters.iter().enumerate() {
                        let z = r.get(0, row_i, col);
                        if r.is_nodata(z) {
                            is_nodata = true;
                            break;
                        }
                        feat[b] = (z - band_scale[b].0) / band_scale[b].1;
                    }
                    if !is_nodata {
                        local.push((flat_idx, feat));
                    }
                }
                local
            })
            .collect();

        let mut points: Vec<Vec<f64>> = Vec::new();
        let mut pixel_map: Vec<usize> = Vec::new(); // point index -> flat pixel index
        for local in row_features {
            for (flat_idx, feat) in local {
                pixel_map.push(flat_idx);
                points.push(feat);
            }
        }

        let num_valid = points.len();

        // Run DBSCAN in feature space
        // labels: -1 = unvisited, 0 = noise, >=1 = cluster id (1-based)
        let mut labels: Vec<i32> = vec![-1i32; num_valid];

        if num_valid > 0 {
            let eps_sq = eps * eps;
            let mut tree: KdTree<f64, usize, Vec<f64>> = KdTree::new(num_features);
            for (i, pt) in points.iter().enumerate() {
                tree.add(pt.clone(), i)
                    .map_err(|e| ToolError::Execution(format!("kdtree insert failed: {e}")))?;
            }

            let mut cluster_id: i32 = 0;
            for i in 0..num_valid {
                if labels[i] != -1 {
                    continue;
                }
                let neighbors = tree
                    .within(&points[i], eps_sq, &squared_euclidean)
                    .map_err(|e| ToolError::Execution(format!("kdtree range query failed: {e}")))?;
                if neighbors.len() < min_points {
                    labels[i] = 0; // noise
                    continue;
                }
                cluster_id += 1;
                labels[i] = cluster_id;
                let mut seed_set: Vec<usize> =
                    neighbors.into_iter().map(|(_, &j)| j).filter(|&j| j != i).collect();
                let mut si = 0;
                while si < seed_set.len() {
                    let q = seed_set[si];
                    si += 1;
                    if labels[q] == 0 {
                        // noise reclaimed as border point
                        labels[q] = cluster_id;
                        continue;
                    }
                    if labels[q] != -1 {
                        continue; // already part of a cluster
                    }
                    labels[q] = cluster_id;
                    let q_neighbors = tree
                        .within(&points[q], eps_sq, &squared_euclidean)
                        .map_err(|e| ToolError::Execution(format!("kdtree range query failed: {e}")))?;
                    if q_neighbors.len() >= min_points {
                        for (_, &r) in &q_neighbors {
                            if labels[r] == -1 || labels[r] == 0 {
                                seed_set.push(r);
                            }
                        }
                    }
                }
            }
        }

        // Build output raster: I16, nodata = -32768, clusters are 0-based
        const OUT_NODATA: f64 = -32768.0;
        let mut output = Raster::new(RasterConfig {
            rows,
            cols,
            bands: 1,
            x_min: rasters[0].x_min,
            y_min: rasters[0].y_min,
            cell_size: rasters[0].cell_size_x,
            cell_size_y: Some(rasters[0].cell_size_y),
            nodata: OUT_NODATA,
            data_type: DataType::I16,
            crs: rasters[0].crs.clone(),
            metadata: rasters[0].metadata.clone(),
        });

        if let Some(data_slice) = output.data.as_i16_slice_mut() {
            data_slice.par_iter_mut().for_each(|cell| *cell = OUT_NODATA as i16);
            for (pt_idx, &flat_idx) in pixel_map.iter().enumerate() {
                let lbl = labels[pt_idx];
                if lbl > 0 {
                    data_slice[flat_idx] = (lbl - 1) as i16; // 0-based cluster IDs
                }
                // lbl == 0 (noise) -> keep nodata
            }
        } else {
            for i in 0..n_total {
                output.data.set_f64(i, OUT_NODATA);
            }
            for (pt_idx, &flat_idx) in pixel_map.iter().enumerate() {
                let lbl = labels[pt_idx];
                if lbl > 0 {
                    output.data.set_f64(flat_idx, (lbl - 1) as f64); // 0-based cluster IDs
                }
                // lbl == 0 (noise) → keep nodata
            }
        }

        let num_clusters = labels.iter().copied().filter(|&l| l > 0).max().unwrap_or(0) as usize;
        let noise_count = labels.iter().copied().filter(|&l| l == 0).count();

        let report = json!({
            "inputs": input_paths,
            "scaling_method": scaling_method,
            "search_distance": eps,
            "min_points": min_points,
            "num_clusters": num_clusters,
            "num_noise_cells": noise_count,
            "num_valid_cells": num_valid,
        })
        .to_string();

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ConditionalEvaluationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "conditional_evaluation",
            display_name: "Conditional Evaluation",
            summary: "Performs if-then-else conditional evaluation on raster cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "statement", description: "Conditional expression evaluated per cell.", required: true },
                ToolParamSpec { name: "true", description: "Value or raster/expression used when condition is true.", required: false },
                ToolParamSpec { name: "false", description: "Value or raster/expression used when condition is false.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("statement".to_string(), json!("value > 35.0"));
        defaults.insert("true".to_string(), json!(1.0));
        defaults.insert("false".to_string(), json!(0.0));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("statement".to_string(), json!("value > 2500.0"));
        example.insert("true".to_string(), json!(2500.0));
        example.insert("false".to_string(), json!("dem.tif"));
        example.insert("output".to_string(), json!("conditional.tif"));

        ToolManifest {
            id: "conditional_evaluation".to_string(),
            display_name: "Conditional Evaluation".to_string(),
            summary: "Performs if-then-else conditional evaluation on raster cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "statement".to_string(), description: "Conditional expression evaluated per cell.".to_string(), required: true },
                ToolParamDescriptor { name: "true".to_string(), description: "Value or raster/expression used when condition is true.".to_string(), required: false },
                ToolParamDescriptor { name: "false".to_string(), description: "Value or raster/expression used when condition is false.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_conditional_evaluation".to_string(),
                description: "Assign values based on a per-cell condition.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "conditional".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let statement = args
            .get("statement")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .ok_or_else(|| ToolError::Validation("parameter 'statement' is required".to_string()))?;
        if statement.is_empty() {
            return Err(ToolError::Validation("statement must be non-empty".to_string()));
        }
        let normalized = normalize_conditional_expression(statement);
        build_operator_tree::<DefaultNumericTypes>(&normalized)
            .map_err(|e| ToolError::Validation(format!("invalid statement expression: {e}")))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let statement = args
            .get("statement")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .ok_or_else(|| ToolError::Validation("parameter 'statement' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;

        let input = load_raster(&input_path, "input")?;
        let mut output = input.clone();
        output.data_type = DataType::F64;

        let stats = input.statistics();
        let west = input.x_min;
        let south = input.y_min;
        let east = input.x_min + input.cols as f64 * input.cell_size_x;
        let north = input.y_min + input.rows as f64 * input.cell_size_y;

        let normalized_statement = normalize_conditional_expression(statement);
        let statement_contains_nodata = normalized_statement.contains("nodata");
        let condition_tree = build_operator_tree::<DefaultNumericTypes>(&normalized_statement)
            .map_err(|e| ToolError::Validation(format!("invalid statement expression: {e}")))?;

        let true_source = parse_conditional_value_source(args, "true", &input)?;
        let false_source = parse_conditional_value_source(args, "false", &input)?;

        let mut context = HashMapContext::new();
        let rows = input.rows as f64;
        let columns = input.cols as f64;
        let cell_size = 0.5 * (input.cell_size_x + input.cell_size_y);
        let _ = context.set_value("rows".to_string(), EvalValue::Float(rows));
        let _ = context.set_value("columns".to_string(), EvalValue::Float(columns));
        let _ = context.set_value("north".to_string(), EvalValue::Float(north));
        let _ = context.set_value("south".to_string(), EvalValue::Float(south));
        let _ = context.set_value("east".to_string(), EvalValue::Float(east));
        let _ = context.set_value("west".to_string(), EvalValue::Float(west));
        let _ = context.set_value("cellsizex".to_string(), EvalValue::Float(input.cell_size_x));
        let _ = context.set_value("cellsizey".to_string(), EvalValue::Float(input.cell_size_y));
        let _ = context.set_value("cellsize".to_string(), EvalValue::Float(cell_size));
        let _ = context.set_value("minvalue".to_string(), EvalValue::Float(stats.min));
        let _ = context.set_value("maxvalue".to_string(), EvalValue::Float(stats.max));
        let _ = context.set_value("nodata".to_string(), EvalValue::Float(input.nodata));
        let _ = context.set_value("null".to_string(), EvalValue::Float(input.nodata));
        let _ = context.set_value("pi".to_string(), EvalValue::Float(std::f64::consts::PI));
        let _ = context.set_value("e".to_string(), EvalValue::Float(std::f64::consts::E));

        for row in 0..input.rows {
            let row_f = row as f64;
            let rowy = input.row_center_y(row as isize);
            let _ = context.set_value("row".to_string(), EvalValue::Float(row_f));
            let _ = context.set_value("rowy".to_string(), EvalValue::Float(rowy));
            for col in 0..input.cols {
                let idx = row * input.cols + col;
                let col_f = col as f64;
                let columnx = input.col_center_x(col as isize);
                let _ = context.set_value("column".to_string(), EvalValue::Float(col_f));
                let _ = context.set_value("columnx".to_string(), EvalValue::Float(columnx));

                let value = input.data.get_f64(idx);
                let _ = context.set_value("value".to_string(), EvalValue::Float(value));

                if input.is_nodata(value) && !statement_contains_nodata {
                    output.data.set_f64(idx, output.nodata);
                    continue;
                }

                let condition_val = condition_tree
                    .eval_with_context(&context)
                    .map_err(|e| ToolError::Execution(format!(
                        "statement evaluation failed at row {}, col {}: {}",
                        row, col, e
                    )))?;
                let condition = eval_value_to_bool(condition_val)?;

                let out_val = if condition {
                    resolve_conditional_value(&true_source, idx, &context)?
                } else {
                    resolve_conditional_value(&false_source, idx, &context)?
                };
                output.data.set_f64(idx, out_val);
            }
        }

        let locator = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(locator));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for KappaIndexTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "kappa_index",
            display_name: "Kappa Index",
            summary: "Computes Cohen's kappa and agreement metrics between two categorical rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "Input classification raster path.", required: true },
                ToolParamSpec { name: "input2", description: "Input reference raster path.", required: true },
                ToolParamSpec { name: "output", description: "Optional HTML report output path (alias: output_html_file).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("class.tif"));
        defaults.insert("input2".to_string(), json!("reference.tif"));
        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("class.tif"));
        example.insert("input2".to_string(), json!("reference.tif"));
        example.insert("output".to_string(), json!("kappa_index.html"));
        ToolManifest {
            id: "kappa_index".to_string(),
            display_name: "Kappa Index".to_string(),
            summary: "Computes Cohen's kappa and agreement metrics between two categorical rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input classification raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input reference raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_kappa_index".to_string(),
                description: "Compute kappa and confusion matrix metrics for two classified rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let html_output_path = parse_optional_html_report_path(args)?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let counts = (0..in1.data.len())
            .into_par_iter()
            .fold(
                || HashMap::<(i64, i64), usize>::new(),
                |mut acc, i| {
                    let z1 = in1.data.get_f64(i);
                    let z2 = in2.data.get_f64(i);
                    if in1.is_nodata(z1) || in2.is_nodata(z2) {
                        return acc;
                    }
                    let c1 = z1.round() as i64;
                    let c2 = z2.round() as i64;
                    *acc.entry((c1, c2)).or_insert(0) += 1;
                    acc
                },
            )
            .reduce(
                || HashMap::<(i64, i64), usize>::new(),
                |mut a, b| {
                    for (k, v) in b {
                        *a.entry(k).or_insert(0) += v;
                    }
                    a
                },
            );

        let mut classes = BTreeSet::<i64>::new();
        for &(c1, c2) in counts.keys() {
            classes.insert(c1);
            classes.insert(c2);
        }

        let counts: BTreeMap<(i64, i64), usize> = counts.into_iter().collect();

        let classes: Vec<i64> = classes.into_iter().collect();
        if classes.is_empty() {
            return Err(ToolError::Validation("no overlapping valid categorical cells were found".to_string()));
        }

        let matrix: Vec<Vec<usize>> = classes
            .par_iter()
            .map(|rv| {
                classes
                    .iter()
                    .map(|cv| *counts.get(&(*rv, *cv)).unwrap_or(&0))
                    .collect()
            })
            .collect();

        let row_totals: Vec<usize> = matrix
            .par_iter()
            .map(|row| row.iter().copied().sum::<usize>())
            .collect();
        let col_totals: Vec<usize> = (0..classes.len())
            .into_par_iter()
            .map(|ci| matrix.iter().map(|row| row[ci]).sum::<usize>())
            .collect();
        let total: usize = row_totals.iter().copied().sum();
        let diag: usize = (0..classes.len())
            .into_par_iter()
            .map(|i| matrix[i][i])
            .sum();

        if total == 0 {
            return Err(ToolError::Validation("no overlapping valid cells were found".to_string()));
        }

        let expected: f64 = row_totals
            .iter()
            .zip(col_totals.iter())
            .map(|(r, c)| (*r as f64 * *c as f64) / total as f64)
            .sum();
        let kappa = if (total as f64 - expected).abs() < 1.0e-12 {
            0.0
        } else {
            (diag as f64 - expected) / (total as f64 - expected)
        };

        let producers_accuracy: Vec<f64> = (0..classes.len())
            .into_par_iter()
            .map(|i| {
                if col_totals[i] > 0 {
                    matrix[i][i] as f64 / col_totals[i] as f64
                } else {
                    f64::NAN
                }
            })
            .collect();

        let users_accuracy: Vec<f64> = (0..classes.len())
            .into_par_iter()
            .map(|i| {
                if row_totals[i] > 0 {
                    matrix[i][i] as f64 / row_totals[i] as f64
                } else {
                    f64::NAN
                }
            })
            .collect();

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "classes": classes.clone(),
            "matrix": matrix.clone(),
            "overall_accuracy": diag as f64 / total as f64,
            "kappa_index": kappa,
            "producers_accuracy": producers_accuracy.clone(),
            "users_accuracy": users_accuracy.clone(),
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let mut table_html = String::from("<table align=\"center\"><caption>Contingency Table</caption><tr><th class=\"headerCell\">Class Data \\ Reference Data</th>");
            for class in &classes {
                table_html.push_str(&format!("<th class=\"headerCell\">{}</th>", class));
            }
            table_html.push_str("<th class=\"headerCell\">Row Total</th><th class=\"headerCell\">User's Accuracy</th></tr>");

            for (ri, class) in classes.iter().enumerate() {
                table_html.push_str(&format!("<tr><th class=\"headerCell\">{}</th>", class));
                for value in &matrix[ri] {
                    table_html.push_str(&format!("<td class=\"numberCell\">{}</td>", value));
                }
                table_html.push_str(&format!("<td class=\"numberCell\">{}</td>", row_totals[ri]));
                if users_accuracy[ri].is_finite() {
                    table_html.push_str(&format!("<td class=\"numberCell\">{:.2}%</td>", users_accuracy[ri] * 100.0));
                } else {
                    table_html.push_str("<td class=\"numberCell\"></td>");
                }
                table_html.push_str("</tr>");
            }

            table_html.push_str("<tr><th class=\"headerCell\">Column Totals</th>");
            for value in &col_totals {
                table_html.push_str(&format!("<td class=\"numberCell\">{}</td>", value));
            }
            table_html.push_str(&format!("<td class=\"numberCell\">{}</td><td class=\"numberCell\"></td></tr>", total));

            table_html.push_str("<tr><th class=\"headerCell\">Producer's Accuracy</th>");
            for acc in &producers_accuracy {
                if acc.is_finite() {
                    table_html.push_str(&format!("<td class=\"numberCell\">{:.2}%</td>", acc * 100.0));
                } else {
                    table_html.push_str("<td class=\"numberCell\"></td>");
                }
            }
            table_html.push_str("<td class=\"numberCell\"></td><td class=\"numberCell\"></td></tr></table>");

            let body = format!(
                "<h1>Kappa Index of Agreement</h1><p><strong>Classification Data</strong>: {}</p><p><strong>Reference Data</strong>: {}</p>{}<p><strong>Overall Accuracy</strong>: {:.2}%</p><p><strong>Kappa</strong>: {:.4}</p>",
                input1_path,
                input2_path,
                table_html,
                (diag as f64 / total as f64) * 100.0,
                kappa
            );
            let html = html_document("Kappa Index of Agreement", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for PairedSampleTTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "paired_sample_t_test",
            display_name: "Paired Sample T Test",
            summary: "Performs a paired-sample t-test on two rasters using paired valid cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First input raster path.", required: true },
                ToolParamSpec { name: "input2", description: "Second input raster path.", required: true },
                ToolParamSpec { name: "num_samples", description: "Optional sample size with replacement.", required: false },
                ToolParamSpec { name: "output", description: "Optional HTML report output path (alias: output_html_file).", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("input1.tif"));
        defaults.insert("input2".to_string(), json!("input2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("before.tif"));
        example.insert("input2".to_string(), json!("after.tif"));
        example.insert("num_samples".to_string(), json!(1000));
        example.insert("output".to_string(), json!("paired_sample_t_test.html"));

        ToolManifest {
            id: "paired_sample_t_test".to_string(),
            display_name: "Paired Sample T Test".to_string(),
            summary: "Performs a paired-sample t-test on two rasters using paired valid cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Optional sample size with replacement.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional HTML report output path (alias: output_html_file).".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_paired_sample_t_test".to_string(),
                description: "Run a paired t-test on two rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        let _ = parse_optional_html_report_path(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let requested_samples = args.get("num_samples").and_then(|v| v.as_u64()).map(|v| v as usize);
        let html_output_path = parse_optional_html_report_path(args)?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let paired_diffs = collect_paired_differences(&in1, &in2);
        if paired_diffs.len() < 2 {
            return Err(ToolError::Validation("fewer than two valid paired cells were found".to_string()));
        }

        let diffs = if let Some(n) = requested_samples {
            if n == 0 {
                return Err(ToolError::Validation("num_samples must be greater than zero when provided".to_string()));
            }
            sample_with_replacement(&paired_diffs, n)
        } else {
            paired_diffs
        };

        let n = diffs.len();
        let n_f = n as f64;
        let (sum, sq_sum) = diffs
            .par_iter()
            .map(|d| (*d, d * d))
            .reduce(|| (0.0f64, 0.0f64), |a, b| (a.0 + b.0, a.1 + b.1));
        let mean = sum / n_f;
        let variance = (sq_sum / n_f - mean * mean).max(0.0);
        let std_dev = variance.sqrt();
        let std_err = if n > 0 { std_dev / n_f.sqrt() } else { 0.0 };
        let t_value = if std_err > 0.0 { mean / std_err } else { 0.0 };

        // Legacy p-value uses a t-to-z approximation; this uses a direct normal approximation.
        let p_value = two_tailed_normal_p(t_value);

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "num_samples": n,
            "sampled": requested_samples.is_some(),
            "mean_difference": mean,
            "std_dev_difference": std_dev,
            "std_error": std_err,
            "t_value": t_value,
            "p_value": p_value,
            "reject_equal_means_at_0_05": p_value < 0.05,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));

        if let Some(path) = html_output_path {
            let mut sorted_diffs = diffs.clone();
            sorted_diffs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let num_bins = 100usize;
            let min_val = *sorted_diffs.first().unwrap_or(&0.0);
            let max_val = *sorted_diffs.last().unwrap_or(&0.0);
            let mut range = max_val - min_val;
            if range.abs() < 1.0e-12 {
                range = 1.0;
            }
            let bin_width = range / num_bins as f64;
            let mut freq_data = vec![0usize; num_bins];
            for value in &sorted_diffs {
                let idx = (((*value - min_val) / bin_width).floor() as isize)
                    .clamp(0, num_bins as isize - 1) as usize;
                freq_data[idx] += 1;
            }

            let histo = Histogram {
                parent_id: "paired_diffs_cdf".to_string(),
                width: 700.0,
                height: 500.0,
                freq_data,
                min_bin_val: min_val,
                bin_width,
                x_axis_label: "Paired Difference".to_string(),
                cumulative: true,
            };

            let p_value_str = if p_value > 0.001 {
                format!("{p_value:.4}")
            } else {
                "&lt;0.001".to_string()
            };
            let result_str = if p_value < 0.05 {
                "The test <strong>rejects</strong> the null hypothesis that the paired mean difference equals zero."
            } else {
                "The test <strong>fails to reject</strong> the null hypothesis that the paired mean difference equals zero."
            };

            let body = format!(
                "<h1>Paired-Samples <em>t</em>-Test Report</h1><p><strong>Image 1</strong>: {}<br><strong>Image 2</strong>: {}<br><strong>Sample size (N)</strong>: {}<br><strong>Mean of differences</strong>: {:.4}<br><strong>Std. Dev. of differences</strong>: {:.4}<br><strong>Estimated standard error</strong>: {:.4}<br><strong>Test Statistic (<em>t</em>)</strong>: {:.4}<br><strong>Two-tailed Significance (<em>p</em>-value)</strong>: {}<br><strong>Result</strong>: {}</p><p><strong>Caveat</strong>: Given a sufficiently large sample, extremely small and non-notable differences can be found to be statistically significant, and statistical significance says nothing about the practical significance of a difference.</p><div id='paired_diffs_cdf' align=\"center\">{}</div>",
                input1_path,
                input2_path,
                n,
                mean,
                std_dev,
                std_err,
                t_value,
                p_value_str,
                result_str,
                histo.get_svg()
            );

            let html = html_document("Paired-Samples t-Test", &body);
            let written = write_html_report(&path, &html)?;
            outputs.insert("report_html".to_string(), json!(written));
        }

        Ok(ToolRunResult { outputs })
    }
}

impl Tool for TwoSampleKsTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "two_sample_ks_test",
            display_name: "Two Sample K-S Test",
            summary: "Performs a two-sample Kolmogorov-Smirnov test on two raster value distributions.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First input raster path.", required: true },
                ToolParamSpec { name: "input2", description: "Second input raster path.", required: true },
                ToolParamSpec { name: "num_samples", description: "Optional sample size with replacement per raster.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("input1.tif"));
        defaults.insert("input2".to_string(), json!("input2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("before.tif"));
        example.insert("input2".to_string(), json!("after.tif"));
        example.insert("num_samples".to_string(), json!(2000));

        ToolManifest {
            id: "two_sample_ks_test".to_string(),
            display_name: "Two Sample K-S Test".to_string(),
            summary: "Performs a two-sample Kolmogorov-Smirnov test on two raster value distributions.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Optional sample size with replacement per raster.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_two_sample_ks_test".to_string(),
                description: "Run a two-sample K-S test on two rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let requested_samples = args.get("num_samples").and_then(|v| v.as_u64()).map(|v| v as usize);

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let values1 = collect_valid_values(&in1);
        let values2 = collect_valid_values(&in2);
        if values1.is_empty() || values2.is_empty() {
            return Err(ToolError::Validation("one or both input rasters contain no valid cells".to_string()));
        }

        let (sample1, sample2) = if let Some(n) = requested_samples {
            if n == 0 {
                return Err(ToolError::Validation("num_samples must be greater than zero when provided".to_string()));
            }
            (sample_with_replacement(&values1, n), sample_with_replacement(&values2, n))
        } else {
            (values1, values2)
        };

        let (dmax, p_value) = two_sample_ks_statistic(&sample1, &sample2);

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "n1": sample1.len(),
            "n2": sample2.len(),
            "sampled": requested_samples.is_some(),
            "dmax": dmax,
            "p_value": p_value,
            "reject_same_distribution_at_0_05": p_value < 0.05,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for WilcoxonSignedRankTestTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "wilcoxon_signed_rank_test",
            display_name: "Wilcoxon Signed-Rank Test",
            summary: "Performs a Wilcoxon signed-rank test on paired raster differences.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First input raster path.", required: true },
                ToolParamSpec { name: "input2", description: "Second input raster path.", required: true },
                ToolParamSpec { name: "num_samples", description: "Optional sample size with replacement.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("input1.tif"));
        defaults.insert("input2".to_string(), json!("input2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("before.tif"));
        example.insert("input2".to_string(), json!("after.tif"));
        example.insert("num_samples".to_string(), json!(1000));

        ToolManifest {
            id: "wilcoxon_signed_rank_test".to_string(),
            display_name: "Wilcoxon Signed-Rank Test".to_string(),
            summary: "Performs a Wilcoxon signed-rank test on paired raster differences.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Optional sample size with replacement.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_wilcoxon_signed_rank_test".to_string(),
                description: "Run a Wilcoxon signed-rank test on two rasters.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input1")?;
        let _ = parse_raster_path_arg(args, "input2")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;
        let requested_samples = args.get("num_samples").and_then(|v| v.as_u64()).map(|v| v as usize);

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let paired_diffs = collect_paired_differences(&in1, &in2);
        if paired_diffs.len() < 2 {
            return Err(ToolError::Validation("fewer than two valid paired cells were found".to_string()));
        }

        let diffs = if let Some(n) = requested_samples {
            if n == 0 {
                return Err(ToolError::Validation("num_samples must be greater than zero when provided".to_string()));
            }
            sample_with_replacement(&paired_diffs, n)
        } else {
            paired_diffs
        };

        let mut signed_abs: Vec<(f64, f64)> = diffs
            .into_par_iter()
            .filter_map(|d| if d == 0.0 { None } else { Some((d.signum(), d.abs())) })
            .collect();

        if signed_abs.len() < 2 {
            return Err(ToolError::Validation("insufficient non-zero differences for Wilcoxon test".to_string()));
        }

        signed_abs.par_sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut i = 0usize;
        let mut w_pos = 0.0f64;
        let mut w_neg = 0.0f64;
        while i < signed_abs.len() {
            let mut j = i;
            while j + 1 < signed_abs.len() && signed_abs[j + 1].1 == signed_abs[i].1 {
                j += 1;
            }

            let rank_start = i as f64 + 1.0;
            let rank_end = j as f64 + 1.0;
            let avg_rank = 0.5 * (rank_start + rank_end);
            for item in signed_abs.iter().take(j + 1).skip(i) {
                if item.0 > 0.0 {
                    w_pos += avg_rank;
                } else {
                    w_neg -= avg_rank;
                }
            }
            i = j + 1;
        }

        let w = w_pos + w_neg;
        let nr = signed_abs.len() as f64;
        let sigma_w = ((nr * (nr + 1.0) * (2.0 * nr + 1.0)) / 6.0).sqrt();
        let z_value = if sigma_w > 0.0 { w / sigma_w } else { 0.0 };
        let p_value = two_tailed_normal_p(z_value);

        let report = json!({
            "input1": input1_path,
            "input2": input2_path,
            "num_nonzero_pairs": signed_abs.len(),
            "sampled": requested_samples.is_some(),
            "sum_positive_ranks": w_pos,
            "sum_negative_ranks": w_neg,
            "sum_ranks": w,
            "z_value": z_value,
            "p_value": p_value,
            "reject_symmetric_differences_at_0_05": p_value < 0.05,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for MaxTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "max",
            display_name: "Max",
            summary: "Performs a MAX operation on two rasters or a raster and a constant value.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First raster path or numeric constant.", required: true },
                ToolParamSpec { name: "input2", description: "Second raster path or numeric constant.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!("15.0"));
        example.insert("output".to_string(), json!("max_output.tif"));

        ToolManifest {
            id: "max".to_string(),
            display_name: "Max".to_string(),
            summary: "Performs a MAX operation on two rasters or a raster and a constant value.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First raster path or numeric constant.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second raster path or numeric constant.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_max".to_string(),
                description: "Compute cellwise maximum between a raster and a constant.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "max".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_or_constant_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let i1 = parse_raster_or_constant_arg(args, "input1")?;
        let i2 = parse_raster_or_constant_arg(args, "input2")?;
        let output_path = parse_optional_output_path(args, "output")?;

        match (i1, i2) {
            (RasterOrConstant::Raster(p1), RasterOrConstant::Raster(p2)) => {
                let r1 = load_raster(&p1, "input1")?;
                let r2 = load_raster(&p2, "input2")?;
                if r1.rows != r2.rows || r1.cols != r2.cols || r1.bands != r2.bands {
                    return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
                }
                let mut out = r1.clone();
                let out_values: Vec<f64> = (0..out.data.len())
                    .into_par_iter()
                    .map(|i| {
                        let a = r1.data.get_f64(i);
                        let b = r2.data.get_f64(i);
                        if r1.is_nodata(a) || r2.is_nodata(b) {
                            out.nodata
                        } else {
                            a.max(b)
                        }
                    })
                    .collect();
                for (i, z) in out_values.into_iter().enumerate() {
                    out.data.set_f64(i, z);
                }
                let loc = write_or_store_output(out, output_path)?;
                let mut outputs = BTreeMap::new();
                outputs.insert("output".to_string(), typed_raster_output(loc));
                Ok(ToolRunResult { outputs })
            }
            (RasterOrConstant::Raster(p), RasterOrConstant::Constant(c))
            | (RasterOrConstant::Constant(c), RasterOrConstant::Raster(p)) => {
                let r = load_raster(&p, "input")?;
                let mut out = r.clone();
                let out_values: Vec<f64> = (0..out.data.len())
                    .into_par_iter()
                    .map(|i| {
                        let a = r.data.get_f64(i);
                        if r.is_nodata(a) {
                            out.nodata
                        } else {
                            a.max(c)
                        }
                    })
                    .collect();
                for (i, z) in out_values.into_iter().enumerate() {
                    out.data.set_f64(i, z);
                }
                let loc = write_or_store_output(out, output_path)?;
                let mut outputs = BTreeMap::new();
                outputs.insert("output".to_string(), typed_raster_output(loc));
                Ok(ToolRunResult { outputs })
            }
            (RasterOrConstant::Constant(a), RasterOrConstant::Constant(b)) => {
                let mut outputs = BTreeMap::new();
                outputs.insert("value".to_string(), json!(a.max(b)));
                Ok(ToolRunResult { outputs })
            }
        }
    }
}

impl Tool for MinTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "min",
            display_name: "Min",
            summary: "Performs a MIN operation on two rasters or a raster and a constant value.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input1", description: "First raster path or numeric constant.", required: true },
                ToolParamSpec { name: "input2", description: "Second raster path or numeric constant.", required: true },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("in1.tif"));
        defaults.insert("input2".to_string(), json!("in2.tif"));

        let mut example = ToolArgs::new();
        example.insert("input1".to_string(), json!("in1.tif"));
        example.insert("input2".to_string(), json!("15.0"));
        example.insert("output".to_string(), json!("min_output.tif"));

        ToolManifest {
            id: "min".to_string(),
            display_name: "Min".to_string(),
            summary: "Performs a MIN operation on two rasters or a raster and a constant value.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First raster path or numeric constant.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second raster path or numeric constant.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_min".to_string(),
                description: "Compute cellwise minimum between a raster and a constant.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "min".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_or_constant_arg(args, "input1")?;
        let _ = parse_raster_or_constant_arg(args, "input2")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let i1 = parse_raster_or_constant_arg(args, "input1")?;
        let i2 = parse_raster_or_constant_arg(args, "input2")?;
        let output_path = parse_optional_output_path(args, "output")?;

        match (i1, i2) {
            (RasterOrConstant::Raster(p1), RasterOrConstant::Raster(p2)) => {
                let r1 = load_raster(&p1, "input1")?;
                let r2 = load_raster(&p2, "input2")?;
                if r1.rows != r2.rows || r1.cols != r2.cols || r1.bands != r2.bands {
                    return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
                }
                let mut out = r1.clone();
                let out_values: Vec<f64> = (0..out.data.len())
                    .into_par_iter()
                    .map(|i| {
                        let a = r1.data.get_f64(i);
                        let b = r2.data.get_f64(i);
                        if r1.is_nodata(a) || r2.is_nodata(b) {
                            out.nodata
                        } else {
                            a.min(b)
                        }
                    })
                    .collect();
                for (i, z) in out_values.into_iter().enumerate() {
                    out.data.set_f64(i, z);
                }
                let loc = write_or_store_output(out, output_path)?;
                let mut outputs = BTreeMap::new();
                outputs.insert("output".to_string(), typed_raster_output(loc));
                Ok(ToolRunResult { outputs })
            }
            (RasterOrConstant::Raster(p), RasterOrConstant::Constant(c))
            | (RasterOrConstant::Constant(c), RasterOrConstant::Raster(p)) => {
                let r = load_raster(&p, "input")?;
                let mut out = r.clone();
                let out_values: Vec<f64> = (0..out.data.len())
                    .into_par_iter()
                    .map(|i| {
                        let a = r.data.get_f64(i);
                        if r.is_nodata(a) {
                            out.nodata
                        } else {
                            a.min(c)
                        }
                    })
                    .collect();
                for (i, z) in out_values.into_iter().enumerate() {
                    out.data.set_f64(i, z);
                }
                let loc = write_or_store_output(out, output_path)?;
                let mut outputs = BTreeMap::new();
                outputs.insert("output".to_string(), typed_raster_output(loc));
                Ok(ToolRunResult { outputs })
            }
            (RasterOrConstant::Constant(a), RasterOrConstant::Constant(b)) => {
                let mut outputs = BTreeMap::new();
                outputs.insert("value".to_string(), json!(a.min(b)));
                Ok(ToolRunResult { outputs })
            }
        }
    }
}

impl Tool for QuantilesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "quantiles",
            display_name: "Quantiles",
            summary: "Transforms raster values into quantile classes.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "num_quantiles", description: "Number of quantiles (default 5).", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("num_quantiles".to_string(), json!(5));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("num_quantiles".to_string(), json!(5));
        example.insert("output".to_string(), json!("dem_quantiles.tif"));
        ToolManifest {
            id: "quantiles".to_string(),
            display_name: "Quantiles".to_string(),
            summary: "Transforms raster values into quantile classes.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_quantiles".to_string(), description: "Number of quantiles (default 5).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_quantiles".to_string(),
                description: "Assign each raster cell to a quantile class.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let num_quantiles = args
            .get("num_quantiles")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(5)
            .max(2);
        let output_path = parse_optional_output_path(args, "output")?;
        let input = load_raster(&input_path, "input")?;

        // Compute min/max in one parallel pass
        let (min_val, max_val, num_valid) = (0..input.data.len())
            .into_par_iter()
            .fold(
                || (f64::INFINITY, f64::NEG_INFINITY, 0usize),
                |(mut mn, mut mx, mut n), i| {
                    let z = input.data.get_f64(i);
                    if !input.is_nodata(z) {
                        if z < mn {
                            mn = z;
                        }
                        if z > mx {
                            mx = z;
                        }
                        n += 1;
                    }
                    (mn, mx, n)
                },
            )
            .reduce(
                || (f64::INFINITY, f64::NEG_INFINITY, 0usize),
                |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
            );
        if num_valid == 0 {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        // Build histogram with fixed number of bins (same approach as legacy)
        const NUM_BINS: usize = 10_000;
        let value_range = (max_val - min_val).max(f64::EPSILON);
        let bin_size = value_range / NUM_BINS as f64;
        let histo = (0..input.data.len())
            .into_par_iter()
            .fold(
                || vec![0u64; NUM_BINS],
                |mut local_histo, i| {
                    let z = input.data.get_f64(i);
                    if !input.is_nodata(z) {
                        let b = ((z - min_val) / bin_size).floor() as usize;
                        let b = b.min(NUM_BINS - 1);
                        local_histo[b] += 1;
                    }
                    local_histo
                },
            )
            .reduce(
                || vec![0u64; NUM_BINS],
                |mut a, b| {
                    for (i, v) in b.into_iter().enumerate() {
                        a[i] += v;
                    }
                    a
                },
            );

        // Cumulative histogram → quantile class per bin
        let mut cumulative = 0u64;
        let mut bin_class = vec![0u8; NUM_BINS];
        for b in 0..NUM_BINS {
            cumulative += histo[b];
            // quantile class 1..=num_quantiles
            let klass = ((cumulative as f64 / num_valid as f64) * num_quantiles as f64).ceil() as usize;
            bin_class[b] = klass.clamp(1, num_quantiles) as u8;
        }

        let mut out = Raster::new(RasterConfig {
            rows: input.rows,
            cols: input.cols,
            bands: input.bands,
            x_min: input.x_min,
            y_min: input.y_min,
            cell_size: input.cell_size_x,
            cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata,
            data_type: DataType::I16,
            crs: input.crs.clone(),
            metadata: input.metadata.clone(),
        });

        let out_values: Vec<f64> = (0..input.data.len())
            .into_par_iter()
            .map(|i| {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    input.nodata
                } else {
                    let b = ((z - min_val) / bin_size).floor() as usize;
                    let b = b.min(NUM_BINS - 1);
                    bin_class[b] as f64
                }
            })
            .collect();

        for (i, z) in out_values.into_iter().enumerate() {
            out.data.set_f64(i, z);
        }

        let loc = write_or_store_output(out, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ListUniqueValuesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "list_unique_values",
            display_name: "List Unique Values",
            summary: "Lists unique values and frequencies in a vector attribute field.",
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector path.", required: true },
                ToolParamSpec { name: "field", description: "Attribute field name.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.shp"));
        defaults.insert("field".to_string(), json!("class"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("lakes.shp"));
        example.insert("field".to_string(), json!("HEIGHT"));

        ToolManifest {
            id: "list_unique_values".to_string(),
            display_name: "List Unique Values".to_string(),
            summary: "Lists unique values and frequencies in a vector attribute field.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Attribute field name.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_list_unique_values".to_string(),
                description: "List frequencies for a vector field.".to_string(),
                args: example,
            }],
            tags: vec!["vector".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let _ = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;

        let layer = load_vector(&input_path, "input")?;
        let idx = layer
            .schema
            .field_index(field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", field)))?;

        let freq_hash = layer
            .features
            .par_iter()
            .fold(
                HashMap::<String, usize>::new,
                |mut acc, f| {
                    let key = f
                        .attributes
                        .get(idx)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "null".to_string());
                    *acc.entry(key).or_insert(0) += 1;
                    acc
                },
            )
            .reduce(
                HashMap::<String, usize>::new,
                |mut a, b| {
                    for (k, v) in b {
                        *a.entry(k).or_insert(0) += v;
                    }
                    a
                },
            );

        let freq: BTreeMap<String, usize> = freq_hash.into_iter().collect();

        let report = json!({"field": field, "categories": freq}).to_string();
        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RootMeanSquareErrorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "root_mean_square_error",
            display_name: "Root Mean Square Error",
            summary: "Calculates RMSE and related accuracy statistics between two rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Comparison raster path.", required: true },
                ToolParamSpec { name: "base", description: "Base raster path.", required: true },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem_a.tif"));
        defaults.insert("base".to_string(), json!("dem_b.tif"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("base".to_string(), json!("dem_reference.tif"));

        ToolManifest {
            id: "root_mean_square_error".to_string(),
            display_name: "Root Mean Square Error".to_string(),
            summary: "Calculates RMSE and related accuracy statistics between two rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Comparison raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "base".to_string(), description: "Base raster path.".to_string(), required: true },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_root_mean_square_error".to_string(),
                description: "Compute vertical accuracy metrics between two DEMs.".to_string(),
                args: example,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_raster_path_arg(args, "base")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let base_path = parse_raster_path_arg(args, "base")?;
        let input = load_raster(&input_path, "input")?;
        let base = load_raster(&base_path, "base")?;

        let diffs: Vec<f64>;
        let same_grid = input.rows == base.rows && input.cols == base.cols;

        if same_grid {
            diffs = (0..input.data.len())
                .into_par_iter()
                .filter_map(|i| {
                    let z1 = input.data.get_f64(i);
                    let z2 = base.data.get_f64(i);
                    if input.is_nodata(z1) || base.is_nodata(z2) {
                        None
                    } else {
                        Some(z2 - z1)
                    }
                })
                .collect();
        } else {
            diffs = (0..input.rows)
                .into_par_iter()
                .map(|row| {
                    let row_i = row as isize;
                    let mut local = Vec::new();
                    for col in 0..input.cols as isize {
                        let z1 = input.get(0, row_i, col);
                        if input.is_nodata(z1) {
                            continue;
                        }
                        let x = input.col_center_x(col);
                        let y = input.row_center_y(row_i);
                        if let Some(z2) = base.sample_world(
                            0,
                            x,
                            y,
                            ResampleMethod::Bilinear,
                            NodataPolicy::PartialKernel,
                        ) {
                            local.push(z2 - z1);
                        }
                    }
                    local
                })
                .reduce(
                    Vec::new,
                    |mut a, mut b| {
                        a.append(&mut b);
                        a
                    },
                );
        }

        if diffs.is_empty() {
            return Err(ToolError::Validation("no overlapping valid cells found for comparison".to_string()));
        }

        let n = diffs.len() as f64;
        let (sum, sq_sum) = diffs
            .par_iter()
            .map(|d| (*d, d * d))
            .reduce(|| (0.0f64, 0.0f64), |a, b| (a.0 + b.0, a.1 + b.1));
        let mean_vertical_error = sum / n;
        let rmse = (sq_sum / n).sqrt();

        let mut abs_residuals: Vec<f64> = diffs.par_iter().map(|d| d.abs()).collect();
        abs_residuals.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx90 = ((0.9 * n).floor() as usize).min(abs_residuals.len() - 1);
        let le90 = abs_residuals[idx90];

        let report = json!({
            "comparison_file": input_path,
            "base_file": base_path,
            "mean_vertical_error": mean_vertical_error,
            "rmse": rmse,
            "accuracy_95_percent": rmse * 1.96,
            "le90": le90,
            "num_cells": diffs.len(),
            "resampling": if same_grid { "none" } else { "bilinear" },
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ZonalStatisticsTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for ZonalStatisticsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "zonal_statistics",
            display_name: "Zonal Statistics",
            summary: "Summarises the values of a data raster within zones defined by a feature raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input data raster path.", required: true },
                ToolParamSpec { name: "features", description: "Zone-definition raster path (integer zone IDs).", required: true },
                ToolParamSpec {
                    name: "stat_type",
                    description: "Statistic: 'mean' (default), 'median', 'min', 'max', 'range', 'standard deviation', 'diversity', or 'total'.",
                    required: false,
                },
                ToolParamSpec { name: "zero_is_background", description: "Exclude cells with zone ID 0. Default: false.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("stat_type".to_string(), json!("mean"));
        defaults.insert("zero_is_background".to_string(), json!(false));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("slope.tif"));
        example.insert("features".to_string(), json!("watersheds.tif"));
        example.insert("stat_type".to_string(), json!("mean"));
        example.insert("output".to_string(), json!("zonal_mean.tif"));
        ToolManifest {
            id: "zonal_statistics".to_string(),
            display_name: "Zonal Statistics".to_string(),
            summary: "Summarises the values of a data raster within zones defined by a feature raster.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input data raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "features".to_string(), description: "Zone-definition raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "stat_type".to_string(), description: "Statistic: 'mean', 'median', 'min', 'max', 'range', 'standard deviation', 'diversity', 'total'. Default: 'mean'.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_is_background".to_string(), description: "Exclude zone-0 cells. Default: false.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "mean_by_zone".to_string(), description: "Compute mean slope within each watershed zone.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_raster_path_arg(args, "features")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let features_path = parse_raster_path_arg(args, "features")?;
        let raw_stat = args.get("stat_type").and_then(|v| v.as_str()).unwrap_or("mean").to_lowercase();
        let stat_type = if raw_stat.contains("med") { "median" }
            else if raw_stat.contains("min") { "min" }
            else if raw_stat.contains("max") { "max" }
            else if raw_stat.contains("ran") { "range" }
            else if raw_stat.contains("dev") { "standard deviation" }
            else if raw_stat.contains("div") { "diversity" }
            else if raw_stat.contains("tot") || raw_stat.contains("sum") { "total" }
            else { "mean" };
        let zero_is_background = args.get("zero_is_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = load_raster(&input_path, "input")?;
        let features = load_raster(&features_path, "features")?;
        if input.rows != features.rows || input.cols != features.cols {
            return Err(ToolError::Validation(format!(
                "'input' and 'features' must have the same dimensions ({} x {} vs {} x {})",
                input.rows, input.cols, features.rows, features.cols
            )));
        }

        let n = input.rows * input.cols;
        let (mut zone_data, zone_set) = (0..n)
            .into_par_iter()
            .fold(
                || {
                    (
                        HashMap::<i64, Vec<f64>>::new(),
                        HashMap::<i64, std::collections::HashSet<i64>>::new(),
                    )
                },
                |mut acc, i| {
                    let z_val = features.data.get_f64(i);
                    if features.is_nodata(z_val) {
                        return acc;
                    }
                    let zone_id = z_val.round() as i64;
                    if zero_is_background && zone_id == 0 {
                        return acc;
                    }
                    let data_val = input.data.get_f64(i);
                    if input.is_nodata(data_val) {
                        return acc;
                    }
                    acc.0.entry(zone_id).or_default().push(data_val);
                    acc.1
                        .entry(zone_id)
                        .or_default()
                        .insert((data_val * 1000.0).round() as i64);
                    acc
                },
            )
            .reduce(
                || {
                    (
                        HashMap::<i64, Vec<f64>>::new(),
                        HashMap::<i64, std::collections::HashSet<i64>>::new(),
                    )
                },
                |mut a, b| {
                    for (k, mut vals) in b.0 {
                        a.0.entry(k).or_default().append(&mut vals);
                    }
                    for (k, set_b) in b.1 {
                        a.1.entry(k).or_default().extend(set_b);
                    }
                    a
                },
            );

        let zone_stat: HashMap<i64, f64> = zone_data
            .par_iter_mut()
            .map(|(&id, data)| {
                let stat_val = match stat_type {
                    "min" => data.iter().cloned().fold(f64::INFINITY, f64::min),
                    "max" => data.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                    "range" => {
                        data.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                            - data.iter().cloned().fold(f64::INFINITY, f64::min)
                    }
                    "total" => data.iter().sum(),
                    "diversity" => zone_set.get(&id).map(|s| s.len()).unwrap_or(0) as f64,
                    "standard deviation" => {
                        let cnt = data.len() as f64;
                        if cnt < 2.0 {
                            0.0
                        } else {
                            let mean = data.iter().sum::<f64>() / cnt;
                            let var = data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (cnt - 1.0);
                            var.sqrt()
                        }
                    }
                    "median" => {
                        data.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let mid = data.len() / 2;
                        if data.len() % 2 == 0 {
                            (data[mid - 1] + data[mid]) / 2.0
                        } else {
                            data[mid]
                        }
                    }
                    _ => data.iter().sum::<f64>() / data.len() as f64,
                };
                (id, stat_val)
            })
            .collect();

        let mut output = Raster::new(RasterConfig {
            rows: input.rows, cols: input.cols, bands: 1,
            x_min: input.x_min, y_min: input.y_min,
            cell_size: input.cell_size_x, cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata, data_type: DataType::F32,
            crs: input.crs.clone(), metadata: input.metadata.clone(),
            ..Default::default()
        });
        let out_vals: Vec<f64> = (0..n)
            .into_par_iter()
            .map(|i| {
                let z_val = features.data.get_f64(i);
                if features.is_nodata(z_val) {
                    return input.nodata;
                }
                let zone_id = z_val.round() as i64;
                if zero_is_background && zone_id == 0 {
                    return input.nodata;
                }
                zone_stat.get(&zone_id).copied().unwrap_or(input.nodata)
            })
            .collect();

        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            data_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cell)| *cell = out_vals[i] as f32);
        } else {
            for (i, v) in out_vals.iter().enumerate() {
                output.data.set_f64(i, *v);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut sorted_ids: Vec<i64> = zone_stat.keys().copied().collect();
        sorted_ids.par_sort_unstable();
        let mut md = format!("| Zone ID | {} |\n|---------|-------|\n", stat_type);
        for id in &sorted_ids {
            md.push_str(&format!("| {} | {:.6} |\n", id, zone_stat[id]));
        }
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        outputs.insert("report".to_string(), json!(md));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TurningBandsSimulationTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for TurningBandsSimulationTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "turning_bands_simulation",
            display_name: "Turning Bands Simulation",
            summary: "Creates a spatially-autocorrelated random field using the turning bands algorithm.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Base raster (provides grid geometry).", required: true },
                ToolParamSpec { name: "range", description: "Correlation range in map units. Default: 1.0.", required: false },
                ToolParamSpec { name: "iterations", description: "Number of band directions (≥5). Default: 1000.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("range".to_string(), json!(1.0));
        defaults.insert("iterations".to_string(), json!(1000));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("range".to_string(), json!(500.0));
        example.insert("iterations".to_string(), json!(1000));
        example.insert("output".to_string(), json!("random_field.tif"));
        ToolManifest {
            id: "turning_bands_simulation".to_string(),
            display_name: "Turning Bands Simulation".to_string(),
            summary: "Creates a spatially-autocorrelated random field using the turning bands algorithm.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Base raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "range".to_string(), description: "Correlation range in map units. Default: 1.0.".to_string(), required: false },
                ToolParamDescriptor { name: "iterations".to_string(), description: "Number of band directions. Default: 1000.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic".to_string(), description: "Simulate a correlated random field.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "simulation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let range = args.get("range").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.0);
        let iterations = args.get("iterations").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1000).max(5);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = load_raster(&input_path, "input")?;
        let rows = input.rows;
        let cols = input.cols;

        let diagonal_size = ((rows as f64 * rows as f64 + cols as f64 * cols as f64).sqrt()) as usize + 1;
        let filter_half_size = ((range / (2.0 * input.cell_size_x)) as usize).max(1);
        let filter_size = filter_half_size * 2 + 1;
        let cell_offsets: Vec<isize> = (0..filter_size as isize).map(|i| i - filter_half_size as isize).collect();
        let w = (36.0 / (filter_half_size as f64 * (filter_half_size as f64 + 1.0) * filter_size as f64)).sqrt();

        let mut accum = vec![0.0f32; rows * cols];
        let mut rng = rand::rng();

        for _ in 0..iterations {
            let mut t = vec![0.0f64; diagonal_size + 2 * filter_half_size];
            t[..diagonal_size]
                .par_iter_mut()
                .for_each(|cell| {
                    let mut local_rng = rand::rng();
                    *cell = sample_standard_normal(&mut local_rng);
                });

            let mut y: Vec<f32> = (0..diagonal_size)
                .into_par_iter()
                .map(|j| {
                    let mut z = 0.0f64;
                    for k in 0..filter_size {
                        let m = cell_offsets[k];
                        z += m as f64 * t[(j as isize + filter_half_size as isize + m) as usize];
                    }
                    (w * z) as f32
                })
                .collect();
            let (sum, sq_sum) = y
                .par_iter()
                .map(|&v| {
                    let vf = v as f64;
                    (vf, vf * vf)
                })
                .reduce(|| (0.0f64, 0.0f64), |a, b| (a.0 + b.0, a.1 + b.1));
            let mean = sum / diagonal_size as f64;
            let variance = (sq_sum / diagonal_size as f64 - mean * mean).max(0.0);
            let stdev = variance.sqrt();
            if stdev > 1.0e-15 {
                y.par_iter_mut()
                    .for_each(|v| *v = ((*v as f64 - mean) / stdev) as f32);
            }

            // Use a random band angle and project cells onto that 1D axis.
            // This avoids expensive per-cell intersection and sqrt calculations.
            let theta = rng.random_range(0.0..std::f64::consts::PI);
            let dir_x = theta.cos();
            let dir_y = theta.sin();

            let max_col = (cols.saturating_sub(1)) as f64;
            let max_row = (rows.saturating_sub(1)) as f64;
            let p00 = 0.0_f64;
            let p10 = max_col * dir_x;
            let p01 = max_row * dir_y;
            let p11 = max_col * dir_x + max_row * dir_y;
            let min_proj = p00.min(p10).min(p01).min(p11);

            accum
                .par_chunks_mut(cols)
                .enumerate()
                .for_each(|(row, row_accum)| {
                    let mut proj = row as f64 * dir_y - min_proj;
                    for cell in row_accum.iter_mut() {
                        let p = (proj.round() as isize).clamp(0, (diagonal_size - 1) as isize) as usize;
                        *cell += y[p];
                        proj += dir_x;
                    }
                });
        }

        let iter_sqrt = (iterations as f32).sqrt();
        let mut output = Raster::new(RasterConfig {
            rows, cols, bands: 1,
            x_min: input.x_min, y_min: input.y_min,
            cell_size: input.cell_size_x, cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata, data_type: DataType::F32,
            crs: input.crs.clone(), metadata: input.metadata.clone(),
            ..Default::default()
        });
        
        // Parallel normalized accumulation into typed F32 buffer (avoids per-cell dispatch overhead)
        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            use rayon::prelude::*;
            data_slice.par_iter_mut().enumerate().for_each(|(i, cell)| {
                *cell = accum[i] / iter_sqrt;
            });
        } else {
            // Fallback: shouldn't happen since we just created F32 output above
            for i in 0..rows * cols { output.data.set_f64(i, (accum[i] / iter_sqrt) as f64); }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// shared helpers: polynomial regression via QR decomposition
// ─────────────────────────────────────────────────────────────────────────────

fn poly_num_coefficients(order: usize) -> usize {
    let mut n = 0;
    for j in 0..=order { for _k in 0..=(order - j) { n += 1; } }
    n
}

fn fit_polynomial_surface(
    x: &[f64], y: &[f64], z: &[f64],
    order: usize,
) -> Result<(Vec<f64>, f64), ToolError> {
    let n = z.len();
    let num_coeff = poly_num_coefficients(order);
    let mut design = vec![0.0f64; n * num_coeff];
    design
        .par_chunks_mut(num_coeff)
        .enumerate()
        .for_each(|(i, row)| {
            let mut m = 0;
            for j in 0..=order {
                for k in 0..=(order - j) {
                    row[m] = x[i].powf(j as f64) * y[i].powf(k as f64);
                    m += 1;
                }
            }
        });
    let mat = DMatrix::from_row_slice(n, num_coeff, &design);
    let qr = mat.clone().qr();
    let r = qr.r();
    if !r.is_invertible() {
        return Err(ToolError::Execution("polynomial regression matrix is not invertible".to_string()));
    }
    let b = DVector::from_row_slice(z);
    let coeffs = (r.try_inverse().unwrap() * qr.q().transpose() * b).as_slice().to_vec();

    let (ss_resid, z_sum, z_ss) = (0..n)
        .into_par_iter()
        .map(|i| {
            let row = &design[i * num_coeff..(i + 1) * num_coeff];
            let y_hat = row
                .iter()
                .zip(coeffs.iter())
                .map(|(a, c)| a * c)
                .sum::<f64>();
            let zi = z[i];
            let resid = zi - y_hat;
            (resid * resid, zi, zi * zi)
        })
        .reduce(
            || (0.0f64, 0.0f64, 0.0f64),
            |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
        );
    let variance = (z_ss - z_sum * z_sum / n as f64) / n as f64;
    let ss_total = (n - 1) as f64 * variance;
    let r_sqr = if ss_total.abs() < 1.0e-15 { 1.0 } else { 1.0 - ss_resid / ss_total };
    Ok((coeffs, r_sqr))
}

fn eval_poly(x_val: f64, y_val: f64, coeffs: &[f64], order: usize, z_offset: f64) -> f64 {
    let num_coeff = poly_num_coefficients(order);
    let mut z = z_offset;
    let mut m = 0usize;
    for j in 0..=order {
        for k in 0..=(order - j) {
            if m < num_coeff { z += x_val.powf(j as f64) * y_val.powf(k as f64) * coeffs[m]; }
            m += 1;
        }
    }
    z
}

// ─────────────────────────────────────────────────────────────────────────────
// TrendSurfaceTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for TrendSurfaceTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "trend_surface",
            display_name: "Trend Surface",
            summary: "Fits a polynomial trend surface to a raster using least-squares regression.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path.", required: true },
                ToolParamSpec { name: "polynomial_order", description: "Polynomial order 1–10. Default: 1.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("polynomial_order".to_string(), json!(1));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("polynomial_order".to_string(), json!(2));
        example.insert("output".to_string(), json!("trend.tif"));
        ToolManifest {
            id: "trend_surface".to_string(),
            display_name: "Trend Surface".to_string(),
            summary: "Fits a polynomial trend surface to a raster using least-squares regression.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "polynomial_order".to_string(), description: "Polynomial order 1–10. Default: 1.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic".to_string(), description: "Fit a 2nd-order trend surface to a DEM.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "input")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let order = args.get("polynomial_order").and_then(|v| v.as_u64()).map(|v| (v as usize).clamp(1, 10)).unwrap_or(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = load_raster(&input_path, "input")?;
        let rows = input.rows;
        let cols = input.cols;

        let min_x = input.x_min;
        let min_y = input.y_min;
        let min_z = input.statistics().min;

        let per_row_samples: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut x_row = Vec::new();
                let mut y_row = Vec::new();
                let mut z_row = Vec::new();
                for col in 0..cols {
                    let z = input.data.get_f64(row * cols + col);
                    if !input.is_nodata(z) {
                        x_row.push(input.col_center_x(col as isize) - min_x);
                        y_row.push(input.row_center_y(row as isize) - min_y);
                        z_row.push(z - min_z);
                    }
                }
                (x_row, y_row, z_row)
            })
            .collect();
        let mut x_data = Vec::new();
        let mut y_data = Vec::new();
        let mut z_data = Vec::new();
        for (x_row, y_row, z_row) in per_row_samples {
            x_data.extend(x_row);
            y_data.extend(y_row);
            z_data.extend(z_row);
        }

        let (coeffs, r_sqr) = fit_polynomial_surface(&x_data, &y_data, &z_data, order)?;

        let mut output = Raster::new(RasterConfig {
            rows, cols, bands: 1,
            x_min: input.x_min, y_min: input.y_min,
            cell_size: input.cell_size_x, cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata, data_type: DataType::F32,
            crs: input.crs.clone(), metadata: input.metadata.clone(),
            ..Default::default()
        });
        let fitted_values: Vec<f64> = (0..rows * cols)
            .into_par_iter()
            .map(|idx| {
                let row = idx / cols;
                let col = idx % cols;
                let x_val = input.col_center_x(col as isize) - min_x;
                let y_val = input.row_center_y(row as isize) - min_y;
                eval_poly(x_val, y_val, &coeffs, order, min_z)
            })
            .collect();
        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            data_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(idx, cell)| *cell = fitted_values[idx] as f32);
        } else {
            for (idx, z) in fitted_values.into_iter().enumerate() {
                output.data.set_f64(idx, z);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let report = json!({
            "polynomial_order": order,
            "r_squared": r_sqr,
            "min_x": min_x,
            "min_y": min_y,
            "min_z": min_z,
            "coefficients": coeffs,
        }).to_string();
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TrendSurfaceVectorPointsTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for TrendSurfaceVectorPointsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "trend_surface_vector_points",
            display_name: "Trend Surface (Vector Points)",
            summary: "Fits a polynomial trend surface to vector point data using least-squares regression.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input vector points file path.", required: true },
                ToolParamSpec { name: "cell_size", description: "Output raster cell size in map units.", required: true },
                ToolParamSpec { name: "field_name", description: "Attribute field to use as Z values. Default: 'FID'.", required: false },
                ToolParamSpec { name: "polynomial_order", description: "Polynomial order 1–10. Default: 1.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("field_name".to_string(), json!("FID"));
        defaults.insert("polynomial_order".to_string(), json!(1));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("points.gpkg"));
        example.insert("cell_size".to_string(), json!(100.0));
        example.insert("field_name".to_string(), json!("elevation"));
        example.insert("polynomial_order".to_string(), json!(2));
        example.insert("output".to_string(), json!("trend.tif"));
        ToolManifest {
            id: "trend_surface_vector_points".to_string(),
            display_name: "Trend Surface (Vector Points)".to_string(),
            summary: "Fits a polynomial trend surface to vector point data using least-squares regression.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector points file path.".to_string(), required: true },
                ToolParamDescriptor { name: "cell_size".to_string(), description: "Output raster cell size in map units.".to_string(), required: true },
                ToolParamDescriptor { name: "field_name".to_string(), description: "Attribute field for Z values. Default: 'FID'.".to_string(), required: false },
                ToolParamDescriptor { name: "polynomial_order".to_string(), description: "Polynomial order 1–10. Default: 1.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic".to_string(), description: "Fit trend surface from elevation points.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "vector".to_string(), "statistics".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_vector_path_arg(args, "input")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if cell_size <= 0.0 {
            return Err(ToolError::Validation("'cell_size' must be greater than 0".to_string()));
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let cell_size = args.get("cell_size").and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::Validation("'cell_size' is required".to_string()))?;
        if cell_size <= 0.0 {
            return Err(ToolError::Validation("'cell_size' must be > 0".to_string()));
        }
        let field_name = args.get("field_name").and_then(|v| v.as_str()).unwrap_or("FID").to_string();
        let order = args.get("polynomial_order").and_then(|v| v.as_u64()).map(|v| (v as usize).clamp(1, 10)).unwrap_or(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let layer = load_vector(&input_path, "input")?;

        let field_idx = layer.schema.field_index(&field_name)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found in vector layer", field_name)))?;

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut min_z = f64::INFINITY;
        let mut x_pts: Vec<f64> = Vec::new();
        let mut y_pts: Vec<f64> = Vec::new();
        let mut z_pts: Vec<f64> = Vec::new();

        for feature in &layer.features {
            let coord = match &feature.geometry {
                Some(wbvector::Geometry::Point(c)) => c,
                _ => continue,
            };
            let z_val = match feature.attributes.get(field_idx).and_then(|v| v.as_f64()) {
                Some(v) => v,
                None => continue,
            };
            min_x = min_x.min(coord.x);
            min_y = min_y.min(coord.y);
            max_x = max_x.max(coord.x);
            max_y = max_y.max(coord.y);
            min_z = min_z.min(z_val);
            x_pts.push(coord.x);
            y_pts.push(coord.y);
            z_pts.push(z_val);
        }

        if x_pts.is_empty() {
            return Err(ToolError::Execution("no valid point features with numeric field values found".to_string()));
        }

        let x_offset = min_x;
        let y_offset = min_y;
        let z_offset = min_z;
        x_pts
            .par_iter_mut()
            .zip(y_pts.par_iter_mut())
            .zip(z_pts.par_iter_mut())
            .for_each(|((x, y), z)| {
                *x -= x_offset;
                *y -= y_offset;
                *z -= z_offset;
            });

        let (coeffs, r_sqr) = fit_polynomial_surface(&x_pts, &y_pts, &z_pts, order)?;

        let out_rows = ((max_y - min_y) / cell_size).ceil() as usize;
        let out_cols = ((max_x - min_x) / cell_size).ceil() as usize;
        let out_y_min = max_y - out_rows as f64 * cell_size;

        let mut output = Raster::new(RasterConfig {
            rows: out_rows, cols: out_cols, bands: 1,
            x_min: min_x, y_min: out_y_min,
            cell_size, nodata: -32768.0, data_type: DataType::F32,
            ..Default::default()
        });
        let fitted_values: Vec<f64> = (0..out_rows * out_cols)
            .into_par_iter()
            .map(|idx| {
                let row = idx / out_cols;
                let col = idx % out_cols;
                let x_val = output.col_center_x(col as isize) - x_offset;
                let y_val = output.row_center_y(row as isize) - y_offset;
                eval_poly(x_val, y_val, &coeffs, order, z_offset)
            })
            .collect();
        if let Some(data_slice) = output.data.as_f32_slice_mut() {
            data_slice
                .par_iter_mut()
                .enumerate()
                .for_each(|(idx, cell)| *cell = fitted_values[idx] as f32);
        } else {
            for (idx, z) in fitted_values.into_iter().enumerate() {
                output.data.set_f64(idx, z);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let report = json!({
            "polynomial_order": order,
            "r_squared": r_sqr,
            "x_offset": x_offset,
            "y_offset": y_offset,
            "z_offset": z_offset,
            "coefficients": coeffs,
        }).to_string();
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RasterCalculatorTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for RasterCalculatorTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "raster_calculator",
            display_name: "Raster Calculator",
            summary: "Evaluates a mathematical expression on a list of input rasters cell-by-cell.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "expression",
                    description: "Math expression. Raster names are quoted: e.g. ('nir' - 'red') / ('nir' + 'red'). Uses evalexpr syntax.",
                    required: true,
                },
                ToolParamSpec { name: "inputs", description: "Ordered list of input raster paths matching quoted names in the expression.", required: true },
                ToolParamSpec { name: "auto_reproject", description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.", required: false },
                ToolParamSpec { name: "auto_reproject_method", description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut example = ToolArgs::new();
        example.insert("expression".to_string(), json!("('nir' - 'red') / ('nir' + 'red')"));
        example.insert("inputs".to_string(), json!(["nir.tif", "red.tif"]));
        example.insert("output".to_string(), json!("ndvi.tif"));
        ToolManifest {
            id: "raster_calculator".to_string(),
            display_name: "Raster Calculator".to_string(),
            summary: "Evaluates a mathematical expression on a list of input rasters cell-by-cell.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "expression".to_string(), description: "Math expression with quoted raster variable names.".to_string(), required: true },
                ToolParamDescriptor { name: "inputs".to_string(), description: "Ordered input raster paths.".to_string(), required: true },
                ToolParamDescriptor { name: "auto_reproject".to_string(), description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.".to_string(), required: false },
                ToolParamDescriptor { name: "auto_reproject_method".to_string(), description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults: ToolArgs::new(),
            examples: vec![ToolExample { name: "ndvi".to_string(), description: "Compute NDVI from NIR and red bands.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "math".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let expression = args.get("expression").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'expression' is required".to_string()))?;
        if expression.trim().is_empty() {
            return Err(ToolError::Validation("'expression' must be non-empty".to_string()));
        }
        if let Some(method) = args.get("auto_reproject_method").and_then(|v| v.as_str()) {
            let method = method.trim();
            if !method.is_empty() && parse_resample_method(method).is_none() {
                return Err(ToolError::Validation(
                    "parameter 'auto_reproject_method' must be one of: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let expression = args.get("expression").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'expression' is required".to_string()))?.to_string();
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let auto_reproject = args
            .get("auto_reproject")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let resample_override = args
            .get("auto_reproject_method")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let delimiter = if expression.contains('"') { '"' } else { '\'' };
        let parts: Vec<&str> = expression.split(delimiter).collect();
        if parts.len() < 3 {
            return Err(ToolError::Validation("expression must contain at least one quoted raster name".to_string()));
        }
        let mut seen = std::collections::HashSet::new();
        let mut var_names: Vec<String> = Vec::new();
        for (i, tok) in parts.iter().enumerate() {
            if i % 2 == 1 {
                let name = tok.trim().to_string();
                if !seen.contains(&name) {
                    seen.insert(name.clone());
                    var_names.push(name);
                }
            }
        }
        let num_inputs = var_names.len();
        if input_paths.len() != num_inputs {
            return Err(ToolError::Validation(format!(
                "expression has {} raster variable(s) but 'inputs' has {} path(s)", num_inputs, input_paths.len()
            )));
        }

        let mut stmt = expression.clone();
        for (i, name) in var_names.iter().enumerate() {
            let quoted = format!("{}{}{}", delimiter, name, delimiter);
            stmt = stmt.replace(&quoted, &format!("value{}", i));
        }

        let mut inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
        let stack_config = RasterStackConfig {
            auto_reproject,
            resampling_method: resample_override,
            allow_no_overlap: false,
        };
        align_and_validate_raster_stack(&mut inputs, &stack_config)
            .map_err(ToolError::Validation)?;
        let rows = inputs[0].rows;
        let cols = inputs[0].cols;
        for (i, r) in inputs.iter().enumerate() {
            if r.rows != rows || r.cols != cols {
                return Err(ToolError::Validation(format!("inputs[{}] has different dimensions from inputs[0]", i)));
            }
        }

        let nodatas: Vec<f64> = inputs.iter().map(|r| r.nodata).collect();
        let stats: Vec<_> = inputs.iter().map(|r| r.statistics()).collect();
        let out_nodata = -32_768.0f64;

        for i in 0..num_inputs {
            let vn = format!("value{}", i);
            stmt = stmt.replace(&format!("nodata({})", vn), &nodatas[i].to_string());
            stmt = stmt.replace(&format!("null({})", vn), &nodatas[i].to_string());
            stmt = stmt.replace(&format!("minvalue({})", vn), &stats[i].min.to_string());
            stmt = stmt.replace(&format!("maxvalue({})", vn), &stats[i].max.to_string());
        }
        stmt = stmt.replace("nodata()", &nodatas[0].to_string());
        stmt = stmt.replace("null()", &nodatas[0].to_string());
        stmt = stmt.replace("minvalue()", &stats[0].min.to_string());
        stmt = stmt.replace("maxvalue()", &stats[0].max.to_string());

        let statement_contains_nodata = expression.contains("nodata") || expression.contains("null");
        let normalized = normalize_conditional_expression(&stmt);
        let expr_tree = build_operator_tree::<DefaultNumericTypes>(&normalized)
            .map_err(|e| ToolError::Validation(format!("invalid expression: {e}")))?;

        let north = inputs[0].y_min + inputs[0].rows as f64 * inputs[0].cell_size_y;
        let south = inputs[0].y_min;
        let east = inputs[0].x_min + inputs[0].cols as f64 * inputs[0].cell_size_x;
        let west = inputs[0].x_min;

        let value_keys: Vec<String> = (0..num_inputs).map(|i| format!("value{}", i)).collect();

        let mut output = Raster::new(RasterConfig {
            rows, cols, bands: 1,
            x_min: inputs[0].x_min, y_min: inputs[0].y_min,
            cell_size: inputs[0].cell_size_x, cell_size_y: Some(inputs[0].cell_size_y),
            nodata: out_nodata, data_type: DataType::F32,
            crs: inputs[0].crs.clone(), metadata: inputs[0].metadata.clone(),
            ..Default::default()
        });

        let row_results: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut row_context = HashMapContext::new();
                let _ = row_context.set_value("rows".to_string(), EvalValue::Float(rows as f64));
                let _ = row_context.set_value("columns".to_string(), EvalValue::Float(cols as f64));
                let _ = row_context.set_value("north".to_string(), EvalValue::Float(north));
                let _ = row_context.set_value("south".to_string(), EvalValue::Float(south));
                let _ = row_context.set_value("east".to_string(), EvalValue::Float(east));
                let _ = row_context.set_value("west".to_string(), EvalValue::Float(west));
                let _ = row_context.set_value("cellsizex".to_string(), EvalValue::Float(inputs[0].cell_size_x));
                let _ = row_context.set_value("cellsizey".to_string(), EvalValue::Float(inputs[0].cell_size_y));
                let _ = row_context.set_value("cellsize".to_string(), EvalValue::Float(0.5 * (inputs[0].cell_size_x + inputs[0].cell_size_y)));
                let _ = row_context.set_value("nodata".to_string(), EvalValue::Float(nodatas[0]));
                let _ = row_context.set_value("null".to_string(), EvalValue::Float(nodatas[0]));
                let _ = row_context.set_value("minvalue".to_string(), EvalValue::Float(stats[0].min));
                let _ = row_context.set_value("maxvalue".to_string(), EvalValue::Float(stats[0].max));
                let _ = row_context.set_value("pi".to_string(), EvalValue::Float(std::f64::consts::PI));
                let _ = row_context.set_value("e".to_string(), EvalValue::Float(std::f64::consts::E));

                let _ = row_context.set_value("row".to_string(), EvalValue::Float(row as f64));
                let _ = row_context.set_value("rowy".to_string(), EvalValue::Float(inputs[0].row_center_y(row as isize)));

                let mut row_vals = Vec::with_capacity(cols);
                for col in 0..cols {
                    let idx = row * cols + col;
                    let _ = row_context.set_value("column".to_string(), EvalValue::Float(col as f64));
                    let _ = row_context.set_value("columnx".to_string(), EvalValue::Float(inputs[0].col_center_x(col as isize)));

                    let mut any_nodata = false;
                    for (key, inp) in value_keys.iter().zip(inputs.iter()) {
                        let v = inp.data.get_f64(idx);
                        if inp.is_nodata(v) {
                            any_nodata = true;
                        }
                        let _ = row_context.set_value(key.clone(), EvalValue::Float(v));
                    }

                    if any_nodata && !statement_contains_nodata {
                        row_vals.push(out_nodata);
                        continue;
                    }

                    let out_val = match expr_tree.eval_with_context(&row_context) {
                        Ok(EvalValue::Float(v)) => v,
                        Ok(EvalValue::Int(v)) => v as f64,
                        Ok(EvalValue::Boolean(b)) => {
                            if b { 1.0 } else { 0.0 }
                        }
                        _ => out_nodata,
                    };
                    row_vals.push(out_val);
                }
                row_vals
            })
            .collect();

        let flat_results: Vec<f64> = row_results
            .into_par_iter()
            .flat_map(|row_vals| row_vals)
            .collect();

        for (idx, &out_val) in flat_results.iter().enumerate() {
            output.data.set_f64(idx, out_val);
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut outputs = BTreeMap::new();
        outputs.insert("output".to_string(), typed_raster_output(loc));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PrincipalComponentAnalysisTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for PrincipalComponentAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "principal_component_analysis",
            display_name: "Principal Component Analysis",
            summary: "Performs PCA on a stack of rasters, returning component images and a JSON report.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Input raster paths (≥3).", required: true },
                ToolParamSpec { name: "auto_reproject", description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.", required: false },
                ToolParamSpec { name: "auto_reproject_method", description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.", required: false },
                ToolParamSpec { name: "num_components", description: "Number of components to output. Default: all.", required: false },
                ToolParamSpec { name: "standardized", description: "Use correlation matrix (standardized PCA). Default: false.", required: false },
                ToolParamSpec { name: "output", description: "Optional base output path; component files named '{stem}_comp1.{ext}' etc.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("standardized".to_string(), json!(false));
        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["b1.tif", "b2.tif", "b3.tif"]));
        example.insert("num_components".to_string(), json!(3));
        example.insert("output".to_string(), json!("pca.tif"));
        ToolManifest {
            id: "principal_component_analysis".to_string(),
            display_name: "Principal Component Analysis".to_string(),
            summary: "Performs PCA on a stack of rasters, returning component images and a JSON report.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "inputs".to_string(), description: "Input raster paths (≥3).".to_string(), required: true },
                ToolParamDescriptor { name: "auto_reproject".to_string(), description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.".to_string(), required: false },
                ToolParamDescriptor { name: "auto_reproject_method".to_string(), description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.".to_string(), required: false },
                ToolParamDescriptor { name: "num_components".to_string(), description: "Number of components. Default: all.".to_string(), required: false },
                ToolParamDescriptor { name: "standardized".to_string(), description: "Use correlation matrix. Default: false.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional base output path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_pca".to_string(), description: "PCA on 3 spectral bands.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "statistics".to_string(), "pca".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        if paths.len() < 3 {
            return Err(ToolError::Validation("'inputs' must contain at least 3 rasters for PCA".to_string()));
        }
        if let Some(method) = args.get("auto_reproject_method").and_then(|v| v.as_str()) {
            let method = method.trim();
            if !method.is_empty() && parse_resample_method(method).is_none() {
                return Err(ToolError::Validation(
                    "parameter 'auto_reproject_method' must be one of: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let standardized = args.get("standardized").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;
        let auto_reproject = args
            .get("auto_reproject")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let resample_override = args
            .get("auto_reproject_method")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let mut inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
        let stack_config = RasterStackConfig {
            auto_reproject,
            resampling_method: resample_override,
            allow_no_overlap: false,
        };
        align_and_validate_raster_stack(&mut inputs, &stack_config)
            .map_err(ToolError::Validation)?;
        let num_images = inputs.len();
        if num_images < 3 {
            return Err(ToolError::Validation("at least 3 input rasters required for PCA".to_string()));
        }
        let rows = inputs[0].rows;
        let cols = inputs[0].cols;
        for (i, r) in inputs.iter().enumerate() {
            if r.rows != rows || r.cols != cols {
                return Err(ToolError::Validation(format!("inputs[{}] has different dimensions from inputs[0]", i)));
            }
        }
        let num_comp = args.get("num_components").and_then(|v| v.as_u64())
            .map(|v| (v as usize).clamp(1, num_images)).unwrap_or(num_images);

        let per_band_stats: Vec<(f64, f64)> = inputs
            .par_iter()
            .map(|r| {
                let st = r.statistics();
                (st.mean, st.valid_count as f64)
            })
            .collect();
        let averages: Vec<f64> = per_band_stats.iter().map(|(mean, _)| *mean).collect();
        let num_cells: Vec<f64> = per_band_stats.iter().map(|(_, count)| *count).collect();

        let n = rows * cols;
        let (total_dev, mut covariances) = (0..n)
            .into_par_iter()
            .fold(
                || {
                    (
                        vec![0.0f64; num_images],
                        vec![vec![0.0f64; num_images]; num_images],
                    )
                },
                |(mut local_dev, mut local_cov), idx| {
                    for i in 0..num_images {
                        let z1 = inputs[i].data.get_f64(idx);
                        if inputs[i].is_nodata(z1) {
                            continue;
                        }
                        local_dev[i] += (z1 - averages[i]).powi(2);
                        for a in 0..num_images {
                            let z2 = inputs[a].data.get_f64(idx);
                            if !inputs[a].is_nodata(z2) {
                                local_cov[i][a] += (z1 - averages[i]) * (z2 - averages[a]);
                            }
                        }
                    }
                    (local_dev, local_cov)
                },
            )
            .reduce(
                || {
                    (
                        vec![0.0f64; num_images],
                        vec![vec![0.0f64; num_images]; num_images],
                    )
                },
                |(mut dev_a, mut cov_a), (dev_b, cov_b)| {
                    for i in 0..num_images {
                        dev_a[i] += dev_b[i];
                        for a in 0..num_images {
                            cov_a[i][a] += cov_b[i][a];
                        }
                    }
                    (dev_a, cov_a)
                },
            );

        let mut corr = vec![vec![0.0f64; num_images]; num_images];
        corr
            .par_iter_mut()
            .zip(covariances.par_iter_mut())
            .enumerate()
            .for_each(|(i, (corr_row, cov_row))| {
                for a in 0..num_images {
                    let denom = (total_dev[i] * total_dev[a]).sqrt();
                    corr_row[a] = if denom.abs() < 1.0e-15 { 0.0 } else { cov_row[a] / denom };
                    cov_row[a] /= (num_cells[i] - 1.0).max(1.0);
                }
            });

        let matrix = if standardized { &corr } else { &covariances };
        let flat: Vec<f64> = matrix.iter().flat_map(|row| row.iter().copied()).collect();
        let cov_mat = DMatrix::from_row_slice(num_images, num_images, &flat);
        let eig = cov_mat.symmetric_eigen();
        let eigenvalues = eig.eigenvalues.as_slice().to_vec();
        let evec_flat = eig.eigenvectors.as_slice().to_vec(); // column-major: col pc = eigenvector pc

        let total_ev: f64 = eigenvalues.par_iter().copied().sum::<f64>().max(1.0e-15);
        let explained: Vec<f64> = eigenvalues
            .par_iter()
            .map(|&e| 100.0 * e / total_ev)
            .collect();

        // Sort by descending explained variance
        let mut component_order: Vec<usize> = (0..num_images).collect();
        component_order.par_sort_by(|a, b| {
            explained[*b]
                .partial_cmp(&explained[*a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Factor loadings
        let mut factor_loadings = vec![vec![0.0f64; num_images]; num_images];
        factor_loadings
            .par_iter_mut()
            .enumerate()
            .for_each(|(j, row)| {
                for k in 0..num_images {
                    let pc = component_order[k];
                    row[k] = if !standardized {
                        let cov_jj = covariances[j][j].sqrt();
                        if cov_jj > 1.0e-15 {
                            evec_flat[pc * num_images + j] * eigenvalues[pc].sqrt() / cov_jj
                        } else {
                            0.0
                        }
                    } else {
                        evec_flat[pc * num_images + j] * eigenvalues[pc].sqrt()
                    };
                }
            });

        let sorted_eigenvectors: Vec<Vec<f64>> = (0..num_images)
            .into_par_iter()
            .map(|a| {
                let pc = component_order[a];
                (0..num_images)
                    .map(|k| evec_flat[pc * num_images + k])
                    .collect()
            })
            .collect();

        let base_path = output_path.as_ref();
        let (base_stem, base_ext, base_parent) = base_path.map(|bp| {
            let stem = bp.file_stem().and_then(|s| s.to_str()).unwrap_or("pca").to_string();
            let ext = bp.extension().and_then(|s| s.to_str()).unwrap_or("tif").to_string();
            let parent = bp.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
            (stem, ext, parent)
        }).unwrap_or_else(|| ("pca".to_string(), "tif".to_string(), std::path::PathBuf::from(".")));

        let mut comp_locators: Vec<serde_json::Value> = Vec::new();
        for a in 0..num_comp {
            let pc = component_order[a];
            let mut comp_raster = Raster::new(RasterConfig {
                rows, cols, bands: 1,
                x_min: inputs[0].x_min, y_min: inputs[0].y_min,
                cell_size: inputs[0].cell_size_x, cell_size_y: Some(inputs[0].cell_size_y),
                nodata: inputs[0].nodata, data_type: DataType::F32,
                crs: inputs[0].crs.clone(), metadata: inputs[0].metadata.clone(),
                ..Default::default()
            });
            let comp_weights: Vec<f64> = (0..num_images)
                .map(|k| evec_flat[pc * num_images + k])
                .collect();
            let comp_values = weighted_sum_chunked(&inputs, &comp_weights, inputs[0].nodata, n);
            if let Some(data_slice) = comp_raster.data.as_f32_slice_mut() {
                data_slice
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(idx, cell)| *cell = comp_values[idx] as f32);
            } else {
                for (idx, val) in comp_values.into_iter().enumerate() {
                    comp_raster.data.set_f64(idx, val);
                }
            }
            let comp_path = output_path.as_ref().map(|_| base_parent.join(format!("{}_comp{}.{}", base_stem, a + 1, base_ext)));
            let loc = write_or_store_output(comp_raster, comp_path)?;
            comp_locators.push(typed_raster_output(loc));
        }

        let sorted_explained: Vec<f64> = (0..num_images)
            .into_par_iter()
            .map(|i| explained[component_order[i]])
            .collect();
        let sorted_eigenvalues: Vec<f64> = (0..num_images)
            .into_par_iter()
            .map(|i| eigenvalues[component_order[i]])
            .collect();
        let mut cum = 0.0f64;
        let cum_variances: Vec<f64> = sorted_explained.iter().map(|&v| { cum += v; cum }).collect();

        let report = json!({
            "num_images": num_images,
            "num_components": num_comp,
            "standardized": standardized,
            "explained_variances": sorted_explained,
            "cumulative_variances": cum_variances,
            "eigenvalues": sorted_eigenvalues,
            "eigenvectors": sorted_eigenvectors,
            "factor_loadings": factor_loadings,
        }).to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("outputs".to_string(), json!(comp_locators));
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// InversePcaTool
// ─────────────────────────────────────────────────────────────────────────────

impl Tool for InversePcaTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "inverse_pca",
            display_name: "Inverse PCA",
            summary: "Reconstructs original band images from PCA component rasters using stored eigenvectors.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "inputs", description: "Component raster paths (from PCA output, ordered).", required: true },
                ToolParamSpec { name: "auto_reproject", description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.", required: false },
                ToolParamSpec { name: "auto_reproject_method", description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.", required: false },
                ToolParamSpec { name: "pca_report", description: "JSON report string from the principal_component_analysis tool.", required: true },
                ToolParamSpec { name: "output", description: "Optional base output path; images named '{stem}_img1.{ext}' etc.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut example = ToolArgs::new();
        example.insert("inputs".to_string(), json!(["pca_comp1.tif", "pca_comp2.tif", "pca_comp3.tif"]));
        example.insert("pca_report".to_string(), json!("<JSON string from PCA tool>"));
        example.insert("output".to_string(), json!("inv.tif"));
        ToolManifest {
            id: "inverse_pca".to_string(),
            display_name: "Inverse PCA".to_string(),
            summary: "Reconstructs original band images from PCA component rasters using stored eigenvectors.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "inputs".to_string(), description: "Component raster paths.".to_string(), required: true },
                ToolParamDescriptor { name: "auto_reproject".to_string(), description: "If true (default), automatically reproject stack rasters to match inputs[0] when CRS differs.".to_string(), required: false },
                ToolParamDescriptor { name: "auto_reproject_method".to_string(), description: "Optional reprojection resampling method override: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev.".to_string(), required: false },
                ToolParamDescriptor { name: "pca_report".to_string(), description: "JSON report string from PCA.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional base output path.".to_string(), required: false },
            ],
            defaults: ToolArgs::new(),
            examples: vec![ToolExample { name: "basic".to_string(), description: "Reconstruct 3 images from PCA components.".to_string(), args: example }],
            tags: vec!["raster".to_string(), "statistics".to_string(), "pca".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let paths = parse_raster_list_arg(args, "inputs")?;
        if paths.len() < 2 {
            return Err(ToolError::Validation("'inputs' must contain at least 2 component rasters".to_string()));
        }
        if let Some(method) = args.get("auto_reproject_method").and_then(|v| v.as_str()) {
            let method = method.trim();
            if !method.is_empty() && parse_resample_method(method).is_none() {
                return Err(ToolError::Validation(
                    "parameter 'auto_reproject_method' must be one of: nearest, bilinear, cubic, lanczos, average, min, max, mode, median, stddev".to_string(),
                ));
            }
        }
        let _ = args.get("pca_report").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'pca_report' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let pca_report_str = args.get("pca_report").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'pca_report' is required".to_string()))?;
        let output_path = parse_optional_output_path(args, "output")?;
        let auto_reproject = args
            .get("auto_reproject")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let resample_override = args
            .get("auto_reproject_method")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        let report_val: serde_json::Value = serde_json::from_str(pca_report_str)
            .map_err(|e| ToolError::Validation(format!("invalid PCA report JSON: {e}")))?;
        let eigenvectors: Vec<Vec<f64>> = serde_json::from_value(
            report_val.get("eigenvectors").cloned()
                .ok_or_else(|| ToolError::Validation("PCA report missing 'eigenvectors' field".to_string()))?
        ).map_err(|e| ToolError::Validation(format!("failed parsing eigenvectors: {e}")))?;

        if eigenvectors.is_empty() {
            return Err(ToolError::Validation("eigenvectors array is empty".to_string()));
        }
        let num_images = eigenvectors[0].len();
        if num_images == 0 {
            return Err(ToolError::Validation("eigenvector length is 0".to_string()));
        }

        let mut inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
        let stack_config = RasterStackConfig {
            auto_reproject,
            resampling_method: resample_override,
            allow_no_overlap: false,
        };
        align_and_validate_raster_stack(&mut inputs, &stack_config)
            .map_err(ToolError::Validation)?;
        let num_comp = inputs.len();
        let rows = inputs[0].rows;
        let cols = inputs[0].cols;
        for (i, r) in inputs.iter().enumerate() {
            if r.rows != rows || r.cols != cols {
                return Err(ToolError::Validation(format!("inputs[{}] has different dimensions", i)));
            }
        }

        let base_path = output_path.as_ref();
        let (base_stem, base_ext, base_parent) = base_path.map(|bp| {
            let stem = bp.file_stem().and_then(|s| s.to_str()).unwrap_or("inv_pca").to_string();
            let ext = bp.extension().and_then(|s| s.to_str()).unwrap_or("tif").to_string();
            let parent = bp.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."));
            (stem, ext, parent)
        }).unwrap_or_else(|| ("inv_pca".to_string(), "tif".to_string(), std::path::PathBuf::from(".")));

        let n = rows * cols;
        let mut img_locators: Vec<serde_json::Value> = Vec::new();
        for image_num in 0..num_images {
            let mut out_raster = Raster::new(RasterConfig {
                rows, cols, bands: 1,
                x_min: inputs[0].x_min, y_min: inputs[0].y_min,
                cell_size: inputs[0].cell_size_x, cell_size_y: Some(inputs[0].cell_size_y),
                nodata: inputs[0].nodata, data_type: DataType::F32,
                crs: inputs[0].crs.clone(), metadata: inputs[0].metadata.clone(),
                ..Default::default()
            });
            let valid_comp = num_comp.min(eigenvectors.len());
            let comp_weights: Vec<f64> = (0..valid_comp)
                .map(|k| eigenvectors[k].get(image_num).copied().unwrap_or(0.0))
                .collect();
            let out_values = weighted_sum_chunked(&inputs[..valid_comp], &comp_weights, inputs[0].nodata, n);
            if let Some(data_slice) = out_raster.data.as_f32_slice_mut() {
                data_slice
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(idx, cell)| *cell = out_values[idx] as f32);
            } else {
                for (idx, val) in out_values.into_iter().enumerate() {
                    out_raster.data.set_f64(idx, val);
                }
            }
            let img_path = output_path.as_ref().map(|_| base_parent.join(format!("{}_img{}.{}", base_stem, image_num + 1, base_ext)));
            let loc = write_or_store_output(out_raster, img_path)?;
            img_locators.push(typed_raster_output(loc));
        }

        let mut outputs = BTreeMap::new();
        outputs.insert("outputs".to_string(), json!(img_locators));
        Ok(ToolRunResult { outputs })
    }
}
