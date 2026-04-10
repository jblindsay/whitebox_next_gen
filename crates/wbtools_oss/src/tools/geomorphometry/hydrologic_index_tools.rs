use rayon::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata, ToolParamDescriptor,
    ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{Raster, RasterFormat};

use crate::memory_store;

pub struct RelativeStreamPowerIndexTool;
pub struct SedimentTransportIndexTool;
pub struct ElevRelativeToWatershedMinMaxTool;

struct HydrologicIndexCore;

impl HydrologicIndexCore {
    fn load_named_raster(args: &ToolArgs, key: &str) -> Result<Raster, ToolError> {
        let path = parse_raster_path_arg(args, key)?;
        if memory_store::raster_is_memory_path(&path) {
            let id = memory_store::raster_path_to_id(&path).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' has malformed in-memory raster path",
                    key
                ))
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter '{}' references unknown in-memory raster id '{}'",
                    key, id
                ))
            });
        }
        Raster::read(&path)
            .map_err(|e| ToolError::Execution(format!("failed reading '{}' raster: {}", key, e)))
    }

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

    fn build_result(output_locator: String) -> ToolRunResult {
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn validate_same_dimensions(first: &Raster, second: &Raster) -> Result<(), ToolError> {
        if first.rows != second.rows || first.cols != second.cols || first.bands != second.bands {
            return Err(ToolError::Validation(
                "input rasters must have the same rows, columns, and bands".to_string(),
            ));
        }
        Ok(())
    }

    fn relative_stream_power_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "relative_stream_power_index",
            display_name: "Relative Stream Power Index",
            summary: "Calculates the relative stream power index from specific catchment area and slope.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "sca", description: "Specific catchment area raster path or typed raster object.", required: true },
                ToolParamSpec { name: "slope", description: "Slope raster in degrees.", required: true },
                ToolParamSpec { name: "exponent", description: "Specific catchment area exponent p (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn relative_stream_power_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("sca".to_string(), json!("sca.tif"));
        defaults.insert("slope".to_string(), json!("slope.tif"));
        defaults.insert("exponent".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("sca".to_string(), json!("sca.tif"));
        example_args.insert("slope".to_string(), json!("slope.tif"));
        example_args.insert("output".to_string(), json!("relative_stream_power_index.tif"));

        ToolManifest {
            id: "relative_stream_power_index".to_string(),
            display_name: "Relative Stream Power Index".to_string(),
            summary: "Calculates the relative stream power index from specific catchment area and slope.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "sca".to_string(), description: "Specific catchment area raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "slope".to_string(), description: "Slope raster in degrees.".to_string(), required: true },
                ToolParamDescriptor { name: "exponent".to_string(), description: "Specific catchment area exponent p (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_relative_stream_power_index".to_string(),
                description: "Compute RSP from SCA and slope rasters.".to_string(),
                args: example_args,
            }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hydrology".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn sediment_transport_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "sediment_transport_index",
            display_name: "Sediment Transport Index",
            summary: "Calculates the sediment transport index (LS factor) from specific catchment area and slope.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "sca", description: "Specific catchment area raster path or typed raster object.", required: true },
                ToolParamSpec { name: "slope", description: "Slope raster in degrees.", required: true },
                ToolParamSpec { name: "sca_exponent", description: "Specific catchment area exponent n (default 0.4).", required: false },
                ToolParamSpec { name: "slope_exponent", description: "Slope exponent m (default 1.3).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn sediment_transport_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("sca".to_string(), json!("sca.tif"));
        defaults.insert("slope".to_string(), json!("slope.tif"));
        defaults.insert("sca_exponent".to_string(), json!(0.4));
        defaults.insert("slope_exponent".to_string(), json!(1.3));

        let mut example_args = ToolArgs::new();
        example_args.insert("sca".to_string(), json!("sca.tif"));
        example_args.insert("slope".to_string(), json!("slope.tif"));
        example_args.insert("output".to_string(), json!("sediment_transport_index.tif"));

        ToolManifest {
            id: "sediment_transport_index".to_string(),
            display_name: "Sediment Transport Index".to_string(),
            summary: "Calculates the sediment transport index (LS factor) from specific catchment area and slope.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "sca".to_string(), description: "Specific catchment area raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "slope".to_string(), description: "Slope raster in degrees.".to_string(), required: true },
                ToolParamDescriptor { name: "sca_exponent".to_string(), description: "Specific catchment area exponent n (default 0.4).".to_string(), required: false },
                ToolParamDescriptor { name: "slope_exponent".to_string(), description: "Slope exponent m (default 1.3).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_sediment_transport_index".to_string(),
                description: "Compute STI from SCA and slope rasters.".to_string(),
                args: example_args,
            }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hydrology".to_string(), "erosion".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn elev_relative_to_watershed_min_max_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "elev_relative_to_watershed_min_max",
            display_name: "Elev Relative To Watershed Min Max",
            summary: "Calculates a DEM cell's relative elevation position within each watershed as a percentage.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "dem", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "watersheds", description: "Watershed ID raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn elev_relative_to_watershed_min_max_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("watersheds".to_string(), json!("watersheds.tif"));

        let mut example_args = ToolArgs::new();
        example_args.insert("dem".to_string(), json!("dem.tif"));
        example_args.insert("watersheds".to_string(), json!("watersheds.tif"));
        example_args.insert("output".to_string(), json!("elev_relative_to_watershed_min_max.tif"));

        ToolManifest {
            id: "elev_relative_to_watershed_min_max".to_string(),
            display_name: "Elev Relative To Watershed Min Max".to_string(),
            summary: "Calculates a DEM cell's relative elevation position within each watershed as a percentage.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "dem".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "watersheds".to_string(), description: "Watershed ID raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_elev_relative_to_watershed_min_max".to_string(),
                description: "Compute relative elevation positions for each watershed polygon/raster zone.".to_string(),
                args: example_args,
            }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "hydrology".to_string(), "watersheds".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn run_relative_stream_power_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_optional_output_path(args, "output")?;
        let exponent = args.get("exponent").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let sca = Self::load_named_raster(args, "sca")?;
        let slope = Self::load_named_raster(args, "slope")?;
        Self::validate_same_dimensions(&sca, &slope)?;

        ctx.progress.info("running relative_stream_power_index");
        let mut output = sca.clone();
        let rows = sca.rows;
        let cols = sca.cols;
        let bands = sca.bands;
        let coalescer = PercentCoalescer::new(1, 99);
        let nodata = sca.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = r as isize;
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let col = c as isize;
                        let sca_val = sca.get(band, row, col);
                        let slope_val = slope.get(band, row, col);
                        if sca.is_nodata(sca_val) || slope.is_nodata(slope_val) || sca_val <= 0.0 {
                            continue;
                        }
                        row_out[c] = sca_val.powf(exponent) * slope_val.to_radians().tan();
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            coalescer.emit_unit_fraction(ctx.progress, (band_idx + 1) as f64 / bands as f64);
        }
        let output_locator = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(output_locator))
    }

    fn run_sediment_transport_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_optional_output_path(args, "output")?;
        let sca_exponent = args.get("sca_exponent").and_then(|v| v.as_f64()).unwrap_or(0.4);
        let slope_exponent = args.get("slope_exponent").and_then(|v| v.as_f64()).unwrap_or(1.3);
        let sca = Self::load_named_raster(args, "sca")?;
        let slope = Self::load_named_raster(args, "slope")?;
        Self::validate_same_dimensions(&sca, &slope)?;

        ctx.progress.info("running sediment_transport_index");
        let mut output = sca.clone();
        let rows = sca.rows;
        let cols = sca.cols;
        let bands = sca.bands;
        let coalescer = PercentCoalescer::new(1, 99);
        let nodata = sca.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = r as isize;
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let col = c as isize;
                        let sca_val = sca.get(band, row, col);
                        let slope_val = slope.get(band, row, col);
                        if sca.is_nodata(sca_val)
                            || slope.is_nodata(slope_val)
                            || sca_val <= 0.0
                        {
                            continue;
                        }
                        let sin_term = slope_val.to_radians().sin() / 0.0896;
                        if sin_term <= 0.0 {
                            continue;
                        }
                        row_out[c] = (sca_exponent + 1.0)
                            * (sca_val / 22.13).powf(sca_exponent)
                            * sin_term.powf(slope_exponent);
                    }
                    row_out
                })
                .collect();
            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
            coalescer.emit_unit_fraction(ctx.progress, (band_idx + 1) as f64 / bands as f64);
        }
        let output_locator = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(output_locator))
    }

    fn run_elev_relative_to_watershed_min_max(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let output_path = parse_optional_output_path(args, "output")?;
        let dem = Self::load_named_raster(args, "dem")?;
        let watersheds = Self::load_named_raster(args, "watersheds")?;
        Self::validate_same_dimensions(&dem, &watersheds)?;

        ctx.progress.info("running elev_relative_to_watershed_min_max");
        let rows = dem.rows;
        let cols = dem.cols;
        let bands = dem.bands;
        let coalescer = PercentCoalescer::new(1, 99);
        let dem_nodata = dem.nodata;
        let ws_nodata = watersheds.nodata;
        let mut output = dem.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut ranges: HashMap<u64, (f64, f64)> = HashMap::new();

            for r in 0..rows {
                for c in 0..cols {
                    let z = dem.get(band, r as isize, c as isize);
                    let w = watersheds.get(band, r as isize, c as isize);
                    if z == dem_nodata || w == ws_nodata {
                        continue;
                    }
                    let key = w.to_bits();
                    let entry = ranges.entry(key).or_insert((f64::INFINITY, f64::NEG_INFINITY));
                    if z < entry.0 {
                        entry.0 = z;
                    }
                    if z > entry.1 {
                        entry.1 = z;
                    }
                }
            }

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = r as isize;
                    let mut row_out = vec![dem_nodata; cols];
                    for c in 0..cols {
                        let col = c as isize;
                        let z = dem.get(band, row, col);
                        let w = watersheds.get(band, row, col);
                        if z == dem_nodata || w == ws_nodata {
                            continue;
                        }
                        if let Some((mn, mx)) = ranges.get(&w.to_bits()) {
                            let range = mx - mn;
                            row_out[c] = if range > 0.0 { ((z - mn) / range) * 100.0 } else { 0.0 };
                        }
                    }
                    row_out
                })
                .collect();

            for (r, row) in row_data.iter().enumerate() {
                output.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing row {}: {}", r, e))
                })?;
            }
            coalescer.emit_unit_fraction(ctx.progress, (band_idx + 1) as f64 / bands as f64);
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        Ok(Self::build_result(output_locator))
    }
}

impl Tool for RelativeStreamPowerIndexTool {
    fn metadata(&self) -> ToolMetadata { HydrologicIndexCore::relative_stream_power_index_metadata() }
    fn manifest(&self) -> ToolManifest { HydrologicIndexCore::relative_stream_power_index_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "sca")?;
        let _ = parse_raster_path_arg(args, "slope")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        HydrologicIndexCore::run_relative_stream_power_index(args, ctx)
    }
}

impl Tool for SedimentTransportIndexTool {
    fn metadata(&self) -> ToolMetadata { HydrologicIndexCore::sediment_transport_index_metadata() }
    fn manifest(&self) -> ToolManifest { HydrologicIndexCore::sediment_transport_index_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "sca")?;
        let _ = parse_raster_path_arg(args, "slope")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        HydrologicIndexCore::run_sediment_transport_index(args, ctx)
    }
}

impl Tool for ElevRelativeToWatershedMinMaxTool {
    fn metadata(&self) -> ToolMetadata { HydrologicIndexCore::elev_relative_to_watershed_min_max_metadata() }
    fn manifest(&self) -> ToolManifest { HydrologicIndexCore::elev_relative_to_watershed_min_max_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "dem")?;
        let _ = parse_raster_path_arg(args, "watersheds")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        HydrologicIndexCore::run_elev_relative_to_watershed_min_max(args, ctx)
    }
}

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
        ToolContext { progress: &PROGRESS, capabilities: &CAPS }
    }

    fn make_constant_raster(rows: usize, cols: usize, value: f64) -> Raster {
        let cfg = RasterConfig { rows, cols, bands: 1, nodata: -9999.0, cell_size: 10.0, ..Default::default() };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, value).unwrap();
            }
        }
        raster
    }

    #[test]
    fn relative_stream_power_index_matches_expected_formula() {
        let sca_id = memory_store::put_raster(make_constant_raster(5, 5, std::f64::consts::E));
        let slope_id = memory_store::put_raster(make_constant_raster(5, 5, 45.0));
        let mut args = ToolArgs::new();
        args.insert("sca".to_string(), json!(memory_store::make_raster_memory_path(&sca_id)));
        args.insert("slope".to_string(), json!(memory_store::make_raster_memory_path(&slope_id)));
        let result = RelativeStreamPowerIndexTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let v = out.get(0, 2, 2);
        assert!((v - std::f64::consts::E).abs() < 1e-6, "expected e, got {v}");
    }

    #[test]
    fn sediment_transport_index_matches_expected_formula() {
        let slope_deg = 0.0896_f64.asin().to_degrees();
        let sca_id = memory_store::put_raster(make_constant_raster(5, 5, 22.13));
        let slope_id = memory_store::put_raster(make_constant_raster(5, 5, slope_deg));
        let mut args = ToolArgs::new();
        args.insert("sca".to_string(), json!(memory_store::make_raster_memory_path(&sca_id)));
        args.insert("slope".to_string(), json!(memory_store::make_raster_memory_path(&slope_id)));
        let result = SedimentTransportIndexTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let v = out.get(0, 2, 2);
        assert!((v - 1.4).abs() < 1e-6, "expected 1.4, got {v}");
    }

    #[test]
    fn elev_relative_to_watershed_min_max_matches_zone_percentages() {
        let cfg = RasterConfig { rows: 1, cols: 4, bands: 1, nodata: -9999.0, cell_size: 1.0, ..Default::default() };
        let mut dem = Raster::new(cfg.clone());
        dem.set(0, 0, 0, 10.0).unwrap();
        dem.set(0, 0, 1, 20.0).unwrap();
        dem.set(0, 0, 2, 100.0).unwrap();
        dem.set(0, 0, 3, 130.0).unwrap();

        let mut ws = Raster::new(cfg);
        ws.set(0, 0, 0, 1.0).unwrap();
        ws.set(0, 0, 1, 1.0).unwrap();
        ws.set(0, 0, 2, 2.0).unwrap();
        ws.set(0, 0, 3, 2.0).unwrap();

        let dem_id = memory_store::put_raster(dem);
        let ws_id = memory_store::put_raster(ws);
        let mut args = ToolArgs::new();
        args.insert("dem".to_string(), json!(memory_store::make_raster_memory_path(&dem_id)));
        args.insert("watersheds".to_string(), json!(memory_store::make_raster_memory_path(&ws_id)));

        let result = ElevRelativeToWatershedMinMaxTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();

        assert!((out.get(0, 0, 0) - 0.0).abs() < 1e-6);
        assert!((out.get(0, 0, 1) - 100.0).abs() < 1e-6);
        assert!((out.get(0, 0, 2) - 0.0).abs() < 1e-6);
        assert!((out.get(0, 0, 3) - 100.0).abs() < 1e-6);
    }
}
