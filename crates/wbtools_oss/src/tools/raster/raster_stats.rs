use std::collections::{BTreeMap, BTreeSet, HashMap};

use evalexpr::{build_operator_tree, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value as EvalValue};
use nalgebra::{DMatrix, DVector};
use kdtree::distance::squared_euclidean;
use kdtree::KdTree;
use rand::seq::SliceRandom;
use rand::RngExt;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::memory_store;

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
    F: Fn(f64, f64, f64, bool) -> Option<f64>,
{
    let input1_path = parse_raster_path_arg(args, "input1")?;
    let input2 = parse_raster_or_constant_arg(args, "input2")?;
    let mut in1 = load_raster(&input1_path, "input1")?;

    match input2 {
        RasterOrConstant::Constant(c) => {
            if tool_id == "inplace_divide" && c == 0.0 {
                return Err(ToolError::Validation("illegal division by zero".to_string()));
            }
            for i in 0..in1.data.len() {
                let a = in1.data.get_f64(i);
                if in1.is_nodata(a) {
                    continue;
                }
                let out = op(a, c, in1.nodata, false).unwrap_or(in1.nodata);
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
            for i in 0..in1.data.len() {
                let a = in1.data.get_f64(i);
                let b = in2.data.get_f64(i);
                if in1.is_nodata(a) {
                    continue;
                }
                if in2.is_nodata(b) {
                    in1.data.set_f64(i, in1.nodata);
                    continue;
                }
                let out = op(a, b, in1.nodata, true).unwrap_or(in1.nodata);
                in1.data.set_f64(i, out);
            }
        }
    }

    let locator = write_inplace_raster(in1, &input1_path)?;
    let mut outputs = BTreeMap::new();
    outputs.insert("output".to_string(), typed_raster_output(locator));
    Ok(ToolRunResult { outputs })
}

fn collect_numeric_field_values(layer: &wbvector::Layer, field_name: &str) -> Result<Vec<f64>, ToolError> {
    let idx = layer
        .schema
        .field_index(field_name)
        .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", field_name)))?;

    let field_type = layer.schema.fields()[idx].field_type;
    if !matches!(field_type, wbvector::FieldType::Integer | wbvector::FieldType::Float) {
        return Err(ToolError::Validation(format!(
            "field '{}' must be numeric",
            field_name
        )));
    }

    let mut values = Vec::<f64>::new();
    for feat in &layer.features {
        if let Some(v) = feat.attributes.get(idx).and_then(|v| v.as_f64()) {
            values.push(v);
        }
    }
    Ok(values)
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
    let mut values = Vec::<f64>::new();
    for i in 0..r.data.len() {
        let z = r.data.get_f64(i);
        if !r.is_nodata(z) {
            values.push(z);
        }
    }
    values
}

fn sample_with_replacement(values: &[f64], count: usize) -> Vec<f64> {
    let mut rng = rand::rng();
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let idx = rng.random_range(0..values.len());
        out.push(values[idx]);
    }
    out
}

fn collect_paired_differences(in1: &Raster, in2: &Raster) -> Vec<f64> {
    let mut diffs = Vec::<f64>::new();
    for i in 0..in1.data.len() {
        let a = in1.data.get_f64(i);
        let b = in2.data.get_f64(i);
        if in1.is_nodata(a) || in2.is_nodata(b) {
            continue;
        }
        diffs.push(b - a);
    }
    diffs
}

fn two_sample_ks_statistic(data1: &[f64], data2: &[f64]) -> (f64, f64) {
    let mut v1 = data1.to_vec();
    let mut v2 = data2.to_vec();
    v1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

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
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

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
    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;
    let mut dev_x = 0.0;
    let mut dev_y = 0.0;
    let mut dev_xy = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        dev_x += dx * dx;
        dev_y += dy * dy;
        dev_xy += dx * dy;
    }

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

fn kendall_tau_b_from_pairs(x: &[f64], y: &[f64]) -> Option<(f64, usize)> {
    if x.len() != y.len() || x.len() < 3 {
        return None;
    }

    let n = x.len();
    let mut concordant = 0.0f64;
    let mut discordant = 0.0f64;
    let mut ties_x = 0.0f64;
    let mut ties_y = 0.0f64;

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = x[i] - x[j];
            let dy = y[i] - y[j];
            if dx == 0.0 && dy == 0.0 {
                continue;
            }
            if dx == 0.0 {
                ties_x += 1.0;
                continue;
            }
            if dy == 0.0 {
                ties_y += 1.0;
                continue;
            }
            if dx.signum() == dy.signum() {
                concordant += 1.0;
            } else {
                discordant += 1.0;
            }
        }
    }

    let numer = concordant - discordant;
    let n0 = (n * (n - 1) / 2) as f64;
    let denom = ((n0 - ties_x) * (n0 - ties_y)).sqrt();
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

        let mut count = 0usize;
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let mut sum2 = 0.0;

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            count += 1;
            if z < min_val {
                min_val = z;
            }
            if z > max_val {
                max_val = z;
            }
            sum += z;
            sum2 += z * z;
        }

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
                    name: "bins",
                    description: "Number of histogram bins (default 256).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("bins".to_string(), json!(256));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("image.tif"));
        example.insert("bins".to_string(), json!(256));

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
                    name: "bins".to_string(),
                    description: "Number of histogram bins (default 256).".to_string(),
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let bins = args
            .get("bins")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(256)
            .max(2);
        let input = load_raster(&input_path, "input")?;

        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut values = Vec::<f64>::new();

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            if z < min_val {
                min_val = z;
            }
            if z > max_val {
                max_val = z;
            }
            values.push(z);
        }

        if values.is_empty() {
            return Err(ToolError::Validation(
                "input raster contains no valid cells".to_string(),
            ));
        }

        let range = (max_val - min_val).max(1e-12);
        let mut counts = vec![0usize; bins];
        for z in values {
            let idx = (((z - min_val) / range) * bins as f64).floor() as isize;
            let idx = idx.clamp(0, bins as isize - 1) as usize;
            counts[idx] += 1;
        }

        let report = json!({
            "min": min_val,
            "max": max_val,
            "bins": bins,
            "counts": counts,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for ListUniqueValuesRasterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "list_unique_values_raster",
            display_name: "List Unique Values (Raster)",
            summary: "Lists unique valid values in a raster (capped to protect memory).",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input raster path.",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_values",
                    description: "Maximum unique values to include in output (default 10000).",
                    required: false,
                },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("max_values".to_string(), json!(10000));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("classified.tif"));
        example.insert("max_values".to_string(), json!(5000));

        ToolManifest {
            id: "list_unique_values_raster".to_string(),
            display_name: "List Unique Values (Raster)".to_string(),
            summary: "Lists unique valid values in a raster (capped to protect memory).".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input raster path.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "max_values".to_string(),
                    description: "Maximum unique values to include in output (default 10000).".to_string(),
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
        let max_values = args
            .get("max_values")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10000)
            .max(1);
        let input = load_raster(&input_path, "input")?;

        let mut set = BTreeSet::<i64>::new();
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            set.insert(z.round() as i64);
            if set.len() >= max_values {
                break;
            }
        }

        let values: Vec<i64> = set.into_iter().collect();
        let report = json!({
            "count": values.len(),
            "values": values,
            "truncated": values.len() >= max_values,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
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

        let mut count = 0usize;
        let mut sum = 0.0;
        let mut sum2 = 0.0;
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            count += 1;
            sum += z;
            sum2 += z * z;
        }
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

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                output.data.set_f64(i, input.nodata);
            } else {
                output.data.set_f64(i, (z - mean) / stdev);
            }
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

        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            let zz = z.max(clip_min.unwrap_or(z)).min(clip_max.unwrap_or(z));
            if zz < min_val {
                min_val = zz;
            }
            if zz > max_val {
                max_val = zz;
            }
        }

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

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                output.data.set_f64(i, input.nodata);
            } else {
                let zz = z.max(clip_min.unwrap_or(z)).min(clip_max.unwrap_or(z));
                let out = out_min + (zz - min_val) * scale;
                output.data.set_f64(i, out);
            }
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

        let mut rng = rand::rng();
        for i in 0..output.data.len() {
            output.data.set_f64(i, sample_standard_normal(&mut rng));
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

        let mut valid_indices = Vec::<usize>::new();
        for i in 0..base.data.len() {
            let z = base.data.get_f64(i);
            if !base.is_nodata(z) {
                valid_indices.push(i);
            }
        }

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
        for i in 0..output.data.len() {
            output.data.set_f64(i, 0.0);
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

        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut num_cells = 0usize;
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            min_val = min_val.min(z);
            max_val = max_val.max(z);
            num_cells += 1;
        }

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
            for i in 0..input.data.len() {
                let z = input.data.get_f64(i);
                output.data.set_f64(i, if input.is_nodata(z) { input.nodata } else { 1.0 });
            }
        } else {
            let num_bins = 50_000usize;
            let bin_size = (max_val - min_val) / num_bins as f64;
            let mut histogram = vec![0usize; num_bins];
            for i in 0..input.data.len() {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    continue;
                }
                let idx = (((z - min_val) / bin_size) as isize).clamp(0, num_bins as isize - 1) as usize;
                histogram[idx] += 1;
            }

            let mut cdf = vec![0.0; num_bins];
            let mut running = 0.0;
            for (i, count) in histogram.iter().enumerate() {
                running += *count as f64;
                cdf[i] = running / num_cells as f64;
            }

            for i in 0..input.data.len() {
                let z = input.data.get_f64(i);
                if input.is_nodata(z) {
                    output.data.set_f64(i, input.nodata);
                } else {
                    let idx = (((z - min_val) / bin_size) as isize).clamp(0, num_bins as isize - 1) as usize;
                    output.data.set_f64(i, cdf[idx]);
                }
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
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("membership.tif"));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("membership.tif"));

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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let input = load_raster(&input_path, "input")?;

        let mut count = 0usize;
        let mut sum = 0.0;
        let mut ss_mp = 0.0;
        let mut warning = false;

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            if !(0.0..=1.0).contains(&z) {
                warning = true;
            }
            count += 1;
            sum += z;
        }

        if count == 0 {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        let mean = sum / count as f64;
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                continue;
            }
            ss_mp += (z - mean) * (z - mean);
        }

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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let requested_samples = args.get("num_samples").and_then(|v| v.as_u64()).map(|v| v as usize);
        let input = load_raster(&input_path, "input")?;

        let mut valid_values = Vec::<f64>::new();
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if !input.is_nodata(z) {
                valid_values.push(z);
            }
        }

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
            let mut histogram = vec![0usize; num_bins];
            for z in &values {
                let idx = (((*z - min_value) / bin_size).floor() as isize).clamp(0, num_bins as isize - 1) as usize;
                histogram[idx] += 1;
            }

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
        ToolManifest {
            id: "attribute_histogram".to_string(),
            display_name: "Attribute Histogram".to_string(),
            summary: "Creates a histogram for numeric field values in a vector attribute table.".to_string(),
            category: ToolCategory::Vector,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input vector path.".to_string(), required: true },
                ToolParamDescriptor { name: "field".to_string(), description: "Numeric attribute field name.".to_string(), required: true },
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'field' is required".to_string()))?;

        let layer = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading vector '{}': {}", input_path, e)))?;
        let values = collect_numeric_field_values(&layer, field)?;
        if values.is_empty() {
            return Err(ToolError::Validation("field contains no numeric values".to_string()));
        }

        let min = values.iter().copied().fold(f64::INFINITY, |a, b| a.min(b));
        let max = values.iter().copied().fold(f64::NEG_INFINITY, |a, b| a.max(b));
        let num_bins = (values.len() as f64).log2().ceil().max(1.0) as usize + 1;
        let width = (max - min + 1.0e-5) / num_bins as f64;
        let mut counts = vec![0usize; num_bins];
        for v in values {
            let idx = (((v - min) / width).floor() as isize).clamp(0, num_bins as isize - 1) as usize;
            counts[idx] += 1;
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let fieldx = args.get("fieldx").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldx' is required".to_string()))?;
        let fieldy = args.get("fieldy").and_then(|v| v.as_str()).ok_or_else(|| ToolError::Validation("parameter 'fieldy' is required".to_string()))?;
        let trendline = args.get("trendline").and_then(|v| v.as_bool()).unwrap_or(false);

        let layer = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading vector '{}': {}", input_path, e)))?;
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

        let n = xs.len() as f64;
        let mean_x = xs.iter().sum::<f64>() / n;
        let mean_y = ys.iter().sum::<f64>() / n;
        let mut sxx = 0.0;
        let mut syy = 0.0;
        let mut sxy = 0.0;
        for i in 0..xs.len() {
            let dx = xs[i] - mean_x;
            let dy = ys[i] - mean_y;
            sxx += dx * dx;
            syy += dy * dy;
            sxy += dx * dy;
        }
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
            "x_min": xs.iter().copied().fold(f64::INFINITY, |a, b| a.min(b)),
            "x_max": xs.iter().copied().fold(f64::NEG_INFINITY, |a, b| a.max(b)),
            "y_min": ys.iter().copied().fold(f64::INFINITY, |a, b| a.min(b)),
            "y_max": ys.iter().copied().fold(f64::NEG_INFINITY, |a, b| a.max(b)),
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
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
            }],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("data.shp"));
        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("data.shp"));
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_vector_path_arg(args, "input")?;
        let layer = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading vector '{}': {}", input_path, e)))?;

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
        for a in 0..k {
            for b in 0..a {
                let mut xs = Vec::new();
                let mut ys = Vec::new();
                for i in 0..columns[a].len() {
                    let x = columns[a][i];
                    let y = columns[b][i];
                    if x.is_finite() && y.is_finite() {
                        xs.push(x);
                        ys.push(y);
                    }
                }

                let corr = if xs.len() < 2 {
                    f64::NAN
                } else {
                    let n = xs.len() as f64;
                    let mx = xs.iter().sum::<f64>() / n;
                    let my = ys.iter().sum::<f64>() / n;
                    let mut sxx = 0.0;
                    let mut syy = 0.0;
                    let mut sxy = 0.0;
                    for i in 0..xs.len() {
                        let dx = xs[i] - mx;
                        let dy = ys[i] - my;
                        sxx += dx * dx;
                        syy += dy * dy;
                        sxy += dx * dy;
                    }
                    if sxx > 0.0 && syy > 0.0 {
                        sxy / (sxx * syy).sqrt()
                    } else {
                        f64::NAN
                    }
                };

                matrix[a][b] = corr;
                matrix[b][a] = corr;
            }
        }

        let report = json!({
            "input": input_path,
            "fields": field_names,
            "matrix": matrix,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
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
        ToolManifest {
            id: "cross_tabulation".to_string(),
            display_name: "Cross Tabulation".to_string(),
            summary: "Performs cross-tabulation on two categorical rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input raster 1 path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input raster 2 path.".to_string(), required: true },
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let mut row_classes = BTreeSet::<i64>::new();
        let mut col_classes = BTreeSet::<i64>::new();
        let mut counts = BTreeMap::<(i64, i64), usize>::new();

        for i in 0..in1.data.len() {
            let z1 = in1.data.get_f64(i);
            let z2 = in2.data.get_f64(i);
            if in1.is_nodata(z1) || in2.is_nodata(z2) {
                continue;
            }
            let c1 = z1.round() as i64;
            let c2 = z2.round() as i64;
            col_classes.insert(c1);
            row_classes.insert(c2);
            *counts.entry((c2, c1)).or_insert(0) += 1;
        }

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

        ToolManifest {
            id: "anova".to_string(),
            display_name: "ANOVA".to_string(),
            summary: "Performs one-way ANOVA on raster values grouped by class raster categories.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Measurement raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "features".to_string(), description: "Class/category raster path.".to_string(), required: true },
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = parse_raster_path_arg(args, "input")?;
        let feature_path = parse_raster_path_arg(args, "features")?;

        let input = load_raster(&input_path, "input")?;
        let features = load_raster(&feature_path, "features")?;
        if input.rows != features.rows || input.cols != features.cols || input.bands != features.bands {
            return Err(ToolError::Validation(
                "input and features rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        let mut class_stats = BTreeMap::<i64, (usize, f64, f64)>::new();
        let mut overall_n = 0usize;
        let mut overall_sum = 0.0f64;
        let mut overall_sum_sqr = 0.0f64;

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            let cls = features.data.get_f64(i);
            if input.is_nodata(z) || features.is_nodata(cls) {
                continue;
            }

            let class_id = cls.round() as i64;
            let entry = class_stats.entry(class_id).or_insert((0usize, 0.0, 0.0));
            entry.0 += 1;
            entry.1 += z;
            entry.2 += z * z;

            overall_n += 1;
            overall_sum += z;
            overall_sum_sqr += z * z;
        }

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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        // Binary contingency table entries:
        // a=true positive, b=false positive, c=false negative, d=true negative.
        let mut a = 0usize;
        let mut b = 0usize;
        let mut c = 0usize;
        let mut d = 0usize;

        for i in 0..in1.data.len() {
            let z1 = in1.data.get_f64(i);
            let z2 = in2.data.get_f64(i);
            if in1.is_nodata(z1) || in2.is_nodata(z2) {
                continue;
            }

            let p = z1 != 0.0;
            let q = z2 != 0.0;
            match (p, q) {
                (true, true) => a += 1,
                (true, false) => b += 1,
                (false, true) => c += 1,
                (false, false) => d += 1,
            }
        }

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

        let mut rasters = Vec::<Raster>::new();
        for p in &input_paths {
            rasters.push(load_raster(p, "inputs")?);
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
            let mut s = 0.0;
            let mut n = 0usize;
            for i in 0..r.data.len() {
                let z = r.data.get_f64(i);
                if !r.is_nodata(z) {
                    s += z;
                    n += 1;
                }
            }
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
                let mut dev_a = 0.0;
                let mut dev_b = 0.0;
                let mut dev_ab = 0.0;
                let mut n = 0usize;
                for i in 0..rasters[a].data.len() {
                    let z1 = rasters[a].data.get_f64(i);
                    let z2 = rasters[b].data.get_f64(i);
                    if rasters[a].is_nodata(z1) || rasters[b].is_nodata(z2) {
                        continue;
                    }
                    let d1 = z1 - means[a];
                    let d2 = z2 - means[b];
                    dev_a += d1 * d1;
                    dev_b += d2 * d2;
                    dev_ab += d1 * d2;
                    n += 1;
                }

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

        let mut rasters = Vec::<Raster>::new();
        for p in &input_paths {
            rasters.push(load_raster(p, "inputs")?);
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
            let mut sum = 0.0;
            let mut n = 0.0;
            for i in 0..r.data.len() {
                let z = r.data.get_f64(i);
                if !r.is_nodata(z) {
                    sum += z;
                    n += 1.0;
                }
            }

            if n <= 3.0 {
                per_image.push(json!({
                    "input": input_paths[idx],
                    "valid_count": n,
                    "error": "insufficient valid cells for autocorrelation",
                }));
                continue;
            }

            let mean = sum / n;
            let mut total_deviation = 0.0;
            let mut w = 0.0;
            let mut numerator = 0.0;
            let mut s2 = 0.0;
            let mut k = 0.0;

            for row in 0..rows as isize {
                for col in 0..cols as isize {
                    let z = r.get_raw(0, row, col).unwrap_or(r.nodata);
                    if r.is_nodata(z) {
                        continue;
                    }

                    let dz = z - mean;
                    total_deviation += dz * dz;
                    k += dz * dz * dz * dz;

                    let mut wij = 0.0;
                    for nidx in 0..dx.len() {
                        let x = col + dx[nidx];
                        let y = row + dy[nidx];
                        if x < 0 || x >= cols as isize || y < 0 || y >= rows as isize {
                            continue;
                        }
                        let zn = r.get_raw(0, y, x).unwrap_or(r.nodata);
                        if r.is_nodata(zn) {
                            continue;
                        }
                        w += 1.0;
                        numerator += dz * (zn - mean);
                        wij += 1.0;
                    }
                    s2 += wij * wij;
                }
            }

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

        for i in 0..out_corr.data.len() {
            out_corr.data.set_f64(i, in1.nodata);
            out_sig.data.set_f64(i, in1.nodata);
        }

        let half = (filter_size as isize) / 2;
        let mut offsets = Vec::<(isize, isize)>::with_capacity(filter_size * filter_size);
        for r in 0..filter_size as isize {
            for c in 0..filter_size as isize {
                offsets.push((r - half, c - half));
            }
        }

        for band_idx in 0..bands {
            let band = band_idx as isize;
            for row in 0..rows_i {
                for col in 0..cols_i {
                    let z1 = in1.get_raw(band, row, col).unwrap_or(in1.nodata);
                    let z2 = in2.get_raw(band, row, col).unwrap_or(in2.nodata);
                    if in1.is_nodata(z1) || in2.is_nodata(z2) {
                        continue;
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
                        continue;
                    }

                    let (corr, pval) = if stat == "kendall" {
                        if let Some((tau, n)) = kendall_tau_b_from_pairs(&a, &b) {
                            let nn = n as f64;
                            let z = if nn > 2.0 {
                                3.0 * tau * (nn * (nn - 1.0) / (2.0 * (2.0 * nn + 5.0))).sqrt()
                            } else {
                                0.0
                            };
                            (tau, two_tailed_normal_p(z))
                        } else {
                            continue;
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
                            continue;
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
                        continue;
                    };

                    let idx = band_idx * rows * cols + row as usize * cols + col as usize;
                    out_corr.data.set_f64(idx, corr);
                    out_sig.data.set_f64(idx, pval);
                }
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

        let mut n = 0.0f64;
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_xx = 0.0f64;
        let mut sum_yy = 0.0f64;

        for i in 0..in1.data.len() {
            let x = in1.data.get_f64(i);
            let y = in2.data.get_f64(i);
            if in1.is_nodata(x) || in2.is_nodata(y) {
                continue;
            }
            n += 1.0;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
            sum_yy += y * y;
        }

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

        let mut ss_error = 0.0f64;
        let mut ss_total = 0.0f64;
        for i in 0..in1.data.len() {
            let x = in1.data.get_f64(i);
            let y = in2.data.get_f64(i);
            if in1.is_nodata(x) || in2.is_nodata(y) {
                continue;
            }
            let yhat = slope * x + intercept;
            ss_error += (y - yhat) * (y - yhat);
            ss_total += (y - y_mean) * (y - y_mean);
        }

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

        for i in 0..residuals.data.len() {
            let x = in1.data.get_f64(i);
            let y = in2.data.get_f64(i);
            if in1.is_nodata(x) || in2.is_nodata(y) {
                residuals.data.set_f64(i, in1.nodata);
                continue;
            }
            let yhat = slope * x + intercept;
            let mut res = y - yhat;
            if standardize_residuals && se_of_estimate > 0.0 {
                res /= se_of_estimate;
            }
            residuals.data.set_f64(i, res);
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
            .iter()
            .map(|r| {
                if normalize {
                    let stats = r.statistics();
                    let rng = stats.max - stats.min;
                    (stats.min, if rng.abs() < 1.0e-15 { 1.0 } else { rng })
                } else if standardize {
                    let mut n = 0.0f64;
                    let mut sum = 0.0f64;
                    let mut sum_sq = 0.0f64;
                    for i in 0..r.data.len() {
                        let z = r.data.get_f64(i);
                        if !r.is_nodata(z) {
                            n += 1.0;
                            sum += z;
                            sum_sq += z * z;
                        }
                    }
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
        let mut points: Vec<Vec<f64>> = Vec::new();
        let mut pixel_map: Vec<usize> = Vec::new(); // point index -> flat pixel index

        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let flat_idx = row as usize * cols + col as usize;
                let mut feat = vec![0.0f64; num_features];
                let mut is_nodata = false;
                for (b, r) in rasters.iter().enumerate() {
                    let z = r.get(0, row, col);
                    if r.is_nodata(z) {
                        is_nodata = true;
                        break;
                    }
                    feat[b] = (z - band_scale[b].0) / band_scale[b].1;
                }
                if !is_nodata {
                    pixel_map.push(flat_idx);
                    points.push(feat);
                }
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
        ToolManifest {
            id: "kappa_index".to_string(),
            display_name: "Kappa Index".to_string(),
            summary: "Computes Cohen's kappa and agreement metrics between two categorical rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "Input classification raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Input reference raster path.".to_string(), required: true },
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
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input1_path = parse_raster_path_arg(args, "input1")?;
        let input2_path = parse_raster_path_arg(args, "input2")?;

        let in1 = load_raster(&input1_path, "input1")?;
        let in2 = load_raster(&input2_path, "input2")?;
        if in1.rows != in2.rows || in1.cols != in2.cols || in1.bands != in2.bands {
            return Err(ToolError::Validation("input rasters must have identical rows, columns, and bands".to_string()));
        }

        let mut classes = BTreeSet::<i64>::new();
        let mut counts = BTreeMap::<(i64, i64), usize>::new();
        for i in 0..in1.data.len() {
            let z1 = in1.data.get_f64(i);
            let z2 = in2.data.get_f64(i);
            if in1.is_nodata(z1) || in2.is_nodata(z2) {
                continue;
            }
            let c1 = z1.round() as i64;
            let c2 = z2.round() as i64;
            classes.insert(c1);
            classes.insert(c2);
            *counts.entry((c1, c2)).or_insert(0) += 1;
        }

        let classes: Vec<i64> = classes.into_iter().collect();
        if classes.is_empty() {
            return Err(ToolError::Validation("no overlapping valid categorical cells were found".to_string()));
        }

        let mut matrix = vec![vec![0usize; classes.len()]; classes.len()];
        let mut row_totals = vec![0usize; classes.len()];
        let mut col_totals = vec![0usize; classes.len()];
        let mut total = 0usize;
        let mut diag = 0usize;

        for (ri, rv) in classes.iter().enumerate() {
            for (ci, cv) in classes.iter().enumerate() {
                let n = *counts.get(&(*rv, *cv)).unwrap_or(&0);
                matrix[ri][ci] = n;
                row_totals[ri] += n;
                col_totals[ci] += n;
                total += n;
                if ri == ci {
                    diag += n;
                }
            }
        }

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

        let producers_accuracy: Vec<f64> = classes
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if col_totals[i] > 0 {
                    matrix[i][i] as f64 / col_totals[i] as f64
                } else {
                    f64::NAN
                }
            })
            .collect();

        let users_accuracy: Vec<f64> = classes
            .iter()
            .enumerate()
            .map(|(i, _)| {
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
            "classes": classes,
            "matrix": matrix,
            "overall_accuracy": diag as f64 / total as f64,
            "kappa_index": kappa,
            "producers_accuracy": producers_accuracy,
            "users_accuracy": users_accuracy,
        })
        .to_string();

        let mut outputs = BTreeMap::new();
        outputs.insert("report".to_string(), json!(report));
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
            id: "paired_sample_t_test".to_string(),
            display_name: "Paired Sample T Test".to_string(),
            summary: "Performs a paired-sample t-test on two rasters using paired valid cells.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input1".to_string(), description: "First input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "input2".to_string(), description: "Second input raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "num_samples".to_string(), description: "Optional sample size with replacement.".to_string(), required: false },
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

        let n = diffs.len();
        let n_f = n as f64;
        let mean = diffs.iter().sum::<f64>() / n_f;
        let variance = diffs.iter().map(|d| {
            let v = *d - mean;
            v * v
        }).sum::<f64>() / n_f;
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

        let mut signed_abs = Vec::<(f64, f64)>::new();
        for d in diffs {
            if d == 0.0 {
                continue;
            }
            signed_abs.push((d.signum(), d.abs()));
        }

        if signed_abs.len() < 2 {
            return Err(ToolError::Validation("insufficient non-zero differences for Wilcoxon test".to_string()));
        }

        signed_abs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

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
                for i in 0..out.data.len() {
                    let a = r1.data.get_f64(i);
                    let b = r2.data.get_f64(i);
                    let z = if r1.is_nodata(a) || r2.is_nodata(b) { out.nodata } else { a.max(b) };
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
                for i in 0..out.data.len() {
                    let a = r.data.get_f64(i);
                    let z = if r.is_nodata(a) { out.nodata } else { a.max(c) };
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
                for i in 0..out.data.len() {
                    let a = r1.data.get_f64(i);
                    let b = r2.data.get_f64(i);
                    let z = if r1.is_nodata(a) || r2.is_nodata(b) { out.nodata } else { a.min(b) };
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
                for i in 0..out.data.len() {
                    let a = r.data.get_f64(i);
                    let z = if r.is_nodata(a) { out.nodata } else { a.min(c) };
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

        let mut vals = Vec::<f64>::new();
        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if !input.is_nodata(z) {
                vals.push(z);
            }
        }
        if vals.is_empty() {
            return Err(ToolError::Validation("input raster contains no valid cells".to_string()));
        }

        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut cuts = Vec::<f64>::new();
        for q in 1..num_quantiles {
            let idx = ((q as f64 / num_quantiles as f64) * (vals.len() - 1) as f64).round() as usize;
            cuts.push(vals[idx]);
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

        for i in 0..input.data.len() {
            let z = input.data.get_f64(i);
            if input.is_nodata(z) {
                out.data.set_f64(i, input.nodata);
                continue;
            }
            let mut klass = 1usize;
            for cut in &cuts {
                if z > *cut {
                    klass += 1;
                } else {
                    break;
                }
            }
            out.data.set_f64(i, klass as f64);
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

        let layer = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading vector '{}': {}", input_path, e)))?;
        let idx = layer
            .schema
            .field_index(field)
            .ok_or_else(|| ToolError::Validation(format!("field '{}' not found", field)))?;

        let mut freq = BTreeMap::<String, usize>::new();
        for f in &layer.features {
            let key = f
                .attributes
                .get(idx)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());
            *freq.entry(key).or_insert(0) += 1;
        }

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

        let mut diffs = Vec::<f64>::new();
        let same_grid = input.rows == base.rows && input.cols == base.cols;

        if same_grid {
            for i in 0..input.data.len() {
                let z1 = input.data.get_f64(i);
                let z2 = base.data.get_f64(i);
                if input.is_nodata(z1) || base.is_nodata(z2) {
                    continue;
                }
                diffs.push(z2 - z1);
            }
        } else {
            for row in 0..input.rows as isize {
                for col in 0..input.cols as isize {
                    let z1 = input.get(0, row, col);
                    if input.is_nodata(z1) {
                        continue;
                    }
                    let x = input.col_center_x(col);
                    let y = input.row_center_y(row);
                    if let Some((bcol, brow)) = base.world_to_pixel(x, y) {
                        let z2 = base.get(0, brow, bcol);
                        if base.is_nodata(z2) {
                            continue;
                        }
                        diffs.push(z2 - z1);
                    }
                }
            }
        }

        if diffs.is_empty() {
            return Err(ToolError::Validation("no overlapping valid cells found for comparison".to_string()));
        }

        let n = diffs.len() as f64;
        let sum: f64 = diffs.iter().sum();
        let sq_sum: f64 = diffs.iter().map(|d| d * d).sum();
        let mean_vertical_error = sum / n;
        let rmse = (sq_sum / n).sqrt();

        let mut abs_residuals: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
        abs_residuals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
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
            "resampling": if same_grid { "none" } else { "nearest" },
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
        let mut zone_data: HashMap<i64, Vec<f64>> = HashMap::new();
        let mut zone_set: HashMap<i64, std::collections::HashSet<i64>> = HashMap::new();
        for i in 0..n {
            let z_val = features.data.get_f64(i);
            if features.is_nodata(z_val) { continue; }
            let zone_id = z_val.round() as i64;
            if zero_is_background && zone_id == 0 { continue; }
            let data_val = input.data.get_f64(i);
            if input.is_nodata(data_val) { continue; }
            zone_data.entry(zone_id).or_default().push(data_val);
            zone_set.entry(zone_id).or_default().insert((data_val * 1000.0).round() as i64);
        }

        let mut zone_stat: HashMap<i64, f64> = HashMap::new();
        for (&id, data) in &mut zone_data {
            let stat_val = match stat_type {
                "min" => data.iter().cloned().fold(f64::INFINITY, f64::min),
                "max" => data.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                "range" => data.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - data.iter().cloned().fold(f64::INFINITY, f64::min),
                "total" => data.iter().sum(),
                "diversity" => zone_set.get(&id).map(|s| s.len()).unwrap_or(0) as f64,
                "standard deviation" => {
                    let cnt = data.len() as f64;
                    if cnt < 2.0 { 0.0 } else {
                        let mean = data.iter().sum::<f64>() / cnt;
                        let var = data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (cnt - 1.0);
                        var.sqrt()
                    }
                }
                "median" => {
                    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let mid = data.len() / 2;
                    if data.len() % 2 == 0 { (data[mid - 1] + data[mid]) / 2.0 } else { data[mid] }
                }
                _ => data.iter().sum::<f64>() / data.len() as f64,
            };
            zone_stat.insert(id, stat_val);
        }

        let mut output = Raster::new(RasterConfig {
            rows: input.rows, cols: input.cols, bands: 1,
            x_min: input.x_min, y_min: input.y_min,
            cell_size: input.cell_size_x, cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata, data_type: DataType::F32,
            crs: input.crs.clone(), metadata: input.metadata.clone(),
            ..Default::default()
        });
        for i in 0..n { output.data.set_f64(i, input.nodata); }
        for i in 0..n {
            let z_val = features.data.get_f64(i);
            if features.is_nodata(z_val) { continue; }
            let zone_id = z_val.round() as i64;
            if zero_is_background && zone_id == 0 { continue; }
            if let Some(&sv) = zone_stat.get(&zone_id) {
                output.data.set_f64(i, sv);
            }
        }

        let loc = write_or_store_output(output, output_path)?;
        let mut sorted_ids: Vec<i64> = zone_stat.keys().copied().collect();
        sorted_ids.sort_unstable();
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

        let mut accum = vec![0.0f64; rows * cols];
        let mut rng = rand::rng();

        for _ in 0..iterations {
            let mut t = vec![0.0f64; diagonal_size + 2 * filter_half_size];
            for j in 0..diagonal_size { t[j] = sample_standard_normal(&mut rng); }

            let mut y = vec![0.0f64; diagonal_size];
            let mut sum = 0.0f64;
            let mut sq_sum = 0.0f64;
            for j in 0..diagonal_size {
                let mut z = 0.0f64;
                for k in 0..filter_size {
                    let m = cell_offsets[k];
                    z += m as f64 * t[(j as isize + filter_half_size as isize + m) as usize];
                }
                y[j] = w * z;
                sum += y[j];
                sq_sum += y[j] * y[j];
            }
            let mean = sum / diagonal_size as f64;
            let variance = (sq_sum / diagonal_size as f64 - mean * mean).max(0.0);
            let stdev = variance.sqrt();
            if stdev > 1.0e-15 { for j in 0..diagonal_size { y[j] = (y[j] - mean) / stdev; } }

            // Pick two edge points on different sides of the grid
            let edge1 = rng.random_range(0..4usize);
            let mut edge2 = edge1;
            while edge2 == edge1 { edge2 = rng.random_range(0..4usize); }

            let (p1x, p1y) = match edge1 {
                0 => (0.0f64, rng.random_range(0..rows) as f64),
                1 => (rng.random_range(0..cols) as f64, 0.0f64),
                2 => ((cols as f64 - 1.0), rng.random_range(0..rows) as f64),
                _ => (rng.random_range(0..cols) as f64, (rows as f64 - 1.0)),
            };
            let (mut p2x, mut p2y) = match edge2 {
                0 => (0.0f64, rng.random_range(0..rows) as f64),
                1 => (rng.random_range(0..cols) as f64, 0.0f64),
                2 => ((cols as f64 - 1.0), rng.random_range(0..rows) as f64),
                _ => (rng.random_range(0..cols) as f64, (rows as f64 - 1.0)),
            };
            let mut attempts = 0usize;
            while p1x == p2x || p1y == p2y {
                p2x = match edge2 { 0 | 2 => if edge2 == 0 { 0.0 } else { cols as f64 - 1.0 }, _ => rng.random_range(0..cols) as f64 };
                p2y = match edge2 { 1 | 3 => if edge2 == 1 { 0.0 } else { rows as f64 - 1.0 }, _ => rng.random_range(0..rows) as f64 };
                attempts += 1;
                if attempts > 50 { break; }
            }
            if p1x == p2x || p1y == p2y { continue; }

            let slope = (p2y - p1y) / (p2x - p1x);
            let intercept = p1y - slope * p1x;
            let perp = -1.0 / slope;
            let slope_diff = slope - perp;

            // Find starting corner (min y-projection)
            let grid_corners = [(0.0f64, 0.0f64), (0.0, cols as f64), (rows as f64, 0.0), (rows as f64, cols as f64)];
            let (lsx, lsy) = grid_corners.iter().map(|&(r, c)| {
                let b = r - perp * c;
                let cx = (b - intercept) / slope_diff;
                let cy = slope * cx - intercept;
                (cx, cy)
            }).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).unwrap();

            for row in 0..rows {
                for col in 0..cols {
                    let b = row as f64 - perp * col as f64;
                    let ix = (b - intercept) / slope_diff;
                    let iy = slope * ix - intercept;
                    let p = (((ix - lsx).powi(2) + (iy - lsy).powi(2)).sqrt() as isize)
                        .clamp(0, (diagonal_size - 1) as isize) as usize;
                    accum[row * cols + col] += y[p];
                }
            }
        }

        let iter_sqrt = (iterations as f64).sqrt();
        let mut output = Raster::new(RasterConfig {
            rows, cols, bands: 1,
            x_min: input.x_min, y_min: input.y_min,
            cell_size: input.cell_size_x, cell_size_y: Some(input.cell_size_y),
            nodata: input.nodata, data_type: DataType::F32,
            crs: input.crs.clone(), metadata: input.metadata.clone(),
            ..Default::default()
        });
        for i in 0..rows * cols { output.data.set_f64(i, accum[i] / iter_sqrt); }

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
    for i in 0..n {
        let mut m = 0;
        for j in 0..=order {
            for k in 0..=(order - j) {
                design[i * num_coeff + m] = x[i].powf(j as f64) * y[i].powf(k as f64);
                m += 1;
            }
        }
    }
    let mat = DMatrix::from_row_slice(n, num_coeff, &design);
    let qr = mat.clone().qr();
    let r = qr.r();
    if !r.is_invertible() {
        return Err(ToolError::Execution("polynomial regression matrix is not invertible".to_string()));
    }
    let b = DVector::from_row_slice(z);
    let coeffs = (r.try_inverse().unwrap() * qr.q().transpose() * b).as_slice().to_vec();

    let mut ss_resid = 0.0;
    let mut z_sum = 0.0;
    let mut z_ss = 0.0;
    for i in 0..n {
        let mut y_hat = 0.0;
        for j in 0..num_coeff { y_hat += design[i * num_coeff + j] * coeffs[j]; }
        let resid = z[i] - y_hat;
        ss_resid += resid * resid;
        z_sum += z[i];
        z_ss += z[i] * z[i];
    }
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

        let mut x_data = Vec::new();
        let mut y_data = Vec::new();
        let mut z_data = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let z = input.data.get_f64(row * cols + col);
                if !input.is_nodata(z) {
                    x_data.push(input.col_center_x(col as isize) - min_x);
                    y_data.push(input.row_center_y(row as isize) - min_y);
                    z_data.push(z - min_z);
                }
            }
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
        for row in 0..rows {
            for col in 0..cols {
                let x_val = input.col_center_x(col as isize) - min_x;
                let y_val = input.row_center_y(row as isize) - min_y;
                output.data.set_f64(row * cols + col, eval_poly(x_val, y_val, &coeffs, order, min_z));
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

        let layer = wbvector::read(&input_path)
            .map_err(|e| ToolError::Execution(format!("failed reading vector file: {e}")))?;

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
        for i in 0..x_pts.len() {
            x_pts[i] -= x_offset;
            y_pts[i] -= y_offset;
            z_pts[i] -= z_offset;
        }

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
        for row in 0..out_rows {
            for col in 0..out_cols {
                let x_val = output.col_center_x(col as isize) - x_offset;
                let y_val = output.row_center_y(row as isize) - y_offset;
                output.data.set_f64(row * out_cols + col, eval_poly(x_val, y_val, &coeffs, order, z_offset));
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
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let expression = args.get("expression").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'expression' is required".to_string()))?.to_string();
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let output_path = parse_optional_output_path(args, "output")?;

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

        let inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
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

        let mut context = HashMapContext::new();
        let _ = context.set_value("rows".to_string(), EvalValue::Float(rows as f64));
        let _ = context.set_value("columns".to_string(), EvalValue::Float(cols as f64));
        let _ = context.set_value("north".to_string(), EvalValue::Float(north));
        let _ = context.set_value("south".to_string(), EvalValue::Float(south));
        let _ = context.set_value("east".to_string(), EvalValue::Float(east));
        let _ = context.set_value("west".to_string(), EvalValue::Float(west));
        let _ = context.set_value("cellsizex".to_string(), EvalValue::Float(inputs[0].cell_size_x));
        let _ = context.set_value("cellsizey".to_string(), EvalValue::Float(inputs[0].cell_size_y));
        let _ = context.set_value("cellsize".to_string(), EvalValue::Float(0.5 * (inputs[0].cell_size_x + inputs[0].cell_size_y)));
        let _ = context.set_value("nodata".to_string(), EvalValue::Float(nodatas[0]));
        let _ = context.set_value("null".to_string(), EvalValue::Float(nodatas[0]));
        let _ = context.set_value("minvalue".to_string(), EvalValue::Float(stats[0].min));
        let _ = context.set_value("maxvalue".to_string(), EvalValue::Float(stats[0].max));
        let _ = context.set_value("pi".to_string(), EvalValue::Float(std::f64::consts::PI));
        let _ = context.set_value("e".to_string(), EvalValue::Float(std::f64::consts::E));

        let mut output = Raster::new(RasterConfig {
            rows, cols, bands: 1,
            x_min: inputs[0].x_min, y_min: inputs[0].y_min,
            cell_size: inputs[0].cell_size_x, cell_size_y: Some(inputs[0].cell_size_y),
            nodata: out_nodata, data_type: DataType::F32,
            crs: inputs[0].crs.clone(), metadata: inputs[0].metadata.clone(),
            ..Default::default()
        });

        for row in 0..rows {
            let _ = context.set_value("row".to_string(), EvalValue::Float(row as f64));
            let _ = context.set_value("rowy".to_string(), EvalValue::Float(inputs[0].row_center_y(row as isize)));
            for col in 0..cols {
                let idx = row * cols + col;
                let _ = context.set_value("column".to_string(), EvalValue::Float(col as f64));
                let _ = context.set_value("columnx".to_string(), EvalValue::Float(inputs[0].col_center_x(col as isize)));
                let mut any_nodata = false;
                for (i, inp) in inputs.iter().enumerate() {
                    let v = inp.data.get_f64(idx);
                    if inp.is_nodata(v) { any_nodata = true; }
                    let _ = context.set_value(format!("value{}", i), EvalValue::Float(v));
                }
                if any_nodata && !statement_contains_nodata {
                    output.data.set_f64(idx, out_nodata);
                    continue;
                }
                let out_val = match expr_tree.eval_with_context(&context) {
                    Ok(EvalValue::Float(v)) => v,
                    Ok(EvalValue::Int(v)) => v as f64,
                    Ok(EvalValue::Boolean(b)) => if b { 1.0 } else { 0.0 },
                    _ => out_nodata,
                };
                output.data.set_f64(idx, out_val);
            }
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
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, _ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_paths = parse_raster_list_arg(args, "inputs")?;
        let standardized = args.get("standardized").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        let inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
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

        let mut averages = vec![0.0f64; num_images];
        let mut num_cells = vec![0.0f64; num_images];
        for i in 0..num_images {
            let st = inputs[i].statistics();
            averages[i] = st.mean;
            num_cells[i] = st.valid_count as f64;
        }

        let n = rows * cols;
        let mut total_dev = vec![0.0f64; num_images];
        let mut covariances = vec![vec![0.0f64; num_images]; num_images];
        for idx in 0..n {
            for i in 0..num_images {
                let z1 = inputs[i].data.get_f64(idx);
                if inputs[i].is_nodata(z1) { continue; }
                total_dev[i] += (z1 - averages[i]).powi(2);
                for a in 0..num_images {
                    let z2 = inputs[a].data.get_f64(idx);
                    if !inputs[a].is_nodata(z2) {
                        covariances[i][a] += (z1 - averages[i]) * (z2 - averages[a]);
                    }
                }
            }
        }

        let mut corr = vec![vec![0.0f64; num_images]; num_images];
        for i in 0..num_images {
            for a in 0..num_images {
                let denom = (total_dev[i] * total_dev[a]).sqrt();
                corr[i][a] = if denom.abs() < 1.0e-15 { 0.0 } else { covariances[i][a] / denom };
                covariances[i][a] /= (num_cells[i] - 1.0).max(1.0);
            }
        }

        let matrix = if standardized { &corr } else { &covariances };
        let flat: Vec<f64> = matrix.iter().flat_map(|row| row.iter().copied()).collect();
        let cov_mat = DMatrix::from_row_slice(num_images, num_images, &flat);
        let eig = cov_mat.symmetric_eigen();
        let eigenvalues = eig.eigenvalues.as_slice().to_vec();
        let evec_flat = eig.eigenvectors.as_slice().to_vec(); // column-major: col pc = eigenvector pc

        let total_ev: f64 = eigenvalues.iter().map(|v| v.abs()).sum::<f64>().max(1.0e-15);
        let explained: Vec<f64> = eigenvalues.iter().map(|&e| 100.0 * e.abs() / total_ev).collect();

        // Sort by descending explained variance
        let mut component_order = vec![0usize; num_images];
        {
            let mut used = vec![false; num_images];
            for i in 0..num_images {
                let (k, _) = explained.iter().enumerate()
                    .filter(|(j, _)| !used[*j])
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or((0, &0.0));
                component_order[i] = k;
                used[k] = true;
            }
        }

        // Factor loadings
        let mut factor_loadings = vec![vec![0.0f64; num_images]; num_images];
        for j in 0..num_images {
            for k in 0..num_images {
                let pc = component_order[k];
                factor_loadings[j][k] = if !standardized {
                    let cov_jj = covariances[j][j].abs().sqrt();
                    if cov_jj > 1.0e-15 { evec_flat[pc * num_images + j] * eigenvalues[pc].abs().sqrt() / cov_jj } else { 0.0 }
                } else {
                    evec_flat[pc * num_images + j] * eigenvalues[pc].abs().sqrt()
                };
            }
        }

        let sorted_eigenvectors: Vec<Vec<f64>> = (0..num_images).map(|a| {
            let pc = component_order[a];
            (0..num_images).map(|k| evec_flat[pc * num_images + k]).collect()
        }).collect();

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
            for idx in 0..n {
                let v0 = inputs[0].data.get_f64(idx);
                if inputs[0].is_nodata(v0) {
                    comp_raster.data.set_f64(idx, inputs[0].nodata);
                    continue;
                }
                let z: f64 = (0..num_images).map(|k| inputs[k].data.get_f64(idx) * evec_flat[pc * num_images + k]).sum();
                comp_raster.data.set_f64(idx, z);
            }
            let comp_path = output_path.as_ref().map(|_| base_parent.join(format!("{}_comp{}.{}", base_stem, a + 1, base_ext)));
            let loc = write_or_store_output(comp_raster, comp_path)?;
            comp_locators.push(typed_raster_output(loc));
        }

        let sorted_explained: Vec<f64> = (0..num_images).map(|i| explained[component_order[i]]).collect();
        let sorted_eigenvalues: Vec<f64> = (0..num_images).map(|i| eigenvalues[component_order[i]]).collect();
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

        let inputs: Vec<Raster> = input_paths.iter().enumerate()
            .map(|(i, p)| load_raster(p, &format!("inputs[{}]", i)))
            .collect::<Result<_, _>>()?;
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
            for idx in 0..n {
                let v0 = inputs[0].data.get_f64(idx);
                if inputs[0].is_nodata(v0) {
                    out_raster.data.set_f64(idx, inputs[0].nodata);
                    continue;
                }
                let z: f64 = (0..num_comp.min(eigenvectors.len())).map(|k| {
                    if image_num < eigenvectors[k].len() {
                        inputs[k].data.get_f64(idx) * eigenvectors[k][image_num]
                    } else { 0.0 }
                }).sum();
                out_raster.data.set_f64(idx, z);
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
