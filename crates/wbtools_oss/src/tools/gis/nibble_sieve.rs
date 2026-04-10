use serde_json::json;
use rayon::prelude::*;
use wbcore::{
    parse_optional_output_path, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

use super::{ClumpTool, EuclideanAllocationTool, RasterAreaTool};

pub struct NibbleTool;
pub struct SieveTool;

fn get_path(result: &ToolRunResult) -> Result<String, ToolError> {
    result
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Execution("sub-tool returned no path".to_string()))
}

fn load_raster(path: &str, label: &str) -> Result<Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        Raster::read(std::path::Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

fn put_raster(r: Raster) -> String {
    let id = memory_store::put_raster(r);
    memory_store::make_raster_memory_path(&id)
}

fn write_or_store(
    r: Raster,
    output_path: Option<std::path::PathBuf>,
) -> Result<ToolRunResult, ToolError> {
    let mut outputs = std::collections::BTreeMap::new();
    if let Some(p) = output_path {
        let path_str = p.to_string_lossy().to_string();
        let fmt = RasterFormat::for_output_path(&path_str)
            .map_err(|e| ToolError::Validation(format!("unsupported output format: {e}")))?;
        r.write(&path_str, fmt)
            .map_err(|e| ToolError::Execution(format!("failed writing output: {e}")))?;
        outputs.insert("path".to_string(), json!(p.to_string_lossy().to_string()));
    } else {
        let id = memory_store::put_raster(r);
        let mem_path = memory_store::make_raster_memory_path(&id);
        outputs.insert("path".to_string(), json!(mem_path));
    }
    Ok(ToolRunResult { outputs, ..Default::default() })
}

impl Tool for NibbleTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "nibble",
            display_name: "Nibble",
            summary: "Fills background (zero/nodata) regions of a raster by propagating values from the nearest foreground cell, masked by an optional mask raster.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input categorical or continuous raster to be nibbled.", required: true },
                ToolParamSpec { name: "mask", description: "Binary mask raster. Cells with value 0 or nodata are treated as background regions to fill.", required: true },
                ToolParamSpec { name: "use_nodata", description: "If true, nodata cells in the input are treated as additional class values (not background). Default false.", required: false },
                ToolParamSpec { name: "nibble_nodata", description: "If true, apply nibble also into mask-off regions that were nodata in the input. Default true.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("mask".to_string(), json!("mask.tif"));
        defaults.insert("use_nodata".to_string(), json!(false));
        defaults.insert("nibble_nodata".to_string(), json!(true));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("nibbled.tif"));

        ToolManifest {
            id: "nibble".to_string(),
            display_name: "Nibble".to_string(),
            summary: "Fills background regions using nearest-neighbour allocation.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input raster to be nibbled.".to_string(), required: true },
                ToolParamDescriptor { name: "mask".to_string(), description: "Binary mask raster.".to_string(), required: true },
                ToolParamDescriptor { name: "use_nodata".to_string(), description: "Treat input nodata as class value (not background).".to_string(), required: false },
                ToolParamDescriptor { name: "nibble_nodata".to_string(), description: "Also nibble into mask-off nodata areas.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "nibble_basic".to_string(),
                description: "Fill background using nearest class.".to_string(),
                args: example_args,
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "nibble".to_string(), "allocation".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        args.get("input").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        args.get("mask").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'mask' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args.get("input").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        let mask_path = args.get("mask").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'mask' is required".to_string()))?;
        let use_nodata = args.get("use_nodata").and_then(|v| v.as_bool()).unwrap_or(false);
        let nibble_nodata = args.get("nibble_nodata").and_then(|v| v.as_bool()).unwrap_or(true);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("nibble: loading input");
        let input = load_raster(input_path, "input")?;
        let mask_raw = load_raster(mask_path, "mask")?;

        let rows = input.rows;
        let cols = input.cols;
        let input_nodata = input.nodata;

        let band_stride = rows * cols;
        let mask: Vec<f64> = (0..band_stride)
            .into_par_iter()
            .map(|idx| {
                let mv = mask_raw.data.get_f64(idx);
                if !mask_raw.is_nodata(mv) && mv != 0.0 {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();

        let input_nodata_mask: Vec<f64> = (0..band_stride)
            .into_par_iter()
            .map(|i| {
                let v = input.data.get_f64(i);
                if input.is_nodata(v) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();

        let max_class = {
            let m = (0..band_stride)
                .into_par_iter()
                .map(|i| {
                    let v = input.data.get_f64(i);
                    if !input.is_nodata(v) {
                        v
                    } else {
                        f64::NEG_INFINITY
                    }
                })
                .reduce(|| f64::NEG_INFINITY, f64::max);
            if m == f64::NEG_INFINITY { 0.0 } else { m }
        };

        let mut source = input.clone();
        let nodata_replacement = if use_nodata { max_class + 1.0 } else { 0.0 };
        let source_values: Vec<f64> = (0..band_stride)
            .into_par_iter()
            .map(|i| {
                let mut v = input.data.get_f64(i);
                if input.is_nodata(v) {
                    v = nodata_replacement;
                }
                if mask[i] == 0.0 {
                    v = 0.0;
                }
                v
            })
            .collect();
        for (i, v) in source_values.into_iter().enumerate() {
            source.data.set_f64(i, v);
        }

        ctx.progress.info("nibble: running euclidean allocation");
        let source_mem = put_raster(source);
        let mut ea_args = ToolArgs::new();
        ea_args.insert("input".to_string(), json!(source_mem));
        let ea_result = EuclideanAllocationTool.run(&ea_args, ctx)?;
        let ea_path = get_path(&ea_result)?;
        let mut nibbled = load_raster(&ea_path, "euclidean_allocation")?;
        nibbled.nodata = input_nodata;

        if use_nodata {
            let restore_mask: Vec<bool> = (0..band_stride)
                .into_par_iter()
                .map(|i| {
                    let v = nibbled.data.get_f64(i);
                    (v - (max_class + 1.0)).abs() < 1e-9
                })
                .collect();
            for (i, restore) in restore_mask.into_iter().enumerate() {
                if restore {
                    nibbled.data.set_f64(i, input_nodata);
                }
            }
        } else {
            let restore_mask: Vec<bool> = (0..band_stride)
                .into_par_iter()
                .map(|i| mask[i] == 1.0 && input_nodata_mask[i] == 1.0)
                .collect();
            for (i, restore) in restore_mask.into_iter().enumerate() {
                if restore {
                    nibbled.data.set_f64(i, input_nodata);
                }
            }
        }

        if nibble_nodata {
            let restore_mask: Vec<bool> = (0..band_stride)
                .into_par_iter()
                .map(|i| mask[i] == 0.0 && input_nodata_mask[i] == 1.0)
                .collect();
            for (i, restore) in restore_mask.into_iter().enumerate() {
                if restore {
                    nibbled.data.set_f64(i, input_nodata);
                }
            }
        }

        ctx.progress.info("nibble: writing output");
        write_or_store(nibbled, output_path)
    }
}

impl Tool for SieveTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "sieve",
            display_name: "Sieve",
            summary: "Removes small isolated raster patches below a cell-count threshold by replacing them with values of surrounding larger patches.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input categorical raster.", required: true },
                ToolParamSpec { name: "threshold", description: "Minimum patch size in cells to keep. Patches smaller than this are removed. Default 1.", required: false },
                ToolParamSpec { name: "zero_background", description: "If true, cells with value 0 are treated as background and zeroed in the output. Default false.", required: false },
                ToolParamSpec { name: "output", description: "Optional output raster path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.tif"));
        defaults.insert("threshold".to_string(), json!(1));
        defaults.insert("zero_background".to_string(), json!(false));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("sieved.tif"));

        ToolManifest {
            id: "sieve".to_string(),
            display_name: "Sieve".to_string(),
            summary: "Removes small isolated patches below a cell-count threshold.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input categorical raster.".to_string(), required: true },
                ToolParamDescriptor { name: "threshold".to_string(), description: "Minimum patch size in cells to keep.".to_string(), required: false },
                ToolParamDescriptor { name: "zero_background".to_string(), description: "Zero-out background (value 0) cells in output.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output raster path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "sieve_basic".to_string(),
                description: "Remove patches smaller than 10 cells.".to_string(),
                args: {
                    let mut a = ToolArgs::new();
                    a.insert("input".to_string(), json!("classified.tif"));
                    a.insert("threshold".to_string(), json!(10));
                    a.insert("output".to_string(), json!("sieved.tif"));
                    a
                },
            }],
            tags: vec!["raster".to_string(), "gis".to_string(), "sieve".to_string(), "patch".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        args.get("input").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = args.get("input").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::Validation("parameter 'input' is required".to_string()))?;
        let threshold = args.get("threshold").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let zero_background = args.get("zero_background").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        ctx.progress.info("sieve: clumping input");
        let mut clump_args = ToolArgs::new();
        clump_args.insert("input".to_string(), json!(input_path));
        clump_args.insert("diag".to_string(), json!(true));
        clump_args.insert("zero_background".to_string(), json!(false));
        let clump_result = ClumpTool.run(&clump_args, ctx)?;
        let clump_path = get_path(&clump_result)?;

        ctx.progress.info("sieve: computing patch areas");
        let mut area_args = ToolArgs::new();
        area_args.insert("input".to_string(), json!(clump_path));
        area_args.insert("units".to_string(), json!("grid cells"));
        area_args.insert("zero_background".to_string(), json!(false));
        let area_result = RasterAreaTool.run(&area_args, ctx)?;
        let area_path = get_path(&area_result)?;

        ctx.progress.info("sieve: building size mask");
        let area_raster = load_raster(&area_path, "area")?;
        let rows = area_raster.rows;
        let cols = area_raster.cols;
        let band_stride = rows * cols;

        let mut mask_raster = area_raster.clone();
        mask_raster.nodata = -999.0;
        let mask_values: Vec<f64> = (0..band_stride)
            .into_par_iter()
            .map(|i| {
                let v = area_raster.data.get_f64(i);
                if area_raster.is_nodata(v) || v < threshold {
                    mask_raster.nodata
                } else {
                    1.0
                }
            })
            .collect();
        for (i, v) in mask_values.into_iter().enumerate() {
            mask_raster.data.set_f64(i, v);
        }
        let mask_mem = put_raster(mask_raster);

        ctx.progress.info("sieve: nibbling using size mask");
        let mut nibble_args = ToolArgs::new();
        nibble_args.insert("input".to_string(), json!(input_path));
        nibble_args.insert("mask".to_string(), json!(mask_mem));
        nibble_args.insert("use_nodata".to_string(), json!(false));
        nibble_args.insert("nibble_nodata".to_string(), json!(true));
        let nibble_result = NibbleTool.run(&nibble_args, ctx)?;
        let nibble_path = get_path(&nibble_result)?;

        ctx.progress.info("sieve: finalizing output");
        let mut sieved = load_raster(&nibble_path, "nibbled")?;
        if zero_background {
            let original = load_raster(input_path, "input")?;
            let zero_mask: Vec<bool> = (0..band_stride)
                .into_par_iter()
                .map(|i| original.data.get_f64(i) == 0.0)
                .collect();
            for (i, is_zero) in zero_mask.into_iter().enumerate() {
                if is_zero {
                    sieved.data.set_f64(i, 0.0);
                }
            }
        }

        write_or_store(sieved, output_path)
    }
}
