//! Openness tool: Yokoyama et al. (2002) topographic openness index
//! Computes positive (convex) and negative (concave) openness using 8-directional horizon angles.

use rayon::prelude::*;
use serde_json::json;
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolManifest, ToolMetadata, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::Raster;

use wbraster::memory_store;

pub struct OpennessTool;

struct OpennessCore;

impl OpennessCore {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation("parameter 'input' has malformed in-memory raster path".to_string())
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!("parameter 'input' references unknown in-memory raster id '{}'", id))
            });
        }
        Raster::read(path).map_err(|e| ToolError::Execution(format!("failed to read raster: {}", e)))
    }

    fn write_or_store_output(raster: Raster, output_path: Option<&str>) -> Result<String, ToolError> {
        if let Some(path) = output_path {
            raster.write(path, wbraster::RasterFormat::GeoTiff).map_err(|e| ToolError::Execution(format!("failed to write raster: {}", e)))?;
            Ok(path.to_string())
        } else {
            let id = memory_store::put_raster(raster);
            Ok(memory_store::make_raster_memory_path(&id))
        }
    }

    fn build_result(output_path1: String, output_path2: String) -> ToolRunResult {
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("pos_output".to_string(), json!(output_path1));
        outputs.insert("neg_output".to_string(), json!(output_path2));
        ToolRunResult { outputs }
    }

    fn openness_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "openness",
            display_name: "Openness",
            summary: "Calculates Yokoyama et al. (2002) topographic openness index from an input DEM.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "dist", description: "Search distance in cells (default 20).", required: false },
                ToolParamSpec { name: "pos_output", description: "Optional output path for positive openness.", required: false },
                ToolParamSpec { name: "neg_output", description: "Optional output path for negative openness.", required: false },
            ],
        }
    }

    fn openness_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("dist".to_string(), json!(20));
        ToolManifest {
            id: "openness".to_string(),
            display_name: "Openness".to_string(),
            summary: "Calculates Yokoyama et al. (2002) topographic openness from an input DEM. \
                      Returns positive (convex) and negative (concave) openness rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Stable,
        }
    }

    fn run_openness(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let input_path = Self::parse_input(args)?;
        let pos_output_path = parse_optional_output_path(args, "pos_output")?;
        let neg_output_path = parse_optional_output_path(args, "neg_output")?;
        let search_dist = args
            .get("dist")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .max(1) as usize;

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let coalescer = PercentCoalescer::new(1, 99);
        let cols = input.cols;
        let _bands = input.bands;
        let nodata = input.nodata;
        let cell_size_x = input.cell_size_x.abs();
        let cell_size_y = input.cell_size_y.abs();
        let res_diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let diag_search_dist = ((search_dist as f64 * cell_size_x) / res_diag).ceil() as usize;

        // For now, process only the first band
        let band = 0isize;

        ctx.progress.info(&format!("running openness with search_dist={}", search_dist));

        // Process all rows in parallel
        let results: Vec<(Vec<f64>, Vec<f64>)> = (0..rows)
            .into_par_iter()
            .map(|r| {
                let mut pos_data = vec![nodata; cols];
                let mut neg_data = vec![nodata; cols];

                for c in 0..cols {
                    let z = input.get(band, r as isize, c as isize);
                    if input.is_nodata(z) {
                        continue;
                    }

                    let mut max_theta = [f64::NEG_INFINITY; 8];
                    let mut min_theta = [f64::INFINITY; 8];

                    // North
                    for n in 1..search_dist as isize {
                        let zn = input.get(band, r as isize - n, c as isize);
                        if !input.is_nodata(zn) {
                            let dist = cell_size_y * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[0] = max_theta[0].max(theta);
                            min_theta[0] = min_theta[0].min(theta);
                        }
                    }

                    // Northeast
                    for n in 1..diag_search_dist as isize {
                        let zn = input.get(band, r as isize - n, c as isize + n);
                        if !input.is_nodata(zn) {
                            let dist = res_diag * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[1] = max_theta[1].max(theta);
                            min_theta[1] = min_theta[1].min(theta);
                        }
                    }

                    // East
                    for n in 1..search_dist as isize {
                        let zn = input.get(band, r as isize, c as isize + n);
                        if !input.is_nodata(zn) {
                            let dist = cell_size_x * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[2] = max_theta[2].max(theta);
                            min_theta[2] = min_theta[2].min(theta);
                        }
                    }

                    // Southeast
                    for n in 1..diag_search_dist as isize {
                        let zn = input.get(band, r as isize + n, c as isize + n);
                        if !input.is_nodata(zn) {
                            let dist = res_diag * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[3] = max_theta[3].max(theta);
                            min_theta[3] = min_theta[3].min(theta);
                        }
                    }

                    // South
                    for n in 1..search_dist as isize {
                        let zn = input.get(band, r as isize + n, c as isize);
                        if !input.is_nodata(zn) {
                            let dist = cell_size_y * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[4] = max_theta[4].max(theta);
                            min_theta[4] = min_theta[4].min(theta);
                        }
                    }

                    // Southwest
                    for n in 1..diag_search_dist as isize {
                        let zn = input.get(band, r as isize + n, c as isize - n);
                        if !input.is_nodata(zn) {
                            let dist = res_diag * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[5] = max_theta[5].max(theta);
                            min_theta[5] = min_theta[5].min(theta);
                        }
                    }

                    // West
                    for n in 1..search_dist as isize {
                        let zn = input.get(band, r as isize, c as isize - n);
                        if !input.is_nodata(zn) {
                            let dist = cell_size_x * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[6] = max_theta[6].max(theta);
                            min_theta[6] = min_theta[6].min(theta);
                        }
                    }

                    // Northwest
                    for n in 1..diag_search_dist as isize {
                        let zn = input.get(band, r as isize - n, c as isize - n);
                        if !input.is_nodata(zn) {
                            let dist = res_diag * n as f64;
                            let theta = ((zn - z) / dist).atan();
                            max_theta[7] = max_theta[7].max(theta);
                            min_theta[7] = min_theta[7].min(theta);
                        }
                    }

                    // Convert angles to openness
                    let mut pos_openness = 0.0f64;
                    let mut neg_openness = 0.0f64;
                    let mut pos_count = 0.0f64;
                    let mut neg_count = 0.0f64;

                    for i in 0..8 {
                        if !max_theta[i].is_infinite() {
                            pos_openness += 90.0 - max_theta[i].to_degrees();
                            pos_count += 1.0;
                        }
                        if !min_theta[i].is_infinite() {
                            neg_openness += 90.0 + min_theta[i].to_degrees();
                            neg_count += 1.0;
                        }
                    }

                    if pos_count > 0.0 {
                        pos_data[c] = pos_openness / pos_count;
                    }
                    if neg_count > 0.0 {
                        neg_data[c] = neg_openness / neg_count;
                    }
                }

                (pos_data, neg_data)
            })
            .collect();

        // Write results to output rasters
        let mut pos_output = input.clone();
        let mut neg_output = input.clone();

        for (r, (pos_row, neg_row)) in results.into_iter().enumerate() {
            pos_output
                .set_row_slice(band, r as isize, &pos_row)
                .map_err(|e| ToolError::Execution(format!("failed writing pos row {}: {}", r, e)))?;
            neg_output
                .set_row_slice(band, r as isize, &neg_row)
                .map_err(|e| ToolError::Execution(format!("failed writing neg row {}: {}", r, e)))?;
            coalescer.emit_unit_fraction(ctx.progress, r as f64 / rows as f64);
        }

        let pos_out_path = Self::write_or_store_output(pos_output, pos_output_path.as_ref().map(|p| p.to_str()).flatten())?;
        let neg_out_path = Self::write_or_store_output(neg_output, neg_output_path.as_ref().map(|p| p.to_str()).flatten())?;

        Ok(Self::build_result(pos_out_path, neg_out_path))
    }
}

impl Tool for OpennessTool {
    fn metadata(&self) -> ToolMetadata {
        OpennessCore::openness_metadata()
    }

    fn manifest(&self) -> ToolManifest {
        OpennessCore::openness_manifest()
    }

    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = OpennessCore::parse_input(args)?;
        Ok(())
    }

    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        OpennessCore::run_openness(args, ctx)
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
            cell_size: 10.0,
            ..Default::default()
        };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, value).unwrap();
            }
        }
        raster
    }

    #[test]
    fn openness_flat_raster_returns_zero() {
        let dem = make_constant_raster(11, 11, 100.0);
        let id = memory_store::put_raster(dem);
        let input_path = memory_store::make_raster_memory_path(&id);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(input_path));
        args.insert("dist".to_string(), json!(5));
        let result = OpennessTool.run(&args, &make_ctx()).unwrap();
        let pos_path = result.outputs.get("pos_output").unwrap().as_str().unwrap();
        let neg_path = result.outputs.get("neg_output").unwrap().as_str().unwrap();
        let pos_id = memory_store::raster_path_to_id(pos_path).unwrap();
        let neg_id = memory_store::raster_path_to_id(neg_path).unwrap();
        let pos_out = memory_store::get_raster_by_id(pos_id).unwrap();
        let neg_out = memory_store::get_raster_by_id(neg_id).unwrap();
        // For flat surface, all directions see zero slope, so openness should be 90°
        let pos_v = pos_out.get(0, 5, 5);
        let neg_v = neg_out.get(0, 5, 5);
        assert!((pos_v - 90.0).abs() < 1e-4, "expected pos ~90, got {}", pos_v);
        assert!((neg_v - 90.0).abs() < 1e-4, "expected neg ~90, got {}", neg_v);
    }
}
