use rayon::prelude::*;
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufWriter;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::Path;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba, RgbaImage};
use wide::{f32x8, CmpGt, CmpNe};
use wbcore::{PercentCoalescer, 
    parse_optional_output_path, parse_raster_path_arg, parse_vector_path_arg, LicenseTier, Tool,
    ToolArgs, ToolCategory, ToolContext, ToolError, ToolExample, ToolManifest, ToolMetadata,
    ToolParamDescriptor, ToolParamSpec, ToolRunResult, ToolStability,
};
use wbraster::{DataType, Raster, RasterConfig, RasterFormat};

use crate::memory_store;
use crate::palettes::LegacyPalette;

pub struct DifferenceFromMeanElevationTool;
pub struct DeviationFromMeanElevationTool;
pub struct StandardDeviationOfSlopeTool;
pub struct MaxDifferenceFromMeanTool;
pub struct MaxElevationDeviationTool;
pub struct TopographicPositionAnimationTool;
pub struct MultiscaleTopographicPositionImageTool;
pub struct MultiscaleElevationPercentileTool;
pub struct MaxAnisotropyDevTool;
pub struct MultiscaleRoughnessTool;
pub struct MaxElevDevSignatureTool;
pub struct MaxAnisotropyDevSignatureTool;
pub struct MultiscaleRoughnessSignatureTool;
pub struct MultiscaleStdDevNormalsTool;
pub struct MultiscaleStdDevNormalsSignatureTool;
/// Feature-preserving DEM smoothing tool.
///
/// Authors: John Lindsay and Anthony Francioni
pub struct FeaturePreservingSmoothingTool;
/// Feature-preserving DEM smoothing using screened Poisson reconstruction (alternative to
/// the iterative elevation-update variant). Normal-vector smoothing is identical; the
/// elevation reconstruction step solves a screened Poisson linear system via double-buffered
/// Jacobi iteration instead of the ad-hoc 8-neighbour tangent-plane average.
#[allow(dead_code)]
pub struct FeaturePreservingSmoothingPoissonTool;
/// Multiscale coarse-to-fine extension of feature-preserving DEM smoothing.
pub struct FeaturePreservingSmoothingMultiscaleTool;
pub struct FillMissingDataTool;
pub struct RemoveOffTerrainObjectsTool;
pub struct MapOffTerrainObjectsTool;
pub struct EmbankmentMappingTool;
pub struct SmoothVegetationResidualTool;
pub struct LocalHypsometricAnalysisTool;
pub struct MultiscaleElevatedIndexTool;
pub struct MultiscaleLowLyingIndexTool;

#[derive(Clone, Copy, Debug, PartialEq)]
struct EmbankmentCell {
    row: isize,
    col: isize,
    distance: f64,
}

impl Eq for EmbankmentCell {}

impl PartialOrd for EmbankmentCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for EmbankmentCell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

struct TerrainWindowCore;

#[derive(Clone, Copy)]
struct PoissonSmoothingSettings {
    outer_iterations: usize,
    normal_smoothing_strength: f32,
    edge_sensitivity: f32,
    lambda: f32,
    convergence_threshold: f32,
    outer_convergence_threshold: f32,
    z_factor: f32,
    use_local_adaptivity: bool,
    local_adaptivity_strength: f32,
    local_adaptivity_radius: usize,
}

#[derive(Clone)]
struct RasterPyramidLevel {
    dem: Vec<f32>,
    rows: usize,
    cols: usize,
    res_x: f32,
    res_y: f32,
}

impl TerrainWindowCore {
    fn parse_input(args: &ToolArgs) -> Result<String, ToolError> {
        parse_raster_path_arg(args, "input")
    }

    fn parse_filter_sizes(args: &ToolArgs) -> (usize, usize) {
        let mut filter_size_x = args
            .get("filter_size_x")
            .or_else(|| args.get("filterx"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);
        let mut filter_size_y = args
            .get("filter_size_y")
            .or_else(|| args.get("filtery"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(filter_size_x);

        filter_size_x = filter_size_x.max(3);
        filter_size_y = filter_size_y.max(3);
        if filter_size_x % 2 == 0 {
            filter_size_x += 1;
        }
        if filter_size_y % 2 == 0 {
            filter_size_y += 1;
        }
        (filter_size_x, filter_size_y)
    }

    fn load_raster(path: &str) -> Result<Raster, ToolError> {
        if memory_store::raster_is_memory_path(path) {
            let id = memory_store::raster_path_to_id(path).ok_or_else(|| {
                ToolError::Validation(
                    "parameter 'input' has malformed in-memory raster path".to_string(),
                )
            })?;
            return memory_store::get_raster_by_id(id).ok_or_else(|| {
                ToolError::Validation(format!(
                    "parameter 'input' references unknown in-memory raster id '{}'",
                    id
                ))
            });
        }
        Raster::read(path)
            .map_err(|e| ToolError::Execution(format!("failed reading input raster: {}", e)))
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

    fn build_result_with_scale(output_locator: String, scale_locator: String) -> ToolRunResult {
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("scale_path".to_string(), json!(scale_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn build_result_with_gif(output_locator: String, gif_locator: String) -> ToolRunResult {
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(output_locator));
        outputs.insert("gif_path".to_string(), json!(gif_locator));
        ToolRunResult {
            outputs,
            ..Default::default()
        }
    }

    fn write_animation_html(
        html_path: &Path,
        title: &str,
        heading: &str,
        label: &str,
        image_name: &str,
        width: usize,
        height: usize,
        details: &[(&str, String)],
    ) -> Result<(), ToolError> {
        if let Some(parent) = html_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }
        let details_html = details
            .iter()
            .map(|(k, v)| format!("<div><strong>{}</strong>: {}</div>", k, v))
            .collect::<Vec<_>>()
            .join("");
        let label_html = if label.trim().is_empty() {
            String::new()
        } else {
            format!("<div class='label'>{}</div>", label)
        };
        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>{}</title><style>body{{margin:0;background:#ececec;font-family:Helvetica,Arial,sans-serif;color:#111}}main{{max-width:1280px;margin:0 auto;padding:24px}}h1{{margin:0 0 12px 0;font-size:30px;font-weight:600}}.card{{background:#f3f3f3;border:1px solid #cfcfcf;border-radius:12px;padding:16px;box-shadow:0 8px 24px rgba(0,0,0,.06)}}.meta{{display:flex;flex-wrap:wrap;gap:14px;font-size:14px;margin:0 0 14px 0}}.viewer{{position:relative;display:inline-block;overflow:hidden;border:1px solid #c7c7c7;background:#fff;cursor:grab;max-width:100%}}.viewer img{{display:block;transform-origin:0 0;user-select:none;-webkit-user-drag:none}}.label{{position:absolute;left:10px;top:10px;padding:3px 8px;background:rgba(255,255,255,.8);border-radius:6px;font-size:13px}}.hint{{margin-top:10px;font-size:13px;color:#444}}</style></head><body><main><h1>{}</h1><div class='card'><div class='meta'>{}</div><div class='viewer' id='viewer' style='width:{}px;height:{}px'>{}<img id='anim' src='{}' width='{}' height='{}' alt='{} animation'></div><div class='hint'>Use the mouse wheel to zoom and drag to pan.</div></div></main><script>(function(){{const viewer=document.getElementById('viewer');const img=document.getElementById('anim');let scale=1,x=0,y=0,drag=false,sx=0,sy=0;function render(){{img.style.transform=`translate(${{x}}px,${{y}}px) scale(${{scale}})`;}}viewer.addEventListener('wheel',function(e){{e.preventDefault();const rect=viewer.getBoundingClientRect();const px=e.clientX-rect.left;const py=e.clientY-rect.top;const next=Math.min(12,Math.max(1,scale*(e.deltaY<0?1.12:1/1.12)));const ratio=next/scale;x=px-(px-x)*ratio;y=py-(py-y)*ratio;scale=next;render();}},{{passive:false}});viewer.addEventListener('mousedown',function(e){{drag=true;sx=e.clientX-x;sy=e.clientY-y;viewer.style.cursor='grabbing';}});window.addEventListener('mousemove',function(e){{if(!drag)return;x=e.clientX-sx;y=e.clientY-sy;render();}});window.addEventListener('mouseup',function(){{drag=false;viewer.style.cursor='grab';}});render();}})();</script></body></html>",
            title,
            heading,
            details_html,
            width,
            height,
            label_html,
            image_name,
            width,
            height,
            heading,
        );
        std::fs::write(html_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {e}")))
    }

    fn parse_scale_settings(args: &ToolArgs) -> (usize, usize, usize) {
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        (min_scale, max_scale, step_size)
    }

    fn feature_preserving_smoothing_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "feature_preserving_smoothing",
            display_name: "Feature Preserving Smoothing",
            summary: "Smooths DEM roughness while preserving breaks-in-slope using normal-vector filtering.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd filter size in cells (default 11).", required: false },
                ToolParamSpec { name: "normal_diff_threshold", description: "Maximum normal angle difference in degrees (default 8.0).", required: false },
                ToolParamSpec { name: "iterations", description: "Number of smoothing iterations (default 3).", required: false },
                ToolParamSpec { name: "max_elevation_diff", description: "Maximum allowed vertical change from original DEM (default inf).", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional z conversion factor (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // Feature-Preserving Smoothing – Screened Poisson variant
    // -----------------------------------------------------------------------

    #[allow(dead_code)]
    fn feature_preserving_smoothing_poisson_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("normal_smoothing_strength".to_string(), json!(0.6));
        defaults.insert("edge_sensitivity".to_string(), json!(0.7));
        defaults.insert("outer_iterations".to_string(), json!(3));
        defaults.insert("lambda".to_string(), json!(0.5));
        defaults.insert("z_factor".to_string(), json!(1.0));

        ToolManifest {
            id: "feature_preserving_smoothing_poisson".to_string(),
            display_name: "Feature Preserving Smoothing (Poisson)".to_string(),
            summary:
                "Smooths DEM roughness while preserving major breaks-in-slope. Each outer \
                 iteration re-derives surface normals from the current elevation surface, \
                 smooths the normal field using robust anisotropic diffusion, then reconstructs \
                 elevations using a screened Poisson solve (Jacobi with early stopping). \
                 Compared with bilateral normal filtering, robust diffusion supports stronger \
                 progressive generalisation while still suppressing diffusion across strong \
                 edges."
                    .to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "normal_smoothing_strength".to_string(), description: "Normal-field smoothing strength in [0,1] (default 0.6). Higher values apply more aggressive smoothing.".to_string(), required: false },
                ToolParamDescriptor { name: "edge_sensitivity".to_string(), description: "Edge preservation sensitivity in [0,1] (default 0.7). Higher values preserve edges more strongly.".to_string(), required: false },
                ToolParamDescriptor { name: "outer_iterations".to_string(), description: "Number of full outer passes (default 3).".to_string(), required: false },
                ToolParamDescriptor { name: "lambda".to_string(), description: "Data-fidelity weight in screened Poisson solve (default 0.5). Lower values increase smoothing aggressiveness.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Optional z conversion factor (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "smoothing".to_string(),
            ],
            stability: ToolStability::Stable,
        }
    }

    #[allow(dead_code)]
    fn run_feature_preserving_smoothing_poisson(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let settings = Self::parse_poisson_smoothing_settings(args);
        let input = Self::load_raster(&input_path)?;
        let nodata = input.nodata as f32;
        let dem_orig = Self::raster_to_f32_vec(&input);
        let z_cur = Self::run_poisson_smoothing_core(
            &dem_orig,
            input.rows,
            input.cols,
            nodata,
            input.cell_size_x as f32,
            input.cell_size_y as f32,
            &settings,
            None,
        );

        coalescer.emit_unit_fraction(ctx.progress, 0.9);
        let output = Self::dem_to_output_raster(&input, &z_cur, nodata)?;
        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    // -----------------------------------------------------------------------

    fn feature_preserving_smoothing_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("normal_diff_threshold".to_string(), json!(8.0));
        defaults.insert("iterations".to_string(), json!(3));
        defaults.insert("max_elevation_diff".to_string(), json!("inf"));
        defaults.insert("z_factor".to_string(), json!(1.0));

        ToolManifest {
            id: "feature_preserving_smoothing".to_string(),
            display_name: "Feature Preserving Smoothing".to_string(),
            summary: "Smooths DEM roughness while preserving breaks-in-slope using normal-vector filtering."
                .to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "smoothing".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    #[allow(dead_code)]
    fn feature_preserving_smoothing_poisson_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "feature_preserving_smoothing_poisson",
            display_name: "Feature Preserving Smoothing (Poisson)",
            summary:
                "Feature-preserving DEM smoothing with iterative normal filtering and screened \
                 Poisson elevation reconstruction.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "normal_smoothing_strength", description: "Normal-field smoothing strength in [0,1] (default 0.6).", required: false },
                ToolParamSpec { name: "edge_sensitivity", description: "Edge preservation sensitivity in [0,1] (default 0.7).", required: false },
                ToolParamSpec { name: "outer_iterations", description: "Number of full outer passes (default 3).", required: false },
                ToolParamSpec { name: "lambda", description: "Data-fidelity weight in screened Poisson solve (default 0.5).", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional z conversion factor (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn feature_preserving_smoothing_multiscale_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("smoothing_amount".to_string(), json!(0.65));
        defaults.insert("edge_preservation".to_string(), json!(0.75));
        defaults.insert("scale_levels".to_string(), json!(3));
        defaults.insert("fidelity".to_string(), json!(0.45));
        defaults.insert("z_factor".to_string(), json!(1.0));

        ToolManifest {
            id: "feature_preserving_smoothing_multiscale".to_string(),
            display_name: "Feature Preserving Smoothing (Multiscale)".to_string(),
            summary:
                "Smooths DEM roughness with a multiscale coarse-to-fine continuation. Each scale re-derives normals, applies adaptive robust normal-field diffusion, and reconstructs elevations with a screened Poisson solve."
                    .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "smoothing_amount".to_string(), description: "Overall smoothing amount in [0,1] (default 0.65). Higher values increase the diffusion budget, especially at coarse scales.".to_string(), required: false },
                ToolParamDescriptor { name: "edge_preservation".to_string(), description: "Edge preservation strength in [0,1] (default 0.75). Higher values suppress smoothing across major breaks-in-slope more strongly.".to_string(), required: false },
                ToolParamDescriptor { name: "scale_levels".to_string(), description: "Number of pyramid levels for coarse-to-fine smoothing (default 3).".to_string(), required: false },
                ToolParamDescriptor { name: "fidelity".to_string(), description: "Data-fidelity weight in screened Poisson reconstruction (default 0.45). Higher values keep the result closer to the source DEM.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Optional z conversion factor (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "smoothing".to_string(),
                "multiscale".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn feature_preserving_smoothing_multiscale_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "feature_preserving_smoothing_multiscale",
            display_name: "Feature Preserving Smoothing (Multiscale)",
            summary:
                "Multiscale coarse-to-fine feature-preserving DEM smoothing with adaptive normal diffusion and screened Poisson reconstruction.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "smoothing_amount", description: "Overall smoothing amount in [0,1] (default 0.65).", required: false },
                ToolParamSpec { name: "edge_preservation", description: "Edge preservation strength in [0,1] (default 0.75).", required: false },
                ToolParamSpec { name: "scale_levels", description: "Number of pyramid levels for coarse-to-fine smoothing (default 3).", required: false },
                ToolParamSpec { name: "fidelity", description: "Data-fidelity weight in screened Poisson reconstruction (default 0.45).", required: false },
                ToolParamSpec { name: "z_factor", description: "Optional z conversion factor (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn run_feature_preserving_smoothing_multiscale(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let smoothing_amount = args
            .get("smoothing_amount")
            .and_then(Self::parse_f32_value)
            .or_else(|| {
                args.get("normal_smoothing_strength")
                    .and_then(Self::parse_f32_value)
            })
            .unwrap_or(0.65)
            .clamp(0.0, 1.0);
        let edge_preservation = args
            .get("edge_preservation")
            .and_then(Self::parse_f32_value)
            .or_else(|| args.get("edge_sensitivity").and_then(Self::parse_f32_value))
            .or_else(|| args.get("edge_sensitive").and_then(Self::parse_f32_value))
            .unwrap_or(0.75)
            .clamp(0.0, 1.0);
        let scale_levels = args
            .get("scale_levels")
            .or_else(|| args.get("levels"))
            .and_then(Self::parse_usize_value)
            .unwrap_or(3)
            .clamp(1, 8);
        let fidelity = args
            .get("fidelity")
            .and_then(Self::parse_f32_value)
            .or_else(|| args.get("lambda").and_then(Self::parse_f32_value))
            .unwrap_or(0.45)
            .max(f32::EPSILON);
        let convergence_threshold = args
            .get("convergence_threshold")
            .and_then(Self::parse_f32_value)
            .unwrap_or(0.0001)
            .max(0.0);
        let outer_convergence_threshold = args
            .get("outer_convergence_threshold")
            .and_then(Self::parse_f32_value)
            .unwrap_or(0.0)
            .max(0.0);
        let z_factor = args
            .get("z_factor")
            .and_then(Self::parse_f32_value)
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let nodata = input.nodata as f32;
        let dem_orig = Self::raster_to_f32_vec(&input);
        let pyramid = Self::build_dem_pyramid(
            &dem_orig,
            input.rows,
            input.cols,
            input.cell_size_x as f32,
            input.cell_size_y as f32,
            nodata,
            scale_levels,
        );

        coalescer.emit_unit_fraction(ctx.progress, 0.08);

        let total_levels = pyramid.len();
        let mut prev_surface: Option<Vec<f32>> = None;
        let mut prev_rows = 0usize;
        let mut prev_cols = 0usize;

        for (level_idx, level) in pyramid.iter().rev().enumerate() {
            let level_frac = if total_levels > 1 {
                level_idx as f32 / (total_levels - 1) as f32
            } else {
                1.0
            };

            let initial_surface = if let Some(prev) = prev_surface.as_ref() {
                let mut upsampled = Self::bilinear_upsample_dem(
                    prev,
                    prev_rows,
                    prev_cols,
                    level.rows,
                    level.cols,
                    nodata,
                );
                for idx in 0..upsampled.len() {
                    if level.dem[idx] == nodata {
                        upsampled[idx] = nodata;
                    } else if upsampled[idx] == nodata || !upsampled[idx].is_finite() {
                        upsampled[idx] = level.dem[idx];
                    }
                }
                upsampled
            } else {
                level.dem.clone()
            };

            let level_settings = PoissonSmoothingSettings {
                outer_iterations: ((2.0
                    + 4.0 * smoothing_amount * (1.10 - 0.30 * level_frac))
                    .round() as usize)
                    .max(1),
                normal_smoothing_strength: (smoothing_amount * (1.20 - 0.35 * level_frac))
                    .clamp(0.0, 1.0),
                edge_sensitivity: (edge_preservation
                    + (1.0 - edge_preservation) * 0.20 * level_frac)
                    .clamp(0.0, 1.0),
                lambda: (fidelity * (0.70 + 0.60 * level_frac)).max(f32::EPSILON),
                convergence_threshold,
                outer_convergence_threshold,
                z_factor,
                use_local_adaptivity: true,
                local_adaptivity_strength: 0.75,
                local_adaptivity_radius: 3,
            };

            let result = Self::run_poisson_smoothing_core(
                &level.dem,
                level.rows,
                level.cols,
                nodata,
                level.res_x,
                level.res_y,
                &level_settings,
                Some(&initial_surface),
            );

            prev_rows = level.rows;
            prev_cols = level.cols;
            prev_surface = Some(result);

            let progress = 0.08 + 0.88 * ((level_idx + 1) as f64 / total_levels as f64);
            coalescer.emit_unit_fraction(ctx.progress, progress.min(0.98));
        }

        let final_surface = prev_surface.unwrap_or(dem_orig);
        let output = Self::dem_to_output_raster(&input, &final_surface, nodata)?;
        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn normal_angle_cos(a1: f32, b1: f32, a2: f32, b2: f32) -> f32 {
        let denom = ((a1 * a1 + b1 * b1 + 1.0) * (a2 * a2 + b2 * b2 + 1.0)).sqrt();
        if denom <= f32::EPSILON {
            1.0
        } else {
            (a1 * a2 + b1 * b2 + 1.0) / denom
        }
    }

    fn run_feature_preserving_smoothing(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let mut filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11)
            .max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }

        let mut max_norm_diff = args
            .get("normal_diff_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(8.0)
            .clamp(0.0, 90.0) as f32;
        if !max_norm_diff.is_finite() {
            max_norm_diff = 8.0;
        }
        let threshold = max_norm_diff.to_radians().cos();

        let iterations = args
            .get("iterations")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .max(1);

        let max_z_diff = args
            .get("max_elevation_diff")
            .and_then(|v| {
                if let Some(n) = v.as_f64() {
                    Some(n as f32)
                } else {
                    let s = v.as_str()?;
                    if s.eq_ignore_ascii_case("inf") || s.eq_ignore_ascii_case("infinity") {
                        Some(f32::INFINITY)
                    } else {
                        s.parse::<f32>().ok()
                    }
                }
            })
            .unwrap_or(f32::INFINITY);

        let z_factor = args
            .get("z_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata as f32;
        let res_x = input.cell_size_x as f32;
        let res_y = input.cell_size_y as f32;
        let eight_res_x = res_x * 8.0;
        let eight_res_y = res_y * 8.0;

        let mut dem = vec![nodata; rows * cols];
        for row in 0..rows {
            for col in 0..cols {
                dem[Self::idx(row, col, cols)] = input.get(0, row as isize, col as isize) as f32;
            }
        }

        // Stage 1: Generate compact normal-vector field (a, b), where c is implicitly 1.
        let mut normals_a = vec![0.0f32; rows * cols];
        let mut normals_b = vec![0.0f32; rows * cols];
        normals_a
            .par_chunks_mut(cols)
            .zip(normals_b.par_chunks_mut(cols))
            .enumerate()
            .for_each(|(row, (row_a, row_b))| {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = dem[idx];
                    if z == nodata {
                        continue;
                    }

                    let sample = |r: isize, c: isize| -> f32 {
                        if r < 0 || c < 0 || r >= rows as isize || c >= cols as isize {
                            z
                        } else {
                            let v = dem[Self::idx(r as usize, c as usize, cols)];
                            if v == nodata { z } else { v }
                        }
                    };

                    let z0 = sample(row as isize - 1, col as isize - 1) * z_factor;
                    let z1 = sample(row as isize - 1, col as isize) * z_factor;
                    let z2 = sample(row as isize - 1, col as isize + 1) * z_factor;
                    let z3 = sample(row as isize, col as isize + 1) * z_factor;
                    let z4 = sample(row as isize + 1, col as isize + 1) * z_factor;
                    let z5 = sample(row as isize + 1, col as isize) * z_factor;
                    let z6 = sample(row as isize + 1, col as isize - 1) * z_factor;
                    let z7 = sample(row as isize, col as isize - 1) * z_factor;

                    row_a[col] = -((z2 - z6) + 2.0 * (z3 - z7) + (z4 - z0)) / eight_res_x;
                    row_b[col] = -((z6 - z0) + 2.0 * (z5 - z1) + (z4 - z2)) / eight_res_y;
                }
            });
        coalescer.emit_unit_fraction(ctx.progress, 0.2);

        // Stage 2: Smooth normal vectors in a filter window.
        // Interior rows/columns are processed with SIMD (f32x8); edge cells fall back to scalar.
        let mut smooth_a = vec![0.0f32; rows * cols];
        let mut smooth_b = vec![0.0f32; rows * cols];
        let midpoint = (filter_size / 2) as isize;
        let midp = midpoint as usize;

        // Pre-compute f32x8 constants (broadcast) for use inside the parallel closure.
        let thr_v   = f32x8::splat(threshold);
        let nd_v    = f32x8::splat(nodata);
        let zero_v  = f32x8::splat(0.0_f32);
        let one_v   = f32x8::splat(1.0_f32);
        let eps_v   = f32x8::splat(f32::EPSILON);

        smooth_a
            .par_chunks_mut(cols)
            .zip(smooth_b.par_chunks_mut(cols))
            .enumerate()
            .for_each(|(row, (row_sa, row_sb))| {
                // Scalar processing for one cell; returns (smooth_a, smooth_b).
                let scalar_cell = |col: usize| -> (f32, f32) {
                    let center_idx = row * cols + col;
                    if dem[center_idx] == nodata {
                        return (0.0, 0.0);
                    }
                    let ca = normals_a[center_idx];
                    let cb = normals_b[center_idx];
                    let mut sum_w = 0.0_f32;
                    let mut sum_a = 0.0_f32;
                    let mut sum_b = 0.0_f32;
                    for oy in -midpoint..=midpoint {
                        let rr = row as isize + oy;
                        if rr < 0 || rr >= rows as isize {
                            continue;
                        }
                        for ox in -midpoint..=midpoint {
                            let cc = col as isize + ox;
                            if cc < 0 || cc >= cols as isize {
                                continue;
                            }
                            let n_idx = rr as usize * cols + cc as usize;
                            if dem[n_idx] == nodata {
                                continue;
                            }
                            let cosine = Self::normal_angle_cos(ca, cb, normals_a[n_idx], normals_b[n_idx]);
                            if cosine > threshold {
                                let w = (cosine - threshold) * (cosine - threshold);
                                sum_w += w;
                                sum_a += normals_a[n_idx] * w;
                                sum_b += normals_b[n_idx] * w;
                            }
                        }
                    }
                    if sum_w > 0.0 { (sum_a / sum_w, sum_b / sum_w) } else { (0.0, 0.0) }
                };

                // SIMD path only when every neighbour in the filter window is guaranteed in-bounds.
                // Row condition: row in [midp, rows - midp).
                // Column condition: col in [midp, cols - midp), processed in chunks of 8.
                let row_interior = row >= midp && row + midp < rows;

                // SIMD-eligible column range: col such that col + 8 <= cols - midp.
                // (ensures col + 7 + midp < cols, i.e. rightmost neighbour always in-bounds)
                // and col >= midp (leftmost neighbour always in-bounds).
                let simd_end = if cols >= midp * 2 + 8 { cols - midp } else { midp };

                if row_interior && simd_end > midp {
                    // Leading edge: columns [0, midp) — scalar
                    for col in 0..midp {
                        let (sa, sb) = scalar_cell(col);
                        row_sa[col] = sa;
                        row_sb[col] = sb;
                    }

                    // SIMD interior columns in steps of 8
                    let mut col = midp;
                    while col + 8 <= simd_end {
                        let cbase = row * cols + col;

                        // Load 8 centre normals and DEM values
                        let ca_v: f32x8 = f32x8::new(
                            normals_a[cbase..cbase + 8].try_into().unwrap(),
                        );
                        let cb_v: f32x8 = f32x8::new(
                            normals_b[cbase..cbase + 8].try_into().unwrap(),
                        );
                        let cdem: f32x8 = f32x8::new(
                            dem[cbase..cbase + 8].try_into().unwrap(),
                        );
                        // Mask: lane is "live" when centre DEM ≠ nodata
                        let c_valid = cdem.simd_ne(nd_v);
                        // |n_centre|² = a²+b²+1 (implicit c=1)
                        let c_mag = ca_v * ca_v + cb_v * cb_v + one_v;

                        let mut sw = zero_v;
                        let mut sa = zero_v;
                        let mut sb = zero_v;

                        for oy in -midpoint..=midpoint {
                            let rr = (row as isize + oy) as usize; // safe: row_interior
                            for ox in -midpoint..=midpoint {
                                // nbase maps each of the 8 centre columns to its neighbour column
                                let nbase = rr * cols + (col as isize + ox) as usize;

                                let na: f32x8 = f32x8::new(
                                    normals_a[nbase..nbase + 8].try_into().unwrap(),
                                );
                                let nb: f32x8 = f32x8::new(
                                    normals_b[nbase..nbase + 8].try_into().unwrap(),
                                );
                                let ndem: f32x8 = f32x8::new(
                                    dem[nbase..nbase + 8].try_into().unwrap(),
                                );
                                let n_valid = ndem.simd_ne(nd_v);

                                // Cosine similarity between centre and neighbour normals
                                let n_mag  = na * na + nb * nb + one_v;
                                let dot    = ca_v * na + cb_v * nb + one_v;
                                let denom  = (c_mag * n_mag).sqrt().max(eps_v);
                                let cosine = dot / denom;

                                // Weight: (cosine − threshold)² where cosine > threshold AND both valid
                                let above  = cosine.simd_gt(thr_v);
                                let mask   = above & c_valid & n_valid;
                                let diff   = cosine - thr_v;
                                let w      = mask.blend(diff * diff, zero_v);

                                sw += w;
                                sa += na * w;
                                sb += nb * w;
                            }
                        }

                        // Divide and zero-out nodata centre lanes
                        let has_w  = sw.simd_gt(zero_v);
                        let inv_sw = one_v / sw.max(eps_v);
                        let out_a  = c_valid.blend(has_w.blend(sa * inv_sw, zero_v), zero_v);
                        let out_b  = c_valid.blend(has_w.blend(sb * inv_sw, zero_v), zero_v);

                        row_sa[col..col + 8].copy_from_slice(&<[f32; 8]>::from(out_a));
                        row_sb[col..col + 8].copy_from_slice(&<[f32; 8]>::from(out_b));

                        col += 8;
                    }

                    // Trailing edge: remaining columns [col, cols) — scalar
                    for c in col..cols {
                        let (sa, sb) = scalar_cell(c);
                        row_sa[c] = sa;
                        row_sb[c] = sb;
                    }
                } else {
                    // Boundary row: pure scalar
                    for col in 0..cols {
                        let (sa, sb) = scalar_cell(col);
                        row_sa[col] = sa;
                        row_sb[col] = sb;
                    }
                }
            });
        coalescer.emit_unit_fraction(ctx.progress, 0.5);

        // Stage 3: Iterative update with ping-pong buffers (no full clone each loop).
        let original = dem.clone();
        let mut current = dem;
        let mut next = vec![nodata; rows * cols];

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let x = [-res_x, -res_x, -res_x, 0.0, res_x, res_x, res_x, 0.0];
        let y = [-res_y, 0.0, res_y, res_y, res_y, 0.0, -res_y, -res_y];

        for it in 0..iterations {
            let current_ref = &current;
            next.par_chunks_mut(cols)
                .enumerate()
                .for_each(|(row, row_next)| {
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        let z0 = original[idx];
                        if z0 == nodata {
                            row_next[col] = nodata;
                            continue;
                        }

                        let mut sum_w = 0.0f32;
                        let mut sum_z = 0.0f32;
                        let ca = smooth_a[idx];
                        let cb = smooth_b[idx];

                        for n in 0..8 {
                            let rr = row as isize + dy[n];
                            let cc = col as isize + dx[n];
                            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                continue;
                            }
                            let n_idx = Self::idx(rr as usize, cc as usize, cols);
                            if original[n_idx] == nodata {
                                continue;
                            }
                            let cosine = Self::normal_angle_cos(ca, cb, smooth_a[n_idx], smooth_b[n_idx]);
                            if cosine > threshold {
                                let w = (cosine - threshold).powi(2);
                                sum_w += w;
                                sum_z +=
                                    (-(smooth_a[n_idx] * x[n] + smooth_b[n_idx] * y[n] - current_ref[n_idx])) * w;
                            }
                        }

                        row_next[col] = if sum_w > 0.0 {
                            let z_new = sum_z / sum_w;
                            if (z_new - z0).abs() <= max_z_diff { z_new } else { z0 }
                        } else {
                            z0
                        };
                    }
                });

            std::mem::swap(&mut current, &mut next);
            ctx.progress
                .progress(0.5 + ((it + 1) as f64 / iterations as f64) * 0.5);
        }

        let mut output = input.clone();
        let output_rows: Vec<Vec<f64>> = (0..rows)
            .into_par_iter()
            .map(|row| {
                let mut row_data = vec![nodata as f64; cols];
                for col in 0..cols {
                    row_data[col] = current[Self::idx(row, col, cols)] as f64;
                }
                row_data
            })
            .collect();
        for (row, row_data) in output_rows.iter().enumerate() {
            output
                .set_row_slice(0, row as isize, row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn fill_missing_data_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "fill_missing_data",
            display_name: "Fill Missing Data",
            summary: "Fills NoData gaps using inverse-distance weighting from valid gap-edge cells.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Search radius in grid cells (default 11).", required: false },
                ToolParamSpec { name: "weight", description: "Inverse-distance power exponent (default 2.0).", required: false },
                ToolParamSpec { name: "exclude_edge_nodata", description: "Exclude edge-connected NoData regions from filling (default false). Alias: no_edges.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn remove_off_terrain_objects_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "remove_off_terrain_objects",
            display_name: "Remove Off-Terrain Objects",
            summary: "Removes steep off-terrain objects from DEMs using white top-hat normalization, slope-constrained region growing, and local interpolation.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "filter_size",
                    description: "Maximum expected object width in cells; coerced to odd size >= 3 (default 11).",
                    required: false,
                },
                ToolParamSpec {
                    name: "slope_threshold",
                    description: "Minimum object edge slope in degrees (default 15.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, result stays in memory.",
                    required: false,
                },
            ],
        }
    }

    fn remove_off_terrain_objects_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("slope_threshold".to_string(), json!(15.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("filter_size".to_string(), json!(11));
        example_args.insert("slope_threshold".to_string(), json!(15.0));
        example_args.insert("output".to_string(), json!("bare_earth_dem.tif"));

        ToolManifest {
            id: "remove_off_terrain_objects".to_string(),
            display_name: "Remove Off-Terrain Objects".to_string(),
            summary: "Removes steep off-terrain objects from DEMs using white top-hat normalization, slope-constrained region growing, and local interpolation.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "filter_size".to_string(),
                    description:
                        "Maximum expected object width in cells; coerced to odd size >= 3 (default 11)."
                            .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "slope_threshold".to_string(),
                    description: "Minimum object edge slope in degrees (default 15.0)."
                        .to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result stays in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_remove_off_terrain_objects".to_string(),
                description: "Generate a bare-earth DEM from a surface DEM with buildings and vegetation residuals.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "dem".to_string(),
                "smoothing".to_string(),
                "lidar".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn map_off_terrain_objects_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "map_off_terrain_objects",
            display_name: "Map Off-Terrain Objects",
            summary: "Maps off-terrain object segments in DSMs using slope-constrained region growing and optional minimum feature-size filtering.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DSM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "max_slope",
                    description: "Maximum connecting slope in degrees [1, 90] (default inf, clamped to 90).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_feature_size",
                    description: "Minimum retained segment size in cells (default 0). Smaller segments are assigned to background class 1.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path. If omitted, result stays in memory.",
                    required: false,
                },
            ],
        }
    }

    fn map_off_terrain_objects_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dsm.tif"));
        defaults.insert("max_slope".to_string(), json!(90.0));
        defaults.insert("min_feature_size".to_string(), json!(0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dsm.tif"));
        example_args.insert("max_slope".to_string(), json!(15.0));
        example_args.insert("min_feature_size".to_string(), json!(8));
        example_args.insert("output".to_string(), json!("mapped_oto.tif"));

        ToolManifest {
            id: "map_off_terrain_objects".to_string(),
            display_name: "Map Off-Terrain Objects".to_string(),
            summary: "Maps off-terrain object segments in DSMs using slope-constrained region growing and optional minimum feature-size filtering.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DSM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "max_slope".to_string(),
                    description: "Maximum connecting slope in degrees [1, 90] (default inf, clamped to 90).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "min_feature_size".to_string(),
                    description: "Minimum retained segment size in cells (default 0). Smaller segments are assigned to background class 1.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path. If omitted, result stays in memory.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_map_off_terrain_objects".to_string(),
                description: "Map connected off-terrain objects in a surface model.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "dsm".to_string(),
                "segmentation".to_string(),
                "lidar".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn embankment_mapping_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "embankment_mapping",
            display_name: "Embankment Mapping",
            summary: "Maps transportation embankments from a DEM and road network, with optional embankment-surface removal via interpolation. Authored by John Lindsay and Nigel VanNieuwenhuizen.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec {
                    name: "dem",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "roads_vector",
                    description: "Input roads vector path (polyline geometry).",
                    required: true,
                },
                ToolParamSpec {
                    name: "search_dist",
                    description: "Road seed reposition distance in map units (default 2.5).",
                    required: false,
                },
                ToolParamSpec {
                    name: "min_road_width",
                    description: "Minimum mapped road width in map units (default 6.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "typical_embankment_width",
                    description: "Typical embankment width in map units (default 30.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "typical_embankment_max_height",
                    description: "Typical embankment max height in elevation units (default 2.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "embankment_max_width",
                    description: "Maximum embankment width in map units (default 60.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "max_upwards_increment",
                    description: "Maximum permitted local upward increment during growth (default 0.05).",
                    required: false,
                },
                ToolParamSpec {
                    name: "spillout_slope",
                    description: "Maximum spillout slope in degrees for uphill transitions (default 4.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "remove_embankments",
                    description: "If true, additionally outputs an embankment-removed DEM via local interpolation.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional embankment mask output path.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_dem",
                    description: "Optional embankment-removed DEM output path when remove_embankments=true.",
                    required: false,
                },
            ],
        }
    }

    fn embankment_mapping_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("dem".to_string(), json!("dem.tif"));
        defaults.insert("roads_vector".to_string(), json!("roads.shp"));
        defaults.insert("search_dist".to_string(), json!(2.5));
        defaults.insert("min_road_width".to_string(), json!(6.0));
        defaults.insert("typical_embankment_width".to_string(), json!(30.0));
        defaults.insert("typical_embankment_max_height".to_string(), json!(2.0));
        defaults.insert("embankment_max_width".to_string(), json!(60.0));
        defaults.insert("max_upwards_increment".to_string(), json!(0.05));
        defaults.insert("spillout_slope".to_string(), json!(4.0));
        defaults.insert("remove_embankments".to_string(), json!(false));
        defaults.insert("output".to_string(), json!("embankments.tif"));

        ToolManifest {
            id: "embankment_mapping".to_string(),
            display_name: "Embankment Mapping".to_string(),
            summary: "Maps transportation embankments from a DEM and road network, with optional embankment-surface removal via interpolation. Authored by John Lindsay and Nigel VanNieuwenhuizen.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "lidar".to_string(),
                "embankment".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn collect_line_strings(geom: &wbvector::Geometry, lines: &mut Vec<Vec<wbvector::Coord>>) {
        match geom {
            wbvector::Geometry::LineString(coords) => {
                if coords.len() >= 2 {
                    lines.push(coords.clone());
                }
            }
            wbvector::Geometry::MultiLineString(parts) => {
                for part in parts {
                    if part.len() >= 2 {
                        lines.push(part.clone());
                    }
                }
            }
            wbvector::Geometry::GeometryCollection(geoms) => {
                for g in geoms {
                    Self::collect_line_strings(g, lines);
                }
            }
            _ => {}
        }
    }

    fn rasterize_line_to_grid(
        roads: &mut [u8],
        rows: usize,
        cols: usize,
        dem: &Raster,
        a: &wbvector::Coord,
        b: &wbvector::Coord,
    ) {
        let x0 = (a.x - dem.x_min) / dem.cell_size_x;
        let y0 = (dem.y_max() - a.y) / dem.cell_size_y;
        let x1 = (b.x - dem.x_min) / dem.cell_size_x;
        let y1 = (dem.y_max() - b.y) / dem.cell_size_y;

        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).ceil() as usize;
        if steps == 0 {
            let rr = y0.round() as isize;
            let cc = x0.round() as isize;
            if rr >= 0 && cc >= 0 && rr < rows as isize && cc < cols as isize {
                roads[Self::idx(rr as usize, cc as usize, cols)] = 1;
            }
            return;
        }

        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let rr = (y0 + t * dy).round() as isize;
            let cc = (x0 + t * dx).round() as isize;
            if rr >= 0 && cc >= 0 && rr < rows as isize && cc < cols as isize {
                roads[Self::idx(rr as usize, cc as usize, cols)] = 1;
            }
        }
    }

    fn run_embankment_mapping(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let dem_path = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
        let roads_path = parse_vector_path_arg(args, "roads_vector")
            .or_else(|_| parse_vector_path_arg(args, "road_vec"))
            .or_else(|_| parse_vector_path_arg(args, "roads"))?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_dem_path = parse_optional_output_path(args, "output_dem")?;

        let search_dist = args
            .get("search_dist")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.5);
        let mut min_road_width = args
            .get("min_road_width")
            .and_then(|v| v.as_f64())
            .unwrap_or(6.0);
        let mut typical_width = args
            .get("typical_embankment_width")
            .or_else(|| args.get("typical_width"))
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0);
        let max_height = args
            .get("typical_embankment_max_height")
            .or_else(|| args.get("max_height"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        let mut max_width = args
            .get("embankment_max_width")
            .or_else(|| args.get("max_width"))
            .and_then(|v| v.as_f64())
            .unwrap_or(60.0);
        let max_increment = args
            .get("max_upwards_increment")
            .or_else(|| args.get("max_increment"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.05);
        let spillout_slope = args
            .get("spillout_slope")
            .and_then(|v| v.as_f64())
            .unwrap_or(4.0);
        let remove_embankments = args
            .get("remove_embankments")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !search_dist.is_finite() || search_dist <= 0.0 {
            return Err(ToolError::Validation(
                "parameter 'search_dist' must be > 0".to_string(),
            ));
        }
        if min_road_width > typical_width {
            return Err(ToolError::Validation(
                "parameter 'min_road_width' must be <= 'typical_embankment_width'".to_string(),
            ));
        }
        if typical_width > max_width {
            return Err(ToolError::Validation(
                "parameter 'typical_embankment_width' must be <= 'embankment_max_width'"
                    .to_string(),
            ));
        }
        if max_increment < 0.0 || max_height < 0.0 {
            return Err(ToolError::Validation(
                "parameters 'max_upwards_increment' and 'typical_embankment_max_height' must be >= 0"
                    .to_string(),
            ));
        }

        min_road_width *= 0.5;
        typical_width *= 0.5;
        max_width *= 0.5;

        let dem = Self::load_raster(&dem_path)?;
        let roads_layer = wbvector::read(&roads_path).map_err(|e| {
            ToolError::Execution(format!("failed reading roads vector '{}': {}", roads_path, e))
        })?;
        let rows = dem.rows;
        let cols = dem.cols;
        let nodata = dem.nodata;

        let mut roads = vec![0u8; rows * cols];
        for feature in &roads_layer.features {
            if let Some(geom) = &feature.geometry {
                let mut parts = Vec::new();
                Self::collect_line_strings(geom, &mut parts);
                for part in parts {
                    for i in 0..part.len() - 1 {
                        Self::rasterize_line_to_grid(&mut roads, rows, cols, &dem, &part[i], &part[i + 1]);
                    }
                }
            }
        }

        let res_x = dem.cell_size_x.abs();
        let res_y = dem.cell_size_y.abs();
        let res_diag = (res_x * res_x + res_y * res_y).sqrt();
        let dist_array = [res_diag, res_x, res_diag, res_y, res_diag, res_x, res_diag, res_y];
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];

        let mut mask = vec![nodata; rows * cols];
        let mut distance = vec![-1.0_f64; rows * cols];
        let mut seed_elev = vec![-1.0_f64; rows * cols];
        let mut max_abs_slope = vec![0.0_f64; rows * cols];

        let mut seed_search = search_dist / res_x;
        if seed_search < 1.0 {
            seed_search = 1.0;
        }
        if (seed_search as usize).is_multiple_of(2) {
            seed_search += 1.0;
        }
        let kernel = seed_search as isize;
        let midpoint = kernel / 2;

        let mut pqueue_dist = BinaryHeap::new();
        let mut pqueue = BinaryHeap::new();

        for row in 0..rows {
            for col in 0..cols {
                let idx = Self::idx(row, col, cols);
                if roads[idx] == 0 {
                    continue;
                }

                let mut maxval = dem.get(0, row as isize, col as isize);
                let mut max_point = (row as isize, col as isize);
                for kr in -midpoint..=midpoint {
                    for kc in -midpoint..=midpoint {
                        let rr = row as isize + kr;
                        let cc = col as isize + kc;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let ni = Self::idx(rr as usize, cc as usize, cols);
                        if roads[ni] > 0 {
                            continue;
                        }
                        let z = dem.get(0, rr, cc);
                        if dem.is_nodata(z) {
                            continue;
                        }
                        if z > maxval && mask[ni] != 1.0 {
                            maxval = z;
                            max_point = (rr, cc);
                        }
                    }
                }

                let mi = Self::idx(max_point.0 as usize, max_point.1 as usize, cols);
                mask[mi] = 1.0;
                distance[mi] = 0.0;
                seed_elev[mi] = maxval;
                max_abs_slope[mi] = 0.0;
                let seed_cell = EmbankmentCell {
                    row: max_point.0,
                    col: max_point.1,
                    distance: 0.0,
                };
                pqueue_dist.push(seed_cell);
                pqueue.push(seed_cell);
            }
        }

        while let Some(cell) = pqueue_dist.pop() {
            let r = cell.row;
            let c = cell.col;
            let seed_z = seed_elev[Self::idx(r as usize, c as usize, cols)];
            for n in 0..8 {
                let rr = r + dy[n];
                let cc = c + dx[n];
                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                    continue;
                }
                let ni = Self::idx(rr as usize, cc as usize, cols);
                if distance[ni] >= 0.0 {
                    continue;
                }
                let zn = dem.get(0, rr, cc);
                if dem.is_nodata(zn) {
                    continue;
                }
                let dist = cell.distance + dist_array[n];
                if dist < max_width {
                    distance[ni] = dist;
                    seed_elev[ni] = seed_z;
                    let embankment_height = seed_z - zn;
                    let embankment_slope = (embankment_height / dist.max(f64::EPSILON)).atan();
                    let pi = Self::idx(r as usize, c as usize, cols);
                    max_abs_slope[ni] = embankment_slope
                        .abs()
                        .to_degrees()
                        .max(max_abs_slope[pi]);
                    pqueue_dist.push(EmbankmentCell {
                        row: rr,
                        col: cc,
                        distance: dist,
                    });
                }
            }
        }

        while let Some(cell) = pqueue.pop() {
            let r = cell.row;
            let c = cell.col;
            let z = dem.get(0, r, c);
            if dem.is_nodata(z) {
                continue;
            }

            for n in 0..8 {
                let rr = r + dy[n];
                let cc = c + dx[n];
                if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                    continue;
                }
                let ni = Self::idx(rr as usize, cc as usize, cols);
                if mask[ni] != nodata {
                    continue;
                }
                let zn = dem.get(0, rr, cc);
                if dem.is_nodata(zn) {
                    continue;
                }
                let dist = distance[ni];
                if dist < 0.0 || dist > max_width {
                    continue;
                }

                let mut accept = false;
                if dist <= min_road_width {
                    accept = true;
                } else {
                    let seed_z = seed_elev[ni];
                    let embankment_height = seed_z - zn;
                    let embankment_slope = (embankment_height / dist.max(f64::EPSILON)).atan().to_degrees();

                    if dist <= typical_width
                        && z - zn > -max_increment
                        && embankment_height <= max_height
                    {
                        if zn <= z
                            || (zn > z && max_abs_slope[ni] < spillout_slope)
                        {
                            accept = true;
                        }
                    } else if max_abs_slope[ni] - embankment_slope.abs() <= 1.0
                        && embankment_slope >= 0.0
                    {
                        accept = true;
                    }
                }

                if accept {
                    mask[ni] = 1.0;
                    pqueue.push(EmbankmentCell {
                        row: rr,
                        col: cc,
                        distance: dist,
                    });
                }
            }
        }

        let mut output_mask = dem.clone();
        for r in 0..rows {
            let start = Self::idx(r, 0, cols);
            let end = start + cols;
            output_mask
                .set_row_slice(0, r as isize, &mask[start..end])
                .map_err(|e| ToolError::Execution(format!("failed writing embankment mask row {}: {}", r, e)))?;
            coalescer.emit_unit_fraction(ctx.progress, (r + 1) as f64 / rows as f64);
        }

        let mask_locator = Self::write_or_store_output(output_mask, output_path)?;
        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(mask_locator));

        if remove_embankments {
            let mut edge_cells: Vec<(usize, usize, f64)> = Vec::new();
            for r in 0..rows {
                for c in 0..cols {
                    let i = Self::idx(r, c, cols);
                    if mask[i] < 1.0 || dem.is_nodata(dem.get(0, r as isize, c as isize)) {
                        continue;
                    }
                    let mut edge = false;
                    for n in 0..8 {
                        let rr = r as isize + dy[n];
                        let cc = c as isize + dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let ni = Self::idx(rr as usize, cc as usize, cols);
                        let zv = dem.get(0, rr, cc);
                        if (mask[ni] == nodata || mask[ni] < 1.0) && !dem.is_nodata(zv) {
                            edge = true;
                            break;
                        }
                    }
                    if edge {
                        edge_cells.push((r, c, dem.get(0, r as isize, c as isize)));
                    }
                }
            }

            let mut out_dem = dem.clone();
            let radius_cells = (max_width / res_x.max(res_y)).ceil() as isize;
            let radius2 = max_width * max_width;
            for r in 0..rows {
                let y = dem.row_center_y(r as isize);
                for c in 0..cols {
                    let i = Self::idx(r, c, cols);
                    if mask[i] < 1.0 {
                        continue;
                    }
                    let x = dem.col_center_x(c as isize);

                    let r0 = (r as isize - radius_cells).max(0) as usize;
                    let r1 = (r as isize + radius_cells).min(rows as isize - 1) as usize;
                    let c0 = (c as isize - radius_cells).max(0) as usize;
                    let c1 = (c as isize + radius_cells).min(cols as isize - 1) as usize;

                    let mut sum_w = 0.0;
                    let mut sum_z = 0.0;
                    for &(er, ec, ez) in &edge_cells {
                        if er < r0 || er > r1 || ec < c0 || ec > c1 {
                            continue;
                        }
                        let ex = dem.col_center_x(ec as isize);
                        let ey = dem.row_center_y(er as isize);
                        let dxm = ex - x;
                        let dym = ey - y;
                        let d2 = dxm * dxm + dym * dym;
                        if d2 <= f64::EPSILON || d2 > radius2 {
                            continue;
                        }
                        let w = 1.0 / d2;
                        sum_w += w;
                        sum_z += ez * w;
                    }

                    if sum_w > 0.0 {
                        out_dem
                            .set(0, r as isize, c as isize, sum_z / sum_w)
                            .map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing embankment-removed DEM cell: {}",
                                    e
                                ))
                            })?;
                    } else {
                        out_dem
                            .set(0, r as isize, c as isize, nodata)
                            .map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing embankment-removed DEM cell: {}",
                                    e
                                ))
                            })?;
                    }
                }
            }

            let dem_locator = Self::write_or_store_output(out_dem, output_dem_path)?;
            outputs.insert("output_dem".to_string(), json!(dem_locator));
        }

        coalescer.finish(ctx.progress);
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn fill_missing_data_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("weight".to_string(), json!(2.0));
        defaults.insert("exclude_edge_nodata".to_string(), json!(false));

        ToolManifest {
            id: "fill_missing_data".to_string(),
            display_name: "Fill Missing Data".to_string(),
            summary: "Fills NoData gaps using inverse-distance weighting from valid gap-edge cells."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "gap-filling".to_string(),
                "idw".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn smooth_vegetation_residual_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "smooth_vegetation_residual",
            display_name: "Smooth Vegetation Residual",
            summary: "Reduces canopy residual roughness by masking high local DEV responses at small scales and re-interpolating masked elevations.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "max_scale", description: "Maximum DEV half-window radius in cells (default 30).", required: false },
                ToolParamSpec { name: "dev_threshold", description: "Minimum DEV magnitude used to flag roughness cells (default 1.0).", required: false },
                ToolParamSpec { name: "scale_threshold", description: "Maximum scale considered roughness (default 5).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn smooth_vegetation_residual_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("max_scale".to_string(), json!(30));
        defaults.insert("dev_threshold".to_string(), json!(1.0));
        defaults.insert("scale_threshold".to_string(), json!(5));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("max_scale".to_string(), json!(30));
        example_args.insert("dev_threshold".to_string(), json!(1.0));
        example_args.insert("scale_threshold".to_string(), json!(5));
        example_args.insert("output".to_string(), json!("smooth_vegetation_residual.tif"));

        ToolManifest {
            id: "smooth_vegetation_residual".to_string(),
            display_name: "Smooth Vegetation Residual".to_string(),
            summary: "Reduces canopy residual roughness by masking high local DEV responses at small scales and re-interpolating masked elevations.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum DEV half-window radius in cells (default 30).".to_string(), required: false },
                ToolParamDescriptor { name: "dev_threshold".to_string(), description: "Minimum DEV magnitude used to flag roughness cells (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "scale_threshold".to_string(), description: "Maximum scale considered roughness (default 5).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_smooth_vegetation_residual".to_string(),
                description: "Suppress vegetation-residual roughness in a LiDAR DEM.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "lidar".to_string(),
                "smoothing".to_string(),
                "dem".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn run_fill_missing_data(args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as isize)
            .unwrap_or(11)
            .max(1);
        let mut weight = args
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0);
        if !weight.is_finite() || weight <= 0.0 {
            weight = 2.0;
        }
        let exclude_edge_nodata = args
            .get("exclude_edge_nodata")
            .or_else(|| args.get("no_edges"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let mut output = input.clone();

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut vals = vec![nodata; rows * cols];
            for row in 0..rows {
                for col in 0..cols {
                    vals[Self::idx(row, col, cols)] = input.get(band, row as isize, col as isize);
                }
            }

            let mut edge_excluded = vec![false; rows * cols];
            if exclude_edge_nodata {
                let mut stack: Vec<(usize, usize)> = Vec::new();
                for row in 0..rows {
                    let li = Self::idx(row, 0, cols);
                    if vals[li] == nodata && !edge_excluded[li] {
                        edge_excluded[li] = true;
                        stack.push((row, 0));
                    }
                    let ri = Self::idx(row, cols - 1, cols);
                    if vals[ri] == nodata && !edge_excluded[ri] {
                        edge_excluded[ri] = true;
                        stack.push((row, cols - 1));
                    }
                }
                for col in 0..cols {
                    let ti = Self::idx(0, col, cols);
                    if vals[ti] == nodata && !edge_excluded[ti] {
                        edge_excluded[ti] = true;
                        stack.push((0, col));
                    }
                    let bi = Self::idx(rows - 1, col, cols);
                    if vals[bi] == nodata && !edge_excluded[bi] {
                        edge_excluded[bi] = true;
                        stack.push((rows - 1, col));
                    }
                }

                while let Some((r, c)) = stack.pop() {
                    for n in 0..8 {
                        let rr = r as isize + dy[n];
                        let cc = c as isize + dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let i = Self::idx(rr as usize, cc as usize, cols);
                        if vals[i] == nodata && !edge_excluded[i] {
                            edge_excluded[i] = true;
                            stack.push((rr as usize, cc as usize));
                        }
                    }
                }
            }

            // Valid cells adjacent to an interior NoData cell are interpolation seed points.
            let mut seed = vec![false; rows * cols];
            for row in 0..rows {
                for col in 0..cols {
                    let i = Self::idx(row, col, cols);
                    if vals[i] == nodata {
                        continue;
                    }
                    for n in 0..8 {
                        let rr = row as isize + dy[n];
                        let cc = col as isize + dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let ni = Self::idx(rr as usize, cc as usize, cols);
                        if vals[ni] == nodata && !edge_excluded[ni] {
                            seed[i] = true;
                            break;
                        }
                    }
                }
            }

            let mut out_vals = vals.clone();
            let radius = filter_size as f64;

            for row in 0..rows {
                for col in 0..cols {
                    let i = Self::idx(row, col, cols);
                    if vals[i] != nodata || edge_excluded[i] {
                        continue;
                    }

                    let r0 = (row as isize - filter_size).max(0) as usize;
                    let r1 = (row as isize + filter_size).min(rows as isize - 1) as usize;
                    let c0 = (col as isize - filter_size).max(0) as usize;
                    let c1 = (col as isize + filter_size).min(cols as isize - 1) as usize;

                    let mut sum_w = 0.0;
                    let mut sum_z = 0.0;
                    for rr in r0..=r1 {
                        let dr = rr as isize - row as isize;
                        for cc in c0..=c1 {
                            let ni = Self::idx(rr, cc, cols);
                            if !seed[ni] {
                                continue;
                            }
                            let dc = cc as isize - col as isize;
                            let dist = ((dr * dr + dc * dc) as f64).sqrt();
                            if dist <= 0.0 || dist > radius {
                                continue;
                            }
                            let w = if (weight - 1.0).abs() < f64::EPSILON {
                                1.0 / dist
                            } else if (weight - 2.0).abs() < f64::EPSILON {
                                1.0 / (dist * dist)
                            } else {
                                1.0 / dist.powf(weight)
                            };
                            sum_w += w;
                            sum_z += vals[ni] * w;
                        }
                    }

                    if sum_w > 0.0 {
                        out_vals[i] = sum_z / sum_w;
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + (row + 1) as f64 / rows as f64) / bands as f64,
                );
            }

            for row in 0..rows {
                let start = Self::idx(row, 0, cols);
                let end = start + cols;
                output
                    .set_row_slice(band, row as isize, &out_vals[start..end])
                    .map_err(|e| {
                        ToolError::Execution(format!("failed writing row {}: {}", row, e))
                    })?;
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_remove_off_terrain_objects(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let mut filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11)
            .max(3);
        if filter_size % 2 == 0 {
            filter_size += 1;
        }

        let mut slope_threshold = args
            .get("slope_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(15.0);
        if !slope_threshold.is_finite() {
            slope_threshold = 15.0;
        }

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let midpoint = (filter_size / 2) as isize;
        let cell_size_x = input.cell_size_x.abs();
        let cell_size_y = input.cell_size_y.abs();
        let cell_size_diag = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let slope = slope_threshold.to_radians().tan();
        let height_diff_threshold = [
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y,
            slope * cell_size_diag,
            slope * cell_size_x,
            slope * cell_size_diag,
            slope * cell_size_y,
        ];
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let initial = f64::NEG_INFINITY;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running remove_off_terrain_objects");

            let mut vals = vec![nodata; rows * cols];
            for r in 0..rows {
                for c in 0..cols {
                    vals[Self::idx(r, c, cols)] = input.get(band, r as isize, c as isize);
                }
            }

            // White top-hat transform = original - opening(opening = dilation(erosion(original))).
            let mut erosion = vec![nodata; rows * cols];
            for r in 0..rows {
                let r_i = r as isize;
                for c in 0..cols {
                    let c_i = c as isize;
                    let z = vals[Self::idx(r, c, cols)];
                    if z == nodata {
                        continue;
                    }
                    let mut min_val = f64::INFINITY;
                    for rr in (r_i - midpoint).max(0)..=(r_i + midpoint).min(rows as isize - 1) {
                        for cc in (c_i - midpoint).max(0)..=(c_i + midpoint).min(cols as isize - 1)
                        {
                            let zv = vals[Self::idx(rr as usize, cc as usize, cols)];
                            if zv != nodata && zv < min_val {
                                min_val = zv;
                            }
                        }
                    }
                    if min_val.is_finite() {
                        erosion[Self::idx(r, c, cols)] = min_val;
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + ((r + 1) as f64 / rows as f64) * 0.2) / bands as f64,
                );
            }

            let mut opening = vec![nodata; rows * cols];
            let mut tophat = vec![nodata; rows * cols];
            for r in 0..rows {
                let r_i = r as isize;
                for c in 0..cols {
                    let c_i = c as isize;
                    let z = vals[Self::idx(r, c, cols)];
                    if z == nodata {
                        continue;
                    }
                    let mut max_val = f64::NEG_INFINITY;
                    for rr in (r_i - midpoint).max(0)..=(r_i + midpoint).min(rows as isize - 1) {
                        for cc in (c_i - midpoint).max(0)..=(c_i + midpoint).min(cols as isize - 1)
                        {
                            let ev = erosion[Self::idx(rr as usize, cc as usize, cols)];
                            if ev != nodata && ev > max_val {
                                max_val = ev;
                            }
                        }
                    }
                    if max_val > f64::NEG_INFINITY {
                        opening[Self::idx(r, c, cols)] = max_val;
                        tophat[Self::idx(r, c, cols)] = z - max_val;
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + 0.2 + ((r + 1) as f64 / rows as f64) * 0.2)
                        / bands as f64,
                );
            }

            // Back-fill shallow hills using slope-limited region growing on tophat values.
            let mut out = vec![initial; rows * cols];
            let mut stack: Vec<(isize, isize)> = Vec::new();
            for r in 0..rows {
                for c in 0..cols {
                    let idx = Self::idx(r, c, cols);
                    let t = tophat[idx];
                    if t == nodata {
                        out[idx] = nodata;
                        continue;
                    }
                    if t <= height_diff_threshold[1] {
                        out[idx] = t;
                        stack.push((r as isize, c as isize));
                    }
                }
            }

            while let Some((r, c)) = stack.pop() {
                let z = tophat[Self::idx(r as usize, c as usize, cols)];
                for n in 0..8 {
                    let rr = r + dy[n];
                    let cc = c + dx[n];
                    if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                        continue;
                    }
                    let ni = Self::idx(rr as usize, cc as usize, cols);
                    let zn = tophat[ni];
                    if zn == nodata || out[ni] != initial {
                        continue;
                    }
                    if zn - z < height_diff_threshold[n] {
                        out[ni] = zn;
                        stack.push((rr, cc));
                    }
                }
            }

            // Identify edge cells bordering masked OTO zones for interpolation seeds.
            let mut seeds: Vec<(usize, usize, f64)> = Vec::new();
            for r in 0..rows {
                for c in 0..cols {
                    let i = Self::idx(r, c, cols);
                    if tophat[i] == nodata || out[i] == initial {
                        continue;
                    }
                    let mut is_edge = false;
                    for n in 0..8 {
                        let rr = r as isize + dy[n];
                        let cc = c as isize + dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let ni = Self::idx(rr as usize, cc as usize, cols);
                        if tophat[ni] != nodata && out[ni] == initial {
                            is_edge = true;
                            break;
                        }
                    }
                    if is_edge {
                        seeds.push((r, c, opening[i] + tophat[i]));
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + 0.4 + ((r + 1) as f64 / rows as f64) * 0.2)
                        / bands as f64,
                );
            }

            let radius = filter_size as f64 / 1.5;
            let radius2 = radius * radius;
            let r_cells = radius.ceil() as isize;

            for r in 0..rows {
                for c in 0..cols {
                    let i = Self::idx(r, c, cols);
                    if tophat[i] == nodata {
                        out[i] = nodata;
                        continue;
                    }
                    if out[i] == initial {
                        let r0 = (r as isize - r_cells).max(0) as usize;
                        let r1 = (r as isize + r_cells).min(rows as isize - 1) as usize;
                        let c0 = (c as isize - r_cells).max(0) as usize;
                        let c1 = (c as isize + r_cells).min(cols as isize - 1) as usize;

                        let mut sum_w = 0.0;
                        let mut sum_z = 0.0;
                        for &(sr, sc, sz) in &seeds {
                            if sr < r0 || sr > r1 || sc < c0 || sc > c1 {
                                continue;
                            }
                            let dr = sr as isize - r as isize;
                            let dc = sc as isize - c as isize;
                            let dist2 = (dr * dr + dc * dc) as f64;
                            if dist2 <= 0.0 || dist2 > radius2 {
                                continue;
                            }
                            let w = 1.0 / dist2;
                            sum_w += w;
                            sum_z += sz * w;
                        }

                        if sum_w > 0.0 {
                            out[i] = sum_z / sum_w;
                        } else {
                            out[i] = nodata;
                        }
                    } else {
                        out[i] = opening[i] + tophat[i];
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + 0.6 + ((r + 1) as f64 / rows as f64) * 0.4)
                        / bands as f64,
                );
            }

            for r in 0..rows {
                let mut row_out = vec![nodata; cols];
                for c in 0..cols {
                    let i = Self::idx(r, c, cols);
                    if tophat[i] != nodata && out[i] != initial {
                        row_out[c] = out[i];
                    }
                }
                output
                    .set_row_slice(band, r as isize, &row_out)
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", r, e)))?;
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_map_off_terrain_objects(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;

        let max_slope_deg = args
            .get("max_slope")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY)
            .clamp(1.0, 90.0);
        let max_slope = max_slope_deg.to_radians().tan();
        let min_feature_size = args
            .get("min_feature_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let res_x = input.cell_size_x.abs();
        let res_y = input.cell_size_y.abs();
        let res_diag = (res_x * res_x + res_y * res_y).sqrt();
        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
        let cell_size = [res_diag, res_x, res_diag, res_y, res_diag, res_x, res_diag, res_y];

        let mut output = input.clone();
        output.data_type = DataType::F64;
        output.nodata = -32768.0;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let mut labels = vec![-1.0_f64; rows * cols];
            let mut fid = 1.0_f64;
            let mut visited = 0usize;

            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = input.get(band, row as isize, col as isize);
                    if z == nodata {
                        labels[idx] = -32768.0;
                        visited += 1;
                        continue;
                    }
                    if labels[idx] != -1.0 {
                        continue;
                    }

                    let mut stack: Vec<(usize, usize)> = vec![(row, col)];
                    labels[idx] = fid;
                    let mut region_cells: Vec<usize> = vec![idx];

                    while let Some((r, c)) = stack.pop() {
                        let zc = input.get(band, r as isize, c as isize);
                        for n in 0..8 {
                            let rr = r as isize + dy[n];
                            let cc = c as isize + dx[n];
                            if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                                continue;
                            }
                            let rru = rr as usize;
                            let ccu = cc as usize;
                            let ni = Self::idx(rru, ccu, cols);
                            if labels[ni] != -1.0 {
                                continue;
                            }
                            let zn = input.get(band, rr, cc);
                            if zn == nodata {
                                continue;
                            }
                            if (zc - zn).abs() / cell_size[n] < max_slope {
                                labels[ni] = fid;
                                stack.push((rru, ccu));
                                region_cells.push(ni);
                            }
                        }
                        visited += 1;
                    }

                    if region_cells.len() < min_feature_size {
                        for &ri in &region_cells {
                            labels[ri] = 1.0;
                        }
                    } else {
                        fid += 1.0;
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + (row + 1) as f64 / rows as f64) / bands as f64,
                );
            }

            let _ = visited;
            for row in 0..rows {
                let start = Self::idx(row, 0, cols);
                let end = start + cols;
                output
                    .set_row_slice(band, row as isize, &labels[start..end])
                    .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_smooth_vegetation_residual(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(30)
            .max(1);
        let mut dev_threshold = args
            .get("dev_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        if !dev_threshold.is_finite() {
            dev_threshold = 1.0;
        }
        let mut scale_threshold = args
            .get("scale_threshold")
            .and_then(|v| v.as_i64())
            .map(|v| v.max(1) as usize)
            .or_else(|| {
                args.get("scale_threshold")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.max(1) as usize)
            })
            .unwrap_or(5);
        if scale_threshold > max_scale {
            scale_threshold = max_scale;
        }

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let mut output = input.clone();

        let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];

        let mut scales = Vec::with_capacity(max_scale);
        for s in 1..=max_scale {
            scales.push(s);
        }

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running smooth_vegetation_residual");

            let (sum, sum_sq, count) = Self::build_integrals(&input, band);
            let mut dev_mag = vec![0.0_f64; rows * cols];
            let mut dev_scale = vec![usize::MAX; rows * cols];

            for midpoint in &scales {
                let midpoint = *midpoint;
                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![0.0; cols];
                        for (c, out_cell) in row_out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n <= 1 {
                                continue;
                            }
                            let n_f = n as f64;
                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                            let variance =
                                ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                            let std_dev = variance.sqrt();
                            if std_dev > 0.0 {
                                *out_cell = (z - local_sum / n_f) / std_dev;
                            }
                        }
                        row_out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, z2) in row.iter().enumerate().take(cols) {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let idx = Self::idx(r, c, cols);
                        if dev_scale[idx] == usize::MAX || z2.abs() > dev_mag[idx].abs() {
                            dev_mag[idx] = *z2;
                            dev_scale[idx] = midpoint;
                        }
                    }
                }
            }

            let mut thresholded = vec![false; rows * cols];
            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = input.get(band, row as isize, col as isize);
                    if input.is_nodata(z) {
                        continue;
                    }
                    if dev_scale[idx] <= scale_threshold && dev_mag[idx] >= dev_threshold {
                        thresholded[idx] = true;
                    }
                }
            }

            let mut seed = vec![false; rows * cols];
            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = input.get(band, row as isize, col as isize);
                    if input.is_nodata(z) || thresholded[idx] {
                        continue;
                    }
                    for n in 0..8 {
                        let rr = row as isize + dy[n];
                        let cc = col as isize + dx[n];
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let ni = Self::idx(rr as usize, cc as usize, cols);
                        if thresholded[ni] {
                            seed[idx] = true;
                            break;
                        }
                    }
                }
            }

            let radius = scale_threshold as isize;
            let radius2 = radius * radius;
            let mut out_vals = vec![nodata; rows * cols];

            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = input.get(band, row as isize, col as isize);
                    if input.is_nodata(z) {
                        out_vals[idx] = nodata;
                        continue;
                    }
                    if !thresholded[idx] {
                        out_vals[idx] = z;
                        continue;
                    }

                    let r0 = (row as isize - radius).max(0) as usize;
                    let r1 = (row as isize + radius).min(rows as isize - 1) as usize;
                    let c0 = (col as isize - radius).max(0) as usize;
                    let c1 = (col as isize + radius).min(cols as isize - 1) as usize;

                    let mut sum_w = 0.0;
                    let mut sum_z = 0.0;
                    for rr in r0..=r1 {
                        let dr = rr as isize - row as isize;
                        for cc in c0..=c1 {
                            let ni = Self::idx(rr, cc, cols);
                            if !seed[ni] {
                                continue;
                            }
                            let dc = cc as isize - col as isize;
                            let dist2 = dr * dr + dc * dc;
                            if dist2 <= 0 || dist2 > radius2 {
                                continue;
                            }
                            let w = 1.0 / dist2 as f64;
                            sum_w += w;
                            sum_z += input.get(band, rr as isize, cc as isize) * w;
                        }
                    }

                    out_vals[idx] = if sum_w > 0.0 { sum_z / sum_w } else { nodata };
                }

                coalescer.emit_unit_fraction(ctx.progress, 
                    (band_idx as f64 + (row + 1) as f64 / rows as f64) / bands as f64,
                );
            }

            for row in 0..rows {
                let start = Self::idx(row, 0, cols);
                let end = start + cols;
                output
                    .set_row_slice(band, row as isize, &out_vals[start..end])
                    .map_err(|e| {
                        ToolError::Execution(format!("failed writing row {}: {}", row, e))
                    })?;
            }
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn multiscale_elevated_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_elevated_index",
            display_name: "Multiscale Elevated Index",
            summary: "Calculates multiscale elevated-index (MsEI) and key-scale rasters using Gaussian scale-space residuals.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum search radius in grid cells (default 2).", required: false },
                ToolParamSpec { name: "step_size", description: "Base step size in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "num_steps", description: "Number of scales to evaluate (default 100).", required: false },
                ToolParamSpec { name: "step_nonlinearity", description: "Scale-step nonlinearity in [1,4] (default 1.1).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for index raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for key-scale raster.", required: false },
            ],
        }
    }

    fn multiscale_elevated_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(2));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(100));
        defaults.insert("step_nonlinearity".to_string(), json!(1.1));

        ToolManifest {
            id: "multiscale_elevated_index".to_string(),
            display_name: "Multiscale Elevated Index".to_string(),
            summary: "Calculates multiscale elevated-index (MsEI) and key-scale rasters using Gaussian scale-space residuals."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "multiscale".to_string(),
                "gss".to_string(),
                "elevated-index".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_low_lying_index_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_low_lying_index",
            display_name: "Multiscale Low-Lying Index",
            summary: "Calculates multiscale low-lying-index (MsLLI) and key-scale rasters using Gaussian scale-space residuals.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum search radius in grid cells (default 2).", required: false },
                ToolParamSpec { name: "step_size", description: "Base step size in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "num_steps", description: "Number of scales to evaluate (default 100).", required: false },
                ToolParamSpec { name: "step_nonlinearity", description: "Scale-step nonlinearity in [1,4] (default 1.1).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for index raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for key-scale raster.", required: false },
            ],
        }
    }

    fn multiscale_low_lying_index_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(2));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(100));
        defaults.insert("step_nonlinearity".to_string(), json!(1.1));

        ToolManifest {
            id: "multiscale_low_lying_index".to_string(),
            display_name: "Multiscale Low-Lying Index".to_string(),
            summary: "Calculates multiscale low-lying-index (MsLLI) and key-scale rasters using Gaussian scale-space residuals."
                .to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![],
            defaults,
            examples: vec![],
            tags: vec![
                "geomorphometry".to_string(),
                "multiscale".to_string(),
                "gss".to_string(),
                "low-lying-index".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn local_hypsometric_analysis_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "local_hypsometric_analysis",
            display_name: "Local Hypsometric Analysis",
            summary: "Computes the minimum local hypsometric integral across a nonlinearly sampled range of neighbourhood scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec {
                    name: "input",
                    description: "Input DEM raster path or typed raster object.",
                    required: true,
                },
                ToolParamSpec {
                    name: "min_scale",
                    description: "Minimum half-window radius in cells (default 4).",
                    required: false,
                },
                ToolParamSpec {
                    name: "step_size",
                    description: "Base step size in cells (default 1). Alias: step.",
                    required: false,
                },
                ToolParamSpec {
                    name: "num_steps",
                    description: "Number of scales to evaluate (default 10).",
                    required: false,
                },
                ToolParamSpec {
                    name: "step_nonlinearity",
                    description: "Scale-step nonlinearity in [1,4] (default 1.0).",
                    required: false,
                },
                ToolParamSpec {
                    name: "output",
                    description: "Optional output path for local HI minimum raster.",
                    required: false,
                },
                ToolParamSpec {
                    name: "output_scale",
                    description: "Optional output path for scale-of-minimum-HI raster.",
                    required: false,
                },
            ],
        }
    }

    fn local_hypsometric_analysis_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(4));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(10));
        defaults.insert("step_nonlinearity".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(4));
        example_args.insert("step_size".to_string(), json!(1));
        example_args.insert("num_steps".to_string(), json!(10));
        example_args.insert("step_nonlinearity".to_string(), json!(1.0));
        example_args.insert("output".to_string(), json!("local_hypsometric_analysis.tif"));
        example_args.insert(
            "output_scale".to_string(),
            json!("local_hypsometric_analysis_scale.tif"),
        );

        ToolManifest {
            id: "local_hypsometric_analysis".to_string(),
            display_name: "Local Hypsometric Analysis".to_string(),
            summary: "Computes the minimum local hypsometric integral across a nonlinearly sampled range of neighbourhood scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamDescriptor {
                    name: "input".to_string(),
                    description: "Input DEM raster path or typed raster object.".to_string(),
                    required: true,
                },
                ToolParamDescriptor {
                    name: "min_scale".to_string(),
                    description: "Minimum half-window radius in cells (default 4).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "step_size".to_string(),
                    description: "Base step size in cells (default 1). Alias: step.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "num_steps".to_string(),
                    description: "Number of scales to evaluate (default 10).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "step_nonlinearity".to_string(),
                    description: "Scale-step nonlinearity in [1,4] (default 1.0).".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output".to_string(),
                    description: "Optional output path for local HI minimum raster.".to_string(),
                    required: false,
                },
                ToolParamDescriptor {
                    name: "output_scale".to_string(),
                    description: "Optional output path for scale-of-minimum-HI raster.".to_string(),
                    required: false,
                },
            ],
            defaults,
            examples: vec![ToolExample {
                name: "basic_local_hypsometric_analysis".to_string(),
                description:
                    "Compute minimum local hypsometric integral and associated scale.".to_string(),
                args: example_args,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "multiscale".to_string(),
                "hypsometry".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn difference_from_mean_elevation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "difference_from_mean_elevation",
            display_name: "Difference From Mean Elevation",
            summary: "Calculates the difference between each elevation and the local mean elevation.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default 11). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size_x). Alias: filtery.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn difference_from_mean_elevation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("filter_size_x".to_string(), json!(11));
        example_args.insert("filter_size_y".to_string(), json!(11));
        example_args.insert("output".to_string(), json!("difference_from_mean_elevation.tif"));

        ToolManifest {
            id: "difference_from_mean_elevation".to_string(),
            display_name: "Difference From Mean Elevation".to_string(),
            summary: "Calculates the difference between each elevation and the local mean elevation.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "filter_size_x".to_string(), description: "Odd filter width in cells (default 11). Alias: filterx.".to_string(), required: false },
                ToolParamDescriptor { name: "filter_size_y".to_string(), description: "Odd filter height in cells (default filter_size_x). Alias: filtery.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_difference_from_mean_elevation".to_string(), description: "Compute local mean difference from a DEM.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "local-relief".to_string(), "integral-image".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn deviation_from_mean_elevation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "deviation_from_mean_elevation",
            display_name: "Deviation From Mean Elevation",
            summary: "Calculates the local topographic z-score using local mean and standard deviation.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default 11). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size_x). Alias: filtery.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn deviation_from_mean_elevation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size_x".to_string(), json!(11));
        defaults.insert("filter_size_y".to_string(), json!(11));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("filter_size_x".to_string(), json!(11));
        example_args.insert("filter_size_y".to_string(), json!(11));
        example_args.insert("output".to_string(), json!("deviation_from_mean_elevation.tif"));

        ToolManifest {
            id: "deviation_from_mean_elevation".to_string(),
            display_name: "Deviation From Mean Elevation".to_string(),
            summary: "Calculates the local topographic z-score using local mean and standard deviation.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "filter_size_x".to_string(), description: "Odd filter width in cells (default 11). Alias: filterx.".to_string(), required: false },
                ToolParamDescriptor { name: "filter_size_y".to_string(), description: "Odd filter height in cells (default filter_size_x). Alias: filtery.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_deviation_from_mean_elevation".to_string(), description: "Compute local elevation deviation z-scores from a DEM.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "local-relief".to_string(), "integral-image".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn standard_deviation_of_slope_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "standard_deviation_of_slope",
            display_name: "Standard Deviation Of Slope",
            summary: "Calculates local standard deviation of slope as a terrain roughness metric.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "filter_size", description: "Odd filter size in cells (default 11).", required: false },
                ToolParamSpec { name: "filter_size_x", description: "Odd filter width in cells (default filter_size). Alias: filterx.", required: false },
                ToolParamSpec { name: "filter_size_y", description: "Odd filter height in cells (default filter_size). Alias: filtery.", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor when vertical and horizontal units differ (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path. If omitted, result stays in memory.", required: false },
            ],
        }
    }

    fn standard_deviation_of_slope_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("filter_size".to_string(), json!(11));
        defaults.insert("z_factor".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("filter_size".to_string(), json!(11));
        example_args.insert("output".to_string(), json!("standard_deviation_of_slope.tif"));

        ToolManifest {
            id: "standard_deviation_of_slope".to_string(),
            display_name: "Standard Deviation Of Slope".to_string(),
            summary: "Calculates local standard deviation of slope as a terrain roughness metric.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "filter_size".to_string(), description: "Odd filter size in cells (default 11).".to_string(), required: false },
                ToolParamDescriptor { name: "filter_size_x".to_string(), description: "Odd filter width in cells (default filter_size). Alias: filterx.".to_string(), required: false },
                ToolParamDescriptor { name: "filter_size_y".to_string(), description: "Odd filter height in cells (default filter_size). Alias: filtery.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor when vertical and horizontal units differ (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path. If omitted, result stays in memory.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_standard_deviation_of_slope".to_string(), description: "Compute local slope roughness from a DEM.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "roughness".to_string(), "integral-image".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn max_difference_from_mean_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_difference_from_mean",
            display_name: "Max Difference From Mean",
            summary: "Calculates maximum absolute difference-from-mean over a range of neighbourhood scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for max-difference magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of max response.", required: false },
            ],
        }
    }

    fn max_difference_from_mean_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(100));
        example_args.insert("step_size".to_string(), json!(1));
        example_args.insert("output".to_string(), json!("max_difference_from_mean.tif"));
        example_args.insert("output_scale".to_string(), json!("max_difference_from_mean_scale.tif"));

        ToolManifest {
            id: "max_difference_from_mean".to_string(),
            display_name: "Max Difference From Mean".to_string(),
            summary: "Calculates maximum absolute difference-from-mean over a range of neighbourhood scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for max-difference magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of max response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_max_difference_from_mean".to_string(), description: "Compute multiscale maximum local relief contrast.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "integral-image".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn max_elevation_deviation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_elevation_deviation",
            display_name: "Max Elevation Deviation",
            summary: "Calculates maximum standardized elevation deviation (DEVmax) over a range of neighbourhood scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for DEVmax magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of max response.", required: false },
            ],
        }
    }

    fn max_elevation_deviation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(100));
        example_args.insert("step_size".to_string(), json!(1));
        example_args.insert("output".to_string(), json!("max_elevation_deviation.tif"));
        example_args.insert("output_scale".to_string(), json!("max_elevation_deviation_scale.tif"));

        ToolManifest {
            id: "max_elevation_deviation".to_string(),
            display_name: "Max Elevation Deviation".to_string(),
            summary: "Calculates maximum standardized elevation deviation (DEVmax) over a range of neighbourhood scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for DEVmax magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of max response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_max_elevation_deviation".to_string(), description: "Compute multiscale DEVmax and corresponding optimal scale.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "topographic-position".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn topographic_position_animation_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "topographic_position_animation",
            display_name: "Topographic Position Animation",
            summary: "Creates an interactive HTML viewer and animated GIF of DEV or DEVmax across nonlinearly sampled scales.",
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "output", description: "Output HTML path.", required: false },
                ToolParamSpec { name: "palette", description: "Palette name used to colour DEV values.", required: false },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "num_steps", description: "Number of animation steps (default 10).", required: false },
                ToolParamSpec { name: "step_nonlinearity", description: "Nonlinear scale exponent in [1, 4] controlling coarse-to-fine spacing.", required: false },
                ToolParamSpec { name: "image_height", description: "Displayed image height in pixels.", required: false },
                ToolParamSpec { name: "delay", description: "Per-frame GIF delay in milliseconds.", required: false },
                ToolParamSpec { name: "label", description: "Optional label displayed in the viewer.", required: false },
                ToolParamSpec { name: "use_dev_max", description: "If true, frames accumulate the strongest absolute DEV response encountered so far.", required: false },
            ],
        }
    }

    fn topographic_position_animation_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("output".to_string(), json!("topographic_position_animation.html"));
        defaults.insert("palette".to_string(), json!("soft"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(10));
        defaults.insert("step_nonlinearity".to_string(), json!(1.0));
        defaults.insert("image_height".to_string(), json!(600));
        defaults.insert("delay".to_string(), json!(250));
        defaults.insert("label".to_string(), json!(""));
        defaults.insert("use_dev_max".to_string(), json!(false));

        let mut example = ToolArgs::new();
        example.insert("input".to_string(), json!("dem.tif"));
        example.insert("output".to_string(), json!("topographic_position_animation.html"));
        example.insert("num_steps".to_string(), json!(8));
        example.insert("use_dev_max".to_string(), json!(true));

        ToolManifest {
            id: "topographic_position_animation".to_string(),
            display_name: "Topographic Position Animation".to_string(),
            summary: "Creates an interactive HTML viewer and animated GIF of DEV or DEVmax across nonlinearly sampled scales.".to_string(),
            category: ToolCategory::Terrain,
            license_tier: LicenseTier::Pro,
            params: vec![],
            defaults,
            examples: vec![ToolExample {
                name: "basic_topographic_position_animation".to_string(),
                description: "Animate terrain topographic position through a sequence of DEV scales.".to_string(),
                args: example,
            }],
            tags: vec![
                "geomorphometry".to_string(),
                "terrain".to_string(),
                "topographic-position".to_string(),
                "animation".to_string(),
                "integral-image".to_string(),
                "legacy-port".to_string(),
            ],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_topographic_position_image_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_topographic_position_image",
            display_name: "Multiscale Topographic Position Image",
            summary: "Creates a packed RGB multiscale topographic-position image from local, meso, and broad DEVmax rasters.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "local", description: "Input local-scale DEVmax raster path.", required: true },
                ToolParamSpec { name: "meso", description: "Input meso-scale DEVmax raster path.", required: true },
                ToolParamSpec { name: "broad", description: "Input broad-scale DEVmax raster path.", required: true },
                ToolParamSpec { name: "hillshade", description: "Optional hillshade raster path used for illumination modulation.", required: false },
                ToolParamSpec { name: "lightness", description: "Image lightness scaling factor (default 1.2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for packed RGB raster.", required: false },
            ],
        }
    }

    fn multiscale_topographic_position_image_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("local".to_string(), json!("dev_local.tif"));
        defaults.insert("meso".to_string(), json!("dev_meso.tif"));
        defaults.insert("broad".to_string(), json!("dev_broad.tif"));
        defaults.insert("lightness".to_string(), json!(1.2));

        let mut example_args = ToolArgs::new();
        example_args.insert("local".to_string(), json!("dev_local.tif"));
        example_args.insert("meso".to_string(), json!("dev_meso.tif"));
        example_args.insert("broad".to_string(), json!("dev_broad.tif"));
        example_args.insert("hillshade".to_string(), json!("multidirectional_hillshade.tif"));
        example_args.insert("lightness".to_string(), json!(1.2));
        example_args.insert("output".to_string(), json!("mtp.tif"));

        ToolManifest {
            id: "multiscale_topographic_position_image".to_string(),
            display_name: "Multiscale Topographic Position Image".to_string(),
            summary: "Creates a packed RGB multiscale topographic-position image from local, meso, and broad DEVmax rasters.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "local".to_string(), description: "Input local-scale DEVmax raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "meso".to_string(), description: "Input meso-scale DEVmax raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "broad".to_string(), description: "Input broad-scale DEVmax raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "hillshade".to_string(), description: "Optional hillshade raster path used for illumination modulation.".to_string(), required: false },
                ToolParamDescriptor { name: "lightness".to_string(), description: "Image lightness scaling factor (default 1.2).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for packed RGB raster.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_topographic_position_image".to_string(), description: "Compose local/meso/broad DEVmax rasters into a packed RGB visualization.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "visualization".to_string(), "topographic-position".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_elevation_percentile_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_elevation_percentile",
            display_name: "Multiscale Elevation Percentile",
            summary: "Calculates the most extreme local elevation percentile across a range of neighbourhood scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 4).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "sig_digits", description: "Significant decimal digits used during percentile binning (default 2).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for percentile magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of most extreme percentile response.", required: false },
            ],
        }
    }

    fn multiscale_elevation_percentile_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(4));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("sig_digits".to_string(), json!(2));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(4));
        example_args.insert("max_scale".to_string(), json!(100));
        example_args.insert("step_size".to_string(), json!(2));
        example_args.insert("sig_digits".to_string(), json!(2));
        example_args.insert("output".to_string(), json!("multiscale_elevation_percentile.tif"));
        example_args.insert("output_scale".to_string(), json!("multiscale_elevation_percentile_scale.tif"));

        ToolManifest {
            id: "multiscale_elevation_percentile".to_string(),
            display_name: "Multiscale Elevation Percentile".to_string(),
            summary: "Calculates the most extreme local elevation percentile across a range of neighbourhood scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 4).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "sig_digits".to_string(), description: "Significant decimal digits used during percentile binning (default 2).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for percentile magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of most extreme percentile response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_elevation_percentile".to_string(), description: "Compute the most extreme local elevation percentile across multiple scales.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "topographic-position".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn max_anisotropy_dev_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_anisotropy_dev",
            display_name: "Max Anisotropy Dev",
            summary: "Calculates maximum anisotropy in elevation deviation over a range of neighbourhood scales. Written by Dan Newman.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 3).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 2). Alias: step.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for anisotropy magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of max response.", required: false },
            ],
        }
    }

    fn max_anisotropy_dev_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(3));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(2));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(3));
        example_args.insert("max_scale".to_string(), json!(100));
        example_args.insert("step_size".to_string(), json!(2));
        example_args.insert("output".to_string(), json!("max_anisotropy_dev.tif"));
        example_args.insert("output_scale".to_string(), json!("max_anisotropy_dev_scale.tif"));

        ToolManifest {
            id: "max_anisotropy_dev".to_string(),
            display_name: "Max Anisotropy Dev".to_string(),
            summary: "Calculates maximum anisotropy in elevation deviation over a range of neighbourhood scales. Written by Dan Newman.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 3).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 2). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for anisotropy magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of max response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_max_anisotropy_dev".to_string(), description: "Compute multiscale anisotropy response in standardized elevation deviation.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "anisotropy".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_roughness_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_roughness",
            display_name: "Multiscale Roughness",
            summary: "Calculates surface roughness over a range of neighbourhood scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor when vertical and horizontal units differ (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for roughness magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of max response.", required: false },
            ],
        }
    }

    fn multiscale_roughness_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("z_factor".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(100));
        example_args.insert("step_size".to_string(), json!(2));
        example_args.insert("output".to_string(), json!("multiscale_roughness.tif"));
        example_args.insert("output_scale".to_string(), json!("multiscale_roughness_scale.tif"));

        ToolManifest {
            id: "multiscale_roughness".to_string(),
            display_name: "Multiscale Roughness".to_string(),
            summary: "Calculates surface roughness over a range of neighbourhood scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor when vertical and horizontal units differ (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for roughness magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of max response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_roughness".to_string(), description: "Compute roughness from scale-dependent normal-vector deviations.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn max_elev_dev_signature_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_elev_dev_signature",
            display_name: "Max Elevation Deviation Signature",
            summary: "Calculates multiscale elevation-deviation signatures for input point sites and writes an HTML report.",
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path.", required: true },
                ToolParamSpec { name: "points", description: "Input vector point or multipoint file path.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 10). Alias: step.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for the HTML signature report.", required: false },
            ],
        }
    }

    fn max_elev_dev_signature_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("points".to_string(), json!("sites.geojson"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(10));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("points".to_string(), json!("sites.geojson"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(150));
        example_args.insert("step_size".to_string(), json!(5));
        example_args.insert("output".to_string(), json!("max_elev_dev_signature.html"));

        ToolManifest {
            id: "max_elev_dev_signature".to_string(),
            display_name: "Max Elevation Deviation Signature".to_string(),
            summary: "Calculates multiscale elevation-deviation signatures for input point sites and writes an HTML report.".to_string(),
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "points".to_string(), description: "Input vector point or multipoint file path.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 10). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for the HTML signature report.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_max_elev_dev_signature".to_string(), description: "Generate DEV signatures for a set of sample locations.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "signature".to_string(), "multiscale".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn max_anisotropy_dev_signature_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "max_anisotropy_dev_signature",
            display_name: "Max Anisotropy Dev Signature",
            summary: "Calculates multiscale anisotropy signatures for input point sites and writes an HTML report. Written by Dan Newman.",
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path.", required: true },
                ToolParamSpec { name: "points", description: "Input vector point or multipoint file path.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for the HTML signature report.", required: false },
            ],
        }
    }

    fn max_anisotropy_dev_signature_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("points".to_string(), json!("sites.geojson"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("points".to_string(), json!("sites.geojson"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(150));
        example_args.insert("step_size".to_string(), json!(2));
        example_args.insert("output".to_string(), json!("max_anisotropy_dev_signature.html"));

        ToolManifest {
            id: "max_anisotropy_dev_signature".to_string(),
            display_name: "Max Anisotropy Dev Signature".to_string(),
            summary: "Calculates multiscale anisotropy signatures for input point sites and writes an HTML report. Written by Dan Newman.".to_string(),
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "points".to_string(), description: "Input vector point or multipoint file path.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for the HTML signature report.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_max_anisotropy_dev_signature".to_string(), description: "Generate anisotropy signatures for a set of sample locations.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "signature".to_string(), "anisotropy".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_roughness_signature_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_roughness_signature",
            display_name: "Multiscale Roughness Signature",
            summary: "Calculates multiscale roughness signatures for input point sites and writes an HTML report.",
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path.", required: true },
                ToolParamSpec { name: "points", description: "Input vector point or multipoint file path.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "max_scale", description: "Maximum half-window radius in cells (default 100).", required: false },
                ToolParamSpec { name: "step_size", description: "Scale increment in cells (default 1). Alias: step.", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor when vertical and horizontal units differ (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for the HTML signature report.", required: false },
            ],
        }
    }

    fn multiscale_roughness_signature_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("points".to_string(), json!("sites.geojson"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("max_scale".to_string(), json!(100));
        defaults.insert("step_size".to_string(), json!(1));
        defaults.insert("z_factor".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("points".to_string(), json!("sites.geojson"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("max_scale".to_string(), json!(150));
        example_args.insert("step_size".to_string(), json!(2));
        example_args.insert("z_factor".to_string(), json!(1.0));
        example_args.insert("output".to_string(), json!("multiscale_roughness_signature.html"));

        ToolManifest {
            id: "multiscale_roughness_signature".to_string(),
            display_name: "Multiscale Roughness Signature".to_string(),
            summary: "Calculates multiscale roughness signatures for input point sites and writes an HTML report.".to_string(),
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "points".to_string(), description: "Input vector point or multipoint file path.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "max_scale".to_string(), description: "Maximum half-window radius in cells (default 100).".to_string(), required: false },
                ToolParamDescriptor { name: "step_size".to_string(), description: "Scale increment in cells (default 1). Alias: step.".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor when vertical and horizontal units differ (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for the HTML signature report.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_roughness_signature".to_string(), description: "Generate roughness signatures for a set of sample locations.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "signature".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_std_dev_normals_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_std_dev_normals",
            display_name: "Multiscale Std Dev Normals",
            summary: "Calculates maximum spherical standard deviation of surface normals over a nonlinearly sampled range of scales.",
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path or typed raster object.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "step", description: "Base step size in cells used by nonlinear scale schedule (default 1).", required: false },
                ToolParamSpec { name: "num_steps", description: "Number of sampled scales (default 10).", required: false },
                ToolParamSpec { name: "step_nonlinearity", description: "Nonlinearity exponent for scale schedule (default 1.0).", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor when vertical and horizontal units differ (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for spherical standard-deviation magnitude raster.", required: false },
                ToolParamSpec { name: "output_scale", description: "Optional output path for raster storing scale of max response.", required: false },
            ],
        }
    }

    fn multiscale_std_dev_normals_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("step".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(10));
        defaults.insert("step_nonlinearity".to_string(), json!(1.0));
        defaults.insert("z_factor".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("step".to_string(), json!(2));
        example_args.insert("num_steps".to_string(), json!(20));
        example_args.insert("step_nonlinearity".to_string(), json!(1.5));
        example_args.insert("output".to_string(), json!("multiscale_std_dev_normals.tif"));
        example_args.insert("output_scale".to_string(), json!("multiscale_std_dev_normals_scale.tif"));

        ToolManifest {
            id: "multiscale_std_dev_normals".to_string(),
            display_name: "Multiscale Std Dev Normals".to_string(),
            summary: "Calculates maximum spherical standard deviation of surface normals over a nonlinearly sampled range of scales.".to_string(),
            category: ToolCategory::Raster,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path or typed raster object.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "step".to_string(), description: "Base step size in cells used by nonlinear scale schedule (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "num_steps".to_string(), description: "Number of sampled scales (default 10).".to_string(), required: false },
                ToolParamDescriptor { name: "step_nonlinearity".to_string(), description: "Nonlinearity exponent for scale schedule (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor when vertical and horizontal units differ (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for spherical standard-deviation magnitude raster.".to_string(), required: false },
                ToolParamDescriptor { name: "output_scale".to_string(), description: "Optional output path for raster storing scale of max response.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_std_dev_normals".to_string(), description: "Compute multiscale maximum spherical standard deviation of normals.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "multiscale".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn multiscale_std_dev_normals_signature_metadata() -> ToolMetadata {
        ToolMetadata {
            id: "multiscale_std_dev_normals_signature",
            display_name: "Multiscale Std Dev Normals Signature",
            summary: "Calculates spherical-standard-deviation scale signatures for input point sites and writes an HTML report.",
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamSpec { name: "input", description: "Input DEM raster path.", required: true },
                ToolParamSpec { name: "points", description: "Input vector point or multipoint file path.", required: true },
                ToolParamSpec { name: "min_scale", description: "Minimum half-window radius in cells (default 1).", required: false },
                ToolParamSpec { name: "step", description: "Base step size in cells used by nonlinear scale schedule (default 1).", required: false },
                ToolParamSpec { name: "num_steps", description: "Number of sampled scales (default 10).", required: false },
                ToolParamSpec { name: "step_nonlinearity", description: "Nonlinearity exponent for scale schedule (default 1.0).", required: false },
                ToolParamSpec { name: "z_factor", description: "Z conversion factor when vertical and horizontal units differ (default 1.0).", required: false },
                ToolParamSpec { name: "output", description: "Optional output path for the HTML signature report.", required: false },
            ],
        }
    }

    fn multiscale_std_dev_normals_signature_manifest() -> ToolManifest {
        let mut defaults = ToolArgs::new();
        defaults.insert("input".to_string(), json!("dem.tif"));
        defaults.insert("points".to_string(), json!("sites.geojson"));
        defaults.insert("min_scale".to_string(), json!(1));
        defaults.insert("step".to_string(), json!(1));
        defaults.insert("num_steps".to_string(), json!(10));
        defaults.insert("step_nonlinearity".to_string(), json!(1.0));
        defaults.insert("z_factor".to_string(), json!(1.0));

        let mut example_args = ToolArgs::new();
        example_args.insert("input".to_string(), json!("dem.tif"));
        example_args.insert("points".to_string(), json!("sites.geojson"));
        example_args.insert("min_scale".to_string(), json!(1));
        example_args.insert("step".to_string(), json!(2));
        example_args.insert("num_steps".to_string(), json!(20));
        example_args.insert("step_nonlinearity".to_string(), json!(1.5));
        example_args.insert("output".to_string(), json!("multiscale_std_dev_normals_signature.html"));

        ToolManifest {
            id: "multiscale_std_dev_normals_signature".to_string(),
            display_name: "Multiscale Std Dev Normals Signature".to_string(),
            summary: "Calculates spherical-standard-deviation scale signatures for input point sites and writes an HTML report.".to_string(),
            category: ToolCategory::Other,
            license_tier: LicenseTier::Open,
            params: vec![
                ToolParamDescriptor { name: "input".to_string(), description: "Input DEM raster path.".to_string(), required: true },
                ToolParamDescriptor { name: "points".to_string(), description: "Input vector point or multipoint file path.".to_string(), required: true },
                ToolParamDescriptor { name: "min_scale".to_string(), description: "Minimum half-window radius in cells (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "step".to_string(), description: "Base step size in cells used by nonlinear scale schedule (default 1).".to_string(), required: false },
                ToolParamDescriptor { name: "num_steps".to_string(), description: "Number of sampled scales (default 10).".to_string(), required: false },
                ToolParamDescriptor { name: "step_nonlinearity".to_string(), description: "Nonlinearity exponent for scale schedule (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "z_factor".to_string(), description: "Z conversion factor when vertical and horizontal units differ (default 1.0).".to_string(), required: false },
                ToolParamDescriptor { name: "output".to_string(), description: "Optional output path for the HTML signature report.".to_string(), required: false },
            ],
            defaults,
            examples: vec![ToolExample { name: "basic_multiscale_std_dev_normals_signature".to_string(), description: "Generate spherical-standard-deviation signatures for sample locations.".to_string(), args: example_args }],
            tags: vec!["geomorphometry".to_string(), "terrain".to_string(), "signature".to_string(), "roughness".to_string(), "legacy-port".to_string()],
            stability: ToolStability::Experimental,
        }
    }

    fn idx(row: usize, col: usize, cols: usize) -> usize {
        row * cols + col
    }

    fn parse_usize_value(v: &Value) -> Option<usize> {
        if let Some(u) = v.as_u64() {
            return Some(u as usize);
        }
        if let Some(i) = v.as_i64() {
            if i >= 0 {
                return Some(i as usize);
            }
        }
        if let Some(f) = v.as_f64() {
            if f.is_finite() && f >= 0.0 {
                return Some(f.round() as usize);
            }
        }
        if let Some(s) = v.as_str() {
            if let Ok(u) = s.trim().parse::<usize>() {
                return Some(u);
            }
            if let Ok(f) = s.trim().parse::<f64>() {
                if f.is_finite() && f >= 0.0 {
                    return Some(f.round() as usize);
                }
            }
        }
        None
    }

    fn parse_f32_value(v: &Value) -> Option<f32> {
        if let Some(f) = v.as_f64() {
            if f.is_finite() {
                return Some(f as f32);
            }
        }
        if let Some(i) = v.as_i64() {
            return Some(i as f32);
        }
        if let Some(u) = v.as_u64() {
            return Some(u as f32);
        }
        if let Some(s) = v.as_str() {
            if let Ok(f) = s.trim().parse::<f32>() {
                if f.is_finite() {
                    return Some(f);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    fn parse_poisson_smoothing_settings(args: &ToolArgs) -> PoissonSmoothingSettings {
        let outer_iterations = args
            .get("outer_iterations")
            .or_else(|| args.get("iterations"))
            .and_then(Self::parse_usize_value)
            .unwrap_or(3)
            .max(1);
        let normal_smoothing_strength = args
            .get("normal_smoothing_strength")
            .and_then(Self::parse_f32_value)
            .or_else(|| {
                args.get("filter_size")
                    .and_then(Self::parse_f32_value)
                    .map(|v| ((v - 3.0) / 28.0).clamp(0.0, 1.0))
            })
            .unwrap_or(0.6)
            .clamp(0.0, 1.0);
        let edge_sensitivity = args
            .get("edge_sensitivity")
            .or_else(|| args.get("edge_sensitive"))
            .and_then(Self::parse_f32_value)
            .or_else(|| {
                args.get("normal_diff_threshold")
                    .and_then(Self::parse_f32_value)
                    .map(|deg| (1.0 - ((deg - 5.0) / 35.0)).clamp(0.0, 1.0))
            })
            .unwrap_or(0.7)
            .clamp(0.0, 1.0);
        let lambda = args
            .get("lambda")
            .and_then(Self::parse_f32_value)
            .unwrap_or(0.5)
            .max(f32::EPSILON);
        let convergence_threshold = args
            .get("convergence_threshold")
            .and_then(Self::parse_f32_value)
            .unwrap_or(0.0001)
            .max(0.0);
        let outer_convergence_threshold = args
            .get("outer_convergence_threshold")
            .and_then(Self::parse_f32_value)
            .unwrap_or(0.0)
            .max(0.0);
        let z_factor = args
            .get("z_factor")
            .and_then(Self::parse_f32_value)
            .unwrap_or(1.0);

        PoissonSmoothingSettings {
            outer_iterations,
            normal_smoothing_strength,
            edge_sensitivity,
            lambda,
            convergence_threshold,
            outer_convergence_threshold,
            z_factor,
            use_local_adaptivity: false,
            local_adaptivity_strength: 0.0,
            local_adaptivity_radius: 0,
        }
    }

    fn raster_to_f32_vec(input: &Raster) -> Vec<f32> {
        let mut dem = vec![input.nodata as f32; input.rows * input.cols];
        for row in 0..input.rows {
            for col in 0..input.cols {
                dem[Self::idx(row, col, input.cols)] = input.get(0, row as isize, col as isize) as f32;
            }
        }
        dem
    }

    fn dem_to_output_raster(
        template: &Raster,
        dem: &[f32],
        nodata: f32,
    ) -> Result<Raster, ToolError> {
        let mut output = template.clone();
        let output_rows: Vec<Vec<f64>> = (0..template.rows)
            .into_par_iter()
            .map(|row| {
                let mut row_data = vec![nodata as f64; template.cols];
                for col in 0..template.cols {
                    row_data[col] = dem[Self::idx(row, col, template.cols)] as f64;
                }
                row_data
            })
            .collect();
        for (row, row_data) in output_rows.iter().enumerate() {
            output
                .set_row_slice(0, row as isize, row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
        }
        Ok(output)
    }

    fn build_dem_pyramid(
        dem_orig: &[f32],
        rows: usize,
        cols: usize,
        res_x: f32,
        res_y: f32,
        nodata: f32,
        requested_levels: usize,
    ) -> Vec<RasterPyramidLevel> {
        let mut levels = vec![RasterPyramidLevel {
            dem: dem_orig.to_vec(),
            rows,
            cols,
            res_x,
            res_y,
        }];

        while levels.len() < requested_levels {
            let prev = levels.last().expect("pyramid should contain a base level");
            let next_rows = (prev.rows + 1) / 2;
            let next_cols = (prev.cols + 1) / 2;
            if next_rows < 64 || next_cols < 64 {
                break;
            }
            if next_rows == prev.rows && next_cols == prev.cols {
                break;
            }
            levels.push(Self::downsample_dem_half(prev, nodata));
        }

        levels
    }

    fn downsample_dem_half(prev: &RasterPyramidLevel, nodata: f32) -> RasterPyramidLevel {
        let next_rows = (prev.rows + 1) / 2;
        let next_cols = (prev.cols + 1) / 2;
        let mut dem = vec![nodata; next_rows * next_cols];

        for row in 0..next_rows {
            for col in 0..next_cols {
                let mut sum = 0.0f32;
                let mut count = 0u32;
                for rr in (row * 2)..((row * 2 + 2).min(prev.rows)) {
                    for cc in (col * 2)..((col * 2 + 2).min(prev.cols)) {
                        let value = prev.dem[Self::idx(rr, cc, prev.cols)];
                        if value != nodata {
                            sum += value;
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    dem[Self::idx(row, col, next_cols)] = sum / count as f32;
                }
            }
        }

        RasterPyramidLevel {
            dem,
            rows: next_rows,
            cols: next_cols,
            res_x: prev.res_x * 2.0,
            res_y: prev.res_y * 2.0,
        }
    }

    fn bilinear_upsample_dem(
        coarse_dem: &[f32],
        coarse_rows: usize,
        coarse_cols: usize,
        fine_rows: usize,
        fine_cols: usize,
        nodata: f32,
    ) -> Vec<f32> {
        let mut fine = vec![nodata; fine_rows * fine_cols];
        if coarse_rows == 0 || coarse_cols == 0 || fine_rows == 0 || fine_cols == 0 {
            return fine;
        }

        let row_scale = if fine_rows > 1 && coarse_rows > 1 {
            (coarse_rows - 1) as f32 / (fine_rows - 1) as f32
        } else {
            0.0
        };
        let col_scale = if fine_cols > 1 && coarse_cols > 1 {
            (coarse_cols - 1) as f32 / (fine_cols - 1) as f32
        } else {
            0.0
        };

        for row in 0..fine_rows {
            let src_row = row as f32 * row_scale;
            let r0 = src_row.floor() as usize;
            let r1 = (r0 + 1).min(coarse_rows - 1);
            let fr = src_row - r0 as f32;

            for col in 0..fine_cols {
                let src_col = col as f32 * col_scale;
                let c0 = src_col.floor() as usize;
                let c1 = (c0 + 1).min(coarse_cols - 1);
                let fc = src_col - c0 as f32;

                let neighbours = [
                    (r0, c0, (1.0 - fr) * (1.0 - fc)),
                    (r0, c1, (1.0 - fr) * fc),
                    (r1, c0, fr * (1.0 - fc)),
                    (r1, c1, fr * fc),
                ];

                let mut weighted_sum = 0.0f32;
                let mut weight_sum = 0.0f32;
                for (rr, cc, weight) in neighbours {
                    if weight <= 0.0 {
                        continue;
                    }
                    let value = coarse_dem[Self::idx(rr, cc, coarse_cols)];
                    if value != nodata {
                        weighted_sum += value * weight;
                        weight_sum += weight;
                    }
                }

                if weight_sum > 0.0 {
                    fine[Self::idx(row, col, fine_cols)] = weighted_sum / weight_sum;
                }
            }
        }

        fine
    }

    fn run_poisson_smoothing_core(
        dem_orig: &[f32],
        rows: usize,
        cols: usize,
        nodata: f32,
        res_x: f32,
        res_y: f32,
        settings: &PoissonSmoothingSettings,
        initial_surface: Option<&[f32]>,
    ) -> Vec<f32> {
        let eight_res_x = res_x * 8.0;
        let eight_res_y = res_y * 8.0;
        let mut z_cur = initial_surface
            .filter(|surface| surface.len() == dem_orig.len())
            .map(|surface| surface.to_vec())
            .unwrap_or_else(|| dem_orig.to_vec());

        for idx in 0..z_cur.len() {
            if dem_orig[idx] == nodata {
                z_cur[idx] = nodata;
            } else if z_cur[idx] == nodata || !z_cur[idx].is_finite() {
                z_cur[idx] = dem_orig[idx];
            }
        }

        let valid: Vec<bool> = dem_orig.iter().map(|&v| v != nodata).collect();
        let mut normals_a = vec![0.0f32; rows * cols];
        let mut normals_b = vec![0.0f32; rows * cols];
        let mut smooth_a = vec![0.0f32; rows * cols];
        let mut smooth_b = vec![0.0f32; rows * cols];
        let mut z_nxt = vec![nodata; rows * cols];
        let mut diff_a_nxt = vec![0.0f32; rows * cols];
        let mut diff_b_nxt = vec![0.0f32; rows * cols];
        let mut z_prev_outer = vec![nodata; rows * cols];

        const MAX_JACOBI: usize = 200;

        for _ in 0..settings.outer_iterations {
            if settings.outer_convergence_threshold > 0.0 {
                z_prev_outer.copy_from_slice(&z_cur);
            }

            normals_a
                .par_chunks_mut(cols)
                .zip(normals_b.par_chunks_mut(cols))
                .enumerate()
                .for_each(|(row, (row_a, row_b))| {
                    for col in 0..cols {
                        let z = z_cur[row * cols + col];
                        if z == nodata {
                            continue;
                        }
                        let sample = |r: isize, c: isize| -> f32 {
                            if r < 0 || c < 0 || r >= rows as isize || c >= cols as isize {
                                z
                            } else {
                                let v = z_cur[r as usize * cols + c as usize];
                                if v == nodata { z } else { v }
                            }
                        };
                        let z0 = sample(row as isize - 1, col as isize - 1) * settings.z_factor;
                        let z1 = sample(row as isize - 1, col as isize) * settings.z_factor;
                        let z2 = sample(row as isize - 1, col as isize + 1) * settings.z_factor;
                        let z3 = sample(row as isize, col as isize + 1) * settings.z_factor;
                        let z4 = sample(row as isize + 1, col as isize + 1) * settings.z_factor;
                        let z5 = sample(row as isize + 1, col as isize) * settings.z_factor;
                        let z6 = sample(row as isize + 1, col as isize - 1) * settings.z_factor;
                        let z7 = sample(row as isize, col as isize - 1) * settings.z_factor;
                        row_a[col] = -((z2 - z6) + 2.0 * (z3 - z7) + (z4 - z0)) / eight_res_x;
                        row_b[col] = -((z6 - z0) + 2.0 * (z5 - z1) + (z4 - z2)) / eight_res_y;
                    }
                });

            smooth_a.copy_from_slice(&normals_a);
            smooth_b.copy_from_slice(&normals_b);

            let diffusion_iterations =
                (2.0 + 78.0 * settings.normal_smoothing_strength).round() as usize;
            let tau = 0.10 + 0.14 * settings.normal_smoothing_strength;

            let (sum_mag2, count_mag) = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    if !valid[idx] {
                        return (0.0f64, 0usize);
                    }
                    let row = idx / cols;
                    let col = idx % cols;
                    let mut s = 0.0f64;
                    let mut n = 0usize;
                    if col + 1 < cols {
                        let ni = row * cols + (col + 1);
                        if valid[ni] {
                            let da = normals_a[ni] - normals_a[idx];
                            let db = normals_b[ni] - normals_b[idx];
                            s += (da * da + db * db) as f64;
                            n += 1;
                        }
                    }
                    if row + 1 < rows {
                        let ni = (row + 1) * cols + col;
                        if valid[ni] {
                            let da = normals_a[ni] - normals_a[idx];
                            let db = normals_b[ni] - normals_b[idx];
                            s += (da * da + db * db) as f64;
                            n += 1;
                        }
                    }
                    (s, n)
                })
                .reduce(|| (0.0f64, 0usize), |a, b| (a.0 + b.0, a.1 + b.1));

            let sigma = if count_mag > 0 {
                (sum_mag2 / count_mag as f64).sqrt() as f32
            } else {
                0.1
            }
            .max(1.0e-6);
            let kappa_factor = 0.2 + (1.0 - settings.edge_sensitivity) * 2.0;
            let kappa_base = sigma * kappa_factor;
            let gradient_scale = (1.0
                - 0.85
                    * settings.normal_smoothing_strength.powf(1.15)
                    * (1.0 - 0.5 * settings.edge_sensitivity))
                .clamp(0.12, 1.0);

            // Local scale-adaptive conductance map for heterogeneous terrain.
            // Kept disabled for the current single-scale Poisson tool to preserve behavior.
            let mut kappa_base_map: Option<Vec<f32>> = None;
            if settings.use_local_adaptivity && settings.local_adaptivity_strength > 0.0 {
                let adaptivity = settings.local_adaptivity_strength.clamp(0.0, 1.0);
                let radius = settings.local_adaptivity_radius.max(1) as isize;

                let mut grad_mag = vec![0.0f32; rows * cols];
                grad_mag
                    .par_chunks_mut(cols)
                    .enumerate()
                    .for_each(|(row, row_grad)| {
                        for col in 0..cols {
                            let idx = row * cols + col;
                            if !valid[idx] {
                                continue;
                            }
                            let a0 = normals_a[idx];
                            let b0 = normals_b[idx];
                            let mut sum_g = 0.0f32;
                            let mut n_g = 0u32;

                            macro_rules! add_grad {
                                ($r:expr, $c:expr) => {
                                    if $r >= 0
                                        && $c >= 0
                                        && ($r as usize) < rows
                                        && ($c as usize) < cols
                                    {
                                        let ni = $r as usize * cols + $c as usize;
                                        if valid[ni] {
                                            let da = normals_a[ni] - a0;
                                            let db = normals_b[ni] - b0;
                                            sum_g += (da * da + db * db).sqrt();
                                            n_g += 1;
                                        }
                                    }
                                };
                            }

                            add_grad!(row as isize, col as isize + 1);
                            add_grad!(row as isize, col as isize - 1);
                            add_grad!(row as isize + 1, col as isize);
                            add_grad!(row as isize - 1, col as isize);
                            row_grad[col] = if n_g > 0 { sum_g / n_g as f32 } else { 0.0 };
                        }
                    });

                let mut grad_sum = vec![0.0f64; rows * cols];
                let mut grad_count = vec![0i64; rows * cols];
                for row in 0..rows {
                    let mut row_sum = 0.0f64;
                    let mut row_count = 0i64;
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        if valid[idx] {
                            row_sum += grad_mag[idx] as f64;
                            row_count += 1;
                        }
                        if row > 0 {
                            let above = Self::idx(row - 1, col, cols);
                            grad_sum[idx] = row_sum + grad_sum[above];
                            grad_count[idx] = row_count + grad_count[above];
                        } else {
                            grad_sum[idx] = row_sum;
                            grad_count[idx] = row_count;
                        }
                    }
                }

                let mut local_map = vec![kappa_base; rows * cols];
                for row in 0..rows {
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        if !valid[idx] {
                            continue;
                        }
                        let y1 = (row as isize - radius).max(0) as usize;
                        let x1 = (col as isize - radius).max(0) as usize;
                        let y2 = (row as isize + radius).min(rows as isize - 1) as usize;
                        let x2 = (col as isize + radius).min(cols as isize - 1) as usize;
                        let n = Self::rect_count(&grad_count, cols, y1, x1, y2, x2);
                        if n <= 0 {
                            continue;
                        }
                        let local_mean = (Self::rect_sum(&grad_sum, cols, y1, x1, y2, x2)
                            / n as f64) as f32;
                        let local_sigma = local_mean.max(1.0e-6);
                        let blended_sigma =
                            ((1.0 - adaptivity) * sigma + adaptivity * local_sigma).max(1.0e-6);
                        local_map[idx] = blended_sigma * kappa_factor;
                    }
                }

                kappa_base_map = Some(local_map);
            }

            for it in 0..diffusion_iterations {
                let decay = if diffusion_iterations > 1 {
                    1.0 - 0.50 * (it as f32 / (diffusion_iterations - 1) as f32)
                } else {
                    1.0
                };
                let kappa = (kappa_base * decay).max(1.0e-6);
                let kappa2_inv_global = 1.0 / (kappa * kappa);
                let kappa_base_map_ref = kappa_base_map.as_ref();

                diff_a_nxt
                    .par_chunks_mut(cols)
                    .zip(diff_b_nxt.par_chunks_mut(cols))
                    .enumerate()
                    .for_each(|(row, (row_an, row_bn))| {
                        for col in 0..cols {
                            let idx = row * cols + col;
                            if !valid[idx] {
                                row_an[col] = 0.0;
                                row_bn[col] = 0.0;
                                continue;
                            }

                            let a0 = smooth_a[idx];
                            let b0 = smooth_b[idx];
                            let kappa2_inv = if let Some(local_map) = kappa_base_map_ref {
                                let local_kappa = (local_map[idx] * decay).max(1.0e-6);
                                1.0 / (local_kappa * local_kappa)
                            } else {
                                kappa2_inv_global
                            };
                            let mut flux_a = 0.0f32;
                            let mut flux_b = 0.0f32;

                            let mut add_flux = |r: isize, c: isize| {
                                if r < 0 || c < 0 || r >= rows as isize || c >= cols as isize {
                                    return;
                                }
                                let ni = r as usize * cols + c as usize;
                                if !valid[ni] {
                                    return;
                                }
                                let da = smooth_a[ni] - a0;
                                let db = smooth_b[ni] - b0;
                                let mag2 = da * da + db * db;
                                let g = 1.0 / (1.0 + mag2 * kappa2_inv);
                                flux_a += g * da;
                                flux_b += g * db;
                            };

                            add_flux(row as isize, col as isize + 1);
                            add_flux(row as isize, col as isize - 1);
                            add_flux(row as isize + 1, col as isize);
                            add_flux(row as isize - 1, col as isize);

                            row_an[col] = a0 + tau * flux_a;
                            row_bn[col] = b0 + tau * flux_b;
                        }
                    });

                std::mem::swap(&mut smooth_a, &mut diff_a_nxt);
                std::mem::swap(&mut smooth_b, &mut diff_b_nxt);
            }

            let div: Vec<f32> = (0..rows * cols)
                .into_par_iter()
                .map(|idx| {
                    if !valid[idx] {
                        return 0.0f32;
                    }
                    let row = idx / cols;
                    let col = idx % cols;
                    let a_curr = smooth_a[idx] * gradient_scale;
                    let a_left = if col > 0 && valid[row * cols + col - 1] {
                        smooth_a[row * cols + col - 1] * gradient_scale
                    } else {
                        a_curr
                    };
                    let b_curr = smooth_b[idx] * gradient_scale;
                    let b_up = if row > 0 && valid[(row - 1) * cols + col] {
                        smooth_b[(row - 1) * cols + col] * gradient_scale
                    } else {
                        b_curr
                    };
                    (a_curr - a_left) * res_x + (b_curr - b_up) * res_y
                })
                .collect();

            for _ in 0..MAX_JACOBI {
                let max_change: f32 = z_nxt
                    .par_chunks_mut(cols)
                    .enumerate()
                    .map(|(row, row_nxt)| {
                        let mut local_max = 0.0f32;
                        for col in 0..cols {
                            let idx = row * cols + col;
                            if !valid[idx] {
                                row_nxt[col] = nodata;
                                continue;
                            }
                            let mut sum_nbr = 0.0f32;
                            let mut n_nbr = 0u32;
                            macro_rules! try_nbr {
                                ($r:expr, $c:expr) => {
                                    if $r >= 0
                                        && $c >= 0
                                        && ($r as usize) < rows
                                        && ($c as usize) < cols
                                    {
                                        let ni = $r as usize * cols + $c as usize;
                                        if valid[ni] {
                                            sum_nbr += z_cur[ni];
                                            n_nbr += 1;
                                        }
                                    }
                                };
                            }
                            try_nbr!(row as isize, col as isize + 1);
                            try_nbr!(row as isize, col as isize - 1);
                            try_nbr!(row as isize + 1, col as isize);
                            try_nbr!(row as isize - 1, col as isize);
                            let new_z = (settings.lambda * dem_orig[idx] + sum_nbr + div[idx])
                                / (settings.lambda + n_nbr as f32);
                            row_nxt[col] = new_z;
                            local_max = local_max.max((new_z - z_cur[idx]).abs());
                        }
                        local_max
                    })
                    .reduce(|| 0.0_f32, f32::max);

                std::mem::swap(&mut z_cur, &mut z_nxt);
                if max_change < settings.convergence_threshold {
                    break;
                }
            }

            if settings.outer_convergence_threshold > 0.0 {
                let outer_change = z_cur
                    .par_iter()
                    .zip(z_prev_outer.par_iter())
                    .zip(valid.par_iter())
                    .map(|((&a, &b), &ok)| if ok { (a - b).abs() } else { 0.0f32 })
                    .reduce(|| 0.0f32, f32::max);
                if outer_change < settings.outer_convergence_threshold {
                    break;
                }
            }
        }

        z_cur
    }

    fn rect_sum(sum: &[f64], cols: usize, y1: usize, x1: usize, y2: usize, x2: usize) -> f64 {
        let a = sum[Self::idx(y2, x2, cols)];
        let b = if y1 > 0 { sum[Self::idx(y1 - 1, x2, cols)] } else { 0.0 };
        let c = if x1 > 0 { sum[Self::idx(y2, x1 - 1, cols)] } else { 0.0 };
        let d = if y1 > 0 && x1 > 0 { sum[Self::idx(y1 - 1, x1 - 1, cols)] } else { 0.0 };
        a - b - c + d
    }

    fn rect_count(count: &[i64], cols: usize, y1: usize, x1: usize, y2: usize, x2: usize) -> i64 {
        let a = count[Self::idx(y2, x2, cols)];
        let b = if y1 > 0 { count[Self::idx(y1 - 1, x2, cols)] } else { 0 };
        let c = if x1 > 0 { count[Self::idx(y2, x1 - 1, cols)] } else { 0 };
        let d = if y1 > 0 && x1 > 0 { count[Self::idx(y1 - 1, x1 - 1, cols)] } else { 0 };
        a - b - c + d
    }

    fn build_integrals(input: &Raster, band: isize) -> (Vec<f64>, Vec<f64>, Vec<i64>) {
        let rows = input.rows;
        let cols = input.cols;
        let mut sum = vec![0.0; rows * cols];
        let mut sum_sq = vec![0.0; rows * cols];
        let mut count = vec![0i64; rows * cols];

        for row in 0..rows {
            let mut row_sum = 0.0;
            let mut row_sum_sq = 0.0;
            let mut row_count = 0i64;
            for col in 0..cols {
                let val = input.get(band, row as isize, col as isize);
                if !input.is_nodata(val) {
                    row_sum += val;
                    row_sum_sq += val * val;
                    row_count += 1;
                }
                let idx = Self::idx(row, col, cols);
                if row > 0 {
                    let above = Self::idx(row - 1, col, cols);
                    sum[idx] = row_sum + sum[above];
                    sum_sq[idx] = row_sum_sq + sum_sq[above];
                    count[idx] = row_count + count[above];
                } else {
                    sum[idx] = row_sum;
                    sum_sq[idx] = row_sum_sq;
                    count[idx] = row_count;
                }
            }
        }

        (sum, sum_sq, count)
    }

    fn run_local_window_metric(
        args: &ToolArgs,
        ctx: &ToolContext,
        standardize: bool,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let (filter_size_x, filter_size_y) = Self::parse_filter_sizes(args);
        let mid_x = filter_size_x / 2;
        let mid_y = filter_size_y / 2;

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info(if standardize {
                "running deviation_from_mean_elevation"
            } else {
                "running difference_from_mean_elevation"
            });
            let (sum, sum_sq, count) = Self::build_integrals(&input, band);
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid_y);
                        let x1 = c.saturating_sub(mid_x);
                        let y2 = (r + mid_y).min(rows - 1);
                        let x2 = (c + mid_x).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n == 0 {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let n_f = n as f64;
                        let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                        let mean = local_sum / n_f;
                        if standardize {
                            let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                            let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                            let std_dev = variance.sqrt();
                            row_out[c] = if std_dev > 0.0 { (z - mean) / std_dev } else { 0.0 };
                        } else {
                            row_out[c] = z - mean;
                        }
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
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_difference_from_mean_elevation(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        Self::run_local_window_metric(args, ctx, false)
    }

    fn run_deviation_from_mean_elevation(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        Self::run_local_window_metric(args, ctx, true)
    }

    fn run_standard_deviation_of_slope(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let filter_size = args
            .get("filter_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(11);
        let mut local_args = args.clone();
        local_args.insert("filter_size_x".to_string(), json!(filter_size));
        local_args.insert("filter_size_y".to_string(), json!(filter_size));
        let (filter_size_x, filter_size_y) = Self::parse_filter_sizes(&local_args);
        let mid_x = filter_size_x / 2;
        let mid_y = filter_size_y / 2;
        let z_factor = args
            .get("z_factor")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let cell = input.cell_size_x.abs().max(f64::EPSILON);
        let eight_grid_res = 8.0 * cell;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running standard_deviation_of_slope");

            let slope_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let row = r as isize;
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let col = c as isize;
                        let z = input.get(band, row, col);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let sample = |rr: isize, cc: isize| {
                            let v = input.get(band, rr, cc);
                            if input.is_nodata(v) { z * z_factor } else { v * z_factor }
                        };

                        let n0 = sample(row - 1, col - 1);
                        let n1 = sample(row - 1, col);
                        let n2 = sample(row - 1, col + 1);
                        let n3 = sample(row, col + 1);
                        let n4 = sample(row + 1, col + 1);
                        let n5 = sample(row + 1, col);
                        let n6 = sample(row + 1, col - 1);
                        let n7 = sample(row, col - 1);

                        let fy = (n6 - n4 + 2.0 * (n7 - n3) + n0 - n2) / eight_grid_res;
                        let fx = (n2 - n4 + 2.0 * (n1 - n5) + n0 - n6) / eight_grid_res;
                        row_out[c] = (fx.mul_add(fx, fy * fy)).sqrt().atan().to_degrees();
                    }
                    row_out
                })
                .collect();

            let mut slope_raster = input.clone();
            for (r, row) in slope_rows.iter().enumerate() {
                slope_raster.set_row_slice(band, r as isize, row).map_err(|e| {
                    ToolError::Execution(format!("failed writing slope row {}: {}", r, e))
                })?;
            }

            let (sum, sum_sq, count) = Self::build_integrals(&slope_raster, band);
            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(band, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(mid_y);
                        let x1 = c.saturating_sub(mid_x);
                        let y2 = (r + mid_y).min(rows - 1);
                        let x2 = (c + mid_x).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n <= 0 {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let n_f = n as f64;
                        let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                        let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                        let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                        row_out[c] = variance.sqrt();
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
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_max_difference_from_mean(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;
        let (min_scale, max_scale, step_size) = Self::parse_scale_settings(args);

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running max_difference_from_mean");

            let (sum, _, count) = Self::build_integrals(&input, band);

            for r in 0..rows {
                let fill = vec![nodata; cols];
                output_mag.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e))
                })?;
                output_scale.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing scale row {}: {}", r, e))
                })?;
            }

            let mut scales = Vec::new();
            let mut s = min_scale;
            while s <= max_scale {
                scales.push(s);
                if let Some(next) = s.checked_add(step_size) {
                    s = next;
                } else {
                    break;
                }
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        for c in 0..cols {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n <= 0 {
                                row_out[c] = 0.0;
                                continue;
                            }
                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            let mean = local_sum / n as f64;
                            row_out[c] = z - mean;
                        }
                        row_out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, z2) in row.iter().enumerate().take(cols) {
                        let z2 = *z2;
                        if z2 == nodata {
                            continue;
                        }
                        let z1 = output_mag.get(band, r as isize, c as isize);
                        if z1 == nodata || z2 * z2 > z1 * z1 {
                            output_mag.set(band, r as isize, c as isize, z2).map_err(|e| {
                                ToolError::Execution(format!("failed writing max diff at row {} col {}: {}", r, c, e))
                            })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!("failed writing scale at row {} col {}: {}", r, c, e))
                                })?;
                        }
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn run_max_elevation_deviation(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;
        let (min_scale, max_scale, step_size) = Self::parse_scale_settings(args);

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running max_elevation_deviation");

            let (sum, sum_sq, count) = Self::build_integrals(&input, band);

            for r in 0..rows {
                let fill = vec![nodata; cols];
                output_mag.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e))
                })?;
                output_scale.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing scale row {}: {}", r, e))
                })?;
            }

            let mut scales = Vec::new();
            let mut s = min_scale;
            while s <= max_scale {
                scales.push(s);
                if let Some(next) = s.checked_add(step_size) {
                    s = next;
                } else {
                    break;
                }
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        for c in 0..cols {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n <= 1 {
                                row_out[c] = 0.0;
                                continue;
                            }
                            let n_f = n as f64;
                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                            let mean = local_sum / n_f;
                            let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                            let std_dev = variance.sqrt();
                            row_out[c] = if std_dev > 0.0 { (z - mean) / std_dev } else { 0.0 };
                        }
                        row_out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, z2) in row.iter().enumerate().take(cols) {
                        let z2 = *z2;
                        if z2 == nodata {
                            continue;
                        }
                        let z1 = output_mag.get(band, r as isize, c as isize);
                        if z1 == nodata || z2 * z2 > z1 * z1 {
                            output_mag.set(band, r as isize, c as isize, z2).map_err(|e| {
                                ToolError::Execution(format!("failed writing max deviation at row {} col {}: {}", r, c, e))
                            })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!("failed writing scale at row {} col {}: {}", r, c, e))
                                })?;
                        }
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn validate_topographic_position_animation(args: &ToolArgs) -> Result<(), ToolError> {
        let _ = Self::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        if let Some(name) = args.get("palette").and_then(|v| v.as_str()) {
            if LegacyPalette::from_name(name).is_none() {
                return Err(ToolError::Validation(format!(
                    "unsupported palette '{}'; supported: {}",
                    name,
                    LegacyPalette::supported_names().join(", ")
                )));
            }
        }
        if let Some(v) = args.get("step_nonlinearity").and_then(|v| v.as_f64()) {
            if !(1.0..=4.0).contains(&v) {
                return Err(ToolError::Validation(
                    "step_nonlinearity must be in [1, 4]".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn run_topographic_position_animation(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_html = parse_optional_output_path(args, "output")?
            .unwrap_or_else(|| std::env::temp_dir().join("topographic_position_animation.html"));
        let palette_name = args
            .get("palette")
            .and_then(|v| v.as_str())
            .unwrap_or("soft");
        let palette = LegacyPalette::from_name(palette_name).ok_or_else(|| {
            ToolError::Validation(format!(
                "unsupported palette '{}'; supported: {}",
                palette_name,
                LegacyPalette::supported_names().join(", ")
            ))
        })?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1) as usize;
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .max(1) as usize;
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(1.0, 4.0);
        let image_height = args
            .get("image_height")
            .or_else(|| args.get("height"))
            .and_then(|v| v.as_u64())
            .unwrap_or(600)
            .max(50) as usize;
        let delay_ms = args
            .get("delay")
            .and_then(|v| v.as_u64())
            .unwrap_or(250) as u32;
        let label = args
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let use_dev_max = args
            .get("use_dev_max")
            .or_else(|| args.get("dev_max"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(parent) = output_html.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }
        let gif_path = output_html.with_extension("gif");
        let gif_name = gif_path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| ToolError::Execution("invalid GIF output path".to_string()))?
            .to_string();

        let input = Self::load_raster(&input_path)?;
        if input.bands != 1 {
            return Err(ToolError::Validation(
                "topographic_position_animation requires a single-band DEM".to_string(),
            ));
        }
        let rows = input.rows;
        let cols = input.cols;
        let nodata = input.nodata;
        let width = ((image_height as f64) * (cols as f64 / rows as f64)).round().max(1.0) as usize;

        let palette_vals = palette.get_palette();
        let palette_vals: Vec<(f64, f64, f64)> = palette_vals
            .into_iter()
            .map(|(r, g, b)| {
                (
                    r.clamp(0.0, 255.0) as f64,
                    g.clamp(0.0, 255.0) as f64,
                    b.clamp(0.0, 255.0) as f64,
                )
            })
            .collect();
        let p_last = (palette_vals.len().saturating_sub(1)).max(1) as f64;

        let mut z_factor = 1.0_f64;
        if input.y_min >= -90.0 && input.y_max() <= 90.0 && input.x_min >= -180.0 && input.x_max() <= 180.0 {
            let mid_lat = ((input.y_min + input.y_max()) * 0.5).to_radians();
            z_factor = 1.0 / (111_320.0 * mid_lat.cos().abs().max(1.0e-8));
        }

        let (sum, sum_sq, count) = Self::build_integrals(&input, 0);
        let eight_grid_res = (input.cell_size_x.abs() * 8.0).max(f64::EPSILON);
        let altitude = 30.0_f64.to_radians();
        let azimuth = (315.0_f64 - 90.0).to_radians();
        let sin_theta = altitude.sin();
        let cos_theta = altitude.cos();
        let half_pi = std::f64::consts::PI / 2.0;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut hillshade = vec![0.0_f64; rows * cols];
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let z = input.get(0, row, col);
                if input.is_nodata(z) {
                    continue;
                }
                let z_scaled = z * z_factor;
                let mut n = [0.0_f64; 8];
                for i in 0..8 {
                    let zn = input.get(0, row + dy[i], col + dx[i]);
                    n[i] = if input.is_nodata(zn) { z_scaled } else { zn * z_factor };
                }
                let fy = (n[6] - n[4] + 2.0 * (n[7] - n[3]) + n[0] - n[2]) / eight_grid_res;
                let fx = (n[2] - n[4] + 2.0 * (n[1] - n[5]) + n[0] - n[6]) / eight_grid_res;
                let mut tan_slope = (fx * fx + fy * fy).sqrt();
                if tan_slope < 0.00017 {
                    tan_slope = 0.00017;
                }
                let aspect = if fx != 0.0 {
                    std::f64::consts::PI - (fy / fx).atan() + half_pi * (fx / fx.abs())
                } else {
                    std::f64::consts::PI
                };
                let term1 = tan_slope / (1.0 + tan_slope * tan_slope).sqrt();
                let term2 = sin_theta / tan_slope;
                let term3 = cos_theta * (azimuth - aspect).sin();
                hillshade[Self::idx(row as usize, col as usize, cols)] =
                    (term1 * (term2 - term3)).max(0.0);
            }
        }

        let file_out = File::create(&gif_path)
            .map_err(|e| ToolError::Execution(format!("failed creating GIF output: {e}")))?;
        let mut encoder = GifEncoder::new(BufWriter::new(file_out));
        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| ToolError::Execution(format!("failed setting GIF repeat: {e}")))?;
        let delay = Delay::from_numer_denom_ms(delay_ms, 1);
        let mut output_mag = vec![f64::NEG_INFINITY; rows * cols];
        let mut frames_written = 0usize;

        for step_idx in 0..num_steps {
            let midpoint = min_scale + (step_idx as f64).powf(step_nonlinearity).floor() as usize;
            let filter_size = midpoint * 2 + 1;
            if filter_size > rows.max(cols) {
                break;
            }

            let row_data: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut row_out = vec![nodata; cols];
                    for c in 0..cols {
                        let z = input.get(0, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(midpoint);
                        let x1 = c.saturating_sub(midpoint);
                        let y2 = (r + midpoint).min(rows - 1);
                        let x2 = (c + midpoint).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n <= 1 {
                            row_out[c] = 0.0;
                            continue;
                        }
                        let n_f = n as f64;
                        let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                        let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                        let mean = local_sum / n_f;
                        let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                        let std_dev = variance.sqrt();
                        row_out[c] = if std_dev > 0.0 { (z - mean) / std_dev } else { 0.0 };
                    }
                    row_out
                })
                .collect();

            let mut img = RgbaImage::new(cols as u32, rows as u32);
            for r in 0..rows {
                for c in 0..cols {
                    let z = input.get(0, r as isize, c as isize);
                    if input.is_nodata(z) {
                        img.put_pixel(c as u32, r as u32, Rgba([0, 0, 0, 0]));
                        continue;
                    }
                    let idx = Self::idx(r, c, cols);
                    let current = row_data[r][c];
                    let shown = if use_dev_max {
                        if !output_mag[idx].is_finite() || current.abs() > output_mag[idx].abs() {
                            output_mag[idx] = current;
                        }
                        output_mag[idx]
                    } else {
                        output_mag[idx] = current;
                        current
                    };
                    let clamped = shown.clamp(-2.5, 2.5);
                    let proportion = (clamped + 2.5) / 5.0;
                    let idxf = proportion * p_last;
                    let i0 = idxf.floor() as usize;
                    let i1 = (i0 + 1).min(palette_vals.len() - 1);
                    let t = (idxf - i0 as f64).clamp(0.0, 1.0);
                    let (r0, g0, b0) = palette_vals[i0];
                    let (r1, g1, b1) = palette_vals[i1];
                    let hs = 0.7 + hillshade[idx].clamp(0.0, 1.0) * 0.3;
                    let red = ((r0 + t * (r1 - r0)) * hs).round().clamp(0.0, 255.0) as u8;
                    let green = ((g0 + t * (g1 - g0)) * hs).round().clamp(0.0, 255.0) as u8;
                    let blue = ((b0 + t * (b1 - b0)) * hs).round().clamp(0.0, 255.0) as u8;
                    img.put_pixel(c as u32, r as u32, Rgba([red, green, blue, 255]));
                }
            }

            encoder
                .encode_frame(Frame::from_parts(img, 0, 0, delay))
                .map_err(|e| ToolError::Execution(format!("failed encoding GIF frame: {e}")))?;
            frames_written += 1;
            coalescer.emit_unit_fraction(ctx.progress, (step_idx + 1) as f64 / num_steps as f64);
        }

        if frames_written == 0 {
            return Err(ToolError::Execution(
                "topographic_position_animation produced no frames; reduce min_scale or num_steps"
                    .to_string(),
            ));
        }

        Self::write_animation_html(
            &output_html,
            "Topographic Position Animation",
            "Topographic Position Animation",
            &label,
            &gif_name,
            width,
            image_height,
            &[
                ("Input DEM", input_path.clone()),
                ("Mode", if use_dev_max { "DEVmax".to_string() } else { "DEV".to_string() }),
                ("Minimum scale", min_scale.to_string()),
                ("Steps", frames_written.to_string()),
            ],
        )?;

        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_gif(
            output_html.to_string_lossy().to_string(),
            gif_path.to_string_lossy().to_string(),
        ))
    }

    fn pack_rgba(r: u32, g: u32, b: u32, a: u32) -> f64 {
        ((a << 24) | (b << 16) | (g << 8) | r) as f64
    }

    fn run_multiscale_topographic_position_image(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let local_path = parse_raster_path_arg(args, "local")?;
        let meso_path = parse_raster_path_arg(args, "meso")?;
        let broad_path = parse_raster_path_arg(args, "broad")?;
        let hillshade_path = args
            .get("hillshade")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let output_path = parse_optional_output_path(args, "output")?;
        let lightness = args
            .get("lightness")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.2)
            .max(0.0);

        let local = Self::load_raster(&local_path)?;
        let meso = Self::load_raster(&meso_path)?;
        let broad = Self::load_raster(&broad_path)?;
        if local.rows != meso.rows || local.cols != meso.cols || local.rows != broad.rows || local.cols != broad.cols {
            return Err(ToolError::Validation(
                "local, meso, and broad rasters must share identical dimensions".to_string(),
            ));
        }
        if local.bands != 1 || meso.bands != 1 || broad.bands != 1 {
            return Err(ToolError::Validation(
                "local, meso, and broad rasters must be single-band".to_string(),
            ));
        }

        let hillshade = if let Some(path) = hillshade_path {
            let hs = Self::load_raster(&path)?;
            if hs.rows != local.rows || hs.cols != local.cols || hs.bands != 1 {
                return Err(ToolError::Validation(
                    "hillshade raster must be single-band and match local/meso/broad dimensions"
                        .to_string(),
                ));
            }
            Some(hs)
        } else {
            None
        };

        let mut output = Raster::new(RasterConfig {
            rows: broad.rows,
            cols: broad.cols,
            bands: 1,
            x_min: broad.x_min,
            y_min: broad.y_min,
            cell_size: broad.cell_size_x,
            cell_size_y: Some(broad.cell_size_y),
            nodata: 0.0,
            data_type: DataType::U32,
            crs: broad.crs.clone(),
            metadata: {
                let mut md = broad.metadata.clone();
                md.push(("color_interpretation".to_string(), "packed_rgb".to_string()));
                md
            },
        });

        let hs_stats = hillshade.as_ref().map(|hs| {
            let mut min_v = f64::INFINITY;
            let mut max_v = f64::NEG_INFINITY;
            for row in 0..hs.rows as isize {
                for col in 0..hs.cols as isize {
                    let z = hs.get(0, row, col);
                    if hs.is_nodata(z) {
                        continue;
                    }
                    min_v = min_v.min(z);
                    max_v = max_v.max(z);
                }
            }
            if min_v.is_finite() && max_v.is_finite() {
                (min_v, (max_v - min_v).max(1e-12))
            } else {
                (0.0, 1.0)
            }
        });

        let logistic = |v: f64| -> f64 {
            (512.0 / (1.0 + (-(lightness * v.abs())).exp())).floor() - 256.0
        };

        let rows = local.rows as isize;
        let cols = local.cols as isize;
        let n_local = local.nodata;
        let n_meso = meso.nodata;
        let n_broad = broad.nodata;

        ctx.progress.info("running multiscale_topographic_position_image");
        for row in 0..rows {
            let mut row_data = vec![0.0; cols as usize];
            for col in 0..cols {
                let r_broad = broad.get(0, row, col);
                let g_meso = meso.get(0, row, col);
                let b_local = local.get(0, row, col);
                if r_broad == n_broad || g_meso == n_meso || b_local == n_local {
                    continue;
                }

                let mut r = logistic(r_broad).clamp(0.0, 255.0);
                let mut g = logistic(g_meso).clamp(0.0, 255.0);
                let mut b = logistic(b_local).clamp(0.0, 255.0);

                if let (Some(hs), Some((hs_min, hs_rng))) = (hillshade.as_ref(), hs_stats.as_ref()) {
                    let h = hs.get(0, row, col);
                    if h != hs.nodata {
                        let shade = ((h - *hs_min) / *hs_rng).clamp(0.0, 1.0);
                        r *= shade;
                        g *= shade;
                        b *= shade;
                    }
                }

                row_data[col as usize] = Self::pack_rgba(
                    r.round() as u32,
                    g.round() as u32,
                    b.round() as u32,
                    255,
                );
            }
            output
                .set_row_slice(0, row, &row_data)
                .map_err(|e| ToolError::Execution(format!("failed writing row {}: {}", row, e)))?;
            coalescer.emit_unit_fraction(ctx.progress, (row + 1) as f64 / rows as f64);
        }

        let output_locator = Self::write_or_store_output(output, output_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result(output_locator))
    }

    fn run_multiscale_elevation_percentile(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;

        let mut min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(4)
            .max(1);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let sig_digits = args
            .get("sig_digits")
            .and_then(|v| v.as_i64())
            .unwrap_or(2)
            .clamp(0, 9) as i32;
        let multiplier = 10f64.powi(sig_digits);
        if min_scale == 0 {
            min_scale = 1;
        }

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        for band_idx in 0..bands {
            let band = band_idx as isize;
            ctx.progress.info("running multiscale_elevation_percentile");

            for r in 0..rows {
                let fill = vec![nodata; cols];
                output_mag.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e))
                })?;
                output_scale.set_row_slice(band, r as isize, &fill).map_err(|e| {
                    ToolError::Execution(format!("failed initializing scale row {}: {}", r, e))
                })?;
            }

            let mut scales = Vec::new();
            let mut s = min_scale;
            while s <= max_scale {
                scales.push(s);
                if let Some(next) = s.checked_add(step_size) {
                    s = next;
                } else {
                    break;
                }
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        for c in 0..cols {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let z_bin = (z * multiplier).floor();
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let mut n = 0.0;
                            let mut n_less = 0.0;
                            for rr in y1..=y2 {
                                for cc in x1..=x2 {
                                    let v = input.get(band, rr as isize, cc as isize);
                                    if !input.is_nodata(v) {
                                        n += 1.0;
                                        if (v * multiplier).floor() < z_bin {
                                            n_less += 1.0;
                                        }
                                    }
                                }
                            }
                            if n > 0.0 {
                                row_out[c] = n_less / n * 100.0;
                            }
                        }
                        row_out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, p2) in row.iter().enumerate().take(cols) {
                        let p2 = *p2;
                        if p2 == nodata {
                            continue;
                        }
                        let p1 = output_mag.get(band, r as isize, c as isize);
                        if p1 == nodata || (p2 - 50.0).abs() > (p1 - 50.0).abs() {
                            output_mag.set(band, r as isize, c as isize, p2).map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing percentile value at row {} col {}: {}",
                                    r, c, e
                                ))
                            })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing percentile scale at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                        }
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn panel_dev(
        z: f64,
        sum: &[f64],
        sum_sq: &[f64],
        count: &[i64],
        cols: usize,
        y1: usize,
        x1: usize,
        y2: usize,
        x2: usize,
    ) -> Option<f64> {
        let n = Self::rect_count(count, cols, y1, x1, y2, x2);
        if n <= 3 {
            return None;
        }
        let n_f = n as f64;
        let local_sum = Self::rect_sum(sum, cols, y1, x1, y2, x2);
        let local_sum_sq = Self::rect_sum(sum_sq, cols, y1, x1, y2, x2);
        let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
        let std_dev = variance.sqrt();
        if std_dev <= 0.0 {
            return None;
        }
        Some((z - local_sum / n_f) / std_dev)
    }

    fn run_max_anisotropy_dev(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3)
            .max(3);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2)
            .max(1);

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let mut scales = Vec::new();
        let mut s = min_scale;
        while s <= max_scale {
            scales.push(s);
            if let Some(next) = s.checked_add(step_size) {
                s = next;
            } else {
                break;
            }
        }

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let (sum, sum_sq, count) = Self::build_integrals(&input, band);

            for r in 0..rows {
                output_mag
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e)))?;
                output_scale
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing scale row {}: {}", r, e)))?;
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                let middle_radius = ((midpoint * 2 + 1) / 6).max(1);

                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut row_out = vec![nodata; cols];
                        if r < midpoint || r + midpoint >= rows {
                            return row_out;
                        }
                        for (c, cell_out) in row_out.iter_mut().enumerate().take(cols) {
                            if c < midpoint || c + midpoint >= cols {
                                continue;
                            }
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }

                            let oy1 = r - midpoint;
                            let oy2 = r + midpoint;
                            let ox1 = c - midpoint;
                            let ox2 = c + midpoint;
                            let iy1 = r.saturating_sub(middle_radius);
                            let iy2 = (r + middle_radius).min(rows - 1);
                            let ix1 = c.saturating_sub(middle_radius);
                            let ix2 = (c + middle_radius).min(cols - 1);

                            let overall = match Self::panel_dev(
                                z, &sum, &sum_sq, &count, cols, oy1, ox1, oy2, ox2,
                            ) {
                                Some(v) => v,
                                None => continue,
                            };

                            let mut sq_sum = 0.0;
                            let mut valid = 0usize;

                            if let Some(v) = Self::panel_dev(
                                z, &sum, &sum_sq, &count, cols, oy1, ix1, oy2, ix2,
                            ) {
                                let d = v - overall;
                                sq_sum += d * d;
                                valid += 1;
                            }
                            if let Some(v) = Self::panel_dev(
                                z, &sum, &sum_sq, &count, cols, iy1, ox1, iy2, ox2,
                            ) {
                                let d = v - overall;
                                sq_sum += d * d;
                                valid += 1;
                            }

                            let diag_center = Self::panel_dev(
                                z, &sum, &sum_sq, &count, cols, iy1, ix1, iy2, ix2,
                            );

                            let ne_sw = if oy1 <= iy1 && ix2 <= ox2 {
                                let top_right = Self::panel_dev(
                                    z,
                                    &sum,
                                    &sum_sq,
                                    &count,
                                    cols,
                                    oy1,
                                    ix2,
                                    iy1,
                                    ox2,
                                );
                                let bottom_left = Self::panel_dev(
                                    z,
                                    &sum,
                                    &sum_sq,
                                    &count,
                                    cols,
                                    iy2,
                                    ox1,
                                    oy2,
                                    ix1,
                                );
                                if let (Some(a), Some(b), Some(cn)) = (top_right, diag_center, bottom_left) {
                                    Some((a + cn + b) / 3.0)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if let Some(v) = ne_sw {
                                let d = v - overall;
                                sq_sum += d * d;
                                valid += 1;
                            }

                            let nw_se = if oy1 <= iy1 && ox1 <= ix1 {
                                let top_left = Self::panel_dev(
                                    z,
                                    &sum,
                                    &sum_sq,
                                    &count,
                                    cols,
                                    oy1,
                                    ox1,
                                    iy1,
                                    ix1,
                                );
                                let bottom_right = Self::panel_dev(
                                    z,
                                    &sum,
                                    &sum_sq,
                                    &count,
                                    cols,
                                    iy2,
                                    ix2,
                                    oy2,
                                    ox2,
                                );
                                if let (Some(a), Some(b), Some(cn)) = (top_left, diag_center, bottom_right) {
                                    Some((a + cn + b) / 3.0)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            if let Some(v) = nw_se {
                                let d = v - overall;
                                sq_sum += d * d;
                                valid += 1;
                            }

                            if valid > 0 {
                                *cell_out = (sq_sum / valid as f64).sqrt();
                            }
                        }
                        row_out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, z2) in row.iter().enumerate().take(cols) {
                        let z2 = *z2;
                        if z2 == nodata {
                            continue;
                        }
                        let z1 = output_mag.get(band, r as isize, c as isize);
                        if z1 == nodata || z2 * z2 > z1 * z1 {
                            output_mag
                                .set(band, r as isize, c as isize, z2)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing anisotropy value at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing anisotropy scale at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                        }
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn normal_from_raster(input: &Raster, band: isize, row: isize, col: isize, z_factor: f64) -> Option<[f64; 3]> {
        let z = input.get(band, row, col);
        if input.is_nodata(z) {
            return None;
        }
        let center = z * z_factor;
        let sample = |rr: isize, cc: isize| {
            let v = input.get(band, rr, cc);
            if input.is_nodata(v) { center } else { v * z_factor }
        };

        let n0 = sample(row - 1, col - 1);
        let n1 = sample(row - 1, col);
        let n2 = sample(row - 1, col + 1);
        let n3 = sample(row, col + 1);
        let n4 = sample(row + 1, col + 1);
        let n5 = sample(row + 1, col);
        let n6 = sample(row + 1, col - 1);
        let n7 = sample(row, col - 1);
        let c = 8.0 * input.cell_size_x.abs().max(f64::EPSILON);

        let a = -(n2 - n4 + 2.0 * (n1 - n5) + n0 - n6);
        let b = -(n6 - n4 + 2.0 * (n7 - n3) + n0 - n2);
        Some([a, b, c])
    }

    fn angle_between_normals(a: [f64; 3], b: [f64; 3]) -> f64 {
        let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let mag_a = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        let mag_b = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
        if mag_a <= 0.0 || mag_b <= 0.0 {
            return 0.0;
        }
        let cos_t = (dot / (mag_a * mag_b)).clamp(-1.0, 1.0);
        cos_t.acos().to_degrees()
    }

    fn unit_normal_from_raster(
        input: &Raster,
        band: isize,
        row: isize,
        col: isize,
        z_factor: f64,
    ) -> Option<[f64; 3]> {
        let n = Self::normal_from_raster(input, band, row, col, z_factor)?;
        let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if mag <= 0.0 {
            None
        } else {
            Some([n[0] / mag, n[1] / mag, n[2] / mag])
        }
    }

    fn make_nonlinear_scales(
        min_scale: usize,
        step: usize,
        num_steps: usize,
        step_nonlinearity: f64,
    ) -> Vec<usize> {
        let mut scales = Vec::with_capacity(num_steps.max(1));
        let mut last = 0usize;
        for i in 0..num_steps.max(1) {
            let offset = ((i as f64 * step as f64).powf(step_nonlinearity)).floor() as usize;
            let s = min_scale.saturating_add(offset).max(1);
            if scales.is_empty() || s > last {
                scales.push(s);
                last = s;
            }
        }
        if scales.is_empty() {
            scales.push(min_scale.max(1));
        }
        scales
    }

    fn gss_smooth_band(
        src: &[f64],
        i_n: &[u32],
        rows: usize,
        cols: usize,
        nodata: f64,
        midpoint: usize,
    ) -> Vec<f64> {
        let filter_size = midpoint * 2 + 1;
        if filter_size <= 3 {
            return src.to_vec();
        }

        let sigma = (midpoint as f64 + 0.5) / 3.0;
        if sigma < 1.8 {
            let recip = 1.0 / ((2.0 * std::f64::consts::PI).sqrt() * sigma);
            let two_sigma_sq = 2.0 * sigma * sigma;

            let mut filter_size_smooth = 3usize;
            for i in 0..250usize {
                let weight = recip * (-((i * i) as f64) / two_sigma_sq).exp();
                if weight <= 0.001 {
                    filter_size_smooth = i * 2 + 1;
                    break;
                }
            }
            if filter_size_smooth % 2 == 0 {
                filter_size_smooth += 1;
            }
            filter_size_smooth = filter_size_smooth.max(3);

            let midpoint_smoothed = (filter_size_smooth as f64 / 2.0).floor() as isize + 1;
            let mut offsets = Vec::new();
            for row in 0..filter_size {
                for col in 0..filter_size {
                    let x = col as isize - midpoint_smoothed;
                    let y = row as isize - midpoint_smoothed;
                    let weight = recip * (-((x * x + y * y) as f64) / two_sigma_sq).exp();
                    offsets.push((x, y, weight));
                }
            }

            let mut out = vec![nodata; rows * cols];
            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = src[idx];
                    if z == nodata {
                        continue;
                    }

                    let mut sum_w = 0.0;
                    let mut sum_z = 0.0;
                    for (dx, dy, w) in &offsets {
                        let rr = row as isize + *dy;
                        let cc = col as isize + *dx;
                        if rr < 0 || cc < 0 || rr >= rows as isize || cc >= cols as isize {
                            continue;
                        }
                        let zn = src[Self::idx(rr as usize, cc as usize, cols)];
                        if zn != nodata {
                            sum_w += *w;
                            sum_z += *w * zn;
                        }
                    }

                    if sum_w > 0.0 {
                        out[idx] = sum_z / sum_w;
                    }
                }
            }
            out
        } else {
            let n = 4usize;
            let w_ideal = (12.0 * sigma * sigma / n as f64 + 1.0).sqrt();
            let mut wl = w_ideal.floor() as isize;
            if wl % 2 == 0 {
                wl -= 1;
            }
            let wu = wl + 2;
            let m = ((12.0 * sigma * sigma
                - (n as isize * wl * wl) as f64
                - (4 * n as isize * wl) as f64
                - (3 * n as isize) as f64)
                / (-4 * wl - 4) as f64)
                .round() as isize;

            let mut current = src.to_vec();
            let mut next = vec![nodata; rows * cols];
            let mut integral = vec![0.0f64; rows * cols];

            for iteration_num in 0..n {
                let midpoint = if iteration_num as isize <= m {
                    (wl as f64 / 2.0).floor() as isize
                } else {
                    (wu as f64 / 2.0).floor() as isize
                };

                for row in 0..rows {
                    let mut row_sum = 0.0;
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        let val = if current[idx] == nodata { 0.0 } else { current[idx] };
                        row_sum += val;
                        integral[idx] = if row > 0 {
                            row_sum + integral[Self::idx(row - 1, col, cols)]
                        } else {
                            row_sum
                        };
                    }
                }

                for row in 0..rows {
                    let mut y1 = row as isize - midpoint - 1;
                    if y1 < 0 {
                        y1 = 0;
                    }
                    let mut y2 = row as isize + midpoint;
                    if y2 >= rows as isize {
                        y2 = rows as isize - 1;
                    }
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        if src[idx] == nodata {
                            next[idx] = nodata;
                            continue;
                        }

                        let mut x1 = col as isize - midpoint - 1;
                        if x1 < 0 {
                            x1 = 0;
                        }
                        let mut x2 = col as isize + midpoint;
                        if x2 >= cols as isize {
                            x2 = cols as isize - 1;
                        }

                        let y1u = y1 as usize;
                        let y2u = y2 as usize;
                        let x1u = x1 as usize;
                        let x2u = x2 as usize;

                        let num_cells = i_n[Self::idx(y2u, x2u, cols)]
                            + i_n[Self::idx(y1u, x1u, cols)]
                            - i_n[Self::idx(y1u, x2u, cols)]
                            - i_n[Self::idx(y2u, x1u, cols)];
                        if num_cells > 0 {
                            let sum = integral[Self::idx(y2u, x2u, cols)]
                                + integral[Self::idx(y1u, x1u, cols)]
                                - integral[Self::idx(y1u, x2u, cols)]
                                - integral[Self::idx(y2u, x1u, cols)];
                            next[idx] = sum / num_cells as f64;
                        } else {
                            next[idx] = 0.0;
                        }
                    }
                }

                std::mem::swap(&mut current, &mut next);
            }

            current
        }
    }

    fn run_multiscale_ei_lli(
        args: &ToolArgs,
        ctx: &ToolContext,
        elevated_mode: bool,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;

        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2)
            .max(1);
        let step = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(1);
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.1)
            .clamp(1.0, 4.0);

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let scales = Self::make_nonlinear_scales(min_scale, step, num_steps, step_nonlinearity);

        let mut output_mag = input.clone();
        let mut output_scale = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;

            let mut src = vec![nodata; rows * cols];
            let mut min_val = f64::INFINITY;
            for row in 0..rows {
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    let z = input.get(band, row as isize, col as isize);
                    src[idx] = z;
                    if z != nodata {
                        min_val = min_val.min(z);
                    }
                }
            }
            if min_val.is_finite() {
                for v in &mut src {
                    if *v != nodata {
                        *v -= min_val;
                    }
                }
            }

            let mut i_n = vec![0u32; rows * cols];
            for row in 0..rows {
                let mut row_sum = 0u32;
                for col in 0..cols {
                    let idx = Self::idx(row, col, cols);
                    if src[idx] != nodata {
                        row_sum += 1;
                    }
                    i_n[idx] = if row > 0 {
                        row_sum + i_n[Self::idx(row - 1, col, cols)]
                    } else {
                        row_sum
                    };
                }
            }

            for row in 0..rows {
                let mut mag_row = vec![nodata; cols];
                let scale_row = vec![nodata; cols];
                for col in 0..cols {
                    if src[Self::idx(row, col, cols)] != nodata {
                        mag_row[col] = 0.0;
                    }
                }
                output_mag
                    .set_row_slice(band, row as isize, &mag_row)
                    .map_err(|e| ToolError::Execution(format!("failed initializing magnitude row {}: {}", row, e)))?;
                output_scale
                    .set_row_slice(band, row as isize, &scale_row)
                    .map_err(|e| ToolError::Execution(format!("failed initializing scale row {}: {}", row, e)))?;
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                if midpoint * 2 + 1 > rows.max(cols) / 2 {
                    break;
                }

                let smooth = Self::gss_smooth_band(&src, &i_n, rows, cols, nodata, midpoint);

                let mut diff = vec![0.0f64; rows * cols];
                let mut sum_sq = 0.0f64;
                let mut n = 0usize;
                for i in 0..diff.len() {
                    if src[i] == nodata {
                        diff[i] = nodata;
                        continue;
                    }
                    let d = src[i] - smooth[i];
                    diff[i] = d;
                    if d != 0.0 {
                        sum_sq += d * d;
                        n += 1;
                    }
                }
                if n == 0 {
                    continue;
                }
                let std_dev = (sum_sq / n as f64).sqrt();
                if std_dev <= f64::EPSILON {
                    continue;
                }

                for row in 0..rows {
                    for col in 0..cols {
                        let idx = Self::idx(row, col, cols);
                        let d = diff[idx];
                        if d == nodata {
                            continue;
                        }
                        let scaled = if d != 0.0 { d / std_dev } else { 0.0 };
                        let cur = output_mag.get(band, row as isize, col as isize);
                        if elevated_mode {
                            if scaled > cur {
                                output_mag
                                    .set(band, row as isize, col as isize, scaled)
                                    .map_err(|e| ToolError::Execution(format!("failed writing MsEI at row {} col {}: {}", row, col, e)))?;
                                output_scale
                                    .set(band, row as isize, col as isize, midpoint as f64)
                                    .map_err(|e| ToolError::Execution(format!("failed writing MsEI scale at row {} col {}: {}", row, col, e)))?;
                            }
                        } else if scaled < 0.0 && -scaled > cur {
                            output_mag
                                .set(band, row as isize, col as isize, -scaled)
                                .map_err(|e| ToolError::Execution(format!("failed writing MsLLI at row {} col {}: {}", row, col, e)))?;
                            output_scale
                                .set(band, row as isize, col as isize, midpoint as f64)
                                .map_err(|e| ToolError::Execution(format!("failed writing MsLLI scale at row {} col {}: {}", row, col, e)))?;
                        }
                    }
                }

                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn run_multiscale_elevated_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        Self::run_multiscale_ei_lli(args, ctx, true)
    }

    fn run_multiscale_low_lying_index(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        Self::run_multiscale_ei_lli(args, ctx, false)
    }

    fn run_local_hypsometric_analysis(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;

        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(4)
            .max(1);
        let step = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10)
            .max(1);
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(1.0, 4.0);

        let input = Self::load_raster(&input_path)?;
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let scales = Self::make_nonlinear_scales(min_scale, step, num_steps, step_nonlinearity);

        let mut output_mag = input.clone();
        let mut output_scale = input.clone();

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let (sum, _, count) = Self::build_integrals(&input, band);

            for row in 0..rows {
                let mut mag_row = vec![nodata; cols];
                let scale_row = vec![nodata; cols];
                for (col, mag_cell) in mag_row.iter_mut().enumerate().take(cols) {
                    let z = input.get(band, row as isize, col as isize);
                    if !input.is_nodata(z) {
                        *mag_cell = 1.0;
                    }
                }
                output_mag
                    .set_row_slice(band, row as isize, &mag_row)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed initializing hypsometric magnitude row {}: {}",
                            row, e
                        ))
                    })?;
                output_scale
                    .set_row_slice(band, row as isize, &scale_row)
                    .map_err(|e| {
                        ToolError::Execution(format!(
                            "failed initializing hypsometric scale row {}: {}",
                            row, e
                        ))
                    })?;
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                let filter_size = midpoint * 2 + 1;

                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![nodata; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }

                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);

                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n <= 0 {
                                continue;
                            }

                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            let mean = local_sum / n as f64;

                            let mut min_val = f64::INFINITY;
                            let mut max_val = f64::NEG_INFINITY;
                            for rr in y1..=y2 {
                                for cc in x1..=x2 {
                                    let zv = input.get(band, rr as isize, cc as isize);
                                    if input.is_nodata(zv) {
                                        continue;
                                    }
                                    if zv < min_val {
                                        min_val = zv;
                                    }
                                    if zv > max_val {
                                        max_val = zv;
                                    }
                                }
                            }

                            if max_val > min_val {
                                *out_cell = (mean - min_val) / (max_val - min_val);
                            }
                        }
                        out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, v) in row.iter().enumerate().take(cols) {
                        if *v == nodata {
                            continue;
                        }
                        let cur = output_mag.get(band, r as isize, c as isize);
                        if cur == nodata || *v < cur {
                            output_mag.set(band, r as isize, c as isize, *v).map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing local HI at row {} col {}: {}",
                                    r, c, e
                                ))
                            })?;
                            output_scale
                                .set(band, r as isize, c as isize, filter_size as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing local HI scale at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                        }
                    }
                }

                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn build_unit_normal_component_integrals(
        input: &Raster,
        band: isize,
        z_factor: f64,
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<i64>) {
        let rows = input.rows;
        let cols = input.cols;
        let mut sum_x = vec![0.0; rows * cols];
        let mut sum_y = vec![0.0; rows * cols];
        let mut sum_z = vec![0.0; rows * cols];
        let mut count = vec![0i64; rows * cols];

        for row in 0..rows {
            let mut row_sum_x = 0.0;
            let mut row_sum_y = 0.0;
            let mut row_sum_z = 0.0;
            let mut row_count = 0i64;
            for col in 0..cols {
                if let Some(n) = Self::unit_normal_from_raster(
                    input,
                    band,
                    row as isize,
                    col as isize,
                    z_factor,
                ) {
                    row_sum_x += n[0];
                    row_sum_y += n[1];
                    row_sum_z += n[2];
                    row_count += 1;
                }
                let idx = Self::idx(row, col, cols);
                if row > 0 {
                    let above = Self::idx(row - 1, col, cols);
                    sum_x[idx] = row_sum_x + sum_x[above];
                    sum_y[idx] = row_sum_y + sum_y[above];
                    sum_z[idx] = row_sum_z + sum_z[above];
                    count[idx] = row_count + count[above];
                } else {
                    sum_x[idx] = row_sum_x;
                    sum_y[idx] = row_sum_y;
                    sum_z[idx] = row_sum_z;
                    count[idx] = row_count;
                }
            }
        }

        (sum_x, sum_y, sum_z, count)
    }

    fn run_multiscale_roughness(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;
        let mut min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let mut max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);

        if max_scale < min_scale {
            std::mem::swap(&mut min_scale, &mut max_scale);
        }

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;

        let mut scales = Vec::new();
        let mut s = min_scale;
        while s <= max_scale {
            scales.push(s);
            if let Some(next) = s.checked_add(step_size) {
                s = next;
            } else {
                break;
            }
        }

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let (sum, _, count) = Self::build_integrals(&input, band);
            let mut base_normals: Vec<Option<[f64; 3]>> = Vec::with_capacity(rows * cols);
            for r in 0..rows {
                for c in 0..cols {
                    base_normals.push(Self::normal_from_raster(
                        &input,
                        band,
                        r as isize,
                        c as isize,
                        z_factor,
                    ));
                }
            }

            for r in 0..rows {
                output_mag
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e)))?;
                output_scale
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing scale row {}: {}", r, e)))?;
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                if midpoint * 2 + 1 > rows.max(cols) {
                    break;
                }

                let smoothed_rows: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![nodata; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n > 0 {
                                let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                                *out_cell = local_sum / n as f64;
                            }
                        }
                        out
                    })
                    .collect();

                let mut smooth = input.clone();
                for (r, row) in smoothed_rows.iter().enumerate() {
                    smooth
                        .set_row_slice(band, r as isize, row)
                        .map_err(|e| ToolError::Execution(format!("failed writing smoothed row {}: {}", r, e)))?;
                }

                let diff_rows: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![0.0; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let idx = Self::idx(r, c, cols);
                            let base = match base_normals[idx] {
                                Some(v) => v,
                                None => continue,
                            };
                            if let Some(smooth_n) = Self::normal_from_raster(
                                &smooth,
                                band,
                                r as isize,
                                c as isize,
                                z_factor,
                            ) {
                                *out_cell = Self::angle_between_normals(base, smooth_n);
                            }
                        }
                        out
                    })
                    .collect();

                let mut diff_raster = input.clone();
                for (r, row) in diff_rows.iter().enumerate() {
                    diff_raster
                        .set_row_slice(band, r as isize, row)
                        .map_err(|e| ToolError::Execution(format!("failed writing roughness row {}: {}", r, e)))?;
                }
                let (diff_sum, _, diff_count) = Self::build_integrals(&diff_raster, band);

                let avg_rows: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![nodata; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&diff_count, cols, y1, x1, y2, x2);
                            if n > 0 {
                                let local_sum = Self::rect_sum(&diff_sum, cols, y1, x1, y2, x2);
                                *out_cell = local_sum / n as f64;
                            }
                        }
                        out
                    })
                    .collect();

                for (r, row) in avg_rows.iter().enumerate() {
                    for (c, v2) in row.iter().enumerate().take(cols) {
                        let v2 = *v2;
                        if v2 == nodata {
                            continue;
                        }
                        let v1 = output_mag.get(band, r as isize, c as isize);
                        if v1 == nodata || v2 > v1 {
                            output_mag
                                .set(band, r as isize, c as isize, v2)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing roughness value at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing roughness scale at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                        }
                    }
                }

                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn parse_vector_points(layer: &wbvector::Layer) -> Result<Vec<(f64, f64)>, ToolError> {
        let mut points = Vec::new();
        for feature in &layer.features {
            if let Some(geom) = &feature.geometry {
                match geom {
                    wbvector::Geometry::Point(c) => points.push((c.x, c.y)),
                    wbvector::Geometry::MultiPoint(cs) => {
                        points.extend(cs.iter().map(|c| (c.x, c.y)));
                    }
                    wbvector::Geometry::GeometryCollection(gs) => {
                        for g in gs {
                            match g {
                                wbvector::Geometry::Point(c) => points.push((c.x, c.y)),
                                wbvector::Geometry::MultiPoint(cs) => {
                                    points.extend(cs.iter().map(|c| (c.x, c.y)));
                                }
                                _ => {
                                    return Err(ToolError::Validation(
                                        "points input must contain only point geometries"
                                            .to_string(),
                                    ))
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(ToolError::Validation(
                            "points input must contain only point geometries".to_string(),
                        ))
                    }
                }
            }
        }
        if points.is_empty() {
            return Err(ToolError::Validation(
                "points input must contain at least one point feature".to_string(),
            ));
        }
        Ok(points)
    }

    fn write_signature_html(
        output_path: &std::path::Path,
        title: &str,
        y_label: &str,
        input_path: &str,
        sites: &[(usize, usize, usize)],
        site_values: &[Vec<(f64, f64)>],
    ) -> Result<(), ToolError> {
        let width = 900.0;
        let height = 520.0;
        let pad = 60.0;
        let min_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::INFINITY, f64::min);
        let max_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(f64::INFINITY, f64::min)
            .floor();
        let max_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let min_y = if min_y.is_finite() { min_y.min(0.0) } else { -1.0 };
        let max_y = if max_y.is_finite() { max_y.max(0.0) } else { 1.0 };
        let x_rng = (max_x - min_x).max(1.0);
        let y_rng = (max_y - min_y).max(1.0);
        let sx = |x: f64| pad + (x - min_x) / x_rng * (width - 2.0 * pad);
        let sy = |y: f64| height - pad - (y - min_y) / y_rng * (height - 2.0 * pad);

        let palette = [
            "#bf616a", "#5e81ac", "#a3be8c", "#d08770", "#b48ead", "#88c0d0", "#ebcb8b",
            "#2e3440",
        ];
        let mut series_svg = String::new();
        let mut legend = String::new();
        for (i, series) in site_values.iter().enumerate() {
            if series.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let points = series
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            series_svg.push_str(&format!(
                "<polyline fill='none' stroke='{}' stroke-width='2' points='{}' />",
                color, points
            ));
            legend.push_str(&format!(
                "<span style='display:inline-flex;align-items:center;margin-right:14px'><span style='width:14px;height:3px;background:{};display:inline-block;margin-right:6px'></span>Site {}</span>",
                color,
                sites[i].0
            ));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>{}</title><style>body{{font-family:Georgia,serif;background:#f6f1e8;color:#1f1f1f;margin:0;padding:24px}}main{{max-width:1080px;margin:0 auto}}h1{{font-weight:600;letter-spacing:.02em}}.card{{background:#fff;border:1px solid #e6dfd2;border-radius:12px;padding:16px;box-shadow:0 8px 24px rgba(0,0,0,.06)}}.legend{{margin:12px 0 0 4px;font-size:14px}}</style></head><body><main><h1>{}</h1><div class='card'><p><strong>Input DEM</strong>: {}</p><svg viewBox='0 0 {} {}' width='100%' height='{}'><rect x='{}' y='{}' width='{}' height='{}' fill='#ffffff' stroke='#c8c2b5'/><line x1='{}' y1='{:.2}' x2='{}' y2='{:.2}' stroke='#b0aa9b' stroke-dasharray='4 4'/>{}<text x='{:.2}' y='{:.2}' font-size='13' fill='#3d3b35'>Filter size (cells)</text><text transform='translate({:.2},{:.2}) rotate(-90)' font-size='13' fill='#3d3b35'>{}</text></svg><div class='legend'>{}</div></div></main></body></html>",
            title,
            title,
            input_path,
            width,
            height,
            height,
            pad,
            pad,
            width - 2.0 * pad,
            height - 2.0 * pad,
            pad,
            sy(0.0),
            width - pad,
            sy(0.0),
            series_svg,
            width / 2.0 - 48.0,
            height - 16.0,
            16.0,
            height / 2.0 + 48.0,
            y_label,
            legend
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {e}")))
    }

    fn write_signature_html_legacy(
        output_path: &std::path::Path,
        title: &str,
        y_label: &str,
        input_path: &str,
        sites: &[(usize, usize, usize)],
        site_values: &[Vec<(f64, f64)>],
    ) -> Result<(), ToolError> {
        let width = 700.0;
        let height = 500.0;
        let pad = 55.0;
        let min_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::INFINITY, f64::min);
        let max_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(f64::INFINITY, f64::min)
            .floor();
        let max_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil();
        let min_y = if min_y.is_finite() { min_y.min(0.0) } else { -1.0 };
        let max_y = if max_y.is_finite() { max_y.max(0.0) } else { 1.0 };
        let x_rng = (max_x - min_x).max(1.0);
        let y_rng = (max_y - min_y).max(1.0);
        let sx = |x: f64| pad + (x - min_x) / x_rng * (width - 2.0 * pad);
        let sy = |y: f64| height - pad - (y - min_y) / y_rng * (height - 2.0 * pad);

        let palette = [
            "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2",
            "#17becf",
        ];
        let mut series_svg = String::new();
        for (i, series) in site_values.iter().enumerate() {
            if series.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let points = series
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            series_svg.push_str(&format!(
                "<polyline fill='none' stroke='{}' stroke-width='1.8' points='{}' />",
                color, points
            ));
        }

        let mut legend = String::new();
        for (i, (sid, _, _)) in sites.iter().enumerate() {
            let color = palette[i % palette.len()];
            legend.push_str(&format!(
                "<span style='display:inline-flex;align-items:center;margin-right:12px'><span style='width:12px;height:2px;background:{};display:inline-block;margin-right:5px'></span>Site {}</span>",
                color, sid
            ));
        }

        let html = format!(
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\"><head><meta content=\"text/html; charset=UTF-8\" http-equiv=\"content-type\"><title>{}</title><style>body{{font-family:Arial,Helvetica,sans-serif;color:#111;background:#fff}}h1{{font-size:24px}}#graph{{margin:0 auto;width:760px;text-align:center}}</style></head><body><h1>{}</h1><p><strong>Input DEM</strong>: {}</p><div id='graph' align='center'><svg viewBox='0 0 {} {}' width='{}' height='{}'><rect x='{}' y='{}' width='{}' height='{}' fill='#ffffff' stroke='#b5b5b5'/><line x1='{}' y1='{:.2}' x2='{}' y2='{:.2}' stroke='#c7c7c7' stroke-dasharray='4 4'/>{}<text x='{:.2}' y='{:.2}' font-size='12'>Filter Size (cells)</text><text transform='translate({:.2},{:.2}) rotate(-90)' font-size='12'>{}</text></svg><div style='font-size:12px;margin-top:8px'>{}</div></div></body>",
            title,
            title,
            input_path,
            width,
            height,
            width,
            height,
            pad,
            pad,
            width - 2.0 * pad,
            height - 2.0 * pad,
            pad,
            sy(0.0),
            width - pad,
            sy(0.0),
            series_svg,
            width / 2.0 - 45.0,
            height - 14.0,
            15.0,
            height / 2.0 + 40.0,
            y_label,
            legend
        );

        std::fs::write(output_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {e}")))
    }

    fn run_max_elev_dev_signature(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let points_path = parse_vector_path_arg(args, "points")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10)
            .max(1);

        let input = Self::load_raster(&input_path)?;
        let points_layer = wbvector::read(&points_path).map_err(|e| {
            ToolError::Validation(format!("failed reading points vector '{}': {}", points_path, e))
        })?;
        let point_coords = Self::parse_vector_points(&points_layer)?;

        let mut sites = Vec::new();
        for (i, (x, y)) in point_coords.iter().enumerate() {
            if let Some((col, row)) = input.world_to_pixel(*x, *y) {
                let z = input.get(0, row, col);
                if !input.is_nodata(z) {
                    sites.push((i + 1, row as usize, col as usize));
                }
            }
        }
        if sites.is_empty() {
            return Err(ToolError::Validation(
                "no points intersect valid DEM cells".to_string(),
            ));
        }

        let (sum, sum_sq, count) = Self::build_integrals(&input, 0);
        let rows = input.rows;
        let cols = input.cols;

        let mut scales = Vec::new();
        let mut s = min_scale;
        while s <= max_scale {
            scales.push(s);
            if let Some(next) = s.checked_add(step_size) {
                s = next;
            } else {
                break;
            }
        }

        let mut site_values: Vec<Vec<(f64, f64)>> = vec![Vec::new(); sites.len()];
        for (scale_idx, midpoint) in scales.iter().enumerate() {
            let midpoint = *midpoint;
            for (site_idx, (_sid, row, col)) in sites.iter().enumerate() {
                let z = input.get(0, *row as isize, *col as isize);
                if input.is_nodata(z) {
                    continue;
                }
                let y1 = row.saturating_sub(midpoint);
                let x1 = col.saturating_sub(midpoint);
                let y2 = (row + midpoint).min(rows - 1);
                let x2 = (col + midpoint).min(cols - 1);

                let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                let mut dev = 0.0;
                if n > 1 {
                    let n_f = n as f64;
                    let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                    let local_sum_sq = Self::rect_sum(&sum_sq, cols, y1, x1, y2, x2);
                    let variance = ((local_sum_sq - (local_sum * local_sum) / n_f) / n_f).max(0.0);
                    let std_dev = variance.sqrt();
                    if std_dev > 0.0 {
                        dev = (z - local_sum / n_f) / std_dev;
                    }
                }
                site_values[site_idx].push(((midpoint * 2 + 1) as f64, dev));
            }
            coalescer.emit_unit_fraction(ctx.progress, (scale_idx + 1) as f64 / scales.len() as f64);
        }

        let out_path = output_path.unwrap_or_else(|| std::env::temp_dir().join("max_elev_dev_signature.html"));
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        let width = 900.0;
        let height = 520.0;
        let pad = 60.0;
        let min_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::INFINITY, f64::min);
        let max_x = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(x, _)| *x))
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(-4.0, f64::min)
            .floor();
        let max_y = site_values
            .iter()
            .flat_map(|s| s.iter().map(|(_, y)| *y))
            .fold(4.0, f64::max)
            .ceil();
        let x_rng = (max_x - min_x).max(1.0);
        let y_rng = (max_y - min_y).max(1.0);
        let sx = |x: f64| pad + (x - min_x) / x_rng * (width - 2.0 * pad);
        let sy = |y: f64| height - pad - (y - min_y) / y_rng * (height - 2.0 * pad);

        let palette = ["#bf616a", "#5e81ac", "#a3be8c", "#d08770", "#b48ead", "#88c0d0", "#ebcb8b", "#2e3440"];
        let mut series_svg = String::new();
        let mut legend = String::new();
        for (i, series) in site_values.iter().enumerate() {
            if series.is_empty() {
                continue;
            }
            let color = palette[i % palette.len()];
            let points = series
                .iter()
                .map(|(x, y)| format!("{:.2},{:.2}", sx(*x), sy(*y)))
                .collect::<Vec<_>>()
                .join(" ");
            series_svg.push_str(&format!(
                "<polyline fill='none' stroke='{}' stroke-width='2' points='{}' />",
                color, points
            ));
            legend.push_str(&format!(
                "<span style='display:inline-flex;align-items:center;margin-right:14px'><span style='width:14px;height:3px;background:{};display:inline-block;margin-right:6px'></span>Site {}</span>",
                color,
                sites[i].0
            ));
        }

        let html = format!(
            "<!doctype html><html><head><meta charset='utf-8'><title>Max Elevation Deviation Signature</title><style>body{{font-family:Georgia,serif;background:#f6f1e8;color:#1f1f1f;margin:0;padding:24px}}main{{max-width:1080px;margin:0 auto}}h1{{font-weight:600;letter-spacing:.02em}}.card{{background:#fff;border:1px solid #e6dfd2;border-radius:12px;padding:16px;box-shadow:0 8px 24px rgba(0,0,0,.06)}}.legend{{margin:12px 0 0 4px;font-size:14px}}</style></head><body><main><h1>Maximum Elevation Deviation Signature</h1><div class='card'><p><strong>Input DEM</strong>: {}</p><svg viewBox='0 0 {} {}' width='100%' height='{}'><rect x='{}' y='{}' width='{}' height='{}' fill='#ffffff' stroke='#c8c2b5'/><line x1='{}' y1='{:.2}' x2='{}' y2='{:.2}' stroke='#b0aa9b' stroke-dasharray='4 4'/>{}<text x='{:.2}' y='{:.2}' font-size='13' fill='#3d3b35'>Filter size (cells)</text><text transform='translate({:.2},{:.2}) rotate(-90)' font-size='13' fill='#3d3b35'>DEV</text></svg><div class='legend'>{}</div></div></main></body></html>",
            input_path,
            width,
            height,
            height,
            pad,
            pad,
            width - 2.0 * pad,
            height - 2.0 * pad,
            pad,
            sy(0.0),
            width - pad,
            sy(0.0),
            series_svg,
            width / 2.0 - 48.0,
            height - 16.0,
            16.0,
            height / 2.0 + 48.0,
            legend
        );

        std::fs::write(&out_path, html)
            .map_err(|e| ToolError::Execution(format!("failed writing HTML report: {e}")))?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(out_path.to_string_lossy().to_string()));
        coalescer.finish(ctx.progress);
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn run_max_anisotropy_dev_signature(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let points_path = parse_vector_path_arg(args, "points")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = Self::load_raster(&input_path)?;
        let points_layer = wbvector::read(&points_path).map_err(|e| {
            ToolError::Validation(format!("failed reading points vector '{}': {}", points_path, e))
        })?;
        let point_coords = Self::parse_vector_points(&points_layer)?;

        let mut sites = Vec::new();
        for (i, (x, y)) in point_coords.iter().enumerate() {
            if let Some((col, row)) = input.world_to_pixel(*x, *y) {
                let z = input.get(0, row, col);
                if !input.is_nodata(z) {
                    sites.push((i + 1, row as usize, col as usize));
                }
            }
        }
        if sites.is_empty() {
            return Err(ToolError::Validation(
                "no points intersect valid DEM cells".to_string(),
            ));
        }

        let (sum, sum_sq, count) = Self::build_integrals(&input, 0);
        let rows = input.rows;
        let cols = input.cols;

        let mut scales = Vec::new();
        let mut s = min_scale;
        while s <= max_scale {
            scales.push(s);
            if let Some(next) = s.checked_add(step_size) {
                s = next;
            } else {
                break;
            }
        }

        let mut site_values: Vec<Vec<(f64, f64)>> = vec![Vec::new(); sites.len()];
        for (scale_idx, midpoint) in scales.iter().enumerate() {
            let midpoint = *midpoint;
            let middle_radius = ((midpoint * 2 + 1) / 6).max(1);
            for (site_idx, (_sid, row, col)) in sites.iter().enumerate() {
                if *row < midpoint || *row + midpoint >= rows || *col < midpoint || *col + midpoint >= cols {
                    site_values[site_idx].push(((midpoint * 2 + 1) as f64, 0.0));
                    continue;
                }
                let z = input.get(0, *row as isize, *col as isize);
                if input.is_nodata(z) {
                    site_values[site_idx].push(((midpoint * 2 + 1) as f64, 0.0));
                    continue;
                }

                let oy1 = *row - midpoint;
                let oy2 = *row + midpoint;
                let ox1 = *col - midpoint;
                let ox2 = *col + midpoint;
                let iy1 = row.saturating_sub(middle_radius);
                let iy2 = (*row + middle_radius).min(rows - 1);
                let ix1 = col.saturating_sub(middle_radius);
                let ix2 = (*col + middle_radius).min(cols - 1);

                let overall = match Self::panel_dev(
                    z, &sum, &sum_sq, &count, cols, oy1, ox1, oy2, ox2,
                ) {
                    Some(v) => v,
                    None => {
                        site_values[site_idx].push(((midpoint * 2 + 1) as f64, 0.0));
                        continue;
                    }
                };

                let mut sq_sum = 0.0;
                let mut valid = 0usize;

                if let Some(v) = Self::panel_dev(z, &sum, &sum_sq, &count, cols, oy1, ix1, oy2, ix2) {
                    let d = v - overall;
                    sq_sum += d * d;
                    valid += 1;
                }
                if let Some(v) = Self::panel_dev(z, &sum, &sum_sq, &count, cols, iy1, ox1, iy2, ox2) {
                    let d = v - overall;
                    sq_sum += d * d;
                    valid += 1;
                }

                let diag_center = Self::panel_dev(z, &sum, &sum_sq, &count, cols, iy1, ix1, iy2, ix2);

                let ne_sw = {
                    let top_right = Self::panel_dev(z, &sum, &sum_sq, &count, cols, oy1, ix2, iy1, ox2);
                    let bottom_left = Self::panel_dev(z, &sum, &sum_sq, &count, cols, iy2, ox1, oy2, ix1);
                    if let (Some(a), Some(cn), Some(b)) = (top_right, diag_center, bottom_left) {
                        Some((a + cn + b) / 3.0)
                    } else {
                        None
                    }
                };
                if let Some(v) = ne_sw {
                    let d = v - overall;
                    sq_sum += d * d;
                    valid += 1;
                }

                let nw_se = {
                    let top_left = Self::panel_dev(z, &sum, &sum_sq, &count, cols, oy1, ox1, iy1, ix1);
                    let bottom_right = Self::panel_dev(z, &sum, &sum_sq, &count, cols, iy2, ix2, oy2, ox2);
                    if let (Some(a), Some(cn), Some(b)) = (top_left, diag_center, bottom_right) {
                        Some((a + cn + b) / 3.0)
                    } else {
                        None
                    }
                };
                if let Some(v) = nw_se {
                    let d = v - overall;
                    sq_sum += d * d;
                    valid += 1;
                }

                let anis = if valid > 0 {
                    (sq_sum / valid as f64).sqrt()
                } else {
                    0.0
                };
                site_values[site_idx].push(((midpoint * 2 + 1) as f64, anis));
            }
            coalescer.emit_unit_fraction(ctx.progress, (scale_idx + 1) as f64 / scales.len() as f64);
        }

        let out_path = output_path
            .unwrap_or_else(|| std::env::temp_dir().join("max_anisotropy_dev_signature.html"));
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        Self::write_signature_html_legacy(
            &out_path,
            "Max Anisotropy Deviation Signature",
            "Anisotropy",
            &input_path,
            &sites,
            &site_values,
        )?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(out_path.to_string_lossy().to_string()));
        coalescer.finish(ctx.progress);
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn run_multiscale_roughness_signature(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let points_path = parse_vector_path_arg(args, "points")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let max_scale = args
            .get("max_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100)
            .max(min_scale);
        let step_size = args
            .get("step_size")
            .or_else(|| args.get("step"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let output_path = parse_optional_output_path(args, "output")?;

        let input = Self::load_raster(&input_path)?;
        let points_layer = wbvector::read(&points_path).map_err(|e| {
            ToolError::Validation(format!("failed reading points vector '{}': {}", points_path, e))
        })?;
        let point_coords = Self::parse_vector_points(&points_layer)?;

        let mut sites = Vec::new();
        for (i, (x, y)) in point_coords.iter().enumerate() {
            if let Some((col, row)) = input.world_to_pixel(*x, *y) {
                let z = input.get(0, row, col);
                if !input.is_nodata(z) {
                    sites.push((i + 1, row as usize, col as usize));
                }
            }
        }
        if sites.is_empty() {
            return Err(ToolError::Validation(
                "no points intersect valid DEM cells".to_string(),
            ));
        }

        let (sum, _, count) = Self::build_integrals(&input, 0);
        let rows = input.rows;
        let cols = input.cols;
        let mut base_normals: Vec<Option<[f64; 3]>> = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                base_normals.push(Self::normal_from_raster(
                    &input,
                    0,
                    r as isize,
                    c as isize,
                    z_factor,
                ));
            }
        }

        let mut scales = Vec::new();
        let mut s = min_scale;
        while s <= max_scale {
            scales.push(s);
            if let Some(next) = s.checked_add(step_size) {
                s = next;
            } else {
                break;
            }
        }

        let mut site_values: Vec<Vec<(f64, f64)>> = vec![Vec::new(); sites.len()];
        for (scale_idx, midpoint) in scales.iter().enumerate() {
            let midpoint = *midpoint;
            let smoothed_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out = vec![input.nodata; cols];
                    for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                        let z = input.get(0, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(midpoint);
                        let x1 = c.saturating_sub(midpoint);
                        let y2 = (r + midpoint).min(rows - 1);
                        let x2 = (c + midpoint).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n > 0 {
                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            *out_cell = local_sum / n as f64;
                        }
                    }
                    out
                })
                .collect();

            let mut smooth = input.clone();
            for (r, row) in smoothed_rows.iter().enumerate() {
                smooth
                    .set_row_slice(0, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing smoothed row {}: {}", r, e)))?;
            }

            let diff_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out = vec![0.0; cols];
                    for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                        let idx = Self::idx(r, c, cols);
                        let base = match base_normals[idx] {
                            Some(v) => v,
                            None => continue,
                        };
                        if let Some(smooth_n) = Self::normal_from_raster(
                            &smooth,
                            0,
                            r as isize,
                            c as isize,
                            z_factor,
                        ) {
                            *out_cell = Self::angle_between_normals(base, smooth_n);
                        }
                    }
                    out
                })
                .collect();

            let mut diff_raster = input.clone();
            for (r, row) in diff_rows.iter().enumerate() {
                diff_raster
                    .set_row_slice(0, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing roughness row {}: {}", r, e)))?;
            }
            let (diff_sum, _, diff_count) = Self::build_integrals(&diff_raster, 0);

            for (site_idx, (_sid, row, col)) in sites.iter().enumerate() {
                let z = input.get(0, *row as isize, *col as isize);
                if input.is_nodata(z) {
                    site_values[site_idx].push(((midpoint * 2 + 1) as f64, 0.0));
                    continue;
                }
                let y1 = row.saturating_sub(midpoint);
                let x1 = col.saturating_sub(midpoint);
                let y2 = (*row + midpoint).min(rows - 1);
                let x2 = (*col + midpoint).min(cols - 1);
                let n = Self::rect_count(&diff_count, cols, y1, x1, y2, x2);
                let rough = if n > 0 {
                    Self::rect_sum(&diff_sum, cols, y1, x1, y2, x2) / n as f64
                } else {
                    0.0
                };
                site_values[site_idx].push(((midpoint * 2 + 1) as f64, rough));
            }

            coalescer.emit_unit_fraction(ctx.progress, (scale_idx + 1) as f64 / scales.len() as f64);
        }

        let out_path = output_path
            .unwrap_or_else(|| std::env::temp_dir().join("multiscale_roughness_signature.html"));
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        Self::write_signature_html(
            &out_path,
            "Multiscale Roughness Signature",
            "Roughness (deg)",
            &input_path,
            &sites,
            &site_values,
        )?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(out_path.to_string_lossy().to_string()));
        coalescer.finish(ctx.progress);
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }

    fn run_multiscale_std_dev_normals(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let output_path = parse_optional_output_path(args, "output")?;
        let output_scale_path = parse_optional_output_path(args, "output_scale")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let step = args
            .get("step")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10)
            .max(1);
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(1.0, 4.0);
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let mut output_mag = input.clone();
        let mut output_scale = input.clone();
        let rows = input.rows;
        let cols = input.cols;
        let bands = input.bands;
        let nodata = input.nodata;
        let scales = Self::make_nonlinear_scales(min_scale, step, num_steps, step_nonlinearity);

        for band_idx in 0..bands {
            let band = band_idx as isize;
            let (sum, _, count) = Self::build_integrals(&input, band);

            for r in 0..rows {
                output_mag
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing magnitude row {}: {}", r, e)))?;
                output_scale
                    .set_row_slice(band, r as isize, &vec![nodata; cols])
                    .map_err(|e| ToolError::Execution(format!("failed initializing scale row {}: {}", r, e)))?;
            }

            for (loop_idx, midpoint) in scales.iter().enumerate() {
                let midpoint = *midpoint;
                if midpoint * 2 + 1 > rows.max(cols) {
                    continue;
                }

                let smoothed_rows: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![nodata; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                            if n > 0 {
                                let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                                *out_cell = local_sum / n as f64;
                            }
                        }
                        out
                    })
                    .collect();

                let mut smooth = input.clone();
                for (r, row) in smoothed_rows.iter().enumerate() {
                    smooth
                        .set_row_slice(band, r as isize, row)
                        .map_err(|e| ToolError::Execution(format!("failed writing smoothed row {}: {}", r, e)))?;
                }

                let (sum_x, sum_y, sum_z, count_n) =
                    Self::build_unit_normal_component_integrals(&smooth, band, z_factor);

                let row_data: Vec<Vec<f64>> = (0..rows)
                    .into_par_iter()
                    .map(|r| {
                        let mut out = vec![nodata; cols];
                        for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                            let z = input.get(band, r as isize, c as isize);
                            if input.is_nodata(z) {
                                continue;
                            }
                            let y1 = r.saturating_sub(midpoint);
                            let x1 = c.saturating_sub(midpoint);
                            let y2 = (r + midpoint).min(rows - 1);
                            let x2 = (c + midpoint).min(cols - 1);
                            let n = Self::rect_count(&count_n, cols, y1, x1, y2, x2);
                            if n > 1 {
                                let sx = Self::rect_sum(&sum_x, cols, y1, x1, y2, x2);
                                let sy = Self::rect_sum(&sum_y, cols, y1, x1, y2, x2);
                                let sz = Self::rect_sum(&sum_z, cols, y1, x1, y2, x2);
                                let rlen = (sx * sx + sy * sy + sz * sz).sqrt();
                                let ratio = (rlen / n as f64).clamp(1e-12, 1.0);
                                *out_cell = (-2.0 * ratio.ln()).max(0.0).sqrt() * 57.29577951308232;
                            } else {
                                *out_cell = 0.0;
                            }
                        }
                        out
                    })
                    .collect();

                for (r, row) in row_data.iter().enumerate() {
                    for (c, v2) in row.iter().enumerate().take(cols) {
                        let v2 = *v2;
                        if v2 == nodata {
                            continue;
                        }
                        let v1 = output_mag.get(band, r as isize, c as isize);
                        if v1 == nodata || v2 > v1 {
                            output_mag.set(band, r as isize, c as isize, v2).map_err(|e| {
                                ToolError::Execution(format!(
                                    "failed writing std-dev value at row {} col {}: {}",
                                    r, c, e
                                ))
                            })?;
                            output_scale
                                .set(band, r as isize, c as isize, midpoint as f64)
                                .map_err(|e| {
                                    ToolError::Execution(format!(
                                        "failed writing std-dev scale at row {} col {}: {}",
                                        r, c, e
                                    ))
                                })?;
                        }
                    }
                }
                coalescer.emit_unit_fraction(ctx.progress, (loop_idx + 1) as f64 / scales.len() as f64);
            }
        }

        let output_locator = Self::write_or_store_output(output_mag, output_path)?;
        let scale_locator = Self::write_or_store_output(output_scale, output_scale_path)?;
        coalescer.finish(ctx.progress);
        Ok(Self::build_result_with_scale(output_locator, scale_locator))
    }

    fn run_multiscale_std_dev_normals_signature(
        args: &ToolArgs,
        ctx: &ToolContext,
    ) -> Result<ToolRunResult, ToolError> {
        let coalescer = PercentCoalescer::new(1, 99);
        let input_path = Self::parse_input(args)?;
        let points_path = parse_vector_path_arg(args, "points")?;
        let output_path = parse_optional_output_path(args, "output")?;
        let min_scale = args
            .get("min_scale")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let step = args
            .get("step")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .max(1);
        let num_steps = args
            .get("num_steps")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10)
            .max(1);
        let step_nonlinearity = args
            .get("step_nonlinearity")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0)
            .clamp(1.0, 4.0);
        let z_factor = args.get("z_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let input = Self::load_raster(&input_path)?;
        let points_layer = wbvector::read(&points_path).map_err(|e| {
            ToolError::Validation(format!("failed reading points vector '{}': {}", points_path, e))
        })?;
        let point_coords = Self::parse_vector_points(&points_layer)?;

        let mut sites = Vec::new();
        for (i, (x, y)) in point_coords.iter().enumerate() {
            if let Some((col, row)) = input.world_to_pixel(*x, *y) {
                let z = input.get(0, row, col);
                if !input.is_nodata(z) {
                    sites.push((i + 1, row as usize, col as usize));
                }
            }
        }
        if sites.is_empty() {
            return Err(ToolError::Validation(
                "no points intersect valid DEM cells".to_string(),
            ));
        }

        let rows = input.rows;
        let cols = input.cols;
        let (sum, _, count) = Self::build_integrals(&input, 0);
        let scales = Self::make_nonlinear_scales(min_scale, step, num_steps, step_nonlinearity);
        let mut site_values: Vec<Vec<(f64, f64)>> = vec![Vec::new(); sites.len()];

        for (scale_idx, midpoint) in scales.iter().enumerate() {
            let midpoint = *midpoint;
            if midpoint * 2 + 1 > rows.max(cols) {
                continue;
            }

            let smoothed_rows: Vec<Vec<f64>> = (0..rows)
                .into_par_iter()
                .map(|r| {
                    let mut out = vec![input.nodata; cols];
                    for (c, out_cell) in out.iter_mut().enumerate().take(cols) {
                        let z = input.get(0, r as isize, c as isize);
                        if input.is_nodata(z) {
                            continue;
                        }
                        let y1 = r.saturating_sub(midpoint);
                        let x1 = c.saturating_sub(midpoint);
                        let y2 = (r + midpoint).min(rows - 1);
                        let x2 = (c + midpoint).min(cols - 1);
                        let n = Self::rect_count(&count, cols, y1, x1, y2, x2);
                        if n > 0 {
                            let local_sum = Self::rect_sum(&sum, cols, y1, x1, y2, x2);
                            *out_cell = local_sum / n as f64;
                        }
                    }
                    out
                })
                .collect();

            let mut smooth = input.clone();
            for (r, row) in smoothed_rows.iter().enumerate() {
                smooth
                    .set_row_slice(0, r as isize, row)
                    .map_err(|e| ToolError::Execution(format!("failed writing smoothed row {}: {}", r, e)))?;
            }

            let (sum_x, sum_y, sum_z, count_n) =
                Self::build_unit_normal_component_integrals(&smooth, 0, z_factor);

            for (site_idx, (_sid, row, col)) in sites.iter().enumerate() {
                let z = input.get(0, *row as isize, *col as isize);
                if input.is_nodata(z) {
                    site_values[site_idx].push(((midpoint * 2 + 1) as f64, 0.0));
                    continue;
                }
                let y1 = row.saturating_sub(midpoint);
                let x1 = col.saturating_sub(midpoint);
                let y2 = (*row + midpoint).min(rows - 1);
                let x2 = (*col + midpoint).min(cols - 1);
                let n = Self::rect_count(&count_n, cols, y1, x1, y2, x2);
                let sigma = if n > 1 {
                    let sx = Self::rect_sum(&sum_x, cols, y1, x1, y2, x2);
                    let sy = Self::rect_sum(&sum_y, cols, y1, x1, y2, x2);
                    let sz = Self::rect_sum(&sum_z, cols, y1, x1, y2, x2);
                    let rlen = (sx * sx + sy * sy + sz * sz).sqrt();
                    let ratio = (rlen / n as f64).clamp(1e-12, 1.0);
                    (-2.0 * ratio.ln()).max(0.0).sqrt() * 57.29577951308232
                } else {
                    0.0
                };
                site_values[site_idx].push(((midpoint * 2 + 1) as f64, sigma));
            }
            coalescer.emit_unit_fraction(ctx.progress, (scale_idx + 1) as f64 / scales.len() as f64);
        }

        let out_path = output_path
            .unwrap_or_else(|| std::env::temp_dir().join("multiscale_std_dev_normals_signature.html"));
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ToolError::Execution(format!("failed creating output directory: {e}"))
                })?;
            }
        }

        Self::write_signature_html(
            &out_path,
            "Multiscale Std Dev Normals Signature",
            "Sigma_s (deg)",
            &input_path,
            &sites,
            &site_values,
        )?;

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("path".to_string(), json!(out_path.to_string_lossy().to_string()));
        coalescer.finish(ctx.progress);
        Ok(ToolRunResult {
            outputs,
            ..Default::default()
        })
    }
}

impl Tool for DifferenceFromMeanElevationTool {
    fn metadata(&self) -> ToolMetadata { TerrainWindowCore::difference_from_mean_elevation_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainWindowCore::difference_from_mean_elevation_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_difference_from_mean_elevation(args, ctx)
    }
}

impl Tool for DeviationFromMeanElevationTool {
    fn metadata(&self) -> ToolMetadata { TerrainWindowCore::deviation_from_mean_elevation_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainWindowCore::deviation_from_mean_elevation_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_deviation_from_mean_elevation(args, ctx)
    }
}

impl Tool for StandardDeviationOfSlopeTool {
    fn metadata(&self) -> ToolMetadata { TerrainWindowCore::standard_deviation_of_slope_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainWindowCore::standard_deviation_of_slope_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_standard_deviation_of_slope(args, ctx)
    }
}

impl Tool for MaxDifferenceFromMeanTool {
    fn metadata(&self) -> ToolMetadata { TerrainWindowCore::max_difference_from_mean_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainWindowCore::max_difference_from_mean_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_max_difference_from_mean(args, ctx)
    }
}

impl Tool for MaxElevationDeviationTool {
    fn metadata(&self) -> ToolMetadata { TerrainWindowCore::max_elevation_deviation_metadata() }
    fn manifest(&self) -> ToolManifest { TerrainWindowCore::max_elevation_deviation_manifest() }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_max_elevation_deviation(args, ctx)
    }
}

impl Tool for TopographicPositionAnimationTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::topographic_position_animation_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::topographic_position_animation_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        TerrainWindowCore::validate_topographic_position_animation(args)
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_topographic_position_animation(args, ctx)
    }
}

impl Tool for MultiscaleTopographicPositionImageTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_topographic_position_image_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_topographic_position_image_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "local")?;
        let _ = parse_raster_path_arg(args, "meso")?;
        let _ = parse_raster_path_arg(args, "broad")?;
        if let Some(v) = args.get("hillshade") {
            let s = v
                .as_str()
                .ok_or_else(|| ToolError::Validation("parameter 'hillshade' must be a string path".to_string()))?;
            if s.trim().is_empty() {
                return Err(ToolError::Validation(
                    "parameter 'hillshade' must not be empty when provided".to_string(),
                ));
            }
        }
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_topographic_position_image(args, ctx)
    }
}

impl Tool for MultiscaleElevationPercentileTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_elevation_percentile_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_elevation_percentile_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_elevation_percentile(args, ctx)
    }
}

impl Tool for MaxAnisotropyDevTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::max_anisotropy_dev_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::max_anisotropy_dev_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_max_anisotropy_dev(args, ctx)
    }
}

impl Tool for MultiscaleRoughnessTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_roughness_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_roughness_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_roughness(args, ctx)
    }
}

impl Tool for MaxElevDevSignatureTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::max_elev_dev_signature_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::max_elev_dev_signature_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_vector_path_arg(args, "points")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_max_elev_dev_signature(args, ctx)
    }
}

impl Tool for MaxAnisotropyDevSignatureTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::max_anisotropy_dev_signature_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::max_anisotropy_dev_signature_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_vector_path_arg(args, "points")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_max_anisotropy_dev_signature(args, ctx)
    }
}

impl Tool for MultiscaleRoughnessSignatureTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_roughness_signature_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_roughness_signature_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_vector_path_arg(args, "points")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_roughness_signature(args, ctx)
    }
}

impl Tool for MultiscaleStdDevNormalsTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_std_dev_normals_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_std_dev_normals_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_std_dev_normals(args, ctx)
    }
}

impl Tool for MultiscaleStdDevNormalsSignatureTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_std_dev_normals_signature_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_std_dev_normals_signature_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_vector_path_arg(args, "points")?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_std_dev_normals_signature(args, ctx)
    }
}

impl Tool for FeaturePreservingSmoothingTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::feature_preserving_smoothing_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::feature_preserving_smoothing_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_feature_preserving_smoothing(args, ctx)
    }
}

impl Tool for FeaturePreservingSmoothingPoissonTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::feature_preserving_smoothing_poisson_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::feature_preserving_smoothing_poisson_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_feature_preserving_smoothing_poisson(args, ctx)
    }
}

impl Tool for FeaturePreservingSmoothingMultiscaleTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::feature_preserving_smoothing_multiscale_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::feature_preserving_smoothing_multiscale_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_feature_preserving_smoothing_multiscale(args, ctx)
    }
}

impl Tool for FillMissingDataTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::fill_missing_data_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::fill_missing_data_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_fill_missing_data(args, ctx)
    }
}

impl Tool for RemoveOffTerrainObjectsTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::remove_off_terrain_objects_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::remove_off_terrain_objects_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_remove_off_terrain_objects(args, ctx)
    }
}

impl Tool for MapOffTerrainObjectsTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::map_off_terrain_objects_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::map_off_terrain_objects_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_map_off_terrain_objects(args, ctx)
    }
}

impl Tool for EmbankmentMappingTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::embankment_mapping_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::embankment_mapping_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = parse_raster_path_arg(args, "dem").or_else(|_| parse_raster_path_arg(args, "input"))?;
        let _ = parse_vector_path_arg(args, "roads_vector")
            .or_else(|_| parse_vector_path_arg(args, "road_vec"))
            .or_else(|_| parse_vector_path_arg(args, "roads"))?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_dem")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_embankment_mapping(args, ctx)
    }
}

impl Tool for SmoothVegetationResidualTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::smooth_vegetation_residual_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::smooth_vegetation_residual_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_smooth_vegetation_residual(args, ctx)
    }
}

impl Tool for LocalHypsometricAnalysisTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::local_hypsometric_analysis_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::local_hypsometric_analysis_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_local_hypsometric_analysis(args, ctx)
    }
}

impl Tool for MultiscaleElevatedIndexTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_elevated_index_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_elevated_index_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_elevated_index(args, ctx)
    }
}

impl Tool for MultiscaleLowLyingIndexTool {
    fn metadata(&self) -> ToolMetadata {
        TerrainWindowCore::multiscale_low_lying_index_metadata()
    }
    fn manifest(&self) -> ToolManifest {
        TerrainWindowCore::multiscale_low_lying_index_manifest()
    }
    fn validate(&self, args: &ToolArgs) -> Result<(), ToolError> {
        let _ = TerrainWindowCore::parse_input(args)?;
        let _ = parse_optional_output_path(args, "output")?;
        let _ = parse_optional_output_path(args, "output_scale")?;
        Ok(())
    }
    fn run(&self, args: &ToolArgs, ctx: &ToolContext) -> Result<ToolRunResult, ToolError> {
        TerrainWindowCore::run_multiscale_low_lying_index(args, ctx)
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

    fn make_raster_with_center_peak(rows: usize, cols: usize, peak: f64) -> Raster {
        let cfg = RasterConfig { rows, cols, bands: 1, nodata: -9999.0, cell_size: 10.0, ..Default::default() };
        let mut raster = Raster::new(cfg);
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                raster.set(0, row, col, 0.0).unwrap();
            }
        }
        raster.set(0, (rows / 2) as isize, (cols / 2) as isize, peak).unwrap();
        raster
    }

    fn run_window_tool(tool: &dyn Tool, input: Raster) -> Raster {
        let id = memory_store::put_raster(input);
        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size_x".to_string(), json!(3));
        args.insert("filter_size_y".to_string(), json!(3));
        let result = tool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        memory_store::get_raster_by_id(out_id).unwrap()
    }

    #[test]
    fn difference_from_mean_elevation_center_peak_matches_expected() {
        let out = run_window_tool(&DifferenceFromMeanElevationTool, make_raster_with_center_peak(5, 5, 8.0));
        let v = out.get(0, 2, 2);
        assert!((v - (64.0 / 9.0)).abs() < 1e-6, "expected 64/9, got {v}");
    }

    #[test]
    fn deviation_from_mean_elevation_center_peak_matches_expected() {
        let out = run_window_tool(&DeviationFromMeanElevationTool, make_raster_with_center_peak(5, 5, 8.0));
        let v = out.get(0, 2, 2);
        assert!((v - 2.8284271247461903).abs() < 1e-6, "expected sqrt(8), got {v}");
    }

    #[test]
    fn standard_deviation_of_slope_is_zero_for_flat_dem() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(5, 5, 0.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size".to_string(), json!(3));
        let result = StandardDeviationOfSlopeTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 2, 2).abs() < 1e-12);
    }

    #[test]
    fn max_difference_from_mean_returns_magnitude_and_scale() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(7, 7, 10.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("min_scale".to_string(), json!(1));
        args.insert("max_scale".to_string(), json!(3));
        args.insert("step_size".to_string(), json!(1));
        let result = MaxDifferenceFromMeanTool.run(&args, &make_ctx()).unwrap();

        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        assert!(out.get(0, 3, 3) > 0.0);

        let scale_path = result.outputs.get("scale_path").unwrap().as_str().unwrap();
        let scale_id = memory_store::raster_path_to_id(scale_path).unwrap();
        let scale = memory_store::get_raster_by_id(scale_id).unwrap();
        let s = scale.get(0, 3, 3);
        assert!((1.0..=3.0).contains(&s));
    }

    #[test]
    fn smooth_vegetation_residual_removes_single_cell_spike() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(9, 9, 10.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("max_scale".to_string(), json!(3));
        args.insert("dev_threshold".to_string(), json!(2.0));
        args.insert("scale_threshold".to_string(), json!(3));

        let result = SmoothVegetationResidualTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();

        let center = out.get(0, 4, 4);
        assert!(center < 1.0, "expected center spike to be smoothed, got {center}");
    }

    #[test]
    fn remove_off_terrain_objects_reduces_center_spike() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(9, 9, 20.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("filter_size".to_string(), json!(5));
        args.insert("slope_threshold".to_string(), json!(10.0));

        let result = RemoveOffTerrainObjectsTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();

        let center = out.get(0, 4, 4);
        assert!(center < 20.0, "expected center spike to be reduced, got {center}");
    }

    #[test]
    fn map_off_terrain_objects_identifies_center_spike_segment() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(9, 9, 20.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("max_slope".to_string(), json!(10.0));
        args.insert("min_feature_size".to_string(), json!(0));

        let result = MapOffTerrainObjectsTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();

        let center = out.get(0, 4, 4);
        let corner = out.get(0, 0, 0);
        assert!(center > 0.0 && corner > 0.0);
        assert_ne!(center, corner, "expected center to be a distinct segment");
    }

    #[test]
    fn embankment_mapping_outputs_mask_and_optional_dem() {
        let cfg = RasterConfig {
            rows: 15,
            cols: 15,
            bands: 1,
            nodata: -9999.0,
            cell_size: 1.0,
            ..Default::default()
        };
        let mut dem = Raster::new(cfg);
        for r in 0..15isize {
            for c in 0..15isize {
                dem.set(0, r, c, 0.0).unwrap();
            }
        }
        for c in 3..12isize {
            dem.set(0, 7, c, 3.0).unwrap();
        }
        let dem_id = memory_store::put_raster(dem);

        let mut roads = wbvector::Layer::new("roads").with_geom_type(wbvector::GeometryType::LineString);
        roads
            .add_feature(
                Some(wbvector::Geometry::line_string(vec![
                    wbvector::Coord::xy(3.5, 7.5),
                    wbvector::Coord::xy(11.5, 7.5),
                ])),
                &[],
            )
            .unwrap();
        let roads_path = std::env::temp_dir().join("embankment_mapping_roads_test.shp");
        wbvector::write(&roads, roads_path.as_path(), wbvector::VectorFormat::Shapefile).unwrap();

        let mut args = ToolArgs::new();
        args.insert(
            "dem".to_string(),
            json!(memory_store::make_raster_memory_path(&dem_id)),
        );
        args.insert(
            "roads_vector".to_string(),
            json!(roads_path.to_string_lossy().to_string()),
        );
        args.insert("search_dist".to_string(), json!(2.5));
        args.insert("remove_embankments".to_string(), json!(true));

        let result = EmbankmentMappingTool.run(&args, &make_ctx()).unwrap();
        let mask_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let mask_id = memory_store::raster_path_to_id(mask_path).unwrap();
        let mask = memory_store::get_raster_by_id(mask_id).unwrap();
        assert_eq!(mask.get(0, 7, 7), 1.0);

        let dem_path = result.outputs.get("output_dem").unwrap().as_str().unwrap();
        let dem_out_id = memory_store::raster_path_to_id(dem_path).unwrap();
        let dem_out = memory_store::get_raster_by_id(dem_out_id).unwrap();
        assert!(dem_out.get(0, 7, 7) < 3.0);
    }

    #[test]
    fn local_hypsometric_analysis_center_peak_matches_expected() {
        let mut args = ToolArgs::new();
        let id = memory_store::put_raster(make_raster_with_center_peak(5, 5, 8.0));
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("min_scale".to_string(), json!(1));
        args.insert("step_size".to_string(), json!(1));
        args.insert("num_steps".to_string(), json!(1));
        args.insert("step_nonlinearity".to_string(), json!(1.0));

        let result = LocalHypsometricAnalysisTool.run(&args, &make_ctx()).unwrap();
        let out_path = result.outputs.get("path").unwrap().as_str().unwrap();
        let out_id = memory_store::raster_path_to_id(out_path).unwrap();
        let out = memory_store::get_raster_by_id(out_id).unwrap();
        let v = out.get(0, 2, 2);
        assert!((v - (1.0 / 9.0)).abs() < 1e-6, "expected 1/9, got {v}");

        let scale_path = result.outputs.get("scale_path").unwrap().as_str().unwrap();
        let scale_id = memory_store::raster_path_to_id(scale_path).unwrap();
        let scale = memory_store::get_raster_by_id(scale_id).unwrap();
        assert!((scale.get(0, 2, 2) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn topographic_position_animation_writes_html_and_gif() {
        let dem = make_raster_with_center_peak(11, 11, 10.0);
        let id = memory_store::put_raster(dem);

        let tmp_dir = std::env::temp_dir().join("wbtools_oss_topo_pos_anim_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let output_html = tmp_dir.join("topographic_position_animation.html");
        let output_gif = tmp_dir.join("topographic_position_animation.gif");

        let mut args = ToolArgs::new();
        args.insert("input".to_string(), json!(memory_store::make_raster_memory_path(&id)));
        args.insert("palette".to_string(), json!("soft"));
        args.insert("min_scale".to_string(), json!(1u64));
        args.insert("num_steps".to_string(), json!(2u64));
        args.insert("step_nonlinearity".to_string(), json!(1.0));
        args.insert("image_height".to_string(), json!(50u64));
        args.insert("delay".to_string(), json!(250u64));
        args.insert("output".to_string(), json!(output_html.to_string_lossy().to_string()));

        let result = TopographicPositionAnimationTool.run(&args, &make_ctx()).unwrap();
        let html_path = result.outputs.get("path").and_then(|v| v.as_str()).unwrap();
        let gif_path = result.outputs.get("gif_path").and_then(|v| v.as_str()).unwrap();
        assert!(std::path::Path::new(html_path).exists(), "HTML file not found: {html_path}");
        assert!(std::path::Path::new(gif_path).exists(), "GIF file not found: {gif_path}");
        assert_eq!(std::path::Path::new(gif_path), output_gif);
    }
}
