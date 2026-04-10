use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::json;
use wbcore::{
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, PercentCoalescer, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};
use crate::memory_store;

pub struct RasterAddTool;
pub struct RasterAtan2Tool;
pub struct RasterBoolAndTool;
pub struct RasterBoolOrTool;
pub struct RasterBoolXorTool;
pub struct RasterSubtractTool;
pub struct RasterMultiplyTool;
pub struct RasterDivideTool;
pub struct RasterEqualToTool;
pub struct RasterGreaterThanTool;
pub struct RasterIntegerDivisionTool;
pub struct RasterLessThanTool;
pub struct RasterModuloTool;
pub struct RasterNotEqualToTool;
pub struct RasterPowerTool;

#[derive(Clone, Copy)]
enum BinaryMathOp {
    Add,
    Atan2,
    BoolAnd,
    BoolOr,
    BoolXor,
    Subtract,
    Multiply,
    Divide,
    EqualTo,
    GreaterThan,
    IntegerDivision,
    LessThan,
    Modulo,
    NotEqualTo,
    Power,
}

impl BinaryMathOp {
    fn id(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Atan2 => "atan2",
            Self::BoolAnd => "bool_and",
            Self::BoolOr => "bool_or",
            Self::BoolXor => "bool_xor",
            Self::Subtract => "subtract",
            Self::Multiply => "multiply",
            Self::Divide => "divide",
            Self::EqualTo => "equal_to",
            Self::GreaterThan => "greater_than",
            Self::IntegerDivision => "integer_division",
            Self::LessThan => "less_than",
            Self::Modulo => "modulo",
            Self::NotEqualTo => "not_equal_to",
            Self::Power => "power",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Atan2 => "Atan2",
            Self::BoolAnd => "BoolAnd",
            Self::BoolOr => "BoolOr",
            Self::BoolXor => "BoolXor",
            Self::Subtract => "Subtract",
            Self::Multiply => "Multiply",
            Self::Divide => "Divide",
            Self::EqualTo => "EqualTo",
            Self::GreaterThan => "GreaterThan",
            Self::IntegerDivision => "IntegerDivision",
            Self::LessThan => "LessThan",
            Self::Modulo => "Modulo",
            Self::NotEqualTo => "NotEqualTo",
            Self::Power => "Power",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Add => "Adds two rasters on a cell-by-cell basis.",
            Self::Atan2 => "Computes the four-quadrant inverse tangent using two rasters on a cell-by-cell basis.",
            Self::BoolAnd => "Computes a logical AND of two rasters on a cell-by-cell basis.",
            Self::BoolOr => "Computes a logical OR of two rasters on a cell-by-cell basis.",
            Self::BoolXor => "Computes a logical XOR of two rasters on a cell-by-cell basis.",
            Self::Subtract => "Subtracts the second raster from the first on a cell-by-cell basis.",
            Self::Multiply => "Multiplies two rasters on a cell-by-cell basis.",
            Self::Divide => "Divides the first raster by the second on a cell-by-cell basis.",
            Self::EqualTo => "Tests whether two rasters are equal on a cell-by-cell basis.",
            Self::GreaterThan => "Tests whether the first raster is greater than the second on a cell-by-cell basis.",
            Self::IntegerDivision => "Divides two rasters and truncates each result toward zero.",
            Self::LessThan => "Tests whether the first raster is less than the second on a cell-by-cell basis.",
            Self::Modulo => "Computes the remainder of dividing the first raster by the second on a cell-by-cell basis.",
            Self::NotEqualTo => "Tests whether two rasters are not equal on a cell-by-cell basis.",
            Self::Power => "Raises the first raster to the power of the second on a cell-by-cell basis.",
        }
    }

    fn run_message(self) -> &'static str {
        match self {
            Self::Add => "running add",
            Self::Atan2 => "running atan2",
            Self::BoolAnd => "running bool_and",
            Self::BoolOr => "running bool_or",
            Self::BoolXor => "running bool_xor",
            Self::Subtract => "running subtract",
            Self::Multiply => "running multiply",
            Self::Divide => "running divide",
            Self::EqualTo => "running equal_to",
            Self::GreaterThan => "running greater_than",
            Self::IntegerDivision => "running integer_division",
            Self::LessThan => "running less_than",
            Self::Modulo => "running modulo",
            Self::NotEqualTo => "running not_equal_to",
            Self::Power => "running power",
        }
    }

    fn processing_message(self) -> &'static str {
        match self {
            Self::Add => "adding raster cells",
            Self::Atan2 => "computing atan2 across raster cells",
            Self::BoolAnd => "computing bool_and across raster cells",
            Self::BoolOr => "computing bool_or across raster cells",
            Self::BoolXor => "computing bool_xor across raster cells",
            Self::Subtract => "subtracting raster cells",
            Self::Multiply => "multiplying raster cells",
            Self::Divide => "dividing raster cells",
            Self::EqualTo => "computing equal_to across raster cells",
            Self::GreaterThan => "computing greater_than across raster cells",
            Self::IntegerDivision => "computing integer_division across raster cells",
            Self::LessThan => "computing less_than across raster cells",
            Self::Modulo => "computing modulo across raster cells",
            Self::NotEqualTo => "computing not_equal_to across raster cells",
            Self::Power => "raising raster cells to powers",
        }
    }

    fn default_output_name(self) -> &'static str {
        match self {
            Self::Add => "dem_sum.tif",
            Self::Atan2 => "dem_atan2.tif",
            Self::BoolAnd => "dem_bool_and.tif",
            Self::BoolOr => "dem_bool_or.tif",
            Self::BoolXor => "dem_bool_xor.tif",
            Self::Subtract => "dem_difference.tif",
            Self::Multiply => "dem_product.tif",
            Self::Divide => "dem_ratio.tif",
            Self::EqualTo => "dem_equal_to.tif",
            Self::GreaterThan => "dem_greater_than.tif",
            Self::IntegerDivision => "dem_integer_division.tif",
            Self::LessThan => "dem_less_than.tif",
            Self::Modulo => "dem_modulo.tif",
            Self::NotEqualTo => "dem_not_equal_to.tif",
            Self::Power => "dem_power.tif",
        }
    }

    fn tag(self) -> &'static str {
        self.id()
    }

    fn apply(self, z1: f64, z2: f64, nodata: f64) -> f64 {
        match self {
            Self::Add => z1 + z2,
            Self::Atan2 => z1.atan2(z2),
            Self::BoolAnd => {
                if z1 != 0.0 && z2 != 0.0 { 1.0 } else { 0.0 }
            }
            Self::BoolOr => {
                if z1 != 0.0 || z2 != 0.0 { 1.0 } else { 0.0 }
            }
            Self::BoolXor => {
                if (z1 != 0.0) ^ (z2 != 0.0) { 1.0 } else { 0.0 }
            }
            Self::Subtract => z1 - z2,
            Self::Multiply => z1 * z2,
            Self::Divide => {
                if z2 == 0.0 {
                    nodata
                } else {
                    z1 / z2
                }
            }
            Self::EqualTo => {
                if z1 == z2 { 1.0 } else { 0.0 }
            }
            Self::GreaterThan => {
                if z1 > z2 { 1.0 } else { 0.0 }
            }
            Self::IntegerDivision => {
                if z2 == 0.0 { nodata } else { (z1 / z2).trunc() }
            }
            Self::LessThan => {
                if z1 < z2 { 1.0 } else { 0.0 }
            }
            Self::Modulo => {
                if z2 == 0.0 { nodata } else { z1 % z2 }
            }
            Self::NotEqualTo => {
                if z1 != z2 { 1.0 } else { 0.0 }
            }
            Self::Power => z1.powf(z2),
        }
    }
}

const BINARY_MATH_PAR_CHUNK: usize = 16_384;
const BINARY_MATH_PROGRESS_BATCH_CHUNKS: usize = 64;

impl RasterAddTool {
    fn parse_input_paths(args: &ToolArgs) -> Result<(String, String), ToolError> {
        let input1 = parse_raster_path_arg(args, "input1")?;
        let input2 = parse_raster_path_arg(args, "input2")?;
        Ok((input1, input2))
    }

    fn load_raster_from_arg(path: &str, param_name: &str) -> Result<Raster, ToolError> {
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
                    param_name,
                    id
                ))
            });
        }

        Raster::read(path).map_err(|e| {
            ToolError::Execution(format!("failed reading {} raster: {}", param_name, e))
        })
    }

    fn metadata_for(op: BinaryMathOp) -> ToolMetadata {
        ToolMetadata {
            id: op.id(),
            display_name: op.display_name(),
            summary: op.summary(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input1",
                    description: "First input raster (path string or typed raster object).",
                    required: true,
                },
                ToolParamSpec {
                    name: "input2",
                    description: "Second input raster (path string or typed raster object).",
                    required: true,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.",
                    required: false,
                },
            ],
        }
    }

    fn manifest_for(op: BinaryMathOp) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input1".to_string(), json!("input1.tif"));
        defaults.insert("input2".to_string(), json!("input2.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("input1".to_string(), json!("dem_a.tif"));
        example_args.insert("input2".to_string(), json!("dem_b.tif"));
        example_args.insert("output".to_string(), json!(op.default_output_name()));

        ToolManifest {
            id: op.id().to_string(),
            display_name: op.display_name().to_string(),
            summary: op.summary().to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input1".to_string(),
                    description: "First input raster (path string or typed raster object).".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "input2".to_string(),
                    description: "Second input raster (path string or typed raster object).".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output raster file path. If omitted, output remains in memory and is returned as a memory:// raster handle.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: format!("basic_{}", op.tag()),
                description: format!("Runs {} on two DEM rasters and writes the result to {}.", op.id(), op.default_output_name()),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "math".to_string(), op.tag().to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_with_op(op: BinaryMathOp, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let (input1_path, input2_path) = Self::parse_input_paths(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info(op.run_message());
        ctx.progress.info("reading input rasters");

        let input1 = Self::load_raster_from_arg(&input1_path, "input1")?;
        let input2 = Self::load_raster_from_arg(&input2_path, "input2")?;

        if input1.rows != input2.rows || input1.cols != input2.cols || input1.bands != input2.bands {
            return Err(ToolError::Validation(
                "input rasters must have identical rows, columns, and bands".to_string(),
            ));
        }

        let mut output = input1.clone();
        let len = output.data.len();
        let coalescer = PercentCoalescer::new(1, 99);

        ctx.progress.info(op.processing_message());

        let in1_values: Vec<f64> = (0..len).into_par_iter().map(|i| input1.data.get_f64(i)).collect();
        let in2_values: Vec<f64> = (0..len).into_par_iter().map(|i| input2.data.get_f64(i)).collect();
        let mut out_values = vec![input1.nodata; len];
        let total_chunks = len.div_ceil(BINARY_MATH_PAR_CHUNK).max(1);
        let compute_progress = PercentCoalescer::new(1, 75);
        let mut completed_chunks = 0usize;

        while completed_chunks < total_chunks {
            let batch_chunks = (total_chunks - completed_chunks).min(BINARY_MATH_PROGRESS_BATCH_CHUNKS);
            let batch_start = completed_chunks * BINARY_MATH_PAR_CHUNK;
            let batch_end = (batch_start + batch_chunks * BINARY_MATH_PAR_CHUNK).min(len);

            out_values[batch_start..batch_end]
                .par_chunks_mut(BINARY_MATH_PAR_CHUNK)
                .zip(in1_values[batch_start..batch_end].par_chunks(BINARY_MATH_PAR_CHUNK))
                .zip(in2_values[batch_start..batch_end].par_chunks(BINARY_MATH_PAR_CHUNK))
                .for_each(|((out_chunk, in1_chunk), in2_chunk)| {
                    for i in 0..out_chunk.len() {
                        let z1 = in1_chunk[i];
                        let z2 = in2_chunk[i];
                        out_chunk[i] = if input1.is_nodata(z1) || input2.is_nodata(z2) {
                            input1.nodata
                        } else {
                            op.apply(z1, z2, input1.nodata)
                        };
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

        let output_locator = if let Some(output_path) = output_path {
            if let Some(parent) = output_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ToolError::Execution(format!("failed creating output directory: {e}")))?;
                }
            }

            let output_path_str = output_path.to_string_lossy().to_string();
            let output_format = RasterFormat::for_output_path(&output_path_str)
                .map_err(|e| ToolError::Validation(format!("unsupported output path: {e}")))?;

            ctx.progress.info("writing output raster");
            output
                .write(&output_path_str, output_format)
                .map_err(|e| ToolError::Execution(format!("failed writing output raster: {e}")))?;

            output_path_str
        } else {
            ctx.progress.info("storing output raster in memory");
            let id = memory_store::put_raster(output);
            memory_store::make_raster_memory_path(&id)
        };

        ctx.progress.progress(1.0);

        let mut outputs = BTreeMap::new();
        outputs.insert("__wbw_type__".to_string(), json!("raster"));
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("active_band".to_string(), json!(0));
        Ok(ToolRunResult { outputs })
    }
}

impl Tool for RasterAddTool {
    fn metadata(&self) -> ToolMetadata {
        Self::metadata_for(BinaryMathOp::Add)
    }

    fn manifest(&self) -> ToolManifest {
        Self::manifest_for(BinaryMathOp::Add)
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = Self::parse_input_paths(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        Self::run_with_op(BinaryMathOp::Add, args, ctx)
    }
}

macro_rules! impl_binary_tool {
    ($tool:ident, $op:expr) => {
        impl Tool for $tool {
            fn metadata(&self) -> ToolMetadata {
                RasterAddTool::metadata_for($op)
            }

            fn manifest(&self) -> ToolManifest {
                RasterAddTool::manifest_for($op)
            }

            fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
                let _ = RasterAddTool::parse_input_paths(args)?;
                let _ = parse_optional_output_path(args, "output")?;
                Ok(())
            }

            fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
                RasterAddTool::run_with_op($op, args, ctx)
            }
        }
    };
}

impl_binary_tool!(RasterAtan2Tool, BinaryMathOp::Atan2);
impl_binary_tool!(RasterBoolAndTool, BinaryMathOp::BoolAnd);
impl_binary_tool!(RasterBoolOrTool, BinaryMathOp::BoolOr);
impl_binary_tool!(RasterBoolXorTool, BinaryMathOp::BoolXor);
impl_binary_tool!(RasterDivideTool, BinaryMathOp::Divide);
impl_binary_tool!(RasterEqualToTool, BinaryMathOp::EqualTo);
impl_binary_tool!(RasterGreaterThanTool, BinaryMathOp::GreaterThan);
impl_binary_tool!(RasterIntegerDivisionTool, BinaryMathOp::IntegerDivision);
impl_binary_tool!(RasterLessThanTool, BinaryMathOp::LessThan);
impl_binary_tool!(RasterModuloTool, BinaryMathOp::Modulo);
impl_binary_tool!(RasterMultiplyTool, BinaryMathOp::Multiply);
impl_binary_tool!(RasterNotEqualToTool, BinaryMathOp::NotEqualTo);
impl_binary_tool!(RasterPowerTool, BinaryMathOp::Power);
impl_binary_tool!(RasterSubtractTool, BinaryMathOp::Subtract);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::Mutex;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wbcore::{CapabilityProvider, ToolContext};
    use wbraster::{DataType, RasterConfig};

    struct AllowAll;

    impl CapabilityProvider for AllowAll {
        fn has_tool_access(&self, _tool_id: &'static str, _tier: LicenseTier) -> bool {
            true
        }
    }

    struct NoopProgress;
    impl wbcore::ProgressSink for NoopProgress {}

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

    impl wbcore::ProgressSink for RecordingProgress {
        fn progress(&self, pct: f64) {
            self.percents.lock().unwrap().push(pct);
        }
    }

    fn test_context() -> ToolContext<'static> {
        static CAPS: AllowAll = AllowAll;
        static PROGRESS: NoopProgress = NoopProgress;
        ToolContext {
            progress: &PROGRESS,
            capabilities: &CAPS,
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
                "wbtools_oss_add_{}_{}_{}",
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

    fn write_raster(path: &str, values: [f64; 4]) {
        let mut r = Raster::new(RasterConfig {
            cols: 2,
            rows: 2,
            bands: 1,
            x_min: 0.0,
            y_min: 0.0,
            cell_size: 1.0,
            nodata: -9999.0,
            data_type: DataType::F32,
            ..Default::default()
        });
        r.set(0, 0, 0, values[0]).unwrap();
        r.set(0, 0, 1, values[1]).unwrap();
        r.set(0, 1, 0, values[2]).unwrap();
        r.set(0, 1, 1, values[3]).unwrap();
        r.write(path, RasterFormat::GeoTiff).unwrap();
    }

    fn write_raster_grid(path: &str, rows: usize, cols: usize) {
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
                let value = if (row + col) % 19 == 0 {
                    -5.0
                } else {
                    (row * cols + col) as f64
                };
                r.set(0, row as isize, col as isize, value).unwrap();
            }
        }

        r.write(path, RasterFormat::GeoTiff).unwrap();
    }

    #[test]
    fn add_tool_creates_typed_raster_output() {
        let td = TempDirGuard::new("typed_output");
        let input1 = td.path().join("input1.tif");
        let input2 = td.path().join("input2.tif");
        let output = td.path().join("output_sum.tif");

        write_raster(input1.to_str().unwrap(), [1.0, 2.0, 3.0, 4.0]);
        write_raster(input2.to_str().unwrap(), [10.0, 20.0, 30.0, 40.0]);

        let mut args = ToolArgs::new();
        args.insert("input1".to_string(), json!(input1));
        args.insert("input2".to_string(), json!(input2));
        args.insert("output".to_string(), json!(output));

        let tool = RasterAddTool;
        let result = tool.run(&args, &test_context()).expect("tool should run");
        assert_eq!(result.outputs.get("__wbw_type__"), Some(&json!("raster")));
        assert_eq!(
            result.outputs.get("path"),
            Some(&json!(output.to_string_lossy().to_string()))
        );

        let out = Raster::read(output.to_str().unwrap()).expect("output raster should be readable");
        assert_eq!(out.get(0, 0, 0), 11.0);
        assert_eq!(out.get(0, 0, 1), 22.0);
        assert_eq!(out.get(0, 1, 0), 33.0);
        assert_eq!(out.get(0, 1, 1), 44.0);
    }

    #[test]
    fn add_tool_supports_in_memory_chaining_when_output_omitted() {
        let td = TempDirGuard::new("memory_chain");
        let input1 = td.path().join("input1.tif");
        let input2 = td.path().join("input2.tif");
        let output = td.path().join("output_sum.tif");

        write_raster(input1.to_str().unwrap(), [1.0, 2.0, 3.0, 4.0]);
        write_raster(input2.to_str().unwrap(), [10.0, 20.0, 30.0, 40.0]);

        let mut first_args = ToolArgs::new();
        first_args.insert("input1".to_string(), json!(input1));
        first_args.insert("input2".to_string(), json!(input2));

        let tool = RasterAddTool;
        let first = tool
            .run(&first_args, &test_context())
            .expect("first run should succeed");
        let mem_path = first
            .outputs
            .get("path")
            .and_then(Value::as_str)
            .expect("typed path should exist");
        assert!(mem_path.starts_with(memory_store::RASTER_MEMORY_PREFIX));

        let mut second_args = ToolArgs::new();
        second_args.insert("input1".to_string(), json!(mem_path));
        second_args.insert("input2".to_string(), json!(input1));
        second_args.insert("output".to_string(), json!(output));

        tool.run(&second_args, &test_context())
            .expect("second run should succeed");

        let out = Raster::read(output.to_str().unwrap()).expect("output raster should be readable");
        assert_eq!(out.get(0, 0, 0), 12.0);
        assert_eq!(out.get(0, 0, 1), 24.0);
        assert_eq!(out.get(0, 1, 0), 36.0);
        assert_eq!(out.get(0, 1, 1), 48.0);
    }

    #[test]
    fn add_progress_is_monotonic_bounded_and_completes() {
        let td = TempDirGuard::new("progress");
        let input1 = td.path().join("input1.tif");
        let input2 = td.path().join("input2.tif");

        write_raster_grid(input1.to_str().unwrap(), 1024, 1024);
        write_raster_grid(input2.to_str().unwrap(), 1024, 1024);

        let mut args = ToolArgs::new();
        args.insert("input1".to_string(), json!(input1));
        args.insert("input2".to_string(), json!(input2));

        let caps = AllowAll;
        let progress = RecordingProgress::new();
        let ctx = ToolContext {
            progress: &progress,
            capabilities: &caps,
        };

        let tool = RasterAddTool;
        tool.run(&args, &ctx).expect("tool should run");

        let percents = progress.percents();
        assert!(!percents.is_empty(), "expected progress events");
        assert!(percents.len() <= 101, "progress events should be bounded to percent buckets");

        for w in percents.windows(2) {
            assert!(w[1] >= w[0], "progress should be monotonic non-decreasing");
        }

        let final_pct = *percents.last().unwrap();
        assert!((final_pct - 1.0).abs() < 1e-9, "final progress should be 100%");
    }
}
