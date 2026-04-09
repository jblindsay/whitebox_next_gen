/// LiDAR processing tools migrated from wbtools_pro.
///
/// This module currently contains:
/// - ImprovedGroundPointFilter: multi-stage ground point filtering pipeline

use serde_json::json;
use rayon::prelude::*;

/// Minimum cell count before Rayon thread dispatch pays off.
/// Below this threshold, `with_min_len` collapses the parallel iterator
/// to a single chunk, avoiding pool overhead for small LiDAR grids.
const RAYON_MIN_CHUNK: usize = 65_536;
use wbcore::{
    parse_optional_output_path, LicenseTier, Tool, ToolArgs, ToolCategory,
    ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use crate::{
    memory_store,
    tools::{FilterLidarByPercentileTool, LidarTinGriddingTool, FillPitsTool, RemoveOffTerrainObjectsTool, FilterLidarByReferenceSurfaceTool},
};

pub struct ImprovedGroundPointFilterTool;

// ── helpers ──────────────────────────────────────────────────────────────────

fn get_path(result: &ToolRunResult) -> Result<String, ToolError> {
    result
        .outputs
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::Execution("sub-tool returned no path".to_string()))
}

fn raster_mem(r: wbraster::Raster) -> String {
    let id = memory_store::put_raster(r);
    memory_store::make_raster_memory_path(&id)
}

fn load_raster(path: &str, label: &str) -> Result<wbraster::Raster, ToolError> {
    if memory_store::raster_is_memory_path(path) {
        let id = memory_store::raster_path_to_id(path)
            .ok_or_else(|| ToolError::Execution(format!("invalid memory path for '{label}'")))?;
        memory_store::get_raster_by_id(id)
            .ok_or_else(|| ToolError::Execution(format!("memory raster not found for '{label}'")))
    } else {
        wbraster::Raster::read(std::path::Path::new(path))
            .map_err(|e| ToolError::Execution(format!("failed reading '{label}': {e}")))
    }
}

// ── ImprovedGroundPointFilterTool ─────────────────────────────────────────────

impl Tool for ImprovedGroundPointFilterTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            id: "improved_ground_point_filter",
            display_name: "Improved Ground Point Filter",
            summary: "Multi-stage ground point filtering pipeline: percentile filter → TIN gridding → fill pits → remove off-terrain objects → reference surface filter.",
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input LiDAR path.", required: true },
                ToolParamSpec { name: "block_size", description: "Grid cell size in xy units for TIN gridding and percentile filter (default 1.0).", required: false },
                ToolParamSpec { name: "max_building_size", description: "Maximum expected building/object width in xy units for off-terrain object removal (default 150.0).", required: false },
                ToolParamSpec { name: "slope_threshold", description: "Minimum edge slope in degrees used for off-terrain object removal (default 15.0).", required: false },
                ToolParamSpec { name: "elev_threshold", description: "Elevation distance threshold in z units used for final reference surface filter (default 0.15).", required: false },
                ToolParamSpec { name: "classify", description: "If true, classify points as ground/non-ground instead of filtering out non-ground points (default false).", required: false },
                ToolParamSpec { name: "preserve_classes", description: "If true, preserve existing class values for non-matching points in classify mode (default false).", required: false },
                ToolParamSpec { name: "output", description: "Optional output LiDAR path. If omitted, result is auto-named.", required: false },
            ],
        }
    }

    fn manifest(&self) -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("input.las"));
        defaults.insert("block_size".to_string(), json!(1.0));
        defaults.insert("max_building_size".to_string(), json!(150.0));
        defaults.insert("slope_threshold".to_string(), json!(15.0));
        defaults.insert("elev_threshold".to_string(), json!(0.15));
        defaults.insert("classify".to_string(), json!(false));
        defaults.insert("preserve_classes".to_string(), json!(false));

        let mut example_args = defaults.clone();
        example_args.insert("output".to_string(), json!("ground.las"));

        ToolManifest {
            id: "improved_ground_point_filter".to_string(),
            display_name: "Improved Ground Point Filter".to_string(),
            summary: "Multi-stage ground point filtering pipeline.".to_string(),
            category: ToolCategory::Lidar,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input LiDAR path.".to_string(), required: true },
                ToolParamDescriptor { name: "block_size".to_string(), description: "Grid cell size in xy units.".to_string(), required: false },
                ToolParamDescriptor { name: "max_building_size".to_string(), description: "Maximum expected building width in xy units.".to_string(), required: false },
                ToolParamDescriptor { name: "slope_threshold".to_string(), description: "Minimum edge slope in degrees.".to_string(), required: false },
                ToolParamDescriptor { name: "elev_threshold".to_string(), description: "Elevation distance threshold.".to_string(), required: false },
                ToolParamDescriptor { name: "classify".to_string(), description: "Classify rather than filter points.".to_string(), required: false },
                ToolParamDescriptor { name: "preserve_classes".to_string(), description: "Preserve existing classes in classify mode.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output LiDAR path.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "igpf_basic".to_string(),
                description: "Extract ground points from a LAS file.".to_string(),
                args: example_args,
            }],
            tags: vec!["lidar".to_string(), "ground".to_string(), "filter".to_string(), "dtm".to_string(), "legacy-port".to_string()],
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
        let block_size = args.get("block_size").and_then(|v| v.as_f64()).unwrap_or(1.0).max(0.01);
        let max_building_size = args.get("max_building_size").and_then(|v| v.as_f64()).unwrap_or(150.0);
        let slope_threshold = args.get("slope_threshold").and_then(|v| v.as_f64()).unwrap_or(15.0);
        let elev_threshold = args.get("elev_threshold").and_then(|v| v.as_f64()).unwrap_or(0.15);
        let classify = args.get("classify").and_then(|v| v.as_bool()).unwrap_or(false);
        let preserve_classes = args.get("preserve_classes").and_then(|v| v.as_bool()).unwrap_or(false);
        let output_path = parse_optional_output_path(args, "output")?;

        // max_building_size expressed in cells
        let max_building_cells = (max_building_size / block_size).ceil() as usize;

        // Step 1: FilterLidarByPercentile — select lowest point per block
        ctx.progress.info("improved_ground_point_filter: step 1 – percentile filter");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(input_path));
        a.insert("percentile".to_string(), json!(0.0));
        a.insert("block_size".to_string(), json!(block_size));
        let r = FilterLidarByPercentileTool.run(&a, ctx)?;
        let grd_pts_path = get_path(&r)?;

        // Step 2: LidarTinGridding — interpolate DEM from ground candidate points
        ctx.progress.info("improved_ground_point_filter: step 2 – TIN gridding");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(grd_pts_path));
        a.insert("interpolation_parameter".to_string(), json!("elevation"));
        a.insert("returns_included".to_string(), json!("all"));
        a.insert("resolution".to_string(), json!(block_size));
        let r = LidarTinGriddingTool.run(&a, ctx)?;
        let tin_path = get_path(&r)?;

        // Step 3: FillPits — handle low noise artefacts, then conditional blend
        ctx.progress.info("improved_ground_point_filter: step 3 – fill pits");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(tin_path.clone()));
        let r = FillPitsTool.run(&a, ctx)?;
        let tin2_path = get_path(&r)?;

        // Blend: where (tin2 - tin) > 2*elev_threshold use tin2, else keep tin
        let tin = load_raster(&tin_path, "tin")?;
        let tin2 = load_raster(&tin2_path, "tin2")?;
        let rows = tin.rows;
        let cols = tin.cols;
        let band_stride = rows * cols;
        let double_thresh = elev_threshold * 2.0;
        let mut blended = tin.clone();
        let blended_values: Vec<Option<f64>> = (0..band_stride)
            .into_par_iter()
            .with_min_len(RAYON_MIN_CHUNK)
            .map(|i| {
                let v1 = tin.data.get_f64(i);
                let v2 = tin2.data.get_f64(i);
                if tin.is_nodata(v1) || tin2.is_nodata(v2) {
                    None
                } else if (v2 - v1) > double_thresh {
                    Some(v2)
                } else {
                    None
                }
            })
            .collect();
        for (i, v) in blended_values.into_iter().enumerate() {
            if let Some(z) = v {
                blended.data.set_f64(i, z);
            }
        }
        let blended_mem = raster_mem(blended);

        // Step 4: RemoveOffTerrainObjects — remove buildings/vegetation
        ctx.progress.info("improved_ground_point_filter: step 4 – remove off-terrain objects");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(blended_mem));
        a.insert("filter_size".to_string(), json!(max_building_cells));
        a.insert("slope_threshold".to_string(), json!(slope_threshold));
        let r = RemoveOffTerrainObjectsTool.run(&a, ctx)?;
        let dtm_path = get_path(&r)?;

        // Step 5: FilterLidarByReferenceSurface — extract final ground points
        ctx.progress.info("improved_ground_point_filter: step 5 – filter by reference surface");
        let mut a = ToolArgs::new();
        a.insert("input".to_string(), json!(input_path));
        a.insert("ref_surface".to_string(), json!(dtm_path));
        a.insert("query".to_string(), json!("within"));
        a.insert("threshold".to_string(), json!(elev_threshold));
        a.insert("classify".to_string(), json!(classify));
        a.insert("true_class_value".to_string(), json!(2.0));
        a.insert("false_class_value".to_string(), json!(1.0));
        a.insert("preserve_classes".to_string(), json!(preserve_classes));
        if let Some(ref p) = output_path {
            a.insert("output".to_string(), json!(p.to_string_lossy().to_string()));
        }
        let r = FilterLidarByReferenceSurfaceTool.run(&a, ctx)?;

        Ok(r)
    }
}

